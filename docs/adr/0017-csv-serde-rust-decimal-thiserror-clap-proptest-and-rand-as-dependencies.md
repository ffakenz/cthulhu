# 17. csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies

## Status

Accepted

## Motivation

- Supersedes *csv, serde, rust_decimal and proptest as dependencies*. It admitted no crate for
  error messages, for the command line or for generated benchmark inputs: every error type wrote
  its `Display`, and all but `Failure` an `Error`, by hand, away from its variants, and the
  command line read `--help` and `--version` as paths.
- Builds on *Sources, sinks and projections*: formats, and so the crates behind them, stay in
  adapters.
- Builds on *Effects behind ports* and *Three failure tiers*: only `main.rs` touches the world,
  and each failure tier has its own type.
- Builds on *Correctness defined by laws and a model*: laws over generated inputs, each shown to
  fail once, and a failing law that replays exactly.
- Builds on *Runtimes proven against a sequential reference*: a runtime lands only with a
  benchmark, and the benchmarks need inputs generated from a seed.
- Builds on *Layers as packages that depend inward*: each package's manifest names the crates it
  uses.
- The engine needs a CSV reader and writer and exact decimal arithmetic. The standard library has
  neither, and floating point cannot hold money exactly.
- The standard library has no property testing, no derive for `Display` or `Error`, no argument
  parser and no seedable random generator.
- Every crate is code to audit and keep current, and a crate only the dev targets use must never
  reach the program.

## Decision

- Admitted for the program, as before: `csv`; `serde` with `derive`; `rust_decimal` with
  `default-features = false` and `std`. New: `thiserror` 2.0.21, in `engine` and `backend`; `clap`
  4.6.7 with `default-features = false` and `std`, `help`, `usage`, `error-context` and `derive`,
  in `backend`. With them, the 19 crates they build on: `csv-core`, `memchr`, `itoa`, `ryu`,
  `serde_core`, `serde_derive`, `syn`, `quote`, `proc-macro2`, `unicode-ident`, `arrayvec`,
  `num-traits`, `autocfg`, `thiserror-impl`, `clap_builder`, `clap_derive`, `clap_lex`, `anstyle`,
  `heck`.
- clap's features: `help`, `usage` and `error-context` give `--help`, the usage line and messages
  that say what was wrong; `derive` declares the arguments as a struct; `std` is required. Off:
  `env` (arguments from environment variables), `color` (it reads variables such as `NO_COLOR` and
  `TERM`, and asks whether a stream is a terminal), `wrap_help` (it reads the terminal's width, or
  `COLUMNS` and `LINES`) and `suggestions` (did-you-mean hints, through `strsim`). Parsing reads
  only the arguments it is given.
- The minimum versions keep both derives on `syn` 3, which the program already builds: before
  2.0.19 and 4.6.4, `thiserror` and `clap` built their derives on `syn` 2.
- Admitted for the dev targets only: `proptest` 1.12 with `default-features = false` and `std`, as
  before; `rand` 0.10 with `default-features = false`, now a dev-dependency of its own, for the
  seeded benchmark generator under `backend/examples/`. Both are dev-dependencies of `backend`.
  proptest already builds `rand`, so naming it adds no crate, and no target uses it directly until
  the generator lands. With them, the 12 crates they build on that the program's set lacks:
  `bitflags`, `cfg-if`, `chacha20`, `core_detect`, `cpufeatures`, `getrandom`, `libc`, `r-efi`,
  `rand_core`, `rand_xorshift`, `regex-syntax`, `unarray`. They also build on `num-traits` and
  `autocfg`. Nothing outside these two sets is built.
- The sets are crate names, as `make deps` compares them. Being listed in `Cargo.lock` admits
  nothing: the lock also lists crates behind features nothing enables, and they are never built.
- `make deps` checks the build graph (`cargo tree`, every dependency kind, feature and target)
  against both sets, and the graph without dev-dependencies against the program's set alone, so a
  dev-only crate that reaches the program fails the gate.
- `csv` and `serde` appear only in the CSV adapters; `rust_decimal` only inside `Amount` and
  `Balance`; `thiserror` only in the derives of the error types, `Overflow`, `AmountError`,
  `RowError`, `SourceError`, `RunError`, `SinkError` and `Failure`, without `#[from]`,
  `#[source]`, `#[error(transparent)]` or a field named `source`, so no error type converts into
  another and `source()` stays `None`; `clap` only in `main.rs`, which parses its own snapshot of
  the arguments and maps every `clap::Error` to an exit status itself; `proptest` only in
  `backend/tests/it`, whose `support::for_all` runs every law; `rand` only in `backend`'s dev
  targets.
- Governs: the dependency tables of every `Cargo.toml`, `make deps`, `csv_io`, `Amount`,
  `Balance`, the error types above, `main.rs`, `backend/tests/it`, `backend/examples/`.

## Consequences

- An error type states each message beside its variant, and a variant without one does not
  compile. The derived `Error` of `RunError<L, S>` asks only that `L` and `S` be `Debug` and
  `Display`, where the impl written by hand asked that they be `Error`.
- The command line answers `--help` and `--version`, a usage error says what was wrong, and a path
  that begins with `-`, other than `-` itself, follows `--`. clap words the messages and lays out
  the help; the binary's fixtures pin both, so an update that changes them fails them.
- As before, a failing law reports a shrunk counterexample, and its seed, saved under
  `backend/tests/proptest-regressions/`, replays first on the next run; a generator whose draws
  depend on earlier ones draws its choices first and then interprets them.
- 38 crates to audit and keep current, 14 of them for the dev targets only. Of the program's 8 new
  crates, `thiserror` runs a build script that probes the compiler, and `thiserror-impl` and
  `clap_derive` are procedural macros; `clap_derive` turns on `syn`'s `full` feature, so clean
  builds are slower. As before, of the dev-only crates, `getrandom` and `libc` run build scripts.
- proptest reads its own `PROPTEST_*` environment variables, and seeds each run from the operating
  system.
- `rand` is a direct dev-dependency that no target uses yet. No lint the workspace enables flags
  an unused dependency, so review keeps it from staying one.
- The gate compares names, so a second version of an admitted crate passes it; the minimum
  versions above keep one `syn`.
- `Cargo.lock` is not the admitted set, so the gate must read the build graph, not the lock file.
- Containment keeps each crate replaceable: the CSV crates by changing one adapter, `rust_decimal`
  by changing `Amount` and `Balance`, `thiserror` by writing the impls it derives, `clap` by
  changing `main.rs`, proptest by changing the test crate, `rand` by changing the generator.
- Rejected: `anyhow` and `eyre` (an opaque report in place of the closed enums, so `main` could no
  longer map every failure to its exit status with a total `match`); `async-trait` (`async fn` in
  traits is native, and the store ports are synchronous); `chrono` (nothing holds a timestamp, and
  nothing reads the clock); an argument parser written here (`--help`, `--version`, `--` and every
  usage message to own and prove); a random generator written here (its quality and its seeding to
  own and prove); clap's default features (`color`, `suggestions` and the crates behind them);
  rand's default features (a generator seeded from the operating system, and `StdRng`, whose
  stream may change between versions, where the benchmark generator seeds a portable one itself).
  Still rejected, as before: the harness written here (no shrinking, ours to maintain);
  `quickcheck` (shrinkers written by hand for draws that depend on earlier ones); proptest's
  default features (forking, timeouts and bit sets, unused here); `proptest-state-machine` (it
  turns those features on); proptest 1.11 (more crates for tests); the standard library only for
  the program (our own CSV reader and fixed-point type to prove); `csv` alone (fields parsed by
  hand).
- Deferred, each to a record of its own when the roadmap's service runtime needs it: `tokio`, and
  `tracing` with `tracing-subscriber`.
