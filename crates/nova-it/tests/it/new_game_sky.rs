//! The new game draws the layered sky: over the checked-in sample
//! (`nova_data::new_game::fixture::sample_folder`), the router wired with
//! the rulebook `ruleset_rulebook` gives `Ruleset::NewGame`, as `main`
//! wires it, the flight's world layer holds one `DrawCommand::Sky` and
//! none of the data's stand-in dots, and the sky's eye moves with the
//! ship. Under the stock ruleset's readings the same flight draws the
//! dots and no sky.
//!
//! Wired as `main` wires the world assets (`nova_render::world_assets`
//! over the source, its sky looks handed to the renderer), the frame's
//! sky batch is the system's look: with no `sky` block
//! (`fixture::modelled_sample`) the seeded backdrop under the stars, and
//! with one (`fixture::sky_sample`) the data's red clouds and no stars.
//! Under the stock readings the looks registered draw nothing. A system
//! whose data lists decorations (`fixture::decorated_sample`) draws them
//! as one sky-images batch after its stars, each image uploaded once, and
//! under the stock readings none. A system that lists none draws its
//! seed's default, the far galaxy the renderer gives it or nothing.
//!
//! Wiring only: the layered sky's rule is nova-view's (`flight::sky` and
//! the view's `sky` module), its look nova-render's (`sky`, and
//! `new_game_sky` for the data's), the seed nova-view's
//! (`flight::sky::sky_seed`), each pinned there; this pins that the
//! ruleset, the data, the edge, the router, the view and the renderer
//! meet.

use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use nova::app::screen::AppScreen;
use nova::app::{Control, WindowEvent};
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::new_game;
use nova_flight::SessionViewExt as _;
use nova_render::gpu::Batch;
use nova_render::recording::GpuCall;
use nova_render::sky::{Placement, SkyLook, StarInstance, backdrop, galaxy, placed};
use nova_render::{
    Frame, LogicalSize, SkyImage, SkyImageInstance, WorldAssets, WorldSpace, world_assets,
};
use nova_sim::Vec2;
use nova_sim::fixture::MemoryPilots;
use nova_view::{DrawCommand, DrawLayer, DrawList, Key, Screen, SkyPose};

use crate::common::{draw, fly_new_pilot_wired};
use crate::new_game_models::flying_over;
use crate::pilots::Harness;

/// The app flying the sample's start under `ruleset`'s readings.
fn flying(ruleset: Ruleset, pilots: &MemoryPilots) -> Harness {
    let data = std::rc::Rc::new(
        new_game::open(&new_game::fixture::sample_folder())
            .expect("the sample lists")
            .data,
    );
    let rules = ruleset_rulebook(None, ruleset.profile().readings).0;
    let settings = MemorySettings::new();
    let (keeper, warning) =
        nova::settings::SettingsKeeper::open(Box::new(settings) as Box<dyn SettingsStore>);
    assert_eq!(warning, None);
    fly_new_pilot_wired(&data, pilots, keeper, |screen: AppScreen| {
        screen.with_rulebook(&rules)
    })
}

/// The world layer of the view's frame drawn now.
fn world(game: &Harness) -> Vec<DrawCommand> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .filter(|(layer, _)| *layer == DrawLayer::World)
        .map(|(_, command)| command.clone())
        .collect()
}

/// The skies in the world layer now.
fn skies(game: &Harness) -> Vec<SkyPose> {
    world(game)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Sky { sky } => Some(sky),
            _ => None,
        })
        .collect()
}

/// How many OR fills (the stand-in dots) the world layer holds now.
fn dots(game: &Harness) -> usize {
    world(game)
        .iter()
        .filter(|command| matches!(command, DrawCommand::OrFill { .. }))
        .count()
}

/// The player's ship's position and velocity in the world now.
fn ship(game: &Harness) -> (Vec2, Vec2) {
    let player = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player();
    (player.position, player.velocity)
}

/// Sends Up going down, or up when not `pressed`.
fn thrust(game: &mut Harness, pressed: bool) {
    let event = WindowEvent::Key {
        key: Key::ArrowUp,
        pressed,
        repeat: false,
    };
    assert_eq!(game.handle(event), Control::Continue);
}

#[test]
fn the_new_game_draws_the_layered_sky_and_no_dots() {
    let pilots = MemoryPilots::new();
    let game = flying(Ruleset::NewGame, &pilots);
    assert_eq!(skies(&game).len(), 1, "one sky");
    assert!(
        matches!(world(&game).first(), Some(DrawCommand::Sky { .. })),
        "first in the world"
    );
    assert_eq!(dots(&game), 0, "none of the data's dots");
}

#[test]
fn the_stock_readings_draw_the_dots_and_no_sky() {
    let pilots = MemoryPilots::new();
    let game = flying(Ruleset::Stock, &pilots);
    assert_eq!(skies(&game), []);
    assert!(dots(&game) > 0, "the data's dots");
}

#[test]
fn the_new_game_s_sky_moves_with_the_ship() {
    let pilots = MemoryPilots::new();
    let mut game = flying(Ruleset::NewGame, &pilots);
    // Up to speed, then coasting, so the drawn frame's lag behind the
    // session is the same at both ends of the measure. Up is pressed
    // again each frame until it takes.
    for _ in 0..2000 {
        if ship(&game).1.length() > 0.0 {
            break;
        }
        thrust(&mut game, true);
        draw(&mut game);
    }
    assert!(ship(&game).1.length() > 0.0, "Up never thrusts");
    for _ in 0..120 {
        draw(&mut game);
    }
    thrust(&mut game, false);
    for _ in 0..5 {
        draw(&mut game);
    }
    let (from, before) = (ship(&game).0, skies(&game)[0].eye);
    for _ in 0..20 {
        draw(&mut game);
    }
    let (to, after) = (ship(&game).0, skies(&game)[0].eye);
    let travel = to - from;
    assert!(travel.length() > 10.0, "the ship travels: {travel:?}");
    // The eye is over the ship as drawn, which trails the session by
    // under a step at each end: within a few units of its travel.
    let eye = (after.x - before.x, after.y - before.y);
    assert!(
        (eye.0 - travel.x).abs() < 3.0 && (eye.1 - travel.y).abs() < 3.0,
        "the eye moved {eye:?}, the ship {travel:?}"
    );
    let camera = game.app.screen().flight_view().expect("flying").camera();
    assert_eq!(after, camera.center(), "over the camera's centre");
}

/// The new game in the in-memory folder `$source` loaded, and its world
/// assets built over it, as `main` builds them.
macro_rules! loaded {
    ($source:expr) => {{
        let source = $source;
        let game = new_game::load(&source, Path::new("sample"));
        let assets = world_assets(&game, &source);
        (game, assets)
    }};
}

/// The app flying `game`'s start under `ruleset`'s readings, wired with
/// its world `assets`, sky looks included, as `main` wires them; and those
/// assets.
fn flying_with_looks(
    (game, assets): (new_game::NewGame, WorldAssets),
    ruleset: Ruleset,
    pilots: &MemoryPilots,
) -> (Harness, WorldAssets) {
    assert_eq!(
        game.problems
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        Vec::<String>::new()
    );
    assert_eq!(assets.problems, Vec::<String>::new());
    let harness = flying_over(ruleset, &Rc::new(game.data), &assets, pilots);
    (harness, assets)
}

/// The frame drawn next.
fn frame_drawn(game: &mut Harness) -> Frame {
    draw(game);
    (game.gpu_calls().iter().rev())
        .find_map(|call| match call {
            GpuCall::Submit(frame) => Some(frame.clone()),
            _ => None,
        })
        .expect("a frame submitted")
}

/// The sky batches of the frame drawn next, and its logical size.
fn sky_batches(game: &mut Harness) -> (Vec<Vec<StarInstance>>, LogicalSize) {
    let frame = frame_drawn(game);
    let skies = frame
        .batches
        .into_iter()
        .filter_map(|batch| match batch {
            Batch::Sky(drawn) => Some(drawn),
            _ => None,
        })
        .collect();
    (skies, frame.logical)
}

#[test]
fn a_system_with_no_sky_block_draws_its_seeded_backdrop_under_its_stars() {
    let pilots = MemoryPilots::new();
    let loaded = loaded!(new_game::fixture::modelled_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::NewGame, &pilots);
    assert!(assets.skies.is_empty(), "no system sets a look");
    let (batches, logical) = sky_batches(&mut game);
    let pose = skies(&game)[0];
    let [drawn] = &batches[..] else {
        panic!("one sky batch: {batches:?}");
    };
    // The backdrop's puffs don't hang on the texels, so the world
    // texture's needn't be known.
    let puffs = backdrop(&pose, &SkyLook::default(), logical);
    assert!(!puffs.is_empty(), "the start view shows a cloud");
    assert_eq!(drawn[..puffs.len()], puffs[..], "the backdrop first");
    assert!(drawn.len() > puffs.len() + 100, "stars after it");
}

#[test]
fn a_system_s_sky_block_draws_its_own_look() {
    let pilots = MemoryPilots::new();
    let loaded = loaded!(new_game::fixture::sky_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::NewGame, &pilots);
    let (batches, logical) = sky_batches(&mut game);
    let pose = skies(&game)[0];
    let look = assets.skies.look(pose.seed);
    assert_ne!(look, SkyLook::default(), "the start system's own look");
    let [drawn] = &batches[..] else {
        panic!("one sky batch: {batches:?}");
    };
    // No stars: the backdrop alone, red, a cloud in every cell.
    assert_eq!(*drawn, backdrop(&pose, &look, logical));
    assert!(!drawn.is_empty());
    for puff in drawn {
        let [r, g, b] = puff.color;
        assert!(r > 0.0 && g == 0.0 && b == 0.0, "{puff:?}");
    }
}

#[test]
fn the_stock_readings_draw_no_sky_with_looks_registered() {
    let pilots = MemoryPilots::new();
    let loaded = loaded!(new_game::fixture::sky_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::Stock, &pilots);
    assert_eq!(assets.skies.len(), 1, "a look registered");
    let (batches, _) = sky_batches(&mut game);
    assert_eq!(batches, Vec::<Vec<StarInstance>>::new());
    assert_eq!(skies(&game), []);
}

/// The texels a logical unit is in the canvas `frame`'s world is drawn
/// into: the smaller axis's, as the renderer hands the sky.
fn world_texels(frame: &Frame) -> f32 {
    let (width, height) = match frame.world_space {
        WorldSpace::Texture { size } => size,
        WorldSpace::Target => (frame.viewport.w, frame.viewport.h),
    };
    let across = width as f32 / frame.logical.w as f32;
    let down = height as f32 / frame.logical.h as f32;
    across.min(down)
}

/// The sky-image batches of `frame`, each with the index of the batch
/// before it that is a sky's stars, if the one before it is.
fn sky_image_batches(frame: &Frame) -> Vec<(Option<usize>, Vec<SkyImageInstance>)> {
    (frame.batches.iter().enumerate())
        .filter_map(|(at, batch)| match batch {
            Batch::SkyImages(images) => {
                let after_sky = at
                    .checked_sub(1)
                    .filter(|before| matches!(frame.batches[*before], Batch::Sky(_)));
                Some((after_sky, images.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The sky images uploaded to `game`'s GPU so far, in order.
fn sky_image_uploads(game: &Harness) -> Vec<SkyImage> {
    (game.gpu_calls().iter())
        .filter_map(|call| match call {
            GpuCall::UploadSkyImage { data, .. } => Some(data.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_system_that_lists_decorations_draws_them_after_its_stars() {
    let pilots = MemoryPilots::new();
    let loaded = loaded!(new_game::fixture::decorated_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::NewGame, &pilots);
    let frame = frame_drawn(&mut game);
    let pose = skies(&game)[0];
    let list = assets.skies.decorations(pose.seed).expect("its list");
    assert_eq!(list.len(), 2, "the start system's two");
    // What the renderer's core places for the view's pose and the
    // assets' decorations, in its order.
    let expected = placed(&pose, list, frame.logical, world_texels(&frame));
    assert_eq!(expected.len(), 2, "both show from the start");
    let [(after_sky, images)] = &sky_image_batches(&frame)[..] else {
        panic!("one sky-images batch: {:?}", sky_image_batches(&frame));
    };
    assert!(after_sky.is_some(), "after the sky's own batch");
    let drawn: Vec<Placement> = images.iter().map(|image| image.placement).collect();
    let placements: Vec<Placement> = expected.iter().map(|(_, placement)| *placement).collect();
    assert_eq!(drawn, placements);
    // Each image uploaded once, in the order drawn, however many frames
    // draw it.
    frame_drawn(&mut game);
    let images: Vec<SkyImage> = (expected.iter())
        .map(|(index, _)| (*list[*index].image).clone())
        .collect();
    assert_eq!(sky_image_uploads(&game), images);
    // The sky's clock is the flight's: once the flight's clock runs (it
    // holds while the screen enters), the pose is later, and the frame
    // draws what the core places then, the spinning hulk turned.
    for _ in 0..2000 {
        if skies(&game)[0].time > pose.time + 0.5 {
            break;
        }
        draw(&mut game);
    }
    let frame = frame_drawn(&mut game);
    let later = skies(&game)[0];
    assert!(
        later.time > pose.time + 0.5,
        "{} then {}",
        pose.time,
        later.time
    );
    let turned = placed(&later, list, frame.logical, world_texels(&frame));
    let [(_, images)] = &sky_image_batches(&frame)[..] else {
        panic!("one sky-images batch");
    };
    let drawn: Vec<Placement> = images.iter().map(|image| image.placement).collect();
    let placements: Vec<Placement> = turned.iter().map(|(_, placement)| *placement).collect();
    assert_eq!(drawn, placements);
    let then = placed(
        &later.with_time(pose.time),
        list,
        frame.logical,
        world_texels(&frame),
    );
    assert_ne!(turned, then, "the hulk turned with the clock");
}

/// What the renderer's core draws by default for the sky `pose` sees in
/// `frame`: its seed's galaxy, baked and placed, or nothing.
fn seeded_default(pose: &SkyPose, frame: &Frame) -> Vec<(SkyImage, Placement)> {
    let Some(far) = galaxy::seeded(pose.seed) else {
        return Vec::new();
    };
    let image = Arc::new(galaxy::bake(&far));
    let list = [galaxy::decoration(&far, Arc::clone(&image))];
    placed(pose, &list, frame.logical, world_texels(frame))
        .into_iter()
        .map(|(_, placement)| ((*image).clone(), placement))
        .collect()
}

/// The frame `game` draws next shows `expected` as its sky images, each
/// uploaded once.
#[track_caller]
fn assert_draws_default(game: &mut Harness, assets: &WorldAssets, galaxy: bool) {
    let frame = frame_drawn(game);
    let pose = skies(game)[0];
    assert_eq!(assets.skies.decorations(pose.seed), None, "no list");
    let expected = seeded_default(&pose, &frame);
    assert_eq!(expected.len(), usize::from(galaxy), "seed {}", pose.seed);
    let drawn: Vec<Placement> = sky_image_batches(&frame)
        .into_iter()
        .flat_map(|(after_sky, images)| {
            assert!(after_sky.is_some(), "after the sky's own batch");
            images
        })
        .map(|image| image.placement)
        .collect();
    let placements: Vec<Placement> = expected.iter().map(|(_, placement)| *placement).collect();
    assert_eq!(drawn, placements);
    frame_drawn(game);
    let images: Vec<SkyImage> = expected.into_iter().map(|(image, _)| image).collect();
    assert_eq!(sky_image_uploads(game), images);
}

#[test]
fn a_system_that_lists_no_decorations_draws_its_seeds_default() {
    let pilots = MemoryPilots::new();
    // The sample's start system, Luyten Crossing, has a seed with no
    // galaxy: its sky draws no images.
    let loaded = loaded!(new_game::fixture::modelled_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::NewGame, &pilots);
    assert_draws_default(&mut game, &assets, false);
    // The same sample with the start system pinned at Brightwater's ID,
    // a seed that has one: its sky draws it, baked at its first frame.
    let pilots = MemoryPilots::new();
    let ids = std::fs::read_to_string(new_game::fixture::sample_folder().join("nova-ids.json"))
        .expect("the sample's ID table");
    let swapped = ids
        .replacen("\"brightwater\": 128", "\"brightwater\": 129", 1)
        .replacen("\"luyten-crossing\": 129", "\"luyten-crossing\": 128", 1);
    assert_ne!(swapped, ids);
    let loaded = loaded!(new_game::fixture::modelled_sample().with("nova-ids.json", &swapped));
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::NewGame, &pilots);
    assert_draws_default(&mut game, &assets, true);
}

#[test]
fn the_stock_readings_draw_no_seeded_galaxy() {
    let pilots = MemoryPilots::new();
    let ids = std::fs::read_to_string(new_game::fixture::sample_folder().join("nova-ids.json"))
        .expect("the sample's ID table");
    let swapped = ids
        .replacen("\"brightwater\": 128", "\"brightwater\": 129", 1)
        .replacen("\"luyten-crossing\": 129", "\"luyten-crossing\": 128", 1);
    assert_ne!(swapped, ids);
    let loaded = loaded!(new_game::fixture::modelled_sample().with("nova-ids.json", &swapped));
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::Stock, &pilots);
    // The system flown has a seed the renderer gives a galaxy, and no
    // list in its place: the new game's readings would draw one here.
    let flight = game.app.screen().flight_view().expect("flying");
    let system = flight.session().expect("a session").system();
    let seed = nova_view::flight::sky::sky_seed(system);
    assert!(galaxy::seeded(seed).is_some(), "seed {seed} has a galaxy");
    assert_eq!(assets.skies.decorations(seed), None, "no list");
    let frame = frame_drawn(&mut game);
    assert_eq!(sky_image_batches(&frame), []);
    assert_eq!(sky_image_uploads(&game), []);
}

#[test]
fn the_stock_readings_draw_no_decorations_with_them_registered() {
    let pilots = MemoryPilots::new();
    let loaded = loaded!(new_game::fixture::decorated_sample());
    let (mut game, assets) = flying_with_looks(loaded, Ruleset::Stock, &pilots);
    let listing = (new_game::load(&new_game::fixture::decorated_sample(), Path::new("sample")))
        .ids
        .systems
        .values()
        .filter(|&&nova| {
            let seed = nova_view::flight::sky::sky_seed(nova_sim::SystemId(nova));
            assets.skies.decorations(seed).is_some()
        })
        .count();
    assert_eq!(listing, 1, "a system's decorations registered");
    let frame = frame_drawn(&mut game);
    assert_eq!(sky_image_batches(&frame), []);
    assert_eq!(sky_image_uploads(&game), []);
    assert_eq!(skies(&game), []);
}
