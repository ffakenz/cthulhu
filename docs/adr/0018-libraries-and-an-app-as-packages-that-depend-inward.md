# 18. Libraries and an app as packages that depend inward

## Status

Accepted

## Motivation

- Supersedes *Layers as packages that depend inward*. It kept the name `backend` for the package
  that holds the command line: its composition root, its adapters to formats and the black-box
  tests. The name said nothing of what the package holds, and read as if it held the engine,
  which lives in `libs/`.
- That record rejected the rename because the binary's package, its test crate and paths that
  accepted records name would all move, for no boundary the compiler checks. It still adds no
  boundary; what it buys is a name that says what the package holds. The move costs one commit
  before the packages land, and this record names where every moved path lives.
- Builds on *A pure core*, *Effects behind ports*, *Sources, sinks and projections* and *Stores
  defined by their laws*.
- The core must never reach an adapter. In one package, only review keeps a core module from
  importing an adapter or an adapter's crate, and a review can miss one.
- A package can import only the packages its manifest names, and Cargo refuses a cycle, so a
  boundary between packages is one the compiler checks.
- Every stage adds stores and adapters around the same core, and each is one more thing the core
  could import.

## Decision

- The workspace has three packages, two libraries and an app, and every dependency between them
  points inward:
  - `engine`, in `libs/engine/`: the domain model, `domain`; the decider, `core`; and the runtime,
    `run`, which declares the store ports `LedgerStore` and `SeenIds`. It depends on no other
    package of the workspace and on none of the crates behind a format, such as `csv` and
    `serde`.
  - `store`, in `libs/store/`: the stores that implement those ports, for now `memory`, with
    `MemoryLedger` and `MemorySeenIds`. It depends on `engine` and on no other package of the
    workspace.
  - `app`, in `app/`: the adapters to formats, `csv_io`, which uses `csv` and `serde`, and
    `text_io`; and the composition root, `main.rs`, the binary `cthulhu`. It depends on `engine`
    and `store`, and no package depends on it.
- The workspace keeps one binary, `cthulhu`, so `cargo run` at its root runs it.
- The black-box test crate is `app/tests/it`, and reaches every package's public API through
  `app`'s dependencies.
- The workspace's packages are members, not admitted crates: this record admits no crate.
- Accepted records name some of these paths and packages as they were:
  - *A domain model in the records' language* names `backend/src/domain.rs`,
    `backend/src/domain/` and `backend/src/core.rs`: the `domain` and `core` modules, now in
    `libs/engine/src/`.
  - *Correctness defined by laws and a model* names `backend/tests/it`: now `app/tests/it`.
  - *csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies* names the
    package `backend`, its `backend/tests/it`, its `backend/tests/proptest-regressions/` and its
    `backend/examples/`: now the package `app`, `app/tests/it`, `app/tests/proptest-regressions/`
    and `app/examples/`.
- Governs: the root `Cargo.toml`'s `members`; `libs/engine`: its manifest, `domain`
  (`libs/engine/src/domain.rs`, `libs/engine/src/domain/`), `core` (`libs/engine/src/core.rs`)
  and `run`; `libs/store`: its manifest and `memory`; `app`: its manifest, `csv_io`, `text_io`,
  `main.rs` and `tests/it`.

## Consequences

- A module of `engine` that imports `memory`, `csv_io`, `text_io`, `csv` or `serde`, or a module
  of `store` that imports `csv_io` or `text_io`, does not build; nor does an adapter that names
  `rust_decimal`, which only `engine`'s manifest names.
- A disk store joins `store` without touching `engine`, and a later runtime can build on `engine`
  without the CSV adapters.
- Each package's name says what it holds. Earlier records, plans and prompt histories call the
  binary's package `backend`; this record is where the two names meet.
- Four manifests instead of two; each package's names the shared fields and the lint table it
  inherits.
- Privacy holds only within a crate: `core` builds accounts and deposits through the ledger's
  crate-private constructors and reads an account's lock through a crate-private method, so
  `domain` and `core` stay in one package, and the layering between the modules of `engine` stays
  a review matter.
- A manifest is checked by review: nothing but review stops `engine`'s manifest from naming a
  format's crate, and `make deps` checks the admitted crates for the workspace as a whole, not per
  package.
- `engine` and `store` have no tests of their own: their laws run in `app/tests/it`.
- Deferred: a package per runtime. A runtime that needs a crate `engine` must not depend on, such
  as a network edge, would be an app of its own beside `app`, depending on `engine`, with that
  crate admitted by a record of its own; the two apps would then take the names of their runtimes.
- Rejected: `backend/` for the libraries (the word fits the adapters as well as the stores, and
  every record, plan and prompt history that uses it gives it to the package that holds the
  binary); `app/cli/`
  now (a folder for more apps before there is a second one); an `adapters/` folder inside one
  package (a module path is no boundary, so the layering stays review-enforced); a finer split, a
  package each for `domain`, `core` and `run` or for each adapter (the ledger's crate-private
  items would have to become public, so any code could build an account no event made; `run`,
  pure and generic over its ports, gains no boundary worth a manifest; and a package per adapter
  fences off adapters that never import each other, from a core that already cannot reach them).
