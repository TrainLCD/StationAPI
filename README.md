# StationAPI

![Billboard](.github/images/billboard.png)

<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->

[![All Contributors](https://img.shields.io/badge/all_contributors-4-orange.svg?style=flat-square)](#contributors-)

<!-- ALL-CONTRIBUTORS-BADGE:END -->

A GraphQL API that provides nearby Japanese train stations and bus stops, running on Cloudflare Workers.

## Documentation

- For automation agent and contributor workflows, see [AGENTS.md](AGENTS.md).
- For contribution guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).
- For system architecture and design decisions, see [docs/architecture.md](docs/architecture.md).
- For technical debt analysis and architectural concerns, see [docs/technical_debt.md](docs/technical_debt.md).
- For the record of the Cloudflare Workers migration, see [docs/cloudflare-workers-migration.md](docs/cloudflare-workers-migration.md).
- For the published GraphQL schema, see [schema/public.graphql](schema/public.graphql).

## Data Contribution

This project includes a comprehensive dataset of Japanese railway information in the `data/` directory. The data is maintained in CSV format and contributions are primarily targeted at Japanese speakers. For detailed information about data structure and contribution guidelines, please refer to [data/README.md](data/README.md).

## Contributors ✨

Thanks goes to these wonderful people ([emoji key](https://allcontributors.org/docs/en/emoji-key)):

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://sw-saturn.dev"><img src="https://avatars.githubusercontent.com/u/20313668?v=4?s=100" width="100px;" alt="Kanta Demizu"/><br /><sub><b>Kanta Demizu</b></sub></a><br /><a href="#data-Sw-Saturn" title="Data">🔣</a> <a href="#infra-Sw-Saturn" title="Infrastructure (Hosting, Build-Tools, etc)">🚇</a> <a href="https://github.com/TrainLCD/StationAPI/commits?author=Sw-Saturn" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://nrsy.jp"><img src="https://avatars.githubusercontent.com/u/31317056?v=4?s=100" width="100px;" alt="SAIGUSA Tomotada"/><br /><sub><b>SAIGUSA Tomotada</b></sub></a><br /><a href="#ideas-10mocy" title="Ideas, Planning, & Feedback">🤔</a> <a href="#data-10mocy" title="Data">🔣</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/mittan12"><img src="https://avatars.githubusercontent.com/u/147319703?v=4?s=100" width="100px;" alt="mittan12"/><br /><sub><b>mittan12</b></sub></a><br /><a href="#data-mittan12" title="Data">🔣</a></td>
      <td align="center" valign="top" width="14.28%"><a href="http://coderabbit.ai"><img src="https://avatars.githubusercontent.com/u/132028505?v=4?s=100" width="100px;" alt="CodeRabbit"/><br /><sub><b>CodeRabbit</b></sub></a><br /><a href="https://github.com/TrainLCD/StationAPI/pulls?q=is%3Apr+reviewed-by%3Acoderabbitai" title="Reviewed Pull Requests">👀</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind welcome!

## Join our community(Japanese only)

Do you like this project? Join our Discord community!
[https://discord.gg/tsemdME9Nz](https://discord.gg/tsemdME9Nz)

## Development

No database is required. The data is generated into `generated/*.csv` and embedded
into the WASM binary at build time.

```bash
rustup target add wasm32-unknown-unknown
cargo install worker-build --locked

make data     # build generated/*.csv from data/ and the GTFS feeds
make build    # build the Worker (wasm)
make dev      # run it locally on http://127.0.0.1:8787
```

`make help` lists every target.

### Testing

```bash
make test     # unit tests for every native crate
make check    # type-check, including the wasm32 target
make clippy   # lint, including the wasm32 target
make fmt      # formatting check
```

Tests need no external services. `stationapi` (domain / use case) and
`preprocessor` (data pipeline) are covered by unit tests; the published GraphQL
schema is verified in CI by diffing the Worker's SDL against
[`schema/public.graphql`](schema/public.graphql).

### Deploying

```bash
make deploy             # staging   -> stationapi-stg
make deploy-production  # production -> stationapi
```

The data lives inside the WASM binary, so **a data change needs a rebuild and a
redeploy**; it is not picked up at runtime.
