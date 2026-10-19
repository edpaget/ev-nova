//! What a warship or interceptor does once its weapons run out of
//! ammunition ([`ammo_left`]): the out-of-ammo retreat (`_WarshipAI`
//! @0x8bf3e-0x8bf83, `_InterceptorAI` @0x8d343-0x8d391) and the warship's
//! fully-dry drop (`_WarshipAI` @0x8c0e7-0x8c117). Neither plays a sound.
//!
//! - **The retreat**: a ship whose hull has `shïp` `Flags2` 0x0080
//!   ([`HullSpec::retreats_dry`](crate::combat::hull::HullSpec::retreats_dry))
//!   and anything but [`AmmoLeft::Armed`] goes to state 3, its target
//!   kept, unless its state is 2, 3 or 11 (leaving, retreating already,
//!   or following its leader out). State 3 with a target is
//!   [`Goal::Flee`]; with none, `_HighLevelAIHandler` makes it state 0,
//!   sub 0, provocation 0 at once, in the same dispatch (@0x8dff6 →
//!   @0x8e228-0x8e23d): [`Goal::Idle`], its provocation forgotten
//!   ([`stops`]).
//!   - A warship meets it only with a target, attacking: target -1 and
//!     states 7 and 3 jump past it (@0x8bc48, @0x8bcb4, @0x8be03,
//!     @0x8be14, @0x8be1e), and any other state with a target is made 4
//!     first (@0x8be24-0x8be2d). So it flees the ship it attacks or snipes
//!     at, after the odds and shield retreats ([`hunt`](super::warship::hunt)),
//!     and a warship with no target goes about its business.
//!   - An interceptor meets it with or without a target, after its
//!     patrol and inspection (@0x8cc6d-0x8d136) and its odds retreat: it
//!     flees the ship it attacks, snipes at or inspects (an inspection
//!     holds its ship as the target, @0x8ce34-0x8ce42), and with none,
//!     patrolling, holding or idle, it stops ([`after_patrol`]); leaving
//!     by hyperspace (state 2) it leaves.
//!   - A leaderless warship of a government with `Flags` 0x1000
//!     ([`PLUNDERS`]) thinks as `_PirateWarshipAI` (`_AIDispatch`
//!     @0x8ffc8-0x8fff0, [`pirate`](super::pirate)), which has no `Flags2`
//!     0x0080 test: it never retreats so.
//! - **The fully-dry drop**: a warship attacking (state 4) with
//!   [`AmmoLeft::Dry`] goes to state 0, sub 0, target and travel stellar
//!   -1: [`Goal::Idle`] ([`drop_when_dry`]). With `Flags2` 0x0080 the
//!   retreat has already made state 4 into 3, so the drop reaches only a
//!   ship without it. `_PirateWarshipAI` drops the same way (@0x8c848-0x8c887),
//!   its plunder (goal 0xd) too ([`pirate`](super::pirate)); the
//!   interceptor has no drop.

use crate::ai::Goal;
use crate::ai::fire::{AmmoLeft, ammo_left};
#[cfg(doc)]
use crate::govt::PLUNDERS;
use crate::traffic::npc::Npc;

/// Whether `npc` retreats out of ammunition (see the module docs): its
/// hull's `Flags2` 0x0080, and anything but [`AmmoLeft::Armed`].
#[must_use]
pub fn retreats(npc: &Npc) -> bool {
    npc.hull.retreats_dry && ammo_left(npc) != AmmoLeft::Armed
}

/// The goal of a warship `npc` that has decided on `goal`, after its
/// fully-dry drop (see the module docs): an attack or a snipe of a ship
/// with [`AmmoLeft::Dry`] is [`Goal::Idle`]; any other goal is kept.
#[must_use]
pub fn drop_when_dry(npc: &Npc, goal: Goal) -> Goal {
    if goal.attacking().is_some() && ammo_left(npc) == AmmoLeft::Dry {
        Goal::Idle
    } else {
        goal
    }
}

/// The goal of an interceptor `npc` that, with nothing to fight, has
/// decided on `goal` by its patrol, inspection or hold, after its
/// out-of-ammo retreat (see the module docs): retreating, it flees the
/// ship it inspects, and stops ([`Goal::Idle`]) otherwise, but leaving by
/// hyperspace ([`Goal::JumpOut`], state 2) it leaves.
#[must_use]
pub fn after_patrol(npc: &Npc, goal: Goal) -> Goal {
    if !retreats(npc) {
        return goal;
    }
    match goal {
        Goal::Inspect(ship) => Goal::Flee(ship),
        Goal::JumpOut => goal,
        _ => Goal::Idle,
    }
}

/// Whether an interceptor `npc` that, with nothing to fight, has decided
/// on `goal` stops as it retreats out of ammunition ([`after_patrol`]):
/// state 3 with no target, which `_HighLevelAIHandler` makes state 0 at
/// once, its provocation zeroed (@0x8dff6 → @0x8e228-0x8e23d).
#[must_use]
pub fn stops(npc: &Npc, goal: Goal) -> bool {
    retreats(npc) && !matches!(goal, Goal::Inspect(_) | Goal::JumpOut)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{POLICE, n, ship, short_of_ammo};
    use crate::combat::ShipRef;
    use crate::traffic::npc::AiType;

    fn police(left: AmmoLeft, retreats: bool) -> Npc {
        short_of_ammo(
            ship(1, POLICE, AiType::Interceptor, 0.0, 0.0),
            left,
            retreats,
        )
    }

    #[test]
    fn only_a_hull_of_flags2_0x0080_short_of_ammunition_retreats() {
        assert!(retreats(&police(AmmoLeft::Dry, true)));
        assert!(retreats(&police(AmmoLeft::OnlyUnlimited, true)));
        assert!(!retreats(&police(AmmoLeft::Armed, true)));
        assert!(!retreats(&police(AmmoLeft::Dry, false)));
    }

    #[test]
    fn only_an_attack_or_snipe_fully_dry_is_dropped() {
        let dry = police(AmmoLeft::Dry, false);
        let p = ShipRef::Player;
        assert_eq!(drop_when_dry(&dry, Goal::Attack(p)), Goal::Idle);
        assert_eq!(drop_when_dry(&dry, Goal::Snipe(n(2))), Goal::Idle);
        assert_eq!(drop_when_dry(&dry, Goal::Flee(p)), Goal::Flee(p));
        assert_eq!(drop_when_dry(&dry, Goal::JumpOut), Goal::JumpOut);
        let short = police(AmmoLeft::OnlyUnlimited, false);
        assert_eq!(drop_when_dry(&short, Goal::Attack(p)), Goal::Attack(p));
    }

    #[test]
    fn after_its_patrol_a_retreating_interceptor_flees_what_it_inspects_or_stops() {
        let dry = police(AmmoLeft::Dry, true);
        let p = ShipRef::Player;
        assert_eq!(after_patrol(&dry, Goal::Inspect(p)), Goal::Flee(p));
        assert_eq!(after_patrol(&dry, Goal::JumpOut), Goal::JumpOut);
        for goal in [
            Goal::Idle,
            Goal::Hold,
            Goal::Land(crate::catalog::StellarId(128)),
            Goal::EnterGate(crate::catalog::StellarId(129)),
        ] {
            assert_eq!(after_patrol(&dry, goal), Goal::Idle, "{goal:?}");
        }
        let armed = police(AmmoLeft::Armed, true);
        assert_eq!(after_patrol(&armed, Goal::Inspect(p)), Goal::Inspect(p));
    }

    #[test]
    fn only_a_retreat_with_no_ship_to_flee_and_not_leaving_stops() {
        // State 3 with a target flees it, and state 2 leaves; only state 3
        // with target -1 is made 0, its provocation zeroed (@0x8e228-0x8e23d).
        let p = ShipRef::Player;
        let dry = police(AmmoLeft::Dry, true);
        assert!(stops(&dry, Goal::Wait), "retreating, waiting");
        assert!(stops(&dry, Goal::Idle), "retreating, idle");
        assert!(
            !stops(&dry, Goal::Inspect(p)),
            "retreating, it flees what it inspects"
        );
        assert!(!stops(&dry, Goal::JumpOut), "retreating, it leaves");
        let armed = police(AmmoLeft::Armed, true);
        assert!(!stops(&armed, Goal::Wait), "not retreating, waiting");
        assert!(
            !stops(&armed, Goal::Inspect(p)),
            "not retreating, inspecting"
        );
        assert!(!stops(&armed, Goal::JumpOut), "not retreating, leaving");
    }
}
