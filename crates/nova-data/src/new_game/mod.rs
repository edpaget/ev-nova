//! The new game's data: a `GAME_DATA` folder in the bible's format
//! (`nova-galaxy-format`), mapped into the [`GameData`] the engine runs on.
//!
//! [`load`] reads any [`Source`], and [`map()`] turns a loaded `Galaxy`
//! into records. The records reach the store through
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
//! | `systems/` | `sÿst` | `x_pos`/`y_pos` from `map`; `govt` from `government`; `nav_def` its stellars, in ID order; no `con` links |
//! | `stellars/` | `spöb` | `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` from `services` and `kind` ([`records::stellar_flags`]); no `hyper_link`s |
//! | a stellar's body | `dësc` | the body, trimmed, at its `spöb`'s ID: the engine reads a stellar's landing text from the `dësc` with its `spöb`'s ID; an empty body has none |
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
//! - A system with more than 16 stellars keeps the first 16 in `nav_def`;
//!   the rest have records but no place in it.
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
//! - `jump_points`: phase 3 maps each to a hypergate `spöb` with one link,
//!   in the system's `nav_def` (sharing its 16 slots with the stellars),
//!   and adds the arrival `Rulebook` key. Gate IDs follow the stellars', so
//!   no stellar is renumbered ([`ids`]).
//! - `ships/`, `mods/`, `outfits/`, `liveries/`: they need the flat-outline
//!   look and the NPC fleets of phase 3, and `play-the-sample`'s mod kits.
//! - `storylines/`, `missions/`, `news/`, `characters/`, `dialog/`: they
//!   belong to `engine-story`.
//! - `assets/`, `budgets/`: tooling (art budgets and provenance). Only the
//!   export media paths are read, for the LFS report
//!   ([`NewGame::lfs_pointers`]).
//! - A stellar's `visual` and a faction's `kind`: Nova has nothing like
//!   them yet (the planet shader and 3D come with `engine-3d`).
//! - A start `chär`: the bible names no start system or ship yet, so New
//!   Pilot has nowhere to fly from. Phase 3 needs one.
//! - Commodity prices: a trade centre's price levels (the upper `Flags`
//!   bits) stay unset, so it trades nothing until the bible prices goods.

pub mod ids;
mod map;
pub mod records;
#[cfg(test)]
mod tests;

use std::path::Path;

use nova_galaxy_format::load::Problem;
use nova_galaxy_format::media::lfs_pointers;
use nova_galaxy_format::source::Source;

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
