# Contributing to StationAPI

StationAPIへのコントリビュートに興味を持っていただきありがとうございます！

## はじめに

StationAPIは日本の鉄道駅・バス停の情報を提供する GraphQL API です。Cloudflare Workers 上で動きます。
コード・データの両面でコントリビューションを歓迎しています。

## 開発環境のセットアップ

### 必要なツール

- **Rust** (stable toolchain): `rustup default stable`
- **wasm32 ターゲット**: `rustup target add wasm32-unknown-unknown`
- **worker-build**: `cargo install worker-build --locked`
- **wrangler** (ローカル実行・デプロイ用): `npm i -g wrangler`

データベースは不要です。データは `generated/*.csv` としてビルド時に WASM へ埋め込みます。

### ローカルでの起動

```bash
# 1. Worker が読むデータを作る (data/*.csv と GTFS から組み立てる)
make data

# 2. Worker をビルドしてローカルで動かす
make build
make dev        # http://127.0.0.1:8787
```

`generated/` が無いまま `make build` すると `data/*.csv` にフォールバックします。
その場合は各駅停車の生成系統とバスのデータが欠けた状態になります (警告が出ます)。

バスの一部フィードは `ODPT_ACCESS_TOKEN` を必要とします。設定が無い場合は
トークン不要な都営バスだけが取り込まれます。鉄道だけで十分なときは
`DISABLE_BUS_FEATURE=true make data` としてください。

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
- 新しいクエリを追加する場合は [AGENTS.md](AGENTS.md) のアーキテクチャパターンを参照してください
- 公開スキーマを変える場合は `schema/public.graphql` も更新してください。
  CI が Worker の SDL と突き合わせ、差分があれば失敗します

#### データ変更の場合

- データの構造については [data/README.md](data/README.md) を参照してください
- CSVファイルは `data/` ディレクトリに `N!table.csv` の命名規則で配置されています
- データバリデーションは `cargo run -p data_validator` で実行できます

### 4. コミット前のチェック

コミットする前に以下のチェックを必ず実行してください：

```bash
make fmt      # フォーマットチェック
make clippy   # Lint (wasm32 ターゲットを含む)
make test     # テスト
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
