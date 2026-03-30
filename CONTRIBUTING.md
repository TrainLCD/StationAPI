# Contributing to StationAPI

StationAPIへのコントリビュートに興味を持っていただきありがとうございます！

## はじめに

StationAPIは日本の鉄道駅情報を提供するgRPC APIです。コード・データの両面でコントリビューションを歓迎しています。

## 開発環境のセットアップ

### 必要なツール

- **Rust** (stable toolchain): `rustup default stable`
- **protoc** (Protocol Buffers コンパイラ): `sudo apt-get install protobuf-compiler`
- **PostgreSQL** 15+ (`pg_trgm`, `btree_gist` 拡張が必要)
- **Docker / Docker Compose** (推奨)

### ローカルでの起動

```bash
# Docker Composeで起動（PostgreSQL + API）
docker compose up

# もしくは手動で起動
# 1. PostgreSQLを準備し、DATABASE_URLを設定
# 2. APIを起動
cargo run -p stationapi
```

### オフラインビルド

PostgreSQLが利用できない環境では、環境変数を設定してビルドできます：

```bash
SQLX_OFFLINE=true cargo build
```

## コントリビューションの流れ

### 1. Issueの確認・作成

- 既存のIssueを確認し、重複がないか確認してください
- 新しい機能やバグ修正に取り組む前に、Issueを作成して相談することをお勧めします

### 2. ブランチの作成

以下の命名規則に従ってブランチを作成してください：

| 種類 | プレフィックス | 例 |
|------|------------|-----|
| 新機能 | `feature/` | `feature/add-new-rpc` |
| バグ修正 | `fix/` | `fix/station-query-error` |
| データ変更 | `data/` | `data/update-numbering` |
| 雑務 | `chore/` | `chore/update-deps` |
| リリース | `release/` | `release/v1.2.0` |

### 3. 変更の実装

#### コード変更の場合

- Rustの標準的なコーディング規約に従ってください
- 新しいRPCを追加する場合は [AGENTS.md](AGENTS.md) のアーキテクチャパターンを参照してください

#### データ変更の場合

- データの構造については [data/README.md](data/README.md) を参照してください
- CSVファイルは `data/` ディレクトリに `N!table.csv` の命名規則で配置されています
- データバリデーションは `cargo run -p data_validator` で実行できます

### 4. コミット前のチェック

コミットする前に以下のチェックを必ず実行してください：

```bash
# フォーマットチェック
cargo fmt --all -- --check

# Lintチェック
SQLX_OFFLINE=true cargo clippy -- -D warnings

# テスト
SQLX_OFFLINE=true cargo test
```

### 5. Pull Requestの作成

- `dev` ブランチに向けてPRを作成してください
- PRテンプレートに従って説明を記入してください
- 関連するIssueがあればリンクしてください

## データコントリビューション

鉄道データの修正・追加は特に歓迎しています。詳細は [data/README.md](data/README.md) を参照してください。

## 質問・相談

- [Discord コミュニティ](https://discord.gg/tsemdME9Nz)で質問やディスカッションができます
- Issueでの質問も歓迎しています

## ライセンス

コントリビューションは本プロジェクトのライセンスに従います。
