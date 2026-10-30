//! The audio core: sound events and the screen shown in, playback
//! commands out.

use std::time::Duration;

use nova_data::SoundId;
use nova_view::Showing;
use nova_view::sound::{CombatSound, MAIN_SCREEN_SOUNDS, SimSound, Sound, SoundCue, UiSound};

use crate::port::{Audio, AudioCommand, PlaybackRate, Priority, Volume};
use crate::settings::{AudioSettings, music_volume_level};
use crate::table::SoundTable;

/// Decides what plays: turns each frame's sound events and the screen
/// shown into commands on its [`Audio`] port.
///
/// It remembers what it has asked the port for (the music on, the engine
/// loop on) and what it has been told (the scene, whether the ship is
/// thrusting), and sends a command only when what it wants changes.
///
/// - Music is the original's one-shot intro track: it plays on the main
///   menu, and the dialogs over it, while the music setting is on (the
///   scene rule in `has_music`: the main menu and its dialogs only;
///   flight, the spaceport, the maps and the system view are silent). It
///   is spent for good, as in the original, once it has
///   played to its end, or once a pilot enters the game or the About text
///   opens, which fade it out ([`music_fade`]). Spent music never plays
///   again in the run.
/// - Known divergence: the original reads its Intro Music checkbox only
///   at launch. Here the music setting acts at once: off stops the music,
///   and on, before the music is spent, starts it again from the top.
/// - Each of the flight session's and the interface's sounds plays its own
///   cue ([`SimSound::cue`], [`UiSound::cue`]) once, at the effects volume
///   and the cue's priority, while the sound setting is on: on the one
///   mission channel when it is exclusive ([`AudioCommand::PlayExclusive`]),
///   pitched when it is off its own rate ([`AudioCommand::PlayAtRate`]). The
///   core keeps only the sounds that need its state: the thrust (the engine
///   loop), the klaxon, which plays unless it is still playing
///   ([`AudioCommand::PlayUnlessPlaying`], which the port decides), the
///   halts, an explosion, heard by its distance as a fight's sound is, the
///   keyed click, the looped port ambient and the intro picture.
/// - The intro screen's pictures keep the music; a picture's own `snd `
///   ([`UiSound::IntroPicture`]) spends it, then plays like any event, and
///   the intro's text spends it.
/// - Switching the app out ([`UiSound::SwitchedOut`]) fades the music and
///   spends it, but only while it plays, whatever the sound setting.
/// - Entering play halts the main screen's four sounds
///   ([`UiSound::MainScreenSoundsHalted`]), whatever the sound setting.
/// - The spaceport's ambient sound plays its cue the same way: over
///   anything, or unless still playing for a looped one
///   ([`UiSound::PortAmbient`]); the spaceport's close halts it
///   ([`UiSound::PortAmbientHalted`]), whatever the sound setting.
/// - A key's click on a main-menu button plays the button-down sound,
///   then the button-up sound once the port says the first has ended.
///   Until then the click is waited on ([`AudioCore::click_waiting`]):
///   the main menu holds the key's choice until the app sees it is not.
/// - A halt halts each `snd ` it names, in order ([`AudioCommand::Halt`]),
///   whatever the sound setting: halting what is not playing is harmless.
/// - Each sound plays at its cue's priority, by which the adapter's voice
///   list keeps or drops it.
/// - Each of a fight's sounds plays its own `snd ` once, at the effects
///   volume times its [`distance_gain`] and at its own priority, while the
///   sound setting is on; one that waits ([`CombatSound::until_stopped`])
///   is skipped while its `snd ` plays on any voice.
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
    /// A keyed click's second half: the `snd ` it waits on to end, then
    /// the cue it plays.
    keyed: Option<(SoundId, SoundCue)>,
}

impl<A: Audio> AudioCore<A> {
    /// A core playing the original game's sounds through `audio`, with
    /// the default settings.
    pub fn new(audio: A) -> Self {
        Self::with_table(audio, SoundTable::ORIGINAL)
    }

    /// A core playing through `audio` with `table`'s engine loop, with the
    /// default settings.
    pub fn with_table(audio: A, table: SoundTable) -> Self {
        Self {
            audio,
            table,
            settings: AudioSettings::default(),
            scene: None,
            thrusting: false,
            music: Music::Stopped,
            loop_on: false,
            keyed: None,
        }
    }

    /// Whether a keyed click is still waited on: its button-down sound
    /// was asked for, and its button-up sound has not been played yet, as
    /// `_PlayMainScreenClickSound` @0x177af spins until 600 has ended
    /// (@0x177e0-0x177f2) and plays 601 before `_MainScreenChoice` runs.
    /// False before any keyed click, with sound off, and from the
    /// [`update`](Self::update) that plays the button-up sound.
    #[must_use]
    pub fn click_waiting(&self) -> bool {
        self.keyed.is_some()
    }

    /// Takes in one frame's worth: the screen `showing` (`None` keeps the
    /// scene as it was), then each of `sounds` in order.
    pub fn update(&mut self, showing: Option<Showing>, sounds: &[Sound]) {
        if let Some((waiting, then)) = self.keyed
            && !self.audio.playing(waiting)
        {
            self.keyed = None;
            self.play_cue(Some(then));
        }
        if let Some(scene) = showing.filter(|&scene| !is_overlay(scene)) {
            self.scene = Some(scene);
        }
        // Entering the game (`_EnterGameFromMainScreen` @0x19ec6, and
        // `_ShowIntroScreen` @0x170ad before its text) and the About box
        // (`_DoAboutBox` @0x15fbf) cancel the music for good; the intro's
        // pictures keep it.
        if showing == Some(Showing::About)
            || self
                .scene
                .is_some_and(|scene| !matches!(scene, Showing::MainMenu | Showing::Intro))
        {
            self.spend_music();
        }
        // The Preferences dialog, the boarding dialogs (the rename among
        // them), the comm dialogs,
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
                    | Showing::CaptureNaming
                    | Showing::Comm
                    | Showing::EscortDialog
                    | Showing::Haggle
                    | Showing::Offer
                    | Showing::MissionText
                    | Showing::MissionInfo
                    | Showing::PlayerInfo
                    | Showing::Movie
            )
        );
        if self.scene != Some(Showing::Flight) || pauses {
            self.thrusting = false;
        }
        self.reconcile();
        for &sound in sounds {
            self.take(sound);
            self.reconcile();
        }
    }

    /// Takes in one of a frame's `sounds`, each at its priority: its cue's
    /// or its fight's own.
    fn take(&mut self, sound: Sound) {
        match sound {
            Sound::Sim(sim) => self.take_sim(sim),
            Sound::Ui(ui) => self.take_ui(ui),
            Sound::Combat(fight) => self.take_fight(fight),
        }
    }

    /// Takes in one of the flight session's sounds: each plays its own
    /// cue ([`SimSound::cue`]) but those that need the core's state. The
    /// thrust drives the engine loop, the klaxon plays unless it is still
    /// playing, the halts halt, and an explosion is heard from its offset.
    fn take_sim(&mut self, sound: SimSound) {
        match sound {
            // Thrust counts only in flight, where the ship flies.
            SimSound::ThrustStarted => {
                self.thrusting = self.scene == Some(Showing::Flight);
            }
            SimSound::ThrustStopped => self.thrusting = false,
            SimSound::Klaxon => self.play_unless_playing(sound.cue()),
            SimSound::Halt(sounds) => self.halt(sounds),
            SimSound::HaltVoice { sound } => self.halt(&[sound]),
            // Heard as a fight's explosion is (`_PlaySoundDistance`).
            SimSound::Explosion {
                sound,
                offset,
                priority,
            } => self.take_fight(CombatSound {
                sound,
                offset,
                priority,
                until_stopped: false,
            }),
            sound => self.play_cue(sound.cue()),
        }
    }

    /// Plays `cue`, if there is one, at the effects volume and its
    /// priority, while sound is on: on the one mission channel when it is
    /// exclusive ([`AudioCommand::PlayExclusive`], at its own rate), else
    /// pitched when it is off its own rate ([`AudioCommand::PlayAtRate`]),
    /// else over the rest ([`AudioCommand::Play`]).
    fn play_cue(&mut self, cue: Option<SoundCue>) {
        let Some(cue) = cue.filter(|_| self.settings.sound) else {
            return;
        };
        let (sound, volume) = (cue.sound, self.settings.effects_volume);
        let priority = Priority::new(cue.priority);
        let rate = PlaybackRate::from_fixed(cue.rate);
        self.audio.run(if cue.exclusive {
            AudioCommand::PlayExclusive {
                sound,
                volume,
                priority,
            }
        } else if rate == PlaybackRate::NATURAL {
            AudioCommand::Play {
                sound,
                volume,
                priority,
            }
        } else {
            AudioCommand::PlayAtRate {
                sound,
                volume,
                priority,
                rate,
            }
        });
    }

    /// Takes in one of the interface's sounds: each plays its own cue
    /// ([`UiSound::cue`]) but those that need the core's state. The keyed
    /// click plays in two steps, a looped port ambient plays unless it is
    /// still playing, an intro picture spends the music first, and the
    /// halts halt.
    fn take_ui(&mut self, sound: UiSound) {
        match sound {
            UiSound::ButtonKeyed => self.click_keyed(),
            UiSound::PortAmbient {
                until_stopped: true,
                ..
            } => self.play_unless_playing(sound.cue()),
            UiSound::PortAmbientHalted(sound) | UiSound::IntroPictureHalted(sound) => {
                self.halt(&[sound]);
            }
            UiSound::IntroPicture(_) => {
                self.spend_music();
                self.play_cue(sound.cue());
            }
            UiSound::MainScreenSoundsHalted => self.halt(&MAIN_SCREEN_SOUNDS),
            UiSound::SwitchedOut => self.switch_out(),
            sound => self.play_cue(sound.cue()),
        }
    }

    /// Plays one of a fight's sounds at its own priority, at the effects
    /// volume times its [`distance_gain`], unless it waits
    /// ([`CombatSound::until_stopped`]) and its `snd ` plays on any voice
    /// (the port's [`Audio::playing`], the original's `ST_IsSoundPlaying`).
    ///
    /// The original tests a looped weapon as it fires; here the frame's
    /// sounds are taken after the tick, in the order they were raised. The
    /// test sees the same voices either way: no frame is drawn between
    /// them.
    fn take_fight(&mut self, fight: CombatSound) {
        if fight.until_stopped && self.audio.playing(fight.sound) {
            return;
        }
        let gain = distance_gain(fight.offset);
        let volume = self.settings.effects_volume.amplitude() * gain;
        let priority = Priority::new(fight.priority);
        self.play_at(Some(fight.sound), Volume::new(volume), priority);
    }

    /// Plays a key's click on a main-menu button, as
    /// `_PlayMainScreenClickSound` @0x177af does: the button-down sound,
    /// then, on the first update after it has ended, the button-up sound
    /// (the original busy-waits on `ST_IsSoundPlaying`, @0x177e0-0x177f2),
    /// each its own cue. A click still waiting is finished first, as the
    /// original finishes one before it reads the next key. With sound off,
    /// nothing plays and nothing waits.
    fn click_keyed(&mut self) {
        if let Some((_, then)) = self.keyed.take() {
            self.play_cue(Some(then));
        }
        if !self.settings.sound {
            return;
        }
        let (down, up) = (UiSound::ButtonDown.cue(), UiSound::ButtonUp.cue());
        self.play_cue(down);
        self.keyed = down.zip(up).map(|(down, up)| (down.sound, up));
    }

    /// Plays `cue`, if there is one, unless its `snd ` is still playing,
    /// at the effects volume and its priority, while sound is on: the port
    /// decides ([`AudioCommand::PlayUnlessPlaying`]).
    fn play_unless_playing(&mut self, cue: Option<SoundCue>) {
        if let Some(cue) = cue
            && self.settings.sound
        {
            self.audio.run(AudioCommand::PlayUnlessPlaying {
                sound: cue.sound,
                volume: self.settings.effects_volume,
                priority: Priority::new(cue.priority),
            });
        }
    }

    /// Halts each of `sounds` that is playing.
    fn halt(&mut self, sounds: &[SoundId]) {
        for &sound in sounds {
            self.audio.run(AudioCommand::Halt { sound });
        }
    }

    /// Plays `sound`, if there is one, at `volume` and `priority`, while
    /// sound is on.
    fn play_at(&mut self, sound: Option<SoundId>, volume: Volume, priority: Priority) {
        if let Some(sound) = sound
            && self.settings.sound
        {
            self.audio.run(AudioCommand::Play {
                sound,
                volume,
                priority,
            });
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

    /// The app was switched out: playing music fades and is spent, as
    /// `_SwitchedOut` pauses it only while `_introMusicMovie` is set
    /// (@0x4f390-0x4f3a0) and `_MusicThread` then stops it for good
    /// (@0x61e4f, @0x61e5f). Music stopped, spent or not yet started is
    /// left as it is.
    fn switch_out(&mut self) {
        if self.music == Music::Playing {
            self.spend_music();
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
            self.keyed = None;
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
/// entry and its ship's christening, the saved pilots' list, a note and a question over the main
/// menu, and the plunder,
/// assignment, rename, comm, haggle and offer dialogs, the mission texts and the
/// mission info screen over flight.
fn is_overlay(showing: Showing) -> bool {
    matches!(
        showing,
        Showing::About
            | Showing::Preferences
            | Showing::NewPilot
            | Showing::Christening
            | Showing::OpenPilot
            | Showing::Note
            | Showing::Question
            | Showing::Plunder
            | Showing::Assignment
            | Showing::CaptureNaming
            | Showing::Comm
            | Showing::EscortDialog
            | Showing::Haggle
            | Showing::Offer
            | Showing::MissionText
            | Showing::MissionInfo
            | Showing::PlayerInfo
            | Showing::Movie
    )
}

/// How long the music at `volume` fades out over: 4 ticks for each
/// Preferences level, at the music level nearest `volume` (32 ticks at the
/// top level, 8). `_StopIntroMusic(1)` @0x61c82 starts at level x 32 and takes
/// 8 off a tick until it reaches 0, so the fade lasts level x 4 ticks.
#[must_use]
pub fn music_fade(volume: Volume) -> Duration {
    Duration::from_secs(4 * u64::from(music_volume_level(volume))) / 60
}

/// Whether music plays on `scene`: the main menu, and the intro's
/// pictures, which follow the main screen's fade out with the music
/// playing on until a picture's sound or the text cancels it; the dialogs
/// over the main menu keep it. The original's intro music starts once a
/// launch (`_InitEVNova` @0x471ec, `_SpawnIntroMusicThread` @0x61a5c),
/// plays once and is not looped (`_MusicThread` @0x61e10 ends at
/// `IsMovieDone` @0x61e3d), and is cancelled for good when a pilot enters
/// the game (`_EnterGameFromMainScreen` @0x19ec6) or About opens
/// (`_DoAboutBox` @0x15fbf). Every other screen, present or to come, is
/// silent: the game's screens (flight, the spaceport, the maps, the system
/// view) never play it, and the overlays are never the scene, as they keep
/// the one below.
///
/// Known divergence: the original reads its Intro Music checkbox
/// (`-[EVPrefsDialogController accept:]` @0xcfeae) only at launch; here
/// turning the setting off stops the music at once, and turning it on
/// before the music is spent restarts it from the top.
fn has_music(scene: Showing) -> bool {
    matches!(scene, Showing::MainMenu | Showing::Intro)
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
    const JUMP: Sound = Sound::Sim(SimSound::JumpBegan { rate: 0x1_0000 });
    const ARRIVED: Sound = Sound::Sim(SimSound::Arrived);

    const LANDED: Sound = Sound::Sim(SimSound::Landed);

    /// The default effects volume, level 4's: a quarter of full.
    const EFFECTS: f32 = 0.25;
    /// The default music volume, level 4's: half of full.
    const MUSIC: f32 = 0.5;

    /// The priority [`drain`] gives every command: the tests of what plays
    /// and how loud leave priorities out, and
    /// `each_event_plays_at_its_priority` tests them.
    const UNCHECKED: Priority = Priority::new(u16::MAX);

    fn play(id: i16, volume: f32) -> AudioCommand {
        AudioCommand::Play {
            sound: SoundId(id),
            volume: Volume::new(volume),
            priority: UNCHECKED,
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

    /// A table with an engine sound, 200.
    fn engine() -> (AudioCore<RecordingAudio>, AudioLog) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let table = SoundTable {
            engine: Some(SoundId(200)),
        };
        (AudioCore::with_table(audio, table), log)
    }

    /// The commands logged since the last call, each sound's priority
    /// left out (as [`UNCHECKED`]).
    fn drain(log: &AudioLog) -> Vec<AudioCommand> {
        drain_prioritised(log)
            .into_iter()
            .map(|command| match command {
                AudioCommand::Play { sound, volume, .. } => AudioCommand::Play {
                    sound,
                    volume,
                    priority: UNCHECKED,
                },
                AudioCommand::PlayExclusive { sound, volume, .. } => AudioCommand::PlayExclusive {
                    sound,
                    volume,
                    priority: UNCHECKED,
                },
                AudioCommand::PlayUnlessPlaying { sound, volume, .. } => {
                    AudioCommand::PlayUnlessPlaying {
                        sound,
                        volume,
                        priority: UNCHECKED,
                    }
                }
                other => other,
            })
            .collect()
    }

    /// The commands logged since the last call, as they were given.
    fn drain_prioritised(log: &AudioLog) -> Vec<AudioCommand> {
        std::mem::take(&mut *log.borrow_mut())
    }

    /// The cue `sound` plays, by its own say ([`SimSound::cue`],
    /// [`UiSound::cue`]).
    fn cue_of(sound: Sound) -> SoundCue {
        match sound {
            Sound::Sim(sim) => sim.cue(),
            Sound::Ui(ui) => ui.cue(),
            Sound::Combat(_) => None,
        }
        .unwrap_or_else(|| panic!("{sound:?} plays a cue"))
    }

    /// What `sound` plays at `volume`, its priority left out (as
    /// [`UNCHECKED`]): its cue's `snd `, at its own rate.
    fn plays(sound: Sound, volume: f32) -> AudioCommand {
        play(cue_of(sound).sound.0, volume)
    }

    /// The `snd ` `sound`'s cue plays.
    fn snd(sound: Sound) -> SoundId {
        cue_of(sound).sound
    }

    #[test]
    fn a_sim_sound_with_no_arm_of_its_own_plays_its_cue() {
        let effects = Volume::new(EFFECTS);
        for sound in [
            SimSound::RedAlert,
            SimSound::PropulsionRepaired,
            SimSound::SelfDestructCountdown,
        ] {
            let cue = cue_of(Sound::Sim(sound));
            let (mut core, log) = original();
            core.update(Some(Showing::ShipBrowser), &[Sound::Sim(sound)]);
            assert_eq!(
                drain_prioritised(&log),
                [AudioCommand::Play {
                    sound: cue.sound,
                    volume: effects,
                    priority: Priority::new(cue.priority),
                }],
                "{sound:?}"
            );
        }
        let exclusive = Sound::Sim(SimSound::Script {
            sound: SoundId(300),
            exclusive: true,
        });
        let pitched = Sound::Sim(SimSound::JumpBegan { rate: 0x8000 });
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[exclusive, pitched, TOOK_OFF]);
        let (script, warp) = (cue_of(exclusive), cue_of(pitched));
        assert_eq!(
            drain_prioritised(&log),
            [
                AudioCommand::PlayExclusive {
                    sound: script.sound,
                    volume: effects,
                    priority: Priority::new(script.priority),
                },
                AudioCommand::PlayAtRate {
                    sound: warp.sound,
                    volume: effects,
                    priority: Priority::new(warp.priority),
                    rate: PlaybackRate::from_fixed(warp.rate),
                },
            ],
            "the take-off has no cue and plays nothing"
        );
        core.set_sound(false);
        drain(&log);
        core.update(
            None,
            &[Sound::Sim(SimSound::RedAlert), exclusive, pitched, TOOK_OFF],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_ui_sound_with_no_arm_of_its_own_plays_its_cue() {
        let ambient = Sound::Ui(UiSound::PortAmbient {
            sound: SoundId(10_032),
            until_stopped: false,
        });
        let sounds = [
            Sound::Ui(UiSound::MapOpened),
            Sound::Ui(UiSound::NoEscorts),
            ambient,
        ];
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &sounds);
        let played = sounds.map(|sound| {
            let cue = cue_of(sound);
            AudioCommand::Play {
                sound: cue.sound,
                volume: Volume::new(EFFECTS),
                priority: Priority::new(cue.priority),
            }
        });
        assert_eq!(drain_prioritised(&log), played);
        core.set_sound(false);
        drain(&log);
        core.update(None, &sounds);
        assert_eq!(drain(&log), [], "sound off");
    }

    /// The priority of each sound `commands` play.
    fn priorities(commands: &[AudioCommand]) -> Vec<Priority> {
        commands
            .iter()
            .filter_map(|command| match *command {
                AudioCommand::Play { priority, .. }
                | AudioCommand::PlayExclusive { priority, .. }
                | AudioCommand::PlayUnlessPlaying { priority, .. } => Some(priority),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_keyed_clicks_halves_play_at_their_cues_priorities() {
        let (mut core, log, audio) = keyed();
        audio.set_playing(snd(DOWN), true);
        core.update(Some(Showing::ShipBrowser), &[KEYED]);
        audio.set_playing(snd(DOWN), false);
        core.update(None, &[]);
        core.update(None, &[KEYED, KEYED]);
        let played = drain_prioritised(&log);
        let at = |sound| Priority::new(cue_of(sound).priority);
        assert_eq!(
            priorities(&played),
            [at(DOWN), at(UP), at(DOWN), at(UP), at(DOWN)],
            "{played:?}"
        );
    }

    // Effects.

    #[test]
    fn each_event_plays_its_sound_at_the_effects_volume() {
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[JUMP, ARRIVED, DOWN, UP]);
        assert_eq!(
            drain(&log),
            [
                plays(JUMP, EFFECTS),
                plays(ARRIVED, EFFECTS),
                plays(DOWN, EFFECTS),
                plays(UP, EFFECTS)
            ]
        );
    }

    #[test]
    fn the_main_screens_slides_play_at_the_effects_volume() {
        const SLIDES: [Sound; 2] = [
            Sound::Ui(UiSound::RowSlideStart),
            Sound::Ui(UiSound::RowSlideEnd),
        ];
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &SLIDES);
        assert_eq!(
            drain(&log),
            [
                start_music(MUSIC),
                plays(SLIDES[0], EFFECTS),
                plays(SLIDES[1], EFFECTS)
            ]
        );
        core.set_sound(false);
        drain(&log);
        core.update(None, &[Sound::Ui(UiSound::RowSlideStart)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    const KEYED: Sound = Sound::Ui(UiSound::ButtonKeyed);

    /// The original table: a core, its log and a handle on its port, to
    /// say what is still playing.
    fn keyed() -> (AudioCore<RecordingAudio>, AudioLog, RecordingAudio) {
        let audio = RecordingAudio::new();
        let log = audio.log();
        let handle = audio.clone();
        (AudioCore::new(audio), log, handle)
    }

    #[test]
    fn a_keyed_button_plays_down_then_up_once_down_has_ended() {
        let (mut core, log, audio) = keyed();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &[KEYED]);
        assert_eq!(drain(&log), [plays(DOWN, 0.5)]);
        audio.set_playing(snd(DOWN), true);
        core.update(None, &[]);
        core.update(None, &[]);
        assert_eq!(drain(&log), [], "600 still playing");
        audio.set_playing(snd(DOWN), false);
        core.update(None, &[JUMP]);
        assert_eq!(
            drain(&log),
            [plays(UP, 0.5), plays(JUMP, 0.5)],
            "601 first, before the frame's events"
        );
        core.update(None, &[]);
        assert_eq!(drain(&log), [], "once");
    }

    #[test]
    fn a_second_keyed_button_finishes_the_first_click_before_its_own() {
        let (mut core, log, audio) = keyed();
        audio.set_playing(snd(DOWN), true);
        core.update(Some(Showing::ShipBrowser), &[KEYED, KEYED]);
        assert_eq!(
            drain(&log),
            [
                plays(DOWN, EFFECTS),
                plays(UP, EFFECTS),
                plays(DOWN, EFFECTS)
            ]
        );
        core.update(None, &[]);
        assert_eq!(drain(&log), []);
        audio.set_playing(snd(DOWN), false);
        core.update(None, &[]);
        assert_eq!(drain(&log), [plays(UP, EFFECTS)]);
    }

    #[test]
    fn a_keyed_button_is_silent_with_sound_off_and_leaves_nothing_pending() {
        let (mut core, log, _) = keyed();
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[KEYED]);
        core.set_sound(true);
        core.update(None, &[]);
        assert_eq!(drain(&log), [], "sound off");

        let (mut core, log, _) = keyed();
        core.update(Some(Showing::ShipBrowser), &[KEYED]);
        core.set_sound(false);
        core.set_sound(true);
        drain(&log);
        core.update(None, &[]);
        assert_eq!(drain(&log), [], "turning sound off forgets 601");
    }

    #[test]
    fn a_keyed_click_is_waited_on_from_down_until_the_update_that_plays_up() {
        let (mut core, log, audio) = keyed();
        assert!(!core.click_waiting(), "no click yet");
        core.update(Some(Showing::MainMenu), &[KEYED]);
        assert!(core.click_waiting(), "600 has just been asked for");
        audio.set_playing(snd(DOWN), true);
        core.update(None, &[]);
        assert!(core.click_waiting(), "600 still playing");
        audio.set_playing(snd(DOWN), false);
        drain(&log);
        core.update(None, &[]);
        assert_eq!(drain(&log), [plays(UP, EFFECTS)]);
        assert!(!core.click_waiting(), "601 has started");
    }

    #[test]
    fn a_keyed_click_is_not_waited_on_with_sound_off() {
        let (mut core, _, audio) = keyed();
        audio.set_playing(snd(DOWN), true);
        core.set_sound(false);
        core.update(Some(Showing::MainMenu), &[KEYED]);
        assert!(!core.click_waiting(), "sound off");
    }

    #[test]
    fn enter_refused_on_the_main_menu_beeps_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_music(false);
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::MainMenu), &[Sound::Ui(UiSound::EnterRefused)]);
        assert_eq!(drain(&log), [plays(Sound::Ui(UiSound::EnterRefused), 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(None, &[Sound::Ui(UiSound::EnterRefused)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_escort_menu_and_commands_beep_their_cues_in_order() {
        const ESCORTS: [Sound; 4] = [
            Sound::Ui(UiSound::EscortMenuToggled),
            Sound::Sim(SimSound::EscortsCommanded),
            Sound::Ui(UiSound::EscortGroupSelected),
            Sound::Ui(UiSound::NoEscorts),
        ];
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &ESCORTS);
        assert_eq!(drain(&log), ESCORTS.map(|sound| plays(sound, EFFECTS)));
    }

    #[test]
    fn a_floating_map_zoom_beeps_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Ui(UiSound::FloatingMapZoomed)],
        );
        assert_eq!(
            drain(&log),
            [plays(Sound::Ui(UiSound::FloatingMapZoomed), 0.5)]
        );
    }

    #[test]
    fn a_tutorial_hint_and_a_told_refusal_beep_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::TutorialHint),
                Sound::Sim(SimSound::Refused),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                plays(Sound::Sim(SimSound::TutorialHint), 0.5),
                plays(Sound::Sim(SimSound::Refused), 0.5)
            ]
        );
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
    fn a_q_message_in_flight_beeps() {
        let (mut core, log) = original();
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::ScriptMessage)],
        );
        assert_eq!(
            drain(&log),
            [plays(Sound::Sim(SimSound::ScriptMessage), EFFECTS)]
        );
        core.set_sound(false);
        drain(&log);
        core.update(None, &[Sound::Sim(SimSound::ScriptMessage)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_scan_beeps_for_a_failed_mission_and_for_smuggling() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::ScanFailedMission),
                Sound::Sim(SimSound::Smuggling),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                plays(Sound::Sim(SimSound::ScanFailedMission), 0.5),
                plays(Sound::Sim(SimSound::Smuggling), 0.5)
            ]
        );
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
    fn a_jump_begun_in_double_time_plays_its_warp_up_at_the_effects_volume() {
        let jump = Sound::Sim(SimSound::JumpBeganInDoubleTime { rate: 0x1_0000 });
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::Flight), &[jump]);
        assert_eq!(drain(&log), [plays(jump, 0.5)]);
    }

    #[test]
    fn a_warp_up_pitched_by_w_plays_at_its_rate_and_one_at_its_own_plainly() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        let pitched = |rate| Sound::Sim(SimSound::JumpBegan { rate });
        let at_rate = |sound: Sound, rate| {
            let cue = cue_of(sound);
            AudioCommand::PlayAtRate {
                sound: cue.sound,
                volume: Volume::new(0.5),
                priority: Priority::new(cue.priority),
                rate: PlaybackRate::from_fixed(rate),
            }
        };
        core.update(Some(Showing::Flight), &[pitched(50_412)]);
        assert_eq!(drain(&log), [at_rate(pitched(50_412), 50_412)]);
        let double = Sound::Sim(SimSound::JumpBeganInDoubleTime { rate: 72_017 });
        core.update(Some(Showing::Flight), &[double]);
        assert_eq!(drain(&log), [at_rate(double, 72_017)]);
        core.update(Some(Showing::Flight), &[pitched(0x1_0000)]);
        assert_eq!(drain(&log), [plays(pitched(0x1_0000), 0.5)], "its own rate");
        core.set_sound(false);
        core.update(Some(Showing::Flight), &[pitched(50_412)]);
        assert!(
            drain(&log)
                .iter()
                .all(|command| !matches!(command, AudioCommand::PlayAtRate { .. })),
            "sound off"
        );
    }

    #[test]
    fn a_jettison_in_flight_beeps_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::CargoJettisoned)],
        );
        assert_eq!(
            drain(&log),
            [plays(Sound::Sim(SimSound::CargoJettisoned), 0.5)]
        );
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::CargoJettisoned)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_hail_quote_beeps_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::HailQuote)],
        );
        assert_eq!(drain(&log), [plays(Sound::Sim(SimSound::HailQuote), 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::HailQuote)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_persons_offer_beeps_on_its_hail_and_its_boarding_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::HailOffer),
                Sound::Sim(SimSound::PersonOfferBoarded),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                plays(Sound::Sim(SimSound::HailOffer), 0.5),
                plays(Sound::Sim(SimSound::PersonOfferBoarded), 0.5)
            ]
        );
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::HailOffer),
                Sound::Sim(SimSound::PersonOfferBoarded),
            ],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_board_plays_its_airlock_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Boarded)]);
        assert_eq!(drain(&log), [plays(Sound::Sim(SimSound::Boarded), 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Boarded)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_red_alert_plays_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::RedAlert)],
        );
        assert_eq!(drain(&log), [plays(Sound::Sim(SimSound::RedAlert), 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::RedAlert)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_cloak_and_the_uncloak_play_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Cloaked)]);
        assert_eq!(drain(&log), [plays(Sound::Sim(SimSound::Cloaked), 0.5)]);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Uncloaked)],
        );
        assert_eq!(drain(&log), [plays(Sound::Sim(SimSound::Uncloaked), 0.5)]);
    }

    #[test]
    fn the_klaxon_plays_unless_still_playing_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Klaxon), Sound::Sim(SimSound::Klaxon)],
        );
        let klaxon = AudioCommand::PlayUnlessPlaying {
            sound: cue_of(Sound::Sim(SimSound::Klaxon)).sound,
            volume: Volume::new(0.5),
            priority: UNCHECKED,
        };
        assert_eq!(drain(&log), [klaxon, klaxon], "the adapter decides");
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Klaxon)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_eject_plays_every_time() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::Ejected), Sound::Sim(SimSound::Ejected)],
        );
        assert_eq!(
            drain(&log),
            [
                plays(Sound::Sim(SimSound::Ejected), 0.5),
                plays(Sound::Sim(SimSound::Ejected), 0.5)
            ],
            "every time"
        );
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &[Sound::Sim(SimSound::Ejected)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_sabotage_outfit_going_off_plays_every_time() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::ShipBrowser),
            &[
                Sound::Sim(SimSound::OnboardExplosion),
                Sound::Sim(SimSound::OnboardExplosion),
            ],
        );
        assert_eq!(
            drain(&log),
            [
                plays(Sound::Sim(SimSound::OnboardExplosion), 0.5),
                plays(Sound::Sim(SimSound::OnboardExplosion), 0.5)
            ],
            "every time"
        );
        core.set_sound(false);
        drain(&log);
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::OnboardExplosion)],
        );
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_self_destructs_beeps_play_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        for sound in [
            SimSound::SelfDestructStarted,
            SimSound::SelfDestructCountdown,
            SimSound::SelfDestructCancelled,
            SimSound::SelfDestructed,
        ] {
            core.update(Some(Showing::ShipBrowser), &[Sound::Sim(sound)]);
            assert_eq!(drain(&log), [plays(Sound::Sim(sound), 0.5)], "{sound:?}");
        }
    }

    #[test]
    fn a_collapsed_jump_beeps_after_its_halts() {
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
                plays(Sound::Sim(SimSound::JumpCollapsed), 0.5),
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

    fn ambient(id: i16, until_stopped: bool) -> Sound {
        Sound::Ui(UiSound::PortAmbient {
            sound: SoundId(id),
            until_stopped,
        })
    }

    #[test]
    fn the_ports_ambient_sound_plays_over_anything_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(
            Some(Showing::Spaceport),
            &[ambient(10_032, false), ambient(10_032, false)],
        );
        assert_eq!(drain(&log), [play(10_032, 0.5), play(10_032, 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[ambient(10_032, false)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_looped_ambient_sound_plays_unless_still_playing() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::Spaceport), &[ambient(10_033, true)]);
        assert_eq!(
            drain_prioritised(&log),
            [AudioCommand::PlayUnlessPlaying {
                sound: SoundId(10_033),
                volume: Volume::new(0.5),
                priority: Priority::new(cue_of(ambient(10_033, true)).priority),
            }],
            "the adapter decides"
        );
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[ambient(10_033, true)]);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_ports_close_halts_its_ambient_sound_even_with_sound_off() {
        let halted = Sound::Ui(UiSound::PortAmbientHalted(SoundId(10_032)));
        let halt = AudioCommand::Halt {
            sound: SoundId(10_032),
        };
        let (mut core, log) = original();
        core.update(Some(Showing::Spaceport), &[halted]);
        assert_eq!(drain(&log), [halt]);
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[halted]);
        assert_eq!(drain(&log), [halt], "sound off halts too");
    }

    const INTRO_PICTURE: Sound = Sound::Ui(UiSound::IntroPicture(SoundId(30_003)));

    #[test]
    fn an_intro_pictures_sound_spends_the_music_then_plays_at_the_effects_volume() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::Intro), &[INTRO_PICTURE]);
        let played = drain_prioritised(&log);
        assert_eq!(
            played,
            [
                fade_out(MUSIC),
                AudioCommand::Play {
                    sound: SoundId(30_003),
                    volume: Volume::new(0.5),
                    priority: Priority::new(cue_of(INTRO_PICTURE).priority),
                }
            ]
        );
        core.update(Some(Showing::MainMenu), &[]);
        core.set_music(false);
        core.set_music(true);
        assert_eq!(drain(&log), [], "spent for good, even on the main menu");
    }

    #[test]
    fn with_sound_off_an_intro_pictures_sound_plays_nothing_but_still_spends_the_music() {
        let (mut core, log) = original();
        core.set_sound(false);
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::Intro), &[INTRO_PICTURE]);
        assert_eq!(drain(&log), [fade_out(MUSIC)]);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [], "spent");
    }

    const SWITCHED_OUT: Sound = Sound::Ui(UiSound::SwitchedOut);

    /// `_SwitchedOut` sets `_introMusicPaused` while the music plays
    /// (@0x4f390-0x4f3a0), and `_MusicThread` (@0x61e4f) leaves for
    /// `_StopIntroMusic(1)` (@0x61e5f): faded, and spent for good.
    #[test]
    fn switching_out_fades_the_playing_music_and_spends_it() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(None, &[SWITCHED_OUT]);
        assert_eq!(drain(&log), [fade_out(MUSIC)], "faded, nothing else");
        core.update(Some(Showing::MainMenu), &[]);
        core.set_music(false);
        core.set_music(true);
        assert_eq!(drain(&log), [], "spent for good");
    }

    /// The original touches the music only while `_introMusicMovie` is
    /// set (@0x4f390-0x4f3a0): stopped music stays free to start.
    #[test]
    fn switching_out_leaves_stopped_music_free() {
        let (mut core, log) = original();
        core.update(None, &[SWITCHED_OUT]);
        assert_eq!(drain(&log), [], "before the main menu");
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(MUSIC)], "it starts");
        core.set_music(false);
        drain(&log);
        core.update(None, &[SWITCHED_OUT]);
        assert_eq!(drain(&log), [], "set off");
        core.set_music(true);
        assert_eq!(drain(&log), [start_music(MUSIC)], "on again, it starts");
    }

    #[test]
    fn switching_out_with_the_music_spent_sends_nothing() {
        let (mut core, log) = original();
        core.update(Some(Showing::Flight), &[]);
        drain(&log);
        core.update(None, &[SWITCHED_OUT]);
        assert_eq!(drain(&log), []);
    }

    #[test]
    fn the_intro_and_main_screen_halts_halt_whatever_the_sound_setting() {
        let picture_halted = Sound::Ui(UiSound::IntroPictureHalted(SoundId(30_003)));
        let main_halted = Sound::Ui(UiSound::MainScreenSoundsHalted);
        let halt = |id| AudioCommand::Halt { sound: SoundId(id) };
        for sound_on in [true, false] {
            let (mut core, log) = original();
            core.set_sound(sound_on);
            drain(&log);
            core.update(Some(Showing::ShipBrowser), &[picture_halted]);
            assert_eq!(drain(&log), [halt(30_003)], "sound {sound_on}");
            core.update(Some(Showing::ShipBrowser), &[main_halted]);
            assert_eq!(
                drain(&log),
                MAIN_SCREEN_SOUNDS.map(|sound| AudioCommand::Halt { sound }),
                "sound {sound_on}"
            );
        }
    }

    #[test]
    fn an_escorts_voice_plays_its_clip_at_its_priority_and_its_halt_halts_it() {
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        let clip = SoundId(1523);
        let voice = Sound::Sim(SimSound::Voice { sound: clip });
        core.update(Some(Showing::ShipBrowser), &[voice]);
        assert_eq!(
            drain_prioritised(&log),
            [AudioCommand::Play {
                sound: clip,
                volume: Volume::new(0.5),
                priority: Priority::new(cue_of(voice).priority),
            }]
        );
        core.update(
            Some(Showing::ShipBrowser),
            &[Sound::Sim(SimSound::HaltVoice { sound: clip })],
        );
        assert_eq!(drain(&log), [AudioCommand::Halt { sound: clip }]);
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
    fn the_course_map_beeps_as_it_opens_and_closes() {
        const MAP: [Sound; 2] = [Sound::Ui(UiSound::MapOpened), Sound::Ui(UiSound::MapClosed)];
        let (mut core, log) = original();
        core.update(None, &MAP);
        assert_eq!(
            drain(&log),
            [plays(MAP[0], EFFECTS), plays(MAP[1], EFFECTS)]
        );
        core.set_effects_volume(Volume::new(0.5));
        core.update(None, &MAP);
        assert_eq!(
            drain(&log),
            [plays(MAP[0], 0.5), plays(MAP[1], 0.5)],
            "at the effects volume"
        );
        core.set_sound(false);
        drain(&log);
        core.update(None, &MAP);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_map_beeps_as_space_centres_it_and_find_finds_or_not() {
        const MAP: [Sound; 3] = [
            Sound::Ui(UiSound::MapCentred),
            Sound::Ui(UiSound::MapSystemFound),
            Sound::Ui(UiSound::MapSystemNotFound),
        ];
        let (mut core, log) = original();
        core.update(None, &MAP);
        assert_eq!(drain(&log), MAP.map(|sound| plays(sound, EFFECTS)));
        core.set_effects_volume(Volume::new(0.5));
        core.update(None, &MAP);
        assert_eq!(
            drain(&log),
            MAP.map(|sound| plays(sound, 0.5)),
            "at the effects volume"
        );
        core.set_sound(false);
        drain(&log);
        core.update(None, &MAP);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn the_player_downed_and_its_failed_mission_each_beep_their_cue() {
        const DOWNED: [Sound; 2] = [
            Sound::Sim(SimSound::ShipDowned),
            Sound::Sim(SimSound::DownedMissionFailed),
        ];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &DOWNED);
        assert_eq!(drain(&log), [plays(DOWNED[0], 0.5), plays(DOWNED[1], 0.5)]);
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &DOWNED);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn a_mission_ship_lost_and_one_boarded_each_beep_their_cue() {
        const MISSION_SHIPS: [Sound; 2] = [
            Sound::Sim(SimSound::MissionShipLost),
            Sound::Sim(SimSound::MissionShipBoarded),
        ];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &MISSION_SHIPS);
        assert_eq!(
            drain(&log),
            [plays(MISSION_SHIPS[0], 0.5), plays(MISSION_SHIPS[1], 0.5)]
        );
        core.set_sound(false);
        drain(&log);
        core.update(Some(Showing::ShipBrowser), &MISSION_SHIPS);
        assert_eq!(drain(&log), [], "sound off");
    }

    #[test]
    fn traffic_control_beeps_its_cue() {
        const ANSWERED: [Sound; 1] = [Sound::Sim(SimSound::TrafficControl)];
        let (mut core, log) = original();
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::ShipBrowser), &ANSWERED);
        assert_eq!(drain(&log), [plays(ANSWERED[0], 0.5)]);
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
                    priority: UNCHECKED,
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
    fn a_landing_plays_only_the_beep() {
        let (mut core, log) = original();
        core.update(Some(Showing::ShipBrowser), &[LANDED]);
        assert_eq!(drain(&log), [plays(LANDED, EFFECTS)]);
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
    fn an_alert_plays_nothing() {
        let (mut core, log) = original();
        core.update(Some(Showing::Spaceport), &[]);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[ALERT]);
        assert_eq!(drain(&log), []);
    }

    // The engine.

    #[test]
    fn the_engine_loops_while_the_ship_thrusts_in_flight() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_loop(200, EFFECTS)]);
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
                start_loop(200, EFFECTS),
                AudioCommand::StopLoop,
                start_loop(200, EFFECTS)
            ]
        );
    }

    #[test]
    fn the_loop_stops_before_the_sounds_that_follow_it() {
        let (mut core, log) = engine();
        core.set_music(false);
        core.update(Some(Showing::Flight), &[THRUST]);
        drain(&log);
        core.update(Some(Showing::Flight), &[COAST, LANDED]);
        assert_eq!(
            drain(&log),
            [AudioCommand::StopLoop, plays(LANDED, EFFECTS)]
        );
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
        Sound::Combat(CombatSound {
            sound: SoundId(id),
            offset,
            priority: 4,
            until_stopped: false,
        })
    }

    /// A looped weapon's `snd ` `id`, fired next to the player, waiting
    /// while it plays.
    fn looped(id: i16) -> Sound {
        Sound::Combat(CombatSound {
            sound: SoundId(id),
            offset: (0, 0),
            priority: 4,
            until_stopped: true,
        })
    }

    #[test]
    fn a_fights_sound_plays_at_its_own_priority() {
        let (mut core, log) = original();
        core.update(Some(Showing::Flight), &[fight(208, (0, 0))]);
        let six = Sound::Combat(CombatSound {
            sound: SoundId(302),
            offset: (0, 0),
            priority: 6,
            until_stopped: false,
        });
        core.update(None, &[six]);
        let played = drain_prioritised(&log);
        assert_eq!(priorities(&played), [Priority::new(4), Priority::new(6)]);
    }

    #[test]
    fn a_looped_sound_is_skipped_while_its_snd_plays_on_any_voice() {
        let (mut core, log, audio) = keyed();
        audio.set_playing(SoundId(205), true);
        core.update(Some(Showing::Flight), &[looped(205)]);
        assert_eq!(drain(&log), [], "still playing, from whichever ship");
        core.update(None, &[fight(205, (0, 0))]);
        assert_eq!(drain(&log), [play(205, EFFECTS)], "one that doesn't wait");
        audio.set_playing(SoundId(205), false);
        core.update(None, &[looped(205)]);
        assert_eq!(drain(&log), [play(205, EFFECTS)], "played out");
    }

    /// A port whose sounds count as playing once they are played, until
    /// told otherwise: as an adapter's voices do.
    #[derive(Default)]
    struct Voices {
        played: Vec<AudioCommand>,
        playing: Vec<SoundId>,
    }

    impl Audio for Voices {
        fn run(&mut self, command: AudioCommand) {
            if let AudioCommand::Play { sound, .. } = command {
                self.playing.push(sound);
            }
            self.played.push(command);
        }

        fn music_ended(&self) -> bool {
            false
        }

        fn playing(&self, sound: SoundId) -> bool {
            self.playing.contains(&sound)
        }
    }

    #[test]
    fn a_second_looped_sound_of_a_snd_in_one_frame_is_skipped() {
        let mut core = AudioCore::new(Voices::default());
        let other_ship = Sound::Combat(CombatSound {
            offset: (300, -400),
            ..match looped(205) {
                Sound::Combat(sound) => sound,
                _ => unreachable!("a fight's"),
            }
        });
        core.update(
            Some(Showing::Flight),
            &[looped(205), other_ship, looped(206)],
        );
        let sounds: Vec<SoundId> = core
            .audio()
            .played
            .iter()
            .filter_map(|command| match *command {
                AudioCommand::Play { sound, .. } => Some(sound),
                _ => None,
            })
            .collect();
        assert_eq!(sounds, [SoundId(205), SoundId(206)]);
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

    #[test]
    fn a_ys_explosion_plays_its_snd_at_its_priority_by_its_distance() {
        // `_CreateExplosion`'s `_PlaySoundDistance(-1, snd, 6, ..)`
        // (@0x3f8f1-0x3f91a), docked as in flight.
        let explosion = |offset| {
            Sound::Sim(SimSound::Explosion {
                sound: SoundId(302),
                offset,
                priority: 6,
            })
        };
        let (mut core, log) = original();
        core.set_music(false);
        core.set_effects_volume(Volume::new(0.5));
        core.update(Some(Showing::Flight), &[explosion((400, 0))]);
        assert_eq!(drain(&log), [play(302, 0.3125)], "by its distance");
        core.update(Some(Showing::Spaceport), &[explosion((0, 0))]);
        assert_eq!(drain(&log), [play(302, 0.5)], "docked");
        let (mut core, log) = original();
        core.update(Some(Showing::Flight), &[fight(208, (0, 0))]);
        core.update(None, &[explosion((0, 0))]);
        let played = drain_prioritised(&log);
        assert_eq!(priorities(&played), [Priority::new(4), Priority::new(6)]);
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
            Showing::Christening,
            Showing::OpenPilot,
            Showing::Note,
            Showing::Question,
        ] {
            let (mut core, log) = original();
            core.update(Some(overlay), &[]);
            assert_eq!(drain(&log), [], "{overlay:?} over nothing");
        }
        let (mut core, log) = original();
        core.update(None, &[]);
        assert_eq!(drain(&log), []);
        core.update(Some(Showing::MainMenu), &[]);
        assert_eq!(drain(&log), [start_music(MUSIC)]);
    }

    #[test]
    fn only_the_main_menu_and_the_intro_have_music() {
        assert!(has_music(Showing::MainMenu));
        assert!(has_music(Showing::Intro));
        for scene in [
            Showing::IntroText,
            Showing::Flight,
            Showing::Spaceport,
            Showing::GalaxyMap,
            Showing::About,
            Showing::Preferences,
            Showing::Movie,
        ] {
            assert!(!has_music(scene), "{scene:?}");
        }
    }

    #[test]
    fn leaving_the_main_menu_fades_the_music_out_for_good() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::Spaceport), &[]);
        assert_eq!(drain(&log), [fade_out(MUSIC)]);
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
        assert_eq!(drain(&log), [fade_out(MUSIC)]);
        core.update(Some(Showing::MainMenu), &[]);
        core.set_music(false);
        core.set_music(true);
        assert_eq!(drain(&log), []);
    }

    /// The original cancels the music on entering the game and opening
    /// About whatever the Music checkbox says (`_CancelMusicThread` from
    /// @0x19ec6 and @0x15fbf), so stopped music is spent too.
    #[test]
    fn music_off_on_the_main_menu_is_still_spent_by_leaving_it() {
        for leave in [Showing::Flight, Showing::About] {
            let (mut core, log) = original();
            core.update(Some(Showing::MainMenu), &[]);
            core.set_music(false);
            assert_eq!(
                drain(&log),
                [start_music(MUSIC), AudioCommand::StopMusic],
                "{leave:?}"
            );
            core.update(Some(leave), &[]);
            assert_eq!(drain(&log), [], "{leave:?}: no fade while stopped");
            core.update(Some(Showing::MainMenu), &[]);
            core.set_music(true);
            assert_eq!(drain(&log), [], "{leave:?}: spent, nothing starts");
        }
    }

    #[test]
    fn the_intro_keeps_the_music_and_starts_it_like_the_main_menu() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        core.update(Some(Showing::Intro), &[]);
        assert_eq!(drain(&log), [], "plays on");
        let (mut core, log) = original();
        core.update(Some(Showing::Intro), &[]);
        assert_eq!(drain(&log), [start_music(MUSIC)], "unspent, it starts");
    }

    #[test]
    fn the_intro_text_and_flight_spend_the_music() {
        for scene in [Showing::IntroText, Showing::Flight] {
            let (mut core, log) = original();
            core.update(Some(Showing::MainMenu), &[]);
            core.update(Some(Showing::Intro), &[]);
            drain(&log);
            core.update(Some(scene), &[]);
            assert_eq!(drain(&log), [fade_out(MUSIC)], "{scene:?}");
            core.update(Some(Showing::MainMenu), &[]);
            assert_eq!(drain(&log), [], "{scene:?}: spent");
        }
    }

    #[test]
    fn a_game_scene_before_the_main_menu_spends_the_music() {
        for scene in [Showing::Flight, Showing::ShipBrowser] {
            let (mut core, log) = original();
            core.update(Some(scene), &[]);
            core.update(Some(Showing::MainMenu), &[]);
            assert_eq!(drain(&log), [], "{scene:?} first");
        }
    }

    #[test]
    fn the_menu_dialogs_keep_the_music() {
        let (mut core, log) = original();
        core.update(Some(Showing::MainMenu), &[]);
        drain(&log);
        for overlay in [
            Showing::Preferences,
            Showing::NewPilot,
            Showing::Christening,
            Showing::OpenPilot,
            Showing::Note,
            Showing::Question,
        ] {
            core.update(Some(overlay), &[]);
            core.update(Some(Showing::MainMenu), &[]);
            assert_eq!(drain(&log), [], "{overlay:?}");
        }
    }

    #[test]
    fn the_fade_lasts_four_ticks_a_preferences_level() {
        use crate::settings::music_level_volume;
        assert_eq!(
            music_fade(music_level_volume(8)),
            Duration::from_secs(32) / 60
        );
        assert_eq!(music_fade(Volume::FULL), Duration::from_secs(32) / 60);
        assert_eq!(
            music_fade(music_level_volume(2)),
            Duration::from_secs(8) / 60
        );
        assert_eq!(
            music_fade(music_level_volume(1)),
            Duration::from_secs(4) / 60
        );
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
        assert_eq!(drain(&log), [start_music(MUSIC)], "from the top");
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
        assert_eq!(drain(&log), [start_music(MUSIC)]);
    }

    #[test]
    fn the_screen_is_taken_in_before_the_sounds() {
        let (mut core, log) = engine();
        core.update(Some(Showing::Flight), &[THRUST]);
        assert_eq!(drain(&log), [start_loop(200, EFFECTS)]);
        core.update(Some(Showing::Spaceport), &[COAST, LANDED]);
        assert_eq!(
            drain(&log),
            [AudioCommand::StopLoop, plays(LANDED, EFFECTS)]
        );
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
        assert_eq!(drain(&log), [start_music(MUSIC)], "on the main menu");
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
        assert_eq!(drain(&log), [start_music(MUSIC)]);
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
                LANDED,
                ambient(10_032, false),
                ambient(10_032, true),
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
        assert_eq!(drain(&log), [start_loop(200, EFFECTS)], "still thrusting");
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
        assert_eq!(drain(&log), [AudioCommand::StopMusic, plays(JUMP, EFFECTS)]);
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
        assert_eq!(drain(&log), [start_music(0.25), plays(JUMP, 0.5)]);
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
        assert_eq!(drain(&log), [plays(ARRIVED, 0.75)]);
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
        assert_eq!(drain(&log), [start_loop(200, EFFECTS)], "a fresh thrust");
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
            Showing::CaptureNaming,
            Showing::Comm,
            Showing::EscortDialog,
            Showing::Haggle,
            Showing::Offer,
            Showing::MissionText,
            Showing::MissionInfo,
            Showing::PlayerInfo,
            Showing::Movie,
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
            [fade_out(0.25), start_loop(200, 0.5), plays(JUMP, 0.5)]
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
        assert_eq!(drain(&log), [start_loop(200, EFFECTS)], "still thrusting");
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
        assert_eq!(drain(&log), [start_music(MUSIC)]);
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
        assert_eq!(drain(&log), [plays(JUMP, 0.5)]);
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
        assert_eq!(*core.audio().log().borrow(), [start_music(MUSIC)]);
    }
}
