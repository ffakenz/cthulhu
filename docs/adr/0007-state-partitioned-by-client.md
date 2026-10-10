# 7. State partitioned by client

## Status

Accepted

## Motivation

- Builds on *A pure core* and *Stores defined by their laws*.
- Parallel and distributed runtimes need state that splits without locks, and an order that every
  split agrees on.
- Each command concerns one client: its account and, for a dispute, one of its deposits.

## Decision

- All state but the used tx ids belongs to exactly one client. What one command touches, its
  client's account and at most one of its deposits, is the command's *footprint*; the core decides
  from the footprint and, for a client command, intake's `Sight`.
- A client's state has a single writer, which applies that client's commands in arrival order.
- What spans clients, the arrival order and the used tx ids, is settled once, at an *intake* ahead
  of every partition.
- A *partition* is a set of clients with one writer; the sequential runtime is a single partition.
- Governs: the footprints passed to `decide_client` and `decide_dispute`; `SeenIds`, used at
  intake.

## Consequences

- Partitions share nothing and need no locks; merging their views is a union of accounts and a
  sum of counts.
- The client is the unit of ownership and ordering, and later of distribution.
- One busy client cannot be spread across writers.
- Intake is a sequential step in every runtime; a rule that spans clients belongs there or
  nowhere.
