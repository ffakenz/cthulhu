# Overview

The engine's behaviour: the accounts it keeps, the commands it accepts or rejects, and what it
reads and prints. Each rule that needed a decision has a design record in `docs/design/`, named
here by its title.

## Terms

| Term | Meaning |
|---|---|
| client command | `deposit` or `withdrawal`: the client moves money |
| dispute command | `dispute`, `resolve` or `chargeback`: the payment provider acts on a recorded deposit |
| event | a fact the engine accepted |
| rejection | a refused command, with its reason |
| history | a client's deposits not charged back |
| footprint | what one command touches: its client's account and at most one deposit |
| address | a command's client and tx id: where its footprint is loaded and committed |
| amount refusal | why text is not an amount: malformed, too precise, too large, or zero |

## Accounts

`available` and `held` are stored; `total = available + held` by construction. `locked` is set by
a chargeback and never cleared.

| Event | `available` | `held` | `total` | Also |
|---|---|---|---|---|
| `Opened` | `0` | `0` | `0` | account created, unlocked |
| `Deposited` | `+a` | | `+a` | deposit recorded as `Settled` |
| `Withdrawn` | `-a` | | `-a` | |
| `Held` | `-a` | `+a` | | deposit `Settled` to `Disputed` |
| `Released` | `+a` | `-a` | | deposit `Disputed` to `Settled` |
| `ChargedBack` | | `-a` | `-a` | deposit removed; account locked |

`a` is the command's amount or, for a dispute command, the recorded deposit's amount.

## Deposit lifecycle

*A per-deposit dispute lifecycle*.

```text
                       dispute            chargeback
 deposit ──► Settled ──────────► Disputed ──────────► removed, account locked
                ▲                   │
                └────── resolve ────┘
```

A dispute applies even when it makes `available` negative. After a chargeback, a negative `total`
is what the client owes. Withdrawals are not in history and cannot be disputed.

## Commands

*Accounts opened by the first command*, *Tx ids consumed on first sight*, *Client commands refused
on locked accounts*.

The first command naming a client opens its account, whatever its outcome. Checks run in the
order listed; the first that fails names the reason.

| Command | Rejected when | Accepted as |
|---|---|---|
| `deposit` | tx id used by an earlier deposit or withdrawal: `DuplicateTx`; account locked: `AccountLocked` | `Deposited` |
| `withdrawal` | tx id used: `DuplicateTx`; account locked: `AccountLocked`; amount above `available`: `InsufficientFunds` | `Withdrawn` |
| `dispute` | no such deposit for this client: `UnknownDeposit`; deposit `Disputed`: `NotSettled` | `Held` |
| `resolve` | no such deposit for this client: `UnknownDeposit`; deposit `Settled`: `NotDisputed` | `Released` |
| `chargeback` | no such deposit for this client: `UnknownDeposit`; deposit `Settled`: `NotDisputed` | `ChargedBack` |

Dispute commands apply on locked accounts too. An arithmetic overflow would reject a command as
`Overflow`; the amount cap keeps every balance exact, so it never occurs.

## Input

*Rows parsed into commands at the boundary*, *Exact decimal amounts with four places*.

- A header naming `type`, `client`, `tx` and optionally `amount`, in any order; whitespace around
  fields is ignored.
- `type` is one of the five commands, lowercase; `client` is a `u16`; `tx` is a `u32`.
- `amount` is required for client commands and ignored for dispute commands: digits with an
  optional fraction, positive, exact at four decimals, below 10^15.
- A malformed row is skipped and counted by kind; an I/O error, or a header missing any of
  `type`, `client` or `tx`, stops the run.

## Output

*The command-line contract*.

- stdout: the header `client,available,held,total,locked`, then one row per account by client id;
  balances with four decimals, never `-0.0000`; `locked` as `true` or `false`.
- stderr: counts of rejected commands by reason, and of malformed rows by kind.

Input:

```text
type, client, tx, amount
deposit, 1, 10, 25.5
withdrawal, 1, 11, 5.25
deposit, 2, 20, 10
withdrawal, 2, 21, 10
dispute, 2, 20,
chargeback, 2, 20,
deposit, 2, 22, 3
withdrawal, 3, 30, 1
dispute, 4, 40,
```

Output:

```text
client,available,held,total,locked
1,20.2500,0.0000,20.2500,false
2,-10.0000,0.0000,-10.0000,true
3,0.0000,0.0000,0.0000,false
4,0.0000,0.0000,0.0000,false
```

Report:

```text
1 rejected: the account is locked
1 rejected: the amount is above available
1 rejected: no such deposit for this client
```

Client 2 withdrew a deposit that was then charged back: it owes 10, is locked, and its last
deposit is rejected. Clients 3 and 4 are named only by rejected commands, which still open their
accounts.

## Laws

Every runtime keeps these; each is a property test.

1. `held` equals the sum of the client's `Disputed` deposits, and is never negative.
2. `total` equals deposited minus withdrawn minus charged back.
3. `locked` holds exactly when a chargeback happened.
4. A rejection changes no balance.
5. A repeated deposit or withdrawal row changes no balance.
6. Only `Held` makes `available` negative.
7. Reordering rows of different clients changes no account, given unique tx ids.
8. The engine agrees with a naive model on every generated input.
