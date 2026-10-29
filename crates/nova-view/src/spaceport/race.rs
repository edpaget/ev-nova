//! The bar's race: the Galactic Racing Network dialog Gamble opens with
//! cash above 0, laid out by the interface file's race dialog (`DLOG` and
//! `DITL` 1023; `_DoRaceDialog` @0x489ca in the `EV Nova` executable).
//!
//! The stock dialog is 470 x 230, centred on the main window
//! (`_CenterWindow` @0x48aaa), covering the 263 x 185 bar. Its `DITL` has
//! six items: Leave (1) at (243, 196), Help (2) at (129, 196), each 99 x
//! 25, and the four racers (3-6), 100 x 100 at x 13, 128, 243 and 357, y
//! 88. `_RaceDialogUpdate` (@0x4879d) fills it black, draws `PICT` 8529
//! over it, then `_DrawRaceButtons` (@0x26617) the six buttons:
//!
//! - Leave reads `STR#` 150 #34 "Cancel" while no winner is shown and no
//!   race has run in this opening, else #5 "Done" (@0x26698-0x266c4);
//!   Help is #58. Both are always enabled.
//! - Racer `r` (0-3) shows its winner picture (`PICT` 8550 + `r`) when it
//!   is the winner shown, its loser picture (8560 + `r`) while another is,
//!   and otherwise its pressed picture (8540 + `r`) while it is tracked
//!   and its normal one (8530 + `r`), as [`RacerLook`] says
//!   (@0x26775-0x26875). A picture button draws its picture alone:
//!   `_DrawButton`'s pressed flag adds nothing to one
//!   (@0x25ed7-0x25f0a).
//!
//! **Input** (`_RaceFilter` @0x4890e, `_TrackRaceButtons` @0x2689d): only
//! Return, Enter and Escape (key presses, never repeats) do anything,
//! pressing Leave (@0x4891b-0x48946); G, R, the map, player info and
//! mission info keys do nothing. A click tracks over the six buttons
//! alike, the one under the pointer drawn pressed and chosen on release;
//! a release outside chooses nothing. Leave closes it back to the bar
//! (@0x48b81).
//!
//! - **Help** asks for `dësc` 32764's text
//!   ([`RaceScreen::take_help_request`]), which whoever holds the session
//!   loads and shows over it (@0x48b89-0x48bd3).
//! - **A racer** (@0x48bdc-0x48e7b): with Option (Alt) held, the quantity
//!   dialog asks for the bet ([`Purpose::Bet`], opening at the asked
//!   bet's most), asked even when a winner is shown, and Cancel gives 0;
//!   otherwise the house bet. Then, with a winner shown or an asked 0, the
//!   winner and the racer chosen are let go and nothing is bet. Otherwise
//!   the racer is chosen, drawn pressed until the result shows, and the
//!   bet asked for ([`RaceScreen::take_bet`]): whoever holds the session
//!   runs it and shows the result ([`RaceScreen::show_result`], or
//!   [`RaceScreen::no_race`] when the session runs none), whose text shows
//!   over the race. The result shows beneath the text only once the text's
//!   movie has finished ([`RaceScreen::reveal`]), as the original's
//!   invalidated dialog redraws only once the text dialog's loop runs.
//!
//! It plays no sound of its own. Without its dialog it says why, and
//! Return or Escape still closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::gamble::{Bet, RACERS};

use super::layout::DONE_LABEL;
use super::quantity_ask::{Purpose, QuantityAsk};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, ItemSpec, Placement, Role};

/// The race dialog's `DLOG` (and `DITL`) ID (`GetNewDialog` 0x3ff
/// @0x489ef).
pub const RACE_DIALOG: i16 = 1023;
/// The picture the race is drawn over: `PICT` 8529 (0x2151).
pub const BACKGROUND: ImageKey = ImageKey::picture(8529);
/// Leave's item: Cancel, then Done.
pub const LEAVE_ITEM: usize = 1;
/// Help's item.
pub const HELP_ITEM: usize = 2;
/// The first racer's item; racer `r` is item 3 + `r`.
pub const FIRST_RACER_ITEM: usize = 3;
/// `STR#` 150 #34: Leave before any race.
pub const CANCEL_LABEL: &str = "Cancel";
/// `STR#` 150 #58.
pub const HELP_LABEL: &str = "Help";

/// How a racer's button looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RacerLook {
    /// No winner shown, not tracked: `PICT` 8530 + the racer.
    Normal,
    /// Tracked, or chosen and waiting for its result: 8540 + the racer.
    Pressed,
    /// The winner shown: 8550 + the racer.
    Winner,
    /// Another racer is the winner shown: 8560 + the racer.
    Loser,
}

impl RacerLook {
    /// `racer`'s picture in this look.
    #[must_use]
    pub fn picture(self, racer: u8) -> i16 {
        let first = match self {
            Self::Normal => 8530,
            Self::Pressed => 8540,
            Self::Winner => 8550,
            Self::Loser => 8560,
        };
        first + i16::from(racer)
    }
}

/// The race, laid out.
#[derive(Clone, Debug)]
struct Laid {
    template: DialogTemplate,
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

/// The race dialog, open over the bar.
#[derive(Clone, Debug)]
pub struct RaceScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    style: ButtonStyle,
    /// The asked bet's dialog, and whether Alt is held.
    quantity: QuantityAsk<u8>,
    /// The most an asked bet can stake.
    asked_max: i64,
    /// The winner shown (`*0x310b4a`).
    shown: Option<u8>,
    /// The racer chosen (`*0x310b48`).
    chosen: Option<u8>,
    /// The winner run but not shown yet, under its text's movie.
    pending: Option<u8>,
    /// How many races have run in this opening (`*0x310b40`).
    races: u32,
    /// The bet asked for since this was last taken.
    bet: Option<(u8, Bet)>,
    /// Whether Help has asked for its text since this was last taken.
    help: bool,
    closed: bool,
    sounds: Vec<Sound>,
}

impl RaceScreen {
    /// The race, laid out by `layout`: the race dialog's template and the
    /// metrics its labels are measured by, or why there are none; centred
    /// as `_CenterWindow` centres it, its buttons in `style`. Its asked
    /// bet opens in the quantity dialog laid out by `quantity` (or its
    /// fallback), at most `asked_max`.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        quantity: Option<DialogTemplate>,
        style: ButtonStyle,
        asked_max: i64,
    ) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let mut template = DialogTemplate {
                placement: Placement::Center,
                ..template
            };
            // Every button is enabled, and Leave and Help are labelled by
            // the race, as `_DrawRaceButtons` draws them, whatever the file
            // titles them.
            for (index, item) in template
                .items
                .iter_mut()
                .enumerate()
                .take(FIRST_RACER_ITEM - 1 + usize::from(RACERS))
            {
                item.enabled = true;
                if index + 1 < FIRST_RACER_ITEM {
                    item.kind = ItemSpec::User;
                }
            }
            let dialog = laid_dialog(&template, CANCEL_LABEL, &metrics, style);
            Laid {
                template,
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        Self {
            laid,
            style,
            quantity: QuantityAsk::new(quantity, style),
            asked_max,
            shown: None,
            chosen: None,
            pending: None,
            races: 0,
            bet: None,
            help: false,
            closed: false,
            sounds: Vec::new(),
        }
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// Why the race cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether the player has left it.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The asked bet's quantity dialog, while it is open.
    #[must_use]
    pub fn quantity(&self) -> Option<&QuantityAsk<u8>> {
        self.quantity.is_open().then_some(&self.quantity)
    }

    /// Leave's label: [`CANCEL_LABEL`] while no winner is shown and no race
    /// has run, else [`DONE_LABEL`].
    #[must_use]
    pub fn leave_label(&self) -> &'static str {
        if self.shown.is_none() && self.races == 0 {
            CANCEL_LABEL
        } else {
            DONE_LABEL
        }
    }

    /// How `racer` looks now ([`RacerLook`]).
    #[must_use]
    pub fn racer_look(&self, racer: u8) -> RacerLook {
        match self.shown {
            Some(winner) if winner == racer => RacerLook::Winner,
            Some(_) => RacerLook::Loser,
            None => {
                let tracked = self.dialog().and_then(Dialog::pressed_item)
                    == Some(FIRST_RACER_ITEM + usize::from(racer));
                if tracked || self.chosen == Some(racer) {
                    RacerLook::Pressed
                } else {
                    RacerLook::Normal
                }
            }
        }
    }

    /// The racer and the bet asked for since this was last taken, once.
    pub fn take_bet(&mut self) -> Option<(u8, Bet)> {
        self.bet.take()
    }

    /// Whether Help has asked for its text since this was last asked.
    pub fn take_help_request(&mut self) -> bool {
        std::mem::take(&mut self.help)
    }

    /// The race asked for ran, `winner` winning, and an asked bet can now
    /// stake at most `asked_max`: it counts, and the winner waits to show
    /// until [`RaceScreen::reveal`], the racer chosen drawn pressed till
    /// then.
    pub fn show_result(&mut self, winner: u8, asked_max: i64) {
        self.pending = Some(winner);
        self.asked_max = asked_max;
        self.races += 1;
        self.relabel();
    }

    /// Shows the winner waiting, if any: the result text's movie is over.
    pub fn reveal(&mut self) {
        if let Some(winner) = self.pending.take() {
            self.shown = Some(winner);
        }
    }

    /// Whether a winner is waiting to show.
    #[must_use]
    pub fn revealing(&self) -> bool {
        self.pending.is_some()
    }

    /// The race asked for ran none: the winner shown and the racer chosen
    /// are let go (@0x48e37-0x48e42).
    pub fn no_race(&mut self) {
        self.shown = None;
        self.chosen = None;
    }

    /// A click on `racer` with `bet` (after any ask): with a winner shown
    /// or a stake asked of 0 or less, lets them go; otherwise chooses it
    /// and asks for the bet.
    fn place(&mut self, racer: u8, bet: Bet) {
        if self.shown.is_some() || matches!(bet, Bet::Asked(..=0)) {
            self.no_race();
            return;
        }
        self.chosen = Some(racer);
        self.bet = Some((racer, bet));
    }

    /// A click chose `racer`: with Alt held asks for the bet, else bets
    /// the house bet.
    fn click_racer(&mut self, racer: u8) {
        if !self.quantity.alt_held() {
            self.place(racer, Bet::House);
            return;
        }
        if let Ok(laid) = &self.laid {
            self.quantity.open(
                racer,
                Purpose::Bet,
                self.asked_max,
                false,
                Rc::clone(&laid.metrics.0),
            );
        }
    }

    /// Rebuilds the dialog when Leave's label changes.
    fn relabel(&mut self) {
        let label = self.leave_label();
        let style = self.style;
        if let Ok(laid) = &mut self.laid {
            laid.dialog = laid_dialog(&laid.template, label, &laid.metrics.0, style);
        }
    }
}

/// The race's dialog over `template`, Leave labelled `leave`: Return and
/// Escape press Leave.
fn laid_dialog(
    template: &DialogTemplate,
    leave: &str,
    metrics: &Rc<dyn TextMetrics>,
    style: ButtonStyle,
) -> Dialog {
    let roles = [
        (LEAVE_ITEM, Role::Button(leave.to_owned())),
        (HELP_ITEM, Role::Button(HELP_LABEL.to_owned())),
    ];
    Dialog::new(template, &roles, Rc::clone(metrics))
        .with_buttons(ButtonSkin::NOVA, style)
        .with_default(Some(LEAVE_ITEM))
        .with_cancel(Some(LEAVE_ITEM))
}

impl Screen for RaceScreen {
    /// The quantity dialog takes every input while open. Otherwise Return,
    /// Enter and Escape press Leave, and a click presses the button it is
    /// released on; nothing else does anything. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.quantity.is_open() {
            if let Some((racer, asked)) = self.quantity.answer(input, &mut self.sounds) {
                self.place(racer, Bet::Asked(i64::from(asked)));
            }
            return ScreenAction::None;
        }
        self.quantity.follow_alt(input);
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
        match laid.dialog.input(input) {
            Some(DialogEvent::Item(LEAVE_ITEM)) => self.closed = true,
            Some(DialogEvent::Item(HELP_ITEM)) => self.help = true,
            Some(DialogEvent::Item(item)) => {
                if let Some(racer) = item
                    .checked_sub(FIRST_RACER_ITEM)
                    .and_then(|racer| u8::try_from(racer).ok())
                {
                    self.click_racer(racer);
                }
            }
            _ => {}
        }
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    /// Black over the dialog, `PICT` 8529, each racer's picture, then the
    /// buttons, and the quantity dialog over them.
    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(laid) => {
                let bounds = laid.dialog.bounds();
                fill_rect(list, bounds, Color::BLACK);
                list.picture(BACKGROUND, bounds.min);
                for racer in 0..RACERS {
                    let item = FIRST_RACER_ITEM + usize::from(racer);
                    if let Some(area) = laid.dialog.item_bounds(item) {
                        let picture = self.racer_look(racer).picture(racer);
                        list.picture(ImageKey::picture(picture), area.min);
                    }
                }
                laid.dialog.draw(list);
            }
            Err(reason) => {
                list.text(
                    format!("Cannot show the race: {reason}. Press Return or Escape to leave it."),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
        self.quantity.draw(list);
    }

    fn cancel_pointer(&mut self) {
        if self.quantity.cancel_pointer() {
            return;
        }
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    fn release_keys(&mut self) {
        self.quantity.release_keys();
    }

    /// The quantity dialog's sounds: the race plays none of its own.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::ItemTemplate;
    use crate::ui::quantity::{CANCEL_ITEM, OK_ITEM};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DITL` 1023, 470 x 230: Leave, Help and the four racers, the
    /// racers disabled as in the stock file.
    fn template() -> DialogTemplate {
        let item = |x, y, w, h, kind, enabled| ItemTemplate {
            bounds: rect(x, y, w, h),
            enabled,
            kind,
        };
        let racer = |x| item(x, 88.0, 100.0, 100.0, ItemSpec::User, false);
        DialogTemplate {
            bounds: rect(0.0, 0.0, 470.0, 230.0),
            placement: Placement::Fixed,
            items: vec![
                item(
                    243.0,
                    196.0,
                    99.0,
                    25.0,
                    ItemSpec::Button("Done".into()),
                    false,
                ),
                item(
                    129.0,
                    196.0,
                    99.0,
                    25.0,
                    ItemSpec::Button("Help".into()),
                    false,
                ),
                racer(13.0),
                racer(128.0),
                racer(243.0),
                racer(357.0),
            ],
        }
    }

    /// Stock `DITL` 1003: OK (1), the prompt (2), the field (3), Cancel (4).
    fn quantity_template() -> DialogTemplate {
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

    /// Where the dialog goes: (1024 - 470) / 2 and (768 - 230) / 2.
    const ORIGIN: Point = Point::new(277.0, 269.0);

    fn open() -> RaceScreen {
        RaceScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            Some(quantity_template()),
            ButtonStyle::STOCK,
            5_000,
        )
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn pointer(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn centre(screen: &RaceScreen, item: usize) -> Point {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(item)
            .expect("an item")
            .center()
    }

    /// Clicks `item`, down and up on it.
    fn click(screen: &mut RaceScreen, item: usize) {
        let at = centre(screen, item);
        screen.input(&pointer(true, at));
        screen.input(&pointer(false, at));
    }

    fn racer_item(racer: u8) -> usize {
        FIRST_RACER_ITEM + usize::from(racer)
    }

    fn texts(screen: &RaceScreen) -> Vec<String> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn pictures(screen: &RaceScreen) -> Vec<(ImageKey, Point)> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } => Some((*image, *top_left)),
                _ => None,
            })
            .collect()
    }

    fn looks(screen: &RaceScreen) -> [RacerLook; 4] {
        [0, 1, 2, 3].map(|racer| screen.racer_look(racer))
    }

    /// Clicks the quantity dialog's `item`.
    fn click_quantity(screen: &mut RaceScreen, item: usize) {
        let at = screen
            .quantity()
            .expect("asking")
            .dialog()
            .expect("open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
            .center();
        screen.input(&pointer(true, at));
        screen.input(&pointer(false, at));
    }

    #[test]
    fn the_named_values() {
        assert_eq!(RACE_DIALOG, 1023);
        assert_eq!(BACKGROUND, ImageKey::picture(8529));
        assert_eq!((LEAVE_ITEM, HELP_ITEM, FIRST_RACER_ITEM), (1, 2, 3));
        assert_eq!((CANCEL_LABEL, HELP_LABEL), ("Cancel", "Help"));
        assert_eq!(
            [
                RacerLook::Normal,
                RacerLook::Pressed,
                RacerLook::Winner,
                RacerLook::Loser
            ]
            .map(|look| look.picture(2)),
            [8532, 8542, 8552, 8562]
        );
    }

    #[test]
    fn it_is_laid_out_centred_at_the_ditls_rects() {
        let screen = open();
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(dialog.bounds(), Bounds::at(ORIGIN, 470.0, 230.0));
        let at = |x: f32, y: f32, w, h| Bounds::at(Point::new(ORIGIN.x + x, ORIGIN.y + y), w, h);
        assert_eq!(
            dialog.item_bounds(LEAVE_ITEM),
            Some(at(243.0, 196.0, 99.0, 25.0))
        );
        assert_eq!(
            dialog.item_bounds(HELP_ITEM),
            Some(at(129.0, 196.0, 99.0, 25.0))
        );
        for (racer, x) in [13.0, 128.0, 243.0, 357.0].into_iter().enumerate() {
            assert_eq!(
                dialog.item_bounds(FIRST_RACER_ITEM + racer),
                Some(at(x, 88.0, 100.0, 100.0))
            );
        }
    }

    #[test]
    fn it_draws_black_its_picture_the_racers_then_cancel_and_help() {
        let screen = open();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        let mut black = DrawList::new();
        fill_rect(&mut black, Bounds::at(ORIGIN, 470.0, 230.0), Color::BLACK);
        assert_eq!(list.iter().next(), black.iter().next(), "black first");
        let racers: Vec<(ImageKey, Point)> = (0..4)
            .map(|racer| {
                let item = screen
                    .dialog()
                    .expect("laid out")
                    .item_bounds(racer_item(racer))
                    .expect("a racer");
                (ImageKey::picture(8530 + i16::from(racer)), item.min)
            })
            .collect();
        let mut expected = vec![(BACKGROUND, ORIGIN)];
        expected.extend(racers);
        assert_eq!(pictures(&screen), expected);
        assert_eq!(texts(&screen), ["Cancel", "Help"]);
    }

    #[test]
    fn a_tracked_racer_draws_pressed_while_the_pointer_is_on_it() {
        let mut screen = open();
        let at = centre(&screen, racer_item(1));
        screen.input(&pointer(true, at));
        assert_eq!(
            looks(&screen),
            [
                RacerLook::Normal,
                RacerLook::Pressed,
                RacerLook::Normal,
                RacerLook::Normal
            ]
        );
        assert!(
            pictures(&screen).contains(&(
                ImageKey::picture(8541),
                screen
                    .dialog()
                    .expect("laid")
                    .item_bounds(4)
                    .expect("racer")
                    .min
            ))
        );
        screen.input(&Input::PointerMoved(Point::new(0.0, 0.0)));
        assert_eq!(screen.racer_look(1), RacerLook::Normal);
    }

    #[test]
    fn a_click_on_a_racer_bets_the_house_bet_and_holds_it_pressed_until_its_result_shows() {
        let mut screen = open();
        click(&mut screen, racer_item(2));
        assert_eq!(screen.take_bet(), Some((2, Bet::House)));
        assert_eq!(screen.take_bet(), None, "once");
        assert_eq!(
            screen.racer_look(2),
            RacerLook::Pressed,
            "after the release"
        );
        screen.show_result(0, 4_000);
        assert!(screen.revealing());
        assert_eq!(screen.racer_look(2), RacerLook::Pressed, "under the movie");
        assert_eq!(screen.leave_label(), DONE_LABEL, "a race has run");
        screen.reveal();
        assert!(!screen.revealing());
        assert_eq!(
            looks(&screen),
            [
                RacerLook::Winner,
                RacerLook::Loser,
                RacerLook::Loser,
                RacerLook::Loser
            ]
        );
        assert_eq!(texts(&screen), ["Done", "Help"]);
    }

    #[test]
    fn a_click_while_a_winner_shows_lets_it_go_with_no_bet_and_done_stays() {
        let mut screen = open();
        click(&mut screen, racer_item(0));
        let _ = screen.take_bet();
        screen.show_result(3, 4_000);
        screen.reveal();
        click(&mut screen, racer_item(1));
        assert_eq!(screen.take_bet(), None);
        assert_eq!(looks(&screen), [RacerLook::Normal; 4]);
        assert_eq!(screen.leave_label(), DONE_LABEL);
        click(&mut screen, racer_item(1));
        assert_eq!(screen.take_bet(), Some((1, Bet::House)), "then bets again");
    }

    #[test]
    fn a_race_the_session_runs_none_of_lets_the_racer_go() {
        let mut screen = open();
        click(&mut screen, racer_item(3));
        let _ = screen.take_bet();
        screen.no_race();
        assert_eq!(looks(&screen), [RacerLook::Normal; 4]);
        assert_eq!(screen.leave_label(), CANCEL_LABEL, "no race ran");
    }

    #[test]
    fn with_alt_held_a_racer_asks_the_bet_titled_bet_at_the_asked_most() {
        let mut screen = open();
        screen.input(&key(Key::Alt, true, false));
        click(&mut screen, racer_item(1));
        assert_eq!(screen.take_bet(), None, "asked first");
        let asking = screen.quantity().expect("asking");
        let dialog = asking.dialog().expect("open");
        assert_eq!(dialog.field().text(), "5000");
        assert!(texts(&screen).contains(&"Bet".to_owned()));
        assert!(texts(&screen).contains(&"Amount to bet:".to_owned()));
        click_quantity(&mut screen, OK_ITEM);
        assert!(screen.quantity().is_none());
        assert_eq!(screen.take_bet(), Some((1, Bet::Asked(5_000))));
    }

    #[test]
    fn an_asked_cancel_bets_nothing_and_lets_the_racer_go() {
        let mut screen = open();
        screen.input(&key(Key::Alt, true, false));
        click(&mut screen, racer_item(0));
        click_quantity(&mut screen, CANCEL_ITEM);
        assert_eq!(screen.take_bet(), None);
        assert_eq!(looks(&screen), [RacerLook::Normal; 4]);
    }

    #[test]
    fn the_ask_comes_even_with_a_winner_shown_and_is_then_thrown_away() {
        let mut screen = open();
        click(&mut screen, racer_item(0));
        let _ = screen.take_bet();
        screen.show_result(0, 4_000);
        screen.reveal();
        screen.input(&key(Key::Alt, true, false));
        click(&mut screen, racer_item(2));
        assert!(screen.quantity().is_some(), "asked");
        click_quantity(&mut screen, OK_ITEM);
        assert_eq!(screen.take_bet(), None, "thrown away");
        assert_eq!(looks(&screen), [RacerLook::Normal; 4]);
    }

    #[test]
    fn the_asked_most_follows_each_result() {
        let mut screen = open();
        click(&mut screen, racer_item(0));
        let _ = screen.take_bet();
        screen.show_result(1, 70);
        screen.reveal();
        click(&mut screen, racer_item(0));
        screen.input(&key(Key::Alt, true, false));
        click(&mut screen, racer_item(0));
        let field = screen
            .quantity()
            .and_then(QuantityAsk::dialog)
            .map(|dialog| dialog.field().text().to_owned());
        assert_eq!(field.as_deref(), Some("70"));
    }

    #[test]
    fn a_release_outside_every_button_chooses_nothing() {
        let mut screen = open();
        let at = centre(&screen, racer_item(0));
        screen.input(&pointer(true, at));
        screen.input(&pointer(false, ORIGIN));
        assert_eq!(screen.take_bet(), None);
        assert!(!screen.take_help_request());
        assert!(!screen.closed());
    }

    #[test]
    fn help_asks_for_its_text() {
        let mut screen = open();
        click(&mut screen, HELP_ITEM);
        assert!(screen.take_help_request());
        assert!(!screen.take_help_request(), "once");
        assert!(!screen.closed());
    }

    #[test]
    fn leave_return_enter_and_escape_leave_on_a_press_only() {
        let mut screen = open();
        click(&mut screen, LEAVE_ITEM);
        assert!(screen.closed());
        for leave in [Key::Enter, Key::Escape] {
            let mut screen = open();
            screen.input(&key(leave, true, true));
            assert!(!screen.closed(), "{leave:?} repeating");
            screen.input(&key(leave, true, false));
            assert!(screen.closed(), "{leave:?}");
        }
    }

    #[test]
    fn the_bars_keys_do_nothing_in_the_race() {
        let mut screen = open();
        for pressed in [
            Key::Char('g'),
            Key::Char('r'),
            crate::flight::view::MAP_KEY,
            crate::ui::mission_info::MISSION_INFO_KEY,
            crate::ui::player_info::PLAYER_INFO_KEY,
        ] {
            screen.input(&key(pressed, true, false));
        }
        assert_eq!(screen.take_bet(), None);
        assert!(!screen.take_help_request());
        assert!(!screen.closed());
        assert!(screen.quantity().is_none());
    }

    #[test]
    fn without_its_dialog_it_says_why_and_return_or_escape_leaves() {
        for leave in [Key::Enter, Key::Escape] {
            let mut screen =
                RaceScreen::new(Err("no DLOG 1023".to_owned()), None, ButtonStyle::STOCK, 5);
            assert_eq!(screen.problem(), Some("no DLOG 1023"));
            assert!(texts(&screen)[0].contains("no DLOG 1023"));
            screen.input(&key(Key::Char('g'), true, false));
            assert!(!screen.closed());
            screen.input(&key(leave, true, false));
            assert!(screen.closed(), "{leave:?}");
        }
    }
}
