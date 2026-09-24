# StationAPI 技術負債分析レポート

> 分析時点: 2026年1月 (冒頭の注記と「現状」の注記は 2026年9月に追記)
>
> **注意: 本書は Cloudflare Workers へ移行する前 (gRPC + PostgreSQL 構成) の分析です。**
> 本文の指摘、ファイル名、行番号はすべて分析した時点のものです。現在の構成は
> [architecture.md](./architecture.md) を参照してください。
>
> 移行 (#1640) で `stationapi/src/infrastructure/` (sqlx のリポジトリ)、
> `stationapi/src/presentation/` (tonic)、`stationapi/src/import.rs`
> (PostgreSQL への取り込み) は削除され、`StationRow` もなくなりました。
> そのため「SQL クエリの未最適化」「複雑な SQL クエリ」「デッドコード」
> 「gRPC コントローラーテスト」といった項目は、対象のコードごとなくなっています。
> 該当する項目には「現状」の注記を付けました。
>
> 一方、domain / use_case 層への次の指摘は今も当てはまります。
>
> - `Station` エンティティのフィールドが多い (現在 65 個)。`Line` も 34 個あり、
>   `Station` と `TrainType` を埋め込んだままです
> - `Station` / `Line` / `TrainType` / `Company` の impl ブロックに
>   `#![allow(clippy::too_many_arguments)]` が残っています
> - clone が多い (例: `query.rs` の `line.station = Some(station.clone())`)
> - ハードコードされた値 (`normalize.rs` の `0x60` / `0xFEE0`)
> - `FIXME` 付きのメソッド名 `get_by_line_group_id_vec_for_routes`
> - 駅ナンバリングと路線記号を 1〜4 番まで手作業で並べるマッピング処理

## 目次

- [概要](#概要)
- [プロジェクト情報](#プロジェクト情報)
- [高優先度の技術負債](#高優先度の技術負債)
- [中優先度の技術負債](#中優先度の技術負債)
- [低優先度の技術負債](#低優先度の技術負債)
- [良好な点](#良好な点)
- [改善提案](#改善提案)
- [優先度別サマリー](#優先度別サマリー)

---

## 概要

StationAPI の技術負債を洗い出し、整理したドキュメントです。項目は優先度別に
分け、それぞれに該当するファイルと行番号を記載しています。

---

## プロジェクト情報

| 項目 | 内容 |
|------|------|
| 言語 | Rust (Edition 2021) |
| アーキテクチャ | クリーンアーキテクチャ (Domain/UseCase/Infrastructure/Presentation) ※分析時点 |
| 主要な依存関係 | tokio 1.28.0, sqlx 0.8.3, tonic 0.12.3 |
| コード規模 | 約 10,600 行 (Rust) |
| データ | 8 つの CSV ファイル (日本の鉄道データ) |

> **現状 (2026年9月)**: sqlx と tonic は依存から外れ、GraphQL は async-graphql 7、
> 実行環境は Cloudflare Workers (`worker` 0.8) です。レイヤー構成は
> Presentation / Model / UseCase / Domain / Index に変わりました
> ([architecture.md](./architecture.md#レイヤー構造))。

---

## 高優先度の技術負債

### 1. 肥大化した構造体

#### Station 構造体

- **ファイル**: `stationapi/src/domain/entity/station.rs:8-76`
- **フィールド数**: 64 個
- **問題点**:
  - 駅・路線・列車種別の情報が 1 つの構造体に混在している
  - `Line`、`TrainType`、`StationNumber` などの関連データを抱え込んでいる
  - 責務の境界がはっきりしない
  - 路線記号 (`symbol1-4`) と、その色・形の組み合わせを手作業で管理している

```rust
pub struct Station {
    // 駅情報 (station_cd, station_g_cd, station_name, ...)
    // 路線情報 (line_cd, line, lines, line_name, line_symbol1, ...)
    // 列車種別情報 (train_type, type_name, ...)
    // 合計64フィールド
}
```

#### Line 構造体

- **ファイル**: `stationapi/src/domain/entity/line.rs:6-41`
- **フィールド数**: 33 個
- **問題点**:
  - `Station` を埋め込んでいる (循環参照になるおそれがある)
  - `TrainType` も埋め込んでいる
  - 路線記号が 4 つ (`line_symbol1-4`) までしか持てず、拡張しにくい

#### StationRow 構造体

- **ファイル**: `stationapi/src/infrastructure/station_repository.rs:19-79`
- **フィールド数**: 58 個 (初版では 79 個と書いていましたが、79 は定義の最終行の
  行番号でした)
- **問題点**:
  - 複数のテーブルを JOIN して大量のカラムを取得している
  - Row 構造体から Entity への変換が複雑

> **現状 (2026年9月)**: `infrastructure/` ごと削除され、`StationRow` も存在しません。

#### Clippy 警告の抑制

次の impl ブロックで `#![allow(clippy::too_many_arguments)]` を使っています。

| ファイル | 構造体 |
|----------|--------|
| `src/domain/entity/station.rs:79` | Station |
| `src/domain/entity/line.rs:43` | Line |
| `src/domain/entity/train_type.rs:25` | TrainType |
| `src/domain/entity/company.rs:20` | Company |

---

### 2. SQL クエリの未最適化 (TODO への対応が必要)

データベースから全件を取得した後、アプリケーション側のメモリ上で絞り込んでいる
箇所があります。

| ファイル | 行番号 | 内容 |
|----------|--------|------|
| `stationapi/src/use_case/interactor/query.rs` | 604 | `// TODO: SQLで同等の処理を行う` - 経路の検証をアプリケーション側で実行 |
| `stationapi/src/use_case/interactor/query.rs` | 702 | `// TODO: SQLで同等の処理を行う` - 経路の絞り込みをアプリケーション層で実行 |

```rust
// query.rs:604-610
// TODO: SQLで同等の処理を行う
let includes_requested_station = stops
    .iter()
    .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
```

**影響**: パフォーマンスが落ちる可能性があります。

> **現状 (2026年9月)**: 対象コードごと削除済みです。SQL はなくなり、検索はすべて
> インメモリ索引に対して行います。2 つの TODO コメントも残っていません。

---

### 3. clone() の多用

> **ステータス**: ✅ **対応済み** (2026年1月)
>
> 次の最適化を行いました。

#### 対応済みの改善

| 改善内容 | 詳細 |
|----------|------|
| HashMap による検索 | O(n) の線形探索を O(1) の HashMap 検索に変更 (Company, TrainType, Station) |
| `build_route_tree_map` の参照化 | `BTreeMap<i32, Vec<Station>>` → `BTreeMap<i32, Vec<&Station>>` にして Station の clone を回避 |
| `train_types.clone()` の削除 | ベクター全体の clone をやめ、必要な要素だけを HashMap に格納 |
| バス停検索の最適化 | `get_nearby_bus_lines` を HashMap による検索に変更 |

#### 残っている clone()

次の clone() は、構造体のフィールドに所有権を移すために必要で、避けられません。

- `line.station = Some(station.clone())` - Line 構造体が `Option<Station>` を所有している
- `line.company = ...` - Line 構造体が `Option<Company>` を所有している
- 絞り込んだ後に Vec を組み立てるときの `.cloned()`

---

## 中優先度の技術負債

### 4. メソッド名が分かりにくい

| ファイル | 行番号 | 問題 |
|----------|--------|------|
| `stationapi/src/domain/repository/line_repository.rs` | 23 | `// FIXME: もっとマシな命名` - `get_by_line_group_id_vec_for_routes()` |

命名の規則がはっきりせず、メソッドの意図が読み取りにくくなっています。

---

### 5. 複雑な SQL クエリ

- **ファイル**: `stationapi/src/infrastructure/station_repository.rs:950-1088`
- **クエリの長さ**: 140 行を超える多段の CTE (Common Table Expression)

**問題点**:
- 駅名検索が複数言語のフィールド (`LIKE $2-$6`) に対応している
- 同じような処理が複数のメソッドで繰り返されている
- クエリの設計意図が文書化されていない

**繰り返されているクエリのパターン**:
- `find_by_id()`: 駅を 1 件取得する
- `get_by_line_id()`: 路線ごとに駅を取得する
- `get_by_station_group_id()`: 駅グループごとに駅を取得する
- `get_route_stops()`: 経路上の駅と停車条件を処理する

> **現状 (2026年9月)**: 対象コードごと削除済みです。

---

### 6. デッドコード

```rust
// stationapi/src/infrastructure/station_repository.rs:25
#[allow(dead_code)]
pub station_name_rn: Option<String>,
```

> **現状 (2026年9月)**: 対象コードごと削除済みです。`#[allow(dead_code)]` は
> リポジトリのどこにも残っていません。

---

### 7. ハードコードされた値

| ファイル | 行番号 | 値 | 用途 |
|----------|--------|-----|------|
| `stationapi/src/infrastructure/station_repository.rs` | 1494 | `"99991231"` | 廃止駅の終了日付 |
| `stationapi/src/domain/normalize.rs` | 8 | `0x60` | ひらがな → カタカナ変換のコードポイント差 |
| `stationapi/src/domain/normalize.rs` | 11, 14 | `0xFEE0` | 全角英数字 → 半角変換のコードポイント差 |

これらの値は定数として定義し、意味が分かるようにすべきです。

補足: `station_repository.rs:1494` は `#[cfg(test)]` (1466 行目から) の中にある
テスト用データでした。

> **現状 (2026年9月)**: `station_repository.rs` は削除済みです。`normalize.rs` の
> `0x60` / `0xFEE0` は同じ行に残っています。

---

### 8. マッピング処理の煩雑さ

- **ファイル**: `stationapi/src/use_case/interactor/query.rs:292-349`

```rust
// 線号シンボル(1-4)を手動で配列に変換
let line_symbols_raw = [
    &station.line_symbol1,
    &station.line_symbol2,
    &station.line_symbol3,
    &station.line_symbol4,
];
let station_numbers_raw = [
    station.station_number1.as_deref().unwrap_or_default(),
    // ... (4つすべて手動で列挙)
];
```

---

## 低優先度の技術負債

### 9. アーキテクチャドキュメントの不足

> **ステータス**: ✅ **対応済み** (2026年1月)
>
> [docs/architecture.md](./architecture.md) に次の内容をまとめました。

#### 対応済みの領域

| 領域 | 対応状況 |
|------|----------|
| アーキテクチャドキュメント | ✅ 4 層構造 (Domain/UseCase/Infrastructure/Presentation) の設計思想を文書化 |
| 命名規則 | ✅ Row 構造体と Entity の違いを明記 |
| キャッシュ戦略 | ✅ バッチクエリによる暗黙のキャッシュと、その設計判断を文書化 (query.rs:169-265) |
| データフロー | ✅ リクエストの流れとエラーの伝播経路を図示 |

#### 残っている課題

| 領域 | 内容 |
|------|------|
| SQL の設計ドキュメント | 複雑なクエリの意図がインラインコメントにしか書かれていない |

> **現状 (2026年9月)**: architecture.md は移行後の構成に合わせて書き直しました。
> SQL はなくなったため、SQL の設計ドキュメントという課題もなくなりました。

---

### 10. テスト

#### 分析時点の状況

- **テスト関数の数**: 200 個
- **テストの範囲**: Repository 層が中心

#### 足りない領域

| 領域 | 状態 |
|------|------|
| gRPC コントローラーのテスト | `src/presentation/controller/grpc.rs` (353 行) がテストされていない |
| End-to-End テスト | なし |
| パフォーマンステスト | なし |

> **現状 (2026年9月)**: `presentation/` は削除済みです。

---

## 良好な点

### セキュリティ

- **unsafe コード**: なし
- **SQL インジェクション対策**: sqlx のマクロ (`query_as!` など) と、
  プレースホルダへのバインドを使っている
- **認証・認可**: 初版では「gRPC レベルで実装あり」と書いていましたが、分析時点の
  コード (`stationapi/src/`) には認証・認可の処理は見当たりません

> **現状 (2026年9月)**: unsafe コードは今もありません。sqlx は依存から外れました。

### CI/CD パイプライン

- **ファイル**: `.github/workflows/ci.yml`
- **実行内容**:
  - `cargo check` - コンパイルチェック
  - `cargo test` - テストの実行
  - `cargo fmt --check` - フォーマットの検証
  - `cargo clippy -- -D warnings` - Lint (警告をエラーとして扱う)

> **現状 (2026年9月)**: `ci.yml` はネイティブの crate (`stationapi`、
> `stationapi-preprocessor`、`data_validator`) と wasm32 向けの
> `stationapi-worker` を分けて `cargo check` / `cargo clippy` しています。
> `cargo test` の対象はネイティブの 3 crate だけで、`stationapi-worker` の
> テストは含まれていません。

### 依存関係

| パッケージ | バージョン | 状態 |
|-----------|----------|------|
| tokio | 1.28.0 | 問題なし |
| sqlx | 0.8.3 | ほぼ最新 |
| tonic | 0.12.3 | ほぼ最新 |
| serde | 1.0.189 | 最新 |

> **現状 (2026年9月)**: sqlx と tonic は依存から外れました。tokio は
> `stationapi` で `macros` フィーチャーだけを使い、ランタイムは持ち込んでいません。

### エラーハンドリング

- エラーハンドリングのテストが 17 個あります。

---

## 改善提案

### 短期

1. **SQL の最適化**: `get_route_stops` での絞り込みを SQL 側に移す
   (現状: SQL ごと削除済み)
2. ~~**clone の削減**: 参照ベースの処理を検討する~~ ✅ 対応済み
3. **命名の改善**: `get_by_line_group_id_vec_for_routes()` をより分かりやすい名前にする
4. **定数化**: ハードコードされた値を定数として定義する

### 中期

1. **Station 構造体のリファクタリング**
   - `StationCore` (基本情報) と `StationDetails` (関連データ) に分割する
2. **DTO レイヤーの標準化**
   - コードの自動生成ツールを導入する
   - Row → Entity → Protobuf の一貫性を保つ
   - 現状: Row 構造体と Protobuf はなくなりました
3. **プレゼンテーション層のテスト**
   - gRPC コントローラーのテストを追加する
   - 現状: gRPC コントローラーは削除済みです

### 長期

1. **パフォーマンスの最適化**
   - クエリプランを見直す (現状: SQL ごと削除済み)
   - キャッシュ戦略を導入する
2. **エラーハンドリングの統一**
   - domain、use_case、presentation の各層で方針を揃える

---

## 優先度別サマリー

| 優先度 | 項目 | ファイル | 影響 | 現状 (2026年9月) |
|--------|------|---------|------|------------------|
| **高** | Station 構造体の設計見直し | `src/domain/entity/station.rs` | 保守性、パフォーマンス | 未対応 (65 フィールド) |
| **高** | SQL クエリの最適化 (TODO 対応) | `src/use_case/interactor/query.rs:604,702` | パフォーマンス | 対象コードごと削除済み |
| ~~高~~ | ~~clone の多用の削減~~ | ✅ 対応済み (HashMap 検索、参照化) | メモリ効率 | - |
| ~~高~~ | ~~アーキテクチャドキュメントの作成~~ | ✅ 対応済み ([docs/architecture.md](./architecture.md)) | オンボーディング、保守性 | - |
| **中** | Row 構造体のコード生成の検討 | `src/infrastructure/*.rs` | 保守性 | 対象コードごと削除済み |
| **中** | メソッド名の改善 | `src/domain/repository/line_repository.rs:23` | 可読性 | 未対応 |
| **中** | ハードコード値の定数化 | 複数ファイル | 保守性 | `normalize.rs` の分が未対応 |
| **低** | UI レイヤーのテスト追加 | `src/presentation/` | テストカバレッジ | 対象コードごと削除済み |
