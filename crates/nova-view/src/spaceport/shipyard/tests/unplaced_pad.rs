//! The shipyard without its dialog, with a pad last used: the fallback
//! prompt names the pad's buttons (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs).

use super::*;
use crate::input::{ControllerFamily, Device, Surroundings};

#[test]
fn without_the_dialog_it_names_the_buttons_with_a_pad_last_used() {
    let mut screen = ShipyardScreen::new(
        Err("no DLOG 1004".to_owned()),
        info_templates(),
        Ok(text_input_template()),
        shipyard(),
        art() as Rc<dyn ShipyardCatalog>,
        ButtonStyle::STOCK,
    );
    screen.input(&Input::Surroundings(Surroundings {
        device: Device::Gamepad(ControllerFamily::Generic),
    }));
    assert_eq!(
        drawn(&screen),
        [DrawCommand::Text {
            text: "Cannot show the shipyard: no DLOG 1004. Press South or East to close it."
                .to_owned(),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }]
    );
}
