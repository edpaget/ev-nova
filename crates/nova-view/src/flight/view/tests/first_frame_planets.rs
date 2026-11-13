//! What the renderer's planets rely on in the flight
//! (rdm:phase/rendering-3d/phase-11-new-game-world-load-time): a planet is
//! baked and uploaded in the first frame whose draw list commands it, so
//! which frame that is decides where the pause falls.
//!
//! - The stellars' pass commands a planet however far off screen its
//!   stand-in is: nothing culls it. So a system's planets are all commanded
//!   in the first frame drawn there, not one by one as each scrolls into
//!   view.
//! - The first frame drawn after an arrival commands the new system's
//!   planets under the arrival's solid white: a jump's and a hypergate's,
//!   fading in from white with the Hyperspace Effects preference on
//!   ([`JumpPhase::FadeIn`] at its start) and flashing white with it off
//!   ([`JumpPhase::Flash`]).
//!
//! On [`FakeFlight`], with a hand-built table, as `stellar_models` is.

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_sim::flight_frame::{FlightEvent, ShipPlace};
use nova_sim::{SessionEvent, SimMessage};

use super::faked_landing::{arrived, at_the_hypergate_in, close_on};
use super::faked_travel::{arrival_report, jump_now, now_shows, step_the_arrival};
use super::faked_world::{world, world_list};
use super::*;
use crate::flight::jump::JumpEffect;
use crate::flight::world_models::{StellarModel, WorldModels};
use crate::outline::StellarStandIn;
use crate::planet::PlanetKey;

const GIANT: PlanetKey = PlanetKey(3);
/// A generated planet, with no sheet: in Sol or in Alpha Centauri.
const PLANET: i16 = 141;

/// The planet at `at`, standing in as a disc.
fn planet_at(at: (i16, i16)) -> StellarContents {
    StellarContents {
        sprite: Err(format!("no spïn for {PLANET}")),
        stand_in: Some(StellarStandIn::Disc(Color::WHITE)),
        ..crate::flight::fake_catalog::stellar(PLANET, "Kestrel Reach", at, 1)
    }
}

/// The table naming the planet's key.
fn table() -> Rc<WorldModels> {
    Rc::new(WorldModels {
        stellars: BTreeMap::from([(StellarId(PLANET), StellarModel::planet(GIANT, 2.3))]),
        ..WorldModels::default()
    })
}

/// Where `list` commands the planet, and at what pose.
fn planet_in(list: &DrawList) -> Option<(usize, crate::planet::PlanetPose)> {
    list.iter().enumerate().find_map(|(at, c)| match c {
        DrawCommand::Planet { planet, pose } if *planet == GIANT => Some((at, *pose)),
        _ => None,
    })
}

#[test]
fn the_stellars_pass_commands_a_planet_far_off_screen() {
    // The player at Sol's centre, the planet 25,000 pixels away.
    let catalog = FakeCatalog {
        sol_more: vec![planet_at((20_000, -15_000))],
        ..catalog()
    };
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame.stellars.push(record::stellar(StellarId(PLANET)));
    let view = world(catalog, frame).with_world_models(table());
    let (_, pose) = planet_in(&world_list(&view)).expect("commanded all the same");
    let (width, height) = view.view;
    assert!(
        pose.center.x > 10.0 * width && pose.center.y < -10.0 * height,
        "far outside the {width} by {height} view: {:?}",
        pose.center
    );
}

/// Alpha Centauri with the planet beside Proxima.
fn alpha_with_a_planet() -> FakeCatalog {
    FakeCatalog {
        alpha_more: vec![planet_at((300, 200))],
        ..catalog()
    }
}

/// The flight arrived in Alpha Centauri from now on, the planet shown,
/// and the session's events of the system entered and `message` drained
/// next.
fn arrives_in_alpha(view: &mut Faked, message: SimMessage) {
    let mut frame = in_alpha();
    frame.stellars.push(record::stellar(StellarId(PLANET)));
    view.sim.set_frame(frame);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim.queue_event(FlightEvent::Message(message));
}

/// Checks that the frame `view` draws now commands the planet and covers
/// it, and everything else, in solid white.
fn draws_the_planet_under_solid_white(view: &Faked, when: &str) {
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    let mut list = DrawList::new();
    Screen::draw(view, &mut list);
    let (planet, _) = planet_in(&list).unwrap_or_else(|| panic!("{when}: the planet"));
    let (white, color) = fade(&list).unwrap_or_else(|| panic!("{when}: the white"));
    assert_eq!(color, Color::WHITE, "{when}: solid");
    assert!(planet < white, "{when}: the white is drawn over it");
    assert_eq!(white, list.len() - 1, "{when}: over everything");
}

/// A view in Sol, jumped, whose warp has just ended in Alpha Centauri:
/// the step that reports the arrival has run, and no frame is drawn yet.
fn arrived_by_jump(fades: bool) -> Faked {
    let mut view = faked_in(alpha_with_a_planet(), sol())
        .with_hyperspace_effects(fades)
        .with_world_models(table());
    assert_eq!(planet_in(&world_list(&view)), None, "none in Sol");
    jump_now(&mut view);
    now_shows(&mut view, under_way(true));
    view.sim
        .queue_report(arrival_report(0, ShipPlace::default()));
    arrives_in_alpha(&mut view, SimMessage::Arrived(SystemId(131)));
    step_the_arrival(&mut view);
    view
}

#[test]
fn the_first_frame_after_a_jump_s_arrival_draws_the_new_system_s_planet_under_the_fade_s_white() {
    let view = arrived_by_jump(true);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    draws_the_planet_under_solid_white(&view, "fading in");
}

#[test]
fn with_no_fades_the_first_frame_after_a_jump_s_arrival_draws_it_under_the_flash() {
    let mut view = arrived_by_jump(false);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash)
    );
    draws_the_planet_under_solid_white(&view, "the step's own frame");
    // The frame the tick presents is that white one; the next shows the
    // system, its planet commanded already.
    view.present_jump_effect();
    draws_the_planet_under_solid_white(&view, "presented");
}

/// A view at Sol's hypergate that has just come out of Alpha Centauri's:
/// the map closed on its pick, and no frame is drawn yet.
fn come_out_of_the_gate(fades: bool) -> Faked {
    let mut view = at_the_hypergate_in(alpha_with_a_planet())
        .with_hyperspace_effects(fades)
        .with_world_models(table());
    arrives_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    close_on(&mut view, Some(SystemId(131)), arrived());
    view
}

#[test]
fn the_first_frame_out_of_a_hypergate_draws_the_new_system_s_planet_under_the_fade_s_white() {
    let view = come_out_of_the_gate(true);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    draws_the_planet_under_solid_white(&view, "fading in");
}

/// The system the frame `view` draws now names on its list
/// ([`DrawList::system`]), so the renderer counts a new visit there.
fn named(view: &Faked) -> Option<SystemId> {
    let mut list = DrawList::new();
    Screen::draw(view, &mut list);
    assert!(planet_in(&list).is_some(), "the frame commands the planet");
    list.system()
}

#[test]
fn the_first_frame_after_a_jump_names_the_new_system() {
    for fades in [true, false] {
        // Sol before the jump.
        let view = faked_in(alpha_with_a_planet(), sol())
            .with_hyperspace_effects(fades)
            .with_world_models(table());
        let mut list = DrawList::new();
        Screen::draw(&view, &mut list);
        assert_eq!(list.system(), Some(SystemId(130)), "Sol, fades {fades}");
        let arrived = arrived_by_jump(fades);
        assert_eq!(
            named(&arrived),
            Some(SystemId(131)),
            "jumped, fades {fades}"
        );
        let gated = come_out_of_the_gate(fades);
        assert_eq!(named(&gated), Some(SystemId(131)), "gated, fades {fades}");
    }
}

#[test]
fn with_no_fades_the_first_frame_out_of_a_hypergate_draws_it_under_the_flash() {
    // The close comes in input, so the next tick's frame is the white's.
    let mut view = come_out_of_the_gate(false);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash)
    );
    draws_the_planet_under_solid_white(&view, "after the close");
    view.tick(ms(1));
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash)
    );
    draws_the_planet_under_solid_white(&view, "the tick after the close");
}
