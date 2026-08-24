# 近傍バス停検索機能

鉄道駅から半径300m以内のバス停・バス路線を取得する機能の仕様。

## 概要

各クエリの `transportType` 引数で、鉄道駅・バス停の絞り込みを制御できる。
未指定のときは鉄道とバスの両方を返す。

## パラメータ

### TransportType

```graphql
enum TransportType {
  TransportTypeUnspecified
  Rail
  Bus
  RailAndBus
}
```

## 動作仕様

| transportType | 動作 |
|----------------|------|
| **未指定 / TransportTypeUnspecified** | 鉄道駅とバス停の両方を返す |
| **Rail** | 鉄道駅のみを返す |
| **Bus** | バス停のみを返す |
| **RailAndBus** | 鉄道駅とバス停の両方を返す。`lines`配列にも近傍バス路線を含める |

**注**: `stationsNearby` の並びは常に近い順です。`transportType` を指定しない場合も鉄道とバスを分けず、距離だけで並べます。

## 対象API

| クエリ | 近傍バス停対応 | 備考 |
|-----|---------------|------|
| `station` | ✅ | |
| `stations` | ✅ | |
| `stationGroupStations` | ✅ | |
| `lineStations` | ❌ | 路線の停車駅のみ返す（`transportType` は無視） |
| `lineGroupStations` | ❌ | 路線の停車駅のみ返す（`transportType` は無視） |
| `stationsNearby` | ✅ | |
| `stationsByName` | ✅ | |

**注**: 路線系クエリ（`lineStations`、`lineGroupStations`）は路線の停車駅一覧を返すため、近傍バス停を混ぜる意味がありません。これらのクエリでは `transportType` は無視されます。

## 距離計算

- **アルゴリズム**: Haversine公式（地球の曲率を考慮）
- **半径**: 300メートル（定数 `NEARBY_BUS_STOP_RADIUS_METERS`）
- **基準点**: 取得した鉄道駅の座標

## 使用例

### 鉄道駅のみを取得

```graphql
query {
  stationGroupStations(groupId: 1130201, transportType: Rail) {
    id
    name
    transportType
  }
}
```

### バス停のみを取得

```graphql
query {
  stationGroupStations(groupId: 1130201, transportType: Bus) {
    id
    name
    transportType
  }
}
```

### 鉄道駅とバス停の両方を取得（未指定時と同じ）

```graphql
query {
  stationGroupStations(groupId: 1130201, transportType: RailAndBus) {
    id
    name
    transportType
    lines {
      id
      nameShort
      transportType
    }
  }
}
```

## 実装詳細

### 関連ファイル

- `schema/public.graphql`: 公開スキーマ
- `stationapi/src/use_case/interactor/query.rs`: ビジネスロジック
- `src/graphql/query.rs`: GraphQL リゾルバ
- `src/repository.rs`: 近傍バス停の検索 (インメモリ索引)

### 定数

```rust
// src/use_case/interactor/query.rs
const NEARBY_BUS_STOP_RADIUS_METERS: f64 = 300.0;
```

### ヘルパーメソッド

```rust
/// 指定座標から半径300m以内のバス路線を取得
async fn get_nearby_bus_lines(&self, ref_lat: f64, ref_lon: f64) -> Result<Vec<Line>, UseCaseError>
```

## バス停の `has_train_types`

バス停も鉄道駅と同様に `TrainType` を持ち、`Station.has_train_types` が `true` になります。これは GTFS インポート時に `(route_id, shape_id)` のバリエーション (循環ループ / 短ターン / サンシャインシティ経由など) ごとに `types` (`kind = TrainTypeKind::BusRoute (= 7)`) と `station_station_types` を生成しているためです。詳細は [`architecture.md` のバス統合節](./architecture.md) と `preprocessor/src/gtfs/integrate.rs` の `trip_variations_to_types` を参照してください。

クライアントは `GetTrainTypesByStationId` でバス停の系統バリエーションを取得し、UI 上で「池袋駅東口 (循環)」「新宿伊勢丹前 ⇔ 池袋駅東口」のように切り替え表示できます。なお、停留所集合が同じで方向だけが違う shape ペアは 1 つの TrainType に畳まれ、`direction = Both` (双方向) として返されます。

## 注意事項

- バス路線検索は300m以内のバス停を近い順に見て、有効な路線を持つものを最大50件採用
- 鉄道駅の `lines` 配列に近傍バス路線が追加されるのは、未指定または `transportType: RailAndBus` の場合
