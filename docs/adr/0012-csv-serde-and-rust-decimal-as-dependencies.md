# 12. csv, serde and rust_decimal as dependencies

## Status

Accepted

## Motivation

- Builds on *Sources, sinks and projections*: formats, and so the crates behind them, stay in
  adapters.
- The workspace builds on the standard library alone unless a record admits a crate.
- The engine needs a CSV reader and writer and exact decimal arithmetic. The standard library has
  neither, and floating point cannot hold money exactly.
- Every crate is code to audit and keep current.

## Decision

- Admitted: `csv`; `serde` with `derive`; `rust_decimal` with `default-features = false` and
  `std`. With them, the 13 crates they build on: `csv-core`, `memchr`, `itoa`, `ryu`,
  `serde_core`, `serde_derive`, `syn`, `quote`, `proc-macro2`, `unicode-ident`, `arrayvec`,
  `num-traits`, `autocfg`. Nothing else is built.
- `Cargo.lock` also lists crates behind features nothing enables, such as `borsh`, `rand` and
  `serde_json`. They are never built, and they are not admitted.
- When the crates are first used, `make deps` gains a check of the build graph (`cargo tree`,
  every dependency kind) against exactly this set.
- `csv` and `serde` appear only in the CSV adapters; `rust_decimal` only inside `Amount` and
  `Balance`.
- Governs: the dependency tables of `Cargo.toml`, `make deps`, `csv_io`, `Amount`, `Balance`.

## Consequences

- A proven CSV implementation and exact decimals, without owning either.
- 16 crates to audit and keep current; the derive macros slow clean builds.
- `Cargo.lock` is not the admitted set, so the gate must read the build graph, not the lock file.
- Containment keeps each crate replaceable by changing one adapter or one type.
- Rejected: the standard library only (our own CSV reader and fixed-point type to prove); `csv`
  alone (fields parsed by hand).
