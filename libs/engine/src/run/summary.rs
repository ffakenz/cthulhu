//! The views a run returns: the accounts by client id, and the report of what was refused or
//! skipped.

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use crate::domain::{Account, ClientId, Reason};
use crate::run::RowError;

/// What a run returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Every account, by client id.
    pub accounts: Accounts,
    /// The rejections by reason, and the malformed rows by kind.
    pub report: Report,
}

/// The accounts view: each client id with its account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accounts(BTreeMap<ClientId, Account>);

impl Accounts {
    /// Each client id with its account, by client id.
    pub fn by_client(&self) -> impl Iterator<Item = (ClientId, Account)> + '_ {
        self.0.iter().map(|(&client, &account)| (client, account))
    }
}

impl FromIterator<(ClientId, Account)> for Accounts {
    fn from_iter<I: IntoIterator<Item = (ClientId, Account)>>(accounts: I) -> Self {
        Self(accounts.into_iter().collect())
    }
}

/// How many commands were rejected for each reason, and how many rows were malformed of each
/// kind. A reason or a kind is listed only once it has happened, so every count is at least one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    rejected: BTreeMap<Reason, NonZeroU64>,
    malformed: BTreeMap<RowError, NonZeroU64>,
}

impl Report {
    /// Nothing rejected and nothing malformed.
    pub(super) const EMPTY: Self = Self { rejected: BTreeMap::new(), malformed: BTreeMap::new() };

    /// The report with one more rejection for `reason`.
    pub(super) fn with_rejection(mut self, reason: Reason) -> Self {
        count(&mut self.rejected, reason);
        self
    }

    /// The report with one more malformed row of `error`'s kind.
    pub(super) fn with_malformed(mut self, error: RowError) -> Self {
        count(&mut self.malformed, error);
        self
    }

    /// Each reason with how many commands it rejected, by reason.
    pub fn rejected(&self) -> impl Iterator<Item = (Reason, NonZeroU64)> + '_ {
        self.rejected.iter().map(|(&reason, &count)| (reason, count))
    }

    /// Each kind with how many malformed rows it counts, by kind.
    pub fn malformed(&self) -> impl Iterator<Item = (RowError, NonZeroU64)> + '_ {
        self.malformed.iter().map(|(&error, &count)| (error, count))
    }
}

/// Adds one to `key`'s count, listing the key at one when it is new.
fn count<K: Ord>(counts: &mut BTreeMap<K, NonZeroU64>, key: K) {
    let _ = counts.entry(key).and_modify(|n| *n = n.saturating_add(1)).or_insert(NonZeroU64::MIN);
}
