//! Spawn availability as the last galaxy refresh read it
//! ([`RuleKey::SpawnAvailability`]).
//!
//! **What the original caches.** At each galaxy refresh,
//! `_PropagateMissionBitEffects` step 4 (@0x997c3-0x998ef) evaluates and
//! keeps a verdict for every record whose test the spawners read:
//!
//! - shïp: `+0xa41 = _EvalMissionBitTestString(+0x2ea AppearOn)`, or 0 for
//!   an unused slot (@0x997c3-0x99800, the eval @0x997e0);
//! - përs: `+0x622 = eval(+0x644 ActiveOn)`, or 0 when `+0x623` is clear
//!   (@0x99802-0x99840, the eval @0x99820);
//! - flët: `+0x122 = eval(+0x20 AppearOn)`, or 0 for ID -1
//!   (@0x99842-0x9987a, the eval @0x9985a);
//! - mïsn: `+0x16 = eval(+0x1c AvailBits)` (@0x9987c-0x998b2), which
//!   [`RuleKey::CachedAvailBits`] covers ([`Session::propagate_avail_bits`]);
//! - nëbu `ActiveOn` after these (see the `nebulae` module).
//!
//! The refresh runs at a landing (@0xa175f, @0xa1af2), a jump's arrival
//! before its days (@0x6bf3f), a new pilot and a load, and under
//! `visibility_refresh = bible` after every settled script as well; each
//! runs [`Session::propagate_cached_tests`].
//!
//! **Who reads the shïp and flët caches.** Every reader uses the cache
//! alone, with no live test: `_SelectShipFieldFromDude` (@0x662b, @0x6662,
//! @0x6716; reached from `_RandomShipSpawn` @0x3c19d, `_MissionDudeSpawn`
//! @0x3ce11/@0x3ce31, `_SpecificDudeSpawn` @0x3d141 and
//! `_LoadCurrentMissionData` @0xa0bc5/@0xa0bf3, the mission accept's
//! ship-type draw), `_HyperSpawnFleet` (@0x4217f the escort ship type,
//! @0x41cd3 the fleet; reached from `_HandleReinforcements` @0x390e0,
//! `_SpawnFleet` @0x4290a and `_EnterMoreShips` @0x43bd3/@0x43dcf),
//! `_SelectFleetFromSystem` (@0x6884) and `_SpawnFleet` (@0x4275b,
//! @0x428a5; reached from `_HyperShipSpawn` @0x4297e and
//! `_SetupShipsInSystem` @0x4328d). A debug key in `_HandlePlayer`
//! (@0x6e773) also reaches `_HyperSpawnFleet`; the engine has no such key.
//!
//! **Persons.** The only reader of përs +0x622 is `_SpawnPerson`
//! (@0x409b5), in its random-draw loop alone (a person argument above
//! 0x3fe: -1 from `_HyperShipSpawn` @0x4294e and `_SetupShipsInSystem`
//! @0x431fd), which then tests `ActiveOn` live again (@0x40b5d-0x40b6d):
//! a drawn person needs both verdicts. A system's person slots skip the
//! cache: `_SetupShipsInSystem` tests `ActiveOn` live (@0x4334d-0x4335d)
//! and spawns the person by ID (@0x433af), which skips the draw's checks
//! (@0x408f1-0x4094f), as the bounty hunter (0x3fe, `_SpawnBountyHunter`
//! @0x4470b) does.
//!
//! **Here.** Rather than a verdict per record, the session keeps the
//! player as the refresh read them ([`RefreshedFacts`]): a test's verdict
//! is a function of the pilot's facts alone, so testing a record against
//! the snapshot at a spawn gives the verdict the original cached, with no
//! fleet or person record needed at the refresh. By the engine, every
//! `AppearOn` and a drawn person's `ActiveOn` read it
//! ([`Session::appear_gate`], and the persons' world's `refreshed`); by
//! the other reading the player now does, as at any other test. The
//! snapshot is never saved: a load refreshes it.
//!
//! The shipyard, the outfitter, the exchange, the cröns, the öops, the
//! hire, the hail and the missions' own tests are no spawners and read
//! the player now.

use std::collections::BTreeSet;

use super::Session;
use super::control::{Facts, REGISTERED};
use crate::catalog::{OutfitId, SystemId};
use crate::control::{Bit, ControlBitSet, Gate, PilotFacts};
use crate::pilot::Gender;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// The player as a galaxy refresh read them: what a control-bit test
/// reads, kept as it was (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RefreshedFacts {
    /// The control bits.
    bits: ControlBitSet,
    /// The gender.
    gender: Gender,
    /// The outfits the player had, a carried fighter's dock outfit among
    /// them.
    outfits: BTreeSet<OutfitId>,
    /// The systems explored.
    explored: BTreeSet<SystemId>,
}

impl RefreshedFacts {
    /// The player as `facts` reads them now.
    pub(crate) fn of(facts: &Facts<'_>) -> Self {
        let pilot = facts.pilot;
        Self {
            bits: pilot.control_bits().clone(),
            gender: pilot.gender(),
            outfits: facts.counted_outfits().collect(),
            explored: pilot.explored().collect(),
        }
    }
}

impl PilotFacts for RefreshedFacts {
    fn bit(&self, bit: Bit) -> bool {
        self.bits.get(bit)
    }

    fn gender(&self) -> Gender {
        self.gender
    }

    /// [`REGISTERED`], as [`Facts`] has it.
    fn paid(&self, _days: u16) -> bool {
        REGISTERED
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.outfits.contains(&outfit)
    }

    fn explored(&self, system: SystemId) -> bool {
        self.explored.contains(&system)
    }
}

/// The player a spawn's `AppearOn` reads by `rules`: by the engine the
/// `refreshed` one, by the other reading the player `now`.
pub(crate) fn appear_pilot<'a>(
    rules: &Rulebook,
    refreshed: &'a RefreshedFacts,
    now: &'a dyn PilotFacts,
) -> &'a dyn PilotFacts {
    match rules.source_for(RuleKey::SpawnAvailability) {
        RuleSource::Engine => refreshed,
        RuleSource::Bible => now,
    }
}

impl Session {
    /// Takes the player as they are now as the refresh's snapshot, as
    /// `_PropagateMissionBitEffects` caches the shïp, përs and flët tests
    /// (@0x997c3-0x9987a).
    pub(crate) fn propagate_spawn_availability(&mut self) {
        self.run.refreshed = RefreshedFacts::of(&self.ship.facts(&self.pilot, &self.statics));
    }

    /// Every test the galaxy refresh caches: the missions' `AvailBits`
    /// ([`Session::propagate_avail_bits`]) and the spawners' tests
    /// ([`Session::propagate_spawn_availability`]).
    pub(crate) fn propagate_cached_tests(&mut self) {
        self.propagate_avail_bits();
        self.propagate_spawn_availability();
    }

    /// The session's control bits and the player a spawn's `AppearOn`
    /// reads ([`appear_pilot`]), as a [`Gate`].
    pub(crate) fn appear_gate(&self) -> Gate<'_> {
        Gate {
            control_bits: &*self.setup.control_bits.0,
            pilot: appear_pilot(&self.setup.rules, &self.run.refreshed, self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pilot::Pilot;
    use crate::testkit::catalog;

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    #[test]
    fn the_snapshot_answers_as_the_pilot_view_did_when_it_was_taken() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "Eve")
            .expect("starts")
            .with_gender(Gender::Female);
        pilot.outfits.insert(OutfitId(300), 1);
        pilot.outfits.insert(OutfitId(302), 0);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.set_control_bit(bit(12), true);
        let start = session.pilot().system();
        let snapshot = RefreshedFacts::of(&session.ship.facts(&session.pilot, &session.statics));
        let check = |facts: &dyn PilotFacts| {
            assert!(facts.bit(bit(12)));
            assert!(!facts.bit(bit(13)));
            assert_eq!(facts.gender(), Gender::Female);
            assert!(facts.paid(30) && facts.paid(0));
            assert!(facts.has_outfit(OutfitId(300)));
            assert!(!facts.has_outfit(OutfitId(301)));
            assert!(!facts.has_outfit(OutfitId(302)), "none of it");
            assert!(facts.explored(start));
            assert!(!facts.explored(SystemId(start.0 + 1)));
        };
        check(&session.ship.facts(&session.pilot, &session.statics));
        check(&snapshot);
        // A later change to the pilot leaves the snapshot as it was.
        session.set_control_bit(bit(12), false);
        session.set_control_bit(bit(13), true);
        session.pilot.outfits.remove(&OutfitId(300));
        session.pilot.explore(SystemId(start.0 + 1));
        check(&snapshot);
        assert!(
            !session
                .ship
                .facts(&session.pilot, &session.statics)
                .bit(bit(12))
        );
    }

    #[test]
    fn the_session_takes_its_snapshot_when_it_starts_and_at_each_propagation() {
        let catalog = catalog();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        assert_eq!(
            session.run.refreshed,
            RefreshedFacts::of(&session.ship.facts(&session.pilot, &session.statics))
        );
        session.set_control_bit(bit(7), true);
        assert!(!session.run.refreshed.bit(bit(7)), "not yet");
        session.propagate_spawn_availability();
        assert!(session.run.refreshed.bit(bit(7)));
        session.set_control_bit(bit(7), false);
        session.propagate_cached_tests();
        assert!(!session.run.refreshed.bit(bit(7)));
    }

    #[test]
    fn appear_on_reads_the_snapshot_by_the_engine_and_the_player_now_by_the_other_reading() {
        let catalog = catalog();
        let mut session =
            Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies");
        session.set_control_bit(bit(7), true);
        let test = crate::control::Test::parse("b7");
        assert!(!session.appear_gate().allows(&test), "by the engine");
        let session = session.with_rules(
            Rulebook::default().with_override(RuleKey::SpawnAvailability, RuleSource::Bible),
        );
        assert!(session.appear_gate().allows(&test), "by the other reading");
    }

    // A bit set between refreshes, through the session's flow.

    use crate::catalog::{
        DudeId, DudeRecord, EscortRecord, FleetId, FleetRecord, GovtId, PersonId, ShipId,
        SystemTraffic,
    };
    use crate::chance::{Chance, NeverFires};
    use crate::control::Test;
    use crate::person::{PersonRoll, PersonRules};
    use crate::testkit::{Draws, FAST, FakePilotCatalog, person, ship};
    use std::rc::Rc;

    /// The traffic of one ship on average, of düde 128 (ship 129, AI 3,
    /// govt 140).
    fn one_ship() -> SystemTraffic {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        SystemTraffic {
            dude_types,
            avg_ships: 1,
            persons: Default::default(),
        }
    }

    /// Systems 130 and 131, linked, of [`one_ship`] each, governed by 140
    /// and 141, and `flët`s 130, linked to govt 140, and 131, linked to
    /// govt 141, both appearing on b7: each a lead of ship 129 and one to
    /// three escorts of ship 130, of govt 150.
    fn gated_fleets() -> FakePilotCatalog {
        let fleet = |id: i16, govt: i16| FleetRecord {
            id: FleetId(id),
            lead: Some(ShipId(129)),
            escorts: vec![EscortRecord {
                ship: ShipId(130),
                min: 1,
                max: 3,
            }],
            govt: Some(GovtId(150)),
            link_syst: 10_000 + (govt - 128),
            appear_on: Test::parse("b7"),
            quote: None,
            flags: 0,
        };
        let mut catalog = FakePilotCatalog {
            traffic: vec![(SystemId(130), one_ship()), (SystemId(131), one_ship())],
            dudes: vec![(
                DudeId(128),
                DudeRecord {
                    ai_type: 3,
                    govt: Some(GovtId(140)),
                    ships: vec![(ShipId(129), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ship_records: vec![ship(129, FAST), ship(130, FAST)],
            fleets: vec![fleet(130, 140), fleet(131, 141)],
            ..crate::testkit::catalog()
        };
        catalog.star_map[0].govt = Some(GovtId(140));
        catalog.star_map[1].govt = Some(GovtId(141));
        catalog.star_map[0].links.push(SystemId(131));
        catalog
    }

    /// A setup pass that draws the `LinkSyst` fleet in `slot`, with three
    /// escorts.
    fn fleet_draws(slot: u32) -> Draws {
        Draws::of(&[1, 0, slot, 0, 2])
    }

    /// How many ships of the fleets' government fly.
    fn fleet_ships(session: &Session) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.govt == Some(GovtId(150)))
            .count()
    }

    /// Jumps to 131, its arrival's setup drawing fleet 131's slot.
    fn jump_to_131(session: &mut Session, catalog: &FakePilotCatalog) {
        let mut draws: Vec<u32> = vec![0];
        draws.extend(std::iter::repeat_n(99, crate::mission::ROLLED));
        draws.extend([1, 0, 3, 0, 2]);
        crate::testkit::jump_with(session, catalog, 131, &mut Draws::of(&draws)).expect("arrives");
    }

    fn by(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::SpawnAvailability, source)
    }

    #[test]
    fn by_the_engine_a_bit_set_between_refreshes_brings_a_fleet_only_from_the_next() {
        let catalog = gated_fleets();
        let mut session = Session::start(&catalog).expect("starts");
        session.set_control_bit(bit(7), true);
        session.populate(&catalog, &mut fleet_draws(2));
        assert_eq!(fleet_ships(&session), 0, "the start's refresh saw b7 clear");
        jump_to_131(&mut session, &catalog);
        assert_eq!(fleet_ships(&session), 4, "the arrival refreshed (@0x6bf3f)");
        // Cleared in flight, the fleet keeps coming until the next refresh.
        session.set_control_bit(bit(7), false);
        session.populate(&catalog, &mut fleet_draws(3));
        assert_eq!(fleet_ships(&session), 4);
    }

    /// Takes every NPC out, so the system is short of its average and the
    /// next traffic tick rolls for an arrival.
    fn empty_the_system(session: &mut Session) {
        let here: Vec<_> = session.npcs().iter().map(|npc| npc.id).collect();
        for id in here {
            session.space.traffic.remove(id);
        }
    }

    #[test]
    fn a_hyperspace_arrival_in_flight_reads_a_fleets_appear_on_as_the_last_refresh_saw_it() {
        // `_HyperShipSpawn` → `_SpawnFleet` reads flët +0x122 (@0x4275b,
        // @0x428a5): by the engine b7, set in flight, is unseen until the
        // next refresh.
        let catalog = gated_fleets();
        // An arrival's roll comes, as the `LinkSyst` fleet of slot 2,
        // fleet 130, with three escorts.
        let arrival = || Draws::of(&[0, 1, 0, 2, 0, 2]);
        for (source, at_once) in [(RuleSource::Engine, 0), (RuleSource::Bible, 4)] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(by(source));
            session.populate(&catalog, &mut NeverFires);
            empty_the_system(&mut session);
            session.set_control_bit(bit(7), true);
            session.tick_traffic(&catalog, &NovaAi::default(), &mut arrival());
            assert_eq!(fleet_ships(&session), at_once, "{source:?}");
            if at_once == 0 {
                assert!(session.npcs().is_empty(), "nothing came");
                // The galaxy refresh's own step, as a landing or an
                // arrival runs it.
                session.propagate_cached_tests();
                session.tick_traffic(&catalog, &NovaAi::default(), &mut arrival());
                assert_eq!(fleet_ships(&session), 4, "{source:?}: refreshed");
            }
        }
    }

    #[test]
    fn by_the_other_reading_a_bit_set_between_refreshes_brings_a_fleet_at_once() {
        let catalog = gated_fleets();
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(by(RuleSource::Bible));
        session.populate(&catalog, &mut fleet_draws(2));
        assert_eq!(fleet_ships(&session), 0);
        session.set_control_bit(bit(7), true);
        session.populate(&catalog, &mut fleet_draws(2));
        assert_eq!(fleet_ships(&session), 4);
        session.set_control_bit(bit(7), false);
        session.populate(&catalog, &mut fleet_draws(2));
        assert_eq!(fleet_ships(&session), 0);
    }

    #[test]
    fn a_load_with_the_bit_saved_brings_the_fleet_at_once() {
        let catalog = gated_fleets();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.bits_mut().set(bit(7));
        let mut session = Session::load(&catalog, pilot, &Rulebook::default()).expect("loads");
        session.populate(&catalog, &mut fleet_draws(2));
        assert_eq!(fleet_ships(&session), 4, "the load refreshed");
    }

    /// [`gated_fleets`] with no fleet, ship 129 appearing on b7.
    fn gated_dude() -> FakePilotCatalog {
        let mut catalog = gated_fleets();
        catalog.fleets.clear();
        catalog.ship_records[0].appear_on = Test::parse("b7");
        catalog
    }

    #[test]
    fn a_dude_ship_types_appear_on_reads_the_last_refresh_by_the_engine_and_now_by_the_other() {
        let catalog = gated_dude();
        for (source, at_once) in [(RuleSource::Engine, 0), (RuleSource::Bible, 1)] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(by(source));
            session.set_control_bit(bit(7), true);
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(session.npcs().len(), at_once, "{source:?}");
            crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
            assert_eq!(session.npcs().len(), 1, "{source:?}: refreshed");
        }
    }

    #[test]
    fn by_the_engine_a_landings_refresh_brings_a_dude_ship_type_set_in_flight_to_the_take_off() {
        let catalog = gated_dude();
        let land_and_settle = |session: &mut Session| {
            crate::testkit::land_now(session).expect("lands");
            let _ = session.settle_script(&catalog, &mut NeverFires);
        };
        // Set in flight: the landing's refresh (@0xa175f, @0xa1af2) sees it.
        let mut session = Session::start(&catalog).expect("starts");
        session.set_control_bit(bit(7), true);
        land_and_settle(&mut session);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.npcs().len(), 1, "the landing refreshed");
        // Set once landed: the take-off comes before the next refresh.
        let mut session = Session::start(&catalog).expect("starts");
        land_and_settle(&mut session);
        session.set_control_bit(bit(7), true);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.npcs().len(), 0, "set since the landing's refresh");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(session.npcs().len(), 1, "the arrival refreshed");
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
            true
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

    /// [`gated_dude`] with ship 129 always appearing and person 600 of
    /// ship 129, `ActiveOn` b7, drawn in every system or, `slotted`, in
    /// 130's slot alone.
    fn gated_person(slotted: bool) -> FakePilotCatalog {
        let mut catalog = gated_dude();
        catalog.ship_records[0].appear_on = Test::default();
        catalog.persons = vec![crate::catalog::PersonRecord {
            active_on: Test::parse("b7"),
            link_syst: if slotted { 0 } else { -1 },
            ..person(600, 129)
        }];
        if slotted {
            catalog.traffic[0].1.avg_ships = 0;
            catalog.traffic[0].1.persons[0] = (Some(PersonId(600)), 100);
        }
        catalog
    }

    fn persons(session: &Session) -> Vec<i16> {
        session
            .npcs()
            .iter()
            .filter_map(|npc| npc.person.as_ref().map(|person| person.id.0))
            .collect()
    }

    #[test]
    fn a_drawn_person_needs_its_active_on_at_the_last_refresh_by_the_engine() {
        let catalog = gated_person(false);
        for (source, at_once) in [(RuleSource::Engine, vec![]), (RuleSource::Bible, vec![600])] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(by(source))
                .with_person_rules(Rc::new(FirstComes));
            session.set_control_bit(bit(7), true);
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(persons(&session), at_once, "{source:?}");
            crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
            assert_eq!(persons(&session), [600], "{source:?}: refreshed");
            // Cleared in flight, the live test takes it out at once.
            session.set_control_bit(bit(7), false);
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(persons(&session), Vec::<i16>::new(), "{source:?}: cleared");
        }
    }

    #[test]
    fn a_slot_person_reads_its_active_on_now_under_either_reading() {
        let catalog = gated_person(true);
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(by(source))
                .with_person_rules(Rc::new(FirstComes));
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(persons(&session), Vec::<i16>::new(), "{source:?}");
            session.set_control_bit(bit(7), true);
            session.populate(&catalog, &mut NeverFires);
            assert_eq!(persons(&session), [600], "{source:?}: at once");
        }
    }

    use super::super::mission_ships::tests::{
        Names, forget_uniform_type, hailing_ace, marked, marked_types, person_here, settle,
        ships_of, swapping, two_types,
    };
    use crate::ai::NovaAi;
    use crate::catalog::{MissionId, StellarId};
    use crate::person::REPLACED_BY_MISSION_SHIP;
    use crate::testkit::land_now;

    // A mission ship's `AppearOn` at its spawn (the `mission_ships`
    // module): b5 set after the landing's refresh, ship 129 appearing on
    // !b5 and 200 on b5, by the engine the spawn draws 129 and by the other
    // reading 200.

    /// Each reading of [`RuleKey::SpawnAvailability`], and the one type
    /// a `düde` ship of [`gate_on_b5`] spawns as under it.
    const B5_SINCE_THE_REFRESH: [(RuleSource, i16); 2] =
        [(RuleSource::Engine, 129), (RuleSource::Bible, 200)];

    /// Ship 129 appearing on !b5 and ship 200 on b5.
    fn gate_on_b5(catalog: &mut FakePilotCatalog) {
        for record in &mut catalog.ship_records {
            match record.id.0 {
                129 => record.appear_on = Test::parse("!b5"),
                200 => record.appear_on = Test::parse("b5"),
                _ => {}
            }
        }
    }

    /// [`two_types`] with [`gate_on_b5`].
    fn gated_two_types(flags: u16, start: i16) -> FakePilotCatalog {
        let mut catalog = two_types(flags, start);
        gate_on_b5(&mut catalog);
        catalog
    }

    /// A session of `catalog` under `source` of
    /// [`RuleKey::SpawnAvailability`], docked at 128, b5 set since the
    /// landing's refresh, with mission 140 accepted into slot 0.
    fn accepted_since_the_refresh(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Names))
            .with_rules(by(source));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        session.set_control_bit(bit(5), true);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, catalog);
        session
    }

    #[test]
    fn the_setups_draws_read_appear_on_as_the_last_galaxy_refresh_saw_it_by_the_engine() {
        // `_MissionDudeSpawn` reads shïp +0xa41 (@0x3ce11/@0x3ce31).
        let catalog = gated_two_types(0, 0);
        for (source, drawn) in B5_SINCE_THE_REFRESH {
            let mut session = accepted_since_the_refresh(&catalog, source);
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            assert_eq!(marked_types(&session), [ShipId(drawn); 3], "{source:?}");
        }
    }

    #[test]
    fn a_0x0800_slots_draw_at_its_setup_reads_appear_on_as_the_last_galaxy_refresh_saw_it() {
        let catalog = gated_two_types(0x0800, 0);
        for (source, drawn) in B5_SINCE_THE_REFRESH {
            let mut session = accepted_since_the_refresh(&catalog, source);
            forget_uniform_type(&mut session);
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            assert_eq!(
                ships_of(&session).uniform_type,
                Some(ShipId(drawn)),
                "{source:?}"
            );
            assert_eq!(marked_types(&session), [ShipId(drawn); 3], "{source:?}");
        }
    }

    #[test]
    fn later_arrivals_draws_read_appear_on_as_the_last_galaxy_refresh_saw_it_by_the_engine() {
        // `_EnterMoreShips` → `_SpecificDudeSpawn` reads shïp +0xa41
        // (@0x3d141).
        let catalog = gated_two_types(0, 1);
        for (source, drawn) in B5_SINCE_THE_REFRESH {
            let mut session = accepted_since_the_refresh(&catalog, source);
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            assert!(marked(&session).is_empty(), "{source:?}: none at setup");
            for _ in 0..199 {
                session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
            }
            assert_eq!(marked_types(&session), [ShipId(drawn); 3], "{source:?}");
        }
    }

    #[test]
    fn a_swaps_draw_reads_appear_on_as_the_last_galaxy_refresh_saw_it_by_the_engine() {
        // Without 0x0800 the swapped-in ship's type is drawn from the düde
        // (`_MissionDudeSpawn` @0x3cdea), reading shïp +0xa41.
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].flags = 0;
        gate_on_b5(&mut catalog);
        for (source, drawn) in B5_SINCE_THE_REFRESH {
            let mut session = hailing_ace(&catalog).with_rules(by(source));
            session.set_control_bit(bit(5), true);
            assert_eq!(
                session.accept_mission(MissionId(140), &mut NeverFires),
                Ok(0)
            );
            settle(&mut session, &catalog);
            assert!(!person_here(&session), "{source:?}: swapped");
            assert_eq!(marked_types(&session), [ShipId(drawn)], "{source:?}");
        }
    }
}
