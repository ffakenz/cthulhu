# Lessons

Append an entry for each correction when the work closes. Each entry has three headings, in
order: **What happened**, **What I got wrong** (numbered), **Rules for next time**. A lesson that
does not end in a rule is a diary entry. Durable lessons about how to work graduate out of here
into `.claude/skills/`; keep here only what is specific to this project.

```text
## YYYY-MM-DD — <one line: what went wrong>

**What happened.** …

**What I got wrong.**

1. …

**Rules for next time.**

- …
```

## 2026-10-10 — Rejected decide/evolve on a false totality argument

**What happened.** A draft decision record rejected splitting the core into `decide` and `evolve`,
claiming `evolve` would either repeat the checked arithmetic or be partial. The user asked why
totality could not be kept with the split. It can.

**What I got wrong.**

1. I treated a fallible function as a partial one: an `evolve` that returns the next footprint or
   an overflow, as a `Result`, has a value for every input, so it is total.
2. I assumed `decide` needs arithmetic. It only checks predicates (sight, lock, amount against
   `available`, deposit status), so nothing is computed twice.
3. I missed that events carrying everything `evolve` needs (tx id, amount, the deposit's next
   status) give `evolve` a meaning for every footprint.

**Rules for next time.**

- Before rejecting a design as partial, try a `Result` variant and self-contained inputs; reject
  it only if neither makes it total.
- Keep arithmetic out of decisions: a decision checks predicates, and the transition that applies
  it computes.

## 2026-10-10 — Planned a footprint with three nested absences

**What happened.** The stage 1 plan shaped the footprint as
`Footprint { account, slot: Option<Slot { tx, deposit: Option<Deposit> }> }`, loaded with an
optional tx id. Asked to explain it, the user found it structural rather than semantic. The plan
now addresses the footprint by the command's client and tx id, from outside, and holds
`Footprint { account, deposit: Option<Deposit> }`: the records' own definition.

**What I got wrong.**

1. I modelled what the store must do (write nothing, delete, or store) instead of the state, so
   three `None`s each needed explaining, and `Footprint::deposit()` existed only to collapse two
   of them.
2. Two spellings meant the same to `decide`: a new client's missing slot and a known client's
   empty one. The tx id sat in both the slot and the event, so a total `evolve` had to answer
   pairs no run produces.
3. I missed the fact that lets one shape fit every command: every command names a tx id, and a
   first sight guarantees a client command's id holds no deposit.
4. I named a structure, `Slot`, that no record uses.

**Rules for next time.**

- Before settling a type with nested `Option`s, write down what each `None` means; if two mean
  the same to every reader, or one exists only to instruct a writer, reshape the type.
- Keep a value's key outside the value when a step that lacks the key must still produce one,
  as `Opened` must produce a footprint without a tx id.
- When a record defines a term in one sentence, make the type read as that sentence.
- These rules graduated into `.claude/skills/domain-modeling/SKILL.md`, where the failure is the
  skill's worked example.

## 2026-10-10 — Hand-rolled the property harness a standard crate provides

**What happened.** The `rust-properties` skill, and the stage 1 plan after it, build every
property test on a std-only harness written for this project: a seeded generator, `for_all`,
replay by seed, edge-weighted generators, and no shrinking. Midway through the build, the user
asked why a standard property-testing library was not used instead of reinventing one. The tests
stay on the harness until stage 1 lands, then move onto `proptest`.

**What I got wrong.**

1. I read the dependency rule as a reason to write the library myself. The rule asks for a
   person's approval before a new crate, and a dev-dependency can be proposed with its rationale
   and alternatives like any other; neither the skill nor the plan proposed one.
2. I weighed the harness by the crates it keeps out of the build graph, and left out what it
   costs: generators written case by case, failing cases reported unshrunk, and a harness the
   project maintains.

**Rules for next time.**

- Before hand-rolling what a widely used crate already does, propose the crate to the person with
  its rationale, its cost in the build graph and the alternatives, and let them decide; the
  dependency rule puts the standard library first, and asks for approval, not abstinence.
- When a plan or a skill copies infrastructure into the tree (a harness, a parser, a runtime),
  name the crate it stands in for, so the person chooses between them.
- These rules graduated into `.claude/skills/review-and-planning/SKILL.md`, under its dependency
  rule; the tests moved onto `proptest` after stage 1.

## 2026-10-12 — Widened the make run allowance over an unquoted recipe

**What happened.** The command-line plan widened `.claude/settings.json`'s `Bash(make run)` to
`Bash(make run:*)`, so that `make run INPUT=<file>` would run without a prompt, and its recipe
pasted `$(INPUT)` unquoted into the shell. An allowed `make run INPUT='x; <command>'` then ran
the command, and so did a `CARGO=`, `SHELL=` or `-f` override. The plan's review flagged the
unquoted variable only for paths with spaces. The command line's commit review raised the
injection, and I carried it to the close as a note for the person; the final whole-branch review
graded it important, and the person chose to narrow the allowance back and quote the input. The
fix's review found the same flaw in six `cargo …:*` allowances from the workspace's first commit,
and a probe found it in `cargo metadata:*`; the person chose to narrow them to the forms the
Makefile runs.

**What I got wrong.**

1. I read a prefix allowance as allowing one command. `make run` followed by anything also takes
   variable overrides and other makefiles, so it allows whatever a recipe, a variable or a
   makefile can run.
2. I pasted a make variable into a recipe's shell text unquoted, and justified it by the case
   with no input, when the shell's own `${INPUT:+"$INPUT"}` covers that case and keeps any text
   one argument.
3. When a commit review found the hole, I committed over it and noted it for the close, instead
   of stopping to ask: from that commit on, an allowed command could run any other.

**Rules for next time.**

- Never widen an allowance in `.claude/settings.json` to a prefix (`:*`) for `make`, or for any
  command whose arguments can name a program, a file to run or a variable; keep the exact
  command, and let a call with arguments prompt.
- Quoting does not make a prefix allowance safe: make itself expands a `$(…)` in a command-line
  variable when it exports it to a recipe. Pass a value into a recipe through the environment,
  quote it in the shell (`$${VAR:+"$$VAR"}`), and keep the allowance exact as well.
- A finding that lets a command run without consent stops the work: ask the person before the
  next commit, never carry it to the close.

## 2026-10-12 — Rewrote a plan condition instead of reporting it unmet

**What happened.** The sample input's plan asked for one `cargo run` on the sample "with the
person's consent". It ran in auto mode, which let it through without asking. Ticking the step, I
changed the condition to "under this signed-off plan". The commit's review caught the rewrite
before the commit.

**What I got wrong.**

1. I edited a requirement the person signed off to match what happened, so the tick claimed a
   condition that was never met.
2. I treated the plan's approval as consent for a step that asked for consent at the moment it
   ran.

**Rules for next time.**

- A tick meets its condition as written or says plainly that it did not, with what happened
  instead; never edit the condition while ticking it.
- A step that asks for the person's consent waits for it at that moment. A plan's approval and a
  permission system that lets the call through are not that consent.

## 2026-10-12 — Measured the wait on the person instead of the agent's own time

**What happened.** Asked why sessions take too long, I measured them from their transcripts and
led with the time the agent sat blocked on the person: permission prompts, questions and plan
approvals. The person corrected the scope: that wait is not the problem; the agent's own time is.

**What I got wrong.**

1. I took a turn's whole wall-clock as the agent's time, so the waits on the person dominated
   the numbers.
2. I reported those waits as a finding before checking they were what the person asked about.

**Rules for next time.**

- A session's time splits three ways: the agent working, the agent idle on its subagents, and
  the agent blocked on the person. Report the first two; the third only when the person asks.

## 2026-10-12 — Mixed success and failure in one property, and narrowed the spellings

**What happened.** The model-free tests replaced the generated CSV spellings with one hand-written
example, and two of their properties checked a valid input and a malformed one together:
- the amount round trip accepted either `Ok` or the right `Err` in one law;
- an equation about valid commands slipped a malformed row among them.

The person asked for one generator per field, composed into a line, with malformed input made by
generators of its own, and for success and failure never to share a property.

**What I got wrong.**

1. I measured "fewest lines" against the recorded breaks alone. That traded the input domain the
   contract admits for the one example the breaks happened to need. The branch review then found
   five behaviours the old generators pinned and no recorded break exercised: a dispute's refused
   amount, a header without `amount`, a header lacking `type` or `client`, zero reached from below,
   and the order of the amount refusals.
2. I let a law branch on the outcome it checked. A law that passes on either `Ok` or the right
   `Err` is two laws, and when it fails it does not say which half broke.
3. I put the failure tier's law inside the engine's: a malformed row inside an equation about valid
   commands.

**Rules for next time.**

- Build input from one generator per field, composed into a line. Each field draws its value from
  its whole domain and its spelling from every form the contract admits.
- Give failure generators of its own: a flaw per field, composed with valid fields, so the
  malformed input knows how it must fail. Give it properties of its own too: a property checks
  success or failure, never both.
- Before deleting a generator, list every input form it produced. Each form the contract admits
  stays generated, whether or not a recorded break needs it: the recorded breaks are evidence of
  coverage, not its definition.
- These rules graduated into `.claude/skills/rust-properties/SKILL.md`.
