//! Persons in flight: the session's side of [`person`](crate::person).
//!
//! **Appearing.** A system is populated with its persons as the session's
//! [`PersonRules`] say ([`Session::with_person_rules`], Nova's by
//! default), the pilot's persons gone for good kept away and each
//! person's `ActiveOn` tested through the session's
//! [`ControlBits`](crate::hire::ControlBits). A person NPC is named by
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
//! person may say its hail quote (`STR#` 7101 #`HailQuote`, its tags
//! read, or the `'STR '` 4999 + `HailQuote` of any open file in its place,
//! `_ShowPersonHailQuote` @0x445b8-0x445e1), on the trigger its `Flags` give: when it begins to attack the
//! player, now and then, and so on (see [`person`](crate::person)); the
//! flight shows each in its message line ([`Session::take_quotes`]).
//! A person escorting the player says none, nor does one gone for good
//! (@0x33c62), as one whose mission was accepted with `Flags` 0x0100
//! is. One of `Flags` 0x0400 says it only while its `LinkMission` is
//! available from a ship, checked in full (`_HandleShip`
//! @0x33dc7-0x33e31). "No quote showing" counts
//! only the hail quotes, the last said
//! [`QUOTE_SHOWN_TICKS`] or more ago. A person's quote state is never
//! saved, and "once" is once for its stay.
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

use std::rc::Rc;

use super::{Session, hire};
use crate::catalog::StringPatch;
use crate::catalog::{CommCatalog, MissionId, PersonId};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::hail::likes_player;
use crate::person::{
    ESCAPE_POD, Eligible, GRUDGE, HAIL_QUOTES, PersonRules, QUOTE_GAP_TICKS, QUOTE_ODDS,
    QUOTE_SHOWN_TICKS, QuoteTags, QuoteView, expand_tags, hail_quote_patch_id, hail_quote_text,
    quote_eligible,
};
use crate::traffic::npc::{Npc, NpcId};

/// A person's hail quote said, for the flight's message line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonQuote {
    /// The NPC that said it.
    pub npc: NpcId,
    /// Its words, its tags read.
    pub text: String,
}

/// The hail quotes' clock: the quote ticks so far, and the tick until
/// which a quote shows, keeping any other from being said at random.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct QuoteClock {
    pub(super) ticks: u64,
    quiet_until: u64,
}

impl Session {
    /// This session with `rules` deciding how persons appear (see
    /// [`person`](crate::person)): Nova's
    /// ([`NovaPersons`](crate::NovaPersons)) by default.
    #[must_use]
    pub fn with_person_rules(mut self, rules: Rc<dyn PersonRules>) -> Self {
        self.person_rules = hire::Shared(rules);
        self
    }

    /// How persons appear.
    #[must_use]
    pub fn person_rules(&self) -> &dyn PersonRules {
        &*self.person_rules.0
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
        if let Some(name) = self.mission_ship_text(npc.mission, |ships| &ships.name) {
            return Some(name);
        }
        match npc.person.and_then(|person| self.traffic.person(person.id)) {
            Some(person) => Some(&person.record.name),
            None => self.ship_name(npc.ship),
        }
    }

    /// `npc`'s subtitle, in place of its ship type's: while it is a
    /// marked mission ship, the mission's drawn subtitle unless that is
    /// empty, its person's never (@0x4b80f-0x4b86f); otherwise its
    /// person's, unless it has none or it is empty (@0x4b880).
    #[must_use]
    pub fn npc_subtitle(&self, npc: &Npc) -> Option<&str> {
        if npc.mission.is_some() {
            return self.mission_ship_text(npc.mission, |ships| &ships.subtitle);
        }
        let person = self.traffic.person(npc.person?.id)?;
        Some(person.record.subtitle.as_str()).filter(|subtitle| !subtitle.is_empty())
    }

    /// A tick of the persons' hail quotes in flight, each drawn on
    /// `chance` and read from `catalog`, its `'STR '` patch before
    /// `STR#` 7101 (@0x445b8-0x445e1; see the module docs and
    /// [`person`](crate::person)): first each attack quote queued, said
    /// with no draw; then, in order, each person NPC whose quote
    /// may be said draws `Rand(140)`, and says it when it is due now, or
    /// on a draw of 0 when no quote has shown for [`QUOTE_SHOWN_TICKS`]
    /// and its own last is more than [`QUOTE_GAP_TICKS`] back. While the
    /// ship is landed nothing happens. Then the quote clock moves on a
    /// tick: ours moves it once a frame in place of `TickCount`, and
    /// double time's second pass reads it unmoved
    /// ([`Session::tick_quotes_again`]).
    pub fn tick_quotes(&mut self, catalog: &(impl CommCatalog + ?Sized), chance: &mut dyn Chance) {
        if self.landed.is_some() {
            return;
        }
        self.roll_quotes(catalog, chance);
        self.quote_clock.ticks += 1;
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
        if self.landed.is_some() {
            return;
        }
        self.roll_quotes(catalog, chance);
    }

    /// The quote block of [`Session::tick_quotes`], the clock unmoved.
    fn roll_quotes(&mut self, catalog: &(impl CommCatalog + ?Sized), chance: &mut dyn Chance) {
        let lines = catalog.string_list(HAIL_QUOTES);
        self.say_queued_attack_quotes(catalog, &lines);
        let player_ai = self
            .ship_record(self.pilot.ship)
            .map_or(0, |record| record.inherent_ai);
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
                        .traffic
                        .person(person.id)
                        .and_then(|spawned| spawned.record.link_mission);
                    let view = QuoteView {
                        liked: likes_player(npc, &around, self.mission_hail(npc)),
                        player_jumping: self.jumping().is_some(),
                        player_ai,
                        mission_available: linked
                            .is_some_and(|mission| self.ship_mission_available(MissionId(mission))),
                    };
                    Some((npc.id, quote_eligible(npc, &person, &view, self.npcs())))
                })
                .collect()
        };
        let ticks = self.quote_clock.ticks;
        for (id, eligible) in decided {
            if eligible == Eligible::No {
                continue;
            }
            let draw = chance.below(QUOTE_ODDS);
            let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) else {
                continue;
            };
            let Some(person) = npc.person.as_mut() else {
                continue;
            };
            let quiet = ticks >= self.quote_clock.quiet_until;
            let rested = person
                .quoted_at
                .is_none_or(|at| ticks > at + QUOTE_GAP_TICKS);
            if eligible != Eligible::Now && !(draw == 0 && quiet && rested) {
                continue;
            }
            // `_HandleShip` keeps the time of its last quote (+0xac,
            // @0x33fba).
            person.quoted_at = Some(ticks);
            self.say_hail_quote(id, catalog, &lines);
        }
    }

    /// Says each attack quote queued, its line read from `catalog` or
    /// `lines`.
    fn say_queued_attack_quotes(
        &mut self,
        catalog: &(impl CommCatalog + ?Sized),
        lines: &[String],
    ) {
        for id in self.traffic.take_attack_quotes() {
            self.say_hail_quote(id, catalog, lines);
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
    /// here, as every one goes through `_ShowPersonHailQuote`.
    fn say_hail_quote(
        &mut self,
        id: NpcId,
        catalog: &(impl CommCatalog + ?Sized),
        lines: &[String],
    ) {
        let ticks = self.quote_clock.ticks;
        let Some(person) = self
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
        self.quote_clock.quiet_until = ticks + QUOTE_SHOWN_TICKS;
        let patch = hail_quote_patch_id(hail_quote)
            .map_or(StringPatch::Absent, |patch| catalog.plugin_string(patch));
        let line = hail_quote_text(hail_quote, patch, lines);
        let tags = QuoteTags {
            person: self
                .traffic
                .person(person)
                .map_or("", |person| &person.record.name),
            pilot: &self.pilot.name,
            ship_type: self.ship_name(self.pilot.ship).unwrap_or_default(),
            ship_name: self.pilot.ship_name().unwrap_or_default(),
            ranks: self.rank_names(),
        };
        let text = expand_tags(&line, &tags);
        self.quotes.push(PersonQuote { npc: id, text });
        self.sounds.push(crate::SimSound::HailQuote);
    }

    /// The hail quotes said since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_quotes(&mut self) -> Vec<PersonQuote> {
        std::mem::take(&mut self.quotes)
    }

    /// Person `id` is gone for good: it never appears again, and its
    /// grudge goes with it (`_LoadPilotData` @0x75e1e-0x75e2f). No save is
    /// due: the next one keeps it.
    pub(super) fn go(&mut self, id: PersonId) {
        self.pilot.gone_persons.insert(id);
        self.pilot.grudges.remove(&id);
    }

    /// NPC `id`, destroyed, is taken out of the system: a person without
    /// an escape pod is gone for good (`_HandleShipDisplay`
    /// @0x2dc4a-0x2dca1).
    pub(super) fn lose_destroyed(&mut self, id: NpcId) {
        let person = self
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| npc.person);
        if let Some(person) = person.filter(|person| person.flags & ESCAPE_POD == 0) {
            self.go(person.id);
        }
        self.traffic.remove(id);
    }

    /// NPC `id`, captured, no longer flies as its person, who is gone for
    /// good, escape pod or not (`_DoShipCapture` @0x41180).
    pub(super) fn lose_captured(&mut self, id: NpcId) {
        let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
        // A capture also unmarks a mission's special ship, its ships left
        // kept (`_DoShipCapture` @0x413c2).
        let person = npc.and_then(|npc| {
            npc.mission = None;
            npc.person.take()
        });
        if let Some(person) = person {
            self.go(person.id);
        }
    }

    /// Each person of `Flags` 0x0001 that a shot of the player's hit among
    /// `strikes` holds a grudge against it from now on, on the pilot too
    /// (`_DamageShip` @0x3be83-0x3beb3); an escort's shot does not count,
    /// nor does the player's death shockwave, which provokes nothing
    /// ([`Strike::provokes`]). No save is due.
    pub(super) fn hold_grudges(&mut self, strikes: &[Strike]) {
        for strike in strikes
            .iter()
            .filter(|strike| strike.by == ShipRef::Player && strike.provokes)
        {
            let ShipRef::Npc(id) = strike.ship else {
                continue;
            };
            let npc = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id);
            let Some(person) = npc.and_then(|npc| npc.person.as_mut()) else {
                continue;
            };
            if person.flags & GRUDGE != 0 {
                person.grudge = true;
                self.pilot.grudges.insert(person.id);
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
    /// player, who holds its fire on it.
    fn facing_ace(catalog: &FakePilotCatalog, pilot: Pilot) -> Session {
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.populate(catalog, &mut Draws::of(&ACE_ABOVE));
        assert_eq!(persons(&session), [Some(600)]);
        assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
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
        session.aboard = Some(super::super::Aboard {
            npc: NpcId(0),
            ship: ShipId(129),
            plunder: crate::board::Plunder::default(),
            captured: true,
        });
        session
    }

    #[test]
    fn a_captured_person_is_gone_for_good_escape_pod_or_not() {
        use crate::board::Assignment;
        for (flags, choice) in [
            (ESCAPE_POD, Assignment::Escort),
            (0, Assignment::Escort),
            (ESCAPE_POD, Assignment::MyShip),
        ] {
            let catalog = duel(flags, 0);
            let mut session = captured(&catalog);
            assert!(session.assign(choice, &mut NeverFires).is_some());
            assert!(session.pilot().gone(PersonId(600)), "{flags} {choice:?}");
            assert!(
                session.npcs().iter().all(|npc| npc.person.is_none()),
                "no person flies for the player: {choice:?}"
            );
        }
    }

    #[test]
    fn the_players_hit_on_a_person_of_0x0001_holds_its_grudge() {
        let catalog = duel(GRUDGE | ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(session.npcs()[0].reserves.shield.now < 30.0, "hit");
        assert!(session.npcs()[0].person.expect("Ace").grudge);
        assert!(session.pilot().grudge(PersonId(600)));
        assert!(!session.take_save_due(), "no save is due");
        let catalog = duel(ESCAPE_POD, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        for _ in 0..8 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert!(!session.npcs()[0].person.expect("Ace").grudge, "no 0x0001");
        assert!(!session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn an_escorts_hit_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        let hit = |by| Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by,
            damage: 1.0,
            downed: None,
            provokes: true,
        };
        session.hold_grudges(&[hit(ShipRef::Npc(NpcId(7)))]);
        assert!(!session.pilot().grudge(PersonId(600)));
        session.hold_grudges(&[hit(ShipRef::Player)]);
        assert!(session.pilot().grudge(PersonId(600)));
    }

    #[test]
    fn the_players_shockwave_holds_no_grudge() {
        let catalog = duel(GRUDGE, 0);
        let mut session = facing_ace(&catalog, ada(&catalog));
        session.hold_grudges(&[Strike {
            ship: ShipRef::Npc(NpcId(0)),
            by: ShipRef::Player,
            damage: 32.0,
            downed: None,
            provokes: false,
        }]);
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
            by: ShipRef::Player,
            damage: 9.0,
            downed: Some(crate::combat::Downed::Disabled),
            provokes: true,
        };
        let mut strikes = [
            downed(ShipRef::Npc(NpcId(0))),
            downed(ShipRef::Npc(NpcId(7))),
        ];
        session.refill_invincible(&mut strikes);
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
            .strikes
            .iter()
            .filter(|strike| strike.ship == ShipRef::Npc(NpcId(0)) && strike.damage > 0.0)
            .count();
        assert!(hits > 100, "the blaster hit it: {hits}");
        assert!(
            session.strikes.iter().all(|strike| strike.downed.is_none()),
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
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_rules(Rulebook::default().with_override(RuleKey::CommQuote, rule));
        session.populate(catalog, &mut NeverFires);
        if hostile {
            session.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        }
        session.target = Some(NpcId(0));
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
        session.target = Some(NpcId(0));
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answers");
        assert_eq!(view.portrait, Some(7800));
        let catalog = mixed("");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut NeverFires);
        session.target = Some(NpcId(0));
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
        session.traffic.npcs_mut()[1].person = None;
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
        session.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
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
        session.traffic.npcs_mut()[1].person = None;
        session.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
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
        session.traffic.npcs_mut()[1].person = None;
        session.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said, ["Halt, Cmdr! A Commander?"], "not captain");
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
        let ticks = session.quote_clock.ticks;
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
        assert_eq!(session.quote_clock.ticks, ticks, "the clock stands");
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
        session.traffic.npcs_mut()[1].goal = crate::ai::Goal::Attack(ShipRef::Player);
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
        session.offers.set_roll(crate::catalog::MissionId(130), 51);
        assert_eq!(
            quote_tick(&catalog, &mut session, &[1]),
            (vec![], vec![140]),
            "unavailable: Bee alone draws"
        );
        session.offers.set_roll(crate::catalog::MissionId(130), 50);
        let (said, asked) = quote_tick(&catalog, &mut session, &[0, 1]);
        assert_eq!(said, ["Ace: Prepare to die, Stock!"]);
        assert_eq!(asked, [140, 140]);
    }

    #[test]
    fn nothing_is_said_or_drawn_without_a_person() {
        let catalog = mixed("");
        let mut session = Session::start(&catalog).expect("starts");
        session.populate(&catalog, &mut Draws::of(&[]));
        for npc in session.traffic.npcs_mut() {
            npc.person = None;
        }
        assert_eq!(quote_tick(&catalog, &mut session, &[0]), (vec![], vec![]));
    }

    #[test]
    fn nothing_is_said_while_the_player_is_landed() {
        let (catalog, mut session) = quoting([0, 0]);
        session.landed = Some(crate::catalog::StellarId(128));
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
    /// already quoted and holding no grudge; Bee flies no person.
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
        session.traffic.npcs_mut()[1].person = None;
        let ace = session.traffic.npcs_mut()[0].person.as_mut().expect("Ace");
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
        session.traffic.npcs_mut()[0].mission = Some(crate::traffic::npc::MissionMark {
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
        session.traffic.npcs_mut()[1].person = None;
        session.traffic.npcs_mut()[0].goal = crate::ai::Goal::Attack(ShipRef::Player);
        session.take_sounds();
        let (said, _) = quote_tick(&catalog, &mut session, &[77]);
        assert_eq!(said.len(), 1);
        assert_eq!(session.take_sounds(), [SimSound::HailQuote], "now");
    }

    #[test]
    fn an_attack_quote_of_an_out_of_range_hail_quote_shows_an_empty_line_and_beeps() {
        for hail_quote in [0, 5] {
            let (catalog, mut session) = quoting([QUOTE_ON_ATTACK, 0]);
            session.traffic.npcs_mut()[0]
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
