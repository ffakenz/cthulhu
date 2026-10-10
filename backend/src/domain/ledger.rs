//! A client's ledger, one footprint at a time: its account, the deposits in its history, and what
//! one command touches of them.

use crate::domain::{Amount, Balance};

/// A client's account: available and held funds, and whether a chargeback has locked it. Its
/// client id is the key it is stored under; the total is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Account {
    available: Balance,
    held: Balance,
    lock: Lock,
}

impl Account {
    /// Funds not held by a dispute; only a dispute takes them below zero.
    pub fn available(&self) -> Balance {
        self.available
    }

    /// Funds held by disputes: the sum of the disputed deposits' amounts.
    pub fn held(&self) -> Balance {
        self.held
    }

    /// Available plus held, or `None` when the sum overflows.
    pub fn total(&self) -> Option<Balance> {
        self.available.checked_add(self.held)
    }

    /// Whether a chargeback has locked the account.
    pub fn is_locked(&self) -> bool {
        match self.lock {
            Lock::Locked => true,
            Lock::Unlocked => false,
        }
    }
}

/// Whether a chargeback has locked an account; nothing unlocks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lock {
    /// No chargeback has happened on the account.
    Unlocked,
    /// A chargeback has happened on the account.
    Locked,
}

/// A deposit in history: its amount and where it stands in its dispute lifecycle. Its client and
/// tx id are the key it is stored under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deposit {
    amount: Amount,
    status: Status,
}

impl Deposit {
    /// The amount deposited.
    pub fn amount(&self) -> Amount {
        self.amount
    }

    /// Whether a dispute holds the deposit.
    pub fn status(&self) -> Status {
        self.status
    }
}

/// Where a deposit stands in its dispute lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// No dispute holds it: a dispute may.
    Settled,
    /// A dispute holds it, until a resolve settles it or a chargeback removes it.
    Disputed,
}

/// What one command touches, at its address (its client and tx id): the account, and the deposit
/// under that tx id, `None` when nothing was recorded there or a chargeback removed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Footprint {
    account: Account,
    deposit: Option<Deposit>,
}

impl Footprint {
    /// The footprint holding `account` and the deposit at the same address, if any.
    pub fn new(account: Account, deposit: Option<Deposit>) -> Self {
        Self { account, deposit }
    }

    /// The account.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// The deposit at the address, `None` when history holds none there.
    pub fn deposit(&self) -> Option<&Deposit> {
        self.deposit.as_ref()
    }
}
