//! What a source yields, and how a run fails: a malformed row is skipped and counted; a source or
//! store failure stops the run.

use std::error;
use std::fmt;

use crate::domain::{AmountError, Command};

/// A row as a source delivers it: the command it reads as, or why it is malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// The row reads as a command.
    Parsed(Command),
    /// The row is malformed, and opens nothing.
    Malformed(RowError),
}

/// Why a row is malformed: its kind, by which it is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowError {
    /// A field does not read: a byte that is not UTF-8, an unknown type, a client or tx id that is
    /// missing or is not a `u16` or a `u32`, a column other than `amount` that the row stops
    /// before, or `type`, `client`, `tx` or `amount` named twice in the header.
    Unreadable,
    /// More fields than the header names.
    TooManyFields,
    /// A deposit or a withdrawal without an amount.
    MissingAmount,
    /// A deposit or a withdrawal whose amount is refused, and why.
    InvalidAmount(AmountError),
}

impl fmt::Display for RowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable => f.write_str("a field does not read"),
            Self::TooManyFields => f.write_str("more fields than the header"),
            Self::MissingAmount => f.write_str("no amount"),
            Self::InvalidAmount(refusal) => write!(f, "invalid amount: {refusal}"),
        }
    }
}

impl error::Error for RowError {}

/// Why a source stops a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    /// Reading the input failed.
    Unreadable,
    /// The header lacks `type`, `client` or `tx`.
    IncompleteHeader,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable => f.write_str("the input cannot be read"),
            Self::IncompleteHeader => f.write_str("the header lacks type, client or tx"),
        }
    }
}

impl error::Error for SourceError {}

/// Why a run stopped with no summary: its source failed, or one of its stores did.
#[derive(Debug, PartialEq, Eq)]
pub enum RunError<L, S> {
    /// The source stopped.
    Source(SourceError),
    /// The ledger store failed.
    Ledger(L),
    /// The tx-id store failed.
    Seen(S),
}

impl<L: fmt::Display, S: fmt::Display> fmt::Display for RunError<L, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "{error}"),
            Self::Ledger(error) => write!(f, "the ledger store failed: {error}"),
            Self::Seen(error) => write!(f, "the tx-id store failed: {error}"),
        }
    }
}

impl<L: error::Error, S: error::Error> error::Error for RunError<L, S> {}
