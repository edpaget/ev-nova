//! A storyline walker: drives a [`Session`] through a mission chain from a
//! script of [`Step`]s, through the session's public API alone, so a test
//! can play a storyline without flying it.
//!
//! - **`take <mission>`** ([`Step::Take`]) accepts the mission where it is
//!   offered. Offered at the stellar docked at, it is accepted there.
//!   Otherwise the walker lands ([`Session::land_at`]) at every stellar the
//!   catalog lists, each at the first system listing it, by ascending ID,
//!   the stellar docked at last, so that landing elsewhere first rolls its
//!   offers afresh; it stops at the first that offers the mission, and a
//!   stellar it cannot land at is skipped. It reads why each stellar does
//!   not offer it ([`Session::offer_refusals`]), never deriving `AvailStel`
//!   itself.
//! - **`deliver <mission>`** ([`Step::Deliver`]) lands at the active
//!   mission's destination, when it is not reached and is not its return
//!   (where a `PickupMode` 1 cargo is picked up), then at its return, and
//!   checks that the mission completed, unfailed, paid what its `PayVal`
//!   pays ([`expected_pay`]) and set the bits its `OnSuccess` sets
//!   ([`expected_bits`]).
//! - **`pass <n> days`** ([`Step::PassDays`]) takes off, when docked, then
//!   lets `n` days go by ([`Session::let_days_pass`]): the take-off passes
//!   its own days by the rules, on top of `n`. The walker stays in flight,
//!   and its next landing is a walker's landing. A mission whose deadline
//!   lapses fails, in flight, as the missions' deadline check says.
//!
//! Every action is settled ([`Session::settle_script`]) before the next,
//! so the `Landed`, `DayPassed` and `TookOff` subscriptions run. The
//! landings, settlings and accepts draw on [`NeverFires`], whose rolls
//! are all 1: every `AvailRandom` above 0 passes and the offer rolls are
//! forced through the source the session is handed, with no state of its
//! own. The days draw on the walker's day chance, [`NeverFires`] unless
//! [`StoryWalker::with_day_chance`] says otherwise.
//!
//! A failed step stops the walk with a [`WalkFailure`]: which step, what
//! went wrong ([`Fault`]), where it looked ([`Looked`]) and which
//! availability fields refused it at each stellar, and the control bits
//! set. Its `Display` is the report a stock test prints.
//!
//! The walker skips flight: ship goals, crons and ranks get their own step
//! kinds as their features land. A mission whose goals need ships is never
//! completed by a landing, and is reported [`Fault::NotCompleted`].

use std::fmt;

use crate::catalog::{
    ListedStellar, MissionId, MissionRecord, PilotCatalog, StellarId, SystemId, TrafficCatalog,
};
use crate::chance::{Chance, NeverFires};
use crate::control::{Bit, Script, SetOp};
use crate::date::GameDate;
use crate::mission::{AcceptError, ActiveMission, PayEffect, Refusal, Stop, Venue, pay_effect};
use crate::rules::Verdict;
use crate::session::{RelocateRefusal, Session};

/// One step of a walker's script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Find a stellar offering the mission, land there and accept it.
    Take(MissionId),
    /// Take the active mission to its destination and its return, and
    /// check it completed.
    Deliver(MissionId),
    /// Take off, when docked, and let so many days go by.
    PassDays(u32),
}

impl fmt::Display for Step {
    /// "take 504", "deliver 504", "pass 3 days" or "pass 1 day".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Take(mission) => write!(f, "take {}", mission.0),
            Self::Deliver(mission) => write!(f, "deliver {}", mission.0),
            Self::PassDays(1) => write!(f, "pass 1 day"),
            Self::PassDays(days) => write!(f, "pass {days} days"),
        }
    }
}

/// What one step did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// The mission was accepted into `slot`, docked at `stellar`.
    Took {
        /// The mission.
        mission: MissionId,
        /// Where it was accepted.
        stellar: StellarId,
        /// The slot it went into.
        slot: u8,
    },
    /// The mission completed at its return, `stellar`, its cash pay
    /// `pay`.
    Delivered {
        /// The mission.
        mission: MissionId,
        /// Its return.
        stellar: StellarId,
        /// How much the cash rose.
        pay: i64,
    },
    /// The days went by: `days`, and before them, when the ship was
    /// docked, the take-off's own days, as the rules pass them.
    Passed {
        /// The days asked for.
        days: u32,
        /// Whether the ship took off first.
        took_off: bool,
        /// The date after.
        date: GameDate,
        /// The missions held before the step that failed, or are no longer
        /// held, after it, in slot order.
        failed: Vec<MissionId>,
    },
}

/// What a walk did: one entry for each step, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walked {
    /// What each step did.
    pub done: Vec<Done>,
}

/// What went wrong with a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fault {
    /// No stellar offers the mission.
    NotOffered,
    /// It was offered, but the accept refused it.
    Refused(AcceptError),
    /// No slot holds the mission.
    NotActive,
    /// It was not completed at its return: failed, or not reached, or its
    /// goals not met. Its state.
    NotCompleted(Box<ActiveMission>),
    /// The cash rose by `got`, not by the `expected` pay.
    WrongPay {
        /// What its `PayVal` pays.
        expected: i64,
        /// How much the cash rose.
        got: i64,
    },
    /// These bits its `OnSuccess` sets are not set.
    BitsNotSet(Vec<Bit>),
    /// A landing was refused.
    CannotLand(RelocateRefusal),
    /// It has no return stellar, so no landing completes it.
    Undeliverable,
}

/// A stellar the walker landed at looking for a mission, and why the
/// mission is not offered there: each availability check that refused,
/// by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Looked {
    /// The stellar.
    pub stellar: StellarId,
    /// Its system.
    pub system: SystemId,
    /// Each check that refused it, by name, in the checks' order.
    pub refusals: Vec<(&'static str, Refusal)>,
}

/// Why a walk stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkFailure {
    /// The step that failed, counted from 1.
    pub step: usize,
    /// What it was.
    pub at: Step,
    /// What went wrong.
    pub fault: Fault,
    /// Where a `take` looked: only the stellars whose `AvailStel` takes
    /// them, or every stellar when none does. Empty for any other fault.
    pub looked: Vec<Looked>,
    /// Every control bit set.
    pub bits: Vec<Bit>,
}

impl fmt::Display for WalkFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "step {} ({}) failed: ", self.step, self.at)?;
        let mission = match self.at {
            Step::Take(mission) | Step::Deliver(mission) => mission.0,
            Step::PassDays(_) => -1,
        };
        match &self.fault {
            Fault::NotOffered => write!(f, "mission {mission} is offered nowhere")?,
            Fault::Refused(error) => write!(f, "mission {mission} was refused: {error}")?,
            Fault::NotActive => write!(f, "mission {mission} is not active")?,
            Fault::NotCompleted(active) => {
                write!(f, "mission {mission} was not completed: {active:?}")?;
            }
            Fault::WrongPay { expected, got } => {
                write!(f, "mission {mission} paid {got}, not {expected}")?;
            }
            Fault::BitsNotSet(bits) => {
                write!(f, "mission {mission} left unset:")?;
                for bit in bits {
                    write!(f, " b{}", bit.get())?;
                }
            }
            Fault::CannotLand(refusal) => write!(f, "mission {mission}: cannot land: {refusal}")?,
            Fault::Undeliverable => write!(f, "mission {mission} has no return stellar")?,
        }
        for looked in &self.looked {
            write!(
                f,
                "\n  stellar {} in system {}:",
                looked.stellar.0, looked.system.0
            )?;
            for (name, refusal) in &looked.refusals {
                write!(f, " {name} ({refusal:?})")?;
            }
        }
        write!(f, "\n  bits set:")?;
        for bit in &self.bits {
            write!(f, " b{}", bit.get())?;
        }
        Ok(())
    }
}

impl std::error::Error for WalkFailure {}

/// A step's failure: what went wrong, and where a `take` looked.
type Failed = (Fault, Vec<Looked>);

/// Walks a [`Session`] through a script of [`Step`]s over `catalog` (see
/// the module docs).
pub struct StoryWalker<'c, C: PilotCatalog + TrafficCatalog + ?Sized> {
    catalog: &'c C,
    day_chance: Box<dyn Chance + 'c>,
}

impl<'c, C: PilotCatalog + TrafficCatalog + ?Sized> StoryWalker<'c, C> {
    /// A walker over `catalog`, its days drawing on [`NeverFires`].
    #[must_use]
    pub fn new(catalog: &'c C) -> Self {
        Self {
            catalog,
            day_chance: Box::new(NeverFires),
        }
    }

    /// This walker, its days (the take-off, the daily step and their
    /// settling) drawing on `chance`.
    #[must_use]
    pub fn with_day_chance(self, chance: impl Chance + 'c) -> Self {
        Self {
            day_chance: Box::new(chance),
            ..self
        }
    }

    /// Walks `session` through `script`, a step at a time: what each step
    /// did.
    ///
    /// # Errors
    ///
    /// The first step that fails, with the report (see [`WalkFailure`]);
    /// the steps after it are not taken.
    pub fn walk(&mut self, session: &mut Session, script: &[Step]) -> Result<Walked, WalkFailure> {
        let mut walked = Walked::default();
        for (index, &step) in script.iter().enumerate() {
            let done = match step {
                Step::Take(mission) => self.take(session, mission),
                Step::Deliver(mission) => self.deliver(session, mission),
                Step::PassDays(days) => Ok(self.pass(session, days)),
            };
            match done {
                Ok(done) => walked.done.push(done),
                Err((fault, looked)) => {
                    return Err(WalkFailure {
                        step: index + 1,
                        at: step,
                        fault,
                        looked,
                        bits: session.pilot().control_bits().iter().collect(),
                    });
                }
            }
        }
        Ok(walked)
    }

    /// The catalog's record of `mission`.
    fn record(&self, mission: MissionId) -> Option<MissionRecord> {
        self.catalog
            .missions()
            .into_iter()
            .find(|record| record.id == mission)
    }

    /// Settles what the last action left, every roll forced.
    fn settle(&self, session: &mut Session) {
        let _ = session.settle_script(self.catalog, &mut NeverFires);
    }

    /// `take <mission>` (see the module docs).
    fn take(&self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
        let Some(record) = self.record(mission) else {
            return Err((Fault::NotOffered, Vec::new()));
        };
        let venue = Venue::of(record.avail_loc);
        if session.landed().is_some() && offered(session, mission, venue) {
            return self.accept(session, mission);
        }
        let mut looked = Vec::new();
        for (stellar, system) in candidates(&self.catalog.listed_stellars(), session.landed()) {
            if session.land_at(self.catalog, system, stellar).is_err() {
                continue;
            }
            self.settle(session);
            if offered(session, mission, venue) {
                return self.accept(session, mission);
            }
            looked.push(Looked {
                stellar,
                system,
                refusals: refusals(session, mission),
            });
        }
        Err((Fault::NotOffered, kept(looked)))
    }

    /// Accepts `mission`, offered at the stellar docked at.
    fn accept(&self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
        let stellar = session.landed().unwrap_or(StellarId(-1));
        let slot = session
            .accept_mission(mission, &mut NeverFires)
            .map_err(|error| (Fault::Refused(error), Vec::new()))?;
        self.settle(session);
        Ok(Done::Took {
            mission,
            stellar,
            slot,
        })
    }

    /// `deliver <mission>` (see the module docs).
    fn deliver(&self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
        let held = session
            .pilot()
            .missions()
            .iter()
            .find(|(_, active)| active.mission == mission)
            .map(|(slot, active)| (slot, active.clone()));
        let Some((slot, active)) = held else {
            return Err((Fault::NotActive, Vec::new()));
        };
        let Some(back) = active.return_to else {
            return Err((Fault::Undeliverable, Vec::new()));
        };
        if let Some(destination) = active.destination
            && !active.reached
            && destination.stellar != back.stellar
        {
            self.land(session, destination)?;
        }
        if let Some(active) = still_held(session, slot, mission)
            && active.failed.is_some()
        {
            return Err((Fault::NotCompleted(Box::new(active)), Vec::new()));
        }
        let cash = session.pilot().cash();
        self.land(session, back)?;
        if let Some(active) = still_held(session, slot, mission) {
            return Err((Fault::NotCompleted(Box::new(active)), Vec::new()));
        }
        let got = session.pilot().cash() - cash;
        if let Some(expected) = expected_pay(active.pay)
            && got != expected
        {
            return Err((Fault::WrongPay { expected, got }, Vec::new()));
        }
        let on_success = self.record(mission).map(|record| record.on_success);
        let unset: Vec<Bit> = on_success
            .map(|script| expected_bits(&script))
            .unwrap_or_default()
            .into_iter()
            .filter(|&bit| !session.pilot().control_bits().get(bit))
            .collect();
        if !unset.is_empty() {
            return Err((Fault::BitsNotSet(unset), Vec::new()));
        }
        Ok(Done::Delivered {
            mission,
            stellar: back.stellar,
            pay: got,
        })
    }

    /// Lands at `stop` and settles.
    fn land(&self, session: &mut Session, stop: Stop) -> Result<(), Failed> {
        let landed = match stop.system {
            Some(system) => session.land_at(self.catalog, system, stop.stellar),
            None => Err(RelocateRefusal::NoSystem),
        };
        landed.map_err(|refusal| (Fault::CannotLand(refusal), Vec::new()))?;
        self.settle(session);
        Ok(())
    }

    /// `pass <days> days` (see the module docs).
    fn pass(&mut self, session: &mut Session, days: u32) -> Done {
        let held: Vec<(u8, MissionId)> = session
            .pilot()
            .missions()
            .iter()
            .map(|(slot, active)| (slot, active.mission))
            .collect();
        let chance = &mut *self.day_chance;
        let took_off = session.take_off(chance).is_some();
        session.let_days_pass(days, chance);
        let _ = session.settle_script(self.catalog, chance);
        let failed = held
            .into_iter()
            .filter(|&(slot, mission)| {
                still_held(session, slot, mission).is_none_or(|active| active.failed.is_some())
            })
            .map(|(_, mission)| mission)
            .collect();
        Done::Passed {
            days,
            took_off,
            date: session.date(),
            failed,
        }
    }
}

/// Whether `mission` is offered at `venue` on the visit under way.
fn offered(session: &Session, mission: MissionId, venue: Venue) -> bool {
    session
        .mission_offers(venue)
        .iter()
        .any(|offer| offer.mission == mission)
}

/// Why `mission` is not offered at the stellar docked at: each check that
/// refused, by name.
fn refusals(session: &Session, mission: MissionId) -> Vec<(&'static str, Refusal)> {
    session
        .offer_refusals(mission)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, verdict)| match verdict {
            Verdict::Refuses(refusal) => Some((name, refusal)),
            Verdict::Draws => None,
        })
        .collect()
}

/// `slot`'s mission, while it is still `mission`.
fn still_held(session: &Session, slot: u8, mission: MissionId) -> Option<ActiveMission> {
    session
        .pilot()
        .missions()
        .get(slot)
        .filter(|active| active.mission == mission)
        .cloned()
}

/// The stellars a `take` lands at, in order: every one `listed`, at the
/// first system listing it, by ascending ID, and `docked` last. A stellar
/// no system lists is left out.
#[must_use]
pub fn candidates(
    listed: &[ListedStellar],
    docked: Option<StellarId>,
) -> Vec<(StellarId, SystemId)> {
    let mut found: Vec<(StellarId, SystemId)> = listed
        .iter()
        .filter_map(|stellar| Some((stellar.id, *stellar.systems.first()?)))
        .collect();
    found.sort_by_key(|&(stellar, _)| stellar);
    if let Some(index) = found
        .iter()
        .position(|&(stellar, _)| Some(stellar) == docked)
    {
        let here = found.remove(index);
        found.push(here);
    }
    found
}

/// The stellars a failed `take` reports: those whose refusals do not
/// include `AvailStel`, or all of them when none is left.
fn kept(looked: Vec<Looked>) -> Vec<Looked> {
    let allowed: Vec<Looked> = looked
        .iter()
        .filter(|looked| {
            !looked
                .refusals
                .iter()
                .any(|&(_, refusal)| refusal == Refusal::AvailStel)
        })
        .cloned()
        .collect();
    if allowed.is_empty() { looked } else { allowed }
}

/// The bits `script` leaves set: each `b` not cleared or toggled by a
/// later operator, in the order first set. An `R(...)`'s arms are left
/// out, as is a script that did not parse.
#[must_use]
pub fn expected_bits(script: &Script) -> Vec<Bit> {
    let mut set = Vec::new();
    for op in script
        .tree()
        .map(|tree| tree.ops.as_slice())
        .unwrap_or_default()
    {
        match *op {
            SetOp::Set(bit) => {
                if !set.contains(&bit) {
                    set.push(bit);
                }
            }
            SetOp::Clear(bit) | SetOp::Toggle(bit) => set.retain(|&held| held != bit),
            _ => {}
        }
    }
    set
}

/// How much a mission's pay of `pay` raises the cash at its completion:
/// a positive pay, itself; a cleaning pay, nothing, as a pay of nothing;
/// `None` for a fine, which takes a share of the cash.
#[must_use]
pub fn expected_pay(pay: i64) -> Option<i64> {
    match pay_effect(pay) {
        PayEffect::Cash(credits) => Some(credits),
        PayEffect::Nothing | PayEffect::Clean { .. } => Some(0),
        PayEffect::Fine(_) => None,
    }
}

#[cfg(test)]
mod tests;
