# Cthulhu

A Rust workspace built on the standard library alone. Its engineering conventions — a functional
core, total functions, errors as values — are enforced by the compiler, the lint gate and review.

## Setup

Requires [rustup](https://rustup.rs). `rust-toolchain.toml` pins the toolchain: Rust 1.88.0 with
rustfmt and clippy. Install it once from the repository root:

```sh
rustup toolchain install
```

There are no other dependencies.

## Usage

```sh
make          # list the targets
make ci       # the commit gate: deps, fmt-check, lint, build, test
make run      # run the cthulhu binary
```

## Layout

```text
.
├── backend/              the crate: a library (the functional core) and the `cthulhu` binary
│   └── src/
│       ├── lib.rs        the functional core
│       └── main.rs       the composition root
├── docs/
│   ├── adr/              architecture decision records and their template
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
- [docs/adr/](docs/adr/README.md) — the decision log. Read it before changing the architecture.
- [docs/prompts/](docs/prompts/) — one prompt history per working session, so the way the project
  was built stays reproducible.
- [tasks/](tasks/README.md) — plans of record and lessons learned.
- [.claude/skills/](.claude/skills/) — the functional-programming doctrine and its Rust spelling.
