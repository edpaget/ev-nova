//! A ship of the player's fleet that a hit leaves disabled leaves the
//! fleet as the phase's hits settle (rdm weapons-fidelity phase 42), as
//! `_DamageShip`'s escort block (@0x3af8f-0x3b0d7) runs before its
//! reaction block (@0x3b408) and before anything later that frame: the
//! AI (`_AIDispatch` @0x44270) reads it out of the fleet.

use std::cell::RefCell;

use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::projectile::Source;
use crate::combat::{Rules, ShipRef};
use crate::fleet::Escort;
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::session::Session;
use crate::session::tests::{FIRE, armed, facing_an_npc};
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::{Npc, NpcId};

/// Nova's AI, recording on each decision which ships the player leads.
#[derive(Debug, Default)]
struct Watching {
    ai: NovaAi,
    led: RefCell<Vec<Vec<NpcId>>>,
}

impl Behaviour for Watching {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        let led = around
            .npcs
            .iter()
            .filter(|npc| npc.escort.is_some())
            .map(|npc| npc.id)
            .collect();
        self.led.borrow_mut().push(led);
        self.ai.decide(npc, around, chance)
    }
}

/// [`facing_an_npc`] with NPC 0, 100 pixels ahead of the player, enlisted
/// as its escort, its shield down and its armour one point above the
/// line, and a bystander far off to decide each pass.
fn escorted(catalog: &FakePilotCatalog) -> Session {
    let mut session = facing_an_npc(catalog, 180);
    let mut bystander = session.npcs()[0].clone();
    bystander.state.position.x += 3000.0;
    bystander.goal = Goal::Idle;
    session.traffic.add_npc(bystander);
    let ship = session.npcs()[0].ship;
    let reserves = session.npcs()[0].reserves;
    session.enlist(
        NpcId(0),
        Escort {
            ship,
            reserves,
            order: None,
            carried: false,
            wage: None,
            person: None,
        },
    );
    let npc = session.npc_mut(NpcId(0)).expect("there");
    npc.reserves.shield.now = 0.0;
    npc.reserves.armor.now = (npc.reserves.armor.max * 0.34).ceil();
    session
}

/// The player fires one shot, which is then made no ship's and set on a
/// move into NPC 0, for the next pass's collision passes to land.
fn shot_into_the_escort(session: &mut Session, catalog: &FakePilotCatalog) {
    session.hold_trigger(FIRE);
    session.game_work(
        Controls::default(),
        catalog,
        &Watching::default(),
        Rules::default(),
        &mut NeverFires,
    );
    session.hold_trigger(crate::combat::armament::Trigger::default());
    let at = session.npc(NpcId(0)).expect("there").state.position;
    let shot = session
        .combat
        .shots_mut()
        .iter_mut()
        .find(|shot| shot.firer == ShipRef::Player)
        .expect("a shot");
    shot.source = Source::Other;
    shot.aimed_at = None;
    shot.moved_from = Some(at + Vec2::new(0.0, 20.0));
    shot.position = at;
}

#[test]
fn an_escort_disabled_in_the_collision_passes_is_unled_for_the_rest_of_the_frame() {
    let catalog = armed();
    let mut session = escorted(&catalog);
    shot_into_the_escort(&mut session, &catalog);
    let watching = Watching::default();
    session.game_work(
        Controls::default(),
        &catalog,
        &watching,
        Rules::default(),
        &mut NeverFires,
    );
    let escort = session.npc(NpcId(0)).expect("still here");
    assert_ne!(escort.condition, crate::combat::hull::Condition::Intact);
    let led = watching.led.take();
    assert!(!led.is_empty(), "the bystander decided");
    assert!(
        led.iter().all(|led| !led.contains(&NpcId(0))),
        "out of the fleet when the AI runs: {led:?}"
    );
}

#[test]
fn a_trader_escort_disabled_in_the_passes_takes_its_cargo_share_once() {
    let food = crate::market::Good::Commodity(0);
    let mut catalog = armed();
    for record in &mut catalog.ship_records {
        record.inherent_ai = 1;
        record.fields.holds = 20;
    }
    let mut session = escorted(&catalog);
    session.pilot.cargo = std::collections::BTreeMap::from([(food, 10)]);
    shot_into_the_escort(&mut session, &catalog);
    let mut held = Vec::new();
    for _ in 0..3 {
        session.game_work(
            Controls::default(),
            &catalog,
            &Watching::default(),
            Rules::default(),
            &mut NeverFires,
        );
        held.push(session.pilot.cargo.get(&food).copied().unwrap_or(0));
    }
    assert!(held[0] < 10, "its share went as it was lost: {held:?}");
    assert_eq!(held[1..], [held[0], held[0]], "once: {held:?}");
}
