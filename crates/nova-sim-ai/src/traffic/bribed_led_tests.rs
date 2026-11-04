//! A bribed fleet escort through the traffic and Nova's AI: the bribe
//! writes +0x88 = 1 and keeps the parent +0x9a (`_DoCommDialog`
//! @0x961f8), so `_AIDispatch` calls `_WimpyTraderAI` for it
//! (@0x8ff9b-0x8ffab, `_WimpyTraderAI` @0x8b029-0x8b201), not `_EscortAI`.
//! It keeps the hail's leave and its lead, takes no order word, flees and
//! calls for help when hit, isn't drawn onto its lead's jump, and keeps
//! its slot in its lead's formation, while an unbribed fellow escorts on.

use std::collections::BTreeSet;
use std::rc::Rc;

use crate::ai::{Behaviour, CommCall, Goal, NovaAi, Surroundings};
use crate::catalog::{EscortClass, GovtId, GovtRecord};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::fleet::OrderWord;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::stats::ShipStats;
use crate::traffic::fixture::calls_on_line;
use crate::traffic::npc::{AiType, Npc, NpcId};
use crate::traffic::warp_out::WarpGoal;
use crate::traffic::{Traffic, World};
use nova_sim_base::fixture::{Draws, FAST};
use nova_sim_catalog::fixture::govt;

/// The lead, the traffic's first ship.
const LEAD: NpcId = NpcId(0);
/// Its unbribed escort, the second.
const FELLOW: NpcId = NpcId(1);
/// Its bribed escort, the third.
const BRIBED: NpcId = NpcId(2);
/// A raider, the fourth.
const RAIDER: NpcId = NpcId(3);

/// The fleet's government and the raiders', at war.
fn govts() -> Governments {
    Governments::new([
        GovtRecord {
            enemies: [1, -1, -1, -1],
            classes: [0, -1, -1, -1],
            ..govt(128)
        },
        GovtRecord {
            enemies: [0, -1, -1, -1],
            classes: [1, -1, -1, -1],
            ..govt(129)
        },
    ])
}

/// Keeps every goal and target.
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

/// Nova's AI, the interceptors (the lead and the raider) keeping their
/// goals.
fn ai() -> NovaAi {
    NovaAi::default().with(AiType::Interceptor, Rc::new(Keep))
}

/// A ship of `govt` and `ai_type` at (`x`, `y`) with `goal`.
fn ship(govt: i16, ai_type: AiType, (x, y): (f32, f32), goal: Goal) -> Npc {
    let mut npc = crate::traffic::fixture::npc(0, ShipStats::new(FAST, &[]));
    npc.govt = Some(GovtId(govt));
    npc.ai_type = ai_type;
    npc.state.position = Vec2::new(x, y);
    npc.goal = goal;
    npc.target = goal.quarry();
    npc.hull.strength = 100.0;
    npc
}

/// A medium warship escort the lead leads, at (`x`, `y`).
fn escort((x, y): (f32, f32)) -> Npc {
    let mut npc = ship(128, AiType::Warship, (x, y), Goal::Idle);
    npc.leader = Some(LEAD);
    npc.class = EscortClass::Medium;
    npc
}

/// `npc` paid off by the hail: the bribe and its leave (`_DoCommDialog`
/// @0x961f8, `_AIMakeShipLeave`).
fn bribed(mut npc: Npc) -> Npc {
    npc.take_bribe();
    npc.goal = Goal::JumpOut;
    npc.target = None;
    npc
}

/// The traffic of `ships`, deciding every tick, added in turn at berths
/// 1, 2, ...
fn traffic(ships: impl IntoIterator<Item = Npc>) -> Traffic {
    let mut traffic = Traffic::with_interval(1);
    for npc in ships {
        traffic.add_npc(npc);
    }
    traffic
}

/// An idle lead at the centre, its fellow escort and its bribed one
/// beside it, and a raider of the other government far below.
fn idle_fleet() -> Traffic {
    traffic([
        ship(128, AiType::Interceptor, (0.0, 0.0), Goal::Idle),
        escort((40.0, 0.0)),
        bribed(escort((-40.0, 0.0))),
        ship(129, AiType::Interceptor, (0.0, 3000.0), Goal::Idle),
    ])
}

fn npc(traffic: &Traffic, id: NpcId) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("in the system")
}

/// `traffic` a tick on by [`ai`] on `frame`, answering `strikes`, every
/// ship kept in place. The lead, in berth 1, issues on frames that are
/// multiples of 8.
fn tick(traffic: &mut Traffic, frame: i16, strikes: &[crate::combat::Strike]) {
    let govts = govts();
    let world = World {
        govts: &govts,
        frame,
        ..World::new(&[])
    };
    let still: BTreeSet<NpcId> = traffic.npcs().iter().map(|npc| npc.id).collect();
    traffic.tick_holding(&ai(), world, strikes, &still, &mut Draws::of(&[0; 64]));
}

#[test]
fn a_bribed_escort_keeps_its_leave_and_its_lead_while_its_fellow_follows() {
    // `_WimpyTraderAI` keeps any goal but 0 unprovoked (@0x8b06c), and
    // `_AIIssueEscortOrders` skips it (@0x89b39).
    let mut traffic = idle_fleet();
    for frame in [0, 1, 8] {
        tick(&mut traffic, frame, &[]);
        let bribed = npc(&traffic, BRIBED);
        assert_eq!(bribed.goal, Goal::JumpOut, "frame {frame}: its leave");
        assert_eq!(bribed.leader, Some(LEAD), "frame {frame}: +0x9a kept");
        assert_eq!(bribed.order_word, OrderWord::NONE, "frame {frame}");
        assert!(!bribed.ordered_by_lead, "frame {frame}");
        let fellow = npc(&traffic, FELLOW);
        assert_eq!(fellow.order_word, OrderWord(3), "frame {frame}: Return");
        assert!(fellow.ordered_by_lead, "frame {frame}");
        assert!(
            matches!(fellow.goal, Goal::Follow { lead: LEAD, .. }),
            "frame {frame}: {:?}",
            fellow.goal
        );
    }
}

#[test]
fn a_bribed_escort_hit_flees_and_calls_for_help() {
    // Provoked with a target: goal 3 (@0x8b1d8) and `_AICallForHelp`
    // (@0x8b1f5); its +0x9a names an NPC, so not the formation on the
    // player (@0x8b1bd).
    let mut traffic = idle_fleet();
    tick(
        &mut traffic,
        1,
        &[strike(ShipRef::Npc(BRIBED), ShipRef::Npc(RAIDER))],
    );
    let bribed = npc(&traffic, BRIBED);
    assert_eq!(bribed.goal, Goal::Flee(ShipRef::Npc(RAIDER)));
    assert_eq!(bribed.leader, Some(LEAD));
    assert!(
        calls_on_line(&traffic.take_on_line())
            .iter()
            .any(|&(id, call, _)| id == BRIBED && call == CommCall::Distress),
        "it calls for help"
    );
}

#[test]
fn its_leads_jump_draws_its_fellow_not_the_bribed_ship() {
    // Goal 0xb is entered only by `_EscortAI` (@0x839fd-0x83a92).
    let mut lead = ship(128, AiType::Interceptor, (3000.0, 0.0), Goal::JumpOut);
    lead.state.velocity = Vec2::new(2.0, 0.0);
    lead.state.heading = 90.0;
    let mut traffic = traffic([
        lead,
        escort((3000.0, 40.0)),
        bribed(escort((3000.0, -40.0))),
    ]);
    let govts = govts();
    let mut world = World {
        govts: &govts,
        frame: 1,
        ..World::new(&[])
    };
    world.warp.durations = [364, 252];
    traffic.tick_in(&ai(), world, &[], &mut Draws::of(&[0; 64]));
    assert_eq!(
        npc(&traffic, FELLOW).warp.goal,
        Some(WarpGoal::Follow(ShipRef::Npc(LEAD)))
    );
    let bribed = npc(&traffic, BRIBED);
    assert_ne!(bribed.warp.goal, Some(WarpGoal::Follow(ShipRef::Npc(LEAD))));
    assert_eq!(bribed.goal, Goal::JumpOut, "its own leave");
}

#[test]
fn a_bribed_escort_keeps_its_slot_in_its_leads_formation() {
    // The marking pass skips it (@0x440d8), so its +0xc92e stays, and
    // `_AICalcFormationPositions` tests no AI type (@0x850bc, @0x85166).
    let mut traffic = traffic([
        ship(128, AiType::Interceptor, (0.0, 0.0), Goal::Hold),
        escort((40.0, 0.0)),
        escort((-40.0, 0.0)),
    ]);
    tick(&mut traffic, 1, &[]);
    let (parent, slots) = (
        npc(&traffic, BRIBED).form_parent,
        [
            npc(&traffic, FELLOW).fleet_slot,
            npc(&traffic, BRIBED).fleet_slot,
        ],
    );
    assert!(parent.is_some(), "its lead");
    assert!(slots.iter().all(Option::is_some), "both numbered");
    let spared = traffic
        .npcs_mut()
        .iter_mut()
        .find(|npc| npc.id == BRIBED)
        .expect("there");
    *spared = bribed(spared.clone());
    for frame in [2, 3] {
        tick(&mut traffic, frame, &[]);
        assert_eq!(npc(&traffic, BRIBED).form_parent, parent, "frame {frame}");
        assert_eq!(
            [
                npc(&traffic, FELLOW).fleet_slot,
                npc(&traffic, BRIBED).fleet_slot,
            ],
            slots,
            "frame {frame}"
        );
    }
}
