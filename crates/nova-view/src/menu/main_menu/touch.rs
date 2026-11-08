//! The main screen's rollover under a finger
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): a finger down is
//! the pointer, so the rollover names the button under it while it is
//! down; its lift is the pointer leaving, so the rollover fades to its
//! none frame instead of naming the button the finger last rested on.
//!
//! These drive the real touch router through [`Fingers`].

use std::rc::Rc;

use nova_sim::mission::screens::mac_ticks;

use super::*;
use crate::menu::fixture::main_screen_art;
use crate::screen::Screen;
use crate::text::fixture::MonoMetrics;
use crate::touch::testkit::Fingers;

fn with_art() -> MainMenu {
    MainMenu::new(
        Some(main_screen_art()),
        ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    )
}

/// A point on no button.
const MISS: Point = Point::new(5.0, 5.0);

/// The fixture's none frame, past its six buttons'.
const NONE: u16 = 6;

#[test]
fn a_finger_held_on_a_button_shows_its_rollover() {
    // Passes when written: phase 14 made the finger the pointer.
    let mut menu = with_art();
    let quit = menu.press_point(MenuChoice::Quit);
    let mut fingers = Fingers::on(&mut menu);
    fingers.down(1, quit);
    let menu = fingers.screen();
    menu.tick(Duration::ZERO);
    menu.tick(mac_ticks(1));
    assert_eq!(menu.rollover(), Some((2, 4)), "Quit's frame, fading in");
    menu.tick(mac_ticks(1));
    assert_eq!(menu.rollover(), Some((2, 8)));
}

#[test]
fn a_finger_lifted_off_the_buttons_clears_the_rollover() {
    // Pressed on a miss, slid onto Quit and lifted there: nothing is
    // chosen, and the rollover no longer names Quit.
    let mut menu = with_art();
    let quit = menu.press_point(MenuChoice::Quit);
    let mut fingers = Fingers::on(&mut menu);
    fingers.down(1, MISS);
    fingers.move_to(1, quit);
    fingers.screen().tick(mac_ticks(20));
    assert_eq!(fingers.screen().rollover(), Some((2, 32)), "named");
    fingers.up(1, quit);
    let menu = fingers.screen();
    assert_eq!(menu.take_choice(), None, "nothing chosen");
    menu.tick(mac_ticks(20));
    assert_eq!(menu.rollover().map(|(frame, _)| frame), Some(NONE));
}

#[test]
fn a_finger_pressed_on_a_button_slid_off_and_lifted_chooses_nothing() {
    let mut menu = with_art();
    let quit = menu.press_point(MenuChoice::Quit);
    let mut fingers = Fingers::on(&mut menu);
    fingers.down(1, quit);
    fingers.move_to(1, MISS);
    fingers.up(1, MISS);
    let menu = fingers.screen();
    assert_eq!(menu.take_choice(), None);
    menu.tick(mac_ticks(20));
    assert_eq!(menu.rollover().map(|(frame, _)| frame), Some(NONE));
}
