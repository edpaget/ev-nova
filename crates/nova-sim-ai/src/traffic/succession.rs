//! A fleet whose lead is lost: `_AICheckParentExists` (@0x8926a-0x89501),
//! read whole ([`Traffic::succeed_leaders`]), and a ship leaving its fleet
//! ([`Npc::leave_fleet`]).
//!
//! **When.** `_DoPlayGameWork`'s marking pass (@0x4405e-0x441f7) runs it
//! once a frame, before the arrivals and every dispatch, for each ship
//! that is active, of AI type above 4 and led by an NPC (@0x440f4-0x44101),
//! then works out the formation parents (@0x4415c). The think half runs
//! it after the answers to the last tick's hits and before the formation
//! pass ([`fleet_formation`](super::fleet_formation)), so a successor's
//! followers take slots in its formation in the same tick. It runs by the
//! engine's reading of [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)
//! alone; by the other, the session's earlier lost-lead pass stands
//! (`nova_sim`'s `lost_lead` module).
//!
//! **The snapshot.** The pass first copies every ship's parent into
//! `_prevParent` (@0x44092-0x440ba), its only reader the check
//! (@0x892af): every follower of one lost lead finds the same successor
//! though a lower slot's check has rewritten its own parent. Here every
//! check reads the ships as the pass began.
//!
//! **The check**, for a ship whose leader is lost: gone from the system,
//! or there and not intact (disabled or breaking up, @0x89287-0x892a9).
//!
//! 1. **The successor** (@0x892af-0x8933d): among the ships there, intact,
//!    whose leader was the lost one, the greatest class `Mass` (+0x9ee,
//!    signed, [`HullSpec::signed_mass`](crate::combat::hull::HullSpec::signed_mass)),
//!    the lowest slot on a tie (`setg` @0x89319). The ship itself is a
//!    candidate.
//! 2. **None** (@0x894b1-0x894f3), a disabled ship with no intact fellow:
//!    it leaves its fleet ([`Npc::leave_fleet`]), +0x50 = -1.0, and pauses
//!    in goal 0x13, sub 0 ([`Kept::Coast`]), its target kept.
//! 3. **It leads** (@0x8934a-0x89474): it leaves its fleet, +0x50 = -1.0.
//!    Under the lost lead's own government (both not independent and
//!    equal, @0x8938a-0x893ae), it takes over the lead's goal and
//!    substate, target and travel stellar (@0x893b0-0x893e1), read from
//!    the lead's record, there or gone ([`gone`](super::gone)); goal 0xa
//!    with no leader is a hold ([`Goal::Hold`]). Otherwise its own goal
//!    maps (@0x893ea-0x8945f):
//!    - following its lead's jump (goal 0xb) → its own jump (goal 2, sub
//!      4);
//!    - a fighter's return or a formation (goals 5, 0xa, 0xc) → goal 0;
//!    - a target held in any goal but inspect (7) or refuel (9) → goal 4
//!      on it. A repair (0xf), whose target is the player it repairs
//!      (`_AIMakeShipRepairPlayer` @0x82e08), attacks it by the engine's
//!      reading of [`RuleKey::SuccessorRepairer`](crate::RuleKey::SuccessorRepairer)
//!      and idles by the other;
//!    - else goal 0.
//!
//!    Each writes sub 0 (pursuit and docking none) but the jump's sub 4,
//!    and clears the warp's sub-goal but in the jump.
//! 4. **Another leads** (@0x89479-0x894af, [`Npc::follow_successor`]): its
//!    parent is the successor; a fighter (AI type 5) becomes an escort (6,
//!    @0x89480-0x89491), so off its carrier with its class's capacities;
//!    and while its +0x50 is below 0.0 it pauses in goal 0x13, its
//!    substate kept ([`Kept::of`]). The engine's stage sits at 0 at rest,
//!    so it reads "not under way" for "below 0", and a ship on a jump (sub
//!    4 or 0xd) keeps it:
//!    rdm:phase/escort-fleet-fidelity/phase-8-jump-with-leader's.
//!
//! It writes no order word (+0xc90a): a follower flies the word its old
//! lead gave it until its new lead's next order frame. Nor a follower's
//! travel stellar (+0x6c, the lost lead's slot, kept), +0x8c or +0x96. A
//! lead that landed holds no record here, where the original keeps it in
//! its slot, so its successor takes the other government's branch
//! (rdm:phase/escort-fleet-fidelity-cross-roadmap/phase-1-landed-lead-keeps-its-fleet).
//!
//! **Ordering.** The original checks per ship in slot order. Every check
//! here reads the ships as the pass began, which the checks of one lost
//! lead's fleet don't write; in a chain of lost leads a lower slot's check
//! may change the lead a higher slot's copies, a difference within the
//! frame's first pass that no player sees. Nothing here plays a sound or
//! draws a random number.

use std::collections::BTreeSet;

use super::npc::{AiType, Npc, NpcId};
use super::warp_out::WarpGoal;
use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::pursuit::Pursuit;
use crate::ai::regroup::Kept;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;

/// The warp stage the check writes the ship that leads or is left alone
/// (-1.0).
pub const LEFT_STAGE: f32 = -1.0;

impl Traffic {
    /// Each ship whose lead is lost takes a new lead, leads, or is left
    /// alone, in `world` (see the module docs): by the engine's reading of
    /// its [`npc_escort_ai`](World::npc_escort_ai) alone.
    pub(crate) fn succeed_leaders(&mut self, world: &World) {
        if world.npc_escort_ai != RuleSource::Engine {
            return;
        }
        let lost: BTreeSet<NpcId> = self
            .npcs
            .iter()
            .filter(|npc| npc.condition.active())
            .filter_map(|npc| npc.leader)
            .filter(|&lead| !self.intact(lead))
            .collect();
        if lost.is_empty() {
            return;
        }
        let before = self.npcs.clone();
        for lead in lost {
            let record = before
                .iter()
                .find(|npc| npc.id == lead)
                .or_else(|| {
                    self.gone
                        .ships()
                        .iter()
                        .find(|gone| gone.ship.id == lead)
                        .map(|gone| &gone.ship)
                })
                .cloned();
            let successor = successor(&before, lead);
            for npc in self
                .npcs
                .iter_mut()
                .filter(|npc| npc.condition.active() && npc.leader == Some(lead))
            {
                match successor {
                    None => npc.leave_alone(),
                    Some(id) if id == npc.id => {
                        npc.take_over(record.as_ref(), world.successor_repairer);
                    }
                    Some(id) => npc.follow_successor(id),
                }
            }
        }
    }

    /// Whether NPC `id` is here and intact.
    fn intact(&self, id: NpcId) -> bool {
        self.npcs
            .iter()
            .any(|npc| npc.id == id && npc.condition == Condition::Intact)
    }
}

/// The successor of `lead` among `npcs`: of the intact ships it led, the
/// one of greatest class mass, the lowest berth on a tie (see the module
/// docs).
fn successor(npcs: &[Npc], lead: NpcId) -> Option<NpcId> {
    npcs.iter()
        .filter(|npc| npc.leader == Some(lead) && npc.condition == Condition::Intact)
        .min_by_key(|npc| (std::cmp::Reverse(npc.hull.signed_mass), npc.berth))
        .map(|npc| npc.id)
}

impl Npc {
    /// It leaves its fleet: no leader (+0x9a = -1), no carrier, holding its
    /// class's capacities again ([`Npc::leave_carrier`]), and its class's
    /// `InherentAI` (+0x12) as its AI type. The one writer of both: the
    /// succession, the disabled reset
    /// ([`disabled_reset`](super::disabled_reset)), `_EscortAI`'s lost
    /// parent ([`Decision::leaves_fleet`](crate::ai::Decision::leaves_fleet))
    /// and the session's releases of the player's ships.
    pub fn leave_fleet(&mut self) {
        self.leave_carrier();
        self.leader = None;
        self.ai_type = AiType::from_raw(self.hull.inherent_ai);
    }

    /// It follows `successor`, its lost lead's successor (see the module
    /// docs): led by it, keeping with it or following its jump in place of
    /// the lost lead, off any carrier, and in the regroup pause, its
    /// substate kept, while its jump is not under way.
    pub fn follow_successor(&mut self, successor: NpcId) {
        if let Goal::Follow { lead, .. } = &mut self.goal {
            *lead = successor;
        }
        if let Some(WarpGoal::Follow(ShipRef::Npc(lead))) = &mut self.warp.goal {
            *lead = successor;
        }
        self.leader = Some(successor);
        if self.carrier.is_some() {
            self.leave_carrier();
        }
        if self.warp.goal.is_none() && !self.warp.under_way() {
            self.goal = Goal::Regroup {
                kept: Kept::of(self.goal),
            };
        }
    }

    /// It is left with no fellow to follow: out of its fleet, its warp at
    /// -1.0, in the regroup pause coasting (sub 0), its target kept.
    fn leave_alone(&mut self) {
        self.leave_fleet();
        self.warp.stage = LEFT_STAGE;
        self.goal = Goal::Regroup { kept: Kept::Coast };
        self.pursuit = Pursuit::default();
        self.docking = None;
    }

    /// It leads its lost lead's fleet, the lead's `record` read when there
    /// is one, a repairer by `repairer`'s reading of
    /// [`RuleKey::SuccessorRepairer`](crate::RuleKey::SuccessorRepairer)
    /// (see the module docs).
    fn take_over(&mut self, record: Option<&Npc>, repairer: RuleSource) {
        self.leave_fleet();
        self.warp.stage = LEFT_STAGE;
        if let Some(lead) = record.filter(|lead| lead.govt.is_some() && lead.govt == self.govt) {
            self.goal = match lead.goal {
                Goal::Follow { .. } | Goal::Formation { .. } => Goal::Hold,
                goal => goal,
            };
            self.target = lead.target;
            self.travel_set = lead.travel_set;
            self.warp.goal = lead.warp.goal;
            self.pursuit = lead.pursuit;
            self.docking = lead.docking;
            return;
        }
        self.pursuit = Pursuit::default();
        self.docking = None;
        if matches!(self.warp.goal, Some(WarpGoal::Follow(_))) {
            self.goal = Goal::JumpOut;
            self.warp.goal = Some(WarpGoal::Own);
            return;
        }
        self.warp.goal = None;
        let attacks = match self.goal {
            Goal::Dock(_) => false,
            goal if goal.flies_in_form() => false,
            Goal::Inspect(_) | Goal::Assist(crate::ai::Help::Refuel) => false,
            Goal::Assist(crate::ai::Help::Repair) => repairer == RuleSource::Engine,
            _ => true,
        };
        self.goal = match self.target {
            Some(target) if attacks => Goal::Attack(target),
            _ => Goal::Idle,
        };
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::pursuit::Kept as PursuitKept;
    use crate::ai::{Behaviour, Help, Surroundings};
    use crate::bay::Carrier;
    use crate::catalog::GovtId;
    use crate::chance::{Chance, NeverFires};
    use crate::escort::FormationSlot;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use nova_sim_base::fixture::FAST;

    /// Every ship keeps its goal and target.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    const FEDS: Option<GovtId> = Some(GovtId(128));
    const REBELS: Option<GovtId> = Some(GovtId(129));
    /// The lead, at berth 1.
    const LEAD: NpcId = NpcId(0);
    /// The player, the ships' target.
    const PLAYER: ShipRef = ShipRef::Player;

    /// A [`FAST`] ship of `govt` and class mass `mass`, a trader of a
    /// warship's class (`InherentAI` 3), at the centre.
    fn ship(govt: Option<GovtId>, mass: i16) -> Npc {
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.govt = govt;
        ship.hull.signed_mass = mass;
        ship.hull.inherent_ai = 3;
        ship
    }

    /// [`ship`] following the lead, keeping formation on it.
    fn follower(govt: Option<GovtId>, mass: i16) -> Npc {
        Npc {
            leader: Some(LEAD),
            goal: Goal::Follow {
                lead: LEAD,
                guard: None,
            },
            ..ship(govt, mass)
        }
    }

    /// A traffic of the lead, of `govt` and in `condition`, at berth 1,
    /// and `followers` at berths 2, 3, …
    fn fleet(govt: Option<GovtId>, condition: Condition, followers: Vec<Npc>) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        let lead = Npc {
            condition,
            ..ship(govt, 500)
        };
        traffic.add_npc_at(lead, 1);
        for (at, follower) in (2..).zip(followers) {
            traffic.add_npc_at(follower, at);
        }
        traffic
    }

    fn of(traffic: &Traffic, id: u32) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == NpcId(id))
            .expect("there")
    }

    fn of_mut(traffic: &mut Traffic, id: u32) -> &mut Npc {
        traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == NpcId(id))
            .expect("there")
    }

    /// `traffic`'s think half by `reading` of `npc_escort_ai` and
    /// `repairer` of `successor_repairer`, never releasing a pause.
    fn think_by(traffic: &mut Traffic, reading: RuleSource, repairer: RuleSource) {
        let world = World {
            npc_escort_ai: reading,
            successor_repairer: repairer,
            ..World::new(&[])
        };
        traffic.think_holding(&Keep, world, &[], &mut NeverFires);
    }

    /// `traffic`'s think half by the engine.
    fn think(traffic: &mut Traffic) {
        think_by(traffic, RuleSource::Engine, RuleSource::Engine);
    }

    const DISABLED: Condition = Condition::Disabled;

    #[test]
    fn a_ship_leaving_its_fleet_drops_its_leader_and_carrier_and_takes_its_classs_ai() {
        let mut fighter = npc(1, ShipStats::new(FAST, &[]));
        fighter.leader = Some(NpcId(4));
        fighter.hull.inherent_ai = 3;
        fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(NpcId(4)),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        fighter.reserves.shield.now = 35.0;
        assert_eq!(fighter.reserves.shield.max, 39.99);
        fighter.leave_fleet();
        assert_eq!(fighter.leader, None);
        assert_eq!(fighter.carrier, None);
        assert_eq!(fighter.ai_type, AiType::Warship);
        assert_eq!(
            (fighter.reserves.shield.now, fighter.reserves.shield.max),
            (35.0, 30.0),
            "its class's capacity, what it holds kept"
        );
    }

    // The successor.

    #[test]
    fn the_intact_follower_of_greatest_mass_leads_the_lowest_berth_on_a_tie() {
        let mut heavy_down = follower(FEDS, 900);
        heavy_down.condition = DISABLED;
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![
                follower(FEDS, 100),
                heavy_down,
                follower(FEDS, 300),
                follower(FEDS, 300),
            ],
        );
        think(&mut traffic);
        assert_eq!(of(&traffic, 3).leader, None, "the successor at berth 4");
        for id in [1, 2, 4] {
            assert_eq!(of(&traffic, id).leader, Some(NpcId(3)), "NPC {id}");
        }
    }

    #[test]
    fn the_followers_below_and_above_the_successor_both_follow_it() {
        // `_prevParent`: the lower slot's check has rewritten its own
        // parent before the higher slot's reads the snapshot.
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![
                follower(FEDS, 100),
                follower(FEDS, 300),
                follower(FEDS, 100),
            ],
        );
        think(&mut traffic);
        assert_eq!(of(&traffic, 1).leader, Some(NpcId(2)));
        assert_eq!(of(&traffic, 3).leader, Some(NpcId(2)));
        assert_eq!(of(&traffic, 2).leader, None);
    }

    #[test]
    fn a_lead_disabled_breaking_up_destroyed_jumped_out_or_missing_is_succeeded() {
        let dying = Condition::Dying {
            ticks_left: 4,
            starting: false,
        };
        for condition in [DISABLED, dying] {
            let mut traffic = fleet(FEDS, condition, vec![follower(FEDS, 300)]);
            think(&mut traffic);
            assert_eq!(of(&traffic, 1).leader, None, "{condition:?}");
        }
        let mut destroyed = fleet(FEDS, Condition::Destroyed, vec![follower(FEDS, 300)]);
        destroyed.remove_destroyed(LEAD);
        let mut jumped = fleet(FEDS, Condition::Intact, vec![follower(FEDS, 300)]);
        let lead = of(&jumped, 0).clone();
        jumped.remove(LEAD);
        jumped.gone.record(lead, Freed::InDispatch);
        let mut missing = fleet(FEDS, Condition::Intact, vec![follower(FEDS, 300)]);
        missing.remove(LEAD);
        for (name, mut traffic) in [
            ("destroyed", destroyed),
            ("jumped", jumped),
            ("missing", missing),
        ] {
            think(&mut traffic);
            assert_eq!(of(&traffic, 1).leader, None, "{name}");
            assert_eq!(of(&traffic, 1).warp.stage, LEFT_STAGE, "{name}");
        }
    }

    #[test]
    fn an_intact_lead_changes_nothing() {
        let mut traffic = fleet(FEDS, Condition::Intact, vec![follower(FEDS, 300)]);
        think(&mut traffic);
        let kept = of(&traffic, 1);
        assert_eq!(kept.leader, Some(LEAD));
        assert!(matches!(kept.goal, Goal::Follow { lead: LEAD, .. }));
    }

    // The successor leads.

    /// The lead of `govt`, disabled, attacking NPC 9 with a travel
    /// stellar, a rush kept and a docking, its warp following the player's
    /// jump, its goal `goal`; and its one follower, a warship of the
    /// rebels, a fighter of the lead, idle, with its own rush and docking.
    fn taking_over(govt: Option<GovtId>, goal: Goal) -> Traffic {
        let mut taker = follower(REBELS, 300);
        taker.take_carrier(
            Carrier {
                ship: ShipRef::Npc(LEAD),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        taker.goal = Goal::Idle;
        let mut traffic = fleet(govt, DISABLED, vec![taker]);
        let lead = of_mut(&mut traffic, 0);
        lead.goal = goal;
        lead.target = Some(ShipRef::Npc(NpcId(9)));
        lead.travel_set = true;
        lead.warp.goal = Some(WarpGoal::Follow(PLAYER));
        lead.pursuit = Pursuit {
            kept: Some(PursuitKept::Rush),
            frame: None,
        };
        lead.docking = Some(Docking {
            ship: PLAYER,
            sub: DockSub::Close,
        });
        traffic
    }

    #[test]
    fn under_its_leads_government_the_successor_takes_over_the_leads_job() {
        let mut traffic = taking_over(REBELS, Goal::Attack(ShipRef::Npc(NpcId(9))));
        let lead = of(&traffic, 0).clone();
        think(&mut traffic);
        let taker = of(&traffic, 1);
        assert_eq!(taker.goal, Goal::Attack(ShipRef::Npc(NpcId(9))));
        assert_eq!(taker.target, lead.target);
        assert!(taker.travel_set);
        assert_eq!(taker.warp.goal, lead.warp.goal);
        assert_eq!(taker.leader, None);
        assert_eq!(taker.carrier, None);
        assert_eq!(taker.ai_type, AiType::Warship);
        assert_eq!(taker.reserves.shield.max, 30.0, "its class's capacity");
        assert_eq!(taker.warp.stage, LEFT_STAGE);
    }

    #[test]
    fn the_successor_takes_over_the_leads_substate_too() {
        // Its pursuit, which its own high level then flies on.
        let traffic = taking_over(REBELS, Goal::Attack(ShipRef::Npc(NpcId(9))));
        let mut taker = of(&traffic, 1).clone();
        taker.take_over(Some(of(&traffic, 0)), RuleSource::Engine);
        assert_eq!(taker.pursuit.kept, Some(PursuitKept::Rush));
        assert_eq!(taker.docking, of(&traffic, 0).docking);
        let mut own = of(&traffic, 1).clone();
        own.take_over(None, RuleSource::Engine);
        assert_eq!(own.pursuit, Pursuit::default(), "with no record");
    }

    #[test]
    fn a_leads_formation_is_taken_over_as_a_hold() {
        for goal in [
            Goal::Follow {
                lead: NpcId(7),
                guard: None,
            },
            Goal::Formation { guard: None },
        ] {
            let mut traffic = taking_over(REBELS, goal);
            think(&mut traffic);
            assert_eq!(of(&traffic, 1).goal, Goal::Hold, "{goal:?}");
        }
    }

    /// What the successor's own `goal`, targeting `target`, on `warp` maps
    /// to under a lead of `govt`, by `repairer` of `successor_repairer`.
    fn mapped(
        govt: Option<GovtId>,
        goal: Goal,
        target: Option<ShipRef>,
        warp: Option<WarpGoal>,
        repairer: RuleSource,
    ) -> Npc {
        let mut traffic = taking_over(govt, Goal::Attack(ShipRef::Npc(NpcId(9))));
        let taker = of_mut(&mut traffic, 1);
        taker.goal = goal;
        taker.target = target;
        taker.warp.goal = warp;
        taker.pursuit.kept = Some(PursuitKept::Rush);
        taker.docking = Some(Docking {
            ship: PLAYER,
            sub: DockSub::Approach,
        });
        think_by(&mut traffic, RuleSource::Engine, repairer);
        of(&traffic, 1).clone()
    }

    #[test]
    fn under_another_government_or_none_its_own_goal_maps() {
        let t = Some(PLAYER);
        let engine = RuleSource::Engine;
        let follow = Goal::Follow {
            lead: LEAD,
            guard: None,
        };
        for govt in [FEDS, None] {
            for (goal, target, mapped_to) in [
                (Goal::Dock(ShipRef::Npc(LEAD)), t, Goal::Idle),
                (follow, t, Goal::Idle),
                (Goal::Formation { guard: None }, t, Goal::Idle),
                (Goal::Hold, t, Goal::Idle),
                (Goal::Idle, t, Goal::Attack(PLAYER)),
                (Goal::Wait, t, Goal::Attack(PLAYER)),
                (Goal::Snipe(PLAYER), t, Goal::Attack(PLAYER)),
                (Goal::Inspect(PLAYER), t, Goal::Idle),
                (Goal::Assist(Help::Refuel), t, Goal::Idle),
                (Goal::Assist(Help::Repair), t, Goal::Attack(PLAYER)),
                (Goal::Idle, None, Goal::Idle),
                (Goal::Wait, None, Goal::Idle),
            ] {
                let taker = mapped(govt, goal, target, None, engine);
                assert_eq!(taker.goal, mapped_to, "{govt:?} {goal:?}");
                assert_eq!(taker.target, target, "{govt:?} {goal:?}: kept");
                assert_eq!(taker.pursuit, Pursuit::default(), "{govt:?} {goal:?}");
                assert_eq!(taker.docking, None, "{govt:?} {goal:?}");
                assert_eq!(taker.warp.goal, None, "{govt:?} {goal:?}");
                assert_eq!(taker.leader, None, "{govt:?} {goal:?}");
                assert_eq!(taker.ai_type, AiType::Warship, "{govt:?} {goal:?}");
            }
        }
    }

    #[test]
    fn following_its_leads_jump_it_jumps_on_its_own() {
        let taker = mapped(
            FEDS,
            Goal::Idle,
            None,
            Some(WarpGoal::Follow(ShipRef::Npc(LEAD))),
            RuleSource::Engine,
        );
        assert_eq!(taker.goal, Goal::JumpOut);
        assert_eq!(taker.warp.goal, Some(WarpGoal::Own));
    }

    #[test]
    fn a_repairer_attacks_the_player_it_repaired_by_the_engine_and_idles_by_the_other() {
        let repairing = |repairer| {
            mapped(
                FEDS,
                Goal::Assist(Help::Repair),
                Some(PLAYER),
                None,
                repairer,
            )
            .goal
        };
        assert_eq!(repairing(RuleSource::Engine), Goal::Attack(PLAYER));
        assert_eq!(repairing(RuleSource::Bible), Goal::Idle);
        // The key reads only the repair.
        let refuelling = mapped(
            FEDS,
            Goal::Assist(Help::Refuel),
            Some(PLAYER),
            None,
            RuleSource::Bible,
        );
        assert_eq!(refuelling.goal, Goal::Idle);
        let idle = mapped(FEDS, Goal::Idle, Some(PLAYER), None, RuleSource::Bible);
        assert_eq!(idle.goal, Goal::Attack(PLAYER));
    }

    // Another ship leads.

    #[test]
    fn every_other_follower_follows_the_successor_and_pauses_keeping_its_substate() {
        let mut guarding = follower(FEDS, 100);
        guarding.goal = Goal::Follow {
            lead: LEAD,
            guard: Some(PLAYER),
        };
        let mut fighting = follower(FEDS, 100);
        fighting.goal = Goal::Attack(PLAYER);
        fighting.target = Some(PLAYER);
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![follower(FEDS, 300), guarding, fighting],
        );
        think(&mut traffic);
        let successor = NpcId(1);
        let guarding = of(&traffic, 2);
        assert_eq!(guarding.leader, Some(successor));
        assert_eq!(
            guarding.goal,
            Goal::Regroup {
                kept: Kept::Formation {
                    guard: Some(PLAYER)
                }
            }
        );
        assert_eq!(
            guarding.flown_goal(),
            Goal::Follow {
                lead: successor,
                guard: Some(PLAYER)
            }
        );
        assert_eq!(guarding.ai_type, AiType::WimpyTrader, "it keeps its type");
        let fighting = of(&traffic, 3);
        assert_eq!(
            fighting.goal,
            Goal::Regroup {
                kept: Kept::Attack(PLAYER)
            }
        );
        assert_eq!(fighting.target, Some(PLAYER));
    }

    #[test]
    fn a_fighter_following_the_successor_leaves_its_carrier_for_its_classs_capacities() {
        let mut fighter = follower(FEDS, 100);
        fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(LEAD),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        let mut traffic = fleet(FEDS, DISABLED, vec![follower(FEDS, 300), fighter]);
        think(&mut traffic);
        let fighter = of(&traffic, 2);
        assert_eq!(fighter.leader, Some(NpcId(1)));
        assert_eq!(fighter.carrier, None);
        assert_eq!(fighter.reserves.shield.max, 30.0);
    }

    #[test]
    fn a_follower_on_a_jump_follows_the_successors_and_keeps_its_goal() {
        let mut jumping = follower(FEDS, 100);
        jumping.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(LEAD)));
        let mut under_way = follower(FEDS, 100);
        under_way.warp.stage = 3.0;
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![follower(FEDS, 300), jumping, under_way],
        );
        think(&mut traffic);
        let jumping = of(&traffic, 2);
        assert_eq!(
            jumping.warp.goal,
            Some(WarpGoal::Follow(ShipRef::Npc(NpcId(1))))
        );
        assert_eq!(
            jumping.goal,
            Goal::Follow {
                lead: NpcId(1),
                guard: None
            }
        );
        let under_way = of(&traffic, 3);
        assert_eq!(under_way.leader, Some(NpcId(1)));
        assert_eq!(
            under_way.goal,
            Goal::Follow {
                lead: NpcId(1),
                guard: None
            },
            "its jump is under way"
        );
    }

    #[test]
    fn a_disabled_follower_alone_is_left_leaderless_in_the_pause_its_target_kept() {
        let mut alone = follower(FEDS, 300);
        alone.condition = DISABLED;
        alone.target = Some(PLAYER);
        alone.leader = Some(LEAD);
        let mut traffic = fleet(FEDS, DISABLED, vec![alone]);
        think(&mut traffic);
        let alone = of(&traffic, 1);
        assert_eq!(alone.leader, None);
        assert_eq!(alone.ai_type, AiType::Warship);
        assert_eq!(alone.goal, Goal::Regroup { kept: Kept::Coast });
        assert_eq!(alone.target, Some(PLAYER), "held in the pause, not reset");
        assert_eq!(alone.warp.stage, LEFT_STAGE);
    }

    #[test]
    fn the_successors_followers_form_on_it_in_the_same_tick() {
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![follower(FEDS, 300), follower(FEDS, 100)],
        );
        think(&mut traffic);
        assert_eq!(
            of(&traffic, 2).fleet_slot,
            Some(FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 38.0
            })
        );
        assert_eq!(of(&traffic, 1).fleet_slot, None);
    }

    #[test]
    fn by_the_other_reading_nothing_changes() {
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![follower(FEDS, 300), follower(FEDS, 100)],
        );
        think_by(&mut traffic, RuleSource::Bible, RuleSource::Engine);
        for id in [1, 2] {
            let kept = of(&traffic, id);
            assert_eq!(kept.leader, Some(LEAD), "NPC {id}");
            assert!(
                matches!(kept.goal, Goal::Follow { lead: LEAD, .. }),
                "NPC {id}"
            );
        }
    }
}
