//! NPC decisions: what each NPC sets out to do, on a timer kept apart from
//! the physics step (see [`traffic`](crate::traffic)).
//!
//! A [`Behaviour`] decides an NPC's [`Goal`] from what is around it, and
//! the fire command it holds and the ship it targets; [`Peaceful`] is
//! Nova's default, which never fires and targets nothing. The [`traffic`](crate::traffic)
//! autopilot then flies each goal, every tick.

use std::fmt::Debug;

use crate::catalog::{LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::hyperspace::JUMP_FUEL;
use crate::landing::landable;
use crate::traffic::npc::{Npc, NpcId};

/// What an NPC sets out to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Goal {
    /// Nothing: it comes to a stop where it is.
    #[default]
    Idle,
    /// Land on this stellar, and leave the system that way.
    Land(StellarId),
    /// Fly out past its jump distance and jump out.
    JumpOut,
    /// Keep with this NPC, its fleet's lead.
    Follow(NpcId),
}

/// What an NPC deciding can see.
#[derive(Clone, Copy, Debug)]
pub struct Surroundings<'a> {
    /// The system's stellars.
    pub sites: &'a [LandingSite],
    /// Every NPC in the system, the one deciding among them.
    pub npcs: &'a [Npc],
}

/// How NPCs decide what to do.
pub trait Behaviour: Debug {
    /// `npc`'s goal now, among `around`, rolling any choice on `chance`.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal;

    /// The fire command `npc` holds now, among `around`: none by default.
    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        let _ = (npc, around);
        Trigger::default()
    }

    /// The ship `npc` targets now, among `around`: none by default.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        let _ = (npc, around);
        None
    }
}

/// Nova's peaceful traffic, the default behaviour (see [`Peaceful::decide`]):
/// it never fires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Peaceful;

/// Nova's peaceful traffic (the idle pattern of `_WimpyTraderAI`,
/// `_BraveTraderAI`, `_WarshipAI` and `_InterceptorAI` in the `EV Nova`
/// executable, narrowed):
///
/// - An escort follows its lead while the lead is in the system; once it
///   is gone, the escort decides by its own AI type, as below.
/// - A goal it can still fly is kept: a stellar that is still there and
///   landable, or a jump it still has the fuel for.
/// - A trader (AI 1 or 2) lands on a landable stellar picked at random
///   (`_SelectRandomStellarDest`; the enemy filter waits for governments'
///   relations), or jumps out when there is none.
/// - A warship or interceptor (AI 3 or 4) jumps out.
/// - A ship that would jump without a jump's fuel stays idle.
impl Behaviour for Peaceful {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        if let Some(lead) = npc.leader
            && around.npcs.iter().any(|other| other.id == lead)
        {
            return Goal::Follow(lead);
        }
        if can_still_fly(npc, around) {
            return npc.goal;
        }
        let leave = if npc.reserves.fuel.now >= JUMP_FUEL {
            Goal::JumpOut
        } else {
            Goal::Idle
        };
        if !npc.ai_type.trades() {
            return leave;
        }
        let landable: Vec<_> = around.sites.iter().filter(|site| landable(site)).collect();
        if landable.is_empty() {
            return leave;
        }
        let len = u32::try_from(landable.len()).unwrap_or(u32::MAX);
        let pick = chance.below(len) as usize;
        landable.get(pick).map_or(leave, |site| Goal::Land(site.id))
    }
}

/// Whether `npc` can still fly its goal among `around`: a stellar still
/// there and landable, or a jump it has the fuel for. Idling, or
/// following once the lead is gone, is no goal to keep.
fn can_still_fly(npc: &Npc, around: &Surroundings) -> bool {
    match npc.goal {
        Goal::Land(stellar) => around
            .sites
            .iter()
            .any(|site| site.id == stellar && landable(site)),
        Goal::JumpOut => npc.reserves.fuel.now >= JUMP_FUEL,
        Goal::Idle | Goal::Follow(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::landing::StellarFlags;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, planet};
    use crate::traffic::npc::AiType;

    fn npc(id: u32, ai_type: AiType) -> Npc {
        Npc {
            ai_type,
            ..crate::testkit::npc(id, ShipStats::new(FAST, &[]))
        }
    }

    /// Planets 128 and 130 can be landed on; 129 cannot, and 131 only once
    /// destroyed.
    fn sites() -> Vec<LandingSite> {
        let mut sites: Vec<_> = (128..=131).map(|id| planet(id, 0.0, 0.0)).collect();
        sites[1].flags = 0;
        sites[3].flags |= StellarFlags::ONLY_WHEN_DESTROYED;
        sites
    }

    fn decide(npc: &Npc, sites: &[LandingSite], npcs: &[Npc], chance: &mut Draws) -> Goal {
        Peaceful.decide(npc, &Surroundings { sites, npcs }, chance)
    }

    const TRADERS: [AiType; 2] = [AiType::WimpyTrader, AiType::BraveTrader];
    const FIGHTERS: [AiType; 2] = [AiType::Warship, AiType::Interceptor];

    #[test]
    fn a_trader_lands_on_a_landable_stellar_picked_at_random() {
        for ai_type in TRADERS {
            for (draw, stellar) in [(0, 128), (1, 130)] {
                let mut chance = Draws::of(&[draw]);
                let goal = decide(&npc(1, ai_type), &sites(), &[], &mut chance);
                assert_eq!(goal, Goal::Land(StellarId(stellar)), "{ai_type:?}");
                assert_eq!(chance.asked, [2], "among the two landable");
            }
        }
    }

    #[test]
    fn a_trader_with_nowhere_to_land_jumps_out() {
        let unlandable = &sites()[1..2];
        for ai_type in TRADERS {
            let mut chance = Draws::of(&[]);
            assert_eq!(
                decide(&npc(1, ai_type), &[], &[], &mut chance),
                Goal::JumpOut
            );
            assert_eq!(
                decide(&npc(1, ai_type), unlandable, &[], &mut chance),
                Goal::JumpOut
            );
            assert!(chance.asked.is_empty());
        }
    }

    #[test]
    fn warships_and_interceptors_jump_out() {
        for ai_type in FIGHTERS {
            let mut chance = Draws::of(&[]);
            assert_eq!(
                decide(&npc(1, ai_type), &sites(), &[], &mut chance),
                Goal::JumpOut
            );
            assert!(chance.asked.is_empty());
        }
    }

    #[test]
    fn a_ship_without_a_jumps_fuel_that_would_jump_stays_idle() {
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            let mut short = npc(1, ai_type);
            short.reserves.fuel.now = 99.9;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::Idle);
            short.reserves.fuel.now = 100.0;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::JumpOut);
        }
        // A trader with somewhere to land needs no fuel.
        let mut dry = npc(1, AiType::BraveTrader);
        dry.reserves.fuel.now = 0.0;
        assert_eq!(
            decide(&dry, &sites(), &[], &mut Draws::of(&[0])),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn an_escort_follows_its_lead_and_once_it_is_gone_decides_by_its_own_ai_type() {
        let lead = npc(1, AiType::WimpyTrader);
        for (ai_type, alone) in [
            (AiType::Warship, Goal::JumpOut),
            (AiType::Interceptor, Goal::JumpOut),
            (AiType::BraveTrader, Goal::Land(StellarId(128))),
        ] {
            let mut escort = npc(2, ai_type);
            escort.leader = Some(NpcId(1));
            let together = [lead.clone(), escort.clone()];
            let mut chance = Draws::of(&[0]);
            assert_eq!(
                decide(&escort, &sites(), &together, &mut chance),
                Goal::Follow(NpcId(1))
            );
            assert!(chance.asked.is_empty());
            escort.goal = Goal::Follow(NpcId(1));
            assert_eq!(
                decide(&escort, &sites(), &[escort.clone()], &mut chance),
                alone,
                "{ai_type:?} without its lead"
            );
        }
    }

    #[test]
    fn peaceful_traffic_targets_nothing() {
        let npcs = [npc(1, AiType::Warship), npc(2, AiType::Interceptor)];
        let around = Surroundings {
            sites: &sites(),
            npcs: &npcs,
        };
        for npc in &npcs {
            assert_eq!(Peaceful.target(npc, &around), None);
        }
    }

    #[test]
    fn peaceful_traffic_never_fires() {
        let around = Surroundings {
            sites: &sites(),
            npcs: &[],
        };
        for ai_type in TRADERS.into_iter().chain(FIGHTERS) {
            assert_eq!(
                Peaceful.trigger(&npc(1, ai_type), &around),
                Trigger::default()
            );
        }
    }

    #[test]
    fn a_goal_it_can_still_fly_is_kept() {
        let mut lander = npc(1, AiType::WimpyTrader);
        lander.goal = Goal::Land(StellarId(130));
        let mut chance = Draws::of(&[]);
        assert_eq!(decide(&lander, &sites(), &[], &mut chance), lander.goal);
        assert!(chance.asked.is_empty(), "no new pick");
        let mut jumper = npc(1, AiType::Warship);
        jumper.goal = Goal::JumpOut;
        assert_eq!(decide(&jumper, &sites(), &[], &mut chance), Goal::JumpOut);
        // A trader that settled on jumping keeps to it.
        let mut leaving = npc(1, AiType::BraveTrader);
        leaving.goal = Goal::JumpOut;
        assert_eq!(decide(&leaving, &sites(), &[], &mut chance), Goal::JumpOut);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_goal_it_cannot_fly_is_decided_again() {
        let mut lander = npc(1, AiType::WimpyTrader);
        for gone in [129, 131, 140] {
            lander.goal = Goal::Land(StellarId(gone));
            let mut chance = Draws::of(&[1]);
            assert_eq!(
                decide(&lander, &sites(), &[], &mut chance),
                Goal::Land(StellarId(130)),
                "{gone}"
            );
        }
        let mut stranded = npc(1, AiType::Warship);
        stranded.goal = Goal::JumpOut;
        stranded.reserves.fuel.now = 50.0;
        assert_eq!(decide(&stranded, &[], &[], &mut Draws::of(&[])), Goal::Idle);
        let mut follower = npc(1, AiType::Warship);
        follower.goal = Goal::Follow(NpcId(7));
        assert_eq!(
            decide(&follower, &[], &[], &mut Draws::of(&[])),
            Goal::JumpOut
        );
        let mut idle = npc(1, AiType::Warship);
        idle.goal = Goal::Idle;
        assert_eq!(decide(&idle, &[], &[], &mut Draws::of(&[])), Goal::JumpOut);
    }
}
