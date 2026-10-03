//! Sound decoder: `snd ` resources in, 16-bit PCM out.

mod bytes;
mod error;
mod header;
mod ima4;
mod pcm;
mod snd;

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

pub use error::SoundError;
pub use pcm::{Pcm, SampleRate};
pub use snd::decode_snd;
