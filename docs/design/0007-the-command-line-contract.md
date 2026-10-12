# 7. The command-line contract

## Status

Superseded by 8

## Motivation

Complies with *Effects behind ports* and *Three failure tiers*.

A caller sees only the process: its argument, its two output streams and its exit status. They
must stay stable, and stdout must carry nothing but the account table.

## Decision

- Exactly one argument, the input path; any other count is a usage error.
- stdout: the header `client,available,held,total,locked`, then one row per account by client id,
  balances as *Exact decimal amounts with four places* prints them, `locked` as `true` or
  `false`.
- stderr: counts of rejected commands by reason, and of malformed rows by kind.
- The table is written only after the run succeeds. Any failure exits non-zero with a message on
  stderr, as *Three failure tiers* requires; one while writing can leave a partial table. The
  program never opens a file for writing.
- `main.rs` maps every failure (usage, opening the input, `RunError`, writing the output) to an
  `ExitCode`.

## Consequences

- Easier: the output can be redirected or piped without filtering.
- Worse: the table appears only once the input ends.
- Worse: a caller learns why a run failed only from stderr.
- Deferred: other output formats, as other sinks.
