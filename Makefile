# StationAPI Makefile
# Cargoを使ったテスト実行のためのシンプルなタスク定義

.PHONY: test test-unit test-integration test-all clean help

# デフォルトターゲット
help:
	@echo "Available targets:"
	@echo "  test-unit        - Run unit tests only (no database required)"
	@echo "  test-integration - Run integration tests (requires PostgreSQL)"
	@echo "  test-all         - Run all tests"
	@echo "  test             - Alias for test-unit"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "Environment variables:"
	@echo "  TEST_DATABASE_URL - Database URL for integration tests"
	@echo "                      (default: postgres://test:test@localhost/stationapi_test)"

# ユニットテストのみ実行（データベース不要）
test-unit:
	@echo "Running unit tests (no database required)..."
	cargo test --lib --package stationapi

# 統合テストのみ実行（データベース必要）
test-integration:
	@echo "Running integration tests (requires PostgreSQL)..."
	@echo "Make sure PostgreSQL is running and TEST_DATABASE_URL is set"
	cargo test --lib --package stationapi --features integration-tests

# 全てのテストを実行
test-all: test-unit test-integration

# デフォルトはユニットテスト
test: test-unit

# クリーンアップ
clean:
	cargo clean
