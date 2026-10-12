# Workspace packages and admitted crates — plan

**Goal:** the rule that the core never reaches an adapter, which review keeps today, becomes one
the build keeps: the library splits into three packages whose dependencies point inward. Two
crates are admitted with it, and a third becomes a direct dev-dependency: `thiserror` derives the
error types' messages, `clap` gives the command line `--help` and `--version`, and `rand`, which
the tests already build, waits for the benchmark generator.

**Context:** before the benchmarks, the person asked for an evaluation of nine candidate crates
(`eyre`, `anyhow`, `rand`, `clap`, `tracing`, `tracing-subscriber`, `tokio`, `async-trait`,
`chrono`), of moving the CSV and text adapters into a folder of their own, of an engine, store
and app split like the one in another of their repositories, and of a config module. `thiserror`
was proposed in place of `anyhow` and `eyre`: it removes the hand-written error impls and keeps
the closed enums. Decisions holds the outcome and the person's answers. The work starts from
`main` at `a31b805`.

The person signed off this plan as written.

## Decisions

1. **Three packages whose dependencies point inward**, recorded as *Layers as packages that
   depend inward*.
   - `engine`, at `libs/engine`: `domain`, `core` and `run`, which declares the store ports.
   - `store`, at `libs/store`: `memory`.
   - `backend` stays the app: `main.rs`, `csv_io` and `text_io`, and the one black-box test crate,
     `backend/tests/it`.
   - It answers the folder asked for: `csv_io` and `text_io` become the only library modules of
     the app's package, without moving.
   - Nothing widens: the ledger's `pub(crate)` items are used only by `core`, and the report's
     `pub(super)` builders only by `run`, both inside `engine`.
   - Rejected: an `adapters/` folder in one package, which only review would keep; a finer split;
     renaming `backend`.
   - The accepted record *A domain model in the records' language* names the old paths of `domain`
     and `core`. By the person's decision it stays as it is, and the new record names where they
     live.
2. **`thiserror`** derives `Display` and `Error` for the seven error types, every message byte for
   byte as before, with no `#[from]`, `#[source]`, `transparent` or field named `source`.
   `Failure`, which had only `Display`, gains `Error`, and the derived `Error` of `RunError<L, S>`
   asks `Debug` and `Display` of `L` and `S` where the impl by hand asked `Error`.
3. **`clap`**, with `std`, `help`, `usage`, `error-context` and `derive` only, parses the arguments
   snapshot `main` already takes, through `try_parse_from`. `env`, `color`, `wrap_help` and
   `suggestions` stay off: the first three read the environment or the terminal, and the last
   builds `strsim`. *The command-line contract with help and version* supersedes *The
   command-line contract*.
4. **`rand` 0.10** joins `backend`'s dev-dependencies now, with no default features, for the
   seeded benchmark generator. proptest already builds that version, so it adds no crate, and no
   target uses it until the generator lands: the person's call.
5. **Rejected:** `anyhow` and `eyre`, an opaque report in place of the closed enums and the total
   exit-status `match`; `async-trait`, since `async fn` in traits is native and the store ports
   are synchronous; `chrono`, since nothing holds a timestamp or reads the clock. **Deferred** to
   the service stage, each to a record of its own: `tokio`, and `tracing` with
   `tracing-subscriber`. The dependency record names them all, and supersedes *csv, serde,
   rust_decimal and proptest as dependencies*.
6. **No config module yet.** It lands with its first setting, such as the CSV buffer sizes the
   command line's plan left to the benchmarks, or stage 2's partition count, as `CTHULHU_*`
   environment variables parsed from an environment snapshot (`env::vars_os()`) that `main` takes
   once, so the command line keeps its one input argument.
7. **The closed plan is kept** as `tasks/sample-input-plan.md`. Its last two items were ticked
   first, with their evidence, and the one link that named it as `tasks/todo.md`, in prompt
   history 0006, names the new file.
8. **Prompt history 0007**, with the prompt-history skill, committed alone.

## Tasks

- [x] **0.** Branch `feat/workspace-packages` from `main`; archive and repoint (Decisions, item 7);
      this plan into `tasks/todo.md`.
  - Per-commit gate: `make ci` green, 39 passed and 1 ignored; the name greps and the record scan
    are silent.
  - The code-reviewer found no blocker. Its five should-fixes are taken: this tick and its
    evidence; the red rewrites the existing usage test instead of adding beside it; the six clap
    crates are named; the new manifests inherit the workspace lints; and the config module's
    environment snapshot is one `main` will take, not one it takes now. Its ten nits are taken
    too.
  - Commit `docs(tasks): plan the workspace packages and keep the sample input's plan`.
- [x] **1.** The three records, drafted together and shown to the person. On sign-off each is
      `Accepted` and committed alone with its index line:
  - *Layers as packages that depend inward*, committed as
    `docs(adr): record layers as packages that depend inward`
  - *csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies*, with the
    superseded record's Status line, committed as
    `docs(adr): record csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies`
  - *The command-line contract with help and version*, with the superseded record's Status line,
    committed as `docs(design): record the command-line contract with help and version`
  - The code-reviewer read the three drafts before the sign-off and found two blockers, both
    fixed: `engine` is kept off the crates behind a format, not off every crate an adapter uses,
    since `thiserror` serves both; and only the help, the version and the usage lines promise the
    name `cthulhu`, since clap's message for an empty path names no program. Its five
    should-fixes and fourteen nits are taken.
  - The crate sets were resolved in a throwaway project outside the repository, on the pinned
    toolchain: the program's 24 crates and the dev targets' 14 match the dependency record name
    for name, with one `syn`.
  - The person signed off all three. Per-commit gate: `make ci` green, 39 passed and 1 ignored,
    once for the three, since none touches code; the name greps and the record scan are silent
    for each.
  - Commits `3981bf7`, `663c067` and `09718e2`.
  - Task 4, red, gained two cases after the sign-off, an unknown flag and a renamed copy of the
    binary, because the signed design record decides both.
- [x] **2.** The split, a refactor.
  - `libs/engine` and `libs/store`, each a manifest and a `lib.rs`. Each manifest takes the shared
    `[workspace.package]` fields and `[lints] workspace = true`; `engine` names `rust_decimal`
    with `default-features = false` and `std`, and `store` names `engine`. `git mv` moves
    `core.rs`, `domain.rs`, `domain/`, `run.rs` and `run/` into `libs/engine/src`, and `memory.rs`
    into `libs/store/src`. The root manifest's `members` names both; `backend`'s manifest drops
    `rust_decimal` and names `engine` and `store`.
  - Imports: `crate::` becomes `engine::` in `memory.rs`, `csv_io.rs` and `text_io.rs`; `backend::`
    becomes `engine::` or `store::` in `main.rs` and in the test crate.
  - `cargo update --workspace`: the lock gains `engine` and `store`, and `backend`'s list changes;
    nothing else.
  - Docs, skills and agent: `CLAUDE.md` (layout, the paragraph under it, Tests), `README.md`
    (layout), `docs/architecture.md`, `docs/patterns.md`, `docs/roadmap.md`,
    `.claude/agents/code-reviewer.md`, and the skills whose checks or examples name `backend/src`
    or `backend::`.
  - Evidence that nothing changed: `git diff --cached -M --summary` shows the engine's files
    renamed whole; the rename diff of `memory.rs` shows only `use` lines; each new manifest holds
    `[lints] workspace = true`; and the tests give 39 passed and 1 ignored, as before, with only
    their `use` lines changed.
    - Run: the summary lists the eleven files of `engine` renamed at 100% and `memory.rs` at 92%;
      `git diff --cached -M -- backend/src/memory.rs libs/store/src/memory.rs` shows its two `use`
      lines and nothing else. With one path only, git shows the file as new.
    - `cargo update --workspace` changed the lock by 16 lines added and 1 removed: the `engine`
      and `store` entries, and `backend`'s list; no version moved.
    - `make ci` green: 39 passed and 1 ignored.
  - The layering holds: a probe `use store::memory::MemoryLedger;` added to `engine`'s `run.rs`
    failed `cargo build --workspace --locked` with E0433, "use of unresolved module or unlinked
    crate `store`", and was removed.
  - The greps: no `backend::core`, `domain`, `run` or `memory` path is left; no `crate::` path
    names them outside `engine`; `rust_decimal` is named only in `engine`'s manifest and in
    `money.rs`; `disallowed_methods` only in `main.rs`. `cargo tree --workspace --depth 0` lists
    `backend`, `engine` and `store`.
  - Once, from the root, with the person's consent: `cargo run -- transactions.csv` exits 0, its
    stdout equals the sample's table byte for byte, and stderr holds cargo's two lines, then the
    sample's report. No test runs a bare `cargo run`.
  - The code-reviewer found no blocker. Its should-fix is taken: the reviewer agent's architecture
    check names `engine`'s manifest as free of other workspace packages and formats' crates, not
    of every crate, since `thiserror` joins it next. Its nits are taken too: "of the workspace"
    where a package names no other; "the libraries" for the old single library; the wiring
    skill's sketch no longer places a library that names an adapter's error in `engine`; two
    lines back within 100 columns; this tick; and the renamed-copy test's red.
  - Commit `refactor(workspace): split the library into engine, store and backend packages`.
- [x] **3.** `thiserror`, a refactor.
  - `thiserror = "2.0.21"` in `engine` and `backend`; the derives and their `#[error]` attributes,
    transcribed byte for byte; the hand-written impls, and the imports only they used, removed.
    `ADMITTED` gains `thiserror` and `thiserror-impl`, and `CLAUDE.md` § Dependencies,
    `README.md`, `docs/architecture.md` and the `rust-errors` skill follow.
  - Evidence that the messages are equal: a throwaway test, never committed, prints every public
    error value's `to_string()` before and after, and the two outputs are identical.
    - Run: 20 values, from `Overflow`, `AmountError`, `RowError`, `SourceError`, `RunError` over
      `AmountError`, and `SinkError`, print the same message before and after, each with no
      `source()`. `Failure` is private to `main.rs`: the reviewer compared its expansion with the
      deleted impl, and `cli.rs` pins its messages.
    - `cargo update --workspace` added only `thiserror` and `thiserror-impl` 2.0.21, and one `syn`
      stays in the graph. `make deps` passes, the program's graph holding the 18 admitted crates.
    - `make ci` green: 39 passed and 1 ignored.
  - The test crate's own `Failure`, in `support.rs`, keeps its hand-written `Display`: it is a
    test helper, not one of the program's seven error types.
  - The code-reviewer found no blocker. Its two should-fixes are taken: the `rust-errors` rule
    also bans an `#[error]` on the enum itself, which thiserror copies to every variant that has
    none, and the skill's own example derives its messages. Its nits are taken too: the derive is
    spelled `thiserror::Error`, with no bare `Error` beside `io::Error`; the skill gains keywords
    and a grep; the Uses cells list crates first; and this tick. The module box stays a picture of
    the format and decimal crates.
  - Commit `refactor(errors): derive Display and Error with thiserror`.
- [x] **4, red.** In `backend/tests/it/cli.rs`, `a_wrong_argument_count_is_a_usage_error`, which
      pins today's one-line usage, is rewritten to clap's messages for no argument and for two.
      New tests pin the help and version flags, an empty path and an unknown flag, a path that
      begins with `-` after `--`, a help that cannot be written, and clap's whole usage error from
      a copy of the binary under another name, which must still name `cthulhu`, as the signed
      record decides; the wrong implementation it rejects is clap taking the program's name from
      its path. Each fails against today's binary, and nothing else fails.
  - The fixtures are clap's own output: a throwaway project outside the repository, on clap 4.6.7
    with the same features and attributes, printed the help, the version and each usage error.
  - The unknown flag shares a test with the empty path, so the count test keeps a name that holds.
    The second path is a fixed `extra.csv`, so its message is a constant. The binary runs under
    another name through its `argv[0]`, not through another file: a copy can fail with `ETXTBSY`
    on Linux, and a hard link, made while other tests ran the binary, got those runs killed with
    SIGKILL on macOS, in one or two tests of most runs, while twelve runs without it all passed.
  - The wrong implementations are rejected: in the same throwaway project, a parser without
    `bin_name`, run under `argv[0]` `renamed`, prints `Usage: renamed <input.csv>`, which only the
    renamed-binary test catches; one without `name` prints the package's name for `-V`, which the
    version test catches.
  - Run: `make deps fmt-check lint build` green; `cargo test --workspace --locked --no-fail-fast`
    gives 38 passed, 7 failed and 1 ignored in `it`, and every other target passes, holding no
    tests. The seven are the six new tests and the rewritten one, each failing for its reason at
    its first case: `-h`, `-V`, the empty path and `--help` into a closed stdout exit 3, read as
    paths that do not open; `-- -dash.csv` exits 2, two arguments; no argument and the renamed
    binary print the old one-line usage. The cases behind them, run by hand against today's
    binary: `--help`, `--version` and `--input` exit 3 the same way, and a second path exits 2
    with the old line. Eight runs of `it` fail the same seven tests and pass every other.
  - The code-reviewer found no blocker; it rebuilt the green design independently and every
    fixture matched byte for byte. Its should-fix is taken: the check against the wrong
    implementations above. Its nits are taken too: the run's evidence per first case and by hand,
    the fixed second argument, the commit body, and a renamed binary that no writer holds open,
    first as a hard link, then, after the hard link's SIGKILLs, as `argv[0]`.
  - Commit `test(cli): pin the help, the version and clap's usage errors`.
- [x] **4, green.** `clap` 4.6.7 in `backend`, and `ADMITTED` gains `clap`, `clap_builder`,
      `clap_derive`, `clap_lex`, `anstyle` and `heck`. `main.rs` parses with `try_parse_from` and
      splits `clap::Error` with `use_stderr`. `clippy.toml` disallows the clap calls that read the
      process's arguments, exit it or write to its streams, if clippy accepts their paths.
      `README.md`, `docs/overview.md`, `docs/architecture.md`, `CLAUDE.md` § Dependencies and the
      CLI line of the `functional-architecture` reference follow.
  - `cargo update --workspace` added exactly the six crates: `anstyle` 1.0.14, `clap`,
    `clap_builder` and `clap_derive` 4.6.7, `clap_lex` 1.1.1 and `heck` 0.5.0.
  - Run: `make ci` green, 45 passed and 1 ignored; the green's output matches every fixture of
    the red byte for byte. The first runs failed only by SIGKILL, from the red's hard link, which
    the red now replaces with `argv[0]`. Sixty further runs of `it` passed but one, a pre-existing
    race that the next commit fixes.
  - `clippy.toml`: clippy 1.88 takes `clap::Parser::{parse, try_parse, parse_from, update_from}`,
    `clap::Command::{get_matches, get_matches_mut, try_get_matches, get_matches_from, print_help,
    print_long_help}` and `clap::error::Error::{exit, print}`, and says nothing in the packages
    without clap; `clap::Error::{exit, print}`, an alias's path, does not resolve. A probe, never
    committed, calling all twelve from `main.rs` drew all twelve `disallowed_methods` warnings.
  - `make run` passes `--` before `INPUT`, so a path that begins with `-` still runs through it;
    in `sh`, an empty `INPUT` gives no argument, and `a b.csv` or `-dash.csv` one path after `--`.
    `make run INPUT=transactions.csv` prints the sample's table byte for byte.
  - Once, from the root, with the person's consent: `cargo run -- transactions.csv`,
    `cargo run -- --help` and `cargo run -- --version`.
    - Run: the sample exits 0 with its table on stdout, byte for byte, and its report on stderr;
      `--help` prints `HELP` and `--version` prints `cthulhu 0.1.0`, both exiting 0.
  - The code-reviewer found no blocker. Its three should-fixes are taken: `clippy.toml` also
    disallows `clap::Command`'s entry points, which the derive reaches through `CommandFactory`;
    the `rust-effects` skill and the Banned table name clap's calls; and `make run` passes `--`
    before `INPUT`. Its nits are taken too: `main`'s doc comment, the manifest's clap line within
    100 columns, and reasons that name the help and the version among the exits.
  - Commit `feat(cli): parse the arguments with clap`.
- [ ] **4, fix.** The two tests that need a stream no write can reach take a socket shut for
      writing, not a pipe whose reading end was dropped: another process can hold a copy of that
      reading end, and a write then succeeds. A pre-existing race, found while running the green.
  - Commit `test(cli): make the unwritable streams fail whoever holds them`.
- [ ] **5.** `rand` 0.10 in `backend`'s dev-dependencies; the Makefile, `CLAUDE.md` and
      `README.md` say "the dev targets" where they said "tests"; this plan's Review.
  - Commit `chore(backend): add rand as a dev-dependency`.
- [ ] **6.** Prompt history 0007, alone:
      `docs(prompts): add prompt history 0007 (workspace packages and admitted crates)`.
- [ ] **7.** Report what is on the branch. Landing is the person's.

## Per-commit gate

1. `make ci`; for the red commit, only its new and rewritten tests fail.
2. The code-reviewer agent on the staged diff, with every blocker and should-fix acted on.
3. The two forbidden-name greps, both silent, built from a pattern file kept in the scratchpad.
4. The record scan: `git grep -n --untracked -E
   'docs/(adr|design)/[0-9]{4}|ADR ?[0-9]+|[Dd]ecision #?[0-9]+|record #?[0-9]+' -- .
   ':!docs/adr' ':!docs/design' ':!.claude/skills/architecture-decisions'` lists nothing.
5. `git commit -F` from a scratch file, staging explicit paths. Paths written here show the
   checkout as `<repo>`.
