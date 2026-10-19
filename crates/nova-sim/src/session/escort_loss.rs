//! A ship of the player's fleet that a hit leaves disabled leaves the
//! fleet as the phase's hits settle (rdm weapons-fidelity phase 42), as
//! `_DamageShip`'s escort block (@0x3af8f-0x3b0d7) runs before its
//! reaction block (@0x3b408) and before anything later that frame: the
//! AI (`_AIDispatch` @0x44270) reads it out of the fleet.

use std::cell::RefCell;

use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
use crate::chance::{Chance, NeverFires};
use crate::combat::projectile::{ShotId, Source};
use crate::combat::{Rules, ShipRef};
use crate::escort::FormationSlot;
use crate::fleet::Escort;
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::mission::ships::ShipBehav;
use crate::session::Session;
use crate::session::tests::{FIRE, armed, facing_an_npc};
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::{MissionMark, Npc, NpcId};

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

/// [`escorted`], but with NPC 0 a mission escort that has joined the
/// player's formation in place of an enlisted one, its mission held in
/// slot 0.
fn mission_escorted(catalog: &FakePilotCatalog) -> Session {
    let mut session = facing_an_npc(catalog, 180);
    let mut bystander = session.npcs()[0].clone();
    bystander.state.position.x += 3000.0;
    bystander.goal = Goal::Idle;
    session.traffic.add_npc(bystander);
    assert!(
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 0))
    );
    let npc = session.npc_mut(NpcId(0)).expect("there");
    npc.mission = Some(MissionMark {
        slot: 0,
        behav: ShipBehav::EscortPlayer,
        formation: Some(FormationSlot::default()),
        order: None,
    });
    npc.reserves.shield.now = 0.0;
    npc.reserves.armor.now = (npc.reserves.armor.max * 0.34).ceil();
    assert!(session.npcs()[0].escorts_player());
    session
}

/// The player fires until two of its shots are in flight, both then set
/// on a move into NPC 0 for the
/// next pass's collision passes to land: the first made no ship's, so it
/// hits whatever it meets, and the second left the player's own, which
/// passes through a ship the player leads. Gives the second shot's id.
fn two_shots_into_the_escort(session: &mut Session, catalog: &FakePilotCatalog) -> ShotId {
    session.hold_trigger(FIRE);
    let players = |session: &Session| {
        session
            .combat
            .shots()
            .iter()
            .filter(|shot| shot.firer == ShipRef::Player)
            .count()
    };
    for _ in 0..60 {
        if players(session) >= 2 {
            break;
        }
        session.game_work(
            Controls::default(),
            catalog,
            &Watching::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }
    session.hold_trigger(crate::combat::armament::Trigger::default());
    assert!(session.npcs()[0].escorts_player(), "still led");
    let at = session.npc(NpcId(0)).expect("there").state.position;
    let mut shots = session
        .combat
        .shots_mut()
        .iter_mut()
        .filter(|shot| shot.firer == ShipRef::Player);
    let first = shots.next().expect("a first shot");
    first.source = Source::Other;
    first.aimed_at = None;
    first.moved_from = Some(at + Vec2::new(0.0, 20.0));
    first.position = at;
    let second = shots.next().expect("a second shot");
    second.moved_from = Some(at + Vec2::new(0.0, 20.0));
    second.position = at;
    second.id
}

/// Whether the shot `id` is still in flight after the next pass.
fn passes_through(session: &mut Session, catalog: &FakePilotCatalog, id: ShotId) -> bool {
    session.game_work(
        Controls::default(),
        catalog,
        &Watching::default(),
        Rules::default(),
        &mut NeverFires,
    );
    assert_ne!(
        session.npc(NpcId(0)).expect("still here").condition,
        crate::combat::hull::Condition::Intact,
        "the first shot disabled it"
    );
    session.combat.shots().iter().any(|shot| shot.id == id)
}

#[test]
fn the_players_next_shot_in_the_pass_strikes_an_enlisted_escort_just_disabled() {
    let catalog = armed();
    let mut session = escorted(&catalog);
    let second = two_shots_into_the_escort(&mut session, &catalog);
    assert!(!passes_through(&mut session, &catalog, second));
}

#[test]
fn the_players_next_shot_in_the_pass_still_passes_a_mission_escort_just_disabled() {
    let catalog = armed();
    let mut session = mission_escorted(&catalog);
    let second = two_shots_into_the_escort(&mut session, &catalog);
    assert!(passes_through(&mut session, &catalog, second));
}

#[test]
fn only_an_enlisted_escort_fights_as_a_ship_of_the_fleet() {
    let catalog = armed();
    let enlisted = escorted(&catalog);
    let (player, npcs) = enlisted.fight_terms();
    assert!(!player.fleet_escort);
    let fleet: Vec<bool> = npcs.iter().map(|terms| terms.fleet_escort).collect();
    assert_eq!(fleet, [true, false], "the escort, then the bystander");
    let mission = mission_escorted(&catalog);
    let (_, npcs) = mission.fight_terms();
    let fleet: Vec<bool> = npcs.iter().map(|terms| terms.fleet_escort).collect();
    assert_eq!(
        fleet,
        [false, false],
        "the mission escort, then the bystander"
    );
}
