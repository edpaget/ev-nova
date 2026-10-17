//! The galaxy map's government borders, as the original works them out:
//! a grid of screen cells, each holding up to four (government, value)
//! slots, that the in-flight maps shade under their nebulae.
//!
//! Addresses are in `EV Nova.app/Contents/MacOS/EV Nova` (i386). The
//! functions ported are `_MapPosToInfluenceCoords` (@0x114f7-0x11591),
//! `_AddSystemInfluenceToMap` (@0x11592-0x119ed), `_RecalcMapInfluence`
//! (@0x119ee-0x11c36), `_DrawMapInfluence`'s choice of cells and slots
//! (@0xe360-0xe567) and `_MapFilter`'s fine pass (@0x1463a-0x147ab). None
//! of them plays a sound or makes a random draw.
//!
//! # The grid ([`grid`])
//!
//! `_influenceOwner` (0xe09a0) and `_influenceValue` (0x1209c0) are 256 ×
//! 256 cells of four slots, laid out `[x][y][slot]`. A cell is 2 × 2 screen
//! pixels of the map item, so the grid moves with every zoom and scroll.
//! A system's cell is
//! trunc(trunc((pos − scroll) / zoom) / 2) + trunc(map size / 4)
//! ([`grid::cell_of`]), every division toward zero.
//!
//! A system adds to the cells within a radius of trunc(30.72 / zoom)
//! cells, or trunc(15.36 / zoom) when its government has `Flags2`
//! [`SMALL_INFLUENCE`](crate::govt::SMALL_INFLUENCE) ([`grid::radius`]),
//! each the value trunc(((r² − d²) × 0.2) × zoom), or × 0.4 with that
//! bit, clamped to 1..=255 ([`grid::value`]). A cell's slots take it as
//! [`grid::merge`] says: the first slot an allied government holds
//! (`_GovtAllies`, the same government included) takes the larger value
//! and ends the cell; otherwise the first empty slot; otherwise the
//! weakest slot is overwritten, even by a smaller value.
//!
//! # The passes ([`passes`])
//!
//! - **The coarse pass** (`_RecalcMapInfluence`, step 8): the lattice of
//!   every eighth cell is cleared, each place's system added once, and
//!   each lattice cell copied over its 8 × 8 block.
//! - **The fine pass** (`_MapFilter` on null events, step 1): the first
//!   event clears the grid; each later one adds the next contributing
//!   system, by ascending ID, a place's system once for each system at
//!   the place; the event that passes the last system asks for the map to
//!   be redrawn.
//!
//! Which systems contribute is the session's (`nova_sim::Session`'s
//! `map_border_sources`): [`BorderSources`] holds them for both passes.
//! The viewer, which has no session, counts every system with a
//! government ([`BorderSources::unfiltered`]).
//!
//! The owner index byte, the 16-bit cells, scroll and radius, and
//! `_nextInfluenceSyst`'s word sentinels (−1, 0x800, 0x801) are widths,
//! not behaviour: the cells and radius are `i32`, and the sentinels are
//! [`passes::Next`]'s states.

pub mod grid;
pub mod passes;

pub use grid::{InfluenceGrid, Slot};
pub use passes::{BorderPasses, Next, NullEvent, PaintedCell};

use crate::catalog::{GovtId, SystemId};
use crate::govt::{Governments, SMALL_INFLUENCE};

/// A system that adds to the borders: where it is, whose government it
/// is, and whether that government's spread is the smaller one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BorderSource {
    /// The system added: the system standing for its place.
    pub system: SystemId,
    /// Its map position.
    pub pos: (i16, i16),
    /// Its government.
    pub govt: GovtId,
    /// Whether the government has `Flags2`
    /// [`SMALL_INFLUENCE`](crate::govt::SMALL_INFLUENCE): half the radius,
    /// twice the value.
    pub small: bool,
}

/// What the two passes add, in their orders, and the governments their
/// slot merge asks about alliances.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BorderSources {
    /// The coarse pass's sources: one for each place, in the order of
    /// each place's first system (`_RecalcMapInfluence` @0x11a7a-0x11ade).
    pub coarse: Vec<BorderSource>,
    /// The fine pass's entries, by ascending ID of the system walked: one
    /// for each system whose place's system contributes, that system's
    /// source, so a place's system appears once for each of its systems
    /// (@0x146fe-0x14727).
    pub walk: Vec<(SystemId, BorderSource)>,
    /// The governments, for `_GovtAllies` and the flags.
    pub govts: Governments,
}

impl BorderSources {
    /// The viewer's sources: every one of `systems` with a government,
    /// each on its own place, by ascending ID, whatever its government's
    /// `Flags2` [`NO_INFLUENCE`](crate::govt::NO_INFLUENCE) says, with
    /// [`SMALL_INFLUENCE`] read from `govts`. The viewer has no session, so
    /// nothing is active, known, explored or inhabited to filter by.
    #[must_use]
    pub fn unfiltered(
        systems: impl IntoIterator<Item = (SystemId, (i16, i16), Option<GovtId>)>,
        govts: Governments,
    ) -> Self {
        let mut systems: Vec<_> = systems.into_iter().collect();
        systems.sort_by_key(|&(id, _, _)| id);
        let coarse: Vec<BorderSource> = systems
            .into_iter()
            .filter_map(|(system, pos, govt)| {
                let govt = govt?;
                Some(BorderSource {
                    system,
                    pos,
                    govt,
                    small: govts.flag2(Some(govt), SMALL_INFLUENCE),
                })
            })
            .collect();
        let walk = coarse
            .iter()
            .map(|source| (source.system, *source))
            .collect();
        Self {
            coarse,
            walk,
            govts,
        }
    }
}

/// Where the grid lies: the map's exact zoom and its scroll, the map
/// item's size in pixels, and the grid's size in cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridFrame {
    /// `_mapZoom`: map units per pixel.
    pub zoom: f64,
    /// `_mapScroll`: the map point at the map item's centre.
    pub scroll: (i32, i32),
    /// The map item's width and height in pixels: the in-flight maps'
    /// item 3 is 458 × 420.
    pub area: (i32, i32),
    /// The grid's width and height in cells.
    pub cells: (i32, i32),
}

impl GridFrame {
    /// The in-flight maps' grid: 256 × 256 cells.
    pub const IN_FLIGHT_CELLS: (i32, i32) = (256, 256);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::govt;

    #[test]
    fn the_viewers_sources_are_every_governed_system_by_id_with_the_small_bit() {
        let govts = Governments::new([
            govt(128),
            crate::catalog::GovtRecord {
                flags2: SMALL_INFLUENCE | crate::govt::NO_INFLUENCE,
                ..govt(129)
            },
        ]);
        let sources = BorderSources::unfiltered(
            [
                (SystemId(131), (5, 6), Some(GovtId(129))),
                (SystemId(130), (0, 0), None),
                (SystemId(129), (1, 2), Some(GovtId(128))),
                (SystemId(132), (1, 2), Some(GovtId(128))),
            ],
            govts.clone(),
        );
        let source = |system, pos, govt, small| BorderSource {
            system: SystemId(system),
            pos,
            govt: GovtId(govt),
            small,
        };
        let expected = [
            source(129, (1, 2), 128, false),
            source(131, (5, 6), 129, true),
            source(132, (1, 2), 128, false),
        ];
        assert_eq!(sources.coarse, expected, "each on its own place");
        let walked: Vec<(SystemId, BorderSource)> = expected
            .iter()
            .map(|source| (source.system, *source))
            .collect();
        assert_eq!(sources.walk, walked);
        assert_eq!(sources.govts, govts);
    }
}
