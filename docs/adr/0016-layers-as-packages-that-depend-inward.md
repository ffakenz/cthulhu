# 16. Layers as packages that depend inward

## Status

Superseded by ADR 18

## Motivation

- Builds on *A pure core*, *Effects behind ports*, *Sources, sinks and projections* and *Stores
  defined by their laws*.
- The core must never reach an adapter. In one package, only review keeps a core module from
  importing an adapter or an adapter's crate, and a review can miss one.
- A package can import only the packages its manifest names, and Cargo refuses a cycle, so a
  boundary between packages is one the compiler checks.
- Every stage adds stores and adapters around the same core, and each is one more thing the core
  could import.

## Decision

- The workspace has three packages, and every dependency between them points inward:
  - `engine`, in `libs/engine/`: the domain model, `domain`; the decider, `core`; and the runtime,
    `run`, which declares the store ports `LedgerStore` and `SeenIds`. It depends on no other
    package of the workspace and on none of the crates behind a format, such as `csv` and
    `serde`.
  - `store`, in `libs/store/`: the stores that implement those ports, for now `memory`, with
    `MemoryLedger` and `MemorySeenIds`. It depends on `engine` and on no other package of the
    workspace.
  - `backend`, in `backend/`: the adapters to formats, `csv_io`, which uses `csv` and `serde`, and
    `text_io`; and the composition root, `main.rs`, the binary `cthulhu`. It depends on `engine`
    and `store`, and no package depends on it.
- The workspace keeps one binary, `cthulhu`, so `cargo run` at its root runs it.
- The black-box test crate stays `backend/tests/it`, and reaches every package's public API
  through `backend`'s dependencies.
- The workspace's packages are members, not admitted crates: the split admits no crate, and
  `rust_decimal` moves to `engine`'s manifest with the same features.
- The `domain` and `core` modules, which *A domain model in the records' language* names at
  `backend/src/domain.rs`, `backend/src/domain/` and `backend/src/core.rs`, move into `engine`.
- Governs: the root `Cargo.toml`'s `members`; `libs/engine`: its manifest, `domain`
  (`libs/engine/src/domain.rs`, `libs/engine/src/domain/`), `core` (`libs/engine/src/core.rs`)
  and `run`; `libs/store`: its manifest and `memory`; `backend`: its manifest, `csv_io`,
  `text_io` and `main.rs`.

## Consequences

- A module of `engine` that imports `memory`, `csv_io`, `text_io`, `csv` or `serde`, or a module
  of `store` that imports `csv_io` or `text_io`, does not build; nor does an adapter that names
  `rust_decimal`, which only `engine`'s manifest names.
- A disk store joins `store` without touching `engine`, and a later runtime can build on `engine`
  without the CSV adapters.
- Four manifests instead of two; each package's names the shared fields and the lint table it
  inherits.
- Privacy holds only within a crate: `core` builds accounts and deposits through the ledger's
  crate-private constructors and reads an account's lock through a crate-private method, so
  `domain` and `core` stay in one package, and the layering between the modules of `engine` stays
  a review matter.
- A manifest is checked by review: nothing but review stops `engine`'s manifest from naming a
  format's crate, and `make deps` checks the admitted crates for the workspace as a whole, not per
  package.
- `engine` and `store` have no tests of their own: their laws run in `backend/tests/it`.
- *A domain model in the records' language* names the old paths of `domain` and `core`; it stays
  as it was accepted, and this record names where they live.
- Deferred: a package per runtime. A runtime that needs a crate `engine` must not depend on, such
  as a network edge, would be a package of its own that depends on `engine`, with that crate
  admitted by a record of its own.
- Rejected: an `adapters/` folder inside one package (a module path is no boundary, so the
  layering stays review-enforced); a finer split, a package each for `domain`, `core` and `run` or
  for each adapter (the ledger's crate-private items would have to become public, so any code
  could build an account no event made; `run`, pure and generic over its ports, gains no boundary
  worth a manifest; and a package per adapter fences off adapters that never import each other,
  from a core that already cannot reach them); renaming `backend` to an app folder (the binary's
  package, its test crate and the paths that accepted records name would all move, for no
  boundary the compiler checks).
