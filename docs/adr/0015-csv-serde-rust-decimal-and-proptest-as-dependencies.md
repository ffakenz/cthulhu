# 15. csv, serde, rust_decimal and proptest as dependencies

## Status

Superseded by ADR 17

## Motivation

- Supersedes *csv, serde and rust_decimal as dependencies*. It admitted no crate for tests, so the
  property tests ran on a harness written here, and owning that harness cost more than admitting
  a crate for it.
- Builds on *Sources, sinks and projections*: formats, and so the crates behind them, stay in
  adapters.
- Builds on *Correctness defined by laws and a model*: laws over generated inputs, each shown to
  fail once, and a failing law that replays exactly.
- The engine needs a CSV reader and writer and exact decimal arithmetic. The standard library has
  neither, and floating point cannot hold money exactly.
- The harness reported a failing input as it was drawn, never a smaller one; its runner and its
  seed handling were code to maintain.
- The standard library has no property testing.
- Every crate is code to audit and keep current, and a crate only the tests use must never reach
  the program.

## Decision

- Admitted for the program, as before: `csv`; `serde` with `derive`; `rust_decimal` with
  `default-features = false` and `std`. With them, the 13 crates they build on: `csv-core`,
  `memchr`, `itoa`, `ryu`, `serde_core`, `serde_derive`, `syn`, `quote`, `proc-macro2`,
  `unicode-ident`, `arrayvec`, `num-traits`, `autocfg`.
- Admitted for tests only: `proptest` 1.12 with `default-features = false` and `std`, a
  dev-dependency of `backend`. With it, the 13 crates it builds on that the program's set lacks:
  `bitflags`, `cfg-if`, `chacha20`, `core_detect`, `cpufeatures`, `getrandom`, `libc`, `r-efi`,
  `rand`, `rand_core`, `rand_xorshift`, `regex-syntax`, `unarray`. It also builds on `num-traits`
  and `autocfg`. Nothing outside these two sets is built.
- The sets are crate names, as `make deps` compares them. Being listed in `Cargo.lock` admits
  nothing: the lock also lists crates behind features nothing enables, and they are never built.
- `make deps` checks the build graph (`cargo tree`, every dependency kind, feature and target)
  against both sets, and the graph without dev-dependencies against the program's set alone, so a
  test-only crate that reaches the program fails the gate.
- `csv` and `serde` appear only in the CSV adapters; `rust_decimal` only inside `Amount` and
  `Balance`; `proptest` only in `backend/tests/it`, whose `support::for_all` runs every law.
- Governs: the dependency tables of `Cargo.toml`, `make deps`, `csv_io`, `Amount`, `Balance`,
  `backend/tests/it`.

## Consequences

- A failing law reports a shrunk counterexample. proptest saves the failing case's seed under
  `backend/tests/proptest-regressions/` and replays it first on the next run, so a failing law
  still prints its seed and replays exactly.
- Generators become strategies. A generator whose draws depend on earlier ones draws its choices
  first and then interprets them, which takes more code than drawing as it goes.
- 30 crates to audit and keep current, 14 of them for tests only. Of those 14, `getrandom` and
  `libc` run build scripts, and none is a procedural macro. Clean test builds are slower.
- proptest reads its own `PROPTEST_*` environment variables, and seeds each run from the
  operating system.
- `Cargo.lock` is not the admitted set, so the gate must read the build graph, not the lock file.
- Containment keeps each crate replaceable: the CSV crates by changing one adapter,
  `rust_decimal` by changing `Amount` and `Balance`, proptest by changing the test crate.
- Rejected: the harness written here (no shrinking, ours to maintain); `quickcheck` (it shrinks
  each type through `Arbitrary`, so generators whose draws depend on earlier ones would need
  shrinkers written by hand); proptest's default features (forking, timeouts and bit sets, about
  twice the crates, unused here); `proptest-state-machine` (it turns on proptest's default
  features, rejected above, and the laws' drivers are already folds); proptest 1.11 (17 crates for
  tests against 14, among them a derive macro and two WebAssembly-only crates). Still rejected, as
  before: the standard library only for the program (our own CSV reader and fixed-point type to
  prove); `csv` alone (fields parsed by hand).
