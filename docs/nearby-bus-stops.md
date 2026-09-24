# 近傍バス停検索機能

鉄道駅から半径 300m 以内にあるバス停を探し、そこを通るバス路線を駅の
`lines` に加える機能の仕様です。

## 概要

駅を返すクエリの多くは `transportType` 引数を受け付けます。この引数は
次の 2 つを決めます。

- 返す駅を、鉄道駅とバス停のどちらに絞るか
- 鉄道駅の `lines` に、近くのバス路線を加えるか

未指定のときは `RailAndBus` として扱い、鉄道駅とバス停の両方を返したうえで、
鉄道駅の `lines` に近くのバス路線を加えます。

近傍のバス停そのものが駅の一覧に加わるわけではありません。近くのバス停は、
鉄道駅の `lines` に加わったバス路線の `station` として返ります。

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

| transportType | 返す駅 | 鉄道駅の `lines` |
|----------------|------|------|
| **未指定 / TransportTypeUnspecified** | 鉄道駅とバス停 | 乗換路線に加え、近傍のバス路線 (`RailAndBus` と同じ) |
| **Rail** | 鉄道駅のみ | 鉄道路線のみ |
| **Bus** | バス停のみ | (鉄道駅は返らない) |
| **RailAndBus** | 鉄道駅とバス停 | 乗換路線に加え、近傍のバス路線 |

- 未指定と `TransportTypeUnspecified` は、どちらも `RailAndBus` と同じ扱いに
  なります。gRPC 版では未指定を `Rail` として扱っていましたが、Worker 版では
  バスを含めた結果を既定で返すよう、意図的に変えています。
- `lines` に加えたバス路線の `station` には、その路線で最も近いバス停が
  入ります。
- `Rail` と `Bus` では、`lines` もその種別の路線だけに絞られます。
- 近傍のバス路線を加えるのは鉄道駅だけです。バス停の `lines` には、その
  停留所を通るバス路線だけが入ります。

**注**: `stationsNearby` で種別を絞らない場合 (未指定・
`TransportTypeUnspecified`・`RailAndBus`) は、鉄道駅を先に、バス停を後に
返します。それぞれの中は距離の昇順です。`limit` は種別ごとではなく並べた後の
全体にかかるため、鉄道駅だけで `limit` 件そろう地点ではバス停は返りません。
`limit` を省略した場合は 1 件です。

## 対象API

`transportType` を受け付けるクエリと、その効き方は次のとおりです。

| クエリ | 返す駅の絞り込み | 近傍のバス路線 |
|-----|---------------|------|
| `station` | ✅ | ✅ |
| `stations` | ✅ | ✅ |
| `stationGroupStations` | ✅ | ✅ |
| `stationsNearby` | ✅ | ✅ |
| `stationsByName` | ✅ | ✅ |
| `lineListStations` | ✅ | ✅ |
| `lineGroupListStations` | ✅ | ✅ |
| `lineStations` | ❌ | ✅ |
| `lineGroupStations` | ❌ | ✅ |

- `lineStations` と `lineGroupStations` は路線 (系統) の停車駅をそのまま
  返すため、`transportType` で駅は絞りません。`lines` の絞り込みと近傍の
  バス路線の追加には、他のクエリと同じく `transportType` が効きます。
- `trainRoute` は `transportType` を受け付けませんが、常に `RailAndBus` と
  同じ扱いで、区間の各駅に近傍のバス路線を加えます。
- `stationGroupStations` などで鉄道駅の駅グループを指定した場合、
  `transportType: Bus` の結果は空になります。バス停の駅グループ
  (`station_g_cd`) は鉄道とは別の値域で生成しているため、鉄道駅と同じ
  グループには入りません。

## 距離計算

- **アルゴリズム**: haversine 公式 (地球を半径 6,371km の球とみなす)
- **半径**: 300m (定数 `NEARBY_BUS_STOP_RADIUS_METERS`)
- **基準点**: 取得した各鉄道駅の座標

候補の検索は駅グループごとに代表の座標 1 点で行います。同じ駅グループでも
路線ごとに座標が少しずつ違うため、検索半径は 300m に「代表の座標と各駅の座標の
最大の隔たり」を足したものにしています。そのうえで、採用するかどうかは駅ごとの
座標から 300m 以内かどうかで判定します。こうしないと、代表の座標からは半径の
外でも、同じグループの別の駅からは内側にあるバス停を取りこぼします。

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

駅グループは鉄道とバスで分かれているため、バス停だけを探すときは座標や
名前で検索します。

```graphql
query {
  stationsNearby(latitude: 35.619772, longitude: 139.728439, limit: 10, transportType: Bus) {
    id
    name
    transportType
  }
}
```

### 鉄道駅と近傍のバス路線を取得 (未指定時と同じ)

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
- `src/graphql/query.rs`: GraphQL のリゾルバ。`to_filter` で `transportType` を
  `TransportTypeFilter` に変換する
- `stationapi/src/use_case/interactor/query.rs`: ビジネスロジック。駅の絞り込みと
  近傍のバス路線の追加
- `src/repository.rs`: 近傍のバス停の検索 (`get_bus_stops_near_stations`)
- `src/index.rs`: グリッド索引による座標検索 (`within_radius`、`nearest`)

### 定数

```rust
// stationapi/src/use_case/interactor/query.rs
const NEARBY_BUS_STOP_RADIUS_METERS: f64 = 300.0;
```

### ヘルパーメソッド

```rust
// stationapi/src/use_case/interactor/query.rs
/// 駅に路線・事業者・駅番号・列車種別を付け、RailAndBus のときは近傍のバス路線を
/// `lines` に加える
async fn update_station_vec_with_attributes_inner(
    &self,
    mut stations: Vec<Station>,
    line_group_id: Option<u32>,
    transport_type: TransportTypeFilter,
    skip_types_join: bool,
    prefetched_group_stations: Option<Vec<Station>>,
) -> Result<Vec<Station>, UseCaseError>

/// 駅グループの座標ごとに、半径内のバス停を近い順に最大 limit_per_station 件返す
async fn get_bus_stops_near_stations(
    &self,
    coords: &[(u32, f64, f64)],
    limit_per_station: u32,
    radius_meters: f64,
) -> Result<Vec<(u32, Station)>, UseCaseError>
```

バス停の検索は `src/index.rs` の `within_radius` で行います。全件は走査せず、
バス停だけを載せたグリッド索引 (`Grid`、緯度経度 0.05° 四方のマス) で半径の
内側のマスだけを調べます。詳しくは
[アーキテクチャドキュメントの「座標による検索」](./architecture.md#座標による検索)
を参照してください。

## バス停の `has_train_types`

バス停も鉄道駅と同じく列車種別 (`TrainType`) を持ち、系統に属するバス停では
`Station.hasTrainTypes` が `true` になります。これは、バスのデータを取り込む
ときに、`(route_id, shape_id)` の運行パターンごとに次の 2 つを生成している
ためです。

- `types` の行 (`kind = TrainTypeKind::BusRoute (= 7)`)
- `station_station_types` の行

運行パターンとは、池86 でいえば一周する便・サンシャインシティ経由・短ターン
のような違いです。同じ系統の中で通る停留所の集合が同じ shape は、上下線と
みなして 1 つの種別にまとめ、`direction = Both` (双方向) として返します。

クライアントは `stationTrainTypes(stationId:)` でバス停の運行パターンを取得し、
「池袋駅東口 (循環)」「新宿伊勢丹前 ⇔ 池袋駅東口」のように切り替えて表示
できます。詳しくは
[アーキテクチャドキュメントの「バスのコード生成」](./architecture.md#バスのコード生成)
と、`preprocessor/src/gtfs/integrate.rs` の `trip_variations_to_types` を参照して
ください。

## 注意事項

- 候補のバス停は、駅グループごとに検索半径内のものを近い順に見て、有効な
  路線を持つものを最大 50 件まで採ります。バス停の行は停留所と系統の組ごとに
  あるため、この 50 件は行の数です。
- 近傍のバス路線は、採用したバス停の近い順に並びます。同じ路線は 1 回だけ
  加えます。
- 鉄道駅の `lines` に近傍のバス路線が加わるのは、`transportType` が未指定・
  `TransportTypeUnspecified`・`RailAndBus` のいずれかの場合です。
