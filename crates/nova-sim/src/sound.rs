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
    /// A nav key was taken: Nav Off and Hyperspace Mode on every press
    /// (`_HandlePlayer` @0x69b19), a stellar key that found a stellar,
    /// even the one selected (@0x69ebc), and a click on a stellar not
    /// selected (@0x6a6c1): `_beepSnd[0]`, `snd ` 150.
    NavSelected,
    /// Hyper Select moved the selection through the system's links
    /// (`_HandlePlayer` @0x69c89): `_beepSnd[2]`, `snd ` 152. With no
    /// links it is silent.
    HyperSelected,
    /// Option-backquote let go of the ship target (`_HandlePlayer`
    /// @0x69a7d): `_beepSnd[0]`, `snd ` 150. With no target it is silent.
    TargetCleared,
}
