# Lessons

Append an entry after any correction. Each entry has three headings, in order: **What
happened**, **What I got wrong** (numbered), **Rules for next time**. A lesson that does not end
in a rule is a diary entry. Durable lessons about how to work graduate out of here into
`.claude/skills/`; keep here only what is specific to this project.

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
