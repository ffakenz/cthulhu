# 1. Record architecture decisions

## Status

Accepted

## Motivation

- The engine starts as a command-line program and is meant to grow into a partitioned, durable
  service. The choices that shape that growth must outlive the sessions that made them.
- Two kinds of choice arise: principles that shape the architecture whatever the input's rules
  are, and choices about how the engine treats its input. Mixed in one log, the second kind
  buries the first.

## Decision

- Architecture decisions are recorded in `docs/adr/`: one principle per record, stated by the
  roles it constrains, with the forces behind it and its consequences.
- Choices about how the engine treats its input are design records, in `docs/design/`, in the
  same format.
- An accepted record is never edited; a reversal is a new record that supersedes it.
- A rejected alternative stays in the record that rejected it, with its reason.
- Records name each other by title; only the indexes and `Superseded by` lines carry numbers.
- Governs: `docs/adr/`, `docs/design/`, their indexes and `docs/adr/template.md`.

## Consequences

- The architecture can be read from the log alone, in the order it was decided.
- Each ADR names the records it builds on, and each design record the ADRs it complies with, so
  changing a principle shows what depends on it.
- Two logs to keep consistent: a design record complies with the architecture, never the reverse.
- Records cost time to write; a choice that costs an afternoon to reverse belongs in a commit
  message instead.
