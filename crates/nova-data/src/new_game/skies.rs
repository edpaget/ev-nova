//! The check of each system's sky decorations ([`unknown_images`]): a
//! decoration's `image` is an asset's id
//! (`nova_galaxy_format::sky::decoration::Decoration`), and one the
//! manifest doesn't hold is a problem.
//!
//! The store keeps nothing of a sky: the edge reads each system's look
//! and decorations from the loaded galaxy
//! ([`NewGame::galaxy`](super::NewGame::galaxy)) for the 3D layer
//! (`nova_render::sky_decorations`), which leaves out a decoration whose
//! image it can't have and reports only what it finds reading the image.
//! An id that names no asset is the data's fault, known without reading
//! any file, so it is reported here, as a stellar's `visual` is
//! ([`super::looks`]).
//!
//! # Problems
//!
//! Each against the system's file, once for each id however often the
//! file lists it, in the list's order:
//!
//! - a decoration's `image` that isn't in `assets/`: it is not drawn, and
//!   the others keep their places.
//!
//! A reference to an asset that failed to load isn't reported again. What
//! the asset holds (a model in place of a texture, a file that doesn't
//! read) is the edge's to report, since it reads it.

use std::collections::BTreeSet;

use nova_galaxy_format::load::{Galaxy, Problem};

use super::ids::IdTable;
use super::map::absent;

/// Adds to `problems` each sky decoration's image, of the systems of
/// `galaxy` that have a record in `ids`, that isn't in `assets/` (see the
/// module docs).
pub fn unknown_images(galaxy: &Galaxy, ids: &IdTable, problems: &mut Vec<Problem>) {
    for id in ids.systems.keys() {
        let doc = &galaxy.systems[id];
        let listed = (doc.data.sky.as_ref())
            .and_then(|sky| sky.decorations.as_ref())
            .map_or(&[][..], |listed| listed.as_slice());
        let mut told = BTreeSet::new();
        for image in listed.iter().map(|decoration| &decoration.image) {
            if absent(galaxy, &galaxy.assets, "assets", image) && told.insert(image) {
                problems.push(Problem::new(
                    &doc.path,
                    format!("its sky decoration `{image}` is not in assets/; it is not drawn"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::{MemSource, Source};

    use crate::new_game::load;
    use crate::new_game::tests::galaxy;

    /// `source` with `block` at the end of system `system`'s front matter.
    fn with_sky(source: MemSource, system: &str, block: &str) -> MemSource {
        let file = format!("systems/{system}.md");
        let bytes = source.bytes(&file).expect("it reads").expect("the file");
        let text = String::from_utf8(bytes).expect("text");
        let (front, body) = text
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---\n"))
            .expect("front matter");
        source.with(&file, &format!("---\n{front}\n{block}---\n{body}"))
    }

    /// What loading `source` finds wrong.
    fn problems(source: &MemSource) -> Vec<String> {
        load(source, Path::new("sample"))
            .problems
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn an_image_not_in_assets_is_reported_against_its_systems_file() {
        let source = with_sky(galaxy(), "alpha", "sky: {decorations: [{image: gone}]}\n");
        assert_eq!(
            problems(&source),
            ["systems/alpha.md: its sky decoration `gone` is not in assets/; it is not drawn"]
        );
    }

    #[test]
    fn an_image_in_assets_is_not_reported_whatever_its_asset_holds() {
        // A texture, a model (the edge's to report, reading it) and a
        // placeholder with no file.
        let source = with_sky(
            galaxy(),
            "alpha",
            "sky: {decorations: [{image: surface}, {image: hull}, {image: spire}]}\n",
        );
        assert_eq!(problems(&source), Vec::<String>::new());
    }

    #[test]
    fn a_system_with_no_sky_no_list_or_an_empty_one_reports_nothing() {
        let source = with_sky(
            with_sky(galaxy(), "alpha", "sky: {decorations: []}\n"),
            "beta",
            "sky: {stars: 0.5}\n",
        );
        assert_eq!(problems(&source), Vec::<String>::new());
        assert_eq!(problems(&galaxy()), Vec::<String>::new());
    }

    #[test]
    fn each_unknown_image_of_a_list_is_reported_in_the_lists_order_among_those_that_load() {
        let source = with_sky(
            galaxy(),
            "alpha",
            "sky: {decorations: [{image: zeta}, {image: surface}, {image: gone}]}\n",
        );
        assert_eq!(
            problems(&source),
            [
                "systems/alpha.md: its sky decoration `zeta` is not in assets/; it is not drawn",
                "systems/alpha.md: its sky decoration `gone` is not in assets/; it is not drawn",
            ]
        );
    }

    #[test]
    fn an_image_a_file_lists_twice_is_reported_once_and_again_for_another_file() {
        let source = with_sky(
            with_sky(
                galaxy(),
                "alpha",
                "sky: {decorations: [{image: gone}, {image: gone, size: 40}]}\n",
            ),
            "beta",
            "sky: {decorations: [{image: gone}]}\n",
        );
        assert_eq!(
            problems(&source),
            [
                "systems/alpha.md: its sky decoration `gone` is not in assets/; it is not drawn",
                "systems/beta.md: its sky decoration `gone` is not in assets/; it is not drawn",
            ]
        );
    }

    #[test]
    fn an_image_whose_asset_failed_to_load_is_not_reported_again() {
        let source = with_sky(
            galaxy().with("assets/surface.md", "---\nkind: [\n---\n"),
            "beta",
            "sky: {decorations: [{image: surface}]}\n",
        );
        let problems = problems(&source);
        assert!(!problems.is_empty(), "the asset's own problem");
        assert!(
            problems.iter().all(|p| p.starts_with("assets/surface.md")),
            "{problems:?}"
        );
    }
}
