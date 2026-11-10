//! Tests of the session carrying the rulebook's
//! [`RuleKey::DataCollision`] to the fight ([`Session::with_rules`]): a
//! ship whose catalog data gives a collision radius is struck by that disc
//! under the other reading, and by its sprite under the engine's, and a
//! later rulebook's reading takes over at once.

use super::Session;
use crate::RuleKey;
use crate::catalog::{HullRecord, ShipId};
use crate::combat::armament::Trigger;
use crate::combat::data_collision::CollisionShape;
use crate::combat::hull::HullSpec;
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::rulebook::{RuleSource, Rulebook};
use crate::session::tests::{armed, facing_an_npc};
use crate::testkit::{FakePilotCatalog, work};

/// The NPC's class.
const NPC_CLASS: ShipId = ShipId(129);

/// [`armed`]'s catalog, its NPC's class sized like the sample's light
/// freighter: its `shän` 26 by 40, its data radius 20.
fn freighters() -> FakePilotCatalog {
    let mut catalog = armed();
    let npc = catalog
        .hulls
        .iter_mut()
        .find(|hull| hull.id == NPC_CLASS)
        .expect("the NPC's class");
    *npc = HullRecord {
        size: Some(26),
        height: Some(40),
        collision_radius: Some(20),
        ..npc.clone()
    };
    catalog
}

/// The NPC's hull, as the fight measures it.
fn npc_hull(catalog: &FakePilotCatalog) -> HullSpec {
    let record = catalog
        .hulls
        .iter()
        .find(|hull| hull.id == NPC_CLASS)
        .expect("the NPC's class");
    HullSpec::new(record)
}

/// Rules with `data_collision` read as `source`.
fn rules(source: RuleSource) -> Rulebook {
    Rulebook::default().with_override(RuleKey::DataCollision, source)
}

/// `catalog`'s session with the player at rest at the centre, facing up at
/// its NPC 100 above at `npc_x`, its trigger held, by `rules`.
fn shooting(catalog: &FakePilotCatalog, npc_x: f32, rules: Rulebook) -> Session {
    let mut session = facing_an_npc(catalog, 180).with_rules(rules);
    session.ship.player.position = Vec2::ZERO;
    session.ship.player.velocity = Vec2::ZERO;
    session.space.traffic.npcs_mut()[0].state.position = Vec2::new(npc_x, -100.0);
    session.hold_trigger(Trigger {
        primary: true,
        ..Trigger::default()
    });
    session
}

/// Whether the NPC's shield or armour fell in `passes` passes.
fn struck(session: &mut Session, catalog: &FakePilotCatalog, passes: usize) -> bool {
    let before = session.npcs()[0].reserves;
    for _ in 0..passes {
        work(session, catalog, Controls::default());
    }
    let after = session.npcs()[0].reserves;
    after.shield.now < before.shield.now || after.armor.now < before.armor.now
}

/// How far off the player's line of fire the NPC stands: between its hit
/// radius and its data radius.
fn off_line(catalog: &FakePilotCatalog) -> f32 {
    let hull = npc_hull(catalog);
    let disc = CollisionShape::DataDisc.disc(&hull).expect("a data radius");
    assert!(hull.hit_radius < disc);
    f32::midpoint(hull.hit_radius, disc)
}

#[test]
fn the_players_shot_strikes_an_npc_by_its_data_disc_only_under_the_other_reading() {
    let catalog = freighters();
    let x = off_line(&catalog);
    let mut engine = shooting(&catalog, x, rules(RuleSource::Engine));
    assert!(!struck(&mut engine, &catalog, 12), "by its sprite");
    let mut other = shooting(&catalog, x, rules(RuleSource::Bible));
    assert!(struck(&mut other, &catalog, 12), "by its data disc");
}

#[test]
fn a_later_rulebook_s_reading_takes_over_at_once() {
    let catalog = freighters();
    let x = off_line(&catalog);
    let session = shooting(&catalog, x, rules(RuleSource::Engine));
    let mut session = session.with_rules(rules(RuleSource::Bible));
    assert!(struck(&mut session, &catalog, 12), "now by its data disc");
    let mut session = session.with_rules(rules(RuleSource::Engine));
    assert!(!struck(&mut session, &catalog, 12), "by its sprite again");
}
