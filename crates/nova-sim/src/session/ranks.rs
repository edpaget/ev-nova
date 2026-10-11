//! The ranks on the session (see [`rank`](crate::rank)).
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

#[cfg(test)]
mod tests {
    use crate::catalog::RankId;
    use crate::pilot::Pilot;
    use crate::save;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog, rank};

    #[test]
    fn flying_a_pilot_forgets_a_rank_the_data_no_longer_has() {
        let catalog = FakePilotCatalog {
            ranks: vec![rank(128)],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.extend([RankId(128), RankId(200)]);
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert_eq!(pilot.ranks().len(), 2, "the save keeps it");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().ranks().iter().collect::<Vec<_>>(),
            [&RankId(128)]
        );
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.ranks().iter().collect::<Vec<_>>(), [&RankId(128)]);
    }
}
