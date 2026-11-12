//! The mission info screen over the stock data: which stock `mïsn` it
//! hides (`Flags` 0x0400) and which of those can be aborted, and the info
//! an abort hands back with a hidden one active, by each reading of
//! `mission_info_rebuild`. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::catalog::MissionRecord;
use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::mission::screens::HIDDEN;
use nova_sim::{
    ApplyCollaborators, HailOptions, InfoRow, MissionId, NeverFires, NovaBoarding, NovaLaw, Pilot,
    PilotCatalog, RuleKey, RuleSource, Rulebook, Session,
};

/// "Derelict Decoy": hidden, and abortable.
const DECOY: MissionId = MissionId(133);

/// The first two stock missions the screen shows that can be aborted.
fn shown_abortable(missions: &[MissionRecord]) -> [MissionId; 2] {
    let mut shown = missions
        .iter()
        .filter(|record| record.flags & HIDDEN == 0 && record.can_abort != 0)
        .map(|record| record.id);
    [
        shown.next().expect("a shown mission"),
        shown.next().expect("another"),
    ]
}

/// A new stock pilot holding `held`, a mission in each slot, flown by
/// `rules`.
fn holding(data: &GameData, held: &[(u8, MissionId)], rules: Rulebook) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["missions"] = held
        .iter()
        .map(|&(slot, mission)| {
            serde_json::json!({
                "slot": slot,
                "mission": mission.0,
                "destination": null,
                "return": null,
                "cargo_type": null,
                "cargo_tons": 0,
                "pay": 1000,
                "days_left": null,
                "deadline": null,
                "reached": false,
                "goals_met": false,
                "aboard": false,
                "failed": null,
                "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
                "aux": {"countdown": 0, "arrived": 0, "left": 0},
            })
        })
        .collect();
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, saved).expect("flies").with_rules(rules)
}

/// Aborts `slot` as the adapter does.
fn abort(session: &mut Session, data: &GameData, slot: u8) -> Answer {
    session.apply(
        FlightCommand::AbortMission(slot),
        &mut ApplyCollaborators {
            catalog: Some(data),
            chance: &mut NeverFires,
            law: &NovaLaw::default(),
            boarding_rule: &NovaBoarding::default(),
            hail_options: &HailOptions::default(),
        },
    )
}

fn slots(rows: &[InfoRow]) -> Vec<u8> {
    rows.iter().map(|row| row.slot).collect()
}

#[test]
fn stock_hides_88_missions_five_of_them_abortable() {
    let Some(data) = common::opened() else {
        return;
    };
    let missions = data.missions();
    assert_eq!(missions.len(), 791, "stock mïsn");
    let hidden: Vec<&MissionRecord> = missions
        .iter()
        .filter(|record| record.flags & HIDDEN != 0)
        .collect();
    assert_eq!(hidden.len(), 88, "Flags 0x0400");
    let abortable: Vec<MissionId> = hidden
        .iter()
        .filter(|record| record.can_abort != 0)
        .map(|record| record.id)
        .collect();
    assert_eq!(
        abortable,
        [
            DECOY,
            MissionId(609),
            MissionId(610),
            MissionId(896),
            MissionId(905)
        ],
        "Derelict Decoy, the two GOTCHA!! drop bears, Clean Fed Record and a Silent Mission"
    );
}

#[test]
fn an_abort_beside_a_hidden_stock_mission_lists_by_the_rule() {
    let Some(data) = common::opened() else {
        return;
    };
    let [first, second] = shown_abortable(&data.missions());
    let held = [(0, DECOY), (1, first), (2, second)];
    let engine = Rulebook::default().with_override(RuleKey::MissionInfoRebuild, RuleSource::Engine);

    let mut intended = holding(&data, &held, Rulebook::default());
    assert_eq!(slots(&intended.mission_info().rows), [1, 2], "133 unlisted");
    assert_eq!(abort(&mut intended, &data, 1), Answer::Aborted(Ok(())));
    let info = intended.mission_info_after_abort();
    assert_eq!(slots(&info.rows), [2]);
    assert_eq!(info.table, None, "each row picks the mission it names");

    let mut original = holding(&data, &held, engine);
    assert_eq!(abort(&mut original, &data, 1), Answer::Aborted(Ok(())));
    let info = original.mission_info_after_abort();
    assert_eq!(slots(&info.rows), [2], "the row names the second mission");
    let table = info.table.expect("the original's table");
    assert_eq!(
        slots(&table),
        [0, 2],
        "every active slot (@0x9e69f-0x9e6d8)"
    );
    assert!(
        table[0].abortable && table[0].name != info.rows[0].name,
        "its row picks Derelict Decoy, which its Abort would abort: {:?}",
        table[0]
    );
}
