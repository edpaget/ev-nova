//! The audio port: the playback commands the core gives, in its own terms.

use nova_data::SoundId;

/// A volume: linear amplitude from silent (0) to full (1).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Volume(f32);

impl Volume {
    /// Full volume, as the sound was recorded.
    pub const FULL: Self = Self(1.0);
    /// Silence.
    pub const SILENT: Self = Self(0.0);

    /// The volume `amplitude`, clamped to 0..=1; not a number is silent.
    #[must_use]
    pub fn new(amplitude: f32) -> Self {
        if amplitude.is_nan() {
            Self::SILENT
        } else {
            Self(amplitude.clamp(0.0, 1.0))
        }
    }

    /// The linear amplitude, 0..=1.
    #[must_use]
    pub fn amplitude(self) -> f32 {
        self.0
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self::FULL
    }
}

/// One thing for the audio device to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioCommand {
    /// Plays `sound` once, at `volume`, over whatever else is playing.
    Play {
        /// The `snd `.
        sound: SoundId,
        /// How loud.
        volume: Volume,
    },
    /// Starts the one looping effect, the engine: `sound`, over and over,
    /// at `volume`.
    StartLoop {
        /// The `snd `.
        sound: SoundId,
        /// How loud.
        volume: Volume,
    },
    /// Changes the looping effect's volume.
    SetLoopVolume(Volume),
    /// Stops the looping effect.
    StopLoop,
    /// Starts the music, from the top, looping, at `volume`.
    StartMusic {
        /// How loud.
        volume: Volume,
    },
    /// Changes the music's volume.
    SetMusicVolume(Volume),
    /// Stops the music.
    StopMusic,
    /// Stops every effect playing, the looping one included: sound is off.
    StopEffects,
}

/// Plays what the core asks for.
pub trait Audio {
    /// Carries out `command`. A sound that cannot be played is the
    /// adapter's to report; the core never hears back.
    fn run(&mut self, command: AudioCommand);
}

/// A boxed port is a port.
impl Audio for Box<dyn Audio> {
    fn run(&mut self, command: AudioCommand) {
        (**self).run(command);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::recording::RecordingAudio;

    #[test]
    fn a_volume_is_clamped_to_silent_through_full() {
        assert_eq!(Volume::new(0.25).amplitude(), 0.25);
        assert_eq!(Volume::new(1.5), Volume::FULL);
        assert_eq!(Volume::new(-0.5), Volume::SILENT);
        assert_eq!(Volume::new(f32::NAN), Volume::SILENT);
        assert_eq!(Volume::new(f32::INFINITY), Volume::FULL);
        assert_eq!(Volume::FULL.amplitude(), 1.0);
        assert_eq!(Volume::SILENT.amplitude(), 0.0);
        assert_eq!(Volume::default(), Volume::FULL);
    }

    #[test]
    fn a_boxed_port_forwards_each_command() {
        let recording = RecordingAudio::new();
        let log = recording.log();
        let mut boxed: Box<dyn Audio> = Box::new(recording);
        boxed.run(AudioCommand::StopMusic);
        boxed.run(AudioCommand::StopLoop);
        assert_eq!(
            *log.borrow(),
            [AudioCommand::StopMusic, AudioCommand::StopLoop]
        );
    }
}
