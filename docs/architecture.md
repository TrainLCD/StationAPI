# StationAPI アーキテクチャドキュメント

> 最終更新: 2026年1月

## 目次

- [概要](#概要)
- [レイヤー構造](#レイヤー構造)
- [データベース設計](#データベース設計)
- [gRPC/スキーマ設計](#grpcスキーマ設計)
- [命名規則](#命名規則)
- [キャッシュ戦略](#キャッシュ戦略)
- [データフロー](#データフロー)
- [ディレクトリ構造](#ディレクトリ構造)

---

## 概要

StationAPI は日本の鉄道駅情報を提供する gRPC API です。**クリーンアーキテクチャ**に基づいた4層構造を採用し、ビジネスロジックと技術的関心事を明確に分離しています。

### 技術スタック

| 項目 | 技術 |
|------|------|
| 言語 | Rust (Edition 2021) |
| ランタイム | tokio |
| データベース | PostgreSQL 15+ |
| ORM | sqlx (コンパイル時クエリ検証) |
| API | gRPC (tonic) |
| シリアライズ | Protocol Buffers |

---

## レイヤー構造

StationAPI は4つの層で構成されています。各層は依存性の方向が内側（Domain）に向かうよう設計されています。

```txt
┌─────────────────────────────────────────────────────────┐
│                    Presentation 層                       │
│            (gRPC Controller, エラーハンドリング)           │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                      UseCase 層                          │
│           (Interactor, DTO, ビジネスロジック)              │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                   Infrastructure 層                      │
│        (Repository実装, Row構造体, DB接続)                │
└─────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────┐
│                      Domain 層                           │
│       (Entity, Repository Interface, ビジネスルール)       │
└─────────────────────────────────────────────────────────┘
```

### Domain 層 (`src/domain/`)

**責務**: コアビジネスロジックとデータモデルの定義

| ディレクトリ/ファイル | 内容 |
|---------------------|------|
| `entity/` | ドメインエンティティ（Station, Line, TrainType, Company など） |
| `repository/` | リポジトリインターフェース（`async_trait` を使用） |
| `normalize.rs` | テキスト正規化（ひらがな↔カタカナ、全角↔半角変換） |
| `error.rs` | ドメインエラー型（NotFound, InfrastructureError, Unexpected） |

**設計原則**:
- 外部依存を持たない純粋な Rust コード
- リポジトリは trait として定義し、実装を Infrastructure 層に委譲
- 多言語対応（日本語、カタカナ、ローマ字、中国語、韓国語）

### UseCase 層 (`src/use_case/`)

**責務**: アプリケーションビジネスロジックとデータ変換

| ディレクトリ/ファイル | 内容 |
|---------------------|------|
| `interactor/query.rs` | `QueryInteractor` - 主要なユースケース実装（約950行） |
| `traits/query.rs` | `QueryUseCase` トレイト定義（20以上の非同期メソッド） |
| `dto/` | データ変換オブジェクト（Entity ↔ gRPC メッセージ） |
| `error.rs` | ユースケースエラー型 |

**重要なメソッド**:

```rust
// update_station_vec_with_attributes (query.rs:169-265)
// - 駅データにライン、会社、列車種別を付加
// - N+1問題を回避するバッチクエリ設計
async fn update_station_vec_with_attributes(
    &self,
    mut stations: Vec<Station>,
    line_group_id: Option<u32>,
) -> Result<Vec<Station>, UseCaseError>
```

### Infrastructure 層 (`src/infrastructure/`)

**責務**: データ永続化と外部システム連携

| ファイル | 内容 |
|---------|------|
| `station_repository.rs` | `StationRow` + `MyStationRepository` 実装 |
| `line_repository.rs` | `LineRow` + `MyLineRepository` 実装 |
| `train_type_repository.rs` | `TrainTypeRow` + `MyTrainTypeRepository` 実装 |
| `company_repository.rs` | `CompanyRow` + `MyCompanyRepository` 実装 |

**設計パターン**:
- 各 Repository は `Arc<Pool<Postgres>>` をラップ
- `Internal*Repository` 構造体に実際の SQL 実行を委譲
- `#[derive(sqlx::FromRow)]` による型安全な Row マッピング

### Presentation 層 (`src/presentation/`)

**責務**: 外部 API の公開とリクエスト/レスポンスハンドリング

| ファイル | 内容 |
|---------|------|
| `controller/grpc.rs` | `MyApi` - 14の gRPC エンドポイント実装 |
| `error.rs` | `PresentationalError` と `tonic::Status` への変換 |

---

## データベース設計

### テーブル構成

すべてのテーブルは `UNLOGGED` として作成されパフォーマンスを優先しています。

| テーブル | 主キー | 概要 |
|---------|-------|------|
| `companies` | company_cd | 鉄道会社情報 |
| `lines` | line_cd | 路線情報 |
| `stations` | station_cd | 駅情報 |
| `types` | id | 列車種別 |
| `station_station_types` | id | 駅と列車種別の関連 |
| `line_aliases` | id | 路線エイリアス |
| `connections` | - | 駅間接続 |
| `aliases` | - | 検索用エイリアス |

### パフォーマンス最適化

```sql
-- 使用している PostgreSQL 拡張
CREATE EXTENSION IF NOT EXISTS pg_trgm;    -- トライグラム検索
CREATE EXTENSION IF NOT EXISTS btree_gist; -- GiST インデックス

-- 主要インデックス
CREATE INDEX idx_stations_station_g_cd ON stations(station_g_cd);
CREATE INDEX idx_stations_line_cd ON stations(line_cd);
CREATE INDEX idx_performance_station_name_trgm ON stations
    USING gin(station_name gin_trgm_ops);  -- あいまい検索用
```

### スキーマ更新時の注意点

1. **マイグレーション**: `data/create_table.sql` を更新
2. **Row 構造体**: 対応する `*Row` 構造体を Infrastructure 層で更新
3. **Entity**: 必要に応じて Domain 層の Entity を更新
4. **変換ロジック**: `impl From<XxxRow> for Xxx` を更新
5. **DTO**: gRPC メッセージへの変換を `use_case/dto/` で更新

---

## gRPC/スキーマ設計

### サービスエンドポイント

`stationapi.proto` で14のエンドポイントを定義:

| カテゴリ | メソッド |
|---------|---------|
| 駅検索 | `GetStationById`, `GetStationByIdList`, `GetStationsByGroupId`, `GetStationsByCoordinates`, `GetStationsByLineId`, `GetStationsByName`, `GetStationsByLineGroupId` |
| 路線検索 | `GetLineById`, `GetLinesByIdList`, `GetLinesByName` |
| 経路検索 | `GetRoutes`, `GetRoutesMinimal`, `GetConnectedRoutes` |
| 列車種別 | `GetTrainTypesByStationId`, `GetRouteTypes` |

### Proto 更新時の注意点

1. **後方互換性**: 新フィールドには `optional` キーワードを使用
2. **ビルド設定**: `build.rs` で `serde` トレイトを追加
3. **DTO 更新**: `src/use_case/dto/*.rs` のマッピングを更新
4. **テスト更新**: 新フィールドの統合テストを追加

```protobuf
// 後方互換性のある追加例
message Station {
    // 既存フィールド...
    optional string new_field = 25;  // optional で追加
}
```

---

## 命名規則

### Row 構造体 vs Entity の区別

| 種別 | 場所 | 目的 | 特徴 |
|------|------|------|------|
| **Row** | `infrastructure/*.rs` | DB行の直接マッピング | `#[derive(sqlx::FromRow)]`、DBカラム名と一致 |
| **Entity** | `domain/entity/*.rs` | ドメインモデル | ビジネスロジック、ネスト構造、多言語対応 |

### Row 構造体

```rust
// infrastructure/station_repository.rs
#[derive(sqlx::FromRow, Clone)]
pub struct StationRow {
    pub station_cd: i32,           // DBカラム名と一致
    pub station_g_cd: i32,
    pub station_name: String,
    pub line_cd: i32,
    // ... 約19フィールド
}
```

**特徴**:
- フィールド名は PostgreSQL カラム名と**完全一致**（snake_case）
- データベースネイティブ型を使用: `i32`, `i64`, `f64`, `Option<T>`, `String`
- ロジックを持たない純粋なデータホルダー

### Entity 構造体

```rust
// domain/entity/station.rs
pub struct Station {
    pub station_cd: u32,           // ビジネス型（符号なし）
    pub station_g_cd: u32,
    pub station_name: String,
    pub line: Option<Box<Line>>,   // ネスト構造
    pub lines: Vec<Line>,          // コレクション
    pub train_type: Option<Box<TrainType>>,
    pub station_numbers: Vec<StationNumber>,
    // ... 約66フィールド
}
```

**特徴**:
- ビジネスセマンティクスを反映した型（例: `StopCondition` 列挙型）
- ネスト構造を含む（`Option<Box<Line>>`, `Vec<Line>` など）
- 多言語名をサポート: `station_name_r`（ローマ字）, `station_name_zh`（中国語）, `station_name_ko`（韓国語）
- `Clone`, `Debug`, `Serialize`, `Deserialize`, `PartialEq` を実装

### 変換フロー

```txt
Database (PostgreSQL)
    ↓
Row (sqlx::FromRow)      ← 直接マッピング: StationRow
    ↓
Entity (From<Row>)       ← 型変換、None初期化: Station
    ↓
Enriched Entity          ← UseCase層でネストデータ追加
    ↓
gRPC Message             ← Proto変換: proto::Station
    ↓
Network Response
```

---

## キャッシュ戦略

### 現在の設計: 明示的キャッシュなし

StationAPI は現時点で明示的なインメモリキャッシュを実装していません。その代わり、以下の最適化戦略を採用しています。

### バッチクエリによる暗黙的キャッシュ

`query.rs:169-265` の `update_station_vec_with_attributes` メソッドでは、N+1問題を回避するためにバッチクエリを使用しています。

```rust
// 1. すべての station_g_cd を抽出
let station_group_ids = stations.iter()
    .map(|s| s.station_g_cd as u32)
    .collect::<Vec<u32>>();

// 2. 一括クエリで関連データを取得（N+1回避）
let stations_by_group_ids = self
    .get_stations_by_group_id_vec(&station_group_ids).await?;
let lines = self
    .get_lines_by_station_group_id_vec(&station_group_ids).await?;
let train_types = self
    .get_train_types_by_station_id_vec(&station_ids, line_group_id).await?;

// 3. メモリ上で関連付け（O(1)クエリ/エンリッチメント）
```

**結果**: エンリッチメント処理あたり**O(1)クエリ**（N駅に対してN回のクエリではない）

### HashSet による重複排除

`query.rs:223` 付近でインメモリ重複排除を実施:

```rust
let mut seen_line_cds = std::collections::HashSet::new();
let lines: Vec<Line> = lines
    .iter()
    .filter(|&l| {
        l.station_g_cd.unwrap_or(0) == station.station_g_cd
            && seen_line_cds.insert(l.line_cd)  // HashSetで重複防止
    })
    .cloned()
    .collect();
```

### キャッシュを実装しない理由

1. **データ規模**: 日本の鉄道データは比較的小規模（約9,000駅）
2. **更新頻度**: CSV インポートによるデータ更新が前提
3. **ステートレス設計**: 各リクエストは独立して処理
4. **PostgreSQL の最適化**: インデックスとクエリプランナーによる効率化

### 将来の検討事項

大規模化や高頻度アクセスが必要な場合:
- `moka` や `lru` クレートによる有界インメモリキャッシュ
- CSV インポート時のキャッシュ無効化
- `station_g_cd` 単位のタグベース無効化

---

## データフロー

### 典型的なリクエストフロー

```txt
[Client]
    │
    ▼ gRPC Request
┌──────────────────────────────────────────────┐
│ Presentation 層 (grpc.rs)                     │
│  └─ MyApi::get_stations_by_id()              │
└──────────────────────────────────────────────┘
    │
    ▼ QueryUseCase メソッド呼び出し
┌──────────────────────────────────────────────┐
│ UseCase 層 (query.rs)                         │
│  ├─ QueryInteractor::get_station_by_id()     │
│  └─ update_station_vec_with_attributes()     │
│      ├─ 駅グループ一括取得                      │
│      ├─ 路線一括取得                           │
│      ├─ 会社一括取得                           │
│      └─ 列車種別一括取得                        │
└──────────────────────────────────────────────┘
    │
    ▼ Repository メソッド呼び出し
┌──────────────────────────────────────────────┐
│ Infrastructure 層 (station_repository.rs)     │
│  └─ MyStationRepository::find_by_id()        │
│      └─ SQL クエリ実行 (sqlx)                  │
└──────────────────────────────────────────────┘
    │
    ▼ Row → Entity 変換
┌──────────────────────────────────────────────┐
│ Domain 層 (entity/station.rs)                 │
│  └─ impl From<StationRow> for Station        │
└──────────────────────────────────────────────┘
    │
    ▼ Entity → gRPC Message 変換
┌──────────────────────────────────────────────┐
│ UseCase 層 (dto/station.rs)                   │
│  └─ impl From<Station> for proto::Station    │
└──────────────────────────────────────────────┘
    │
    ▼ gRPC Response
[Client]
```

### エラー伝播チェーン

```txt
DomainError (sqlx エラー等)
    ↓ ?演算子
UseCaseError (ユースケース層)
    ↓ From トレイト
PresentationalError (プレゼンテーション層)
    ↓ Into トレイト
tonic::Status (gRPC ワイヤーフォーマット)
```

---

## ディレクトリ構造

```txt
stationapi/src/
├── domain/                          # コアビジネスロジック
│   ├── entity/                      # ドメインエンティティ
│   │   ├── station.rs               # Station (66フィールド)
│   │   ├── line.rs                  # Line (40フィールド)
│   │   ├── train_type.rs            # TrainType
│   │   ├── company.rs               # Company
│   │   ├── line_symbol.rs           # LineSymbol
│   │   └── station_number.rs        # StationNumber
│   ├── repository/                  # 抽象インターフェース
│   │   ├── station_repository.rs
│   │   ├── line_repository.rs
│   │   ├── train_type_repository.rs
│   │   └── company_repository.rs
│   ├── normalize.rs                 # テキスト正規化
│   └── error.rs                     # DomainError
│
├── use_case/                        # アプリケーションロジック
│   ├── interactor/
│   │   └── query.rs                 # QueryInteractor (約950行)
│   ├── traits/
│   │   └── query.rs                 # QueryUseCase トレイト
│   ├── dto/                         # データ変換
│   │   ├── station.rs
│   │   ├── line.rs
│   │   ├── train_type.rs
│   │   └── company.rs
│   └── error.rs                     # UseCaseError
│
├── infrastructure/                  # データ永続化
│   ├── station_repository.rs        # StationRow + MyStationRepository
│   ├── line_repository.rs           # LineRow + MyLineRepository
│   ├── train_type_repository.rs     # TrainTypeRow + MyTrainTypeRepository
│   ├── company_repository.rs        # CompanyRow + MyCompanyRepository
│   └── error.rs                     # InfrastructureError
│
├── presentation/                    # 外部API
│   ├── controller/
│   │   └── grpc.rs                  # MyApi (14エンドポイント)
│   └── error.rs                     # PresentationalError
│
├── lib.rs                           # モジュール宣言
└── main.rs                          # エントリーポイント
```

---

## 関連ドキュメント

- [技術負債分析レポート](./technical_debt.md)
- [リポジトリテストガイド](./repository_testing.md)
- [データ貢献ガイドライン](../data/README.md)
