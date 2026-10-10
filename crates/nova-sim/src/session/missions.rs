//! The session's side of missions (see the [`mission`](crate::mission)
//! module): the visit's offers, rolled as the player lands and kept for
//! the visit, and the facts they are rolled from; and the active
//! missions' lifecycle.
//!
//! The roll is no loop of its own: [`Session::fly`] registers it as the
//! missions' first `Landed` subscription, [`ROLL`], whose consumer effect
//! rolls the offers at the stellar landed on (`_HandlePlayerDockRequest`
//! @0x66d2b-0x66d77), before any mission slot's landing hooks (see the
//! [`story`](crate::story) module's "Dispatch"). A pilot flown docked has
//! `Landed` raised for its stellar, after its `Explored`, as a load
//! forgets the stellar rolled at (@0x75ef5) and opens the spaceport, so
//! the first settling rolls its offers afresh.
//!
//! What forgets the stellar rolled at, so that the next landing rolls the
//! offers afresh, is [`Session::forget_offers`]: a jump's arrival
//! (@0x6bfce) and the developer tools' relocation through the entry
//! table, and a mission's abort, failure and success (@0x99bcf, @0x99db5,
//! @0xa03ef, @0xa0691). A landed `M` or `N` empties the offers and keeps
//! the stellar (@0x1584b, @0x15949). None of it is saved.
//!
//! **Active missions.** The missions accepted are the pilot's, in their
//! slots (see the [`mission`](crate::mission) module's "Active
//! missions"). [`Session::accept_mission`], [`Session::refuse_mission`]
//! and [`Session::abort_mission`] are the player's; `S`, `A` and `F` are
//! `apply_op`'s arms. Each slot filled registers its hooks, `{Missions,
//! slot, hook}`: [`ARRIVAL`], [`FAILURE`] and [`SUCCESS`] on every
//! landing, and, for a mission with a deadline, [`DEADLINE`] on every day
//! and [`DEADLINE_TOOK_OFF`] on every take-off, each with a blank guard
//! and no script; their effects come through the dispatcher's consumer
//! match. [`REOFFER`], registered with [`ROLL`], re-offers after the last
//! slot's hooks when a mission succeeded. [`Session::fly`] registers
//! every active slot's hooks again, so a saved pilot's missions go on.
//! Each end frees its slot and unregisters its hooks.
//!
//! **Text.** A mission's texts are filled through the one substitution
//! path ([`text_tags`](crate::text_tags)): [`Session::offer_text`] for an
//! offer on the visit under way, [`Session::mission_text`] for a slot,
//! and [`Session::plain_text`] with no mission. Their names are the
//! stellars' and systems' the session read as it started
//! ([`PilotCatalog::place_names`](crate::PilotCatalog::place_names)), and
//! the cargo names `STR#` 4001 of its string lists
//! ([`Session::with_strings`]). Each mission script runs with its slot as
//! the script's mission, so a `Q` in it fills its mission tags (see the
//! `script_effects` module).

use super::Session;
use crate::catalog::{GovtId, MissionId, MissionRecord, MissionShip, StellarId, SystemId};
use crate::chance::Chance;
use crate::control::{Script, Test};
use crate::mission::active::{AUTO_ABORT_FUEL, AUTO_ABORT_FUEL_TAKEN, AUTO_ABORT_PAYS};
use crate::mission::{
    self, AbortFacts, AbortRefusal, AcceptFacts, AcceptRefusal, ActiveMission, AvailFacts,
    FailedBy, Offer, PayEffect, Player, Refusal, SLOTS, Stop, Venue, Visit, World, abort_checks,
    abort_penalty, accept_charge, accept_checks, auto_aborts_at_accept, failure_records, fined,
    pay_effect, stellars_of, success_records,
};
use crate::rulebook::{RuleKey, RuleSource};
use crate::rules::Verdict;
use crate::story::{
    Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::text_tags::{self, MissionTags, MissionValues, TagContext, TagNames};
use crate::wares;

/// The offer roll's subscription: the missions' first, before every
/// mission slot's (slots 0-15) landing hooks.
pub const ROLL: SubscriptionId = SubscriptionId {
    consumer: Consumer::Missions,
    record: -1,
    hook: 0,
};

/// The re-offer's subscription: the missions' last, after every slot's
/// landing hooks, as the original re-offers once after its last slot
/// (@0xa1b04).
pub const REOFFER: SubscriptionId = SubscriptionId {
    consumer: Consumer::Missions,
    record: SLOTS as i16,
    hook: 0,
};

/// A slot's landing hook that marks its destination reached: the cargo
/// check's place in `_MissionLandCheck` (@0xa1a1b), which missions phase
/// 5 fills.
pub const ARRIVAL: u8 = 0;
/// A slot's landing hook that fails a failed mission at its return
/// (@0xa1a92).
pub const FAILURE: u8 = 1;
/// A slot's landing hook that completes a mission at its return
/// (@0xa1ace).
pub const SUCCESS: u8 = 2;
/// A timed slot's daily deadline check, on `DayPassed`.
pub const DEADLINE: u8 = 3;
/// A timed slot's deadline check on `TookOff`, for a deadline that ran
/// out while docked with no day passing as the ship leaves.
pub const DEADLINE_TOOK_OFF: u8 = 4;

/// Every hook a slot can register, in order.
const HOOKS: [u8; 5] = [ARRIVAL, FAILURE, SUCCESS, DEADLINE, DEADLINE_TOOK_OFF];

/// The cargo names `<CT>` reads, lower case (`_lcCargoName`, from
/// `_LoadStrings` @0x71ba4).
const CARGO_NAMES: i16 = 4001;

/// Slot `slot`'s subscription for `hook`.
#[must_use]
pub const fn slot_hook(slot: u8, hook: u8) -> SubscriptionId {
    SubscriptionId {
        consumer: Consumer::Missions,
        record: slot as i16,
        hook,
    }
}

/// The mission is not offered on the visit under way: there is no such
/// offer, or the ship is not docked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the mission is not offered here")]
pub struct NotOffered;

/// Why [`Session::accept_mission`] accepted nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AcceptError {
    /// It is not offered here.
    #[error("the mission is not offered here")]
    NotOffered,
    /// An accept check refused it.
    #[error("the mission cannot be accepted: {0}")]
    Refused(AcceptRefusal),
}

/// Why [`Session::abort_mission`] aborted nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AbortError {
    /// The slot holds no mission.
    #[error("no mission is in that slot")]
    NotActive,
    /// An abort check refused it: its `CanAbort` is 0.
    #[error("the mission cannot be aborted")]
    Refused(AbortRefusal),
}

/// The record a mission's lifecycle reads for `id` when the session has
/// none (a mission saved whose `mïsn` is gone): no script, abortable,
/// with no reward, no days and no ships. Only those fields, and `Flags`
/// and `Flags2`, are read of an active mission's record; the rest are
/// none.
fn blank(id: MissionId) -> MissionRecord {
    MissionRecord {
        id,
        avail_stel: 0,
        avail_loc: 0,
        avail_record: 0,
        avail_rating: 0,
        avail_random: 0,
        avail_ship_type: 0,
        avail_bits: Test::default(),
        require: 0,
        flags: 0,
        flags2: 0,
        travel_stel: 0,
        return_stel: 0,
        cargo_type: 0,
        cargo_qty: 0,
        pay_val: 0,
        time_limit: 0,
        disp_weight: 0,
        on_accept: Script::default(),
        on_refuse: Script::default(),
        on_success: Script::default(),
        on_failure: Script::default(),
        on_abort: Script::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip { count: 0, goal: 0 },
    }
}

/// The days a `DatePostInc` of `days` passes: none for 0 or below.
fn post_days(days: i16) -> u32 {
    u32::try_from(days).unwrap_or(0)
}

/// Whether `stop` is `stellar`: by ID, where the original also counts a
/// stellar at the same place with the same name (`_StellarsAreIdentical`
/// @0xab35; rdm:phase/missions/phase-11-stellars-identical-by-place-and-name).
fn at(stop: Option<Stop>, stellar: StellarId) -> bool {
    stop.is_some_and(|stop| stop.stellar == stellar)
}

impl Session {
    /// Registers the offer roll ([`ROLL`]) and the re-offer
    /// ([`REOFFER`]) on every landing, each with a blank guard and no
    /// script.
    pub(super) fn register_offer_roll(&mut self) {
        for id in [ROLL, REOFFER] {
            let landed = Subscription::new(
                id,
                EventFilter::Any(StoryEventKind::Landed),
                Script::default(),
            );
            let registered = self.register_story(landed);
            debug_assert!(registered.is_ok(), "a fresh session registers it once");
        }
    }

    /// Registers every active slot's hooks, as a pilot is flown.
    pub(super) fn register_slots(&mut self) {
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            self.register_slot(slot);
        }
    }

    /// Registers `slot`'s hooks: its landing hooks, and, with a deadline,
    /// its deadline checks, each with a blank guard and no script.
    fn register_slot(&mut self, slot: u8) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let mut hooks = vec![
            (ARRIVAL, StoryEventKind::Landed),
            (FAILURE, StoryEventKind::Landed),
            (SUCCESS, StoryEventKind::Landed),
        ];
        if active.days_left.is_some() {
            hooks.push((DEADLINE, StoryEventKind::DayPassed));
            hooks.push((DEADLINE_TOOK_OFF, StoryEventKind::TookOff));
        }
        for (hook, kind) in hooks {
            let subscription = Subscription::new(
                slot_hook(slot, hook),
                EventFilter::Any(kind),
                Script::default(),
            );
            let registered = self.register_story(subscription);
            debug_assert!(registered.is_ok(), "a slot filled registers once");
        }
    }

    /// Frees `slot` and unregisters its hooks, clearing their once-fired
    /// entries, so a mission accepted into it starts fresh: the mission it
    /// held, if any.
    fn free_slot(&mut self, slot: u8) -> Option<ActiveMission> {
        let freed = self.pilot.missions.free(slot)?;
        for hook in HOOKS {
            self.unregister_story(slot_hook(slot, hook));
        }
        self.save_due = true;
        Some(freed)
    }

    /// The record of `mïsn` `id`, if the session has one.
    pub(super) fn mission_record(&self, id: MissionId) -> Option<&MissionRecord> {
        self.missions.iter().find(|record| record.id == id)
    }

    /// The record an active mission's lifecycle reads: its own, or a
    /// blank one when its `mïsn` is gone.
    fn lifecycle_record(&self, id: MissionId) -> MissionRecord {
        self.mission_record(id)
            .cloned()
            .unwrap_or_else(|| blank(id))
    }

    /// The offer of `mission` on the visit under way: none in flight.
    fn offer_of(&self, mission: MissionId) -> Option<Offer> {
        self.landed?;
        Venue::ORDER.into_iter().find_map(|venue| {
            self.offers
                .at(venue)
                .iter()
                .find(|offer| offer.mission == mission)
                .cloned()
        })
    }

    /// The first refusal of the accept checks, drawing on `chance`.
    pub(super) fn accept_refusal(&self, chance: &mut dyn Chance) -> Result<(), AcceptRefusal> {
        let facts = AcceptFacts {
            active: &self.pilot.missions,
            rules: &self.rules,
        };
        accept_checks().first_refusal(&facts, chance)
    }

    /// Accepts `mission`, offered on the visit under way, drawing on
    /// `chance`: the accept checks, then the original's accept
    /// (`_DoMissionAccept` @0xa1b11; see the [`mission`]
    /// module's "Active missions"). The slot it went into.
    ///
    /// # Errors
    ///
    /// [`AcceptError::NotOffered`] in flight or for a mission not offered;
    /// [`AcceptError::Refused`] when a check refuses, and then nothing
    /// runs.
    pub fn accept_mission(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Result<u8, AcceptError> {
        let offer = self.offer_of(mission).ok_or(AcceptError::NotOffered)?;
        let record = self
            .mission_record(mission)
            .cloned()
            .ok_or(AcceptError::NotOffered)?;
        self.accept_refusal(chance).map_err(AcceptError::Refused)?;
        self.start_mission(&record, &offer, chance)
            .ok_or(AcceptError::Refused(AcceptRefusal::SlotsFull))
    }

    /// Refuses `mission`, offered on the visit under way, drawing on
    /// `chance`: its `OnRefuse` runs (`_OfferOneMission` @0xa2f41), then
    /// its offer is dropped (`_OfferMissionFromPort` @0xa3765-0xa37a1).
    /// Nothing is forgotten.
    ///
    /// # Errors
    ///
    /// [`NotOffered`] in flight or for a mission not offered.
    pub fn refuse_mission(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Result<(), NotOffered> {
        self.offer_of(mission).ok_or(NotOffered)?;
        let record = self.lifecycle_record(mission);
        // `RefuseText`, with no mission (index -1, @0xa2ec9): missions
        // phase 4, through `Session::plain_text`.
        self.run_script(&record.on_refuse, chance);
        self.offers.drop_offer(mission);
        Ok(())
    }

    /// The player's abort of the mission in `slot`, drawing on `chance`
    /// (`_DoMissionInfoDialog` @0x9e56d-0x9e5e6): the abort checks, then
    /// with `Flags` 0x0040 the record with `CompGovt` less 5 ×
    /// `CompReward`, then the mission ends, running its `OnAbort`.
    ///
    /// # Errors
    ///
    /// [`AbortError::NotActive`] for a free slot; [`AbortError::Refused`]
    /// when a check refuses, and then nothing runs.
    pub fn abort_mission(&mut self, slot: u8, chance: &mut dyn Chance) -> Result<(), AbortError> {
        let mission = self
            .pilot
            .missions
            .get(slot)
            .ok_or(AbortError::NotActive)?
            .mission;
        let record = self.lifecycle_record(mission);
        let facts = AbortFacts {
            mission: &record,
            rules: &self.rules,
        };
        abort_checks()
            .first_refusal(&facts, chance)
            .map_err(AbortError::Refused)?;
        let penalty = abort_penalty(&record, |govt| self.pilot.legal_record(govt));
        self.set_records(&penalty);
        self.end_mission(slot, &record, true, chance);
        Ok(())
    }

    /// Accepts `record`'s `offer` into the lowest free slot, as the
    /// original's accept does once its checks pass (`_DoMissionAccept`
    /// @0xa1c0c-0xa2196): the slot filled and its hooks registered;
    /// `OnAccept`; the cash a pay below -50000 takes; the destination
    /// reached when it has none, or, docked, when it is the stellar docked
    /// at; docked, the mission's offer dropped and the others kept only
    /// while available; the auto-abort; and its roll made 0. The slot, or
    /// `None` when every slot is taken.
    fn start_mission(
        &mut self,
        record: &MissionRecord,
        offer: &Offer,
        chance: &mut dyn Chance,
    ) -> Option<u8> {
        let active = ActiveMission::accepted(record, offer, self.pilot.date);
        let slot = self.pilot.missions.fill(active)?;
        self.register_slot(slot);
        self.save_due = true;
        // `BriefText` and `LoadCargText`, filled for the slot (@0xa1e56,
        // @0xa1f80): missions phase 4, through `Session::mission_text`;
        // the cargo loaded: missions phase 5.
        self.run_mission_script(slot, &record.on_accept, chance);
        self.pilot.cash = accept_charge(offer.pay, self.pilot.cash);
        let docked = self.landed;
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.reached = active
                .destination
                .is_none_or(|stop| Some(stop.stellar) == docked);
        }
        if docked.is_some() {
            self.offers.drop_offer(record.id);
            self.refilter_offers();
        }
        let aborts = self
            .pilot
            .missions
            .get(slot)
            .filter(|active| active.mission == record.id)
            .is_some_and(|active| auto_aborts_at_accept(record, active, docked.is_some()));
        if aborts {
            self.auto_abort(slot, record, chance);
        }
        self.offers.set_roll(record.id, 0);
        Some(slot)
    }

    /// Runs `script`, the script of the mission in `slot`, drawing on
    /// `chance`, as `_EvalCurrentMissionBitSetString` (@0x99a43) runs
    /// one: with `slot` the script's mission
    /// (`_currentMissionIndexForSetParser`, 0x31d6c2), so a `Q` fills its
    /// mission tags from it, then none, not the one before (@0x99a79), so
    /// after a nested mission's script the outer script has none.
    fn run_mission_script(&mut self, slot: u8, script: &Script, chance: &mut dyn Chance) {
        self.script_slot = Some(slot);
        self.run_script(script, chance);
        self.script_slot = None;
    }

    /// The mission in `slot`, to change, if it is `mission`.
    fn slot_holding(&mut self, slot: u8, mission: MissionId) -> Option<&mut ActiveMission> {
        self.save_due = true;
        self.pilot
            .missions
            .get_mut(slot)
            .filter(|active| active.mission == mission)
    }

    /// Keeps only the offers still available at the stellar docked at
    /// ([`Offers::refilter`](mission::Offers::refilter)).
    fn refilter_offers(&mut self) {
        let Some(stellar) = self.landed else {
            return;
        };
        let mut offers = std::mem::take(&mut self.offers);
        self.at_visit(stellar, |visit| offers.refilter(&self.missions, visit));
        self.offers = offers;
    }

    /// Ends the mission `record` in `slot` as `_AbortMission` (@0x99a92)
    /// does, drawing on `chance`: its ships leave (mission-ships phase 2);
    /// its `OnAbort` runs when `on_abort` says; the slot is freed and its
    /// hooks unregistered; and the offers' stellar is forgotten. Nothing
    /// when the slot holds another mission, or is ending already, so an
    /// `A` of the mission inside its own `OnAbort` does not end it twice.
    fn end_mission(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        on_abort: bool,
        chance: &mut dyn Chance,
    ) {
        if !self.pilot.missions.holds(slot, record.id) || !self.ending.insert(slot) {
            return;
        }
        if on_abort {
            self.run_mission_script(slot, &record.on_abort, chance);
        }
        self.ending.remove(&slot);
        self.close_slot(slot, record.id);
    }

    /// Frees `slot` if it still holds `mission`, and forgets the offers'
    /// stellar (@0x99bb4-0x99bcf).
    fn close_slot(&mut self, slot: u8, mission: MissionId) {
        if self.pilot.missions.holds(slot, mission) {
            self.free_slot(slot);
        }
        self.forget_offers();
    }

    /// Aborts the mission `record` in `slot` at once, as an accept that
    /// leaves it nothing to do does (`_AutoAbortMission` @0x99bdc),
    /// drawing on `chance`: its `OnAbort`; its `DatePostInc` days through
    /// the daily step; with `Flags` 0x0008, a jump's fuel taken; with
    /// `Flags2` 0x0002, its pay; then the slot freed and the offers'
    /// stellar forgotten, with no second `OnAbort`.
    fn auto_abort(&mut self, slot: u8, record: &MissionRecord, chance: &mut dyn Chance) {
        let Some(pay) = self.pilot.missions.get(slot).map(|active| active.pay) else {
            return;
        };
        if !self.ending.insert(slot) {
            return;
        }
        self.run_mission_script(slot, &record.on_abort, chance);
        self.pass_days(post_days(record.date_post_inc), chance);
        if record.flags & AUTO_ABORT_FUEL != 0 {
            self.pilot.reserves.fuel.now -= AUTO_ABORT_FUEL_TAKEN;
        }
        if record.flags2 & AUTO_ABORT_PAYS != 0 {
            self.apply_pay(pay);
        }
        self.ending.remove(&slot);
        self.close_slot(slot, record.id);
    }

    /// Applies a mission's pay (`_ApplyMissionPay` @0x98326; see
    /// [`pay_effect`]).
    fn apply_pay(&mut self, pay: i64) {
        match pay_effect(pay) {
            PayEffect::Nothing => return,
            PayEffect::Cash(credits) => self.pilot.cash = self.pilot.cash.saturating_add(credits),
            PayEffect::Clean { of, among } => {
                let cleaned: Vec<GovtId> = self
                    .pilot
                    .legal_records()
                    .filter(|&(govt, record)| {
                        record.is_negative() && among.takes(govt, of, &self.govts)
                    })
                    .map(|(govt, _)| govt)
                    .collect();
                for govt in cleaned {
                    self.pilot.set_legal_record(govt, 0);
                }
            }
            PayEffect::Fine(percent) => self.pilot.cash = fined(self.pilot.cash, percent),
        }
        self.save_due = true;
    }

    /// Sets each government's record as `records` says.
    fn set_records(&mut self, records: &[(GovtId, i16)]) {
        for &(govt, record) in records {
            self.pilot.set_legal_record(govt, record);
            self.save_due = true;
        }
    }

    /// A slot's own effect on `event`, routed by its hook (see the
    /// [`story`](crate::story) module's "Mission lifecycle"), drawing on
    /// `chance`.
    pub(super) fn mission_effect(
        &mut self,
        id: SubscriptionId,
        event: &StoryEvent,
        chance: &mut dyn Chance,
    ) {
        let Ok(slot) = u8::try_from(id.record) else {
            return;
        };
        match (id.hook, *event) {
            (ARRIVAL, StoryEvent::Landed { stellar }) => self.mission_arrival(slot, stellar),
            (FAILURE, StoryEvent::Landed { stellar }) => {
                self.mission_failure_at(slot, stellar, chance);
            }
            (SUCCESS, StoryEvent::Landed { stellar }) => {
                self.mission_success_at(slot, stellar, chance);
            }
            (DEADLINE, StoryEvent::DayPassed { .. })
            | (DEADLINE_TOOK_OFF, StoryEvent::TookOff { .. }) => {
                self.mission_deadline(slot, chance);
            }
            _ => {}
        }
    }

    /// Landing on `stellar` marks the destination of the mission in
    /// `slot` reached (`_MissionLandCargoCheck` @0x9edfd-0x9ef20; with
    /// `PickupMode` 1, missions phase 5 marks it only once the pickup
    /// succeeds).
    fn mission_arrival(&mut self, slot: u8, stellar: StellarId) {
        if let Some(active) = self.pilot.missions.get_mut(slot)
            && at(active.destination, stellar)
            && !active.reached
        {
            active.reached = true;
            self.save_due = true;
        }
    }

    /// Landing on `stellar`, the return of the failed mission in `slot`,
    /// fails it (`_DoMissionFailure` @0xa0285), drawing on `chance`: its
    /// `OnFailure`, the record with `CompGovt` less half `CompReward`,
    /// then the slot freed and the offers' stellar forgotten, no
    /// `OnAbort` run. A mission failed by its deadline ran `OnFailure`
    /// already: by [`RuleKey::RepeatMissionFailure`]'s engine reading it
    /// runs again; by the other, not.
    fn mission_failure_at(&mut self, slot: u8, stellar: StellarId, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let Some(failed) = active.failed.filter(|_| at(active.return_to, stellar)) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let repeats = self.rules.source_for(RuleKey::RepeatMissionFailure) == RuleSource::Engine;
        if failed == FailedBy::Operator || repeats {
            self.run_mission_script(slot, &record.on_failure, chance);
        }
        let changes = failure_records(&record, |govt| self.pilot.legal_record(govt));
        self.set_records(&changes);
        // `FailText`: missions phase 4, through `Session::mission_text`.
        // The original fills it after the slot is freed (@0xa0398), so its
        // mission tags read "[Error]".
        self.end_mission(slot, &record, false, chance);
        self.forget_offers();
    }

    /// Landing on `stellar`, the return of the mission in `slot`,
    /// completes it once its destination is reached and its goals are
    /// met, and it has not failed (`_MissionLandCheck` @0xa1a4e-0xa1ace,
    /// `_DoMissionSuccess` @0xa03fc), drawing on `chance`: the slot freed,
    /// then its `OnSuccess`, its `DatePostInc` days, the legal records
    /// ([`success_records`]), its pay, the offers' stellar forgotten, and
    /// a re-offer due. Its destination is reached here when it is the
    /// return, or there is none.
    fn mission_success_at(&mut self, slot: u8, stellar: StellarId, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get_mut(slot) else {
            return;
        };
        if !at(active.return_to, stellar) {
            return;
        }
        if active
            .destination
            .is_none_or(|stop| at(active.return_to, stop.stellar))
        {
            active.reached = true;
            self.save_due = true;
        }
        if !active.reached || !active.goals_met || active.failed.is_some() {
            return;
        }
        let mission = active.mission;
        let record = self.lifecycle_record(mission);
        // `CompText`: missions phase 4, through `Session::mission_text`,
        // here, before the slot is freed, as the original fills it
        // (@0xa0457).
        let Some(done) = self.free_slot(slot) else {
            return;
        };
        self.run_mission_script(slot, &record.on_success, chance);
        self.pass_days(post_days(record.date_post_inc), chance);
        let changes = success_records(&record, &self.govts, |govt| self.pilot.legal_record(govt));
        self.set_records(&changes);
        self.apply_pay(done.pay);
        self.forget_offers();
        self.reoffer_due = true;
    }

    /// The deadline check of the mission in `slot` (`_MissionObjectivesCheck`
    /// @0x9eba5-0x9ec61), drawing on `chance`: with none of its days left,
    /// not failed already, and the ship not docked, it fails at once
    /// (`_QuickMissionFailure` @0x99d1b): its `OnFailure`, then marked
    /// failed by its deadline; an abortable one ends, its slot freed, no
    /// `OnAbort` run; and the offers' stellar is forgotten either way. An
    /// unabortable one stays active and failed until its return.
    fn mission_deadline(&mut self, slot: u8, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        if active.failed.is_some() || active.days_left != Some(0) || self.landed.is_some() {
            return;
        }
        let record = self.lifecycle_record(active.mission);
        // The comm message, `STR#` 2002 0x11d unless `Flags` has 0x0400:
        // missions phase 4.
        self.run_mission_script(slot, &record.on_failure, chance);
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.failed = Some(FailedBy::Deadline);
        }
        if record.can_abort != 0 {
            self.end_mission(slot, &record, false, chance);
        }
        self.forget_offers();
    }

    /// Re-offers the missions at the stellar docked at, the table kept,
    /// once after a landing's last slot when a mission succeeded
    /// (`_RandomizeMissionData` @0xa1b04), drawing on `chance`.
    pub(super) fn reoffer_missions(&mut self, chance: &mut dyn Chance) {
        if !std::mem::take(&mut self.reoffer_due) {
            return;
        }
        let Some(stellar) = self.landed else {
            return;
        };
        let mut offers = std::mem::take(&mut self.offers);
        self.at_visit(stellar, |visit| {
            offers.reoffer(&self.missions, visit, chance);
        });
        self.offers = offers;
    }

    /// `A`: ends every slot holding `mission`, running its `OnAbort`,
    /// whatever its `CanAbort` (`_EvalSetExp` @0x152f3-0x1534c), drawing
    /// on `chance`.
    pub(super) fn script_abort(&mut self, mission: MissionId, chance: &mut dyn Chance) {
        let record = self.lifecycle_record(mission);
        for slot in self.pilot.missions.holding(mission) {
            self.end_mission(slot, &record, true, chance);
        }
    }

    /// `F`: marks every slot holding `mission` failed, running nothing
    /// (`_EvalSetExp` @0x15353-0x153b2): it fails at its return. One
    /// failed by its deadline already stays so.
    pub(super) fn script_fail(&mut self, mission: MissionId) {
        for slot in self.pilot.missions.holding(mission) {
            if let Some(active) = self.slot_holding(slot, mission) {
                active.failed.get_or_insert(FailedBy::Operator);
            }
        }
    }

    /// `S`: accepts `mission` at once, drawing on `chance`
    /// (`_EvalSetExp` @0x153b9-0x15418): resolved from the stellar docked
    /// at, or in flight from the first stellar the player's system lists
    /// (128 with none, `_RandomizeOneMission` @0x153ff), then accepted as
    /// the player's accept is once its checks pass (`_DoMissionAccept`
    /// @0x15407, its free-slot check @0xa1b32), without asking whether it
    /// is available. A mission active already takes another slot, unless
    /// [`RuleKey::StartActiveMission`] reads otherwise. Nothing for a
    /// mission the session has no record of; with every slot taken, its
    /// values are still drawn, then nothing is accepted.
    pub(super) fn script_start(&mut self, mission: MissionId, chance: &mut dyn Chance) {
        let Some(record) = self.mission_record(mission).cloned() else {
            return;
        };
        let restarts = self.rules.source_for(RuleKey::StartActiveMission) == RuleSource::Engine;
        if !restarts && !self.pilot.missions.holding(mission).is_empty() {
            return;
        }
        let origin = self
            .landed
            .or_else(|| self.sites.first().map(|site| site.id))
            .unwrap_or(StellarId(128));
        let today = self.pilot.date;
        let offer = self.in_world(|world| {
            mission::resolve(
                &record,
                Venue::of(record.avail_loc),
                origin,
                world,
                today,
                chance,
            )
        });
        if self.accept_refusal(chance).is_err() {
            return;
        }
        self.start_mission(&record, &offer, chance);
    }

    /// Rolls the offers at `stellar`, the stellar landed on, drawing on
    /// `chance`, unless they were rolled there last (see the
    /// [`mission`](crate::mission) module).
    pub(super) fn roll_offers(&mut self, stellar: StellarId, chance: &mut dyn Chance) {
        let mut offers = std::mem::take(&mut self.offers);
        self.at_visit(stellar, |visit| {
            offers.roll(&self.missions, visit, chance);
        });
        self.offers = offers;
    }

    /// Forgets the stellar the offers were rolled at, so the next landing
    /// rolls them afresh: the hook a jump's arrival calls, and missions
    /// phase 2's abort, failure and success (@0x99bcf, @0x99db5,
    /// @0xa03ef, @0xa0691).
    pub(crate) fn forget_offers(&mut self) {
        self.offers.forget();
    }

    /// Empties the offers, keeping the stellar they were rolled at.
    pub(super) fn empty_offers(&mut self) {
        self.offers.empty();
    }

    /// The offers at `venue` of the visit under way, in the order rolled:
    /// none before the first landing settles.
    #[must_use]
    pub fn mission_offers(&self, venue: Venue) -> &[Offer] {
        self.offers.at(venue)
    }

    /// Why `mission` is not offered at the stellar the ship is docked at,
    /// at its own venue: every check that refuses, by name, in the
    /// checks' order, the visit's rolls read and nothing drawn
    /// ([`Checks::all_refusals`](crate::rules::Checks::all_refusals)).
    /// Empty when it is offered there; `None` when the ship is not docked
    /// or there is no such `mïsn`.
    #[must_use]
    pub fn offer_refusals(
        &self,
        mission: MissionId,
    ) -> Option<Vec<(&'static str, Verdict<Refusal>)>> {
        let record = self.missions.iter().find(|record| record.id == mission)?;
        let stellar = self.landed?;
        self.at_visit(stellar, |visit| {
            let facts = AvailFacts {
                visit,
                mission: record,
                venue: Venue::of(record.avail_loc),
                rolls: self.offers.table(),
            };
            mission::availability()
                .all_refusals(&facts)
                .into_iter()
                .map(|(descriptor, verdict)| (descriptor.name(), verdict))
                .collect()
        })
    }

    /// The room a mission's cargo has, in tons, as `_TotalMissionCargoSpace`
    /// (@0xcffb) reckons it: the ship's own holds, with its cargo outfits,
    /// less the cargo held that the trader escorts' holds do not take; below
    /// 0 when it holds more. Mission cargo, which rides in the ship alone,
    /// is missions phase 5's.
    fn mission_cargo_space(&self) -> i64 {
        let own = i64::from(self.stats.capacity);
        let escorts = i64::from(self.capacity()) - own;
        let held: i64 = self.pilot.cargo.values().map(|&tons| i64::from(tons)).sum();
        own - (held - escorts).max(0)
    }

    /// `f` of the galaxy a mission's stellars are chosen from.
    fn in_world<T>(&self, f: impl FnOnce(&World) -> T) -> T {
        let stellars = stellars_of(&self.galaxy, &self.listed_stellars);
        f(&World {
            stellars: &stellars,
            map: self.galaxy.map(),
            govts: &self.govts,
        })
    }

    /// `f` of the visit to `stellar`, one of the stellars shown: `None`
    /// when it is not shown.
    pub(super) fn at_visit<T>(&self, stellar: StellarId, f: impl FnOnce(&Visit) -> T) -> Option<T> {
        let docked = *self.sites.iter().find(|site| site.id == stellar)?;
        let map = self.galaxy.map();
        let ship = self.ship_record(self.pilot.ship);
        let player = Player {
            record: map
                .govt(self.shown)
                .map_or(0, |govt| self.pilot.legal_record(govt)),
            rating: self.pilot.combat_rating,
            ship: self.pilot.ship,
            inherent_ai: ship.map_or(0, |ship| ship.inherent_ai),
            inherent_govt: ship.and_then(|ship| ship.inherent_govt),
            combat_govt: ship.and_then(|ship| ship.combat_govt),
            contributed: wares::contributed(
                self.fields.contribute,
                &self.pilot.outfits,
                &self.outfits,
            ),
            fuel: self.pilot.reserves.fuel.now,
            free_cargo: self.mission_cargo_space(),
            cash: self.pilot.cash,
        };
        let active = self.pilot.missions.missions();
        Some(self.in_world(|&world| {
            f(&Visit {
                docked,
                system: self.shown,
                world,
                player,
                active: &active,
                gate: self.gate(),
                rules: &self.rules,
                today: self.pilot.date,
            })
        }))
    }

    /// The visit's roll table.
    #[cfg(test)]
    pub(crate) fn offer_rolls(&self) -> &mission::RollTable {
        self.offers.table()
    }

    /// What `text`'s tags read for `mission`, or no mission: the pilot,
    /// its ship, no person, and the session's cash, date and rules.
    fn tag_context(&self, mission: Option<MissionValues>) -> TagContext<'_> {
        TagContext {
            pilot: &self.pilot.name,
            ship_type: self.ship_name(self.pilot.ship).unwrap_or_default(),
            ship_name: self.pilot.ship_name().unwrap_or_default(),
            person: None,
            mission: mission.map(|values| MissionTags {
                values,
                cash: self.pilot.cash,
                today: self.pilot.date,
                date_affixes: &self.date_affixes,
                pay_millions: self.rules.source_for(RuleKey::PayMillions),
            }),
        }
    }

    /// `text` with its tags filled for `mission`'s offer on the visit
    /// under way (`_MungeBriefing` with `offer` set; see
    /// [`text_tags`](crate::text_tags)): none when it is not offered, or
    /// in flight.
    #[must_use]
    pub fn offer_text(&self, mission: MissionId, text: &str) -> Option<String> {
        let offer = self.offer_of(mission)?;
        let context = self.tag_context(Some(MissionValues::offered(&offer)));
        Some(text_tags::expand(text, &context, self))
    }

    /// `text` with its tags filled for the mission in `slot`
    /// (`_MungeBriefing` with a slot; see [`text_tags`](crate::text_tags)):
    /// a free slot's mission tags read [`UNSET`](text_tags::UNSET).
    #[must_use]
    pub fn mission_text(&self, slot: u8, text: &str) -> String {
        let values = self
            .pilot
            .missions
            .get(slot)
            .map(MissionValues::active)
            .unwrap_or_default();
        text_tags::expand(text, &self.tag_context(Some(values)), self)
    }

    /// `text` with its tags filled with no mission, as the original's
    /// `RefuseText`, comm dialog and hail quotes are (`_MungeBriefing`
    /// with index -1; see [`text_tags`](crate::text_tags)): the mission
    /// tags are left as written.
    #[must_use]
    pub fn plain_text(&self, text: &str) -> String {
        text_tags::expand(text, &self.tag_context(None), self)
    }
}

/// The names the session read as it started, and the cargo names its
/// string lists hold (`STR#` 4001).
impl TagNames for Session {
    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        self.place_names.stellars.get(&stellar).cloned()
    }

    fn system_name(&self, system: SystemId) -> Option<String> {
        self.place_names.systems.get(&system).cloned()
    }

    fn cargo_name(&self, cargo: u8) -> Option<String> {
        self.strings
            .0
            .string_list(CARGO_NAMES)
            .into_iter()
            .nth(usize::from(cargo))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::geometry::Vec2;
    use crate::mission::{ROLLED, Stop};
    use crate::pilot::Pilot;
    use crate::testkit::{Draws, FakePilotCatalog, catalog, jump, land_now, mission, planet};

    /// [`catalog`] with stellar 150 in system 132, two jumps from 130, 131
    /// linked back to 130, and missions 130 (to a random inhabited
    /// stellar), 131 (to 140) and 132 (`AvailRandom` 10), each at the
    /// mission computer.
    fn offering() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog
            .sites
            .push((SystemId(132), vec![planet(150, 0.0, 0.0)]));
        let mut random = mission(130);
        random.travel_stel = -2;
        let mut fixed = mission(131);
        fixed.travel_stel = 140;
        let mut rare = mission(132);
        rare.avail_random = 10;
        catalog.missions = vec![random, fixed, rare];
        catalog
    }

    fn ids(offers: &[Offer]) -> Vec<i16> {
        offers.iter().map(|offer| offer.mission.0).collect()
    }

    /// A session of `catalog` landed on 128 and settled on `chance`.
    fn landed(catalog: &FakePilotCatalog, chance: &mut dyn Chance) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, chance);
        session
    }

    /// Every roll's draw, then each of `more`.
    fn table_then(more: &[u32]) -> Vec<u32> {
        let mut asked = vec![100; ROLLED];
        asked.extend_from_slice(more);
        asked
    }

    #[test]
    fn a_refused_accept_or_abort_reads_plainly() {
        assert_eq!(
            AcceptError::Refused(AcceptRefusal::SlotsFull).to_string(),
            "the mission cannot be accepted: every mission slot is taken"
        );
        assert_eq!(
            AcceptError::NotOffered.to_string(),
            "the mission is not offered here"
        );
        assert_eq!(
            AbortError::Refused(AbortRefusal::CannotAbort).to_string(),
            "the mission cannot be aborted"
        );
        assert_eq!(
            AbortError::NotActive.to_string(),
            "no mission is in that slot"
        );
    }

    #[test]
    fn landing_rolls_the_mission_computers_offers_as_the_landing_settles() {
        let catalog = offering();
        let mut session = Session::start(&catalog).expect("starts");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "in flight"
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "not settled"
        );
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(
            chance.asked,
            table_then(&[1]),
            "the table, then 130's stellar"
        );
        let offers = session.mission_offers(Venue::Computer);
        assert_eq!(ids(offers), [130, 131], "132's roll of 100 is above 10");
        let stop = |stellar, system| {
            Some(Stop {
                stellar: StellarId(stellar),
                system: Some(SystemId(system)),
            })
        };
        assert_eq!(offers[0].destination, stop(150, 132));
        assert_eq!(offers[1].destination, stop(140, 131));
        assert!(session.mission_offers(Venue::Outside).is_empty());
    }

    #[test]
    fn landing_again_on_the_same_stellar_keeps_the_offers_and_draws_nothing() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        let offers = session.mission_offers(Venue::Computer).to_vec();
        assert_eq!(ids(&offers), [130, 131, 132], "every roll 1");
        assert_eq!(session.take_off(&mut NeverFires), Some(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        assert_eq!(session.mission_offers(Venue::Computer), offers);
    }

    /// Takes `session` off and puts it at rest over 128 in 130.
    fn back_over_128(session: &mut Session) {
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(128))
            .expect("128 shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
    }

    #[test]
    fn a_jumps_arrival_forgets_the_stellar_so_landing_back_rolls_afresh() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        assert_eq!(session.take_off(&mut NeverFires), Some(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        // From the centre, where `fly_out` heads straight out.
        session.player.position = Vec2::ZERO;
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(
            ids(session.mission_offers(Venue::Computer)),
            [130, 131, 132],
            "kept until the next landing"
        );
        assert_eq!(jump(&mut session, &catalog, 130), Some(SystemId(130)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        back_over_128(&mut session);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn after_the_forget_hook_landing_again_on_the_same_stellar_rolls_afresh() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.take_off(&mut NeverFires);
        session.forget_offers();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(session.offer_rolls().roll(MissionId(130)), Some(100));
    }

    #[test]
    fn a_landed_m_empties_the_offers_and_keeps_the_stellar() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(session.offers.rolled_at(), Some(StellarId(128)));
    }

    #[test]
    fn a_relocation_empties_the_offers_and_forgets_their_stellar() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("relocates");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(session.offers.rolled_at(), None);
    }

    #[test]
    fn a_pilot_flown_docked_has_its_offers_rolled_as_it_settles() {
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[1]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn flying_a_docked_pilot_raises_its_landing_after_its_exploration() {
        use crate::story::StoryEvent;
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let docked = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            docked.story_raised(),
            [
                StoryEvent::Explored {
                    system: SystemId(130)
                },
                StoryEvent::Landed {
                    stellar: StellarId(128)
                },
            ]
        );
        let in_flight = Session::start(&catalog).expect("starts");
        assert_eq!(
            in_flight.story_raised(),
            [StoryEvent::Explored {
                system: SystemId(130)
            }]
        );
    }

    #[test]
    fn the_roll_is_registered_and_runs_before_every_mission_slots_hooks() {
        let catalog = offering();
        let mut session = Session::start(&catalog).expect("starts");
        let slot = |record| SubscriptionId {
            consumer: Consumer::Missions,
            record,
            hook: 0,
        };
        assert!(ROLL < slot(0));
        assert!(ROLL > slot(-2));
        assert!(
            session.unregister_story(ROLL),
            "registered as the session starts"
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "unregistered, nothing rolls"
        );
    }

    #[test]
    fn the_offer_refusals_name_avail_random_and_draw_nothing() {
        let catalog = offering();
        let session = landed(&catalog, &mut Draws::of(&[]));
        assert_eq!(
            session.offer_refusals(MissionId(132)),
            Some(vec![(
                "AvailRandom",
                Verdict::Refuses(Refusal::AvailRandom)
            )])
        );
        assert_eq!(session.offer_refusals(MissionId(130)), Some(vec![]));
        assert_eq!(session.offer_refusals(MissionId(133)), None, "no such mïsn");
        let in_flight = Session::start(&catalog).expect("starts");
        assert_eq!(in_flight.offer_refusals(MissionId(130)), None);
    }

    #[test]
    fn an_active_mission_is_refused_as_not_active() {
        use crate::mission::ActiveMission;
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        let offer = session.mission_offers(Venue::Computer)[1].clone();
        let record = mission(131);
        session
            .pilot
            .missions
            .fill(ActiveMission::accepted(&record, &offer, session.date()));
        assert_eq!(
            session.offer_refusals(MissionId(131)),
            Some(vec![("not active", Verdict::Refuses(Refusal::Active))])
        );
        assert_eq!(session.offer_refusals(MissionId(130)), Some(vec![]));
    }

    #[test]
    fn a_missions_cargo_space_is_the_ships_own_less_the_cargo_the_escorts_do_not_take() {
        use crate::market::Good;
        use crate::pilot::Escort;
        let mut catalog = offering();
        catalog.ship_records = vec![crate::catalog::ShipRecord {
            inherent_ai: 1,
            ..crate::testkit::ship(
                129,
                crate::handling::ShipFields {
                    holds: 15,
                    ..crate::testkit::FAST
                },
            )
        }];
        catalog.missions = vec![MissionRecord {
            flags2: 0x0001,
            cargo_qty: 6,
            ..mission(140)
        }];
        let mut session = landed(&catalog, &mut NeverFires);
        session.pilot.escorts = vec![Escort {
            ship: crate::catalog::ShipId(129),
            reserves: crate::reserves::Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        let refused = |session: &Session| {
            session
                .offer_refusals(MissionId(140))
                .expect("docked")
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        };
        let none: [&str; 0] = [];
        for (held, refusals) in [(0, &none[..]), (25, &none[..]), (30, &["cargo space"][..])] {
            session.pilot.cargo.insert(Good::Commodity(0), held);
            assert_eq!(
                refused(&session),
                refusals,
                "{held} tons held: 20 own, 15 the escort's"
            );
        }
        assert_eq!(session.mission_cargo_space(), 5);
        session.pilot.escorts.clear();
        session.pilot.cargo.insert(Good::Commodity(0), 14);
        assert_eq!(session.mission_cargo_space(), 6, "no escort");
        session.pilot.cargo.insert(Good::Commodity(0), 25);
        assert_eq!(session.mission_cargo_space(), -5, "over the holds");
    }

    #[test]
    fn the_offers_read_the_pilot_and_its_ship() {
        let mut catalog = offering();
        catalog.missions = vec![
            MissionRecord {
                avail_rating: 5,
                ..mission(140)
            },
            MissionRecord {
                avail_ship_type: 1128,
                ..mission(141)
            },
            MissionRecord {
                flags: 0x2000,
                ..mission(142)
            },
            MissionRecord {
                pay_val: -50100,
                ..mission(143)
            },
            MissionRecord {
                flags: 0x0008,
                ..mission(144)
            },
            MissionRecord {
                flags2: 0x0001,
                cargo_qty: 21,
                ..mission(145)
            },
            MissionRecord {
                require: 0x2,
                ..mission(146)
            },
            MissionRecord {
                avail_record: 3,
                ..mission(147)
            },
        ];
        let session = landed(&catalog, &mut NeverFires);
        let refused = |id| {
            session
                .offer_refusals(MissionId(id))
                .expect("docked")
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        };
        assert_eq!(refused(140), ["AvailRating"]);
        assert_eq!(refused(141), ["AvailShipType"], "flying 128");
        assert_eq!(refused(142), ["InherentAI"], "no ship record: AI 0");
        assert_eq!(refused(143), ["PayVal cash"]);
        assert_eq!(refused(144), [] as [&str; 0], "the tank holds 300");
        assert_eq!(refused(145), ["cargo space"], "20 tons");
        assert_eq!(refused(146), ["Require"], "the ship contributes 0x1");
        assert_eq!(refused(147), ["AvailRecord"]);
    }

    #[test]
    fn the_offers_take_a_pilot_and_ship_that_meet_them() {
        use crate::catalog::{GovtId, OutfitId, OutfitRecord, ShipRecord};
        let mut catalog = offering();
        // System 130 is govt 128's; the stellar docked at, 128, govt 129's.
        catalog.star_map[0].govt = Some(GovtId(128));
        catalog.sites[0].1[0].govt = Some(GovtId(129));
        catalog.ship_records = vec![ShipRecord {
            inherent_govt: Some(GovtId(130)),
            combat_govt: Some(GovtId(131)),
            ..crate::testkit::ship(128, crate::testkit::FAST)
        }];
        catalog.outfits = vec![OutfitRecord {
            contribute: 0x2,
            ..crate::testkit::outfit(200, &[])
        }];
        catalog.missions = vec![
            MissionRecord {
                avail_record: 3,
                ..mission(140)
            },
            MissionRecord {
                avail_rating: 5,
                ..mission(141)
            },
            MissionRecord {
                require: 0x2,
                ..mission(142)
            },
            MissionRecord {
                pay_val: -50100,
                ..mission(143)
            },
            MissionRecord {
                avail_ship_type: 2130,
                ..mission(144)
            },
            MissionRecord {
                avail_ship_type: 2131,
                ..mission(145)
            },
        ];
        let mut session = landed(&catalog, &mut NeverFires);
        session.pilot.set_legal_record(GovtId(128), 3);
        session.pilot.set_legal_record(GovtId(129), -5);
        session.pilot.combat_rating = 5;
        session.pilot.outfits.insert(OutfitId(200), 1);
        session.pilot.cash = 100;
        for (id, why) in [
            (
                140,
                "a record of 3 with the system's govt, not the stellar's",
            ),
            (141, "a rating of 5"),
            (142, "the outfit contributes 0x2"),
            (143, "100 credits: -50000 less -50100"),
            (144, "the ship's inherent govt, 130"),
            (145, "the ship's combat govt, 131"),
        ] {
            assert_eq!(
                session.offer_refusals(MissionId(id)),
                Some(vec![]),
                "{id}: {why}"
            );
        }
    }

    // The active missions' lifecycle.

    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::catalog::{GovtId, GovtRecord};
    use crate::control::{Bit, SetOp};
    use crate::date::GameDate;
    use crate::mission::{AbortRefusal, ActiveMissions};
    use crate::rulebook::Rulebook;
    use crate::session::control::SetOpObserver;

    const FED: GovtId = GovtId(128);
    const CIVVIES: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(130);
    const NEUTRAL: GovtId = GovtId(131);

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// The date `days` days after `date`.
    fn after(date: GameDate, days: usize) -> GameDate {
        std::iter::successors(Some(date), |date| Some(date.next_day()))
            .nth(days)
            .expect("a date")
    }

    /// [`catalog`] with 131 linked back to 130, the Federation (class 1,
    /// allied with 13, at war with 9), the Civvies (class 13), the pirates
    /// (class 9) and a neutral (class 4), and `missions`.
    fn lifecycle(missions: Vec<MissionRecord>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.govts = vec![
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..crate::testkit::govt(128)
            },
            GovtRecord {
                classes: [13, -1, -1, -1],
                ..crate::testkit::govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..crate::testkit::govt(130)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..crate::testkit::govt(131)
            },
        ];
        catalog.missions = missions;
        catalog
    }

    /// The `X` operators each script runs, by the system each names: the
    /// marks a test's scripts leave, in the order run.
    #[derive(Debug, Default)]
    struct Marks(RefCell<Vec<i16>>);

    impl SetOpObserver for Marks {
        fn saw(&self, op: &SetOp, _session: &Session) {
            if let SetOp::Explore(system) = op {
                self.0.borrow_mut().push(system.0);
            }
        }
    }

    impl Marks {
        fn taken(&self) -> Vec<i16> {
            std::mem::take(&mut *self.0.borrow_mut())
        }
    }

    /// A session of `catalog` under `rules`, its marks watched, landed on
    /// 128 and settled, every roll 1.
    fn watched(catalog: &FakePilotCatalog, rules: Rulebook) -> (Session, Rc<Marks>) {
        let marks = Rc::new(Marks::default());
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_rules(rules)
            .with_set_op_observer(marks.clone());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        (session, marks)
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// Takes `session` off, settles, puts it at rest over `stellar` (one
    /// of 130's), lands there and settles.
    fn land_on(session: &mut Session, catalog: &FakePilotCatalog, stellar: i16) {
        if session.landed().is_some() {
            session.take_off(&mut NeverFires).expect("takes off");
            settle(session, catalog);
        }
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(session), Ok(StellarId(stellar)));
        settle(session, catalog);
    }

    fn accept(session: &mut Session, id: i16) -> u8 {
        session
            .accept_mission(MissionId(id), &mut NeverFires)
            .expect("accepted")
    }

    fn held(session: &Session) -> Vec<(u8, i16)> {
        session
            .pilot()
            .missions()
            .iter()
            .map(|(slot, active)| (slot, active.mission.0))
            .collect()
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    /// Mission `id`, offered at 128, to 129 and back to 129. (Offered
    /// anywhere, a mission to a stellar of the player's system would not
    /// be.)
    fn to_129(id: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            ..mission(id)
        }
    }

    /// Mission `id`, offered at 128, with no destination, completed at
    /// 128.
    fn back_to_128(id: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            return_stel: 128,
            ..mission(id)
        }
    }

    #[test]
    fn accepting_runs_on_accept_and_holds_the_offer_with_a_deadline_from_the_accept() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 3,
            pay_val: 900,
            on_accept: Script::parse("b20"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        let offer = session.mission_offers(Venue::Computer)[0].clone();
        let today = after(session.date(), 2);
        session.set_date(today);
        let _ = session.take_save_due();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        assert!(session.control_bit(bit(20)));
        assert_eq!(
            session.pilot().missions().get(0),
            Some(&ActiveMission {
                mission: MissionId(140),
                destination: offer.destination,
                return_to: offer.return_to,
                cargo_type: None,
                cargo_tons: 0,
                pay: 900,
                days_left: Some(3),
                deadline: Some(after(today, 3)),
                reached: false,
                goals_met: true,
                failed: None,
            })
        );
        assert_eq!(
            offer.destination.map(|stop| stop.stellar),
            Some(StellarId(129))
        );
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "dropped"
        );
        assert!(session.take_save_due());
        assert_eq!(
            session.offers.rolled_at(),
            Some(StellarId(128)),
            "nothing forgotten"
        );
    }

    #[test]
    fn an_accept_with_every_slot_taken_is_refused_and_runs_nothing() {
        let catalog = lifecycle(vec![MissionRecord {
            on_accept: Script::parse("b20"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        let offer = session.mission_offers(Venue::Computer)[0].clone();
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(141), &offer, session.date());
            session.pilot.missions.fill(active);
        }
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
        );
        assert!(!session.control_bit(bit(20)));
        assert_eq!(session.mission_offers(Venue::Computer), [offer]);
    }

    #[test]
    fn only_a_mission_offered_while_docked_is_accepted_or_refused() {
        let catalog = lifecycle(vec![to_129(140)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(
            session.accept_mission(MissionId(141), &mut NeverFires),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(
            session.refuse_mission(MissionId(141), &mut NeverFires),
            Err(NotOffered)
        );
        session.take_off(&mut NeverFires).expect("takes off");
        assert_eq!(session.mission_offers(Venue::Computer).len(), 1, "kept");
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Err(AcceptError::NotOffered),
            "in flight"
        );
        assert_eq!(
            session.refuse_mission(MissionId(140), &mut NeverFires),
            Err(NotOffered)
        );
        assert_eq!(held(&session), []);
    }

    #[test]
    fn an_accept_takes_the_lowest_slot_an_abort_freed() {
        let catalog = lifecycle(vec![to_129(140), to_129(141), to_129(142)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(accept(&mut session, 140), 0);
        assert_eq!(accept(&mut session, 141), 1);
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_eq!(accept(&mut session, 142), 0);
        assert_eq!(held(&session), [(0, 142), (1, 141)]);
    }

    #[test]
    fn an_accept_drops_the_offers_its_on_accept_makes_unavailable() {
        let catalog = lifecycle(vec![
            MissionRecord {
                on_accept: Script::parse("b21"),
                ..to_129(140)
            },
            MissionRecord {
                avail_bits: Test::parse("!b21"),
                ..to_129(141)
            },
            MissionRecord {
                avail_loc: 1,
                ..to_129(142)
            },
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [140, 141]);
        let kept = session.mission_offers(Venue::Outside).to_vec();
        accept(&mut session, 140);
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(session.mission_offers(Venue::Outside), kept, "as rolled");
    }

    #[test]
    fn an_accepted_missions_roll_is_0() {
        let catalog = lifecycle(vec![to_129(140), to_129(141)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        assert_eq!(session.offer_rolls().roll(MissionId(140)), Some(0));
        assert_eq!(session.offer_rolls().roll(MissionId(141)), Some(1));
    }

    #[test]
    fn a_pay_below_minus_50000_takes_its_cash_at_accept() {
        let catalog = lifecycle(vec![MissionRecord {
            pay_val: -50_100,
            ..to_129(140)
        }]);
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.cash = 1000;
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        accept(&mut session, 140);
        assert_eq!(session.pilot().cash(), 900);
    }

    #[test]
    fn an_on_accept_that_aborts_its_own_mission_leaves_the_slot_free_aborted_once() {
        let catalog = lifecycle(vec![MissionRecord {
            flags: 0x0001,
            on_accept: Script::parse("A719"),
            on_abort: Script::parse("X200"),
            ..mission(719)
        }]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        assert_eq!(accept(&mut session, 719), 0);
        assert_eq!(held(&session), []);
        assert_eq!(marks.taken(), [200], "the A's OnAbort, and no auto-abort");
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    #[test]
    fn refusing_runs_on_refuse_and_drops_the_offer_forgetting_nothing() {
        let catalog = lifecycle(vec![
            MissionRecord {
                on_refuse: Script::parse("b22"),
                ..to_129(140)
            },
            to_129(141),
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(
            session.refuse_mission(MissionId(140), &mut NeverFires),
            Ok(())
        );
        assert!(session.control_bit(bit(22)));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [141]);
        assert_eq!(session.offers.rolled_at(), Some(StellarId(128)));
        assert_eq!(held(&session), []);
    }

    #[test]
    fn aborting_runs_on_abort_and_frees_the_slot() {
        let catalog = lifecycle(vec![MissionRecord {
            on_abort: Script::parse("b23"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        let _ = session.take_save_due();
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert!(session.control_bit(bit(23)));
        assert_eq!(held(&session), []);
        assert!(session.take_save_due());
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
        assert_eq!(
            session.abort_mission(0, &mut NeverFires),
            Err(AbortError::NotActive)
        );
        assert_eq!(
            session.abort_mission(16, &mut NeverFires),
            Err(AbortError::NotActive)
        );
    }

    #[test]
    fn an_a_of_the_mission_inside_its_own_on_abort_ends_it_once() {
        let catalog = lifecycle(vec![
            MissionRecord {
                on_abort: Script::parse("A140 X203"),
                ..to_129(140)
            },
            MissionRecord {
                on_abort: Script::parse("A141 X204"),
                ..MissionRecord {
                    id: MissionId(141),
                    ..auto_aborting()
                }
            },
        ]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_eq!(marks.taken(), [203]);
        assert_eq!(accept(&mut session, 141), 0);
        assert_eq!(marks.taken(), [200, 204], "auto-aborted once");
        assert_eq!(held(&session), []);
    }

    #[test]
    fn can_abort_0_refuses_the_abort_and_runs_nothing() {
        let catalog = lifecycle(vec![MissionRecord {
            can_abort: 0,
            on_abort: Script::parse("b23"),
            comp_govt: Some(FED),
            comp_reward: 5,
            flags: 0x0040,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        assert_eq!(
            session.abort_mission(0, &mut NeverFires),
            Err(AbortError::Refused(AbortRefusal::CannotAbort))
        );
        assert!(!session.control_bit(bit(23)));
        assert_eq!(held(&session), [(0, 140)]);
        assert_eq!(session.pilot().legal_record(FED), 0, "no penalty");
        assert_eq!(session.offers.rolled_at(), Some(StellarId(128)));
    }

    #[test]
    fn an_abort_with_flag_0x0040_costs_five_times_the_reward() {
        let rewarding = |id, flags| MissionRecord {
            comp_govt: Some(FED),
            comp_reward: 5,
            flags,
            ..to_129(id)
        };
        let catalog = lifecycle(vec![rewarding(140, 0x0040), rewarding(141, 0x0002)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(FED, 10);
        accept(&mut session, 140);
        accept(&mut session, 141);
        assert_eq!(session.abort_mission(1, &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().legal_record(FED), 10, "no 0x0040");
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_eq!(session.pilot().legal_record(FED), -15);
    }

    #[test]
    fn after_an_abort_landing_again_on_the_same_stellar_rolls_afresh() {
        let catalog = lifecycle(vec![to_129(140)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.abort_mission(0, &mut NeverFires).expect("aborts");
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[]), "the whole table");
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [140]);
    }

    /// Mission 140, auto-aborting with nothing to do: no destination,
    /// return or ships, `DatePostInc` 2; its `OnAccept` marks 200 and its
    /// `OnAbort` 201.
    fn auto_aborting() -> MissionRecord {
        MissionRecord {
            flags: 0x0001,
            date_post_inc: 2,
            on_accept: Script::parse("X200"),
            on_abort: Script::parse("X201"),
            ..mission(140)
        }
    }

    #[test]
    fn accepting_an_auto_aborted_mission_aborts_it_at_once_passing_its_days() {
        let catalog = lifecycle(vec![auto_aborting()]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        let today = session.date();
        session.drop_raised();
        assert_eq!(accept(&mut session, 140), 0);
        assert_eq!(marks.taken(), [200, 201], "OnAccept, then OnAbort once");
        assert_eq!(session.date(), after(today, 2));
        let days = session
            .story_raised()
            .into_iter()
            .filter(|event| matches!(event, StoryEvent::DayPassed { .. }))
            .count();
        assert_eq!(days, 2);
        assert_eq!(held(&session), []);
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
        assert_eq!(session.offer_rolls().roll(MissionId(140)), Some(0));
    }

    #[test]
    #[allow(clippy::float_cmp)] // 300 less 100, exact
    fn an_auto_abort_with_flag_0x0008_takes_a_jumps_fuel() {
        let catalog = lifecycle(vec![MissionRecord {
            flags: 0x0009,
            ..auto_aborting()
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        let fuel = session.pilot().reserves().fuel.now;
        accept(&mut session, 140);
        assert_eq!(session.pilot().reserves().fuel.now, fuel - 100.0);
    }

    #[test]
    #[allow(clippy::float_cmp)] // untouched
    fn an_auto_abort_without_flag_0x0008_keeps_the_fuel() {
        let catalog = lifecycle(vec![auto_aborting()]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        let fuel = session.pilot().reserves().fuel.now;
        accept(&mut session, 140);
        assert_eq!(held(&session), []);
        assert_eq!(session.pilot().reserves().fuel.now, fuel);
    }

    #[test]
    fn a_cleaning_pay_sets_the_negative_records_of_a_government_and_its_allies_to_none() {
        let catalog = lifecycle(vec![MissionRecord {
            pay_val: -20_128,
            ..back_to_128(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        for (govt, record) in [(FED, -5), (CIVVIES, 3), (PIRATES, -5), (NEUTRAL, -5)] {
            session.pilot.set_legal_record(govt, record);
        }
        session.pilot.set_legal_record(GovtId(132), -7);
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), []);
        let records: Vec<i16> = [FED, CIVVIES, PIRATES, NEUTRAL, GovtId(132)]
            .into_iter()
            .map(|govt| session.pilot().legal_record(govt))
            .collect();
        assert_eq!(
            records,
            [0, 3, -5, -5, -7],
            "the Federation's and its ally's, the positive one kept"
        );
    }

    #[test]
    fn a_fining_pay_takes_its_percent_of_the_cash_at_the_return() {
        let catalog = lifecycle(vec![MissionRecord {
            pay_val: -40_010,
            ..back_to_128(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pilot.cash = 12_345;
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), []);
        assert_eq!(session.pilot().cash(), 11_110, "12345 less 10%, truncated");
    }

    #[test]
    fn a_mission_whose_record_is_gone_still_ends() {
        let catalog = lifecycle(vec![to_129(140), back_to_128(141)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        session.missions.clear();
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        let cash = session.pilot().cash();
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [], "completed with no pay");
        assert_eq!(session.pilot().cash(), cash);
    }

    #[test]
    fn a_return_that_is_the_destination_counts_it_reached_at_the_landing() {
        let catalog = lifecycle(vec![MissionRecord {
            on_success: Script::parse("b31"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        assert!(session.unregister_story(slot_hook(0, ARRIVAL)));
        land_on(&mut session, &catalog, 129);
        assert!(session.control_bit(bit(31)), "reached by the success hook");
        assert_eq!(held(&session), []);
    }

    #[test]
    fn an_auto_abort_with_flags2_0x0002_applies_its_pay() {
        let paying = |id, flags2| MissionRecord {
            flags2,
            pay_val: 500,
            ..MissionRecord {
                id: MissionId(id),
                ..auto_aborting()
            }
        };
        let catalog = lifecycle(vec![paying(140, 0x0002), paying(141, 0x0004)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        let cash = session.pilot().cash();
        accept(&mut session, 141);
        assert_eq!(session.pilot().cash(), cash, "no 0x0002");
        accept(&mut session, 140);
        assert_eq!(session.pilot().cash(), cash + 500);
    }

    #[test]
    fn an_auto_aborted_mission_with_a_destination_or_a_ship_stays_active() {
        let catalog = lifecycle(vec![
            MissionRecord {
                avail_stel: 128,
                travel_stel: 129,
                ..auto_aborting()
            },
            MissionRecord {
                id: MissionId(141),
                ships: crate::catalog::MissionShip { count: 1, goal: -1 },
                ..auto_aborting()
            },
        ]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        assert_eq!(held(&session), [(0, 140), (1, 141)]);
        assert_eq!(marks.taken(), [200, 200], "no OnAbort");
    }

    #[test]
    fn a_completion_pays_rewards_passes_its_days_and_re_offers_at_that_stellar() {
        let catalog = lifecycle(vec![
            MissionRecord {
                pay_val: 900,
                comp_govt: Some(FED),
                comp_reward: 4,
                date_post_inc: 2,
                on_success: Script::parse("b30"),
                ..to_129(140)
            },
            MissionRecord {
                avail_bits: Test::parse("b30"),
                ..mission(150)
            },
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        for govt in [FED, CIVVIES, PIRATES, NEUTRAL] {
            session.pilot.set_legal_record(govt, 10);
        }
        let cash = session.pilot().cash();
        accept(&mut session, 140);
        assert!(session.mission_offers(Venue::Computer).is_empty());
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        let landed = session.date();
        land_on(&mut session, &catalog, 129);
        assert!(session.control_bit(bit(30)));
        assert_eq!(held(&session), []);
        assert_eq!(session.pilot().cash(), cash + 900);
        let records: Vec<i16> = [FED, CIVVIES, PIRATES, NEUTRAL]
            .into_iter()
            .map(|govt| session.pilot().legal_record(govt))
            .collect();
        assert_eq!(records, [14, 12, 8, 10], "the govt, its ally, its enemy");
        assert_eq!(session.date(), after(landed, 2));
        assert_eq!(
            ids(session.mission_offers(Venue::Computer)),
            [150],
            "re-offered with OnSuccess's bit"
        );
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    #[test]
    fn missions_settled_on_one_landing_run_in_slot_order_each_failure_before_its_success() {
        let catalog = lifecycle(vec![
            MissionRecord {
                on_failure: Script::parse("X200"),
                on_success: Script::parse("X210"),
                ..back_to_128(140)
            },
            MissionRecord {
                on_failure: Script::parse("X201"),
                on_success: Script::parse("X211"),
                ..back_to_128(141)
            },
            MissionRecord {
                on_failure: Script::parse("X202"),
                on_success: Script::parse("X212"),
                ..back_to_128(142)
            },
        ]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        for id in [140, 141, 142] {
            accept(&mut session, id);
        }
        run(&mut session, "F140 F142");
        assert_eq!(marks.taken(), Vec::<i16>::new(), "F runs nothing");
        land_on(&mut session, &catalog, 128);
        assert_eq!(marks.taken(), [200, 211, 202]);
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_failure_applies_half_the_reward_and_forgets_the_stellar() {
        let catalog = lifecycle(vec![MissionRecord {
            comp_govt: Some(FED),
            comp_reward: 5,
            ..back_to_128(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(FED, 10);
        accept(&mut session, 140);
        run(&mut session, "F140");
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), []);
        assert_eq!(session.pilot().legal_record(FED), 8);
        assert_eq!(session.offers.rolled_at(), None);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[]), "the whole table");
    }

    #[test]
    fn a_mission_with_a_destination_completes_only_once_it_was_reached() {
        let catalog = lifecycle(vec![MissionRecord {
            return_stel: 128,
            on_success: Script::parse("b31"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [(0, 140)], "not there yet");
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().get(0).expect("held").reached);
        assert!(!session.control_bit(bit(31)));
        land_on(&mut session, &catalog, 128);
        assert!(session.control_bit(bit(31)));
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_mission_whose_ship_goal_is_not_met_stays_active_at_its_return() {
        let catalog = lifecycle(vec![MissionRecord {
            ships: crate::catalog::MissionShip { count: 1, goal: 0 },
            ..back_to_128(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [(0, 140)]);
        assert!(session.pilot().missions().get(0).expect("held").reached);
    }

    #[test]
    fn a_day_takes_one_off_each_limit_and_none_below_none() {
        let catalog = lifecycle(vec![
            MissionRecord {
                time_limit: 2,
                ..to_129(140)
            },
            to_129(141),
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        let left = |session: &Session| {
            session
                .pilot()
                .missions()
                .iter()
                .map(|(_, active)| active.days_left)
                .collect::<Vec<_>>()
        };
        session.pass_days(1, &mut NeverFires);
        assert_eq!(left(&session), [Some(1), None]);
        session.pass_days(2, &mut NeverFires);
        assert_eq!(left(&session), [Some(0), None]);
    }

    #[test]
    fn a_missed_deadline_fails_the_mission_after_the_take_offs_day() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            on_failure: Script::parse("b40"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        assert!(!session.control_bit(bit(40)), "docked: not yet");
        assert_eq!(held(&session), [(0, 140)]);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert!(session.control_bit(bit(40)));
        assert_eq!(held(&session), []);
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    #[test]
    fn a_deadline_that_ran_out_docked_fails_on_the_take_off_with_no_day() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            on_failure: Script::parse("b40"),
            ..to_129(140)
        }]);
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let (mut session, _) = watched(&catalog, rules);
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        let today = session.date();
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(session.date(), today, "no day passed");
        assert!(session.control_bit(bit(40)));
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_deadline_that_runs_out_in_flight_fails_on_that_day_with_no_take_off() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            on_failure: Script::parse("b40"),
            ..to_129(140)
        }]);
        let rules = Rulebook::default().with_override(RuleKey::StayDays, RuleSource::Bible);
        let (mut session, _) = watched(&catalog, rules);
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert!(!session.control_bit(bit(40)), "a day still left");
        assert_eq!(held(&session), [(0, 140)]);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        assert!(session.control_bit(bit(40)));
        assert_eq!(held(&session), []);
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    /// Mission 140, unabortable, timed one day, completed at 128 for the
    /// Federation's reward of 4; its `OnFailure` marks 200.
    fn unabortable() -> MissionRecord {
        MissionRecord {
            can_abort: 0,
            time_limit: 1,
            comp_govt: Some(FED),
            comp_reward: 4,
            on_failure: Script::parse("X200"),
            ..back_to_128(140)
        }
    }

    /// Takes an unabortable mission past its deadline in flight, then
    /// back to its return under `rules`: the marks at the deadline and at
    /// the return, and the Federation's record at each.
    fn missed_unabortable(rules: Rulebook) -> [(Vec<i16>, i16); 2] {
        let catalog = lifecycle(vec![unabortable()]);
        let (mut session, marks) = watched(&catalog, rules);
        session.pilot.set_legal_record(FED, 10);
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        let active = session.pilot().missions().get(0).expect("stays").clone();
        assert_eq!(active.failed, Some(FailedBy::Deadline));
        let deadline = (marks.taken(), session.pilot().legal_record(FED));
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), []);
        [deadline, (marks.taken(), session.pilot().legal_record(FED))]
    }

    #[test]
    fn by_default_an_unabortable_missions_deadline_fails_it_again_at_its_return() {
        assert_eq!(
            missed_unabortable(Rulebook::default()),
            [(vec![200], 10), (vec![200], 8)],
            "OnFailure twice, the records at the return"
        );
    }

    #[test]
    fn by_the_other_reading_its_return_frees_it_running_on_failure_no_second_time() {
        let rules =
            Rulebook::default().with_override(RuleKey::RepeatMissionFailure, RuleSource::Bible);
        assert_eq!(
            missed_unabortable(rules),
            [(vec![200], 10), (vec![], 8)],
            "OnFailure once, the records once"
        );
    }

    #[test]
    fn an_unabortable_missions_missed_deadline_forgets_the_stellar_while_it_stays_held() {
        let catalog = lifecycle(vec![MissionRecord {
            can_abort: 0,
            time_limit: 1,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        let active = session.pilot().missions().get(0).expect("stays");
        assert_eq!(active.failed, Some(FailedBy::Deadline));
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
        back_over_128(&mut session);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(held(&session), [(0, 140)], "not its return");
        assert_eq!(chance.asked, table_then(&[]), "the whole table");
    }

    #[test]
    fn a_mission_failed_by_f_runs_on_failure_once_at_its_return_by_either_reading() {
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = lifecycle(vec![MissionRecord {
                time_limit: 0,
                ..unabortable()
            }]);
            let rules = Rulebook::default().with_override(RuleKey::RepeatMissionFailure, reading);
            let (mut session, marks) = watched(&catalog, rules);
            session.pilot.set_legal_record(FED, 10);
            accept(&mut session, 140);
            run(&mut session, "F140");
            assert_eq!(
                session.pilot().missions().get(0).and_then(|m| m.failed),
                Some(FailedBy::Operator)
            );
            land_on(&mut session, &catalog, 128);
            assert_eq!(marks.taken(), [200], "{reading:?}");
            assert_eq!(session.pilot().legal_record(FED), 8, "{reading:?}");
            assert_eq!(held(&session), []);
        }
    }

    #[test]
    fn a_deadline_a_completions_days_run_out_docked_fails_only_after_the_take_off() {
        let catalog = lifecycle(vec![
            MissionRecord {
                time_limit: 2,
                on_failure: Script::parse("b42"),
                ..to_129(140)
            },
            MissionRecord {
                date_post_inc: 5,
                ..back_to_128(141)
            },
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [(0, 140)], "141 completed");
        let active = session.pilot().missions().get(0).expect("held");
        assert_eq!((active.days_left, active.failed), (Some(0), None));
        assert!(!session.control_bit(bit(42)));
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert!(session.control_bit(bit(42)));
        assert_eq!(held(&session), []);
    }

    #[test]
    fn s_accepts_the_mission_running_its_on_accept_before_the_next_operator() {
        let catalog = lifecycle(vec![MissionRecord {
            on_accept: Script::parse("X201"),
            ..to_129(140)
        }]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        run(&mut session, "S140 X202");
        assert_eq!(marks.taken(), [201, 202]);
        assert_eq!(held(&session), [(0, 140)]);
        let active = session.pilot().missions().get(0).expect("held");
        assert_eq!(
            active.destination.map(|stop| stop.stellar),
            Some(StellarId(129))
        );
        assert!(
            session.mission_offers(Venue::Computer).is_empty(),
            "dropped"
        );
        run(&mut session, "S141");
        assert_eq!(held(&session), [(0, 140)], "no such mïsn");
    }

    /// A session of [`lifecycle`], with 133 (which lists no stellars)
    /// shown too and mission 140 to its origin (`TravelStel` -4), started
    /// in flight in `system`, after an `S140`.
    fn started_in_flight_in(system: i16) -> Session {
        let mut catalog = lifecycle(vec![MissionRecord {
            travel_stel: -4,
            ..mission(140)
        }]);
        catalog.systems.push(SystemId(133));
        if let Ok(start) = &mut catalog.character {
            start.systems[0] = Some(SystemId(system));
        }
        let mut session = Session::start(&catalog).expect("starts");
        settle(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(system));
        run(&mut session, "S140");
        session
    }

    fn destination(session: &Session) -> Option<StellarId> {
        let active = session.pilot().missions().get(0).expect("held");
        active.destination.map(|stop| stop.stellar)
    }

    #[test]
    fn s_in_flight_resolves_from_the_systems_first_stellar() {
        let session = started_in_flight_in(131);
        assert_eq!(destination(&session), Some(StellarId(140)), "131's first");
        assert_eq!(session.offer_rolls().roll(MissionId(140)), Some(0));
    }

    #[test]
    fn s_in_flight_in_a_system_with_no_stellars_resolves_from_128() {
        let session = started_in_flight_in(133);
        assert!(session.sites.is_empty(), "133 lists none");
        assert_eq!(destination(&session), Some(StellarId(128)));
    }

    #[test]
    fn s_of_a_mission_active_already_takes_another_slot_by_the_engine() {
        let catalog = lifecycle(vec![MissionRecord {
            on_accept: Script::parse("X201"),
            ..to_129(140)
        }]);
        for (reading, slots, marks_run) in [
            (RuleSource::Engine, vec![(0, 140), (1, 140)], vec![201, 201]),
            (RuleSource::Bible, vec![(0, 140)], vec![201]),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::StartActiveMission, reading);
            let (mut session, marks) = watched(&catalog, rules);
            run(&mut session, "S140 S140");
            assert_eq!(held(&session), slots, "{reading:?}");
            assert_eq!(marks.taken(), marks_run, "{reading:?}");
        }
    }

    #[test]
    fn s_with_every_slot_taken_does_nothing() {
        let catalog = lifecycle(vec![MissionRecord {
            on_accept: Script::parse("X201"),
            ..to_129(140)
        }]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        for _ in 0..SLOTS {
            run(&mut session, "S140");
        }
        let _ = marks.taken();
        run(&mut session, "S140");
        assert_eq!(marks.taken(), Vec::<i16>::new());
        assert_eq!(session.pilot().missions().lowest_free(), None);
    }

    #[test]
    fn s_with_every_slot_taken_draws_the_missions_values_then_refuses() {
        let catalog = lifecycle(vec![
            MissionRecord {
                cargo_type: 1000,
                cargo_qty: -10,
                on_accept: Script::parse("X201"),
                ..to_129(140)
            },
            to_129(141),
        ]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        for _ in 0..SLOTS {
            run(&mut session, "S141");
        }
        let full = session.pilot().missions().clone();
        let mut chance = Draws::of(&[]);
        session.run_set(&SetExpr::parse("S140").expect("parses"), &mut chance);
        assert_eq!(
            chance.asked,
            [6, 10],
            "the random cargo, then its tons, before the slot check"
        );
        assert_eq!(marks.taken(), Vec::<i16>::new(), "no OnAccept");
        assert_eq!(session.pilot().missions(), &full, "nothing accepted");
    }

    #[test]
    fn a_runs_on_abort_and_frees_every_slot_holding_the_mission_whatever_can_abort() {
        let catalog = lifecycle(vec![
            MissionRecord {
                can_abort: 0,
                on_abort: Script::parse("X203"),
                ..to_129(140)
            },
            to_129(141),
        ]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        run(&mut session, "S140 S141 S140");
        run(&mut session, "A140");
        assert_eq!(marks.taken(), [203, 203]);
        assert_eq!(held(&session), [(1, 141)]);
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    #[test]
    fn f_marks_the_mission_failed_running_nothing_and_it_fails_at_its_return() {
        let catalog = lifecycle(vec![MissionRecord {
            on_failure: Script::parse("X200"),
            on_success: Script::parse("X210"),
            ..back_to_128(140)
        }]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        let _ = session.take_save_due();
        run(&mut session, "F140");
        assert_eq!(marks.taken(), Vec::<i16>::new());
        assert!(session.take_save_due());
        assert_eq!(
            session.pilot().missions().get(0).and_then(|m| m.failed),
            Some(FailedBy::Operator)
        );
        land_on(&mut session, &catalog, 128);
        assert_eq!(marks.taken(), [200]);
        assert_eq!(held(&session), []);
    }

    #[test]
    fn f_leaves_a_mission_failed_by_its_deadline_so() {
        let catalog = lifecycle(vec![unabortable()]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        run(&mut session, "F140");
        assert_eq!(
            session.pilot().missions().get(0).and_then(|m| m.failed),
            Some(FailedBy::Deadline)
        );
    }

    /// Lands on 128 with a fixture subscription that runs `S140 b2` on the
    /// landing, and another that runs `b3` on a day passing while `!b2`,
    /// under `reading` of `StoryReentry`: whether `b3` was set.
    fn reentered(reading: RuleSource) -> bool {
        let catalog = lifecycle(vec![MissionRecord {
            date_post_inc: 1,
            ..auto_aborting()
        }]);
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::StoryReentry, reading));
        let fixture = |hook, on, guard, script| Subscription {
            guard: Test::parse(guard),
            ..Subscription::new(
                SubscriptionId {
                    consumer: Consumer::Fixture,
                    record: 1,
                    hook,
                },
                EventFilter::Any(on),
                Script::parse(script),
            )
        };
        for sub in [
            fixture(0, StoryEventKind::Landed, "", "S140 b2"),
            fixture(1, StoryEventKind::DayPassed, "!b2", "b3"),
        ] {
            session.register_story(sub).expect("registers");
        }
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.control_bit(bit(2)));
        assert_eq!(held(&session), [], "auto-aborted");
        session.control_bit(bit(3))
    }

    #[test]
    fn by_the_engine_the_days_an_s_passes_dispatch_before_the_scripts_next_operator() {
        assert!(reentered(RuleSource::Engine));
    }

    #[test]
    fn by_the_other_reading_the_days_an_s_passes_wait_for_the_script() {
        assert!(!reentered(RuleSource::Bible));
    }

    #[test]
    fn a_saved_pilots_missions_fly_on_and_complete_at_their_return() {
        let catalog = lifecycle(vec![MissionRecord {
            on_success: Script::parse("b31"),
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        assert_eq!(pilot.missions(), session.pilot().missions());
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        settle(&mut session, &catalog);
        assert_eq!(held(&session), [(0, 140)]);
        land_on(&mut session, &catalog, 129);
        assert!(session.control_bit(bit(31)));
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_slot_freed_unregisters_its_hooks_and_clears_their_fired_entries() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 2,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        for hook in HOOKS {
            session.pilot.story_fired.insert(slot_hook(0, hook));
        }
        session.abort_mission(0, &mut NeverFires).expect("aborts");
        assert!(session.pilot().missions().get(0).is_none());
        for hook in HOOKS {
            assert!(!session.unregister_story(slot_hook(0, hook)), "{hook}");
        }
        assert!(session.pilot.story_fired.is_empty());
        assert_eq!(session.pilot().missions(), &ActiveMissions::default());
    }

    #[test]
    fn a_timed_slot_registers_its_deadline_checks_and_an_untimed_one_none() {
        let catalog = lifecycle(vec![
            MissionRecord {
                time_limit: 2,
                ..to_129(140)
            },
            to_129(141),
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        for (slot, timed) in [(0, true), (1, false)] {
            for hook in [ARRIVAL, FAILURE, SUCCESS] {
                assert!(
                    session.unregister_story(slot_hook(slot, hook)),
                    "{slot} {hook}"
                );
            }
            for hook in [DEADLINE, DEADLINE_TOOK_OFF] {
                assert_eq!(
                    session.unregister_story(slot_hook(slot, hook)),
                    timed,
                    "{slot} {hook}"
                );
            }
        }
        assert!(
            session.unregister_story(REOFFER),
            "registered as the session starts"
        );
        assert!(REOFFER > slot_hook(15, DEADLINE_TOOK_OFF));
        assert!(ROLL < slot_hook(0, ARRIVAL));
    }

    // The text tags.

    use crate::catalog::{CommCatalog, PlaceNames};
    use crate::text_tags::UNSET;

    /// `STR#` 4001, the cargo names, and 25048, one message naming the
    /// pilot and the destination.
    #[derive(Debug)]
    struct Told;

    impl CommCatalog for Told {
        fn string_list(&self, id: i16) -> Vec<String> {
            let strings: &[&str] = match id {
                4001 => &["food", "*passengers", "industrial"],
                25048 => &["<PN>: off to <DST>."],
                _ => &[],
            };
            strings.iter().map(|&string| string.to_owned()).collect()
        }
    }

    /// Stellars 128 Earth, 129 Luna, 140 Mars and 150 Vega Prime;
    /// systems 130 Sol, 131 Alpha and 132 Vega.
    fn place_names() -> PlaceNames {
        let names = |pairs: &[(i16, &str)]| {
            pairs
                .iter()
                .map(|&(id, name)| (id, name.to_owned()))
                .collect::<Vec<_>>()
        };
        PlaceNames {
            stellars: names(&[
                (128, "Earth"),
                (129, "Luna"),
                (140, "Mars"),
                (150, "Vega Prime"),
            ])
            .into_iter()
            .map(|(id, name)| (StellarId(id), name))
            .collect(),
            systems: names(&[(130, "Sol"), (131, "Alpha"), (132, "Vega")])
                .into_iter()
                .map(|(id, name)| (SystemId(id), name))
                .collect(),
        }
    }

    /// Stock's session of `catalog`, told [`Told`]'s strings, landed on
    /// 128 and settled, every roll the first.
    fn stocks(catalog: &FakePilotCatalog) -> Session {
        let mut pilot = Pilot::new(catalog, "Stock").expect("a pilot");
        pilot.ship_name = Some("Kestrel".to_owned());
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Told));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        session
    }

    const BRIEFING: &str = "<PN>: take <CQ> tons of <CT> to <DST> in <DSY> by <DL> for <PAY>.";

    /// [`offering`], its names given, with mission 130 (to a random
    /// inhabited stellar) carrying 12 tons of industrial for 25,000 in 8
    /// days.
    fn briefed() -> FakePilotCatalog {
        let mut catalog = offering();
        catalog.place_names = place_names();
        catalog.ship_records = vec![crate::testkit::ship(128, crate::testkit::FAST)];
        let random = &mut catalog.missions[0];
        random.cargo_type = 2;
        random.cargo_qty = 12;
        random.pay_val = 25_000;
        random.time_limit = 8;
        catalog
    }

    #[test]
    fn an_offers_text_names_the_stellar_it_resolved_and_its_slot_the_same() {
        let catalog = briefed();
        let mut session = stocks(&catalog);
        let filled = "Stock: take 12 tons of industrial to Vega Prime in Vega by July 1st, \
                      1177 NC for 25,000.";
        assert_eq!(
            session.offer_text(MissionId(130), BRIEFING).as_deref(),
            Some(filled),
            "not the encoding, -2"
        );
        assert_eq!(
            session.offer_text(MissionId(999), BRIEFING),
            None,
            "not offered"
        );
        let slot = accept(&mut session, 130);
        assert_eq!(session.mission_text(slot, BRIEFING), filled);
        assert_eq!(
            session.offer_text(MissionId(130), BRIEFING),
            None,
            "accepted"
        );
        assert_eq!(
            session.mission_text(slot, "<SN>|<OSN>"),
            format!("|{UNSET}"),
            "no ship name drawn yet, and no person"
        );
        assert_eq!(
            *catalog.place_name_reads.borrow(),
            1,
            "read once, at the start"
        );
    }

    #[test]
    fn an_offers_ship_name_and_person_read_unset() {
        let catalog = briefed();
        let session = stocks(&catalog);
        assert_eq!(
            session.offer_text(MissionId(130), "<SN> <OSN>").as_deref(),
            Some("[Error] [Error]")
        );
    }

    #[test]
    fn in_flight_nothing_is_offered_to_fill() {
        let catalog = briefed();
        let mut session = stocks(&catalog);
        session.take_off(&mut NeverFires).expect("takes off");
        assert_eq!(session.offer_text(MissionId(130), BRIEFING), None);
    }

    #[test]
    fn a_free_slots_mission_tags_read_unset_and_its_player_tags_fill() {
        let catalog = briefed();
        let session = stocks(&catalog);
        assert_eq!(
            session.mission_text(3, BRIEFING),
            "Stock: take [Error] tons of [Error] to [Error] in [Error] by [Error] for \
             [Error]."
        );
    }

    #[test]
    fn a_plain_text_fills_the_player_tags_and_leaves_the_mission_tags() {
        let catalog = briefed();
        let session = stocks(&catalog);
        assert_eq!(
            session.plain_text("<PN> of the <PSN>, a <PST>, <PRK>: <DST> <PAY> <OSN>"),
            "Stock of the Kestrel, a Ship 128, captain: <DST> <PAY> [Error]"
        );
    }

    #[test]
    fn a_pay_of_millions_follows_its_rule() {
        let mut catalog = briefed();
        catalog.missions[0].pay_val = 1_234_567;
        let session = stocks(&catalog);
        assert_eq!(
            session.offer_text(MissionId(130), "<PAY>").as_deref(),
            Some("1.23M")
        );
        let full = session
            .with_rules(Rulebook::default().with_override(RuleKey::PayMillions, RuleSource::Bible));
        assert_eq!(
            full.offer_text(MissionId(130), "<PAY>").as_deref(),
            Some("1,234,567")
        );
    }

    #[test]
    fn a_fining_pays_amount_is_its_percent_of_the_cash() {
        let mut catalog = briefed();
        catalog.missions[0].pay_val = -40_010;
        let mut session = stocks(&catalog);
        session.pilot.cash = 12_345;
        assert_eq!(
            session.offer_text(MissionId(130), "<PAY>").as_deref(),
            Some("1,234")
        );
    }

    // `Q` inside a mission's script.

    /// [`lifecycle`] of `missions`, with [`place_names`].
    fn scripted(missions: Vec<MissionRecord>) -> FakePilotCatalog {
        let mut catalog = lifecycle(missions);
        catalog.place_names = place_names();
        catalog
    }

    /// The message the next settling leaves on.
    fn left_on(session: &mut Session, catalog: &FakePilotCatalog) -> Option<String> {
        session.settle_script(catalog, &mut NeverFires).message
    }

    const FILLED: &str = "Stock: off to Luna.";
    const AS_WRITTEN: &str = "<PN>: off to <DST>.";

    #[test]
    fn a_q_in_a_missions_on_accept_fills_its_mission_tags() {
        let catalog = scripted(vec![MissionRecord {
            on_accept: Script::parse("Q25048"),
            ..to_129(140)
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(FILLED));
    }

    #[test]
    fn a_q_from_no_mission_or_its_on_refuse_is_shown_as_written() {
        let catalog = scripted(vec![MissionRecord {
            on_refuse: Script::parse("Q25048"),
            ..to_129(140)
        }]);
        let mut session = stocks(&catalog);
        run(&mut session, "Q25048");
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(AS_WRITTEN));
        let mut session = stocks(&catalog);
        session
            .refuse_mission(MissionId(140), &mut NeverFires)
            .expect("offered");
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(AS_WRITTEN));
    }

    #[test]
    fn a_q_in_a_missions_on_abort_fills_its_mission_tags() {
        let catalog = scripted(vec![MissionRecord {
            on_abort: Script::parse("Q25048"),
            ..to_129(140)
        }]);
        let mut session = stocks(&catalog);
        let slot = accept(&mut session, 140);
        session
            .abort_mission(slot, &mut NeverFires)
            .expect("aborts");
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(FILLED));
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        run(&mut session, "A140");
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some(FILLED),
            "by A too"
        );
    }

    #[test]
    fn a_q_in_an_auto_aborts_on_abort_fills_its_mission_tags() {
        let catalog = scripted(vec![MissionRecord {
            on_abort: Script::parse("Q25048"),
            ..auto_aborting()
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        assert_eq!(held(&session), [], "aborted at once");
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some("Stock: off to [Error]."),
            "its slot's, which has no stellar"
        );
    }

    #[test]
    fn a_q_in_a_missions_on_failure_fills_its_mission_tags() {
        let catalog = scripted(vec![MissionRecord {
            on_failure: Script::parse("Q25048"),
            ..to_129(140)
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        run(&mut session, "F140");
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        let site = *session
            .sites
            .iter()
            .find(|site| site.id == StellarId(129))
            .expect("shown");
        session.player.position = site.position;
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(FILLED));
        assert_eq!(held(&session), [], "failed at its return");
    }

    #[test]
    fn a_q_in_a_missed_deadlines_on_failure_fills_its_mission_tags() {
        let catalog = scripted(vec![MissionRecord {
            time_limit: 1,
            on_failure: Script::parse("Q25048"),
            ..to_129(140)
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some(FILLED),
            "missed as the take-off's day passes"
        );
    }

    #[test]
    fn a_q_in_a_missions_on_success_reads_its_freed_slot() {
        let catalog = scripted(vec![MissionRecord {
            on_success: Script::parse("Q25048"),
            ..back_to_128(140)
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some("Stock: off to [Error].")
        );
    }

    #[test]
    fn a_q_after_a_nested_missions_script_is_shown_as_written() {
        let catalog = scripted(vec![
            MissionRecord {
                on_accept: Script::parse("S141 Q25048"),
                ..to_129(140)
            },
            to_129(141),
        ]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        assert_eq!(held(&session), [(0, 140), (1, 141)]);
        assert_eq!(left_on(&mut session, &catalog).as_deref(), Some(AS_WRITTEN));
    }

    #[test]
    fn a_q_inside_a_nested_missions_script_fills_with_its_own_slot() {
        let catalog = scripted(vec![
            MissionRecord {
                on_accept: Script::parse("S141"),
                ..to_129(140)
            },
            MissionRecord {
                on_accept: Script::parse("Q25048"),
                travel_stel: 140,
                ..to_129(141)
            },
        ]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some("Stock: off to Mars.")
        );
    }
}
