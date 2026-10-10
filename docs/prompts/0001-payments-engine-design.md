# Prompt History 0001 — Payments engine design

Date: 2026-10-10

The session set out to understand the exercise brief and its solution space well enough to choose
the engine's approach, then to record that approach before any product code. It started from the
workspace template: the conventions, the gate and an empty decision log.

**Outcome:** twelve architecture decision records — Record architecture decisions; A pure core;
State changes only through events; Effects behind ports; Sources, sinks and projections; Stores
defined by their laws; State partitioned by client; Memory bounded by state, not input; Three
failure tiers; Correctness defined by laws and a model; Runtimes proven against a sequential
reference; csv, serde and rust_decimal as dependencies — and seven design records — Exact decimal
amounts with four places; Rows parsed into commands at the boundary; Tx ids consumed on first
sight; Accounts opened by the first command; A per-deposit dispute lifecycle; Client commands
refused on locked accounts; The command-line contract. Plus the overview, architecture, patterns
and roadmap documents, with README.md, CLAUDE.md, two skills and the reviewer agent brought in
line. No product code.

---

### 1. Explore the brief and brainstorm

> the goal for this session is for you to explore the following spec: [...] become an expert
> the idea is that we can talk togetehr about the problem domain and solution space in a
> brainstorming session
> the goal is for me to decide on how to approach the problem and what would be the ideal solution
> [...]

Three research tracks ran in parallel: public solutions to the same exercise, hands-on probes of
`csv` and `rust_decimal`, and real-world dispute and chargeback rules. They fed a briefing: the
domain model, the library traps, a map of ambiguous decisions and three strategies.

### 2. Review the first engine draft

> i made my draft on how would i model the engine:
> [...]
> (cmd, state) -> (event, state)
>     (tx, account) -> (event, account)
> [...]
> txs > group by client >parallel by client> sort by tx > fold
> [...]
> wdyt?

The review kept the pure fold but widened its state to each client's deposits, showed that
sorting by tx id breaks order so rows are routed by client instead, put duplicate detection at
intake, and proposed concrete laws with an edge-weighted generator.

### 3. Choose an event-driven engine and the mainstream crates

> option c: i want the input to be streamed and splitted in workers that process inputs as they
> come and result build a projection that becomes the final merge result, [...]
> option 3: The usual csv + serde + rust_decimal
> [...]
> i would like to use abstractions like: engine = source + pipeline + sink + projections

Each abstraction got one job, commands were separated from events, and the reply listed the
dependency footprint, the crate-trap checklist and the laws that make a parallel engine credible.

### 4. Sink as a table, CSV in the CLI, history in the pipeline

> - sink should be a store | table -> projection built from merging ledgers
> - csv is output piped in cli, not in run
> - pipeline must also use not only the commands as source but also the in-memory tx history [...]
> next is to continue the discussion about parallelism, how to run the workers, chunks or strems,
> etc

`run` returns a bounded summary, CSV adapters sit at both edges, and history is per-client state
inside each partition. The parallelism analysis found decoding dominant and treated chunks and
streams as one mechanism.

### 5. Store ports, future runtimes, staged delivery

> - have store interfaces against in-memory data structures
> - understand the client currently is a cli [...] but in future will be a tokio server that has to
>   scale horizontally, [...] thus the engine must be simple and pure [...]
> i would start with option "Sequential core plus all the laws" and move towards B after first
> implementation [...]

The reply set out a footprint-based pure core, synchronous store ports, the first stage's scope, a
staged roadmap and proposed defaults for every transaction rule.

### 6. Approve and start the records

> ok we reached an aggreemnet, approved!
> lets start building the ADRs (bottom-up) then the architecture md with ascii diagram, overview
> docs explaining state machine (lifecycle of txs) etc
> [...]

A first set of decision records and documents was drafted and sent to an independent review.

### 7. Hold the session record

> do not write this session yet, i want to follow-up on the ADRs and docs/ first

Nothing was committed, and the draft of this entry was set aside.

### 8. Open accounts on any first command

> shouldn't the account get created on first command regardless if deposit/withdrawal? [...]

Agreed: the first well-formed command naming a client opens its account, whatever its outcome.

### 9. Keep decide and evolve

> also i do not understnad why can't we preserve total functions using  decide/evolve

The split was restored, with `decide` only checking and `evolve` the only computation; the mistaken
argument against it went to [tasks/lessons.md](../../tasks/lessons.md).

### 10. Architecture principles, not solution space

> i think your ADRs describe the approach to address problems on the spec but now all really
> states why they drive the architecture.
> a good example on how ADRs are evolved and useful is here: [...]
> [...]
> note i dont want to mention [...] anywhere in this repo

The records were split: principle-level architecture decisions in [docs/adr/](../adr/README.md),
and how the engine treats its input as design records in [docs/design/](../design/README.md).

### 11. Roles of the overview and architecture documents

> architecture.md is a concise picture of ADRs and overview is a concise description of what you
> did along with other importnat highlevel details/decisions made

[docs/architecture.md](../architecture.md) became a picture of the architecture records, and
[docs/overview.md](../overview.md) a summary of the engine and its design records.

### 12. Write the restructured set

> yes

Twelve architecture records and seven design records were written, the documents and rule files
were brought in line, and two more independent reviews were resolved.

### 13. Name the decider pattern and restore the module diagram

> i think we should mention the "decider pattern" somewhere
> maybe in docs/patterns.md
> last i think the architecture md used to have a nice ascii diagram of the system modules [...]

[docs/patterns.md](../patterns.md) now leads with the decider, and the layered module diagram is
back in the architecture document.

### 14. Draw the decider

> can we add a diagram on decider pattern

The decider's loop of decide, evolve and state was drawn in the patterns document.

### 15. Detail the pipeline

> i also think would be nice to have a greater/enhanced diagram for the pipeline, [...] that
> includes ports etc

A detailed pipeline diagram, with every port, adapter and failure path, was added below the
summary one.

### 16. Approve, commit and land

> 1. all records approved
> 2. keep everything in this session as a single commit (include session history to prompt/)
> 3. then commit + push branch remote + merge to main (do not remote branch remote)

Every record was accepted, and the session landed as one commit, pushed on its branch and merged
into `main`.
