//! The flight HUD: the status bar down the right edge of the screen, laid
//! out by an `ïntf`, with the radar and the shield, armour and fuel bars.
//! The screen is the view's, widened or not: the bar and its areas are
//! placed from the view's width ([`bar_origin`]), and the message box from
//! its width and height ([`message_box`]), as the original places them
//! from the window's at any size.
//! The `ïntf`'s nav area is the navigation display (Nova Bible, "The ïntf
//! resource": `NavArea`; STR# 2002 342-349), never the system's name: a
//! [`NavDisplay`], drawn as `_DrawStatusNav` (@0x49f43-0x4a451) draws it,
//! in the bar's `StatusFont` at its `StatFontSize`, each line centred
//! between the area's sides by `_DrawCenteredText` (@0x263d) on a
//! baseline below its top: the dim "Nav System Off" alone on
//! [`NAV_OFF_BASELINE`], or a label ("Stellar Navigation" or
//! "Hyperspace") on [`NAV_LABEL_BASELINE`] over the selected stellar's
//! name, the next system's ("Unexplored System" for one not explored) or
//! the dim "No Destination", on [`NAV_VALUE_BASELINE`]. The next system's
//! name is dim until the ship is clear to jump, and "Hyperspace" turns
//! bright too once a jump is under way. An empty or inverted nav area
//! draws nothing (@0x49f55-0x49f6b). Lines are never wrapped, as
//! `DrawString` never wraps; the original's clip to the area (it copies
//! only the area's rect to the window, @0x4a442) is left out, as a draw
//! list has no clipping and stock names fit the stock area.
//!
//! The `ïntf`'s `CargoArea` lists the hold as `_DrawStatusCargo`
//! (@0x4c313-0x4c7ad) does, from the [`StatusCargo`] the session decides
//! (see `nova_sim::status_cargo`): each held commodity on its own row, "Free:"
//! and the tons free, "Special:" naming the mission cargo or `jünk` when
//! there is any, and the credits, each line from a fixed pen position on
//! a fixed baseline (the `CARGO_*` consts), labels dim and values bright,
//! in the bar's font. An empty or inverted area draws nothing
//! (@0x4c325-0x4c33b); the original's clear to the background and its
//! blit to the window are left out, as a draw list needs neither.
//!
//! Flight draws no date. The original shows it only on the player info
//! screen (`STR#` 2002 #252 "Current Date:"), and no `ïntf` area is for
//! it.
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
//! draws them: a pixel for each ship and the player, a filled 2×2 square
//! for a station or gate or a heavy ship under a density scanner (framing
//! a rect that small fills it), an oval framed as [`oval_frame_pixels`]
//! rasterizes it, the IFF backdrop filled black, the static as its `ppat`
//! tiled, and the centre arrow's lines as [`line_pixels`] rasterizes
//! them.
//!
//! Every `ïntf` area is relative to the bar's top-left corner. The bar is
//! as wide as its background picture (194 pixels in every stock bar) and
//! sits against the right edge of the screen, at the top.

use nova_sim::hyperspace::max_jumps;
use nova_sim::{JumpReadiness, Reserves, Special, StatusCargo};

use nova_sim::Gauge;

use super::catalog::{GovtId, StatusBarLayout, StatusBars};
use super::pen::Pen;
use super::radar::{self, IRect, RadarColor, RadarMark, RadarScene};
use crate::draw::fill_rect;
use crate::geometry::{Bounds, Point};
use crate::oval::oval_frame_pixels;
use crate::spaceport::credits::comma_number;
use crate::text::TextMetrics;
use crate::ui::text_box::TextBox;
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
    let layout = catalog.status_bar(interface_for(catalog, government)?)?;
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
    let id = interface_for(catalog, government)?;
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
/// below 128 meaning 128 (@0x7364d-0x7365a). The original keeps the ID
/// it loaded (`_currentIntf`) and loads the bar again only for another
/// (@0x73663-0x73672).
pub(crate) fn interface_for(
    catalog: &impl StatusBars,
    government: Option<GovtId>,
) -> Result<i16, String> {
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

/// Where the bar's top-left corner goes in a view `view_width` wide:
/// against its right edge, as wide as its background, or
/// [`STATUS_BAR_WIDTH`] without one. The original draws the bar at the
/// window's width less 194 whatever the width (`_BlitStatusArea`
/// @0x4957d-0x495a7, `_RedrawStatus` @0x4cafd-0x4cc04), and offsets each
/// of its `ïntf` areas by the same (`_DrawStatusRadar` @0x49769 and the
/// others).
#[must_use]
pub fn bar_origin(bar: &StatusBar, view_width: f32) -> Point {
    let width = bar.background.map_or(STATUS_BAR_WIDTH, |b| b.width);
    Point::new(view_width - width, 0.0)
}

/// `STR#` 2002 #342: the nav area with the nav system off (nav mode −1).
pub const NAV_OFF: &str = "Nav System Off";
/// `STR#` 2002 #343: the nav area's label over a selected stellar.
pub const NAV_STELLAR: &str = "Stellar Navigation";
/// `STR#` 2002 #344: the nav area's value with no destination.
pub const NAV_NO_DESTINATION: &str = "No Destination";
/// `STR#` 2002 #345: the nav area's label over a plotted jump.
pub const NAV_HYPERSPACE: &str = "Hyperspace";
/// `STR#` 2002 #346: the nav area's name for a system not yet explored.
pub const NAV_UNEXPLORED: &str = "Unexplored System";
/// The label's baseline below the nav area's top (@0x4a0de, @0x4a1f5).
pub const NAV_LABEL_BASELINE: f32 = 12.0;
/// "Nav System Off"'s baseline below the nav area's top (@0x4a07f).
pub const NAV_OFF_BASELINE: f32 = 22.0;
/// The value's baseline below the nav area's top (@0x4a11d, @0x4a174,
/// @0x4a34c, @0x4a3b0).
pub const NAV_VALUE_BASELINE: f32 = 29.0;

/// `STR#` 2002 #19: the cargo area's free space label.
pub const CARGO_FREE: &str = "Free:";
/// `STR#` 2002 #20: the cargo area's mission cargo and `jünk` label.
pub const CARGO_SPECIAL: &str = "Special:";
/// `STR#` 2002 #21: "Special:"'s value for more than one.
pub const CARGO_MULTIPLE: &str = "Multiple";
/// `STR#` 2002 #33, which the credits line's label is made from
/// ([`credits_label`]).
pub const CARGO_CREDITS: &str = "credits";
/// The first commodity row's baseline below the cargo area's top
/// (@0x4c44b, @0x4c480); "Free:" shares it (@0x4c4dc, @0x4c51b).
pub const CARGO_ROW_BASELINE: f32 = 12.0;
/// How far each commodity row's baseline is below the one before
/// (@0x4c4c6).
pub const CARGO_ROW_STEP: f32 = 14.0;
/// A commodity row's label, right of the cargo area's left (@0x4c457).
pub const CARGO_LABEL_LEFT: f32 = 3.0;
/// A commodity row's tons, right of the cargo area's left (@0x4c48c).
pub const CARGO_TONS_LEFT: f32 = 41.0;
/// The "Free:", "Special:" and credits labels, right of the cargo area's
/// left (@0x4c4e8, @0x4c627, @0x4c708).
pub const CARGO_SIDE_LEFT: f32 = 77.0;
/// The free tons, right of the cargo area's left (@0x4c527).
pub const CARGO_FREE_VALUE_LEFT: f32 = 110.0;
/// "Special:"'s and the credits line's values, right of the cargo area's
/// left (@0x4c666, @0x4c782).
pub const CARGO_VALUE_LEFT: f32 = 87.0;
/// "Special:"'s baseline below the cargo area's top (@0x4c61b).
pub const CARGO_SPECIAL_BASELINE: f32 = 30.0;
/// "Special:"'s value's baseline below the cargo area's top (@0x4c65a).
pub const CARGO_SPECIAL_VALUE_BASELINE: f32 = 46.0;
/// The credits label's baseline below the cargo area's top (@0x4c6fc).
pub const CARGO_CREDITS_BASELINE: f32 = 66.0;
/// The credits value's baseline below the cargo area's top (@0x4c776),
/// the lowest the area draws: the sixth commodity row's is the same.
pub const CARGO_CREDITS_VALUE_BASELINE: f32 = 82.0;

/// The credits line's label made from `word` (`STR#` 2002 #33) as
/// `_DrawStatusCargo` makes it: its first character upper-cased
/// (@0x4c73a-0x4c752, nothing for an empty word), then ":" (the Pascal
/// string at 0xd97b2).
#[must_use]
pub fn credits_label(word: &str) -> String {
    let mut chars = word.chars();
    let first = chars.next().map(char::to_uppercase);
    first
        .into_iter()
        .flatten()
        .chain(chars)
        .chain([':'])
        .collect()
}

/// What the nav area shows: one variant for each of the original's nav
/// modes (player +0x6a) the engine reaches. Modes 0 and 1 are never set
/// on the player, and mode 4 (the hypergate map) is reset to −1
/// (@0x63fa2) before any status draw; `_DrawStatusNav` has no branch for
/// them, so they would show the bare background.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum NavDisplay {
    /// Mode −1, the nav system off, whatever the course: "Nav System Off"
    /// (@0x4a044-0x4a097).
    #[default]
    Off,
    /// Mode 2, a stellar selected (@0x4a09e-0x4a18c): its name, or `None`
    /// for "No Destination".
    Stellar(Option<String>),
    /// Mode 3, hyperspace (@0x4a198-0x4a3cb): the system selected, and
    /// whether the ship can jump there, which picks the colours.
    Hyperspace {
        /// What the value line shows.
        destination: NavDestination,
        /// Whether the ship can jump there.
        readiness: JumpReadiness,
    },
}

/// The value line of [`NavDisplay::Hyperspace`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavDestination {
    /// An explored system, by name (its +0x90 > 0, @0x4a311-0x4a330).
    Named(String),
    /// A system not yet explored: "Unexplored System".
    Unexplored,
    /// No hop to name (+0x6c == −1, @0x4a21d): "No Destination", dim.
    None,
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
    /// What the cargo area lists, or `None` with no session to list.
    pub cargo: Option<&'a StatusCargo>,
    /// Whether the player's ship is disabled (`_IsDisabled`), which hides
    /// the fuel gauge.
    pub disabled: bool,
}

/// Draws `bar` showing `state` against the right edge of a view
/// `view_width` wide ([`bar_origin`]): its background, the radar (see the
/// module docs), then the shield, armour and fuel bars, the nav area,
/// its lines centred as `metrics` measures them, and the cargo area.
pub fn draw(
    list: &mut DrawList,
    bar: &StatusBar,
    state: &HudState,
    metrics: Option<&dyn TextMetrics>,
    view_width: f32,
) {
    let origin = bar_origin(bar, view_width);
    let layout = &bar.layout;
    if let Some(background) = bar.background {
        list.picture(ImageKey::picture(background.id), origin);
    }
    let radar = IRect::from_bounds(layout.radar.offset(origin));
    for mark in radar::marks(radar, state.radar, state.blink_on) {
        draw_mark(list, layout, radar, mark);
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
    draw_nav(list, layout, origin, &state.nav, metrics);
    if let Some(cargo) = state.cargo {
        draw_cargo(list, layout, origin, cargo, metrics);
    }
}

/// Draws one radar mark on `radar`: in the layout's radar colours, or a
/// fixed one ([`RadarColor::fixed`]). A pixel is a 1-pixel dot over it (a
/// QuickDraw pixel at (x, y) covers [x, x + 1)), a square and a fill are
/// filled, an oval is [`oval_frame_pixels`] at a pen of 1, the static is its `ppat`
/// tiled over its rect, and a line is [`line_pixels`], those inside the
/// radar only (@0x49f33 copies only the radar's rect to the window).
fn draw_mark(list: &mut DrawList, layout: &StatusBarLayout, radar: IRect, mark: RadarMark) {
    let color = |color: RadarColor| match color {
        RadarColor::Dim => layout.dim_radar,
        RadarColor::Bright => layout.bright_radar,
        fixed => fixed.fixed().unwrap_or(layout.dim_radar),
    };
    match mark {
        RadarMark::Pixel { x, y, color: c } => pixel(list, x, y, color(c)),
        RadarMark::Square { rect, color: c } | RadarMark::Fill { rect, color: c } => {
            fill_rect(list, rect.to_bounds(), color(c));
        }
        RadarMark::Oval { rect, color: c } => {
            for (x, y) in oval_frame_pixels(rect, 1) {
                pixel(list, x, y, color(c));
            }
        }
        RadarMark::Pattern { rect, id } => {
            list.tiled_picture(ImageKey::pattern(id), rect.to_bounds());
        }
        RadarMark::Line { from, to, color: c } => {
            for (x, y) in line_pixels(from, to) {
                if radar.holds(x, y) {
                    pixel(list, x, y, color(c));
                }
            }
        }
    }
}

/// The pixels of a 1-pixel line from `from` to `to`, both ends included,
/// in order from `from`: Bresenham's. QuickDraw's own `LineTo` raster is
/// in the Toolbox, not in the original's binary, so this is our
/// approximation of it, exact for horizontal, vertical and diagonal
/// lines.
#[must_use]
pub fn line_pixels(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = ((to.0 - from.0).abs(), -(to.1 - from.1).abs());
    let (sx, sy) = ((to.0 - from.0).signum(), (to.1 - from.1).signum());
    let (mut x, mut y) = from;
    let mut error = dx + dy;
    let mut pixels = vec![from];
    while (x, y) != to {
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
        pixels.push((x, y));
    }
    pixels
}

/// Draws the pixel at (`x`, `y`) in `color`.
fn pixel(list: &mut DrawList, x: i32, y: i32, color: Color) {
    list.dot(Point::new(x as f32 + 0.5, y as f32 + 0.5), 1.0, color);
}

/// The face flight messages are drawn in. `_DisplayComm` (@0x90201) is
/// passed font 0, the system font, by every flight caller but the FPS
/// readout; the game bundles `Fonts/Charcoal.ttf` to stand in for the
/// classic Mac OS system font, and the renderer has no Lucida Grande, so
/// Charcoal it is.
pub const MESSAGE_FONT: Font = Font::Charcoal;
/// The size flight messages are drawn at: 12 pt, as every flight caller
/// passes `_DisplayComm`.
pub const MESSAGE_TEXT_SIZE: f32 = 12.0;

/// The `_DisplayComm` rect for a window of `view` (width, height), as
/// `_InitSprites` sets it up (@0x20496-0x20526), in whole pixels: left
/// 25, right 244 short of the right edge, bottom 5 above the bottom edge,
/// and as tall as 29470 divided by its width, truncated. In the 1024 x
/// 768 window it is (25, 724)-(780, 763).
#[must_use]
pub fn message_box(view: (f32, f32)) -> Bounds {
    // Window sizes are whole pixels.
    #[allow(clippy::cast_possible_truncation)]
    let (width, height) = (view.0 as i32, view.1 as i32);
    let (left, right, bottom) = (25, width - 244, height - 5);
    let top = bottom - 29470 / (right - left);
    #[allow(clippy::cast_precision_loss)]
    let point = |x: i32, y: i32| Point::new(x as f32, y as f32);
    Bounds {
        min: point(left, top),
        max: point(right, bottom),
    }
}

/// Draws the flight message `text` in [`message_box`] of a view `view`
/// in size, as
/// `_DisplayComm` (@0x90294-0x902f5) draws it with `TETextBox`:
/// left-justified and wrapped to the box, in [`MESSAGE_FONT`] at
/// [`MESSAGE_TEXT_SIZE`], white (`InitColors` @0x1bece sets the colour
/// it is passed to 0xffff, 0xffff, 0xffff). See [`draw_message_in`].
pub fn draw_message(
    list: &mut DrawList,
    text: &str,
    view: (f32, f32),
    metrics: Option<&dyn TextMetrics>,
) {
    draw_message_in(list, text, message_box(view), metrics);
}

/// Draws the flight message `text` in `area`: with `metrics`, through a
/// [`TextBox`] as `TETextBox` draws it, each line it wraps to as a text
/// of its own down from the area's top-left, every line whose top is
/// inside the area; without, one text wrapped by the renderer at the
/// area's width.
///
/// `TETextBox` clips to the rect, so a line partly inside shows its top,
/// clipped to the box. In the stock 1024 x 768 window [`message_box`] is
/// 39 pixels tall: two whole lines of Charcoal 12 fit, and a third shows
/// its top 10 pixels.
///
/// `_DisplayComm` erases the rect in the system background colour
/// before `TETextBox` (`SetSystemBackgroundBackColor` @0x902cf: the
/// current system's `BkgndColor`, or black without one, @0x6e06). This
/// paints nothing behind the text, so the starfield shows through it.
pub fn draw_message_in(
    list: &mut DrawList,
    text: &str,
    area: Bounds,
    metrics: Option<&dyn TextMetrics>,
) {
    let (font, size) = (MESSAGE_FONT, MESSAGE_TEXT_SIZE);
    let Some(metrics) = metrics else {
        list.text_in(font, text, area.min, size, Some(area.width()), Color::WHITE);
        return;
    };
    TextBox::new(text, font, size, Color::WHITE, area, &metrics).draw(list);
}

/// Draws `cargo` in the layout's cargo area, at the bar's `origin`, as
/// `_DrawStatusCargo` (@0x4c313-0x4c7ad) does: nothing in an empty or
/// inverted area (@0x4c325-0x4c33b); otherwise each held commodity's
/// row, "Free:", "Special:" when there is any, and the credits line,
/// labels in the dim text colour and values in the bright, each from
/// its pen position on its baseline. The cash is written as
/// `_DrawCommaNumber` writes it ([`comma_number`]).
fn draw_cargo(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    cargo: &StatusCargo,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.cargo.offset(origin);
    if area.min.x >= area.max.x || area.min.y >= area.max.y {
        return;
    }
    let (dim, bright) = (layout.dim_text, layout.bright_text);
    let size = layout.font_size;
    let mut pen = Pen {
        list,
        font: layout.font,
        metrics,
    };
    let mut line = |text: &str, x: f32, baseline: f32, color: Color| {
        pen.at(text, area.min.x + x, area.min.y + baseline, size, color);
    };
    let mut baseline = CARGO_ROW_BASELINE;
    for row in &cargo.rows {
        if let Some(row) = row {
            line(&row.label, CARGO_LABEL_LEFT, baseline, dim);
            line(&row.tons.to_string(), CARGO_TONS_LEFT, baseline, bright);
        }
        baseline += CARGO_ROW_STEP;
    }
    line(CARGO_FREE, CARGO_SIDE_LEFT, CARGO_ROW_BASELINE, dim);
    let free = cargo.free.to_string();
    line(&free, CARGO_FREE_VALUE_LEFT, CARGO_ROW_BASELINE, bright);
    if let Some(special) = &cargo.special {
        line(CARGO_SPECIAL, CARGO_SIDE_LEFT, CARGO_SPECIAL_BASELINE, dim);
        let value = match special {
            Special::Named(name) => name.as_str(),
            Special::Multiple => CARGO_MULTIPLE,
        };
        line(
            value,
            CARGO_VALUE_LEFT,
            CARGO_SPECIAL_VALUE_BASELINE,
            bright,
        );
    }
    let label = credits_label(CARGO_CREDITS);
    line(&label, CARGO_SIDE_LEFT, CARGO_CREDITS_BASELINE, dim);
    let cash = comma_number(cargo.cash);
    line(
        &cash,
        CARGO_VALUE_LEFT,
        CARGO_CREDITS_VALUE_BASELINE,
        bright,
    );
}

/// Draws `nav` in the layout's nav area, at the bar's `origin`, as
/// `_DrawStatusNav` (@0x49f43-0x4a451) does (see the module docs): nothing
/// in an empty or inverted area, otherwise each line centred between the
/// area's sides on its baseline, measured by `metrics`. A jump's label is
/// bright once it is under way, and its destination once the ship is
/// clear (@0x4a272-0x4a2f9); "No Destination" is always dim.
fn draw_nav(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    nav: &NavDisplay,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.nav.offset(origin);
    if area.min.x >= area.max.x || area.min.y >= area.max.y {
        return;
    }
    let (dim, bright) = (layout.dim_text, layout.bright_text);
    let lines: [Option<(&str, f32, Color)>; 2] = match nav {
        NavDisplay::Off => [Some((NAV_OFF, NAV_OFF_BASELINE, dim)), None],
        NavDisplay::Stellar(name) => [
            Some((NAV_STELLAR, NAV_LABEL_BASELINE, dim)),
            Some(
                name.as_deref()
                    .map_or((NAV_NO_DESTINATION, NAV_VALUE_BASELINE, dim), |name| {
                        (name, NAV_VALUE_BASELINE, bright)
                    }),
            ),
        ],
        NavDisplay::Hyperspace {
            destination,
            readiness,
        } => {
            let (label, value) = match readiness {
                JumpReadiness::Blocked => (dim, dim),
                JumpReadiness::Clear => (dim, bright),
                JumpReadiness::Underway => (bright, bright),
            };
            let value = match destination {
                NavDestination::Named(name) => (name.as_str(), value),
                NavDestination::Unexplored => (NAV_UNEXPLORED, value),
                NavDestination::None => (NAV_NO_DESTINATION, dim),
            };
            [
                Some((NAV_HYPERSPACE, NAV_LABEL_BASELINE, label)),
                Some((value.0, NAV_VALUE_BASELINE, value.1)),
            ]
        }
    };
    let mut pen = Pen {
        list,
        font: layout.font,
        metrics,
    };
    for (text, baseline, color) in lines.into_iter().flatten() {
        pen.centred_text(
            text,
            area.min.x,
            area.max.x,
            area.min.y + baseline,
            layout.font_size,
            color,
        );
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

/// Draws why no status bar can be shown, where the bar would go in a view
/// `view_width` wide.
pub fn draw_unavailable(list: &mut DrawList, reason: &str, view_width: f32) {
    list.text_in(
        Font::Geneva,
        format!("Status bar unavailable: {reason}"),
        Point::new(view_width - STATUS_BAR_WIDTH, 0.0),
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
    use crate::system::camera::VIEW_SIZE;
    use crate::text::fixture::MonoMetrics;
    use nova_sim::{ShipIff, StellarIff};

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

        fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
            self.asked.borrow_mut().push(format!("cicn {id}"));
            None
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
        assert_eq!(bar_origin(&bar, 1024.0), at(830.0, 0.0));
        assert_eq!(bar_origin(&bar, 1365.0), at(1171.0, 0.0), "widened");
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 10.0,
        });
        assert_eq!(bar_origin(&bar, 1024.0), at(824.0, 0.0));
        bar.background = None;
        assert_eq!(bar_origin(&bar, 1024.0), at(830.0, 0.0));
        assert_eq!(bar_origin(&bar, 1366.0), at(1172.0, 0.0));
        assert_eq!(STATUS_BAR_WIDTH, 194.0);
    }

    #[test]
    fn a_widened_view_draws_the_whole_bar_moved_to_its_right_edge() {
        let bar = stock();
        let hud = HudState {
            nav: NavDisplay::Stellar(Some("Earth".to_owned())),
            ..state(reserves(15.0, 30.0, 150.0))
        };
        let classic = drawn(&bar, &hud);
        let mut widened = DrawList::new();
        draw(&mut widened, &bar, &hud, Some(&MonoMetrics), 1365.0);
        let mut moved = DrawList::new();
        moved.extend_translated(&classic, at(341.0, 0.0));
        assert_eq!(widened, moved);
        let mut unavailable = DrawList::new();
        draw_unavailable(&mut unavailable, "no ïntf 128", 1365.0);
        assert!(matches!(
            unavailable.iter().next(),
            Some(DrawCommand::Text { origin, .. }) if *origin == at(1171.0, 0.0)
        ));
    }

    // The radar.

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
        draw(&mut list, bar, state, Some(&MonoMetrics), VIEW_SIZE.0);
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
        iff: false,
        density_scanner: false,
        static_pattern: None,
        hyperspace_mode: false,
    };

    fn state(reserves: Reserves) -> HudState<'static> {
        HudState {
            radar: &NO_RADAR,
            blink_on: false,
            reserves,
            nav: NavDisplay::Off,
            cargo: None,
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
                    iff: StellarIff::Cleared,
                },
                RadarStellar {
                    position: at(320.0, 0.0),
                    square: false,
                    sprite_width: Some(40),
                    iff: StellarIff::Cleared,
                },
                RadarStellar {
                    position: at(-64.0, 0.0),
                    square: false,
                    sprite_width: Some(200),
                    iff: StellarIff::Cleared,
                },
            ],
            ships: vec![
                RadarShip {
                    position: at(-160.0, 0.0),
                    targeted: false,
                    iff: ShipIff::Other,
                    heavy: false,
                },
                RadarShip {
                    position: at(0.0, 160.0),
                    targeted: true,
                    iff: ShipIff::Other,
                    heavy: false,
                },
            ],
            ..RadarScene::default()
        };
        let cargo = full_cargo();
        let hud = HudState {
            radar: &scene,
            blink_on: true,
            reserves: reserves(30.0, 60.0, 300.0),
            nav: NavDisplay::Stellar(Some("Earth".to_owned())),
            cargo: Some(&cargo),
            disabled: false,
        };
        let list = drawn(&stock(), &hud);
        let mut expected = DrawList::new();
        expected.picture(ImageKey::picture(700), at(830.0, 0.0));
        fill_rect(&mut expected, rect(925.0, 85.0, 927.0, 87.0), DIM_RADAR);
        for (x, y) in [(935, 95), (936, 95), (935, 96), (936, 96)] {
            expected.dot(pixel_at(x, y), 1.0, DIM_RADAR);
        }
        for (x, y) in oval_frame_pixels(IRect::new(921, 93, 927, 99), 1) {
            expected.dot(pixel_at(x, y), 1.0, DIM_RADAR);
        }
        expected.dot(pixel_at(921, 96), 1.0, DIM_RADAR);
        expected.dot(pixel_at(926, 101), 1.0, RADAR);
        expected.dot(pixel_at(926, 96), 1.0, RADAR);
        expected.line(at(865.0, 202.5), at(1014.0, 202.5), 7.0, SHIELD);
        expected.line(at(865.0, 219.5), at(1014.0, 219.5), 7.0, ARMOR);
        expected.line(at(865.0, 237.5), at(1014.0, 237.5), 7.0, FUEL_FULL);
        expected.push(nav_text(NAV_STELLAR, NAV_LABEL_BASELINE, Color::DIM));
        expected.push(nav_text("Earth", NAV_VALUE_BASELINE, TEXT));
        for line in cargo_drawn(&stock(), &cargo) {
            expected.push(line);
        }
        assert_eq!(list, expected, "the cargo area last");
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
                iff: StellarIff::Cleared,
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
                iff: ShipIff::Other,
                heavy: false,
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

    // IFF, static and the arrow.

    fn radar_drawn(scene: &RadarScene, blink_on: bool) -> DrawList {
        let hud = HudState {
            radar: scene,
            blink_on,
            ..state(reserves(0.0, 0.0, 0.0))
        };
        drawn(&stock(), &hud)
    }

    /// The radar's commands: those after the background picture and before
    /// the texts.
    fn radar_commands(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .skip(1)
            .take_while(|command| !matches!(command, DrawCommand::Text { .. }))
            .cloned()
            .collect()
    }

    const BLACK: Color = Color::rgba(0, 0, 0, 255);
    const CYAN: Color = Color::rgba(0, 255, 255, 255);

    #[test]
    fn with_iff_the_radar_is_filled_black_then_drawn_in_fixed_colours() {
        let scene = RadarScene {
            iff: true,
            ships: vec![RadarShip {
                position: at(-160.0, 0.0),
                targeted: false,
                iff: ShipIff::Threat,
                heavy: false,
            }],
            stellars: vec![RadarStellar {
                position: at(0.0, -320.0),
                square: true,
                sprite_width: None,
                iff: StellarIff::Unlandable,
            }],
            ..RadarScene::default()
        };
        let mut expected = DrawList::new();
        fill_rect(&mut expected, rect(838.0, 8.0, 1014.0, 184.0), BLACK);
        fill_rect(
            &mut expected,
            rect(925.0, 85.0, 927.0, 87.0),
            Color::rgba(128, 128, 128, 255),
        );
        expected.dot(pixel_at(921, 96), 1.0, Color::rgba(255, 0, 0, 255));
        expected.dot(pixel_at(926, 96), 1.0, CYAN);
        assert_eq!(
            radar_commands(&radar_drawn(&scene, false)),
            expected.iter().cloned().collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_layouts_colours_stay_dim_and_bright() {
        let scene = RadarScene {
            ships: vec![RadarShip {
                position: at(-160.0, 0.0),
                targeted: false,
                iff: ShipIff::Threat,
                heavy: false,
            }],
            ..RadarScene::default()
        };
        let list = radar_drawn(&scene, false);
        assert_eq!(dots(&list, DIM_RADAR), [pixel_at(921, 96)]);
        assert_eq!(dots(&list, RADAR), [pixel_at(926, 96)]);
    }

    #[test]
    fn static_is_its_pattern_tiled_over_the_radar() {
        let scene = RadarScene {
            static_pattern: Some(133),
            ..RadarScene::default()
        };
        let commands = radar_commands(&radar_drawn(&scene, false));
        assert_eq!(
            commands[0],
            DrawCommand::TiledPicture {
                image: ImageKey::pattern(133),
                area: rect(838.0, 8.0, 1014.0, 184.0),
                origin: Point::new(0.0, 0.0),
            }
        );
        assert_eq!(commands.len(), 2, "then the player");
    }

    #[test]
    fn a_line_is_every_pixel_from_end_to_end() {
        assert_eq!(line_pixels((3, 4), (3, 4)), [(3, 4)]);
        assert_eq!(
            line_pixels((0, 0), (3, 0)),
            [(0, 0), (1, 0), (2, 0), (3, 0)]
        );
        assert_eq!(
            line_pixels((3, 0), (0, 0)),
            [(3, 0), (2, 0), (1, 0), (0, 0)]
        );
        assert_eq!(
            line_pixels((5, 2), (5, -1)),
            [(5, 2), (5, 1), (5, 0), (5, -1)]
        );
        assert_eq!(
            line_pixels((0, 0), (3, 3)),
            [(0, 0), (1, 1), (2, 2), (3, 3)]
        );
        assert_eq!(line_pixels((0, 0), (-2, 2)), [(0, 0), (-1, 1), (-2, 2)]);
        assert_eq!(
            line_pixels((0, 0), (5, 2)),
            [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)]
        );
        assert_eq!(
            line_pixels((0, 0), (2, 5)),
            [(0, 0), (0, 1), (1, 2), (1, 3), (2, 4), (2, 5)]
        );
        assert_eq!(
            line_pixels((0, 0), (-5, -1)),
            [(0, 0), (-1, 0), (-2, 0), (-3, -1), (-4, -1), (-5, -1)]
        );
    }

    #[test]
    fn the_arrows_pixels_outside_the_radar_are_dropped() {
        // A 60 × 40 radar, (840, 30)-(900, 70) on the bar at (830, 0),
        // centred on (870, 50): the arrow points left, its shaft from
        // (845, 50) to (820, 50) and its heads past that, so only the
        // shaft's first six pixels are inside it.
        let mut bar = stock();
        bar.layout.radar = rect(10.0, 30.0, 70.0, 70.0);
        let scene = RadarScene {
            player: at(3000.0, 0.0),
            stellars: vec![RadarStellar {
                position: at(0.0, 0.0),
                square: false,
                sprite_width: None,
                iff: StellarIff::Cleared,
            }],
            ..RadarScene::default()
        };
        let hud = HudState {
            radar: &scene,
            blink_on: true,
            ..state(reserves(0.0, 0.0, 0.0))
        };
        assert_eq!(
            dots(&drawn(&bar, &hud), DIM_RADAR),
            (840..=845)
                .rev()
                .map(|x| pixel_at(x, 50))
                .collect::<Vec<_>>()
        );
    }

    // The nav area.

    fn texts(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .filter(|command| matches!(command, DrawCommand::Text { .. }))
            .cloned()
            .collect()
    }

    /// `text` centred in the stock bar's nav area on `baseline` below its
    /// top: at (830, 0) its (8, 254)-(184, 286) is (838, 254)-(1014, 286),
    /// centred on 926, in the layout's Charcoal 11, 6.6 a character in
    /// [`MonoMetrics`], its top 11 above its baseline.
    fn nav_text(text: &str, baseline: f32, color: Color) -> DrawCommand {
        let half = (MonoMetrics.width(Font::Charcoal, 11.0, text) / 2.0).trunc();
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Charcoal,
            origin: at(926.0 - half, 254.0 + baseline - 11.0),
            size: 11.0,
            wrap_width: None,
            color,
        }
    }

    fn nav_drawn(nav: NavDisplay) -> Vec<DrawCommand> {
        let hud = HudState {
            nav,
            ..state(reserves(30.0, 60.0, 300.0))
        };
        texts(&drawn(&stock(), &hud))
    }

    #[test]
    fn the_nav_strings_are_the_originals() {
        assert_eq!(
            [
                NAV_OFF,
                NAV_STELLAR,
                NAV_NO_DESTINATION,
                NAV_HYPERSPACE,
                NAV_UNEXPLORED
            ],
            [
                "Nav System Off",
                "Stellar Navigation",
                "No Destination",
                "Hyperspace",
                "Unexplored System"
            ]
        );
        assert_eq!(
            [NAV_LABEL_BASELINE, NAV_OFF_BASELINE, NAV_VALUE_BASELINE],
            [12.0, 22.0, 29.0]
        );
        assert_eq!(NavDisplay::default(), NavDisplay::Off);
    }

    #[test]
    fn nothing_selected_is_nav_system_off_dim() {
        assert_eq!(
            nav_drawn(NavDisplay::Off),
            [nav_text(NAV_OFF, NAV_OFF_BASELINE, Color::DIM)]
        );
    }

    #[test]
    fn a_selected_stellar_is_stellar_navigation_over_its_bright_name() {
        assert_eq!(
            nav_drawn(NavDisplay::Stellar(Some("Earth".to_owned()))),
            [
                nav_text(NAV_STELLAR, NAV_LABEL_BASELINE, Color::DIM),
                nav_text("Earth", NAV_VALUE_BASELINE, TEXT)
            ]
        );
    }

    #[test]
    fn a_stellar_target_with_no_stellar_is_stellar_navigation_over_no_destination_dim() {
        assert_eq!(
            nav_drawn(NavDisplay::Stellar(None)),
            [
                nav_text(NAV_STELLAR, NAV_LABEL_BASELINE, Color::DIM),
                nav_text(NAV_NO_DESTINATION, NAV_VALUE_BASELINE, Color::DIM)
            ]
        );
    }

    fn hyperspace(destination: NavDestination, readiness: JumpReadiness) -> NavDisplay {
        NavDisplay::Hyperspace {
            destination,
            readiness,
        }
    }

    #[test]
    fn a_jumps_label_is_bright_once_underway_and_its_destination_once_clear() {
        use JumpReadiness::{Blocked, Clear, Underway};
        let sol = || NavDestination::Named("Sol".to_owned());
        for (destination, value) in [(sol(), "Sol"), (NavDestination::Unexplored, NAV_UNEXPLORED)] {
            for (readiness, label_color, value_color) in [
                (Blocked, Color::DIM, Color::DIM),
                (Clear, Color::DIM, TEXT),
                (Underway, TEXT, TEXT),
            ] {
                assert_eq!(
                    nav_drawn(hyperspace(destination.clone(), readiness)),
                    [
                        nav_text(NAV_HYPERSPACE, NAV_LABEL_BASELINE, label_color),
                        nav_text(value, NAV_VALUE_BASELINE, value_color)
                    ],
                    "{destination:?} {readiness:?}"
                );
            }
        }
    }

    #[test]
    fn hyperspace_with_no_destination_is_no_destination_dim() {
        use JumpReadiness::{Blocked, Clear, Underway};
        for (readiness, label_color) in
            [(Blocked, Color::DIM), (Clear, Color::DIM), (Underway, TEXT)]
        {
            assert_eq!(
                nav_drawn(hyperspace(NavDestination::None, readiness)),
                [
                    nav_text(NAV_HYPERSPACE, NAV_LABEL_BASELINE, label_color),
                    nav_text(NAV_NO_DESTINATION, NAV_VALUE_BASELINE, Color::DIM)
                ],
                "{readiness:?}"
            );
        }
    }

    /// The stock bar with a 200-wide background, Geneva 12 (6 a character
    /// in [`MonoMetrics`]) and a nav area of (10, 300)-(111, 340): at
    /// (824, 0) that is (834, 300)-(935, 340), centred on
    /// trunc(1769 / 2) = 884.
    fn odd_bar() -> StatusBar {
        let mut bar = stock();
        bar.background = Some(Background {
            id: 700,
            width: 200.0,
            height: 767.0,
        });
        bar.layout.font = Font::Geneva;
        bar.layout.font_size = 12.0;
        bar.layout.nav = rect(10.0, 300.0, 111.0, 340.0);
        bar.layout.dim_text = FUEL_PARTIAL;
        bar.layout.bright_text = SHIELD;
        bar
    }

    fn geneva(text: &str, x: f32, top: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: at(x, top),
            size: 12.0,
            wrap_width: None,
            color,
        }
    }

    #[test]
    fn the_nav_lines_are_centred_on_their_baselines_in_a_non_stock_area() {
        let bar = odd_bar();
        let shown = |nav| {
            let hud = HudState {
                nav,
                ..state(reserves(30.0, 60.0, 300.0))
            };
            texts(&drawn(&bar, &hud))
        };
        // "Stellar Navigation" is 18 × 6 = 108: 884 − 54 = 830, top
        // 300 + 12 − 12. "Earth" (odd) is 30: 884 − 15 = 869, top
        // 300 + 29 − 12. "Nav System Off" is 84: 884 − 42 = 842, top
        // 300 + 22 − 12.
        assert_eq!(
            shown(NavDisplay::Stellar(Some("Earth".to_owned()))),
            [
                geneva(NAV_STELLAR, 830.0, 300.0, FUEL_PARTIAL),
                geneva("Earth", 869.0, 317.0, SHIELD)
            ]
        );
        assert_eq!(
            shown(NavDisplay::Off),
            [geneva(NAV_OFF, 842.0, 310.0, FUEL_PARTIAL)]
        );
        // "Sol" is 18: 884 − 9 = 875; "Hyperspace" is 60: 884 − 30.
        assert_eq!(
            shown(hyperspace(
                NavDestination::Named("Sol".to_owned()),
                JumpReadiness::Underway
            )),
            [
                geneva(NAV_HYPERSPACE, 854.0, 300.0, SHIELD),
                geneva("Sol", 875.0, 317.0, SHIELD)
            ]
        );
    }

    #[test]
    fn without_metrics_the_nav_lines_start_at_the_areas_left() {
        let hud = HudState {
            nav: NavDisplay::Stellar(Some("Earth".to_owned())),
            ..state(reserves(30.0, 60.0, 300.0))
        };
        let mut list = DrawList::new();
        draw(&mut list, &odd_bar(), &hud, None, VIEW_SIZE.0);
        assert_eq!(
            texts(&list),
            [
                geneva(NAV_STELLAR, 834.0, 300.0, FUEL_PARTIAL),
                geneva("Earth", 834.0, 317.0, SHIELD)
            ]
        );
    }

    #[test]
    fn an_empty_nav_area_draws_nothing() {
        let hud = HudState {
            nav: NavDisplay::Stellar(Some("Earth".to_owned())),
            ..state(reserves(30.0, 60.0, 300.0))
        };
        for nav in [
            rect(10.0, 300.0, 10.0, 340.0),
            rect(10.0, 300.0, 111.0, 300.0),
            rect(111.0, 300.0, 10.0, 340.0),
            rect(10.0, 340.0, 111.0, 300.0),
        ] {
            let mut bar = odd_bar();
            bar.layout.nav = nav;
            assert_eq!(texts(&drawn(&bar, &hud)), [], "{nav:?}");
        }
        let mut bar = odd_bar();
        bar.layout.nav = rect(10.0, 300.0, 11.0, 301.0);
        assert_eq!(texts(&drawn(&bar, &hud)).len(), 2, "one pixel is enough");
    }

    // The cargo area.

    use nova_sim::CommodityRow;

    fn row(label: &str, tons: i64) -> CommodityRow {
        CommodityRow {
            label: label.to_owned(),
            tons,
        }
    }

    /// 12 tons of food and 3 of equipment, 25 tons free, one mission's
    /// passengers, and 1,234 credits.
    fn full_cargo() -> StatusCargo {
        StatusCargo {
            rows: [
                Some(row("Food:", 12)),
                None,
                None,
                None,
                None,
                Some(row("Equ:", 3)),
            ],
            free: 25,
            special: Some(Special::Named("Passengers".to_owned())),
            cash: 1234,
        }
    }

    /// The texts drawn for `cargo` on `bar`, the nav area's left out.
    fn cargo_drawn(bar: &StatusBar, cargo: &StatusCargo) -> Vec<DrawCommand> {
        let hud = HudState {
            cargo: Some(cargo),
            ..state(reserves(30.0, 60.0, 300.0))
        };
        texts(&drawn(bar, &hud))
            .into_iter()
            .filter(|command| !matches!(command, DrawCommand::Text { text, .. } if text == NAV_OFF))
            .collect()
    }

    /// `text` from `x` right of the stock bar's cargo area's left, on
    /// `baseline` below its top: at (830, 0) its (8, 458)-(184, 552) is
    /// (838, 458)-(1014, 552), in Charcoal 11, its top 11 above its
    /// baseline.
    fn cargo_text(text: &str, x: f32, baseline: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Charcoal,
            origin: at(838.0 + x, 458.0 + baseline - 11.0),
            size: 11.0,
            wrap_width: None,
            color,
        }
    }

    #[test]
    fn the_cargo_areas_positions_are_draw_status_cargos() {
        assert_eq!(CARGO_FREE, "Free:");
        assert_eq!(CARGO_SPECIAL, "Special:");
        assert_eq!(CARGO_MULTIPLE, "Multiple");
        assert_eq!(CARGO_CREDITS, "credits");
        assert_eq!(
            [
                CARGO_ROW_BASELINE,
                CARGO_ROW_STEP,
                CARGO_LABEL_LEFT,
                CARGO_TONS_LEFT
            ],
            [12.0, 14.0, 3.0, 41.0]
        );
        assert_eq!(
            [CARGO_SIDE_LEFT, CARGO_FREE_VALUE_LEFT, CARGO_VALUE_LEFT],
            [77.0, 110.0, 87.0]
        );
        assert_eq!(
            [
                CARGO_SPECIAL_BASELINE,
                CARGO_SPECIAL_VALUE_BASELINE,
                CARGO_CREDITS_BASELINE,
                CARGO_CREDITS_VALUE_BASELINE
            ],
            [30.0, 46.0, 66.0, 82.0]
        );
    }

    #[test]
    fn the_credits_label_is_its_word_upper_cased_and_a_colon() {
        assert_eq!(credits_label(CARGO_CREDITS), "Credits:");
        assert_eq!(credits_label("cr\u{e9}dits"), "Cr\u{e9}dits:");
        assert_eq!(credits_label("Money"), "Money:");
        assert_eq!(credits_label(""), ":");
    }

    #[test]
    fn the_cargo_area_draws_each_line_where_draw_status_cargo_does() {
        let dim = Color::DIM;
        assert_eq!(
            cargo_drawn(&stock(), &full_cargo()),
            [
                cargo_text("Food:", 3.0, 12.0, dim),
                cargo_text("12", 41.0, 12.0, TEXT),
                cargo_text("Equ:", 3.0, 82.0, dim),
                cargo_text("3", 41.0, 82.0, TEXT),
                cargo_text("Free:", 77.0, 12.0, dim),
                cargo_text("25", 110.0, 12.0, TEXT),
                cargo_text("Special:", 77.0, 30.0, dim),
                cargo_text("Passengers", 87.0, 46.0, TEXT),
                cargo_text("Credits:", 77.0, 66.0, dim),
                cargo_text("1,234", 87.0, 82.0, TEXT),
            ]
        );
    }

    #[test]
    fn each_commodity_row_is_its_own_line_down_the_area() {
        let cargo = StatusCargo {
            rows: std::array::from_fn(|n| Some(row(&format!("C{n}:"), n as i64 + 1))),
            ..StatusCargo::default()
        };
        let drawn = cargo_drawn(&stock(), &cargo);
        for n in 0..6 {
            let baseline = 12.0 + 14.0 * n as f32;
            assert_eq!(
                drawn[2 * n],
                cargo_text(&format!("C{n}:"), 3.0, baseline, Color::DIM)
            );
            assert_eq!(
                drawn[2 * n + 1],
                cargo_text(&(n + 1).to_string(), 41.0, baseline, TEXT)
            );
        }
    }

    #[test]
    fn several_specials_are_multiple_and_none_draws_no_special_line() {
        let multiple = StatusCargo {
            special: Some(Special::Multiple),
            ..StatusCargo::default()
        };
        let drawn = cargo_drawn(&stock(), &multiple);
        assert!(
            drawn.contains(&cargo_text("Special:", 77.0, 30.0, Color::DIM)),
            "{drawn:?}"
        );
        assert!(drawn.contains(&cargo_text("Multiple", 87.0, 46.0, TEXT)));
        let none = cargo_drawn(&stock(), &StatusCargo::default());
        assert_eq!(
            none,
            [
                cargo_text("Free:", 77.0, 12.0, Color::DIM),
                cargo_text("0", 110.0, 12.0, TEXT),
                cargo_text("Credits:", 77.0, 66.0, Color::DIM),
                cargo_text("0", 87.0, 82.0, TEXT),
            ]
        );
    }

    #[test]
    fn the_credits_value_is_written_as_draw_comma_number_writes_it() {
        let rich = StatusCargo {
            cash: 12_345_678,
            ..StatusCargo::default()
        };
        assert_eq!(
            cargo_drawn(&stock(), &rich).last(),
            Some(&cargo_text("12.34M", 87.0, 82.0, TEXT))
        );
    }

    #[test]
    fn the_cargo_area_moves_with_the_bar_and_reads_its_layout() {
        let mut bar = odd_bar();
        bar.layout.cargo = rect(20.0, 400.0, 150.0, 520.0);
        let cargo = StatusCargo {
            rows: [Some(row("Food:", 12)), None, None, None, None, None],
            ..StatusCargo::default()
        };
        // The odd bar is 200 wide, so at (824, 0); the area's top-left
        // is (844, 400), and Geneva 12's top is 12 above its baseline.
        assert_eq!(
            cargo_drawn(&bar, &cargo)[..3],
            [
                geneva("Food:", 847.0, 400.0, FUEL_PARTIAL),
                geneva("12", 885.0, 400.0, SHIELD),
                geneva("Free:", 921.0, 400.0, FUEL_PARTIAL),
            ]
        );
    }

    #[test]
    fn an_empty_or_inverted_cargo_area_draws_nothing() {
        for cargo in [
            rect(8.0, 458.0, 8.0, 552.0),
            rect(8.0, 458.0, 184.0, 458.0),
            rect(184.0, 458.0, 8.0, 552.0),
            rect(8.0, 552.0, 184.0, 458.0),
        ] {
            let mut bar = stock();
            bar.layout.cargo = cargo;
            assert_eq!(cargo_drawn(&bar, &full_cargo()), [], "{cargo:?}");
        }
        let mut bar = stock();
        bar.layout.cargo = rect(8.0, 458.0, 9.0, 459.0);
        assert_eq!(
            cargo_drawn(&bar, &full_cargo()).len(),
            10,
            "one pixel is enough"
        );
    }

    #[test]
    fn with_no_cargo_to_list_the_area_draws_no_cargo_lines() {
        assert_eq!(
            texts(&drawn(&stock(), &state(reserves(30.0, 60.0, 300.0)))),
            [nav_text(NAV_OFF, NAV_OFF_BASELINE, Color::DIM)]
        );
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
        draw_unavailable(&mut list, "no ïntf 128", VIEW_SIZE.0);
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

    #[test]
    fn message_box_is_the_rect_init_sprites_sets() {
        assert_eq!(message_box(VIEW_SIZE), rect(25.0, 724.0, 780.0, 763.0));
        // 800 x 600: right 556, width 531, height 29470 / 531 = 55.
        assert_eq!(message_box((800.0, 600.0)), rect(25.0, 540.0, 556.0, 595.0));
        // 1365 x 768: right 1121, width 1096, height 29470 / 1096 = 26.
        assert_eq!(
            message_box((1365.0, 768.0)),
            rect(25.0, 737.0, 1121.0, 763.0)
        );
    }

    fn message_text(text: &str, origin: Point, wrap_width: Option<f32>) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: MESSAGE_FONT,
            origin,
            size: MESSAGE_TEXT_SIZE,
            wrap_width,
            color: Color::WHITE,
        }
    }

    #[test]
    fn a_message_is_drawn_in_the_box_at_12_pt_in_white() {
        let mut list = DrawList::new();
        draw_message(&mut list, "Mission failed.", VIEW_SIZE, Some(&MonoMetrics));
        let drawn: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            drawn,
            [&message_text("Mission failed.", at(25.0, 724.0), None)]
        );
        assert_eq!(MESSAGE_FONT, Font::Charcoal);
        assert_eq!(MESSAGE_TEXT_SIZE, 12.0);
    }

    #[test]
    fn a_long_message_wraps_inside_the_box() {
        // 60 words of 4: 299 characters at 7.2 each, 104 to a 755-wide line.
        let text = vec!["abcd"; 60].join(" ");
        let mut list = DrawList::new();
        draw_message(&mut list, &text, VIEW_SIZE, Some(&MonoMetrics));
        let all = crate::text::wrap(&MonoMetrics, Font::Charcoal, 12.0, &text, 755.0);
        assert_eq!(all.len(), 3, "a third line to clip");
        let drawn: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            drawn,
            [
                &message_text(&all[0], at(25.0, 724.0), None),
                &message_text(&all[1], at(25.0, 724.0 + 14.4), None),
                &DrawCommand::Clipped {
                    clip: message_box(VIEW_SIZE),
                    command: Box::new(message_text(&all[2], at(25.0, 724.0 + 28.8), None)),
                },
            ],
            "the third line's bottom, 767.2, passes 763: it shows its top"
        );
        for line in &all {
            assert!(MonoMetrics.width(Font::Charcoal, 12.0, line) <= 755.0);
        }
    }

    #[test]
    fn no_line_below_the_box_is_drawn() {
        let line = MonoMetrics.line_height(MESSAGE_FONT, MESSAGE_TEXT_SIZE);
        let area = Bounds::at(at(0.0, 0.0), 755.0, 1.5 * line);
        let text = vec!["abcd"; 100].join(" ");
        let mut list = DrawList::new();
        draw_message_in(&mut list, &text, area, Some(&MonoMetrics));
        let kinds: Vec<bool> = list
            .iter()
            .map(|c| matches!(c, DrawCommand::Clipped { .. }))
            .collect();
        assert_eq!(kinds, [false, true], "one whole line, one clipped, no more");
    }

    #[test]
    fn a_line_ending_on_the_box_bottom_is_drawn() {
        let line = MonoMetrics.line_height(MESSAGE_FONT, MESSAGE_TEXT_SIZE);
        let area = Bounds::at(at(0.0, 0.0), 755.0, 2.0 * line);
        let text = vec!["abcd"; 60].join(" ");
        let mut list = DrawList::new();
        draw_message_in(&mut list, &text, area, Some(&MonoMetrics));
        assert_eq!(list.iter().count(), 2, "the second line ends on the bottom");
    }

    #[test]
    fn without_metrics_the_message_wraps_at_the_box_width() {
        let mut list = DrawList::new();
        draw_message(&mut list, "Mission failed.", VIEW_SIZE, None);
        let drawn: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            drawn,
            [&message_text(
                "Mission failed.",
                at(25.0, 724.0),
                Some(755.0)
            )]
        );
    }
}
