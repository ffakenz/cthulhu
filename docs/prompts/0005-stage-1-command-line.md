# Prompt History 0005 — Stage 1 command line

Date: 2026-10-12

The session set out to plan and build the rest of stage 1 up to a working command line: the CSV
adapters, the composition root in `main.rs` and tests through the binary, so that `make run` turns
an input file into the accounts table. It started from the library core, held by 25 laws on
`proptest` on `feat/stage-1-core`, before that branch had reached `main`.

**Outcome:** `feat/stage-1-cli`, cut from `main` once the library core landed there, holds the
command line. Its first commit is the plan of record, [tasks/stage-1-cli-plan.md](../../tasks/stage-1-cli-plan.md), with the
stage 1 and proptest plans kept beside it as
[tasks/stage-1-library-plan.md](../../tasks/stage-1-library-plan.md) and
[tasks/stage-1-proptest-plan.md](../../tasks/stage-1-proptest-plan.md). Three red and green pairs
follow, with the runtime's laws moved onto CSV text between the first two.
[app/src/csv_io.rs](../../app/src/csv_io.rs) reads rows from CSV text into commands and
writes the accounts table as CSV, and [app/src/main.rs](../../app/src/main.rs) runs a file
through them, with the report's lines from [app/src/text_io.rs](../../app/src/text_io.rs) on
stderr and an exit status for every failure. All but one of the runtime's laws now reach the engine
through the CSV source, fed text in every spelling the strategies write; the one left states `run`'s
contract with any failing source. Fixtures drive the binary, the worked example read from
[docs/overview.md](../../docs/overview.md) itself, and `make ci` passes with 38 tests and a
Linux-only fixture beside them. No record changed: the exit statuses and the report's lines are
details of *The command-line contract*, documented in the overview and the README. The final
review's one important finding is fixed in the command line's own commit: `make run` passes its
input as one argument, and only the bare command runs without a prompt. A further commit narrows the
cargo allowances `main` already held, which let an allowed call run another program, to the forms
the Makefile runs. The branch stays local: landing it is the person's to do.

---

### 1. Plan the command line

> One thing to know: tasks/todo.md is replaced for each new piece of work, so the earlier prompt
> histories' links to it point at whatever plan is current. The stage 1 and proptest plans survive
> only in git history [...]. If you'd rather keep them as separate ticket files, that's your call.
>
> Plan the rest of stage 1 up to a working command line: the CSV adapters, main.rs and end-to-end
> tests through the binary, so that `make run` turns an input file into the accounts table. Write
> the plan and check in with me before implementing anything.
>
> [...] check that local main has that work: `git merge-base --is-ancestor 779a604 main`, or, if
> the branch landed by squash or rebase, an empty `git diff --stat 779a604 main`. If neither holds,
> stop and ask me; I land branches and update main myself.
> [...]
> - Put every open decision in the plan with a recommendation. [...] A gap or a contradiction in
>   the records is a question for me, not a silent ruling [...]
> - TDD for every piece. Every law is shown failing once against a deliberate break [...]
> - At the end:
>   - a final whole-branch review on the most capable model;
>   - prompt history 0005, written with the prompt-history skill and committed alone;
>   - superpowers:finishing-a-development-branch, keeping the branch as it is.
> [...]
> - The repo is public. Never name [...]. Never quote, restate or paraphrase the brief in any
>   tracked file or commit message. [...]

`main` did not hold the library core yet, so the session stopped and asked before going on.

### 2. Land the core, plan meanwhile

> I'll land it; plan meanwhile (Recommended)

Chosen among the options offered when the check failed. Planning went on read-only against the
core's last commit while the person landed it on `main` as one squash commit, and an adversarial
review of the draft found one blocker and seven should-fixes, all taken. The plan went to the
person with ten decisions for sign-off and seven questions on gaps in the records, each with a
recommendation, among them red and green commits apart, the closed plans kept as files, an exit
status per failing step, the report's lines in a sink of their own, and `make run INPUT=<file>`.

### 3. Sign-off

> [the plan, approved as written]

The plan landed as the branch's first commit, with the stage 1 and proptest plans restored as
files and the 31 references that named them as `tasks/todo.md` repointed. The session then asked
how to run it.

### 4. Inline

> Inline (Recommended)

The plan ran inline, task by task: the CSV source, the runtime's laws moved onto CSV text through
it, the CSV sink, and the command line, each red commit failing only on its new tests and each
green commit passing `make ci`. Every commit passed the code-reviewer agent with its blockers and
should-fixes taken, and every new or rewired law, with the two tests review added, failed against
a deliberate break, 18 in all, as the plan's Law evidence records. A final whole-branch review on
the most capable model found one important issue.

### 5. Narrow and quote `make run`

> Narrow + quote (Recommended)

Chosen among the fixes offered for that issue: an allowed `make run` with arguments could run any
command through its input or a variable override. The allowance is the exact `make run` again and
the recipe passes the input as one quoted argument, so the command that ran before the fix now
reaches the binary as a path it cannot open. The plan's Review records what the plan got wrong and
the minor findings left to the person, and the staged fix went to the code-reviewer.

### 6. Resume

> resume

The session had stopped while the code-reviewer checked the staged fix. Its review found no blocker,
but six `cargo …:*` allowances already on `main` could run another program through `--config`, and
under zsh a `~` after `=` now stays a `~`, which the README states. After a lesson in
[tasks/lessons.md](../../tasks/lessons.md) whose new rule stops on such a finding, the cargo
allowances went to the person before the next commit, with a question on where the sink red's plan
sync had landed.

### 7. Narrow the cargo allowances, fold the sync back

> Narrow on this branch
>
> Fold it back (Recommended)

The `make run` fix, its review's fixes and the lesson went into the command line's commit. The sink
red was rebuilt with its plan sync and the later commits replayed onto it, each differing from
before only in the red's hash and a sentence that had turned false.

### 8. Drop `cargo metadata` too

> Drop it too (Recommended)

A probe had found `cargo metadata` running a program through `--config` as well. The cargo
allowances now hold `cargo fmt:*` and the exact forms the Makefile runs, in a commit of their own:
of eight calls that run a program through `--config`, the old list let all through and the new one
none.
