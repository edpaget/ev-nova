//! The borders' grid: a system's cell, its spread and value, and how a
//! cell's four slots take a value (`_MapPosToInfluenceCoords`
//! @0x114f7-0x11591, `_AddSystemInfluenceToMap` @0x11592-0x119ed).

use super::{BorderSource, GridFrame};
use crate::catalog::GovtId;
use crate::govt::Governments;

/// The radius in cells at a zoom of 1, `Flags2` 0x0002 clear (0xdd1e0).
pub const RADIUS: f64 = 30.72;
/// The radius in cells at a zoom of 1 with `Flags2` 0x0002 (0xdd1d8).
pub const SMALL_RADIUS: f64 = 15.36;
/// What r² − d² is multiplied by, `Flags2` 0x0002 clear (0xdd108).
pub const VALUE_SCALE: f64 = 0.2;
/// What r² − d² is multiplied by with `Flags2` 0x0002 (0xdd1e8).
pub const SMALL_VALUE_SCALE: f64 = 0.4;
/// How many slots a cell has.
pub const SLOTS: usize = 4;

/// The cell of map point `pos` (`_MapPosToInfluenceCoords`
/// @0x114f7-0x11591): trunc(trunc((pos − scroll) / zoom) / 2) +
/// trunc(size / 4) on each axis, the size the map item's, every division
/// toward zero.
#[must_use]
pub fn cell_of(pos: (i16, i16), frame: &GridFrame) -> (i32, i32) {
    let axis = |pos: i16, scroll: i32, size: i32| {
        let offset = f64::from(i32::from(pos) - scroll) / frame.zoom;
        // `cvttsd2si`, then the halving toward zero.
        #[allow(clippy::cast_possible_truncation)] // Grid cells.
        let pixels = offset.trunc() as i32;
        pixels / 2 + size / 4
    };
    (
        axis(pos.0, frame.scroll.0, frame.area.0),
        axis(pos.1, frame.scroll.1, frame.area.1),
    )
}

/// `v` less its remainder by `step`, C's `%` (toward zero), as
/// `_AddSystemInfluenceToMap` snaps a cell and the radius to the lattice
/// (@0x1166a-0x1169f, @0x11703-0x1171b).
#[must_use]
pub fn snap(v: i32, step: i32) -> i32 {
    v - v % step
}

/// The spread's radius in cells (@0x116c2-0x1171b): trunc([`RADIUS`] /
/// zoom), or trunc([`SMALL_RADIUS`] / zoom) when `small`, snapped to the
/// lattice when `step` > 1.
#[must_use]
pub fn radius(zoom: f64, small: bool, step: i32) -> i32 {
    let base = if small { SMALL_RADIUS } else { RADIUS };
    #[allow(clippy::cast_possible_truncation)] // Grid cells.
    let r = (base / zoom).trunc() as i32;
    if step > 1 { snap(r, step) } else { r }
}

/// What a cell `d2` (d²) from the source, within the radius squared
/// `r2`, takes (@0x11809-0x1184f): trunc(((r² − d²) × [`VALUE_SCALE`]) ×
/// zoom), or [`SMALL_VALUE_SCALE`] when `small`, worked in doubles in that
/// order, a value of 0 or less made 1 and one of 256 or more 255.
#[must_use]
pub fn value(r2: i32, d2: i32, small: bool, zoom: f64) -> u8 {
    let scale = if small {
        SMALL_VALUE_SCALE
    } else {
        VALUE_SCALE
    };
    #[allow(clippy::cast_possible_truncation)] // Clamped below.
    let v = ((f64::from(r2 - d2) * scale) * zoom).trunc() as i32;
    u8::try_from(v.clamp(1, 255)).unwrap_or(u8::MAX)
}

/// One of a cell's slots: a government and its value there, 0 for an
/// empty slot. A cleared slot's owner byte is 0, the first government's
/// index: government 128.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    /// Whose value it is.
    pub govt: GovtId,
    /// Its value, 1-255; 0 is empty, whoever the owner.
    pub value: u8,
}

impl Slot {
    /// A cleared slot.
    pub const EMPTY: Self = Self {
        govt: GovtId(128),
        value: 0,
    };
}

/// A cell's `slots` taking `value` of `govt` (@0x1187f-0x11983): the first
/// held slot whose owner `govts` makes an ally of `govt` (the same
/// government included) takes it, and the government, when it is larger,
/// and that ends the cell; otherwise the first empty slot takes it;
/// otherwise the slot of least value (ties to the first) is overwritten,
/// even by a smaller value.
pub fn merge(slots: &mut [Slot; SLOTS], govt: GovtId, value: u8, govts: &Governments) {
    let held = slots
        .iter_mut()
        .find(|slot| slot.value != 0 && govts.allies(Some(govt), Some(slot.govt)));
    if let Some(slot) = held {
        if value > slot.value {
            *slot = Slot { govt, value };
        }
        return;
    }
    let index = slots
        .iter()
        .position(|slot| slot.value == 0)
        .unwrap_or_else(|| {
            let mut weakest = 0;
            for (index, slot) in slots.iter().enumerate() {
                if slot.value < slots[weakest].value {
                    weakest = index;
                }
            }
            weakest
        });
    slots[index] = Slot { govt, value };
}

/// The grid: each cell's four slots, laid out [x][y] as the original's
/// `_influenceOwner` and `_influenceValue` are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfluenceGrid {
    cells: Vec<[Slot; SLOTS]>,
    size: (i32, i32),
}

impl InfluenceGrid {
    /// A grid of `size` cells, every slot empty.
    #[must_use]
    pub fn new(size: (i32, i32)) -> Self {
        let (w, h) = size;
        let count = usize::try_from(w.max(0) * h.max(0)).unwrap_or(0);
        Self {
            cells: vec![[Slot::EMPTY; SLOTS]; count],
            size,
        }
    }

    /// Its size in cells.
    #[must_use]
    pub fn size(&self) -> (i32, i32) {
        self.size
    }

    /// Cell (`x`, `y`)'s slots; empty ones off the grid.
    #[must_use]
    pub fn slots(&self, x: i32, y: i32) -> [Slot; SLOTS] {
        self.index(x, y)
            .map_or([Slot::EMPTY; SLOTS], |index| self.cells[index])
    }

    /// Every slot of every cell cleared, as the open (@0x12440-0x1249e)
    /// and the fine pass's first event (@0x14664-0x146c8) clear them.
    pub fn clear_all(&mut self) {
        self.cells.fill([Slot::EMPTY; SLOTS]);
    }

    /// The slots of every `step`-th cell on each axis cleared, as the
    /// coarse pass starts (@0x11a16-0x11a64).
    pub fn clear_lattice(&mut self, step: i32) {
        for (x, y) in self.lattice(step) {
            self.set(x, y, [Slot::EMPTY; SLOTS]);
        }
    }

    /// Each lattice cell's slots copied over its `step` × `step` block,
    /// within the grid, as the coarse pass ends (@0x11ae0-0x11c1e).
    pub fn fill_blocks(&mut self, step: i32) {
        for (x0, y0) in self.lattice(step) {
            let slots = self.slots(x0, y0);
            for x in x0..x0 + step {
                for y in y0..y0 + step {
                    self.set(x, y, slots);
                }
            }
        }
    }

    /// `_AddSystemInfluenceToMap` after its flag tests (@0x1162d-0x119da),
    /// which the session applies: `source`'s cell in `frame`, snapped to
    /// the lattice of `step`, and its radius; nothing, and false, when the
    /// cell is more than the radius off the grid on either axis (−r ..=
    /// r + the grid's cells, @0x11733-0x1175f, the original's r + 0x100);
    /// otherwise every lattice cell on the grid within the radius merges
    /// its value ([`merge`]), and true, whether any was written or not.
    pub fn add(
        &mut self,
        source: &BorderSource,
        frame: &GridFrame,
        step: i32,
        govts: &Governments,
    ) -> bool {
        let step = step.max(1);
        let (mut cx, mut cy) = cell_of(source.pos, frame);
        if step > 1 {
            cx = snap(cx, step);
            cy = snap(cy, step);
        }
        let r = radius(frame.zoom, source.small, step);
        let (w, h) = frame.cells;
        if cx < -r || cx > r + w || cy < -r || cy > r + h {
            return false;
        }
        let r2 = r * r;
        let lattice = |c: i32| (c - r..=c + r).step_by(step_of(step));
        for x in lattice(cx) {
            for y in lattice(cy) {
                let Some(index) = self.index(x, y) else {
                    continue;
                };
                let d2 = (x - cx).pow(2) + (y - cy).pow(2);
                if d2 <= r2 {
                    let value = value(r2, d2, source.small, frame.zoom);
                    merge(&mut self.cells[index], source.govt, value, govts);
                }
            }
        }
        true
    }

    /// Cell (`x`, `y`)'s place in `cells`, if it is on the grid.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (w, h) = self.size;
        if (0..w).contains(&x) && (0..h).contains(&y) {
            usize::try_from(x * h + y).ok()
        } else {
            None
        }
    }

    /// Sets cell (`x`, `y`)'s slots, if it is on the grid.
    pub(super) fn set(&mut self, x: i32, y: i32, slots: [Slot; SLOTS]) {
        if let Some(index) = self.index(x, y) {
            self.cells[index] = slots;
        }
    }

    /// Every `step`-th cell on each axis, x outer, y inner.
    fn lattice(&self, step: i32) -> Vec<(i32, i32)> {
        let (w, h) = self.size;
        let step = step_of(step);
        (0..w)
            .step_by(step)
            .flat_map(|x| (0..h).step_by(step).map(move |y| (x, y)))
            .collect()
    }
}

/// `step` as a stride, at least 1, as `_RecalcMapInfluence` makes a step
/// of 0 or less 1 (@0x119ff).
fn step_of(step: i32) -> usize {
    usize::try_from(step).unwrap_or(1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtRecord, SystemId};
    use crate::testkit::govt;

    /// Item 3's frame at `zoom`, looking at (0, 0).
    fn frame(zoom: f64) -> GridFrame {
        GridFrame {
            zoom,
            scroll: (0, 0),
            area: (458, 420),
            cells: GridFrame::IN_FLIGHT_CELLS,
        }
    }

    // The cell.

    #[test]
    fn the_system_at_the_scroll_is_cell_114_105_at_each_zoom() {
        for zoom in [0.5625, 1.0] {
            assert_eq!(cell_of((0, 0), &frame(zoom)), (114, 105), "{zoom}");
            let scrolled = GridFrame {
                scroll: (-40, 70),
                ..frame(zoom)
            };
            assert_eq!(cell_of((-40, 70), &scrolled), (114, 105), "{zoom}");
        }
    }

    #[test]
    fn an_offset_is_divided_by_the_zoom_then_halved() {
        // 100 / 0.5625 = 177.7 → 177 → 88; −37 / 0.5625 = −65.7 → −65 →
        // −32.
        assert_eq!(cell_of((100, -37), &frame(0.5625)), (202, 73));
        assert_eq!(cell_of((100, -37), &frame(1.0)), (164, 87));
    }

    #[test]
    fn a_negative_offset_truncates_toward_zero_at_both_divisions() {
        // −1 / 0.5625 = −1.7 → −1, not −2; −1 / 2 → 0, not −1.
        assert_eq!(cell_of((-1, 0), &frame(0.5625)), (114, 105));
        // −3 / 2 → −1, not −2.
        assert_eq!(cell_of((-3, -3), &frame(1.0)), (113, 104));
        // The offsets are the area's quarters, truncated: 458 / 4 = 114.5.
        let odd = GridFrame {
            area: (7, 10),
            ..frame(1.0)
        };
        assert_eq!(cell_of((0, 0), &odd), (1, 2));
    }

    // The radius and the value.

    #[test]
    fn the_radius_is_30_72_or_15_36_over_the_zoom_truncated() {
        assert_eq!(radius(0.5625, false, 1), 54);
        assert_eq!(radius(0.5625, true, 1), 27);
        assert_eq!(radius(1.0, false, 1), 30);
        assert_eq!(radius(1.0, true, 1), 15);
    }

    #[test]
    fn at_step_8_the_radius_is_snapped_down() {
        assert_eq!(radius(0.5625, false, 8), 48);
        assert_eq!(radius(0.5625, true, 8), 24);
        assert_eq!(radius(1.0, false, 8), 24);
        assert_eq!(radius(1.0, true, 8), 8);
    }

    #[test]
    fn a_snap_loses_the_remainder_toward_zero() {
        assert_eq!(snap(13, 8), 8);
        assert_eq!(snap(16, 8), 16);
        assert_eq!(snap(-5, 8), 0);
        assert_eq!(snap(-13, 8), -8);
        assert_eq!(snap(7, 1), 7);
    }

    #[test]
    fn the_centre_value_is_clamped_to_255_and_the_edge_is_1() {
        // 2916 × 0.2 × 0.5625 = 328.05.
        assert_eq!(value(2916, 0, false, 0.5625), 255);
        // 729 × 0.4 × 0.5625 = 164.025.
        assert_eq!(value(729, 0, true, 0.5625), 164);
        assert_eq!(value(900, 0, false, 1.0), 180);
        assert_eq!(value(225, 0, true, 1.0), 90);
        assert_eq!(value(900, 900, false, 1.0), 1, "d² = r²: 0 made 1");
        assert_eq!(value(225, 225, true, 0.5625), 1);
    }

    #[test]
    fn a_mid_cell_takes_its_share_at_each_zoom() {
        // (2916 − 2000) × 0.2 × 0.5625 = 103.05.
        assert_eq!(value(2916, 2000, false, 0.5625), 103);
        // (729 − 400) × 0.4 × 0.5625 = 74.025.
        assert_eq!(value(729, 400, true, 0.5625), 74);
        // (900 − 413) × 0.2 = 97.4.
        assert_eq!(value(900, 413, false, 1.0), 97);
        // 256 or more is 255, 255 itself kept.
        assert_eq!(value(1280, 0, false, 1.0), 255);
        assert_eq!(value(1275, 0, false, 1.0), 255);
        assert_eq!(value(1270, 0, false, 1.0), 254);
    }

    // The bounds.

    const FED: GovtId = GovtId(128);

    fn source(pos: (i16, i16)) -> BorderSource {
        BorderSource {
            system: SystemId(128),
            pos,
            govt: FED,
            small: false,
        }
    }

    fn held(grid: &InfluenceGrid) -> usize {
        let (w, h) = grid.size();
        (0..w)
            .flat_map(|x| (0..h).map(move |y| (x, y)))
            .filter(|&(x, y)| grid.slots(x, y).iter().any(|slot| slot.value != 0))
            .count()
    }

    #[test]
    fn a_source_up_to_the_radius_past_the_grid_adds_even_with_nothing_written() {
        let govts = Governments::empty();
        // At zoom 1, r 30: x 344 is cell 114 + 172 = 286 = r + 256.
        for pos in [(344, 0), (0, 362)] {
            let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
            assert!(grid.add(&source(pos), &frame(1.0), 1, &govts), "{pos:?}");
            assert_eq!(held(&grid), 0, "{pos:?}: every cell off the grid");
        }
        for pos in [(346, 0), (0, 364)] {
            let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
            assert!(!grid.add(&source(pos), &frame(1.0), 1, &govts), "{pos:?}");
        }
    }

    #[test]
    fn a_source_up_to_the_radius_before_the_grid_adds_and_one_further_does_not() {
        let govts = Governments::empty();
        // x −288 is cell 114 − 144 = −30 = −r: (0, cy) is r away.
        for (pos, cell) in [((-288, 0), (0, 105)), ((0, -270), (114, 0))] {
            let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
            assert!(grid.add(&source(pos), &frame(1.0), 1, &govts), "{pos:?}");
            assert_eq!(held(&grid), 1, "{pos:?}");
            assert_eq!(grid.slots(cell.0, cell.1)[0].value, 1, "{pos:?}");
        }
        for pos in [(-290, 0), (0, -272)] {
            let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
            assert!(!grid.add(&source(pos), &frame(1.0), 1, &govts), "{pos:?}");
            assert_eq!(held(&grid), 0, "{pos:?}");
        }
    }

    #[test]
    fn a_source_writes_every_cell_within_its_radius_on_the_grid() {
        let govts = Governments::empty();
        let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
        let small = BorderSource {
            small: true,
            ..source((0, 0))
        };
        assert!(grid.add(&small, &frame(1.0), 1, &govts));
        // r 15: the cells with dx² + dy² ≤ 225.
        let within = (-15i32..=15)
            .flat_map(|dx| (-15i32..=15).map(move |dy| dx * dx + dy * dy))
            .filter(|&d2| d2 <= 225)
            .count();
        assert_eq!(held(&grid), within);
        assert_eq!(
            grid.slots(114, 105)[0],
            Slot {
                govt: FED,
                value: 90
            }
        );
        assert_eq!(grid.slots(114 + 15, 105)[0].value, 1);
        assert_eq!(grid.slots(114 + 16, 105)[0].value, 0);
        assert_eq!(grid.slots(114 + 9, 105 + 12)[0].value, 1, "d² 225");
        assert_eq!(grid.slots(114 + 10, 105 + 12)[0].value, 0, "d² 244");
    }

    #[test]
    fn at_step_8_only_the_snapped_lattice_is_written() {
        let govts = Governments::empty();
        let mut grid = InfluenceGrid::new(GridFrame::IN_FLIGHT_CELLS);
        // Cell (114, 105) snaps to (112, 104); r 24 at zoom 1.
        assert!(grid.add(&source((0, 0)), &frame(1.0), 8, &govts));
        assert_eq!(grid.slots(112, 104)[0].value, 115, "576 × 0.2 = 115.2");
        assert_eq!(grid.slots(113, 104)[0].value, 0, "off the lattice");
        assert_eq!(grid.slots(112 + 24, 104)[0].value, 1);
        assert_eq!(grid.slots(112 + 16, 104 + 16)[0].value, 12, "64 × 0.2");
        assert_eq!(grid.slots(112 + 24, 104 + 8)[0].value, 0, "d² 640");
        let lattice = (-3i32..=3)
            .flat_map(|dx| (-3i32..=3).map(move |dy| dx * dx + dy * dy))
            .filter(|&d2| d2 <= 9)
            .count();
        assert_eq!(held(&grid), lattice);
    }

    #[test]
    fn a_grid_off_cell_reads_empty() {
        let mut grid = InfluenceGrid::new((4, 3));
        assert_eq!(grid.size(), (4, 3));
        assert_eq!(grid.slots(4, 0), [Slot::EMPTY; SLOTS]);
        assert_eq!(grid.slots(0, 3), [Slot::EMPTY; SLOTS]);
        assert_eq!(grid.slots(-1, 0), [Slot::EMPTY; SLOTS]);
        assert_eq!(grid.slots(0, -1), [Slot::EMPTY; SLOTS]);
        let corner = GridFrame {
            zoom: 1.0,
            scroll: (0, 0),
            area: (0, 0),
            cells: (4, 3),
        };
        // At the area's (0, 0) the cell is (0, 0), r 30 covering all.
        assert!(grid.add(&source((6, 4)), &corner, 1, &Governments::empty()));
        assert_eq!(grid.slots(3, 2)[0].value, 180, "the source's cell (3, 2)");
        assert_eq!(grid.slots(0, 0)[0].value, 177, "(900 − 13) × 0.2 = 177.4");
        assert_eq!(held(&grid), 12);
    }

    // The clears and the blocks.

    fn filled(size: (i32, i32)) -> InfluenceGrid {
        let mut grid = InfluenceGrid::new(size);
        let frame = GridFrame {
            zoom: 0.1,
            scroll: (0, 0),
            area: (0, 0),
            cells: size,
        };
        assert!(grid.add(&source((0, 0)), &frame, 1, &Governments::empty()));
        grid
    }

    #[test]
    fn clearing_the_lattice_clears_every_stepth_cell_only() {
        let mut grid = filled((20, 20));
        assert_eq!(held(&grid), 400, "r 307 covers it");
        grid.clear_lattice(8);
        for x in 0..20 {
            for y in 0..20 {
                let cleared = x % 8 == 0 && y % 8 == 0;
                assert_eq!(grid.slots(x, y)[0].value == 0, cleared, "({x}, {y})");
            }
        }
        grid.clear_all();
        assert_eq!(held(&grid), 0);
    }

    #[test]
    fn the_blocks_copy_each_lattice_cells_slots_within_the_grid() {
        let mut grid = InfluenceGrid::new((12, 10));
        let frame = GridFrame {
            zoom: 1.0,
            scroll: (0, 0),
            area: (0, 0),
            cells: (12, 10),
        };
        let govts = Governments::empty();
        assert!(grid.add(&source((0, 0)), &frame, 8, &govts));
        let lattice = |x, y| grid.slots(x, y);
        let (a, b) = (lattice(0, 0), lattice(8, 8));
        assert_ne!(a[0].value, 0);
        assert_ne!(b[0].value, a[0].value);
        assert_eq!(grid.slots(1, 0)[0].value, 0, "not yet filled");
        grid.fill_blocks(8);
        for x in 0..12 {
            for y in 0..10 {
                let from = grid.slots(x - x % 8, y - y % 8);
                assert_eq!(grid.slots(x, y), from, "({x}, {y})");
            }
        }
        assert_eq!(grid.slots(11, 9), b, "the corner block, clipped");
        assert_eq!(grid.slots(7, 7), a);
    }

    // The slots.

    /// The Federation (128, class 1) allied with class 2, which 129 is;
    /// 130-134 allied with none.
    fn table() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
            govt(130),
            govt(131),
            govt(132),
            govt(133),
            govt(134),
        ])
    }

    fn slot(govt: i16, value: u8) -> Slot {
        Slot {
            govt: GovtId(govt),
            value,
        }
    }

    fn merged(mut slots: [Slot; SLOTS], govt: i16, value: u8) -> [Slot; SLOTS] {
        merge(&mut slots, GovtId(govt), value, &table());
        slots
    }

    const E: Slot = Slot::EMPTY;

    #[test]
    fn an_empty_cell_takes_slot_0() {
        assert_eq!(merged([E; 4], 130, 50), [slot(130, 50), E, E, E]);
    }

    #[test]
    fn the_same_governments_smaller_slot_is_raised_and_the_others_left() {
        let cell = [slot(131, 70), slot(130, 20), slot(132, 90), E];
        assert_eq!(
            merged(cell, 130, 50),
            [slot(131, 70), slot(130, 50), slot(132, 90), E]
        );
    }

    #[test]
    fn the_same_governments_larger_slot_is_kept_and_no_empty_slot_written() {
        let cell = [slot(130, 80), E, E, E];
        assert_eq!(merged(cell, 130, 50), cell);
        assert_eq!(merged(cell, 130, 80), cell, "equal is not larger");
    }

    #[test]
    fn an_allied_slot_before_the_systems_own_is_taken_only_when_larger() {
        let cell = [slot(131, 10), slot(129, 40), slot(128, 30), E];
        assert_eq!(
            merged(cell, 128, 50),
            [slot(131, 10), slot(128, 50), slot(128, 30), E],
            "129 an ally of 128, first: it becomes 128's"
        );
        assert_eq!(merged(cell, 128, 35), cell, "35 below 129's 40");
    }

    #[test]
    fn an_unallied_cell_with_two_slots_held_takes_slot_2() {
        let cell = [slot(131, 10), slot(132, 40), E, E];
        assert_eq!(
            merged(cell, 130, 5),
            [slot(131, 10), slot(132, 40), slot(130, 5), E]
        );
    }

    #[test]
    fn with_four_unallied_slots_the_weakest_is_overwritten_ties_to_the_first() {
        let cell = [slot(131, 40), slot(132, 20), slot(133, 20), slot(134, 30)];
        assert_eq!(
            merged(cell, 130, 5),
            [slot(131, 40), slot(130, 5), slot(133, 20), slot(134, 30)],
            "even by a smaller value"
        );
        let cell = [slot(131, 10), slot(132, 20), slot(133, 30), slot(134, 40)];
        assert_eq!(
            merged(cell, 130, 255),
            [slot(130, 255), slot(132, 20), slot(133, 30), slot(134, 40)]
        );
        let cell = [slot(131, 40), slot(132, 30), slot(133, 20), slot(134, 10)];
        assert_eq!(
            merged(cell, 130, 1),
            [slot(131, 40), slot(132, 30), slot(133, 20), slot(130, 1)]
        );
    }

    #[test]
    fn a_slot_of_value_0_counts_as_empty_whoever_owns_it() {
        let cell = [slot(131, 10), E, slot(129, 20), E];
        assert_eq!(
            merged(cell, 128, 5),
            cell,
            "slot 1, empty, isn't 128's though its owner byte is: 129's 20 ends it"
        );
        let cell = [slot(131, 10), slot(132, 0), slot(130, 9), E];
        assert_eq!(
            merged(cell, 130, 50),
            [slot(131, 10), slot(132, 0), slot(130, 50), E],
            "the held slot of its own comes first"
        );
    }
}
