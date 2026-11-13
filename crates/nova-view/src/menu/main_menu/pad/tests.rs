use std::rc::Rc;
use std::time::Duration;

use nova_sim_frame::mission::screens::mac_ticks;

use super::*;
use crate::draw::{DrawCommand, DrawList};
use crate::geometry::Point;
use crate::input::{Key, PadButton};
use crate::menu::fixture::{BUTTON_SPRITES, ROLLOVER_SPRITE, main_screen_art};
use crate::menu::{Chosen, ChosenBy};
use crate::screen::Screen;
use crate::sound::{Sound, UiSound};
use crate::text::fixture::MonoMetrics;
use crate::ui::ButtonStyle;
use crate::ui::pad_focus::every_screen::{PAD, PadProbe, highlights, pad, pointer, told};

const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);
const UP: Sound = Sound::Ui(UiSound::ButtonUp);

fn with_art() -> MainMenu {
    MainMenu::new(
        Some(main_screen_art()),
        ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    )
}

fn stand_in() -> MainMenu {
    MainMenu::new(None, ButtonStyle::STOCK, Rc::new(MonoMetrics))
}

fn padded(mut menu: MainMenu) -> MainMenu {
    menu.input(&told(PAD));
    menu
}

fn tap(menu: &mut MainMenu, button: PadButton) {
    menu.input(&pad(button, true));
    menu.input(&pad(button, false));
}

fn chosen(choice: MenuChoice) -> Chosen {
    Chosen {
        choice,
        by: ChosenBy::Pointer,
    }
}

fn drawn(menu: &MainMenu) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    menu.draw(&mut list);
    list.iter().cloned().collect()
}

#[test]
fn told_the_pad_the_focus_starts_on_new_pilot() {
    for menu in [with_art(), stand_in()] {
        let menu = padded(menu);
        assert_eq!(menu.pad_highlight(), Some(MenuChoice::NewPilot));
        assert_eq!(menu.device(), PAD);
    }
    assert_eq!(with_art().pad_highlight(), None, "untold");
}

#[test]
fn the_dpad_moves_between_the_arts_two_columns_and_the_stand_ins_one() {
    let mut menu = padded(with_art());
    tap(&mut menu, PadButton::DPadRight);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::EnterShip));
    tap(&mut menu, PadButton::DPadDown);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::Preferences));
    tap(&mut menu, PadButton::DPadLeft);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::OpenPilot));
    let mut menu = padded(stand_in());
    tap(&mut menu, PadButton::DPadRight);
    assert_eq!(
        menu.pad_highlight(),
        Some(MenuChoice::NewPilot),
        "one column"
    );
    for _ in 0..5 {
        tap(&mut menu, PadButton::DPadDown);
    }
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::About));
    tap(&mut menu, PadButton::LeftShoulder);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::Preferences));
}

#[test]
fn south_presses_with_600_and_its_release_chooses_with_601_as_a_click_does() {
    let mut menu = padded(with_art());
    tap(&mut menu, PadButton::DPadDown);
    menu.input(&pad(PadButton::South, true));
    assert_eq!(menu.take_sounds(), [DOWN]);
    assert_eq!(menu.take_choice(), None, "not on the press");
    assert_eq!(
        format!("{menu:?}"),
        format!("{:?}", {
            let mut clicked = with_art();
            clicked.input(&pointer(true, clicked.press_point(MenuChoice::OpenPilot)));
            clicked
        }),
        "drawn pressed as a held click is"
    );
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_sounds(), [UP]);
    assert_eq!(menu.take_choice(), Some(chosen(MenuChoice::OpenPilot)));
}

#[test]
fn south_finishes_the_slides_as_a_primary_press_does() {
    let mut menu = with_art();
    menu.appear();
    menu.input(&told(PAD));
    menu.tick(mac_ticks(4));
    assert!(menu.row_shown(0) && menu.slides_running());
    menu.take_sounds();
    menu.input(&pad(PadButton::South, true));
    assert!(!menu.slides_running());
    assert_eq!(menu.take_sounds(), [DOWN], "the slides end silently");
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_choice(), Some(chosen(MenuChoice::NewPilot)));
}

#[test]
fn a_row_still_sliding_has_no_target() {
    let mut menu = with_art();
    menu.appear();
    menu.input(&told(PAD));
    assert_eq!(menu.pad_highlight(), None, "nothing shows yet");
    tap(&mut menu, PadButton::South);
    assert_eq!(menu.take_choice(), None);
    assert!(
        menu.slides_running(),
        "a South on nothing shown does nothing"
    );
    menu.tick(mac_ticks(4));
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::NewPilot));
    tap(&mut menu, PadButton::DPadDown);
    tap(&mut menu, PadButton::DPadDown);
    assert_eq!(
        menu.pad_highlight(),
        Some(MenuChoice::OpenPilot),
        "Quit's row still slides"
    );
}

#[test]
fn a_move_drops_a_held_south_silently() {
    let mut menu = padded(with_art());
    menu.input(&pad(PadButton::South, true));
    menu.take_sounds();
    tap(&mut menu, PadButton::DPadDown);
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_sounds(), []);
    assert_eq!(menu.take_choice(), None);
}

#[test]
fn a_pointer_press_cancels_the_pads_press() {
    let mut menu = padded(with_art());
    menu.input(&pad(PadButton::South, true));
    let quit = menu.press_point(MenuChoice::Quit);
    menu.input(&pointer(true, quit));
    menu.input(&pad(PadButton::South, false));
    assert_eq!(
        menu.take_choice(),
        None,
        "South's release lets go of nothing"
    );
    menu.input(&pointer(false, quit));
    assert_eq!(menu.take_choice(), Some(chosen(MenuChoice::Quit)));
}

#[test]
fn the_pointer_moving_leaves_the_pads_press_alone() {
    let mut menu = padded(with_art());
    menu.input(&pad(PadButton::South, true));
    menu.take_sounds();
    menu.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
    assert_eq!(menu.take_sounds(), [], "no 601 off the button");
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_choice(), Some(chosen(MenuChoice::NewPilot)));
}

#[test]
fn a_release_the_menu_never_saw_pressed_does_nothing() {
    let mut menu = padded(with_art());
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_choice(), None);
    let quit = menu.press_point(MenuChoice::Quit);
    menu.input(&pointer(true, quit));
    menu.take_sounds();
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_choice(), None, "the pointer's press is its own");
    assert_eq!(menu.take_sounds(), []);
}

#[test]
fn east_and_keyboard_space_do_nothing() {
    let mut menu = padded(with_art());
    tap(&mut menu, PadButton::East);
    for pressed in [true, false] {
        menu.input(&Input::Key {
            key: Key::Space,
            pressed,
            repeat: false,
        });
    }
    assert_eq!(menu.take_choice(), None);
    assert_eq!(menu.take_sounds(), []);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::NewPilot));
}

#[test]
fn the_highlight_is_drawn_only_while_the_pad_was_used_last() {
    let new_pilot = with_art().button_bounds(MenuChoice::NewPilot);
    let mut menu = padded(with_art());
    assert!(highlights(&drawn(&menu), new_pilot));
    menu.input(&told(crate::input::Device::Mouse));
    assert!(!highlights(&drawn(&menu), new_pilot));
    assert_eq!(drawn(&menu), drawn(&with_art()), "as it always drew");
}

#[test]
fn the_highlight_is_drawn_over_the_buttons_and_under_the_rollover() {
    let mut menu = padded(with_art());
    menu.set_pointer(Some(menu.press_point(MenuChoice::About)));
    menu.tick(Duration::ZERO);
    menu.tick(mac_ticks(1));
    let commands = drawn(&menu);
    let picture = |id: i16| {
        commands
            .iter()
            .rposition(|command| match command {
                DrawCommand::Picture { image, .. } | DrawCommand::Sprite { image, .. } => {
                    image.id == id
                }
                _ => false,
            })
            .expect("drawn")
    };
    let corner = menu.button_bounds(MenuChoice::NewPilot).grown(2.0).min;
    let ring = commands
        .iter()
        .position(|command| matches!(*command, DrawCommand::Line { from, .. } if from == corner))
        .expect("the highlight");
    assert!(picture(BUTTON_SPRITES[5]) < ring, "over the buttons");
    assert!(ring < picture(ROLLOVER_SPRITE), "under the rollover");
}

#[test]
fn appearing_again_keeps_the_focus() {
    let mut menu = padded(with_art());
    tap(&mut menu, PadButton::DPadRight);
    menu.appear();
    menu.tick(mac_ticks(10));
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::EnterShip));
}

/// The main menu under the pad's probe.
struct Probe(MainMenu);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        MenuChoice::ALL
            .into_iter()
            .map(|choice| (choice.label(), self.0.button_bounds(choice)))
            .collect()
    }

    fn outcome(&mut self) -> String {
        format!("{:?}", self.0.take_choice())
    }

    /// A press on one of the button's opaque pixels: its rect's centre
    /// can miss its mask.
    fn click(&self, name: &'static str, _rect: Bounds) -> Vec<Input> {
        let choice = MenuChoice::ALL
            .into_iter()
            .find(|choice| choice.label() == name)
            .expect("a button");
        let at = self.0.press_point(choice);
        vec![pointer(true, at), pointer(false, at)]
    }
}

/// The main menu drawn from the fixture art.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(with_art()))
}

/// The main menu's stand-in.
pub(crate) fn stand_in_pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(stand_in()))
}

/// The stock art's left buttons sit staggered, so a button in the row
/// above can lie nearer to the right of Quit than About does; each left
/// and right moves to its row-mate all the same.
#[test]
fn left_and_right_move_to_the_row_mate_where_the_columns_stagger() {
    let mut art = main_screen_art();
    art.buttons[0].position = Point::new(103.0, 50.0);
    art.buttons[5].position = Point::new(197.0, 70.0);
    let menu = || {
        padded(MainMenu::new(
            Some(art.clone()),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        ))
    };
    for (from, right) in [
        (MenuChoice::NewPilot, MenuChoice::EnterShip),
        (MenuChoice::OpenPilot, MenuChoice::Preferences),
        (MenuChoice::Quit, MenuChoice::About),
    ] {
        let mut menu = menu();
        menu.focus.set_by_pad(from);
        tap(&mut menu, PadButton::DPadRight);
        assert_eq!(menu.pad_highlight(), Some(right), "{from:?}");
        tap(&mut menu, PadButton::DPadLeft);
        assert_eq!(menu.pad_highlight(), Some(from), "{right:?}");
    }
}

#[test]
fn the_pointer_leaving_leaves_the_pads_press_alone() {
    let mut menu = padded(with_art());
    menu.input(&pad(PadButton::South, true));
    menu.take_sounds();
    menu.input(&Input::PointerLeft);
    assert_eq!(menu.take_sounds(), [], "no 601 off the button");
    menu.input(&pad(PadButton::South, false));
    assert_eq!(menu.take_choice(), Some(chosen(MenuChoice::NewPilot)));
}

#[test]
fn the_pad_is_read_through_the_bindings_given() {
    use crate::keymap::{Bindings, Command, PadBinding, PadEntry, SharedBindings, UiCommand};
    let mut bindings = Bindings::default();
    bindings.gamepad.bind(
        Command::Ui(UiCommand::FocusRight),
        PadBinding::Entry(PadEntry::plain(PadButton::North)),
    );
    let mut menu = padded(with_art().with_bindings(SharedBindings::new(bindings)));
    tap(&mut menu, PadButton::North);
    assert_eq!(menu.pad_highlight(), Some(MenuChoice::EnterShip));
}
