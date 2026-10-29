//! Use As My Ship's rename: the session's side of
//! [`capture_naming`](crate::capture_naming), the branch of
//! `_DoPlunderDialog` after `_DoCapturedShipAssignmentDialog` returns Use
//! As My Ship (@0x943a3-0x94596). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The gate** (@0x9437a-0x943a8). After a successful capture roll
//! (@0x94368) and `_CanHireEscorts` (@0x9436d), a player whose class has
//! `Crew` 0 takes the ship straight into the fleet, with no assignment and
//! no rename ([`Session::plunder`]'s capture). Otherwise the assignment
//! dialog runs, and its Use As Escort also goes to the escort path
//! (@0x9459b). So the rename is reached only by Use As My Ship
//! ([`Session::assign`] answers [`Assigned::Naming`]), after the dialog's
//! button beep (`SimSound::CaptureAssigned`, @0x5b56b / @0x5b66b). A
//! mission's ship of `Crew` 0, boardable as a mission's, is renamed the same
//! way, its default built from its own `Name`.
//!
//! **The prompt** ([`Session::capture_naming`]): `STR#` 2002 #119 alone,
//! and the captured class's `Name` with three digits each drawn as
//! `Rand(9)` + 1, drawn afresh at each ask (see
//! [`capture_naming`](crate::capture_naming)). The branch makes no sound of
//! its own.
//!
//! **OK** ([`Session::name_capture`] with a name). The typed text goes
//! into the ship's name as it is, with no `_CullNameString` (`_pstrcpy`
//! @0x944ce-0x944df), and then `_DoShipCapture(target, 0)` (@0x94515)
//! takes the ship over. Its `OnRetire` and `OnCapture` run after the
//! name, so a `T` in them renames again, reading the typed name for its
//! `*`. A `_DoShipCapture` that finds no slot for the old ship (#305,
//! @0x41136) has already had the name written, so the player's old ship
//! keeps the typed name: the original's behaviour, which breaks no stock
//! mission. The swap makes a save due; the abandoned one makes none. A
//! captured ship gone by the time of either Use As My Ship or OK closes the
//! dialog with no name written: defensive, as the ship can't leave under
//! the original's modal dialogs and our flight is paused under them.
//!
//! **Cancel** ([`Session::name_capture`] with none). `_DisplayComm` says
//! #122 for 240 ticks (@0x9453c-0x9458f; the view says it) and the code
//! leaves by the plunder dialog's exit (@0x936ec → @0x936fc): the dialog
//! is disposed (@0x949f1) and the session resets once (@0x94a0b). No
//! `_DoShipCapture` runs, so no hook runs and no save is due. The NPC stays
//! as it was, disabled and marked boarded at the board, so it can't be
//! boarded again (@0x65064-0x6507a): the prize is lost.
//!
//! **The plunder dialog** stays drawn under both the assignment dialog
//! and the rename (neither hides it; it is disposed only at the exit,
//! @0x949f1). The session keeps the boarding (its stage) until the rename
//! is answered, but answers no plunder take or [`Session::boarding`]
//! meanwhile; the view keeps the dialog's last contents.

use super::{Session, Stage};
use crate::board::Assigned;
use crate::capture_naming::{CaptureNaming, capture_naming};
use crate::catalog::CommCatalog;
use crate::chance::Chance;

impl Session {
    /// The rename prompt for the ship captured, its text read from
    /// `catalog` and its default's digits drawn on `chance`, as the module
    /// says: none unless Use As My Ship awaits a name, or the captured
    /// class has no record.
    pub fn capture_naming(
        &self,
        catalog: &(impl CommCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<CaptureNaming> {
        let aboard = self
            .space
            .escorts
            .aboard
            .filter(|aboard| aboard.stage == Stage::Naming)?;
        let record = self.statics.ship_record(aboard.ship)?;
        Some(capture_naming(catalog, record, chance))
    }

    /// Answers the rename, as the module says: with `typed`, the ship's
    /// name is set to it unculled and the player takes the ship over
    /// ([`Assigned::MyShip`], or [`Assigned::Abandoned`] with no slot for
    /// the old ship), the hooks drawing on `chance`; with none (Cancel),
    /// the plunder dialog closes and nothing else changes
    /// ([`Assigned::Declined`]). None unless a name is awaited.
    ///
    /// None too, with the plunder dialog closed, when the captured ship
    /// has gone, and then the name is not written. In the original the
    /// ship can't leave while the modal rename is up, and our flight is
    /// paused under it, so this is defensive: no name is written for a
    /// capture that didn't happen.
    pub fn name_capture(
        &mut self,
        typed: Option<&str>,
        chance: &mut dyn Chance,
    ) -> Option<Assigned> {
        let aboard = self
            .space
            .escorts
            .aboard
            .filter(|aboard| aboard.stage == Stage::Naming)?;
        let Some(name) = typed else {
            self.close_plunder();
            return Some(Assigned::Declined);
        };
        if !self.npcs().iter().any(|npc| npc.id == aboard.npc) {
            self.close_plunder();
            return None;
        }
        // `_pstrcpy` alone, before `_DoShipCapture` (@0x944ce-0x944df).
        self.pilot.ship_name = Some(name.to_owned());
        self.take_over_captured(aboard, chance)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::board::{Assignment, BoardRefusal, Take, Taken};
    use crate::catalog::ShipId;
    use crate::combat::hull::Condition;
    use crate::control::Script;
    use crate::legal::NovaLaw;
    use crate::scan::Latches;
    use crate::session::tests::{aboard, board_by, boardable, captured, take};
    use crate::sound::SimSound;
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::Scripted;

    /// Use As My Ship, then the rename answered `name`: the two steps an
    /// existing takeover test runs, on one `chance`.
    pub(crate) fn use_as_my_ship(
        session: &mut Session,
        name: &str,
        chance: &mut dyn Chance,
    ) -> Option<Assigned> {
        match session.assign(Assignment::MyShip, chance) {
            Some(Assigned::Naming) => session.name_capture(Some(name), chance),
            other => other,
        }
    }

    /// [`boardable`] with `STR#` 2002 #119 and the captured trader (129)
    /// named "Shuttle".
    fn prompted() -> FakePilotCatalog {
        let mut catalog = boardable();
        let mut prompts = vec![String::new(); 118];
        prompts.push("Now rename this captured ship:".to_owned());
        catalog.strings.push((2002, prompts));
        for record in &mut catalog.ship_records {
            if record.id == ShipId(129) {
                record.name = "Shuttle".to_owned();
            }
        }
        catalog
    }

    /// [`captured`], Use As My Ship chosen, and its sounds and save taken.
    pub(crate) fn naming(catalog: &FakePilotCatalog) -> Session {
        let mut session = captured(catalog);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[])),
            Some(Assigned::Naming)
        );
        session.take_sounds();
        session.take_save_due();
        session
    }

    #[test]
    fn use_as_my_ship_beeps_and_awaits_a_name_changing_nothing_else() {
        let catalog = boardable();
        let mut session = captured(&catalog);
        session.take_sounds();
        session.take_save_due();
        let pilot = session.pilot().clone();
        let ship = session.ship.player;
        let npcs = session.npcs().to_vec();
        let mut chance = Draws::of(&[]);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut chance),
            Some(Assigned::Naming)
        );
        assert_eq!(chance.asked, Vec::<u32>::new(), "no draw");
        assert_eq!(session.take_sounds(), [SimSound::CaptureAssigned]);
        assert_eq!(session.pilot(), &pilot);
        assert_eq!(session.ship.player, ship);
        assert_eq!(session.npcs(), npcs.as_slice());
        assert!(!session.take_save_due());
        assert_eq!(session.boarding(), None, "no plunder answered");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[])),
            None,
            "assigned once"
        );
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            None
        );
        assert_eq!(
            take(&mut session, Take::Credits, &[]).0,
            Taken::Nothing,
            "the plunder takes nothing under the rename"
        );
    }

    #[test]
    fn the_prompt_is_119_and_the_captured_class_name_with_three_digits() {
        let catalog = prompted();
        let session = naming(&catalog);
        let mut chance = Scripted::rolling(&[0, 4, 8]);
        assert_eq!(
            session.capture_naming(&catalog, &mut chance),
            Some(CaptureNaming {
                prompt: "Now rename this captured ship:".to_owned(),
                default: "Shuttle 159".to_owned(),
            })
        );
        assert_eq!(chance.sides_asked, [9, 9, 9]);
    }

    #[test]
    fn no_prompt_without_a_name_awaited() {
        let catalog = prompted();
        let chance = &mut Scripted::default();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.capture_naming(&catalog, chance),
            None,
            "no boarding"
        );
        assert_eq!(
            aboard(&catalog).capture_naming(&catalog, chance),
            None,
            "plunder"
        );
        let session = captured(&catalog);
        assert_eq!(session.capture_naming(&catalog, chance), None, "assignment");
        let mut session = captured(&catalog);
        session.assign(Assignment::Escort, &mut Draws::of(&[]));
        assert_eq!(session.capture_naming(&catalog, chance), None, "escort");
        let mut session = naming(&catalog);
        session.name_capture(None, &mut Draws::of(&[]));
        assert_eq!(session.capture_naming(&catalog, chance), None, "declined");
    }

    #[test]
    fn ok_takes_the_ship_over_named_as_typed_unculled() {
        let catalog = boardable();
        let mut session = naming(&catalog);
        assert_eq!(
            session.name_capture(Some("The Prize"), &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
        assert_eq!(session.ship(), ShipId(129));
        assert_eq!(session.pilot().ship_name(), Some("The Prize"));
        assert_eq!(session.pilot().escorts().len(), 1, "the old ship");
        assert!(session.take_save_due());
        assert_eq!(session.space.escorts.aboard, None, "the dialog closed");
        assert_eq!(
            session.name_capture(Some("Again"), &mut Draws::of(&[])),
            None,
            "answered once"
        );
    }

    /// [`boardable`] whose ship 128 retires with `T25040` and whose
    /// trader 129 is captured with `T25041`, each list's one name built
    /// on the name before it.
    fn renaming() -> FakePilotCatalog {
        let mut catalog = boardable();
        for record in &mut catalog.ship_records {
            match record.id.0 {
                128 => record.on_retire = Script::parse("T25040"),
                129 => record.on_capture = Script::parse("T25041"),
                _ => {}
            }
        }
        catalog
    }

    /// The names [`renaming`]'s hooks pick: 25040 holds "* retired" and
    /// 25041 "*'s prize".
    struct Names;

    impl CommCatalog for Names {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                25040 => vec!["* retired".to_owned()],
                25041 => vec!["*'s prize".to_owned()],
                _ => Vec::new(),
            }
        }
    }

    #[test]
    fn the_hooks_run_after_the_name_and_rename_it_again() {
        let catalog = renaming();
        let mut session = naming(&catalog).with_strings(Rc::new(Names));
        assert_eq!(
            session.name_capture(Some("Kite"), &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
        assert_eq!(
            session.pilot().ship_name(),
            Some("Kite retired's prize"),
            "OnRetire, then OnCapture, each reading the name before it"
        );
    }

    /// [`naming`] with every ship slot the old ship could take filled.
    fn crowded(catalog: &FakePilotCatalog) -> Session {
        let mut session = naming(catalog);
        let reserve = session.setup.spawn_reserve(crate::traffic::CAPTURE_RESERVE);
        while session.space.traffic.free_berth(reserve).is_some() {
            let filler = session.space.traffic.npcs()[1].clone();
            session.space.traffic.add_npc(filler);
        }
        session
    }

    #[test]
    fn with_no_slot_the_swap_is_dropped_and_the_typed_name_kept() {
        let catalog = boardable();
        let mut session = crowded(&catalog);
        let ship = session.ship();
        assert_eq!(
            session.name_capture(Some("The Prize"), &mut Draws::of(&[])),
            Some(Assigned::Abandoned)
        );
        assert_eq!(session.ship(), ship, "not swapped");
        assert_eq!(session.pilot().ship_name(), Some("The Prize"));
        assert!(session.pilot().escorts().is_empty());
        assert_eq!(session.space.escorts.aboard, None);
    }

    #[test]
    fn cancel_declines_changing_nothing_and_loses_the_prize() {
        let catalog = renaming();
        let mut session = naming(&catalog).with_strings(Rc::new(Names));
        crate::session::precalced::tests::convicted(&mut session);
        let pilot = session.pilot().clone();
        let ship = session.ship.player;
        assert_eq!(
            session.name_capture(None, &mut Draws::of(&[])),
            Some(Assigned::Declined)
        );
        assert_eq!(
            session.pilot(),
            &pilot,
            "no ship, outfit, cargo, fleet, name or hook"
        );
        assert_eq!(session.ship.player, ship);
        assert!(!session.take_save_due());
        assert_eq!(session.take_sounds(), [], "silent");
        assert_eq!(session.space.escorts.aboard, None, "the dialog closed");
        assert_eq!(
            session.ship.latches.scan_latches,
            Latches::ARMED,
            "the close's reset"
        );
        let npc = session.space.npc(NpcId(0)).expect("still in the system");
        assert_eq!(npc.condition, Condition::Disabled);
        assert_eq!(
            board_by(&mut session, &NovaLaw::default()),
            Err(BoardRefusal::CantBoard),
            "boarded already"
        );
        assert_eq!(
            session.name_capture(None, &mut Draws::of(&[])),
            None,
            "answered once"
        );
    }

    #[test]
    fn nothing_awaited_answers_none() {
        let catalog = boardable();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.name_capture(Some("A"), &mut Draws::of(&[])), None);
        let mut session = captured(&catalog);
        assert_eq!(session.name_capture(Some("A"), &mut Draws::of(&[])), None);
        assert_eq!(session.name_capture(None, &mut Draws::of(&[])), None);
        assert_eq!(session.pilot().escorts().len(), 0);
        let mut session = aboard(&catalog);
        assert_eq!(session.name_capture(None, &mut Draws::of(&[])), None);
        assert!(session.boarding().is_some(), "the plunder still open");
    }

    /// The captured NPC, gone from the system.
    fn remove_captured(session: &mut Session) {
        let npc = session.space.escorts.aboard.expect("a capture").npc;
        session.space.traffic.remove(npc);
    }

    #[test]
    fn use_as_my_ship_with_the_ship_gone_beeps_and_closes_with_no_rename() {
        let catalog = prompted();
        let mut session = captured(&catalog);
        session.take_sounds();
        session.take_save_due();
        remove_captured(&mut session);
        assert_eq!(
            session.assign(Assignment::MyShip, &mut Draws::of(&[])),
            None
        );
        assert_eq!(session.space.escorts.aboard, None, "the dialog closed");
        assert_eq!(session.take_sounds(), [SimSound::CaptureAssigned]);
        assert!(!session.take_save_due());
        assert_eq!(
            session.capture_naming(&catalog, &mut Scripted::default()),
            None,
            "no rename"
        );
    }

    #[test]
    fn ok_with_the_ship_gone_writes_no_name_and_takes_nothing_over() {
        let catalog = boardable();
        let mut session = naming(&catalog);
        remove_captured(&mut session);
        let pilot = session.pilot().clone();
        let ship = session.ship.player;
        assert_eq!(session.name_capture(Some("X"), &mut Draws::of(&[])), None);
        assert_eq!(session.pilot(), &pilot, "no name, ship or fleet");
        assert_eq!(session.ship.player, ship);
        assert_eq!(session.space.escorts.aboard, None, "the dialog closed");
        assert!(!session.take_save_due());
        assert_eq!(session.take_sounds(), [], "silent");
    }

    #[test]
    fn use_as_escort_still_joins_the_fleet_at_once() {
        let catalog = boardable();
        let mut session = captured(&catalog);
        assert_eq!(
            session.assign(Assignment::Escort, &mut Draws::of(&[])),
            Some(Assigned::Escort)
        );
        assert_eq!(session.pilot().escorts().len(), 1);
        assert_eq!(session.space.escorts.aboard, None);
        assert_eq!(session.name_capture(None, &mut Draws::of(&[])), None);
    }
}
