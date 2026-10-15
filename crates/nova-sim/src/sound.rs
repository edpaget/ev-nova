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
    /// An announced fleet arrived with its `flët`'s `Quote`:
    /// `_beepSnd[4]`, `snd ` 154 (`_HyperSpawnFleet` @0x42698-0x426bb),
    /// even when the line is empty.
    FleetArrivalQuote,
    /// A hail went unanswered, whatever the refusal, with no target too:
    /// `_beepSnd[3]`, `snd ` 153 (`_HandlePlayerCommunication` @0x626cf,
    /// @0x6206c, @0x6261c, @0x6264e; see
    /// [`HailRefusal`](crate::hail::HailRefusal)).
    HailRefused,
    /// A person's mission was offered on its hail, in place of the comm
    /// dialog: `_beepSnd[4]`, `snd ` 154 (`_HandlePlayerCommunication`
    /// @0x62232-0x6223d).
    HailOffer,
}
