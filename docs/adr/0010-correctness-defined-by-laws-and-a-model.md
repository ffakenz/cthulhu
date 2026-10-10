# 10. Correctness defined by laws and a model

## Status

Accepted

## Motivation

- Builds on *A pure core* and *State changes only through events*.
- Examples cover the cases someone thought of; costly bugs live in sequences nobody listed, such
  as a dispute after a chargeback or one tx id reused across clients.
- Later stores, runtimes and rewrites must be proven to change nothing.

## Decision

- Correctness is a set of laws over generated inputs: invariants of the state, conservation of
  money, idempotence of repeats, and agreement with a naive model of the engine.
- Generators write raw input text, weighted toward edge cases, and feed it through the production
  source.
- Black-box tests through the binary pin the input and output contract; hand-picked examples are
  fixtures beside them.
- A law that has never failed is suspect: each one is shown to fail once against a deliberately
  broken implementation.
- Governs: `backend/tests/it`.

## Consequences

- The laws are the contract that every refactor, store and runtime must keep.
- The model and the generators are code to maintain, and they grow with every rule.
- A failing law prints its seed and replays exactly.
