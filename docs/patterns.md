# Patterns

The patterns the architecture is built from, where each lives, and the decision that adopts it.

## Decider

The core is a decider: an initial state and two pure functions that together define a state
machine, independent of where commands come from or where state is kept.

```text
       command
          │
          ▼
     ┌─────────┐   event    ┌─────────┐
 ┌──►│ decide  ├───────────►│ evolve  ├───┐
 │   └────┬────┘            └────▲────┘   │
 │        │ rejection            │        │ next state
 │        ▼                      │        │
 │ state unchanged               │        │
 │                               │        │
 └──────────────── state ◄───────┴────────┘
   starts as the initial state: no account, then Opened
```

Both functions read the current state; only `evolve` produces the next one.

| Part | Here |
|---|---|
| initial state | a client with no account; `evolve` applies `Opened` to it, giving the footprint `open()` returns |
| decide: command and state to events | `decide_client`, `decide_dispute`: one event of the command's family, or a rejection |
| evolve: state and event to state | `evolve`, the only place state is computed |
| terminal state | none for a client; a charged-back deposit is terminal and leaves history |

- One decider instance per client. A partition runs many; nothing crosses instances except intake.
- The same decider runs in every runtime: a loop now, partition workers and a service later.
  Recovery folds `evolve` over a log of events, each kept with its address.
- This variant splits off opening: `Opened` is applied before *decide*, which then returns one
  event or a rejection, with its reason, rather than a list of events.
- Origin: Jérémie Chassaing, [Functional Event Sourcing Decider](https://thinkbeforecoding.com/post/2021/12/17/functional-event-sourcing-decider).

## Others

| Pattern | Where | Decision |
|---|---|---|
| Functional core, imperative shell | `core` is pure; `run` reaches effects only through ports; both live in `engine`, which depends on no adapter; only `main.rs` touches the world | *A pure core*; *Layers as packages that depend inward* |
| Domain-driven design, in functional form | `domain`: values, entities and messages named from the overview's Terms; each entity reached by its key; a client's ledger as the aggregate | *A domain model in the records' language*; *Entities identified by their keys* |
| Event sourcing with projections | state is a fold of `evolve`; `Accounts` and `Report` are views | *State changes only through events*; *Sources, sinks and projections* |
| Ports and adapters | `LedgerStore`, `SeenIds`, the CSV source and sink, the report's text sink | *Effects behind ports* |
| Composition root | `main.rs` builds every adapter | *Effects behind ports* |
| Single writer per partition | each client's state has one writer | *State partitioned by client* |
| Idempotent intake | `SeenIds`: the first sight of a tx id wins | *State partitioned by client*; *Tx ids consumed on first sight* (design) |
| Parse, don't validate | `CsvRow` to `Command`; `Amount` has one constructor | *Three failure tiers*; *Rows parsed into commands at the boundary* (design) |
| Property and model-based testing | laws over generated input, a naive model, runtimes proven equal | *Correctness defined by laws and a model*; *Runtimes proven against a sequential reference* |
