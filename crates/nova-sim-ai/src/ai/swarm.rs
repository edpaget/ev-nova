//! Swarming: a ship of a `shïp` `Flags2` 0x0001 class ([`SWARMS`]) names
//! a swarm leader (the original's +0xc906, [`Npc::swarm_leader`]) and,
//! outside the dogfight box, flies at a point ahead of it, firing its
//! missiles (`_AIVerifySwarmLeader` @0x7e2fe-0x7e39d,
//! `_AIFindSwarmLeader` @0x7e39e-0x7e46d, `_AIDispatch` @0x8ff18-0x8ff35,
//! and `_LowLevelAIHandler`'s substate 0x12, @0x85247-0x85361, in the
//! `EV Nova` executable; see [`pursuit`](crate::ai::pursuit)).
//!
//! - **The pass** ([`pass`]): each tick, each ship the dispatch lets
//!   through ([`verifies`]) on its AI timer verifies its swarm leader
//!   ([`verify`]) and, failing that, finds one ([`find`]), before its type
//!   AI. Neither reads another ship's swarm leader, so the order the ships
//!   are met in changes nothing.
//! - **Verify** ([`verify`]): a class that doesn't swarm keeps whatever it
//!   holds (@0x7e316-0x7e31e). A swarmer keeps berth L while 0 < L < its
//!   own berth (@0x7e327-0x7e333), a ship is at L (+0xb8, @0x7e343), L's
//!   class swarms (@0x7e356), L has the same target, none matching none
//!   (+0x70, @0x7e360-0x7e368), and L shares its government, not
//!   independent, or its leader, not none ([`shares`],
//!   @0x7e36a-0x7e390).
//! - **Find** ([`find`]): none, then the lowest berth from 1 below its own
//!   whose ship passes the same tests (@0x7e3af-0x7e45b). It doesn't test
//!   whether the candidate is disabled, and never names the player.
//! - **Who verifies** ([`verifies`], the paths of `_AIDispatch` from
//!   @0x8fb52 to @0x8ff26): not an escape pod (@0x8fb62), not disabled or
//!   breaking up (`_IsDisabled` @0x8fece), not below -900 on its jump
//!   state (gliding in, [`Mode::JumpingIn`], or bursting out of a gate,
//!   [`Mode::LeavingGate`] past tick 0, both -999.0, @0x8fcf5), and not
//!   in goal 0x15 (waiting in its gate, [`Mode::Emerging`], or its wait
//!   just out, [`Mode::LeavingGate`] at tick 0, @0x8ff18): a ship flying,
//!   coasting, entering a gate or held at its gate verifies. The check sits
//!   behind the AI timer's stagger (@0x8fe36-0x8fe6f), so a ship off its
//!   timer keeps its swarm leader unchecked. It also skips a ship in
//!   substate 4 (its warp out) or 0xd (an escort jumping with its
//!   leader, @0x8fd83-0x8fd9a), which here is no ship: one departs as it
//!   reaches sub 4, and sub 0xd is rdm
//!   phase escort-fleet-fidelity/phase-8-jump-with-leader's.
//! - **The swarm lead** ([`lead`]): where a swarmer aims, its swarm
//!   leader's position plus [`SWARM_AHEAD`] times its own `_ShipMaxSpeed`
//!   along the leader's whole-degree heading (`_Accel`, @0x8526e-0x852c4),
//!   and whether the swarm leader is its own leader, when the high level
//!   approaches instead (@0x8e66b).
//! - **The afterburner's test** ([`named`], `_AIHasAfterburner`
//!   @0x64ff-0x6539): a ship spawned at a berth another ship still names
//!   as its swarm leader has no afterburner.
//! - **The formation parent** ([`formation_parent`], `_DoPlayGameWork`
//!   @0x44160-0x441c0): a ship with a leader (the original's AI type 5 or
//!   6) attacking with a swarm leader forms on it, any other on its leader.
//! - Nothing here plays a sound.
//!
//! # Not modelled
//!
//! - `_DrawStatusTarg`'s debug readout, which calls Find on the player's
//!   target (@0x4b20c, behind the debug flag 0x3b5144 @0x4ae4b): the engine
//!   has no debug mode.
//! - `_AIDoSwarming` (@0x80250-0x802a4), a copy of the high level's test
//!   that nothing calls.
//! - The mission escort's +0xc906 of 0 (@0x430da): no reader tells it from
//!   -1, as every swarm test needs one above 0 and the parent it gives is
//!   its own leader.
//! - The swarm leader's slot formation for an escort or fighter, the
//!   `_AIMaintainFormation` calls and glow copies in substates 5, 0x11 and
//!   7, the stale parent of a ship whose AI type `_AICheckParentExists`
//!   dropped, goal 0x13's skip of the check, and a leftover substate 0x12
//!   outside state 4: rdm phase
//!   combat-ai-fidelity/phase-22-left-out-of-phase-14.

use crate::ai::Surroundings;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
#[cfg(doc)]
use crate::combat::hull::SWARMS;
use crate::geometry::Vec2;
use crate::traffic::npc::{Mode, Npc, NpcId};
use crate::traffic::pod_flight;
use crate::trig;

/// How many of its top speeds ahead of its swarm leader a swarmer aims
/// (15.0 @0xdda30).
pub const SWARM_AHEAD: f32 = 15.0;

/// Whether `npc` reaches `_AIDispatch`'s swarm check on a tick its AI
/// timer is due (see the module docs).
#[must_use]
pub fn verifies(npc: &Npc) -> bool {
    !pod_flight::is_pod(npc)
        && npc.condition == Condition::Intact
        && !matches!(
            npc.mode,
            Mode::JumpingIn { .. } | Mode::Emerging { .. } | Mode::LeavingGate { .. }
        )
}

/// Whether `other` passes the swarm leader's tests for `npc`: its class
/// swarms, it has `npc`'s target, and it shares its government or leader
/// (@0x7e356-0x7e390, @0x7e400-0x7e447).
fn suits(npc: &Npc, other: &Npc) -> bool {
    other.hull.swarms && other.target == npc.target && shares(npc, other)
}

/// Whether `npc` and `other` share a government that is not independent
/// (+0x98 not -1) or a leader (+0x9a not -1): a carrier for its fighters,
/// the player for its escorts.
fn shares(npc: &Npc, other: &Npc) -> bool {
    (npc.govt.is_some() && npc.govt == other.govt)
        || (npc.leader_ref().is_some() && npc.leader_ref() == other.leader_ref())
}

/// The ship at `berth` among `npcs`, if any.
fn at_berth(npcs: &[Npc], berth: u32) -> Option<&Npc> {
    npcs.iter().find(|npc| npc.berth == berth)
}

/// Whether `npc` keeps the swarm leader it names among `npcs`
/// (`_AIVerifySwarmLeader`, see the module docs).
#[must_use]
pub fn verify(npc: &Npc, npcs: &[Npc]) -> bool {
    if !npc.hull.swarms {
        return true;
    }
    npc.swarm_leader
        .filter(|&berth| berth > 0 && berth < npc.berth)
        .and_then(|berth| at_berth(npcs, berth))
        .is_some_and(|leader| suits(npc, leader))
}

/// The swarm leader `npc` finds among `npcs` (`_AIFindSwarmLeader`, see
/// the module docs): the lowest berth below its own whose ship suits.
#[must_use]
pub fn find(npc: &Npc, npcs: &[Npc]) -> Option<u32> {
    npcs.iter()
        .filter(|other| other.berth > 0 && other.berth < npc.berth && suits(npc, other))
        .map(|other| other.berth)
        .min()
}

/// The swarm check of each of `npcs` that [`verifies`] and `thinks` on its
/// AI timer (see the module docs): it keeps its swarm leader, or finds
/// another.
pub fn pass(npcs: &mut [Npc], thinks: impl Fn(&Npc) -> bool) {
    for index in 0..npcs.len() {
        let npc = &npcs[index];
        if !thinks(npc) || !verifies(npc) || verify(npc, npcs) {
            continue;
        }
        npcs[index].swarm_leader = find(&npcs[index], npcs);
    }
}

/// A swarmer's swarm leader as its high level and substate 0x12 read it
/// ([`lead`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwarmLead {
    /// The point it aims at, [`SWARM_AHEAD`] top speeds ahead of its swarm
    /// leader.
    pub aim: Vec2,
    /// Whether its swarm leader is its own leader (+0xc906 == +0x9a).
    pub leads_it: bool,
}

/// `npc`'s swarm lead among `around` (see the module docs): none with no
/// swarm leader, or none at its berth.
#[must_use]
pub fn lead(npc: &Npc, around: &Surroundings) -> Option<SwarmLead> {
    let leader = at_berth(around.npcs, npc.swarm_leader?)?;
    let ahead = npc.top_speed(around.led_speed) * SWARM_AHEAD;
    Some(SwarmLead {
        aim: leader.state.position + trig::accel(trig::whole(leader.state.heading), ahead),
        leads_it: npc.leader_ref() == Some(ShipRef::Npc(leader.id)),
    })
}

/// Whether a ship of `npcs` other than the one at `berth` names `berth`
/// as its swarm leader (`_AIHasAfterburner` @0x64ff-0x6539): never berth
/// 0, the player's.
#[must_use]
pub fn named(berth: u32, npcs: &[Npc]) -> bool {
    berth != 0
        && npcs
            .iter()
            .any(|npc| npc.berth != berth && npc.swarm_leader == Some(berth))
}

/// The ship a led ship forms on (the original's +0xc92e).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormationParent {
    /// The player.
    Player,
    /// The ship at this berth: its swarm leader.
    Berth(u32),
    /// Its NPC leader ([`Npc::leader`]), whose berth the formation of any
    /// NPC parent will read (rdm phase
    /// escort-fleet-fidelity/phase-6-npc-formations).
    Leader(NpcId),
}

/// The ship `npc` forms on (`_DoPlayGameWork` @0x44160-0x441c0): its
/// swarm leader while it attacks or snipes (`_IsShipAttacking`, state 4)
/// with a target, else its leader; none with no leader, as the
/// original's AI types 1-4 are left out (@0x440d8).
#[must_use]
pub fn formation_parent(npc: &Npc) -> Option<FormationParent> {
    let leader = npc.leader_ref()?;
    let attacking = npc.goal.attacking().is_some() && npc.target.is_some();
    Some(match (attacking, npc.swarm_leader) {
        (true, Some(berth)) => FormationParent::Berth(berth),
        _ => match leader {
            ShipRef::Player => FormationParent::Player,
            ShipRef::Npc(id) => FormationParent::Leader(id),
        },
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::catalog::GovtId;
    use crate::combat::hull::Condition;
    use crate::flight::ShipState;
    use crate::stats::ShipStats;
    use crate::traffic::npc::Mode;
    use crate::trig;
    use nova_sim_base::fixture::FAST;

    const FEDS: Option<GovtId> = Some(GovtId(128));

    /// A hired escort's duty, keeping formation.
    fn duty() -> crate::escort::EscortDuty {
        crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        }
    }
    const AT_PLAYER: Goal = Goal::Attack(ShipRef::Player);

    /// NPC `id` at berth `berth`, of a swarming class, of the Federation,
    /// attacking the player.
    fn swarmer(id: u32, berth: u32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.berth = berth;
        npc.hull.swarms = true;
        npc.govt = FEDS;
        npc.goal = AT_PLAYER;
        npc.target = Some(ShipRef::Player);
        npc
    }

    /// The swarm leaders of `npcs` after one pass with every ship's AI
    /// timer due.
    fn passed(npcs: &mut [Npc]) -> Vec<Option<u32>> {
        pass(npcs, |_| true);
        npcs.iter().map(|npc| npc.swarm_leader).collect()
    }

    #[test]
    fn the_named_values() {
        assert_eq!(SWARM_AHEAD, 15.0);
    }

    #[test]
    fn the_lowest_berth_swarmer_sharing_target_and_government_leads() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2), swarmer(2, 3)];
        assert_eq!(passed(&mut npcs), [None, Some(1), Some(1)]);
        // Met out of berth order: berths, not places, decide.
        let mut npcs = [swarmer(0, 3), swarmer(1, 2), swarmer(2, 1)];
        assert_eq!(passed(&mut npcs), [Some(1), Some(1), None]);
    }

    #[test]
    fn a_swarm_leader_is_kept_while_valid_even_when_a_lower_swarmer_appears() {
        let mut npcs = vec![swarmer(0, 2), swarmer(1, 3)];
        assert_eq!(passed(&mut npcs), [None, Some(2)]);
        npcs.push(swarmer(2, 1));
        // The new ship at berth 1 leads berth 2, but berth 3 keeps berth 2.
        assert_eq!(passed(&mut npcs), [Some(1), Some(2), None]);
    }

    /// Whether the swarmer at berth 3, naming berth 2, keeps it once
    /// `change` is made to the ship at berth 2; with none kept, it finds
    /// berth 1, of another government, or none.
    fn kept_after(change: impl Fn(&mut Npc)) -> Option<u32> {
        let mut other = swarmer(9, 1);
        other.govt = Some(GovtId(200));
        let mut lead = swarmer(1, 2);
        change(&mut lead);
        let mut ship = swarmer(2, 3);
        ship.swarm_leader = Some(2);
        let mut npcs = [other, lead, ship];
        passed(&mut npcs)[2]
    }

    #[test]
    fn a_swarm_leader_is_dropped_and_found_again_when() {
        assert_eq!(kept_after(|_| {}), Some(2), "valid");
        assert_eq!(kept_after(|lead| lead.berth = 7), None, "it leaves");
        assert_eq!(
            kept_after(|lead| lead.hull.swarms = false),
            None,
            "it stops swarming"
        );
        assert_eq!(
            kept_after(|lead| lead.target = Some(ShipRef::Npc(NpcId(5)))),
            None,
            "it takes another target"
        );
        assert_eq!(
            kept_after(|lead| lead.govt = Some(GovtId(201))),
            None,
            "it shares neither government nor leader"
        );
        // Its berth not below the ship's own: berth 4, above 3.
        let mut lead = swarmer(1, 4);
        lead.swarm_leader = None;
        let mut ship = swarmer(2, 3);
        ship.swarm_leader = Some(4);
        let mut npcs = [lead, ship];
        assert_eq!(passed(&mut npcs)[1], None);
        // Nor at the ship's own berth.
        let mut ship = swarmer(2, 3);
        ship.swarm_leader = Some(3);
        assert!(!verify(&ship, &[ship.clone()]));
        // Berth 0 is never valid.
        ship.swarm_leader = Some(0);
        assert!(!verify(&ship, &[ship.clone()]));
    }

    #[test]
    fn an_independent_ship_with_no_leader_never_shares() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        for npc in &mut npcs {
            npc.govt = None;
        }
        assert_eq!(passed(&mut npcs), [None, None]);
    }

    #[test]
    fn ships_of_one_leader_swarm_whatever_their_government() {
        // A carrier's fighters: their leader is their carrier.
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        npcs[0].govt = Some(GovtId(200));
        for npc in &mut npcs {
            npc.leader = Some(NpcId(7));
        }
        assert_eq!(passed(&mut npcs), [None, Some(1)]);
        // The player's escorts: their leader is the player.
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        npcs[0].govt = None;
        npcs[1].govt = None;
        for npc in &mut npcs {
            npc.escort = Some(duty());
        }
        assert_eq!(passed(&mut npcs), [None, Some(1)]);
        // Of two leaders, and two governments, none.
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        npcs[0].govt = Some(GovtId(200));
        npcs[0].leader = Some(NpcId(7));
        npcs[1].leader = Some(NpcId(8));
        assert_eq!(passed(&mut npcs), [None, None]);
    }

    #[test]
    fn two_ships_with_no_target_share_it() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        for npc in &mut npcs {
            npc.goal = Goal::Idle;
            npc.target = None;
        }
        assert_eq!(passed(&mut npcs), [None, Some(1)]);
    }

    #[test]
    fn a_disabled_candidate_still_leads() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        npcs[0].condition = Condition::Disabled;
        assert_eq!(passed(&mut npcs), [None, Some(1)]);
    }

    #[test]
    fn a_ship_whose_class_does_not_swarm_keeps_what_it_holds() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2)];
        npcs[1].hull.swarms = false;
        npcs[1].swarm_leader = Some(5);
        assert_eq!(passed(&mut npcs), [None, Some(5)]);
        npcs[1].swarm_leader = None;
        assert_eq!(passed(&mut npcs), [None, None], "nor finds one");
    }

    #[test]
    fn a_ship_the_dispatch_skips_keeps_its_swarm_leader_unchecked() {
        let skipped = |change: &dyn Fn(&mut Npc)| {
            let mut ship = swarmer(1, 2);
            ship.swarm_leader = Some(7);
            change(&mut ship);
            let mut npcs = [swarmer(0, 1), ship];
            passed(&mut npcs)[1]
        };
        assert_eq!(skipped(&|_| {}), Some(1), "checked");
        assert_eq!(
            skipped(&|ship| ship.condition = Condition::Disabled),
            Some(7),
            "disabled"
        );
        assert_eq!(
            skipped(&|ship| {
                ship.condition = Condition::Dying {
                    ticks_left: 10,
                    starting: false,
                };
            }),
            Some(7),
            "breaking up"
        );
        assert_eq!(
            skipped(&|ship| ship.mode = Mode::Emerging { ticks_left: 5 }),
            Some(7),
            "emerging from a gate (goal 0x15)"
        );
        assert_eq!(
            skipped(&|ship| ship.mode = Mode::LeavingGate { tick: 0 }),
            Some(7),
            "its wait just out, goal 0x15 still"
        );
        assert_eq!(
            skipped(&|ship| ship.mode = Mode::LeavingGate { tick: 1 }),
            Some(7),
            "bursting out, below -900"
        );
        assert_eq!(
            skipped(&|ship| ship.mode = Mode::JumpingIn { tick: 3 }),
            Some(7),
            "gliding in, below -900"
        );
        assert_eq!(
            skipped(&|ship| ship.ship = crate::combat::hull::ESCAPE_POD_SHIP),
            Some(7),
            "an escape pod"
        );
        for mode in [
            Mode::HeldAtGate { ticks_left: 5 },
            Mode::Coasting { ticks_left: 5 },
        ] {
            assert_eq!(skipped(&|ship| ship.mode = mode), Some(1), "{mode:?}");
        }
        // Off its AI timer it is not checked either.
        let mut ship = swarmer(1, 2);
        ship.swarm_leader = Some(7);
        let mut npcs = [swarmer(0, 1), ship];
        pass(&mut npcs, |npc| npc.berth != 2);
        assert_eq!(npcs[1].swarm_leader, Some(7));
    }

    /// The player's escort at `berth` attacking the player's target, NPC 5.
    fn escort(id: u32, berth: u32) -> Npc {
        let mut npc = swarmer(id, berth);
        npc.escort = Some(duty());
        npc.goal = Goal::Attack(ShipRef::Npc(NpcId(5)));
        npc.target = Some(ShipRef::Npc(NpcId(5)));
        npc
    }

    #[test]
    fn the_formation_parent_of_an_escort_attacking_with_a_swarm_leader_is_its_swarm_leader() {
        let mut npc = escort(1, 3);
        npc.swarm_leader = Some(2);
        assert_eq!(formation_parent(&npc), Some(FormationParent::Berth(2)));
        npc.goal = Goal::Snipe(ShipRef::Npc(NpcId(5)));
        assert_eq!(
            formation_parent(&npc),
            Some(FormationParent::Berth(2)),
            "sniping is attacking"
        );
        // Its leader otherwise.
        let not_attacking = Npc {
            goal: Goal::Idle,
            ..npc.clone()
        };
        assert_eq!(
            formation_parent(&not_attacking),
            Some(FormationParent::Player)
        );
        let no_target = Npc {
            target: None,
            ..npc.clone()
        };
        assert_eq!(formation_parent(&no_target), Some(FormationParent::Player));
        let no_swarm_leader = Npc {
            swarm_leader: None,
            ..npc.clone()
        };
        assert_eq!(
            formation_parent(&no_swarm_leader),
            Some(FormationParent::Player)
        );
        // A fleet escort and a carried fighter: their NPC leader.
        let mut fleet = swarmer(2, 4);
        fleet.leader = Some(NpcId(8));
        assert_eq!(
            formation_parent(&fleet),
            Some(FormationParent::Leader(NpcId(8)))
        );
        fleet.swarm_leader = Some(1);
        assert_eq!(formation_parent(&fleet), Some(FormationParent::Berth(1)));
        // A ship of AI type 1-4: no leader, so none.
        let mut traffic = swarmer(3, 5);
        traffic.swarm_leader = Some(1);
        assert_eq!(formation_parent(&traffic), None);
    }

    #[test]
    fn a_berth_another_ship_names_as_its_swarm_leader_is_named() {
        let mut npcs = [swarmer(0, 1), swarmer(1, 2), swarmer(2, 3)];
        assert!(!named(1, &npcs));
        npcs[2].swarm_leader = Some(1);
        assert!(named(1, &npcs));
        assert!(!named(2, &npcs));
        // The ship at the berth itself doesn't count.
        npcs[2].swarm_leader = Some(3);
        assert!(!named(3, &npcs));
        // Berth 0 is never named.
        npcs[2].swarm_leader = Some(0);
        assert!(!named(0, &npcs));
    }

    #[test]
    fn a_swarm_lead_aims_15_top_speeds_ahead_of_its_swarm_leader() {
        let mut leader = swarmer(0, 1);
        leader.state = ShipState {
            position: Vec2::new(100.0, -50.0),
            heading: 90.9,
            ..ShipState::default()
        };
        let mut ship = swarmer(1, 2);
        ship.swarm_leader = Some(1);
        let npcs = [leader.clone(), ship.clone()];
        let around = Surroundings::new(&[], &npcs);
        let top = ship.stats.handling.max_speed;
        let aim = leader.state.position + trig::accel(90, top * 15.0);
        assert_eq!(
            lead(&ship, &around),
            Some(SwarmLead {
                aim,
                leads_it: false
            })
        );
        // Its own leader too.
        ship.leader = Some(NpcId(0));
        assert_eq!(lead(&ship, &around).map(|lead| lead.leads_it), Some(true));
        // The player's escort aims by its led top speed.
        let mut escort = ship.clone();
        escort.leader = None;
        escort.escort = Some(duty());
        let led = Surroundings {
            led_speed: 1.5,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            lead(&escort, &led).map(|lead| lead.aim),
            Some(leader.state.position + trig::accel(90, top * 1.5 * 15.0))
        );
        // None with no swarm leader, or one gone.
        ship.swarm_leader = None;
        assert_eq!(lead(&ship, &around), None);
        ship.swarm_leader = Some(4);
        assert_eq!(lead(&ship, &around), None);
    }

    #[test]
    fn only_these_ships_reach_the_verify() {
        let ship = swarmer(0, 1);
        assert!(verifies(&ship));
        assert!(!verifies(&Npc {
            condition: Condition::Disabled,
            ..ship.clone()
        }));
        assert!(verifies(&Npc {
            mode: Mode::EnteringGate {
                gate: crate::catalog::StellarId(128),
                ticks_left: 3,
            },
            ..ship
        }));
    }
}
