//! The device-free commands, grouped by the context that reads them.
//!
//! Nothing here names a device: a [`Command`] is what the player means,
//! and each device's table (the keyboard's is
//! [`KeyMap`](super::KeyMap)) says which of its inputs means it. Every
//! fact about a command is data on its enum: its [`Context`] and whether
//! it is [`Kind::Held`] or [`Kind::Pressed`]. So no screen, and no crate
//! above this one, matches every command: each screen matches its own
//! context's enum and passes over the rest.
//!
//! A context's enum is declared in the order phase 4's keys screen lists
//! it. Flight's follows `Keys.nib`'s, which is `_loadKeys`' too
//! (@0xcd5f8-0xce6c4): Navigation, Battle, Escort and Misc Controls, each
//! column top down, then the commands the original keeps outside
//! `_keyPrefs` (the variants a fixed modifier poll makes, the stellar
//! slots and the floating map's zoom). [`Command::KEYS_WINDOW`] holds the
//! 34 `Keys.nib` commands in that order, across their contexts.

/// Whether a command is read held or pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// True while its input is down, as the original polls it every frame
    /// (thrust, turn, fire, a modifier variant).
    Held,
    /// Acts once, on the press (land, hail, a dialog button).
    Pressed,
}

/// Declares one context's command enum, with its `ALL` list in
/// declaration order and each command's [`Kind`].
macro_rules! context_commands {
    (
        $(#[$meta:meta])*
        $name:ident in $context:ident {
            $( $(#[$vmeta:meta])* $variant:ident: $kind:ident, )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )*
        }

        impl $name {
            /// Every command of the context, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// Whether the command is read held or pressed.
            #[must_use]
            pub const fn kind(self) -> Kind {
                match self {
                    $(Self::$variant => Kind::$kind,)*
                }
            }

            /// The command's stable name, `Context.Variant`, as the
            /// settings file saves it. Renaming a variant renames its saved
            /// key, which the saved form's pinned table catches.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!(stringify!($context), ".", stringify!($variant)),)*
                }
            }
        }

        impl From<$name> for Command {
            fn from(command: $name) -> Self {
                Self::$context(command)
            }
        }
    };
}

/// Declares [`Context`] and [`Command`] over the contexts' enums.
macro_rules! contexts {
    ( $( $(#[$meta:meta])* $context:ident($commands:ident), )* ) => {
        /// A group of commands read together. A screen reads an ordered
        /// list of contexts (the `contexts` module's lists), and two
        /// contexts are active together when some list holds both.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Context {
            $( $(#[$meta])* $context, )*
        }

        impl Context {
            /// Every context, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$context),*];

            /// The context's commands, in declaration order.
            pub fn commands(self) -> impl Iterator<Item = Command> {
                let commands: Box<dyn Iterator<Item = Command>> = match self {
                    $(Self::$context => Box::new(
                        $commands::ALL.iter().map(|&command| Command::$context(command)),
                    ),)*
                };
                commands
            }
        }

        /// A device-free command: what the player means, whichever device
        /// said it.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Command {
            $( $(#[$meta])* $context($commands), )*
        }

        impl Command {
            /// The context the command belongs to.
            #[must_use]
            pub const fn context(self) -> Context {
                match self {
                    $(Self::$context(_) => Context::$context,)*
                }
            }

            /// Whether the command is read held or pressed.
            #[must_use]
            pub const fn kind(self) -> Kind {
                match self {
                    $(Self::$context(command) => command.kind(),)*
                }
            }

            /// The command's stable name, `Context.Variant`
            /// (`"Flight.Accelerate"`), as the settings file saves it.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$context(command) => command.name(),)*
                }
            }

            /// The command named `name` ([`Command::name`]), if any.
            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                Self::all().find(|command| command.name() == name)
            }

            /// Every command, context by context, each in declaration
            /// order.
            pub fn all() -> impl Iterator<Item = Self> {
                Context::ALL.iter().flat_map(|context| context.commands())
            }
        }
    };
}

contexts! {
    /// Flight's own commands, read by `_HandlePlayer`.
    Flight(FlightCommand),
    /// The escort menu's group keys, which shadow flight's stellar slots
    /// while the menu shows.
    EscortMenu(EscortMenuCommand),
    /// The map, player info and mission info keys, which flight and the
    /// spaceport's screens and dialogs all read (`mapKey`,
    /// `playerInfoKey`, `missionInfoKey`).
    Play(PlayCommand),
    /// The Pause key, which flight and the intro read (`pauseKey`).
    Pause(PauseCommand),
    /// The shared UI commands every screen but flight reads: Confirm,
    /// Back, the arrows, Tab, Space and the scroll keys.
    Ui(UiCommand),
    /// The comm dialog's own letter (`_CommFilter` @0x908ed).
    Comm(CommCommand),
    /// The haggle dialog's letters (`_HaggleFilter` @0x91d9d).
    Haggle(HaggleCommand),
    /// The planet comm dialog's letters (`_PlanetCommFilter` @0x911a9).
    PlanetComm(PlanetCommCommand),
    /// The escort dialog's letters (`_EscortFilter` @0x950e9).
    EscortDialog(EscortDialogCommand),
    /// The captured ship's assignment's letters
    /// (`_CapturedAssignmentFilter` @0x51782).
    Assignment(AssignmentCommand),
    /// The player info screen's own (`_PlayerInfoFilter` @0x53ae9).
    PlayerInfo(PlayerInfoCommand),
    /// The spaceport's letters (`_PortFilter` @0x4f209-0x4f257).
    Port(PortCommand),
    /// The Trade Center's (`_TradeFilter`, `_DoTradeDialog`).
    Trade(TradeCommand),
    /// The outfitter's (`_OutfitFilter`, `_DoOutfitDialog`).
    Outfitter(OutfitterCommand),
    /// The shipyard's own letter (`_ShipyardFilter`).
    Shipyard(ShipyardCommand),
    /// The bar's letters (`_BarFilter` @0x47b1c-0x47b48).
    Bar(BarCommand),
    /// The hire screen's own letter (`_ShipyardFilter` in hire mode).
    Hire(HireCommand),
    /// The mission computer's letters (`_MissionFilter`).
    MissionComputer(MissionComputerCommand),
    /// The bar race's Option (`_DoRaceDialog`).
    Race(RaceCommand),
    /// The galaxy map's own keys (`_MapFilter`, `_DoSystemMap`).
    Map(MapCommand),
    /// The intro screen's Return and Space (`_ShowIntroScreen` @0x16cf1).
    Intro(IntroCommand),
    /// The system view's WASD, the fixed twins of the shared arrows.
    SystemView(SystemViewCommand),
    /// The app's own, which its router reads on every screen, flight
    /// included: Caps Lock's double time.
    App(AppCommand),
    /// The main screen's keys the app's router reads on the main menu and
    /// the developer's sides: Fly, About and Preferences.
    MainScreen(MainScreenCommand),
    /// The main menu's keys the app's router reads beside the menu's
    /// typed letters (`_MainScreenKey` @0x1aba0).
    MainMenu(MainMenuCommand),
}

context_commands! {
    /// Flight's commands: the `Keys.nib` commands `_HandlePlayer` reads,
    /// in `Keys.nib`'s order, then the ones the original keeps outside
    /// `_keyPrefs`.
    ///
    /// The held ones are those flight tracks as held: `_HandlePlayer`
    /// polls each every frame, setting the cursor's fade
    /// (`_keyPressedThisFrame` *0x3b5550) on every frame one is down and
    /// counting it for `_quickKeyCheckResult`, which the floating map's
    /// zoom reads; the pressed ones act only as they go down. Which held
    /// ones wait on a latch, and which are read live across a close, is
    /// the flight view's `map_wait` module's.
    FlightCommand in Flight {
        /// Thrust (`accelKey`, `_keyPrefs`+0x2a), held.
        Accelerate: Held,
        /// Turn to face against the ship's motion (`reverseKey`, +0x2c),
        /// held.
        ReverseCourse: Held,
        /// Turn right (`rightTurn`, +0x28), held.
        RotateRight: Held,
        /// Turn left (`leftTurn`, +0x26), held.
        RotateLeft: Held,
        /// Engage the afterburner (`afterburner`, +0x30, `_loadKeys`
        /// @0xcd899-0xcd8bc), held. It fades the cursor, as its press sets
        /// *0x3b5550 whether or not the ship carries an afterburner, on a
        /// ship not disabled (@0x6caf2, @0x6cb03).
        Afterburner: Held,
        /// Turn to face the ship target, or the navigation stellar with
        /// none (`autopilot`, +0x0e, `_loadKeys` @0xcd8c5-0xcd935), held.
        /// It fades the cursor, as its press sets *0x3b5550 (@0x6aeeb).
        Autopilot: Held,
        /// Hyperspace Mode (`hyperMode`, +0x18, key code 0x04), polled
        /// with a latch: puts the nav system in hyperspace mode and selects
        /// the course's next hop (`_HandlePlayer` @0x69b76-0x69b89). Held
        /// outside a jump, it shows the floating map (@0x69b8e-0x69ba8).
        /// Held through a jump's end, it acts as a press on the first frame
        /// after it, as its latch is cleared on every frame of a jump.
        HyperspaceMode: Held,
        /// Hyper Select (`hyperSel`, +0x1a, key code 0x2a), polled with a
        /// latch: cycles the hyperspace selection through the systems the
        /// current one links to, the course kept by the engine's reading.
        /// Held in nav mode 3, during a jump too, it shows the floating map
        /// (@0x69cdd-0x69ce7, @0x69be7). One first pressed during a jump,
        /// or let go and pressed again then, and held past its end selects
        /// on the first frame after it; one held from before the jump does
        /// nothing after it.
        HyperSelect: Held,
        /// Hyper Jump (`jumpKey`, +0x1c), polled with a latch.
        HyperJump: Held,
        /// Nav System Off (`navOff`, +0x10, key code 0x32), polled with a
        /// latch: turns the nav system off, the course kept (`_HandlePlayer`
        /// @0x69a94-0x69b63). It shares Hyperspace Mode's latch: held
        /// through a jump's end it turns the nav off after it, and Option
        /// let go with it still down turns the nav off. While the dev
        /// overlay is built in, its default key toggles the overlay instead
        /// and never reaches flight.
        NavOff: Held,
        /// Hail the target (`hailKey`, +0x08), one hail a press.
        Communicate: Held,
        /// Land, or request clearance (`landReq`, +0x0a; `STR#` 129 and
        /// `STR#` 2002 #25 name it).
        LandDock: Held,
        /// Fire the primary weapons (`fireKey0`, +0x04), held.
        FirePrimary: Held,
        /// Fire the secondary weapon (`fireKey1`, +0x06), held.
        FireSecondary: Held,
        /// Select the next secondary weapon (`weapSelect`, +0x00), in
        /// `wëap` ID order. Held, it fades the cursor, as `_HandlePlayer`
        /// sets *0x3b5550 on every pass it is down (@0x6ac2a).
        SelectSecondary: Held,
        /// Let go of the secondary weapon (`weapSafe`, +0x02, key code
        /// 0x01, `_loadKeys` @0xcdde9-0xcde0c), held: on every pass with
        /// one selected (`_HandlePlayer` @0x6adc2-0x6ae1b), fading the
        /// cursor only on those passes (@0x6ade1).
        WeaponSafety: Held,
        /// Target the next ship (`targSel`, +0x14).
        TargetSelect: Held,
        /// Target the nearest threat (`closeTarg`, +0x16).
        ClosestTarget: Held,
        /// Open or close the escort menu (`escortMenuKey`, +0x54).
        EscortMenu: Pressed,
        /// The escorts attack the target (`escortCmdKey0`, +0x60).
        AttackTarget: Pressed,
        /// The escorts defend the player (`escortCmdKey1`, +0x62).
        DefendMe: Pressed,
        /// The escorts hold position (`escortCmdKey2`, +0x64).
        HoldPosition: Pressed,
        /// The escorts return to formation (`escortCmdKey3`, +0x66,
        /// "Recall: (option = dock)").
        Recall: Pressed,
        /// Clear the message line, close the escort menu and hide the
        /// floating map (`ackComm`, +0x0c).
        Acknowledge: Held,
        /// Board the target (`boardKey`, +0x20), one attempt a press,
        /// polled with a latch.
        Board: Held,
        /// Jettison all the cargo, Command held (`dumpCargo`, +0x1e, key
        /// code 0x28, `_loadKeys` @0xce33d-0xce360): `_HandlePlayer` reads
        /// it only with Command down (@0x6fd54-0x6fdaf), on every pass it
        /// is held. It works during a jump too: `_JettisonCargo`
        /// (0x3dffd-0x3e3c4) reads no jump state.
        JettisonCargo: Held,
        /// Eject in the escape pod, Command held (`ejectKey`, +0x22, key
        /// code 0x07): `_HandlePlayer` reads it only with Command down
        /// (@0x6f29e-0x6f2c1).
        Eject: Held,
        /// Self-destruct, Command held (`destructKey`, +0x24), which
        /// `_loadKeys` reads (@0xce458) and then forces to key code 0x1b
        /// (@0xce71c, left out: the phase's findings). `_HandlePlayer`
        /// reads it only with Command down (@0x6fdb4-0x6fddb), so it never
        /// clashes with the floating map's zoom out on the same key, which
        /// refuses any press with Command held (@0x70fa7-0x70fb5): a
        /// Command-`-` press means this. The keypad's `-` (0x4e) arrives
        /// as the same key, as for the zoom out.
        SelfDestruct: Held,
        /// Cloak or uncloak, once a hold (`cloakKey`, +0x52, key code
        /// 0x20, `_loadKeys` @0xce461-0xce4d4), with no Command
        /// (`_HandlePlayer` @0x705b7).
        EngageCloak: Held,
        /// Toggle the frame-rate readout (`framerateKey`, +0x68, F12, key
        /// code 0x6f, `_loadKeys` @0xce6a1-0xce6c4): a press, no modifier
        /// read (`_HandlePlayer` @0x6e454-0x6e4bd), during a jump too.
        ShowFramerate: Pressed,
        /// Autopilot with Option: face the navigation stellar.
        FaceStellar: Held,
        /// Select Secondary with Option: the weapon before, in `wëap` ID
        /// order.
        SelectSecondaryBack: Held,
        /// Target Select with Shift: the ship before.
        TargetSelectBack: Held,
        /// Target Select with Option: the next escort.
        TargetEscort: Held,
        /// Target Select with Shift and Option: the escort before.
        TargetEscortBack: Held,
        /// Closest Target with Option: the nearest ship.
        ClosestShip: Held,
        /// Nav System Off with Option: let go of the ship target
        /// (@0x69a17-0x69a90), during a jump too. It shares Nav System
        /// Off's latch, so held it lets go of a target picked meanwhile on
        /// the next frame.
        ClearTarget: Held,
        /// Communicate with Option: hail the navigation stellar.
        HailStellar: Held,
        /// Recall with Option: Return to Hangar.
        RecallToHangar: Pressed,
        /// Jettison Cargo with Option too: all but the mission cargo.
        JettisonNonMission: Held,
        /// Select the stellar in the system's first nav slot: key code
        /// 0x12, with F1 its twin (`_loadKeys` hard-codes F1-F4 at
        /// +0x34..+0x3a, @0xce6c8-0xce6da). `_HandlePlayer` reads the
        /// slots, the nearest stellar's keys after them, as one block with
        /// one latch (@0x69d2b-0x69f17, the flight view's `slot_block`
        /// module). While the escort menu shows, `_QuickKeyCheck` hides
        /// 1-4 from the block (@0xa106-0xa124) and they pick its groups
        /// instead ([`EscortMenuCommand`]); the F keys always pick a
        /// stellar.
        StellarSlot1: Held,
        /// Select the stellar in the second nav slot (0x13, F2 its twin).
        StellarSlot2: Held,
        /// Select the stellar in the third nav slot (0x14, F3 its twin).
        StellarSlot3: Held,
        /// Select the stellar in the fourth nav slot (0x15, F4 its twin).
        StellarSlot4: Held,
        /// Select the nearest stellar, as the land key would with nothing
        /// selected: Command and the top-row 5 (0x37, 0x17, @0x69dbc-0x69dda),
        /// with F5 (0x60, @0x69dac) its twin. While the escort menu shows,
        /// `_QuickKeyCheck` hides the 5 from the block (@0xa106-0xa124), so
        /// Command-5 picks the menu's Freighters; 5 alone picks nothing.
        NearestStellar: Held,
        /// Zoom the floating map out (`-` and the keypad's `-`, key codes
        /// 0x1b and 0x4e, `_HandlePlayer` @0x70f4e-0x71110, which the
        /// platform gives as one key).
        ZoomOut: Pressed,
        /// Zoom the floating map in (`=`, key code 0x18, the keypad's `+`,
        /// 0x45, its twin).
        ZoomIn: Pressed,
    }
}

context_commands! {
    /// The escort menu's group keys (`_keyPrefs`+0x56..+0x5e, which
    /// `_loadKeys` writes after the prefs, @0xce722-0xce73a).
    EscortMenuCommand in EscortMenu {
        /// Select the menu's first group.
        Group1: Held,
        /// Select the second group.
        Group2: Held,
        /// Select the third group.
        Group3: Held,
        /// Select the fourth group.
        Group4: Held,
        /// Select the fifth group, Freighters.
        Group5: Held,
    }
}

context_commands! {
    /// The keys flight and the spaceport's screens and dialogs all read.
    PlayCommand in Play {
        /// Open or close the map (`mapKey`, `_keyPrefs`+0x12).
        GalaxyMap: Held,
        /// Open the player info (`playerInfoKey`, +0x32).
        PlayerInfo: Held,
        /// Open the mission info (`missionInfoKey`, +0x50).
        MissionInfo: Held,
    }
}

context_commands! {
    /// The Pause key.
    PauseCommand in Pause {
        /// Pause the game (`pauseKey`, `_keyPrefs`+0x2e), or skip the
        /// intro.
        Pause: Pressed,
    }
}

context_commands! {
    /// The shared UI commands.
    UiCommand in Ui {
        /// Activate the default button.
        Confirm: Pressed,
        /// Activate the cancel button, or leave.
        Back: Pressed,
        /// Move up.
        Up: Pressed,
        /// Move down.
        Down: Pressed,
        /// Move left.
        Left: Pressed,
        /// Move right.
        Right: Pressed,
        /// Move the focus to the next field.
        Tab: Pressed,
        /// Activate the focused item.
        Activate: Pressed,
        /// Scroll to the top.
        Home: Pressed,
        /// Scroll to the bottom.
        End: Pressed,
        /// Scroll up a page.
        PageUp: Pressed,
        /// Scroll down a page.
        PageDown: Pressed,
    }
}

context_commands! {
    /// The comm dialog's own letter. Its hail options' letters come from
    /// their buttons' titles, so they are no commands: the dialog reads
    /// them from the key pressed, as text entry is.
    CommCommand in Comm {
        /// Close the channel (E, beside Return and Escape).
        Close: Pressed,
    }
}

context_commands! {
    /// The haggle dialog's letters.
    HaggleCommand in Haggle {
        /// Accept the price (A, beside Return).
        Accept: Pressed,
        /// Lower the price (L).
        Lower: Pressed,
    }
}

context_commands! {
    /// The planet comm dialog's letters, one a button; each presses its
    /// button only while the dialog shows it.
    PlanetCommCommand in PlanetComm {
        /// Close the channel (E, beside Return and Escape).
        Close: Pressed,
        /// Greetings (G).
        Greetings: Pressed,
        /// Offer Bribe (O, B its twin).
        OfferBribe: Pressed,
        /// Demand Tribute (D, T its twin).
        DemandTribute: Pressed,
        /// Release (R).
        Release: Pressed,
    }
}

context_commands! {
    /// The escort dialog's letters.
    EscortDialogCommand in EscortDialog {
        /// Close the channel (C, beside Return and Escape).
        Close: Pressed,
        /// Release the escort (R).
        Release: Pressed,
        /// Upgrade the escort, or cancel the upgrade (U).
        Upgrade: Pressed,
        /// Sell the escort, or cancel the sale (S).
        Sell: Pressed,
    }
}

context_commands! {
    /// The captured ship's assignment's letters.
    AssignmentCommand in Assignment {
        /// Use As My Ship (M).
        MyShip: Pressed,
        /// Use As Escort (E).
        Escort: Pressed,
    }
}

context_commands! {
    /// The player info screen's own commands.
    PlayerInfoCommand in PlayerInfo {
        /// The previous tab: the shared Tab with Shift, as
        /// `_PlayerInfoFilter` reads the `shiftKey` bit beside Tab
        /// (@0x53b1e), so a rebound Tab keeps it.
        TabBack: Pressed,
    }
}

context_commands! {
    /// The spaceport's letters, one a button; each presses its button as
    /// a click does. Their second letters are fixed twins.
    PortCommand in Port {
        /// Recharge (R, F its twin).
        Recharge: Pressed,
        /// The Trade Center (C, T its twin).
        TradeCenter: Pressed,
        /// The Outfitter (O).
        Outfitter: Pressed,
        /// The Shipyard (S).
        Shipyard: Pressed,
        /// The Mission BBS (N).
        MissionBbs: Pressed,
        /// The Bar (B).
        Bar: Pressed,
    }
}

context_commands! {
    /// The Trade Center's own commands.
    TradeCommand in Trade {
        /// Buy (B), repeats too.
        Buy: Pressed,
        /// Sell (S), repeats too.
        Sell: Pressed,
        /// Held, Buy and Sell ask for a count or trade the most: Option,
        /// as `_DoTradeDialog` reads the `optionKey` bit (@0x5e211,
        /// @0x5e45c), so the right Option is its twin.
        MaxLot: Held,
        /// The selection back a row: the shared Tab with Shift, as
        /// `_TradeFilter` reads the `shiftKey` bit beside Tab (@0x4e18a),
        /// so a rebound Tab keeps it.
        TabBack: Pressed,
    }
}

context_commands! {
    /// The outfitter's own commands.
    OutfitterCommand in Outfitter {
        /// Buy one (B), repeats too.
        Buy: Pressed,
        /// Sell one (S), repeats too.
        Sell: Pressed,
        /// Held, Buy and Sell ask for a count: Option, as
        /// `_DoOutfitDialog` tests the `optionKey` bit (@0x5c0c0,
        /// @0x5c5e3), so the right Option is its twin.
        Count: Held,
    }
}

context_commands! {
    /// The shipyard's own letter; its Info is the shared Activate (Space,
    /// `_ShipyardFilter` @0x60526).
    ShipyardCommand in Shipyard {
        /// Buy Ship (B).
        Buy: Pressed,
    }
}

context_commands! {
    /// The bar's letters, one a button; their second letters are fixed
    /// twins.
    BarCommand in Bar {
        /// Hire Escort (H, E its twin).
        Hire: Pressed,
        /// Gamble (G, R its twin).
        Gamble: Pressed,
        /// Holovid, the news (N, W its twin).
        News: Pressed,
    }
}

context_commands! {
    /// The hire screen's own letter; its Info is the shared Activate.
    HireCommand in Hire {
        /// Hire Escort (H, `_ShipyardFilter` @0x60503).
        Hire: Pressed,
    }
}

context_commands! {
    /// The mission computer's letters.
    MissionComputerCommand in MissionComputer {
        /// Leave (L), beside Confirm and Back (`_MissionFilter`
        /// @0x9f885-0x9f8af).
        Leave: Pressed,
        /// Accept the selected offer (A, @0x9f8b4-0x9f8c7).
        Accept: Pressed,
    }
}

context_commands! {
    /// The bar race's own command.
    RaceCommand in Race {
        /// Held as a racer is clicked, asks for the bet: the left Option,
        /// as `_DoRaceDialog` polls its key code (`_KeyCheck(0x3a)`
        /// @0x48be9), so it has no right twin.
        Ask: Held,
    }
}

context_commands! {
    /// The galaxy map's own commands, which `_MapFilter` reads on the
    /// in-flight maps (@0x13cdd-0x144d6) and the viewer reads likewise.
    /// Its arrows, Return, Space and Tab are the shared UI context's, and
    /// the map key that closes it the shared play keys'.
    MapCommand in Map {
        /// Flip the government borders, as the Borders button does (B,
        /// its repeats too, @0x14251).
        Borders: Pressed,
        /// Ask to clear the route, as Clear Route does (C, its repeats
        /// too, @0x14268).
        ClearRoute: Pressed,
        /// Open Find, as the Find button does (F, S its twin,
        /// @0x144b9-0x144d6).
        Find: Pressed,
        /// Step the zoom out, item 4 (`-`, @0x14271-0x14293).
        ZoomOut: Pressed,
        /// Step the zoom in, item 5 (`=`, the keypad's `+` its twin).
        ZoomIn: Pressed,
        /// Cycle the hypergate map's selection back: the shared Tab with
        /// Shift, as `_MapFilter` reads the `shiftKey` bit beside Tab and
        /// `\` (@0x13ee4), so a rebound Tab keeps it.
        TabBack: Pressed,
        /// Held as a system is clicked, the click edits the course: the
        /// left Shift, as `_DoSystemMap` polls its key code
        /// (`_KeyCheck(0x38)` @0x13071), so it has no right twin.
        PlotCourse: Held,
    }
}

context_commands! {
    /// The intro screen's own keys, which `_ShowIntroScreen` polls by key
    /// code beside the Pause key (@0x16ef2, @0x16fc3, @0x17034): Return
    /// alone, with no keypad Enter twin, and Space. The intro reads no
    /// shared UI key.
    IntroCommand in Intro {
        /// End the picture showing (Return, 0x24).
        Confirm: Held,
        /// End the picture showing (Space, 0x31).
        Activate: Held,
    }
}

context_commands! {
    /// The system view's own commands: none. Its context holds W, A, S
    /// and D, the fixed twins of the shared arrows it pans with.
    #[allow(clippy::empty_enums, reason = "a context of twins alone")]
    SystemViewCommand in SystemView {}
}

context_commands! {
    /// The app's own commands, read by its router whatever is shown.
    AppCommand in App {
        /// Turn double time on or off, on each press (Caps Lock, whose
        /// lock state the original follows, `_QuickKeyCheck(0x39)`
        /// @0x463c3).
        DoubleTime: Pressed,
    }
}

context_commands! {
    /// The main screen's keys the app's router reads on the main menu,
    /// the ship browser and the galaxy viewer. They are the engine's own
    /// but for Preferences, which on the main menu is `_MainScreenKey`'s
    /// P (@0x1ac59).
    MainScreenCommand in MainScreen {
        /// Enter flight from the ship browser or the galaxy viewer (F).
        Fly: Pressed,
        /// Open the About text (I).
        About: Pressed,
        /// Open the Preferences (P).
        Preferences: Pressed,
    }
}

context_commands! {
    /// The main menu's keys the app's router reads beside the menu's own
    /// letters, which it reads as typed characters (`_MainScreenKey`
    /// @0x1aba0).
    MainMenuCommand in MainMenu {
        /// Show the About text, after a click (A, @0x1ac70).
        About: Pressed,
        /// Show `dësc` 32766, after a click (X, @0x1ac7e).
        Text: Pressed,
        /// Held, Open Pilot does nothing: the S key, which
        /// `_MainScreenKey` polls by key code (`_KeyCheck(0x01)`
        /// @0x1abd3). No other code reads it, so its purpose is unknown.
        OpenGuardS: Held,
        /// Held, Open Pilot does nothing: the L key (`_KeyCheck(0x25)`
        /// @0x1abf5), beside S.
        OpenGuardL: Held,
        /// Held as Enter Ship is typed, the choice waits for its release
        /// (the E key, `_QuickKeyCheck(0x0e)` @0x1ac32-0x1ac57).
        EnterShip: Held,
    }
}

impl Command {
    /// The 34 commands `Keys.nib` offers, in its order, which is
    /// `_loadKeys`' too: Navigation, Battle, Escort and Misc Controls.
    pub const KEYS_WINDOW: [Self; 34] = [
        Self::Flight(FlightCommand::Accelerate),
        Self::Flight(FlightCommand::ReverseCourse),
        Self::Flight(FlightCommand::RotateRight),
        Self::Flight(FlightCommand::RotateLeft),
        Self::Flight(FlightCommand::Afterburner),
        Self::Flight(FlightCommand::Autopilot),
        Self::Flight(FlightCommand::HyperspaceMode),
        Self::Flight(FlightCommand::HyperSelect),
        Self::Flight(FlightCommand::HyperJump),
        Self::Flight(FlightCommand::NavOff),
        Self::Flight(FlightCommand::Communicate),
        Self::Flight(FlightCommand::LandDock),
        Self::Flight(FlightCommand::FirePrimary),
        Self::Flight(FlightCommand::FireSecondary),
        Self::Flight(FlightCommand::SelectSecondary),
        Self::Flight(FlightCommand::WeaponSafety),
        Self::Flight(FlightCommand::TargetSelect),
        Self::Flight(FlightCommand::ClosestTarget),
        Self::Flight(FlightCommand::EscortMenu),
        Self::Flight(FlightCommand::AttackTarget),
        Self::Flight(FlightCommand::DefendMe),
        Self::Flight(FlightCommand::HoldPosition),
        Self::Flight(FlightCommand::Recall),
        Self::Pause(PauseCommand::Pause),
        Self::Flight(FlightCommand::Acknowledge),
        Self::Flight(FlightCommand::Board),
        Self::Flight(FlightCommand::JettisonCargo),
        Self::Flight(FlightCommand::Eject),
        Self::Flight(FlightCommand::SelfDestruct),
        Self::Flight(FlightCommand::EngageCloak),
        Self::Play(PlayCommand::GalaxyMap),
        Self::Play(PlayCommand::PlayerInfo),
        Self::Play(PlayCommand::MissionInfo),
        Self::Flight(FlightCommand::ShowFramerate),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_reports_its_own_context() {
        for &context in Context::ALL {
            for command in context.commands() {
                assert_eq!(command.context(), context, "{command:?}");
            }
        }
        assert_eq!(
            Command::from(FlightCommand::Eject).context(),
            Context::Flight
        );
        assert_eq!(Command::from(UiCommand::Back).context(), Context::Ui);
        assert_eq!(
            Command::from(EscortMenuCommand::Group5).context(),
            Context::EscortMenu
        );
    }

    #[test]
    fn all_lists_every_context_s_commands_once() {
        let all: Vec<Command> = Command::all().collect();
        let counted: usize = Context::ALL.iter().map(|c| c.commands().count()).sum();
        assert_eq!(all.len(), counted);
        let unique: std::collections::BTreeSet<Command> = all.iter().copied().collect();
        assert_eq!(unique.len(), all.len());
        assert_eq!(
            all.first(),
            Some(&Command::Flight(FlightCommand::Accelerate))
        );
    }

    #[test]
    fn a_command_the_original_polls_is_held_and_one_it_acts_on_once_is_pressed() {
        for held in [
            FlightCommand::Accelerate,
            FlightCommand::RotateLeft,
            FlightCommand::FirePrimary,
            FlightCommand::FireSecondary,
            FlightCommand::Eject,
            FlightCommand::FaceStellar,
            FlightCommand::JettisonNonMission,
        ] {
            assert_eq!(Command::from(held).kind(), Kind::Held, "{held:?}");
        }
        for pressed in [
            FlightCommand::ShowFramerate,
            FlightCommand::EscortMenu,
            FlightCommand::Recall,
            FlightCommand::RecallToHangar,
            FlightCommand::ZoomIn,
        ] {
            assert_eq!(Command::from(pressed).kind(), Kind::Pressed, "{pressed:?}");
        }
        assert_eq!(Command::from(UiCommand::Confirm).kind(), Kind::Pressed);
        assert_eq!(Command::from(PauseCommand::Pause).kind(), Kind::Pressed);
        assert_eq!(Command::from(PlayCommand::GalaxyMap).kind(), Kind::Held);
        assert_eq!(Command::from(EscortMenuCommand::Group1).kind(), Kind::Held);
    }

    #[test]
    fn every_command_has_a_unique_name_that_finds_it_again() {
        let names: std::collections::BTreeSet<&str> = Command::all().map(Command::name).collect();
        assert_eq!(names.len(), Command::all().count());
        for command in Command::all() {
            assert_eq!(
                Command::from_name(command.name()),
                Some(command),
                "{command:?}"
            );
        }
    }

    #[test]
    fn a_command_s_name_is_its_context_and_variant() {
        assert_eq!(
            Command::from(FlightCommand::Accelerate).name(),
            "Flight.Accelerate"
        );
        assert_eq!(Command::from(PauseCommand::Pause).name(), "Pause.Pause");
        assert_eq!(Command::from(UiCommand::Back).name(), "Ui.Back");
        assert_eq!(
            Command::from(EscortMenuCommand::Group5).name(),
            "EscortMenu.Group5"
        );
        assert_eq!(FlightCommand::Board.name(), "Flight.Board");
    }

    #[test]
    fn a_name_no_command_has_finds_none() {
        for name in [
            "",
            "Flight.Nope",
            "flight.accelerate",
            "Flight.",
            "Accelerate",
            "Flight.Accelerate ",
        ] {
            assert_eq!(Command::from_name(name), None, "{name:?}");
        }
    }

    #[test]
    fn the_keys_window_lists_34_distinct_commands_in_keys_nib_s_order() {
        let unique: std::collections::BTreeSet<Command> =
            Command::KEYS_WINDOW.iter().copied().collect();
        assert_eq!(unique.len(), 34);
        assert_eq!(
            Command::KEYS_WINDOW[0],
            Command::Flight(FlightCommand::Accelerate)
        );
        assert_eq!(
            Command::KEYS_WINDOW[23],
            Command::Pause(PauseCommand::Pause)
        );
        assert_eq!(
            Command::KEYS_WINDOW[33],
            Command::Flight(FlightCommand::ShowFramerate)
        );
    }
}
