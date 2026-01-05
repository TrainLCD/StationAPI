# 近傍バス停検索機能

鉄道駅から半径300m以内のバス停・バス路線を取得する機能の仕様。

## 概要

各APIで`transport_type`パラメータを使用して、鉄道駅・バス停のフィルタリングを制御できる。デフォルトでは後方互換性のため鉄道駅のみを返す。

## パラメータ

### TransportType

```protobuf
enum TransportType {
  TransportTypeUnspecified = 0;  // 鉄道駅のみ（デフォルト）
  Rail = 1;                       // 鉄道駅のみ
  Bus = 2;                        // バス停のみ
  RailAndBus = 3;                 // 鉄道駅 + バス停
}
```

## 動作仕様

| transport_type | 動作 |
|----------------|------|
| **未指定 / Unspecified** | 鉄道駅のみを返す（デフォルト） |
| **Rail** | 鉄道駅のみを返す |
| **Bus** | バス停のみを返す |
| **RailAndBus** | 鉄道駅とバス停の両方を返す。`lines`配列にも近傍バス路線を含める |

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
- **基準点**: 取得した鉄道駅の座標

## 使用例

### 鉄道駅のみを取得（デフォルト）

```protobuf
// transport_type未指定で鉄道駅のみを取得
GetStationByGroupIdRequest {
  group_id: 1130201
}

// または明示的にRailを指定
GetStationByGroupIdRequest {
  group_id: 1130201
  transport_type: Rail
}
```

### バス停のみを取得

```protobuf
GetStationByGroupIdRequest {
  group_id: 1130201
  transport_type: Bus
}
```

### 鉄道駅とバス停の両方を取得

```protobuf
GetStationByGroupIdRequest {
  group_id: 1130201
  transport_type: RailAndBus
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
/// 指定座標から半径300m以内のバス路線を取得
async fn get_nearby_bus_lines(&self, ref_lat: f64, ref_lon: f64) -> Result<Vec<Line>, UseCaseError>
```

## 注意事項

- バス路線検索は最大50件のバス停候補を取得し、その中から300m以内のものをフィルタリング
- 鉄道駅の`lines`配列に近傍バス路線が追加されるのは`transport_type: RailAndBus`を指定した場合のみ
