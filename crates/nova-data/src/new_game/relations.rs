//! A faction's relations, mapped to the `gövt` fields the engine's legal
//! and combat code reads, so the new game adds no sim rule:
//!
//! - **Classes.** The engine reads relations by class, not by government:
//!   two governments are allies or enemies when a `Class1-4` of one is
//!   among the other's `Ally1-4` or `Enemy1-4`, either way round
//!   (`_GovtAllies` @0x4e3d, `_GovtEnemies` @0x4f22). So a faction that
//!   another names as an ally or an enemy gets `Class1` = its own `gövt`
//!   ID, unique and stable under the pins, and a faction nobody names
//!   keeps every class -1: with no relations it is the record it was
//!   before relations were read.
//! - **`allies` and `enemies`** fill `Ally1-4` and `Enemy1-4` with the
//!   classes of the factions named, in order, -1 after them. One side
//!   naming the other is enough, as the engine reads either way round. A
//!   name that is no faction, the faction itself, and a fifth name are
//!   each left out with a [`Problem`] against the faction's file; a name
//!   given twice in one list counts once. A faction named both an ally
//!   and an enemy is written as given; the engine makes the enemy win.
//! - **`attacks_player`** sets `Flags` [`ALWAYS_ATTACKS_PLAYER`] (`always`)
//!   or [`NEVER_ATTACKS_PLAYER`] (`never`). The engine's warships of an
//!   always-attacking government attack the player on sight whatever its
//!   record (`_WarshipAI` @0x8b76c-0x8b80f); without it a government
//!   that governs no system never finds the player hostile through its
//!   record, which is read in a system against the system's government.
//! - **`player_record`** is `InitialRec`: the record in each system the
//!   faction governs that a new pilot starts with, and the one the escape
//!   pod's rescue puts back.
//! - **`penalties`** are `ShootPenalty` (`attack`), `DisabPenalty`
//!   (`disable`) and `KillPenalty` (`destroy`). A crime against a ship of
//!   the faction lowers the player's record with it by the penalty and
//!   raises the record with each government not allied with it by half
//!   (`_RecursivePenaltySlap` @0x980a). `ShootPenalty` changes no record:
//!   the engine reads it only to forgive a stray hit (`_DamageShip`
//!   @0x3b80a). A mission's ship, a blockade's guards among them, convicts
//!   the player of nothing (@0x9c10).
//!
//! `player_record` and each penalty are held within `i16`, with a
//! [`Problem`] when one is outside it.

use std::collections::BTreeMap;

use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::model::PlayerStance;

use crate::records::govt::flags::{ALWAYS_ATTACKS_PLAYER, NEVER_ATTACKS_PLAYER};

/// The most allies, or enemies, a `gövt` holds: `Ally1-4`, `Enemy1-4`.
pub const RELATION_SLOTS: usize = 4;

/// What a faction's relations set on its `gövt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relations {
    /// `Class1-4`.
    pub class: [i16; 4],
    /// `Ally1-4`.
    pub ally: [i16; 4],
    /// `Enemy1-4`.
    pub enemy: [i16; 4],
    /// The `Flags` bits it sets.
    pub flags: u16,
    /// `InitialRec`.
    pub initial_rec: i16,
    /// `ShootPenalty`.
    pub shoot_penalty: i16,
    /// `DisabPenalty`.
    pub disab_penalty: i16,
    /// `KillPenalty`.
    pub kill_penalty: i16,
}

impl Relations {
    /// No relations: every class, ally and enemy slot -1, no flags, no
    /// penalties, a starting record of 0, as the neutral `gövt` has.
    pub const NONE: Self = Self {
        class: [-1; 4],
        ally: [-1; 4],
        enemy: [-1; 4],
        flags: 0,
        initial_rec: 0,
        shoot_penalty: 0,
        disab_penalty: 0,
        kill_penalty: 0,
    };
}

/// Each faction's relations in `galaxy`, by its bible id, with its Nova
/// ID from `factions`; what couldn't be read as written goes to
/// `problems`, against the faction's file.
pub fn relations(
    galaxy: &Galaxy,
    factions: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, Relations> {
    let mut lists = BTreeMap::new();
    for id in factions.keys() {
        let doc = &galaxy.factions[id];
        let named = |list: &[Id], what: &str, problems: &mut Vec<Problem>| {
            kept(id, list, what, factions, &doc.path, problems)
        };
        let allies = named(&doc.data.allies, "allies", problems);
        let enemies = named(&doc.data.enemies, "enemies", problems);
        lists.insert(id.clone(), (allies, enemies));
    }
    let named_by_any: Vec<&Id> = lists
        .values()
        .flat_map(|(allies, enemies)| allies.iter().chain(enemies))
        .collect();
    lists
        .iter()
        .map(|(id, (allies, enemies))| {
            let doc = &galaxy.factions[id];
            let mut relations = Relations::NONE;
            if named_by_any.contains(&id) {
                relations.class[0] = factions[id];
            }
            relations.ally = slots(allies, factions);
            relations.enemy = slots(enemies, factions);
            relations.flags = stance_flags(doc.data.attacks_player);
            let mut held = |value: i64, what: &str| held_i16(value, what, &doc.path, problems);
            relations.initial_rec = held(doc.data.player_record.into(), "player_record");
            let penalties = doc.data.penalties;
            relations.shoot_penalty = held(penalties.attack.into(), "penalties.attack");
            relations.disab_penalty = held(penalties.disable.into(), "penalties.disable");
            relations.kill_penalty = held(penalties.destroy.into(), "penalties.destroy");
            (id.clone(), relations)
        })
        .collect()
}

/// The factions `list` names that are kept: each a faction other than
/// `of`, once, at most [`RELATION_SLOTS`]; each left out is a problem
/// against `path`, but a repeat.
fn kept(
    of: &Id,
    list: &[Id],
    what: &str,
    factions: &BTreeMap<Id, i16>,
    path: &str,
    problems: &mut Vec<Problem>,
) -> Vec<Id> {
    let mut kept: Vec<Id> = Vec::new();
    for named in list {
        if kept.contains(named) {
            continue;
        }
        let why = if named == of {
            Some("itself")
        } else if !factions.contains_key(named) {
            Some("no faction")
        } else if kept.len() == RELATION_SLOTS {
            Some("past the fourth")
        } else {
            None
        };
        match why {
            Some(why) => problems.push(Problem::new(
                path,
                format!("its {what} name `{named}`, {why}; it is left out"),
            )),
            None => kept.push(named.clone()),
        }
    }
    kept
}

/// The classes of `named`, each its faction's Nova ID, then -1.
fn slots(named: &[Id], factions: &BTreeMap<Id, i16>) -> [i16; 4] {
    let mut slots = [-1; 4];
    for (slot, id) in slots.iter_mut().zip(named) {
        *slot = factions[id];
    }
    slots
}

/// The `Flags` bits `stance` sets.
fn stance_flags(stance: Option<PlayerStance>) -> u16 {
    match stance {
        None => 0,
        Some(PlayerStance::Always) => ALWAYS_ATTACKS_PLAYER,
        Some(PlayerStance::Never) => NEVER_ATTACKS_PLAYER,
    }
}

/// `value`, held within `i16`, with a problem against `path` naming
/// `what` when it is outside.
fn held_i16(value: i64, what: &str, path: &str, problems: &mut Vec<Problem>) -> i16 {
    let held = i16::try_from(value).unwrap_or(if value < 0 { i16::MIN } else { i16::MAX });
    if i64::from(held) != value {
        problems.push(Problem::new(
            path,
            format!(
                "its {what} {value} is outside {} to {}; it is held at {held}",
                i16::MIN,
                i16::MAX
            ),
        ));
    }
    held
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::new_game::records::neutral_govt;
    use crate::new_game::tests::galaxy;
    use crate::new_game::{NewGame, load};
    use crate::records::govt::Govt;

    // `galaxy()`: the format's test galaxy, whose one faction, `league`,
    // names no relation.

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    /// The faction `name`'s file, with `extra` front matter.
    fn faction_file(name: &str, extra: &str) -> String {
        format!("---\nname: {name}\nkind: minor\ncolor: \"#123456\"\n{extra}---\n")
    }

    fn govt(game: &NewGame, faction: &str) -> Govt {
        let nova = game.ids.factions[&id(faction)];
        game.data
            .get::<Govt>(nova)
            .expect("present")
            .expect("decodes")
            .record
            .clone()
    }

    /// The test galaxy with `band` and `crown`, each with `band` and
    /// `crown` front matter.
    fn with_two(band: &str, crown: &str) -> MemSource {
        galaxy()
            .with("factions/band.md", &faction_file("Band", band))
            .with("factions/crown.md", &faction_file("Crown", crown))
    }

    #[test]
    fn two_enemies_take_their_own_ids_as_classes_and_each_other_s_as_enemies() {
        let game = loaded(&with_two("enemies: [crown]\n", "enemies: [band]\n"));
        assert_eq!(problems(&game), Vec::<String>::new());
        let (band, crown) = (
            game.ids.factions[&id("band")],
            game.ids.factions[&id("crown")],
        );
        assert_ne!(band, crown);
        let (b, c) = (govt(&game, "band"), govt(&game, "crown"));
        assert_eq!(b.class, [band, -1, -1, -1]);
        assert_eq!(c.class, [crown, -1, -1, -1]);
        assert_eq!(b.enemy, [crown, -1, -1, -1]);
        assert_eq!(c.enemy, [band, -1, -1, -1]);
        assert_eq!((b.ally, c.ally), ([-1; 4], [-1; 4]));
    }

    #[test]
    fn one_side_naming_the_other_gives_the_named_one_a_class_and_the_namer_none() {
        let game = loaded(&with_two("allies: [crown]\n", ""));
        assert_eq!(problems(&game), Vec::<String>::new());
        let crown = game.ids.factions[&id("crown")];
        let (b, c) = (govt(&game, "band"), govt(&game, "crown"));
        assert_eq!(b.ally, [crown, -1, -1, -1]);
        assert_eq!(b.class, [-1; 4], "nobody names the band");
        assert_eq!(c.class, [crown, -1, -1, -1]);
        assert_eq!((c.ally, c.enemy), ([-1; 4], [-1; 4]));
    }

    #[test]
    fn a_faction_with_no_relations_beside_two_enemies_is_as_neutral_as_before() {
        let game = loaded(&with_two("enemies: [crown]\n", "enemies: [band]\n"));
        let league = govt(&game, "league");
        let expected = Govt {
            color: league.color,
            ship_color: league.ship_color,
            comm_name: league.comm_name.clone(),
            smug_penalty: league.smug_penalty,
            scan_fine: league.scan_fine,
            scan_mask: league.scan_mask,
            ..neutral_govt()
        };
        assert_eq!(league, expected);
    }

    #[test]
    fn the_stance_toward_the_player_sets_its_flag() {
        let game = loaded(&with_two(
            "attacks_player: always\n",
            "attacks_player: never\n",
        ));
        assert_eq!(govt(&game, "band").flags.0, ALWAYS_ATTACKS_PLAYER);
        assert_eq!(govt(&game, "crown").flags.0, NEVER_ATTACKS_PLAYER);
        assert_eq!(govt(&game, "league").flags.0, 0);
    }

    #[test]
    fn the_player_record_and_the_penalties_take_their_fields() {
        let game = loaded(&with_two(
            "player_record: -7\npenalties: {attack: 3, disable: 11, destroy: 29}\n",
            "",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        let band = govt(&game, "band");
        assert_eq!(
            (
                band.initial_rec,
                band.shoot_penalty,
                band.disab_penalty,
                band.kill_penalty
            ),
            (-7, 3, 11, 29)
        );
        assert_eq!(band.board_penalty, 0, "not mapped");
        assert_eq!(band.crime_tol, 0, "not mapped");
        assert_eq!(band.max_odds, 0, "not mapped");
    }

    #[test]
    fn a_record_or_penalty_outside_i16_is_held_and_reported() {
        let game = loaded(&with_two(
            "player_record: -40000\npenalties: {destroy: 70000}\n",
            "player_record: 40000\n",
        ));
        let (band, crown) = (govt(&game, "band"), govt(&game, "crown"));
        assert_eq!(band.initial_rec, i16::MIN);
        assert_eq!(band.kill_penalty, i16::MAX);
        assert_eq!(crown.initial_rec, i16::MAX);
        assert_eq!(
            problems(&game),
            [
                "factions/band.md: its player_record -40000 is outside -32768 to 32767; it is \
                 held at -32768",
                "factions/band.md: its penalties.destroy 70000 is outside -32768 to 32767; it is \
                 held at 32767",
                "factions/crown.md: its player_record 40000 is outside -32768 to 32767; it is \
                 held at 32767",
            ]
        );
    }

    #[test]
    fn an_unknown_name_itself_and_a_fifth_are_left_out_and_reported() {
        let source = galaxy()
            .with(
                "factions/band.md",
                &faction_file(
                    "Band",
                    "enemies: [ghost, band, a, b, a, c, d, e]\nallies: [league]\n",
                ),
            )
            .with("factions/a.md", &faction_file("A", ""))
            .with("factions/b.md", &faction_file("B", ""))
            .with("factions/c.md", &faction_file("C", ""))
            .with("factions/d.md", &faction_file("D", ""))
            .with("factions/e.md", &faction_file("E", ""));
        let game = loaded(&source);
        let nova = |name: &str| game.ids.factions[&id(name)];
        let band = govt(&game, "band");
        assert_eq!(band.enemy, [nova("a"), nova("b"), nova("c"), nova("d")]);
        assert_eq!(band.ally, [nova("league"), -1, -1, -1]);
        assert_eq!(band.class, [-1; 4], "naming itself gives it no class");
        assert_eq!(govt(&game, "e").class, [-1; 4], "left out, so not named");
        assert_eq!(govt(&game, "a").class, [nova("a"), -1, -1, -1]);
        assert_eq!(
            problems(&game),
            [
                "factions/band.md: its enemies name `ghost`, no faction; it is left out",
                "factions/band.md: its enemies name `band`, itself; it is left out",
                "factions/band.md: its enemies name `e`, past the fourth; it is left out",
            ]
        );
    }

    #[test]
    fn a_faction_named_both_ally_and_enemy_is_written_as_given() {
        let game = loaded(&with_two("allies: [crown]\nenemies: [crown]\n", ""));
        assert_eq!(problems(&game), Vec::<String>::new());
        let crown = game.ids.factions[&id("crown")];
        let band = govt(&game, "band");
        assert_eq!(
            (band.ally, band.enemy),
            ([crown, -1, -1, -1], [crown, -1, -1, -1])
        );
    }

    #[test]
    fn no_relations_is_the_neutral_record_s() {
        let neutral = neutral_govt();
        let none = Relations::NONE;
        assert_eq!(
            (none.class, none.ally, none.enemy, none.flags),
            (neutral.class, neutral.ally, neutral.enemy, neutral.flags.0)
        );
        assert_eq!(
            [
                none.initial_rec,
                none.shoot_penalty,
                none.disab_penalty,
                none.kill_penalty
            ],
            [
                neutral.initial_rec,
                neutral.shoot_penalty,
                neutral.disab_penalty,
                neutral.kill_penalty
            ]
        );
    }
}
