//! The crons on the session (see [`cron`](crate::cron)).
//!
//! The pilot keeps each crön running or holding off, its `days` and
//! `holdoff`, keyed by ID. A pilot flown from a save naming a crön the
//! data no longer has forgets it, so it is inactive and the next save
//! leaves it out, as the original's `_LoadPilotData` forces a slot whose
//! crön is gone to inactive (@0x75fbc-0x76025).

#[cfg(test)]
mod tests {
    use crate::catalog::CronId;
    use crate::cron::CronState;
    use crate::pilot::Pilot;
    use crate::save;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog, cron};

    #[test]
    fn a_save_naming_a_missing_cron_flies_with_it_inactive() {
        let catalog = FakePilotCatalog {
            crons: vec![cron(128)],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let running = CronState {
            days: 3,
            holdoff: 0,
        };
        pilot.crons.insert(CronId(128), running);
        pilot.crons.insert(CronId(300), running);
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert_eq!(pilot.crons().len(), 2, "the save keeps it");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().crons().iter().collect::<Vec<_>>(),
            [(&CronId(128), &running)]
        );
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.crons().keys().collect::<Vec<_>>(), [&CronId(128)]);
    }
}
