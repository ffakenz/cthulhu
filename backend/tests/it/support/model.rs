//! The model the laws compare against: the amount grammar, read apart from the production parser;
//! the model's money; and the naive engine, which states the overview's tables over plain numbers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU64;

use backend::domain::{AmountError, ClientId, ClientReason, DisputeReason, Reason};
use backend::run::RowError;

use super::generate::{InputRow, Kind};

/// What the amount grammar makes of `text`: its value in units of 0.0001, never zero, or why it
/// is refused.
pub(crate) fn amount_units(text: &str) -> Result<NonZeroU64, AmountError> {
    let mut whole = String::new();
    let mut fraction = String::new();
    let mut point = false;
    for character in text.chars() {
        match character {
            '0'..='9' if point => fraction.push(character),
            '0'..='9' => whole.push(character),
            '.' if !point => point = true,
            _ => return Err(AmountError::Malformed),
        }
    }
    if whole.is_empty() || (point && fraction.is_empty()) {
        return Err(AmountError::Malformed);
    }
    let significant_whole = whole.trim_start_matches('0');
    let significant_fraction = fraction.trim_end_matches('0');
    if significant_fraction.len() > 4 {
        return Err(AmountError::TooPrecise);
    }
    if significant_whole.len() > 15 {
        return Err(AmountError::TooLarge);
    }
    let units = format!("{significant_whole}{significant_fraction:0<4}")
        .parse::<u64>()
        .map_err(|_| AmountError::TooLarge)?;
    NonZeroU64::new(units).ok_or(AmountError::Zero)
}

/// An amount of money in units of 0.0001, signed: the model's money.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Units(i128);

impl Units {
    pub(crate) const ZERO: Self = Self(0);

    /// The sum, saturating at the bounds of `i128`, far past any amount the cap admits.
    pub(crate) fn plus(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// The difference, saturating likewise.
    pub(crate) fn minus(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl From<NonZeroU64> for Units {
    fn from(units: NonZeroU64) -> Self {
        Self(i128::from(units.get()))
    }
}

/// As a balance prints: a sign when negative, then exactly four decimals.
impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let magnitude = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:04}", magnitude / 10_000, magnitude % 10_000)
    }
}

/// An account as the model keeps it.
#[derive(Debug, Clone, Copy, Default)]
struct Account {
    available: Units,
    held: Units,
    locked: bool,
}

/// A deposit in the model's history.
#[derive(Debug, Clone, Copy)]
struct Deposit {
    amount: Units,
    disputed: bool,
}

/// The naive engine: accounts by client, history by client and tx id, the used tx ids, and the
/// counts.
#[derive(Debug, Default)]
struct Engine {
    accounts: BTreeMap<u16, Account>,
    deposits: BTreeMap<(u16, u32), Deposit>,
    seen: BTreeSet<u32>,
    rejected: BTreeMap<Reason, NonZeroU64>,
    malformed: BTreeMap<RowError, NonZeroU64>,
}

/// An account as the output prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrintedAccount {
    pub(crate) client: ClientId,
    pub(crate) available: String,
    pub(crate) held: String,
    pub(crate) total: Option<String>,
    pub(crate) locked: bool,
}

/// What a run reports, printed: the accounts by client id, then the counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Printed {
    pub(crate) accounts: Vec<PrintedAccount>,
    pub(crate) rejected: Vec<(Reason, NonZeroU64)>,
    pub(crate) malformed: Vec<(RowError, NonZeroU64)>,
}

/// What the naive engine prints for `rows`.
pub(crate) fn printed(rows: &[InputRow]) -> Printed {
    let engine = Engine::after(rows);
    Printed {
        accounts: engine
            .accounts
            .iter()
            .map(|(&client, account)| PrintedAccount {
                client: ClientId::from(client),
                available: account.available.to_string(),
                held: account.held.to_string(),
                total: Some(account.available.plus(account.held).to_string()),
                locked: account.locked,
            })
            .collect(),
        rejected: engine.rejected.into_iter().collect(),
        malformed: engine.malformed.into_iter().collect(),
    }
}

/// The accounts table the naive engine prints for `rows`: the header, then a line per account by
/// client id.
pub(crate) fn table(rows: &[InputRow]) -> String {
    let lines: Vec<String> = Engine::after(rows)
        .accounts
        .iter()
        .map(|(client, &Account { available, held, locked })| {
            let total = available.plus(held);
            format!("{client},{available},{held},{total},{locked}\n")
        })
        .collect();
    format!("client,available,held,total,locked\n{}", lines.concat())
}

impl Engine {
    /// The naive engine after taking `rows`, in order.
    fn after(rows: &[InputRow]) -> Self {
        let mut engine = Self::default();
        for input in rows {
            engine.take(input);
        }
        engine
    }

    /// One input row, as the overview's tables say.
    fn take(&mut self, input: &InputRow) {
        match input {
            InputRow::Unreadable(_) => count(&mut self.malformed, RowError::Unreadable),
            InputRow::TooManyFields { .. } => count(&mut self.malformed, RowError::TooManyFields),
            InputRow::Fields { kind, client, tx, amount } => match kind {
                Kind::Deposit => {
                    if let Some(units) = self.amount(amount.as_deref()) {
                        self.deposit(*client, *tx, units);
                    }
                }
                Kind::Withdrawal => {
                    if let Some(units) = self.amount(amount.as_deref()) {
                        self.withdraw(*client, *tx, units);
                    }
                }
                Kind::Dispute => self.dispute(*client, *tx),
                Kind::Resolve => self.resolve(*client, *tx),
                Kind::Chargeback => self.chargeback(*client, *tx),
            },
        }
    }

    /// A client command's amount in units, or `None` once the row is counted as malformed.
    fn amount(&mut self, text: Option<&str>) -> Option<Units> {
        match text.map(amount_units) {
            None => {
                count(&mut self.malformed, RowError::MissingAmount);
                None
            }
            Some(Err(refusal)) => {
                count(&mut self.malformed, RowError::InvalidAmount(refusal));
                None
            }
            Some(Ok(units)) => Some(Units::from(units)),
        }
    }

    /// The client's account, opened with zero balances and unlocked if absent.
    fn open(&mut self, client: u16) -> Account {
        *self.accounts.entry(client).or_default()
    }

    /// Intake and the checks both client commands make: the tx id is recorded on first sight and
    /// the account opened; a repeat is `DuplicateTx`, then a locked account `AccountLocked`.
    fn checked(&mut self, client: u16, tx: u32) -> Result<Account, ClientReason> {
        let first = self.seen.insert(tx);
        let account = self.open(client);
        match (first, account.locked) {
            (false, _) => Err(ClientReason::DuplicateTx),
            (true, true) => Err(ClientReason::AccountLocked),
            (true, false) => Ok(account),
        }
    }

    /// A deposit: available rises, and the deposit enters history undisputed.
    fn deposit(&mut self, client: u16, tx: u32, units: Units) {
        match self.checked(client, tx) {
            Err(reason) => count(&mut self.rejected, Reason::Client(reason)),
            Ok(account) => {
                let available = account.available.plus(units);
                let _ = self.accounts.insert(client, Account { available, ..account });
                let _ =
                    self.deposits.insert((client, tx), Deposit { amount: units, disputed: false });
            }
        }
    }

    /// A withdrawal: refused above available, otherwise available falls.
    fn withdraw(&mut self, client: u16, tx: u32, units: Units) {
        match self.checked(client, tx) {
            Err(reason) => count(&mut self.rejected, Reason::Client(reason)),
            Ok(account) if units > account.available => {
                count(&mut self.rejected, Reason::Client(ClientReason::InsufficientFunds));
            }
            Ok(account) => {
                let available = account.available.minus(units);
                let _ = self.accounts.insert(client, Account { available, ..account });
            }
        }
    }

    /// The account, opened if absent, and the deposit a dispute command names: `UnknownDeposit`
    /// when the client's history holds none under that tx id.
    fn named(&mut self, client: u16, tx: u32) -> Result<(Account, Deposit), DisputeReason> {
        let account = self.open(client);
        self.deposits
            .get(&(client, tx))
            .map(|&deposit| (account, deposit))
            .ok_or(DisputeReason::UnknownDeposit)
    }

    /// A dispute: refused on a disputed deposit, otherwise its amount moves to held.
    fn dispute(&mut self, client: u16, tx: u32) {
        match self.named(client, tx) {
            Err(reason) => count(&mut self.rejected, Reason::Dispute(reason)),
            Ok((_, deposit)) if deposit.disputed => {
                count(&mut self.rejected, Reason::Dispute(DisputeReason::NotSettled));
            }
            Ok((account, deposit)) => {
                let available = account.available.minus(deposit.amount);
                let held = account.held.plus(deposit.amount);
                let _ = self.accounts.insert(client, Account { available, held, ..account });
                let _ = self.deposits.insert((client, tx), Deposit { disputed: true, ..deposit });
            }
        }
    }

    /// A resolve: refused on an undisputed deposit, otherwise its amount moves back to available.
    fn resolve(&mut self, client: u16, tx: u32) {
        match self.named(client, tx) {
            Err(reason) => count(&mut self.rejected, Reason::Dispute(reason)),
            Ok((_, deposit)) if !deposit.disputed => {
                count(&mut self.rejected, Reason::Dispute(DisputeReason::NotDisputed));
            }
            Ok((account, deposit)) => {
                let available = account.available.plus(deposit.amount);
                let held = account.held.minus(deposit.amount);
                let _ = self.accounts.insert(client, Account { available, held, ..account });
                let _ = self.deposits.insert((client, tx), Deposit { disputed: false, ..deposit });
            }
        }
    }

    /// A chargeback: refused on an undisputed deposit, otherwise its amount leaves held, the
    /// deposit leaves history and the account locks.
    fn chargeback(&mut self, client: u16, tx: u32) {
        match self.named(client, tx) {
            Err(reason) => count(&mut self.rejected, Reason::Dispute(reason)),
            Ok((_, deposit)) if !deposit.disputed => {
                count(&mut self.rejected, Reason::Dispute(DisputeReason::NotDisputed));
            }
            Ok((account, deposit)) => {
                let held = account.held.minus(deposit.amount);
                let _ = self.accounts.insert(client, Account { held, locked: true, ..account });
                let _ = self.deposits.remove(&(client, tx));
            }
        }
    }
}

/// Adds one to `key`'s count, listing the key at one when it is new.
pub(crate) fn count<K: Ord>(counts: &mut BTreeMap<K, NonZeroU64>, key: K) {
    let _ = counts.entry(key).and_modify(|n| *n = n.saturating_add(1)).or_insert(NonZeroU64::MIN);
}
