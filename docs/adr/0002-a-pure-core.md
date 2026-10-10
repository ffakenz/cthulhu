# 2. A pure core

## Status

Accepted

## Motivation

- The same rules must hold whether the engine runs as a command-line program, on threads or as a
  networked service.
- Money rules are easy to get subtly wrong, so they must be testable exhaustively, without files,
  processes or clocks.
- Anything a function reads besides its arguments (input, storage, time, scheduling) makes two
  runs differ.

## Decision

- The engine's logic is a pure, total function of the current state and one command. It performs
  no effect.
- The vocabulary of every record:
  - a *command* is a request from outside, which the core may refuse;
  - an *event* is a fact the core accepted;
  - a *rejection* is a refused command, with its reason;
  - an *effect* is anything outside the core: reading input, storing state, writing output,
    threads, time.
- Effects are performed by the code around the core, never inside it.
- Governs: the library's `core` module, which imports no effectful API.

## Consequences

- The same state and command give the same result in every runtime, so a run replays exactly.
- Every rule is tested as a plain function.
- Everything the core needs must arrive as a value, time included if a rule ever needs it.
- The code around the core grows: it loads state, calls the core and performs what follows.
