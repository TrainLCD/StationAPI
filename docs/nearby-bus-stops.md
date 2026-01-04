# 近傍バス停検索機能

鉄道駅から半径300m以内のバス停を同一グループとして返す機能の仕様。

## 概要

各APIで`transport_type`パラメータを使用して、鉄道駅に加えて近くのバス停を含めるかどうかを制御できる。

## パラメータ

### TransportType

```protobuf
enum TransportType {
  TransportTypeUnspecified = 0;  // 鉄道駅 + 近くのバス停を含める
  Rail = 1;                       // 鉄道駅のみ
  Bus = 2;                        // バス停のみ
}
```

## 動作仕様

| transport_type | 動作 |
|----------------|------|
| **未指定 / Unspecified** | 鉄道駅を取得し、最初の鉄道駅から半径300m以内のバス停も追加して返す |
| **Rail** | 鉄道駅のみを返す |
| **Bus** | 最初の鉄道駅から半径300m以内のバス停のみを返す |

## 対象API

| API | 近傍バス停対応 | 備考 |
|-----|---------------|------|
| `GetStationById` | ✅ | |
| `GetStationByIdList` | ✅ | |
| `GetStationsByGroupId` | ✅ | |
| `GetStationsByLineId` | ❌ | 路線の停車駅のみ返す（`transport_type`は無視） |
| `GetStationsByLineGroupId` | ❌ | 路線の停車駅のみ返す（`transport_type`は無視） |
| `GetStationsByCoordinates` | ✅ | |
| `GetStationsByName` | ✅ | |

**注**: 路線系API（`GetStationsByLineId`、`GetStationsByLineGroupId`）は、路線の停車駅一覧を返すため、近傍バス停を混在させることは意味がありません。これらのAPIでは`transport_type`パラメータは無視されます。

## 距離計算

- **アルゴリズム**: Haversine公式（地球の曲率を考慮）
- **半径**: 300メートル（定数 `NEARBY_BUS_STOP_RADIUS_METERS`）
- **基準点**: 取得した鉄道駅の最初の1件の座標

## 使用例

### 鉄道駅 + 近くのバス停を取得

```protobuf
// transport_type未指定で鉄道駅と近くのバス停を両方取得
GetStationByGroupIdRequest {
  group_id: 1130201
}
```

### 鉄道駅のみを取得

```protobuf
GetStationByGroupIdRequest {
  group_id: 1130201
  transport_type: Rail
}
```

### 近くのバス停のみを取得

```protobuf
GetStationByGroupIdRequest {
  group_id: 1130201
  transport_type: Bus
}
```

## 実装詳細

### 関連ファイル

- `proto/stationapi.proto`: リクエスト定義
- `src/use_case/interactor/query.rs`: ビジネスロジック
- `src/presentation/controller/grpc.rs`: gRPCコントローラー

### 定数

```rust
// src/use_case/interactor/query.rs
const NEARBY_BUS_STOP_RADIUS_METERS: f64 = 300.0;
```

### ヘルパーメソッド

```rust
/// 指定座標から半径300m以内のバス停を取得
async fn get_nearby_bus_stops(&self, ref_lat: f64, ref_lon: f64) -> Result<Vec<Station>, UseCaseError>
```

## 注意事項

- バス停検索は最大50件の候補を取得し、その中から300m以内のものをフィルタリング
- 鉄道駅が存在しない場合、`transport_type: Bus`は空の結果を返す
- 複数の鉄道駅がある場合、最初の1件の座標を基準点として使用
