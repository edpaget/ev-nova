//! The traffic the data names: each fleet (`fleets/`) as a `düde`
//! ([`fleets`]), and each system's `traffic` as its `sÿst`'s traffic
//! slots ([`slots`]). A system that names no traffic flies the placeholder
//! civilian traffic ([`super::fleet::civilian_traffic`]).
//!
//! # A fleet's `düde`
//!
//! | Field | Value |
//! |---|---|
//! | name | the fleet's `name` |
//! | `Govt` | its `faction`'s `gövt`; none for a fleet of no faction |
//! | `AIType` | its `role`'s ([`ai_type`]): `trader` 1, a wimpy trader that flees; `armed-trader` 2, a brave trader that fights back; `warship` 3, which hunts its government's enemies, and the player by its government's word or the player's record; `interceptor` 4, police, hunting as a warship does with no retreat on its shields |
//! | `ShipType1-16`, `Probability1-16` | the ships it names that have a `shïp`, hulls and variants alike, in ship id order, each at its weight held within `i16`: the engine reads the weights as raw odds against each other |
//!
//! Every other field is neutral ([`neutral_dude`]). A pirate fleet is a
//! `warship` fleet of a faction that attacks the player on sight; the
//! `gövt` bit that sends a warship to the pirate AI isn't mapped.
//!
//! The fleets are numbered after the last guard `düde`
//! ([`super::blockades::number_guards`]), or after [`CIVILIAN_DUDE`] with
//! none, in fleet id order, up to the last of [`DUDE_IDS`]. Like the
//! guards, they are unpinned: nothing saves a `düde` ID ([`super::ids`]).
//!
//! # A system's slots
//!
//! A system that names `traffic` gets each fleet's `düde` in
//! `DudeTypes1-8` and its weight as a whole percentage ([`percentages`])
//! in `Prob1-8`, in fleet id order. The engine rescales a system's `düde`
//! weights to sum to 100, rounding each down, and never picks one at 0, so
//! raw weights of 1 and 150 would fly the first never. The mapper does the
//! rescale itself instead: each weight comes out at least 1 and they sum
//! to 100, so the engine's rescale leaves them as they are. The rounding:
//!
//! 1. A fleet whose share of what is left is under 1% is held at 1. Holding
//!    some leaves the others less, so this repeats until none is under.
//! 2. The others share what the held ones leave in proportion to their
//!    weights, each rounded down.
//! 3. What that leaves goes one each to the fleets with the largest
//!    remainders, the earliest first on a tie.
//!
//! A system flies at most eight fleets, so the floor of 1 always fits in
//! 100. The arithmetic is wide, so a weight past `i16` keeps its
//! proportion. Its `AvgShips`
//! is `average_ships`, held within `i16`, or [`TRAFFIC_SHIPS`] when it
//! names none. A system whose `traffic` names no fleet that flies
//! (`fleets: {}`, say) has no traffic: every `DudeTypes` -1 and `AvgShips`
//! 0. A system that names no `traffic` flies the civilian traffic,
//! [`CIVILIAN_DUDE`] at 100 with [`TRAFFIC_SHIPS`] ships, or none when
//! there is no civilian hull.
//!
//! # Problems
//!
//! Each is reported once, and the rest still maps:
//!
//! - Against the fleet's file: a ship that isn't in `ships/` is left out;
//!   a seventeenth ship it can fly is left out, the first sixteen kept; a
//!   faction that isn't in `factions/` gives no government; a fleet with
//!   no ship it can fly, or past the last `düde` ID, gets no `düde`.
//! - Against the system's file: a fleet that isn't in `fleets/` is left
//!   out; a ninth fleet that flies is left out, the first eight kept.
//!
//! A ship that is in `ships/` but was left out for its chain or its ID,
//! and a fleet that failed to load or got no `düde`, aren't reported
//! again.

use std::collections::BTreeMap;

use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::System as BibleSystem;
use nova_galaxy_format::traffic::{Fleet, FleetRole};

use super::fleet::{TRAFFIC_SHIPS, saturated};
use super::ids::{CIVILIAN_DUDE, DUDE_IDS, IdTable};
use super::map::{self, absent, number};
use super::records::neutral_dude;
use crate::decode::Entry;
use crate::records::dude::Dude;
use crate::registry::AnyRecord;
use crate::wire::id::ShipId;

/// How many ship types a `düde` holds.
const FLEET_SHIPS: usize = 16;

/// How many `düde`s a `sÿst` names.
const SYSTEM_FLEETS: usize = 8;

/// The AI type a fleet's `role` flies its ships as (see the module docs).
///
/// It matches every role: each one the format adds must be given its AI
/// type here.
#[must_use]
pub fn ai_type(role: FleetRole) -> i16 {
    match role {
        FleetRole::Trader => 1,
        FleetRole::ArmedTrader => 2,
        FleetRole::Warship => 3,
        FleetRole::Interceptor => 4,
    }
}

/// The fleets' `düde`s and their IDs.
#[derive(Debug, Default)]
pub struct Fleets {
    /// Each fleet's `düde` ID, by its id: only the fleets that got one.
    pub ids: BTreeMap<Id, i16>,
    /// The `düde`s, in ID order.
    pub records: Vec<Entry<AnyRecord>>,
}

/// Every fleet of `galaxy` that flies a ship as its `düde`, numbered after
/// the last of `guards` (see the module docs).
pub fn fleets(
    galaxy: &Galaxy,
    ids: &IdTable,
    guards: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> Fleets {
    let mut flying: BTreeMap<&Id, (&Doc<Fleet>, Vec<Flown>)> = BTreeMap::new();
    for (id, doc) in &galaxy.fleets {
        let ships = flown(galaxy, ids, doc, problems);
        if ships.is_empty() {
            problems.push(Problem::new(
                &doc.path,
                "it has no ship it can fly; it is left out",
            ));
        } else {
            flying.insert(id, (doc, ships));
        }
    }
    let first = guards
        .values()
        .max()
        .map_or(CIVILIAN_DUDE + 1, |&last| last + 1);
    let mut free = first..=*DUDE_IDS.end();
    let docs = flying.iter().map(|(&id, (doc, _))| (id, *doc));
    let numbered = number(docs, &BTreeMap::new(), &mut free, "düde", problems);
    let records = numbered
        .iter()
        .map(|(id, &nova)| {
            let (doc, ships) = &flying[id];
            fleet(galaxy, ids, doc, ships, nova, problems)
        })
        .collect();
    Fleets {
        ids: numbered,
        records,
    }
}

/// A ship a fleet flies, and its weight.
type Flown = (ShipId, i16);

/// The ships fleet `doc` names that have a `shïp`, in ship id order, each
/// with its weight, the first [`FLEET_SHIPS`] of them; the rest, and each
/// that isn't in `ships/`, reported.
fn flown(
    galaxy: &Galaxy,
    ids: &IdTable,
    doc: &Doc<Fleet>,
    problems: &mut Vec<Problem>,
) -> Vec<Flown> {
    let mut ships = Vec::new();
    let mut over = Vec::new();
    for (ship, weight) in &doc.data.ships {
        if let Some(&nova) = ids.ships.get(ship) {
            if ships.len() < FLEET_SHIPS {
                ships.push((ShipId(nova), saturated(weight.get().into())));
            } else {
                over.push(format!("`{ship}`"));
            }
        } else if absent(galaxy, &galaxy.ships, "ships", ship) {
            problems.push(Problem::new(
                &doc.path,
                format!("ships names `{ship}`, which is not in ships/; it is left out"),
            ));
        }
    }
    if !over.is_empty() {
        problems.push(Problem::new(
            &doc.path,
            format!(
                "ships names more than {FLEET_SHIPS} ships it can fly; {} left out",
                over.join(", ")
            ),
        ));
    }
    ships
}

/// The `düde` of fleet `doc`, numbered `nova`, flying `ships`.
fn fleet(
    galaxy: &Galaxy,
    ids: &IdTable,
    doc: &Doc<Fleet>,
    ships: &[Flown],
    nova: i16,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let mut dude = Dude {
        ai_type: ai_type(doc.data.role),
        govt: map::govt(galaxy, ids, doc.data.faction.as_ref(), &doc.path, problems),
        ..neutral_dude()
    };
    for (slot, &(ship, weight)) in ships.iter().enumerate() {
        dude.ship_type[slot] = Some(ship);
        dude.probability[slot] = weight;
    }
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::Dude(Box::new(dude)),
    }
}

/// A `sÿst`'s traffic slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slots {
    /// `DudeTypes1-8`: a `düde` ID, or -1 for none.
    pub dude_types: [i16; SYSTEM_FLEETS],
    /// `Prob1-8`: each `düde`'s weight, as a percentage.
    pub prob: [i16; SYSTEM_FLEETS],
    /// `AvgShips`.
    pub avg_ships: i16,
}

/// The traffic slots of system `doc`: the fleets its `traffic` names that
/// have a `düde` in `fleets`, or, naming none, the civilian traffic when
/// `civilian` (see the module docs).
pub fn slots(
    galaxy: &Galaxy,
    fleets: &BTreeMap<Id, i16>,
    civilian: bool,
    doc: &Doc<BibleSystem>,
    problems: &mut Vec<Problem>,
) -> Slots {
    let mut slots = Slots {
        dude_types: [-1; SYSTEM_FLEETS],
        prob: [0; SYSTEM_FLEETS],
        avg_ships: 0,
    };
    let Some(traffic) = &doc.data.traffic else {
        if civilian {
            slots.dude_types[0] = CIVILIAN_DUDE;
            slots.prob[0] = 100;
            slots.avg_ships = TRAFFIC_SHIPS;
        }
        return slots;
    };
    let mut flying = Vec::new();
    let mut over = Vec::new();
    for (fleet, weight) in &traffic.fleets {
        if let Some(&nova) = fleets.get(fleet) {
            if flying.len() < SYSTEM_FLEETS {
                flying.push((nova, weight.get()));
            } else {
                over.push(format!("`{fleet}`"));
            }
        } else if absent(galaxy, &galaxy.fleets, "fleets", fleet) {
            problems.push(Problem::new(
                &doc.path,
                format!("traffic.fleets names `{fleet}`, which is not in fleets/; it is left out"),
            ));
        }
    }
    if !over.is_empty() {
        problems.push(Problem::new(
            &doc.path,
            format!(
                "traffic.fleets names more than {SYSTEM_FLEETS} fleets that fly; {} left out",
                over.join(", ")
            ),
        ));
    }
    let weights: Vec<u32> = flying.iter().map(|&(_, weight)| weight).collect();
    for (slot, (&(nova, _), percent)) in flying.iter().zip(percentages(&weights)).enumerate() {
        slots.dude_types[slot] = nova;
        slots.prob[slot] = percent;
    }
    if !flying.is_empty() {
        slots.avg_ships = traffic
            .average_ships
            .map_or(TRAFFIC_SHIPS, |ships| saturated(ships.into()));
    }
    slots
}

/// What a system's `Prob` sum to: the engine's rescale of a `düde`'s
/// `% Prob` leaves weights summing to this as they are.
const PERCENT: u64 = 100;

/// `weights`, at most [`SYSTEM_FLEETS`] of them and each at least 1, as
/// whole percentages, each at least 1 and summing to [`PERCENT`] (see the
/// module docs' rounding rule). The arithmetic is in `u64`, so no weight is
/// held within `i16` first.
fn percentages(weights: &[u32]) -> Vec<i16> {
    let weights: Vec<u64> = weights.iter().map(|&weight| u64::from(weight)).collect();
    let mut held = vec![false; weights.len()];
    // The fleets held at 1 leave the others less, so holding some can push
    // another under 1.
    let (pool, total) = loop {
        let pool = PERCENT - held.iter().map(|&h| u64::from(h)).sum::<u64>();
        let total: u64 = (weights.iter().zip(&held))
            .filter(|&(_, &h)| !h)
            .map(|(&weight, _)| weight)
            .sum();
        let mut more = false;
        for (&weight, h) in weights.iter().zip(&mut held) {
            if !*h && pool * weight < total {
                *h = true;
                more = true;
            }
        }
        if !more {
            break (pool, total);
        }
    };
    let mut shares: Vec<u64> = (weights.iter().zip(&held))
        .map(|(&weight, &h)| if h { 1 } else { pool * weight / total })
        .collect();
    let mut by_remainder: Vec<usize> = (0..weights.len()).filter(|&i| !held[i]).collect();
    // Stable, so the earliest first on a tie.
    by_remainder.sort_by_key(|&i| std::cmp::Reverse(pool * weights[i] % total));
    // Each share is rounded down from its part of `pool`, so they leave
    // fewer than one per fleet not held.
    let left = PERCENT - shares.iter().sum::<u64>();
    for &i in by_remainder.iter().take(usize::try_from(left).unwrap_or(0)) {
        shares[i] += 1;
    }
    shares
        .into_iter()
        .map(|share| i16::try_from(share).unwrap_or(i16::MAX))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::new_game::tests::galaxy;
    use crate::new_game::{NewGame, load};
    use crate::records::system::System;
    use crate::wire::id::GovtId;

    // `galaxy()`: the format's test galaxy. `league` (128) wears
    // `league-paint`; the variant `armed-skiff` is 128 and the hull `skiff`
    // 129; `alpha` is 128 and `beta` 129. Nothing holds a point, so there
    // are no guards and the first fleet is 129.

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("sample"))
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    /// `source` with the fleet `name`, whose front matter is `yaml`.
    fn with_fleet(source: MemSource, name: &str, yaml: &str) -> MemSource {
        source.with(&format!("fleets/{name}.md"), &format!("---\n{yaml}---\n"))
    }

    /// `source` with the test galaxy's system `name` naming `traffic` too.
    fn with_system(source: MemSource, name: &str, traffic: &str) -> MemSource {
        let file = match name {
            "alpha" => {
                "name: Alpha\nregion: rim\ngovernment: league\nmap: [0, 0]\njump_points:\n\
                 \x20 - id: to-beta\n    position: [100, 0]\n    to: beta/to-alpha\n    \
                 stability: stable\n"
            }
            "beta" => {
                "name: Beta\nregion: rim\nmap: [200, 50]\njump_points:\n\
                 \x20 - id: to-alpha\n    position: [-100, 0]\n    to: alpha/to-beta\n    \
                 stability: unstable\n    owner: league\n"
            }
            _ => panic!("{name} isn't one of the test galaxy's systems"),
        };
        source.with(
            &format!("systems/{name}.md"),
            &format!("---\n{file}{traffic}---\n"),
        )
    }

    fn dude(game: &NewGame, nova: i16) -> Option<(String, Dude)> {
        let entry = game.data.get::<Dude>(nova)?.expect("decodes");
        Some((entry.name.expect("named").to_owned(), entry.record.clone()))
    }

    fn system(game: &NewGame, name: &str) -> System {
        let nova = game.ids.systems[&id(name)];
        let entry = game.data.get::<System>(nova).expect("present");
        entry.expect("decodes").record.clone()
    }

    fn slots_of(game: &NewGame, name: &str) -> Slots {
        let system = system(game, name);
        Slots {
            dude_types: system.dude_types,
            prob: system.prob,
            avg_ships: system.avg_ships,
        }
    }

    /// The slots holding `fleets`, as (`düde`, weight), then `avg_ships`.
    fn holding(fleets: &[(i16, i16)], avg_ships: i16) -> Slots {
        let mut slots = Slots {
            dude_types: [-1; 8],
            prob: [0; 8],
            avg_ships,
        };
        for (slot, &(dude, weight)) in fleets.iter().enumerate() {
            slots.dude_types[slot] = dude;
            slots.prob[slot] = weight;
        }
        slots
    }

    const PATROL: &str =
        "name: League patrol\nfaction: league\nrole: warship\nships: {skiff: 3, armed-skiff: 1}\n";

    #[test]
    fn a_fleet_maps_to_a_dude_with_its_name_faction_role_and_weighted_ships() {
        let game = loaded(&with_fleet(galaxy(), "patrol", PATROL));
        assert_eq!(problems(&game), Vec::<String>::new());
        let (name, patrol) = dude(&game, CIVILIAN_DUDE + 1).expect("the patrol");
        assert_eq!(name, "League patrol");
        let mut ship_type = [None; 16];
        ship_type[0] = Some(ShipId(game.ids.ships[&id("armed-skiff")]));
        ship_type[1] = Some(ShipId(game.ids.ships[&id("skiff")]));
        let mut probability = [0; 16];
        probability[..2].copy_from_slice(&[1, 3]);
        assert_eq!(
            patrol,
            Dude {
                ai_type: 3,
                govt: Some(GovtId(game.ids.factions[&id("league")])),
                ship_type,
                probability,
                ..neutral_dude()
            },
            "the variant too, in ship id order"
        );
    }

    #[test]
    fn each_role_flies_its_ai_type() {
        assert_eq!(ai_type(FleetRole::Trader), 1);
        assert_eq!(ai_type(FleetRole::ArmedTrader), 2);
        assert_eq!(ai_type(FleetRole::Warship), 3);
        assert_eq!(ai_type(FleetRole::Interceptor), 4);
        let game = loaded(&with_fleet(
            galaxy(),
            "haulers",
            "name: Haulers\nrole: armed-trader\nships: {skiff: 1}\n",
        ));
        let (_, haulers) = dude(&game, CIVILIAN_DUDE + 1).expect("the haulers");
        assert_eq!(haulers.ai_type, 2, "read from the file");
    }

    #[test]
    fn a_fleet_of_no_faction_has_no_government_and_no_problem() {
        let game = loaded(&with_fleet(
            galaxy(),
            "haulers",
            "name: Haulers\nrole: trader\nships: {skiff: 1}\n",
        ));
        assert_eq!(problems(&game), Vec::<String>::new());
        let (_, haulers) = dude(&game, CIVILIAN_DUDE + 1).expect("the haulers");
        assert_eq!((haulers.govt, haulers.ai_type), (None, 1));
    }

    #[test]
    fn a_weight_past_i16_is_held_at_the_most() {
        let game = loaded(&with_fleet(
            galaxy(),
            "haulers",
            "name: Haulers\nrole: trader\nships: {skiff: 40000}\n",
        ));
        let (_, haulers) = dude(&game, CIVILIAN_DUDE + 1).expect("the haulers");
        assert_eq!(haulers.probability[0], i16::MAX);
    }

    /// `source` with seventeen bare hulls, `s00` to `s16`.
    fn seventeen_hulls(source: MemSource) -> MemSource {
        (0..17).fold(source, |source, n| {
            source.with(
                &format!("ships/s{n:02}.md"),
                "---\nname: S\nstyle: union\nrole: fighter\n---\n",
            )
        })
    }

    #[test]
    fn a_fleet_s_problems_are_reported_and_it_maps_what_is_left() {
        let crowd = (0..17).fold(String::new(), |mut ships, n| {
            let _ = write!(ships, "s{n:02}: 1, ");
            ships
        });
        let source = seventeen_hulls(galaxy());
        let source = with_fleet(
            source,
            "crowd",
            &format!("name: Crowd\nrole: trader\nships: {{{crowd}}}\n"),
        );
        let source = with_fleet(
            source,
            "empty",
            "name: Empty\nrole: trader\nships: {ghost: 1}\n",
        );
        let source = with_fleet(
            source,
            "lost",
            "name: Lost\nrole: trader\nships: {skiff: 1, wraith: 2}\n",
        );
        let source = with_fleet(
            source,
            "stray",
            "name: Stray\nfaction: nobody\nrole: trader\nships: {skiff: 1}\n",
        );
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            [
                "fleets/crowd.md: ships names more than 16 ships it can fly; `s16` left out",
                "fleets/empty.md: ships names `ghost`, which is not in ships/; it is left out",
                "fleets/empty.md: it has no ship it can fly; it is left out",
                "fleets/lost.md: ships names `wraith`, which is not in ships/; it is left out",
                "fleets/stray.md: the faction `nobody` is not in factions/; it has no government",
            ]
        );
        let ship = |name: &str| Some(ShipId(game.ids.ships[&id(name)]));
        let (name, crowd) = dude(&game, CIVILIAN_DUDE + 1).expect("the crowd");
        assert_eq!(name, "Crowd");
        assert_eq!(crowd.ship_type[15], ship("s15"), "the first sixteen kept");
        assert_eq!(crowd.probability, [1; 16]);
        let (name, lost) = dude(&game, CIVILIAN_DUDE + 2).expect("the lost, after the crowd");
        assert_eq!(name, "Lost", "the empty fleet gets no düde");
        assert_eq!(lost.ship_type[..2], [ship("skiff"), None]);
        let (name, stray) = dude(&game, CIVILIAN_DUDE + 3).expect("the stray");
        assert_eq!((name.as_str(), stray.govt), ("Stray", None));
        assert_eq!(dude(&game, CIVILIAN_DUDE + 4), None);
    }

    #[test]
    fn a_ship_left_out_for_its_chain_is_not_reported_again() {
        let source = galaxy().with("ships/looped.md", "---\nname: Looped\nbase: looped\n---\n");
        let game = loaded(&with_fleet(
            source,
            "loopers",
            "name: Loopers\nrole: trader\nships: {looped: 1, skiff: 1}\n",
        ));
        let fleet_problems: Vec<_> = (problems(&game).into_iter())
            .filter(|p| p.starts_with("fleets/"))
            .collect();
        assert_eq!(fleet_problems, Vec::<String>::new());
    }

    /// The test galaxy with the fleets `a-fleet` and `b-fleet`, and `alpha`
    /// naming `traffic`.
    fn two_fleets(traffic: &str) -> MemSource {
        let source = with_fleet(
            galaxy(),
            "a-fleet",
            "name: A\nrole: trader\nships: {skiff: 1}\n",
        );
        let source = with_fleet(
            source,
            "b-fleet",
            "name: B\nrole: warship\nships: {skiff: 1}\n",
        );
        with_system(source, "alpha", traffic)
    }

    #[test]
    fn a_system_names_its_fleets_with_their_weights_and_average_ships() {
        let source =
            two_fleets("traffic:\n  fleets: {b-fleet: 30, a-fleet: 10}\n  average_ships: 9\n");
        let source = with_system(source, "beta", "traffic: {fleets: {a-fleet: 5}}\n");
        let game = loaded(&source);
        assert_eq!(problems(&game), Vec::<String>::new());
        let (a, b) = (CIVILIAN_DUDE + 1, CIVILIAN_DUDE + 2);
        assert_eq!(
            slots_of(&game, "alpha"),
            holding(&[(a, 25), (b, 75)], 9),
            "its weights as percentages"
        );
        assert_eq!(
            slots_of(&game, "beta"),
            holding(&[(a, 100)], TRAFFIC_SHIPS),
            "the usual number when it names none"
        );
    }

    /// The `Prob` that `alpha`'s `traffic` naming the two fleets at
    /// `a` and `b` maps to.
    fn two_weights(a: u64, b: u64) -> [i16; 2] {
        let game = loaded(&two_fleets(&format!(
            "traffic: {{fleets: {{a-fleet: {a}, b-fleet: {b}}}}}\n"
        )));
        let prob = slots_of(&game, "alpha").prob;
        assert_eq!(prob[2..], [0; 6]);
        [prob[0], prob[1]]
    }

    #[test]
    fn a_fleet_under_a_percent_of_its_system_still_flies_at_one() {
        assert_eq!(two_weights(1, 150), [1, 99]);
    }

    #[test]
    fn an_even_split_stays_even() {
        assert_eq!(two_weights(50, 50), [50, 50]);
        assert_eq!(two_weights(7, 7), [50, 50]);
    }

    #[test]
    fn the_remainder_goes_to_the_largest_remainders_earliest_first() {
        assert_eq!(two_weights(2, 1), [67, 33], "66.7 and 33.3");
        assert_eq!(two_weights(1, 2), [33, 67]);
        assert_eq!(two_weights(1, 1), [50, 50]);
        assert_eq!(two_weights(1, 7), [13, 87], "12.5 and 87.5: the first");
    }

    #[test]
    fn weights_past_i16_keep_their_proportions() {
        assert_eq!(two_weights(40_000, 120_000), [25, 75]);
        assert_eq!(two_weights(40_000, 32_767), [55, 45], "not held even");
        let most = u64::from(u32::MAX);
        assert_eq!(two_weights(most, most), [50, 50], "without overflow");
        assert_eq!(two_weights(1, most), [1, 99]);
    }

    #[test]
    fn a_system_s_average_past_i16_is_held_at_the_most() {
        let game = loaded(&two_fleets(
            "traffic: {fleets: {a-fleet: 40000}, average_ships: 40000}\n",
        ));
        let slots = slots_of(&game, "alpha");
        assert_eq!((slots.prob[0], slots.avg_ships), (100, i16::MAX));
    }

    /// The test galaxy with the fleets `f1` to `f{count}`, each flying a
    /// skiff.
    fn numbered_fleets(count: usize) -> MemSource {
        (1..=count).fold(galaxy(), |source, n| {
            with_fleet(
                source,
                &format!("f{n}"),
                "name: F\nrole: trader\nships: {skiff: 1}\n",
            )
        })
    }

    #[test]
    fn eight_fleets_one_far_heavier_each_keep_one_and_sum_to_100() {
        let source = with_system(
            numbered_fleets(8),
            "alpha",
            "traffic: {fleets: {f1: 1, f2: 1, f3: 1, f4: 1, f5: 1, f6: 1, f7: 1, f8: 10000}}\n",
        );
        let slots = slots_of(&loaded(&source), "alpha");
        assert_eq!(slots.prob, [1, 1, 1, 1, 1, 1, 1, 93]);
    }

    #[test]
    fn the_fleets_held_at_one_leave_the_rest_to_the_others_in_proportion() {
        // Six fleets under a percent get 1 each; 3000 and 7000 share the
        // other 94 as 28.2 and 65.8.
        let source = with_system(
            numbered_fleets(8),
            "alpha",
            "traffic: {fleets: {f1: 1, f2: 1, f3: 1, f4: 1, f5: 1, f6: 1, f7: 3000, f8: 7000}}\n",
        );
        let prob = slots_of(&loaded(&source), "alpha").prob;
        assert_eq!(prob, [1, 1, 1, 1, 1, 1, 28, 66]);
    }

    #[test]
    fn a_fleet_the_others_held_at_one_push_under_a_percent_is_held_at_one_too() {
        // f4's 9 is 1.03% of 870, but 0.99% of what the five held at one
        // leave (95 of 861): it is held too, and f5 and f8 share 94 as
        // 7.503 and 86.497.
        let source = with_system(
            numbered_fleets(8),
            "alpha",
            "traffic: {fleets: {f1: 1, f2: 1, f3: 1, f4: 9, f5: 68, f6: 1, f7: 5, f8: 784}}\n",
        );
        let prob = slots_of(&loaded(&source), "alpha").prob;
        assert_eq!(prob, [1, 1, 1, 1, 8, 1, 1, 86]);
    }

    #[test]
    fn every_mapped_weight_is_at_least_one_and_they_sum_to_100() {
        // So the engine's rescale to 100 leaves them as they are.
        for weights in [
            [1, 150, 3, 9_999, 2, 40_000, 7, 1],
            [u32::MAX, 1, 1, 1, 1, 1, 1, 1],
            [3, 3, 3, 3, 3, 3, 3, 3],
            [1, 2, 3, 4, 5, 6, 7, 8],
        ] {
            let named = (weights.iter().enumerate()).fold(String::new(), |mut named, (n, w)| {
                let _ = write!(named, "f{}: {w}, ", n + 1);
                named
            });
            let source = with_system(
                numbered_fleets(8),
                "alpha",
                &format!("traffic: {{fleets: {{{named}}}}}\n"),
            );
            let prob = slots_of(&loaded(&source), "alpha").prob;
            assert!(prob.iter().all(|&p| p >= 1), "{weights:?}: {prob:?}");
            assert_eq!(prob.iter().sum::<i16>(), 100, "{weights:?}: {prob:?}");
        }
    }

    #[test]
    fn a_system_naming_no_fleet_has_no_traffic() {
        let game = loaded(&two_fleets("traffic: {fleets: {}, average_ships: 9}\n"));
        assert_eq!(problems(&game), Vec::<String>::new());
        assert_eq!(slots_of(&game, "alpha"), holding(&[], 0));
    }

    #[test]
    fn a_missing_fleet_and_a_ninth_are_reported_and_left_out() {
        let source = (1..=9).fold(galaxy(), |source, n| {
            with_fleet(
                source,
                &format!("f{n}"),
                "name: F\nrole: trader\nships: {skiff: 1}\n",
            )
        });
        let source = with_fleet(
            source,
            "grounded",
            "name: Grounded\nrole: trader\nships: {ghost: 1}\n",
        );
        let source = source.with("fleets/garbled.md", "---\nname: G\nrole: pirate\n---\n");
        let named = (1..=9).fold(String::new(), |mut named, n| {
            let _ = write!(named, "f{n}: 1, ");
            named
        });
        let source = with_system(
            source,
            "alpha",
            &format!("traffic: {{fleets: {{{named}ghost: 1, grounded: 1, garbled: 1}}}}\n"),
        );
        let game = loaded(&source);
        let alpha: Vec<_> = (problems(&game).into_iter())
            .filter(|p| p.starts_with("systems/alpha.md"))
            .collect();
        assert_eq!(
            alpha,
            [
                "systems/alpha.md: traffic.fleets names `ghost`, which is not in fleets/; it is \
                 left out",
                "systems/alpha.md: traffic.fleets names more than 8 fleets that fly; `f9` left out",
            ],
            "the one that failed to load and the one with no düde aren't reported again"
        );
        let slots = slots_of(&game, "alpha");
        let first: Vec<i16> = (1..=8).map(|n| CIVILIAN_DUDE + n).collect();
        assert_eq!(slots.dude_types.to_vec(), first, "f1 to f8");
        assert_eq!(
            slots.prob,
            [13, 13, 13, 13, 12, 12, 12, 12],
            "eight at 12.5: the first four rounded up"
        );
        assert_eq!(slots.avg_ships, TRAFFIC_SHIPS);
    }

    #[test]
    fn a_system_naming_traffic_beside_one_naming_none_leaves_the_other_civilian() {
        let game = loaded(&two_fleets("traffic: {fleets: {a-fleet: 1}}\n"));
        assert_eq!(
            slots_of(&game, "alpha").dude_types[0],
            CIVILIAN_DUDE + 1,
            "its own fleet"
        );
        assert_eq!(
            slots_of(&game, "beta"),
            holding(&[(CIVILIAN_DUDE, 100)], TRAFFIC_SHIPS),
            "the civilian traffic, as before"
        );
    }

    #[test]
    fn a_galaxy_with_no_civilian_hull_has_no_traffic_where_it_names_none() {
        let galaxy = nova_galaxy_format::load::load(&galaxy()).0;
        let doc = &galaxy.systems[&id("beta")];
        let none = slots(&galaxy, &BTreeMap::new(), false, doc, &mut Vec::new());
        assert_eq!(none, holding(&[], 0));
        let civilian = slots(&galaxy, &BTreeMap::new(), true, doc, &mut Vec::new());
        assert_eq!(civilian, holding(&[(CIVILIAN_DUDE, 100)], TRAFFIC_SHIPS));
    }

    #[test]
    fn fleets_number_after_the_guards_in_fleet_id_order() {
        let source = two_fleets("");
        let game = loaded(&source);
        let names = |game: &NewGame, at: i16| dude(game, at).expect("a düde").0;
        assert_eq!(names(&game, CIVILIAN_DUDE + 1), "A", "no holder: from 129");
        assert_eq!(names(&game, CIVILIAN_DUDE + 2), "B");
        let held = source.with(
            "missions/toll.md",
            "---\ntitle: Toll\nstoryline: errands\nact: crossroads\n\
             objective: {take_point: {point: beta/to-alpha, held_by: league}}\n---\n",
        );
        let game = loaded(&held);
        assert_eq!(names(&game, CIVILIAN_DUDE + 1), "League guards");
        assert_eq!(names(&game, CIVILIAN_DUDE + 2), "A", "after the guards");
        assert_eq!(names(&game, CIVILIAN_DUDE + 3), "B");
    }

    #[test]
    fn a_fleet_past_the_last_dude_id_is_reported_and_gets_none() {
        let source = two_fleets("");
        let (galaxy, _) = nova_galaxy_format::load::load(&source);
        let game = loaded(&source);
        let guards = BTreeMap::from([(id("league"), *DUDE_IDS.end() - 1)]);
        let mut problems = Vec::new();
        let fleets = fleets(&galaxy, &game.ids, &guards, &mut problems);
        assert_eq!(
            fleets.ids,
            BTreeMap::from([(id("a-fleet"), *DUDE_IDS.end())])
        );
        assert_eq!(fleets.records.len(), 1);
        assert_eq!(
            problems,
            [Problem::new(
                "fleets/b-fleet.md",
                "it is past the last düde ID, 639; it is left out"
            )]
        );
        assert_eq!(DUDE_IDS, 128..=639);
    }
}
