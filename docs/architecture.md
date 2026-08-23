# StationAPI アーキテクチャドキュメント

> 最終更新: 2026年8月22日

## 目次

- [概要](#概要)
- [全体構成](#全体構成)
- [レイヤー構造](#レイヤー構造)
- [データパイプライン](#データパイプライン)
- [インメモリ索引](#インメモリ索引)
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
