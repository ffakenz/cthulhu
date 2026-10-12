//! Strategies: raw input as a source would deliver it, one draw in four at an edge.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use backend::domain::{
    Address, Amount, ClientCommand, ClientEvent, ClientId, Command, DisputeCommand, DisputeEvent,
    Event, TxId,
};
use backend::run::{Row, RowError};
use proptest::arbitrary::any;
use proptest::collection::{SizeRange, vec};
use proptest::option;
use proptest::prop_oneof;
use proptest::sample::{Index, select};
use proptest::strategy::{Just, Strategy};

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
pub(crate) fn amount_spelling() -> impl Strategy<Value = String> {
    let number = (spelled(&DIGITS, 0..=17), option::of(spelled(&DIGITS, 0..=6))).prop_map(
        |(whole, fraction)| match fraction {
            Some(fraction) => format!("{whole}.{fraction}"),
            None => whole,
        },
    );
    prop_oneof![
        2 => select(&AMOUNT_EDGES).prop_map(str::to_owned),
        3 => number,
        3 => spelled(&AMOUNT_CHARACTERS, 0..=8),
    ]
}

/// Text of characters drawn from `alphabet`, as many as `lengths` allows, each length as likely.
fn spelled(
    alphabet: &'static [char],
    lengths: impl Into<SizeRange>,
) -> impl Strategy<Value = String> {
    vec(select(alphabet), lengths).prop_map(|characters| characters.into_iter().collect())
}

/// An amount's value as a whole part and a fraction.
#[derive(Debug, Clone, Copy)]
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
pub(crate) fn amount_value() -> impl Strategy<Value = AmountValue> {
    let drawn = (any::<u64>(), 0..u64::BITS, 0..10_000_u64)
        .prop_map(|(bits, shift, fraction)| AmountValue { whole: bits >> shift, fraction });
    prop_oneof![1 => select(&AMOUNT_VALUE_EDGES), 3 => drawn]
}

/// Characters for totality: ASCII digits, punctuation and letters, and multi-byte characters (an
/// Arabic-Indic three, a full-width one, an accented letter and a byte-order mark).
const ANY_CHARACTERS: [char; 18] = [
    '0', '1', '5', '9', '.', ',', '-', '+', '_', ' ', 'e', 'E', 'x', 'a', '\u{663}', '\u{ff11}',
    '\u{e9}', '\u{feff}',
];

/// Any text up to sixteen characters: one draw in four an edge (empty, sixty-four zeros then a
/// one, a multi-byte character after a point).
pub(crate) fn any_text() -> impl Strategy<Value = String> {
    let edges = vec![String::new(), format!("{}1", "0".repeat(64)), "1.\u{ff11}".to_owned()];
    prop_oneof![1 => select(edges), 3 => spelled(&ANY_CHARACTERS, 0..=16)]
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

/// The most moves a sequence holds.
const MOVES: usize = 12;

/// Amounts a move takes at its edges: the smallest and the largest.
const MOVE_EDGES: [&str; 2] = ["0.0001", "999999999999999.9999"];

/// The units of 0.0001 in 10^15, the smallest value too large to be an amount.
const TOO_LARGE_UNITS: u64 = 10_000_000_000_000_000_000;

/// Where a move comes from: its own direction and amount, or the moves before it, which fall back
/// to its own when they offer none.
#[derive(Debug, Clone, Copy)]
enum Origin {
    /// Its own direction and amount.
    Own,
    /// The previous move, the other way.
    Back,
    /// The whole balance back to zero, spelled without trailing zeros.
    ToZero,
}

/// Up to twelve moves of a balance, each a direction and the text of a valid amount: one draw in
/// four an edge amount, otherwise `{whole}.{fraction:04}` below 1000 and above zero. One draw in
/// four instead repeats the previous amount the other way, so a balance lands on zero from either
/// side; and, while the balance's magnitude is an amount, one draw in four of the rest moves the
/// whole balance back to zero, spelled without trailing zeros, so a balance whose last decimal is
/// zero also lands on zero through an amount spelled with fewer decimals.
pub(crate) fn balance_moves() -> impl Strategy<Value = Vec<(Direction, String)>> {
    let any_origin = prop_oneof![
        9 => Just(Origin::Own),
        4 => Just(Origin::Back),
        3 => Just(Origin::ToZero),
    ];
    vec((any_origin, direction(), move_amount()), 0..=MOVES).prop_map(|draws| {
        let mut moves: Vec<(Direction, String)> = Vec::new();
        for (origin, direction, amount) in draws {
            let taken = match origin {
                Origin::Own => None,
                Origin::Back => {
                    moves.last().map(|(previous, text)| (previous.opposite(), text.clone()))
                }
                Origin::ToZero => moves.iter().try_fold(0, moved).and_then(zeroing),
            };
            moves.push(taken.unwrap_or((direction, amount)));
        }
        moves
    })
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
fn direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Credit), Just(Direction::Debit)]
}

/// The text of a valid amount: one draw in four an edge, otherwise above zero and below 1000.
fn move_amount() -> impl Strategy<Value = String> {
    let units =
        (1..=9_999_999_u64).prop_map(|units| format!("{}.{:04}", units / 10_000, units % 10_000));
    prop_oneof![1 => select(&MOVE_EDGES).prop_map(str::to_owned), 3 => units]
}

/// A row of the input as a source reads it, before parsing: a missing amount is `None`, never
/// `Some("")`, and no field is padded.
#[derive(Debug, Clone)]
pub(crate) enum InputRow {
    /// A row whose fields read: its kind, its client and tx id, and its amount's text, if any.
    Fields { kind: Kind, client: u16, tx: u32, amount: Option<String> },
    /// A row that does not read, and how.
    Unreadable(Flaw),
    /// A row with more fields than the header names: `first`, then `rest`, after a full row.
    TooManyFields { first: &'static str, rest: &'static [&'static str] },
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
        InputRow::Unreadable(_) => return Row::Malformed(RowError::Unreadable),
        InputRow::TooManyFields { .. } => return Row::Malformed(RowError::TooManyFields),
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

/// A builder of a dispute command's row.
type DisputeRow = fn(u16, u32) -> InputRow;

/// One draw of a run: an ordinary row or, one draw in four, the rows of an edge.
#[derive(Debug)]
enum Draw {
    /// An ordinary row.
    Ordinary(Ordinary),
    /// The rows of an edge.
    Edge(Edge),
}

/// An ordinary row as drawn, by a client from `CLIENTS`: a deposit or a withdrawal of its amount,
/// or a dispute command naming a recorded deposit.
#[derive(Debug)]
enum Ordinary {
    /// A deposit of this amount.
    Deposit { client: u16, amount: String },
    /// A withdrawal of this amount.
    Withdrawal { client: u16, amount: String },
    /// A dispute command, built by `command`, naming the recorded deposit `target` picks.
    Dispute { client: u16, command: DisputeRow, target: Index },
}

/// The edges a run visits, each as likely, as drawn: every choice their rows may need, made before
/// the rows ahead of them are known.
#[derive(Debug)]
enum Edge {
    /// A deposit or a withdrawal reusing an earlier deposit's or withdrawal's tx id, one draw in
    /// two by that row's client (`None`) and otherwise by `client`, one of `EDGE_CLIENTS`;
    /// `instead` before any.
    RepeatedTx {
        used: Index,
        client: Option<u16>,
        amount: String,
        is_deposit: bool,
        instead: Ordinary,
    },
    /// A dispute command naming `u32::MAX`, `4_000_000_000` or a withdrawal's tx id.
    UnknownTx { target: Index, client: u16, command: DisputeRow },
    /// A dispute command naming another client's deposit; `instead` before any deposit.
    ForeignTx { target: Index, other: Index, command: DisputeRow, instead: Ordinary },
    /// On a fresh client: two deposits, a dispute and a chargeback of the first, then up to three
    /// of a deposit, a withdrawal and any dispute command on either deposit.
    AfterChargeback { first: String, second: String, then: Vec<Later> },
    /// On a fresh client: a deposit, maybe a second deposit and its dispute, then a withdrawal of
    /// exactly the first amount, maybe followed by `0.0001` more.
    ExactBalance { amount: String, disputed: Option<String>, over: bool },
    /// A deposit or a withdrawal of an amount at or past an edge of the grammar.
    BoundaryAmount { client: u16, amount: &'static str, is_deposit: bool },
    /// An unreadable row, a row with too many fields, or a deposit or a withdrawal with no amount
    /// or with an amount edge a source delivers.
    MalformedRow(Shape),
}

/// A row after a chargeback, on its client.
#[derive(Debug)]
enum Later {
    /// A deposit of this amount.
    Deposit(String),
    /// A withdrawal of this amount.
    Withdrawal(String),
    /// A dispute command on the first deposit, or else on the second.
    Dispute { on_first: bool, command: DisputeRow },
}

/// What a malformed-row edge builds: an unreadable row and how it fails to read, a row with too
/// many fields and its extra ones, or a deposit or a withdrawal with no amount or with an amount
/// edge a source delivers, which may parse.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// A row that does not read, and how.
    Unreadable(Flaw),
    /// A full row followed by `first`, then `rest`.
    TooManyFields { first: &'static str, rest: &'static [&'static str] },
    /// A deposit or a withdrawal by `client`, with no amount or with `spelling`.
    Fields { client: u16, is_deposit: bool, spelling: Option<&'static str> },
}

/// How a row fails to read.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Flaw {
    /// Its type is none of the five commands, as spelled.
    Type(&'static str),
    /// Its client is not a `u16`, as spelled.
    Client(&'static str),
    /// Its tx is not a `u32`, as spelled.
    Tx(&'static str),
    /// It stops before a column every header names.
    Short,
    /// It holds a byte that is not UTF-8.
    NotUtf8,
    /// It holds nothing but spaces.
    Blank,
}

/// Types no reader accepts: a capital, all capitals, an unknown word, a plural and nothing.
const BAD_TYPES: [&str; 5] = ["Deposit", "DEPOSIT", "transfer", "deposits", ""];

/// Clients no reader accepts: past `u16`, negative, a fraction, a word, nothing, and a hex prefix
/// with a capital X.
const BAD_CLIENTS: [&str; 6] = ["65536", "-1", "1.0", "x", "", "0X1"];

/// Tx ids no reader accepts: past `u32`, negative, an exponent, a word, nothing, and hex past
/// `u32`.
const BAD_TXS: [&str; 6] = ["4294967296", "-1", "1e3", "x", "", "0x100000000"];

/// The fields after a full row with too many, the first and the rest: an empty one, a word, two
/// empty ones, an amount and a word.
const EXTRA_FIELDS: [(&str, &[&str]); 4] = [("", &[]), ("x", &[]), ("", &[""]), ("1.5", &["note"])];

/// Amount edges no source delivers: it reads an empty field as absent and trims every field.
const UNDELIVERED: [&str; 3] = ["", " 1", "1 "];

/// The builders of the three dispute commands.
const DISPUTE_ROWS: [DisputeRow; 3] = [dispute, resolve, chargeback];

/// The clients ordinary rows name, and the edges that need no other.
const CLIENTS: [u16; 3] = [1, 2, 3];

/// The clients repeated and unknown tx ids come from: the ordinary ones and the extremes.
const EDGE_CLIENTS: [u16; 5] = [1, 2, 3, 0, u16::MAX];

/// Tx ids no row records.
const UNKNOWN_TXS: [u32; 2] = [u32::MAX, 4_000_000_000];

/// Amounts at or past an edge of the grammar: the smallest and the largest, 10^15, zero, five
/// decimals, trailing zeros and leading zeros.
const BOUNDARY_AMOUNTS: [&str; 7] =
    ["0.0001", "999999999999999.9999", "1000000000000000", "0", "0.00001", "1.50000", "0001.5"];

/// The most rows a run holds.
pub(crate) const MAX_ROWS: usize = 16;

/// An ordinary amount's text, `whole[.digits]`: a whole below 100 and, one draw in two, a point
/// and one to four digits.
fn ordinary_amount() -> impl Strategy<Value = String> {
    (0..100_u64, option::of(spelled(&DIGITS, 1..=4))).prop_map(|(whole, digits)| match digits {
        Some(digits) => format!("{whole}.{digits}"),
        None => whole.to_string(),
    })
}

/// An ordinary row's draw: a client from `CLIENTS` and any of the five kinds, as often each.
fn ordinary() -> impl Strategy<Value = Ordinary> {
    prop_oneof![
        1 => (select(&CLIENTS), ordinary_amount())
            .prop_map(|(client, amount)| Ordinary::Deposit { client, amount }),
        1 => (select(&CLIENTS), ordinary_amount())
            .prop_map(|(client, amount)| Ordinary::Withdrawal { client, amount }),
        3 => (select(&CLIENTS), select(&DISPUTE_ROWS), any::<Index>())
            .prop_map(|(client, command, target)| Ordinary::Dispute { client, command, target }),
    ]
}

/// An edge's draw, each edge as likely.
fn edge() -> impl Strategy<Value = Edge> {
    let repeated_tx = (
        any::<Index>(),
        option::of(select(&EDGE_CLIENTS)),
        ordinary_amount(),
        any::<bool>(),
        ordinary(),
    )
        .prop_map(|(used, client, amount, is_deposit, instead)| Edge::RepeatedTx {
            used,
            client,
            amount,
            is_deposit,
            instead,
        });
    let unknown_tx = (any::<Index>(), select(&EDGE_CLIENTS), select(&DISPUTE_ROWS))
        .prop_map(|(target, client, command)| Edge::UnknownTx { target, client, command });
    let foreign_tx = (any::<Index>(), any::<Index>(), select(&DISPUTE_ROWS), ordinary()).prop_map(
        |(target, other, command, instead)| Edge::ForeignTx { target, other, command, instead },
    );
    let after_chargeback = (ordinary_amount(), ordinary_amount(), vec(later(), 0..=3))
        .prop_map(|(first, second, then)| Edge::AfterChargeback { first, second, then });
    let exact_balance = (ordinary_amount(), option::of(ordinary_amount()), any::<bool>())
        .prop_map(|(amount, disputed, over)| Edge::ExactBalance { amount, disputed, over });
    let boundary_amount = (select(&CLIENTS), select(&BOUNDARY_AMOUNTS), any::<bool>()).prop_map(
        |(client, amount, is_deposit)| Edge::BoundaryAmount { client, amount, is_deposit },
    );
    let malformed_row = shape().prop_map(Edge::MalformedRow);
    prop_oneof![
        repeated_tx,
        unknown_tx,
        foreign_tx,
        after_chargeback,
        exact_balance,
        boundary_amount,
        malformed_row,
    ]
}

/// A row after a chargeback: a deposit, a withdrawal or any dispute command, as often each.
fn later() -> impl Strategy<Value = Later> {
    prop_oneof![
        ordinary_amount().prop_map(Later::Deposit),
        ordinary_amount().prop_map(Later::Withdrawal),
        (any::<bool>(), select(&DISPUTE_ROWS))
            .prop_map(|(on_first, command)| Later::Dispute { on_first, command }),
    ]
}

/// What a malformed-row edge builds, each of its four rows as likely: an unreadable row with any
/// flaw, a row with any of `EXTRA_FIELDS` past the header, and a deposit or a withdrawal by a
/// client from `CLIENTS` with no amount or with an amount edge a source delivers.
fn shape() -> impl Strategy<Value = Shape> {
    let delivered: Vec<&'static str> =
        AMOUNT_EDGES.into_iter().filter(|spelling| !UNDELIVERED.contains(spelling)).collect();
    let fields = (select(&CLIENTS), any::<bool>(), option::of(select(delivered)))
        .prop_map(|(client, is_deposit, spelling)| Shape::Fields { client, is_deposit, spelling });
    prop_oneof![
        1 => flaw().prop_map(Shape::Unreadable),
        1 => select(&EXTRA_FIELDS).prop_map(|(first, rest)| Shape::TooManyFields { first, rest }),
        2 => fields,
    ]
}

/// A flaw, each kind as likely.
pub(crate) fn flaw() -> impl Strategy<Value = Flaw> {
    prop_oneof![
        select(&BAD_TYPES).prop_map(Flaw::Type),
        select(&BAD_CLIENTS).prop_map(Flaw::Client),
        select(&BAD_TXS).prop_map(Flaw::Tx),
        Just(Flaw::Short),
        Just(Flaw::NotUtf8),
        Just(Flaw::Blank),
    ]
}

/// The item of `items` that `index` picks, `None` when there are none.
fn pick<T: Copy>(index: Index, items: &[T]) -> Option<T> {
    if items.is_empty() {
        return None;
    }
    items.get(index.index(items.len())).copied()
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

/// A run being built from its draws: its rows, the fresh ids left, and the deposits and
/// withdrawals it has built on fresh tx ids.
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

    /// The tx id of the recorded deposit `target` picks, any client's, or a fresh one before any
    /// deposit.
    fn target(&mut self, target: Index) -> u32 {
        match pick(target, &self.deposits) {
            Some(named) => named.tx,
            None => self.fresh.tx(),
        }
    }

    /// The rows of `draw`.
    fn draw(&mut self, draw: &Draw) {
        match draw {
            Draw::Ordinary(ordinary) => self.ordinary(ordinary),
            Draw::Edge(edge) => self.edge(edge),
        }
    }

    /// An ordinary row: a fresh tx id for a deposit or a withdrawal, and a recorded deposit's tx
    /// id for a dispute command.
    fn ordinary(&mut self, ordinary: &Ordinary) {
        match ordinary {
            Ordinary::Deposit { client, amount } => {
                let tx = self.fresh.tx();
                self.push_deposit(*client, tx, amount);
            }
            Ordinary::Withdrawal { client, amount } => {
                let tx = self.fresh.tx();
                self.push_withdrawal(*client, tx, amount);
            }
            Ordinary::Dispute { client, command, target } => {
                let tx = self.target(*target);
                self.rows.push(command(*client, tx));
            }
        }
    }

    /// The rows of `edge`.
    fn edge(&mut self, edge: &Edge) {
        match edge {
            Edge::RepeatedTx { used, client, amount, is_deposit, instead } => {
                self.repeated_tx(*used, *client, amount, *is_deposit, instead);
            }
            Edge::UnknownTx { target, client, command } => {
                self.unknown_tx(*target, *client, *command);
            }
            Edge::ForeignTx { target, other, command, instead } => {
                self.foreign_tx(*target, *other, *command, instead);
            }
            Edge::AfterChargeback { first, second, then } => {
                self.after_chargeback(first, second, then);
            }
            Edge::ExactBalance { amount, disputed, over } => {
                self.exact_balance(amount, disputed.as_deref(), *over);
            }
            Edge::BoundaryAmount { client, amount, is_deposit } => {
                self.boundary_amount(*client, amount, *is_deposit);
            }
            Edge::MalformedRow(shape) => self.malformed_row(*shape),
        }
    }

    /// A deposit or a withdrawal reusing the tx id `used` picks among the earlier deposits' and
    /// withdrawals', by `client`, or by that row's client when `client` is `None`; the row
    /// `instead` before any.
    fn repeated_tx(
        &mut self,
        used: Index,
        client: Option<u16>,
        amount: &str,
        is_deposit: bool,
        instead: &Ordinary,
    ) {
        let earlier: Vec<Named> = self.deposits.iter().chain(&self.withdrawals).copied().collect();
        let Some(Named { client: owner, tx }) = pick(used, &earlier) else {
            self.ordinary(instead);
            return;
        };
        let client = client.unwrap_or(owner);
        let row =
            if is_deposit { deposit(client, tx, amount) } else { withdrawal(client, tx, amount) };
        self.rows.push(row);
    }

    /// A dispute command by `client` naming the tx id `target` picks among those no deposit holds.
    fn unknown_tx(&mut self, target: Index, client: u16, command: DisputeRow) {
        let targets: Vec<u32> =
            UNKNOWN_TXS.into_iter().chain(self.withdrawals.iter().map(|named| named.tx)).collect();
        let tx = pick(target, &targets).unwrap_or(u32::MAX);
        self.rows.push(command(client, tx));
    }

    /// A dispute command naming the deposit `target` picks, by the client `other` picks among the
    /// rest of `CLIENTS`; the row `instead` before any deposit.
    fn foreign_tx(&mut self, target: Index, other: Index, command: DisputeRow, instead: &Ordinary) {
        let Some(Named { client: owner, tx }) = pick(target, &self.deposits) else {
            self.ordinary(instead);
            return;
        };
        let others: Vec<u16> = CLIENTS.into_iter().filter(|&client| client != owner).collect();
        let client = pick(other, &others).unwrap_or_default();
        self.rows.push(command(client, tx));
    }

    /// On a fresh client, deposits of `first` and `second`, the first disputed and charged back,
    /// then the rows `then` names.
    fn after_chargeback(&mut self, first: &str, second: &str, then: &[Later]) {
        let client = self.fresh.client();
        let first_tx = self.fresh.tx();
        let second_tx = self.fresh.tx();
        self.push_deposit(client, first_tx, first);
        self.push_deposit(client, second_tx, second);
        self.rows.push(dispute(client, first_tx));
        self.rows.push(chargeback(client, first_tx));
        for later in then {
            match later {
                Later::Deposit(amount) => {
                    let tx = self.fresh.tx();
                    self.push_deposit(client, tx, amount);
                }
                Later::Withdrawal(amount) => {
                    let tx = self.fresh.tx();
                    self.push_withdrawal(client, tx, amount);
                }
                Later::Dispute { on_first, command } => {
                    let tx = if *on_first { first_tx } else { second_tx };
                    self.rows.push(command(client, tx));
                }
            }
        }
    }

    /// On a fresh client, a deposit of `amount`, then a second deposit of `disputed` and its
    /// dispute when there is one, then a withdrawal of exactly `amount`, followed by `0.0001` more
    /// when `over`.
    fn exact_balance(&mut self, amount: &str, disputed: Option<&str>, over: bool) {
        let client = self.fresh.client();
        let deposit_tx = self.fresh.tx();
        self.push_deposit(client, deposit_tx, amount);
        if let Some(disputed) = disputed {
            let disputed_tx = self.fresh.tx();
            self.push_deposit(client, disputed_tx, disputed);
            self.rows.push(dispute(client, disputed_tx));
        }
        let withdrawal_tx = self.fresh.tx();
        self.push_withdrawal(client, withdrawal_tx, amount);
        if over {
            let extra_tx = self.fresh.tx();
            self.push_withdrawal(client, extra_tx, "0.0001");
        }
    }

    /// A deposit or a withdrawal of `amount` by `client`, on a fresh tx id.
    fn boundary_amount(&mut self, client: u16, amount: &str, is_deposit: bool) {
        let tx = self.fresh.tx();
        if is_deposit {
            self.push_deposit(client, tx, amount);
        } else {
            self.push_withdrawal(client, tx, amount);
        }
    }

    /// The row `shape` names; a deposit or a withdrawal takes a fresh tx id.
    fn malformed_row(&mut self, shape: Shape) {
        let row = match shape {
            Shape::Unreadable(flaw) => InputRow::Unreadable(flaw),
            Shape::TooManyFields { first, rest } => InputRow::TooManyFields { first, rest },
            Shape::Fields { client, is_deposit, spelling } => {
                let kind = if is_deposit { Kind::Deposit } else { Kind::Withdrawal };
                InputRow::Fields {
                    kind,
                    client,
                    tx: self.fresh.tx(),
                    amount: spelling.map(str::to_owned),
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

/// The rows of `unique`, each client's in their order, the clients taking turns: each of `turns`
/// picks the next among the clients still waiting, and once the turns run out the first waiting
/// goes.
pub(crate) fn interleave(unique: &Unique, turns: &[Index]) -> Unique {
    let mut queues: BTreeMap<ClientId, VecDeque<InputRow>> = BTreeMap::new();
    for input in unique.rows() {
        if let Some(client) = client_of(input) {
            queues.entry(client).or_default().push_back(input.clone());
        }
    }
    let mut merged = Vec::new();
    let mut turns = turns.iter().copied();
    loop {
        let waiting = waiting(&queues);
        let next = match turns.next() {
            Some(turn) => pick(turn, &waiting),
            None => waiting.first().copied(),
        };
        let Some(input) =
            next.and_then(|client| queues.get_mut(&client)).and_then(VecDeque::pop_front)
        else {
            break;
        };
        merged.push(input);
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

/// Up to sixteen input rows: ordinary draws and, one draw in four, an edge's rows, until the run
/// holds as many rows as it has draws.
pub(crate) fn rows() -> impl Strategy<Value = Vec<InputRow>> {
    let any_draw = prop_oneof![
        3 => ordinary().prop_map(Draw::Ordinary),
        1 => edge().prop_map(Draw::Edge),
    ];
    vec(any_draw, 0..=MAX_ROWS).prop_map(|draws| {
        let mut draft = Draft::new();
        for draw in &draws {
            if draft.rows.len() >= draws.len() {
                break;
            }
            draft.draw(draw);
        }
        draft.rows.truncate(MAX_ROWS);
        draft.rows
    })
}

/// The rows of a run that read as commands: what the core receives.
pub(crate) fn commands() -> impl Strategy<Value = Vec<Command>> {
    rows().prop_map(|rows| rows.iter().filter_map(command).collect())
}

/// Turns for `interleave`: one for each row a run may hold.
pub(crate) fn turns() -> impl Strategy<Value = Vec<Index>> {
    vec(any::<Index>(), MAX_ROWS)
}

/// The most events a sequence holds.
pub(crate) const MAX_EVENTS: usize = 12;

/// Up to twelve events of every kind, each amount parsed from a valid amount's text.
pub(crate) fn events() -> impl Strategy<Value = Vec<Event>> {
    vec(event(), 0..=MAX_EVENTS).prop_map(|events| events.into_iter().flatten().collect())
}

/// One event of any kind, its amount parsed from a valid amount's text; `None` only if that text
/// does not parse.
pub(crate) fn event() -> impl Strategy<Value = Option<Event>> {
    let kinds: [fn(Amount) -> Event; 6] = [
        |_| Event::Opened,
        |amount| Event::Client(ClientEvent::Deposited(amount)),
        |amount| Event::Client(ClientEvent::Withdrawn(amount)),
        |amount| Event::Dispute(DisputeEvent::Held(amount)),
        |amount| Event::Dispute(DisputeEvent::Released(amount)),
        |amount| Event::Dispute(DisputeEvent::ChargedBack(amount)),
    ];
    (move_amount(), select(kinds.to_vec()))
        .prop_map(|(text, kind)| text.parse::<Amount>().ok().map(kind))
}

/// Clients an address draws from: few, so addresses recur, with both extremes.
const ADDRESS_CLIENTS: [u16; 5] = [1, 2, 3, 0, u16::MAX];

/// Tx ids an address draws from: few, so addresses recur, with both extremes.
const ADDRESS_TXS: [u32; 5] = [1, 2, 3, 0, u32::MAX];

/// One address: a client from `ADDRESS_CLIENTS` and a tx id from `ADDRESS_TXS`, so addresses
/// recur.
pub(crate) fn address() -> impl Strategy<Value = Address> {
    (select(&ADDRESS_CLIENTS), select(&ADDRESS_TXS))
        .prop_map(|(client, tx)| Address::new(ClientId::from(client), TxId::from(tx)))
}
