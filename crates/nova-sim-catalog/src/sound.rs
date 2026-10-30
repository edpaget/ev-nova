//! The sounds a flight session makes, as events: what happened. Each one
//! says what it plays, if anything, as a [`SoundCue`] ([`SimSound::cue`]):
//! the `snd ` and the priority of the original's `PlaySound` call that
//! makes it, decided here, where the call is ported. The audio side plays
//! the cue, and keeps only what needs its own state: the engine loop the
//! thrust drives, the klaxon's still-playing test, the halts and an
//! explosion's distance.

use crate::catalog::SoundId;
pub use crate::sound_cue::{BEEP, NATURAL_RATE, SoundCue};

/// Something a `nova_sim::Session` did that may make a sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimSound {
    /// The ship began thrusting.
    ThrustStarted,
    /// The ship stopped thrusting: the player let go, or it landed or
    /// began a jump.
    ThrustStopped,
    /// The ship landed at an uninhabited stellar: `_PlayerLandOnStellar`'s
    /// `_beepSnd[1]`, `snd ` 151 (@0x632be-0x632ef). An inhabited
    /// stellar's touchdown is silent. The stellar's own sound is its spaceport's
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
    /// (see the [`warp`](crate::warp) module's `WarpUps`). It plays at
    /// `rate`, a 16.16 fraction of its own rate, pitched by the ship
    /// class's warp factor (`_AdjustWarpSoundSpeed` @0x5ecc, called
    /// @0x6b67d-0x6b688; [`warp_up_rate`](crate::warp::warp_up_rate)).
    JumpBegan {
        /// The playback rate, 0x10000 at its own.
        rate: u32,
    },
    /// The warp stage of a jump began in double time: the double-time
    /// warp-up, `_warpUpSnd[1]`, `snd ` 129 "Warp up x2" (`_HandlePlayer`
    /// @0x6b60d-0x6b63f, @0x6b66f-0x6b690), in place of
    /// [`SimSound::JumpBegan`], with its own still-playing skip
    /// (@0x6b617-0x6b62b), pitched as it is.
    JumpBeganInDoubleTime {
        /// The playback rate, 0x10000 at its own.
        rate: u32,
    },
    /// A jump ended in the next system: the warp-out, `snd ` 130
    /// (`_HandlePlayer` @0x6bbd3-0x6bc08; the hypergate @0x63e57 and the
    /// wormhole @0x6443d play it too). A jump's warp end first halts the
    /// warp-ups, a [`SimSound::Halt`] of [`WARP_UP_HALTS`] just before it
    /// (@0x6c6b7, @0x6c6d2); a gate or wormhole exit halts nothing. The
    /// fallback end raises both on its own pass, a pass before the system
    /// switch (@0x6c706; see `nova_sim`'s `warp_stage` module); the arrival
    /// after it is the natural end. A jump's natural end, both warp-ups
    /// finished or halted, raises none while the last warp-out still plays
    /// (@0x6bbe1-0x6bbe9; see [`WarpOut`](crate::warp::WarpOut)).
    Arrived,
    /// The player became threatened: the Red Alert, `snd ` 370
    /// (`_HandlePlayer` @0x68cdd-0x68cff; see
    /// `nova_sim::red_alert`).
    RedAlert,
    /// The player's ship began to cloak: `cloakSnd[1]`, `snd ` 381
    /// (`_DoShipCloak` @0xdec0-0xdef3, at priority 8). It plays only for
    /// the player (+0x86 0) and only when the fade was not already going
    /// in (`jg` @0xded2): no NPC's cloak ever sounds. The cloak key raises
    /// it (see `nova_sim_combat::player_cloak`).
    Cloaked,
    /// The player's ship began to uncloak: `cloakSnd[0]`, `snd ` 380
    /// (`_DoShipUncloak` @0xdf9e-0xdfd0, at priority 8). It plays only for
    /// the player, cloaked, and only when the fade was not already going
    /// out (`js` @0xdfb0). The cloak key, the upkeep's uncloak and a
    /// direct hit's decloak raise it (see `nova_sim_combat::player_cloak`).
    Uncloaked,
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
    /// An escort's voice clip plays: `_HandleVoiceSound`'s
    /// `PlaySound(handle, 15, …)` (@0x31f7a), at priority 15, of `snd `
    /// 1000 + 100 × set + 10 × message + clip (see the
    /// [`voice`](crate::voice) module). The message is a command's
    /// acknowledgement (`_IssueNewEscortCommand` @0x6667a), `_EscortAI`'s
    /// 1-in-3 call on a new target (@0x83e34) or a victory
    /// (`_HandleShipHit` @0x36d35).
    Voice {
        /// The clip's `snd `.
        sound: SoundId,
    },
    /// The voice clip `sound` is halted if it is playing: a message
    /// waiting cuts it off (`ST_HaltSound` @0x31e89), and landing kills it
    /// (`_KillVoiceSound` @0x31fab). [`SimSound::Halt`] takes the sounds
    /// it names as `&'static` slices, so the voice's halt is its own.
    HaltVoice {
        /// The clip's `snd `.
        sound: SoundId,
    },
    /// A disabled former escort or fighter repaired itself and rejoined
    /// the fleet (`_HandleShip` @0x33bb1-0x33bd4, `_beepSnd[4]`).
    EscortRepaired,
    /// The disabled player's Repair Droids repaired it, as it is told
    /// "Propulsion systems repaired.": `_beepSnd[4]`, `snd ` 154
    /// (`_HandlePlayer` @0x68c44-0x68c67; see the session's
    /// `player_repair` module).
    PropulsionRepaired,
    /// An announced fleet arrived with its `flët`'s `Quote`:
    /// `_beepSnd[4]`, `snd ` 154 (`_HyperSpawnFleet` @0x42698-0x426bb),
    /// even when the line is empty.
    FleetArrivalQuote,
    /// A ship called for help: `_beepSnd[4]`, `snd ` 154 (`_AICallForHelp`
    /// `_PlaySound` @0x830e0-0x83109).
    CallForHelp,
    /// The player's jump leaves fighters behind that can't jump, with its
    /// "Wait for me!" line: `_beepSnd[4]`, `snd ` 154 at priority 50
    /// (`_HandlePlayer` `PlaySound` @0x6b542-0x6b565).
    WaitForMe,
    /// An interceptor hailed the player before its scan: `_beepSnd[4]`,
    /// `snd ` 154 (`_InterceptorAI` `ST_PlaySound` @0x8cf9d-0x8cfbc).
    ScanHail,
    /// The sensors detect a reinforcement fleet approaching, three
    /// quarters of its way in: `_beepSnd[4]`, `snd ` 154
    /// (`_HandleReinforcements` `_PlaySound` @0x3924b, before its line).
    ReinforcementsApproaching,
    /// A person's mission was offered on its hail, in place of the comm
    /// dialog: `_beepSnd[4]`, `snd ` 154 (`_HandlePlayerCommunication`
    /// @0x62232-0x6223d).
    HailOffer,
    /// The escort dialog opened for a fleet escort hailed: `_beepSnd[4]`,
    /// `snd ` 154, at priority 1 (`_DoEscortDialog` @0x95228-0x9524b).
    EscortDialogOpened,
    /// The escort dialog closed, by any button, Release's included:
    /// `_beepSnd[2]`, `snd ` 152, at priority 1 (`_DoEscortDialog`
    /// @0x955d9-0x955fc).
    EscortDialogClosed,
    /// The planet comm dialog opened for a stellar hailed: `_beepSnd[4]`,
    /// `snd ` 154, at priority 1 (`_DoPlanetCommDialog` @0x96b3c-0x96b5f).
    PlanetCommOpened,
    /// Item 2 or 3 of the planet comm dialog was pressed: `_beepSnd[1]`,
    /// `snd ` 151, at priority 1 (`_DoPlanetCommDialog`
    /// @0x96ffc-0x97021).
    PlanetCommPressed,
    /// The planet comm dialog's bribe haggle ended, by any choice but a
    /// won Lower Price: `_beepSnd[1]`, `snd ` 151, at priority 1
    /// (`_DoPlanetCommDialog` @0x970c2-0x970e9).
    PlanetCommHaggled,
    /// The planet comm dialog closed, by Close Channel or a bribe paid:
    /// `_beepSnd[2]`, `snd ` 152, at priority 1 (`_DoPlanetCommDialog`
    /// @0x97a81-0x97aa4).
    PlanetCommClosed,
    /// The comm dialog opened for a ship hailed that gets neither the
    /// escort dialog nor a person's offer: `_beepSnd[4]`, `snd ` 154, at
    /// priority 1 (`_DoCommDialog` @0x95b34, before `_LoadAdvice`).
    CommDialogOpened,
    /// A comm dialog button was pressed, the middle one (`_DoCommDialog`
    /// @0x9601f) or Greetings (@0x96698), before anything it does:
    /// `_beepSnd[1]`, `snd ` 151, at priority 1.
    CommPressed,
    /// The comm dialog closed: `_beepSnd[2]`, `snd ` 152, at priority 1
    /// (`_DoCommDialog` @0x96843), before Release's release.
    CommDialogClosed,
    /// A person's hail quote was said in flight, by its own filters
    /// (`_HandleShip` @0x33f9e) or as it turned on the player
    /// (`_AIMakeShipAttackPlayer` @0x89ce1): `_beepSnd[4]`, `snd ` 154
    /// (`_ShowPersonHailQuote` @0x44586-0x445ad), for every hail quote
    /// said, even when its line is empty.
    HailQuote,
    /// A pirate's plunder took cargo or credits from the player or a former
    /// hired or captured escort, said on the message line: `_beepSnd[1]`,
    /// `snd ` 151 (`_AIPlunderShipContents` @0x84739-0x84758).
    CargoStolen,
    /// A pirate took a former escort of the player's into its fleet, said
    /// on the message line: `_beepSnd[1]`, `snd ` 151
    /// (`_AIPlunderShipContents` @0x848e9-0x84908).
    ShipStolen,
    /// A ship repairing the player finished, said on the comm line:
    /// `_beepSnd[1]`, `snd ` 151 (`_LowLevelAIHandler` @0x86ddf-0x86e0a).
    RepairsComplete,
    /// A ship refuelling the player finished, said on the comm line:
    /// `_beepSnd[1]`, `snd ` 151 (`_HighLevelAIHandler` @0x8f585-0x8f5b0).
    EnergyTransferComplete,
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
    /// A secondary-weapon key was taken (`_HandlePlayer`'s secondary
    /// block; see `nova_sim::session`'s `secondary_select`): Weapon Select
    /// picking a secondary, even the one already selected when it is the
    /// only candidate, before its step (@0x6acce-0x6acf1), and Weapon
    /// Safety letting go of the selection (@0x6adf8-0x6ae1b), on every pass
    /// it is held with one selected: `_beepSnd[2]`, `snd ` 152, at
    /// priority 1. Weapon Select with no candidate raises
    /// [`SimSound::Refused`] instead; the let-go of a secondary that runs
    /// out is silent.
    SecondarySelected,
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
    /// (@0x658e5-0x65907). Its priority is 8.
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
    /// 8.
    Boarded,
    /// A press in the plunder dialog that does something: `_beepSnd[2]`,
    /// `snd ` 152, priority 1 (`_DoPlunderDialog`). Abort (@0x935cb); the
    /// self-destruct tripped by a take's press (@0x93681, before `STR#`
    /// 2002 #113); the cargo taken (@0x939b9), or with no room for it
    /// (@0x937a0, #114); the credits taken (@0x93b3b); some ammunition
    /// taken (@0x93f10), or none fitting (@0x93f77, #117); the energy, with
    /// any on board (@0x94095); and the capture's successful roll, before
    /// the fleet's room is checked (@0x94368). A person's grant line plays
    /// it too, as the dialog opens, when it grants any (@0x933fe). The
    /// capture's own self-destruct (@0x942de-0x94336) and the escort path
    /// (@0x9459b-0x947cf) are silent; the refusals raise
    /// [`SimSound::Refused`].
    PlunderBeep,
    /// Either button of the captured-ship assignment dialog:
    /// `_beepSnd[1]`, `snd ` 151, priority 1
    /// (`_DoCapturedShipAssignmentDialog`: "Use As My Ship" @0x5b66b,
    /// "Use As Escort" @0x5b56b), before anything else is done, so a swap
    /// dropped for want of a ship slot beeps too.
    CaptureAssigned,
    /// The player's ship is breaking up: `_klaxxonSnd`, `snd ` 371
    /// (`_LoadSounds` @0x1c22c-0x1c23e). `_HandlePlayer` plays it
    /// (@0x6f1e4-0x6f227) while the death timer is above 1.0 and it is
    /// not already playing (`_ST_IsSoundPlaying` @0x6f200), so it repeats
    /// through the breakup; the audio side does that check. Its priority,
    /// 15. The player's final explosion halts it
    /// ([`PLAYER_EXPLOSION_HALTS`]).
    Klaxon,
    /// Each named `snd ` is halted if it is playing, as the original asks
    /// `ST_IsSoundPlaying` and then calls `ST_HaltSound`: every instance
    /// of it, on any channel. A halted warp-up is over for the session's
    /// still-playing skip and the double-time switch too. The player's final explosion raises it with
    /// [`PLAYER_EXPLOSION_HALTS`]; the eject, a sabotage kill outfit going
    /// off, a disabled player's jump collapse and a jump's warp end (its
    /// fallback's pass, or else its arrival) with [`WARP_UP_HALTS`], 128
    /// and 129.
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
    /// (`_HandlePlayer` @0x69375). Its priority is 100.
    OnboardExplosion,
    /// A stellar a `Y` destroyed exploded (`_EvalSetExp` @0x15ab4): the
    /// explosion's `snd `, 300 + its `bööm`'s `SoundIndex` when that is
    /// 0-63, which `_CreateExplosion` plays at once, whether the player is
    /// docked or in flight, with `_PlaySoundDistance(-1, snd, 6, ..)` from
    /// the explosion to the player (@0x3f8ca-0x3f91a). The view draws the
    /// explosion without playing it again (`CombatEvent::ExplodedHeard`).
    Explosion {
        /// The `snd `.
        sound: SoundId,
        /// Where the explosion is from the player, in whole pixels, x
        /// right and y down: how far away it is heard.
        offset: (i32, i32),
        /// Its priority, [`EXPLOSION_PRIORITY`].
        priority: u16,
    },
    /// A disabled or dying player's pre-jump collapsed: `_beepSnd[1]`,
    /// `snd ` 151 (`_PlaySound` @0x68aaf), after a [`SimSound::Halt`] of
    /// [`WARP_UP_HALTS`]. Its priority is 50.
    JumpCollapsed,
    /// A key's action was refused: `_beepSnd[3]`, `snd ` 153, the
    /// original's shared refusal beep, whether or not a message says why.
    /// The player info key's refusals raise it (`_HandlePlayer`
    /// @0x7039a-0x703bd, priority 5; see
    /// `nova_sim::Session::press_player_info`),
    /// as does the mission info key with no mission to show, before its
    /// "You have no active missions." (`PlaySound` @0x70557, priority 1;
    /// see
    /// `nova_sim::Session::press_mission_info`),
    /// as do the jump key's (@0x6b89f, @0x6b9cc, @0x6ba72; see
    /// `nova_sim::Session::jump_refusal_told`)
    /// and every refusal of a hail, with no target too
    /// (`_HandlePlayerCommunication` @0x626cf, @0x6206c, @0x6261c,
    /// @0x6264e, its `PlaySound` priority 1 or 5; see
    /// `nova_sim::hail::HailRefusal`), as do the land key's
    /// not-landable, too-far and too-fast refusals from the dock request
    /// (@0x67e8f, @0x676aa, @0x676fe; see
    /// `nova_sim::landing::LandingRefusal::beeps`),
    /// and Closest Target finding no ship (`_HandlePlayer` @0x6aadb; see
    /// `nova_sim::Session::select_target`),
    /// and Weapon Select finding no secondary to pick (`_HandlePlayer`
    /// @0x6ad90-0x6adb3, priority 1; see `nova_sim::session`'s
    /// `secondary_select`),
    /// as does each board refusal said, #131 too far (@0x65f9a), #132 too
    /// fast (@0x65fcc) and #130 can't board, a refused `PickupMode` 2
    /// pickup's too (@0x65ffe), all at priority 1, not modelled (see
    /// `nova_sim::Session::board`),
    /// as does the plunder dialog's (`_DoPlunderDialog`, priority 1; see
    /// `nova_sim::Session::plunder`): a take with nothing on board, cargo
    /// (@0x93ad2), credits (@0x93cb8), ammunition (@0x93d2f) or energy
    /// (@0x94256); a failed capture (@0x94936, #125); and a capture into a
    /// full fleet (@0x9487f, #124),
    /// and every other refusal
    /// that beeps reuses it rather than adding a variant of its own. The
    /// audio side plays it at 1, its most common site's priority (rdm task
    /// `refused-and-port-ambient-priorities-by-site`).
    Refused,
    /// The player began the self-destruct countdown: `_beepSnd[3]`,
    /// `snd ` 153, at priority 15 (`_HandlePlayer` `PlaySound` @0x6fe33).
    /// A dying player's start is silent and plays nothing, so it is no
    /// [`SimSound::Refused`].
    SelfDestructStarted,
    /// A self-destruct countdown line, 4 to 1 seconds left: `_beepSnd[2]`,
    /// `snd ` 152, at priority 15 (`PlaySound` @0x70065).
    SelfDestructCountdown,
    /// The player let go of the self-destruct chord while counting down:
    /// `_beepSnd[1]`, `snd ` 151, at priority 15 (`PlaySound` @0x700de).
    SelfDestructCancelled,
    /// The self-destruct countdown ran out and the player's ship goes:
    /// `_beepSnd[1]`, `snd ` 151, at priority 15 (`PlaySound` @0x701fb).
    SelfDestructed,
}

impl SimSound {
    /// Whether the original plays it after a landing's `dësc` movie, so a
    /// view playing that movie holds it back until the movie ends:
    /// `_PlayerLandOnStellar` plays the movie (@0x632a7) before
    /// `_DoAutoRefuel` (@0x632c5, silent) and the landing beep
    /// ([`SimSound::Landed`], @0x632ef). Everything else the land press
    /// raises comes before the movie, as `_HandlePlayerDockRequest` makes
    /// it (the voice kill [`SimSound::HaltVoice`] @0x66daf among it), and
    /// plays at once.
    #[must_use]
    pub const fn follows_landing_movie(self) -> bool {
        matches!(self, Self::Landed)
    }
}

/// The priority an explosion's sound plays at: `_CreateExplosion`'s
/// `_PlaySoundDistance(.., 6, ..)` (@0x3f8f1, played @0x3f91a).
pub const EXPLOSION_PRIORITY: u16 = 6;

/// The warp-up sounds `_warpUpSnd[0]` and `[1]`, `snd ` 128 and 129
/// (loaded @0x1c12f-0x1c14f), in the order the player's eject halts them
/// after its [`SimSound::Ejected`] (`_HandlePlayer` @0x6f925-0x6f95a), a
/// sabotage kill outfit halts them as it goes off (@0x695ea, @0x6960d),
/// a disabled player's jump collapse halts them before its
/// [`SimSound::JumpCollapsed`] (@0x68a5c, @0x68a7f), and a jump's warp
/// end halts them before its [`SimSound::Arrived`] (`_HandlePlayer`
/// @0x6c6b7, @0x6c6d2, the warp-out @0x6c706): its fallback end on the
/// fallback's own pass, a pass before the arrival, which then halts
/// nothing; a jump's arrival with no fallback pass first, at the
/// arrival.
pub const WARP_UP_HALTS: [SoundId; 2] = [SoundId(128), SoundId(129)];

/// The sounds the player's final explosion halts, in order:
/// `_HandleShipDisplay`, for the player only (`cmpw $0x0,0x86(%edi)`
/// @0x2d86d), halts the klaxon `_klaxxonSnd`, `snd ` 371 (@0x2d891), the
/// warp-up sounds `warpUpSnd[0]` and `warpUpSnd[1]`, 128 (@0x2d8b0) and
/// 129 (@0x2d8cb), and the warp-out `warpOutSnd`, 130 (@0x2d8ea).
pub const PLAYER_EXPLOSION_HALTS: [SoundId; 4] =
    [SoundId(371), SoundId(128), SoundId(129), SoundId(130)];

/// `_escapePodSnd`, `snd ` 372 "Eject" (`_LoadSounds` @0x1c240-0x1c252):
/// the player's eject plays it ([`SimSound::Ejected`]), and so does a dying
/// NPC's every escape pod, from where the NPC is, at priority 5
/// (`_HandleShip` @0x351fa, @0x35300), which the view plays for the fight's
/// `CombatEvent::PodEjected`.
pub const ESCAPE_POD_SOUND: SoundId = SoundId(372);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_escape_pod_sound_is_snd_372() {
        assert_eq!(ESCAPE_POD_SOUND, SoundId(372));
    }

    #[test]
    fn only_the_landing_beep_follows_the_landing_movie() {
        assert!(SimSound::Landed.follows_landing_movie());
        for sound in [
            SimSound::HaltVoice {
                sound: SoundId(1010),
            },
            SimSound::ThrustStopped,
            SimSound::TrafficControl,
            SimSound::Refused,
        ] {
            assert!(!sound.follows_landing_movie(), "{sound:?}");
        }
    }

    #[test]
    fn an_explosion_plays_at_priority_6() {
        assert_eq!(EXPLOSION_PRIORITY, 6);
    }
}
