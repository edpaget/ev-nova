//! A screen's touch aid lives only while that screen is in front
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): with player info,
//! mission info or a text open over the trade center, the outfitter or the
//! race, the screen's toggle is neither drawn nor tapped, and one armed
//! before the overlay opened is disarmed once it closes. The spaceport's
//! own play keys hide while whoever holds the spaceport covers it.

use super::*;
use crate::input::{Device, Surroundings};
use crate::spaceport::bar::BarScreen;
use crate::touch::testkit::Fingers;
use crate::ui::touch_aids::ModifierToggle;
use nova_sim_frame::gamble::GambleOpening;

/// A host of a toggle the spaceport holds: its name, how to build the
/// spaceport with it open, and its toggle there.
type Host = (
    &'static str,
    fn() -> SpaceportView,
    fn(&SpaceportView) -> &ModifierToggle,
);

/// An overlay the spaceport opens over its screens: its name, how to open
/// it, and how to close it.
type Overlay = (&'static str, fn(&mut SpaceportView), fn(&mut SpaceportView));

fn venues_informed() -> SpaceportView {
    let mut view = informed(venues());
    let _ = view.take_offer_request();
    view
}

fn trade() -> SpaceportView {
    let mut view = venues_informed();
    click_item(&mut view, 7);
    assert!(view.open_trade().is_some(), "trading");
    view
}

fn outfitter() -> SpaceportView {
    let mut view = venues_informed();
    click_item(&mut view, 8);
    assert!(view.open_outfitter().is_some(), "outfitting");
    view
}

fn race() -> SpaceportView {
    let mut view =
        venues_informed().with_race(Ok(super::super::gamble::tests::race_template()), None);
    click_item(&mut view, 11);
    view.open_gamble(GambleOpening::Race { asked_max: 5_000 });
    assert!(
        view.open_bar().and_then(BarScreen::race_screen).is_some(),
        "racing"
    );
    view
}

fn hosts() -> [Host; 3] {
    [
        ("trade center", trade, |view| {
            view.open_trade().expect("trading").touch_toggle()
        }),
        ("outfitter", outfitter, |view| {
            view.open_outfitter().expect("outfitting").touch_toggle()
        }),
        ("race", race, |view| {
            view.open_bar()
                .and_then(BarScreen::race_screen)
                .expect("racing")
                .touch_toggle()
        }),
    ]
}

fn overlays() -> [Overlay; 3] {
    [
        (
            "player info",
            |view| {
                view.open_player_info(pilot_info());
                assert!(view.player_info().is_some(), "player info open");
            },
            |view| {
                view.input(&key(Key::Escape));
                assert!(view.player_info().is_none(), "player info closed");
            },
        ),
        (
            "mission info",
            |view| {
                view.open_mission_info(actives(2));
                assert!(view.mission_info().is_some(), "mission info open");
            },
            |view| {
                view.input(&key(Key::Escape));
                assert!(view.mission_info().is_none(), "mission info closed");
            },
        ),
        (
            "a text",
            |view| {
                view.show_text("Hello.", None);
                assert!(view.mission_text().is_some(), "a text open");
            },
            |view| {
                view.input(&key(Key::Enter));
                assert!(view.mission_text().is_none(), "the text closed");
            },
        ),
    ]
}

fn touched(build: fn() -> SpaceportView) -> SpaceportView {
    let mut view = build();
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    view
}

/// What `toggle` draws alone.
fn alone(toggle: &ModifierToggle) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    toggle.draw(&mut list);
    list.iter().cloned().collect()
}

/// Whether `view` draws all of `toggle_drawing`.
fn draws(view: &SpaceportView, toggle_drawing: &[DrawCommand]) -> bool {
    let drawn = drawn(view);
    !toggle_drawing.is_empty() && toggle_drawing.iter().all(|command| drawn.contains(command))
}

fn tap(view: &mut SpaceportView, at: Point) {
    let mut fingers = Fingers::on(view);
    fingers.down(1, at);
    fingers.up(1, at);
}

#[test]
fn an_overlay_over_a_host_hides_its_toggle() {
    for (host, build, toggle) in hosts() {
        for (overlay, open, close) in overlays() {
            let mut view = touched(build);
            let drawing = alone(toggle(&view));
            assert!(draws(&view, &drawing), "{host}: drawn before");
            open(&mut view);
            assert!(!draws(&view, &drawing), "{host} under {overlay}: not drawn");
            assert!(!toggle(&view).shown(), "{host} under {overlay}: hidden");
            close(&mut view);
            assert!(toggle(&view).shown(), "{host} after {overlay}: shown");
            assert!(draws(&view, &drawing), "{host} after {overlay}: drawn");
        }
    }
}

#[test]
fn a_tap_where_a_covered_toggle_was_never_reaches_it() {
    for (host, build, toggle) in hosts() {
        for (overlay, open, close) in overlays() {
            let mut view = touched(build);
            let at = toggle(&view).rect().center();
            open(&mut view);
            tap(&mut view, at);
            assert!(!toggle(&view).armed(), "{host} under {overlay}: not armed");
            if view.player_info().is_some()
                || view.mission_info().is_some()
                || view.mission_text().is_some()
            {
                close(&mut view);
            }
            assert!(!toggle(&view).armed(), "{host} after {overlay}: not armed");
        }
    }
}

#[test]
fn an_armed_toggle_is_disarmed_by_an_overlay_over_its_host() {
    for (host, build, toggle) in hosts() {
        for (overlay, open, close) in overlays() {
            let mut view = touched(build);
            let at = toggle(&view).rect().center();
            tap(&mut view, at);
            assert!(toggle(&view).armed(), "{host}: armed");
            open(&mut view);
            close(&mut view);
            assert!(!toggle(&view).armed(), "{host} after {overlay}: disarmed");
        }
    }
}

#[test]
fn the_play_keys_and_the_screen_open_hide_while_the_spaceport_is_covered() {
    for (host, build, toggle) in hosts() {
        let mut view = touched(build);
        let at = toggle(&view).rect().center();
        tap(&mut view, at);
        assert!(view.play_keys().shown(), "{host}: the play keys shown");
        let mut keys = DrawList::new();
        crate::ui::touch_aids::host::draw(&view, &mut keys);
        let keys: Vec<DrawCommand> = keys.iter().cloned().collect();
        view.set_covered(true);
        assert!(!view.play_keys().shown(), "{host}: the play keys hidden");
        assert!(!draws(&view, &keys), "{host}: the play keys not drawn");
        assert!(!toggle(&view).shown(), "{host}: its toggle hidden");
        view.set_covered(false);
        assert!(view.play_keys().shown(), "{host}: the play keys back");
        assert!(toggle(&view).shown(), "{host}: its toggle back");
        assert!(!toggle(&view).armed(), "{host}: disarmed");
    }
}
