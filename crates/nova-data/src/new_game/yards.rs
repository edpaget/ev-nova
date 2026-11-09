//! Which yards sell which items: every item a `sold_at` lists gets a
//! `TechLevel` no yard reaches by its own, and each yard it names holds
//! that number in one of its eight `SpecialTech` slots.
//!
//! The original lists an outfit when its `TechLevel` is at or below the
//! `spöb`'s, or equals one of the `spöb`'s eight `SpecialTech` words
//! (`_SetupPortAvailableItems` @0xbfa2-0xbfbd, `nova_sim_port`'s
//! `wares::outfit_tech_allows`), and Buy tests the same
//! (`_CanBuyOutfitItem` @0x4e7ff-0x4e83d). The new game's yards stand at
//! [`YARD_TECH_LEGAL`](super::kits::YARD_TECH_LEGAL) and
//! [`YARD_TECH_LAWLESS`], so an item numbered from [`SOLD_TECH_FIRST`] to
//! [`SOLD_TECH_LAST`] is listed only where a slot holds its number;
//! [`OUTFIT_TECH_NEVER`] stays "never".
//!
//! - **One number per set of yards** ([`YardTech`]). Items sold at exactly
//!   the same yards share a number, and each new set takes the next, in
//!   the order first asked for. Two items sold at different sets of yards
//!   can't share one, and a yard needs one slot per set it is in, so this
//!   is the fewest slots there can be. The numbers are never saved (a
//!   pilot keeps outfit and ship IDs, not tech levels), so renumbering
//!   when the data grows changes nothing a player keeps.
//! - **The yards** ([`yards`]): each stellar a `sold_at` names, once,
//!   that has the service the item is sold from.
//! - **The outfits** ([`sales`]): each mapped outfit, in id order, with
//!   at least one yard, gets its set's number ([`Sales::outfits`]); its
//!   `oütf` takes it as `TechLevel`, with `BuyRandom` 100
//!   (`super::outfits`). Each yard's slots are its first eight numbers,
//!   ascending, `-1` for an unused slot ([`Sales::slots`]); its `spöb`
//!   takes them as `SpecialTech` (`super::map`).
//!
//! # Problems
//!
//! - A `sold_at` stellar that isn't in `stellars/`, or has no service the
//!   item is sold from, is reported against the item's file, and the
//!   item isn't sold there. A stellar that failed to load, or was left out
//!   for its system or its ID, is reported already and skipped.
//! - A yard in more than eight sets is reported against its own file: it
//!   keeps its eight lowest numbers, and the items of the rest aren't sold
//!   there (they still are at their other yards).
//! - Past [`SOLD_TECH_LAST`] an item is reported and not sold. The data
//!   can't reach it: an outfit and a hull each need a set, and there are
//!   far fewer `oütf` and `shïp` IDs than numbers.

use std::collections::{BTreeMap, BTreeSet};

use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Galaxy, Problem};
use nova_galaxy_format::model::Service;

use super::ids::IdTable;
use super::kits::YARD_TECH_LAWLESS;
use super::map::absent;
use super::records::OUTFIT_TECH_NEVER;

/// The first `TechLevel` an item sold at named yards takes: one past
/// every yard's own.
pub const SOLD_TECH_FIRST: i16 = YARD_TECH_LAWLESS + 1;

/// The last `TechLevel` an item sold at named yards takes: one below
/// "never".
pub const SOLD_TECH_LAST: i16 = OUTFIT_TECH_NEVER - 1;

/// How many `SpecialTech` slots a `spöb` has.
pub const SPECIAL_TECH_SLOTS: usize = 8;

/// The tech number of each distinct set of yards (`spöb` IDs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct YardTech {
    /// Each set given a number, and its number.
    numbers: BTreeMap<BTreeSet<i16>, i16>,
}

impl YardTech {
    /// The number of the set `yards`: the one it was given, or the next
    /// free one. None for an empty set, or once [`SOLD_TECH_LAST`] is
    /// passed.
    pub fn number(&mut self, yards: &BTreeSet<i16>) -> Option<i16> {
        if yards.is_empty() {
            return None;
        }
        if let Some(&number) = self.numbers.get(yards) {
            return Some(number);
        }
        let given = i16::try_from(self.numbers.len()).ok()?;
        let number = SOLD_TECH_FIRST
            .checked_add(given)
            .filter(|&number| number <= SOLD_TECH_LAST)?;
        self.numbers.insert(yards.clone(), number);
        Some(number)
    }

    /// Each yard's numbers, ascending.
    #[must_use]
    pub fn slots(&self) -> BTreeMap<i16, Vec<i16>> {
        let mut slots: BTreeMap<i16, Vec<i16>> = BTreeMap::new();
        for (yards, &number) in &self.numbers {
            for &yard in yards {
                slots.entry(yard).or_default().push(number);
            }
        }
        for numbers in slots.values_mut() {
            numbers.sort_unstable();
        }
        slots
    }
}

/// The `spöb` IDs of the stellars `sold_at` names, each once, that have
/// `service`; each that isn't in `stellars/` or hasn't `service` reported
/// against `file` (see the module docs).
pub fn yards(
    galaxy: &Galaxy,
    ids: &IdTable,
    file: &str,
    sold_at: &[Id],
    service: Service,
    problems: &mut Vec<Problem>,
) -> BTreeSet<i16> {
    let named: BTreeSet<&Id> = sold_at.iter().collect();
    let mut yards = BTreeSet::new();
    for stellar in named {
        match ids.stellars.get(stellar) {
            Some(&nova) if galaxy.stellars[stellar].data.services.contains(&service) => {
                yards.insert(nova);
            }
            Some(_) => problems.push(Problem::new(
                file,
                format!(
                    "the stellar `{stellar}` has no `{}` service; it isn't sold there",
                    service.as_str()
                ),
            )),
            None if absent(galaxy, &galaxy.stellars, "stellars", stellar) => {
                problems.push(Problem::new(
                    file,
                    format!("the stellar `{stellar}` is not in stellars/; it isn't sold there"),
                ));
            }
            None => {}
        }
    }
    yards
}

/// What the outfitters sell: each sold outfit's tech number, and each
/// yard's `SpecialTech` slots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sales {
    /// Each outfit sold at a yard, and its `TechLevel`.
    pub outfits: BTreeMap<Id, i16>,
    /// Each yard selling anything, by `spöb` ID, and its `SpecialTech`
    /// slots.
    pub slots: BTreeMap<i16, [i16; SPECIAL_TECH_SLOTS]>,
}

/// The sales of `galaxy`'s outfits numbered in `ids` (see the module
/// docs).
pub fn sales(galaxy: &Galaxy, ids: &IdTable, problems: &mut Vec<Problem>) -> Sales {
    let mut tech = YardTech::default();
    let mut sales = Sales::default();
    for id in ids.outfits.keys() {
        let doc = &galaxy.outfits[id];
        let at = yards(
            galaxy,
            ids,
            &doc.path,
            &doc.data.sold_at,
            Service::Outfitter,
            problems,
        );
        if at.is_empty() {
            continue;
        }
        match tech.number(&at) {
            Some(number) => {
                sales.outfits.insert(id.clone(), number);
            }
            None => problems.push(Problem::new(
                &doc.path,
                "no tech number is left; it isn't sold",
            )),
        }
    }
    let stellars: BTreeMap<i16, &Id> = ids.stellars.iter().map(|(id, &nova)| (nova, id)).collect();
    for (yard, numbers) in tech.slots() {
        let mut slots = [-1; SPECIAL_TECH_SLOTS];
        for (slot, &number) in slots.iter_mut().zip(&numbers) {
            *slot = number;
        }
        let over = numbers.get(SPECIAL_TECH_SLOTS..).unwrap_or_default();
        if !over.is_empty() {
            let left: Vec<String> = (sales.outfits.iter())
                .filter(|(_, number)| over.contains(number))
                .map(|(id, _)| format!("`{id}`"))
                .collect();
            problems.push(Problem::new(
                &galaxy.stellars[stellars[&yard]].path,
                format!(
                    "it sells from more than eight sets of yards, more than its eight \
                     `SpecialTech` slots; it doesn't sell {}",
                    left.join(", ")
                ),
            ));
        }
        sales.slots.insert(yard, slots);
    }
    sales
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::super::kits::YARD_TECH_LAWLESS;
    use super::super::tests::galaxy;
    use super::super::{NewGame, load};
    use super::*;
    use crate::records::outfit::Outfit;
    use crate::records::stellar::Stellar;

    fn set(yards: &[i16]) -> BTreeSet<i16> {
        yards.iter().copied().collect()
    }

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("games"))
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    /// The `oütf` of the outfit `outfit`.
    fn outfit(game: &NewGame, outfit: &str) -> Outfit {
        let nova = game.ids.outfits[&id(outfit)];
        game.data
            .get::<Outfit>(nova)
            .expect("present")
            .expect("decoded")
            .record
            .clone()
    }

    /// The `SpecialTech` slots of the `spöb` numbered `nova`.
    fn slots_of(game: &NewGame, nova: i16) -> [i16; 8] {
        let spob = game
            .data
            .get::<Stellar>(nova)
            .expect("present")
            .expect("decoded")
            .record
            .clone();
        let mut slots = [0; 8];
        slots[..3].copy_from_slice(&spob.special_tech1_3);
        slots[3..].copy_from_slice(&spob.special_tech4_8);
        slots
    }

    /// The `SpecialTech` slots of the stellar `stellar`.
    fn slots(game: &NewGame, stellar: &str) -> [i16; 8] {
        slots_of(game, game.ids.stellars[&id(stellar)])
    }

    /// The slots holding `numbers`, the rest unused.
    fn holding(numbers: &[i16]) -> [i16; 8] {
        let mut slots = [-1; 8];
        slots[..numbers.len()].copy_from_slice(numbers);
        slots
    }

    /// A stellar `name` in `system` offering `services`.
    fn stellar_text(name: &str, system: &str, services: &str) -> String {
        format!(
            "---\nname: {name}\nkind: station\nsystem: {system}\nposition: [0, 0]\n\
             services: [{services}]\n---\n"
        )
    }

    /// An outfit sold at `sold_at`, a YAML list's insides (none for no
    /// field).
    fn outfit_text(sold_at: Option<&str>) -> String {
        let sold_at = sold_at.map_or(String::new(), |at| format!("sold_at: [{at}]\n"));
        format!(
            "---\nname: O\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\nprice: 1\n{sold_at}---\n"
        )
    }

    /// The fixture galaxy with both its stellars outfitters and four more
    /// in `beta`, `b1` to `b4`.
    fn yards_galaxy() -> MemSource {
        let source = galaxy()
            .with(
                "stellars/alpha-yard.md",
                &stellar_text("Alpha Yard", "alpha", "outfitter"),
            )
            .with(
                "stellars/alpha-prime.md",
                &stellar_text("Alpha Prime", "alpha", "outfitter, refuel"),
            );
        (1..=4).fold(source, |source, n| {
            source.with(
                &format!("stellars/b{n}.md"),
                &stellar_text("B", "beta", "outfitter"),
            )
        })
    }

    #[test]
    fn the_sold_numbers_lie_between_every_yard_s_own_and_never() {
        assert_eq!(SOLD_TECH_FIRST, YARD_TECH_LAWLESS + 1);
        assert_eq!(SOLD_TECH_LAST, OUTFIT_TECH_NEVER - 1);
        assert_eq!((SOLD_TECH_FIRST, SOLD_TECH_LAST), (3, 0x7ffe));
        assert_eq!(SPECIAL_TECH_SLOTS, 8);
    }

    #[test]
    fn the_first_set_is_3_an_equal_set_shares_it_and_new_sets_count_up() {
        let mut tech = YardTech::default();
        assert_eq!(tech.number(&set(&[128])), Some(3));
        assert_eq!(tech.number(&set(&[128, 129])), Some(4));
        assert_eq!(tech.number(&set(&[128])), Some(3), "the same set");
        assert_eq!(tech.number(&set(&[129, 128])), Some(4));
        assert_eq!(tech.number(&set(&[130])), Some(5));
    }

    #[test]
    fn an_empty_set_gets_no_number_and_takes_none() {
        let mut tech = YardTech::default();
        assert_eq!(tech.number(&BTreeSet::new()), None);
        assert_eq!(tech.number(&set(&[128])), Some(3));
        assert_eq!(tech.slots(), BTreeMap::from([(128, vec![3])]));
    }

    #[test]
    fn each_yard_s_slots_are_its_numbers_ascending() {
        let mut tech = YardTech::default();
        tech.number(&set(&[129]));
        tech.number(&set(&[128, 129]));
        tech.number(&set(&[128]));
        assert_eq!(
            tech.slots(),
            BTreeMap::from([(128, vec![4, 5]), (129, vec![3, 4])])
        );
    }

    #[test]
    fn past_the_last_number_a_new_set_gets_none_and_an_old_one_keeps_its_own() {
        let mut tech = YardTech::default();
        let last = SOLD_TECH_LAST - SOLD_TECH_FIRST;
        for yard in 0..last {
            assert!(tech.number(&set(&[yard])).is_some(), "{yard}");
        }
        assert_eq!(tech.number(&set(&[last])), Some(SOLD_TECH_LAST));
        assert_eq!(tech.number(&set(&[last + 1])), None);
        assert_eq!(tech.number(&set(&[0])), Some(SOLD_TECH_FIRST));
        assert_eq!(tech.number(&set(&[last + 2])), None);
    }

    #[test]
    fn an_outfit_sold_at_a_yard_is_tech_3_always_offered_and_in_the_yard_s_first_slot() {
        let game =
            loaded(&yards_galaxy().with("outfits/blaster.md", &outfit_text(Some("alpha-yard"))));
        assert_eq!(problems(&game), Vec::<String>::new());
        let blaster = outfit(&game, "blaster");
        assert_eq!((blaster.tech_level, blaster.buy_random), (3, 100));
        assert_eq!(slots(&game, "alpha-yard"), holding(&[3]));
        assert_eq!(slots(&game, "alpha-prime"), holding(&[]), "sells nothing");
    }

    #[test]
    fn outfits_at_one_set_share_its_number_and_a_yard_holds_one_slot_per_set() {
        let game = loaded(
            &yards_galaxy()
                .with("outfits/blaster.md", &outfit_text(Some("alpha-yard")))
                .with("outfits/o1.md", &outfit_text(Some("alpha-yard, b1")))
                .with("outfits/o2.md", &outfit_text(Some("b1, alpha-yard"))),
        );
        assert_eq!(problems(&game), Vec::<String>::new());
        let tech = |id: &str| outfit(&game, id).tech_level;
        assert_eq!((tech("blaster"), tech("o1"), tech("o2")), (3, 4, 4));
        assert_eq!(slots(&game, "alpha-yard"), holding(&[3, 4]));
        assert_eq!(slots(&game, "b1"), holding(&[4]));
    }

    #[test]
    fn a_yard_named_twice_is_one_yard() {
        let game = loaded(
            &yards_galaxy()
                .with(
                    "outfits/blaster.md",
                    &outfit_text(Some("alpha-yard, alpha-yard")),
                )
                .with("outfits/o1.md", &outfit_text(Some("alpha-yard"))),
        );
        assert_eq!(problems(&game), Vec::<String>::new());
        assert_eq!(outfit(&game, "blaster").tech_level, 3);
        assert_eq!(outfit(&game, "o1").tech_level, 3, "the same set");
        assert_eq!(slots(&game, "alpha-yard"), holding(&[3]));
    }

    #[test]
    fn an_outfit_sold_nowhere_is_never_listed_or_offered() {
        for sold_at in [None, Some("")] {
            let game = loaded(&yards_galaxy().with("outfits/blaster.md", &outfit_text(sold_at)));
            assert_eq!(problems(&game), Vec::<String>::new(), "{sold_at:?}");
            let blaster = outfit(&game, "blaster");
            assert_eq!(
                (blaster.tech_level, blaster.buy_random),
                (OUTFIT_TECH_NEVER, 0),
                "{sold_at:?}"
            );
            assert_eq!(slots(&game, "alpha-yard"), holding(&[]), "{sold_at:?}");
        }
    }

    #[test]
    fn a_missing_stellar_or_one_without_an_outfitter_is_reported_and_not_sold_there() {
        let game = loaded(
            &yards_galaxy()
                .with(
                    "stellars/alpha-prime.md",
                    &stellar_text("Alpha Prime", "alpha", "bar, refuel"),
                )
                .with(
                    "outfits/blaster.md",
                    &outfit_text(Some("lost, alpha-prime, alpha-yard")),
                )
                .with("outfits/o1.md", &outfit_text(Some("alpha-prime"))),
        );
        assert_eq!(
            problems(&game),
            [
                "outfits/blaster.md: the stellar `alpha-prime` has no `outfitter` service; \
                 it isn't sold there",
                "outfits/blaster.md: the stellar `lost` is not in stellars/; it isn't sold there",
                "outfits/o1.md: the stellar `alpha-prime` has no `outfitter` service; \
                 it isn't sold there",
            ]
        );
        assert_eq!(outfit(&game, "blaster").tech_level, 3, "at the yard alone");
        assert_eq!(slots(&game, "alpha-yard"), holding(&[3]));
        assert_eq!(slots(&game, "alpha-prime"), holding(&[]));
        let o1 = outfit(&game, "o1");
        assert_eq!(
            (o1.tech_level, o1.buy_random),
            (OUTFIT_TECH_NEVER, 0),
            "no yard left"
        );
    }

    #[test]
    fn a_stellar_left_out_or_that_failed_to_load_is_not_reported_again() {
        let game = loaded(
            &yards_galaxy()
                .with(
                    "stellars/stray.md",
                    &stellar_text("Stray", "nowhere", "outfitter"),
                )
                .with("stellars/broken.md", "---\nname: Broken\n---\n")
                .with("outfits/blaster.md", &outfit_text(Some("stray, broken"))),
        );
        let lines = problems(&game);
        assert!(
            !lines.iter().any(|line| line.starts_with("outfits/")),
            "{lines:?}"
        );
        assert_eq!(outfit(&game, "blaster").tech_level, OUTFIT_TECH_NEVER);
    }

    #[test]
    fn a_yard_in_nine_sets_keeps_its_eight_lowest_and_reports_the_ninth_s_outfits() {
        let sets = [
            "alpha-yard",
            "alpha-yard, b1",
            "alpha-yard, b2",
            "alpha-yard, b3",
            "alpha-yard, b4",
            "alpha-yard, b1, b2",
            "alpha-yard, b1, b3",
            "alpha-yard, b1, b4",
            "alpha-yard, b2, b3",
        ];
        let source = sets
            .iter()
            .enumerate()
            .fold(yards_galaxy(), |source, (n, at)| {
                source.with(&format!("outfits/o{}.md", n + 1), &outfit_text(Some(at)))
            })
            .with("outfits/twin.md", &outfit_text(Some("alpha-yard, b2, b3")));
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            [
                "stellars/alpha-yard.md: it sells from more than eight sets of yards, more \
                 than its eight `SpecialTech` slots; it doesn't sell `o9`, `twin`"
            ]
        );
        assert_eq!(slots(&game, "alpha-yard"), [3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(outfit(&game, "o9").tech_level, 11);
        assert_eq!(outfit(&game, "twin").tech_level, 11);
        assert_eq!(slots(&game, "b3"), holding(&[6, 9, 11]), "still sold here");
    }

    #[test]
    fn a_yard_selling_nothing_and_every_gate_keep_every_slot_unused() {
        let game = loaded(&yards_galaxy());
        assert_eq!(problems(&game), Vec::<String>::new());
        for stellar in ["alpha-yard", "alpha-prime", "b1"] {
            assert_eq!(slots(&game, stellar), holding(&[]), "{stellar}");
        }
        assert!(!game.ids.gates.is_empty());
        for &gate in game.ids.gates.values() {
            assert_eq!(slots_of(&game, gate), holding(&[]), "{gate}");
        }
    }
}
