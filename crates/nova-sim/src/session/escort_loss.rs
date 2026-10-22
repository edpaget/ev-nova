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
    session.space.traffic.add_npc(bystander);
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
    let npc = session.space.npc_mut(NpcId(0)).expect("there");
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
    let at = session.space.npc(NpcId(0)).expect("there").state.position;
    let shot = session
        .space
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
    let escort = session.space.npc(NpcId(0)).expect("still here");
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
    session.space.traffic.add_npc(bystander);
    assert!(
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 0))
    );
    let npc = session.space.npc_mut(NpcId(0)).expect("there");
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

/// The player fires until two of its shots are in flight, then lets go
/// of the trigger, its escort NPC 0 still led.
fn fire_until_two_shots(session: &mut Session, catalog: &FakePilotCatalog) {
    session.hold_trigger(FIRE);
    let players = |session: &Session| {
        session
            .space
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
}

/// The player fires until two of its shots are in flight, both then set
/// on a move into NPC 0 for the
/// next pass's collision passes to land: the first made no ship's, so it
/// hits whatever it meets, and the second left the player's own, which
/// passes through a ship the player leads. Gives the second shot's id.
fn two_shots_into_the_escort(session: &mut Session, catalog: &FakePilotCatalog) -> ShotId {
    fire_until_two_shots(session, catalog);
    let at = session.space.npc(NpcId(0)).expect("there").state.position;
    let mut shots = session
        .space
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
        session.space.npc(NpcId(0)).expect("still here").condition,
        crate::combat::hull::Condition::Intact,
        "the first shot disabled it"
    );
    session
        .space
        .combat
        .shots()
        .iter()
        .any(|shot| shot.id == id)
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

/// [`escorted`] (in `catalog`), its bystander NPC 1 with its shield down
/// and `armour` left, and the player's first two shots in flight: the
/// first re-owned by the escort NPC 0, as a shot of the player's fleet
/// would be, aimed at NPC 1 and set on a move into it for the next
/// pass's collision passes to land; the second still the player's, where
/// it was. No pass has run since.
pub(super) fn an_escorts_shot_into_the_bystander(
    catalog: &FakePilotCatalog,
    armour: f32,
) -> Session {
    use crate::combat::sides::Ties;
    let mut session = escorted(catalog);
    let victim = session.space.npc_mut(NpcId(1)).expect("there");
    victim.reserves.shield.now = 0.0;
    victim.reserves.armor.now = armour;
    fire_until_two_shots(&mut session, catalog);
    let victim_at = session.space.npc(NpcId(1)).expect("there").state.position;
    let shot = session
        .space
        .combat
        .shots_mut()
        .iter_mut()
        .find(|shot| shot.firer == ShipRef::Player)
        .expect("a first shot");
    shot.firer = ShipRef::Npc(NpcId(0));
    shot.fleet = ShipRef::Player;
    shot.source = Source::Ship(Ties {
        leader: Some(ShipRef::Player),
        govt: None,
        booty: None,
        planet_type: false,
    });
    shot.aimed_at = Some(ShipRef::Npc(NpcId(1)));
    shot.target = Some(ShipRef::Npc(NpcId(1)));
    shot.moved_from = Some(victim_at + Vec2::new(0.0, 20.0));
    shot.position = victim_at;
    session
}

/// [`an_escorts_shot_into_the_bystander`] with NPC 1 one point of armour
/// from breaking up, and the player's second shot made no ship's and set
/// on a move into the escort, that disables it. The escort's lands first
/// when `kill_first` (it is in the shot layer, walked before the homing
/// layer the other is in), else last. The player's combat rating after
/// the pass.
fn rating_after_an_escorts_kill(kill_first: bool) -> i64 {
    use crate::combat::shot_layer::ShotLayer;
    let catalog = armed();
    let mut session = an_escorts_shot_into_the_bystander(&catalog, 1.0);
    assert_eq!(session.pilot.combat_rating, 0);
    let escort_at = session.space.npc(NpcId(0)).expect("there").state.position;
    let (kill_layer, loss_layer) = if kill_first {
        (ShotLayer::Shots, ShotLayer::Homing)
    } else {
        (ShotLayer::Homing, ShotLayer::Shots)
    };
    let shots = session.space.combat.shots_mut();
    let kill = shots
        .iter_mut()
        .find(|shot| shot.firer == ShipRef::Npc(NpcId(0)))
        .expect("the escort's shot");
    kill.layer = kill_layer;
    let loss = shots
        .iter_mut()
        .find(|shot| shot.firer == ShipRef::Player)
        .expect("a second shot");
    loss.source = Source::Other;
    loss.aimed_at = None;
    loss.layer = loss_layer;
    loss.moved_from = Some(escort_at + Vec2::new(0.0, 20.0));
    loss.position = escort_at;
    session.game_work(
        Controls::default(),
        &catalog,
        &Watching::default(),
        Rules::default(),
        &mut NeverFires,
    );
    assert_ne!(
        session.space.npc(NpcId(0)).expect("still here").condition,
        crate::combat::hull::Condition::Intact,
        "the escort was disabled"
    );
    assert!(
        session
            .space
            .npc(NpcId(1))
            .is_none_or(|npc| npc.condition != crate::combat::hull::Condition::Intact),
        "the victim was killed"
    );
    session.pilot.combat_rating
}

#[test]
fn an_escorts_kill_before_its_loss_in_the_walk_credits_the_player() {
    // `_DamageShip` reads the shooter's leader +0x9a as the kill lands
    // (@0x3ad72): the escort is still the player's.
    assert!(rating_after_an_escorts_kill(true) > 0);
}

#[test]
fn an_escorts_kill_after_its_loss_in_the_walk_credits_no_one() {
    // Its leader was cleared to -1 as the disabling hit landed (@0x3afdd).
    assert_eq!(rating_after_an_escorts_kill(false), 0);
}
