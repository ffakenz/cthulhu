# 3. Tx ids consumed on first sight

## Status

Accepted

## Motivation

Complies with *State partitioned by client* and *Stores defined by their laws*.

A replayed deposit or withdrawal would move money twice. Whether a later row repeats an id must
not depend on whether the first one succeeded, or the check could not run at intake, ahead of the
balances.

## Decision

- Intake answers `Sight::First` or `Sight::Repeat` for the tx id of every deposit and withdrawal,
  and records the id on first sight, whatever the command's outcome.
- A repeat is rejected as `DuplicateTx`, across all clients.
- Dispute commands act on a recorded deposit and consume no id.

## Consequences

- Easier: replays are harmless, and the answer depends only on arrival order.
- Worse: a rejected withdrawal uses up its id; a corrected retry needs a new one.
- Worse: the set of used ids grows with the input, up to 2^32.
- Rejected: uniqueness per client only; consuming an id only when its command succeeds.
