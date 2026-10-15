//! The audio core: sound events and the screen shown in, playback
//! commands out.

use std::time::Duration;

use nova_data::SoundId;
use nova_view::Showing;
use nova_view::sound::{SimSound, Sound, UiSound};

use crate::port::{Audio, AudioCommand, Volume};
use crate::settings::{AudioSettings, volume_level};
use crate::table::SoundTable;

/// Decides what plays: turns each frame's sound events and the screen
/// shown into commands on its [`Audio`] port.
///
/// It remembers what it has asked the port for (the music on, the engine
/// loop on) and what it has been told (the scene, whether the ship is
/// thrusting), and sends a command only when what it wants changes.
///
/// - Music is the original's one-shot intro track: it plays on the main
///   menu, and the dialogs over it, while the music setting is on (see
///   [`has_music`]). It is spent for good, as in the original, once it has
///   played to its end, or once a pilot enters the game or the About text
///   opens, which fade it out ([`music_fade`]). Spent music never plays
///   again in the run.
/// - Known divergence: the original reads its Intro Music checkbox only
///   at launch. Here the music setting acts at once: off stops the music,
///   and on, before the music is spent, starts it again from the top.
/// - Each event with a `snd ` in the table plays it once, at the effects
///   volume, while the sound setting is on. A landing plays the table's
///   landing sound, then the stellar's own. A `P` set operator's sound
///   plays its own `snd ` the same way, on the one mission channel when
///   it is exclusive ([`AudioCommand::PlayExclusive`]). The klaxon plays
///   unless it is still playing ([`AudioCommand::PlayUnlessPlaying`]),
///   which the port decides.
/// - A halt halts each `snd ` it names, in order ([`AudioCommand::Halt`]),
///   whatever the sound setting: halting what is not playing is harmless.
/// - Each of a fight's sounds plays its own `snd ` once, at the effects
///   volume times its [`distance_gain`], while the sound setting is on.
/// - The engine loops while the sound setting is on, the ship thrusts in
///   flight, and the table has an engine sound. Leaving flight, or opening
///   the Preferences dialog that pauses it, stops it and forgets the
///   thrust: flight lets go of its keys when it is hidden or paused.
pub struct AudioCore<A: Audio> {
    audio: A,
    table: SoundTable,
    settings: AudioSettings,
    /// The last scene shown, the overlays (About, Preferences) aside.
    scene: Option<Showing>,
    /// Whether the ship is thrusting, as the events last said.
    thrusting: bool,
    /// Where the music stands.
    music: Music,
    /// Whether the engine loop has been started and not stopped since.
    loop_on: bool,
}

impl<A: Audio> AudioCore<A> {
    /// A core playing the original game's sounds through `audio`, with
    /// the default settings.
    pub fn new(audio: A) -> Self {
        Self::with_table(audio, SoundTable::ORIGINAL)
    }

    /// A core playing `table`'s sounds through `audio`, with the default
    /// settings.
    pub fn with_table(audio: A, table: SoundTable) -> Self {
        Self {
            audio,
            table,
            settings: AudioSettings::default(),
            scene: None,
            thrusting: false,
            music: Music::Stopped,
            loop_on: false,
        }
    }

    /// Takes in one frame's worth: the screen `showing` (`None` keeps the
    /// scene as it was), then each of `sounds` in order.
    pub fn update(&mut self, showing: Option<Showing>, sounds: &[Sound]) {
        if let Some(scene) = showing.filter(|&scene| !is_overlay(scene)) {
            self.scene = Some(scene);
        }
        // Entering the game (`_EnterGameFromMainScreen` @0x19ec6) and the
        // About box (`_DoAboutBox` @0x15fbf) cancel the music for good.
        if showing == Some(Showing::About)
            || self.scene.is_some_and(|scene| scene != Showing::MainMenu)
        {
            self.spend_music();
        }
        // The Preferences dialog, the boarding dialogs, the comm dialogs,
        // a person's offer dialog, the mission dialogs and the player info
        // screen pause flight
        // below them, and flight lets go of its keys, so the thrust is
        // forgotten as for leaving it.
        let pauses = matches!(
            showing,
            Some(
                Showing::Preferences
                    | Showing::Plunder
                    | Showing::Assignment
                    | Showing::Comm
                    | Showing::Haggle
                    | Showing::Offer
                    | Showing::MissionText
                    | Showing::MissionInfo
                    | Showing::PlayerInfo
            )
        );
        if self.scene != Some(Showing::Flight) || pauses {
            self.thrusting = false;
        }
        self.reconcile();
        for &sound in sounds {
            match sound {
                // Thrust counts only in flight, where the ship flies.
                Sound::Sim(SimSound::ThrustStarted) => {
                    self.thrusting = self.scene == Some(Showing::Flight);
                }
                Sound::Sim(SimSound::ThrustStopped) => self.thrusting = false,
                Sound::Sim(SimSound::Landed { stellar_sound }) => {
                    self.play(self.table.landing);
                    self.play(stellar_sound);
                }
                Sound::Sim(SimSound::TookOff) => self.play(self.table.take_off),
                Sound::Sim(SimSound::JumpBegan) => self.play(self.table.jump),
                Sound::Sim(SimSound::Arrived) => self.play(self.table.arrival),
                Sound::Sim(SimSound::ScriptMessage) => self.play(self.table.script_message),
                Sound::Sim(SimSound::RedAlert) => self.play(self.table.red_alert),
                Sound::Sim(SimSound::CargoJettisoned) => self.play(self.table.jettison),
                Sound::Sim(SimSound::ScanFailedMission) => self.play(self.table.scan_failed),
                Sound::Sim(SimSound::Smuggling) => self.play(self.table.smuggling),
                Sound::Sim(SimSound::Script {
                    sound,
                    exclusive: false,
                }) => self.play(Some(sound)),
                Sound::Sim(SimSound::Script {
                    sound,
                    exclusive: true,
                }) => {
                    if self.settings.sound {
                        self.audio.run(AudioCommand::PlayExclusive {
                            sound,
                            volume: self.settings.effects_volume,
                        });
                    }
                }
                Sound::Ui(UiSound::ButtonDown) => self.play(self.table.button_down),
                Sound::Ui(UiSound::ButtonUp) => self.play(self.table.button_up),
                Sound::Ui(UiSound::Alert) => self.play(self.table.alert),
                Sound::Sim(SimSound::EscortsCommanded) => self.play(self.table.escorts_commanded),
                Sound::Sim(SimSound::EscortRepaired) => self.play(self.table.escort_repaired),
                Sound::Sim(SimSound::FleetArrivalQuote) => self.play(self.table.fleet_quote),
                Sound::Sim(SimSound::HailOffer) => self.play(self.table.hail_offer),
                Sound::Sim(SimSound::HailQuote) => self.play(self.table.hail_quote),
                Sound::Sim(SimSound::NavSelected) => self.play(self.table.nav_selected),
                Sound::Sim(SimSound::HyperSelected) => self.play(self.table.hyper_selected),
                Sound::Sim(SimSound::TargetCleared) => self.play(self.table.target_cleared),
                Sound::Sim(SimSound::TutorialHint) => self.play(self.table.tutorial_hint),
                Sound::Sim(SimSound::ShipDowned) => self.play(self.table.ship_downed),
                Sound::Sim(SimSound::DownedMissionFailed) => {
                    self.play(self.table.downed_mission_failed);
                }
                Sound::Sim(SimSound::TrafficControl) => self.play(self.table.traffic_control),
                Sound::Sim(SimSound::Klaxon) => self.sound_klaxon(),
                Sound::Sim(SimSound::Ejected) => self.play(self.table.eject),
                Sound::Sim(SimSound::OnboardExplosion) => {
                    self.play(self.table.onboard_explosion);
                }
                Sound::Sim(SimSound::JumpCollapsed) => self.play(self.table.jump_collapsed),
                Sound::Sim(SimSound::Refused) => self.play(self.table.refused),
                Sound::Sim(SimSound::Halt(sounds)) => self.halt(sounds),
                Sound::Ui(UiSound::EscortMenuToggled) => self.play(self.table.escort_menu),
                Sound::Ui(UiSound::EscortGroupSelected) => self.play(self.table.escort_group),
                Sound::Ui(UiSound::NoEscorts) => self.play(self.table.no_escorts),
                Sound::Ui(UiSound::NewsOpened) => self.play(self.table.news_open),
                Sound::Ui(UiSound::NewsClosed) => self.play(self.table.news_close),
                Sound::Combat(fight) => {
                    let gain = distance_gain(fight.offset);
                    let volume = self.settings.effects_volume.amplitude() * gain;
                    self.play_at(Some(fight.sound), Volume::new(volume));
                }
            }
            self.reconcile();
        }
    }

    /// Plays the klaxon, unless it is still playing, at the effects
    /// volume, while sound is on.
    fn sound_klaxon(&mut self) {
        if let Some(sound) = self.table.klaxon
            && self.settings.sound
        {
            self.audio.run(AudioCommand::PlayUnlessPlaying {
                sound,
                volume: self.settings.effects_volume,
            });
        }
    }

    /// Halts each of `sounds` that is playing.
    fn halt(&mut self, sounds: &[SoundId]) {
        for &sound in sounds {
            self.audio.run(AudioCommand::Halt { sound });
        }
    }

    /// Plays `sound`, if there is one, at the effects volume, while sound
    /// is on.
    fn play(&mut self, sound: Option<SoundId>) {
        self.play_at(sound, self.settings.effects_volume);
    }

    /// Plays `sound`, if there is one, at `volume`, while sound is on.
    fn play_at(&mut self, sound: Option<SoundId>, volume: Volume) {
        if let Some(sound) = sound
            && self.settings.sound
        {
            self.audio.run(AudioCommand::Play { sound, volume });
        }
    }

    /// Starts or stops the music and the engine loop where what is wanted
    /// differs from what was last asked for.
    fn reconcile(&mut self) {
        if self.music == Music::Playing && self.audio.music_ended() {
            self.music = Music::Spent;
        }
        if self.music != Music::Spent {
            let wanted = self.settings.music && self.scene.is_some_and(has_music);
            if wanted != (self.music == Music::Playing) {
                self.music = if wanted {
                    Music::Playing
                } else {
                    Music::Stopped
                };
                self.audio.run(if wanted {
                    AudioCommand::StartMusic {
                        volume: self.settings.music_volume,
                    }
                } else {
                    AudioCommand::StopMusic
                });
            }
        }
        let engine = self.table.engine.filter(|_| {
            self.settings.sound && self.thrusting && self.scene == Some(Showing::Flight)
        });
        if engine.is_some() != self.loop_on {
            self.loop_on = engine.is_some();
            self.audio.run(match engine {
                Some(sound) => AudioCommand::StartLoop {
                    sound,
                    volume: self.settings.effects_volume,
                },
                None => AudioCommand::StopLoop,
            });
        }
    }

    /// Spends the music for good, fading it out if it is playing.
    fn spend_music(&mut self) {
        if self.music == Music::Playing {
            self.audio.run(AudioCommand::FadeOutMusic(music_fade(
                self.settings.music_volume,
            )));
        }
        self.music = Music::Spent;
    }

    /// Turns sound effects on or off. Turning them off stops every effect
    /// playing.
    pub fn set_sound(&mut self, on: bool) {
        if self.settings.sound && !on {
            self.audio.run(AudioCommand::StopEffects);
            self.loop_on = false;
        }
        self.settings.sound = on;
        self.reconcile();
    }

    /// Turns the music on or off.
    pub fn set_music(&mut self, on: bool) {
        self.settings.music = on;
        self.reconcile();
    }

    /// Sets how loud sound effects are, the engine loop included.
    pub fn set_effects_volume(&mut self, volume: Volume) {
        self.settings.effects_volume = volume;
        if self.loop_on {
            self.audio.run(AudioCommand::SetLoopVolume(volume));
        }
    }

    /// Sets how loud the music is.
    pub fn set_music_volume(&mut self, volume: Volume) {
        self.settings.music_volume = volume;
        if self.music == Music::Playing {
            self.audio.run(AudioCommand::SetMusicVolume(volume));
        }
    }

    /// The core with `settings` in place of the defaults, before anything
    /// plays.
    #[must_use]
    pub fn with_settings(self, settings: AudioSettings) -> Self {
        Self { settings, ..self }
    }

    /// Changes the settings to `settings`, through the setters, only for
    /// the fields that differ: the volumes first, then sound, then music.
    pub fn apply(&mut self, settings: AudioSettings) {
        if settings.effects_volume != self.settings.effects_volume {
            self.set_effects_volume(settings.effects_volume);
        }
        if settings.music_volume != self.settings.music_volume {
            self.set_music_volume(settings.music_volume);
        }
        if settings.sound != self.settings.sound {
            self.set_sound(settings.sound);
        }
        if settings.music != self.settings.music {
            self.set_music(settings.music);
        }
    }

    /// The settings.
    #[must_use]
    pub fn settings(&self) -> AudioSettings {
        self.settings
    }

    /// The port it plays through.
    #[must_use]
    pub fn audio(&self) -> &A {
        &self.audio
    }
}

/// Where the music stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Music {
    /// Not playing, and may start.
    Stopped,
    /// Started and not stopped since.
    Playing,
    /// Played to its end, or faded out by leaving the main menu or
    /// opening About: it never plays again.
    Spent,
}

/// Within this many pixels of the player, a sound is heard at full volume.
pub const NEAR: f64 = 200.0;
/// The squared distance whose ratio gives a sound's volume up and down.
pub const FAR_ALONG: f64 = 722_500.0;
/// The squared distance whose ratio gives a sound's volume across, beyond
/// [`NEAR`].
pub const FAR_ACROSS: f64 = NEAR * NEAR;
/// The quietest a sound is heard, however far away.
pub const QUIETEST: f64 = 0.125;

/// How loud a sound `offset` pixels from the player is heard, from 1/8 to
/// 1, as the original hears it (`_PlaySoundDistance` @0xe007): in full
/// within [`NEAR`] pixels; beyond, `722500 / d²`, or, when it is more than
/// [`NEAR`] pixels across, the mean of that and `40000 / d²`, each kept
/// within [`QUIETEST`] and 1.
#[must_use]
pub fn distance_gain(offset: (i32, i32)) -> f32 {
    let (dx, dy) = (f64::from(offset.0), f64::from(offset.1));
    let squared = dx.mul_add(dx, dy * dy);
    if squared <= FAR_ACROSS {
        return 1.0;
    }
    let heard = |ratio: f64| (ratio / squared).clamp(QUIETEST, 1.0);
    let gain = if dx.abs() > NEAR {
        f64::midpoint(heard(FAR_ALONG), heard(FAR_ACROSS))
    } else {
        heard(FAR_ALONG)
    };
    gain as f32
}

/// Whether `showing` is shown over another screen, keeping the scene
/// below it: the About text, the Preferences dialog, the new pilot's name
/// entry, the saved pilots' list and a note over the main menu, and the plunder,
/// assignment, comm, haggle and offer dialogs, the mission texts and the
/// mission info screen over flight.
fn is_overlay(showing: Showing) -> bool {
    matches!(
        showing,
        Showing::About
            | Showing::Preferences
            | Showing::NewPilot
            | Showing::OpenPilot
            | Showing::Note
            | Showing::Plunder
            | Showing::Assignment
            | Showing::Comm
            | Showing::Haggle
            | Showing::Offer
            | Showing::MissionText
            | Showing::MissionInfo
            | Showing::PlayerInfo
    )
}

/// How long the music at `volume` fades out over: 4 ticks for each
/// Preferences level, at the level nearest `volume` (28 ticks at the top
/// level, 7). `_StopIntroMusic(1)` @0x61c82 starts at level x 32 and takes
/// 8 off a tick until it reaches 0, so the fade lasts level x 4 ticks.
#[must_use]
pub fn music_fade(volume: Volume) -> Duration {
    Duration::from_secs(4 * u64::from(volume_level(volume))) / 60
}

/// Whether music plays on `scene`: the main menu only, while the music is
/// not spent; the dialogs over it keep it. The original's intro music
/// starts once a launch (`_InitEVNova` @0x471ec, `_SpawnIntroMusicThread`
/// @0x61a5c), plays once and is not looped (`_MusicThread` @0x61e10 ends at
/// `IsMovieDone` @0x61e3d), and is cancelled for good when a pilot enters
/// the game (`_EnterGameFromMainScreen` @0x19ec6) or About opens
/// (`_DoAboutBox` @0x15fbf). Flight, the spaceport, the maps and the system
/// view never play it.
///
/// Known divergence: the original reads its Intro Music checkbox
/// (`-[EVPrefsDialogController accept:]` @0xcfeae) only at launch; here
/// turning the setting off stops the music at once, and turning it on
/// before the music is spent restarts it from the top.
fn has_music(scene: Showing) -> bool {
    match scene {
        Showing::MainMenu => true,
        // The game's screens never play it; the overlays are never the
        // scene, as they keep the one below.
        Showing::GalaxyMap
        | Showing::FlightMap
        | Showing::Spaceport
        | Showing::Flight
        | Showing::System
        | Showing::ShipBrowser
        | Showing::About
        | Showing::Preferences
        | Showing::NewPilot
        | Showing::OpenPilot
        | Showing::Note
        | Showing::Plunder
        | Showing::Assignment
        | Showing::Comm
        | Showing::Haggle
        | Showing::Offer
        | Showing::MissionText
        | Showing::MissionInfo
        | Showing::PlayerInfo => false,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::recording::{AudioLog, RecordingAudio};

    const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(UiSound::ButtonUp);
    const ALERT: Sound = Sound::Ui(UiSound::Alert);
    const THRUST: Sound = Sound::Sim(SimSound::ThrustStarted);
    const COAST: Sound = Sound::Sim(SimSound::ThrustStopped);
    const TOOK_OFF: Sound = Sound::Sim(SimSound::TookOff);
    const JUMP: Sound = Sound::Sim(SimSound::JumpBegan);
    const ARRIVED: Sound = Sound::Sim(SimSound::Arrived);

    fn landed(stellar_sound: Option<i16>) -> Sound {
        Sound::Sim(SimSound::Landed {
            stellar_sound: stellar_sound.map(SoundId),
        })
    }

    fn play(id: i16, volume: f32) -> AudioCommand {
        AudioCommand::Play {
            sound: SoundId(id),
            volume: Volume::new(volume),
        }
    }

    fn start_music(volume: f32) -> AudioCommand {
        AudioCommand::StartMusic {
            volume: Volume::new(volume),
        }
    }

    fn start_loop(id: i16, volume: f32) -> AudioCommand {
        AudioCommand::StartLoop {
            sound: SoundId(id),
            volume: Volume::new(volume),
        }
    }

    /// The original table: a core and its log.
    fn original() -> (AudioCore<RecordingAudio>, AudioLog) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        (AudioCore::new(audio), log)
    }

    /// A table with an engine (200) and a take-off sound (201).
    fn engine() -> (AudioCore<RecordingAudio>, AudioLog) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            engine: Some(SoundId(200)),
            take_off: Some(SoundId(201)),
            ..SoundTable::ORIGINAL
        };
        (AudioCore::with_table(audio, table), log)
    }

    /// The commands logged since the last call.
    fn drain(log: &AudioLog) -> Vec<AudioCommand> {
        std::mem::take(&mut *log.borrow_mut())
    }

    // Effects.

    #[test]
    fn each_event_plays_its_sound_at_the_effects_volume() {
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[JUMP, ARRIVED, DOWN, UP]);
        assert_eq!(
            drain(&log),
            [
                play(128, 1.0),
                play(130, 1.0),
                play(600, 1.0),
                play(601, 1.0)
            ]
        );
    }

    #[test]
    fn the_escort_menu_and_commands_beep_as_the_original_does() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Ui(UiSound::EscortMenuToggled),
                Sound::Sim(SimSound::EscortsCommanded),
                Sound::Ui(UiSound::EscortGroupSelected),
                Sound::Ui(UiSound::NoEscorts),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                play(150, 1.0),
                play(151, 1.0),
                play(152, 1.0),
                play(153, 1.0)
            ]
        );
    }

    #[test]
    fn the_nav_keys_beep_as_the_original_does() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::NavSelected),
                Sound::Sim(SimSound::HyperSelected),
                Sound::Sim(SimSound::TargetCleared),
            ],
        );
        assert_eq!(
            drain(&log),
            [play(150, 1.0), play(152, 1.0), play(150, 1.0)]
        );
    }

    #[test]
    fn a_tutorial_hint_beeps_154_and_a_told_refusal_153() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::TutorialHint),
                Sound::Sim(SimSound::Refused),
            ],
        );
        assert_eq!(drain(&log), [play(154, 0.5), play(153, 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::TutorialHint),
                Sound::Sim(SimSound::Refused),
            ],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn an_escort_repairing_itself_and_rejoining_beeps() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::EscortRepaired)],
        );
        assert_eq!(drain(&log), [play(154, 1.0)]);
    }

    #[test]
    fn an_announced_fleets_arrival_quote_beeps() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::FleetArrivalQuote)],
        );
        assert_eq!(drain(&log), [play(154, 1.0)]);
    }

    #[test]
    fn a_hail_refused_an_offer_on_a_hail_and_a_hail_quote_beep() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::Refused),
                Sound::Sim(SimSound::HailOffer),
                Sound::Sim(SimSound::HailQuote),
            ],
        );
        assert_eq!(
            drain(&log),
            [play(153, 1.0), play(154, 1.0), play(154, 1.0)]
        );
    }

    #[test]
    fn a_q_message_in_flight_beeps() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::ScriptMessage)],
        );
        assert_eq!(drain(&log), [play(154, 1.0)]);
        core.set_sound(false);
        drain(&log);
        core.update(None, &[Sound::Sim(SimSound::ScriptMessage)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_scan_beeps_151_for_a_failed_mission_and_154_for_smuggling() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::ScanFailedMission),
                Sound::Sim(SimSound::Smuggling),
            ],
        );
        assert_eq!(drain(&log), [play(151, 0.5), play(154, 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::ScanFailedMission),
                Sound::Sim(SimSound::Smuggling),
            ],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_jettison_in_flight_beeps_154_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::CargoJettisoned)],
        );
        assert_eq!(drain(&log), [play(154, 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::CargoJettisoned)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_red_alert_plays_370_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::RedAlert)],
        );
        assert_eq!(drain(&log), [play(370, 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::RedAlert)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_klaxon_plays_371_unless_still_playing_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Klaxon), Sound::Sim(SimSound::Klaxon)],
        );
        let klaxon = AudioCommand::PlayUnlessPlaying {
            sound: SoundId(371),
            volume: Volume::new(0.5),
        };
        assert_eq!(drain(&log), [klaxon, klaxon], "the adapter decides");
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Klaxon)]);
        assert_eq!(drain(&log), [], "sound off");
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            klaxon: None,
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Klaxon)]);
        assert_eq!(drain(&log), [], "a table without one");
    }

    #[test]
    fn the_eject_plays_snd_372() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Ejected), Sound::Sim(SimSound::Ejected)],
        );
        assert_eq!(drain(&log), [play(372, 0.5), play(372, 0.5)], "every time");
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Ejected)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_sabotage_outfit_going_off_plays_snd_300() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::OnboardExplosion),
                Sound::Sim(SimSound::OnboardExplosion),
            ],
        );
        assert_eq!(drain(&log), [play(300, 0.5), play(300, 0.5)], "every time");
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::OnboardExplosion)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_collapsed_jump_beeps_151() {
        const WARP_UP: [SoundId; 2] = [SoundId(128), SoundId(129)];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::Halt(&WARP_UP)),
                Sound::Sim(SimSound::JumpCollapsed),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                AudioCommand::Halt {
                    sound: SoundId(128)
                },
                AudioCommand::Halt {
                    sound: SoundId(129)
                },
                play(151, 0.5),
            ]
        );
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::JumpCollapsed)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_halt_halts_each_named_sound_in_order() {
        const HALTS: [SoundId; 4] = [SoundId(371), SoundId(128), SoundId(129), SoundId(130)];
        let halts: Vec<AudioCommand> = HALTS
            .iter()
            .map(|&sound| AudioCommand::Halt { sound })
            .collect();
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Halt(&HALTS))],
        );
        assert_eq!(drain(&log), halts);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Halt(&HALTS))],
        );
        assert_eq!(drain(&log), halts, "sound off halts too");
    }

    #[test]
    fn the_news_dialog_beeps_as_it_opens_and_closes() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Ui(UiSound::NewsOpened),
                Sound::Ui(UiSound::NewsClosed),
            ],
        );
        assert_eq!(drain(&log), [play(151, 1.0), play(152, 1.0)]);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            news_open: Some(SoundId(300)),
            news_close: Some(SoundId(301)),
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Ui(UiSound::NewsClosed),
                Sound::Ui(UiSound::NewsOpened),
            ],
        );
        assert_eq!(drain(&log), [play(301, 1.0), play(300, 1.0)], "the table's");
    }

    #[test]
    fn a_refusal_beeps_beep4() {
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Refused)]);
        assert_eq!(drain(&log), [play(153, 1.0)]);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            refused: Some(SoundId(302)),
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Refused)]);
        assert_eq!(drain(&log), [play(302, 1.0)], "the table's");
    }

    #[test]
    fn the_player_downed_and_its_failed_mission_each_beep_their_table_entry() {
        const DOWNED: [Sound; 2] = [
            Sound::Sim(SimSound::ShipDowned),
            Sound::Sim(SimSound::DownedMissionFailed),
        ];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &DOWNED);
        assert_eq!(drain(&log), [play(151, 0.5), play(151, 0.5)]);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            ship_downed: Some(SoundId(301)),
            downed_mission_failed: Some(SoundId(302)),
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &DOWNED);
        assert_eq!(drain(&log), [play(301, 1.0), play(302, 1.0)], "the table's");
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &DOWNED);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn traffic_control_beeps_its_table_entry() {
        const ANSWERED: [Sound; 1] = [Sound::Sim(SimSound::TrafficControl)];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &ANSWERED);
        assert_eq!(drain(&log), [play(151, 0.5)]);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            traffic_control: Some(SoundId(303)),
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &ANSWERED);
        assert_eq!(drain(&log), [play(303, 1.0)], "the table's");
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &ANSWERED);
        assert_eq!(drain(&log), [], "sound off");
    }

    fn mission(id: i16, exclusive: bool) -> Sound {
        Sound::Sim(SimSound::Script {
            sound: SoundId(id),
            exclusive,
        })
    }

    #[test]
    fn a_mission_sound_plays_on_its_channel_or_over_the_rest() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[mission(300, true), mission(301, false)],
        );
        assert_eq!(
            drain(&log),
            [
                AudioCommand::PlayExclusive {
                    sound: SoundId(300),
                    volume: Volume::new(0.5),
                },
                play(301, 0.5)
            ]
        );
        core.set_sound(false);
        drain(&log);
        core.update(None, &[mission(300, true), mission(301, false)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_landing_plays_the_beep_then_the_stellars_own_sound() {
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[landed(Some(10_032))]);
        assert_eq!(drain(&log), [play(151, 1.0), play(10_032, 1.0)]);
        core.update(None, &[landed(None)]);
        assert_eq!(drain(&log), [play(151, 1.0)]);
    }

    #[test]
    fn the_original_thrusts_and_takes_off_in_silence() {
        let (mut core, log) = original();
        core.update(Some(Showing::Flight), &[]);
        drain(&log);
        core.update(Some(Showing::Flight), &[THRUST, TOOK_OFF, COAST]);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn an_alert_plays_the_tables_alert_and_the_original_none() {
        let (mut core, log) = original();
        core.update(Some(Showing::Spaceport), &[]);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[ALERT]);
        assert_eq!(drain(&log), []);
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            alert: Some(SoundId(150)),
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &[ALERT]);
        assert_eq!(drain(&log), [play(150, 1.0)]);
    }

    #[test]
    fn a_take_off_sound_plays_when_the_table_has_one() {
        let (mut core, log) = engine();
        core.update(Some(Showing::ShipBrowser), &[TOOK_OFF]);
        assert_eq!(drain(&log), [play(201, 1.0)]);
    }

    #[test]
    fn a_table_without_a_sound_plays_nothing_for_it() {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            landing: None,
            jump: None,
            ..SoundTable::ORIGINAL
        };
        let mut core = AudioCore::with_table(audio, table);
        core.update(Some(Showing::ShipBrowser), &[landed(Some(10_000)), JUMP]);
        assert_eq!(drain(&log), [play(10_000, 1.0)]);
    }

    // The engine.

    #[test]
    fn the_engine_loops_while_the_ship_thrusts_in_flight() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_loop(200, 1.0)]);
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [], "already looping");
        core.update(Some(Showing::Flight), &[COAST]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop]);
        core.update(Some(Showing::Flight), &[COAST]);
        assert_eq!(drain(&log), [], "already stopped");
        core.update(Some(Showing::Flight), &[THRUST, COAST, THRUST]);
        assert_eq!(
            drain(&log),
            [
                start_loop(200, 1.0),
                AudioCommand::StopLoop,
                start_loop(200, 1.0)
            ]
        );
    }

    #[test]
    fn the_loop_stops_before_the_sounds_that_follow_it() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.update(Some(Showing::Flight), &[COAST, landed(None)]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop, play(151, 1.0)]);
    }

    #[test]
    fn leaving_flight_stops_the_engine_and_forgets_the_thrust() {
        for away in [
            Showing::FlightMap,
            Showing::Spaceport,
            Showing::GalaxyMap,
            Showing::ShipBrowser,
            Showing::System,
        ] {
            let (mut core, log) = engine();
            core.set_music(false);
            core.update(Some(Showing::Flight), &[THRUST]);
            drain(&log);
            core.update(Some(away), &[]);
            assert_eq!(drain(&log), [AudioCommand::StopLoop], "{away:?}");
            core.update(Some(Showing::Flight), &[]);
            assert_eq!(drain(&log), [], "{away:?}: not thrusting on return");
        }
    }

    #[test]
    fn thrust_away_from_flight_does_not_loop() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Spaceport), &[THRUST]);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::Flight), &[]);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn the_about_text_and_no_screen_keep_flight_and_its_engine() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.update(Some(Showing::About), &[]);
        core.update(None, &[]);
        assert_eq!(drain(&log), []);
        core.update(None, &[COAST]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop]);
    }

    // The fight's sounds.

    fn fight(id: i16, offset: (i32, i32)) -> Sound {
        Sound::Combat(nova_view::sound::CombatSound {
            sound: SoundId(id),
            offset,
        })
    }

    #[test]
    fn a_sound_within_200_pixels_is_heard_at_full_volume() {
        for offset in [(0, 0), (200, 0), (0, -200), (120, 160), (-141, 141)] {
            assert_eq!(distance_gain(offset), 1.0, "{offset:?}");
        }
    }

    #[test]
    fn a_far_sound_is_heard_at_an_eighth() {
        for offset in [
            (10_000, 0),
            (0, -10_000),
            (5000, 5000),
            (i32::MAX, i32::MIN),
        ] {
            assert_eq!(distance_gain(offset), 0.125, "{offset:?}");
        }
    }

    #[test]
    fn a_sound_far_across_falls_off_faster_than_one_far_up_or_down() {
        // d² 160,000: 722,500 / d² is over 1, and 40,000 / d² a quarter.
        assert_eq!(distance_gain((0, 400)), 1.0);
        assert_eq!(distance_gain((400, 0)), 0.625);
        assert_eq!(distance_gain((-400, 0)), 0.625);
        // d² 1,000,000: 0.7225, and 0.04 floored at an eighth.
        assert!((distance_gain((0, 1000)) - 0.7225).abs() < 1e-6);
        assert!((distance_gain((1000, 0)) - 0.42375).abs() < 1e-6);
        // 200 across is not past 200.
        assert!((distance_gain((200, 980)) - 722_500.0 / 1_000_400.0).abs() < 1e-6);
        assert!((distance_gain((201, 980)) - 0.423_461).abs() < 1e-5);
    }

    #[test]
    fn a_fights_sound_plays_at_the_effects_volume_by_its_distance() {
        let (mut core, log) = original();
        core.set_music(false);
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::Flight),
            &[fight(208, (0, 0)), fight(302, (400, 0))],
        );
        assert_eq!(drain(&log), [play(208, 0.5), play(302, 0.3125)]);
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::Flight), &[fight(208, (0, 0))]);
        assert_eq!(drain(&log), [], "sound off");
    }

    // Music.

    fn fade_out(volume: f32) -> AudioCommand {
        AudioCommand::FadeOutMusic(music_fade(Volume::new(volume)))
    }

    /// The original table on a mock whose music can be ended: a core, its
    /// log and the mock's other handle.
    fn ending() -> (AudioCore<RecordingAudio>, AudioLog, RecordingAudio) {
        let audio = RecordingAudio::new();
        let other = audio.clone();
        let log = audio.log();
        (AudioCore::new(audio), log, other)
    }

    #[test]
    fn music_plays_on_the_main_menu_only() {
        for scene in [
            Showing::ShipBrowser,
            Showing::GalaxyMap,
            Showing::System,
            Showing::Flight,
            Showing::FlightMap,
            Showing::Spaceport,
        ] {
            let (mut core, log) = original();
            core.update(Some(scene), &[]);
            assert_eq!(drain(&log), [], "{scene:?}");
        }
        for overlay in [
            Showing::About,
            Showing::NewPilot,
            Showing::OpenPilot,
            Showing::Note,
        ] {
            let (mut core, log) = original();
            core.update(Some(overlay), &[]);
            assert_eq!(drain(&log), [], "{overlay:?} over nothing");
        }
        let (mut core, log) = original();
        core.update(None, &[]);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(1.0)]);
    }

    #[test]
    fn leaving_the_main_menu_fades_the_music_out_for_good() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[]);
        assert_eq!(drain(&log), [fade_out(1.0)]);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [], "back on the main menu");
        core.set_music(false);
        core.set_music(true);
        assert_eq!(drain(&log), [], "the setting off and on");
    }

    #[test]
    fn about_over_the_main_menu_fades_it_out_for_good() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::About), &[]);
        assert_eq!(drain(&log), [fade_out(1.0)]);
        core.update(Some(Showing::MainMenu), &[]);
        core.set_music(false);
        core.set_music(true);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn the_menu_dialogs_keep_the_music() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        for overlay in [
            Showing::Preferences,
            Showing::NewPilot,
            Showing::OpenPilot,
            Showing::Note,
        ] {
            core.update(Some(overlay), &[]);
            core.update(Some(Showing::MainMenu), &[]);
            assert_eq!(drain(&log), [], "{overlay:?}");
        }
    }

    #[test]
    fn the_fade_lasts_four_ticks_a_preferences_level() {
        use crate::settings::level_volume;
        assert_eq!(music_fade(level_volume(7)), Duration::from_secs(28) / 60);
        assert_eq!(music_fade(Volume::FULL), Duration::from_secs(28) / 60);
        assert_eq!(music_fade(level_volume(2)), Duration::from_secs(8) / 60);
        assert_eq!(music_fade(level_volume(1)), Duration::from_secs(4) / 60);
        assert_eq!(music_fade(Volume::new(0.25)), Duration::from_secs(8) / 60);
        assert_eq!(music_fade(Volume::SILENT), Duration::ZERO);
        let (mut core, log) = original();
        core.set_music_volume(Volume::new(0.25));
        core.update(Some(Showing::MainMenu), &[]);
        core.update(Some(Showing::Flight), &[]);
        assert_eq!(drain(&log), [start_music(0.25), fade_out(0.25)]);
    }

    /// An engine divergence the original lacks: it reads the checkbox only
    /// at launch.
    #[test]
    fn engine_divergence_music_toggled_on_the_main_menu_stops_and_restarts_it() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.set_music(false);
        assert_eq!(drain(&log), [AudioCommand::StopMusic]);
        core.set_music(true);
        assert_eq!(drain(&log), [start_music(1.0)], "from the top");
    }

    #[test]
    fn music_played_to_its_end_is_spent() {
        let (mut core, log, audio) = ending();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        audio.end_music();
        core.update(None, &[]);
        assert_eq!(drain(&log), [], "it stopped on its own");
        core.set_music(false);
        core.set_music(true);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [], "the setting off and on starts nothing");
    }

    #[test]
    fn music_off_before_its_end_is_not_spent_by_it() {
        let (mut core, log, audio) = ending();
        core.update(Some(Showing::MainMenu), &[]);
        core.set_music(false);
        drain(&log);
        audio.end_music();
        core.set_music(true);
        assert_eq!(drain(&log), [start_music(1.0)]);
    }

    #[test]
    fn the_screen_is_taken_in_before_the_sounds() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_loop(200, 1.0)]);
        core.update(Some(Showing::Spaceport), &[COAST, landed(None)]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop, play(151, 1.0)]);
    }

    // The settings.

    #[test]
    fn music_off_stops_it_and_keeps_it_stopped() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.set_music(false);
        assert!(!core.settings().music);
        assert_eq!(drain(&log), [AudioCommand::StopMusic]);
        core.set_music(false);
        for scene in [Showing::NewPilot, Showing::MainMenu, Showing::Note] {
            core.update(Some(scene), &[]);
        }
        assert_eq!(drain(&log), []);
        core.set_music(true);
        assert!(core.settings().music);
        assert_eq!(drain(&log), [start_music(1.0)], "on the main menu");
        core.set_music(true);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn music_on_away_from_a_music_scene_waits_for_one() {
        let (mut core, log) = original();
        core.set_music(false);
        core.update(None, &[]);
        core.set_music(true);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(1.0)]);
    }

    #[test]
    fn sound_off_stops_the_effects_and_plays_none() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.set_sound(false);
        assert!(!core.settings().sound);
        assert_eq!(drain(&log), [AudioCommand::StopEffects]);
        core.set_sound(false);
        assert_eq!(drain(&log), [], "already off");
        core.update(
            Some(Showing::Flight),
            &[
                COAST,
                THRUST,
                landed(Some(10_032)),
                TOOK_OFF,
                JUMP,
                ARRIVED,
                DOWN,
                UP,
            ],
        );
        assert_eq!(drain(&log), []);
        core.set_sound(true);
        assert!(core.settings().sound);
        assert_eq!(drain(&log), [start_loop(200, 1.0)], "still thrusting");
        core.set_sound(true);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::Flight), &[COAST]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop]);
    }

    #[test]
    fn sound_off_leaves_the_music_and_music_off_leaves_the_sound() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.set_sound(false);
        core.update(Some(Showing::Spaceport), &[]);
        assert_eq!(drain(&log), [AudioCommand::StopEffects]);
        let (mut core, log) = engine();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.set_music(false);
        core.update(Some(Showing::MainMenu), &[JUMP]);
        assert_eq!(drain(&log), [AudioCommand::StopMusic, play(128, 1.0)]);
    }

    #[test]
    fn the_effects_and_music_volumes_are_separate() {
        let (mut core, log) = engine();
        core.set_effects_volume(Volume::new(0.5));
        core.set_music_volume(Volume::new(0.25));
        assert_eq!(drain(&log), [], "nothing playing to change");
        assert_eq!(core.settings().effects_volume, Volume::new(0.5));
        assert_eq!(core.settings().music_volume, Volume::new(0.25));
        core.update(Some(Showing::MainMenu), &[JUMP]);
        assert_eq!(drain(&log), [start_music(0.25), play(128, 0.5)]);
        core.set_music_volume(Volume::new(0.125));
        assert_eq!(
            drain(&log),
            [AudioCommand::SetMusicVolume(Volume::new(0.125))]
        );
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [fade_out(0.125), start_loop(200, 0.5)]);
        core.set_effects_volume(Volume::new(0.75));
        assert_eq!(
            drain(&log),
            [AudioCommand::SetLoopVolume(Volume::new(0.75))]
        );
        core.update(Some(Showing::Flight), &[ARRIVED]);
        assert_eq!(drain(&log), [play(130, 0.75)]);
    }

    #[test]
    fn a_volume_change_with_the_loop_or_music_stopped_sends_nothing() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST, COAST]);
        core.update(Some(Showing::ShipBrowser), &[]);
        drain(&log);
        core.set_effects_volume(Volume::new(0.5));
        core.set_music_volume(Volume::new(0.5));
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::Flight), &[THRUST]);
        core.set_sound(false);
        drain(&log);
        core.set_effects_volume(Volume::new(0.25));
        assert_eq!(drain(&log), [], "sound off stopped the loop");
    }

    #[test]
    fn the_preferences_dialog_keeps_the_scene_below_it() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.update(Some(Showing::Preferences), &[]);
        assert_eq!(
            drain(&log),
            [AudioCommand::StopLoop],
            "the engine stops, as flight is paused"
        );
        core.update(Some(Showing::Flight), &[]);
        assert_eq!(drain(&log), [], "the thrust is forgotten on return");
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_loop(200, 1.0)], "a fresh thrust");
        let (mut core, log) = original();
        core.update(Some(Showing::Preferences), &[]);
        assert_eq!(drain(&log), [], "over nothing");
        core.update(Some(Showing::ShipBrowser), &[]);
        core.update(Some(Showing::Preferences), &[]);
        assert_eq!(drain(&log), [], "over the ship browser, still silent");
    }

    #[test]
    fn the_boarding_and_comm_dialogs_pause_flight() {
        for overlay in [
            Showing::Plunder,
            Showing::Assignment,
            Showing::Comm,
            Showing::Haggle,
            Showing::Offer,
            Showing::MissionText,
            Showing::MissionInfo,
            Showing::PlayerInfo,
        ] {
            let (mut core, log) = engine();
            core.update(Some(Showing::Flight), &[THRUST]);
            drain(&log);
            core.update(Some(overlay), &[]);
            assert_eq!(
                drain(&log),
                [AudioCommand::StopLoop],
                "{overlay:?}: the engine stops"
            );
            core.update(Some(Showing::Flight), &[]);
            assert_eq!(drain(&log), [], "{overlay:?}: the thrust is forgotten");
            assert!(is_overlay(overlay));
            assert!(!has_music(overlay));
        }
    }

    // Applying settings.

    fn quiet() -> AudioSettings {
        AudioSettings {
            sound: true,
            music: true,
            effects_volume: Volume::new(0.5),
            music_volume: Volume::new(0.25),
        }
    }

    #[test]
    fn a_core_can_start_from_given_settings() {
        let (core, log) = engine();
        let mut core = core.with_settings(quiet());
        assert_eq!(core.settings(), quiet());
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(0.25)]);
        core.update(Some(Showing::Flight), &[THRUST, JUMP]);
        assert_eq!(
            drain(&log),
            [fade_out(0.25), start_loop(200, 0.5), play(128, 0.5)]
        );
        let (core, log) = engine();
        let mut core = core.with_settings(AudioSettings {
            music: false,
            sound: false,
            ..quiet()
        });
        core.update(Some(Showing::Flight), &[THRUST, JUMP]);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn applying_sound_off_stops_the_effects_and_the_loop() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.apply(AudioSettings {
            sound: false,
            ..AudioSettings::default()
        });
        assert_eq!(drain(&log), [AudioCommand::StopEffects]);
        assert!(!core.settings().sound);
        core.update(Some(Showing::Flight), &[JUMP]);
        assert_eq!(drain(&log), [], "silent");
        core.apply(AudioSettings::default());
        assert_eq!(drain(&log), [start_loop(200, 1.0)], "still thrusting");
    }

    #[test]
    fn applying_music_off_and_on_stops_and_starts_it() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        let off = AudioSettings {
            music: false,
            ..AudioSettings::default()
        };
        core.apply(off);
        assert_eq!(drain(&log), [AudioCommand::StopMusic]);
        assert_eq!(core.settings(), off);
        core.apply(AudioSettings::default());
        assert_eq!(drain(&log), [start_music(1.0)]);
    }

    #[test]
    fn applying_a_volume_changes_what_plays_and_what_plays_next() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.apply(quiet());
        assert_eq!(
            drain(&log),
            [AudioCommand::SetLoopVolume(Volume::new(0.5))],
            "no music in flight"
        );
        assert_eq!(core.settings(), quiet());
        core.update(Some(Showing::Flight), &[JUMP]);
        assert_eq!(drain(&log), [play(128, 0.5)]);
        let (mut core, log) = engine();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.apply(quiet());
        assert_eq!(
            drain(&log),
            [AudioCommand::SetMusicVolume(Volume::new(0.25))]
        );
    }

    #[test]
    fn applying_a_volume_with_nothing_playing_sends_nothing() {
        let (mut core, log) = engine();
        core.update(None, &[]);
        core.apply(quiet());
        assert_eq!(drain(&log), []);
        assert_eq!(core.settings(), quiet());
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(0.25)]);
    }

    #[test]
    fn applying_music_on_with_a_new_volume_starts_it_at_that_volume() {
        let (mut core, log) = original();
        core.set_music(false);
        core.update(Some(Showing::MainMenu), &[]);
        core.apply(quiet());
        assert_eq!(drain(&log), [start_music(0.25)]);
    }

    #[test]
    fn applying_the_settings_already_in_place_sends_nothing() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.apply(AudioSettings::default());
        assert_eq!(drain(&log), []);
        let mut core = core.with_settings(quiet());
        core.apply(quiet());
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn the_core_plays_through_its_port() {
        let (mut core, _) = original();
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(*core.audio().log().borrow(), [start_music(1.0)]);
    }
}
