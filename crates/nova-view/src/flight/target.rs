//! The target on the HUD: the status bar's target panel and secondary
//! weapon line, as the original draws them (`_DrawStatusTarg` @0x4ace1
//! and `_DrawStatusWeap` @0x4c00a in the `EV Nova` executable).
//!
//! The panel fills the `ïntf`'s `TargArea`, and nothing is drawn when the
//! area is empty (top at or below its bottom, or left at or right of its
//! right, @0x4acf3-0x4ad09). Every line is set on its baseline, and a
//! centred line is placed as `_DrawCenteredText` places it
//! ([`Pen::centred_text`]):
//!
//! - With no target, "No Target" (`STR#` 2002 #349), dim, centred, 47
//!   below the top (@0x4b6a5).
//! - Otherwise, in the original's order: the target's name, bright,
//!   centred, 16 below the top (@0x4b6ab-0x4b765); its picture stretched
//!   into a 128 x 64 rectangle about the area's centre, each half of
//!   which is truncated (@0x4b76a-0x4b808), none without one; its
//!   subtitle, bright, centred, at `SubtitleSize`, 29 below the top
//!   (@0x4b80f-0x4b8de).
//! - Its status line, 5 in from the left and 6 above the bottom
//!   (@0x4b976-0x4b98d), as the session decides it
//!   ([`TargetStatus`]): "Shield: " or "Armor: ", dim (the original draws
//!   the space in the label's colour), then the percentage or "N/A"
//!   (#396), bright; or "Disabled" (#347), "Waiting" (#348), "No Shields"
//!   (#14) or "Shields Down" (#15), bright.
//! - Its label ([`Tag`]), dim, set against 7 in from the right on the
//!   same baseline (@0x4bd51-0x4bece): its government's `TargetCode`, or
//!   "Escort" (#168) or "Fighter" (#169).
//! - A hidden status line ([`TargetStatus::Hidden`]) leaves the bottom
//!   left empty and puts the government's `TargetCode`, dim, centred on
//!   the bottom line instead of the label; with no government nothing is
//!   drawn there (@0x4b9bb-0x4ba22, @0x4bfd3-0x4bfe1).
//! - The original also draws an "Sp" tag on a mission ship (and "Df " on
//!   a defence ship), but only in its cheats-only AI debug overlay, behind
//!   `_cheatsActive` and `_targKeyOnNow` (@0x4ae42-0x4b693); the engine
//!   leaves that overlay out, so no tag is drawn. It also leaves out the
//!   shareware nag ship's "Ambrosia" (@0x4b90b-0x4b92a).
//!
//! The secondary weapon's line fills the `WeapArea`, nothing for an empty
//! one (@0x4c022-0x4c038): the weapon's name, bright, centred, 12 below
//! the top, followed by " - " and the count the session gives
//! ([`FrameHud::secondary_line_count`](nova_sim::flight_frame::FrameHud::secondary_line_count));
//! or "No Secondary Weapon" (#350), dim (@0x4c105).
//!
//! Text is placed by its baseline, as the original places it: a line's
//! top is its size above its baseline. A line is centred, or set against
//! the right, by the [`TextMetrics`] given; without them, it starts at the
//! area's left (or right) edge.
//!
//! The brackets round the target in space are the
//! [`brackets`](super::brackets) module's; [`Standing`] is the frame
//! choice they take from the session.

use nova_sim::TargetStatus;
pub use nova_sim::brackets::Standing;

use super::catalog::StatusBarLayout;
use super::pen::Pen;
use crate::text::TextMetrics;
use crate::{DrawList, ImageKey, Point};

/// `STR#` 2002 #349.
pub const NO_TARGET: &str = "No Target";
/// `STR#` 2002 #350.
pub const NO_SECONDARY: &str = "No Secondary Weapon";
/// `STR#` 2002 #347.
pub const DISABLED: &str = "Disabled";
/// `STR#` 2002 #348.
pub const WAITING: &str = "Waiting";
/// `STR#` 2002 #13.
pub const SHIELD: &str = "Shield:";
/// `STR#` 2002 #16.
pub const ARMOR: &str = "Armor:";
/// `STR#` 2002 #396.
pub const NOT_AVAILABLE: &str = "N/A";
/// `STR#` 2002 #14.
pub const NO_SHIELDS: &str = "No Shields";
/// `STR#` 2002 #15.
pub const SHIELDS_DOWN: &str = "Shields Down";
/// `STR#` 2002 #168.
pub const ESCORT: &str = "Escort";
/// `STR#` 2002 #169.
pub const FIGHTER: &str = "Fighter";
/// The target's picture's width.
pub const PICTURE_WIDTH: f32 = 128.0;
/// The target's picture's height.
pub const PICTURE_HEIGHT: f32 = 64.0;
/// The name's baseline, below the panel's top.
pub const NAME_BASELINE: f32 = 16.0;
/// The subtitle's baseline, below the panel's top.
pub const SUBTITLE_BASELINE: f32 = 29.0;
/// "No Target"'s baseline, below the panel's top.
pub const NO_TARGET_BASELINE: f32 = 47.0;
/// The status line's distance in from the panel's left.
pub const STATUS_INSET: f32 = 5.0;
/// The status line's and the label's baseline, above the panel's bottom.
pub const BOTTOM_BASELINE: f32 = 6.0;
/// The label's distance in from the panel's right.
pub const CODE_INSET: f32 = 7.0;
/// The secondary weapon's baseline, below its area's top.
pub const WEAPON_BASELINE: f32 = 12.0;
/// The label against the target panel's bottom right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tag {
    /// Its government's `TargetCode`.
    Code(String),
    /// "Escort".
    Escort,
    /// "Fighter".
    Fighter,
}

/// What the target panel shows of the target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetPanel {
    /// Its name: a marked mission ship's drawn name (`_DrawStatusTarg`
    /// @0x4b6c8), else a person's own, else its ship type's.
    pub name: String,
    /// Its subtitle: while it is a marked mission ship, the mission's
    /// drawn subtitle unless that is empty, never a person's; otherwise a
    /// person's; else its ship type's (@0x4b80f-0x4b8de).
    pub subtitle: String,
    /// Its ship type's picture, if any.
    pub picture: Option<i16>,
    /// Its status line.
    pub status: TargetStatus,
    /// Its label, if any.
    pub tag: Option<Tag>,
}

/// Draws the target panel of a bar laid out by `layout` at `origin`:
/// `target`, or "No Target" (see the module docs).
pub fn draw_target_panel(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    target: Option<&TargetPanel>,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.targ.offset(origin);
    let (left, top, right, bottom) = (area.min.x, area.min.y, area.max.x, area.max.y);
    if top >= bottom || left >= right {
        return;
    }
    let mut pen = Pen {
        list,
        font: layout.font,
        metrics,
    };
    let (size, bright, dim) = (layout.font_size, layout.bright_text, layout.dim_text);
    let Some(target) = target else {
        pen.centred_text(NO_TARGET, left, right, top + NO_TARGET_BASELINE, size, dim);
        return;
    };
    pen.centred_text(&target.name, left, right, top + NAME_BASELINE, size, bright);
    if let Some(picture) = target.picture {
        let (x, y) = (
            f32::midpoint(left, right).trunc(),
            f32::midpoint(top, bottom).trunc(),
        );
        pen.list.stretched_picture(
            ImageKey::picture(picture),
            Point::new(x - PICTURE_WIDTH / 2.0, y - PICTURE_HEIGHT / 2.0),
            PICTURE_WIDTH,
            PICTURE_HEIGHT,
        );
    }
    pen.centred_text(
        &target.subtitle,
        left,
        right,
        top + SUBTITLE_BASELINE,
        layout.subtitle_size,
        bright,
    );
    let baseline = bottom - BOTTOM_BASELINE;
    let (label, value) = match target.status {
        TargetStatus::Hidden => {
            if let Some(Tag::Code(code)) = &target.tag {
                pen.centred_text(code, left, right, baseline, size, dim);
            }
            return;
        }
        TargetStatus::Shield(percent) => (Some(SHIELD), format!("{percent}%")),
        TargetStatus::Armor(percent) => (Some(ARMOR), format!("{percent}%")),
        TargetStatus::ArmorNa => (Some(ARMOR), NOT_AVAILABLE.to_owned()),
        TargetStatus::Disabled => (None, DISABLED.to_owned()),
        TargetStatus::Waiting => (None, WAITING.to_owned()),
        TargetStatus::NoShields => (None, NO_SHIELDS.to_owned()),
        TargetStatus::ShieldsDown => (None, SHIELDS_DOWN.to_owned()),
    };
    let x = left + STATUS_INSET;
    let x = match label {
        Some(label) => pen.at(&format!("{label} "), x, baseline, size, dim),
        None => x,
    };
    pen.at(&value, x, baseline, size, bright);
    if let Some(tag) = &target.tag {
        let text = match tag {
            Tag::Code(code) => code.as_str(),
            Tag::Escort => ESCORT,
            Tag::Fighter => FIGHTER,
        };
        let width = pen.width(text, size);
        pen.at(text, right - CODE_INSET - width, baseline, size, dim);
    }
}

/// The secondary weapon's line: `name`, and ` - ` and `count` when it
/// has one to show.
#[must_use]
pub fn secondary_text(name: &str, count: Option<u32>) -> String {
    match count {
        Some(count) => format!("{name} - {count}"),
        None => name.to_owned(),
    }
}

/// Draws the secondary weapon's line of a bar laid out by `layout` at
/// `origin`: `weapon`'s line, or "No Secondary Weapon"; nothing for an
/// empty `WeapArea`.
pub fn draw_secondary(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    weapon: Option<&str>,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.weap.offset(origin);
    if area.min.y >= area.max.y || area.min.x >= area.max.x {
        return;
    }
    let mut pen = Pen {
        list,
        font: layout.font,
        metrics,
    };
    let (text, color) = match weapon {
        Some(weapon) => (weapon, layout.bright_text),
        None => (NO_SECONDARY, layout.dim_text),
    };
    let baseline = area.min.y + WEAPON_BASELINE;
    pen.centred_text(
        text,
        area.min.x,
        area.max.x,
        baseline,
        layout.font_size,
        color,
    );
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Bounds;
    use crate::text::fixture::MonoMetrics;
    use crate::{Color, DrawCommand, Font};

    const BRIGHT: Color = Color::rgba(250, 250, 250, 255);
    const DIM: Color = Color::rgba(100, 100, 100, 255);

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: Point::new(left, top),
            max: Point::new(right, bottom),
        }
    }

    /// Stock `ïntf` 128's areas, in Geneva at 12 with subtitles at 10.
    fn layout() -> StatusBarLayout {
        let none = rect(0.0, 0.0, 0.0, 0.0);
        StatusBarLayout {
            radar: none,
            shield: none,
            armor: none,
            fuel: none,
            nav: none,
            weap: rect(8.0, 300.0, 184.0, 315.0),
            targ: rect(8.0, 330.0, 184.0, 442.0),
            cargo: none,
            bright_text: BRIGHT,
            dim_text: DIM,
            bright_radar: Color::WHITE,
            dim_radar: Color::WHITE,
            shield_color: Color::WHITE,
            armor_color: Color::WHITE,
            fuel_full: Color::WHITE,
            fuel_partial: Color::WHITE,
            font: Font::Geneva,
            font_size: 12.0,
            subtitle_size: 10.0,
            status_bkgnd: 700,
        }
    }

    /// The bar's origin: against the right edge, at the top.
    const ORIGIN: Point = Point::new(830.0, 0.0);

    fn text(text: &str, x: f32, top: f32, size: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(x, top),
            size,
            wrap_width: None,
            color,
        }
    }

    fn panel_in(
        layout: &StatusBarLayout,
        target: Option<&TargetPanel>,
        metrics: Option<&dyn TextMetrics>,
    ) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_target_panel(&mut list, layout, ORIGIN, target, metrics);
        list.iter().cloned().collect()
    }

    fn panel(target: Option<&TargetPanel>, metrics: Option<&dyn TextMetrics>) -> Vec<DrawCommand> {
        panel_in(&layout(), target, metrics)
    }

    /// The panel runs from x 838 to 1014: 176 across, centred on 926.
    #[test]
    fn with_no_target_the_panel_says_so_dim_and_centred() {
        // "No Target" is 9 characters of 6: 54 wide; baseline 377.
        assert_eq!(
            panel(None, Some(&MonoMetrics)),
            [text(NO_TARGET, 899.0, 365.0, 12.0, DIM)]
        );
        assert_eq!(
            panel(None, None),
            [text(NO_TARGET, 838.0, 365.0, 12.0, DIM)],
            "without metrics, at the left edge"
        );
        assert_eq!(NO_TARGET_BASELINE, 47.0);
    }

    /// A shuttle of the Federation, shields at 73 %.
    fn shuttle() -> TargetPanel {
        TargetPanel {
            name: "Shuttle".to_owned(),
            subtitle: "Light Transport".to_owned(),
            picture: Some(3000),
            status: TargetStatus::Shield(73),
            tag: Some(Tag::Code("Fed.".to_owned())),
        }
    }

    #[test]
    fn a_target_shows_its_name_picture_subtitle_status_and_code_in_order() {
        assert_eq!(
            panel(Some(&shuttle()), Some(&MonoMetrics)),
            [
                // "Shuttle": 7 x 6 = 42 wide; baseline 346.
                text("Shuttle", 905.0, 334.0, 12.0, BRIGHT),
                DrawCommand::StretchedPicture {
                    image: ImageKey::picture(3000),
                    top_left: Point::new(862.0, 354.0),
                    width: 128.0,
                    height: 64.0,
                },
                // "Light Transport": 15 x 5 = 75 wide, half 37.5 → 37;
                // baseline 359.
                text("Light Transport", 889.0, 349.0, 10.0, BRIGHT),
                // Baseline 436: "Shield: " is 8 x 6 = 48 wide.
                text("Shield: ", 843.0, 424.0, 12.0, DIM),
                text("73%", 891.0, 424.0, 12.0, BRIGHT),
                // "Fed." is 4 x 6 = 24 wide, against 1007.
                text("Fed.", 983.0, 424.0, 12.0, DIM),
            ]
        );
        assert_eq!((PICTURE_WIDTH, PICTURE_HEIGHT), (128.0, 64.0));
        assert_eq!(
            (NAME_BASELINE, SUBTITLE_BASELINE, BOTTOM_BASELINE),
            (16.0, 29.0, 6.0)
        );
        assert_eq!((STATUS_INSET, CODE_INSET), (5.0, 7.0));
    }

    #[test]
    fn the_picture_sits_on_the_truncated_centre_of_an_odd_area() {
        let odd = StatusBarLayout {
            targ: rect(8.0, 331.0, 185.0, 442.0),
            ..layout()
        };
        let commands = panel_in(&odd, Some(&shuttle()), Some(&MonoMetrics));
        // (838 + 1015) / 2 = 926.5 → 926; (331 + 442) / 2 = 386.5 → 386.
        assert_eq!(
            commands[1],
            DrawCommand::StretchedPicture {
                image: ImageKey::picture(3000),
                top_left: Point::new(862.0, 354.0),
                width: 128.0,
                height: 64.0,
            }
        );
    }

    #[test]
    fn a_target_without_a_picture_or_label_shows_neither() {
        let target = TargetPanel {
            picture: None,
            tag: None,
            ..shuttle()
        };
        let commands = panel(Some(&target), Some(&MonoMetrics));
        assert_eq!(commands.len(), 4, "{commands:?}");
        assert!(
            commands
                .iter()
                .all(|c| matches!(c, DrawCommand::Text { .. })),
            "{commands:?}"
        );
    }

    #[test]
    fn an_empty_target_area_draws_nothing() {
        for targ in [
            rect(8.0, 330.0, 184.0, 330.0),
            rect(8.0, 330.0, 184.0, 300.0),
            rect(8.0, 330.0, 8.0, 442.0),
            rect(184.0, 330.0, 8.0, 442.0),
        ] {
            let empty = StatusBarLayout { targ, ..layout() };
            assert_eq!(panel_in(&empty, Some(&shuttle()), None), [], "{targ:?}");
            assert_eq!(panel_in(&empty, None, None), [], "{targ:?}");
        }
        let thin = StatusBarLayout {
            targ: rect(8.0, 330.0, 9.0, 331.0),
            ..layout()
        };
        assert_eq!(panel_in(&thin, None, None).len(), 1, "a pixel is enough");
    }

    /// The bottom line of a target with `status` and `tag`, as drawn:
    /// each text, where it starts and its colour.
    fn bottom(status: TargetStatus, tag: Option<Tag>) -> Vec<(String, f32, Color)> {
        let target = TargetPanel {
            picture: None,
            status,
            tag,
            ..shuttle()
        };
        panel(Some(&target), Some(&MonoMetrics))
            .into_iter()
            .skip(2)
            .map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => {
                    assert_eq!(origin.y, 424.0, "{text}");
                    (text, origin.x, color)
                }
                other => panic!("{other:?}"),
            })
            .collect()
    }

    fn line(parts: &[(&str, f32, Color)]) -> Vec<(String, f32, Color)> {
        parts
            .iter()
            .map(|(t, x, c)| ((*t).to_owned(), *x, *c))
            .collect()
    }

    #[test]
    fn the_shield_shows_dim_label_and_bright_percentage() {
        for (percent, value) in [(0, "0%"), (73, "73%"), (100, "100%")] {
            assert_eq!(
                bottom(TargetStatus::Shield(percent), None),
                line(&[("Shield: ", 843.0, DIM), (value, 891.0, BRIGHT)])
            );
        }
    }

    #[test]
    fn the_armour_shows_dim_label_and_bright_percentage_or_n_a() {
        // "Armor: " is 7 x 6 = 42 wide.
        for (percent, value) in [(0, "0%"), (40, "40%"), (100, "100%")] {
            assert_eq!(
                bottom(TargetStatus::Armor(percent), None),
                line(&[("Armor: ", 843.0, DIM), (value, 885.0, BRIGHT)])
            );
        }
        assert_eq!(
            bottom(TargetStatus::ArmorNa, None),
            line(&[("Armor: ", 843.0, DIM), (NOT_AVAILABLE, 885.0, BRIGHT)])
        );
    }

    #[test]
    fn a_word_status_is_bright_at_the_left() {
        for (status, word) in [
            (TargetStatus::Disabled, DISABLED),
            (TargetStatus::Waiting, WAITING),
            (TargetStatus::NoShields, NO_SHIELDS),
            (TargetStatus::ShieldsDown, SHIELDS_DOWN),
        ] {
            assert_eq!(
                bottom(status, None),
                line(&[(word, 843.0, BRIGHT)]),
                "{status:?}"
            );
        }
        assert_eq!(
            [DISABLED, WAITING, NO_SHIELDS, SHIELDS_DOWN, NOT_AVAILABLE],
            ["Disabled", "Waiting", "No Shields", "Shields Down", "N/A"]
        );
    }

    #[test]
    fn the_label_is_dim_against_seven_in_from_the_right() {
        let disabled = (DISABLED, 843.0, BRIGHT);
        // 1014 - 7 = 1007, less each label's width.
        assert_eq!(
            bottom(TargetStatus::Disabled, Some(Tag::Code("Fed.".to_owned()))),
            line(&[disabled, ("Fed.", 983.0, DIM)])
        );
        assert_eq!(
            bottom(TargetStatus::Disabled, Some(Tag::Escort)),
            line(&[disabled, (ESCORT, 971.0, DIM)])
        );
        assert_eq!(
            bottom(TargetStatus::Disabled, Some(Tag::Fighter)),
            line(&[disabled, (FIGHTER, 965.0, DIM)])
        );
        assert_eq!((ESCORT, FIGHTER), ("Escort", "Fighter"));
    }

    #[test]
    fn a_hidden_line_centres_the_government_code_instead() {
        // (838 + 1014) / 2 = 926, less half of 24.
        assert_eq!(
            bottom(TargetStatus::Hidden, Some(Tag::Code("Fed.".to_owned()))),
            line(&[("Fed.", 914.0, DIM)])
        );
        assert_eq!(bottom(TargetStatus::Hidden, Some(Tag::Escort)), []);
        assert_eq!(bottom(TargetStatus::Hidden, Some(Tag::Fighter)), []);
        assert_eq!(bottom(TargetStatus::Hidden, None), []);
    }

    #[test]
    fn without_metrics_the_lines_start_at_the_panels_edges() {
        let commands = panel(Some(&shuttle()), None);
        let origins: Vec<Point> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { origin, .. } => Some(*origin),
                _ => None,
            })
            .collect();
        assert_eq!(
            origins,
            [
                Point::new(838.0, 334.0),
                Point::new(838.0, 349.0),
                Point::new(843.0, 424.0),
                Point::new(843.0, 424.0),
                Point::new(1007.0, 424.0),
            ]
        );
    }

    #[test]
    fn the_secondary_line_is_its_name_and_its_count_if_any() {
        assert_eq!(secondary_text("Light Blaster", None), "Light Blaster");
        assert_eq!(secondary_text("Rocket", Some(12)), "Rocket - 12");
        assert_eq!(secondary_text("Rocket", Some(0)), "Rocket - 0");
    }

    fn secondary_in(
        layout: &StatusBarLayout,
        weapon: Option<&str>,
        metrics: Option<&dyn TextMetrics>,
    ) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_secondary(&mut list, layout, ORIGIN, weapon, metrics);
        list.iter().cloned().collect()
    }

    fn secondary(weapon: Option<&str>, metrics: Option<&dyn TextMetrics>) -> Vec<DrawCommand> {
        secondary_in(&layout(), weapon, metrics)
    }

    #[test]
    fn the_secondary_line_is_centred_in_the_weapon_area() {
        // "Rocket - 12": 11 x 6 = 66 wide; baseline 312.
        assert_eq!(
            secondary(Some("Rocket - 12"), Some(&MonoMetrics)),
            [text("Rocket - 12", 893.0, 300.0, 12.0, BRIGHT)]
        );
        // "Mine": 4 x 6 = 24 wide.
        assert_eq!(
            secondary(Some("Mine"), Some(&MonoMetrics)),
            [text("Mine", 914.0, 300.0, 12.0, BRIGHT)]
        );
        // 19 x 6 = 114 wide.
        assert_eq!(
            secondary(None, Some(&MonoMetrics)),
            [text(NO_SECONDARY, 869.0, 300.0, 12.0, DIM)]
        );
        assert_eq!(
            secondary(None, None),
            [text(NO_SECONDARY, 838.0, 300.0, 12.0, DIM)]
        );
        assert_eq!(WEAPON_BASELINE, 12.0);
    }

    #[test]
    fn the_secondary_line_is_centred_as_draw_centered_text_does() {
        let odd = StatusBarLayout {
            weap: rect(8.0, 300.0, 185.0, 315.0),
            ..layout()
        };
        // (838 + 1015) / 2 = 926.5 → 926; "abc" 18 wide.
        assert_eq!(
            secondary_in(&odd, Some("abc"), Some(&MonoMetrics)),
            [text("abc", 917.0, 300.0, 12.0, BRIGHT)]
        );
    }

    #[test]
    fn an_empty_weapon_area_draws_nothing() {
        for weap in [
            rect(8.0, 300.0, 184.0, 300.0),
            rect(8.0, 300.0, 8.0, 315.0),
            rect(184.0, 315.0, 8.0, 300.0),
        ] {
            let empty = StatusBarLayout { weap, ..layout() };
            assert_eq!(secondary_in(&empty, None, None), [], "{weap:?}");
            assert_eq!(secondary_in(&empty, Some("Mine"), None), [], "{weap:?}");
        }
        let thin = StatusBarLayout {
            weap: rect(8.0, 300.0, 9.0, 301.0),
            ..layout()
        };
        assert_eq!(secondary_in(&thin, None, None).len(), 1);
    }
}
