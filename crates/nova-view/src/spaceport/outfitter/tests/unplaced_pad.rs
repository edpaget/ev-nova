//! The outfitter without its dialog, with a pad last used: the fallback
//! prompt names the pad's buttons (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs).

use super::*;
use crate::input::{ControllerFamily, Device, Surroundings};

#[test]
fn without_the_dialog_it_names_the_buttons_with_a_pad_last_used() {
    let mut screen = OutfitterScreen::new(
        Err("no DLOG 1002".to_owned()),
        Ok(quantity()),
        Ok(desc_template()),
        outfitter(),
        art() as Rc<dyn OutfitterCatalog>,
        ButtonStyle::STOCK,
    );
    screen.input(&Input::Surroundings(Surroundings::of(Device::Gamepad(
        ControllerFamily::Generic,
    ))));
    assert_eq!(
        drawn(&screen),
        [DrawCommand::Text {
            text: "Cannot show the outfitter: no DLOG 1002. Press South or East to close it."
                .to_owned(),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }]
    );
}
