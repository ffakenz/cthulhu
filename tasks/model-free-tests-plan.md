# Model-free tests — plan

**Goal:** the black-box test crate checks the engine without re-implementing it. Every expected
value is either another run of the engine or a hand-computed fixture. The crate shrinks from 3,481
lines to about 760, and every behaviour the old laws pinned stays pinned, shown by the deliberate
breaks they were proven on.

**Context:** the person asked to simplify the tests while keeping their coverage, and to remove
the tests that mirror the code they test:
- the reference store beside `MemoryLedger`;
- the copies of the runtime and of the deciders in `core.rs`;
- the naive model, with its own `Account`;
- the strategies that track the engine's rules, and the spellings that copy the CSV adapter's.

Nothing under `app/src` or `libs/` changes. The work starts from `main` at `28ab587`, on
`feat/model-free-tests`, in a worktree of its own.

The person signed off this plan as written.

## Decisions

1. **Equations between runs.** After any generated prefix of rows, two suffixes that the rules
   make equivalent leave the same accounts. Both sides are runs of the engine, so no test computes
   a balance. Random rows over tiny domains give a client any state, and fresh tx ids let a suffix
   control its own deposit. The person chose this over step laws with expected deltas, which would
   restate the commands table, and over fixtures alone.
2. **The seam** is `run` over `CsvSource` and the in-memory stores. The stores' laws hold through
   the engine's, and the binary's fixtures stay the contract.
3. **The record:** *Correctness defined by equations between runs* supersedes *Correctness defined
   by laws and a model*. The docs follow at the close.
4. **Deliberately not covered:**
   - `evolve`'s `Opened` arm, which `run` never reaches;
   - `evolve` on footprints no run builds, where a panicking shortcut fails `make lint`;
   - overflow, which no test reached before either.
5. **The closed plan is kept** as `tasks/workspace-packages-plan.md`, with its last two items
   ticked first, and prompt history 0007's link names it.

## Files and interfaces

| File | After the change |
|---|---|
| `app/tests/it/main.rs` | the modules `support`, `cli`, `domain` and `run` |
| `app/tests/it/support.rs` | `for_all` and `Failure` as they are; `summary(text: &[u8])`: `run` over `CsvSource` and fresh in-memory stores |
| `app/tests/it/run.rs` | the row strategy, two laws and two examples (Tests) |
| `app/tests/it/domain.rs` | one round trip, one example and one law on money (Tests) |
| `app/tests/it/cli.rs` | `/dev/full` replaced by a portable write failure on `unwritable()`; a non-UTF-8 header; `input(name, contents: impl AsRef<[u8]>)` |
| deleted | `core.rs`, `memory.rs`, `csv_io.rs`, `support/generate.rs`, `support/spelling.rs`, `support/model.rs` |

## Tests

- **Rows:** 0–16 lines of any type, by clients 1–3, on tx ids 0–7, for amounts at mixed scales up
  to the cap.
- `prop_clients_run_independently`: with each deposit's and withdrawal's tx id set to its row
  index, the rows and the same rows stably sorted by client give the same summary.
- `prop_equivalent_commands_leave_the_same_accounts`: after any rows, with client `c`, another
  client `o`, a client `F` no row names, amounts `a` and `b` and fresh ids `X Y Z`, each pair leaves
  the same accounts. Here `deposit` is `deposit(c,X,a)`, `held` is `deposit·dispute(c,X)` and
  `charged` is `held·chargeback(c,X)`.

  | # | Left ≡ right | The rule |
  |---|---|---|
  | 1 | held·resolve ≡ deposit | a resolve releases what its dispute held |
  | 2 | deposit·resolve·chargeback ≡ deposit | only a disputed deposit resolves or is charged back |
  | 3 | held·dispute ≡ held | a deposit is disputed once at a time |
  | 4 | charged ≡ the same with `Y` and `b` | a chargeback reverses its deposit and locks, whatever the amount |
  | 5 | charged·deposit(c,Y,b) ≡ charged | a locked account refuses deposits |
  | 6 | charged·withdrawal(c,Y,b) ≡ charged | … and withdrawals |
  | 7 | charged·resolve ≡ charged | a charged-back deposit leaves history |
  | 8 | charged·dispute ≡ charged | … |
  | 9 | deposit(c,Y,b)·charged·dispute(c,Y) ≡ deposit(c,Y,b)·dispute(c,Y)·charged | dispute commands still apply on a locked account |
  | 10 | withdrawal(c,X,a)·withdrawal(o,X,b)·deposit(c,X,b) ≡ withdrawal(c,X,a)·dispute(o,Z) | a tx id is spent on first sight, refused or not, for every client |
  | 11 | deposit·dispute(o,X) ≡ deposit·dispute(o,Z) | a dispute reaches only its own client's deposits |
  | 12 | withdrawal(c,X,a)·dispute(c,X) ≡ withdrawal(c,X,a)·dispute(c,Z) | a withdrawal is not in history |
  | 13 | deposit(F,X,a)·withdrawal(F,Y,a) ≡ dispute(F,Z) | a withdrawal of exactly `available` is accepted |
  | 14 | deposit(F,X,a)·dispute(F,X)·withdrawal(F,Y,a) ≡ deposit(F,X,a)·dispute(F,X) | held funds cannot be withdrawn |
  | 15 | deposit(F,X,0)·deposit ≡ deposit | a malformed row opens no account and spends no tx id |

- **A source error stops the run:** after a header, a reader that always fails, then three
  malformed rows. The run stops on the error, having pulled exactly one item.
- **The input as the overview spells it** reads as its canonical spelling. It carries:
  - permuted columns, with an ignored one before `amount`;
  - padding, a BOM, a blank line, a quoted field, CRLF, and an id spelled `0x1`;
  - a dispute row cut before its amount, and one carrying an amount;
  - a non-UTF-8 row.
- `prop_amount_text_parses_back_to_its_value`: four-decimal text with edge-weighted whole parts
  parses and prints back, except zero and 10^15 and up; a leading zero and a fifth decimal change
  nothing.
- **Malformed spellings:** the list in `AmountError::Malformed`'s doc, each `Malformed`.
- `prop_credits_then_a_debit_of_their_sum_print_zero`: 2–4 credits at mixed scales, then one debit
  of their sum, print `0.0000`.
- `a_table_that_cannot_be_written_is_a_write_failure`: one deposit, and 300 clients, on an
  unwritable stdout give status 5 and the sink's message.

## Deviation: the person's request, after task 2

The person asked for one generator per field, composed into a line, and for success and failure
never to be checked in the same property. The generators live in `support/input.rs`:
- **Valid fields:** type, client, tx and amount. Each draws its value from the whole domain,
  edges weighted, and its spelling from every form the Input section of the overview admits; ids
  also take the forms *Rows parsed into commands at the boundary* records.
- **A valid line** composes the fields. **A file layout** composes the columns in any order, an
  extra column, a byte-order mark, line ends and blank lines.
- **Flaws:** a type that names no command, an id outside its type, each reason an amount is
  refused, an extra field, a short line, a byte that is not UTF-8. **A malformed line** composes a
  valid line with one flaw, and the flaw names the kind the line is counted as.

The success properties run on valid lines only:
- client independence;
- the equations, without pair 15;
- `prop_any_spelling_reads_as_the_canonical_one`;
- `prop_any_spelling_of_an_amount_reads_as_its_value`;
- the balance law.

The failure properties run on malformed input only:
- `prop_a_malformed_line_changes_nothing_but_its_count`, which takes over pair 15;
- `prop_text_an_amount_refuses_says_why`;
- the source-error example.

The spelling example and the malformed-spelling example go. Task 3's breaks are run against this
suite.

## Tasks

- [x] **0.** The worktree and the branch; the archive and the link; this plan. Commit
      `docs(tasks): keep the workspace plan and plan the model-free tests`.
- [x] **1.** The record, its index line and the superseded record's Status line. Commit
      `docs(adr): record correctness defined by equations between runs`.
- [x] **2.** The new suite replaces the old, with `make ci` green. These are characterisation
      tests: the behaviour already works. Commit
      `test(it): check the engine by equations between its own runs`.
- [x] **3.** Law evidence, run in a scratch copy, never in the repo. Every recorded break of the
      stage 1 and command-line plans is run against the new suite. Each must fail a test, or fail
      `make lint` for the `.expect` break. If a break escapes, the strategy is fixed or a pair is
      added; a law is never weakened.
- [x] **4.** The docs:
      - the overview's Laws, the architecture, the patterns and the roadmap;
      - `README.md` and `CLAUDE.md`;
      - the two skill references to the test crate.
      Commit `docs(engine): describe the model-free tests`.
- [x] **5.** One `code-reviewer` run on the branch. Its fixes go in new commits.
- [x] **6.** The Review, the Law evidence and any lesson. Commit
      `docs(tasks): close the model-free tests plan`.
- [x] **7.** Prompt history 0008, alone:
      `docs(prompts): add prompt history 0008 (model-free tests)`.
- [x] **8.** Report what is on the branch. Landing is the person's.

## Review

**What the work became.** The test crate went from 3,481 lines to 1,269, and no test computes what
a run should give:
- `run.rs` holds the engine to equations between its runs;
- `domain.rs` holds the laws of money;
- `support/input.rs` holds a generator per field, composed into valid and malformed lines;
- `cli.rs` keeps the binary's fixtures. Its write-failure test now runs everywhere, and a non-UTF-8
  header joins the incomplete-header loop.

**What the framing got wrong.**
- The first suite traded the spelling domain for one example, the one the recorded breaks
  happened to need, and two of its properties checked success and failure together. The person
  asked for one generator per field, composed into a line, and for failure to have generators and
  properties of its own. The deviation above records the change.
- Coverage was measured against the recorded breaks alone. The branch review found five
  behaviours the old generators pinned and no recorded break exercised:
  - a dispute's refused amount;
  - a header without `amount`;
  - a header lacking `type` or `client`;
  - zero reached from below;
  - the order of the amount refusals.

  Each now has generated coverage, shown failing on the reviewer's break, and the lesson of this
  date turns them into rules.
- `main` moved twice while the plan waited for approval: the workspace split landed, and so did the
  conventions for lean plans and one review per branch. The plan was revised for both before any
  work began.

**What was narrower or wider than expected.**
- Wider: the stores' laws are now held through the engine's laws, as *Libraries and an app as
  packages that depend inward* already says of `engine` and `store`. A malformed line among valid
  ones also catches seven of the recorded breaks.
- Narrower: no production code changed, and the `.expect` break is caught by `make lint`, not by a
  test.

**Deliberately not done.**
- `evolve`'s `Opened` arm stays unchecked: `run` never produces `Opened`.
- Overflow stays unchecked, as before.
- A header that names a column twice is left to no test, as before.
- The review's nits are left to the person.

**Law evidence.** Every recorded break from the Law evidence tables of
`tasks/stage-1-proptest-plan.md` and `tasks/stage-1-cli-plan.md`, and the review's five, was run in
a scratch copy, never the repo. The pass ran against the suite at `5f6d1e7`. The two test commits
after it, a named direction and refused amounts drawn from their generator, were run again on the
two breaks they touch.
- **Results:** of the 45 breaks, 43 fail a test on the first run, one fails `make lint`, and one
  is unreachable.
- **Single-law catches:** each break that a single randomized law catches failed 20 of 20 runs of
  that law alone, with fresh seeds.
- **The review's five:** all five pass the suite as it stood before the review.
- **Restores:** each restored file was given a fresh mtime, since cargo reuses a build when a
  restored file is older than it.

The table's test codes:
- `EQ`: `prop_equivalent_commands_leave_the_same_accounts`, with its pair number;
- `IND`: `prop_clients_run_independently`;
- `SPELL`: `prop_any_spelling_reads_as_the_canonical_one`;
- `MAL`: `prop_a_malformed_line_changes_nothing_but_its_count`;
- `INC`: `prop_a_header_lacking_type_client_or_tx_stops_the_run`;
- `SRC`: the source-error example;
- `AMT`, `REF`, `ZERO`: the money laws;
- the `cli` fixtures:
  - `OVW`: the overview example;
  - `SMP`: the sample input;
  - `RM`: every rejection and malformed kind;
  - `HDR`: the incomplete headers;
  - `UNRD`: the unreadable input;
  - `TBLW` and `RPTW`: the table and report write failures.

| Break | Caught by |
|---|---|
| the size check refuses fifteen whole digits | AMT, EQ |
| `from_str` strips a leading `+` | REF, MAL |
| `from_str` reads its first byte by index | REF |
| `checked_sub` negates its difference | ZERO, OVW, SMP, RM, RPTW |
| `checked_sub` negates a zero reached across two scales | ZERO |
| a static flag refuses every other client command | EQ, IND, SPELL, MAL, OVW, SMP, RM, RPTW |
| `Released` reads the deposit with `.expect` | `make lint` |
| `Released` leaves `held` | EQ 1, SMP, RM, RPTW |
| `ChargedBack` leaves `held` | EQ 4, OVW, SMP, RM, RPTW |
| `ChargedBack` keeps the lock | EQ 5, OVW, SMP, RM, RPTW |
| the funds check compares with `total` | EQ 14 |
| a resolve of a settled deposit releases it | EQ 2, SMP |
| the funds check refuses exactly `available` | EQ 13, OVW |
| the lock is checked for deposits only | EQ 6 |
| `open` locks the account | OVW, SMP, RM, RPTW |
| `Opened` makes `open()` of a loaded footprint | none: `run` never produces `Opened` |
| withdrawals skip intake | EQ 10 |
| a refusal at a loaded footprint commits `open()` | EQ 2, OVW, SMP, RM, RPTW |
| malformed rows are skipped uncounted | MAL, RM |
| `run` skips an `Err` like a malformed row | SRC, UNRD, HDR, INC |
| the source is collected whole before the run | SRC |
| rejections kept as one entry per refused row | IND, SMP, RM |
| a static run counter in the first rejection count | IND, SPELL, MAL |
| a footprint cache keyed by the tx id alone, across runs | EQ 11, IND, SPELL, MAL |
| the same cache, reset at each run | EQ 11, IND |
| `commit` keeps a charged-back deposit | EQ 7 |
| `commit` skips the account when it writes a deposit | EQ 1, OVW, SMP, RM, RPTW |
| `snapshot` leaves out the accounts at zero | OVW, RM, RPTW |
| `sight` forgets an id it answers `Repeat` for | EQ 10 |
| fields are not trimmed | SPELL, OVW, SMP, RM, RPTW |
| a row must have the header's length | SPELL, MAL, EQ, RM |
| the extra-field check is removed | MAL, RM, RPTW |
| ids are parsed by `str::parse`, which refuses `0x…` | SPELL |
| the header check leaves out `tx` | HDR, INC |
| the source reads an I/O error as a malformed row | SRC |
| a non-UTF-8 header reads as unreadable | HDR |
| the total column prints `held` | OVW, SMP, RM, RPTW |
| `render_csv` returns without flushing | TBLW |
| a failed table write reads as an overflow | TBLW |
| a failed report exits with status 4 | RPTW |
| a dispute's amount is parsed (review) | SPELL |
| the header must name `amount` (review) | SPELL |
| the header check leaves out `type` (review) | INC |
| `checked_add` negates a zero reached from below (review) | ZERO |
| the 10^15 check runs before the four-decimal one (review) | REF, MAL |
