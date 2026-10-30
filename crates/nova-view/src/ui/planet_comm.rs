//! The planet comm dialog (`DLOG` 1009, over `PICT` 8512), which a
//! stellar hailed opens (`_DoPlanetCommDialog` @0x96949,
//! `_PlanetCommDialogUpdate` @0x90a98, `_PlanetCommFilter` @0x911a9,
//! `_DrawPlanetCommDialogButtons` @0x2893c and
//! `_TrackPlanetCommDialogButtons` @0x28b28 in the `EV Nova` executable).
//!
//! It shows a [`PlanetCommView`] in Geneva 9 ([`TEXT_SIZE`]):
//!
//! - **Item 4, the reply** (@0x90b7c-0x90c57): white text on a black box,
//!   the box the item inset 1 and the text set in the item inset (4, 2).
//! - **Item 5, the stellar's sprite** (@0x90cc6-0x90e26): the frame the
//!   flight shows of it, centred in the item.
//! - **Item 6, the text** (@0x90e8d-0x910f2): its name in white on the
//!   baseline 12 below the item's top, its type label in medium grey on
//!   26, and on 42 the status line: "Status:" (`STR#` 2002 #196) in medium
//!   grey, two spaces, then "Owned" or "Dominated" in bright green, or,
//!   bold, "Forbidden" in orange or "Hostile" in red ([`status_color`]).
//! - **Items 1 to 3** are its buttons: "Close Channel" (`STR#` 150 #21),
//!   then item 2 and item 3 as the view names them, item 3 hidden for a
//!   stellar always dominated.
//!
//! **Keys** (`_PlanetCommFilter`): a key press, never its repeat, either
//! case: Return, Enter, Escape and E close the channel; G presses
//! Greetings, O and B Offer Bribe, D and T Demand Tribute, and R Release,
//! each only while item 2 or 3 says so.
//!
//! **Clicks** (`_TrackPlanetCommDialogButtons`): a button pressed and let
//! go over it presses its item. The original tracks item 3's rect even
//! while it is hidden (@0x28b7e-0x28b9a), so a click where it stands
//! presses item 3, which beeps and does nothing; so does this.
//!
//! Without the interface file, [`PlanetCommDialog::fallback`] lays out the
//! same items itself, over a dark backdrop.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::planet_comm::{
    PlanetCommButton, PlanetCommItem, PlanetCommView, PlanetStatus, STATUS_LABEL,
};

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key, MouseButton};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::comm::{
    CLOSE_CHANNEL, ESCORT_COLOR, HOSTILE_COLOR, HOSTILE_GAP, LABEL_COLOR, TEXT_COLOR, TEXT_SIZE,
    backdrop,
};
use super::dialog::{
    Dialog, DialogEvent, DialogTemplate, FocusRing, ItemSpec, ItemTemplate, Placement, Role,
    STOCK_WORD, TextBacking,
};

/// The planet comm dialog's `DLOG` (and `DITL`) ID (@0x96b85).
pub const PLANET_COMM_DIALOG: i16 = 1009;
/// The picture the planet comm dialog is drawn over (@0x96e19).
pub const PLANET_COMM_PICTURE: ImageKey = ImageKey::picture(8512);
/// "Close Channel".
pub const CLOSE_ITEM: usize = 1;
/// Greetings or Offer Bribe.
pub const SECOND_ITEM: usize = 2;
/// Demand Tribute or Release.
pub const THIRD_ITEM: usize = 3;
/// The reply.
pub const REPLY_ITEM: usize = 4;
/// The stellar's sprite.
pub const SPRITE_ITEM: usize = 5;
/// The name, the type label and the status line.
pub const TEXT_ITEM: usize = 6;
/// The text's baselines below item 6's top: the name, the type label and
/// the status line (@0x90eb1, @0x90efa, @0x90f41).
pub const TEXT_BASELINES: [f32; 3] = [12.0, 26.0, 42.0];
/// "Forbidden"'s orange (0xffff, 0x6666, 0, @0x90aa9-0x90ab5).
pub const FORBIDDEN_COLOR: Color = Color::rgba(0xFF, 0x66, 0x00, 255);

/// The reply's box (@0x90bd2-0x90c57): item 4 inset 1 painted white, the
/// text set black in the item inset (4, 2), then the inset-1 box
/// inverted.
const REPLY_BACKING: TextBacking = TextBacking {
    fill: Color::BLACK,
    fill_inset: 1.0,
    text_inset: (4.0, 2.0),
};

/// The stellar's frame the flight shows, keyed, with its width and
/// height: what item 5 draws.
pub type StellarFrame = (ImageKey, (f32, f32));

/// `status`'s colour, and whether it is bold (@0x90fb6-0x910e1).
#[must_use]
pub fn status_color(status: PlanetStatus) -> (Color, bool) {
    match status {
        PlanetStatus::Owned | PlanetStatus::Dominated => (ESCORT_COLOR, false),
        PlanetStatus::Forbidden => (FORBIDDEN_COLOR, true),
        PlanetStatus::Hostile => (HOSTILE_COLOR, true),
    }
}

/// The press a key is, for a dialog showing `view` (see the module docs).
#[must_use]
pub fn key_press(key: Key, view: &PlanetCommView) -> Option<PlanetCommItem> {
    let second = |button| (view.second == button).then_some(PlanetCommItem::Second);
    let third = |button| (view.third == Some(button)).then_some(PlanetCommItem::Third);
    match key {
        Key::Enter | Key::Escape => Some(PlanetCommItem::Close),
        Key::Char(c) => match c.to_ascii_lowercase() {
            'e' => Some(PlanetCommItem::Close),
            'g' => second(PlanetCommButton::Greetings),
            'o' | 'b' => second(PlanetCommButton::OfferBribe),
            'd' | 't' => third(PlanetCommButton::DemandTribute),
            'r' => third(PlanetCommButton::Release),
            _ => None,
        },
        _ => None,
    }
}

/// The planet comm dialog (see the module docs).
#[derive(Clone)]
pub struct PlanetCommDialog {
    template: DialogTemplate,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    picture: Option<ImageKey>,
    view: PlanetCommView,
    sprite: Option<StellarFrame>,
    dialog: Dialog,
    /// Whether its buttons take the keyboard focus, kept across each
    /// [`PlanetCommDialog::set_view`] that lays it out again.
    focus_ring: FocusRing,
    press: Option<PlanetCommItem>,
    /// Whether the pointer went down on item 3 while it is hidden.
    hidden_pressed: bool,
}

impl std::fmt::Debug for PlanetCommDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlanetCommDialog")
            .field("view", &self.view)
            .field("sprite", &self.sprite)
            .field("dialog", &self.dialog)
            .field("press", &self.press)
            .finish_non_exhaustive()
    }
}

impl PlanetCommDialog {
    /// The dialog `template` (stock `DLOG` 1009) showing `view`, the
    /// stellar drawn as `sprite`, its buttons labelled in `style` and its
    /// text measured by `metrics`, over [`PLANET_COMM_PICTURE`].
    ///
    /// # Errors
    ///
    /// When the template has fewer than its six items.
    pub fn new(
        template: &DialogTemplate,
        view: &PlanetCommView,
        sprite: Option<StellarFrame>,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        Self::built(
            template,
            view,
            sprite,
            style,
            metrics,
            Some(PLANET_COMM_PICTURE),
        )
    }

    /// The dialog laid out without the interface file, as `DLOG` 1009 is,
    /// over a dark backdrop.
    #[must_use]
    pub fn fallback(
        view: &PlanetCommView,
        sprite: Option<StellarFrame>,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self::built(&planet_comm_template(), view, sprite, style, metrics, None)
            .expect("the fallback has every item")
    }

    fn built(
        template: &DialogTemplate,
        view: &PlanetCommView,
        sprite: Option<StellarFrame>,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
        picture: Option<ImageKey>,
    ) -> Result<Self, String> {
        if template.items.len() < TEXT_ITEM {
            return Err(format!(
                "DITL {PLANET_COMM_DIALOG} has {} items, not the planet comm dialog's {TEXT_ITEM}",
                template.items.len()
            ));
        }
        let dialog = layout(template, view, style, &metrics);
        Ok(Self {
            template: template.clone(),
            style,
            metrics,
            picture,
            view: view.clone(),
            sprite,
            dialog,
            focus_ring: FocusRing::default(),
            press: None,
            hidden_pressed: false,
        })
    }

    /// The planet comm dialog with its buttons taking the keyboard focus
    /// as `ring` says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        self.focus_ring = ring;
        self.dialog.set_focus_ring(ring);
        self
    }

    /// Shows `view`: its reply, its buttons and its status line.
    pub fn set_view(&mut self, view: &PlanetCommView) {
        self.dialog = layout(&self.template, view, self.style, &self.metrics);
        self.dialog.set_focus_ring(self.focus_ring);
        self.view = view.clone();
    }

    /// The press the player made, once.
    pub fn take_press(&mut self) -> Option<PlanetCommItem> {
        self.press.take()
    }

    /// The dialog itself, for its layout.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The view shown.
    #[must_use]
    pub fn view(&self) -> &PlanetCommView {
        &self.view
    }

    /// A click where item 3 stands while it is hidden, as the original's
    /// track takes one (see the module docs): whether `input` was one of
    /// its halves.
    fn hidden_click(&mut self, input: &Input) -> bool {
        let Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        } = *input
        else {
            return false;
        };
        let over = self.view.third.is_none()
            && self
                .dialog
                .item_bounds(THIRD_ITEM)
                .is_some_and(|bounds| bounds.contains(at));
        if pressed {
            self.hidden_pressed = over;
            return over;
        }
        let pressed_here = std::mem::take(&mut self.hidden_pressed);
        if pressed_here && over {
            self.press = Some(PlanetCommItem::Third);
        }
        pressed_here
    }

    /// Draws item 6's lines (see the module docs).
    fn draw_text(&self, list: &mut DrawList) {
        let Some(at) = self.dialog.item_bounds(TEXT_ITEM) else {
            return;
        };
        let top = |baseline: f32| at.min.y + baseline - TEXT_SIZE;
        let [name, label, status] = TEXT_BASELINES;
        list.text(
            self.view.name.clone(),
            Point::new(at.min.x, top(name)),
            TEXT_SIZE,
            None,
            TEXT_COLOR,
        );
        list.text(
            self.view.type_label.clone(),
            Point::new(at.min.x, top(label)),
            TEXT_SIZE,
            None,
            LABEL_COLOR,
        );
        let Some(shown) = self.view.status else {
            return;
        };
        list.text(
            STATUS_LABEL,
            Point::new(at.min.x, top(status)),
            TEXT_SIZE,
            None,
            LABEL_COLOR,
        );
        let width = |text| self.metrics.width(Font::Geneva, TEXT_SIZE, text);
        let x = at.min.x + width(STATUS_LABEL) + width(HOSTILE_GAP);
        let (color, bold) = status_color(shown);
        let passes: &[f32] = if bold { &[0.0, 1.0] } else { &[0.0] };
        for dx in passes {
            list.text(
                shown.text(),
                Point::new(x + dx, top(status)),
                TEXT_SIZE,
                None,
                color,
            );
        }
    }

    /// Draws the stellar's frame centred in item 5, its top left where the
    /// original's rect puts it (the item's centre less half the frame,
    /// each halving truncated, @0x90d04-0x90d66).
    fn draw_sprite(&self, list: &mut DrawList) {
        let (Some((image, (width, height))), Some(at)) =
            (self.sprite, self.dialog.item_bounds(SPRITE_ITEM))
        else {
            return;
        };
        let centre = |min: f32, max: f32| f32::midpoint(min, max).trunc();
        let left = centre(at.min.x, at.max.x) - (width / 2.0).trunc();
        let top = centre(at.min.y, at.max.y) - (height / 2.0).trunc();
        list.sprite(
            image,
            Point::new(left + width / 2.0, top + height / 2.0),
            Color::WHITE,
        );
    }
}

/// The planet comm dialog for `view` on `template`: the three buttons
/// labelled, item 3 hidden when the view has none, and the reply set.
fn layout(
    template: &DialogTemplate,
    view: &PlanetCommView,
    style: ButtonStyle,
    metrics: &Rc<dyn TextMetrics>,
) -> Dialog {
    let third = view.third.map_or(Role::Hidden, |button| {
        Role::Button(button.label().to_owned())
    });
    let roles = [
        (CLOSE_ITEM, Role::Button(CLOSE_CHANNEL.to_owned())),
        (SECOND_ITEM, Role::Button(view.second.label().to_owned())),
        (THIRD_ITEM, third),
        (
            REPLY_ITEM,
            Role::ScrollText {
                text: view.reply.clone(),
                font: Font::Geneva,
                size: TEXT_SIZE,
                color: TEXT_COLOR,
                backing: Some(REPLY_BACKING),
            },
        ),
    ];
    let mut enabled = template.clone();
    for item in [CLOSE_ITEM, SECOND_ITEM, THIRD_ITEM] {
        enabled.items[item - 1].enabled = true;
    }
    Dialog::new(&enabled, &roles, Rc::clone(metrics))
        .with_buttons(ButtonSkin::NOVA, style)
        .with_default(None)
        .with_cancel(None)
}

/// The built-in planet comm dialog: `DLOG` 1009's items, 540 x 295,
/// placed as the loaded `DLOG` is ([`Placement::for_dlog`]).
fn planet_comm_template() -> DialogTemplate {
    let at = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let user = |bounds, enabled| ItemTemplate {
        bounds,
        enabled,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: at(0.0, 0.0, 540.0, 295.0),
        placement: Placement::for_dlog(PLANET_COMM_DIALOG, Some(STOCK_WORD)),
        items: vec![
            user(at(27.0, 244.0, 146.0, 26.0), true),
            user(at(27.0, 184.0, 146.0, 26.0), true),
            user(at(27.0, 214.0, 146.0, 26.0), true),
            user(at(5.0, 5.0, 200.0, 60.0), false),
            user(at(222.0, 5.0, 310.0, 283.0), false),
            user(at(16.0, 82.0, 120.0, 50.0), false),
        ],
    }
}

impl Screen for PlanetCommDialog {
    /// A key press (not a repeat) that is a press is one; a click where a
    /// hidden item 3 stands presses it; every other event goes to the
    /// dialog, and a button activated is its press. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key,
            pressed: true,
            repeat,
        } = *input
            && let Some(press) = key_press(key, &self.view)
        {
            if !repeat {
                self.press = Some(press);
            }
            return ScreenAction::None;
        }
        if self.hidden_click(input) {
            return ScreenAction::None;
        }
        self.press = match self.dialog.input(input) {
            Some(DialogEvent::Item(CLOSE_ITEM)) => Some(PlanetCommItem::Close),
            Some(DialogEvent::Item(SECOND_ITEM)) => Some(PlanetCommItem::Second),
            Some(DialogEvent::Item(THIRD_ITEM)) => Some(PlanetCommItem::Third),
            _ => self.press,
        };
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The picture (or the backdrop), the stellar's sprite, the buttons and
    /// the reply, then the text.
    fn draw(&self, list: &mut DrawList) {
        backdrop(list, self.dialog.bounds(), self.picture);
        self.draw_sprite(list);
        self.dialog.draw(list);
        self.draw_text(list);
    }

    fn cancel_pointer(&mut self) {
        self.hidden_pressed = false;
        self.dialog.cancel_pointer();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::StellarId;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;

    /// Earth greeting, its buttons Greetings and Demand Tribute.
    fn greeting() -> PlanetCommView {
        PlanetCommView {
            stellar: StellarId(128),
            name: "Earth".to_owned(),
            type_label: "[Earth/Luna System]".to_owned(),
            reply: "Communications channel open to Earth.".to_owned(),
            second: PlanetCommButton::Greetings,
            third: Some(PlanetCommButton::DemandTribute),
            status: None,
            asking: None,
        }
    }

    /// Earth refusing landing, its buttons Offer Bribe and Demand Tribute.
    fn refusing(status: PlanetStatus) -> PlanetCommView {
        PlanetCommView {
            reply: "What is it you want?".to_owned(),
            second: PlanetCommButton::OfferBribe,
            status: Some(status),
            ..greeting()
        }
    }

    /// An always-dominated stellar: no item 3.
    fn owned() -> PlanetCommView {
        PlanetCommView {
            third: None,
            status: Some(PlanetStatus::Owned),
            ..greeting()
        }
    }

    /// Stock `DLOG` 1009's items, fixed at (0, 0).
    fn stock() -> DialogTemplate {
        DialogTemplate {
            placement: Placement::Fixed,
            ..planet_comm_template()
        }
    }

    const SPRITE: StellarFrame = (ImageKey::sprite(1000, 3), (101.0, 64.0));

    fn dialog(view: &PlanetCommView) -> PlanetCommDialog {
        PlanetCommDialog::new(
            &stock(),
            view,
            Some(SPRITE),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    fn drawn(screen: &impl Screen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// Each text drawn in item 6, with its origin and colour.
    fn item_six(screen: &impl Screen) -> Vec<(String, Point, Color)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    size,
                    ..
                } if size == TEXT_SIZE && origin.y >= 82.0 && origin.y < 132.0 => {
                    Some((text, origin, color))
                }
                _ => None,
            })
            .collect()
    }

    /// Every text drawn, the buttons' labels included.
    fn labels(screen: &impl Screen) -> Vec<String> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn text(text: &str, x: f32, y: f32, color: Color) -> (String, Point, Color) {
        (text.to_owned(), Point::new(x, y), color)
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn click_at(screen: &mut PlanetCommDialog, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click(screen: &mut PlanetCommDialog, item: usize) {
        let at = screen.dialog().item_bounds(item).expect("an item").center();
        click_at(screen, at);
    }

    #[test]
    fn its_debug_names_it_and_its_view() {
        let shown = format!("{:?}", dialog(&greeting()));
        assert!(shown.starts_with("PlanetCommDialog"), "{shown}");
        assert!(shown.contains("Earth"), "the view: {shown}");
    }

    #[test]
    fn its_items_stand_where_ditl_1009_puts_them() {
        let dialog = dialog(&greeting());
        let laid = dialog.dialog();
        let button = |y| Bounds::at(Point::new(27.0, y), 146.0, 26.0);
        assert_eq!(laid.item_bounds(CLOSE_ITEM), Some(button(244.0)));
        assert_eq!(laid.item_bounds(SECOND_ITEM), Some(button(184.0)));
        assert_eq!(laid.item_bounds(THIRD_ITEM), Some(button(214.0)));
        assert_eq!(
            (laid.bounds().width(), laid.bounds().height()),
            (540.0, 295.0)
        );
        assert_eq!(
            (PLANET_COMM_DIALOG, PLANET_COMM_PICTURE),
            (1009, ImageKey::picture(8512))
        );
    }

    #[test]
    fn its_buttons_say_what_the_view_names() {
        let shown = labels(&dialog(&greeting()));
        for label in ["Close Channel", "Greetings", "Demand Tribute"] {
            assert!(shown.iter().any(|text| text == label), "{label}: {shown:?}");
        }
        let shown = labels(&dialog(&refusing(PlanetStatus::Forbidden)));
        assert!(shown.iter().any(|text| text == "Offer Bribe"), "{shown:?}");
        let dominated = PlanetCommView {
            third: Some(PlanetCommButton::Release),
            ..greeting()
        };
        assert!(
            labels(&dialog(&dominated))
                .iter()
                .any(|text| text == "Release")
        );
        let shown = labels(&dialog(&owned()));
        assert!(
            !shown
                .iter()
                .any(|text| text == "Release" || text == "Demand Tribute"),
            "item 3 hidden: {shown:?}"
        );
        assert!(!dialog(&owned()).dialog().item_shown(THIRD_ITEM));
    }

    #[test]
    fn item_six_shows_the_name_the_type_label_and_no_status_while_open() {
        // Item 6's top is 82; a baseline less 9 is a top.
        assert_eq!(
            item_six(&dialog(&greeting())),
            [
                text("Earth", 16.0, 85.0, TEXT_COLOR),
                text("[Earth/Luna System]", 16.0, 99.0, LABEL_COLOR),
            ]
        );
    }

    #[test]
    fn a_refusing_stellars_status_is_bold_forbidden_or_hostile() {
        // MonoMetrics: "Status:" and two spaces are 9 characters wide.
        let gap = MonoMetrics.width(Font::Geneva, TEXT_SIZE, "Status:  ");
        let x = 16.0 + gap;
        assert_eq!(
            item_six(&dialog(&refusing(PlanetStatus::Forbidden)))[2..],
            [
                text("Status:", 16.0, 115.0, LABEL_COLOR),
                text("Forbidden", x, 115.0, FORBIDDEN_COLOR),
                text("Forbidden", x + 1.0, 115.0, FORBIDDEN_COLOR),
            ]
        );
        assert_eq!(
            item_six(&dialog(&refusing(PlanetStatus::Hostile)))[3..],
            [
                text("Hostile", x, 115.0, HOSTILE_COLOR),
                text("Hostile", x + 1.0, 115.0, HOSTILE_COLOR),
            ]
        );
        assert_eq!(FORBIDDEN_COLOR, Color::rgba(0xFF, 0x66, 0x00, 255));
    }

    #[test]
    fn a_dominated_stellars_status_is_green_and_plain() {
        let shown = item_six(&dialog(&owned()));
        let gap = MonoMetrics.width(Font::Geneva, TEXT_SIZE, "Status:  ");
        assert_eq!(shown[3..], [text("Owned", 16.0 + gap, 115.0, ESCORT_COLOR)]);
        let dominated = PlanetCommView {
            status: Some(PlanetStatus::Dominated),
            ..greeting()
        };
        assert_eq!(
            item_six(&dialog(&dominated))[3..],
            [text("Dominated", 16.0 + gap, 115.0, ESCORT_COLOR)]
        );
    }

    #[test]
    fn the_reply_is_set_in_item_four() {
        let mut shown = dialog(&greeting());
        assert_eq!(
            shown
                .dialog()
                .text(REPLY_ITEM)
                .map(|text| text.lines().to_vec()),
            Some(vec!["Communications channel open to Earth.".to_owned()])
        );
        shown.set_view(&refusing(PlanetStatus::Hostile));
        assert_eq!(
            shown
                .dialog()
                .text(REPLY_ITEM)
                .map(|text| text.lines().to_vec()),
            Some(vec!["What is it you want?".to_owned()])
        );
        assert_eq!(shown.view(), &refusing(PlanetStatus::Hostile));
    }

    #[test]
    fn the_stellars_frame_is_centred_in_item_five() {
        let sprites: Vec<_> = drawn(&dialog(&greeting()))
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, center, .. } => Some((image, center)),
                _ => None,
            })
            .collect();
        // Item 5's centre is (377, 146); 101 x 64 truncates to a top left of
        // (327, 114).
        assert_eq!(
            sprites,
            [(SPRITE.0, Point::new(327.0 + 50.5, 114.0 + 32.0))]
        );
        let none = PlanetCommDialog::new(
            &stock(),
            &greeting(),
            None,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert!(
            !drawn(&none)
                .iter()
                .any(|command| matches!(command, DrawCommand::Sprite { .. }))
        );
    }

    #[test]
    fn each_key_presses_its_item_only_while_its_button_says_so() {
        let open = greeting();
        let hated = refusing(PlanetStatus::Forbidden);
        let dominated = PlanetCommView {
            third: Some(PlanetCommButton::Release),
            ..greeting()
        };
        let cases = [
            (Key::Enter, &open, Some(PlanetCommItem::Close)),
            (Key::Escape, &open, Some(PlanetCommItem::Close)),
            (Key::Char('E'), &open, Some(PlanetCommItem::Close)),
            (Key::Char('g'), &open, Some(PlanetCommItem::Second)),
            (Key::Char('g'), &hated, None),
            (Key::Char('o'), &hated, Some(PlanetCommItem::Second)),
            (Key::Char('B'), &hated, Some(PlanetCommItem::Second)),
            (Key::Char('o'), &open, None),
            (Key::Char('d'), &open, Some(PlanetCommItem::Third)),
            (Key::Char('T'), &open, Some(PlanetCommItem::Third)),
            (Key::Char('d'), &dominated, None),
            (Key::Char('r'), &dominated, Some(PlanetCommItem::Third)),
            (Key::Char('r'), &open, None),
            (Key::Char('r'), &owned(), None),
            (Key::Char('d'), &owned(), None),
            (Key::Char('x'), &open, None),
            (Key::Tab, &open, None),
        ];
        for (pressed, view, press) in cases {
            assert_eq!(key_press(pressed, view), press, "{pressed:?} on {view:?}");
        }
    }

    #[test]
    fn a_key_press_presses_once_and_its_repeat_does_not() {
        let mut shown = dialog(&greeting());
        shown.input(&key(Key::Char('g'), true));
        assert_eq!(shown.take_press(), None, "a repeat");
        shown.input(&key(Key::Char('g'), false));
        assert_eq!(shown.take_press(), Some(PlanetCommItem::Second));
        assert_eq!(shown.take_press(), None, "once");
    }

    #[test]
    fn a_click_presses_its_button() {
        let mut shown = dialog(&greeting());
        for (item, press) in [
            (CLOSE_ITEM, PlanetCommItem::Close),
            (SECOND_ITEM, PlanetCommItem::Second),
            (THIRD_ITEM, PlanetCommItem::Third),
        ] {
            click(&mut shown, item);
            assert_eq!(shown.take_press(), Some(press), "item {item}");
        }
    }

    #[test]
    fn its_buttons_take_clicks_though_the_template_disables_them() {
        let mut disabled = stock();
        for item in &mut disabled.items {
            item.enabled = false;
        }
        let mut shown = PlanetCommDialog::new(
            &disabled,
            &greeting(),
            None,
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        for (item, press) in [
            (CLOSE_ITEM, PlanetCommItem::Close),
            (SECOND_ITEM, PlanetCommItem::Second),
            (THIRD_ITEM, PlanetCommItem::Third),
        ] {
            click(&mut shown, item);
            assert_eq!(shown.take_press(), Some(press), "item {item}");
        }
    }

    #[test]
    fn a_click_where_a_hidden_item_three_stands_presses_it() {
        let mut shown = dialog(&owned());
        let at = shown
            .dialog()
            .item_bounds(THIRD_ITEM)
            .expect("an item")
            .center();
        click_at(&mut shown, at);
        assert_eq!(shown.take_press(), Some(PlanetCommItem::Third));
        let away = Point::new(at.x, at.y + 200.0);
        shown.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        shown.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: away,
        });
        assert_eq!(shown.take_press(), None, "let go elsewhere");
        shown.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        shown.cancel_pointer();
        shown.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        assert_eq!(shown.take_press(), None, "cancelled");
        let mut open = dialog(&greeting());
        click_at(&mut open, away);
        assert_eq!(open.take_press(), None, "nothing there");
    }

    #[test]
    fn the_fallback_lays_out_the_same_items_over_a_backdrop() {
        let fallback = PlanetCommDialog::fallback(
            &greeting(),
            Some(SPRITE),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        );
        let stocked = dialog(&greeting());
        for item in 1..=TEXT_ITEM {
            let size = |dialog: &PlanetCommDialog| {
                dialog
                    .dialog()
                    .item_bounds(item)
                    .map(|bounds| (bounds.width(), bounds.height()))
            };
            assert_eq!(size(&fallback), size(&stocked), "item {item}");
        }
        let pictured = |screen: &PlanetCommDialog| {
            drawn(screen).iter().any(|command| {
                matches!(
                    command,
                    DrawCommand::StretchedPicture { image, .. } if *image == PLANET_COMM_PICTURE
                )
            })
        };
        assert!(pictured(&stocked), "PICT 8512 under the stock dialog");
        assert!(
            !pictured(&fallback),
            "no picture without the interface file"
        );
        let short = DialogTemplate {
            items: stock().items[..5].to_vec(),
            ..stock()
        };
        assert!(
            PlanetCommDialog::new(
                &short,
                &greeting(),
                None,
                ButtonStyle::STOCK,
                Rc::new(MonoMetrics)
            )
            .is_err()
        );
    }

    #[test]
    fn the_built_in_planet_comm_dialog_is_placed_as_center_window_places_dlog_1009() {
        // `_DoPlanetCommDialog` moves it with `_CenterWindow` (@0x96eaf).
        let template = planet_comm_template();
        assert_eq!(template.placement, Placement::CenterWindow);
        assert_eq!(
            template.placement,
            Placement::for_dlog(PLANET_COMM_DIALOG, Some(STOCK_WORD)),
            "as the loaded DLOG is placed"
        );
        assert_eq!(
            template.placement.origin(template.bounds),
            Point::new(242.0, 237.0),
            "384 less 295 / 2 truncated: a pixel below the one-step floor"
        );
    }

    #[test]
    fn its_buttons_take_the_focus_ring_it_is_given_through_a_new_view() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let mut shown = dialog(&greeting()).with_focus_ring(ring);
            assert_eq!(shown.dialog().focus_ring(), ring);
            shown.set_view(&greeting());
            assert_eq!(shown.dialog().focus_ring(), ring, "laid out again");
        }
    }
}
