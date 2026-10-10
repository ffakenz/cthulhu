# 6. Stores defined by their laws

## Status

Accepted

## Motivation

- Builds on *Effects behind ports* and *State changes only through events*.
- The engine's state (accounts, the deposits that can still change, the tx ids already used)
  lives in memory now, and later on disk or split across partitions.
- An interface shaped after its first implementation breaks the second.

## Decision

- State is reached only through store ports, each defined by laws every implementation keeps:
  - a read returns what the last commit wrote;
  - one command's changes commit together, or not at all;
  - a snapshot reflects every commit before it;
  - a recorded id stays recorded.
- Store ports are synchronous, exchange owned values, and report failure through their own error
  type (`Infallible` in memory).
- For now, every implementation keeps its state in memory.
- Governs: `LedgerStore`, `SeenIds`, `MemoryLedger`, `MemorySeenIds`.

## Consequences

- A disk-backed or partitioned store replaces an in-memory one without touching the core, and
  passes the same laws.
- In-memory stores pay nothing for failure handling.
- A store that must wait on a remote service does not fit; each partition owns local stores
  instead.
