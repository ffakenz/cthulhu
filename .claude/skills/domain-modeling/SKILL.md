---
name: domain-modeling
description: "Use when a change names or shapes a domain type, adds a term to the model, chooses an id or a key, or maps a file format onto the domain. Covers domain-driven design in functional form: the records' terms as the one vocabulary, classifying a concept as a value, an entity, the aggregate, a message, a view or a boundary type, identity held by keys, derived quantities, one meaning per absence, the aggregate seen through footprints, and the review questions for a new domain type. Keywords: DDD, domain-driven design, domain model, ubiquitous language, glossary, term, naming, synonym, value object, entity, aggregate, identity, id, key, address, command, event, view, projection, DTO, boundary type, anti-corruption layer, footprint, Option, nested Option, derived field, invariant, consistency boundary, row, CSV."
---

# Domain modeling

## The rule

**Name each type from the records, classify it before writing it, and make it read as the
sentence that defines it.**

The records define the engine's concepts: `docs/overview.md` § Terms, and the decision and design
records. A type is the code's copy of one of those sentences. When the copy drifts, every reader
translates; when it admits more than the sentence allows, it holds states nothing should. The
decisions behind this skill are *A domain model in the records' language* and *Entities identified
by their keys*. This skill decides what to type; [[curry-howard]] reads each type as the
proposition it states, and [[functional-architecture]] spells it in Rust (newtypes, private
fields, parsers, exhaustive matches).

## The failure this prevents

Suppose the footprint were shaped like this, and loaded as an `Option<Footprint>`:

```rust
pub struct Footprint { pub account: Account, pub slot: Option<Slot> }
pub struct Slot { pub tx: TxId, pub deposit: Option<Deposit> }
```

It compiles, and every rule can be written against it. But it models what a store must do (write
nothing, delete a key, store a deposit) instead of the state, and it shows:

- three `None`s with three meanings (no account, no slot, an empty slot), and an accessor that
  exists only to collapse two of them;
- two spellings of one fact: a new client's missing slot and a known client's empty slot mean the
  same to `decide_dispute`;
- a tx id in the slot and in the event, so a total `evolve` must answer pairs no run produces;
- an `Opened` that cannot build a slot, since it names no tx id.

The glossary already holds the answer: what one command touches is its client's account and at
most one deposit. Read as a type, reached from outside at the command's address, its client and
tx id:

```rust
pub struct Footprint { account: Account, deposit: Option<Deposit> }
```

## The rules that follow

- **The records' terms are the only vocabulary.** A domain type, variant or core function takes
  its name from § Terms or a record. A new concept enters § Terms with its code, and a record too
  if it decides something ([[architecture-decisions]]). A type that only partitions an existing
  term takes its name from that term and needs no row of its own, as the events of client
  commands are `ClientEvent`; types in the test crate are outside the rule. One word per concept,
  and where the records use one word twice the types keep the two apart: `ClientCommand::Deposit`
  is a request, the type `Deposit` a deposit in history, and the event `Deposited` the fact.
- **Classify before you type.** Each concept is one of the kinds below, and each kind has its own
  rules.
- **Identity is a key, never a field.** Store an entity under its id and leave the id out of its
  value; a view pairs them again. One sign: the step that creates the entity cannot name its id,
  as `Opened` creates an account and knows no client id, so `Account` holds none. A command
  carries its address; an entity never carries its own key. A footprint is reached at its
  command's address, which `load` and `commit` take beside it.
- **Store what is independent; derive what could disagree.** An account stores `available` and
  `held`; `total()` is their sum, so no account can hold a total that contradicts them.
- **Every absence has one meaning.** Before settling a type, write down what each `None` means.
  Reshape it when two `None`s mean the same to every reader, or when one exists to instruct a
  writer, such as "delete this key", instead of stating a fact. An accessor that only flattens
  options is the symptom.
- **The aggregate is the consistency boundary.** A client's ledger, its account and its history,
  has one writer (*State partitioned by client*). What no type can hold, such as held equalling
  the sum of the disputed deposits, is a law ([[rust-properties]]). No domain type holds the
  aggregate whole: the core sees one footprint. A rule that needs more than a footprint changes
  the footprint, and so a record; a rule that spans clients belongs at intake.
- **Behaviour lives in the decider.** Entities expose readings; `decide_client` and
  `decide_dispute` hold the rules and `evolve` the changes. A method that changes an entity is an
  event without a name.
- **File formats stop at the adapters.** A row and a printed account are adapter types (*Sources,
  sinks and projections*), mapped to and from the model once, at the boundary. No column name,
  kind string or optional amount reaches `domain`.
- **Fields are not the interface.** Domain types keep their fields private and expose readings as
  methods ([references/rust.md](../functional-architecture/references/rust.md) § 1). A type with
  no parser gets one constructor that takes its parts: `pub(crate)` when only the core builds it,
  as for an account or a deposit, and `pub` when a store must rebuild it, as for a footprint.

## The kinds

| Kind | What it is | Here | Its rules |
|---|---|---|---|
| Value | defined by its contents | `ClientId`, `TxId`, `Address`, `Amount`, `Balance`, `Lock`, `Status`, `Sight`, `Reason`; `Footprint`, entity states without their keys | immutable; valid by construction; compared by value |
| Entity | an identity with a lifecycle | an account; a deposit in history | reached by its key; readings only |
| Aggregate | the unit of consistency | a client's ledger | one writer; what no type holds is a law; seen through footprints |
| Message | a request, or a fact | `Command`, `Event` | a command carries its address; an event carries everything `evolve` needs |
| View | a projection for the outside | `Accounts`, `Report` | never stored back; `Accounts` pairs each client id with its account |
| Boundary type | a file format's shape | `CsvRow`; the printed account row | adapters only; mapped once |

## Review questions

Ask them of every domain type a change adds or reshapes:

1. Is its name, and each variant's, in § Terms or a record?
2. Which kind is it, and does it keep that kind's rules?
3. Does its value carry the key it is stored under?
4. Does it store anything that could be derived?
5. What does each `None` in it mean, and do two mean the same?
6. Does a file format's shape, column or string reach `domain`?
7. Does it grow a method that changes it?

## Verify

`make ci` is green, every new term is in `docs/overview.md` § Terms, and each domain type the
change touches has an answer to every review question.

Related: [[curry-howard]] · [[functional-architecture]] · [[architecture-decisions]] ·
[[rust-properties]] · [[review-and-planning]]
