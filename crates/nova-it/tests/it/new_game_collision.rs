//! The new game's ships collide by their data radius: over the checked-in
//! sample (`nova_data::new_game::fixture::sample_folder`), under the
//! rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, the player's
//! light freighter carries the sample's collision radius, and a shot
//! passing a light freighter off its sprite but within its data disc
//! strikes it. Under the stock ruleset's readings the same shot passes it
//! by.
//!
//! What a ship is struck by under each reading is nova-sim-combat's rule
//! (`data_collision`), and how the radius is read from the data is
//! nova-data's (`new_game::fleet::collision_radius`), each pinned there;
//! this pins that the ruleset, the data and the fight meet.

use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_sim::fixture::{place, put_player_at};
use nova_sim::{
    Behaviour, Chance, CombatCatalog, Controls, Goal, Npc, NpcId, OutfitId, Pilot, Rules, Session,
    ShipId, Surroundings, TargetPick, Trigger, Vec2,
};

/// The checked-in sample, mapped.
fn sample() -> NewGame {
    new_game::open(&new_game::fixture::sample_folder()).expect("the sample lists")
}

/// The Nova ID `table` gives the bible id `name`.
fn nova_id<K: std::fmt::Display>(table: &std::collections::BTreeMap<K, i16>, name: &str) -> i16 {
    table
        .iter()
        .find(|(id, _)| id.to_string() == name)
        .map(|(_, &nova)| nova)
        .expect("in the sample")
}

/// The id of the sample's ship `name`.
fn ship(game: &NewGame, name: &str) -> ShipId {
    ShipId(nova_id(&game.ids.ships, name))
}

/// The light freighter's collision radius, in pixels, as the data gives
/// it.
fn freighter_radius(game: &NewGame) -> f32 {
    let radius = game
        .data
        .collision_radius(ship(game, "light-freighter"))
        .expect("the freighter's data radius");
    f32::from(radius)
}

/// The rules `ruleset` pins.
fn rules_of(ruleset: Ruleset) -> nova_sim::Rulebook {
    ruleset_rulebook(None, ruleset.profile().readings).0
}

/// A new pilot of the sample, its light freighter carrying a mass driver,
/// flying under `ruleset`'s readings.
fn armed_session(game: &NewGame, ruleset: Ruleset) -> Session {
    let mut pilot = Pilot::new(&game.data, "Tester").expect("the sample starts");
    assert_eq!(pilot.ship, ship(game, "light-freighter"));
    let gun = OutfitId(nova_id(&game.ids.outfits, "mass-driver"));
    pilot.outfits.insert(gun, 1);
    Session::load(&game.data, pilot, &rules_of(ruleset)).expect("flies")
}

/// Every NPC idles.
#[derive(Debug)]
struct Still;

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

/// `session`'s system filled with two of its traffic at least, on the
/// first seed that places them.
fn two_ships(session: &mut Session, game: &NewGame) -> (NpcId, NpcId) {
    for seed in 0..200 {
        session.populate(&game.data, &mut SplitMix::new(seed));
        if let [first, second, ..] = session.npcs() {
            return (first.id, second.id);
        }
    }
    panic!("no seed placed two ships");
}

/// Whether the player's mass driver, fired at a light freighter far up its
/// line of fire, strikes another light freighter standing `off` to the
/// side of that line, and whether it strikes the target, under
/// `ruleset`'s readings.
fn passing_strikes(ruleset: Ruleset, off: f32) -> (bool, bool) {
    let game = sample();
    let mut session = armed_session(&game, ruleset);
    let (target, passed) = two_ships(&mut session, &game);
    for npc in session.npcs() {
        assert_eq!(npc.ship, ship(&game, "light-freighter"), "the traffic");
    }
    put_player_at(&mut session, Vec2::ZERO);
    // The target nearest, to be picked, then placed far up the line.
    place(&mut session, target, Vec2::new(0.0, -40.0));
    place(&mut session, passed, Vec2::new(500.0, 500.0));
    assert_eq!(session.select_target(TargetPick::Nearest), Some(target));
    let stand = |session: &mut Session| {
        place(session, target, Vec2::new(0.0, -300.0));
        place(session, passed, Vec2::new(off, -100.0));
        put_player_at(session, Vec2::ZERO);
    };
    stand(&mut session);
    session.hold_trigger(Trigger {
        primary: true,
        ..Trigger::default()
    });
    let armour = |session: &Session, id: NpcId| {
        let npc = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("still here");
        npc.reserves.shield.now + npc.reserves.armor.now
    };
    let before = [passed, target].map(|id| armour(&session, id));
    let mut chance = SplitMix::new(7);
    for _ in 0..40 {
        stand(&mut session);
        let rescued = session.game_work(
            Controls::default(),
            &game.data,
            &Still,
            Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
    }
    let after = [passed, target].map(|id| armour(&session, id));
    (after[0] < before[0], after[1] < before[1])
}

#[test]
fn the_player_s_light_freighter_carries_its_data_radius_under_either_ruleset() {
    let game = sample();
    let radius = freighter_radius(&game);
    for ruleset in [Ruleset::NewGame, Ruleset::Stock] {
        let session = armed_session(&game, ruleset);
        assert_eq!(session.hull().collision_radius, Some(radius), "{ruleset:?}");
    }
}

#[test]
fn under_the_new_game_a_shot_passing_a_light_freighter_within_its_data_disc_strikes_it() {
    let game = sample();
    let radius = freighter_radius(&game);
    let record = CombatCatalog::hulls(&game.data)
        .into_iter()
        .find(|hull| hull.id == ship(&game, "light-freighter"))
        .expect("the freighter's hull");
    let hull = nova_sim::HullSpec::new(&record);
    let width = hull.sprite_size.expect("its sprite width");
    // Off its sprite (half its width, and a shot's breadth, from its
    // centre) and off its hit radius, but within its data disc.
    let off = radius - 1.0;
    assert!(off > hull.hit_radius && off > width / 2.0 + 4.0);
    let (passed, _) = passing_strikes(Ruleset::NewGame, off);
    assert!(passed, "by its data disc");
    assert_eq!(
        passing_strikes(Ruleset::Stock, off),
        (false, true),
        "by its sprite: the shot flies on past it to the target"
    );
}
