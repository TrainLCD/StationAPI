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
- Required environment variables:
  - `DATABASE_URL` – SQLx connection string (e.g., `postgres://stationapi:stationapi@localhost/stationapi`).
  - `DISABLE_GRPC_WEB` – `false` enables gRPC-Web; set to `true` for pure gRPC/HTTP2.
  - `HOST` and `PORT` – listen address (defaults to `[::1]:50051`; Docker uses `0.0.0.0:50051`).
  - `.env.test` exports `TEST_DATABASE_URL`, `RUST_LOG`, `RUST_BACKTRACE`, and `RUST_TEST_THREADS=1` for integration tests.
- Recommended: copy `.env` to `.env.local`, override local values, and rely on `dotenv::from_filename(".env.local")` during startup.

## Running and Deploying
- **Local development**
  1. Prepare PostgreSQL and set `DATABASE_URL`.
  2. `cargo run -p stationapi` rebuilds the schema from `data/create_table.sql`, imports every `data/*.csv`, and then boots the gRPC server. If extension creation fails, grant the needed privileges manually.
  3. Health checks respond to `grpc.health.v1.Health/Check`; gRPC Reflection is available for tooling such as `grpcurl`.
- **Docker / Compose**
  - `docker compose up --build` launches both the API and PostgreSQL containers. Source code mounts into `/app`, but hot reload is not provided; rebuild after code changes.
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

## gRPC Endpoint Overview
- **Stations** – `GetStationById`, `GetStationByIdList`, `GetStationsByGroupId`, `GetStationsByCoordinates`, `GetStationsByLineId`, `GetStationsByName`, `GetStationsByLineGroupId`. `QueryInteractor` enriches stations with lines, companies, station numbers, and train types.
- **Lines** – `GetLineById`, `GetLineByIdList`, `GetLinesByName`. Results include company data and computed line symbols based on repository helpers.
- **Routes** – `GetRoutes`, `GetRoutesMinimal`. The minimal variant returns `RouteMinimalResponse` with deduplicated `LineMinimal` data; paging tokens are currently empty (pagination not implemented).
- **Train types** – `GetTrainTypesByStationId`, `GetRouteTypes`. Train types aggregate by line group and include related lines plus optional train type metadata.
- **Connected routes** – `GetConnectedRoutes`. `QueryInteractor::get_connected_stations` is not implemented yet and returns an empty vector; update the use-case and infrastructure layers together when adding real logic.
- Changes to the service contract require coordinated updates to `proto/stationapi.proto`, regenerated code via `tonic-build`, and corresponding adjustments in both presentation and use-case layers.

## Contribution Guidelines
- Document the commands you executed (for example, ``cargo fmt && cargo clippy --all-targets --all-features && make test-unit``) and their outcomes in every pull request.
- For database, gRPC, or schema updates, add architectural notes under `docs/` and synchronize README references so onboarding materials stay accurate.
- When modifying `QueryInteractor`, ensure the enrichment steps (companies, train types, line symbols) still behave as expected. Double-check helper methods such as `update_station_vec_with_attributes` and `build_route_tree_map`.
- Introducing new tables, endpoints, or feature flags must come with matching updates to this document and any other affected guidance.

## Maintenance
Keep this guide aligned with the repository. If a workflow, environment requirement, or endpoint changes, update AGENTS.md in the same pull request so automation agents and contributors work from current instructions.
