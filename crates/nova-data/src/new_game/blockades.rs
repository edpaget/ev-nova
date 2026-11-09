//! The `take_point` missions' blockades: the holder's guards at the point's
//! gate, as one hidden `mïsn` per mission ([`blockade`]), and each
//! holder's guard `düde` ([`guard`]).
//!
//! A `take_point` mission is met by beating the faction its `held_by` names
//! at its point: every one of its guards destroyed or driven out of the
//! point's system (`nova_sim_story::storylines::lifecycle`). The guards
//! are the stock mission-ship machinery's special ships, so the format's
//! objective becomes a `mïsn` the storylines start while the mission is
//! active and unmet, and end after it (`nova_sim::Session`'s blockades).
//! The stock machinery then sets the guards up at the gate, sends them at
//! the player, brings the survivors back each time the point's system is
//! set up, counts their fates in the pilot's save and checks the goal.
//!
//! # A faction's ships
//!
//! The format has no fleets: a faction's ships are the mapped ships, hulls
//! and variants, whose livery (the nearest in the `base` chain) names it
//! as its `faction` ([`faction_ships`]).
//!
//! # The guard `düde`
//!
//! Each faction that holds some `take_point` objective and has ships gets
//! one `düde`, "<faction name> guards": of its `gövt`, its ships (the 16
//! with the lowest IDs) at even odds in ID order, flown as warships
//! ([`GUARD_AI`], a placeholder), with no booty. The mission's `ShipBehav`
//! sends them at the player whatever their government thinks of the
//! player. They are numbered after [`CIVILIAN_DUDE`], in faction id
//! order, unpinned: nothing saves a `düde` ID.
//!
//! # The blockade `mïsn`
//!
//! One per `take_point` mission whose point has a gate, pinned in the ID
//! table's `blockades` kind by the mission's id ([`super::pins`]): a
//! pilot saved mid-blockade holds its slot by the `mïsn` ID. The store's
//! objective names it ([`MissionObjective::TakePoint`](crate::MissionObjective)).
//! Every field the format has no word for is neutral
//! ([`neutral_mission`]); the rest are:
//!
//! | Field | Value |
//! |---|---|
//! | `ShipSyst` | the point's system's `sÿst` |
//! | `ShipStart` | `-(n+1)`, `n` the point's gate's slot in that system's `nav_def` |
//! | `ShipDude` | the holder's guard `düde` |
//! | `ShipCount` | [`BLOCKADE_SHIPS`], a placeholder, or 0 when the holder has no ships |
//! | `ShipGoal` | [`CHASE_OFF`]: met once every guard is destroyed or chased off |
//! | `ShipBehav` | [`ATTACK_PLAYER`] |
//! | `Flags` | [`HIDDEN`] alone: kept off the mission info screen, never `Flags` 0x0001, which aborts a mission as its ships spawn |
//! | `AvailRandom` | 0: no landing or mission computer offers it |
//! | `TravelStel`, `ReturnStel` | -1: no landing completes it |
//!
//! It is named for its mission's title, "<title> blockade".
//!
//! # Problems
//!
//! Each is reported against the mission's file, and the blockade still
//! maps:
//!
//! - A holder with no ships: `ShipCount` 0. By the stock goal check a
//!   mission whose goal has no ships is met at its first check, so an
//!   unguarded point is taken.
//! - A point whose gate has no place in its system's `nav_def` (a system
//!   of more than 16 gates): `ShipStart` 0, placed at random.

use std::collections::{BTreeMap, BTreeSet};

use nova_galaxy_format::fleet::resolve;
use nova_galaxy_format::id::{Id, PointRef};
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::{Mission as BibleMission, Objective};

use super::gates;
use super::ids::{CIVILIAN_DUDE, IdTable, MISSION_IDS, after_pins};
use super::map::number;
use super::records::{neutral_dude, neutral_mission};
use crate::decode::Entry;
use crate::records::dude::Dude;
use crate::records::mission::Mission;
use crate::registry::AnyRecord;
use crate::wire::flags::Flags16;
use crate::wire::id::{DudeId, GovtId, ShipId, StellarId};

/// How many guards hold a point: 3, the briefings' "gunships", plural. A
/// placeholder until the format gives a blockade's size.
pub const BLOCKADE_SHIPS: i16 = 3;

/// The AI type the guards fly as: 3, a warship. A placeholder until the
/// format gives one.
pub const GUARD_AI: i16 = 3;

/// The `ShipGoal` of a blockade: 6, chase off, met once every guard is
/// destroyed or has left the system.
pub const CHASE_OFF: i16 = 6;

/// The `ShipBehav` of a blockade: 0, the guards attack the player.
pub const ATTACK_PLAYER: i16 = 0;

/// The `mïsn` `Flags` bit that keeps a mission off the mission info
/// screen: 0x0400.
pub const HIDDEN: u16 = 0x0400;

/// How many ship types a `düde` holds.
const DUDE_SLOTS: usize = 16;

/// The `shïp` IDs of the ships in `ships` whose livery, the nearest in
/// its `base` chain, names `faction`, in ID order.
#[must_use]
pub fn faction_ships(galaxy: &Galaxy, ships: &BTreeMap<Id, i16>, faction: &Id) -> Vec<i16> {
    let mut found: Vec<i16> = ships
        .iter()
        .filter(|(ship, _)| {
            resolve(galaxy, ship)
                .ok()
                .and_then(|resolved| galaxy.liveries.get(resolved.livery?))
                .and_then(|livery| livery.data.faction.as_ref())
                == Some(faction)
        })
        .map(|(_, &nova)| nova)
        .collect();
    // `ships` iterates in bible id order, which pins can break.
    found.sort_unstable();
    found
}

/// The `take_point` missions of `galaxy`, in id order, each with its point
/// and its holder.
fn take_points(galaxy: &Galaxy) -> impl Iterator<Item = (&Id, &Doc<BibleMission>, &PointRef, &Id)> {
    galaxy
        .missions
        .iter()
        .filter_map(|(id, doc)| match &doc.data.objective {
            Some(Objective::TakePoint { point, held_by }) => Some((id, doc, point, held_by)),
            _ => None,
        })
}

/// Each guard `düde`'s ID, by its faction: the factions with a `gövt`
/// that hold the objective of a mission with a blockade (in `ids`) and
/// have ships, numbered after [`CIVILIAN_DUDE`] in faction id order.
#[must_use]
pub fn number_guards(galaxy: &Galaxy, ids: &IdTable) -> BTreeMap<Id, i16> {
    let holders: BTreeSet<&Id> = take_points(galaxy)
        .filter(|(mission, _, _, _)| ids.blockades.contains_key(*mission))
        .map(|(_, _, _, held_by)| held_by)
        .filter(|faction| ids.factions.contains_key(*faction))
        .filter(|faction| !faction_ships(galaxy, &ids.ships, faction).is_empty())
        .collect();
    holders
        .into_iter()
        .cloned()
        .zip(CIVILIAN_DUDE + 1..)
        .collect()
}

/// The guard `düde` of `faction`, numbered `nova` (see the module docs).
#[must_use]
pub fn guard(galaxy: &Galaxy, ids: &IdTable, faction: &Id, nova: i16) -> Entry<AnyRecord> {
    let mut ships = faction_ships(galaxy, &ids.ships, faction);
    ships.truncate(DUDE_SLOTS);
    let odds = i16::try_from(100 / ships.len().max(1)).unwrap_or(1).max(1);
    let mut dude = Dude {
        ai_type: GUARD_AI,
        govt: ids.factions.get(faction).copied().map(GovtId),
        ..neutral_dude()
    };
    for (slot, &ship) in ships.iter().enumerate() {
        dude.ship_type[slot] = Some(ShipId(ship));
        dude.probability[slot] = odds;
    }
    let name = galaxy
        .factions
        .get(faction)
        .map_or_else(|| faction.to_string(), |doc| doc.data.name.clone());
    Entry {
        id: nova,
        name: Some(format!("{name} guards")),
        record: AnyRecord::Dude(Box::new(dude)),
    }
}

/// Numbers the `take_point` missions of `galaxy` whose point has one of
/// `gates`, within [`MISSION_IDS`], each `pins` names at its pin and the
/// others after the highest pin ([`super::ids`]); each past the range's
/// end is reported and gets no blockade.
pub fn number_blockades(
    galaxy: &Galaxy,
    gates: &BTreeMap<PointRef, i16>,
    pins: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let guarded: Vec<(&Id, &Doc<BibleMission>)> = take_points(galaxy)
        .filter(|(_, _, point, _)| gates.contains_key(*point))
        .map(|(id, doc, _, _)| (id, doc))
        .collect();
    let mut free = after_pins(MISSION_IDS, pins.values());
    number(guarded, pins, &mut free, "mïsn", problems)
}

/// The blockade `mïsn` of the `take_point` mission `mission`, numbered
/// `nova`, its guards from `guards` (see the module docs); none for a
/// mission that isn't a `take_point` on a gated point.
#[must_use]
pub fn blockade(
    galaxy: &Galaxy,
    ids: &IdTable,
    guards: &BTreeMap<Id, i16>,
    mission: &Id,
    nova: i16,
    problems: &mut Vec<Problem>,
) -> Option<Entry<AnyRecord>> {
    let doc = galaxy.missions.get(mission)?;
    let Some(Objective::TakePoint { point, held_by }) = &doc.data.objective else {
        return None;
    };
    let system = point.system();
    let ship_syst = *ids.systems.get(&system)?;
    let gate = StellarId(*ids.gates.get(point)?);
    let file = &galaxy.systems.get(&system)?.path;
    // The system's own mapping reports its nav_def's problems.
    let nav_def = gates::nav_def(galaxy, ids, &system, file, &mut Vec::new());
    let slot = nav_def.iter().position(|slot| *slot == Some(gate));
    let ship_start = if let Some(slot) = slot {
        -i16::try_from(slot + 1).unwrap_or(i16::MAX)
    } else {
        problems.push(Problem::new(
            &doc.path,
            format!(
                "objective.take_point.point names `{point}`, whose gate has no nav slot; \
                 its guards are placed at random"
            ),
        ));
        0
    };
    let ship_dude = guards.get(held_by).copied().map(DudeId);
    if ship_dude.is_none() && ids.factions.contains_key(held_by) {
        problems.push(Problem::new(
            &doc.path,
            format!(
                "objective.take_point.held_by names `{held_by}`, which has no ships; its point \
                 is unguarded"
            ),
        ));
    }
    let record = Mission {
        ship_count: if ship_dude.is_some() {
            BLOCKADE_SHIPS
        } else {
            0
        },
        ship_syst,
        ship_dude,
        ship_goal: CHASE_OFF,
        ship_behav: ATTACK_PLAYER,
        ship_start,
        flags: Flags16(HIDDEN),
        avail_random: 0,
        travel_stel: -1,
        return_stel: -1,
        time_limit: 0,
        ..neutral_mission()
    };
    Some(Entry {
        id: nova,
        name: Some(format!("{} blockade", doc.data.title)),
        record: AnyRecord::Mission(Box::new(record)),
    })
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::new_game::tests::galaxy;
    use crate::new_game::{NewGame, load};
    use crate::store::MissionObjective;
    use crate::wire::id::MissionId;

    // `galaxy()`: the format's test galaxy. `league` holds `beta/to-alpha`
    // and wears `league-paint`, which the hull `skiff` (128 as
    // `armed-skiff` sorts first, so `skiff` is 129) and, through its base,
    // the variant `armed-skiff` (128) wear. `beta` has no stellars, so its
    // gate is its first nav slot; `alpha` has two stellars before its gate.

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    const HEAD: &str = "---\ntitle: Toll\nstoryline: errands\nact: crossroads\n";

    /// The galaxy with the mission `name`, a `take_point` on `point` held by
    /// `holder`.
    fn taking(source: MemSource, name: &str, point: &str, holder: &str) -> MemSource {
        source.with(
            &format!("missions/{name}.md"),
            &format!(
                "{HEAD}objective: {{take_point: {{point: {point}, held_by: {holder}}}}}\n---\n"
            ),
        )
    }

    fn with_toll() -> MemSource {
        taking(galaxy(), "toll", "beta/to-alpha", "league")
    }

    fn dude(game: &NewGame, nova: i16) -> Option<(String, Dude)> {
        let entry = game.data.get::<Dude>(nova)?.expect("decodes");
        Some((entry.name.expect("named").to_owned(), entry.record.clone()))
    }

    fn mission(game: &NewGame, nova: i16) -> (String, Mission) {
        let entry = game
            .data
            .get::<Mission>(nova)
            .expect("present")
            .expect("decodes");
        (entry.name.expect("named").to_owned(), entry.record.clone())
    }

    #[test]
    fn a_faction_s_ships_are_those_whose_livery_names_it_a_variant_through_its_base() {
        let source = galaxy()
            .with(
                "liveries/other-paint.md",
                "---\nname: Other paint\nfaction: other\n\
                 palette: {primary: \"#111111\", secondary: \"#222222\", trim: \"#333333\"}\n---\n",
            )
            .with(
                "factions/other.md",
                "---\nname: Other\nkind: minor\ncolor: \"#111111\"\n---\n",
            )
            .with(
                "ships/repainted-skiff.md",
                "---\nname: Repainted skiff\nbase: skiff\nlivery: other-paint\n---\n",
            )
            .with(
                "ships/bare.md",
                "---\nname: Bare\nstyle: civilian\nrole: shuttle\nstats: {hull: 5}\n---\n",
            );
        let game = loaded(&source);
        let ship = |name: &str| game.ids.ships[&id(name)];
        assert_eq!(
            faction_ships(&galaxy_of(&source), &game.ids.ships, &id("league")),
            [ship("armed-skiff"), ship("skiff")],
            "the hull, and the variant through its base's livery"
        );
        assert_eq!(
            faction_ships(&galaxy_of(&source), &game.ids.ships, &id("other")),
            [ship("repainted-skiff")],
            "a variant's own livery wins over its base's"
        );
    }

    fn galaxy_of(source: &MemSource) -> Galaxy {
        nova_galaxy_format::load::load(source).0
    }

    #[test]
    fn a_holder_gets_one_guard_dude_of_its_govt_flying_its_ships() {
        let game = loaded(&with_toll());
        assert_eq!(problems(&game), Vec::<String>::new());
        let (name, guards) = dude(&game, CIVILIAN_DUDE + 1).expect("the league's guards");
        assert_eq!(name, "League guards");
        let mut ship_type = [None; 16];
        ship_type[0] = Some(ShipId(128));
        ship_type[1] = Some(ShipId(129));
        let mut probability = [0; 16];
        probability[..2].copy_from_slice(&[50, 50]);
        assert_eq!(
            guards,
            Dude {
                ai_type: GUARD_AI,
                govt: Some(GovtId(game.ids.factions[&id("league")])),
                ship_type,
                probability,
                ..neutral_dude()
            }
        );
        assert_eq!(GUARD_AI, 3);
    }

    #[test]
    fn two_holders_number_their_guards_in_faction_id_order() {
        let source = taking(with_toll(), "raid", "alpha/to-beta", "aaa-band")
            .with(
                "factions/aaa-band.md",
                "---\nname: Band\nkind: minor\ncolor: \"#111111\"\n---\n",
            )
            .with(
                "liveries/band-paint.md",
                "---\nname: Band paint\nfaction: aaa-band\n\
                 palette: {primary: \"#111111\", secondary: \"#222222\", trim: \"#333333\"}\n---\n",
            )
            .with(
                "ships/raider.md",
                "---\nname: Raider\nbase: skiff\nlivery: band-paint\n---\n",
            );
        let game = loaded(&source);
        assert_eq!(problems(&game), Vec::<String>::new());
        let name = |nova| dude(&game, nova).expect("a guard").0;
        assert_eq!(name(CIVILIAN_DUDE + 1), "Band guards");
        assert_eq!(name(CIVILIAN_DUDE + 2), "League guards");
    }

    #[test]
    fn a_faction_holding_no_take_point_gets_no_guards() {
        let game = loaded(&galaxy());
        assert_eq!(dude(&game, CIVILIAN_DUDE + 1), None);
        assert!(game.ids.blockades.is_empty());
    }

    /// The galaxy with `toll` held by `band`, a faction with no ships.
    fn held_by_a_shipless_band() -> MemSource {
        taking(galaxy(), "toll", "beta/to-alpha", "band").with(
            "factions/band.md",
            "---\nname: Band\nkind: minor\ncolor: \"#111111\"\n---\n",
        )
    }

    #[test]
    fn a_holder_with_no_ships_is_a_problem_and_its_blockade_has_no_ships() {
        let game = loaded(&held_by_a_shipless_band());
        assert_eq!(
            problems(&game),
            [
                "missions/toll.md: objective.take_point.held_by names `band`, which has no ships; \
                 its point is unguarded"
            ]
        );
        assert_eq!(dude(&game, CIVILIAN_DUDE + 1), None);
        let (_, blockade) = mission(&game, game.ids.blockades[&id("toll")]);
        assert_eq!((blockade.ship_count, blockade.ship_dude), (0, None));
    }

    #[test]
    fn a_take_point_maps_its_blockade_mission() {
        let game = loaded(&with_toll());
        let nova = game.ids.blockades[&id("toll")];
        assert_eq!(nova, 128, "the first mïsn ID");
        let (name, blockade) = mission(&game, nova);
        assert_eq!(name, "Toll blockade");
        assert_eq!(
            blockade,
            Mission {
                ship_count: BLOCKADE_SHIPS,
                ship_syst: game.ids.systems[&id("beta")],
                ship_dude: Some(DudeId(CIVILIAN_DUDE + 1)),
                ship_goal: CHASE_OFF,
                ship_behav: ATTACK_PLAYER,
                ship_start: -1,
                flags: Flags16(HIDDEN),
                ..neutral_mission()
            },
            "beta's gate is its first nav slot"
        );
        assert_eq!(
            (
                blockade.avail_random,
                blockade.travel_stel,
                blockade.return_stel
            ),
            (0, -1, -1)
        );
        assert_eq!(
            (BLOCKADE_SHIPS, CHASE_OFF, ATTACK_PLAYER, HIDDEN),
            (3, 6, 0, 0x0400)
        );
    }

    #[test]
    fn the_guards_start_at_the_gate_s_own_nav_slot() {
        let game = loaded(&taking(galaxy(), "toll", "alpha/to-beta", "league"));
        let (_, blockade) = mission(&game, game.ids.blockades[&id("toll")]);
        assert_eq!(blockade.ship_syst, game.ids.systems[&id("alpha")]);
        assert_eq!(blockade.ship_start, -3, "after alpha's two stellars");
    }

    #[test]
    fn the_store_s_objective_names_its_blockade() {
        let game = loaded(&with_toll());
        assert_eq!(
            game.data.storylines().missions["toll"].objective,
            Some(MissionObjective::TakePoint {
                point: "beta/to-alpha".to_owned(),
                held_by: "league".to_owned(),
                blockade: Some(MissionId(game.ids.blockades[&id("toll")])),
            })
        );
    }

    #[test]
    fn a_pinned_blockade_keeps_its_id_and_an_unpinned_one_numbers_after_the_pins() {
        let source = taking(with_toll(), "aaa-first", "alpha/to-beta", "league")
            .with(super::super::pins::FILE, r#"{"blockades": {"toll": 140}}"#);
        let game = loaded(&source);
        assert_eq!(game.ids.blockades[&id("toll")], 140, "pinned");
        assert_eq!(
            game.ids.blockades[&id("aaa-first")],
            141,
            "added before it in id order, numbered after the pins"
        );
        let (name, _) = mission(&game, 140);
        assert_eq!(name, "Toll blockade");
    }

    #[test]
    fn blockade_pins_round_trip_through_the_writer() {
        let game = loaded(&with_toll());
        let pinned = super::super::pins::pin(None, &game.ids).expect("pins");
        assert_eq!(pinned.table.blockades, game.ids.blockades);
        let read = super::super::pins::parse(pinned.text.as_bytes()).expect("parses");
        assert_eq!(read.pins.blockades[&id("toll")], 128);
    }

    #[test]
    fn a_gate_with_no_nav_slot_is_a_problem_and_its_guards_are_placed_at_random() {
        // Seventeen gates in `crowded`: the last, by ID, has no nav slot.
        let points = (0..17).fold(String::new(), |mut points, n| {
            let _ = write!(
                points,
                "  - id: g{n:02}\n    position: [0, {n}]\n    to: beta/to-alpha\n    \
                 stability: stable\n"
            );
            points
        });
        let source = taking(
            galaxy().with(
                "systems/crowded.md",
                &format!(
                    "---\nname: Crowded\nregion: rim\nmap: [9, 9]\njump_points:\n{points}---\n"
                ),
            ),
            "toll",
            "crowded/g16",
            "league",
        );
        let game = loaded(&source);
        assert!(
            problems(&game).contains(
                &"missions/toll.md: objective.take_point.point names `crowded/g16`, whose gate \
                  has no nav slot; its guards are placed at random"
                    .to_owned()
            ),
            "{:?}",
            problems(&game)
        );
        let (_, blockade) = mission(&game, game.ids.blockades[&id("toll")]);
        assert_eq!(blockade.ship_start, 0);
    }

    #[test]
    fn a_take_point_on_no_jump_point_gets_no_blockade() {
        let game = loaded(&taking(galaxy(), "toll", "beta/nowhere", "league"));
        assert!(game.ids.blockades.is_empty());
        assert_eq!(
            dude(&game, CIVILIAN_DUDE + 1),
            None,
            "nothing it holds is guarded"
        );
    }
}
