//! The audio core: sound events and the screen shown in, playback
//! commands out.

use nova_data::SoundId;
use nova_view::Showing;
use nova_view::sound::{SimSound, Sound, UiSound};

use crate::port::{Audio, AudioCommand, Volume};
use crate::settings::AudioSettings;
use crate::table::SoundTable;

/// Decides what plays: turns each frame's sound events and the screen
/// shown into commands on its [`Audio`] port.
///
/// It remembers what it has asked the port for (the music on, the engine
/// loop on) and what it has been told (the scene, whether the ship is
/// thrusting), and sends a command only when what it wants changes.
///
/// - Music plays in menus (the galaxy map, flight's course map and the
///   spaceport) and in space (flight, and a system opened from the map),
///   while the music setting is on. The ship browser, the developer's
///   viewer the app opens on, is silent. The About text and the
///   Preferences dialog, over another screen, keep the scene below them. Moving between music scenes does
///   not restart the track.
/// - Each event with a `snd ` in the table plays it once, at the effects
///   volume, while the sound setting is on. A landing plays the table's
///   landing sound, then the stellar's own.
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
    /// Whether the music has been started and not stopped since.
    music_on: bool,
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
            music_on: false,
            loop_on: false,
        }
    }

    /// Takes in one frame's worth: the screen `showing` (`None` keeps the
    /// scene as it was), then each of `sounds` in order.
    pub fn update(&mut self, showing: Option<Showing>, sounds: &[Sound]) {
        if let Some(scene) = showing.filter(|&scene| !is_overlay(scene)) {
            self.scene = Some(scene);
        }
        // The Preferences dialog, the boarding dialogs and the comm
        // dialogs pause flight below them, and flight lets go of its keys,
        // so the thrust is forgotten as for leaving it.
        let pauses = matches!(
            showing,
            Some(
                Showing::Preferences
                    | Showing::Plunder
                    | Showing::Assignment
                    | Showing::Comm
                    | Showing::Haggle
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
                Sound::Ui(UiSound::ButtonDown) => self.play(self.table.button_down),
                Sound::Ui(UiSound::ButtonUp) => self.play(self.table.button_up),
                Sound::Ui(UiSound::Alert) => self.play(self.table.alert),
                Sound::Combat(fight) => {
                    let gain = distance_gain(fight.offset);
                    let volume = self.settings.effects_volume.amplitude() * gain;
                    self.play_at(Some(fight.sound), Volume::new(volume));
                }
            }
            self.reconcile();
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
        let music = self.settings.music && self.scene.is_some_and(has_music);
        if music != self.music_on {
            self.music_on = music;
            self.audio.run(if music {
                AudioCommand::StartMusic {
                    volume: self.settings.music_volume,
                }
            } else {
                AudioCommand::StopMusic
            });
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
        if self.music_on {
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
/// entry and the saved pilots' list over the main menu, and the plunder,
/// assignment, comm and haggle dialogs over flight.
fn is_overlay(showing: Showing) -> bool {
    matches!(
        showing,
        Showing::About
            | Showing::Preferences
            | Showing::NewPilot
            | Showing::OpenPilot
            | Showing::Plunder
            | Showing::Assignment
            | Showing::Comm
            | Showing::Haggle
    )
}

/// Whether music plays on `scene`: menus and space do; the ship browser,
/// the developer's viewer, does not.
fn has_music(scene: Showing) -> bool {
    match scene {
        Showing::MainMenu
        | Showing::GalaxyMap
        | Showing::FlightMap
        | Showing::Spaceport
        | Showing::Flight
        | Showing::System => true,
        // The overlays are never the scene: they keep the one below.
        Showing::ShipBrowser
        | Showing::About
        | Showing::Preferences
        | Showing::NewPilot
        | Showing::OpenPilot
        | Showing::Plunder
        | Showing::Assignment
        | Showing::Comm
        | Showing::Haggle => false,
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

    #[test]
    fn music_plays_in_menus_and_in_space_without_restarting() {
        let (mut core, log) = original();
        core.update(None, &[]);
        core.update(Some(Showing::ShipBrowser), &[]);
        assert_eq!(drain(&log), [], "the ship browser is silent");
        core.update(Some(Showing::GalaxyMap), &[]);
        assert_eq!(drain(&log), [start_music(1.0)]);
        for scene in [
            Showing::System,
            Showing::Flight,
            Showing::FlightMap,
            Showing::Spaceport,
            Showing::About,
            Showing::MainMenu,
            Showing::NewPilot,
            Showing::OpenPilot,
            Showing::GalaxyMap,
        ] {
            core.update(Some(scene), &[]);
            assert_eq!(drain(&log), [], "{scene:?} keeps it playing");
        }
        core.update(None, &[]);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::ShipBrowser), &[]);
        assert_eq!(drain(&log), [AudioCommand::StopMusic]);
        core.update(Some(Showing::About), &[]);
        assert_eq!(drain(&log), [], "over the ship browser, still silent");
        core.update(Some(Showing::Flight), &[]);
        assert_eq!(drain(&log), [start_music(1.0)], "from the top");
    }

    #[test]
    fn each_music_scene_starts_the_music() {
        for scene in [
            Showing::GalaxyMap,
            Showing::System,
            Showing::Flight,
            Showing::FlightMap,
            Showing::Spaceport,
            Showing::MainMenu,
        ] {
            let (mut core, log) = original();
            core.update(Some(scene), &[]);
            assert_eq!(drain(&log), [start_music(1.0)], "{scene:?}");
        }
        for overlay in [Showing::About, Showing::NewPilot, Showing::OpenPilot] {
            let (mut core, log) = original();
            core.update(Some(overlay), &[]);
            assert_eq!(drain(&log), [], "{overlay:?} over nothing");
        }
    }

    #[test]
    fn the_screen_is_taken_in_before_the_sounds() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_music(1.0), start_loop(200, 1.0)]);
        core.update(Some(Showing::Spaceport), &[COAST, landed(None)]);
        assert_eq!(drain(&log), [AudioCommand::StopLoop, play(151, 1.0)]);
    }

    // The settings.

    #[test]
    fn music_off_stops_it_and_keeps_it_stopped() {
        let (mut core, log) = original();
        core.update(Some(Showing::GalaxyMap), &[]);
        drain(&log);
        core.set_music(false);
        assert!(!core.settings().music);
        assert_eq!(drain(&log), [AudioCommand::StopMusic]);
        core.set_music(false);
        for scene in [Showing::Flight, Showing::ShipBrowser, Showing::Spaceport] {
            core.update(Some(scene), &[]);
        }
        assert_eq!(drain(&log), []);
        core.set_music(true);
        assert!(core.settings().music);
        assert_eq!(drain(&log), [start_music(1.0)], "in the spaceport");
        core.set_music(true);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn music_on_away_from_a_music_scene_waits_for_one() {
        let (mut core, log) = original();
        core.set_music(false);
        core.update(Some(Showing::ShipBrowser), &[]);
        core.set_music(true);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::GalaxyMap), &[]);
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
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.set_music(false);
        core.update(Some(Showing::Flight), &[JUMP]);
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
        core.update(Some(Showing::Flight), &[THRUST, JUMP]);
        assert_eq!(
            drain(&log),
            [start_music(0.25), start_loop(200, 0.5), play(128, 0.5)]
        );
        core.set_effects_volume(Volume::new(0.75));
        assert_eq!(
            drain(&log),
            [AudioCommand::SetLoopVolume(Volume::new(0.75))]
        );
        core.set_music_volume(Volume::new(0.125));
        assert_eq!(
            drain(&log),
            [AudioCommand::SetMusicVolume(Volume::new(0.125))]
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
            "the music goes on; the engine stops, as flight is paused"
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
    fn the_boarding_and_comm_dialogs_pause_flight_and_keep_its_music() {
        for overlay in [
            Showing::Plunder,
            Showing::Assignment,
            Showing::Comm,
            Showing::Haggle,
        ] {
            let (mut core, log) = engine();
            core.update(Some(Showing::Flight), &[THRUST]);
            drain(&log);
            core.update(Some(overlay), &[]);
            assert_eq!(
                drain(&log),
                [AudioCommand::StopLoop],
                "{overlay:?}: the music goes on; the engine stops"
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
        core.update(Some(Showing::Flight), &[THRUST, JUMP]);
        assert_eq!(
            drain(&log),
            [start_music(0.25), start_loop(200, 0.5), play(128, 0.5)]
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
        core.update(Some(Showing::Spaceport), &[]);
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
            [
                AudioCommand::SetLoopVolume(Volume::new(0.5)),
                AudioCommand::SetMusicVolume(Volume::new(0.25)),
            ]
        );
        assert_eq!(core.settings(), quiet());
        core.update(Some(Showing::Flight), &[JUMP]);
        assert_eq!(drain(&log), [play(128, 0.5)]);
    }

    #[test]
    fn applying_a_volume_with_nothing_playing_sends_nothing() {
        let (mut core, log) = engine();
        core.update(Some(Showing::ShipBrowser), &[]);
        core.apply(quiet());
        assert_eq!(drain(&log), []);
        assert_eq!(core.settings(), quiet());
        core.update(Some(Showing::GalaxyMap), &[]);
        assert_eq!(drain(&log), [start_music(0.25)]);
    }

    #[test]
    fn applying_music_on_with_a_new_volume_starts_it_at_that_volume() {
        let (mut core, log) = original();
        core.set_music(false);
        core.update(Some(Showing::GalaxyMap), &[]);
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
        core.update(Some(Showing::GalaxyMap), &[]);
        assert_eq!(*core.audio().log().borrow(), [start_music(1.0)]);
    }
}
