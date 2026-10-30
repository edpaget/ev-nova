//! The checked-in H.264 movie the decoder's tests play
//! (`tests/fixtures/main-bframes.mov`, made as `tests/fixtures/README.md`
//! says), for other crates' tests to write into a folder of their own.

/// Its file name.
pub const NAME: &str = "main-bframes.mov";

/// Its bytes: 10 frames of H.264 Main with B-frames, 10 a second.
pub const MOVIE: &[u8] = include_bytes!("../tests/fixtures/main-bframes.mov");

/// Its natural size, width then height.
pub const SIZE: (u32, u32) = (64, 32);
