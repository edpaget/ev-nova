//! The sounds a flight session makes, as events: what happened, not what
//! to play. The audio side decides which `snd ` each one plays, if any.

use crate::catalog::SoundId;

/// Something a [`Session`](crate::Session) did that may make a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimSound {
    /// The ship began thrusting.
    ThrustStarted,
    /// The ship stopped thrusting: the player let go, or it landed or
    /// began a jump.
    ThrustStopped,
    /// The ship landed, on a stellar with this landing sound, if any.
    Landed {
        /// The stellar's own landing sound.
        stellar_sound: Option<SoundId>,
    },
    /// The ship took off.
    TookOff,
    /// A jump began.
    JumpBegan,
    /// A jump ended in the next system.
    Arrived,
    /// The player became threatened: the Red Alert, `snd ` 370
    /// (`_HandlePlayer` @0x68cdd-0x68cff; see
    /// [`red_alert`](crate::red_alert)).
    RedAlert,
    /// A `Q` set operator's message is shown in flight.
    ScriptMessage,
    /// Cargo was jettisoned in flight: `_beepSnd[4]`, `snd ` 154
    /// (`_JettisonCargo` @0x3e31a-0x3e33d), whether or not a mission
    /// failed with it (see [`cargo_box`](crate::cargo_box)).
    CargoJettisoned,
    /// A scan found a mission's cargo and failed the mission, the player
    /// told: `_beepSnd[1]`, `snd ` 151 (`_ScanPlayer` @0x7e721-0x7e740).
    /// A hidden mission (`Flags` 0x0400) fails in silence.
    ScanFailedMission,
    /// A scan caught the player smuggling a mission's cargo, a `jünk` or
    /// an outfit: `_beepSnd[4]`, `snd ` 154 (`_ScanPlayer`
    /// @0x7ea3e-0x7ea5d, @0x7ee76-0x7ee95, @0x7f337-0x7f356), with no fine
    /// too.
    Smuggling,
    /// A `P` set operator's sound plays: `exclusive`ly, as the one mission
    /// sound, dropped while the last is still playing; or over whatever
    /// plays.
    Script {
        /// The `snd `.
        sound: SoundId,
        /// Whether it plays only when the last mission sound is over.
        exclusive: bool,
    },
    /// The player's escort command changed some escort's order or target
    /// (`_IssueNewEscortCommand` @0x665be-0x665e1, `_beepSnd[1]`).
    EscortsCommanded,
    /// A disabled former escort or fighter repaired itself and rejoined
    /// the fleet (`_HandleShip` @0x33bb1-0x33bd4, `_beepSnd[4]`).
    EscortRepaired,
    /// The player's ship is breaking up: `_klaxxonSnd`, `snd ` 371
    /// (`_LoadSounds` @0x1c22c-0x1c23e). `_HandlePlayer` plays it
    /// (@0x6f1e4-0x6f227) while the death timer is above 1.0 and it is
    /// not already playing (`_ST_IsSoundPlaying` @0x6f200), so it repeats
    /// through the breakup; the audio side does that check. Its priority,
    /// 15, is not modelled. The player's final explosion halts it
    /// ([`PLAYER_EXPLOSION_HALTS`]).
    Klaxon,
    /// Each named `snd ` is halted if it is playing, as the original asks
    /// `ST_IsSoundPlaying` and then calls `ST_HaltSound`: every instance
    /// of it, on any channel. The player's final explosion raises it with
    /// [`PLAYER_EXPLOSION_HALTS`]; the `player-death` roadmap's phases 11
    /// and 14 raise it with 128 and 129.
    Halt(&'static [SoundId]),
}

/// The sounds the player's eject halts, in order, after its
/// [`SimSound::Ejected`]: the warp-up sounds `_warpUpSnd[0]` and `[1]`,
/// `snd ` 128 and 129 (`_HandlePlayer` @0x6f925-0x6f95a).
pub const EJECT_HALTS: [SoundId; 2] = [SoundId(128), SoundId(129)];

/// The sounds the player's final explosion halts, in order:
/// `_HandleShipDisplay`, for the player only (`cmpw $0x0,0x86(%edi)`
/// @0x2d86d), halts the klaxon `_klaxxonSnd`, `snd ` 371 (@0x2d891), the
/// warp-up sounds `warpUpSnd[0]` and `warpUpSnd[1]`, 128 (@0x2d8b0) and
/// 129 (@0x2d8cb), and the warp-out `warpOutSnd`, 130 (@0x2d8ea).
pub const PLAYER_EXPLOSION_HALTS: [SoundId; 4] =
    [SoundId(371), SoundId(128), SoundId(129), SoundId(130)];
