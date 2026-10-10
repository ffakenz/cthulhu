# 8. Memory bounded by state, not input

## Status

Accepted

## Motivation

- Builds on *Sources, sinks and projections* and *Stores defined by their laws*.
- An input can be far longer than memory.
- What must be remembered is the state: the accounts, the deposits that can still change and the
  tx ids already used. It grows with distinct clients and ids, not with rows.

## Decision

- The source yields one row at a time; nothing collects the input.
- Everything besides the state has a bound that does not depend on input length: the report keeps
  counts by kind, and queues between stages hold a fixed number of batches.
- The state keeps only what a later command can use: a deposit leaves it once no command can
  change it.
- Governs: `CsvSource`, `run`, `Report`.

## Consequences

- Memory follows the number of distinct clients, live deposits and used ids, never the number of
  rows.
- Diagnostics are counts, not one line per row.
- When the state itself outgrows memory, the answer is a store that keeps it on disk
  (*Stores defined by their laws*), not a change to the pipeline.
