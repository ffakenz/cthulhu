# Architecture decisions

One numbered record per decision. `NNNN-kebab-case-title.md`, sequential with no gaps, and a
number is never reused. Copy `template.md` to start one; the conventions are in the
`architecture-decisions` skill.

A reversed decision is superseded, never edited: set the old record's Status to
`Superseded by ADR M` and leave the rest of it alone.

Outside this index, the design records' index and a record's `Superseded by` line, never cite a
record's number, in code or in any tracked document — the record names the code, not the other
way round.

Records here state architecture principles. How the engine treats its input is in
[docs/design/](../design/README.md).

## Index

- [1. Record architecture decisions](0001-record-architecture-decisions.md)
- [2. A pure core](0002-a-pure-core.md)
- [3. State changes only through events](0003-state-changes-only-through-events.md)
- [4. Effects behind ports](0004-effects-behind-ports.md)
- [5. Sources, sinks and projections](0005-sources-sinks-and-projections.md)
- [6. Stores defined by their laws](0006-stores-defined-by-their-laws.md)
- [7. State partitioned by client](0007-state-partitioned-by-client.md)
- [8. Memory bounded by state, not input](0008-memory-bounded-by-state-not-input.md)
- [9. Three failure tiers](0009-three-failure-tiers.md)
- [10. Correctness defined by laws and a model](0010-correctness-defined-by-laws-and-a-model.md)
- [11. Runtimes proven against a sequential reference](0011-runtimes-proven-against-a-sequential-reference.md)
- [12. csv, serde and rust_decimal as dependencies](0012-csv-serde-and-rust-decimal-as-dependencies.md)
- [13. A domain model in the records' language](0013-a-domain-model-in-the-records-language.md)
- [14. Entities identified by their keys](0014-entities-identified-by-their-keys.md)
