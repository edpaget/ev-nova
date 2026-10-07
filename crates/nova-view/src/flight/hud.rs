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
//! Which `ïntf`: a pilot with no government shows `ïntf` 128, the
//! "Default status bar"; one who belongs to a government shows that
//! `gövt`'s `Interface`. Either way an ID below 128 means 128
//! ([`interface_id`]), and the same rule picks the background `PICT` from
//! the bar's `StatusBkgnd`.
//!
//! Every `ïntf` area is relative to the bar's top-left corner. The bar is
//! as wide as its background picture (194 pixels in every stock bar) and
//! sits against the right edge of the screen, at the top.

use nova_sim::hyperspace::max_jumps;
use nova_sim::{JumpReadiness, Reserves};

use super::catalog::{GovtId, StatusBarLayout, StatusBars};
use crate::geometry::{Bounds, Point};
use crate::system::camera::VIEW_SIZE;
use crate::text::LINE_HEIGHT;
use crate::{Color, DrawList, Font, ImageKey};

/// The `ïntf` shown by a pilot with no government, and the ID any ID
/// below it means.
pub const DEFAULT_INTERFACE: i16 = 128;
/// The bar's width when its background cannot be read: the stock bars'.
pub const STATUS_BAR_WIDTH: f32 = 194.0;
/// Radar units per world unit: the stock radar, 176 pixels across, shows
/// 1408 pixels each way from the player, which holds nearly every stock
/// system's stellars.
pub const RADAR_SCALE: f32 = 1.0 / 16.0;
/// The side of a radar dot.
pub const RADAR_DOT_SIZE: f32 = 2.0;
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

/// The status bar for a pilot of `government`, read from `catalog`, or why
/// it cannot be read.
pub fn choose_status_bar(
    catalog: &impl StatusBars,
    government: Option<GovtId>,
) -> Result<StatusBar, String> {
    let id = match government {
        None => DEFAULT_INTERFACE,
        Some(govt) => interface_id(catalog.government_interface(govt)?),
    };
    let layout = catalog.status_bar(id)?;
    let picture = interface_id(layout.status_bkgnd);
    let background = catalog
        .picture_size(picture)
        .map(|(width, height)| Background {
            id: picture,
            width: width as f32,
            height: height as f32,
        });
    Ok(StatusBar { layout, background })
}

/// Where the bar's top-left corner goes: against the right edge of the
/// screen, as wide as its background, or [`STATUS_BAR_WIDTH`] without one.
#[must_use]
pub fn bar_origin(bar: &StatusBar) -> Point {
    let width = bar.background.map_or(STATUS_BAR_WIDTH, |b| b.width);
    Point::new(VIEW_SIZE.0 - width, 0.0)
}

/// Where the radar shows a stellar at `stellar` to a player at `player`
/// (both in world units): the radar's centre plus the offset scaled by
/// [`RADAR_SCALE`], or `None` when that falls outside `radar`.
#[must_use]
pub fn radar_point(radar: Bounds, player: Point, stellar: Point) -> Option<Point> {
    let center = radar.center();
    let at = Point::new(
        (stellar.x - player.x).mul_add(RADAR_SCALE, center.x),
        (stellar.y - player.y).mul_add(RADAR_SCALE, center.y),
    );
    radar.contains(at).then_some(at)
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
    /// Where the player's ship is drawn, in world units.
    pub position: Point,
    /// Where the system's stellars are, in world units.
    pub stellars: &'a [Point],
    /// The player's shield, armour and fuel.
    pub reserves: Reserves,
    /// What the nav area shows.
    pub nav: NavDisplay,
    /// Today's date, as it is displayed.
    pub date: &'a str,
}

/// Draws `bar` showing `state`: its background, a radar dot for each
/// stellar within range, then the shield, armour and fuel bars.
pub fn draw(list: &mut DrawList, bar: &StatusBar, state: &HudState) {
    let origin = bar_origin(bar);
    let layout = &bar.layout;
    if let Some(background) = bar.background {
        list.picture(ImageKey::picture(background.id), origin);
    }
    let radar = layout.radar.offset(origin);
    for &stellar in state.stellars {
        if let Some(at) = radar_point(radar, state.position, stellar) {
            list.dot(at, RADAR_DOT_SIZE, layout.bright_radar);
        }
    }
    let reserves = &state.reserves;
    let shield = layout.shield.offset(origin);
    let armor = layout.armor.offset(origin);
    let fuel = layout.fuel.offset(origin);
    fill(
        list,
        shield,
        0.0,
        reserves.shield.fraction(),
        layout.shield_color,
    );
    fill(
        list,
        armor,
        0.0,
        reserves.armor.fraction(),
        layout.armor_color,
    );
    let whole = whole_jumps(reserves);
    fill(list, fuel, 0.0, whole, layout.fuel_full);
    fill(
        list,
        fuel,
        whole,
        reserves.fuel.fraction(),
        layout.fuel_partial,
    );
    draw_nav(list, layout, origin, &state.nav);
    draw_date(list, layout, origin, state.date);
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

/// The fraction of the fuel bar the whole jumps' worth of fuel fills.
fn whole_jumps(reserves: &Reserves) -> f32 {
    let fuel = reserves.fuel;
    if fuel.max > 0.0 {
        let held = fuel.now.clamp(0.0, fuel.max);
        (max_jumps(held) as f32 * FUEL_PER_JUMP / fuel.max).min(1.0)
    } else {
        0.0
    }
}

/// Fills `area` from `from` to `to` of its width, as one horizontal line as
/// thick as `area` is tall; nothing when that is empty, since a line of no
/// length still draws a square.
fn fill(list: &mut DrawList, area: Bounds, from: f32, to: f32, color: Color) {
    if to <= from {
        return;
    }
    let y = area.center().y;
    let width = area.width();
    list.line(
        Point::new(from.mul_add(width, area.min.x), y),
        Point::new(to.mul_add(width, area.min.x), y),
        area.height(),
        color,
    );
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
    const TEXT: Color = Color::rgba(250, 250, 250, 255);

    /// Stock `ïntf` 128's areas, with its background `bkgnd`.
    fn layout(bkgnd: i16) -> StatusBarLayout {
        StatusBarLayout {
            radar: rect(8.0, 8.0, 184.0, 184.0),
            shield: rect(35.0, 199.0, 184.0, 206.0),
            armor: rect(35.0, 216.0, 184.0, 223.0),
            fuel: rect(35.0, 234.0, 184.0, 241.0),
            nav: rect(8.0, 254.0, 184.0, 286.0),
            cargo: rect(8.0, 458.0, 184.0, 552.0),
            bright_text: TEXT,
            dim_text: Color::DIM,
            bright_radar: RADAR,
            dim_radar: Color::BLACK,
            shield_color: SHIELD,
            armor_color: ARMOR,
            fuel_full: FUEL_FULL,
            fuel_partial: FUEL_PARTIAL,
            font: Font::Charcoal,
            font_size: 11.0,
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
        fn government_interface(&self, id: GovtId) -> Result<i16, String> {
            self.asked.borrow_mut().push(format!("gövt {}", id.0));
            self.governments
                .iter()
                .find(|(govt, _)| *govt == id.0)
                .map_or_else(|| Err(format!("no gövt {}", id.0)), |(_, i)| i.clone())
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

    /// `ïntf` 128 (background 700) and 130 (background 702); govt 128
    /// shows 130 and govt 129 shows 0; both pictures are 194 x 767.
    fn bars() -> FakeBars {
        FakeBars {
            governments: vec![(128, Ok(130)), (129, Ok(0))],
            bars: vec![(128, Ok(layout(700))), (130, Ok(layout(702)))],
            pictures: vec![(700, (194, 767)), (702, (194, 767))],
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
        let catalog = bars();
        let shown = choose_status_bar(&catalog, Some(GovtId(129))).expect("reads");
        assert_eq!(shown.layout, layout(700));
        assert_eq!(
            *catalog.asked.borrow(),
            ["gövt 129", "ïntf 128", "PICT 700"]
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

    const RADAR_AREA: Bounds = Bounds {
        min: Point::new(8.0, 8.0),
        max: Point::new(184.0, 184.0),
    };

    #[test]
    fn a_stellar_under_the_player_is_at_the_radars_centre() {
        assert_eq!(
            radar_point(RADAR_AREA, at(300.0, -200.0), at(300.0, -200.0)),
            Some(at(96.0, 96.0))
        );
    }

    #[test]
    fn offsets_scale_by_the_radar_scale_with_y_down() {
        assert_eq!(RADAR_SCALE, 0.0625);
        assert_eq!(
            radar_point(RADAR_AREA, at(0.0, 0.0), at(500.0, -250.0)),
            Some(at(127.25, 80.375))
        );
        assert_eq!(
            radar_point(RADAR_AREA, at(0.0, 0.0), at(-1000.0, 1000.0)),
            Some(at(33.5, 158.5))
        );
    }

    #[test]
    fn a_stellar_beyond_the_radar_is_not_shown() {
        // 1408 is the edge, still in; just past it, on each side, is out.
        assert_eq!(
            radar_point(RADAR_AREA, at(0.0, 0.0), at(1408.0, -1408.0)),
            Some(at(184.0, 8.0))
        );
        assert_eq!(
            radar_point(RADAR_AREA, at(0.0, 0.0), at(-1408.0, 1408.0)),
            Some(at(8.0, 184.0))
        );
        for stellar in [
            at(1424.0, 0.0),
            at(-1424.0, 0.0),
            at(0.0, 1424.0),
            at(0.0, -1424.0),
        ] {
            assert_eq!(radar_point(RADAR_AREA, at(0.0, 0.0), stellar), None);
        }
    }

    #[test]
    fn moving_the_player_moves_the_dots_the_other_way() {
        let earth = at(0.0, -600.0);
        let still = radar_point(RADAR_AREA, at(0.0, 0.0), earth).expect("in range");
        let up = radar_point(RADAR_AREA, at(0.0, -100.0), earth).expect("in range");
        let right = radar_point(RADAR_AREA, at(100.0, 0.0), earth).expect("in range");
        assert_eq!(still, at(96.0, 58.5));
        assert_eq!(up, at(96.0, 64.75), "the player went up, the dot down");
        assert_eq!(
            right,
            at(89.75, 58.5),
            "the player went right, the dot left"
        );
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

    fn state(reserves: Reserves) -> HudState<'static> {
        HudState {
            position: at(0.0, 0.0),
            stellars: &[],
            reserves,
            nav: NavDisplay::None,
            date: DATE,
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
    fn each_bar_fills_as_far_as_its_gauge() {
        let list = drawn(&stock(), &state(reserves(15.0, 15.0, 200.0)));
        assert_eq!(
            lines(&list),
            [
                bar_line(199.0, 0.0, 74.5, SHIELD),
                bar_line(216.0, 0.0, 37.25, ARMOR),
                bar_line(234.0, 0.0, 149.0 * 2.0 / 3.0, FUEL_FULL),
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
    }

    #[test]
    fn fuel_beyond_whole_jumps_is_drawn_in_the_partial_colour() {
        // 250 of 300: two jumps (2/3 of the bar), then half a jump (1/6).
        let list = drawn(&stock(), &state(reserves(0.0, 0.0, 250.0)));
        let third = 149.0 / 3.0;
        assert_eq!(
            lines(&list),
            [
                bar_line(234.0, 0.0, 2.0 * third, FUEL_FULL),
                bar_line(234.0, 2.0 * third, 149.0 * 250.0 / 300.0, FUEL_PARTIAL),
            ]
        );
        // Under one jump is all partial.
        let list = drawn(&stock(), &state(reserves(0.0, 0.0, 50.0)));
        assert_eq!(
            lines(&list),
            [bar_line(234.0, 0.0, 149.0 / 6.0, FUEL_PARTIAL)]
        );
    }

    #[test]
    fn over_full_or_odd_fuel_stays_within_the_bar() {
        let over = drawn(&stock(), &state(reserves(0.0, 0.0, 450.0)));
        assert_eq!(lines(&over), [bar_line(234.0, 0.0, 149.0, FUEL_FULL)]);
        // A capacity that is not whole jumps: 250 of 250 is two jumps, then
        // a half.
        let odd = Reserves {
            fuel: Gauge::full(250.0),
            ..Reserves::default()
        };
        assert_eq!(
            lines(&drawn(&stock(), &state(odd))),
            [
                bar_line(234.0, 0.0, 149.0 * 0.8, FUEL_FULL),
                bar_line(234.0, 149.0 * 0.8, 149.0, FUEL_PARTIAL),
            ]
        );
        let negative = drawn(&stock(), &state(reserves(0.0, 0.0, -50.0)));
        assert_eq!(lines(&negative), []);
    }

    #[test]
    fn it_draws_the_background_then_the_radar_then_the_bars() {
        let stellars = [at(0.0, -600.0), at(300.0, -200.0), at(5000.0, 0.0)];
        let hud = HudState {
            position: at(0.0, 0.0),
            stellars: &stellars,
            reserves: reserves(30.0, 60.0, 300.0),
            nav: NavDisplay::Stellar("Earth".to_owned()),
            date: DATE,
        };
        let list = drawn(&stock(), &hud);
        let mut expected = DrawList::new();
        expected.picture(ImageKey::picture(700), at(830.0, 0.0));
        expected.dot(at(926.0, 58.5), RADAR_DOT_SIZE, RADAR);
        expected.dot(at(944.75, 83.5), RADAR_DOT_SIZE, RADAR);
        expected.line(at(865.0, 202.5), at(1014.0, 202.5), 7.0, SHIELD);
        expected.line(at(865.0, 219.5), at(1014.0, 219.5), 7.0, ARMOR);
        expected.line(at(865.0, 237.5), at(1014.0, 237.5), 7.0, FUEL_FULL);
        expected.push(nav_text(NAV_STELLAR, 0, Color::DIM));
        expected.push(nav_text("Earth", 1, TEXT));
        expected.push(date_text(DATE));
        assert_eq!(list, expected);
        assert_eq!(RADAR_DOT_SIZE, 2.0);
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
