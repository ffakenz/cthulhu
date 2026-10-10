# 5. Sources, sinks and projections

## Status

Accepted

## Motivation

- Builds on *State changes only through events* and *Effects behind ports*.
- Input arrives as one stream of rows today and as many connections later; output is one table
  today, and logs, metrics or live queries later.
- Formats change more often than rules.

## Decision

- The engine is a pipeline of four roles:
  - a *source* yields commands, or malformed rows, in arrival order;
  - the *runtime* turns each command into events or a rejection, through the core;
  - a *projection* folds outcomes (events, rejections, malformed rows) into a view, such as the
    account table or the report;
  - a *sink* renders a view for the outside.
- The source is a port the runtime consumes; a source adapter turns a byte stream (`Read`) into
  commands. A sink adapter renders a view onto a byte stream (`Write`). Every input and output
  format lives only in these adapters.
- Views are values returned to the composition root, which hands them to sinks.
- Governs: the source port (an iterator of `Result<Row, SourceError>`); the adapters `CsvSource`
  (over `Read`) and `render_csv` (onto `Write`); `run`, `Summary`, `Accounts`, `Report`.

## Consequences

- A new format is a new adapter; the core and the projections do not change.
- A new view, such as an exposure report or a metric, is a new projection over the same outcomes.
- Only the adapters know column names, spacing or number formats.
- The account table is complete only when the source ends; a live view needs another sink.
