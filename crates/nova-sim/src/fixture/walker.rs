//! A storyline walker: drives a [`Session`] through a mission chain from a
//! script of [`Step`]s, through the session's public API alone, so a test
//! can play a storyline without flying it.
//!
//! - **`take <mission>`** ([`Step::Take`]) accepts the mission where it is
//!   offered. Offered at the stellar docked at, it is accepted there.
//!   Otherwise the walker lands ([`Session::land_at`]) at every visible
//!   stellar the catalog lists, each at its first visible lister, by
//!   ascending ID, the stellar docked at last, so that landing elsewhere
//!   first rolls its offers afresh; it stops at the first that offers the
//!   mission, and a stellar it cannot land at is skipped. A hidden
//!   stellar is never landed at: its visible variant is scanned in its
//!   own turn. It reads why each stellar does
//!   not offer it ([`Session::offer_refusals`]), never deriving `AvailStel`
//!   itself.
//! - **`deliver <mission>`** ([`Step::Deliver`]) lands at the active
//!   mission's destination, when it is not reached and is not its return
//!   (where a `PickupMode` 1 cargo is picked up), then at its return,
//!   each where a player can land: a hidden stop at its visible variant
//!   of the same place and name, as `_StellarsAreIdentical` matches
//!   them ([`Fault::StopHidden`] when there is none), and
//!   checks that the mission completed, unfailed, paid what its `PayVal`
//!   pays ([`expected_pay`]) and set the bits its `OnSuccess` sets
//!   ([`expected_bits`]). The pay of another mission completed at the
//!   return is taken off the cash's rise first; when one of them is a
//!   fine, the pay is not checked.
//! - **`meet goal <mission>`** ([`Step::MeetGoal`]) leaves the active
//!   mission's special ships as meeting its ship goal in flight would,
//!   then checks its goals as the flight does, so its `ShipDoneText`,
//!   `OnShipDone` and auto-abort run (the session's `meet_ship_goal`):
//!   destroyed, disabled, boarded, seen or chased off. For an escort
//!   goal it first lands at the mission's destination, when it is not
//!   reached and is not its return, as `deliver` does, and brings the
//!   escorts in there. It does not land at the return: a landing there
//!   meets an escort goal with none of its ships destroyed and completes
//!   the mission, which is the `deliver` after it.
//! - **`pass <n> days`** ([`Step::PassDays`]) takes off, when docked, then
//!   lets `n` days go by ([`Session::let_days_pass`]): the take-off passes
//!   its own days by the rules, on top of `n`. The walker stays in flight,
//!   and its next landing is a walker's landing. A mission whose deadline
//!   lapses fails, in flight, as the missions' deadline check says.
//! - **`wait for cron <cron>`** ([`Step::WaitForCron`]) passes days until
//!   the crön has started: the pilot holds it past its pre-holdoff
//!   ([`CronState::started`](crate::cron::CronState::started), which
//!   reads either rule for a crön's end). One already started passes
//!   nothing. Otherwise the walker takes off, when docked, then lets
//!   single days go by, settling each, and checks after the take-off and
//!   after each day. It gives up after [`CRON_WAIT_DAYS`] single days
//!   ([`StoryWalker::with_cron_bound`]), reporting each check of the
//!   crön's eligibility on the last day ([`Session::cron_refusals`]),
//!   `Random` as drawing. A crön the data lacks, or one whose `Duration`
//!   disables it, fails at once, as no day can start it. The walker stays
//!   in flight, as after `pass`. A `Duration` 0 crön with no
//!   `PostHoldoff` starts and ends inside one day's pass and leaves no
//!   state, so the walker cannot see that it started, and its report
//!   says no check refuses; no session event marks a crön's start. A
//!   storyline that waits on such a crön waits on what its `OnStart`
//!   sets, through a mission's `take`.
//! - **`has rank <rank>`** ([`Step::HasRank`]) checks that the pilot holds
//!   the rank. It passes no days, lands nowhere and draws nothing. Ranks
//!   have no availability conditions of their own (scripts grant them),
//!   so a failure names only the rank.
//!
//! A landing runs every held mission's `Landed` hooks, and an accept its
//! `OnAccept`, so a step can start, reach, complete, fail or abort a
//! mission other than its own. The walker compares the missions held
//! either side of each landing and accept, and logs what became of each
//! other mission ([`Became`]) on the step's [`Done`]: side completions
//! are part of a storyline, not a fault.
//!
//! Every action is settled ([`Session::settle_script`]) before the next,
//! so the `Landed`, `DayPassed` and `TookOff` subscriptions run. A `Q`
//! run while docked (a finale's `OnSuccess`, `... M472 Q25048`) then
//! leaves at once ([`Session::leave_if_due`]), as nothing is shown over
//! the port to hold it, and its take-off settles in turn: the walker is
//! in flight after the step, and its next landing is a walker's landing.
//! The
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
//! The walker skips flight: a mission's ship goals are met by `meet
//! goal`. A mission whose goals need ships, delivered without a `meet
//! goal`, is reported [`Fault::NotCompleted`].

use std::fmt;

use crate::catalog::{
    CronId, ListedStellar, MissionId, MissionRecord, PilotCatalog, RankId, StellarId, SystemId,
    TrafficCatalog,
};
use crate::chance::{Chance, NeverFires};
use crate::control::{Bit, Script, SetOp};
use crate::cron::CronRefusal;
use crate::date::GameDate;
use crate::mission::{
    AcceptError, ActiveMission, PayEffect, Refusal, ShipGoal, Stop, Venue, pay_effect,
};
use crate::rules::Verdict;
use crate::session::{RelocateRefusal, Session, StopRefusal};

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
    /// Meet the active mission's ship goal.
    MeetGoal(MissionId),
    /// Take off, when docked, then pass days one at a time until the
    /// crön has started.
    WaitForCron(CronId),
    /// Check that the pilot holds the rank.
    HasRank(RankId),
}

impl fmt::Display for Step {
    /// "take 504", "deliver 504", "meet goal 504", "pass 3 days", "pass
    /// 1 day", "wait for cron 156" or "has rank 145".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Take(mission) => write!(f, "take {}", mission.0),
            Self::Deliver(mission) => write!(f, "deliver {}", mission.0),
            Self::MeetGoal(mission) => write!(f, "meet goal {}", mission.0),
            Self::PassDays(1) => write!(f, "pass 1 day"),
            Self::PassDays(days) => write!(f, "pass {days} days"),
            Self::WaitForCron(cron) => write!(f, "wait for cron {}", cron.0),
            Self::HasRank(rank) => write!(f, "has rank {}", rank.0),
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
        /// What became of the other missions at the walker's landings and
        /// the accept, in order.
        others: Vec<(MissionId, Became)>,
    },
    /// The mission completed at its return, landed at `stellar`, its
    /// cash pay `pay`.
    Delivered {
        /// The mission.
        mission: MissionId,
        /// The stellar landed at for its return: the return, or its
        /// visible variant when the return is hidden.
        stellar: StellarId,
        /// How much the cash rose at its return, less the pay of the
        /// other missions completed there.
        pay: i64,
        /// What became of the other missions at its destination and its
        /// return, in order.
        others: Vec<(MissionId, Became)>,
    },
    /// The mission's ship goal was met.
    MetGoal {
        /// The mission.
        mission: MissionId,
        /// What became of the other missions at the landings an escort
        /// goal takes and as the goal was met, in order.
        others: Vec<(MissionId, Became)>,
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
    /// The crön started: `days` single days after the take-off, when
    /// the ship was docked, and its own days, as the rules pass them.
    WaitedForCron {
        /// The crön.
        cron: CronId,
        /// The single days passed after the take-off; 0 when the crön had
        /// already started.
        days: u32,
        /// Whether the ship took off first.
        took_off: bool,
        /// The date after.
        date: GameDate,
        /// The missions held before the step that failed, or are no longer
        /// held, after it, in slot order.
        failed: Vec<MissionId>,
    },
    /// The pilot holds the rank.
    HadRank {
        /// The rank.
        rank: RankId,
    },
}

/// What became of a mission other than a step's own at one of the step's
/// landings or its accept, as the walker reads it from the missions held
/// either side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Became {
    /// It was accepted: held after, not before (an `S`).
    Started,
    /// Its destination was reached, where a `PickupMode` 1 cargo is
    /// picked up.
    Reached,
    /// It was completed: no longer held, unfailed, its return the stellar
    /// landed at.
    Completed,
    /// It failed: still held, failed now; or no longer held, having
    /// failed.
    Failed,
    /// It ended otherwise, unfailed and away from its return (an `A`).
    Aborted,
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
    /// The cash rose by `got`, less the other missions' pay, not by the
    /// `expected` pay.
    WrongPay {
        /// What its `PayVal` pays.
        expected: i64,
        /// How much the cash rose, less the pay of the other missions
        /// completed at its return.
        got: i64,
    },
    /// These bits its `OnSuccess` sets are not set.
    BitsNotSet(Vec<Bit>),
    /// A landing was refused.
    CannotLand(RelocateRefusal),
    /// The stop's stellar is hidden, and no visible stellar is
    /// identical to it (`_StellarsAreIdentical`), so a player cannot
    /// land there.
    StopHidden {
        /// The stop's stellar.
        stellar: StellarId,
    },
    /// It has no return stellar, so no landing completes it.
    Undeliverable,
    /// It has no ship goal `meet goal` meets: no ships, or no goal, or a
    /// goal code it does not read.
    NoShipGoal,
    /// The crön had not started after `waited` single days: each check
    /// of its eligibility on the last day, by name, as
    /// [`Session::cron_refusals`] reports them (`Random` as
    /// [`Verdict::Draws`]).
    CronNotStarted {
        /// The single days passed after the take-off.
        waited: u32,
        /// Each check that refused it or draws, by name, in the checks'
        /// order.
        refusals: Vec<(&'static str, Verdict<CronRefusal>)>,
    },
    /// The data has no such crön.
    NoSuchCron,
    /// The pilot does not hold the rank.
    RankNotHeld,
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
        let subject = match self.at {
            Step::Take(mission) | Step::Deliver(mission) | Step::MeetGoal(mission) => {
                format!("mission {}", mission.0)
            }
            Step::PassDays(_) => "the step".to_owned(),
            Step::WaitForCron(cron) => format!("cron {}", cron.0),
            Step::HasRank(rank) => format!("rank {}", rank.0),
        };
        match &self.fault {
            Fault::NotOffered => write!(f, "{subject} is offered nowhere")?,
            Fault::Refused(error) => write!(f, "{subject} was refused: {error}")?,
            Fault::NotActive => write!(f, "{subject} is not active")?,
            Fault::NotCompleted(active) => {
                write!(f, "{subject} was not completed: {active:?}")?;
            }
            Fault::WrongPay { expected, got } => {
                write!(f, "{subject} paid {got}, not {expected}")?;
            }
            Fault::BitsNotSet(bits) => {
                write!(f, "{subject} left unset:")?;
                for bit in bits {
                    write!(f, " b{}", bit.get())?;
                }
            }
            Fault::CannotLand(refusal) => write!(f, "{subject}: cannot land: {refusal}")?,
            Fault::StopHidden { stellar } => write!(
                f,
                "{subject}: stop stellar {} is hidden, and no visible stellar shares its place \
                 and name",
                stellar.0
            )?,
            Fault::Undeliverable => write!(f, "{subject} has no return stellar")?,
            Fault::NoShipGoal => write!(f, "{subject} has no ship goal to meet")?,
            Fault::CronNotStarted { waited, refusals } => {
                write!(f, "{subject} did not start within {waited} days:")?;
                for (index, (name, verdict)) in refusals.iter().enumerate() {
                    let sep = if index == 0 { "" } else { "," };
                    match verdict {
                        Verdict::Refuses(refusal) => write!(f, "{sep} {name} ({refusal:?})")?,
                        Verdict::Draws => write!(f, "{sep} {name} (draws)")?,
                    }
                }
                if !refusals
                    .iter()
                    .any(|(_, verdict)| matches!(verdict, Verdict::Refuses(_)))
                {
                    let sep = if refusals.is_empty() { "" } else { "," };
                    write!(f, "{sep} no check refuses")?;
                }
            }
            Fault::NoSuchCron => write!(f, "{subject} is not in the data")?,
            Fault::RankNotHeld => write!(f, "{subject} is not held")?,
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
    cron_bound: u32,
}

/// How many single days `wait for cron` passes, by default, before it
/// gives up: more than a year, so a yearly date window such as crön
/// 156's 1 Sep to 30 Dec opens from any day.
pub const CRON_WAIT_DAYS: u32 = 400;

impl<'c, C: PilotCatalog + TrafficCatalog + ?Sized> StoryWalker<'c, C> {
    /// A walker over `catalog`, its days drawing on [`NeverFires`].
    #[must_use]
    pub fn new(catalog: &'c C) -> Self {
        Self {
            catalog,
            day_chance: Box::new(NeverFires),
            cron_bound: CRON_WAIT_DAYS,
        }
    }

    /// This walker, its `wait for cron` giving up after `days` single
    /// days rather than [`CRON_WAIT_DAYS`].
    #[must_use]
    pub fn with_cron_bound(self, days: u32) -> Self {
        Self {
            cron_bound: days,
            ..self
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
                Step::MeetGoal(mission) => self.meet_goal(session, mission),
                Step::WaitForCron(cron) => self.wait_for_cron(session, cron),
                Step::HasRank(rank) => has_rank(session, rank),
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

    /// Settles what the last action left, every roll forced, then
    /// carries out a `Q`'s leave it left due ([`Session::leave_if_due`]),
    /// on the day chance: nothing shown over the port holds it here.
    fn settle(&mut self, session: &mut Session) {
        let _ = session.settle_script(self.catalog, &mut NeverFires);
        let _ = session.leave_if_due(self.catalog, &mut *self.day_chance);
    }

    /// `take <mission>` (see the module docs).
    fn take(&mut self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
        let Some(record) = self.record(mission) else {
            return Err((Fault::NotOffered, Vec::new()));
        };
        let venue = Venue::of(record.avail_loc);
        if let Some(here) = session.landed()
            && offered(session, mission, venue)
        {
            return self.accept(session, mission, here, Vec::new());
        }
        let mut looked = Vec::new();
        let mut others = Vec::new();
        for (stellar, system) in candidates(&self.catalog.listed_stellars(), session.landed()) {
            let stop = Stop {
                stellar,
                system: Some(system),
            };
            // A hidden stellar is never landed at; its visible variant is
            // scanned in its own turn, so its offers are rolled once.
            let Ok((system, landing)) = session.stop_landing(stop) else {
                continue;
            };
            if landing != stellar {
                continue;
            }
            let stop = Stop {
                stellar,
                system: Some(system),
            };
            let (landed, _) = watched(session, mission, &mut others, |session| {
                self.land(session, stop)
            });
            if landed.is_err() {
                continue;
            }
            if offered(session, mission, venue) {
                return self.accept(session, mission, stellar, others);
            }
            looked.push(Looked {
                stellar,
                system,
                refusals: refusals(session, mission),
            });
        }
        Err((Fault::NotOffered, kept(looked)))
    }

    /// Accepts `mission`, offered at `stellar`, the stellar docked at,
    /// logging on `others` what it did to the other missions.
    fn accept(
        &mut self,
        session: &mut Session,
        mission: MissionId,
        stellar: StellarId,
        mut others: Vec<(MissionId, Became)>,
    ) -> Result<Done, Failed> {
        let (accepted, _) = watched(session, mission, &mut others, |session| {
            let accepted = session.accept_mission(mission, &mut NeverFires);
            if accepted.is_ok() {
                self.settle(session);
            }
            accepted
        });
        let slot = accepted.map_err(|error| (Fault::Refused(error), Vec::new()))?;
        Ok(Done::Took {
            mission,
            stellar,
            slot,
            others,
        })
    }

    /// `deliver <mission>` (see the module docs).
    fn deliver(&mut self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
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
        let mut others = Vec::new();
        if let Some(destination) = active.destination
            && !active.reached
            && destination.stellar != back.stellar
        {
            watched(session, mission, &mut others, |session| {
                self.land(session, destination)
            })
            .0?;
        }
        if let Some(active) = still_held(session, slot, mission)
            && active.failed.is_some()
        {
            return Err((Fault::NotCompleted(Box::new(active)), Vec::new()));
        }
        let cash = session.pilot().cash();
        let (landed, paid) = watched(session, mission, &mut others, |session| {
            self.land(session, back)
        });
        landed?;
        if let Some(active) = still_held(session, slot, mission) {
            return Err((Fault::NotCompleted(Box::new(active)), Vec::new()));
        }
        let got = session.pilot().cash() - cash - paid.unwrap_or_default();
        if let Some(expected) = expected_pay(active.pay)
            && paid.is_some()
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
            stellar: session.landed().unwrap_or(back.stellar),
            pay: got,
            others,
        })
    }

    /// `meet goal <mission>` (see the module docs).
    fn meet_goal(&mut self, session: &mut Session, mission: MissionId) -> Result<Done, Failed> {
        let held = session
            .pilot()
            .missions()
            .iter()
            .find(|(_, active)| active.mission == mission)
            .map(|(slot, active)| (slot, active.clone()));
        let Some((slot, active)) = held else {
            return Err((Fault::NotActive, Vec::new()));
        };
        let mut others = Vec::new();
        let escorts = self
            .record(mission)
            .is_some_and(|record| ShipGoal::from_code(record.ships.goal) == ShipGoal::Escort);
        // Not at the return: a landing there completes an escort mission
        // with none of its ships destroyed (`_MissionLandCheck`'s escort
        // branch, @0xa1a99-0xa1ab1), which is `deliver`'s to do.
        if escorts
            && let Some(back) = active.return_to
            && let Some(destination) = active.destination
            && !active.reached
            && destination.stellar != back.stellar
        {
            watched(session, mission, &mut others, |session| {
                self.land(session, destination)
            })
            .0?;
        }
        let (met, _) = watched(session, mission, &mut others, |session| {
            let met = session.meet_ship_goal(self.catalog, slot, &mut NeverFires);
            self.settle(session);
            met
        });
        if !met {
            return Err((Fault::NoShipGoal, Vec::new()));
        }
        Ok(Done::MetGoal { mission, others })
    }

    /// Lands at `stop` where a player can, as the session resolves it
    /// (`Session::stop_landing`): a visible stellar in its stored system,
    /// when that is visible, else in its first visible lister; a hidden
    /// one at its visible variant of the same place and name. Then
    /// settles. No lister is [`Fault::CannotLand`] for want of a system;
    /// hidden with no visible variant, [`Fault::StopHidden`].
    fn land(&mut self, session: &mut Session, stop: Stop) -> Result<(), Failed> {
        let (system, stellar) = session.stop_landing(stop).map_err(|refusal| {
            let fault = match refusal {
                StopRefusal::NoSystem => Fault::CannotLand(RelocateRefusal::NoSystem),
                StopRefusal::Hidden => Fault::StopHidden {
                    stellar: stop.stellar,
                },
            };
            (fault, Vec::new())
        })?;
        session
            .land_at(self.catalog, system, stellar)
            .map_err(|refusal| (Fault::CannotLand(refusal), Vec::new()))?;
        self.settle(session);
        Ok(())
    }

    /// `pass <days> days` (see the module docs).
    fn pass(&mut self, session: &mut Session, days: u32) -> Done {
        let held = held_missions(session);
        let took_off = self.take_off(session);
        self.let_days_pass(session, days);
        Done::Passed {
            days,
            took_off,
            date: session.date(),
            failed: failed_since(session, held),
        }
    }

    /// `wait for cron <cron>` (see the module docs).
    fn wait_for_cron(&mut self, session: &mut Session, cron: CronId) -> Result<Done, Failed> {
        let (Some(refusals), Some(record)) = (
            session.cron_refusals(cron),
            self.catalog
                .crons()
                .into_iter()
                .find(|record| record.id == cron),
        ) else {
            return Err((Fault::NoSuchCron, Vec::new()));
        };
        let started = |session: &Session| {
            session
                .pilot()
                .crons()
                .get(&cron)
                .is_some_and(|state| state.started(&record))
        };
        let disabled = refusals.contains(&("Duration", Verdict::Refuses(CronRefusal::Disabled)));
        let held = held_missions(session);
        let mut took_off = false;
        let mut days = 0;
        if !started(session) {
            if disabled {
                return Err((
                    Fault::CronNotStarted {
                        waited: 0,
                        refusals,
                    },
                    Vec::new(),
                ));
            }
            took_off = self.take_off(session);
            self.settle_days(session);
            while !started(session) {
                if days == self.cron_bound {
                    let refusals = session.cron_refusals(cron).unwrap_or_default();
                    return Err((
                        Fault::CronNotStarted {
                            waited: days,
                            refusals,
                        },
                        Vec::new(),
                    ));
                }
                self.let_days_pass(session, 1);
                days += 1;
            }
        }
        Ok(Done::WaitedForCron {
            cron,
            days,
            took_off,
            date: session.date(),
            failed: failed_since(session, held),
        })
    }

    /// Takes off, when docked, on the day chance, and settles the
    /// take-off ([`Session::leave_port`]): whether it took off.
    fn take_off(&mut self, session: &mut Session) -> bool {
        session
            .leave_port(self.catalog, &mut *self.day_chance)
            .is_some()
    }

    /// Lets `days` days go by on the day chance, and settles.
    fn let_days_pass(&mut self, session: &mut Session, days: u32) {
        session.let_days_pass(days, &mut *self.day_chance);
        self.settle_days(session);
    }

    /// Settles what the days left, on the day chance.
    fn settle_days(&mut self, session: &mut Session) {
        let _ = session.settle_script(self.catalog, &mut *self.day_chance);
    }
}

/// The missions held, by slot.
fn held_missions(session: &Session) -> Vec<(u8, MissionId)> {
    session
        .pilot()
        .missions()
        .iter()
        .map(|(slot, active)| (slot, active.mission))
        .collect()
}

/// The missions of `held` that failed, or are no longer held, in slot
/// order.
fn failed_since(session: &Session, held: Vec<(u8, MissionId)>) -> Vec<MissionId> {
    held.into_iter()
        .filter(|&(slot, mission)| {
            still_held(session, slot, mission).is_none_or(|active| active.failed.is_some())
        })
        .map(|(_, mission)| mission)
        .collect()
}

/// `has rank <rank>` (see the module docs).
fn has_rank(session: &Session, rank: RankId) -> Result<Done, Failed> {
    if session.pilot().ranks().contains(&rank) {
        Ok(Done::HadRank { rank })
    } else {
        Err((Fault::RankNotHeld, Vec::new()))
    }
}

/// Does `act` to `session` (a landing or an accept, and its settling), and
/// logs on `others` what became of each mission but `own` ([`changes`]):
/// what `act` returned, and the pay of the missions it completed, as
/// [`expected_pay`] reads them, `None` when one is a fine.
fn watched<T>(
    session: &mut Session,
    own: MissionId,
    others: &mut Vec<(MissionId, Became)>,
    act: impl FnOnce(&mut Session) -> T,
) -> (T, Option<i64>) {
    let before = holding(session);
    let done = act(session);
    let after = holding(session);
    let changed = changes(&before, &after, own, |stop| {
        session.docked_at_mission_stop(stop)
    });
    let paid = changed
        .iter()
        .filter(|&&(_, became)| became == Became::Completed)
        .map(|(active, _)| expected_pay(active.pay))
        .sum();
    others.extend(
        changed
            .into_iter()
            .map(|(active, became)| (active.mission, became)),
    );
    (done, paid)
}

/// The missions held, by slot.
fn holding(session: &Session) -> Vec<(u8, ActiveMission)> {
    session
        .pilot()
        .missions()
        .iter()
        .map(|(slot, active)| (slot, active.clone()))
        .collect()
}

/// What became of each mission but `own` between `before` and `after`,
/// the missions held, by slot, either side of an action that left the
/// ship docked where `returned` says a mission's return is reached,
/// with its state before (after, for one started); those held before
/// first, in slot order, then those started. A mission no longer held
/// was completed when it had not failed and `returned` says its return
/// is docked at: an `A` there reads as a completion.
fn changes<'a>(
    before: &'a [(u8, ActiveMission)],
    after: &'a [(u8, ActiveMission)],
    own: MissionId,
    returned: impl Fn(Stop) -> bool,
) -> Vec<(&'a ActiveMission, Became)> {
    let find = |held: &'a [(u8, ActiveMission)], slot: u8, mission: MissionId| {
        held.iter()
            .find(|(held, active)| *held == slot && active.mission == mission)
            .map(|(_, active)| active)
    };
    let mut changes = Vec::new();
    for (slot, was) in before {
        if was.mission == own {
            continue;
        }
        let returned = was.return_to.is_some_and(&returned);
        let became = match find(after, *slot, was.mission) {
            None if was.failed.is_some() => Became::Failed,
            None if returned => Became::Completed,
            None => Became::Aborted,
            Some(now) if was.failed.is_none() && now.failed.is_some() => Became::Failed,
            Some(now) if !was.reached && now.reached => Became::Reached,
            Some(_) => continue,
        };
        changes.push((was, became));
    }
    for (slot, now) in after {
        if now.mission != own && find(before, *slot, now.mission).is_none() {
            changes.push((now, Became::Started));
        }
    }
    changes
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
