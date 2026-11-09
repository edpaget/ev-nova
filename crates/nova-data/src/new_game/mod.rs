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
//! from [`ids`]: the ID the folder's table pins it at, or, unpinned,
//! within a kind in id order after the highest pin of its range (from 128
//! with none). A field the bible has no word for is neutral ([`records`]:
//! zero, blank, none, no flags).
//!
//! | Bible | Record | Fields |
//! |---|---|---|
//! | `nova-ids.json` | every record's ID ([`pins`], [`ids`]) | the Nova ID each record keeps, by kind and bible id; written only by `nova-dump --pin-ids` |
//! | `factions/` | `gövt` | `color` and `ship_color` from `color`, as `00RRGGBB`; one that keeps the law (not `lawless`) `SmugPenalty` [`kits::LAW_SMUGGLE_PENALTY`], `ScanFine` [`kits::LAW_SCAN_FINE`] and `ScanMask` [`kits::LAWLESS_KIT_SCAN`], placeholders, so its scans find a lawless kit; a lawless one none |
//! | `systems/` | `sÿst` | `x_pos`/`y_pos` from `map`; `govt` from `government`; `nav_def` its stellars, then its gates, each in ID order; the civilian traffic; no `con` links |
//! | `stellars/` | `spöb` | `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` from `services` and `kind` ([`records::stellar_flags`]); no `hyper_link`s; a law-keeping owner's `TechLevel` [`kits::YARD_TECH_LEGAL`] and `MinStatus` 0, a lawless owner's or an unowned one's `TechLevel` [`kits::YARD_TECH_LAWLESS`] and `MinStatus` [`records::ANY_RECORD`] |
//! | a stellar's body | `dësc` | the body, trimmed, at its `spöb`'s ID: the engine reads a stellar's landing text from the `dësc` with its `spöb`'s ID; an empty body has none |
//! | a system's `jump_points` | hypergate `spöb` ([`gates`]) | unpinned, numbered after the unpinned stellars; `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` `CAN_LAND`; `flags2` the hypergate bit; `hyper_link[0]` the partner point's gate; named for the partner's system; a random exit heading; `MinStatus` [`records::ANY_RECORD`] |
//! | `ships/`, hulls and variants | `shïp` and `shän` ([`fleet`]) | through the `base` chain: `speed`, `shields`, `hull` and `cargo` as `Speed`, `Shield`, `Armor` and `Holds`; placeholder acceleration, turn rate, fuel and crew; its kits as `DefaultItems`; its chain's hardpoints as `MaxGun` and `MaxTur`; its hardpoints' weapon outfits as stock weapons; the kit `fits` bits of its `role` as `Contribute`; the `shän` `BaseXSize`/`BaseYSize` from `size` and its hardpoints' and kits' as exit points |
//! | (placeholder) | `düde` 128 ([`fleet`]) | "Civilian traffic": every civilian hull (no `base`, no `mods`) at even odds, wimpy traders of no government |
//! | (placeholder) | `ïntf` 128 ([`records::default_interface`]) | "Default status bar": a radar over the shield, armour and fuel gauges and the nav, weapon, target and cargo readouts, no background picture |
//! | `outfits/` | `oütf` ([`outfits`]) | its `name`, and `Cost` from `price`; massless, with no `Max` and never for sale (`TechLevel` 0x7fff); a weapon (`damage` above 0, not a `bay`) a `ModType` 1 naming its `wëap`, `Flags` gun or turret; any other doing nothing; `model` isn't read |
//! | a weapon outfit | `wëap` ([`outfits`]) | pinned by, and numbered in, outfit id order; `damage`, `range` and `reload` as damage, life and `Reload` at a placeholder speed; unguided, turreted for a turret; its mount's `ExitType`; drawn as the placeholder box, with no sound or explosion |
//! | `mods/` | `oütf` ([`kits`]) | unpinned, numbered after the unpinned outfits; `Cost` from `price`; `Max` 1, `BuyRandom` 100, hidden unless it fits; its effects in the four `ModType` slots; its `fits` as a `Require` bit; `fitted_at` as `TechLevel` and, lawless, `ScanMask`; its hardpoints' outfits given by `OnPurchase` and taken back by `OnSell` |
//! | a ship's chain hardpoints, a kit's hardpoints and `attention` | the store's fittings ([`GameData::fittings`], [`fleet`], [`kits`]) | each `shïp`'s own exit spots and each kit `oütf`'s, and each kit's attention, which `nova_sim::dock_scan` reads |
//! | `starts/` | `chär` ([`fleet`]) | its hull, its system in `system[0]`, its cash, and a placeholder date |
//! | a hull's `model`, `livery` and `size` | a ship stand-in ([`looks`]) | its model seen from above, painted by its livery, in pixels at [`fleet::PX_PER_METRE`]; else its size box in its livery's primary, or bare |
//! | a stellar's `visual` | a stellar stand-in ([`looks`]) | a station's or outpost's model seen from above; else a disc in its look's first colour, its owner's, or bare grey |
//! | a system's `jump_points` | a gate stand-in ([`looks`]) | a grey ring |
//! | a jump point's `stability` and `owner` | its point on the storylines, with its gate ([`missions`], [`PointAt`](crate::PointAt)) | the point's own, which play overlays; a jump between two points works as well as the worse of its ends, and only a stable one is open, which the galaxy's lanes and gates read (`nova_sim_story::storylines::gate_points`) |
//! | `dialog/<file>.ink.json` | its compiled text on the store ([`dialogs`], [`GameData::dialogs`]) | by the Ink file a ref names (`dialog/<file>.ink`); the `.ink` source isn't read |
//! | a stellar's `dialog` | its `spöb`'s conversation on the store ([`dialogs`]) | the file and the knot (or `knot.stitch`) a landed pilot's conversation enters |
//! | `missions/` | a mission on the store ([`missions`], [`GameData::storylines`]) | by its id: its title, `requires`, `offer`, `objective`, `complete` and the four effect blocks, its ids kept as text; a stellar it names as its `spöb`; an outfit an effect grants or removes as its `oütf`; its dialogs as places in the compiled texts |
//! | `news/` | a news item on the store ([`missions`]) | by its id: its headline, its body trimmed, and its `from` system's `sÿst` |
//! | a jump point's `owner` and `stability` | its static state on the store ([`missions`]) | by `system/point`: its gate's `spöb`, its owner's id and its stability, which missions' effects overlay in play |
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
//! The bible has models, not sprites, so nothing names a sprite: each hull,
//! stellar and gate draws as its stand-in, kept on the store
//! ([`GameData::stand_in`]) and read from the media through the same
//! [`Source`]. Nor is there a star sheet: the stars draw as stand-in dots
//! ([`GameData::star_dots`]).
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
//! - A start whose ship isn't there, or whose system isn't there, is left
//!   out.
//! - A ship whose `base` chain loops or names a ship that isn't there is
//!   left out, and a start in it with it. One with more than eight
//!   default items or stock weapons keeps the first eight; one with more
//!   than four hardpoints of a kind fires from the first four's places. A
//!   mod kit or a hardpoint outfit it names itself that isn't there is
//!   left out.
//! - A kit needing more than four `ModType` slots keeps the first four;
//!   one whose hardpoint names an outfit that isn't there doesn't give
//!   it; one past the 64th distinct `fits` set is left out ([`kits`]).
//! - A weapon past the last `wëap` ID does nothing ([`outfits`]).
//! - A record past its kind's ID range is left out.
//! - The ID table, `nova-ids.json` ([`pins`]): one that doesn't parse or
//!   names an unknown kind is ignored whole, and every record is unpinned;
//!   an entry with a malformed key, an ID outside its range, or an ID an
//!   earlier entry of its range holds is ignored; and a table that leaves
//!   out a mapped record says how many, names the first and says to run
//!   `nova-dump --pin-ids`. A folder with no table is no problem.
//! - A stellar's `dialog` whose compiled `dialog/<file>.ink.json` is
//!   missing maps no conversation. One whose knot the story lacks does the
//!   same once [`NewGame::check_dialogs`] has asked the dialog port
//!   ([`dialogs::DialogCheck`]), which the game runs as it opens the
//!   folder.
//! - A mission naming a stellar, mission, news item, jump point, faction
//!   or outfit that isn't there is left out, with a problem for each such
//!   name. So
//!   is one whose `offer` or `complete` dialog has no compiled file, or,
//!   once [`NewGame::check_dialogs`] has run, names a knot its story
//!   lacks. One whose `requires` names a mission left out is left out
//!   too, with a problem naming it. A news item whose `from` isn't a
//!   system is left out.
//! - A hull's `livery` that isn't in `liveries/` draws bare; a hull's
//!   `model` or a stellar's `visual` that isn't in `assets/` draws as its
//!   box or its disc; an asset whose model file doesn't decode is reported
//!   once, against the asset ([`looks`]).
//!
//! A reference to a file that failed to load, or to a record left out for
//! its ID, isn't reported again; a stellar in such a system is left out
//! with it.
//!
//! # Not mapped yet
//!
//! - The status bar: the bible authors no interface, so `ïntf` 128 is a
//!   placeholder with no art ([`records::default_interface`]).
//! - `regions/` and `harshness`, and a system's `hazards`: Nova has no
//!   region record, and hazards belong to a later hazards phase (nebulae,
//!   murk, interference). They stay readable on the loaded `Galaxy`.
//! - A kit's `hidden_cargo`: nothing in the new game is contraband cargo
//!   yet. A kit's `attention` is read only as "any": how much more 2 is
//!   noticed than 1 waits for the bible's law numbers.
//! - A launcher's homing and ammunition, and `bay` hardpoints and outfits
//!   as fighter bays: a launcher fires unguided, and a bay does nothing.
//! - Loose outfits for sale: no yard sells one until the bible says which
//!   yards sell which weapons.
//! - Variants in traffic: the civilian traffic flies hulls alone.
//! - A livery's `decals` aren't drawn, nor are mounted outfits' models.
//! - Traffic: the bible authors none, so the civilian `düde` and each
//!   system's traffic are placeholders ([`fleet`]), as are a hull's
//!   acceleration, turn rate, fuel and crew and a start's date.
//! - `storylines/`, a mission's `storyline`, `act` and an offer's
//!   `giver`, and `characters/`: nothing in the engine reads them yet.
//! - `budgets/` and an asset's provenance: tooling. An asset's models are
//!   read for the stand-ins ([`looks`]), and the export media paths for the
//!   LFS report ([`NewGame::lfs_pointers`]); its textures aren't drawn.
//! - A stellar's generated look beyond its first colour, and a faction's
//!   `kind`: Nova has nothing like them yet (the planet shader and 3D come
//!   with `engine-3d`).
//! - Commodity prices: a trade centre's price levels (the upper `Flags`
//!   bits) stay unset, so it trades nothing until the bible prices goods.
//!
//! The IDs are pinned by the folder's table, so adding a record moves no
//! other record and a pilot saved before keeps its place ([`ids`]).

pub mod dialogs;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod fleet;
pub mod gates;
pub mod ids;
pub mod kits;
pub mod looks;
mod map;
pub mod missions;
pub mod outfits;
pub mod pins;
pub mod records;
#[cfg(test)]
mod tests;

use std::io;
use std::path::{Path, PathBuf};

use nova_galaxy_format::media::lfs_pointers;
use nova_galaxy_format::source::{DirSource, Source};

pub use nova_galaxy_format::load::Problem;

pub use self::ids::IdTable;
pub use self::map::{Mapped, map, map_pinned};
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
    /// pointers, not the files: the checkout hasn't fetched them. A model
    /// that is a pointer draws as its hull's size box or its stellar's
    /// disc, so they don't stop anything, and they aren't reported again
    /// among the problems.
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
/// the format's loader, the ID table ([`pins`]), the LFS check over its
/// media, the mapping under the table's pins, then the stand-ins
/// ([`looks`]), whose models are read from `source` too. It never writes
/// the table.
pub fn load(source: &impl Source, dir: &Path) -> NewGame {
    let (galaxy, mut problems) = nova_galaxy_format::load::load(source);
    let pins = pins::read(source, &mut problems);
    let lfs_pointers = lfs_pointers(&galaxy, source)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let Mapped {
        records,
        ids,
        problems: mapping,
        fittings,
    } = map_pinned(&galaxy, pins.as_ref().unwrap_or(&IdTable::default()));
    problems.extend(mapping);
    problems.extend(pins.and_then(|pins| pins::unpinned(&pins, &ids, dir)));
    let stand_ins = looks::stand_ins(&galaxy, &ids, source, &mut problems);
    let ship_lines = looks::ship_lines(&galaxy, &ids);
    let compiled = dialogs::compiled(&galaxy, source, &mut problems);
    let dialogs = dialogs::read(&galaxy, &ids, &compiled, &mut problems);
    let storylines = missions::map(&galaxy, &ids, &compiled, &mut problems);
    NewGame {
        data: GameData::from_decoded(dir.to_path_buf(), records)
            .with_stand_ins(stand_ins)
            .with_ship_lines(ship_lines)
            .with_star_dots(looks::STAR_DOTS.to_vec())
            .with_dialogs(dialogs)
            .with_storylines(storylines)
            .with_fittings(fittings),
        ids,
        problems,
        lfs_pointers,
    }
}
