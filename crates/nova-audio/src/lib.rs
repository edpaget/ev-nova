//! EV Nova's sound effects and music.
//!
//! The headless core decides what plays; adapters play it.
//!
//! - [`port`]: the [`Audio`] port, which takes [`AudioCommand`]s (play a
//!   `snd ` once, loop the engine, start or stop the music, change a
//!   [`Volume`]).
//! - [`table`]: the [`SoundTable`] of which `snd ` each event plays, and
//!   the original game's, [`SoundTable::ORIGINAL`].
//! - [`settings`]: the player's [`AudioSettings`]: sound and music on or
//!   off, and their separate volumes.
//! - [`core`](mod@core): the [`AudioCore`], which turns each frame's
//!   sound events (`nova_view::Sound`, from the screens) and the screen
//!   shown (`nova_view::Showing`) into commands on the port.
//! - [`kira`](mod@kira): the adapter, [`KiraAudio`], which plays the
//!   commands through kira: the `snd ` resources, read through the
//!   [`SoundBank`] port and decoded by `nova_data`, and the MP3
//!   soundtrack, streamed.
//!
//! With the `recording` feature (and in this crate's tests), `recording`
//! holds `RecordingAudio`, a port mock that logs every command.

pub mod core;
pub mod kira;
pub mod port;
#[cfg(any(test, feature = "recording"))]
pub mod recording;
pub mod settings;
pub mod table;

pub use crate::core::AudioCore;
pub use crate::kira::{KiraAudio, OpenError, SoundBank};
pub use port::{Audio, AudioCommand, Volume};
pub use settings::AudioSettings;
pub use table::SoundTable;
