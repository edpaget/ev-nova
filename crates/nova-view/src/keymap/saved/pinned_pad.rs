//! Every default pad binding's saved text, pinned by command name, as
//! [`pinned`](super::pinned) pins the keyboard's: a renamed variant, a
//! renamed position or a changed default can't change the settings file
//! unnoticed. The fixed rows are listed too, though never saved, so
//! their names can't drift either. A new default must be added.

use crate::keymap::{Command, PadBinding, PadMap};

/// Each command with a default pad binding, by name, and its text, in
/// `Command::all()`'s order.
const PINNED: &[(&str, &str)] = &[
    ("Flight.Accelerate", "dpup"),
    ("Flight.ReverseCourse", "dpdown"),
    ("Flight.RotateRight", "dpright"),
    ("Flight.RotateLeft", "dpleft"),
    ("Flight.Afterburner", "lefttrigger"),
    ("Flight.Autopilot", "north"),
    ("Flight.HyperspaceMode", "back+dpup"),
    ("Flight.HyperSelect", "back+dpright"),
    ("Flight.HyperJump", "back+south"),
    ("Flight.NavOff", "back+dpdown"),
    ("Flight.Communicate", "back+west"),
    ("Flight.LandDock", "east"),
    ("Flight.FirePrimary", "south"),
    ("Flight.FireSecondary", "righttrigger"),
    ("Flight.SelectSecondary", "rightstick"),
    ("Flight.WeaponSafety", "back+-leftx"),
    ("Flight.TargetSelect", "rightshoulder"),
    ("Flight.ClosestTarget", "west"),
    ("Flight.EscortMenu", "start+rightshoulder"),
    ("Flight.AttackTarget", "start+south"),
    ("Flight.DefendMe", "start+west"),
    ("Flight.HoldPosition", "start+east"),
    ("Flight.Recall", "start+north"),
    ("Flight.Acknowledge", "leftstick"),
    ("Flight.Board", "back+east"),
    ("Flight.JettisonCargo", "start+righttrigger"),
    ("Flight.Eject", "start+rightstick"),
    ("Flight.SelfDestruct", "start++leftx"),
    ("Flight.EngageCloak", "back++leftx"),
    ("Flight.ShowFramerate", "back+leftstick"),
    ("Flight.FaceStellar", "back+north"),
    ("Flight.SelectSecondaryBack", "back+righttrigger"),
    ("Flight.TargetSelectBack", "leftshoulder"),
    ("Flight.TargetEscort", "back+rightshoulder"),
    ("Flight.TargetEscortBack", "back+leftshoulder"),
    ("Flight.ClosestShip", "back+lefttrigger"),
    ("Flight.ClearTarget", "back+dpleft"),
    ("Flight.HailStellar", "back+rightstick"),
    ("Flight.RecallToHangar", "start+leftshoulder"),
    ("Flight.JettisonNonMission", "start+lefttrigger"),
    ("Flight.StellarSlot1", "start+dpup"),
    ("Flight.StellarSlot2", "start+dpright"),
    ("Flight.StellarSlot3", "start+dpdown"),
    ("Flight.StellarSlot4", "start+dpleft"),
    ("Flight.NearestStellar", "start+leftstick"),
    ("Flight.ZoomOut", "back+-righty"),
    ("Flight.ZoomIn", "back++righty"),
    ("Flight.TurnAxis", "rightx"),
    ("Flight.ThrottleAxis", "lefty"),
    ("EscortMenu.Group1", "start+dpup"),
    ("EscortMenu.Group2", "start+dpright"),
    ("EscortMenu.Group3", "start+dpdown"),
    ("EscortMenu.Group4", "start+dpleft"),
    ("EscortMenu.Group5", "start+leftstick"),
    ("Play.GalaxyMap", "start++righty"),
    ("Play.PlayerInfo", "start+-righty"),
    ("Play.MissionInfo", "start+-leftx"),
    ("Pause.Pause", "back+start"),
    ("Ui.Back", "east"),
    ("Ui.Up", "+lefty"),
    ("Ui.Down", "-lefty"),
    ("Ui.Left", "-leftx"),
    ("Ui.Right", "+leftx"),
    ("Ui.Tab", "rightshoulder"),
    ("Ui.Activate", "south"),
    ("Ui.PageUp", "+righty"),
    ("Ui.PageDown", "-righty"),
    ("Ui.FocusUp", "dpup"),
    ("Ui.FocusDown", "dpdown"),
    ("Ui.FocusLeft", "dpleft"),
    ("Ui.FocusRight", "dpright"),
    ("Ui.PrevFocus", "leftshoulder"),
    ("Trade.MaxLot", "west"),
    ("Outfitter.Count", "west"),
    ("Race.Ask", "west"),
    ("Map.ZoomOut", "lefttrigger"),
    ("Map.ZoomIn", "righttrigger"),
    ("Intro.Activate", "south"),
    ("Intro.Skip", "east"),
];

#[test]
fn every_default_pad_binding_saves_as_its_pinned_text_and_parses_back() {
    let map = PadMap::default();
    let defaults: Vec<(Command, PadBinding)> = Command::all()
        .map(|command| (command, map.binding(command)))
        .filter(|&(_, binding)| binding != PadBinding::Unbound)
        .collect();
    assert_eq!(
        defaults.len(),
        PINNED.len(),
        "every command with a default pad binding is pinned"
    );
    for (&(command, binding), &(name, text)) in defaults.iter().zip(PINNED) {
        assert_eq!(command.name(), name, "in Command::all()'s order");
        assert_eq!(super::pad::text(binding), text, "{name}");
        assert_eq!(
            super::pad::parse(command, text),
            Some(binding),
            "{name} parses back"
        );
    }
}
