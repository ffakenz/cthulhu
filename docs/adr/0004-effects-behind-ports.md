# 4. Effects behind ports

## Status

Accepted

## Motivation

- Builds on *A pure core*.
- Effects differ by runtime: a file now and network connections later; memory now and disk
  later; real input in production and generated input in tests.
- Code that constructs its own effects can only ever run one way.

## Decision

- Each effect the engine uses is a port: a narrow interface declared beside the code that
  consumes it, or a standard-library trait (`Read`, `Write`) where one fits.
- A port's implementations are interchangeable. Only the composition root, `main.rs`, constructs
  them, and it passes them down as values.
- A cross-cutting concern (metrics, retries, batching) wraps an implementation; it never branches
  inside one.
- Governs: `main.rs`; the ports: the source, `LedgerStore`, `SeenIds`, and `Read` and `Write` for
  byte streams.

## Consequences

- A runtime, a test and a benchmark are different wirings of the same code.
- Every port has an implementation in production use; no port exists only for tests.
- Indirection: to learn which implementation runs, a reader follows the port to `main.rs`.
