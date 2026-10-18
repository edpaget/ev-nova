//! The borders' passes over the grid: the open's clear, the coarse pass
//! (`_RecalcMapInfluence` @0x119ee-0x11c36), the fine pass's null events
//! (`_MapFilter` @0x1463a-0x147ab), and the cells `_DrawMapInfluence`
//! paints (@0xe360-0xe567).
//!
//! **The globals.** `_influenceStepSize` (0x1209a0) is the lattice's step,
//! `_nextInfluenceSyst` (0x160ec0) the fine pass's cursor, −1 before it
//! starts, the next system index while it walks and 0x801 once it is
//! done ([`Next`]), and `_influenceCoordsCached` (0xe0986) whether a
//! coarse pass has run since the last open or pass began: the fine pass
//! runs only after one (@0x14656).
//!
//! **The coarse pass.** Each caller sets the step to 8, the cursor to −1
//! and the cached flag to 0 first, and calls it only with the borders on:
//! the open, the borders turned on, a zoom step, a drag's release, Space
//! and each arrow (the map's). Then `_RecalcMapInfluence` clears the
//! lattice (@0x11a16-0x11a64), adds each place's system once
//! (@0x11a7a-0x11ade), copies each lattice cell over its block
//! (@0x11ae0-0x11c1e) and sets the cached flag (@0x11c24).
//!
//! **The fine pass.** Each null event while the borders are on and the
//! cached flag is set ([`BorderPasses::null_event`]):
//!
//! - with the cursor at −1, sets it to 0, the step to 1, and clears the
//!   whole grid, and does nothing else (@0x14664-0x146c8);
//! - otherwise walks on from the cursor, passing each index to
//!   `_FindActiveCoLocatedSystem` and adding its answer, and stops once an
//!   add returns true (@0x146fe-0x14737); an add that returns false, out
//!   of bounds, walks on in the same event;
//! - when the walk reaches 0x800 it becomes 0x801 and item 3 is
//!   invalidated (@0x14741-0x1479a): the final redraw comes on the event
//!   that passes the last index, so with the same event's add only when
//!   index 0x7ff ([`LAST_SLOT`]) contributes, and on a later event that
//!   adds nothing otherwise;
//! - at 0x801 does nothing.
//!
//! The walk is [`BorderSources::walk`], the systems whose place's system
//! contributes, by ascending ID; the cursor is the system walked last,
//! not a place in the list, so sources taken again mid-pass go on from
//! the next system.

use super::grid::InfluenceGrid;
use super::{BorderSources, GridFrame};
use crate::catalog::{GovtId, SystemId};

/// The step of the coarse pass's lattice (`_RecalcAllMapInfluence`
/// @0x11c42, and each caller).
pub const COARSE_STEP: i32 = 8;
/// The step of the fine pass (@0x14686).
pub const FINE_STEP: i32 = 1;
/// The last of the original's 0x800 system slots, index 0x7ff: when the
/// walk's last system is it, its add redraws in the same event.
pub const LAST_SLOT: SystemId = SystemId(128 + 0x7ff);

/// Where the fine pass's walk is: `_nextInfluenceSyst`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// −1: the next null event clears the grid and starts the walk.
    Unstarted,
    /// Walking: from the start (`None`, 0) or on from the system added
    /// last.
    Walking {
        /// The system walked and added last, if any.
        after: Option<SystemId>,
    },
    /// 0x801: done until the next coarse pass.
    Idle,
}

/// What a null event did ([`BorderPasses::null_event`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullEvent {
    /// Nothing: no coarse pass since the open, or the pass is done.
    Nothing,
    /// The grid cleared and the walk started at step 1.
    Cleared,
    /// One more system added.
    Added,
    /// The walk passed the last system: item 3 is to be redrawn.
    Redraw,
}

/// A lattice cell `_DrawMapInfluence` paints: its place, the step it
/// stands for, and the strongest of its slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintedCell {
    /// Its column.
    pub x: i32,
    /// Its row.
    pub y: i32,
    /// The lattice's step: its square is 2 × step pixels on a side.
    pub step: i32,
    /// The strongest slot's government.
    pub govt: GovtId,
    /// The strongest slot's value.
    pub value: u8,
}

/// The borders' grid and the passes' globals: the step, the fine pass's
/// cursor and the cached flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BorderPasses {
    grid: InfluenceGrid,
    step: i32,
    next: Next,
    cached: bool,
}

impl Default for BorderPasses {
    fn default() -> Self {
        Self::new()
    }
}

impl BorderPasses {
    /// No grid yet, uncached: a null event does nothing until a coarse
    /// pass.
    #[must_use]
    pub fn new() -> Self {
        Self {
            grid: InfluenceGrid::new((0, 0)),
            step: FINE_STEP,
            next: Next::Unstarted,
            cached: false,
        }
    }

    /// The grid.
    #[must_use]
    pub fn grid(&self) -> &InfluenceGrid {
        &self.grid
    }

    /// The lattice's step.
    #[must_use]
    pub fn step(&self) -> i32 {
        self.step
    }

    /// The fine pass's cursor.
    #[must_use]
    pub fn next(&self) -> Next {
        self.next
    }

    /// Whether a coarse pass has run since the open or the last pass
    /// began: `_influenceCoordsCached`.
    #[must_use]
    pub fn cached(&self) -> bool {
        self.cached
    }

    /// The map opening (`_DoSystemMap` @0x1233f, @0x12440-0x1249e): the
    /// whole grid cleared and the cached flag with it.
    pub fn open(&mut self) {
        self.grid.clear_all();
        self.cached = false;
    }

    /// A coarse pass over `sources` in `frame` (see the module docs): the
    /// step 8, the cursor −1, then `_RecalcMapInfluence`. The grid takes
    /// the frame's size, cleared, if it had another.
    pub fn coarse(&mut self, sources: &BorderSources, frame: &GridFrame) {
        self.fit(frame.cells);
        self.step = COARSE_STEP;
        self.next = Next::Unstarted;
        self.cached = false;
        self.grid.clear_lattice(self.step);
        for source in &sources.coarse {
            self.grid.add(source, frame, self.step, &sources.govts);
        }
        // The original fills only at a step above 1; at 1 each block is
        // its own lattice cell, and the fill changes nothing.
        self.grid.fill_blocks(self.step);
        self.cached = true;
    }

    /// One null event of the fine pass over `sources` in `frame` (see the
    /// module docs). The caller runs it only while the borders are on.
    pub fn null_event(&mut self, sources: &BorderSources, frame: &GridFrame) -> NullEvent {
        if !self.cached {
            return NullEvent::Nothing;
        }
        self.fit(frame.cells);
        let after = match self.next {
            Next::Idle => return NullEvent::Nothing,
            Next::Unstarted => {
                self.next = Next::Walking { after: None };
                self.step = FINE_STEP;
                self.grid.clear_all();
                return NullEvent::Cleared;
            }
            Next::Walking { after } => after,
        };
        let last = sources.walk.last().map(|&(system, _)| system);
        let ahead = sources
            .walk
            .iter()
            .filter(|&&(system, _)| after.is_none_or(|after| system > after));
        for &(system, source) in ahead {
            if self.grid.add(&source, frame, self.step, &sources.govts) {
                if Some(system) == last && system >= LAST_SLOT {
                    break;
                }
                self.next = Next::Walking {
                    after: Some(system),
                };
                return NullEvent::Added;
            }
        }
        self.next = Next::Idle;
        NullEvent::Redraw
    }

    /// The cells `_DrawMapInfluence` paints (@0xe3a4-0xe54b): each lattice
    /// cell of the current step, x outer and y inner, whose slot 0 holds a
    /// value (@0xe3c9), with the slot of greatest value, ties to the
    /// first (@0xe4af-0xe516).
    #[must_use]
    pub fn painted(&self) -> Vec<PaintedCell> {
        let (w, h) = self.grid.size();
        let step = usize::try_from(self.step).unwrap_or(1).max(1);
        let mut cells = Vec::new();
        for x in (0..w).step_by(step) {
            for y in (0..h).step_by(step) {
                let slots = self.grid.slots(x, y);
                if slots[0].value == 0 {
                    continue;
                }
                let mut best = slots[0];
                for slot in &slots[1..] {
                    if slot.value > best.value {
                        best = *slot;
                    }
                }
                cells.push(PaintedCell {
                    x,
                    y,
                    step: self.step,
                    govt: best.govt,
                    value: best.value,
                });
            }
        }
        cells
    }

    /// The grid at `cells`' size: a new, empty one when it had another.
    fn fit(&mut self, cells: (i32, i32)) {
        if self.grid.size() != cells {
            self.grid = InfluenceGrid::new(cells);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::govt::Governments;
    use crate::map_borders::BorderSource;
    use crate::map_borders::grid::{SLOTS, Slot};
    use crate::testkit::govt;

    /// Item 3's frame at zoom 1, looking at (0, 0).
    fn frame() -> GridFrame {
        GridFrame {
            zoom: 1.0,
            scroll: (0, 0),
            area: (458, 420),
            cells: GridFrame::IN_FLIGHT_CELLS,
        }
    }

    fn source(system: i16, pos: (i16, i16), govt: i16) -> BorderSource {
        BorderSource {
            system: SystemId(system),
            pos,
            govt: GovtId(govt),
            small: false,
        }
    }

    /// Governments 128-136, none allied with another.
    fn govts() -> Governments {
        Governments::new((128..=136).map(govt))
    }

    /// `coarse`, each walked by its own system.
    fn sources(coarse: &[BorderSource]) -> BorderSources {
        BorderSources {
            coarse: coarse.to_vec(),
            walk: coarse
                .iter()
                .map(|source| (source.system, *source))
                .collect(),
            govts: govts(),
        }
    }

    fn held(passes: &BorderPasses) -> usize {
        let (w, h) = passes.grid().size();
        (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| passes.grid().slots(x, y)[0].value != 0)
            .count()
    }

    /// The fine pass's events until the first `Nothing`, which isn't
    /// listed.
    fn run_fine(passes: &mut BorderPasses, sources: &BorderSources) -> Vec<NullEvent> {
        let mut events = Vec::new();
        loop {
            match passes.null_event(sources, &frame()) {
                NullEvent::Nothing => return events,
                event => events.push(event),
            }
            assert!(events.len() < 100, "runs on: {events:?}");
        }
    }

    // The coarse pass.

    #[test]
    fn a_new_grid_does_nothing_until_a_coarse_pass() {
        let mut passes = BorderPasses::new();
        let sources = sources(&[source(128, (0, 0), 128)]);
        assert!(!passes.cached());
        assert_eq!(passes.null_event(&sources, &frame()), NullEvent::Nothing);
        assert_eq!(passes.painted(), []);
        assert_eq!(passes, BorderPasses::default());
    }

    #[test]
    fn the_coarse_pass_fills_8_by_8_blocks_from_each_lattice_cell_and_caches() {
        let mut passes = BorderPasses::new();
        passes.coarse(&sources(&[source(128, (0, 0), 128)]), &frame());
        assert!(passes.cached());
        assert_eq!(passes.step(), COARSE_STEP);
        assert_eq!(passes.next(), Next::Unstarted);
        let grid = passes.grid();
        // Cell (114, 105) snaps to (112, 104), r 24.
        let centre = grid.slots(112, 104);
        assert_eq!(centre[0].value, 115);
        for x in 112..120 {
            for y in 104..112 {
                assert_eq!(grid.slots(x, y), centre, "({x}, {y})");
            }
        }
        assert_eq!(grid.slots(111, 104)[0].value, 102, "(104, 104)'s block");
        assert_eq!(held(&passes), 29 * 64, "29 lattice cells, each a block");
    }

    #[test]
    fn the_blocks_stop_at_the_grids_edges() {
        let mut passes = BorderPasses::new();
        let corner = GridFrame {
            cells: (20, 12),
            area: (0, 0),
            ..frame()
        };
        passes.coarse(&sources(&[source(128, (0, 0), 128)]), &corner);
        assert_eq!(passes.grid().size(), (20, 12));
        assert_eq!(
            passes.grid().slots(19, 11),
            passes.grid().slots(16, 8),
            "the last block, clipped"
        );
        assert_eq!(held(&passes), 20 * 12);
    }

    #[test]
    fn the_coarse_pass_clears_the_lattice_of_a_grid_holding_fine_cells() {
        let mut passes = BorderPasses::new();
        let fed = sources(&[source(128, (0, 0), 128)]);
        passes.coarse(&fed, &frame());
        run_fine(&mut passes, &fed);
        assert_eq!(passes.step(), FINE_STEP);
        assert_ne!(passes.grid().slots(113, 105)[0].value, 0, "a fine cell");
        let far = sources(&[source(129, (-200, -180), 129)]);
        passes.coarse(&far, &frame());
        assert_eq!(passes.grid().slots(113, 105), [Slot::EMPTY; SLOTS]);
        assert!(!passes.painted().is_empty());
        assert!(
            passes
                .painted()
                .iter()
                .all(|cell| cell.govt == GovtId(129) && cell.step == 8)
        );
    }

    #[test]
    fn opening_clears_every_cell_and_uncaches_until_a_coarse_pass() {
        let mut passes = BorderPasses::new();
        let fed = sources(&[source(128, (0, 0), 128)]);
        passes.coarse(&fed, &frame());
        passes.open();
        assert_eq!(held(&passes), 0);
        assert!(!passes.cached());
        assert_eq!(passes.null_event(&fed, &frame()), NullEvent::Nothing);
        assert_eq!(passes.null_event(&fed, &frame()), NullEvent::Nothing);
        passes.coarse(&fed, &frame());
        assert_eq!(passes.null_event(&fed, &frame()), NullEvent::Cleared);
    }

    // The fine pass.

    #[test]
    fn the_first_null_event_clears_the_grid_at_step_1() {
        let mut passes = BorderPasses::new();
        let fed = sources(&[source(128, (0, 0), 128)]);
        passes.coarse(&fed, &frame());
        assert_eq!(passes.null_event(&fed, &frame()), NullEvent::Cleared);
        assert_eq!(held(&passes), 0);
        assert_eq!(passes.step(), FINE_STEP);
        assert_eq!(passes.next(), Next::Walking { after: None });
        assert!(passes.cached(), "still cached");
        assert_eq!(passes.painted(), [], "nothing until the next add");
    }

    #[test]
    fn each_later_event_adds_one_system_skipping_one_out_of_bounds() {
        let mut passes = BorderPasses::new();
        let three = sources(&[
            source(128, (0, 0), 128),
            source(129, (2000, 0), 129),
            source(130, (100, 100), 130),
        ]);
        passes.coarse(&three, &frame());
        assert_eq!(passes.null_event(&three, &frame()), NullEvent::Cleared);
        assert_eq!(passes.null_event(&three, &frame()), NullEvent::Added);
        let one = held(&passes);
        assert_eq!(one, 2821, "the cells within 30 of (114, 105)");
        assert_eq!(
            passes.next(),
            Next::Walking {
                after: Some(SystemId(128))
            }
        );
        assert_eq!(
            passes.null_event(&three, &frame()),
            NullEvent::Added,
            "129, far off the grid, walked past in the same event"
        );
        assert!(held(&passes) > one);
        assert_eq!(
            passes.next(),
            Next::Walking {
                after: Some(SystemId(130))
            }
        );
        assert_eq!(
            passes.null_event(&three, &frame()),
            NullEvent::Redraw,
            "the last system, 130, isn't index 0x7ff: a later empty event"
        );
        assert_eq!(passes.next(), Next::Idle);
        let done = passes.grid().clone();
        assert_eq!(passes.null_event(&three, &frame()), NullEvent::Nothing);
        assert_eq!(passes.grid(), &done);
    }

    #[test]
    fn an_add_of_the_last_slot_redraws_in_the_same_event() {
        let mut passes = BorderPasses::new();
        let last = sources(&[source(128, (0, 0), 128), source(LAST_SLOT.0, (40, 0), 129)]);
        passes.coarse(&last, &frame());
        assert_eq!(
            run_fine(&mut passes, &last),
            [NullEvent::Cleared, NullEvent::Added, NullEvent::Redraw],
            "index 0x7ff's add and the redraw in one event"
        );
        assert_eq!(
            passes.grid().slots(134, 105)[1],
            Slot {
                govt: GovtId(129),
                value: 180
            },
            "0x7ff added, beside 128's 100"
        );
        let out = sources(&[
            source(128, (0, 0), 128),
            source(LAST_SLOT.0, (3000, 0), 129),
        ]);
        passes.coarse(&out, &frame());
        assert_eq!(
            run_fine(&mut passes, &out),
            [NullEvent::Cleared, NullEvent::Added, NullEvent::Redraw],
            "0x7ff out of bounds: walked past in the redraw's event"
        );
    }

    #[test]
    fn an_empty_walk_redraws_on_the_event_after_the_clear() {
        let mut passes = BorderPasses::new();
        let none = sources(&[]);
        passes.coarse(&none, &frame());
        assert_eq!(
            run_fine(&mut passes, &none),
            [NullEvent::Cleared, NullEvent::Redraw]
        );
    }

    // The walk's order.

    #[test]
    fn a_source_between_two_repeats_is_overwritten_by_the_second() {
        // At cell (114, 105) the values are 100, 90, 80, 10 and 50 (d²
        // 400, 450, 500, 850 and 650 at zoom 1); yellow is walked twice.
        let pirate = source(128, (40, 0), 130);
        let queen = source(129, (42, 6), 131);
        let rebel = source(130, (44, 8), 132);
        let yellow = source(131, (58, 6), 133);
        let xeno = source(132, (50, 10), 134);
        let sources = BorderSources {
            coarse: vec![pirate, queen, rebel, yellow, xeno],
            walk: vec![
                (SystemId(128), pirate),
                (SystemId(129), queen),
                (SystemId(130), rebel),
                (SystemId(131), yellow),
                (SystemId(132), xeno),
                (SystemId(133), yellow),
            ],
            govts: govts(),
        };
        let owners =
            |grid: &InfluenceGrid| grid.slots(114, 105).map(|slot| (slot.govt.0, slot.value));
        let mut passes = BorderPasses::new();
        passes.coarse(&sources, &frame());
        assert_eq!(passes.null_event(&sources, &frame()), NullEvent::Cleared);
        for _ in 0..5 {
            assert_eq!(passes.null_event(&sources, &frame()), NullEvent::Added);
        }
        assert_eq!(
            owners(passes.grid()),
            [(130, 100), (131, 90), (132, 80), (134, 50)],
            "xeno overwrote yellow, the weakest"
        );
        assert_eq!(passes.null_event(&sources, &frame()), NullEvent::Added);
        assert_eq!(
            owners(passes.grid()),
            [(130, 100), (131, 90), (132, 80), (133, 10)],
            "yellow's repeat took xeno's slot back"
        );
        let mut coarse_order = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
        for source in &sources.coarse {
            coarse_order.add(source, &frame(), FINE_STEP, &sources.govts);
        }
        assert_eq!(
            owners(&coarse_order),
            [(130, 100), (131, 90), (132, 80), (134, 50)],
            "each place once: xeno keeps it"
        );
    }

    #[test]
    fn sources_taken_again_mid_pass_go_on_after_the_system_walked_last() {
        let mut passes = BorderPasses::new();
        let first = sources(&[source(129, (0, 0), 128), source(131, (100, 0), 129)]);
        passes.coarse(&first, &frame());
        assert_eq!(passes.null_event(&first, &frame()), NullEvent::Cleared);
        assert_eq!(passes.null_event(&first, &frame()), NullEvent::Added);
        // 128 and 130 now contribute too: 128 is behind the cursor, 130
        // ahead of it.
        let retaken = sources(&[
            source(128, (-100, 0), 130),
            source(129, (0, 0), 128),
            source(130, (0, 100), 131),
            source(131, (100, 0), 129),
        ]);
        assert_eq!(passes.null_event(&retaken, &frame()), NullEvent::Added);
        assert_eq!(
            passes.next(),
            Next::Walking {
                after: Some(SystemId(130))
            }
        );
        assert_eq!(passes.grid().slots(114, 155)[0].govt, GovtId(131));
        assert_eq!(passes.null_event(&retaken, &frame()), NullEvent::Added);
        assert_eq!(passes.null_event(&retaken, &frame()), NullEvent::Redraw);
        assert_eq!(
            passes.grid().slots(64, 105)[0].value,
            0,
            "128, behind the cursor, never walked"
        );
    }

    // What is painted.

    #[test]
    fn the_painted_cells_are_the_lattices_with_slot_0_held() {
        let mut passes = BorderPasses::new();
        passes.coarse(&sources(&[source(128, (0, 0), 128)]), &frame());
        let painted = passes.painted();
        assert_eq!(painted.len(), 29, "the blocks' lattice cells only");
        assert!(
            painted
                .iter()
                .all(|cell| cell.x % 8 == 0 && cell.y % 8 == 0 && cell.step == 8)
        );
        assert_eq!(
            painted[0],
            PaintedCell {
                x: 88,
                y: 104,
                step: 8,
                govt: GovtId(128),
                value: 1
            },
            "x outer, y inner"
        );
        assert_eq!((painted[1].x, painted[1].y), (96, 88));
    }

    #[test]
    fn a_cell_whose_slot_0_is_empty_is_skipped_and_the_strongest_slot_wins() {
        let mut passes = BorderPasses::new();
        passes.coarse(&sources(&[]), &frame());
        let slot = |govt, value| Slot {
            govt: GovtId(govt),
            value,
        };
        let e = Slot::EMPTY;
        passes.grid.set(0, 0, [e, e, slot(129, 200), e]);
        passes.grid.set(
            8,
            0,
            [slot(128, 10), slot(129, 30), slot(130, 30), slot(131, 5)],
        );
        passes.grid.set(0, 8, [slot(128, 40), slot(129, 30), e, e]);
        passes.grid.set(1, 0, [slot(132, 90), e, e, e]);
        assert_eq!(
            passes.painted(),
            [
                PaintedCell {
                    x: 0,
                    y: 8,
                    step: 8,
                    govt: GovtId(128),
                    value: 40
                },
                PaintedCell {
                    x: 8,
                    y: 0,
                    step: 8,
                    govt: GovtId(129),
                    value: 30
                },
            ],
            "(0, 0) skipped though slot 2 holds 200, (1, 0) off the lattice; \
             ties to the first"
        );
    }

    #[test]
    fn an_allied_governments_slots_merge_through_the_sources_table() {
        let allied = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
        ]);
        let pair = BorderSources {
            govts: allied,
            ..sources(&[source(128, (0, 0), 128), source(129, (10, 0), 129)])
        };
        let mut passes = BorderPasses::new();
        passes.coarse(&pair, &frame());
        run_fine(&mut passes, &pair);
        let at = passes.grid().slots(114, 105);
        assert_eq!(at[0].govt, GovtId(128), "128's 180 beats 129's 175");
        assert_eq!(at[1], Slot::EMPTY, "allies share a slot");
    }
}
