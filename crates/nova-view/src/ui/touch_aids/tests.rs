//! The table over every modifier click
//! ([`CLICK_MODIFIERS`](crate::keymap::CLICK_MODIFIERS)): each has a
//! toggle on its screen, shown under touch, that holds its command
//! across the next tap as the host's reader reads it. Each host
//! publishes a `toggle_probe` from its own `touch` module.

use super::ModifierToggle;
use crate::geometry::Point;
use crate::input::{Device, Input, Surroundings};
use crate::keymap::{CLICK_MODIFIERS, Command};
use crate::screen::Screen;
use crate::touch::testkit::Fingers;

/// A host of a modifier toggle, under test.
pub(crate) trait ToggleProbe {
    /// The host.
    fn screen(&mut self) -> &mut dyn Screen;

    /// Its toggle.
    fn toggle(&self) -> &ModifierToggle;

    /// A point a click there reads the modifier at.
    fn target(&self) -> Point;

    /// Whether the host's reader holds the toggle's command now, mid-tap.
    fn holds(&mut self) -> bool;
}

/// Each modifier click's host.
fn probes() -> Vec<(Command, Box<dyn ToggleProbe>)> {
    vec![
        (
            Command::Trade(crate::keymap::TradeCommand::MaxLot),
            crate::spaceport::trade::touch::toggle_probe(),
        ),
        (
            Command::Outfitter(crate::keymap::OutfitterCommand::Count),
            crate::spaceport::outfitter::touch::toggle_probe(),
        ),
        (
            Command::Race(crate::keymap::RaceCommand::Ask),
            crate::spaceport::race::touch::toggle_probe(),
        ),
        (
            Command::Map(crate::keymap::MapCommand::PlotCourse),
            crate::galaxy::toggle::toggle_probe(),
        ),
    ]
}

#[test]
fn every_click_modifier_has_a_toggle_on_its_screen() {
    let probes = probes();
    let listed: Vec<Command> = probes.iter().map(|(command, _)| *command).collect();
    assert_eq!(listed, CLICK_MODIFIERS, "one host each, in order");
    for (command, mut probe) in probes {
        probe
            .screen()
            .input(&Input::Surroundings(Surroundings::of(Device::Touch)));
        assert!(probe.toggle().shown(), "{command:?}: shown under touch");
        assert_eq!(probe.toggle().command(), command);
        let toggle = probe.toggle().rect().center();
        let target = probe.target();
        {
            let mut fingers = Fingers::on(probe.screen());
            fingers.down(1, toggle);
            fingers.up(1, toggle);
            fingers.down(1, target);
        }
        assert!(probe.holds(), "{command:?}: held across the tap");
    }
}
