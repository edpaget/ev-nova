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
    /// A note over the main menu, such as the warning that an opened
    /// pilot's load had to fall back, before its flight shows.
    Note,
    /// The plunder dialog, over flight, which it pauses.
    Plunder,
    /// The captured-ship assignment dialog, over flight, which it pauses.
    Assignment,
    /// The comm dialog, over flight, which it pauses.
    Comm,
    /// The haggle dialog, over the comm dialog.
    Haggle,
    /// The offer dialog of a person hailed or boarded, over flight, which
    /// it pauses.
    Offer,
    /// A mission text in "Desc Dialog", over flight, which it pauses.
    MissionText,
    /// The mission info screen, over flight, which it pauses.
    MissionInfo,
    /// The player info screen, over flight, which it pauses.
    PlayerInfo,
    /// A `dësc`'s movie, over flight, which it pauses: a mission text's,
    /// or an offer's.
    Movie,
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
    /// The audio side is not waiting on a keyed click: its button-down
    /// sound has ended and its button-up sound has started, or there was
    /// nothing to wait for (sound off, no button-down sound, no audio).
    /// The app calls it after each time it feeds the audio side, so an
    /// action held for the click ([`UiSound::ButtonKeyed`]) runs now.
    /// Does nothing by default.
    ///
    /// [`UiSound::ButtonKeyed`]: crate::sound::UiSound::ButtonKeyed
    fn keyed_click_ended(&mut self) {}
    /// Whether the screen has asked to quit since this was last taken,
    /// outside an input's own [`ScreenAction`]: a quit that waited out a
    /// delay on a tick, or one asked by an input replayed after a keyed
    /// click. Each ask is reported once. `false` by default.
    fn take_quit(&mut self) -> bool {
        false
    }
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
    /// The galaxy map's borders setting to save since this was last taken,
    /// if a map's close changed it, as the original's map saves
    /// `ShowMapBorders` (`_SaveMapBordersState` @0xcf110): each change is
    /// reported once. None by default.
    fn take_map_borders_save(&mut self) -> Option<bool> {
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
    /// Whether the system's pointer is hidden over the window, the screen
    /// drawing its own in its place (flight's cursor sprite). `false` by
    /// default.
    fn hides_pointer(&self) -> bool {
        false
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
    fn a_screen_reports_no_map_borders_save_by_default() {
        assert_eq!(Counter::default().take_map_borders_save(), None);
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
    fn a_screen_leaves_the_system_pointer_shown_by_default() {
        let screen: &dyn Screen = &Counter::default();
        assert!(!screen.hides_pointer());
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
    fn a_keyed_click_ending_does_nothing_and_no_quit_is_asked_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.keyed_click_ended();
        assert!(!screen.take_quit());
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(counter.elapsed, Duration::ZERO);
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
