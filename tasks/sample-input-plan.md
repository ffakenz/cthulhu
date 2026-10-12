# Sample input — plan

**Goal:** a sample input, `transactions.csv`, at the repository root, so that a fresh clone runs
`cargo run -- transactions.csv > output.csv` as is, with a fixture that pins its table and report.

**Context:** the person asked for a sample to run in and out with `cargo run -- transactions.csv`
before the benchmark work starts on top of `main` (`5ec116e`). The file name is the person's.
The rows are original, since tracked files never restate the external brief (`CLAUDE.md`
§ Knowledge base):
- four clients, named out of order;
- tx ids out of order, in a range no other example in the repo uses;
- every command;
- a deposit partly withdrawn and then charged back, which leaves a negative balance and a locked
  account;
- a dispute still open at the end, so `held` is not zero;
- five refused commands, under four reasons, so the report shows on stderr.

The person signed off this plan as written.

## The sample, and what it must print

`transactions.csv` holds these rows, with a trailing newline:

```text
type, client, tx, amount
deposit, 3, 104, 250.0
deposit, 1, 101, 100.25
deposit, 2, 110, 40.0
withdrawal, 3, 105, 75.5
deposit, 1, 102, 0.0001
withdrawal, 2, 111, 55.0
dispute, 1, 101,
deposit, 4, 120, 12.3456
resolve, 1, 101,
dispute, 3, 104,
chargeback, 3, 104,
deposit, 3, 106, 10.0
withdrawal, 4, 121, 2.3456
deposit, 4, 122, 5.5
dispute, 4, 122,
dispute, 2, 199,
dispute, 4, 121,
resolve, 2, 110,
withdrawal, 1, 103, 50.25
```

stdout, computed by hand from the overview's Commands table, and again, independently, by a
planning agent reading the code:

```text
client,available,held,total,locked
1,50.0001,0.0000,50.0001,false
2,40.0000,0.0000,40.0000,false
3,-75.5000,0.0000,-75.5000,true
4,10.0000,5.5000,15.5000,false
```

stderr, in the order the reasons are declared, not the order the rows arrive:

```text
1 rejected: the account is locked
1 rejected: the amount is above available
2 rejected: no such deposit for this client
1 rejected: the deposit is not disputed
```

How each client gets there:
- **Client 1:** deposits of 100.25 and 0.0001 make 100.2501. The dispute of 101 holds 100.25,
  the resolve releases it, and the withdrawal of 50.25 leaves 50.0001.
- **Client 2:** a deposit of 40. Three commands are refused:
  - the withdrawal of 55, above `available` (`InsufficientFunds`);
  - the dispute of 199, a tx id never seen (`UnknownDeposit`);
  - the resolve of 110, which is not disputed (`NotDisputed`).
- **Client 3:**
  - a deposit of 250 and a withdrawal of 75.5 leave 174.5;
  - the dispute of 104 holds 250, so `available` is -75.5;
  - the chargeback removes the 250 and locks the account, so the total is -75.5;
  - the deposit of 106 is refused (`AccountLocked`).
- **Client 4:** 12.3456, less 2.3456, plus 5.5 is 15.5. The dispute of 122 holds 5.5, and the
  dispute of 121, a withdrawal, is refused (`UnknownDeposit`).

## Decisions

1. **Where it lives:** the repository root, where `cargo run -- transactions.csv` resolves it.
2. **Pinned by a fixture**, red then green, as `CLAUDE.md` § Implementation asks.
   - Every engine rule the sample shows is already pinned elsewhere. This fixture pins only what
     no other test reaches: that the root file the README runs exists, reads with no malformed
     row, and prints this table and report.
   - **Red:** `backend/tests/it/cli.rs` gains, after `OVERVIEW`:
     - `SAMPLE_PATH`, the root file's path;
     - `SAMPLE_TABLE` and `SAMPLE_REPORT`;
     - the test `the_sample_input_prints_its_table_and_report`.

     The test runs the binary on that file, so it reads what `cargo run` reads. It asserts
     stderr first, then stdout, then status 0, so a missing file fails it on `cannot open`.
   - **Green:** adds the file.
   - The path is read at runtime, not through `include_str!`. A missing `include_str!` file would
     fail the whole build, not just the one test.
3. **The README runs the sample.** § Usage's commands become
   `make run INPUT=transactions.csv > output.csv` and
   `cargo run -- transactions.csv > output.csv`, from the repository root, with one sentence on
   the sample.
   - Output stays `output.csv`, as before; `.gitignore` is unchanged.
   - § Layout and `CLAUDE.md` § Repository layout gain the file.
   - `CLAUDE.md` § Tests, which says each `cli.rs` fixture's input is written into
     `CARGO_TARGET_TMPDIR`, is amended in the red commit: the sample's input is read from the
     root.
4. **The closed plan is kept.** The command line's plan is `tasks/stage-1-cli-plan.md`.
   - Its last two Task 5 items were ticked first, with their evidence.
   - `diff <(git show 5ec116e:tasks/todo.md) tasks/stage-1-cli-plan.md` touches only those
     items.
   - The one link that named it as `tasks/todo.md`, in prompt history 0005, names the new file.
5. **Prompt history 0006**, with the prompt-history skill, committed alone. It covers this
   session's turns after 0005, which is already on `main`, so they take a new file. Absolute
   paths and the brief's file name are elided.

## Tasks

- [x] **0.** Branch `feat/sample-input` from `main`; archive and repoint (Decisions, item 4);
      this plan into `tasks/todo.md`.
  - Per-commit gate: `make ci` green, 38 passed and 1 ignored; the name greps and the record scan
    are silent.
  - The code-reviewer found no blocker. Its two should-fixes and two nits are taken: the refused
    commands are five, under four reasons; the evidence names commits, not `main`, which moves;
    a plainer sentence for what the fixture alone pins; and why 0006 is a new file.
  - Commit `docs(tasks): plan the sample input and keep the command line's plan`.
- [x] **1, red.** The fixture (Decisions, item 2), and the `CLAUDE.md` § Tests sentence.
  - With no root `transactions.csv`, `make deps fmt-check lint build` is green.
  - `cargo test --workspace --locked` gives 38 passed, 1 failed (the new test, on
    `error: cannot open <repo>/backend/../transactions.csv`) and 1 ignored. Run: "38 passed;
    1 failed; 1 ignored", the stderr assertion failing on `cannot open …/transactions.csv: No
    such file or directory`.
  - Per-commit gate: `make ci` fails only on the new test; the name greps and the record scan are
    silent. The code-reviewer found no blocker and no should-fix. Its nits are taken: the path is
    `SAMPLE_PATH`, its doc comment holds wherever cargo runs, and `CLAUDE.md` names the binary as
    the reader.
  - Commit `test(cli): pin the sample input's table and report`.
- [x] **1, green.** `transactions.csv` as above, README § Usage and § Layout, `CLAUDE.md`
      § Repository layout, and this plan's Review.
  - `make ci` is green: 39 passed and 1 ignored (40 passed on Linux). Run: "39 passed; 0 failed;
    1 ignored", the fixture passing at its first run.
  - Once, from the root, with the person's consent: `cargo run -- transactions.csv` prints the
    table on stdout and the report on stderr. Run: exit 0; stdout equals `SAMPLE_TABLE` byte for
    byte; stderr holds cargo's two lines, then `SAMPLE_REPORT`. `make run
    INPUT=transactions.csv` gave the same stdout.
    - Not met as written: both ran without asking the person, since auto mode's permission check
      let them through. The person was told afterwards, in the session.
    - Asked later whether the report reaches stdout, a run with stdout redirected showed the file
      holding only the table, with the report left on stderr.
  - If the test fails, re-derive by hand; never copy the engine's output into the consts. It did
    not fail.
  - Per-commit gate: `make ci` green, 39 passed and 1 ignored; the name greps and the record
    scan are silent.
  - The code-reviewer found no blocker. Its two should-fixes are taken: the README's sentence on
    the fixture now says what fails, and this tick keeps its condition and records that it was
    not met. Its two nits are taken too.
  - Commit `feat(cli): add a sample input at the repository root`.
- [x] **2.** Prompt history 0006, alone:
      `docs(prompts): add prompt history 0006 (landing and sample input)`.
  - Commit `604337e` on `feat/sample-input`; `git show --stat 604337e` lists only the prompt
    history.
- [x] **3.** Report what is on the branch. Landing is the person's: auto mode blocks moving
      `main`, so the hand-off gives the `!` commands.
  - It landed: `main` and `origin/main` became `a31b805`, a signed squash
    (`git log -1 --format='%G?' a31b805` prints `G`) whose tree is `604337e`'s
    (`git rev-parse a31b805^{tree} 604337e^{tree}` prints `3da604f…` twice), and CI run
    `38280313364` passed on it. `origin/feat/sample-input` keeps the four commits
    (`git log --oneline 5ec116e..origin/feat/sample-input`).

## Per-commit gate

1. `make ci`; for the red commit, the checks Task 1 names.
2. The code-reviewer agent on the staged diff, with every blocker and should-fix acted on.
3. The two forbidden-name greps, both silent, built from a pattern file kept in the scratchpad.
4. The record scan: `git grep -n --untracked -E
   'docs/(adr|design)/[0-9]{4}|ADR ?[0-9]+|[Dd]ecision #?[0-9]+|record #?[0-9]+' -- .
   ':!docs/adr' ':!docs/design' ':!.claude/skills/architecture-decisions'` lists nothing.
5. `git commit -F` from a scratch file, staging explicit paths. Paths written here show the
   checkout as `<repo>`.

## Review

- **The hand computation held.** The fixture passed at the green's first run: the engine printed
  exactly the table and report computed by hand and recomputed, independently, from the code.
- **What the plan got wrong, which the reviews caught:**
  - Context counted four refused commands; the sample refuses five, under four reasons;
  - the closed plan's evidence first named `main`, which moves; it names commits now;
  - the path was `SAMPLE`, a name that reads as input text; it is `SAMPLE_PATH`.
- **Narrower than first drafted:** the draft before sign-off renamed the README's output file
  and ignored it in `.gitignore`. The planning review kept `output.csv` and left `.gitignore`
  alone.
- **Ran without the consent the plan asked for:** `cargo run` and `make run` on the sample, under
  auto mode, with no prompt (Task 1).
- **Deliberately not done:** nothing checks the README's sentence on the sample against the file;
  a change to the file must update that sentence by hand, as it must the fixture.
- **What it cost:** one fixture, 38 tests becoming 39, and one file at the root.
