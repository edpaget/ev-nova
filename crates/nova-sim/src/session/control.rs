//! The control bits on the session: the pilot's side of
//! [`control`](crate::control).
//!
//! **The bit edit.** [`Session::set_control_bit`] sets or clears one of the
//! pilot's control bits, and [`Session::control_bit`] reads one. An edit
//! makes a save due, as a change made in the spaceport does.

use super::Session;
use crate::control::Bit;

impl Session {
    /// Whether the pilot's control bit `bit` is set.
    #[must_use]
    pub fn control_bit(&self, bit: Bit) -> bool {
        self.pilot.control_bit(bit)
    }

    /// Sets the pilot's control bit `bit` when `on`, and clears it
    /// otherwise; a save is due.
    pub fn set_control_bit(&mut self, bit: Bit, on: bool) {
        let bits = self.pilot.bits_mut();
        if on {
            bits.set(bit);
        } else {
            bits.clear(bit);
        }
        self.save_due = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pilot::Pilot;
    use crate::save;
    use crate::testkit::catalog;

    fn session() -> Session {
        let catalog = catalog();
        Session::fly(&catalog, Pilot::new(&catalog, "Ada").expect("starts")).expect("flies")
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    #[test]
    fn a_control_bit_can_be_set_and_cleared() {
        let mut session = session();
        assert!(!session.control_bit(bit(42)));
        session.set_control_bit(bit(42), true);
        assert!(session.control_bit(bit(42)));
        assert!(!session.control_bit(bit(43)));
        assert!(session.pilot().control_bit(bit(42)));
        session.set_control_bit(bit(42), true);
        assert!(session.control_bit(bit(42)), "setting it again keeps it");
        session.set_control_bit(bit(42), false);
        assert!(!session.control_bit(bit(42)));
        session.set_control_bit(bit(42), false);
        assert!(!session.control_bit(bit(42)), "clearing it again keeps it");
    }

    #[test]
    fn each_edit_makes_a_save_due() {
        let mut session = session();
        assert!(!session.take_save_due());
        session.set_control_bit(bit(7), true);
        assert!(session.take_save_due());
        assert!(!session.take_save_due());
        session.set_control_bit(bit(7), false);
        assert!(session.take_save_due());
    }

    #[test]
    fn an_edited_bit_is_saved() {
        let mut session = session();
        session.set_control_bit(bit(9999), true);
        session.set_control_bit(bit(3), true);
        session.set_control_bit(bit(3), false);
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert!(saved.control_bit(bit(9999)));
        assert!(!saved.control_bit(bit(3)));
        assert_eq!(saved.control_bits(), session.pilot().control_bits());
    }
}
