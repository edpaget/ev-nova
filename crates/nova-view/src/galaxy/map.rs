//! The galaxy map screen: every system, hyperlink and nebula, which pans
//! and zooms, and an info panel about the selected system.
//!
//! The viewer ([`GalaxyMap::new`]) fills the screen, fitted to the galaxy,
//! and pans and zooms through its [`MapView`]. The in-flight maps are laid
//! out as the original's dialog, DITL 2000 ([`dialog`](super::dialog)),
//! and look where the original's `_mapZoom` and `_mapScroll` say
//! ([`place`](super::place)).
//!
//! The screen reads the galaxy once, through the [`GalaxyCatalog`] port,
//! when it is built; drawing and input never read anything. The course
//! map is shown a flight's galaxy ([`GalaxyMap::show_galaxy`]): the
//! session's galaxy overlay overrides which systems are clicked and which
//! nebulae are drawn, as each change to it is told; the geometry stays the
//! catalog's.
//!
//! Input (presses only, except the left button's release):
//!
//! - On the viewer the arrow keys pan the view by [`PAN_STEP`]; `=` or
//!   `+` zooms in and `-` zooms out. On an in-flight map the arrows scroll
//!   10 units and `-`, `=` and `+` step the zoom within its limits
//!   ([`MapDialog::key`]); each repeats while held.
//! - On the viewer, pressing the left button above its panel and moving
//!   the pointer drags the map by the pointer's moves. Releasing it after
//!   moving no more than [`CLICK_SLOP`] is a click instead: it selects
//!   the system under the pointer (clicking again cycles through systems
//!   that share a position, from a system a click selected) or, in empty
//!   space, clears the selection.
//! - On an in-flight map a left press in item 3 acts at once, as
//!   `_MapFilter` gives item 3 on mouse-down (@0x14552;
//!   [`click`](super::click)): a press whose hit box holds systems is held
//!   for the owner ([`GalaxyMap::take_press`]), which answers it
//!   ([`GalaxyMap::answer_press`], `nova_sim::Session::map_click`); one
//!   the owner accepts nothing of, or one that hits nothing, drags the map
//!   by the travel from the press times the zoom ([`MapPlace::drag`]). The
//!   release only ends a drag.
//! - A press outside the map area is a panel's or a button's.
//! - Return (a press, not its repeats), or a click on the "Enter system"
//!   button the panel shows while a system is selected, asks to enter the
//!   selected system. The map only records the request; its owner takes
//!   it with [`GalaxyMap::take_entry`]. A click on the button is a press
//!   and a release both on it.
//! - On the viewer, Tab is the app's router's and Escape the
//!   navigator's.
//! - Space (a press, not its repeats) on the course and hypergate maps
//!   beeps ([`UiSound::MapCentred`]) and centres the map on the current
//!   system, selected with no destination, as `_MapFilter` does on every
//!   map (@0x140e0-0x14169). The viewer has no current system: there it
//!   does nothing.
//! - F or S (a press, any case, not its repeats) opens Find, as the Find
//!   button does (@0x144b9-0x144d6; see "Find" below).
//! - C (any case, its repeats too) on an in-flight map asks the owner to
//!   clear the route ([`GalaxyMap::take_clear_route`]), as Clear Route
//!   does, whether the button is enabled or not (`_MapFilter` @0x14268).
//!
//! The map has two modes ([`MapMode`]). The viewer ([`GalaxyMap::new`]),
//! the developer path the app's Tab reaches, enters systems as above. The
//! course map ([`GalaxyMap::course`]), opened from flight, plots the
//! player's course instead: it has no "Enter system" button, and its
//! clicks are the owner's to answer, as above. On it and the hypergate
//! map, Tab or `\` (repeats too) asks the owner to cycle the selection
//! ([`GalaxyMap::take_cycle`], Shift held stepping the hypergate map's
//! back), and Return asks it to close the map
//! ([`GalaxyMap::take_close`]), as the original's `_MapFilter` does
//! (@0x13cdd-0x13f0f). As the course map opens from flight, its owner
//! selects the system the original's `_DoSystemMap` does
//! ([`GalaxyMap::preselect`], @0x124ac-0x124ff), which the panel then
//! shows.
//!
//! While a hypergate is entered from flight, the course map becomes the
//! hypergate map ([`MapMode::Hypergate`], [`GalaxyMap::offer_gates`]), as
//! the original's `_DoSystemMap` in hypergate mode: the owner's answer to
//! a click picks one of the systems the gate offers, or the current
//! system with no pick (@0x12f8f-0x13040, @0x136fc-0x13734). The owner
//! takes the pick with [`GalaxyMap::take_gate_choice`], which returns the
//! map to the course map. The pick is kept apart from the selection, as
//! the original's mode 4 and +0x6c are from `_selSystem`: a click and the
//! Tab cycle ([`GalaxyMap::select_gate`]) write both, while Space, Find
//! and the preselect move only the selection, so a destination picked
//! before them is still the one entered; Clear Route drops it.
//!
//! # Find, Done and the buttons
//!
//! Find ([`find`](super::find), `DLOG` 3005) opens over the map from F,
//! S or the Find button, laid out from the template
//! [`GalaxyMap::with_find`] gives, measured by the map's metrics (without
//! them it never opens). While it is open it takes every input: the
//! arrows don't pan, Space types a space, and a click selects nothing.
//! Find with an empty text, or Cancel, changes nothing and plays nothing.
//! A text is matched as `_DoSystemSearch` does
//! ([`map_search`](nova_sim::map_search)): the viewer searches every
//! system in the data itself, and an in-flight map leaves the search to
//! its owner ([`GalaxyMap::take_find`]), which answers with
//! [`GalaxyMap::show_search`]: a system found is selected and centred on
//! with [`UiSound::MapSystemFound`], and none found plays
//! [`UiSound::MapSystemNotFound`]. The map's beeps are read with
//! [`GalaxyMap::take_ui_sounds`], or as its screen sounds.
//!
//! The course and hypergate maps draw DITL 2000's six buttons
//! ([`buttons`](super::buttons)). A click on Done asks to close the map,
//! as Return does; on "-" or "+" steps the zoom, as `-` and `=` do; on
//! Find opens Find; on Clear Route, while it is enabled, asks the owner to
//! clear the route, as C does. Clear Route is enabled by
//! `showClearRouteButton` ([`GalaxyMap::set_clear_route`]): the owner sets
//! it as the map opens, a press that writes a hop sets it (by the
//! planner's reading a press writes it outright, off when it plots no
//! course), and the route's clear turns it off
//! ([`GalaxyMap::route_cleared`]). Borders, or B, flips the borders (see
//! "The borders" below). Each is silent, as `_TrackMapButtons` is.
//!
//! # The borders
//!
//! Every map shades government borders under its nebulae
//! ([`borders`](super::borders), worked out by
//! [`map_borders`](nova_sim::map_borders)). An in-flight map adds the
//! systems its owner shows ([`GalaxyMap::show_border_sources`],
//! `nova_sim::Session::map_border_sources`), and runs the original's
//! passes where `_DoSystemMap` and `_MapFilter` do, each only while the
//! borders are on:
//!
//! - the open clears the grid and runs the coarse pass (step 8)
//!   ([`GalaxyMap::opened`], @0x12440-0x12655);
//! - so do an arrow and a zoom step taken, by key or button (@0x14301,
//!   @0x143af, @0x1445f, @0x129e0, @0x12c13), Space (@0x1421a-0x1424c),
//!   the end of every drag, moved or not (@0x138de-0x1392f), and the
//!   borders turned on (@0x127b3-0x127fe); Find's centring runs one only
//!   by [`RuleKey::MapFindBorders`]' other reading;
//! - the map's ticks run the fine pass's null events, unless Find is open
//!   (its dialog has its own filter), at [`RuleKey::MapBorderPace`]'s
//!   pace ([`GalaxyMap::set_border_rules`]), and its last redraws item 3
//!   (@0x1479a).
//!
//! B (either case, its repeats too, @0x14251) and the Borders button flip
//! the setting, `_settings` +0x13, and redraw items 9 and 3
//! (@0x126f0-0x12803); the owner reads whether the close should save it
//! ([`GalaxyMap::borders_changed_since_open`]). After each input, tick or
//! owner's call that redrew item 3, the map takes the cells the grid holds
//! then, which the full update draws until the next redraw: so a redraw
//! mid-pass shows the grid as far as the pass has got, as the original's
//! update event does.
//!
//! The viewer, which has no session, counts every system with a
//! government ([`BorderSources::unfiltered`]), runs the coarse pass
//! whenever its view changes and the fine pass on its ticks, and B flips
//! them there too, with nothing saved.
//!
//! # What the map draws
//!
//! First the borders' cells, then the nebulae, as `_MapDialogUpdate`'s
//! nebula pass draws them
//! ([`nebulae`](super::nebulae)): each one's rect truncated in the port,
//! filled by the picture [`RuleKey::NebulaPicture`] picks
//! ([`GalaxyMap::set_nebula_pick`]), clipped to the map area and copied
//! with `addMax`, so overlaps take the brighter of each channel. The
//! course and hypergate maps draw the nebulae the session's galaxy shows,
//! active and explored; while a drag shows the partial update
//! ([`update`](super::update), [`GalaxyMap::update`]) they are plain
//! copies, and no border cells are drawn (@0xf838-0xf840). The partial frame stays
//! until a repair redraws item 3: the map makes most itself, and its
//! owner calls [`GalaxyMap::redraw`] for the Tab cycle's pick, and
//! [`GalaxyMap::route_cleared`] for Clear Route. The viewer draws every
//! nebula in the data, active or not, explored or not, with the engine's
//! pick, always with the full update.
//!
//! Over its nebulae, the map draws a [`MapPicture`]: in flight the
//! session's drawing ([`GalaxyMap::show_drawing`], `nova_sim`'s
//! [`galaxy_map`](nova_sim::galaxy_map) decides what `_DrawMap` draws),
//! and on the viewer every system and link in the data
//! ([`MapPicture::viewer`]). [`GalaxyMap::draw_marks`] draws it in
//! `_DrawMap`'s order (@0xe568-0xf5d0), and the floating map draws it
//! through the same method. Then the course and hypergate maps, and the
//! viewer, bracket the selection and name the systems again, as
//! `_MapDialogUpdate` does after `_DrawMap` (@0xfc9c-0x1001f).
//!
//! Points are whole pixels: each is the transformed point truncated toward
//! zero, as `_DrawMap` works trunc(centre + pos/zoom − scroll/zoom)
//! (@0xe5e9-0xe662; [`Transform::pixel`]). The zoom tests take the
//! original's zoom, the inverse of the scale ([`Transform::zoom`]):
//! against 0.5 (0xdd128) only the largest scale, 64/27, is at or below
//! it, and against 1.1 (0xdd1b0) the scales from 1 up are.
//!
//! 1. **The course** in [`BRIGHT_GREEN`] at pen 2 (@0xe661-0xe7a2).
//! 2. **The current system's links** in [`LESS_DARK_GREY`], or
//!    [`REAL_DARK_GREY`] on the hypergate map, at pen 1 (@0xe7a8-0xe986).
//! 3. **Every other link** in the same greys, the mode-3 slot's in
//!    [`DIM_GREEN`] (@0xe9da-0xebd2).
//! 4. **The mode-3 link** in [`DIM_GREEN`] (@0xebd8-0xed31).
//! 5. **Each system**: its point's rect inset by −[`SYSTEM_INSET_ZOOMED`]
//!    at pen 2 when the zoom ≤ 0.5, otherwise −[`SYSTEM_INSET`] at pen 1
//!    (@0xeeec-0xef68), an oval painted black (@0xef70-0xef85) and framed
//!    in its colour code's colour ([`code_color`], @0xef8a-0xefad); then
//!    its red arrow, `cicn` [`RED_ARROW`], in the 16 × 16 rect up and
//!    left of that rect (@0xefbe-0xf00b), and its green one, `cicn`
//!    [`GREEN_ARROW`], up and right (@0xf010-0xf058), so a system's
//!    arrows are under later systems' ovals.
//! 6. **The current system's dot**: its rect inset −3 when the zoom ≤
//!    0.5, otherwise −2, an oval painted [`CYAN`] (`ForeColor` 273), at
//!    pen 1 (@0xf07d-0xf190).
//! 7. **The hypergate arrows**: a shaft from each system offered to the
//!    current one, then two strokes from its tip back
//!    ([`arrow_head`]), in white
//!    (`ForeColor` 30) at pen 3 for the system selected, otherwise
//!    [`CYAN`] at pen 2 (@0xf195-0xf499).
//! 8. **The names**, white, in Geneva ([`NAME_FONT`], `_smallTextFont` 3)
//!    at [`NAME_SIZE`] (`_smallTextSize` 9), the baseline at trunc(x +
//!    7.0), trunc(y + 4.0) (0xdd1c0, 0xdd1b8): every one at a zoom ≤ 1.1,
//!    only the selection's above (@0xf49a-0xf5c0).
//!
//! After `_DrawMap`, the brackets: the selection's rect inset −8 when the
//! zoom ≤ 0.5, otherwise −6, and eight ticks in [`BRIGHT_GREEN`] at pen 1,
//! each a `LineTo` 3 units long starting a pixel in from a corner, so 4
//! pixels from (left + 1, top) to (left + 4, top) and likewise at each
//! corner along both edges (@0xfd30-0xff03). Then the names again, at the
//! same size, as `_DrawMap` set it last (@0xff14-0x1001f).
//!
//! **Pens and rasters.** QuickDraw hangs a pen below and to the right of
//! the path, and rasterises its own `LineTo`, `PaintOval` and `FrameOval`
//! in the Toolbox, not in the original's binary. A horizontal or vertical
//! line is drawn exactly, as the pixels its pen covers; any other line is
//! a line its pen wide along the path offset by half the pen, its ends
//! not squared off; and an oval is [`oval`](crate::oval)'s approximation,
//! drawn one run a row.
//!
//! # The panel
//!
//! The course and hypergate maps draw the session's info panels
//! ([`GalaxyMap::show_panel`], `nova_sim`'s
//! [`map_panel`](nova_sim::map_panel) decides them, the
//! [`panel`](super::panel) module lays them out), measured by the
//! flight's text metrics ([`GalaxyMap::with_metrics`]), at DITL 2000's
//! items 2 and 6 from the dialog's top-left corner. They draw none of the
//! viewer's lines: the original has no help, zoom, stack, problems or
//! course line.
//!
//! The viewer keeps its own panel, unfiltered: every stellar of the
//! system selected, landable or not, inhabited or not, with no title, no
//! explored level and nothing unexplored, and its lines in the right
//! column.
//!
//! [`RuleKey::NebulaPicture`]: nova_sim::RuleKey::NebulaPicture
//! [`RuleKey::MapBorderPace`]: nova_sim::RuleKey::MapBorderPace
//! [`RuleKey::MapFindBorders`]: nova_sim::RuleKey::MapFindBorders

use std::rc::Rc;
use std::time::Duration;

use nova_sim::Vec2;
use nova_sim::galaxy_map::{MapDrawing, MapLink, arrow_head};
use nova_sim::iff::SystemColorCode;
use nova_sim::map_borders::{BorderPasses, BorderSources, PaintedCell};
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::{self, SearchOutcome};
use nova_sim::story::OverlaidGalaxy;

use super::borders::MapBorders;
use super::buttons::MapButton;
use super::catalog::{GalaxyCatalog, SystemEntry, SystemId};
use super::click::{PendingPress, PressAnswer, hits};
use super::clip::clip_opaque;
use super::dialog::{DIALOG_ORIGIN, MapDialog, in_map_item, map_area};
use super::find::{FindDialog, FindOutcome};
use super::model::GalaxyModel;
use super::nebulae::{NebulaFrame, PicturePick, draw_nebulae};
use super::panel::draw_panel;
use super::place::MapPlace;
use super::update::MapUpdate;
use super::view::{Bounds, MAP_CENTER, MAP_HEIGHT, MAP_WIDTH, MapView, PAN_STEP, Transform};
use crate::draw::fill_rect;
use crate::flight::radar::IRect;
use crate::oval::{fill_pixels, oval_fill_pixels, oval_frame_pixels};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::{
    Color, DrawList, Font, ImageKey, Input, Key, MouseButton, Point, Screen, ScreenAction,
};

/// How far the pointer may travel between pressing and releasing the left
/// button for the press to count as a click rather than a drag.
pub const CLICK_SLOP: f32 = 4.0;

/// `_brightGreen`, (0, 0xffff, 0) (`_InitColors` @0x1beb8): the course
/// and the brackets. Each colour takes its 16-bit channels' high bytes, as
/// the radar's fixed colours do.
pub const BRIGHT_GREEN: Color = Color::rgba(0, 255, 0, 255);
/// `_dimGreen`, (0, 0x9900, 0) (@0x1be8c): the link to the system
/// selected in nav mode 3.
pub const DIM_GREEN: Color = Color::rgba(0, 0x99, 0, 255);
/// `_rgbDarkGrey`, 0x4000 (@0x1bee4): an unexplored system's frame.
pub const DARK_GREY: Color = Color::rgba(0x40, 0x40, 0x40, 255);
/// `_rgbRealDarkGrey`, 0x0fa0 (@0x1befa): the hypergate map's links.
pub const REAL_DARK_GREY: Color = Color::rgba(0x0f, 0x0f, 0x0f, 255);
/// `_rgbLessDarkGrey`, 0x4e20 (@0x1bf26): the links.
pub const LESS_DARK_GREY: Color = Color::rgba(0x4e, 0x4e, 0x4e, 255);
/// `_rgbLightGrey`, 0xc000 (@0x1bf3c): an uninhabited system's frame.
pub const LIGHT_GREY: Color = Color::rgba(0xc0, 0xc0, 0xc0, 255);
/// `ForeColor` 273: the current system's dot and the hypergate arrows.
pub const CYAN: Color = Color::rgba(0, 255, 255, 255);
/// `_ColorCodeSystem`'s blue, (0, 0, 0xffff) (@0xd8c5-0xd8d3).
pub const CLEARED: Color = Color::rgba(0, 0, 255, 255);
/// `_ColorCodeSystem`'s orange, (0xffff, 0x6666, 0) (@0xd8e2-0xd8f0).
pub const DENIED: Color = Color::rgba(255, 0x66, 0, 255);
/// `_ColorCodeSystem`'s red, (0xffff, 0, 0) (@0xd8ff-0xd90d).
pub const HOSTILE: Color = Color::rgba(255, 0, 0, 255);

/// The red mission arrow, `cicn` 15000 (`_LoadSprites` @0x21b0a).
pub const RED_ARROW: i16 = 15000;
/// The green mission arrow, `cicn` 15001 (@0x21b1e).
pub const GREEN_ARROW: i16 = 15001;
/// A mission arrow's rect's side.
pub const ARROW_SIZE: i32 = 16;

/// How far a system's point rect is inset (outward) for its oval, and
/// its pen, at a zoom above 0.5.
pub const SYSTEM_INSET: i32 = 4;
/// The same at a zoom of 0.5 or below, with a pen of 2.
pub const SYSTEM_INSET_ZOOMED: i32 = 6;
/// How far the current system's point rect is inset for its dot, at a
/// zoom above 0.5, and at 0.5 or below.
pub const CURRENT_INSET: i32 = 2;
pub const CURRENT_INSET_ZOOMED: i32 = 3;
/// How far the selection's point rect is inset for its brackets, at a
/// zoom above 0.5, and at 0.5 or below.
pub const BRACKET_INSET: i32 = 6;
pub const BRACKET_INSET_ZOOMED: i32 = 8;
/// How long a bracket tick's `LineTo` runs, in units: 4 pixels at a pen
/// of 1.
pub const TICK: i32 = 3;

/// The names' font: `_smallTextFont`, Geneva (@0x1d3d1, 3 @0x467a5).
pub const NAME_FONT: Font = Font::Geneva;
/// The names' size: `_smallTextSize`, 9 (@0x1d3e6, @0x467b4).
pub const NAME_SIZE: f32 = 9.0;
/// Where a name's baseline starts from its system's point (0xdd1c0,
/// 0xdd1b8).
pub const NAME_OFFSET: (f32, f32) = (7.0, 4.0);

/// The zoom at or below which the ovals, the current dot and the brackets
/// are the larger ones (0.5, 0xdd128).
pub const ZOOMED: f64 = 0.5;
/// The zoom at or below which every name is drawn, and above which only
/// the selection's (1.1, 0xdd1b0).
pub const NAMED: f64 = 1.1;

/// The bottom of the screen, where the info panel ends.
const SCREEN_HEIGHT: f32 = 768.0;
/// The info panel's background.
pub const PANEL: Color = Color::rgba(16, 16, 28, 255);
/// The line between the map and the panel: its colour and width.
const SEPARATOR: Color = Color::rgba(128, 128, 128, 255);
const SEPARATOR_WIDTH: f32 = 1.0;

/// The panel's left column: the selected system's name, then its stellars.
const LEFT: f32 = 16.0;
const LEFT_WRAP: f32 = 496.0;
const TITLE_TOP: f32 = 616.0;
const TITLE_SIZE: f32 = 24.0;
const STELLARS_TOP: f32 = 652.0;
const STELLAR_SPACING: f32 = 20.0;
const STELLAR_SIZE: f32 = 16.0;
/// The panel's right column: help, zoom, the stack line and problems.
const RIGHT: f32 = 528.0;
const RIGHT_WRAP: f32 = 480.0;
const RIGHT_SIZE: f32 = 14.0;
const HELP_TOP: f32 = 616.0;
const ZOOM_TOP: f32 = 636.0;
const STACK_TOP: f32 = 656.0;
const PROBLEMS_TOP: f32 = 676.0;
/// The help line.
pub const HELP: &str = "Arrows or drag: pan   +/-: zoom   Click: select   Return: enter   S: find";
/// The "Enter system" button, in the panel's right column under the
/// problems line; shown only while a system is selected.
pub const ENTER_BUTTON: Bounds = Bounds {
    min: Point::new(528.0, 712.0),
    max: Point::new(708.0, 744.0),
};
/// The button's colour.
pub const BUTTON: Color = Color::rgba(48, 48, 80, 255);
/// The button's label, where it goes and its size.
pub const ENTER_LABEL: &str = "Enter system (Return)";
const ENTER_LABEL_AT: Point = Point::new(540.0, 720.0);
const ENTER_LABEL_SIZE: f32 = 14.0;

// The two columns stay on screen and apart.
const _: () = assert!(LEFT + LEFT_WRAP <= RIGHT);
const _: () = assert!(RIGHT + RIGHT_WRAP <= MAP_WIDTH);
const _: () = assert!(TITLE_TOP > MAP_HEIGHT && HELP_TOP > MAP_HEIGHT);

/// A left-button press that has not been released yet.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    /// Where the pointer was last seen.
    last: Point,
    /// How far the pointer has travelled since the press.
    travelled: f32,
}

/// A system the map draws an oval for, with what it is framed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PicturedSystem {
    /// The system.
    pub id: SystemId,
    /// Its oval's frame colour.
    pub frame: Color,
    /// Whether its red mission arrow is drawn.
    pub red_arrow: bool,
    /// Whether its green mission arrow is drawn.
    pub green_arrow: bool,
}

/// What [`GalaxyMap::draw_marks`] draws, in `_DrawMap`'s order: a
/// session's [`MapDrawing`] with each frame's colour ([`MapPicture::of`]),
/// or the viewer's ([`MapPicture::viewer`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPicture {
    /// Whether it is the hypergate map's: its links in
    /// [`REAL_DARK_GREY`].
    pub hypergate: bool,
    /// The course's segments, raw.
    pub course: Vec<(SystemId, SystemId)>,
    /// The current system's links.
    pub current_links: Vec<(SystemId, SystemId)>,
    /// Every other link.
    pub links: Vec<MapLink>,
    /// The link to the system selected in mode 3.
    pub mode3_link: Option<(SystemId, SystemId)>,
    /// The systems drawn, by ascending ID.
    pub systems: Vec<PicturedSystem>,
    /// The current system, whose dot is drawn; none on the viewer.
    pub current: Option<SystemId>,
    /// The systems the hypergate arrows point to.
    pub gate_arrows: Vec<SystemId>,
    /// The systems that may be named, by ascending ID.
    pub named: Vec<SystemId>,
}

/// The colour `_ColorCodeSystem` gives `code` (@0xd672-0xd941).
#[must_use]
pub fn code_color(code: SystemColorCode) -> Color {
    match code {
        SystemColorCode::Unexplored => DARK_GREY,
        SystemColorCode::Uninhabited => LIGHT_GREY,
        SystemColorCode::Dominated => BRIGHT_GREEN,
        SystemColorCode::Cleared => CLEARED,
        SystemColorCode::Denied => DENIED,
        SystemColorCode::Hostile => HOSTILE,
    }
}

impl MapPicture {
    /// The session's `drawing`, each system framed in its code's colour
    /// ([`code_color`]).
    #[must_use]
    pub fn of(drawing: &MapDrawing) -> Self {
        Self {
            hypergate: drawing.hypergate,
            course: drawing.course.clone(),
            current_links: drawing.current_links.clone(),
            links: drawing.links.clone(),
            mode3_link: drawing.mode3_link,
            systems: drawing
                .systems
                .iter()
                .map(|system| PicturedSystem {
                    id: system.id,
                    frame: code_color(system.code),
                    red_arrow: system.red_arrow,
                    green_arrow: system.green_arrow,
                })
                .collect(),
            current: Some(drawing.current),
            gate_arrows: drawing.gate_arrows.clone(),
            named: drawing.named.clone(),
        }
    }

    /// The viewer's picture of `model`, which has no session and so no
    /// filters: every system the catalog has, hidden by its `Visibility`
    /// or not, framed in its government's colour, as there is no
    /// `_ColorCodeSystem` without a session; every link in the data once,
    /// in [`LESS_DARK_GREY`]; every system named, by the zoom's rule; and
    /// no course, arrows or current dot.
    #[must_use]
    pub fn viewer(model: &GalaxyModel) -> Self {
        let systems = model.catalog_systems();
        Self {
            hypergate: false,
            course: Vec::new(),
            current_links: Vec::new(),
            links: model
                .links()
                .iter()
                .map(|&(from, to)| MapLink {
                    from,
                    to,
                    selected: false,
                })
                .collect(),
            mode3_link: None,
            systems: systems
                .iter()
                .map(|system| PicturedSystem {
                    id: system.entry.id,
                    frame: system.color,
                    red_arrow: false,
                    green_arrow: false,
                })
                .collect(),
            current: None,
            gate_arrows: Vec::new(),
            named: systems.iter().map(|system| system.entry.id).collect(),
        }
    }
}

/// What [`GalaxyMap::draw_marks`] draws over the nebulae: a picture, and
/// the system selected (`_selSystem`), whose name alone is drawn above a
/// zoom of 1.1 and whose hypergate arrow is drawn white.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapMarks<'a> {
    /// What is drawn.
    pub picture: &'a MapPicture,
    /// The system selected.
    pub selected: Option<SystemId>,
}

/// A Tab or `\` pressed on an in-flight map ([`GalaxyMap::take_cycle`]):
/// the original's `_MapFilter` cycles the selection (@0x13d0c-0x13f0a).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapCycle {
    /// Whether Shift was held, which steps the hypergate map's cycle back
    /// (the event's modifiers 0x0200, @0x13ee4).
    pub back: bool,
}

/// What the map is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapMode {
    /// Browsing the galaxy and entering systems: the developer path.
    Viewer,
    /// Choosing where the player's ship goes, from flight.
    Course,
    /// Choosing which of a hypergate's linked systems the ship comes out
    /// in, from flight: the course map while a hypergate is entered
    /// ([`GalaxyMap::offer_gates`]).
    Hypergate,
}

/// The galaxy map.
#[derive(Clone, Debug)]
pub struct GalaxyMap {
    mode: MapMode,
    model: GalaxyModel,
    look: Look,
    selected: Option<SystemId>,
    /// Whether a click made the selection, so that a click again at its
    /// place moves on through the systems there; not when the map
    /// preselected it ([`GalaxyMap::preselect`]).
    clicked: bool,
    press: Option<Press>,
    /// Whether the left button went down on the "Enter system" button and
    /// has not been released yet.
    armed: bool,
    /// A request to enter a system, not yet taken.
    entry: Option<SystemId>,
    /// An in-flight map's press that hit systems, not yet taken by the
    /// owner.
    pending: Option<PendingPress>,
    /// Where a press taken by the owner and not yet answered was made:
    /// the drag starts there if the owner accepts no hit.
    awaiting: Option<Point>,
    /// The system the player is in, as last shown.
    current: Option<SystemId>,
    /// The course from it, as last shown.
    route: Vec<SystemId>,
    /// The info panels an in-flight map draws, once shown.
    panel: Option<MapPanel>,
    /// What the panels' text is measured by, if anything.
    metrics: Option<MapMetrics>,
    /// The systems a hypergate offers, on the hypergate map.
    offered: Vec<SystemId>,
    /// What the map draws: the viewer's from the start, an in-flight
    /// map's once shown ([`GalaxyMap::show_drawing`]).
    picture: Option<MapPicture>,
    /// Whether Shift is held.
    shift: bool,
    /// A Tab, `\`, Return, C or Clear Route on an in-flight map, not yet
    /// taken.
    request: Option<Request>,
    /// The hypergate map's pick, the original's mode 4 and +0x6c: what a
    /// click on an offered system or the Tab cycle picked, which Space
    /// and Find, writing `_selSystem` alone, leave alone.
    gate_pick: Option<SystemId>,
    /// The Find dialog, while it is open.
    find: Option<FindDialog>,
    /// What Find is laid out from: `None` for its fallback.
    find_template: Option<DialogTemplate>,
    /// How Find's buttons and the map's are labelled.
    style: ButtonStyle,
    /// A text Find confirmed on an in-flight map, not yet taken.
    query: Option<String>,
    /// The beeps the map made, not yet taken.
    sounds: Vec<UiSound>,
    /// Which picture fills each nebula's rect on an in-flight map
    /// ([`GalaxyMap::set_nebula_pick`]).
    nebula_pick: PicturePick,
    /// The government borders ([`borders`](super::borders)).
    borders: MapBorders,
}

/// How a map is laid out: the viewer full-screen with its own pan and
/// zoom, the in-flight maps as DITL 2000 with the original's. The mode
/// moves between the course and hypergate maps only, so a map's look
/// never changes.
#[derive(Clone, Debug)]
enum Look {
    /// The viewer's: fitted, panned and zoomed through [`MapView`].
    Viewer(MapView),
    /// An in-flight map's: the dialog, its zoom and scroll.
    Dialog(MapDialog),
}

/// What a key on an in-flight map asks its owner for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    /// Tab or `\`: cycle the selection.
    Cycle(MapCycle),
    /// Return: close the map.
    Close,
    /// C or Clear Route: clear the route.
    ClearRoute,
}

impl GalaxyMap {
    /// The viewer map of `catalog`'s galaxy, fitted to the map area, with
    /// nothing selected.
    pub fn new(catalog: &impl GalaxyCatalog) -> Self {
        Self::with_mode(catalog, MapMode::Viewer)
    }

    /// The course map of `catalog`'s galaxy, laid out as DITL 2000 and
    /// looking at (0, 0) at the first zoom of a run, with nothing
    /// selected and no course shown.
    pub fn course(catalog: &impl GalaxyCatalog) -> Self {
        Self::with_mode(catalog, MapMode::Course)
    }

    fn with_mode(catalog: &impl GalaxyCatalog, mode: MapMode) -> Self {
        let model = GalaxyModel::new(catalog.galaxy());
        let viewer = mode == MapMode::Viewer;
        let look = if viewer {
            Look::Viewer(MapView::fit(model.bounds()))
        } else {
            Look::Dialog(MapDialog::new((0, 0)))
        };
        let picture = viewer.then(|| MapPicture::viewer(&model));
        let mut borders = MapBorders::new();
        if viewer {
            let systems = model.catalog_systems().iter().map(|system| {
                (
                    system.entry.id,
                    (system.entry.x, system.entry.y),
                    system.entry.govt,
                )
            });
            borders.show_sources(BorderSources::unfiltered(systems, model.govts().clone()));
        }
        let mut map = Self {
            mode,
            model,
            look,
            selected: None,
            clicked: false,
            press: None,
            armed: false,
            entry: None,
            pending: None,
            awaiting: None,
            current: None,
            route: Vec::new(),
            panel: None,
            metrics: None,
            offered: Vec::new(),
            picture,
            shift: false,
            request: None,
            gate_pick: None,
            find: None,
            find_template: None,
            style: ButtonStyle::STOCK,
            query: None,
            sounds: Vec::new(),
            nebula_pick: PicturePick::LargestThatFits,
            borders,
        };
        map.settle_borders();
        map
    }

    /// What the map is for.
    #[must_use]
    pub fn mode(&self) -> MapMode {
        self.mode
    }

    /// The press on an in-flight map's item 3 that hit systems since this
    /// was last taken, for the owner to answer with
    /// [`GalaxyMap::answer_press`] (`nova_sim::Session::map_click`); taking
    /// it clears it. Until it is answered nothing drags. The viewer never
    /// holds one: it clicks on the release.
    pub fn take_press(&mut self) -> Option<PendingPress> {
        let press = self.pending.take()?;
        self.awaiting = Some(press.at);
        Some(press)
    }

    /// The owner's answer to the press taken ([`GalaxyMap::take_press`]):
    /// `None`, no hit accepted, starts the drag from the press, as
    /// `_DoSystemMap` drags for a press that selects nothing (@0x137e9);
    /// an answer sets the selection, the hypergate map's pick and
    /// `showClearRouteButton` as it says, and nothing drags.
    pub fn answer_press(&mut self, answer: Option<PressAnswer>) {
        let Some(at) = self.awaiting.take() else {
            return;
        };
        let Some(answer) = answer else {
            self.start_drag(at);
            return;
        };
        // An accepted press redraws items 2, 3 and 6 (@0x131af-0x13259).
        self.invalidate();
        if let Some(selected) = answer.selected {
            self.selected = Some(selected);
            self.clicked = false;
        }
        if let Some(pick) = answer.gate_pick {
            self.gate_pick = pick;
        }
        if let Some(enabled) = answer.clear_route {
            self.set_clear_route(enabled);
        }
        self.settle_borders();
    }

    /// Whether C, or a click on Clear Route while it is enabled, asked an
    /// in-flight map to clear the route since this was last taken (item 8,
    /// `_MapFilter` @0x14268); taking it clears it. The owner clears it
    /// (`nova_sim::Session::clear_route`) and answers with
    /// [`GalaxyMap::route_cleared`].
    pub fn take_clear_route(&mut self) -> bool {
        let clear = self.request == Some(Request::ClearRoute);
        if clear {
            self.request = None;
        }
        clear
    }

    /// The route cleared (@0x12c69-0x12dbf): Clear Route disabled
    /// (`showClearRouteButton` cleared), the hypergate map's pick
    /// dropped, as the original's +0x6c = −1, and item 3 redrawn
    /// (@0x12cf0, `_MapDialogUpdate` @0x12d9f).
    pub fn route_cleared(&mut self) {
        self.set_clear_route(false);
        self.gate_pick = None;
        self.invalidate();
    }

    /// Redraws an in-flight map's item 3 with the full update
    /// ([`MapDialog::redraw`]), for a change its owner settles: the Tab
    /// cycle's pick, which invalidates items 2, 3 and 6 (the hypergate
    /// cycle's @0x13e6f, the course cycle's @0x14032). See
    /// [`update`](super::update) for every repair.
    pub fn redraw(&mut self) {
        self.invalidate();
        self.settle_borders();
    }

    /// Redraws an in-flight map's item 3 ([`MapDialog::redraw`]) for the
    /// handler running: the borders take their cells once it is done, as
    /// the original's update event draws after the handler returns.
    fn invalidate(&mut self) {
        if let Look::Dialog(dialog) = &mut self.look {
            dialog.redraw();
        }
    }

    /// The update an in-flight map's item 3 shows; `None` on the viewer,
    /// which always draws the full one.
    #[must_use]
    pub fn update(&self) -> Option<MapUpdate> {
        match &self.look {
            Look::Viewer(_) => None,
            Look::Dialog(dialog) => Some(dialog.update()),
        }
    }

    /// Sets which picture fills each nebula's rect on an in-flight map,
    /// as `source` reads [`RuleKey::NebulaPicture`]. The viewer has no
    /// rules: it keeps the engine's.
    ///
    /// [`RuleKey::NebulaPicture`]: nova_sim::RuleKey::NebulaPicture
    pub fn set_nebula_pick(&mut self, source: nova_sim::RuleSource) {
        if self.mode != MapMode::Viewer {
            self.nebula_pick = PicturePick::of(source);
        }
    }

    /// Sets `showClearRouteButton`, as the map opens with it
    /// (@0x12396-0x123c3). A press that writes a hop sets it, and the
    /// route's clear turns it off; by the engine's reading nothing else
    /// touches it. By the planner's (`nova_sim::RuleKey::MapPlotting`)
    /// a press writes it outright, off when it plots no course.
    pub fn set_clear_route(&mut self, enabled: bool) {
        if let Look::Dialog(dialog) = &mut self.look {
            dialog.set_clear_route(enabled);
        }
    }

    /// Switches to the hypergate map, offering the systems `offered`, with
    /// nothing selected. An arrow points from the current system to each.
    pub fn offer_gates(&mut self, offered: Vec<SystemId>) {
        self.mode = MapMode::Hypergate;
        self.offered = offered;
        self.selected = None;
        self.gate_pick = None;
    }

    /// The systems the hypergate offers, on the hypergate map; none on
    /// the others.
    #[must_use]
    pub fn offered(&self) -> &[SystemId] {
        &self.offered
    }

    /// The system picked on the hypergate map, if one is, and the map goes
    /// back to the course map with nothing selected or picked; `None`, and
    /// nothing changes, when no gates are offered. The pick is what a
    /// click on an offered system or the Tab cycle
    /// ([`GalaxyMap::select_gate`]) picked last, the original's mode 4 and
    /// +0x6c: not the selection as such, which Space, Find and the
    /// preselect move without picking, as they write `_selSystem` alone.
    /// Only an offered system is taken.
    pub fn take_gate_choice(&mut self) -> Option<SystemId> {
        if self.mode != MapMode::Hypergate {
            return None;
        }
        self.mode = MapMode::Course;
        let offered = std::mem::take(&mut self.offered);
        self.selected = None;
        self.gate_pick.take().filter(|pick| offered.contains(pick))
    }

    /// Selects and picks `pick` on the hypergate map, the pick its Tab /
    /// `\` cycle made ([`MapCycle`]), as it is: a system the map doesn't
    /// draw, such as a hidden one, or none, stays selected, so the next
    /// press steps on from it, as `_MapFilter` writes `_selSystem` and
    /// +0x6c (@0x13dfd). [`GalaxyMap::take_gate_choice`] still takes only
    /// an offered system.
    pub fn select_gate(&mut self, pick: Option<SystemId>) {
        self.selected = pick;
        self.gate_pick = pick;
        self.clicked = false;
    }

    /// Whether Tab or `\` was pressed on the course or hypergate map
    /// since this was last taken, and whether Shift was held then; taking
    /// it clears it. Its repeats count, as `_MapFilter` takes the
    /// auto-key event too (@0x13cbf-0x13cd7). The viewer never asks.
    pub fn take_cycle(&mut self) -> Option<MapCycle> {
        match self.request {
            Some(Request::Cycle(cycle)) => {
                self.request = None;
                Some(cycle)
            }
            _ => None,
        }
    }

    /// Whether Return (or Enter) was pressed on the course or hypergate
    /// map since this was last taken, asking to close it, as `_MapFilter`
    /// gives item 1 on a key-down of either (@0x13cdd-0x13d07); taking it
    /// clears it. Its repeats don't count. The viewer never asks: its
    /// Return enters the system selected.
    pub fn take_close(&mut self) -> bool {
        let close = self.request == Some(Request::Close);
        if close {
            self.request = None;
        }
        close
    }

    /// Shows the player in `current` with `route` ahead: the systems still
    /// to jump to, in order.
    pub fn show_course(&mut self, current: SystemId, route: &[SystemId]) {
        self.current = Some(current);
        self.route = route.to_vec();
    }

    /// Shows `drawing`, the session's ([`Session::map_drawing`]), as what
    /// the map draws, each system framed in its code's colour
    /// ([`MapPicture::of`]), in place of any shown before.
    ///
    /// [`Session::map_drawing`]: nova_sim::Session::map_drawing
    pub fn show_drawing(&mut self, drawing: &MapDrawing) {
        self.picture = Some(MapPicture::of(drawing));
    }

    /// What the map draws, once there is anything: the viewer's from the
    /// start, an in-flight map's once shown.
    #[must_use]
    pub fn picture(&self) -> Option<&MapPicture> {
        self.picture.as_ref()
    }

    /// The system the map draws the mode-3 link to, if any.
    #[must_use]
    pub fn hyper_selection(&self) -> Option<SystemId> {
        self.picture
            .as_ref()
            .and_then(|picture| picture.mode3_link)
            .map(|(_, to)| to)
    }

    /// Selects `system`, as the map selects as it opens (`_DoSystemMap`
    /// @0x124ac-0x124ff), when it is on the map, and nothing otherwise. It
    /// picks no hypergate destination, and the viewer's click at its
    /// place is a first click, selecting the first system there.
    pub fn preselect(&mut self, system: Option<SystemId>) {
        self.selected = system.filter(|&id| self.model.system(id).is_some());
        self.clicked = false;
    }

    /// Takes the session's `galaxy` over the catalog's geometry
    /// ([`GalaxyModel::apply`]): its visible systems are the ones a click
    /// selects, and the nebulae it shows, active and explored, are drawn.
    /// A selected system it hides is let go of. What the map draws over
    /// the nebulae is its picture ([`GalaxyMap::show_drawing`]).
    pub fn show_galaxy(&mut self, galaxy: &OverlaidGalaxy) {
        self.model.apply(galaxy);
        self.selected = self.selected.filter(|&id| self.model.system(id).is_some());
    }

    /// Shows `panel`, the session's ([`Session::map_panel`]), as the info
    /// panels an in-flight map draws, in place of any shown before. The
    /// viewer never draws it.
    ///
    /// [`Session::map_panel`]: nova_sim::Session::map_panel
    pub fn show_panel(&mut self, panel: MapPanel) {
        self.panel = Some(panel);
    }

    /// The info panels shown, once shown.
    #[must_use]
    pub fn panel(&self) -> Option<&MapPanel> {
        self.panel.as_ref()
    }

    /// The map with its panels' text measured by `metrics`. Find needs
    /// them: without them it never opens.
    #[must_use]
    pub fn with_metrics(self, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            metrics: Some(MapMetrics(metrics)),
            ..self
        }
    }

    /// The map with Find laid out from `template` (stock `DLOG` 3005), or
    /// from its fallback for `None` or a template it can't be laid out
    /// from, and its buttons and the map's labelled in `style`.
    #[must_use]
    pub fn with_find(self, template: Option<DialogTemplate>, style: ButtonStyle) -> Self {
        Self {
            find_template: template,
            style,
            ..self
        }
    }

    /// Whether Find is open: while it is, it takes every input.
    #[must_use]
    pub fn finding(&self) -> bool {
        self.find.is_some()
    }

    /// The Find dialog, while it is open.
    #[must_use]
    pub fn find_dialog(&self) -> Option<&FindDialog> {
        self.find.as_ref()
    }

    /// Closes Find, if it is open, with nothing searched, as the map
    /// closes: a map closed some other way reopens without it.
    pub fn close_find(&mut self) {
        self.find = None;
    }

    /// The text Find confirmed on an in-flight map since this was last
    /// taken, not empty, for the owner to search
    /// (`nova_sim::Session::find_map_system`) and answer with
    /// [`GalaxyMap::show_search`]; taking it clears it. The viewer
    /// searches by itself, and never asks.
    pub fn take_find(&mut self) -> Option<String> {
        self.query.take()
    }

    /// Shows what a search found, as `_DoSystemSearch` does
    /// (@0x1205c-0x120f3): a system found is selected, not as a
    /// destination or a hypergate pick, and the map centres on it
    /// (`_selSystem` and `_mapScroll`), with [`UiSound::MapSystemFound`];
    /// none found changes nothing, with [`UiSound::MapSystemNotFound`].
    pub fn show_search(&mut self, outcome: SearchOutcome) {
        match outcome {
            SearchOutcome::Found(id) => {
                self.select_and_centre(id);
                if let Look::Dialog(dialog) = &mut self.look {
                    self.borders.found(dialog);
                }
                self.sounds.push(UiSound::MapSystemFound);
            }
            SearchOutcome::NotFound => self.sounds.push(UiSound::MapSystemNotFound),
        }
        self.settle_borders();
    }

    /// The beeps the map made since they were last taken, in order:
    /// Space's and Find's. Taking them empties the list.
    pub fn take_ui_sounds(&mut self) -> Vec<UiSound> {
        std::mem::take(&mut self.sounds)
    }

    /// The system the player is in, as last shown.
    #[must_use]
    pub fn current(&self) -> Option<SystemId> {
        self.current
    }

    /// The course from it, as last shown.
    #[must_use]
    pub fn route(&self) -> &[SystemId] {
        &self.route
    }

    /// The selected system, if any.
    #[must_use]
    pub fn selected(&self) -> Option<SystemId> {
        self.selected
    }

    /// What the viewer's map area shows; `None` on an in-flight map.
    #[must_use]
    pub fn view(&self) -> Option<&MapView> {
        match &self.look {
            Look::Viewer(view) => Some(view),
            Look::Dialog(_) => None,
        }
    }

    /// Where an in-flight map looks; `None` on the viewer.
    #[must_use]
    pub fn place(&self) -> Option<&MapPlace> {
        match &self.look {
            Look::Viewer(_) => None,
            Look::Dialog(dialog) => Some(dialog.place()),
        }
    }

    /// Where the map draws each galaxy point: the viewer's view, or an
    /// in-flight map's scroll at item 3's centre.
    #[must_use]
    pub fn transform(&self) -> Transform {
        match &self.look {
            Look::Viewer(view) => view.transform(),
            Look::Dialog(dialog) => dialog.transform(),
        }
    }

    /// Centres the map on system `id`, if it is on the map: the viewer's
    /// view stopping at the galaxy's edge, an in-flight map's scroll set
    /// to its position, unclamped. The selection is left alone.
    pub fn centre_on_system(&mut self, id: SystemId) {
        let Some(at) = self.model.position(id) else {
            return;
        };
        match &mut self.look {
            Look::Viewer(view) => view.centre_on(at),
            Look::Dialog(dialog) => dialog.place_mut().centre_on(at),
        }
    }

    /// Puts an in-flight map's scroll back on the current system shown,
    /// with no selection and no beep, as each arrival, each floating-map
    /// draw and each pilot entered set `_mapScroll` (see
    /// [`place`](super::place)). The viewer has no current system.
    pub fn recentre(&mut self) {
        if let (Look::Dialog(_), Some(current)) = (&self.look, self.current) {
            self.centre_on_system(current);
        }
    }

    /// An in-flight map opening: its zoom's guard ([`MapPlace::opened`]),
    /// and the full update, as the open clears `_partialMapDialogUpdate`
    /// (@0x12458) and draws (@0x125a5).
    pub fn opened(&mut self) {
        if let Look::Dialog(dialog) = &mut self.look {
            dialog.place_mut().opened();
            dialog.redraw();
            self.borders.opened(dialog);
        }
        self.settle_borders();
    }

    /// An in-flight map's zoom, which a run keeps across its flights;
    /// `None` on the viewer.
    #[must_use]
    pub fn zoom(&self) -> Option<f64> {
        self.place().map(MapPlace::zoom)
    }

    /// The map with an in-flight map's zoom set to `zoom`, as the run
    /// left it; the viewer keeps its own.
    #[must_use]
    pub fn with_zoom(mut self, zoom: f64) -> Self {
        if let Look::Dialog(dialog) = &mut self.look {
            *dialog.place_mut() = dialog.place().with_zoom(zoom);
        }
        self
    }

    /// Sets whether the borders are shown, as the Borders button reads it:
    /// "Hide Borders" while they are, "Show Borders" while they aren't.
    /// An in-flight map starts with them shown.
    pub fn show_borders_setting(&mut self, shown: bool) {
        if let Look::Dialog(dialog) = &mut self.look {
            dialog.set_borders_shown(shown);
        }
    }

    /// Shows `sources`, the session's ([`Session::map_border_sources`]),
    /// as what an in-flight map's borders add from now on. The owner shows
    /// them before the map opens and again with each drawing.
    ///
    /// [`Session::map_border_sources`]: nova_sim::Session::map_border_sources
    pub fn show_border_sources(&mut self, sources: BorderSources) {
        self.borders.show_sources(sources);
        self.settle_borders();
    }

    /// Sets an in-flight map's readings of [`RuleKey::MapBorderPace`] and
    /// [`RuleKey::MapFindBorders`]. The viewer has no rules: it keeps the
    /// engine's.
    ///
    /// [`RuleKey::MapBorderPace`]: nova_sim::RuleKey::MapBorderPace
    /// [`RuleKey::MapFindBorders`]: nova_sim::RuleKey::MapFindBorders
    pub fn set_border_rules(&mut self, pace: nova_sim::RuleSource, find: nova_sim::RuleSource) {
        if self.mode != MapMode::Viewer {
            self.borders.set_rules(pace, find);
        }
    }

    /// Whether the borders are shown: `_settings` +0x13 on an in-flight
    /// map, the viewer's own toggle on the viewer.
    #[must_use]
    pub fn borders_shown(&self) -> bool {
        match &self.look {
            Look::Viewer(_) => self.borders.viewer_on(),
            Look::Dialog(dialog) => dialog.borders_shown(),
        }
    }

    /// Whether an in-flight map's borders setting differs from the one it
    /// last opened with, as its close saves it (@0x13c3a); never on the
    /// viewer, which saves nothing.
    #[must_use]
    pub fn borders_changed_since_open(&self) -> bool {
        match &self.look {
            Look::Viewer(_) => false,
            Look::Dialog(dialog) => self.borders.changed_since_open(dialog),
        }
    }

    /// The borders' passes and grid.
    #[must_use]
    pub fn border_passes(&self) -> &BorderPasses {
        self.borders.passes()
    }

    /// The border cells a full update draws, as last taken.
    #[must_use]
    pub fn border_cells(&self) -> &[PaintedCell] {
        self.borders.shown()
    }

    /// Whether Clear Route is enabled: `showClearRouteButton`
    /// ([`GalaxyMap::set_clear_route`]); never on the viewer, which has
    /// no buttons.
    #[must_use]
    pub fn clear_route_enabled(&self) -> bool {
        match &self.look {
            Look::Viewer(_) => false,
            Look::Dialog(dialog) => dialog.clear_route(),
        }
    }

    /// The galaxy as the map lays it out.
    #[must_use]
    pub fn model(&self) -> &GalaxyModel {
        &self.model
    }

    /// The system the map has been asked to enter, by Return or the
    /// "Enter system" button, if any; taking it clears the request.
    pub fn take_entry(&mut self) -> Option<SystemId> {
        self.entry.take()
    }

    /// Everything that could not be read or laid out.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        self.model.problems()
    }
}

impl GalaxyMap {
    /// Handles a key press or repeat: the viewer pans and zooms through
    /// its view, an in-flight map as the dialog does ([`MapDialog::key`]).
    fn key(&mut self, key: Key) {
        if let Look::Dialog(dialog) = &mut self.look {
            let before = *dialog.place();
            if dialog.key(key) {
                // An arrow or a zoom step taken (@0x14301, @0x143af,
                // @0x1445f, @0x129e0, @0x12c13).
                if *dialog.place() != before {
                    self.borders.recompute(dialog);
                }
                return;
            }
        }
        match (&mut self.look, key) {
            // B, either case, its repeats too (@0x14251): item 9.
            (Look::Dialog(dialog), Key::Char('b' | 'B')) => self.borders.toggle(dialog),
            (Look::Viewer(_), Key::Char('b' | 'B')) => self.borders.toggle_viewer(),
            (Look::Viewer(view), Key::Left) => view.pan(-PAN_STEP, 0.0),
            (Look::Viewer(view), Key::Right) => view.pan(PAN_STEP, 0.0),
            (Look::Viewer(view), Key::Up) => view.pan(0.0, -PAN_STEP),
            (Look::Viewer(view), Key::Down) => view.pan(0.0, PAN_STEP),
            (Look::Viewer(view), Key::Char('=' | '+')) => view.zoom_in(),
            (Look::Viewer(view), Key::Char('-')) => view.zoom_out(),
            (_, Key::Tab | Key::Char('\\')) if self.mode != MapMode::Viewer => {
                self.request = Some(Request::Cycle(MapCycle { back: self.shift }));
            }
            // C, either case, its repeats too, whatever Clear Route's
            // flag (`_MapFilter` @0x14268).
            (Look::Dialog(_), Key::Char('c' | 'C')) => self.request = Some(Request::ClearRoute),
            _ => {}
        }
    }

    /// Selects `id`, as no click did, and centres the map on it.
    fn select_and_centre(&mut self, id: SystemId) {
        self.selected = Some(id);
        self.clicked = false;
        self.centre_on_system(id);
    }

    /// Space on an in-flight map (`_MapFilter` @0x140e0-0x14169): the
    /// beep first, then the map centred on the current system, selected
    /// with no destination or hypergate pick, and item 3 redrawn
    /// (@0x14215). The viewer has no current system: there Space does
    /// nothing.
    fn centre_on_current(&mut self) {
        if self.mode == MapMode::Viewer {
            return;
        }
        self.sounds.push(UiSound::MapCentred);
        if let Some(current) = self.current {
            self.select_and_centre(current);
        }
        self.invalidate();
        // @0x1421a-0x1424c.
        if let Look::Dialog(dialog) = &mut self.look {
            self.borders.recompute(dialog);
        }
    }

    /// Opens Find, abandoning any press on the map, when the map has text
    /// metrics to lay it out with; the key that opened it types nothing
    /// into it.
    fn open_find(&mut self) {
        let Some(MapMetrics(metrics)) = &self.metrics else {
            return;
        };
        let mut dialog = self
            .find_template
            .as_ref()
            .and_then(|template| FindDialog::new(template, self.style, Rc::clone(metrics)).ok())
            .unwrap_or_else(|| FindDialog::fallback(self.style, Rc::clone(metrics)));
        dialog.flush_typed_key();
        self.find = Some(dialog);
        self.press = None;
        self.armed = false;
        if let Look::Dialog(dialog) = &mut self.look
            && dialog.cancel_pointer()
        {
            self.borders.recompute(dialog);
        }
    }

    /// Find's input while it is open: it closes on Find or Cancel, which
    /// redraws the whole dialog (@0x11e26-0x11e5d), and a text found is
    /// searched.
    fn find_input(&mut self, input: &Input) {
        let Some(dialog) = &mut self.find else {
            return;
        };
        dialog.input(input);
        let Some(outcome) = dialog.take_outcome() else {
            return;
        };
        self.find = None;
        self.invalidate();
        if let FindOutcome::Search(text) = outcome {
            self.searched(text);
        }
    }

    /// A text Find confirmed (`_DoSystemSearch` @0x11e26-0x12056): an
    /// empty one does nothing and plays nothing; on the viewer the map
    /// searches every system in the data, active or not, known or not,
    /// explored or not, as it has no session; an in-flight map asks its
    /// owner ([`GalaxyMap::take_find`]).
    fn searched(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.mode != MapMode::Viewer {
            self.query = Some(text);
            return;
        }
        let candidates = self
            .model
            .catalog_systems()
            .iter()
            .map(|system| (system.entry.id, system.entry.name.as_str()));
        let outcome = map_search::search(&text, candidates);
        self.show_search(outcome);
    }

    /// Asks to enter the selected system, if there is one and the map
    /// enters systems.
    fn enter(&mut self) {
        if self.mode == MapMode::Viewer
            && let Some(id) = self.selected
        {
            self.entry = Some(id);
        }
    }

    /// Whether the "Enter system" button is shown.
    fn has_enter_button(&self) -> bool {
        self.mode == MapMode::Viewer && self.selected.is_some()
    }

    /// Ends the press, if there is one, with the left button released at
    /// `at`. On the viewer it is a click if the pointer stayed within
    /// [`CLICK_SLOP`] and is still over the map, otherwise the end of a
    /// drag. On an in-flight map it only ends a drag: its click was the
    /// press's ([`GalaxyMap::take_press`]).
    fn release(&mut self, at: Point) {
        let Some(press) = self.press.take() else {
            return;
        };
        if let Look::Dialog(dialog) = &mut self.look {
            // Every drag's release, moved or not (@0x138de-0x1392f).
            if dialog.release() {
                self.borders.recompute(dialog);
            }
            return;
        }
        if press.travelled <= CLICK_SLOP && self.in_map(at) {
            let from = self.selected.filter(|_| self.clicked);
            self.selected = self.model.click(at, &self.transform(), from);
            self.clicked = true;
        }
    }

    /// A left press in the map area at `at`: the viewer's press, clicked
    /// or dragged on its release; on an in-flight map, the systems its
    /// hit box holds ([`hits`]) held for the owner
    /// ([`GalaxyMap::take_press`]), or with none the drag at once.
    fn press_map(&mut self, at: Point) {
        if let Look::Dialog(_) = &self.look {
            let hits = hits(&self.model, at, &self.transform());
            self.awaiting = None;
            if !hits.is_empty() {
                self.press = None;
                if let Look::Dialog(dialog) = &mut self.look
                    && dialog.release()
                {
                    self.borders.recompute(dialog);
                }
                self.pending = Some(PendingPress {
                    at,
                    hits,
                    shift: self.shift,
                });
                return;
            }
            self.pending = None;
        }
        self.start_drag(at);
    }

    /// Starts a press at `at` that drags the map as the pointer moves:
    /// replacing any whose release was lost.
    fn start_drag(&mut self, at: Point) {
        self.press = Some(Press {
            last: at,
            travelled: 0.0,
        });
        if let Look::Dialog(dialog) = &mut self.look {
            dialog.press(at);
        }
    }

    /// Drags the map with the pointer, if the left button is down: the
    /// viewer by the pointer's moves, an in-flight map from where the
    /// press started ([`MapDialog::pointer_moved`]).
    fn pointer_moved(&mut self, to: Point) {
        let Some(press) = &mut self.press else {
            return;
        };
        let (dx, dy) = (to.x - press.last.x, to.y - press.last.y);
        press.travelled += dx.hypot(dy);
        press.last = to;
        match &mut self.look {
            Look::Viewer(view) => view.pan(-dx, -dy),
            Look::Dialog(dialog) => dialog.pointer_moved(to),
        }
    }

    /// The viewer's map: every nebula in the data, active or not,
    /// explored or not, as `_MapDialogUpdate` draws one (see
    /// [`nebulae`](super::nebulae)) in its own frame with the engine's
    /// pick, then its picture and the selection, through its view.
    fn draw_viewer_map(&self, view: &MapView, list: &mut DrawList) {
        let transform = view.transform();
        let centre = view.center();
        #[allow(clippy::cast_possible_truncation)] // The map area's pixels.
        let frame = NebulaFrame {
            origin: Point::new(0.0, 0.0),
            centre: (MAP_CENTER.x as i32, MAP_CENTER.y as i32),
            zoom: transform.zoom(),
            scroll: (f64::from(centre.x), f64::from(centre.y)),
            area: IRect::new(0, 0, MAP_WIDTH as i32, MAP_HEIGHT as i32),
        };
        self.borders.draw_viewer(list, self.model.govt_colors());
        let nebulae = self.model.catalog_nebulae();
        draw_nebulae(
            list,
            nebulae,
            &frame,
            MapUpdate::Full,
            PicturePick::LargestThatFits,
        );
        self.draw_marks_and_selection(list, &transform);
    }

    /// Draws the picture and the selection through `transform`: what
    /// `_DrawMap` draws, then the brackets and the names again.
    fn draw_marks_and_selection(&self, list: &mut DrawList, transform: &Transform) {
        if let Some(picture) = &self.picture {
            let marks = MapMarks {
                picture,
                selected: self.selected,
            };
            self.draw_marks(list, transform, &marks);
            self.draw_brackets(list, transform);
            self.draw_names(list, transform, &marks);
        } else {
            self.draw_brackets(list, transform);
        }
    }

    /// An in-flight map, as `_MapDialogUpdate` draws it (see
    /// [`dialog`](super::dialog)):
    ///
    /// 1. the frame and its buttons, item 3 black;
    /// 2. the borders' cells ([`MapBorders::draw`]), before the nebulae,
    ///    and none while [`MapUpdate::Partial`];
    /// 3. the nebulae the session's galaxy shows, active and explored
    ///    ([`draw_nebulae`]), blended as [`MapDialog::update`] says;
    /// 4. what `_DrawMap` draws, the brackets and the names, clipped to
    ///    item 3 ([`clip_opaque`]);
    /// 5. the panels.
    ///
    /// In a partial update the frame outside item 3 is drawn as in a full
    /// one: the window keeps those pixels in the original.
    fn draw_dialog(&self, dialog: &MapDialog, list: &mut DrawList) {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        dialog.draw_frame(list, &self.style, metrics);
        self.borders.draw(dialog, list, self.model.govt_colors());
        let nebulae = self.model.nebulae();
        let frame = dialog.nebula_frame();
        draw_nebulae(list, nebulae, &frame, dialog.update(), self.nebula_pick);
        let mut map = DrawList::new();
        self.draw_marks_and_selection(&mut map, &dialog.transform());
        clip_opaque(&map, map_area(), list);
        if let Some(panel) = &self.panel {
            draw_panel(list, panel, DIALOG_ORIGIN, metrics);
        }
    }

    /// Draws what `_DrawMap` draws over the nebulae, through `transform`,
    /// with `marks` (see the module docs' "What the map draws"): the
    /// course, the links, the mode-3 link, each system's oval and arrows,
    /// the current system's dot, the hypergate arrows and the names. The
    /// course, hypergate and viewer maps draw their own picture and
    /// selection through it, then their brackets and names again; the
    /// floating map (the flight's) its own centre, zoom, picture and
    /// selection, and nothing after.
    pub fn draw_marks(&self, list: &mut DrawList, transform: &Transform, marks: &MapMarks<'_>) {
        let picture = marks.picture;
        let zoomed = transform.zoom() <= ZOOMED;
        let point = |id: SystemId| {
            self.model
                .position(id)
                .map(|at| transform.world_to_screen(at))
        };
        let pixel = |id: SystemId| self.model.position(id).map(|at| transform.pixel(at));
        let segment = |list: &mut DrawList, (from, to): (SystemId, SystemId), pen, color| {
            if let (Some(from), Some(to)) = (pixel(from), pixel(to)) {
                pen_line(list, from, to, pen, color);
            }
        };
        let grey = if picture.hypergate {
            REAL_DARK_GREY
        } else {
            LESS_DARK_GREY
        };
        for &hop in &picture.course {
            segment(list, hop, 2, BRIGHT_GREEN);
        }
        for &link in &picture.current_links {
            segment(list, link, 1, grey);
        }
        for link in &picture.links {
            let color = if link.selected { DIM_GREEN } else { grey };
            segment(list, (link.from, link.to), 1, color);
        }
        if let Some(link) = picture.mode3_link {
            segment(list, link, 1, DIM_GREEN);
        }
        let (inset, pen) = if zoomed {
            (SYSTEM_INSET_ZOOMED, 2)
        } else {
            (SYSTEM_INSET, 1)
        };
        // Every system's oval is the same shape: rasterised once, about
        // (0, 0), and moved to each.
        let fill = oval_fill_pixels(around((0, 0), inset));
        let frame = oval_frame_pixels(around((0, 0), inset), pen);
        let moved = |pixels: &[(i32, i32)], (x, y): (i32, i32)| -> Vec<(i32, i32)> {
            pixels.iter().map(|&(px, py)| (px + x, py + y)).collect()
        };
        for system in &picture.systems {
            let Some(at) = pixel(system.id) else {
                continue;
            };
            let rect = around(at, inset);
            fill_pixels(list, &moved(&fill, at), Color::BLACK);
            fill_pixels(list, &moved(&frame, at), system.frame);
            if system.red_arrow {
                icon(
                    list,
                    RED_ARROW,
                    (rect.left - ARROW_SIZE, rect.top - ARROW_SIZE),
                );
            }
            if system.green_arrow {
                icon(list, GREEN_ARROW, (rect.right, rect.top - ARROW_SIZE));
            }
        }
        if let Some(at) = picture.current.and_then(pixel) {
            let inset = if zoomed {
                CURRENT_INSET_ZOOMED
            } else {
                CURRENT_INSET
            };
            fill_pixels(list, &oval_fill_pixels(around(at, inset)), CYAN);
        }
        if let Some(tail) = picture.current.and_then(point) {
            for &target in &picture.gate_arrows {
                let Some(tip) = point(target) else {
                    continue;
                };
                let (color, pen) = if marks.selected == Some(target) {
                    (Color::WHITE, 3)
                } else {
                    (CYAN, 2)
                };
                pen_line(list, truncated(tip), truncated(tail), pen, color);
                let head = arrow_head(Vec2::new(tip.x, tip.y), Vec2::new(tail.x, tail.y));
                for end in head {
                    let end = truncated(Point::new(end.x, end.y));
                    pen_line(list, truncated(tip), end, pen, color);
                }
            }
        }
        self.draw_names(list, transform, marks);
    }

    /// Draws the names `marks` may name, through `transform`: every one at
    /// a zoom ≤ 1.1, only the selection's above, each white in
    /// [`NAME_FONT`] at [`NAME_SIZE`], its baseline at
    /// trunc(x + 7.0), trunc(y + 4.0) (`_DrawMap` @0xf49a-0xf5c0, again
    /// `_MapDialogUpdate` @0xff14-0x1001f). The baseline is a size above
    /// the run's top, as the spaceport's text sets it.
    fn draw_names(&self, list: &mut DrawList, transform: &Transform, marks: &MapMarks<'_>) {
        let every = transform.zoom() <= NAMED;
        for &id in &marks.picture.named {
            if !every && marks.selected != Some(id) {
                continue;
            }
            let Some(system) = self.model.catalog_system(id) else {
                continue;
            };
            let at = transform.world_to_screen(system.position());
            let (x, y) = (at.x + NAME_OFFSET.0, at.y + NAME_OFFSET.1);
            let origin = Point::new(x.trunc(), y.trunc() - NAME_SIZE);
            list.text_in(
                NAME_FONT,
                system.entry.name.clone(),
                origin,
                NAME_SIZE,
                None,
                Color::WHITE,
            );
        }
    }

    /// Draws the brackets round the selection, through `transform`, as
    /// `_MapDialogUpdate` does after `_DrawMap` (@0xfc9c-0xff03; see the
    /// module docs): nothing with none selected.
    fn draw_brackets(&self, list: &mut DrawList, transform: &Transform) {
        let Some(at) = self
            .selected
            .and_then(|id| self.model.position(id))
            .map(|at| transform.pixel(at))
        else {
            return;
        };
        let inset = if transform.zoom() <= ZOOMED {
            BRACKET_INSET_ZOOMED
        } else {
            BRACKET_INSET
        };
        let IRect {
            left,
            top,
            right,
            bottom,
        } = around(at, inset);
        let ticks = [
            ((left + 1, top), (left + 1 + TICK, top)),
            ((left + 1, bottom), (left + 1 + TICK, bottom)),
            ((right - 1, top), (right - 1 - TICK, top)),
            ((right - 1, bottom), (right - 1 - TICK, bottom)),
            ((left, top + 1), (left, top + 1 + TICK)),
            ((left, bottom - 1), (left, bottom - 1 - TICK)),
            ((right, top + 1), (right, top + 1 + TICK)),
            ((right, bottom - 1), (right, bottom - 1 - TICK)),
        ];
        for (from, to) in ticks {
            pen_line(list, from, to, 1, BRIGHT_GREEN);
        }
    }

    /// The viewer's panel: the selected system and its stellars, the help,
    /// zoom, stack and problems lines, and the "Enter system" button.
    fn draw_viewer_panel(&self, view: &MapView, list: &mut DrawList) {
        let panel = Bounds {
            min: Point::new(0.0, MAP_HEIGHT),
            max: Point::new(MAP_WIDTH, SCREEN_HEIGHT),
        };
        fill_rect(list, panel, PANEL);
        list.line(
            Point::new(0.0, MAP_HEIGHT),
            Point::new(MAP_WIDTH, MAP_HEIGHT),
            SEPARATOR_WIDTH,
            SEPARATOR,
        );
        let selected = self.selected.and_then(|id| self.model.system(id));
        if self.has_enter_button() {
            fill_rect(list, ENTER_BUTTON, BUTTON);
        }
        match selected {
            Some(system) => draw_system(&system.entry, list),
            None => {
                list.text(
                    "Click a system to see its stellars",
                    Point::new(LEFT, TITLE_TOP),
                    STELLAR_SIZE,
                    Some(LEFT_WRAP),
                    Color::DIM,
                );
            }
        }
        let right = |list: &mut DrawList, text: String, top: f32, color: Color| {
            list.text(
                text,
                Point::new(RIGHT, top),
                RIGHT_SIZE,
                Some(RIGHT_WRAP),
                color,
            );
        };
        right(list, HELP.to_owned(), HELP_TOP, Color::DIM);
        let percent = (view.scale() * 100.0).round();
        right(list, format!("Zoom {percent}%"), ZOOM_TOP, Color::DIM);
        if let Some(system) = selected {
            let stack = self.model.stack(system.entry.id);
            if stack.len() > 1 {
                let index = stack.iter().position(|&id| id == system.entry.id);
                let ids: Vec<String> = stack.iter().map(|id| id.0.to_string()).collect();
                let again = if self.clicked {
                    ": click again for the next"
                } else {
                    ""
                };
                let line = format!(
                    "{} of {} systems here (sÿst {}){again}",
                    index.map_or(0, |index| index + 1),
                    stack.len(),
                    ids.join(", ")
                );
                right(list, line, STACK_TOP, Color::WHITE);
            }
        }
        let problems = self.model.problems();
        if let Some(first) = problems.first() {
            let line = format!(
                "{} problem(s) reading the map data: {first}",
                problems.len()
            );
            right(list, line, PROBLEMS_TOP, Color::ERROR);
        }
        if self.has_enter_button() {
            list.text(
                ENTER_LABEL,
                ENTER_LABEL_AT,
                ENTER_LABEL_SIZE,
                None,
                Color::WHITE,
            );
        }
    }
}

/// The viewer's panel: the system's name and ID, then every one of its
/// stellars.
fn draw_system(system: &SystemEntry, list: &mut DrawList) {
    list.text(
        format!("{} (sÿst {})", system.name, system.id.0),
        Point::new(LEFT, TITLE_TOP),
        TITLE_SIZE,
        Some(LEFT_WRAP),
        Color::WHITE,
    );
    let names: Vec<&str> = if system.stellars.is_empty() {
        vec!["No stellars"]
    } else {
        system.stellars.iter().map(|s| s.name.as_str()).collect()
    };
    for (index, name) in names.into_iter().enumerate() {
        let top = STELLARS_TOP + index as f32 * STELLAR_SPACING;
        list.text(
            name,
            Point::new(LEFT, top),
            STELLAR_SIZE,
            Some(LEFT_WRAP),
            Color::WHITE,
        );
    }
}

/// The point rect at `at` inset by −`inset`: `inset` out on every side.
fn around((x, y): (i32, i32), inset: i32) -> IRect {
    IRect::new(x - inset, y - inset, x + inset, y + inset)
}

/// `at` truncated toward zero, as the original's `cvttss2si` takes a
/// float point to `MoveTo` / `LineTo`.
#[allow(clippy::cast_possible_truncation)] // Screen pixels.
fn truncated(at: Point) -> (i32, i32) {
    (at.x.trunc() as i32, at.y.trunc() as i32)
}

/// A QuickDraw `MoveTo(from)`, `LineTo(to)` with a `pen` × `pen` pen in
/// `color` (see the module docs' "Pens and rasters"): a horizontal or
/// vertical line is the rect its pen covers, both ends included; any other
/// is a line `pen` wide from `from` to `to`, each offset by half the pen.
#[allow(clippy::cast_precision_loss)] // Screen pixels.
fn pen_line(list: &mut DrawList, from: (i32, i32), to: (i32, i32), pen: i32, color: Color) {
    if from.0 == to.0 || from.1 == to.1 {
        let (left, right) = (from.0.min(to.0), from.0.max(to.0) + pen);
        let (top, bottom) = (from.1.min(to.1), from.1.max(to.1) + pen);
        let area = Bounds {
            min: Point::new(left as f32, top as f32),
            max: Point::new(right as f32, bottom as f32),
        };
        fill_rect(list, area, color);
        return;
    }
    let half = pen as f32 / 2.0;
    let shifted = |(x, y): (i32, i32)| Point::new(x as f32 + half, y as f32 + half);
    list.line(shifted(from), shifted(to), pen as f32, color);
}

/// `cicn` `id` in the [`ARROW_SIZE`]-square rect whose top-left corner is
/// `top_left`, as `PlotCIcon` plots it (`_DrawMap` @0xefbe-0xf058): a
/// sprite centred on the rect, so the floating map can fade it. A stock
/// map arrow is 16 × 16; one of another size would be drawn at its own
/// size where `PlotCIcon` scales it to the rect.
#[allow(clippy::cast_precision_loss)] // Screen pixels.
fn icon(list: &mut DrawList, id: i16, top_left: (i32, i32)) {
    let half = ARROW_SIZE as f32 / 2.0;
    let center = Point::new(top_left.0 as f32 + half, top_left.1 as f32 + half);
    list.sprite(ImageKey::icon(id), center, Color::WHITE);
}

impl GalaxyMap {
    /// Whether `at` is in the map area: on the viewer above its panel, on
    /// an in-flight map in item 3 ([`in_map_item`]).
    fn in_map(&self, at: Point) -> bool {
        match self.look {
            Look::Viewer(_) => at.y < MAP_HEIGHT,
            Look::Dialog(_) => in_map_item(at),
        }
    }

    /// The in-flight maps' six buttons' input (`_TrackMapButtons`): Done
    /// asks to close the map, as Return does; "-" and "+" step the zoom,
    /// each only while its flag allows, a step taken redrawing item 3;
    /// Find opens Find; Clear Route, while enabled, asks to clear the
    /// route, as C does. Borders flips the borders, as B does
    /// ([`MapBorders::toggle`]). A zoom step taken runs the borders'
    /// coarse pass.
    fn button_input(&mut self, input: &Input) {
        let Look::Dialog(dialog) = &mut self.look else {
            return;
        };
        let state = dialog.state();
        match dialog.buttons_mut().input(input, &state) {
            Some(MapButton::Done) => self.request = Some(Request::Close),
            // A step taken redraws item 3 (@0x128d9, @0x12b0c).
            Some(MapButton::ZoomOut) => {
                if dialog.place_mut().zoom_out() {
                    dialog.redraw();
                    self.borders.recompute(dialog);
                }
            }
            Some(MapButton::ZoomIn) => {
                if dialog.place_mut().zoom_in() {
                    dialog.redraw();
                    self.borders.recompute(dialog);
                }
            }
            Some(MapButton::Find) => self.open_find(),
            Some(MapButton::ClearRoute) => self.request = Some(Request::ClearRoute),
            Some(MapButton::Borders) => self.borders.toggle(dialog),
            None => {}
        }
    }
}

/// Text metrics, shared.
#[derive(Clone)]
struct MapMetrics(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MapMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MapMetrics")
    }
}

impl GalaxyMap {
    /// [`Screen::input`]'s work, before the borders settle.
    fn take_input(&mut self, input: &Input) {
        if let Input::Key {
            key: Key::Shift,
            pressed,
            ..
        } = *input
        {
            self.shift = pressed;
        }
        if self.finding() {
            self.find_input(input);
            return;
        }
        self.button_input(input);
        match *input {
            // Return enters once, however long it is held.
            Input::Key {
                key: Key::Enter,
                pressed: true,
                repeat,
            } => {
                if !repeat {
                    if self.mode == MapMode::Viewer {
                        self.enter();
                    } else {
                        self.request = Some(Request::Close);
                    }
                }
            }
            // Once, on key-down, as `_MapFilter` takes Space (@0x140e0).
            Input::Key {
                key: Key::Space,
                pressed: true,
                repeat,
            } => {
                if !repeat {
                    self.centre_on_current();
                }
            }
            // F and S, any case, on key-down: the Find button
            // (@0x144b9-0x144d6).
            Input::Key {
                key: Key::Char('f' | 's' | 'F' | 'S'),
                pressed: true,
                repeat: false,
            } => self.open_find(),
            // Repeats too: holding a key keeps panning or zooming.
            Input::Key {
                key, pressed: true, ..
            } => self.key(key),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            } => {
                if !pressed {
                    // A click on the button: pressed and released on it.
                    if std::mem::take(&mut self.armed) && ENTER_BUTTON.contains(at) {
                        self.enter();
                    }
                    self.release(at);
                } else if self.has_enter_button() && ENTER_BUTTON.contains(at) {
                    self.armed = true;
                } else {
                    // Replaces any button press whose release was lost.
                    self.armed = false;
                    if self.in_map(at) {
                        self.press_map(at);
                    } else {
                        // `_MapDialogUpdate` before `_TrackMapButtons`
                        // (@0x14558).
                        self.invalidate();
                    }
                }
            }
            Input::PointerMoved(to) => self.pointer_moved(to),
            _ => {}
        }
    }

    /// The borders settled after anything that can redraw item 3: an
    /// in-flight map takes its cells when item 3 was redrawn
    /// ([`MapBorders::settle`]), the viewer runs its coarse pass when its
    /// view changed ([`MapBorders::settle_viewer`]).
    fn settle_borders(&mut self) {
        match &mut self.look {
            Look::Dialog(dialog) => self.borders.settle(dialog),
            Look::Viewer(view) => self.borders.settle_viewer(MapBorders::viewer_frame(view)),
        }
    }
}

impl Screen for GalaxyMap {
    /// Never quits: Escape is the navigator's. While Find is open, it
    /// takes every input.
    fn input(&mut self, input: &Input) -> ScreenAction {
        self.take_input(input);
        self.settle_borders();
        ScreenAction::None
    }

    /// The map's frame: the borders' null events while they are on and
    /// Find isn't open, as only `_MapFilter` runs them and Find's dialog
    /// has its own filter (see [`MapBorders::tick`]); then the cells taken
    /// if item 3 was redrawn.
    fn tick(&mut self, _dt: Duration) {
        if !self.finding() {
            match &mut self.look {
                Look::Dialog(dialog) => self.borders.tick(dialog),
                Look::Viewer(view) => self.borders.tick_viewer(MapBorders::viewer_frame(view)),
            }
        }
        self.settle_borders();
    }

    /// The map, its panel, then Find over them while it is open: the
    /// viewer full-screen, an in-flight map as its dialog.
    fn draw(&self, list: &mut DrawList) {
        match &self.look {
            Look::Viewer(view) => {
                self.draw_viewer_map(view, list);
                self.draw_viewer_panel(view, list);
            }
            Look::Dialog(dialog) => self.draw_dialog(dialog, list),
        }
        if let Some(dialog) = &self.find {
            dialog.draw(list);
        }
    }

    /// Forgets the left-button press, if any: the map stops dragging and the
    /// release, wherever it ends up, is not a click, on the map, on the
    /// "Enter system" button, on Done or Find, or in Find.
    fn cancel_pointer(&mut self) {
        self.press = None;
        self.pending = None;
        self.awaiting = None;
        self.armed = false;
        if let Look::Dialog(dialog) = &mut self.look
            && dialog.cancel_pointer()
        {
            self.borders.recompute(dialog);
        }
        if let Some(dialog) = &mut self.find {
            dialog.cancel_pointer();
        }
        self.settle_borders();
    }

    /// Space's and Find's beeps ([`GalaxyMap::take_ui_sounds`]).
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.take_ui_sounds().into_iter().map(Sound::Ui).collect()
    }

    /// Forgets Shift as held.
    fn release_keys(&mut self) {
        self.shift = false;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::Blend;
    use crate::galaxy::catalog::{Galaxy, GovtId, StellarEntry, StellarId};
    use crate::galaxy::model::{NEUTRAL, star_system};
    use crate::galaxy::nebulae::{choose, nebula_rect};
    use crate::galaxy::testkit::{BLUE, FakeCatalog, galaxy, system};
    use crate::galaxy::view::{MAP_CENTER, SCALES};
    use crate::text::fixture::MonoMetrics;
    use crate::{DrawCommand, Font, ImageKey, Key, MouseButton};
    use nova_sim::story::{GalaxyOverlay, OverlaidGalaxy};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// The viewer, its borders turned off with B, so that what it draws
    /// is its nebulae and `_DrawMap`'s alone: the borders' own tests are
    /// [`borders`](crate::galaxy::borders)'.
    fn map() -> GalaxyMap {
        let mut map = GalaxyMap::new(&FakeCatalog::new(galaxy()));
        map.input(&press(Key::Char('b')));
        assert!(!map.borders_shown());
        map
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    /// A key held down past the OS key-repeat delay.
    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn button(button: MouseButton, pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button,
            pressed,
            at,
        }
    }

    /// Presses and releases the left button at `at`.
    fn click(map: &mut GalaxyMap, at: Point) {
        assert_eq!(
            map.input(&button(MouseButton::Left, true, at)),
            ScreenAction::None
        );
        assert_eq!(
            map.input(&button(MouseButton::Left, false, at)),
            ScreenAction::None
        );
    }

    /// Clicks on system `id`'s dot. On an in-flight map, which opens at
    /// 0.5625 and shows little of the galaxy, the map is brought onto it
    /// first, and the owner answers the press by selecting `id`, as the
    /// session's click does for a system it accepts off the hypergate map.
    fn click_on(map: &mut GalaxyMap, id: i16) {
        answered_on(map, id, None);
    }

    /// Clicks on system `id`'s dot on the hypergate map, the owner
    /// answering by selecting it and picking it.
    fn gate_click_on(map: &mut GalaxyMap, id: i16) {
        answered_on(map, id, Some(SystemId(id)));
    }

    /// Clicks on system `id`'s dot on the hypergate map, the owner
    /// answering by selecting it and dropping the pick, as for the current
    /// system.
    fn gate_click_dropping(map: &mut GalaxyMap, id: i16) {
        answer_on(map, id, Some(None));
    }

    /// [`click_on`], the owner's answer picking `pick` on the hypergate
    /// map.
    fn answered_on(map: &mut GalaxyMap, id: i16, pick: Option<SystemId>) {
        answer_on(map, id, pick.map(Some));
    }

    /// [`click_on`], the owner's answer setting `gate_pick` as
    /// [`PressAnswer::gate_pick`] does.
    #[allow(clippy::option_option)] // `PressAnswer::gate_pick`'s.
    fn answer_on(map: &mut GalaxyMap, id: i16, gate_pick: Option<Option<SystemId>>) {
        if map.place().is_none() {
            let at = dot(map, id);
            click(map, at);
            return;
        }
        map.centre_on_system(SystemId(id));
        let at = dot(map, id);
        map.input(&button(MouseButton::Left, true, at));
        let press = map.take_press().expect("a press for the owner");
        assert!(press.hits.contains(&SystemId(id)), "{press:?}");
        map.answer_press(Some(PressAnswer {
            selected: Some(SystemId(id)),
            gate_pick,
            clear_route: None,
        }));
        map.input(&button(MouseButton::Left, false, at));
    }

    /// Where system `id` is drawn.
    fn dot(map: &GalaxyMap, id: i16) -> Point {
        let system = map.model().system(SystemId(id)).expect("a system");
        map.transform().world_to_screen(system.position())
    }

    /// The viewer's view.
    fn view_of(map: &GalaxyMap) -> MapView {
        *map.view().expect("the viewer's view")
    }

    fn drawn(map: &GalaxyMap) -> DrawList {
        let mut list = DrawList::new();
        map.draw(&mut list);
        list
    }

    /// Whether `command` is a system's name on the map, not the panel's.
    fn is_name(command: &DrawCommand) -> bool {
        matches!(command, DrawCommand::Text { size, .. } if *size == NAME_SIZE)
    }

    /// The panel's texts: every text but the systems' names.
    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter(|command| !is_name(command))
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn text(list: &DrawList, starting: &str) -> DrawCommand {
        list.iter()
            .filter(|command| !is_name(command))
            .find(|command| matches!(command, DrawCommand::Text { text, .. } if text.starts_with(starting)))
            .cloned()
            .unwrap_or_else(|| panic!("no text starting {starting:?} in {:?}", texts(list)))
    }

    fn origin(list: &DrawList, starting: &str) -> Point {
        match text(list, starting) {
            DrawCommand::Text { origin, .. } => origin,
            _ => unreachable!(),
        }
    }

    /// The rects of `color` (the horizontal lines [`fill_rect`] draws), as
    /// (left, top, right, bottom), in draw order.
    fn rects(list: &DrawList, of_color: Color) -> Vec<(f32, f32, f32, f32)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if color == of_color && from.y == to.y => Some((
                    from.x.min(to.x),
                    from.y - width / 2.0,
                    from.x.max(to.x),
                    from.y + width / 2.0,
                )),
                _ => None,
            })
            .collect()
    }

    /// The rects [`fill_pixels`] fills `pixels` with.
    fn filled(pixels: &[(i32, i32)]) -> Vec<(f32, f32, f32, f32)> {
        let mut list = DrawList::new();
        fill_pixels(&mut list, pixels, Color::WHITE);
        rects(&list, Color::WHITE)
    }

    /// The lines of `color`, as (from, to, width), in draw order.
    fn lines(list: &DrawList, of_color: Color) -> Vec<(Point, Point, f32)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Line {
                    from,
                    to,
                    width,
                    color,
                } if *color == of_color => Some((*from, *to, *width)),
                _ => None,
            })
            .collect()
    }

    // Building.

    #[test]
    fn it_reads_the_galaxy_once_and_starts_fitted_with_nothing_selected() {
        let catalog = FakeCatalog::new(galaxy());
        let mut map = GalaxyMap::new(&catalog);
        assert_eq!(catalog.reads.get(), 1);
        assert_eq!(map.selected(), None);
        // 600 x 300: 133% fits, 178% does not.
        assert_eq!(view_of(&map).scale(), SCALES[4]);
        assert_eq!(view_of(&map).center(), at(300.0, 150.0));
        assert_eq!(map.model().systems().len(), 4);
        drawn(&map);
        click(&mut map, MAP_CENTER);
        map.tick(Duration::from_secs(1));
        assert_eq!(catalog.reads.get(), 1, "only building reads the galaxy");
    }

    #[test]
    fn problems_are_the_catalogs_and_the_models() {
        let mut galaxy = galaxy();
        galaxy.problems.push("sÿst 140: bad".to_owned());
        galaxy.systems[1].govt = Some(GovtId(200));
        let map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        assert_eq!(map.problems(), ["sÿst 140: bad", "sÿst 129: no gövt 200"]);
    }

    // Keys.

    #[test]
    fn the_arrow_keys_pan_one_step() {
        let mut map = map();
        let scale = view_of(&map).scale();
        let start = view_of(&map).center();
        let steps = [
            (Key::Right, (1.0, 0.0)),
            (Key::Left, (0.0, 0.0)),
            (Key::Left, (-1.0, 0.0)),
            (Key::Down, (-1.0, 1.0)),
            (Key::Up, (-1.0, 0.0)),
        ];
        for (key, (x, y)) in steps {
            assert_eq!(map.input(&press(key)), ScreenAction::None);
            let expected = at(
                start.x + x * PAN_STEP / scale,
                start.y + y * PAN_STEP / scale,
            );
            let center = view_of(&map).center();
            assert!(
                (center.x - expected.x).abs() < 1e-3 && (center.y - expected.y).abs() < 1e-3,
                "{key:?}: {center:?} is not {expected:?}"
            );
        }
    }

    #[test]
    fn held_keys_go_on_panning_and_zooming() {
        let mut map = map();
        let (scale, start, fitted) = (
            view_of(&map).scale(),
            view_of(&map).center(),
            view_of(&map).zoom(),
        );
        map.input(&press(Key::Right));
        assert_eq!(map.input(&held(Key::Right)), ScreenAction::None);
        map.input(&held(Key::Right));
        let center = view_of(&map).center();
        let expected = start.x + 3.0 * PAN_STEP / scale;
        assert!((center.x - expected).abs() < 1e-3, "{center:?}");
        map.input(&held(Key::Char('=')));
        assert_eq!(view_of(&map).zoom(), fitted + 1);
    }

    #[test]
    fn equals_and_plus_zoom_in_and_minus_zooms_out() {
        let mut map = map();
        let fitted = view_of(&map).zoom();
        map.input(&press(Key::Char('=')));
        assert_eq!(view_of(&map).zoom(), fitted + 1);
        map.input(&press(Key::Char('+')));
        assert_eq!(view_of(&map).zoom(), fitted + 2);
        for _ in 0..3 {
            assert_eq!(map.input(&press(Key::Char('-'))), ScreenAction::None);
        }
        assert_eq!(view_of(&map).zoom(), fitted - 1);
    }

    #[test]
    fn releases_and_other_keys_change_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        let (view, selected) = (view_of(&map), map.selected());
        let others = [
            Input::Key {
                key: Key::Right,
                pressed: false,
                repeat: false,
            },
            Input::Key {
                key: Key::Char('='),
                pressed: false,
                repeat: false,
            },
            press(Key::Escape),
            press(Key::Tab),
            press(Key::Space),
            press(Key::Char('x')),
            press(Key::Other),
        ];
        for input in others {
            assert_eq!(map.input(&input), ScreenAction::None, "{input:?}");
        }
        assert_eq!((view_of(&map), map.selected()), (view, selected));
    }

    // The pointer.

    #[test]
    fn a_click_on_a_system_selects_it_and_on_empty_space_clears_it() {
        let mut map = map();
        click_on(&mut map, 129);
        assert_eq!(map.selected(), Some(SystemId(129)));
        let beta = dot(&map, 129);
        click(&mut map, at(beta.x - 40.0, beta.y));
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn clicking_a_stacked_position_again_cycles_through_its_systems() {
        let mut map = map();
        let alpha = dot(&map, 128);
        let mut visited = Vec::new();
        for _ in 0..3 {
            click(&mut map, alpha);
            visited.push(map.selected().expect("a system").0);
        }
        assert_eq!(visited, [128, 131, 128]);
    }

    #[test]
    fn a_press_moved_no_more_than_the_slop_is_still_a_click() {
        let mut map = map();
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, beta));
        map.input(&Input::PointerMoved(at(beta.x + 2.0, beta.y)));
        map.input(&Input::PointerMoved(at(beta.x + 2.0, beta.y + 2.0)));
        map.input(&button(
            MouseButton::Left,
            false,
            at(beta.x + 2.0, beta.y + 2.0),
        ));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn a_press_moved_past_the_slop_drags_the_map_and_keeps_the_selection() {
        let mut map = map();
        click_on(&mut map, 129);
        let start = view_of(&map).center();
        let scale = view_of(&map).scale();
        let gamma = dot(&map, 130);
        map.input(&button(MouseButton::Left, true, gamma));
        map.input(&Input::PointerMoved(at(gamma.x + 3.0, gamma.y - 4.0)));
        // Travelled 5 in all, back where it started.
        map.input(&Input::PointerMoved(gamma));
        map.input(&button(MouseButton::Left, false, gamma));
        assert_eq!(
            map.selected(),
            Some(SystemId(129)),
            "a drag selects nothing"
        );
        assert_eq!(view_of(&map).center(), start, "dragged there and back");

        let empty = at(gamma.x + 100.0, gamma.y);
        map.input(&button(MouseButton::Left, true, empty));
        map.input(&Input::PointerMoved(at(empty.x - 20.0, empty.y + 10.0)));
        map.input(&button(
            MouseButton::Left,
            false,
            at(empty.x - 20.0, empty.y + 10.0),
        ));
        let center = view_of(&map).center();
        assert_eq!(center, at(start.x + 20.0 / scale, start.y - 10.0 / scale));
        assert_eq!(map.selected(), Some(SystemId(129)), "nor clears it");
    }

    #[test]
    fn moving_the_pointer_without_a_press_pans_nothing() {
        let mut map = map();
        let view = view_of(&map);
        map.input(&Input::PointerMoved(at(10.0, 10.0)));
        map.input(&Input::PointerMoved(at(400.0, 300.0)));
        assert_eq!(view_of(&map), view);
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        let alpha = dot(&map, 128);
        map.input(&button(MouseButton::Left, false, alpha));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    /// Systems 128 at (0, -250) and 129 at (0, 250), fitted at 100%
    /// round (0, 0), then dragged 52 down so 129's dot is 2 above the panel.
    fn near_the_panel() -> GalaxyMap {
        let galaxy = Galaxy {
            systems: vec![system(128, 0, -250), system(129, 0, 250)],
            ..Galaxy::default()
        };
        let mut map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        assert_eq!(view_of(&map).scale(), 1.0);
        map.input(&button(MouseButton::Left, true, at(300.0, 300.0)));
        map.input(&Input::PointerMoved(at(300.0, 352.0)));
        map.input(&button(MouseButton::Left, false, at(300.0, 352.0)));
        assert_eq!(dot(&map, 129), at(512.0, 606.0));
        map
    }

    #[test]
    fn a_press_in_the_panel_starts_nothing() {
        let mut map = near_the_panel();
        let low = dot(&map, 129);
        let view = view_of(&map);
        // From the panel's top edge, 2 below the dot: no click.
        map.input(&button(MouseButton::Left, true, at(low.x, MAP_HEIGHT)));
        map.input(&Input::PointerMoved(low));
        map.input(&button(MouseButton::Left, false, low));
        assert_eq!(map.selected(), None);
        // Nor a drag.
        map.input(&button(MouseButton::Left, true, at(100.0, 700.0)));
        map.input(&Input::PointerMoved(at(200.0, 300.0)));
        map.input(&button(MouseButton::Left, false, at(200.0, 300.0)));
        assert_eq!(view_of(&map), view);
        // Just above the edge is the map.
        map.input(&button(
            MouseButton::Left,
            true,
            at(low.x, MAP_HEIGHT - 0.5),
        ));
        map.input(&Input::PointerMoved(low));
        map.input(&button(MouseButton::Left, false, low));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn a_click_released_in_the_panel_keeps_the_selection() {
        let mut map = near_the_panel();
        click_on(&mut map, 128);
        let low = dot(&map, 129);
        // Released on the panel's top edge, 2 below the dot.
        map.input(&button(MouseButton::Left, true, low));
        map.input(&button(MouseButton::Left, false, at(low.x, MAP_HEIGHT)));
        assert_eq!(map.selected(), Some(SystemId(128)));
        // Released just above it.
        map.input(&button(MouseButton::Left, true, low));
        map.input(&button(
            MouseButton::Left,
            false,
            at(low.x, MAP_HEIGHT - 0.5),
        ));
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn other_buttons_do_nothing() {
        let mut map = map();
        let beta = dot(&map, 129);
        for other in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            map.input(&button(other, true, beta));
            map.input(&Input::PointerMoved(at(beta.x + 50.0, beta.y)));
            map.input(&button(other, false, at(beta.x + 50.0, beta.y)));
        }
        assert_eq!(map.selected(), None);
        assert_eq!(view_of(&map), view_of(&self::map()));
    }

    #[test]
    fn a_new_press_replaces_one_whose_release_was_lost() {
        let mut map = map();
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, at(beta.x - 100.0, beta.y)));
        map.input(&Input::PointerMoved(at(beta.x - 50.0, beta.y)));
        // The release happened outside the window.
        let beta = dot(&map, 129);
        click(&mut map, beta);
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    #[test]
    fn cancelling_the_pointer_forgets_the_press_without_a_click() {
        let mut map = map();
        let view = view_of(&map);
        let beta = dot(&map, 129);
        map.input(&button(MouseButton::Left, true, beta));
        map.cancel_pointer();
        map.input(&Input::PointerMoved(at(beta.x + 50.0, beta.y)));
        assert_eq!(view_of(&map), view, "no drag");
        map.input(&button(MouseButton::Left, false, beta));
        assert_eq!(map.selected(), None, "no click");

        // The next press starts afresh.
        click(&mut map, beta);
        assert_eq!(map.selected(), Some(SystemId(129)));
    }

    // Drawing.

    fn kind(command: &DrawCommand) -> &'static str {
        match command {
            DrawCommand::Sprite { .. } => "sprite",
            DrawCommand::Colorized { .. } => "colorized",
            DrawCommand::Picture { .. } => "picture",
            DrawCommand::StretchedPicture { .. } => "stretched",
            DrawCommand::BlendedPicture { .. } => "blended",
            DrawCommand::TiledPicture { .. } => "tiled",
            DrawCommand::Text { .. } => "text",
            DrawCommand::Line { .. } => "line",
            DrawCommand::Dot { .. } => "dot",
            DrawCommand::Clipped { .. } => "clipped",
        }
    }

    #[test]
    fn nebulae_are_drawn_before_every_line_and_dot() {
        let list = drawn(&map());
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        let last_nebula = kinds.iter().rposition(|&k| k == "clipped");
        let first_shape = kinds.iter().position(|&k| k == "line" || k == "dot");
        assert_eq!(last_nebula, Some(0));
        assert_eq!(first_shape, Some(1));
        assert!(
            matches!(
                list.iter().next(),
                Some(DrawCommand::Clipped { command, .. })
                    if matches!(**command, DrawCommand::BlendedPicture { blend: Blend::Max, .. })
            ),
            "{:?}",
            list.iter().next()
        );
        assert!(!kinds.contains(&"sprite") && !kinds.contains(&"picture"));
    }

    /// The viewer's frame for the nebula pass: the map area, no port
    /// offset, at its view's centre and zoom.
    fn viewer_frame(map: &GalaxyMap) -> NebulaFrame {
        let view = view_of(map);
        NebulaFrame {
            origin: at(0.0, 0.0),
            centre: (512, 304),
            zoom: view.transform().zoom(),
            scroll: (f64::from(view.center().x), f64::from(view.center().y)),
            area: IRect::new(0, 0, 1024, 608),
        }
    }

    #[test]
    fn the_viewer_draws_a_nebula_through_its_own_frame_at_every_zoom() {
        let mut map = map();
        for _ in 0..SCALES.len() {
            let list = drawn(&map);
            let nebula = &map.model().nebulae()[0];
            let rect = nebula_rect(nebula, &viewer_frame(&map));
            let picture = choose(nebula, rect, PicturePick::LargestThatFits).expect("one");
            let whole = rect.to_bounds();
            assert_eq!(
                list.iter().next(),
                Some(&DrawCommand::Clipped {
                    clip: whole,
                    command: Box::new(DrawCommand::BlendedPicture {
                        image: ImageKey::picture(picture.id),
                        top_left: whole.min,
                        width: whole.width(),
                        height: whole.height(),
                        blend: Blend::Max,
                    }),
                })
            );
            map.input(&press(Key::Char('-')));
        }
    }

    /// As the original's `_DrawMap` (@0xe7a8) draws it: a plain line
    /// between the pair, whichever end lists the link, though by the
    /// engine's `HyperlinkRule` the ship jumps along it one way only.

    #[test]
    fn the_panel_covers_the_bottom_of_the_screen_under_a_separator_then_its_text() {
        let list = drawn(&map());
        assert_eq!(
            lines(&list, PANEL),
            [(at(0.0, 688.0), at(MAP_WIDTH, 688.0), 160.0)]
        );
        assert_eq!(
            lines(&list, SEPARATOR)
                .into_iter()
                .filter(|(from, _, _)| from.y == MAP_HEIGHT)
                .collect::<Vec<_>>(),
            [(at(0.0, MAP_HEIGHT), at(MAP_WIDTH, MAP_HEIGHT), 1.0)]
        );
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        let panel = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == PANEL))
            .expect("the panel");
        assert_eq!(kinds[panel + 1], "line", "the separator is next");
        assert!(
            list.iter()
                .take(panel)
                .all(|c| kind(c) != "text" || is_name(c)),
            "only the names before the panel"
        );
        assert!(kinds[panel + 2..].iter().all(|&k| k == "text"), "{kinds:?}");
        assert!(list.iter().last().is_some_and(|c| kind(c) == "text"));
    }

    // The panel's text.

    #[test]
    fn with_nothing_selected_the_panel_says_how_to_select() {
        let list = drawn(&map());
        assert_eq!(
            text(&list, "Click a system"),
            DrawCommand::Text {
                text: "Click a system to see its stellars".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, TITLE_TOP),
                size: STELLAR_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::DIM,
            }
        );
        assert_eq!(
            text(&list, "Arrows"),
            DrawCommand::Text {
                text: HELP.to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, HELP_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::DIM,
            }
        );
        assert_eq!(texts(&list).len(), 3, "{:?}", texts(&list));
    }

    #[test]
    fn a_selected_system_shows_its_name_and_id_then_its_stellars() {
        let mut map = map();
        click_on(&mut map, 128);
        let list = drawn(&map);
        assert_eq!(
            text(&list, "Alpha"),
            DrawCommand::Text {
                text: "Alpha (sÿst 128)".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, TITLE_TOP),
                size: TITLE_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::WHITE,
            }
        );
        assert_eq!(
            text(&list, "Alpha Station"),
            DrawCommand::Text {
                text: "Alpha Station".to_owned(),
                font: Font::Geneva,
                origin: at(LEFT, STELLARS_TOP + STELLAR_SPACING),
                size: STELLAR_SIZE,
                wrap_width: Some(LEFT_WRAP),
                color: Color::WHITE,
            }
        );
        assert_eq!(origin(&list, "Alpha Prime"), at(LEFT, STELLARS_TOP));
        assert!(!texts(&list).iter().any(|t| t.starts_with("Click a system")));
    }

    #[test]
    fn a_system_without_stellars_says_so() {
        let mut map = map();
        click_on(&mut map, 129);
        let list = drawn(&map);
        let texts = texts(&list);
        assert!(texts.contains(&"Beta (sÿst 129)".to_owned()), "{texts:?}");
        assert_eq!(origin(&list, "No stellars"), at(LEFT, STELLARS_TOP));
        // The title, "No stellars", help, zoom and the button's label: no
        // stack line.
        assert_eq!(texts.len(), 5, "{texts:?}");
    }

    #[test]
    fn a_system_sharing_its_position_says_which_of_the_stack_it_is() {
        let mut map = map();
        click_on(&mut map, 128);
        let list = drawn(&map);
        let line = "1 of 2 systems here (sÿst 128, 131): click again for the next";
        assert_eq!(
            text(&list, "1 of 2"),
            DrawCommand::Text {
                text: line.to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, STACK_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::WHITE,
            }
        );
        click_on(&mut map, 128);
        let list = drawn(&map);
        assert!(texts(&list).contains(&"Alpha Reborn (sÿst 131)".to_owned()));
        text(&list, "2 of 2 systems here (sÿst 128, 131)");
    }

    #[test]
    fn the_zoom_is_shown_as_a_rounded_percentage() {
        let mut map = map();
        for _ in 0..SCALES.len() {
            map.input(&press(Key::Char('-')));
        }
        let mut shown = Vec::new();
        for _ in 0..SCALES.len() {
            let list = drawn(&map);
            assert_eq!(origin(&list, "Zoom"), at(RIGHT, ZOOM_TOP));
            shown.push(
                texts(&list)
                    .into_iter()
                    .find(|t| t.starts_with("Zoom"))
                    .expect("zoom"),
            );
            map.input(&press(Key::Char('=')));
        }
        assert_eq!(
            shown,
            [
                "Zoom 42%",
                "Zoom 56%",
                "Zoom 75%",
                "Zoom 100%",
                "Zoom 133%",
                "Zoom 178%",
                "Zoom 237%"
            ]
        );
    }

    #[test]
    fn problems_are_counted_with_the_first_one_in_red() {
        let mut galaxy = galaxy();
        galaxy.problems = vec!["sÿst 140: bad".to_owned(), "gövt 129: bad".to_owned()];
        let list = drawn(&GalaxyMap::new(&FakeCatalog::new(galaxy)));
        assert_eq!(
            text(&list, "2 problem"),
            DrawCommand::Text {
                text: "2 problem(s) reading the map data: sÿst 140: bad".to_owned(),
                font: Font::Geneva,
                origin: at(RIGHT, PROBLEMS_TOP),
                size: RIGHT_SIZE,
                wrap_width: Some(RIGHT_WRAP),
                color: Color::ERROR,
            }
        );
        assert!(!texts(&drawn(&map())).iter().any(|t| t.contains("problem")));
    }

    #[test]
    fn an_empty_galaxy_draws_only_the_panel() {
        let map = GalaxyMap::new(&FakeCatalog::new(Galaxy::default()));
        let list = drawn(&map);
        let kinds: Vec<&str> = list.iter().map(kind).collect();
        assert_eq!(kinds, ["line", "line", "text", "text", "text"]);
        assert_eq!(view_of(&map).zoom(), 3);
    }

    // Entering a system.

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn return_on_a_selected_system_asks_to_enter_it_once() {
        let mut map = map();
        click_on(&mut map, 129);
        let (view, selected) = (view_of(&map), map.selected());
        assert_eq!(map.input(&press(Key::Enter)), ScreenAction::None);
        assert_eq!(map.take_entry(), Some(SystemId(129)));
        assert_eq!(map.take_entry(), None, "taken");
        assert_eq!((view_of(&map), map.selected()), (view, selected));
    }

    #[test]
    fn return_with_nothing_selected_asks_nothing() {
        let mut map = map();
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), None);
        click_on(&mut map, 130);
        map.input(&press(Key::Enter));
        // The last request is the one taken.
        click_on(&mut map, 129);
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), Some(SystemId(129)));
    }

    #[test]
    fn a_return_repeat_or_release_asks_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&held(Key::Enter));
        map.input(&release(Key::Enter));
        assert_eq!(map.take_entry(), None);
    }

    /// The button's centre.
    fn button_centre() -> Point {
        ENTER_BUTTON.center()
    }

    /// The button's quad: a line through its middle as thick as it is.
    fn button_quad() -> (Point, Point, f32) {
        (at(528.0, 728.0), at(708.0, 728.0), 32.0)
    }

    #[test]
    fn the_enter_button_is_drawn_only_while_a_system_is_selected() {
        let mut map = map();
        let list = drawn(&map);
        assert_eq!(lines(&list, BUTTON), []);
        assert!(!texts(&list).contains(&ENTER_LABEL.to_owned()));

        click_on(&mut map, 129);
        let list = drawn(&map);
        assert_eq!(lines(&list, BUTTON), [button_quad()]);
        assert_eq!(
            text(&list, "Enter system"),
            DrawCommand::Text {
                text: "Enter system (Return)".to_owned(),
                font: Font::Geneva,
                origin: at(540.0, 720.0),
                size: 14.0,
                wrap_width: None,
                color: Color::WHITE,
            }
        );
        assert_eq!(
            ENTER_BUTTON,
            Bounds {
                min: at(528.0, 712.0),
                max: at(708.0, 744.0)
            }
        );
        assert_eq!(BUTTON, Color::rgba(48, 48, 80, 255));
        // Over the panel, under its text.
        let quad = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == BUTTON));
        let panel = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Line { color, .. } if *color == PANEL));
        let first_text = list
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { .. }) && !is_name(c));
        assert!(panel < quad && quad < first_text, "{quad:?}");
    }

    #[test]
    fn clicking_the_enter_button_asks_to_enter_the_selected_system() {
        let mut map = map();
        click_on(&mut map, 130);
        let (view, selected) = (view_of(&map), map.selected());
        click(&mut map, button_centre());
        assert_eq!(map.take_entry(), Some(SystemId(130)));
        assert_eq!((view_of(&map), map.selected()), (view, selected));
        // At its corners too.
        for corner in [ENTER_BUTTON.min, ENTER_BUTTON.max] {
            click(&mut map, corner);
            assert_eq!(map.take_entry(), Some(SystemId(130)), "{corner:?}");
        }
    }

    #[test]
    fn the_enter_button_does_nothing_with_nothing_selected() {
        let mut map = map();
        click(&mut map, button_centre());
        assert_eq!(map.take_entry(), None);
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn a_press_on_the_button_released_off_it_does_not_enter() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.input(&button(MouseButton::Left, false, at(720.0, 728.0)));
        assert_eq!(map.take_entry(), None);
        // That release disarmed it: a release on the button now is not a
        // click.
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn a_press_elsewhere_released_on_the_button_does_not_enter() {
        let mut map = map();
        click_on(&mut map, 129);
        let view = view_of(&map);
        for start in [at(300.0, 728.0), at(512.0, 300.0)] {
            map.input(&button(MouseButton::Left, true, start));
            map.input(&button(MouseButton::Left, false, button_centre()));
            assert_eq!(map.take_entry(), None, "{start:?}");
        }
        assert_eq!(map.selected(), Some(SystemId(129)), "nor clears it");
        assert_eq!(view_of(&map), view);
    }

    #[test]
    fn a_new_press_off_the_button_disarms_it() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        // Its release was lost; the next press is elsewhere in the panel.
        map.input(&button(MouseButton::Left, true, at(300.0, 728.0)));
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn a_press_on_the_button_starts_no_drag() {
        let mut map = map();
        click_on(&mut map, 129);
        let view = view_of(&map);
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.input(&Input::PointerMoved(at(100.0, 100.0)));
        assert_eq!(view_of(&map), view);
    }

    #[test]
    fn other_buttons_on_the_enter_button_do_nothing() {
        let mut map = map();
        click_on(&mut map, 129);
        for other in [MouseButton::Right, MouseButton::Middle, MouseButton::Other] {
            map.input(&button(other, true, button_centre()));
            map.input(&button(other, false, button_centre()));
        }
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn cancelling_the_pointer_disarms_the_button() {
        let mut map = map();
        click_on(&mut map, 129);
        map.input(&button(MouseButton::Left, true, button_centre()));
        map.cancel_pointer();
        map.input(&button(MouseButton::Left, false, button_centre()));
        assert_eq!(map.take_entry(), None);
    }

    #[test]
    fn the_help_line_mentions_return() {
        assert_eq!(
            HELP,
            "Arrows or drag: pan   +/-: zoom   Click: select   Return: enter   S: find"
        );
    }

    // Plotting a course.

    fn course_map() -> GalaxyMap {
        GalaxyMap::course(&FakeCatalog::new(galaxy()))
    }

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    #[test]
    fn a_course_map_reads_the_galaxy_once_and_starts_with_no_course() {
        let catalog = FakeCatalog::new(galaxy());
        let map = GalaxyMap::course(&catalog);
        assert_eq!(catalog.reads.get(), 1);
        assert_eq!(map.mode(), MapMode::Course);
        assert_eq!(map.current(), None);
        assert_eq!(map.route(), []);
        assert_eq!(map.picture(), None, "nothing drawn until shown");
        assert_eq!(self::map().mode(), MapMode::Viewer);
    }

    #[test]
    fn a_course_map_has_no_enter_button_and_return_enters_nothing() {
        let mut map = course_map();
        click_on(&mut map, 129);
        let list = drawn(&map);
        assert_eq!(lines(&list, BUTTON), []);
        assert!(!texts(&list).contains(&ENTER_LABEL.to_owned()));
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), None);
        click(&mut map, button_centre());
        assert_eq!(map.take_entry(), None);
        assert_eq!(
            map.selected(),
            Some(SystemId(129)),
            "the panel is not the map"
        );
    }

    /// The course map in Alpha (128), looking at Beta (129).
    fn on_beta() -> (GalaxyMap, Point) {
        let mut map = course_map();
        map.show_course(SystemId(128), &[]);
        map.centre_on_system(SystemId(129));
        let beta = dot(&map, 129);
        (map, beta)
    }

    /// The pointer moved by (`dx`, `dy`) from `from`.
    fn moved(from: Point, dx: f32, dy: f32) -> Input {
        Input::PointerMoved(at(from.x + dx, from.y + dy))
    }

    #[test]
    fn a_press_on_a_system_is_held_for_the_owner_once_with_its_hits_and_shift() {
        let (mut map, beta) = on_beta();
        assert_eq!(map.take_press(), None);
        map.input(&button(MouseButton::Left, true, beta));
        assert_eq!(
            map.take_press(),
            Some(PendingPress {
                at: beta,
                hits: ids(&[129]),
                shift: false,
            })
        );
        assert_eq!(map.take_press(), None, "taken");
        assert_eq!(map.selected(), None, "the owner selects");
        map.input(&button(MouseButton::Left, false, beta));
        assert_eq!(map.take_press(), None, "no click on the release");
        map.input(&press(Key::Shift));
        map.input(&button(MouseButton::Left, true, beta));
        assert!(map.take_press().expect("held").shift, "Shift held");
    }

    #[test]
    fn a_held_press_drags_nothing_until_the_owner_accepts_no_hit() {
        let (mut map, beta) = on_beta();
        let scroll = map.place().expect("in flight").scroll();
        map.input(&button(MouseButton::Left, true, beta));
        map.input(&moved(beta, 40.0, 0.0));
        assert_eq!(map.place().expect("in flight").scroll(), scroll, "held");
        map.take_press().expect("held");
        map.input(&moved(beta, 40.0, 0.0));
        assert_eq!(map.place().expect("in flight").scroll(), scroll, "taken");
        map.answer_press(None);
        map.input(&moved(beta, 40.0, 0.0));
        assert_ne!(
            map.place().expect("in flight").scroll(),
            scroll,
            "dragged from the press"
        );
        map.input(&button(MouseButton::Left, false, beta));
        let released = map.place().expect("in flight").scroll();
        map.input(&moved(beta, 80.0, 0.0));
        assert_eq!(map.place().expect("in flight").scroll(), released);
        assert_eq!(map.selected(), None, "a drag selects nothing");
    }

    #[test]
    fn an_answered_press_selects_and_drags_nothing() {
        let (mut map, beta) = on_beta();
        let scroll = map.place().expect("in flight").scroll();
        map.input(&button(MouseButton::Left, true, beta));
        map.take_press().expect("held");
        map.answer_press(Some(PressAnswer {
            selected: Some(SystemId(129)),
            gate_pick: None,
            clear_route: None,
        }));
        assert_eq!(map.selected(), Some(SystemId(129)));
        map.input(&moved(beta, 40.0, 0.0));
        map.input(&button(MouseButton::Left, false, beta));
        assert_eq!(map.place().expect("in flight").scroll(), scroll);
        map.answer_press(Some(PressAnswer {
            selected: Some(SystemId(128)),
            gate_pick: None,
            clear_route: None,
        }));
        assert_eq!(map.selected(), Some(SystemId(129)), "nothing to answer");
    }

    #[test]
    fn an_answer_with_no_selection_keeps_the_selection() {
        let (mut map, beta) = on_beta();
        map.preselect(Some(SystemId(130)));
        map.input(&button(MouseButton::Left, true, beta));
        map.take_press().expect("held");
        map.answer_press(Some(PressAnswer {
            selected: None,
            gate_pick: None,
            clear_route: None,
        }));
        assert_eq!(map.selected(), Some(SystemId(130)));
    }

    #[test]
    fn a_press_on_empty_space_drags_at_once_and_holds_nothing() {
        let (mut map, beta) = on_beta();
        let scroll = map.place().expect("in flight").scroll();
        map.preselect(Some(SystemId(129)));
        let empty = at(beta.x - 40.0, beta.y);
        map.input(&button(MouseButton::Left, true, empty));
        assert_eq!(map.take_press(), None);
        map.input(&moved(empty, 40.0, 0.0));
        assert_ne!(map.place().expect("in flight").scroll(), scroll);
        map.input(&button(MouseButton::Left, false, empty));
        assert_eq!(
            map.selected(),
            Some(SystemId(129)),
            "a release on empty space keeps the selection"
        );
    }

    #[test]
    fn a_new_press_replaces_a_held_one_and_cancelling_forgets_it() {
        let (mut map, beta) = on_beta();
        map.input(&button(MouseButton::Left, true, beta));
        let empty = at(beta.x - 40.0, beta.y);
        map.input(&button(MouseButton::Left, true, empty));
        assert_eq!(map.take_press(), None, "replaced by a drag");
        map.input(&button(MouseButton::Left, true, beta));
        map.cancel_pointer();
        assert_eq!(map.take_press(), None, "cancelled");
        map.input(&button(MouseButton::Left, true, beta));
        map.take_press().expect("held");
        map.cancel_pointer();
        map.answer_press(None);
        let scroll = map.place().expect("in flight").scroll();
        map.input(&moved(beta, 40.0, 0.0));
        assert_eq!(
            map.place().expect("in flight").scroll(),
            scroll,
            "a cancelled press answered starts no drag"
        );
    }

    #[test]
    fn the_viewer_holds_no_press_and_selects_an_unknown_inactive_system() {
        // A course map shown a galaxy hiding Beta (129) gives no hit for
        // it; the viewer, unfiltered, selects it.
        let (mut course, beta) = on_beta();
        course.show_galaxy(&session_galaxy(|overlay| {
            overlay.set_visible(SystemId(129), Some(false));
        }));
        course.input(&button(MouseButton::Left, true, beta));
        assert_eq!(course.take_press(), None, "no hit on the course map");
        let mut map = map();
        click_on(&mut map, 129);
        assert_eq!(map.selected(), Some(SystemId(129)));
        assert_eq!(map.take_press(), None, "nothing for an owner");
        assert!(!map.take_clear_route(), "no clear asked");
        map.input(&press(Key::Enter));
        assert_eq!(map.take_entry(), Some(SystemId(129)));
    }

    // The hyperspace selection.

    #[test]
    fn the_preselect_selects_a_system_and_the_panel_shows_it() {
        let mut map = course_map();
        map.centre_on_system(SystemId(129));
        map.preselect(Some(SystemId(129)));
        assert_eq!(map.selected(), Some(SystemId(129)));
        let list = drawn(&map);
        assert_eq!(rects(&list, BRIGHT_GREEN).len(), 8, "bracketed");
        assert!(
            !texts(&list).iter().any(|text| text.starts_with("Beta (")),
            "the session's panel, not the viewer's"
        );
        assert_eq!(map.take_press(), None, "no press");
        map.preselect(None);
        assert_eq!(map.selected(), None);
        assert!(
            !texts(&drawn(&map))
                .iter()
                .any(|text| text.starts_with("Click a system"))
        );
        map.preselect(Some(SystemId(999)));
        assert_eq!(map.selected(), None, "not on the map");
    }

    #[test]
    fn the_preselect_is_kept_only_while_the_session_shows_it() {
        let mut map = course_map();
        map.show_galaxy(&session_galaxy(|overlay| {
            overlay.set_visible(SystemId(129), Some(false));
        }));
        map.preselect(Some(SystemId(129)));
        assert_eq!(map.selected(), None, "hidden");
    }

    #[test]
    fn an_in_flight_press_gives_every_hit_at_a_place_whatever_the_selection() {
        // `_DoSystemMap`'s click (@0x12dd4-0x1314b) never reads
        // `_selSystem`, and has no stack cycling: it walks the systems.
        for mut map in [course_map(), gate_map()] {
            map.show_course(SystemId(129), &[]);
            for selected in [None, Some(SystemId(128)), Some(SystemId(131))] {
                map.preselect(selected);
                map.centre_on_system(SystemId(128));
                let alpha = dot(&map, 128);
                map.input(&button(MouseButton::Left, true, alpha));
                assert_eq!(
                    map.take_press().expect("held").hits,
                    ids(&[128, 131]),
                    "{selected:?}"
                );
                map.answer_press(None);
                map.input(&button(MouseButton::Left, false, alpha));
                assert_eq!(map.selected(), selected, "{selected:?}");
            }
        }
    }

    // The hypergate map.

    /// The course map in Alpha (128), offering Beta (129) and Gamma (130)
    /// through a hypergate.
    fn gate_map() -> GalaxyMap {
        let mut map = course_map();
        map.show_course(SystemId(128), &[]);
        map.offer_gates(ids(&[129, 130]));
        map
    }

    #[test]
    fn offering_gates_switches_to_the_hypergate_map_with_nothing_selected() {
        let mut map = course_map();
        click_on(&mut map, 129);
        map.offer_gates(ids(&[129, 130]));
        assert_eq!(map.mode(), MapMode::Hypergate);
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn the_hypergate_map_takes_the_owners_pick_and_the_current_system_drops_it() {
        let mut map = gate_map();
        gate_click_on(&mut map, 129);
        assert_eq!(map.selected(), Some(SystemId(129)));
        gate_click_dropping(&mut map, 128);
        assert_eq!(map.selected(), Some(SystemId(128)), "the current system");
        assert_eq!(map.take_gate_choice(), None, "the pick dropped");
        let mut map = gate_map();
        gate_click_on(&mut map, 130);
        click(&mut map, at(250.0, 150.0));
        assert_eq!(map.selected(), Some(SystemId(130)), "empty space");
        click_on(&mut map, 129);
        assert_eq!(map.take_gate_choice(), Some(SystemId(130)), "pick kept");
        assert_eq!(map.route(), []);
    }

    #[test]
    fn taking_the_gate_choice_gives_it_and_returns_to_the_course_map() {
        let mut map = gate_map();
        gate_click_on(&mut map, 130);
        assert_eq!(map.take_gate_choice(), Some(SystemId(130)));
        assert_eq!(map.mode(), MapMode::Course);
        assert_eq!(map.selected(), None);
        assert_eq!(map.take_gate_choice(), None, "taken");
        assert_eq!(map.offered(), [], "a course map again");

        let mut unpicked = gate_map();
        assert_eq!(unpicked.take_gate_choice(), None);
        assert_eq!(unpicked.mode(), MapMode::Course);

        let mut course = course_map();
        click_on(&mut course, 129);
        assert_eq!(course.take_gate_choice(), None, "no gates offered");
        assert_eq!(course.selected(), Some(SystemId(129)));
    }

    // Tab, `\` and Return on the in-flight maps.

    #[test]
    fn tab_and_backslash_ask_the_course_and_hypergate_maps_to_cycle_repeats_too() {
        for mut map in [course_map(), gate_map()] {
            assert_eq!(map.take_cycle(), None);
            for k in [Key::Tab, Key::Char('\\')] {
                map.input(&press(k));
                assert_eq!(map.take_cycle(), Some(MapCycle { back: false }), "{k:?}");
                assert_eq!(map.take_cycle(), None, "taken");
                map.input(&held(k));
                assert_eq!(map.take_cycle(), Some(MapCycle { back: false }), "a repeat");
                map.input(&release(k));
                assert_eq!(map.take_cycle(), None, "a release");
            }
            map.input(&press(Key::Shift));
            map.input(&press(Key::Tab));
            assert_eq!(map.take_cycle(), Some(MapCycle { back: true }), "Shift");
            map.input(&release(Key::Shift));
            map.input(&press(Key::Char('\\')));
            assert_eq!(map.take_cycle(), Some(MapCycle { back: false }));
            map.input(&press(Key::Shift));
            map.release_keys();
            map.input(&press(Key::Tab));
            assert_eq!(
                map.take_cycle(),
                Some(MapCycle { back: false }),
                "Shift let go"
            );
        }
    }

    #[test]
    fn the_viewer_never_asks_to_cycle_or_close_and_return_still_enters() {
        let mut map = map();
        click_on(&mut map, 129);
        let view = view_of(&map);
        for k in [Key::Tab, Key::Char('\\'), Key::Enter] {
            map.input(&press(k));
        }
        assert_eq!(map.take_cycle(), None);
        assert!(!map.take_close());
        assert_eq!(map.take_entry(), Some(SystemId(129)));
        assert_eq!(view_of(&map), view);
    }

    #[test]
    fn return_or_enter_asks_the_course_and_hypergate_maps_to_close_but_not_a_repeat() {
        for mut map in [course_map(), gate_map()] {
            assert!(!map.take_close());
            map.input(&held(Key::Enter));
            map.input(&release(Key::Enter));
            assert!(!map.take_close(), "a repeat or a release");
            map.input(&press(Key::Enter));
            assert!(map.take_close());
            assert!(!map.take_close(), "taken");
            assert_eq!(map.take_entry(), None);
        }
    }

    #[test]
    fn a_gate_pick_is_selected_as_it_is_and_taken_only_when_offered() {
        let mut map = gate_map();
        map.select_gate(Some(SystemId(999)));
        assert_eq!(map.selected(), Some(SystemId(999)), "not on the map");
        assert_eq!(map.take_gate_choice(), None, "not offered");
        let mut map = gate_map();
        map.select_gate(Some(SystemId(130)));
        assert_eq!(map.selected(), Some(SystemId(130)));
        assert_eq!(map.take_gate_choice(), Some(SystemId(130)));
        let mut map = gate_map();
        click_on(&mut map, 129);
        map.select_gate(None);
        assert_eq!(map.selected(), None);
        assert_eq!(map.take_gate_choice(), None);
    }

    // The session's galaxy.

    /// The session's galaxy over [`galaxy`]'s systems, with `change` made.
    fn session_galaxy(change: impl FnOnce(&mut GalaxyOverlay)) -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        change(&mut overlay);
        let systems = galaxy().systems.iter().map(star_system).collect();
        OverlaidGalaxy::new(systems, overlay)
    }

    #[test]
    fn showing_the_sessions_galaxy_lets_go_of_a_selection_it_hides() {
        let mut map = map();
        click_on(&mut map, 129);
        map.show_galaxy(&session_galaxy(|_| {}));
        assert_eq!(map.selected(), Some(SystemId(129)), "still drawn");
        map.show_galaxy(&session_galaxy(|overlay| {
            overlay.set_visible(SystemId(130), Some(false));
        }));
        assert_eq!(map.selected(), Some(SystemId(129)), "another hidden");
        map.show_galaxy(&session_galaxy(|overlay| {
            overlay.set_visible(SystemId(129), Some(false));
        }));
        assert_eq!(map.selected(), None);
    }

    // Drawing as `_DrawMap` does.

    use nova_sim::galaxy_map::DrawnSystem;

    fn link(from: i16, to: i16, selected: bool) -> MapLink {
        MapLink {
            from: SystemId(from),
            to: SystemId(to),
            selected,
        }
    }

    fn pair(from: i16, to: i16) -> (SystemId, SystemId) {
        (SystemId(from), SystemId(to))
    }

    fn drawn_system(id: i16, code: SystemColorCode, red: bool, green: bool) -> DrawnSystem {
        DrawnSystem {
            id: SystemId(id),
            code,
            red_arrow: red,
            green_arrow: green,
        }
    }

    /// In Alpha (128), with the course to Beta (129), Alpha's link to
    /// Beta (its mode-3 slot), Gamma's (130) to Alpha, Alpha cleared with
    /// a red arrow and Gamma hostile with a green one, Alpha and Beta
    /// named.
    fn drawing() -> MapDrawing {
        MapDrawing {
            hypergate: false,
            course: vec![pair(128, 129)],
            current_links: vec![pair(128, 129)],
            links: vec![link(130, 128, false), link(128, 129, true)],
            mode3_link: Some(pair(128, 129)),
            systems: vec![
                drawn_system(128, SystemColorCode::Cleared, true, false),
                drawn_system(130, SystemColorCode::Hostile, false, true),
            ],
            current: SystemId(128),
            gate_arrows: Vec::new(),
            named: vec![SystemId(128), SystemId(129)],
        }
    }

    /// The viewer showing `drawing` at scale 1, where Alpha is at
    /// (212, 154), Beta at (812, 154) and Gamma at (212, 454): the
    /// viewer draws a picture as the in-flight maps do, unclipped
    /// (`draw_marks`).
    fn showing(drawing: &MapDrawing) -> GalaxyMap {
        let mut map = map();
        map.input(&press(Key::Char('-')));
        assert_eq!(view_of(&map).scale(), 1.0);
        map.show_drawing(drawing);
        map
    }

    /// Each command of `list` after its nebulae and before its panel's
    /// fill, by kind and colour, each run of the same one once.
    fn order(list: &DrawList) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for command in list {
            let name = match command {
                DrawCommand::Line { color, .. } => match *color {
                    BRIGHT_GREEN => "bright green".to_owned(),
                    LESS_DARK_GREY => "grey".to_owned(),
                    DIM_GREEN => "dim green".to_owned(),
                    Color::BLACK => "black".to_owned(),
                    CLEARED => "blue".to_owned(),
                    HOSTILE => "red".to_owned(),
                    CYAN => "cyan".to_owned(),
                    Color::WHITE => "white".to_owned(),
                    PANEL => break,
                    other => format!("{other:?}"),
                },
                DrawCommand::Sprite { image, .. } => format!("cicn {}", image.id),
                DrawCommand::Text { text, .. } => text.clone(),
                DrawCommand::Clipped { command, .. }
                    if matches!(**command, DrawCommand::BlendedPicture { .. }) =>
                {
                    continue;
                }
                other => format!("{other:?}"),
            };
            if names.last() != Some(&name) {
                names.push(name);
            }
        }
        names
    }

    #[test]
    fn the_map_draws_in_draw_maps_order_then_names_again() {
        let map = showing(&drawing());
        assert_eq!(
            order(&drawn(&map)),
            [
                "bright green",
                "grey",
                "dim green",
                "black",
                "blue",
                "cicn 15000",
                "black",
                "red",
                "cicn 15001",
                "cyan",
                "Alpha",
                "Beta",
                "Alpha",
                "Beta",
            ],
            "course, links, mode-3 link, each system's oval and arrows, the \
             current dot, the names; then (no selection, no brackets) the \
             names again"
        );
    }

    #[test]
    fn the_course_and_links_are_drawn_in_their_colours_and_pens() {
        let map = showing(&drawing());
        let list = drawn(&map);
        assert_eq!(
            rects(&list, BRIGHT_GREEN),
            [(212.0, 154.0, 814.0, 156.0)],
            "the course at pen 2, both ends included"
        );
        assert_eq!(
            rects(&list, LESS_DARK_GREY),
            [(212.0, 154.0, 813.0, 155.0), (212.0, 154.0, 213.0, 455.0)],
            "the current system's link, then Gamma's, at pen 1"
        );
        assert_eq!(
            rects(&list, DIM_GREEN),
            [(212.0, 154.0, 813.0, 155.0), (212.0, 154.0, 813.0, 155.0)],
            "the selected slot, then the mode-3 link again"
        );
        let hypergate = showing(&MapDrawing {
            hypergate: true,
            ..drawing()
        });
        let list = drawn(&hypergate);
        assert_eq!(rects(&list, LESS_DARK_GREY), []);
        assert_eq!(
            rects(&list, REAL_DARK_GREY).len(),
            2,
            "the hypergate map's grey"
        );
    }

    #[test]
    fn a_diagonal_link_is_its_pen_wide_along_the_path_offset_by_half_the_pen() {
        let map = showing(&MapDrawing {
            course: Vec::new(),
            current_links: vec![pair(129, 130)],
            ..drawing()
        });
        let list = drawn(&map);
        assert_eq!(
            lines(&list, LESS_DARK_GREY)[0],
            (at(812.5, 154.5), at(212.5, 454.5), 1.0)
        );
    }

    /// Where `map` draws system `id`, in whole pixels.
    fn pixel_of(map: &GalaxyMap, id: i16) -> (i32, i32) {
        let system = map.model().catalog_system(SystemId(id)).expect("a system");
        map.transform().pixel(system.position())
    }

    fn oval_rect((x, y): (i32, i32), inset: i32) -> IRect {
        IRect::new(x - inset, y - inset, x + inset, y + inset)
    }

    #[test]
    fn each_oval_is_filled_black_then_framed_by_its_pen_at_each_zoom() {
        let mut map = showing(&drawing());
        let list = drawn(&map);
        let alpha = oval_rect((212, 154), SYSTEM_INSET);
        let gamma = oval_rect((212, 454), SYSTEM_INSET);
        let mut black = filled(&oval_fill_pixels(alpha));
        black.extend(filled(&oval_fill_pixels(gamma)));
        assert_eq!(rects(&list, Color::BLACK), black);
        assert_eq!(rects(&list, CLEARED), filled(&oval_frame_pixels(alpha, 1)));
        assert_eq!(rects(&list, HOSTILE), filled(&oval_frame_pixels(gamma, 1)));
        assert_eq!(
            rects(&list, CYAN),
            filled(&oval_fill_pixels(oval_rect((212, 154), CURRENT_INSET)))
        );
        for _ in 0..3 {
            map.input(&press(Key::Char('=')));
        }
        assert_eq!(view_of(&map).scale(), 64.0 / 27.0, "the zoom ≤ 0.5");
        let list = drawn(&map);
        let alpha = oval_rect(pixel_of(&map, 128), SYSTEM_INSET_ZOOMED);
        assert_eq!(rects(&list, CLEARED), filled(&oval_frame_pixels(alpha, 2)));
        assert_eq!(
            rects(&list, CYAN),
            filled(&oval_fill_pixels(oval_rect(
                pixel_of(&map, 128),
                CURRENT_INSET_ZOOMED
            )))
        );
        map.input(&press(Key::Char('-')));
        let list = drawn(&map);
        let alpha = oval_rect(pixel_of(&map, 128), SYSTEM_INSET);
        assert_eq!(
            rects(&list, CLEARED),
            filled(&oval_frame_pixels(alpha, 1)),
            "16/9 is above 0.5"
        );
    }

    #[test]
    fn each_code_frames_in_color_code_systems_colour() {
        let rgb = |code| {
            let color = code_color(code);
            (color.r, color.g, color.b, color.a)
        };
        assert_eq!(rgb(SystemColorCode::Unexplored), (0x40, 0x40, 0x40, 255));
        assert_eq!(rgb(SystemColorCode::Uninhabited), (0xc0, 0xc0, 0xc0, 255));
        assert_eq!(rgb(SystemColorCode::Dominated), (0, 255, 0, 255));
        assert_eq!(rgb(SystemColorCode::Cleared), (0, 0, 255, 255));
        assert_eq!(rgb(SystemColorCode::Denied), (255, 0x66, 0, 255));
        assert_eq!(rgb(SystemColorCode::Hostile), (255, 0, 0, 255));
        assert_eq!(
            (LESS_DARK_GREY.r, REAL_DARK_GREY.r, DIM_GREEN.g, CYAN.b),
            (0x4e, 0x0f, 0x99, 255)
        );
    }

    /// The `cicn` sprites in `list`, as (ID, centre).
    fn icons(list: &DrawList) -> Vec<(i16, Point)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite {
                    image,
                    center,
                    tint,
                    ..
                } if image.kind == crate::ImageKind::Icon => {
                    assert_eq!(tint, Color::WHITE);
                    Some((image.id, center))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_mission_arrows_sit_up_left_and_up_right_of_the_oval() {
        let map = showing(&drawing());
        // Alpha's oval is (208, 150)-(216, 158): the red arrow in
        // (192, 134)-(208, 150); Gamma's (208, 450)-(216, 458): the green
        // one in (216, 434)-(232, 450).
        assert_eq!(
            icons(&drawn(&map)),
            [
                (RED_ARROW, at(200.0, 142.0)),
                (GREEN_ARROW, at(224.0, 442.0))
            ]
        );
        assert_eq!((RED_ARROW, GREEN_ARROW, ARROW_SIZE), (15000, 15001, 16));
    }

    #[test]
    fn the_gate_arrows_are_white_at_pen_3_for_the_selection_else_cyan_at_pen_2() {
        let mut map = showing(&MapDrawing {
            hypergate: true,
            course: Vec::new(),
            current_links: Vec::new(),
            links: Vec::new(),
            mode3_link: None,
            systems: Vec::new(),
            gate_arrows: vec![SystemId(129), SystemId(130)],
            ..drawing()
        });
        map.preselect(Some(SystemId(129)));
        let list = drawn(&map);
        let head = |tip: (f32, f32), tail: (f32, f32)| {
            arrow_head(Vec2::new(tip.0, tip.1), Vec2::new(tail.0, tail.1))
                .map(|end| (end.x.trunc(), end.y.trunc()))
        };
        let [b1, b2] = head((812.0, 154.0), (212.0, 154.0));
        assert_eq!(
            rects(&list, Color::WHITE)[0],
            (212.0, 154.0, 815.0, 157.0),
            "Beta's shaft, horizontal, at pen 3"
        );
        assert_eq!(
            lines(&list, Color::WHITE)[1..3],
            [
                (at(813.5, 155.5), at(b1.0 + 1.5, b1.1 + 1.5), 3.0),
                (at(813.5, 155.5), at(b2.0 + 1.5, b2.1 + 1.5), 3.0),
            ]
        );
        let [g1, g2] = head((212.0, 454.0), (212.0, 154.0));
        assert!(
            rects(&list, CYAN).contains(&(212.0, 154.0, 214.0, 456.0)),
            "Gamma's shaft, vertical, at pen 2"
        );
        let cyan = lines(&list, CYAN);
        let wide: Vec<&(Point, Point, f32)> = cyan.iter().filter(|line| line.2 == 2.0).collect();
        assert_eq!(
            wide,
            [
                &(at(213.0, 455.0), at(g1.0 + 1.0, g1.1 + 1.0), 2.0),
                &(at(213.0, 455.0), at(g2.0 + 1.0, g2.1 + 1.0), 2.0),
            ],
            "its head's strokes"
        );
    }

    #[test]
    fn the_gate_arrows_are_drawn_after_the_current_dot_and_before_the_names() {
        let mut map = showing(&MapDrawing {
            hypergate: true,
            course: Vec::new(),
            current_links: Vec::new(),
            links: Vec::new(),
            mode3_link: None,
            systems: Vec::new(),
            gate_arrows: vec![SystemId(129), SystemId(130)],
            ..drawing()
        });
        map.preselect(Some(SystemId(129)));
        assert_eq!(
            order(&drawn(&map)),
            [
                "cyan",
                "white",
                "cyan",
                "Alpha",
                "Beta",
                "bright green",
                "Alpha",
                "Beta",
            ],
            "the current dot, Beta's (selected) arrow, Gamma's arrow, the \
             names; then the brackets and the names again"
        );
    }

    /// The names drawn in `list`: those at the names' size.
    fn names(list: &DrawList) -> Vec<(String, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font,
                    origin,
                    size,
                    color,
                    wrap_width,
                } if *size == NAME_SIZE => {
                    assert_eq!(
                        (*font, *color, *wrap_width),
                        (NAME_FONT, Color::WHITE, None)
                    );
                    Some((text.clone(), *origin))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn names_sit_on_a_baseline_7_right_and_4_down_and_above_1_1_only_the_selections() {
        let mut map = showing(&drawing());
        let list = drawn(&map);
        let (alpha, beta) = (at(219.0, 149.0), at(819.0, 149.0));
        assert_eq!(
            names(&list),
            [
                ("Alpha".to_owned(), alpha),
                ("Beta".to_owned(), beta),
                ("Alpha".to_owned(), alpha),
                ("Beta".to_owned(), beta),
            ],
            "the baseline 158, Geneva 9 white; Gamma, drawn, unnamed"
        );
        map.input(&press(Key::Char('-')));
        assert_eq!(view_of(&map).scale(), 0.75, "a zoom of 4/3");
        assert_eq!(names(&drawn(&map)), [], "nothing selected");
        click_on(&mut map, 129);
        let beta = names(&drawn(&map));
        assert_eq!(beta.len(), 2);
        assert!(beta.iter().all(|(name, _)| name == "Beta"));
    }

    #[test]
    fn the_brackets_are_eight_4_pixel_ticks_at_each_zoom() {
        let mut map = showing(&MapDrawing {
            course: Vec::new(),
            ..drawing()
        });
        map.preselect(Some(SystemId(130)));
        let ticks = |(x, y): (i32, i32), inset: i32| {
            let (l, t, r, b) = (x - inset, y - inset, x + inset, y + inset);
            [
                (l + 1, t, l + 5, t + 1),
                (l + 1, b, l + 5, b + 1),
                (r - 4, t, r, t + 1),
                (r - 4, b, r, b + 1),
                (l, t + 1, l + 1, t + 5),
                (l, b - 4, l + 1, b),
                (r, t + 1, r + 1, t + 5),
                (r, b - 4, r + 1, b),
            ]
            .map(|(l, t, r, b)| (l as f32, t as f32, r as f32, b as f32))
        };
        let list = drawn(&map);
        assert_eq!(rects(&list, BRIGHT_GREEN), ticks((212, 454), BRACKET_INSET));
        let after = list
            .iter()
            .position(|command| matches!(command, DrawCommand::Line { color, .. } if *color == BRIGHT_GREEN))
            .expect("brackets");
        assert!(
            list.iter().skip(after).any(
                |command| matches!(command, DrawCommand::Text { size, .. } if *size == NAME_SIZE)
            ),
            "the names again after the brackets"
        );
        for _ in 0..3 {
            map.input(&press(Key::Char('=')));
        }
        assert_eq!(
            rects(&drawn(&map), BRIGHT_GREEN),
            ticks(pixel_of(&map, 130), BRACKET_INSET_ZOOMED)
        );
    }

    #[test]
    fn draw_marks_alone_draws_no_brackets_and_names_once() {
        let map = showing(&MapDrawing {
            course: Vec::new(),
            ..drawing()
        });
        let picture = map.picture().expect("shown").clone();
        let transform = Transform {
            screen: at(686.0, 278.0),
            world: at(0.0, 0.0),
            scale: 16.0 / 9.0,
        };
        let marks = MapMarks {
            picture: &picture,
            selected: Some(SystemId(128)),
        };
        let mut list = DrawList::new();
        map.draw_marks(&mut list, &transform, &marks);
        assert_eq!(rects(&list, BRIGHT_GREEN), [], "no brackets");
        assert_eq!(names(&list).len(), 2, "each name once");
        assert!(
            !list.iter().any(|command| matches!(
                command,
                DrawCommand::StretchedPicture { .. }
                    | DrawCommand::BlendedPicture { .. }
                    | DrawCommand::Clipped { .. }
            )),
            "no nebula"
        );
        assert_eq!(
            rects(&list, CYAN),
            filled(&oval_fill_pixels(oval_rect((686, 278), CURRENT_INSET))),
            "the current system at the transform's centre"
        );
    }

    #[test]
    fn the_viewer_draws_every_system_link_and_name_in_the_data() {
        let mut map = map();
        // Nothing the viewer is told filters what it draws.
        map.show_galaxy(&session_galaxy(|overlay| {
            overlay.set_visible(SystemId(130), Some(false));
        }));
        let picture = map.picture().expect("the viewer's");
        let ids: Vec<i16> = picture.systems.iter().map(|system| system.id.0).collect();
        assert_eq!(ids, [128, 129, 130, 131]);
        let frames: Vec<Color> = picture.systems.iter().map(|system| system.frame).collect();
        let blue = Color::from_rgb24(BLUE);
        assert_eq!(frames, [blue, NEUTRAL, blue, NEUTRAL], "by government");
        assert_eq!(
            picture.links,
            [link(128, 129, false), link(128, 130, false)]
        );
        assert_eq!(picture.named, ids_of(&[128, 129, 130, 131]));
        assert_eq!(
            (
                picture.current,
                picture.course.len(),
                picture.gate_arrows.len()
            ),
            (None, 0, 0)
        );
        map.input(&press(Key::Char('-')));
        assert_eq!(view_of(&map).scale(), 1.0);
        let list = drawn(&map);
        let names = names(&list);
        assert!(
            names.iter().any(|(name, _)| name == "Gamma"),
            "hidden, unexplored: {names:?}"
        );
        assert_eq!(lines(&list, LESS_DARK_GREY).len(), 2);
        assert_eq!(
            rects(&list, Color::BLACK).len(),
            4 * filled(&oval_fill_pixels(oval_rect((0, 0), SYSTEM_INSET))).len(),
            "every system's oval"
        );
    }

    fn ids_of(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    // The session's panel.

    use nova_sim::map_panel::{Column, ColumnBody, MapPanel, PanelWords, Ports, Strip};

    /// The stock titles, `<Unknown>` and "Unexplored"'s neighbours.
    fn panel_words() -> PanelWords {
        let mut list = vec![String::new(); 340];
        list[309] = "<Unknown>".to_owned();
        for (n, title) in [
            (337, "Current System:"),
            (338, "Selected System:"),
            (339, "Destination System:"),
            (340, "Hypergate Destination:"),
        ] {
            list[n - 1] = title.to_owned();
        }
        PanelWords::from_list(&list)
    }

    /// Every text drawn, the names' and the panel's at Geneva 9 among
    /// them, with its origin.
    fn every_text(list: &DrawList) -> Vec<(String, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    /// Where `text` is drawn, among [`every_text`].
    fn every_origin(list: &DrawList, text: &str) -> Option<Point> {
        every_text(list)
            .into_iter()
            .find_map(|(drawn, origin)| (drawn == text).then_some(origin))
    }

    /// An unexplored system's panel, titled "Selected System:".
    fn unexplored_panel() -> MapPanel {
        let words = panel_words();
        MapPanel {
            strip: Strip::Info {
                ports: Ports::Unknown,
                hazards: words.unknown.clone(),
                date: "Jun 23, 1177".to_owned(),
            },
            column: Column {
                title: words.selected.clone(),
                body: ColumnBody::Unknown,
            },
            words,
        }
    }

    #[test]
    fn the_viewer_lists_an_uninhabited_unlandable_stellar_of_an_unexplored_system() {
        // Beta's only stellar is a gas giant: uninhabited (`Flags` 0x20)
        // and not landable. The viewer is never shown what is explored,
        // and draws no session panel even when shown one.
        let mut galaxy = galaxy();
        galaxy.systems[1].stellars = vec![StellarEntry {
            id: StellarId(150),
            name: "Beta Giant".to_owned(),
        }];
        let mut map = GalaxyMap::new(&FakeCatalog::new(galaxy));
        map.show_panel(unexplored_panel());
        click_on(&mut map, 129);
        let texts: Vec<String> = every_text(&drawn(&map))
            .into_iter()
            .map(|(text, _)| text)
            .collect();
        assert!(texts.contains(&"Beta Giant".to_owned()), "{texts:?}");
        for hidden in [
            "<Unknown>",
            "Unexplored",
            "Current System:",
            "Selected System:",
            "Destination System:",
            "Hypergate Destination:",
        ] {
            assert!(!texts.contains(&hidden.to_owned()), "{hidden}: {texts:?}");
        }
    }

    #[test]
    fn an_in_flight_map_draws_the_panel_shown_from_the_dialogs_origin() {
        let mut map = course_map().with_metrics(Rc::new(MonoMetrics));
        assert!(map.panel().is_none());
        let list = drawn(&map);
        assert_eq!(every_origin(&list, "Selected System:"), None, "none yet");
        map.show_panel(unexplored_panel());
        assert_eq!(map.panel(), Some(&unexplored_panel()));
        let list = drawn(&map);
        // The column's title at (212 + 474 + 10, 128 + 8 + 12), its
        // baseline; the date measured against the strip's right edge,
        // 212 + 594 − 54 − 5.
        assert_eq!(
            every_origin(&list, "Selected System:"),
            Some(at(696.0, 139.0))
        );
        assert_eq!(every_origin(&list, "Jun 23, 1177"), Some(at(747.0, 591.0)));
        let course = GalaxyMap::course(&FakeCatalog::new(galaxy()));
        let mut unmeasured = course;
        unmeasured.show_panel(unexplored_panel());
        assert_eq!(
            every_origin(&drawn(&unmeasured), "Jun 23, 1177"),
            Some(at(801.0, 591.0)),
            "without metrics"
        );
    }

    #[test]
    fn a_measured_map_debugs_its_metrics_by_name() {
        let map = course_map().with_metrics(Rc::new(MonoMetrics));
        assert!(format!("{map:?}").contains("MapMetrics"));
    }

    // Space, Find and the Done and Find buttons.

    use crate::galaxy::buttons::MapButtons;
    use crate::galaxy::dialog::DIALOG_ORIGIN;
    use crate::galaxy::find::{FIND_ITEM, PROMPT};
    use crate::sound::{Sound, UiSound};
    use nova_sim::map_search::SearchOutcome;

    fn measured(map: GalaxyMap) -> GalaxyMap {
        map.with_metrics(Rc::new(MonoMetrics))
    }

    /// The course map with the player in Beta (129), measured.
    fn flown_map() -> GalaxyMap {
        let mut map = measured(course_map());
        map.show_course(SystemId(129), &[]);
        map
    }

    /// The hypergate map, from Alpha (128) offering 129 and 130, measured.
    fn measured_gate_map() -> GalaxyMap {
        measured(gate_map())
    }

    /// Types `text` as the platform sends it: each key, then its
    /// character.
    fn type_in(map: &mut GalaxyMap, text: &str) {
        for c in text.chars() {
            let key = if c == ' ' {
                Key::Space
            } else {
                Key::Char(c.to_ascii_lowercase())
            };
            map.input(&press(key));
            map.input(&Input::Text(c));
        }
    }

    /// Opens Find with S, types `text` and confirms it with Return.
    fn find(map: &mut GalaxyMap, text: &str) {
        map.input(&press(Key::Char('s')));
        map.input(&Input::Text('s'));
        assert!(map.finding(), "Find opened");
        type_in(map, text);
        map.input(&press(Key::Enter));
        assert!(!map.finding(), "Find closed");
    }

    /// Where system `id` is in the world.
    fn place(map: &GalaxyMap, id: i16) -> Point {
        map.model().position(SystemId(id)).expect("a system")
    }

    fn ui(sounds: &[UiSound]) -> Vec<Sound> {
        sounds.iter().copied().map(Sound::Ui).collect()
    }

    #[test]
    fn space_centres_the_course_map_on_the_current_system_and_selects_it() {
        let mut map = flown_map();
        click_on(&mut map, 130);
        map.input(&press(Key::Space));
        assert_eq!(map.transform().world, place(&map, 129));
        assert_eq!(map.selected(), Some(SystemId(129)));
        assert_eq!(map.take_press(), None, "nothing for the owner");
        assert_eq!(map.take_ui_sounds(), [UiSound::MapCentred]);
        assert_eq!(map.take_ui_sounds(), [], "taken");
    }

    #[test]
    fn a_space_repeat_or_release_does_nothing() {
        let mut map = flown_map();
        click_on(&mut map, 130);
        let view = map.transform();
        map.input(&held(Key::Space));
        map.input(&release(Key::Space));
        assert_eq!(
            (map.transform(), map.selected()),
            (view, Some(SystemId(130)))
        );
        assert_eq!(map.take_ui_sounds(), []);
    }

    #[test]
    fn space_on_the_hypergate_map_selects_the_current_system_and_keeps_the_pick() {
        let mut map = measured_gate_map();
        gate_click_on(&mut map, 130);
        map.input(&press(Key::Space));
        assert_eq!(map.selected(), Some(SystemId(128)));
        assert_eq!(map.transform().world, place(&map, 128));
        assert_eq!(map.take_ui_sounds(), [UiSound::MapCentred]);
        assert_eq!(map.take_gate_choice(), Some(SystemId(130)));
    }

    #[test]
    fn space_on_the_viewer_does_nothing() {
        let mut map = measured(map());
        click_on(&mut map, 130);
        let view = map.transform();
        map.input(&press(Key::Space));
        assert_eq!(
            (map.transform(), map.selected()),
            (view, Some(SystemId(130)))
        );
        assert_eq!(map.take_ui_sounds(), []);
    }

    #[test]
    fn f_and_s_open_find_on_the_in_flight_maps_but_not_their_repeats() {
        for (name, build) in [
            ("course", flown_map as fn() -> GalaxyMap),
            ("hypergate", measured_gate_map),
        ] {
            for key in ['f', 's', 'F', 'S'] {
                let mut map = build();
                map.input(&held(Key::Char(key)));
                assert!(!map.finding(), "{name}: a repeat of {key}");
                map.input(&press(Key::Char(key)));
                assert!(map.finding(), "{name}: {key}");
            }
            let mut shifted = build();
            shifted.input(&press(Key::Shift));
            shifted.input(&press(Key::Char('f')));
            assert!(shifted.finding(), "{name}: Shift-F");
        }
    }

    #[test]
    fn s_opens_find_on_the_viewer() {
        let mut map = measured(map());
        map.input(&press(Key::Char('s')));
        assert!(map.finding());
    }

    #[test]
    fn without_metrics_find_does_not_open() {
        let mut map = course_map();
        map.show_course(SystemId(129), &[]);
        map.input(&press(Key::Char('f')));
        assert!(!map.finding());
        let find = MapButtons::new(DIALOG_ORIGIN)
            .rect(MapButton::Find)
            .center();
        click(&mut map, find);
        assert!(!map.finding(), "the button");
    }

    #[test]
    fn find_opens_empty_dropping_the_key_that_opened_it() {
        let mut map = flown_map();
        map.input(&press(Key::Char('f')));
        map.input(&Input::Text('f'));
        let dialog = map.find_dialog().expect("open");
        assert_eq!(dialog.field().text(), "");
        let list = drawn(&map);
        assert!(
            texts(&list).contains(&PROMPT.to_owned()),
            "drawn over the map"
        );
        map.input(&Input::Text('S'));
        assert_eq!(map.find_dialog().expect("open").field().text(), "S");
    }

    #[test]
    fn find_takes_every_input_while_it_is_open() {
        let mut map = flown_map();
        click_on(&mut map, 130);
        map.input(&press(Key::Char('f')));
        let view = map.transform();
        for key in [
            Key::Left,
            Key::Up,
            Key::Char('='),
            Key::Tab,
            Key::Char('\\'),
        ] {
            map.input(&press(key));
        }
        assert_eq!(map.transform(), view, "no pan or zoom");
        assert_eq!(map.take_cycle(), None, "no cycle");
        type_in(&mut map, " m");
        assert_eq!(map.find_dialog().expect("open").field().text(), " m");
        let alpha = dot(&map, 128);
        click(&mut map, alpha);
        assert_eq!(map.selected(), Some(SystemId(130)), "no click on the map");
        assert_eq!(map.take_press(), None);
        assert_eq!(map.take_ui_sounds(), [], "no Space beep");
        map.input(&press(Key::Enter));
        assert!(!map.finding(), "Return confirms");
        assert!(!map.take_close(), "and closes only Find");
    }

    #[test]
    fn the_viewer_searches_every_system_itself() {
        let mut map = measured(map());
        find(&mut map, "gam");
        assert_eq!(map.selected(), Some(SystemId(130)));
        assert_eq!(map.transform().world, place(&map, 130));
        assert_eq!(map.take_ui_sounds(), [UiSound::MapSystemFound]);
        assert_eq!(map.take_find(), None, "nothing asked of an owner");
        find(&mut map, "alpha r");
        assert_eq!(map.selected(), Some(SystemId(131)));
        map.take_ui_sounds();
        let view = map.transform();
        find(&mut map, "zzz");
        assert_eq!(
            (map.selected(), map.transform()),
            (Some(SystemId(131)), view)
        );
        assert_eq!(map.take_ui_sounds(), [UiSound::MapSystemNotFound]);
    }

    #[test]
    fn an_empty_text_and_cancel_change_nothing_and_say_nothing() {
        for mut map in [measured(map()), flown_map()] {
            click_on(&mut map, 129);
            let view = map.transform();
            find(&mut map, "");
            map.input(&press(Key::Char('s')));
            type_in(&mut map, "gamma");
            let cancel = map
                .find_dialog()
                .expect("open")
                .dialog()
                .item_bounds(crate::galaxy::find::CANCEL_ITEM)
                .expect("Cancel")
                .center();
            click(&mut map, cancel);
            assert!(!map.finding(), "Cancel closes it");
            assert_eq!(
                (map.selected(), map.transform()),
                (Some(SystemId(129)), view)
            );
            assert_eq!(map.take_ui_sounds(), []);
            assert_eq!(map.take_find(), None);
        }
    }

    #[test]
    fn an_in_flight_map_asks_its_owner_to_search() {
        let mut map = flown_map();
        find(&mut map, "Sol ");
        assert_eq!(map.take_find(), Some("Sol ".to_owned()), "as typed");
        assert_eq!(map.take_find(), None, "taken");
        assert_eq!(map.take_ui_sounds(), [], "the owner's answer beeps");
        assert_eq!(map.selected(), None);
    }

    #[test]
    fn a_search_found_selects_and_centres_and_one_not_found_keeps_the_map() {
        let mut map = flown_map();
        map.show_search(SearchOutcome::Found(SystemId(130)));
        assert_eq!(map.selected(), Some(SystemId(130)));
        assert_eq!(map.take_press(), None, "nothing for the owner");
        assert_eq!(map.transform().world, place(&map, 130));
        assert_eq!(map.take_ui_sounds(), [UiSound::MapSystemFound]);
        click_on(&mut map, 128);
        let view = map.transform();
        map.show_search(SearchOutcome::NotFound);
        assert_eq!(
            (map.selected(), map.transform()),
            (Some(SystemId(128)), view)
        );
        assert_eq!(map.take_ui_sounds(), [UiSound::MapSystemNotFound]);
    }

    #[test]
    fn a_search_found_on_the_hypergate_map_keeps_the_pick() {
        let mut map = measured_gate_map();
        gate_click_on(&mut map, 129);
        map.show_search(SearchOutcome::Found(SystemId(131)));
        assert_eq!(map.selected(), Some(SystemId(131)));
        assert_eq!(map.take_gate_choice(), Some(SystemId(129)));
    }

    #[test]
    fn the_hypergate_pick_is_kept_apart_from_the_selection() {
        let mut map = gate_map();
        map.preselect(Some(SystemId(129)));
        assert_eq!(map.selected(), Some(SystemId(129)));
        assert_eq!(map.take_gate_choice(), None, "a preselect picks nothing");
        let mut map = gate_map();
        gate_click_on(&mut map, 129);
        map.offer_gates(ids(&[129, 130]));
        assert_eq!(map.selected(), None);
        assert_eq!(map.take_gate_choice(), None, "offering clears the pick");
        let mut map = gate_map();
        click_on(&mut map, 129);
        map.select_gate(Some(SystemId(130)));
        map.preselect(Some(SystemId(129)));
        assert_eq!(map.take_gate_choice(), Some(SystemId(130)), "the cycle's");
        assert_eq!(map.selected(), None);
        assert_eq!(map.take_gate_choice(), None, "taken");
    }

    #[test]
    fn closing_find_drops_the_dialog_and_its_text() {
        let mut map = flown_map();
        map.input(&press(Key::Char('f')));
        type_in(&mut map, "sol");
        map.close_find();
        assert!(!map.finding());
        assert_eq!(map.take_find(), None);
        map.input(&press(Key::Char('f')));
        assert_eq!(map.find_dialog().expect("open").field().text(), "");
    }

    #[test]
    fn the_map_reports_its_beeps_as_screen_sounds() {
        let mut map = flown_map();
        map.input(&press(Key::Space));
        map.show_search(SearchOutcome::NotFound);
        assert_eq!(
            map.take_sounds(),
            ui(&[UiSound::MapCentred, UiSound::MapSystemNotFound])
        );
        assert_eq!(map.take_sounds(), []);
        assert_eq!(map.take_ui_sounds(), []);
    }

    #[test]
    fn a_click_on_done_asks_to_close_and_on_find_opens_find_silently() {
        for build in [flown_map as fn() -> GalaxyMap, measured_gate_map] {
            let buttons = MapButtons::new(DIALOG_ORIGIN);
            let done = buttons.rect(MapButton::Done).center();
            let find = buttons.rect(MapButton::Find).center();
            let mut map = build();
            click(&mut map, done);
            assert!(map.take_close(), "Done, as Return");
            click(&mut map, find);
            assert!(map.finding());
            assert_eq!(map.take_sounds(), [], "silent");
            map.close_find();
            map.input(&button(MouseButton::Left, true, done));
            map.input(&button(MouseButton::Left, false, find));
            assert!(!map.take_close() && !map.finding(), "released off it");
            map.input(&button(MouseButton::Left, true, done));
            map.cancel_pointer();
            map.input(&button(MouseButton::Left, false, done));
            assert!(!map.take_close(), "cancelled");
        }
    }

    #[test]
    fn the_buttons_are_drawn_on_the_in_flight_maps_alone() {
        let labels = |map: &GalaxyMap| {
            let texts = texts(&drawn(map));
            ["Done", "Find"].map(|label| texts.contains(&label.to_owned()))
        };
        assert_eq!(labels(&flown_map()), [true, true]);
        assert_eq!(labels(&measured_gate_map()), [true, true]);
        assert_eq!(labels(&course_map()), [true, true], "unmeasured too");
        let mut viewer = measured(map());
        assert_eq!(labels(&viewer), [false, false]);
        let buttons = MapButtons::new(DIALOG_ORIGIN);
        click(&mut viewer, buttons.rect(MapButton::Done).center());
        click(&mut viewer, buttons.rect(MapButton::Find).center());
        assert!(!viewer.take_close() && !viewer.finding(), "nor hit");
    }

    #[test]
    fn find_is_laid_out_from_the_template_given_in_the_style_given() {
        let mut template = crate::galaxy::find::fallback();
        template.items[FIND_ITEM - 1].kind = crate::ui::dialog::ItemSpec::Button("Seek".to_owned());
        let style = ButtonStyle {
            size: 20.0,
            ..ButtonStyle::STOCK
        };
        let mut map = flown_map().with_find(Some(template), style);
        map.input(&press(Key::Char('f')));
        let list = drawn(&map);
        assert!(texts(&list).contains(&"Seek".to_owned()));
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, size, .. } if text == "Done" && *size == 20.0
            )),
            "the buttons in the style too"
        );
        let mut broken = crate::galaxy::find::fallback();
        broken.items.truncate(2);
        let mut map = flown_map().with_find(Some(broken), ButtonStyle::STOCK);
        map.input(&press(Key::Char('f')));
        assert!(map.finding(), "a broken template falls back");
        assert!(format!("{map:?}").contains("FindDialog"));
    }
}
