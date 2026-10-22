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
//! - **The escort's fallback** (`_EscortAI` @0x83b88-0x83bda, for every
//!   parent, [`escort_word`]): an escort or fighter on Defend or Attack
//!   (word 1 or 2, @0x83b8f-0x83b94) whose hull has `Flags2` 0x0080
//!   (@0x83ba8) and anything but [`AmmoLeft::Armed`] (@0x83bb5) writes
//!   its order word: a fighter (AI type 5, @0x83bbf) Return (3,
//!   @0x83bc9), so it docks; any other ship formation (0, @0x83bda), but
//!   only [`AmmoLeft::Dry`], so one with unlimited weapons left keeps
//!   fighting. It runs after the count wait, the assist and the lost
//!   parent, and before the jump's return (@0x83be3), so it writes the
//!   word on the frame a ship takes its leader's jump too. The order
//!   block that follows flies the word. No sound, no draw.

use crate::ai::Goal;
use crate::ai::fire::{AmmoLeft, ammo_left};
use crate::escort::EscortOrder;
use crate::fleet::OrderWord;
#[cfg(doc)]
use crate::govt::PLUNDERS;
use crate::traffic::npc::Npc;

/// Whether `npc` retreats out of ammunition (see the module docs): its
/// hull's `Flags2` 0x0080, and anything but [`AmmoLeft::Armed`].
#[must_use]
pub fn retreats(npc: &Npc) -> bool {
    npc.hull.retreats_dry && ammo_left(npc) != AmmoLeft::Armed
}

/// The order word `_EscortAI`'s out-of-ammunition fallback writes for
/// `npc`, an escort or fighter following `order` (@0x83b88-0x83bda, see
/// the module docs); none when the fallback doesn't run. It runs on
/// Defend or Attack alone, while `npc` [`retreats`]: a fighter (one with
/// a [`carrier`](Npc::carrier), the original's AI type 5, @0x83bbf) takes
/// Return (3, @0x83bc9) short of any ammunition, and any other ship takes
/// formation (0, @0x83bda) only [`AmmoLeft::Dry`], keeping its order with
/// unlimited weapons left. It plays no sound and draws nothing.
#[must_use]
pub fn escort_word(npc: &Npc, order: Option<EscortOrder>) -> Option<OrderWord> {
    if !matches!(order, Some(EscortOrder::Defend | EscortOrder::Attack)) || !retreats(npc) {
        return None;
    }
    if npc.carrier.is_some() {
        Some(OrderWord(3))
    } else {
        (ammo_left(npc) == AmmoLeft::Dry).then_some(OrderWord(0))
    }
}

/// The order `npc`, an escort or fighter following `order`, flies after
/// `_EscortAI`'s out-of-ammunition fallback: the order of the word
/// [`escort_word`] writes, else `order`.
#[must_use]
pub fn escort_order(npc: &Npc, order: Option<EscortOrder>) -> Option<EscortOrder> {
    escort_word(npc, order).map_or(order, OrderWord::order)
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
    use crate::escort::EscortOrder;
    use crate::fleet::OrderWord;
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

    /// A ship of `left` ammunition, with `Flags2` 0x0080 as `retreats`
    /// says, a fighter (AI type 5) as `fighter` says.
    fn escort(left: AmmoLeft, retreats: bool, fighter: bool) -> Npc {
        let mut npc = police(left, retreats);
        if fighter {
            npc.carrier = Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            });
        }
        npc
    }

    const FIGHTING: [Option<EscortOrder>; 2] =
        [Some(EscortOrder::Defend), Some(EscortOrder::Attack)];

    #[test]
    fn a_fighter_short_of_any_ammunition_on_defend_or_attack_returns() {
        // AI type 5 takes word 3 for a result of 1 or 2 (@0x83bbf-0x83bc9).
        for order in FIGHTING {
            for left in [AmmoLeft::OnlyUnlimited, AmmoLeft::Dry] {
                let fighter = escort(left, true, true);
                assert_eq!(
                    escort_word(&fighter, order),
                    Some(OrderWord(3)),
                    "{order:?} {left:?}"
                );
                assert_eq!(
                    escort_order(&fighter, order),
                    Some(EscortOrder::Dock),
                    "{order:?} {left:?}"
                );
            }
        }
    }

    #[test]
    fn an_escort_with_no_firepower_forms_and_one_with_unlimited_weapons_keeps_its_order() {
        // Any other type takes word 0 for 2 alone (@0x83bda).
        for order in FIGHTING {
            let dry = escort(AmmoLeft::Dry, true, false);
            assert_eq!(escort_word(&dry, order), Some(OrderWord(0)), "{order:?}");
            assert_eq!(escort_order(&dry, order), None, "{order:?}");
            let short = escort(AmmoLeft::OnlyUnlimited, true, false);
            assert_eq!(escort_word(&short, order), None, "{order:?}");
            assert_eq!(escort_order(&short, order), order, "{order:?}");
        }
    }

    #[test]
    fn the_fallback_needs_flags2_0x0080_and_defend_or_attack() {
        // The word 1 or 2 (@0x83b8f-0x83b94), `Flags2` 0x0080 (@0x83ba8)
        // and `_AIShipIsOutOfAmmo` not 0 (@0x83bb5).
        for fighter in [false, true] {
            for order in FIGHTING {
                let armed = escort(AmmoLeft::Armed, true, fighter);
                assert_eq!(escort_word(&armed, order), None, "{fighter} {order:?}");
                assert_eq!(escort_order(&armed, order), order, "{fighter} {order:?}");
                let unflagged = escort(AmmoLeft::Dry, false, fighter);
                assert_eq!(escort_word(&unflagged, order), None, "{fighter} {order:?}");
            }
            for order in [None, Some(EscortOrder::Hold), Some(EscortOrder::Dock)] {
                let dry = escort(AmmoLeft::Dry, true, fighter);
                assert_eq!(escort_word(&dry, order), None, "{fighter} {order:?}");
                assert_eq!(escort_order(&dry, order), order, "{fighter} {order:?}");
            }
        }
    }
}
