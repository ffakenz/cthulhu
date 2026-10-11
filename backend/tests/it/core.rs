//! Laws of the core: the decider folded over generated commands the way a runtime takes them,
//! each law asked after every step.

use std::collections::{BTreeMap, BTreeSet};

use backend::core::{decide_client, decide_dispute, evolve, open};
use backend::domain::DisputeCommand::{Chargeback, Dispute, Resolve};
use backend::domain::Status::{Disputed, Settled};
use backend::domain::{
    Account, Address, Amount, Balance, ClientCommand, ClientEvent, ClientId, ClientReason, Command,
    Deposit, DisputeCommand, DisputeEvent, DisputeReason, Event, Footprint, Reason, Sight, Status,
    TxId,
};

use crate::support::model::{self, Units};
use crate::support::{self, Failure, generate};

/// One command as the runtime took it.
#[derive(Debug, Clone, Copy)]
struct Step {
    intake: Intake,
    start: Start,
    outcome: Outcome,
}

/// The command as intake passed it on, with its sight for a client command.
#[derive(Debug, Clone, Copy)]
enum Intake {
    Client(ClientCommand, Sight),
    Dispute(DisputeCommand),
}

/// Where the footprint the command was decided on came from: the world, or `open` for a client
/// with no account.
#[derive(Debug, Clone, Copy)]
enum Start {
    Loaded(Footprint),
    Opened,
}

/// What the decider answered: an event and the footprint `evolve` made from it, or a rejection.
#[derive(Debug, Clone, Copy)]
enum Outcome {
    Accepted { event: Event, next: Footprint },
    Rejected(Reason),
}

/// A client's running sums of what was deposited, withdrawn and charged back.
#[derive(Debug, Clone, Copy, Default)]
struct Books {
    deposited: Units,
    withdrawn: Units,
    charged_back: Units,
}

/// What the stores and intake would hold, with each client's history and books: the world the
/// laws read.
#[derive(Debug, Default)]
struct World {
    accounts: BTreeMap<ClientId, Account>,
    history: BTreeMap<ClientId, BTreeMap<TxId, Deposit>>,
    seen: BTreeSet<TxId>,
    books: BTreeMap<ClientId, Books>,
}

/// The decision for `intake` on `footprint`, routed and widened as the runtime does it.
fn decide(intake: &Intake, footprint: &Footprint) -> Result<Event, Reason> {
    match intake {
        Intake::Client(command, sight) => decide_client(footprint.account(), command, *sight)
            .map(Event::Client)
            .map_err(Reason::Client),
        Intake::Dispute(command) => decide_dispute(footprint.deposit(), command)
            .map(Event::Dispute)
            .map_err(Reason::Dispute),
    }
}

impl Intake {
    fn address(&self) -> Address {
        match self {
            Self::Client(command, _) => command.address(),
            Self::Dispute(command) => command.address(),
        }
    }
}

impl Start {
    /// The footprint the command was decided on.
    fn footprint(&self) -> Footprint {
        match *self {
            Self::Loaded(footprint) => footprint,
            Self::Opened => open(),
        }
    }
}

impl Outcome {
    /// What `decide` answered.
    fn decision(&self) -> Result<Event, Reason> {
        match *self {
            Self::Accepted { event, .. } => Ok(event),
            Self::Rejected(reason) => Err(reason),
        }
    }
}

impl Step {
    /// The footprint the step committed: the evolved one, or the one it was decided on.
    fn after(&self) -> Footprint {
        match self.outcome {
            Outcome::Accepted { next, .. } => next,
            Outcome::Rejected(_) => self.start.footprint(),
        }
    }
}

impl Books {
    /// The books after `decision`: an accepted deposit, withdrawal or chargeback moves its sum.
    fn after(self, decision: Result<Event, Reason>) -> Option<Self> {
        Some(match decision {
            Ok(Event::Client(ClientEvent::Deposited(amount))) => {
                Self { deposited: self.deposited.plus(units(amount)?), ..self }
            }
            Ok(Event::Client(ClientEvent::Withdrawn(amount))) => {
                Self { withdrawn: self.withdrawn.plus(units(amount)?), ..self }
            }
            Ok(Event::Dispute(DisputeEvent::ChargedBack(amount))) => {
                Self { charged_back: self.charged_back.plus(units(amount)?), ..self }
            }
            Ok(
                Event::Opened | Event::Dispute(DisputeEvent::Held(_) | DisputeEvent::Released(_)),
            )
            | Err(_) => self,
        })
    }

    /// Whether a chargeback happened: every chargeback moves at least one unit.
    fn charged(&self) -> bool {
        self.charged_back > Units::ZERO
    }
}

impl World {
    /// One command through intake, load, open, decide, evolve and commit at its address, with the
    /// outcome booked; `None` if `evolve` overflowed or an amount did not read back as units.
    fn step(&mut self, command: Command) -> Option<Step> {
        let intake = match command {
            Command::Client(client_command) => {
                let sight = if self.seen.insert(client_command.address().tx()) {
                    Sight::First
                } else {
                    Sight::Repeat
                };
                Intake::Client(client_command, sight)
            }
            Command::Dispute(dispute_command) => Intake::Dispute(dispute_command),
        };
        let address = intake.address();
        let start = match self.accounts.get(&address.client()) {
            Some(&account) => Start::Loaded(Footprint::new(account, self.deposit(address))),
            None => Start::Opened,
        };
        let outcome = match decide(&intake, &start.footprint()) {
            Ok(event) => {
                Outcome::Accepted { event, next: evolve(Some(start.footprint()), &event).ok()? }
            }
            Err(reason) => Outcome::Rejected(reason),
        };
        let step = Step { intake, start, outcome };
        let after = step.after();
        let _ = self.accounts.insert(address.client(), *after.account());
        let history = self.history.entry(address.client()).or_default();
        let _ = match after.deposit() {
            Some(&deposit) => history.insert(address.tx(), deposit),
            None => history.remove(&address.tx()),
        };
        let books = self.books.entry(address.client()).or_default();
        *books = books.after(outcome.decision())?;
        Some(step)
    }

    /// The deposit in history at `address`.
    fn deposit(&self, address: Address) -> Option<Deposit> {
        self.history.get(&address.client()).and_then(|history| history.get(&address.tx())).copied()
    }

    /// The client's books; a client no command has named has none.
    fn books(&self, client: ClientId) -> Books {
        self.books.get(&client).copied().unwrap_or_default()
    }

    /// The sum of the client's `Disputed` deposits.
    fn disputed(&self, client: ClientId) -> Option<Units> {
        self.history
            .get(&client)
            .into_iter()
            .flat_map(BTreeMap::values)
            .filter(|deposit| deposit.status() == Status::Disputed)
            .try_fold(Units::ZERO, |sum, deposit| Some(sum.plus(units(deposit.amount())?)))
    }
}

/// An amount in units of 0.0001, read back through the grammar model.
fn units(amount: Amount) -> Option<Units> {
    model::amount_units(&amount.to_string()).ok().map(Units::from)
}

/// Runs `commands` through a fresh world and asks `law` after every step, of the step and the
/// world as the step left it; a step the world cannot take fails the law.
fn holds(commands: &[Command], law: impl Fn(&World, &Step) -> bool) -> bool {
    commands
        .iter()
        .try_fold(World::default(), |mut world, &command| {
            let step = world.step(command)?;
            law(&world, &step).then_some(world)
        })
        .is_some()
}

/// A law over every step of generated runs.
fn driver_law(label: &'static str, law: impl Fn(&World, &Step) -> bool) -> Result<(), Failure> {
    support::for_all(label, generate::commands(), |commands| holds(commands, &law))
}

/// The dispute lifecycle: from a deposit's status, the status a dispute command leaves, `None`
/// once charged back, or the reason it is refused.
fn transition(command: DisputeCommand, from: Status) -> Result<Option<Status>, DisputeReason> {
    match (command, from) {
        (Dispute(_), Settled) => Ok(Some(Disputed)),
        (Resolve(_), Disputed) => Ok(Some(Settled)),
        (Chargeback(_), Disputed) => Ok(None),
        (Dispute(_), Disputed) => Err(DisputeReason::NotSettled),
        (Resolve(_) | Chargeback(_), Settled) => Err(DisputeReason::NotDisputed),
    }
}

/// The event a dispute command answers when the lifecycle allows it.
fn dispute_event(command: DisputeCommand, amount: Amount) -> DisputeEvent {
    match command {
        Dispute(_) => DisputeEvent::Held(amount),
        Resolve(_) => DisputeEvent::Released(amount),
        Chargeback(_) => DisputeEvent::ChargedBack(amount),
    }
}

/// Whether a dispute command answered, and left its deposit, as the lifecycle says: an unknown
/// deposit is refused, an allowed transition answers its event, and a refusal changes nothing.
fn follows_lifecycle(
    command: DisputeCommand,
    before: Option<&Deposit>,
    decision: Result<Event, Reason>,
    after: Option<&Deposit>,
) -> bool {
    let Some(&deposit) = before else {
        return decision == Err(Reason::Dispute(DisputeReason::UnknownDeposit)) && after.is_none();
    };
    match transition(command, deposit.status()) {
        Ok(to) => {
            decision == Ok(Event::Dispute(dispute_event(command, deposit.amount())))
                && after.map(|left| (left.amount(), left.status()))
                    == to.map(|status| (deposit.amount(), status))
        }
        Err(reason) => decision == Err(Reason::Dispute(reason)) && after == Some(&deposit),
    }
}

/// The Commands table for client commands: the event a client command answers on `account`, or
/// the reason it is refused, checked in the table's order.
fn client_decision(
    command: ClientCommand,
    sight: Sight,
    account: &Account,
) -> Result<ClientEvent, ClientReason> {
    match (sight, account.is_locked(), command) {
        (Sight::Repeat, _, _) => Err(ClientReason::DuplicateTx),
        (Sight::First, true, _) => Err(ClientReason::AccountLocked),
        (Sight::First, false, ClientCommand::Deposit { amount, .. }) => {
            Ok(ClientEvent::Deposited(amount))
        }
        (Sight::First, false, ClientCommand::Withdrawal { amount, .. })
            if Balance::from(amount) > account.available() =>
        {
            Err(ClientReason::InsufficientFunds)
        }
        (Sight::First, false, ClientCommand::Withdrawal { amount, .. }) => {
            Ok(ClientEvent::Withdrawn(amount))
        }
    }
}

#[test]
fn prop_decide_never_changes_state() {
    let law =
        driver_law("deciding twice on the same footprint gives the same answer", |_, step| {
            let footprint = step.start.footprint();
            (0..2).all(|_| decide(&step.intake, &footprint) == step.outcome.decision())
        });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_evolve_is_total() {
    let law = support::for_all(
        "evolve applies any event to any footprint, or to none, and never fails below the amount cap",
        (generate::commands(), generate::events()),
        |(commands, events)| {
            holds(commands, |_, step| {
                [None, Some(step.start.footprint()), Some(step.after())]
                    .into_iter()
                    .all(|state| events.iter().all(|event| evolve(state, event).is_ok()))
            })
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_held_is_the_sum_of_disputed_deposits_and_never_negative() {
    let law = driver_law(
        "held equals the sum of the client's disputed deposits, and is never negative",
        |world, step| {
            let held = step.after().account().held();
            held >= Balance::ZERO
                && world
                    .disputed(step.intake.address().client())
                    .is_some_and(|disputed| held.to_string() == disputed.to_string())
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_total_is_deposited_minus_withdrawn_minus_charged_back() {
    let law =
        driver_law("total equals deposited minus withdrawn minus charged back", |world, step| {
            let books = world.books(step.intake.address().client());
            let expected = books.deposited.minus(books.withdrawn).minus(books.charged_back);
            step.after()
                .account()
                .total()
                .is_some_and(|total| total.to_string() == expected.to_string())
        });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_locked_exactly_when_charged_back() {
    let law =
        driver_law("an account is locked exactly when a chargeback happened", |world, step| {
            step.after().account().is_locked()
                == world.books(step.intake.address().client()).charged()
        });
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_only_held_makes_available_negative() {
    let law = driver_law(
        "available falls below zero, or further below it, only through Held",
        |_, step| {
            let before = step.start.footprint().account().available();
            let after = step.after().account().available();
            let fell = after < Balance::ZERO && after < before;
            !fell
                || matches!(
                    step.outcome,
                    Outcome::Accepted { event: Event::Dispute(DisputeEvent::Held(_)), .. }
                )
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_dispute_lifecycle_allows_exactly_its_transitions() {
    let law = driver_law(
        "a deposit enters history Settled, and dispute commands move it exactly as the lifecycle table says",
        |_, step| match step.intake {
            Intake::Dispute(command) => follows_lifecycle(
                command,
                step.start.footprint().deposit(),
                step.outcome.decision(),
                step.after().deposit(),
            ),
            Intake::Client(ClientCommand::Deposit { amount, .. }, _) => match step.outcome {
                Outcome::Accepted { .. } => step.after().deposit().is_some_and(|deposit| {
                    deposit.amount() == amount && deposit.status() == Settled
                }),
                Outcome::Rejected(_) => true,
            },
            Intake::Client(ClientCommand::Withdrawal { .. }, _) => true,
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_client_commands_follow_the_commands_table() {
    let law = driver_law(
        "a client command is refused for a repeated tx id, then a locked account, then a withdrawal above available, and is otherwise deposited or withdrawn",
        |_, step| match step.intake {
            Intake::Client(command, sight) => {
                step.outcome.decision()
                    == client_decision(command, sight, step.start.footprint().account())
                        .map(Event::Client)
                        .map_err(Reason::Client)
            }
            Intake::Dispute(_) => true,
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}

#[test]
fn prop_first_command_opens_the_account() {
    let law = driver_law(
        "the first command naming a client opens its account with zero balances, unlocked, and opening an open account changes nothing",
        |_, step| match step.start {
            Start::Loaded(footprint) => evolve(Some(footprint), &Event::Opened) == Ok(footprint),
            Start::Opened => {
                let opened = open();
                let account = opened.account();
                let empty = account.available() == Balance::ZERO
                    && account.held() == Balance::ZERO
                    && account.total() == Some(Balance::ZERO)
                    && !account.is_locked()
                    && opened.deposit().is_none()
                    && evolve(None, &Event::Opened) == Ok(opened);
                empty
                    && match step.outcome {
                        Outcome::Accepted { event, next } => evolve(None, &event) == Ok(next),
                        Outcome::Rejected(_) => true,
                    }
            }
        },
    );
    if let Err(failure) = law {
        panic!("{failure}");
    }
}
