//! Outfits as minimal `oütf`s: something a pilot can own, so a mission
//! can grant and remove one ([`super::missions`]).
//!
//! Each outfit is numbered within [`OUTFIT_IDS`] ([`number`]) and maps to
//! a neutral `oütf` ([`outfit`], over [`neutral_outfit`]) with the
//! outfit's `name` and its `price` as `Cost`. It weighs nothing, does
//! nothing (every `ModType` slot unused), has no limit on how many are
//! owned, and is never for sale: its `TechLevel` is the original's
//! "never listed" value, so no outfitter lists it under either reading
//! of the tech level rule.
//!
//! The rest of an outfit (`mount`, `damage`, `range`, `reload` and
//! `model`) isn't read: weapons and systems come with `play-the-sample`
//! phase 3.

use std::collections::BTreeMap;

use nova_galaxy_format::fleet::Outfit as BibleOutfit;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};

use super::ids::OUTFIT_IDS;
use super::map::number;
use super::records::neutral_outfit;
use crate::decode::Entry;
use crate::records::outfit::Outfit;
use crate::registry::AnyRecord;

/// Numbers the outfits of `galaxy` within [`OUTFIT_IDS`].
pub fn number_outfits(galaxy: &Galaxy, problems: &mut Vec<Problem>) -> BTreeMap<Id, i16> {
    number(&galaxy.outfits, OUTFIT_IDS, "oütf", problems)
}

/// The `oütf` of the outfit `doc`, numbered `nova` (see the module docs).
#[must_use]
pub fn outfit(nova: i16, doc: &Doc<BibleOutfit>) -> Entry<AnyRecord> {
    let outfit = Outfit {
        cost: i32::try_from(doc.data.price).unwrap_or(i32::MAX),
        ..neutral_outfit()
    };
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::Outfit(Box::new(outfit)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::super::records::OUTFIT_TECH_NEVER;
    use super::super::tests::galaxy;
    use super::super::{NewGame, load};
    use super::*;
    use crate::decode::Record;

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("games"))
    }

    fn named(game: &NewGame, nova: i16) -> (String, Outfit) {
        let entry = (game.data.get::<Outfit>(nova))
            .expect("present")
            .expect("decoded");
        (entry.name.expect("named").to_owned(), entry.record.clone())
    }

    fn pass(name: &str, price: u32) -> String {
        format!(
            "---\nname: {name}\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\nprice: {price}\n---\n"
        )
    }

    #[test]
    fn an_outfit_maps_at_128_with_its_name_and_price() {
        let game = loaded(&galaxy());
        assert_eq!(game.data.ids(Outfit::TYPE), [128], "the blaster alone");
        assert_eq!(
            game.ids.outfits[&Id::try_from("blaster".to_owned()).unwrap()],
            128
        );
        let (name, outfit) = named(&game, 128);
        assert_eq!(name, "Blaster");
        assert_eq!(outfit.cost, 100);
    }

    #[test]
    fn an_outfit_weighs_nothing_does_nothing_has_no_limit_and_is_never_sold() {
        let (_, outfit) = named(&loaded(&galaxy()), 128);
        assert_eq!(outfit.mass, 0);
        assert_eq!(outfit.max, i16::MAX);
        assert_eq!(outfit.tech_level, OUTFIT_TECH_NEVER);
        assert_eq!(OUTFIT_TECH_NEVER, 0x7fff);
        let slots = [
            (outfit.mod_type, outfit.mod_val),
            (outfit.mod_type2, outfit.mod_val2),
            (outfit.mod_type3, outfit.mod_val3),
            (outfit.mod_type4, outfit.mod_val4),
        ];
        assert_eq!(slots, [(-1, 0); 4]);
        assert_eq!(outfit.buy_random, 0);
        assert_eq!(outfit.require_govt, -1);
        assert_eq!(outfit.flags.bits(), 0);
    }

    #[test]
    fn two_outfits_number_in_id_order() {
        let game = loaded(&galaxy().with("outfits/anchor.md", &pass("Anchor", 7)));
        assert_eq!(game.data.ids(Outfit::TYPE), [128, 129]);
        assert_eq!(named(&game, 128).0, "Anchor");
        assert_eq!(named(&game, 129).0, "Blaster");
    }

    #[test]
    fn an_outfit_past_the_last_outfit_id_is_a_problem_and_left_out() {
        let source = (0..512).fold(galaxy(), |source, n| {
            source.with(&format!("outfits/a{n:03}.md"), &pass("A", 0))
        });
        let game = loaded(&source);
        let lines: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            ["outfits/blaster.md: it is past the last oütf ID, 639; it is left out"]
        );
        assert_eq!(game.data.ids(Outfit::TYPE).len(), 512);
        assert!(
            !game
                .ids
                .outfits
                .contains_key(&Id::try_from("blaster".to_owned()).unwrap())
        );
    }

    #[test]
    fn a_price_past_i32_is_the_most() {
        let game = loaded(&galaxy().with("outfits/blaster.md", &pass("Blaster", u32::MAX)));
        assert_eq!(named(&game, 128).1.cost, i32::MAX);
    }
}
