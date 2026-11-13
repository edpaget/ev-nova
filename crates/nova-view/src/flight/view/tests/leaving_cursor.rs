//! Flight's cursor across a landing and a gate's entry, on [`FakeFlight`]:
//! `_PlayerLandOnStellar` (@0x6326f) and `_PlayerEnterHypergate`
//! (@0x639b1) call `_HideCursorSprite` on every accepted landing or entry,
//! movies on or off, so back in flight a same-point pointer doesn't show
//! it. `_PlayerEnterWormhole` calls none. Kept apart from `tests/mod.rs`,
//! which is large.

use super::faked_landing::{arrived, at_gate, came_out_in_alpha};
use super::faked_movies::{
    ENTER_ALPHA, at_the_filmed_hypergate, filmed, filming, land_on_mars, run_out,
};
use super::*;
use nova_sim_frame::{GateKind, GateRefusal};

/// Where the pointer rests before the landing or entry.
const AT: Point = Point::new(40.0, 400.0);
/// Somewhere else.
const OTHER: Point = Point::new(60.0, 420.0);

/// Shows `view`'s cursor at [`AT`].
fn shown_at(view: &mut Faked) {
    view.input(&Input::PointerMoved(AT));
    view.tick(TICK);
    assert_eq!(view.cursor_at(), Some(AT));
}

/// Back in flight, as the router's `switch_to` cancels the pointer: a
/// same-point move keeps the cursor hidden and another point shows it.
fn hidden_until_it_moves(view: &mut Faked) {
    view.cancel_pointer();
    view.input(&Input::PointerMoved(AT));
    view.tick(TICK);
    assert_eq!(view.cursor_at(), None, "a same-point move");
    view.input(&Input::PointerMoved(OTHER));
    assert_eq!(view.cursor_at(), Some(OTHER));
}

#[test]
fn a_landing_hides_the_cursor_with_a_zero_fade_movies_off() {
    let (mut view, _) = filming(filmed(&[129], &[]), false);
    shown_at(&mut view);
    land_on_mars(&mut view);
    assert_eq!(view.cursor_at(), None, "hidden from the press (@0x6326f)");
    hidden_until_it_moves(&mut view);
}

#[test]
fn a_landing_movie_hides_the_cursor_from_the_press() {
    let (mut view, _) = filming(filmed(&[129], &[]), true);
    shown_at(&mut view);
    land_on_mars(&mut view);
    assert!(view.playing());
    assert_eq!(view.cursor_at(), None, "hidden under the movie");
    run_out(&mut view);
    hidden_until_it_moves(&mut view);
}

/// At the filmed hypergate with movies `on`, the cursor the map hid
/// relit under it by a move away and back to [`AT`].
fn relit_under_the_map(on: bool) -> Faked {
    let (mut view, _) = at_the_filmed_hypergate(filmed(&[131], &[]), on);
    view.input(&Input::PointerMoved(AT));
    view.input(&Input::PointerMoved(OTHER));
    view.input(&Input::PointerMoved(AT));
    assert!(view.map_open());
    assert_eq!(view.cursor_at(), Some(AT), "relit under the map");
    view
}

#[test]
fn a_hypergate_entry_hides_the_cursor_the_map_relit() {
    let mut view = relit_under_the_map(true);
    view.sim
        .queue_screen(ScreenAnswer::GateEntryDesc(Some(131)));
    view.close_map();
    assert!(view.playing());
    assert_eq!(view.cursor_at(), None, "hidden under the movie (@0x639b1)");
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    view.sim.queue_answer(arrived());
    run_out(&mut view);
    assert_eq!(
        super::faked_travel::sent_but_lines(&view).last(),
        Some(&ENTER_ALPHA)
    );
    hidden_until_it_moves(&mut view);
}

#[test]
fn a_hypergate_entry_hides_the_cursor_the_map_relit_movies_off() {
    let mut view = relit_under_the_map(false);
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    view.sim.queue_answer(arrived());
    view.close_map();
    assert_eq!(view.cursor_at(), None, "hidden on the entry (@0x639b1)");
    hidden_until_it_moves(&mut view);
}

#[test]
fn a_refused_hypergate_entry_leaves_the_cursor_as_the_map_left_it() {
    let mut view = relit_under_the_map(true);
    view.sim.queue_screen(ScreenAnswer::GateEntryDesc(None));
    view.sim
        .queue_answer(Answer::GateRefused(GateRefusal::Cancelled));
    view.close_map();
    assert!(!view.playing());
    assert_eq!(
        view.cursor_at(),
        Some(AT),
        "no hide past a refusal (@0x6392c)"
    );
}

/// L pressed over Sol's cleared wormhole (300), which passes through.
fn into_the_wormhole(view: &mut Faked, on: bool) {
    if on {
        view.sim
            .queue_screen(ScreenAnswer::GateEntryDesc(Some(300)));
    }
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    view.sim.queue_answer(arrived());
    view.land();
}

#[test]
fn a_wormhole_entry_keeps_the_cursor() {
    for on in [false, true] {
        let (mut view, _) = filming(filmed(&[300], &[]), on);
        shown_at(&mut view);
        into_the_wormhole(&mut view, on);
        assert_eq!(view.playing(), on);
        assert_eq!(view.cursor_at(), Some(AT), "no hide (movies {on})");
    }
}
