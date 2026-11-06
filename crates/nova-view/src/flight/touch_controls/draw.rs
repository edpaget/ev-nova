//! Drawing the touch controls with the renderer's solid fills and text:
//! translucent, so the scene shows through, and highlighted while held.
//!
//! - A square control is a translucent fill, a one-point outline and its
//!   short label ([`label`]) at 12 pt.
//! - A round control (the fire buttons and the afterburner) is a filled
//!   oval ([`oval_fill_pixels`]).
//! - A held control fills brighter, in the highlight colour.
//! - The open menu's panel is a dark fill; Eject and Self-Destruct are
//!   drawn in the warning colour.
//! - The stick draws its ring where the thumb landed and its knob at the
//!   finger, only while held; the zone itself shows a faint ring at its
//!   centre as a hint.
//!
//! Nothing is drawn while the overlay is hidden.

use super::TouchControls;
use super::layout::{ControlId, Placed};
use crate::Color;
use crate::draw::{DrawList, fill_rect};
use crate::flight::radar::IRect;
use crate::geometry::{Bounds, Point};
use crate::keymap::command::{
    AppCommand, Command, EscortMenuCommand, FlightCommand as F, PauseCommand, PlayCommand,
};
use crate::oval::{fill_pixels, oval_fill_pixels, oval_frame_pixels};
use crate::ui::touch_aids::look;

pub use crate::ui::touch_aids::look::{FILL, HIGHLIGHT, LABEL_PT, OUTLINE};

/// Eject and Self-Destruct.
pub const WARNING: Color = Color::rgba(230, 70, 60, 160);
/// The open menu's panel.
pub const PANEL: Color = Color::rgba(10, 12, 20, 160);
/// The held stick's ring.
pub const RING: Color = Color::rgba(230, 235, 245, 140);
/// The stick zone's hint ring.
pub const HINT: Color = Color::rgba(200, 210, 230, 48);

/// Draws the overlay over flight: nothing while it is hidden.
pub fn draw(controls: &TouchControls, list: &mut DrawList) {
    if !controls.showing() {
        return;
    }
    let layout = controls.layout();
    let points = controls.told.points_per_pixel;
    if let Some(panel) = layout.panel {
        fill_rect(list, panel, PANEL);
    }
    for placed in &layout.placed {
        match placed.id {
            ControlId::Stick => draw_stick(controls, placed.rect, list),
            id => draw_button(controls, placed, id, points, list),
        }
    }
}

fn draw_button(
    controls: &TouchControls,
    placed: &Placed,
    id: ControlId,
    points: f32,
    list: &mut DrawList,
) {
    let held = match id {
        ControlId::Command(command) => controls.holds(command),
        ControlId::More => controls.menu_open(),
        _ => false,
    };
    let warning = matches!(
        id,
        ControlId::Command(Command::Flight(F::Eject | F::SelfDestruct))
    );
    let fill = if held {
        HIGHLIGHT
    } else if warning {
        WARNING
    } else {
        FILL
    };
    let rect = placed.rect;
    let text = match id {
        ControlId::Command(command) => label(command),
        ControlId::More => "More",
        ControlId::PagePrev => "\u{25c0}",
        ControlId::PageNext => "\u{25b6}",
        ControlId::Stick => "",
    };
    if placed.round {
        fill_pixels(list, &oval_fill_pixels(whole(rect)), fill);
        look::draw_label(list, rect, text, points);
    } else {
        look::draw_square(list, rect, text, fill, points);
    }
}

fn draw_stick(controls: &TouchControls, zone: Bounds, list: &mut DrawList) {
    let radius = controls.stick_radius();
    if let Some((base, at)) = controls.stick() {
        let ring = around(base, radius);
        fill_pixels(list, &oval_frame_pixels(whole(ring), 2), RING);
        fill_pixels(list, &oval_fill_pixels(whole(ring)), FILL);
        let knob = around(at, radius / 2.0);
        fill_pixels(list, &oval_fill_pixels(whole(knob)), HIGHLIGHT);
    } else {
        let hint = around(zone.center(), radius);
        fill_pixels(list, &oval_frame_pixels(whole(hint), 2), HINT);
    }
}

/// The square of half-side `radius` about `centre`.
fn around(centre: Point, radius: f32) -> Bounds {
    Bounds {
        min: Point::new(centre.x - radius, centre.y - radius),
        max: Point::new(centre.x + radius, centre.y + radius),
    }
}

/// `rect` on whole pixels.
#[allow(clippy::cast_possible_truncation)] // A canvas: a few thousand units.
fn whole(rect: Bounds) -> IRect {
    IRect {
        left: rect.min.x.round() as i32,
        top: rect.min.y.round() as i32,
        right: rect.max.x.round() as i32,
        bottom: rect.max.y.round() as i32,
    }
}

/// The short label a control shows for `command`.
#[must_use]
pub const fn label(command: Command) -> &'static str {
    match command {
        Command::Flight(command) => flight_label(command),
        Command::Play(PlayCommand::GalaxyMap) => "Map",
        Command::Play(PlayCommand::PlayerInfo) => "Pilot",
        Command::Play(PlayCommand::MissionInfo) => "Missions",
        Command::Pause(PauseCommand::Pause) => "Pause",
        Command::App(AppCommand::DoubleTime) => "2x Time",
        Command::EscortMenu(EscortMenuCommand::Group1) => "Group 1",
        Command::EscortMenu(EscortMenuCommand::Group2) => "Group 2",
        Command::EscortMenu(EscortMenuCommand::Group3) => "Group 3",
        Command::EscortMenu(EscortMenuCommand::Group4) => "Group 4",
        Command::EscortMenu(EscortMenuCommand::Group5) => "Freight",
        other => other.label(),
    }
}

const fn flight_label(command: F) -> &'static str {
    match command {
        F::Accelerate => "Thrust",
        F::ReverseCourse => "Reverse",
        F::RotateRight => "Right",
        F::RotateLeft => "Left",
        F::Afterburner => "Burn",
        F::Autopilot => "Face",
        F::HyperspaceMode => "Hyper",
        F::HyperSelect => "Select Sys",
        F::HyperJump => "Jump",
        F::NavOff => "Nav Off",
        F::Communicate => "Hail",
        F::LandDock => "Land",
        F::FirePrimary => "Fire",
        F::FireSecondary => "Fire 2",
        F::SelectSecondary => "Weapon",
        F::WeaponSafety => "Safety",
        F::TargetSelect => "Target",
        F::ClosestTarget => "Nearest",
        F::EscortMenu => "Escorts",
        F::AttackTarget => "Attack",
        F::DefendMe => "Defend",
        F::HoldPosition => "Hold",
        F::Recall => "Recall",
        F::Acknowledge => "Ack",
        F::Board => "Board",
        F::JettisonCargo => "Jettison",
        F::Eject => "Eject",
        F::SelfDestruct => "Destruct",
        F::EngageCloak => "Cloak",
        F::ShowFramerate => "FPS",
        F::FaceStellar => "Face Planet",
        F::SelectSecondaryBack => "Weapon Back",
        F::TargetSelectBack => "Target Back",
        F::TargetEscort => "Escort",
        F::TargetEscortBack => "Escort Back",
        F::ClosestShip => "Near Ship",
        F::ClearTarget => "Untarget",
        F::HailStellar => "Hail Planet",
        F::RecallToHangar => "Hangar",
        F::JettisonNonMission => "Jettison 2",
        F::StellarSlot1 => "Planet 1",
        F::StellarSlot2 => "Planet 2",
        F::StellarSlot3 => "Planet 3",
        F::StellarSlot4 => "Planet 4",
        F::NearestStellar => "Near Planet",
        F::ZoomOut => "Zoom Out",
        F::ZoomIn => "Zoom In",
        F::TurnAxis => "Turn",
        F::ThrottleAxis => "Throttle",
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::{Device, FingerId, Surroundings, Touch, TouchPhase};

    fn showing() -> TouchControls {
        let mut controls = TouchControls::default();
        controls.set_view((1024.0, 768.0));
        controls.tell(Surroundings::of(Device::Touch));
        controls.set_live(true);
        controls
    }

    fn drawn(controls: &TouchControls) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw(controls, &mut list);
        list.iter().cloned().collect()
    }

    fn colours(drawn: &[DrawCommand]) -> Vec<Color> {
        drawn
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Line { color, .. } | DrawCommand::Text { color, .. } => Some(color),
                _ => None,
            })
            .collect()
    }

    fn down(controls: &mut TouchControls, n: usize, at: Point) {
        controls.touch(Touch {
            finger: FingerId::new(n),
            phase: TouchPhase::Started(at),
        });
    }

    fn centre(controls: &TouchControls, id: ControlId) -> Point {
        controls.layout().get(id).expect("placed").rect.center()
    }

    #[test]
    fn each_control_draws_translucent() {
        let drawn = drawn(&showing());
        assert!(!drawn.is_empty());
        for colour in colours(&drawn) {
            assert!(colour.a < 255, "{colour:?}");
        }
    }

    #[test]
    fn a_held_control_draws_in_the_highlight() {
        let mut controls = showing();
        assert!(!colours(&drawn(&controls)).contains(&HIGHLIGHT));
        let fire = centre(&controls, ControlId::Command(F::FirePrimary.into()));
        down(&mut controls, 1, fire);
        assert!(colours(&drawn(&controls)).contains(&HIGHLIGHT));
    }

    #[test]
    fn the_stick_draws_its_ring_where_the_thumb_landed_and_its_knob_at_the_finger() {
        let mut controls = showing();
        let zone = centre(&controls, ControlId::Stick);
        let hint = drawn(&controls);
        assert!(colours(&hint).contains(&HINT), "the zone's hint");
        let base = Point::new(zone.x - 20.0, zone.y + 10.0);
        down(&mut controls, 1, base);
        let radius = controls.stick_radius();
        let to = Point::new(base.x + radius / 2.0, base.y);
        controls.touch(Touch {
            finger: FingerId::new(1),
            phase: TouchPhase::Moved(to),
        });
        let held = drawn(&controls);
        let rows = |color: Color| -> Vec<(f32, f32, f32)> {
            held.iter()
                .filter_map(|command| match *command {
                    DrawCommand::Line {
                        from, to, color: c, ..
                    } if c == color => Some((from.y, from.x, to.x)),
                    _ => None,
                })
                .collect()
        };
        let ring = rows(RING);
        let ring_left = ring.iter().map(|r| r.1).fold(f32::MAX, f32::min);
        let ring_right = ring.iter().map(|r| r.2).fold(f32::MIN, f32::max);
        assert!(
            (f32::midpoint(ring_left, ring_right) - base.x).abs() <= 1.0,
            "ring at the base"
        );
        let knob = rows(HIGHLIGHT);
        let knob_left = knob.iter().map(|r| r.1).fold(f32::MAX, f32::min);
        let knob_right = knob.iter().map(|r| r.2).fold(f32::MIN, f32::max);
        assert!(
            (f32::midpoint(knob_left, knob_right) - to.x).abs() <= 1.0,
            "knob at the finger"
        );
        assert!(!colours(&held).contains(&HINT), "no hint while held");
    }

    #[test]
    fn the_open_menu_draws_eject_and_self_destruct_in_the_warning_colour() {
        let mut controls = showing();
        assert!(!colours(&drawn(&controls)).contains(&WARNING));
        let more = centre(&controls, ControlId::More);
        down(&mut controls, 1, more);
        controls.touch(Touch {
            finger: FingerId::new(1),
            phase: TouchPhase::Ended(more),
        });
        let open = drawn(&controls);
        assert!(colours(&open).contains(&PANEL));
        let eject = controls
            .layout()
            .get(ControlId::Command(F::Eject.into()))
            .expect("in the menu")
            .rect;
        let warned = open.iter().any(|command| {
            matches!(*command, DrawCommand::Line { color, from, .. }
                if color == WARNING && eject.contains(from))
        });
        assert!(warned);
    }

    #[test]
    fn nothing_is_drawn_while_hidden() {
        let mut controls = showing();
        controls.tell(Surroundings::of(Device::Keyboard));
        assert_eq!(drawn(&controls), []);
        let mut dead = showing();
        dead.set_live(false);
        assert_eq!(drawn(&dead), []);
    }

    #[test]
    fn more_draws_in_the_highlight_while_the_menu_is_open() {
        let mut controls = showing();
        let more = controls.layout().get(ControlId::More).expect("More").rect;
        let lit = |controls: &TouchControls| {
            drawn(controls).iter().any(|command| {
                matches!(*command, DrawCommand::Line { color, from, .. }
                    if color == HIGHLIGHT && more.contains(from))
            })
        };
        assert!(!lit(&controls));
        down(&mut controls, 1, more.center());
        controls.touch(Touch {
            finger: FingerId::new(1),
            phase: TouchPhase::Ended(more.center()),
        });
        assert!(lit(&controls));
    }

    #[test]
    fn a_square_control_draws_its_outline_and_its_label_sized_in_points() {
        let mut controls = showing();
        controls.tell(Surroundings {
            points_per_pixel: 2.0,
            ..Surroundings::of(Device::Touch)
        });
        let land = F::LandDock.into();
        let rect = controls
            .layout()
            .get(ControlId::Command(land))
            .expect("Land")
            .rect;
        let drawn = drawn(&controls);
        let size = LABEL_PT / 2.0;
        let text = drawn
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    size,
                    wrap_width,
                    ..
                } if text == label(land) => Some((*origin, *size, *wrap_width)),
                _ => None,
            })
            .expect("its label");
        assert_eq!(
            text,
            (
                Point::new(rect.min.x + size / 2.0, rect.center().y - size / 2.0),
                size,
                Some(rect.width() - size)
            )
        );
        let top = DrawCommand::Line {
            from: rect.min,
            to: Point::new(rect.max.x, rect.min.y),
            width: 0.5,
            color: OUTLINE,
        };
        assert!(drawn.contains(&top), "a one-point outline");
    }

    #[test]
    fn the_rings_are_the_sticks_size_and_the_knob_half_of_it() {
        let mut controls = showing();
        let zone = centre(&controls, ControlId::Stick);
        down(&mut controls, 1, zone);
        let radius = controls.stick_radius();
        let extent = |color: Color| {
            let rows: Vec<(f32, f32)> = drawn(&controls)
                .iter()
                .filter_map(|command| match *command {
                    DrawCommand::Line {
                        from, to, color: c, ..
                    } if c == color => Some((from.x, to.x)),
                    _ => None,
                })
                .collect();
            let left = rows.iter().map(|r| r.0).fold(f32::MAX, f32::min);
            let right = rows.iter().map(|r| r.1).fold(f32::MIN, f32::max);
            right - left
        };
        assert!(
            (extent(RING) - 2.0 * radius).abs() <= 1.0,
            "{}",
            extent(RING)
        );
        assert!(
            (extent(HIGHLIGHT) - radius).abs() <= 1.0,
            "{}",
            extent(HIGHLIGHT)
        );
        let rows: Vec<f32> = drawn(&controls)
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Line { from, color, .. } if color == RING => Some(from.y),
                _ => None,
            })
            .collect();
        let top = rows.iter().copied().fold(f32::MAX, f32::min);
        let bottom = rows.iter().copied().fold(f32::MIN, f32::max);
        assert!((top - (zone.y - radius)).abs() <= 1.0, "{top}");
        assert!((bottom - (zone.y + radius)).abs() <= 2.0, "{bottom}");
    }

    #[test]
    fn the_labels_are_the_short_names() {
        assert_eq!(label(F::FirePrimary.into()), "Fire");
        assert_eq!(label(F::LandDock.into()), "Land");
        assert_eq!(label(PlayCommand::GalaxyMap.into()), "Map");
        assert_eq!(label(EscortMenuCommand::Group5.into()), "Freight");
        let flight: Vec<&str> = Command::all()
            .filter(|command| matches!(command, Command::Flight(_)))
            .map(label)
            .collect();
        let distinct: std::collections::BTreeSet<&str> = flight.iter().copied().collect();
        assert_eq!(distinct.len(), flight.len(), "each its own");
    }

    #[test]
    fn every_control_has_a_label() {
        for command in Command::all() {
            if let Command::Flight(_)
            | Command::Play(_)
            | Command::Pause(_)
            | Command::App(_)
            | Command::EscortMenu(_) = command
            {
                assert!(!label(command).is_empty(), "{command:?}");
            }
        }
    }
}
