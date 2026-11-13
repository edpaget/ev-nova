//! A tutorial hint, [`TutorialHint`], from `nova_sim_combat::tutorial`,
//! which re-exports it and keeps the first-flight tutorial that raises
//! it.

use nova_sim_catalog::catalog::StellarId;

/// A tutorial hint, in the order of the hint block's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TutorialHint {
    /// Land on the nearest stellar (with the can-land flag): `STR#` 2002 #22 for a station
    /// (`spöb` `Flags` 0x10, @0x70a31) or #23, then #24 and #25 around the
    /// land key's name.
    Land {
        /// The stellar to land on.
        stellar: StellarId,
        /// Whether it is a station, docked at rather than landed on.
        station: bool,
    },
    /// Hyperspace to another system: `STR#` 2002 #26-#28 around the map
    /// and jump keys' names.
    Hyperspace,
    /// The ship is far enough out to jump: `STR#` 2002 #30.
    FarEnough,
    /// There is nothing to find this far out: `STR#` 2002 #31.
    NothingOutHere,
}
