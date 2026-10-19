//! The warship's state 6 ([`Goal::Wait`]; `_WarshipAI` @0x8bbab-0x8bc37
//! in the `EV Nova` executable): a stopped warship waits for its fighters.
//!
//! Each think in state 6 it drops its target (-1) and looks for one
//! (`_SelectWarshipTarget` @0x8bbc2, [`hunt`](super::warship::hunt)): a
//! target found makes it attack (goal 4, via @0x8be09-0x8be2d). With none
//! it counts slots 1-63 (@0x8bbd1-0x8bc2e) for a live ship whose leader
//! (+0x9a) is this one (+0x86), of AI type 5 (+0x88), with no defended
//! stellar (+0x8c of -1) and not `_IsDisabled`: its own fighters out
//! ([`fighters_out`]). While any is out it stays in state 6; otherwise it
//! takes state 0 (@0x8bc37), and its idle block runs on its next think
//! ([`warship_waits`]).
//!
//! The engine's fighters of AI type 5 led by this ship are those it
//! launched ([`Npc::carrier`]). It has no stellar defenders
//! (rdm:task/planet-domination), so +0x8c is always -1. No sound plays.

use crate::ai::{Goal, Surroundings};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::traffic::npc::Npc;

/// Whether any of `npc`'s own fighters is out among `around`: another NPC
/// it launched ([`Npc::carrier`]) that is intact, as `_IsDisabled` lets
/// pass neither a disabled ship nor one breaking up (@0x8bbd1-0x8bc2e).
#[must_use]
pub fn fighters_out(npc: &Npc, around: &Surroundings) -> bool {
    let me = ShipRef::Npc(npc.id);
    around.npcs.iter().any(|other| {
        other.id != npc.id
            && other.carrier.map(|carrier| carrier.ship) == Some(me)
            && other.condition == Condition::Intact
    })
}

/// What a waiting warship with no target does among `around`: it waits
/// ([`Goal::Wait`]) while its fighters are out ([`fighters_out`]), and
/// otherwise takes state 0 ([`Goal::Idle`], @0x8bc37).
#[must_use]
pub fn warship_waits(npc: &Npc, around: &Surroundings) -> Goal {
    if fighters_out(npc, around) {
        Goal::Wait
    } else {
        Goal::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;

    fn warship() -> Npc {
        Npc {
            goal: Goal::Wait,
            ..crate::traffic::fixture::npc(1, ShipStats::default())
        }
    }

    /// A fighter (NPC `id`) launched by NPC `carrier`, in `condition`.
    fn fighter(id: u32, carrier: u32, condition: Condition) -> Npc {
        Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Npc(NpcId(carrier)),
                window: 100.0,
                reach: 30.0,
            }),
            leader: Some(NpcId(carrier)),
            condition,
            ..crate::traffic::fixture::npc(id, ShipStats::default())
        }
    }

    #[test]
    fn a_waiting_warship_waits_while_its_fighters_are_out() {
        let npcs = [warship(), fighter(2, 1, Condition::Intact)];
        let around = Surroundings::new(&[], &npcs);
        assert!(fighters_out(&npcs[0], &around));
        assert_eq!(warship_waits(&npcs[0], &around), Goal::Wait);
    }

    #[test]
    fn a_waiting_warship_whose_fighters_are_disabled_breaking_up_anothers_or_none_goes_idle() {
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        for others in [
            vec![fighter(2, 1, Condition::Disabled)],
            vec![fighter(2, 1, dying)],
            vec![fighter(2, 3, Condition::Intact)],
            vec![crate::traffic::fixture::npc(2, ShipStats::default())],
            Vec::new(),
        ] {
            let npcs: Vec<Npc> = std::iter::once(warship()).chain(others).collect();
            let around = Surroundings::new(&[], &npcs);
            assert!(!fighters_out(&npcs[0], &around), "{:?}", npcs.get(1));
            assert_eq!(warship_waits(&npcs[0], &around), Goal::Idle);
        }
    }

    #[test]
    fn a_ship_is_not_its_own_fighter() {
        let mut odd = fighter(1, 1, Condition::Intact);
        odd.goal = Goal::Wait;
        let npcs = [odd];
        let around = Surroundings::new(&[], &npcs);
        assert!(!fighters_out(&npcs[0], &around));
    }
}
