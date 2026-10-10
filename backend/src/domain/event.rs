//! Events, the facts the core accepted; the reasons a rejection carries; and intake's sight of a
//! tx id.

use crate::domain::Amount;

/// A fact the core accepted. It carries what `evolve` needs and no id: it applies at its command's
/// address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The account was created, with zero balances and unlocked.
    Opened,
    /// A client command was accepted.
    Client(ClientEvent),
    /// A dispute command was accepted.
    Dispute(DisputeEvent),
}

/// What an accepted client command did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientEvent {
    /// The amount was added to available, and the deposit recorded as settled.
    Deposited(Amount),
    /// The amount was taken from available.
    Withdrawn(Amount),
}

/// What an accepted dispute command did, with the disputed deposit's amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeEvent {
    /// The amount moved from available to held, and the deposit became disputed.
    Held(Amount),
    /// The amount moved from held back to available, and the deposit became settled.
    Released(Amount),
    /// The amount left held, the deposit left history, and the account locked.
    ChargedBack(Amount),
}

/// Why the core refused a command: the reason its rejection carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// A client command was refused.
    Client(ClientReason),
    /// A dispute command was refused.
    Dispute(DisputeReason),
    /// Applying the command's event would overflow a balance.
    Overflow,
}

/// Why a client command was refused. The variants are checked in order, and the first that
/// applies names the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClientReason {
    /// An earlier deposit or withdrawal used the tx id.
    DuplicateTx,
    /// A chargeback has locked the account.
    AccountLocked,
    /// A withdrawal's amount is above available.
    InsufficientFunds,
}

/// Why a dispute command was refused. The variants are checked in order, and the first that
/// applies names the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisputeReason {
    /// History holds no deposit of this client under that tx id.
    UnknownDeposit,
    /// A dispute named a deposit already disputed.
    NotSettled,
    /// A resolve or a chargeback named a deposit that is not disputed.
    NotDisputed,
}

/// Whether intake has seen a deposit's or a withdrawal's tx id before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sight {
    /// Never before: intake records it now.
    First,
    /// Already recorded, whatever that earlier command's outcome.
    Repeat,
}
