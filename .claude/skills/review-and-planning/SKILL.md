---
name: review-and-planning
description: "Use when resolving PR review comments, reviewing a diff, planning a multi-step change, proposing a dependency, or writing a test that is supposed to fail first. Covers reply etiquette, resolution ordering, where rejected designs are recorded, the legitimate TDD carve-outs, making a red fail for the right reason, proving a rewrite equivalent, how a finding is reported, and the dependency gate. Keywords: PR review, review comment, resolve, reply, review a diff, finding, severity, blocker, should-fix, nit, file:line, plan, rejected design, red green, TDD, characterisation test, rewrite, refactor, equivalence, differential property, dependency, new crate, add a crate, cargo add, Cargo.toml, Cargo.lock, make deps, third party."
---

# Review and planning

A multi-step change is planned under `tasks/`: the plan of record in `todo.md`, each ticket's
working state in its own file (`tasks/README.md` says what each holds). These rules hold for every
plan and every review.

## Replies are drafted, never posted

Draft replies to review comments in a working document; the ticket's file under `tasks/` is the
usual place. **Never post them on your own initiative.** Posting is the person's call — the draft
is the deliverable.

Do not name that document in code, in a commit message or in any document outside `tasks/` and
`docs/prompts/` either (`CLAUDE.md` § Comments, § Knowledge base).

## Order resolutions by blast radius, not by comment order

Comments arrive in file order; that is not the order to fix them in.

- Sequence by blast radius — **widest first**. Land the widest-reaching change so the
  narrower ones build on its final shape.
- "Widest" is *reach*, not effort. A one-line change to a shared type outranks a long
  change confined to one file. Do not read this as "save the big one for last".
- Comments touching the same expression or the same type **land together, or the smaller one
  second** — otherwise the second commit rewrites the first.
- One comment at a time, each resolution its own self-contained commit, and wait for approval
  before the next.

## Record rejected designs

When a design is tried and abandoned, write down what it was and why it failed — in the ticket's
file under `tasks/`, or in the PR. Unrecorded, the next person re-derives it, or worse, re-ships
it. If the rejection constrains later work, it is a decision: record it
([[architecture-decisions]]).

## The two legitimate carve-outs from red/green

TDD is the default: a failing test first, then the minimum implementation. Two situations are
genuine carve-outs, and faking a red in either is worse than naming the carve-out:

1. **The behaviour already works.** Pin it as a *characterisation test* and say so — do not
   break the code to manufacture a red.
2. **There is no prior behaviour to fail against** (a new guard, a new startup check). Land the
   guard and its test together, and say that is what you did.

## Make the red fail for the right reason

A test that fails for the wrong reason proves nothing.

- **Size the fixture to the bug.** If the bug is about volume, the fixture needs volume: a small
  one passes against the broken code. Otherwise keep it as small as the bug allows.
- **Assert on the output.** When a change adds a field to what the program writes, assert on the
  written output: the test then compiles against the old struct and fails on the missing field,
  which is the failure you want. A test that does not compile fails on a missing name, not on the
  missing behaviour.
- Before writing a guard test, name the wrong implementation it is supposed to reject and
  check by hand that it fails against it. If you cannot name that implementation, the test
  guards nothing.

## Prove a rewrite by equivalence, not by speed

A faster function that returns a different answer is not an optimisation. Copy the old
implementation into the test crate as the reference — production code never keeps a version only
a test calls — and assert one differential property, `old(x) == new(x)` for every generated `x`,
with the generator biased toward the edges where the two could disagree ([[rust-properties]]).
Record in the ticket how many cases ran. Speed alone is not evidence of correctness.

## A finding has a severity and a location

Report every finding as **blocker / should-fix / nit**, each with `file:line`. A review that
grades nothing forces the reader to re-derive the priority you already knew. The `code-reviewer`
agent reports in this shape.

**Recommend; do not rewrite.** A review that arrives as a patch has skipped the step where the
author gets to disagree — and the author is the one who has to maintain it.

## A new dependency is a blocker, not a detail

This repo builds on the standard library alone. Never add a crate on your own initiative — no
`cargo add`, no new line in any dependency table of a `Cargo.toml` (`[dependencies]`,
`[dev-dependencies]`, `[build-dependencies]`, `[workspace.dependencies]`). Propose it with the
rationale **and the alternatives considered** — writing it on the standard library is always one
— then wait for a person to sign off. The sign-off is a decision record
([[architecture-decisions]]). Fewer dependencies is fewer things to prove correct, patch, and
audit.

`make deps` is the proof: it fails while `Cargo.lock` resolves any package from outside the
workspace, and `.claude/settings.json` asks before `cargo add` or `cargo install` runs. An
approved crate is admitted by changing that check to allow exactly the approved set, never by
dropping it.

Once a crate is approved, `make deps` grows two checks, so the rule keeps holding without a
reviewer:

- **A test-only dependency must be unreachable from production.** Prove it from the graph the
  build reports — `cargo tree --edges no-dev` must not list it — not by reading manifests.
- **Check the source and `Cargo.lock`, not a compiled binary.** Compilation inlines, mangles and
  can strip the names a search would look for, so a scan of the binary passes for the wrong
  reason.

Related: [[architecture-decisions]] · [[rust-properties]] · [[functional-architecture]]
