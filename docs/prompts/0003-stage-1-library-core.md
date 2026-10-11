# Prompt History 0003 — Stage 1 library core

Date: 2026-10-10

The session set out to build the library core of stage 1 from the approved plan, inline and
without re-planning: the domain, the core, the sequential runtime and the in-memory stores, one
commit each, with every law shown failing once against a deliberate break. It started from the
amended records commit on `feat/stage-1-core`, with the plan, its four tickets and the planning
session's prompt history still uncommitted.

**Outcome:** four commits on `feat/stage-1-core`: the domain (ids, exact four-decimal money,
accounts, deposits, commands and events, and a dependency gate that reads the build graph), the
pure decider, the sequential runtime over the store ports, and the in-memory stores, held by 25
laws and the worked example, 26 tests in all. A whole-branch review on the most capable model
found one important gap, fixed and folded into the domain commit; the plan's Review,
[tasks/todo.md](../../tasks/todo.md), records the rulings and the deferred findings. The planning
session's prompt history and this one land as the branch's last two commits. The branch stays
local: landing it is the person's to do.

---

### 1. Build the library core inline

> Execute the stage 1 library core plan inline: use superpowers:executing-plans with
> tasks/todo.md as the plan of record. The plan is approved and inline is the method I chose. Do
> not brainstorm, re-plan or ask for approval again; rule on any gap as the skill says.
> [...]
> - TDD drives every piece: write each test or law first and watch it fail for the right reason.
>   By my decision, red and green land together in the task's one commit.
> - Show every law failing once against its ticket's deliberate break. Put the failure line in
>   the ticket's Law evidence table, and undo the break by editing. [...]
> [...]
> - Run the final whole-branch review on the most capable model. [...]
> - Then follow Closing: [...] write docs/prompts/0003-stage-1-library-core.md with the
>   prompt-history skill and commit it alone.
> [...]

The four tasks landed in order, each through `make ci`, a code review of its staged diff and the
forbidden-name scans, with every law's deliberate break caught and recorded in its ticket. The
final review's one important finding, a gap in the balance law's generator, which brought a
balance back to zero only at the scale it was added at, was fixed test-first; the fixes and the
plan's Review were folded into their commits, and this entry was written.

### 2. A standard property-testing library

> question: isn't there any standard testing library, similarly to quickcheck that we can
> leverage here instead of reinventing the wheel?
>
> ref rust projects to learn from: [...]

The reply surveyed what the reference projects use, and compared the std-only harness with
`proptest`, `quickcheck` and three other crates by their cost in the build graph, shrinking and
seed replay. It recommended keeping the harness for stage 1, and named what `proptest` would need:
a record admitting it as a dev-dependency, and a gate keeping it out of production.

### 3. Proptest after stage 1

> agree, lets finish stage 1 and then lets follow up with simplification with library proptest

Stage 1 continued on the harness. The follow-up, moving the tests onto `proptest` as an admitted
dev-dependency, is deferred in the plan's Review, and the lesson behind it is in
[tasks/lessons.md](../../tasks/lessons.md).
