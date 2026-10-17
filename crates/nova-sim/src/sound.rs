//! The sounds a flight session makes, as events: what happened, not what
//! to play. The audio side decides which `snd ` each one plays, if any.

use crate::catalog::SoundId;

/// Something a `nova_sim::Session` did that may make a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimSound {
    /// The ship began thrusting.
    ThrustStarted,
    /// The ship stopped thrusting: the player let go, or it landed or
    /// began a jump.
    ThrustStopped,
    /// The ship landed. The stellar's own sound is its spaceport's
    /// ambient sound, which the spaceport plays once it is frontmost, not
    /// at touchdown (`_DoPortDialog` @0x5fa50, `_PortFilter` @0x4f041).
    Landed,
    /// The ship took off.
    TookOff,
    /// The warp stage of a jump began out of double time, as the brake
    /// ended or at once when the ship needed none: the warp-up,
    /// `_warpUpSnd[0]`, `snd ` 128 "Warp up" (`_HandlePlayer`
    /// @0x6b645-0x6b690), before the turn to the jump's bearing. None
    /// sounds while 128 still plays from an earlier start: the
    /// still-playing skip (@0x6b645-0x6b658), which the session decides
    /// (see the [`warp`](crate::warp) module's `WarpUps`).
    JumpBegan,
    /// The warp stage of a jump began in double time: the double-time
    /// warp-up, `_warpUpSnd[1]`, `snd ` 129 "Warp up x2" (`_HandlePlayer`
    /// @0x6b60d-0x6b63f, @0x6b66f-0x6b690), in place of
    /// [`SimSound::JumpBegan`], with its own still-playing skip
    /// (@0x6b617-0x6b62b).
    JumpBeganInDoubleTime,
    /// A jump ended in the next system: the warp-out, `snd ` 130
    /// (`_HandlePlayer` @0x6bbd3-0x6bc08; the hypergate @0x63e57 and the
    /// wormhole @0x6443d play it too). A jump's warp end first halts the
    /// warp-ups, a [`SimSound::Halt`] of [`WARP_UP_HALTS`] just before it
    /// (@0x6c6b7, @0x6c6d2); a gate or wormhole exit halts nothing. The
    /// original's fallback end starts it a frame before the system switch
    /// (@0x6c706); that lag isn't visible to the player. A jump's natural
    /// end, both warp-ups finished, raises none while the last warp-out
    /// still plays (@0x6bbe1-0x6bbe9; see
    /// [`WarpOut`](crate::warp::WarpOut)).
    Arrived,
    /// The player became threatened: the Red Alert, `snd ` 370
    /// (`_HandlePlayer` @0x68cdd-0x68cff; see
    /// `nova_sim::red_alert`).
    RedAlert,
    /// A `Q` set operator's message is shown in flight.
    ScriptMessage,
    /// Cargo was jettisoned in flight: `_beepSnd[4]`, `snd ` 154
    /// (`_JettisonCargo` @0x3e31a-0x3e33d), whether or not a mission
    /// failed with it (see `nova_sim::cargo_box`).
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
    /// A person's mission was offered on its hail, in place of the comm
    /// dialog: `_beepSnd[4]`, `snd ` 154 (`_HandlePlayerCommunication`
    /// @0x62232-0x6223d).
    HailOffer,
    /// A person's hail quote was said in flight, by its own filters
    /// (`_HandleShip` @0x33f9e) or as it turned on the player
    /// (`_AIMakeShipAttackPlayer` @0x89ce1): `_beepSnd[4]`, `snd ` 154
    /// (`_ShowPersonHailQuote` @0x44586-0x445ad), for every hail quote
    /// said, even when its line is empty.
    HailQuote,
    /// The player's ship was newly disabled or newly started to break up:
    /// `_beepSnd[1]`, `snd ` 151 (`_DamageShip` @0x3b11e, @0x3b221; see
    /// the session's `player_downed` module).
    ShipDowned,
    /// The player's ship being disabled or breaking up failed a `Flags2`
    /// 0x0004 mission: `_beepSnd[1]`, `snd ` 151 (`_DamageShip`
    /// @0x3b04e, @0x3b2f1), hidden or not.
    DownedMissionFailed,
    /// A mission's special ship lost (destroyed or disabled) failed a
    /// mission that isn't hidden, with
    /// `nova_sim::SimMessage::MissionShipLost`:
    /// `_beepSnd[1]`, `snd ` 151 (`_HandleShipDisplay` @0x2db31-0x2db5a,
    /// `_DamageShip` @0x3aee8-0x3af11).
    MissionShipLost,
    /// The comm line said as a mission's special ship is boarded, on the
    /// pickup path or the assist path: `_beepSnd[4]`, `snd ` 154
    /// (`_HandlePlayerBoardAttempt` @0x65548-0x6556b, @0x656cb-0x656ee).
    /// A board off the assist path says nothing and is silent.
    MissionShipBoarded,
    /// A nav key was taken: Nav Off and Hyperspace Mode on every press
    /// (`_HandlePlayer` @0x69b19), a stellar key that found a stellar,
    /// even the one selected (@0x69ebc), and a click on a stellar not
    /// selected (@0x6a6c1): `_beepSnd[0]`, `snd ` 150.
    NavSelected,
    /// Hyper Select moved the selection through the system's links
    /// (`_HandlePlayer` @0x69c89): `_beepSnd[2]`, `snd ` 152. With no
    /// links it is silent.
    HyperSelected,
    /// A target key was taken: Target Select (Tab, with Shift or Option or
    /// both) on every press, before its search and whatever it finds
    /// (`_HandlePlayer` @0x69f6f), Closest Target (R, with or without
    /// Option) when it picks a ship other than the target (@0x6aa55), and
    /// a click in space on a ship other than the target
    /// (`nova_sim::Session::click_in_space`,
    /// @0x6a585): `_beepSnd[2]`, `snd ` 152. Closest Target finding no
    /// ship raises [`SimSound::Refused`] instead.
    TargetSelected,
    /// Option-backquote let go of the ship target (`_HandlePlayer`
    /// @0x69a7d), or a click on the player's own ship did
    /// (`nova_sim::Session::click_in_space`,
    /// @0x6a585): `_beepSnd[0]`, `snd ` 150. With no target either is
    /// silent.
    TargetCleared,
    /// A first-flight tutorial hint was shown: `_beepSnd[4]`, `snd ` 154
    /// (`_HandlePlayer` @0x70a1c, @0x70c12, @0x70dab, @0x70e59; see
    /// `nova_sim::tutorial`).
    TutorialHint,
    /// Traffic control answered the player: `_beepSnd[1]`, `snd ` 151.
    /// The land key's request with its line (`_HandlePlayerDockRequest`
    /// @0x67a69) and its denial (@0x67ec6), and the dock approach's
    /// greeting (`_HandlePlayerDockApproach` @0x649a9) raise it; a request
    /// within reach and "No response." are silent (see
    /// `nova_sim::clearance`).
    TrafficControl,
    /// The player boarded a person who offers its mission: `snd ` 390
    /// "Airlock" (loaded by `_LoadSounds` @0x1c277-0x1c289), played by
    /// `_HandlePlayerBoardAttempt` before `_OfferOneMission`
    /// (@0x658e5-0x65907). Its priority, 8, is not modelled.
    PersonOfferBoarded,
    /// The player's boarders went aboard a ship to plunder it: `snd ` 390
    /// "Airlock", as for [`SimSound::PersonOfferBoarded`], played by
    /// `_HandlePlayerBoardAttempt` before `_DoPlunderDialog` on both of
    /// its plunder paths: a person whose mission is not available
    /// (@0x65991-0x659b3), and every other ship plundered, after
    /// `_DoGoodSamaritan` or not (@0x65ede-0x65f00). So it sounds for a
    /// failed mission's special ship too, but never for one whose mission
    /// stands, nor for a person's offer, nor for a board refused. It
    /// sounds for a board the crew repels by the Bible's
    /// [`RuleKey::EmptyBooty`](crate::RuleKey::EmptyBooty), which the
    /// original never does: the boarders go aboard first. Its priority,
    /// 8, is not modelled.
    Boarded,
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
    /// of it, on any channel. A halted warp-up is over for the session's
    /// still-playing skip and the double-time switch too. The player's final explosion raises it with
    /// [`PLAYER_EXPLOSION_HALTS`]; the eject, a sabotage kill outfit going
    /// off, a disabled player's jump collapse and a jump's arrival with
    /// [`WARP_UP_HALTS`], 128 and 129.
    Halt(&'static [SoundId]),
    /// The player ejected from its disabled or dying ship, into the escape
    /// pod or an escape ship: `_escapePodSnd`, `snd ` 372 "Eject"
    /// (`_LoadSounds` @0x1c240-0x1c252), played by `PlaySound` with no
    /// test of whether it is playing (`_HandlePlayer` @0x6f920). A
    /// [`SimSound::Halt`] of [`WARP_UP_HALTS`] follows it.
    Ejected,
    /// A sabotage outfit went off aboard the player's ship: `_explodSnd[0]`,
    /// `snd ` 300 (`_LoadSounds`' loop from 0x1c200 loads `snd ` 300-363),
    /// played by `PlaySound` with no test of whether it is playing
    /// (`_HandlePlayer` @0x69375). Its priority, 100, is not modelled.
    OnboardExplosion,
    /// A disabled or dying player's pre-jump collapsed: `_beepSnd[1]`,
    /// `snd ` 151 (`_PlaySound` @0x68aaf), after a [`SimSound::Halt`] of
    /// [`WARP_UP_HALTS`]. Its priority, 50, is not modelled.
    JumpCollapsed,
    /// A key's action was refused: `_beepSnd[3]`, `snd ` 153, the
    /// original's shared refusal beep, whether or not a message says why.
    /// The player info key's refusals raise it (`_HandlePlayer`
    /// @0x7039a-0x703bd, priority 5, not modelled; see
    /// `nova_sim::Session::press_player_info`),
    /// as does the mission info key with no mission to show, before its
    /// "You have no active missions." (`PlaySound` @0x70557, priority 1,
    /// not modelled; see
    /// `nova_sim::Session::press_mission_info`),
    /// as do the jump key's (@0x6b89f, @0x6b9cc, @0x6ba72; see
    /// `nova_sim::Session::jump_refusal_told`)
    /// and every refusal of a hail, with no target too
    /// (`_HandlePlayerCommunication` @0x626cf, @0x6206c, @0x6261c,
    /// @0x6264e, its `PlaySound` argument of 1 or 5 not modelled; see
    /// `nova_sim::hail::HailRefusal`), as do the land key's
    /// not-landable, too-far and too-fast refusals from the dock request
    /// (@0x67e8f, @0x676aa, @0x676fe; see
    /// `nova_sim::landing::LandingRefusal::beeps`),
    /// and Closest Target finding no ship (`_HandlePlayer` @0x6aadb; see
    /// `nova_sim::Session::select_target`),
    /// and every other refusal
    /// that beeps reuses it rather than adding a variant of its own.
    Refused,
}

/// The warp-up sounds `_warpUpSnd[0]` and `[1]`, `snd ` 128 and 129
/// (loaded @0x1c12f-0x1c14f), in the order the player's eject halts them
/// after its [`SimSound::Ejected`] (`_HandlePlayer` @0x6f925-0x6f95a), a
/// sabotage kill outfit halts them as it goes off (@0x695ea, @0x6960d),
/// a disabled player's jump collapse halts them before its
/// [`SimSound::JumpCollapsed`] (@0x68a5c, @0x68a7f), and a jump's warp
/// end halts them before its [`SimSound::Arrived`] (`_HandlePlayer`
/// @0x6c6b7, @0x6c6d2, the warp-out @0x6c706).
pub const WARP_UP_HALTS: [SoundId; 2] = [SoundId(128), SoundId(129)];

/// The sounds the player's final explosion halts, in order:
/// `_HandleShipDisplay`, for the player only (`cmpw $0x0,0x86(%edi)`
/// @0x2d86d), halts the klaxon `_klaxxonSnd`, `snd ` 371 (@0x2d891), the
/// warp-up sounds `warpUpSnd[0]` and `warpUpSnd[1]`, 128 (@0x2d8b0) and
/// 129 (@0x2d8cb), and the warp-out `warpOutSnd`, 130 (@0x2d8ea).
pub const PLAYER_EXPLOSION_HALTS: [SoundId; 4] =
    [SoundId(371), SoundId(128), SoundId(129), SoundId(130)];
