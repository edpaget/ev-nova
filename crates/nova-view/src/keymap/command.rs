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
}

context_commands! {
    /// Flight's commands: the `Keys.nib` commands `_HandlePlayer` reads,
    /// in `Keys.nib`'s order, then the ones the original keeps outside
    /// `_keyPrefs`.
    FlightCommand in Flight {
        /// Thrust (`accelKey`, `_keyPrefs`+0x2a).
        Accelerate: Held,
        /// Turn to face against the ship's motion (`reverseKey`, +0x2c).
        ReverseCourse: Held,
        /// Turn right (`rightTurn`, +0x28).
        RotateRight: Held,
        /// Turn left (`leftTurn`, +0x26).
        RotateLeft: Held,
        /// Engage the afterburner (`afterburner`, +0x30).
        Afterburner: Held,
        /// Turn to face the ship target, or the navigation stellar with
        /// none (`autopilot`, +0x0e).
        Autopilot: Held,
        /// Hyperspace Mode (`hyperMode`, +0x18), polled with a latch.
        HyperspaceMode: Held,
        /// Hyper Select (`hyperSel`, +0x1a), polled with a latch.
        HyperSelect: Held,
        /// Hyper Jump (`jumpKey`, +0x1c), polled with a latch.
        HyperJump: Held,
        /// Nav System Off (`navOff`, +0x10), polled with a latch.
        NavOff: Held,
        /// Hail the target (`hailKey`, +0x08).
        Communicate: Held,
        /// Land, or request clearance (`landReq`, +0x0a).
        LandDock: Held,
        /// Fire the primary weapons (`fireKey0`, +0x04).
        FirePrimary: Held,
        /// Fire the secondary weapon (`fireKey1`, +0x06).
        FireSecondary: Held,
        /// Select the next secondary weapon (`weapSelect`, +0x00).
        SelectSecondary: Held,
        /// Let go of the secondary weapon (`weapSafe`, +0x02).
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
        /// The escorts return to formation (`escortCmdKey3`, +0x66).
        Recall: Pressed,
        /// Clear the message line, close the escort menu and hide the
        /// floating map (`ackComm`, +0x0c).
        Acknowledge: Held,
        /// Board the target (`boardKey`, +0x20), polled with a latch.
        Board: Held,
        /// Jettison all the cargo, Command held (`dumpCargo`, +0x1e).
        JettisonCargo: Held,
        /// Eject in the escape pod, Command held (`ejectKey`, +0x22).
        Eject: Held,
        /// Self-destruct, Command held (`destructKey`, +0x24).
        SelfDestruct: Held,
        /// Cloak or uncloak, once a hold (`cloakKey`, +0x52).
        EngageCloak: Held,
        /// Toggle the frame-rate readout (`framerateKey`, +0x68).
        ShowFramerate: Pressed,
        /// Autopilot with Option: face the navigation stellar.
        FaceStellar: Held,
        /// Select Secondary with Option: the weapon before.
        SelectSecondaryBack: Held,
        /// Target Select with Shift: the ship before.
        TargetSelectBack: Held,
        /// Target Select with Option: the next escort.
        TargetEscort: Held,
        /// Target Select with Shift and Option: the escort before.
        TargetEscortBack: Held,
        /// Closest Target with Option: the nearest ship.
        ClosestShip: Held,
        /// Nav System Off with Option: let go of the ship target.
        ClearTarget: Held,
        /// Communicate with Option: hail the navigation stellar.
        HailStellar: Held,
        /// Recall with Option: Return to Hangar.
        RecallToHangar: Pressed,
        /// Jettison Cargo with Option too: all but the mission cargo.
        JettisonNonMission: Held,
        /// Select the stellar in the system's first nav slot.
        StellarSlot1: Held,
        /// Select the stellar in the second nav slot.
        StellarSlot2: Held,
        /// Select the stellar in the third nav slot.
        StellarSlot3: Held,
        /// Select the stellar in the fourth nav slot.
        StellarSlot4: Held,
        /// Select the nearest stellar.
        NearestStellar: Held,
        /// Zoom the floating map out.
        ZoomOut: Pressed,
        /// Zoom the floating map in.
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
