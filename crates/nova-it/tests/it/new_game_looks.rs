//! The new game's ships and stellars drawn as their stand-ins through the
//! whole app (rdm:phase/new-game-engine/phase-4-draw-flat-outlines): New
//! Pilot over the checked-in fixture puts the player in flight in
//! `hearth`, in the hauler, and the flight draws the hauler as solid
//! geometry in its livery and `hearth-dock` as its stand-in, with no
//! "Sprite unavailable" anywhere.
//!
//! With the stock ship look on
//! (rdm:phase/play-the-sample/phase-12-new-game-stock-sprite-look), over
//! a synthetic stock store made in code whose freighter default is there,
//! the hauler draws its borrowed stock sprites instead, and the same
//! flight saves byte-equal pilots and settings with the look on or off.
//!
//! Wiring only: what each stand-in is, is nova-data's
//! (`new_game::looks`); how one is drawn and turned, nova-view's
//! (`outline`, the flight and scene tests); how a triangle reaches the
//! GPU, nova-render's. What a hull borrows is nova-data's
//! (`stock_look`), and how a borrowed sheet is read and drawn,
//! nova-view's (`StockShipLooks`, the flight's tests). The borrowed frames
//! the flight draws decode through the game's images as `main` builds
//! them (`stock_look::images`); how one is decoded and scaled is
//! nova-render's (`GameImages::with_stock_art`).

use std::rc::Rc;

use nova::app::WindowEvent;
use nova::app::screen::AppScreen;
use nova::stock_look::{StockLook, images, wire};
use nova_audio::SettingsStore;
use nova_audio::recording::MemorySettings;
use nova_data::stock_look::fixture::stock_store;
use nova_data::stock_look::{ROLE_DEFAULTS, Role};
use nova_data::{GameData, StockArt, new_game};
use nova_render::ImageSource;
use nova_sim::Rulebook;
use nova_sim::fixture::MemoryPilots;
use nova_view::{Color, DrawCommand, DrawLayer, DrawList, ImageKind, Point, Screen};

use crate::common::{draw, fly_new_pilot, fly_new_pilot_wired};
use crate::pilots::Harness;

/// The guild's colour, `#c08040`: the hauler's livery's primary, and the
/// dock's owner's.
const GUILD: Color = Color::rgba(0xc0, 0x80, 0x40, 255);

/// The world's triangles in `color` that flight draws, as their corners.
fn world_triangles(list: &DrawList, color: Color) -> Vec<[Point; 3]> {
    list.layered()
        .filter_map(|(layer, command)| match command {
            DrawCommand::Triangle { corners, color: c }
                if layer == DrawLayer::World && *c == color =>
            {
                Some(*corners)
            }
            _ => None,
        })
        .collect()
}

/// Whether `at` is inside the box round `corners`.
fn boxes(corners: &[Point; 3], at: Point) -> bool {
    let (xs, ys) = (corners.map(|p| p.x), corners.map(|p| p.y));
    let low = |v: [f32; 3]| v.into_iter().fold(f32::MAX, f32::min);
    let high = |v: [f32; 3]| v.into_iter().fold(f32::MIN, f32::max);
    (low(xs)..=high(xs)).contains(&at.x) && (low(ys)..=high(ys)).contains(&at.y)
}

#[test]
fn the_new_game_s_ships_and_stellars_draw_as_their_stand_ins() {
    let store = MemoryPilots::new();
    let (game, _) = fly_new_pilot(&new_game::fixture::folder(), &store);
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    let guild = world_triangles(&list, GUILD);

    // The hauler: solid geometry in its livery round where the player is
    // drawn.
    let player = view.camera().world_to_screen(view.shown_position());
    assert!(
        guild.iter().any(|corners| boxes(corners, player)),
        "the hauler round {player:?}: {guild:?}"
    );

    // The dock: its stand-in, in the guild's colour too, told from the
    // hauler by its shape: a disc's triangles fan out from its centre, so
    // a corner three or more of them share, where no corner of a box is.
    let shared = guild.iter().flatten().find(|&&corner| {
        guild
            .iter()
            .filter(|corners| corners.contains(&corner))
            .count()
            >= 3
    });
    assert!(shared.is_some(), "the dock's disc: {guild:?}");

    let unavailable: Vec<_> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } if text.contains("Sprite unavailable") => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(unavailable, Vec::<&String>::new());
}

/// The settings file turning the look on.
const LOOK_ON: &str = r#"{"stock_ship_look": {"on": true}}"#;

/// The stock freighter the hauler borrows by default.
fn freighter() -> i16 {
    ROLE_DEFAULTS
        .iter()
        .find(|(role, _)| *role == Role::Freighter)
        .map(|&(_, stock)| stock)
        .expect("a default")
}

/// The app flying a new pilot over the fixture, its pilots in `pilots`
/// and its settings in `settings`; with the stock look wired as `main`
/// wires it, over a synthetic stock store holding the freighter, when
/// `look`.
fn fly(pilots: &MemoryPilots, settings: &MemorySettings, look: bool) -> Harness {
    fly_wired(pilots, settings, look).0
}

/// The app [`fly`] flies, with the game data and the stock art `main`
/// builds the game's images from.
fn fly_wired(
    pilots: &MemoryPilots,
    settings: &MemorySettings,
    look: bool,
) -> (Harness, Rc<GameData>, Option<Rc<StockArt>>) {
    let data = Rc::new(
        new_game::open(&new_game::fixture::folder())
            .expect("the fixture lists")
            .data,
    );
    let on = StockLook {
        on: true,
        ..StockLook::default()
    };
    let stock = look.then(|| stock_store(&[(freighter(), 40)]));
    let (wired, _) = wire(&data, &on, stock);
    let (art, looks) = wired.map(|(art, looks)| (art, Rc::new(looks))).unzip();
    let (keeper, warning) =
        nova::settings::SettingsKeeper::open(Box::new(settings.clone()) as Box<dyn SettingsStore>);
    assert_eq!(warning, None);
    let game = fly_new_pilot_wired(&data, pilots, keeper, |screen: AppScreen| match looks {
        Some(looks) => screen.with_ship_looks(looks),
        None => screen,
    });
    (game, data, art)
}

/// The borrowed stock frames `list` draws.
fn stock_sprites(list: &DrawList) -> Vec<nova_view::ImageKey> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, .. } | DrawCommand::Colorized { image, .. }
                if matches!(image.kind, ImageKind::StockShip { .. }) =>
            {
                Some(*image)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn with_the_stock_look_on_the_hauler_draws_its_borrowed_stock_sprites() {
    let game = fly(
        &MemoryPilots::new(),
        &MemorySettings::holding(LOOK_ON),
        true,
    );
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    let stock = stock_sprites(&list);
    assert!(
        stock.iter().any(|key| key.id == freighter()),
        "the freighter's frames: {stock:?}"
    );
    let player = view.camera().world_to_screen(view.shown_position());
    let livery = world_triangles(&list, GUILD);
    assert!(
        !livery.iter().any(|corners| boxes(corners, player)),
        "no livery round {player:?}: {livery:?}"
    );
}

#[test]
fn the_borrowed_frames_the_flight_draws_decode_through_the_game_s_images() {
    let (game, data, art) = fly_wired(
        &MemoryPilots::new(),
        &MemorySettings::holding(LOOK_ON),
        true,
    );
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    let key = *stock_sprites(&list)
        .iter()
        .find(|key| key.id == freighter())
        .expect("the freighter's frames");
    let images = images(&data, &Rulebook::default(), None, art);
    let frames = images.frames(key.kind, key.id);
    assert!(
        frames.as_ref().is_ok_and(|frames| !frames.is_empty()),
        "{key:?} decodes: {frames:?}"
    );
}

#[test]
fn without_the_stock_store_the_hauler_draws_its_stand_in_as_before() {
    let game = fly(
        &MemoryPilots::new(),
        &MemorySettings::holding(LOOK_ON),
        false,
    );
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    assert_eq!(stock_sprites(&list), []);
    let player = view.camera().world_to_screen(view.shown_position());
    assert!(
        world_triangles(&list, GUILD)
            .iter()
            .any(|corners| boxes(corners, player))
    );
}

#[test]
fn the_same_flight_saves_the_same_pilot_and_settings_with_the_look_on_or_off() {
    let run = |look: bool| {
        let (pilots, settings) = (MemoryPilots::new(), MemorySettings::holding(LOOK_ON));
        let mut game = fly(&pilots, &settings, look);
        for _ in 0..30 {
            draw(&mut game);
        }
        // Back to the menu, which saves the pilot, then quit.
        game.press(winit::keyboard::KeyCode::Escape);
        draw(&mut game);
        game.handle(WindowEvent::CloseRequested);
        (pilots.written(), settings.text(), settings.writes())
    };
    let off = run(false);
    let on = run(true);
    assert!(!off.0.is_empty(), "a pilot was saved");
    assert_eq!(on, off);
}
