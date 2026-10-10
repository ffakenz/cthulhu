---
name: architecture-decisions
description: "Use when a choice about this repo's architecture is being made, reversed or looked up, or when a dependency is proposed — what earns a decision record, how to scaffold one under docs/adr/, the numbering rule, and why a superseded decision is never edited. Covers what is NOT a decision record, the template, the index, the commit that lands a record, and the one thing this repo forbids: citing a record's number from code. Keywords: ADR, architecture decision, decision record, new ADR, docs/adr, template, index, numbering, superseded, deprecated decision, why did we, why do we, rationale, we decided, record the decision, reverse a decision, dependency approval, add a crate, design doc."
---

# Architecture decisions

A decision record is a numbered file under `docs/adr/` saying what was chosen, why, and what it
costs. It exists so the *reasoning* survives the people: a commit message explains one change, a
record explains a constraint that shapes many.

## What earns one

An architecturally significant decision: one that constrains later work, is expensive to reverse,
or that someone will otherwise relitigate. The approved dependency set is one — this repo builds
on the standard library plus the crates its decision records admit, so admitting any crate takes
a person's sign-off and a record of its own ([[review-and-planning]]). A layering rule is one. A
wire format is one.

**Tooling is not.** A formatter choice, a CI runner, a `make` target — those live in the file that
configures them, the README or `CLAUDE.md`. If reversing it costs an afternoon, it is not a
decision record.

**A plan is not.** Plans describe intended work and go stale the moment the work lands. A record
describes a constraint that outlives it.

## The mechanics

- Four digits, **sequential with no gaps**, named for the decision:

  ```text
  docs/adr/NNNN-kebab-case-title.md
  ```

- **A number is never reused**, including for a record that was withdrawn before it was accepted.
- One decision per record. Two decisions in one file cannot be superseded independently.
- Body, in order: **Status · Motivation · Decision · Consequences**. Status is one of `Draft`,
  `Accepted` or `Superseded by ADR M`. Consequences includes what gets worse — a record with only
  upsides was written to justify something already done.
- An index under `## Index` in `docs/adr/README.md`, in number order.

## Scaffolding a record

1. Find the highest existing `docs/adr/NNNN-*.md` and take the next number, zero-padded to four
   digits; the first record is `0001`. No gaps, and a number once assigned is never reused.
2. Copy `docs/adr/template.md` and set its heading to `# N. Title`: the number unpadded, then the
   decision's title.
3. Name the file `docs/adr/NNNN-kebab-case-title.md`.
4. Status starts as `Draft` and becomes `Accepted` when a person signs it off. Fill Motivation,
   Decision and Consequences; Decision names the types, functions and files the record governs.
5. Add the record under `## Index` in `docs/adr/README.md`, in number order. The first entry
   replaces `*(none yet)*`:

   ```text
   - [N. Title](NNNN-kebab-case-title.md)
   ```

6. Commit the record and its index line, and nothing else:

   ```text
   docs(adr): record <title>

   > <the business or technical impact of the decision>
   ```

For example, a first record titled "Record architecture decisions" is the file
`docs/adr/0001-record-architecture-decisions.md`, headed `# 1. Record architecture decisions`,
indexed as `- [1. Record architecture decisions](0001-record-architecture-decisions.md)` and
committed as `docs(adr): record architecture decisions`.

## A reversal supersedes; it never edits

Set the old record's Status to `Superseded by ADR M`, where M is the new record's number, and
leave every other word of it intact. The new record names, by title, what it replaces and why the
original reasoning stopped holding.

Editing a record in place destroys the only evidence of what was believed at the time, which is
the thing that makes the record worth keeping. Someone re-proposing the original idea needs to
read why it was chosen *and* why it was dropped.

Deferring is also a decision: recording that something is deliberately out of scope, with the
shape it would take if it came back, is what stops it being relitigated every quarter.

## Never cite the number from code

This is a standing rule of this repo, and no record overrides it:

> `CLAUDE.md` § Comments — never reference a plan document, an ADR filename, a task number or
> "decision #N" in code or in a tracked document. Link a type or a function instead.

So a `// see ADR N` comment is out, and so is "per ADR N" in the README or in a doc comment. The
reference is brittle in the direction that matters: the code moves, the number stays, and nobody
notices.

Only the decision logs write a number: their indexes, and a superseded record's `Superseded by`
line. Everywhere else, prompt histories included, a decision is named by its
title ([[prompt-history]]).

The record points the other way — **it** names the types, functions and files it governs, so the
link is maintained in the one place a reader is already looking when they need it. When a review
needs to invoke a decision, quote the constraint, not the filename.

## Reading them

Read the records that touch what you are about to change, before changing it. A decision you did
not know about is the most common source of a plausible change that gets rejected.

Related: [[review-and-planning]] · [[functional-architecture]] · [[prompt-history]]
