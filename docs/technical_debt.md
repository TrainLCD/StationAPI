# StationAPI 技術負債分析レポート

> 最終更新: 2025年12月

## 目次

- [概要](#概要)
- [プロジェクト情報](#プロジェクト情報)
- [高優先度の技術負債](#高優先度の技術負債)
- [中優先度の技術負債](#中優先度の技術負債)
- [低優先度の技術負債](#低優先度の技術負債)
- [良好な点](#良好な点)
- [改善提案](#改善提案)

---

## 概要

本ドキュメントは StationAPI プロジェクトの技術負債を分析・整理したものです。技術負債は優先度別に分類され、各項目には該当ファイルと行番号が記載されています。

---

## プロジェクト情報

| 項目 | 内容 |
|------|------|
| 言語 | Rust (Edition 2021) |
| アーキテクチャ | クリーンアーキテクチャ (Domain/UseCase/Infrastructure/Presentation) |
| 主要依存関係 | tokio 1.28.0, sqlx 0.8.3, tonic 0.12.3 |
| コード規模 | 約 10,600 行 (Rust) |
| データ | 8つの CSV ファイル (日本の鉄道データ) |

---

## 高優先度の技術負債

### 1. 過大な構造体設計

#### Station 構造体

- **ファイル**: `stationapi/src/domain/entity/station.rs:8-76`
- **フィールド数**: 66個
- **問題点**:
  - 駅情報、路線情報、列車種別情報が1つの構造体に混在
  - `Line`, `TrainType`, `StationNumber` などの関連データを包含
  - 責務分離が不明確
  - 線号シンボル (`symbol1-4`) と色・形状の組み合わせが手動管理

```rust
pub struct Station {
    // 駅情報 (station_cd, station_g_cd, station_name, ...)
    // 路線情報 (line_cd, line, lines, line_name, line_symbol1, ...)
    // 列車種別情報 (train_type, type_name, ...)
    // 合計66フィールド
}
```

#### Line 構造体

- **ファイル**: `stationapi/src/domain/entity/line.rs:6-41`
- **フィールド数**: 39個以上
- **問題点**:
  - `Station` の埋め込み参照を含む (循環参照の可能性)
  - `TrainType` の埋め込み参照を含む
  - 線号シンボルが4つまで (`line_symbol1-4`) に制限 → スケーラビリティ問題

#### StationRow 構造体

- **ファイル**: `stationapi/src/infrastructure/station_repository.rs:19-79`
- **フィールド数**: 79個
- **問題点**:
  - 複数テーブルから大量のカラムを JOIN で取得
  - Row 構造体と Entity の変換が複雑

#### Clippy 警告の抑制

以下の箇所で `#[allow(clippy::too_many_arguments)]` が使用されています:

| ファイル | 構造体 |
|----------|--------|
| `src/domain/entity/station.rs:79` | Station |
| `src/domain/entity/line.rs:43` | Line |
| `src/domain/entity/train_type.rs:25` | TrainType |
| `src/domain/entity/company.rs:20` | Company |

---

### 2. SQL クエリの未最適化 (TODO 対応必須)

アプリケーション層でデータベースから全データを取得後、メモリ上でフィルタリングを行っている箇所があります。

| ファイル | 行番号 | 内容 |
|----------|--------|------|
| `stationapi/src/use_case/interactor/query.rs` | 604 | `// TODO: SQLで同等の処理を行う` - 経路検証がアプリケーション側で実行 |
| `stationapi/src/use_case/interactor/query.rs` | 702 | `// TODO: SQLで同等の処理を行う` - 経路フィルタリングがアプリケーション層で処理 |
| `stationapi/src/use_case/interactor/query.rs` | 843 | `// TODO: 未実装` - `get_connected_stations()` が空配列を返却 |

```rust
// query.rs:604-610
// TODO: SQLで同等の処理を行う
let includes_requested_station = stops
    .iter()
    .any(|stop| stop.group_id == from_station_id || stop.group_id == to_station_id);
```

**影響**: パフォーマンス低下の可能性

---

### 3. 過度な clone() の使用

- **ファイル**: `stationapi/src/use_case/interactor/query.rs`
- **clone() 呼び出し回数**: 94回

主な箇所:

| 行番号 | 内容 |
|--------|------|
| 508, 578 | `line.clone()` |
| 209, 244 | `station.clone()`, `station_ref.clone()` |
| 230-231 | ベクタフィルタリング時の clone |

```rust
// 行230-231: ベクタ操作時のクローン
let mut lines: Vec<Line> = lines
    .iter()
    .filter(|&l| { ... })
    .cloned()      // <-- 66フィールドの構造体を全てクローン
    .collect();
```

**影響**: Station が 66 フィールド × clone → メモリ使用量増加、不要なアロケーション

---

## 中優先度の技術負債

### 4. メソッド命名の問題

| ファイル | 行番号 | 問題 |
|----------|--------|------|
| `stationapi/src/domain/repository/line_repository.rs` | 23 | `// FIXME: もっとマシな命名` - `get_by_line_group_id_vec_for_routes()` |

命名規則が不明確で、メソッドの意図が分かりにくい。

---

### 5. 複雑な SQL クエリ

- **ファイル**: `stationapi/src/infrastructure/station_repository.rs:950-1088`
- **クエリ長**: 140行以上のマルチレベル CTE (Common Table Expression)

**問題点**:
- 駅名検索で複数の言語フィールド (`LIKE $2-$6`) をサポート
- 同等の処理が複数メソッドで繰り返される
- クエリの設計意図がドキュメント化されていない

**繰り返されるクエリパターン**:
- `find_by_id()`: 基本的な単一駅取得
- `get_by_line_id()`: 路線別駅取得
- `get_by_station_group_id()`: グループ別駅取得
- `get_route_stops()`: 経路駅停止条件処理

---

### 6. 死んだコード (Dead Code)

```rust
// stationapi/src/infrastructure/station_repository.rs:25
#[allow(dead_code)]
pub station_name_rn: Option<String>,
```

---

### 7. ハードコードされた値

| ファイル | 行番号 | 値 | 用途 |
|----------|--------|-----|------|
| `stationapi/src/infrastructure/station_repository.rs` | 1494 | `"99991231"` | 閉鎖駅の終了日付 |
| `stationapi/src/domain/normalize.rs` | 8 | `0x60` | Unicode 正規化 |
| `stationapi/src/domain/normalize.rs` | 11, 14 | `0xFEE0` | Unicode 正規化 |

これらの値は定数として定義し、意味を明確にすべきです。

---

### 8. マッピング処理の複雑性

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

### 9. テスト関連

#### 現状

- **テスト関数数**: 200個
- **テスト範囲**: Repository 層中心
- **テストドキュメント**: `docs/repository_testing.md`

#### 不足している領域

| 領域 | 状態 |
|------|------|
| gRPC コントローラーテスト | `src/presentation/controller/grpc.rs` (353行) がテスト対象外 |
| End-to-End テスト | なし |
| パフォーマンステスト | なし |

---

### 10. ドキュメント不足

#### 存在するドキュメント

- `README.md`: プロジェクト概要
- `docs/repository_testing.md`: テスト詳細ガイド
- `data/README.md`: データ構造説明
- `AGENTS.md`: 自動化エージェント向けドキュメント

#### 不足している領域

| 領域 | 内容 |
|------|------|
| アーキテクチャドキュメント | 3層構造の設計思想が文書化されていない |
| SQL 設計ドキュメント | 複雑なクエリの使用意図が不明確 |
| 命名規則 | Row 構造体と Entity の区別が不明確 |
| キャッシュ戦略 | 判断理由が未文書化 (query.rs:217付近) |

---

## 良好な点

### セキュリティ

- **Unsafe コード**: なし
- **SQL インジェクション対策**: sqlx! マクロで型安全
- **認証・認可**: gRPC レベルで実装あり

### CI/CD パイプライン

- **ファイル**: `.github/workflows/ci.yml`
- **実行内容**:
  - `cargo check` - コンパイルチェック
  - `cargo test` - テスト実行
  - `cargo fmt --check` - コードフォーマット検証
  - `cargo clippy -- -D warnings` - Lint チェック (警告は ERROR)

### 依存関係

| パッケージ | バージョン | 状態 |
|-----------|----------|------|
| tokio | 1.28.0 | 問題なし |
| sqlx | 0.8.3 | 最新近い |
| tonic | 0.12.3 | 最新近い |
| serde | 1.0.189 | 最新 |

### エラーハンドリング

- 17個のエラーハンドリングテストが実装済み

---

## 改善提案

### 短期改善

1. **SQL 最適化**: `get_route_stops` でのフィルタリングを SQL 側に移動
2. **Clone 削減**: 参照ベースの処理を検討
3. **命名改善**: `get_by_line_group_id_vec_for_routes()` をより明確な名前に変更
4. **定数化**: ハードコードされた値を定数として定義

### 中期改善

1. **Station 構造体リファクタリング**
   - `StationCore` (基本情報) と `StationDetails` (関連データ) に分割
2. **DTO レイヤーの標準化**
   - 自動コード生成ツール導入
   - Row → Entity → Protobuf の一貫性確保
3. **プレゼンテーション層テスト**
   - gRPC controller テスト追加

### 長期改善

1. **パフォーマンス最適化**
   - クエリ計画の再検討
   - キャッシング戦略の導入
2. **エラーハンドリング統一**
   - domain, use_case, presentation 層での戦略統一

---

## 優先度別サマリー

| 優先度 | 項目 | ファイル | 影響 |
|--------|------|---------|------|
| **高** | Station 構造体の設計見直し | `src/domain/entity/station.rs` | 保守性、パフォーマンス |
| **高** | SQL クエリの最適化 (TODO対応) | `src/use_case/interactor/query.rs:604,702` | パフォーマンス |
| **高** | Clone の過度な使用削減 | `src/use_case/interactor/query.rs` | メモリ効率 |
| **中** | Row 構造体のコード生成検討 | `src/infrastructure/*.rs` | メンテナンス性 |
| **中** | メソッド命名の改善 | `src/domain/repository/line_repository.rs:23` | 可読性 |
| **中** | ハードコード値の定数化 | 複数ファイル | 保守性 |
| **低** | get_connected_stations の実装 | `src/use_case/interactor/query.rs:843` | 機能完成度 |
| **低** | UI レイヤーのテスト追加 | `src/presentation/` | テストカバレッジ |
