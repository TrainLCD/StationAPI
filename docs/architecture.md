# StationAPI アーキテクチャドキュメント

> 最終更新: 2026年9月24日

## 目次

- [概要](#概要)
- [全体構成](#全体構成)
- [レイヤー構造](#レイヤー構造)
- [データパイプライン](#データパイプライン)
- [インメモリ索引](#インメモリ索引)
- [乗換経路探索](#乗換経路探索)
- [GraphQL とスキーマ一致の担保](#graphql-とスキーマ一致の担保)
- [命名規則](#命名規則)
- [データフロー](#データフロー)
- [ディレクトリ構造](#ディレクトリ構造)
- [運用](#運用)
- [関連ドキュメント](#関連ドキュメント)

---

## 概要

日本の鉄道駅とバス停の情報を返す GraphQL API です。Cloudflare Workers 上で
動作し、データはビルド時に WASM バイナリへ埋め込んで配布します。
**サーバープロセスもデータベースもありません。**

以前は、オンプレミスのサーバーが PostgreSQL からデータを読んで gRPC-Web で
返し、クライアントは BFF (TrainLCD/BFF) が GraphQL に変換したものを使って
いました。gRPC-Web を使い続ける理由がなかったため、GraphQL を直接返す構成に
改め、BFF も廃止しました。

### 技術スタック

| 用途 | 採用しているもの |
|---|---|
| 実行環境 | Cloudflare Workers (wasm32-unknown-unknown) |
| API | GraphQL ([async-graphql](https://github.com/async-graphql/async-graphql) 7) |
| データ | ビルド時に WASM へ埋め込む CSV (`generated/*.csv`) |
| データ生成 | `preprocessor` crate (Rust のみで実装) |
| デプロイ | wrangler |

データベースを使わないので、`sqlx` も接続プールもありません。検索はすべて、
埋め込みデータから isolate の中に組み立てたインメモリ索引に対して行います。

---

## 全体構成

```txt
  data/*.csv          GTFS (ZIP)        ODPT (JSON)
  鉄道の正本データ      バス 6 フィード      東急バス
      │                    │                 │
      └────────────────────┴─────────────────┘
                           │
                 ┌─────────▼──────────┐
                 │   preprocessor     │  Rust のみで実装。各駅停車の
                 │   (ビルド時ツール)  │  系統生成と GTFS の統合を行う
                 └─────────┬──────────┘
                           │
                    generated/*.csv     7 テーブル
                           │
                 ┌─────────▼──────────┐
                 │  build.rs          │  CSV を OUT_DIR に配置し、
                 │                    │  sst を固定長バイナリに変換
                 └─────────┬──────────┘
                           │
                 ┌─────────▼──────────┐
                 │  worker-build      │
                 └─────────┬──────────┘
                           │
                        WASM ────────► Cloudflare Workers
                                          │
                                     GraphQL (POST /)
                                          │
                                       TrainLCD
```

データは WASM に埋め込まれるため、**データを更新するたびに再デプロイが
必要です。** 起動時にデータを読み込んで自動的に反映する仕組みではありません。

---

## レイヤー構造

```txt
┌──────────────────────────────────────────────┐
│ Presentation (src/graphql/)                   │  async-graphql のリゾルバと型
│   Query / 型 / enum / スカラー                 │
├──────────────────────────────────────────────┤
│ Model (stationapi/src/model.rs)               │  API が返す値の型
├──────────────────────────────────────────────┤
│ UseCase (stationapi/src/use_case/)            │  クエリの処理、
│   QueryInteractor / DTO 変換                   │  IPA・TTS の生成
├──────────────────────────────────────────────┤
│ Domain (stationapi/src/domain/)               │  エンティティ、経路探索、
│   entity / repository トレイト / 速度表        │  到着時刻の推定、正規化
├──────────────────────────────────────────────┤
│ Index (src/index.rs, src/repository.rs)       │  埋め込みデータの索引と
│                                               │  repository トレイトの実装
└──────────────────────────────────────────────┘
```

`stationapi` crate は Domain / UseCase / Model だけを含むライブラリで、
Worker と preprocessor の両方から使われます。wasm32 向けにビルドできなければ
ならないため、I/O を伴う依存は追加しません。

### Domain 層 (`stationapi/src/domain/`)

エンティティ、リポジトリの抽象、そして純粋な計算処理 (haversine 距離、
経路探索、到着時刻の推定、速度表、ローマ字・IPA 変換、検索用の正規化) を
置いています。

### UseCase 層 (`stationapi/src/use_case/`)

`QueryInteractor` が repository トレイトを通してデータを集め、駅に路線・
事業者・列車種別の情報を付け加えます。N+1 問題を避けるため、関連データは
常にまとめて取得します。

DTO (`use_case/dto/`) はドメインエンティティを Model に変換します。IPA と
TTS セグメントもここで生成します。

### Model 層 (`stationapi/src/model.rs`)

API が返す値を表す型です。もともとは prost が `.proto` から生成していた型
です。gRPC をやめた後も、IPA・TTS の生成がこの型への変換処理に組み込まれて
いるため、ドメインエンティティと GraphQL 型の間の層として残しています。

### Presentation 層 (`src/graphql/`)

`async-graphql` の Query リゾルバと型定義を置いています。Model を GraphQL 型に
変換するのもこの層です。

### Index 層 (`src/index.rs`, `src/repository.rs`)

埋め込まれた CSV は、isolate の中で最初に必要になったときに一度だけパースし、
`OnceLock` に保持します。`src/repository.rs` は 4 つの repository トレイトを実装しているので、
UseCase 層からはデータベースを使っていた頃と同じインターフェースに見えます。

---

## データパイプライン

`data/*.csv` をそのまま Worker に渡すと、**本番とは異なる挙動になります。**

- 列車種別を持たない路線には、各駅停車の系統を補う必要があります (約 2,400 行。
  有効な駅の約 21% が該当します)。これらの行は `data/*.csv` には含まれて
  いません
- バス停・バス路線・バス系統は、GTFS と ODPT の JSON から生成する必要が
  あります

これらを担うのが `preprocessor` crate です。

```bash
make data     # cargo run --profile tool -p stationapi-preprocessor
```

処理の流れは次のとおりです。

1. `data/*.csv` を読み込む (`#` で始まる列は読み込まない)
2. 各駅停車の系統を生成する (`generate_virtual_local_rail_services`)
3. GTFS フィード 6 本を取得・展開して読み込む (都営バス・西武バス・京王バスと、
   東急バスが運行する大田区・品川区・目黒区のコミュニティバス)
4. 東急バスの ODPT JSON を読み込む (7 日間キャッシュする)
5. バスのデータを lines / stations / types / station_station_types に統合する
6. `generated/*.csv` に書き出す (7 テーブル)

取得や読み込みに失敗したフィードは、警告を出して飛ばします。ただし、GTFS の
路線を 1 つも取り込めなかった場合は、バスが丸ごと欠けたデータを出さないよう
失敗させます。デプロイ用のワークフローでは、フィードを 1 つでも飛ばした時点で
失敗させています (`fail-on-missing-bus-feeds`)。

`station_station_types.id` はそのまま停車順として使われるため、行の順序に
意味があります。書き出すときは必ず `id` の昇順に並べます。

### バスのコード生成

バス由来のレコードには、FNV-1a ハッシュを使って、鉄道と重ならない値域の
コードを決定的に割り当てます。実行するたびに同じ値にならなければならない
ので、`DefaultHasher` は使いません。

| 対象 | 値域 | ハッシュの入力 |
|---|---|---|
| `line_cd` | 100,000,000〜 (1,000 万件分) | `route_id` |
| `station_cd` | 200,000,000〜 (1 億件分) | `(stop_id, route_id)` |
| `station_g_cd` | 200,000,000〜 (1 億件分) | `stop_id` (上下線などのポールをまとめた代表の停留所) |
| `type_cd` | 100,000,000〜 (1 億件分) | `(route_id, shape_id)` |
| `line_group_cd` | 100,000,000〜 (1 億件分) | `(route_id, shape_id)` |

ハッシュなので、別々の入力が同じ値になることがあります (実データでも、都営
バスと西武バスの停留所で `station_cd` が 1 件衝突していました)。`station_g_cd`
以外は、衝突したら次の空き値にずらして必ず一意にします。`station_g_cd` は
停留所をまとめるためのキーで一意である必要がないため、ずらしません。

### 環境変数

| 変数 | 効果 |
|---|---|
| `ODPT_ACCESS_TOKEN` | 都営バス以外のフィードの取得に必要。未設定の場合は、展開済みの GTFS (`data/*-GTFS/`) と 7 日以内の ODPT JSON のキャッシュだけを使い、どちらもないフィードは警告を出して飛ばす |
| `DISABLE_BUS_FEATURE` | `true` (または `1`) にするとバスを取り込まない (鉄道のみ) |

---

## インメモリ索引

PostgreSQL のクエリは、次のように置き換えています。

| PostgreSQL | Worker |
|---|---|
| `point(lat,lon) <-> point()` | グリッド索引 (`Grid`) で探索半径の内側だけを調べ、haversine で距離を計算する |
| `pg_trgm` の GIN インデックス | 全件走査と `contains()` |
| `station_station_types` の JOIN | `HashMap` による索引 |

### 座標による検索

座標による検索は、`src/index.rs` の 2 つの関数で行います。

- `nearest`: 近い順に `limit` 件を返します (`stationsNearby`)
- `within_radius`: 半径内のものをすべて返します (鉄道駅に近傍のバス停を
  付けるとき)

どちらも全件は走査せず、鉄道とバスで別々に作ったグリッド索引 (`Grid`) を
使います。グリッドは緯度経度 0.05° (約 5.5km) 四方のマスで、マスごとの駅を
CSR 形式で持ちます。

`nearest` は半径 1km から探し始め、半径の内側に `limit` 件そろわなければ
半径を 4 倍ずつ広げます。半径の内側に `limit` 件そろえば、その外側の駅が
上位 `limit` 件に入ることはないためです。索引の範囲全体を覆っても足りない
場合は、その時点の結果を返します。`transportType` を省略した場合は、移行前の
SQL (`ORDER BY transport_type, distance`) と同じく、鉄道駅を先に、バス停を
後に並べます。件数の上限は並べた後の全体にかかるので、まず鉄道駅で埋め、
残りの枠の分だけバス停を探します。距離が同じ場合は `station_cd` の順に
並べ、結果が不安定なソートに左右されないようにしています。

鉄道駅を返すクエリでは、近傍のバス停を付けるために駅ごとに座標検索が
走ります。座標による検索を追加するときは、全件走査ではなくこのグリッド
索引を使ってください。

### 名前による検索

名前による検索 (`search_by_name`) は、駅名・読み・ローマ字・中国語・韓国語の
いずれかに部分一致する駅を、全件走査で探します。`pg_trgm` は
`LIKE '%...%'` を高速化するためのインデックスで、類似度検索に使っていた
わけではありません。そのため `contains()` でも論理的に同じ結果になります。
正規化には domain 層の `normalize_for_search` をそのまま使います。

### 停車駅 (`station_station_types`)

`station_station_types.csv` は鉄道だけでも 4 万行を超え (`data/` で 41,976
行)、バスを含めるとさらに増えます。CSV のパースがコールドスタート時間の
大半を占めていたので、`build.rs` で 1 行を `i32` 4 つ (`station_cd`、
`type_cd`、`line_group_cd`、`pass`) の固定長バイナリ (`sst.bin`) に事前変換して
います。`id` は保存せず、読み込み時に行順で 1 から振り直します。

---

## 乗換経路探索

`connectedRoutes` は、乗換案内アプリと同じように、乗換を含む経路を自動で
探します。`routes` / `routeTypes` が出発駅と到着駅の両方に停車する系統だけを
返すのに対し、`connectedRoutes` は複数の系統を乗り継ぐ経路を返します。
実装は `stationapi/src/domain/route_search.rs` (純粋なロジック) にあります。
探索アルゴリズムの内部設計 (データ構造、走査の計算式、枝刈り、代替経路の
生成、決定性) は [乗換経路探索 (RAPTOR) の設計](./route-search.md) に
まとめています。

### レスポンスの形

アプリは列車 (系統) 1 本ごとに、「種別を選ぶ → `lineGroupStations` で系統
全体の駅を取得する → LCD 表示を動かす」という流れで動作します。乗換経路も
この流れで扱えるよう、経路は区間 (`legs`) のリストとして返します。各区間は、
その区間で乗車できる種別 (`trainTypes`。`routeTypes` と同じ形で、実在する
`groupId` を持つ) と、乗車駅・降車駅 (`Station`) を持ちます。乗車駅・降車駅は
探索で使った系統が走る路線の駅なので、乗換駅では、前の区間の降車駅と次の
区間の乗車駅が、同じ駅グループに属する別の駅になることがあります
(例: 丸ノ内線の赤坂見附 → 半蔵門線の永田町)。

```graphql
connectedRoutes(fromStationGroupId: Int!, toStationGroupId: Int!, viaLineId: Int, sortBy: ConnectedRouteSort): [ConnectedRoute!]!

enum ConnectedRouteSort { Recommended  ArrivalTime  TransferCount }

type ConnectedRoute { legs: [RouteLeg!] }
type RouteLeg { trainTypes: [TrainType!]  fromStation: Station  toStation: Station  stationGroupIds: [Int!] }
```

`stationGroupIds` は、乗車駅から降車駅までの駅グループ ID を進行順に並べた
もの (通過駅を含む) で、探索で使った系統の駅の並びをそのまま返します。
駅 ID ではなく駅グループ ID にしているのは、アプリが区間ごとに選ぶ種別
(既定は各駅停車) が、探索で使った系統とは別の路線を走ることがあるためです。
駅 ID は路線ごとに異なりますが、駅グループ ID ならどの種別の駅リストとも
照合できます。アプリはこの並びに沿って、選んだ種別の駅リストから駅を
取り出すので、環状線でどちら回りにするかをアプリ側で判断する必要は
ありません。同じ駅グループが 2 回現れる系統 (大江戸線の都庁前) では、
直前に取り出した駅に隣接するほうを選びます。

探索では、停車駅が同じ並行種別 (中央線の快速と通勤快速など) を 1 つの経路に
まとめ、代替経路を再探索するときもそれらをまとめて除外します。そのままでは
並行種別がレスポンスに一度も現れず、種別一覧 (TrainTypeListModal) を表示して
各駅停車を既定で選ぶという、現在のアプリの挙動を維持できません。そこで
`trainTypes` には、その区間で乗車できる種別をすべて返します。中身は
`routeTypes(乗車駅グループ, 降車駅グループ, 降車駅の路線)` の結果と同一で、
停車駅が同じ種別の集約、路線の付与、並び順も `routeTypes` と同じです (同じ
関数を呼び出しています)。探索が選んだ代表の種別、推定所要時間、乗換回数は
並べ替えにだけ使います。アプリでは使わないため、API では返しません。

`viaLineId` は `routeTypes` と同様に、検索結果でタップした駅の路線を指します。
指定すると、目的地にその路線の駅で到着する経路 (最後の区間がその路線を走る
経路) だけに絞り込みます。

`sortBy` は経路の並び順です。推定所要時間と乗換回数は API で返さないので、
並べ替えはサーバー側で行います。どれを選んでも返す経路の集合は変わらず、
並びだけが変わります。

| `sortBy` | 並び |
|---|---|
| `Recommended` (省略時) | おすすめ順。「評価値 (最初の列車の待ち時間を含む所要時間の見込み) + 乗換 1 回につき 5 分」の小さい順 (後述) |
| `ArrivalTime` | 到着の早い順。同じなら乗換の少ない順 |
| `TransferCount` | 乗換の少ない順。同じなら到着の早い順 |

到着の早い順は、`estimateArrivalTimes` と同じ定義の所要時間 (最初の列車の
待ち時間は含めず、乗換ごとに徒歩時間と乗換先の待ち時間を加える) で並べます。
ただし、並べ替えに使うのは探索が選んだ代表の種別での見込みです。アプリが
区間ごとに別の種別 (既定の各駅停車など) を選んで `estimateArrivalTimes` で
求めた到着見込みとは、並びが一致しないことがあります。キーが同じ経路同士は、
おすすめ順のまま並びます。

### 系統網

系統 (`line_group_cd`) ごとの停車駅の並びを 1 つのパターンとし、駅グループ
(`station_g_cd`) を乗換のノードとします。駅間の所要時間には
`arrival_estimation` の推定値を使います。環状線は 2 周分に展開し、継ぎ目を
またぐ乗車も同じ配列で計算できるようにしています。対象は鉄道だけで、バスは
含みません。

網の構築には全系統の駅と所要時間の推定が必要で、ネイティブ環境で約 190ms
かかります (`data/*.csv` の 1,185 系統・41,706 行。時間の半分が `Station` の
組み立て、残り半分が推定)。そのため起動時には構築せず、最初に
`connectedRoutes` が呼ばれたときに `OnceLock` に構築し、isolate が生きている
間は使い回します。

### 探索 (RAPTOR)

時刻表のデータを持たないため、運行頻度に基づく RAPTOR で探索します。
ラウンド k では「k 本目の列車に乗った時点」での最良値を求め、前のラウンドで
値が改善した駅を通るパターンだけを走査します。各ラウンドでの目的地の値が、
そのまま「k 本以内の乗車で到達できる最良値」になるので、所要時間と乗換回数の
パレート解が得られます。

評価値 (秒) は次の合計です。

| 項目 | 値 |
|---|---|
| 乗車時間 | `arrival_estimation` の推定値 (停車時間・通過を含む) |
| 乗車ごとの待ち時間 | 特急・新幹線 15 分 / 急行・新快速クラス 5 分 / その他 3 分 |
| 乗換の徒歩時間 | 3 分 |

待ち時間を加えないと、本数の少ない特急が「直通で速い」と評価され、
東京 → 渋谷で山手線より成田エクスプレスを勧めてしまいます。

### 代替経路と並び順

パレート解には、最速の経路と乗換が最も少ない経路しか含まれません。そこで、
見つかった経路の区間を 1 つずつ禁止して再探索します (Yen の k 最短経路
アルゴリズムの簡略版。探索は初回を含めて最大 8 回)。禁止するのはその区間の並行系統 (同じ
乗車駅と降車駅に停車する快速・各停など) で、区間内のどの駅からも乗車でき
ないようにします。乗車駅だけを禁止すると、別の列車で 1 駅進んでから同じ
新幹線に乗る経路が出てきてしまうためです。

順位は「評価値 + 乗換 1 回につき 5 分」で決め、最大 6 件を返します。代替
経路のうち、この順位の値が最良の経路の 1.15 倍 + 15 分を超えるものと、
乗換回数がパレート解の最多乗換回数より 2 回以上多いものは捨てます。乗車
本数を増やしても順位の値が良くならないパレート解 (1 分縮めるために乗換を
重ねる経路) も捨てます。

代替経路のうち、別々の区間で同じ駅グループに停車するものも捨てます。区間を
禁止して再探索すると、「1 駅戻って同じ列車に乗り直す」逆戻りの経路が代替
経路として出てくるためです (例: 大宮から土呂へ行き、大宮に停車する列車で
東京へ向かう)。通過した駅は数えません (急行で通過した駅に、先の駅から
戻るのは実際にある乗り方です)。1 つの区間の中で同じ駅に 2 回停車するのは
実在する運行 (大江戸線の都庁前など) なので、除外しません。初回探索の
パレート解 (最適解) にはこの除外を適用しません。適用すると、逆戻りでしか
到達できない駅が、`stationsByName` では「行ける駅」として返るのに、経路が
0 件になってしまうためです。

### 到着見込みと走行区間 (`estimateArrivalTimes` / `trainRoute`)

どちらも `legs: [RouteLegInput!]` を受け取り、乗換経路全体を通した値を
返します。`legs` には、`connectedRoutes` の各区間の `trainTypes` から選んだ
種別の `groupId` と、その区間の `fromStation.id`・`toStation.id` を渡します。
経路 ID は存在しないので、クライアントが経路を区間の並びとして渡します。

選んだ種別が、区間の乗車駅・降車駅とは別の路線の駅に停車することが
あります (たとえば乗降駅は中央線快速の三鷹だが、選んだ各駅停車は中央・
総武線の三鷹に停車する、など)。そのため、系統に含まれない乗降駅は、同じ
駅グループの駅で代用します。`station_cd` が一致する駅があればそれを使い、
同じ駅グループの候補が複数ある場合は、区間が最も短くなる組み合わせを選び
ます。直通系統は、接続駅に同じ駅グループの駅を 2 行持つためです (例:
宇都宮線の上野と上野東京ラインの上野)。

```graphql
input RouteLegInput { lineGroupId: Int!  fromStationId: Int!  toStationId: Int! }
```

- `estimateArrivalTimes`: 各区間を指定された系統だけで推定し (両駅に停車
  する別の系統は使いません)、出発駅からの累積時間でつないだ 1 本の経路を
  返します (系統をまたぐので `id` は空です)。乗換駅は、前の区間の降車駅と
  次の区間の乗車駅の 2 行になります。乗車駅の行には、「徒歩 3 分後に到着し、
  乗換先の種別の待ち時間が経ってから出発する」値が入ります。この見込みは、
  `connectedRoutes` の並べ替えに使うもの (徒歩 3 分と種別ごとの待ち時間) と
  同じです。
- `trainRoute`: 区間ごとの走行区間を順につなげます。区間ごとに別の列車
  なので、各区間の最初の駅の `distanceFromPrevious` は 0 になり、通過駅が
  あるか (優等種別の速度を使うか) も区間ごとに判定します。

区間の切り出しには両者で同じ関数を使い、環状線では継ぎ目をまたぐ短いほうの
弧を選ぶので、両者の駅の並びは一致します (`lineGroupId` を指定した
`trainRoute` は、従来どおり格納順で切り出します)。次のいずれかに当てはまる
場合はエラーになります。

- 区間がつながっていない (前の区間の降車駅と次の区間の乗車駅が、別の
  駅グループにある)
- 区間の数が 6 (`MAX_RIDES`。`connectedRoutes` が返しうる最大の乗車回数) を
  超える
- 両端の駅が `fromStationId` / `toStationId` と一致しない
- `viaLineIds`・`directionId`・`lineGroupId` と同時に指定されている

### 行き先の検索 (`stationsByName`)

`stationsByName` に `fromStationGroupId` を指定すると、その駅から行ける駅だけに
絞り込みます。返すのは次の駅です。

- 出発駅と系統を共有する駅 (直通で行ける駅)
- どちらかが系統を持たない場合は、同じ路線の駅
- 乗り換えれば、その駅の路線の列車で到着できる鉄道駅

3 つ目は、「`viaLineId` にその駅の路線を指定して `connectedRoutes` を呼ぶと
経路が返る駅」と一致させています。これらの駅は出発駅と系統を共有しないので
`line_group_cd` は空、`hasTrainTypes` は偽になり、直通で行ける駅と区別
できます。

判定には、`connectedRoutes` と同じ系統から作った**所要時間を持たない網**
(`stationapi/src/domain/route_topology.rs` の `RouteTopology`) を使います。
必要なのは「どの系統がどの駅に停車するか」だけなので、`Station` の組み立ても
所要時間の推定もせずに索引から直接構築でき、構築時間は約 20ms (所要時間
つきの網の約 1/10) です。`RouteNetwork` も内部に同じ網を持っており、系統の
整形 (`trim_pattern`) と駅の選び方 (`line_group_rows`) を共有しています。
両者が一致することは、実データを使ったテストで確認しています。

判定では、乗車 6 本以内の範囲で系統を幅優先探索します (1 回あたり数 ms)。
ただし `connectedRoutes` の探索は、目的地の駅グループで途中下車して乗り継ぐ
経路を作りません。そのため単純な幅優先探索では、「支線の分岐駅に、いったん
支線へ出てから戻ってきて到着する」経路まで数えてしまいます (箕面線で
石橋阪大前に着く、函館本線で新函館北斗に着く、など。実データでは 0.3〜0.7%
の駅が該当します)。これが起きるのは、目的地が「駅と系統からなる二部グラフ」
の関節点である場合に限られます。そこで網の構築時に関節点を求めておき
(Tarjan のアルゴリズム)、該当する駅についてだけ、目的地で途中下車しない
幅優先探索で改めて確認します。導入時には、実データの 3 つの出発駅について
それぞれ 1,500 駅を突き合わせ、`connectedRoutes` の探索結果と食い違いが
ないことを確認しました。網の構築後であれば、`stationsByName` は 100 件ヒットする
場合でも 2〜4ms で応答します。

### 計算量

1 回の探索の計算量は O(乗車回数 × 走査したパターンの駅数) です。旧実装
(列車種別と降車駅のすべての組み合わせを、乗換回数の分だけ列挙する深さ制限
つきの BFS) との比較は次のとおりです (ネイティブの release ビルド、
`data/*.csv` を使用)。

| 区間 | 旧実装 | 新実装 (探索のみ) |
|---|---|---|
| 東京 → 渋谷 | 370ms | 11〜13ms |
| 三鷹 → 中目黒 | 173ms (0 件) | 13〜18ms |
| 大宮 → 新大阪 | 349ms | 31〜36ms |
| 仙台 → 博多 | 693ms | 14〜20ms |

時刻表、運転間隔、駅グループをまたぐ徒歩連絡のデータがない
(`8!connections.csv` は空) ため、待ち時間は種別から見積もった値です。
季節運行の臨時列車も、通常の系統と同じように扱います。

---

## GraphQL とスキーマ一致の担保

クライアントとの互換性を保つため、エンドポイントはサブドメインのルートで
クエリを受け付けます。

| パス | 内容 |
|---|---|
| `POST /` | クエリの実行 |
| `GET /` | GraphiQL |
| `GET /__schema` | SDL (CI が取得して突き合わせる) |
| `GET /__health` | 索引の件数 |
| `GET /__ping` | データに触れない疎通確認 |

`async-graphql` はコードファーストなので、Rust の型を変更すると SDL も
変わります。クライアントを壊す変更に気付けるよう、`schema/public.graphql` を
正として `scripts/compare_schema.py` で突き合わせ、差分があれば CI を失敗
させます。型とフィールドは集合として比較し、enum は順序も含めて比較します。

スキーマを意図的に変更するときは、このファイルも更新します。その差分が、
そのままクライアントへの影響範囲になります。

実装上の注意:

- `async-graphql` は既定で enum の値を SCREAMING_SNAKE_CASE にする。公開
  スキーマは PascalCase なので、`rename_items` で合わせている
- PascalCase に変換すると `JR` が `Jr` になるため、この値だけは `name` を
  明示している
- `Station` と `StationNested` のように、構造が同じで名前だけが違う型は、
  SDL を合わせるためにマクロで両方を定義している。Nested 型は互いに参照
  し合うので、`Box` で間接参照にしないと無限サイズの型になる

---

## 命名規則

同じ「駅」を表す型が、層ごとに 3 つあります。

| 種別 | 場所 | 目的 | 特徴 |
|---|---|---|---|
| **Record** | `src/index.rs` | 埋め込み CSV の 1 行 | 検索に必要な列だけを持つ軽量な構造体 |
| **Entity** | `stationapi/src/domain/entity/` | ドメインモデル | ネスト構造、多言語対応、約 65 フィールド |
| **Model** | `stationapi/src/model.rs` | API が返す値 | 列挙型を `i32` のまま保持する |

### Record 構造体

```rust
// src/index.rs
pub struct StationRecord {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub name: String,
    // 検索に使う列だけを持つ。レスポンス用の Station は必要になった時点で組み立てる
}
```

名前による検索はリクエストのたびに全件走査するので、索引には `Station`
エンティティ (65 フィールド) を持たせず、レスポンスを生成するときにだけ
組み立てます。ローマ字名の小文字版のように、比較のたびに計算するとコストが
かかる値は、索引の構築時に計算して持っておきます。

### Entity 構造体

```rust
// stationapi/src/domain/entity/station.rs
pub struct Station {
    pub station_cd: i32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub station_numbers: Vec<StationNumber>,
    // ...
}
```

- ビジネス上の意味を反映した型 (`StopCondition` 列挙型など) を使う
- 多言語の名称: `station_name_r` (ローマ字)、`station_name_zh`、`station_name_ko`

### 変換フロー

```txt
generated/*.csv
    ↓  起動時に一度だけパース
Record (StationRecord)
    ↓  to_entity(): 路線の属性を埋める
Entity (Station)
    ↓  UseCase 層でネストしたデータを付与
Enriched Entity
    ↓  DTO 変換: IPA / TTS セグメントを生成
Model (model::Station)
    ↓  From による変換
GraphQL 型
```

---

## データフロー

### 典型的なリクエストの流れ

```txt
[Client]
    │
    ▼ POST / (GraphQL)
┌──────────────────────────────────────────────┐
│ Presentation (src/graphql/query.rs)           │
│  └─ Query::station()                          │
└──────────────────────────────────────────────┘
    │
    ▼ QueryUseCase のメソッド呼び出し
┌──────────────────────────────────────────────┐
│ UseCase (use_case/interactor/query.rs)        │
│  ├─ QueryInteractor::find_station_by_id()     │
│  └─ update_station_vec_with_attributes()      │
│      ├─ 同じ駅グループの駅を一括取得            │
│      ├─ 路線を一括取得                          │
│      ├─ 近傍のバス停を一括取得                  │
│      ├─ 事業者を一括取得                        │
│      └─ 列車種別を一括取得                      │
└──────────────────────────────────────────────┘
    │
    ▼ repository トレイト経由
┌──────────────────────────────────────────────┐
│ Index (src/repository.rs, src/index.rs)       │
│  └─ MemStationRepository::find_by_id()        │
│      └─ HashMap 参照                          │
└──────────────────────────────────────────────┘
    │
    ▼ Record → Entity 変換
    ▼ Entity → Model 変換 (use_case/dto/)
    ▼ Model → GraphQL 型
[Client]
```

一括取得は N+1 問題を避けるためのもので、データベースを使っていた頃から
変えていません。インメモリであっても、駅ごとに索引を引き直すより、一度に
まとめて集めるほうが素直な実装になります。

### エラーの伝播

```txt
DomainError
    ↓ ? 演算子 (From トレイト)
UseCaseError
    ↓ ? 演算子 (Display を実装した型を受け取る async-graphql の汎用 From 実装)
async_graphql::Error
    ↓
GraphQL の errors フィールド
```

repository の実装がないメソッドは、空の結果ではなく `DomainError` を返す
方針です (現在は、`StationRepository::get_route_network` の既定実装がこれに
あたります)。黙って空の結果を返すと正常な応答に見えてしまい、実装漏れに
気付けないためです。移行時には、実際にこの方針のおかげで実装漏れを 1 件
検出できました。

---

## ディレクトリ構造

```txt
.
├── Cargo.toml            # stationapi-worker (wasm32 専用) + workspace
├── wrangler.jsonc        # staging / production の設定
├── build.rs              # CSV を OUT_DIR に配置し、sst.bin を生成
├── src/                  # Worker 本体
│   ├── lib.rs            # エンドポイント
│   ├── index.rs          # 埋め込みデータのパースと索引
│   ├── repository.rs     # repository トレイトの実装
│   └── graphql/          # GraphQL の型とリゾルバ
│       ├── query.rs      # 18 クエリ
│       ├── types.rs      # オブジェクト型
│       ├── enums.rs      # 列挙型
│       └── scalar.rs     # UInt32 スカラー
│
├── schema/
│   └── public.graphql    # 公開スキーマの正本 (CI で突き合わせる)
│
├── stationapi/           # ドメインとユースケース (Worker と preprocessor で共有)
│   └── src/
│       ├── domain/
│       │   ├── entity/           # Station / Line / TrainType / Company ...
│       │   ├── repository/       # 抽象インターフェース
│       │   ├── arrival_estimation.rs
│       │   ├── route_search.rs       # 乗換経路探索 (RAPTOR)
│       │   ├── route_topology.rs     # 所要時間を持たない系統網 (stationsByName の到達判定)
│       │   ├── segment_speed_table.rs
│       │   ├── speed_table.rs
│       │   ├── ipa.rs
│       │   ├── romaji.rs
│       │   └── normalize.rs
│       ├── use_case/
│       │   ├── interactor/query.rs   # QueryInteractor
│       │   ├── traits/query.rs       # QueryUseCase トレイト
│       │   └── dto/                  # Entity → Model 変換
│       └── model.rs                  # API が返す値の型
│
├── preprocessor/         # generated/*.csv を生成するビルド時ツール
│   └── src/
│       ├── rail.rs       # data/*.csv の読み込みと各駅停車の系統生成
│       ├── gtfs/         # GTFS / ODPT の取得・解析・統合
│       ├── codes.rs      # バス用コードの生成
│       ├── table.rs      # 出力テーブルの表現
│       └── emit.rs       # CSV の書き出し
│
├── data_validator/       # data/*.csv の整合性チェック
├── data/                 # 鉄道の正本データ (CSV) と GTFS の展開先
├── generated/            # preprocessor の出力 (git 管理外)
├── scripts/              # データ整備とスキーマ比較のスクリプト
└── tools/                # IPA カバレッジの監査
```

---

## 運用

### 環境の使い分け

他の Worker に合わせて、env を省略したときは staging にデプロイされるように
しています。

```bash
make deploy             # wrangler deploy --env=""         -> stationapi-stg
make deploy-production  # wrangler deploy --env production -> stationapi
```

wrangler 4 は、複数の環境が定義されている状態で `--env` を省略すると警告を
出します。そのため、staging にデプロイするときも `--env=""` を明示しています。

### 注意点

- **データを更新するたびに再デプロイが必要。** データを WASM に埋め込んで
  いるため
- **custom domain は二重に登録できない。** ドメインを別の Worker に移すときは、
  先に元の Worker から外してデプロイしておく必要がある
- **`generated/` は git 管理外。** クローンした直後には存在しないので、
  `make data` で生成する。存在しないまま `worker-build` を実行すると
  `data/*.csv` にフォールバックし、各駅停車の系統とバスが欠けた状態で
  ビルドされる (警告は出る)

---

## 関連ドキュメント

- [Cloudflare Workers 移行の記録](./cloudflare-workers-migration.md)
- [技術負債分析レポート](./technical_debt.md)
- [近傍バス停検索機能](./nearby-bus-stops.md)
- [乗換経路探索 (RAPTOR) の設計](./route-search.md)
- [データ貢献ガイドライン](../data/README.md)
