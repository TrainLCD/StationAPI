# StationAPI アーキテクチャドキュメント

> 最終更新: 2026年9月22日

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

日本の鉄道駅・バス停の情報を返す GraphQL API です。Cloudflare Workers 上で動き、
データは WASM に埋め込んで配ります。**サーバープロセスもデータベースも持ちません。**

以前は gRPC-Web を返すオンプレのサーバーで、PostgreSQL を読み、クライアントは
BFF (TrainLCD/BFF) が GraphQL へ変換したものを使っていました。gRPC-Web である
必然性が無かったため GraphQL を直接返す形にし、BFF ごと廃止しています。

### 技術スタック

| 用途 | 採用しているもの |
|---|---|
| 実行環境 | Cloudflare Workers (wasm32-unknown-unknown) |
| API | GraphQL ([async-graphql](https://github.com/async-graphql/async-graphql) 7) |
| データ | ビルド時に WASM へ埋め込む CSV (`generated/*.csv`) |
| データ生成 | `preprocessor` crate (純 Rust) |
| デプロイ | wrangler |

データベースを使わないため、`sqlx` も接続プールもありません。検索は
起動時に組み立てたインメモリ索引に対する走査で行います。

---

## 全体構成

```txt
  data/*.csv          GTFS (ZIP)        ODPT (JSON)
  鉄道の正データ        バス 5 フィード      東急バス
      │                    │                 │
      └────────────────────┴─────────────────┘
                           │
                 ┌─────────▼──────────┐
                 │   preprocessor     │  純 Rust。各駅停車の系統生成と
                 │   (ビルド時ツール)  │  GTFS 統合を行う
                 └─────────┬──────────┘
                           │
                    generated/*.csv     7 テーブル
                           │
                 ┌─────────▼──────────┐
                 │  build.rs          │  CSV を OUT_DIR へ配置し、
                 │                    │  sst は固定長バイナリへ変換
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

データは WASM に埋め込まれるため、**データ更新のたびに再デプロイが要ります。**
起動時取り込みで自動反映される作りではありません。

---

## レイヤー構造

```txt
┌──────────────────────────────────────────────┐
│ Presentation (src/graphql/)                   │  async-graphql のリゾルバと型
│   Query / 型 / enum / スカラー                 │
├──────────────────────────────────────────────┤
│ Model (stationapi/src/model.rs)               │  API が返す値の表現
├──────────────────────────────────────────────┤
│ UseCase (stationapi/src/use_case/)            │  問い合わせの組み立て、
│   QueryInteractor / DTO 変換                   │  IPA・TTS の生成
├──────────────────────────────────────────────┤
│ Domain (stationapi/src/domain/)               │  エンティティ、経路探索、
│   entity / repository トレイト / 速度表        │  到達時間推定、正規化
├──────────────────────────────────────────────┤
│ Index (src/index.rs, src/repository.rs)       │  埋め込みデータの索引と
│                                               │  repository トレイトの実装
└──────────────────────────────────────────────┘
```

`stationapi` crate は Domain / UseCase / Model だけを持つライブラリで、
Worker と preprocessor の双方から参照されます。wasm32 でビルドできる必要が
あるため、I/O を伴う依存は入れません。

### Domain 層 (`stationapi/src/domain/`)

エンティティ、リポジトリの抽象、および純粋な計算 (haversine、経路の探索、
到達時間の推定、速度表、ローマ字・IPA 変換、検索用の正規化) を持ちます。

### UseCase 層 (`stationapi/src/use_case/`)

`QueryInteractor` が repository トレイト越しにデータを集め、駅へ路線・事業者・
列車種別を付与します。N+1 を避けるため、関連データは常に一括で取ります。

DTO (`use_case/dto/`) がドメインエンティティを Model へ変換します。IPA と
TTS セグメントの生成はここにあります。

### Model 層 (`stationapi/src/model.rs`)

API が返す値の表現です。もとは `.proto` から prost が生成していた型で、
gRPC をやめたあとも、上記の IPA・TTS 生成がここへの変換にぶら下がっているため
ドメインエンティティと GraphQL 型の間に残してあります。

### Presentation 層 (`src/graphql/`)

`async-graphql` の Query リゾルバと型定義です。Model から GraphQL 型へ変換します。

### Index 層 (`src/index.rs`, `src/repository.rs`)

埋め込み CSV を isolate 起動時に一度だけパースし、`OnceLock` に保持します。
`src/repository.rs` が 4 つの repository トレイトを実装し、UseCase 層からは
データベース版と同じインターフェースで見えます。

---

## データパイプライン

`data/*.csv` をそのまま Worker へ渡すと**本番と挙動が変わります。**

- 列車種別を持たない路線には各駅停車の系統を補う必要がある (約2,400行、
  有効な駅の約21%が影響を受ける)。この行は `data/*.csv` に存在しない
- バス停・バス路線・バス系統は GTFS と ODPT の JSON から起こす必要がある

これを行うのが `preprocessor` crate です。

```bash
make data     # cargo run --profile tool -p stationapi-preprocessor
```

処理の流れ:

1. `data/*.csv` を読む (`#` 始まりの列は取り込まない)
2. 各駅停車の系統を生成する (`generate_virtual_local_rail_services`)
3. GTFS フィードを取得・展開して読む (都営・西武・京王・東急コミュニティ 3 区)
4. 東急バスの ODPT JSON を読む (7 日間キャッシュ)
5. バスを lines / stations / types / station_station_types へ統合する
6. `generated/*.csv` を書き出す

`station_station_types.id` は停車順序そのものとして参照されるため、
行の並びに意味があります。書き出しは必ず `id` 昇順で行います。

### バスのコード生成

バス由来のレコードは、鉄道と衝突しない値域へ FNV-1a で決定的に割り当てます。
実行のたびに同じ値になる必要があるため、`DefaultHasher` は使いません。

| 対象 | 値域 | 入力 |
|---|---|---|
| `line_cd` | 100,000,000 + | `route_id` |
| `station_cd` | 200,000,000 + | `(stop_id, route_id)` |
| `station_g_cd` | 200,000,000 + | `stop_id` (まとめ後の代表) |
| `type_cd` | 100,000,000 + | `(route_id, shape_id)` |
| `line_group_cd` | 100,000,000 + | `(route_id, shape_id)` |

### 環境変数

| 変数 | 効果 |
|---|---|
| `ODPT_ACCESS_TOKEN` | 都営バス以外のフィードに必要。無い場合は警告のうえ読み飛ばす |
| `DISABLE_BUS_FEATURE` | `true` でバスを取り込まない (鉄道のみ) |

---

## インメモリ索引

PostgreSQL のクエリは以下のように置き換えています。

| PostgreSQL | Worker |
|---|---|
| `point(lat,lon) <-> point()` | haversine の全件走査 (`select_nth_unstable_by` で上位のみ確定) |
| `pg_trgm` の GIN インデックス | `contains()` |
| `station_station_types` の JOIN | `HashMap` による索引 |

`pg_trgm` は `LIKE '%...%'` を高速化するインデックスであって類似度検索では
ないため、`contains()` で論理的に等価な結果になります。正規化は domain 層の
`normalize_for_search` をそのまま呼びます。

39,204 件 (バス込み) の全件走査でも実測 10ms 台に収まります。

`station_station_types.csv` は 65,281 行あり、起動時の CSV パースが
コールドスタートの大半を占めていました。全列が整数なので、`build.rs` が
1 行 = `i32` x 4 の固定長バイナリ (`sst.bin`) へ事前変換しています。

---

## 乗換経路探索

`connectedRoutes` は、乗換を含む経路を乗換案内アプリと同じ要領で自動的に
探します。`routes` / `routeTypes` が「発着の両方に停車する系統」だけを返すのに
対し、こちらは系統をまたいで乗り継ぐ経路を返します。実装は
`stationapi/src/domain/route_search.rs` (純粋ロジック) にあります。

### 返す形

アプリは 1 本の列車 (系統) ごとに「種別を選ぶ → `lineGroupStations` で系統
全体の駅を取る → LCD を動かす」流れで動きます。乗換経路もこの流れに乗せられる
よう、経路は区間 (`legs`) の並びで返し、各区間はその区間で乗れる種別
(`trainTypes`、`routeTypes` と同じ形で実在の `groupId`) と乗車駅・降車駅
(`Station`) を持ちます。乗降駅には探索が乗った系統が走る路線の駅を返すので、乗換駅では前の区間の降車駅と
次の区間の乗車駅が別の駅 (同じ駅グループ) になることがあります
(例: 丸ノ内線の赤坂見附 → 半蔵門線の永田町)。

```graphql
connectedRoutes(fromStationGroupId: Int!, toStationGroupId: Int!, viaLineId: Int): [ConnectedRoute!]!

type ConnectedRoute { legs: [RouteLeg!] }
type RouteLeg { trainTypes: [TrainType!]  fromStation: Station  toStation: Station }
```

探索は停車駅が同じ並行種別 (中央線の快速・通勤快速など) を 1 つの経路に
まとめ、代替経路の再探索でもその並行種別を外します。このままでは並行する
種別がレスポンスに一度も出ず、アプリが種別一覧 (TrainTypeListModal) を出して
既定で各停を選ぶ今の挙動を保てません。そこで `trainTypes` に、その区間で
乗れる種別すべてを返します。中身は
`routeTypes(乗車駅グループ, 降車駅グループ, 降車駅の路線)` そのもので、停車駅が
同じ種別のまとめ・路線の付与・並び順も `routeTypes` と同じです (同じ関数を
呼んでいます)。探索が選んだ代表の種別、推定所要時間、乗換回数は並べ替えに
使うだけで、API では返しません (アプリが使わないため)。

`viaLineId` は `routeTypes` と同じく検索結果でタップした駅の路線で、目的地に
その路線の駅で着く経路 (最後の区間がその路線を走る経路) だけに絞ります。

### 系統網

系統 (`line_group_cd`) ごとの停車駅列を 1 本のパターンとし、駅グループ
(`station_g_cd`) を乗換の節点にします。駅間の所要時間は
`arrival_estimation` の推定値で、環状線は二周ぶん展開して継ぎ目を跨ぐ乗車も
同じ配列で引きます。対象は鉄道だけで、バスは含めません。

組み立てには全系統の駅と所要時間の推定が要り、ネイティブで約 190ms
(`data/*.csv` の 1,185 系統・41,706 行、半分が `Station` の組み立て、
半分が推定) かかります。起動時には作らず、最初に `connectedRoutes` が
呼ばれたときに `OnceLock` へ組み立てて isolate の寿命の間使い回します。

### 探索 (RAPTOR)

時刻表を持たないので、頻度ベースの RAPTOR で探します。ラウンド k は
「k 本目の列車に乗った時点」の最良値を求め、前ラウンドで改善した駅を通る
パターンだけを走査します。各ラウンドの目的地の値がそのまま「乗車 k 本以内での
最良」なので、所要時間と乗換回数のパレート解が得られます。

評価値 (秒) は次の合計です。

| 項目 | 値 |
|---|---|
| 乗車時間 | `arrival_estimation` の推定 (停車時間・通過を含む) |
| 乗車ごとの待ち時間 | 特急・新幹線 15 分 / 急行・新快速級 5 分 / それ以外 3 分 |
| 乗換の徒歩 | 3 分 |

待ち時間を入れないと、本数の少ない特急が「直通で速い」ことになり、
東京→渋谷で山手線より成田エクスプレスを勧めてしまいます。

### 代替経路と並び順

パレート解は最良と最少乗換しか含まないため、見つかった経路の区間を 1 つずつ
禁止して再探索します (Yen の k 最短経路の簡略版、最大 8 回)。禁止するのは
その区間の並行系統 (同じ乗車駅と降車駅に止まる快速・各停など) で、区間内の
どの駅からも乗れなくします。乗車駅だけを禁止すると、別の列車で 1 駅進んでから
同じ新幹線に乗る経路が出てしまうためです。

順位は「評価値 + 乗換 1 回あたり 5 分」で決め、最大 6 件返します。代替経路は
最良の 1.15 倍 + 15 分を超えるものと、パレート解の最多乗換回数を 2 回以上
上回るものを捨てます。乗車回数を増やしても順位の値が良くならないパレート解
(1 分縮めるために乗り換え続ける経路) も捨てます。

代替経路では、別々の区間で同じ駅グループに停車する経路も捨てます。区間を
禁止して再探索すると、「1 駅戻って同じ列車に乗り直す」逆戻りが代替経路として
出てくるためです (例: 大宮 → 土呂 → 大宮に停車して東京)。通過した駅は数えません
(急行で通過した駅へ先の駅から戻るのは実際にある乗り方です)。1 つの区間の中で
同じ駅に止まるのは実在する運行 (大江戸線の都庁前など) なので構いません。初回
探索のパレート解 (最適解) にはこの除外をかけません。かけると、逆戻りしか経路の
無い駅が「行ける駅」(`stationsByName`) なのに 0 件になるためです。

### 到着見込みと走行区間 (`estimateArrivalTimes` / `trainRoute`)

どちらも `legs: [RouteLegInput!]` を受け付け、乗換経路全体を通した値を返します。
`legs` には `connectedRoutes` の各区間の `trainTypes` から選んだ種別の
`groupId` と、区間の `fromStation.id`・`toStation.id` を渡します。経路 ID は
持たないので、経路はクライアントが区間の並びとして渡します。

選んだ種別が区間の乗降駅とは別の路線の駅に止まることがあります (乗降駅は
中央線快速の三鷹だが、選んだ各停は中央・総武線の三鷹に止まる、など)。そこで
系統に無い乗降駅は、同じ駅グループの駅で引き当てます。`station_cd` が一致する
駅があればそれを使い、駅グループの候補が複数あれば区間が最も短くなる組を
選びます (直通系統は接続駅で同じ駅グループの駅を 2 行持つため。宇都宮線の上野と
上野東京ラインの上野など)。

```graphql
input RouteLegInput { lineGroupId: Int!  fromStationId: Int!  toStationId: Int! }
```

- `estimateArrivalTimes`: 各区間を指定された系統だけで推定し (両駅に止まる別の
  系統は使わない)、出発駅からの累積でつないだ 1 本の経路を返します (`id` は
  系統をまたぐので空)。乗換駅は前の区間の降車駅と次の区間の乗車駅の 2 行で、
  乗車駅の行は「徒歩 3 分後に着き、乗換先の種別の待ち時間の後に出る」値です。
  見込みは `connectedRoutes` の並べ替えと同じ (徒歩 3 分と種別ごとの待ち時間) です。
- `trainRoute`: 区間ごとの走行区間を順につなげます。区間ごとに別の列車なので、
  各区間の最初の駅の `distanceFromPrevious` は 0 で、通過駅の有無 (優等種別の
  速度を使うか) も区間ごとに判定します。

区間の切り出しは 2 つで同じ関数を通し、環状線では継ぎ目を跨ぐ短い方の弧を取る
ので、両者の駅の並びは一致します (`lineGroupId` 指定の `trainRoute` は従来どおり
格納順で切り出します)。区間がつながっていない (前の区間の降車駅と次の区間の
乗車駅が別の駅グループ)、区間が 6 (`MAX_RIDES`、`connectedRoutes` が返しうる
乗車回数) を超える、端の駅が `fromStationId` / `toStationId` と食い違う、
`viaLineIds`・`directionId`・`lineGroupId` と併用した、のいずれかはエラーです。

### 行き先の検索 (`stationsByName`)

`stationsByName` に `fromStationGroupId` を指定すると、そこから行ける駅に
絞ります。出発駅と系統を共有する駅 (直通) と、どちらかが系統を持たない同じ
路線の駅に加え、乗り換えればその駅の路線の列車で着ける鉄道駅も返します。
最後のものは「`connectedRoutes` で `viaLineId` をその駅の路線にすると経路が
出る駅」と一致させてあり、系統を共有しないので `line_group_cd` は空、
`hasTrainTypes` は偽です (直通の駅と見分けられます)。

判定には、`connectedRoutes` と同じ系統から作った**所要時間を持たない網**
(`stationapi/src/domain/route_topology.rs` の `RouteTopology`) を使います。
要るのは「どの系統がどの駅に止まるか」だけなので、`Station` の組み立ても
所要時間の推定もせず索引から直接作り、組み立ては約 20ms (所要時間つきの網の
約 1/10) です。`RouteNetwork` も内部に同じ網を持ち、系統の整え方
(`trim_pattern`) と駅の選び方 (`line_group_rows`) を共有しています。両者が
一致することは実データのテストで確かめています。乗車 6 本以内で系統を
幅優先でたどります (1 回数 ms)。ただし探索は目的地の駅グループで途中下車しないので、
幅優先だけでは「支線の根元の駅に、支線へ一度出て戻って着く」経路を数えて
しまいます (石橋阪大前に箕面線で着く、新函館北斗に函館本線で着く、など。
実データで 0.3〜0.7% の駅)。これが起きるのは目的地が駅と系統の二部グラフの
関節点のときだけなので、系統網の組み立て時に関節点を求めておき (Tarjan)、
該当する駅に限って目的地で降りない幅優先で確かめます。実データの 3 つの
出発駅で各 1,500 駅を突き合わせ、探索との食い違いが無いことを確認しています。
`stationsByName` は 100 件ヒットでも 2〜4ms です (網の組み立て後)。

### 計算量

1 回の探索は O(乗車回数 × 触れたパターンの駅数) です。旧実装 (列車種別と
降車駅の全組み合わせを乗換回数ぶん列挙する有界 BFS) と比べ、実データでは次の
とおりです (ネイティブ release、`data/*.csv`)。

| 区間 | 旧実装 | 新実装 (探索のみ) |
|---|---|---|
| 東京 → 渋谷 | 370ms | 11〜13ms |
| 三鷹 → 中目黒 | 173ms (0 件) | 13〜18ms |
| 大宮 → 新大阪 | 349ms | 31〜36ms |
| 仙台 → 博多 | 693ms | 14〜20ms |

時刻表・運転間隔・駅グループを跨ぐ徒歩連絡 (`8!connections.csv` は空) の
データが無いため、待ち時間は種別からの見込みです。季節運行の臨時列車も
通常の系統と同じに扱います。

---

## GraphQL とスキーマ一致の担保

エンドポイントはクライアント互換のため、サブドメイン直下でクエリを受けます。

| パス | 内容 |
|---|---|
| `POST /` | クエリ実行 |
| `GET /` | GraphiQL |
| `GET /__schema` | SDL (CI が取得して突き合わせる) |
| `GET /__health` | 索引の件数 |
| `GET /__ping` | データに触らない疎通確認 |

`async-graphql` はコードファーストなので、Rust の型を変えると SDL が変わります。
クライアントが壊れる変更に気付けるよう、`schema/public.graphql` を正として
`scripts/compare_schema.py` が突き合わせ、CI で差分があれば失敗させます。
型とフィールドは集合として、enum は順序込みで比較します。

意図的にスキーマを変えるときはこのファイルも更新します。その差分が
クライアントへの影響範囲そのものになります。

実装上の注意:

- `async-graphql` は enum 値を既定で SCREAMING_SNAKE_CASE にする。公開スキーマは
  PascalCase なので `rename_items` で揃えている
- PascalCase 変換では `JR` が `Jr` になるため、この値だけ `name` を明示している
- `Station` / `StationNested` のように同一構造で名前が違う型は、SDL を合わせる
  ためマクロで両方定義している。Nested 型は互いを参照するので `Box` で
  間接化しないと無限サイズになる

---

## 命名規則

同じ「駅」を指す型が層ごとに 3 つあります。

| 種別 | 場所 | 目的 | 特徴 |
|---|---|---|---|
| **Record** | `src/index.rs` | 埋め込み CSV の 1 行 | 検索に要る列だけを持つ軽量な構造体 |
| **Entity** | `stationapi/src/domain/entity/` | ドメインモデル | ネスト構造、多言語対応、約66フィールド |
| **Model** | `stationapi/src/model.rs` | API が返す値 | 列挙型は `i32` のまま持つ |

### Record 構造体

```rust
// src/index.rs
pub struct StationRecord {
    pub station_cd: i32,
    pub station_g_cd: i32,
    pub name: String,
    // 検索に使う列だけ。応答用の Station は必要になってから組み立てる
}
```

全件走査を毎リクエスト行うため、`Station` エンティティ (66 フィールド) を
索引に持たせず、応答生成時にだけ組み立てます。ローマ字名の小文字版のように、
比較のたびに計算すると高くつくものは索引時に持っておきます。

### Entity 構造体

```rust
// stationapi/src/domain/entity/station.rs
pub struct Station {
    pub station_cd: u32,
    pub line: Option<Box<Line>>,
    pub lines: Vec<Line>,
    pub station_numbers: Vec<StationNumber>,
    // ...
}
```

- ビジネスセマンティクスを反映した型 (`StopCondition` 列挙型など)
- 多言語名: `station_name_r` (ローマ字)、`station_name_zh`、`station_name_ko`

### 変換フロー

```txt
generated/*.csv
    ↓  起動時に一度だけパース
Record (StationRecord)
    ↓  to_entity(): 路線の属性を埋める
Entity (Station)
    ↓  UseCase 層でネストデータを付与
Enriched Entity
    ↓  DTO 変換: IPA / TTS セグメントを生成
Model (model::Station)
    ↓  From 変換
GraphQL 型
```

---

## データフロー

### 典型的なリクエストフロー

```txt
[Client]
    │
    ▼ POST / (GraphQL)
┌──────────────────────────────────────────────┐
│ Presentation (src/graphql/query.rs)           │
│  └─ Query::station()                          │
└──────────────────────────────────────────────┘
    │
    ▼ QueryUseCase メソッド呼び出し
┌──────────────────────────────────────────────┐
│ UseCase (use_case/interactor/query.rs)        │
│  ├─ QueryInteractor::get_station_by_id()      │
│  └─ update_station_vec_with_attributes()      │
│      ├─ 駅グループ一括取得                      │
│      ├─ 路線一括取得                            │
│      ├─ 事業者一括取得                          │
│      └─ 列車種別一括取得                        │
└──────────────────────────────────────────────┘
    │
    ▼ repository トレイト経由
┌──────────────────────────────────────────────┐
│ Index (src/repository.rs, src/index.rs)       │
│  └─ MemStationRepository::find_by_id()        │
│      └─ HashMap 参照 / 全件走査                │
└──────────────────────────────────────────────┘
    │
    ▼ Record → Entity 変換
    ▼ Entity → Model 変換 (use_case/dto/)
    ▼ Model → GraphQL 型
[Client]
```

一括取得は N+1 を避けるためのもので、データベース時代から変えていません。
インメモリでも、駅ごとに索引を引き直すより一度に集めたほうが素直です。

### エラー伝播

```txt
DomainError
    ↓ ? 演算子
UseCaseError
    ↓ From トレイト
async_graphql::Error
    ↓
GraphQL の errors フィールド
```

未実装の repository メソッドは `DomainError` を返す設計にしてあります。
黙って空を返すと正常応答に見えて実装漏れに気付けないためです (移行時、
実際にこの設計のおかげで 1 件の漏れが 500 応答として検出できました)。

---

## ディレクトリ構造

```txt
.
├── Cargo.toml            # stationapi-worker (wasm32 専用) + workspace
├── wrangler.jsonc        # staging / production の設定
├── build.rs              # CSV を OUT_DIR へ配置、sst.bin を生成
├── src/                  # Worker 本体
│   ├── lib.rs            # エンドポイント
│   ├── index.rs          # 埋め込みデータのパースと索引
│   ├── repository.rs     # repository トレイトの実装
│   └── graphql/          # GraphQL の型・リゾルバ
│       ├── query.rs      # 18 クエリ
│       ├── types.rs      # オブジェクト型
│       ├── enums.rs      # 列挙型
│       └── scalar.rs     # UInt32 スカラー
│
├── schema/
│   └── public.graphql    # 公開スキーマの正 (CI が突き合わせる)
│
├── stationapi/           # ドメインとユースケース (Worker と preprocessor が共有)
│   └── src/
│       ├── domain/
│       │   ├── entity/           # Station / Line / TrainType / Company ...
│       │   ├── repository/       # 抽象インターフェース
│       │   ├── arrival_estimation.rs
│       │   ├── route_search.rs       # 乗換経路探索 (RAPTOR)
│       │   ├── segment_speed_table.rs
│       │   ├── speed_table.rs
│       │   ├── ipa.rs
│       │   ├── romaji.rs
│       │   └── normalize.rs
│       ├── use_case/
│       │   ├── interactor/query.rs   # QueryInteractor
│       │   ├── traits/query.rs       # QueryUseCase トレイト
│       │   └── dto/                  # Entity → Model 変換
│       └── model.rs                  # API が返す値の表現
│
├── preprocessor/         # generated/*.csv を作るビルド時ツール
│   └── src/
│       ├── rail.rs       # data/*.csv の読み込みと各駅停車の系統生成
│       ├── gtfs/         # GTFS / ODPT の取得・解釈・統合
│       ├── codes.rs      # バス用コードの生成
│       ├── table.rs      # 出力テーブルの表現
│       └── emit.rs       # CSV 書き出し
│
├── data_validator/       # data/*.csv の整合性検査
├── data/                 # 鉄道の正データ (CSV) と GTFS の展開先
├── generated/            # preprocessor の出力 (git 管理外)
├── scripts/              # データ整備・スキーマ比較のスクリプト
└── tools/                # IPA カバレッジ監査
```

---

## 運用

### 環境の使い分け

他の Worker と揃えて、env 省略時を staging にしてあります。

```bash
make deploy             # wrangler deploy --env=""         -> stationapi-stg
make deploy-production  # wrangler deploy --env production -> stationapi
```

wrangler 4 は複数環境がある状態で `--env` を省略すると警告するため、
staging を指す場合も `--env=""` を明示します。

### 注意点

- **データ更新のたびに再デプロイが要る。** WASM に埋め込むため
- **custom domain は二重に登録できない。** ドメインを移す際は、先に元の
  Worker から外してデプロイする必要がある
- **`generated/` は git 管理外。** クローン直後には無いので、`make data` で
  作る。無いまま `worker-build` すると `data/*.csv` にフォールバックし、
  各駅停車の系統とバスが欠けた状態でビルドされる (警告は出る)

---

## 関連ドキュメント

- [Cloudflare Workers 移行の記録](./cloudflare-workers-migration.md)
- [技術負債分析レポート](./technical_debt.md)
- [近傍バス停検索機能](./nearby-bus-stops.md)
- [データ貢献ガイドライン](../data/README.md)
