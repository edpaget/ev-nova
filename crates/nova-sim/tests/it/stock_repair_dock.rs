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

use crate::common::{opened, player, stock_npc};

use nova_sim::combat::ShipRef;
use nova_sim::combat::armament::Arsenal;
use nova_sim::geometry::Vec2;
use nova_sim::govt::Governments;
use nova_sim::hail::Help;
use nova_sim::traffic::npc::Npc;
use nova_sim::traffic::plunder::PirateEvent;
use nova_sim::traffic::plunder_dock::DockSub;
use nova_sim::traffic::repair_dock::repair_box;
use nova_sim::traffic::{Traffic, World};
use nova_sim::{Condition, Goal, GovtId, NovaAi, Rulebook, ShipId};
use nova_sim_base::fixture::Draws;

const FEDERATION: GovtId = GovtId(128);
const SHUTTLE: ShipId = ShipId(128);

#[test]
fn a_stock_ship_repairing_the_disabled_player_docks_boards_repairs_and_drifts() {
    let Some(data) = opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let govts = Governments::read(&data);
    let ai = NovaAi::from_rulebook(&Rulebook::default());
    let mut shuttle = stock_npc(
        &data,
        &arsenal,
        1,
        (SHUTTLE, FEDERATION),
        Vec2::new(0.0, 600.0),
    );
    shuttle.goal = Goal::Assist(Help::Repair);
    shuttle.target = Some(ShipRef::Player);
    let side = repair_box(shuttle.hull.class_turn) as f32;
    assert!(side > 0.0, "{side}");
    let mut traffic = Traffic::with_interval(1);
    let helper = traffic.add_npc(shuttle);
    let ship = |traffic: &Traffic| -> Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == helper)
            .expect("here")
            .clone()
    };
    let world = |condition: Condition| World {
        govts: &govts,
        player: Some(player(condition)),
        ..World::new(&[])
    };
    let mut events = Vec::new();
    let mut closed = false;
    let mut docked_from: Option<Vec2> = None;
    for _ in 0..2000 {
        let before = ship(&traffic).state.position;
        traffic.tick_in(
            &ai,
            world(Condition::Disabled),
            &[],
            &mut Draws::of(&[0; 256]),
        );
        let after = ship(&traffic);
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
    assert!(closed, "it closed through sub 0xb");
    let from = docked_from.expect("it docked");
    assert!(
        from.x.abs() <= side && from.y.abs() <= side,
        "sub 0xf from {from:?}, within {side}"
    );
    assert_eq!(
        events,
        [
            PirateEvent::PlayerDockedWith { pirate: helper },
            PirateEvent::PlayerRepaired {
                helper,
                ship: SHUTTLE
            },
        ]
    );
    assert_eq!(ship(&traffic).goal, Goal::Drift);
    for _ in 0..50 {
        traffic.tick_in(
            &ai,
            world(Condition::Intact),
            &[],
            &mut Draws::of(&[0; 256]),
        );
    }
    assert_eq!(ship(&traffic).goal, Goal::Drift, "still drifting");
}
