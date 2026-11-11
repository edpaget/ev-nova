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
//! | `nova-ids.json` | every record's ID ([`pins`], [`ids`]) | the Nova ID each record keeps, by kind and bible id (a blockade's by its `take_point` mission's id); written only by `nova-dump --pin-ids` |
//! | `factions/` | `gövt` | its `name` as `CommName`; `color` and `ship_color` from `color`, as `00RRGGBB`; one that keeps the law (not `lawless`) `SmugPenalty` [`kits::LAW_SMUGGLE_PENALTY`], `ScanFine` [`kits::LAW_SCAN_FINE`] and `ScanMask` [`kits::LAWLESS_KIT_SCAN`], placeholders, so its scans find a lawless kit; a lawless one none |
//! | a faction's `allies`, `enemies`, `attacks_player`, `player_record` and `penalties` | `gövt` ([`relations`]) | `Class1` its own ID when another faction names it, else no class; `Ally1-4` and `Enemy1-4` the classes of the factions named, in order; `attacks_player` `always` or `never` as `Flags` [`ALWAYS_ATTACKS_PLAYER`](crate::records::govt::flags::ALWAYS_ATTACKS_PLAYER) or [`NEVER_ATTACKS_PLAYER`](crate::records::govt::flags::NEVER_ATTACKS_PLAYER); `player_record` as `InitialRec`; `penalties`' `attack`, `disable` and `destroy` as `ShootPenalty`, `DisabPenalty` and `KillPenalty`, of which `ShootPenalty` changes no record and only forgives a stray hit; a faction that names and is named by none stays neutral |
//! | `systems/` | `sÿst` | `x_pos`/`y_pos` from `map`; `govt` from `government`; `nav_def` its stellars, then its gates, each in ID order; the civilian traffic unless it names its own; no `con` links |
//! | a system's `traffic` | `sÿst` `DudeTypes`, `Prob` and `AvgShips` ([`traffic`]) | each fleet's `düde` and its weight as a whole percentage, each at least 1 and summing to 100 so the engine's rescale keeps them ([`traffic`] gives the rounding), in fleet id order, at most eight; `average_ships`, held within `i16`, as `AvgShips`, or [`fleet::TRAFFIC_SHIPS`] when it names none; no fleet that flies, no traffic |
//! | a system's `hazards` | `sÿst` ([`hazards`]) | `nebula` sets `Murk` [`hazards::NEBULA_MURK`] and `sensor-dead` sets `Interference` [`hazards::SENSOR_DEAD_INTERFERENCE`], both placeholders; a system naming both gets both, and a word named twice counts once; `radiation`, `ion-storm`, `solar-flares` and `gravity-well` are not read |
//! | `stellars/` | `spöb` | `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` from `services` and `kind` ([`records::stellar_flags`]); no `hyper_link`s; a law-keeping owner's `TechLevel` [`kits::YARD_TECH_LEGAL`] and `MinStatus` 0, a lawless owner's or an unowned one's `TechLevel` [`kits::YARD_TECH_LAWLESS`] and `MinStatus` [`records::ANY_RECORD`]; `SpecialTech` the tech numbers of the outfits and hulls sold there, ascending, `-1` for an unused slot ([`yards`]) |
//! | `goods/`, the standard ones | `STR#` 4000, 4001, 4003 and 4004 ([`goods`]) | the first six goods by the ID table, each in its slot: the name, lower-cased, with a `:` (placeholders), and its `price` as text; six entries each, empty for an unused slot; none when no good takes a slot |
//! | `goods/`, the rest | `jünk` ([`goods`]) | named for the good: `BasePrice` its `price`, `LCName` its name lower-cased, `Abbrev` its name, blank `BuyOn` and `SellOn`, no flags and no `ScanMask` |
//! | a stellar's `prices` | `spöb` `Flags`' price nibbles, and each `jünk`'s `SoldAt`/`BoughtAt` ([`goods`]) | a standard good's level as its slot's low, medium or high bit ([`crate::records::stellar::flags::price_bits`]); a special good's `low` stellars as `SoldAt` and `high` ones as `BoughtAt`, in `spöb` ID order; only at a stellar with `trade` |
//! | a stellar's body | `dësc` ([`descriptions`]) | the body, trimmed, at its `spöb`'s ID: the engine reads a stellar's landing text from the `dësc` with its `spöb`'s ID; an empty body has none |
//! | a ship's body | `dësc` ([`descriptions`]) | the body, trimmed, at [`ship_desc_id`](crate::ship_desc_id) of its `shïp`, 13000 + (ID - 128): the shipyard's description; a variant's is its own file's, not inherited through `base`; an empty body has none |
//! | an outfit's body | `dësc` ([`descriptions`]) | the body, trimmed, at [`outfit_desc_id`](crate::outfit_desc_id) of its `oütf`, 3000 + (ID - 128): the outfitter's description; an empty body has none |
//! | a mod kit's body | `dësc` ([`descriptions`]) | the body, trimmed, at [`outfit_desc_id`](crate::outfit_desc_id) of its kit `oütf`, as an outfit's: the outfitter's description; an empty body has none |
//! | `game.md` | `dësc` [`ids::ABOUT_TEXT`] ([`descriptions`]) | its body, trimmed, named for its `name`: the About text; none for an empty body, or with no `game.md` |
//! | a system's `jump_points` | hypergate `spöb` ([`gates`]) | unpinned, numbered after the unpinned stellars; `x_pos`/`y_pos` from `position`; `govt` from `owner`; `flags` `CAN_LAND`; `flags2` the hypergate bit; `hyper_link[0]` the partner point's gate; named for the partner's system; a random exit heading; `MinStatus` [`records::ANY_RECORD`] |
//! | `ships/`, hulls and variants | `shïp` and `shän` ([`fleet`]) | its own `name` as `ShortName`, `LongName` and `CommName`; through the `base` chain: `speed`, `shields`, `hull` and `cargo` as `Speed`, `Shield`, `Armor` and `Holds`; `acceleration`, `turn`, `fuel`, `crew`, `shield_recharge`, `hull_recharge` and `mass` as `Accel`, `Maneuver`, `Fuel`, `Crew`, `ShieldRech`, `ArmorRech` and `Mass`, left out the speed, its `handling` on the turn scale, [`fleet::HULL_FUEL`], one, and 0 (a `turn` anywhere in the chain winning over `handling`); its kits as `DefaultItems`; its chain's hardpoints as `MaxGun` and `MaxTur`; its hardpoints' weapon outfits as stock weapons; the kit `fits` bits of its `role` and its chain hardpoints' mount bits ([`kits::FitBits::mount_bit`]) as `Contribute`; the `shän` `BaseXSize`/`BaseYSize` from `size` and its hardpoints' and kits' as exit points; `price` through the chain as `Cost`; sold at its `sold_at` yards as [`yards`] says, else `TechLevel` 0x7fff and `BuyRandom` 0; never for hire (`HireRandom` 0) |
//! | (placeholder) | `düde` 128 ([`fleet`]) | "Civilian traffic": every civilian hull (no `base`, no `mods`) at even odds, wimpy traders of no government; flown in each system that names no `traffic` |
//! | `fleets/` | `düde` ([`traffic`]) | numbered after the guards, in fleet id order, unpinned; its `name`; its `faction`'s `gövt`; its `role`'s AI type ([`traffic::ai_type`]: `trader` 1, `armed-trader` 2, `warship` 3, `interceptor` 4); the ships it names that are mapped, hulls and variants, in ship id order at their weights held within `i16`, at most sixteen |
//! | a `take_point`'s `held_by` | a guard `düde` ([`blockades`]) | "<faction> guards", numbered after 128 in faction id order, unpinned: the holder's ships (those whose livery names it) at even odds, of its `gövt`, flown as placeholder warships ([`blockades::GUARD_AI`]) |
//! | a `take_point` objective | a hidden blockade `mïsn` ([`blockades`]) | its point's system as `ShipSyst`, its gate's nav slot as `ShipStart`, the holder's guards as `ShipDude`, its `guards` of them held within `i16`, or [`blockades::BLOCKADE_SHIPS`] when it names none, as `ShipCount`, chase off as `ShipGoal`, attacking the player; never offered, never completed by a landing; the store's objective names it |
//! | (placeholder) | `ïntf` 128 ([`records::default_interface`]) | "Default status bar": a radar over the shield, armour and fuel gauges and the nav, weapon, target and cargo readouts, no background picture |
//! | (placeholder) | `STR#` 128, 134, 138, 1100, 2002, 3000, 3001, 3002 and 5003 ([`strings`]) | the engine's own strings, in plain English, at the IDs and indices the engine reads: comm replies, messages, prompts and labels, default names, legal statuses, combat ratings and the stellar type label; each only at an ID no mapped `STR#` holds |
//! | (placeholder) | the dialogs ([`interface::default_dialogs`]) | a `DLOG` and its items, titled "Default spaceport" and so on, for each dialog the view has no built-in layout for: the spaceport and every screen it opens, the text, note and question dialogs, the player info and the preferences; each fits the 640 x 480 view, with no art; the app reads them when the folder has no interface file (`nova::interface`) |
//! | `outfits/` | `oütf` ([`outfits`]) | its `name`, as `ShortName` and lower-cased as `LCName`, and `Cost` from `price`; massless, with no `Max`; its `mount`'s bit as `Require`, with `Flags` `HIDE_UNLESS_REQUIRED`, so it is listed only for a ship with a hardpoint of that mount; sold at the stellars its `sold_at` names, as a `TechLevel` from [`yards::SOLD_TECH_FIRST`] that only their `SpecialTech` slots hold and `BuyRandom` 100 ([`yards`]); with no `sold_at`, never for sale (`TechLevel` 0x7fff, `BuyRandom` 0); a weapon (`damage` above 0, not a `bay`) a `ModType` 1 naming its `wëap`, `Flags` gun or turret; any other doing nothing; `model` goes to the mounts, below |
//! | a weapon outfit | `wëap` ([`outfits`]) | pinned by, and numbered in, outfit id order; `damage`, `range` and `reload` as damage, life and `Reload` at a placeholder speed; unguided, turreted for a turret; its mount's `ExitType`; drawn as the placeholder box, with no sound or explosion |
//! | `mods/` | `oütf` ([`kits`]) | unpinned, numbered after the unpinned outfits; its `name` as `ShortName` and lower-cased as `LCName`; `Cost` from `price`; `Max` 1, `BuyRandom` 100, hidden unless it fits; its effects in the four `ModType` slots, `acceleration`, `fuel`, `shield_recharge` and `hull_recharge` among them; its `fits` as a `Require` bit; its hardpoints' mount bits as `Contribute`; `fitted_at` as `TechLevel` and, lawless, `ScanMask`; its hardpoints' outfits given by `OnPurchase` and taken back by `OnSell` |
//! | a ship's chain hardpoints, a kit's hardpoints and `attention` | the store's fittings ([`GameData::fittings`], [`fleet`], [`kits`]) | each `shïp`'s own exit spots and each kit `oütf`'s, and each kit's attention, which `nova_sim::dock_scan` reads |
//! | a ship's `size`'s `collision_radius` | its collision radius on the store ([`GameData::collision_radius`], [`fleet::collision_radius`]) | through the `base` chain, the nearest `size`'s, in whole pixels at [`fleet::PX_PER_METRE`]; none when it isn't finite or rounds to no positive pixel; the fight reads it under `data_collision` |
//! | a ship's `stats.gate_fuel` | its gate fuel on the store ([`GameData::gate_fuel`], [`fleet::gate_fuel`]) | through the `base` chain, the nearest given, held within `i16`; none when no ship of the chain gives one; a hypergate passage charges it under `gate_fuel` (`nova_sim::gate_fuel`), one jump's worth for a ship with none |
//! | each hardpoint's `position` on its hull's `model`, and its outfit's `model` | its mount on the store ([`GameData::mounts`], [`mounts`]) | a ray cast down at the hardpoint onto the hull's model at its `size`'s length, standing on the highest surface, on the plane for a placeholder; every class's chain hardpoints, and every kit's on every class; the outfit model it shows, lifted to rest on it; the view draws them (`nova_view::flight::mounted_outfits`) |
//! | a weapon outfit's `model` | its muzzle reach on the store ([`GameData::muzzle_reach`], [`mounts`]) | half the model's length at a metre a unit, in whole pixels at [`fleet::PX_PER_METRE`]; none with no model that reads; the fight starts its shots that far ahead of the exit under `mounted_muzzles` |
//! | `starts/` | `chär` ([`fleet`]) | its hull, its system in `system[0]`, its cash, and a placeholder date |
//! | a hull's `model`, `livery` and `size` | a ship stand-in ([`looks`]) | its model seen from above, painted by its livery, in pixels at [`fleet::PX_PER_METRE`]; else its size box in its livery's primary, or bare |
//! | a stellar's `visual` | a stellar stand-in ([`looks`]) | a station's or outpost's model seen from above; else a disc in its look's first colour, its owner's, or bare grey |
//! | a system's `jump_points` | a gate stand-in ([`looks`]) | a grey ring |
//! | a jump point's `stability` and `owner` | its point on the storylines, with its gate ([`missions`], [`PointAt`](crate::PointAt)) | the point's own, which play overlays; a jump between two points works as well as the worse of its ends, and only a stable one is open, which the galaxy's lanes and gates read (`nova_sim_story::storylines::gate_points`) |
//! | `dialog/<file>.ink.json` | its compiled text on the store ([`dialogs`], [`GameData::dialogs`]) | by the Ink file a ref names (`dialog/<file>.ink`); the `.ink` source isn't read |
//! | a stellar's `dialog` | its `spöb`'s conversation on the store ([`dialogs`]) | the file and the knot (or `knot.stitch`) a landed pilot's conversation enters |
//! | `missions/` | a mission on the store ([`missions`], [`GameData::storylines`]) | by its id: its title, `requires`, `offer`, `objective`, `complete` and the four effect blocks, its ids kept as text; a stellar it names as its `spöb`; an outfit an effect grants or removes as its `oütf`; a `take_point`'s blockade as its `mïsn`; its dialogs as places in the compiled texts |
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
//! Every body read (a stellar's, a ship's, an outfit's, a kit's, a news
//! item's and `game.md`'s) is kept as written: Markdown isn't rendered,
//! and its line breaks stay line breaks.
//!
//! The bible has models, not sprites, so nothing names a sprite: each hull,
//! stellar and gate draws as its stand-in, kept on the store
//! ([`GameData::stand_in`]) and read from the media through the same
//! [`Source`]. Each ship's lineage, its chain of bible ids and its role, is
//! kept too ([`GameData::ship_line`]), for the opt-in stock look, which
//! draws a hull in stock sprites from the player's own data instead
//! ([`crate::stock_look`]). Nor is there a star sheet: the stars draw as stand-in dots
//! ([`GameData::star_dots`]).
//!
//! # Problems
//!
//! Nothing panics. Each problem is reported against its file, beside the
//! loader's own (which come through unchanged), and the game goes on with
//! what mapped:
//!
//! - A `government` or `owner` naming no faction maps to no government.
//! - A faction's `allies` or `enemies` naming no faction, the faction
//!   itself, or a fifth faction leave that name out; a name given twice in
//!   one list counts once. A `player_record` or penalty outside `i16` is
//!   held at the nearest ([`relations`]).
//! - A stellar whose `system` names no system is left out.
//! - A map or system position outside `i16` is moved to the nearest.
//! - A system with more than 16 stellars and gates keeps every gate in
//!   `nav_def` and the first stellars that fit beside them; the rest have
//!   records but no place in it, all named in one problem.
//! - A jump point whose `to` names no jump point maps to a gate with no
//!   link, which refuses its entry.
//! - A stellar's price for a good that isn't in `goods/` is left out. So
//!   is a special good's `medium` price, and its price at a ninth stellar
//!   at one level, by `spöb` ID. A good priced above 32767, the most its
//!   record holds, is priced at 32767; that bounds the base price, not
//!   the marked-up price a port charges. A stellar with `prices` and no
//!   `trade` service trades nothing ([`goods`]).
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
//!   it; one past the 60th distinct `fits` set is left out ([`kits`]).
//! - An outfit's `sold_at` stellar that isn't in `stellars/`, or has no
//!   `outfitter` service, doesn't sell it. A stellar selling from more
//!   than eight distinct sets of yards, more than its eight `SpecialTech`
//!   slots, keeps the eight lowest numbers, and the outfits of the rest
//!   aren't sold there; they still are at their other yards. An outfit
//!   past the last tech number isn't sold ([`yards`]).
//! - A ship's `sold_at` stellar that isn't in `stellars/`, or has no
//!   `shipyard` service, doesn't sell it, and the eight-slot limit counts
//!   its sets with the outfits'. A ship with a `sold_at` but no `price`
//!   anywhere in its `base` chain isn't sold ([`yards`]).
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
//! - A `take_point` whose holder has no ships maps a blockade with no
//!   ships, met at once; one whose gate has no nav slot places its guards
//!   at random ([`blockades`]).
//! - A fleet's ship that isn't in `ships/`, or a seventeenth it can fly,
//!   is left out; its faction that isn't in `factions/` gives no
//!   government; a fleet with no ship it can fly, or past the last `düde`
//!   ID, gets no `düde`. A system's `traffic` fleet that isn't in
//!   `fleets/`, or a ninth that flies, is left out ([`traffic`]).
//! - A hull's `livery` that isn't in `liveries/` draws bare; a hull's
//!   `model` or a stellar's `visual` that isn't in `assets/` draws as its
//!   box or its disc; an asset whose model file doesn't decode is reported
//!   once, against the asset ([`looks`]).
//! - A hardpoint over no surface of its hull's model stands on the plane,
//!   reported against the file that adds it, naming the hardpoint and the
//!   model; an outfit's `model` that isn't in `assets/` shows nothing
//!   ([`mounts`]).
//!
//! A reference to a file that failed to load, or to a record left out for
//! its ID, isn't reported again; a stellar in such a system is left out
//! with it.
//!
//! # Not mapped yet
//!
//! - The status bar: the bible authors no interface, so `ïntf` 128 is a
//!   placeholder with no art ([`records::default_interface`]).
//! - The dialogs: the bible authors no interface file either, so each
//!   dialog is a placeholder with no art ([`interface::default_dialogs`]).
//! - The engine's strings: the bible authors no string lists, so the
//!   engine's own are placeholders in plain English ([`strings`]); the
//!   lists only a person, a fleet, a buoy, a government's hail or the
//!   drawn news read are left out.
//! - A good's body: no commodity screen reads a description.
//! - The intro text (the `dësc` the opening shows): an opening needs its
//!   pictures and text authored together, so it is left for the bible's
//!   interface.
//! - The main menu's X text, `dësc` 32766: the format has no word for it.
//! - The name forms that don't derive from the one `name`: an outfit's and
//!   a kit's `LCPlural` (English plurals don't derive), a ship's
//!   `Subtitle`, a faction's `TargetCode` (an abbreviation) and its
//!   `MediumName`, which only reinforcements read, and the new game has
//!   none. Each stays blank.
//! - `regions/` and `harshness` (a region's, and a system's override):
//!   Nova has no region record, and no `sÿst` field carries a harshness.
//!   They stay readable on the loaded `Galaxy`.
//! - The hazards `radiation`, `ion-storm`, `solar-flares` and
//!   `gravity-well`: no `sÿst` field carries them, nor does the engine
//!   have a system-wide shield drain, ion storm or scheduled sweep; the
//!   nearest to a gravity well is a `spöb`'s `Gravity`, which belongs to
//!   one stellar ([`hazards`]). The loader reports nothing for them, and
//!   they stay readable on the loaded `Galaxy`.
//! - A kit's `hidden_cargo`: nothing in the new game is contraband cargo
//!   yet. A kit's `attention` is read only as "any": how much more 2 is
//!   noticed than 1 waits for the bible's law numbers.
//! - A launcher's homing and ammunition, and `bay` hardpoints and outfits
//!   as fighter bays: a launcher fires unguided, and a bay does nothing.
//! - Hiring escorts: a ship's `HireRandom` is 0, never for hire, until the
//!   format has a word for it. The new game greys the bar's Hire Escort
//!   while nothing is for hire, and the Mission BBS while the computer
//!   lists nothing, as its only `mïsn` are hidden blockades
//!   (`nova_sim::RuleKey::EmptyServices`, pinned by `nova::ruleset`).
//! - A livery's `decals` aren't drawn.
//! - Traffic: a system that names no `traffic` flies the placeholder
//!   civilian `düde`, which flies hulls alone ([`fleet`]); a start's date
//!   is a placeholder too. A blockade's guards fly a placeholder AI
//!   ([`blockades::GUARD_AI`]): the format gives guards no role. The
//!   plunderer `gövt` bit that sends a warship to the pirate AI isn't
//!   mapped ([`traffic`]).
//! - A hull's `FreeMass` and `Strength` stay 0: nothing in the new game
//!   uses outfit mass, and the strength feeds odds no phase needs yet.
//! - A faction's `BoardPenalty`, `CrimeTol`, `MaxOdds` and every `Flags`
//!   and `Flags2` bit but the stance toward the player stay neutral: the
//!   format has no word for them. With every hull's `Strength` 0 the
//!   odds filter never drops a target, so `MaxOdds` 0 changes nothing yet
//!   ([`relations`]).
//! - `storylines/`, a mission's `storyline`, `act` and an offer's
//!   `giver`, and `characters/`: nothing in the engine reads them yet.
//! - `budgets/` and an asset's provenance: tooling. An asset's models are
//!   read for the stand-ins ([`looks`]), and the export media paths for the
//!   LFS report ([`NewGame::lfs_pointers`]); its textures aren't drawn.
//! - A stellar's generated look beyond its first colour, and a faction's
//!   `kind`: Nova has nothing like them yet (the planet shader and 3D come
//!   with `engine-3d`).
//! - A good's legality (a `jünk`'s `ScanMask`, contraband), the `jünk`
//!   flags (tribbles, perishable), prices that move with what a port has
//!   heard, and `STR#` 4002 (mission cargo): they wait on the spaceport's
//!   design. A trade centre that prices nothing still opens an empty
//!   exchange: the original's clearing of its trade bit isn't ported
//!   (rdm:phase/ports-and-story/phase-5-load-object-data-junk-and-trade-flag).
//!
//! The IDs are pinned by the folder's table, so adding a record moves no
//! other record and a pilot saved before keeps its place ([`ids`]).

pub mod blockades;
mod descriptions;
pub mod dialogs;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod fleet;
pub mod gates;
pub mod goods;
pub mod hazards;
pub mod ids;
pub mod interface;
pub mod kits;
pub mod looks;
mod map;
pub mod missions;
pub mod mounts;
pub mod outfits;
pub mod pins;
pub mod records;
pub mod relations;
pub mod strings;
#[cfg(test)]
mod tests;
pub mod traffic;
pub mod yards;

use std::io;
use std::path::{Path, PathBuf};

use nova_galaxy_format::load::Galaxy;
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
    /// The galaxy the records were mapped from, as the format's loader read
    /// it: the edge reads the ships', outfits' and stellars' models and
    /// planet looks from it for the 3D layer (`nova_render`'s
    /// `new_game_world`), with [`ids`](Self::ids) to name them.
    pub galaxy: Galaxy,
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
/// ([`looks`]) and the mounts ([`mounts`]), whose models are read from
/// `source` too, each once. It never writes the table.
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
    let mut models = looks::Models::new(source);
    let stand_ins = looks::stand_ins(&galaxy, &ids, &mut models, &mut problems);
    let mounted = mounts::mounted(&galaxy, &ids, &mut models, &mut problems);
    let ship_lines = looks::ship_lines(&galaxy, &ids);
    let collision_radii = fleet::collision_radii(&galaxy, &ids);
    let gate_fuels = fleet::gate_fuels(&galaxy, &ids);
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
            .with_fittings(fittings)
            .with_collision_radii(collision_radii)
            .with_gate_fuels(gate_fuels)
            .with_mounts(mounted.mounts)
            .with_muzzle_reaches(mounted.muzzle_reaches),
        ids,
        problems,
        lfs_pointers,
        galaxy,
    }
}
