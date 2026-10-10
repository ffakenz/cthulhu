# 1. Exact decimal amounts with four places

## Status

Accepted

## Motivation

Complies with *State changes only through events* and *csv, serde and rust_decimal as
dependencies*.

Balances must be exact to four decimal places. Floating point rounds them, and a decimal type
rounds silently once a value outgrows its 96-bit mantissa.

## Decision

- `Amount`: a positive `Decimal`, exact at four decimal places once trailing zeros are dropped,
  and below 10^15. Its only constructor parses text of the form `digits[.digits]`: `1`, `1.5` and
  `1.50000` parse; `0`, `-1`, `1.23456`, `1e3`, `+1` and `.5` do not.
- `Balance`: a signed `Decimal` for `available`, `held` and `total`, changed only by checked
  arithmetic.
- Output prints every balance with exactly four decimals; zero prints `0.0000`, never `-0.0000`.

## Consequences

- Easier: every reachable balance is exact. 2^32 amounts below 10^15 sum below 4.3 × 10^24,
  inside the range where `Decimal` is exact at four places (7.9 × 10^24).
- Worse: the overflow branch of the checked arithmetic is unreachable through valid input, so no
  test reaches it without bypassing `Amount`.
- Worse: an amount of 10^15 or more, or one not exact at four places, makes its row malformed
  instead of being rounded.
- Rejected: `f64` (inexact); an `i64` in units of 0.0001 (it tops out at 9.2 × 10^14, so sums
  over many deposits overflow at ordinary amounts).
- Deferred: rounding inputs to four places; that needs a rounding mode, and a record of its own.
