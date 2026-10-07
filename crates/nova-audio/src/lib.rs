//! EV Nova's sound effects and music.
//!
//! The headless core decides what plays; adapters play it.
//!
//! - [`port`]: the [`Audio`] port, which takes [`AudioCommand`]s (play a
//!   `snd ` once, loop the engine, start or stop the music, change a
//!   [`Volume`]).
//! - [`table`]: the [`SoundTable`] of which `snd ` each event plays, and
//!   the original game's, [`SoundTable::ORIGINAL`].
//! - [`settings`]: the player's [`AudioSettings`] (sound and music on or
//!   off, and their separate volumes), and the [`SettingsStore`] port
//!   settings are saved through. The keeper that reads and saves them,
//!   with the game's other settings, is `nova`'s.
//! - [`file`]: the settings store's adapter, [`FileSettings`], over one
//!   file on disk.
//! - [`core`](mod@core): the [`AudioCore`], which turns each frame's
//!   sound events (`nova_view::Sound`, from the screens) and the screen
//!   shown (`nova_view::Showing`) into commands on the port.
//! - [`kira`](mod@kira): the adapter, [`KiraAudio`], which plays the
//!   commands through kira: the `snd ` resources, read through the
//!   [`SoundBank`] port and decoded by `nova_data`, and the MP3
//!   soundtrack, streamed.
//!
//! With the `recording` feature (and in this crate's tests), `recording`
//! holds the port mocks: `RecordingAudio`, which logs every command, and
//! `MemorySettings`, a settings store in memory.

pub mod core;
pub mod file;
pub mod kira;
pub mod port;
#[cfg(any(test, feature = "recording"))]
pub mod recording;
pub mod settings;
pub mod table;

pub use crate::core::AudioCore;
pub use crate::file::FileSettings;
pub use crate::kira::{KiraAudio, OpenError, SoundBank};
pub use port::{Audio, AudioCommand, Volume};
pub use settings::{AudioSettings, SettingsStore};
pub use table::SoundTable;
