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
    /// The player's ship was newly disabled or newly started to break up:
    /// `_beepSnd[1]`, `snd ` 151 (`_DamageShip` @0x3b11e, @0x3b221; see
    /// the session's `player_downed` module).
    ShipDowned,
    /// The player's ship being disabled or breaking up failed a `Flags2`
    /// 0x0004 mission: `_beepSnd[1]`, `snd ` 151 (`_DamageShip`
    /// @0x3b04e, @0x3b2f1), hidden or not.
    DownedMissionFailed,
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
    /// A first-flight tutorial hint was shown: `_beepSnd[4]`, `snd ` 154
    /// (`_HandlePlayer` @0x70a1c, @0x70c12, @0x70dab, @0x70e59; see
    /// [`tutorial`](crate::tutorial)).
    TutorialHint,
    /// A key's action was refused and the player told why: `_beepSnd[3]`,
    /// `snd ` 153, the original's shared refusal beep. The jump key's
    /// refusals raise it (`_HandlePlayer` @0x6b89f, @0x6b9cc, @0x6ba72;
    /// see [`Session::jump_refusal_told`](crate::Session::jump_refusal_told)),
    /// and rdm:phase/player-death/phase-16-handle-player-left-out-gates
    /// reuses it for its own refusals.
    Refused,
}
