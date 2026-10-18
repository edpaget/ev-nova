//! A fleeing ship giving up fleeing an ally it spares, as
//! `_HighLevelAIHandler` does after every type's think (state 3,
//! @0x8df75-0x8e0c3 → @0x8e81d in the `EV Nova` executable).
//!
//! A ship fleeing a ship within [`FLEEING_REACH`] on both axes, each
//! offset cut to a whole number (@0x8e025-0x8e050), stops fleeing it
//! (state 0, target -1: [`Goal::Idle`]) and makes no call for
//! reinforcements when it has a government, isn't led by the player, and
//! the ship it flees is allied with it and isn't led by the player either
//! (@0x8e077-0x8e0c3). Which allies those are is
//! [`RuleKey::LedAllyAttack`](crate::rulebook::RuleKey::LedAllyAttack)'s
//! ([`spares_ally`]): by the engine, the original's test, so a ship keeps
//! fleeing an allied ship the player leads; by the other reading, any
//! ally.
//!
//! The original's gates of the jump state +0x50 at or below 0 (@0x8df91)
//! and the substate not 4 (@0x8e059) have no engine counterpart: a ship
//! leaving by hyperspace has [`Goal::JumpOut`], not a flight
//! (rdm:phase/combat-ai-fidelity/phase-11-leave-by-hyperspace owns the
//! leaving branch). Within reach the original's substate becomes 5
//! (@0x8e06b), which is rdm:phase/combat-ai-fidelity/phase-7-attack-pursuit-substates's.

use crate::ai::target::spares_ally;
use crate::ai::{Decision, Goal, Surroundings};
use crate::combat::ShipRef;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// How near, in whole pixels on either axis, a ship fled from must be for
/// the fleeing ship's handler to read it (`cmpw $0xfa` @0x8e025,
/// @0x8e047): to call for reinforcements, or to give up fleeing an ally.
pub const FLEEING_REACH: f32 = 250.0;

/// Whether `from`, the ship `npc` flees, is within [`FLEEING_REACH`] of it
/// on both axes, each offset truncated, among `around`.
#[must_use]
pub fn within_reach(npc: &Npc, from: ShipRef, around: &Surroundings) -> bool {
    around.state_of(from).is_some_and(|state| {
        let off = state.position - npc.state.position;
        off.x.trunc().abs() <= FLEEING_REACH && off.y.trunc().abs() <= FLEEING_REACH
    })
}

/// Whether `npc` flees `from`, an ally it spares by the engine's reading,
/// among `around`: `_HighLevelAIHandler`'s test (@0x8e077-0x8e0c3), which
/// the player is never (its government is none).
#[must_use]
pub fn flees_a_stranger_ally(npc: &Npc, from: ShipRef, around: &Surroundings) -> bool {
    flees_a_spared_ally(npc, from, around, RuleSource::Engine)
}

/// Whether `npc` flees `from`, an ally it spares by `led_ally`'s reading
/// of [`RuleKey::LedAllyAttack`](crate::rulebook::RuleKey::LedAllyAttack),
/// among `around`.
fn flees_a_spared_ally(
    npc: &Npc,
    from: ShipRef,
    around: &Surroundings,
    led_ally: RuleSource,
) -> bool {
    let ShipRef::Npc(id) = from else {
        return false;
    };
    around
        .npc(id)
        .is_some_and(|quarry| spares_ally(npc, quarry, around, led_ally))
}

/// `decision` with `_HighLevelAIHandler`'s state-3 stop applied (see the
/// module docs): a flight from an ally within reach that `npc` spares by
/// `led_ally`'s reading becomes [`Goal::Idle`].
#[must_use]
pub fn with_flee_drop(
    npc: &Npc,
    decision: Decision,
    around: &Surroundings,
    led_ally: RuleSource,
) -> Decision {
    match decision.goal {
        Goal::Flee(from)
            if within_reach(npc, from, around)
                && flees_a_spared_ally(npc, from, around, led_ally) =>
        {
            Decision {
                goal: Goal::Idle,
                ..decision
            }
        }
        _ => decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{NEUTRAL, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::traffic::npc::AiType;

    /// The goal of a trader (NPC 1) at the centre that decided to flee
    /// `from` (NPC 2, `quarry`), after the stop by `led_ally`.
    fn after(npc: &Npc, quarry: &Npc, led_ally: RuleSource) -> Goal {
        let npcs = [npc.clone(), quarry.clone()];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
        let decision = Decision {
            goal: Goal::Flee(n(2)),
            ..Decision::default()
        };
        with_flee_drop(&npcs[0], decision, &around, led_ally).goal
    }

    fn trader() -> Npc {
        ship(1, TRADERS, AiType::Warship, 0.0, 0.0)
    }

    fn led(mut npc: Npc) -> Npc {
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        npc
    }

    #[test]
    fn a_ship_gives_up_fleeing_a_stranger_ally_within_250_under_both_readings() {
        let police = ship(2, POLICE, AiType::Warship, 250.9, -250.9);
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                after(&trader(), &police, led_ally),
                Goal::Idle,
                "{led_ally:?}"
            );
        }
        let far = ship(2, POLICE, AiType::Warship, 251.0, 0.0);
        assert_eq!(
            after(&trader(), &far, RuleSource::Engine),
            Goal::Flee(n(2)),
            "beyond"
        );
        let pirate = ship(2, NEUTRAL, AiType::Warship, 10.0, 0.0);
        assert_eq!(
            after(&trader(), &pirate, RuleSource::Engine),
            Goal::Flee(n(2)),
            "no ally"
        );
    }

    #[test]
    fn by_default_a_ship_keeps_fleeing_an_ally_the_player_leads_and_gives_up_on_it_otherwise() {
        let police = led(ship(2, POLICE, AiType::Warship, 10.0, 0.0));
        assert_eq!(
            after(&trader(), &police, RuleSource::Engine),
            Goal::Flee(n(2))
        );
        assert_eq!(after(&trader(), &police, RuleSource::Bible), Goal::Idle);
        let escort = led(trader());
        let police = ship(2, POLICE, AiType::Warship, 10.0, 0.0);
        assert_eq!(
            after(&escort, &police, RuleSource::Engine),
            Goal::Flee(n(2)),
            "a ship the player leads keeps fleeing"
        );
    }

    #[test]
    fn a_ship_of_no_government_gives_up_fleeing_no_ally_by_default() {
        let mut independent = trader();
        independent.govt = None;
        let mut other = ship(2, POLICE, AiType::Warship, 10.0, 0.0);
        other.govt = None;
        assert_eq!(
            after(&independent, &other, RuleSource::Engine),
            Goal::Flee(n(2))
        );
        assert_eq!(after(&independent, &other, RuleSource::Bible), Goal::Idle);
    }

    #[test]
    fn only_a_flight_is_given_up() {
        let npcs = [trader(), ship(2, POLICE, AiType::Warship, 10.0, 0.0)];
        let govts = govts(0, 0);
        let sites = sites();
        let far = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
        let attack = Decision {
            goal: Goal::Attack(n(2)),
            ..Decision::default()
        };
        assert_eq!(
            with_flee_drop(&npcs[0], attack, &far, RuleSource::Bible),
            attack
        );
        let from_player = Decision {
            goal: Goal::Flee(ShipRef::Player),
            ..Decision::default()
        };
        let near = around(&sites, &npcs, &govts, (0.0, -10.0), 0);
        assert_eq!(
            with_flee_drop(&npcs[0], from_player, &near, RuleSource::Bible),
            from_player
        );
    }
}
