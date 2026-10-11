//! The flight HUD: the status bar down the right edge of the screen, laid
//! out by an `ïntf`, with the radar and the shield, armour and fuel bars.
//! The `ïntf`'s nav area is the navigation display (Nova Bible, "The ïntf
//! resource": `NavArea`; STR# 2002 342-349), never the system's name: a
//! [`NavDisplay`], drawn as a dim label ("Stellar Navigation" or
//! "Hyperspace") over the bright name of the selected stellar or of the
//! next system on the course ("Unexplored System" for one not explored),
//! or the dim "No Destination" alone, in the bar's `StatusFont` at its
//! `StatFontSize`. As in the original's `_DrawStatusNav`, the next
//! system's name is dim until the ship is clear to jump, and "Hyperspace"
//! turns bright too once a jump is under way.
//!
//! The date is shown on the last line of the `ïntf`'s `CargoArea`, bright,
//! in the same font. The original shows it only on the player info screen
//! (`STR#` 2002 #252 "Current Date:"), and no `ïntf` area is for it, so
//! this is our choice of where it goes in flight: the cargo area is the
//! stock bar's largest empty panel ((8, 458)-(184, 552), room for about
//! six lines), the nav area is full with its two lines, and the weapon
//! and target areas have their own texts. A cargo display would fill the
//! area from the top and leave its last line to the date.
//!
//! Which `ïntf`: as `_LoadIntfResource` (@0x735f2) picks it, the player's
//! ship type does, not the pilot. Its `InherentGovt` names a government
//! ([`Session::interface_government`](nova_sim::Session::interface_government)),
//! whose `gövt` `Interface` is the bar. A ship type with no government, a
//! government with no `gövt` record, or an `Interface` below 128 means
//! `ïntf` 128, the "Default status bar" ([`interface_id`]), and the same
//! rule picks the background `PICT` from the bar's `StatusBkgnd`
//! (@0x738ed). The bar is chosen again whenever the ship type changes
//! ([`rechoose_status_bar`]).
//!
//! The gauges are `_DrawStatusShield` (@0x4a452) and `_DrawStatusFuel`
//! (@0x4a8ff): each fills from the left when its area is wider than tall,
//! otherwise from the bottom ([`gauge_rect`]). Shield and armour past
//! their capacity fill the whole area ([`shield_share`]); the fuel gauge is
//! not drawn while the player is disabled.
//!
//! The radar draws the [`RadarMark`]s [`radar::marks`] works out for the
//! [`RadarScene`] the view last took, as `_DrawStatusRadar` (@0x496e0)
//! draws them without IFF: a pixel for each ship and the player, a filled
//! 2×2 square for a station or gate (framing a rect that small fills it),
//! and an oval framed as [`oval_frame_pixels`] rasterizes it.
//!
//! Every `ïntf` area is relative to the bar's top-left corner. The bar is
//! as wide as its background picture (194 pixels in every stock bar) and
//! sits against the right edge of the screen, at the top.

use nova_sim::hyperspace::max_jumps;
use nova_sim::{JumpReadiness, Reserves};

use nova_sim::Gauge;

use super::catalog::{GovtId, StatusBarLayout, StatusBars};
use super::radar::{self, IRect, RadarColor, RadarMark, RadarScene};
use crate::draw::fill_rect;
use crate::geometry::{Bounds, Point};
use crate::system::camera::VIEW_SIZE;
use crate::text::LINE_HEIGHT;
use crate::{Color, DrawList, Font, ImageKey};

/// The `ïntf` shown by a pilot with no government, and the ID any ID
/// below it means.
pub const DEFAULT_INTERFACE: i16 = 128;
/// The bar's width when its background cannot be read: the stock bars'.
pub const STATUS_BAR_WIDTH: f32 = 194.0;
/// Fuel for one jump (the Bible: "100 is one jump"): the simulation's
/// [`JUMP_FUEL`](nova_sim::hyperspace::JUMP_FUEL).
pub const FUEL_PER_JUMP: f32 = nova_sim::hyperspace::JUMP_FUEL;
/// The size of the message shown when no status bar can be read.
pub const MESSAGE_SIZE: f32 = 12.0;

/// The `ïntf` ID `raw` means: IDs below 128 mean 128.
#[must_use]
pub fn interface_id(raw: i16) -> i16 {
    raw.max(DEFAULT_INTERFACE)
}

/// A status bar's background picture and its size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Background {
    /// The `PICT`'s ID.
    pub id: i16,
    /// Its width in pixels.
    pub width: f32,
    /// Its height in pixels.
    pub height: f32,
}

/// The status bar the HUD draws: its layout, and its background when the
/// picture can be read.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusBar {
    /// The `ïntf`'s layout.
    pub layout: StatusBarLayout,
    /// The background picture, or `None` when it is missing or does not
    /// decode.
    pub background: Option<Background>,
}

/// The status bar for a ship type of `government` (see the module docs),
/// read from `catalog`, or why it cannot be read: a `gövt` or `ïntf` that
/// does not decode, or a missing `ïntf`.
pub fn choose_status_bar(
    catalog: &impl StatusBars,
    government: Option<GovtId>,
) -> Result<StatusBar, String> {
    let layout = catalog.status_bar(chosen_interface(catalog, government)?)?;
    let background = background(catalog, interface_id(layout.status_bkgnd));
    Ok(StatusBar { layout, background })
}

/// The status bar once the player's ship type changes to one of
/// `government`, where `previous` is the bar shown, if one is.
///
/// With a bar shown, a new `ïntf` that is missing or does not decode
/// keeps the bar's layout and colours under `PICT` 128, as
/// `_LoadIntfResource` does when `GetResource` fails (@0x7369a ->
/// 0x7390d). We treat an `ïntf` that does not decode as missing, since
/// that is the original's one path for an `ïntf` it cannot use. A `gövt`
/// that does not decode is still an error, as on the first choice: the
/// original's `gövt` table always has the record. With no bar shown (the
/// first choice failed) it is a first choice ([`choose_status_bar`]), so
/// a missing `ïntf` is an error again.
pub fn rechoose_status_bar(
    catalog: &impl StatusBars,
    government: Option<GovtId>,
    previous: Option<&StatusBar>,
) -> Result<StatusBar, String> {
    let Some(previous) = previous else {
        return choose_status_bar(catalog, government);
    };
    let id = chosen_interface(catalog, government)?;
    Ok(match catalog.status_bar(id) {
        Ok(layout) => StatusBar {
            background: background(catalog, interface_id(layout.status_bkgnd)),
            layout,
        },
        Err(_) => StatusBar {
            layout: previous.layout.clone(),
            background: background(catalog, DEFAULT_INTERFACE),
        },
    })
}

/// The `ïntf` ID a ship type of `government` shows: its `gövt`'s
/// `Interface` (@0x73648), 128 for none or one with no record, and any
/// below 128 meaning 128 (@0x7364d-0x7365a).
fn chosen_interface(catalog: &impl StatusBars, government: Option<GovtId>) -> Result<i16, String> {
    Ok(match government {
        None => DEFAULT_INTERFACE,
        Some(govt) => catalog
            .government_interface(govt)?
            .map_or(DEFAULT_INTERFACE, interface_id),
    })
}

/// `PICT` `id` as a background, if it reads.
fn background(catalog: &impl StatusBars, id: i16) -> Option<Background> {
    catalog.picture_size(id).map(|(width, height)| Background {
        id,
        width: width as f32,
        height: height as f32,
    })
}

/// Where the bar's top-left corner goes: against the right edge of the
/// screen, as wide as its background, or [`STATUS_BAR_WIDTH`] without one.
#[must_use]
pub fn bar_origin(bar: &StatusBar) -> Point {
    let width = bar.background.map_or(STATUS_BAR_WIDTH, |b| b.width);
    Point::new(VIEW_SIZE.0 - width, 0.0)
}

/// `STR#` 2002 #343: the nav area's label over a selected stellar.
pub const NAV_STELLAR: &str = "Stellar Navigation";
/// `STR#` 2002 #344: the nav area with nothing to show.
pub const NAV_NO_DESTINATION: &str = "No Destination";
/// `STR#` 2002 #345: the nav area's label over a plotted jump.
pub const NAV_HYPERSPACE: &str = "Hyperspace";
/// `STR#` 2002 #346: the nav area's name for a system not yet explored.
pub const NAV_UNEXPLORED: &str = "Unexplored System";

/// What the nav area shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NavDisplay {
    /// Nothing selected and no course: "No Destination".
    #[default]
    None,
    /// The stellar selected as the navigation target, by name.
    Stellar(String),
    /// The next system on the course, and whether the ship can jump to
    /// it, which picks the colours (`_DrawStatusNav`).
    Hyperspace {
        /// Its name, or `None` when the pilot has not explored it.
        name: Option<String>,
        /// Whether the ship can jump there.
        readiness: JumpReadiness,
    },
}

/// What the HUD shows.
#[derive(Clone, Debug, PartialEq)]
pub struct HudState<'a> {
    /// What the radar shows: the snapshot taken at its last refresh, or
    /// this frame's.
    pub radar: &'a RadarScene,
    /// Whether the radar's target shows bright.
    pub blink_on: bool,
    /// The player's shield, armour and fuel.
    pub reserves: Reserves,
    /// What the nav area shows.
    pub nav: NavDisplay,
    /// Today's date, as it is displayed.
    pub date: &'a str,
    /// Whether the player's ship is disabled (`_IsDisabled`), which hides
    /// the fuel gauge.
    pub disabled: bool,
}

/// Draws `bar` showing `state`: its background, the radar (see the
/// module docs), then the shield, armour and fuel bars.
pub fn draw(list: &mut DrawList, bar: &StatusBar, state: &HudState) {
    let origin = bar_origin(bar);
    let layout = &bar.layout;
    if let Some(background) = bar.background {
        list.picture(ImageKey::picture(background.id), origin);
    }
    let radar = IRect::from_bounds(layout.radar.offset(origin));
    for mark in radar::marks(radar, state.radar, state.blink_on) {
        draw_mark(list, layout, mark);
    }
    let reserves = &state.reserves;
    for (area, gauge, color) in [
        (layout.shield, reserves.shield, layout.shield_color),
        (layout.armor, reserves.armor, layout.armor_color),
    ] {
        if let Some(share) = shield_share(gauge) {
            paint(list, gauge_rect(area.offset(origin), 0.0, share), color);
        }
    }
    if !state.disabled {
        draw_fuel(list, layout, layout.fuel.offset(origin), reserves.fuel);
    }
    draw_nav(list, layout, origin, &state.nav);
    draw_date(list, layout, origin, state.date);
}

/// Draws one radar mark in the layout's radar colours: a pixel as a
/// 1-pixel dot over it (a QuickDraw pixel at (x, y) covers [x, x + 1)),
/// a square filled, and an oval as [`oval_frame_pixels`].
fn draw_mark(list: &mut DrawList, layout: &StatusBarLayout, mark: RadarMark) {
    let color = |color| match color {
        RadarColor::Dim => layout.dim_radar,
        RadarColor::Bright => layout.bright_radar,
    };
    match mark {
        RadarMark::Pixel { x, y, color: c } => pixel(list, x, y, color(c)),
        RadarMark::Square { rect, color: c } => fill_rect(list, rect.to_bounds(), color(c)),
        RadarMark::Oval { rect, color: c } => {
            for (x, y) in oval_frame_pixels(rect) {
                pixel(list, x, y, color(c));
            }
        }
    }
}

/// Draws the pixel at (`x`, `y`) in `color`.
fn pixel(list: &mut DrawList, x: i32, y: i32, color: Color) {
    list.dot(Point::new(x as f32 + 0.5, y as f32 + 0.5), 1.0, color);
}

/// The pixels framing the oval inscribed in `rect`, row by row: those
/// whose centres lie inside the ellipse (on it included) with a pixel
/// above, below, left or right that does not. QuickDraw's own `FrameOval`
/// raster is in the Toolbox, not in the original's binary, so this is our
/// approximation of it: a 2×2 oval is all 4 pixels, and a 4×4 one the 8
/// that are not its corners.
#[must_use]
pub fn oval_frame_pixels(rect: IRect) -> Vec<(i32, i32)> {
    let (width, height) = (
        i64::from(rect.right) - i64::from(rect.left),
        i64::from(rect.bottom) - i64::from(rect.top),
    );
    // Twice each pixel centre's offset from the oval's centre, scaled so
    // the test stays in whole numbers.
    let inside = |x: i32, y: i32| {
        let dx = 2 * (i64::from(x) - i64::from(rect.left)) + 1 - width;
        let dy = 2 * (i64::from(y) - i64::from(rect.top)) + 1 - height;
        dx * dx * height * height + dy * dy * width * width <= width * width * height * height
    };
    let mut pixels = Vec::new();
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let edge = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .any(|(x, y)| !inside(x, y));
            if inside(x, y) && edge {
                pixels.push((x, y));
            }
        }
    }
    pixels
}

/// Draws `date` on the last line of the layout's cargo area, at the bar's
/// `origin`, in the bright text colour and the bar's font.
fn draw_date(list: &mut DrawList, layout: &StatusBarLayout, origin: Point, date: &str) {
    let area = layout.cargo.offset(origin);
    let size = layout.font_size;
    let last_line = Point::new(area.min.x, area.max.y - LINE_HEIGHT * size);
    let width = Some(area.width());
    list.text_in(
        layout.font,
        date,
        last_line,
        size,
        width,
        layout.bright_text,
    );
}

/// Draws `nav` in the layout's nav area, at the bar's `origin`: a label
/// over a value, both in the bar's font, or the dim "No Destination"
/// alone. The label is dim and the value bright, except as the original's
/// `_DrawStatusNav` (@0x49f43-0x4a451) draws a jump: its destination is
/// dim while the ship is blocked, and "Hyperspace" is bright once the jump
/// is under way.
fn draw_nav(list: &mut DrawList, layout: &StatusBarLayout, origin: Point, nav: &NavDisplay) {
    let area = layout.nav.offset(origin);
    let (dim, bright) = (layout.dim_text, layout.bright_text);
    let (label, value) = match nav {
        NavDisplay::None => ((NAV_NO_DESTINATION, dim), None),
        NavDisplay::Stellar(name) => ((NAV_STELLAR, dim), Some((name.as_str(), bright))),
        NavDisplay::Hyperspace { name, readiness } => {
            let (label, value) = match readiness {
                JumpReadiness::Blocked => (dim, dim),
                JumpReadiness::Clear => (dim, bright),
                JumpReadiness::Underway => (bright, bright),
            };
            let name = name.as_deref().unwrap_or(NAV_UNEXPLORED);
            ((NAV_HYPERSPACE, label), Some((name, value)))
        }
    };
    let size = layout.font_size;
    let width = Some(area.width());
    let (label, color) = label;
    list.text_in(layout.font, label, area.min, size, width, color);
    if let Some((value, color)) = value {
        let below = Point::new(area.min.x, LINE_HEIGHT.mul_add(size, area.min.y));
        list.text_in(layout.font, value, below, size, width, color);
    }
}

/// How much of a shield or armour gauge `_DrawStatusShield` fills: none
/// when it holds nothing (@0x4a518-0x4a524), all of it past its capacity,
/// a capacity of 0 or less included (the jb @0x4a550), and otherwise
/// `now / max`.
#[must_use]
pub fn shield_share(gauge: Gauge) -> Option<f32> {
    if gauge.now <= 0.0 {
        None
    } else if gauge.max < gauge.now {
        Some(1.0)
    } else {
        Some(gauge.now / gauge.max)
    }
}

/// The part of `area` a gauge fills from `from` to `to` of its capacity,
/// as `_DrawStatusShield` and `_DrawStatusFuel` work it out: from the left
/// when the area is wider than tall (@0x4a570-0x4a596), otherwise, square
/// included, from the bottom (@0x4a59c-0x4a5bf). Each moved edge is
/// truncated toward zero, and the far one goes no further than the area.
/// `None` when that leaves nothing, which also covers an empty or inverted
/// area (@0x4a46f-0x4a485), since a line of no length still draws a
/// square.
#[must_use]
pub fn gauge_rect(area: Bounds, from: f32, to: f32) -> Option<Bounds> {
    let (min, max) = (area.min, area.max);
    let (width, height) = (max.x - min.x, max.y - min.y);
    let painted = if width > height {
        Bounds {
            min: Point::new(width.mul_add(from, min.x).trunc(), min.y),
            max: Point::new(width.mul_add(to, min.x).trunc().min(max.x), max.y),
        }
    } else {
        Bounds {
            min: Point::new(min.x, (-height).mul_add(to, max.y).trunc().max(min.y)),
            max: Point::new(max.x, (-height).mul_add(from, max.y).trunc()),
        }
    };
    (painted.min.x < painted.max.x && painted.min.y < painted.max.y).then_some(painted)
}

/// Fills `painted`, if there is any, with `color`.
fn paint(list: &mut DrawList, painted: Option<Bounds>, color: Color) {
    if let Some(painted) = painted {
        fill_rect(list, painted, color);
    }
}

/// Draws the fuel gauge in `area` as `_DrawStatusFuel` does: nothing for
/// no fuel (or no capacity, where the original divides by zero), else the
/// full colour over all the fuel (@0x4aa1f-0x4aaf6), then the partial
/// colour over the part past the whole jumps (@0x4ab5d-0x4ac11).
fn draw_fuel(list: &mut DrawList, layout: &StatusBarLayout, area: Bounds, fuel: Gauge) {
    if fuel.now <= 0.0 || fuel.max <= 0.0 {
        return;
    }
    let to = fuel.now / fuel.max;
    paint(list, gauge_rect(area, 0.0, to), layout.fuel_full);
    let whole = max_jumps(fuel.now) as f32 * FUEL_PER_JUMP / fuel.max;
    paint(list, gauge_rect(area, whole, to), layout.fuel_partial);
}

/// Draws why no status bar can be shown, where the bar would go.
pub fn draw_unavailable(list: &mut DrawList, reason: &str) {
    list.text_in(
        Font::Geneva,
        format!("Status bar unavailable: {reason}"),
        Point::new(VIEW_SIZE.0 - STATUS_BAR_WIDTH, 0.0),
        MESSAGE_SIZE,
        Some(STATUS_BAR_WIDTH),
        Color::ERROR,
    );
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim::Gauge;

    use super::*;
    use crate::DrawCommand;
    use crate::flight::radar::{RadarShip, RadarStellar};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    const SHIELD: Color = Color::rgba(0, 0, 255, 255);
    const ARMOR: Color = Color::rgba(255, 0, 0, 255);
    const FUEL_FULL: Color = Color::rgba(255, 255, 0, 255);
    const FUEL_PARTIAL: Color = Color::rgba(128, 128, 0, 255);
    const RADAR: Color = Color::rgba(0, 255, 0, 255);
    const DIM_RADAR: Color = Color::rgba(0, 128, 0, 255);
    const TEXT: Color = Color::rgba(250, 250, 250, 255);

    /// Stock `ïntf` 128's areas, with its background `bkgnd`.
    fn layout(bkgnd: i16) -> StatusBarLayout {
        StatusBarLayout {
            radar: rect(8.0, 8.0, 184.0, 184.0),
            shield: rect(35.0, 199.0, 184.0, 206.0),
            armor: rect(35.0, 216.0, 184.0, 223.0),
            fuel: rect(35.0, 234.0, 184.0, 241.0),
            nav: rect(8.0, 254.0, 184.0, 286.0),
            weap: rect(8.0, 300.0, 184.0, 315.0),
            targ: rect(8.0, 330.0, 184.0, 442.0),
            cargo: rect(8.0, 458.0, 184.0, 552.0),
            bright_text: TEXT,
            dim_text: Color::DIM,
            bright_radar: RADAR,
            dim_radar: DIM_RADAR,
            shield_color: SHIELD,
            armor_color: ARMOR,
            fuel_full: FUEL_FULL,
            fuel_partial: FUEL_PARTIAL,
            font: Font::Charcoal,
            font_size: 11.0,
            subtitle_size: 9.0,
            status_bkgnd: bkgnd,
        }
    }

    /// Canned governments, bars and pictures; records what is asked.
    #[derive(Default)]
    struct FakeBars {
        governments: Vec<(i16, Result<i16, String>)>,
        bars: Vec<(i16, Result<StatusBarLayout, String>)>,
        pictures: Vec<(i16, (u32, u32))>,
        asked: RefCell<Vec<String>>,
    }

    impl StatusBars for FakeBars {
        fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
            self.asked.borrow_mut().push(format!("gövt {}", id.0));
            self.governments
                .iter()
                .find(|(govt, _)| *govt == id.0)
                .map_or(Ok(None), |(_, i)| i.clone().map(Some))
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            self.asked.borrow_mut().push(format!("ïntf {id}"));
            self.bars
                .iter()
                .find(|(bar, _)| *bar == id)
                .map_or_else(|| Err(format!("no ïntf {id}")), |(_, b)| b.clone())
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            self.asked.borrow_mut().push(format!("PICT {id}"));
            self.pictures
                .iter()
                .find(|(pict, _)| *pict == id)
                .map(|(_, size)| *size)
        }
    }

    /// `ïntf` 128 (background 700), 129 (background 701) and 130
    /// (background 702); govt 128 shows 130, govt 129 shows 0, govt 130
    /// shows 129, govt 131 shows 127, govt 132 shows -1, govt 133 shows
    /// 131, which is missing, and govt 134 shows 132, which does not
    /// decode; there is no govt 135. The pictures are 194 x 767, but
    /// `PICT` 128 is 200 x 600.
    fn bars() -> FakeBars {
        FakeBars {
            governments: vec![
                (128, Ok(130)),
                (129, Ok(0)),
                (130, Ok(129)),
                (131, Ok(127)),
                (132, Ok(-1)),
                (133, Ok(131)),
                (134, Ok(132)),
            ],
            bars: vec![
                (128, Ok(layout(700))),
                (129, Ok(layout(701))),
                (130, Ok(layout(702))),
                (132, Err("ïntf 132: too short".to_owned())),
            ],
            pictures: vec![
                (128, (200, 600)),
                (700, (194, 767)),
                (701, (194, 767)),
                (702, (194, 767)),
            ],
            asked: RefCell::default(),
        }
    }

    fn background(id: i16) -> Background {
        Background {
            id,
            width: 194.0,
            height: 767.0,
        }
    }

    // The 128 rule.

    #[test]
    fn an_interface_below_128_means_128() {
        let ids = [-1, 0, 127, 128, 130, 383, i16::MIN, i16::MAX].map(interface_id);
        assert_eq!(ids, [128, 128, 128, 128, 130, 383, 128, i16::MAX]);
    }

    // Choosing the bar.

    #[test]
    fn no_government_shows_the_default_bar_without_asking_for_one() {
        let catalog = bars();
        assert_eq!(
            choose_status_bar(&catalog, None),
            Ok(StatusBar {
                layout: layout(700),
                background: Some(background(700)),
            })
        );
        assert_eq!(*catalog.asked.borrow(), ["ïntf 128", "PICT 700"]);
    }

    #[test]
    fn a_government_shows_its_interface_below_128_meaning_128() {
        let catalog = bars();
        let shown = choose_status_bar(&catalog, Some(GovtId(128))).expect("reads");
        assert_eq!(shown.background, Some(background(702)));
        assert_eq!(
            *catalog.asked.borrow(),
            ["gövt 128", "ïntf 130", "PICT 702"]
        );
        for govt in [129, 131, 132] {
            let catalog = bars();
            let shown = choose_status_bar(&catalog, Some(GovtId(govt))).expect("reads");
            assert_eq!(shown.layout, layout(700));
            assert_eq!(
                *catalog.asked.borrow(),
                [format!("gövt {govt}"), "ïntf 128".into(), "PICT 700".into()]
            );
        }
    }

    #[test]
    fn an_interface_of_129_shows_that_bar_and_its_background() {
        let catalog = bars();
        assert_eq!(
            choose_status_bar(&catalog, Some(GovtId(130))),
            Ok(StatusBar {
                layout: layout(701),
                background: Some(background(701)),
            })
        );
        assert_eq!(
            *catalog.asked.borrow(),
            ["gövt 130", "ïntf 129", "PICT 701"]
        );
    }

    #[test]
    fn a_government_with_no_record_shows_the_default_bar() {
        let catalog = bars();
        let shown = choose_status_bar(&catalog, Some(GovtId(135))).expect("reads");
        assert_eq!(shown.layout, layout(700));
        assert_eq!(
            *catalog.asked.borrow(),
            ["gövt 135", "ïntf 128", "PICT 700"]
        );
    }

    // Choosing again, when the ship type changes.

    fn kept_with_picture_128() -> StatusBar {
        StatusBar {
            layout: layout(702),
            background: Some(Background {
                id: 128,
                width: 200.0,
                height: 600.0,
            }),
        }
    }

    #[test]
    fn choosing_again_reads_the_new_governments_bar() {
        let catalog = bars();
        let previous = choose_status_bar(&catalog, Some(GovtId(128))).expect("reads");
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(130)), Some(&previous)),
            choose_status_bar(&catalog, Some(GovtId(130)))
        );
    }

    #[test]
    fn a_missing_or_undecodable_new_bar_keeps_the_layout_over_picture_128() {
        for govt in [133, 134] {
            let catalog = bars();
            let previous = choose_status_bar(&catalog, Some(GovtId(128))).expect("reads");
            catalog.asked.borrow_mut().clear();
            assert_eq!(
                rechoose_status_bar(&catalog, Some(GovtId(govt)), Some(&previous)),
                Ok(kept_with_picture_128()),
                "gövt {govt}"
            );
            let intf = if govt == 133 {
                "ïntf 131"
            } else {
                "ïntf 132"
            };
            assert_eq!(
                *catalog.asked.borrow(),
                [format!("gövt {govt}"), intf.into(), "PICT 128".into()]
            );
        }
        // With no picture 128 the kept layout has no background.
        let catalog = FakeBars {
            pictures: Vec::new(),
            ..bars()
        };
        let previous = StatusBar {
            layout: layout(702),
            background: Some(background(702)),
        };
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(133)), Some(&previous)),
            Ok(StatusBar {
                layout: layout(702),
                background: None,
            })
        );
    }

    #[test]
    fn with_no_bar_shown_choosing_again_is_a_first_choice() {
        let catalog = bars();
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(133)), None),
            Err("no ïntf 131".to_owned())
        );
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(134)), None),
            Err("ïntf 132: too short".to_owned())
        );
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(130)), None),
            choose_status_bar(&catalog, Some(GovtId(130)))
        );
    }

    #[test]
    fn an_undecodable_government_is_an_error_even_when_choosing_again() {
        let catalog = FakeBars {
            governments: vec![(128, Err("gövt 128: too short".to_owned()))],
            ..bars()
        };
        let previous = kept_with_picture_128();
        assert_eq!(
            rechoose_status_bar(&catalog, Some(GovtId(128)), Some(&previous)),
            Err("gövt 128: too short".to_owned())
        );
    }

    #[test]
    fn a_background_below_128_means_picture_128() {
        let catalog = FakeBars {
            bars: vec![(128, Ok(layout(-1)))],
            pictures: vec![(128, (200, 600))],
            ..FakeBars::default()
        };
        let shown = choose_status_bar(&catalog, None).expect("reads");
        assert_eq!(
            shown.background,
            Some(Background {
                id: 128,
                width: 200.0,
                height: 600.0
            })
        );
        assert_eq!(*catalog.asked.borrow(), ["ïntf 128", "PICT 128"]);
    }

    #[test]
    fn a_missing_background_is_none() {
        let catalog = FakeBars {
            bars: vec![(128, Ok(layout(700)))],
            ..FakeBars::default()
        };
        assert_eq!(
            choose_status_bar(&catalog, None).map(|bar| bar.background),
            Ok(None)
        );
    }

    #[test]
    fn each_read_error_is_the_result() {
        let catalog = FakeBars {
            governments: vec![(128, Err("gövt 128: too short".to_owned()))],
            ..bars()
        };
        assert_eq!(
            choose_status_bar(&catalog, Some(GovtId(128))),
            Err("gövt 128: too short".to_owned())
        );
        assert_eq!(*catalog.asked.borrow(), ["gövt 128"]);
        let catalog = FakeBars::default();
        assert_eq!(
            choose_status_bar(&catalog, None),
            Err("no ïntf 128".to_owned())
        );
        assert_eq!(*catalog.asked.borrow(), ["ïntf 128"]);
    }

    // Placement.

    #[test]
    fn the_bar_sits_against_the_right_edge_as_wide_as_its_background() {
        let mut bar = StatusBar {
            layout: layout(700),
            background: Some(background(700)),
        };
        assert_eq!(bar_origin(&bar), at(830.0, 0.0));
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 10.0,
        });
        assert_eq!(bar_origin(&bar), at(824.0, 0.0));
        bar.background = None;
        assert_eq!(bar_origin(&bar), at(830.0, 0.0));
        assert_eq!(STATUS_BAR_WIDTH, 194.0);
    }

    // The radar.

    #[test]
    fn small_ovals_frame_every_pixel_and_a_four_pixel_one_leaves_its_corners() {
        assert_eq!(
            oval_frame_pixels(IRect::new(10, 20, 12, 22)),
            [(10, 20), (11, 20), (10, 21), (11, 21)]
        );
        assert_eq!(
            oval_frame_pixels(IRect::new(0, 0, 4, 4)),
            [
                (1, 0),
                (2, 0),
                (0, 1),
                (3, 1),
                (0, 2),
                (3, 2),
                (1, 3),
                (2, 3)
            ]
        );
    }

    #[test]
    fn a_six_pixel_oval_is_rounded() {
        // Rows 0 and 5 run from 1 to 4; rows 1 to 4 are their ends alone.
        let mut expected = vec![(1, 0), (2, 0), (3, 0), (4, 0)];
        for y in 1..5 {
            expected.extend([(0, y), (5, y)]);
        }
        expected.extend([(1, 5), (2, 5), (3, 5), (4, 5)]);
        assert_eq!(oval_frame_pixels(IRect::new(0, 0, 6, 6)), expected);
    }

    #[test]
    fn a_clipped_oval_is_squashed_into_its_rect() {
        // 3 across and 6 down: a point at each end, the sides between.
        let mut expected = vec![(101, 0)];
        for y in 1..5 {
            expected.extend([(100, y), (102, y)]);
        }
        expected.push((101, 5));
        assert_eq!(oval_frame_pixels(IRect::new(100, 0, 103, 6)), expected);
        assert_eq!(oval_frame_pixels(IRect::new(0, 0, 0, 6)), []);
    }

    // Drawing.

    fn reserves(shield: f32, armor: f32, fuel: f32) -> Reserves {
        Reserves {
            shield: Gauge {
                now: shield,
                max: 30.0,
            },
            armor: Gauge {
                now: armor,
                max: 60.0,
            },
            fuel: Gauge {
                now: fuel,
                max: 300.0,
            },
        }
    }

    fn drawn(bar: &StatusBar, state: &HudState) -> DrawList {
        let mut list = DrawList::new();
        draw(&mut list, bar, state);
        list
    }

    fn stock() -> StatusBar {
        StatusBar {
            layout: layout(700),
            background: Some(background(700)),
        }
    }

    /// A radar with nothing on it but the player, at (0, 0).
    static NO_RADAR: RadarScene = RadarScene {
        player: Point::new(0.0, 0.0),
        stellars: Vec::new(),
        ships: Vec::new(),
        blank: false,
    };

    fn state(reserves: Reserves) -> HudState<'static> {
        HudState {
            radar: &NO_RADAR,
            blink_on: false,
            reserves,
            nav: NavDisplay::None,
            date: DATE,
            disabled: false,
        }
    }

    fn lines(list: &DrawList) -> Vec<(Point, Point, f32, Color)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } => Some((from, to, width, color)),
                _ => None,
            })
            .collect()
    }

    /// The line filling the stock bar whose top is `top` (relative to the
    /// bar) from `from` to `to` pixels along it, at (830, 0).
    fn bar_line(top: f32, from: f32, to: f32, color: Color) -> (Point, Point, f32, Color) {
        (
            at(865.0 + from, top + 3.5),
            at(865.0 + to, top + 3.5),
            7.0,
            color,
        )
    }

    #[test]
    fn full_reserves_fill_each_bar_in_its_colour() {
        let list = drawn(&stock(), &state(reserves(30.0, 60.0, 300.0)));
        assert_eq!(
            lines(&list),
            [
                bar_line(199.0, 0.0, 149.0, SHIELD),
                bar_line(216.0, 0.0, 149.0, ARMOR),
                bar_line(234.0, 0.0, 149.0, FUEL_FULL),
            ]
        );
    }

    #[test]
    fn each_bar_fills_as_far_as_its_gauge_truncated() {
        // 74.5, 37.25 and 99.33 pixels of 149.
        let list = drawn(&stock(), &state(reserves(15.0, 15.0, 200.0)));
        assert_eq!(
            lines(&list),
            [
                bar_line(199.0, 0.0, 74.0, SHIELD),
                bar_line(216.0, 0.0, 37.0, ARMOR),
                bar_line(234.0, 0.0, 99.0, FUEL_FULL),
            ]
        );
    }

    #[test]
    fn an_empty_bar_draws_nothing() {
        let list = drawn(&stock(), &state(reserves(0.0, 0.0, 0.0)));
        assert_eq!(lines(&list), []);
        let none = HudState {
            reserves: Reserves::default(),
            ..state(Reserves::default())
        };
        assert_eq!(lines(&drawn(&stock(), &none)), []);
        let negative = drawn(&stock(), &state(reserves(-1.0, -1.0, -50.0)));
        assert_eq!(lines(&negative), []);
    }

    #[test]
    fn over_capacity_shield_and_armour_draw_full() {
        let over = Reserves {
            shield: Gauge {
                now: 40.0,
                max: 30.0,
            },
            armor: Gauge { now: 5.0, max: 0.0 },
            fuel: Gauge::default(),
        };
        assert_eq!(
            lines(&drawn(&stock(), &state(over))),
            [
                bar_line(199.0, 0.0, 149.0, SHIELD),
                bar_line(216.0, 0.0, 149.0, ARMOR),
            ]
        );
    }

    #[test]
    fn fuel_beyond_whole_jumps_is_painted_over_in_the_partial_colour() {
        // 250 of 300: all of it (124.17 pixels) in the full colour, then
        // the half jump past the two whole ones (from 99.33) over it.
        let list = drawn(&stock(), &state(reserves(0.0, 0.0, 250.0)));
        assert_eq!(
            lines(&list),
            [
                bar_line(234.0, 0.0, 124.0, FUEL_FULL),
                bar_line(234.0, 99.0, 124.0, FUEL_PARTIAL),
            ]
        );
        // Under one jump: the partial colour over all of it.
        let list = drawn(&stock(), &state(reserves(0.0, 0.0, 50.0)));
        assert_eq!(
            lines(&list),
            [
                bar_line(234.0, 0.0, 24.0, FUEL_FULL),
                bar_line(234.0, 0.0, 24.0, FUEL_PARTIAL),
            ]
        );
    }

    #[test]
    fn over_full_or_odd_fuel_stays_within_the_bar() {
        let over = drawn(&stock(), &state(reserves(0.0, 0.0, 450.0)));
        assert_eq!(lines(&over), [bar_line(234.0, 0.0, 149.0, FUEL_FULL)]);
        // A capacity that is not whole jumps: 250 of 250 is two jumps, then
        // a half (from 119.2).
        let odd = Reserves {
            fuel: Gauge::full(250.0),
            ..Reserves::default()
        };
        assert_eq!(
            lines(&drawn(&stock(), &state(odd))),
            [
                bar_line(234.0, 0.0, 149.0, FUEL_FULL),
                bar_line(234.0, 119.0, 149.0, FUEL_PARTIAL),
            ]
        );
        // No capacity: no gauge, whatever the fuel.
        let none = Reserves {
            fuel: Gauge {
                now: 50.0,
                max: 0.0,
            },
            ..Reserves::default()
        };
        assert_eq!(lines(&drawn(&stock(), &state(none))), []);
    }

    #[test]
    fn a_disabled_player_has_no_fuel_gauge() {
        let hud = HudState {
            disabled: true,
            ..state(reserves(30.0, 60.0, 250.0))
        };
        assert_eq!(
            lines(&drawn(&stock(), &hud)),
            [
                bar_line(199.0, 0.0, 149.0, SHIELD),
                bar_line(216.0, 0.0, 149.0, ARMOR),
            ]
        );
    }

    #[test]
    fn a_tall_fuel_area_fills_from_the_bottom() {
        // (10, 100)-(20, 200) on the bar at (830, 0): 250 of 300 is 83.33
        // pixels up from 200, and the two whole jumps 66.67.
        let mut bar = stock();
        bar.layout.fuel = rect(10.0, 100.0, 20.0, 200.0);
        let fill = |top: f32, bottom: f32, color| {
            (
                at(840.0, f32::midpoint(top, bottom)),
                at(850.0, f32::midpoint(top, bottom)),
                bottom - top,
                color,
            )
        };
        let list = drawn(&bar, &state(reserves(0.0, 0.0, 250.0)));
        assert_eq!(
            lines(&list),
            [
                fill(116.0, 200.0, FUEL_FULL),
                fill(116.0, 133.0, FUEL_PARTIAL)
            ]
        );
        // An exact number of jumps has no partial part.
        let list = drawn(&bar, &state(reserves(0.0, 0.0, 200.0)));
        assert_eq!(lines(&list), [fill(133.0, 200.0, FUEL_FULL)]);
    }

    // Gauge geometry.

    const WIDE: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(110.0, 30.0),
    };

    const TALL: Bounds = Bounds {
        min: Point::new(10.0, 20.0),
        max: Point::new(20.0, 120.0),
    };

    #[test]
    fn a_wide_gauge_fills_from_the_left_truncating_its_edges() {
        assert_eq!(
            gauge_rect(WIDE, 0.0, 0.5),
            Some(rect(10.0, 20.0, 60.0, 30.0))
        );
        assert_eq!(
            gauge_rect(WIDE, 0.0, 0.505),
            Some(rect(10.0, 20.0, 60.0, 30.0))
        );
        assert_eq!(
            gauge_rect(WIDE, 0.255, 0.5),
            Some(rect(35.0, 20.0, 60.0, 30.0))
        );
        // Toward zero, not down.
        let left = rect(-110.0, 20.0, -10.0, 30.0);
        assert_eq!(
            gauge_rect(left, 0.0, 0.505),
            Some(rect(-110.0, 20.0, -59.0, 30.0))
        );
    }

    #[test]
    fn a_tall_or_square_gauge_fills_from_the_bottom() {
        assert_eq!(
            gauge_rect(TALL, 0.0, 0.5),
            Some(rect(10.0, 70.0, 20.0, 120.0))
        );
        assert_eq!(
            gauge_rect(TALL, 0.0, 0.505),
            Some(rect(10.0, 69.0, 20.0, 120.0))
        );
        assert_eq!(
            gauge_rect(TALL, 0.255, 0.5),
            Some(rect(10.0, 70.0, 20.0, 94.0))
        );
        let square = rect(0.0, 0.0, 10.0, 10.0);
        assert_eq!(
            gauge_rect(square, 0.0, 0.5),
            Some(rect(0.0, 5.0, 10.0, 10.0))
        );
    }

    #[test]
    fn a_gauge_past_its_capacity_stops_at_its_area() {
        assert_eq!(gauge_rect(WIDE, 0.0, 1.5), Some(WIDE));
        assert_eq!(gauge_rect(TALL, 0.0, 1.5), Some(TALL));
        assert_eq!(gauge_rect(WIDE, 1.2, 1.5), None);
        assert_eq!(gauge_rect(TALL, 1.2, 1.5), None);
    }

    #[test]
    fn an_empty_span_or_area_paints_nothing() {
        assert_eq!(gauge_rect(WIDE, 0.5, 0.5), None);
        assert_eq!(gauge_rect(WIDE, 0.0, 0.001), None, "under a pixel");
        // Truncating the top edge moves it up: under a pixel is one.
        assert_eq!(
            gauge_rect(TALL, 0.0, 0.001),
            Some(rect(10.0, 119.0, 20.0, 120.0))
        );
        assert_eq!(gauge_rect(TALL, 0.5, 0.5), None);
        for area in [
            rect(10.0, 20.0, 10.0, 30.0),
            rect(110.0, 20.0, 10.0, 30.0),
            rect(10.0, 20.0, 110.0, 20.0),
            rect(10.0, 30.0, 110.0, 20.0),
        ] {
            assert_eq!(gauge_rect(area, 0.0, 1.0), None, "{area:?}");
        }
    }

    #[test]
    fn a_shield_shows_its_share_or_full_past_its_capacity() {
        let share = |now, max| shield_share(Gauge { now, max });
        assert_eq!(share(0.0, 30.0), None);
        assert_eq!(share(-1.0, 30.0), None);
        assert_eq!(share(15.0, 30.0), Some(0.5));
        assert_eq!(share(30.0, 30.0), Some(1.0));
        assert_eq!(share(40.0, 30.0), Some(1.0));
        assert_eq!(share(5.0, 0.0), Some(1.0));
        assert_eq!(share(5.0, -10.0), Some(1.0));
    }

    #[test]
    fn it_draws_the_background_then_the_radar_then_the_bars() {
        // About the stock radar's centre, (926, 96): a station 10 up, a
        // small planet 10 right, a big one 2 left (an oval 6 across), a
        // ship 5 left, and the targeted ship 5 down, bright with the blink.
        let scene = RadarScene {
            player: at(0.0, 0.0),
            stellars: vec![
                RadarStellar {
                    position: at(0.0, -320.0),
                    square: true,
                    sprite_width: Some(300),
                },
                RadarStellar {
                    position: at(320.0, 0.0),
                    square: false,
                    sprite_width: Some(40),
                },
                RadarStellar {
                    position: at(-64.0, 0.0),
                    square: false,
                    sprite_width: Some(200),
                },
            ],
            ships: vec![
                RadarShip {
                    position: at(-160.0, 0.0),
                    targeted: false,
                },
                RadarShip {
                    position: at(0.0, 160.0),
                    targeted: true,
                },
            ],
            blank: false,
        };
        let hud = HudState {
            radar: &scene,
            blink_on: true,
            reserves: reserves(30.0, 60.0, 300.0),
            nav: NavDisplay::Stellar("Earth".to_owned()),
            date: DATE,
            disabled: false,
        };
        let list = drawn(&stock(), &hud);
        let mut expected = DrawList::new();
        expected.picture(ImageKey::picture(700), at(830.0, 0.0));
        fill_rect(&mut expected, rect(925.0, 85.0, 927.0, 87.0), DIM_RADAR);
        for (x, y) in [(935, 95), (936, 95), (935, 96), (936, 96)] {
            expected.dot(pixel_at(x, y), 1.0, DIM_RADAR);
        }
        for (x, y) in oval_frame_pixels(IRect::new(921, 93, 927, 99)) {
            expected.dot(pixel_at(x, y), 1.0, DIM_RADAR);
        }
        expected.dot(pixel_at(921, 96), 1.0, DIM_RADAR);
        expected.dot(pixel_at(926, 101), 1.0, RADAR);
        expected.dot(pixel_at(926, 96), 1.0, RADAR);
        expected.line(at(865.0, 202.5), at(1014.0, 202.5), 7.0, SHIELD);
        expected.line(at(865.0, 219.5), at(1014.0, 219.5), 7.0, ARMOR);
        expected.line(at(865.0, 237.5), at(1014.0, 237.5), 7.0, FUEL_FULL);
        expected.push(nav_text(NAV_STELLAR, 0, Color::DIM));
        expected.push(nav_text("Earth", 1, TEXT));
        expected.push(date_text(DATE));
        assert_eq!(list, expected);
        let blink_off = HudState {
            blink_on: false,
            ..hud
        };
        assert_eq!(
            dots(&drawn(&stock(), &blink_off), RADAR),
            [pixel_at(926, 96)],
            "the target dim, the player bright"
        );
    }

    /// The centre of the pixel at (`x`, `y`).
    fn pixel_at(x: i32, y: i32) -> Point {
        at(x as f32 + 0.5, y as f32 + 0.5)
    }

    #[test]
    fn a_clipped_ovals_pixels_stay_inside_the_radar() {
        // A big planet at the radar's right edge, x = 1013.
        let scene = RadarScene {
            stellars: vec![RadarStellar {
                position: at(2784.0, 0.0),
                square: false,
                sprite_width: Some(200),
            }],
            ..NO_RADAR.clone()
        };
        let hud = HudState {
            radar: &scene,
            ..state(reserves(30.0, 60.0, 300.0))
        };
        let shown = dots(&drawn(&stock(), &hud), DIM_RADAR);
        assert!(!shown.is_empty());
        for dot in shown {
            assert!(
                (838.0..1014.0).contains(&dot.x) && (8.0..184.0).contains(&dot.y),
                "{dot:?}"
            );
        }
    }

    #[test]
    fn the_radar_moves_with_the_bar_and_reads_its_layout() {
        let mut bar = stock();
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 767.0,
        });
        bar.layout.radar = rect(10.0, 20.0, 110.0, 120.0);
        bar.layout.dim_radar = SHIELD;
        bar.layout.bright_radar = ARMOR;
        let scene = RadarScene {
            ships: vec![RadarShip {
                position: at(64.0, 0.0),
                targeted: false,
            }],
            ..NO_RADAR.clone()
        };
        let hud = HudState {
            radar: &scene,
            ..state(reserves(0.0, 0.0, 0.0))
        };
        // The bar at (824, 0): the radar (834, 20)-(934, 120), centred on
        // (884, 70).
        let list = drawn(&bar, &hud);
        assert_eq!(dots(&list, SHIELD), [pixel_at(886, 70)]);
        assert_eq!(dots(&list, ARMOR), [pixel_at(884, 70)]);
        bar.layout.radar = rect(10.0, 20.0, 10.0, 120.0);
        let list = drawn(&bar, &hud);
        assert_eq!(dots(&list, SHIELD), []);
        assert_eq!(dots(&list, ARMOR), [], "an empty radar draws nothing");
    }

    /// The dots drawn in `color`.
    fn dots(list: &DrawList, color: Color) -> Vec<Point> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot {
                    center, color: c, ..
                } if c == color => Some(center),
                _ => None,
            })
            .collect()
    }

    // The nav area.

    fn texts(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .filter(|command| matches!(command, DrawCommand::Text { .. }))
            .cloned()
            .collect()
    }

    /// The texts but the date.
    fn nav_texts(list: &DrawList) -> Vec<DrawCommand> {
        texts(list)
            .into_iter()
            .filter(|command| !matches!(command, DrawCommand::Text { text, .. } if text == DATE))
            .collect()
    }

    /// `text` on line `line` (0 or 1) of the stock bar's nav area, at
    /// (830, 0): its (8, 254)-(184, 286) is (838, 254), 176 wide, in the
    /// layout's Charcoal 11, one line height (1.2 x 11) a line.
    fn nav_text(text: &str, line: u8, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Charcoal,
            origin: at(838.0, 254.0 + f32::from(line) * 13.2),
            size: 11.0,
            wrap_width: Some(176.0),
            color,
        }
    }

    fn nav_drawn(nav: NavDisplay) -> Vec<DrawCommand> {
        let hud = HudState {
            nav,
            ..state(reserves(30.0, 60.0, 300.0))
        };
        nav_texts(&drawn(&stock(), &hud))
    }

    #[test]
    fn the_nav_strings_are_the_originals() {
        assert_eq!(
            [
                NAV_STELLAR,
                NAV_NO_DESTINATION,
                NAV_HYPERSPACE,
                NAV_UNEXPLORED
            ],
            [
                "Stellar Navigation",
                "No Destination",
                "Hyperspace",
                "Unexplored System"
            ]
        );
        assert_eq!(NavDisplay::default(), NavDisplay::None);
    }

    #[test]
    fn with_nothing_to_show_the_nav_area_says_no_destination_dim() {
        assert_eq!(
            nav_drawn(NavDisplay::None),
            [nav_text(NAV_NO_DESTINATION, 0, Color::DIM)]
        );
    }

    #[test]
    fn a_selected_stellar_is_stellar_navigation_over_its_name() {
        assert_eq!(
            nav_drawn(NavDisplay::Stellar("Earth".to_owned())),
            [
                nav_text(NAV_STELLAR, 0, Color::DIM),
                nav_text("Earth", 1, TEXT)
            ]
        );
    }

    fn hyperspace(name: Option<&str>, readiness: JumpReadiness) -> NavDisplay {
        NavDisplay::Hyperspace {
            name: name.map(str::to_owned),
            readiness,
        }
    }

    #[test]
    fn a_blocked_jump_draws_hyperspace_and_the_destination_dim() {
        assert_eq!(
            nav_drawn(hyperspace(Some("Sol"), JumpReadiness::Blocked)),
            [
                nav_text(NAV_HYPERSPACE, 0, Color::DIM),
                nav_text("Sol", 1, Color::DIM)
            ]
        );
        assert_eq!(
            nav_drawn(hyperspace(None, JumpReadiness::Blocked)),
            [
                nav_text(NAV_HYPERSPACE, 0, Color::DIM),
                nav_text(NAV_UNEXPLORED, 1, Color::DIM)
            ]
        );
    }

    #[test]
    fn a_clear_jump_draws_the_destination_bright() {
        assert_eq!(
            nav_drawn(hyperspace(Some("Sol"), JumpReadiness::Clear)),
            [
                nav_text(NAV_HYPERSPACE, 0, Color::DIM),
                nav_text("Sol", 1, TEXT)
            ]
        );
        assert_eq!(
            nav_drawn(hyperspace(None, JumpReadiness::Clear)),
            [
                nav_text(NAV_HYPERSPACE, 0, Color::DIM),
                nav_text(NAV_UNEXPLORED, 1, TEXT)
            ]
        );
    }

    #[test]
    fn an_underway_jump_draws_both_bright() {
        assert_eq!(
            nav_drawn(hyperspace(Some("Sol"), JumpReadiness::Underway)),
            [nav_text(NAV_HYPERSPACE, 0, TEXT), nav_text("Sol", 1, TEXT)]
        );
    }

    #[test]
    fn the_nav_area_moves_with_the_bar_and_reads_its_layout() {
        let mut bar = stock();
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 767.0,
        });
        bar.layout.font = Font::Geneva;
        bar.layout.font_size = 12.0;
        bar.layout.nav = rect(10.0, 300.0, 110.0, 340.0);
        bar.layout.dim_text = FUEL_PARTIAL;
        bar.layout.bright_text = SHIELD;
        let hud = HudState {
            nav: NavDisplay::Stellar("Earth".to_owned()),
            ..state(reserves(30.0, 60.0, 300.0))
        };
        let text = |text: &str, y: f32, color| DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: at(834.0, y),
            size: 12.0,
            wrap_width: Some(100.0),
            color,
        };
        assert_eq!(
            nav_texts(&drawn(&bar, &hud)),
            [
                text(NAV_STELLAR, 300.0, FUEL_PARTIAL),
                text("Earth", 314.4, SHIELD)
            ]
        );
    }

    // The date.

    const DATE: &str = "June 23, 1177 NC";

    /// `text` on the last line of the stock bar's cargo area, at (830, 0):
    /// its (8, 458)-(184, 552) is (838, 458)-(1014, 552), and the last
    /// line of Charcoal 11 starts one line height (13.2) above its bottom.
    fn date_text(text: &str) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Charcoal,
            origin: at(838.0, 552.0 - 13.2),
            size: 11.0,
            wrap_width: Some(176.0),
            color: TEXT,
        }
    }

    fn dates(list: &DrawList) -> Vec<DrawCommand> {
        texts(list)
            .into_iter()
            .filter(|command| matches!(command, DrawCommand::Text { text, .. } if text == DATE))
            .collect()
    }

    #[test]
    fn the_date_is_on_the_cargo_areas_last_line_bright() {
        let list = drawn(&stock(), &state(reserves(30.0, 60.0, 300.0)));
        assert_eq!(dates(&list), [date_text(DATE)]);
        let mut expected = nav_texts(&list);
        expected.push(date_text(DATE));
        assert_eq!(texts(&list), expected, "after the nav area");
    }

    #[test]
    fn the_date_moves_with_the_bar_and_reads_its_layout() {
        let mut bar = stock();
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 767.0,
        });
        bar.layout.font = Font::Geneva;
        bar.layout.font_size = 10.0;
        bar.layout.cargo = rect(10.0, 400.0, 110.0, 500.0);
        bar.layout.bright_text = SHIELD;
        let list = drawn(&bar, &state(reserves(30.0, 60.0, 300.0)));
        assert_eq!(
            dates(&list),
            [DrawCommand::Text {
                text: DATE.to_owned(),
                font: Font::Geneva,
                origin: at(834.0, 488.0),
                size: 10.0,
                wrap_width: Some(100.0),
                color: SHIELD,
            }]
        );
        bar.background = None;
        let list = drawn(&bar, &state(reserves(30.0, 60.0, 300.0)));
        let DrawCommand::Text { origin, .. } = &dates(&list)[0] else {
            unreachable!("a text")
        };
        assert_eq!(*origin, at(VIEW_SIZE.0 - STATUS_BAR_WIDTH + 10.0, 488.0));
    }

    #[test]
    fn without_a_background_nothing_is_drawn_under_the_bar() {
        let bar = StatusBar {
            layout: layout(700),
            background: None,
        };
        let list = drawn(&bar, &state(reserves(30.0, 60.0, 300.0)));
        assert!(
            !list
                .iter()
                .any(|c| matches!(c, DrawCommand::Picture { .. })),
            "{list:?}"
        );
        assert_eq!(lines(&list)[0], bar_line(199.0, 0.0, 149.0, SHIELD));
    }

    #[test]
    fn an_unreadable_bar_says_why_where_the_bar_goes() {
        let mut list = DrawList::new();
        draw_unavailable(&mut list, "no ïntf 128");
        let mut expected = DrawList::new();
        expected.text_in(
            Font::Geneva,
            "Status bar unavailable: no ïntf 128",
            at(830.0, 0.0),
            12.0,
            Some(194.0),
            Color::ERROR,
        );
        assert_eq!(list, expected);
        assert_eq!(MESSAGE_SIZE, 12.0);
    }
}
