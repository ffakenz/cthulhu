# Cthulhu

Cthulhu is an event-sourced ledger engine: a pure decider folds client and dispute commands into
per-client accounts, and a command-line runtime streams the commands from a CSV file and prints
the final accounts. Its engineering conventions — a functional core, total functions, errors as
values — are enforced by the compiler, the lint gate and review.

## Status

The library and the command line are built: the domain model, the core, the sequential runtime,
the in-memory stores and the CSV adapters, each held to its laws, and the report's text adapter,
held to the binary's fixtures. The `cthulhu` binary turns an input file into the accounts table.
The benchmarks are next.

## Setup

Requires [rustup](https://rustup.rs). `rust-toolchain.toml` pins the toolchain: Rust 1.88.0 with
rustfmt and clippy. Install it once from the repository root:

```sh
rustup toolchain install
```

Admitted crates: `csv`, `serde` and `rust_decimal`, with the 13 crates they build on; for tests
only, `proptest`, with the 13 more it builds on. All of them are in use.

## Usage

From the repository root:

```sh
make                                          # list the targets
make ci                                       # the commit gate: deps, fmt-check, lint, build, test
make run INPUT=transactions.csv > output.csv  # the accounts table for the sample input
```

`make run` passes `INPUT` to the binary as one argument, with no shell expansion. By default, zsh
does not expand a `~` after `=` either, so under zsh write `INPUT=$HOME/…`, not `INPUT=~/…`.

The engine runs as:

```sh
cargo run -- transactions.csv > output.csv
```

`transactions.csv`, at the repository root, is a sample input: four clients, every command, a
chargeback that leaves an account negative and locked, a dispute still open, and five refused
commands, which the report counts on stderr. A fixture in `backend/tests/it/cli.rs` pins its
table and report, so `make ci` fails when the file changes and that fixture does not.

The accounts table goes to stdout, written only once the run succeeds. The counts of rejected
commands and malformed rows go to stderr, one line each. The exit status of `cthulhu`, which
`cargo run` passes on, is `0` on success, `2` for not exactly one argument, `3` when the input
cannot be opened, `4` when it cannot be read or its header lacks `type`, `client` or `tx`, and `5`
when the table or the report cannot be written; `make run` names it in its `Error` line.

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
│   │   ├── csv_io.rs     the CSV adapters
│   │   ├── domain.rs     the domain model: re-exports every type of `domain/`
│   │   ├── domain/       ids, money, accounts and deposits, commands and events
│   │   ├── memory.rs     the in-memory stores: `MemoryLedger`, `MemorySeenIds`
│   │   ├── run.rs        the runtime: `run`; re-exports every type of `run/`
│   │   ├── run/          the store ports, rows and run errors, the summary views
│   │   ├── text_io.rs    the text adapter: the report
│   │   └── main.rs       the composition root
│   └── tests/it/         the black-box test crate: laws, strategies and the model
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
├── transactions.csv      a sample input: `cargo run -- transactions.csv`
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
