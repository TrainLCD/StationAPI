# Repository Layer Testing Guide

このドキュメントでは、StationAPI の Repository Layer のテストについて説明します。

## テスト構成

Repository Layer のテストは以下の 3 つのファイルに分かれています：

1. `line_repository.rs` - 路線データのリポジトリテスト
2. `company_repository.rs` - 鉄道会社データのリポジトリテスト
3. `train_type_repository.rs` - 列車種別データのリポジトリテスト

## テストの種類

### 1. ユニットテスト

データベース接続を必要としない、純粋なユニットテストです。

- **構造体変換テスト**: `Row` 構造体から `Entity` 構造体への変換ロジックをテスト
- **リポジトリインスタンス化テスト**: リポジトリクラスの正常なインスタンス化をテスト

### 2. 統合テスト

実際の PostgreSQL データベースを使用する統合テストです。feature flag `integration-tests` で制御されます。

- **データベースアクセステスト**: 実際のクエリ実行とデータ取得をテスト
- **条件フィルタリングテスト**: WHERE 句や条件分岐の動作をテスト
- **エラーハンドリングテスト**: データベースエラーやデータ不整合の処理をテスト

## テスト実行方法

### ユニットテストのみ実行

```bash
# Cargoを直接使用
cargo test --lib --package stationapi

# Makefileを使用
make test-unit
```

### 統合テストの実行

統合テストを実行するには、事前に PostgreSQL サーバーの準備と環境変数の設定が必要です。

```bash
# 環境変数を設定して統合テストを実行
source .env.test
cargo test --lib --package stationapi --features integration-tests

# または Makefileを使用
make test-integration
```

### 全テストの実行

```bash
# ユニットテスト + 統合テスト
make test-all
```

## テストデータベース設定

統合テストでは、テスト専用の PostgreSQL データベースを使用します。

### 環境変数

```bash
TEST_DATABASE_URL=postgres://test:test@localhost/stationapi_test
```

### データベース準備

統合テストは以下の手順でデータベースを準備します：

1. テスト用テーブルの削除（存在する場合）
2. テスト用テーブルの作成
3. テストデータの挿入
4. テスト実行
5. テスト用テーブルの削除（クリーンアップ）

## テストケース詳細

### LineRepository テスト

**ユニットテスト:**

- `test_line_row_to_line_conversion`: 正常なデータ変換
- `test_line_row_to_line_conversion_with_defaults`: NULL 値のデフォルト処理
- `test_my_line_repository_new`: リポジトリインスタンス化

**統合テスト:**

- `test_find_by_id_*`: ID による路線検索
- `test_find_by_station_id_*`: 駅 ID による路線検索（エイリアス考慮）
- `test_get_by_ids_*`: 複数 ID での路線取得
- `test_get_by_station_group_id_*`: 駅グループ ID での路線取得
- `test_get_by_line_group_id_*`: 路線グループ ID での路線取得
- `test_get_by_name_*`: 路線名での検索

### CompanyRepository テスト

**ユニットテスト:**

- `test_company_row_to_company_conversion`: 正常なデータ変換
- `test_company_row_to_company_conversion_with_null_url`: NULL URL の処理
- `test_my_company_repository_new`: リポジトリインスタンス化

**統合テスト:**

- `test_find_by_id_vec_*`: 複数 ID での会社情報取得

### TrainTypeRepository テスト

**ユニットテスト:**

- `test_train_type_row_to_train_type_conversion`: 正常なデータ変換
- `test_train_type_row_to_train_type_conversion_with_nulls`: NULL 値の処理
- `test_my_train_type_repository_new`: リポジトリインスタンス化

**統合テスト:**

- `test_get_by_station_id_*`: 駅 ID での列車種別取得
- `test_get_by_station_id_vec_*`: 複数駅 ID での列車種別取得
- `test_get_by_line_group_id_vec_*`: 路線グループ ID での列車種別取得

## テスト拡張時の注意点

1. **データベーステスト**: 必ず `#[ignore]` アトリビュートを付与
2. **テストデータ**: `setup_test_data()` と `cleanup_test_data()` を適切に使用
3. **並行実行**: データベーステストは並行実行を避ける設計
4. **型変換**: u32 ↔ i32, u32 ↔ i64 の変換が適切に行われることを確認

## カバレッジ

現在のテストは以下をカバーしています：

- ✅ 全ての public メソッド
- ✅ データ変換ロジック
- ✅ エラーハンドリング
- ✅ エッジケース（空配列、存在しない ID 等）
- ✅ 条件フィルタリング（e_status, pass フィールド等）
- ✅ エイリアス処理（line_repository）

これらのテストにより、Repository Layer の動作が正確であることを保証し、リファクタリングや機能追加時の回帰を防ぐことができます。
