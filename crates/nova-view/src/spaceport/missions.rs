//! The mission screens of the spaceport: the mission computer
//! ([`MissionComputerScreen`], `DLOG`/`DITL` 1006, `_DoMissionDialog`
//! @0xa37cf in the `EV Nova` executable) and the mission info screen
//! ([`MissionInfoScreen`], `DLOG`/`DITL` 1012, `_DoMissionInfoDialog`
//! @0x9e19b).
//!
//! The stock mission computer is 510 x 201, centred, over `PICT` 8505:
//! the heading "The following missions are available here:" (`STR#` 2002
//! #359) in item 8 and today's medium date in item 11; the offers' names
//! listed in item 2 ([`RowList`]), the first selected; the selected
//! offer's name in item 5 and its text in item 4; and Accept (item 1,
//! `STR#` 150 #26) and Leave (item 7, #1). Items 6, 9 and 10 (the map,
//! player info and mission info) lie outside the dialog, reached only by
//! keys: M ([`MAP_KEY`](crate::flight::view::MAP_KEY), a press) asks for
//! the course map marked for the selected offer, or for none with no row
//! selected ([`MissionComputerScreen::take_map_request`]). The screen
//! enables its buttons itself.
//!
//! What it shows is the session's [`MissionComputer`], decided in
//! `nova-sim`. Accept asks for the selected mission
//! ([`MissionComputerScreen::take_accept`]), and is greyed with nothing
//! listed; whoever holds the session accepts it and hands the computer
//! back ([`MissionComputerScreen::set_computer`]), as the original closes
//! the dialog after an accept and opens it again. Leave, Return, Enter,
//! Escape and L leave (`_MissionFilter` @0x9f885-0x9f8af); A
//! ([`ACCEPT_KEY`], a press) accepts the selected offer, as Accept does
//! (@0x9f8b4-0x9f8c7). Up and Down (key repeats too) move the selection a
//! row, wrapping from the last offer to the first and back (@0x9faf2,
//! @0x9fce3), and show the offer selected. Under the engine reading of
//! the `dialog_focus_ring` rule ([`FocusRing::Off`]), a Tab press steps
//! to the next offer as Down does, Shift or not (@0x9f8cc; the original
//! steps on autoKey too, but dialog keys are press-only by the user's
//! decision of 2026-10-23). By default ([`FocusRing::Tab`]) Tab moves the
//! dialog's button focus instead and the selection stays. Without its
//! dialog, the screen says why, and Return or Escape still leaves.
//!
//! The stock mission info screen is 471 x 155, centred, over `PICT` 8517:
//! the heading "Currently active missions:" (`STR#` 2002 #360) in item 3
//! and today's medium date in item 7; the active missions shown listed in
//! item 2, a failed one marked "• ", the first selected; the selected
//! one's `QuickBrief` in item 4, which shows its deadline through `<DL>`;
//! and Done (item 1, `STR#` 150 #5) and Abort (item 5, #35), greyed unless
//! the selected mission can be aborted (`_DrawMissionInfoButtons`
//! @0x28359). Item 6, the map, lies outside it: M asks for the course map
//! marked for the selected mission ([`MissionInfoScreen::take_map_request`]),
//! unless the screen was opened over an offer or the computer. What it shows is the
//! session's [`MissionInfo`]; Abort asks for the selected slot
//! ([`MissionInfoScreen::take_abort`]), with no confirmation, and whoever
//! holds the session aborts it and hands the info back
//! ([`MissionInfoScreen::set_info`]), which closes the screen once
//! nothing is listed (@0x9e5eb-0x9e610). By the engine reading of the
//! `mission_info_rebuild` rule the info handed back carries the
//! original's table, every active slot hidden ones included
//! ([`MissionInfo::table`]): each row then picks the slot at its place,
//! a step can select past the rows, and the screen stays open while any
//! mission is active (@0x9e5f9-0x9e60a, @0x9e69f-0x9e6d8). With missions left, an abort
//! leaves nothing selected (@0x9e62d-0x9e632, @0x9e66a-0x9e680): item 4 is
//! an empty black box (@0x987d1-0x987e6), Abort is greyed, and M asks for
//! the map marked for nothing (@0x9e3e5-0x9e3f1). A refused abort keeps
//! the selection (@0x9e583).
//!
//! Done, Return, Enter, Escape and the mission info key
//! ([`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), a
//! press, so the key that opens the screen closes it) close it
//! (`_MissionInfoFilter` @0x9d73c-0x9d769). Up and Down (key repeats too)
//! move the selection a row, wrapping from the last mission to the first
//! and back (@0x9d833, @0x9da56), and from no selection take the first or
//! the last (@0x9d839, @0x9da60). Under [`FocusRing::Off`], a Tab press
//! steps to the next mission as Down does, Shift or not (@0x9d778; the
//! original steps on autoKey too, but dialog keys are press-only by the
//! user's decision of 2026-10-23). By default ([`FocusRing::Tab`]) Tab
//! moves the dialog's button focus instead and the selection stays. The
//! filter plays no sound of its own: only the close's
//! ([`UiSound::MissionInfoClosed`]). Without its dialog, the screen says
//! why, and Return or Escape alone closes it.
//!
//! Both lists are the List Manager's, as [`RowList`] runs them. A press in
//! item 2 is held until its release: the one selection follows the
//! pointer across the rows, and past the list's top or bottom edge the
//! list scrolls a row at a time, the row scrolled in selected. The text,
//! Accept's or Abort's grey and M's mark follow the selection at the
//! release, when `LClick` returns and the filter reads the selection back
//! (`_MissionFilter` @0xa0003-0xa007c, `_MissionInfoFilter`
//! @0x9dc5d-0x9dcd8), not at the press; a cancelled press ends as a
//! release. A press below the last row selects nothing: item 4 is an
//! empty black box, Accept and A or Abort ask for nothing, and M asks for
//! the map marked for nothing. The rows don't scroll on their own: when
//! they don't fit, a scroll bar does. On the mission info screen it is
//! carved from item 2's right, the rows narrowed by 15
//! (`_SetupMissionInfoList` @0x9df6f-0x9df90). On the computer it lies in
//! item 3's column, right of item 2 (`_SetupMissionList` @0xa07a6), and a
//! press in item 3 goes to it (the second `LClick`, @0xa01c5-0xa01e7);
//! when the rows fit, the list is widened over item 3 (@0xa07a8-0xa07e3).
//! A press in item 3 over a list that fits is left out: there the
//! original's `LClick` moves the highlight and nothing reads it back. The
//! bar's last column and its top and bottom rows lie outside item 2 (and
//! item 3), whose `PtInRect` doesn't hand them to `LClick`; the engine's
//! item test counts the item's right and bottom edges in (the half-open
//! tests are filed apart). Neither list plays a sound.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::mission::screens::{
    ABORT_LABEL, ACCEPT_LABEL, ACTIVE_MISSIONS, AVAILABLE_HERE, FAILED_MARK,
};
use nova_sim::{ComputerRow, InfoRow, MissionComputer, MissionId, MissionInfo};

use super::layout::{DONE_LABEL, LEAVE_LABEL};
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::flight::MapOpener;
use crate::flight::view::is_map_press;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key, MouseButton};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::{TextMetrics, truncate_end};
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, FocusRing, Role, TextBacking};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::player_info::is_player_info_press;
use crate::ui::row_list::{BarRoom, ListPress, RowList};

/// The mission computer's `DLOG` (and `DITL`) ID.
pub const COMPUTER_DIALOG: i16 = 1006;
/// The picture the mission computer is drawn over.
pub const COMPUTER_BACKGROUND: ImageKey = ImageKey::picture(8505);

/// Accept's item.
pub const ACCEPT_ITEM: usize = 1;
/// The list's item.
pub const LIST_ITEM: usize = 2;
/// The item right of the list: the list covers it when its rows fit, and
/// its scroll bar lies in it when they don't (`_SetupMissionList`
/// @0xa07a6-0xa07e3).
pub const BAR_ITEM: usize = 3;
/// The selected offer's text's item.
pub const TEXT_ITEM: usize = 4;
/// The selected offer's name's item.
pub const NAME_ITEM: usize = 5;
/// Leave's item.
pub const LEAVE_ITEM: usize = 7;
/// The heading's item.
pub const HEADING_ITEM: usize = 8;
/// Today's date's item.
pub const DATE_ITEM: usize = 11;

/// The offer's name's size, in Times: `_MissionDialogUpdate` sets font
/// 20 at 18 for item 5 (@0x9f312-0x9f325).
pub const NAME_SIZE: f32 = 18.0;

/// Fills a mission's tags for its offer on the visit under way, as
/// `_MungeBriefing` does: the session's
/// `nova_sim::Session::offer_text`, through whoever flies the ship.
pub trait OfferNames {
    /// `raw` with its tags filled for `mission`'s offer, or `None` when
    /// it can't be (no session, or the mission no longer offered).
    /// It takes `&mut self`, as the fill is a screen's ask of the flight.
    fn munged(&mut self, mission: MissionId, raw: &str) -> Option<String>;
}

/// Where the list goes when the template has no item 2: nowhere.
const NO_LIST: Bounds = Bounds {
    min: Point::new(0.0, 0.0),
    max: Point::new(0.0, 0.0),
};

/// The keys that leave the mission computer, besides Leave: Return (and
/// Enter), Escape and L (`_MissionFilter` @0x9f885-0x9f8af, @0xa0212).
pub const LEAVE_KEYS: [Key; 4] = [Key::Enter, Key::NumpadEnter, Key::Escape, Key::KeyL];

/// The key that accepts the selected offer: A (key code 0x00), on a
/// press only, giving Accept when a row is selected and nothing otherwise
/// (`_MissionFilter` @0x9f8b4-0x9f8c7, @0xa0201).
pub const ACCEPT_KEY: Key = Key::KeyA;

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// Whether `input` goes to the list in `dialog`'s item `list`: a press on
/// it.
fn on_list(dialog: &Dialog, list: usize, input: &Input) -> bool {
    match *input {
        Input::PointerButton {
            pressed: true, at, ..
        } => dialog
            .item_bounds(list)
            .is_some_and(|bounds| bounds.contains(at)),
        _ => false,
    }
}

/// What `input` did to `rows`, if it went to them: while they hold a
/// press, every pointer move and the left release, as `LClick` keeps the
/// pointer until the button comes up; otherwise a press on one of
/// `dialog`'s `items`, as each filter's `PtInRect` hands one to `LClick`.
fn list_pointer(
    dialog: &Dialog,
    rows: &mut RowList,
    items: &[usize],
    input: &Input,
) -> Option<ListPress> {
    let held = rows.tracking()
        && matches!(
            input,
            Input::PointerMoved(_)
                | Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: false,
                    ..
                }
        );
    (held || items.iter().any(|&item| on_list(dialog, item, input))).then(|| rows.input(input))
}

/// Which way `input` steps a mission list, forward (`true`) or back, if it
/// does, with the dialog's focus ring `ring`. `_MissionFilter` steps on
/// Tab (@0x9f8cc), Down (@0x9faf2) and Up (@0x9fce3), and
/// `_MissionInfoFilter` on Tab and Down (@0x9d778) and Up (@0x9d9ac), each
/// wrapping at the ends, and neither tests Shift. Tab steps only under the
/// engine reading, where the focus ring doesn't take it, and on a press
/// only, though the original steps on autoKey too; the arrows keep their
/// repeats.
fn list_step(ring: FocusRing, input: &Input) -> Option<bool> {
    match *input {
        Input::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        } if ring == FocusRing::Off => Some(true),
        Input::Key {
            key: key @ (Key::ArrowUp | Key::ArrowDown),
            pressed: true,
            ..
        } => Some(key == Key::ArrowDown),
        _ => None,
    }
}

/// The mission computer, laid out.
#[derive(Clone, Debug)]
struct Laid {
    dialog: Dialog,
    rows: RowList,
    metrics: MetricsHandle,
}

/// The mission computer of the stellar landed on.
#[derive(Clone, Debug)]
pub struct MissionComputerScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    computer: MissionComputer,
    /// The mission Accept asked for, until it is taken.
    accepted: Option<MissionId>,
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) has asked for the mission info screen
    /// since this was last taken.
    info_requested: bool,
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen.
    player_info_requested: bool,
    /// The map [`MAP_KEY`](crate::flight::view::MAP_KEY) asked for, with
    /// the selected row's offer, until it is taken.
    map_request: Option<MapOpener>,
    /// The offer shown selected whose text is to be loaded, until it is
    /// taken.
    text_request: Option<MissionId>,
    closed: bool,
}

impl MissionComputerScreen {
    /// `computer`, laid out by `layout`: the mission computer's template
    /// and the metrics its text is measured by, or why there are none;
    /// its buttons labelled in `style`.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        computer: MissionComputer,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(mut template, metrics)| {
            for item in [ACCEPT_ITEM, LEAVE_ITEM] {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let text = |text: &str, backing| Role::ScrollText {
                text: text.to_owned(),
                font: Font::Geneva,
                size: BODY_SIZE,
                color: BODY_COLOR,
                backing,
            };
            // The offer's name is in Times 18 (@0x9f312-0x9f325).
            let name = Role::ScrollText {
                text: String::new(),
                font: Font::Times,
                size: NAME_SIZE,
                color: BODY_COLOR,
                backing: Some(TextBacking::BLACK_BOX),
            };
            // The offer's text and name are white on black boxes, selected
            // or not: `_MissionDialogUpdate` inverts each under its
            // `TETextBox` (item 5 @0x9f47a, item 4 @0x9f575) and paints it
            // black with no selection (@0x9f481-0x9f496, @0x9f57c-0x9f591).
            let black = Some(TextBacking::BLACK_BOX);
            let roles = [
                (ACCEPT_ITEM, Role::Button(ACCEPT_LABEL.to_owned())),
                (LEAVE_ITEM, Role::Button(LEAVE_LABEL.to_owned())),
                (TEXT_ITEM, text("", black)),
                (NAME_ITEM, name),
                (HEADING_ITEM, text(AVAILABLE_HERE, None)),
                (DATE_ITEM, text("", None)),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
            Laid {
                rows: computer_rows(&dialog, Vec::new(), &*metrics),
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        let mut screen = Self {
            laid,
            computer: MissionComputer {
                rows: Vec::new(),
                today: String::new(),
            },
            accepted: None,
            info_requested: false,
            player_info_requested: false,
            map_request: None,
            text_request: None,
            closed: false,
        };
        screen.set_computer(computer);
        screen
    }

    /// The mission computer with its buttons taking the keyboard focus as `ring`
    /// says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_focus_ring(ring);
        }
        self
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// The list, if the dialog could be laid out.
    #[must_use]
    pub fn rows(&self) -> Option<&RowList> {
        self.laid.as_ref().ok().map(|laid| &laid.rows)
    }

    /// Why the mission computer cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The mission computer shown.
    #[must_use]
    pub fn computer(&self) -> &MissionComputer {
        &self.computer
    }

    /// Whether the player has left it.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The mission Accept asked for since this was last taken, once.
    pub fn take_accept(&mut self) -> Option<MissionId> {
        self.accepted.take()
    }

    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) has asked for the mission info screen
    /// since this was last asked (`_DoMissionDialog` item 10,
    /// @0xa3e37-0xa3ec1).
    pub fn take_info_request(&mut self) -> bool {
        std::mem::take(&mut self.info_requested)
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// has asked for the player info screen since this was last asked
    /// (`_MissionFilter` @0x9f82c, `_DoMissionDialog` @0xa3ffc).
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.player_info_requested)
    }

    /// The map [`MAP_KEY`](crate::flight::view::MAP_KEY) has asked for
    /// since this was last asked, once, with the selected row's offer, or
    /// none with no row selected: `_MissionFilter` gives item 6
    /// (@0x9f7fa-0x9f818), and `_DoMissionDialog` opens the course map
    /// marked for that offer, or for none (@0xa3c7b-0xa3daa; see
    /// [`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)).
    pub fn take_map_request(&mut self) -> Option<MapOpener> {
        self.map_request.take()
    }

    /// Shows `computer` in place of the one shown, keeping the selected
    /// mission selected while it is listed, and otherwise the first.
    pub fn set_computer(&mut self, computer: MissionComputer) {
        let kept = self.selected_mission();
        self.computer = computer;
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let names = self.computer.rows.iter().map(|row| row.name.clone());
        laid.rows = computer_rows(&laid.dialog, names.collect(), &*laid.metrics.0);
        if let Some(index) = kept.and_then(|mission| {
            self.computer
                .rows
                .iter()
                .position(|row| row.mission == mission)
        }) {
            laid.rows.select(index);
        }
        laid.dialog.set_text(DATE_ITEM, &self.computer.today);
        laid.dialog
            .set_greyed(ACCEPT_ITEM, self.computer.rows.is_empty());
        self.show_selected();
    }

    /// The mission selected, if any.
    fn selected_mission(&self) -> Option<MissionId> {
        let index = self.rows()?.selected()?;
        self.computer.rows.get(index).map(|row| row.mission)
    }

    /// Shows the selected offer's text, or none, and asks for it to be
    /// loaded ([`MissionComputerScreen::take_text_request`]). The name
    /// is blanked: only [`MissionComputerScreen::show_row`] fills it, cut
    /// to its box and then munged, as the original does. The request is
    /// answered in the same input pass (the app serves the mission screens
    /// after every input that can open the computer or move its
    /// selection), so the blank name never reaches a drawn frame.
    fn show_selected(&mut self) {
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let row = laid
            .rows
            .selected()
            .and_then(|index| self.computer.rows.get(index));
        laid.dialog.set_text(NAME_ITEM, "");
        laid.dialog
            .set_text(TEXT_ITEM, row.map_or("", |row| row.text.as_str()));
        self.text_request = row.map(|row| row.mission);
    }

    /// The offer shown selected since this was last taken, once, whose
    /// text is to be loaded as the original's selection loads it
    /// (`_MissionFilter` @0x9f9bd): whoever flies the ship answers with
    /// [`MissionComputerScreen::show_row`].
    pub fn take_text_request(&mut self) -> Option<MissionId> {
        self.text_request.take()
    }

    /// Shows `row`, its text loaded, while its offer is the one selected;
    /// otherwise nothing changes. Its name is `row.raw_name` cut to item
    /// 5's width less 1 in Times 18 ([`truncate_end`]) and then filled by
    /// `names`, as `_MissionDialogUpdate` cuts the stored name with
    /// `TruncString` before `_MungeBriefing` (@0x9f37d-0x9f40c). A name
    /// `names` can't fill stays blank: an empty black box.
    pub fn show_row(&mut self, row: &ComputerRow, names: &mut impl OfferNames) {
        if self.selected_mission() != Some(row.mission) {
            return;
        }
        if let Ok(laid) = &mut self.laid {
            let width = laid
                .dialog
                .item_bounds(NAME_ITEM)
                .map_or(0.0, |item| item.width() - 1.0);
            let cut = truncate_end(
                &laid.metrics.0,
                Font::Times,
                NAME_SIZE,
                &row.raw_name,
                width,
            );
            let name = names.munged(row.mission, &cut).unwrap_or_default();
            laid.dialog.set_text(NAME_ITEM, &name);
            laid.dialog.set_text(TEXT_ITEM, &row.text);
        }
    }

    /// Activates dialog item `item`: Accept asks for the selected mission
    /// and Leave leaves. Anything else does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            ACCEPT_ITEM => self.accepted = self.selected_mission(),
            LEAVE_ITEM => self.closed = true,
            _ => {}
        }
    }
}

impl Screen for MissionComputerScreen {
    /// The leave keys leave and A accepts; Up, Down and (under
    /// [`FocusRing::Off`]) Tab step the list, wrapping, and a press on the
    /// list selects a row; everything else goes to the dialog. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && LEAVE_KEYS.contains(&key)
        {
            self.closed = true;
            return ScreenAction::None;
        }
        if is_map_press(input) {
            self.map_request = Some(MapOpener::Computer(self.selected_mission()));
            return ScreenAction::None;
        }
        if is_mission_info_press(input) {
            self.info_requested = true;
            return ScreenAction::None;
        }
        if is_player_info_press(input) {
            self.player_info_requested = true;
            return ScreenAction::None;
        }
        let Ok(laid) = &mut self.laid else {
            return ScreenAction::None;
        };
        if let Input::Key {
            key: ACCEPT_KEY,
            pressed: true,
            repeat: false,
        } = *input
        {
            self.accepted = self.selected_mission();
            return ScreenAction::None;
        }
        if let Some(forward) = list_step(laid.dialog.focus_ring(), input) {
            if laid.rows.step_wrapping(forward) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        // Item 3 goes to the bar (the second `LClick`, @0xa01c5-0xa01e7);
        // over a list that fits, it is left out (see the module docs).
        let items: &[usize] = if laid.rows.bar().is_some() {
            &[LIST_ITEM, BAR_ITEM]
        } else {
            &[LIST_ITEM]
        };
        if let Some(press) = list_pointer(&laid.dialog, &mut laid.rows, items, input) {
            if press == (ListPress::Released { changed: true }) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        let event = laid.dialog.input(input);
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// A press held on the list's bar steps on, and a drag past its ends
    /// scrolls on.
    fn tick(&mut self, dt: Duration) {
        if let Ok(laid) = &mut self.laid {
            laid.rows.tick(dt);
        }
    }

    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(laid) => {
                list.picture(COMPUTER_BACKGROUND, laid.dialog.bounds().min);
                laid.dialog.draw(list);
                laid.rows.draw(list);
            }
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the mission computer: {reason}. Press Return or Escape to leave it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// The dialog lets go of its press, and the list ends its own as a
    /// release would, loading the row it reached.
    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
            if laid.rows.cancel_pointer() == (ListPress::Released { changed: true }) {
                self.show_selected();
            }
        }
    }
}

/// The mission computer's list of `names` in `dialog`'s item 2, its bar
/// beside it in item 3's column, or, when they fit, the list widened to
/// item 3's right (`_SetupMissionList` @0xa07a6-0xa07e3).
fn computer_rows(dialog: &Dialog, names: Vec<String>, metrics: &dyn TextMetrics) -> RowList {
    let rect = dialog.item_bounds(LIST_ITEM).unwrap_or(NO_LIST);
    let fitted_right = dialog
        .item_bounds(BAR_ITEM)
        .map_or(rect.max.x, |item| item.max.x);
    RowList::new(names, rect, &metrics, BarRoom::Beside { fitted_right })
}

/// The mission info screen's list of `names` in `dialog`'s item 2, its
/// bar carved from the item's right when they don't fit
/// (`_SetupMissionInfoList` @0x9df6f-0x9df90).
fn info_rows(dialog: &Dialog, names: Vec<String>, metrics: &dyn TextMetrics) -> RowList {
    let rect = dialog.item_bounds(INFO_LIST_ITEM).unwrap_or(NO_LIST);
    RowList::new(names, rect, &metrics, BarRoom::Inside)
}

/// The mission info screen's `DLOG` (and `DITL`) ID.
pub const INFO_DIALOG: i16 = 1012;
/// The picture the mission info screen is drawn over.
pub const INFO_BACKGROUND: ImageKey = ImageKey::picture(8517);

/// Done's item.
pub const INFO_DONE_ITEM: usize = 1;
/// The list's item.
pub const INFO_LIST_ITEM: usize = 2;
/// The heading's item.
pub const INFO_HEADING_ITEM: usize = 3;
/// The selected mission's text's item.
pub const INFO_TEXT_ITEM: usize = 4;
/// Abort's item.
pub const ABORT_ITEM: usize = 5;
/// Today's date's item.
pub const INFO_DATE_ITEM: usize = 7;

/// The mission info screen, laid out.
#[derive(Clone, Debug)]
struct InfoLaid {
    dialog: Dialog,
    rows: RowList,
    metrics: MetricsHandle,
}

/// The mission info screen: the active missions, read and aborted.
#[derive(Clone, Debug)]
pub struct MissionInfoScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<InfoLaid, String>,
    info: MissionInfo,
    /// The slot Abort asked for, until it is taken.
    abort: Option<u8>,
    /// The slot shown selected whose text is to be loaded, until it is
    /// taken.
    text_request: Option<u8>,
    /// Whether [`MAP_KEY`](crate::flight::view::MAP_KEY) may open the
    /// map: not when the screen is opened over an offer or the computer.
    map_allowed: bool,
    /// The map [`MAP_KEY`](crate::flight::view::MAP_KEY) asked for, with
    /// the selected slot, until it is taken.
    map_request: Option<MapOpener>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl MissionInfoScreen {
    /// `info`, laid out by `layout`: the mission info screen's template
    /// and the metrics its text is measured by, or why there are none;
    /// its buttons labelled in `style`.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        info: MissionInfo,
        style: ButtonStyle,
    ) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let text = |text: &str, backing| Role::ScrollText {
                text: text.to_owned(),
                font: Font::Geneva,
                size: BODY_SIZE,
                color: BODY_COLOR,
                backing,
            };
            // The QuickBrief is white on a black box, selected or not:
            // `_MissionInfoDialogUpdate` inverts it under its `TETextBox`
            // (@0x987ca) and paints it black with no selection
            // (@0x987d1-0x987e6).
            let roles = [
                (INFO_DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (ABORT_ITEM, Role::Button(ABORT_LABEL.to_owned())),
                (INFO_HEADING_ITEM, text(ACTIVE_MISSIONS, None)),
                (INFO_TEXT_ITEM, text("", Some(TextBacking::BLACK_BOX))),
                (INFO_DATE_ITEM, text("", None)),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(INFO_DONE_ITEM))
                .with_cancel(Some(INFO_DONE_ITEM));
            InfoLaid {
                rows: info_rows(&dialog, Vec::new(), &*metrics),
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        let mut screen = Self {
            laid,
            info: MissionInfo::shown(Vec::new(), String::new()),
            abort: None,
            text_request: None,
            map_allowed: true,
            map_request: None,
            closed: false,
            sounds: vec![Sound::Ui(UiSound::MissionInfoOpened)],
        };
        screen.set_info(info);
        screen
    }

    /// The mission info screen with its buttons taking the keyboard focus as `ring`
    /// says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_focus_ring(ring);
        }
        self
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok().map(|laid| &laid.dialog)
    }

    /// The list, if the dialog could be laid out.
    #[must_use]
    pub fn rows(&self) -> Option<&RowList> {
        self.laid.as_ref().ok().map(|laid| &laid.rows)
    }

    /// Why the mission info screen cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The mission info shown.
    #[must_use]
    pub fn info(&self) -> &MissionInfo {
        &self.info
    }

    /// Whether it has closed.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// This screen, letting [`MAP_KEY`](crate::flight::view::MAP_KEY)
    /// open the map or not: `_DoMissionInfoDialog` opens none while
    /// `_missionDialog` is set (@0x9e3c6-0x9e3cd), as it is when the
    /// screen is opened over an offer (its item 7) or the computer (its
    /// item 10).
    #[must_use]
    pub fn with_map_allowed(self, allowed: bool) -> Self {
        Self {
            map_allowed: allowed,
            ..self
        }
    }

    /// The map [`MAP_KEY`](crate::flight::view::MAP_KEY) has asked for
    /// since this was last asked, once, with the selected slot, if any:
    /// `_MissionInfoFilter` gives item 6 (@0x9d70a-0x9d72d), and
    /// `_DoMissionInfoDialog` opens the course map marked for that
    /// mission (@0x9e3bc-0x9e4d8; see
    /// [`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)),
    /// unless the map isn't allowed ([`MissionInfoScreen::with_map_allowed`]),
    /// when the key does nothing.
    pub fn take_map_request(&mut self) -> Option<MapOpener> {
        self.map_request.take()
    }

    /// The slot Abort asked for since this was last taken, once.
    pub fn take_abort(&mut self) -> Option<u8> {
        self.abort.take()
    }

    /// Shows `info` in place of the one shown: the first row selected as
    /// the screen opens, and after that the selected slot kept while it is
    /// listed. When an abort takes it, nothing is selected
    /// (`_DoMissionInfoDialog` @0x9e62d-0x9e632, @0x9e66a-0x9e680). The
    /// list's labels are `info`'s rows, and each place picks from
    /// [`MissionInfo::picks`]: the rows, or the table an abort left by the
    /// engine reading of `mission_info_rebuild`, which the steps wrap over
    /// past the rows ([`RowList::with_extent`]). The selection, its text,
    /// Abort's grey, Abort, M's mark and the text asked for all read the
    /// pick at the selected place. It closes once nothing is picked
    /// (@0x9e5eb-0x9e610).
    pub fn set_info(&mut self, info: MissionInfo) {
        let kept = self.selected_row().map(|row| row.slot);
        self.info = info;
        if self.info.picks().is_empty() {
            self.close();
        }
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let names = self.info.rows.iter().map(|row| {
            if row.failed {
                format!("{FAILED_MARK}{}", row.name)
            } else {
                row.name.clone()
            }
        });
        laid.rows = info_rows(&laid.dialog, names.collect(), &*laid.metrics.0)
            .with_extent(self.info.picks().len());
        if let Some(slot) = kept {
            match self.info.picks().iter().position(|row| row.slot == slot) {
                Some(index) => laid.rows.select(index),
                None => laid.rows.clear_selection(),
            }
        }
        laid.dialog.set_text(INFO_DATE_ITEM, &self.info.today);
        self.show_selected();
    }

    /// The pick at the selected place, if any.
    fn selected_row(&self) -> Option<&InfoRow> {
        let index = self.rows()?.selected()?;
        self.info.picks().get(index)
    }

    /// Shows the selected mission's text, and greys Abort unless it can
    /// be aborted (`_DrawMissionInfoButtons` @0x28359).
    fn show_selected(&mut self) {
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let row = laid
            .rows
            .selected()
            .and_then(|index| self.info.picks().get(index));
        laid.dialog
            .set_text(INFO_TEXT_ITEM, row.map_or("", |row| row.text.as_str()));
        laid.dialog
            .set_greyed(ABORT_ITEM, !row.is_some_and(|row| row.abortable));
        self.text_request = row.map(|row| row.slot);
    }

    /// The slot shown selected since this was last taken, once, whose
    /// `QuickBrief` is to be loaded as the original's selection loads it
    /// (`_MissionInfoFilter` @0x9d89f): whoever flies the ship answers
    /// with [`MissionInfoScreen::show_text`].
    pub fn take_text_request(&mut self) -> Option<u8> {
        self.text_request.take()
    }

    /// Shows `text`, slot `slot`'s loaded, while that slot is the one
    /// selected; otherwise nothing changes.
    pub fn show_text(&mut self, slot: u8, text: &str) {
        if self.selected_row().map(|row| row.slot) != Some(slot) {
            return;
        }
        if let Ok(laid) = &mut self.laid {
            laid.dialog.set_text(INFO_TEXT_ITEM, text);
        }
    }

    /// Activates dialog item `item`: Done closes, and Abort asks for the
    /// selected slot. Anything else does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            INFO_DONE_ITEM => self.close(),
            ABORT_ITEM => self.abort = self.selected_row().map(|row| row.slot),
            _ => {}
        }
    }

    /// Closes it, beeping `_beepSnd[2]` (@0x9e791) the first time when it
    /// was laid out; without its dialog the original returns before it
    /// (@0x9e20b).
    fn close(&mut self) {
        if !std::mem::replace(&mut self.closed, true) && self.laid.is_ok() {
            self.sounds.push(Sound::Ui(UiSound::MissionInfoClosed));
        }
    }
}

impl Screen for MissionInfoScreen {
    /// The map key asks for the map and the mission info key closes it, in
    /// `_MissionInfoFilter`'s order (@0x9d70a, @0x9d756); Up, Down and (under
    /// [`FocusRing::Off`]) Tab step the list, wrapping, and a press on the
    /// list selects a row; everything else goes to the dialog. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if is_map_press(input) {
            if self.map_allowed {
                self.map_request = Some(MapOpener::MissionInfo(
                    self.selected_row().map(|row| row.slot),
                ));
            }
            return ScreenAction::None;
        }
        let Ok(laid) = &mut self.laid else {
            if let Input::Key {
                key: Key::Enter | Key::NumpadEnter | Key::Escape,
                pressed: true,
                repeat: false,
            } = *input
            {
                self.close();
            }
            return ScreenAction::None;
        };
        if is_mission_info_press(input) {
            self.close();
            return ScreenAction::None;
        }
        if let Some(forward) = list_step(laid.dialog.focus_ring(), input) {
            if laid.rows.step_wrapping(forward) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        if let Some(press) = list_pointer(&laid.dialog, &mut laid.rows, &[INFO_LIST_ITEM], input) {
            if press == (ListPress::Released { changed: true }) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        let event = laid.dialog.input(input);
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// A press held on the list's bar steps on, and a drag past its ends
    /// scrolls on.
    fn tick(&mut self, dt: Duration) {
        if let Ok(laid) = &mut self.laid {
            laid.rows.tick(dt);
        }
    }

    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(laid) => {
                list.picture(INFO_BACKGROUND, laid.dialog.bounds().min);
                laid.dialog.draw(list);
                laid.rows.draw(list);
            }
            Err(reason) => {
                list.text(
                    format!(
                        "Cannot show the mission info: {reason}. Press Return or Escape to close it."
                    ),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// The dialog lets go of its press, and the list ends its own as a
    /// release would, loading the row it reached.
    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
            if laid.rows.cancel_pointer() == (ListPress::Released { changed: true }) {
                self.show_selected();
            }
        }
    }

    /// Its opening's and closing's sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ui::mission_info::MISSION_INFO_KEY;
    use nova_sim::ComputerRow;

    use super::*;
    use crate::draw::{DrawCommand, fill_command};
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};
    use crate::ui::scroll_text::ScrollText;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// `items`, each (x, y, w, h) and enabled or not, as user items.
    fn items(items: &[(f32, f32, f32, f32, bool)]) -> Vec<ItemTemplate> {
        items
            .iter()
            .map(|&(x, y, w, h, enabled)| ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled,
                kind: ItemSpec::User,
            })
            .collect()
    }

    /// Stock `DLOG` 1006, 510 x 201 and centred, with its eleven user
    /// items as `DITL` 1006 has them, 6, 9 and 10 parked outside it.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(-72.0, 101.0, 510.0, 201.0),
            placement: Placement::Center,
            items: items(&[
                (266.0, 170.0, 99.0, 25.0, true),
                (10.0, 30.0, 195.0, 144.0, true),
                (205.0, 30.0, 15.0, 144.0, true),
                (233.0, 60.0, 267.0, 93.0, false),
                (233.0, 34.0, 269.0, 21.0, false),
                (175.0, 246.0, 68.0, 30.0, false),
                (368.0, 170.0, 99.0, 25.0, true),
                (14.0, 3.0, 396.0, 15.0, false),
                (285.0, 269.0, 32.0, 32.0, false),
                (204.0, 297.0, 68.0, 30.0, false),
                (311.0, 0.0, 87.0, 15.0, false),
            ]),
        }
    }

    /// Where the dialog goes: (1024 - 510) / 2 and (768 - 201) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(257.0, 283.0);

    /// Offer `mission`, its name `name` (stored as it reads) and its text.
    fn row(mission: i16, name: &str, text: &str) -> ComputerRow {
        ComputerRow {
            mission: MissionId(mission),
            name: name.to_owned(),
            raw_name: name.to_owned(),
            text: text.to_owned(),
        }
    }

    /// A session's munge: fills `<DST>` with "Mars", or refuses every
    /// offer, recording what it is asked.
    #[derive(Default)]
    struct FakeNames {
        asked: std::cell::RefCell<Vec<(MissionId, String)>>,
        refuse: bool,
    }

    impl OfferNames for FakeNames {
        fn munged(&mut self, mission: MissionId, raw: &str) -> Option<String> {
            self.asked.borrow_mut().push((mission, raw.to_owned()));
            (!self.refuse).then(|| raw.replace("<DST>", "Mars"))
        }
    }

    /// Two offers: 130 "Run to Mars" and 131 "Walk".
    fn computer() -> MissionComputer {
        MissionComputer {
            rows: vec![
                row(130, "Run to Mars", "Take this to Mars."),
                row(131, "Walk", "Go on foot."),
            ],
            today: "June 23rd, 1177 NC".to_owned(),
        }
    }

    fn screen_of(computer: MissionComputer) -> MissionComputerScreen {
        MissionComputerScreen::new(
            Ok((template(), Rc::new(MonoMetrics))),
            computer,
            ButtonStyle::STOCK,
        )
    }

    fn screen() -> MissionComputerScreen {
        screen_of(computer())
    }

    fn drawn(screen: &MissionComputerScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(screen: &MissionComputerScreen) -> Vec<(String, Color)> {
        drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text, color)),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click_at(screen: &mut MissionComputerScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(screen: &mut MissionComputerScreen, number: usize) {
        let at = screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        click_at(screen, at);
    }

    /// Item `item`'s text, its lines joined by spaces.
    fn item_text(screen: &MissionComputerScreen, item: usize) -> String {
        screen
            .dialog()
            .expect("laid out")
            .text(item)
            .expect("a text item")
            .lines()
            .join(" ")
    }

    /// Whether `label` draws enabled.
    fn enabled(screen: &MissionComputerScreen, label: &str) -> bool {
        texts(screen)
            .into_iter()
            .find_map(|(text, color)| (text == label).then_some(color))
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    #[test]
    fn the_named_values() {
        assert_eq!(COMPUTER_DIALOG, 1006);
        assert_eq!(COMPUTER_BACKGROUND, ImageKey::picture(8505));
        assert_eq!(
            [
                ACCEPT_ITEM,
                LIST_ITEM,
                TEXT_ITEM,
                NAME_ITEM,
                LEAVE_ITEM,
                HEADING_ITEM,
                DATE_ITEM
            ],
            [1, 2, 4, 5, 7, 8, 11]
        );
    }

    #[test]
    fn it_shows_the_heading_the_date_the_offers_and_the_first_ones_name_and_text() {
        let screen = screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(item_text(&screen, HEADING_ITEM), AVAILABLE_HERE);
        assert_eq!(item_text(&screen, DATE_ITEM), "June 23rd, 1177 NC");
        let rows = screen.rows().expect("laid out");
        assert_eq!(rows.rows(), ["Run to Mars", "Walk"]);
        assert_eq!(rows.selected(), Some(0));
        assert_eq!(
            rows.row_bounds(0).map(|row| row.min),
            Some(Point::new(ORIGIN.x + 10.0, ORIGIN.y + 30.0)),
            "in item 2"
        );
        assert_eq!(
            item_text(&screen, NAME_ITEM),
            "",
            "the name waits for show_row"
        );
        assert_eq!(item_text(&screen, TEXT_ITEM), "Take this to Mars.");
        for label in [ACCEPT_LABEL, LEAVE_LABEL] {
            assert!(enabled(&screen, label), "{label}");
        }
    }

    #[test]
    fn selecting_another_offer_shows_its_text_and_blanks_the_name_until_it_loads() {
        let mut screen = screen();
        screen.input(&key(Key::ArrowDown));
        assert_eq!(item_text(&screen, NAME_ITEM), "");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Go on foot.");
        let first = screen
            .rows()
            .and_then(|rows| rows.row_bounds(0))
            .expect("shown")
            .center();
        click_at(&mut screen, first);
        assert_eq!(item_text(&screen, NAME_ITEM), "");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Take this to Mars.");
        assert!(!screen.closed());
    }

    #[test]
    fn no_selection_by_key_click_or_new_computer_shows_a_name_unloaded() {
        // The name is filled only by `show_row`: never the list's munged
        // name, nor the stored one uncut.
        let mut screen = screen();
        let mut names = FakeNames::default();
        let _ = screen.take_text_request();
        screen.show_row(&row(130, "Run to Mars", "Take this to Mars."), &mut names);
        assert_eq!(item_text(&screen, NAME_ITEM), "Run to Mars");
        screen.input(&key(Key::ArrowDown));
        assert_eq!(item_text(&screen, NAME_ITEM), "", "by key");
        screen.input(&key(Key::ArrowUp));
        assert_eq!(item_text(&screen, NAME_ITEM), "", "back by key");
        screen.show_row(&row(130, "Run to Mars", "Take this to Mars."), &mut names);
        let second = screen
            .rows()
            .and_then(|rows| rows.row_bounds(1))
            .expect("shown")
            .center();
        click_at(&mut screen, second);
        assert_eq!(item_text(&screen, NAME_ITEM), "", "by a click");
        screen.show_row(&row(131, "Walk", "Go on foot."), &mut names);
        screen.set_computer(computer());
        assert_eq!(item_text(&screen, NAME_ITEM), "", "by a new computer");
        assert_eq!(
            screen.take_text_request(),
            Some(MissionId(131)),
            "asked again"
        );
    }

    #[test]
    fn a_name_too_wide_for_its_box_is_cut_before_it_is_filled_and_drawn_in_times_18() {
        // Item 5 is 269 wide: cut to 268, Times 18 in `MonoMetrics` is
        // 9.9 a character, so 26 characters and the ellipsis.
        let raw = "Passengers for <DST> and on to the far stars";
        let mut screen = screen_of(MissionComputer {
            rows: vec![ComputerRow {
                raw_name: raw.to_owned(),
                ..row(130, "Passengers for Mars and on to the far stars", "Go.")
            }],
            ..computer()
        });
        let mut names = FakeNames::default();
        let _ = screen.take_text_request();
        let loaded = screen.computer().rows[0].clone();
        screen.show_row(&loaded, &mut names);
        assert_eq!(
            *names.asked.borrow(),
            [(MissionId(130), "Passengers for <DST> and o…".to_owned())],
            "the stored name, cut, not the whole"
        );
        assert_eq!(item_text(&screen, NAME_ITEM), "Passengers for Mars and o…");
        let area = screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(NAME_ITEM))
            .expect("item 5");
        let drawn = drawn(&screen);
        let name = drawn
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    font,
                    size,
                    color,
                    origin,
                    ..
                } if area.contains(*origin) => Some((text.as_str(), *font, *size, *color)),
                _ => None,
            })
            .expect("the name drawn");
        assert_eq!(
            name,
            (
                "Passengers for Mars and o…",
                Font::Times,
                18.0,
                Color::WHITE
            )
        );
    }

    #[test]
    fn the_name_is_cut_to_one_less_than_its_items_width() {
        // Item 5 made 268 wide: the name is cut to 267, so 27 Times 18
        // characters (267.3) no longer fit, though they fit 268.
        let mut narrow = template();
        narrow.items[NAME_ITEM - 1].bounds.max.x -= 1.0;
        let raw = "abcdefghijklmnopqrstuvwxyz0";
        let mut screen = MissionComputerScreen::new(
            Ok((narrow, Rc::new(MonoMetrics))),
            MissionComputer {
                rows: vec![row(130, raw, "Go.")],
                ..computer()
            },
            ButtonStyle::STOCK,
        );
        let mut names = FakeNames::default();
        let _ = screen.take_text_request();
        screen.show_row(&row(130, raw, "Go."), &mut names);
        assert_eq!(
            *names.asked.borrow(),
            [(MissionId(130), "abcdefghijklmnopqrstuvwxy…".to_owned())]
        );
    }

    #[test]
    fn a_name_the_session_cannot_fill_stays_blank() {
        let mut screen = screen();
        let mut names = FakeNames {
            refuse: true,
            ..FakeNames::default()
        };
        let _ = screen.take_text_request();
        screen.show_row(&row(130, "Run to Mars", "Loaded."), &mut names);
        assert_eq!(item_text(&screen, NAME_ITEM), "");
        assert_eq!(
            item_text(&screen, TEXT_ITEM),
            "Loaded.",
            "the text still shows"
        );
    }

    #[test]
    fn each_offer_shown_selected_asks_once_for_its_text_to_be_loaded() {
        let mut screen = screen();
        assert_eq!(
            screen.take_text_request(),
            Some(MissionId(130)),
            "the first, as it opens (_DoMissionDialog @0xa3a96)"
        );
        assert_eq!(screen.take_text_request(), None, "once");
        screen.input(&key(Key::ArrowDown));
        assert_eq!(screen.take_text_request(), Some(MissionId(131)));
        screen.input(&key(Key::Escape));
        assert_eq!(screen.take_text_request(), None, "no new selection");
    }

    #[test]
    fn a_loaded_row_shows_its_name_and_text_while_it_is_selected() {
        let mut screen = screen();
        let _ = screen.take_text_request();
        let mut names = FakeNames::default();
        screen.show_row(&row(131, "Walk loaded", "Not now."), &mut names);
        assert_eq!(
            item_text(&screen, TEXT_ITEM),
            "Take this to Mars.",
            "131 is not selected"
        );
        assert!(names.asked.borrow().is_empty(), "nothing filled for 131");
        screen.show_row(&row(130, "Run, loaded", "Loaded text."), &mut names);
        assert_eq!(item_text(&screen, NAME_ITEM), "Run, loaded");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Loaded text.");
        assert_eq!(screen.take_text_request(), None, "showing asks nothing");
    }

    #[test]
    fn accept_asks_for_the_selected_mission_once() {
        let mut screen = screen();
        assert_eq!(screen.take_accept(), None);
        screen.input(&key(Key::ArrowDown));
        click_item(&mut screen, ACCEPT_ITEM);
        assert_eq!(screen.take_accept(), Some(MissionId(131)));
        assert_eq!(screen.take_accept(), None, "once");
        assert!(!screen.closed());
        let mut off = super::tests::screen().with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Space));
        assert_eq!(off.take_accept(), None, "Space accepts nothing");
    }

    /// Three offers: 130, 131 and 132.
    fn three() -> MissionComputer {
        let mut three = computer();
        three.rows.push(row(132, "Fly", "Take wing."));
        three
    }

    /// Each selected mission, shown and asked for, after each of `inputs`.
    fn stepped(screen: &mut MissionComputerScreen, inputs: &[Input]) -> Vec<Option<i16>> {
        screen.take_text_request();
        inputs
            .iter()
            .map(|input| {
                screen.input(input);
                let selected = screen.selected_mission();
                assert_eq!(screen.take_text_request(), selected, "asked for");
                selected.map(|mission| mission.0)
            })
            .collect()
    }

    #[test]
    fn up_and_down_wrap_at_the_ends_of_the_computer() {
        let mut screen = screen_of(three());
        assert_eq!(
            stepped(
                &mut screen,
                &[key(Key::ArrowUp), key(Key::ArrowDown), key(Key::ArrowDown)]
            ),
            [Some(132), Some(130), Some(131)]
        );
        let repeat = Input::Key {
            key: Key::ArrowDown,
            pressed: true,
            repeat: true,
        };
        assert_eq!(
            stepped(&mut screen, &[repeat, repeat]),
            [Some(132), Some(130)],
            "repeats step and wrap too"
        );
        let release = Input::Key {
            key: Key::ArrowUp,
            pressed: false,
            repeat: false,
        };
        screen.input(&release);
        assert_eq!(screen.selected_mission(), Some(MissionId(130)));
        assert_eq!(screen.take_text_request(), None, "a release steps nothing");
    }

    #[test]
    fn tab_steps_the_offers_and_wraps_under_the_engine_reading() {
        let mut screen = screen_of(three()).with_focus_ring(FocusRing::Off);
        assert_eq!(
            stepped(&mut screen, &[key(Key::Tab); 3]),
            [Some(131), Some(132), Some(130)]
        );
        assert_eq!(item_text(&screen, TEXT_ITEM), "Take this to Mars.");
        assert_eq!(screen.dialog().and_then(Dialog::focus), None, "no ring");
        assert_eq!(screen.take_accept(), None);
        assert!(!screen.closed());
    }

    /// `_MissionFilter`'s Tab branch tests no Shift (@0x9f8cc).
    #[test]
    fn shift_tab_also_steps_forward_on_the_computer() {
        let mut screen = screen_of(three()).with_focus_ring(FocusRing::Off);
        screen.input(&key(Key::ShiftLeft));
        screen.input(&key(Key::Tab));
        assert_eq!(screen.selected_mission(), Some(MissionId(131)));
    }

    /// The original steps on autoKey too (@0x9f883), but dialog keys are
    /// press-only by the user's decision of 2026-10-23.
    #[test]
    fn a_tab_repeat_or_release_steps_no_offer() {
        let mut screen = screen().with_focus_ring(FocusRing::Off);
        for (pressed, repeat) in [(true, true), (false, false)] {
            screen.input(&Input::Key {
                key: Key::Tab,
                pressed,
                repeat,
            });
        }
        assert_eq!(screen.selected_mission(), Some(MissionId(130)));
    }

    #[test]
    fn tab_up_and_down_on_an_empty_computer_step_nothing() {
        let mut screen = screen_of(MissionComputer {
            rows: Vec::new(),
            ..computer()
        })
        .with_focus_ring(FocusRing::Off);
        assert_eq!(
            stepped(
                &mut screen,
                &[key(Key::Tab), key(Key::ArrowUp), key(Key::ArrowDown)]
            ),
            [None, None, None]
        );
    }

    #[test]
    fn under_the_focus_ring_tab_moves_the_button_focus_not_the_offer() {
        let mut screen = screen_of(three());
        screen.take_text_request();
        screen.input(&key(Key::Tab));
        assert_eq!(screen.selected_mission(), Some(MissionId(130)));
        assert_eq!(screen.take_text_request(), None);
        assert_eq!(screen.dialog().and_then(Dialog::focus), Some(ACCEPT_ITEM));
        screen.input(&key(Key::Space));
        assert_eq!(screen.take_accept(), Some(MissionId(130)));
    }

    #[test]
    fn a_accepts_the_selected_offer() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let mut screen = screen().with_focus_ring(ring);
            screen.input(&key(Key::ArrowDown));
            screen.input(&key(ACCEPT_KEY));
            assert_eq!(screen.take_accept(), Some(MissionId(131)), "{ring:?}");
            assert!(!screen.closed());
            for (pressed, repeat) in [(true, true), (false, false)] {
                screen.input(&Input::Key {
                    key: ACCEPT_KEY,
                    pressed,
                    repeat,
                });
            }
            assert_eq!(screen.take_accept(), None, "keyDown only");
            let mut empty = screen_of(MissionComputer {
                rows: Vec::new(),
                ..computer()
            })
            .with_focus_ring(ring);
            empty.input(&key(ACCEPT_KEY));
            assert_eq!(empty.take_accept(), None, "nothing selected");
        }
        assert_eq!(ACCEPT_KEY, Key::KeyA);
    }

    #[test]
    fn with_nothing_listed_accept_is_greyed_and_asks_nothing() {
        let mut screen = screen_of(MissionComputer {
            rows: Vec::new(),
            ..computer()
        });
        assert!(!enabled(&screen, ACCEPT_LABEL));
        click_item(&mut screen, ACCEPT_ITEM);
        assert_eq!(screen.take_accept(), None);
        assert_eq!(item_text(&screen, NAME_ITEM), "");
        assert_eq!(item_text(&screen, TEXT_ITEM), "");
    }

    #[test]
    fn leave_return_escape_and_l_leave() {
        for leave in LEAVE_KEYS {
            let mut screen = screen();
            screen.input(&key(leave));
            assert!(screen.closed(), "{leave:?}");
            assert_eq!(screen.take_accept(), None, "{leave:?} accepts nothing");
        }
        let mut repeating = screen();
        repeating.input(&Input::Key {
            key: Key::KeyL,
            pressed: true,
            repeat: true,
        });
        assert!(!repeating.closed(), "a repeat does nothing");
        let mut screen = screen();
        click_item(&mut screen, LEAVE_ITEM);
        assert!(screen.closed());
        assert_eq!(
            LEAVE_KEYS,
            [Key::Enter, Key::NumpadEnter, Key::Escape, Key::KeyL]
        );
    }

    #[test]
    fn a_new_computer_keeps_the_selected_mission_while_it_is_listed() {
        let mut screen = screen();
        screen.input(&key(Key::ArrowDown));
        screen.set_computer(MissionComputer {
            rows: vec![row(129, "Fly", "Up."), row(131, "Walk", "Go on foot.")],
            ..computer()
        });
        assert_eq!(screen.rows().and_then(RowList::selected), Some(1));
        assert_eq!(item_text(&screen, TEXT_ITEM), "Go on foot.");
        assert_eq!(item_text(&screen, NAME_ITEM), "", "until it loads");
        screen.set_computer(MissionComputer {
            rows: vec![row(129, "Fly", "Up.")],
            today: "June 24th, 1177 NC".to_owned(),
        });
        assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
        assert_eq!(item_text(&screen, TEXT_ITEM), "Up.");
        assert_eq!(item_text(&screen, NAME_ITEM), "", "until it loads");
        assert_eq!(item_text(&screen, DATE_ITEM), "June 24th, 1177 NC");
        assert_eq!(screen.computer().rows.len(), 1);
        screen.set_computer(MissionComputer {
            rows: Vec::new(),
            ..computer()
        });
        assert!(!enabled(&screen, ACCEPT_LABEL));
    }

    #[test]
    fn it_draws_its_picture_then_its_dialog_then_the_list() {
        let screen = screen();
        let commands = drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: COMPUTER_BACKGROUND,
                top_left: ORIGIN,
            }
        );
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(dialog.bounds().min, ORIGIN);
        let mut alone = DrawList::new();
        dialog.draw(&mut alone);
        screen.rows().expect("laid out").draw(&mut alone);
        assert_eq!(commands[1..], alone.iter().cloned().collect::<Vec<_>>()[..]);
        assert!(
            texts(&screen)
                .iter()
                .any(|(text, color)| text == "Take this to Mars." && *color == BODY_COLOR)
        );
    }

    /// Where in `commands` item `area`'s black fill is, and where its first
    /// text run is, if it has one.
    fn fill_and_text(commands: &[DrawCommand], area: Bounds) -> (usize, Option<usize>) {
        let fill = commands
            .iter()
            .position(|command| *command == fill_command(area, Color::BLACK))
            .expect("filled black");
        let text = commands.iter().position(
            |command| matches!(command, DrawCommand::Text { origin, .. } if area.contains(*origin)),
        );
        (fill, text)
    }

    /// A text long enough to overflow either screen's text box.
    fn long_text(tag: &str) -> String {
        (0..120)
            .map(|n| format!("{tag}{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Checks that `text`, which overflows its box, is shown from its first
    /// line, and that `commands` draw exactly the whole lines that fit, the
    /// last one's line ending inside the box.
    fn shows_whole_lines_from_the_top(commands: &[DrawCommand], text: &ScrollText) {
        assert!(text.overflows(), "the text is longer than its box");
        assert_eq!(text.first(), 0, "shown from its first line");
        let rect = text.rect();
        let runs: Vec<(String, Point)> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if rect.contains(*origin) => {
                    Some((text.clone(), *origin))
                }
                _ => None,
            })
            .collect();
        assert!(!runs.is_empty(), "lines drawn: {commands:#?}");
        assert_eq!(
            runs.iter()
                .map(|(line, _)| line.as_str())
                .collect::<Vec<_>>(),
            text.lines()[..text.visible()],
        );
        let line_height = MonoMetrics.line_height(Font::Geneva, BODY_SIZE);
        let last = runs.last().expect("a line").1;
        assert!(
            last.y + line_height <= rect.min.y + rect.height(),
            "the last line drawn is whole"
        );
    }

    #[test]
    fn up_and_down_move_the_offers_and_never_scroll_their_black_boxed_text() {
        // As at base: Up and Down go to the list, so nothing scrolls the
        // offer's text; each offer shows from its first line.
        let mut screen = screen_of(MissionComputer {
            rows: vec![
                row(130, "Run to Mars", &long_text("a")),
                row(131, "Walk", &long_text("b")),
            ],
            ..computer()
        });
        let text = |screen: &MissionComputerScreen| {
            screen
                .dialog()
                .expect("laid out")
                .text(TEXT_ITEM)
                .expect("the offer's text")
                .clone()
        };
        shows_whole_lines_from_the_top(&drawn(&screen), &text(&screen));
        for (input, selected) in [
            (key(Key::ArrowDown), 1),
            (
                Input::Key {
                    key: Key::ArrowDown,
                    pressed: true,
                    repeat: true,
                },
                0,
            ),
            (key(Key::ArrowUp), 1),
            (key(Key::ArrowUp), 0),
        ] {
            screen.input(&input);
            assert_eq!(screen.selected_mission(), Some(MissionId(130 + selected)));
            let shown = text(&screen);
            assert!(shown.lines()[0].starts_with(["a0", "b0"][selected as usize]));
            shows_whole_lines_from_the_top(&drawn(&screen), &shown);
        }
    }

    #[test]
    fn the_offers_text_and_name_draw_over_black_boxes_filling_their_items() {
        let mut screen = screen();
        let _ = screen.take_text_request();
        screen.show_row(
            &row(130, "Run to Mars", "Take this to Mars."),
            &mut FakeNames::default(),
        );
        let commands = drawn(&screen);
        let dialog = screen.dialog().expect("laid out");
        for item in [TEXT_ITEM, NAME_ITEM] {
            let area = dialog.item_bounds(item).expect("an item");
            let (fill, text) = fill_and_text(&commands, area);
            let text = text.unwrap_or_else(|| panic!("item {item}'s text: {commands:#?}"));
            assert!(fill < text, "item {item}: the box is under the text");
        }
    }

    #[test]
    fn with_no_offer_selected_the_text_and_name_are_still_black_boxes() {
        // `_MissionDialogUpdate` paints items 5 (@0x9f481-0x9f496) and 4
        // (@0x9f57c-0x9f591) black with no selection.
        let screen = screen_of(MissionComputer {
            rows: Vec::new(),
            ..computer()
        });
        let commands = drawn(&screen);
        let dialog = screen.dialog().expect("laid out");
        for item in [TEXT_ITEM, NAME_ITEM] {
            let area = dialog.item_bounds(item).expect("an item");
            assert_eq!(fill_and_text(&commands, area).1, None, "item {item}");
        }
    }

    #[test]
    fn the_heading_and_date_have_no_box() {
        let screen = screen();
        let dialog = screen.dialog().expect("laid out");
        let commands = drawn(&screen);
        for item in [HEADING_ITEM, DATE_ITEM] {
            let area = dialog.item_bounds(item).expect("an item");
            assert!(
                !commands.contains(&fill_command(area, Color::BLACK)),
                "item {item}"
            );
        }
    }

    #[test]
    fn without_its_dialog_it_says_why_and_escape_leaves() {
        let mut screen = MissionComputerScreen::new(
            Err("no DLOG 1006".to_owned()),
            computer(),
            ButtonStyle::STOCK,
        );
        assert_eq!(screen.problem(), Some("no DLOG 1006"));
        assert!(screen.rows().is_none());
        assert!(
            texts(&screen)
                .iter()
                .any(|(text, color)| text.contains("no DLOG 1006") && *color == Color::ERROR)
        );
        screen.set_computer(computer());
        screen.input(&key(Key::Escape));
        assert!(screen.closed());
    }

    /// `_TrackMissionButtons` @0x287f5 plays nothing under the pointer.
    #[test]
    fn pointer_click_is_silent_as_the_original_and_a_click_abandoned_does_nothing() {
        let mut screen = screen();
        click_item(&mut screen, ACCEPT_ITEM);
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(screen.take_accept(), Some(MissionId(130)));
        let at = screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(ACCEPT_ITEM))
            .expect("Accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_accept(), None, "abandoned");
        assert_eq!(screen.take_sounds(), []);
    }

    // The mission info screen.

    /// Stock `DLOG` 1012, 471 x 155 and centred, with its seven user
    /// items as `DITL` 1012 has them, 6 parked outside it.
    fn info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 471.0, 155.0),
            placement: Placement::Center,
            items: items(&[
                (290.0, 125.0, 99.0, 25.0, true),
                (9.0, 24.0, 195.0, 84.0, true),
                (13.0, 1.0, 193.0, 12.0, true),
                (218.0, 26.0, 242.0, 91.0, false),
                (57.0, 125.0, 99.0, 25.0, true),
                (177.0, 222.0, 55.0, 37.0, true),
                (343.0, 4.0, 122.0, 12.0, false),
            ]),
        }
    }

    /// Where the dialog goes: (1024 - 471) / 2 and (768 - 155) / 2,
    /// floored.
    const INFO_ORIGIN: Point = Point::new(276.0, 306.0);

    fn info_row(slot: u8, name: &str, failed: bool, text: &str, abortable: bool) -> InfoRow {
        InfoRow {
            slot,
            name: name.to_owned(),
            failed,
            text: text.to_owned(),
            abortable,
        }
    }

    /// Slot 0's run, abortable, and slot 3's stay, failed and not.
    fn info() -> MissionInfo {
        MissionInfo::shown(
            vec![
                info_row(0, "Run to Mars", false, "By June 30th.", true),
                info_row(3, "Stay", true, "Hold on.", false),
            ],
            "June 23rd, 1177 NC".to_owned(),
        )
    }

    fn info_screen() -> MissionInfoScreen {
        MissionInfoScreen::new(
            Ok((info_template(), Rc::new(MonoMetrics))),
            info(),
            ButtonStyle::STOCK,
        )
    }

    fn info_text(screen: &MissionInfoScreen, item: usize) -> String {
        screen
            .dialog()
            .expect("laid out")
            .text(item)
            .expect("a text item")
            .lines()
            .join(" ")
    }

    fn info_drawn(screen: &MissionInfoScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    /// Whether `label` draws enabled on `screen`.
    fn info_enabled(screen: &MissionInfoScreen, label: &str) -> bool {
        info_drawn(screen)
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == label => Some(color),
                _ => None,
            })
            .is_some_and(|color| color != ButtonStyle::STOCK.grey)
    }

    fn click_info(screen: &mut MissionInfoScreen, number: usize) {
        let at = screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn the_info_screens_named_values() {
        assert_eq!(INFO_DIALOG, 1012);
        assert_eq!(INFO_BACKGROUND, ImageKey::picture(8517));
        assert_eq!(
            [
                INFO_DONE_ITEM,
                INFO_LIST_ITEM,
                INFO_HEADING_ITEM,
                INFO_TEXT_ITEM,
                ABORT_ITEM,
                INFO_DATE_ITEM
            ],
            [1, 2, 3, 4, 5, 7]
        );
    }

    #[test]
    fn mission_info_shows_the_heading_the_date_the_missions_and_the_first_ones_text() {
        let screen = info_screen();
        assert_eq!(screen.problem(), None);
        assert_eq!(info_text(&screen, INFO_HEADING_ITEM), ACTIVE_MISSIONS);
        assert_eq!(info_text(&screen, INFO_DATE_ITEM), "June 23rd, 1177 NC");
        let rows = screen.rows().expect("laid out");
        assert_eq!(
            rows.rows(),
            ["Run to Mars".to_owned(), format!("{FAILED_MARK}Stay")]
        );
        assert_eq!(
            rows.row_bounds(0).map(|row| row.min),
            Some(Point::new(INFO_ORIGIN.x + 9.0, INFO_ORIGIN.y + 24.0))
        );
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "By June 30th.");
        assert!(info_enabled(&screen, ABORT_LABEL));
        assert!(info_enabled(&screen, "Done"));
        let commands = info_drawn(&screen);
        assert_eq!(
            commands[0],
            DrawCommand::Picture {
                image: INFO_BACKGROUND,
                top_left: INFO_ORIGIN,
            }
        );
        assert_eq!(screen.info(), &info());
    }

    #[test]
    fn selecting_a_mission_shows_its_text_and_greys_abort_unless_it_can_be_aborted() {
        let mut screen = info_screen();
        screen.input(&key(Key::ArrowDown));
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Hold on.");
        assert!(!info_enabled(&screen, ABORT_LABEL));
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), None, "greyed");
        screen.input(&key(Key::ArrowUp));
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), Some(0));
        assert_eq!(screen.take_abort(), None, "once");
        assert!(!screen.closed());
    }

    #[test]
    fn each_mission_shown_selected_asks_once_for_its_text_to_be_loaded() {
        let mut screen = info_screen();
        let first = screen.info().rows[0].slot;
        assert_eq!(screen.take_text_request(), Some(first), "as it opens");
        assert_eq!(screen.take_text_request(), None, "once");
        screen.show_text(first, "Loaded brief.");
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Loaded brief.");
        screen.show_text(first + 1, "Another's.");
        assert_eq!(
            info_text(&screen, INFO_TEXT_ITEM),
            "Loaded brief.",
            "not selected"
        );
    }

    #[test]
    fn the_quickbrief_draws_over_a_black_box_filling_its_item_selected_or_not() {
        // `_MissionInfoDialogUpdate` paints item 4 black with no selection
        // (@0x987d1-0x987e6) and inverts it under the text with one.
        let screen = info_screen();
        let area = screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(INFO_TEXT_ITEM))
            .expect("item 4");
        let commands = info_drawn(&screen);
        let (fill, text) = fill_and_text(&commands, area);
        assert!(fill < text.expect("the QuickBrief"), "under the text");
        let empty = MissionInfoScreen::new(
            Ok((info_template(), Rc::new(MonoMetrics))),
            MissionInfo {
                rows: Vec::new(),
                ..info()
            },
            ButtonStyle::STOCK,
        );
        assert_eq!(fill_and_text(&info_drawn(&empty), area).1, None);
        for item in [INFO_HEADING_ITEM, INFO_DATE_ITEM] {
            let area = screen
                .dialog()
                .and_then(|dialog| dialog.item_bounds(item))
                .expect("an item");
            assert!(
                !commands.contains(&fill_command(area, Color::BLACK)),
                "item {item}"
            );
        }
    }

    #[test]
    fn up_and_down_move_the_missions_and_never_scroll_the_black_boxed_quickbrief() {
        // Up and Down step the list, wrapping (`_MissionInfoFilter` @0x9d833,
        // @0x9da56), so nothing scrolls the QuickBrief; each mission's shows
        // from its first line.
        let mut screen = MissionInfoScreen::new(
            Ok((info_template(), Rc::new(MonoMetrics))),
            MissionInfo {
                rows: vec![
                    info_row(0, "Run to Mars", false, &long_text("a"), true),
                    info_row(3, "Stay", true, &long_text("b"), false),
                ],
                ..info()
            },
            ButtonStyle::STOCK,
        );
        let text = |screen: &MissionInfoScreen| {
            screen
                .dialog()
                .expect("laid out")
                .text(INFO_TEXT_ITEM)
                .expect("the QuickBrief")
                .clone()
        };
        shows_whole_lines_from_the_top(&info_drawn(&screen), &text(&screen));
        for (input, slot) in [
            (key(Key::ArrowDown), 3),
            (
                Input::Key {
                    key: Key::ArrowDown,
                    pressed: true,
                    repeat: true,
                },
                0,
            ),
            (key(Key::ArrowUp), 3),
            (key(Key::ArrowUp), 0),
        ] {
            screen.input(&input);
            assert_eq!(screen.selected_row().map(|row| row.slot), Some(slot));
            let shown = text(&screen);
            assert!(shown.lines()[0].starts_with(if slot == 0 { "a0" } else { "b0" }));
            shows_whole_lines_from_the_top(&info_drawn(&screen), &shown);
        }
    }

    /// Three missions: slot 0's and 5's abortable, 3's not.
    fn three_info() -> MissionInfo {
        let mut three = info();
        three
            .rows
            .push(info_row(5, "Fly", false, "Take wing.", true));
        three
    }

    fn info_screen_of(info: MissionInfo) -> MissionInfoScreen {
        MissionInfoScreen::new(
            Ok((info_template(), Rc::new(MonoMetrics))),
            info,
            ButtonStyle::STOCK,
        )
    }

    /// Each selected slot after each of `inputs`, each asked for once and
    /// with Abort regreyed for it.
    fn info_stepped(screen: &mut MissionInfoScreen, inputs: &[Input]) -> Vec<Option<u8>> {
        screen.take_text_request();
        inputs
            .iter()
            .map(|input| {
                screen.input(input);
                let selected = screen.selected_row().cloned();
                let slot = selected.as_ref().map(|row| row.slot);
                assert_eq!(screen.take_text_request(), slot, "asked for");
                assert_eq!(screen.take_text_request(), None, "once");
                assert_eq!(
                    info_enabled(screen, ABORT_LABEL),
                    selected.is_some_and(|row| row.abortable),
                    "Abort, on {slot:?}"
                );
                slot
            })
            .collect()
    }

    const BOTH_RINGS: [FocusRing; 2] = [FocusRing::Tab, FocusRing::Off];

    #[test]
    fn up_and_down_wrap_at_the_ends_of_the_missions() {
        for ring in BOTH_RINGS {
            let mut screen = info_screen_of(three_info()).with_focus_ring(ring);
            assert_eq!(
                info_stepped(
                    &mut screen,
                    &[key(Key::ArrowUp), key(Key::ArrowDown), key(Key::ArrowDown)]
                ),
                [Some(5), Some(0), Some(3)],
                "@0x9da56, @0x9d833 ({ring:?})"
            );
            let repeat = |key| Input::Key {
                key,
                pressed: true,
                repeat: true,
            };
            assert_eq!(
                info_stepped(
                    &mut screen,
                    &[
                        repeat(Key::ArrowDown),
                        repeat(Key::ArrowDown),
                        repeat(Key::ArrowUp)
                    ]
                ),
                [Some(5), Some(0), Some(5)],
                "repeats step and wrap too ({ring:?})"
            );
            for release in [Key::ArrowUp, Key::ArrowDown] {
                screen.input(&Input::Key {
                    key: release,
                    pressed: false,
                    repeat: false,
                });
            }
            assert_eq!(screen.rows().and_then(RowList::selected), Some(2));
            assert_eq!(screen.take_text_request(), None, "a release steps nothing");
        }
    }

    #[test]
    fn a_long_list_of_missions_follows_the_wraps_and_the_steps() {
        for ring in BOTH_RINGS {
            let mut long = info();
            long.rows = (0..12)
                .map(|slot| info_row(slot, "Run", false, "Go.", true))
                .collect();
            let mut screen = info_screen_of(long).with_focus_ring(ring);
            let visible = screen.rows().expect("laid out").visible();
            assert!(visible < 12, "more than fit");
            let first = |screen: &MissionInfoScreen| screen.rows().map(RowList::first);
            screen.input(&key(Key::ArrowUp));
            assert_eq!(first(&screen), Some(12 - visible), "the last row shown");
            screen.input(&key(Key::ArrowDown));
            assert_eq!(first(&screen), Some(0), "the first row shown");
            for _ in 0..visible {
                screen.input(&key(Key::ArrowDown));
            }
            assert_eq!(first(&screen), Some(1), "a row at a time");
        }
    }

    #[test]
    fn one_mission_steps_nothing_and_asks_nothing() {
        for ring in BOTH_RINGS {
            let mut screen = info_screen_of(MissionInfo {
                rows: vec![info_row(3, "Stay", false, "Hold on.", true)],
                ..info()
            })
            .with_focus_ring(ring);
            screen.take_text_request();
            screen.show_text(3, "Loaded.");
            for k in [Key::ArrowUp, Key::ArrowDown] {
                screen.input(&key(k));
                assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
                assert_eq!(screen.take_text_request(), None, "{k:?} ({ring:?})");
                assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Loaded.");
            }
        }
        let mut screen = info_screen_of(MissionInfo {
            rows: vec![info_row(3, "Stay", false, "Hold on.", true)],
            ..info()
        })
        .with_focus_ring(FocusRing::Off);
        screen.take_text_request();
        screen.input(&key(Key::Tab));
        assert_eq!(screen.take_text_request(), None, "Tab");
    }

    #[test]
    fn up_down_and_tab_with_no_missions_step_nothing() {
        let mut screen = info_screen_of(MissionInfo {
            rows: Vec::new(),
            ..info()
        })
        .with_focus_ring(FocusRing::Off);
        assert_eq!(
            info_stepped(
                &mut screen,
                &[key(Key::Tab), key(Key::ArrowUp), key(Key::ArrowDown)]
            ),
            [None, None, None]
        );
    }

    #[test]
    fn tab_steps_the_missions_and_wraps_under_the_engine_reading() {
        let mut screen = info_screen_of(three_info()).with_focus_ring(FocusRing::Off);
        assert_eq!(
            info_stepped(&mut screen, &[key(Key::Tab); 3]),
            [Some(3), Some(5), Some(0)],
            "@0x9d778"
        );
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "By June 30th.");
        assert_eq!(screen.rows().map(RowList::first), Some(0));
        assert_eq!(screen.dialog().and_then(Dialog::focus), None, "no ring");
        assert_eq!(screen.take_abort(), None);
        assert!(!screen.closed());
    }

    #[test]
    fn a_tab_shows_the_row_it_steps_to() {
        let mut long = info();
        long.rows = (0..12)
            .map(|slot| info_row(slot, "Run", false, "Go.", true))
            .collect();
        let mut screen = info_screen_of(long).with_focus_ring(FocusRing::Off);
        let visible = screen.rows().expect("laid out").visible();
        for _ in 0..visible {
            screen.input(&key(Key::Tab));
        }
        assert_eq!(screen.rows().map(RowList::first), Some(1));
    }

    /// `_MissionInfoFilter`'s Tab branch tests no Shift (@0x9d778).
    #[test]
    fn shift_tab_also_steps_forward_on_the_info_screen() {
        let mut screen = info_screen_of(three_info()).with_focus_ring(FocusRing::Off);
        screen.input(&key(Key::ShiftLeft));
        screen.input(&key(Key::Tab));
        assert_eq!(screen.selected_row().map(|row| row.slot), Some(3));
    }

    /// The original steps on autoKey too (@0x9d778), but dialog keys are
    /// press-only by the user's decision of 2026-10-23.
    #[test]
    fn a_tab_repeat_or_release_steps_no_mission() {
        let mut screen = info_screen().with_focus_ring(FocusRing::Off);
        screen.take_text_request();
        for (pressed, repeat) in [(true, true), (false, false)] {
            screen.input(&Input::Key {
                key: Key::Tab,
                pressed,
                repeat,
            });
        }
        assert_eq!(screen.selected_row().map(|row| row.slot), Some(0));
        assert_eq!(screen.take_text_request(), None);
    }

    #[test]
    fn under_the_focus_ring_tab_moves_the_button_focus_not_the_mission() {
        let mut screen = info_screen_of(three_info());
        screen.take_text_request();
        screen.input(&key(Key::Tab));
        assert_eq!(screen.selected_row().map(|row| row.slot), Some(0));
        assert_eq!(screen.take_text_request(), None);
        assert_eq!(
            screen.dialog().and_then(Dialog::focus),
            Some(INFO_DONE_ITEM)
        );
        screen.input(&key(Key::Tab));
        assert_eq!(screen.dialog().and_then(Dialog::focus), Some(ABORT_ITEM));
        screen.input(&key(Key::Space));
        assert_eq!(screen.take_abort(), Some(0), "the focused button");
        assert!(!screen.closed());
    }

    #[test]
    fn done_return_and_escape_close_mission_info() {
        for close in [Key::Enter, Key::NumpadEnter, Key::Escape] {
            let mut screen = info_screen();
            screen.input(&key(close));
            assert!(screen.closed(), "{close:?}");
            assert_eq!(screen.take_abort(), None);
        }
        let mut screen = info_screen();
        click_info(&mut screen, INFO_DONE_ITEM);
        assert!(screen.closed());
    }

    #[test]
    fn new_info_keeps_the_selected_slot_and_closes_with_nothing_left() {
        let mut screen = info_screen();
        screen.input(&key(Key::ArrowDown));
        screen.set_info(MissionInfo {
            rows: vec![info_row(3, "Stay", true, "Hold on.", false)],
            ..info()
        });
        assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Hold on.");
        assert!(!screen.closed());
        screen.set_info(MissionInfo {
            rows: vec![
                info_row(1, "New", false, "Fresh.", true),
                info_row(3, "Stay", true, "Hold on.", false),
            ],
            ..info()
        });
        assert_eq!(
            screen.rows().and_then(RowList::selected),
            Some(1),
            "slot 3 kept"
        );
        screen.set_info(MissionInfo {
            rows: Vec::new(),
            ..info()
        });
        assert!(screen.closed());
    }

    /// The three missions with slot 0, selected, aborted: Abort asked and
    /// the info handed back without it.
    fn after_an_abort() -> MissionInfoScreen {
        let mut screen = info_screen_of(three_info());
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), Some(0));
        let mut left = three_info();
        left.rows.remove(0);
        screen.take_text_request();
        screen.set_info(left);
        screen
    }

    /// `_DoMissionInfoDialog` rebuilds the list with no selection
    /// (@0x9e62d-0x9e632, @0x9e66a-0x9e680).
    #[test]
    fn an_abort_with_missions_left_leaves_nothing_selected() {
        let mut screen = after_an_abort();
        assert_eq!(screen.rows().and_then(RowList::selected), None);
        assert!(!screen.closed());
        let area = screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(INFO_TEXT_ITEM))
            .expect("item 4");
        assert_eq!(
            fill_and_text(&info_drawn(&screen), area).1,
            None,
            "an empty black box (@0x987d1-0x987e6)"
        );
        assert_eq!(screen.take_text_request(), None, "no text to load");
        assert!(!info_enabled(&screen, ABORT_LABEL), "@0x283c7-0x283e4");
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), None, "@0x285a3-0x285b1");
        screen.input(&key(crate::flight::view::MAP_KEY));
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::MissionInfo(None)),
            "@0x9e3e5-0x9e3f1"
        );
    }

    #[test]
    fn after_an_abort_down_selects_the_first_mission_and_up_the_last() {
        let mut screen = after_an_abort();
        assert_eq!(
            info_stepped(&mut screen, &[key(Key::ArrowDown)]),
            [Some(3)],
            "@0x9d839"
        );
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Hold on.");
        let mut screen = after_an_abort();
        assert_eq!(
            info_stepped(&mut screen, &[key(Key::ArrowUp)]),
            [Some(5)],
            "@0x9da60"
        );
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Take wing.");
    }

    /// A refused Abort does nothing (@0x9e583).
    #[test]
    fn a_refused_abort_keeps_the_selected_slot() {
        let mut screen = info_screen_of(three_info());
        screen.input(&key(Key::ArrowDown));
        screen.set_info(three_info());
        assert_eq!(screen.selected_row().map(|row| row.slot), Some(3));
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Hold on.");
    }

    #[test]
    fn without_its_dialog_mission_info_says_why_and_escape_closes_it() {
        let mut screen =
            MissionInfoScreen::new(Err("no DLOG 1012".to_owned()), info(), ButtonStyle::STOCK);
        assert_eq!(screen.problem(), Some("no DLOG 1012"));
        assert!(screen.rows().is_none());
        assert!(info_drawn(&screen).iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, .. } if text.contains("no DLOG 1012") && *color == Color::ERROR
        )));
        screen.input(&key(Key::Escape));
        assert!(screen.closed());
    }

    /// `_TrackMissionInfoButtons` @0x2850e plays nothing under the pointer.
    #[test]
    fn mission_info_beeps_as_it_opens_and_as_it_closes() {
        const OPENED: Sound = Sound::Ui(crate::sound::UiSound::MissionInfoOpened);
        const CLOSED: Sound = Sound::Ui(crate::sound::UiSound::MissionInfoClosed);
        let mut screen = info_screen();
        assert_eq!(screen.take_sounds(), [OPENED]);
        click_info(&mut screen, INFO_DONE_ITEM);
        assert_eq!(screen.take_sounds(), [CLOSED]);
        for close in [Key::Enter, Key::NumpadEnter, Key::Escape] {
            let mut screen = info_screen();
            screen.take_sounds();
            screen.input(&key(close));
            assert_eq!(screen.take_sounds(), [CLOSED], "{close:?}");
            screen.input(&key(close));
            assert_eq!(screen.take_sounds(), [], "{close:?} once closed");
        }
        let mut screen = info_screen();
        screen.take_sounds();
        screen.set_info(info());
        assert_eq!(screen.take_sounds(), [], "still listing");
        screen.set_info(MissionInfo {
            rows: Vec::new(),
            ..info()
        });
        assert_eq!(screen.take_sounds(), [CLOSED], "emptied");
        let mut broken =
            MissionInfoScreen::new(Err("no DLOG 1012".to_owned()), info(), ButtonStyle::STOCK);
        broken.input(&key(Key::Escape));
        assert_eq!(broken.take_sounds(), [OPENED], "without its dialog");
    }

    #[test]
    fn mission_infos_pointer_click_is_silent_and_a_click_abandoned_does_nothing() {
        let mut screen = info_screen();
        let at = screen
            .dialog()
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.take_abort(), None, "abandoned");
        assert_eq!(screen.take_sounds(), [OPENED], "its opening's alone");
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(screen.take_abort(), Some(0));
    }

    const OPENED: Sound = Sound::Ui(crate::sound::UiSound::MissionInfoOpened);
    const CLOSED: Sound = Sound::Ui(crate::sound::UiSound::MissionInfoClosed);

    #[test]
    fn mission_info_beeps_as_it_opens_and_as_it_closes_laid_out() {
        let mut screen = info_screen();
        assert_eq!(screen.take_sounds(), [OPENED], "_beepSnd[1] @0x9e1d5");
        click_info(&mut screen, INFO_DONE_ITEM);
        assert_eq!(screen.take_sounds(), [CLOSED], "_beepSnd[2] @0x9e791");
        screen.set_info(MissionInfo {
            rows: Vec::new(),
            ..info()
        });
        assert_eq!(screen.take_sounds(), [], "closed once");
        for close in [Key::Enter, Key::NumpadEnter, Key::Escape] {
            let mut screen = info_screen();
            screen.take_sounds();
            screen.input(&key(close));
            assert_eq!(screen.take_sounds().last(), Some(&CLOSED), "{close:?}");
        }
        let mut screen = info_screen();
        screen.take_sounds();
        screen.set_info(MissionInfo {
            rows: Vec::new(),
            ..info()
        });
        assert_eq!(screen.take_sounds(), [CLOSED], "an abort leaving nothing");
        let mut bare =
            MissionInfoScreen::new(Err("no DLOG 1012".to_owned()), info(), ButtonStyle::STOCK);
        assert_eq!(bare.take_sounds(), [OPENED], "before GetNewDialog");
        bare.input(&key(Key::Escape));
        assert!(bare.closed());
        assert_eq!(bare.take_sounds(), [], "@0x9e20b skips the close's");
    }

    #[test]
    fn the_mission_info_key_in_the_computer_asks_for_mission_info() {
        let mut screen = screen();
        assert!(!screen.take_info_request());
        screen.input(&key(MISSION_INFO_KEY));
        assert!(screen.take_info_request());
        assert!(!screen.take_info_request(), "once");
        assert!(!screen.closed());
        assert_eq!(screen.take_accept(), None);
    }

    /// A press, a repeat and a release of [`MAP_KEY`](crate::flight::view::MAP_KEY), in that order.
    fn map_keys() -> [Input; 3] {
        let m = crate::flight::view::MAP_KEY;
        [
            Input::Key {
                key: m,
                pressed: true,
                repeat: true,
            },
            Input::Key {
                key: m,
                pressed: false,
                repeat: false,
            },
            key(m),
        ]
    }

    #[test]
    fn the_map_key_in_the_computer_asks_for_the_map_with_its_selected_offer() {
        let mut screen = screen();
        let [repeat, release, press] = map_keys();
        for input in [repeat, release] {
            screen.input(&input);
            assert_eq!(screen.take_map_request(), None, "{input:?}");
        }
        screen.input(&press);
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::Computer(Some(MissionId(130))))
        );
        assert_eq!(screen.take_map_request(), None, "once");
        screen.input(&key(Key::ArrowDown));
        screen.input(&press);
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::Computer(Some(MissionId(131))))
        );
        assert!(!screen.closed());
        assert!(!screen.take_info_request());
        assert_eq!(screen.take_accept(), None);
        assert_eq!(screen.take_sounds(), [], "silent");
        let mut empty = screen_of(MissionComputer {
            rows: Vec::new(),
            today: String::new(),
        });
        empty.input(&press);
        assert_eq!(
            empty.take_map_request(),
            Some(MapOpener::Computer(None)),
            "no row selected"
        );
    }

    #[test]
    fn the_map_key_in_mission_info_asks_for_the_map_with_its_selected_slot_unless_refused() {
        let mut screen = info_screen();
        screen.take_sounds();
        let [repeat, release, press] = map_keys();
        for input in [repeat, release] {
            screen.input(&input);
            assert_eq!(screen.take_map_request(), None, "{input:?}");
        }
        screen.input(&press);
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::MissionInfo(Some(0)))
        );
        assert_eq!(screen.take_map_request(), None, "once");
        screen.input(&key(Key::ArrowDown));
        screen.input(&press);
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::MissionInfo(Some(3)))
        );
        assert!(!screen.closed());
        assert_eq!(screen.take_sounds(), [], "silent");
        let mut refused = info_screen().with_map_allowed(false);
        refused.input(&press);
        assert_eq!(
            refused.take_map_request(),
            None,
            "opened over an offer or the computer (@0x9e3c6-0x9e3cd)"
        );
        assert!(!refused.closed());
    }

    #[test]
    fn their_debug_names_what_they_hold() {
        for debug in [format!("{:?}", screen()), format!("{:?}", info_screen())] {
            for part in ["Dialog", "RowList", "TextMetrics"] {
                assert!(debug.contains(part), "{part}: {debug}");
            }
        }
    }

    fn press_and_release(key: Key) -> [Input; 2] {
        [true, false].map(|pressed| Input::Key {
            key,
            pressed,
            repeat: false,
        })
    }

    /// The original's `_MissionFilter` (@0x9f7a9) maps Tab, the arrows,
    /// Return and Escape to items without a click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original_on_the_computer() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            for k in [
                Key::Tab,
                Key::ArrowUp,
                Key::ArrowDown,
                MISSION_INFO_KEY,
                crate::ui::player_info::PLAYER_INFO_KEY,
                ACCEPT_KEY,
            ]
            .into_iter()
            .chain(LEAVE_KEYS)
            {
                let mut screen = screen().with_focus_ring(ring);
                for input in press_and_release(k) {
                    screen.input(&input);
                }
                assert_eq!(screen.take_sounds(), [], "{k:?} ({ring:?})");
            }
        }
    }

    /// The original's `_MissionInfoFilter` (@0x9d6e5) maps Return, Escape
    /// and the mission info key to Done without a click sound: only the
    /// close's beep plays. Its steps play nothing.
    #[test]
    fn keyboard_activation_is_silent_as_the_original_on_the_info_screen() {
        for ring in BOTH_RINGS {
            for k in [Key::Enter, Key::Escape, MISSION_INFO_KEY] {
                let mut screen = info_screen().with_focus_ring(ring);
                screen.take_sounds();
                for input in press_and_release(k) {
                    screen.input(&input);
                }
                assert_eq!(screen.take_sounds(), [CLOSED], "{k:?} ({ring:?})");
            }
            for k in [Key::Tab, Key::ArrowUp, Key::ArrowDown] {
                let mut screen = info_screen().with_focus_ring(ring);
                screen.take_sounds();
                for input in press_and_release(k) {
                    screen.input(&input);
                }
                assert_eq!(screen.take_sounds(), [], "{k:?} ({ring:?})");
            }
        }
    }

    /// `_MissionInfoFilter` gives Done for the key that opens the screen
    /// (@0x9d756-0x9d769).
    #[test]
    fn the_mission_info_key_closes_mission_info() {
        for ring in BOTH_RINGS {
            for screen in [info_screen(), info_screen().with_map_allowed(false)] {
                let mut screen = screen.with_focus_ring(ring);
                screen.take_sounds();
                screen.input(&key(MISSION_INFO_KEY));
                assert!(screen.closed(), "{ring:?}");
                assert_eq!(screen.take_sounds(), [CLOSED], "once, {ring:?}");
                assert_eq!(screen.take_abort(), None, "{ring:?}");
                assert_eq!(screen.take_map_request(), None, "{ring:?}");
            }
        }
    }

    /// Dialog keys are press-only by the user's decision of 2026-10-23.
    #[test]
    fn a_repeat_or_release_of_the_mission_info_key_does_not_close_mission_info() {
        let mut screen = info_screen();
        for (pressed, repeat) in [(true, true), (false, false)] {
            screen.input(&Input::Key {
                key: MISSION_INFO_KEY,
                pressed,
                repeat,
            });
        }
        assert!(!screen.closed());
    }

    /// Without its dialog the screen's message offers Return and Escape
    /// alone, and they alone close it.
    #[test]
    fn without_its_dialog_the_mission_info_key_does_not_close_mission_info() {
        let mut screen =
            MissionInfoScreen::new(Err("no DLOG 1012".to_owned()), info(), ButtonStyle::STOCK);
        screen.input(&key(MISSION_INFO_KEY));
        assert!(!screen.closed());
    }

    /// The computer's and the info screen's buttons take Tab then Space
    /// only with the focus ring; by the original's reading they press
    /// nothing.
    #[test]
    fn tab_then_space_presses_the_focused_button_only_with_the_focus_ring() {
        let mut ringed = screen();
        ringed.input(&key(Key::Tab));
        assert_eq!(ringed.dialog().and_then(Dialog::focus), Some(ACCEPT_ITEM));
        ringed.input(&key(Key::Space));
        assert!(ringed.take_accept().is_some(), "Accept, by default");
        let mut off = screen().with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert!(!off.closed(), "the computer");
        assert_eq!(off.take_accept(), None);
        assert_eq!(off.dialog().and_then(Dialog::focus), None);
        let mut ringed = info_screen();
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(ringed.closed(), "the info screen, by default");
        let mut off = info_screen().with_focus_ring(FocusRing::Off);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert!(!off.closed(), "the info screen");
        assert_eq!(off.take_abort(), None);
        assert_eq!(off.dialog().and_then(Dialog::focus), None);
    }

    // The table an abort leaves by the engine reading of
    // `mission_info_rebuild`.

    /// Rows for slots 1 and 2 over a table of 0 (hidden, not abortable),
    /// 1 and 2: what the engine's rebuild leaves after an abort with a
    /// hidden mission in slot 0.
    fn tabled() -> MissionInfo {
        MissionInfo::shown(
            vec![
                info_row(1, "Run", false, "Go.", true),
                info_row(2, "Walk", false, "Stroll.", true),
            ],
            "June 23rd, 1177 NC",
        )
        .with_table(vec![
            info_row(0, "Hidden", false, "Shh.", false),
            info_row(1, "Run", false, "Go.", true),
            info_row(2, "Walk", false, "Stroll.", true),
        ])
    }

    #[test]
    fn with_a_table_a_row_selects_the_slot_at_its_position() {
        let mut screen = info_screen_of(tabled());
        assert_eq!(
            screen.rows().map(RowList::rows),
            Some(&["Run".to_owned(), "Walk".to_owned()][..])
        );
        assert_eq!(screen.take_text_request(), Some(0), "row 0 picks slot 0");
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Shh.");
        assert!(!info_enabled(&screen, ABORT_LABEL), "slot 0's grey");
        screen.input(&key(Key::KeyM));
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::MissionInfo(Some(0)))
        );
        screen.input(&key(Key::ArrowDown));
        assert_eq!(
            screen.take_text_request(),
            Some(1),
            "row 1, named Walk, picks slot 1"
        );
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), Some(1));
    }

    #[test]
    fn with_a_table_a_step_can_select_past_the_rows() {
        let mut screen = info_screen_of(tabled());
        let _ = screen.take_text_request();
        screen.input(&key(Key::ArrowDown));
        screen.input(&key(Key::ArrowDown));
        assert_eq!(
            screen.rows().and_then(RowList::selected),
            Some(2),
            "past the rows"
        );
        assert_eq!(screen.take_text_request(), Some(2));
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Stroll.");
        let highlights = info_drawn(&screen)
            .into_iter()
            .filter(|command| {
                *command
                    == fill_command(
                        Bounds::at(Point::new(285.0, 354.0), 195.0, 12.0),
                        crate::ui::row_list::SELECTED,
                    )
            })
            .count();
        assert_eq!(highlights, 0, "no row highlighted");
        screen.input(&key(Key::ArrowDown));
        assert_eq!(
            screen.rows().and_then(RowList::selected),
            Some(0),
            "wrapped over three"
        );
    }

    #[test]
    fn with_a_table_and_no_rows_the_screen_stays_open() {
        let mut screen = info_screen();
        screen.set_info(
            MissionInfo::shown(Vec::new(), "June 23rd, 1177 NC")
                .with_table(vec![info_row(0, "Hidden", false, "Shh.", false)]),
        );
        assert!(
            !screen.closed(),
            "a hidden mission is still active (@0x9e5f9-0x9e60a)"
        );
        assert_eq!(screen.rows().map(|rows| rows.rows().len()), Some(0));
        screen.set_info(MissionInfo::shown(Vec::new(), "").with_table(Vec::new()));
        assert!(screen.closed(), "nothing active");
    }

    // The lists' scroll bars and pressed tracking.

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    /// `n` Mac ticks.
    fn mac_ticks(n: u32) -> Duration {
        Duration::from_secs(1) / 60 * n
    }

    /// `count` missions in slots 0 on, the even ones abortable.
    fn many_info(count: u8) -> MissionInfo {
        MissionInfo::shown(
            (0..count)
                .map(|slot| {
                    info_row(
                        slot,
                        &format!("Run {slot}"),
                        false,
                        &format!("Brief {slot}."),
                        slot % 2 == 0,
                    )
                })
                .collect(),
            "June 23rd, 1177 NC".to_owned(),
        )
    }

    /// The info screen's item 2 on screen: (285, 330) to (480, 414), seven
    /// rows of 12.
    const INFO_LIST_AT: Point = Point::new(285.0, 330.0);

    /// A point on the info screen's row `row`, as shown from the top.
    fn info_row_at(row: f32) -> Point {
        Point::new(INFO_LIST_AT.x + 50.0, INFO_LIST_AT.y + 12.0 * row + 6.0)
    }

    /// The info screen's bar's down arrow, with rows that don't fit.
    const INFO_DOWN_ARROW: Point = Point::new(470.0, 410.0);

    fn first_shown(screen: &MissionInfoScreen) -> Option<usize> {
        screen.rows().map(RowList::first)
    }

    #[test]
    fn a_long_list_shows_its_scroll_bar_and_a_press_on_it_scrolls() {
        let mut screen = info_screen_of(many_info(10));
        let _ = screen.take_text_request();
        assert_eq!(
            info_screen_of(many_info(7)).rows().and_then(RowList::bar),
            None
        );
        assert_eq!(
            screen.rows().and_then(RowList::bar),
            Some(Bounds {
                min: Point::new(465.0, 329.0),
                max: Point::new(481.0, 415.0),
            }),
            "item 2 less 15, then 16 wide (@0x9df84-0x9df90, GetScSize)"
        );
        screen.input(&button(true, INFO_DOWN_ARROW));
        assert_eq!(first_shown(&screen), Some(1));
        screen.input(&button(false, INFO_DOWN_ARROW));
        assert_eq!(screen.take_text_request(), None, "the selection stays");
        assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
    }

    #[test]
    fn a_press_on_the_list_tracks_across_rows_to_its_release() {
        let mut screen = info_screen_of(many_info(3));
        let _ = screen.take_text_request();
        screen.input(&button(true, info_row_at(0.0)));
        screen.input(&Input::PointerMoved(info_row_at(2.0)));
        assert_eq!(
            screen.rows().and_then(RowList::selected),
            Some(2),
            "highlighted"
        );
        assert_eq!(screen.take_text_request(), None, "not loaded while held");
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Brief 0.");
        assert!(info_enabled(&screen, ABORT_LABEL), "still row 0's Abort");
        screen.input(&button(false, info_row_at(2.0)));
        assert_eq!(screen.rows().and_then(RowList::selected), Some(2));
        assert_eq!(
            screen.take_text_request(),
            Some(2),
            "loaded at LClick's return (@0x9dcd8)"
        );
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Brief 2.");
        assert!(info_enabled(&screen, ABORT_LABEL), "row 2's");
        screen.input(&button(true, info_row_at(1.0)));
        screen.input(&button(false, info_row_at(1.0)));
        assert!(
            !info_enabled(&screen, ABORT_LABEL),
            "row 1 can't be aborted"
        );
    }

    #[test]
    fn a_click_below_the_last_row_selects_nothing() {
        let mut screen = info_screen_of(many_info(2));
        let _ = screen.take_text_request();
        screen.input(&button(true, info_row_at(4.0)));
        screen.input(&button(false, info_row_at(4.0)));
        assert_eq!(screen.rows().and_then(RowList::selected), None);
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "", "item 4 black");
        assert!(!info_enabled(&screen, ABORT_LABEL), "Abort greyed");
        assert_eq!(screen.take_text_request(), None);
        screen.input(&key(Key::KeyM));
        assert_eq!(
            screen.take_map_request(),
            Some(MapOpener::MissionInfo(None)),
            "marked for nothing"
        );
    }

    #[test]
    fn a_step_with_the_selection_scrolled_out_of_view_lands_where_lautoscroll_puts_it() {
        let mut screen = info_screen_of(many_info(10));
        for _ in 0..3 {
            screen.input(&button(true, INFO_DOWN_ARROW));
            screen.input(&button(false, INFO_DOWN_ARROW));
        }
        assert_eq!(first_shown(&screen), Some(3), "row 0 scrolled out above");
        let _ = screen.take_text_request();
        screen.input(&key(Key::ArrowDown));
        assert_eq!(screen.rows().and_then(RowList::selected), Some(1));
        assert_eq!(
            first_shown(&screen),
            Some(1),
            "no LRect scroll (@0x9d7e8), then LAutoScroll to the top (@0x9d8f1)"
        );
        assert_eq!(screen.take_text_request(), Some(1));
    }

    #[test]
    fn cancelling_a_drag_keeps_the_row_it_reached() {
        let mut screen = info_screen_of(many_info(3));
        let _ = screen.take_text_request();
        screen.input(&button(true, info_row_at(0.0)));
        screen.input(&Input::PointerMoved(info_row_at(1.0)));
        screen.cancel_pointer();
        assert_eq!(screen.rows().and_then(RowList::selected), Some(1));
        assert_eq!(
            screen.take_text_request(),
            Some(1),
            "read back as a release"
        );
        screen.input(&Input::PointerMoved(info_row_at(2.0)));
        assert_eq!(screen.rows().and_then(RowList::selected), Some(1), "let go");
    }

    #[test]
    fn ticks_hold_the_bars_arrow() {
        let mut screen = info_screen_of(many_info(10));
        screen.input(&button(true, INFO_DOWN_ARROW));
        assert_eq!(first_shown(&screen), Some(1));
        screen.tick(mac_ticks(11));
        assert_eq!(first_shown(&screen), Some(1));
        screen.tick(mac_ticks(1));
        assert_eq!(first_shown(&screen), Some(2), "ScrollDelay's 12 ticks");
    }

    /// `count` offers, 130 on.
    fn many_offers(count: i16) -> MissionComputer {
        MissionComputer {
            rows: (0..count)
                .map(|n| row(130 + n, &format!("Offer {n}"), &format!("Text {n}.")))
                .collect(),
            today: String::new(),
        }
    }

    /// The computer's item 2 on screen: (267, 313) to (462, 457), twelve
    /// rows of 12; item 3 is (462, 313) to (477, 457).
    fn computer_row_at(row: f32) -> Point {
        Point::new(300.0, 313.0 + 12.0 * row + 6.0)
    }

    #[test]
    fn the_computers_rows_cover_item_3_when_they_fit() {
        let screen = screen_of(many_offers(3));
        let rows = screen.rows().expect("laid out");
        assert_eq!(rows.bar(), None);
        assert_eq!(
            rows.row_bounds(0).map(|row| row.width()),
            Some(210.0),
            "item 2's 195 and item 3's 15 (@0xa07a8-0xa07e3)"
        );
    }

    #[test]
    fn the_computers_bar_sits_in_item_3_when_the_rows_dont_fit() {
        let mut screen = screen_of(many_offers(13));
        let _ = screen.take_text_request();
        let rows = screen.rows().expect("laid out");
        assert_eq!(rows.row_bounds(0).map(|row| row.width()), Some(195.0));
        assert_eq!(
            rows.bar(),
            Some(Bounds {
                min: Point::new(462.0, 312.0),
                max: Point::new(478.0, 458.0),
            })
        );
        let down = Point::new(470.0, 450.0);
        screen.input(&button(true, down));
        assert_eq!(
            screen.rows().map(RowList::first),
            Some(1),
            "item 3's LClick (@0xa01e7)"
        );
        screen.input(&button(false, down));
        assert_eq!(screen.take_text_request(), None);
    }

    #[test]
    fn ticks_hold_the_computers_bar_arrow_and_a_cancel_loads_the_row_reached() {
        let mut screen = screen_of(many_offers(20));
        let down = Point::new(470.0, 450.0);
        screen.input(&button(true, down));
        screen.tick(mac_ticks(12));
        assert_eq!(screen.rows().map(RowList::first), Some(2), "held");
        screen.input(&button(false, down));
        let mut dragged = screen_of(many_offers(3));
        let _ = dragged.take_text_request();
        dragged.input(&button(true, computer_row_at(0.0)));
        dragged.input(&Input::PointerMoved(computer_row_at(1.0)));
        dragged.cancel_pointer();
        assert_eq!(
            dragged.take_text_request(),
            Some(MissionId(131)),
            "as a release"
        );
        assert_eq!(item_text(&dragged, TEXT_ITEM), "Text 1.");
    }

    #[test]
    fn a_press_in_item_3_over_a_list_that_fits_is_left_out() {
        let mut screen = screen_of(many_offers(3));
        let _ = screen.take_text_request();
        let in_item_3 = Point::new(470.0, computer_row_at(1.0).y);
        assert!(
            screen
                .rows()
                .and_then(|rows| rows.row_bounds(1))
                .is_some_and(|row| row.contains(in_item_3)),
            "the widened row 1 covers it"
        );
        screen.input(&button(true, in_item_3));
        screen.input(&button(false, in_item_3));
        assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
        assert_eq!(screen.take_text_request(), None);
        assert_eq!(item_text(&screen, TEXT_ITEM), "Text 0.");
    }

    #[test]
    fn a_press_on_the_computers_list_tracks_to_its_release() {
        let mut screen = screen_of(many_offers(3));
        let _ = screen.take_text_request();
        screen.input(&button(true, computer_row_at(0.0)));
        screen.input(&Input::PointerMoved(computer_row_at(2.0)));
        assert_eq!(screen.take_text_request(), None, "not loaded while held");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Text 0.");
        screen.input(&button(false, computer_row_at(2.0)));
        assert_eq!(
            screen.take_text_request(),
            Some(MissionId(132)),
            "loaded at LClick's return (@0xa007c)"
        );
        assert_eq!(item_text(&screen, TEXT_ITEM), "Text 2.");
    }

    #[test]
    fn a_click_below_the_computers_last_row_selects_nothing() {
        let mut screen = screen_of(many_offers(2));
        let _ = screen.take_text_request();
        screen.input(&button(true, computer_row_at(5.0)));
        screen.input(&button(false, computer_row_at(5.0)));
        assert_eq!(screen.rows().and_then(RowList::selected), None);
        assert_eq!(item_text(&screen, TEXT_ITEM), "");
        assert_eq!(screen.take_text_request(), None);
        screen.input(&key(ACCEPT_KEY));
        assert_eq!(screen.take_accept(), None, "A accepts nothing");
        screen.input(&key(Key::KeyM));
        assert_eq!(screen.take_map_request(), Some(MapOpener::Computer(None)));
    }
}
