//! Tests of the session carrying the rulebook's
//! [`RuleKey::MountedMuzzles`] to the fight ([`Session::with_rules`]): a
//! shot whose weapon's catalog data gives a muzzle reach starts that far
//! ahead of its exit along its aim under the other reading, and at its exit
//! under the engine's, and a later rulebook's reading takes over at once.

use super::Session;
use crate::RuleKey;
use crate::catalog::WeaponRecord;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::flight::{Controls, facing};
use crate::geometry::Vec2;
use crate::rulebook::{RuleSource, Rulebook};
use crate::session::tests::{armed, facing_an_npc};
use crate::testkit::{FakePilotCatalog, work};

/// How far ahead of its exit the player's turret's muzzle is, in pixels.
const REACH: i16 = 5;

/// [`armed`]'s catalog, its blaster a turret (guidance 4) whose muzzle
/// is [`REACH`] ahead of its exit.
fn turreted() -> FakePilotCatalog {
    let mut catalog = armed();
    let blaster = &mut catalog.weapons[0];
    *blaster = WeaponRecord {
        guidance: 4,
        muzzle_reach: Some(REACH),
        ..*blaster
    };
    catalog
}

/// Rules with `mounted_muzzles` read as `source`.
fn rules(source: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::MountedMuzzles, source)
}

/// `catalog`'s session with the player at rest at the centre, facing up,
/// its trigger held, its NPC 100 above and 60 to the right, off its nose,
/// by `rules`.
fn shooting(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
    let mut session = facing_an_npc(catalog, 180).with_rules(rules);
    session.ship.player.position = Vec2::ZERO;
    session.ship.player.velocity = Vec2::ZERO;
    session.space.traffic.npcs_mut()[0].state.position = Vec2::new(60.0, -100.0);
    // A turret fires at the player's target.
    session.select_target(crate::TargetPick::Nearest);
    session.hold_trigger(Trigger {
        primary: true,
        ..Trigger::default()
    });
    session
}

/// Where the player's newest shot started from its centre, after a pass,
/// and the heading it flies.
fn next_shot(session: &mut Session, catalog: &FakePilotCatalog) -> (Vec2, f32) {
    let centre = session.player().position;
    let newest = |session: &Session| {
        session
            .shots()
            .iter()
            .filter(|shot| shot.firer == ShipRef::Player)
            .max_by_key(|shot| shot.id)
            .copied()
    };
    let before = newest(session).map(|shot| shot.id);
    work(session, catalog, Controls::default());
    let shot = newest(session)
        .filter(|shot| Some(shot.id) != before)
        .expect("the player fired");
    (
        shot.moved_from.unwrap_or(shot.position) - centre,
        shot.heading,
    )
}

fn close_to(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn the_players_turret_shot_starts_its_reach_along_its_aim_only_under_the_other_reading() {
    let catalog = turreted();
    let mut engine = shooting(&catalog, rules(RuleSource::Engine));
    let (at_exit, aimed) = next_shot(&mut engine, &catalog);
    assert!(close_to(at_exit, Vec2::ZERO), "at its exit: {at_exit:?}");
    let mut other = shooting(&catalog, rules(RuleSource::Bible));
    let (ahead, heading) = next_shot(&mut other, &catalog);
    assert!((heading - aimed).abs() < 1e-6, "aimed the same both ways");
    assert!(heading.abs() > 1.0, "off the hull's nose: {heading}");
    let along = facing(heading) * f32::from(REACH);
    assert!(close_to(ahead, along), "{ahead:?} not {along:?}");
}

#[test]
fn a_later_rulebook_s_reading_takes_over_at_once() {
    let catalog = turreted();
    let session = shooting(&catalog, rules(RuleSource::Engine));
    let mut session = session.with_rules(rules(RuleSource::Bible));
    let (ahead, heading) = next_shot(&mut session, &catalog);
    let along = facing(heading) * f32::from(REACH);
    assert!(close_to(ahead, along), "now ahead: {ahead:?}");
    let mut session = session.with_rules(rules(RuleSource::Engine));
    // Past its reload, so it fires again.
    for _ in 0..3 {
        work(&mut session, &catalog, Controls::default());
    }
    let (at_exit, _) = next_shot(&mut session, &catalog);
    assert!(
        close_to(at_exit, Vec2::ZERO),
        "at its exit again: {at_exit:?}"
    );
}
