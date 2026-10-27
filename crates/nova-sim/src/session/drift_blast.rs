//! The session's tests of where a spent shot's blast meets a drifting NPC
//! (rdm view-sim-seam phase 18): `_AIDispatch` (@0x4429d) runs each NPC's
//! `_HighLevelAIHandler`, whose goal 0xe drifts the ship (@0x8eb6b-0x8ebd4),
//! before `_HandleShot` (@0x44385), whose blast reads each ship's record
//! live (+0x18 and +0x1c, @0x3603c-0x3605b). So the blast meets the ship
//! where that pass's drift left it.

use super::tests::{FIRE, arming, with_an_npc_at};
use super::*;
use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::WeaponRecord;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::flight::Controls;
use crate::testkit::{weapon, work_with};
use crate::traffic::npc::Npc;
use std::cell::Cell;

/// NPC 0 holds its place until `drifting` is set; then it drifts.
#[derive(Debug, Default)]
struct Drifting {
    drifting: Cell<bool>,
}

impl Behaviour for Drifting {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if self.drifting.get() {
            Goal::Drift
        } else {
            Goal::Hold
        }
    }
}

#[test]
fn a_spent_shots_blast_meets_an_npc_where_its_drift_that_pass_left_it() {
    // `_HighLevelAIHandler`'s goal 0xe drifts the ship (@0x8eb6b-0x8ebd4)
    // inside `_AIDispatch` (@0x4429d), before `_HandleShot` (@0x44385),
    // whose blast reads the ship's record live (@0x3603c-0x3605b). The
    // player fires a flak shell straight up on pass 1, its third move
    // spending it 30 above the player on pass 3; on pass 4 its blast of
    // radius 40 lands, reaching a ship 40 off on either axis. The NPC,
    // 40.5 to the right of where the shell is spent, facing left and at
    // rest, is out of its reach unless it drifts 0.7 towards it in pass
    // 4's think, before the blast.
    let flak = WeaponRecord {
        count: 3,
        speed: 1000,
        blast_radius: 40,
        flags: 0x8000,
        ..super::tests::blaster()
    };
    let catalog = arming(flak, weapon(129), 0);
    for drifts in [false, true] {
        let mut session = with_an_npc_at(&catalog, 790, 720, 270);
        session.space.traffic.npcs_mut()[0].state.position = Vec2::new(40.5, -30.0);
        let behaviour = Drifting::default();
        let pass = |session: &mut Session| {
            work_with(
                session,
                &catalog,
                Controls::default(),
                &behaviour,
                Rules::default(),
                &mut NeverFires,
            );
        };
        session.hold_trigger(FIRE);
        pass(&mut session);
        session.hold_trigger(Trigger::default());
        pass(&mut session);
        pass(&mut session);
        let start = session.npcs()[0].state.position;
        assert_eq!(start, Vec2::new(40.5, -30.0), "{drifts}");
        behaviour.drifting.set(drifts);
        session.take_combat_events();
        pass(&mut session);
        let blasted = session.take_combat_events().iter().any(|event| {
            matches!(
                event,
                CombatEvent::Hit { ship, at, .. }
                    if *ship == ShipRef::Npc(NpcId(0)) && *at == Vec2::new(0.0, -30.0)
            )
        });
        let npc = &session.npcs()[0];
        let drifted = if drifts { 0.7 } else { 0.0 };
        assert!(
            (npc.state.position - Vec2::new(40.5 - drifted, -30.0)).length() < 1e-3,
            "{drifts}: {:?}",
            npc.state.position
        );
        assert_eq!(blasted, drifts, "{drifts}: blasted where it drifted to");
    }
}
