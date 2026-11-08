//! The sample cut's mod kits played headless through the session's own
//! commands: the pilot fits armour plating at Luyten Exchange, a legal
//! yard, and a gun mount at Marrow Yard, a lawless one, feels each kit's
//! costs, then asks Luyten Exchange to land with the lawless kit aboard
//! and is scanned, fined, convicted and denied.
//!
//! This pins only that the sample's data, the new game's mapping
//! (`nova_data::new_game`), the outfitter and the session meet. The rules
//! are pinned in their crates: the mapping in nova-data, the exit points
//! in nova-sim-combat, the scan's rule in nova-sim-ai, the dock scan and
//! the refit in nova-sim, the wording in nova-view.

use nova_data::new_game::fixture::sample_folder;
use nova_data::records::stellar::Stellar;
use nova_sim::flight_frame::{Answer, FlightCommand, FlightEvent, StepInput, StepReport};
use nova_sim::outfitter::OutfitOrder;
use nova_sim::{
    ApplyCollaborators, Direction, DockFind, GovtId, HailOptions, LandingRefusal, NovaAi,
    NovaBoarding, NovaLaw, OutfitId, Rules, ShipRef, SimMessage, StellarId, StepCollaborators,
    TargetPick, Vec2,
};

use crate::new_game_play::Play;
use crate::new_game_travel::{new_game_rules, nova_of, pass};

/// The sample's start in Luyten Crossing, with 20,000 credits.
fn play() -> Play {
    Play::new(&sample_folder(), "luyten-crossing", |pilot| {
        pilot.cash = 20_000;
    })
}

fn kit(play: &Play, id: &str) -> OutfitId {
    OutfitId(nova_of(&play.game.ids.kits, id))
}

/// The outfits the outfitter where the ship is docked lists.
fn listed(play: &mut Play) -> Vec<OutfitId> {
    let outfitter = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter");
    outfitter.rows.iter().map(|row| row.id).collect()
}

fn buy(play: &mut Play, outfit: OutfitId) {
    let order = OutfitOrder {
        outfit,
        direction: Direction::Buy,
    };
    play.session
        .outfit(order, &mut play.chance)
        .expect("bought");
}

/// Takes off, waits for traffic, targets the nearest ship and fires the
/// primary once beside it: where the player's first shot left from, off
/// the ship's centre.
fn turret_shot_from(play: &mut Play) -> Vec2 {
    play.take_off();
    let rules = new_game_rules();
    for _ in 0..600 {
        if !play.session.npcs().is_empty() {
            break;
        }
        pass(&mut play.session, &play.data, &rules, &mut play.chance);
    }
    assert!(
        play.session.select_target(TargetPick::Nearest).is_some(),
        "traffic to aim at"
    );
    let near = play.session.target().expect("a target").state.position + Vec2::new(150.0, 0.0);
    nova_sim::fixture::put_player_at(&mut play.session, near);
    let centre = play.session.player().position;
    let behaviour = NovaAi::from_rulebook(&rules);
    let input = StepInput {
        window_width: 1024,
        fire: (true, false),
        ..StepInput::default()
    };
    let mut collab = StepCollaborators {
        catalog: &play.data,
        behaviour: &behaviour,
        rules: Rules::default(),
        chance: &mut play.chance,
    };
    play.session
        .step(&input, 1, &mut collab, &mut StepReport::default());
    let shot = play
        .session
        .shots()
        .iter()
        .find(|shot| shot.firer == ShipRef::Player)
        .expect("the mass driver fired");
    shot.moved_from.unwrap_or(shot.position) - centre
}

/// Puts the ship over `stellar` and presses L as `FlightCommand::Land`
/// does, with Nova's law.
fn press_land_over(play: &mut Play, stellar: StellarId) -> Answer {
    let spob = play
        .data
        .get::<Stellar>(stellar.0)
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    let at = Vec2::new(f32::from(spob.x_pos), f32::from(spob.y_pos));
    nova_sim::fixture::put_player_at(&mut play.session, at);
    play.session.apply(
        FlightCommand::Land,
        &mut ApplyCollaborators {
            catalog: Some(&play.data),
            chance: &mut play.chance,
            law: &NovaLaw::default(),
            boarding_rule: &NovaBoarding::default(),
            hail_options: &HailOptions::default(),
        },
    )
}

#[test]
fn a_kit_fitted_at_a_yard_costs_what_it_says_and_draws_the_law() {
    let mut play = play();
    let plating = kit(&play, "armour-plating");
    let mount = kit(&play, "gun-mount");

    // A legal yard lists the legal kit alone.
    play.land("luyten-exchange");
    let legal = listed(&mut play);
    assert!(legal.contains(&plating), "{legal:?}");
    assert!(!legal.contains(&mount), "{legal:?}");

    let before = play.session.stats();
    assert_eq!(before.capacity, 80);
    buy(&mut play, plating);
    let plated = play.session.stats();
    assert_eq!(play.session.pilot().cash(), 14_000);
    assert_eq!(plated.capacity, 70);
    assert!(plated.handling.max_speed < before.handling.max_speed);
    // The light freighter's placeholder turn (handling 35, a quarter of it
    // as `Maneuver`) is under the 1° a tick floor with the kit or without,
    // so its handling cost doesn't show in the turn rate yet: the
    // `new-game-turn-scale` task.
    assert!(plated.handling.turn_rate <= before.handling.turn_rate);
    assert!(plated.armor > before.armor);

    // A lawless yard lists both.
    play.go_through("luyten-crossing/marrow-road");
    play.land("marrow-yard");
    let lawless = listed(&mut play);
    assert!(lawless.contains(&mount), "{lawless:?}");
    buy(&mut play, mount);
    assert_eq!(play.session.stats().capacity, 64);
    let driver = OutfitId(nova_of(&play.game.ids.outfits, "mass-driver"));
    assert_eq!(play.session.pilot().owned(driver), 1, "given with the kit");
    let armed = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter")
        .armed;
    assert_eq!((armed.turrets, armed.turret_limit), (1, 1));

    // The mass driver, a turret, fires at a target from the gun bed, 2
    // pixels off the centre.
    let from = turret_shot_from(&mut play);
    assert!((from.length() - 2.0).abs() < 0.01, "{from:?}");

    // Back at Luyten Exchange the lawless kit is found.
    play.go_through("marrow/luyten-road");
    let exchange = play.stellar("luyten-exchange");
    play.session.drain_events(&mut Vec::new());
    let cash = play.session.pilot().cash();
    let answer = press_land_over(&mut play, exchange);
    assert!(
        matches!(
            answer,
            Answer::LandRefused {
                refusal: LandingRefusal::Denied { .. },
                ..
            }
        ),
        "{answer:?}"
    );
    assert_eq!(play.session.pilot().cash(), cash - 500);
    let mut events = Vec::new();
    play.session.drain_events(&mut events);
    assert!(
        events.contains(&FlightEvent::Message(SimMessage::DockScanned {
            stellar: exchange,
            found: DockFind::Outfit {
                id: mount,
                one: true,
            },
            fine: 500,
        })),
        "{events:?}"
    );
    let crossing = play.system("luyten-crossing");
    assert!(play.session.pilot().system_record(crossing) < 0);
    let commonwealth = GovtId(nova_of(&play.game.ids.factions, "commonwealth"));
    assert!(play.session.pilot().legal_record(commonwealth) < 0);
}
