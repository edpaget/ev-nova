//! Wiring of an NPC's pick at a ship breaking up (rdm
//! escort-fleet-fidelity-followups phase 15): the AI's pick
//! ([`Trigger::only`]) reaches the fight's `_FireAIShipWeapon`, which
//! aims it at any target whose slot is active, breaking up included
//! (combat's `ai_dispatch::active`, whose rules the fight's tests pin).

#![allow(clippy::float_cmp)]

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::chance::{Chance, NeverFires};
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{Rules, ShipRef};
use crate::flight::Controls;
use crate::geometry::Vec2;
use crate::session::tests::{armed, blaster, facing_an_npc};
use crate::stats::ShipStats;
use crate::testkit::{FAST, FakePilotCatalog, npc};
use crate::traffic::npc::{Npc, NpcId};

/// Every NPC but `0` picks its turret at `0`; `0` holds nothing.
#[derive(Debug)]
struct PickingAt(NpcId);

impl Behaviour for PickingAt {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
        if npc.id == self.0 {
            return Trigger::default();
        }
        Trigger {
            only: Some(WeaponId(128)),
            ..Trigger::default()
        }
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        (npc.id != self.0).then_some(ShipRef::Npc(self.0))
    }
}

/// The ships the shooter's turret fires at in an advance pass, its
/// target 40 pixels above it in `condition`; and that target.
fn fired_at(condition: Condition) -> (Vec<Option<ShipRef>>, ShipRef) {
    let catalog = FakePilotCatalog {
        weapons: vec![WeaponRecord {
            guidance: 4,
            ..blaster()
        }],
        ..armed()
    };
    let mut session = facing_an_npc(&catalog, 180);
    let at = Vec2::new(600.0, 0.0);
    let shooter = &mut session.space.traffic.npcs_mut()[0];
    shooter.state.position = at;
    let shooter = ShipRef::Npc(shooter.id);
    let mut target = npc(0, ShipStats::new(FAST, &[]));
    target.state.position = at + Vec2::new(0.0, -40.0);
    target.condition = condition;
    let target = session.space.traffic.add_npc(target);
    let rescued = session.game_work(
        Controls::default(),
        &catalog,
        &PickingAt(target),
        Rules::default(),
        &mut NeverFires,
    );
    assert!(!rescued);
    let fired = session
        .shots()
        .iter()
        .filter(|shot| shot.firer == shooter)
        .map(|shot| shot.target)
        .collect();
    (fired, ShipRef::Npc(target))
}

#[test]
fn an_npcs_turret_pick_fires_at_a_ship_breaking_up() {
    let breaking_up = Condition::Dying {
        ticks_left: 30,
        starting: false,
    };
    let (fired, target) = fired_at(breaking_up);
    assert_eq!(fired, [Some(target)]);
}
