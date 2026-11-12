//! The pilot store's round trips that resume a pilot through [`Session`]:
//! they stay in the facade because the save schema and the pilot store live
//! in a crate below the session, which can't fly the pilot it opens.

#![allow(clippy::float_cmp)]

use crate::catalog::{GovtId, StellarId, SystemId};
use crate::fixture::MemoryPilots;
use crate::pilot::Pilot;
use crate::reserves::Gauge;
use crate::saves::PilotKeeper;
use crate::session::Session;
use crate::testkit::{catalog, edge_lander, jump, land_now};

fn keeper(store: &MemoryPilots) -> PilotKeeper<MemoryPilots> {
    PilotKeeper::new(store.clone())
}

/// A pilot named `name` that has jumped, landed and changed: a day
/// and a jump's fuel gone, 7 credits and a legal record, docked at
/// planet 140 at the edge of system 131.
fn travelled(name: &str) -> Pilot {
    let catalog = edge_lander();
    let mut session =
        Session::fly(&catalog, Pilot::new(&catalog, name).expect("starts")).expect("flies");
    jump(&mut session, &catalog, 132);
    land_now(&mut session).expect("lands");
    session.transact(|pilot| {
        pilot.set_cash(7);
        pilot.set_legal_record(GovtId(129), -3);
    });
    session.pilot().clone()
}

#[test]
fn a_saved_pilot_lists_and_opens_with_every_field_as_it_was() {
    let store = MemoryPilots::new();
    let pilot = travelled("Ada");
    assert_eq!(
        pilot.reserves().fuel,
        Gauge {
            now: 200.0,
            max: 300.0
        }
    );
    assert_eq!(pilot.stellar(), Some(StellarId(140)));
    assert_eq!(keeper(&store).save(&pilot), Ok(()));
    assert_eq!(store.writes(), 1);
    let reopened = keeper(&store);
    assert_eq!(reopened.list(), Ok(vec!["Ada".to_owned()]));
    let opened = reopened.open("Ada").expect("opens");
    assert_eq!(opened, pilot);
    assert_eq!(opened.reserves(), pilot.reserves());
    assert_eq!(opened.system(), SystemId(131));
    let resumed = Session::fly(&edge_lander(), opened).expect("flies");
    assert_eq!(crate::testkit::resumed_on(&resumed), Some(StellarId(140)));
    assert_eq!(resumed.reserves().fuel.now, 200.0);
}

#[test]
fn the_last_pilot_opens_the_pilot_its_record_names() {
    let store = MemoryPilots::new();
    let keeper = keeper(&store);
    let ada = travelled("Ada");
    keeper.save(&ada).expect("saves");
    keeper
        .save(&Pilot::new(&catalog(), "Bea").expect("starts"))
        .expect("saves");
    store.put_last("Ada");
    assert_eq!(keeper.open_last(), Some(ada));
    assert_eq!(store.last().as_deref(), Some("Ada"));
    assert_eq!(store.writes(), 2, "opening writes no pilot");
}
