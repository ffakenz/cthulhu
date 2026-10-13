//! Generators of the input, one per field, composed into lines: valid values and every spelling the
//! input admits for them, and the flaws that make a line malformed.

use engine::domain::AmountError;
use engine::run::RowError;
use proptest::arbitrary::any;
use proptest::array::uniform5;
use proptest::option;
use proptest::prop_oneof;
use proptest::sample::select;
use proptest::strategy::{Just, Strategy};

/// The smallest whole part an amount cannot have: 10^15.
const TOO_LARGE: u64 = 1_000_000_000_000_000;

/// The commands' types, as a line names them.
const KINDS: [&str; 5] = ["deposit", "withdrawal", "dispute", "resolve", "chargeback"];

/// The types whose amounts are read.
const MOVERS: [&str; 2] = ["deposit", "withdrawal"];

/// Whether a line of type `kind` moves money, and so has its amount read.
pub(crate) fn moves_money(kind: &str) -> bool {
    MOVERS.contains(&kind)
}

/// An amount's value: whole units and ten-thousandths, below 10^15 and never zero.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Money {
    whole: u64,
    fraction: u16,
}

/// The values at the edges: the smallest, one, and the largest.
const EDGE_MONEY: [Money; 3] = [
    Money { whole: 0, fraction: 1 },
    Money { whole: 1, fraction: 0 },
    Money { whole: TOO_LARGE - 1, fraction: 9_999 },
];

impl Money {
    /// The value as a balance prints it: four decimals.
    pub(crate) fn text(self) -> String {
        format!("{}.{:04}", self.whole, self.fraction)
    }

    /// The value after `leading` zeros, with `trailing` zeros after its last significant decimal,
    /// and with no point when it is whole and nothing trails.
    pub(crate) fn spelled(self, leading: usize, trailing: usize) -> String {
        let decimals = format!("{:04}", self.fraction);
        let significant = decimals.trim_end_matches('0');
        let point = if significant.is_empty() && trailing == 0 {
            String::new()
        } else {
            format!(".{significant}{}", "0".repeat(trailing))
        };
        format!("{}{}{point}", "0".repeat(leading), self.whole)
    }
}

/// Any amount's value, one draw in four at an edge.
pub(crate) fn any_money() -> impl Strategy<Value = Money> {
    let ordinary = (prop_oneof![0..10_u64, 0..TOO_LARGE], prop_oneof![Just(0_u16), 0..=9_999_u16])
        .prop_map(|(whole, fraction)| Money {
            whole,
            fraction: if whole == 0 { fraction.max(1) } else { fraction },
        });
    prop_oneof![1 => select(&EDGE_MONEY), 3 => ordinary]
}

/// Any client id, one draw in four at an edge.
pub(crate) fn any_client() -> impl Strategy<Value = u16> {
    prop_oneof![1 => select(&[0, u16::MAX]), 3 => any::<u16>()]
}

/// Any tx id, one draw in four at an edge.
pub(crate) fn any_tx() -> impl Strategy<Value = u32> {
    prop_oneof![1 => select(&[0, u32::MAX]), 3 => any::<u32>()]
}

/// A valid line's values; a dispute command's amount goes unread.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Line {
    pub(crate) kind: &'static str,
    pub(crate) client: u16,
    pub(crate) tx: u32,
    pub(crate) amount: Money,
}

impl Line {
    /// The line in the canonical spelling, with no amount for a dispute command.
    fn text(self) -> String {
        let amount = if moves_money(self.kind) { self.amount.text() } else { String::new() };
        format!("{},{},{},{amount}\n", self.kind, self.client, self.tx)
    }
}

/// A line of any type, its ids drawn by `clients` and `txs`, for any amount.
pub(crate) fn any_line(
    clients: impl Strategy<Value = u16>,
    txs: impl Strategy<Value = u32>,
) -> impl Strategy<Value = Line> {
    (select(&KINDS), clients, txs, any_money()).prop_map(|(kind, client, tx, amount)| Line {
        kind,
        client,
        tx,
        amount,
    })
}

/// `lines` in the canonical spelling, without the header.
pub(crate) fn rows(lines: &[Line]) -> String {
    lines.iter().map(|line| line.text()).collect()
}

/// `lines` in the canonical spelling: the header `type,client,tx,amount`, then a line each.
pub(crate) fn canonical(lines: &[Line]) -> String {
    String::from("type,client,tx,amount\n") + &rows(lines)
}

/// How a field is written: padded on either side, or quoted, which only its first byte can open.
#[derive(Debug, Clone, Copy)]
enum Wrap {
    Padded(&'static str, &'static str),
    Quoted,
}

/// Padding, most often none.
const PADS: [&str; 4] = ["", "", " ", " \t"];

impl Wrap {
    /// `text` written this way.
    fn around(self, text: &str) -> String {
        match self {
            Self::Padded(before, after) => format!("{before}{text}{after}"),
            Self::Quoted => format!("\"{text}\""),
        }
    }
}

/// Any wrap, one draw in four quoted.
fn any_wrap() -> impl Strategy<Value = Wrap> {
    prop_oneof![
        3 => (select(&PADS), select(&PADS)).prop_map(|(before, after)| Wrap::Padded(before, after)),
        1 => Just(Wrap::Quoted),
    ]
}

/// How an id is written: in decimal, after zeros, after a plus sign, or in hex after `0x`.
#[derive(Debug, Clone, Copy)]
enum Id {
    Decimal,
    Zeros,
    Plus,
    Hex,
}

/// Every way to write an id.
const IDS: [Id; 4] = [Id::Decimal, Id::Zeros, Id::Plus, Id::Hex];

impl Id {
    /// `id` written this way.
    fn spell(self, id: u64) -> String {
        match self {
            Self::Decimal => id.to_string(),
            Self::Zeros => format!("00{id}"),
            Self::Plus => format!("+{id}"),
            Self::Hex => format!("0x{id:x}"),
        }
    }
}

/// How a dispute command's amount is written: empty, as a value, as text an amount refuses, or left
/// out where `amount` is the last column.
#[derive(Debug, Clone)]
enum Stray {
    Empty,
    Value,
    Refused(String),
    Absent,
}

/// Any way to write a dispute command's amount.
fn any_stray() -> impl Strategy<Value = Stray> {
    prop_oneof![
        Just(Stray::Empty),
        Just(Stray::Value),
        any_refused_amount().prop_map(|(text, _)| Stray::Refused(text)),
        Just(Stray::Absent),
    ]
}

/// The line ends the input reads.
const ENDS: [&str; 3] = ["\n", "\r\n", "\r"];

/// How one line is written: a wrap for each field in order, its ids', its amount's zeros before
/// and after, a dispute command's amount, its end, and whether a blank line comes before it.
#[derive(Debug, Clone)]
pub(crate) struct Spelling {
    wraps: [Wrap; 5],
    client: Id,
    tx: Id,
    zeros: (usize, usize),
    stray: Stray,
    end: &'static str,
    blank: bool,
}

/// Any spelling of a line.
pub(crate) fn any_spelling() -> impl Strategy<Value = Spelling> {
    let zeros = (0..=2_usize, 0..=2_usize);
    let ends = select(&ENDS);
    (uniform5(any_wrap()), select(&IDS), select(&IDS), zeros, any_stray(), ends, any::<bool>())
        .prop_map(|(wraps, client, tx, zeros, stray, end, blank)| Spelling {
            wraps,
            client,
            tx,
            zeros,
            stray,
            end,
            blank,
        })
}

/// How the input around its lines is written: a byte-order mark or not, the header's columns in
/// order with a wrap for each, the header's end, and whether the last line ends.
#[derive(Debug, Clone)]
pub(crate) struct Layout {
    bom: bool,
    columns: Vec<&'static str>,
    header: [Wrap; 5],
    end: &'static str,
    last_end: bool,
}

impl Layout {
    /// `line` as this layout carries it: under a header that names no `amount`, a deposit or a
    /// withdrawal becomes a dispute of the same client and tx id.
    pub(crate) fn fit(&self, line: Line) -> Line {
        if moves_money(line.kind) && !self.columns.contains(&"amount") {
            Line { kind: "dispute", ..line }
        } else {
            line
        }
    }
}

/// Any layout: `type`, `client` and `tx` in any order, with `amount`, most often, and a column no
/// line needs, each inserted or not.
pub(crate) fn any_layout() -> impl Strategy<Value = Layout> {
    let three = vec!["type", "client", "tx"];
    let inserted = (option::weighted(0.9, 0..=3_usize), option::of(0..=4_usize));
    let columns =
        (Just(three).prop_shuffle(), inserted).prop_map(|(mut columns, (amount, note))| {
            if let Some(at) = amount {
                columns.insert(at.min(columns.len()), "amount");
            }
            if let Some(at) = note {
                columns.insert(at.min(columns.len()), "note");
            }
            columns
        });
    (any::<bool>(), columns, uniform5(any_wrap()), select(&ENDS), any::<bool>()).prop_map(
        |(bom, columns, header, end, last_end)| Layout { bom, columns, header, end, last_end },
    )
}

/// How a header lacks one of `type`, `client` and `tx`: the `column` is left out, or named
/// `instead` in another case or with a letter more.
#[derive(Debug, Clone, Copy)]
struct Lack {
    column: &'static str,
    instead: Option<&'static str>,
}

/// Every way a header lacks one of `type`, `client` and `tx`.
const LACKS: [Lack; 9] = [
    Lack { column: "type", instead: None },
    Lack { column: "type", instead: Some("Type") },
    Lack { column: "type", instead: Some("types") },
    Lack { column: "client", instead: None },
    Lack { column: "client", instead: Some("CLIENT") },
    Lack { column: "client", instead: Some("clients") },
    Lack { column: "tx", instead: None },
    Lack { column: "tx", instead: Some("tX") },
    Lack { column: "tx", instead: Some("txs") },
];

/// Any layout whose header lacks one of `type`, `client` and `tx`.
pub(crate) fn any_incomplete_layout() -> impl Strategy<Value = Layout> {
    (any_layout(), select(&LACKS)).prop_map(|(mut layout, lack)| {
        layout.columns = layout
            .columns
            .into_iter()
            .filter_map(|column| if column == lack.column { lack.instead } else { Some(column) })
            .collect();
        layout
    })
}

/// `line`'s fields in the order of `columns`, as `spelling` writes them.
fn fields(line: Line, spelling: &Spelling, columns: &[&str]) -> String {
    let amount_last = columns.last() == Some(&"amount");
    let (leading, trailing) = spelling.zeros;
    let texts: Vec<String> = columns
        .iter()
        .zip(spelling.wraps)
        .filter_map(|(&column, wrap)| {
            let text = match column {
                "type" => line.kind.to_owned(),
                "client" => spelling.client.spell(u64::from(line.client)),
                "tx" => spelling.tx.spell(u64::from(line.tx)),
                "amount" if moves_money(line.kind) => line.amount.spelled(leading, trailing),
                "amount" => match &spelling.stray {
                    Stray::Absent if amount_last => return None,
                    Stray::Value => line.amount.spelled(leading, trailing),
                    Stray::Refused(text) => text.clone(),
                    Stray::Empty | Stray::Absent => String::new(),
                },
                _ => "x".to_owned(),
            };
            Some(wrap.around(&text))
        })
        .collect();
    texts.join(",")
}

/// `lines` under `layout`, each written as its spelling says.
pub(crate) fn spelled(lines: &[(Line, Spelling)], layout: &Layout) -> String {
    let names: Vec<String> =
        layout.columns.iter().zip(layout.header).map(|(name, wrap)| wrap.around(name)).collect();
    let bom = if layout.bom { "\u{feff}" } else { "" };
    let header = String::from(bom) + &names.join(",") + layout.end;
    let body = lines.iter().fold(String::new(), |text, (line, spelling)| {
        let blank = if spelling.blank { spelling.end } else { "" };
        text + blank + &fields(*line, spelling, &layout.columns) + spelling.end
    });
    let last_end = lines.last().map_or("", |(_, spelling)| spelling.end);
    match body.strip_suffix(last_end) {
        Some(unended) if !layout.last_end => header + unended,
        Some(_) | None => header + &body,
    }
}

/// What makes a line malformed.
#[derive(Debug, Clone)]
pub(crate) enum Flaw {
    /// A type that names no command.
    Kind(&'static str),
    /// A client id that is not a `u16`.
    Client(String),
    /// A tx id that is not a `u32`.
    Tx(String),
    /// A deposit or a withdrawal with no amount.
    NoAmount,
    /// A deposit or a withdrawal whose amount is refused, and why.
    Amount(String, AmountError),
    /// A deposit or a withdrawal whose amount is quoted after a space, so its quotes read as part of
    /// it.
    LateQuote,
    /// A field more than the header names.
    Extra,
    /// A line that stops before its tx id.
    Short,
    /// A byte that is not UTF-8.
    NotUtf8,
}

impl Flaw {
    /// The kind the report counts a line with this flaw as.
    pub(crate) fn kind(&self) -> RowError {
        match self {
            Self::Kind(_) | Self::Client(_) | Self::Tx(_) | Self::Short | Self::NotUtf8 => {
                RowError::Unreadable
            }
            Self::NoAmount => RowError::MissingAmount,
            Self::Amount(_, why) => RowError::InvalidAmount(*why),
            Self::LateQuote => RowError::InvalidAmount(AmountError::Malformed),
            Self::Extra => RowError::TooManyFields,
        }
    }
}

/// Types that name no command.
const NOT_KINDS: [&str; 4] = ["Deposit", "WITHDRAWAL", "transfer", ""];

/// Ids of no integer type.
const NOT_IDS: [&str; 6] = ["", "-1", "1.0", "one", "0X1", "1 1"];

/// Text before and after an amount's shortest spelling that takes it outside the grammar: a sign,
/// an exponent, a point with no digits on one side, a digit that is not ASCII, a letter.
const BREAKS: [(&str, &str); 7] =
    [("+", ""), ("-", ""), ("", "e2"), (".", ""), ("", "."), ("", "\u{663}"), ("", "a")];

/// Zero, as `whole` zeros and, unless `decimals` is zero, a point and that many zeros.
fn zero(whole: usize, decimals: usize) -> String {
    let fraction = if decimals == 0 { String::new() } else { format!(".{}", "0".repeat(decimals)) };
    "0".repeat(whole) + &fraction
}

/// Digits an amount refuses, with why: zero, more than four decimals, or 10^15 or more. More than
/// four decimals is drawn on whole parts of any size, since that refusal comes before 10^15 or
/// more. A whole part of 10^15 or more is at an edge one draw in four: 10^15 itself, or the
/// largest.
fn any_out_of_bounds() -> impl Strategy<Value = (String, AmountError)> {
    let any_whole = prop_oneof![0..TOO_LARGE, TOO_LARGE..=u64::MAX];
    let any_too_large = prop_oneof![1 => select(&[TOO_LARGE, u64::MAX]), 3 => TOO_LARGE..=u64::MAX];
    prop_oneof![
        (1..=3_usize, 0..=5_usize)
            .prop_map(|(whole, decimals)| (zero(whole, decimals), AmountError::Zero)),
        (any_whole, 0..=9_999_u16, 1..=9_u8).prop_map(|(whole, fraction, digit)| (
            format!("{whole}.{fraction:04}{digit}"),
            AmountError::TooPrecise
        )),
        (any_too_large, 0..=9_999_u16).prop_map(|(whole, fraction)| (
            format!("{whole}.{fraction:04}"),
            AmountError::TooLarge
        )),
    ]
}

/// Text an amount refuses, with why: zero, more than four decimals, 10^15 or more, or not digits
/// with an optional fraction, which comes first and so breaks refused digits as well as valid
/// ones. None of it is empty, padded or holds a comma, so a CSV field carries it as it is.
pub(crate) fn any_refused_amount() -> impl Strategy<Value = (String, AmountError)> {
    let any_digits = prop_oneof![
        any_money().prop_map(|money| money.spelled(0, 0)),
        any_out_of_bounds().prop_map(|(text, _)| text)
    ];
    prop_oneof![
        any_out_of_bounds(),
        (any_digits, select(&BREAKS)).prop_map(|(text, (before, after))| {
            (format!("{before}{text}{after}"), AmountError::Malformed)
        }),
    ]
}

/// Any flaw.
fn any_flaw() -> impl Strategy<Value = Flaw> {
    let not_id = select(&NOT_IDS).prop_map(str::to_owned);
    let above_u16 = (u64::from(u16::MAX) + 1..=u64::from(u32::MAX)).prop_map(|id| id.to_string());
    let above_u32 = (u64::from(u32::MAX) + 1..=u64::MAX).prop_map(|id| id.to_string());
    prop_oneof![
        select(&NOT_KINDS).prop_map(Flaw::Kind),
        prop_oneof![not_id.clone(), above_u16].prop_map(Flaw::Client),
        prop_oneof![not_id, above_u32].prop_map(Flaw::Tx),
        Just(Flaw::NoAmount),
        any_refused_amount().prop_map(|(text, why)| Flaw::Amount(text, why)),
        Just(Flaw::LateQuote),
        Just(Flaw::Extra),
        Just(Flaw::Short),
        Just(Flaw::NotUtf8),
    ]
}

/// A malformed line: a line of any type, its ids drawn by `clients` and `txs`, with one flaw. A
/// flaw of the amount falls on a deposit or a withdrawal, whose amounts are read.
pub(crate) fn any_malformed(
    clients: impl Strategy<Value = u16>,
    txs: impl Strategy<Value = u32>,
) -> impl Strategy<Value = (Line, Flaw)> {
    (any_line(clients, txs), any_flaw(), select(&MOVERS)).prop_map(|(line, flaw, mover)| {
        let kind = match &flaw {
            Flaw::NoAmount | Flaw::Amount(..) | Flaw::LateQuote => mover,
            Flaw::Kind(_)
            | Flaw::Client(_)
            | Flaw::Tx(_)
            | Flaw::Extra
            | Flaw::Short
            | Flaw::NotUtf8 => line.kind,
        };
        (Line { kind, ..line }, flaw)
    })
}

/// `line` in the canonical spelling but for `flaw`.
pub(crate) fn malformed(line: Line, flaw: &Flaw) -> Vec<u8> {
    let (kind, client, tx, amount) = (line.kind, line.client, line.tx, line.amount.text());
    let text = match flaw {
        Flaw::Kind(name) => format!("{name},{client},{tx},{amount}\n"),
        Flaw::Client(id) => format!("{kind},{id},{tx},{amount}\n"),
        Flaw::Tx(id) => format!("{kind},{client},{id},{amount}\n"),
        Flaw::NoAmount => format!("{kind},{client},{tx},\n"),
        Flaw::Amount(refused, _) => format!("{kind},{client},{tx},{refused}\n"),
        Flaw::LateQuote => format!("{kind},{client},{tx}, \"{amount}\"\n"),
        Flaw::Extra => format!("{kind},{client},{tx},{amount},x\n"),
        Flaw::Short => format!("{kind},{client}\n"),
        Flaw::NotUtf8 => format!("{kind},{client},{tx},{amount}\n"),
    };
    match flaw {
        Flaw::NotUtf8 => [b"\xff".as_slice(), text.as_bytes()].concat(),
        Flaw::Kind(_)
        | Flaw::Client(_)
        | Flaw::Tx(_)
        | Flaw::NoAmount
        | Flaw::Amount(..)
        | Flaw::LateQuote
        | Flaw::Extra
        | Flaw::Short => text.into_bytes(),
    }
}

/// The columns a line's fields are read from.
const COLUMNS: [&str; 4] = ["type", "client", "tx", "amount"];

/// A line's canonical spelling: no padding, ids in decimal, no zeros added, an empty amount for a
/// dispute command, and a `\n` end with no blank line before it.
const CANONICAL: Spelling = Spelling {
    wraps: [Wrap::Padded("", ""); 5],
    client: Id::Decimal,
    tx: Id::Decimal,
    zeros: (0, 0),
    stray: Stray::Empty,
    end: "\n",
    blank: false,
};

/// How a header misfits the lines under it, so that each is malformed.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Misfit {
    /// The header names this one of `type`, `client`, `tx` and `amount` twice, and each line has a
    /// field for each name.
    Twice(&'static str),
    /// The header ends in a column besides the four, and each line stops before it.
    Unfinished,
    /// The header names no `amount`, and each line is this deposit or withdrawal.
    NoAmount(&'static str),
}

impl Misfit {
    /// The kind the report counts each line under this header as.
    pub(crate) fn kind(self) -> RowError {
        match self {
            Self::Twice(_) | Self::Unfinished => RowError::Unreadable,
            Self::NoAmount(_) => RowError::MissingAmount,
        }
    }

    /// The header, then `lines` under it in the canonical spelling.
    pub(crate) fn text(self, lines: &[Line]) -> String {
        let (named, read, kind) = match self {
            Self::Twice(column) => {
                let twice = [COLUMNS.as_slice(), &[column]].concat();
                (twice.clone(), twice, None)
            }
            Self::Unfinished => ([COLUMNS.as_slice(), &["note"]].concat(), COLUMNS.to_vec(), None),
            Self::NoAmount(mover) => {
                (vec!["type", "client", "tx"], vec!["type", "client", "tx"], Some(mover))
            }
        };
        lines.iter().fold(named.join(",") + "\n", |text, &line| {
            let written = Line { kind: kind.unwrap_or(line.kind), ..line };
            text + &fields(written, &CANONICAL, &read) + "\n"
        })
    }
}

/// Any misfit, its column named twice or its deposit or withdrawal drawn among all.
pub(crate) fn any_misfit() -> impl Strategy<Value = Misfit> {
    prop_oneof![
        select(&COLUMNS).prop_map(Misfit::Twice),
        Just(Misfit::Unfinished),
        select(&MOVERS).prop_map(Misfit::NoAmount),
    ]
}
