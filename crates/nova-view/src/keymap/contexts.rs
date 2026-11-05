//! Each key-reading screen's contexts, as data.
//!
//! A screen reads an ordered list of [`Context`]s: its own first, then
//! the shared UI context ([`Context::Ui`]) on every screen but flight and
//! the intro (which polls Return's key code alone, [`INTRO`]), and last
//! the app's own ([`Context::App`]), which the app's router reads
//! whatever is shown, flight included, so a key bound there clashes with
//! every other context's. Two
//! contexts are **active together** when some list holds both, and the
//! clash query ([`Bindings::clashes`](super::Bindings::clashes)) checks
//! every pair of commands over that. An entry may be **conditional**: on
//! only while its screen says so (the escort menu's group keys, while the
//! menu shows), but counted as active together with the rest of its list
//! all the same. A context may **shadow** another ([`shadows`]): listed
//! first, it wins the keys it binds, so the other's commands on those keys
//! neither resolve nor clash with it.
//!
//! The app's router reads every key before the developer's sides
//! ([`ROUTED`]: the ship browser, and the galaxy viewer and system view
//! under the navigator), with its own reader ([`ROUTER`]). It acts on the
//! main screen's Fly and About and the shared Tab and Back there, and
//! hands every other key to the screen, so its contexts that act
//! ([`ROUTER_OVER`]) count as active together with each routed screen's,
//! though no routed list holds them: holding them would make the screen's
//! own reader resolve the router's keys. One pair is a deliberate shadow
//! ([`router_shadows`]): the router's Fly wins F, by default, over the
//! viewer's Find, which still finds through its S twin.

use super::command::{Command, Context, MainScreenCommand, MapCommand};

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
        on(Context::App),
    ],
};

/// A dialog that reads only the shared UI keys (the `ui` module's
/// `Dialog`).
pub const DIALOG: ScreenContexts = ScreenContexts {
    name: "dialog",
    entries: &[on(Context::Ui), on(Context::App)],
};

/// The comm dialog: its Close letter, then the shared UI keys.
pub const COMM: ScreenContexts = ScreenContexts {
    name: "comm",
    entries: &[on(Context::Comm), on(Context::Ui), on(Context::App)],
};

/// The haggle dialog: Accept and Lower, then the shared UI keys.
pub const HAGGLE: ScreenContexts = ScreenContexts {
    name: "haggle",
    entries: &[on(Context::Haggle), on(Context::Ui), on(Context::App)],
};

/// The planet comm dialog: its buttons' letters, then the shared UI keys.
pub const PLANET_COMM: ScreenContexts = ScreenContexts {
    name: "planet comm",
    entries: &[on(Context::PlanetComm), on(Context::Ui), on(Context::App)],
};

/// The escort dialog: its buttons' letters, then the shared UI keys.
pub const ESCORT_DIALOG: ScreenContexts = ScreenContexts {
    name: "escort dialog",
    entries: &[on(Context::EscortDialog), on(Context::Ui), on(Context::App)],
};

/// The captured ship's assignment: M and E, then the shared UI keys.
pub const ASSIGNMENT: ScreenContexts = ScreenContexts {
    name: "assignment",
    entries: &[on(Context::Assignment), on(Context::Ui), on(Context::App)],
};

/// The player info screen: Shift-Tab, the shared play keys (its own key
/// closes it, `_PlayerInfoFilter` @0x53ba3) and the shared UI keys.
pub const PLAYER_INFO: ScreenContexts = ScreenContexts {
    name: "player info",
    entries: &[
        on(Context::PlayerInfo),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// A text dialog ("Desc Dialog"): the shared play keys, as
/// `_TextDialogFilter` reads the map key (@0x51fa6), and the shared UI
/// keys.
pub const DESC: ScreenContexts = ScreenContexts {
    name: "desc",
    entries: &[on(Context::Play), on(Context::Ui), on(Context::App)],
};

/// A close's wait for its key's release (the `ui` module's `key_wait`):
/// the shared play keys, which the filters under it read live
/// (`_PortFilter` @0x4f171-0x4f1c5 and its kin), beside the shared UI
/// keys of the screen it waits on.
pub const KEY_WAIT: ScreenContexts = ScreenContexts {
    name: "key wait",
    entries: &[on(Context::Play), on(Context::Ui), on(Context::App)],
};

/// The spaceport: its buttons' letters, the shared play keys
/// (`_PortFilter` reads the map key first, @0x4f165, then mission info
/// and player info) and the shared UI keys.
pub const PORT: ScreenContexts = ScreenContexts {
    name: "spaceport",
    entries: &[
        on(Context::Port),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The Trade Center: Buy, Sell, Option and Shift-Tab, the shared play
/// keys (`_TradeFilter` @0x4e0e7, @0x4e109) and the shared UI keys.
pub const TRADE: ScreenContexts = ScreenContexts {
    name: "trade center",
    entries: &[
        on(Context::Trade),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The outfitter: Buy, Sell and Option, the shared play keys
/// (`_OutfitFilter` reads the map and mission info keys, @0x580f3) and
/// the shared UI keys.
pub const OUTFITTER: ScreenContexts = ScreenContexts {
    name: "outfitter",
    entries: &[
        on(Context::Outfitter),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The shipyard and its Info panel: Buy Ship, the shared play keys
/// (`_ShipyardFilter` @0x608ed, `_ShipDescFilter` @0x51108) and the
/// shared UI keys.
pub const SHIPYARD: ScreenContexts = ScreenContexts {
    name: "shipyard",
    entries: &[
        on(Context::Shipyard),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The bar and the hire screen it hosts: its letters, the shared play
/// keys (`_BarFilter` @0x47a83, and the hire screen's filters') and the
/// shared UI keys.
pub const BAR: ScreenContexts = ScreenContexts {
    name: "bar",
    entries: &[
        on(Context::Bar),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The hire screen: Hire Escort and the shared UI keys; the bar reads its
/// play keys for it.
pub const HIRE: ScreenContexts = ScreenContexts {
    name: "hire",
    entries: &[on(Context::Hire), on(Context::Ui), on(Context::App)],
};

/// The mission computer: Leave and Accept, the shared play keys
/// (`_MissionFilter` @0x9f7fa) and the shared UI keys.
pub const MISSION_COMPUTER: ScreenContexts = ScreenContexts {
    name: "mission computer",
    entries: &[
        on(Context::MissionComputer),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The mission info screen: the shared play keys (`_MissionInfoFilter`
/// reads the map key and its own, @0x9d715) and the shared UI keys.
pub const MISSION_INFO: ScreenContexts = ScreenContexts {
    name: "mission info",
    entries: &[on(Context::Play), on(Context::Ui), on(Context::App)],
};

/// A mission offer: the shared play keys (`_SingleMissionFilter`
/// @0x98db0) and the shared UI keys. Its labels' letters are typed
/// characters, no commands.
pub const OFFER: ScreenContexts = ScreenContexts {
    name: "offer",
    entries: &[on(Context::Play), on(Context::Ui), on(Context::App)],
};

/// The bar race: its Option and the shared UI keys.
pub const RACE: ScreenContexts = ScreenContexts {
    name: "race",
    entries: &[on(Context::Race), on(Context::Ui), on(Context::App)],
};

/// The galaxy viewer, the developer's map: its own keys and the shared UI
/// keys (the arrows pan, Return enters a system). The map key means
/// nothing there, and its Tab and Escape are the app's router's and the
/// navigator's. The router reads each key first ([`ROUTED`]), its Fly
/// winning F over Find ([`router_shadows`]).
pub const GALAXY_VIEWER: ScreenContexts = ScreenContexts {
    name: "galaxy viewer",
    entries: &[on(Context::Map), on(Context::Ui), on(Context::App)],
};

/// The course and hypergate maps over flight or the spaceport: their own
/// keys, the shared play keys (the map key closes them, `_MapFilter`
/// @0x144ea) and the shared UI keys.
pub const COURSE_MAP: ScreenContexts = ScreenContexts {
    name: "course map",
    entries: &[
        on(Context::Map),
        on(Context::Play),
        on(Context::Ui),
        on(Context::App),
    ],
};

/// The navigator over the viewer and the system view: Escape, the shared
/// Back, goes from a system back to the map.
pub const NAVIGATOR: ScreenContexts = ScreenContexts {
    name: "navigator",
    entries: &[on(Context::Ui), on(Context::App)],
};

/// The system view: WASD, its own context's twins of the shared arrows,
/// and the shared UI keys (the arrows pan, Space recentres).
pub const SYSTEM_VIEW: ScreenContexts = ScreenContexts {
    name: "system view",
    entries: &[on(Context::SystemView), on(Context::Ui), on(Context::App)],
};

/// The ship browser: the shared arrows. The router reads each key first
/// ([`ROUTED`]).
pub const SHIP_BROWSER: ScreenContexts = ScreenContexts {
    name: "ship browser",
    entries: &[on(Context::Ui), on(Context::App)],
};

/// The saved pilots' list: the shared arrows, Confirm and Back.
pub const PILOT_LIST: ScreenContexts = ScreenContexts {
    name: "pilot list",
    entries: &[on(Context::Ui), on(Context::App)],
};

/// The intro screen: its own Return and Space, and the Pause key, its
/// skip (`_ShowIntroScreen` @0x16d05). No shared UI: it polls Return's
/// key code alone, so the keypad's Enter, the shared Confirm's twin,
/// doesn't end a picture.
pub const INTRO: ScreenContexts = ScreenContexts {
    name: "intro",
    entries: &[on(Context::Intro), on(Context::Pause), on(Context::App)],
};

/// A movie playing (`_PlayQTMovie`): the shared Activate (Space) and
/// Confirm (Return, the keypad's Enter its twin) skip it.
pub const MOVIE: ScreenContexts = ScreenContexts {
    name: "movie",
    entries: &[on(Context::Ui), on(Context::App)],
};

/// The app's router on the main menu and the developer's sides (the ship
/// browser and the galaxy viewer): the main menu's keys it reads beside
/// the menu's typed letters, the main screen's Fly, About and
/// Preferences, the shared UI keys (Tab switches sides, Back goes back
/// or quits) and the app's own. In flight it reads Pause through
/// [`FLIGHT`]'s list, and on a map its own Back. On the developer's sides
/// it reads each key before the screen shown ([`ROUTED`]).
pub const ROUTER: ScreenContexts = ScreenContexts {
    name: "router",
    entries: &[
        on(Context::MainMenu),
        on(Context::MainScreen),
        on(Context::Ui),
        on(Context::App),
    ],
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
    GALAXY_VIEWER,
    COURSE_MAP,
    NAVIGATOR,
    SYSTEM_VIEW,
    SHIP_BROWSER,
    PILOT_LIST,
    INTRO,
    MOVIE,
    ROUTER,
];

/// The screens whose keys the app's router reads first (`nova`'s
/// `AppScreen::route`, on the ship browser's and the galaxy's sides): it
/// takes what [`ROUTER_OVER`]'s contexts mean to it and hands the rest on.
pub const ROUTED: &[ScreenContexts] = &[GALAXY_VIEWER, NAVIGATOR, SYSTEM_VIEW, SHIP_BROWSER];

/// The router's contexts that act on a [`ROUTED`] screen's keys: the main
/// screen's (Fly enters flight, About opens its dialog), the shared UI
/// context's (Tab switches sides, Back goes back or quits) and the app's.
/// Not the main menu's: the router hands its keys on there.
pub const ROUTER_OVER: &[Context] = &[Context::MainScreen, Context::Ui, Context::App];

/// Whether `shadowing`, listed first, wins the keys it binds over
/// `shadowed`.
#[must_use]
pub const fn shadows(shadowing: Context, shadowed: Context) -> bool {
    matches!(
        (shadowing, shadowed),
        (Context::EscortMenu, Context::Flight)
    )
}

/// Whether the router's `shadowing`, read before a [`ROUTED`] screen's
/// keys, wins its key over the screen's `shadowed` by design, so the two
/// don't clash: the main screen's Fly over the galaxy viewer's Find, both
/// on F by default. The router enters flight on F; the viewer still finds
/// through S, the twin `_MapFilter` reads beside F (@0x144b9-0x144d6).
#[must_use]
pub const fn router_shadows(shadowing: Command, shadowed: Command) -> bool {
    matches!(
        (shadowing, shadowed),
        (
            Command::MainScreen(MainScreenCommand::Fly),
            Command::Map(MapCommand::Find)
        )
    )
}

/// Whether `a` and `b` are active together on one of `screens`: both on
/// its list, or, on a [`ROUTED`] screen, one of them the router's
/// ([`ROUTER_OVER`]).
#[must_use]
pub fn together_on(screens: &[ScreenContexts], a: Context, b: Context) -> bool {
    screens.iter().any(|screen| {
        let routed = ROUTED.contains(screen);
        let holds = |context| {
            screen.entries.iter().any(|entry| entry.context == context)
                || (routed && ROUTER_OVER.contains(&context))
        };
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
            assert_eq!(
                has_ui,
                !["flight", "intro"].contains(&screen.name),
                "{}",
                screen.name
            );
        }
    }

    #[test]
    fn every_screen_reads_the_app_s_context_last() {
        for screen in SCREENS {
            assert_eq!(
                screen.entries.last(),
                Some(&on(Context::App)),
                "{}",
                screen.name
            );
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
    fn the_router_s_acting_contexts_are_active_together_with_each_routed_screen_s() {
        for screen in ROUTED {
            for entry in screen.entries {
                for &router in ROUTER_OVER {
                    assert!(
                        together_on(&[*screen], router, entry.context),
                        "{router:?} on the {}",
                        screen.name
                    );
                }
            }
        }
        assert!(together(Context::MainScreen, Context::Map));
        assert!(
            !together(Context::MainMenu, Context::Map),
            "the router hands the main menu's keys on"
        );
        assert!(
            !together_on(&[COURSE_MAP], Context::MainScreen, Context::Map),
            "flight's map isn't routed"
        );
    }

    #[test]
    fn only_the_router_s_fly_shadows_by_design_and_only_the_viewer_s_find() {
        let fly = Command::MainScreen(MainScreenCommand::Fly);
        let find = Command::Map(MapCommand::Find);
        assert!(router_shadows(fly, find));
        assert!(!router_shadows(find, fly));
        assert!(!router_shadows(fly, Command::Map(MapCommand::Borders)));
        assert!(!router_shadows(
            Command::MainScreen(MainScreenCommand::About),
            find
        ));
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
