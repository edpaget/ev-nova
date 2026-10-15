//! The quantity dialog as the exchange and the outfitter host it: a
//! [`ModalHost`] over a [`QuantityDialog`], plus the rules the two share.
//!
//! [`QuantityAsk`] opens the dialog (`DLOG` 1003, or its fallback) at the
//! most a trade allows, OK titled Buy or Sell and asking [`PROMPT`], and
//! drops the typed key when B or S opened it. It follows Alt
//! ([`MAX_LOT_KEY`]), which is held to ask for a count, both while the
//! dialog is open and, through [`QuantityAsk::follow_alt`], while it is
//! not. A count above 0 confirmed there is given back with what it
//! counts; 0 or Cancel closes it with nothing.

use std::rc::Rc;

use nova_sim::Direction;

use super::modal::ModalHost;
use super::trade::{BUY_LABEL, MAX_LOT_KEY, SELL_LABEL};
use crate::draw::DrawList;
use crate::input::Input;
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::ui::quantity::{PROMPT, QuantityDialog};

/// The quantity dialog over a screen, what it counts (`T`), and whether
/// Alt is held.
#[derive(Clone, Debug)]
pub struct QuantityAsk<T> {
    host: ModalHost<T, QuantityDialog>,
    /// The dialog's template, if the interface file has one.
    template: Option<DialogTemplate>,
    style: ButtonStyle,
    /// Whether Alt is held.
    alt: bool,
}

impl<T> QuantityAsk<T> {
    /// Nothing open and Alt not held. The dialog is laid out by
    /// `template` (or, without one, its fallback), its buttons labelled in
    /// `style`.
    #[must_use]
    pub fn new(template: Option<DialogTemplate>, style: ButtonStyle) -> Self {
        Self {
            host: ModalHost::default(),
            template,
            style,
            alt: false,
        }
    }

    /// Follows [`MAX_LOT_KEY`] presses, repeats and releases.
    pub fn follow_alt(&mut self, input: &Input) {
        if let Input::Key {
            key: MAX_LOT_KEY,
            pressed,
            ..
        } = *input
        {
            self.alt = pressed;
        }
    }

    /// Whether Alt is held.
    #[must_use]
    pub fn alt_held(&self) -> bool {
        self.alt
    }

    /// Opens the dialog over the screen for `payload`: OK titled
    /// [`BUY_LABEL`] or [`SELL_LABEL`] by `direction`, asking [`PROMPT`],
    /// the field at `max`. It is laid out by the template, or by
    /// [`QuantityDialog::fallback`] when there is none or it is unusable,
    /// its text measured by `metrics`. `by_key` says B or S opened it,
    /// whose character it then drops.
    pub fn open(
        &mut self,
        payload: T,
        direction: Direction,
        max: i64,
        by_key: bool,
        metrics: Rc<dyn TextMetrics>,
    ) {
        let title = match direction {
            Direction::Buy => BUY_LABEL,
            Direction::Sell => SELL_LABEL,
        };
        let built = self.template.as_ref().and_then(|template| {
            QuantityDialog::new(
                template,
                max,
                title,
                PROMPT,
                self.style,
                Rc::clone(&metrics),
            )
            .ok()
        });
        let mut dialog = built
            .unwrap_or_else(|| QuantityDialog::fallback(max, title, PROMPT, self.style, metrics));
        if by_key {
            dialog.flush_typed_key();
        }
        self.host.open(payload, dialog);
    }

    /// Whether the dialog is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.host.is_open()
    }

    /// The dialog, while it is open.
    #[must_use]
    pub fn dialog(&self) -> Option<&QuantityDialog> {
        self.host.dialog()
    }

    /// Only while the dialog is open (otherwise it gives `None` and
    /// changes nothing): follows Alt, then gives `input` to the dialog,
    /// appending its sounds to `sounds`. Gives `(payload, n)` for a count
    /// `n` above 0 confirmed there; a count of 0 (Cancel included) closes
    /// it and gives `None`.
    pub fn input(&mut self, input: &Input, sounds: &mut Vec<Sound>) -> Option<(T, u32)> {
        if !self.is_open() {
            return None;
        }
        self.follow_alt(input);
        self.host
            .input(input, sounds)
            .filter(|&(_, count)| count > 0)
    }

    /// Draws the open dialog: call it last, over the screen.
    pub fn draw(&self, list: &mut DrawList) {
        self.host.draw(list);
    }

    /// Cancels the open dialog's pointer. Returns whether one was open.
    pub fn cancel_pointer(&mut self) -> bool {
        self.host.cancel_pointer()
    }

    /// Lets go of Alt, and of the keys held in the open dialog.
    pub fn release_keys(&mut self) {
        self.alt = false;
        self.host.release_keys();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};
    use crate::ui::quantity::{CANCEL_ITEM, OK_ITEM};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DITL` 1003, fixed at (10, 20): OK (1), the prompt (2), the
    /// field (3) and Cancel (4).
    fn template() -> DialogTemplate {
        let item = |x, y, w, h, kind| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled: true,
            kind,
        };
        DialogTemplate {
            bounds: rect(10.0, 20.0, 172.0, 72.0),
            placement: Placement::Fixed,
            items: vec![
                item(92.0, 42.0, 70.0, 20.0, ItemSpec::Button("OK".into())),
                item(6.0, 8.0, 102.0, 16.0, ItemSpec::StaticText("^0".into())),
                item(112.0, 8.0, 51.0, 16.0, ItemSpec::EditText("12345".into())),
                item(10.0, 42.0, 70.0, 20.0, ItemSpec::Button("Cancel".into())),
            ],
        }
    }

    fn ask() -> QuantityAsk<&'static str> {
        QuantityAsk::new(Some(template()), ButtonStyle::STOCK)
    }

    /// `ask`, opened for "row" at 7 by a click.
    fn opened(direction: Direction) -> QuantityAsk<&'static str> {
        let mut ask = ask();
        ask.open("row", direction, 7, false, Rc::new(MonoMetrics));
        ask
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn texts(ask: &QuantityAsk<&str>) -> Vec<String> {
        let mut list = DrawList::new();
        ask.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn field(ask: &QuantityAsk<&str>) -> Option<String> {
        ask.dialog().map(|dialog| dialog.field().text().to_owned())
    }

    /// Clicks item `item` of the open dialog, down and up.
    fn click(ask: &mut QuantityAsk<&'static str>, item: usize) -> Option<(&'static str, u32)> {
        let at = ask
            .dialog()
            .expect("open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
            .center();
        let mut sounds = Vec::new();
        ask.input(
            &Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at,
            },
            &mut sounds,
        );
        ask.input(
            &Input::PointerButton {
                button: MouseButton::Left,
                pressed: false,
                at,
            },
            &mut sounds,
        )
    }

    #[test]
    fn it_opens_at_the_max_titled_buy_or_sell() {
        let ask = opened(Direction::Buy);
        assert!(ask.is_open());
        assert_eq!(texts(&ask), ["Buy", "Enter quantity:", "Cancel", "7"]);
        let dialog = ask.dialog().expect("open");
        assert_eq!(dialog.field().selection(), 1, "selected whole");
        assert_eq!(
            dialog.dialog().bounds().min,
            Point::new(10.0, 20.0),
            "laid out by its template"
        );
        let ask = opened(Direction::Sell);
        assert_eq!(texts(&ask), ["Sell", "Enter quantity:", "Cancel", "7"]);
    }

    #[test]
    fn closed_it_draws_nothing_and_has_no_dialog() {
        let ask = ask();
        assert!(!ask.is_open());
        assert!(ask.dialog().is_none());
        assert!(texts(&ask).is_empty());
    }

    #[test]
    fn without_a_usable_template_it_opens_the_fallback() {
        let metrics: Rc<dyn TextMetrics> = Rc::new(MonoMetrics);
        let fallback =
            QuantityDialog::fallback(7, "Buy", PROMPT, ButtonStyle::STOCK, Rc::clone(&metrics));
        let empty = DialogTemplate {
            items: Vec::new(),
            ..template()
        };
        for template in [None, Some(empty)] {
            let mut ask = QuantityAsk::new(template, ButtonStyle::STOCK);
            ask.open("row", Direction::Buy, 7, false, Rc::clone(&metrics));
            let bounds = ask.dialog().expect("open").dialog().bounds();
            assert_eq!(bounds, fallback.dialog().bounds());
            assert_eq!((bounds.width(), bounds.height()), (172.0, 72.0));
            assert_eq!(texts(&ask), ["Buy", "Enter quantity:", "Cancel", "7"]);
        }
    }

    #[test]
    fn a_count_above_0_gives_the_payload_and_the_count() {
        let mut ask = opened(Direction::Buy);
        assert_eq!(
            ask.input(&key(Key::Enter, true), &mut Vec::new()),
            Some(("row", 7))
        );
        assert!(!ask.is_open());
        let mut ask = opened(Direction::Sell);
        assert_eq!(ask.input(&Input::Text('2'), &mut Vec::new()), None);
        assert_eq!(click(&mut ask, OK_ITEM), Some(("row", 2)));
        assert!(!ask.is_open());
    }

    #[test]
    fn a_count_of_0_or_cancel_gives_nothing_and_closes_it() {
        let mut ask = opened(Direction::Buy);
        ask.input(&Input::Text('0'), &mut Vec::new());
        assert_eq!(ask.input(&key(Key::Enter, true), &mut Vec::new()), None);
        assert!(!ask.is_open());
        let mut ask = opened(Direction::Buy);
        assert_eq!(click(&mut ask, CANCEL_ITEM), None);
        assert!(!ask.is_open());
    }

    #[test]
    fn a_refused_count_beeps_and_stays_open() {
        let mut ask = opened(Direction::Buy);
        let mut sounds = vec![Sound::Ui(UiSound::ButtonUp)];
        ask.input(&Input::Text('9'), &mut sounds);
        assert_eq!(ask.input(&key(Key::Enter, true), &mut sounds), None);
        assert_eq!(
            sounds,
            [Sound::Ui(UiSound::ButtonUp), Sound::Ui(UiSound::Alert)]
        );
        assert!(ask.is_open());
        assert_eq!(field(&ask).as_deref(), Some("7"), "reset to the most");
    }

    #[test]
    fn alt_is_followed_while_open_and_while_closed() {
        let mut ask = ask();
        assert!(!ask.alt_held());
        ask.follow_alt(&key(MAX_LOT_KEY, true));
        assert!(ask.alt_held());
        ask.follow_alt(&key(Key::Char('b'), false));
        assert!(ask.alt_held(), "another key changes nothing");
        ask.open("row", Direction::Buy, 7, false, Rc::new(MonoMetrics));
        ask.input(&key(MAX_LOT_KEY, false), &mut Vec::new());
        assert!(!ask.alt_held());
        ask.input(
            &Input::Key {
                key: MAX_LOT_KEY,
                pressed: true,
                repeat: true,
            },
            &mut Vec::new(),
        );
        assert!(ask.alt_held(), "a repeat holds it");
        assert!(ask.is_open());
        ask.follow_alt(&key(MAX_LOT_KEY, false));
        assert!(!ask.alt_held());
    }

    #[test]
    fn releasing_the_keys_lets_go_of_alt_open_or_closed() {
        let mut ask = ask();
        ask.follow_alt(&key(MAX_LOT_KEY, true));
        ask.release_keys();
        assert!(!ask.alt_held());
        ask.open("row", Direction::Buy, 7, false, Rc::new(MonoMetrics));
        ask.input(&key(MAX_LOT_KEY, true), &mut Vec::new());
        ask.release_keys();
        assert!(!ask.alt_held());
        assert!(ask.is_open(), "it stays open");
    }

    #[test]
    fn opening_by_key_drops_the_typed_character_and_by_click_keeps_it() {
        let mut ask = ask();
        ask.open("row", Direction::Sell, 7, true, Rc::new(MonoMetrics));
        ask.input(&Input::Text('s'), &mut Vec::new());
        assert_eq!(field(&ask).as_deref(), Some("7"));
        let mut ask = opened(Direction::Sell);
        ask.input(&Input::Text('3'), &mut Vec::new());
        assert_eq!(field(&ask).as_deref(), Some("3"));
    }

    #[test]
    fn input_while_closed_changes_nothing() {
        let mut ask = ask();
        let mut sounds = vec![Sound::Ui(UiSound::ButtonUp)];
        assert_eq!(ask.input(&key(MAX_LOT_KEY, true), &mut sounds), None);
        assert!(!ask.alt_held(), "only follow_alt follows it when closed");
        assert_eq!(sounds, [Sound::Ui(UiSound::ButtonUp)]);
        assert!(!ask.is_open());
    }

    #[test]
    fn cancelling_the_pointer_reaches_the_open_dialog_only() {
        let mut ask = ask();
        assert!(!ask.cancel_pointer());
        let mut ask = opened(Direction::Buy);
        let at = ask
            .dialog()
            .expect("open")
            .dialog()
            .item_bounds(OK_ITEM)
            .expect("OK")
            .center();
        let down = Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        };
        let mut sounds = Vec::new();
        ask.input(&down, &mut sounds);
        assert!(ask.cancel_pointer());
        let up = Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        };
        assert_eq!(ask.input(&up, &mut sounds), None, "the click was abandoned");
        assert!(ask.is_open());
    }

    /// The original's `_DoQuantityDialog` (@0x56fa5) runs with the Toolbox's
    /// standard filter, which maps Return and Escape to items silently.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [Key::Enter, Key::Escape] {
            let mut ask = opened(Direction::Buy);
            let mut sounds = Vec::new();
            ask.input(&key(k, true), &mut sounds);
            ask.input(&key(k, false), &mut sounds);
            assert_eq!(sounds, [], "{k:?}");
        }
    }
}
