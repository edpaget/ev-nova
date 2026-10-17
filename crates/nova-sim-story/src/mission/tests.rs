use std::cell::{OnceCell, RefCell};

use super::*;
use crate::catalog::{GovtRecord, MissionRecord};
use crate::chance::NeverFires;
use crate::control::{ControlBits, FreshPilot, PilotFacts, Test, TestExpr};
use crate::govt::Governments;
use crate::hyperspace::StarMap;
use crate::landing::StellarFlags;
use crate::market::DOMINATED;
use crate::rulebook::{RuleKey, RuleSource};
use crate::rules::Verdict;
use crate::story::Whereabouts;
use nova_sim_base::fixture::{Draws, Scripted};
use nova_sim_catalog::fixture::{govt, mission, planet, star};

const FED: GovtId = GovtId(128);
const CIVVIES: GovtId = GovtId(129);
const PIRATES: GovtId = GovtId(130);

/// The Federation (class 1, allied with 13, at war with 9); the Civvies
/// (class 13); the pirates (class 9).
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            classes: [1, -1, -1, -1],
            allies: [13, -1, -1, -1],
            enemies: [9, -1, -1, -1],
            ..govt(128)
        },
        GovtRecord {
            classes: [13, -1, -1, -1],
            ..govt(129)
        },
        GovtRecord {
            classes: [9, -1, -1, -1],
            ..govt(130)
        },
    ])
}

/// Stellar `id` in `system`, inhabited and whole, of `govt`.
fn at(system: i16, id: i16, govt: Option<GovtId>) -> Stellar {
    Stellar {
        id: StellarId(id),
        system: SystemId(system),
        flags: StellarFlags::CAN_LAND,
        flags2: 0,
        govt,
        destroyed: false,
        listed_at_place: true,
        index_govt: None,
    }
}

/// Each of `stellars` placed as it is held: its system both holds it and
/// is what `_FindSystemFromStellar` gives it.
fn held_where_they_are(stellars: &[Stellar]) -> Vec<Placed> {
    let mut placed: Vec<Placed> = stellars
        .iter()
        .map(|stellar| Placed {
            id: stellar.id,
            at: Whereabouts {
                holder: Some(stellar.system),
                found: Some(stellar.system),
            },
        })
        .collect();
    placed.sort_by_key(|placed| placed.id);
    placed
}

/// The landing site of stellar `id`, the Federation's.
fn docked_at(id: i16) -> LandingSite {
    LandingSite {
        govt: Some(FED),
        ..planet(id, 0.0, 0.0)
    }
}

/// The galaxy of a visit: system 130, the player's, lists 131 and 135;
/// 132 is far; 134 is at 130's place. Stellar 128, docked at, and 129
/// are the Federation's in 130; 140 is in 131; 150 (the Federation's),
/// 151 (the Civvies') and 152 (uninhabited) are in 132; 160 is in 134.
struct Galaxy {
    stellars: Vec<Stellar>,
    /// Where every listed stellar is: set by a test, else each of
    /// `stellars` held by its system and found there, at the first visit.
    placed: OnceCell<Vec<Placed>>,
    map: StarMap,
    govts: Governments,
    rules: Rulebook,
}

impl Galaxy {
    fn new() -> Self {
        let mut uninhabited = at(132, 152, None);
        uninhabited.flags |= StellarFlags::UNINHABITED;
        Self {
            stellars: vec![
                at(130, 128, Some(FED)),
                at(130, 129, Some(FED)),
                at(131, 140, None),
                at(132, 150, Some(FED)),
                at(132, 151, Some(CIVVIES)),
                uninhabited,
                at(134, 160, None),
            ],
            placed: OnceCell::new(),
            map: StarMap::new(vec![
                star(130, (0.0, 0.0), &[131, 135]),
                star(131, (100.0, 0.0), &[]),
                star(132, (200.0, 0.0), &[]),
                star(134, (0.0, 0.0), &[]),
                star(135, (300.0, 0.0), &[]),
            ]),
            govts: govts(),
            rules: Rulebook::default(),
        }
    }

    /// The galaxy, its random -2 drawn and its distance told by the
    /// Bible's readings of [`RuleKey::MissionStellarDraw`] and
    /// [`RuleKey::MissionStellarDistance`]: phase 1's draw.
    fn by_bible() -> Self {
        Self {
            rules: Rulebook::default()
                .with_override(RuleKey::MissionStellarDraw, RuleSource::Bible)
                .with_override(RuleKey::MissionStellarDistance, RuleSource::Bible),
            ..Self::new()
        }
    }

    /// A visit docked at 128 in 130, by a pilot of a clean record and no
    /// rating in ship 128 (`InherentAI` 1, of no government) with a full
    /// tank (300), 20 tons free and 1000 credits, contributing nothing,
    /// on 23 June 1177.
    fn visit(&self) -> Visit<'_> {
        Visit {
            docked: Some(docked_at(128)),
            system: SystemId(130),
            world: World {
                stellars: &self.stellars,
                placed: self
                    .placed
                    .get_or_init(|| held_where_they_are(&self.stellars)),
                map: &self.map,
                govts: &self.govts,
                draw: self.rules.source_for(RuleKey::MissionStellarDraw),
                distance: self.rules.source_for(RuleKey::MissionStellarDistance),
                ally_index: self.rules.source_for(RuleKey::MissionAllyIndex),
            },
            player: Player {
                record: 0,
                rating: 0,
                ship: ShipId(128),
                inherent_ai: 1,
                inherent_govt: None,
                combat_govt: None,
                contributed: 0,
                fuel: 300.0,
                free_cargo: 20,
                cash: 1000,
            },
            active: &[],
            gate: Gate::FRESH,
            rules: &self.rules,
            today: today(),
        }
    }
}

fn today() -> GameDate {
    GameDate::new(1177, 6, 23).expect("a date")
}

/// Every roll 1, so every `AvailRandom` from 1 passes.
fn ones() -> RollTable {
    let mut table = RollTable::default();
    table.draw(&mut NeverFires);
    table
}

/// Why `mission` is not offered on `visit` at its own venue, every rule's
/// refusal in order, with `rolls`.
fn refusals_with(visit: &Visit, mission: &MissionRecord, rolls: &RollTable) -> Vec<Refusal> {
    let facts = AvailFacts {
        visit,
        mission,
        venue: Venue::of(mission.avail_loc),
        rolls,
        cached_avail_bits: None,
    };
    availability()
        .all_refusals(&facts)
        .into_iter()
        .map(|(_, verdict)| match verdict {
            Verdict::Refuses(refusal) => refusal,
            Verdict::Draws => panic!("no availability check draws"),
        })
        .collect()
}

/// Why `mission` is not offered on `visit`, every roll 1.
fn refusals(visit: &Visit, mission: &MissionRecord) -> Vec<Refusal> {
    refusals_with(visit, mission, &ones())
}

/// Mission 130 with `edit` made.
fn with(edit: impl FnOnce(&mut MissionRecord)) -> MissionRecord {
    let mut record = mission(130);
    edit(&mut record);
    record
}

/// Whether mission 130 with `edit` made passes on [`Galaxy::visit`] as
/// `adjust` changes it, or else its refusals.
fn offered(adjust: impl FnOnce(&mut Visit), edit: impl FnOnce(&mut MissionRecord)) -> Vec<Refusal> {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    adjust(&mut visit);
    refusals(&visit, &with(edit))
}

/// Mission 130 with `edit` made, on the plain visit.
fn plainly(edit: impl FnOnce(&mut MissionRecord)) -> Vec<Refusal> {
    offered(|_| {}, edit)
}

#[test]
fn a_plain_mission_is_offered_at_the_computer() {
    assert_eq!(plainly(|_| {}), []);
}

#[test]
fn no_check_draws_and_each_names_the_original_it_reproduces() {
    let checks = availability();
    let descriptors: Vec<_> = checks.descriptors().collect();
    assert_eq!(descriptors.len(), 17);
    for descriptor in descriptors {
        assert!(!descriptor.draws(), "{}", descriptor.name());
        assert_eq!(descriptor.reads(), None, "{}", descriptor.name());
        assert!(
            descriptor.reproduces().contains("@0x9b"),
            "{}",
            descriptor.name()
        );
    }
}

#[test]
fn all_refusals_lists_every_failing_field_with_its_descriptor() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    visit.player.fuel = 50.0;
    let refused = with(|m| {
        m.avail_stel = 129;
        m.avail_rating = 5;
        m.flags = NEEDS_FUEL;
        m.travel_stel = 10002;
    });
    let facts = AvailFacts {
        visit: &visit,
        mission: &refused,
        venue: Venue::Computer,
        rolls: &ones(),
        cached_avail_bits: None,
    };
    let named: Vec<(&str, Verdict<Refusal>)> = availability()
        .all_refusals(&facts)
        .into_iter()
        .map(|(descriptor, verdict)| (descriptor.name(), verdict))
        .collect();
    assert_eq!(
        named,
        [
            ("fuel", Verdict::Refuses(Refusal::Fuel)),
            ("AvailStel", Verdict::Refuses(Refusal::AvailStel)),
            ("AvailRating", Verdict::Refuses(Refusal::AvailRating)),
            ("stellars found", Verdict::Refuses(Refusal::TravelStel)),
        ]
    );
    assert_eq!(
        availability().first_refusal(&facts, &mut NeverFires),
        Err(Refusal::Fuel)
    );
}

#[test]
fn the_facts_hand_over_the_visits_rule_set() {
    let rules = Rulebook::default().with_override(
        crate::rulebook::RuleKey::DayRolls,
        crate::rulebook::RuleSource::Bible,
    );
    let galaxy = Galaxy {
        rules,
        ..Galaxy::new()
    };
    let visit = galaxy.visit();
    let mission = mission(130);
    let facts = AvailFacts {
        visit: &visit,
        mission: &mission,
        venue: Venue::Computer,
        rolls: &ones(),
        cached_avail_bits: None,
    };
    assert_eq!(facts.rulebook(), &rules);
}

#[test]
fn an_avail_stel_of_minus_32000_or_below_is_never_offered() {
    assert!(plainly(|m| m.avail_stel = -32000).contains(&Refusal::Unoffered));
    assert!(plainly(|m| m.avail_stel = i16::MIN).contains(&Refusal::Unoffered));
    assert!(!plainly(|m| m.avail_stel = -31999).contains(&Refusal::Unoffered));
}

#[test]
fn avail_loc_0_is_the_computers_and_every_other_but_minus_1_and_2_outside() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let at = |venue, avail_loc| {
        let mission = with(|m| m.avail_loc = avail_loc);
        let facts = AvailFacts {
            visit: &visit,
            mission: &mission,
            venue,
            rolls: &ones(),
            cached_avail_bits: None,
        };
        availability().first_refusal(&facts, &mut NeverFires)
    };
    assert_eq!(at(Venue::Computer, 0), Ok(()));
    for avail_loc in [-1, 1, 2, 3, 6] {
        assert_eq!(
            at(Venue::Computer, avail_loc),
            Err(Refusal::AvailLoc),
            "{avail_loc}"
        );
    }
    for avail_loc in [1, 3, 4, 5, 6, 7] {
        assert_eq!(at(Venue::Outside, avail_loc), Ok(()), "{avail_loc}");
    }
    for avail_loc in [-1, 0, 2] {
        assert_eq!(
            at(Venue::Outside, avail_loc),
            Err(Refusal::AvailLoc),
            "{avail_loc}"
        );
    }
    assert_eq!(Venue::of(0), Venue::Computer);
    assert_eq!(Venue::of(1), Venue::Outside);
    assert_eq!(Venue::of(-1), Venue::Outside);
}

/// The refusals of mission 130 offered from a ship (`AvailLoc` 2) with
/// `edit` made, on the plain visit as `adjust` changes it.
fn from_a_ship(
    adjust: impl FnOnce(&mut Visit),
    edit: impl FnOnce(&mut MissionRecord),
) -> Vec<Refusal> {
    offered(adjust, |m| {
        m.avail_loc = SHIP;
        edit(m);
    })
}

#[test]
fn a_ship_offers_avail_loc_2_alone_and_only_a_ship_does() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let at = |venue, avail_loc| {
        let mission = with(|m| m.avail_loc = avail_loc);
        let facts = AvailFacts {
            visit: &visit,
            mission: &mission,
            venue,
            rolls: &ones(),
            cached_avail_bits: None,
        };
        availability().first_refusal(&facts, &mut NeverFires)
    };
    assert_eq!(at(Venue::Ship, 2), Ok(()));
    for avail_loc in [-1, 0, 1, 3, 6] {
        assert_eq!(
            at(Venue::Ship, avail_loc),
            Err(Refusal::AvailLoc),
            "{avail_loc}"
        );
    }
    assert_eq!(at(Venue::Computer, 2), Err(Refusal::AvailLoc));
    assert_eq!(at(Venue::Outside, 2), Err(Refusal::AvailLoc));
    assert_eq!(Venue::of(2), Venue::Ship);
    assert_eq!(Venue::ORDER, [Venue::Computer, Venue::Outside], "no ship");
}

#[test]
fn from_a_ship_avail_stel_is_not_read_even_with_no_stellar_docked() {
    for code in [129, 10001, 32000, -2] {
        assert_eq!(from_a_ship(|_| {}, |m| m.avail_stel = code), [], "{code}");
        assert_eq!(
            from_a_ship(|visit| visit.docked = None, |m| m.avail_stel = code),
            [],
            "{code} in flight"
        );
    }
    assert_eq!(
        offered(|visit| visit.docked = None, |_| {}),
        [Refusal::AvailStel],
        "the computer with no stellar docked"
    );
    assert_eq!(
        offered(|visit| visit.docked = None, |m| m.avail_loc = 1),
        [Refusal::AvailStel],
        "outside"
    );
}

#[test]
fn from_a_ship_an_avail_record_of_domination_needs_a_record_that_low() {
    let record_of = |record, avail_record| {
        from_a_ship(
            |visit| visit.player.record = record,
            |m| m.avail_record = avail_record,
        )
    };
    assert_eq!(record_of(-32000, -32000), []);
    assert_eq!(record_of(i16::MIN, -32001), []);
    assert_eq!(record_of(-31999, -32000), [Refusal::AvailRecord]);
    assert_eq!(record_of(-32000, -32001), [Refusal::AvailRecord]);
    assert_eq!(record_of(-10, -10), [], "as at a stellar above it");
    assert_eq!(record_of(-9, -10), [Refusal::AvailRecord]);
    assert_eq!(record_of(9, 10), [Refusal::AvailRecord]);
    assert_eq!(record_of(-500, 0), []);
    assert_eq!(
        from_a_ship(
            |visit| {
                visit.player.record = -31999;
                dominate_docked(visit);
            },
            |m| m.avail_record = -32000
        ),
        [Refusal::AvailRecord],
        "a dominated stellar does not stand in for the record"
    );
}

#[test]
fn from_a_ship_any_cargo_above_0_needs_room_whatever_flags2_says() {
    let roomy = |free, qty, flags2| {
        from_a_ship(
            |visit| visit.player.free_cargo = free,
            |m| {
                m.flags2 = flags2;
                m.cargo_qty = qty;
            },
        )
    };
    assert_eq!(roomy(20, 20, 0), []);
    assert_eq!(roomy(19, 20, 0), [Refusal::CargoSpace]);
    assert_eq!(roomy(19, 20, NEEDS_ROOM), [Refusal::CargoSpace]);
    assert_eq!(roomy(-5, 1, 0), [Refusal::CargoSpace]);
    assert_eq!(roomy(-5, 0, 0), [], "no tons");
    assert_eq!(roomy(-11, -10, NEEDS_ROOM), [], "a range passes");
    assert_eq!(roomy(-11, -1, 0), []);
}

#[test]
fn a_fixed_destination_or_return_at_the_players_place_refuses_unless_avail_stel_is_a_stellar() {
    for code in [129, 160] {
        assert_eq!(
            plainly(|m| m.travel_stel = code),
            [Refusal::GoesHere],
            "{code}: in 130, or 134 at its place"
        );
        assert_eq!(plainly(|m| m.return_stel = code), [Refusal::GoesHere]);
        assert_eq!(
            plainly(|m| {
                m.travel_stel = code;
                m.avail_stel = 128;
            }),
            [],
            "{code}: AvailStel 128 names a stellar"
        );
    }
    assert_eq!(plainly(|m| m.travel_stel = 140), [], "next door");
    assert_eq!(
        plainly(|m| m.travel_stel = 2000),
        [],
        "in no visible system"
    );
    assert_eq!(
        plainly(|m| {
            m.travel_stel = 129;
            m.avail_stel = 2176;
        }),
        [Refusal::GoesHere, Refusal::AvailStel],
        "2176 names no stellar"
    );
    assert_eq!(
        plainly(|m| {
            m.travel_stel = 129;
            m.avail_stel = 127;
        }),
        [Refusal::GoesHere, Refusal::AvailStel]
    );
}

/// Mission 130 with `edit` made, on `galaxy`'s plain visit.
fn plainly_on(galaxy: &Galaxy, edit: impl FnOnce(&mut MissionRecord)) -> Vec<Refusal> {
    refusals(&galaxy.visit(), &with(edit))
}

#[test]
fn a_fixed_stop_held_by_no_system_refuses_by_the_system_find_system_from_stellar_gives() {
    // 134, hidden say, is at the player's 130's place and alone lists 2000.
    let here = placing(2000, None, Some(134));
    assert_eq!(
        plainly_on(&here, |m| m.travel_stel = 2000),
        [Refusal::GoesHere],
        "@0x9b225"
    );
    assert_eq!(
        plainly_on(&here, |m| m.return_stel = 2000),
        [Refusal::GoesHere],
        "@0x9b294"
    );
    let elsewhere = placing(2000, None, Some(131));
    assert_eq!(plainly_on(&elsewhere, |m| m.travel_stel = 2000), []);
    assert_eq!(plainly_on(&elsewhere, |m| m.return_stel = 2000), []);
    let nowhere = placing(2000, None, None);
    assert_eq!(
        plainly_on(&nowhere, |m| m.travel_stel = 2000),
        [],
        "no system at all passes"
    );
}

#[test]
fn a_fixed_stop_rescued_into_the_players_system_refuses() {
    let galaxy = placing(2000, Some(130), Some(131));
    assert_eq!(
        plainly_on(&galaxy, |m| m.travel_stel = 2000),
        [Refusal::GoesHere],
        "the holder wins over _FindSystemFromStellar's 131"
    );
    assert_eq!(
        plainly_on(&galaxy, |m| m.return_stel = 2000),
        [Refusal::GoesHere]
    );
}

#[test]
fn flags_0x0008_needs_a_jumps_fuel() {
    let fuelled = |fuel| {
        offered(
            |visit| visit.player.fuel = fuel,
            |m| m.flags = NEEDS_FUEL | 0x0001,
        )
    };
    assert_eq!(fuelled(100.0), []);
    assert_eq!(fuelled(99.9), [Refusal::Fuel]);
    assert_eq!(offered(|visit| visit.player.fuel = 0.0, |_| {}), []);
}

/// The refusals of a mission of `avail_stel` docked at 128 (of `govt`),
/// in 130.
fn avail_stel_at(govt: Option<GovtId>, avail_stel: i16) -> Vec<Refusal> {
    offered(
        |visit| visit.docked.as_mut().expect("docked").govt = govt,
        |m| m.avail_stel = avail_stel,
    )
}

#[test]
fn avail_stel_minus_1_and_the_docked_stellar_pass() {
    assert_eq!(avail_stel_at(Some(FED), -1), []);
    assert_eq!(avail_stel_at(Some(FED), 128), []);
    assert_eq!(avail_stel_at(Some(FED), 129), [Refusal::AvailStel]);
    for code in [-2, 0, 127, 32000] {
        assert_eq!(
            avail_stel_at(Some(FED), code),
            [Refusal::AvailStel],
            "{code}"
        );
    }
}

#[test]
fn avail_stel_4999_is_a_stellar_and_5000_up_a_system_the_players_lists() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    visit.docked.as_mut().expect("docked").id = StellarId(4999);
    assert_eq!(refusals(&visit, &with(|m| m.avail_stel = 4999)), []);
    assert_eq!(avail_stel_at(None, 5003), [], "130 lists 131");
    assert_eq!(avail_stel_at(None, 5007), [], "and 135");
    assert_eq!(avail_stel_at(None, 5004), [Refusal::AvailStel], "not 132");
    assert_eq!(
        avail_stel_at(None, 5002),
        [Refusal::AvailStel],
        "nor itself"
    );
    assert_eq!(avail_stel_at(None, 9998), [Refusal::AvailStel]);
}

#[test]
fn avail_stel_from_9999_reads_the_docked_stellars_government() {
    assert_eq!(avail_stel_at(Some(FED), 10000), []);
    assert_eq!(avail_stel_at(Some(FED), 10001), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(None, 9999), [], "independent");
    assert_eq!(avail_stel_at(Some(FED), 9999), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(None, 10000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(CIVVIES), 14999), [Refusal::AvailStel]);
}

#[test]
fn avail_stel_15000_up_takes_an_ally_of_the_government_not_an_independent() {
    assert_eq!(
        avail_stel_at(Some(CIVVIES), 15000),
        [],
        "allied with the Federation"
    );
    assert_eq!(avail_stel_at(Some(FED), 15000), [], "its own ally");
    assert_eq!(avail_stel_at(Some(PIRATES), 15000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(None, 15000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(FED), 19999), [Refusal::AvailStel]);
}

#[test]
fn avail_stel_20000_up_takes_any_other_government_an_independent_included() {
    assert_eq!(avail_stel_at(Some(CIVVIES), 20000), []);
    assert_eq!(avail_stel_at(None, 20000), []);
    assert_eq!(avail_stel_at(Some(FED), 20000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(FED), 24999), []);
}

#[test]
fn avail_stel_25000_up_takes_an_enemy_not_an_independent() {
    assert_eq!(avail_stel_at(Some(PIRATES), 25000), []);
    assert_eq!(avail_stel_at(Some(CIVVIES), 25000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(None, 25000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(FED), 29999), [Refusal::AvailStel]);
}

#[test]
fn avail_stel_30000_up_reads_a_shared_class_and_31000_up_none_shared() {
    assert_eq!(avail_stel_at(Some(FED), 30000), []);
    assert_eq!(avail_stel_at(Some(CIVVIES), 30000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(None, 30000), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(FED), 30999), [Refusal::AvailStel]);
    assert_eq!(avail_stel_at(Some(CIVVIES), 31000), []);
    assert_eq!(avail_stel_at(Some(FED), 31000), [Refusal::AvailStel]);
    assert_eq!(
        avail_stel_at(None, 31000),
        [Refusal::AvailStel],
        "not independent"
    );
    assert_eq!(avail_stel_at(Some(FED), 31999), []);
    assert_eq!(avail_stel_at(Some(FED), 32000), [Refusal::AvailStel]);
}

/// Control bits that hold every test but record each one asked.
#[derive(Debug, Default)]
struct Recording {
    asked: RefCell<Vec<TestExpr>>,
    holds: bool,
}

impl ControlBits for Recording {
    fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
        self.asked.borrow_mut().push(test.clone());
        self.holds
    }
}

#[test]
fn avail_bits_goes_through_the_control_bits_and_a_broken_one_never_holds() {
    let galaxy = Galaxy::new();
    for holds in [true, false] {
        let bits = Recording {
            holds,
            ..Recording::default()
        };
        let mut visit = galaxy.visit();
        visit.gate = Gate {
            control_bits: &bits,
            pilot: &FreshPilot,
        };
        let refused = refusals(&visit, &with(|m| m.avail_bits = Test::parse("b7 & !b8")));
        let expected: &[Refusal] = if holds { &[] } else { &[Refusal::AvailBits] };
        assert_eq!(refused, expected, "{holds}");
        assert_eq!(
            *bits.asked.borrow(),
            [Test::parse("b7 & !b8").tree().expect("parses").clone()]
        );
    }
    let bits = Recording {
        holds: true,
        ..Recording::default()
    };
    let mut visit = galaxy.visit();
    visit.gate = Gate {
        control_bits: &bits,
        pilot: &FreshPilot,
    };
    assert_eq!(
        refusals(&visit, &with(|m| m.avail_bits = Test::parse("(b7"))),
        [Refusal::AvailBits]
    );
    assert!(bits.asked.borrow().is_empty(), "not asked");
}

#[test]
fn require_needs_every_bit_contributed() {
    let contributing = |contributed| {
        offered(
            |visit| visit.player.contributed = contributed,
            |m| m.require = 0x8000_0000_0000_0001,
        )
    };
    assert_eq!(contributing(0x8000_0000_0000_0001), []);
    assert_eq!(contributing(u64::MAX), []);
    assert_eq!(contributing(0x0000_0000_0000_0001), [Refusal::Require]);
    assert_eq!(contributing(0x8000_0000_0000_0000), [Refusal::Require]);
}

/// The refusals of a mission of `avail_record` to a pilot of `record`.
fn record_of(record: i16, avail_record: i16) -> Vec<Refusal> {
    offered(
        |visit| visit.player.record = record,
        |m| m.avail_record = avail_record,
    )
}

#[test]
fn avail_record_0_passes_above_0_needs_that_high_and_below_that_low() {
    assert_eq!(record_of(-500, 0), []);
    assert_eq!(record_of(5, 0), []);
    assert_eq!(record_of(10, 10), []);
    assert_eq!(record_of(9, 10), [Refusal::AvailRecord]);
    assert_eq!(record_of(-10, -10), []);
    assert_eq!(record_of(-9, -10), [Refusal::AvailRecord]);
    assert_eq!(record_of(-31999, -31999), []);
}

/// Why mission 130 with `AvailRecord` `avail_record` is not offered on
/// [`Galaxy::visit`], once `stellars` has changed the galaxy's stellars
/// and `adjust` the visit.
fn dominion(
    stellars: impl FnOnce(&mut Vec<Stellar>),
    adjust: impl FnOnce(&mut Visit),
    avail_record: i16,
) -> Vec<Refusal> {
    let mut galaxy = Galaxy::new();
    stellars(&mut galaxy.stellars);
    let mut visit = galaxy.visit();
    adjust(&mut visit);
    refusals(&visit, &with(|m| m.avail_record = avail_record))
}

/// Gives stellar `id` of the galaxy the always-dominated bit.
fn dominate(stellars: &mut [Stellar], id: i16) {
    for stellar in stellars.iter_mut().filter(|s| s.id == StellarId(id)) {
        stellar.flags2 |= DOMINATED;
    }
}

/// Gives the docked stellar's site the always-dominated bit.
fn dominate_docked(visit: &mut Visit) {
    if let Some(site) = visit.docked.as_mut() {
        site.flags2 |= DOMINATED;
    }
}

#[test]
fn avail_record_minus_32000_needs_the_docked_stellar_dominated() {
    for record in [i16::MAX, 0, i16::MIN] {
        assert_eq!(
            dominion(
                |_| {},
                |visit| {
                    visit.player.record = record;
                    dominate_docked(visit);
                },
                -32000
            ),
            [],
            "dominated, record {record}"
        );
        assert_eq!(
            dominion(|_| {}, |visit| visit.player.record = record, -32000),
            [Refusal::AvailRecord],
            "not dominated, record {record}"
        );
    }
    assert_eq!(
        dominion(
            |stellars| {
                dominate(stellars, 128);
                dominate(stellars, 129);
            },
            |visit| visit.player.record = i16::MIN,
            -32000
        ),
        [Refusal::AvailRecord],
        "the docked site's bit, not the world list's"
    );
}

#[test]
fn avail_record_minus_32001_needs_any_stellar_of_a_visible_system_dominated() {
    for record in [i16::MAX, 0, i16::MIN] {
        assert_eq!(
            dominion(
                |stellars| dominate(stellars, 150),
                |visit| visit.player.record = record,
                -32001
            ),
            [],
            "150 dominated, record {record}"
        );
        assert_eq!(
            dominion(|_| {}, |visit| visit.player.record = record, -32001),
            [Refusal::AvailRecord],
            "none dominated, record {record}"
        );
    }
    assert_eq!(
        dominion(
            |stellars| {
                dominate(stellars, 160);
                for stellar in stellars.iter_mut() {
                    stellar.destroyed = stellar.id == StellarId(160);
                }
            },
            |_| {},
            -32001
        ),
        [],
        "destroying a stellar leaves its +0x44 set"
    );
    assert_eq!(
        dominion(
            |stellars| stellars.retain(|s| s.id != StellarId(128)),
            dominate_docked,
            -32001
        ),
        [Refusal::AvailRecord],
        "the world list's bits, not the docked site's"
    );
}

#[test]
fn avail_record_below_minus_32001_refuses_at_a_stellar() {
    for avail_record in [-32002, i16::MIN] {
        assert_eq!(
            dominion(
                |stellars| {
                    for stellar in stellars.iter_mut() {
                        stellar.flags2 |= DOMINATED;
                    }
                },
                |visit| {
                    visit.player.record = i16::MIN;
                    dominate_docked(visit);
                },
                avail_record
            ),
            [Refusal::AvailRecord],
            "{avail_record}"
        );
    }
}

#[test]
fn avail_rating_0_or_below_passes_else_needs_the_rating() {
    let rated = |rating, avail_rating| {
        offered(
            |visit| visit.player.rating = rating,
            |m| m.avail_rating = avail_rating,
        )
    };
    assert_eq!(rated(0, 0), []);
    assert_eq!(rated(0, -1), []);
    assert_eq!(rated(400, 400), []);
    assert_eq!(rated(399, 400), [Refusal::AvailRating]);
    assert_eq!(
        rated(i64::from(i16::MAX) + 1, i16::MAX),
        [],
        "wider than 16 bits"
    );
}

#[test]
fn avail_random_reads_the_visits_roll_for_the_mission() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let rolls = RollTable::default().with(MissionId(130), 40);
    let random =
        |avail_random| refusals_with(&visit, &with(|m| m.avail_random = avail_random), &rolls);
    assert_eq!(random(40), [], "the roll no higher");
    assert_eq!(random(39), [Refusal::AvailRandom]);
    assert_eq!(random(0), [Refusal::AvailRandom, Refusal::AvailRandom]);
    assert_eq!(random(-5), [Refusal::AvailRandom, Refusal::AvailRandom]);
    let unrolled = RollTable::default().with(MissionId(130), 101);
    let above =
        |avail_random| refusals_with(&visit, &with(|m| m.avail_random = avail_random), &unrolled);
    assert_eq!(above(100), [], "above 99 reads no roll");
    assert_eq!(above(99), [Refusal::AvailRandom]);
}

#[test]
fn a_mission_outside_the_roll_table_passes_avail_random_only_above_99() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    for id in [127, 1128] {
        let random = |avail_random| {
            let mut record = with(|m| m.avail_random = avail_random);
            record.id = MissionId(id);
            refusals(&visit, &record)
        };
        assert_eq!(random(100), [], "{id}");
        assert_eq!(random(99), [Refusal::AvailRandom], "{id}");
    }
}

#[test]
fn flags2_0x0001_needs_room_for_the_cargo() {
    let roomy = |free, qty| {
        offered(
            |visit| visit.player.free_cargo = free,
            |m| {
                m.flags2 = NEEDS_ROOM;
                m.cargo_qty = qty;
            },
        )
    };
    assert_eq!(roomy(20, 20), []);
    assert_eq!(roomy(19, 20), [Refusal::CargoSpace]);
    assert_eq!(roomy(0, -10), [], "a range passes");
    assert_eq!(roomy(-5, -10), [], "even over the hold, compared raw");
    assert_eq!(roomy(-11, -10), [Refusal::CargoSpace]);
    assert_eq!(
        offered(|visit| visit.player.free_cargo = 0, |m| m.cargo_qty = 20),
        [],
        "without the flag"
    );
}

/// The refusals of a mission of `avail_ship_type` to a pilot flying
/// `ship`, its class of `inherent` attributes and `combat` government.
fn flying(
    ship: i16,
    inherent: Option<GovtId>,
    combat: Option<GovtId>,
    avail_ship_type: i16,
) -> Vec<Refusal> {
    offered(
        |visit| {
            visit.player.ship = ShipId(ship);
            visit.player.inherent_govt = inherent;
            visit.player.combat_govt = combat;
        },
        |m| m.avail_ship_type = avail_ship_type,
    )
}

#[test]
fn avail_ship_type_128_to_896_takes_that_ship_and_1128_to_1896_any_other() {
    assert_eq!(flying(140, None, None, 140), []);
    assert_eq!(flying(141, None, None, 140), [Refusal::AvailShipType]);
    assert_eq!(flying(896, None, None, 896), []);
    assert_eq!(flying(141, None, None, 1140), []);
    assert_eq!(flying(140, None, None, 1140), [Refusal::AvailShipType]);
    assert_eq!(flying(896, None, None, 1896), [Refusal::AvailShipType]);
    assert_eq!(flying(128, None, None, 128), []);
    assert_eq!(flying(129, None, None, 1128), []);
}

#[test]
fn avail_ship_type_2128_up_reads_either_inherent_government_and_3128_up_neither() {
    assert_eq!(flying(140, Some(FED), None, 2128), [], "attributes");
    assert_eq!(flying(140, None, Some(FED), 2128), [], "combat");
    assert_eq!(
        flying(140, Some(CIVVIES), Some(CIVVIES), 2128),
        [Refusal::AvailShipType]
    );
    assert_eq!(flying(140, Some(GovtId(384)), None, 2384), []);
    assert_eq!(flying(140, None, None, 3128), []);
    assert_eq!(flying(140, Some(FED), None, 3128), [Refusal::AvailShipType]);
    assert_eq!(flying(140, None, Some(FED), 3128), [Refusal::AvailShipType]);
    assert_eq!(
        flying(140, None, Some(GovtId(384)), 3384),
        [Refusal::AvailShipType]
    );
}

#[test]
fn avail_ship_type_outside_its_ranges_takes_any_ship() {
    for code in [-1, 0, 127, 897, 1127, 1897, 2127, 2385, 3127, 3385] {
        assert_eq!(flying(140, Some(FED), Some(FED), code), [], "{code}");
    }
}

#[test]
fn flags_0x2000_needs_a_warship_and_0x4000_a_trader() {
    let ai = |inherent_ai, flags| {
        offered(
            |visit| visit.player.inherent_ai = inherent_ai,
            |m| m.flags = flags,
        )
    };
    assert_eq!(ai(3, WARSHIPS_ONLY), []);
    assert_eq!(ai(2, WARSHIPS_ONLY), [Refusal::InherentAi]);
    assert_eq!(ai(2, TRADERS_ONLY), []);
    assert_eq!(ai(3, TRADERS_ONLY), [Refusal::InherentAi]);
    assert_eq!(ai(3, WARSHIPS_ONLY | TRADERS_ONLY), [Refusal::InherentAi]);
    assert_eq!(ai(1, 0), []);
}

#[test]
fn a_pay_val_below_minus_50000_needs_the_cash_it_takes() {
    let paying = |cash, pay_val| offered(|visit| visit.player.cash = cash, |m| m.pay_val = pay_val);
    assert_eq!(paying(0, -50000), [], "takes nothing");
    assert_eq!(paying(1, -50001), []);
    assert_eq!(paying(0, -50001), [Refusal::Cash]);
    assert_eq!(paying(999, -51000), [Refusal::Cash]);
    assert_eq!(paying(1000, -51000), []);
    assert_eq!(paying(-5, 25000), [], "a pay");
}

#[test]
fn a_destination_or_return_that_finds_no_stellar_refuses() {
    for code in [1, 127, 2176, 9998, 10002] {
        assert_eq!(
            plainly(|m| m.travel_stel = code),
            [Refusal::TravelStel],
            "{code}"
        );
        assert_eq!(
            plainly(|m| m.return_stel = code),
            [Refusal::ReturnStel],
            "{code}"
        );
    }
    assert_eq!(
        plainly(|m| {
            m.travel_stel = 1;
            m.return_stel = 1;
        }),
        [Refusal::TravelStel],
        "one refusal, the destination's first"
    );
    for code in [-1, -4, -5, 0, 140, 2175, -2, -3, 10000, 10001] {
        assert_eq!(plainly(|m| m.travel_stel = code), [], "{code}");
    }
}

#[test]
fn availability_chain_tests_a_random_destination_by_the_engine_alone() {
    let mut galaxy = Galaxy::new();
    for stellar in &mut galaxy.stellars {
        stellar.listed_at_place = stellar.id != StellarId(152);
    }
    let minus_3 = with(|m| m.travel_stel = -3);
    assert_eq!(
        refusals(&galaxy.visit(), &minus_3),
        [Refusal::TravelStel],
        "152, the only uninhabited stellar, is not listed by every system at its place (@0x9e60-0x9ee4)"
    );
    galaxy.rules =
        Rulebook::default().with_override(RuleKey::MissionStellarDistance, RuleSource::Bible);
    assert_eq!(
        refusals(&galaxy.visit(), &minus_3),
        [],
        "the other reading tests no distance with no origin"
    );
}

#[test]
fn availabilitys_destination_check_has_no_origin_to_keep_away_from() {
    let galaxy = Galaxy {
        stellars: vec![at(130, 128, Some(FED))],
        ..Galaxy::new()
    };
    let visit = galaxy.visit();
    assert_eq!(
        refusals(&visit, &with(|m| m.travel_stel = -2)),
        [],
        "the docked stellar itself is a candidate (@0x9b859-0x9b873)"
    );
    let offer = resolve(
        &with(|m| m.travel_stel = -2),
        Venue::Computer,
        Some(StellarId(128)),
        &visit.world,
        today(),
        &mut Draws::default(),
    );
    let home = Some(Stop {
        stellar: StellarId(128),
        system: Some(SystemId(130)),
    });
    assert_eq!(
        (offer.destination, offer.return_to),
        (home, home),
        "resolved from the docked stellar, it falls back to it"
    );
}

#[test]
fn an_active_mission_is_not_offered() {
    static ACTIVE: [MissionId; 2] = [MissionId(129), MissionId(130)];
    assert_eq!(
        offered(|visit| visit.active = &ACTIVE, |_| {}),
        [Refusal::Active]
    );
    assert_eq!(offered(|visit| visit.active = &ACTIVE[..1], |_| {}), []);
}

#[test]
fn the_roll_table_draws_1000_rolls_of_1_to_100_by_mission_index() {
    let mut chance = Scripted::rolling(&[99, 0, 41]);
    let mut table = RollTable::default();
    table.draw(&mut chance);
    assert_eq!(chance.sides_asked, vec![100; ROLLED]);
    assert_eq!(table.roll(MissionId(128)), Some(100));
    assert_eq!(table.roll(MissionId(129)), Some(1));
    assert_eq!(table.roll(MissionId(130)), Some(42));
    assert_eq!(table.roll(MissionId(1127)), Some(1), "then 0s, each + 1");
    assert_eq!(table.roll(MissionId(127)), None);
    assert_eq!(table.roll(MissionId(1128)), None);
    assert_eq!(table.roll(MissionId(i16::MIN)), None);
    assert_eq!(
        RollTable::default().roll(MissionId(500)),
        Some(0),
        "undrawn"
    );
}

/// A mission going to a random inhabited stellar and back, with a random
/// cargo of 5 to 14 tons, paying 25000 within 3 days.
fn courier() -> MissionRecord {
    with(|m| {
        m.travel_stel = -2;
        m.return_stel = -1;
        m.cargo_type = 1000;
        m.cargo_qty = -10;
        m.pay_val = 25000;
        m.time_limit = 3;
    })
}

#[test]
fn an_offer_resolves_its_values_in_the_originals_draw_order() {
    // Phase 1's draw: only distant candidates.
    let galaxy = Galaxy::by_bible();
    let visit = galaxy.visit();
    let mut chance = Draws::of(&[1, 4, 7]);
    let offer = resolve(
        &courier(),
        Venue::Computer,
        Some(StellarId(128)),
        &visit.world,
        today(),
        &mut chance,
    );
    let to = Some(Stop {
        stellar: StellarId(151),
        system: Some(SystemId(132)),
    });
    assert_eq!(
        offer,
        Offer {
            mission: MissionId(130),
            venue: Venue::Computer,
            destination: to,
            return_to: to,
            cargo_type: Some(4),
            cargo_tons: 7 + 5,
            pay: 25000,
            deadline: GameDate::new(1177, 6, 26),
        }
    );
    assert_eq!(
        chance.asked,
        [3, 6, 10],
        "150, 151 and 160 distant and inhabited; then the cargo; then the tons"
    );
}

#[test]
fn a_return_draws_never_the_destination_and_minus_4_returns_to_the_origin() {
    // Phase 1's draw: only distant candidates.
    let galaxy = Galaxy::by_bible();
    let visit = galaxy.visit();
    let mut chance = Draws::of(&[0, 0]);
    let there_and_back = with(|m| {
        m.travel_stel = -2;
        m.return_stel = -2;
    });
    let offer = resolve(
        &there_and_back,
        Venue::Computer,
        Some(StellarId(128)),
        &visit.world,
        today(),
        &mut chance,
    );
    assert_eq!(
        offer.destination.map(|stop| stop.stellar),
        Some(StellarId(150))
    );
    assert_eq!(
        offer.return_to.map(|stop| stop.stellar),
        Some(StellarId(151))
    );
    assert_eq!(chance.asked, [3, 2]);
    let home = resolve(
        &with(|m| m.return_stel = -4),
        Venue::Computer,
        Some(StellarId(128)),
        &visit.world,
        today(),
        &mut chance,
    );
    assert_eq!(offer.venue, Venue::Computer);
    assert_eq!(home.destination, None, "TravelStel -1");
    assert_eq!(
        home.return_to,
        Some(Stop {
            stellar: StellarId(128),
            system: Some(SystemId(130)),
        })
    );
}

#[test]
fn a_fixed_stellar_in_no_visible_system_has_no_system() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let offer = resolve(
        &with(|m| m.travel_stel = 2000),
        Venue::Outside,
        Some(StellarId(128)),
        &visit.world,
        today(),
        &mut Draws::default(),
    );
    assert_eq!(
        offer.destination,
        Some(Stop {
            stellar: StellarId(2000),
            system: None,
        })
    );
    assert_eq!(offer.venue, Venue::Outside);
}

/// [`Galaxy::new`] with stellar `id` placed `holder` and `found` alone.
fn placing(id: i16, holder: Option<i16>, found: Option<i16>) -> Galaxy {
    let galaxy = Galaxy::new();
    galaxy
        .placed
        .set(vec![Placed {
            id: StellarId(id),
            at: Whereabouts {
                holder: holder.map(SystemId),
                found: found.map(SystemId),
            },
        }])
        .expect("not placed yet");
    galaxy
}

/// The stops `galaxy`'s visit resolves for mission 130 with `TravelStel`
/// `travel` and `ReturnStel` `back`.
fn stops(galaxy: &Galaxy, travel: i16, back: i16) -> (Option<Stop>, Option<Stop>) {
    let offer = resolve(
        &with(|m| {
            m.travel_stel = travel;
            m.return_stel = back;
        }),
        Venue::Computer,
        Some(StellarId(128)),
        &galaxy.visit().world,
        today(),
        &mut Draws::default(),
    );
    (offer.destination, offer.return_to)
}

/// Stellar `id` in `system`.
fn stop(id: i16, system: Option<i16>) -> Stop {
    Stop {
        stellar: StellarId(id),
        system: system.map(SystemId),
    }
}

#[test]
fn a_destination_is_in_its_holders_stand_in_and_a_return_in_its_holder_as_it_is() {
    // 160 is rescued into 134, which 130 shadows at its place.
    let galaxy = placing(160, Some(134), Some(134));
    assert_eq!(
        stops(&galaxy, 160, 160),
        (Some(stop(160, Some(130))), Some(stop(160, Some(134)))),
        "travel: _FindActiveCoLocatedSystem (@0xa15a8-0xa15b5); return: the owner as it is (@0xa1653-0xa1658)"
    );
}

#[test]
fn a_stop_no_system_holds_is_in_the_system_find_system_from_stellar_gives() {
    let galaxy = placing(2000, None, Some(131));
    assert_eq!(
        stops(&galaxy, 2000, 2000),
        (Some(stop(2000, Some(131))), Some(stop(2000, Some(131)))),
        "@0xa15be-0xa15ca, @0xa1661-0xa1669"
    );
    let galaxy = placing(2000, Some(134), Some(131));
    assert_eq!(
        stops(&galaxy, 2000, 2000),
        (Some(stop(2000, Some(130))), Some(stop(2000, Some(134)))),
        "a holder wins over it"
    );
}

#[test]
fn with_return_stel_minus_1_the_return_system_is_resolved_afresh() {
    let galaxy = placing(160, Some(134), Some(134));
    assert_eq!(
        stops(&galaxy, 160, -1),
        (Some(stop(160, Some(130))), Some(stop(160, Some(134)))),
        "the destination's stellar (@0xa15f0-0xa15ff), its system worked out again (@0xa1632-0xa1669)"
    );
}

#[test]
fn the_cargo_type_is_kept_from_0_to_999_drawn_at_1000_and_else_none() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let cargo = |cargo_type| {
        let mut chance = Draws::of(&[5]);
        let offer = resolve(
            &with(|m| m.cargo_type = cargo_type),
            Venue::Computer,
            Some(StellarId(128)),
            &visit.world,
            today(),
            &mut chance,
        );
        (offer.cargo_type, chance.asked)
    };
    assert_eq!(cargo(0), (Some(0), vec![]));
    assert_eq!(cargo(999), (Some(999), vec![]));
    assert_eq!(cargo(1000), (Some(5), vec![6]));
    assert_eq!(cargo(-1), (None, vec![]));
    assert_eq!(cargo(1001), (None, vec![]));
}

#[test]
fn the_tons_are_kept_from_0_none_at_minus_1_and_drawn_below_even_with_no_cargo() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let tons = |cargo_qty| {
        let mut chance = Draws::of(&[0]);
        let offer = resolve(
            &with(|m| m.cargo_qty = cargo_qty),
            Venue::Computer,
            Some(StellarId(128)),
            &visit.world,
            today(),
            &mut chance,
        );
        (offer.cargo_tons, chance.asked)
    };
    assert_eq!(tons(0), (0, vec![]));
    assert_eq!(tons(25), (25, vec![]));
    assert_eq!(tons(-1), (0, vec![]));
    assert_eq!(tons(-2), (1, vec![2]), "Rand(2) + 1");
    assert_eq!(tons(-7), (3, vec![7]), "Rand(7) + 3");
    assert_eq!(tons(i16::MIN), (16384, vec![32768]));
}

#[test]
fn with_no_origin_a_random_code_draws_among_every_candidate_and_one_naming_it_finds_none() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut chance = Draws::of(&[0]);
    let offer = resolve(
        &with(|m| {
            m.travel_stel = -2;
            m.return_stel = -4;
        }),
        Venue::Ship,
        None,
        &visit.world,
        today(),
        &mut chance,
    );
    assert_eq!(
        offer.destination,
        Some(Stop {
            stellar: StellarId(128),
            system: Some(SystemId(130)),
        }),
        "no distance kept: the lowest of all"
    );
    assert_eq!(chance.asked, [6], "128, 129, 140, 150, 151 and 160");
    assert_eq!(offer.return_to, None, "-4 names the origin");
    assert_eq!(offer.venue, Venue::Ship);
    let nowhere = resolve(
        &mission(130),
        Venue::Ship,
        None,
        &visit.world,
        today(),
        &mut Draws::default(),
    );
    assert_eq!((nowhere.destination, nowhere.return_to), (None, None));
}

#[test]
fn with_no_origin_a_drawn_stellar_is_chain_tested_by_the_engine_alone() {
    // A person's offer with no navigation target passes the origin -1 on
    // to `_StellarsAreDistant` (`_RandomizeOneMission` @0xa151f-0xa157e,
    // @0x9a2b6-0x9a310), which then takes the chain walk (@0x9cf4-0x9cf9).
    let federation = with(|m| m.travel_stel = 10000);
    let drawn = |galaxy: &Galaxy, last: u32| {
        let mut chance = Draws::of(&[last]);
        let offer = resolve(
            &federation,
            Venue::Ship,
            None,
            &galaxy.visit().world,
            today(),
            &mut chance,
        );
        (offer.destination.map(|stop| stop.stellar.0), chance.asked)
    };
    let mut galaxy = Galaxy::new();
    for stellar in &mut galaxy.stellars {
        stellar.listed_at_place = stellar.id != StellarId(150);
    }
    assert_eq!(
        drawn(&galaxy, 1),
        (Some(129), vec![2]),
        "128 and 129: 150 is not listed by every system at its place (@0x9e60-0x9ee4)"
    );
    galaxy.rules =
        Rulebook::default().with_override(RuleKey::MissionStellarDistance, RuleSource::Bible);
    assert_eq!(
        drawn(&galaxy, 2),
        (Some(150), vec![3]),
        "the other reading tests no distance with no origin"
    );
}

#[test]
fn the_pay_is_raw_and_the_deadline_today_plus_its_days() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let offer = |pay_val, time_limit| {
        resolve(
            &with(|m| {
                m.pay_val = pay_val;
                m.time_limit = time_limit;
            }),
            Venue::Computer,
            Some(StellarId(128)),
            &visit.world,
            today(),
            &mut Draws::default(),
        )
    };
    assert_eq!(offer(-50001, 0).pay, -50001);
    assert_eq!(offer(0, 0).deadline, None);
    assert_eq!(offer(0, -1).deadline, None);
    assert_eq!(offer(0, 1).deadline, GameDate::new(1177, 6, 24));
    assert_eq!(offer(0, 8).deadline, GameDate::new(1177, 7, 1));
}

/// Missions 130 (the computer's, random-destination), 131 (outside, the
/// bar's), 132 (the computer's, `AvailRandom` 10) and 133 (outside,
/// going somewhere with no candidate).
fn catalog() -> Vec<MissionRecord> {
    let mut bar = mission(131);
    bar.avail_loc = 1;
    bar.cargo_qty = -4;
    let mut rare = mission(132);
    rare.avail_random = 10;
    let mut nowhere = mission(133);
    nowhere.avail_loc = 3;
    nowhere.travel_stel = 10002;
    vec![courier(), bar, rare, nowhere]
}

fn ids(offers: &[Offer]) -> Vec<i16> {
    offers.iter().map(|offer| offer.mission.0).collect()
}

#[test]
fn a_roll_draws_the_table_then_resolves_the_computers_offers_then_the_others() {
    // Phase 1's draw: only distant candidates.
    let galaxy = Galaxy::by_bible();
    let visit = galaxy.visit();
    let mut chance = Draws::of(&[]);
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut chance);
    let mut expected = vec![100; ROLLED];
    expected.extend([3, 6, 10, 4]);
    assert_eq!(
        chance.asked, expected,
        "the table; 130's stellar, cargo and tons; 131's tons"
    );
    assert_eq!(
        ids(offers.at(Venue::Computer)),
        [130],
        "132's roll of 100 is above 10"
    );
    assert_eq!(ids(offers.at(Venue::Outside)), [131]);
    assert_eq!(offers.rolled_at(), Some(StellarId(128)));
    assert_eq!(offers.table().roll(MissionId(130)), Some(100));
}

#[test]
fn a_roll_is_due_at_any_stellar_but_the_one_last_rolled_at() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    assert!(offers.due_at(StellarId(128)), "never rolled");
    offers.roll(&catalog(), &visit, &mut NeverFires);
    assert!(!offers.due_at(StellarId(128)), "rolled there last");
    assert!(offers.due_at(StellarId(129)));
    offers.forget();
    assert!(offers.due_at(StellarId(128)), "forgotten");
}

#[test]
fn the_same_stellar_again_draws_nothing_and_keeps_the_offers() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut Draws::of(&[]));
    let kept = offers.clone();
    let mut again = Draws::of(&[]);
    offers.roll(&catalog(), &visit, &mut again);
    assert!(again.asked.is_empty(), "{:?}", again.asked);
    assert_eq!(offers, kept);
}

#[test]
fn with_no_stellar_docked_a_roll_draws_nothing_and_keeps_the_offers() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    let kept = offers.clone();
    visit.docked = None;
    let mut chance = Draws::of(&[]);
    offers.roll(&catalog(), &visit, &mut chance);
    assert!(chance.asked.is_empty(), "{:?}", chance.asked);
    assert_eq!(offers, kept);
    let mut fresh = Offers::default();
    fresh.roll(&catalog(), &visit, &mut chance);
    assert!(chance.asked.is_empty(), "never rolled: {:?}", chance.asked);
    assert_eq!(fresh, Offers::default());
}

#[test]
fn another_stellar_or_one_forgotten_rolls_afresh() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    visit.docked = Some(docked_at(129));
    let mut elsewhere = Draws::of(&[]);
    offers.roll(&catalog(), &visit, &mut elsewhere);
    assert_eq!(
        elsewhere.asked.len(),
        ROLLED + 4,
        "{:?}",
        &elsewhere.asked[ROLLED..]
    );
    assert_eq!(offers.rolled_at(), Some(StellarId(129)));
    offers.forget();
    assert_eq!(offers.rolled_at(), None);
    assert_eq!(ids(offers.at(Venue::Computer)), [130], "kept until then");
    let mut forgotten = Draws::of(&[]);
    offers.roll(&catalog(), &visit, &mut forgotten);
    assert_eq!(forgotten.asked.len(), ROLLED + 4);
}

#[test]
fn a_redraw_draws_the_table_afresh_and_forgets_the_stellar_keeping_the_offers() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    let kept = offers.at(Venue::Computer).to_vec();
    let mut chance = Scripted::rolling(&[41]);
    offers.redraw(&mut chance);
    assert_eq!(chance.sides_asked, vec![100; ROLLED], "the table alone");
    assert_eq!(offers.table().roll(MissionId(128)), Some(42));
    assert_eq!(offers.table().roll(MissionId(129)), Some(1));
    assert_eq!(offers.rolled_at(), None);
    assert_eq!(offers.at(Venue::Computer), kept, "kept until the landing");
}

#[test]
fn the_rare_mission_is_offered_on_a_low_roll() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    assert_eq!(ids(offers.at(Venue::Computer)), [130, 132], "every roll 1");
}

#[test]
fn an_active_mission_is_skipped_by_a_roll() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    let active = [MissionId(130)];
    visit.active = &active;
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    assert_eq!(ids(offers.at(Venue::Computer)), [132]);
}

#[test]
fn a_reoffer_lists_and_resolves_again_keeping_the_table() {
    // Phase 1's draw: only distant candidates.
    let galaxy = Galaxy::by_bible();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    let table = offers.table().clone();
    let mut chance = Draws::of(&[2]);
    let mut fewer = catalog();
    fewer.remove(1);
    offers.reoffer(&fewer, &visit, &mut chance);
    assert_eq!(chance.asked, [3, 6, 10], "130's draws alone");
    assert_eq!(offers.table(), &table);
    assert_eq!(ids(offers.at(Venue::Computer)), [130, 132]);
    assert!(offers.at(Venue::Outside).is_empty(), "131 gone");
    assert_eq!(
        offers.at(Venue::Computer)[0]
            .destination
            .map(|stop| stop.stellar),
        Some(StellarId(160))
    );
}

#[test]
fn emptying_clears_both_venues_and_keeps_the_stellar() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    offers.empty();
    assert!(offers.at(Venue::Computer).is_empty());
    assert!(offers.at(Venue::Outside).is_empty());
    assert_eq!(offers.rolled_at(), Some(StellarId(128)));
}

#[test]
fn a_mission_refused_by_avail_random_stays_refused_and_rechecking_draws_nothing() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut Draws::of(&[]));
    let rare = &catalog()[2];
    let facts = AvailFacts {
        visit: &visit,
        mission: rare,
        venue: Venue::Computer,
        rolls: offers.table(),
        cached_avail_bits: None,
    };
    let mut chance = Draws::of(&[]);
    for _ in 0..3 {
        assert_eq!(
            availability().first_refusal(&facts, &mut chance),
            Err(Refusal::AvailRandom)
        );
        let all: Vec<_> = availability()
            .all_refusals(&facts)
            .into_iter()
            .map(|(descriptor, verdict)| (descriptor.name(), verdict))
            .collect();
        assert_eq!(
            all,
            [("AvailRandom", Verdict::Refuses(Refusal::AvailRandom))]
        );
    }
    assert!(chance.asked.is_empty(), "{:?}", chance.asked);
}

#[test]
fn an_offer_dropped_leaves_the_others_and_the_stellar() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    assert!(offers.drop_offer(MissionId(130)));
    assert!(!offers.drop_offer(MissionId(130)), "dropped already");
    assert!(offers.drop_offer(MissionId(131)), "outside");
    assert_eq!(ids(offers.at(Venue::Computer)), [132]);
    assert!(offers.at(Venue::Outside).is_empty());
    assert_eq!(offers.rolled_at(), Some(StellarId(128)));
}

/// The courier at the mission computer, and 131, 134 and 135 at the bar.
fn three_at_the_bar() -> Vec<MissionRecord> {
    let bar = |id| MissionRecord {
        avail_loc: 1,
        ..mission(id)
    };
    vec![courier(), bar(131), bar(134), bar(135)]
}

#[test]
fn drop_at_removes_the_entry_at_the_index_and_nothing_past_the_end() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&three_at_the_bar(), &visit, &mut NeverFires);
    assert_eq!(ids(offers.at(Venue::Outside)), [131, 134, 135]);
    assert_eq!(offers.drop_at(Venue::Outside, 1), Some(MissionId(134)));
    assert_eq!(ids(offers.at(Venue::Outside)), [131, 135]);
    assert_eq!(offers.drop_at(Venue::Outside, 2), None, "past the end");
    assert_eq!(ids(offers.at(Venue::Outside)), [131, 135]);
    assert_eq!(offers.drop_at(Venue::Ship, 0), None, "never listed");
    assert_eq!(offers.drop_at(Venue::Computer, 0), Some(MissionId(130)));
    assert!(offers.at(Venue::Computer).is_empty());
    assert_eq!(ids(offers.at(Venue::Outside)), [131, 135]);
}

#[test]
fn position_finds_an_offer_at_its_venue_only() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&three_at_the_bar(), &visit, &mut NeverFires);
    assert_eq!(offers.position(Venue::Outside, MissionId(134)), Some(1));
    assert_eq!(offers.position(Venue::Outside, MissionId(131)), Some(0));
    assert_eq!(offers.position(Venue::Computer, MissionId(134)), None);
    assert_eq!(offers.position(Venue::Computer, MissionId(130)), Some(0));
    assert_eq!(offers.position(Venue::Outside, MissionId(130)), None);
    assert_eq!(offers.position(Venue::Ship, MissionId(131)), None);
}

#[test]
fn a_refilter_keeps_only_the_offers_still_available_resolving_nothing() {
    let galaxy = Galaxy::new();
    let mut visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &visit, &mut NeverFires);
    let kept = offers.at(Venue::Computer)[1].clone();
    let active = [MissionId(130), MissionId(131)];
    visit.active = &active;
    offers.refilter(&catalog(), &visit);
    assert_eq!(offers.at(Venue::Computer), [kept], "132 as rolled");
    assert!(offers.at(Venue::Outside).is_empty(), "131 active now");
    let mut fewer = catalog();
    fewer.remove(2);
    offers.refilter(&fewer, &visit);
    assert!(offers.at(Venue::Computer).is_empty(), "132 has no record");
}

#[test]
fn a_roll_set_is_read_back_and_one_outside_the_table_is_ignored() {
    let galaxy = Galaxy::new();
    let mut offers = Offers::default();
    offers.roll(&catalog(), &galaxy.visit(), &mut NeverFires);
    offers.set_roll(MissionId(130), 0);
    assert_eq!(offers.table().roll(MissionId(130)), Some(0));
    assert_eq!(offers.table().roll(MissionId(131)), Some(1));
    offers.set_roll(MissionId(127), 0);
    offers.set_roll(MissionId(1128), 0);
    assert_eq!(offers.table().roll(MissionId(127)), None);
}

/// Computer missions 130-135 weighing 3, -1, 0, -5, -1 and 7, and bar
/// missions 140 and 141 weighing -2 and 4.
fn weighted() -> Vec<MissionRecord> {
    let computer = [3, -1, 0, -5, -1, 7].into_iter().zip(130..);
    let outside = [-2, 4].into_iter().zip(140..);
    computer
        .chain(outside)
        .map(|(weight, id)| {
            let mut record = mission(id);
            record.disp_weight = weight;
            if id >= 140 {
                record.avail_loc = 1;
            }
            record
        })
        .collect()
}

#[test]
fn a_roll_and_a_reoffer_put_each_venues_offers_in_display_order() {
    let galaxy = Galaxy::new();
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&weighted(), &visit, &mut NeverFires);
    assert_eq!(
        ids(offers.at(Venue::Computer)),
        [130, 132, 135, 131, 134, 133]
    );
    assert_eq!(ids(offers.at(Venue::Outside)), [141, 140]);
    offers.reoffer(&weighted(), &visit, &mut NeverFires);
    assert_eq!(
        ids(offers.at(Venue::Computer)),
        [130, 132, 135, 131, 134, 133]
    );
}

#[test]
fn by_the_bibles_order_a_roll_sorts_every_offer_by_descending_weight() {
    let mut galaxy = Galaxy::new();
    galaxy.rules = Rulebook::default().with_override(RuleKey::MissionOrder, RuleSource::Bible);
    let visit = galaxy.visit();
    let mut offers = Offers::default();
    offers.roll(&weighted(), &visit, &mut NeverFires);
    assert_eq!(
        ids(offers.at(Venue::Computer)),
        [135, 130, 132, 131, 134, 133]
    );
    assert_eq!(ids(offers.at(Venue::Outside)), [141, 140]);
}
