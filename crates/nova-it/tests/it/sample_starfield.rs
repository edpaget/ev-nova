//! The sample's starfield drawn through the whole app
//! (rdm:phase/play-the-sample/phase-5-stand-in-starfield): New Pilot over
//! the sample's cut puts the player in flight in Luyten Crossing, whose
//! data has no star sheet, and the frame draws the shown stars as the
//! data's stand-in dots, which scroll against the ship's travel by their
//! parallax.
//!
//! Wiring only: the dots are read back from the data through the view's
//! port (`CombatLooks::star_dots`), not retyped. The palette the loader
//! sets is nova-data's (`new_game::looks::STAR_DOTS`); how a dot is
//! placed, scrolled and drawn, nova-view's (`flight::stars`).
//!
//! The start is clear, so its dots draw at full level. The sample's
//! Marrow is murky (its `nebula`, `new_game::hazards`): one test flies
//! there from the start on the new game's gate autopilot, with the
//! player's settings file choosing the dots over the layered sky
//! (`layered_starfield`'s engine reading), and shows the dots drawn
//! dimmer there. How much dimmer stays nova-view's rule, pinned over
//! fakes in its `flight/view/tests/stars.rs` and `flight::stars`.

use std::rc::Rc;

use nova::app::{Control, WindowEvent};
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_audio::recording::MemorySettings;
use nova_data::GameData;
use nova_data::new_game;
use nova_data::new_game::fixture::sample_folder;
use nova_flight::SessionViewExt as _;
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{SystemId, Vec2};
use nova_view::flight::catalog::{CombatLooks, StarDot};
use nova_view::geometry::Bounds;
use nova_view::{Color, DrawCommand, DrawLayer, DrawList, Key, Screen};
use winit::keyboard::KeyCode;

use crate::common::{draw, fly_the_sample};
use crate::new_game_conversation::{new_pilot_flying, opened_from};
use crate::new_game_port_loop::{fly_the_autopilot, plot_course_to};
use crate::new_game_travel::nova_of;
use crate::pilots::Harness;

/// The data's stand-in dots, as the view reads them.
fn data_dots(data: &GameData) -> Vec<StarDot> {
    CombatLooks::star_dots(data)
        .expect("the sample's stars stand in as dots")
        .as_slice()
        .to_vec()
}

/// Every OR fill in the world layer of the view's frame drawn now: the
/// stars' dots, in draw order.
fn world_dots(game: &Harness) -> Vec<(Bounds, Color)> {
    let view = game.app.screen().flight_view().expect("flying");
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.layered()
        .filter_map(|(layer, command)| match (layer, command) {
            (DrawLayer::World, DrawCommand::OrFill { area, color }) => Some((*area, *color)),
            _ => None,
        })
        .collect()
}

/// The player's ship's position in the world now.
fn ship_position(game: &Harness) -> Vec2 {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player()
        .position
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

/// The player's ship's velocity now.
fn ship_velocity(game: &Harness) -> Vec2 {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .player()
        .velocity
}

#[test]
fn the_sample_draws_its_stars_as_the_data_s_dots_at_full_level() {
    let store = MemoryPilots::new();
    let (game, data) = fly_the_sample(&store);
    let dots = data_dots(&data);
    assert_full_level(&world_dots(&game), &dots);
}

/// Asserts that the stars draw, each as one of the data's `dots` at its
/// full colour.
fn assert_full_level(drawn: &[(Bounds, Color)], dots: &[StarDot]) {
    assert!(!drawn.is_empty(), "the stars draw");
    for &(area, color) in drawn {
        assert!(
            dots.iter()
                .any(|dot| dot.color == color && is_side_of(area, dot.side)),
            "{area:?} in {color:?} is none of the data's dots {dots:?}"
        );
    }
}

/// Whether `area` is a square of `side` pixels.
#[allow(clippy::float_cmp)]
fn is_side_of(area: Bounds, side: u8) -> bool {
    area.width() == f32::from(side) && area.height() == f32::from(side)
}

/// The new game over the sample's cut as `main` builds it, with a new
/// pilot flying in its start: the new game's profile, with the player's
/// settings file choosing the stand-in dots over the layered sky
/// (`layered_starfield`'s engine reading), and the Ink dialog loader.
fn fly_the_new_game_over_the_sample(store: &MemoryPilots) -> (Harness, Rc<GameData>) {
    let data = opened_from(&sample_folder());
    let mut settings = MemorySettings::holding(DOTS_OVER_THE_SKY);
    let (rules, warnings) =
        ruleset_rulebook(Some(&mut settings), Ruleset::NewGame.profile().readings);
    assert_eq!(warnings, Vec::<String>::new(), "the override reads");
    let game = Harness::wired(Rc::clone(&data), store, |screen| {
        screen
            .with_rulebook(&rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    });
    (new_pilot_flying(game), data)
}

/// A settings file choosing the stand-in dots over the layered sky.
const DOTS_OVER_THE_SKY: &str = r#"{"rule_overrides": {"layered_starfield": "engine"}}"#;

/// The sample's system `id`.
fn sample_system(id: &str) -> SystemId {
    let game = new_game::open(&sample_folder()).expect("the sample lists");
    SystemId(nova_of(&game.ids.systems, id))
}

/// The murk the session hands the frame now, which the view's stars read.
fn system_murk(game: &Harness) -> i16 {
    let mut frame = FlightFrame::default();
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
        .fill_frame(&mut frame);
    frame.system_murk
}

/// The most frames the stars may take to show again after a gate's exit.
const SETTLE_FRAMES: u32 = 600;

/// Flown from the clear start to Marrow on the new game's gate autopilot,
/// the stars draw as the data's dots dimmer than at full level: each in no
/// dot's full colour, and each a dot of its side with no channel brighter
/// and the sum of them darker. How much dimmer is nova-view's
/// (`flight::stars`), and Marrow's murk nova-data's mapping.
#[test]
fn the_sample_s_dots_draw_dimmer_in_murky_marrow_than_in_the_clear_start() {
    let store = MemoryPilots::new();
    let (mut game, data) = fly_the_new_game_over_the_sample(&store);
    let dots = data_dots(&data);
    assert_eq!(system_murk(&game), 0, "the start is clear");
    assert_full_level(&world_dots(&game), &dots);

    let marrow = sample_system("marrow");
    plot_course_to(&mut game, marrow);
    game.press(KeyCode::KeyJ);
    fly_the_autopilot(&mut game);
    assert_eq!(game.ship_session().pilot().system, marrow, "in Marrow");

    let drawn = (0..SETTLE_FRAMES)
        .find_map(|_| {
            draw(&mut game);
            let drawn = world_dots(&game);
            (!drawn.is_empty()).then_some(drawn)
        })
        .expect("the stars show again");
    let rgb = |color: Color| [color.r, color.g, color.b];
    for (area, color) in drawn {
        assert!(
            dots.iter().all(|dot| rgb(dot.color) != rgb(color)),
            "{area:?} in {color:?} is at a dot's full colour"
        );
        assert!(
            dots.iter().any(|dot| {
                let (full, now) = (rgb(dot.color), rgb(color));
                is_side_of(area, dot.side)
                    && full.iter().zip(&now).all(|(full, now)| now <= full)
                    && now.iter().map(|&c| u32::from(c)).sum::<u32>()
                        < full.iter().map(|&c| u32::from(c)).sum::<u32>()
            }),
            "{area:?} in {color:?} is no dimmed dot of {dots:?}"
        );
    }
    assert!(system_murk(&game) > 0, "Marrow's murk reaches the frame");
}

/// A star moves on screen by its factor of the ship's move less the move
/// itself (`flight::stars`' scroll and placement): against the travel, by
/// between nothing and the whole move. Dots that never scroll fail: at
/// least one must move.
#[test]
fn the_sample_s_dots_scroll_against_the_ship_s_travel() {
    let store = MemoryPilots::new();
    let (mut game, _) = fly_the_sample(&store);
    // Up to speed, then coasting, so the drawn frame's lag behind the
    // session is the same at both ends of the measure. Up is pressed
    // again each frame until it takes: a press sent as flight first shows
    // doesn't thrust.
    for _ in 0..2000 {
        if ship_velocity(&game).length() > 0.0 {
            break;
        }
        thrust(&mut game, true);
        draw(&mut game);
    }
    assert!(ship_velocity(&game).length() > 0.0, "Up never thrusts");
    for _ in 0..120 {
        draw(&mut game);
    }
    thrust(&mut game, false);
    for _ in 0..5 {
        draw(&mut game);
    }
    let (from, before) = (ship_position(&game), world_dots(&game));
    for _ in 0..20 {
        draw(&mut game);
    }
    let (to, after) = (ship_position(&game), world_dots(&game));
    let travel = to - from;
    assert!(travel.length() > 10.0, "the ship travels: {travel:?}");
    assert_eq!(before.len(), after.len(), "the same stars shown");

    // A wrapped dot jumps the play area's width or height, far more than
    // the travel; the others move with it. A pixel each way is the
    // placement's truncation.
    let slack = 1.5;
    let mut moved = 0;
    for ((was, _), (now, _)) in before.iter().zip(&after) {
        let shift = Vec2::new(
            now.center().x - was.center().x,
            now.center().y - was.center().y,
        );
        if shift.length() > 200.0 {
            continue;
        }
        // The shift's parts along the travel and across it.
        let along = (shift.x * travel.x + shift.y * travel.y) / travel.length();
        let across = (shift.y * travel.x - shift.x * travel.y) / travel.length();
        assert!(
            along <= slack && -along <= travel.length() + slack,
            "{shift:?} against {travel:?}: between 0 and the travel"
        );
        assert!(across.abs() <= slack, "{shift:?} across {travel:?}");
        if shift.length() > slack {
            moved += 1;
        }
    }
    assert!(moved > 0, "some dot scrolls");
}
