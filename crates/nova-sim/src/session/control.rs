//! The control bits on the session: the pilot's side of
//! [`control`](crate::control).
//!
//! **The bit edit.** [`Session::set_control_bit`] sets or clears one of the
//! pilot's control bits, and [`Session::control_bit`] reads one. An edit
//! makes a save due, as a change made in the spaceport does.
//!
//! **The pilot view.** Control-bit tests read the pilot through [`Facts`]:
//! its bits, gender and explored systems as the pilot holds them; the game
//! always counts as paid for, as the engine takes the game as registered
//! (see [`hire`](crate::hire)); and an outfit is had when the pilot owns
//! one or when a fighter out of its bay would dock back into it, as the
//! original counts deployed fighters (`_GetToken` @0x14a83): the bay
//! launching its ship type, that bay's ammunition outfit of lowest ID, the
//! same routing a fighter docking takes.

use super::Session;
use crate::catalog::{OutfitId, ShipId, SystemId, WeaponId};
use crate::combat::armament::Armament;
use crate::control::{Bit, PilotFacts};
use crate::pilot::{Gender, Pilot};

/// What a control-bit test reads about the session's pilot (see the module
/// docs).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts<'a> {
    /// The pilot.
    pub(crate) pilot: &'a Pilot,
    /// Each ammunition outfit, with the `wëap` it is the rounds of.
    pub(crate) ammo_outfits: &'a [(WeaponId, OutfitId)],
    /// The player's weapons, its fighter bays among them.
    pub(crate) armament: &'a Armament,
}

impl Facts<'_> {
    /// The outfit a carried fighter of ship type `ship` docks back into:
    /// the first bay launching it, that bay's ammunition outfit of lowest
    /// ID.
    fn docks_into(&self, ship: ShipId) -> Option<OutfitId> {
        let bay = self
            .armament
            .mounts()
            .iter()
            .find(|mount| mount.spec.carried == Some(ship))?
            .spec
            .id;
        self.ammo_outfits
            .iter()
            .filter(|&&(of, _)| of == bay)
            .map(|&(_, outfit)| outfit)
            .min()
    }
}

impl PilotFacts for Facts<'_> {
    fn bit(&self, bit: Bit) -> bool {
        self.pilot.control_bit(bit)
    }

    fn gender(&self) -> Gender {
        self.pilot.gender()
    }

    /// Always: the engine takes the game as registered.
    fn paid(&self, _days: u16) -> bool {
        true
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.pilot.owned(outfit) > 0
            || self
                .pilot
                .escorts()
                .iter()
                .filter(|escort| escort.carried)
                .any(|escort| self.docks_into(escort.ship) == Some(outfit))
    }

    fn explored(&self, system: SystemId) -> bool {
        self.pilot.has_explored(system)
    }
}

/// The session's pilot, as its `Facts` read it.
impl PilotFacts for Session {
    fn bit(&self, bit: Bit) -> bool {
        self.facts().bit(bit)
    }

    fn gender(&self) -> Gender {
        self.facts().gender()
    }

    fn paid(&self, days: u16) -> bool {
        self.facts().paid(days)
    }

    fn has_outfit(&self, outfit: OutfitId) -> bool {
        self.facts().has_outfit(outfit)
    }

    fn explored(&self, system: SystemId) -> bool {
        self.facts().explored(system)
    }
}

impl Session {
    /// What a control-bit test reads about the pilot.
    pub(crate) fn facts(&self) -> Facts<'_> {
        Facts {
            pilot: &self.pilot,
            ammo_outfits: &self.ammo_outfits,
            armament: &self.armament,
        }
    }

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
    fn the_pilot_view_reads_the_bits_gender_and_explored_systems() {
        let mut session = session();
        session.set_control_bit(bit(12), true);
        let facts = session.facts();
        assert!(facts.bit(bit(12)));
        assert!(!facts.bit(bit(13)));
        assert_eq!(facts.gender(), Gender::Male);
        let start = session.pilot().system();
        assert!(facts.explored(start));
        assert!(!facts.explored(SystemId(start.0 + 1)));
        let catalog = catalog();
        let female = Pilot::new(&catalog, "Eve")
            .expect("starts")
            .with_gender(Gender::Female);
        let session = Session::fly(&catalog, female).expect("flies");
        assert_eq!(session.facts().gender(), Gender::Female);
    }

    #[test]
    fn the_pilot_view_always_counts_the_game_as_paid() {
        let session = session();
        for days in [0, 30, u16::MAX] {
            assert!(session.facts().paid(days), "{days}");
        }
    }

    #[test]
    fn the_pilot_view_has_an_outfit_the_pilot_owns() {
        let catalog = catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.outfits.insert(OutfitId(300), 1);
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.facts().has_outfit(OutfitId(300)));
        assert!(!session.facts().has_outfit(OutfitId(301)));
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
