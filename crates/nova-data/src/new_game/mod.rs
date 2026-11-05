//! The new game's data: a `GAME_DATA` folder in the bible's format
//! (`nova-galaxy-format`), mapped into the [`GameData`] the engine runs on.
//!
//! [`open`] reads a folder on disk, [`load`] reads any [`Source`], and
//! [`map()`] turns a loaded `Galaxy` into records. The records reach the store through
//! [`GameData::from_decoded`], with no resource bytes behind them, and
//! every catalog port, the view and the renderer read them as they read
//! the stock game's.
//!
//! # What maps to what
//!
//! Each record's resource name is the bible's `name`, and its ID comes
//! from [`ids`]: within a kind, in id order, from 128. A field the bible
//! has no word for is neutral ([`records`]: zero, blank, none, no flags).
//!
//! | Bible | Record | Fields |
//! |---|---|---|
//! | `factions/` | `gövt` | `color` and `ship_color` from `color`, as `00RRGGBB` |
//! | `systems/` | `sÿst` | `x_pos`/`y_pos` from `map`; `govt` from `government`; `nav_def` its stellars, then its gates, each in ID order; the civilian traffic; no `con` links |
//! | `stellars/` | `spöb` | `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` from `services` and `kind` ([`records::stellar_flags`]); no `hyper_link`s |
//! | a stellar's body | `dësc` | the body, trimmed, at its `spöb`'s ID: the engine reads a stellar's landing text from the `dësc` with its `spöb`'s ID; an empty body has none |
//! | a system's `jump_points` | hypergate `spöb` ([`gates`]) | numbered after the last stellar; `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` `CAN_LAND`; `flags2` the hypergate bit; `hyper_link[0]` the partner point's gate; named for the partner's system; a random exit heading |
//! | `ships/` hulls | `shïp` and `shän` ([`fleet`]) | `speed`, `shields`, `hull` and `cargo` as `Speed`, `Shield`, `Armor` and `Holds`; placeholder acceleration, turn rate, fuel and crew; the `shän` only `BaseXSize`/`BaseYSize` from `size` |
//! | (placeholder) | `düde` 128 ([`fleet`]) | "Civilian traffic": every civilian hull at even odds, wimpy traders of no government |
//! | `starts/` | `chär` ([`fleet`]) | its hull, its system in `system[0]`, its cash, and a placeholder date |
//!
//! A jump point is a hypergate with one link, to its partner's gate: the
//! original comes out at the linked `spöb` itself, so the partner is
//! reached even when two points of a system lead to one neighbour, and a
//! one-way point's gate reaches its partner though the partner links
//! elsewhere ([`gates`]). The new game's ruleset lets the traffic use the
//! gates as its only way in and out (`nova::ruleset`).
//!
//! The body is kept as written: Markdown isn't rendered, and its line
//! breaks stay line breaks.
//!
//! # Problems
//!
//! Nothing panics. Each problem is reported against its file, beside the
//! loader's own (which come through unchanged), and the game goes on with
//! what mapped:
//!
//! - A `government` or `owner` naming no faction maps to no government.
//! - A stellar whose `system` names no system is left out.
//! - A map or system position outside `i16` is moved to the nearest.
//! - A system with more than 16 stellars and gates keeps every gate in
//!   `nav_def` and the first stellars that fit beside them; the rest have
//!   records but no place in it, all named in one problem.
//! - A jump point whose `to` names no jump point maps to a gate with no
//!   link, which refuses its entry.
//! - A start whose ship is no hull, or whose system isn't there, is left
//!   out.
//! - A record past its kind's ID range is left out.
//!
//! A reference to a file that failed to load, or to a record left out for
//! its ID, isn't reported again; a stellar in such a system is left out
//! with it.
//!
//! # Not mapped yet
//!
//! - `regions/` and `harshness`, and a system's `hazards`: Nova has no
//!   region record, and hazards belong to a later hazards phase (nebulae,
//!   murk, interference). They stay readable on the loaded `Galaxy`.
//! - A jump point's `stability`: unstable and dead points are travelled as
//!   stable ones until `play-the-sample` phase 1.
//! - Ship variants (a `base` or `mods`) and `mods/`: `play-the-sample`
//!   phase 3. `outfits/` aren't mapped either, so the hulls are unarmed.
//!   `liveries/` and a hull's `model` wait for
//!   `phase-4-draw-flat-outlines`; until then ships draw as the view's
//!   crossed box.
//! - Traffic: the bible authors none, so the civilian `düde` and each
//!   system's traffic are placeholders ([`fleet`]), as are a hull's
//!   acceleration, turn rate, fuel and crew and a start's date.
//! - `storylines/`, `missions/`, `news/`, `characters/`, `dialog/`: they
//!   belong to `engine-story`.
//! - `assets/`, `budgets/`: tooling (art budgets and provenance). Only the
//!   export media paths are read, for the LFS report
//!   ([`NewGame::lfs_pointers`]).
//! - A stellar's `visual` and a faction's `kind`: Nova has nothing like
//!   them yet (the planet shader and 3D come with `engine-3d`).
//! - Commodity prices: a trade centre's price levels (the upper `Flags`
//!   bits) stay unset, so it trades nothing until the bible prices goods.
//!
//! IDs aren't pinned: adding a data file renumbers what follows it, and a
//! pilot saved before can load into another place ([`ids`]).

pub mod fleet;
pub mod gates;
pub mod ids;
mod map;
pub mod records;
#[cfg(test)]
mod tests;

use std::io;
use std::path::{Path, PathBuf};

use nova_galaxy_format::media::lfs_pointers;
use nova_galaxy_format::source::{DirSource, Source};

pub use nova_galaxy_format::load::Problem;

pub use self::ids::IdTable;
pub use self::map::{Mapped, map};
use crate::store::GameData;

/// A new game's data, loaded and mapped.
#[derive(Debug)]
pub struct NewGame {
    /// The records, in the store the engine reads.
    pub data: GameData,
    /// Each record's Nova ID, by its bible id.
    pub ids: IdTable,
    /// What the loader and the mapping found wrong, the loader's first,
    /// each against its file. The data holds what loaded.
    pub problems: Vec<Problem>,
    /// The export media the asset manifest names that are Git LFS
    /// pointers, not the files: the checkout hasn't fetched them. Nothing
    /// reads media yet, so they don't stop anything.
    pub lfs_pointers: Vec<String>,
}

/// Why a new game's folder couldn't be read at all.
#[derive(Debug, thiserror::Error)]
#[error("listing {}: {source}", path.display())]
pub struct ListError {
    /// The folder.
    pub path: PathBuf,
    /// Why it couldn't be listed: it is missing, or not a folder.
    pub source: io::Error,
}

/// Opens the new game's folder `dir` on disk.
///
/// The folder itself must list: the format's directory adapter reads a
/// missing folder as empty, which would start a mistyped `GAME_DATA` over
/// no galaxy. Inside it, a file that fails is a problem, not an error.
pub fn open(dir: &Path) -> Result<NewGame, ListError> {
    std::fs::read_dir(dir).map_err(|source| ListError {
        path: dir.to_path_buf(),
        source,
    })?;
    Ok(load(&DirSource::new(dir), dir))
}

/// Loads the new game's data from `source`, read from the folder `dir`:
/// the format's loader, the LFS check over its media, then the mapping.
pub fn load(source: &impl Source, dir: &Path) -> NewGame {
    let (galaxy, mut problems) = nova_galaxy_format::load::load(source);
    let lfs_pointers = lfs_pointers(&galaxy, source)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let Mapped {
        records,
        ids,
        problems: mapping,
    } = map(&galaxy);
    problems.extend(mapping);
    NewGame {
        data: GameData::from_decoded(dir.to_path_buf(), records),
        ids,
        problems,
        lfs_pointers,
    }
}
