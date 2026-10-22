//! The galaxy map's info panels over the stock data
//! (`nova_sim::map_panel`, `_MapDialogUpdate`). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! - Sol (`sÿst` 130), charted: its ports are Earth, Mars and Europa, as
//!   Jupiter (`Flags` 0x20) and the wormhole (0x21) are uninhabited; its
//!   hazards are "Sparse asteroid field" (raw `Asteroids` 2); its
//!   government is "Federation" ("Federation;hates Temmin Shard" cut at
//!   its last ';'); it trades all six commodities and has Trading,
//!   Outfitting and the Shipyard.
//! - Sol at level 1: the goods and services read "<Unknown>".
//! - Nesre Primus (132) lists its hypergate among its ports.

use crate::common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::map_panel::{ColumnBody, Listing, MapPanel, Ports, Services, Strip};
use nova_sim::market;
use nova_sim::{CommCatalog, Pilot, PilotCatalog, Session, SystemId};

const SOL: SystemId = SystemId(130);
const NESRE_PRIMUS: SystemId = SystemId(132);

/// A new stock pilot, changed by `change`, flying with the stock strings.
fn flown(data: &Rc<GameData>, change: impl FnOnce(&mut Pilot)) -> Session {
    let mut pilot = Pilot::new(data.as_ref(), "").expect("the stock first chär starts");
    change(&mut pilot);
    Session::fly(data.as_ref(), pilot)
        .expect("flies")
        .with_strings(Rc::clone(data) as Rc<dyn CommCatalog>)
}

fn opened() -> Option<Rc<GameData>> {
    let dir = common::nova_data()?;
    Some(Rc::new(
        GameData::open(&dir, None).expect("the stock data opens"),
    ))
}

fn panel(session: &Session, data: &Rc<GameData>, system: SystemId) -> MapPanel {
    session.map_panel(data.as_ref(), Some(system), None)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|&item| item.to_owned()).collect()
}

fn ports(panel: &MapPanel) -> &Ports {
    match &panel.strip {
        Strip::Info { ports, .. } => ports,
        Strip::HypergatePrompt { .. } => panic!("the prompt"),
    }
}

#[test]
fn charted_sol_lists_its_ports_hazards_government_goods_and_services() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |pilot| {
        pilot.chart(SOL);
    });
    let panel = panel(&session, &data, SOL);
    assert_eq!(
        *ports(&panel),
        Ports::Listed(strings(&["Earth", "Mars", "Europa"]))
    );
    let Strip::Info { hazards, .. } = &panel.strip else {
        panic!("the info strip");
    };
    assert_eq!(hazards, "Sparse asteroid field");
    let ColumnBody::Inhabited {
        name,
        government,
        goods,
        services,
        ..
    } = &panel.column.body
    else {
        panic!("inhabited: {:?}", panel.column.body);
    };
    assert_eq!((name.as_str(), government.as_str()), ("Sol", "Federation"));
    let commodities: Vec<String> = market::commodities(&data.commodity_strings())
        .into_iter()
        .map(|(_, commodity)| commodity.name)
        .collect();
    assert_eq!(commodities.len(), 6);
    assert_eq!(*goods, Listing::Items(commodities));
    assert_eq!(
        *services,
        Services::Items(strings(&["Trading", "Outfitting", "Shipyard"]))
    );
    assert_eq!(panel.words.ports, "Ports:");
    assert_eq!(panel.words.unknown, "<Unknown>");
}

#[test]
fn sol_at_level_1_reads_its_goods_and_services_unknown() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |pilot| {
        pilot.explore(SOL);
    });
    let panel = panel(&session, &data, SOL);
    let ColumnBody::Inhabited {
        goods, services, ..
    } = &panel.column.body
    else {
        panic!("inhabited: {:?}", panel.column.body);
    };
    assert_eq!(
        (goods, services),
        (&Listing::Unknown, &Services::Unknown { any: true })
    );
    assert_eq!(
        *ports(&panel),
        Ports::Listed(strings(&["Earth", "Mars", "Europa"]))
    );
}

#[test]
fn nesre_primus_lists_its_hypergate_among_its_ports() {
    let Some(data) = opened() else {
        return;
    };
    let session = flown(&data, |pilot| {
        pilot.explore(NESRE_PRIMUS);
    });
    assert_eq!(
        *ports(&panel(&session, &data, NESRE_PRIMUS)),
        Ports::Listed(strings(&[
            "New Babylon",
            "Spacedock IV",
            "Georgia",
            "HG-Primus"
        ]))
    );
}
