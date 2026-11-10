//! The new game's shots leave from the mounted muzzle: over the modelled
//! sample (`nova_data::new_game::fixture::modelled_sample`), whose mass
//! driver shows a turret model, the player's light freighter carrying the
//! gun mount fires its mass driver at a ship off its nose. Under the
//! rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, the first shot
//! starts the mass driver's muzzle reach further along its aim than over
//! the checked-in sample, whose mass driver has no model; under the stock
//! ruleset's readings it starts at the gun bed's exit, as over the
//! checked-in sample.
//!
//! Where a shot starts under each reading is nova-sim-combat's rule
//! (`muzzle`), and how the reach is read from the outfit's model is
//! nova-data's (`new_game::mounts`), each pinned there; this pins that the
//! ruleset, the data and the fight meet.

use std::path::Path;

use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_data::new_game::{self, NewGame};
use nova_sim::fixture::{place, put_player_at};
use nova_sim::flight::facing;
use nova_sim::{
    Behaviour, Chance, Controls, Goal, Npc, NpcId, OutfitId, Pilot, Rules, Session, ShipId,
    ShipRef, Surroundings, TargetPick, Trigger, Vec2, WeaponId,
};

/// The checked-in sample, mapped: its light freighter a placeholder, its
/// mass driver with no model.
fn plain() -> NewGame {
    new_game::open(&new_game::fixture::sample_folder()).expect("the sample lists")
}

/// The modelled sample, mapped.
fn modelled() -> NewGame {
    new_game::load(&new_game::fixture::modelled_sample(), Path::new("sample"))
}

/// The Nova ID `table` gives the bible id `name`.
fn nova_id<K: std::fmt::Display>(table: &std::collections::BTreeMap<K, i16>, name: &str) -> i16 {
    table
        .iter()
        .find(|(id, _)| id.to_string() == name)
        .map(|(_, &nova)| nova)
        .expect("in the sample")
}

/// The rules `ruleset` pins.
fn rules_of(ruleset: Ruleset) -> nova_sim::Rulebook {
    ruleset_rulebook(None, ruleset.profile().readings).0
}

/// The mass driver's `wëap`.
fn mass_driver(game: &NewGame) -> WeaponId {
    WeaponId(nova_id(&game.ids.weapons, "mass-driver"))
}

/// A new pilot of `game`, its light freighter carrying the gun mount and
/// its mass driver, flying under `ruleset`'s readings.
fn armed_session(game: &NewGame, ruleset: Ruleset) -> Session {
    let mut pilot = Pilot::new(&game.data, "Tester").expect("the sample starts");
    assert_eq!(
        pilot.ship,
        ShipId(nova_id(&game.ids.ships, "light-freighter"))
    );
    for outfit in [
        OutfitId(nova_id(&game.ids.kits, "gun-mount")),
        OutfitId(nova_id(&game.ids.outfits, "mass-driver")),
    ] {
        pilot.outfits.insert(outfit, 1);
    }
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

/// `session`'s system filled with one of its traffic at least, on the
/// first seed that places one.
fn one_ship(session: &mut Session, game: &NewGame) -> NpcId {
    for seed in 0..200 {
        session.populate(&game.data, &mut SplitMix::new(seed));
        if let [first, ..] = session.npcs() {
            return first.id;
        }
    }
    panic!("no seed placed a ship");
}

/// Where the player's first shot of its mass driver starts from its
/// centre, and the heading it flies, fired over `game` under `ruleset`'s
/// readings at a ship 100 ahead and 60 to the right, off its nose.
fn first_shot(game: &NewGame, ruleset: Ruleset) -> (Vec2, f32) {
    let mut session = armed_session(game, ruleset);
    let target = one_ship(&mut session, game);
    let stand = |session: &mut Session| {
        place(session, target, Vec2::new(60.0, -100.0));
        put_player_at(session, Vec2::ZERO);
    };
    stand(&mut session);
    assert_eq!(session.select_target(TargetPick::Nearest), Some(target));
    session.hold_trigger(Trigger {
        primary: true,
        ..Trigger::default()
    });
    let mut chance = SplitMix::new(7);
    for _ in 0..10 {
        stand(&mut session);
        let centre = session.player().position;
        let rescued = session.game_work(
            Controls::default(),
            &game.data,
            &Still,
            Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
        if let Some(shot) = session
            .shots()
            .iter()
            .find(|shot| shot.firer == ShipRef::Player && shot.weapon.id == mass_driver(game))
        {
            return (
                shot.moved_from.unwrap_or(shot.position) - centre,
                shot.heading,
            );
        }
    }
    panic!("the player never fired");
}

fn close_to(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn under_the_new_game_the_first_shot_starts_at_the_mounted_muzzle() {
    let (modelled, plain) = (modelled(), plain());
    let reach = modelled
        .data
        .muzzle_reach(mass_driver(&modelled))
        .expect("the mass driver's reach");
    assert!(reach > 0);
    assert_eq!(plain.data.muzzle_reach(mass_driver(&plain)), None);

    // (a) By the stock readings, from the gun bed's exit, as over the
    // checked-in sample.
    let (stock, _) = first_shot(&modelled, Ruleset::Stock);
    let (stock_plain, _) = first_shot(&plain, Ruleset::Stock);
    assert!(close_to(stock, stock_plain), "{stock:?} {stock_plain:?}");
    assert!(stock.length() > 0.5, "the gun bed's exit, off the centre");

    // (b) By the new game's, its reach further along its aim, aimed the
    // same.
    let (ahead, heading) = first_shot(&modelled, Ruleset::NewGame);
    let (at_exit, aimed) = first_shot(&plain, Ruleset::NewGame);
    assert!((heading - aimed).abs() < 1e-6, "{heading} {aimed}");
    let along = facing(heading) * f32::from(reach);
    assert!(
        close_to(ahead - at_exit, along),
        "{ahead:?} - {at_exit:?} not {along:?}"
    );
}

#[test]
fn over_the_checked_in_sample_the_new_game_s_shot_starts_where_the_stock_one_does() {
    let game = plain();
    let (stock, _) = first_shot(&game, Ruleset::Stock);
    let (new_game, _) = first_shot(&game, Ruleset::NewGame);
    assert!(close_to(stock, new_game), "{stock:?} {new_game:?}");
}
