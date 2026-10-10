# 3. State changes only through events

## Status

Accepted

## Motivation

- Builds on *A pure core*.
- Checking a command and computing the next state are two concerns: whether a command is allowed,
  and what an accepted fact does to the state.
- Recovery and audit need facts that rebuild the state when applied in order.

## Decision

- Handling a command takes two pure steps:
  - *decide*: from the state and the command, an event or a rejection. It checks conditions and
    computes nothing.
  - *evolve*: from the state and an event, the next state. It is the only place state is
    computed, and its one failure, arithmetic overflow, is a value.
- With the state a client starts from, *decide* and *evolve* form a decider.
- An event is self-contained: it carries everything *evolve* needs, so applying it consults only
  the state it changes.
- A rejection is not an event and changes no state.
- Governs: `decide_client`, `decide_dispute`, `evolve`, `Event`, `Reason`.

## Consequences

- State is a fold of *evolve* over events, so the ledger and every recovery replay the same facts.
- *decide* and *evolve* are tested apart: rejection rules against the first, conservation of money
  against the second.
- Each rule is split across two functions, and events carry more than bare deltas.
- Durability needs no new rule, only a store that appends events.
