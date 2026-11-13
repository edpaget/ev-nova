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
//!   a key. It is drawn at [`PX_PER_METRE`], as see-through as its
//!   resolved `translucency` (its `BaseTransp`, 0 when none gives one).
//!   Its kits as built are its resolved `mods`
//!   (`nova_galaxy_format::fleet::resolve`), root base first, each that
//!   is a kit. Its radius ([`ShipModel::radius`]) is the furthest its hull
//!   mesh or a part its own and as-built mounts show reaches, measured
//!   once the outfits are keyed.
//! - **Outfits**: each outfit model a mount shows, keyed by (asset) at a
//!   metre a unit, unpainted.
//! - **Stations and outposts**: each whose `visual` is a model, keyed by
//!   (asset, livery) at a metre a unit, painted in its owner's livery
//!   ([`station_livery`]): two stations of one owner share a key, and an
//!   unowned one, or one whose owner no livery names, is unpainted. The
//!   owner is the data's, painted once here: the new game's data has no
//!   way to change a stellar's owner in play. Each carries the reach of
//!   its stand-in outline (the model seen from above, in its own units),
//!   so the view scales it to the stand-in's size (one whose reach can't
//!   size it, the view draws as its stand-in). One whose stand-in isn't
//!   its outline (the model didn't read for the loader) has none.
//! - **Planets and moons**: each [`GalaxyPlanets`] answers with a planet,
//!   one key each, with how far its rings and atmosphere reach in its
//!   radii ([`LoadedPlanet::reach`]), so a picture can fit the whole of
//!   it. A generated planet is keyed here and handed on unbaked: baking
//!   every look at start-up is what made start-up slow, so the renderer
//!   bakes each the first frame that draws it
//!   ([`Renderer::add_planet`](crate::Renderer::add_planet)). A textured
//!   planet is read and decoded here, where a failure can be reported.
//! - **Skies**: each system's own sky look, from its `sky` block, keyed
//!   by its sky's seed ([`sky_looks`]), for the app to hand the renderer
//!   ([`Renderer::set_sky_looks`](crate::Renderer::set_sky_looks)).
//!
//! Each model file is read once, by the loader: [`world_assets`] builds
//! from the scenes it kept ([`NewGame::scenes`], through
//! [`GalaxyModels::reading`]), and every model this builder asks for is
//! one the loader read (a ship's by its mounts, whether or not it has a
//! size; an outfit's a mount shows; a station's for its stand-in), so it
//! reads no model file itself. Each (asset, length) is asked of the model
//! source once and painted once in each livery it
//! wears. A livery on a model with no part in a painted material paints
//! nothing, so that model keeps its one unpainted key and mesh; keys
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

use crate::galaxy_models::{GalaxyModels, ship_livery, ship_request, station_livery};
use crate::galaxy_planets::GalaxyPlanets;
use crate::livery::{Livery, painted, takes_paint};
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

/// The world models of `game` (see the module docs): its models from the
/// scenes the loader read ([`NewGame::scenes`]), so no model file is read
/// again, and its planets' textures read through `source`.
pub fn world_assets(game: &NewGame, source: &impl Source) -> WorldAssets {
    world_assets_from(
        game,
        &GalaxyModels::new(&game.galaxy, source).reading(&game.scenes),
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
            let model = ShipModel {
                key,
                scale,
                base_transp: translucency(game, id),
                radius: 0.0,
            };
            table.ships.insert(ship, model);
        }
    }
    for asset in shown_assets(&table) {
        if let Some(key) = keys.key(&ModelRequest::new(&asset, None), None) {
            table.outfits.insert(asset, key);
        }
    }
    let radii: Vec<(ShipId, f32)> = table
        .ships
        .iter()
        .map(|(&ship, model)| (ship, ship_radius(&table, &keys, ship, model.key)))
        .collect();
    for (ship, radius) in radii {
        if let Some(model) = table.ships.get_mut(&ship) {
            model.radius = radius;
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
            if let Some(key) = keys.key(&request, station_livery(&game.galaxy, id)) {
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
                let reach = planet.reach();
                table
                    .stellars
                    .insert(stellar, StellarModel::planet(key, reach));
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

/// How see-through ship `id`'s hull is, in 32nds as a sheet's
/// `BaseTransp`: its resolved `translucency`, 0 when none in its chain
/// gives one or it doesn't resolve.
fn translucency(game: &NewGame, id: &Id) -> i16 {
    resolve(&game.galaxy, id).map_or(0, |resolved| i16::from(resolved.translucency))
}

/// How far ship class `ship`'s hull `hull` and the parts its own and
/// as-built kits' mounts show reach from its origin, in model units
/// ([`ShipModel::radius`]): the hull mesh's radius, or a part's further,
/// each the length of its mount's point raised by its lift plus its own
/// mesh's radius. A part with no key isn't drawn, so it reaches nowhere.
fn ship_radius<M: ModelSource>(
    table: &WorldModels,
    keys: &Keyring<'_, M>,
    ship: ShipId,
    hull: ModelKey,
) -> f32 {
    let hull_radius = keys.radius(hull).unwrap_or(0.0);
    table
        .mounts(ship, table.built_kits(ship))
        .iter()
        .filter_map(|mount| {
            let shown = mount.shows.as_ref()?;
            let part = keys.radius(table.outfit(&shown.asset)?)?;
            Some(part_reach(mount.at, shown.lift, part))
        })
        .fold(hull_radius, f32::max)
}

/// How far a part of radius `part` reaches from its hull's origin, on a
/// mount at `at` raised by `lift`: the lifted point's distance, plus the
/// part's own radius.
fn part_reach([x, y, z]: [f64; 3], lift: f64, part: f32) -> f32 {
    let lifted = z + lift;
    #[allow(clippy::cast_possible_truncation, reason = "a mount's metres")]
    let reach = (x * x + y * y + lifted * lifted).sqrt() as f32;
    reach + part
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
    /// mesh the first time; none when the model doesn't load. A livery on
    /// a model that takes no paint ([`takes_paint`], the livery module's
    /// rule) paints nothing, so it takes the unpainted key, and no copy of
    /// its mesh is made.
    fn key(&mut self, request: &ModelRequest, livery: Option<Livery>) -> Option<ModelKey> {
        let length = request.length.map(f32::to_bits);
        let read = (request.asset.clone(), length);
        self.load(request, &read);
        let model = self.loaded[&read].as_ref()?;
        let livery = livery.filter(|_| takes_paint(model));
        let id: ModelId = (
            request.asset.clone(),
            length,
            livery.map(|l| [l.primary, l.secondary, l.trim]),
        );
        if let Some(&key) = self.keys.get(&id) {
            return Some(key);
        }
        let mesh = livery.map_or_else(|| model.mesh(), |livery| painted(model, &livery).mesh());
        let key = ModelKey(next(self.meshes.len()));
        self.keys.insert(id, key);
        self.meshes.push((key, mesh));
        Some(key)
    }

    /// Reads the model `request` names into [`Self::loaded`] under `read`
    /// (its asset and length bits) the first time it is asked for, or
    /// none when it doesn't load, reporting a failure that is news.
    fn load(&mut self, request: &ModelRequest, read: &(String, Option<u32>)) {
        if !self.loaded.contains_key(read) {
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
    }

    /// The radius of the mesh `key` names, if it was handed out.
    fn radius(&self, key: ModelKey) -> Option<f32> {
        self.meshes
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, mesh)| mesh.radius())
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
    use std::path::Path;

    use nova_data::new_game::fixture::{
        LIGHT_FREIGHTER_GLB, STATION_HUB_GLB, TURRET_MOUNT_GLB, liveried_stations_3d,
        modelled_sample, modelled_sample_3d,
    };
    use nova_data::new_game::load;
    use nova_galaxy_format::source::{CountingSource, MemSource};
    use nova_galaxy_media::planet::Sizes;

    use super::*;

    /// Planets baked small, so the tests stay fast.
    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    /// `source` loaded and built as [`world_assets`] builds it, from the
    /// loader's scenes, but with its planets baked small.
    fn built(source: &impl Source) -> (NewGame, WorldAssets) {
        let game = load(source, Path::new("sample"));
        let assets = world_assets_from(
            &game,
            &GalaxyModels::new(&game.galaxy, source).reading(&game.scenes),
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
    fn a_class_s_model_carries_its_chain_s_translucency_as_its_base_transp() {
        let source = modelled_sample_3d()
            .with(
                "ships/ghost-freighter.md",
                "---\nname: Ghost freighter\nbase: light-freighter\ntranslucency: 10\n---\n",
            )
            .with(
                "ships/pale-freighter.md",
                "---\nname: Pale freighter\nbase: ghost-freighter\n---\n",
            );
        let (game, assets) = built(&source);
        let base_transp = |name| {
            let model = assets.models.ship(ship(&game, name)).expect("a model");
            model.base_transp
        };
        assert_eq!(base_transp("ghost-freighter"), 10);
        assert_eq!(base_transp("pale-freighter"), 10, "inherited");
        assert_eq!(base_transp("light-freighter"), 0);
    }

    #[test]
    fn each_model_file_is_read_once_by_the_loader_and_the_world_builder_together() {
        // A third class on the freighter's model, at another length: the
        // builder makes its mesh twice from the one scene the loader read.
        let memory = with_a_plain_freighter().with(
            "ships/long-freighter.md",
            "---\nname: Long freighter\nbase: light-freighter\n\
             size: {length: 80, beam: 26, collision_radius: 20}\n---\n",
        );
        let counting = CountingSource::new(&memory);
        let game = load(&counting, Path::new("sample"));
        let assets = world_assets(&game, &counting);
        assert_eq!(assets.problems, Vec::<String>::new());
        let key = |name| assets.models.ship(ship(&game, name)).expect("a model").key;
        assert_ne!(key("light-freighter"), key("long-freighter"));
        assert!(assets.models.outfit("turret-mount").is_some());
        assert!(matches!(
            assets.models.stellar(stellar(&game, "luyten-exchange")),
            Some(StellarModel::Model { .. })
        ));
        for file in [LIGHT_FREIGHTER_GLB, TURRET_MOUNT_GLB, STATION_HUB_GLB] {
            assert_eq!(
                counting.bytes_asked(file),
                1,
                "{file}: {:?}",
                counting.all_bytes_asked()
            );
        }
    }

    #[test]
    fn a_ship_with_a_model_and_no_size_is_read_by_the_loader_and_the_builder_reads_nothing() {
        // With no size it has no stand-in, but the loader's mounts read
        // its model all the same, so the builder has its scene.
        const SKIFF_GLB: &str = "media/export/pack/skiff.glb";
        let memory = modelled_sample_3d()
            .with(
                "ships/skiff.md",
                "---\nname: Skiff\nmodel: skiff-hull\n---\n",
            )
            .with(
                "assets/skiff-hull.md",
                &format!(
                    "---\nkind: model\nplaceholder: false\nfront: \"-z\"\nfiles: [{SKIFF_GLB}]\n\
                     licence: CC0-1.0\nauthor: test\n---\n"
                ),
            )
            .with_bytes(
                SKIFF_GLB,
                &nova_galaxy_media::fixture::glb([1.0, 1.0, 2.0], [0.0; 3]),
            );
        let loading = CountingSource::new(&memory);
        let game = load(&loading, Path::new("sample"));
        assert_eq!(
            game.data.stand_in(StandInOf::Ship(ship(&game, "skiff"))),
            None
        );
        assert_eq!(loading.bytes_asked(SKIFF_GLB), 1, "by the loader's mounts");
        let building = CountingSource::new(&memory);
        let assets = world_assets(&game, &building);
        assert!(assets.models.ship(ship(&game, "skiff")).is_some());
        assert_eq!(assets.problems, Vec::<String>::new());
        assert_eq!(building.all_bytes_asked(), [], "the builder reads nothing");
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

    /// A station in Luyten Crossing named `name`, owned by `owner` (or by
    /// nobody), drawing the hub.
    fn hub_station(name: &str, owner: Option<&str>) -> String {
        let owner = owner.map_or(String::new(), |owner| format!("owner: {owner}\n"));
        format!(
            "---\nname: {name}\nkind: station\nsystem: luyten-crossing\nposition: [300, 300]\n\
             {owner}visual: {{asset: station-hub}}\n---\n"
        )
    }

    /// The model key `name`'s table entry names.
    fn station_key(game: &NewGame, assets: &WorldAssets, name: &str) -> ModelKey {
        match assets.models.stellar(stellar(game, name)) {
            Some(StellarModel::Model { key, .. }) => key,
            other => panic!("{name}: {other:?}"),
        }
    }

    /// The hub as its model source loads it, and as the commonwealth's
    /// livery paints it, in `game`.
    fn hubs(game: &NewGame, source: &impl Source) -> (Mesh, Mesh) {
        let models = GalaxyModels::new(&game.galaxy, source);
        let hub = models
            .model(&ModelRequest::new("station-hub", None))
            .expect("the hub loads");
        let livery = station_livery(&game.galaxy, &id("luyten-exchange")).expect("a livery");
        (hub.mesh(), painted(&hub, &livery).mesh())
    }

    #[test]
    fn a_station_is_painted_in_its_owner_s_livery_and_one_whose_owner_has_none_is_not() {
        let source = liveried_stations_3d();
        let (game, assets) = built(&source);
        let (plain, painted) = hubs(&game, &source);
        assert_ne!(plain, painted, "the hub takes paint");
        let exchange = station_key(&game, &assets, "luyten-exchange");
        let yard = station_key(&game, &assets, "marrow-yard");
        assert_eq!(mesh(&assets, exchange), &painted);
        assert_eq!(mesh(&assets, yard), &plain, "the commons wear no livery");
        assert_ne!(exchange, yard);
        assert_eq!(assets.problems, Vec::<String>::new());
    }

    #[test]
    fn stations_of_one_livery_share_one_painted_mesh_and_an_unowned_one_is_unpainted() {
        let source = liveried_stations_3d()
            .with(
                "stellars/second-exchange.md",
                &hub_station("Second Exchange", Some("commonwealth")),
            )
            .with(
                "stellars/drifting-hub.md",
                &hub_station("Drifting Hub", None),
            );
        let (game, assets) = built(&source);
        let (plain, painted) = hubs(&game, &source);
        let exchange = station_key(&game, &assets, "luyten-exchange");
        assert_eq!(station_key(&game, &assets, "second-exchange"), exchange);
        let drifting = station_key(&game, &assets, "drifting-hub");
        assert_eq!(drifting, station_key(&game, &assets, "marrow-yard"));
        assert_eq!(mesh(&assets, drifting), &plain);
        let painted_hubs = assets.meshes.iter().filter(|(_, m)| *m == painted).count();
        assert_eq!(painted_hubs, 1, "one painted hub");
        assert_eq!(assets.problems, Vec::<String>::new());
    }

    #[test]
    fn a_livery_on_a_model_with_nothing_to_paint_keys_the_unpainted_mesh() {
        // The commonwealth's livery, on the plain sample's unpainted hub.
        let source = modelled_sample_3d().with(
            "liveries/commonwealth-colours.md",
            "---\nname: Commonwealth colours\nfaction: commonwealth\n\
             palette: {primary: \"#ff8c00\", secondary: \"#718cff\", trim: \"#00ffff\"}\n---\n",
        );
        let (game, assets) = built(&source);
        assert!(station_livery(&game.galaxy, &id("luyten-exchange")).is_some());
        let exchange = station_key(&game, &assets, "luyten-exchange");
        assert_eq!(exchange, station_key(&game, &assets, "marrow-yard"));
        let (_, plain_assets) = built(&modelled_sample_3d());
        assert_eq!(
            assets.meshes.len(),
            plain_assets.meshes.len(),
            "no extra mesh"
        );
        assert_eq!(assets.problems, Vec::<String>::new());
    }

    #[test]
    fn a_liveried_ship_on_a_model_with_nothing_to_paint_shares_the_unpainted_key() {
        // The light freighter's asset left without `paint`: the gunship's
        // livery paints nothing, so it shares the freighter's mesh.
        let source = modelled_sample_3d().with(
            "assets/light-freighter.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfront: \"-z\"\n\
                 files: [{LIGHT_FREIGHTER_GLB}]\nlicence: CC0-1.0\nauthor: test\n---\n"
            ),
        );
        let (game, assets) = built(&source);
        let key = |name| assets.models.ship(ship(&game, name)).expect("a model").key;
        assert_eq!(key("light-freighter"), key("harrow-gunship"));
        assert_eq!(assets.problems, Vec::<String>::new());
    }

    #[test]
    fn the_hub_is_read_once_for_both_its_liveries() {
        let memory = liveried_stations_3d();
        let counting = CountingSource::new(&memory);
        let (game, assets) = built(&counting);
        assert_ne!(
            station_key(&game, &assets, "luyten-exchange"),
            station_key(&game, &assets, "marrow-yard")
        );
        assert_eq!(counting.bytes_asked(STATION_HUB_GLB), 1);
    }

    #[test]
    fn a_builder_given_no_scenes_reads_each_model_once_for_all_its_lengths_and_liveries() {
        // The phase-9 rule, for a model source that reads its files: one
        // read per (asset, length), however many liveries it wears.
        let memory = liveried_stations_3d();
        let game = load(&memory, Path::new("sample"));
        let counting = CountingSource::new(&memory);
        let assets = world_assets_from(
            &game,
            &GalaxyModels::new(&game.galaxy, &counting),
            &GalaxyPlanets::new(&game.galaxy, &counting).baked_at(SMALL),
        );
        assert_ne!(
            station_key(&game, &assets, "luyten-exchange"),
            station_key(&game, &assets, "marrow-yard")
        );
        for file in [LIGHT_FREIGHTER_GLB, TURRET_MOUNT_GLB, STATION_HUB_GLB] {
            assert_eq!(counting.bytes_asked(file), 1, "{file}");
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_generated_planet_is_keyed_and_handed_on_unbaked_at_the_engine_s_sizes() {
        let source = modelled_sample_3d();
        let game = load(&source, Path::new("sample"));
        let assets = world_assets(&game, &source);
        assert_eq!(assets.problems, Vec::<String>::new());
        let Some(Visual::Generated(look)) = &game.galaxy.stellars[&id("kestrel-reach")].data.visual
        else {
            panic!("Kestrel Reach is generated");
        };
        assert_eq!(
            assets.planets,
            [(
                PlanetKey(1),
                LoadedPlanet::unbaked(look.clone(), Sizes::ENGINE)
            )]
        );
        assert_eq!(assets.planets[0].1.baked(), None, "nothing baked here");
        // Its reach is its look's: rings to 2.3.
        assert_eq!(
            assets.models.stellar(stellar(&game, "kestrel-reach")),
            Some(StellarModel::planet(PlanetKey(1), 2.3))
        );
    }

    /// The 3D sample with Pale, a moon in the start system on the texture
    /// `pale-surface`, its file holding `png`.
    fn with_a_textured_moon(png: &[u8]) -> MemSource {
        modelled_sample_3d()
            .with(
                "stellars/pale.md",
                "---\nname: Pale\nkind: moon\nsystem: luyten-crossing\nposition: [30, 30]\n\
                 visual: {asset: pale-surface}\n---\n",
            )
            .with(
                "assets/pale-surface.md",
                "---\nkind: texture\nplaceholder: false\nfiles: [media/export/pack/pale.png]\n\
                 licence: CC0-1.0\nauthor: test\n---\n",
            )
            .with_bytes("media/export/pack/pale.png", png)
    }

    #[test]
    fn a_textured_planet_is_still_read_at_load_and_one_that_fails_is_a_problem() {
        let source = with_a_textured_moon(&nova_galaxy_media::fixture::png(4, 2, &[50; 32]));
        let game = load(&source, Path::new("sample"));
        let assets = world_assets(&game, &source);
        assert_eq!(assets.problems, Vec::<String>::new());
        let Some(StellarModel::Planet { key, .. }) = assets.models.stellar(stellar(&game, "pale"))
        else {
            panic!("Pale's planet");
        };
        let pale = &assets
            .planets
            .iter()
            .find(|(k, _)| *k == key)
            .expect("it")
            .1;
        let surface = pale
            .baked()
            .expect("read here, not left for the first draw");
        assert_eq!(surface.surface.rgba, [50, 50, 50, 0].repeat(8));
        assert_eq!(assets.planets.len(), 2, "and Kestrel Reach, unbaked");

        let broken = with_a_textured_moon(b"not a png");
        let game = load(&broken, Path::new("sample"));
        let assets = world_assets(&game, &broken);
        assert_eq!(assets.models.stellar(stellar(&game, "pale")), None);
        assert_eq!(assets.planets.len(), 1, "Kestrel Reach alone");
        let [problem] = &assets.problems[..] else {
            panic!("one problem: {:?}", assets.problems);
        };
        assert!(
            problem.starts_with("stellar `pale`'s texture `pale-surface` doesn't read")
                && problem.ends_with("; it draws its disc"),
            "{problem}"
        );
    }

    #[test]
    fn a_generated_stellar_gets_a_planet_and_one_with_no_visual_none() {
        let source = modelled_sample_3d().with(
            "stellars/pebble.md",
            "---\nname: Pebble\nkind: moon\nsystem: luyten-crossing\nposition: [10, 10]\n---\n",
        );
        let (game, assets) = built(&source);
        assert!(matches!(
            assets.models.stellar(stellar(&game, "kestrel-reach")),
            Some(StellarModel::Planet {
                key: PlanetKey(1),
                ..
            })
        ));
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

    /// Kestrel Reach generated with `extra`, front-matter lines of its
    /// `generated` block, each indented four spaces.
    fn kestrel_reach_with(extra: &str) -> MemSource {
        modelled_sample_3d().with(
            "stellars/kestrel-reach.md",
            &format!(
                "---\nname: Kestrel Reach\nkind: planet\nsystem: luyten-crossing\n\
                 position: [700, -500]\nvisual:\n  generated:\n    type: gas-giant\n\
                 \x20   palette: [\"#c9a36b\"]\n{extra}    seed: 4417\n---\n"
            ),
        )
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_planet_reaches_as_far_as_its_rings_and_atmosphere() {
        for (source, want) in [
            // The sample's: rings to 2.3, an atmosphere 0.04 thick.
            (modelled_sample_3d(), 2.3),
            (
                kestrel_reach_with("    atmosphere: {color: \"#f0e0c0\", thickness: 0.5}\n"),
                1.5,
            ),
            (
                kestrel_reach_with("    rings: {inner: 1.4, outer: 2.0, color: \"#d8c8a8\"}\n"),
                2.0,
            ),
            (kestrel_reach_with(""), 1.0),
        ] {
            let (game, assets) = built(&source);
            assert_eq!(assets.problems, Vec::<String>::new());
            let [(key, planet)] = &assets.planets[..] else {
                panic!("one planet: {:?}", assets.planets.len());
            };
            let surface = &planet.clone().into_surface();
            #[allow(clippy::cast_possible_truncation, reason = "as the renderer reads it")]
            let reach = crate::planet::reach(
                surface.atmosphere.as_ref().map(|a| a.thickness as f32),
                surface.rings.as_ref().map(|r| r.outer as f32),
            );
            assert!((reach - want).abs() < 1e-6, "{reach} is not {want}");
            assert_eq!(
                assets.models.stellar(stellar(&game, "kestrel-reach")),
                Some(StellarModel::planet(*key, reach))
            );
        }
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
            Some(StellarModel::planet(PlanetKey(1), 1.0)),
            "a bare globe"
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
    fn a_part_reaches_its_lifted_mount_s_distance_plus_its_own_radius() {
        // (3, 4, 10 + 2) is 13 from the origin.
        assert!((part_reach([3.0, 4.0, 10.0], 2.0, 0.5) - 13.5).abs() < 1e-6);
        // (0, 0, 0 - 1) is 1 from it: a lift down reaches as far.
        assert!((part_reach([0.0, 0.0, 0.0], -1.0, 0.25) - 1.25).abs() < 1e-6);
    }

    #[test]
    fn a_class_s_radius_reaches_its_hull_and_each_part_its_mounts_as_built_show() {
        let (game, assets) = built(&modelled_sample_3d());
        let table = &assets.models;
        let turret = table.outfit("turret-mount").expect("the turret's key");
        let turret_radius = mesh(&assets, turret).radius();
        let gunship = ship(&game, "harrow-gunship");
        let model = table.ship(gunship).expect("the gunship's model");
        let hull_radius = mesh(&assets, model.key).radius();
        let mounts = table.mounts(gunship, table.built_kits(gunship));
        let parts: Vec<f32> = mounts
            .iter()
            .filter_map(|mount| {
                let shown = mount.shows.as_ref()?;
                let [x, y, z] = mount.at;
                let lifted = (x * x + y * y + (z + shown.lift) * (z + shown.lift)).sqrt();
                #[allow(clippy::cast_possible_truncation)]
                Some(lifted as f32 + turret_radius)
            })
            .collect();
        assert!(!parts.is_empty(), "the gun bed shows the turret");
        let want = parts.iter().copied().fold(hull_radius, f32::max);
        assert!(
            (model.radius - want).abs() < 1e-5,
            "{} is not {want}",
            model.radius
        );
        assert!(model.radius.is_finite() && model.radius > 0.0);

        let freighter = ship(&game, "light-freighter");
        let plain = table.ship(freighter).expect("the freighter's model");
        assert!(
            table
                .mounts(freighter, table.built_kits(freighter))
                .iter()
                .all(|mount| mount.shows.is_none()),
            "the freighter shows no part"
        );
        let hull = mesh(&assets, plain.key).radius();
        assert!((plain.radius - hull).abs() < f32::EPSILON);
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
