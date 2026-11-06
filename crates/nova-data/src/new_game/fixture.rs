//! The checked-in new-game folder (`tests/fixtures/new-game/`), for other
//! crates' tests to load without retyping its path.
//!
//! It holds the factions `guild` and `warden`; the systems `drift`, of
//! jump points alone, `hearth`, the guild's, and `verge`, ungoverned;
//! the stellars `hearth-dock`, `hearth-moon` and `verge-camp`; jump
//! points that cover every case a link can take (two from `hearth` to
//! `verge`, a dead pair between `verge` and `drift`, and `drift`'s
//! one-way back door to `verge`); the civilian hull `hauler` and the
//! crown hull `cutter`, with a livery and a placeholder model; the
//! start `dock-hand`, in the hauler in `hearth`; and one conversation,
//! `dialog/dock-job.ink`, an offer that `hearth-dock` names
//! (`dialog/dock-job.ink#job_offer`), with its compiled
//! `dock-job.ink.json` beside it. The other two stellars name none. The
//! `.ink`'s header, and [`dialogs`](super::dialogs)' docs, give the
//! `inklecate` command that compiles it again.

use std::path::PathBuf;

/// The fixture folder's path.
#[must_use]
pub fn folder() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("new-game")
}
