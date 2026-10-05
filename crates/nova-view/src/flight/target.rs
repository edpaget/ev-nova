//! The target on the HUD and in space: the status bar's target panel and
//! secondary weapon line, and the brackets round the target, as the
//! original draws them (`_DrawStatusTarg` @0x4ace1, `_DrawStatusWeap`
//! @0x4c00a and `_HandleWeapTargetSprites` @0x2ff1a in the `EV Nova`
//! executable).
//!
//! The panel fills the `ïntf`'s `TargArea`:
//!
//! - With no target, "No Target" (`STR#` 2002 #349), dim, centred.
//! - Otherwise the target's name, bright, centred, at `StatFontSize`; its
//!   picture stretched into a 128 x 64 rectangle centred in the area (none
//!   without one); its subtitle, bright, centred, at `SubtitleSize`; its
//!   status at the bottom left, a dim label and a bright value; and its
//!   government's `TargetCode`, dim, against the bottom right.
//! - The status is "Disabled" (#347), bright, for a disabled target;
//!   "Shield:" (#13) and its percentage while its shields hold; and
//!   otherwise "Armor:" (#16) and its armour's percentage, each truncated
//!   and kept within 0-100 %. The original shows "Shields Down" (#15)
//!   instead of the armour unless the ship type says otherwise; here the
//!   armour always shows, so the panel shows it as the phase asks. A ship
//!   with neither shield nor armour shows "No Shields" (#14).
//!
//! The secondary weapon's line fills the `WeapArea`: the weapon's name,
//! bright, centred, followed by its rounds left for one that fires rounds
//! (unless its `Flags2` 0x0040 hides them), or "No Secondary Weapon"
//! (#350), dim.
//!
//! Text is placed by its baseline, as the original places it: a line's
//! top is its size above its baseline. A line is centred, or set against
//! the right, by the [`TextMetrics`] given; without them, it starts at the
//! area's left (or right) edge.
//!
//! The brackets in space are four corners, each two
//! [`BRACKET_LENGTH`]-pixel lines, half the target's sprite out from its
//! centre, coloured by the target's [`Standing`]: red for a target that
//! threatens the player (the original's hostile brackets, `cicn`
//! 10008-10011, and its colour-coded threat red, `_ColorCodeShip`
//! @0x4ae8), grey for a disabled one (`plunder.html`), and blue otherwise.
//! The original's corners are `cicn`s, which the renderer does not have
//! (the colours are placeholders for them), and zoom in as the target is
//! picked, which is left out.

use nova_sim::reserves::Gauge;
use nova_sim::{Condition, Npc, Reserves};

use super::catalog::{StatusBarLayout, TargetCard};
use super::weapons::HIDES_AMMO;
use crate::text::TextMetrics;
use crate::{Color, DrawList, Font, ImageKey, Point};

/// `STR#` 2002 #349.
pub const NO_TARGET: &str = "No Target";
/// `STR#` 2002 #350.
pub const NO_SECONDARY: &str = "No Secondary Weapon";
/// `STR#` 2002 #347.
pub const DISABLED: &str = "Disabled";
/// `STR#` 2002 #13.
pub const SHIELD: &str = "Shield:";
/// `STR#` 2002 #16.
pub const ARMOR: &str = "Armor:";
/// `STR#` 2002 #14.
pub const NO_SHIELDS: &str = "No Shields";
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
/// The status line's and the code's baseline, above the panel's bottom.
pub const BOTTOM_BASELINE: f32 = 6.0;
/// The government's code's distance in from the panel's right.
pub const CODE_INSET: f32 = 7.0;
/// The secondary weapon's baseline, below its area's top.
pub const WEAPON_BASELINE: f32 = 12.0;
/// Each bracket line's length.
pub const BRACKET_LENGTH: f32 = 16.0;
/// The brackets round a target: neutral blue, a placeholder.
pub const BRACKETS: Color = Color::from_rgb24(0x0040_80FF);
/// The brackets round a target threatening the player: red, a
/// placeholder for the original's hostile brackets.
pub const HOSTILE_BRACKETS: Color = Color::from_rgb24(0x00FF_0000);
/// The brackets round a disabled target: grey.
pub const DISABLED_BRACKETS: Color = Color::from_rgb24(0x0088_8888);

/// Sets lines of text in one font into a draw list, measured by the
/// metrics, if any.
struct Pen<'a, 'b> {
    list: &'a mut DrawList,
    font: Font,
    metrics: Option<&'b dyn TextMetrics>,
}

impl Pen<'_, '_> {
    /// How wide `text` is at `size`: nothing without metrics.
    fn width(&self, text: &str, size: f32) -> f32 {
        self.metrics
            .map_or(0.0, |metrics| metrics.width(self.font, size, text))
    }

    /// Sets `text` from `x` on `baseline` at `size` in `color`, and gives
    /// where it ends.
    fn at(&mut self, text: &str, x: f32, baseline: f32, size: f32, color: Color) -> f32 {
        let top = baseline - size;
        self.list
            .text_in(self.font, text, Point::new(x, top), size, None, color);
        x + self.width(text, size)
    }

    /// Sets `text` centred between `left` and `right` on `baseline` at
    /// `size` in `color`.
    fn centred(
        &mut self,
        text: &str,
        left: f32,
        right: f32,
        baseline: f32,
        size: f32,
        color: Color,
    ) {
        let x = left + (right - left - self.width(text, size)) / 2.0;
        let x = if self.metrics.is_some() { x } else { left };
        self.at(text, x, baseline, size, color);
    }
}

/// What the panel shows of the target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetShown<'a> {
    /// Its ship type's name.
    pub name: &'a str,
    /// Its ship type's subtitle and picture.
    pub card: &'a TargetCard,
    /// Its government's `TargetCode`, if any.
    pub code: Option<&'a str>,
    /// Its shield and armour.
    pub reserves: Reserves,
    /// Whether it is disabled.
    pub disabled: bool,
}

/// Draws the target panel of a bar laid out by `layout` at `origin`:
/// `target`, or "No Target" (see the module docs).
pub fn draw_target_panel(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    target: Option<&TargetShown>,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.targ.offset(origin);
    let (top, bottom) = (area.min.y, area.max.y);
    let mut pen = Pen {
        list,
        font: layout.font,
        metrics,
    };
    let Some(target) = target else {
        let at = top + NO_TARGET_BASELINE;
        pen.centred(
            NO_TARGET,
            area.min.x,
            area.max.x,
            at,
            layout.font_size,
            layout.dim_text,
        );
        return;
    };
    let card = target.card;
    if let Some(picture) = card.picture {
        let center = area.center();
        pen.list.stretched_picture(
            ImageKey::picture(picture),
            Point::new(
                center.x - PICTURE_WIDTH / 2.0,
                center.y - PICTURE_HEIGHT / 2.0,
            ),
            PICTURE_WIDTH,
            PICTURE_HEIGHT,
        );
    }
    let (left, right) = (area.min.x, area.max.x);
    let size = layout.font_size;
    pen.centred(
        target.name,
        left,
        right,
        top + NAME_BASELINE,
        size,
        layout.bright_text,
    );
    pen.centred(
        &card.subtitle,
        left,
        right,
        top + SUBTITLE_BASELINE,
        layout.subtitle_size,
        layout.bright_text,
    );
    let baseline = bottom - BOTTOM_BASELINE;
    let x = left + STATUS_INSET;
    let reserves = target.reserves;
    let (label, gauge) = if target.disabled {
        pen.at(DISABLED, x, baseline, size, layout.bright_text);
        (None, None)
    } else if reserves.shield.now > 0.0 {
        (Some(SHIELD), Some(reserves.shield))
    } else if reserves.shield.max <= 0.0 && reserves.armor.max <= 0.0 {
        (Some(NO_SHIELDS), None)
    } else {
        (Some(ARMOR), Some(reserves.armor))
    };
    if let Some(label) = label {
        let after = pen.at(label, x, baseline, size, layout.dim_text);
        if let Some(gauge) = gauge {
            let value = format!(" {}%", percent(gauge));
            pen.at(&value, after, baseline, size, layout.bright_text);
        }
    }
    if let Some(code) = target.code {
        let width = pen.width(code, size);
        pen.at(
            code,
            right - CODE_INSET - width,
            baseline,
            size,
            layout.dim_text,
        );
    }
}

/// The secondary weapon's line: `name`, and its `rounds` unless its
/// `flags2` hide them.
#[must_use]
pub fn secondary_text(name: &str, rounds: Option<u32>, flags2: u16) -> String {
    match rounds {
        Some(rounds) if flags2 & HIDES_AMMO == 0 => format!("{name} - {rounds}"),
        _ => name.to_owned(),
    }
}

/// Draws the secondary weapon's line of a bar laid out by `layout` at
/// `origin`: `weapon`'s line, or "No Secondary Weapon".
pub fn draw_secondary(
    list: &mut DrawList,
    layout: &StatusBarLayout,
    origin: Point,
    weapon: Option<&str>,
    metrics: Option<&dyn TextMetrics>,
) {
    let area = layout.weap.offset(origin);
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
    pen.centred(
        text,
        area.min.x,
        area.max.x,
        baseline,
        layout.font_size,
        color,
    );
}

/// How a target stands towards the player, for its brackets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Neither threatening the player nor disabled.
    Neutral,
    /// Threatening the player ([`Npc::threatens_player`]).
    Hostile,
    /// Disabled.
    Disabled,
}

/// How `npc` stands towards the player.
#[must_use]
pub fn standing(npc: &Npc) -> Standing {
    if npc.condition == Condition::Disabled {
        Standing::Disabled
    } else if npc.threatens_player() {
        Standing::Hostile
    } else {
        Standing::Neutral
    }
}

/// Draws the brackets round a target whose sprite is `sprite_size`
/// across, centred at `at` on screen, coloured by its `standing`.
pub fn draw_brackets(list: &mut DrawList, at: Point, sprite_size: f32, standing: Standing) {
    let half = sprite_size / 2.0;
    let color = match standing {
        Standing::Neutral => BRACKETS,
        Standing::Hostile => HOSTILE_BRACKETS,
        Standing::Disabled => DISABLED_BRACKETS,
    };
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let corner = Point::new(at.x + sx * half, at.y + sy * half);
        let across = Point::new(corner.x - sx * BRACKET_LENGTH, corner.y);
        let down = Point::new(corner.x, corner.y - sy * BRACKET_LENGTH);
        list.line(corner, across, 1.0, color);
        list.line(corner, down, 1.0, color);
    }
}

/// How full `gauge` is, as a truncated percentage within 0-100; full when
/// it holds nothing.
fn percent(gauge: Gauge) -> u32 {
    if gauge.max <= 0.0 {
        return 100;
    }
    (gauge.now / gauge.max * 100.0).clamp(0.0, 100.0) as u32
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Bounds;
    use crate::text::fixture::MonoMetrics;
    use crate::{DrawCommand, Font};
    use nova_sim::{Goal, NpcId, ShipRef};

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

    fn panel(target: Option<&TargetShown>, metrics: Option<&dyn TextMetrics>) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_target_panel(&mut list, &layout(), ORIGIN, target, metrics);
        list.iter().cloned().collect()
    }

    /// The panel runs from x 838 to 1014: 176 across, centred on 926.
    #[test]
    fn with_no_target_the_panel_says_so_dim_and_centred() {
        // "No Target" is 9 characters of 6: 54 wide.
        assert_eq!(
            panel(None, Some(&MonoMetrics)),
            [text(NO_TARGET, 899.0, 365.0, 12.0, DIM)]
        );
        assert_eq!(
            panel(None, None),
            [text(NO_TARGET, 838.0, 365.0, 12.0, DIM)],
            "without metrics, at the left edge"
        );
    }

    fn card(picture: Option<i16>) -> TargetCard {
        TargetCard {
            subtitle: "Light Transport".to_owned(),
            picture,
        }
    }

    fn reserves(shield: f32, armor: f32) -> Reserves {
        Reserves {
            shield: Gauge {
                now: shield,
                max: 40.0,
            },
            armor: Gauge {
                now: armor,
                max: 50.0,
            },
            fuel: Gauge::full(100.0),
        }
    }

    fn shown(card: &TargetCard, reserves: Reserves, disabled: bool) -> TargetShown<'_> {
        TargetShown {
            name: "Shuttle",
            card,
            code: Some("Fed."),
            reserves,
            disabled,
        }
    }

    #[test]
    fn a_target_shows_its_picture_name_subtitle_status_and_code() {
        let card = card(Some(3000));
        let target = shown(&card, reserves(29.3, 50.0), false);
        assert_eq!(
            panel(Some(&target), Some(&MonoMetrics)),
            [
                DrawCommand::StretchedPicture {
                    image: ImageKey::picture(3000),
                    top_left: Point::new(862.0, 354.0),
                    width: 128.0,
                    height: 64.0,
                },
                // "Shuttle": 7 x 6 = 42 wide.
                text("Shuttle", 905.0, 334.0, 12.0, BRIGHT),
                // "Light Transport": 15 x 5 = 75 wide.
                text("Light Transport", 888.5, 349.0, 10.0, BRIGHT),
                text(SHIELD, 843.0, 424.0, 12.0, DIM),
                // "Shield:" is 7 x 6 = 42 wide.
                text(" 73%", 885.0, 424.0, 12.0, BRIGHT),
                // "Fed." is 4 x 6 = 24 wide, against 1007.
                text("Fed.", 983.0, 424.0, 12.0, DIM),
            ]
        );
        assert_eq!((PICTURE_WIDTH, PICTURE_HEIGHT), (128.0, 64.0));
    }

    #[test]
    fn a_target_without_a_picture_or_code_shows_neither() {
        let card = card(None);
        let target = TargetShown {
            code: None,
            ..shown(&card, reserves(40.0, 50.0), false)
        };
        let commands = panel(Some(&target), Some(&MonoMetrics));
        assert_eq!(commands.len(), 4, "{commands:?}");
        assert!(
            commands
                .iter()
                .all(|c| matches!(c, DrawCommand::Text { .. })),
            "{commands:?}"
        );
        assert_eq!(commands[3], text(" 100%", 885.0, 424.0, 12.0, BRIGHT));
    }

    /// The status line's label and value, as drawn.
    fn status(reserves: Reserves, disabled: bool) -> Vec<(String, Color)> {
        let card = card(None);
        let target = TargetShown {
            code: None,
            ..shown(&card, reserves, disabled)
        };
        panel(Some(&target), Some(&MonoMetrics))
            .into_iter()
            .skip(2)
            .map(|command| match command {
                DrawCommand::Text { text, color, .. } => (text, color),
                other => panic!("{other:?}"),
            })
            .collect()
    }

    fn line(parts: &[(&str, Color)]) -> Vec<(String, Color)> {
        parts.iter().map(|(t, c)| ((*t).to_owned(), *c)).collect()
    }

    #[test]
    fn with_its_shields_down_a_target_shows_its_armour() {
        assert_eq!(
            status(reserves(0.0, 20.0), false),
            line(&[(ARMOR, DIM), (" 40%", BRIGHT)])
        );
        assert_eq!(
            status(reserves(-3.0, 49.9), false),
            line(&[(ARMOR, DIM), (" 99%", BRIGHT)])
        );
        assert_eq!(
            status(reserves(0.0, -5.0), false),
            line(&[(ARMOR, DIM), (" 0%", BRIGHT)])
        );
        assert_eq!(
            status(reserves(0.0, 80.0), false),
            line(&[(ARMOR, DIM), (" 100%", BRIGHT)])
        );
    }

    #[test]
    fn the_shield_percentage_is_truncated_and_capped() {
        assert_eq!(
            status(reserves(0.1, 50.0), false),
            line(&[(SHIELD, DIM), (" 0%", BRIGHT)])
        );
        assert_eq!(
            status(reserves(39.9, 50.0), false),
            line(&[(SHIELD, DIM), (" 99%", BRIGHT)])
        );
        assert_eq!(
            status(reserves(60.0, 50.0), false),
            line(&[(SHIELD, DIM), (" 100%", BRIGHT)])
        );
    }

    #[test]
    fn a_disabled_target_says_so() {
        assert_eq!(
            status(reserves(30.0, 10.0), true),
            line(&[(DISABLED, BRIGHT)])
        );
    }

    #[test]
    fn a_ship_holding_neither_shield_nor_armour_has_no_shields() {
        let bare = Reserves::default();
        assert_eq!(status(bare, false), line(&[(NO_SHIELDS, DIM)]));
        let armoured = Reserves {
            armor: Gauge::full(10.0),
            ..Reserves::default()
        };
        assert_eq!(
            status(armoured, false),
            line(&[(ARMOR, DIM), (" 100%", BRIGHT)])
        );
    }

    #[test]
    fn without_metrics_the_lines_start_at_the_panels_edges() {
        let card = card(None);
        let target = shown(&card, reserves(20.0, 50.0), false);
        let commands = panel(Some(&target), None);
        let origins: Vec<Point> = commands
            .iter()
            .map(|command| match command {
                DrawCommand::Text { origin, .. } => *origin,
                other => panic!("{other:?}"),
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
    fn the_secondary_line_is_its_name_and_its_rounds_unless_hidden() {
        assert_eq!(secondary_text("Light Blaster", None, 0), "Light Blaster");
        assert_eq!(secondary_text("Rocket", Some(12), 0), "Rocket - 12");
        assert_eq!(secondary_text("Rocket", Some(0), 0), "Rocket - 0");
        assert_eq!(secondary_text("Mine", Some(12), HIDES_AMMO), "Mine");
        assert_eq!(
            secondary_text("Rocket", Some(12), !HIDES_AMMO),
            "Rocket - 12"
        );
    }

    fn secondary(weapon: Option<&str>, metrics: Option<&dyn TextMetrics>) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_secondary(&mut list, &layout(), ORIGIN, weapon, metrics);
        list.iter().cloned().collect()
    }

    #[test]
    fn the_secondary_line_is_centred_in_the_weapon_area() {
        // "Rocket - 12": 11 x 6 = 66 wide; baseline 312.
        assert_eq!(
            secondary(Some("Rocket - 12"), Some(&MonoMetrics)),
            [text("Rocket - 12", 893.0, 300.0, 12.0, BRIGHT)]
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
    }

    fn brackets(standing: Standing) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_brackets(&mut list, Point::new(500.0, 400.0), 48.0, standing);
        list.iter().cloned().collect()
    }

    fn stroke(from: (f32, f32), to: (f32, f32), color: Color) -> DrawCommand {
        DrawCommand::Line {
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            width: 1.0,
            color,
        }
    }

    #[test]
    fn the_brackets_are_four_corners_half_the_sprite_out() {
        assert_eq!(
            brackets(Standing::Neutral),
            [
                stroke((476.0, 376.0), (492.0, 376.0), BRACKETS),
                stroke((476.0, 376.0), (476.0, 392.0), BRACKETS),
                stroke((524.0, 376.0), (508.0, 376.0), BRACKETS),
                stroke((524.0, 376.0), (524.0, 392.0), BRACKETS),
                stroke((524.0, 424.0), (508.0, 424.0), BRACKETS),
                stroke((524.0, 424.0), (524.0, 408.0), BRACKETS),
                stroke((476.0, 424.0), (492.0, 424.0), BRACKETS),
                stroke((476.0, 424.0), (476.0, 408.0), BRACKETS),
            ]
        );
        assert_eq!(BRACKETS, Color::rgba(0x40, 0x80, 0xFF, 255));
        assert_eq!(BRACKET_LENGTH, 16.0);
    }

    /// Whether every line of `commands`, eight in all, is `color`.
    fn all_of(commands: &[DrawCommand], color: Color) -> bool {
        commands.len() == 8
            && commands.iter().all(|command| {
                matches!(
                    command,
                    DrawCommand::Line { color: c, .. } if *c == color
                )
            })
    }

    #[test]
    fn a_disabled_targets_brackets_are_grey() {
        assert!(all_of(&brackets(Standing::Disabled), DISABLED_BRACKETS));
        assert_eq!(DISABLED_BRACKETS, Color::rgba(0x88, 0x88, 0x88, 255));
    }

    #[test]
    fn a_hostile_targets_brackets_are_red() {
        assert!(all_of(&brackets(Standing::Hostile), HOSTILE_BRACKETS));
        assert_eq!(HOSTILE_BRACKETS, Color::rgba(0xFF, 0x00, 0x00, 255));
    }

    /// NPC 1 with `goal`, in `condition`.
    fn npc(goal: Goal, condition: Condition) -> nova_sim::Npc {
        nova_sim::Npc {
            id: NpcId(1),
            ship: nova_sim::ShipId(128),
            govt: None,
            ai_type: nova_sim::AiType::Warship,
            leader: None,
            class: nova_sim::escort::EscortClass::Warship,
            stats: nova_sim::ShipStats::default(),
            reserves: Reserves::full(1.0, 1.0, 1.0),
            state: nova_sim::ShipState::default(),
            mode: nova_sim::traffic::npc::Mode::Flying,
            goal,
            condition,
            hull: nova_sim::HullSpec::default(),
            armament: nova_sim::Armament::default(),
            rounds: std::collections::BTreeMap::new(),
            trigger: nova_sim::Trigger::default(),
            target: None,
            provoked: 0.0,
            aggression: 0,
            inspected: None,
            booty: 0,
            boarded: false,
            info_types: 0,
            spared: false,
            assisting: 0,
        }
    }

    #[test]
    fn a_target_threatening_the_player_is_hostile_and_a_disabled_one_disabled() {
        let player = ShipRef::Player;
        for goal in [
            Goal::Attack(player),
            Goal::Snipe(player),
            Goal::Flee(player),
        ] {
            assert_eq!(
                standing(&npc(goal, Condition::Intact)),
                Standing::Hostile,
                "{goal:?}"
            );
            assert_eq!(
                standing(&npc(goal, Condition::Disabled)),
                Standing::Disabled,
                "{goal:?}"
            );
        }
        let other = ShipRef::Npc(NpcId(5));
        for goal in [Goal::Idle, Goal::Attack(other), Goal::Inspect(player)] {
            assert_eq!(
                standing(&npc(goal, Condition::Intact)),
                Standing::Neutral,
                "{goal:?}"
            );
        }
    }
}
