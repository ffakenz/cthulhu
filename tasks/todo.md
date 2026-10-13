# Test audit nits — plan

**Goal:** close every nit still open from the audit of the black-box tests and from this branch's
review:
- a failing run law shows the inputs it ran and what they gave;
- the engine's laws reach counts above one, and the malformed forms that no generator wrote;
- the binary's fixtures pin every word they check;
- the wording of a malformed row lives in one place;
- the balance laws state all that holds, drawn from the one generator of amounts;
- the closed plans and their references agree with the conventions.

**Context:** the audit of the tests on `main` at `9e4fe53` left six nits, and the branch's review
left four once the person had chosen three. The person asked to continue with those still open,
and asked why `deposits` in `app/tests/it/cli.rs` writes each client's number as its tx id. The
work continues on `fix/test-audit-findings`, which has not landed; the audit fixes' closed plan is
kept as [tasks/test-audit-fixes-plan.md](test-audit-fixes-plan.md). The person's approval of this
plan accepts the production refactor in decision 1, the convention in decision 8 and the second
review in decision 9.

## Decisions

1. **A malformed row's kind prints as its error reads.** `render_report` writes each malformed
   kind through `RowError`'s `Display`, and the `malformation` match is deleted.
   - The two wordings agree today, but nothing prints the `#[error]` texts, so they could drift
     apart silently.
   - *csv, serde, rust_decimal, thiserror, clap, proptest and rand as dependencies* names
     `RowError` and `AmountError` among the error types whose messages `thiserror` derives. Their
     texts stay, and the report reads them.
   - A rejection's reason keeps its wording in `text_io`: `Reason` is not an error and has no
     `Display`.
   - A refactor: the binary's fixture pins every malformed kind's words, before and after.
2. **A failing run law shows both texts and both results.** The four run laws that return a bool
   assert instead, as the pairs law does. A failure prints the CSV text of each run and each
   summary; the header law prints its text and the result.
   - *Correctness defined by equations between runs* says a failing equation prints both inputs
     and both sets of accounts. Today these four print only the generators' structs.
   - Rejected: `for_all` taking a reason instead of a bool. Every law would change, the domain's
     included, whose inputs already print in full.
3. **Several malformed lines, of any kinds.** The malformed-line law inserts one to three flawed
   lines among valid ones, each counted once as its kind, so counts above one are pinned below the
   binary. It becomes `prop_malformed_lines_change_nothing_but_their_counts`.
4. **The malformed forms no generator wrote**, all but the third in the overview's Input:
   - a flaw: an amount quoted after a space keeps its quotes, so it is refused as malformed;
   - a generator of headers that misfit the lines under them, and a law: under such a header each
     line is malformed and no account opens:
     - a header naming `type`, `client`, `tx` or `amount` twice: each line is unreadable;
     - a header ending in a column besides the four, which each line stops before: unreadable;
     - a header naming no `amount`, over deposits and withdrawals: no amount.

     The lines are canonical: the misfit lies in the header, and the spellings' law already
     covers how each field is written.
5. **The binary's fixtures spell out what they check.**
   - The three expected error lines built from `SourceError`'s and `SinkError`'s `Display` are
     written out, so the binary's level pins their words.
   - `an_unreadable_input_stops_the_run_with_no_table` runs on Unix only, as its neighbours do:
     elsewhere, opening a directory fails before the read the test means.
   - `deposits` draws its tx ids apart from its clients. The numbers stay the same, so every
     deposit is still accepted, but the line no longer reads as a typo.
6. **The balance laws state all that holds, from the one generator of amounts.**
   - `any_amount` maps `input::any_money()` to units through a new `Money::units`; `CAP` and the
     repeated edges go. Probe: dropping the bound still fails the printing and grouping laws, and
     a bound at 10^20 the 32-doublings law, in 3 runs of 3 each.
   - The grouping law asserts `in_turn == at_once`. Two moves the same way are both refused or
     equal, since the accepted range is an interval. Probe: it holds over 20,000 cases, and a
     per-move limit on debits fails it where today's law passes.
   - Rejected: deriving `CAP` from `TOO_LARGE`, which keeps a second generator of the same values.
7. **`LACKS` names its parts:** a `Lack`, with the `column` left out and the name written
   `instead`, if any. `BREAKS` keeps its pairs, which read as the text before and after, as the
   review allowed.
8. **Closed plans agree with the conventions.**
   - Prompt history 0008's narrative names `tasks/model-free-tests-plan.md`, as the conventions
     ask of every reference but a quoted prompt.
   - `CLAUDE.md` says a closed plan is kept verbatim but for ticking the items done after its
     Review: the prompt history and the hand-off. The model-free tests' archive did that; the
     archive here needs no tick.
9. **A review of this plan's commits.** The branch's one review covered the commits before them.
   This work changes production code and five test files, so it gets one review of its own.

## Files and interfaces

| File | After the change |
|---|---|
| `app/src/text_io.rs` | a malformed kind prints through `RowError`'s `Display`; `malformation` deleted |
| `app/tests/it/run.rs` | the four bool laws assert, with both texts and both results; the malformed-line law takes up to three flawed lines; the misfit law; the module doc to match |
| `app/tests/it/support/input.rs` | the late-quote flaw; `Misfit`, its generator and its text; `Lack`; `Money::units` |
| `app/tests/it/domain.rs` | `any_amount` maps `any_money`, and `CAP` goes; the grouping law asserts the equality |
| `app/tests/it/cli.rs` | three error lines written out; the directory test on Unix only; `deposits` draws its tx ids apart |
| `docs/testing.md` | the engine's row names several malformed lines and the misfits |
| `docs/prompts/0008-model-free-tests.md`, `docs/prompts/0009-test-audit-fixes.md` | name the closed plans |
| `CLAUDE.md` | a closed plan's ticks |
| `tasks/test-audit-fixes-plan.md` | the audit fixes' closed plan, verbatim |

## Tests

No check here has a red: each pins behaviour that already works, or restates a law. Instead, each
is shown to catch its break in a scratch copy, never in the repo; the results go in the Review.

| Check | Break it must catch |
|---|---|
| the report through `RowError`'s `Display` | `RowError::MissingAmount`'s message changed: the fixture fails, where today nothing does |
| the four laws' failure output | one break per law: its failure shows both texts and both results |
| several malformed lines | a malformed kind counted once, however often it occurs |
| the late-quote flaw | quotes stripped from an amount's text |
| a header naming a column twice | a header's repeated names dropped before its rows are read |
| a header ending in a column the lines stop before | a short row padded to the header's length |
| a header naming no amount | a deposit or a withdrawal under such a header read as a dispute |
| `Lack` | header names compared ignoring case |
| `any_amount` from `any_money` | the bound dropped; the bound at 10^20 |
| the grouping law's equality | a per-move limit on debits |
| the error lines written out | `SourceError::Unreadable`'s message changed |

## Tasks

Every commit passes `make ci` and the forbidden-name check.

- [x] **0.** The archive and its link; this plan. Commit
      `docs(tasks): keep the audit fixes plan and plan the nits`.
- [x] **1.** The report through `RowError`'s `Display`. Commit
      `refactor(app): print a malformed row's kind as its error reads`.
- [x] **2.** The four laws assert. Commit
      `test(run): show both texts and both results when a run law fails`.
- [x] **3.** Several flawed lines. Commit `test(run): count several malformed lines of any kinds`.
- [x] **4.** The late quote. Commit `test(run): refuse an amount quoted after padding`.
- [x] **5.** The misfits and their law. Commit
      `test(run): find each line malformed under a header that misfits it`.
- [x] **6.** `Lack`. Commit `refactor(run): name the column a header lacks and its stand-in`.
- [x] **7.** `any_amount` from `any_money`. Commit
      `test(domain): draw the balance laws' amounts from the one generator of amounts`.
- [x] **8.** The grouping law's equality. Commit
      `test(domain): assert that two moves equal one by their sum, refusals included`.
- [x] **9.** The error lines written out. Commit
      `test(cli): spell out the error lines the binary prints`.
      Deviation: amended before the next commit, so that `UNWRITABLE` is `#[cfg(unix)]`, as its one
      test is.
- [x] **10.** The directory test on Unix only. Commit
      `test(cli): run the unreadable-input test on Unix only`.
- [x] **11.** `deposits`' tx ids. Commit
      `refactor(cli): draw the deposits' tx ids apart from their clients`.
- [x] **12.** Evidence, in a scratch copy and never in the repo: each break in the table fails its
      check. The results go in the Review.
- [x] **13.** The engine's row in `docs/testing.md`. Commit
      `docs(testing): name what the engine's laws reach now`.
- [x] **14.** Prompt history 0008's narrative. Commit
      `docs(prompts): name where the model-free tests plan is kept`.
- [x] **15.** The convention on ticks. Commit
      `docs(conventions): let a closed plan tick the items done after its Review`.
- [x] **16.** One `code-reviewer` run on this plan's commits; its fixes go in new commits.
      Deviation: no blocker. The two should-fixes landed as
      `refactor(run): name the misfits' spelling for what it writes` and
      `docs(tasks): give the closed plans' row the convention's exception`.
- [x] **17.** The Review and any lesson. Commit `docs(tasks): close the nits plan`.
- [ ] **18.** Prompt history 0009 gains this work's turns, alone:
      `docs(prompts): add the nits to prompt history 0009`.
- [ ] **19.** Report what is on the branch. Landing is the person's.

## Review

**What the work became.** Nineteen commits on `537e953`, this Review's and the prompt history's
included; the test crate went from 28 tests to 29.
- A malformed row's kind prints through `RowError`'s `Display`, so its words live once, in the
  error's message.
- The four run laws that returned a bool assert: a failure prints the CSV text of both runs and
  both results.
- The malformed-line law takes one to three flawed lines, a new flaw quotes an amount after
  padding, and a new law covers the three headers that misfit their lines.
- The balance laws draw their amounts from `input::any_money()`, and the grouping law asserts the
  equality outright.
- The binary's fixtures spell out their error lines, the unreadable-input test is Unix-only, and
  `deposits` draws its tx ids apart.
- `Lack` names its parts, prompt history 0008 names its closed plan, and `CLAUDE.md` and
  `tasks/README.md` let a closed plan tick the items done after its Review.

**Evidence.** Each break ran in the scratch worktree, at `61ecf01` and, where the check is new, at
`537e953`, the branch before this plan.

| Break | Fails at `61ecf01` | At `537e953` |
|---|---|---|
| `RowError::MissingAmount`'s message changed | the rejected-and-malformed fixture | escaped every test |
| `SourceError::Unreadable`'s message changed | the unreadable-input fixture | escaped every test |
| a malformed kind counted once, however often it occurs | the malformed-lines law, the misfit law and the fixture | the fixture only |
| quotes stripped from an amount's text | the malformed-lines law | escaped every test |
| a header's repeated names dropped before its rows are read | the misfit law | escaped every test |
| a short row padded to the header's length | the misfit law | escaped every test |
| a deposit or a withdrawal under a header with no amount read as a dispute | the misfit law | escaped every test |
| header names compared ignoring case | the incomplete-header law | not rerun: caught before too |
| the bound dropped, in 3 runs | the printing and grouping laws and the largest-balance example, in each | not rerun |
| the bound at 10^20, in 3 runs | the 32-doublings law and the largest-balance example, in each | not rerun |
| a per-move limit on debits | the grouping law, the largest-balance example and the pairs law | the example and the pairs law; the grouping law passed |

Each of the four laws that now assert was failed once more, by the report keeping only its last
rejection, fields left untrimmed, extra fields read, and the header compared ignoring case. Each
failure printed the CSV texts and both results. The spelling law, for one, printed
`"note,amount,type,client,tx\nx,0.0001,deposit,0, 0"` against
`"type,client,tx,amount\ndeposit,0,0,0.0001\n"`, with both summaries.

**What the framing got wrong.**
1. Decision 4 called the misfits' lines canonical. Their spelling writes each amount at its
   shortest, where `input::canonical` writes four decimals; the review had it named plain.
2. Decision 8 changed the convention in `CLAUDE.md` alone. `tasks/README.md` states it too, and
   the review caught the two disagreeing.
3. Task 9 defined `UNWRITABLE` on every target, while its one test runs on Unix only. The commit
   was amended before the next one; no target but the host's is installed, so no build off Unix
   checked it.

**Deliberately not done.** The review's one nit, for the person to choose: the grouping law's
label, "or both moves are refused", can be read as the two amounts' moves. "Refusals included",
the commit's wording, reads one way only.

No lesson: the person corrected nothing.
