# 8. The command-line contract with help and version

## Status

Accepted

## Motivation

Supersedes *The command-line contract*. It took exactly one argument and treated any other count
as a usage error, so `--help` and `--version` were read as paths that could not be opened, and a
usage error showed the usage without saying what was wrong.

Complies with *Effects behind ports*, *Three failure tiers* and *csv, serde, rust_decimal,
thiserror, clap, proptest and rand as dependencies*.

A caller sees only the process: its arguments, its two output streams and its exit status. They
must stay stable, and stdout must carry nothing but what the caller asked for.

## Decision

- `main.rs` reads the arguments once, and `clap` parses them: one positional argument, the input
  path, shown as `<input.csv>`; `-h` and `--help`; `-V` and `--version`. `--` ends the flags, so
  any path, even one that begins with `-`, can follow it. Parsing reads nothing but those
  arguments, neither the environment nor the terminal, and the help, the version and every usage
  line name the program `cthulhu`, whatever path ran it.
- `-h` or `--help` writes the help to stdout, and `-V` or `--version` writes `cthulhu` and its
  version; either opens no file and exits 0.
- No path, a second argument, an empty path or an unknown flag is a usage error: clap's message,
  which says what was wrong and points to `--help`, on stderr, and exit status 2.
- stdout: the header `client,available,held,total,locked`, then one row per account by client id,
  balances as *Exact decimal amounts with four places* prints them, `locked` as `true` or
  `false`; or, when asked for, the help or the version instead.
- stderr: counts of rejected commands by reason, and of malformed rows by kind.
- The table is written only after the run succeeds. Any failure exits non-zero with a message on
  stderr, as *Three failure tiers* requires; one while writing can leave a partial table. The
  program never opens a file for writing.
- `main.rs` maps every failure (usage, opening the input, `RunError`, writing the table, the
  report, the help or the version) to an `ExitCode`; a failed write of the help or the version
  exits as every failed write does.

## Consequences

- Easier: the output can be redirected or piped without filtering.
- Easier: a caller can ask the binary how to run it, and a usage error says what was wrong.
- Worse: the table appears only once the input ends.
- Worse: a caller learns why a run failed only from stderr.
- Worse: an argument that begins with `-`, other than `-` itself, is read as a flag, so such a
  path needs `--` before it; an empty path, which failed to open, is now a usage error.
- Worse: clap words the usage errors and lays out the help; the binary's fixtures pin both, so an
  update to clap that changes them fails them.
- Deferred: other output formats, as other sinks.
- Deferred: settings beyond the input path. They would come as `CTHULHU_*` environment variables,
  parsed once from a snapshot `main` takes, so the command line keeps its one input argument.
