//! The new game's world models at the program's edge ([`world_assets`]):
//! the flight screen's table ([`WorldModels`]) and the meshes and planets
//! its keys name, for the app to register with the renderer
//! ([`Renderer::add_model`](crate::Renderer::add_model),
//! [`Renderer::add_planet`](crate::Renderer::add_planet)) and hand the
//! table to the flight (`nova_view::flight::FlightView::with_world_models`).
//!
//! It is pure and needs no GPU. From a loaded [`NewGame`] (its galaxy, its
//! ID table, its store's mounts and stand-ins):
//!
//! - **Ships**: each ship class with a model ([`ship_request`]) takes the
//!   key of its (asset, length, livery), painted in its livery
//!   ([`ship_livery`], [`painted`]); two classes sharing all three share
//!   a key. It is drawn at [`PX_PER_METRE`]. Its kits as built are its
//!   resolved `mods` (`nova_galaxy_format::fleet::resolve`), root base
//!   first, each that is a kit.
//! - **Outfits**: each outfit model a mount shows, keyed by (asset) at a
//!   metre a unit, unpainted.
//! - **Stations and outposts**: each whose `visual` is a model, keyed as
//!   an outfit is, with the reach of its stand-in outline (the model seen
//!   from above, in its own units), so the view scales it to the
//!   stand-in's size (one whose reach can't size it, the view draws as
//!   its stand-in). One whose stand-in isn't its outline (the model
//!   didn't read for the loader) has none.
//! - **Planets and moons**: each [`GalaxyPlanets`] answers with a planet,
//!   one key each.
//! - **Skies**: each system's own sky look, from its `sky` block, keyed
//!   by its sky's seed ([`sky_looks`]), for the app to hand the renderer
//!   ([`Renderer::set_sky_looks`](crate::Renderer::set_sky_looks)).
//!
//! Each (asset, length) is read once and each livery painted once; keys
//! count up from 1 in a stable order (bible id order: ships, then
//! outfits, then stations; planets on their own count). A model or planet
//! that fails keeps its stand-in, with no key. A failure only this
//! builder finds is a problem, once: over its budget, no depth, no mesh,
//! not a model, a base-colour image that doesn't decode or is larger than
//! the renderer draws (the loader decodes no image, so it can't have
//! reported one); for a planet, a texture that doesn't read or fails its
//! budget or its shape. A model that doesn't read is not: the loader read
//! it first and reported it in [`NewGame::problems`]. Nor is a
//! placeholder, a missing file or a Git LFS pointer: the loader's
//! stand-ins fall back unreported the same way, and the LFS list names the
//! pointers. Stock data is never a new game, so it has none of this.

use std::collections::{BTreeMap, BTreeSet};

use nova_data::new_game::NewGame;
use nova_data::new_game::fleet::PX_PER_METRE;
use nova_data::store::{StandIn, StandInOf};
use nova_data::wire::id::{OutfitId, ShipId, StellarId};
use nova_galaxy_format::fleet::resolve;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::model::{StellarKind, Visual};
use nova_galaxy_format::source::Source;
use nova_view::flight::{ShipModel, StellarModel, WorldModels};
use nova_view::{ModelKey, PlanetKey};

use crate::galaxy_models::{GalaxyModels, ship_livery, ship_request};
use crate::galaxy_planets::GalaxyPlanets;
use crate::livery::{Livery, painted};
use crate::model::Mesh;
use crate::model_source::{LoadedModel, ModelError, ModelRequest, ModelSource};
use crate::new_game_sky::sky_looks;
use crate::planet_source::{LoadedPlanet, PlanetError, PlanetRequest, PlanetSource};
use crate::sky::SkyLooks;

/// The table the flight draws by, and what its keys name (see the module
/// docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldAssets {
    /// The flight's table.
    pub models: WorldModels,
    /// Each model key's mesh, by ascending key.
    pub meshes: Vec<(ModelKey, Mesh)>,
    /// Each planet key's planet, by ascending key.
    pub planets: Vec<(PlanetKey, LoadedPlanet)>,
    /// Each system's own sky look, by its sky's seed ([`sky_looks`]).
    pub skies: SkyLooks,
    /// What failed that is news, each a line ready to report.
    pub problems: Vec<String>,
}

/// The world models of `game`, its files read through `source` (see the
/// module docs).
pub fn world_assets(game: &NewGame, source: &impl Source) -> WorldAssets {
    world_assets_from(
        game,
        &GalaxyModels::new(&game.galaxy, source),
        &GalaxyPlanets::new(&game.galaxy, source),
    )
}

/// [`world_assets`] over the folder `dir` that `game` was opened from,
/// read as the format's directory adapter reads it, so the app names no
/// source.
pub fn world_assets_in(game: &NewGame, dir: &std::path::Path) -> WorldAssets {
    world_assets(game, &nova_galaxy_format::source::DirSource::new(dir))
}

/// [`world_assets`] with its models and planets from `models` and
/// `planets`: the planets baked smaller in tests, say.
pub fn world_assets_from(
    game: &NewGame,
    models: &impl ModelSource,
    planets: &impl PlanetSource,
) -> WorldAssets {
    let mut keys = Keyring::new(models);
    let mut table = WorldModels {
        mounts: game.data.mount_table().clone(),
        ..WorldModels::default()
    };
    for (id, &nova) in &game.ids.ships {
        let ship = ShipId(nova);
        let kits = built_kits(game, id);
        if !kits.is_empty() {
            table.built_kits.insert(ship, kits);
        }
        let Some(request) = ship_request(&game.galaxy, id) else {
            continue;
        };
        if let Some(key) = keys.key(&request, ship_livery(&game.galaxy, id)) {
            #[allow(clippy::cast_possible_truncation, reason = "a metre's pixels")]
            let scale = PX_PER_METRE as f32;
            table.ships.insert(ship, ShipModel { key, scale });
        }
    }
    for asset in shown_assets(&table) {
        if let Some(key) = keys.key(&ModelRequest::new(&asset, None), None) {
            table.outfits.insert(asset, key);
        }
    }
    let mut planet_list = Vec::new();
    let mut problems = Vec::new();
    for (id, &nova) in &game.ids.stellars {
        let stellar = StellarId(nova);
        let doc = &game.galaxy.stellars[id].data;
        if matches!(doc.kind, StellarKind::Station | StellarKind::Outpost) {
            let Some(Visual::Asset(asset)) = &doc.visual else {
                continue;
            };
            let reach = match game.data.stand_in(StandInOf::Stellar(stellar)) {
                // Any reach is handed on, nothing included: the flight
                // view scales the model by the stand-in's radius over it,
                // and draws the stand-in for a scale that isn't finite.
                Some(StandIn::Outline(outline)) => outline.reach(),
                _ => continue,
            };
            let request = ModelRequest::new(asset.as_str(), None);
            if let Some(key) = keys.key(&request, None) {
                #[allow(clippy::cast_possible_truncation, reason = "a model's units")]
                let reach = reach as f32;
                table
                    .stellars
                    .insert(stellar, StellarModel::Model { key, reach });
            }
            continue;
        }
        match planets.planet(&PlanetRequest::new(id.as_str())) {
            Ok(planet) => {
                let key = PlanetKey(next(planet_list.len()));
                table.stellars.insert(stellar, StellarModel::Planet { key });
                planet_list.push((key, planet));
            }
            Err(error) if planet_news(&error) => {
                problems.push(format!("{error}; it draws its disc"));
            }
            Err(_) => {}
        }
    }
    let (meshes, mut model_problems) = keys.finish();
    model_problems.extend(problems);
    WorldAssets {
        models: table,
        meshes,
        planets: planet_list,
        skies: sky_looks(game),
        problems: model_problems,
    }
}

/// The kits ship `id` is built with: its resolved `mods`, root base
/// first, each that is a kit, by its `oütf`.
fn built_kits(game: &NewGame, id: &Id) -> Vec<OutfitId> {
    resolve(&game.galaxy, id).map_or_else(
        |_| Vec::new(),
        |resolved| {
            resolved
                .mods
                .iter()
                .filter_map(|kit| game.ids.kits.get(kit))
                .map(|&nova| OutfitId(nova))
                .collect()
        },
    )
}

/// Every outfit model asset a mount in `table` shows.
fn shown_assets(table: &WorldModels) -> BTreeSet<String> {
    let hulls = table.mounts.hulls.values().flatten();
    let kits = table.mounts.kits.values().flatten();
    hulls
        .chain(kits)
        .filter_map(|mount| mount.shows.as_ref())
        .map(|shown| shown.asset.clone())
        .collect()
}

/// The `n`th key handed out, from 1.
fn next(n: usize) -> u32 {
    u32::try_from(n + 1).expect("a key fits in u32")
}

/// A model as it is keyed: its asset, its length's bits, and its livery's
/// colours.
type ModelId = (String, Option<u32>, Option<[[u8; 3]; 3]>);

/// The models read and the keys handed out so far.
struct Keyring<'a, M: ModelSource> {
    models: &'a M,
    /// Each (asset, length bits) read, once: its model, or none.
    loaded: BTreeMap<(String, Option<u32>), Option<LoadedModel>>,
    /// Each model's key.
    keys: BTreeMap<ModelId, ModelKey>,
    meshes: Vec<(ModelKey, Mesh)>,
    problems: Vec<String>,
}

impl<'a, M: ModelSource> Keyring<'a, M> {
    fn new(models: &'a M) -> Self {
        Keyring {
            models,
            loaded: BTreeMap::new(),
            keys: BTreeMap::new(),
            meshes: Vec::new(),
            problems: Vec::new(),
        }
    }

    /// The key of `request`'s model in `livery`, handing one out with its
    /// mesh the first time; none when the model doesn't load.
    fn key(&mut self, request: &ModelRequest, livery: Option<Livery>) -> Option<ModelKey> {
        let length = request.length.map(f32::to_bits);
        let id: ModelId = (
            request.asset.clone(),
            length,
            livery.map(|l| [l.primary, l.secondary, l.trim]),
        );
        if let Some(&key) = self.keys.get(&id) {
            return Some(key);
        }
        let read = (request.asset.clone(), length);
        if !self.loaded.contains_key(&read) {
            let model = match self.models.model(request) {
                Ok(model) => Some(model),
                Err(error) => {
                    if model_news(&error) {
                        self.problems
                            .push(format!("{error}; what shows it draws its stand-in"));
                    }
                    None
                }
            };
            self.loaded.insert(read.clone(), model);
        }
        let model = self.loaded[&read].as_ref()?;
        let mesh = livery.map_or_else(|| model.mesh(), |livery| painted(model, &livery).mesh());
        let key = ModelKey(next(self.meshes.len()));
        self.keys.insert(id, key);
        self.meshes.push((key, mesh));
        Some(key)
    }

    /// The meshes keyed and the problems found.
    fn finish(self) -> (Vec<(ModelKey, Mesh)>, Vec<String>) {
        (self.meshes, self.problems)
    }
}

/// Whether `error` is news to report (see the module docs). A model that
/// doesn't read the loader reports already (`nova_data::new_game::load`
/// reads every model a ship, a mount or a station shows), so it isn't.
fn model_news(error: &ModelError) -> bool {
    !matches!(
        error,
        ModelError::NoAsset(_)
            | ModelError::NoFile(_)
            | ModelError::LfsPointer(_)
            | ModelError::Missing { .. }
            | ModelError::Unreadable { .. }
    )
}

/// Whether `error` is news to report (see the module docs).
fn planet_news(error: &PlanetError) -> bool {
    !matches!(
        error,
        PlanetError::NoStellar(_)
            | PlanetError::NotAPlanet(_)
            | PlanetError::NoAsset { .. }
            | PlanetError::NoFile { .. }
            | PlanetError::LfsPointer { .. }
            | PlanetError::Missing { .. }
    )
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::path::Path;

    use nova_data::new_game::fixture::{
        LIGHT_FREIGHTER_GLB, STATION_HUB_GLB, TURRET_MOUNT_GLB, modelled_sample, modelled_sample_3d,
    };
    use nova_data::new_game::load;
    use nova_galaxy_format::source::{Listed, MemSource, SourceFile};
    use nova_galaxy_media::planet::Sizes;

    use super::*;

    /// Planets baked small, so the tests stay fast.
    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    /// `source` read as `bytes` counts each path it is asked for.
    struct Counting<'a> {
        source: &'a MemSource,
        reads: RefCell<BTreeMap<String, u32>>,
    }

    impl Source for Counting<'_> {
        fn files(&self, dir: &str, ext: &str) -> io::Result<Vec<SourceFile>> {
            self.source.files(dir, ext)
        }

        fn listing(&self, dir: &str, ext: &str) -> io::Result<Vec<Listed>> {
            self.source.listing(dir, ext)
        }

        fn bytes(&self, path: &str) -> io::Result<Option<Vec<u8>>> {
            *self.reads.borrow_mut().entry(path.to_owned()).or_default() += 1;
            self.source.bytes(path)
        }
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    /// `source` loaded and built, its planets baked small.
    fn built(source: &impl Source) -> (NewGame, WorldAssets) {
        let game = load(source, Path::new("sample"));
        let assets = world_assets_from(
            &game,
            &GalaxyModels::new(&game.galaxy, source),
            &GalaxyPlanets::new(&game.galaxy, source).baked_at(SMALL),
        );
        (game, assets)
    }

    /// The 3D sample with a plain freighter that shares the light
    /// freighter's model, length and lack of livery.
    fn with_a_plain_freighter() -> MemSource {
        modelled_sample_3d().with(
            "ships/plain-freighter.md",
            "---\nname: Plain freighter\nbase: light-freighter\n---\n",
        )
    }

    fn ship(game: &NewGame, name: &str) -> ShipId {
        ShipId(game.ids.ships[&id(name)])
    }

    fn stellar(game: &NewGame, name: &str) -> StellarId {
        StellarId(game.ids.stellars[&id(name)])
    }

    fn mesh(assets: &WorldAssets, key: ModelKey) -> &Mesh {
        &assets
            .meshes
            .iter()
            .find(|(k, _)| *k == key)
            .expect("a mesh for the key")
            .1
    }

    #[test]
    fn classes_sharing_a_model_length_and_livery_share_a_key_and_a_livery_paints_its_own() {
        let source = with_a_plain_freighter();
        let (game, assets) = built(&source);
        let key = |name| assets.models.ship(ship(&game, name)).expect("a model").key;
        assert_eq!(key("light-freighter"), key("plain-freighter"));
        assert_ne!(key("light-freighter"), key("harrow-gunship"));
        let models = GalaxyModels::new(&game.galaxy, &source);
        for name in ["light-freighter", "harrow-gunship"] {
            let request = ship_request(&game.galaxy, &id(name)).expect("a request");
            let loaded = models.model(&request).expect("it loads");
            let want = ship_livery(&game.galaxy, &id(name))
                .map_or_else(|| loaded.mesh(), |livery| painted(&loaded, &livery).mesh());
            assert_eq!(mesh(&assets, key(name)), &want, "{name}");
        }
        let scale = assets
            .models
            .ship(ship(&game, "harrow-gunship"))
            .expect("a model")
            .scale;
        #[allow(clippy::cast_possible_truncation)]
        let px = PX_PER_METRE as f32;
        assert!((scale - px).abs() < f32::EPSILON);
    }

    #[test]
    fn each_model_file_is_read_once() {
        let memory = with_a_plain_freighter();
        let game = load(&memory, Path::new("sample"));
        let counting = Counting {
            source: &memory,
            reads: RefCell::default(),
        };
        let assets = world_assets_from(
            &game,
            &GalaxyModels::new(&game.galaxy, &counting),
            &GalaxyPlanets::new(&game.galaxy, &counting).baked_at(SMALL),
        );
        assert!(!assets.meshes.is_empty());
        let reads = counting.reads.borrow();
        for file in [LIGHT_FREIGHTER_GLB, TURRET_MOUNT_GLB, STATION_HUB_GLB] {
            assert_eq!(reads.get(file), Some(&1), "{file}: {reads:?}");
        }
    }

    #[test]
    fn the_turret_and_both_stations_have_keys_each_station_its_stand_ins_reach() {
        let (game, assets) = built(&modelled_sample_3d());
        let turret = assets
            .models
            .outfit("turret-mount")
            .expect("the turret's key");
        let mut hubs = Vec::new();
        for name in ["luyten-exchange", "marrow-yard"] {
            let at = stellar(&game, name);
            let Some(StellarModel::Model { key, reach }) = assets.models.stellar(at) else {
                panic!("{name}: a model");
            };
            let Some(StandIn::Outline(outline)) = game.data.stand_in(StandInOf::Stellar(at)) else {
                panic!("{name}: an outline stand-in");
            };
            #[allow(clippy::cast_possible_truncation)]
            let want = outline.reach() as f32;
            assert!((reach - want).abs() < 1e-6, "{name}: {reach} not {want}");
            assert!(reach > 0.0);
            hubs.push(key);
        }
        assert_eq!(hubs[0], hubs[1], "one hub, one key");
        assert_ne!(hubs[0], turret);
        assert_eq!(assets.problems, Vec::<String>::new());
    }

    #[test]
    fn a_generated_stellar_gets_a_planet_and_one_with_no_visual_none() {
        let source = modelled_sample_3d().with(
            "stellars/pebble.md",
            "---\nname: Pebble\nkind: moon\nsystem: luyten-crossing\nposition: [10, 10]\n---\n",
        );
        let (game, assets) = built(&source);
        assert_eq!(
            assets.models.stellar(stellar(&game, "kestrel-reach")),
            Some(StellarModel::Planet { key: PlanetKey(1) })
        );
        assert_eq!(assets.planets.len(), 1);
        assert_eq!(assets.planets[0].0, PlanetKey(1));
        assert_eq!(assets.models.stellar(stellar(&game, "pebble")), None);
        assert_eq!(assets.problems, Vec::<String>::new());
        // The 2D sample's stations have no visual: nothing at all.
        let (plain, flat) = built(&modelled_sample());
        assert!(
            flat.models.stellars.is_empty(),
            "{:?}",
            flat.models.stellars
        );
        assert!(flat.planets.is_empty());
        assert!(plain.ids.stellars.len() >= 2);
    }

    #[test]
    fn an_over_budget_model_gets_no_key_and_one_problem() {
        let source = modelled_sample_3d().with(
            "budgets/default.md",
            "---\nmodel: {triangles: 11, kib: 4096}\n---\n",
        );
        let (_, assets) = built(&source);
        assert!(assets.models.ships.is_empty());
        assert!(assets.models.outfits.is_empty());
        assert!(
            !assets
                .models
                .stellars
                .values()
                .any(|model| matches!(model, StellarModel::Model { .. }))
        );
        assert!(assets.meshes.is_empty());
        // The freighter (its classes share one read), the turret, the hub.
        assert_eq!(assets.problems.len(), 3, "{:?}", assets.problems);
        assert!(
            assets
                .problems
                .iter()
                .all(|p| p.contains("over its art budget")),
            "{:?}",
            assets.problems
        );
    }

    #[test]
    fn a_model_refused_for_its_texture_keeps_its_stand_in_and_is_reported_once() {
        use crate::model_source::{FakeModels, ImageRef, Measure, Overrun};
        // Two classes share the light freighter's model.
        let source = with_a_plain_freighter();
        let game = load(&source, Path::new("sample"));
        let refusals = [
            ModelError::TextureOverBudget {
                asset: "light-freighter".into(),
                image: ImageRef::Embedded(1),
                over: vec![Overrun {
                    measure: Measure::Side,
                    used: 4_096,
                    most: 2_048,
                    budget: "budgets/default.md".into(),
                }],
            },
            ModelError::TextureUndecodable {
                asset: "light-freighter".into(),
                image: ImageRef::Embedded(0),
                why: "it is neither a PNG nor a JPEG".into(),
            },
            ModelError::TextureTooLarge {
                asset: "light-freighter".into(),
                image: ImageRef::File("media/export/skin.png".into()),
                side: 16_384,
                most: 8_192,
            },
        ];
        for refusal in refusals {
            let said = refusal.to_string();
            let models = FakeModels::default().with_error("light-freighter", refusal);
            let planets = GalaxyPlanets::new(&game.galaxy, &source).baked_at(SMALL);
            let assets = world_assets_from(&game, &models, &planets);
            for name in ["light-freighter", "plain-freighter"] {
                assert_eq!(assets.models.ship(ship(&game, name)), None, "{name}");
            }
            assert_eq!(
                assets.problems,
                [format!("{said}; what shows it draws its stand-in")]
            );
        }
    }

    /// The 3D sample with a ship on each kind of model that doesn't load.
    /// A folder of this test's own under the system's temporary folder,
    /// removed when dropped.
    struct Folder(std::path::PathBuf);

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Copies every file under `from` into `to`.
    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).expect("a folder");
        for entry in std::fs::read_dir(from).expect("lists") {
            let path = entry.expect("an entry").path();
            let into = to.join(path.file_name().expect("a name"));
            if path.is_dir() {
                copy_tree(&path, &into);
            } else {
                std::fs::copy(&path, &into).expect("copies");
            }
        }
    }

    #[test]
    fn the_world_of_a_folder_on_disk_is_read_from_its_files() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("after 1970")
            .as_nanos();
        let folder = Folder(
            std::env::temp_dir().join(format!("nova-render-world-{}-{nanos}", std::process::id())),
        );
        copy_tree(&nova_data::new_game::fixture::sample_folder(), &folder.0);
        std::fs::write(
            folder.0.join("stellars").join("kestrel-reach.md"),
            "---\nname: Kestrel Reach\nkind: planet\nsystem: luyten-crossing\n\
             position: [700, -500]\nvisual:\n  generated:\n    type: gas-giant\n\
             \x20   palette: [\"#c9a36b\"]\n    seed: 4417\n---\n",
        )
        .expect("writes");
        let game = nova_data::new_game::open(&folder.0).expect("lists");
        let assets = world_assets_in(&game, &folder.0);
        assert_eq!(
            assets.models.stellar(stellar(&game, "kestrel-reach")),
            Some(StellarModel::Planet { key: PlanetKey(1) })
        );
        assert_eq!(assets.planets.len(), 1);
    }

    fn with_broken_models() -> MemSource {
        let ship = |name: &str, asset: &str| {
            format!(
                "---\nname: {name}\nmodel: {asset}\n\
                 size: {{length: 30, beam: 20, collision_radius: 15}}\n\
                 stats: {{hull: 100, shields: 0, speed: 200, handling: 30, cargo: 10}}\n---\n"
            )
        };
        let asset = |file: &str| {
            format!(
                "---\nkind: model\nplaceholder: false\nfront: \"-z\"\nfiles: [{file}]\n\
                 licence: CC0-1.0\nauthor: test\n---\n"
            )
        };
        modelled_sample_3d()
            .with("ships/broken.md", &ship("Broken", "broken"))
            .with("assets/broken.md", &asset("media/export/broken.glb"))
            .with_bytes("media/export/broken.glb", b"glTF nonsense")
            .with("ships/ghost.md", &ship("Ghost", "ghost"))
            .with(
                "assets/ghost.md",
                "---\nkind: model\nplaceholder: true\n---\n",
            )
            .with("ships/lost.md", &ship("Lost", "lost"))
            .with("assets/lost.md", &asset("media/export/lost.glb"))
            .with("ships/pointer.md", &ship("Pointer", "pointer"))
            .with("assets/pointer.md", &asset("media/export/pointer.glb"))
            .with_bytes(
                "media/export/pointer.glb",
                b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n",
            )
    }

    #[test]
    fn an_unreadable_placeholder_missing_or_pointer_model_gets_no_key_and_is_no_world_problem() {
        let (game, assets) = built(&with_broken_models());
        for name in ["broken", "ghost", "lost", "pointer"] {
            assert_eq!(assets.models.ship(ship(&game, name)), None, "{name}");
        }
        // The loader reports the unreadable one.
        assert_eq!(assets.problems, Vec::<String>::new());
        assert!(assets.models.ship(ship(&game, "light-freighter")).is_some());
    }

    #[test]
    fn the_loader_s_and_the_world_s_problems_together_name_each_broken_model_once() {
        let source = with_broken_models()
            .with_bytes(TURRET_MOUNT_GLB, b"glTF nonsense")
            .with_bytes(STATION_HUB_GLB, b"glTF nonsense");
        let (game, assets) = built(&source);
        let lines: Vec<String> = game
            .problems
            .iter()
            .map(ToString::to_string)
            .chain(assets.problems.iter().cloned())
            .collect();
        for file in ["media/export/broken.glb", TURRET_MOUNT_GLB, STATION_HUB_GLB] {
            let naming = lines.iter().filter(|line| line.contains(file)).count();
            assert_eq!(naming, 1, "{file}: {lines:?}");
        }
    }

    #[test]
    fn a_planet_that_fails_with_news_is_one_problem_and_otherwise_none() {
        let source = modelled_sample_3d();
        let game = load(&source, Path::new("sample"));
        let named = || ("kestrel-reach".to_owned(), "surface".to_owned());
        let news = [
            PlanetError::NotATexture {
                stellar: named().0,
                asset: named().1,
            },
            PlanetError::Unreadable {
                stellar: named().0,
                asset: named().1,
                why: "bad".into(),
            },
            PlanetError::OverBudget {
                stellar: named().0,
                asset: named().1,
                over: Vec::new(),
            },
        ];
        let quiet = [
            PlanetError::NoStellar(named().0),
            PlanetError::NotAPlanet(named().0),
            PlanetError::NoAsset {
                stellar: named().0,
                asset: named().1,
            },
            PlanetError::NoFile {
                stellar: named().0,
                asset: named().1,
            },
            PlanetError::LfsPointer {
                stellar: named().0,
                asset: named().1,
            },
            PlanetError::Missing {
                stellar: named().0,
                asset: named().1,
                file: "f.png".into(),
            },
        ];
        let failing = |error: PlanetError| {
            let planets =
                crate::planet_source::FakePlanets::default().with_error("kestrel-reach", error);
            world_assets_from(&game, &GalaxyModels::new(&game.galaxy, &source), &planets)
        };
        for error in news {
            let said = error.to_string();
            let assets = failing(error);
            assert_eq!(assets.problems, [format!("{said}; it draws its disc")]);
            assert!(assets.planets.is_empty());
        }
        for error in quiet {
            let said = error.to_string();
            let assets = failing(error);
            assert_eq!(assets.problems, Vec::<String>::new(), "{said}");
            assert!(assets.planets.is_empty());
        }
    }

    #[test]
    fn a_station_whose_model_did_not_read_for_its_stand_in_has_none() {
        let source = modelled_sample_3d().with_bytes(STATION_HUB_GLB, b"glTF nonsense");
        let (game, assets) = built(&source);
        for name in ["luyten-exchange", "marrow-yard"] {
            assert_eq!(assets.models.stellar(stellar(&game, name)), None, "{name}");
        }
        assert!(
            assets.meshes.len() >= 2,
            "the ships' and the turret's still"
        );
    }

    #[test]
    fn no_two_entries_share_a_key() {
        let (_, assets) = built(&with_a_plain_freighter());
        let keys: Vec<u32> = assets.meshes.iter().map(|(key, _)| key.0).collect();
        let counted: Vec<u32> = (1..=u32::try_from(keys.len()).expect("few")).collect();
        assert_eq!(keys, counted, "one each, from 1 up");
        let planets: Vec<u32> = assets.planets.iter().map(|(key, _)| key.0).collect();
        assert_eq!(planets, [1]);
        // Light freighter (shared with the plain one), the gunship's
        // paint, the turret and the hub.
        assert_eq!(keys.len(), 4);
    }

    #[test]
    fn the_mounts_and_kits_as_built_answer_as_the_store_does() {
        let (game, assets) = built(&modelled_sample_3d());
        let gunship = ship(&game, "harrow-gunship");
        let gun_mount = OutfitId(game.ids.kits[&id("gun-mount")]);
        assert_eq!(assets.models.built_kits(gunship), [gun_mount]);
        assert_eq!(assets.models.built_kits(ship(&game, "light-freighter")), []);
        let mounts = assets
            .models
            .mounts(gunship, assets.models.built_kits(gunship));
        assert_eq!(mounts, game.data.mounts(gunship, &[gun_mount]));
        let bed = mounts
            .iter()
            .find(|mount| mount.hardpoint == "gun-bed")
            .expect("the gun bed");
        assert_eq!(
            bed.shows.as_ref().map(|shown| shown.asset.as_str()),
            Some("turret-mount")
        );
    }
}
