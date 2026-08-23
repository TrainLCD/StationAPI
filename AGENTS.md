# StationAPI Repository Guidelines

This guide explains how automation agents and human contributors should work with the StationAPI repository so releases stay predictable, auditable, and safe. Update this file whenever you change the workflow or behavior it documents.

## Project Layout
- `src/` – The Worker itself (`stationapi-worker`, wasm32 only). `lib.rs` holds the endpoints, `index.rs` parses the embedded CSVs into in-memory indexes, `repository.rs` implements the repository traits against those indexes, and `graphql/` holds the async-graphql types and resolvers.
- `schema/public.graphql` – The published GraphQL schema. CI diffs the Worker's SDL against this file, so an unintended change fails the build.
- `build.rs` – Stages `generated/*.csv` (falling back to `data/*.csv`) into `OUT_DIR` and pre-converts `station_station_types` into a fixed-width binary.
- `wrangler.jsonc` – Staging and production deployment settings.
- `stationapi/src/domain/` – Entity definitions and repository abstractions. `repository/` provides `async_trait`-based interfaces, and `normalize.rs` contains text normalization for search.
- `stationapi/src/use_case/` – Application logic. `interactor/query.rs` implements the `QueryUseCase` contract defined in `traits/query.rs`; `dto/` converts entities into `model` types (this is where IPA and TTS segments are built).
- `stationapi/src/model.rs` – The values the API returns. Formerly generated from `.proto`; kept as the layer between entities and GraphQL types.
- `preprocessor/` – Build-time CLI that assembles `generated/*.csv` from `data/*.csv`, the GTFS feeds, and the Tokyu ODPT JSON.
- `data/` – Canonical CSV datasets. Files follow the `N!table.csv` naming scheme. Detailed instructions are in `data/README.md`.
- `data_validator/` – CLI that verifies cross-file constraints (`cargo run -p data_validator`).
- `Makefile` – Convenience targets (`make help` lists them all).

The Worker is the workspace root package. `stationapi`, `preprocessor`, and `data_validator` are native workspace members, so type-checking and linting are split by target (see `make check` / `make clippy`).

## Tooling and Environment
- Rust: Use the stable toolchain (`rustup default stable`) plus the wasm target (`rustup target add wasm32-unknown-unknown`).
- `worker-build` (`cargo install worker-build --locked`) and `wrangler` are needed to build and run the Worker.
- No database. The data is embedded into the WASM binary at build time.
- Environment variables:
  - `ODPT_ACCESS_TOKEN` – ODPT consumer key used to download authenticated data such as Seibu Bus GTFS, Keio Bus GTFS, Tokyu Bus JSON, and the Tokyu-operated Ota, Shinagawa, and Meguro community bus GTFS feeds. Only used by `preprocessor`.
  - `DISABLE_BUS_FEATURE` – set to `true` to build rail-only data.
- Keep local secrets in `.env.local` (git-ignored) and export them before running `make data`.

## Running and Deploying
- **Local development**
  1. `make data` builds `generated/*.csv` from `data/*.csv`, the GTFS feeds, and the Tokyu ODPT JSON. Feeds already extracted under `data/*-GTFS/` are reused; the ODPT JSON is cached for seven days.
  2. `make build` compiles the Worker, `make dev` serves it on `http://127.0.0.1:8787`.
  3. `GET /__ping` answers without touching the data, `GET /__health` reports index sizes, `GET /` serves GraphiQL, and `GET /__schema` returns the SDL.
- **Deploying**
  - **A branch determines the target, and the workflow file encodes it.** `dev` triggers `deploy_staging.yml` (staging, `stationapi-stg`); `master` triggers `deploy_production.yml` (production, `stationapi`). No other branch deploys anywhere. `build_worker.yml` only builds and verifies; it runs on pull requests and on pushes to every branch except `dev` and `master`, which the deploy workflows already cover.
  - **Never pick the environment with an expression.** Unlike `if:`, a job's `environment` applies whenever the job runs, so a computed name puts every branch and pull request into that environment's deployment history and hands them every secret it holds, `CLOUDFLARE_API_TOKEN` included. Each deploy workflow therefore hard-codes one `environment`, and `build_worker.yml` declares none. `workflow_dispatch` has no branch filter, so the deploy jobs also carry an `if:` pinning them to their branch.
  - **The build steps live in `.github/actions/build-worker`,** a composite action all three workflows share, so verification and deployment build identically. A local action needs a checkout first, so each workflow runs `actions/checkout` with `persist-credentials: false` (the default leaves `GITHUB_TOKEN` in `.git/config`, readable by the third-party code `cargo install` and `npx` execute) and then calls the action.
  - **A deploy must not ship truncated data.** `preprocessor` only warns when a feed fails, so the deploy workflows pass `fail-on-missing-bus-feeds: true` and fail on any feed that did not import. Without it, an empty or expired `ODPT_ACCESS_TOKEN` silently produces a dataset holding only Toei Bus and ships it. `build_worker.yml` has no environment and therefore no token, so it warns instead — the schema and bundle-size checks do not depend on bus data.
  - **Required secrets.** `CLOUDFLARE_ACCOUNT_ID` is a repository secret. `CLOUDFLARE_API_TOKEN` and `ODPT_ACCESS_TOKEN` are environment secrets in both `staging` and `production`. wrangler cannot mint the API token — it has no such command, and the `wrangler login` OAuth token lacks the `API Tokens Write` scope that `POST /user/tokens` requires — so create it in the dashboard.
  - **Scope the API token to what a deploy actually needs.** The `Edit Cloudflare Workers` template is convenient but also grants `Workers KV Storage: Edit`, `Workers R2 Storage: Edit`, and `Workers Tail: Read`, none of which this Worker uses — `wrangler.jsonc` declares no bindings at all. Build a custom token holding only:
    - Account — `Workers Scripts: Edit`, `Account Settings: Read`
    - Zone (`trainlcd.app`) — `Workers Routes: Edit`, which registers the custom domains
    - User — `User Details: Read`, `User Memberships: Read`

    Do not trim below that. Wrangler resolves the account through the user endpoints, and dropping them surfaces as an opaque `code 10000` authentication error rather than a permission message. Setting `CLOUDFLARE_ACCOUNT_ID` reduces how often wrangler needs the membership lookup but does not remove it.
  - **Local deploys** use `make deploy` (staging) and `make deploy-production` (production). Both refuse to run outside their branch. wrangler is invoked through `npx` pinned to `WRANGLER_VERSION`, which appears in the `Makefile`, in both deploy workflows, and as the composite action's default; keep the four in step. wrangler 4 warns when `--env` is omitted with multiple environments defined, so the staging target passes `--env=""` explicitly.
  - `wrangler deploy` always runs the `build.command` in `wrangler.jsonc` (`worker-build --release`); wrangler offers no flag to skip it. Anything that deploys therefore needs the Rust toolchain, the wasm32 target, and `worker-build` on `PATH`.
  - The data lives inside the WASM binary, so **a data change needs a rebuild and a redeploy**. It is not picked up at runtime.
  - A custom domain cannot be registered twice. When moving a domain, remove it from the old Worker and deploy that first.

## Data Management
- CSV load order depends on the numeric prefix (`1!`, `2!`, ...). When adding datasets, choose a prefix that preserves cross-file dependencies.
- Column sets live in `preprocessor/src/rail.rs` (`*_COLUMNS`). Update them alongside any CSV column change; `generated/*.csv` must keep the same column order because `src/index.rs` reads it by name and `build.rs` by position.
- Columns whose name starts with `#` are notes and are not loaded.
- **Through-service junction stations** – When a train type runs through a station where its lines connect, add a `5!station_station_types.csv` row for every line-specific `station_cd` at that station, even when those rows share one `station_g_cd`. The only exception is when the train type explicitly identifies a direction or line-specific operation that excludes one side. Omitting either ID makes the train type selectable from only one line in the app. For example, Hida at Gifu must include both the Takayama Main Line station (`1141601`) and the Tokaido Main Line station (`1150239`). Audit both sides whenever adding or editing a through-service pattern.
- `data_validator` currently verifies that `5!station_station_types.csv` references valid station and type IDs, and that order-sensitive station sequences in `3!stations.csv` stay intact under `ORDER BY e_sort, station_cd` (e.g. the Toei Oedo Line's Tochomae rows, whose misordering silently drops the station from ETA estimation). Extend the validator when new cross-references or order-sensitive spots are introduced and keep the process fail-fast (panic on invalid data).

## Testing and Quality
- **Tests** – `make test` runs the unit tests for every native crate. They need no external services.
- **Type checks** – `make check` covers the native crates and the wasm32 target separately. The Worker also compiles for the host, but only runs on Workers.
- **Linting and formatting** – `make fmt` and `make clippy` before committing (clippy covers the wasm32 target too). Resolve new Clippy warnings unless an existing `#![allow]` covers the case.
- **Schema** – Changing a GraphQL type changes the SDL. Update `schema/public.graphql` in the same change; CI compares it against the running Worker's `/__schema` and fails on any difference. That diff is exactly the client-visible impact.
- **Data verification** – Execute `cargo run -p data_validator` whenever CSVs change and record results in pull requests.
- **IPA coverage audit** – Execute `make ipa-audit` when English or romanized CSV names change. This is a read-only report for `data/2!lines.csv`, `data/3!stations.csv`, and `data/4!types.csv`; it does not fail validation, but highlights unresolved tokens and example names so the IPA dictionary can be extended deliberately.

## GraphQL Query Overview
- **Stations** – `station`, `stations`, `stationGroupStations`, `stationsNearby`, `lineStations`, `stationsByName`, `lineGroupStations`, `lineListStations`, `lineGroupListStations`. `QueryInteractor` enriches stations with lines, companies, station numbers, and train types. `lineStations` resolves the line's local train-type group (rail `kind` 0/1 or a `priority > 0` type); when no such group exists — bus lines only carry `BusRoute` (`kind` 7, `priority` 0) variants — it falls back to the line's plain typeless station list so bus stop listings never return empty.
- **Lines** – `line`, `lines`, `linesByName`. Results include company data and computed line symbols based on repository helpers.
- **Routes** – `routes`, `connectedRoutes`, `estimateArrivalTimes`, `trainRoute`. Paging tokens are currently empty (pagination not implemented).
- **Train types** – `stationTrainTypes`, `routeTypes`. Train types aggregate by line group and include related lines plus optional train type metadata. Rail variants use `TrainTypeKind::{Default, Branch, Rapid, Express, LimitedExpress, HighSpeedRapid, CommuterRapid}` (0-6); bus variants use `BusRoute` (7), which represents a `(route_id, shape_id)` operation pattern (e.g. 循環 / 短ターン / 支線) generated automatically from the configured GTFS bus feeds (Toei Bus, Seibu Bus, Keio Bus) and the converted Tokyu Bus JSON.
- **Default rail train types** – `preprocessor` fills every active rail line containing at least one station with no `station_station_types` row with a deterministic, complete all-stop group. The generated rows exist only in `generated/*.csv`; canonical CSV files remain unchanged. `type_cd=100` represents 「普通」 and `type_cd=101` represents 「各駅停車」. An existing 100/101 assignment on the line takes precedence; otherwise the label is selected per line through `LOCAL_SERVICE_RAIL_LINE_IDS` in `preprocessor/src/rail.rs`. Generated `line_group_cd` values use `1,000,000,000 + line_cd`; generation fails on a collision. Bus lines are excluded and continue to use their GTFS-derived `BusRoute` groups.
- **GTFS bus integration** – `preprocessor/src/gtfs/` reads the GTFS feeds into an in-memory representation and then projects them onto the shared `stations` / `lines` / `types` / `station_station_types` tables (`gtfs/integrate.rs`). Only routes, stops, trips, and stop_times are read; calendar, shapes, feed_info, and agencies do not affect the output. Every configured GTFS feed is imported, including Seibu Bus and Keio Bus (both downloaded from ODPT with `ODPT_ACCESS_TOKEN`). Tokyu Bus ordinary-route `BusroutePattern`, `BusstopPole`, and `BusTimetable` JSON are converted into the same representation; pattern IDs become `shape_id` values so route variants remain queryable as bus TrainTypes. The Tokyu-operated Ota, Shinagawa, and Meguro community buses use their official GTFS feeds and matching JSON routes are excluded to prevent duplicates. `ODPT_ACCESS_TOKEN` is required for authenticated sources; without it those feeds are skipped with a warning rather than failing the build. Stops whose Tokyu JSON records omit coordinates remain available to name and route queries but not coordinate searches. `transport_type` (0: rail, 1: bus) on both `stations` and `lines` keeps rail and bus records queryable side by side. GTFS IDs are namespaced per feed before import to avoid cross-operator collisions. `line_cd` (100,000,000+), `station_cd` / `station_g_cd` (200,000,000+), and bus `type_cd` / `line_group_cd` (100,000,000+) are all deterministic fnv1a hashes that stay clear of the rail data ranges. Disable the entire bus pipeline with `DISABLE_BUS_FEATURE=true`.
- **Bus stop translations (readings & English)** – GTFS-JP `translations.txt` layouts differ per feed, so `load_gtfs_translations` resolves columns by header name (Seibu ships 6 columns without `record_sub_id`; Keio and the Tokyu community feeds ship 7) and indexes each `stop_name` translation under both keys it may use: `record_id` (== the stop_id, Seibu — with the "-NN" pole suffix also mapped to the parent stop_id) and `field_value` (== the Japanese stop_name, Keio / Tokyu community, where `record_id` is left empty). `import_gtfs_stops` then looks a stop's translation up by stop_id first, then by name. Keying only by `record_id` (the previous behavior) silently dropped every field_value-keyed feed, leaving `station_name_k` filled with the kanji stop_name and `station_name_r` empty. Readings arriving as half-width katakana (`ﾆｼﾊﾁｵｳｼﾞ`, Keio / Tokyu community) are folded to full-width via `romaji::to_fullwidth_katakana()` before storage.
- **Bus English-name fallback** – When a feed provides no English (`en`) translation for a stop — e.g. Tokyu Bus ordinary-route JSON, which carries only `dc:title` and `odpt:kana` — `src/domain/romaji.rs::romaji_display_name()` derives a modified-Hepburn romanization (with macrons for long vowels, matching the curated rail style: Tōkyō / Kyōto / Shin-Ōsaka) from the kana reading, and the GTFS reader fills `stop_name_r` with it. The fallback never overwrites a real `en` value, and a reading with no convertible kana stays `NULL` rather than emitting a partial transcription. Because `stop_name_r` is the single upstream source that fans out into the `stations` projection, `search_by_name`, and the romanized bus route/headsign names, this supplements every English-facing surface at once. When projecting into `stations`, `station_name_rn` is filled with the plain-ASCII spelling via `romaji::strip_macrons()` (Tōkyō → Tokyo), mirroring the rail dataset's `_r` (macron) / `_rn` (macron-free) column pair.
- **TTS metadata** – `Station`, `StationNested`, `Line`, `LineNested`, `TrainType`, and `TrainTypeNested` expose `name_ipa` / `name_roman_ipa` plus `name_tts_segments` for multi-segment pronunciation output. Use `name_tts_segments` when clients need per-token SSML construction for mixed-language names such as `Kasai-Rinkai Park`.
- **Connected routes** – `GetConnectedRoutes` performs a bounded breadth-first search across train-type line groups. Transfers join at a shared station group, route order and per-stop pass metadata are preserved, and each returned candidate receives a deterministic virtual line-group ID in the upper half of the `uint32` range. Revisiting station groups and already-used train types is rejected to prevent cycles. Exploration loads only line-group ID, station-station-type ID, station-group ID, and pass metadata; full station rows are fetched after the result set is fixed. The search is additionally capped at eight train types, 4,096 expanded states, 65,536 evaluated candidates, and 32 results to bound computation and result size.
- Changes to the published contract require coordinated updates to `schema/public.graphql`, the async-graphql types in `src/graphql/`, and, when the shape of a value changes, `stationapi/src/model.rs` and the DTO conversions.

## Version Control (Jujutsu)
This repository is a **colocated jj/git checkout** — `.jj/` and `.git/` sit side by side. Agents run every version-control operation through `jj`. Do not run a `git` command that writes (`commit`, `switch`, `branch`, `push`, `rebase`, `stash`): jj re-imports the Git refs on its next invocation, so a Git-side change is either abandoned or resurfaces as a divergent change. `gh` remains the tool for pull requests, and GitHub Actions keeps consuming the Git side unchanged.

- **`trunk()` resolves to `dev@origin`,** aliased in `.jj/repo/config.toml`. Prefer it to a hard-coded branch name.
- **There is no staging area and no untracked file.** The working copy is itself a commit, and jj snapshots every file under the root on each command (`snapshot.auto-track = "all()"`), so a scratch file lands in the change unless `.gitignore` covers it. Nothing corresponds to `git add`, so read `jj status` before describing a change and remove what does not belong — `jj restore <path>` to drop it, `jj split` to move it into a commit of its own.
- **Bookmarks are jj's branches, and they do not follow new commits.** After committing, move the bookmark yourself (`jj bookmark set <name> -r @-`); forgetting it makes the next push a no-op.
- **Never rewrite a pushed commit without asking.** `jj describe`, `jj squash`, and `jj rebase` rewrite history in place, and the next `jj git push` moves the remote bookmark with force-with-lease semantics. Confirm with the user first, exactly as for a Git force push.
- **`jj undo` reverses the last operation** and `jj op log` lists them; prefer both to reconstructing state by hand.

A typical change:
```bash
jj git fetch                        # refresh dev@origin and the other remote bookmarks
jj new 'trunk()'                    # start a new change on top of dev@origin
# ... edit files; jj snapshots them automatically ...
jj status                           # confirm exactly what the change contains
jj commit -m "日本語の単文"          # describe @ and open a fresh empty working copy on top
jj bookmark create feature/<description> -r @-
jj git push -b feature/<description>
```

Equivalents for the operations this guide and `.claude/skills/create-pr` rely on:

| Purpose | Command |
| --- | --- |
| Repository root | `jj root` |
| Working-copy state | `jj status` |
| History | `jj log` (`jj log -r 'trunk()..@'` for the current change set) |
| Bookmark closest to `@` | `jj log -r 'heads(::@ & bookmarks())' --no-graph -T 'local_bookmarks.map(\|b\| b.name()).join("\n")'` |
| Commit subjects on a bookmark | `jj log -r 'dev@origin..<bookmark>@origin' --no-graph -T 'description.first_line() ++ "\n"'` |
| Files changed against a base | `jj diff --name-only --from 'dev@origin' --to '<bookmark>@origin'` |
| Rebase onto the latest `dev` | `jj git fetch && jj rebase -d 'trunk()'` |

`CONTRIBUTING.md` still documents the Git workflow, because outside contributors are not required to install jj. Keep the two aligned in intent — base branch, naming convention, and pull-request rules are identical; only the commands differ.

## Contribution Guidelines
- **Git-flow** – Follow Git-flow with `dev` serving as this repository's `develop` branch. Create ordinary work bookmarks from the latest `trunk()` (`dev@origin`) with `jj new 'trunk()'`, use the `feature/<description>` naming convention, and target their pull requests to `dev`. Do not create or target a bookmark named `develop`. **Version Control (Jujutsu)** above has the full command sequence.
- **Pull requests** – Assign every pull request to `@TinyKitten` when creating it, open it as ready for review rather than as a draft, and use `.github/pull_request_template.md` without omitting or replacing its sections or checklists.
- **Prioritize quality and performance over implementation speed** – Always favor code quality and runtime performance over velocity. Be mindful of algorithmic complexity and look for opportunities to replace O(n×m) linear scans with O(n+m) indexed lookups (e.g., HashMaps). The indexes are rebuilt on every isolate start and every request scans them, so prefer indexed lookups over repeated full scans. When a change affects performance, document the before/after complexity and query plan impact in the pull request.
- Document the commands you executed (for example, ``make fmt && make clippy && make test``) and their outcomes in every pull request.
- For data pipeline or schema updates, add architectural notes under `docs/` and synchronize README references so onboarding materials stay accurate.
- When modifying `QueryInteractor`, ensure the enrichment steps (companies, train types, line symbols) still behave as expected. Double-check helper methods such as `update_station_vec_with_attributes` and `build_route_tree_map`.
- Introducing new tables, endpoints, or feature flags must come with matching updates to this document and any other affected guidance.

## Maintenance
Keep this guide aligned with the repository. If a workflow, environment requirement, or endpoint changes, update AGENTS.md in the same pull request so automation agents and contributors work from current instructions.
