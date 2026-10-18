//! The idle block of Nova's AI types (in the `EV Nova` executable): what a
//! ship with nothing to fight does, once it has decided to go about its
//! business. [`idle`](super::idle) runs it for the traders, the warship and
//! peaceful traffic; [`patrol`] is the interceptor's own, and
//! [`pirate_idle`] the pirate's copy.
//!
//! - **The idle block** (`_WimpyTraderAI` @0x8b06c-0x8b1a3,
//!   `_BraveTraderAI` @0x8b4d6-0x8b60d, `_WarshipAI` @0x8b88f-0x8b9bf), run
//!   only in state 0 (`testw cx` @0x8b06c; `_WarshipAI` @0x8b835). It counts
//!   the system's nav slots that aren't -1 (system +0x2a, @0x8b075-0x8b09c).
//!   With none, or with one that is the stellar the ship has visited
//!   (+0x92, [`Npc::visited`], @0x8b0a3-0x8b0c8), it has visited
//!   ([`visited_here`]) and skips the pick. Otherwise it picks with
//!   `_SelectRandomStellarDest(ship, 0, 0)` (@0x8b0d1-0x8b0ea), heading for
//!   the stellar picked (goal 1, @0x8b0ef-0x8b106). With no pick, or once
//!   visited, the inlined `_AIShipHasFuelForJump` (@0x8b10b-0x8b170,
//!   [`leave::can_jump`]) sends it out by hyperspace (goal 2,
//!   @0x8b179-0x8b19b) or makes it stop and wait (state 6, [`Goal::Wait`],
//!   @0x8b1a3): [`leave_or_wait`].
//! - **State 6 is sticky** for the traders and the interceptor: their idle
//!   blocks need state 0, and the interceptor's patrol, inspection and
//!   search need a goal of 1 or below, or 0x14 (`_InterceptorAI`
//!   @0x8cc6d-0x8cc7d). Only provocation (traders @0x8b1ac, the interceptor
//!   @0x8cbf6-0x8cc28) or a hunter's government's word lifts it. The
//!   warship's state 6 is its own ([`wait`](super::wait)).
//! - **The interceptor's patrol** (`_InterceptorAI` @0x8d00d-0x8d136, state
//!   0 only, @0x8d010): it writes +0x92 = -2 (@0x8d01e), then picks with
//!   `_SelectRandomStellarDest(ship, 0, 1)` (@0x8d027-0x8d03a), plain
//!   stellars only ([`Admit::PlainOnly`]). With none it leaves or waits
//!   (@0x8d04b-0x8d0fb); a pick that is destroyed (`_StellarIsDestroyed`
//!   @0x8d118) makes it wait (@0x8d124); any other it heads for
//!   (@0x8d12f-0x8d136). It has no visited test, and the -2 has no reader
//!   for an interceptor, so the patrol ignores [`Npc::visited`].
//! - **The pirate's copy** (`_PirateWarshipAI` @0x8c410-0x8c5d2) runs
//!   whenever its search finds nothing (@0x8c385), in any goal but 0xe
//!   (@0x8c339-0x8c357). Once visited it leaves or waits (@0x8c52d,
//!   @0x8c5c9). Not visited, it picks only while its travel stellar (+0x6c)
//!   is -1 (@0x8c472-0x8c477), heading there (@0x8c49d-0x8c4a3); a failed
//!   pick, or a travel stellar already set, leaves or waits (@0x8c4b1,
//!   @0x8c51f). So a pirate with nothing to fight never lands: on the think
//!   after its pick it leaves or stops.
//! - **+0x92's writers**: every spawn starts it at -2 (@0x1c3d6, @0x3d4dc,
//!   @0x3d7ef, @0x41ed4, @0x423ed, @0x42565, @0x42a85, @0x43108, @0x436bd,
//!   @0x43a38, @0x7f3fa), [`Npc::visited`]'s `None`. A ship with a fuel tank
//!   that comes out of a gate has visited it
//!   (`_AIMakeShipEmergeFromHyperGate` @0x8955e-0x89574,
//!   [`Npc::gate_visit`]), so on its first idle think it leaves by
//!   hyperspace, or stops if it can't jump. The landing's end
//!   (`_HighLevelAIHandler` @0x8db00) is
//!   rdm:phase/stellar-fidelity/phase-5-npc-landing-reach's: until it lands,
//!   a landing ship leaves the system. `_LoadPilotData` @0x751ed writes the
//!   player's, and `_LoadObjectData` @0x77c42 a system's field at the same
//!   offset: neither is an NPC's.
//!
//! None of these blocks plays a sound. +0x92 and +0x6c are 16-bit there;
//! not replicated.
//!
//! **Left out:**
//!
//! - The sub-goal +0x6a = 2 a pick writes (@0x8b0f7 and the other copies),
//!   and its absence from the interceptor's patrol: the engine flies a
//!   landing without that sub-goal
//!   (rdm:phase/stellar-fidelity/phase-5-npc-landing-reach Part A owns the
//!   approach).
//! - The leave's +0x50 clamp and +0xa4 stamp (@0x8b179-0x8b19b and the
//!   copies): rdm:phase/ship-handling-fidelity/phase-8-warp-out's.
//! - The warship's second copy (@0x8b9c8-0x8bb03, while its target is still
//!   -1): the same decision with more draws, so not modelled.
//! - The interceptor's +0x92 = -2 (@0x8d01e): no reader for AI type 4.
//! - The emerge's gate-index test (@0x8952d): every stellar passes it.
//! - Owned by rdm:phase/combat-ai-fidelity/phase-20-left-out-of-phase-12:
//!   the pirate's travel stellar kept through goal 0 (the engine's
//!   [`Goal::Idle`] carries none, so it picks first and leaves a think
//!   later), and a waiting pirate whose pick failed picking again (the
//!   engine's [`Goal::Wait`] reads as bound and never re-picks); the
//!   miner's +0x92 (@0x8b373, @0x8b3ae); and state 6 written outside the
//!   idle block.

use crate::ai::{Admit, Goal, Surroundings, leave, random_stellar};
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::traffic::npc::Npc;

/// Whether `npc` has visited a stellar of the system among `around`, so the
/// idle block skips its pick: the system has no nav stellar at all, or one
/// of them is [`Npc::visited`] (@0x8b0a3-0x8b0c8).
#[must_use]
pub fn visited_here(npc: &Npc, around: &Surroundings) -> bool {
    around.sites.is_empty()
        || npc
            .visited
            .is_some_and(|visited| around.sites.iter().any(|site| site.id == visited))
}

/// The idle block's fuel test: [`Goal::JumpOut`] when `npc` can jump
/// ([`leave::can_jump`]), else [`Goal::Wait`] (state 6, @0x8b1a3).
#[must_use]
pub fn leave_or_wait(npc: &Npc, around: &Surroundings) -> Goal {
    if leave::can_jump(npc, around.jump_fuel) {
        Goal::JumpOut
    } else {
        Goal::Wait
    }
}

/// The interceptor's patrol (see the module docs), admitting uninhabited
/// stellars as `uninhabited` says
/// ([`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)),
/// rolling its pick on `chance`. An escort follows its lead, and a goal it
/// can still fly is kept, as [`idle`](super::idle) has them.
pub fn patrol(
    npc: &Npc,
    around: &Surroundings,
    chance: &mut dyn Chance,
    uninhabited: bool,
) -> Goal {
    if let Some(kept) = super::kept(npc, around) {
        return kept;
    }
    match random_stellar(npc, around, chance, Admit::PlainOnly { uninhabited }) {
        None => leave_or_wait(npc, around),
        Some(Goal::Land(stellar)) if destroyed(stellar, around) => Goal::Wait,
        Some(goal) => goal,
    }
}

/// The pirate's copy of the idle block (see the module docs), rolling its
/// pick on `chance`. Its think has turned a hold into state 0 already. An
/// escort follows its lead.
pub fn pirate_idle(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    if let Some(lead) = npc.leader
        && around.npc(lead).is_some()
    {
        return Goal::Follow(lead);
    }
    if visited_here(npc, around) || bound(npc.goal) {
        return leave_or_wait(npc, around);
    }
    random_stellar(npc, around, chance, Admit::All).unwrap_or_else(|| leave_or_wait(npc, around))
}

/// Whether a pirate in `goal` has its travel stellar (+0x6c) set, so its
/// idle block picks none (@0x8c472-0x8c477): bound for a stellar or a gate,
/// leaving, or waiting. The engine's leave and wait carry no stellar, so
/// they read as set.
fn bound(goal: Goal) -> bool {
    matches!(
        goal,
        Goal::Land(_) | Goal::EnterGate(_) | Goal::JumpOut | Goal::Wait
    )
}

/// Whether `stellar` among `around` is destroyed (`_StellarIsDestroyed`).
fn destroyed(stellar: StellarId, around: &Surroundings) -> bool {
    around
        .sites
        .iter()
        .any(|site| site.id == stellar && site.destroyed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::idle;
    use crate::catalog::LandingSite;
    use crate::gate::HYPERGATE;
    use crate::landing::StellarFlags;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::planet;

    fn ship() -> Npc {
        crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]))
    }

    /// `ship` having visited `stellar`.
    fn visited(stellar: i16) -> Npc {
        Npc {
            visited: Some(StellarId(stellar)),
            ..ship()
        }
    }

    /// `npc` unable to jump: its class's `Fuel` 99 (@0x8b10b).
    fn stranded(npc: Npc) -> Npc {
        Npc {
            class_fuel: 99,
            ..npc
        }
    }

    fn sites() -> Vec<LandingSite> {
        vec![planet(128, 0.0, 0.0), planet(129, 0.0, 0.0)]
    }

    #[test]
    fn a_ship_that_has_landed_and_taken_off_leaves() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        let ship = visited(128);
        assert!(visited_here(&ship, &around));
        let mut chance = Draws::of(&[]);
        assert_eq!(idle(&ship, &around, &mut chance), Goal::JumpOut);
        assert!(chance.asked.is_empty(), "no pick");
    }

    #[test]
    fn a_ship_that_has_landed_and_taken_off_stops_when_it_cannot_jump() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            idle(&stranded(visited(129)), &around, &mut chance),
            Goal::Wait
        );
        assert!(chance.asked.is_empty(), "no pick");
    }

    #[test]
    fn a_visit_to_another_systems_stellar_counts_for_nothing() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        let elsewhere = visited(300);
        assert!(!visited_here(&elsewhere, &around));
        assert!(!visited_here(&ship(), &around), "never visited");
        let mut chance = Draws::of(&[1]);
        assert_eq!(
            idle(&elsewhere, &around, &mut chance),
            Goal::Land(StellarId(129))
        );
        assert_eq!(chance.asked, [2]);
    }

    #[test]
    fn a_system_with_no_stellars_sends_it_to_the_fuel_test() {
        let around = Surroundings::new(&[], &[]);
        assert!(visited_here(&ship(), &around), "no nav slot at all");
        let mut chance = Draws::of(&[]);
        assert_eq!(idle(&ship(), &around, &mut chance), Goal::JumpOut);
        assert_eq!(idle(&stranded(ship()), &around, &mut chance), Goal::Wait);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn with_nothing_to_pick_it_leaves_or_waits() {
        let unlandable = [LandingSite {
            flags: 0,
            ..planet(128, 0.0, 0.0)
        }];
        let around = Surroundings::new(&unlandable, &[]);
        assert_eq!(leave_or_wait(&ship(), &around), Goal::JumpOut);
        assert_eq!(leave_or_wait(&stranded(ship()), &around), Goal::Wait);
        assert_eq!(
            idle(&stranded(ship()), &around, &mut Draws::of(&[])),
            Goal::Wait
        );
    }

    #[test]
    fn a_patrol_ignores_the_visit_and_never_picks_a_gate() {
        let gate = LandingSite {
            flags2: HYPERGATE,
            ..planet(130, 0.0, 0.0)
        };
        let sites = [planet(128, 0.0, 0.0), gate];
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            patrol(&visited(128), &around, &mut chance, true),
            Goal::Land(StellarId(128))
        );
        assert_eq!(chance.asked, [1], "the plain stellar alone");
    }

    #[test]
    fn a_patrol_admits_an_uninhabited_stellar_as_its_reading_says() {
        let empty = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..planet(129, 0.0, 0.0)
        };
        let sites = [empty, planet(128, 0.0, 0.0)];
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            patrol(&ship(), &around, &mut chance, true),
            Goal::Land(StellarId(129))
        );
        assert_eq!(chance.asked, [2], "both");
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            patrol(&ship(), &around, &mut chance, false),
            Goal::Land(StellarId(128))
        );
        assert_eq!(chance.asked, [1], "the inhabited one alone");
    }

    #[test]
    fn a_destroyed_patrol_pick_makes_it_wait() {
        let ruin = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
            destroyed: true,
            ..planet(128, 0.0, 0.0)
        };
        let sites = [ruin];
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(patrol(&ship(), &around, &mut chance, false), Goal::Wait);
        assert_eq!(chance.asked, [1], "picked, then found destroyed");
    }

    #[test]
    fn no_patrol_pick_leaves_or_waits() {
        let around = Surroundings::new(&[], &[]);
        let mut chance = Draws::of(&[]);
        assert_eq!(patrol(&ship(), &around, &mut chance, true), Goal::JumpOut);
        assert_eq!(
            patrol(&stranded(ship()), &around, &mut chance, true),
            Goal::Wait
        );
    }

    #[test]
    fn a_patrolling_escort_follows_its_lead_and_a_kept_goal_is_kept() {
        let sites = sites();
        let lead = Npc {
            id: NpcId(2),
            ..ship()
        };
        let escort = Npc {
            leader: Some(NpcId(2)),
            ..ship()
        };
        let npcs = [lead];
        let around = Surroundings::new(&sites, &npcs);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            patrol(&escort, &around, &mut chance, true),
            Goal::Follow(NpcId(2))
        );
        for goal in [Goal::Land(StellarId(129)), Goal::Wait, Goal::Hold] {
            let kept = Npc { goal, ..ship() };
            assert_eq!(patrol(&kept, &around, &mut chance, true), goal);
        }
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_pirate_picks_only_while_it_has_no_travel_stellar() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[1]);
        assert_eq!(
            pirate_idle(&ship(), &around, &mut chance),
            Goal::Land(StellarId(129))
        );
        assert_eq!(chance.asked, [2]);
        for goal in [
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::JumpOut,
            Goal::Wait,
        ] {
            let mut chance = Draws::of(&[]);
            let bound = Npc { goal, ..ship() };
            assert_eq!(
                pirate_idle(&bound, &around, &mut chance),
                Goal::JumpOut,
                "{goal:?}"
            );
            assert_eq!(
                pirate_idle(&stranded(bound), &around, &mut chance),
                Goal::Wait,
                "{goal:?}"
            );
            assert!(chance.asked.is_empty(), "{goal:?}");
        }
    }

    #[test]
    fn a_visited_pirate_leaves_or_waits_and_one_with_no_pick_too() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[]);
        assert_eq!(
            pirate_idle(&visited(128), &around, &mut chance),
            Goal::JumpOut
        );
        assert_eq!(
            pirate_idle(&stranded(visited(128)), &around, &mut chance),
            Goal::Wait
        );
        let none = Surroundings::new(&[], &[]);
        assert_eq!(
            pirate_idle(&stranded(ship()), &none, &mut chance),
            Goal::Wait
        );
        let unlandable = [LandingSite {
            flags: 0,
            ..planet(128, 0.0, 0.0)
        }];
        let around = Surroundings::new(&unlandable, &[]);
        assert_eq!(pirate_idle(&ship(), &around, &mut chance), Goal::JumpOut);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_pirate_escort_follows_its_lead() {
        let lead = Npc {
            id: NpcId(2),
            ..ship()
        };
        let escort = Npc {
            leader: Some(NpcId(2)),
            goal: Goal::Wait,
            ..ship()
        };
        let npcs = [lead];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            pirate_idle(&escort, &around, &mut Draws::of(&[])),
            Goal::Follow(NpcId(2))
        );
        let alone = Surroundings::new(&[], &[]);
        assert_eq!(
            pirate_idle(&escort, &alone, &mut Draws::of(&[])),
            Goal::JumpOut,
            "its lead gone"
        );
    }
}
