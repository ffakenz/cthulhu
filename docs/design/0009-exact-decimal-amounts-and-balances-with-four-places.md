# 9. Exact decimal amounts and balances with four places

## Status

Accepted

## Motivation

Supersedes *Exact decimal amounts with four places*. Its balances changed only by checked
arithmetic, and its claim was that every reachable balance stays exact. But `Decimal`'s checks fail
only once a value outgrows its 96-bit mantissa, near 7.9 × 10^28, while four decimal places stay
exact only below about 7.9 × 10^24. Between the two:
- a sum rounds silently;
- from 10^27, printing a balance with four decimals panics, because the decimal type formats into
  32 bytes.

Valid input never gets there, but `Balance`'s own operations do.

Complies with *State changes only through events* and *csv, serde, rust_decimal, thiserror, clap,
proptest and rand as dependencies*.

Balances must be exact to four decimal places. Floating point rounds them, and a decimal type
rounds silently once a value outgrows its 96-bit mantissa.

## Decision

- `Amount`: a positive `Decimal`, exact at four decimal places once trailing zeros are dropped,
  and below 10^15. Its only constructor parses text of the form `digits[.digits]`. `1`, `1.5` and
  `1.50000` parse; `0`, `-1`, `1.23456`, `1e3`, `+1` and `.5` do not.
- `Balance`: a signed `Decimal` for `available`, `held` and `total`. Only `Balance::checked_add`
  and `Balance::checked_sub` change it, and each returns `None` once its result leaves
  ±(2^96 − 1) × 10^−4: the range where `Decimal` holds four places. So every balance is exact.
- Output prints every balance with exactly four decimals; zero prints `0.0000`, never `-0.0000`.
- Governs: `Amount`, `AmountError` and `Balance` in `libs/engine/src/domain/money.rs`.

## Consequences

- Easier: every balance is exact by construction, and printing one never fails.
- Easier: 2^32 amounts below 10^15 sum below 4.3 × 10^24, inside the bound, so valid input never
  meets it.
- Worse: since valid input never meets the bound, no run reaches the `Overflow` rejection or the
  table's overflowing total. Only `Balance`'s own operations reach the bound.
- Worse: an amount of 10^15 or more, or one not exact at four places, makes its row malformed
  instead of being rounded.
- Rejected:
  - `f64`: inexact;
  - an `i64` in units of 0.0001: it tops out at 9.2 × 10^14, so sums over many deposits overflow
    at ordinary amounts;
  - a `Display` that never panics: it would still print balances that had rounded;
  - an `i128` in units of 0.0001: exact without a bound, but replacing `Decimal` throughout needs
    a dependency record of its own.
- Deferred: rounding inputs to four places. That needs a rounding mode, and a record of its own.
