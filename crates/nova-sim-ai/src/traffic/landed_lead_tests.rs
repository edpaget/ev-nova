//! A fleet lead that lands, through the traffic and Nova's AI. Its
//! landing keeps its slot active (`_HighLevelAIHandler` @0x8dadc-0x8db5a,
//! [`land_flight::stay`](super::land_flight::stay)), so
//! `_AICheckParentExists` (@0x8926a, inactive or `_IsDisabled`
//! @0x89287-0x892a9) and `_EscortAI`'s lost parent (@0x83917-0x8397b) find
//! it present: no succession, and its escorts keep their lead and their
//! orders through its wait. On its order frames it orders them to Return
//! (`_AIIssueEscortOrders`' overrides @0x89a8d-0x89b0a): an escort keeps
//! formation on it and a fighter docks. The jump draw (@0x83a1a-0x83a5f)
//! takes no one while it waits, though its think writes goal 2, as the
//! wait gate (@0x8d4b9-0x8d4d2) holds its sub at the landing's 0; once its
//! wait is out it jumps from rest (sub 4) and draws them onto its jump.
//! Under [`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves)' other
//! reading a mission lead leaves as it lands, unrecorded, and its
//! heaviest escort succeeds it by the other government's branch. A lone
//! ship's stay is in [`landed_stay_tests`](super::landed_stay_tests).

use std::rc::Rc;

use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
use crate::bay::Carrier;
use crate::catalog::{EscortClass, GovtId, GovtRecord, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::fleet::OrderWord;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::rulebook::RuleSource;
use crate::stats::ShipStats;
use crate::traffic::ShipBehav;
use crate::traffic::autopilot::Outcome;
use crate::traffic::jump_with_leader::draws_followers;
use crate::traffic::land_flight;
use crate::traffic::npc::{AiType, MissionMark, Npc, NpcId};
use crate::traffic::warp_out::WarpGoal;
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::{govt, planet};

/// The lead, the traffic's first ship.
const LEAD: NpcId = NpcId(0);
/// Its heavier escort, the second.
const HEAVY: NpcId = NpcId(1);
/// Its lighter escort, the third.
const LIGHT: NpcId = NpcId(2);
/// Where the lead lands: stellar 140, beyond the leave's reach of the
/// centre, so it jumps from rest (sub 4) once its wait is out.
const AT: (f32, f32) = (1500.0, 0.0);

/// Lands on stellar 140 until it has visited a stellar, then leaves by
/// hyperspace, as a trader's idle block sends a ship that has landed
/// (`ai::idle_block::visited_here`).
#[derive(Debug)]
struct LandThenLeave;

impl Behaviour for LandThenLeave {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        if npc.visited.is_some() {
            Goal::JumpOut
        } else {
            Goal::Land(StellarId(140))
        }
    }
}

/// Nova's AI, the lead (a wimpy trader) landing then leaving.
fn ai() -> NovaAi {
    NovaAi::default().with(AiType::WimpyTrader, Rc::new(LandThenLeave))
}

/// The fleet's government.
fn govts() -> Governments {
    Governments::new([GovtRecord {
        classes: [0, -1, -1, -1],
        ..govt(128)
    }])
}

/// A ship of the fleet's government and `ai_type` at (`dx`, `dy`) off the
/// stellar, at rest.
fn ship(ai_type: AiType, (dx, dy): (f32, f32)) -> Npc {
    let mut npc = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    npc.govt = Some(GovtId(128));
    npc.ai_type = ai_type;
    npc.state.position = Vec2::new(AT.0 + dx, AT.1 + dy);
    npc.state.velocity = Vec2::ZERO;
    npc.hull.strength = 100.0;
    npc
}

/// A medium warship escort the lead leads, at (`dx`, `dy`) off the
/// stellar, of `mass`.
fn escort((dx, dy): (f32, f32), mass: i16) -> Npc {
    let mut npc = ship(AiType::Warship, (dx, dy));
    npc.leader = Some(LEAD);
    npc.class = EscortClass::Medium;
    npc.hull.signed_mass = mass;
    npc
}

/// The lead on the stellar, idle, and its heavy and light escorts beside
/// it, deciding every tick.
fn fleet() -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    traffic.add_npc(ship(AiType::WimpyTrader, (0.0, 0.0)));
    traffic.add_npc(escort((0.0, 60.0), 300));
    traffic.add_npc(escort((0.0, -60.0), 100));
    traffic
}

fn npc(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// `traffic` a tick on by [`ai`] on `frame`, stellar 140 its one site and
/// the warp sound 364 Mac ticks, the world as `edit` leaves it. The lead,
/// in berth 1, issues on frames that are multiples of 8.
fn tick(traffic: &mut Traffic, frame: i16, edit: impl FnOnce(&mut World)) {
    let govts = govts();
    let sites = [planet(140, AT.0, AT.1)];
    let mut world = World {
        govts: &govts,
        frame,
        ..World::new(&sites)
    };
    world.warp.durations = [364, 252];
    edit(&mut world);
    traffic.tick_in(&ai(), world, &[], &mut Draws::of(&[0; 64]));
}

/// Ticks `traffic` from frame `from` until the lead has landed, returning
/// the next frame.
fn land(traffic: &mut Traffic, from: i16) -> i16 {
    for frame in from..from + 50 {
        tick(traffic, frame, |_| {});
        if npc(traffic, LEAD).visited == Some(StellarId(140)) {
            return frame + 1;
        }
    }
    panic!("never landed");
}

#[test]
fn a_fleet_lead_that_lands_keeps_its_escorts_through_its_wait() {
    let mut traffic = fleet();
    let mut frame = land(&mut traffic, 0);
    assert!(npc(&traffic, LEAD).ai_wait > 0.0, "it waits");
    while npc(&traffic, LEAD).ai_wait > 0.0 {
        tick(&mut traffic, frame, |_| {});
        assert_eq!(traffic.departed(), [], "frame {frame}");
        let lead = npc(&traffic, LEAD);
        assert_eq!(lead.leader, None, "frame {frame}");
        // Its think writes goal 2 through the wait, and nothing puts it
        // on its jump's sub or stage: the draw takes no one (finding 4).
        assert!(!draws_followers(lead), "frame {frame}: {:?}", lead.warp);
        for id in [HEAVY, LIGHT] {
            let escort = npc(&traffic, id);
            assert_eq!(escort.leader, Some(LEAD), "frame {frame}: no succession");
            assert_eq!(escort.ai_type, AiType::Warship, "frame {frame}");
            assert!(
                matches!(escort.goal, Goal::Follow { lead: LEAD, .. }),
                "frame {frame}: {:?}",
                escort.goal
            );
            assert!(
                !escort.follows_leaders_jump() && !escort.joins_leaders_jump(),
                "frame {frame}: not on its lead's jump"
            );
            assert_eq!(escort.warp.goal, None, "frame {frame}");
        }
        frame += 1;
    }
    assert_eq!(npc(&traffic, LEAD).goal, Goal::JumpOut, "its think's leave");
}

#[test]
fn a_waiting_lead_orders_its_escorts_to_return() {
    // Landing on frame 7, its first order frame of the wait is 8, the
    // landing's goal 0 standing, then 16 under its think's goal 2: both
    // override the table to Return (@0x89a8d-0x89b0a).
    let mut traffic = fleet();
    for frame in 7..=16 {
        tick(&mut traffic, frame, |_| {});
        assert!(npc(&traffic, LEAD).visited.is_some(), "landed on frame 7");
        if frame % 8 == 0 {
            for id in [HEAVY, LIGHT] {
                let escort = npc(&traffic, id);
                assert_eq!(escort.order_word, OrderWord(3), "frame {frame}: Return");
                assert!(escort.ordered_by_lead, "frame {frame}");
            }
        }
    }
    assert!(npc(&traffic, LEAD).ai_wait > 0.0, "still waiting");
}

#[test]
fn a_landed_carrier_recalls_its_fighters() {
    let mut traffic = fleet();
    let mut fighter = ship(AiType::Warship, (0.0, -200.0));
    fighter.leader = Some(LEAD);
    fighter.carrier = Some(Carrier {
        ship: ShipRef::Npc(LEAD),
        window: 100.0,
        reach: 40.0,
    });
    let fighter_ship = fighter.ship;
    let id = traffic.add_npc(fighter);
    // Landing on frame 1, before its first order frame.
    let mut frame = land(&mut traffic, 1);
    assert_eq!(frame, 2);
    assert_eq!(
        npc(&traffic, id).order_word,
        OrderWord::NONE,
        "no order yet"
    );
    let mut docked = Vec::new();
    let mut recalled = false;
    while npc(&traffic, LEAD).ai_wait > 0.0 && docked.is_empty() {
        tick(&mut traffic, frame, |_| {});
        docked.extend(traffic.departed().iter().copied());
        if let Some(fighter) = traffic.npcs().iter().find(|npc| npc.id == id) {
            assert_eq!(fighter.leader, Some(LEAD), "frame {frame}");
            if fighter.goal == Goal::Dock(ShipRef::Npc(LEAD)) {
                assert!(frame >= 8, "frame {frame}: recalled by the wait's order");
                recalled = true;
            }
        }
        frame += 1;
    }
    assert_eq!(
        docked,
        [(
            id,
            Outcome::Docked {
                carrier: ShipRef::Npc(LEAD),
                ship: fighter_ship,
            }
        )],
        "it docks while its carrier waits"
    );
    assert!(recalled, "Return docks a fighter (@0x83e79-0x83ea3)");
    assert_eq!(
        npc(&traffic, LEAD).visited,
        Some(StellarId(140)),
        "it stays"
    );
}

#[test]
fn once_its_wait_is_out_the_lead_jumps_and_its_escorts_follow() {
    let mut traffic = fleet();
    let mut frame = land(&mut traffic, 0);
    while npc(&traffic, LEAD).ai_wait > 0.0 {
        tick(&mut traffic, frame, |_| {});
        frame += 1;
    }
    // At rest beyond the reach, goal 2 goes straight to sub 4.
    tick(&mut traffic, frame, |_| {});
    frame += 1;
    let lead = npc(&traffic, LEAD);
    assert_eq!(lead.warp.goal, Some(WarpGoal::Own), "sub 4");
    let travel = lead.travel;
    assert_eq!(travel, Some(StellarId(140)));
    // The escorts read its tick-start goal, a tick behind.
    for _ in 0..2 {
        tick(&mut traffic, frame, |_| {});
        frame += 1;
    }
    for id in [HEAVY, LIGHT] {
        let escort = npc(&traffic, id);
        assert_eq!(
            escort.warp.goal,
            Some(WarpGoal::Follow(ShipRef::Npc(LEAD))),
            "{id:?}: drawn onto its jump (goal 0xb, sub 0xd)"
        );
        assert_eq!(escort.travel, travel, "{id:?}: +0x6c copied");
    }
    let mut departed = Vec::new();
    for _ in 0..2000 {
        if traffic.npcs().is_empty() {
            break;
        }
        tick(&mut traffic, frame, |_| {});
        departed.extend(traffic.departed().iter().copied());
        frame = frame.wrapping_add(1);
    }
    departed.sort_by_key(|(id, _)| *id);
    assert_eq!(
        departed,
        [
            (LEAD, Outcome::JumpedOut),
            (HEAVY, Outcome::JumpedOut),
            (LIGHT, Outcome::JumpedOut)
        ]
    );
}

#[test]
fn succeed_leaders_finds_a_landed_lead_present() {
    let mut traffic = fleet();
    land_flight::stay(&mut traffic.npcs[0], StellarId(140), &mut Draws::of(&[]));
    for id in [HEAVY, LIGHT] {
        let escort = traffic
            .npcs
            .iter_mut()
            .find(|npc| npc.id == id)
            .expect("there");
        escort.goal = Goal::Follow {
            lead: LEAD,
            guard: None,
        };
    }
    let before = traffic.npcs.clone();
    let govts = govts();
    let world = World {
        govts: &govts,
        ..World::new(&[])
    };
    assert!(traffic.succeed_leaders(&world).is_empty());
    assert_eq!(
        traffic.npcs, before,
        "no follower's lead, goal or parent written"
    );
}

#[test]
fn under_the_other_reading_a_mission_lead_that_lands_is_lost() {
    let mut traffic = fleet();
    traffic.npcs[0].mission = Some(MissionMark {
        order: None,
        slot: 1,
        behav: ShipBehav::from_code(0),
        formation: None,
    });
    let bible = |world: &mut World| world.landed_leaves = RuleSource::Bible;
    for frame in 0..50 {
        tick(&mut traffic, frame, bible);
        if !traffic.departed().is_empty() {
            break;
        }
    }
    assert_eq!(
        traffic.departed(),
        [(LEAD, Outcome::Landed(StellarId(140)))],
        "it leaves as it lands"
    );
    let heavy = npc(&traffic, HEAVY).clone();
    assert!(heavy.goal.flies_in_form(), "{:?}", heavy.goal);
    // The next tick's check finds no record of it.
    let govts = govts();
    let mut world = World {
        govts: &govts,
        ..World::new(&[])
    };
    bible(&mut world);
    // The successor drops out of the formation marking; its fellow forms
    // on it.
    assert_eq!(
        traffic.succeed_leaders(&world),
        [HEAVY].into_iter().collect()
    );
    let successor = npc(&traffic, HEAVY);
    assert_eq!(successor.leader, None, "the heavier escort leads");
    // The other government's branch: its own formation goal mapped to
    // goal 0, its own travel stellar kept.
    assert_eq!(successor.goal, Goal::Idle);
    assert_eq!(successor.travel, heavy.travel);
    assert_eq!(npc(&traffic, LIGHT).leader, Some(HEAVY));
}
