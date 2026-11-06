//! New Pilot's pad focus (rdm:phase/key-bindings/phase-11-gamepad-menus).
//!
//! `_DoNewPilotNameDialog` keeps no control focus (`ModalDialog` with no
//! filter @0x16956; the name's selection @0x16802 is text focus), so the
//! pad's focus here is a new feature, built on the dialog's own
//! ([`Dialog`]'s pad module):
//!
//! - **Targets** are OK and Cancel, and, taken in
//!   ([`Dialog::set_pad_included`]), Strict Play, the Full Name and
//!   Nickname fields and the gender control, each while it is shown. The
//!   focus starts on OK, the default, so South at once accepts the drawn
//!   default name and nickname.
//! - **South** does what a click does: on a field it focuses the field
//!   with the caret at its end, and raises the on-screen keyboard to type
//!   into it (the `entry` module,
//!   rdm:phase/key-bindings/phase-17-text-entry-port), on Strict Play
//!   it flips the box, on the gender control it switches the gender.
//! - **The right shoulder** is Tab, which switches between the fields as
//!   Tab does; **the left** moves the pad's focus back.
//! - **East** is Escape: nothing by the engine's reading of
//!   `new_pilot_escape`, a cancel by the other.

use super::{Dialog, GENDER_ITEM, NAME_ITEM, NICKNAME_ITEM, STRICT_PLAY_ITEM};

/// The items besides the buttons that the pad's focus rests on: those
/// whose click the dialog acts on.
pub(super) const INCLUDED: [usize; 4] = [STRICT_PLAY_ITEM, NAME_ITEM, NICKNAME_ITEM, GENDER_ITEM];

/// Takes [`INCLUDED`] into `dialog`'s pad targets.
pub(super) fn include(dialog: &mut Dialog) {
    dialog.set_pad_included(INCLUDED.to_vec());
}

#[cfg(test)]
pub(crate) mod tests {
    use std::rc::Rc;

    use nova_sim::{Gender, RuleKey, RuleSource, Rulebook};

    use super::super::tests::stock;
    use super::super::{CANCEL_ITEM, Focus, NewPilotDialog, NewPilotOutcome, OK_ITEM};
    use super::*;
    use crate::geometry::Bounds;
    use crate::input::{Input, Key, MouseButton, PadButton};
    use crate::screen::Screen;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::button::ButtonStyle;
    use crate::ui::pad_focus::every_screen::{PAD, PadProbe, pad, told};

    fn dialog() -> NewPilotDialog {
        NewPilotDialog::new(&stock(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds")
            .with_defaults("Ada Lovelace", "Ada")
    }

    fn tap(dialog: &mut NewPilotDialog, button: PadButton) {
        dialog.input(&pad(button, true));
        dialog.input(&pad(button, false));
    }

    /// Steps the pad's focus back with the left shoulder until it is on
    /// `item`.
    fn focus_on(dialog: &mut NewPilotDialog, item: usize) {
        for _ in 0..8 {
            if dialog.dialog().pad_highlight() == Some(item) {
                return;
            }
            tap(dialog, PadButton::LeftShoulder);
        }
        panic!("the left shoulder never reaches item {item}");
    }

    fn padded() -> NewPilotDialog {
        let mut dialog = dialog();
        dialog.input(&told(PAD));
        dialog
    }

    #[test]
    fn the_pads_focus_starts_on_ok_and_south_creates_the_pilot_with_the_default_name() {
        let mut dialog = padded();
        assert_eq!(dialog.dialog().pad_highlight(), Some(OK_ITEM));
        tap(&mut dialog, PadButton::South);
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "Ada Lovelace".into(),
                nickname: "Ada".into(),
                gender: Gender::Male,
                strict_play: false,
            })
        );
    }

    #[test]
    fn south_on_the_name_or_nickname_field_focuses_it_as_a_click_does() {
        let mut dialog = padded();
        focus_on(&mut dialog, NICKNAME_ITEM);
        tap(&mut dialog, PadButton::South);
        assert_eq!(dialog.focus(), Focus::Nickname);
        let nickname = dialog.nickname_field().expect("a nickname field").clone();
        focus_on(&mut dialog, NAME_ITEM);
        tap(&mut dialog, PadButton::South);
        assert_eq!(dialog.focus(), Focus::Name);
        let mut clicked = self::dialog();
        click(&mut clicked, NICKNAME_ITEM);
        assert_eq!(
            clicked.nickname_field(),
            Some(&nickname),
            "caret at its end"
        );
        click(&mut clicked, NAME_ITEM);
        assert_eq!(clicked.field(), dialog.field());
        assert_eq!(dialog.take_outcome(), None, "nothing created");
    }

    fn click(dialog: &mut NewPilotDialog, item: usize) {
        let at = dialog.dialog().item_bounds(item).expect("an item").center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn south_on_strict_play_ticks_it_and_on_gender_switches_it() {
        let mut dialog = padded();
        focus_on(&mut dialog, STRICT_PLAY_ITEM);
        tap(&mut dialog, PadButton::South);
        assert!(dialog.strict_play());
        tap(&mut dialog, PadButton::South);
        assert!(!dialog.strict_play(), "and clears it again");
        focus_on(&mut dialog, GENDER_ITEM);
        tap(&mut dialog, PadButton::South);
        assert_eq!(dialog.gender(), Gender::Female);
    }

    #[test]
    fn the_right_shoulder_switches_fields_as_tab_does_and_the_left_moves_the_pads_focus_back() {
        let mut dialog = padded();
        tap(&mut dialog, PadButton::RightShoulder);
        assert_eq!(dialog.focus(), Focus::Nickname, "as Tab");
        assert_eq!(
            dialog.dialog().pad_highlight(),
            Some(OK_ITEM),
            "the pad's stays"
        );
        let mut tabbed = self::dialog();
        tabbed.input(&Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        });
        assert_eq!(tabbed.nickname_field(), dialog.nickname_field());
        tap(&mut dialog, PadButton::LeftShoulder);
        assert_eq!(
            dialog.dialog().pad_highlight(),
            Some(GENDER_ITEM),
            "back, wrapping"
        );
    }

    fn escape_rules(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::NewPilotEscape, source)
    }

    #[test]
    fn east_does_nothing_by_the_engines_reading_and_cancels_by_the_other() {
        let mut engine = dialog().with_rules(&escape_rules(RuleSource::Engine));
        engine.input(&told(PAD));
        tap(&mut engine, PadButton::East);
        assert_eq!(engine.take_outcome(), None);
        let mut other = dialog().with_rules(&escape_rules(RuleSource::Bible));
        other.input(&told(PAD));
        tap(&mut other, PadButton::East);
        assert_eq!(other.take_outcome(), Some(NewPilotOutcome::Cancel));
    }

    #[test]
    fn the_built_in_dialog_has_no_strict_play_target() {
        let mut dialog = NewPilotDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics));
        dialog.input(&told(PAD));
        let mut reached = Vec::new();
        for _ in 0..6 {
            reached.extend(dialog.dialog().pad_highlight());
            tap(&mut dialog, PadButton::RightShoulder);
            tap(&mut dialog, PadButton::LeftShoulder);
            tap(&mut dialog, PadButton::LeftShoulder);
        }
        reached.sort_unstable();
        reached.dedup();
        assert_eq!(
            reached,
            [OK_ITEM, CANCEL_ITEM, NAME_ITEM, NICKNAME_ITEM, GENDER_ITEM]
        );
    }

    #[test]
    fn untold_the_pads_moves_and_south_do_nothing() {
        let mut dialog = dialog();
        for button in [PadButton::DPadUp, PadButton::LeftShoulder, PadButton::South] {
            tap(&mut dialog, button);
        }
        assert_eq!(dialog.dialog().pad_highlight(), None);
        assert_eq!(dialog.take_outcome(), None);
        assert_eq!(dialog.gender(), Gender::Male);
    }

    /// New Pilot under the pad's probe.
    struct Probe(NewPilotDialog);

    impl PadProbe for Probe {
        fn screen(&mut self) -> &mut dyn Screen {
            &mut self.0
        }

        fn controls(&self) -> Vec<(&'static str, Bounds)> {
            [
                ("OK", OK_ITEM),
                ("Cancel", CANCEL_ITEM),
                ("Strict Play", STRICT_PLAY_ITEM),
                ("Full Name", NAME_ITEM),
                ("Nickname", NICKNAME_ITEM),
                ("Gender", GENDER_ITEM),
            ]
            .into_iter()
            .filter(|&(_, item)| self.0.dialog().item_shown(item))
            .map(|(name, item)| (name, self.0.dialog().item_bounds(item).expect("an item")))
            .collect()
        }

        fn outcome(&mut self) -> String {
            format!(
                "{:?} {:?} {:?} {:?}",
                self.0,
                self.0.focus(),
                self.0.field(),
                self.0.nickname_field()
            )
        }

        fn default_control(&self) -> Option<&'static str> {
            Some("OK")
        }

        /// The on-screen keyboard's drawing, which a safe-area change
        /// moves and touch shows, as it does a touch aid's.
        fn touch_aids_drawn(&mut self) -> Vec<crate::draw::DrawCommand> {
            crate::ui::edit_dialog::keyboard_drawn(&self.0)
        }
    }

    /// Stock "Create a new pilot:" with its default names.
    pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
        Box::new(Probe(dialog()))
    }

    /// The built-in "Create a new pilot:", which has no Strict Play.
    pub(crate) fn fallback_pad_probe() -> Box<dyn PadProbe> {
        Box::new(Probe(
            NewPilotDialog::fallback(ButtonStyle::STOCK, Rc::new(MonoMetrics))
                .with_defaults("Ada Lovelace", "Ada"),
        ))
    }
}
