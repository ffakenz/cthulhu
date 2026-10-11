//! Generators: raw input as a source would deliver it, one draw in four at an edge.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;

use backend::domain::{
    Address, Amount, ClientCommand, ClientEvent, ClientId, Command, DisputeCommand, DisputeEvent,
    Event, TxId,
};
use backend::run::{Row, RowError};

use super::{EDGE_ODDS, Gen};

/// Amount spellings at the edges of the grammar: empty, zero, a bare or trailing point, a sign, an
/// exponent, a separator, padding and a non-ASCII digit; then amounts that parse or just miss:
/// the smallest, trailing and leading zeros, five decimals, the largest and one past it, and past
/// `u64`.
pub(crate) const AMOUNT_EDGES: [&str; 27] = [
    "",
    "0",
    "0.0000",
    ".",
    "1.",
    ".5",
    "+1",
    "-1",
    "1e3",
    "1_000",
    " 1",
    "1 ",
    "\u{663}",
    "0.0001",
    "1",
    "1.5",
    "1.50000",
    "1.23456",
    "0001.5000",
    "1.00001",
    "999999999999999.9999",
    "999999999999999.99990",
    "1000000000000000",
    "0999999999999999",
    "00000000000000000000000000000000000000001",
    "1.000000000000000000000000000000",
    "18446744073709551616",
];

/// One draw in two.
const EVEN_ODDS: NonZeroU64 = NonZeroU64::MIN.saturating_add(1);

/// The ASCII digits.
const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// What an amount's text may hold by mistake: digits, the characters number parsers treat
/// specially, and an Arabic-Indic three.
const AMOUNT_CHARACTERS: [char; 18] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '-', '+', 'e', ' ', '_', ',', '\u{663}',
];

/// Raw amount text: one draw in four an edge; otherwise, as often each, a number-shaped spelling
/// (up to seventeen whole digits, leading zeros included, then an optional point and up to six
/// fraction digits) or up to eight characters of `AMOUNT_CHARACTERS`.
pub(crate) fn amount_spelling(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&AMOUNT_EDGES).copied().unwrap_or_default().to_owned();
    }
    if random.one_in(EVEN_ODDS) {
        let whole: String = random.up_to(17, &DIGITS).into_iter().collect();
        if random.one_in(EVEN_ODDS) {
            let fraction: String = random.up_to(6, &DIGITS).into_iter().collect();
            return format!("{whole}.{fraction}");
        }
        return whole;
    }
    random.up_to(8, &AMOUNT_CHARACTERS).into_iter().collect()
}

/// Shifts a draw may take: every bit width of a `u64`.
const BITS: NonZeroU64 = NonZeroU64::MIN.saturating_add(63);

/// Fractions in units of 0.0001: `0..10_000`.
const FRACTIONS: NonZeroU64 = NonZeroU64::MIN.saturating_add(9_999);

/// An amount's value as a whole part and a fraction.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AmountValue {
    /// The whole part.
    pub(crate) whole: u64,
    /// The fraction, in units of 0.0001: below `10_000`.
    pub(crate) fraction: u64,
}

/// Values at the edges: zero, the smallest amount, one, the largest fraction, the largest amount,
/// 10^15, and the largest `u64`.
const AMOUNT_VALUE_EDGES: [AmountValue; 7] = [
    AmountValue { whole: 0, fraction: 0 },
    AmountValue { whole: 0, fraction: 1 },
    AmountValue { whole: 1, fraction: 0 },
    AmountValue { whole: 0, fraction: 9_999 },
    AmountValue { whole: 999_999_999_999_999, fraction: 9_999 },
    AmountValue { whole: 1_000_000_000_000_000, fraction: 0 },
    AmountValue { whole: u64::MAX, fraction: 9_999 },
];

/// An amount's value: one draw in four an edge, otherwise a whole part of random magnitude.
pub(crate) fn amount_value(random: &mut Gen) -> AmountValue {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&AMOUNT_VALUE_EDGES).copied().unwrap_or_default();
    }
    let whole = random.next_u64() >> random.below(BITS);
    AmountValue { whole, fraction: random.below(FRACTIONS) }
}

/// Characters for totality: ASCII digits, punctuation and letters, and multi-byte characters (an
/// Arabic-Indic three, a full-width one, an accented letter and a byte-order mark).
const ANY_CHARACTERS: [char; 18] = [
    '0', '1', '5', '9', '.', ',', '-', '+', '_', ' ', 'e', 'E', 'x', 'a', '\u{663}', '\u{ff11}',
    '\u{e9}', '\u{feff}',
];

/// Any text up to sixteen characters: one draw in four an edge (empty, sixty-four zeros then a
/// one, a multi-byte character after a point).
pub(crate) fn any_text(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        let edges = [String::new(), format!("{}1", "0".repeat(64)), "1.\u{ff11}".to_owned()];
        return random.pick(&edges).cloned().unwrap_or_default();
    }
    random.up_to(16, &ANY_CHARACTERS).into_iter().collect()
}

/// Which way a move takes a balance.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Direction {
    /// The amount is added.
    Credit,
    /// The amount is subtracted.
    Debit,
}

impl Direction {
    /// The other way.
    fn opposite(self) -> Self {
        match self {
            Self::Credit => Self::Debit,
            Self::Debit => Self::Credit,
        }
    }
}

/// Lengths of a sequence of moves: `0..=12`.
const MOVES: NonZeroU64 = NonZeroU64::MIN.saturating_add(12);

/// Amounts a move takes at its edges: the smallest and the largest.
const MOVE_EDGES: [&str; 2] = ["0.0001", "999999999999999.9999"];

/// How many amounts lie above zero and below 1000, in units of 0.0001.
const MOVE_UNITS: NonZeroU64 = NonZeroU64::MIN.saturating_add(9_999_998);

/// The units of 0.0001 in 10^15, the smallest value too large to be an amount.
const TOO_LARGE_UNITS: u64 = 10_000_000_000_000_000_000;

/// Up to twelve moves of a balance, each a direction and the text of a valid amount: one draw in
/// four an edge amount, otherwise `{whole}.{fraction:04}` below 1000 and above zero. One draw in
/// four instead repeats the previous amount the other way, so a balance lands on zero from either
/// side; and, while the balance's magnitude is an amount, one draw in four of the rest moves the
/// whole balance back to zero, spelled without trailing zeros, so a balance whose last decimal is
/// zero also lands on zero through an amount spelled with fewer decimals.
pub(crate) fn balance_moves(random: &mut Gen) -> Vec<(Direction, String)> {
    let count = random.below(MOVES);
    let mut moves: Vec<(Direction, String)> = Vec::new();
    for _ in 0..count {
        let repeat = moves
            .last()
            .filter(|_| random.one_in(EDGE_ODDS))
            .map(|(direction, text)| (direction.opposite(), text.clone()));
        let next = repeat
            .or_else(|| {
                moves
                    .iter()
                    .try_fold(0, moved)
                    .and_then(zeroing)
                    .filter(|_| random.one_in(EDGE_ODDS))
            })
            .unwrap_or_else(|| (direction(random), move_amount(random)));
        moves.push(next);
    }
    moves
}

/// `balance`, in units of 0.0001, after the move `(direction, text)`, its amount read through the
/// grammar model; `None` if the model refuses the amount.
fn moved(balance: i128, (direction, text): &(Direction, String)) -> Option<i128> {
    let units = i128::from(super::model::amount_units(text).ok()?.get());
    Some(match direction {
        Direction::Credit => balance.saturating_add(units),
        Direction::Debit => balance.saturating_sub(units),
    })
}

/// The move that takes `balance`, in units of 0.0001, back to zero, its amount spelled without
/// trailing zeros; `None` when the balance is zero or its magnitude is no amount.
fn zeroing(balance: i128) -> Option<(Direction, String)> {
    let magnitude = u64::try_from(balance.unsigned_abs())
        .ok()
        .filter(|units| (1..TOO_LARGE_UNITS).contains(units))?;
    let direction = if balance > 0 { Direction::Debit } else { Direction::Credit };
    Some((direction, unpadded(magnitude)))
}

/// `units` of 0.0001 spelled without trailing zeros: `5` for 50000, `5.12` for 51200.
fn unpadded(units: u64) -> String {
    let (whole, fraction) = (units / 10_000, units % 10_000);
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:04}").trim_end_matches('0').to_owned()
    }
}

/// A credit or a debit, as often each.
fn direction(random: &mut Gen) -> Direction {
    if random.one_in(EVEN_ODDS) { Direction::Credit } else { Direction::Debit }
}

/// The text of a valid amount: one draw in four an edge, otherwise above zero and below 1000.
fn move_amount(random: &mut Gen) -> String {
    if random.one_in(EDGE_ODDS) {
        return random.pick(&MOVE_EDGES).copied().unwrap_or_default().to_owned();
    }
    let units = random.below(MOVE_UNITS).saturating_add(1);
    format!("{}.{:04}", units / 10_000, units % 10_000)
}

/// A row of the input as a source reads it, before parsing: a missing amount is `None`, never
/// `Some("")`, and no field is padded.
#[derive(Debug, Clone)]
pub(crate) enum InputRow {
    /// A row whose fields read: its kind, its client and tx id, and its amount's text, if any.
    Fields { kind: Kind, client: u16, tx: u32, amount: Option<String> },
    /// A row with a field that does not read.
    Unreadable,
    /// A row with more fields than the header names.
    TooManyFields,
}

/// What a row asks for: one of the five commands.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Kind {
    /// A deposit.
    Deposit,
    /// A withdrawal.
    Withdrawal,
    /// A dispute.
    Dispute,
    /// A resolve.
    Resolve,
    /// A chargeback.
    Chargeback,
}

/// A deposit row.
pub(crate) fn deposit(client: u16, tx: u32, amount: &str) -> InputRow {
    InputRow::Fields { kind: Kind::Deposit, client, tx, amount: Some(amount.to_owned()) }
}

/// A withdrawal row.
pub(crate) fn withdrawal(client: u16, tx: u32, amount: &str) -> InputRow {
    InputRow::Fields { kind: Kind::Withdrawal, client, tx, amount: Some(amount.to_owned()) }
}

/// A dispute row, with no amount.
pub(crate) fn dispute(client: u16, tx: u32) -> InputRow {
    InputRow::Fields { kind: Kind::Dispute, client, tx, amount: None }
}

/// A resolve row, with no amount.
pub(crate) fn resolve(client: u16, tx: u32) -> InputRow {
    InputRow::Fields { kind: Kind::Resolve, client, tx, amount: None }
}

/// A chargeback row, with no amount.
pub(crate) fn chargeback(client: u16, tx: u32) -> InputRow {
    InputRow::Fields { kind: Kind::Chargeback, client, tx, amount: None }
}

/// What a source makes of `input`: the command it reads as, or why it is malformed. A client
/// command needs an amount that parses; a dispute command's amount is ignored.
pub(crate) fn row(input: &InputRow) -> Row {
    let (kind, client, tx, text) = match input {
        InputRow::Unreadable => return Row::Malformed(RowError::Unreadable),
        InputRow::TooManyFields => return Row::Malformed(RowError::TooManyFields),
        InputRow::Fields { kind, client, tx, amount } => (kind, client, tx, amount),
    };
    let address = Address::new(ClientId::from(*client), TxId::from(*tx));
    let parsed = || match text.as_deref() {
        None => Err(RowError::MissingAmount),
        Some(spelling) => spelling.parse::<Amount>().map_err(RowError::InvalidAmount),
    };
    let command = match kind {
        Kind::Deposit => {
            parsed().map(|amount| Command::Client(ClientCommand::Deposit { address, amount }))
        }
        Kind::Withdrawal => {
            parsed().map(|amount| Command::Client(ClientCommand::Withdrawal { address, amount }))
        }
        Kind::Dispute => Ok(Command::Dispute(DisputeCommand::Dispute(address))),
        Kind::Resolve => Ok(Command::Dispute(DisputeCommand::Resolve(address))),
        Kind::Chargeback => Ok(Command::Dispute(DisputeCommand::Chargeback(address))),
    };
    match command {
        Ok(command) => Row::Parsed(command),
        Err(error) => Row::Malformed(error),
    }
}

/// The command `input` reads as, `None` for a malformed row.
pub(crate) fn command(input: &InputRow) -> Option<Command> {
    match row(input) {
        Row::Parsed(command) => Some(command),
        Row::Malformed(_) => None,
    }
}

/// The edges a run visits, one draw in four.
#[derive(Debug, Clone, Copy)]
enum Edge {
    /// A deposit or a withdrawal reusing an earlier deposit's or withdrawal's tx id.
    RepeatedTx,
    /// A dispute command naming `u32::MAX`, `4_000_000_000` or a withdrawal's tx id.
    UnknownTx,
    /// A dispute command naming another client's deposit.
    ForeignTx,
    /// On a fresh client: two deposits, a dispute and a chargeback of the first, then up to three
    /// of a deposit, a withdrawal and any dispute command on either deposit.
    AfterChargeback,
    /// On a fresh client: a deposit, maybe a second deposit and its dispute, then a withdrawal of
    /// exactly the first amount, maybe followed by `0.0001` more.
    ExactBalance,
    /// A deposit or a withdrawal of an amount at or past an edge of the grammar.
    BoundaryAmount,
    /// An unreadable row, a row with too many fields, or a deposit or a withdrawal with no amount
    /// or with an amount edge a source delivers.
    MalformedRow,
}

/// Every edge, each as likely.
const EDGES: [Edge; 7] = [
    Edge::RepeatedTx,
    Edge::UnknownTx,
    Edge::ForeignTx,
    Edge::AfterChargeback,
    Edge::ExactBalance,
    Edge::BoundaryAmount,
    Edge::MalformedRow,
];

/// Amount edges no source delivers: it reads an empty field as absent and trims every field.
const UNDELIVERED: [&str; 3] = ["", " 1", "1 "];

/// Every kind, each as likely.
const KINDS: [Kind; 5] =
    [Kind::Deposit, Kind::Withdrawal, Kind::Dispute, Kind::Resolve, Kind::Chargeback];

/// The builders of the three dispute commands.
const DISPUTE_ROWS: [fn(u16, u32) -> InputRow; 3] = [dispute, resolve, chargeback];

/// The clients ordinary rows name.
const CLIENTS: [u16; 3] = [1, 2, 3];

/// The clients repeated and unknown tx ids come from: the ordinary ones and the extremes.
const EDGE_CLIENTS: [u16; 5] = [1, 2, 3, 0, u16::MAX];

/// Tx ids no row records.
const UNKNOWN_TXS: [u32; 2] = [u32::MAX, 4_000_000_000];

/// Amounts at or past an edge of the grammar: the smallest and the largest, 10^15, zero, five
/// decimals, trailing zeros and leading zeros.
const BOUNDARY_AMOUNTS: [&str; 7] =
    ["0.0001", "999999999999999.9999", "1000000000000000", "0", "0.00001", "1.50000", "0001.5"];

/// Wholes of an ordinary amount: `0..100`.
const WHOLES_BELOW_100: NonZeroU64 = NonZeroU64::MIN.saturating_add(99);

/// A choice among four.
const FOUR: NonZeroU64 = NonZeroU64::MIN.saturating_add(3);

/// A choice among three.
const THREE: NonZeroU64 = NonZeroU64::MIN.saturating_add(2);

/// The most rows a run holds.
const MAX_ROWS: usize = 16;

/// Lengths a run aims for: `0..=16`.
const ROW_COUNTS: NonZeroU64 = NonZeroU64::MIN.saturating_add(16);

/// An ordinary amount's text, `whole[.digits]`: a whole below 100 and one to four digits.
fn ordinary_amount(random: &mut Gen) -> String {
    let whole = random.below(WHOLES_BELOW_100);
    if random.one_in(EVEN_ODDS) {
        return whole.to_string();
    }
    let digits: String =
        (0..=random.below(FOUR)).filter_map(|_| random.pick(&DIGITS).copied()).collect();
    format!("{whole}.{digits}")
}

/// Any dispute command by `client` naming `tx`.
fn any_dispute(random: &mut Gen, client: u16, tx: u32) -> InputRow {
    random.pick(&DISPUTE_ROWS).map_or_else(|| dispute(client, tx), |row| row(client, tx))
}

/// Ids no edge names: clients from 100 and tx ids from 0, each handed out once.
#[derive(Debug)]
struct Fresh {
    client: u16,
    tx: u32,
}

impl Fresh {
    /// A client no row has named.
    fn client(&mut self) -> u16 {
        let client = self.client;
        self.client = client.saturating_add(1);
        client
    }

    /// A tx id no row has named.
    fn tx(&mut self) -> u32 {
        let tx = self.tx;
        self.tx = tx.saturating_add(1);
        tx
    }
}

/// The ids a row names.
#[derive(Debug, Clone, Copy)]
struct Named {
    client: u16,
    tx: u32,
}

/// A run being drawn: its rows, the fresh ids left, and the deposits and withdrawals it has
/// drawn on fresh tx ids.
#[derive(Debug)]
struct Draft {
    rows: Vec<InputRow>,
    fresh: Fresh,
    deposits: Vec<Named>,
    withdrawals: Vec<Named>,
}

impl Draft {
    /// No rows yet.
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            fresh: Fresh { client: 100, tx: 0 },
            deposits: Vec::new(),
            withdrawals: Vec::new(),
        }
    }

    /// A deposit row, recorded as a dispute command's target.
    fn push_deposit(&mut self, client: u16, tx: u32, amount: &str) {
        self.rows.push(deposit(client, tx, amount));
        self.deposits.push(Named { client, tx });
    }

    /// A withdrawal row, recorded as an unknown dispute command's target.
    fn push_withdrawal(&mut self, client: u16, tx: u32, amount: &str) {
        self.rows.push(withdrawal(client, tx, amount));
        self.withdrawals.push(Named { client, tx });
    }

    /// A recorded deposit's tx id, any client's, or a fresh one before any deposit.
    fn target(&mut self, random: &mut Gen) -> u32 {
        match random.pick(&self.deposits) {
            Some(named) => named.tx,
            None => self.fresh.tx(),
        }
    }

    /// An ordinary row: any kind, a client from `CLIENTS`, a fresh tx id for a deposit or a
    /// withdrawal, and a recorded deposit's tx id for a dispute command.
    fn ordinary(&mut self, random: &mut Gen) {
        let client = random.pick(&CLIENTS).copied().unwrap_or_default();
        match random.pick(&KINDS).copied().unwrap_or(Kind::Deposit) {
            Kind::Deposit => {
                let tx = self.fresh.tx();
                self.push_deposit(client, tx, &ordinary_amount(random));
            }
            Kind::Withdrawal => {
                let tx = self.fresh.tx();
                self.push_withdrawal(client, tx, &ordinary_amount(random));
            }
            kind @ (Kind::Dispute | Kind::Resolve | Kind::Chargeback) => {
                let tx = self.target(random);
                self.rows.push(InputRow::Fields { kind, client, tx, amount: None });
            }
        }
    }

    /// The rows of `edge`.
    fn edge(&mut self, random: &mut Gen, edge: Edge) {
        match edge {
            Edge::RepeatedTx => self.repeated_tx(random),
            Edge::UnknownTx => self.unknown_tx(random),
            Edge::ForeignTx => self.foreign_tx(random),
            Edge::AfterChargeback => self.after_chargeback(random),
            Edge::ExactBalance => self.exact_balance(random),
            Edge::BoundaryAmount => self.boundary_amount(random),
            Edge::MalformedRow => self.malformed_row(random),
        }
    }

    /// A deposit or a withdrawal reusing an earlier one's tx id, one draw in two by that row's
    /// client and otherwise by one of `EDGE_CLIENTS`; an ordinary row before any.
    fn repeated_tx(&mut self, random: &mut Gen) {
        let used: Vec<Named> = self.deposits.iter().chain(&self.withdrawals).copied().collect();
        let Some(&Named { client: owner, tx }) = random.pick(&used) else {
            self.ordinary(random);
            return;
        };
        let client = if random.one_in(EVEN_ODDS) {
            owner
        } else {
            random.pick(&EDGE_CLIENTS).copied().unwrap_or_default()
        };
        let amount = ordinary_amount(random);
        let row = if random.one_in(EVEN_ODDS) {
            deposit(client, tx, &amount)
        } else {
            withdrawal(client, tx, &amount)
        };
        self.rows.push(row);
    }

    /// A dispute command naming a tx id no deposit holds.
    fn unknown_tx(&mut self, random: &mut Gen) {
        let targets: Vec<u32> =
            UNKNOWN_TXS.into_iter().chain(self.withdrawals.iter().map(|named| named.tx)).collect();
        let tx = random.pick(&targets).copied().unwrap_or(u32::MAX);
        let client = random.pick(&EDGE_CLIENTS).copied().unwrap_or_default();
        let row = any_dispute(random, client, tx);
        self.rows.push(row);
    }

    /// A dispute command naming another client's deposit; an ordinary row before any deposit.
    fn foreign_tx(&mut self, random: &mut Gen) {
        let Some(&Named { client: owner, tx }) = random.pick(&self.deposits) else {
            self.ordinary(random);
            return;
        };
        let others: Vec<u16> = CLIENTS.into_iter().filter(|&client| client != owner).collect();
        let client = random.pick(&others).copied().unwrap_or_default();
        let row = any_dispute(random, client, tx);
        self.rows.push(row);
    }

    /// On a fresh client, two deposits, the first disputed and charged back, then up to three
    /// more rows.
    fn after_chargeback(&mut self, random: &mut Gen) {
        let client = self.fresh.client();
        let first = self.fresh.tx();
        let second = self.fresh.tx();
        self.push_deposit(client, first, &ordinary_amount(random));
        self.push_deposit(client, second, &ordinary_amount(random));
        self.rows.push(dispute(client, first));
        self.rows.push(chargeback(client, first));
        for _ in 0..random.below(FOUR) {
            match random.below(THREE) {
                0 => {
                    let tx = self.fresh.tx();
                    self.push_deposit(client, tx, &ordinary_amount(random));
                }
                1 => {
                    let tx = self.fresh.tx();
                    self.push_withdrawal(client, tx, &ordinary_amount(random));
                }
                _ => {
                    let tx = if random.one_in(EVEN_ODDS) { first } else { second };
                    let row = any_dispute(random, client, tx);
                    self.rows.push(row);
                }
            }
        }
    }

    /// On a fresh client, a deposit, maybe a second one disputed, then a withdrawal of exactly the
    /// first amount, maybe followed by `0.0001` more.
    fn exact_balance(&mut self, random: &mut Gen) {
        let client = self.fresh.client();
        let amount = ordinary_amount(random);
        let deposit_tx = self.fresh.tx();
        self.push_deposit(client, deposit_tx, &amount);
        if random.one_in(EVEN_ODDS) {
            let disputed_tx = self.fresh.tx();
            self.push_deposit(client, disputed_tx, &ordinary_amount(random));
            self.rows.push(dispute(client, disputed_tx));
        }
        let withdrawal_tx = self.fresh.tx();
        self.push_withdrawal(client, withdrawal_tx, &amount);
        if random.one_in(EVEN_ODDS) {
            let extra_tx = self.fresh.tx();
            self.push_withdrawal(client, extra_tx, "0.0001");
        }
    }

    /// A deposit or a withdrawal of a boundary amount, on a fresh tx id.
    fn boundary_amount(&mut self, random: &mut Gen) {
        let client = random.pick(&CLIENTS).copied().unwrap_or_default();
        let amount = random.pick(&BOUNDARY_AMOUNTS).copied().unwrap_or_default();
        let tx = self.fresh.tx();
        if random.one_in(EVEN_ODDS) {
            self.push_deposit(client, tx, amount);
        } else {
            self.push_withdrawal(client, tx, amount);
        }
    }

    /// A malformed row: unreadable, with too many fields, or a deposit or a withdrawal on a fresh
    /// tx id with no amount or with an amount edge a source delivers.
    fn malformed_row(&mut self, random: &mut Gen) {
        let client = random.pick(&CLIENTS).copied().unwrap_or_default();
        let kind = if random.one_in(EVEN_ODDS) { Kind::Deposit } else { Kind::Withdrawal };
        let delivered: Vec<&str> =
            AMOUNT_EDGES.into_iter().filter(|spelling| !UNDELIVERED.contains(spelling)).collect();
        let row = match random.below(FOUR) {
            0 => InputRow::Unreadable,
            1 => InputRow::TooManyFields,
            2 => InputRow::Fields { kind, client, tx: self.fresh.tx(), amount: None },
            _ => {
                let spelling = random.pick(&delivered).copied().unwrap_or_default();
                InputRow::Fields {
                    kind,
                    client,
                    tx: self.fresh.tx(),
                    amount: Some(spelling.to_owned()),
                }
            }
        };
        self.rows.push(row);
    }
}

/// Rows whose deposits and withdrawals name distinct tx ids, none malformed: the reordering law's
/// hypothesis. Only `unique` and `interleave` build one.
#[derive(Debug, Clone)]
pub(crate) struct Unique(Vec<InputRow>);

impl Unique {
    /// The rows, in order.
    pub(crate) fn rows(&self) -> &[InputRow] {
        &self.0
    }
}

/// `rows` without its malformed rows, and without each deposit or withdrawal whose tx id an
/// earlier one used.
pub(crate) fn unique(rows: &[InputRow]) -> Unique {
    let mut used = BTreeSet::new();
    Unique(
        rows.iter()
            .filter(|input| match row(input) {
                Row::Malformed(_) => false,
                Row::Parsed(Command::Client(command)) => used.insert(command.address().tx()),
                Row::Parsed(Command::Dispute(_)) => true,
            })
            .cloned()
            .collect(),
    )
}

/// The rows of `unique`, each client's in their order, the clients taking turns at random.
pub(crate) fn interleave(random: &mut Gen, unique: &Unique) -> Unique {
    let mut queues: BTreeMap<ClientId, VecDeque<InputRow>> = BTreeMap::new();
    for input in unique.rows() {
        if let Some(client) = client_of(input) {
            queues.entry(client).or_default().push_back(input.clone());
        }
    }
    let mut merged = Vec::new();
    while let Some(client) = random.pick(&waiting(&queues)).copied() {
        if let Some(input) = queues.get_mut(&client).and_then(VecDeque::pop_front) {
            merged.push(input);
        }
    }
    Unique(merged)
}

/// The clients with rows still to take.
fn waiting(queues: &BTreeMap<ClientId, VecDeque<InputRow>>) -> Vec<ClientId> {
    queues.iter().filter(|(_, queue)| !queue.is_empty()).map(|(&client, _)| client).collect()
}

/// The client a row's command names, `None` for a malformed row.
fn client_of(input: &InputRow) -> Option<ClientId> {
    match row(input) {
        Row::Parsed(Command::Client(command)) => Some(command.address().client()),
        Row::Parsed(Command::Dispute(command)) => Some(command.address().client()),
        Row::Malformed(_) => None,
    }
}

/// Up to sixteen input rows: ordinary draws and, one draw in four, an edge's rows.
pub(crate) fn rows(random: &mut Gen) -> Vec<InputRow> {
    let count = usize::try_from(random.below(ROW_COUNTS)).unwrap_or_default();
    let mut draft = Draft::new();
    while draft.rows.len() < count {
        if random.one_in(EDGE_ODDS) {
            let edge = random.pick(&EDGES).copied().unwrap_or(Edge::RepeatedTx);
            draft.edge(random, edge);
        } else {
            draft.ordinary(random);
        }
    }
    draft.rows.truncate(MAX_ROWS);
    draft.rows
}

/// The rows of a run that read as commands: what the core receives.
pub(crate) fn commands(random: &mut Gen) -> Vec<Command> {
    rows(random).iter().filter_map(command).collect()
}

/// Lengths of a sequence of events: `0..=12`.
const EVENT_COUNTS: NonZeroU64 = NonZeroU64::MIN.saturating_add(12);

/// Up to twelve events of every kind, each amount parsed from a valid amount's text.
pub(crate) fn events(random: &mut Gen) -> Vec<Event> {
    let count = random.below(EVENT_COUNTS);
    (0..count).filter_map(|_| event(random)).collect()
}

/// One event of any kind, its amount parsed from a valid amount's text.
fn event(random: &mut Gen) -> Option<Event> {
    let amount = move_amount(random).parse::<Amount>().ok()?;
    let events = [
        Event::Opened,
        Event::Client(ClientEvent::Deposited(amount)),
        Event::Client(ClientEvent::Withdrawn(amount)),
        Event::Dispute(DisputeEvent::Held(amount)),
        Event::Dispute(DisputeEvent::Released(amount)),
        Event::Dispute(DisputeEvent::ChargedBack(amount)),
    ];
    random.pick(&events).copied()
}
