//! The screen trait: what every game screen does.

use std::fmt;
use std::time::Duration;

use nova_sim::SimDiagnostic;

use crate::draw::DrawList;
use crate::input::Input;
use crate::preferences::Prefs;
use crate::sound::Sound;

/// What a screen asks of the app after an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenAction {
    /// Carry on.
    None,
    /// Quit the game.
    Quit,
}

/// Which screen the app is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Showing {
    /// The ship browser.
    ShipBrowser,
    /// The galaxy map.
    GalaxyMap,
    /// A system opened from the galaxy map.
    System,
    /// The player's ship in flight.
    Flight,
    /// Flight's course map, opened with M.
    FlightMap,
    /// The spaceport of the stellar landed on.
    Spaceport,
    /// The About text, over another screen.
    About,
    /// The Preferences dialog, over another screen.
    Preferences,
    /// The main menu: New Pilot, Open Pilot and Quit.
    MainMenu,
    /// The new pilot's name entry, over the main menu.
    NewPilot,
    /// The list of saved pilots, over the main menu.
    OpenPilot,
    /// The plunder dialog, over flight, which it pauses.
    Plunder,
    /// The captured-ship assignment dialog, over flight, which it pauses.
    Assignment,
    /// The comm dialog, over flight, which it pauses.
    Comm,
    /// The haggle dialog, over the comm dialog.
    Haggle,
}

/// Something in the game data a screen cannot use, for the app to write
/// out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Diagnostic {
    /// What the simulation does not handle yet.
    Sim(SimDiagnostic),
    /// A resource the screen could not read, and why ("wëap 140: no spïn
    /// 3005").
    Unreadable(String),
}

impl From<SimDiagnostic> for Diagnostic {
    fn from(diagnostic: SimDiagnostic) -> Self {
        Self::Sim(diagnostic)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sim(diagnostic) => diagnostic.fmt(f),
            Self::Unreadable(reason) => f.write_str(reason),
        }
    }
}

/// A game screen: takes input, advances with time and draws itself.
pub trait Screen {
    /// Handles one input event.
    fn input(&mut self, input: &Input) -> ScreenAction;
    /// Advances the screen's clock by `dt`.
    fn tick(&mut self, dt: Duration);
    /// Draws the screen into `list`, in logical coordinates.
    fn draw(&self, list: &mut DrawList);
    /// Abandons any pointer gesture in progress (a held button, a drag)
    /// without completing it: what follows is not a click or a drop. Called
    /// when the screen stops receiving input, such as when the app hides it,
    /// since the button's release will then go elsewhere. Does nothing by
    /// default.
    fn cancel_pointer(&mut self) {}
    /// Forgets every key it holds as down, as if each had been released.
    /// Called when the screen stops receiving input (the app hides it, or
    /// the window loses focus), since the keys' releases will not reach
    /// it. A screen that acts on held keys over time stops acting on them.
    /// Does nothing by default.
    fn release_keys(&mut self) {}
    /// The sounds the screen has made since they were last taken, in
    /// order; taking them empties the list. None by default.
    fn take_sounds(&mut self) -> Vec<Sound> {
        vec![]
    }
    /// The preferences the player has chosen since they were last taken,
    /// if they changed: each change is reported once. None by default.
    fn take_prefs(&mut self) -> Option<Prefs> {
        None
    }
    /// Which screen is showing, for a screen that routes between others;
    /// `None` by default.
    fn now_showing(&self) -> Option<Showing> {
        None
    }
    /// The app is about to quit, whichever way: the screen's last chance
    /// to keep what should outlive it, such as saving the pilot. Does
    /// nothing by default.
    fn quit(&mut self) {}
    /// The problems the screen has run into since they were last taken,
    /// such as a save that failed, each a message for the app to show;
    /// taking them empties the list. None by default.
    fn take_warnings(&mut self) -> Vec<String> {
        Vec::new()
    }
    /// What the screen has found since this was last taken in game data
    /// it cannot use (resources it could not read, and what the simulation
    /// does not handle yet), each for the app to write out; taking them
    /// empties the list. None by default.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;
    use crate::{Color, Point};

    /// Counts ticks and quits on Escape.
    #[derive(Default)]
    struct Counter {
        elapsed: Duration,
    }

    impl Screen for Counter {
        fn input(&mut self, input: &Input) -> ScreenAction {
            match input {
                Input::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                } => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, dt: Duration) {
            self.elapsed += dt;
        }

        fn draw(&self, list: &mut DrawList) {
            list.dot(Point::new(0.0, 0.0), 1.0, Color::WHITE);
        }
    }

    #[test]
    fn a_screen_is_driven_through_the_trait_object() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        let escape = Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        };
        let space = Input::Key {
            key: Key::Space,
            pressed: true,
            repeat: false,
        };
        assert_eq!(screen.input(&space), ScreenAction::None);
        assert_eq!(screen.input(&escape), ScreenAction::Quit);
        screen.tick(Duration::from_millis(5));
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::from_millis(5));
    }

    #[test]
    fn cancelling_the_pointer_does_nothing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.cancel_pointer();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::ZERO);
    }

    #[test]
    fn a_screen_makes_no_sounds_changes_no_prefs_and_names_nothing_showing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(screen.take_prefs(), None);
        assert_eq!(screen.now_showing(), None);
    }

    #[test]
    fn quitting_does_nothing_and_there_are_no_warnings_or_diagnostics_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.quit();
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
        assert_eq!(screen.take_diagnostics(), []);
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::ZERO);
    }

    #[test]
    fn a_diagnostic_reads_as_a_line() {
        let sim = SimDiagnostic::UnimplementedGuidance {
            weapon: nova_sim::WeaponId(131),
            guidance: 1,
        };
        assert_eq!(Diagnostic::from(sim), Diagnostic::Sim(sim));
        assert_eq!(Diagnostic::Sim(sim).to_string(), sim.to_string());
        let unreadable = Diagnostic::Unreadable("wëap 140: no spïn 3005".to_owned());
        assert_eq!(unreadable.to_string(), "wëap 140: no spïn 3005");
    }

    #[test]
    fn releasing_the_keys_does_nothing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.release_keys();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::ZERO);
    }
}
