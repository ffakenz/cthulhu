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
- [ ] **1.** The three records, drafted together and shown to the person. On sign-off each is
      `Accepted` and committed alone with its index line:
  - *Layers as packages that depend inward*, committed as
    `docs(adr): record layers as packages that depend inward`
  - *csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies*, with the
    superseded record's Status line, committed as
    `docs(adr): record csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies`
  - *The command-line contract with help and version*, with the superseded record's Status line,
    committed as `docs(design): record the command-line contract with help and version`
- [ ] **2.** The split, a refactor.
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
    renamed whole; `git diff --cached -M -- libs/store/src/memory.rs` shows only `use` lines; each
    new manifest holds `[lints] workspace = true`; and the tests give 39 passed and 1 ignored, as
    before, with only their `use` lines changed.
  - Commit `refactor(workspace): split the library into engine, store and backend packages`.
- [ ] **3.** `thiserror`, a refactor.
  - `thiserror = "2.0.21"` in `engine` and `backend`; the derives and their `#[error]` attributes,
    transcribed byte for byte; the hand-written impls, and the imports only they used, removed.
    `ADMITTED` gains `thiserror` and `thiserror-impl`, and `CLAUDE.md` § Dependencies,
    `README.md`, `docs/architecture.md` and the `rust-errors` skill follow.
  - Evidence that the messages are equal: a throwaway test, never committed, prints every public
    error value's `to_string()` before and after, and the two outputs are identical.
  - Commit `refactor(errors): derive Display and Error with thiserror`.
- [ ] **4, red.** In `backend/tests/it/cli.rs`, `a_wrong_argument_count_is_a_usage_error`, which
      pins today's one-line usage, is rewritten to clap's messages for no argument and for two.
      New tests pin the help and version flags, an empty argument, a path after `--`, and a help
      that cannot be written. Each fails against today's binary, and nothing else fails.
  - Commit `test(cli): pin the help, the version and clap's usage errors`.
- [ ] **4, green.** `clap` 4.6.7 in `backend`, and `ADMITTED` gains `clap`, `clap_builder`,
      `clap_derive`, `clap_lex`, `anstyle` and `heck`. `main.rs` parses with `try_parse_from` and
      splits `clap::Error` with `use_stderr`. `clippy.toml` disallows the clap calls that read the
      process's arguments or exit it, if clippy accepts their paths. `README.md`,
      `docs/overview.md`, `docs/architecture.md`, `CLAUDE.md` § Dependencies and the CLI line of
      the `functional-architecture` reference follow.
  - Once, from the root, with the person's consent: `cargo run -- transactions.csv`,
    `cargo run -- --help` and `cargo run -- --version`.
  - Commit `feat(cli): parse the arguments with clap`.
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
