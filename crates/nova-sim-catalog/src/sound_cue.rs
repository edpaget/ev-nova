//! What each [`SimSound`] plays: a [`SoundCue`], the `snd ` and priority
//! of the original's `PlaySound` call, decided where the call is ported
//! ([`SimSound::cue`]). Re-exported from [`sound`](crate::sound).

use crate::catalog::SoundId;
use crate::sound::SimSound;

/// What one sound event plays: one `snd `, at the original's priority for
/// it, over the rest or on the one mission channel, at its own rate or
/// pitched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundCue {
    /// The `snd `.
    pub sound: SoundId,
    /// The original's `PlaySound` priority at the call site (its second
    /// argument, or a param block's +0x14): how much it matters when the
    /// voices are full.
    pub priority: u16,
    /// Whether it plays on the one mission channel, dropped while the last
    /// sound there still plays, as a `P` set operator's exclusive sound
    /// does.
    pub exclusive: bool,
    /// Its playback rate, a 16.16 fraction of its own: [`NATURAL_RATE`]
    /// at its own.
    pub rate: u32,
}

/// A [`SoundCue`]'s rate at the sound's own, 1.0 in 16.16.
pub const NATURAL_RATE: u32 = 0x1_0000;

impl SoundCue {
    /// `sound` at `priority`, over whatever plays, at its own rate.
    #[must_use]
    pub const fn new(sound: SoundId, priority: u16) -> Self {
        Self {
            sound,
            priority,
            exclusive: false,
            rate: NATURAL_RATE,
        }
    }

    /// The cue on the one mission channel instead.
    #[must_use]
    pub const fn exclusively(self) -> Self {
        Self {
            exclusive: true,
            ..self
        }
    }

    /// The cue at `rate`, 16.16, instead of its own.
    #[must_use]
    pub const fn at_rate(self, rate: u32) -> Self {
        Self { rate, ..self }
    }
}

/// `_beepSnd[0..=4]`, the five beeps `_LoadSounds` loads from `snd ` 150 to
/// 154 (@0x1c174): "Beep1" to "Beep5". `_LoadSounds` fills them before the
/// main screen runs (`_InitEVNova` @0x47196, @0x47428). The interface's
/// beeps play them too (`nova_view::UiSound::cue`).
pub const BEEP: [SoundId; 5] = [
    SoundId(150),
    SoundId(151),
    SoundId(152),
    SoundId(153),
    SoundId(154),
];
/// `_warpUpSnd[0]` and `[1]`, `snd ` 128 "Warp up" and 129 "Warp up x2"
/// (the stock resource's name is "Warp up.x2"), loaded @0x1c12f-0x1c14f.
const WARP_UP: [SoundId; 2] = [SoundId(128), SoundId(129)];
/// The warp param block's priority, 0x7d00, which `_LoadSounds` fills
/// (@0x1c18e) and `ST_PlaySoundParam` plays the warp-ups by (@0x6b690).
const WARP_PRIORITY: u16 = 32_000;
/// `_warpOutSnd`, `snd ` 130 "Warp out" (loaded @0x1c152-0x1c16a).
const WARP_OUT: SoundId = SoundId(130);
/// `_redAlertSnd`, `snd ` 370 "Red Alert" (loaded @0x1c218-0x1c22a).
const RED_ALERT: SoundId = SoundId(370);
/// `_klaxxonSnd`, `snd ` 371 (loaded @0x1c22c-0x1c23e).
const KLAXON: SoundId = SoundId(371);
/// `cloakSnd[0]` and `[1]`, `snd ` 380 (the uncloak) and 381 (the cloak),
/// loaded @0x1c254-0x1c274.
const CLOAK: [SoundId; 2] = [SoundId(380), SoundId(381)];
/// `snd ` 390 "Airlock" (loaded @0x1c277-0x1c289).
const AIRLOCK: SoundId = SoundId(390);
/// `_explodSnd[0]`, `snd ` 300, the first of the explosions `_LoadSounds`'
/// loop from 0x1c200 loads from 300 to 363.
const FIRST_EXPLOSION: SoundId = SoundId(300);

impl SimSound {
    /// What the sound plays: the `snd ` and priority of the original's
    /// call that makes it. `None` for the events that play no one `snd `:
    /// the thrust, which drives the audio side's engine loop (no stock
    /// `snd ` is an engine sound, and the original thrusts in silence); the
    /// take-off, silent in the original; and the halts. The `snd ` names
    /// are the stock resources' own (all 227 are in `Nova Sounds.ndat`).
    /// The community *EV Nova Resource ID Guide* (github.com/andrews05/evstuff,
    /// `guides/resourceidguide.html`, "snd IDs"), which lists the engine's
    /// fixed sounds, agrees: "128 Warp up", "130 Warp out" and "151 Beep 2:
    /// ... request/grant landing clearance" (the clearance beeps are
    /// [`SimSound::TrafficControl`]'s), and it reserves no ID for an engine
    /// or a take-off sound.
    ///
    /// Every variant has its arm, with no wildcard: a new sound decides
    /// here what it plays.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one arm for each of the session's sounds
    #[allow(clippy::match_same_arms)] // each arm is its own call sites, with their addresses
    pub const fn cue(self) -> Option<SoundCue> {
        let cue = match self {
            Self::ThrustStarted | Self::ThrustStopped => return None,
            // 151 "Beep2": an uninhabited stellar's touchdown
            // (`_PlayerLandOnStellar` @0x632ef), at 1. The stellar's own
            // sound (its `CustSndID`) is the spaceport's ambient sound.
            Self::Landed => SoundCue::new(BEEP[1], 1),
            // None: the community resource guide lists none, and the
            // `NovaSwift` remake (`Game/GameScene.swift`) notes that
            // "lifting off a planet is silent in the original".
            Self::TookOff => return None,
            // The warp-ups, out of double time (@0x6b645-0x6b690) and in it
            // (@0x6b60d-0x6b63f, @0x6b66f-0x6b690), pitched by the warp
            // factor. The session decides both still-playing skips.
            Self::JumpBegan { rate } => SoundCue::new(WARP_UP[0], WARP_PRIORITY).at_rate(rate),
            Self::JumpBeganInDoubleTime { rate } => {
                SoundCue::new(WARP_UP[1], WARP_PRIORITY).at_rate(rate)
            }
            // `_HandlePlayer` @0x6bbd3-0x6bc08 or the fallback
            // @0x6c6fc-0x6c706, the hypergate @0x63e57 (@0x63e61), the
            // wormhole @0x6443d (@0x64447) and `_WarpUpCallback` @0x32049,
            // all at 50.
            Self::Arrived => SoundCue::new(WARP_OUT, 50),
            // `_HandlePlayer` @0x68cff, at 5.
            Self::RedAlert => SoundCue::new(RED_ALERT, 5),
            // `_DoShipCloak` @0xdef3 and `_DoShipUncloak` @0xdfd0, at 8.
            Self::Cloaked => SoundCue::new(CLOAK[1], 8),
            Self::Uncloaked => SoundCue::new(CLOAK[0], 8),
            // `_PlayGame` shows a `Q`'s message in flight (@0x4657a), at 1.
            Self::ScriptMessage => SoundCue::new(BEEP[4], 1),
            // `_JettisonCargo` @0x3e31a-0x3e33d, at 1.
            Self::CargoJettisoned => SoundCue::new(BEEP[4], 1),
            // `_ScanPlayer`'s `ST_PlaySound(.., 1, ..)` @0x7e721-0x7e740.
            Self::ScanFailedMission => SoundCue::new(BEEP[1], 1),
            // `_ScanPlayer`'s `ST_PlaySound(.., 1, ..)` @0x7ea5d, @0x7ee95,
            // @0x7f356.
            Self::Smuggling => SoundCue::new(BEEP[4], 1),
            // `_PlayGame` plays it on the one mission handle with
            // `PlaySound(.., 0x32, ..)` @0x46343. The Bible's reading,
            // played at once over the rest, has no site of its own and
            // plays the same sound at the same priority.
            Self::Script { sound, exclusive } => {
                let cue = SoundCue::new(sound, 50);
                if exclusive { cue.exclusively() } else { cue }
            }
            // After "New escort orders assigned:"
            // (`_IssueNewEscortCommand` @0x665be-0x665e1), at 1.
            Self::EscortsCommanded => SoundCue::new(BEEP[1], 1),
            // `_HandleVoiceSound`'s `PlaySound(handle, 15, …)` @0x31f7a.
            Self::Voice { sound } => SoundCue::new(sound, 15),
            Self::HaltVoice { .. } | Self::Halt(_) => return None,
            // After "Escort repaired." or "Fighter repaired." (`_HandleShip`
            // @0x33bc9-0x33bd4), at 8.
            Self::EscortRepaired => SoundCue::new(BEEP[4], 8),
            // After "Propulsion systems repaired." (`_HandlePlayer`'s
            // `PlaySound(.., 1, ..)` @0x68c4c, played @0x68c67).
            Self::PropulsionRepaired => SoundCue::new(BEEP[4], 1),
            // `_HyperSpawnFleet` @0x42698-0x426bb, even for an empty line,
            // at 1.
            Self::FleetArrivalQuote => SoundCue::new(BEEP[4], 1),
            // `_AICallForHelp`'s `PlaySound(.., 1, ..)` @0x830e0-0x83109.
            Self::CallForHelp => SoundCue::new(BEEP[4], 1),
            // The fighters' "Wait for me!": `_HandlePlayer`'s
            // `PlaySound(.., 50, ..)` @0x6b542-0x6b565.
            Self::WaitForMe => SoundCue::new(BEEP[4], 50),
            // `_InterceptorAI`'s `ST_PlaySound(.., 1, ..)`
            // @0x8cf9d-0x8cfbc.
            Self::ScanHail => SoundCue::new(BEEP[4], 1),
            // `_HandleReinforcements`' `PlaySound(.., 1, ..)` @0x3924b.
            Self::ReinforcementsApproaching => SoundCue::new(BEEP[4], 1),
            // `_HandlePlayerCommunication` @0x62232-0x6223d, at 1.
            Self::HailOffer => SoundCue::new(BEEP[4], 1),
            // `_DoEscortDialog` @0x95228-0x9524b and @0x955d9-0x955fc, at 1.
            Self::EscortDialogOpened => SoundCue::new(BEEP[4], 1),
            Self::EscortDialogClosed => SoundCue::new(BEEP[2], 1),
            // `_DoPlanetCommDialog` @0x96b5f, @0x97021, @0x970e9 and
            // @0x97aa4, at 1.
            Self::PlanetCommOpened => SoundCue::new(BEEP[4], 1),
            Self::PlanetCommPressed | Self::PlanetCommHaggled => SoundCue::new(BEEP[1], 1),
            Self::PlanetCommClosed => SoundCue::new(BEEP[2], 1),
            // `_ShowPersonHailQuote` @0x44586-0x445ad, at 1.
            Self::HailQuote => SoundCue::new(BEEP[4], 1),
            // `_AIPlunderShipContents`' cargo line (@0x84758) and
            // stolen-escort line (@0x84908), at 1.
            Self::CargoStolen | Self::ShipStolen => SoundCue::new(BEEP[1], 1),
            // A repairer's "Repairs complete" (`_LowLevelAIHandler`
            // @0x86e0a) and a refueller's "Energy transfer complete"
            // (`_HighLevelAIHandler` @0x8f5b0), at 1.
            Self::RepairsComplete | Self::EnergyTransferComplete => SoundCue::new(BEEP[1], 1),
            // `_DamageShip`: the player's ship newly disabled (@0x3b11e) or
            // breaking up (@0x3b221), and either failing its first
            // `Flags2` 0x0004 mission (@0x3b04e, @0x3b2f1), at 1.
            Self::ShipDowned | Self::DownedMissionFailed => SoundCue::new(BEEP[1], 1),
            // `_HandleShipDisplay` @0x2db5a (destroyed) and `_DamageShip`
            // @0x3af11 (disabled), at 1.
            Self::MissionShipLost => SoundCue::new(BEEP[1], 1),
            // `_HandlePlayerBoardAttempt`'s pickup path (@0x6556b) and
            // assist path (@0x656ee), at 8.
            Self::MissionShipBoarded => SoundCue::new(BEEP[4], 8),
            // `_HandlePlayer`: Nav Off and Hyperspace Mode (@0x69b19), a
            // stellar key (@0x69ebc), a click on a stellar (@0x6a6c1), at 1.
            Self::NavSelected => SoundCue::new(BEEP[0], 1),
            // Hyper Select (@0x69c89) or a click on the floating map
            // (@0x6a75d), at 1.
            Self::HyperSelected => SoundCue::new(BEEP[2], 1),
            // Target Select (@0x69f64-0x69f6f), Closest Target
            // (@0x6aa32-0x6aa55) and a click in space (@0x6a585), at 1.
            Self::TargetSelected => SoundCue::new(BEEP[2], 1),
            // Option-backquote (@0x69a7d) or a click on the player's own
            // ship (@0x6a585), at 1.
            Self::TargetCleared => SoundCue::new(BEEP[0], 1),
            // Weapon Select (@0x6acce-0x6acf1) and Weapon Safety
            // (@0x6adf8-0x6ae1b), at 1.
            Self::SecondarySelected => SoundCue::new(BEEP[2], 1),
            // `_HandlePlayer` @0x70a1c, @0x70c12, @0x70dab, @0x70e59, at 1.
            Self::TutorialHint => SoundCue::new(BEEP[4], 1),
            // `_HandlePlayerDockRequest`'s line (@0x67a69) and denial
            // (@0x67ec6), and `_HandlePlayerDockApproach`'s greeting
            // (@0x649a9), at 1.
            Self::TrafficControl => SoundCue::new(BEEP[1], 1),
            // `_HandlePlayerBoardAttempt` before `_OfferOneMission`
            // (@0x65907) and before `_DoPlunderDialog` on its two plunder
            // paths (@0x659b3, @0x65f00), at 8.
            Self::PersonOfferBoarded | Self::Boarded => SoundCue::new(AIRLOCK, 8),
            // `_DoPlunderDialog` @0x935cb, @0x93681, @0x937a0, @0x939b9,
            // @0x93b3b, @0x93f10, @0x93f77, @0x94095, @0x94368, @0x933fe,
            // at 1.
            Self::PlunderBeep => SoundCue::new(BEEP[2], 1),
            // `_DoCapturedShipAssignmentDialog` @0x5b56b, @0x5b66b, at 1.
            Self::CaptureAssigned => SoundCue::new(BEEP[1], 1),
            // `_HandlePlayer` @0x6f1e4-0x6f227, at 15; the audio side plays
            // it unless it is still playing (`_ST_IsSoundPlaying`
            // @0x6f200).
            Self::Klaxon => SoundCue::new(KLAXON, 15),
            // `_escapePodSnd`: `_HandlePlayer`'s `PlaySound` @0x6f920, at
            // 50, with no test of whether it is still playing.
            Self::Ejected => SoundCue::new(crate::sound::ESCAPE_POD_SOUND, 50),
            // `_HandlePlayer`'s `PlaySound` @0x69375, at 100, with no test
            // of whether it is still playing.
            Self::OnboardExplosion => SoundCue::new(FIRST_EXPLOSION, 100),
            // Its own `snd ` at its own priority; the audio side hears it
            // from its offset (`_PlaySoundDistance` @0x3f8ca-0x3f91a).
            Self::Explosion {
                sound, priority, ..
            } => SoundCue::new(sound, priority),
            // `_HandlePlayer`'s `_PlaySound` @0x68aaf, at 50.
            Self::JumpCollapsed => SoundCue::new(BEEP[1], 50),
            // The shared refusal beep. Most of the sites it models play it
            // at 1: the mission info key (@0x70557), the jump key
            // (@0x6b89f, @0x6b9cc, @0x6ba72), two of the hail refusals
            // (@0x62671, and @0x626f2 from @0x626cf), the land key
            // (@0x676aa, @0x676fe, @0x67e8f) and Closest Target
            // (@0x6aadb). The player info key (@0x703bd) and two hail
            // refusals (@0x6263f, and @0x626f2 from @0x6206c) play it at 5
            // (rdm task `refused-and-port-ambient-priorities-by-site`).
            Self::Refused => SoundCue::new(BEEP[3], 1),
            // The self-destruct (`_HandlePlayer` block Z), each at 15: its
            // start (@0x6fe33), each countdown line (@0x70065), its let-go
            // (@0x700de) and its end (@0x701fb).
            Self::SelfDestructStarted => SoundCue::new(BEEP[3], 15),
            Self::SelfDestructCountdown => SoundCue::new(BEEP[2], 15),
            Self::SelfDestructCancelled | Self::SelfDestructed => SoundCue::new(BEEP[1], 15),
        };
        Some(cue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::{EXPLOSION_PRIORITY, WARP_UP_HALTS};

    fn cue(sound: i16, priority: u16) -> SoundCue {
        SoundCue::new(SoundId(sound), priority)
    }

    #[test]
    fn a_cue_plays_over_the_rest_at_its_own_rate_unless_told_otherwise() {
        let cue = SoundCue::new(SoundId(151), 8);
        assert_eq!(
            cue,
            SoundCue {
                sound: SoundId(151),
                priority: 8,
                exclusive: false,
                rate: NATURAL_RATE,
            }
        );
        assert_eq!(NATURAL_RATE, 0x1_0000);
        assert!(cue.exclusively().exclusive);
        assert_eq!(cue.exclusively().sound, SoundId(151));
        assert_eq!(cue.exclusively().priority, 8);
        assert_eq!(cue.at_rate(0x8000).rate, 0x8000);
        assert!(!cue.at_rate(0x8000).exclusive);
        assert_eq!(cue.at_rate(0x8000).exclusively().rate, 0x8000);
    }

    /// One row for each variant, each with the original's global and call
    /// site. Rust can't count an enum's variants, so a new variant could
    /// miss its row here: the exhaustive [`SimSound::cue`] is what makes
    /// its author decide what it plays, and its row is CLAUDE.md's "sounds
    /// come with the function".
    #[test]
    #[allow(clippy::too_many_lines)] // one row for each of the session's sounds
    fn each_sim_sound_plays_its_snd_at_its_priority() {
        let pinned = [
            (
                SimSound::ThrustStarted,
                None,
                "the audio side's engine loop",
            ),
            (
                SimSound::ThrustStopped,
                None,
                "the audio side's engine loop",
            ),
            (SimSound::Landed, Some(cue(151, 1)), "_beepSnd[1] @0x632ef"),
            (SimSound::TookOff, None, "the original takes off in silence"),
            (
                SimSound::JumpBegan { rate: 0x8000 },
                Some(SoundCue::new(SoundId(128), 32_000).at_rate(0x8000)),
                "_warpUpSnd[0] @0x6b690, the warp param block @0x1c18e",
            ),
            (
                SimSound::JumpBeganInDoubleTime { rate: 0x8000 },
                Some(SoundCue::new(SoundId(129), 32_000).at_rate(0x8000)),
                "_warpUpSnd[1] @0x6b690, the warp param block @0x1c18e",
            ),
            (
                SimSound::Arrived,
                Some(cue(130, 50)),
                "_warpOutSnd @0x6bc08",
            ),
            (
                SimSound::RedAlert,
                Some(cue(370, 5)),
                "_redAlertSnd @0x68cff",
            ),
            (SimSound::Cloaked, Some(cue(381, 8)), "cloakSnd[1] @0xdef3"),
            (
                SimSound::Uncloaked,
                Some(cue(380, 8)),
                "cloakSnd[0] @0xdfd0",
            ),
            (
                SimSound::ScriptMessage,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x4657a",
            ),
            (
                SimSound::CargoJettisoned,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x3e33d",
            ),
            (
                SimSound::ScanFailedMission,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x7e740",
            ),
            (
                SimSound::Smuggling,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x7ea5d",
            ),
            (
                SimSound::Script {
                    sound: SoundId(300),
                    exclusive: true,
                },
                Some(SoundCue::new(SoundId(300), 50).exclusively()),
                "the mission handle @0x46343",
            ),
            (
                SimSound::Script {
                    sound: SoundId(300),
                    exclusive: false,
                },
                Some(cue(300, 50)),
                "the Bible's reading, over the rest",
            ),
            (
                SimSound::EscortsCommanded,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x665e1",
            ),
            (
                SimSound::Voice {
                    sound: SoundId(1523),
                },
                Some(cue(1523, 15)),
                "_HandleVoiceSound @0x31f7a",
            ),
            (
                SimSound::HaltVoice {
                    sound: SoundId(1523),
                },
                None,
                "a halt",
            ),
            (
                SimSound::EscortRepaired,
                Some(cue(154, 8)),
                "_beepSnd[4] @0x33bd4",
            ),
            (
                SimSound::PropulsionRepaired,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x68c67",
            ),
            (
                SimSound::FleetArrivalQuote,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x426bb",
            ),
            (
                SimSound::CallForHelp,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x83109",
            ),
            (
                SimSound::WaitForMe,
                Some(cue(154, 50)),
                "_beepSnd[4] @0x6b565",
            ),
            (
                SimSound::ScanHail,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x8cfbc",
            ),
            (
                SimSound::ReinforcementsApproaching,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x3924b",
            ),
            (
                SimSound::HailOffer,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x6223d",
            ),
            (
                SimSound::EscortDialogOpened,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x9524b",
            ),
            (
                SimSound::EscortDialogClosed,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x955fc",
            ),
            (
                SimSound::PlanetCommOpened,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x96b5f",
            ),
            (
                SimSound::PlanetCommPressed,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x97021",
            ),
            (
                SimSound::PlanetCommHaggled,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x970e9",
            ),
            (
                SimSound::PlanetCommClosed,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x97aa4",
            ),
            (
                SimSound::HailQuote,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x445ad",
            ),
            (
                SimSound::CargoStolen,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x84758",
            ),
            (
                SimSound::ShipStolen,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x84908",
            ),
            (
                SimSound::RepairsComplete,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x86e0a",
            ),
            (
                SimSound::EnergyTransferComplete,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x8f5b0",
            ),
            (
                SimSound::ShipDowned,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x3b11e",
            ),
            (
                SimSound::DownedMissionFailed,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x3b04e",
            ),
            (
                SimSound::MissionShipLost,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x2db5a",
            ),
            (
                SimSound::MissionShipBoarded,
                Some(cue(154, 8)),
                "_beepSnd[4] @0x6556b",
            ),
            (
                SimSound::NavSelected,
                Some(cue(150, 1)),
                "_beepSnd[0] @0x69b19",
            ),
            (
                SimSound::HyperSelected,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x69c89",
            ),
            (
                SimSound::TargetSelected,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x69f6f",
            ),
            (
                SimSound::TargetCleared,
                Some(cue(150, 1)),
                "_beepSnd[0] @0x69a7d",
            ),
            (
                SimSound::SecondarySelected,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x6acf1",
            ),
            (
                SimSound::TutorialHint,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x70a1c",
            ),
            (
                SimSound::TrafficControl,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x67a69",
            ),
            (
                SimSound::PersonOfferBoarded,
                Some(cue(390, 8)),
                "Airlock @0x65907",
            ),
            (SimSound::Boarded, Some(cue(390, 8)), "Airlock @0x659b3"),
            (
                SimSound::PlunderBeep,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x935cb",
            ),
            (
                SimSound::CaptureAssigned,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x5b56b",
            ),
            (SimSound::Klaxon, Some(cue(371, 15)), "_klaxxonSnd @0x6f227"),
            (SimSound::Halt(&WARP_UP_HALTS), None, "a halt"),
            (
                SimSound::Ejected,
                Some(cue(372, 50)),
                "_escapePodSnd @0x6f920",
            ),
            (
                SimSound::OnboardExplosion,
                Some(cue(300, 100)),
                "_explodSnd[0] @0x69375",
            ),
            (
                SimSound::Explosion {
                    sound: SoundId(302),
                    offset: (400, 0),
                    priority: EXPLOSION_PRIORITY,
                },
                Some(cue(302, 6)),
                "_CreateExplosion @0x3f91a",
            ),
            (
                SimSound::JumpCollapsed,
                Some(cue(151, 50)),
                "_beepSnd[1] @0x68aaf",
            ),
            (
                SimSound::Refused,
                Some(cue(153, 1)),
                "_beepSnd[3], its most common site's 1",
            ),
            (
                SimSound::SelfDestructStarted,
                Some(cue(153, 15)),
                "_beepSnd[3] @0x6fe33",
            ),
            (
                SimSound::SelfDestructCountdown,
                Some(cue(152, 15)),
                "_beepSnd[2] @0x70065",
            ),
            (
                SimSound::SelfDestructCancelled,
                Some(cue(151, 15)),
                "_beepSnd[1] @0x700de",
            ),
            (
                SimSound::SelfDestructed,
                Some(cue(151, 15)),
                "_beepSnd[1] @0x701fb",
            ),
        ];
        for (sound, cue, source) in pinned {
            assert_eq!(sound.cue(), cue, "{sound:?}: {source}");
        }
    }
}
