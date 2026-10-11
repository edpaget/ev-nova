//! The crons on the session (see [`cron`](crate::cron)).
//!
//! The pilot keeps each crön running or holding off, its `days` and
//! `holdoff`, keyed by ID. A pilot flown from a save naming a crön the
//! data no longer has forgets it, so it is inactive and the next save
//! leaves it out, as the original's `_LoadPilotData` forces a slot whose
//! crön is gone to inactive (@0x75fbc-0x76025).
//!
//! **Contribute.** Every reader of the `Contribute` bits the player
//! holds sums them through one function, [`wares::contributed`]: the
//! mission offers, the bar's hires ([`Session::contributed`]), the
//! outfitter and the shipyard.

use crate::session::Session;
use crate::wares::{self, Contributors};

impl Session {
    /// The `Contribute` bits the player holds, as they stand
    /// (`_GetPlayerContributeBits` @0x76b2; see [`wares::contributed`]).
    pub(super) fn contributed(&self) -> u64 {
        let by = Contributors {
            ship: self.fields.contribute,
            outfits: &self.outfits,
            crons: &self.crons,
        };
        wares::contributed(&by, &self.pilot)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{CronId, CronRecord, MissionId, MissionRecord, StellarId};
    use crate::chance::NeverFires;
    use crate::cron::CronState;
    use crate::mission::Refusal;
    use crate::pilot::Pilot;
    use crate::rules::Verdict;
    use crate::save;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog, cron, land_now, mission};

    /// [`catalog`] with crön 128 contributing 0x4, and mission 130 at the
    /// computer requiring it.
    fn contributing() -> FakePilotCatalog {
        FakePilotCatalog {
            crons: vec![CronRecord {
                duration: 5,
                contribute: 0x4,
                ..cron(128)
            }],
            missions: vec![MissionRecord {
                require: 0x4,
                ..mission(130)
            }],
            ..catalog()
        }
    }

    /// A session of `catalog` landed on 128.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    /// Whether `session` refuses mission 130 for its `Require`.
    fn requires(session: &Session) -> bool {
        session
            .offer_refusals(MissionId(130))
            .expect("docked")
            .contains(&("Require", Verdict::Refuses(Refusal::Require)))
    }

    #[test]
    fn a_running_crons_contribute_unlocks_a_missions_require() {
        let catalog = contributing();
        let mut session = landed(&catalog);
        assert!(requires(&session));
        assert_eq!(session.contributed() & 0x4, 0);
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 0,
            },
        );
        assert!(!requires(&session));
        assert_eq!(session.contributed() & 0x4, 0x4);
        session.pilot.crons.insert(
            CronId(128),
            CronState {
                days: 5,
                holdoff: 2,
            },
        );
        assert!(requires(&session), "in its pre-holdoff");
    }

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
