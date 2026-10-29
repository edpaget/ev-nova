//! The opening of a pilot from a save: [`Session::open_saved`] resumes the
//! pilot ([`Session::resume`]), then runs the setup for how it was opened
//! ([`SavedOpening`]), as one call.
//!
//! The original runs each pair as one routine, so a caller can't resume a
//! pilot without its setup, or set it up without resuming it:
//!
//! - **At launch** ([`SavedOpening::AtLaunch`]): `_OpenLastPilotFile`
//!   (@0x76502) loads the last pilot, and `_DoMainScreen`'s once-a-launch
//!   setup follows ([`Session::open_at_launch`]).
//! - **From Open Pilot** ([`SavedOpening::OpenPilot`]): `_OpenPilotFile`
//!   (@0x7673e) loads the chosen pilot, and `_MainScreenChoice(1)`'s loop
//!   over the 16 mission slots follows (@0x1a918-0x1a982,
//!   [`Session::restart_aux_from_main_screen`]).
//!
//! Neither makes a save due, and both settle before the main menu's first
//! frame.

use crate::chance::Chance;

use super::Session;

/// How a pilot from a save was opened, which decides the setup
/// [`Session::open_saved`] runs after resuming it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedOpening {
    /// The last pilot, opened as the game launches (`_OpenLastPilotFile`
    /// @0x76502, then `_DoMainScreen`'s once-a-launch setup).
    AtLaunch,
    /// A pilot chosen with Open Pilot (`_OpenPilotFile` @0x7673e, then
    /// `_MainScreenChoice(1)`'s loop @0x1a918-0x1a982).
    OpenPilot,
}

impl Session {
    /// Opens a pilot from a save, after [`Session::fly`], drawing on
    /// `chance`: resumes it ([`Session::resume`]), then runs `opening`'s
    /// setup, [`Session::open_at_launch`] at launch or
    /// [`Session::restart_aux_from_main_screen`] from Open Pilot. No save
    /// is due.
    pub fn open_saved(&mut self, opening: SavedOpening, chance: &mut dyn Chance) {
        self.resume(chance);
        match opening {
            SavedOpening::AtLaunch => self.open_at_launch(chance),
            SavedOpening::OpenPilot => self.restart_aux_from_main_screen(chance),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MissionShip;
    use crate::mission::aux_ships::AuxShips;
    use crate::session::mission_ships::tests::{accepted, aux_catalog, aux_ships};
    use crate::testkit::Draws;

    /// A session holding mission 140 with auxiliary ships, all of them
    /// spent: the launch's unguarded restart redraws them, Open Pilot's
    /// leaves them as saved.
    fn saved() -> Session {
        let catalog = aux_catalog(MissionShip::NONE, aux_ships(2, -1), 0);
        let mut session = accepted(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.aux = AuxShips {
                countdown: 5,
                arrived: 2,
                left: 0,
            };
        }
        let _ = session.take_save_due();
        session
    }

    #[test]
    fn opening_a_saved_pilot_at_launch_resumes_it_then_runs_the_launch_setup() {
        let mut session = saved();
        let mut by_hand = session.clone();
        let mut draws = Draws::of(&[]);
        session.open_saved(SavedOpening::AtLaunch, &mut draws);
        let mut hand_draws = Draws::of(&[]);
        by_hand.resume(&mut hand_draws);
        by_hand.open_at_launch(&mut hand_draws);
        assert_eq!(session, by_hand);
        assert_eq!(draws.asked, hand_draws.asked);
        assert!(!session.take_save_due(), "no save is due");
    }

    #[test]
    fn opening_a_saved_pilot_from_open_pilot_resumes_it_then_restarts_the_aux_from_the_main_screen()
    {
        let mut session = saved();
        let mut by_hand = session.clone();
        let mut draws = Draws::of(&[]);
        session.open_saved(SavedOpening::OpenPilot, &mut draws);
        let mut hand_draws = Draws::of(&[]);
        by_hand.resume(&mut hand_draws);
        by_hand.restart_aux_from_main_screen(&mut hand_draws);
        assert_eq!(session, by_hand);
        assert_eq!(draws.asked, hand_draws.asked);
        assert!(!session.take_save_due(), "no save is due");
    }
}
