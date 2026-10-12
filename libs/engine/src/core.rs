//! The decider: checks of a command against the footprint it touches, and the computation of the
//! next footprint from an accepted event. Pure and total; the initial state is `None`, a client
//! with no account.

use crate::domain::{
    Account, Amount, Balance, ClientCommand, ClientEvent, ClientReason, Deposit, DisputeCommand,
    DisputeEvent, DisputeReason, Event, Footprint, Lock, Sight, Status,
};

/// A balance would leave the range the decimal type holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a balance overflowed")]
pub struct Overflow;

/// The footprint `Opened` makes: zero balances, unlocked, no deposit.
pub fn open() -> Footprint {
    Footprint::new(Account::new(Balance::ZERO, Balance::ZERO, Lock::Unlocked), None)
}

/// The event a client command makes on `account`, or why it is refused: a repeated tx id, then a
/// locked account, then a withdrawal above `available`.
pub fn decide_client(
    account: &Account,
    command: &ClientCommand,
    sight: Sight,
) -> Result<ClientEvent, ClientReason> {
    match sight {
        Sight::Repeat => return Err(ClientReason::DuplicateTx),
        Sight::First => {}
    }
    match account.lock() {
        Lock::Locked => return Err(ClientReason::AccountLocked),
        Lock::Unlocked => {}
    }
    match *command {
        ClientCommand::Deposit { amount, .. } => Ok(ClientEvent::Deposited(amount)),
        ClientCommand::Withdrawal { amount, .. } if Balance::from(amount) > account.available() => {
            Err(ClientReason::InsufficientFunds)
        }
        ClientCommand::Withdrawal { amount, .. } => Ok(ClientEvent::Withdrawn(amount)),
    }
}

/// The event a dispute command makes on the deposit at its address, or why it is refused: no
/// deposit there, then a status the dispute lifecycle does not move from.
pub fn decide_dispute(
    deposit: Option<&Deposit>,
    command: &DisputeCommand,
) -> Result<DisputeEvent, DisputeReason> {
    let Some(deposit) = deposit else {
        return Err(DisputeReason::UnknownDeposit);
    };
    let amount = deposit.amount();
    match (command, deposit.status()) {
        (DisputeCommand::Dispute(_), Status::Settled) => Ok(DisputeEvent::Held(amount)),
        (DisputeCommand::Dispute(_), Status::Disputed) => Err(DisputeReason::NotSettled),
        (DisputeCommand::Resolve(_), Status::Disputed) => Ok(DisputeEvent::Released(amount)),
        (DisputeCommand::Chargeback(_), Status::Disputed) => Ok(DisputeEvent::ChargedBack(amount)),
        (DisputeCommand::Resolve(_) | DisputeCommand::Chargeback(_), Status::Settled) => {
            Err(DisputeReason::NotDisputed)
        }
    }
}

/// The footprint `event` leaves at its command's address, starting from `state`, or `open()` for
/// a client with no account. `Opened` on an open account changes nothing.
pub fn evolve(state: Option<Footprint>, event: &Event) -> Result<Footprint, Overflow> {
    let footprint = state.unwrap_or_else(open);
    let account = footprint.account();
    let (available, held, lock) = (account.available(), account.held(), account.lock());
    let next = match *event {
        Event::Opened => footprint,
        Event::Client(ClientEvent::Deposited(amount)) => Footprint::new(
            Account::new(plus(available, amount)?, held, lock),
            Some(Deposit::new(amount, Status::Settled)),
        ),
        Event::Client(ClientEvent::Withdrawn(amount)) => Footprint::new(
            Account::new(minus(available, amount)?, held, lock),
            footprint.deposit().copied(),
        ),
        Event::Dispute(DisputeEvent::Held(amount)) => Footprint::new(
            Account::new(minus(available, amount)?, plus(held, amount)?, lock),
            Some(Deposit::new(amount, Status::Disputed)),
        ),
        Event::Dispute(DisputeEvent::Released(amount)) => Footprint::new(
            Account::new(plus(available, amount)?, minus(held, amount)?, lock),
            Some(Deposit::new(amount, Status::Settled)),
        ),
        Event::Dispute(DisputeEvent::ChargedBack(amount)) => {
            Footprint::new(Account::new(available, minus(held, amount)?, Lock::Locked), None)
        }
    };
    Ok(next)
}

/// `balance` plus `amount`, or `Overflow`.
fn plus(balance: Balance, amount: Amount) -> Result<Balance, Overflow> {
    balance.checked_add(Balance::from(amount)).ok_or(Overflow)
}

/// `balance` minus `amount`, or `Overflow`.
fn minus(balance: Balance, amount: Amount) -> Result<Balance, Overflow> {
    balance.checked_sub(Balance::from(amount)).ok_or(Overflow)
}
