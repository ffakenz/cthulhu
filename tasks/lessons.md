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
