# Cthulhu

Cthulhu is an event-sourced ledger engine: a pure decider folds client and dispute commands into
per-client accounts, and a command-line runtime streams the commands from a CSV file and prints
the final accounts. Its engineering conventions — a functional core, total functions, errors as
values — are enforced by the compiler, the lint gate and review.

## Status

The library is built: the domain model, the core, the sequential runtime and the in-memory stores,
each held to its laws. The command-line runtime is next.

## Setup

Requires [rustup](https://rustup.rs). `rust-toolchain.toml` pins the toolchain: Rust 1.88.0 with
rustfmt and clippy. Install it once from the repository root:

```sh
rustup toolchain install
```

Admitted crates: `csv`, `serde` and `rust_decimal`, with the 13 crates they build on.
`rust_decimal` is in use; `csv` and `serde` arrive with the CSV adapters.

## Usage

```sh
make          # list the targets
make ci       # the commit gate: deps, fmt-check, lint, build, test
make run      # run the cthulhu binary
```

Once built, the engine runs as:

```sh
cargo run -- input.csv > output.csv
```

The design record *The command-line contract* fixes the argument, the streams and the exit
status; [docs/overview.md](docs/overview.md) gives the formats and rules.

## Design

- [docs/overview.md](docs/overview.md) — what the engine does, and the decisions behind it.
- [docs/architecture.md](docs/architecture.md) — a picture of the architecture decisions.
- [docs/patterns.md](docs/patterns.md) — the patterns the architecture is built from, the decider
  first.
- [docs/design/](docs/design/README.md) — the design records: how the engine treats its input.
- [docs/roadmap.md](docs/roadmap.md) — the benchmark-gated stages the engine grows through.

## Layout

```text
.
├── backend/              the crate: the library and the `cthulhu` binary
│   ├── src/
│   │   ├── lib.rs        the library: core, runtime and adapters
│   │   ├── core.rs       the decider: `open`, `decide_client`, `decide_dispute`, `evolve`
│   │   ├── domain.rs     the domain model: re-exports every type of `domain/`
│   │   ├── domain/       ids, money, accounts and deposits, commands and events
│   │   ├── memory.rs     the in-memory stores: `MemoryLedger`, `MemorySeenIds`
│   │   ├── run.rs        the runtime: `run`; re-exports every type of `run/`
│   │   ├── run/          the store ports, rows and run errors, the summary views
│   │   └── main.rs       the composition root
│   └── tests/it/         the black-box test crate: laws, generators and the model
├── docs/
│   ├── overview.md       what the engine does
│   ├── architecture.md   a picture of the architecture decisions
│   ├── patterns.md       the patterns the architecture is built from
│   ├── roadmap.md        the stages ahead
│   ├── adr/              architecture decision records and their template
│   ├── design/           design records: how the engine treats its input
│   └── prompts/          the prompt history of each working session
├── tasks/                plans of record and lessons learned
├── .claude/              Claude Code skills, agents and project settings
├── .github/workflows/    CI: `make ci` on every push and pull request to `main`
├── CLAUDE.md             how work is done here
├── Cargo.toml            the workspace manifest and its lint rules
├── clippy.toml           lint configuration, including the disallowed methods
├── rustfmt.toml          formatting
├── rust-toolchain.toml   the pinned toolchain
└── Makefile              the targets above
```

## Conventions

- [CLAUDE.md](CLAUDE.md) — how work is done here: principles, dependencies, git, reviews, tests
  and comments.
- [docs/adr/](docs/adr/README.md) — the architecture decision log. Read it, and the design records
  in [docs/design/](docs/design/README.md), before changing the engine.
- [docs/prompts/](docs/prompts/) — one prompt history per working session, so the way the project
  was built stays reproducible.
- [tasks/](tasks/README.md) — plans of record and lessons learned.
- [.claude/skills/](.claude/skills/) — the functional-programming, Curry–Howard and
  domain-modelling doctrine, and its Rust spelling.
