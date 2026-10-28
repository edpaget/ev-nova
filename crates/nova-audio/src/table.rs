//! The engine loop's `snd `: the one sound that is the audio side's own
//! state, not an event's.
//!
//! Every event says what it plays itself: a flight session's sound
//! ([`SimSound::cue`](nova_view::sound::SimSound::cue)), the interface's
//! ([`UiSound::cue`](nova_view::sound::UiSound::cue)) and a fight's
//! ([`CombatSound`](nova_view::sound::CombatSound)). The engine loop has no
//! event of its own: the core starts and stops it as the thrust says.
//! [`SoundTable::ORIGINAL`] is the original game's, pinned by a test.

use nova_data::SoundId;

/// The engine loop's `snd `; `None` is silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundTable {
    /// Loops while the ship thrusts.
    pub engine: Option<SoundId>,
}

impl SoundTable {
    /// The original game's sounds.
    ///
    /// - `engine`: none. No stock `snd ` is an engine sound, and the
    ///   community *EV Nova Resource ID Guide* (github.com/andrews05/evstuff,
    ///   `guides/resourceidguide.html`, "snd IDs"), which lists the engine's
    ///   fixed sounds, reserves no ID for one: the original thrusts in
    ///   silence.
    pub const ORIGINAL: Self = Self { engine: None };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_original_table_has_no_engine_sound() {
        let table = SoundTable::ORIGINAL;
        assert_eq!(table.engine, None, "the original thrusts in silence");
    }
}
