//! The CSV adapters: `CsvSource` reads rows from CSV text into commands, and `render_csv` writes
//! the accounts table as CSV.

use std::io::{Read, Write};
use std::mem;

use csv::{ErrorKind, ReaderBuilder, StringRecord, Trim, Writer};
use engine::domain::{Address, Amount, ClientCommand, ClientId, Command, DisputeCommand, TxId};
use engine::run::{Accounts, Row, RowError, SourceError};
use serde::Deserialize;

/// The columns every header names.
const REQUIRED: [&str; 3] = ["type", "client", "tx"];

/// The accounts table's header.
const HEADER: [&str; 5] = ["client", "available", "held", "total", "locked"];

/// A row as the CSV source reads it: its type, its client and tx id, and its amount's text,
/// `None` when the header names no amount, or the row leaves it empty or stops before it.
#[derive(Debug, Deserialize)]
struct CsvRow<'a> {
    #[serde(rename = "type")]
    kind: CsvType,
    client: u16,
    tx: u32,
    #[serde(borrow)]
    amount: Option<&'a str>,
}

/// The type column: one of the five commands, in lowercase.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum CsvType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl TryFrom<CsvRow<'_>> for Command {
    type Error = RowError;

    /// The command a row reads as: a deposit or a withdrawal needs an amount that parses; a
    /// dispute command ignores any amount.
    fn try_from(row: CsvRow<'_>) -> Result<Self, Self::Error> {
        let address = Address::new(ClientId::from(row.client), TxId::from(row.tx));
        let amount = || -> Result<Amount, RowError> {
            row.amount.ok_or(RowError::MissingAmount)?.parse().map_err(RowError::InvalidAmount)
        };
        Ok(match row.kind {
            CsvType::Deposit => Self::Client(ClientCommand::Deposit { address, amount: amount()? }),
            CsvType::Withdrawal => {
                Self::Client(ClientCommand::Withdrawal { address, amount: amount()? })
            }
            CsvType::Dispute => Self::Dispute(DisputeCommand::Dispute(address)),
            CsvType::Resolve => Self::Dispute(DisputeCommand::Resolve(address)),
            CsvType::Chargeback => Self::Dispute(DisputeCommand::Chargeback(address)),
        })
    }
}

/// The CSV source over an input: a header naming `type`, `client`, `tx` and optionally `amount`,
/// in any order, then a row per record. Fields are trimmed, and a row may stop before its last
/// column. It yields each row, in order, as the command it reads as or why it is malformed, and
/// ends after a read failure or a header lacking `type`, `client` or `tx`, that error last.
#[derive(Debug)]
pub struct CsvSource<R> {
    reader: csv::Reader<R>,
    record: StringRecord,
    state: State,
}

/// Where a source stands.
#[derive(Debug)]
enum State {
    /// Nothing is read yet.
    Header,
    /// Reading rows under this header.
    Rows(StringRecord),
    /// The input ran out, or a source error was yielded.
    Ended,
}

impl<R: Read> CsvSource<R> {
    /// The source over `input`; nothing is read until the first row is asked for.
    pub fn new(input: R) -> Self {
        let reader = ReaderBuilder::new().trim(Trim::All).flexible(true).from_reader(input);
        Self { reader, record: StringRecord::new(), state: State::Header }
    }
}

impl<R: Read> Iterator for CsvSource<R> {
    type Item = Result<Row, SourceError>;

    fn next(&mut self) -> Option<Self::Item> {
        let headers = match mem::replace(&mut self.state, State::Ended) {
            State::Ended => return None,
            State::Rows(headers) => headers,
            State::Header => match header(&mut self.reader) {
                Ok(headers) => headers,
                Err(error) => return Some(Err(error)),
            },
        };
        let row = match self.reader.read_record(&mut self.record) {
            Ok(false) => return None,
            Ok(true) => row(&self.record, &headers),
            Err(error) => match spoiled(&error) {
                Spoiled::Record => Row::Malformed(RowError::Unreadable),
                Spoiled::Input | Spoiled::Unknown => return Some(Err(SourceError::Unreadable)),
            },
        };
        self.state = State::Rows(headers);
        Some(Ok(row))
    }
}

/// The header, once it names `type`, `client` and `tx`; a header that is not UTF-8 names none, and
/// a failed read is `Unreadable`.
fn header<R: Read>(reader: &mut csv::Reader<R>) -> Result<StringRecord, SourceError> {
    match reader.headers() {
        Ok(headers) if REQUIRED.iter().all(|name| headers.iter().any(|column| column == *name)) => {
            Ok(headers.clone())
        }
        Ok(_) => Err(SourceError::IncompleteHeader),
        Err(error) => Err(match spoiled(&error) {
            Spoiled::Record => SourceError::IncompleteHeader,
            Spoiled::Input | Spoiled::Unknown => SourceError::Unreadable,
        }),
    }
}

/// What `record` reads as under `headers`: the command, or why the row is malformed.
fn row(record: &StringRecord, headers: &StringRecord) -> Row {
    if record.len() > headers.len() {
        return Row::Malformed(RowError::TooManyFields);
    }
    match record.deserialize::<CsvRow<'_>>(Some(headers)).map(Command::try_from) {
        Ok(Ok(command)) => Row::Parsed(command),
        Ok(Err(error)) => Row::Malformed(error),
        Err(_) => Row::Malformed(RowError::Unreadable),
    }
}

/// What a failed read spoils.
#[derive(Debug, Clone, Copy)]
enum Spoiled {
    /// The record being read.
    Record,
    /// The input.
    Input,
    /// A kind of failure `spoiled` does not name.
    Unknown,
}

/// Classifies a failed read: a record that is not UTF-8, that does not deserialize or whose
/// length is refused spoils that record; a failed read, seek or write spoils the input.
fn spoiled(error: &csv::Error) -> Spoiled {
    match error.kind() {
        ErrorKind::Utf8 { .. }
        | ErrorKind::UnequalLengths { .. }
        | ErrorKind::Deserialize { .. } => Spoiled::Record,
        ErrorKind::Io(_) | ErrorKind::Seek | ErrorKind::Serialize(_) => Spoiled::Input,
        _ => Spoiled::Unknown,
    }
}

/// Why the accounts table could not be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SinkError {
    /// Writing the output failed.
    #[error("the output cannot be written")]
    Unwritable,
    /// The client's total, available plus held, passes the largest balance, either way.
    #[error("the total of client {0} overflows")]
    Overflow(ClientId),
}

/// Writes `accounts` to `out` as CSV and flushes it: the header
/// `client,available,held,total,locked`, then a line per account by client id, each balance with
/// four decimals and `locked` as `true` or `false`. Lines written before a failure stay written.
pub fn render_csv(accounts: &Accounts, out: &mut impl Write) -> Result<(), SinkError> {
    let mut writer = Writer::from_writer(out);
    writer.write_record(HEADER).map_err(|_| SinkError::Unwritable)?;
    for (client, account) in accounts.by_client() {
        let total = account.total().ok_or(SinkError::Overflow(client))?;
        let line = [
            client.to_string(),
            account.available().to_string(),
            account.held().to_string(),
            total.to_string(),
            account.is_locked().to_string(),
        ];
        writer.write_record(line).map_err(|_| SinkError::Unwritable)?;
    }
    writer.flush().map_err(|_| SinkError::Unwritable)
}
