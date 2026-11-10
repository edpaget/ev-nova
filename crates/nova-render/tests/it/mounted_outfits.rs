//! The sample's outfits drawn at their hardpoints, offscreen: the light
//! freighter with its gun mount, loaded from the modelled sample
//! (`nova_data::new_game::fixture::modelled_sample`) through the new
//! game's loader, its hull and the mass driver's turret registered
//! through [`GalaxyModels`], and the draws made by the view's
//! `mounted_outfits` from the store's mounts.
//!
//! The turret is a fraction of a metre at a metre a model unit, so the
//! frame is close: eight logical units a metre, two pixels a unit, the
//! hulls' decks filling the background. Two freighters are drawn side by
//! side, the first carrying the kit, the second without it. Every region
//! sampled is worked out from the core (the mount's point, the turret's
//! fitting, the pose's scale and turns), not retyped, and lies inside
//! what it names with a margin.
//!
//! Natively only: the upscalers are covered by `world_models`. Needs a
//! GPU adapter; skips, passing, without one.

use std::path::Path;

use nova_data::new_game::fixture::{TURRET_MOUNT_GLB, modelled_sample};
use nova_data::new_game::{NewGame, load};
use nova_data::store::Mount;
use nova_data::wire::id::{OutfitId, ShipId};
use nova_galaxy_format::asset::Front;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::source::{MemSource, Source};
use nova_galaxy_media::mount::fitting;
use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::{
    GalaxyModels, Gpu, Light, LogicalSize, Mesh, ModelRequest, ModelSource, RenderReport, Renderer,
    Upscaler, Viewport, ship_request,
};
use nova_view::flight::mounted_outfits;
use nova_view::{
    DrawCommand, DrawList, ModelKey, ModelPose, Mounting, Orientation, Point, view_axes,
};

use super::wgpu_offscreen::{Images, gpu_with};

/// The logical screen: wide enough for two freighters' mounts side by
/// side, each hull's deck all round its own.
const LOGICAL: LogicalSize = LogicalSize { w: 440, h: 64 };
/// The target, two pixels a logical unit.
const WIDTH: u32 = 880;
const HEIGHT: u32 = 128;

/// Logical units a metre.
const SCALE: f32 = 8.0;

const HULL: ModelKey = ModelKey(1);
const TURRET: ModelKey = ModelKey(2);

/// Each freighter's centre: the first, with the kit, and the second,
/// without. 26 m across, each spans 208 units, so the two meet midway.
const CENTRES: [(f32, f32); 2] = [(110.0, 16.0), (330.0, 16.0)];

/// The hulls' angle, in degrees, so a mount's place and a turret's turn
/// both go through the hull's turn.
const HULL_ANGLE: f32 = 30.0;

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

/// The modelled sample, loaded, and its source.
fn sample() -> (MemSource, NewGame) {
    let source = modelled_sample();
    let game = load(&source, Path::new("sample"));
    assert_eq!(game.problems.len(), 0, "{:?}", game.problems);
    (source, game)
}

/// The light freighter's mounts carrying the gun mount, or none.
fn mounts(game: &NewGame, with_kit: bool) -> Vec<Mount> {
    let freighter = ShipId(game.ids.ships[&id("light-freighter")]);
    let kit = OutfitId(game.ids.kits[&id("gun-mount")]);
    let kits: &[OutfitId] = if with_kit { &[kit] } else { &[] };
    game.data.mounts(freighter, kits)
}

/// The hull's and the turret's meshes, as the adapter loads them.
fn meshes(source: &MemSource) -> (Mesh, Mesh) {
    let (galaxy, _) = nova_galaxy_format::load::load(source);
    let models = GalaxyModels::new(&galaxy, source);
    let request = ship_request(&galaxy, &id("light-freighter")).expect("a model");
    let hull = models.model(&request).expect("the hull loads").mesh();
    let turret = models
        .model(&ModelRequest::new("turret-mount", None))
        .expect("the turret loads")
        .mesh();
    (hull, turret)
}

/// How the turret sits on its mount: its half length ahead, its muzzle.
fn reach(source: &MemSource) -> f32 {
    let bytes = source
        .bytes(TURRET_MOUNT_GLB)
        .expect("a memory read")
        .expect("the turret's file");
    let model = nova_galaxy_media::model(TURRET_MOUNT_GLB, &bytes, |_| None).expect("it reads");
    fitting(&model, Front::MinusZ).reach as f32
}

/// Freighter `n`'s pose.
fn pose(n: usize) -> ModelPose {
    let (x, y) = CENTRES[n];
    ModelPose {
        center: Point::new(x, y),
        // Turned about the view axis by the hull's angle, as a ship's
        // model is (`nova_view::flight::ship_orientation`).
        orientation: Orientation::about([0.0, 0.0, 1.0], HULL_ANGLE.to_radians()),
        scale: SCALE,
    }
}

fn key_of(asset: &str) -> Option<ModelKey> {
    (asset == "turret-mount").then_some(TURRET)
}

/// `commands` drawn in the world with the hull and the turret
/// registered, read back; `None` with no adapter.
fn drawn(commands: &[DrawCommand], (hull, turret): &(Mesh, Mesh)) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(WIDTH, HEIGHT, &FontFaces::bundled())?;
    gpu.set_upscaler(Upscaler::Native);
    let mut renderer = Renderer::new(Images);
    renderer.add_model(HULL, hull.clone());
    renderer.add_model(TURRET, turret.clone());
    let mut list = DrawList::new();
    list.world(|list| {
        for command in commands {
            list.push(command.clone());
        }
    });
    let viewport = Viewport::new(LOGICAL, (WIDTH, HEIGHT), 2.0);
    let report = renderer.render(&list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The colour at logical `(x, y)`.
fn color_at(pixels: &[u8], (x, y): (f32, f32)) -> [u8; 3] {
    let (px, py) = ((x * 2.0) as u32, (y * 2.0) as u32);
    let at = ((py * WIDTH + px) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The colour `mesh`'s top face shows from above: its highest
/// upward-facing vertex's colour, lit as the core shades a face facing the
/// camera.
fn top_colour(mesh: &Mesh) -> [u8; 3] {
    let top = mesh
        .vertices()
        .iter()
        .filter(|v| v.normal[2] > 0.99)
        .max_by(|a, b| a.position[2].total_cmp(&b.position[2]))
        .expect("a face up");
    let [r, g, b, _] = shade([0.0, 0.0, 1.0], top.color, Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// Whether each channel of `a` is within 2 of `b`'s.
fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= 2)
}

/// Where the point `p` (on the part's own axes, in metres) of a part
/// mounted as `mounting` on freighter `n` lands, in logical units: the
/// hull's turn of `at + R_z(turn) · p`, by `Orientation` alone.
fn landing(n: usize, mounting: Mounting, p: [f32; 3]) -> (f32, f32) {
    let local = Orientation::about([0.0, 0.0, 1.0], mounting.turn)
        .rotate(p)
        .expect("a unit turn");
    let on_hull = [0, 1, 2].map(|k| mounting.at[k] + local[k]);
    let pose = pose(n);
    let turned = pose.orientation.rotate(on_hull).expect("a unit turn");
    (
        pose.center.x + pose.scale * turned[0],
        pose.center.y + pose.scale * turned[1],
    )
}

/// The points, on a turret's own view axes, that a turret's top covers
/// with a margin: across ±0.1 of its 0.4 m width, along `along` of its
/// length (−y is its nose).
fn on_turret(along: &[f32]) -> Vec<[f32; 3]> {
    along
        .iter()
        .flat_map(|&y| [-0.1, 0.0, 0.1].map(|x| [x, y, 0.0]))
        .collect()
}

/// Where a part at the mount `mount` stands, seen from above, turned
/// `turn`: the mount's `[x, y]` from the store, on the view's axes. Its
/// height moves no pixel under the camera looking straight down, so it is
/// left at 0.
fn seen_at(mount: &Mount, turn: f32) -> Mounting {
    let [x, y, _] = mount.at;
    #[allow(clippy::cast_possible_truncation)]
    Mounting {
        at: view_axes([x as f32, y as f32, 0.0]),
        turn,
    }
}

/// Asserts every point of `points`, landed by `land`, shows `colour`.
#[track_caller]
fn assert_all(
    pixels: &[u8],
    points: &[[f32; 3]],
    land: impl Fn([f32; 3]) -> (f32, f32),
    colour: [u8; 3],
    what: &str,
) {
    for &p in points {
        let at = land(p);
        let got = color_at(pixels, at);
        assert!(
            near(got, colour),
            "{what} at {at:?}: {got:?}, not {colour:?}"
        );
    }
}

#[test]
fn the_sample_s_turret_draws_on_its_hardpoint_on_the_deck() {
    let (source, game) = sample();
    let meshes = meshes(&source);
    let (deck, red) = (top_colour(&meshes.0), top_colour(&meshes.1));
    assert!(!near(deck, red), "the turret shows against the deck");
    let reach = reach(&source);
    let with_kit = mounts(&game, true);
    let plain = mounts(&game, false);
    assert_eq!(plain, [], "the freighter alone has no hardpoint");

    let ship = |n: usize, mounts: &[Mount], facings: &[Option<f32>]| {
        mounted_outfits(HULL, pose(n), HULL_ANGLE, mounts, facings, key_of)
    };
    let armed = ship(0, &with_kit, &[]);
    let bare = ship(1, &plain, &[]);
    assert_eq!(bare.len(), 1, "the bare freighter draws its hull alone");
    assert_eq!(armed.len(), 2, "the hull, then its turret: {armed:?}");
    let mounting = seen_at(&with_kit[0], 0.0);
    let whole = on_turret(&[-0.5, -0.25, 0.0, 0.25, 0.5]);

    // (a) The turret on the armed freighter's deck, the bare one's deck
    // where its mount would be; the armed one drawn without its turret
    // shows its deck there.
    let both: Vec<DrawCommand> = armed.iter().chain(&bare).cloned().collect();
    let Some(pixels) = drawn(&both, &meshes) else {
        return;
    };
    assert_all(
        &pixels,
        &whole,
        |p| landing(0, mounting, p),
        red,
        "the turret",
    );
    // (d) The bare freighter shows no outfit at the same place on it.
    assert_all(
        &pixels,
        &whole,
        |p| landing(1, mounting, p),
        deck,
        "the bare freighter's deck",
    );
    let hulls: Vec<DrawCommand> = [armed[0].clone(), bare[0].clone()].into();
    let Some(alone) = drawn(&hulls, &meshes) else {
        return;
    };
    assert_all(
        &alone,
        &whole,
        |p| landing(0, mounting, p),
        deck,
        "the deck without the turret",
    );

    // (b) Turned a quarter off the hull's heading, the barrel shows a
    // quarter turn round its pivot, and the deck where it was.
    let turned = ship(0, &with_kit, &[Some(HULL_ANGLE + 90.0)]);
    let quarter = seen_at(&with_kit[0], std::f32::consts::FRAC_PI_2);
    let barrel = on_turret(&[-0.8 * reach, -0.5 * reach]);
    let Some(pixels) = drawn(&turned, &meshes) else {
        return;
    };
    assert_all(
        &pixels,
        &barrel,
        |p| landing(0, quarter, p),
        red,
        "the turned barrel",
    );
    assert_all(
        &pixels,
        &barrel,
        |p| landing(0, mounting, p),
        deck,
        "where the barrel was",
    );

    // (c) The same turret stood on the plane, raised by its lift alone, is
    // inside the hull: the deck hides it.
    let lift = with_kit[0].shows.as_ref().expect("shows the turret").lift;
    let [x, y, _] = with_kit[0].at;
    #[allow(clippy::cast_possible_truncation)]
    let on_plane = Mounting {
        at: view_axes([x as f32, y as f32, lift as f32]),
        turn: 0.0,
    };
    let sunk = [
        armed[0].clone(),
        DrawCommand::Mounted {
            model: TURRET,
            mounting: on_plane,
        },
    ];
    let Some(pixels) = drawn(&sunk, &meshes) else {
        return;
    };
    assert_all(
        &pixels,
        &whole,
        |p| landing(0, mounting, p),
        deck,
        "the turret under the deck",
    );
}
