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
//! start `dock-hand`, in the hauler in `hearth`; and the storyline
//! `dock-work`, with its four missions and two news items:
//!
//! - `dock-job`: offered at `hearth-dock` (`job_offer`), ten tons of hull
//!   plate to `verge-camp`, completed there (`job_done`), each effect
//!   block used; its success releases `plate-arrives`, from `verge`;
//! - `side-road-closes`: an event on day 3 that kills both ends of the
//!   side road and releases `side-road-closes`, from `hearth`;
//! - `warden-toll`: offered at `verge-camp` (`toll_offer`) once the side
//!   road is dead and the dock job done, under `all`, `any` and `none`
//!   groups, with no objective;
//! - `clear-the-road`: offered at `verge-camp` (`road_offer`) once the
//!   toll is refused, a `take_point` objective on the wardens' road.
//!
//! Their conversations are one file, `dialog/dock-job.ink`, with its
//! compiled `dock-job.ink.json` beside it, which also holds `verge-camp`'s
//! own greeting (`camp_greeting`), the one stellar conversation. The
//! offer reads the world flag `dock_regular` and labels its once-only
//! `route` choice, whose read count adds a line to a later offer and
//! changes the completion's; the greeting reads `hauling_plate`. The
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
