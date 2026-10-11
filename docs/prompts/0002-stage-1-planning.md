# Prompt History 0002 — Stage 1 planning

Date: 2026-10-10

The session set out to plan the library core of stage 1 (domain, core, run and memory, one commit
each) from the accepted records, and to hold the plan for approval before any code. It started
from the records commit on `main`, with the gate green and no product code.

**Outcome:** the plan of record, [tasks/todo.md](../../tasks/todo.md), with one ticket per step,
checked in a scratch crate built from its code blocks. Two architecture decision records — A
domain model in the records' language; Entities identified by their keys — and two skills,
`domain-modeling` and `curry-howard`, were folded into the records commit, with their syncs to
CLAUDE.md, README.md, the reviewer agent, the property harness and the overview, architecture and
patterns documents. Execution moved to a fresh session. No product code.

---

### 1. Plan the library core

> Build the library core of stage 1 in four steps, in this order: domain, core, run, memory.
> Each step lands as exactly one commit. [...]
> - The repo is public: never name [...]. Never quote, restate or paraphrase the external brief
>   in any tracked file. [...]
> - One commit per step, [...] But the red and the green land together in the step's single
>   commit [...]
> - Show each law failing once against a deliberately broken implementation, and record the
>   evidence under tasks/.
> [...]
> - Use superpowers:writing-plans to turn the records into a plan in tasks/todo.md [...] Stop for
>   my approval before writing code, then ask how I want it executed (subagent-driven or inline).

A probe of `rust_decimal` confirmed its four-crate build graph and found its permissive parser and
silent rounding near the top of its range. The plan and its four tickets were written with the
commands that prove each step, and held for approval.

### 2. Question the footprint

> question: explain the Footprint type and its fields, how does it participate in the pipeline
> and why deposit is optional and slot is optional?
> question: why evolve takes an option Footprint?
>
> i start to think we should model Footprint more semantically, wdyt?

The reply traced the footprint through load, decide, evolve and commit, and proposed a flatter
shape: an account and at most one deposit, reached from outside by the command's client and tx id.

### 3. Key deposits by both ids

> question: why deposits have this shape deposits: BTreeMap<(ClientId, TxId), Deposit>, and not
> deposits: BTreeMap<ClientId, List<(TxId, Deposit)>>
>
> ?

A map keyed by both ids answers a command with one lookup and cannot hold a tx id twice for a
client; a list per client would scan, and could.

### 4. A slot with the tx and its type

> ok understod about Footprint, but what about having:
> pub struct Footprint { pub account: Account, pub slot: Slot }
> pub struct Slot { pub tx: TxId, pub txType: TxType }
> ?

Argued against: history keeps deposits only, so the type would always read deposit, and a tx
inside the footprint repeats the address and cannot be built by `Opened`.

### 5. Revise the plan, hold execution

> ok, revise the plan to this shape, but do not proceed to execution yet, there are some other
> questions i want to aslk

The plan and tickets moved to the flat footprint. No code was written.

### 6. A slot with the tx and an optional deposit

> ok understod about Footprint, but what about having:
> [...]
> pub struct Slot { pub tx: TxId, pub deposit: Option\<Deposit> }
> ?

The flat shape stayed: the slot's tx repeats the address, and an optional footprint holding an
optional deposit gives three absences, each needing its own meaning.

### 7. Apply domain-driven design

> question: why i dont see types like
> [...]
> are we applying DDD first principles?>

The reply sorted the domain's types into values, entities and messages, and showed why types
shaped like the files admit states the records rule out.

### 8. Record it in a skill and in records

> i would not only add a domain-model section but also think is important to capture this at the
> .claude/ skill layer and ADR no?

Proposed two architecture records, A domain model in the records' language and Entities
identified by their keys, a `domain-modeling` skill, and a domain-model section in the plan.

### 9. Approve the domain model

> approved

The records were drafted, the skill written, and CLAUDE.md, the reviewer agent and the documents
brought in line with them.

### 10. Amend, don't stack

> lets not commit on top, rather ammend corresponding commits

The plan now folds the records, the skill and their syncs into the records commit at the branch's
base. That commit is already on `main`, so landing the branch needs a force-update, which is the
person's to do.

### 11. Step kinds in the test driver

> in the meantime, resolve me these question:
> why Step has again and we dont have different Step kinds and every step run yields a record
> with each?

The reply proposed building the driver's step from sums: a client command with its sight or a
dispute command, then an accepted or a rejected outcome, with the one law that needed a second
decision computing it itself.

### 12. Redesign the step

> yes

The driver was rewritten in that shape and compiled in a scratch crate under the workspace lints.

### 13. Type everything

> now, i want you to look every review/question i made, find out the pattern over my concerns and
> review everything based on them and come up with other places where we can benefit from:
> - type everything
> - break down types into more semantically broken down ones whenever code becomes more
>   selfdocument
> - benefit DSL
>
> I see there is no skill about Curry–Howard, can we also add it + address the above with it in
> mind

A `curry-howard` skill was written. A review of the plan against it proposed thirteen changes,
among them an `Address` type, decisions that return only their own family's events and reasons,
events without ids, a total `open`, a `Lock` enum, and typed rows, models and oracles in the
tests.

### 14. Apply the thirteen changes

> yes

The records, the interfaces and the tickets took all thirteen. A scratch crate built from the
plan's code blocks passed clippy under the workspace lints and its tests, and caught two
deliberate breaks.

### 15. A mistyped command

> /context/

It arrived as a message instead of running the command. Nothing changed.
