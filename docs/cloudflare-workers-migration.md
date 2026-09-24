# Cloudflare Workers 移行

> 最終更新: 2026年9月24日
>
> **移行は完了しています。** gRPC サーバーと PostgreSQL は削除し、BFF も
> 廃止しました。このリポジトリは、Cloudflare Workers 上で動く GraphQL API
> そのものになっています。現在の構成は [architecture.md](./architecture.md)
> を参照してください。
>
> 以下は移行時の検証記録です。数値や挙動は、特に断りがなければ当時のものです。

## 目次

- [背景と目的](#背景と目的)
- [結論](#結論)
- [アーキテクチャ](#アーキテクチャ)
- [実装の要点](#実装の要点)
- [データの用意](#データの用意)
- [検証方法](#検証方法)
- [実測値](#実測値)
- [作業中に見つかった問題](#作業中に見つかった問題)
- [運用上の注意](#運用上の注意)
- [残作業](#残作業)
- [追記: データパイプラインの純 Rust 化](#追記-データパイプラインの純-rust-化)
- [追記: 本番 (BFF 経由の gRPC) との応答突き合わせ](#追記-本番-bff-経由の-grpc-との応答突き合わせ)
- [関連](#関連)

---

## 背景と目的

オンプレミスで動かしていた gRPC-Web の API を Cloudflare Workers へ移せるかを
検証し、そのまま実装まで進めました。

当時、クライアントは gRPC-Web を直接使わず、BFF (TrainLCD/BFF) が GraphQL に
変換したものを使っていました。gRPC-Web を使い続ける理由がなかったため、
Worker 版は GraphQL を直接返すようにし、BFF を経由しない構成にしました
(BFF はその後廃止しています)。

検証中はオンプレミス版 (gRPC) も残していました。検証を終えた後に、次のものを
削除して Worker 版を本体にしています。

- gRPC サーバー
- sqlx を使った repository 層
- PostgreSQL
- proto

---

## 結論

移行は可能と判断しました。BFF が公開していた 18 クエリをすべて Workers 上で
動かし、当時は staging で稼働させていました。

gRPC の rpc は 19 本ありましたが、`GetRoutesMinimal` はどこからも呼ばれて
いなかったため削除しました。

| 項目 | 結果 |
|---|---|
| domain / use_case 層 (約17,000行) | **既存のロジックは変更していない** (未使用の `GetRoutesMinimal` を削除しただけ) |
| PostgreSQL | 不要 |
| `pg_trgm` / `point() <-> point()` | 不要 |
| GraphQL スキーマ | 当時の公開スキーマと完全に一致 (18クエリ / 28型) |
| バス (GTFS) | 対応済み (都営・西武・京王・東急) |

`sqlx` と `tonic` は wasm32 では動きません。`sqlx` は埋め込みデータの
インメモリ索引に置き換え、`tonic` は GraphQL 化によって不要になりました。
repository トレイトの実装を差し替えるだけで、経路探索を含む既存の
ビジネスロジックはそのまま動きます。

---

## アーキテクチャ

### 移行前

```text
TrainLCD -> BFF (GraphQL -> gRPC-Web 変換) -> StationAPI (gRPC) -> PostgreSQL
```

### 移行後

```text
TrainLCD -> stationapi (GraphQL を直接返す) -> WASM に埋め込んだデータ
```

### レイヤーの対応

| 層 | 旧 (gRPC) | 現在 |
|---|---|---|
| Presentation | `presentation/controller/grpc.rs` (tonic) | `src/graphql/` (async-graphql) |
| UseCase | `use_case/` | **同じものを使用** |
| Domain | `domain/` | **同じものを使用** |
| Infrastructure | `infrastructure/*_repository.rs` (sqlx) | `src/repository.rs` (インメモリ) |
| データ生成 | `import.rs` (PostgreSQL への取り込み) | `preprocessor/` (Rust のみで実装) |

### crate 構成

移行の検証中は、`stationapi` crate を `server` feature で分割し、Worker を
workspace から除外していました。gRPC を削除した後は Worker がルートの crate に
なり、共有部分だけが `stationapi` crate として残っています。

```text
Cargo.toml       # stationapi-worker (wasm32 専用) と workspace の定義
build.rs         # データの配置とバイナリ化
src/
  index.rs       # 埋め込みデータのパースとインメモリ索引
  repository.rs  # 4 つの repository トレイトの実装
  graphql/       # GraphQL の型とリゾルバ
  lib.rs         # エンドポイント
schema/public.graphql  # 公開スキーマの正本 (CI が比較に使う)
scripts/compare_schema.py

stationapi/      # domain / use_case / model (Worker と preprocessor が共有)
preprocessor/    # generated/*.csv の生成 (Rust のみで実装)
data_validator/  # CSV の整合性を検証する CLI
```

---

## 実装の要点

### SQL のインメモリ置換

| PostgreSQL | Worker (移行時) |
|---|---|
| `point(lat,lon) <-> point()` | haversine による全件走査 (`select_nth_unstable_by` で上位だけを確定) |
| `pg_trgm` の GIN インデックス | `contains()` |
| `station_station_types` の JOIN | `HashMap` による索引 |

`pg_trgm` は `LIKE '%...%'` を高速化するためのインデックスで、類似度検索では
ありません。そのため `contains()` で論理的に同じ結果が得られます。検索語の
正規化には、domain 層の `normalize_for_search` をそのまま使っています。

当時は 11,148 駅 (バス停を含めると 39,204 件) を全件走査しても、実測で 10ms 台に
収まっていました。

なお現在の座標検索は全件走査ではなく、交通種別ごとのグリッド索引 (`Grid`、0.05° の
セル) を使っています (`src/index.rs` の `nearest` / `within_radius`)。詳しくは
[アーキテクチャドキュメントの「インメモリ索引」](./architecture.md#インメモリ索引)
を参照してください。

### GraphQL

`async-graphql` 7 を採用しました。wasm32-unknown-unknown 向けにビルドできる
ことを確認してから導入しています。

値は **domain エンティティ → model → GraphQL 型** の順に変換します。IPA や
TTS セグメントの計算は use_case の DTO 側にあるので、この中間表現を経由すれば
そのロジックをそのまま使えます。`model` はもともと proto から生成していた型で、
gRPC を削除した後は手書きの構造体になっています。

クライアントとの互換性のため、エンドポイントはサブドメイン直下 (`/`) で
クエリを受け付けます。

| パス | 内容 |
|---|---|
| `POST /` | クエリの実行 |
| `GET /` | GraphiQL |
| `GET /__schema` | SDL (CI が取得して公開スキーマと比較する) |
| `GET /__health` | 索引の件数 |
| `GET /__ping` | データに触れない疎通確認 |

### スキーマ一致の担保

`async-graphql` はコードファーストなので、Rust の型を変えると SDL も変わります。
クライアントを壊す変更に気付けるよう、`schema/public.graphql` を正本とし、
`scripts/compare_schema.py` で Worker の SDL と比較しています。差分があれば
CI は失敗します。型とフィールドは順序を無視した集合として、enum は順序も含めて
比較します。

このファイルはもともと BFF の `schema.graphql` を写したものです。BFF を廃止した
現在は、これが公開スキーマの基準です。意図してスキーマを変えるときは、この
ファイルも同じ変更で更新します。その差分が、そのままクライアントへの影響範囲に
なります。

実装中に遭遇した差分は次のとおりです。

- `async-graphql` は enum の値を既定で SCREAMING_SNAKE_CASE にする。公開
  スキーマは PascalCase なので、`rename_items` で揃えた
- PascalCase に変換すると `JR` が `Jr` になるため、この値だけ `name` を
  明示した
- `Station` と `StationNested` のように、構造が同じで名前だけが違う型は、
  SDL を合わせるためにマクロで両方を定義した。Nested 型は互いを参照するので、
  `Box` で間接参照にしないと型のサイズが無限になる

---

## データの用意

**`data/*.csv` をそのまま読むと、本番とは挙動が変わります。** 列車種別を
持たない路線には、各駅停車の系統を補う必要があるためです。当時の実測では
2,427 行が生成され、2,268 駅 (有効な駅の約 21%) が影響を受けていました。

移行の検証中は、この変換を PostgreSQL への取り込み時に行い、取り込み後の DB を
`stationapi --export-worker-data` で書き出していました。gRPC の削除にあわせて
同じ変換を `preprocessor` crate (Rust のみで実装) へ移したため、PostgreSQL は
不要になりました。

```text
make data     # cargo run --profile tool -p stationapi-preprocessor
```

出力するのは次の 7 テーブルです。

- companies
- lines
- stations
- types
- station_station_types
- aliases
- line_aliases

`build.rs` は、`generated/*.csv` があればそれを OUT_DIR に配置します。なければ
`data/*.csv` にフォールバックし、警告を出します。一部のテーブルだけが
`generated/` にある状態は、データが食い違うためビルドを失敗させます。
あわせて、`station_station_types` を固定長のバイナリ (`sst.bin`) に変換します。

CI では、この流れを composite action (`.github/actions/build-worker`) が実行
します。検証用の `build_worker.yml` と、デプロイ用の `deploy_staging.yml` /
`deploy_production.yml` がこれを共有しています。移行時点では
`build_worker.yml` が単独で実行していました。

### バス (GTFS)

`DISABLE_BUS_FEATURE` を指定しなければ、GTFS の取得と統合も実行します。
フィードによっては `ODPT_ACCESS_TOKEN` が必要です。

| フィード | トークン |
|---|---|
| 都営バス | 不要 |
| 西武バス / 京王バス / 東急バス (3区のコミュニティバス) / 東急バス ODPT JSON | 必要 |

全フィードを取り込んだ後のデータ量 (当時) は次のとおりです。

| テーブル | 鉄道のみ | 全フィード |
|---|---|---|
| lines | 624 | 1,601 (+977 バス路線) |
| stations | 11,148 | 39,204 (+28,063 バス停) |
| types | 325 | 1,585 (+1,260 バス系統) |
| station_station_types | 43,677 | 65,281 |

---

## 検証方法

`postgres:18` に実データを投入し、**既存の SQL の結果と直接比較しました。**
実装を読んで「同じになるはず」と判断するのではなく、実際のクエリ結果を
比べています。

| 対象 | 内容 |
|---|---|
| 名前検索 | ランダムな 30 クエリで `station_cd` の集合が一致 |
| `lineGroupStations` | 10 グループで順序も含めて一致 (最大 250 件) |
| `lineStations` | 5 路線で順序も含めて一致 (種別あり・フォールバックの両方) |
| `stationTrainTypes` | 6 駅で `sst.id` と種別名が順序も含めて一致 |
| `linesByName` | 6 クエリで順序も含めて一致 |
| `lines[]` の line_cd の集合 | 9 駅グループで一致 |
| `hasTrainTypes` | `lines[]`・駅本体ともに不一致 0 件 |

`station_station_types.id` は `ORDER BY sst.id` として停車順そのものに使われます。
そのため、SERIAL で採番したときの順序が保たれていることを `build.rs` で検証して
います。

### 検証手法の落とし穴

作業中に何度か、**検証スクリプト側の不備で誤った結論を出しかけました。**

- gRPC-Web 用の比較スクリプトを GraphQL 化の後もそのまま使っていたため、404 を
  「1 件」と数えてしまい、差分があるように見えた。さらにそれ以前は、「0 件中
  0 件が不一致」を一致と表示していた
- Node が TTY かどうかを判定して数値に ANSI エスケープを付けたため、順序の
  不一致と誤判定した
- 比較用の SQL に `transport_type` の条件がなく、Worker 側の既定のフィルタとの
  違いが差分に見えた

いずれも実装は正しく、スクリプトを直すと結果は一致しました。

---

## 実測値

staging (`gql-stg.trainlcd.app`) で測定しました。日本から東京のエッジに接続して
います (`cf-ray` は NRT)。

```text
全18クエリ         : 成功
サーバー処理       : 通常 12〜20ms (接続確立の TLS に 21〜40ms かかる)
keep-alive 20回    : p50=0ms 最大58ms 平均3ms
コールドスタート    : 20回に1回程度、60〜130ms
Worker Startup Time: 3〜7ms (Cloudflare の報告値)
WASM gzip          : 3,199KB (上限10MiBの31%)
```

**接続を使い回した (keep-alive) 20 回の測定では、平均 3ms でした。**

### コールドスタートについて

`wrangler dev` (ローカルの workerd) では約 200ms かかりましたが、**これは本番の
指標になりませんでした。** 本番の `Worker Startup Time` は 3〜7ms です。

コールドスタートのばらつきの原因を調べた結果、**データの初期化は主因ではない**
ことが分かりました。データにまったく触れない `/__ping` が、`/__health` と同等か
それ以上に遅い場合があったためです。

`stations.csv` を固定長レコードと文字列プールに変換し、`&'static str` で参照する
案も試しました。しかし 30 回程度の測定では有意な差が出ず、gzip 後のサイズが
235KB 増えるだけだったため採用しませんでした。同じバイナリでも p90 が
27ms → 83ms と変動しており、有意差を確かめるには数百回規模の測定と統計処理が
必要な水準でした。

なお Workers では、Spectre 対策のため同期コードの実行中に `Date.now()` が
進みません。そのためプロセス内で区間ごとの時間を計測することはできず、
切り分けは外部から応答時間の分布を比べる形になります。

---

## 作業中に見つかった問題

### Worker 実装側の漏れ (修正済み)

既存の SQL と照合して見つけたものです。いずれも同じ PR の中で修正しました。

| 内容 | 影響 |
|---|---|
| `get_by_line_id_vec_with_group_stations` が未実装 | `GetStationsByLineIdList` が 500 を返す |
| `get_by_station_group_id_vec_no_types` が `line_group_cd` を埋めていない | `lines[].station.hasTrainTypes` が常に false |
| `lines_of_groups` が路線の `e_status` を見ていない | 無効化された路線 (成田エクスプレス) が `lines[]` に混ざる |
| `LineRepository::get_by_station_group_id_vec` が通過の条件を見ていない | 停車しない系統しか持たない駅の路線が混ざる |
| `TrainTypeRepository::get_by_line_group_id_vec` の並び順 | `priority DESC` で並べていたが、SQL は `sst.id` だけで並べている |
| `LineRepository::find_by_station_id` が sst 由来の列を埋めていない | `line_group_cd` / `type_cd` が NULL のまま |

未実装のメソッドは `DomainError` を返す設計にしていたため、1 件目は 500 応答と
して検出できました。黙って空の結果を返していたら、正常な応答に見えて気付け
なかったはずです。

最後に、すべての repository メソッド (34 個) について、対応する SQL の
`WHERE` / `ORDER BY` を機械的に抜き出して照合しました。

### 移行元 (gRPC 版) のバグ

Worker への移行とは関係のない、既存実装の問題です。gRPC 版で再現することを
確認してから起票しました。

- **[#1636](https://github.com/TrainLCD/StationAPI/issues/1636) GetRoutes / EstimateArrivalTimes が特定の駅ペアでパニックする**

  `get_route_stops` の SQL は `WHERE sst.line_group_cd IS NULL` で絞り込むため、
  返ってくる駅の `line_group_cd` は必ず NULL になります。それを受け取る
  `build_route_tree_map` が `.expect()` していたので、1 件でも返ると必ず
  パニックしていました。`100410 → 100422` で再現します。

- **[#1637](https://github.com/TrainLCD/StationAPI/issues/1637) GTFS を含むデータの取り込みに約 7 分半かかる**

  `build_stop_route_mapping` の再帰 CTE だけで 63 秒かかっていました。
  `main.rs` は起動時にこれを実行するため、再起動のたびに同じ時間がかかって
  いました。

### 削除したもの

`GetRoutesMinimal` は、BFF のスキーマに対応するクエリがなく、どこからも
呼ばれていなかったため削除しました。proto は submodule なので、
[TrainLCD/gRPCProto#30](https://github.com/TrainLCD/gRPCProto/pull/30) で
削除し、マージ済みです。

---

## 運用上の注意

**データを更新するたびに再デプロイが必要です。** Worker はデータを WASM に
埋め込んでいるため、`data/*.csv` や GTFS が変わったらビルドし直さなければ
なりません。オンプレミス版のように、起動時の取り込みで自動的に反映される
わけではありません。

**環境の使い分け。** 他の Worker に揃えて、env を省略したときの環境を staging に
しています。

```text
staging : wrangler deploy --env=""          -> stationapi-stg
本番    : wrangler deploy --env production  -> stationapi
```

wrangler 4 は、複数の環境がある状態で `--env` を省略すると警告を出します。
そのため staging にデプロイするときも `--env=""` を明示します。

現在は、デプロイ先をブランチで固定しています。`dev` への push で
`deploy_staging.yml` が staging へ、`master` への push で `deploy_production.yml`
が本番へデプロイします。手元からは `make deploy` (staging) と
`make deploy-production` (本番) を使い、どちらも対応するブランチ以外からは
実行できません。詳しくは [AGENTS.md](../AGENTS.md) の「Running and Deploying」を
参照してください。

**custom domain は二重に登録できません。** ドメインを別の Worker へ移すときは、
先に元の Worker から外してデプロイしておく必要があります。

---

## 残作業

いずれも完了しています。

- [x] **[#1638](https://github.com/TrainLCD/StationAPI/issues/1638) 本番へ適用する** — 2026年8月27日にクローズ。現在は `deploy_production.yml` が `master` から本番へデプロイしている
- [x] [#1636](https://github.com/TrainLCD/StationAPI/issues/1636) のパニックを修正する — #1640 で `build_route_tree_map` が `line_group_cd` を持たない駅を読み飛ばすようにした
- [x] [#1637](https://github.com/TrainLCD/StationAPI/issues/1637) の取り込み時間 — PostgreSQL をやめたことで解消した (7 分半 → 7 秒)
- [x] CI ワークフローを実行する — `build_worker.yml` と、デプロイ用の `deploy_staging.yml` / `deploy_production.yml` が稼働している。`ODPT_ACCESS_TOKEN` は `staging` / `production` の環境 Secret に設定してあり、デプロイ時は 1 つでも取り込めないフィードがあれば失敗する (`fail-on-missing-bus-feeds`)

---

## 追記: データパイプラインの純 Rust 化

gRPC の削除にあわせて、PostgreSQL への取り込み時に行っていたデータ生成を
`preprocessor` crate へ移しました。移植が正しいことは、**PostgreSQL 版が出力した
`generated/*.csv` を正解データとして比較する**ことで確かめています。

結果 (39,204 駅 / 1,601 路線 / 65,281 station_station_types):

| テーブル | 結果 |
|---|---|
| companies / lines / aliases / line_aliases | **バイト単位で完全に一致** |
| types | `id` 以外の全列が一致。8 行で `id` だけが異なる |
| station_station_types | **2,587 系統すべてで停車順が一致** |
| stations | `e_sort` 以外の全列が一致。バス停 273 件 (11 系統) で `e_sort` だけが異なる |

違いはいずれも、**元の SQL が順序を決めていなかった箇所**から生じています。

- `types.id` の 8 件と、それに伴う `station_station_types` の並びの違い
  - `ORDER BY route_id` の結果が、PostgreSQL コンテナの locale (`en_US.UTF-8`)
    に依存していたことが原因です。
  - glibc の照合順序は、まず大文字と小文字の違いを無視して比較します。その
    ため `...JiyuugaokaekiJiyuugaokaeki` と
    `...JiyuugaokaekiiriguchiJiyuugaokaeki` の前後が、バイト順とは逆に
    なります。
  - Rust 版はバイト順で決めます。CI ランナーの locale によって出力が
    変わらなくなる利点のほうが大きいと判断しました。
- `stations.e_sort` の 273 件
  - `DISTINCT ON` で同点になった行の選び方の違いです。優先度も
    `stop_sequence` も同じ行が複数あり、どれが選ばれるかは実行計画次第でした。
  - たとえば京王の 1972 系統の停留所 `1298_00` は、終点として現れる便と途中で
    停車する便とで `next` が食い違います。
  - Rust 版は `trip_id` まで見て一意に決めます。

どちらも「同等の候補のうちどれを選ぶか」の違いにすぎず、停車順そのものは
2,587 系統すべてで一致しています。

---

## 追記: 本番 (BFF 経由の gRPC) との応答突き合わせ

移行を本番に適用する前に、当時まだ稼働していた `https://gql.trainlcd.app`
(オンプレミスの gRPC + BFF) と現行実装の応答を、公開スキーマの全 18 クエリ・
全フィールドについて比較しました。

- 選択セットは、スキーマのイントロスペクションから自動生成しました。
- 配列は id で対応付けたうえで、「集合」「順序」「値」に分けて比較しました。

### 見つかった実装の不具合 (いずれも修正済み)

| 内容 | 影響 |
|---|---|
| `Company.name` に `nameShort` を入れていた | 略称と正式名称が異なる事業者で名前が食い違う (相模鉄道 → 相鉄、東急電鉄 → 東急 など) |
| `find_by_id` / `get_by_id_vec` が `line_group_cd` を埋めていない | `station.hasTrainTypes` が常に false |
| `LineRepository::get_by_ids` に `e_status = 0` の条件がない | 廃止済み・未開業の路線が `lines(lineIds:)` で返る |
| `find_by_line_group_id_and_line_id` が `pass <> 1` で絞り込み、駅の `e_status` を見ていない | `lines[].trainType.id` が別の駅の値になる |
| `lines.average_distance` を `f32` の最短表記で書き出していた | 読み直すと別の値になり、応答が 31664.842 と 31664.841796875 でずれる |
| バス路線の `nameChinese` / `nameKorean` / `nameRoman` が null | DB 側の既定値が `''` だったため、本番は空文字を返していた |
| `LineRepository::get_by_station_group_id_vec_no_types` が通過の条件を見ていない | その駅を通過するだけの路線が `station.lines` に混ざる。`skip_types_join = true` で動く `station` / `stations` / `stationsNearby` / `stationsByName` / `lineListStations` などが該当し、generated データでは中央線 (快速) の代々木・大久保・東中野など 5 路線 35 駅で発生していた |

### 残っている差分

いずれも実装の誤りではありません。

| 分類 | 内容 |
|---|---|
| 座標の距離計算 | 本番は `point(lat,lon) <-> point()` (ユークリッド距離)、こちらは haversine。近くのバス停の選ばれ方と、駅に付くバス路線の並びが変わる |
| `stationsNearby` の `distance` | 本番は常に null。旧 SQL が距離を SELECT しておらず、`From<StationRow>` が `None` を固定で入れていたため。こちらは実際の距離を返す (本番側の不足) |
| 並び順 | 旧 SQL に `ORDER BY` がない、または同じ値で順序が決まらない箇所。たとえば `stationsByName(name:"渋谷")` は、返る 10 駅は完全に一致するが、全行が同じ `station_g_cd` と同じ駅名なので、`ORDER BY station_g_cd, station_name` では順序が決まらない |
| 既定の `transportType` | gRPC 版は未指定を Rail として扱い、こちらは RailAndBus として扱う。移行時に意図して変更した |

未解決の差分はありません。次の 2 件は比較の過程で目に付きましたが、どちらも
こちらの挙動のほうが正しいものです。

- **`trainType.lines[]` の別名が駅ごとに変わる。**

  `line_aliases.csv` は `station_cd` をキーにしているので、別名は路線ではなく
  (路線, 駅) の組に付く属性です。11314 (総武本線) では、次のように区間ごとに
  別名が分かれています。

  - 1131401〜1131409 (東京〜錦糸町の快速区間): 別名 12 (総武快速線 / `#0067C0`)
  - 1131411〜1131431 (千葉以東): 別名 7 (`#FFD400`)

  成田エクスプレスは両方の区間に停まるため、系統 1095 の `lines[]` には区間
  ごとの見え方が並びます。本番は 4 件とも総武本線 / `#0067C0` にまとめており、
  別名を適用していませんでした。

- **`viaLineId` を渡すと、経由路線上に発駅・着駅を持たない系統が候補から外れる。**

  経由路線での絞り込みによって範囲外の駅が除かれると、その系統は発駅と着駅の
  両方を含まなくなるため、`get_routes` の判定で候補から外れます。本番は
  成田エクスプレス (系統 1095) を返していましたが、その停車駅 16 件には目的地の
  新宿が含まれていません。つまり、目的地に着かない経路を返していました。

  | | 本番 | こちら |
  |---|---|---|
  | `routes(fromStationGroupId: 1130205, toStationGroupId: 1130208, viaLineId: 11302)` | `[363, 1095]` (1095 は新宿を含まない) | `[363]` |

  `viaLineId` を指定しなければ、どちらも 20 件で一致します。

---

## 関連

| リポジトリ | 内容 |
|---|---|
| [StationAPI#1635](https://github.com/TrainLCD/StationAPI/pull/1635) | 移行の PR |
| [gRPCProto#30](https://github.com/TrainLCD/gRPCProto/pull/30) | GetRoutesMinimal の削除 (マージ済み) |
| [BFF#51](https://github.com/TrainLCD/BFF/pull/51) | staging の route の削除 (マージ済み) |
