# Test audit fixes — plan

**Goal:** close the should-fix findings of the audit of the black-box tests, and write the testing
strategy down:
- the checks whose breaks escaped every test catch them;
- a balance can no longer round silently or panic when it is printed;
- no production code exists only for a test;
- `docs/testing.md` describes the levels of tests and how they relate.

**Context:** an audit of the 24 tests on `main` at `9e4fe53` ran 37 deliberate breaks: 33 failed a
test and 4 escaped. It also found production code that only a test calls, and a balance whose
printing panics past 10^27. The person asked to resolve the should-fix findings on a branch, and
to take what fits from an external page on testing strategy into a strategy of our own in `docs/`.
The work runs on `fix/test-audit-findings`, from `main` at `9e4fe53`. The person's approval of
this plan accepts the record in decision 1.

## Decisions

1. **Balances bounded where four decimals are exact.** `Balance::checked_add` and `checked_sub`
   return `None` once a result leaves ±(2^96 − 1) × 10^−4, about 7.9 × 10^24: the range where
   `Decimal` holds four places.
   - Today they fail only past about 7.9 × 10^28. Between the two bounds, sums round silently,
     and from 10^27 printing four decimals panics, because rust_decimal formats into 32 bytes.
   - Valid input sums below 4.3 × 10^24, so the binary never gets there; `Balance`'s API does.
   - The record *Exact decimal amounts and balances with four places* supersedes *Exact decimal
     amounts with four places*. It carries the amounts and the output over unchanged.
   - Rejected: a `Display` that never panics, which would still print balances that had rounded;
     integer units, a wider change whose removal of rust_decimal needs a record of its own.
2. **Amounts print only as balances.** The amount round trip prints through `Balance::from`, as
   the table does. `Display for Amount` is deleted: that law was its only caller.
3. **The four test gaps are characterisation tests.** The behaviour they pin already works, so
   none has a red. Instead, each check is shown to catch the break that escaped, in a scratch copy
   and never in the repo.
4. **The testing strategy goes in `docs/testing.md`**, a standing document beside the
   architecture's picture. It takes no new record: it pictures *Correctness defined by equations
   between runs* and *Runtimes proven against a sequential reference*. From the external page it
   takes:
   - **levels**: the binary; the engine through its source; the domain's functions; and, beside
     them, the gate and, ahead, runtime equivalence and benchmarks. For each: what it proves,
     what it runs against, and through which interface;
   - **two links between levels**:
     - a level *overlaps* a lower one when both run the same code without fakes, and the lower one
       covers cases the upper cannot reach;
     - a level *relies on* a lower one when it fakes what the lower one checks for real;
   - **where a new check goes**: most checks are laws on the engine; the domain takes the fine print
     no run reaches; the binary pins only what an equation cannot see;
   - **outside-in for a new stage**: its outermost check, the equivalence law or the benchmark
     gate, is written first.

   It leaves out test-only interfaces in production, an independently written simulator as an
   oracle, and multi-process levels, which have nothing to test yet. The external project is never
   named.

## Files and interfaces

| File | After the change |
|---|---|
| `libs/engine/src/domain/money.rs` | `Balance::checked_add` and `checked_sub` refuse a result outside the exact range; `Display for Amount` deleted; doc comments to match |
| `docs/design/` | the new record, its index line, and the superseded record's Status line |
| `app/tests/it/domain.rs` | three balance laws (Tests); the amount round trip prints through `Balance` |
| `app/tests/it/support/input.rs` | too-large digits draw 10^15 and `u64::MAX` as edges; the incomplete layout misspells a required column as well as drops it |
| `app/tests/it/run.rs` | one more pair |
| `app/tests/it/cli.rs` | two more fixture lines, and the two counts they change |
| `docs/testing.md` | new: the testing strategy |
| `docs/overview.md`, `docs/architecture.md`, `README.md`, `CLAUDE.md` | name the new record, and link the strategy |

## Tests

- **Balance laws** (`domain.rs`): each runs over the balances an amount reaches in up to 64 steps
  (doubled, credited or debited), stopping at the first refused step.
  - `prop_every_balance_prints_with_four_decimals`. Red today: printing panics past 10^27.
  - `prop_moves_add_up_the_same_in_any_grouping`: a reached balance moved by two amounts in turn
    equals the same balance moved by their sum, wherever both are accepted. Red today: past
    7.9 × 10^24 the two differ. Probe: from 33 doublings of the largest amount.
  - `prop_any_amount_doubled_32_times_is_a_balance`: the most that valid input can sum to stays
    a balance. Green today; it keeps the bound from being set too tight.
- **Gaps closed**, each with the break it must catch:

  | Check | Break it must catch |
  |---|---|
  | fixture: a repeated tx id from the locked client counts as "the tx id was used before" | the lock checked before the repeat |
  | fixture: a withdrawal above available by the locked client counts as "the account is locked" | the funds checked before the lock |
  | incomplete layout: a required column misspelled, in another case or with a letter more | header names compared ignoring case |
  | refused amounts: 10^15 and `u64::MAX` as edges, one draw in four | a whole part of 16 digits accepted |
  | pair: deposit·dispute·resolve·dispute ≡ deposit·dispute | a resolve that drops the deposit from history |
  | `prop_any_amount_doubled_32_times_is_a_balance` | the bound set at 10^20 |

## Tasks

Every commit passes `make ci` and the forbidden-name check, except the red at 3, which fails on its
two new laws and on nothing else.

- [x] **0.** The branch; the archive and the link; this plan. Commit
      `docs(tasks): keep the model-free tests plan and plan the audit fixes`.
- [x] **1.** The record, its index line, and the superseded record's Status line. Commit
      `docs(design): record exact decimal amounts and balances with four places`.
- [x] **2.** `prop_any_amount_doubled_32_times_is_a_balance`. Commit
      `test(domain): hold 2^32 of any amount in a balance`.
- [x] **3.** Red: the other two balance laws. Commit
      `test(domain): print and group balances exactly up to their bound`.
- [x] **4.** Green: the bound in `Balance`. Commit
      `fix(domain): refuse a balance past the range where four decimals are exact`.
- [x] **5.** The amount round trip prints through `Balance`; `Display for Amount` is deleted. Commit
      `refactor(domain): print amounts only as balances`.
- [x] **6.** The fixture's two lines. Commit
      `test(cli): pin the reason a command gets when two checks fail`.
- [x] **7.** The misspelled column. Commit
      `test(run): misspell a required column as well as leave it out`.
- [x] **8.** The edges at 10^15 and `u64::MAX`. Commit
      `test(domain): draw 10^15 among the amounts too large`.
- [x] **9.** The pair. Commit `test(run): dispute a resolved deposit again`.
- [ ] **10.** Evidence, in a scratch copy and never in the repo: each break in the table fails its
      check. The results go in the Review.
- [x] **11.** `docs/testing.md`, and the docs that name the record or link the strategy. Commit
      `docs(testing): describe the testing strategy`.
- [ ] **12.** One `code-reviewer` run on the branch; its fixes go in new commits.
- [ ] **13.** The Review and any lesson. Commit `docs(tasks): close the audit fixes plan`.
- [ ] **14.** Prompt history 0009, alone:
      `docs(prompts): add prompt history 0009 (test audit fixes)`.
- [ ] **15.** Report what is on the branch. Landing is the person's.
