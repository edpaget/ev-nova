//! The traffic the fixture's data names, flown by the real session: the
//! Fringe's fleets come into the system as the ships of their factions,
//! with the AI their roles give, and a `take_point` that names its guard
//! count fields that many guards.
//!
//! nova-data pins the mapping (a fleet's `düde`, a system's slots, a
//! blockade's `ShipCount`) and nova-sim-ai pins how a system's slots
//! spawn ships. These tests cover only that the fixture, the mapping and
//! the session meet: every value they expect is read from the opened data.

use std::collections::BTreeSet;

use nova::chance::SplitMix;
use nova_data::new_game::fixture::folder;
use nova_data::records::dude::Dude;
use nova_data::records::mission::Mission;
use nova_data::records::system::System;
use nova_sim::AiType;
use nova_sim::storylines::state::MissionState;

use crate::new_game_play::Play;
use crate::new_game_travel::nova_of;

/// The seeds the Fringe is filled on, at most, before every faction and
/// ship its fleets name has come in.
const SEEDS: u64 = 64;

/// A `düde`'s flown ships, its government and its AI, as an NPC shows
/// them.
struct Flown {
    ships: Vec<i16>,
    govt: Option<i16>,
    ai: AiType,
}

#[test]
fn the_fringe_fills_with_the_ships_of_its_fleets_factions_variants_included() {
    let mut play = Play::new(&folder(), "fringe", |_| {});
    let fringe = nova_of(&play.game.ids.systems, "fringe");
    let system = play
        .data
        .get::<System>(fringe)
        .expect("present")
        .expect("decodes");
    let flown: Vec<Flown> = (system.record.dude_types.iter())
        .filter(|&&dude| dude >= 128)
        .map(|&dude| {
            let dude = play
                .data
                .get::<Dude>(dude)
                .expect("present")
                .expect("decodes");
            Flown {
                ships: dude
                    .record
                    .ship_type
                    .iter()
                    .flatten()
                    .map(|s| s.0)
                    .collect(),
                govt: dude.record.govt.map(|g| g.0),
                ai: AiType::from_raw(dude.record.ai_type),
            }
        })
        .collect();
    // What the Fringe's file names: its fleets' factions and ships.
    let galaxy = &play.game.galaxy;
    let (_, file) = (galaxy.systems.iter())
        .find(|(id, _)| id.as_str() == "fringe")
        .expect("the Fringe's file");
    let traffic = file
        .data
        .traffic
        .as_ref()
        .expect("the Fringe names its traffic");
    let mut factions = BTreeSet::new();
    let mut ships = BTreeSet::new();
    let mut variant = false;
    for fleet in traffic.fleets.keys() {
        let fleet = &galaxy.fleets[fleet].data;
        let faction = fleet.faction.as_ref().expect("a faction's fleet");
        factions.insert(play.game.ids.factions[faction]);
        for ship in fleet.ships.keys() {
            ships.insert(play.game.ids.ships[ship]);
            variant |= galaxy.ships[ship].data.base.is_some();
        }
    }
    assert!(variant, "a fleet flies a variant");
    assert_eq!(factions.len(), 2, "the wardens' and the raiders'");
    assert_eq!(flown.len(), traffic.fleets.len(), "a slot for each fleet");
    let ais: BTreeSet<_> = flown.iter().map(|f| format!("{:?}", f.ai)).collect();
    assert_eq!(ais.len(), 2, "two roles, two AIs: {ais:?}");
    let mut seen_factions = BTreeSet::new();
    let mut seen_ships = BTreeSet::new();
    for seed in 0..SEEDS {
        play.session.populate(&play.data, &mut SplitMix::new(seed));
        for npc in play.session.npcs() {
            let (ship, govt) = (npc.ship.0, npc.govt.map(|g| g.0));
            assert!(
                flown
                    .iter()
                    .any(|f| f.ships.contains(&ship) && f.govt == govt && f.ai == npc.ai_type),
                "seed {seed}: ship {ship} of {govt:?} as {:?} is none of the Fringe's fleets",
                npc.ai_type
            );
            seen_factions.extend(govt);
            seen_ships.insert(ship);
        }
        if seen_factions == factions && seen_ships == ships {
            return;
        }
    }
    panic!(
        "over {SEEDS} seeds the Fringe showed factions {seen_factions:?} of {factions:?} and \
         ships {seen_ships:?} of {ships:?}"
    );
}

/// The NPCs in the system that belong to a mission: its guards.
fn guards(play: &Play) -> usize {
    (play.session.npcs().iter())
        .filter(|npc| npc.mission.is_some())
        .count()
}

/// The `ShipCount` of `mission`'s blockade, read from the opened data.
fn ship_count(play: &Play, mission: &str) -> usize {
    let blockade = nova_of(&play.game.ids.blockades, mission);
    let record = play
        .data
        .get::<Mission>(blockade)
        .expect("present")
        .expect("decodes");
    usize::try_from(record.record.ship_count).expect("a count")
}

#[test]
fn the_fringe_road_is_held_by_the_guards_its_take_point_names() {
    let mut play = Play::new(&folder(), "drift", |pilot| {
        pilot.story.missions.insert(
            "fringe-road".to_owned(),
            MissionState::Active {
                cargo: 0,
                met: false,
            },
        );
    });
    let count = ship_count(&play, "fringe-road");
    assert_ne!(
        count,
        ship_count(&play, "clear-the-road"),
        "it names its own count, not the default"
    );
    play.go_through("drift/fringe-road");
    assert_eq!(guards(&play), count);
}
