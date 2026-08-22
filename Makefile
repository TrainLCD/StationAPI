# StationAPI Makefile
# よく使うタスクの定義

.PHONY: help test check fmt clippy data build dev deploy deploy-production schema ipa-audit clean

# CI (.github/workflows/build_worker.yml) と同じ版を使う。グローバルへ入れて
# いなくても npx が取ってくるので、版ずれでビルド結果が変わらない。
WRANGLER_VERSION := 4.125.0
WRANGLER := npx --yes wrangler@$(WRANGLER_VERSION)

help:
	@echo "Available targets:"
	@echo "  test             - Run all tests"
	@echo "  check            - Type-check every crate (worker targets wasm32)"
	@echo "  fmt              - Check formatting"
	@echo "  clippy           - Lint every crate"
	@echo "  data             - Rebuild generated/*.csv from data/ and the GTFS feeds"
	@echo "  build            - Build the Worker (wasm)"
	@echo "  dev              - Run the Worker locally (wrangler dev)"
	@echo "  deploy           - Deploy to staging (dev branch only)"
	@echo "  deploy-production- Deploy to production (master branch only)"
	@echo "  schema           - Diff the running Worker's SDL against schema/public.graphql"
	@echo "  ipa-audit        - Print IPA coverage report for English/romanized CSV names"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "Environment variables:"
	@echo "  ODPT_ACCESS_TOKEN   - Required by all bus feeds except Toei"
	@echo "  DISABLE_BUS_FEATURE - Set to true to build rail-only data"

# worker は wasm32 用の crate なので、ネイティブのテストからは外す。
test:
	cargo test -p stationapi -p stationapi-preprocessor -p data_validator

check:
	cargo check -p stationapi -p stationapi-preprocessor -p data_validator
	cargo check --target wasm32-unknown-unknown -p stationapi-worker

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy -p stationapi -p stationapi-preprocessor -p data_validator --all-targets -- -D warnings
	cargo clippy --target wasm32-unknown-unknown -p stationapi-worker --all-targets -- -D warnings

# Worker が読むデータを作り直す。data/*.csv や GTFS が変わったら実行する。
data:
	cargo run --profile tool -p stationapi-preprocessor

build:
	worker-build --release

dev:
	$(WRANGLER) dev

# デプロイ先はブランチで決まる (dev -> staging, master -> production)。取り違えると
# 別環境を上書きするため、対応しないブランチからは実行させない。
# wrangler 4 は環境が複数あると --env の省略を警告するため、staging も明示する。
deploy:
	@branch=$$(git rev-parse --abbrev-ref HEAD); \
	  [ "$$branch" = "dev" ] || { echo "error: staging へのデプロイは dev ブランチからのみ実行できます (現在: $$branch)" >&2; exit 1; }
	$(WRANGLER) deploy --env=""

deploy-production:
	@branch=$$(git rev-parse --abbrev-ref HEAD); \
	  [ "$$branch" = "master" ] || { echo "error: production へのデプロイは master ブランチからのみ実行できます (現在: $$branch)" >&2; exit 1; }
	$(WRANGLER) deploy --env production

# 公開スキーマから外れるとクライアントが壊れる。ローカルの Worker から
# SDL を取って突き合わせる (先に `make dev` を起動しておくこと)。
schema:
	curl -sf http://127.0.0.1:8787/__schema -o /tmp/stationapi-schema.graphql
	python3 scripts/compare_schema.py schema/public.graphql /tmp/stationapi-schema.graphql

ipa-audit:
	@echo "Printing IPA coverage report..."
	rustc tools/ipa_audit.rs -o /tmp/stationapi-ipa-audit
	/tmp/stationapi-ipa-audit

clean:
	cargo clean
	rm -rf build .wrangler
