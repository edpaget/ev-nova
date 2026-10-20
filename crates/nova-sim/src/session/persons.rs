//! Persons in flight: the session's side of [`person`](crate::person).
//!
//! **Appearing.** A system is populated with its persons as the session's
//! [`PersonRules`] say ([`Session::with_person_rules`], Nova's by
//! default), the pilot's persons gone for good kept away and each
//! person's `ActiveOn` tested through the session's
//! [`ControlBits`](crate::control::ControlBits): a person drawn at random
//! both for the player as the last galaxy refresh read them and now, a
//! system's person slot now alone (see the `spawn_availability` module).
//! A person NPC is named by
//! its record, with its subtitle ([`Session::npc_name`],
//! [`Session::npc_subtitle`]).
//!
//! **Hailed.** A person's comm quote (`STR#` 7100 #`CommQuote`, when it
//! is above 0) is said as the rule set reads
//! [`RuleKey::CommQuote`](crate::RuleKey::CommQuote): by the
//! engine, as Greetings' answer from a friendly person, in place of its
//! advice; otherwise as the hail's opening line, whatever its attitude.
//! A person's `HailPict` shows in the comm dialog
//! ([`HailView::portrait`](crate::HailView::portrait)). A person whose
//! `LinkMission` is available from a ship offers it when hailed, or with
//! `Flags` 0x0200 when boarded, in place of the comm dialog or the
//! plunder (see the `person_offers` module).
//!
//! **Hail quotes.** Each tick in flight ([`Session::tick_quotes`]) a
//! person may say its hail quote (a plug-in's `'STR '` 4999 +
//! `HailQuote`, else `STR#` 7101 #`HailQuote`, its tags read; the
//! `'STR '` of any open file in its place, `_ShowPersonHailQuote`
//! @0x445b8-0x445e1), on the trigger its `Flags` give: when it begins to
//! attack the player, now and then, and so on (see
//! [`person`](crate::person)); the flight shows each in its message line
//! ([`Session::take_quotes`]), with a beep ([`SimSound::HailQuote`]).
//! A person escorting the player says none, nor does one gone for good
//! (@0x33c62), as one whose mission was accepted with `Flags` 0x0100
//! is. One of `Flags` 0x0400 says it only while its `LinkMission` is
//! available from a ship (`_HandleShip` @0x33dc7-0x33e31), its
//! `AvailBits` by the engine the verdict cached at the last refresh or
//! check of it ([`RuleKey::CachedAvailBits`](crate::RuleKey::CachedAvailBits)).
//! A quote said, and that check of a linked person, leave `<OSN>` naming
//! nobody (see the `person_offers` module). "No quote showing" is the
//! message line free, whatever shows on it (`_HandleShip`
//! @0x33f42-0x33f4b reads `_commCount`, the session's
//! [`CommLine`](crate::CommLine); see the `comm_calls` module): a hail
//! quote holds it for [`QUOTE_SHOWN_TICKS`], and any other line for its
//! own time. A person's quote state is never saved, and "once" is once
//! for its stay.
//!
//! A person of `Flags` 0x0010 turning on the player as
//! `_AIMakeShipAttackPlayer` has it, while it is no threat to the player
//! yet (`_ExtendedIsThreatToPlayer`,
//! [`Npc::threatens_player_extended`](crate::traffic::npc::Npc::threatens_player_extended)),
//! says its quote at the start of the next quote tick, with no draw and
//! none of the filters above. It turns so through the player's hit on a
//! ship whose wait runs, that the player can't see or that is in a goal
//! the threat leaves out; a declined bribe; or a warship's or
//! interceptor's decision (its government's 0x0004 word, a grudge, a
//! xenophobe's or the hunt's pick of the player: see
//! [`Decision::turns_on_player`](crate::ai::Decision::turns_on_player);
//! [`Traffic::take_attack_quotes`](crate::traffic::Traffic::take_attack_quotes),
//! [`turns_with_quote`](crate::person::turns_with_quote)). It is quoted,
//! and no random quote is said while it shows, but the time of its last
//! quote is kept as it was. Every hail quote said beeps
//! ([`SimSound::HailQuote`](crate::SimSound::HailQuote), `_beepSnd[4]`,
//! `_ShowPersonHailQuote` @0x44586-0x445ad), its line empty or not.
//!
//! **In a fight.** A person whose `Flags` has 0x0001 holds a grudge
//! against the player once a shot of the player's hits it, kept on the
//! pilot, so every later appearance starts with it. A person destroyed
//! without an escape pod (`Flags` 0x0002), self-destructed or shot
//! down, is gone for good, and so is one captured, escape pod or not;
//! its grudge goes with it. One that lands or jumps out stays alive.
//! Neither a grudge nor a death makes a save due: the next save keeps
//! them. An invincible person (`ShieldMod` below 0) ends each fight tick
//! with its shield and armour full, and is never disabled nor broken up
//! (the original refills them every frame; its order against a shot's
//! damage is not traced).

use crate::pilot::Pilot;
use std::rc::Rc;

use super::missions::TextSource;
use super::{Outbox, Session, Setup, Space, Statics, hire};
use crate::catalog::{CommCatalog, MissionId, PersonId, StringPatch};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::hail::likes_player;
use crate::person::{
    ESCAPE_POD, Eligible, GRUDGE, HAIL_QUOTES, PersonRules, QUOTE_GAP_TICKS, QUOTE_IF_MISSION,
    QUOTE_ODDS, QUOTE_SHOWN_TICKS, QuoteTags, QuoteView, expand_tags, hail_quote_patch_id,
    hail_quote_text, quote_eligible,
};
use crate::sound::SimSound;
use crate::text_tags::TagRules;
use crate::traffic::npc::{Mode, Npc, NpcId, NpcPerson};

/// Whether the original's 0x0400 quote filter checks `npc`'s mission this
/// frame (`_HandleShip` @0x33c45-0x33cb4, @0x33dc7-0x33de0), naming its
/// slot around the check: a person of a `LinkMission` (`linked`) and
/// `Flags` 0x0400, with a hail quote, flying, while the player is not
/// jumping. Its other gates (the ship seen by the player's) are not
/// modelled.
fn checks_mission(npc: &Npc, person: &NpcPerson, linked: bool, player_jumping: bool) -> bool {
    let flying = npc.mode == Mode::Flying
        && !matches!(
            npc.condition,
            Condition::Dying { .. } | Condition::Destroyed
        );
    linked
        && person.flags & QUOTE_IF_MISSION != 0
        && person.hail_quote != -1
        && flying
        && !player_jumping
}

/// A person's hail quote said, for the flight's message line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonQuote {
    /// The NPC that said it.
    pub npc: NpcId,
    /// Its words, its tags read.
    pub text: String,
}

impl PersonQuote {
    /// How long the quote shows: [`QUOTE_SHOWN_TICKS`] (@0x4464d), the
    /// one count the session sets the message line to and the flight
    /// shows it for.
    #[must_use]
    pub fn ticks(&self) -> u16 {
        QUOTE_SHOWN_TICKS
    }
}

/// The hail quotes' clock: the quote ticks so far, in place of the
/// original's `TickCount`, which the personal gap ([`QUOTE_GAP_TICKS`])
/// reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct QuoteClock {
    pub(super) ticks: u64,
}

impl Session {
    /// This session with `rules` deciding how persons appear (see
    /// [`person`](crate::person)): Nova's
    /// ([`NovaPersons`](crate::NovaPersons)) by default.
    #[must_use]
    pub fn with_person_rules(mut self, rules: Rc<dyn PersonRules>) -> Self {
        self.setup.person_rules = hire::Shared(rules);
        self
    }

    /// How persons appear.
    #[must_use]
    pub fn person_rules(&self) -> &dyn PersonRules {
        self.setup.person_rules()
    }

    /// `npc`'s comm quote, an entry in `STR#` 7100, when it is a person
    /// with one.
    pub(super) fn quote_of(npc: &Npc) -> Option<u16> {
        npc.person
            .and_then(|person| u16::try_from(person.comm_quote).ok())
            .filter(|&quote| quote > 0)
    }

    /// `npc`'s name: a mission ship's drawn name while it is marked and
    /// that is not empty (`_DrawStatusTarg` @0x4b6c8-0x4b6e7), else its
    /// person's, or its ship type's, if the session has either.
    #[must_use]
    pub fn npc_name(&self, npc: &Npc) -> Option<&str> {
        self.space.npc_name(&self.pilot, &self.statics, npc)
    }

    /// `npc`'s subtitle, in place of its ship type's: while it is a
    /// marked mission ship, the mission's drawn subtitle unless that is
    /// empty, its person's never (@0x4b80f-0x4b86f); otherwise its
    /// person's, unless it has none or it is empty (@0x4b880).
    #[must_use]
    pub fn npc_subtitle(&self, npc: &Npc) -> Option<&str> {
        self.space.npc_subtitle(&self.pilot, npc)
    }

    /// A tick of the persons' hail quotes in flight, each drawn on
    /// `chance` and read from `catalog`, its `'STR '` patch before
    /// `STR#` 7101 (@0x445b8-0x445e1; see the module docs and
    /// [`person`](crate::person)), at whose head the message line counts
    /// down a frame, as `_HandleComm` does on the advance pass (see the
    /// `comm_calls` module): first each attack quote queued, said
    /// with no draw; then, in order, each person NPC whose quote
    /// may be said draws `Rand(140)`, and says it when it is due now, or
    /// on a draw of 0 when the message line is free
    /// ([`CommLine::free_for_calls`](crate::CommLine::free_for_calls))
    /// and its own last is more than [`QUOTE_GAP_TICKS`] back. While the
    /// ship is landed nothing happens. Then the quote clock moves on a
    /// tick: ours moves it once a frame in place of `TickCount`, and
    /// double time's second pass reads it unmoved
    /// ([`Session::tick_quotes_again`]).
    pub fn tick_quotes(&mut self, catalog: &(impl CommCatalog + ?Sized), chance: &mut dyn Chance) {
        if self.mode.landed().is_some() {
            return;
        }
        self.space.count_down_comm_line();
        self.roll_quotes(catalog, chance);
        self.space.escorts.quote_clock.ticks += 1;
    }

    /// Double time's second pass of the hail quotes: the attack quotes
    /// queued, then each eligible person's `Rand(140)` roll, as
    /// [`Session::tick_quotes`] has them, on the quote clock as the
    /// advance pass left it, which this never moves. `_HandleShip`
    /// (@0x44483) runs on both passes and its quote block
    /// (@0x33f0f-0x33fba) tests no pass, while `_HandleComm` counts the
    /// display timer down on the advance pass only (@0x4421e): after any
    /// quote on the advance pass only a quote due now follows, and
    /// otherwise each frame's odds double. The original's `TickCount` can
    /// move on a tick or two between the passes, which only shifts the
    /// personal gap ([`QUOTE_GAP_TICKS`]) by less than a frame, so no
    /// player can see it. While the ship is landed nothing happens.
    pub fn tick_quotes_again(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) {
        if self.mode.landed().is_some() {
            return;
        }
        self.roll_quotes(catalog, chance);
    }

    /// The quote block of [`Session::tick_quotes`], the clock unmoved.
    fn roll_quotes(&mut self, catalog: &(impl CommCatalog + ?Sized), chance: &mut dyn Chance) {
        let lines = catalog.string_list(HAIL_QUOTES);
        self.say_queued_attack_quotes(catalog, &lines, chance);
        let player_ai = self
            .statics
            .ship_record(self.pilot.ship)
            .map_or(0, |record| record.inherent_ai);
        let mut mission_checked = false;
        let decided: Vec<(NpcId, Eligible)> = {
            let world = self.world();
            let around = world.around(self.npcs());
            self.npcs()
                .iter()
                // A person escorting the player says no hail quote: the
                // original's joined ship is a mission ship, not the person.
                .filter(|npc| npc.escort.is_none())
                .filter_map(|npc| {
                    // One gone for good says none (@0x33c62).
                    let person = npc.person.filter(|person| !self.pilot.gone(person.id))?;
                    let linked = self
                        .space
                        .traffic
                        .person(person.id)
                        .and_then(|spawned| spawned.record.link_mission);
                    mission_checked |=
                        checks_mission(npc, &person, linked.is_some(), self.jumping().is_some());
                    let view = QuoteView {
                        liked: likes_player(npc, &around, self.mission_hail(npc)),
                        player_jumping: self.jumping().is_some(),
                        player_ai,
                        mission_available: linked.is_some_and(|mission| {
                            self.quoted_mission_available(MissionId(mission))
                        }),
                    };
                    Some((npc.id, quote_eligible(npc, &person, &view, self.npcs())))
                })
                .collect()
        };
        if mission_checked {
            // The 0x0400 filter names the ship around its check and puts
            // -1 back (@0x33dfb, @0x33e25).
            self.run.offering_ship = None;
        }
        let ticks = self.space.escorts.quote_clock.ticks;
        for (id, eligible) in decided {
            if eligible == Eligible::No {
                continue;
            }
            let draw = chance.below(QUOTE_ODDS);
            let Some(npc) = self
                .space
                .traffic
                .npcs_mut()
                .iter_mut()
                .find(|npc| npc.id == id)
            else {
                continue;
            };
            let Some(person) = npc.person.as_mut() else {
                continue;
            };
            // No quote showing: the message line free, whatever shows on
            // it (`_HandleShip` @0x33f42-0x33f4b reads `_commCount`).
            let quiet = self.space.comm_line.free_for_calls();
            let rested = person
                .quoted_at
                .is_none_or(|at| ticks > at + QUOTE_GAP_TICKS);
            if eligible != Eligible::Now && !(draw == 0 && quiet && rested) {
                continue;
            }
            // `_HandleShip` keeps the time of its last quote (+0xac,
            // @0x33fba).
            person.quoted_at = Some(ticks);
            self.say_hail_quote(id, catalog, &lines, chance);
        }
    }

    /// Says each attack quote queued, its line read from `catalog` or
    /// `lines`, drawing on `chance` as [`Session::say_hail_quote`] takes it
    /// in.
    fn say_queued_attack_quotes(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        lines: &[String],
        chance: &mut dyn Chance,
    ) {
        for id in self.space.traffic.take_attack_quotes() {
            self.say_hail_quote(id, catalog, lines, chance);
        }
    }

    /// NPC `id`'s person says its hail quote, its tags read, as
    /// `_ShowPersonHailQuote` has it (@0x4457e-0x44660): `catalog`'s
    /// `'STR '` 0x1387 + `HailQuote` when there is one (`_LoadPluginString`
    /// @0x445b8-0x445cb), else line `HailQuote` of `lines` (`STR#` 7101,
    /// @0x445e1), an empty line for one out of range
    /// ([`hail_quote_text`]); with `_beepSnd[4]`
    /// ([`SimSound::HailQuote`](crate::SimSound::HailQuote), @0x44586-0x445ad),
    /// shown for [`QUOTE_SHOWN_TICKS`] (@0x4464d), in which no random
    /// quote is said. It is quoted (+0xbc); the time of its last quote is
    /// its caller's to keep. An NPC gone, or flying no person, says
    /// nothing. Every hail quote, on attack, due now or at random, is said
    /// here, as every one goes through `_ShowPersonHailQuote`. Its line is
    /// taken in as a quote ([`TextSource::Quote`]), drawing on `chance`
    /// in the `l33t` mode, before its tags are read; and the ship
    /// `<OSN>` names is put back to none after it (@0x33f7e, @0x33fa6;
    /// the bounty hunter's @0x4474b, @0x44771; an attack's @0x89cbf,
    /// @0x89ced).
    fn say_hail_quote(
        &mut self,
        id: NpcId,
        catalog: &(impl CommCatalog + ?Sized),
        lines: &[String],
        chance: &mut dyn Chance,
    ) {
        let Some(person) = self
            .space
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == id)
            .and_then(|npc| npc.person.as_mut())
        else {
            return;
        };
        person.quoted = true;
        let (person, hail_quote) = (person.id, person.hail_quote);
        self.space.comm_line.show(QUOTE_SHOWN_TICKS);
        let patch = hail_quote_patch_id(hail_quote)
            .map_or(StringPatch::Absent, |patch| catalog.plugin_string(patch));
        let line = hail_quote_text(hail_quote, patch, lines);
        let line = self.take_in(TextSource::Quote, &line, chance);
        let tags = QuoteTags {
            person: self
                .space
                .traffic
                .person(person)
                .map_or("", |person| &person.record.name),
            pilot: &self.pilot.name,
            ship_type: self.ship_name(self.pilot.ship).unwrap_or_default(),
            ship_name: self.pilot.ship_name().unwrap_or_default(),
            ranks: self.run.rank_names(&self.pilot, &self.statics, &self.setup),
            rank_govts: self.run.rank_govts,
            leet: self.setup.leet,
            rules: TagRules::from_rulebook(&self.setup.rules),
        };
        let text = expand_tags(&line, &tags);
        self.out.quotes.push(PersonQuote { npc: id, text });
        self.out.sounds.push(SimSound::HailQuote);
        self.run.offering_ship = None;
    }

    /// The hail quotes said since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_quotes(&mut self) -> Vec<PersonQuote> {
        self.out.take_quotes()
    }

    /// Person `id` is gone for good: it never appears again, and its
    /// grudge goes with it (`_LoadPilotData` @0x75e1e-0x75e2f). No save is
    /// due: the next one keeps it.
    pub(super) fn go(&mut self, id: PersonId) {
        go(&mut self.pilot, id);
    }

    /// Holds the grudges of `strikes` that the traffic never answers, as
    /// they are dropped with the system left (`Session::populate_as`, an
    /// entry's `Reset::Strikes`), while the system's NPCs are still in the
    /// traffic: each answered player's hit is read alone against the NPCs
    /// as they stand ([`grudge`](crate::traffic::in_turn::grudge)), as the
    /// session held every grudge at its settle step before weapons-fidelity
    /// phase 45. Record only, not filed: in the original these hits were
    /// answered in turn, but the NPCs are discarded with the system, so
    /// their reactions are never seen, and the grudge is kept.
    pub(super) fn hold_left_grudges(&mut self, strikes: &[Strike]) {
        let hit: Vec<NpcId> = {
            let world = self.world();
            let around = world.around(self.space.traffic.npcs());
            strikes
                .iter()
                .filter_map(|strike| crate::traffic::in_turn::grudge(strike, &around))
                .collect()
        };
        self.space.hold_grudges(&mut self.pilot, &hit);
    }
}

impl Outbox {
    /// The hail quotes said since they were last taken, in order; taking
    /// them empties the list.
    fn take_quotes(&mut self) -> Vec<PersonQuote> {
        std::mem::take(&mut self.quotes)
    }
}

impl Setup {
    /// How persons appear.
    #[must_use]
    fn person_rules(&self) -> &dyn PersonRules {
        &*self.person_rules.0
    }
}

impl Space {
    /// `npc`'s name: a mission ship's drawn name while it is marked and
    /// that is not empty (`_DrawStatusTarg` @0x4b6c8-0x4b6e7), else its
    /// person's, or its ship type's, if the session has either.
    #[must_use]
    fn npc_name<'a>(
        &'a self,
        pilot: &'a Pilot,
        statics: &'a Statics,
        npc: &Npc,
    ) -> Option<&'a str> {
        if let Some(name) =
            super::mission_ships::mission_ship_text(pilot, npc.mission, |ships| &ships.name)
        {
            return Some(name);
        }
        match npc.person.and_then(|person| self.traffic.person(person.id)) {
            Some(person) => Some(&person.record.name),
            None => statics.ship_name(npc.ship),
        }
    }

    /// `npc`'s subtitle, in place of its ship type's: while it is a
    /// marked mission ship, the mission's drawn subtitle unless that is
    /// empty, its person's never (@0x4b80f-0x4b86f); otherwise its
    /// person's, unless it has none or it is empty (@0x4b880).
    #[must_use]
    fn npc_subtitle<'a>(&'a self, pilot: &'a Pilot, npc: &Npc) -> Option<&'a str> {
        if npc.mission.is_some() {
            return super::mission_ships::mission_ship_text(pilot, npc.mission, |ships| {
                &ships.subtitle
            });
        }
        let person = self.traffic.person(npc.person?.id)?;
        Some(person.record.subtitle.as_str()).filter(|subtitle| !subtitle.is_empty())
    }

    /// NPC `id`, destroyed, is taken out of the system, its record kept
    /// for the thinks that still read its slot
    /// ([`Traffic::remove_destroyed`](crate::traffic::Traffic::remove_destroyed)):
    /// a person without an escape pod is gone for good
    /// (`_HandleShipDisplay` @0x2dc4a-0x2dca1).
    pub(super) fn lose_destroyed(&mut self, pilot: &mut Pilot, id: NpcId) {
        let person = self
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| npc.person);
        if let Some(person) = person.filter(|person| person.flags & ESCAPE_POD == 0) {
            go(pilot, person.id);
        }
        self.traffic.remove_destroyed(id);
    }

    /// NPC `id`, captured, no longer flies as its person, its slot holds
    /// no person from now on, for `<OSN>` (+0xc8d0 = -1: @0x411b3,
    /// @0x89741, @0x946a6), and it has no `düde` (+0x78 = -1, @0x411aa).
    /// The person is gone for good, escape pod or not, when `gone`: Use As
    /// My Ship always (`_DoShipCapture` @0x41180), Use As Escort as
    /// [`RuleKey::CapturedPerson`](crate::RuleKey::CapturedPerson) reads
    /// (see the `capture` module); otherwise it lives on with its grudge.
    ///
    /// A mission's special ship keeps its mark +0xc8d2, as the original
    /// does: the escort path never writes it (`_DoPlunderDialog`
    /// @0x9459b-0x947cf, `_AIShipCapturedByPlayer` @0x89693-0x89780), so
    /// it flies on as a mission escort (see the `captured_mission_ship`
    /// module); and `_DoShipCapture` clears the old ship's mark
    /// (@0x413c2), not the captured one's, whose slot it makes inactive
    /// (@0x41a46). Every reader of the mark tests the active flag first
    /// (@0x9ea2a, @0x99ab5), and Use As My Ship takes the NPC out of the
    /// system, so that stale mark is never seen.
    pub(super) fn lose_captured(&mut self, pilot: &mut Pilot, id: NpcId, gone: bool) {
        let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
        let taken = npc.map(|npc| {
            npc.booty = None;
            (npc.berth, npc.person.take())
        });
        if let Some((berth, person)) = taken {
            self.traffic.forget_berth_person(berth);
            if let Some(person) = person.filter(|_| gone) {
                go(pilot, person.id);
            }
        }
    }

    /// Each person of `Flags` 0x0001 flying one of the NPCs `hit` holds a
    /// grudge against the player from now on, on the pilot too
    /// (`_DamageShip` @0x3be83-0x3beb3). The traffic decides which hits
    /// count as it answers each strike in turn
    /// ([`Traffic::take_grudges`](crate::traffic::Traffic::take_grudges),
    /// `in_turn::grudge`): an answered direct hit of the player's whose
    /// victim, by [`RuleKey::HitFocus`](crate::RuleKey::HitFocus)'s engine
    /// reading, does not keep its target against the player. No save is
    /// due.
    pub(super) fn hold_grudges(&mut self, pilot: &mut Pilot, hit: &[NpcId]) {
        for &id in hit {
            let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
            let Some(person) = npc.and_then(|npc| npc.person.as_mut()) else {
                continue;
            };
            if person.flags & GRUDGE != 0 {
                person.grudge = true;
                pilot.grudges.insert(person.id);
            }
        }
    }

    /// Refills each invincible person's shield and armour at the end of
    /// the fight's tick, after its shots landed, and keeps it flying: no
    /// hit among `strikes` disabled or broke it up (`_HandleShip`
    /// @0x33c0d-0x33c38 refills every frame).
    pub(super) fn refill_invincible(&mut self, strikes: &mut [Strike]) {
        for npc in self.traffic.npcs_mut() {
            if !npc.person.is_some_and(|person| person.invincible) {
                continue;
            }
            npc.reserves.shield.now = npc.reserves.shield.max;
            npc.reserves.armor.now = npc.reserves.armor.max;
            npc.condition = Condition::Intact;
            let me = ShipRef::Npc(npc.id);
            for strike in strikes.iter_mut().filter(|strike| strike.ship == me) {
                strike.downed = None;
            }
        }
    }
}

/// Person `id` is gone for good: it never appears again, and its
/// grudge goes with it (`_LoadPilotData` @0x75e1e-0x75e2f). No save is
/// due: the next one keeps it.
fn go(pilot: &mut Pilot, id: PersonId) {
    pilot.gone_persons.insert(id);
    pilot.grudges.remove(&id);
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{PersonId, PersonRecord, ShipId, SystemId, SystemTraffic};
    use crate::chance::{Chance, NeverFires};
    use crate::control::Test;
    use crate::control::{ControlBits, PilotFacts};
    use crate::person::{PersonRoll, PersonRules};
    use crate::pilot::Pilot;
    use crate::rulebook::RuleSource;
    use crate::rulebook::{RuleKey, Rulebook};
    use crate::testkit::{FAST, FakePilotCatalog, person, ship};

    #[test]
    fn a_destroyed_ships_place_is_read_by_the_brave_trader_it_provoked() {
        // `_HandleShipDisplay` frees the slot as the last explosion ends
        // (@0x2dee0) but keeps its record: a brave trader in an earlier
        // slot, fleeing it, reads its place on its next think, attacks it
        // within 1250 (@0x8b616-0x8b68d), and the high level ends that
        // attack keeping the provocation (@0x8e731).
        use crate::ai::Goal;
        use crate::flight::Controls;
        use crate::stats::ShipStats;
        use crate::traffic::npc::AiType;

        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        let ai = crate::ai::NovaAi::default();
        session.tick(Controls::default());
        session.tick_traffic(&catalog, &ai, &mut NeverFires);
        let mut attacker = crate::traffic::fixture::npc(0, ShipStats::default());
        attacker.ai_type = AiType::Warship;
        attacker.state.position = crate::geometry::Vec2::new(300.0, 0.0);
        let attacker = session.space.traffic.add_npc_at(attacker, 2);
        let mut trader = crate::traffic::fixture::npc(1, ShipStats::default());
        trader.ai_type = AiType::BraveTrader;
        trader.provoked = 10.0;
        trader.target = Some(ShipRef::Npc(attacker));
        trader.goal = Goal::Flee(ShipRef::Npc(attacker));
        let trader = session.space.traffic.add_npc_at(trader, 1);
        session.space.npc_mut(attacker).expect("there").condition = Condition::Destroyed;
        session.take_out_the_destroyed(&mut NeverFires);
        session.tick(Controls::default());
        session.tick_traffic(&catalog, &ai, &mut NeverFires);
        let after = session.space.npc(trader).expect("stays");
        assert_eq!((after.goal, after.target), (Goal::Idle, None));
        assert!((after.provoked - 10.0).abs() < f32::EPSILON, "kept");
    }

    #[test]
    fn a_quote_shows_for_420_ticks() {
        let quote = PersonQuote {
            npc: NpcId(0),
            text: String::new(),
        };
        assert_eq!(quote.ticks(), 420);
    }

    /// System 130 has no traffic of its own (`AvgShips` `avg_ships`) and
    /// one Person slot, naming "Ace" (`përs` 600, subtitle "Top Gun",
    /// flying ship 129) at 100 %. Person 601, linked anywhere, flies ship
    /// 129 too.
    fn peopled(avg_ships: i16) -> FakePilotCatalog {
        let mut slots: [(Option<PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(PersonId(600)), 100);
        FakePilotCatalog {
            traffic: vec![(
                SystemId(130),
                SystemTraffic {
                    dude_types: [(-1, 0); 8],
                    avg_ships,
                    persons: slots,
                },
            )],
            persons: vec![
                PersonRecord {
                    name: "Ace".to_owned(),
                    subtitle: "Top Gun".to_owned(),
                    link_syst: 131,
                    ..person(600, 129)
                },
                person(601, 129),
            ],
            ship_records: vec![ship(129, FAST)],
            ..crate::testkit::catalog()
        }
    }

    /// `pilot`'s session over `catalog`, populated as the draws' last
    /// outcomes say.
    fn populated(catalog: &FakePilotCatalog, session: Session) -> Session {
        let mut session = session;
        session.populate(catalog, &mut NeverFires);
        session
    }

    fn persons(session: &Session) -> Vec<Option<i16>> {
        session
            .npcs()
            .iter()
            .map(|npc| npc.person.map(|person| person.id.0))
            .collect()
    }

    #[test]
    fn populating_brings_the_person_a_slot_names() {
        let catalog = peopled(0);
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        assert_eq!(persons(&session), [Some(600)]);
    }

    #[test]
    fn a_person_gone_for_good_never_appears() {
        let catalog = peopled(0);
        let mut pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
        pilot.gone_persons.insert(PersonId(600));
        let session = populated(&catalog, Session::fly(&catalog, pilot).expect("flies"));
        assert_eq!(persons(&session), []);
    }

    /// Lets no expression hold.
    #[derive(Debug)]
    struct NoneHold;

    impl ControlBits for NoneHold {
        fn allows(&self, _test: &crate::control::TestExpr, _pilot: &dyn PilotFacts) -> bool {
            false
        }
    }

    #[test]
    fn a_person_whose_active_on_the_control_bits_refuse_never_appears() {
        let catalog = peopled(0);
        let session = Session::start(&catalog)
            .expect("starts")
            .with_control_bits(Rc::new(NoneHold));
        assert_eq!(persons(&populated(&catalog, session)), []);
    }

    #[test]
    fn through_the_real_control_bits_a_person_follows_its_active_on_bit() {
        let mut catalog = peopled(0);
        catalog.persons[0].active_on = Test::parse("b3");
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            persons(&populated(&catalog, session)),
            [],
            "never while bit 3 is clear"
        );
        let mut session = Session::start(&catalog).expect("starts");
        session.set_control_bit(crate::control::Bit::new(3).expect("in range"), true);
        assert_eq!(
            persons(&populated(&catalog, session)),
            [Some(600)],
            "at 100 % once it is set"
        );
    }

    /// Brings the first person who may appear, and no slot's.
    #[derive(Debug)]
    struct FirstComes;

    impl PersonRules for FirstComes {
        fn roll(&self, eligible: &[PersonId], _chance: &mut dyn Chance) -> PersonRoll {
            eligible
                .first()
                .map_or(PersonRoll::Empty, |&id| PersonRoll::Person(id))
        }

        fn listed(&self, _prob: i16, _chance: &mut dyn Chance) -> bool {
            false
        }

        fn link_slip(&self) -> RuleSource {
            RuleSource::Engine
        }

        fn shield_mod(&self) -> RuleSource {
            RuleSource::Engine
        }

        fn derelict_setup_speed(&self) -> RuleSource {
            RuleSource::Engine
        }
    }

    #[test]
    fn the_sessions_person_rules_decide_who_comes() {
        let catalog = peopled(1);
        let session = Session::start(&catalog)
            .expect("starts")
            .with_person_rules(Rc::new(FirstComes));
        assert_eq!(persons(&populated(&catalog, session)), [Some(601)]);
    }

    /// [`peopled`] with a düde ship (ship 129) in each setup pass, and
    /// person 601 nowhere; Ace's subtitle `subtitle`.
    fn mixed(subtitle: &str) -> FakePilotCatalog {
        let mut catalog = peopled(1);
        catalog.persons[0].subtitle = subtitle.to_owned();
        catalog.persons[1].link_syst = -2;
        catalog.traffic[0].1.dude_types[0] = (128, 100);
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog
    }

    #[test]
    fn a_person_is_named_and_subtitled_as_its_record_and_a_ship_as_its_type() {
        let catalog = mixed("Top Gun");
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        assert_eq!(persons(&session), [None, Some(600)]);
        let [ship, ace] = session.npcs() else {
            panic!("two NPCs: {:?}", session.npcs());
        };
        assert_eq!(session.npc_name(ace), Some("Ace"));
        assert_eq!(session.npc_subtitle(ace), Some("Top Gun"));
        assert_eq!(session.npc_name(ship), Some("Ship 129"));
        assert_eq!(session.npc_subtitle(ship), None);
        let catalog = mixed("");
        let session = populated(&catalog, Session::start(&catalog).expect("starts"));
        let ace = &session.npcs()[1];
        assert_eq!(session.npc_subtitle(ace), None, "an empty subtitle is none");
    }

    // Fighting persons.

    use crate::catalog::{HullRecord, StockWeapon, WeaponId, WeaponRecord};
    use crate::combat::armament::MOD_WEAPON;
    use crate::combat::armament::Trigger;
    use crate::combat::hull::Condition;
    use crate::combat::{Rules, ShipRef, Strike};
    use crate::geometry::Vec2;
    use crate::person::{ESCAPE_POD, GRUDGE};
    use crate::testkit::{Draws, hull, outfit, weapon};
    use crate::traffic::npc::NpcId;

    /// A blaster firing every other tick, 20 pixels a tick for 30 ticks,
    /// doing 5 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            reload: 2,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    /// Ship `id` armed with a blaster, breaking up for 2 ticks.
    fn armed_hull(id: i16) -> HullRecord {
        HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(128),
                count: 1,
                ammo: 0,
            }],
            death_delay: 2,
            size: Some(30),
            ..hull(id)
        }
    }

    /// [`peopled`] with no setup passes, the player and Ace armed with a
    /// blaster (held by outfit 250), and Ace of `flags` and `ShieldMod`
    /// `shield_mod`.
    fn duel(flags: u16, shield_mod: i16) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            weapons: vec![blaster()],
            hulls: vec![armed_hull(128), armed_hull(129)],
            outfits: vec![outfit(250, &[(MOD_WEAPON, 128)])],
            ..peopled(0)
        };
        catalog.persons[0].flags = flags;
        catalog.persons[0].shield_mod = shield_mod;
        catalog
    }

    /// The draws placing Ace 100 above the player, facing it.
    const ACE_ABOVE: [u32; 4] = [99, 750, 650, 180];

    /// `pilot`'s session over `catalog` with Ace placed 100 above the
    /// player, who targets it, so its hits are no stray ones, and holds
    /// its fire on it.
    fn facing_ace(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut Draws::of(&ACE_ABOVE));
        assert_eq!(persons(&session), [Some(600)]);
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
        session.chart.target = Some(session.npcs()[0].id);
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session.take_save_due();
        session
    }

    /// Fires on Ace until it is out of the system, at most 300 ticks.
    fn shoot_down(session: &mut Session) {
        for _ in 0..300 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs().is_empty() {
                return;
            }
        }
        panic!("never shot down: {:?}", session.npcs());
    }

    /// `session`'s pilot, saved and loaded again, flown once more with the
    /// same draws.
    fn flown_again(catalog: &FakePilotCatalog, session: &Session) -> Session {
        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut Draws::of(&ACE_ABOVE));
        session
    }

    fn ada(catalog: &FakePilotCatalog) -> Pilot {
        Pilot::new(catalog, "Ada").expect("a pilot")
    }

    #[test]
    fn a_unique_person_shot_down_is_gone_for_good_through_a_save() {
        let catalog = duel(0, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        shoot_down(&mut session);
        assert!(session.pilot().gone(PersonId(600)));
        assert!(!session.take_save_due(), "no save is due");
        assert_eq!(persons(&flown_again(&catalog, &session)), []);
    }

    #[test]
    fn a_person_with_an_escape_pod_shot_down_comes_again() {
        let catalog = duel(ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        shoot_down(&mut session);
        assert!(!session.pilot().gone(PersonId(600)));
        assert_eq!(persons(&flown_again(&catalog, &session)), [Some(600)]);
    }

    #[test]
    fn a_self_destructed_person_follows_the_escape_pod_rule() {
        for (flags, gone) in [(0, true), (ESCAPE_POD, false)] {
            let catalog = duel(flags, 0);
            let mut session = facing_ace(&catalog, ada(&catalog));
            session.hold_trigger(Trigger::default());
            session.self_destruct(NpcId(0));
            shoot_down(&mut session);
            assert_eq!(session.pilot().gone(PersonId(600)), gone, "{flags}");
        }
    }

    /// Ace captured, as phase 6's boarding leaves it awaiting its
    /// assignment.
    fn captured(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_ace(catalog, ada(catalog));
        session.space.escorts.aboard = Some(super::super::Aboard {
            npc: NpcId(0),
            ship: ShipId(129),
            plunder: crate::board::Plunder::default(),
            captured: true,
        });
        session
    }

    /// [`captured`] with Ace holding a grudge, by `source`'s reading of
    /// [`RuleKey::CapturedPerson`](crate::RuleKey::CapturedPerson).
    fn captured_by(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let rules =
            crate::Rulebook::default().with_override(crate::RuleKey::CapturedPerson, source);
        let mut session = captured(catalog).with_rules(rules);
        session.pilot.grudges.insert(PersonId(600));
        session
    }

    #[test]
    fn a_person_captured_as_an_escort_lives_on_by_the_engine_and_is_gone_by_the_other_reading() {
        use crate::board::Assignment;
        for flags in [0, ESCAPE_POD] {
            let catalog = duel(flags, 0);
            let mut session = captured_by(&catalog, RuleSource::Engine);
            assert!(
                session
                    .assign(Assignment::Escort, &mut NeverFires)
                    .is_some()
            );
            assert!(!session.pilot().gone(PersonId(600)), "{flags}: alive");
            assert!(
                session.pilot().grudge(PersonId(600)),
                "{flags}: its grudge kept"
            );
            assert!(
                session.npcs().iter().all(|npc| npc.person.is_none()),
                "{flags}: its ship flies for the player as no person"
            );
            assert_eq!(
                session
                    .pilot()
                    .escorts()
                    .iter()
                    .map(|escort| escort.person)
                    .collect::<Vec<_>>(),
                [None],
                "{flags}"
            );
            assert!(
                persons(&flown_again(&catalog, &session)).contains(&Some(600)),
                "{flags}: it appears again"
            );
            let mut session = captured_by(&catalog, RuleSource::Bible);
            assert!(
                session
                    .assign(Assignment::Escort, &mut NeverFires)
                    .is_some()
            );
            assert!(session.pilot().gone(PersonId(600)), "{flags}: gone");
            assert!(!session.pilot().grudge(PersonId(600)), "{flags}");
            assert!(session.npcs().iter().all(|npc| npc.person.is_none()));
        }
    }

    #[test]
    fn a_person_whose_ship_the_player_takes_is_gone_for_good_by_either_reading() {
        use crate::board::Assignment;
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = duel(ESCAPE_POD, 0);
            let mut session = captured_by(&catalog, source);
            assert!(
                session
                    .assign(Assignment::MyShip, &mut NeverFires)
                    .is_some()
            );
            assert!(session.pilot().gone(PersonId(600)), "{source:?}");
            assert!(!session.pilot().grudge(PersonId(600)), "{source:?}");
            assert!(
                session.npcs().iter().all(|npc| npc.person.is_none()),
                "no person flies for the player: {source:?}"
            );
        }
    }

    #[test]
    fn a_captured_ship_loses_its_dude() {
        use crate::board::Assignment;
        let catalog = duel(0, 0);
        let mut session = captured(&catalog);
        session.space.traffic.npcs_mut()[0].booty = Some(0x0141);
        assert!(
            session
                .assign(Assignment::Escort, &mut NeverFires)
                .is_some()
        );
        assert_eq!(session.npcs()[0].booty, None, "+0x78 = -1 (@0x411aa)");
    }

    #[test]
    fn the_players_hit_on_a_person_of_0x0001_holds_its_grudge() {
        let catalog = duel(GRUDGE | ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.answer_strikes(&crate::ai::NovaAi::default());
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "hit");
        assert!(session.npcs()[0].person.expect("Ace").grudge);
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(!session.take_save_due(), "no save is due");
        let catalog = duel(ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.answer_strikes(&crate::ai::NovaAi::default());
        }
        assert!(!session.npcs()[0].person.expect("Ace").grudge, "no 0x0001");
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    /// Fires on Ace until its shield drops, at most 60 ticks, leaving the
    /// strikes for the traffic to answer.
    fn hit_ace(session: &mut Session) {
        for _ in 0..60 {
            let shield = session.npcs()[0].reserves.shield.now;
            session.tick_combat(Rules::default(), &mut NeverFires);
            if session.npcs()[0].reserves.shield.now < shield {
                return;
            }
        }
        panic!("never hit: {:?}", session.npcs()[0]);
    }

    #[test]
    fn a_grudge_whose_strike_the_traffic_tick_answers_is_held_after_the_tick() {
        // A caller that never calls `answer_strikes` leaves the strike to
        // the next traffic tick, which answers it.
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        hit_ace(&mut session);
        session.tick_traffic(&catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(session.npcs()[0].person.expect("Ace").grudge);
    }

    #[test]
    fn a_grudge_whose_strike_a_jumps_arrival_drops_unanswered_is_held() {
        // Hits a jump's arrival comes before any answer to are never
        // answered: the arrival drops them (`Reset::Strikes`), holding
        // their grudges.
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        hit_ace(&mut session);
        session.mode = crate::session::Mode::Jumping(crate::session::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert!(!session.pilot().grudge(PersonId(600)), "unanswered");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert!(session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn a_grudge_whose_strike_populating_drops_unanswered_is_held() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        hit_ace(&mut session);
        session.populate(&catalog, &mut NeverFires);
        assert!(session.pilot().grudge(PersonId(600)));
        let catalog = duel(0, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        hit_ace(&mut session);
        session.populate(&catalog, &mut NeverFires);
        assert!(!session.pilot().grudge(PersonId(600)), "no 0x0001");
    }

    #[test]
    fn the_traffic_steps_in_a_world_holding_the_rulebooks_hit_focus() {
        let catalog = duel(GRUDGE, 0);
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let rules = Rulebook::default().with_override(RuleKey::HitFocus, source);
            let mut session = facing_ace(&catalog, ada(&catalog)).with_rules(rules);
            assert_eq!(session.world().hit_focus, source);
            let mut stepped = None;
            session.step_traffic(|_, world| stepped = Some(world.hit_focus));
            assert_eq!(stepped, Some(source));
        }
    }

    #[test]
    fn an_unanswered_player_hit_holds_no_grudge() {
        // The grudge (@0x3be83-0x3beb3) is inside the reaction block, which
        // a clear answer flag skips.
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let hit = crate::combat::fixture::strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        answer_in_one_pass(
            &mut session,
            &[Strike {
                answered: false,
                ..hit
            }],
        );
        assert!(!session.pilot().grudge(PersonId(600)));
        assert!(!session.npcs()[0].person.expect("Ace").grudge);
        answer_in_one_pass(&mut session, &[hit]);
        assert!(session.pilot().grudge(PersonId(600)), "answered");
    }

    /// [`facing_ace`] over [`duel`]`(GRUDGE, 0)` read by `focus`, with
    /// Ace facing up and attacking a quarry `above` it, the player 100
    /// below it.
    fn ace_hunting(focus: RuleSource, above: f32) -> Session {
        let catalog = duel(GRUDGE, 0);
        let rules = Rulebook::default().with_override(RuleKey::HitFocus, focus);
        let mut session = facing_ace(&catalog, ada(&catalog)).with_rules(rules);
        let mut quarry = session.npcs()[0].clone();
        quarry.person = None;
        quarry.state.position = Vec2::new(0.0, -100.0 - above);
        let quarry = session.space.traffic.add_npc(quarry);
        let ace = &mut session.space.traffic.npcs_mut()[0];
        ace.goal = crate::ai::Goal::Attack(ShipRef::Npc(quarry));
        ace.target = Some(ShipRef::Npc(quarry));
        ace.state.heading = 0.0;
        ace.pursuit.frame = None;
        session
    }

    #[test]
    fn a_person_keeping_its_target_holds_no_grudge_for_the_players_hit() {
        // The focus test's jump to 0x3bed2 (@0x3bcac) skips the grudge
        // (@0x3be83-0x3beb3): its quarry 50 above, the player 100 off
        // (10000 > 0.25 × 2500).
        let hit = crate::combat::fixture::strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        let mut session = ace_hunting(RuleSource::Engine, 50.0);
        answer_in_one_pass(&mut session, &[hit]);
        assert!(!session.pilot().grudge(PersonId(600)));
        assert!(!session.npcs()[0].person.expect("Ace").grudge);
        let mut session = ace_hunting(RuleSource::Bible, 50.0);
        answer_in_one_pass(&mut session, &[hit]);
        assert!(
            session.pilot().grudge(PersonId(600)),
            "by the other reading every answered player hit"
        );
    }

    /// `session` once the traffic has answered `strikes` in one pass.
    fn answer_in_one_pass(session: &mut Session, strikes: &[Strike]) {
        session.space.strikes.extend_from_slice(strikes);
        session.answer_strikes(&crate::ai::NovaAi::default());
    }

    #[test]
    fn a_grudge_reads_the_focus_the_strikes_before_it_left() {
        // Ace attacks a quarry 400 above it, facing it. An NPC A 20 above
        // hits it first (400 is not above 0.25 × 160000): it takes A
        // (+0x70 @0x3bcf6). The player's hit, 100 off, then measures
        // against A (10000 > 0.25 × 400), dead ahead, and is kept: the
        // focus test's jump (@0x3bcac) skips the grudge.
        let mut session = ace_hunting(RuleSource::Engine, 400.0);
        let mut ship = session.npcs()[0].clone();
        ship.person = None;
        ship.goal = crate::ai::Goal::Attack(ShipRef::Npc(NpcId(0)));
        ship.target = Some(ShipRef::Npc(NpcId(0)));
        ship.state.position = Vec2::new(0.0, -120.0);
        let near = ShipRef::Npc(session.space.traffic.add_npc(ship));
        let ace = ShipRef::Npc(NpcId(0));
        answer_in_one_pass(
            &mut session,
            &[
                crate::combat::fixture::strike(ace, near),
                crate::combat::fixture::strike(ace, ShipRef::Player),
            ],
        );
        assert_eq!(session.npcs()[0].target, Some(near));
        assert!(!session.pilot().grudge(PersonId(600)));
        assert!(!session.npcs()[0].person.expect("Ace").grudge);
    }

    #[test]
    fn a_players_first_hit_in_a_pass_holds_the_grudge_its_second_keeps_out() {
        // Ace attacks a quarry 400 above it while facing the player below:
        // the player's first hit turns it (no keep, facing away from its
        // quarry) and holds the grudge; its second is kept (A == T, faced).
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let mut ship = session.npcs()[0].clone();
        ship.person = None;
        ship.state.position = Vec2::new(0.0, -500.0);
        let quarry = ShipRef::Npc(session.space.traffic.add_npc(ship));
        let ace = &mut session.space.traffic.npcs_mut()[0];
        ace.goal = crate::ai::Goal::Attack(quarry);
        ace.target = Some(quarry);
        ace.pursuit.frame = None;
        let hit = crate::combat::fixture::strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        answer_in_one_pass(&mut session, &[hit, hit]);
        assert_eq!(session.npcs()[0].target, Some(ShipRef::Player));
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(session.npcs()[0].person.expect("Ace").grudge);
    }

    #[test]
    fn a_person_hit_by_the_player_nearer_than_half_its_targets_distance_holds_its_grudge() {
        // Its quarry 400 above: 10000 is not above 0.25 × 160000.
        let hit = crate::combat::fixture::strike(ShipRef::Npc(NpcId(0)), ShipRef::Player);
        for focus in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = ace_hunting(focus, 400.0);
            answer_in_one_pass(&mut session, &[hit]);
            assert!(session.pilot().grudge(PersonId(600)), "{focus:?}");
            assert!(session.npcs()[0].person.expect("Ace").grudge, "{focus:?}");
        }
    }

    /// [`facing_ace`] over [`duel`]`(GRUDGE, 0)`, Ace still facing the
    /// player and attacking a quarry at `quarry`, ticked through the
    /// fight's real settle path (each tick's strikes settled, then
    /// answered) until the player's blaster hits it, at most 60 ticks.
    fn ace_hit_while_hunting(quarry: Vec2) -> (Session, NpcId) {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let mut ship = session.npcs()[0].clone();
        ship.person = None;
        ship.state.position = quarry;
        let quarry = session.space.traffic.add_npc(ship);
        let ace = &mut session.space.traffic.npcs_mut()[0];
        ace.goal = crate::ai::Goal::Attack(ShipRef::Npc(quarry));
        ace.target = Some(ShipRef::Npc(quarry));
        ace.pursuit.frame = None;
        for _ in 0..60 {
            let shield = session.npcs()[0].reserves.shield.now;
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.answer_strikes(&crate::ai::NovaAi::default());
            if session.npcs()[0].reserves.shield.now < shield {
                return (session, quarry);
            }
        }
        panic!("never hit: {:?}", session.npcs()[0]);
    }

    #[test]
    fn the_grudges_focus_reads_the_npcs_before_the_strikes_are_answered() {
        // The focus test (@0x3bbf3-0x3bcac) runs before `_DamageShip`'s
        // `_AIMakeShipAttackPlayer`: read after the answer, a person
        // turned on the player and facing it would look to keep its new
        // target and skip the grudge (@0x3be83-0x3beb3).
        //
        // Its quarry beside the player, nearer than twice the player's 100
        // (100² > 0.25 × 116.6²): it keeps its target, with no grudge.
        let (session, quarry) = ace_hit_while_hunting(Vec2::new(60.0, 0.0));
        assert_eq!(session.world().hit_focus, RuleSource::Engine);
        assert!(!session.pilot().grudge(PersonId(600)));
        assert!(!session.npcs()[0].person.expect("Ace").grudge);
        assert_eq!(
            session.npcs()[0].goal,
            crate::ai::Goal::Attack(ShipRef::Npc(quarry))
        );
        // Its quarry behind the player, 400 off (100² is not above
        // 0.25 × 400²): it holds the grudge and turns on the player.
        let (session, _) = ace_hit_while_hunting(Vec2::new(0.0, 300.0));
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(session.npcs()[0].person.expect("Ace").grudge);
        assert_eq!(
            session.npcs()[0].goal,
            crate::ai::Goal::Attack(ShipRef::Player)
        );
    }

    #[test]
    fn a_hit_by_no_ship_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        answer_in_one_pass(
            &mut session,
            &[Strike {
                ship: ShipRef::Npc(NpcId(0)),
                by: None,
                damage: 1.0,
                downed: None,
                direct: true,
                intended: true,
                lock: false,
                nominal: crate::combat::Nominal::default(),
                weight: 0.0,
                answered: true,
            }],
        );
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn an_escorts_hit_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let hit = |by| Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by: Some(by),
            damage: 1.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
        };
        answer_in_one_pass(&mut session, &[hit(ShipRef::Npc(NpcId(7)))]);
        assert!(!session.pilot().grudge(PersonId(600)));
        answer_in_one_pass(&mut session, &[hit(ShipRef::Player)]);
        assert!(session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn the_players_blast_or_shockwave_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        // A blast's strike and a shockwave's alike: no direct hit.
        answer_in_one_pass(
            &mut session,
            &[Strike {
                ship: ShipRef::Npc(NpcId(0)),
                by: Some(ShipRef::Player),
                damage: 32.0,
                downed: None,
                direct: false,
                intended: false,
                lock: false,
                nominal: crate::combat::Nominal::default(),
                weight: 0.0,
                answered: false,
            }],
        );
        assert!(!session.pilot().grudge(PersonId(600)));
        assert!(!session.npcs()[0].person.expect("Ace").grudge);
    }

    #[test]
    fn a_person_gone_for_good_loses_its_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut pilot = ada(&catalog);
        pilot.grudges.insert(PersonId(600));
        let mut session = facing_ace(&catalog, pilot);
        assert!(
            session.npcs()[0].person.expect("Ace").grudge,
            "it enters with it"
        );
        shoot_down(&mut session);
        assert!(session.pilot().gone(PersonId(600)));
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn the_refill_clears_only_the_invincible_persons_downed_marks() {
        let catalog = duel(0, -1);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let downed = |ship| Strike {
            ship,
            by: Some(ShipRef::Player),
            damage: 9.0,
            downed: Some(crate::combat::Downed::Disabled),
            direct: true,
            intended: true,
            lock: false,
            nominal: crate::combat::Nominal::default(),
            weight: 0.0,
            answered: true,
        };
        let mut strikes = [
            downed(ShipRef::Npc(NpcId(0))),
            downed(ShipRef::Npc(NpcId(7))),
        ];
        session.space.refill_invincible(&mut strikes);
        assert_eq!(strikes[0].downed, None, "Ace's");
        assert_eq!(
            strikes[1].downed,
            Some(crate::combat::Downed::Disabled),
            "another ship's kept"
        );
    }

    #[test]
    fn an_invincible_person_ends_each_fight_tick_whole_and_is_never_disabled() {
        let catalog = duel(0, -1);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let full = session.npcs()[0].reserves;
        for _ in 0..300 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            let ace = &session.npcs()[0];
            assert_eq!(ace.reserves, full);
            assert_eq!(ace.condition, Condition::Intact);
        }
        let hits = session
            .space
            .strikes
            .iter()
            .filter(|strike| strike.ship == ShipRef::Npc(NpcId(0)) && strike.damage > 0.0)
            .count();
        assert!(hits > 100, "the blaster hit it: {hits}");
        assert!(
            session
                .space
                .strikes
                .iter()
                .all(|strike| strike.downed.is_none()),
            "never downed"
        );
        assert!(!session.pilot().gone(PersonId(600)));
    }

    // Talking to persons.

    use crate::hail::HailOptions;

    /// `STR#` 3000's groups: each variant of group g reads "c<g>", but
    /// for "Channel open." (0) and "What is it you want?" (2).
    fn comm_strings() -> Vec<String> {
        (0..40)
            .flat_map(|group| {
                let said = match group {
                    0 => "Channel open.".to_owned(),
                    2 => "What is it you want?".to_owned(),
                    _ => format!("c{group}"),
                };
                std::iter::repeat_n(said, 5)
            })
            .collect()
    }

    /// [`peopled`] with Ace quoting `comm_quote` (`STR#` 7100 #24 is
    /// "Well met.") and showing `PICT` 7800 when hailed, and the replies.
    fn talkative(comm_quote: i16) -> FakePilotCatalog {
        let mut catalog = peopled(0);
        catalog.persons[0].comm_quote = comm_quote;
        catalog.persons[0].hail_pict = Some(7800);
        let messages = (1..=200)
            .map(|n| {
                if n == 175 {
                    "Greetings.".to_owned()
                } else {
                    format!("m{n}")
                }
            })
            .collect();
        let mut quotes: Vec<String> = (1..=30).map(|n| format!("q{n}")).collect();
        quotes[23] = "Well met.".to_owned();
        catalog.strings = vec![(3000, comm_strings()), (2002, messages), (7100, quotes)];
        catalog
    }

    /// The hail of Ace by `catalog` under `rule`, hostile or not: its
    /// opening line, then Greetings' answer.
    fn greeted(catalog: &FakePilotCatalog, rule: RuleSource, hostile: bool) -> (String, String) {
        greeted_by(catalog, rule, hostile, "")
    }

    /// [`greeted`], the pilot named `pilot`.
    fn greeted_by(
        catalog: &FakePilotCatalog,
        rule: RuleSource,
        hostile: bool,
        pilot: &str,
    ) -> (String, String) {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::CommQuote, rule));
        if !pilot.is_empty() {
            session.pilot.name = pilot.to_owned();
        }
        session.populate(catalog, &mut NeverFires);
        if hostile {
            session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        }
        session.chart.target = Some(NpcId(0));
        let options = HailOptions::default();
        let opened = session
            .hail(catalog, &options, &mut NeverFires)
            .expect("answers");
        let greeted = session
            .answer(0, catalog, &options, &mut NeverFires)
            .expect("answers");
        (opened.reply, greeted.reply)
    }

    #[test]
    fn by_the_engine_a_friendly_persons_greetings_says_its_comm_quote() {
        let catalog = talkative(24);
        assert_eq!(
            greeted(&catalog, RuleSource::Engine, false),
            ("Channel open.".to_owned(), "Well met.".to_owned())
        );
        assert_eq!(
            greeted(&catalog, RuleSource::Engine, true),
            ("What is it you want?".to_owned(), "c13".to_owned()),
            "a hostile one: stop wasting my time"
        );
        for none in [-1, 0] {
            assert_eq!(
                greeted(&talkative(none), RuleSource::Engine, false).1,
                "Greetings.",
                "{none}"
            );
        }
    }

    #[test]
    fn a_persons_comm_quote_fills_its_tags() {
        let mut catalog = talkative(24);
        catalog.strings[2].1[23] = "Well met, <PN>.".to_owned();
        assert_eq!(
            greeted_by(&catalog, RuleSource::Engine, false, "Stock").1,
            "Well met, Stock.",
            "as Greetings' answer"
        );
        assert_eq!(
            greeted_by(&catalog, RuleSource::Bible, false, "Stock").0,
            "Well met, Stock.",
            "as the opening line"
        );
    }

    #[test]
    fn a_session_keeps_its_person_rules() {
        let catalog = peopled(0);
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            format!("{:?}", session.person_rules()),
            format!("{:?}", crate::person::NovaPersons::default())
        );
        let session = Session::start(&catalog)
            .expect("starts")
            .with_person_rules(Rc::new(FirstComes));
        assert_eq!(format!("{:?}", session.person_rules()), "FirstComes");
    }

    #[test]
    fn by_the_other_reading_a_person_opens_the_hail_with_its_comm_quote() {
        let catalog = talkative(24);
        assert_eq!(
            greeted(&catalog, RuleSource::Bible, false),
            ("Well met.".to_owned(), "Greetings.".to_owned())
        );
        assert_eq!(
            greeted(&catalog, RuleSource::Bible, true).0,
            "Well met.",
            "even when hostile"
        );
        assert_eq!(
            greeted(&talkative(-1), RuleSource::Bible, false),
            ("Channel open.".to_owned(), "Greetings.".to_owned())
        );
    }

    #[test]
    fn a_persons_hail_shows_its_picture() {
        let catalog = talkative(24);
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.chart.target = Some(NpcId(0));
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answers");
        assert_eq!(view.portrait, Some(7800));
        let catalog = mixed("");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.chart.target = Some(NpcId(0));
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answers");
        assert_eq!(view.portrait, None, "a ship has none");
    }

    // Hail quotes.

    use crate::SimSound;
    use crate::person::{QUOTE_ON_ATTACK, QUOTE_ONCE};

    /// Two persons in System 130's Person slots at 100 %, "Ace" (600) and
    /// "Bee" (601), of `flags` each, both saying `STR#` 7101 #1, "<OSN>:
    /// Prepare to die, <PN>!".
    fn quoters(flags: [u16; 2]) -> FakePilotCatalog {
        let mut catalog = peopled(0);
        catalog.traffic[0].1.persons[1] = (Some(PersonId(601)), 100);
        catalog.persons[0].flags = flags[0];
        catalog.persons[0].hail_quote = 1;
        catalog.persons[1] = PersonRecord {
            name: "Bee".to_owned(),
            flags: flags[1],
            hail_quote: 1,
            link_syst: 131,
            ..person(601, 129)
        };
        catalog.strings = vec![(7101, vec!["<OSN>: Prepare to die, <PN>!".to_owned()])];
        catalog
    }

    /// A pilot named "Stock" flying among [`quoters`] of `flags`.
    fn quoting(flags: [u16; 2]) -> (FakePilotCatalog, Session) {
        let catalog = quoters(flags);
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(persons(&session), [Some(600), Some(601)]);
        (catalog, session)
    }

    /// A quote tick drawing `draws`: what was said, and the bounds asked.
    fn quote_tick(
        catalog: &FakePilotCatalog,
        session: &mut Session,
        draws: &[u32],
    ) -> (Vec<String>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        session.tick_quotes(catalog, &mut chance);
        let said = session
            .take_quotes()
            .into_iter()
            .map(|quote| quote.text)
            .collect();
        (said, chance.asked)
    }

    /// Quote ticks with each person drawing 1, never saying a random
    /// quote, `ticks` times.
    fn quiet_ticks(catalog: &FakePilotCatalog, session: &mut Session, ticks: u64) {
        for _ in 0..ticks {
            assert_eq!(
                quote_tick(catalog, session, &[1, 1]).0,
                Vec::<String>::new()
            );
        }
    }

    #[test]
    fn a_person_says_its_attack_quote_once_it_begins_to_attack_the_player() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK | QUOTE_ONCE, QUOTE_ON_ATTACK]);
        session.space.traffic.npcs_mut()[1].person = None;
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, asked) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(asked, [140], "drawn whatever it gives");
        for _ in 0..3 {
            assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        }
        let quote = session.npcs()[0].person.expect("Ace");
        assert!(quote.quoted);
    }

    #[test]
    fn a_quote_names_the_players_ship_by_the_name_it_was_given() {
        let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        catalog.strings = vec![(7101, vec!["Nice <PST>, the <PSN>.".to_owned()])];
        let mut pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        catalog
            .ship_records
            .push(crate::testkit::ship(pilot.ship().0, crate::testkit::FAST));
        pilot.ship_name = Some("Kestrel".to_owned());
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(
            said,
            [format!("Nice Ship {}, the Kestrel.", session.ship().0)]
        );
    }

    #[test]
    fn a_quote_names_the_players_rank_once_one_is_granted() {
        let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        catalog.strings = vec![(7101, vec!["Halt, <PRK>! A <RRK>?".to_owned()])];
        catalog.ranks = vec![crate::catalog::RankRecord {
            name: "Commander".to_owned(),
            conv_name: "Cmdr".to_owned(),
            ..crate::testkit::rank(128)
        }];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session.run_set(
            &crate::control::SetExpr::parse("K128").expect("parses"),
            &mut crate::testkit::Scripted::default(),
        );
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Halt, Cmdr! A Commander?"], "not captain");
    }

    /// [`quoters`] whose quote is `line`, with ranks 200 (government
    /// 128, "Cdr") and 201 (129, "Ad") held, Ace attacking the player.
    fn ranked_quote(line: &str, rules: Rulebook) -> (FakePilotCatalog, Session) {
        let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        catalog.strings = vec![(7101, vec![line.to_owned()])];
        let rank = |id, govt, short: &str| crate::catalog::RankRecord {
            short_name: short.to_owned(),
            affil_govt: Some(crate::catalog::GovtId(govt)),
            ..crate::testkit::rank(id)
        };
        catalog.ranks = vec![rank(200, 128, "Cdr"), rank(201, 129, "Ad")];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(rules);
        session.populate(&catalog, &mut NeverFires);
        session.run_set(
            &crate::control::SetExpr::parse("K200 K201").expect("parses"),
            &mut crate::testkit::Scripted::default(),
        );
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        (catalog, session)
    }

    #[test]
    fn by_the_engine_a_quote_parses_its_rank_governments_onto_those_remembered() {
        let (catalog, mut session) = ranked_quote("<PRK128>, I said!", Rulebook::default());
        let _ = session.take_in(TextSource::Desc, "Earlier: <SRK129>.", &mut NeverFires);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(
            said,
            ["Ad, I said!"],
            "no reset (@0x445ee): the earlier <SRK129> crosses"
        );
        assert_eq!(
            session.plain_text("<SRK129> <PRK128>"),
            "Cdr Ad",
            "both kept for later texts"
        );
    }

    #[test]
    fn by_the_bible_a_quotes_rank_nnn_tag_reads_its_own_government() {
        let rules = Rulebook::default().with_override(RuleKey::CrossedRankTags, RuleSource::Bible);
        let (catalog, mut session) = ranked_quote("<PRK128>, I said!", rules);
        let _ = session.take_in(TextSource::Desc, "Earlier: <SRK129>.", &mut NeverFires);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Cdr, I said!"]);
    }

    #[test]
    fn a_quote_reads_its_str_patch_before_str_7101() {
        use crate::catalog::StringPatch;
        let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        catalog.strings = vec![(7101, vec!["Stock line.".to_owned(); 3])];
        let said_with = |catalog: &FakePilotCatalog| {
            let pilot = Pilot::new(catalog, "Stock").expect("a pilot");
            let mut session = Session::fly(catalog, pilot).expect("flies");
            session.populate(catalog, &mut NeverFires);
            session.space.traffic.npcs_mut()[1].person = None;
            session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
            quote_tick(catalog, &mut session, &[77]).0
        };
        let quote = catalog.persons[0].hail_quote;
        let patch = 4999 + quote;
        assert_eq!(said_with(&catalog), ["Stock line."]);
        catalog.string_patches = vec![(patch, StringPatch::Text("<PN>, patched.".to_owned()))];
        assert_eq!(
            said_with(&catalog),
            ["Stock, patched."],
            "'STR ' 4999 + n (@0x445b2-0x445e6), its tags filled"
        );
        catalog.string_patches = vec![(patch, StringPatch::Empty)];
        assert_eq!(said_with(&catalog), [""], "found, so STR# 7101 is not read");
        catalog.string_patches = vec![(patch + 1, StringPatch::Text("Not mine.".to_owned()))];
        assert_eq!(said_with(&catalog), ["Stock line."]);
    }

    #[test]
    fn each_quote_said_beeps_154() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        session.space.traffic.npcs_mut()[1].person = None;
        let _ = session.take_sounds();
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        assert_eq!(session.take_sounds(), [], "none said");
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said.len(), 1);
        assert_eq!(
            session.take_sounds(),
            [crate::sound::SimSound::HailQuote],
            "_ShowPersonHailQuote's _beepSnd[4] (@0x445a2-0x445ad)"
        );
    }

    /// String lists of none, from data holding a `l33t` resource.
    struct Leet;

    impl crate::catalog::CommCatalog for Leet {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }

        fn leet_mode(&self) -> bool {
            true
        }
    }

    #[test]
    fn with_leet_mode_a_quote_is_munged_but_draws_no_letters() {
        let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        catalog.strings = vec![(7101, vec!["Hello <PN>, you are alone.".to_owned()])];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Leet));
        session.populate(&catalog, &mut NeverFires);
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        // The draws after the quote's own land on 0, which would turn each
        // a to 4 and o to 0 were the letters drawn.
        let (said, asked) = quote_tick(&catalog, &mut session, &[77, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            said,
            ["H377o Stox0r, j00 ar3 a7on3."],
            "munged after its tags (@0x44605 → @0x9d5aa), its a's and o's kept"
        );
        assert_eq!(
            asked,
            [140],
            "only parsed (@0x445ee), never loaded: no letter draws (@0x7194f)"
        );
    }

    #[test]
    fn a_quote_reads_its_conditional_text_by_the_pilots_gender() {
        for (gender, said) in [
            (crate::pilot::Gender::Male, "Die, sir!"),
            (crate::pilot::Gender::Female, "Die, madam!"),
        ] {
            let mut catalog = quoters([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
            catalog.strings = vec![(7101, vec!["Die, {G \"sir\" \"madam\"}!".to_owned()])];
            let pilot = Pilot::new(&catalog, "Stock")
                .expect("a pilot")
                .with_gender(gender);
            let mut session = Session::fly(&catalog, pilot).expect("flies");
            session.populate(&catalog, &mut NeverFires);
            session.space.traffic.npcs_mut()[1].person = None;
            session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
            let (quotes, _) = quote_tick(&catalog, &mut session, &[77]);
            assert_eq!(quotes, [said], "parsed (@0x445ee)");
        }
    }

    /// A hailed ship named for `<OSN>` that is not among the NPCs: "Earlier".
    fn named_earlier(session: &mut Session) {
        session.space.traffic.set_berth_person(99, "Earlier");
        session.run.offering_ship = Some(super::super::person_offers::OfferingShip { berth: 99 });
        assert_eq!(session.plain_text("<OSN>"), "Earlier");
    }

    #[test]
    fn by_the_engine_a_quote_names_its_speaker_and_then_nobody() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        session.space.traffic.npcs_mut()[1].person = None;
        named_earlier(&mut session);
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        assert_eq!(session.plain_text("<OSN>"), "Earlier", "no quote said");
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(
            session.plain_text("<OSN>"),
            "[Error]",
            "set to the speaker around the quote, then -1 (@0x33f7e, @0x33fa6)"
        );
    }

    #[test]
    fn by_the_engine_a_linked_0x0400_persons_mission_check_names_nobody() {
        let (catalog, mut session) = quoting([crate::person::QUOTE_IF_MISSION, 0]);
        named_earlier(&mut session);
        let _ = quote_tick(&catalog, &mut session, &[1, 1]);
        assert_eq!(
            session.plain_text("<OSN>"),
            "Earlier",
            "Ace has no LinkMission: no check"
        );
        let mut linked = quoters([crate::person::QUOTE_IF_MISSION, 0]);
        linked.persons[0].link_mission = Some(130);
        let pilot = Pilot::new(&linked, "Stock").expect("a pilot");
        let mut session = Session::fly(&linked, pilot).expect("flies");
        session.populate(&linked, &mut NeverFires);
        named_earlier(&mut session);
        session.space.traffic.npcs_mut()[0].condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let _ = quote_tick(&linked, &mut session, &[1, 1]);
        assert_eq!(
            session.plain_text("<OSN>"),
            "Earlier",
            "a dying ship: no check"
        );
        session.space.traffic.npcs_mut()[0].condition = Condition::Intact;
        let _ = quote_tick(&linked, &mut session, &[1, 1]);
        assert_eq!(
            session.plain_text("<OSN>"),
            "[Error]",
            "named around _IsMissionAvailable(m, 0), then -1 (@0x33dfb, @0x33e25)"
        );
        let mut unflagged = linked;
        unflagged.persons[0].flags = 0;
        let pilot = Pilot::new(&unflagged, "Stock").expect("a pilot");
        let mut session = Session::fly(&unflagged, pilot).expect("flies");
        session.populate(&unflagged, &mut NeverFires);
        named_earlier(&mut session);
        let _ = quote_tick(&unflagged, &mut session, &[1, 1]);
        assert_eq!(session.plain_text("<OSN>"), "Earlier", "linked, not 0x0400");
    }

    /// [`quoters`] with Ace of `Flags` 0x0400 linked to mission 130,
    /// flown, the hailed ship named "Earlier": a quote tick checks Ace's
    /// mission unless one of the filter's gates is shut.
    fn linked_quoting() -> (FakePilotCatalog, Session) {
        let mut catalog = quoters([crate::person::QUOTE_IF_MISSION, 0]);
        catalog.persons[0].link_mission = Some(130);
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        named_earlier(&mut session);
        (catalog, session)
    }

    /// `<OSN>` after a quote tick in which nobody says a quote.
    fn osn_after_tick(catalog: &FakePilotCatalog, session: &mut Session) -> String {
        let (said, _) = quote_tick(catalog, session, &[1, 1]);
        assert_eq!(said, Vec::<String>::new());
        session.plain_text("<OSN>")
    }

    #[test]
    fn by_the_engine_the_0x0400_mission_check_skips_a_person_with_no_hail_quote() {
        let (catalog, mut session) = linked_quoting();
        let ace = session.space.traffic.npcs_mut()[0]
            .person
            .as_mut()
            .expect("Ace");
        ace.hail_quote = -1;
        assert_eq!(
            osn_after_tick(&catalog, &mut session),
            "Earlier",
            "hail quote -1: no check"
        );
        let ace = session.space.traffic.npcs_mut()[0]
            .person
            .as_mut()
            .expect("Ace");
        ace.hail_quote = 1;
        assert_eq!(osn_after_tick(&catalog, &mut session), "[Error]");
    }

    #[test]
    fn by_the_engine_the_0x0400_mission_check_is_skipped_while_the_player_jumps() {
        let (catalog, mut session) = linked_quoting();
        session.mode = crate::session::Mode::Jumping(super::super::Jump::Hyperspace {
            to: SystemId(131),
            warp: 0,
        });
        assert_eq!(
            osn_after_tick(&catalog, &mut session),
            "Earlier",
            "the player jumping: no check"
        );
        session.mode.end_jump();
        assert_eq!(osn_after_tick(&catalog, &mut session), "[Error]");
    }

    #[test]
    fn by_the_engine_the_0x0400_mission_check_skips_a_ship_not_flying() {
        let (catalog, mut session) = linked_quoting();
        session.space.traffic.npcs_mut()[0].mode = Mode::JumpingIn { tick: 0 };
        assert_eq!(
            osn_after_tick(&catalog, &mut session),
            "Earlier",
            "gliding in: no check"
        );
        session.space.traffic.npcs_mut()[0].mode = Mode::Flying;
        assert_eq!(osn_after_tick(&catalog, &mut session), "[Error]");
    }

    #[test]
    fn by_the_engine_the_0x0400_mission_check_skips_a_destroyed_ship() {
        let (catalog, mut session) = linked_quoting();
        session.space.traffic.npcs_mut()[0].condition = Condition::Destroyed;
        assert_eq!(
            osn_after_tick(&catalog, &mut session),
            "Earlier",
            "destroyed: no check"
        );
        session.space.traffic.npcs_mut()[0].condition = Condition::Intact;
        assert_eq!(osn_after_tick(&catalog, &mut session), "[Error]");
    }

    #[test]
    fn by_the_other_reading_quotes_leave_osn_as_it_was() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, QUOTE_ON_ATTACK]);
        let mut session = {
            session.space.traffic.npcs_mut()[1].person = None;
            session.with_rules(
                Rulebook::default().with_override(RuleKey::OfferingShipName, RuleSource::Bible),
            )
        };
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"], "the speaker");
        assert_eq!(session.plain_text("<OSN>"), "[Error]");
    }

    #[test]
    fn a_plain_person_says_its_quote_on_a_draw_of_0_in_140() {
        let (catalog, mut session) = quoting([0, 0]);
        assert_eq!(
            quote_tick(&catalog, &mut session, &[1, 1]),
            (vec![], vec![140, 140]),
            "each NPC its own draw, in order"
        );
        let (said, _) = quote_tick(&catalog, &mut session, &[1, 0]);
        assert_eq!(said, ["Bee: Prepare to die, Stock!"]);
    }

    /// Double time's second-pass roll drawing `draws`: what was said, and
    /// the bounds asked.
    fn quote_again(
        catalog: &FakePilotCatalog,
        session: &mut Session,
        draws: &[u32],
    ) -> (Vec<String>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        session.tick_quotes_again(catalog, &mut chance);
        let said = session
            .take_quotes()
            .into_iter()
            .map(|quote| quote.text)
            .collect();
        (said, chance.asked)
    }

    #[test]
    fn the_second_pass_rolls_each_eligible_persons_quote_again_and_leaves_the_clock() {
        // `_HandleShip` (@0x44483) runs on both passes and its quote block
        // (@0x33f0f-0x33fba) tests no pass.
        let (catalog, mut session) = quoting([QUOTE_ONCE, 0]);
        quiet_ticks(&catalog, &mut session, 1);
        let ticks = session.space.escorts.quote_clock.ticks;
        assert_eq!(
            quote_again(&catalog, &mut session, &[1, 1]),
            (vec![], vec![140, 140])
        );
        let (said, asked) = quote_again(&catalog, &mut session, &[0, 1]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(asked, [140, 140]);
        assert_eq!(session.take_sounds(), [SimSound::HailQuote]);
        assert_eq!(
            session.npcs()[0].person.expect("Ace").quoted_at,
            Some(ticks)
        );
        assert_eq!(
            quote_again(&catalog, &mut session, &[0]),
            (vec![], vec![140]),
            "Ace, quoted once, draws none"
        );
        assert_eq!(
            session.space.escorts.quote_clock.ticks, ticks,
            "the clock stands"
        );
    }

    #[test]
    fn after_an_advance_pass_quote_the_second_pass_says_no_random_one_but_a_due_one() {
        // `_HandleComm` counts the display timer down on the advance pass
        // only (@0x4421e), so it still shows; a forced quote skips that
        // gate (@0x33f65-0x33f6c).
        let (catalog, mut session) = quoting([0, QUOTE_ON_ATTACK]);
        assert_eq!(quote_tick(&catalog, &mut session, &[0]).0.len(), 1);
        assert_eq!(
            quote_again(&catalog, &mut session, &[0]),
            (vec![], vec![140]),
            "Ace draws, and stays quiet"
        );
        session.space.traffic.npcs_mut()[1].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, asked) = quote_again(&catalog, &mut session, &[0, 77]);
        assert_eq!(said, ["Bee: Prepare to die, Stock!"]);
        assert_eq!(asked, [140, 140]);
    }

    #[test]
    fn no_random_quote_is_said_for_420_ticks_after_any_and_the_same_person_waits_1350() {
        let (catalog, mut session) = quoting([0, 0]);
        // Tick 0: Ace speaks.
        assert_eq!(
            quote_tick(&catalog, &mut session, &[0, 0]).0.len(),
            1,
            "Bee waits"
        );
        quiet_ticks(&catalog, &mut session, 418);
        // Tick 419: still showing.
        assert_eq!(quote_tick(&catalog, &mut session, &[1, 0]).0.len(), 0);
        // Tick 420: Bee may speak.
        let (said, _) = quote_tick(&catalog, &mut session, &[1, 0]);
        assert_eq!(said, ["Bee: Prepare to die, Stock!"]);
        quiet_ticks(&catalog, &mut session, 929);
        // Tick 1350: Ace's last was 1350 ticks back, not more.
        assert_eq!(quote_tick(&catalog, &mut session, &[0, 1]).0.len(), 0);
        // Tick 1351: more.
        let (said, _) = quote_tick(&catalog, &mut session, &[0, 1]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
    }

    #[test]
    fn a_random_quote_waits_while_any_line_shows_and_is_said_once_it_is_free() {
        // `_HandleShip`'s roll reads `_commCount` (@0x33f42-0x33f4b), set
        // by every `_DisplayComm`, not only the quotes'.
        let (catalog, mut session) = quoting([0, 0]);
        session
            .space
            .say(&mut session.out, crate::SimMessage::NoResponse);
        let _ = session.take_messages();
        for tick in 0..249 {
            let (said, _) = quote_tick(&catalog, &mut session, &[0, 0]);
            assert_eq!(said, Vec::<String>::new(), "tick {tick}");
        }
        let (said, _) = quote_tick(&catalog, &mut session, &[0, 0]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"], "the 250th, at 0");
    }

    #[test]
    fn a_quote_said_once_a_stay_draws_no_more() {
        let (catalog, mut session) = quoting([QUOTE_ONCE, QUOTE_ONCE]);
        assert_eq!(quote_tick(&catalog, &mut session, &[0, 1]).0.len(), 1);
        quiet_ticks_one(&catalog, &mut session);
    }

    /// After Ace's quote, only Bee draws.
    fn quiet_ticks_one(catalog: &FakePilotCatalog, session: &mut Session) {
        for _ in 0..2000 {
            let (said, asked) = quote_tick(catalog, session, &[1]);
            assert_eq!((said.len(), asked), (0, vec![140]));
        }
    }

    #[test]
    fn a_person_of_0x0400_says_its_quote_only_while_its_mission_is_available_from_a_ship() {
        let mut catalog = quoters([crate::person::QUOTE_IF_MISSION, 0]);
        catalog.persons[0].link_mission = Some(130);
        catalog.missions = vec![crate::catalog::MissionRecord {
            avail_loc: 2,
            avail_random: 50,
            ..crate::testkit::mission(130)
        }];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session
            .run
            .offers
            .set_roll(crate::catalog::MissionId(130), 51);
        assert_eq!(
            quote_tick(&catalog, &mut session, &[1]),
            (vec![], vec![140]),
            "unavailable: Bee alone draws"
        );
        session
            .run
            .offers
            .set_roll(crate::catalog::MissionId(130), 50);
        let (said, asked) = quote_tick(&catalog, &mut session, &[0, 1]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(asked, [140, 140]);
    }

    /// [`quoters`] with Ace of `Flags` 0x0400 linked to mission 130, from
    /// a ship while bit 7 is set, flying with Bee quiet, by `rules`.
    fn quoting_while_b7(rules: Rulebook) -> (FakePilotCatalog, Session) {
        let mut catalog = quoters([crate::person::QUOTE_IF_MISSION, 0]);
        catalog.persons[0].link_mission = Some(130);
        catalog.missions = vec![crate::catalog::MissionRecord {
            avail_loc: 2,
            avail_bits: Test::parse("b7"),
            ..crate::testkit::mission(130)
        }];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(rules);
        session.populate(&catalog, &mut NeverFires);
        session.space.traffic.npcs_mut()[1].person = None;
        (catalog, session)
    }

    fn set_b7(session: &mut Session) {
        session.run_set(
            &crate::control::SetExpr::parse("b7").expect("parses"),
            &mut crate::testkit::Scripted::default(),
        );
    }

    #[test]
    fn by_the_engine_the_quote_filter_reads_the_avail_bits_verdict_of_the_last_refresh() {
        let (catalog, mut session) = quoting_while_b7(Rulebook::default());
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        set_b7(&mut session);
        assert_eq!(
            quote_tick(&catalog, &mut session, &[0]),
            (vec![], vec![]),
            "b7 set in flight: the cached verdict still refuses (@0x33e1d)"
        );
        session.propagate_avail_bits();
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"], "refreshed");
    }

    #[test]
    fn by_the_other_reading_the_quote_filter_reads_the_bits_fresh() {
        let rules = Rulebook::default().with_override(RuleKey::CachedAvailBits, RuleSource::Bible);
        let (catalog, mut session) = quoting_while_b7(rules);
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        set_b7(&mut session);
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
    }

    #[test]
    fn by_the_engine_hailing_the_person_refreshes_its_missions_verdict() {
        let (catalog, mut session) = quoting_while_b7(Rulebook::default());
        set_b7(&mut session);
        session.chart.target = Some(NpcId(0));
        let _ = session.open_hail(
            &catalog,
            &crate::hail::HailOptions::default(),
            &mut NeverFires,
        );
        session.space.escorts.talk = None;
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(
            said,
            ["Ace: Prepare to die, Stock!"],
            "the hail's _IsMissionAvailable(m, 1) writes it (@0x6220d)"
        );
    }

    #[test]
    fn a_jumps_arrival_caches_every_verdict_again() {
        let (mut catalog, mut session) = quoting_while_b7(Rulebook::default());
        catalog.star_map[0].links.push(SystemId(131));
        set_b7(&mut session);
        assert_eq!(session.run.avail_cache.get(&MissionId(130)), Some(&false));
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(
            session.run.avail_cache.get(&MissionId(130)),
            Some(&true),
            "@0x6bf3f"
        );
    }

    #[test]
    fn nothing_is_said_or_drawn_without_a_person() {
        let catalog = mixed("");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[]));
        for npc in session.space.traffic.npcs_mut() {
            npc.person = None;
        }
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
    }

    #[test]
    fn nothing_is_said_while_the_player_is_landed() {
        let (catalog, mut session) = quoting([0, 0]);
        session.mode = crate::session::Mode::Landed(crate::catalog::StellarId(128));
        assert_eq!(
            quote_tick(&catalog, &mut session, &[0, 0]),
            (vec![], vec![])
        );
    }

    /// Ace turns on the player: its quote queued as
    /// `_AIMakeShipAttackPlayer`'s gate lets it through.
    fn turn_ace(session: &mut Session) {
        let ace = session.npcs()[0].id;
        session
            .space
            .traffic
            .turn_with_quote(ace, crate::person::GateView::default());
    }

    #[test]
    fn an_attack_quote_is_said_at_once_without_a_draw_and_holds_random_quotes_back() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
        turn_ace(&mut session);
        let (said, asked) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(asked, [140], "Bee alone draws, and stays quiet");
        let ace = session.npcs()[0].person.expect("Ace");
        assert!(ace.quoted);
        assert_eq!(ace.quoted_at, None, "`_ShowPersonHailQuote` leaves +0xac");
        for tick in 1..420 {
            let (said, _) = quote_tick(&catalog, &mut session, &[0]);
            assert_eq!(said, Vec::<String>::new(), "tick {tick}");
        }
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said, ["Bee: Prepare to die, Stock!"], "tick 420");
    }

    /// [`quoting`] with Ace of `flags`, AI type `ai_type`, flying for
    /// government 140 of `Flags` 0x0004 (it always attacks the player),
    /// already quoted and holding no grudge, armed with a gun of unlimited
    /// ammunition (an unarmed warship is fully dry, and drops the player
    /// at once, `_WarshipAI` @0x8c0e7-0x8c117); Bee flies no person.
    fn hunted_by_ace(flags: u16, ai_type: i16) -> (FakePilotCatalog, Session) {
        let mut catalog = quoters([flags, 0]);
        catalog.persons[0].govt = Some(crate::catalog::GovtId(140));
        catalog.persons[0].ai_type = ai_type;
        catalog.govts = vec![crate::catalog::GovtRecord {
            flags: crate::govt::ALWAYS_ATTACKS_PLAYER,
            ..crate::testkit::govt(140)
        }];
        let pilot = Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&crate::testkit::weapon(127)),
            1,
        )]);
        let ace = session.space.traffic.npcs_mut()[0]
            .person
            .as_mut()
            .expect("Ace");
        ace.quoted = true;
        assert!(!ace.grudge);
        session.take_sounds();
        (catalog, session)
    }

    #[test]
    fn a_person_a_warships_or_interceptors_decision_turns_on_the_player_says_its_quote_at_once_unfiltered()
     {
        // `_WarshipAI` @0x8b7da and `_InterceptorAI` @0x8cb8f call
        // `_AIMakeShipAttackPlayer` for the 0x0004 word, whose gate reads
        // none of `_HandleShip`'s filters.
        use crate::person::{NOT_TO_WARSHIP, QUOTE_IF_GRUDGE};
        let flags = QUOTE_ON_ATTACK | QUOTE_ONCE | NOT_TO_WARSHIP | QUOTE_IF_GRUDGE;
        for ai_type in [3, 4] {
            let (catalog, mut session) = hunted_by_ace(flags, ai_type);
            session.tick_traffic(&catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
            assert_eq!(
                session.npcs()[0].goal,
                crate::ai::Goal::Attack(ShipRef::Player),
                "{ai_type}"
            );
            let (said, asked) = quote_tick(&catalog, &mut session, &[]);
            assert_eq!(said, ["Ace: Prepare to die, Stock!"], "{ai_type}");
            assert_eq!(asked, Vec::<u32>::new(), "{ai_type}: no Rand(140)");
            assert_eq!(session.take_sounds(), [SimSound::HailQuote], "{ai_type}");
        }
        // The high-level handler's mission hunt (@0x8d6b7) fails +0xc8d2
        // (@0x89c71).
        let (catalog, mut session) = hunted_by_ace(flags, 3);
        // Slot 0 holds a mission, so the mark is kept (@0x8fbcd-0x8fc96).
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 3));
        session.space.traffic.npcs_mut()[0].mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::mission::ships::ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        });
        session.tick_traffic(&catalog, &crate::ai::NovaAi::default(), &mut NeverFires);
        assert!(session.npcs()[0].mission.is_some(), "still marked");
        assert_eq!(
            session.npcs()[0].goal,
            crate::ai::Goal::Attack(ShipRef::Player)
        );
        assert_eq!(quote_tick(&catalog, &mut session, &[]), (vec![], vec![]));
        assert_eq!(session.take_sounds(), [], "a mission's ship");
    }

    #[test]
    fn a_plugins_str_patch_says_the_hail_quote_in_place_of_str_list_7101() {
        // `_ShowPersonHailQuote` looks up `STR ` 0x1387 + HailQuote first
        // (`_LoadPluginString` @0x445b8-0x445cb), `STR#` 7101 only with
        // none (@0x445e1).
        let (mut catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
        catalog.string_patches = vec![(
            5000,
            crate::catalog::StringPatch::Text("Plug-in: <PN>!".to_owned()),
        )];
        session.take_sounds();
        turn_ace(&mut session);
        let (said, _) = quote_tick(&catalog, &mut session, &[1]);
        assert_eq!(said, ["Plug-in: Stock!"], "tags read");
        assert_eq!(session.take_sounds(), [SimSound::HailQuote]);
        for _ in 0..420 {
            quote_tick(&catalog, &mut session, &[1]);
        }
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said, ["Plug-in: Stock!"], "Bee's at random too");
        let (mut catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
        catalog.string_patches = vec![(5000, crate::catalog::StringPatch::Empty)];
        turn_ace(&mut session);
        let (said, _) = quote_tick(&catalog, &mut session, &[1]);
        assert_eq!(said, [""], "an empty STR");
    }

    #[test]
    fn every_hail_quote_said_beeps_snd_154() {
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
        session.take_sounds();
        turn_ace(&mut session);
        quote_tick(&catalog, &mut session, &[1]);
        assert_eq!(session.take_sounds(), [SimSound::HailQuote], "on attack");
        for _ in 0..420 {
            quote_tick(&catalog, &mut session, &[1]);
        }
        assert_eq!(session.take_sounds(), [], "nothing said");
        let (said, _) = quote_tick(&catalog, &mut session, &[0]);
        assert_eq!(said.len(), 1);
        assert_eq!(session.take_sounds(), [SimSound::HailQuote], "at random");
        let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
        session.space.traffic.npcs_mut()[1].person = None;
        session.space.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        session.take_sounds();
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said.len(), 1);
        assert_eq!(session.take_sounds(), [SimSound::HailQuote], "now");
    }

    #[test]
    fn an_attack_quote_of_an_out_of_range_hail_quote_shows_an_empty_line_and_beeps() {
        for hail_quote in [0, 5] {
            let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
            session.space.traffic.npcs_mut()[0]
                .person
                .as_mut()
                .expect("Ace")
                .hail_quote = hail_quote;
            session.take_sounds();
            turn_ace(&mut session);
            let (said, _) = quote_tick(&catalog, &mut session, &[1]);
            assert_eq!(said, [""], "{hail_quote}");
            assert_eq!(session.take_sounds(), [SimSound::HailQuote]);
        }
    }
}
