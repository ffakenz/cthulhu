//! Rows spelled as the CSV text a source reads: a header naming its columns in any order, then a
//! line per row; every field bare or quoted and padded or not, every line but an unended last one
//! ending in `\n`, `\r\n` or `\r`, blank lines before any line and after the last, and a
//! byte-order mark or none.

use proptest::arbitrary::any;
use proptest::array::uniform7;
use proptest::bool::weighted;
use proptest::collection::vec;
use proptest::option;
use proptest::prop_oneof;
use proptest::sample::{Index, select};
use proptest::strategy::{Just, Strategy};

use super::generate::{self, Flaw, InputRow, Kind, MAX_ROWS};

/// A column a header names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    /// `type`.
    Type,
    /// `client`.
    Client,
    /// `tx`.
    Tx,
    /// `amount`.
    Amount,
    /// `note`, whose text every reader ignores.
    Note,
}

impl Column {
    /// The column's name in the header.
    fn name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Client => "client",
            Self::Tx => "tx",
            Self::Amount => "amount",
            Self::Note => "note",
        }
    }
}

/// The columns a complete header names.
const REQUIRED: [Column; 3] = [Column::Type, Column::Client, Column::Tx];

/// The ways a header lacks `type`, `client` or `tx`: each lacks the columns listed.
const LACKING: [&[Column]; 7] = [
    &[Column::Type],
    &[Column::Client],
    &[Column::Tx],
    &[Column::Type, Column::Client],
    &[Column::Type, Column::Tx],
    &[Column::Client, Column::Tx],
    &[Column::Type, Column::Client, Column::Tx],
];

/// Whether a field is quoted. Text holding a comma, a quote or a line break is quoted either way.
#[derive(Debug, Clone, Copy)]
enum Quoting {
    Bare,
    Quoted,
}

/// How a field is written: its quoting, and the whitespace before and after its text, inside any
/// quotes; every reader trims it.
#[derive(Debug, Clone, Copy)]
struct Field {
    quoting: Quoting,
    before: &'static str,
    after: &'static str,
}

/// A bare field with no padding.
const BARE: Field = Field { quoting: Quoting::Bare, before: "", after: "" };

/// Whitespace a field may carry around its text.
const PADDING: [&str; 3] = [" ", "  ", "\t"];

/// How a line ends: each a terminator every reader accepts.
#[derive(Debug, Clone, Copy)]
enum End {
    Lf,
    CrLf,
    Cr,
}

/// Whether a blank line, which every reader skips, is written, and how it ends.
#[derive(Debug, Clone, Copy)]
enum Blank {
    Absent,
    Line(End),
}

/// How an id is spelled; every reader reads `7`, `+7`, `07` and `0x7` as 7.
#[derive(Debug, Clone, Copy)]
enum Spelling {
    Plain,
    Signed,
    Padded,
    Hex,
}

/// How a row's ids are spelled.
#[derive(Debug, Clone, Copy)]
struct Ids {
    client: Spelling,
    tx: Spelling,
}

/// Whether a row whose header ends with `amount` writes its empty amount or leaves it out.
#[derive(Debug, Clone, Copy)]
enum Tail {
    Written,
    Left,
}

/// The choices a line's spelling reads. Every line reads how each of its fields is written, how
/// it ends and whether a blank line comes first. A row also reads how its ids are spelled,
/// whether it leaves out an empty trailing amount, the stray amount a dispute command carries
/// (which every reader ignores) and its `note` column's text.
#[derive(Debug, Clone)]
struct Line {
    fields: [Field; 7],
    end: End,
    blank: Blank,
    ids: Ids,
    tail: Tail,
    stray: Option<&'static str>,
    note: &'static str,
}

/// Amounts a dispute command may carry: every reader ignores them.
const STRAY_AMOUNTS: [&str; 4] = ["1.5", "-1", "abc", "0"];

/// Texts of the `note` column: empty, a word, a comma, quotes and a line break.
const NOTES: [&str; 5] = ["", "memo", "a, b", "say \"hi\"", "two\nlines"];

/// Whether the text starts with a UTF-8 byte-order mark, which every reader skips.
#[derive(Debug, Clone, Copy)]
enum Mark {
    Unmarked,
    Marked,
}

/// Whether the last line ends with its terminator.
#[derive(Debug, Clone, Copy)]
enum Last {
    Ended,
    Unended,
}

/// How rows are spelled: a mark or none, the header's columns and its line, a line for each of
/// the first rows (the rest take the header's), whether the last line ends, and whether a blank
/// line follows it.
#[derive(Debug, Clone)]
struct Layout {
    mark: Mark,
    columns: Vec<Column>,
    header: Line,
    lines: Vec<Line>,
    last: Last,
    trailer: Blank,
}

impl Layout {
    /// The layout whose header lacks `lacking`.
    fn lacking(mut self, lacking: &[Column]) -> Self {
        self.columns.retain(|column| !lacking.contains(column));
        self
    }
}

/// Rows as the source must read them under a layout, and the layout that spells them.
#[derive(Debug, Clone)]
pub(crate) struct Input {
    rows: Vec<InputRow>,
    layout: Layout,
}

impl Input {
    /// `rows` under `layout`; under a header without `amount`, a row carrying an amount holds a
    /// field more than the header names.
    fn new(rows: Vec<InputRow>, layout: Layout) -> Self {
        let names_amount = layout.columns.contains(&Column::Amount);
        let rows = rows
            .into_iter()
            .map(|row| match row {
                InputRow::Fields { amount: Some(_), .. } if !names_amount => {
                    InputRow::TooManyFields { first: "", rest: &[] }
                }
                InputRow::Fields { .. }
                | InputRow::Unreadable(_)
                | InputRow::TooManyFields { .. } => row,
            })
            .collect();
        Self { rows, layout }
    }

    /// The rows, as the source must read them.
    pub(crate) fn rows(&self) -> &[InputRow] {
        &self.rows
    }

    /// The rows spelled as CSV text.
    pub(crate) fn text(&self) -> Vec<u8> {
        let Layout { mark, columns, header, lines, last, trailer } = &self.layout;
        let mut text = Vec::new();
        match mark {
            Mark::Unmarked => {}
            Mark::Marked => text.extend_from_slice("\u{feff}".as_bytes()),
        }
        let names = columns.iter().map(|column| column.name().as_bytes().to_vec()).collect();
        let mut spelled: Vec<(Vec<Vec<u8>>, &Line)> = vec![(names, header)];
        for (position, row) in self.rows.iter().enumerate() {
            let line = lines.get(position).unwrap_or(header);
            spelled.push((row_fields(row, columns, line, position), line));
        }
        let count = spelled.len();
        for (index, (fields, line)) in spelled.iter().enumerate() {
            let ending = if index.saturating_add(1) < count { Last::Ended } else { *last };
            write_line(&mut text, fields, line, ending);
        }
        match trailer {
            Blank::Absent => {}
            Blank::Line(end) => text.extend_from_slice(terminator(*end)),
        }
        text
    }

    /// The first `end` rows under the same layout; `None` when there are fewer than `end` rows.
    pub(crate) fn prefix(&self, end: usize) -> Option<Self> {
        self.rows.get(..end).map(|rows| self.with_rows(rows.to_vec()))
    }

    /// `rows` under this input's layout.
    pub(crate) fn with_rows(&self, rows: Vec<InputRow>) -> Self {
        Self::new(rows, self.layout.clone())
    }
}

/// The fields `row` is written with under `columns`, as text before quoting and padding.
fn row_fields(row: &InputRow, columns: &[Column], line: &Line, position: usize) -> Vec<Vec<u8>> {
    match row {
        InputRow::Fields { kind, client, tx, amount } => {
            let mut fields: Vec<Vec<u8>> = columns
                .iter()
                .map(|column| match column {
                    Column::Type => kind_name(*kind).as_bytes().to_vec(),
                    Column::Client => id(u32::from(*client), line.ids.client),
                    Column::Tx => id(*tx, line.ids.tx),
                    Column::Amount => {
                        amount_text(*kind, amount.as_deref(), line.stray).as_bytes().to_vec()
                    }
                    Column::Note => line.note.as_bytes().to_vec(),
                })
                .collect();
            let leaves_amount = match line.tail {
                Tail::Written => false,
                Tail::Left => {
                    columns.last() == Some(&Column::Amount)
                        && fields.last().is_some_and(Vec::is_empty)
                }
            };
            if leaves_amount {
                let _ = fields.pop();
            }
            fields
        }
        InputRow::Unreadable(flaw) => unreadable(columns, *flaw, position),
        InputRow::TooManyFields { first, rest } => {
            let mut fields = plain(columns, position);
            fields.push(first.as_bytes().to_vec());
            fields.extend(rest.iter().map(|text| text.as_bytes().to_vec()));
            fields
        }
    }
}

/// A deposit of 1 by client 1 on tx `position`, under `columns`.
fn plain(columns: &[Column], position: usize) -> Vec<Vec<u8>> {
    columns
        .iter()
        .map(|column| match column {
            Column::Type => b"deposit".to_vec(),
            Column::Client | Column::Amount => b"1".to_vec(),
            Column::Tx => position.to_string().into_bytes(),
            Column::Note => Vec::new(),
        })
        .collect()
}

/// The fields of a row that does not read: a plain row under `columns`, with `flaw` applied.
fn unreadable(columns: &[Column], flaw: Flaw, position: usize) -> Vec<Vec<u8>> {
    let mut fields = plain(columns, position);
    match flaw {
        Flaw::Type(text) => replace(&mut fields, columns, Column::Type, text.as_bytes()),
        Flaw::Client(text) => replace(&mut fields, columns, Column::Client, text.as_bytes()),
        Flaw::Tx(text) => replace(&mut fields, columns, Column::Tx, text.as_bytes()),
        Flaw::Short => {
            let kept = columns.iter().rposition(|column| REQUIRED.contains(column)).unwrap_or(0);
            fields.truncate(kept);
        }
        Flaw::NotUtf8 => replace(&mut fields, columns, Column::Type, b"deposit\xff"),
        Flaw::Blank => fields = vec![b"   ".to_vec()],
    }
    fields
}

/// Writes `text` in the field of `column`, when `columns` has one.
fn replace(fields: &mut [Vec<u8>], columns: &[Column], column: Column, text: &[u8]) {
    let at = columns.iter().position(|named| *named == column);
    if let Some(field) = at.and_then(|at| fields.get_mut(at)) {
        *field = text.to_vec();
    }
}

/// The type column's text for `kind`.
fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Deposit => "deposit",
        Kind::Withdrawal => "withdrawal",
        Kind::Dispute => "dispute",
        Kind::Resolve => "resolve",
        Kind::Chargeback => "chargeback",
    }
}

/// `value` as `spelling` writes it.
fn id(value: u32, spelling: Spelling) -> Vec<u8> {
    match spelling {
        Spelling::Plain => value.to_string(),
        Spelling::Signed => format!("+{value}"),
        Spelling::Padded => format!("0{value}"),
        Spelling::Hex => format!("0x{value:x}"),
    }
    .into_bytes()
}

/// The amount column's text: the row's amount, else a dispute command's stray amount, else
/// nothing.
fn amount_text<'a>(kind: Kind, amount: Option<&'a str>, stray: Option<&'static str>) -> &'a str {
    match amount {
        Some(text) => text,
        None => match kind {
            Kind::Dispute | Kind::Resolve | Kind::Chargeback => stray.unwrap_or(""),
            Kind::Deposit | Kind::Withdrawal => "",
        },
    }
}

/// Writes `fields` as one line: a blank line first if `line` says so, each field as `line`
/// writes it, then its terminator unless `ending` leaves the line unended.
fn write_line(text: &mut Vec<u8>, fields: &[Vec<u8>], line: &Line, ending: Last) {
    match line.blank {
        Blank::Absent => {}
        Blank::Line(blank) => text.extend_from_slice(terminator(blank)),
    }
    for (index, content) in fields.iter().enumerate() {
        if index > 0 {
            text.push(b',');
        }
        write_field(text, content, line.fields.get(index).copied().unwrap_or(BARE));
    }
    match ending {
        Last::Ended => text.extend_from_slice(terminator(line.end)),
        Last::Unended => {}
    }
}

/// Writes `content` as `field` says, quoted when asked or when it holds a comma, a quote or a
/// line break, its padding inside any quotes and every quote doubled.
fn write_field(text: &mut Vec<u8>, content: &[u8], field: Field) {
    let quoted = match field.quoting {
        Quoting::Quoted => true,
        Quoting::Bare => content.iter().any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n')),
    };
    if quoted {
        text.push(b'"');
    }
    text.extend_from_slice(field.before.as_bytes());
    for &byte in content {
        if byte == b'"' {
            text.push(b'"');
        }
        text.push(byte);
    }
    text.extend_from_slice(field.after.as_bytes());
    if quoted {
        text.push(b'"');
    }
}

/// The bytes `end` writes.
fn terminator(end: End) -> &'static [u8] {
    match end {
        End::Lf => b"\n",
        End::CrLf => b"\r\n",
        End::Cr => b"\r",
    }
}

/// Generated rows under a layout whose header names `type`, `client` and `tx`.
pub(crate) fn input() -> impl Strategy<Value = Input> {
    (layout(), generate::rows()).prop_map(|(layout, rows)| Input::new(rows, layout))
}

/// Generated rows with up to two unreadable rows spliced in at drawn places, under a layout whose
/// header names `type`, `client` and `tx`: each flaw a row can have, often.
pub(crate) fn flawed() -> impl Strategy<Value = Input> {
    (layout(), generate::rows(), vec((any::<Index>(), generate::flaw()), 0..=2)).prop_map(
        |(layout, mut rows, flaws)| {
            for (place, flaw) in flaws {
                rows.insert(place.index(rows.len().saturating_add(1)), InputRow::Unreadable(flaw));
            }
            Input::new(rows, layout)
        },
    )
}

/// Generated rows under such a layout with no byte-order mark: a reader whose first read holds
/// only the mark reads it as an empty input.
pub(crate) fn unmarked() -> impl Strategy<Value = Input> {
    input().prop_map(|input| Input {
        layout: Layout { mark: Mark::Unmarked, ..input.layout },
        ..input
    })
}

/// Generated rows under a header that lacks at least one of `type`, `client` and `tx`.
pub(crate) fn incomplete() -> impl Strategy<Value = Input> {
    (layout(), generate::rows(), select(&LACKING))
        .prop_map(|(layout, rows, lacking)| Input::new(rows, layout.lacking(lacking)))
}

/// A layout: the four columns shuffled; `amount` left out and `note` inserted at a drawn place,
/// one draw in eight each; a byte-order mark one draw in eight; up to a line per row a run may
/// hold; the last line unended one draw in four; a trailing blank line one draw in eight.
fn layout() -> impl Strategy<Value = Layout> {
    let order = Just(vec![Column::Type, Column::Client, Column::Tx, Column::Amount]).prop_shuffle();
    (
        weighted(0.125),
        order,
        weighted(0.125),
        option::weighted(0.125, any::<Index>()),
        line(),
        vec(line(), 0..=MAX_ROWS),
        weighted(0.25),
        prop_oneof![7 => Just(Blank::Absent), 1 => ending().prop_map(Blank::Line)],
    )
        .prop_map(
            |(marked, mut columns, without_amount, note, header, lines, unended, trailer)| {
                if without_amount {
                    columns.retain(|column| *column != Column::Amount);
                }
                if let Some(place) = note {
                    columns.insert(place.index(columns.len().saturating_add(1)), Column::Note);
                }
                Layout {
                    mark: if marked { Mark::Marked } else { Mark::Unmarked },
                    columns,
                    header,
                    lines,
                    last: if unended { Last::Unended } else { Last::Ended },
                    trailer,
                }
            },
        )
}

/// A line's choices: a blank line before it one draw in eight; ids spelled plainly five draws in
/// eight; an empty trailing amount left out one draw in two; a stray amount one draw in four.
fn line() -> impl Strategy<Value = Line> {
    (
        uniform7(field()),
        ending(),
        prop_oneof![7 => Just(Blank::Absent), 1 => ending().prop_map(Blank::Line)],
        (id_spelling(), id_spelling()).prop_map(|(client, tx)| Ids { client, tx }),
        prop_oneof![Just(Tail::Written), Just(Tail::Left)],
        option::weighted(0.25, select(&STRAY_AMOUNTS)),
        select(&NOTES),
    )
        .prop_map(|(fields, end, blank, ids, tail, stray, note)| Line {
            fields,
            end,
            blank,
            ids,
            tail,
            stray,
            note,
        })
}

/// A field: quoted one draw in four, and each side padded one draw in four.
fn field() -> impl Strategy<Value = Field> {
    let padding = || prop_oneof![3 => Just(""), 1 => select(&PADDING)];
    (prop_oneof![3 => Just(Quoting::Bare), 1 => Just(Quoting::Quoted)], padding(), padding())
        .prop_map(|(quoting, before, after)| Field { quoting, before, after })
}

/// A line end: `\n` one draw in two, `\r\n` and `\r` one in four each.
fn ending() -> impl Strategy<Value = End> {
    prop_oneof![2 => Just(End::Lf), 1 => Just(End::CrLf), 1 => Just(End::Cr)]
}

/// An id's spelling: plain five draws in eight, each other one in eight.
fn id_spelling() -> impl Strategy<Value = Spelling> {
    prop_oneof![
        5 => Just(Spelling::Plain),
        1 => Just(Spelling::Signed),
        1 => Just(Spelling::Padded),
        1 => Just(Spelling::Hex),
    ]
}
