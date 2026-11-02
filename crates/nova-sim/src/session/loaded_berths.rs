//! The berths a loaded pilot's escorts and fighters take: packed from
//! berth 1 up, in the order of the berths they held when the pilot was
//! saved.
//!
//! **The original.** `_WritePilotData` writes each escort and fighter into
//! tables indexed by the ship-table slot it holds (@0x725f6-0x72711), so
//! the slot number is the only order the file keeps. `_LoadPilotData`
//! empties the table (`_DeleteAllComputerShips(1)` @0x75993-0x759a4) and
//! walks the saved slots from 0 to 63 (@0x759af-0x75b91), spawning the
//! escort, then the fighter, each old slot holds; `_SpawnEscort` takes the
//! first inactive slot from 1 up (`_GenericRandomShipSpawn` @0x3c8b1-0x3c8f5).
//! So the k-th ship spawned takes slot k: the loaded ships take slots 1,
//! 2, 3, ... in ascending order of their old slot, hired escorts and
//! fighters mixed, every entry skipped taking none (findings F3 and F4 of
//! rdm:phase/escort-fleet-fidelity-followups/phase-20-loaded-escorts-in-slot-order).
//!
//! **Here.** Each record keeps its berth ([`Escort::berth`]), written by
//! [`Space::record_berths`] at every pass and every change to the fleet,
//! so a save holds the berth each ship is in, or for a record not yet
//! placed, the berth it will take, as the original's table already holds
//! it (F7). [`Space::fleet_berths`] places the records with no NPC in the
//! order [`packing_order`] gives. A record from an older save holds none,
//! and takes its berth after every record with one, in fleet order.
//!
//! A saved ship whose class has no record is dropped as the pilot loads
//! ([`drop_unknown_classes`]), as `_LoadPilotData` spawns none for it, and
//! the load counts as repaired
//! ([`Session::load_repaired`](super::Session::load_repaired)).

use super::{Pilot, Space};
use crate::catalog::ShipRecord;
use crate::fleet::Escort;

/// Drops each of `pilot`'s escorts and fighters whose class `ships` has
/// no record of: whether it dropped any. `_LoadPilotData` skips a saved
/// escort (@0x759f9-0x75a15) or fighter (@0x75b02-0x75b1e) whose `shïp`
/// entry holds the no-record mark, -9999 at +0xa, spawning none, so the
/// ship is gone from the fleet and its next save; the skip sets the
/// load's warning flag, on which it returns -46 (@0x7648b-0x76498).
pub(super) fn drop_unknown_classes(pilot: &mut Pilot, ships: &[ShipRecord]) -> bool {
    let before = pilot.escorts.len();
    pilot
        .escorts
        .retain(|escort| ships.iter().any(|record| record.id == escort.ship));
    pilot.escorts.len() < before
}

/// The fleet indices of `unplaced` in the order they take the free
/// berths: by the berth each was saved at, lowest first, as
/// `_LoadPilotData` walks the old slots up (@0x759af-0x75b91); those with
/// none (an older save) after the rest, and ties, in fleet order.
pub(super) fn packing_order(escorts: &[Escort], mut unplaced: Vec<usize>) -> Vec<usize> {
    unplaced.sort_by_key(|&index| {
        let berth = escorts[index].berth;
        (berth.is_none(), berth)
    });
    unplaced
}

impl Space {
    /// Writes each of `pilot`'s escort records the berth it holds, as
    /// [`Space::fleet_berths`] gives it: its NPC's, or for a record not
    /// yet placed, the packed berth it will take, so a save written now
    /// keeps the order `_WritePilotData` would (F7).
    pub(super) fn record_berths(&self, pilot: &mut Pilot) {
        let berths = self.fleet_berths(pilot);
        for (escort, berth) in pilot.escorts.iter_mut().zip(berths) {
            escort.berth = Some(berth);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fighters::tests::{fleet_npcs, flying_by, launch, pilot};
    use super::super::hire::tests::{BAR_AT, paid, paying};
    use super::super::voices::tests::{MEDIUM, MEDIUM_EVEN, VIPER, escort, voiced};
    use super::*;
    use crate::ai::lost_sight::SearchTimer;
    use crate::catalog::ShipId;
    use crate::chance::NeverFires;
    use crate::flight::Controls;
    use crate::rulebook::Rulebook;
    use crate::session::Session;
    use crate::testkit::{land_now, work};
    use crate::voice::VoiceVariant;

    /// `ship`'s escort record, saved at `berth`.
    fn saved(ship: ShipId, berth: Option<u32>) -> Escort {
        Escort {
            berth,
            ..escort(ship)
        }
    }

    /// The session of a pilot loaded with `escorts`, its fleet placed.
    fn loaded(escorts: Vec<Escort>) -> Session {
        let catalog = voiced();
        let mut session = Session::fly(&catalog, pilot(&catalog, 0, escorts)).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session
    }

    /// Each placed escort's class and berth, in fleet order.
    fn berths(session: &Session) -> Vec<(ShipId, u32)> {
        fleet_npcs(session)
            .iter()
            .map(|npc| (npc.ship, npc.berth))
            .collect()
    }

    #[test]
    fn a_saved_escort_or_fighter_of_no_class_is_dropped_and_the_load_repaired() {
        let fighter = Escort {
            carried: true,
            ..saved(ShipId(998), Some(1))
        };
        let session = loaded(vec![
            saved(ShipId(999), Some(2)),
            saved(MEDIUM, Some(5)),
            fighter,
            saved(MEDIUM_EVEN, Some(3)),
        ]);
        assert!(session.load_repaired());
        let ships: Vec<ShipId> = session.pilot.escorts.iter().map(|e| e.ship).collect();
        assert_eq!(ships, [MEDIUM, MEDIUM_EVEN]);
        assert_eq!(berths(&session), [(MEDIUM, 2), (MEDIUM_EVEN, 1)], "no gap");
    }

    #[test]
    fn a_fleet_whose_classes_all_have_records_loads_unrepaired() {
        let session = loaded(vec![saved(MEDIUM, Some(5)), saved(VIPER, None)]);
        assert!(!session.load_repaired());
        assert_eq!(session.pilot.escorts.len(), 2);
    }

    #[test]
    fn loaded_escorts_take_berths_packed_in_their_saved_order() {
        let session = loaded(vec![saved(MEDIUM, Some(5)), saved(MEDIUM_EVEN, Some(2))]);
        assert_eq!(berths(&session), [(MEDIUM, 2), (MEDIUM_EVEN, 1)]);
    }

    #[test]
    fn a_fighter_and_hired_escorts_interleave_by_saved_berth() {
        let fighter = Escort {
            carried: true,
            ..saved(VIPER, Some(1))
        };
        let session = loaded(vec![
            saved(MEDIUM, Some(4)),
            fighter,
            saved(MEDIUM_EVEN, Some(3)),
        ]);
        assert_eq!(
            berths(&session),
            [(MEDIUM, 3), (VIPER, 1), (MEDIUM_EVEN, 2)]
        );
    }

    #[test]
    fn loaded_in_the_same_run_each_escort_takes_the_words_of_the_berth_its_order_gives_it() {
        let catalog = voiced();
        let mut last = Session::fly(&catalog, pilot(&catalog, 0, Vec::new())).expect("flies");
        let traffic = &mut last.space.traffic;
        traffic.write_berth_voice(1, VoiceVariant::Odd);
        traffic.write_berth_voice(2, VoiceVariant::Even);
        traffic.write_berth_search(1, SearchTimer::Searching(40));
        traffic.write_berth_search(2, SearchTimer::Searching(9));
        let escorts = vec![saved(MEDIUM, Some(6)), saved(MEDIUM_EVEN, Some(3))];
        let mut session = Session::fly(&catalog, pilot(&catalog, 0, escorts))
            .expect("flies")
            .with_kept_berths(last.kept_berths());
        session.populate(&catalog, &mut NeverFires);
        let words: Vec<_> = fleet_npcs(&session)
            .iter()
            .map(|npc| (npc.ship, npc.voice, npc.lost_sight))
            .collect();
        assert_eq!(
            words,
            [
                (MEDIUM, Some(VoiceVariant::Even), SearchTimer::Carried(9)),
                (
                    MEDIUM_EVEN,
                    Some(VoiceVariant::Odd),
                    SearchTimer::Carried(40)
                ),
            ]
        );
    }

    /// Each record's berth, in fleet order.
    fn recorded(session: &Session) -> Vec<Option<u32>> {
        session
            .pilot
            .escorts
            .iter()
            .map(|escort| escort.berth)
            .collect()
    }

    #[test]
    fn a_flown_loads_records_take_their_npcs_berths() {
        let catalog = voiced();
        let mut session = loaded(vec![saved(MEDIUM, None), saved(MEDIUM_EVEN, Some(7))]);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(recorded(&session), [Some(2), Some(1)]);
    }

    #[test]
    fn a_launched_fighters_record_takes_its_berth() {
        let catalog = voiced();
        let mut session = flying_by(
            &catalog,
            pilot(&catalog, 1, Vec::new()),
            Rulebook::default(),
        );
        launch(&mut session, &catalog);
        let fighter = fleet_npcs(&session).pop().expect("launched");
        assert_eq!(recorded(&session), [Some(fighter.berth)]);
    }

    /// A pilot of `paying`'s loaded with escorts 129 and 130 saved at
    /// berths 5 and 2, landed at the bar before its fleet is placed, a
    /// ship 129 then hired.
    fn hired_after_loading() -> Session {
        let catalog = paying();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.cash = 1_000_000;
        pilot.escorts = vec![
            Escort {
                berth: Some(5),
                ..paid(129, None, false)
            },
            Escort {
                berth: Some(2),
                ..paid(130, None, false)
            },
        ];
        pilot.stellar = Some(BAR_AT);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(land_now(&mut session), Ok(BAR_AT));
        assert_eq!(session.space.escorts.fleet, [], "none placed yet");
        session.hire(ShipId(129), &mut NeverFires).expect("hired");
        session
    }

    #[test]
    fn a_hire_while_landed_records_its_berth_and_the_packed_berths_of_the_rest() {
        assert_eq!(
            recorded(&hired_after_loading()),
            [Some(2), Some(1), Some(3)]
        );
    }

    #[test]
    fn a_pilot_saved_before_its_fleet_is_placed_loads_in_the_packed_order_its_hire_after() {
        let last = hired_after_loading();
        let text = crate::save::encode(last.pilot());
        let pilot = crate::save::decode(&text).expect("loads");
        let catalog = paying();
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_kept_berths(last.kept_berths());
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(
            berths(&session),
            [(ShipId(129), 2), (ShipId(130), 1), (ShipId(129), 3)]
        );
    }

    #[test]
    fn records_with_no_saved_berth_take_theirs_in_fleet_order_after_the_rest() {
        let session = loaded(vec![
            saved(MEDIUM, None),
            saved(MEDIUM_EVEN, Some(7)),
            saved(VIPER, None),
        ]);
        assert_eq!(
            berths(&session),
            [(MEDIUM, 2), (MEDIUM_EVEN, 1), (VIPER, 3)]
        );
    }
}
