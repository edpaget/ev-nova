//! What the flight view keeps of the shots between steps, for drawing
//! them as the original does: each spinning shot's start
//! ([`SpinStarts`]). The simulation keeps none of it: it is the view's,
//! drawn on the effects' chance, never the simulation's.

use nova_sim::{Chance, Session};

use crate::flight::catalog::Looks;
use crate::flight::spin::SpinStarts;

/// The view's state of the shots in flight.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ShotLooks {
    /// Each spinning shot's start.
    pub(super) spin_starts: SpinStarts,
}

impl ShotLooks {
    /// Steps it after a pass of `session`'s fight, the shots' looks read
    /// from `looks`, drawing on `chance`: each shot new in flight draws
    /// its spin start, and each gone is forgotten.
    pub(super) fn step(&mut self, session: &Session, looks: &Looks, chance: &mut dyn Chance) {
        let flags = |id| looks.weapon(id).map_or(0, |look| look.flags);
        let shots = session
            .shots()
            .iter()
            .map(|shot| (shot.id, flags(shot.weapon.id)));
        self.spin_starts.step(shots, chance);
    }

    /// Forgets every shot, as the effects are cleared on landing and
    /// arriving.
    pub(super) fn clear(&mut self) {
        self.spin_starts.clear();
    }
}
