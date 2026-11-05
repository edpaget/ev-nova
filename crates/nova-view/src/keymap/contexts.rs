//! Each key-reading screen's contexts, as data.
//!
//! A screen reads an ordered list of [`Context`]s: its own first, then
//! the shared UI context ([`Context::Ui`]) on every screen but flight. Two
//! contexts are **active together** when some list holds both, and the
//! clash query ([`Bindings::clashes`](super::Bindings::clashes)) checks
//! every pair of commands over that. An entry may be **conditional**: on
//! only while its screen says so (the escort menu's group keys, while the
//! menu shows), but counted as active together with the rest of its list
//! all the same. A context may **shadow** another ([`shadows`]): listed
//! first, it wins the keys it binds, so the other's commands on those keys
//! neither resolve nor clash with it.

use super::command::Context;

/// One context of a screen's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The context.
    pub context: Context,
    /// Whether it is on only while the screen switches it on
    /// ([`CommandReader::set_active`](super::CommandReader::set_active)).
    pub conditional: bool,
}

/// An entry always on.
const fn on(context: Context) -> Entry {
    Entry {
        context,
        conditional: false,
    }
}

/// An entry on only while the screen switches it on.
const fn when(context: Context) -> Entry {
    Entry {
        context,
        conditional: true,
    }
}

/// A screen's ordered list of contexts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenContexts {
    /// The screen, for messages.
    pub name: &'static str,
    /// Its contexts, the first winning a key two of them bind.
    pub entries: &'static [Entry],
}

/// Flight: the escort menu's group keys while the menu shows (shadowing
/// the stellar slots, as `_QuickKeyCheck` hides 1-4 from them,
/// @0xa106-0xa124), flight's own, the shared play keys and Pause. No
/// shared UI: flight reads its own Return, arrows, Tab and Space.
pub const FLIGHT: ScreenContexts = ScreenContexts {
    name: "flight",
    entries: &[
        when(Context::EscortMenu),
        on(Context::Flight),
        on(Context::Play),
        on(Context::Pause),
    ],
};

/// A dialog that reads only the shared UI keys (the `ui` module's
/// `Dialog`).
pub const DIALOG: ScreenContexts = ScreenContexts {
    name: "dialog",
    entries: &[on(Context::Ui)],
};

/// Every key-reading screen's list.
pub const SCREENS: &[ScreenContexts] = &[FLIGHT, DIALOG];

/// Whether `shadowing`, listed first, wins the keys it binds over
/// `shadowed`.
#[must_use]
pub const fn shadows(shadowing: Context, shadowed: Context) -> bool {
    matches!(
        (shadowing, shadowed),
        (Context::EscortMenu, Context::Flight)
    )
}

/// Whether `a` and `b` are active together on one of `screens`.
#[must_use]
pub fn together_on(screens: &[ScreenContexts], a: Context, b: Context) -> bool {
    screens.iter().any(|screen| {
        let holds = |context| screen.entries.iter().any(|entry| entry.context == context);
        holds(a) && holds(b)
    })
}

/// Whether `a` and `b` are active together on any screen.
#[must_use]
pub fn together(a: Context, b: Context) -> bool {
    together_on(SCREENS, a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_screen_but_flight_reads_the_shared_ui_context() {
        for screen in SCREENS {
            let has_ui = screen.entries.iter().any(|e| e.context == Context::Ui);
            assert_eq!(has_ui, screen.name != "flight", "{}", screen.name);
        }
    }

    #[test]
    fn every_context_is_on_some_screen() {
        for &context in Context::ALL {
            assert!(together(context, context), "{context:?} is on no screen");
        }
    }

    #[test]
    fn flight_lists_the_escort_menu_first_and_only_while_it_shows() {
        assert_eq!(FLIGHT.entries[0], when(Context::EscortMenu));
        assert!(FLIGHT.entries[1..].iter().all(|e| !e.conditional));
        assert!(together(Context::EscortMenu, Context::Flight));
        assert!(together(Context::Flight, Context::Pause));
        assert!(!together(Context::Flight, Context::Ui));
    }

    #[test]
    fn only_the_escort_menu_shadows_and_only_flight() {
        for &a in Context::ALL {
            for &b in Context::ALL {
                assert_eq!(
                    shadows(a, b),
                    (a, b) == (Context::EscortMenu, Context::Flight),
                    "{a:?} over {b:?}"
                );
            }
        }
    }
}
