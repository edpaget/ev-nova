//! The corner bracket art the flight draws round its nav target and its
//! ship target, `cïcn` 10000-10023, for the test modules whose flights
//! select a stellar or target a ship: blank 16 x 16 stand-ins for Nova
//! Graphics 1's icons, so no frame reports them missing.

use nova_data::graphics::CICN;
use nova_data::graphics::fixture::{Cicn, Ctab};
use nova_rsrc::fixture::ForkBuilder;

/// `fork` with `cïcn` 10000-10023, each a blank 16 x 16 icon.
pub fn with_bracket_art(fork: ForkBuilder) -> ForkBuilder {
    let ctab = Ctab {
        device: false,
        entries: vec![(0, [0, 0, 0])],
    };
    let icon = Cicn::new(16, 16, 8, &ctab, &[0; 256], &[true; 256]).bytes();
    (10000..10024).fold(fork, |fork, id| fork.resource(CICN, id, None, &icon))
}
