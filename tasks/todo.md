# Property laws on proptest — implementation plan

> **For agentic workers:** superpowers:executing-plans, inline, in this checkout on
> `feat/stage-1-core`. Steps use checkbox (`- [ ]`) syntax; `- [x]` means the command beside it
> ran and its output supported the claim.

**Goal:** the library's 25 property laws run on `proptest`: the same laws over the same
populations, with counterexamples shrunk and failures persisted, the hand-written generator and
runner gone, and the dependency gate admitting proptest for tests only. One new commit on top of
the branch, folded into nothing, after the record that admits the crate.

**Approach:** each generator becomes a strategy in `backend/tests/it/support/generate.rs`. A
generator whose draws depend on earlier ones draws its choices first and then interprets them, so
every shrunk input is still one the generator could produce. `support::for_all` stays, as a thin
runner over `proptest::test_runner::TestRunner`, so each law keeps its label and its body.

**Tech stack:** Rust 1.88.0, edition 2024; `proptest` 1.12 (`default-features = false`, `std`) as
a dev-dependency of `backend`.

**Spec:** *Correctness defined by laws and a model* (laws over generated raw input weighted to
edges; each law shown failing once; a failing law prints its seed and replays exactly); *csv,
serde and rust_decimal as dependencies*, which this work supersedes; `CLAUDE.md` § Dependencies
and § Tests; the `rust-properties`, `review-and-planning` and `architecture-decisions` skills.

## Decisions for sign-off

Approving this plan signs off the first two decisions, and the record lands `Accepted`.

1. **Admit proptest 1.12.0 for tests only**, `default-features = false, features = ["std"]`.
   - Why: a failing law shrinks to a small counterexample instead of the first random one, and the
     failure persists and replays; strategies replace the draw-by-draw plumbing; the runner and
     seed handling of `support.rs` stop being ours to maintain.
   - Cost: 14 crates in the test build graph and none in the program's: `proptest`, `bitflags`,
     `cfg-if`, `chacha20`, `core_detect`, `cpufeatures`, `getrandom`, `libc`, `r-efi`, `rand`,
     `rand_core`, `rand_xorshift`, `regex-syntax`, `unarray`. It also builds on `num-traits` and
     `autocfg`, already admitted. Build scripts: `getrandom` and `libc`; no procedural macro.
     Slower clean test builds.
   - Age: 1.12.0 was published on 2026-10-10, the day before this plan. Since 1.11.0 it moves to
     `rand` 0.10, raises its minimum Rust to 1.88 (the pinned toolchain) and fixes bugs; none of
     its added lines touches `unsafe`, files, processes, the network or the environment.
   - Alternatives: keep the harness written here (no shrinking, ours to maintain); quickcheck
     (shrinks per type through `Arbitrary`, so generators that read earlier draws need
     hand-written shrinkers); proptest with default features (forking and timeouts, about 28
     crates); `proptest-state-machine` (requires forking and timeouts, and the laws' drivers are
     already folds); proptest 1.11.0 (seven months old; 17 crates, including `zerocopy-derive`, a
     procedural macro, and two WebAssembly-only crates; moving to 1.12 later changes the set, and
     so needs a second record).
2. **A record superseding *csv, serde and rust_decimal as dependencies***, titled *csv, serde,
   rust_decimal and proptest as dependencies*. It restates the program's 16 crates unchanged,
   admits the 14 above for tests only, has `make deps` keep them out of the program's graph, and
   lists the alternatives above as rejected. The superseded record changes its Status line only.
   It is committed alone, before the work, as the `architecture-decisions` skill requires.
3. **`support::for_all` stays, as a thin runner over `TestRunner`**, instead of `proptest!`
   blocks. The call shape is unchanged, so each law's body is untouched and only its generator
   argument changes. rustfmt keeps formatting every law, which it skips inside `proptest!`. The
   label stays the law's name in every failure, with no `prop_assert!` format-string trap. The
   runner sets the caller's file (`#[track_caller]`) so failures persist, and applies the
   `PROPTEST_*` overrides as the macro does (`contextualize_config`).
4. **Failures persist, as proptest's default**: to
   `backend/tests/proptest-regressions/<module>.txt`, replayed first on the next run. A
   regression file is committed with the fix of the failure it records, and never otherwise; no
   seed is pinned in code, configuration or CI. Deliberate breaks run with
   `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`.
5. **500 cases per law**, as today: a `CASES` constant; `PROPTEST_CASES` overrides it for a deeper
   run.
6. **Strategies keep today's populations**: raw spelling, then the production parser; the same
   edge tables and weights (`prop_oneof![3 => ordinary, 1 => edge]` is one draw in four); choices
   then interpretation for `balance_moves`, `rows`, `interleave` and the store operations, with an
   `Index` for every pick among what earlier draws produced; no `prop_filter`, no `prop_assume!`.
7. **`make deps` runs two checks**: the whole graph against members, `ADMITTED` and
   `ADMITTED_DEV`; the graph without dev-dependencies (`--edges no-dev`) against members and
   `ADMITTED` alone.
8. **Commits**: the record alone; then this work as one new commit; then, when the session closes,
   its prompt history alone.

## Global constraints

- Branch `feat/stage-1-core`. Never push, merge or touch a remote; landing is the person's, a
  fast-forward of `main`.
- Commit format: `action(scope): description`, a blank line, `> impact`, no trailer lines. The
  message is written to a scratch file outside the repository and committed with
  `git commit -F`; stage explicit paths, never `git add -A`.
- No dependency but proptest, added by hand to `backend/Cargo.toml`, never by `cargo add`.
  `Cargo.lock` gains proptest's packages and changes no existing pin.
- Nothing under `backend/src` changes. No test-only code in production; every helper in
  `backend/tests/it` is used, or `dead_code` fails `make lint`.
- Comments say what the code is and does: never a record number, a plan, a ticket or a
  trade-off.
- Never name the organisation behind the external brief or the project the decision log's style
  comes from, and never quote, restate or paraphrase the brief, in any tracked file or commit
  message. Before each commit, the two greps with the pattern file the session keeps in its
  scratchpad, never in the repository: `git grep -n -i -E --untracked -f <pattern file>` and
  `git diff --cached | grep -n -i -E -f <pattern file>`, both silent.
- A record is named by its title outside the decision logs, this plan included.
- Every law is shown failing once on proptest, against the same deliberate break as in stage 1
  (each stage 1 ticket's Law evidence table). A break is applied and undone by editing, with the
  work staged first, so `git diff --quiet` proves each undo.

## File map

| Path | Change |
|---|---|
| `docs/adr/` | the new record, its index line, the superseded record's Status line (record commit) |
| `backend/Cargo.toml`, `Cargo.lock` | `[dev-dependencies] proptest`; the lock entries it adds |
| `Makefile` | `ADMITTED_DEV`; the no-dev check; the `deps` help line |
| `backend/tests/it/support.rs` | `CASES`, `Failure`, `for_all` over `TestRunner`; `Gen`, `Seed`, `EDGE_ODDS` and `seed` removed; `infallible` kept |
| `backend/tests/it/support/generate.rs` | every generator a strategy (Strategy map); tables, `InputRow`, the builders, `row`, `command`, `unique`, `moved`, `zeroing` and `unpadded` unchanged |
| `backend/tests/it/{domain,core,run,memory}.rs` | each `for_all` call takes a strategy; `broken_source`, `reordering`, `operations` and the sightings become strategies |
| `backend/tests/it/support/model.rs` | unchanged |
| `.claude/skills/rust-properties/` | `SKILL.md` on proptest; `references/harness.md` replaced by `references/proptest.md` |
| `.claude/skills/rust-effects/SKILL.md` | the sentence on the harness's clock, environment and `catch_unwind` removed |
| `.claude/skills/functional-architecture/SKILL.md` | its pointer to `rust-properties` names strategies and their runner |
| `.claude/skills/review-and-planning/SKILL.md` | the test-only dependency check in the present tense; the harness lesson's rules |
| `CLAUDE.md`, `README.md` | the test-only crates and the no-dev check; `CLAUDE.md` § Tests describes the support tree |
| `tasks/todo.md`, `tasks/lessons.md` | this plan and its Review; the harness lesson's graduation line |

## Strategy map

| Generator | Today, on `&mut Gen` | Strategy |
|---|---|---|
| `amount_spelling` | an edge one in four, else a number-shaped spelling or loose characters, as often each | `prop_oneof![2 => edge, 3 => number-shaped, 3 => characters]` |
| `amount_value` | an edge one in four, else `next_u64 >> below(64)` and a fraction | `prop_oneof![1 => edge, 3 => (any::<u64>(), 0..64, 0..10_000)]` |
| `any_text` | an edge one in four, else up to 16 characters | `prop_oneof![1 => edge, 3 => vec(character, 0..=16)]` |
| `direction`, `move_amount`, `ordinary_amount` | single draws | `any::<bool>()`; `prop_oneof![1 => edge, 3 => 1..=9_999_999 units]`; `(0..100, option::of(vec(digit, 1..=4)))` |
| `balance_moves` | each move repeats the last the other way one in four, else zeroes the balance one in four, else is fresh | `vec(move draw, 0..=12)`; each draw names repeat, zero or fresh and carries its fresh move, used when the other two do not apply |
| `rows` | ordinary rows and, one in four, an edge's rows, until the drawn count | `vec(draw, 0..=16)`: an ordinary draw or one of the seven edge draws, every pick among earlier rows an `Index`, interpreted by `Draft` while it holds fewer rows than draws, then cut to 16 |
| `commands`, `events`, `address` | derived or single draws | `rows()` read as commands; `vec(event, 0..=12)`; a client and a tx id from their tables |
| `interleave` | the next client drawn among those waiting | turns, `vec(any::<Index>(), 16)`, each indexing the clients still waiting |
| run: `broken_source`, `reordering` | a position and an error; an interleaving | `(rows(), any::<Index>(), select(SOURCE_ERRORS))`; `(rows(), turns)` |
| memory: `operations`, `load_address`, sightings | a probe, then a commit per event at a drawn address with a probe after it, one in two | `(option::of(probe), vec((event, address, option::of(probe)), 0..=12))`, interpreted with the reference and `evolve`; `vec(select(SIGHTED), 0..=24)` |

## Phases

### Phase 0 — evidence (read-only)

Run in scratch packages and in copies of the repository, never in this checkout.

- [x] proptest 1.12.0 and 1.11.0 resolve, build and pass on 1.88.0 with
  `default-features = false, features = ["std"]`; `cargo clippy --all-targets -- -D warnings` is
  clean under this workspace's lints and `clippy.toml`.
- [x] `make deps` in a copy of the repository with the dev-dependency names the crates it would
  admit: the 14 listed above for 1.12.0, 17 for 1.11.0.
- [x] Seeds: a random seed per run; `PROPTEST_RNG_SEED` replays a run; a failure prints a `cc`
  line and saves it to `backend/tests/proptest-regressions/<module>.txt` for this crate's layout,
  replayed first on the next run; `PROPTEST_DISABLE_FAILURE_PERSISTENCE` turns that off.
- [x] `TestRunner::run(&strategy, Fn(value) -> TestCaseResult)` outside the macro: a panic is a
  failure with its message, the input is shrunk, and failures persist only when the config names
  a source file. `contextualize_config` is public and applies the `PROPTEST_*` variables.
- [x] `diff -r` of 1.11.0 and 1.12.0: no added line touches `unsafe`, files, processes, the
  network or the environment; in 1.12.0's graph only `getrandom` and `libc` have build scripts.

### Task 1 — the record

- [x] The next record in `docs/adr/`, from `template.md`, as Decisions for sign-off describes it;
  its index line; the superseded record's Status set to `Superseded by` the new record's number.
- [x] Per-commit gate, then commit alone: `74dad33`, signed. The code-reviewer's four should-fixes
  and five nits were all taken before it landed; `make ci` green, 26 passed.

### Task 2 — the gate

- [x] `backend/Cargo.toml`: `[dev-dependencies]` with
  `proptest = { version = "1.12", default-features = false, features = ["std"] }`.
- [x] `cargo check --workspace --tests`, once without `--locked`, records proptest in
  `Cargo.lock`; the lock's diff only adds packages: "Locking 12 packages", proptest 1.12.0 and 11
  others; the one removed line is a lock-only `getrandom` naming `r-efi 5.3.0` beside the new
  6.0.0, no pin changed. Before the Makefile changed, `make deps` failed naming exactly the 14
  crates of the record.
- [x] `Makefile`: `ADMITTED_DEV` and the no-dev check, each graph read once with `|| exit 1`.
- [x] `make deps` green. With proptest moved to `[dependencies]` by hand, `make deps` fails: "The
  program build graph holds crates admitted for tests only", the 14 listed; with `unarray` dropped
  from `ADMITTED_DEV`, it fails: "The build graph holds crates no decision record admits: unarray".
  Both undone by editing; green again.

### Task 3 — runner and strategies

- [x] `support.rs`: `CASES`; `Failure` (a counterexample with its shrunk input and reason, or an
  aborted run); `for_all(law, strategy, holds)` over `TestRunner`, with the caller's file as the
  source file and `contextualize_config` applied; `infallible` kept.
- [x] `support/generate.rs`: the Strategy map, each doc comment still saying what the strategy
  draws.
- [x] `cargo check --workspace --tests` compiles `support/`: with the old callers, every error it
  reported in `support/` was a note of a caller's error; with Task 4's callers, it compiles with
  no warning.

### Task 4 — laws

- [x] `domain.rs`, `core.rs`, `run.rs`, `memory.rs` on the new `for_all`; each law body unchanged
  apart from its generator argument: `git diff -w` shows only the calls' arguments.
- [x] `make ci` green: "test result: ok. 26 passed; 0 failed".
- [x] Before (the record's commit) and after, in scratch copies with their own `target/`: a clean
  `cargo test --workspace --locked --no-run` takes 2.4 s and 4.2 s; the 26 tests run in 0.09 s
  and 0.36 s.

### Task 5 — evidence

- [x] With the work staged and `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`, each of the 25 laws fails
  against its stage 1 break, with its shrunk counterexample in Law evidence; after each undo,
  `git diff --quiet` exits 0. 29 breaks, 25 laws, every run `test rc=101`; no regression file
  written.
- [x] With persistence on, one break writes a regression file holding a `cc` line, and the next
  run fails on the replayed case before any new one; the file is deleted, the break undone, and
  `git status --short` shows only the staged work. The spelling law's `+` break: the first run
  printed "Saving this and future failures in
  .../backend/tests/proptest-regressions/domain.txt" and `cc 1695…f706`, the file held that line
  "# shrinks to "+0"", and the second run failed on `"+0"` with nothing new saved.

### Task 6 — docs and skills

- [x] `rust-properties/SKILL.md`: totality, labels, raw spelling, edge weights, seeds and replay
  on proptest; a rule for choices then interpretation; a rule against filtering what can be built;
  Verify with `make test`, `PROPTEST_CASES` for a deeper run and the regression file for replay.
- [x] `references/proptest.md` in place of `references/harness.md`: `for_all`, a worked pair of
  round trips, a strategy whose draws depend on earlier ones, a state-machine driver, and
  replaying a failure. Every example ran in a scratch crate under this workspace's lints: clippy
  clean; the delegating parser's spelling law shrinks to `"+1"` or `"01"`, and the state machine
  broken to reopen a `Recorded` order shrinks to `[Submit, Written, Submit]` in three runs of
  three; the replay lines are this plan's persistence run.
- [x] `rust-effects`: the harness sentence removed. `review-and-planning`: the test-only check in
  the present tense, and the harness lesson's rules (propose the crate before hand-rolling what it
  does; name the crate a copied piece of infrastructure stands in for). `tasks/lessons.md`: the
  lesson's graduation line. `functional-architecture`'s pointer to `rust-properties` names
  strategies and their runner.
- [x] `CLAUDE.md` § Dependencies and § Tests, and `README.md`, in step with the record.
- [x] `git grep -n -E 'PROPERTY_SEED|SplitMix|EDGE_ODDS|EVEN_ODDS|one_in|harness\.md' -- .
  ':!tasks' ':!docs/prompts'` lists nothing.

### Task 7 — commit

- [x] Per-commit gate; commit with the message in Commits. `make ci` green, 26 passed; the
  code-reviewer's two should-fixes and the final review's minors taken (Review); the greps and the
  record scan silent.
- [x] Review below.

## Law evidence

One row per break, filled in Task 5: the law, its break from the stage 1 tickets, and the
counterexample proptest shrank it to. Commands and rows are written `deposit(client, tx, amount)`,
`withdrawal(…)`, `dispute(client, tx)`, `resolve(…)` and `chargeback(…)`, a row's amount quoted as
its text; a store operation names its address as `(client, tx)`. The second balance break is the
one the stage 1 final review added, rebuilt here: `checked_sub` negates a zero reached across two
scales. The table is the run on the code as committed, after the review fixes: every break failed
(`test rc=101`, 29 of 29) and was undone (`git diff --quiet` exits 0), and no regression file was
written.

| Law | Break | Shrunk counterexample |
|---|---|---|
| `prop_amount_text_parses_back_to_its_value` | the size check refuses fifteen whole digits (`>=`) | `AmountValue { whole: 999999999999999, fraction: 9999 }` |
| `prop_amount_spelling_parses_exactly_when_the_grammar_admits_it` | `from_str` strips a leading `+` first | `"+1"` |
| `prop_amount_parsing_never_panics` | `from_str` reads `text.as_bytes()[0]` first | `""`, panicking: index out of bounds |
| `prop_balance_never_prints_negative_zero` | `checked_sub` computes `-(other - self)` | `[(Credit, "0.0001"), (Debit, "0.0001")]` |
| `prop_balance_never_prints_negative_zero` | `checked_sub` negates a zero reached across two scales | `[(Credit, "0.0001"), (Debit, "0.0001"), (Credit, "711.6569"), (Credit, "0.0001"), (Debit, "711.657")]` |
| `prop_decide_never_changes_state` | a `static` `AtomicBool` that `decide_client` flips on every call | `[deposit(1, 0, 1)]` |
| `prop_evolve_is_total` | `Released` reads the footprint's deposit with `.expect("disputed")` | `([deposit(1, 0, 0.1)], [Released(0.0001)])`, panicking: disputed |
| `prop_held_is_the_sum_of_disputed_deposits_and_never_negative` | `Released` leaves `held` unchanged | `[deposit(1, 1, 1), dispute(1, 1), resolve(1, 1)]` |
| `prop_total_is_deposited_minus_withdrawn_minus_charged_back` | `ChargedBack` leaves `held` unchanged | `[deposit(100, 0, 1), dispute(100, 0), chargeback(100, 0)]` |
| `prop_locked_exactly_when_charged_back` | `ChargedBack` keeps the account's lock | `[deposit(2, 0, 1), dispute(2, 0), chargeback(2, 0)]` |
| `prop_only_held_makes_available_negative` | `InsufficientFunds` compares with `total` | `[deposit(100, 1, 0.1), dispute(100, 1), withdrawal(100, 3, 0.0001)]` |
| `prop_dispute_lifecycle_allows_exactly_its_transitions` | resolve on a `Settled` deposit answers `Released` | `[deposit(3, 0, 0.0001), resolve(3, 0)]` |
| `prop_client_commands_follow_the_commands_table` | `InsufficientFunds` when the amount is not below `available` (`>=`) | `[deposit(100, 0, 0.1), withdrawal(100, 1, 0.1)]` |
| `prop_client_commands_follow_the_commands_table` | the lock checked for deposits only | `[deposit(100, 0, 0.1), dispute(100, 0), chargeback(100, 0), withdrawal(100, 2, 0.1)]` |
| `prop_first_command_opens_the_account` | `open` locks the account | `[deposit(1, 0, 0.1)]` |
| `prop_first_command_opens_the_account` | `Opened` makes `open()` from a loaded footprint | `[deposit(2, 0, 1), dispute(2, 0)]` |
| `prop_tx_ids_are_consumed_on_first_sight_across_clients` | withdrawals skip intake and are decided as `Sight::First` | `[deposit(1, 0, "0.1"), withdrawal(1, 0, "0.1")]` |
| `prop_rejection_changes_no_balance` | after a rejection at a loaded footprint, `apply_at` commits `open()` | `[deposit(100, 0, "0"), deposit(100, 1, "0.1"), dispute(100, 0), chargeback(100, 0)]` |
| `prop_malformed_rows_are_skipped_and_counted` | `Row::Malformed` is skipped uncounted | `[Unreadable]` |
| `prop_source_error_stops_the_run_with_no_summary` | an `Err` row is skipped like a malformed row | `BrokenSource { rows: [], position: 0, first: Unreadable }` |
| `prop_source_error_stops_the_run_with_no_summary` | the source collected whole before the run | `BrokenSource { rows: [], position: 0, first: Unreadable }` |
| `prop_report_stays_bounded` | rejections kept as a `Vec`, one entry per refused row | `[withdrawal(1, 0, "0.1"), deposit(100, 1, "0"), deposit(100, 2, "0"), dispute(100, 1), chargeback(100, 1)]` |
| `prop_same_rows_give_the_same_summary` | a `static` run counter added to the first rejection count | `[withdrawal(1, 0, "0.1")]` |
| `prop_reordering_clients_changes_no_account` | a one-entry footprint cache keyed by the tx id alone | original `[dispute(1, 0), deposit(1, 1, "0.1"), dispute(2, 1)]`, interleaved `[dispute(1, 0), dispute(2, 1), deposit(1, 1, "0.1")]` |
| `prop_engine_agrees_with_the_naive_model` | `InsufficientFunds` when the amount is not below `available` | `[deposit(100, 0, "0.1"), withdrawal(100, 1, "0.1")]` |
| `prop_read_returns_what_the_last_commit_wrote` | `commit` removes nothing when the footprint holds no deposit | `[Commit((3, 1), a Settled deposit of 0.0001), Commit((3, 1), no deposit), Load((3, 1))]` |
| `prop_commit_writes_account_and_deposit_together` | `commit` skips the account when the footprint holds a deposit | `[Commit((1, 1), a Settled deposit of 0.0001)]` |
| `prop_snapshot_reflects_every_commit_before_it` | `snapshot` leaves out accounts whose balances are all zero | `[Commit((1, 1), zero balances, no deposit), Snapshot]` |
| `prop_recorded_id_stays_recorded` | `sight` forgets an id when it answers `Repeat` | `[4294967295, 4294967295, 4294967295]` |

## Commits

```text
docs(adr): record csv, serde, rust_decimal and proptest as dependencies

> The property tests can use a maintained property-testing crate, and the dependency gate keeps
> it, and every crate it brings, out of the shipped program.
```

```text
test(it): run the laws on proptest strategies that shrink

> A failing law now reports a minimal counterexample and replays from a regression file, and the
> project no longer maintains its own generator, runner and seed handling.
```

When the session closes, its prompt history follows alone, as the `prompt-history` skill says.

## Per-commit gate

1. `make ci` green.
2. The code-reviewer agent reviews the staged diff; every blocker and should-fix is acted on and
   `make ci` re-run.
3. The two forbidden-name greps of Global constraints, both silent. No record cited by number or
   filename outside the decision logs: `git grep -n --untracked -E
   'docs/(adr|design)/[0-9]{4}|ADR ?[0-9]+|[Dd]ecision #?[0-9]+|record #?[0-9]+' -- .
   ':!docs/adr' ':!docs/design' ':!.claude/skills/architecture-decisions'` lists nothing.
4. `git status --short` lists only the commit's files.

## Review focus

1. Every draw list interprets, shrunk ones included: an empty list, a pick with nothing earlier
   to pick from, a repeat with no previous move. Each falls back as today's generator does.
2. Each strategy draws the population its doc comment describes, with today's edge tables and
   weights.
3. The no-dev check reads the program's graph, shown by its deliberate break.
4. No law is weakened: each body is unchanged apart from its generator argument.

## Risks

- `cargo tree --target all` and the first proptest download need the network, as `make deps`
  already does.
- A strategy that loses an edge passes its law forever. The 25 breaks guard against it: a break a
  law no longer catches in three runs means the strategy lost its region, and the strategy is
  fixed, not the law.
- `Index::index` panics on an empty collection: every pick checks emptiness first and falls back
  as today.
- A shrinking law prints the panic message of each panicking attempt; the totality breaks' logs
  are noisy, not wrong.
- If proptest 1.12 gets a patch release, the lock is updated; a change to its graph needs a new
  record.

## Review

- **Two commits, not one.** The move was asked for as one new commit; the decision-record
  convention commits a record alone, so the record landed first and this work is the one commit
  after it, folded into nothing.
- **The gate needed a fix the plan did not foresee.** In the recipe, `$members$(ADMITTED)`
  expanded to `$membersarrayvec`, one unset name, so every crate looked unadmitted; `make -n deps`
  showed it, and `${members}` fixed it. The two deliberate breaks of the gate prove both checks.
- **The lints shaped names.** `shadow_unrelated` rejects a strategy bound under the name of the
  value it draws, so strategies are bound as `any_*`.
- **The reviews.** The commit gate's review found two should-fixes, both taken: choice types whose
  fields only some cases read became sums (`Ordinary`, `Edge::RepeatedTx`'s client, `Shape`), and
  the docs stopped implying that proptest catches a panic inside a strategy; it does not, and the
  test then fails with no law, no counterexample and no saved seed. The final whole-branch review
  found nothing critical or important. Its five minor points and the gate's seven nits were the
  accuracy of docs and names in files this commit writes, and were all taken, though the
  executing skill defers minors: each corrected a sentence that was not true.
- **Equivalence.** All 29 stage 1 deliberate breaks of the 25 laws fail on proptest, re-run on the
  code as committed. Both reviewers sampled 200,000 draws from each old generator and its strategy
  and found the populations equal within sampling noise. Most counterexamples shrink to one to
  four commands or rows; the cross-scale balance break ends at five moves, because proptest
  shrinks a list one element at a time and a repeated move depends on the move before it.
- **What it costs.** A clean test build takes 4.2 s instead of 2.4 s, and the 26 tests run in
  0.36 s instead of 0.09 s.
- **Deliberately not done.** No `proptest-derive` and no `proptest-state-machine`, which the record
  rejects. No regression file is committed: no real failure was found. Strategies that run
  production code, `row` parsing amounts and the store operations calling `evolve`, still do; the
  docs now say a panic there is not caught.

Still deferred from stage 1, whose plan this file held:

- The runtime opens an account through `open()` and never answers `Event::Opened`. Before an
  event log lands, decide whether a step yields it, so a replay reopens the clients named only by
  rejected commands.
- The reordering law could compare whole summaries, not only accounts: with unique tx ids the
  report is order-invariant too.
- `prop_decide_never_changes_state` checks determinism, all a `&`-typed decision can break;
  rename it or fold it into the table law.
- `Account` offers `is_locked() -> bool` beside the crate-visible `lock() -> Lock`; one public
  reading, the enum, would do.
