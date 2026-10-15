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
//! keys. The screen enables its buttons itself.
//!
//! What it shows is the session's [`MissionComputer`], decided in
//! `nova-sim`. Accept asks for the selected mission
//! ([`MissionComputerScreen::take_accept`]), and is greyed with nothing
//! listed; whoever holds the session accepts it and hands the computer
//! back ([`MissionComputerScreen::set_computer`]), as the original closes
//! the dialog after an accept and opens it again. Leave, Return, Enter,
//! Escape and L leave; no key accepts (`_MissionFilter` @0x9f885-0x9f8af).
//! Up and Down move the selection. Without its dialog, the screen says
//! why, and Return or Escape still leaves.
//!
//! The stock mission info screen is 471 x 155, centred, over `PICT` 8517:
//! the heading "Currently active missions:" (`STR#` 2002 #360) in item 3
//! and today's medium date in item 7; the active missions shown listed in
//! item 2, a failed one marked "• ", the first selected; the selected
//! one's `QuickBrief` in item 4, which shows its deadline through `<DL>`;
//! and Done (item 1, `STR#` 150 #5) and Abort (item 5, #35), greyed unless
//! the selected mission can be aborted (`_DrawMissionInfoButtons`
//! @0x28359). Item 6, the map, lies outside it. What it shows is the
//! session's [`MissionInfo`]; Abort asks for the selected slot
//! ([`MissionInfoScreen::take_abort`]), with no confirmation, and whoever
//! holds the session aborts it and hands the info back
//! ([`MissionInfoScreen::set_info`]), which closes the screen once
//! nothing is listed (@0x9e5eb-0x9e610). Done, Return, Enter and Escape
//! close it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::mission::screens::{
    ABORT_LABEL, ACCEPT_LABEL, ACTIVE_MISSIONS, AVAILABLE_HERE, FAILED_MARK,
};
use nova_sim::{InfoRow, MissionComputer, MissionId, MissionInfo};

use super::bar::MISSION_INFO_KEY;
use super::layout::LEAVE_LABEL;
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::desc::{BODY_COLOR, BODY_SIZE, DONE_LABEL};
use crate::ui::dialog::{Dialog, DialogEvent, DialogTemplate, Role};
use crate::ui::player_info::is_player_info_press;
use crate::ui::row_list::RowList;

/// The mission computer's `DLOG` (and `DITL`) ID.
pub const COMPUTER_DIALOG: i16 = 1006;
/// The picture the mission computer is drawn over.
pub const COMPUTER_BACKGROUND: ImageKey = ImageKey::picture(8505);

/// Accept's item.
pub const ACCEPT_ITEM: usize = 1;
/// The list's item.
pub const LIST_ITEM: usize = 2;
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

/// Where the list goes when the template has no item 2: nowhere.
const NO_LIST: Bounds = Bounds {
    min: Point::new(0.0, 0.0),
    max: Point::new(0.0, 0.0),
};

/// The keys that leave the mission computer, besides Leave: Return (and
/// Enter), Escape and L (`_MissionFilter` @0x9f885-0x9f8af, @0xa0212).
pub const LEAVE_KEYS: [Key; 3] = [Key::Enter, Key::Escape, Key::Char('l')];

/// The metrics, shared, with a `Debug` that shows nothing of them.
#[derive(Clone)]
struct MetricsHandle(Rc<dyn TextMetrics>);

impl std::fmt::Debug for MetricsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextMetrics")
    }
}

/// Whether `input` goes to the list in `dialog`'s item `list`: Up, Down,
/// or a press on it.
fn on_list(dialog: &Dialog, list: usize, input: &Input) -> bool {
    match *input {
        Input::Key {
            key: Key::Up | Key::Down,
            ..
        } => true,
        Input::PointerButton {
            pressed: true, at, ..
        } => dialog
            .item_bounds(list)
            .is_some_and(|bounds| bounds.contains(at)),
        _ => false,
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
    /// Whether [`MISSION_INFO_KEY`] has asked for the mission info screen
    /// since this was last taken.
    info_requested: bool,
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen.
    player_info_requested: bool,
    closed: bool,
    sounds: Vec<Sound>,
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
            let text = |text: &str| Role::ScrollText {
                text: text.to_owned(),
                font: Font::Geneva,
                size: BODY_SIZE,
                color: BODY_COLOR,
            };
            let roles = [
                (ACCEPT_ITEM, Role::Button(ACCEPT_LABEL.to_owned())),
                (LEAVE_ITEM, Role::Button(LEAVE_LABEL.to_owned())),
                (TEXT_ITEM, text("")),
                (NAME_ITEM, text("")),
                (HEADING_ITEM, text(AVAILABLE_HERE)),
                (DATE_ITEM, text("")),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(LEAVE_ITEM))
                .with_cancel(Some(LEAVE_ITEM));
            let list = dialog.item_bounds(LIST_ITEM).unwrap_or(NO_LIST);
            Laid {
                rows: RowList::new(Vec::new(), list, &metrics),
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
            closed: false,
            sounds: Vec::new(),
        };
        screen.set_computer(computer);
        screen
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

    /// Whether [`MISSION_INFO_KEY`] has asked for the mission info screen
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

    /// Shows `computer` in place of the one shown, keeping the selected
    /// mission selected while it is listed, and otherwise the first.
    pub fn set_computer(&mut self, computer: MissionComputer) {
        let kept = self.selected_mission();
        self.computer = computer;
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let names = self.computer.rows.iter().map(|row| row.name.clone());
        let rect = laid.dialog.item_bounds(LIST_ITEM).unwrap_or(NO_LIST);
        laid.rows = RowList::new(names.collect(), rect, &laid.metrics.0);
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

    /// Shows the selected offer's name and text, or none.
    fn show_selected(&mut self) {
        let Ok(laid) = &mut self.laid else {
            return;
        };
        let row = laid
            .rows
            .selected()
            .and_then(|index| self.computer.rows.get(index));
        laid.dialog
            .set_text(NAME_ITEM, row.map_or("", |row| row.name.as_str()));
        laid.dialog
            .set_text(TEXT_ITEM, row.map_or("", |row| row.text.as_str()));
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
    /// Up, Down and a press on the list go to the list; the leave keys
    /// leave; everything else goes to the dialog. It never quits.
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
        if *input
            == (Input::Key {
                key: MISSION_INFO_KEY,
                pressed: true,
                repeat: false,
            })
        {
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
        if on_list(&laid.dialog, LIST_ITEM, input) {
            if laid.rows.input(input) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        let event = laid.dialog.input(input);
        self.sounds.extend(laid.dialog.take_sound().map(Sound::Ui));
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

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

    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
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
            let text = |text: &str| Role::ScrollText {
                text: text.to_owned(),
                font: Font::Geneva,
                size: BODY_SIZE,
                color: BODY_COLOR,
            };
            let roles = [
                (INFO_DONE_ITEM, Role::Button(DONE_LABEL.to_owned())),
                (ABORT_ITEM, Role::Button(ABORT_LABEL.to_owned())),
                (INFO_HEADING_ITEM, text(ACTIVE_MISSIONS)),
                (INFO_TEXT_ITEM, text("")),
                (INFO_DATE_ITEM, text("")),
            ];
            let dialog = Dialog::new(&template, &roles, Rc::clone(&metrics))
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(Some(INFO_DONE_ITEM))
                .with_cancel(Some(INFO_DONE_ITEM));
            let list = dialog.item_bounds(INFO_LIST_ITEM).unwrap_or(NO_LIST);
            InfoLaid {
                rows: RowList::new(Vec::new(), list, &metrics),
                dialog,
                metrics: MetricsHandle(metrics),
            }
        });
        let mut screen = Self {
            laid,
            info: MissionInfo {
                rows: Vec::new(),
                today: String::new(),
            },
            abort: None,
            closed: false,
            sounds: Vec::new(),
        };
        screen.set_info(info);
        screen
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

    /// The slot Abort asked for since this was last taken, once.
    pub fn take_abort(&mut self) -> Option<u8> {
        self.abort.take()
    }

    /// Shows `info` in place of the one shown, keeping the selected slot
    /// selected while it is listed, and otherwise the first; it closes
    /// once nothing is listed (@0x9e5eb-0x9e610).
    pub fn set_info(&mut self, info: MissionInfo) {
        let kept = self.selected_row().map(|row| row.slot);
        self.info = info;
        if self.info.rows.is_empty() {
            self.closed = true;
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
        let rect = laid.dialog.item_bounds(INFO_LIST_ITEM).unwrap_or(NO_LIST);
        laid.rows = RowList::new(names.collect(), rect, &laid.metrics.0);
        if let Some(index) =
            kept.and_then(|slot| self.info.rows.iter().position(|row| row.slot == slot))
        {
            laid.rows.select(index);
        }
        laid.dialog.set_text(INFO_DATE_ITEM, &self.info.today);
        self.show_selected();
    }

    /// The row selected, if any.
    fn selected_row(&self) -> Option<&InfoRow> {
        let index = self.rows()?.selected()?;
        self.info.rows.get(index)
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
            .and_then(|index| self.info.rows.get(index));
        laid.dialog
            .set_text(INFO_TEXT_ITEM, row.map_or("", |row| row.text.as_str()));
        laid.dialog
            .set_greyed(ABORT_ITEM, !row.is_some_and(|row| row.abortable));
    }

    /// Activates dialog item `item`: Done closes, and Abort asks for the
    /// selected slot. Anything else does nothing.
    fn activate(&mut self, item: usize) {
        match item {
            INFO_DONE_ITEM => self.closed = true,
            ABORT_ITEM => self.abort = self.selected_row().map(|row| row.slot),
            _ => {}
        }
    }
}

impl Screen for MissionInfoScreen {
    /// Up, Down and a press on the list go to the list; everything else
    /// goes to the dialog. It never quits.
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
        if on_list(&laid.dialog, INFO_LIST_ITEM, input) {
            if laid.rows.input(input) {
                self.show_selected();
            }
            return ScreenAction::None;
        }
        let event = laid.dialog.input(input);
        self.sounds.extend(laid.dialog.take_sound().map(Sound::Ui));
        if let Some(DialogEvent::Item(item)) = event {
            self.activate(item);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

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

    fn cancel_pointer(&mut self) {
        if let Ok(laid) = &mut self.laid {
            laid.dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::ComputerRow;

    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

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

    fn row(mission: i16, name: &str, text: &str) -> ComputerRow {
        ComputerRow {
            mission: MissionId(mission),
            name: name.to_owned(),
            text: text.to_owned(),
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
        assert_eq!(item_text(&screen, NAME_ITEM), "Run to Mars");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Take this to Mars.");
        for label in [ACCEPT_LABEL, LEAVE_LABEL] {
            assert!(enabled(&screen, label), "{label}");
        }
    }

    #[test]
    fn selecting_another_offer_shows_its_name_and_text() {
        let mut screen = screen();
        screen.input(&key(Key::Down));
        assert_eq!(item_text(&screen, NAME_ITEM), "Walk");
        assert_eq!(item_text(&screen, TEXT_ITEM), "Go on foot.");
        let first = screen
            .rows()
            .and_then(|rows| rows.row_bounds(0))
            .expect("shown")
            .center();
        click_at(&mut screen, first);
        assert_eq!(item_text(&screen, NAME_ITEM), "Run to Mars");
        assert!(!screen.closed());
    }

    #[test]
    fn accept_asks_for_the_selected_mission_once() {
        let mut screen = screen();
        assert_eq!(screen.take_accept(), None);
        screen.input(&key(Key::Down));
        click_item(&mut screen, ACCEPT_ITEM);
        assert_eq!(screen.take_accept(), Some(MissionId(131)));
        assert_eq!(screen.take_accept(), None, "once");
        assert!(!screen.closed());
        screen.input(&key(Key::Space));
        assert_eq!(screen.take_accept(), None, "no key accepts");
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
            key: Key::Char('l'),
            pressed: true,
            repeat: true,
        });
        assert!(!repeating.closed(), "a repeat does nothing");
        let mut screen = screen();
        click_item(&mut screen, LEAVE_ITEM);
        assert!(screen.closed());
        assert_eq!(LEAVE_KEYS, [Key::Enter, Key::Escape, Key::Char('l')]);
    }

    #[test]
    fn a_new_computer_keeps_the_selected_mission_while_it_is_listed() {
        let mut screen = screen();
        screen.input(&key(Key::Down));
        screen.set_computer(MissionComputer {
            rows: vec![row(129, "Fly", "Up."), row(131, "Walk", "Go on foot.")],
            ..computer()
        });
        assert_eq!(screen.rows().and_then(RowList::selected), Some(1));
        assert_eq!(item_text(&screen, NAME_ITEM), "Walk");
        screen.set_computer(MissionComputer {
            rows: vec![row(129, "Fly", "Up.")],
            today: "June 24th, 1177 NC".to_owned(),
        });
        assert_eq!(screen.rows().and_then(RowList::selected), Some(0));
        assert_eq!(item_text(&screen, NAME_ITEM), "Fly");
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

    const DOWN: Sound = Sound::Ui(crate::sound::UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(crate::sound::UiSound::ButtonUp);

    #[test]
    fn its_buttons_sound_and_a_click_abandoned_does_nothing() {
        let mut screen = screen();
        click_item(&mut screen, ACCEPT_ITEM);
        assert_eq!(screen.take_sounds(), [DOWN, UP]);
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
        assert_eq!(screen.take_sounds(), [DOWN]);
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
        MissionInfo {
            rows: vec![
                info_row(0, "Run to Mars", false, "By June 30th.", true),
                info_row(3, "Stay", true, "Hold on.", false),
            ],
            today: "June 23rd, 1177 NC".to_owned(),
        }
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
        screen.input(&key(Key::Down));
        assert_eq!(info_text(&screen, INFO_TEXT_ITEM), "Hold on.");
        assert!(!info_enabled(&screen, ABORT_LABEL));
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), None, "greyed");
        screen.input(&key(Key::Up));
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_abort(), Some(0));
        assert_eq!(screen.take_abort(), None, "once");
        assert!(!screen.closed());
    }

    #[test]
    fn done_return_and_escape_close_mission_info() {
        for close in [Key::Enter, Key::Escape] {
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
        screen.input(&key(Key::Down));
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

    #[test]
    fn mission_infos_buttons_sound_and_a_click_abandoned_does_nothing() {
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
        assert_eq!(screen.take_sounds(), [DOWN]);
        click_info(&mut screen, ABORT_ITEM);
        assert_eq!(screen.take_sounds(), [DOWN, UP]);
        assert_eq!(screen.take_abort(), Some(0));
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
        for k in [
            Key::Tab,
            Key::Up,
            Key::Down,
            MISSION_INFO_KEY,
            crate::ui::player_info::PLAYER_INFO_KEY,
            Key::Char('a'),
        ]
        .into_iter()
        .chain(LEAVE_KEYS)
        {
            let mut screen = screen();
            for input in press_and_release(k) {
                screen.input(&input);
            }
            assert_eq!(screen.take_sounds(), [], "{k:?}");
        }
    }

    /// The original's `_MissionInfoFilter` (@0x9d6e5) maps Return and Escape
    /// to Done without a click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original_on_the_info_screen() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = info_screen();
            for input in press_and_release(k) {
                screen.input(&input);
            }
            assert_eq!(screen.take_sounds(), [], "{k:?}");
        }
    }
}
