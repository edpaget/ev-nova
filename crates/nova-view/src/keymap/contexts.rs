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

/// The comm dialog: its Close letter, then the shared UI keys.
pub const COMM: ScreenContexts = ScreenContexts {
    name: "comm",
    entries: &[on(Context::Comm), on(Context::Ui)],
};

/// The haggle dialog: Accept and Lower, then the shared UI keys.
pub const HAGGLE: ScreenContexts = ScreenContexts {
    name: "haggle",
    entries: &[on(Context::Haggle), on(Context::Ui)],
};

/// The planet comm dialog: its buttons' letters, then the shared UI keys.
pub const PLANET_COMM: ScreenContexts = ScreenContexts {
    name: "planet comm",
    entries: &[on(Context::PlanetComm), on(Context::Ui)],
};

/// The escort dialog: its buttons' letters, then the shared UI keys.
pub const ESCORT_DIALOG: ScreenContexts = ScreenContexts {
    name: "escort dialog",
    entries: &[on(Context::EscortDialog), on(Context::Ui)],
};

/// The captured ship's assignment: M and E, then the shared UI keys.
pub const ASSIGNMENT: ScreenContexts = ScreenContexts {
    name: "assignment",
    entries: &[on(Context::Assignment), on(Context::Ui)],
};

/// The player info screen: Shift-Tab, the shared play keys (its own key
/// closes it, `_PlayerInfoFilter` @0x53ba3) and the shared UI keys.
pub const PLAYER_INFO: ScreenContexts = ScreenContexts {
    name: "player info",
    entries: &[on(Context::PlayerInfo), on(Context::Play), on(Context::Ui)],
};

/// A text dialog ("Desc Dialog"): the shared play keys, as
/// `_TextDialogFilter` reads the map key (@0x51fa6), and the shared UI
/// keys.
pub const DESC: ScreenContexts = ScreenContexts {
    name: "desc",
    entries: &[on(Context::Play), on(Context::Ui)],
};

/// A close's wait for its key's release (the `ui` module's `key_wait`):
/// the shared play keys, which the filters under it read live
/// (`_PortFilter` @0x4f171-0x4f1c5 and its kin), beside the shared UI
/// keys of the screen it waits on.
pub const KEY_WAIT: ScreenContexts = ScreenContexts {
    name: "key wait",
    entries: &[on(Context::Play), on(Context::Ui)],
};

/// The spaceport: its buttons' letters, the shared play keys
/// (`_PortFilter` reads the map key first, @0x4f165, then mission info
/// and player info) and the shared UI keys.
pub const PORT: ScreenContexts = ScreenContexts {
    name: "spaceport",
    entries: &[on(Context::Port), on(Context::Play), on(Context::Ui)],
};

/// The Trade Center: Buy, Sell, Option and Shift-Tab, the shared play
/// keys (`_TradeFilter` @0x4e0e7, @0x4e109) and the shared UI keys.
pub const TRADE: ScreenContexts = ScreenContexts {
    name: "trade center",
    entries: &[on(Context::Trade), on(Context::Play), on(Context::Ui)],
};

/// The outfitter: Buy, Sell and Option, the shared play keys
/// (`_OutfitFilter` reads the map and mission info keys, @0x580f3) and
/// the shared UI keys.
pub const OUTFITTER: ScreenContexts = ScreenContexts {
    name: "outfitter",
    entries: &[on(Context::Outfitter), on(Context::Play), on(Context::Ui)],
};

/// The shipyard and its Info panel: Buy Ship, the shared play keys
/// (`_ShipyardFilter` @0x608ed, `_ShipDescFilter` @0x51108) and the
/// shared UI keys.
pub const SHIPYARD: ScreenContexts = ScreenContexts {
    name: "shipyard",
    entries: &[on(Context::Shipyard), on(Context::Play), on(Context::Ui)],
};

/// The bar and the hire screen it hosts: its letters, the shared play
/// keys (`_BarFilter` @0x47a83, and the hire screen's filters') and the
/// shared UI keys.
pub const BAR: ScreenContexts = ScreenContexts {
    name: "bar",
    entries: &[on(Context::Bar), on(Context::Play), on(Context::Ui)],
};

/// The hire screen: Hire Escort and the shared UI keys; the bar reads its
/// play keys for it.
pub const HIRE: ScreenContexts = ScreenContexts {
    name: "hire",
    entries: &[on(Context::Hire), on(Context::Ui)],
};

/// The mission computer: Leave and Accept, the shared play keys
/// (`_MissionFilter` @0x9f7fa) and the shared UI keys.
pub const MISSION_COMPUTER: ScreenContexts = ScreenContexts {
    name: "mission computer",
    entries: &[
        on(Context::MissionComputer),
        on(Context::Play),
        on(Context::Ui),
    ],
};

/// The mission info screen: the shared play keys (`_MissionInfoFilter`
/// reads the map key and its own, @0x9d715) and the shared UI keys.
pub const MISSION_INFO: ScreenContexts = ScreenContexts {
    name: "mission info",
    entries: &[on(Context::Play), on(Context::Ui)],
};

/// A mission offer: the shared play keys (`_SingleMissionFilter`
/// @0x98db0) and the shared UI keys. Its labels' letters are typed
/// characters, no commands.
pub const OFFER: ScreenContexts = ScreenContexts {
    name: "offer",
    entries: &[on(Context::Play), on(Context::Ui)],
};

/// The bar race: its Option and the shared UI keys.
pub const RACE: ScreenContexts = ScreenContexts {
    name: "race",
    entries: &[on(Context::Race), on(Context::Ui)],
};

/// Every key-reading screen's list.
pub const SCREENS: &[ScreenContexts] = &[
    FLIGHT,
    DIALOG,
    COMM,
    HAGGLE,
    PLANET_COMM,
    ESCORT_DIALOG,
    ASSIGNMENT,
    PLAYER_INFO,
    DESC,
    KEY_WAIT,
    PORT,
    TRADE,
    OUTFITTER,
    SHIPYARD,
    BAR,
    HIRE,
    MISSION_COMPUTER,
    MISSION_INFO,
    OFFER,
    RACE,
];

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
