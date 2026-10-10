# 13. A domain model in the records' language

## Status

Accepted

## Motivation

- Builds on *A pure core*, *State changes only through events*, *Sources, sinks and projections*
  and *State partitioned by client*.
- The records name the engine's concepts: commands, events, rejections, accounts, history,
  footprints. The files suggest other shapes, a row with a kind and an optional amount, a printed
  account with its id and a stored total, and each admits states the records rule out.
- Code that names things its own way breaks the line from a record's sentence to a type, and
  every review starts by translating.

## Decision

- The `domain` module is the model of the ledger, in the terms `docs/overview.md` and the records
  define. Every domain type, variant and core function takes its name from them; a new term
  enters the overview's Terms before code uses it, or in the same change.
- Each domain type is one of three kinds:
  - a *value*, defined by its contents and valid by construction: an id, an address, an amount,
    a balance, a status, a reason; and a footprint, the part of the aggregate one command
    touches, which holds entity states without their keys;
  - an *entity*, an identity with a lifecycle: an account, a deposit in history;
  - a *message*: a command, which the core may refuse, or an event, a fact it accepted.
- A client's ledger, its account and its history, is the aggregate: the unit *State partitioned
  by client* gives one writer. Its invariants that no type can hold, such as held equalling the
  sum of the disputed deposits, are laws. No domain type holds it whole; the core sees it one
  footprint at a time.
- Entities expose readings, never operations that change them: rules live in `decide_client` and
  `decide_dispute`, changes in `evolve`.
- A quantity that follows from others is derived, never stored: an account's total is its
  available plus its held.
- No file format is a domain type: the rows read from a file and the accounts printed to one
  belong to the adapters, which map them to and from the model.
- Governs: the `domain` module (`backend/src/domain.rs`, `backend/src/domain/`), the names in
  `backend/src/core.rs`, and the Terms table in `docs/overview.md`.

## Consequences

- A record's sentence maps to a type, and a synonym, a row-shaped type or a stored total is a
  review finding.
- A value holds only what its constructors admit, so the core never validates one again; what no
  type can hold is a law, checked over generated input.
- The domain does not mirror the files: a reader finds the row as read and the account as printed
  in the adapters.
- Every new concept costs a glossary entry, and a record if it decides something, with its code.
- Rejected: modelling the files, as a transaction with a kind string and an optional amount, and
  an account that stores its total and its client id; both admit states the records rule out.
