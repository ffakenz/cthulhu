# 11. Runtimes proven against a sequential reference

## Status

Accepted

## Motivation

- Builds on *State partitioned by client* and *Correctness defined by laws and a model*.
- Parallel and networked runtimes add threads, queues and failure modes, and are where ordering
  bugs live.
- Speed is worth having only if it changes no result.

## Decision

- Besides adapters, the runtime, which schedules commands onto the core and the stores, is the
  only part of the engine that varies; the core, the ports and the laws are shared by every
  runtime.
- For now the runtime is sequential: one partition, one command at a time, in arrival order. It
  is the reference.
- A new runtime lands only when:
  - its output equals the reference's on every generated input, for every configuration it
    accepts (partitions, decoders, batch and block sizes); and
  - a benchmark shows the gain it was built for or, for a runtime that only enables the next,
    a cost of at most 5%.
- Governs: `run`.

## Consequences

- Each runtime is an addition that can be removed without touching a rule.
- Every claim about speed comes with a measurement.
- Until decoding runs in parallel, throughput is bound by reading the input on one thread.
- The benchmarks and their input generator are code to maintain.
