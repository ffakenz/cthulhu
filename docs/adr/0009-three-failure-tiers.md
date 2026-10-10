# 9. Three failure tiers

## Status

Accepted

## Motivation

- Builds on *A pure core* and *Sources, sinks and projections*.
- Input fails in three ways: one row is malformed, one command is refused, or the whole source
  becomes unreadable.
- Treating them alike either stops a long run on one bad row, or publishes balances computed
  from a broken input.

## Decision

- A malformed row is skipped and counted; processing continues.
- A refused command is a rejection, an outcome of the core, not an error; it is counted by reason.
- An unreadable source stops the run: nothing reaches the output, and the process exits with
  failure.
- Each tier has its own type, so no code can mistake one for another.
- Governs: `RowError`, `Reason`, `SourceError`, `RunError`.

## Consequences

- One bad row never stops a run, and a broken input never yields partial balances.
- The report tells how much was skipped or refused, and why.
- A malformed row changes no state, since none of its fields can be trusted.
- A fatal failure shows as a failing exit status, an empty output and a message on stderr.
