//! What a grant gave ([`Granted`]), from `nova_sim_port::grant`, which
//! re-exports it and keeps the grant's roll.

use nova_sim_catalog::catalog::OutfitId;

/// What a grant gave: `count` of `outfit`, the outfit picked; a count of
/// none when none of it fits (step 6 of the module docs), which grants
/// nothing and says nothing, but still resets the session as the pick
/// does (`_DoPlunderDialog` @0x931d9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Granted {
    /// The outfit granted.
    pub outfit: OutfitId,
    /// How many: none when none of it fits.
    pub count: u16,
}
