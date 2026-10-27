//! The session's tests of the order between the fight's phases within a
//! pass (rdm weapons-fidelity phase 28, moved up from the fight's
//! `tick_phases` tests): `_HandlePlayer`'s trigger (@0x6ab55), the
//! collision passes (@0x43e70-0x44015), `_AIDispatch`'s point defence
//! (@0x4429d), `_HandleShot` (@0x44385), then `_HandleShip`'s picks
//! (@0x44483), as `Session::pass_after_move` runs them.

use super::tests::{FIRE, arming, facing_an_npc, with_an_npc_at};
use super::*;
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::WeaponRecord;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::armament::Armament;
use crate::combat::weapon::WeaponSpec;
use crate::flight::Controls;
use crate::testkit::{FakePilotCatalog, weapon, work, work_with};
use crate::traffic::npc::Npc;
use std::cell::Cell;

/// NPC 0.
const NPC: ShipRef = ShipRef::Npc(NpcId(0));

/// A blaster firing every other tick, 20 pixels a tick for 30 ticks:
/// weapon `id`.
fn blaster(id: i16) -> WeaponRecord {
    WeaponRecord {
        id: WeaponId(id),
        ..super::tests::blaster()
    }
}

/// A one-at-a-time weapon (`Flags3` 0x0004) of Reload 3, 10 pixels a
/// tick for 10 ticks: weapon `id`.
fn single(id: i16) -> WeaponRecord {
    WeaponRecord {
        reload: 3,
        count: 10,
        speed: 1000,
        flags3: 0x0004,
        ..weapon(id)
    }
}

/// `session`'s NPC 0 holding its place, its trigger `trigger`.
fn hold(session: &mut Session, trigger: Trigger) {
    let npc = &mut session.space.traffic.npcs_mut()[0];
    npc.goal = Goal::Hold;
    npc.trigger = trigger;
}

/// Whether `ship` fired among `events`.
fn fired_by(events: &[CombatEvent], ship: ShipRef) -> bool {
    events
        .iter()
        .any(|event| matches!(event, CombatEvent::Fired { ship: by, .. } if *by == ship))
}

/// The passes, from 0, of the first `passes` of `session` on which `ship`
/// fired.
fn fire_passes(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    ship: ShipRef,
    passes: u32,
) -> Vec<u32> {
    session.take_combat_events();
    (0..passes)
        .filter(|_| {
            work(session, catalog, Controls::default());
            fired_by(&session.take_combat_events(), ship)
        })
        .collect()
}

#[test]
fn an_npcs_shot_first_moves_in_the_pass_after_it_is_fired() {
    // `_HandleShip` fires the pick (@0x34ffd) after `_HandleShot` has
    // moved the shots (@0x44385), so its shot first moves in the next
    // pass's `_HandleShot`.
    let catalog = super::tests::armed();
    let mut session = facing_an_npc(&catalog, 180);
    hold(&mut session, FIRE);
    work(&mut session, &catalog, Controls::default());
    let of_the_npc = |session: &Session| -> Vec<Shot> {
        session
            .shots()
            .iter()
            .filter(|shot| shot.firer == NPC)
            .copied()
            .collect()
    };
    let [fired] = of_the_npc(&session)[..] else {
        panic!("one shot: {:?}", session.shots());
    };
    assert_eq!(fired.moved_from, None, "not moved");
    hold(&mut session, Trigger::default());
    work(&mut session, &catalog, Controls::default());
    let [moved] = of_the_npc(&session)[..] else {
        panic!("one shot: {:?}", session.shots());
    };
    assert_eq!(moved.moved_from, Some(fired.position), "moved once");
    let step = moved.position - fired.position;
    assert!((step.length() - 20.0).abs() < 1e-3, "{step:?}");
}

#[test]
fn an_npcs_shot_is_first_tested_two_passes_after_it_is_fired() {
    // Fired on pass 1, its move onto the player 20 below made on pass 2,
    // that move tested by pass 3's collision passes.
    let catalog = super::tests::armed();
    let mut session = with_an_npc_at(&catalog, 750, 730, 180);
    assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -20.0));
    hold(&mut session, FIRE);
    let full = session.reserves().shield.now;
    let mut struck_on = None;
    for pass in 1..=4 {
        work(&mut session, &catalog, Controls::default());
        hold(&mut session, Trigger::default());
        if session.reserves().shield.now < full {
            struck_on = Some(pass);
            break;
        }
    }
    assert_eq!(struck_on, Some(3));
}

#[test]
fn an_npcs_one_at_a_time_weapon_fires_again_reload_ticks_after_its_shots_last_move() {
    // `_HandleShot` holds the reload at 3 on the shot's every move
    // (@0x35c09), then `_HandleShip` counts it down (@0x33896) before the
    // pick (@0x34ffd): fired on pass 0, the shot moves on passes 1 to 10,
    // the 10th move spending it, and the reload reads 2, 1 then 0 on
    // passes 10 to 12.
    let catalog = arming(blaster(128), single(129), 0);
    // Facing away from the player, its shots meet no one.
    let mut session = facing_an_npc(&catalog, 0);
    hold(&mut session, FIRE);
    assert_eq!(
        fire_passes(&mut session, &catalog, NPC, 25),
        [0, 12, 24],
        "Reload ticks after its last move"
    );
    // It moves onto the player, 50 below, on pass 5, and the collision
    // passes take it on pass 6, before `_HandleShot` could hold the
    // reload again.
    let mut session = with_an_npc_at(&catalog, 750, 700, 180);
    hold(&mut session, FIRE);
    assert_eq!(
        fire_passes(&mut session, &catalog, NPC, 9),
        [0, 7],
        "Reload ticks after its last move, onto its victim"
    );
}

#[test]
fn the_players_one_at_a_time_weapon_fires_again_reload_ticks_after_its_shots_last_move() {
    // `_HandlePlayer` counts its reloads down (@0x6ae25-0x6ae61) after
    // its trigger (@0x6ab55) and before `_HandleShot` holds them: its
    // shot moves on the pass it is fired, passes 0 to 9, and the reload
    // reads 2, 1 then 0 after the triggers of passes 10 to 12.
    let catalog = arming(single(128), blaster(129), 0);
    // The NPC behind the player, out of its shots' way.
    let mut session = with_an_npc_at(&catalog, 750, 850, 0);
    session.hold_trigger(FIRE);
    assert_eq!(
        fire_passes(&mut session, &catalog, ShipRef::Player, 14),
        [0, 13]
    );
}

/// NPC 0 holds its place, quiet, until `fighting` is set; then it attacks
/// the player, holding its pick and its point defence.
#[derive(Debug, Default)]
struct Waiting {
    fighting: Cell<bool>,
}

impl Behaviour for Waiting {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if self.fighting.get() {
            Goal::Attack(ShipRef::Player)
        } else {
            Goal::Hold
        }
    }

    fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
        Trigger {
            point_defence: self.fighting.get(),
            ..if self.fighting.get() {
                FIRE
            } else {
                Trigger::default()
            }
        }
    }

    fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        Some(ShipRef::Player)
    }
}

#[test]
fn a_spent_shots_blast_lands_after_the_npcs_point_defence_and_before_their_picks() {
    // `_AIDispatch` runs the NPCs' point defence (@0x4429d), then
    // `_HandleShot` the spent shell's end of life (@0x36195), then
    // `_HandleShip` the picks (@0x44483). The player, of a class point
    // defence fires on (`Flags2` 0x0008), fires a flak shell on pass 1
    // that passes 30 to the left of the NPC, its third move spending it on
    // pass 3; on pass 4 the NPC, carrying the Quad and a pick, attacks it,
    // and the shell's blast lands. Frail, the NPC is disabled then: its
    // point defence fires and its pick does not.
    let flak = WeaponRecord {
        count: 3,
        speed: 1000,
        blast_radius: 40,
        flags: 0x8000,
        ..blaster(128)
    };
    let quad = WeaponRecord {
        guidance: 9,
        reload: 5,
        count: 12,
        speed: 2000,
        mass_dmg: 1,
        energy_dmg: 4,
        ..weapon(133)
    };
    let pick = WeaponRecord {
        reload: 10,
        ..weapon(129)
    };
    let mut catalog = arming(flak, pick, 0);
    catalog.weapons.push(quad);
    catalog.hulls[0].flags2 = 0x0088;
    for (armor, disabled) in [(16.0, true), (45.0, false)] {
        let mut session = with_an_npc_at(&catalog, 780, 720, 0);
        let npc = &mut session.space.traffic.npcs_mut()[0];
        assert_eq!(npc.state.position, Vec2::new(30.0, -30.0));
        npc.armament = Armament::new([(WeaponSpec::new(&quad), 1), (WeaponSpec::new(&pick), 1)]);
        npc.reserves.shield.now = 0.0;
        npc.reserves.armor.now = armor;
        let waiting = Waiting::default();
        let pass = |session: &mut Session| {
            work_with(
                session,
                &catalog,
                Controls::default(),
                &waiting,
                Rules::default(),
                &mut NeverFires,
            );
        };
        session.hold_trigger(FIRE);
        pass(&mut session);
        session.hold_trigger(Trigger::default());
        pass(&mut session);
        pass(&mut session);
        assert_eq!(session.npcs()[0].condition, Condition::Intact, "{armor}");
        session.take_combat_events();
        waiting.fighting.set(true);
        pass(&mut session);
        let condition = session.npcs()[0].condition;
        assert_eq!(
            condition == Condition::Disabled,
            disabled,
            "{armor}: {condition:?}"
        );
        let fired: Vec<WeaponId> = session
            .take_combat_events()
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired { ship, weapon, .. } if ship == NPC => Some(weapon),
                _ => None,
            })
            .collect();
        let expected: &[WeaponId] = if disabled {
            &[WeaponId(133)]
        } else {
            &[WeaponId(133), WeaponId(129)]
        };
        assert_eq!(fired, expected, "{armor}: its point defence, then its pick");
    }
}
