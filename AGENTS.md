# StationAPI Repository Guidelines

This guide explains how automation agents and human contributors should work with the StationAPI repository so releases stay predictable, auditable, and safe. Update this file whenever you change the workflow or behavior it documents.

## Project Layout
- `stationapi/src/domain/` – Entity definitions and repository abstractions. The `entity/` module mirrors the gRPC schema, `repository/` provides `async_trait`-based interfaces, and `normalize.rs` contains text normalization for search.
- `stationapi/src/use_case/` – Application logic. `interactor/query.rs` implements the `QueryUseCase` contract defined in `traits/query.rs`.
- `stationapi/src/infrastructure/` – SQLx repositories for PostgreSQL. `My*Repository` types share an `Arc<PgPool)` and integration tests live behind `#[cfg_attr(not(feature = "integration-tests"), ignore)]`.
- `stationapi/src/presentation/` – gRPC presentation layer. `presentation/controller/grpc.rs` wires `MyApi` to the generated server types. Health checks and reflection are enabled through `tonic-health` and `tonic-reflection`.
- `stationapi/proto/stationapi.proto` – The gRPC contract. `build.rs` uses `tonic-build` to generate server code and `FILE_DESCRIPTOR_SET`.
- `data/` – Canonical CSV datasets and schema definition (`create_table.sql`). Files follow the `N!table.csv` naming scheme to control import order. Detailed instructions are in `data/README.md`.
- `data_validator/` – CLI that verifies cross-file constraints (`cargo run -p data_validator`).
- `docker/` & `compose.yml` – Container definitions for the API and PostgreSQL 18. `docker/postgres/00_extensions.sql` ensures `pg_trgm` and `btree_gist` are installed.
- `Makefile` – Convenience targets for running tests (`make test-unit`, `make test-integration`, `make test-all`).

## Tooling and Environment
- Rust: Use the stable toolchain (`rustup default stable`). Docker images build with `rust:1`.
- Protobuf: `protoc` must exist when building locally (the Docker image installs it automatically).
- PostgreSQL: Version 15+ with `pg_trgm` and `btree_gist` extensions. Compose spins up PostgreSQL 18 with these extensions preloaded.
- Environment variables:
  - `DATABASE_URL` – SQLx connection string (e.g., `postgres://stationapi:stationapi@localhost/stationapi`).
  - `DISABLE_GRPC_WEB` – `false` enables gRPC-Web; set to `true` for pure gRPC/HTTP2.
  - `ODPT_ACCESS_TOKEN` – ODPT consumer key used to download authenticated GTFS feeds such as Seibu Bus when experimental bus feeds are enabled.
  - `ENABLE_EXPERIMENTAL_BUS_FEATURE` – `true` imports every configured GTFS feed; unset or `false` imports only the stable Toei Bus feed.
  - `HOST` and `PORT` – listen address (defaults to `[::1]:50051`; Docker uses `0.0.0.0:50051`).
  - `.env.test` exports `TEST_DATABASE_URL`, `RUST_LOG`, `RUST_BACKTRACE`, and `RUST_TEST_THREADS=1` for integration tests.
- Recommended: keep shared defaults in `.env`, copy overrides to `.env.local`, and rely on startup loading both files.

## Running and Deploying
- **Local development**
  1. Prepare PostgreSQL and set `DATABASE_URL`.
  2. `cargo run -p stationapi` rebuilds the schema from `data/create_table.sql`, imports every `data/*.csv`, and then boots the gRPC server. If extension creation fails, grant the needed privileges manually.
  3. Health checks respond to `grpc.health.v1.Health/Check`; gRPC Reflection is available for tooling such as `grpcurl`.
- **Docker / Compose**
  - `docker compose up --build` launches both the API and PostgreSQL containers. Compose passes `.env.local` to the API container, so put local runtime secrets such as `ODPT_ACCESS_TOKEN` there before starting the stack. Source code mounts into `/app`, but hot reload is not provided; rebuild after code changes.
  - Production images rely on `docker/api/Dockerfile`, which runs `cargo build -p stationapi --release` and copies `data/`.
- **gRPC-Web**
  - The server accepts HTTP/1.1 and wraps handlers via `tonic_web::enable`. Disable this behavior by exporting `DISABLE_GRPC_WEB=true` to require HTTP/2 clients only.

## Data Management
- CSV import order depends on the numeric prefix (`1!`, `2!`, ...). When adding datasets, choose a prefix that preserves foreign-key dependencies.
- `data/create_table.sql` drops and recreates tables, indexes, and foreign keys. Update this script alongside any schema or CSV column changes.
- `data_validator` currently verifies that `5!station_station_types.csv` references valid station and type IDs. Extend the validator when new cross-references are introduced and keep the process fail-fast (panic on invalid data).

## Testing and Quality
- **Unit tests** – `cargo test --lib --package stationapi` or `make test-unit`; focus on entities and repository mocks without a database.
- **Integration tests** – `source .env.test && cargo test --lib --package stationapi --features integration-tests` or `make test-integration`. Use a dedicated schema behind `TEST_DATABASE_URL` and clean it up after runs.
- **Full suite** – `make test-all` runs unit then integration tests sequentially. Set `RUST_LOG=debug` to inspect SQL queries during debugging.
- **Linting and formatting** – Run `cargo fmt` and `cargo clippy --all-targets --all-features` before committing. Resolve new Clippy warnings unless an existing `#![allow]` covers the case.
- **Data verification** – Execute `cargo run -p data_validator` whenever CSVs change and record results in pull requests.
- **IPA coverage audit** – Execute `make ipa-audit` when English or romanized CSV names change. This is a read-only report for `data/2!lines.csv`, `data/3!stations.csv`, and `data/4!types.csv`; it does not fail validation, but highlights unresolved tokens and example names so the IPA dictionary can be extended deliberately.

## gRPC Endpoint Overview
- **Stations** – `GetStationById`, `GetStationByIdList`, `GetStationsByGroupId`, `GetStationsByCoordinates`, `GetStationsByLineId`, `GetStationsByName`, `GetStationsByLineGroupId`. `QueryInteractor` enriches stations with lines, companies, station numbers, and train types.
- **Lines** – `GetLineById`, `GetLinesByIdList`, `GetLinesByName`. Results include company data and computed line symbols based on repository helpers.
- **Routes** – `GetRoutes`, `GetRoutesMinimal`. The minimal variant returns `RouteMinimalResponse` with deduplicated `LineMinimal` data; paging tokens are currently empty (pagination not implemented).
- **Train types** – `GetTrainTypesByStationId`, `GetRouteTypes`. Train types aggregate by line group and include related lines plus optional train type metadata. Rail variants use `TrainTypeKind::{Default, Branch, Rapid, Express, LimitedExpress, HighSpeedRapid, CommuterRapid}` (0-6); bus variants use `BusRoute` (7), which represents a `(route_id, shape_id)` operation pattern (e.g. 循環 / 短ターン / 支線) generated automatically from the Toei Bus GTFS feed.
- **GTFS bus integration** – At startup, `src/import.rs::integrate_gtfs_to_stations()` ingests GTFS feeds into `gtfs_*` tables and then projects them onto the shared `stations` / `lines` / `types` / `station_station_types` tables. By default only the stable Toei Bus feed is imported; set `ENABLE_EXPERIMENTAL_BUS_FEATURE=true` to import every configured feed, including Seibu Bus (downloaded from ODPT with `ODPT_ACCESS_TOKEN`). `transport_type` (0: rail, 1: bus) on both `stations` and `lines` keeps the two worlds queryable side by side. GTFS IDs are namespaced per feed before import to avoid cross-operator collisions. `line_cd` (100,000,000+), `station_cd` / `station_g_cd` (200,000,000+), and bus `type_cd` / `line_group_cd` (100,000,000+) are all deterministic fnv1a hashes that stay clear of the rail data ranges. Disable the entire bus pipeline with `DISABLE_BUS_FEATURE=true`.
- **TTS metadata** – `Station`, `StationMinimal`, `Line`, and `TrainType` expose `name_ipa` / `name_roman_ipa` plus `name_tts_segments` for multi-segment pronunciation output. Use `name_tts_segments` when clients need per-token SSML construction for mixed-language names such as `Kasai-Rinkai Park`.
- **Japanese pitch accent** – `domain/ipa.rs` annotates the ja-JP reading (`name_ipa` and the katakana-derived ja-JP `name_tts_segments` pronunciation) with the IPA down-step marker `ˈ` placed before the accent-nucleus mora, so Azure AI Speech reproduces station-name pitch accent (issue #1534). Accent type is estimated with [`jpreprocess`](https://github.com/jpreprocess/jpreprocess) (OpenJTalk frontend, bundled `naist-jdic` dictionary loaded once lazily) from the kanji surface, falling back to the katakana reading when the kanji estimate's mora count disagrees. Only word-internal nuclei are marked: word-final (尾高) and heiban readings stay marker-free because they are indistinguishable in isolated speech, and at most one (primary) nucleus is emitted per name. `en-US` segments are never touched and `name_roman_ipa` stays accent-free. Estimation mistakes are corrected via the `accent_override` table (keyed by katakana reading) in `domain/ipa.rs`; add verified entries there rather than reshaping the estimator.
- **Connected routes** – `GetConnectedRoutes`. `QueryInteractor::get_connected_stations` is not implemented yet and returns an empty vector; update the use-case and infrastructure layers together when adding real logic.
- Changes to the service contract require coordinated updates to `proto/stationapi.proto`, regenerated code via `tonic-build`, and corresponding adjustments in both presentation and use-case layers.

## Contribution Guidelines
- **Prioritize quality and performance over implementation speed** – Always favor code quality and runtime performance over velocity. Be mindful of algorithmic complexity and look for opportunities to replace O(n×m) linear scans with O(n+m) indexed lookups (e.g., HashMaps). Avoid unnecessary JOINs and redundant queries at the SQL level. When a change affects performance, document the before/after complexity and query plan impact in the pull request.
- Document the commands you executed (for example, ``cargo fmt && cargo clippy --all-targets --all-features && make test-unit``) and their outcomes in every pull request.
- For database, gRPC, or schema updates, add architectural notes under `docs/` and synchronize README references so onboarding materials stay accurate.
- When modifying `QueryInteractor`, ensure the enrichment steps (companies, train types, line symbols) still behave as expected. Double-check helper methods such as `update_station_vec_with_attributes` and `build_route_tree_map`.
- Introducing new tables, endpoints, or feature flags must come with matching updates to this document and any other affected guidance.

## Maintenance
Keep this guide aligned with the repository. If a workflow, environment requirement, or endpoint changes, update AGENTS.md in the same pull request so automation agents and contributors work from current instructions.
