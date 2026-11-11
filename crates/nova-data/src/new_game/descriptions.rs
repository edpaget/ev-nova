//! The bodies the engine reads as descriptions, each a `dësc` holding the
//! body trimmed and kept as written, named for its thing's `name`:
//!
//! - a stellar's, at its `spöb`'s ID: its landing text;
//! - a ship's, at [`ship_desc_id`] of its `shïp`: the shipyard's
//!   description;
//! - an outfit's or a mod kit's, at [`outfit_desc_id`] of its `oütf`: the
//!   outfitter's description;
//! - `game.md`'s, at [`ABOUT_TEXT`]: the About text.
//!
//! An empty body makes none. A variant's body is its own file's: it isn't
//! inherited through `base`, since prose about a hull may be wrong for its
//! variant. A good's body makes none: no commodity screen reads one.

use nova_galaxy_format::load::Galaxy;

use super::ids::{ABOUT_TEXT, IdTable};
use super::records::text_desc;
use crate::decode::Entry;
use crate::registry::AnyRecord;
use crate::store::{outfit_desc_id, ship_desc_id};
use crate::wire::id::{OutfitId, ShipId};

/// The `dësc` of every non-empty body in `galaxy` the engine reads as a
/// description, at the IDs `ids` gives its things.
pub(super) fn descriptions(galaxy: &Galaxy, ids: &IdTable) -> Vec<Entry<AnyRecord>> {
    let stellars = (ids.stellars.iter()).map(|(id, &nova)| {
        (
            Some(nova),
            &galaxy.stellars[id].data.name,
            &galaxy.stellars[id].body,
        )
    });
    let ships = (ids.ships.iter()).map(|(id, &nova)| {
        let doc = &galaxy.ships[id];
        (ship_desc_id(ShipId(nova)), &doc.data.name, &doc.body)
    });
    let outfits = (ids.outfits.iter()).map(|(id, &nova)| {
        let doc = &galaxy.outfits[id];
        (outfit_desc_id(OutfitId(nova)), &doc.data.name, &doc.body)
    });
    let kits = (ids.kits.iter()).map(|(id, &nova)| {
        let doc = &galaxy.mods[id];
        (outfit_desc_id(OutfitId(nova)), &doc.data.name, &doc.body)
    });
    let about = (galaxy.game.iter()).map(|doc| (Some(ABOUT_TEXT), &doc.data.name, &doc.body));
    stellars
        .chain(ships)
        .chain(outfits)
        .chain(kits)
        .chain(about)
        .filter_map(|(id, name, body)| {
            let body = body.trim();
            let id = id.filter(|_| !body.is_empty())?;
            Some(Entry {
                id,
                name: Some(name.clone()),
                record: AnyRecord::Desc(Box::new(text_desc(body))),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use nova_galaxy_format::source::MemSource;

    use super::super::tests::galaxy;
    use super::super::{NewGame, load};
    use super::*;
    use crate::decode::Record;
    use crate::records::desc::Desc;

    fn loaded(source: &MemSource) -> NewGame {
        load(source, std::path::Path::new("sample"))
    }

    /// `dësc` `id`'s name and text.
    fn desc(game: &NewGame, id: i16) -> Option<(String, String)> {
        let entry = game.data.get::<Desc>(id)?.expect("decodes");
        assert_eq!(entry.record.graphic, None, "dësc {id} has no picture");
        Some((
            entry.name.expect("named").to_owned(),
            entry.record.text.as_str().to_owned(),
        ))
    }

    /// `dësc` `name` holding `text`, as [`desc`] gives it.
    #[expect(clippy::unnecessary_wraps, reason = "compared with `desc`'s")]
    fn pair(name: &str, text: &str) -> Option<(String, String)> {
        Some((name.to_owned(), text.to_owned()))
    }

    /// The fixture's skiff (`shïp` 129, after `armed-skiff`) with `body`.
    fn skiff_with(body: &str) -> MemSource {
        galaxy().with(
            "ships/skiff.md",
            &format!("---\nname: Skiff\nstyle: civilian\nrole: shuttle\n---\n{body}"),
        )
    }

    #[test]
    fn a_ships_body_is_its_description_at_13000_on() {
        let game = loaded(&skiff_with("\nA small hull.\n\nIt hauls.\n\n"));
        assert_eq!(game.ids.ships[&id("skiff")], 129);
        assert_eq!(
            desc(&game, 13001),
            pair("Skiff", "A small hull.\n\nIt hauls.")
        );
    }

    #[test]
    fn a_variant_without_a_body_has_no_description_though_its_base_has() {
        let game = loaded(&skiff_with("A small hull.\n"));
        assert_eq!(game.ids.ships[&id("armed-skiff")], 128);
        assert_eq!(desc(&game, 13000), None);
        assert_eq!(
            loaded(&galaxy().with(
                "ships/armed-skiff.md",
                "---\nname: Armed skiff\nbase: skiff\n---\nArmed.\n"
            ))
            .data
            .ids(Desc::TYPE),
            [13000],
            "its own body is its own"
        );
    }

    #[test]
    fn an_outfits_and_a_kits_bodies_are_their_descriptions_at_3000_on() {
        let source = galaxy()
            .with(
                "outfits/blaster.md",
                "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\n\
                 price: 100\n---\nIt shoots.\n",
            )
            .with(
                "mods/gun-mount.md",
                "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
                 effects: {cargo: -2, attention: 2}\n---\nA ring.\n",
            );
        let game = loaded(&source);
        assert_eq!(
            (
                game.ids.outfits[&id("blaster")],
                game.ids.kits[&id("gun-mount")]
            ),
            (128, 129)
        );
        assert_eq!(desc(&game, 3000), pair("Blaster", "It shoots."));
        assert_eq!(desc(&game, 3001), pair("Gun mount", "A ring."));
    }

    #[test]
    fn game_md_is_the_about_text_named_for_the_game() {
        let game = loaded(&galaxy().with("game.md", "---\nname: Hearthlight\n---\n\nAbout it.\n"));
        assert_eq!(desc(&game, 32767), pair("Hearthlight", "About it."));
        assert_eq!(desc(&loaded(&galaxy()), 32767), None, "none without one");
    }

    #[test]
    fn empty_bodies_give_none() {
        let source = skiff_with(" \n\n")
            .with("game.md", "---\nname: Hearthlight\n---\n\n")
            .with(
                "mods/gun-mount.md",
                "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
                 effects: {cargo: -2, attention: 2}\n---\n\t\n",
            );
        let game = loaded(&source);
        assert!(game.problems.is_empty(), "{:?}", game.problems);
        assert!(game.galaxy.game.is_some());
        assert_eq!(game.data.ids(Desc::TYPE), Vec::<i16>::new());
    }

    #[test]
    fn a_goods_body_gives_none() {
        let source = galaxy().with(
            "goods/grain.md",
            "---\nname: Grain\nprice: 10\n---\nFood.\n",
        );
        let game = loaded(&source);
        assert!(
            game.galaxy.goods.contains_key(&id("grain")),
            "{:?}",
            game.problems
        );
        assert_eq!(game.data.ids(Desc::TYPE), Vec::<i16>::new());
    }

    #[test]
    fn a_thing_past_its_descriptions_range_has_none() {
        let mut ids = IdTable::default();
        ids.ships.insert(id("skiff"), 896);
        let (galaxy, _) = nova_galaxy_format::load::load(&skiff_with("A hull.\n"));
        assert!(descriptions(&galaxy, &ids).is_empty());
    }

    fn id(text: &str) -> nova_galaxy_format::id::Id {
        nova_galaxy_format::id::Id::try_from(text.to_owned()).expect("an id")
    }
}
