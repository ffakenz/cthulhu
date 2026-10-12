//! The text adapter: `render_report` writes the report as lines of text.

use std::io::{self, Write};

use crate::domain::{AmountError, ClientReason, DisputeReason, Reason};
use crate::run::{Report, RowError};

/// Writes `report` to `out`, a line per count: each rejection reason, then each malformed row
/// kind, as `<count> rejected: <reason>` and `<count> malformed: <kind>`.
pub fn render_report(report: &Report, out: &mut impl Write) -> io::Result<()> {
    for (reason, count) in report.rejected() {
        writeln!(out, "{count} rejected: {}", rejection(reason))?;
    }
    for (kind, count) in report.malformed() {
        writeln!(out, "{count} malformed: {}", malformation(kind))?;
    }
    Ok(())
}

/// How the report names a rejection's reason.
fn rejection(reason: Reason) -> &'static str {
    match reason {
        Reason::Client(ClientReason::DuplicateTx) => "the tx id was used before",
        Reason::Client(ClientReason::AccountLocked) => "the account is locked",
        Reason::Client(ClientReason::InsufficientFunds) => "the amount is above available",
        Reason::Dispute(DisputeReason::UnknownDeposit) => "no such deposit for this client",
        Reason::Dispute(DisputeReason::NotSettled) => "the deposit is already disputed",
        Reason::Dispute(DisputeReason::NotDisputed) => "the deposit is not disputed",
        Reason::Overflow => "a balance would overflow",
    }
}

/// How the report names a malformed row's kind.
fn malformation(kind: RowError) -> &'static str {
    match kind {
        RowError::Unreadable => "a field does not read",
        RowError::TooManyFields => "more fields than the header",
        RowError::MissingAmount => "no amount",
        RowError::InvalidAmount(AmountError::Malformed) => {
            "invalid amount: not digits with an optional fraction"
        }
        RowError::InvalidAmount(AmountError::TooPrecise) => {
            "invalid amount: more than four decimal places"
        }
        RowError::InvalidAmount(AmountError::TooLarge) => "invalid amount: 10^15 or more",
        RowError::InvalidAmount(AmountError::Zero) => "invalid amount: zero",
    }
}
