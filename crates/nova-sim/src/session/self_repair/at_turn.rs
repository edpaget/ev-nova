//! Tests of where in a pass an NPC repairs itself: at its own turn of the
//! ship loop (`_HandleShip` @0x33950-0x33bdb, after its slot test
//! @0x33865 and before its move @0x34274), so against each hit site's
//! phase as the original has it.
//!
//! These observe only the session's ordering. What a disabled ship
//! answers and what `_AIDispatch`'s disabled reset wipes are
//! `nova_sim_ai`'s to pin (`nova_sim_ai::traffic::in_turn`,
//! `nova_sim_ai::traffic::disabled_reset`). An intact observer X, in a
//! slot above the disabled ship W, records W's condition each time its
//! think is asked: what the AI loop sees of W at that pass's dispatch,
//! where the reset reads it.

use std::cell::{Cell, RefCell};

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::WeaponRecord;
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{Rules, ShipRef};
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::reserves::Reserves;
use crate::self_repair::{REPAIR_ODDS, repaired_armor};
use crate::session::Session;
use crate::session::tests::{armed, blaster, facing_an_npc};
use crate::stats::ShipStats;
use crate::testkit::{FakePilotCatalog, Ranged, work};
use crate::traffic::npc::{Npc, NpcId};

/// Every NPC keeps its goal; X records W's condition at each think.
#[derive(Debug)]
struct Watching {
    w: NpcId,
    x: NpcId,
    pass: Cell<u32>,
    seen: RefCell<Vec<(u32, Condition)>>,
}

impl Watching {
    fn new(w: NpcId, x: NpcId) -> Self {
        Self {
            w,
            x,
            pass: Cell::new(0),
            seen: RefCell::new(Vec::new()),
        }
    }

    /// W's conditions X saw at pass `pass`'s dispatch.
    fn at(&self, pass: u32) -> Vec<Condition> {
        self.seen
            .borrow()
            .iter()
            .filter(|&&(at, _)| at == pass)
            .map(|&(_, condition)| condition)
            .collect()
    }
}

impl Behaviour for Watching {
    fn decide(&self, npc: &Npc, around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if npc.id == self.x
            && let Some(w) = around.npc(self.w)
        {
            self.seen.borrow_mut().push((self.pass.get(), w.condition));
        }
        npc.goal
    }
}

/// `catalog`'s session with W, its one NPC, 100 above the player, of 10
/// shield and 30 armour and carrying the repair gear, and X, intact, far
/// off in a higher slot: the session, W and X.
fn scene(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId) {
    let mut session = facing_an_npc(catalog, 180);
    let w = session.npcs()[0].id;
    let npc = session.space.npc_mut(w).expect("there");
    npc.reserves = Reserves::full(10.0, 30.0, 100.0);
    npc.self_repair = true;
    npc.goal = Goal::Idle;
    npc.target = None;
    let berth = npc.berth;
    let mut x = crate::testkit::npc(0, ShipStats::default());
    x.state.position = Vec2::new(2000.0, 2000.0);
    let x = session.space.traffic.add_npc_at(x, berth + 4);
    session.prepare_all_sprites();
    (session, w, x)
}

/// An advance pass of `session`, numbered `at` for `watching`, every
/// repair draw coming up 0.
fn pass(session: &mut Session, catalog: &FakePilotCatalog, watching: &Watching, at: u32) {
    watching.pass.set(at);
    let mut chance = Ranged::of(&[(REPAIR_ODDS, 0)]);
    let rescued = session.game_work(
        Controls::default(),
        catalog,
        watching,
        Rules::default(),
        &mut chance,
    );
    assert!(!rescued);
}

fn condition(session: &Session, id: NpcId) -> Condition {
    session.space.npc(id).expect("there").condition
}

fn armor(session: &Session, id: NpcId) -> f32 {
    session.space.npc(id).expect("there").reserves.armor.now
}

/// A beam 200 long, of 25 mass and 10 energy, for one tick.
fn ray() -> WeaponRecord {
    WeaponRecord {
        guidance: 0,
        count: 1,
        reload: 30,
        speed: 0,
        beam_length: 200,
        mass_dmg: 25,
        energy_dmg: 10,
        ..blaster()
    }
}

/// [`scene`] with the player's beam firing in pass 1, disabling W in its
/// `_HandleBeams` (@0x4453d), after the ship loop.
fn beamed() -> (Session, FakePilotCatalog, Watching, NpcId) {
    let catalog = FakePilotCatalog {
        weapons: vec![ray()],
        ..armed()
    };
    let (mut session, w, x) = scene(&catalog);
    let watching = Watching::new(w, x);
    session.hold_fire(true, false);
    pass(&mut session, &catalog, &watching, 1);
    session.hold_fire(false, false);
    (session, catalog, watching, w)
}

#[test]
fn a_ship_a_beam_disables_is_still_disabled_at_the_next_dispatch() {
    // `_HandleBeams` runs after the ship loop, so the ship can repair only
    // at the next frame's `_HandleShip`, after that frame's `_AIDispatch`.
    let (mut session, catalog, watching, w) = beamed();
    assert_eq!(watching.at(1), [Condition::Intact], "before the beam");
    pass(&mut session, &catalog, &watching, 2);
    assert_eq!(watching.at(2), [Condition::Disabled]);
    assert_eq!(condition(&session, w), Condition::Intact, "at its turn");
}

#[test]
fn a_ship_a_beam_disables_is_drawn_disabled_that_pass() {
    let (session, _, _, w) = beamed();
    assert_eq!(condition(&session, w), Condition::Disabled);
}

/// [`scene`] with the player's two shots, fired and still flying; `set`
/// puts the first where it lands this pass. The session, W, and its
/// watcher.
fn shot_at(
    set: impl Fn(&mut crate::combat::projectile::Shot, Vec2, NpcId),
) -> (Session, NpcId, Watching) {
    let catalog = armed();
    let (mut session, w, x) = scene(&catalog);
    session.hold_fire(true, false);
    for _ in 0..10 {
        if session.shots().len() >= 2 {
            break;
        }
        work(&mut session, &catalog, Controls::default());
    }
    session.hold_fire(false, false);
    let npc = session.space.npc_mut(w).expect("there");
    npc.reserves = Reserves::full(10.0, 30.0, 100.0);
    npc.condition = Condition::Intact;
    let w_at = npc.state.position;
    let far = Vec2::new(-3000.0, -3000.0);
    let step = Vec2::new(10.0, 0.0);
    let shots = session.space.combat.shots_mut();
    assert!(shots.len() >= 2, "fired");
    for shot in shots.iter_mut() {
        shot.moved_from = Some(far - step);
        shot.position = far;
    }
    set(&mut shots[0], w_at, w);
    (session, w, Watching::new(w, x))
}

#[test]
fn a_ship_disabled_in_handle_shot_is_repaired_at_its_turn_that_pass() {
    // The detonating shell's blast lands in `_HandleShot` (@0x44385),
    // after the AI loop and before W's `_HandleShip` (@0x44483).
    let (mut session, w, watching) = shot_at(|shell, w_at, _| {
        shell.weapon.flags |= crate::combat::weapon::DETONATES;
        shell.weapon.blast_radius = 120.0;
        shell.weapon.mass_damage = 25.0;
        shell.weapon.energy_damage = 10.0;
        let at = w_at + Vec2::new(0.0, -100.0);
        shell.moved_from = Some(at - Vec2::new(10.0, 0.0));
        shell.position = at;
        shell.age = shell.weapon.lifetime;
    });
    let catalog = armed();
    pass(&mut session, &catalog, &watching, 1);
    assert_eq!(
        watching.at(1),
        [Condition::Intact],
        "hit after the dispatch"
    );
    assert_eq!(
        condition(&session, w),
        Condition::Intact,
        "repaired at its turn"
    );
    assert_eq!(armor(&session, w), repaired_armor(30.0, false), "repaired");
    pass(&mut session, &catalog, &watching, 2);
    assert_eq!(watching.at(2), [Condition::Intact]);
}

#[test]
fn a_ship_disabled_in_the_collision_passes_is_disabled_at_that_dispatch() {
    // The collision passes (@0x43e70-0x44015) come before the AI loop
    // (@0x44270), and the repair after it, at W's `_HandleShip`.
    let (mut session, w, watching) = shot_at(|shot, w_at, w| {
        shot.moved_from = Some(w_at - Vec2::new(10.0, 0.0));
        shot.position = w_at;
        shot.aimed_at = Some(ShipRef::Npc(w));
        shot.weapon.mass_damage = 25.0;
        shot.weapon.energy_damage = 10.0;
    });
    let catalog = armed();
    pass(&mut session, &catalog, &watching, 1);
    assert_eq!(watching.at(1), [Condition::Disabled]);
    assert_eq!(
        condition(&session, w),
        Condition::Intact,
        "repaired at its turn"
    );
    assert_eq!(armor(&session, w), repaired_armor(30.0, false), "repaired");
    pass(&mut session, &catalog, &watching, 2);
    assert_eq!(watching.at(2), [Condition::Intact]);
}

#[test]
fn a_ship_boarded_is_disabled_at_the_dispatch_that_answers_the_call() {
    // The board's Good Samaritan call (`_HandlePlayer` @0x43e19) is
    // answered before the AI loop, the ship boarded still disabled there.
    let catalog = armed();
    let (mut session, w, x) = scene(&catalog);
    let watching = Watching::new(w, x);
    let npc = session.space.npc_mut(w).expect("there");
    npc.reserves.armor.now = 3.0;
    npc.condition = Condition::Disabled;
    session.space.call_samaritans_to(w);
    pass(&mut session, &catalog, &watching, 1);
    assert_eq!(watching.at(1), [Condition::Disabled]);
    assert_eq!(
        condition(&session, w),
        Condition::Intact,
        "repaired at its turn"
    );
    assert_eq!(armor(&session, w), repaired_armor(30.0, false), "repaired");
}

#[test]
fn a_ship_disabled_before_double_times_second_pass_is_repaired_at_its_turn_of_it() {
    // `_DoPlayGameWork(0)` runs `_HandleShip` (@0x44350) with no
    // `_AIDispatch` (@0x4402a), so the repair (@0x33950-0x33bdb) comes
    // with it, and the next advance pass's dispatch reads W whole.
    let catalog = armed();
    let (mut session, w, x) = scene(&catalog);
    let watching = Watching::new(w, x);
    let npc = session.space.npc_mut(w).expect("there");
    npc.reserves.armor.now = 3.0;
    npc.condition = Condition::Disabled;
    let mut chance = Ranged::of(&[(REPAIR_ODDS, 0)]);
    session.tick_again(
        Controls::default(),
        &watching,
        Rules::default(),
        &mut chance,
    );
    assert_eq!(watching.at(0), [], "no dispatch in the second pass");
    assert_eq!(
        condition(&session, w),
        Condition::Intact,
        "repaired at its turn"
    );
    assert_eq!(armor(&session, w), repaired_armor(30.0, false), "repaired");
    pass(&mut session, &catalog, &watching, 1);
    assert_eq!(watching.at(1), [Condition::Intact]);
}

#[test]
fn a_ship_repaired_at_its_turn_flies_that_turn_as_an_intact_ship() {
    // Its move (@0x34274) comes after its repair (@0x33a3b): no disabled
    // drag (@0x33950-0x339a1).
    let catalog = armed();
    let (mut session, w, x) = scene(&catalog);
    let watching = Watching::new(w, x);
    let npc = session.space.npc_mut(w).expect("there");
    npc.reserves.armor.now = 3.0;
    npc.condition = Condition::Disabled;
    npc.state.velocity = Vec2::new(2.0, 0.0);
    let before = npc.state;
    pass(&mut session, &catalog, &watching, 1);
    let after = session.space.npc(w).expect("there").state;
    assert_eq!(after.velocity, before.velocity, "undragged");
    assert_eq!(after.position, before.position + before.velocity);
}

/// W fires its primary at the player, as the Samaritan's write would have
/// it attack; every other NPC keeps its goal.
#[derive(Debug)]
struct FiringW(NpcId);

impl Behaviour for FiringW {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> crate::combat::armament::Trigger {
        if npc.id == self.0 {
            crate::session::tests::FIRE
        } else {
            crate::combat::armament::Trigger::default()
        }
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        (npc.id == self.0).then_some(ShipRef::Player)
    }
}

#[test]
fn a_disabled_ship_carrying_the_write_drifts_and_fires_nothing_at_its_turn() {
    // With no gear to repair it, the write's goal and target carry it
    // nowhere: a disabled ship's flight is a drift (@0x33950-0x339a1) and
    // its fire is gated on its condition.
    let catalog = armed();
    let (mut session, w, _) = scene(&catalog);
    let npc = session.space.npc_mut(w).expect("there");
    npc.self_repair = false;
    npc.reserves.armor.now = 3.0;
    npc.condition = Condition::Disabled;
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    npc.state.velocity = Vec2::new(2.0, 0.0);
    session.take_combat_events();
    let rescued = session.game_work(
        Controls::default(),
        &catalog,
        &FiringW(w),
        Rules::default(),
        &mut crate::chance::NeverFires,
    );
    assert!(!rescued);
    let events = session.take_combat_events();
    assert!(
        !events.iter().any(|event| matches!(
            event,
            crate::combat::CombatEvent::Fired { ship, .. } if *ship == ShipRef::Npc(w)
        )),
        "{events:?}"
    );
    let npc = session.space.npc(w).expect("there");
    assert_eq!(npc.condition, Condition::Disabled);
    assert!(npc.state.velocity.x < 2.0, "dragged: {:?}", npc.state);
}

/// Wiring only, over the real AI (`NovaAi`), which the full gate runs:
/// the beam case end to end. The player's beam disables W in pass 1 and
/// W, its direct victim, turns on the player (`nova_sim_ai`'s to pin);
/// pass 2's dispatch reads it disabled and resets it before its turn
/// repairs it.
#[test]
fn wiring_a_beams_victim_loses_its_turn_on_the_player_at_the_next_dispatch() {
    let catalog = FakePilotCatalog {
        weapons: vec![ray()],
        ..armed()
    };
    let (mut session, w, _) = scene(&catalog);
    let ai = crate::ai::NovaAi::default();
    let mut chance = Ranged::of(&[(REPAIR_ODDS, 0)]);
    // Its target, so the beam's hit is the one it was meant for, no
    // stray hit to weigh.
    session.chart.target = Some(w);
    let mut run = |session: &mut Session| {
        let rescued = session.game_work(
            Controls::default(),
            &catalog,
            &ai,
            Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
    };
    session.hold_fire(true, false);
    run(&mut session);
    session.hold_fire(false, false);
    let npc = session.space.npc(w).expect("there");
    assert_eq!(npc.condition, Condition::Disabled);
    assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
    run(&mut session);
    let npc = session.space.npc(w).expect("there");
    assert_eq!(npc.condition, Condition::Intact, "repaired at its turn");
    assert_eq!((npc.goal, npc.target), (Goal::Idle, None), "reset first");
}
