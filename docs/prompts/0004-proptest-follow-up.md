# Prompt History 0004 — Proptest follow-up

Date: 2026-10-11

The session continued after the library core. It set out to rebase `feat/stage-1-core` on the
latest `main`, then to move the property tests from the hand-written harness onto `proptest`, as
a new commit on top of the branch rather than folded into an earlier one. It started from the
library core's four commits and the two prompt histories after them, on top of the amended records
commit.

**Outcome:** the branch now descends from `main`: the amendment folded into the records commit
became a commit of its own, `docs(engine): record the domain model in the records' language`, so
landing is a fast-forward. Two commits follow the prompt histories. The first records *csv,
serde, rust_decimal and proptest as dependencies*, which supersedes *csv, serde and rust_decimal
as dependencies* and admits `proptest` 1.12, with the 13 more crates it builds on, for tests only.
The second moves the 25 laws onto `proptest` strategies run by a thin `support::for_all`, gives
`make deps` a check that keeps test-only crates out of the program's build graph, rewrites the
`rust-properties` skill, and brings `CLAUDE.md`, `README.md` and three more skills in line with
them. Every deliberate break of stage 1 still fails, now with a shrunk counterexample, as the plan
in [tasks/stage-1-proptest-plan.md](../../tasks/stage-1-proptest-plan.md) records. The branch stays local: landing it is the
person's to do.

---

### 1. Rebase, then proptest in a new commit

> lets first rebase current branch on top of latest main
> then yes, lets follow up with tests onto proptest incrementally in a new commit not folded into
> any other

The rebase replayed the branch onto `main` with `make ci` green after each commit and the final
tree unchanged. Research on `proptest`'s versions, features, seeding and failure persistence, and an
inventory of every place that names the harness or the dependency set, fed a plan in
[tasks/stage-1-proptest-plan.md](../../tasks/stage-1-proptest-plan.md): `proptest` 1.12.0 for tests only, a record superseding the
dependency record and landing alone first, the runner kept as a thin wrapper, failures saved and
replayed, and the migration as one commit. The plan went to the person for sign-off.

### 2. Sign-off

> approved

The record landed alone, after a review whose findings were all taken. The migration followed: the
gate, the strategies, the laws and the docs; a code review and a final whole-branch review found
nothing critical, and their should-fixes and minor points were fixed. On the code as committed, all
29 deliberate breaks of the 25 laws were re-run, each caught with a shrunk counterexample, and
failure persistence was shown once.
