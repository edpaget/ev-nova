//! The Trade Center: the stellar's commodity exchange, laid out by the
//! interface file's "Trade" dialog (`DLOG`/`DITL` 1001) over its
//! background picture.
//!
//! The stock dialog is 426 x 252, centred, and drawn over `PICT` 8510,
//! whose black list area and status bar match its items: the header row
//! (item 3), eight rows of goods under it (items 4 to 11), the area below
//! them (item 12), where the events under way here are named, the status
//! bar (item 15), showing the free cargo space and the credits, and Buy
//! (13), Sell (14) and Done (1) along the bottom.
//!
//! Each row shows a good the stellar trades ([`Market`]): its name, its
//! price and the tons held. One row is selected, by a click on it or with
//! Up and Down; the list scrolls with the selection when there are more
//! than eight. Buy and Sell, clicked or with B and S (key repeats too, so
//! holding a key keeps trading), ask for a ton of the selected good; with
//! Alt held, the most possible, as the original's Option-click and
//! Option-B or -S do. Each is greyed when a ton could not be traded that
//! way. The screen only records the order ([`TradeScreen::take_order`]):
//! whoever holds the session makes the trade and gives back the exchange
//! as it now is ([`TradeScreen::set_market`]).
//!
//! Done, Return and Escape close it. Without the dialog, the screen says
//! why, and Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{Direction, Lot, Market, Order};

use super::layout::DONE_LABEL;
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};

/// The "Trade" dialog's `DLOG` (and `DITL`) ID.
pub const TRADE_DIALOG: i16 = 1001;

/// The picture the dialog is drawn over: "Trade", in Nova Graphics.
pub const BACKGROUND: ImageKey = ImageKey::picture(8510);

/// Done's item.
pub const DONE_ITEM: usize = 1;
/// The header row's item.
pub const HEADER_ITEM: usize = 3;
/// The first row's item; the rows are this and the seven after it.
pub const FIRST_ROW_ITEM: usize = 4;
/// How many rows show at once.
pub const ROWS: usize = 8;
/// The item the events under way are named in.
pub const EVENTS_ITEM: usize = 12;
/// Buy's item.
pub const BUY_ITEM: usize = 13;
/// Sell's item.
pub const SELL_ITEM: usize = 14;
/// The status bar's item.
pub const STATUS_ITEM: usize = 15;

/// `STR#` 150 #2.
pub const BUY_LABEL: &str = "Buy";
/// `STR#` 150 #3.
pub const SELL_LABEL: &str = "Sell";

/// Buys, as Buy does: the original's default.
pub const BUY_KEY: Key = Key::Char('b');
/// Sells, as Sell does: the original's default.
pub const SELL_KEY: Key = Key::Char('s');
/// Held, makes a Buy or Sell the most possible, as the original's
/// Option does.
pub const MAX_LOT_KEY: Key = Key::Alt;

/// The list's font, size and colour.
pub const TEXT_FONT: Font = Font::Geneva;
/// The list's size.
pub const TEXT_SIZE: f32 = 10.0;
/// The list's colour.
pub const TEXT_COLOR: Color = Color::WHITE;
/// The header's colour.
pub const HEADER_COLOR: Color = Color::DIM;
/// The selected row's background.
pub const SELECTED_COLOR: Color = Color::rgba(40, 60, 140, 255);
/// How far into its item each line of text starts.
pub const INSET: f32 = 4.0;
/// Where the price column starts, from a row's left.
pub const PRICE_X: f32 = 220.0;
/// Where the held column starts, from a row's left.
pub const HELD_X: f32 = 290.0;
/// The header's columns.
pub const HEADER: [&str; 3] = ["Commodity", "Price", "Held"];

/// The status bar's text for `market`: the free cargo space and the
/// credits.
#[must_use]
pub fn status_line(market: &Market) -> String {
    format!(
        "Free cargo space: {} tons    Credits: {}",
        market.free, market.cash
    )
}

/// The exchange, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    metrics: MetricsHandle,
}

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// The Trade Center of the stellar landed on.
#[derive(Clone, Debug)]
pub struct TradeScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    market: Market,
    /// The selected row, by index into the market's rows.
    selected: usize,
    /// The row shown first.
    top: usize,
    /// Whether Alt is held.
    max_lot: bool,
    /// The order asked for, until it is taken.
    order: Option<Order>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl TradeScreen {
    /// The exchange `market`, laid out by `layout`: the "Trade" dialog's
    /// template and the metrics its text is measured by, or why there are
    /// none. Its buttons are labelled in `style`. The first row is
    /// selected.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        market: Market,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let roles = [
                (DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (BUY_ITEM, Role::Button(BUY_LABEL.to_owned())),
                (SELL_ITEM, Role::Button(SELL_LABEL.to_owned())),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(DONE_ITEM))
                .with_cancel(Some(DONE_ITEM));
            Laid {
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        let mut screen = Self {
            laid,
            market,
            selected: 0,
            top: 0,
            max_lot: false,
            order: None,
            closed: false,
            sounds: Vec::new(),
        };
        screen.regrey();
        screen
    }

    /// The exchange shown.
    #[must_use]
    pub fn market(&self) -> &Market {
        &self.market
    }

    /// The selected row's index, if there are any rows.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        (self.selected < self.market.rows.len()).then_some(self.selected)
    }

    /// The index of the first row shown.
    #[must_use]
    pub fn top(&self) -> usize {
        self.top
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the exchange cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The order asked for since it was last taken, once.
    pub fn take_order(&mut self) -> Option<Order> {
        self.order.take()
    }

    /// Shows `market`, the exchange after a trade, keeping the selection
    /// where it can.
    pub fn set_market(&mut self, market: Market) {
        self.market = market;
        self.select(self.selected);
    }

    /// Selects row `index`, or the last row when there is no such row, and
    /// scrolls the list to show it, never past its end.
    fn select(&mut self, index: usize) {
        let count = self.market.rows.len();
        self.selected = index.min(count.saturating_sub(1));
        self.top = self.top.min(count.saturating_sub(ROWS));
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + ROWS {
            self.top = self.selected + 1 - ROWS;
        }
        self.regrey();
    }

    /// Greys Buy and Sell when a ton of the selected good could not be
    /// traded that way.
    fn regrey(&mut self) {
        let buys = self.allows(Direction::Buy);
        let sells = self.allows(Direction::Sell);
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_greyed(BUY_ITEM, !buys);
            laid.dialog.set_greyed(SELL_ITEM, !sells);
        }
    }

    /// Whether a ton of the selected good can be traded `direction`.
    fn allows(&self, direction: Direction) -> bool {
        self.selected()
            .is_some_and(|index| self.market.allows(self.market.rows[index].good, direction))
    }

    /// Asks to trade the selected good `direction`, a ton or with Alt held
    /// the most, if a ton could be traded that way.
    fn ask(&mut self, direction: Direction) {
        let Some(index) = self.selected().filter(|_| self.allows(direction)) else {
            return;
        };
        self.order = Some(Order {
            good: self.market.rows[index].good,
            direction,
            lot: if self.max_lot { Lot::Max } else { Lot::One },
        });
    }

    /// Activates dialog item `item`: Done closes, Buy and Sell ask, and a
    /// row is selected.
    fn activate(&mut self, item: usize) {
        match item {
            DONE_ITEM => self.closed = true,
            BUY_ITEM => self.ask(Direction::Buy),
            SELL_ITEM => self.ask(Direction::Sell),
            row => {
                let index = self.top + (row - FIRST_ROW_ITEM);
                if index < self.market.rows.len() {
                    self.select(index);
                }
            }
        }
    }

    /// `text` at the left of item `item`'s line `line` (from 0), plus
    /// `dx`, in the list's font.
    fn line(laid: &Laid, list: &mut DrawList, item: usize, line: usize, dx: f32, text: &str) {
        Self::text(laid, list, item, line, dx, text, TEXT_COLOR);
    }

    fn text(
        laid: &Laid,
        list: &mut DrawList,
        item: usize,
        line: usize,
        dx: f32,
        text: &str,
        color: Color,
    ) {
        let Some(bounds) = laid.dialog.item_bounds(item) else {
            return;
        };
        let height = laid.metrics.0.line_height(TEXT_FONT, TEXT_SIZE);
        #[allow(clippy::cast_precision_loss)] // A handful of lines.
        let y = bounds.min.y + height * line as f32;
        let origin = Point::new(bounds.min.x + dx, y);
        list.text_in(TEXT_FONT, text, origin, TEXT_SIZE, None, color);
    }
}

impl Screen for TradeScreen {
    /// Every input goes to the exchange; it never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        let Ok(laid) = &mut self.laid else {
            if let Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } = *input
            {
                self.closed = true;
            }
            return ScreenAction::None;
        };
        match *input {
            Input::Key {
                key: MAX_LOT_KEY,
                pressed,
                ..
            } => self.max_lot = pressed,
            Input::Key {
                key: Key::Up,
                pressed: true,
                ..
            } => self.select(self.selected.saturating_sub(1)),
            Input::Key {
                key: Key::Down,
                pressed: true,
                ..
            } => self.select(self.selected + 1),
            Input::Key {
                key: BUY_KEY,
                pressed: true,
                ..
            } => self.ask(Direction::Buy),
            Input::Key {
                key: SELL_KEY,
                pressed: true,
                ..
            } => self.ask(Direction::Sell),
            _ => {
                let event = laid.dialog.input(input);
                self.sounds.extend(laid.dialog.take_sound().map(Sound::Ui));
                if let Some(DialogEvent::Item(item)) = event {
                    self.activate(item);
                }
            }
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        let laid = match &self.laid {
            Ok(laid) => laid,
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the trade center: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
                return;
            }
        };
        let dialog = &laid.dialog;
        list.picture(BACKGROUND, dialog.bounds().min);
        for (column, dx) in HEADER.iter().zip([INSET, PRICE_X, HELD_X]) {
            Self::text(laid, list, HEADER_ITEM, 0, dx, column, HEADER_COLOR);
        }
        let shown = self
            .market
            .rows
            .iter()
            .enumerate()
            .skip(self.top)
            .take(ROWS);
        for (item, (index, row)) in (FIRST_ROW_ITEM..).zip(shown) {
            if Some(index) == self.selected()
                && let Some(bounds) = dialog.item_bounds(item)
            {
                fill_rect(list, bounds, SELECTED_COLOR);
            }
            Self::line(laid, list, item, 0, INSET, &row.name);
            Self::line(laid, list, item, 0, PRICE_X, &row.price.to_string());
            Self::line(laid, list, item, 0, HELD_X, &row.held.to_string());
        }
        for (line, event) in self.market.events.iter().enumerate() {
            Self::line(laid, list, EVENTS_ITEM, line, INSET, event);
        }
        Self::line(
            laid,
            list,
            STATUS_ITEM,
            0,
            INSET,
            &status_line(&self.market),
        );
        dialog.draw(list);
    }

    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// Lets go of Alt.
    fn release_keys(&mut self) {
        self.max_lot = false;
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::{Good, JunkId, MarketRow};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Bounds;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock "Trade": `DLOG` 1001, 426 x 252 and centred, and its
    /// seventeen user items, where 2, 3, 12, 15, 16 and 17 are disabled
    /// and 2, 16 and 17 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (272.0, 221.0, 99.0, 25.0, true),
            (129.0, 299.0, 68.0, 30.0, false),
            (38.0, 9.0, 352.0, 17.0, false),
            (38.0, 25.0, 352.0, 13.0, true),
            (38.0, 37.0, 352.0, 13.0, true),
            (38.0, 49.0, 352.0, 13.0, true),
            (38.0, 61.0, 352.0, 13.0, true),
            (38.0, 73.0, 352.0, 13.0, true),
            (38.0, 85.0, 352.0, 14.0, true),
            (38.0, 98.0, 352.0, 14.0, true),
            (38.0, 111.0, 352.0, 14.0, true),
            (38.0, 124.0, 352.0, 60.0, false),
            (60.0, 221.0, 99.0, 25.0, true),
            (166.0, 221.0, 99.0, 25.0, true),
            (41.0, 190.0, 346.0, 24.0, false),
            (212.0, 304.0, 68.0, 30.0, false),
            (178.0, 339.0, 68.0, 30.0, false),
        ];
        DialogTemplate {
            bounds: rect(32.0, 35.0, 426.0, 252.0),
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h, enabled)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 426) / 2, (768 - 252) / 2.
    const ORIGIN: Point = Point::new(299.0, 258.0);

    const FOOD: Good = Good::Commodity(0);
    const METAL: Good = Good::Commodity(4);
    const OPALS: Good = Good::Junk(JunkId(146));

    fn row(good: Good, name: &str, price: i64, held: u32) -> MarketRow {
        MarketRow {
            good,
            name: name.to_owned(),
            price,
            held,
            sold_here: true,
            bought_here: true,
        }
    }

    /// Food at 93 (2 held), metal at 200, and opals bought here only at
    /// 1500 (1 held), for a player with 1000 credits and 7 of 10 tons
    /// free, during a food surplus.
    fn market() -> Market {
        Market {
            rows: vec![
                row(FOOD, "Food", 93, 2),
                row(METAL, "Metal", 200, 0),
                MarketRow {
                    sold_here: false,
                    ..row(OPALS, "Opals", 1500, 1)
                },
            ],
            events: vec!["An enormous food surplus".to_owned()],
            cash: 1000,
            capacity: 10,
            free: 7,
        }
    }

    fn layout() -> (DialogTemplate, Rc<dyn TextMetrics>) {
        (template(), Rc::new(MonoMetrics))
    }

    fn screen_of(market: Market) -> TradeScreen {
        TradeScreen::new(Ok(layout()), market, ButtonStyle::STOCK)
    }

    fn screen() -> TradeScreen {
        screen_of(market())
    }

    fn drawn(screen: &TradeScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(commands: &[DrawCommand]) -> Vec<(String, Point)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
                _ => None,
            })
            .collect()
    }

    fn item(screen: &TradeScreen, number: usize) -> Bounds {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn press(screen: &mut TradeScreen, k: Key) {
        screen.input(&key(k, true, false));
    }

    fn click(screen: &mut TradeScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut TradeScreen, number: usize) {
        let at = item(screen, number).center();
        click(screen, at);
    }

    fn order(good: Good, direction: Direction, lot: Lot) -> Order {
        Order {
            good,
            direction,
            lot,
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(TRADE_DIALOG, 1001);
        assert_eq!(BACKGROUND, ImageKey::picture(8510));
        assert_eq!(
            [
                DONE_ITEM,
                HEADER_ITEM,
                FIRST_ROW_ITEM,
                ROWS,
                EVENTS_ITEM,
                BUY_ITEM,
                SELL_ITEM,
                STATUS_ITEM
            ],
            [1, 3, 4, 8, 12, 13, 14, 15]
        );
        assert_eq!((BUY_LABEL, SELL_LABEL), ("Buy", "Sell"));
        assert_eq!(
            (BUY_KEY, SELL_KEY, MAX_LOT_KEY),
            (Key::Char('b'), Key::Char('s'), Key::Alt)
        );
    }

    #[test]
    fn the_status_line_is_the_free_space_and_the_credits() {
        assert_eq!(
            status_line(&market()),
            "Free cargo space: 7 tons    Credits: 1000"
        );
    }

    #[test]
    fn it_is_centred_with_the_first_row_selected() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.dialog().expect("laid out").bounds().min, ORIGIN);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.top(), 0);
        assert!(!screen.closed());
        assert_eq!(screen.market(), &market());
        assert_eq!(item(&screen, 4), rect(337.0, 283.0, 352.0, 13.0));
    }

    #[test]
    fn it_draws_the_background_the_header_the_rows_the_events_the_status_then_the_dialog() {
        let screen = screen();
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: BACKGROUND,
                top_left: ORIGIN,
            }
        );
        let at = |item_number: usize, dx: f32, line: f32| {
            let bounds = item(&screen, item_number);
            Point::new(bounds.min.x + dx, bounds.min.y + line * 12.0)
        };
        let mut expected = vec![
            ("Commodity".to_owned(), at(3, INSET, 0.0)),
            ("Price".to_owned(), at(3, PRICE_X, 0.0)),
            ("Held".to_owned(), at(3, HELD_X, 0.0)),
            ("Food".to_owned(), at(4, INSET, 0.0)),
            ("93".to_owned(), at(4, PRICE_X, 0.0)),
            ("2".to_owned(), at(4, HELD_X, 0.0)),
            ("Metal".to_owned(), at(5, INSET, 0.0)),
            ("200".to_owned(), at(5, PRICE_X, 0.0)),
            ("0".to_owned(), at(5, HELD_X, 0.0)),
            ("Opals".to_owned(), at(6, INSET, 0.0)),
            ("1500".to_owned(), at(6, PRICE_X, 0.0)),
            ("1".to_owned(), at(6, HELD_X, 0.0)),
            ("An enormous food surplus".to_owned(), at(12, INSET, 0.0)),
            (
                "Free cargo space: 7 tons    Credits: 1000".to_owned(),
                at(15, INSET, 0.0),
            ),
        ];
        let mut dialog = DrawList::new();
        screen.dialog().expect("laid out").draw(&mut dialog);
        let buttons: Vec<DrawCommand> = dialog.iter().cloned().collect();
        expected.extend(texts(&buttons));
        assert_eq!(texts(&commands), expected);
        assert_eq!(commands[commands.len() - buttons.len()..], buttons[..]);
        let labels: Vec<_> = texts(&buttons).into_iter().map(|(t, _)| t).collect();
        assert_eq!(labels, ["Done", "Buy", "Sell"]);
    }

    #[test]
    fn the_list_is_in_geneva_white_under_a_dim_header() {
        let commands = drawn(&screen());
        let style = |wanted: &str| {
            commands.iter().find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font,
                    size,
                    color,
                    wrap_width,
                    ..
                } if text == wanted => Some((*font, *size, *color, *wrap_width)),
                _ => None,
            })
        };
        assert_eq!(
            style("Commodity"),
            Some((Font::Geneva, 10.0, Color::DIM, None))
        );
        for text in [
            "Food",
            "93",
            "An enormous food surplus",
            "Free cargo space: 7 tons    Credits: 1000",
        ] {
            assert_eq!(
                style(text),
                Some((Font::Geneva, 10.0, Color::WHITE, None)),
                "{text}"
            );
        }
    }

    /// The selected row's highlight: the row's item filled.
    fn highlight(screen: &TradeScreen, number: usize) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        fill_rect(&mut list, item(screen, number), SELECTED_COLOR);
        list.iter().cloned().collect()
    }

    fn contains(commands: &[DrawCommand], part: &[DrawCommand]) -> bool {
        commands.windows(part.len()).any(|window| window == part)
    }

    #[test]
    fn the_selected_row_is_highlighted_under_its_text() {
        let mut screen = screen();
        let commands = drawn(&screen);
        let first = highlight(&screen, 4);
        assert!(contains(&commands, &first));
        let at = commands
            .windows(first.len())
            .position(|window| window == first)
            .expect("drawn");
        let food = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Food"))
            .expect("drawn");
        assert!(at < food, "under the text");
        press(&mut screen, Key::Down);
        let commands = drawn(&screen);
        assert!(!contains(&commands, &first));
        assert!(contains(&commands, &highlight(&screen, 5)));
    }

    #[test]
    fn up_and_down_move_the_selection_and_stop_at_the_ends() {
        let mut screen = screen();
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(0));
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(1));
        screen.input(&key(Key::Down, true, true));
        assert_eq!(screen.selected(), Some(2), "repeats too");
        press(&mut screen, Key::Down);
        assert_eq!(screen.selected(), Some(2));
        screen.input(&key(Key::Up, false, false));
        assert_eq!(screen.selected(), Some(2), "not on release");
        press(&mut screen, Key::Up);
        assert_eq!(screen.selected(), Some(1));
    }

    #[test]
    fn a_click_on_a_row_selects_it_and_on_an_empty_row_nothing() {
        let mut screen = screen();
        click_item(&mut screen, 6);
        assert_eq!(screen.selected(), Some(2));
        click_item(&mut screen, 4);
        assert_eq!(screen.selected(), Some(0));
        click_item(&mut screen, 9);
        assert_eq!(screen.selected(), Some(0));
        assert_eq!(screen.take_sounds(), [], "rows are silent");
        assert_eq!(screen.take_order(), None);
    }

    /// `count` goods, numbered from 0, each 10 a ton.
    fn long(count: usize) -> Market {
        Market {
            rows: (0..count)
                .map(|n| {
                    let id = i16::try_from(n).expect("few");
                    row(Good::Junk(JunkId(id)), &format!("Good {n}"), 10, 0)
                })
                .collect(),
            ..market()
        }
    }

    fn names(screen: &TradeScreen) -> Vec<String> {
        texts(&drawn(screen))
            .into_iter()
            .map(|(text, _)| text)
            .filter(|text| text.starts_with("Good "))
            .collect()
    }

    #[test]
    fn the_list_scrolls_with_the_selection_past_eight_rows() {
        let mut screen = screen_of(long(11));
        let first: Vec<_> = (0..8).map(|n| format!("Good {n}")).collect();
        assert_eq!(names(&screen), first);
        for _ in 0..7 {
            press(&mut screen, Key::Down);
        }
        assert_eq!((screen.selected(), screen.top()), (Some(7), 0));
        press(&mut screen, Key::Down);
        assert_eq!((screen.selected(), screen.top()), (Some(8), 1));
        assert_eq!(names(&screen)[0], "Good 1");
        assert!(contains(&drawn(&screen), &highlight(&screen, 11)));
        for _ in 0..5 {
            press(&mut screen, Key::Down);
        }
        assert_eq!((screen.selected(), screen.top()), (Some(10), 3));
        assert_eq!(names(&screen).last().map(String::as_str), Some("Good 10"));
        click_item(&mut screen, 4);
        assert_eq!((screen.selected(), screen.top()), (Some(3), 3));
        press(&mut screen, Key::Up);
        assert_eq!((screen.selected(), screen.top()), (Some(2), 2));
    }

    #[test]
    fn buy_and_sell_ask_for_a_ton_of_the_selected_good() {
        let mut screen = screen();
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Buy, Lot::One))
        );
        assert_eq!(screen.take_order(), None, "once");
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Sell, Lot::One))
        );
        press(&mut screen, Key::Down);
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(METAL, Direction::Buy, Lot::One))
        );
        press(&mut screen, Key::Down);
        press(&mut screen, SELL_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(OPALS, Direction::Sell, Lot::One))
        );
    }

    #[test]
    fn holding_b_or_s_keeps_trading() {
        let mut screen = screen();
        screen.input(&key(BUY_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Buy, Lot::One))
        );
        screen.input(&key(SELL_KEY, true, true));
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Sell, Lot::One))
        );
        screen.input(&key(BUY_KEY, false, false));
        assert_eq!(screen.take_order(), None, "not on release");
    }

    #[test]
    fn with_alt_held_buy_and_sell_ask_for_the_most() {
        let mut screen = screen();
        press(&mut screen, MAX_LOT_KEY);
        click_item(&mut screen, BUY_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Buy, Lot::Max))
        );
        press(&mut screen, SELL_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Sell, Lot::Max))
        );
        screen.input(&key(MAX_LOT_KEY, true, true));
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Buy, Lot::Max))
        );
        screen.input(&key(MAX_LOT_KEY, false, false));
        press(&mut screen, BUY_KEY);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Buy, Lot::One))
        );
        press(&mut screen, MAX_LOT_KEY);
        screen.release_keys();
        click_item(&mut screen, SELL_ITEM);
        assert_eq!(
            screen.take_order(),
            Some(order(FOOD, Direction::Sell, Lot::One))
        );
    }

    /// Whether Buy and Sell draw enabled.
    fn enabled(screen: &TradeScreen) -> (bool, bool) {
        let label_color = |label: &str| {
            drawn(screen).into_iter().find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == label => Some(color),
                _ => None,
            })
        };
        let grey = ButtonStyle::STOCK.grey;
        (
            label_color(BUY_LABEL) != Some(grey),
            label_color(SELL_LABEL) != Some(grey),
        )
    }

    #[test]
    fn buy_and_sell_are_greyed_when_a_ton_could_not_be_traded_that_way() {
        let mut screen = screen();
        assert_eq!(enabled(&screen), (true, true), "food");
        press(&mut screen, Key::Down);
        assert_eq!(enabled(&screen), (true, false), "metal, none held");
        press(&mut screen, Key::Down);
        assert_eq!(enabled(&screen), (false, true), "opals, not sold here");
        click_item(&mut screen, BUY_ITEM);
        press(&mut screen, BUY_KEY);
        assert_eq!(screen.take_order(), None, "greyed asks for nothing");
        press(&mut screen, Key::Up);
        click_item(&mut screen, SELL_ITEM);
        press(&mut screen, SELL_KEY);
        assert_eq!(screen.take_order(), None);
        let broke = Market {
            cash: 10,
            ..market()
        };
        assert_eq!(enabled(&screen_of(broke)), (false, true));
        let empty = screen_of(Market {
            rows: Vec::new(),
            ..market()
        });
        assert_eq!(empty.selected(), None);
        assert_eq!(enabled(&empty), (false, false));
    }

    #[test]
    fn a_new_market_shows_and_regreys_keeping_the_selection() {
        let mut screen = screen();
        press(&mut screen, Key::Down);
        let mut bought = market();
        bought.rows[1].held = 7;
        bought.free = 0;
        bought.cash = 0;
        screen.set_market(bought.clone());
        assert_eq!(screen.market(), &bought);
        assert_eq!(screen.selected(), Some(1));
        assert_eq!(enabled(&screen), (false, true));
        let shown: Vec<_> = texts(&drawn(&screen)).into_iter().map(|(t, _)| t).collect();
        assert!(shown.contains(&"7".to_owned()), "{shown:?}");
        assert!(shown.contains(&"Free cargo space: 0 tons    Credits: 0".to_owned()));
        let mut scrolled = screen_of(long(11));
        for _ in 0..10 {
            press(&mut scrolled, Key::Down);
        }
        scrolled.set_market(long(3));
        assert_eq!(
            (scrolled.selected(), scrolled.top()),
            (Some(2), 0),
            "clamped"
        );
        scrolled.set_market(long(0));
        assert_eq!(scrolled.selected(), None);
        scrolled.set_market(long(5));
        assert_eq!(scrolled.selected(), Some(0));
    }

    #[test]
    fn done_return_and_escape_close_it() {
        let mut screen = screen();
        click_item(&mut screen, DONE_ITEM);
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = screen_of(market());
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
            assert_eq!(screen.take_order(), None);
        }
        let mut held = screen_of(market());
        held.input(&key(Key::Escape, true, true));
        held.input(&key(Key::Space, true, false));
        held.tick(Duration::from_secs(1));
        assert!(!held.closed());
    }

    #[test]
    fn its_buttons_sound_as_they_are_clicked_and_keys_are_silent() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: buy,
        });
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::ButtonUp)]);
        press(&mut screen, BUY_KEY);
        press(&mut screen, Key::Enter);
        assert_eq!(screen.take_sounds(), []);
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut screen = screen();
        let buy = item(&screen, BUY_ITEM).center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: buy,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_order(), None);
        assert!(format!("{screen:?}").contains("TextMetrics"));
    }

    fn problem(reason: &str) -> DrawCommand {
        DrawCommand::Text {
            text: format!(
                "Cannot show the trade center: {reason}. Press Return or Escape to close it."
            ),
            font: Font::Geneva,
            origin: PROBLEM_AT,
            size: PROBLEM_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        }
    }

    #[test]
    fn without_the_dialog_it_says_why_and_return_or_escape_closes_it() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen =
                TradeScreen::new(Err("no DLOG 1001".to_owned()), market(), ButtonStyle::STOCK);
            assert_eq!(screen.problem(), Some("no DLOG 1001"));
            assert!(screen.dialog().is_none());
            assert_eq!(drawn(&screen), [problem("no DLOG 1001")]);
            for other in [
                key(k, true, true),
                key(k, false, false),
                key(BUY_KEY, true, false),
                key(MAX_LOT_KEY, true, false),
                key(Key::Down, true, false),
            ] {
                screen.input(&other);
            }
            click(&mut screen, Point::new(500.0, 500.0));
            screen.cancel_pointer();
            assert!(!screen.closed());
            assert_eq!(screen.take_order(), None);
            screen.input(&key(k, true, false));
            assert!(screen.closed(), "{k:?}");
        }
    }
}
