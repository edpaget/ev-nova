//! The session's side of missions (see the [`mission`](crate::mission)
//! module): the visit's offers, rolled as the player lands and kept for
//! the visit, and the facts they are rolled from; and the active
//! missions' lifecycle.
//!
//! The roll is no loop of its own: [`Session::fly`] registers it as the
//! offers' one `Landed` subscription, [`ROLL`], `{Offers, 0, 0}`, whose
//! consumer effect rolls the offers at the stellar landed on
//! (`_HandlePlayerDockRequest` @0x66d2b-0x66d77), before the mission
//! ships' goal checks and every mission slot's landing hooks (see the
//! [`story`](crate::story) module's "Dispatch"). A load forgets the
//! stellar rolled at (@0x75ef5) and lands nothing, so a pilot flown from a
//! save has its offers rolled afresh at its first landing.
//!
//! What forgets the stellar rolled at, so that the next landing rolls the
//! offers afresh, is [`Session::forget_offers`]: the developer tools'
//! relocation through the entry table, and a mission's abort, failure
//! and success (@0x99bcf, @0x99db5, @0xa03ef, @0xa0691). A jump's arrival
//! forgets it and draws the visit's roll table afresh before the system
//! is populated (@0x6bfce, @0x6c040-0x6c066). A landed `M` or `N`
//! empties the offers and keeps the stellar (@0x1584b, @0x15949). None
//! of it is saved.
//!
//! **Offers from a ship.** In flight a mission is available from a ship
//! as [`Session::ship_mission_available`] checks it: the visit's table
//! read, the player's system and no stellar docked (see the
//! [`mission`](crate::mission) module's "From a ship"). The persons'
//! offers, their resolution at spawn and the accept or refusal of one are
//! the `person_offers` module's; an offer from a ship is accepted and
//! refused through [`Session::accept_mission`] and
//! [`Session::refuse_mission`] while its dialog is open.
//!
//! **Active missions.** The missions accepted are the pilot's, in their
//! slots (see the [`mission`](crate::mission) module's "Active
//! missions"). [`Session::accept_mission`], [`Session::refuse_mission`]
//! and [`Session::abort_mission`] are the player's; `S`, `A` and `F` are
//! `apply_op`'s arms. Each slot filled registers its hooks, `{Missions,
//! slot, hook}`: on every landing, in `_MissionLandCheck`'s order,
//! [`GOALS`] (the leading goal check), [`ARRIVAL`] (the cargo check, in
//! the `mission_cargo` module), [`RETURN`] (failure, else the escort
//! goal, then success) and [`RECHECK`] (the trailing goal check); and, for a mission with a deadline, [`DEADLINE`] on every day
//! and [`DEADLINE_TOOK_OFF`] on every take-off, each with a blank guard
//! and no script; their effects come through the dispatcher's consumer
//! match. [`REOFFER`], registered with [`ROLL`], re-offers after the last
//! slot's hooks when a mission succeeded. [`Session::fly`] registers
//! every active slot's hooks again, so a saved pilot's missions go on.
//! Each slot also registers its special ships' hooks, `{MissionShips,
//! slot, hook}`: the enter-system hook and the goal checks (see the
//! `mission_ships` module); an accept in flight queues its slot's first
//! goal check for the end of the settling. Each end frees its slot and
//! unregisters its hooks.
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
//! `script_effects` module). The texts the lifecycle shows are queued for
//! the view as they come (see the `mission_screens` module): `BriefText`
//! filled for the slot after `OnAccept` and the accept's charge, then
//! `LoadCargText` when the cargo is loaded at accept; `LoadCargText` and
//! `DumpCargoText` as the cargo is picked up and dropped off, filled for
//! the slot; `CompText` before the slot is freed; `FailText` after it is,
//! its mission tags reading [`UNSET`](text_tags::UNSET); `RefuseText`
//! with no mission, before `OnRefuse` runs; and an accept or a pickup
//! refused for want of room for the cargo. A deadline missed in flight
//! raises [`SimMessage::DeadlineMissed`] unless the mission is hidden.

use std::collections::BTreeMap;

use super::Session;
use super::mission_screens::MissionTextKind;
use crate::catalog::{
    AuxShip, DescMovie, GovtId, LandingSite, MissionId, MissionRecord, MissionShip, StellarId,
    SystemId,
};
use crate::chance::{Chance, NeverFires};
use crate::control::{Script, Test};
use crate::message::SimMessage;
use crate::mission::active::{AUTO_ABORT_FUEL, AUTO_ABORT_FUEL_TAKEN, AUTO_ABORT_PAYS};
use crate::mission::aux_ships::AuxShips;
use crate::mission::goals::{ShipGoal, met_at_return};
use crate::mission::screens::HIDDEN;
use crate::mission::{
    self, AbortFacts, AbortRefusal, AcceptError, AcceptFacts, AcceptRefusal, ActiveMission,
    AvailFacts, FailedBy, IdentityRules, Offer, PayEffect, Player, Refusal, Room, SLOTS, Stop,
    Venue, Visit, World, abort_checks, abort_penalty, accept_charge, accept_checks,
    auto_aborts_at_accept, cleaned, failure_records, fined, pay_effect, placements, stellars_of,
    success_records,
};
use crate::news::CARGO_NAMES;
use crate::rulebook::{RuleKey, RuleSource};
use crate::rules::Verdict;
use crate::story::{
    Consumer, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::text_parse::{ParseRules, parse_text_description};
use crate::text_tags::{
    self, MissionTags, MissionValues, RankTagGovts, TagContext, TagNames, TagRules,
};

/// Why a mission's stop has no landing a player can reach
/// ([`Session::stop_landing`]).
#[cfg(any(test, feature = "fixture"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StopRefusal {
    /// No system lists the stellar.
    NoSystem,
    /// Every system listing it is hidden, and no stellar identical to
    /// it is visible.
    Hidden,
}

/// The offer roll's subscription: the offers', which run first on a
/// landing, before the mission ships' goal checks and every mission
/// slot's (slots 0-15) landing hooks.
pub const ROLL: SubscriptionId = SubscriptionId {
    consumer: Consumer::Offers,
    record: 0,
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

/// A slot's leading goal check on `Landed` (`_MissionLandCheck`
/// @0xa1a0c; see the `mission_ships` module).
pub const GOALS: u8 = 0;
/// A slot's landing hook that picks its cargo up and drops it off and
/// marks its destination reached: the cargo check's place in
/// `_MissionLandCheck` (`_MissionLandCargoCheck` @0x9edc5, from
/// @0xa1a1b; see the `mission_cargo` module).
pub const ARRIVAL: u8 = 1;
/// A slot's landing hook at its mission's return (@0xa1a37-0xa1ace):
/// the destination marked reached, then a failed mission failed, else
/// the escort goal settled, then the mission completed.
pub const RETURN: u8 = 2;
/// A slot's trailing goal check on `Landed` (@0xa1ada), on whatever
/// mission the slot holds by then.
pub const RECHECK: u8 = 3;
/// A timed slot's daily deadline check, on `DayPassed`.
pub const DEADLINE: u8 = 4;
/// A timed slot's deadline check on `TookOff`, for a deadline that ran
/// out while docked with no day passing as the ship leaves.
pub const DEADLINE_TOOK_OFF: u8 = 5;

/// Every hook a slot can register, in order.
const HOOKS: [u8; 6] = [GOALS, ARRIVAL, RETURN, RECHECK, DEADLINE, DEADLINE_TOOK_OFF];

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

/// What a refusal from the offer dialog left shown
/// ([`Session::refuse_mission`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefuseShown {
    /// Whether the offer dialog was hidden: only when the mission's
    /// `RefuseText` is not -1 (`_OfferOneMission` @0xa2e70-0xa2e8e). One
    /// not hidden stays drawn, answered, under the texts the refusal
    /// queued and the after-movie, until they have all closed
    /// (`DisposeDialogNoFlash` @0xa361e).
    pub offer_hidden: bool,
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
        pickup_mode: -1,
        drop_off_mode: -1,
        scan_mask: 0,
        pay_val: 0,
        time_limit: 0,
        disp_weight: 0,
        on_accept: Script::default(),
        on_refuse: Script::default(),
        on_success: Script::default(),
        on_failure: Script::default(),
        on_abort: Script::default(),
        on_ship_done: Script::default(),
        can_abort: 1,
        comp_govt: None,
        comp_reward: 0,
        date_post_inc: 0,
        ships: MissionShip::NONE,
        aux: AuxShip::NONE,
    }
}

/// The days a `DatePostInc` of `days` passes: none for 0 or below.
fn post_days(days: i16) -> u32 {
    u32::try_from(days).unwrap_or(0)
}

impl Session {
    /// Whether the stellar `docked` at is a mission's `stop`, as
    /// `_MissionLandCargoCheck` (@0x9edfd, @0x9f01d) and
    /// `_MissionLandCheck` (@0xa1a37) ask `_StellarsAreIdentical`: by
    /// [`RuleKey::MissionStellarIdentity`]'s engine reading, also a
    /// stellar of another ID at the same place with the same name, read
    /// as [`RuleKey::StellarIdentityRecord`] says
    /// ([`StellarMarks::identical`](crate::mission::StellarMarks::identical)). No stop is never docked at.
    pub(super) fn docked_at_stop(&self, stop: Option<Stop>, docked: StellarId) -> bool {
        let rules = IdentityRules::of(&self.rules);
        stop.is_some_and(|stop| self.stellar_marks.identical(stop.stellar, docked, rules))
    }

    /// Whether the ship is docked at a mission's `stop`, as
    /// [`Session::docked_at_stop`] matches it: what tells a mission
    /// gone at its return from one aborted. Built only for the
    /// storyline walker (`fixture`), its one caller.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn docked_at_mission_stop(&self, stop: Stop) -> bool {
        self.landed
            .is_some_and(|docked| self.docked_at_stop(Some(stop), docked))
    }

    /// Where a mission's `stop` is landed at, as a player can reach it:
    /// the system and the stellar. A system is visible as the next
    /// landing's galaxy refresh leaves it (`Session::refresh_now`),
    /// from the control bits now: a bit set since the last landing shows
    /// or hides it already, as any landing would. A visible stellar (one
    /// that a system in the galaxy and visible lists) is landed at in its stored
    /// system when that is a visible lister, else in its first visible
    /// lister. A hidden one is landed at through its first variant
    /// ([`StellarMarks::variants`], by
    /// [`RuleKey::MissionStellarIdentity`] and
    /// [`RuleKey::StellarIdentityRecord`]) that is visible, in that
    /// variant's first visible lister, as `_StellarsAreIdentical`
    /// matches them. A visible lister shadowed by another at its place
    /// (`CoLocatedResolution`) counts as visible. Built only for the
    /// storyline walker (`fixture`), its one caller.
    ///
    /// # Errors
    ///
    /// [`StopRefusal::NoSystem`] when no system lists the stellar;
    /// [`StopRefusal::Hidden`] when every lister is hidden and no
    /// variant is visible.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn stop_landing(&self, stop: Stop) -> Result<(SystemId, StellarId), StopRefusal> {
        let refresh = self.refresh_now();
        let visible_listers = |stellar: StellarId| {
            crate::story::listers_of(&self.listed_stellars, stellar)
                .iter()
                .copied()
                .filter(|&system| self.galaxy.exists(system) && !refresh.hides(system))
        };
        if crate::story::listers_of(&self.listed_stellars, stop.stellar).is_empty() {
            return Err(StopRefusal::NoSystem);
        }
        if let Some(first) = visible_listers(stop.stellar).next() {
            let system = stop
                .system
                .filter(|&stored| visible_listers(stop.stellar).any(|system| system == stored))
                .unwrap_or(first);
            return Ok((system, stop.stellar));
        }
        self.stellar_marks
            .variants(stop.stellar, IdentityRules::of(&self.rules))
            .find_map(|variant| Some((visible_listers(variant).next()?, variant)))
            .ok_or(StopRefusal::Hidden)
    }

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
            (GOALS, StoryEventKind::Landed),
            (ARRIVAL, StoryEventKind::Landed),
            (RETURN, StoryEventKind::Landed),
            (RECHECK, StoryEventKind::Landed),
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
        self.register_ship_hooks(slot);
    }

    /// Frees `slot` and unregisters its hooks, clearing their once-fired
    /// entries, so a mission accepted into it starts fresh: the mission it
    /// held, if any.
    fn free_slot(&mut self, slot: u8) -> Option<ActiveMission> {
        let freed = self.pilot.missions.free(slot)?;
        for hook in HOOKS {
            self.unregister_story(slot_hook(slot, hook));
        }
        for id in super::mission_ships::ship_hooks(slot) {
            self.unregister_story(id);
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
    pub(super) fn lifecycle_record(&self, id: MissionId) -> MissionRecord {
        self.mission_record(id)
            .cloned()
            .unwrap_or_else(|| blank(id))
    }

    /// The offer of `mission` on the visit under way, or in flight the
    /// offer from a ship the offer dialog shows (see the `person_offers`
    /// module).
    pub(super) fn offer_of(&self, mission: MissionId) -> Option<Offer> {
        if self.landed.is_none() {
            let offering = self
                .offering
                .filter(|offering| offering.mission == mission)?;
            return self.ship_offer(offering.mission).cloned();
        }
        Venue::ORDER.into_iter().find_map(|venue| {
            self.offers
                .at(venue)
                .iter()
                .find(|offer| offer.mission == mission)
                .cloned()
        })
    }

    /// The first refusal of the accept checks for an offer of `tons`,
    /// drawing on `chance`: the room for them is the ship's own holds
    /// and [`Session::mission_cargo_space`].
    pub(super) fn accept_refusal(
        &self,
        tons: i64,
        chance: &mut dyn Chance,
    ) -> Result<(), AcceptRefusal> {
        let facts = AcceptFacts {
            active: &self.pilot.missions,
            cargo: self.room_for(tons),
            rules: &self.rules,
        };
        accept_checks().first_refusal(&facts, chance)
    }

    /// The room the ship has for `tons` of a mission's cargo.
    pub(super) fn room_for(&self, tons: i64) -> Room {
        Room {
            tons,
            holds: i64::from(self.stats.capacity),
            space: self.mission_cargo_space(),
        }
    }

    /// The first refusal of the accept checks for `offer`, drawing on
    /// `chance`. A cargo refusal is shown as the original's
    /// `_ShowTextDialog` shows it (@0xa1b61-0xa1be5); a full table shows
    /// nothing here.
    fn refuse_accept(
        &mut self,
        offer: &Offer,
        chance: &mut dyn Chance,
    ) -> Result<(), AcceptRefusal> {
        let refused = self.accept_refusal(offer.cargo_tons, chance);
        if let Err(refusal @ (AcceptRefusal::NoHolds | AcceptRefusal::NoSpace)) = refused {
            self.show_text(MissionTextKind::NoRoom, offer.mission, refusal.message());
        }
        refused
    }

    /// Accepts `mission`, offered on the visit under way or by a person in
    /// flight, drawing on `chance`: the accept checks, then the original's
    /// accept (`_DoMissionAccept` @0xa1b11; see the [`mission`] module's
    /// "Active missions"), and a person's offer answered (see the
    /// `person_offers` module). The slot it went into. It is the player's
    /// accept from the offer dialog or the mission computer, so it runs
    /// with the dialog open (`_missionDialog`, @0xa2e06, @0xa2fba,
    /// @0xa3bb4; see [`Session::start_mission`]).
    ///
    /// # Errors
    ///
    /// [`AcceptError::NotOffered`] for a mission not offered: in flight,
    /// any but the offer from a ship the dialog shows;
    /// [`AcceptError::Refused`] when a check refuses, and then nothing
    /// runs but an outside offer's suppression (see
    /// [`Session::next_offer`]) and, for want of room for its cargo, the
    /// refusal queued to show ([`MissionTextKind::NoRoom`]). Every slot
    /// taken shows nothing and plays nothing (`_DoMissionAccept`
    /// @0xa1b32-0xa1b42).
    ///
    /// Then the offer `dësc` is loaded again, refused or not, as the offer
    /// dialog reloads it after its accept (`_OfferOneMission` @0xa2e29,
    /// @0xa2fd7) and the mission computer after its own (`_DoMissionDialog`
    /// @0xa3bea, then @0xa3c32), and the offer dialog's after-movie is
    /// queued behind whatever the accept queued
    /// ([`Session::show_offer_after_movie`]).
    pub fn accept_mission(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Result<u8, AcceptError> {
        // A new run of docked scripts (see [`Session::take_interface_loads`]).
        self.interface_loads.clear();
        let venue = self.offer_of(mission).map(|offer| offer.venue);
        let accepted = self.in_offer_dialog(|session| session.accept_offered(mission, chance));
        if let Some(venue) = venue {
            self.reload_offer_desc(mission, chance);
            self.show_offer_after_movie(mission, venue);
        }
        accepted
    }

    /// Accepts `mission` as [`Session::accept_mission`] does, but with the
    /// offer dialog as it is: the offer dialog's accept at once, of an
    /// offer that cannot be refused and has no text, comes before the
    /// dialog is made (`_OfferOneMission` @0xa230a, before @0xa237f).
    /// An outside offer accepted is then shifted out of the list as
    /// `_OfferMissionFromPort` shifts it ([`Session::drop_offered`]); one
    /// refused is suppressed and nothing is dropped (@0xa3765-0xa377b).
    pub(super) fn accept_offered(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Result<u8, AcceptError> {
        let offer = self.offer_of(mission).ok_or(AcceptError::NotOffered)?;
        let record = self
            .mission_record(mission)
            .cloned()
            .ok_or(AcceptError::NotOffered)?;
        let at = self.outside_position(&offer);
        let accepted = match self.refuse_accept(&offer, chance) {
            Ok(()) => self
                .start_mission(&record, &offer, chance)
                .ok_or(AcceptRefusal::SlotsFull),
            Err(refusal) => Err(refusal),
        };
        if offer.venue == Venue::Outside {
            if accepted.is_ok() {
                self.drop_offered(mission, at);
            } else {
                self.suppress_offer(mission);
            }
        }
        self.end_ship_offer(accepted.is_ok());
        accepted.map_err(AcceptError::Refused)
    }

    /// Refuses `mission`, offered on the visit under way, drawing on
    /// `chance`: its `OnRefuse` runs with the offer dialog open
    /// (`_OfferOneMission` @0xa2f41), then an outside offer is shifted out
    /// of the list as `_OfferMissionFromPort` shifts it
    /// (@0xa377d-0xa37a1, as [`RuleKey::OfferShift`] reads it), so an
    /// `OnRefuse` that refilters can leave it listed, and a mission
    /// computer's offer is dropped. Nothing is forgotten. An offer from a ship is answered (see the
    /// `person_offers` module). The offer dialog's after-movie is queued
    /// last ([`Session::show_offer_after_movie`]).
    ///
    /// A `RefuseText` that is not -1 hides the dialog and is loaded
    /// (@0xa2e70-0xa2eb5), even when its `dësc` is missing, which shows
    /// nothing and empties the loaded movie. What it gives back says
    /// whether the dialog was hidden ([`RefuseShown`]).
    ///
    /// # Errors
    ///
    /// [`NotOffered`] for a mission not offered: in flight, any but the
    /// offer from a ship the dialog shows.
    pub fn refuse_mission(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Result<RefuseShown, NotOffered> {
        self.interface_loads.clear();
        let offer = self.offer_of(mission).ok_or(NotOffered)?;
        let at = self.outside_position(&offer);
        let record = self.lifecycle_record(mission);
        let offer_hidden = self
            .mission_texts(mission)
            .is_some_and(|texts| texts.refuse_named);
        // `RefuseText`, filled with no mission (index -1, @0xa2ec9).
        if !self.show_mission_text(MissionTextKind::Refuse, mission, None, chance) && offer_hidden {
            // A missing `dësc` still loads (@0xa2eb5), emptying the movie.
            let _ = self.load_description(None, None, chance);
        }
        self.in_offer_dialog(|session| session.run_script(&record.on_refuse, chance));
        if offer.venue == Venue::Outside {
            self.drop_offered(mission, at);
        } else {
            self.offers.drop_offer(mission);
        }
        self.end_ship_offer(false);
        self.show_offer_after_movie(mission, offer.venue);
        Ok(RefuseShown { offer_hidden })
    }

    /// Where `offer` is listed among the outside offers, when it is one:
    /// the index `_OfferMissionFromPort` found it at (-0x2a(%ebp),
    /// @0xa36f5), taken before the answer changes the list.
    fn outside_position(&self, offer: &Offer) -> Option<usize> {
        (offer.venue == Venue::Outside)
            .then(|| self.offers.position(Venue::Outside, offer.mission))
            .flatten()
    }

    /// Drops what answering the outside offer of `mission`, found at `at`,
    /// drops, as [`RuleKey::OfferShift`] reads it: by the engine, the
    /// entry now at `at`, in the list as the answer left it
    /// (`_OfferMissionFromPort` @0xa377d-0xa37a1), so after an accept that
    /// refiltered the list the next offer listed; by the other reading,
    /// `mission`'s own offer.
    ///
    /// It runs in the call that answers the offer, after the answer's
    /// scripts and before the venue is rearmed or a frame is drawn, as the
    /// original shifts once `_OfferOneMission` returns, so no boundary the
    /// player sees comes between the two.
    fn drop_offered(&mut self, mission: MissionId, at: Option<usize>) {
        if self.rules.source_for(RuleKey::OfferShift) == RuleSource::Bible {
            self.offers.drop_offer(mission);
        } else if let Some(at) = at {
            self.offers.drop_at(Venue::Outside, at);
        }
    }

    /// Runs `f` with the offer dialog open, as `_OfferOneMission` and
    /// `_DoMissionDialog` keep `_missionDialog` set from @0xa237f to
    /// @0xa3628 and from @0xa39a4 to @0xa40cd, then puts it back as it
    /// was, so a dialog's accept inside another's script leaves it open.
    fn in_offer_dialog<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let was = std::mem::replace(&mut self.offer_dialog_open, true);
        let done = f(self);
        self.offer_dialog_open = was;
        done
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
        self.interface_loads.clear();
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
        let penalty = abort_penalty(&record, self.govt_targets(&record), |govt| {
            self.pilot.legal_record(govt)
        });
        let in_systems = abort_penalty(&record, self.galaxy.governed(), |system| {
            self.pilot.system_record(system)
        });
        self.set_records(&penalty, &in_systems);
        self.end_mission(slot, &record, true, chance);
        Ok(())
    }

    /// Accepts `record`'s `offer` into the lowest free slot, as the
    /// original's accept does once its checks pass (`_DoMissionAccept`
    /// @0xa1c0c-0xa2196): the slot filled and its hooks registered;
    /// `OnAccept`; the cash a pay below -50000 takes; `BriefText`; the
    /// cargo loaded with `PickupMode` 0, and its `LoadCargText`; the destination
    /// reached when it has none (@0xa1fe7), or, with the offer dialog
    /// open, when it is the stellar docked at, or, in flight with an
    /// offer from a ship open, when it is the stellar
    /// [`Session::ship_offer_reaches`] gives (@0xa1fc5-0xa1ff8, the
    /// dialog read @0xa1fde); docked with the dialog open, the mission's
    /// offer dropped and the others kept only while available
    /// (@0xa1ffd-0xa2136, the dialog read @0xa200b); the auto-abort, which
    /// reads no dialog (@0xa2139-0xa2186); and its roll made 0. The
    /// dialog is open while the player's accept or refusal runs
    /// ([`Session::accept_mission`], [`Session::refuse_mission`]), so for
    /// an `S` in its `OnAccept` or `OnRefuse` too, and closed for an `S`
    /// anywhere else and for the offer dialog's accept at once
    /// ([`Session::accept_offered`]), unless [`RuleKey::StartOutsideDialog`]
    /// reads otherwise. The slot, or `None` when every slot is taken.
    fn start_mission(
        &mut self,
        record: &MissionRecord,
        offer: &Offer,
        chance: &mut dyn Chance,
    ) -> Option<u8> {
        let mut active = ActiveMission::accepted(record, offer, self.pilot.date);
        // A full table draws nothing.
        self.pilot.missions.lowest_free()?;
        // The aux countdown is drawn first, for every mission
        // (@0xa0e5f, before the appear timer @0xa0ed4).
        active.aux = AuxShips::at_accept(&record.aux, chance);
        active.ships = self.ships_at_accept(record, &active, chance);
        let slot = self.pilot.missions.fill(active)?;
        self.register_slot(slot);
        if self.landed.is_none() {
            // In flight the original's per-frame poll checks the goals
            // next (see the `mission_ships` module).
            self.goal_checks_due.push(slot);
        }
        self.save_due = true;
        self.run_mission_script(slot, &record.on_accept, chance);
        self.pilot.cash = accept_charge(offer.pay, self.pilot.cash);
        // `BriefText`, filled for the slot after `OnAccept` and the charge
        // (@0xa1e0b-0xa1f13, @0xa1e56), opening the map marked for the
        // mission accepted (@0xa1e8d-0xa1eda, @0xa1f0e).
        let map = self.briefing_map_marks(slot);
        self.show_mission_text_with_map(
            MissionTextKind::Brief,
            record.id,
            Some(slot),
            Some(map),
            chance,
        );
        self.load_at_accept(slot, record, chance);
        let in_dialog = self.offer_dialog_open
            || self.rules.source_for(RuleKey::StartOutsideDialog) == RuleSource::Bible;
        let docked = self.landed;
        let here = docked
            .or_else(|| self.ship_offer_reaches())
            .filter(|_| in_dialog);
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.reached = active
                .destination
                .is_none_or(|stop| Some(stop.stellar) == here);
        }
        if docked.is_some() && in_dialog {
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
    pub(super) fn run_mission_script(
        &mut self,
        slot: u8,
        script: &Script,
        chance: &mut dyn Chance,
    ) {
        self.script_slot = Some(slot);
        self.run_script(script, chance);
        self.script_slot = None;
    }

    /// The mission in `slot`, to change, if it is `mission`.
    pub(super) fn slot_holding(
        &mut self,
        slot: u8,
        mission: MissionId,
    ) -> Option<&mut ActiveMission> {
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
        // Each offer checked writes its cached `AvailBits` (@0xa207a).
        let checked: Vec<MissionId> = [Venue::Computer, Venue::Outside]
            .into_iter()
            .flat_map(|venue| self.offers.at(venue).iter().map(|offer| offer.mission))
            .collect();
        for mission in checked {
            self.check_avail_bits(mission);
        }
        let mut offers = std::mem::take(&mut self.offers);
        self.at_visit(stellar, |visit| offers.refilter(&self.missions, visit));
        self.offers = offers;
    }

    /// Ends the mission `record` in `slot` as `_AbortMission` (@0x99a92)
    /// does, drawing on `chance`: its special ships lose their mark and
    /// fly on as ordinary traffic, or, docked, are taken out of the
    /// system (@0x99ab5-0x99b24; see the `mission_ships` module);
    /// its `OnAbort` runs when `on_abort` says; the slot is freed and its
    /// hooks unregistered; and the offers' stellar is forgotten. Nothing
    /// when the slot holds another mission, or is ending already, so an
    /// `A` of the mission inside its own `OnAbort` does not end it twice.
    pub(super) fn end_mission(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        on_abort: bool,
        chance: &mut dyn Chance,
    ) {
        if !self.pilot.missions.holds(slot, record.id) || !self.ending.insert(slot) {
            return;
        }
        self.release_mission_ships(slot);
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
    ///
    /// Its ships go as `_AbortMission`, the last call (@0x99d0e), lets
    /// them. Docked (the original's `_portDialog` is set from @0x5fa30 to
    /// @0x60251, covering `_MissionLandCheck` and the mission computer
    /// and bar accepts), the slot's ships are taken out at once
    /// (@0x99b17-0x99b24), so none keep the freed slot's mark for a later
    /// mission in that slot to count. In flight, including the take-off's
    /// setup, where `_portDialog` is already 0 (cleared at @0x60251 before
    /// `_SetupShipsInSystem` @0x674c2), it leaves them marked: the next
    /// tick's `unmark_inactive_mission_ships` (the `_AIDispatch` sweep
    /// @0x8fbcd-0x8fc96) clears their marks, sends a `ShipBehav` 1 escort
    /// away and keeps them as traffic.
    ///
    /// That one-tick lag is deliberate. `_AbortMission` clears the marks at
    /// once (@0x99ac7), but the lag crosses no boundary the player sees,
    /// and it keeps the setup's auto-abort from re-forming the formation
    /// in the middle of `populate_as`, before `enter_escorts` and
    /// `snap_mission_escorts`.
    pub(super) fn auto_abort(&mut self, slot: u8, record: &MissionRecord, chance: &mut dyn Chance) {
        let Some(pay) = self.pilot.missions.get(slot).map(|active| active.pay) else {
            return;
        };
        if self.auto_abort_freed(slot, record, pay, chance) {
            self.close_slot(slot, record.id);
        }
    }

    /// The body of `_AutoAbortMission` (@0x99bdc) for `record`, paying
    /// `pay`, in `slot` whether or not the slot still holds it, drawing on
    /// `chance`: its `OnAbort`; its `DatePostInc` days; with `Flags`
    /// 0x0008 the fuel taken; with `Flags2` 0x0002 its pay; and docked its
    /// ships taken out. The slot is not freed: [`Session::auto_abort`]
    /// frees it after. `_EnterMoreShips`' second auto-abort of a slot its
    /// first already freed (@0x43abc-0x43ad4, under
    /// [`RuleKey::AuxAbortFallThrough`](crate::RuleKey::AuxAbortFallThrough))
    /// runs it alone, the original's `_AbortMission` (@0x99d0e) finding
    /// nothing more to clear; in flight the ships it brought keep the
    /// freed slot's mark until the next tick's sweep, as for
    /// [`Session::auto_abort`]. The docked branch cannot be reached
    /// there, as `_EnterMoreShips` runs only in flight, and is kept so the
    /// two cannot drift. Whether it ran: not while `slot` is already
    /// ending.
    pub(super) fn auto_abort_freed(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        pay: i64,
        chance: &mut dyn Chance,
    ) -> bool {
        if !self.ending.insert(slot) {
            return false;
        }
        self.run_mission_script(slot, &record.on_abort, chance);
        self.pass_days(post_days(record.date_post_inc), chance);
        if record.flags & AUTO_ABORT_FUEL != 0 {
            self.pilot.reserves.fuel.now -= AUTO_ABORT_FUEL_TAKEN;
        }
        if record.flags2 & AUTO_ABORT_PAYS != 0 {
            self.apply_pay(pay);
        }
        if self.landed.is_some() {
            self.release_mission_ships(slot);
        }
        self.ending.remove(&slot);
        true
    }

    /// Applies a mission's pay (`_ApplyMissionPay` @0x98326; see
    /// [`pay_effect`]).
    fn apply_pay(&mut self, pay: i64) {
        match pay_effect(pay) {
            PayEffect::Nothing => return,
            PayEffect::Cash(credits) => self.pilot.cash = self.pilot.cash.saturating_add(credits),
            PayEffect::Clean { of, among } => {
                let govts = cleaned(
                    self.pilot
                        .legal_records()
                        .map(|(govt, _)| (govt, Some(govt))),
                    |govt| self.pilot.legal_record(govt),
                    of,
                    among,
                    &self.govts,
                );
                let systems = cleaned(
                    self.galaxy.governed(),
                    |system| self.pilot.system_record(system),
                    of,
                    among,
                    &self.govts,
                );
                for govt in govts {
                    self.pilot.set_legal_record(govt, 0);
                }
                for system in systems {
                    self.pilot.set_system_record(system, 0);
                }
            }
            PayEffect::Fine(percent) => self.pilot.cash = fined(self.pilot.cash, percent),
        }
        self.save_due = true;
    }

    /// Every government a mission writer changes the record with, each its
    /// own target ([`success_records`]): each in the catalog, and
    /// `record`'s `CompGovt` though it is not.
    fn govt_targets(&self, record: &MissionRecord) -> Vec<(GovtId, Option<GovtId>)> {
        let ids: std::collections::BTreeSet<GovtId> =
            self.govts.ids().chain(record.comp_govt).collect();
        ids.into_iter().map(|govt| (govt, Some(govt))).collect()
    }

    /// Sets each government's record as `records` says, and each
    /// system's as `in_systems` says.
    fn set_records(&mut self, records: &[(GovtId, i16)], in_systems: &[(SystemId, i16)]) {
        for &(govt, record) in records {
            self.pilot.set_legal_record(govt, record);
            self.save_due = true;
        }
        for &(system, record) in in_systems {
            self.pilot.set_system_record(system, record);
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
            (GOALS | RECHECK, StoryEvent::Landed { .. }) => {
                self.check_ship_goals(slot, false, chance);
            }
            (ARRIVAL, StoryEvent::Landed { stellar }) => {
                self.cargo_at_landing(slot, stellar, chance);
            }
            (RETURN, StoryEvent::Landed { stellar }) => {
                self.mission_return_at(slot, stellar, chance);
            }
            (DEADLINE, StoryEvent::DayPassed { .. })
            | (DEADLINE_TOOK_OFF, StoryEvent::TookOff { .. }) => {
                self.mission_deadline(slot, chance);
            }
            _ => {}
        }
    }

    /// The failed mission in `slot`, `failed` as it was, fails at its
    /// return (`_DoMissionFailure` @0xa0285), drawing on `chance`: its
    /// `OnFailure`, the record with `CompGovt` less half `CompReward`,
    /// then the slot freed and the offers' stellar forgotten, no
    /// `OnAbort` run. A mission failed by its deadline ran `OnFailure`
    /// already: by [`RuleKey::RepeatMissionFailure`]'s engine reading it
    /// runs again; by the other, not.
    fn mission_failure_at(&mut self, slot: u8, failed: FailedBy, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let repeats = self.rules.source_for(RuleKey::RepeatMissionFailure) == RuleSource::Engine;
        if failed == FailedBy::Marked || repeats {
            self.run_mission_script(slot, &record.on_failure, chance);
        }
        let changes = failure_records(&record, self.govt_targets(&record), |govt| {
            self.pilot.legal_record(govt)
        });
        let in_systems = failure_records(&record, self.galaxy.governed(), |system| {
            self.pilot.system_record(system)
        });
        self.set_records(&changes, &in_systems);
        self.end_mission(slot, &record, false, chance);
        // `FailText` (@0xa0358-0xa03cd), filled after the slot is freed
        // (@0xa0398), so its mission tags read "[Error]".
        self.show_mission_text(MissionTextKind::Fail, record.id, Some(slot), chance);
        self.forget_offers();
    }

    /// Landing on `stellar`, the return of the mission in `slot`
    /// (`_MissionLandCheck` @0xa1a37-0xa1ace), drawing on `chance`: its
    /// destination marked reached when it is the return, or there is
    /// none; then a failed mission fails ([`Session::mission_failure_at`],
    /// @0xa1a92); else an escort goal with none of its ships destroyed
    /// is met (@0xa1a99-0xa1ab1); then, reached and its goals met, it
    /// completes (`_DoMissionSuccess` @0xa03fc): the slot freed, then its
    /// `OnSuccess`, its `DatePostInc` days, the legal records
    /// ([`success_records`]), its pay, the offers' stellar forgotten, and
    /// a re-offer due.
    fn mission_return_at(&mut self, slot: u8, stellar: StellarId, chance: &mut dyn Chance) {
        if !self.docked_at_stop(
            self.pilot
                .missions
                .get(slot)
                .and_then(|active| active.return_to),
            stellar,
        ) {
            return;
        }
        let Some(active) = self.pilot.missions.get_mut(slot) else {
            return;
        };
        // "The destination is the return" compares raw IDs
        // (`_MissionLandCheck` @0xa1a4e-0xa1a55), not by place and name.
        if active.destination.is_none_or(|stop| {
            active
                .return_to
                .is_some_and(|ret| ret.stellar == stop.stellar)
        }) {
            active.reached = true;
            self.save_due = true;
        }
        if let Some(failed) = active.failed {
            self.mission_failure_at(slot, failed, chance);
            return;
        }
        // The escort branch ([`met_at_return`]) writes the goals met
        // directly: no `ShipDone` moment.
        let mission = active.mission;
        let goal = ShipGoal::from_code(self.lifecycle_record(mission).ships.goal);
        let Some(active) = self.pilot.missions.get_mut(slot) else {
            return;
        };
        if met_at_return(goal, active.ships.destroyed) && !active.goals_met {
            active.goals_met = true;
            self.save_due = true;
        }
        if !active.reached || !active.goals_met {
            return;
        }
        let record = self.lifecycle_record(mission);
        // `CompText` (@0xa0417-0xa048c), filled before the slot is freed
        // (@0xa0457).
        self.show_mission_text(MissionTextKind::Comp, mission, Some(slot), chance);
        let Some(done) = self.free_slot(slot) else {
            return;
        };
        self.run_mission_script(slot, &record.on_success, chance);
        self.pass_days(post_days(record.date_post_inc), chance);
        let changes = success_records(&record, &self.govts, self.govt_targets(&record), |govt| {
            self.pilot.legal_record(govt)
        });
        let in_systems = success_records(&record, &self.govts, self.galaxy.governed(), |system| {
            self.pilot.system_record(system)
        });
        self.set_records(&changes, &in_systems);
        self.apply_pay(done.pay);
        self.forget_offers();
        self.reoffer_due = true;
    }

    /// The deadline check of the mission in `slot` (`_MissionObjectivesCheck`
    /// @0x9eba5-0x9ec61), drawing on `chance`: with none of its days left,
    /// not failed already, and the ship not docked, it fails at once
    /// ([`Session::quick_failure`]).
    fn mission_deadline(&mut self, slot: u8, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        if active.failed.is_some() || active.days_left != Some(0) || self.landed.is_some() {
            return;
        }
        let record = self.lifecycle_record(active.mission);
        // The beep, `_beepSnd[3]` (`snd ` 153, @0x9ebd9-0x9ec02), and the
        // comm message, `STR#` 2002 #285, before the failure, unless
        // `Flags` has 0x0400 (@0x9ebd0-0x9ec53).
        if record.flags & HIDDEN == 0 {
            self.sounds.push(crate::SimSound::Refused);
            self.say(SimMessage::DeadlineMissed);
        }
        self.quick_failure(slot, &record, chance);
    }

    /// Fails the mission `record` in `slot` at once, drawing on `chance`,
    /// as `_QuickMissionFailure` (@0x99d1b) does for a missed deadline, its
    /// cargo jettisoned or a scan finding it: its `OnFailure`, then marked
    /// failed ([`FailedBy::Quick`]); an abortable one ends, its slot freed,
    /// no `OnAbort` run; and the offers' stellar is forgotten either way.
    /// An unabortable one stays active and failed until its return. No
    /// record changes.
    pub(super) fn quick_failure(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        chance: &mut dyn Chance,
    ) {
        self.run_mission_script(slot, &record.on_failure, chance);
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.failed = Some(FailedBy::Quick);
        }
        if record.can_abort != 0 {
            self.end_mission(slot, record, false, chance);
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
                active.failed.get_or_insert(FailedBy::Marked);
            }
        }
    }

    /// `S`: accepts `mission` at once, drawing on `chance`
    /// (`_EvalSetExp` @0x153b9-0x15418): resolved from the stellar docked
    /// at, or in flight from the first stellar the player's system lists
    /// (128 with none, `_RandomizeOneMission` @0x153ff), then accepted as
    /// the player's accept is once its checks pass (`_DoMissionAccept`
    /// @0x15407, its checks @0xa1b32-0xa1be5, a cargo refusal shown),
    /// without asking whether it is available, with the offer dialog as it
    /// is: open in an `OnAccept` or `OnRefuse` the player's answer runs,
    /// closed anywhere else (see [`Session::start_mission`]). A mission
    /// active already takes another slot, unless
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
        let origin = self.landed.unwrap_or_else(|| self.first_listed_stellar());
        let today = self.pilot.date;
        let offer = self.in_world(|world| {
            mission::resolve(
                &record,
                Venue::of(record.avail_loc),
                Some(origin),
                world,
                today,
                chance,
            )
        });
        if self.refuse_accept(&offer, chance).is_err() {
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

    /// Draws the offers' roll table afresh on `chance`, as `_ResetPlayer`
    /// does whatever its argument (@0x1d795-0x1d7ab, @0x1d93a-0x1d956)
    /// when a new pilot starts (`_DoNewPilot` @0x18dd4) or a pilot is
    /// opened from a save (@0x76502, @0x7673e). The table is never saved,
    /// so without this a pilot in space would read an undrawn table, all
    /// 0, until its first landing or arrival. The offers stay; the
    /// stellar they were rolled at is already none on both paths.
    pub(super) fn draw_offer_rolls(&mut self, chance: &mut dyn Chance) {
        self.offers.redraw(chance);
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
                cached_avail_bits: None,
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
    /// less the goods held that the trader escorts' holds do not take and
    /// the mission cargo aboard, which rides in the ship alone
    /// (@0xd0c7-0xd0eb); below 0 when it holds more. With no escort's
    /// holds this is the holds less `_TotalCargo` (@0xd0ed).
    pub(super) fn mission_cargo_space(&self) -> i64 {
        let own = i64::from(self.stats.capacity);
        let escorts = i64::from(self.capacity()) - own;
        let spilled = (self.pilot.goods_held() - escorts).max(0);
        own - (spilled + self.pilot.missions.cargo_aboard())
    }

    /// `f` of the galaxy a mission's stellars are chosen from.
    pub(super) fn in_world<T>(&self, f: impl FnOnce(&World) -> T) -> T {
        let player = self.pilot.system;
        let stellars = stellars_of(&self.galaxy, &self.listed_stellars, player);
        let placed = placements(&self.galaxy, &self.listed_stellars, player);
        f(&World {
            stellars: &stellars,
            placed: &placed,
            map: self.galaxy.map(),
            govts: &self.govts,
            draw: self.rules.source_for(RuleKey::MissionStellarDraw),
            distance: self.rules.source_for(RuleKey::MissionStellarDistance),
            ally_index: self.rules.source_for(RuleKey::MissionAllyIndex),
        })
    }

    /// `f` of the visit to `stellar`, one of the stellars shown: `None`
    /// when it is not shown.
    pub(super) fn at_visit<T>(&self, stellar: StellarId, f: impl FnOnce(&Visit) -> T) -> Option<T> {
        let docked = *self.sites.iter().find(|site| site.id == stellar)?;
        Some(self.visit_with(Some(docked), self.shown, f))
    }

    /// `f` of the flight in the player's system, with no stellar docked:
    /// what a ship's offer reads.
    fn in_flight<T>(&self, f: impl FnOnce(&Visit) -> T) -> T {
        self.visit_with(None, self.pilot.system, f)
    }

    /// `f` of a visit to `docked` (none in flight) in `system`, by the
    /// pilot as it is now.
    fn visit_with<T>(
        &self,
        docked: Option<LandingSite>,
        system: SystemId,
        f: impl FnOnce(&Visit) -> T,
    ) -> T {
        let player = self.mission_player(system);
        let active = self.pilot.missions.missions();
        self.in_world(|&world| {
            f(&Visit {
                docked,
                system,
                world,
                player,
                active: &active,
                gate: self.gate(),
                rules: &self.rules,
                today: self.pilot.date,
            })
        })
    }

    /// The player as availability reads it, its legal record the one in
    /// `system`, as [`Session::record_in`] reads it.
    fn mission_player(&self, system: SystemId) -> Player {
        let ship = self.ship_record(self.pilot.ship);
        Player {
            record: self.record_in(system, None),
            rating: self.pilot.combat_rating,
            ship: self.pilot.ship,
            inherent_ai: ship.map_or(0, |ship| ship.inherent_ai),
            inherent_govt: ship.and_then(|ship| ship.inherent_govt),
            combat_govt: ship.and_then(|ship| ship.combat_govt),
            contributed: self.contributed(),
            fuel: self.pilot.reserves.fuel.now,
            free_cargo: self.mission_cargo_space(),
            cash: self.pilot.cash,
        }
    }

    /// Caches every mission's `AvailBits` verdict now, as the original's
    /// refresh writes `mïsn`+0x16 for all (`_PropagateMissionBitEffects`
    /// @0x99887-0x998b2): at a landing, a jump's arrival before its days,
    /// a new pilot's start and a load, with the galaxy's refresh. Only the
    /// hail quote filter reads the cache (see
    /// [`Session::quoted_mission_available`]).
    pub(crate) fn propagate_avail_bits(&mut self) {
        let gate = self.gate();
        let verdicts: BTreeMap<MissionId, bool> = self
            .missions
            .iter()
            .map(|record| (record.id, gate.allows(&record.avail_bits)))
            .collect();
        self.avail_cache = verdicts;
    }

    /// Caches `mission`'s `AvailBits` verdict now, as a check of it that
    /// writes the cache does (`_IsMissionAvailable(m, 1)` @0x9b4af-0x9b4d3:
    /// an accept, a port offer, the mission computer, a hail or a board).
    pub(super) fn check_avail_bits(&mut self, mission: MissionId) {
        if let Some(record) = self.mission_record(mission) {
            let holds = self.gate().allows(&record.avail_bits);
            self.avail_cache.insert(mission, holds);
        }
    }

    /// Whether `mission` is available from a ship in flight, the visit's
    /// rolls read and nothing drawn (`_IsMissionAvailable(m, 1)` with
    /// `_offeringFromShip` set; see the [`mission`] module's "From a
    /// ship"): never while docked, nor for a mission the session has no
    /// record of.
    pub(crate) fn ship_mission_available(&self, mission: MissionId) -> bool {
        self.ship_mission_available_reading(mission, None)
    }

    /// Whether `mission` is available from a ship in flight as the hail
    /// quote filter checks it (`_IsMissionAvailable(m, 0)`, `_HandleShip`
    /// @0x33e1d): its `AvailBits` as [`RuleKey::CachedAvailBits`] reads
    /// them, by the engine the verdict cached at the last refresh or
    /// check of it ([`Session::propagate_avail_bits`]), by the other
    /// reading tested now; the rest as [`Session::ship_mission_available`].
    pub(crate) fn quoted_mission_available(&self, mission: MissionId) -> bool {
        let cached = match self.rules.source_for(RuleKey::CachedAvailBits) {
            RuleSource::Engine => self.avail_cache.get(&mission).copied(),
            RuleSource::Bible => None,
        };
        self.ship_mission_available_reading(mission, cached)
    }

    /// [`Session::ship_mission_available`], the `AvailBits` read as
    /// `cached` when given.
    fn ship_mission_available_reading(&self, mission: MissionId, cached: Option<bool>) -> bool {
        let Some(record) = self
            .mission_record(mission)
            .filter(|_| self.landed.is_none())
        else {
            return false;
        };
        self.in_flight(|visit| {
            let facts = AvailFacts {
                visit,
                mission: record,
                venue: Venue::Ship,
                rolls: self.offers.table(),
                cached_avail_bits: cached,
            };
            mission::availability()
                .first_refusal(&facts, &mut NeverFires)
                .is_ok()
        })
    }

    /// The visit's roll table.
    #[cfg(test)]
    pub(crate) fn offer_rolls(&self) -> &mission::RollTable {
        self.offers.table()
    }

    /// What `text`'s tags read for `mission`, in `slot` (none for an
    /// offer), or no mission, the crossed rank tags reading `rank_govts`:
    /// the pilot, its ship, no person, and the session's cash, date and
    /// rules.
    fn tag_context(
        &self,
        mission: Option<MissionValues>,
        slot: Option<u8>,
        rank_govts: RankTagGovts,
    ) -> TagContext<'_> {
        TagContext {
            pilot: &self.pilot.name,
            ship_type: self.ship_name(self.pilot.ship).unwrap_or_default(),
            ship_name: self.pilot.ship_name().unwrap_or_default(),
            person: match self.rules.source_for(RuleKey::OfferingShipName) {
                RuleSource::Engine => self.offering_ship_name(),
                RuleSource::Bible => self.offering_person(),
            },
            ranks: self.rank_names(),
            mission: mission.map(|values| MissionTags {
                values,
                cash: self.pilot.cash,
                today: self.pilot.date,
                date_affixes: &self.date_affixes,
                pay_millions: self.rules.source_for(RuleKey::PayMillions),
                slot,
            }),
            rank_govts,
            leet: self.leet,
            rules: TagRules::from_rulebook(&self.rules),
        }
    }

    /// Takes in `raw`, a text from `source`, as the original does before
    /// it fills it: its conditional text is resolved for the pilot as it
    /// is now, and the remembered rank governments become those `source`
    /// leaves ([`TextSource::parse`], `_ParseTextDescription`); then, in
    /// the `l33t` mode, a `dësc`'s letters are drawn on `chance`
    /// ([`load_leet`](text_tags::load_leet), @0x7194f, after the parse at
    /// @0x71936). What it gives back is the text to fill.
    ///
    /// The `&self` texts below ([`Session::offer_text`],
    /// [`Session::mission_text`], [`Session::plain_text`]) read the
    /// remembered governments and change nothing; only the paths that
    /// show a text take it in, so the governments settle as each text is
    /// shown, as the original's `_LoadDescription` does.
    pub(super) fn take_in(
        &mut self,
        source: TextSource,
        raw: &str,
        chance: &mut dyn Chance,
    ) -> String {
        let (parsed, govts) = source.parse(self, self.rank_govts, raw);
        self.rank_govts = govts;
        if self.leet && source == TextSource::Desc {
            text_tags::load_leet(&parsed, chance)
        } else {
            parsed
        }
    }

    /// Loads `raw`, a `dësc` that may be missing, naming `movie`, as
    /// `_LoadDescription` (@0x717fe) loads one: taken in when there is one
    /// ([`Session::take_in`]), its movie the last loaded
    /// ([`Session::loaded_after_movie`]; `MovieFile` and `Flags`
    /// @0x71895-0x718de), and otherwise empty. A missing one fails its
    /// `GetResource` (@0x71825) and only empties the text and the
    /// `MovieFile` (@0x71a5e-0x71a81), never reaching the reset
    /// (@0x71926-0x71930), the parse (@0x71936) or the `l33t` letters, so
    /// the governments the last text left are kept. The `Flags` it leaves
    /// stale are never read, as every movie check tests the name first.
    pub fn load_description(
        &mut self,
        raw: Option<&str>,
        movie: Option<&DescMovie>,
        chance: &mut dyn Chance,
    ) -> String {
        self.loaded_movie = raw.and(movie).cloned();
        raw.map(|raw| self.take_in(TextSource::Desc, raw, chance))
            .unwrap_or_default()
    }

    /// The after-movie a `STR#` text plays once it closes, as
    /// `_ShowTextDialog`'s after-check reads the globals the last
    /// `_LoadDescription` left (@0x5b40b-0x5b44f), as
    /// [`RuleKey::DescMovieGlobals`] reads it: by the engine, the last
    /// `dësc` loaded's movie when its `Flags` 0x0001 is set; by the other
    /// reading none. A `STR#` text is shown with `_ShowTextDialog`'s third
    /// argument 0, which gates off the before-check alone (@0x5a76c).
    #[must_use]
    pub fn loaded_after_movie(&self) -> Option<DescMovie> {
        match self.rules.source_for(RuleKey::DescMovieGlobals) {
            RuleSource::Engine => self.loaded_movie.clone().filter(|movie| movie.after),
            RuleSource::Bible => None,
        }
    }

    /// `raw`, a `dësc` that may be missing, as loading it would give it
    /// before any `l33t` letters are drawn, and the governments it would
    /// leave, as [`Session::load_description`] leaves them, changing
    /// nothing: a missing one empty, keeping the remembered governments.
    pub(super) fn peek_desc(&self, raw: Option<&str>) -> (String, RankTagGovts) {
        raw.map_or((String::new(), self.rank_govts), |raw| {
            TextSource::Desc.parse(self, self.rank_govts, raw)
        })
    }

    /// `text` filled for `mission`'s offer, its crossed rank tags reading
    /// `rank_govts`: none when it is not offered, or in flight.
    pub(super) fn offer_text_reading(
        &self,
        mission: MissionId,
        text: &str,
        rank_govts: RankTagGovts,
    ) -> Option<String> {
        let offer = self.offer_of(mission)?;
        let context = self.tag_context(Some(MissionValues::offered(&offer)), None, rank_govts);
        Some(text_tags::expand(text, &context, self))
    }

    /// `text` filled for the mission in `slot`, its crossed rank tags
    /// reading `rank_govts`.
    pub(super) fn mission_text_reading(
        &self,
        slot: u8,
        text: &str,
        rank_govts: RankTagGovts,
    ) -> String {
        let values = self
            .pilot
            .missions
            .get(slot)
            .map(MissionValues::active)
            .unwrap_or_default();
        let context = self.tag_context(Some(values), Some(slot), rank_govts);
        text_tags::expand(text, &context, self)
    }

    /// `text` with its tags filled for `mission`'s offer on the visit
    /// under way (`_MungeBriefing` with `offer` set; see
    /// [`text_tags`](crate::text_tags)), reading the remembered rank
    /// governments: none when it is not offered, or in flight.
    #[must_use]
    pub fn offer_text(&self, mission: MissionId, text: &str) -> Option<String> {
        self.offer_text_reading(mission, text, self.rank_govts)
    }

    /// `text` with its tags filled for the mission in `slot`
    /// (`_MungeBriefing` with a slot; see [`text_tags`](crate::text_tags)),
    /// reading the remembered rank governments: a free slot's mission
    /// tags read [`UNSET`](text_tags::UNSET).
    #[must_use]
    pub fn mission_text(&self, slot: u8, text: &str) -> String {
        self.mission_text_reading(slot, text, self.rank_govts)
    }

    /// `text` with its tags filled with no mission, as the original's
    /// `RefuseText`, comm dialog and hail quotes are (`_MungeBriefing`
    /// with index -1; see [`text_tags`](crate::text_tags)), reading the
    /// remembered rank governments: the mission tags read as
    /// [`RuleKey::MissionlessTags`] says.
    #[must_use]
    pub fn plain_text(&self, text: &str) -> String {
        let context = self.tag_context(None, None, self.rank_govts);
        text_tags::expand(text, &context, self)
    }
}

/// Where a text the session fills comes from, which decides the rank
/// governments its crossed tags read
/// ([`RuleKey::CrossedRankTags`]; see [`RankTagGovts`]). Each is parsed
/// by `_ParseTextDescription` ([`parse_text_description`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextSource {
    /// A `dësc` loaded as `_LoadDescription` loads one: an offer's,
    /// `BriefText`, `CompText`, `FailText`, `RefuseText`, the cargo texts,
    /// `ShipDoneText`, `QuickBrief`, and the `dësc`s the screens show. The
    /// governments are reset (@0x71926-0x71930), then it is parsed
    /// (@0x71936).
    Desc,
    /// A hail quote, parsed with no reset (`_ShowPersonHailQuote`
    /// @0x445ee).
    Quote,
    /// A ship's specific advice, parsed with no reset (`_LoadAdvice`
    /// @0x919ca).
    Advice,
}

impl TextSource {
    /// `raw` from this source parsed for `session`'s pilot, its rules
    /// those `session` holds, and the governments it leaves,
    /// `remembered` before it.
    pub(super) fn parse(
        self,
        session: &Session,
        remembered: RankTagGovts,
        raw: &str,
    ) -> (String, RankTagGovts) {
        let rules = ParseRules::from_rulebook(&session.rules);
        let before = match self {
            Self::Desc => RankTagGovts::NONE,
            Self::Quote | Self::Advice => remembered,
        };
        parse_text_description(raw, session, rules, before)
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

    fn stellar_system(&self, stellar: StellarId) -> Option<SystemId> {
        self.galaxy
            .whereabouts(self.listers(stellar), self.pilot.system)
            .named(self.galaxy.map().places())
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
    use crate::catalog::{LandingSite, MissionRecord, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::geometry::Vec2;
    use crate::mission::{ROLLED, Stop};
    use crate::pilot::Pilot;
    use crate::testkit::{
        Draws, FakePilotCatalog, arriving, catalog, govt, jump, land_now, mission, planet, swapped,
        with_variants,
    };

    /// [`catalog`] with stellar 150 in system 132, two jumps from 130, 131
    /// linked back to 130, and missions 130 (to a random inhabited
    /// stellar), 131 (to 140) and 132 (`AvailRandom` 10), each at the
    /// mission computer.
    fn offering() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.systems.push(SystemId(132));
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
            table_then(&[3]),
            "the table, then 130's stellar: 129, 140 or 150, at any distance"
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
    fn each_reading_of_the_mission_stellar_draw_reaches_an_offers_destination() {
        let catalog = offering();
        let bible =
            Rulebook::default().with_override(RuleKey::MissionStellarDraw, RuleSource::Bible);
        for (rules, to, asked) in [(Rulebook::default(), (129, 130), 3), (bible, (150, 132), 1)] {
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            let mut chance = arriving(&[0]);
            let _ = session.settle_script(&catalog, &mut chance);
            assert_eq!(chance.asked, table_then(&[asked]), "{to:?}");
            assert_eq!(
                session.mission_offers(Venue::Computer)[0].destination,
                Some(Stop {
                    stellar: StellarId(to.0),
                    system: Some(SystemId(to.1)),
                }),
                "by the engine 129, in the origin's own system; by the other reading the distant 150"
            );
        }
    }

    #[test]
    fn each_reading_of_the_mission_stellar_distance_reaches_the_world() {
        let mut catalog = offering();
        // 133 at 130's place, holding 160: distant only by the other
        // reading, and owned by 133 only while it stands for itself.
        catalog.star_map[3].position = Vec2::ZERO;
        catalog.systems.push(SystemId(133));
        catalog
            .sites
            .push((SystemId(133), vec![planet(160, 0.0, 0.0)]));
        let rules = |distance, colocated| {
            Rulebook::default()
                .with_override(RuleKey::MissionStellarDraw, RuleSource::Bible)
                .with_override(RuleKey::MissionStellarDistance, distance)
                .with_override(RuleKey::ColocatedResolution, colocated)
        };
        for (distance, colocated, asked) in [
            (RuleSource::Engine, RuleSource::Bible, 1),
            (RuleSource::Bible, RuleSource::Bible, 2),
            (RuleSource::Engine, RuleSource::Engine, 1),
            (RuleSource::Bible, RuleSource::Engine, 1),
        ] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(rules(distance, colocated));
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            let mut chance = arriving(&[0]);
            let _ = session.settle_script(&catalog, &mut chance);
            assert_eq!(
                chance.asked,
                table_then(&[asked]),
                "{distance:?}, {colocated:?}: 150, and by the other reading 160 \
                 too while 133, shadowed by 130 by the engine, owns it"
            );
        }
    }

    #[test]
    fn each_reading_of_the_mission_ally_index_reaches_the_world() {
        let mut catalog = offering();
        // The pirates' 131 in 132, two jumps out: not allied with the
        // Federation, but system 131, at its index, is the Federation's.
        catalog.govts = vec![govt(FED.0), govt(PIRATES.0)];
        catalog.star_map[1].govt = Some(FED);
        catalog.sites[2].1.push(LandingSite {
            govt: Some(PIRATES),
            ..planet(131, 0.0, 0.0)
        });
        let mut ally = mission(130);
        ally.travel_stel = 15000;
        catalog.missions = vec![ally];
        let bible = Rulebook::default().with_override(RuleKey::MissionAllyIndex, RuleSource::Bible);
        for (rules, offered) in [(Rulebook::default(), true), (bible, false)] {
            let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            let _ = session.settle_script(&catalog, &mut arriving(&[0]));
            let destinations: Vec<_> = session
                .mission_offers(Venue::Computer)
                .iter()
                .map(|offer| offer.destination)
                .collect();
            let expected = if offered {
                vec![Some(Stop {
                    stellar: StellarId(131),
                    system: Some(SystemId(132)),
                })]
            } else {
                Vec::new()
            };
            assert_eq!(
                destinations, expected,
                "by the engine the pirates' 131 by the Federation's system at its index; \
                 by the other reading no ally, so no offer"
            );
        }
    }

    #[test]
    fn landing_again_on_the_same_stellar_keeps_the_offers_and_draws_nothing() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        let offers = session.mission_offers(Venue::Computer).to_vec();
        assert_eq!(ids(&offers), [130, 131, 132], "every roll 1");
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
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
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
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
        assert_eq!(chance.asked, table_then(&[3]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn a_jumps_arrival_draws_the_table_afresh_before_the_system_is_populated() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        assert_eq!(
            session.take_off(&catalog, &mut NeverFires),
            Some(StellarId(128))
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.player.position = Vec2::ZERO;
        let mut chance = crate::testkit::Scripted::rolling(&[41]);
        assert_eq!(
            crate::testkit::jump_with(&mut session, &catalog, 131, &mut chance),
            Some(SystemId(131))
        );
        assert_eq!(chance.sides_asked, vec![100; ROLLED], "the table alone");
        assert_eq!(session.offer_rolls().roll(MissionId(128)), Some(42));
        assert_eq!(session.offer_rolls().roll(MissionId(130)), Some(1));
        assert_eq!(session.offers.rolled_at(), None);
        assert_eq!(
            ids(session.mission_offers(Venue::Computer)),
            [130, 131, 132],
            "kept until the next landing"
        );
    }

    #[test]
    fn a_relocation_draws_no_table() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.offers.set_roll(MissionId(130), 77);
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("relocates");
        assert_eq!(session.offer_rolls().roll(MissionId(130)), Some(77));
    }

    #[test]
    fn after_the_forget_hook_landing_again_on_the_same_stellar_rolls_afresh() {
        let catalog = offering();
        let mut session = landed(&catalog, &mut NeverFires);
        session.take_off(&catalog, &mut NeverFires);
        session.forget_offers();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[3]));
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
    fn a_landed_out_of_range_n_empties_the_offers_by_the_engine_alone() {
        let catalog = offering();
        for (reading, emptied) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut session = landed(&catalog, &mut NeverFires).with_rules(
                crate::Rulebook::default().with_override(crate::RuleKey::OutOfRangeMove, reading),
            );
            assert!(!session.mission_offers(Venue::Computer).is_empty());
            session.run_set(&SetExpr::parse("N9999").expect("parses"), &mut NeverFires);
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.system(), SystemId(130), "{reading:?}");
            assert_eq!(
                session.mission_offers(Venue::Computer).is_empty(),
                emptied,
                "{reading:?}"
            );
            assert_eq!(
                session.offers.rolled_at(),
                Some(StellarId(128)),
                "{reading:?}"
            );
        }
    }

    #[test]
    fn by_the_engine_a_landed_move_to_a_missing_system_only_empties_the_offers() {
        let catalog = offering();
        for (reading, emptied) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            for text in ["M1500", "N1500"] {
                let mut session = landed(&catalog, &mut NeverFires).with_rules(
                    crate::Rulebook::default()
                        .with_override(crate::RuleKey::MissingSystemMove, reading),
                );
                assert!(!session.mission_offers(Venue::Computer).is_empty());
                session.run_set(&SetExpr::parse(text).expect("parses"), &mut NeverFires);
                let _ = session.settle_script(&catalog, &mut NeverFires);
                assert_eq!(session.system(), SystemId(130), "{reading:?} {text}");
                assert_eq!(
                    session.mission_offers(Venue::Computer).is_empty(),
                    emptied,
                    "{reading:?} {text}"
                );
                assert_eq!(
                    session.offers.rolled_at(),
                    Some(StellarId(128)),
                    "{reading:?} {text}"
                );
            }
        }
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
    fn a_pilot_flown_from_a_save_has_its_offers_rolled_at_its_first_landing() {
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        let mut chance = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut chance);
        assert!(chance.asked.is_empty(), "nothing rolled on resuming");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut chance);
        assert_eq!(chance.asked, table_then(&[3]));
        assert_eq!(ids(session.mission_offers(Venue::Computer)), [130, 131]);
    }

    #[test]
    fn flying_a_pilot_saved_at_a_stellar_raises_its_exploration_and_no_landing() {
        use crate::story::StoryEvent;
        let catalog = offering();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.stellar = Some(StellarId(128));
        let resumed = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            resumed.story_raised(),
            [StoryEvent::Explored {
                system: SystemId(130)
            }]
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
        assert!(ROLL < slot(i16::MIN));
        assert!(ROLL < crate::session::mission_ships::ship_hook(0, 0));
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
    fn mission_cargo_aboard_rides_in_the_ship_alone() {
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
        let mut session = landed(&catalog, &mut NeverFires);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 4));
        assert_eq!(session.mission_cargo_space(), 16, "20 own, 4 aboard");
        session.pilot.escorts = vec![Escort {
            ship: crate::catalog::ShipId(129),
            reserves: crate::reserves::Reserves::default(),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }];
        session.pilot.cargo.insert(Good::Commodity(0), 25);
        assert_eq!(
            session.mission_cargo_space(),
            6,
            "10 of the 25 tons spill past the escort's 15; the 4 aboard stay"
        );
        session.pilot.cargo.insert(Good::Commodity(0), 10);
        assert_eq!(
            session.mission_cargo_space(),
            16,
            "the escort takes all 10, but none of the mission's"
        );
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
        crate::session::tests::with_record(&mut session, GovtId(128), 3);
        crate::session::tests::with_record(&mut session, GovtId(129), -5);
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

    #[test]
    fn by_the_engine_avail_record_reads_the_systems_record() {
        use crate::catalog::GovtId;
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        let mut catalog = offering();
        catalog.star_map[0].govt = Some(GovtId(128));
        catalog.missions = vec![MissionRecord {
            avail_record: 3,
            ..mission(140)
        }];
        let mut session = landed(&catalog, &mut NeverFires);
        session.pilot.set_legal_record(GovtId(128), -5);
        session.pilot.set_system_record(SystemId(130), 5);
        assert_eq!(
            session.offer_refusals(MissionId(140)),
            Some(vec![]),
            "the system's 5"
        );
        let session = session.with_rules(
            Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible),
        );
        let refused = session.offer_refusals(MissionId(140)).expect("docked");
        assert_eq!(
            refused
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["AvailRecord"],
            "by the Bible, govt 128's -5"
        );
    }

    #[test]
    fn by_the_bible_in_a_hidden_system_avail_record_reads_the_markups_record() {
        use crate::catalog::GovtId;
        use crate::rulebook::{RuleKey, RuleSource, Rulebook};
        let mut catalog = offering();
        catalog.star_map[0].govt = Some(GovtId(128));
        catalog.missions = vec![MissionRecord {
            avail_record: 3,
            ..mission(140)
        }];
        let session = landed(&catalog, &mut NeverFires);
        let mut session = session.with_rules(
            Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible),
        );
        session.pilot.set_legal_record(GovtId(128), 5);
        // Hidden under the player directly: a refresh would move the
        // player out (`leave_hidden`), but every reader must still agree.
        let mut overlay = session.galaxy.overlay().clone();
        assert!(overlay.set_visible(SystemId(130), Some(false)));
        session.galaxy.set_overlay(overlay);
        assert!(!session.galaxy().visible(SystemId(130)), "hidden");
        assert_eq!(session.system_record(), 5, "govt 128's, though hidden");
        assert_eq!(
            session.offer_refusals(MissionId(140)),
            Some(vec![]),
            "the same 5"
        );
    }

    #[test]
    fn in_an_independent_system_avail_record_reads_a_record_of_0() {
        use crate::catalog::GovtId;
        let mut catalog = offering();
        // System 130 is independent; the stellar docked at, 128, govt 129's.
        catalog.star_map[0].govt = None;
        catalog.sites[0].1[0].govt = Some(GovtId(129));
        catalog.missions = vec![
            MissionRecord {
                avail_record: 1,
                ..mission(140)
            },
            MissionRecord {
                avail_record: -1,
                ..mission(141)
            },
            MissionRecord {
                avail_record: 0,
                ..mission(142)
            },
        ];
        let mut session = landed(&catalog, &mut NeverFires);
        for record in [-50, 50] {
            for govt in 128..=131 {
                session.pilot.set_legal_record(GovtId(govt), record);
            }
            let refused = |id| {
                session
                    .offer_refusals(MissionId(id))
                    .expect("docked")
                    .into_iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>()
            };
            assert_eq!(refused(140), ["AvailRecord"], "1, every record {record}");
            assert_eq!(refused(141), ["AvailRecord"], "-1, every record {record}");
            assert_eq!(refused(142), [] as [&str; 0], "0, every record {record}");
        }
    }

    // The active missions' lifecycle.

    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::catalog::{GovtId, GovtRecord};
    use crate::control::{Bit, SetOp};
    use crate::date::GameDate;
    use crate::mission::screens::{NO_HOLDS_TO_ACCEPT, NO_SPACE_TO_ACCEPT};
    use crate::mission::{AbortRefusal, ActiveMissions};
    use crate::rulebook::Rulebook;
    use crate::session::control::SetOpObserver;
    use crate::session::mission_screens::MissionText;

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
            session
                .take_off(catalog, &mut NeverFires)
                .expect("takes off");
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

    // The galaxy refreshes at a landing.

    /// Whether 132 and 133 were visible each time a script ran an operator.
    #[derive(Debug, Default)]
    struct Shown(RefCell<Vec<(bool, bool)>>);

    impl SetOpObserver for Shown {
        fn saw(&self, _op: &SetOp, session: &Session) {
            let galaxy = session.galaxy();
            self.0
                .borrow_mut()
                .push((galaxy.visible(SystemId(132)), galaxy.visible(SystemId(133))));
        }
    }

    /// [`lifecycle`] of `missions`, with 132 shown by b50 and 133 by b51.
    fn gated(missions: Vec<MissionRecord>) -> FakePilotCatalog {
        let mut catalog = lifecycle(missions);
        catalog.visibility = [(132, "b50"), (133, "b51")]
            .into_iter()
            .map(|(system, test)| crate::catalog::SystemVisibility {
                id: SystemId(system),
                visibility: crate::control::Test::parse(test),
            })
            .collect();
        catalog
    }

    /// A session of `catalog`, its operators seen by a [`Shown`], docked
    /// at 128 with mission 140 accepted.
    fn gated_session(catalog: &FakePilotCatalog) -> (Session, Rc<Shown>) {
        let shown = Rc::new(Shown::default());
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_set_op_observer(shown.clone());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        accept(&mut session, 140);
        (session, shown)
    }

    #[test]
    fn a_landing_refreshes_the_galaxy_as_its_offers_roll_and_after_its_land_check() {
        let catalog = gated(vec![MissionRecord {
            on_success: Script::parse("b51 X130"),
            ..travel(140, -1, 129)
        }]);
        let (mut session, shown) = gated_session(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        run(&mut session, "b50");
        settle(&mut session, &catalog);
        assert!(!session.galaxy().visible(SystemId(132)), "not in flight");
        land_on(&mut session, &catalog, 129);
        assert_eq!(held(&session), [], "completed");
        assert_eq!(
            *shown.0.borrow(),
            [(true, false)],
            "refreshed as the offers rolled at 129, before the success hook"
        );
        assert!(
            session.galaxy().visible(SystemId(133)),
            "refreshed again after the land check"
        );
    }

    #[test]
    fn landing_again_where_the_offers_were_rolled_refreshes_only_after_the_land_check() {
        let catalog = gated(vec![MissionRecord {
            on_success: Script::parse("X130"),
            ..back_to_128(140)
        }]);
        let (mut session, shown) = gated_session(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        run(&mut session, "b50");
        settle(&mut session, &catalog);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [], "completed");
        assert_eq!(
            *shown.0.borrow(),
            [(false, false)],
            "no roll at 128 again, so no refresh before the land check"
        );
        assert!(session.galaxy().visible(SystemId(132)), "after it");
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
                goals_met: false,
                aboard: false,
                failed: None,
                ships: crate::mission::SpecialShips::default(),
                // NeverFires draws the countdown's highest, 69.
                aux: AuxShips {
                    countdown: 139,
                    ..AuxShips::default()
                },
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

    /// Mission `id`, to 129, carrying `tons` of cargo `cargo_type`; its
    /// `OnAccept` sets bit 20, and it takes 100 credits at accept.
    fn freight(id: i16, cargo_type: i16, tons: i16) -> MissionRecord {
        MissionRecord {
            cargo_type,
            cargo_qty: tons,
            pay_val: -50_100,
            on_accept: Script::parse("b20"),
            ..to_129(id)
        }
    }

    /// A session of `catalog` with 1000 credits and `held` tons of food,
    /// landed on 128 and settled, its texts taken.
    fn holding(catalog: &FakePilotCatalog, held: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.cash = 1000;
        session
            .pilot
            .cargo
            .insert(crate::market::Good::Commodity(0), held);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        let _ = session.take_texts_only();
        session
    }

    fn refusal_text(mission: i16, text: &str) -> MissionText {
        MissionText {
            kind: MissionTextKind::NoRoom,
            mission: MissionId(mission),
            text: text.to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }
    }

    #[test]
    fn an_accept_without_the_holds_for_its_cargo_or_passengers_is_refused_shown_and_runs_nothing() {
        for cargo_type in [0, 6] {
            let catalog = lifecycle(vec![freight(140, cargo_type, 21)]);
            let mut session = holding(&catalog, 0);
            let offers = session.mission_offers(Venue::Computer).to_vec();
            assert_eq!(
                session.accept_mission(MissionId(140), &mut NeverFires),
                Err(AcceptError::Refused(AcceptRefusal::NoHolds)),
                "20 holds, 21 tons of {cargo_type}"
            );
            assert_eq!(
                session.take_texts_only(),
                [refusal_text(140, NO_HOLDS_TO_ACCEPT)]
            );
            assert!(!session.control_bit(bit(20)), "no OnAccept");
            assert_eq!(held(&session), []);
            assert_eq!(session.pilot().cash(), 1000, "nothing taken");
            assert_eq!(session.mission_offers(Venue::Computer), offers, "kept");
        }
    }

    #[test]
    fn an_accept_without_the_free_space_for_its_cargo_is_refused_and_one_with_exactly_it_is_not() {
        let catalog = lifecycle(vec![freight(140, 0, 11)]);
        let mut session = holding(&catalog, 10);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::NoSpace)),
            "10 free, 11 tons"
        );
        assert_eq!(
            session.take_texts_only(),
            [refusal_text(140, NO_SPACE_TO_ACCEPT)]
        );
        assert!(!session.control_bit(bit(20)));
        let catalog = lifecycle(vec![freight(140, 0, 10)]);
        let mut session = holding(&catalog, 10);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        assert!(session.control_bit(bit(20)));
        assert_eq!(session.pilot().cash(), 900);
    }

    #[test]
    fn an_outside_offer_refused_for_want_of_room_is_suppressed() {
        let catalog = lifecycle(vec![MissionRecord {
            avail_loc: crate::mission::screens::BAR,
            ..freight(140, 0, 21)
        }]);
        let mut session = holding(&catalog, 0);
        session.enter_offer_venue(crate::mission::screens::BAR);
        assert!(
            session
                .next_offer(crate::mission::screens::BAR, &mut NeverFires)
                .is_some()
        );
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::NoHolds))
        );
        assert_eq!(
            session.next_offer(crate::mission::screens::BAR, &mut NeverFires),
            None
        );
    }

    #[test]
    fn s_of_a_mission_without_room_for_its_cargo_is_refused_the_same_way() {
        let catalog = lifecycle(vec![freight(140, 0, 21), freight(141, 0, 5)]);
        let mut session = holding(&catalog, 0);
        run(&mut session, "S140");
        assert_eq!(held(&session), []);
        assert!(!session.control_bit(bit(20)));
        assert_eq!(
            session.take_texts_only(),
            [refusal_text(140, NO_HOLDS_TO_ACCEPT)]
        );
        run(&mut session, "S141");
        assert_eq!(held(&session), [(0, 141)], "room for 5");
    }

    #[test]
    fn a_full_table_shows_no_refusal() {
        let catalog = lifecycle(vec![freight(140, 0, 5)]);
        let mut session = holding(&catalog, 0);
        let offer = session.mission_offers(Venue::Computer)[0].clone();
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(141), &offer, session.date());
            session.pilot.missions.fill(active);
        }
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
        );
        run(&mut session, "S140");
        assert_eq!(session.take_texts_only(), []);
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
            session
                .refuse_mission(MissionId(140), &mut NeverFires)
                .map(|_| ()),
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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

    /// [`lifecycle`] with 130 and 131 the Federation's, 132 the pirates'
    /// and 133 the Civvies', every system
    /// [`hideable`](crate::testkit::hideable).
    fn governed_lifecycle(missions: Vec<MissionRecord>) -> FakePilotCatalog {
        let mut catalog = lifecycle(missions);
        for (system, govt) in catalog
            .star_map
            .iter_mut()
            .zip([FED, FED, PIRATES, CIVVIES])
        {
            system.govt = Some(govt);
        }
        crate::testkit::hideable(&mut catalog);
        catalog
    }

    /// `session`'s records in systems 130 to 133.
    fn system_records(session: &Session) -> Vec<i16> {
        (130..=133)
            .map(|system| session.pilot().system_record(SystemId(system)))
            .collect()
    }

    /// `session` with its records in systems 130 to 133 set to `records`,
    /// and 133 hidden.
    fn on_system_records(session: &mut Session, catalog: &FakePilotCatalog, records: [i16; 4]) {
        for (system, record) in (130..=133).zip(records) {
            session.pilot.set_system_record(SystemId(system), record);
        }
        assert!(crate::session::tests::hide(session, catalog, &[133]));
    }

    #[test]
    fn a_mission_success_changes_the_records_in_each_system_and_with_each_government() {
        let catalog = governed_lifecycle(vec![MissionRecord {
            comp_govt: Some(FED),
            comp_reward: 4,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(FED, 10);
        on_system_records(&mut session, &catalog, [10, -2, 10, 10]);
        accept(&mut session, 140);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        land_on(&mut session, &catalog, 129);
        assert_eq!(held(&session), []);
        assert!(!session.galaxy().visible(SystemId(133)), "still hidden");
        assert_eq!(
            system_records(&session),
            [14, 2, 8, 12],
            "the Federation's two, its enemy's, and its ally's though hidden"
        );
        assert_eq!(
            session.pilot().legal_record(FED),
            14,
            "and the government's"
        );
    }

    #[test]
    fn a_mission_failure_and_abort_change_the_records_in_each_system_of_its_government() {
        let catalog = governed_lifecycle(vec![
            MissionRecord {
                comp_govt: Some(FED),
                comp_reward: 5,
                ..back_to_128(140)
            },
            MissionRecord {
                comp_govt: Some(FED),
                comp_reward: 1,
                flags: 0x0040,
                ..to_129(141)
            },
        ]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(FED, 10);
        on_system_records(&mut session, &catalog, [10, -2, 10, 10]);
        accept(&mut session, 140);
        run(&mut session, "F140");
        land_on(&mut session, &catalog, 128);
        assert_eq!(system_records(&session), [8, -4, 10, 10], "half of 5");
        assert_eq!(session.pilot().legal_record(FED), 8);
        let slot = accept(&mut session, 141);
        assert_eq!(session.abort_mission(slot, &mut NeverFires), Ok(()));
        assert_eq!(system_records(&session), [3, -9, 10, 10], "five times 1");
        assert_eq!(session.pilot().legal_record(FED), 3);
    }

    #[test]
    fn a_comp_govt_the_catalog_does_not_hold_still_has_its_record_changed() {
        const STRAY: GovtId = GovtId(140);
        let stray = |id, reward, mission: fn(i16) -> MissionRecord| MissionRecord {
            comp_govt: Some(STRAY),
            comp_reward: reward,
            flags: 0x0040,
            ..mission(id)
        };
        let catalog = lifecycle(vec![
            stray(140, 4, to_129),
            stray(141, 6, back_to_128),
            stray(142, 1, to_129),
        ]);
        assert!(catalog.govts.iter().all(|govt| govt.id != STRAY));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(STRAY, 10);
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 129);
        assert_eq!(held(&session), []);
        assert_eq!(session.pilot().legal_record(STRAY), 14, "success: +4");
        land_on(&mut session, &catalog, 128);
        accept(&mut session, 141);
        run(&mut session, "F141");
        land_on(&mut session, &catalog, 128);
        assert_eq!(
            session.pilot().legal_record(STRAY),
            11,
            "failure: half of 6"
        );
        let slot = accept(&mut session, 142);
        assert_eq!(session.abort_mission(slot, &mut NeverFires), Ok(()));
        assert_eq!(
            session.pilot().legal_record(STRAY),
            6,
            "abort: five times 1"
        );
    }

    #[test]
    fn a_cleaning_pay_cleans_the_records_in_each_system_of_its_governments() {
        let catalog = governed_lifecycle(vec![MissionRecord {
            pay_val: -20_128,
            ..back_to_128(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session.pilot.set_legal_record(FED, -5);
        // 130 clean enough to land in.
        on_system_records(&mut session, &catalog, [4, -3, -5, -6]);
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), []);
        assert_eq!(
            system_records(&session),
            [4, 0, -5, 0],
            "the Federation's and its ally's though hidden, the positive one \
             kept, the enemy's untouched"
        );
        assert_eq!(session.pilot().legal_record(FED), 0);
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

    /// Mission `id`, offered at 128, to `destination` (none for -1),
    /// completed at `ret`.
    fn travel(id: i16, destination: i16, ret: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: destination,
            return_stel: ret,
            ..mission(id)
        }
    }

    #[test]
    fn a_mission_completes_at_another_stellar_of_its_returns_place_and_name() {
        let catalog = with_variants(lifecycle(vec![travel(140, -1, 129)]));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 151);
        assert_eq!(held(&session), [], "151 is 129's variant");
    }

    #[test]
    fn its_destination_is_reached_at_another_stellar_of_its_place_and_name() {
        let catalog = with_variants(lifecycle(vec![travel(140, 129, 128)]));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 151);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [], "reached at 151, completed at 128");
    }

    #[test]
    fn a_failed_mission_fails_at_another_stellar_of_its_returns_place_and_name() {
        let catalog = with_variants(lifecycle(vec![MissionRecord {
            on_failure: Script::parse("X200"),
            ..travel(140, -1, 129)
        }]));
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        run(&mut session, "F140");
        land_on(&mut session, &catalog, 151);
        assert_eq!(marks.taken(), [200]);
        assert_eq!(held(&session), []);
    }

    #[test]
    fn a_stellar_of_another_name_or_place_does_not_complete_it() {
        let catalog = with_variants(lifecycle(vec![travel(140, -1, 129)]));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 152);
        assert_eq!(held(&session), [(0, 140)], "another name");
        land_on(&mut session, &catalog, 153);
        assert_eq!(held(&session), [(0, 140)], "another place");
        land_on(&mut session, &catalog, 150);
        assert_eq!(held(&session), [(0, 140)], "128's variant");
    }

    #[test]
    fn by_the_bible_reading_the_variant_does_not_complete_it() {
        let catalog = with_variants(lifecycle(vec![travel(140, -1, 129), travel(141, 129, 128)]));
        let rules =
            Rulebook::default().with_override(RuleKey::MissionStellarIdentity, RuleSource::Bible);
        let (mut session, _) = watched(&catalog, rules);
        accept(&mut session, 140);
        accept(&mut session, 141);
        land_on(&mut session, &catalog, 151);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [(0, 140), (1, 141)]);
    }

    /// `with_variants` with 153 unmarked, a `spöb` that didn't load, and
    /// 152 marked unnamed at 0,0: what a zeroed record reads as.
    fn with_a_missing_return() -> FakePilotCatalog {
        let mut catalog = with_variants(lifecycle(vec![travel(140, -1, 153)]));
        catalog.marks.retain(|mark| mark.id != StellarId(153));
        catalog.unloaded.push(StellarId(153));
        let unnamed = catalog
            .marks
            .iter_mut()
            .find(|mark| mark.id == StellarId(152))
            .expect("152 marked");
        unnamed.place = (0, 0);
        unnamed.name.clear();
        catalog
    }

    #[test]
    fn a_missing_return_is_reached_at_an_unnamed_stellar_at_0_0_by_the_record() {
        let catalog = with_a_missing_return();
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 152);
        assert_eq!(held(&session), [], "153 reads as a zeroed record");
    }

    #[test]
    fn a_missing_return_is_reached_only_at_itself_by_the_spobs_as_written() {
        let catalog = with_a_missing_return();
        let rules =
            Rulebook::default().with_override(RuleKey::StellarIdentityRecord, RuleSource::Bible);
        let (mut session, _) = watched(&catalog, rules);
        accept(&mut session, 140);
        land_on(&mut session, &catalog, 152);
        assert_eq!(held(&session), [(0, 140)]);
        land_on(&mut session, &catalog, 153);
        assert_eq!(held(&session), []);
    }

    /// The accept compares the docked stellar by its raw ID (@0xa1fd8).
    #[test]
    fn an_accept_at_a_variant_of_its_destination_does_not_reach_it() {
        let catalog = with_variants(lifecycle(vec![MissionRecord {
            avail_stel: 151,
            ..travel(140, 129, 128)
        }]));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        land_on(&mut session, &catalog, 151);
        accept(&mut session, 140);
        assert!(!session.pilot().missions().get(0).expect("held").reached);
        land_on(&mut session, &catalog, 128);
        assert_eq!(held(&session), [(0, 140)]);
    }

    /// The success check counts the destination reached when it is the
    /// return by raw ID (@0xa1a4e-0xa1a55), so a variant does not count.
    #[test]
    fn a_destination_identical_to_the_return_is_not_reached_by_the_success_check_alone() {
        let catalog = with_variants(lifecycle(vec![travel(140, 129, 151)]));
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        assert!(session.unregister_story(slot_hook(0, ARRIVAL)));
        land_on(&mut session, &catalog, 151);
        assert_eq!(held(&session), [(0, 140)]);
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
                ships: crate::catalog::MissionShip {
                    count: 1,
                    goal: -1,
                    ..crate::catalog::MissionShip::NONE
                },
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
            ships: crate::catalog::MissionShip {
                count: 1,
                goal: 0,
                ..crate::catalog::MissionShip::NONE
            },
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert!(session.control_bit(bit(40)));
        assert_eq!(held(&session), []);
        assert_eq!(session.offers.rolled_at(), None, "forgotten");
    }

    #[test]
    fn a_deadline_missed_in_flight_says_so() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(session.take_messages(), [], "docked: nothing yet");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(
            session.take_messages(),
            [took_off(), SimMessage::DeadlineMissed],
            "after the greeting"
        );
    }

    #[test]
    fn a_deadline_missed_in_flight_beeps() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        let _ = session.take_sounds();
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        let refused = session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == crate::sound::SimSound::Refused)
            .count();
        assert_eq!(refused, 1, "_beepSnd[3], snd 153");
    }

    #[test]
    fn a_hidden_missions_missed_deadline_does_not_beep() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            flags: crate::mission::screens::HIDDEN,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(held(&session), [], "failed all the same");
        assert!(
            !session
                .take_sounds()
                .contains(&crate::sound::SimSound::Refused)
        );
    }

    /// The take-off greeting from planet 128, its lead rolled 0.
    fn took_off() -> SimMessage {
        SimMessage::TookOff {
            stellar: StellarId(128),
            lead: 0,
        }
    }

    #[test]
    fn a_hidden_missions_missed_deadline_says_nothing() {
        let catalog = lifecycle(vec![MissionRecord {
            time_limit: 1,
            flags: crate::mission::screens::HIDDEN,
            ..to_129(140)
        }]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        session.pass_days(1, &mut NeverFires);
        settle(&mut session, &catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        assert_eq!(held(&session), [], "failed all the same");
        assert_eq!(session.take_messages(), [took_off()], "the greeting alone");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        let active = session.pilot().missions().get(0).expect("stays").clone();
        assert_eq!(active.failed, Some(FailedBy::Quick));
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        let active = session.pilot().missions().get(0).expect("stays");
        assert_eq!(active.failed, Some(FailedBy::Quick));
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
                Some(FailedBy::Marked)
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        assert_eq!(
            ids(session.mission_offers(Venue::Computer)),
            [140],
            "kept: no offer dialog is open (@0xa200b)"
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
    #[allow(clippy::float_cmp)] // untouched
    fn an_s_in_flight_of_an_auto_aborting_mission_holds_its_slot_and_runs_only_on_accept() {
        let catalog = lifecycle(vec![auto_aborting()]);
        let marks = Rc::new(Marks::default());
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_set_op_observer(marks.clone());
        settle(&mut session, &catalog);
        let (today, fuel) = (session.date(), session.pilot().reserves().fuel.now);
        run(&mut session, "S140");
        assert_eq!(held(&session), [(0, 140)]);
        assert_eq!(marks.taken(), [200], "OnAccept, no OnAbort");
        assert_eq!(session.date(), today);
        assert_eq!(session.pilot().reserves().fuel.now, fuel);
    }

    // `S` and the offer dialog (`_missionDialog`, @0xa1fde, @0xa200b).

    /// [`lifecycle`] with 128 an outfitter selling outfit 300, free, whose
    /// `OnPurchase` is `S141`, and missions offered at 128's computer:
    /// 140 (to 129), available while `!b5`; 141, to 128 itself, whose
    /// `OnAccept` sets b5; and 142 (to 129), whose `OnAccept` and
    /// `OnRefuse` are `S141`.
    fn dialog_catalog() -> FakePilotCatalog {
        let mut catalog = lifecycle(vec![
            MissionRecord {
                avail_bits: crate::control::Test::parse("!b5"),
                ..to_129(140)
            },
            MissionRecord {
                avail_stel: 128,
                travel_stel: 128,
                on_accept: Script::parse("b5"),
                ..mission(141)
            },
            MissionRecord {
                on_accept: Script::parse("S141"),
                on_refuse: Script::parse("S141"),
                ..to_129(142)
            },
        ]);
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id == StellarId(128)) {
                site.flags |= crate::landing::StellarFlags::OUTFITTER;
                site.tech_level = 5;
            }
        }
        catalog.outfits = vec![crate::catalog::OutfitRecord {
            cost: 0,
            mass: 0,
            on_purchase: Script::parse("S141"),
            ..crate::testkit::outfit(300, &[])
        }];
        catalog
    }

    fn reached(session: &Session, slot: u8) -> bool {
        session.pilot().missions().get(slot).expect("held").reached
    }

    fn computer(session: &Session) -> Vec<i16> {
        ids(session.mission_offers(Venue::Computer))
    }

    fn buy_s141(session: &mut Session) {
        let order = crate::session::tests::buy(crate::catalog::OutfitId(300));
        session.outfit(order, &mut NeverFires).expect("bought");
    }

    #[test]
    fn an_s_from_an_outfits_on_purchase_leaves_its_destination_unreached_and_the_offers_as_they_were()
     {
        let catalog = dialog_catalog();
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(computer(&session), [140, 141, 142]);
        buy_s141(&mut session);
        assert_eq!(held(&session), [(0, 141)]);
        assert!(session.control_bit(bit(5)), "its OnAccept ran");
        assert!(!reached(&session, 0), "no dialog open (@0xa1fde)");
        assert_eq!(
            computer(&session),
            [140, 141, 142],
            "none dropped (@0xa200b)"
        );
    }

    #[test]
    fn by_the_other_reading_an_s_from_an_outfits_on_purchase_reaches_and_refilters() {
        let catalog = dialog_catalog();
        let rules =
            Rulebook::default().with_override(RuleKey::StartOutsideDialog, RuleSource::Bible);
        let (mut session, _) = watched(&catalog, rules);
        buy_s141(&mut session);
        assert!(reached(&session, 0));
        assert_eq!(computer(&session), [142], "140 by b5, 141 its own");
    }

    #[test]
    fn an_s_in_an_accepts_on_accept_reaches_its_destination_and_refilters() {
        let catalog = dialog_catalog();
        let (mut session, _) = watched(&catalog, Rulebook::default());
        assert_eq!(accept(&mut session, 142), 0);
        assert_eq!(held(&session), [(0, 142), (1, 141)]);
        assert!(reached(&session, 1), "in the dialog");
        assert_eq!(computer(&session), Vec::<i16>::new());
    }

    #[test]
    fn an_s_in_a_refusals_on_refuse_reaches_its_destination_and_refilters() {
        let catalog = dialog_catalog();
        let (mut session, _) = watched(&catalog, Rulebook::default());
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("offered");
        assert_eq!(held(&session), [(0, 141)]);
        assert!(
            reached(&session, 0),
            "OnRefuse runs in the dialog (@0xa2f41)"
        );
        assert_eq!(computer(&session), Vec::<i16>::new(), "140 by b5");
    }

    #[test]
    fn after_the_dialog_an_s_is_outside_it_again() {
        let catalog = dialog_catalog();
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        buy_s141(&mut session);
        assert_eq!(held(&session), [(0, 140), (1, 141)]);
        assert!(!reached(&session, 1));
        assert_eq!(computer(&session), [141, 142]);
    }

    /// A destination gives a return (`ReturnStel` -1 returns there), so
    /// only a mission with neither auto-aborts, and it is reached with no
    /// dialog open too (@0xa1fe7).
    #[test]
    fn a_docked_s_outside_the_dialog_still_auto_aborts_a_mission_with_no_destination() {
        let catalog = lifecycle(vec![auto_aborting()]);
        let (mut session, marks) = watched(&catalog, Rulebook::default());
        run(&mut session, "S140");
        assert_eq!(held(&session), []);
        assert_eq!(marks.taken(), [200, 201]);
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
            Some(FailedBy::Marked)
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        run(&mut session, "F140");
        assert_eq!(
            session.pilot().missions().get(0).and_then(|m| m.failed),
            Some(FailedBy::Quick)
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
    fn a_landing_checks_each_slots_goals_cargo_return_and_recheck_before_the_next_slot() {
        let catalog = lifecycle(vec![to_129(140), to_129(141)]);
        let (mut session, _) = watched(&catalog, Rulebook::default());
        accept(&mut session, 140);
        accept(&mut session, 141);
        let mut expected = vec![ROLL];
        for slot in 0..2 {
            expected.extend([GOALS, ARRIVAL, RETURN, RECHECK].map(|hook| slot_hook(slot, hook)));
        }
        expected.push(REOFFER);
        assert_eq!(
            session.story_matching(&crate::story::StoryEvent::Landed {
                stellar: StellarId(128)
            }),
            expected
        );
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
            for hook in [GOALS, ARRIVAL, RETURN, RECHECK] {
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
        assert!(ROLL < slot_hook(0, GOALS));
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
                25049 => &["<PN>: paid <PAY>."],
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
    fn a_missions_dsy_follows_its_stellars_holder_after_the_galaxy_changes() {
        let mut catalog = briefed();
        // 140 is listed by 132 as well as 131.
        catalog
            .sites
            .iter_mut()
            .find(|(system, _)| *system == SystemId(132))
            .expect("132's stellars")
            .1
            .push(planet(140, 0.0, 0.0));
        let mut session = stocks(&catalog);
        let slot = accept(&mut session, 131);
        assert_eq!(
            session.mission_text(slot, "<DST> in <DSY>"),
            "Mars in Alpha"
        );
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        }));
        assert_eq!(
            session
                .pilot()
                .missions()
                .get(slot)
                .and_then(|active| active.destination),
            Some(Stop {
                stellar: StellarId(140),
                system: Some(SystemId(131)),
            }),
            "stored as offered"
        );
        assert_eq!(
            session.mission_text(slot, "<DST> in <DSY>"),
            "Mars in Vega",
            "132 owns 140 now (@0x9c8de)"
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
    fn a_plain_text_fills_the_player_tags_and_its_mission_tags_by_the_missionless_rule() {
        let catalog = briefed();
        let session = stocks(&catalog);
        assert_eq!(
            session.plain_text("<PN> of the <PSN>, a <PST>, <PRK>: <DST> <PAY> <OSN>"),
            "Stock of the Kestrel, a Ship 128, captain: [Error] [Error] [Error]",
            "by the engine"
        );
        let other = session.with_rules(
            Rulebook::default().with_override(RuleKey::MissionlessTags, RuleSource::Bible),
        );
        assert_eq!(
            other.plain_text("<PN> of the <PSN>, a <PST>, <PRK>: <DST> <PAY> <OSN>"),
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
    fn a_slots_pay_of_0_reads_the_slots_byte_offset_by_the_engine_and_0_by_the_other() {
        let mut catalog = briefed();
        catalog.missions[0].pay_val = 0;
        let mut session = stocks(&catalog);
        let slot = accept(&mut session, 130);
        let active = session.pilot.missions.free(slot).expect("accepted");
        assert!(session.pilot.missions.put(1, active));
        assert_eq!(session.mission_text(1, "<PAY>"), "2,284", "1 x 0x8ec");
        assert_eq!(
            session.offer_text(MissionId(130), "<PAY>"),
            None,
            "accepted, so no longer offered"
        );
        let other = session.with_rules(
            Rulebook::default().with_override(RuleKey::PaySlotOffset, RuleSource::Bible),
        );
        assert_eq!(other.mission_text(1, "<PAY>"), "0");
    }

    #[test]
    fn an_offers_pay_of_0_reads_0() {
        let mut catalog = briefed();
        catalog.missions[0].pay_val = 0;
        let session = stocks(&catalog);
        assert_eq!(
            session.offer_text(MissionId(130), "<PAY>").as_deref(),
            Some("0")
        );
    }

    /// [`Told`]'s strings, with `'STR '` 9102 patching cargo type 2's name.
    #[derive(Debug)]
    struct Patched;

    impl CommCatalog for Patched {
        fn string_list(&self, id: i16) -> Vec<String> {
            Told.string_list(id)
        }

        fn plugin_string(&self, id: i16) -> crate::catalog::StringPatch {
            match id {
                9102 => crate::catalog::StringPatch::Text("*gems".to_owned()),
                _ => crate::catalog::StringPatch::Absent,
            }
        }
    }

    #[test]
    fn ct_reads_a_cargo_name_a_str_9100_patch_gives_its_star_dropped() {
        let catalog = briefed();
        let session = stocks(&catalog).with_strings(Rc::new(Patched));
        assert_eq!(
            session
                .offer_text(MissionId(130), "<CQ> tons of <CT>")
                .as_deref(),
            Some("12 tons of gems"),
            "_lcCargoName #3 from 'STR ' 9102 (@0x71ba4-0x71bf1)"
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

    /// The message the next settling leaves on: in flight, the
    /// settling's; landed, the take-off's, the leave held to it.
    fn left_on(session: &mut Session, catalog: &FakePilotCatalog) -> Option<String> {
        let settled = session.settle_script(catalog, &mut NeverFires);
        if session.take_off(catalog, &mut NeverFires).is_some() {
            return session.take_leave_message();
        }
        settled.message
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
            on_abort: Script::parse("Q25049"),
            pay_val: 1234,
            ..auto_aborting()
        }]);
        let mut session = stocks(&catalog);
        accept(&mut session, 140);
        assert_eq!(held(&session), [], "aborted at once");
        assert_eq!(
            left_on(&mut session, &catalog).as_deref(),
            Some("Stock: paid 1,234."),
            "its slot's, still held as OnAbort runs (@0x99ba1, @0x99c3e)"
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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

    // Offers from a ship.

    /// [`catalog`] with mission 130 offered from a ship (`AvailLoc` 2,
    /// `AvailRandom` 40), and mission 131 at the mission computer.
    fn from_ships() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.missions = vec![
            MissionRecord {
                avail_loc: 2,
                avail_random: 40,
                ..mission(130)
            },
            mission(131),
        ];
        catalog
    }

    #[test]
    fn in_flight_a_ship_offers_its_mission_reading_the_visits_roll() {
        let catalog = from_ships();
        let mut session = Session::start(&catalog).expect("starts");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(session.ship_mission_available(MissionId(130)), "undrawn: 0");
        session.offers.set_roll(MissionId(130), 41);
        assert!(!session.ship_mission_available(MissionId(130)));
        session.offers.set_roll(MissionId(130), 40);
        assert!(session.ship_mission_available(MissionId(130)));
        assert!(
            !session.ship_mission_available(MissionId(131)),
            "the computer's"
        );
        assert!(
            !session.ship_mission_available(MissionId(132)),
            "no such mïsn"
        );
    }

    /// A chance whose every draw is `draw` (capped below `n`), so a
    /// table drawn on it holds `draw + 1` at every index.
    struct Always(u16);

    impl Chance for Always {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            u32::from(self.0).min(n.saturating_sub(1))
        }
    }

    /// Whether every one of the table's rolls is `roll`.
    fn every_roll_is(session: &Session, roll: u16) -> bool {
        (0..ROLLED)
            .map(|index| MissionId(128 + i16::try_from(index).expect("an index")))
            .all(|id| session.offer_rolls().roll(id) == Some(roll))
    }

    #[test]
    fn a_new_pilot_begun_in_space_has_its_offer_rolls_drawn() {
        let catalog = from_ships();
        for (draw, available) in [(40, false), (39, true)] {
            let mut session = Session::start(&catalog).expect("starts");
            assert_eq!(session.landed(), None, "in space");
            session.begin(&catalog, &mut Always(draw));
            assert!(every_roll_is(&session, draw + 1));
            assert_eq!(
                session.ship_mission_available(MissionId(130)),
                available,
                "AvailRandom 40 against a roll of {}",
                draw + 1
            );
        }
    }

    #[test]
    fn a_pilot_resumed_in_space_has_its_offer_rolls_drawn() {
        let catalog = from_ships();
        for (draw, available) in [(40, false), (39, true)] {
            let mut session = Session::start(&catalog).expect("starts");
            assert_eq!(session.landed(), None, "in space");
            session.resume(&mut Always(draw));
            assert!(every_roll_is(&session, draw + 1));
            assert!(
                (session.player().heading - f32::from(draw)).abs() < f32::EPSILON,
                "still faced on load"
            );
            assert_eq!(
                session.ship_mission_available(MissionId(130)),
                available,
                "AvailRandom 40 against a roll of {}",
                draw + 1
            );
        }
    }

    #[test]
    fn docked_no_ship_offers_its_mission() {
        let catalog = from_ships();
        let session = landed(&catalog, &mut NeverFires);
        assert!(!session.ship_mission_available(MissionId(130)));
    }

    #[test]
    fn in_flight_a_ships_offer_reads_the_player_and_its_system() {
        let mut catalog = from_ships();
        catalog.missions[0].avail_stel = 129;
        catalog.missions[0].flags = 0x0008;
        let mut session = Session::start(&catalog).expect("starts");
        assert!(
            session.ship_mission_available(MissionId(130)),
            "AvailStel unread"
        );
        session.pilot.reserves.fuel.now = 99.0;
        assert!(!session.ship_mission_available(MissionId(130)), "fuel");
    }

    // Where a stop is landed at.

    fn stop(stellar: i16, system: Option<i16>) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: system.map(SystemId),
        }
    }

    fn landing(system: i16, stellar: i16) -> (SystemId, StellarId) {
        (SystemId(system), StellarId(stellar))
    }

    /// A session of `catalog` landed on 128, with b50 set, its galaxy
    /// refreshed.
    fn swapped_session(catalog: &FakePilotCatalog) -> Session {
        let mut session = landed(catalog, &mut NeverFires);
        run(&mut session, "b50");
        session.refresh_galaxy_quietly(catalog);
        session
    }

    #[test]
    fn a_visible_stop_is_landed_at_in_its_stored_system() {
        let mut galaxy = swapped(catalog(), true);
        galaxy.sites[0].1.push(planet(141, 0.0, 3000.0));
        let session = landed(&galaxy, &mut NeverFires);
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Ok(landing(131, 140))
        );
        assert_eq!(session.stop_landing(stop(140, None)), Ok(landing(131, 140)));
        assert_eq!(
            session.stop_landing(stop(141, Some(130))),
            Ok(landing(130, 141)),
            "visible though 134, which also lists it, is not"
        );
    }

    #[test]
    fn a_stop_stored_in_a_hidden_system_is_landed_at_in_the_first_visible_lister() {
        let mut galaxy = swapped(catalog(), true);
        galaxy.sites[0].1.push(planet(141, 0.0, 3000.0));
        let session = landed(&galaxy, &mut NeverFires);
        assert_eq!(
            session.stop_landing(stop(141, Some(134))),
            Ok(landing(130, 141))
        );
        assert_eq!(session.stop_landing(stop(141, None)), Ok(landing(130, 141)));
    }

    #[test]
    fn a_stop_stored_in_its_second_visible_lister_is_landed_at_there() {
        let mut galaxy = catalog();
        galaxy.sites[0].1.push(planet(142, 0.0, 3000.0));
        galaxy.sites[1].1.push(planet(142, 0.0, 3000.0));
        let session = landed(&galaxy, &mut NeverFires);
        assert_eq!(
            session.stop_landing(stop(142, Some(131))),
            Ok(landing(131, 142)),
            "not 130, the first lister"
        );
        assert_eq!(session.stop_landing(stop(142, None)), Ok(landing(130, 142)));
    }

    #[test]
    fn a_stop_whose_first_lister_is_hidden_is_landed_at_in_the_second() {
        let mut galaxy = swapped(catalog(), false);
        galaxy.sites[1].1.push(planet(142, 0.0, 3000.0));
        galaxy.sites[2].1.push(planet(142, 0.0, 3000.0));
        let session = swapped_session(&galaxy);
        assert!(!session.galaxy().visible(SystemId(131)));
        assert_eq!(session.stop_landing(stop(142, None)), Ok(landing(134, 142)));
        assert_eq!(
            session.stop_landing(stop(142, Some(131))),
            Ok(landing(134, 142))
        );
    }

    #[test]
    fn a_hidden_stop_is_landed_at_its_visible_variant() {
        let galaxy = swapped(catalog(), true);
        let session = swapped_session(&galaxy);
        assert!(!session.galaxy().visible(SystemId(131)));
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Ok(landing(134, 141))
        );
        assert_eq!(
            session.stop_landing(stop(141, Some(134))),
            Ok(landing(134, 141))
        );
    }

    #[test]
    fn a_bit_set_since_the_last_landing_shows_and_hides_already() {
        let galaxy = swapped(catalog(), true);
        let mut session = landed(&galaxy, &mut NeverFires);
        run(&mut session, "b50");
        assert!(session.galaxy().visible(SystemId(131)), "not yet refreshed");
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Ok(landing(134, 141))
        );
    }

    #[test]
    fn a_hidden_stop_with_no_visible_variant_or_no_lister_is_refused() {
        let galaxy = swapped(catalog(), false);
        let session = swapped_session(&galaxy);
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Err(StopRefusal::Hidden)
        );
        assert_eq!(
            session.stop_landing(stop(150, None)),
            Err(StopRefusal::NoSystem)
        );
        let mut marked = swapped(catalog(), true);
        marked.visibility[1].visibility = Test::parse("b50 & b51");
        let session = swapped_session(&marked);
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Err(StopRefusal::Hidden),
            "its one variant, 141, hidden too"
        );
    }

    #[test]
    fn by_the_bible_a_hidden_stop_has_no_variant() {
        let galaxy = swapped(catalog(), true);
        let mut session = landed(&galaxy, &mut NeverFires);
        session = session.with_rules(
            Rulebook::default().with_override(RuleKey::MissionStellarIdentity, RuleSource::Bible),
        );
        run(&mut session, "b50");
        session.refresh_galaxy_quietly(&galaxy);
        assert_eq!(
            session.stop_landing(stop(140, Some(131))),
            Err(StopRefusal::Hidden)
        );
    }
}
