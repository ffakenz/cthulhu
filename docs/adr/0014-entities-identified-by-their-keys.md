# 14. Entities identified by their keys

## Status

Accepted

## Motivation

- Builds on *State partitioned by client*, *Stores defined by their laws* and *A domain model in
  the records' language*.
- An entity's identity can live in its value, in the key it is stored under, or in both. In both,
  the two can disagree; in the value alone, every step that creates an entity must be handed an
  id it has no other use for.
- `Opened` creates an account knowing no id, and a footprint or an event that carried a tx id
  could disagree with the address its step loads and commits at.

## Decision

- An entity's identity is the key it is stored and reached under, never a field of its value: an
  account under its client id, a deposit under its client and tx id.
- A command's *address*, `Address`, is its client and tx id. Its footprint is loaded and committed
  at that address, which travels beside the footprint, into `load` and `commit`, never inside it.
- A command carries its address; an event carries no id. An event applies at its command's
  address, and a log that keeps events pairs each with that address.
- `Accounts`, the view of accounts, yields each client id with its account.
- Governs: `Address`, `ClientCommand`, `DisputeCommand`, `Account`, `Deposit`, `Footprint`,
  `Event`, `open`, `LedgerStore::load`, `LedgerStore::commit`, `Accounts`, `MemoryLedger`.

## Consequences

- No value can disagree with the key it is stored under, and `open` builds a footprint without
  being handed an id.
- A store serves a command with two lookups at most: the account by client id, the deposit by its
  address.
- A value means nothing without its key: code that moves an entity carries both, and the store
  ports take the address beside the footprint.
- An event names nothing on its own: whatever reads one outside its step, a log or a later
  projection, needs the address beside it.
- A self-describing record, such as a row on disk or a log line, pairs key and value, or address
  and event, in the adapter that writes it.
- Rejected: an id inside each entity, a tx id inside the footprint, and a tx id inside each event.
