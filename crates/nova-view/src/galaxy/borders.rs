//! The maps' government borders, as `_DrawMapInfluence` paints them
//! (@0xe360-0xe567): each lattice cell [`nova_sim::map_borders`] works
//! out, an opaque square of 2 × step pixels in its strongest government's
//! colour at half the cell's value.
//!
//! **A cell's square** runs from (left + 2x, top + 2y) of the map item to
//! 2 × step pixels on; a square wholly outside the item is skipped, each
//! edge compared inclusively (@0xe405-0xe433), and any other is clamped to
//! the item (@0xe439-0xe48d), so one only touching an edge paints nothing.
//! The grid's offset is a quarter of the item's size, truncated, so on the
//! in-flight maps the system at the scroll is cell (114, 105), drawn at x
//! 221 + 228 = 449, a pixel left of where `_DrawMap` puts it, 450.
//!
//! **Its colour** is the government's `gövt` `Color` (+0x7c/+0x7e/+0x80,
//! 8 bits a channel, @0x7bd38-0x7bd58), each channel × (value × 0.5),
//! truncated, as a 16-bit `RGBColor` component (@0xe4b6-0xe50a), which a
//! direct screen shows as its high byte ([`shade`]): about half the colour
//! at 255. A government with no colour reads 0. Each square is its own
//! `PaintRect`, as the original paints them.

use std::collections::BTreeMap;

use nova_sim::RuleSource;
use nova_sim::map_borders::{BorderPasses, BorderSources, GridFrame, NullEvent, PaintedCell};

use super::catalog::GovtId;
use super::dialog::{MAP_ITEM, MapDialog, map_area};
use super::update::MapUpdate;
use super::view::{Bounds, MAP_HEIGHT, MAP_WIDTH, MapView};
use crate::draw::fill_rect;
use crate::flight::radar::IRect;
use crate::{Color, DrawList, Point};

/// What a cell's value is halved by before it scales a channel (0.5,
/// 0xdd128).
pub const SHADE: f64 = 0.5;

/// `channel`, an 8-bit `gövt` colour channel, at a cell's `value`: the
/// high byte of the 16-bit component trunc(channel × (value × 0.5)).
#[must_use]
pub fn shade(channel: u8, value: u8) -> u8 {
    let component = (f64::from(channel) * (f64::from(value) * SHADE)).trunc();
    // At most 255 × 127.5: a 16-bit component, shown as its high byte.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let component = component as u16;
    component.to_be_bytes()[0]
}

/// Paints `cells` as `_DrawMapInfluence` does (see the module docs): each
/// at `origin` + 2 × its place, clamped to `area` or skipped outside it,
/// in its government's colour from `colours` (a 24-bit `00RRGGBB` value)
/// shaded by its value.
pub fn draw_cells(
    list: &mut DrawList,
    cells: &[PaintedCell],
    origin: (i32, i32),
    area: IRect,
    colours: &BTreeMap<GovtId, u32>,
) {
    for cell in cells {
        let left = origin.0 + 2 * cell.x;
        let top = origin.1 + 2 * cell.y;
        let side = 2 * cell.step;
        let (right, bottom) = (left + side, top + side);
        // A square wholly outside is skipped (@0xe405-0xe433); clamped, it
        // would be empty, as one only touching an edge is, so the one test
        // below covers both.
        let (left, top) = (left.max(area.left), top.max(area.top));
        let (right, bottom) = (right.min(area.right), bottom.min(area.bottom));
        if left >= right || top >= bottom {
            continue;
        }
        let raw = colours.get(&cell.govt).copied().unwrap_or(0);
        let [_, r, g, b] = raw.to_be_bytes();
        let color = Color::rgba(
            shade(r, cell.value),
            shade(g, cell.value),
            shade(b, cell.value),
            255,
        );
        #[allow(clippy::cast_precision_loss)] // Screen pixels.
        let square = Bounds {
            min: Point::new(left as f32, top as f32),
            max: Point::new(right as f32, bottom as f32),
        };
        fill_rect(list, square, color);
    }
}

/// A map's borders: the grid and its passes, the sources they add, and
/// the cells as last drawn by a full update.
///
/// **In flight** the owner shows the sources
/// ([`GalaxyMap::show_border_sources`](super::GalaxyMap::show_border_sources)),
/// and the map runs the passes at the original's trigger points (see the
/// [`map`](super::map) module's "The borders"), the fine pass's null
/// events on its ticks as [`RuleKey::MapBorderPace`] says. Each full
/// update of item 3 ([`MapDialog::take_fresh`]) takes the cells then, as
/// the original's deferred update event draws the grid as the handler
/// left it ([`MapBorders::settle`]).
///
/// **The viewer** has no session and no original to follow: it counts
/// every system with a government
/// ([`BorderSources::unfiltered`]), runs the coarse pass whenever its
/// view changes, the fine pass on its ticks at the engine's pace, and
/// takes the cells at each pass's end.
///
/// [`RuleKey::MapBorderPace`]: nova_sim::RuleKey::MapBorderPace
#[derive(Clone, Debug)]
pub struct MapBorders {
    passes: BorderPasses,
    sources: BorderSources,
    /// The cells as last taken.
    shown: Vec<PaintedCell>,
    /// [`RuleKey::MapBorderPace`](nova_sim::RuleKey::MapBorderPace).
    pace: RuleSource,
    /// [`RuleKey::MapFindBorders`](nova_sim::RuleKey::MapFindBorders).
    find: RuleSource,
    /// Whether the borders were shown as the in-flight map last opened.
    at_open: bool,
    /// Whether the viewer's are shown.
    viewer_on: bool,
    /// The viewer's frame at its last coarse pass.
    last_frame: Option<GridFrame>,
}

impl Default for MapBorders {
    fn default() -> Self {
        Self::new()
    }
}

impl MapBorders {
    /// No sources, no grid, the engine's readings, and the viewer's on.
    #[must_use]
    pub fn new() -> Self {
        Self {
            passes: BorderPasses::new(),
            sources: BorderSources::default(),
            shown: Vec::new(),
            pace: RuleSource::Engine,
            find: RuleSource::Engine,
            at_open: true,
            viewer_on: true,
            last_frame: None,
        }
    }

    /// The passes and their grid.
    #[must_use]
    pub fn passes(&self) -> &BorderPasses {
        &self.passes
    }

    /// The cells as last taken: what a full update draws.
    #[must_use]
    pub fn shown(&self) -> &[PaintedCell] {
        &self.shown
    }

    /// Takes `sources` for the passes from now on.
    pub fn show_sources(&mut self, sources: BorderSources) {
        self.sources = sources;
    }

    /// The sources shown ([`MapBorders::show_sources`]).
    #[must_use]
    pub fn sources(&self) -> &BorderSources {
        &self.sources
    }

    /// Sets the readings of [`RuleKey::MapBorderPace`] and
    /// [`RuleKey::MapFindBorders`].
    ///
    /// [`RuleKey::MapBorderPace`]: nova_sim::RuleKey::MapBorderPace
    /// [`RuleKey::MapFindBorders`]: nova_sim::RuleKey::MapFindBorders
    pub fn set_rules(&mut self, pace: RuleSource, find: RuleSource) {
        self.pace = pace;
        self.find = find;
    }

    /// Where an in-flight map's grid lies: its zoom and scroll, item 3's
    /// 458 × 420 and the 256 × 256 cells.
    #[must_use]
    pub fn dialog_frame(dialog: &MapDialog) -> GridFrame {
        let place = dialog.place();
        GridFrame {
            zoom: place.zoom(),
            scroll: place.scroll(),
            area: (
                MAP_ITEM.right - MAP_ITEM.left,
                MAP_ITEM.bottom - MAP_ITEM.top,
            ),
            cells: GridFrame::IN_FLIGHT_CELLS,
        }
    }

    /// The map opening (`_DoSystemMap` @0x1233c-0x12655): the setting kept
    /// to compare at the close, the grid cleared, and the coarse pass run
    /// when the borders are on.
    pub fn opened(&mut self, dialog: &MapDialog) {
        self.at_open = dialog.borders_shown();
        self.passes.open();
        self.recompute(dialog);
    }

    /// The coarse pass, when `dialog` shows the borders: an arrow, a zoom
    /// step taken, a drag's end, Space, or the borders turned on.
    pub fn recompute(&mut self, dialog: &MapDialog) {
        if dialog.borders_shown() {
            self.passes
                .coarse(&self.sources, &Self::dialog_frame(dialog));
        }
    }

    /// Item 9, the Borders button or B (@0x126f0-0x12803): the setting
    /// flipped, items 9 and 3 redrawn (@0x12756, @0x127ae), and the coarse
    /// pass run when the borders are now on (@0x127b3-0x127fe).
    pub fn toggle(&mut self, dialog: &mut MapDialog) {
        dialog.set_borders_shown(!dialog.borders_shown());
        dialog.redraw();
        self.recompute(dialog);
    }

    /// Find's centring on a system: by [`RuleKey::MapFindBorders`]'
    /// engine reading nothing, as `_DoSystemSearch` runs no pass; by the
    /// other, the coarse pass and item 3 redrawn, as Space does.
    ///
    /// [`RuleKey::MapFindBorders`]: nova_sim::RuleKey::MapFindBorders
    pub fn found(&mut self, dialog: &mut MapDialog) {
        if self.find == RuleSource::Bible {
            dialog.redraw();
            self.recompute(dialog);
        }
    }

    /// The map's tick while the borders are on: the fine pass's null
    /// events, back to back to the pass's end by
    /// [`RuleKey::MapBorderPace`]'s engine reading, or one by the other;
    /// its redraw redraws item 3 (@0x1479a), which repairs a kept partial
    /// frame too.
    ///
    /// [`RuleKey::MapBorderPace`]: nova_sim::RuleKey::MapBorderPace
    pub fn tick(&mut self, dialog: &mut MapDialog) {
        if !dialog.borders_shown() {
            return;
        }
        let frame = Self::dialog_frame(dialog);
        if self.null_events(&frame) {
            dialog.redraw();
        }
    }

    /// The null events a tick runs by the pace in `frame`: whether one
    /// asked for the redraw.
    fn null_events(&mut self, frame: &GridFrame) -> bool {
        loop {
            match self.passes.null_event(&self.sources, frame) {
                NullEvent::Redraw => return true,
                NullEvent::Cleared | NullEvent::Added if self.pace == RuleSource::Engine => {}
                NullEvent::Nothing | NullEvent::Cleared | NullEvent::Added => return false,
            }
        }
    }

    /// Takes the cells when item 3 was redrawn since the last settle: the
    /// grid's when `dialog` shows the borders, none otherwise.
    pub fn settle(&mut self, dialog: &mut MapDialog) {
        if dialog.take_fresh() {
            self.shown = if dialog.borders_shown() {
                self.passes.painted()
            } else {
                Vec::new()
            };
        }
    }

    /// Whether the setting differs from the one the map opened with: what
    /// the close saves (@0x13c3a).
    #[must_use]
    pub fn changed_since_open(&self, dialog: &MapDialog) -> bool {
        dialog.borders_shown() != self.at_open
    }

    /// Draws the cells taken into item 3, in `colours`, unless `dialog`
    /// shows the partial update (@0xf838-0xf840).
    pub fn draw(&self, dialog: &MapDialog, list: &mut DrawList, colours: &BTreeMap<GovtId, u32>) {
        if dialog.update() == MapUpdate::Partial {
            return;
        }
        let area = map_area();
        #[allow(clippy::cast_possible_truncation)] // Screen pixels.
        let origin = (area.min.x as i32, area.min.y as i32);
        draw_cells(list, &self.shown, origin, IRect::from_bounds(area), colours);
    }

    /// The viewer's grid: its zoom, its centre truncated toward zero as
    /// the scroll, and its whole map area, 1024 × 608, in 512 × 304 cells.
    #[must_use]
    pub fn viewer_frame(view: &MapView) -> GridFrame {
        let centre = view.center();
        #[allow(clippy::cast_possible_truncation)] // Galaxy units and pixels.
        let frame = GridFrame {
            zoom: view.transform().zoom(),
            scroll: (centre.x.trunc() as i32, centre.y.trunc() as i32),
            area: (MAP_WIDTH as i32, MAP_HEIGHT as i32),
            cells: ((MAP_WIDTH / 2.0) as i32, (MAP_HEIGHT / 2.0) as i32),
        };
        frame
    }

    /// Whether the viewer's borders are on.
    #[must_use]
    pub fn viewer_on(&self) -> bool {
        self.viewer_on
    }

    /// The viewer settling: with the borders on, the coarse pass and its
    /// cells taken when `frame` differs from the last pass's.
    pub fn settle_viewer(&mut self, frame: GridFrame) {
        if self.viewer_on && self.last_frame != Some(frame) {
            self.passes.coarse(&self.sources, &frame);
            self.last_frame = Some(frame);
            self.shown = self.passes.painted();
        }
    }

    /// The viewer's tick: with the borders on, the fine pass by the
    /// engine's pace, its cells taken at its end.
    pub fn tick_viewer(&mut self, frame: GridFrame) {
        if self.viewer_on && self.null_events(&frame) {
            self.shown = self.passes.painted();
        }
    }

    /// B on the viewer: its borders flipped, none drawn while off, and
    /// the coarse pass at the next settle once on.
    pub fn toggle_viewer(&mut self) {
        self.viewer_on = !self.viewer_on;
        self.last_frame = None;
        if !self.viewer_on {
            self.shown.clear();
        }
    }

    /// Draws the viewer's cells over its whole map area, from its
    /// top-left corner.
    pub fn draw_viewer(&self, list: &mut DrawList, colours: &BTreeMap<GovtId, u32>) {
        #[allow(clippy::cast_possible_truncation)] // Screen pixels.
        let area = IRect::new(0, 0, MAP_WIDTH as i32, MAP_HEIGHT as i32);
        draw_cells(list, &self.shown, (0, 0), area, colours);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;
    use crate::galaxy::dialog::map_area;

    /// The rects `list` fills, as (left, top, right, bottom, colour).
    pub(crate) fn rects(list: &DrawList) -> Vec<(f32, f32, f32, f32, Color)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if from.y == to.y => Some((
                    from.x.min(to.x),
                    from.y - width / 2.0,
                    from.x.max(to.x),
                    from.y + width / 2.0,
                    color,
                )),
                _ => None,
            })
            .collect()
    }

    /// The in-flight maps' item 3, (221, 136)-(679, 556).
    fn item_3() -> IRect {
        IRect::from_bounds(map_area())
    }

    const FED: GovtId = GovtId(128);

    fn cell(x: i32, y: i32, step: i32, value: u8) -> PaintedCell {
        PaintedCell {
            x,
            y,
            step,
            govt: FED,
            value,
        }
    }

    fn colours() -> BTreeMap<GovtId, u32> {
        BTreeMap::from([(FED, 0x00CF_FF10)])
    }

    fn drawn(cells: &[PaintedCell]) -> Vec<(f32, f32, f32, f32, Color)> {
        let mut list = DrawList::new();
        draw_cells(&mut list, cells, (221, 136), item_3(), &colours());
        rects(&list)
    }

    #[test]
    fn a_channel_is_shaded_by_half_the_value_and_shown_as_its_high_byte() {
        // 207 × 127.5 = 26 392.5: 26 392, 0x6718.
        assert_eq!(shade(0xcf, 255), 103);
        assert_eq!(shade(255, 255), 127, "32 512, 0x7f00");
        assert_eq!(shade(255, 1), 0, "127, under a byte");
        assert_eq!(shade(0, 255), 0);
        // 255 × 64 = 16 320, 0x3fc0.
        assert_eq!(shade(255, 128), 63);
        // 16 × 101.5 = 1624, 0x0658.
        assert_eq!(shade(16, 203), 6);
    }

    #[test]
    fn a_fine_cell_is_a_2_pixel_square_a_pixel_left_of_the_maps_centre() {
        assert_eq!(
            drawn(&[cell(114, 105, 1, 255)]),
            [(449.0, 346.0, 451.0, 348.0, Color::rgba(103, 127, 7, 255))]
        );
        assert_eq!(
            crate::galaxy::dialog::map_centre(),
            Point::new(450.0, 346.0)
        );
    }

    #[test]
    fn a_coarse_cell_is_a_16_pixel_square() {
        assert_eq!(
            drawn(&[cell(112, 104, 8, 128)]),
            [(445.0, 344.0, 461.0, 360.0, Color::rgba(51, 63, 4, 255))]
        );
    }

    #[test]
    fn a_square_across_an_edge_is_clamped_to_item_3() {
        // x 221 + 2 × 224 = 669 to 685, past the right edge at 679; y 136
        // − 4 to 12 past the top.
        assert_eq!(
            drawn(&[cell(224, 0, 8, 255), cell(0, -2, 8, 255)]),
            [
                (669.0, 136.0, 679.0, 152.0, Color::rgba(103, 127, 7, 255)),
                (221.0, 136.0, 237.0, 148.0, Color::rgba(103, 127, 7, 255)),
            ]
        );
    }

    #[test]
    fn a_square_wholly_outside_is_skipped_and_one_touching_an_edge_paints_nothing() {
        // 221 + 2 × 232 = 685: past 679. 221 + 2 × 229 = 679: touching.
        let mut list = DrawList::new();
        let cells = [
            cell(232, 0, 1, 255),
            cell(229, 0, 1, 255),
            cell(0, 210, 1, 255),
            cell(-1, 0, 1, 255),
            cell(0, -1, 1, 255),
            cell(-2, 0, 1, 255),
            cell(0, -2, 1, 255),
        ];
        draw_cells(&mut list, &cells, (221, 136), item_3(), &colours());
        assert!(list.is_empty(), "{:?}", rects(&list));
    }

    #[test]
    fn each_cell_is_its_own_rect_in_its_governments_colour() {
        let mut colours = colours();
        colours.insert(GovtId(129), 0x0000_00FF);
        let mut list = DrawList::new();
        let cells = [
            cell(0, 0, 1, 255),
            cell(0, 1, 1, 255),
            PaintedCell {
                govt: GovtId(129),
                ..cell(0, 2, 1, 255)
            },
            PaintedCell {
                govt: GovtId(130),
                ..cell(0, 3, 1, 255)
            },
        ];
        draw_cells(&mut list, &cells, (221, 136), item_3(), &colours);
        let fed = Color::rgba(103, 127, 7, 255);
        assert_eq!(
            rects(&list),
            [
                (221.0, 136.0, 223.0, 138.0, fed),
                (221.0, 138.0, 223.0, 140.0, fed),
                (221.0, 140.0, 223.0, 142.0, Color::rgba(0, 0, 127, 255)),
                (221.0, 142.0, 223.0, 144.0, Color::BLACK),
            ],
            "one PaintRect a cell; 130 has no colour"
        );
    }
}

/// The borders on the maps themselves, driven through [`GalaxyMap`]
/// (`super::map`) as their owners drive them.
///
/// [`GalaxyMap`]: super::map::GalaxyMap
#[cfg(test)]
mod on_the_maps {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use nova_sim::map_borders::{BorderSource, BorderSources, Next, PaintedCell};
    use nova_sim::map_search::SearchOutcome;
    use nova_sim::{Governments, RuleSource};

    use super::draw_cells;
    use crate::flight::radar::IRect;
    use crate::galaxy::buttons::{
        ButtonState, HIDE_BORDERS_LABEL, MapButton, MapButtons, SHOW_BORDERS_LABEL,
    };
    use crate::galaxy::catalog::{Galaxy, GovtId, SystemId};
    use crate::galaxy::click::PressAnswer;
    use crate::galaxy::dialog::{DIALOG_ORIGIN, draw_frame, map_area};
    use crate::galaxy::map::{GalaxyMap, MapMarks};
    use crate::galaxy::testkit::{BLUE, FakeCatalog, galaxy};
    use crate::galaxy::update::MapUpdate;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;
    use crate::{DrawCommand, DrawList, Input, Key, MouseButton, Point, Screen};

    const TICK: std::time::Duration = std::time::Duration::from_millis(16);

    /// Alpha, at item 3's centre.
    const ALPHA: Point = Point::new(450.0, 346.0);
    /// Empty space in item 3.
    const EMPTY: Point = Point::new(300.0, 200.0);
    /// A red government, 129.
    const RED: u32 = 0x00CF_0C0C;

    fn source(system: i16, pos: (i16, i16), govt: i16) -> BorderSource {
        BorderSource {
            system: SystemId(system),
            pos,
            govt: GovtId(govt),
            small: false,
        }
    }

    /// `coarse`, each on its own place.
    fn sources(coarse: &[BorderSource]) -> BorderSources {
        BorderSources {
            coarse: coarse.to_vec(),
            walk: coarse
                .iter()
                .map(|source| (source.system, *source))
                .collect(),
            govts: Governments::default(),
        }
    }

    /// Alpha (128, the Federation's) at (0, 0).
    fn alpha() -> BorderSources {
        sources(&[source(128, (0, 0), 128)])
    }

    /// The testkit galaxy, government 129 red.
    fn coloured() -> Galaxy {
        let mut galaxy = galaxy();
        galaxy.govt_colors = BTreeMap::from([(GovtId(128), BLUE), (GovtId(129), RED)]);
        galaxy
    }

    /// The course map in Alpha at (0, 0), with metrics, shown `shown` and
    /// opened.
    fn opened_with(shown: BorderSources) -> GalaxyMap {
        let mut map =
            GalaxyMap::course(&FakeCatalog::new(coloured())).with_metrics(Rc::new(MonoMetrics));
        map.show_course(SystemId(128), &[]);
        map.recentre();
        map.show_border_sources(shown);
        map.opened();
        map
    }

    fn flown() -> GalaxyMap {
        opened_with(alpha())
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn repeat(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn left(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(map: &mut GalaxyMap, at: Point) {
        map.input(&left(true, at));
        map.input(&left(false, at));
    }

    fn on(button: MapButton) -> Point {
        MapButtons::new(DIALOG_ORIGIN).rect(button).center()
    }

    fn drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// The frame `map` draws under item 3's contents.
    fn frame_len(map: &GalaxyMap) -> usize {
        let place = map.place().expect("in flight");
        let state = ButtonState {
            can_zoom_out: place.can_zoom_out(),
            can_zoom_in: place.can_zoom_in(),
            clear_route: map.clear_route_enabled(),
            borders_shown: map.borders_shown(),
        };
        let mut frame = DrawList::new();
        draw_frame(
            &mut frame,
            &MapButtons::new(DIALOG_ORIGIN),
            &state,
            &ButtonStyle::STOCK,
            Some(&MonoMetrics),
        );
        frame.len()
    }

    /// What the cells `map` shows draw as in item 3.
    fn cells_drawn(map: &GalaxyMap) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_cells(
            &mut list,
            map.border_cells(),
            (221, 136),
            IRect::from_bounds(map_area()),
            &coloured().govt_colors,
        );
        list.iter().cloned().collect()
    }

    fn is_nebula(command: &DrawCommand) -> bool {
        matches!(command, DrawCommand::Clipped { .. })
    }

    /// The steps of the cells `map` shows.
    fn steps(map: &GalaxyMap) -> Vec<i32> {
        let mut steps: Vec<i32> = map.border_cells().iter().map(|cell| cell.step).collect();
        steps.dedup();
        steps
    }

    /// Whether `map`'s last pass was a coarse one not yet followed by a
    /// null event.
    fn coarse_just_ran(map: &GalaxyMap) -> bool {
        let passes = map.border_passes();
        passes.step() == 8 && passes.next() == Next::Unstarted && passes.cached()
    }

    /// `map` ticked until its fine pass is done.
    fn settled(map: &mut GalaxyMap) {
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Idle);
        assert_eq!(map.border_passes().step(), 1);
    }

    fn label_shown(map: &GalaxyMap, label: &str) -> bool {
        drawn(map)
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == label))
    }

    // The open and the tick.

    #[test]
    fn the_open_draws_coarse_blocks_and_the_next_tick_the_fine_cells() {
        let mut map = flown();
        assert!(coarse_just_ran(&map));
        assert!(!map.border_cells().is_empty());
        assert_eq!(steps(&map), [8], "16-pixel blocks");
        map.tick(TICK);
        assert_eq!(steps(&map), [1], "2-pixel cells");
        assert_eq!(map.update(), Some(MapUpdate::Full));
        assert!(
            map.border_cells()
                .iter()
                .all(|cell| cell.govt == GovtId(128))
        );
        let centre = map
            .border_cells()
            .iter()
            .find(|cell| (cell.x, cell.y) == (114, 105))
            .map(|cell| cell.value);
        assert_eq!(
            centre,
            Some(255),
            "Alpha at the scroll: 458 / 4, 420 / 4 in"
        );
        // One step out, 0.5625 × 1.333333, unclamped: r 40, so 1600 ×
        // 0.2 × 0.7499998 = 239.99 at Alpha's own cell, and less, 237 or
        // 236, at the cells a wrong offset would put it in.
        map.input(&press(Key::Char('-')));
        map.tick(TICK);
        let centre = map
            .border_cells()
            .iter()
            .find(|cell| (cell.x, cell.y) == (114, 105))
            .map(|cell| cell.value);
        assert_eq!(centre, Some(239));
    }

    #[test]
    fn a_new_course_map_has_the_borders_on() {
        let map = GalaxyMap::course(&FakeCatalog::new(galaxy()));
        assert!(map.borders_shown());
        assert!(!map.borders_changed_since_open());
        let mut map = flown();
        map.tick(TICK);
        assert!(!map.border_cells().is_empty());
    }

    #[test]
    fn the_cells_come_right_after_item_3s_black_before_the_nebula_and_draw_map() {
        let mut map = flown();
        map.tick(TICK);
        let list = drawn(&map);
        let frame = frame_len(&map);
        let cells = cells_drawn(&map);
        assert!(!cells.is_empty());
        assert_eq!(list[frame..frame + cells.len()], cells[..]);
        assert!(
            is_nebula(&list[frame + cells.len()]),
            "the nebula right after the cells: {:?}",
            list[frame + cells.len()]
        );
    }

    // The partial update.

    #[test]
    fn a_drag_draws_no_cells_and_its_release_runs_the_coarse_pass() {
        let mut map = flown();
        map.tick(TICK);
        map.input(&left(true, EMPTY));
        map.input(&Input::PointerMoved(Point::new(EMPTY.x - 40.0, EMPTY.y)));
        assert_eq!(map.update(), Some(MapUpdate::Partial));
        let list = drawn(&map);
        assert!(
            is_nebula(&list[frame_len(&map)]),
            "no cells: {:?}",
            list[frame_len(&map)]
        );
        assert!(!coarse_just_ran(&map), "nothing until the release");
        map.input(&left(false, Point::new(EMPTY.x - 40.0, EMPTY.y)));
        assert!(coarse_just_ran(&map));
        assert_eq!(steps(&map), [8], "the release redraws: moved");
    }

    #[test]
    fn a_jiggle_runs_the_coarse_pass_and_the_ticks_redraw_repairs_its_frame() {
        let mut map = flown();
        settled(&mut map);
        map.input(&left(true, EMPTY));
        map.input(&Input::PointerMoved(Point::new(EMPTY.x - 1.0, EMPTY.y)));
        map.input(&left(false, Point::new(EMPTY.x - 1.0, EMPTY.y)));
        assert_eq!(map.update(), Some(MapUpdate::Partial), "kept");
        assert!(
            coarse_just_ran(&map),
            "a release that never moved the scroll"
        );
        map.tick(TICK);
        assert_eq!(
            map.update(),
            Some(MapUpdate::Full),
            "the fine pass's redraw"
        );
        assert_eq!(steps(&map), [1]);
        let list = drawn(&map);
        let frame = frame_len(&map);
        let cells = cells_drawn(&map);
        assert_eq!(list[frame..frame + cells.len()], cells[..]);
    }

    // The coarse pass's triggers.

    /// Something done to a map.
    type Done = fn(&mut GalaxyMap);

    #[test]
    fn each_arrow_zoom_space_and_release_runs_the_coarse_pass() {
        // Each, and whether it redraws item 3 (so the blocks show).
        let triggers: [(&str, Done, bool); 10] = [
            (
                "left",
                |map| {
                    map.input(&press(Key::Left));
                },
                true,
            ),
            (
                "up, repeated",
                |map| {
                    map.input(&repeat(Key::Up));
                },
                true,
            ),
            (
                "- key",
                |map| {
                    map.input(&press(Key::Char('-')));
                },
                true,
            ),
            (
                "= key",
                |map| {
                    map.input(&press(Key::Char('=')));
                },
                true,
            ),
            ("- button", |map| click(map, on(MapButton::ZoomOut)), true),
            ("+ button", |map| click(map, on(MapButton::ZoomIn)), true),
            (
                "Space",
                |map| {
                    map.input(&press(Key::Space));
                },
                true,
            ),
            (
                "a drag",
                |map| {
                    map.input(&left(true, EMPTY));
                    map.input(&Input::PointerMoved(Point::new(EMPTY.x - 40.0, EMPTY.y)));
                    map.input(&left(false, Point::new(EMPTY.x - 40.0, EMPTY.y)));
                },
                true,
            ),
            (
                "a press released where it was",
                |map| click(map, EMPTY),
                false,
            ),
            (
                "a drag cancelled where it began",
                |map| {
                    map.input(&left(true, EMPTY));
                    map.cancel_pointer();
                },
                false,
            ),
        ];
        for (name, trigger, redraws) in triggers {
            let mut map = flown();
            settled(&mut map);
            trigger(&mut map);
            assert!(coarse_just_ran(&map), "{name}");
            let shown = if redraws { [8] } else { [1] };
            assert_eq!(steps(&map), shown, "{name}: the picture shown");
        }
    }

    #[test]
    fn a_refused_zoom_or_a_hit_press_runs_no_pass() {
        let refused: [(&str, Done); 3] = [
            ("= at the limit", |map| {
                map.input(&press(Key::Char('=')));
            }),
            ("+ at the limit", |map| click(map, on(MapButton::ZoomIn))),
            ("a press the owner accepts", |map| {
                map.input(&left(true, ALPHA));
                map.take_press().expect("a hit");
                map.answer_press(Some(PressAnswer {
                    selected: Some(SystemId(128)),
                    gate_pick: None,
                    clear_route: None,
                }));
                map.input(&left(false, ALPHA));
            }),
        ];
        for (name, done) in refused {
            let mut map = flown();
            map.input(&press(Key::Char('=')));
            settled(&mut map);
            done(&mut map);
            assert_eq!(map.border_passes().next(), Next::Idle, "{name}");
        }
    }

    // A redraw mid-pass, and the pace.

    /// Alpha's (128, at (0, 0)) and government 129's 131 at (0, 100).
    fn two() -> BorderSources {
        sources(&[source(128, (0, 0), 128), source(131, (0, 100), 129)])
    }

    fn one_a_tick() -> GalaxyMap {
        let mut map =
            GalaxyMap::course(&FakeCatalog::new(coloured())).with_metrics(Rc::new(MonoMetrics));
        map.set_border_rules(RuleSource::Bible, RuleSource::Engine);
        map.show_course(SystemId(128), &[]);
        map.recentre();
        map.show_border_sources(two());
        map.opened();
        map
    }

    fn govts_shown(map: &GalaxyMap) -> Vec<i16> {
        let mut govts: Vec<i16> = map.border_cells().iter().map(|cell| cell.govt.0).collect();
        govts.sort_unstable();
        govts.dedup();
        govts
    }

    fn accepted_press(map: &mut GalaxyMap) {
        map.input(&left(true, ALPHA));
        map.take_press().expect("a hit");
        map.answer_press(Some(PressAnswer {
            selected: Some(SystemId(128)),
            gate_pick: None,
            clear_route: None,
        }));
        map.input(&left(false, ALPHA));
    }

    #[test]
    fn by_the_engines_pace_one_tick_runs_the_fine_pass_to_its_redraw() {
        let mut map = opened_with(two());
        assert!(coarse_just_ran(&map));
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Idle);
        assert_eq!(steps(&map), [1]);
        assert_eq!(govts_shown(&map), [128, 129]);
    }

    #[test]
    fn one_a_tick_the_first_tick_clears_and_each_later_one_adds_a_system() {
        let mut map = one_a_tick();
        let coarse = map.border_cells().to_vec();
        assert_eq!(govts_shown(&map), [128, 129]);
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Walking { after: None });
        assert_eq!(map.border_cells(), coarse, "the coarse picture kept");
        map.tick(TICK);
        assert_eq!(
            map.border_passes().next(),
            Next::Walking {
                after: Some(SystemId(128))
            }
        );
        map.tick(TICK);
        assert_eq!(
            map.border_passes().next(),
            Next::Walking {
                after: Some(SystemId(131))
            }
        );
        assert_eq!(map.border_cells(), coarse, "no redraw yet");
        // 131 isn't index 0x7ff, so the redraw comes on a later null event
        // that adds nothing.
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Idle);
        assert_eq!(steps(&map), [1]);
        assert_eq!(govts_shown(&map), [128, 129]);
    }

    #[test]
    fn a_redraw_mid_pass_shows_the_cleared_grid_and_then_the_partial_one() {
        let mut map = one_a_tick();
        map.tick(TICK);
        accepted_press(&mut map);
        assert_eq!(map.border_cells(), [], "the cleared grid");
        map.tick(TICK);
        assert_eq!(map.border_cells(), [], "no redraw");
        accepted_press(&mut map);
        assert_eq!(govts_shown(&map), [128], "only Alpha added");
        assert_eq!(steps(&map), [1]);
    }

    #[test]
    fn the_fine_pass_waits_while_find_is_open() {
        let mut map = flown();
        map.input(&press(Key::Char('f')));
        assert!(map.finding());
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Unstarted);
        map.close_find();
        map.tick(TICK);
        assert_eq!(map.border_passes().next(), Next::Idle);
    }

    // Find.

    /// Alpha and Gamma (130, at (0, 300)), both the Federation's.
    fn with_gamma(find: RuleSource) -> GalaxyMap {
        let mut map =
            GalaxyMap::course(&FakeCatalog::new(coloured())).with_metrics(Rc::new(MonoMetrics));
        map.set_border_rules(RuleSource::Engine, find);
        map.show_course(SystemId(128), &[]);
        map.recentre();
        map.show_border_sources(sources(&[
            source(128, (0, 0), 128),
            source(130, (0, 300), 128),
        ]));
        map.opened();
        settled(&mut map);
        map
    }

    fn find_gamma(map: &mut GalaxyMap) {
        map.input(&press(Key::Char('f')));
        map.input(&Input::Text('f'));
        map.input(&Input::Text('G'));
        map.input(&press(Key::Enter));
        assert_eq!(map.take_find().as_deref(), Some("G"));
        map.show_search(SearchOutcome::Found(SystemId(130)));
        assert_eq!(map.place().expect("in flight").scroll(), (0, 300));
    }

    #[test]
    fn finds_centring_leaves_the_cells_worked_out_for_the_old_scroll() {
        let mut map = with_gamma(RuleSource::Engine);
        let before = map.border_cells().to_vec();
        let drawn_before = cells_drawn(&map);
        find_gamma(&mut map);
        assert_eq!(map.border_cells(), before, "Alpha's, at the same places");
        assert_eq!(cells_drawn(&map), drawn_before);
        assert_eq!(map.border_passes().next(), Next::Idle, "no pass");
    }

    #[test]
    fn by_the_other_reading_finds_centring_runs_the_coarse_pass() {
        let mut map = with_gamma(RuleSource::Bible);
        let before = map.border_cells().to_vec();
        find_gamma(&mut map);
        assert!(coarse_just_ran(&map));
        assert_ne!(map.border_cells(), before);
        assert_eq!(steps(&map), [8], "Gamma's blocks at the new scroll");
        let centre = map
            .border_cells()
            .iter()
            .find(|cell| (cell.x, cell.y) == (112, 104))
            .copied();
        assert_eq!(
            centre,
            Some(PaintedCell {
                x: 112,
                y: 104,
                step: 8,
                govt: GovtId(128),
                value: 255
            })
        );
    }

    // The toggles.

    #[test]
    fn b_and_its_repeats_flip_the_borders_and_the_label() {
        for key in [Key::Char('b'), Key::Char('B')] {
            let mut map = flown();
            settled(&mut map);
            assert!(label_shown(&map, HIDE_BORDERS_LABEL));
            map.input(&press(key));
            assert!(!map.borders_shown());
            assert!(map.borders_changed_since_open());
            assert!(label_shown(&map, SHOW_BORDERS_LABEL));
            assert_eq!(map.border_cells(), [], "{key:?}: off, none drawn");
            assert!(is_nebula(&drawn(&map)[frame_len(&map)]));
            map.tick(TICK);
            assert_eq!(map.border_cells(), [], "no pass while off");
            map.input(&repeat(key));
            assert!(map.borders_shown(), "{key:?}: a repeat flips too");
            assert!(!map.borders_changed_since_open());
            assert!(coarse_just_ran(&map), "on again: the coarse pass");
            assert_eq!(steps(&map), [8]);
        }
    }

    #[test]
    fn the_borders_button_flips_them_as_b_does() {
        let mut map = flown();
        settled(&mut map);
        click(&mut map, on(MapButton::Borders));
        assert!(!map.borders_shown());
        assert!(label_shown(&map, SHOW_BORDERS_LABEL));
        assert_eq!(map.border_cells(), []);
        assert!(!map.take_close() && !map.take_clear_route() && !map.finding());
        click(&mut map, on(MapButton::Borders));
        assert!(map.borders_shown());
        assert!(coarse_just_ran(&map));
        assert!(label_shown(&map, HIDE_BORDERS_LABEL));
    }

    #[test]
    fn b_is_ignored_while_find_is_open() {
        let mut map = flown();
        map.input(&press(Key::Char('f')));
        map.input(&press(Key::Char('b')));
        assert!(map.borders_shown());
    }

    #[test]
    fn the_open_keeps_the_setting_it_opened_with() {
        let mut map = flown();
        map.input(&press(Key::Char('b')));
        assert!(map.borders_changed_since_open());
        map.opened();
        assert!(!map.borders_changed_since_open(), "opened off");
        assert_eq!(map.border_cells(), [], "and with nothing drawn");
        assert!(!map.border_passes().cached(), "no pass");
        let mut off = GalaxyMap::course(&FakeCatalog::new(coloured()));
        off.show_borders_setting(false);
        off.show_border_sources(alpha());
        off.opened();
        assert!(!off.borders_shown());
        assert!(label_shown(&off, SHOW_BORDERS_LABEL));
        off.tick(TICK);
        assert_eq!(off.border_cells(), []);
    }

    #[test]
    fn the_floating_map_draws_no_cells() {
        let mut map = flown();
        settled(&mut map);
        let bare = GalaxyMap::course(&FakeCatalog::new(coloured()));
        let picture = crate::galaxy::map::MapPicture::viewer(map.model());
        let marks = MapMarks {
            picture: &picture,
            selected: None,
        };
        let transform = map.transform();
        let (mut with, mut without) = (DrawList::new(), DrawList::new());
        map.draw_marks(&mut with, &transform, &marks);
        bare.draw_marks(&mut without, &transform, &marks);
        assert_eq!(with, without);
    }

    // The viewer.

    /// A galaxy of the Federation's Alpha (128), uninhabited, a government
    /// of no influence by `Flags2`, and government 129's Beta, the smaller
    /// influence.
    fn unfiltered() -> Galaxy {
        let mut galaxy = coloured();
        galaxy.systems[0].stellars.clear();
        galaxy.systems[1].govt = Some(GovtId(129));
        galaxy.systems.truncate(2);
        galaxy.govts = Governments::new([
            flagged(128, nova_sim::govt::NO_INFLUENCE),
            flagged(129, nova_sim::govt::SMALL_INFLUENCE),
        ]);
        galaxy
    }

    /// Government `id` of `Flags2` `flags2`, with no classes.
    fn flagged(id: i16, flags2: u16) -> nova_sim::GovtRecord {
        nova_sim::GovtRecord {
            id: GovtId(id),
            name: String::new(),
            flags: 0,
            flags2,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties::default(),
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            medium_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
            news_pic: -1,
            require: 0,
            inh_jam: [0; 4],
            skill_mult: 0,
        }
    }

    fn viewer() -> GalaxyMap {
        GalaxyMap::new(&FakeCatalog::new(unfiltered()))
    }

    fn of_govt(map: &GalaxyMap, govt: i16) -> usize {
        map.border_cells()
            .iter()
            .filter(|cell| cell.govt == GovtId(govt))
            .count()
    }

    #[test]
    fn the_viewer_counts_every_system_with_a_government() {
        let mut map = viewer();
        assert!(map.borders_shown(), "on by default");
        assert_eq!(
            map.border_passes().grid().size(),
            (512, 304),
            "2 pixels a cell"
        );
        assert_eq!(steps(&map), [8], "the coarse pass as it is made");
        map.tick(TICK);
        assert_eq!(steps(&map), [1]);
        assert!(
            of_govt(&map, 128) > 0,
            "no influence and uninhabited: drawn"
        );
        assert!(of_govt(&map, 129) > 0);
        assert!(
            of_govt(&map, 129) < of_govt(&map, 128),
            "0x0002's smaller spread"
        );
        let list = drawn(&map);
        let mut cells = DrawList::new();
        draw_cells(
            &mut cells,
            map.border_cells(),
            (0, 0),
            IRect::new(0, 0, 1024, 608),
            &coloured().govt_colors,
        );
        assert_eq!(
            list[..cells.len()],
            cells.iter().cloned().collect::<Vec<_>>()[..]
        );
        assert!(is_nebula(&list[cells.len()]), "the nebula after them");
    }

    #[test]
    fn a_pan_of_the_viewer_reruns_the_coarse_pass() {
        let mut map = viewer();
        map.tick(TICK);
        map.input(&press(Key::Right));
        assert_eq!(steps(&map), [8]);
        map.tick(TICK);
        assert_eq!(steps(&map), [1]);
        map.input(&press(Key::Char('x')));
        assert_eq!(steps(&map), [1], "the view unchanged: no pass");
    }

    #[test]
    fn b_toggles_the_viewers_borders_and_saves_nothing() {
        let mut map = viewer();
        map.input(&press(Key::Char('b')));
        assert!(!map.borders_shown());
        assert_eq!(map.border_cells(), []);
        map.tick(TICK);
        assert_eq!(map.border_cells(), []);
        assert!(!map.borders_changed_since_open(), "nothing to save");
        map.input(&press(Key::Char('B')));
        assert!(map.borders_shown());
        assert_eq!(steps(&map), [8]);
        assert!(!map.borders_changed_since_open());
    }
}
