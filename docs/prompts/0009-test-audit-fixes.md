# Prompt History 0009 — Test audit fixes

Date: 2026-10-12

The session set out to audit the black-box tests: what each pins and how, its value, any logic that
mirrors production, and any regex. It then resolved the audit's should-fix findings and wrote a
testing strategy into `docs/`, and closed the nits the audit and the branch's review left open.
It started from `main` at `9e4fe53`.

**Outcome:** `fix/test-audit-findings` holds forty-one commits on `9e4fe53`:
- two plans of record: the audit fixes', closed and kept as
  [tasks/test-audit-fixes-plan.md](../../tasks/test-audit-fixes-plan.md), and the nits',
  [tasks/todo.md](../../tasks/todo.md). The model-free tests' closed plan is kept as
  [tasks/model-free-tests-plan.md](../../tasks/model-free-tests-plan.md);
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
  check goes;
- three claims in the branch's own docs, corrected after the hand-off;
- the open nits closed:
  - a failing run law prints its texts and its results;
  - the engine's laws reach several malformed rows, a late quote and the headers that misfit their
    rows;
  - the binary's fixtures pin their error words, and the report's wording lives once;
  - the balance laws draw from the one generator of amounts and state the grouping equality.

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

### 4. Fix three of the nits
> yes

The person approved fixing the three nits that were inaccuracies in the branch's own docs. Each
landed in a commit of its own:
- the record's rejected `i128` in units is exact up to about 1.7 × 10^34, not without a bound;
- a stage on the roadmap owes both output equal to the sequential reference's and its own gate;
- the bound, not the amount cap, keeps a balance exact; the cap keeps every run below the bound.

### 5. Continue with the open threads
> lets continue with those still open threads

A subagent recovered the audit's six nits from the session's transcript; with the review's four,
they were planned one commit each, on the same branch. Two probes in a scratch copy backed the
changes to the balance laws before the plan was written.

### 6. Why `deposits` reuses the client
> why fn deposits does: `&format!("deposit,{client},{client},1.0\n")` and not:
> `&format!("deposit,{client},{tx},1.0\n")`

The client's number doubles as the tx id on purpose: each deposit needs a tx id of its own, and
with one deposit per client, the client's number is one. The plan took one more item, to draw the
tx ids apart so that the line no longer reads as a typo.

### 7. Go
> go

The plan ran to its end:
- the report's wording, the run laws, the generators and the fixtures, a commit each;
- the evidence: each new check caught its break in a scratch copy, and six of those breaks had
  escaped every test before;
- the docs, and the convention on a closed plan's late ticks.

A second review, of this plan's commits only, found no blocker. Its two should-fixes landed in new
commits, and its one nit is listed for the person in the plan's Review.

### 8. Close the session
> ok now lets close up this session: save history, push branch to remote + merge squash commit to
> main, then push main

Recorded the session as this entry and committed it, ahead of the landing the prompt asks for: the
branch pushed with its every commit, then one squash commit on `main`, pushed in turn.
