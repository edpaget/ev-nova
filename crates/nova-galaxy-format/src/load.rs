//! Loads every content file into a [`Galaxy`], reporting each file that
//! can't be read instead of stopping at the first.
//!
//! Each kind's folder is read with [`Source::listing`], so a file that
//! can't be read drops only itself (`couldn't be read: …`, against its
//! path). A folder that can't be listed drops its kind, with one problem
//! against `dir/` (`listing failed: …`), and goes into
//! [`Galaxy::unreadable`] as `dir/`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::DeserializeOwned;

use crate::asset::{Asset, Budget};
use crate::fleet::{Livery, ModKit, Outfit, Ship};
use crate::frontmatter;
use crate::game::{self, Game};
use crate::goods::Good;
use crate::id::{Id, MediaPath, PointRef};
use crate::model::{
    Character, Faction, JumpPoint, Kind, Mission, News, Region, Start, Stellar, Storyline, System,
};
use crate::source::Source;

/// One loaded content file.
#[derive(Debug)]
pub struct Doc<T> {
    /// Its path in the data folder, e.g. `systems/marrow.md`.
    pub path: String,
    /// Its front matter.
    pub data: T,
    /// Its Markdown body.
    pub body: String,
}

/// Every content file that loaded, by kind and id.
#[derive(Debug, Default)]
pub struct Galaxy {
    /// `regions/`.
    pub regions: BTreeMap<Id, Doc<Region>>,
    /// `factions/`.
    pub factions: BTreeMap<Id, Doc<Faction>>,
    /// `systems/`.
    pub systems: BTreeMap<Id, Doc<System>>,
    /// `stellars/`.
    pub stellars: BTreeMap<Id, Doc<Stellar>>,
    /// `goods/`.
    pub goods: BTreeMap<Id, Doc<Good>>,
    /// `storylines/`.
    pub storylines: BTreeMap<Id, Doc<Storyline>>,
    /// `missions/`.
    pub missions: BTreeMap<Id, Doc<Mission>>,
    /// `news/`.
    pub news: BTreeMap<Id, Doc<News>>,
    /// `characters/`.
    pub characters: BTreeMap<Id, Doc<Character>>,
    /// `starts/`.
    pub starts: BTreeMap<Id, Doc<Start>>,
    /// `ships/`.
    pub ships: BTreeMap<Id, Doc<Ship>>,
    /// `mods/`.
    pub mods: BTreeMap<Id, Doc<ModKit>>,
    /// `outfits/`.
    pub outfits: BTreeMap<Id, Doc<Outfit>>,
    /// `liveries/`.
    pub liveries: BTreeMap<Id, Doc<Livery>>,
    /// `assets/`: the asset manifest.
    pub assets: BTreeMap<Id, Doc<Asset>>,
    /// `budgets/`: what art may cost.
    pub budgets: BTreeMap<Id, Doc<Budget>>,
    /// `game.md`, the game as a whole, if the folder has one and it
    /// loaded.
    pub game: Option<Doc<Game>>,
    /// Files with a valid name that failed to load, e.g.
    /// `systems/marrow.md`, and kind folders that failed to list, spelled
    /// with a trailing `/`, e.g. `systems/`. References to them aren't
    /// reported again: see [`Galaxy::is_unreadable`].
    pub unreadable: BTreeSet<String>,
}

impl Galaxy {
    /// Every media file path an asset names (`files`, `sources`,
    /// `licence_file`), each once, sorted.
    #[must_use]
    pub fn media_paths(&self) -> Vec<&str> {
        self.media().into_iter().map(MediaPath::as_str).collect()
    }

    /// Every media file an asset names, each once, sorted.
    pub(crate) fn media(&self) -> BTreeSet<&MediaPath> {
        self.assets
            .values()
            .flat_map(|asset| {
                let a = &asset.data;
                a.files.iter().chain(&a.sources).chain(&a.licence_file)
            })
            .collect()
    }

    /// Whether the file `dir/id.md` failed to load, or the folder `dir`
    /// failed to list, so a reference to `id` isn't reported again.
    #[must_use]
    pub fn is_unreadable(&self, dir: &str, id: &Id) -> bool {
        self.unreadable.contains(&format!("{dir}/"))
            || self.unreadable.contains(&format!("{dir}/{id}.md"))
    }

    /// The jump point `r` names, as `systems/` has it, if its system
    /// loaded and has a point with that id.
    #[must_use]
    pub fn jump_point(&self, r: &PointRef) -> Option<&JumpPoint> {
        let system = self.systems.get(&r.system())?;
        system.data.jump_points.iter().find(|p| p.id == r.point())
    }
}

/// Something wrong with one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The file's path in the data folder.
    pub file: String,
    /// What is wrong.
    pub message: String,
}

impl Problem {
    /// A problem in `file`.
    pub fn new(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// Loads every content file from `source`. Files that can't be read are
/// left out and reported.
pub fn load(source: &impl Source) -> (Galaxy, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut unreadable = BTreeSet::new();
    let (p, u) = (&mut problems, &mut unreadable);
    let galaxy = Galaxy {
        regions: kind::<Region>(source, p, u),
        factions: kind::<Faction>(source, p, u),
        systems: kind::<System>(source, p, u),
        stellars: kind::<Stellar>(source, p, u),
        goods: kind::<Good>(source, p, u),
        storylines: kind::<Storyline>(source, p, u),
        missions: kind::<Mission>(source, p, u),
        news: kind::<News>(source, p, u),
        characters: kind::<Character>(source, p, u),
        starts: kind::<Start>(source, p, u),
        ships: kind::<Ship>(source, p, u),
        mods: kind::<ModKit>(source, p, u),
        outfits: kind::<Outfit>(source, p, u),
        liveries: kind::<Livery>(source, p, u),
        assets: kind::<Asset>(source, p, u),
        budgets: kind::<Budget>(source, p, u),
        game: game::read(source, p, u),
        unreadable,
    };
    (galaxy, problems)
}

fn kind<T: DeserializeOwned + Kind>(
    source: &impl Source,
    problems: &mut Vec<Problem>,
    unreadable: &mut BTreeSet<String>,
) -> BTreeMap<Id, Doc<T>> {
    let dir = T::DIR;
    let listed = match source.listing(dir, "md") {
        Ok(listed) => listed,
        Err(e) => {
            let folder = format!("{dir}/");
            unreadable.insert(folder.clone());
            problems.push(Problem::new(folder, format!("listing failed: {e}")));
            return BTreeMap::new();
        }
    };
    let mut docs = BTreeMap::new();
    for file in listed {
        let stem = file
            .path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".md");
        let loaded = Id::try_from(stem.to_string())
            .map_err(|e| format!("the file name {e}"))
            .and_then(|id| match &file.text {
                Ok(text) => {
                    frontmatter::parse::<T>(text).map(|(t, body)| (id, t, body.to_string()))
                }
                Err(e) => Err(format!("couldn't be read: {e}")),
            });
        match loaded {
            Ok((id, data, body)) => {
                docs.insert(
                    id,
                    Doc {
                        path: file.path,
                        data,
                        body,
                    },
                );
            }
            Err(message) => {
                if Id::try_from(stem.to_string()).is_ok() {
                    unreadable.insert(file.path.clone());
                }
                problems.push(Problem::new(file.path, message));
            }
        }
    }
    docs
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use crate::fixture;
    use crate::source::{Listed, MemSource, SourceFile};

    #[test]
    fn loads_each_kind_by_file_name() {
        let (galaxy, problems) = load(&fixture::galaxy());
        assert!(problems.is_empty(), "{problems:?}");
        let ids: Vec<_> = galaxy.systems.keys().map(Id::as_str).collect();
        assert_eq!(ids, ["alpha", "beta"]);
        assert_eq!(galaxy.regions.len(), 1);
        assert_eq!(galaxy.factions.len(), 1);
        assert_eq!(galaxy.stellars.len(), 2);
        assert_eq!(galaxy.storylines.len(), 1);
        assert_eq!(galaxy.missions.len(), 1);
        assert_eq!(galaxy.characters.len(), 1);
        let news: Vec<_> = galaxy.news.keys().map(Id::as_str).collect();
        assert_eq!(news, ["parcel-delivered"]);
        let paths: Vec<_> = galaxy.news.values().map(|d| d.path.as_str()).collect();
        assert_eq!(paths, ["news/parcel-delivered.md"]);
    }

    #[test]
    fn loads_the_starts() {
        let (galaxy, problems) = load(&fixture::galaxy());
        assert!(problems.is_empty(), "{problems:?}");
        let starts: Vec<_> = galaxy.starts.keys().map(Id::as_str).collect();
        assert_eq!(starts, ["first-flight"]);
        let start = &galaxy.starts.values().next().unwrap();
        assert_eq!(start.path, "starts/first-flight.md");
        assert_eq!(
            (start.data.system.as_str(), start.data.ship.as_str()),
            ("alpha", "skiff")
        );
    }

    #[test]
    fn a_start_naming_what_is_not_there_still_loads() {
        // References are the checker's and the engine's business.
        let source = fixture::galaxy().with(
            "starts/lost.md",
            "---\nname: Lost\nsystem: gamma\nship: barge\ncash: 0\n---\n",
        );
        let (galaxy, problems) = load(&source);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(galaxy.starts.len(), 2);
    }

    #[test]
    fn loads_the_fleet_and_assets() {
        let (galaxy, problems) = load(&fixture::galaxy());
        assert!(problems.is_empty(), "{problems:?}");
        let ships: Vec<_> = galaxy.ships.keys().map(Id::as_str).collect();
        assert_eq!(ships, ["armed-skiff", "skiff"]);
        assert_eq!(galaxy.mods.len(), 1);
        assert_eq!(galaxy.outfits.len(), 1);
        assert_eq!(galaxy.liveries.len(), 1);
        assert_eq!(galaxy.assets.len(), 3);
        let budgets: Vec<_> = galaxy.budgets.values().map(|d| d.path.as_str()).collect();
        assert_eq!(budgets, ["budgets/default.md"]);
    }

    #[test]
    fn lists_each_media_path_the_manifest_names_once_sorted() {
        // `hull` and `surface` both name the pack's licence file.
        let (galaxy, _) = load(&fixture::galaxy());
        assert_eq!(
            galaxy.media_paths(),
            [
                "media/export/pack/LICENSE.txt",
                "media/export/pack/hull.glb",
                "media/export/pack/surface.png"
            ]
        );
    }

    #[test]
    fn lists_sources_among_the_media_paths() {
        let source = fixture::galaxy().with(
            "assets/spire.md",
            "---\nkind: model\nplaceholder: true\nsources: [media/source/spire.blend]\n---\n",
        );
        let (galaxy, _) = load(&source);
        assert!(
            galaxy.media_paths().contains(&"media/source/spire.blend"),
            "{:?}",
            galaxy.media_paths()
        );
    }

    #[test]
    fn finds_the_jump_point_a_point_ref_names() {
        let (galaxy, _) = load(&fixture::galaxy());
        let point = |r: &str| galaxy.jump_point(&PointRef::try_from(r.to_string()).unwrap());
        let to_beta = point("alpha/to-beta").expect("alpha/to-beta");
        assert_eq!(to_beta.to.as_str(), "beta/to-alpha");
        assert_eq!(point("beta/to-alpha").unwrap().id.as_str(), "to-alpha");
        assert!(point("alpha/to-alpha").is_none());
        assert!(point("gamma/to-beta").is_none());
    }

    #[test]
    fn loads_game_md_beside_the_kinds() {
        let (galaxy, problems) = load(&fixture::galaxy());
        assert!(problems.is_empty(), "{problems:?}");
        assert!(galaxy.game.is_none(), "the fixture has no game.md");
        let source = fixture::galaxy().with("game.md", "---\nname: Test\n---\nAbout.\n");
        let (galaxy, problems) = load(&source);
        assert!(problems.is_empty(), "{problems:?}");
        let game = galaxy.game.expect("game.md");
        assert_eq!(
            (game.data.name.as_str(), game.body.as_str()),
            ("Test", "About.\n")
        );
        let (galaxy, problems) = load(&fixture::galaxy().with_bytes("game.md", b"\xff"));
        assert!(galaxy.game.is_none());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(galaxy.unreadable.contains("game.md"));
    }

    #[test]
    fn keeps_the_path_and_body() {
        let (galaxy, _) = load(&fixture::galaxy());
        let alpha = galaxy.systems.values().next().unwrap();
        assert_eq!(alpha.path, "systems/alpha.md");
        assert_eq!(alpha.body, "A quiet place.\n");
    }

    #[test]
    fn a_file_name_that_is_not_an_id_is_a_problem() {
        let source = fixture::galaxy().with("systems/Gamma.md", "---\nname: G\n---\n");
        let (galaxy, problems) = load(&source);
        assert_eq!(galaxy.systems.len(), 2);
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].file, "systems/Gamma.md");
        assert!(
            problems[0].message.contains("kebab-case"),
            "{}",
            problems[0]
        );
    }

    #[test]
    fn a_file_that_cannot_be_read_drops_only_itself() {
        let source = fixture::galaxy().with_bytes("systems/garbled.md", b"\xff");
        let (galaxy, problems) = load(&source);
        let ids: Vec<_> = galaxy.systems.keys().map(Id::as_str).collect();
        assert_eq!(ids, ["alpha", "beta"]);
        // The format's own relative path, always written with `/`, not an
        // OS path: no Windows separator can reach it.
        let garbled: Vec<_> = problems
            .iter()
            .filter(|p| p.file == "systems/garbled.md")
            .collect();
        assert_eq!(garbled.len(), 1, "{problems:?}");
        assert!(
            garbled[0].message.starts_with("couldn't be read: "),
            "{}",
            garbled[0]
        );
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(galaxy.unreadable.contains("systems/garbled.md"));
    }

    #[test]
    fn a_file_that_cannot_be_read_and_is_not_an_id_is_not_marked_unreadable() {
        let source = fixture::galaxy().with_bytes("systems/Garbled.md", b"\xff");
        let (galaxy, problems) = load(&source);
        assert_eq!(galaxy.systems.len(), 2);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert_eq!(problems[0].file, "systems/Garbled.md");
        assert!(galaxy.unreadable.is_empty(), "{:?}", galaxy.unreadable);
    }

    /// The fixture galaxy, whose `systems/` can't be listed.
    struct Unlisted(MemSource);
    impl Source for Unlisted {
        fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
            self.0.files(dir, ext)
        }
        fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
            if dir == "systems" {
                return Err(io::Error::other("the disk is unreadable"));
            }
            self.0.listing(dir, ext)
        }
        fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
            self.0.bytes(path)
        }
    }

    #[test]
    fn a_folder_that_cannot_be_listed_marks_the_kind_unreadable() {
        let (galaxy, problems) = load(&Unlisted(fixture::galaxy()));
        assert!(galaxy.systems.is_empty());
        assert_eq!(galaxy.stellars.len(), 2, "the other kinds load");
        assert_eq!(
            problems,
            [Problem::new(
                "systems/",
                "listing failed: the disk is unreadable"
            )]
        );
        assert!(galaxy.unreadable.contains("systems/"));
        assert!(galaxy.is_unreadable("systems", &id("alpha")));
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_string()).unwrap()
    }

    #[test]
    fn is_unreadable_matches_the_file_or_its_folder() {
        let mut galaxy = Galaxy::default();
        assert!(
            !galaxy.is_unreadable("systems", &id("alpha")),
            "nothing failed"
        );
        galaxy.unreadable.insert("systems/alpha.md".to_string());
        assert!(galaxy.is_unreadable("systems", &id("alpha")), "its file");
        assert!(
            !galaxy.is_unreadable("systems", &id("beta")),
            "another file"
        );
        assert!(
            !galaxy.is_unreadable("stellars", &id("alpha")),
            "another kind"
        );
        galaxy.unreadable.insert("stellars/".to_string());
        assert!(galaxy.is_unreadable("stellars", &id("alpha")), "its folder");
        assert!(
            !galaxy.is_unreadable("missions", &id("alpha")),
            "another folder"
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_a_problem_with_its_line() {
        let source = fixture::galaxy().with(
            "regions/rim.md",
            "---\nname: Rim\nharshness: 9\ncolor: \"#335577\"\n---\n",
        );
        let (galaxy, problems) = load(&source);
        assert!(galaxy.regions.is_empty());
        assert_eq!(
            problems[0]
                .to_string()
                .split(": ")
                .take(2)
                .collect::<Vec<_>>(),
            ["regions/rim.md", "line 3, column 12"]
        );
    }
}
