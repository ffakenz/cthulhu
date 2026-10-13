# Prompt History 0009 — Test audit fixes

Date: 2026-10-12

The session set out to audit the black-box tests: what each pins and how, its value, any logic that
mirrors production, and any regex. It then resolved the audit's should-fix findings and wrote a
testing strategy into `docs/`. It started from `main` at `9e4fe53`.

**Outcome:** `fix/test-audit-findings` holds sixteen commits on `9e4fe53`, unpushed:
- the plan of record, [tasks/todo.md](../../tasks/todo.md), with the model-free tests' closed plan
  kept as [tasks/model-free-tests-plan.md](../../tasks/model-free-tests-plan.md);
- the record *Exact decimal amounts and balances with four places*, which supersedes *Exact decimal
  amounts with four places*. A balance's arithmetic refuses a result past ±(2^96 − 1) × 10^−4, so a
  balance never rounds silently and printing one never panics. Three laws and an example pin the
  bound;
- `Display for Amount` deleted, since only a test called it; the amount round trip prints through
  `Balance`;
- four test gaps closed:
  - the reason a command gets when two checks fail;
  - a misspelled header column;
  - 10^15 as an edge of the amounts too large;
  - a resolved deposit disputed again;
- [docs/testing.md](../../docs/testing.md): the levels of tests, how they relate, and where a new
  check goes.

---

### 1. Audit the tests
> the goal for this session is to:
> - summarize each it test
> - explain what properties and tests are captured
> - explain why the captured tests are high valuable
> - exlain the mechanics of each test
> - score each test by value
> - find if there is anylogic within test that mirrors src/ (duplicate logic)
> - make sure no regexes are used either

The audit was read-only. It found:
- no test logic that mirrors production;
- no regex, the ones proptest hides included;
- 37 hand-made breaks, of which the suite caught 33, twenty of them by one test only.

It scored each test and graded the findings. Among the should-fixes were four test gaps, production
code that only a test called, and a balance whose printing panicked past 10^27.

### 2. Resolve the findings, and write a testing strategy
> yes we should resolve the should-fix items on a branch.
>
> at the same time -> look at [...] it explains the hierarchy of tests used
>
> i wonder if there is anything we could take away from this and similarly add our testing
> strategy to docs/

An external page on testing strategy was read. Its idea of two links between levels of tests, one
level overlapping another and one relying on another, was taken into the strategy. A plan for the
fixes and the strategy was written for approval.

### 3. Go
> go

The plan ran to its end:
- the record, then the red and the green for the bound;
- the refactor, the four gaps and the docs;
- the evidence: each new check was shown to catch its break in a scratch copy.

The branch's one review found no blocker, and its three should-fixes landed in new commits. One of
them closed a gap the evidence had found: a bound that clamped instead of refusing passed every
law. The review's nits are listed for the person in the plan's Review.
