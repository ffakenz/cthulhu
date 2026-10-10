# 5. A per-deposit dispute lifecycle

## Status

Accepted

## Motivation

Complies with *State partitioned by client* and *State changes only through events*.

A dispute, a resolve and a chargeback each act on a deposit recorded earlier and take its amount
from that record. The engine must fix which records they can act on, the states a deposit moves
through, and what a dispute does after the money has been withdrawn.

## Decision

- History keeps deposits only, per client, keyed by tx id: an amount and a status, `Settled` or
  `Disputed`.
- The only transitions:

  | Command | From | To | Balances |
  |---|---|---|---|
  | dispute | `Settled` | `Disputed` | `available -= amount`, `held += amount` |
  | resolve | `Disputed` | `Settled` | `held -= amount`, `available += amount` |
  | chargeback | `Disputed` | removed | `held -= amount`, so `total` falls; the account locks |

- A dispute applies even when it makes `available` negative.
- Rejections: no deposit of this client under that id is `UnknownDeposit`; disputing a `Disputed`
  deposit is `NotSettled`; resolving or charging back a `Settled` one is `NotDisputed`.

## Consequences

- Easier: withdrawals stay final; a claim on money sent out would credit the client, the
  opposite of a hold.
- Easier: after a chargeback, a negative `total` is the debt left by reversing a deposit that was
  already withdrawn.
- Easier: a charged-back deposit leaves history, so later commands naming it are `UnknownDeposit`
  and its memory is freed.
- Worse: one deposit can be disputed and resolved without limit.
- Worse: history grows with every deposit not charged back.
- Rejected: disputable withdrawals; refusing a dispute when `available` is short; a final resolved
  state.
- Deferred: evicting deposits after a dispute window; the input carries no timestamps.
