# 4. Accounts opened by the first command

## Status

Accepted

## Motivation

Complies with *State changes only through events* and *Three failure tiers*.

Some rule must decide when a client's account starts to exist. Every well-formed command names
its client, whether or not the command is then refused.

## Decision

- The first command naming a client opens its account with zero balances (event `Opened`),
  whatever that command's outcome.
- A malformed row opens nothing, as *Three failure tiers* requires.

## Consequences

- Easier: one rule for every command; every client a command names appears in the output.
- Worse: a client named only by refused commands appears with zero balances.
- Worse: a rejected first command still opens its account through `Opened`, so the law is "a
  rejection changes no balance".
- Rejected: opening only on deposits and withdrawals (two rules for one question); opening only
  on a successful deposit (a refused withdrawal would leave no account).
