//! Loads every content file into a [`Galaxy`], reporting each file that
//! can't be read instead of stopping at the first.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::DeserializeOwned;

use crate::asset::{Asset, Budget};
use crate::fleet::{Livery, ModKit, Outfit, Ship};
use crate::frontmatter;
use crate::id::{Id, PointRef};
use crate::model::{
    Character, Faction, JumpPoint, Kind, Mission, News, Region, Stellar, Storyline, System,
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
    /// `storylines/`.
    pub storylines: BTreeMap<Id, Doc<Storyline>>,
    /// `missions/`.
    pub missions: BTreeMap<Id, Doc<Mission>>,
    /// `news/`.
    pub news: BTreeMap<Id, Doc<News>>,
    /// `characters/`.
    pub characters: BTreeMap<Id, Doc<Character>>,
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
    /// Files with a valid name that failed to load, e.g.
    /// `systems/marrow.md`. References to them aren't reported again.
    pub unreadable: BTreeSet<String>,
}

impl Galaxy {
    /// Every media file path an asset names (`files`, `sources`,
    /// `licence_file`), each once, sorted.
    #[must_use]
    pub fn media_paths(&self) -> Vec<&str> {
        let paths: BTreeSet<&str> = self
            .assets
            .values()
            .flat_map(|asset| {
                let a = &asset.data;
                a.files.iter().chain(&a.sources).chain(&a.licence_file)
            })
            .map(super::id::MediaPath::as_str)
            .collect();
        paths.into_iter().collect()
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
        storylines: kind::<Storyline>(source, p, u),
        missions: kind::<Mission>(source, p, u),
        news: kind::<News>(source, p, u),
        characters: kind::<Character>(source, p, u),
        ships: kind::<Ship>(source, p, u),
        mods: kind::<ModKit>(source, p, u),
        outfits: kind::<Outfit>(source, p, u),
        liveries: kind::<Livery>(source, p, u),
        assets: kind::<Asset>(source, p, u),
        budgets: kind::<Budget>(source, p, u),
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
    let files = match source.files(dir, "md") {
        Ok(files) => files,
        Err(e) => {
            problems.push(Problem::new(dir, e.to_string()));
            return BTreeMap::new();
        }
    };
    let mut docs = BTreeMap::new();
    for file in files {
        let stem = file
            .path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".md");
        let loaded = Id::try_from(stem.to_string())
            .map_err(|e| format!("the file name {e}"))
            .and_then(|id| frontmatter::parse::<T>(&file.text).map(|(t, body)| (id, t, body)));
        match loaded {
            Ok((id, data, body)) => {
                let body = body.to_string();
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
    use super::*;
    use crate::fixture;

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
