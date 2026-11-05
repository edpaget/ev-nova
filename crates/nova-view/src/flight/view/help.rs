//! Flight's help line, a developer overlay the original doesn't draw:
//! each entry names its command's key as the player binds it, through
//! the one naming function ([`name_read`], [`Spelling::Bound`]).

use crate::bound_names::{Spelling, group, name_read};
use crate::keymap::{Command, CommandReader, FlightCommand as F, PauseCommand, PlayCommand};

/// What names an entry's keys.
enum Keys {
    /// One command's key.
    One(Command),
    /// Several commands' keys, written as one ([`group`]).
    Group(&'static [Command]),
    /// A key the line names that no command binds: the menu bar's
    /// Preferences chord.
    Literal(&'static str),
}

/// A flight command.
const fn flight(command: F) -> Command {
    Command::Flight(command)
}

/// The turn and thrust keys, named `Arrows` under the defaults.
const FLY: [Command; 4] = [
    flight(F::Accelerate),
    flight(F::ReverseCourse),
    flight(F::RotateLeft),
    flight(F::RotateRight),
];

/// The four escort orders' keys, named `F/D/V/C` under the defaults.
const ESCORT_ORDERS: [Command; 4] = [
    flight(F::AttackTarget),
    flight(F::DefendMe),
    flight(F::HoldPosition),
    flight(F::Recall),
];

/// Each entry of the line, in order: its keys and what they do.
const ENTRIES: [(Keys, &str); 21] = [
    (Keys::Group(&FLY), "fly"),
    (Keys::One(flight(F::Autopilot)), "face target"),
    (Keys::One(flight(F::FirePrimary)), "fire"),
    (Keys::One(flight(F::FireSecondary)), "secondary"),
    (Keys::One(flight(F::SelectSecondary)), "weapon"),
    (Keys::One(flight(F::TargetSelect)), "next target"),
    (Keys::One(flight(F::ClosestTarget)), "nearest"),
    (Keys::One(flight(F::Board)), "board"),
    (Keys::One(flight(F::Communicate)), "hail"),
    (Keys::One(flight(F::JettisonCargo)), "jettison"),
    (Keys::Group(&ESCORT_ORDERS), "escorts"),
    (Keys::One(flight(F::RecallToHangar)), "dock"),
    (Keys::One(flight(F::EscortMenu)), "escort menu"),
    (Keys::One(flight(F::LandDock)), "land"),
    (Keys::One(Command::Play(PlayCommand::GalaxyMap)), "map"),
    (Keys::One(flight(F::HyperJump)), "jump"),
    (Keys::One(flight(F::HyperSelect)), "next system"),
    (
        Keys::One(Command::Play(PlayCommand::PlayerInfo)),
        "player info",
    ),
    (
        Keys::One(Command::Play(PlayCommand::MissionInfo)),
        "mission info",
    ),
    // The menu bar's Preferences… chord, not a command (phase 2's fixed
    // list).
    (Keys::Literal("Cmd-,"), "preferences"),
    (Keys::One(Command::Pause(PauseCommand::Pause)), "leave"),
];

/// The gap between entries.
const GAP: &str = "   ";

/// Flight's help line, each entry naming its keys as `keys` binds them,
/// for the device `keys` reads: under the defaults and a US layout,
/// "Arrows: fly   A: face target   …   Esc: leave". An entry whose
/// commands are all unbound is left out.
#[must_use]
pub fn help_line(keys: &CommandReader) -> String {
    ENTRIES
        .iter()
        .filter_map(|(names, does)| {
            let named = match names {
                Keys::One(command) => name_read(keys, *command, Spelling::Bound),
                Keys::Group(commands) => group(
                    commands
                        .iter()
                        .map(|&command| name_read(keys, command, Spelling::Bound)),
                    "/",
                ),
                Keys::Literal(text) => Some((*text).to_owned()),
            };
            named.map(|named| format!("{named}: {does}"))
        })
        .collect::<Vec<_>>()
        .join(GAP)
}
