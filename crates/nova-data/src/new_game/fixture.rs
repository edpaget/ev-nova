//! The checked-in new-game folders (`tests/fixtures/new-game/`, and the
//! sample's cut, `tests/fixtures/sample/`), for other crates' tests to load
//! without retyping their paths.
//!
//! # `folder()`
//!
//! It holds the factions `guild` and `warden`; the systems `drift`, of
//! jump points alone, `hearth`, the guild's, and `verge`, ungoverned;
//! the stellars `hearth-dock`, `hearth-moon` and `verge-camp`; jump
//! points that cover every case a link can take (two from `hearth` to
//! `verge`, a dead pair between `verge` and `drift`, and `drift`'s
//! one-way back door to `verge`); the civilian hull `hauler`, in guild
//! paint, the crown hull `cutter`, bare, and the wardens' crown hull
//! `warden-picket`, in `warden-colours`, all on a placeholder model; the
//! outfit `dock-pass`, a Dock pass; the legal mod kit `cargo-racks`,
//! fitting freighters, so Hearth Dock's outfitter lists it for the
//! hauler; the start `dock-hand`, in the hauler
//! in `hearth`; and the storyline `dock-work`, with its four missions and
//! two news items:
//!
//! - `dock-job`: offered at `hearth-dock` (`job_offer`), ten tons of hull
//!   plate to `verge-camp`, completed there (`job_done`), each effect
//!   block used; its accept lends the `dock-pass` and its success takes
//!   it back and releases `plate-arrives`, from `verge`;
//! - `side-road-closes`: an event on day 3 that kills both ends of the
//!   side road and releases `side-road-closes`, from `hearth`;
//! - `warden-toll`: offered at `verge-camp` (`toll_offer`) once the side
//!   road is dead and the dock job done, under `all`, `any` and `none`
//!   groups, with no objective;
//! - `clear-the-road`: offered at `verge-camp` (`road_offer`) once the
//!   toll is refused, a `take_point` objective on the wardens' road,
//!   whose blockade is three warden pickets at the road's gate.
//!
//! Their conversations are one file, `dialog/dock-job.ink`, with its
//! compiled `dock-job.ink.json` beside it, which also holds `verge-camp`'s
//! own greeting (`camp_greeting`), the one stellar conversation. The
//! offer reads the world flag `dock_regular` and labels its once-only
//! `route` choice, whose read count adds a line to a later offer and
//! changes the completion's; the greeting reads `hauling_plate`. The
//! `.ink`'s header, and [`dialogs`](super::dialogs)' docs, give the
//! `inklecate` command that compiles it again.
//!
//! Each folder holds its ID table, `nova-ids.json`
//! ([`pins`](super::pins)), pinning every record it maps at the ID it had
//! before tables existed. After adding a record to either folder, run
//! `nova-dump --pin-ids` over it and commit the table with the record.

use std::path::PathBuf;

/// The fixture folder's path.
#[must_use]
pub fn folder() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("new-game")
}

/// The sample fixture folder's path: a three-system cut of game-bible's
/// sample galaxy, holding its "Holding the point" storyline and its first
/// failure, copied in so no test reads game-bible.
///
/// It holds the factions `commonwealth`, `association`, `harrow-band` and
/// `marrow-commons`, the last two lawless; the regions `commonwealth-rim` and
/// `association-border`; the systems `luyten-crossing` (the
/// commonwealth's), `brightwater` (the association's) and `marrow`, with
/// the sample's jump points between them alone: the Luyten road
/// (`luyten-crossing/association-road` to `brightwater/luyten-road`), the
/// Marrow road (`luyten-crossing/marrow-road` to `marrow/luyten-road`) and
/// the far gate (`marrow/far-gate` to `brightwater/marrow-gate`), every
/// one stable, the roads to the sample's other systems left out; the
/// stellars `luyten-exchange`, the commonwealth's, a legal yard, and
/// `marrow-yard`, the commons', a lawless one, each with a bar, trade, an
/// outfitter and fuel (the full sample's Marrow Yard has no outfitter:
/// the cut adds one, so a lawless yard is in reach); the character
/// `sector-organizer`, the missions' giver; the hull `light-freighter`, on
/// a placeholder model, and its variant `harrow-gunship`, in the band's
/// `harrow-raider` livery with a gun mount; the sample's mod kits `armour-plating` (legal) and
/// `gun-mount` (lawless, with a turret bed carrying a `mass-driver`), and
/// the outfit `mass-driver` (its model left out); the start
/// `first-flight`, in it, at `luyten-crossing` (the sample starts at
/// Ninefold, which the cut leaves out); and the storylines
/// `holding-the-point` and `the-first-failure`, with the sample's four
/// missions and one news item:
///
/// - `supply-run`: offered at `luyten-exchange` (`supply_run_offer`),
///   twelve tons to `marrow-yard`, completed there (`supply_run_done`),
///   setting `met_organizer`;
/// - `the-first-failure`: an event from day 60 that makes both ends of the
///   Luyten road unstable and releases `luyten-road-fails`, from
///   `luyten-crossing`;
/// - `the-crossroads-offer`: offered at `marrow-yard` (`crossroads_offer`)
///   once the Luyten road is unstable, its `on_offer` giving the far gate
///   to `harrow-band`, with no objective;
/// - `take-the-point`: offered at `marrow-yard`
///   (`take_the_point_briefing`) once the crossroads is done, a
///   `take_point` objective on the far gate, whose blockade is three
///   Harrow gunships at the gate, completed at `marrow-yard`
///   (`take_the_point_done`), giving the far gate to `marrow-commons`.
///
/// Their conversations are one file, `dialog/holding-the-point.ink`, with
/// its compiled `holding-the-point.ink.json` beside it, whose header gives
/// the `inklecate` command that compiles it again. The crossroads offer
/// reads the world flag `met_organizer` and the read count of
/// `supply_run_done`'s labelled `monists` choice.
///
/// Its `nova-ids.json` pins every record it maps, as the module docs say.
#[must_use]
pub fn sample_folder() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample")
}
