# Cloudflare Workers 移行

> 最終更新: 2026年8月22日

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

---

## 背景と目的

オンプレで動かしている gRPC-Web API を Cloudflare Workers へ移せるかを検証し、実装まで進めた。

あわせて、クライアントは BFF (TrainLCD/BFF) が gRPC-Web を GraphQL へ変換したものを利用していたが、gRPC-Web である必然性が無いため、Worker 版は GraphQL を直接返すようにした。BFF を経由しない。

**オンプレ版 (gRPC) はそのまま残している。** 今回の変更で gRPC を廃止したわけではない。

---

## 結論

移行できる。全19エンドポイント相当を Workers 上で動かし、staging で稼働している。

| 項目 | 結果 |
|---|---|
| domain / use_case 層 (約17,000行) | **1行も変更していない** |
| PostgreSQL | 不要 |
| `pg_trgm` / `point() <-> point()` | 不要 |
| GraphQL スキーマ | BFF の `schema.graphql` と完全一致 (18クエリ / 28型) |
| バス (GTFS) | 対応済み (都営・西武・京王・東急) |

`sqlx` と `tonic` は wasm32 で動かないため、前者は埋め込みデータのインメモリ索引に置き換え、後者は GraphQL 化により不要になった。repository トレイトの実装を差し替えるだけで、経路探索を含む既存のビジネスロジックがそのまま動く。

---

## アーキテクチャ

### 移行前

```
TrainLCD -> BFF (GraphQL -> gRPC-Web 変換) -> StationAPI (gRPC) -> PostgreSQL
```

### 移行後 (staging)

```
TrainLCD -> stationapi-stg (GraphQL 直接) -> WASM に埋め込んだデータ
```

オンプレ版 (gRPC + PostgreSQL) は従来どおり動作する。

### レイヤーの対応

| 層 | オンプレ版 | Worker 版 |
|---|---|---|
| Presentation | `presentation/controller/grpc.rs` (tonic) | `worker/src/graphql/` (async-graphql) |
| UseCase | `use_case/` | **同じものを使用** |
| Domain | `domain/` | **同じものを使用** |
| Infrastructure | `infrastructure/*_repository.rs` (sqlx) | `worker/src/repository.rs` (インメモリ) |

### crate 構成

`stationapi` crate を `server` feature で分割した。feature を外すと domain / use_case / proto だけが残り、wasm32 でビルドできる。

```
stationapi/
  Cargo.toml     # 依存を optional 化、server feature を定義
  build.rs       # CARGO_FEATURE_SERVER で build_server() を切替
  src/lib.rs     # infrastructure / presentation を #[cfg(feature = "server")] に

worker/          # workspace から exclude (wasm32 専用)
  build.rs       # データのバイナリ化と配置
  src/
    index.rs     # 埋め込みデータのパースとインメモリ索引
    repository.rs # 4つの repository トレイトの実装
    graphql/     # GraphQL の型・リゾルバ
    lib.rs       # エンドポイント
  schema/bff.graphql  # BFF スキーマの写し (CI が突き合わせる)
  scripts/compare_schema.py
```

---

## 実装の要点

### SQL のインメモリ置換

| PostgreSQL | Worker |
|---|---|
| `point(lat,lon) <-> point()` | haversine の全件走査 (`select_nth_unstable_by` で上位のみ確定) |
| `pg_trgm` の GIN インデックス | `contains()` |
| `station_station_types` の JOIN | `HashMap` による索引 |

`pg_trgm` は `LIKE '%...%'` を高速化するインデックスであって類似度検索ではないため、`contains()` で論理的に等価な結果が得られる。正規化は domain 層の `normalize_for_search` をそのまま呼んでいる。

11,148駅 (バス込みで39,204件) の全件走査でも実測 10ms 台に収まる。

### GraphQL

`async-graphql` 7 を採用した。wasm32-unknown-unknown でビルドできることを確認してから導入している。

値は **domain エンティティ → proto → GraphQL 型** の順に変換する。IPA や TTS セグメントの計算が use_case の DTO 側にあるため、proto を経由するとそのロジックをそのまま使える。

エンドポイントは BFF に合わせ、サブドメイン直下でクエリを受ける。

| パス | 内容 |
|---|---|
| `POST /` | クエリ実行 |
| `GET /` | GraphiQL |
| `GET /__schema` | SDL (CI が取得して突き合わせる) |
| `GET /__health` | 索引の件数 |
| `GET /__ping` | データに触らない疎通確認 |

### スキーマ一致の担保

`async-graphql` はコードファーストなので、Rust の型を変えると SDL が変わる。クライアントが壊れる変更に気付けるよう、`worker/schema/bff.graphql` を正として `worker/scripts/compare_schema.py` が突き合わせ、CI で差分があれば失敗させる。型とフィールドは集合として、enum は順序込みで比較する。

実装時に踏んだ差分:

- `async-graphql` は enum 値を既定で SCREAMING_SNAKE_CASE にする。BFF は PascalCase なので `rename_items` で揃えた
- PascalCase 変換では `JR` が `Jr` になるため、この値だけ `name` を明示した
- `Station` / `StationNested` のように同一構造で名前が違う型は、SDL を合わせるためマクロで両方定義した。Nested 型は互いを参照するので `Box` で間接化しないと無限サイズになる

---

## データの用意

**`data/*.csv` をそのまま読むと本番と挙動が変わる。** `import_csv` は最後に `generate_virtual_local_rail_services` を実行し、列車種別を持たない路線へ各駅停車の系統を DB 上で生成する。実測で 2,427行が生成され、2,268駅 (有効な駅の約21%) が影響を受ける。

そのため、取り込み後の DB から書き出したものを使う。

```
stationapi --export-worker-data worker/generated
```

companies / lines / stations / types / station_station_types の5テーブルを CSV へ出す。列は `information_schema` から取り、すべて text へキャストして読むので列が増えても追従できる。

`worker/build.rs` は `worker/generated/*.csv` があればそれを OUT_DIR へ配置し、無ければ `data/*.csv` にフォールバックして警告を出す。

CI (`.github/workflows/build_worker.yml`) が PostgreSQL を立ててこの流れを実行する。

### バス (GTFS)

`DISABLE_BUS_FEATURE` が立っていなければ `import_gtfs` と `integrate_gtfs_to_stations` も実行される。`ODPT_ACCESS_TOKEN` が必要なフィードがある。

| フィード | トークン |
|---|---|
| 都営バス | 不要 |
| 西武バス / 京王バス / 東急バス (3区) / 東急バス ODPT JSON | 必要 |

全フィード取り込み後のデータ量:

| テーブル | 鉄道のみ | 全フィード |
|---|---|---|
| lines | 624 | 1,601 (+977 バス路線) |
| stations | 11,148 | 39,204 (+28,063 バス停) |
| types | 325 | 1,585 (+1,260 バス系統) |
| station_station_types | 43,677 | 65,281 |

---

## 検証方法

`postgres:18` に実データを投入し、**既存 SQL の結果と直接突き合わせた。** 実装を読んで「同じはず」と判断するのではなく、実際のクエリ結果を比較している。

| 対象 | 内容 |
|---|---|
| 名前検索 | ランダム30クエリで `station_cd` 集合が一致 |
| `lineGroupStations` | 10グループで順序込み一致 (最大250件) |
| `lineStations` | 5路線で順序込み一致 (種別あり/フォールバック両方) |
| `stationTrainTypes` | 6駅で `sst.id` と種別名が順序込み一致 |
| `linesByName` | 6クエリで順序込み一致 |
| `lines[]` の line_cd 集合 | 9駅グループで一致 |
| `hasTrainTypes` | lines[] / 駅本体ともに不一致 0 |

`station_station_types.id` は `ORDER BY sst.id` として停車順序そのものに使われるため、SERIAL の採番順を保つことを `build.rs` で検証している。

### 検証手法の落とし穴

途中で複数回、**検証スクリプト側の不備で誤った結論を出しかけた。**

- gRPC-Web 用の比較スクリプトを GraphQL 化後もそのまま使い、404 を「1件」と誤集計して差分に見えた。さらに以前は「0件中0件が不一致」を一致と表示していた
- Node が TTY 判定で数値に ANSI エスケープを付け、順序不一致と誤判定した
- 比較 SQL に `transport_type` 条件が無く、Worker 側の既定フィルタとの差が差分に見えた

いずれも実装は正しく、スクリプトを直すと一致した。

---

## 実測値

staging (`gql-stg.trainlcd.app`) での測定。日本から東京エッジ (`cf-ray` は NRT)。

```
全18クエリ         : 成功
サーバー処理       : 通常 12〜20ms (接続確立の TLS が 21〜40ms を占める)
keep-alive 20回    : p50=0ms 最大58ms 平均3ms
コールドスタート    : 20回に1回程度、60〜130ms
Worker Startup Time: 3〜7ms (Cloudflare 報告値)
WASM gzip          : 3,199KB (上限10MiBの31%)
```

**実運用でクライアントが接続を使い回す前提なら平均3ms。**

### コールドスタートについて

`wrangler dev` (ローカル workerd) では約200msだったが、**これは本番の指標にならなかった。** 本番の `Worker Startup Time` は 3〜7ms。

コールドスタートの揺れの原因を調べたところ、**データ初期化は主因ではない。** データを一切参照しない `/__ping` が `/__health` と同等かそれ以上に遅いケースがあることで確認した。

`stations.csv` を固定長レコード + 文字列プールへ変換して `&'static str` 参照にする案も試したが、30回程度の測定では有意差が出ず、gzip が 235KB 増えるだけだったため破棄した。同じバイナリ版で p90 が 27ms → 83ms と変動しており、有意差を出すには数百回規模の測定と統計処理が要る水準だった。

なお Workers は Spectre 対策で同期コード中に `Date.now()` が進まないため、プロセス内での区間計測はできない。切り分けは外から分布を比べる形になる。

---

## 作業中に見つかった問題

### Worker 実装側の漏れ (修正済み)

既存 SQL と照合して見つけたもの。いずれも PR 内で修正した。

| 内容 | 影響 |
|---|---|
| `get_by_line_id_vec_with_group_stations` 未実装 | `GetStationsByLineIdList` が 500 |
| `get_by_station_group_id_vec_no_types` が `line_group_cd` を埋めていない | `lines[].station.hasTrainTypes` が常に false |
| `lines_of_groups` が路線の `e_status` を見ていない | 無効化された路線 (成田エクスプレス) が `lines[]` に混ざる |
| `LineRepository::get_by_station_group_id_vec` が通過条件を見ていない | 停車しない系統しか持たない駅の路線が混ざる |
| `TrainTypeRepository::get_by_line_group_id_vec` の並び順 | `priority DESC` で並べていたが SQL は `sst.id` のみ |
| `LineRepository::find_by_station_id` が sst 由来の列を埋めていない | `line_group_cd` / `type_cd` が NULL のまま |

未実装メソッドが `DomainError` を返す設計にしていたことで、1件目は 500 応答として検出できた。黙って空を返していれば正常応答に見えて気付けなかった。

最終的に全 repository メソッド (34個) について、対応する SQL の `WHERE` / `ORDER BY` を機械的に抽出して突き合わせた。

### 親元 (gRPC 版) のバグ

Worker 移行とは独立した、既存実装の問題。gRPC 版で再現を確認して起票した。

- **[#1636](https://github.com/TrainLCD/StationAPI/issues/1636) GetRoutes / EstimateArrivalTimes が特定の駅ペアでパニックする**

  `get_route_stops` の SQL は `WHERE sst.line_group_cd IS NULL` で絞るため、返る駅の `line_group_cd` は必ず NULL。それを受け取る `build_route_tree_map` が `.expect()` しているので、1件でも返れば必ず落ちる。`100410 → 100422` で再現する。

- **[#1637](https://github.com/TrainLCD/StationAPI/issues/1637) GTFS を含むデータ取り込みに約7分半かかる**

  `build_stop_route_mapping` の再帰CTEが単独で63秒。`main.rs` は起動時にこれを実行するため、再起動のたびに同じ時間がかかる。

### 削除したもの

`GetRoutesMinimal` は BFF のスキーマに対応するクエリが無く、どこからも呼ばれていなかったため削除した。proto は submodule なので [TrainLCD/gRPCProto#30](https://github.com/TrainLCD/gRPCProto/pull/30) でマージ済み。

---

## 運用上の注意

**データ更新のたびに再デプロイが要る。** Worker はデータを WASM に埋め込むため、`data/*.csv` や GTFS が変わったらビルドし直す必要がある。オンプレ版のように起動時取り込みで自動反映される運用とは異なる。

**環境の使い分け。** 他の Worker と揃えて、env 省略時を staging にしてある。

```
staging : wrangler deploy --env=""          -> stationapi-stg
本番    : wrangler deploy --env production  -> stationapi
```

wrangler 4 は複数環境がある状態で `--env` を省略すると警告するため、staging を指す場合も `--env=""` を明示する。

**custom domain は二重に登録できない。** ドメインを移す際は、先に元の Worker から外してデプロイする必要がある。

---

## 残作業

- [ ] **[#1638](https://github.com/TrainLCD/StationAPI/issues/1638) 本番へ適用する** — staging での検証後に実施
- [ ] [#1636](https://github.com/TrainLCD/StationAPI/issues/1636) のパニック修正 (方針判断が必要)
- [ ] [#1637](https://github.com/TrainLCD/StationAPI/issues/1637) の取り込み時間の改善
- [ ] CI ワークフローの実行 (未実行。`ODPT_ACCESS_TOKEN` を Secrets に設定すると全フィードが取り込まれる)

---

## 関連

| リポジトリ | 内容 |
|---|---|
| [StationAPI#1635](https://github.com/TrainLCD/StationAPI/pull/1635) | 本体の PR |
| [gRPCProto#30](https://github.com/TrainLCD/gRPCProto/pull/30) | GetRoutesMinimal 削除 (マージ済み) |
| [BFF#51](https://github.com/TrainLCD/BFF/pull/51) | staging の route 削除 (マージ済み) |
