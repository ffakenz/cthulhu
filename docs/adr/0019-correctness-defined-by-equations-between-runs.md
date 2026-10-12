# 19. Correctness defined by equations between runs

## Status

Accepted

## Motivation

- Supersedes *Correctness defined by laws and a model*. Its laws compared the engine with code
  written to agree with it:
  - a naive model of the engine;
  - a reference store copied from the in-memory one;
  - copies of the deciders and of the runtime;
  - generators that tracked the engine's rules to build valid input, or copied the CSV adapter's.

  A copy shares the understanding of the code it checks, so a misread rule passes both, and every
  rule is written twice. That record already counted the model and the generators as code that
  grows with every rule. By the end of the first stage they outweighed the engine.
- Its reasoning still holds: examples cover the cases someone thought of, the costly bugs live in
  sequences nobody listed, and later stores, runtimes and rewrites must be proven to change
  nothing. What no longer holds is that a second engine is the way to say what the first must do.
- Builds on *A pure core*, *State changes only through events* and *Stores defined by their laws*.

## Decision

- Correctness is a set of laws over generated input. Each law compares runs of the engine with
  each other, so no test computes what a run should give:
  - after any generated input, two inputs that the rules make equivalent leave the same accounts;
    for example, a deposit disputed and then resolved, and the same deposit alone;
  - reordering the rows of different clients, given distinct tx ids, changes no summary;
  - an amount's text parses and prints back.
- Generated input is raw CSV text over small domains, entering through the production source. The
  domains are small so that repeats, collisions, locks and disputes come up without the generator
  tracking them.
- Fixtures through the binary pin the contract and the numbers: the exit statuses, the streams,
  the formats, and the tables and reports of hand-computed inputs.
- No test re-implements the engine: no model of it, no reference store, no copy of a decider or of
  the runtime.
- A store is checked through the engine: the laws run the engine over the store, so a store keeps
  the laws its ports define by passing the engine's.
- A law that has never failed is suspect: each one is shown to fail once against a deliberately
  broken implementation.
- Governs: `app/tests/it`.

## Consequences

- The laws are the contract that every refactor, store and runtime must keep. A new rule adds an
  equation, not a second implementation.
- An equation states a relation, not a value. A mistake that both sides share passes every
  equation, and only the fixtures catch it; examples are every amount read ten times too large, or
  a snapshot that leaves out the accounts at zero.
- A failing equation prints both inputs and both sets of accounts, and replays exactly.
- What no run reaches goes unchecked by the laws: an event the runtime never produces, or a
  footprint no run builds, is held only by the types and the lints.
- Deferred: running the laws over each store. With one store, the laws name it. A second store
  makes them generic over the store ports, and every store runs them.
