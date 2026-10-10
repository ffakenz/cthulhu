# 2. Rows parsed into commands at the boundary

## Status

Accepted

## Motivation

Complies with *Sources, sinks and projections* and *Three failure tiers*.

Rows come from outside and can be malformed. The core must never see an invalid amount or a
deposit without one, and one malformed row must not stop the rest.

## Decision

- The CSV source yields, per row and in order, `Row::Parsed(Command)` or
  `Row::Malformed(RowError)`. A `SourceError` (an I/O error, or a header missing any of `type`,
  `client` or `tx`) ends the stream and fails the run.
- A row deserializes into `CsvRow`, which keeps `amount` as text; `TryFrom<CsvRow>` builds the
  `Command`:
  - `deposit`, `withdrawal`: a `ClientCommand`, whose amount `Amount` must parse;
  - `dispute`, `resolve`, `chargeback`: a `DisputeCommand`; any amount text is ignored.
- Input shape: a header naming `type`, `client`, `tx` and optionally `amount`, in any order;
  whitespace around fields; lowercase type names; `client` as `u16`, `tx` as `u32`.
- `RowError` is `Unreadable`, `TooManyFields`, `MissingAmount` or `InvalidAmount`.
- Reader settings: `Trim::All`, since padded fields fail otherwise; `flexible(true)`, since a row
  may omit the trailing column, with the adapter treating rows longer than the header as malformed.
- Amounts are never deserialized as numbers, which `csv` would route through `f64`.
- `Row`, `RowError` and `SourceError` are declared beside `run`, their consumer.

## Consequences

- Easier: the core's inputs are valid by construction; a malformed row is a value.
- Easier: column order and padding do not matter.
- Worse: no guessing; an uppercase type such as `DEPOSIT`, a stray extra field or a missing
  amount skips the row.
- Worse: ids inherit `csv`'s integer parsing, which also accepts `+1`, `01` and `0x10`.
- Rejected: reading rows straight into an internally tagged command enum; it fails on every `csv`
  row.
