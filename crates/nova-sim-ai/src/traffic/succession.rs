//! A fleet whose lead is lost: `_AICheckParentExists` (@0x8926a-0x89501),
//! read whole ([`Traffic::succeed_leaders`]), and a ship leaving its fleet
//! ([`Npc::leave_fleet`]).
//!
//! **When.** `_DoPlayGameWork`'s marking pass (@0x4405e-0x441f7) runs it
//! once a frame, before the arrivals and every dispatch, for each ship
//! that is active, of AI type above 4 and led by an NPC (@0x440d8-0x440e0,
//! @0x440f4-0x44101), so not a bribed one
//! ([`Npc::original_ai_type`]), which keeps its lost parent, though the
//! search below may pick it. It then works out the formation parents
//! (@0x4415c). The think half runs
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
//!    in goal 0x13, sub 0 ([`Kept::Coast`]), its target kept. The sub 0
//!    write (@0x894e3) ends a jump the disabled cancel left it on, sub 4 or
//!    0xd, so it is off the jump path ([`WarpGoal`] none) and draws its
//!    release, disabled or whole. In its gate it is held there, and its
//!    glide ends ([`Npc::leaves_arrival`], @0x894ec).
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
//!
//!    The lead's word is copied in the marking pass, before the lead's own
//!    dispatch can reset it (@0x8fece), and the engine's order matches: on
//!    the frame it is lost a lead in its gate still holds goal 0x15, a lead
//!    the player hit there goal 4 ([`Npc::hold_at_gate`]). So the copy may
//!    be any of these:
//!    - goal 0x15 ([`Npc::emerging`], [`Npc::takes_gate_goal`], @0x893d0,
//!      @0x8dd61-0x8dd93): the successor stays in or goes back to its gate;
//!      flying with no wait it starts a glide, from what it owes or else
//!      from 50 (sub 0xa keeps a negative +0x34, @0x8811e-0x8812c,
//!      [`burst_word`](super::burst_word)); a glide or burst goes on under
//!      goal 8, as the high level turns 0x15 to 8 at once; and flying under
//!      a wait or holding its course it hides where it is, holding its
//!      lead's gate open, until the wait is out, then bursts from its own
//!      word ([`Npc::bursts_own_word`]). The turn-out's +0x70 = -1
//!      (@0x8dd7b) drops the target copied from the lead, at once for a
//!      ship flying with no wait, gliding or bursting, and at its own
//!      turn-out for one that waits;
//!    - goal 8 (the lead glides in or bursts out, [`Npc::jumping_in`]): at
//!      -1.0 it is goal 0 (@0x8fda0-0x8fdaa), then sub 0 (@0x8fb09-0x8fb13);
//!    - goal 0xb (the lead followed a jump of its own lead): with no parent
//!      the goal's block makes it goal 2, sub 4, target and travel stellar
//!      -1 (@0x8de7c → @0x8df54).
//!
//!    Otherwise the writes end the successor's arrival
//!    ([`Npc::leaves_arrival`]): a goal over goal 0x15 holds it in its gate
//!    ([`Npc::leaves_gate_goal`], @0x893d0, @0x89429-0x8945f), and the -1.0
//!    over a glide's -999.0 (@0x8937c) and a sub over sub 0xa end the
//!    glide or burst, the rest of it owed. The mapping reads the word 0x15
//!    or 8, none of its listed goals, not the goal the engine keeps beside
//!    a gate or a glide.
//! 4. **Another leads** (@0x89479-0x894af, [`Npc::follow_successor`]): its
//!    parent is the successor; a fighter (AI type 5) becomes an escort (6,
//!    @0x89480-0x89491), so off its carrier with its class's capacities;
//!    and while its +0x50 is below 0.0 ([`Npc::stage_below_zero`]) it
//!    pauses in goal 0x13, its substate kept ([`Kept::of`]): sub 4 or 0xd
//!    on its jump (its [`WarpGoal`], rewritten to the successor), else
//!    sub 0, the only substates that reach it
//!    ([`regroup`](crate::ai::regroup)). An escort rests at 0, as the hold
//!    undoes `_EscortAI`'s -1.0 in the same dispatch (see
//!    [`jump_with_leader`](super::jump_with_leader)), so one whose lead is
//!    lost forms on the successor at once; a follower that gave its stage
//!    up pauses, and its jump path then runs every frame.
//!
//!    The emerge's -1.0 (@0x89555) stands through a gate wait, which skips
//!    the hold, so a follower whose lead is lost in its gate pauses too, in
//!    sub 0: goal 0x13 over goal 0x15 holds it there, seen, for about 100
//!    frames ([`Npc::leaves_gate_goal`]), where a ship gliding in takes the
//!    write only for the glide path to overwrite it (@0x8fcf5-0x8fd18).
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

use super::npc::{AiType, Mode, Npc, NpcId};
use super::warp_out::WarpGoal;
use super::{Traffic, World};
use crate::ai::Goal;
use crate::ai::pursuit::Pursuit;
use crate::ai::regroup::Kept;
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;

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
            for npc in self.npcs.iter_mut().filter(|npc| {
                npc.condition.active() && npc.leader == Some(lead) && npc.original_ai_type() > 4
            }) {
                match successor {
                    None => npc.leave_alone(world.held_burst),
                    Some(id) if id == npc.id => npc.take_over(record.as_ref(), world),
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
    /// `InherentAI` (+0x12) as its AI type, the disabled reset's mark
    /// ([`Npc::kept_escort_ai`]) cleared. A bribed ship ([`Npc::spared`])
    /// keeps the bribe's AI type: the original's leaves that write +0x88
    /// run only for AI types 5 and 6, and the disabled reset writes none
    /// (see [`original_ai_type`](super::original_ai_type)). The one writer
    /// of both: the
    /// succession, the disabled reset
    /// ([`disabled_reset`](super::disabled_reset)), `_EscortAI`'s lost
    /// parent ([`Decision::leaves_fleet`](crate::ai::Decision::leaves_fleet))
    /// and the session's releases of the player's ships.
    pub fn leave_fleet(&mut self) {
        self.leave_carrier();
        self.leader = None;
        if !self.spared {
            self.ai_type = AiType::from_raw(self.hull.inherent_ai);
        }
        self.kept_escort_ai = false;
    }

    /// It follows `successor`, its lost lead's successor (see the module
    /// docs): led by it, keeping with it or following its jump in place of
    /// the lost lead, off any carrier, and in the regroup pause, its
    /// substate (and any jump) kept, while its stage is below 0.
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
        if self.stage_below_zero() {
            self.leaves_gate_goal();
            self.goal = Goal::Regroup {
                kept: Kept::of(self),
            };
        }
    }

    /// It is left with no fellow to follow, an owed burst by `burst`'s
    /// reading: out of its fleet, any gate goal and any glide, its warp at
    /// -1.0 and off any jump, in the regroup pause coasting (sub 0), its
    /// target kept.
    fn leave_alone(&mut self, burst: RuleSource) {
        self.leaves_arrival(burst);
        self.leave_fleet();
        self.warp.give_up();
        self.warp.goal = None;
        self.goal = Goal::Regroup { kept: Kept::Coast };
        self.pursuit = Pursuit::default();
        self.docking = None;
    }

    /// A -1.0 and a goal are written over its arrival: it leaves any gate
    /// goal ([`Npc::leaves_gate_goal`]), and any glide or burst, as the
    /// -1.0 over -999.0 ends the glide path (@0x8fcf5) and the sub written
    /// ends sub 0xa. It then flies, at once, its +0x34 left negative: it
    /// owes the rest of its burst, from the word it started from
    /// ([`Npc::burst_speed`]), ([`Npc::owed_burst`]) by `burst`'s reading
    /// of [`RuleKey::HeldGateBurst`](crate::RuleKey::HeldGateBurst), as a
    /// ship held at its gate does. The player's warp stage does the same
    /// ([`Npc::follow_player_jump`]).
    pub(crate) fn leaves_arrival(&mut self, burst: RuleSource) {
        self.leaves_gate_goal();
        let (Mode::JumpingIn { tick } | Mode::LeavingGate { tick }) = self.mode else {
            return;
        };
        let speed = self.burst_speed(tick);
        self.mode = Mode::Flying;
        self.owed_burst = (burst == RuleSource::Engine).then_some(speed);
        self.bursts_own_word = false;
    }

    /// It takes its lead's goal 0x15 (@0x893c8-0x893d0), the gate its
    /// lead's `gate` (+0x6c, @0x893bd-0x893c2), and goal 0 as any burst ends
    /// (`_AIMakeShipStopJumping` @0x82e96), by its mode:
    ///
    /// - **In its gate** it waits on, its emerge's word kept; **held at its
    ///   gate** it hides again, its wait and its emerge's +0x34 running on.
    /// - **Gliding or bursting** it turns back to goal 8 at once (@0x8dd8a),
    ///   so flies on.
    /// - **Flying with no wait** the high level turns it to goal 8 in the
    ///   same dispatch, +0x6c gone, and sub 0xa starts its glide from what
    ///   it owes, else from 50 (@0x8811e-0x8812c, [`Npc::burst_speed`]).
    /// - **Flying under a wait** ([`Npc::ai_wait`]) **or holding its
    ///   course** ([`Mode::Coasting`]) its dispatch skips the type think and
    ///   its high level returns at the wait gate (@0x8ff18 → @0x9001f,
    ///   @0x8d4b9-0x8d4d2), so it keeps goal 0x15 while the wait runs:
    ///   hidden where it is, holding the gate open from there, moving on its
    ///   velocity ([`Mode::Emerging`] for what is left of the wait), its
    ///   trigger released. It then bursts from its own word
    ///   ([`Npc::bursts_own_word`]): what it owes, else 50.
    ///
    /// Whether the turn-out's +0x70 = -1 (@0x8dd7b) reaches it in this
    /// dispatch, as it does unless it waits: the caller drops its target
    /// then. A ship entering a gate or adrift is never led (see the module
    /// docs), so is left.
    fn takes_gate_goal(&mut self, gate: Option<StellarId>) -> bool {
        self.goal = Goal::Idle;
        match self.mode {
            Mode::Emerging { .. } | Mode::LeavingGate { tick: 0 } => {}
            Mode::HeldAtGate { ticks_left } => self.mode = Mode::Emerging { ticks_left },
            Mode::Flying if self.ai_wait == 0 => {
                self.mode = Mode::JumpingIn { tick: 0 };
                return true;
            }
            Mode::Flying | Mode::Coasting { .. } => {
                let ticks_left = match self.mode {
                    Mode::Coasting { ticks_left } => ticks_left,
                    _ => self.ai_wait,
                };
                self.ai_wait = 0;
                self.mode = Mode::Emerging { ticks_left };
                self.trigger = Trigger::default();
                self.bursts_own_word = true;
            }
            Mode::JumpingIn { .. } | Mode::LeavingGate { .. } => return true,
            Mode::EnteringGate { .. } | Mode::Adrift { .. } => return false,
        }
        self.emerges_from = gate;
        false
    }

    /// It leads its lost lead's fleet, the lead's `record` read when there
    /// is one, a repairer by `world`'s reading of
    /// [`RuleKey::SuccessorRepairer`](crate::RuleKey::SuccessorRepairer)
    /// (see the module docs).
    fn take_over(&mut self, record: Option<&Npc>, world: &World) {
        // Its goal word is 0x15 or 8, whatever the engine keeps beside it.
        let arriving = self.emerging() || self.jumping_in();
        self.leave_fleet();
        self.warp.give_up();
        if let Some(lead) = record.filter(|lead| lead.govt.is_some() && lead.govt == self.govt) {
            if lead.follows_leaders_jump() {
                // Goal 0xb copied, with no parent (@0x8de7c → @0x8df54).
                self.leaves_arrival(world.held_burst);
                self.goal = Goal::JumpOut;
                self.warp.goal = Some(WarpGoal::Own);
                self.target = None;
                self.travel_set = false;
                self.pursuit = Pursuit::default();
                self.docking = None;
                return;
            }
            self.target = lead.target;
            if lead.emerging() {
                if self.takes_gate_goal(lead.emerges_from) {
                    // The turn-out in the same dispatch (@0x8dd7b).
                    self.target = None;
                }
            } else {
                self.leaves_arrival(world.held_burst);
                self.goal = match lead.goal {
                    // Goal 8 at -1.0 is goal 0 (@0x8fda0-0x8fdaa).
                    _ if lead.jumping_in() => Goal::Idle,
                    Goal::Follow { .. } | Goal::Formation { .. } => Goal::Hold,
                    goal => goal,
                };
            }
            self.travel_set = lead.travel_set;
            self.warp.goal = lead.warp.goal;
            self.stopping = lead.stopping;
            self.pursuit = lead.pursuit;
            self.docking = lead.docking;
            return;
        }
        self.leaves_arrival(world.held_burst);
        self.pursuit = Pursuit::default();
        self.docking = None;
        if matches!(self.warp.goal, Some(WarpGoal::Follow(_))) || self.joins_leaders_jump() {
            self.goal = Goal::JumpOut;
            self.warp.goal = Some(WarpGoal::Own);
            return;
        }
        self.warp.goal = None;
        let attacks = match self.goal {
            _ if arriving => true,
            Goal::Dock(_) => false,
            goal if goal.flies_in_form() => false,
            Goal::Inspect(_) | Goal::Assist(crate::ai::Help::Refuel) => false,
            Goal::Assist(crate::ai::Help::Repair) => world.successor_repairer == RuleSource::Engine,
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
    use crate::ai::{Behaviour, Help, Surroundings, dispatch_gate};
    use crate::bay::Carrier;
    use crate::catalog::GovtId;
    use crate::chance::{Chance, NeverFires};
    use crate::escort::FormationSlot;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::autopilot::TopSpeeds;
    use crate::traffic::emerge::exit_speed;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use crate::traffic::regroup::RELEASE_DRAW;
    use crate::traffic::spawn::{JUMP_IN_SLOWING, JUMP_IN_SPEED, glide_speed};
    use crate::traffic::warp_out::{FOLLOW_BURST, GIVEN_UP_STAGE};
    use nova_sim_base::fixture::{Draws, FAST};

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

    #[test]
    fn a_ship_leaving_its_fleet_forgets_the_disabled_resets_mark() {
        let mut marked = npc(1, ShipStats::new(FAST, &[]));
        marked.kept_escort_ai = true;
        marked.hull.inherent_ai = 3;
        marked.leave_fleet();
        assert!(!marked.kept_escort_ai);
        assert_eq!(marked.original_ai_type(), 3);
    }

    #[test]
    fn a_bribed_ship_leaving_its_fleet_keeps_the_bribes_word() {
        // None of the original's leaves reaches +0x88 = 1 (see `leave_fleet`).
        let mut spared = npc(1, ShipStats::new(FAST, &[]));
        spared.leader = Some(NpcId(4));
        spared.hull.inherent_ai = 3;
        spared.take_bribe();
        spared.leave_fleet();
        assert_eq!(spared.leader, None);
        assert!(spared.spared);
        assert_eq!(spared.ai_type, AiType::WimpyTrader);
        assert_eq!(spared.original_ai_type(), 1);
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
        for id in [1, 4] {
            assert_eq!(of(&traffic, id).leader, Some(NpcId(3)), "NPC {id}");
        }
        // The disabled one, at rest, takes no pause (its +0x50 isn't below
        // 0), so its dispatch reaches the disabled reset, which drops the
        // successor it was given.
        assert_eq!(of(&traffic, 2).leader, None, "NPC 2");
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
        // Each lead holds, which the successor takes over from its record,
        // there or gone; with no record, its own formation idles.
        let holding = |condition| {
            let mut traffic = fleet(FEDS, condition, vec![follower(FEDS, 300)]);
            of_mut(&mut traffic, 0).goal = Goal::Hold;
            traffic
        };
        let dying = Condition::Dying {
            ticks_left: 4,
            starting: false,
        };
        let mut destroyed = holding(Condition::Destroyed);
        destroyed.remove_destroyed(LEAD);
        let mut jumped = holding(Condition::Intact);
        let lead = of(&jumped, 0).clone();
        jumped.remove(LEAD);
        jumped.gone.record(lead, Freed::InDispatch);
        let mut missing = holding(Condition::Intact);
        missing.remove(LEAD);
        for (name, mut traffic, goal) in [
            ("disabled", holding(DISABLED), Goal::Hold),
            ("dying", holding(dying), Goal::Hold),
            ("destroyed", destroyed, Goal::Hold),
            ("jumped", jumped, Goal::Hold),
            ("missing", missing, Goal::Idle),
        ] {
            think(&mut traffic);
            let successor = of(&traffic, 1);
            assert_eq!(successor.leader, None, "{name}");
            assert_eq!(successor.goal, goal, "{name}");
            assert_eq!(successor.warp.stage, GIVEN_UP_STAGE, "{name}");
        }
    }

    #[test]
    fn a_bribed_follower_takes_no_check_but_may_be_the_successor() {
        // The marking pass skips AI type 1 (@0x440d8-0x440e0), but the
        // successor's search reads only the parent (@0x892af-0x8933d). A
        // later write of its word stays at or below 4 too (`jle` @0x440e0).
        for word in [AiType::WimpyTrader, AiType::Interceptor] {
            let mut bribed = follower(FEDS, 900);
            bribed.take_bribe();
            bribed.ai_type = word;
            let mut traffic = fleet(FEDS, DISABLED, vec![bribed, follower(FEDS, 100)]);
            think(&mut traffic);
            assert_eq!(of(&traffic, 1).leader, Some(LEAD), "{word:?}: no check");
            assert_eq!(of(&traffic, 2).leader, Some(NpcId(1)), "{word:?}");
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
    /// stellar, a rush kept and a docking, on its own jump, its goal
    /// `goal`; and its one follower, a warship of the rebels, a fighter of
    /// the lead, idle, with its own rush and docking.
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
        lead.warp.goal = Some(WarpGoal::Own);
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
        assert_eq!(taker.warp.stage, GIVEN_UP_STAGE);
    }

    #[test]
    fn the_successor_takes_over_the_leads_substate_too() {
        // Its pursuit, which its own high level then flies on.
        let traffic = taking_over(REBELS, Goal::Attack(ShipRef::Npc(NpcId(9))));
        let mut taker = of(&traffic, 1).clone();
        taker.take_over(Some(of(&traffic, 0)), &World::new(&[]));
        assert_eq!(taker.pursuit.kept, Some(PursuitKept::Rush));
        assert_eq!(taker.docking, of(&traffic, 0).docking);
        let mut own = of(&traffic, 1).clone();
        own.take_over(None, &World::new(&[]));
        assert_eq!(own.pursuit, Pursuit::default(), "with no record");
    }

    #[test]
    fn the_successor_takes_its_leads_stored_sub_1_under_its_government() {
        // +0xc8ca copied beside the goal (@0x893d9-0x893e1).
        let traffic = taking_over(REBELS, Goal::Wait);
        let mut lead = of(&traffic, 0).clone();
        lead.stopping = true;
        let mut taker = of(&traffic, 1).clone();
        taker.take_over(Some(&lead), &World::new(&[]));
        assert!(taker.stopping);
        let mut own = of(&traffic, 1).clone();
        own.take_over(None, &World::new(&[]));
        assert!(!own.stopping, "with no record");
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
    fn an_independent_successor_of_an_independent_lead_maps_its_own_goal() {
        // Two ships of no government share none: the take-over reads the
        // lead only when both are of one government.
        let mut traffic = taking_over(None, Goal::Hold);
        let taker = of_mut(&mut traffic, 1);
        taker.govt = None;
        taker.target = Some(PLAYER);
        think(&mut traffic);
        let taker = of(&traffic, 1);
        assert_eq!(taker.goal, Goal::Attack(PLAYER));
        assert_eq!(taker.target, Some(PLAYER), "its own target, not the lead's");
        assert!(!taker.travel_set, "not the lead's travel stellar");
        assert_eq!(taker.warp.goal, None, "not the lead's warp");
        assert_eq!(taker.pursuit, Pursuit::default());
        assert_eq!(taker.docking, None);
        assert_eq!(taker.leader, None);
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
    fn off_its_jump_a_follower_pauses_in_sub_0_whatever_its_goal() {
        // The pause takes a follower whose +0x50 is below 0
        // (@0x8949d-0x894a4), as a give-up leaves it in sub 0, whatever
        // goal the engine still holds beside it.
        for goal in [
            Goal::Follow {
                lead: LEAD,
                guard: Some(PLAYER),
            },
            Goal::Attack(PLAYER),
            Goal::Dock(ShipRef::Npc(LEAD)),
            Goal::Wait,
            Goal::Idle,
        ] {
            let mut given_up = follower(FEDS, 100);
            given_up.goal = goal;
            given_up.target = Some(PLAYER);
            given_up.warp.stage = -1.0;
            let mut traffic = fleet(FEDS, DISABLED, vec![follower(FEDS, 300), given_up]);
            think(&mut traffic);
            let paused = of(&traffic, 2);
            assert_eq!(paused.leader, Some(NpcId(1)), "{goal:?}");
            assert_eq!(paused.goal, Goal::Regroup { kept: Kept::Coast });
            assert_eq!(paused.flown_goal(), Goal::Hold);
            assert_eq!(paused.ai_type, AiType::WimpyTrader, "it keeps its type");
            assert_eq!(paused.target, Some(PLAYER), "{goal:?}");
        }
    }

    #[test]
    fn a_follower_on_its_own_jump_given_up_keeps_sub_4() {
        // The disabled cancel's -1.0 (@0x8fce9) on the jump path, its lead
        // lost: it stays on the jump path.
        let mut given_up = follower(FEDS, 100);
        given_up.goal = Goal::JumpOut;
        given_up.warp.goal = Some(WarpGoal::Own);
        given_up.warp.stage = -1.0;
        let mut traffic = fleet(FEDS, DISABLED, vec![follower(FEDS, 300), given_up]);
        think(&mut traffic);
        let paused = of(&traffic, 2);
        assert_eq!(
            paused.goal,
            Goal::Regroup {
                kept: Kept::OwnJump
            }
        );
        assert_eq!(paused.warp.goal, Some(WarpGoal::Own));
    }

    /// The follower NPC 2 of a lost lead, `setup` applied, once a think
    /// has given NPC 1 the lead.
    fn beside_a_successor(setup: impl Fn(&mut Npc)) -> Npc {
        let mut ship = follower(FEDS, 100);
        ship.emerges_from = Some(StellarId(310));
        setup(&mut ship);
        let mut traffic = fleet(FEDS, DISABLED, vec![follower(FEDS, 300), ship]);
        think(&mut traffic);
        of(&traffic, 2).clone()
    }

    #[test]
    fn a_follower_waiting_in_its_gate_pauses_seen_and_out_of_its_gate() {
        // The emerge's +0x50 of -1.0 (@0x89555), a wait skipping the hold:
        // goal 0x13 over goal 0x15.
        let paused = beside_a_successor(|ship| ship.mode = Mode::Emerging { ticks_left: 30 });
        assert_eq!(paused.leader, Some(NpcId(1)));
        assert_eq!(paused.goal, Goal::Regroup { kept: Kept::Coast });
        assert_eq!(paused.mode, Mode::HeldAtGate { ticks_left: 30 });
        assert_eq!(paused.emerges_from, None);
        assert!(!paused.emerging());
    }

    #[test]
    fn a_follower_held_at_its_gate_pauses_in_sub_0_whatever_its_goal() {
        let paused = beside_a_successor(|ship| {
            ship.mode = Mode::HeldAtGate { ticks_left: 12 };
            ship.emerges_from = None;
            ship.goal = Goal::Attack(PLAYER);
            ship.target = Some(PLAYER);
        });
        assert_eq!(paused.goal, Goal::Regroup { kept: Kept::Coast });
        assert_eq!(paused.mode, Mode::HeldAtGate { ticks_left: 12 });
        assert_eq!(paused.target, Some(PLAYER));
    }

    #[test]
    fn a_follower_on_the_frame_its_wait_ran_out_pauses_and_bursts_no_more() {
        let paused = beside_a_successor(|ship| ship.mode = Mode::LeavingGate { tick: 0 });
        assert_eq!(paused.goal, Goal::Regroup { kept: Kept::Coast });
        assert_eq!(paused.mode, Mode::HeldAtGate { ticks_left: 1 });
        assert_eq!(paused.emerges_from, None);
    }

    #[test]
    fn a_follower_gliding_in_or_bursting_out_takes_no_pause() {
        // Its -999.0 takes the write, which the glide path overwrites in
        // the same frame (@0x8fcf5-0x8fd18).
        for mode in [Mode::JumpingIn { tick: 3 }, Mode::LeavingGate { tick: 1 }] {
            let led = beside_a_successor(|ship| ship.mode = mode);
            assert_eq!(led.leader, Some(NpcId(1)), "{mode:?}");
            assert_eq!(led.mode, mode);
            assert!(matches!(led.goal, Goal::Follow { .. }), "{mode:?}");
            assert_eq!(led.emerges_from, Some(StellarId(310)), "{mode:?}");
        }
    }

    // A lead lost in its gate, gliding in or bursting out.

    /// The gate the lead comes out of in [`under_a_lead_in`].
    const LEADS_GATE: Option<StellarId> = Some(StellarId(311));

    /// A lead of the feds in `mode`, coming out of [`LEADS_GATE`], disabled
    /// by something other than the player's hit, and its one follower of
    /// `govt`, `setup` applied.
    fn under_a_lead_in(mode: Mode, govt: Option<GovtId>, setup: impl Fn(&mut Npc)) -> Traffic {
        let mut ship = follower(govt, 300);
        setup(&mut ship);
        let mut traffic = fleet(FEDS, DISABLED, vec![ship]);
        let lead = of_mut(&mut traffic, 0);
        lead.mode = mode;
        lead.emerges_from = LEADS_GATE;
        traffic
    }

    /// A lead waiting in its gate.
    const LEAD_WAITING: Mode = Mode::Emerging { ticks_left: 9 };

    /// `setup` for a follower waiting in another gate.
    fn in_its_gate(ship: &mut Npc, mode: Mode) {
        ship.mode = mode;
        ship.emerges_from = Some(StellarId(310));
    }

    /// `traffic`'s think half by `burst`'s reading of `held_burst`.
    fn think_bursting(traffic: &mut Traffic, burst: RuleSource) {
        let world = World {
            held_burst: burst,
            ..World::new(&[])
        };
        traffic.think_holding(&Keep, world, &[], &mut NeverFires);
    }

    #[test]
    fn a_successor_in_its_gate_copies_its_leads_goal_0x15_and_stays_there() {
        // @0x893c8-0x893d0 copies the goal word, in the marking pass, so
        // before the lead's disabled reset (@0x8fece) replaces it.
        for lead_mode in [LEAD_WAITING, Mode::LeavingGate { tick: 0 }] {
            for mode in [
                Mode::Emerging { ticks_left: 30 },
                Mode::LeavingGate { tick: 0 },
            ] {
                let mut traffic = under_a_lead_in(lead_mode, FEDS, |ship| in_its_gate(ship, mode));
                think(&mut traffic);
                let leads = of(&traffic, 1);
                assert_eq!(leads.leader, None, "{lead_mode:?} {mode:?}");
                assert_eq!(leads.mode, mode, "{lead_mode:?}");
                assert!(leads.emerging(), "{lead_mode:?} {mode:?}");
                assert_eq!(leads.emerges_from, LEADS_GATE, "+0x6c copied");
                assert_eq!(leads.goal, Goal::Idle, "goal 0 as its burst ends");
                assert_eq!(leads.target, None, "{lead_mode:?} {mode:?}");
            }
        }
    }

    #[test]
    fn a_lead_gone_from_its_gate_leaves_its_goal_0x15_in_its_slot() {
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            in_its_gate(ship, Mode::Emerging { ticks_left: 30 });
        });
        let lead = of(&traffic, 0).clone();
        traffic.remove(LEAD);
        traffic.gone.record(lead, Freed::InDispatch);
        think(&mut traffic);
        let leads = of(&traffic, 1);
        assert_eq!(leads.mode, Mode::Emerging { ticks_left: 30 });
        assert_eq!(leads.emerges_from, LEADS_GATE);
    }

    #[test]
    fn a_successor_that_stayed_in_its_gate_bursts_out_as_its_wait_ends() {
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            in_its_gate(ship, Mode::Emerging { ticks_left: 2 });
        });
        for _ in 0..3 {
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut NeverFires);
        }
        let leads = of(&traffic, 1);
        assert_eq!(leads.mode, Mode::LeavingGate { tick: 1 });
        assert_eq!(leads.emerges_from, None);
        assert_eq!(leads.owed_burst, None, "its own burst, not an owed one");
    }

    #[test]
    fn a_successor_held_at_its_gate_takes_goal_0x15_back_and_hides_again() {
        // Goal 4 is overwritten too; its wait runs on, and its emerge's
        // +0x34 (@0x895aa-0x895bd) stands for the burst.
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            ship.mode = Mode::HeldAtGate { ticks_left: 12 };
            ship.goal = Goal::Attack(PLAYER);
            ship.target = Some(PLAYER);
        });
        think(&mut traffic);
        let leads = of(&traffic, 1);
        assert_eq!(leads.mode, Mode::Emerging { ticks_left: 12 });
        assert!(leads.emerging());
        assert_eq!(leads.emerges_from, LEADS_GATE);
        assert_eq!(leads.goal, Goal::Idle);
        assert_eq!(leads.target, None, "the lead's");
    }

    /// The lead's target in [`with_lead_target`].
    const LEADS_TARGET: Option<ShipRef> = Some(ShipRef::Npc(NpcId(9)));

    /// `traffic` with its lead holding [`LEADS_TARGET`].
    fn with_lead_target(mut traffic: Traffic) -> Traffic {
        of_mut(&mut traffic, 0).target = LEADS_TARGET;
        traffic
    }

    /// How far NPC 1 moves in each of `traffic`'s next ticks by the engine,
    /// while `going` holds for it after the tick, and its state then.
    fn moves_while(traffic: &mut Traffic, going: impl Fn(&Npc) -> bool) -> (Vec<f32>, Npc) {
        let mut moves = Vec::new();
        loop {
            let at = of(traffic, 1).state.position;
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut NeverFires);
            let now = of(traffic, 1);
            moves.push((now.state.position - at).length());
            if !going(now) || moves.len() > 200 {
                return (moves, now.clone());
            }
        }
    }

    /// Whether `npc` is still waiting in its gate, its wait not yet out.
    fn waiting_in_gate(npc: &Npc) -> bool {
        matches!(npc.mode, Mode::Emerging { .. })
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn a_flying_successor_takes_goal_0x15_and_glides_from_what_it_owes() {
        // No wait: its high level makes it goal 8 in the same dispatch
        // (@0x8dd81-0x8dd8a), and sub 0xa keeps a negative +0x34
        // (@0x8811e-0x8812c): it glides from what it owes, down to its cap.
        let mut traffic = with_lead_target(under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            ship.owed_burst = Some(12.0);
        }));
        think(&mut traffic);
        let leads = of(&traffic, 1);
        assert_eq!(leads.mode, Mode::JumpingIn { tick: 0 });
        assert_eq!(leads.goal, Goal::Idle);
        assert_eq!(leads.emerges_from, None, "+0x6c -1 (@0x8dd93)");
        assert_eq!(
            leads.target, None,
            "+0x70 -1 in the same dispatch (@0x8dd7b)"
        );
        let (moves, after) = moves_while(&mut traffic, |npc| {
            matches!(npc.mode, Mode::JumpingIn { .. })
        });
        for (tick, moved) in (0u16..).zip(&moves) {
            let speed = JUMP_IN_SLOWING.mul_add(-f32::from(tick), 12.0);
            assert!(close(*moved, speed), "tick {tick}: {moved} for {speed}");
        }
        let cap = TopSpeeds::own_of(&after).burst_cap();
        let last = moves.last().copied().unwrap_or_default();
        assert!(last > cap && last - JUMP_IN_SLOWING <= cap, "{moves:?}");
        assert!(matches!(after.mode, Mode::Coasting { .. }), "no lead");
        assert_eq!((after.goal, after.owed_burst), (Goal::Idle, None));
    }

    #[test]
    fn a_flying_successor_owing_nothing_glides_from_the_top() {
        // Sub 0xa writes -50.0 over a +0x34 at or above 0 (@0x8811e-0x8812c).
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |_| {});
        let (moves, _) = moves_while(&mut traffic, |_| false);
        assert!(close(moves[0], glide_speed(0)), "{moves:?}");
    }

    #[test]
    fn by_the_other_reading_of_the_held_burst_a_successor_owes_nothing_and_glides_from_the_top() {
        // Its glide cut short owes none by the other reading, so nothing
        // stands in +0x34 for sub 0xa to keep.
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            ship.mode = Mode::JumpingIn { tick: 3 };
            ship.leaves_arrival(RuleSource::Bible);
        });
        assert_eq!(of(&traffic, 1).owed_burst, None);
        let world = World {
            held_burst: RuleSource::Bible,
            ..World::new(&[])
        };
        let at = of(&traffic, 1).state.position;
        traffic.tick_in(&Keep, world, &[], &mut NeverFires);
        let moved = (of(&traffic, 1).state.position - at).length();
        assert!(close(moved, glide_speed(0)), "{moved}");
    }

    #[test]
    fn a_successor_owing_sub_0xds_word_ends_its_glide_at_once() {
        // -1.0 (@0x85c53): one tick at 1.0, the next below any cap.
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            ship.owed_burst = Some(FOLLOW_BURST);
        });
        let (moves, after) = moves_while(&mut traffic, |_| false);
        assert!(close(moves[0], FOLLOW_BURST), "{moves:?}");
        assert!(
            matches!(after.mode, Mode::Coasting { .. }),
            "{:?}",
            after.mode
        );
        assert_eq!(after.owed_burst, None);
    }

    /// A wait set on a ship.
    type Wait = fn(&mut Npc);

    /// The ways a successor runs a wait as it takes goal 0x15: flying in its
    /// AI wait, or holding its course.
    fn under_a_wait() -> [(&'static str, Wait); 2] {
        [
            ("flying", |ship| ship.ai_wait = 5),
            ("coasting", |ship| {
                ship.mode = Mode::Coasting { ticks_left: 5 }
            }),
        ]
    }

    /// A successor far from its lead's gate, drifting right, under the wait
    /// `wait` sets, owing `owed`, its trigger held, once it has taken over.
    fn taken_over_under(wait: Wait, owed: Option<f32>) -> Traffic {
        let mut traffic = with_lead_target(under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            wait(ship);
            ship.owed_burst = owed;
            ship.state.position = Vec2::new(5000.0, 0.0);
            ship.state.velocity = Vec2::new(1.0, 0.0);
            ship.trigger.primary = true;
        }));
        think(&mut traffic);
        traffic
    }

    #[test]
    fn a_successor_flying_or_coasting_under_a_wait_hides_where_it_is_and_holds_the_gate_open() {
        // Goal 0x15 copied over a +0x4c above 0 (@0x893c8-0x893d0): its high
        // level returns at the wait gate, so it keeps the goal while the
        // wait runs, hidden (@0x972d), holding its lead's gate (+0x6c,
        // @0x893bd-0x893c2) open from wherever it is (@0x2e8a3-0x2e8d7).
        for (name, wait) in under_a_wait() {
            let traffic = taken_over_under(wait, None);
            let leads = of(&traffic, 1);
            assert_eq!(leads.mode, Mode::Emerging { ticks_left: 5 }, "{name}");
            assert_eq!(leads.ai_wait, 0, "{name}: its wait is the mode's");
            assert_eq!(leads.goal, Goal::Idle, "{name}");
            assert!(leads.emerging(), "{name}: unseen");
            assert_eq!(leads.emerges_from, LEADS_GATE, "{name}");
            assert!(leads.holds_gate_open(StellarId(311)), "{name}");
            assert_eq!(leads.trigger, Trigger::default(), "{name}");
            assert_eq!(leads.target, LEADS_TARGET, "{name}: until its turn-out");
            assert_eq!(leads.leader, None, "{name}");
        }
    }

    #[test]
    fn a_successor_under_a_wait_drifts_through_it_then_bursts_from_the_top() {
        // At its turn-out goal 0x15's block and sub 0xa (@0x8dd61-0x8dd93,
        // @0x8811e-0x8812c): from 50, not the emerge's 30 or 15.
        for (name, wait) in under_a_wait() {
            let mut traffic = taken_over_under(wait, None);
            let (drifts, out) = moves_while(&mut traffic, waiting_in_gate);
            assert!(
                drifts.iter().all(|&moved| close(moved, 1.0)),
                "{name}: {drifts:?}"
            );
            assert_eq!(out.mode, Mode::LeavingGate { tick: 0 }, "{name}");
            assert_eq!(out.target, LEADS_TARGET, "{name}: before the turn-out");
            let (moves, bursting) = moves_while(&mut traffic, |_| false);
            assert!(close(moves[0], JUMP_IN_SPEED), "{name}: {moves:?}");
            assert_eq!(bursting.target, None, "{name}: the turn-out drops it");
            assert_eq!(bursting.mode, Mode::LeavingGate { tick: 1 }, "{name}");
            let (moves, after) = moves_while(&mut traffic, |npc| {
                matches!(npc.mode, Mode::LeavingGate { .. })
            });
            assert!(close(moves[0], JUMP_IN_SPEED - JUMP_IN_SLOWING), "{name}");
            assert!(matches!(after.mode, Mode::Coasting { .. }), "{name}");
            assert_eq!(after.owed_burst, None, "{name}");
        }
    }

    #[test]
    fn a_successor_under_a_wait_owing_a_burst_bursts_from_what_it_owes() {
        for (name, wait) in under_a_wait() {
            let mut traffic = taken_over_under(wait, Some(12.0));
            moves_while(&mut traffic, waiting_in_gate);
            let (moves, _) = moves_while(&mut traffic, |_| false);
            assert!(close(moves[0], 12.0), "{name}: {moves:?}");
        }
    }

    #[test]
    fn a_successor_the_player_holds_under_its_wait_owes_its_own_word_as_the_hold_ends() {
        // Goal 4 over its goal 0x15 (@0x89d3e): no sub 0xa has run, so its
        // +0x34 is what it owed, or nothing; not the emerge's 30 or 15.
        for (name, wait) in under_a_wait() {
            for owed in [None, Some(12.0)] {
                for (burst, after) in [(RuleSource::Engine, owed), (RuleSource::Bible, None)] {
                    let mut traffic = taken_over_under(wait, owed);
                    of_mut(&mut traffic, 1).hold_at_gate();
                    let world = World {
                        held_burst: burst,
                        ..World::new(&[])
                    };
                    for _ in 0..10 {
                        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
                    }
                    let held = of(&traffic, 1);
                    assert_eq!(held.mode, Mode::Flying, "{name} {owed:?} {burst:?}");
                    assert_eq!(held.owed_burst, after, "{name} {owed:?} {burst:?}");
                }
            }
        }
    }

    #[test]
    fn a_successor_flying_gliding_or_bursting_drops_its_leads_target_and_one_in_its_gate_keeps_it()
    {
        // +0x70 = -1 at the turn-out (@0x8dd7b), the successor's first
        // dispatch with +0x4c at or below 0: at once for a ship flying with
        // no wait, gliding or bursting (goal 8, its +0x50 -1.0 by the
        // check, @0x8937c), at its own turn-out for one in its gate.
        for mode in [
            Mode::Flying,
            Mode::JumpingIn { tick: 3 },
            Mode::LeavingGate { tick: 2 },
        ] {
            let mut traffic =
                with_lead_target(under_a_lead_in(LEAD_WAITING, FEDS, |ship| ship.mode = mode));
            think(&mut traffic);
            assert_eq!(of(&traffic, 1).target, None, "{mode:?}");
        }
        let mut traffic = with_lead_target(under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            in_its_gate(ship, Mode::Emerging { ticks_left: 2 });
        }));
        think(&mut traffic);
        assert_eq!(of(&traffic, 1).target, LEADS_TARGET, "in its gate");
        let (_, out) = moves_while(&mut traffic, waiting_in_gate);
        assert_eq!(out.target, LEADS_TARGET, "to its turn-out");
        think(&mut traffic);
        assert_eq!(of(&traffic, 1).target, None, "at it");
    }

    #[test]
    fn a_successor_gliding_or_bursting_under_a_lead_in_its_gate_flies_on() {
        // Goal 0x15 turns back into goal 8 at once (@0x8dd8a), at -999.0
        // (@0x8f3d2), its +0x34 kept.
        for mode in [Mode::JumpingIn { tick: 3 }, Mode::LeavingGate { tick: 2 }] {
            let mut traffic =
                with_lead_target(under_a_lead_in(LEAD_WAITING, FEDS, |ship| ship.mode = mode));
            think(&mut traffic);
            let leads = of(&traffic, 1);
            assert_eq!(leads.mode, mode);
            assert_eq!(leads.goal, Goal::Idle, "{mode:?}");
            assert_eq!(leads.owed_burst, None, "{mode:?}");
            assert_eq!(leads.target, None, "{mode:?}: @0x8dd7b");
        }
    }

    #[test]
    fn a_glide_from_its_own_word_cut_short_owes_the_rest_of_that_word() {
        // The -1.0 over -999.0 (@0x8937c) leaves +0x34 where the glide had
        // it, from the word sub 0xa kept.
        let mut traffic = under_a_lead_in(LEAD_WAITING, FEDS, |ship| {
            ship.owed_burst = Some(12.0);
        });
        traffic.tick_in(&Keep, World::new(&[]), &[], &mut NeverFires);
        let mut gliding = of(&traffic, 1).clone();
        assert_eq!(gliding.mode, Mode::JumpingIn { tick: 1 });
        gliding.leaves_arrival(RuleSource::Engine);
        assert_eq!(gliding.mode, Mode::Flying);
        let owed = gliding.owed_burst.expect("owed");
        assert!(close(owed, 12.0 - JUMP_IN_SLOWING), "{owed}");
        let mut traffic = taken_over_under(under_a_wait()[0].1, Some(12.0));
        moves_while(&mut traffic, waiting_in_gate);
        moves_while(&mut traffic, |_| false);
        moves_while(&mut traffic, |_| false);
        let mut bursting = of(&traffic, 1).clone();
        assert_eq!(bursting.mode, Mode::LeavingGate { tick: 2 });
        bursting.leaves_arrival(RuleSource::Engine);
        let owed = bursting.owed_burst.expect("owed");
        assert!(close(owed, 12.0 - 2.0 * JUMP_IN_SLOWING), "{owed}");
    }

    #[test]
    fn a_successor_of_a_lead_paused_on_sub_0xd_follows_the_jump_its_lead_followed() {
        // The original's sub 0xd reads ship -1 (@0x859df-0x85c5f), outside
        // the ship table; the engine keeps the copied pause and follow.
        let jump = Some(WarpGoal::Follow(ShipRef::Npc(NpcId(7))));
        let paused = Goal::Regroup {
            kept: Kept::FollowsJump,
        };
        let mut traffic = taking_over(REBELS, paused);
        of_mut(&mut traffic, 0).warp.goal = jump;
        think(&mut traffic);
        let taker = of(&traffic, 1);
        assert_eq!(taker.goal, paused);
        assert_eq!(taker.warp.goal, jump);
        assert_eq!(taker.leader, None);
    }

    #[test]
    fn a_successor_gliding_or_bursting_under_any_other_lead_flies_at_once_owing_its_burst() {
        // @0x8937c's -1.0 over -999.0 ends the glide path (@0x8fcf5), and
        // the sub written ends sub 0xa, its +0x34 left negative.
        for (mode, speed) in [
            (Mode::JumpingIn { tick: 3 }, glide_speed(3)),
            (Mode::LeavingGate { tick: 2 }, exit_speed(false, 2)),
        ] {
            for (lead_mode, govt) in [
                (Mode::Flying, FEDS),
                (Mode::Flying, REBELS),
                (LEAD_WAITING, REBELS),
                (Mode::JumpingIn { tick: 1 }, FEDS),
            ] {
                let setup = |ship: &mut Npc| ship.mode = mode;
                let mut traffic = under_a_lead_in(lead_mode, govt, setup);
                think_bursting(&mut traffic, RuleSource::Engine);
                let leads = of(&traffic, 1);
                assert_eq!(leads.mode, Mode::Flying, "{mode:?} {lead_mode:?}");
                assert_eq!(leads.owed_burst, Some(speed), "{mode:?} {lead_mode:?}");
                let mut traffic = under_a_lead_in(lead_mode, govt, setup);
                think_bursting(&mut traffic, RuleSource::Bible);
                let leads = of(&traffic, 1);
                assert_eq!(leads.mode, Mode::Flying, "{mode:?} {lead_mode:?}");
                assert_eq!(leads.owed_burst, None, "{mode:?} {lead_mode:?}");
            }
        }
        let mut traffic = under_a_lead_in(Mode::Flying, FEDS, |ship| {
            ship.mode = Mode::LeavingGate { tick: 2 };
            ship.led_out = true;
        });
        think(&mut traffic);
        assert_eq!(of(&traffic, 1).owed_burst, Some(exit_speed(true, 2)));
    }

    #[test]
    fn a_disabled_follower_gliding_in_left_alone_ends_its_glide() {
        // @0x894ec's -1.0 over -999.0, as a successor's.
        for (burst, owed) in [
            (RuleSource::Engine, Some(glide_speed(3))),
            (RuleSource::Bible, None),
        ] {
            let mut traffic = under_a_lead_in(Mode::Flying, FEDS, |ship| {
                ship.condition = DISABLED;
                ship.mode = Mode::JumpingIn { tick: 3 };
            });
            think_bursting(&mut traffic, burst);
            let alone = of(&traffic, 1);
            assert_eq!(alone.mode, Mode::Flying, "{burst:?}");
            assert_eq!(alone.owed_burst, owed, "{burst:?}");
            assert_eq!(alone.goal, Goal::Regroup { kept: Kept::Coast }, "{burst:?}");
        }
    }

    #[test]
    fn a_leads_glide_or_burst_is_taken_over_as_goal_0_its_target_kept() {
        // Goal 8 copied at +0x50 -1.0: `_AIDispatch` writes goal 0
        // (@0x8fda0-0x8fdaa), and the high level sub 0 (@0x8fb09-0x8fb13).
        for lead_mode in [Mode::JumpingIn { tick: 3 }, Mode::LeavingGate { tick: 2 }] {
            let mut traffic = under_a_lead_in(lead_mode, FEDS, |_| {});
            let lead = of_mut(&mut traffic, 0);
            lead.goal = Goal::Attack(ShipRef::Npc(NpcId(9)));
            lead.target = Some(ShipRef::Npc(NpcId(9)));
            think(&mut traffic);
            let leads = of(&traffic, 1);
            assert_eq!(leads.goal, Goal::Idle, "{lead_mode:?}");
            assert_eq!(leads.target, Some(ShipRef::Npc(NpcId(9))), "{lead_mode:?}");
            assert_eq!(leads.mode, Mode::Flying, "{lead_mode:?}");
        }
    }

    #[test]
    fn a_lead_the_player_held_at_its_gate_writes_its_goal_4_over_its_successors_0x15() {
        let held = Mode::HeldAtGate { ticks_left: 9 };
        let mut traffic = under_a_lead_in(held, FEDS, |ship| {
            in_its_gate(ship, Mode::Emerging { ticks_left: 30 });
        });
        let lead = of_mut(&mut traffic, 0);
        lead.goal = Goal::Attack(PLAYER);
        lead.target = Some(PLAYER);
        think(&mut traffic);
        let leads = of(&traffic, 1);
        assert_eq!(leads.mode, Mode::HeldAtGate { ticks_left: 30 });
        assert_eq!(leads.emerges_from, None);
        assert_eq!(leads.goal, Goal::Attack(PLAYER));
        assert_eq!(leads.target, Some(PLAYER));
    }

    #[test]
    fn under_another_government_a_successor_in_its_gate_or_glide_maps_goal_0x15_or_8() {
        // @0x893ea-0x8945f read its own word, none of 5, 7, 9, 0xa, 0xb or
        // 0xc, whatever goal the engine keeps beside it: goal 4 on a held
        // target, else goal 0. It never copies the lead's 0x15.
        for (target, goal) in [(None, Goal::Idle), (Some(PLAYER), Goal::Attack(PLAYER))] {
            for (mode, left) in [
                (
                    Mode::Emerging { ticks_left: 30 },
                    Mode::HeldAtGate { ticks_left: 30 },
                ),
                (Mode::JumpingIn { tick: 3 }, Mode::Flying),
            ] {
                let mut traffic = under_a_lead_in(LEAD_WAITING, REBELS, |ship| {
                    ship.mode = mode;
                    ship.emerges_from = ship.emerging().then_some(StellarId(310));
                    ship.target = target;
                });
                think(&mut traffic);
                let leads = of(&traffic, 1);
                assert_eq!(leads.goal, goal, "{mode:?}");
                assert_eq!(leads.mode, left, "{target:?}");
                assert_eq!(leads.emerges_from, None, "{mode:?}");
                assert_eq!(leads.target, target, "{mode:?}");
            }
        }
    }

    #[test]
    fn a_leads_jump_with_its_own_lead_is_taken_over_as_an_own_jump() {
        // Goal 0xb and sub 0xd copied with parent -1: the goal's block takes
        // goal 2, sub 4, target and travel stellar -1 (@0x8de7c → @0x8df54).
        // Its -1.0 and goal are written over any arrival it was in.
        let taken_from = |mode: Mode, burst: RuleSource| {
            let mut traffic = taking_over(
                REBELS,
                Goal::Follow {
                    lead: NpcId(7),
                    guard: None,
                },
            );
            of_mut(&mut traffic, 0).warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(NpcId(7))));
            let taker = of_mut(&mut traffic, 1);
            taker.mode = mode;
            taker.emerges_from = taker.emerging().then_some(StellarId(310));
            think_bursting(&mut traffic, burst);
            of(&traffic, 1).clone()
        };
        let held = |ticks_left| Mode::HeldAtGate { ticks_left };
        for burst in RuleSource::ALL {
            let cut_short = |speed| (burst == RuleSource::Engine).then_some(speed);
            for (mode, left, owed) in [
                (Mode::Flying, Mode::Flying, None),
                (Mode::Emerging { ticks_left: 30 }, held(30), None),
                (Mode::LeavingGate { tick: 0 }, held(1), None),
                (
                    Mode::JumpingIn { tick: 3 },
                    Mode::Flying,
                    cut_short(glide_speed(3)),
                ),
                (
                    Mode::LeavingGate { tick: 2 },
                    Mode::Flying,
                    cut_short(exit_speed(false, 2)),
                ),
            ] {
                let taker = taken_from(mode, burst);
                assert_eq!(taker.mode, left, "{mode:?} {burst:?}");
                assert_eq!(taker.emerges_from, None, "{mode:?} {burst:?}");
                assert_eq!(taker.owed_burst, owed, "{mode:?} {burst:?}");
                assert_eq!(taker.goal, Goal::JumpOut, "{mode:?} {burst:?}");
                assert_eq!(taker.warp.goal, Some(WarpGoal::Own), "{mode:?} {burst:?}");
                assert_eq!(taker.target, None, "{mode:?} {burst:?}");
                assert!(!taker.travel_set, "{mode:?} {burst:?}");
                assert_eq!(taker.pursuit, Pursuit::default(), "{mode:?} {burst:?}");
                assert_eq!(taker.docking, None, "{mode:?} {burst:?}");
                assert_eq!(taker.leader, None, "{mode:?} {burst:?}");
            }
        }
    }

    #[test]
    fn a_successor_waiting_in_its_gate_leaves_it_for_the_goal_it_takes() {
        // Under a lead not in goal 0x15, @0x893d0 and @0x89429-0x8945f each
        // write another goal over its own.
        for (name, govt, goal) in [
            ("the lead's", FEDS, Goal::Hold),
            ("its own", REBELS, Goal::Idle),
        ] {
            let mut waiting = follower(govt, 300);
            waiting.mode = Mode::Emerging { ticks_left: 30 };
            waiting.emerges_from = Some(StellarId(310));
            let mut traffic = fleet(FEDS, DISABLED, vec![waiting]);
            of_mut(&mut traffic, 0).goal = Goal::Hold;
            think(&mut traffic);
            let leads = of(&traffic, 1);
            assert_eq!(leads.leader, None, "{name}");
            assert_eq!(leads.goal, goal, "{name}");
            assert_eq!(leads.mode, Mode::HeldAtGate { ticks_left: 30 }, "{name}");
            assert_eq!(leads.emerges_from, None, "{name}");
        }
    }

    #[test]
    fn a_disabled_follower_alone_in_its_gate_is_left_seen_and_held() {
        let mut alone = follower(FEDS, 300);
        alone.condition = DISABLED;
        alone.mode = Mode::Emerging { ticks_left: 30 };
        alone.emerges_from = Some(StellarId(310));
        let mut traffic = fleet(FEDS, DISABLED, vec![alone]);
        think(&mut traffic);
        let alone = of(&traffic, 1);
        assert_eq!(alone.goal, Goal::Regroup { kept: Kept::Coast });
        assert_eq!(alone.mode, Mode::HeldAtGate { ticks_left: 30 });
        assert_eq!(alone.emerges_from, None);
        assert_eq!(alone.leader, None);
    }

    #[test]
    fn a_follower_at_rest_takes_no_pause_and_forms_on_the_successor_at_once() {
        // An escort rests at +0x50 0: the hold undoes `_EscortAI`'s -1.0 in
        // the same dispatch (@0x8d513), so the strict test takes none.
        let mut traffic = fleet(
            FEDS,
            DISABLED,
            vec![follower(FEDS, 300), follower(FEDS, 100)],
        );
        think(&mut traffic);
        assert_eq!(
            of(&traffic, 2).goal,
            Goal::Follow {
                lead: NpcId(1),
                guard: None
            }
        );
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
    fn a_successor_of_or_in_goal_0xb_written_under_a_wait_takes_goal_2_sub_4() {
        // The goal word reads 0xb whatever substate stands (@0x8de7c →
        // @0x8df54, @0x8940c-0x89427).
        let traffic = taking_over(REBELS, Goal::Formation { guard: None });
        let mut lead = of(&traffic, 0).clone();
        lead.warp.goal = None;
        lead.warp.joining = true;
        let mut taker = of(&traffic, 1).clone();
        taker.take_over(Some(&lead), &World::new(&[]));
        assert_eq!(
            (taker.goal, taker.warp.goal),
            (Goal::JumpOut, Some(WarpGoal::Own)),
            "of"
        );
        let mut own = of(&traffic, 1).clone();
        own.goal = Goal::Formation { guard: None };
        own.warp.goal = None;
        own.warp.joining = true;
        own.take_over(None, &World::new(&[]));
        assert_eq!(
            (own.goal, own.warp.goal),
            (Goal::JumpOut, Some(WarpGoal::Own)),
            "in"
        );
    }

    #[test]
    fn a_follower_that_gave_its_stage_up_pauses_keeping_its_follow_of_the_successors_jump() {
        // Goal 0x13 with sub 0xd kept (@0x8949d-0x894a4): the jump path
        // then runs on it every frame.
        let mut gave_up = follower(FEDS, 100);
        gave_up.warp.goal = Some(WarpGoal::Follow(ShipRef::Npc(LEAD)));
        gave_up.warp.stage = -1.0;
        let mut traffic = fleet(FEDS, DISABLED, vec![follower(FEDS, 300), gave_up]);
        think(&mut traffic);
        let gave_up = of(&traffic, 2);
        assert_eq!(
            gave_up.goal,
            Goal::Regroup {
                kept: Kept::FollowsJump
            }
        );
        assert_eq!(
            gave_up.warp.goal,
            Some(WarpGoal::Follow(ShipRef::Npc(NpcId(1))))
        );
        assert!(!gave_up.jumping(), "`_AIIsShipJumping` reads goal 0x13");
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
        assert_eq!(alone.warp.stage, GIVEN_UP_STAGE);
    }

    // A disabled follower alone, on a jump.

    /// A disabled lead and its one follower, disabled in `goal` on `warp`,
    /// holding the player as its target, its stage given up: the disabled
    /// cancel's row (@0x8fce9), sub 4 or 0xd staying on the jump path.
    fn alone_on_a_jump(goal: Goal, warp: WarpGoal) -> Traffic {
        let mut alone = follower(FEDS, 300);
        alone.condition = DISABLED;
        alone.goal = goal;
        alone.target = Some(PLAYER);
        alone.warp.goal = Some(warp);
        alone.warp.stage = GIVEN_UP_STAGE;
        fleet(FEDS, DISABLED, vec![alone])
    }

    /// Each jump a follower can be on as it is left alone: its own and its
    /// lead's, under its goal or kept in the pause.
    fn jumps() -> [(Goal, WarpGoal); 4] {
        let leads = WarpGoal::Follow(ShipRef::Npc(LEAD));
        [
            (Goal::JumpOut, WarpGoal::Own),
            (
                Goal::Follow {
                    lead: LEAD,
                    guard: None,
                },
                leads,
            ),
            (
                Goal::Regroup {
                    kept: Kept::OwnJump,
                },
                WarpGoal::Own,
            ),
            (
                Goal::Regroup {
                    kept: Kept::FollowsJump,
                },
                leads,
            ),
        ]
    }

    /// `traffic`'s think half by the engine, rolling on `chance`.
    fn think_on(traffic: &mut Traffic, chance: &mut Draws) {
        traffic.think_holding(&Keep, World::new(&[]), &[], chance);
    }

    #[test]
    fn a_disabled_follower_alone_leaves_its_jump_for_sub_0() {
        // The sub 0 write (@0x894e3) ends sub 4 or 0xd, so the jump path.
        for (goal, warp) in jumps() {
            let mut traffic = alone_on_a_jump(goal, warp);
            think(&mut traffic);
            let alone = of(&traffic, 1);
            assert_eq!(alone.goal, Goal::Regroup { kept: Kept::Coast }, "{goal:?}");
            assert_eq!(alone.warp.goal, None, "{goal:?}");
            assert_eq!(alone.warp.stage, GIVEN_UP_STAGE, "{goal:?}");
            assert_eq!(alone.leader, None, "{goal:?}");
            assert_eq!(alone.target, Some(PLAYER), "{goal:?}: kept");
            assert!(!dispatch_gate::takes_jump_path(alone), "{goal:?}");
            assert!(dispatch_gate::reaches_think(alone), "{goal:?}");
        }
    }

    #[test]
    fn a_disabled_follower_alone_off_its_jump_draws_its_release_disabled_and_once_whole() {
        // The draw (@0x8fe73) comes before the disabled reset and tests no
        // condition, so it runs from the frame the check writes the pause.
        for (goal, warp) in jumps() {
            let mut traffic = alone_on_a_jump(goal, warp);
            let mut disabled = Draws::of(&[]).keyed(RELEASE_DRAW, 99);
            think_on(&mut traffic, &mut disabled);
            assert_eq!(disabled.times_asked(RELEASE_DRAW), 1, "{goal:?}");
            of_mut(&mut traffic, 1).condition = Condition::Intact;
            let mut held = Draws::of(&[]).keyed(RELEASE_DRAW, 99);
            think_on(&mut traffic, &mut held);
            assert_eq!(held.times_asked(RELEASE_DRAW), 1, "{goal:?}");
            let paused = Goal::Regroup { kept: Kept::Coast };
            assert_eq!(of(&traffic, 1).goal, paused, "{goal:?}");
            let mut released = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
            think_on(&mut traffic, &mut released);
            assert_eq!(released.times_asked(RELEASE_DRAW), 1, "{goal:?}");
            let free = of(&traffic, 1);
            assert_eq!(free.goal, Goal::Idle, "{goal:?}");
            assert_eq!(free.target, Some(PLAYER), "{goal:?}: whole, no reset");
        }
    }

    #[test]
    fn a_disabled_follower_alone_released_on_its_first_draw_takes_the_disabled_reset() {
        // Released, the dispatch goes on to the reset (@0x8fec4-0x8ff13).
        for (goal, warp) in jumps() {
            let mut traffic = alone_on_a_jump(goal, warp);
            let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
            think_on(&mut traffic, &mut chance);
            let reset = of(&traffic, 1);
            assert_eq!(reset.goal, Goal::Idle, "{goal:?}");
            assert_eq!(reset.target, None, "{goal:?}");
        }
    }

    #[test]
    fn a_follower_left_alone_coasts_once_whole_off_its_lost_leads_velocity() {
        let at = Vec2::new(300.0, 400.0);
        let moving = Vec2::new(0.0, 1.0);
        for (goal, warp) in jumps() {
            let mut traffic = alone_on_a_jump(goal, warp);
            of_mut(&mut traffic, 0).state.velocity = Vec2::new(2.0, 0.0);
            let alone = of_mut(&mut traffic, 1);
            alone.state.position = at;
            alone.state.velocity = moving;
            alone.state.heading = 30.0;
            think(&mut traffic);
            of_mut(&mut traffic, 1).condition = Condition::Intact;
            traffic.tick_in(&Keep, World::new(&[]), &[], &mut NeverFires);
            let state = of(&traffic, 1).state;
            assert_eq!(state.velocity, moving, "{goal:?}");
            assert_eq!(state.heading, 30.0, "{goal:?}");
            assert_eq!(state.position, at + moving, "{goal:?}");
        }
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
