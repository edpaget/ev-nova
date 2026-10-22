//! Whether the pilot has seen the intro screen, the original's
//! `seenIntroScreen` (see [`Pilot::seen_intro`]): the main screen reads it
//! as play is entered, and sets it once the intro is over
//! (`_EnterGameFromMainScreen` @0x19ea2-0x19ebc).

use super::Session;
#[cfg(doc)]
use crate::pilot::Pilot;

impl Session {
    /// Whether the pilot flying has seen the intro screen.
    #[must_use]
    pub fn seen_intro(&self) -> bool {
        self.pilot.seen_intro
    }

    /// Marks the pilot as having seen the intro screen. It asks for no
    /// save: the original saves nothing at entry, so the flag reaches disk
    /// with the next save.
    pub fn mark_intro_seen(&mut self) {
        self.pilot.seen_intro = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tests::alternates;

    #[test]
    fn a_new_pilot_has_not_seen_the_intro_until_marked_and_marking_asks_for_no_save() {
        let mut session = Session::start(&alternates()).expect("starts");
        session.take_save_due();
        assert!(!session.seen_intro());
        session.mark_intro_seen();
        assert!(session.seen_intro());
        assert!(session.pilot().seen_intro());
        assert!(!session.take_save_due());
    }
}
