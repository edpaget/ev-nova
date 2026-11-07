//! The touch controls' shared look: a square control is a translucent
//! fill, a one-point outline and its short label ([`label`]) at
//! [`LABEL_PT`], drawn
//! in [`HIGHLIGHT`] while held or armed. Flight's overlay
//! ([`crate::flight::touch_controls`]) and every touch aid draw with it.

use crate::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::keymap::command::{
    AppCommand, Command, EscortMenuCommand, FlightCommand as F, PauseCommand, PlayCommand,
};

/// A control at rest.
pub const FILL: Color = Color::rgba(200, 210, 230, 72);
/// A control's outline and label.
pub const OUTLINE: Color = Color::rgba(230, 235, 245, 160);
/// A control held down.
pub const HIGHLIGHT: Color = Color::rgba(120, 200, 255, 160);
/// The labels' size, in points.
pub const LABEL_PT: f32 = 12.0;

/// Draws a square control at `rect`: a `fill`, a one-point outline and
/// `text`, sized for `points_per_pixel`.
pub fn draw_square(list: &mut DrawList, rect: Bounds, text: &str, fill: Color, points: f32) {
    fill_rect(list, rect, fill);
    outline(list, rect, 1.0 / points, OUTLINE);
    draw_label(list, rect, text, points);
}

/// Draws `text` as a control's label in `rect`, sized for
/// `points_per_pixel`: [`LABEL_PT`] high, half a label in from the left
/// and centred down.
pub fn draw_label(list: &mut DrawList, rect: Bounds, text: &str, points: f32) {
    let size = LABEL_PT / points;
    list.text(
        text,
        Point::new(rect.min.x + size / 2.0, rect.center().y - size / 2.0),
        size,
        Some(rect.width() - size),
        OUTLINE,
    );
}

/// A `width`-thick outline just inside `rect`.
fn outline(list: &mut DrawList, rect: Bounds, width: f32, color: Color) {
    let Bounds { min, max } = rect;
    for (from, to) in [
        (min, Point::new(max.x, min.y)),
        (Point::new(min.x, max.y), max),
        (min, Point::new(min.x, max.y)),
        (Point::new(max.x, min.y), max),
    ] {
        list.line(from, to, width, color);
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
mod tests {
    use super::*;

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
