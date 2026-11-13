//! Hyper Select over the stock data (`_HandlePlayer` @0x69c16-0x69cce; see
//! `Session::select_next_system`): Bis'valalo (231), the only stock system
//! with a repeated link, lists 237, 230, 230, 221, 232, 220, and each slot
//! is visited in turn. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{
    ApplyCollaborators, HailOptions, NavMode, NeverFires, NovaBoarding, NovaLaw, Pilot, Session,
    SystemId,
};

const BISVALALO: SystemId = SystemId(231);

/// A session of the stock first chär flying from Bis'valalo.
fn at_bisvalalo(data: &GameData) -> Session {
    let mut pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    pilot.system = BISVALALO;
    Session::fly(data, pilot).expect("flies")
}

/// Runs `command` on `session` as the adapter does.
fn apply(session: &mut Session, data: &GameData, command: FlightCommand) -> Answer {
    session.apply(
        command,
        &mut ApplyCollaborators {
            catalog: Some(data),
            chance: &mut NeverFires,
            law: &NovaLaw::default(),
            boarding_rule: &NovaBoarding::default(),
            hail_options: &HailOptions::default(),
        },
    )
}

#[test]
fn hyper_select_at_bisvalalo_visits_its_repeated_link_once_per_slot() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_bisvalalo(&data);
    for expected in [237, 230, 230, 221, 232, 220, 237] {
        apply(&mut session, &data, FlightCommand::SelectNextSystem);
        assert_eq!(
            session.nav_mode(),
            NavMode::Hyperspace(Some(SystemId(expected)))
        );
    }
}

#[test]
fn a_map_cycle_at_bisvalalo_steps_on_from_the_slot_hyper_select_noted() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = at_bisvalalo(&data);
    for _ in 0..3 {
        apply(&mut session, &data, FlightCommand::SelectNextSystem);
    }
    apply(&mut session, &data, FlightCommand::CycleMapSelection);
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(221))));
}
