# stage-1-domain — the ledger's vocabulary, with amounts parsed once into exact money

Plan of record: `tasks/todo.md`

## Context

No product code exists. This ticket admits `rust_decimal`, reworks `make deps` to gate the build
graph instead of `Cargo.lock`, and adds the domain types: ids, `Amount`, `Balance`, the account
and history types, commands, events, reasons and sights. Four laws hold `Amount` and `Balance`; the
other types are declarations whose behaviour the core's laws pin. It lands as one commit,
`feat(domain): …`, together with this plan and its tickets.

Records that govern it: *csv, serde and rust_decimal as dependencies*, *State changes only through
events*, *Correctness defined by laws and a model*, *A domain model in the records' language*,
*Entities identified by their keys*; design records *Exact decimal amounts with four places* and
*A per-deposit dispute lifecycle*. The `domain-modeling` skill applies to every type it adds.

## Phase 0 — evidence (read-only)

- [x] `git log --oneline -1` — the amended records commit, `12a4c3f`; `git show --stat HEAD`
      lists both domain-model records, which read `Accepted`, and the `domain-modeling` and
      `curry-howard` skills.
- [x] `make ci` on the branch, before any change — green.
- [x] `tasks/todo.md`, the four tickets and `docs/prompts/0002-stage-1-planning.md` are present
      and untracked, and `tasks/lessons.md` is modified. If any check here fails, stop and ask the
      person.
- [x] A scratch workspace with the same toolchain, edition, resolver and manifest (outside the
      repository) established:
  - `rust_decimal` resolves to 1.43.0, the newest release (`rust-version` 1.67.1); nothing is
    held back.
  - `cargo tree --workspace --locked --edges normal,build,dev --prefix none --format '{p}'`, with
    and without `--target all`, prints `backend`, `rust_decimal`, `arrayvec`, `num-traits`,
    `autocfg`, which is what `cargo build` compiles. `autocfg` is `num-traits`' build dependency.
  - `Cargo.lock` gains 53 entries under 48 names; 43 names are never built (the `std` feature's
    weak `dep?/std` entries reach the lock resolver only).
  - With `csv` and `serde` (`derive`) added too, the graph is exactly the 17 names
    `make deps` admits, with and without `--target all`.
  - `cargo tree --offline` and `cargo metadata --offline` fail on a cold cache: they read every
    reachable manifest from downloaded crates. The gate runs online, with `--locked`.
  - `Decimal::try_from_i128_with_scale(i128, u32) -> Result<Decimal, Error>` exists;
    `rust_decimal::Error` is not `#[non_exhaustive]`.
  - Negative zero comes only from unary `Neg`: `checked_add` and `checked_sub` return `+0` for
    every cancellation. A negative zero prints `-0.0000` with `{:.4}`.
  - `{:.4}` pads with zeros and truncates past four places; `1.5 == 1.500` and they hash alike.
  - `checked_add` is `None` only when the whole part overflows; near the top of the range it
    rounds silently (`MAX + 0.0001 == MAX`). Below 7.9 × 10^24 every four-place sum is exact.
  - `Decimal::from_str` accepts `1_000`, `+1`, `.5`, `1.` and `1e3`, and rounds past 28 places,
    so `Amount` never calls it.
  - A module named `core` passes the workspace's lints; only a bare `core::` path at the crate
    root would resolve to it.
- [x] The Phase 1 `deps` recipe, run in that scratch workspace under `sh` and `dash`: exit 0 and
      no output with the admitted set; with `autocfg` left out, `The build graph holds crates no
      decision record admits:` then `autocfg`, exit 2; with a stale `Cargo.lock`, cargo's
      `needs to be updated but --locked was passed`, exit 2.

## Phase 1 — admit `rust_decimal`; gate the build graph

Files: `backend/Cargo.toml`, `Cargo.lock`, `Makefile`, `CLAUDE.md`, `README.md`,
`.claude/skills/review-and-planning/SKILL.md`.

- [x] `backend/Cargo.toml`, by hand:
      `[dependencies]` then `rust_decimal = { version = "1", default-features = false, features = ["std"] }`.
      `cargo check --workspace` writes `Cargo.lock`.
- [x] `Makefile`: replace `deps` (recipe lines start with a tab; the variable's continuation line
      with spaces):

```make
# The crates the decision records admit, besides the workspace's own packages.
ADMITTED := arrayvec autocfg csv csv-core itoa memchr num-traits proc-macro2 quote rust_decimal \
    ryu serde serde_core serde_derive syn unicode-ident

deps: ## Fail when the build graph holds a crate no decision record admits
	@graph=$$($(CARGO) tree --workspace --locked --edges normal,build,dev --target all --prefix none --format '{p}') || exit 1; \
	members=$$($(CARGO) tree --workspace --locked --depth 0 --prefix none --format '{p}') || exit 1; \
	allowed=" $$(printf '%s\n' "$$members" | awk 'NF {printf "%s ", $$1}')$(ADMITTED) "; \
	outside=$$(printf '%s\n' "$$graph" | awk 'NF {print $$1}' | sort -u | while read -r crate; do \
		case "$$allowed" in (*" $$crate "*) ;; (*) echo "$$crate" ;; esac; \
	done); \
	if [ -n "$$outside" ]; then printf 'The build graph holds crates no decision record admits:\n%s\n' "$$outside"; exit 1; fi
```

- [x] Red: delete `autocfg` from `ADMITTED` by hand; `make deps` must fail naming `autocfg`.
      Restore it; `make deps` passes silently. Record both outputs below.
- [x] `cargo tree --workspace --locked --edges normal,build,dev --target all --prefix none --format '{p}'`
      prints the five packages of Phase 0; record it below.
- [x] `CLAUDE.md` § Dependencies: `make deps` fails when the build graph (`cargo tree`, every
      dependency kind, every target) holds a package that is neither a workspace member nor an
      admitted crate; it reads the graph, not `Cargo.lock`, which also lists optional crates that
      are never built. The layout's `Cargo.lock` line says the same in a few words.
- [x] `README.md`: `rust_decimal` is in use; `csv` and `serde` arrive with the CSV adapters.
- [x] `review-and-planning` skill, § A new dependency: `make deps` gates the build graph, and an
      approved crate is admitted by adding it to the admitted set with its record, never by
      weakening the check.

## Phase 2 — the harness and the money laws (red)

Files: `backend/src/lib.rs`, `backend/src/domain.rs`, `backend/src/domain/money.rs`,
`backend/tests/it/main.rs`, `backend/tests/it/support.rs`, `backend/tests/it/support/generate.rs`,
`backend/tests/it/support/model.rs`, `backend/tests/it/domain.rs`.

- [x] `lib.rs`: a one-line module doc and `pub mod domain;`. `domain.rs`: module doc,
      `mod money;` and `pub use money::{Amount, AmountError, Balance};`.
- [x] `money.rs` with the `Amount`, `AmountError` and `Balance` items of `tasks/todo.md`
      § Interfaces, and lint-clean stubs: `from_str` answers `Err(AmountError::Malformed)`,
      `checked_add` and `checked_sub` answer `None`, both `Display`s write `0.0000`.
- [x] `support.rs`: the harness block of `references/harness.md`, verbatim, with
      `pub(crate) mod generate;` and `pub(crate) mod model;` after the module doc;
      `tests/it/main.rs`: module doc, `mod support;`, `mod domain;`.
- [x] `support/generate.rs`: `amount_spelling`, `amount_value`, `any_text`, `balance_moves` and
      `Direction`, as `tasks/todo.md` § Generators gives them. Non-ASCII characters are written as
      escapes (`'\u{663}'`, an Arabic-Indic three; `'\u{ff11}'`, a full-width one; `'\u{feff}'`).
      `AMOUNT_EDGES`:

```rust
pub(crate) const AMOUNT_EDGES: [&str; 27] = [
    "", "0", "0.0000", ".", "1.", ".5", "+1", "-1", "1e3", "1_000", " 1", "1 ", "\u{663}",
    "0.0001", "1", "1.5", "1.50000", "1.23456", "0001.5000", "1.00001",
    "999999999999999.9999", "999999999999999.99990", "1000000000000000", "0999999999999999",
    "00000000000000000000000000000000000000001", "1.000000000000000000000000000000",
    "18446744073709551616",
];
```

- [x] `support/model.rs`: the grammar read character by character, apart from the production
      parser, and `Units`, the model's money:

```rust
/// What the amount grammar makes of `text`: its value in units of 0.0001, never zero, or why it
/// is refused.
pub(crate) fn amount_units(text: &str) -> Result<NonZeroU64, AmountError> {
    let mut whole = String::new();
    let mut fraction = String::new();
    let mut point = false;
    for character in text.chars() {
        match character {
            '0'..='9' if point => fraction.push(character),
            '0'..='9' => whole.push(character),
            '.' if !point => point = true,
            _ => return Err(AmountError::Malformed),
        }
    }
    if whole.is_empty() || (point && fraction.is_empty()) {
        return Err(AmountError::Malformed);
    }
    let significant_whole = whole.trim_start_matches('0');
    let significant_fraction = fraction.trim_end_matches('0');
    if significant_fraction.len() > 4 {
        return Err(AmountError::TooPrecise);
    }
    if significant_whole.len() > 15 {
        return Err(AmountError::TooLarge);
    }
    let units = format!("{significant_whole}{significant_fraction:0<4}")
        .parse::<u64>()
        .map_err(|_| AmountError::TooLarge)?;
    NonZeroU64::new(units).ok_or(AmountError::Zero)
}

/// An amount of money in units of 0.0001, signed: the model's money.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Units(i128);

impl Units {
    pub(crate) const ZERO: Self = Self(0);

    /// The sum, saturating at the bounds of `i128`, far past any amount the cap admits.
    pub(crate) fn plus(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// The difference, saturating likewise.
    pub(crate) fn minus(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl From<NonZeroU64> for Units {
    fn from(units: NonZeroU64) -> Self {
        Self(i128::from(units.get()))
    }
}

/// As a balance prints: a sign when negative, then exactly four decimals.
impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:04}", magnitude / 10_000, magnitude % 10_000)
    }
}
```

- [x] `tests/it/domain.rs`: the four laws of Phase 4.
- [x] Red: `cargo test --test it domain::` against the stubs — the forward, reverse and balance
      laws fail with counterexamples (the stub refuses valid text and cannot add); the totality
      law passes, so its red is its break in Phase 4. Record the failure lines.

## Phase 3 — `Amount` and `Balance` (green)

File: `backend/src/domain/money.rs`.

```rust
/// Decimal places an amount may carry, and that every amount and balance prints.
const PLACES: usize = 4;
/// Whole-part digits an amount may have: amounts stay below 10^15.
const WHOLE_DIGITS: usize = 15;

impl FromStr for Amount {
    type Err = AmountError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = text.split_once('.').unwrap_or((text, "0"));
        if !is_digits(whole) || !is_digits(fraction) {
            return Err(AmountError::Malformed);
        }
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        if fraction.len() > PLACES {
            return Err(AmountError::TooPrecise);
        }
        if whole.len() > WHOLE_DIGITS {
            return Err(AmountError::TooLarge);
        }
        let mantissa = whole
            .chars()
            .chain(fraction.chars())
            .try_fold(0_u64, |value, digit| {
                value.checked_mul(10)?.checked_add(u64::from(digit.to_digit(10)?))
            })
            .ok_or(AmountError::TooLarge)?;
        if mantissa == 0 {
            return Err(AmountError::Zero);
        }
        let scale = u32::try_from(fraction.len()).map_err(|_| AmountError::TooPrecise)?;
        Decimal::try_from_i128_with_scale(i128::from(mantissa), scale)
            .map(Self)
            .map_err(|_| AmountError::TooLarge)
    }
}

/// Whether `text` is one or more ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
```

- `"1"` reads as whole `1`, fraction `0`; `"1."` has an empty fraction and is `Malformed`. At most
  nineteen significant digits reach the fold, so it never overflows a `u64`, and `2^96` is never
  reached; the two `map_err` arms are the total spelling of those facts.
- Both `Display`s are `write!(f, "{:.*}", PLACES, self.0)`. `Balance` is `checked_add` /
  `checked_sub` of the decimals, wrapped; `From<Amount>` wraps the amount's decimal. No guard
  against negative zero: no operation here negates.
- `AmountError` gets `Display` (`not digits with an optional fraction`, `more than four decimal
  places`, `10^15 or more`, `zero`) and `error::Error`.
- [x] Verify: `cargo test --test it domain::` — 4 passed.

## Phase 4 — the laws, and each one broken once

File: `backend/tests/it/domain.rs`, 500 cases each.

| Test | Label (the law) | Holds when | Deliberate break |
|---|---|---|---|
| `prop_amount_text_parses_back_to_its_value` | an amount's four-decimal text parses back to it; zero and 10^15 up are refused | for `(whole, fraction)`, `format!("{whole}.{fraction:04}")` parses to an amount printing that text and re-parsing to itself, or is refused as `Zero` (both zero) or `TooLarge` (`whole >= 10^15`) | the size check refuses fifteen whole digits (`>=`) |
| `prop_amount_spelling_parses_exactly_when_the_grammar_admits_it` | a spelling parses exactly when the grammar admits it, and prints as its four-decimal value | `text.parse::<Amount>().map(\|a\| a.to_string()) == model::amount_units(text).map(\|u\| Units::from(u).to_string())` | `from_str` strips a leading `+` first |
| `prop_amount_parsing_never_panics` | parsing any text answers, the same each time, and never panics | over `any_text`, `text.parse::<Amount>() == text.parse::<Amount>()`; the harness turns a panic into a counterexample | `from_str` reads `text.as_bytes()[0]` before anything else |
| `prop_balance_never_prints_negative_zero` | a balance prints its exact value with four decimals, and zero as 0.0000, never -0.0000 | each move's text parses with both `Amount::from_str` and `model::amount_units`; folding the moves from `Balance::ZERO` (`checked_add` for a credit, `checked_sub` for a debit) and from `Units::ZERO` (`plus`, `minus`), every step is `Some` and its `to_string()` equals the units' `to_string()`; a refused text or a `None` fails the law | `checked_sub` computes `-(other - self)`, which is a negative zero when the two are equal |

- [x] Stage this ticket's files first (`git add` with explicit paths), so a break in a new file
      shows in `git diff`. Then for each row: apply the break by editing, run
      `cargo test --test it <test>`, paste the failure line into Law evidence, undo the break by
      editing, and confirm `git diff --quiet` exits 0. Never `git checkout` or `git stash`
      mid-step, and never undo by copying a saved file back: the copy keeps its old modification
      time, and cargo reuses the broken build.

## Phase 5 — the rest of the vocabulary

Files: `backend/src/domain.rs`, `backend/src/domain/id.rs`, `backend/src/domain/ledger.rs`,
`backend/src/domain/command.rs`, `backend/src/domain/event.rs`.

- [x] `ClientId`, `TxId`, `Address`, `Account`, `Lock`, `Deposit`, `Status`, `Footprint`,
      `Command`, `ClientCommand`, `DisputeCommand`, `Event`, `ClientEvent`, `DisputeEvent`,
      `Reason`, `ClientReason`, `DisputeReason`, `Sight` exactly as `tasks/todo.md` § Interfaces
      gives them, each with a doc comment; `domain.rs` re-exports every one.
- [x] No tests here: they are declarations, and a test of construction or derives is banned. The
      core's laws pin their behaviour, `Account::total` included. Fields are private, read
      through methods; `Account::new`, `Account::lock` and `Deposit::new` arrive with `evolve`,
      their first user, so `dead_code` stays quiet. A public struct with private fields that
      nothing builds yet raises no warning: a probe under the workspace lints passed clean.
- [x] Verify: `make lint` — no warning (`unreachable_pub` proves every item is re-exported).
- [x] The `domain-modeling` and `curry-howard` review questions have an answer for every type this
      ticket adds, written in this ticket's Review.

## Phase 6 — sync and land

- [x] `docs/overview.md` § Terms gains *amount refusal*: why text is not an amount, as
      `AmountError` names it (malformed, too precise, too large, zero), in the same commit as the
      type, as *A domain model in the records' language* requires.
- [x] `CLAUDE.md` § Status and `README.md` § Status: the library is being built — domain first,
      then the core, the runtime and the in-memory stores; the command-line program follows.
- [x] `CLAUDE.md` and `README.md` layouts: `domain.rs` and `domain/`, and `tests/it/`.
- [x] `git grep -n rust_decimal backend/src` lists only `backend/src/domain/money.rs`.
- [x] Code-reviewer agent over the staged diff; act on every blocker and should-fix.
- [x] The two forbidden-name greps of `tasks/todo.md` § Global constraints: both silent.
- [x] `make ci` — green; paste the test summary line here.
- [x] Tick this ticket and its line in `tasks/todo.md`; stage explicit paths (this plan, all four
      tickets and `tasks/lessons.md` included); commit with `git commit -F` as `tasks/todo.md`
      § Commits gives it.

## Gate evidence

| Check | Command | Output |
|---|---|---|
| The records commit | `git log --oneline -1`; `git show --name-only HEAD` | `12a4c3f docs(engine): record the architecture, design records and roadmap`; both domain-model records, `Accepted`, and both skills |
| The gate before any change | `make ci` | green, no tests yet |
| The plan's files | `git status --short` | the plan, the four tickets and `0002` untracked; `tasks/lessons.md` modified |
| `rust_decimal` resolved | `cargo check --workspace` | `rust_decimal v1.43.0`; `Cargo.lock` holds 53 entries under 48 names |
| The build graph | `cargo tree --workspace --locked --edges normal,build,dev --target all --all-features --prefix none --format '{p}'` | `backend v0.1.0 (…/backend)`, `rust_decimal v1.43.0`, `arrayvec v0.7.8`, `num-traits v0.2.19`, `autocfg v1.5.1`; the same without `--target all` |
| `deps`, red | `make deps` with `autocfg` left out of `ADMITTED` | `The build graph holds crates no decision record admits:` then `autocfg`, exit 2; shown by hand-editing, and again on the final recipe |
| `deps`, green | `make deps` | no output, exit 0 |
| Red, against the stubs | `cargo test --test it domain::` | `test result: FAILED. 1 passed; 3 failed`: the value law at case 1, the spelling law at case 1 with `"1"`, the balance law at case 1; totality passed, so its red is its break |
| Green | `cargo test --test it domain::` | `test result: ok. 4 passed; 0 failed` |
| Fresh seeds | `cargo test -q --test it domain::`, fifty runs | no failure |
| Containment | `git grep -n --untracked rust_decimal backend/src` | `backend/src/domain/money.rs:7:use rust_decimal::Decimal;` |
| Code review | code-reviewer agent over the staged diff | no blocker; three should-fixes and four nits, acted on as the Review says |
| Forbidden names | both greps of `tasks/todo.md` § Global constraints | silent |
| Record citations | the record scan of § Per-commit gate | nothing |
| The gate | `make ci` | green: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |

## Law evidence

| Law | Break | Command | Failure line |
|---|---|---|---|
| `prop_amount_text_parses_back_to_its_value` | the size check refuses fifteen whole digits (`>=`) | `cargo test --test it prop_amount_text_parses_back_to_its_value` | ``law `an amount's four-decimal text parses back to it; zero and 10^15 up are refused` failed at case 20 with AmountValue { whole: 322763114549985, fraction: 8763 }; replay: PROPERTY_SEED=0x18dd4d2708112ca0`` |
| `prop_amount_spelling_parses_exactly_when_the_grammar_admits_it` | `from_str` strips a leading `+` first | `cargo test --test it prop_amount_spelling_parses_exactly_when_the_grammar_admits_it` | ``law `a spelling parses exactly when the grammar admits it, and prints as its four-decimal value` failed at case 261 with "+1"; replay: PROPERTY_SEED=0x18dd4bd58b5675f0`` |
| `prop_amount_parsing_never_panics` | `from_str` reads `text.as_bytes()[0]` before anything else | `cargo test --test it prop_amount_parsing_never_panics` | ``law `parsing any text answers, the same each time, and never panics` failed at case 1 with ""; replay: PROPERTY_SEED=0x18dd4bd7f0313918`` |
| `prop_balance_never_prints_negative_zero` | `checked_sub` computes `-(other - self)` | `cargo test --test it prop_balance_never_prints_negative_zero` | ``law `a balance prints its exact value with four decimals, and zero as 0.0000, never -0.0000` failed at case 5 with [(Credit, "816.2468"), (Debit, "816.2468"), …]; replay: PROPERTY_SEED=0x18dd4bd979f6eae8`` |

Each break was applied and undone by an in-place edit of the staged `money.rs`; `git diff --quiet`
exited 0 before and after every one.

## Blocked / handed off

- [x] None.

## Review

`rust_decimal` is admitted and contained in `money.rs`; `make deps` reads the build graph. The
vocabulary is in place, and four laws hold `Amount` and `Balance`, each shown failing once
against its break. Forty further runs of the domain laws, each on a fresh seed, found nothing.

Rulings:

- rustfmt lays `AMOUNT_EDGES` out one spelling per line. The 27 spellings and their order are
  this ticket's.
- `make deps` reads the graph with `--all-features` as well, since `make lint` builds every
  feature: the gate reads the graph the commit gate builds. The output is unchanged, and the
  red was shown again with it.
- `amount_value` yields a named `AmountValue { whole, fraction }` instead of a `(u64, u64)`
  tuple, whose two positions of one type would swap silently. `balance_moves` keeps
  `(Direction, String)`, whose types cannot swap.
- `impl Display for Amount` stays. It is in § Interfaces, it is the inverse of the type's only
  constructor (the round trip `rust-properties` asks for), and it shows nothing the public API
  hides: `Balance::from(amount)` prints the same value.

Code review of the staged diff: no blocker. Of the three should-fixes, two were fixed (the
`--all-features` flag and `AmountValue`), and the third, dropping `Display for Amount`, was
declined as ruled above. Two nits were fixed: `Account::available`'s doc now holds after a
chargeback, and the dependency section of `review-and-planning` no longer promises checks that
do not exist. Two nits were declined: the term *amount refusal* is the plan's, and the gate's
name-only match, which a non-registry source with an admitted name would pass, shows in the
manifest diff every review reads.

Review questions, `domain-modeling` (DM) and `curry-howard` (CH), for every type this ticket adds:

| Type | Name (DM 1) and kind (DM 2) | Proposition, and what it rules out (CH 1, 2) | Keys, absences, derivations (DM 3–5, CH 3, 6) | What only a law states (CH 7) |
|---|---|---|---|---|
| `ClientId`, `TxId` | Terms' *client* and *tx id*; values | any `u16` names a client and any `u32` a transaction; neither stands in for the other | no key, no absence | — |
| `Address` | Terms' *address*; value | a client and a tx id from one command, never two | it is the key: stored beside values, never inside one | — |
| `Amount` | *Exact decimal amounts with four places*; value | a decimal that is positive, exact at four places and below 10^15, built only by parsing text | no absence | parsing agrees with the grammar, and printing with parsing: the spelling and value laws |
| `AmountError` | Terms' *amount refusal*; value | one of four refusals, the first that applies | no absence | the order of the checks: the spelling law, against the model |
| `Balance` | the same record's balance; value | a signed decimal, changed only by checked arithmetic | `checked_*` answer `None` on overflow, their one meaning | never `-0.0000`: the balance law |
| `Account` | an account (overview § Accounts); entity | available, held and a lock, with the total derived | its client id is its key, not a field; `total()` is derived; no `Option` | held, total and locked against history: the core's laws |
| `Lock` | § Accounts' *locked*; value | locked or not, named where a `bool` would hide it | — | only a chargeback locks: the core's lock law |
| `Deposit` | Terms' *history*; entity | an amount and its status | its client and tx id are its key, not fields | — |
| `Status` | *A per-deposit dispute lifecycle*; value | settled or disputed; a removed deposit is absent, not a third status | — | moves only as the lifecycle allows: the core's laws |
| `Footprint` | Terms' *footprint*; value | an account and at most one deposit, at an address held outside it | one `None`: no deposit of this client under that id, never recorded or charged back, which is the same to every reader | — |
| `Command`, `ClientCommand`, `DisputeCommand` | Terms' *client command* and *dispute command*; messages | a client command carries an amount and a dispute command none, each with its address | an amount's presence is the family, not an `Option` | — |
| `Event`, `ClientEvent`, `DisputeEvent` | Terms' *event*, with § Accounts' names; messages | an accepted fact carrying what `evolve` needs, and no id | no key | conservation of money: the core's laws |
| `Reason`, `ClientReason`, `DisputeReason` | Terms' *rejection*, with § Commands' reasons; values | why a command was refused, per family, with `Overflow` apart | — | the order of the checks: the core's decision laws |
| `Sight` | *Tx ids consumed on first sight*; value | first or repeat, named where a `bool` would hide it | — | the first sight wins: run's and memory's laws |
| `Units` (test) | the model's money | an exact, signed count of 0.0001 | — | — |
| `Direction` (test) | a move's way | a credit or a debit, named where a `bool` would hide it | — | — |
| `AmountValue` (test) | an amount's value | a whole part and a fraction, each by name | the fraction's bound, below `10_000`, is the generator's | — |

For every type: no file format's column, kind string or optional amount reaches `domain` (DM 6);
every method reads and none changes its value (DM 7); no `bool`, tuple or bare primitive hides a
name (CH 4), the one tuple left being `balance_moves`' `(Direction, String)`, whose types name
both parts; no function's return admits a case it never produces (CH 5).
