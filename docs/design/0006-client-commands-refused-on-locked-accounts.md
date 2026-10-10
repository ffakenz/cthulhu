# 6. Client commands refused on locked accounts

## Status

Accepted

## Motivation

Complies with *State changes only through events*.

A chargeback locks the account. Refusing every later command would also drop the provider's later
reports: a second chargeback would never apply, and its funds would stay held.

## Decision

- A chargeback locks the account; nothing unlocks it.
- On a locked account, deposits and withdrawals are rejected as `AccountLocked`.
- Dispute commands still apply.

## Consequences

- Easier: the client can no longer move money, and the ledger still follows the provider.
- Worse: a negative balance cannot be repaid by deposit; repayment needs an unlock the input
  cannot express.
- Rejected: refusing every command; refusing withdrawals only.
- Deferred: an unlock or repayment command.
