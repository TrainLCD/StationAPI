# 提案: A→B 走行シミュレーション用 gRPC クエリ `GetTrainRoute`

## 背景・目的

MobileApp の [`src/hooks/useSimulationMode.ts`](https://github.com/TrainLCD/MobileApp/blob/dev/src/hooks/useSimulationMode.ts)
は、サーバから取得した経路（A駅→B駅・特定の列車種別）をもとに、**端末内で**以下を行っている。

1. 区間ごとの waypoint（各駅の緯度経度）を組み立てる
2. `getIsPass`（= `stop_condition` / `pass`）で停車駅・通過駅を判定
3. 路線種別（`LineType`）と列車種別（`TrainTypeKind`）から最高速度・加減速度を引き、
   台形速度プロファイル（加速→巡航→減速）を 1 秒間隔で生成
4. waypoint 間距離（geolib `getDistance`）で位置を補間

この「端末内処理に必要な入力」を **1 回の API リクエスト**で揃えることが本提案の目的。

### スコープ（合意事項）

- **粒度**: 区間メタ情報まで。各駅の座標・停車/通過・最高速度・加減速度・駅間距離を返し、
  台形プロファイル生成と位置補間は引き続き端末側で行う（レスポンス軽量・実装堅実）。
- **種別指定**: リクエストで `line_group_id`（列車種別）を 1 つ指定し、その種別 1 本の走行情報のみ返す。
- **proto の扱い**: `.proto` は git submodule（`stationapi/proto` → `TrainLCD/gRPCProto`）であり、
  本リポジトリからは編集・push できない。本ドキュメントは **proto 変更案 + 適用可能な Rust 実装一式**を提示する。
  proto が gRPCProto にマージされ submodule が更新された後、下記コードを各ファイルへ適用すればビルドが通る。

---

## 1. Proto 変更案（`stationapi/proto/stationapi.proto`）

`service StationApi` に RPC を 1 つ追加し、メッセージを 3 つ新設する。

```proto
service StationApi {
  // ... 既存 RPC はそのまま ...
  rpc GetTrainRoute(GetTrainRouteRequest) returns (TrainRouteResponse);
}

// A→B 間を、指定した列車種別 1 本で走行する際の各駅の走行メタ情報。
// 既存 Station をそのまま内包し、シミュレーションに必要なフィールドを付与する。
message TrainRouteSegment {
  // 緯度経度・stop_condition・train_type・line(line_type) を含む既存 Station 型。
  Station station = 1;
  // この駅に停車するか（pass != 1）。通過駅は false。
  bool stops = 2;
  // 直前駅の座標からの距離（メートル, Haversine）。先頭駅は 0。
  double distance_from_previous = 3;
  // この駅へ向かう区間の最高巡航速度 (m/s)。
  double max_speed = 4;
  // 最大加速度 (m/s^2)。
  double max_acceleration = 5;
  // 最大減速度 (m/s^2)。
  double max_deceleration = 6;
}

message GetTrainRouteRequest {
  uint32 from_station_group_id = 1; // A駅 (station_g_cd)
  uint32 to_station_group_id   = 2; // B駅 (station_g_cd)
  uint32 line_group_id         = 3; // 指定する列車種別
}

message TrainRouteResponse {
  repeated TrainRouteSegment segments = 1;
}
```

設計上のポイント:

- **通過駅も含めて順序通り返す**。端末側は線形（track 追従）の waypoint として通過駅座標を使うため、
  `stops=false` の駅も省略しない。
- `distance_from_previous` を累積すれば端末側で各区間距離・全長を即座に得られる。
- `max_speed/acceleration/deceleration` は駅ごとに持たせることで、直通運転で `line_type` が変わる
  ケース（区間ごとに最高速度が変わる）に自然に対応できる。

---

## 2. Rust 実装（proto マージ後に各ファイルへ適用）

### 2.1 新規: 速度プロファイル定数モジュール `stationapi/src/use_case/dto/simulation.rs`

`useSimulationMode` 由来の物理定数を 1 箇所に集約する。値は MobileApp の `simulationMode` 定数と一致させること。

```rust
//! 走行シミュレーション用の速度・加減速度プロファイル。
//!
//! 値は MobileApp の `src/constants/simulationMode.ts`（`useSimulationMode` が参照）と一致させること。
//! 端末側の挙動と差異が出ないよう、定数を変更する際は両リポジトリを同時に更新する。

use crate::proto::{LineType, TrainTypeKind, TransportType};

/// 区間の最高速度・最大加速度・最大減速度（いずれも SI 単位: m/s, m/s^2）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedProfile {
    pub max_speed: f64,
    pub max_acceleration: f64,
    pub max_deceleration: f64,
}

// --- バス ---
const BUS_MAX_SPEED: f64 = 11.111_111_111_11; // 40 km/h
const BUS_MAX_ACCEL: f64 = 1.2;
const BUS_MAX_DECEL: f64 = 1.3;

// --- 路線種別ごとの最高速度 (m/s) ---
const BULLET_TRAIN_MAX_SPEED: f64 = 88.888_888_89; // 320 km/h
const MONORAIL_OR_AGT_MAX_SPEED: f64 = 22.222_222_222_22; // 80 km/h
const SUBWAY_MAX_SPEED: f64 = 22.222_222_222_22; // 80 km/h
const NORMAL_MAX_SPEED: f64 = 25.0; // 90 km/h
const TRAM_MAX_SPEED: f64 = 11.111_111_111_11; // 40 km/h

// --- 列車種別 kind による最高速度上限 (m/s) ---
// LimitedExpress / HighSpeedRapid のみ上限が設定され、他は路線種別の最高速度に従う。
const LIMITED_EXPRESS_KIND_MAX_SPEED: f64 = 36.111_111_111_1; // 130 km/h

/// 路線種別・輸送種別・列車種別 kind から速度プロファイルを決定する。
///
/// 引数は proto 上の i32 値（`LineType` / `TransportType` / `TrainTypeKind`）。
/// バス路線の場合は路線種別に関わらずバス用プロファイルを返す。
/// kind 固有の上限がある場合は路線種別の最高速度より優先する
/// （MobileApp: `maxSpeed = TRAIN_TYPE_KIND_MAX_SPEEDS[kind] ?? lineTypeMax`）。
pub fn resolve_speed_profile(line_type: i32, transport_type: i32, kind: i32) -> SpeedProfile {
    if transport_type == TransportType::Bus as i32 {
        return SpeedProfile {
            max_speed: BUS_MAX_SPEED,
            max_acceleration: BUS_MAX_ACCEL,
            max_deceleration: BUS_MAX_DECEL,
        };
    }

    // 路線種別ごとの (最高速度, 加速, 減速)
    let (line_max_speed, accel, decel) = match LineType::try_from(line_type) {
        Ok(LineType::BulletTrain) => (BULLET_TRAIN_MAX_SPEED, 0.72, 0.56),
        Ok(LineType::Subway) => (SUBWAY_MAX_SPEED, 0.83, 0.83),
        Ok(LineType::MonorailOrAgt) => (MONORAIL_OR_AGT_MAX_SPEED, 0.97, 0.69),
        Ok(LineType::Tram) => (TRAM_MAX_SPEED, 0.83, 0.69),
        // Normal / OtherLineType / Unspecified はすべて在来線標準扱い
        _ => (NORMAL_MAX_SPEED, 0.83, 0.69),
    };

    // kind 固有の上限があれば優先（上書き）
    let max_speed = match TrainTypeKind::try_from(kind) {
        Ok(TrainTypeKind::LimitedExpress) | Ok(TrainTypeKind::HighSpeedRapid) => {
            LIMITED_EXPRESS_KIND_MAX_SPEED
        }
        _ => line_max_speed,
    };

    SpeedProfile {
        max_speed,
        max_acceleration: accel,
        max_deceleration: decel,
    }
}
```

> **要確認（実装時）**: `useSimulationMode` の実コードで kind 上限の適用が「上書き」か「`min(lineMax, kindMax)`」かを最終確認する。
> 現状の読み取りでは `?? lineTypeMax`（kind 値があれば上書き）。`min` だった場合は `max_speed` の決定行を `line_max_speed.min(kindMax)` に変更する。

`stationapi/src/use_case/dto.rs`（モジュール宣言）に追記:

```rust
pub mod simulation;
```

### 2.2 `stationapi/src/use_case/traits/query.rs` に trait メソッド追加

```rust
async fn get_train_route(
    &self,
    from_station_group_id: u32,
    to_station_group_id: u32,
    line_group_id: u32,
) -> Result<Vec<crate::proto::TrainRouteSegment>, UseCaseError>;
```

### 2.3 `stationapi/src/use_case/interactor/query.rs` に実装追加

既存の `get_stations_by_line_group_id`（順序付き全駅を `proto::Station` で返す）と、
同ファイル内の private 関数 `haversine_distance` をそのまま再利用する。
import に `crate::use_case::dto::simulation::resolve_speed_profile` と
`crate::proto::{TrainRouteSegment, StopCondition}` を追加。

```rust
async fn get_train_route(
    &self,
    from_station_group_id: u32,
    to_station_group_id: u32,
    line_group_id: u32,
) -> Result<Vec<TrainRouteSegment>, UseCaseError> {
    // 1. 指定種別(line_group_id)の順序付き全駅（通過駅含む）を取得。
    //    proto::Station へ変換済みで、緯度経度・stop_condition・line(line_type)・train_type(kind) を持つ。
    let stations: Vec<proto::Station> = self
        .get_stations_by_line_group_id(line_group_id, TransportTypeFilter::RailAndBus)
        .await?;

    // 2. A駅 / B駅 の位置（index）を group_id で特定。
    let from_idx = stations
        .iter()
        .position(|s| s.group_id == from_station_group_id)
        .ok_or(UseCaseError::NotFound {
            entity_type: "station in line group",
            entity_id: from_station_group_id.to_string(),
        })?;
    let to_idx = stations
        .iter()
        .position(|s| s.group_id == to_station_group_id)
        .ok_or(UseCaseError::NotFound {
            entity_type: "station in line group",
            entity_id: to_station_group_id.to_string(),
        })?;

    // 3. A→B の向きに合わせて区間を切り出す（逆向きなら反転）。
    let mut sliced: Vec<proto::Station> = if from_idx <= to_idx {
        stations[from_idx..=to_idx].to_vec()
    } else {
        let mut v = stations[to_idx..=from_idx].to_vec();
        v.reverse();
        v
    };

    // 4. 各駅へ走行メタを付与。
    let mut segments: Vec<TrainRouteSegment> = Vec::with_capacity(sliced.len());
    let mut prev_coord: Option<(f64, f64)> = None;
    for station in sliced.drain(..) {
        let stops = station.stop_condition != StopCondition::Not as i32;

        let distance_from_previous = match prev_coord {
            Some((plat, plon)) => haversine_distance(plat, plon, station.latitude, station.longitude),
            None => 0.0,
        };
        prev_coord = Some((station.latitude, station.longitude));

        let line_type = station.line.as_ref().map(|l| l.line_type).unwrap_or(0);
        let kind = station.train_type.as_ref().map(|tt| tt.kind).unwrap_or(0);
        let profile = resolve_speed_profile(line_type, station.transport_type, kind);

        segments.push(TrainRouteSegment {
            station: Some(station),
            stops,
            distance_from_previous,
            max_speed: profile.max_speed,
            max_acceleration: profile.max_acceleration,
            max_deceleration: profile.max_deceleration,
        });
    }

    Ok(segments)
}
```

> 注: `get_stations_by_line_group_id` は内部で `station_repository.get_by_line_group_id(line_group_id)` を呼ぶため、
> 新規 SQL は不要。`sst.id` 昇順で並ぶ前提（既存クエリの ORDER BY）に依存する。

### 2.4 `stationapi/src/presentation/controller/grpc.rs` にハンドラ追加

import の `proto::{...}` に `GetTrainRouteRequest, TrainRouteResponse` を追加し、
`impl StationApi for MyApi` に下記を追加（`get_routes` と同形）。

```rust
async fn get_train_route(
    &self,
    request: tonic::Request<GetTrainRouteRequest>,
) -> Result<tonic::Response<TrainRouteResponse>, tonic::Status> {
    let req = request.get_ref();
    let from_id = req.from_station_group_id;
    let to_id = req.to_station_group_id;
    let line_group_id = req.line_group_id;

    match self
        .query_use_case
        .get_train_route(from_id, to_id, line_group_id)
        .await
    {
        Ok(segments) => Ok(Response::new(TrainRouteResponse { segments })),
        Err(err) => Err(PresentationalError::from(err).into()),
    }
}
```

### 2.5 テスト方針（`query.rs` の単体テスト）

既存のモックリポジトリ（`get_by_line_group_id` を実装）に種別 1 本ぶんの駅列を返させ、以下を検証:

- (a) `stops`: `pass=1`（`StopCondition::Not`）の駅が `false`、それ以外が `true`。
- (b) `distance_from_previous`: 先頭 0、以降は Haversine 正値。座標既知の 2 駅で期待値（±数 m）を確認。
- (c) `max_speed`: `kind=LimitedExpress` で 36.11、`line_type=BulletTrain` で 88.88、バスで 11.11。
- (d) 向き: `from_idx > to_idx` の入力で segments が反転して返る。
- (e) 端点未存在: line_group に含まれない group_id で `UseCaseError::NotFound`。

---

## 3. ドキュメント更新（proto マージ後）

`AGENTS.md` の「gRPC Endpoint Overview」に追記する案:

> - **Train route (simulation)** – `GetTrainRoute`. 指定した `line_group_id`（列車種別）について A→B 間の各駅を順序通り返し、
>   各駅に停車/通過フラグ・直前駅からの距離(m)・最高速度/加減速度(SI 単位)を付与する。MobileApp の `useSimulationMode`
>   が端末内で行っていた経路・速度プロファイル準備を 1 リクエストに集約するための endpoint。速度定数は
>   `use_case/dto/simulation.rs` に集約し、MobileApp の `simulationMode` 定数と一致させる。

---

## 4. 検証手順（proto マージ後）

1. `cargo fmt && cargo clippy --all-targets --all-features`（新規 clippy 警告を解消）。
2. `make test-unit`（上記 (a)〜(e) を含む）。
3. ローカル起動して `grpcurl`（reflection 有効）で疎通確認:
   ```
   grpcurl -plaintext \
     -d '{"from_station_group_id":<A>,"to_station_group_id":<B>,"line_group_id":<G>}' \
     localhost:50051 <package>.StationApi/GetTrainRoute
   ```
   - segments が A→B 順、先頭 `distance_from_previous=0`、通過駅 `stops=false`、特急で `max_speed≈36.11` を確認。
4. `make test-integration` が回帰していないこと。

## 5. 残課題・確認事項

- **kind 上限の適用規則**（上書き / `min`）を `useSimulationMode` 実コードで最終確認（§2.1 の注記）。
- **proto package 名**は実際の `stationapi.proto` の `package` 宣言に合わせる。
- gRPCProto への proto 反映（PR）はリポジトリ所有者側で実施し、StationAPI の submodule ポインタを更新する。
