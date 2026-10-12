# Prompt History 0008 — Model-free tests

Date: 2026-10-12

The session set out to simplify the black-box test crate once stage 1 had landed. The tests were
to keep their coverage while losing every part that mirrored the code it tested, with no change to
production code. It started from `main` at `a31b805`. While its plan waited for approval, the
workspace split and the conventions for lean plans landed, so the work began from `main` at
`28ab587`, in a worktree of its own.

**Outcome:** `feat/model-free-tests` holds sixteen commits on `28ab587`, unpushed:
- the plan of record, [tasks/todo.md](../../tasks/todo.md), with the workspace packages' closed
  plan kept as [tasks/workspace-packages-plan.md](../../tasks/workspace-packages-plan.md);
- the record *Correctness defined by equations between runs*, which supersedes *Correctness defined
  by laws and a model*;
- the model-free suite:
  - [app/tests/it/run.rs](../../app/tests/it/run.rs) holds the engine to equations between its
    runs;
  - [app/tests/it/domain.rs](../../app/tests/it/domain.rs) holds the laws of money;
  - [app/tests/it/support/input.rs](../../app/tests/it/support/input.rs) holds a generator per
    field, composed into valid and malformed lines.

  The crate shrinks from 3,481 lines to 1,269;
- the docs and the skills, describing the new tests;
- the review's fixes, in new commits: four blockers, each alone, and six should-fixes;
- the closed plan, with its Law evidence, and a lesson in
  [tasks/lessons.md](../../tasks/lessons.md);
- this entry.

`make ci` passes, with 24 tests. Every recorded deliberate break, and the review's five, was run
against the final suite:
- 43 of the 45 fail a test;
- one fails `make lint`;
- one is unreachable through the runtime.

Landing the branch is the person's.

---

### 1. Simplify the tests
> now that stage-1 has landed, this task is about simplifying tests while minimizing impact on
> coverage. [...] memory.rs using Reference (identical impl to actual used in src) is kind of
> pointless [...] i also see core.rs kind of mirroring the logic too [...] we should find a better
> way to represent the world without mirroring the src code [...] generate.rs its way too big and
> also kind of mirroring logic [...] in model.rs i see we also use a simplified version of Account
> [...] this session is only meant to alter test code [...] nothing should change src/ [...] we
> should end with the least amount of testing lines of code that verify main properties and
> behaviors

A peer session was working in the same checkout. Asked, the person chose:
- a worktree of its own off `main`;
- the record superseded, with the docs synced;
- equations between runs as the oracle.

Two planning agents checked every equation against every state a prefix can leave. They also
mapped the 40 recorded breaks onto the new tests, and the plan was approved.

### 2. Revisit the plan
> note a refactor has landed on top of latest main, thus i suggest you revisit your plan

The plan was revised for `main` at `28ab587`:
- the crate now lives in `app/tests/it`;
- the record takes the next free number;
- the plan is kept in `tasks/todo.md`;
- one review runs per branch.

Once it was approved again, the plan, the record and the model-free suite landed. The recorded
breaks were then run against the suite in a scratch copy.

### 3. The properties kept
> questions: * what is the set of critical (high value) properties that we kept? * are u sure we
> haven't left any important out? RFC: - can we avoid using regexes?

The turn listed the properties kept, and what was covered more narrowly than before. It answered
that no test uses a regular expression: the only one in the test build graph comes with
proptest's `std` feature. Of the 40 recorded breaks, 38 failed a test, one failed the lint gate,
and one is unreachable.

### 4. One generator per field
> in my mind, spellings should generate most domain random possible, but if input is a few set of
> columns, then we need 1 generator per field and a composed generator for the entire row, that's
> it, and tests should run over valid rows bc malformed inputs should be tested differently ->
> using failed/malformed generators so the compose is a failed generator too [...] we should not
> validate failure and success within same property

The generators were rebuilt as one per field, composed into valid lines, with flaws composed into
malformed lines. Every property now checks success or failure, never both. The breaks gave the
same result on the new suite, and the docs were synced.

The branch's one review then found five behaviours the old generators had pinned and no recorded
break exercised. Each got generated coverage and was shown failing on the reviewer's break, and
the re-review of those fixes found no blocker. The rules went to the lesson and to the
`rust-properties` skill.
