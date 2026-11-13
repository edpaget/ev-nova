//! A stock ship docking with the disabled player to repair it
//! (`_HighLevelAIHandler`'s goal 0xf @0x8f6d4-0x8f82f and the repair half
//! of `_LowLevelAIHandler`'s substate 0xf @0x86d4f-0x86f37; see
//! `nova_sim::traffic::repair_dock`). Skips, passing, when `NOVA_DATA` is
//! unset.
//!
//! - A stock Federation Shuttle asked to repair the disabled player closes
//!   on it (sub 0xb), docks within its class's repair box (sub 0xf),
//!   boards the player, repairs it once its dock's wait is done, and
//!   drifts off, keeping its drift through its type AI's think.
//! - A stock Raven, inertialess by its hull, asked the same closes on it
//!   and docks from outside the class's box, within
//!   [`INERTIALESS_BOX`] times it (@0x8f781-0x8f797), the same repair and
//!   drift following.

use crate::common::{opened, player, stock_npc};

use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::govt::Governments;
use nova_sim::hail::Help;
use nova_sim::traffic::npc::{Npc, NpcId};
use nova_sim::traffic::plunder::PirateEvent;
use nova_sim::traffic::plunder_dock::{DockSub, INERTIALESS_BOX};
use nova_sim::traffic::repair_dock::repair_box;
use nova_sim::traffic::{Traffic, World};
use nova_sim::{Condition, Goal, GovtId, NovaAi, Rulebook, ShipId};
use nova_sim_base::fixture::Draws;

const FEDERATION: GovtId = GovtId(128);
const SHUTTLE: ShipId = ShipId(128);
/// Inertialess by its hull (`Flags2` 0x0040).
const RAVEN: ShipId = ShipId(164);

/// What a repair flown to its end showed.
struct Repair {
    /// The ship's own id in `traffic`.
    helper: NpcId,
    /// The traffic after it, to keep flying.
    traffic: Traffic,
    /// Whether it closed through sub 0xb.
    closed: bool,
    /// Where it took sub 0xf from.
    docked_from: Option<Vec2>,
    /// The pirate events it raised.
    events: Vec<PirateEvent>,
}

fn ship_of(traffic: &Traffic, helper: NpcId) -> Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == helper)
        .expect("here")
        .clone()
}

/// `repairer` set on the disabled player and flown for at most `ticks`,
/// until it has docked and repaired.
fn fly_repair(mut repairer: Npc, govts: &Governments, ai: &NovaAi, ticks: u32) -> Repair {
    repairer.goal = Goal::Assist(Help::Repair);
    repairer.target = Some(ShipRef::Player);
    let mut traffic = Traffic::with_interval(1);
    let helper = traffic.add_npc(repairer);
    let mut events = Vec::new();
    let mut closed = false;
    let mut docked_from: Option<Vec2> = None;
    for _ in 0..ticks {
        let before = ship_of(&traffic, helper).state.position;
        let world = World {
            govts,
            player: Some(player(Condition::Disabled)),
            ..World::new(&[])
        };
        traffic.tick_in(ai, world, &[], &mut Draws::of(&[0; 256]));
        let after = ship_of(&traffic, helper);
        match after.docking.map(|docking| docking.sub) {
            Some(DockSub::Close) => closed = true,
            Some(DockSub::Dock) if docked_from.is_none() => docked_from = Some(before),
            _ => {}
        }
        events.extend(traffic.take_pirate_events());
        if events.len() >= 2 {
            break;
        }
    }
    Repair {
        helper,
        traffic,
        closed,
        docked_from,
        events,
    }
}

#[test]
fn a_stock_ship_repairing_the_disabled_player_docks_boards_repairs_and_drifts() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let shuttle = stock_npc(
        &data,
        &arsenal,
        1,
        (SHUTTLE, FEDERATION),
        Vec2::new(0.0, 600.0),
    );
    let side = repair_box(shuttle.hull.class_turn) as f32;
    assert!(side > 0.0, "{side}");
    let mut repair = fly_repair(shuttle, &govts, &ai, 2000);
    let helper = repair.helper;
    assert!(repair.closed, "it closed through sub 0xb");
    let from = repair.docked_from.expect("it docked");
    assert!(
        from.x.abs() <= side && from.y.abs() <= side,
        "sub 0xf from {from:?}, within {side}"
    );
    assert_eq!(
        repair.events,
        [
            PirateEvent::PlayerDockedWith { pirate: helper },
            PirateEvent::PlayerRepaired {
                helper,
                ship: SHUTTLE
            },
        ]
    );
    assert_eq!(ship_of(&repair.traffic, helper).goal, Goal::Drift);
    for _ in 0..50 {
        let world = World {
            govts: &govts,
            player: Some(player(Condition::Intact)),
            ..World::new(&[])
        };
        repair
            .traffic
            .tick_in(&ai, world, &[], &mut Draws::of(&[0; 256]));
    }
    assert_eq!(
        ship_of(&repair.traffic, helper).goal,
        Goal::Drift,
        "still drifting"
    );
}

#[test]
fn a_stock_inertialess_ship_repairing_the_disabled_player_docks_from_four_times_its_box() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let mut raven = stock_npc(&data, &arsenal, 1, (RAVEN, FEDERATION), Vec2::ZERO);
    assert!(raven.stats.inertialess_hull);
    let side = repair_box(raven.hull.class_turn) as f32;
    assert!(side > 0.0, "{side}");
    let wide = INERTIALESS_BOX as f32 * side;
    raven.state.position = Vec2::new(0.0, wide + 200.0);
    let repair = fly_repair(raven, &govts, &ai, 10_000);
    let helper = repair.helper;
    assert!(repair.closed, "it closed through sub 0xb");
    let from = repair.docked_from.expect("it docked");
    assert!(
        from.x.abs() <= wide && from.y.abs() <= wide,
        "sub 0xf from {from:?}, within {wide}"
    );
    assert!(from.y.abs() > side, "sub 0xf from {from:?}, past {side}");
    assert_eq!(
        repair.events,
        [
            PirateEvent::PlayerDockedWith { pirate: helper },
            PirateEvent::PlayerRepaired {
                helper,
                ship: RAVEN
            },
        ]
    );
    assert_eq!(ship_of(&repair.traffic, helper).goal, Goal::Drift);
}
