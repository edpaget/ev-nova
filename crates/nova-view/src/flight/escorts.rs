//! The escort menu: which of the player's escorts a command goes to, and
//! how the menu is drawn over the flight (`_ShowEscortMenu` @0x5664a and
//! `_DrawEscortMenuInFrame` @0x56268 in the `EV Nova` executable).
//!
//! - E ([`MENU_KEY`]) opens the menu in full ([`FULL_LEVEL`]), selecting
//!   All Ships; with no escorts it stays shut and the flight says so
//!   ([`NO_ESCORTS`]). E again, or Return, closes it at once, fading or
//!   not. E's close beeps; Return's, and the fade's end, beep only while E
//!   is held, as the close block reads the key live (@0x6e01e-0x6e04d).
//! - While it shows, fading too, keys 1-5 ([`GROUP_KEYS`]) select All
//!   Ships, Fighters, Medium Ships, Warships or Freighters and show it in
//!   full again; a class the player has no escort of is not selected,
//!   though its press, silent, still restarts the timeout, holding a fade
//!   where it is (@0x6e14e-0x6e159). No modifier is read, so Command-5
//!   picks Freighters too. While the menu shows, the original's
//!   `_QuickKeyCheck` hides 1-5 from every other key block
//!   (@0xa106-0xa124), so they never reach the stellar slot keys or
//!   Command-5; once it has faded out or closed, 1-4 are the flight's
//!   stellar slot keys and Command-5 its second nearest-stellar key.
//! - Once more than [`MENU_TIMEOUT`] has passed since it was last shown in
//!   full or a press stamped it ([`EscortMenu::touch`] for a command that
//!   changed something), it fades out a level each flight step, the
//!   original's 32-frame fade (`*0x3b51a0`, @0x6dfd5-0x6dff9), and shuts
//!   at 0 ([`EscortMenu::update`]). On a frame no escort is flying, and
//!   one was on the last, or on its first frame shown, the group becomes
//!   All Ships and the fade starts at once (@0x6dfb3-0x6dfcb).
//! - When the flight clears an escort class's kept order
//!   (`nova_sim::flight_frame::StepRun::class_orders_cleared`,
//!   `_HandlePlayer` @0x6e388, @0x6e446), the menu, if it shows, comes
//!   back to full with its old stamp after that step's drop
//!   ([`EscortMenu::run_steps`]), so a fade under way starts again from 32
//!   and runs on from the next step, each clearing step of a tick again.
//! - A jump's arrival, a gate's exit and a take-off shut it at once and
//!   silently (`_HandlePlayer` @0x6c50e, `_HandlePlayerDockRequest`
//!   @0x6705a), as play entered from the main screen does (@0x1a3a3).
//! - A left click in its frame while it shows is the menu's
//!   ([`EscortMenu::click`], `PtInRect`, half-open), during a jump too: on
//!   a row other than the one selected it selects that row's group, with
//!   no test that the player has a ship of it, and no sound
//!   (@0x6a000-0x6a12b). The menu is asked before the floating map, as
//!   the original's branch runs first (@0x6a130).
//! - A command goes to the group selected, or to every escort while the
//!   menu is shut ([`EscortMenu::group`]), and the flight says what it
//!   changed ([`escort_command_message`]): Return to Hangar
//!   (Option-C) "returning to hangar.", or "returning to formation." when
//!   an escort that is no fighter went back to formation. A command given
//!   while a jump is under way says what the escorts will do instead
//!   ("will hold position.", `STR#` 2002 #150-#154).
//! - Fighters abandoned as the player jumps are told in brackets
//!   ([`fighters_abandoned_message`]).
//!
//! **Drawing** ([`EscortMenu::draw`]): a [`MENU_WIDTH`] x [`MENU_HEIGHT`]
//! frame at [`MENU_AT`], bordered in `cölr` `FloatingMap`, titled "Escort
//! Commands" (`STR#` 2002 #133) centred at the top; five rows
//! [`ROW_GAP`] apart, "1) All Ships" to "5) Freighters", white, or dark
//! grey for a class the player has no escort of; each class it has shows
//! its order right-aligned ("Formation" for none); and the selected row
//! sits on a bar of `cölr` `EscortHilite`. The colours come through the
//! [`EscortMenuLooks`] port, with the stock ones as a fallback. The
//! original blits the menu's frame image, black inside, by the floating
//! map's proc at its level (`_FloaterDrawCallback` @0x38c0f-0x38c5b,
//! `_BlitPixieFloatingRectDrawProc`), which ORs it in at level/32, so its
//! black ORs in nothing at any level: ours draws it as the floating map is
//! drawn ([`floating::draw`]), the border faded, the rest clipped to the
//! frame and faded to alpha level/32, no black drawn. As there, each
//! command fades on its own, so while it fades the bar shows through the
//! selected row's text. The original's erase bookkeeping (the counter's
//! 0 to -1, `_FloaterEraseCallback` @0x38e41-0x38ef6) has no counterpart,
//! as the view draws every frame afresh.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::bay::{ABANDONED_MANY, ABANDONED_ONE};
use nova_sim::escort::{ESCORT_COMMANDS, NEW_ORDERS, WILL_ATTACK, order_label};
use nova_sim::{ClassRow, Commanded, EscortCommand, EscortGroup};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::flight::floating;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::Key;
use crate::text::TextMetrics;
use crate::time::sixtieths;

pub use nova_sim::escort::NO_ESCORTS;

/// The escort menu key: the original's default (`Keys.nib`'s
/// `escortMenuKey`).
pub const MENU_KEY: Key = Key::Char('e');
/// The keys that select All Ships, Fighters, Medium Ships, Warships and
/// Freighters while the menu is open (hard-coded, @0xce722).
pub const GROUP_KEYS: [Key; 5] = [
    Key::Char('1'),
    Key::Char('2'),
    Key::Char('3'),
    Key::Char('4'),
    Key::Char('5'),
];
/// How long the menu stays open after it opens, or after the last
/// selection or command: 480 ticks of 60 Hz.
pub const MENU_TIMEOUT: Duration = Duration::from_secs(8);
/// The frame's top-left corner, in the window.
pub const MENU_AT: Point = Point::new(15.0, 150.0);
/// The frame's width.
pub const MENU_WIDTH: f32 = 160.0;
/// The frame's height.
pub const MENU_HEIGHT: f32 = 120.0;
/// How far apart the rows are.
pub const ROW_GAP: f32 = 18.0;
/// The menu's text size.
pub const MENU_TEXT_SIZE: f32 = 10.0;
/// The title's baseline, below the frame's top.
const TITLE_BASELINE: f32 = 15.0;
/// The first row's baseline, below the frame's top.
const ROW_BASELINE: f32 = 40.0;
/// How far in from the frame's sides the rows' text stands.
const TEXT_INSET: f32 = 7.0;
/// How far in from the frame's sides the highlight bar reaches.
const BAR_INSET: f32 = 4.0;
/// The highlight bar's top and bottom, below the frame's top for the
/// first row.
const BAR_TOP: f32 = 30.0;
const BAR_BOTTOM: f32 = 43.0;
/// A class row the player has no escort of: dark grey.
pub const ABSENT_ROW: Color = Color::from_rgb24(0x0060_6060);

/// The escort menu's colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortMenuColors {
    /// The frame's border: `cölr` `FloatingMap`.
    pub border: Color,
    /// The selected row's bar: `cölr` `EscortHilite`.
    pub hilite: Color,
}

impl EscortMenuColors {
    /// Stock `cölr` 128's: a near-black border and a dark red bar.
    pub const STOCK: Self = Self {
        border: Color::from_rgb24(0x000C_0C0C),
        hilite: Color::from_rgb24(0x0080_0000),
    };
}

/// The escort menu's look.
pub trait EscortMenuLooks {
    /// The menu's colours, from `cölr` 128, or [`EscortMenuColors::STOCK`]
    /// when it is missing or does not decode.
    fn escort_menu_colors(&self) -> EscortMenuColors;
}

/// A borrowed catalog is a catalog.
impl<T: EscortMenuLooks + ?Sized> EscortMenuLooks for &T {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        (**self).escort_menu_colors()
    }
}

/// A shared catalog is a catalog.
impl<T: EscortMenuLooks + ?Sized> EscortMenuLooks for Rc<T> {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        (**self).escort_menu_colors()
    }
}

/// What E did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    /// The menu opened, All Ships selected.
    Opened,
    /// The menu closed.
    Closed,
    /// The player has no escorts: the menu stays shut.
    NoEscorts,
}

/// The level the escort menu shows at in full: its counter (`*0x3b51a0`)
/// is 32 while it shows, and steps down to 0 as it fades out.
pub const FULL_LEVEL: u8 = 32;

/// [`MENU_TIMEOUT`] in the sixtieths `_TickCount` counts: 0x1e0.
const TIMEOUT_TICKS: u128 = 480;

/// The escort menu: shut, or showing at a level with a group selected
/// (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscortMenu {
    /// While it shows: its state.
    open: Option<Shown>,
}

/// The escort menu while it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    /// The group selected (`0xdee40`).
    group: EscortGroup,
    /// When it was last shown in full or a press stamped it (`*0x3b5658`):
    /// `None` for the stamp of 0 the fleet's emptying writes, which the
    /// flight's clock has always passed (`_TickCount` counts from boot,
    /// where the flight's clock starts at 0).
    since: Option<Duration>,
    /// Its level, 1 to [`FULL_LEVEL`] (the counter).
    level: u8,
    /// Whether an escort was flying on the last frame it showed (the count
    /// in `0xdee60`): `None` before its first frame, as the count is -1
    /// while the menu is shut (@0x6e188).
    flying: Option<bool>,
}

impl Shown {
    /// Shown in full, stamped at `now`: `_ShowEscortMenu` (@0x5664a) and
    /// the stamp after it.
    fn show(&mut self, now: Duration) {
        self.reshow();
        self.stamp(now);
    }

    /// Shown in full, its stamp kept: `_ShowEscortMenu` (@0x5664a) alone.
    fn reshow(&mut self) {
        self.level = FULL_LEVEL;
    }

    /// Stamped at `now`, its level kept.
    fn stamp(&mut self, now: Duration) {
        self.since = Some(now);
    }

    /// Whether its timeout has passed at `now`: `_TickCount` above the
    /// stamp + 480, strictly (@0x6dfd5-0x6dfea).
    fn timed_out(&self, now: Duration) -> bool {
        self.since
            .is_none_or(|since| sixtieths(now) > sixtieths(since) + TIMEOUT_TICKS)
    }
}

/// Whether `rows` show any escort.
fn any_escort(rows: &[ClassRow; 4]) -> bool {
    rows.iter().any(|row| row.present)
}

/// The menu's frame.
fn frame() -> Bounds {
    Bounds::at(MENU_AT, MENU_WIDTH, MENU_HEIGHT)
}

/// Row `n`'s rect, from 0: the bar's (@0x6a09f-0x6a125).
fn row_rect(n: usize) -> Bounds {
    let frame = frame();
    let step = ROW_GAP * n as f32;
    Bounds {
        min: Point::new(frame.min.x + BAR_INSET, frame.min.y + BAR_TOP + step),
        max: Point::new(frame.max.x - BAR_INSET, frame.min.y + BAR_BOTTOM + step),
    }
}

impl EscortMenu {
    /// Whether the menu shows: open, or fading out (its level above 0).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The level the menu shows at, 1 to [`FULL_LEVEL`]; none while it is
    /// shut.
    #[must_use]
    pub fn level(&self) -> Option<u8> {
        self.open.map(|shown| shown.level)
    }

    /// A left click at screen point `at`, at `now`, once a press
    /// (`_HandlePlayer` @0x6a000-0x6a12b): whether the menu took it, as it
    /// takes every click in its frame while it shows (`PtInRect`, the right
    /// and bottom edges out, @0x6a075-0x6a08c). A click on a row (its bar's
    /// rect, half-open too) selects that row's group, whether the player
    /// has a ship of it or not, and shows the menu in full, stamped
    /// (@0x6a0fc-0x6a119); one on the row selected, or on no row, changes
    /// nothing (@0x6a0e8-0x6a0f6, @0x6a12b). It plays no sound.
    pub fn click(&mut self, at: Point, now: Duration) -> bool {
        let Some(shown) = &mut self.open else {
            return false;
        };
        if !frame().contains_half_open(at) {
            return false;
        }
        let hit = (0..EscortGroup::ALL.len()).find(|&n| row_rect(n).contains_half_open(at));
        if let Some(group) = hit.map(|n| EscortGroup::ALL[n])
            && group != shown.group
        {
            shown.group = group;
            shown.show(now);
        }
        true
    }

    /// The group a command goes to: the one selected, or every escort
    /// while the menu is shut.
    #[must_use]
    pub fn group(&self) -> EscortGroup {
        self.open.map_or(EscortGroup::All, |shown| shown.group)
    }

    /// E, at `now`, the fleet's classes `rows`: opens the menu on All
    /// Ships, in full, or closes it while it shows, fading too.
    pub fn toggle(&mut self, now: Duration, rows: &[ClassRow; 4]) -> Toggled {
        if self.open.take().is_some() {
            return Toggled::Closed;
        }
        if !any_escort(rows) {
            return Toggled::NoEscorts;
        }
        self.open = Some(Shown {
            group: EscortGroup::All,
            since: Some(now),
            level: FULL_LEVEL,
            flying: None,
        });
        Toggled::Opened
    }

    /// Key `n` of [`GROUP_KEYS`] (from 0), at `now`: while the menu shows,
    /// selects its group when the player has an escort of it (All Ships
    /// always), shows the menu in full and says it did. A press of a class
    /// the player has none of selects nothing and keeps the level, but
    /// still restarts the [`MENU_TIMEOUT`] from `now`, as `_HandlePlayer`
    /// stamps the time on every press the menu takes (`_TickCount` into
    /// *0x3b5658, @0x6e14e-0x6e159), so a fade holds where it is.
    pub fn select(&mut self, n: usize, now: Duration, rows: &[ClassRow; 4]) -> bool {
        let Some(group) = EscortGroup::ALL.get(n).copied() else {
            return false;
        };
        let present = match group {
            EscortGroup::All => true,
            EscortGroup::Class(class) => rows.iter().any(|row| row.class == class && row.present),
        };
        let Some(shown) = &mut self.open else {
            return false;
        };
        if present {
            shown.group = group;
            shown.show(now);
        } else {
            shown.stamp(now);
        }
        present
    }

    /// A command that changed something, taken at `now`: the menu, if it
    /// shows, shows in full again, [`MENU_TIMEOUT`] from now
    /// (@0x6e299-0x6e2b4).
    pub fn touch(&mut self, now: Duration) {
        if let Some(shown) = &mut self.open {
            shown.show(now);
        }
    }

    /// The flight cleared an escort class's kept order
    /// (`nova_sim::flight_frame::StepRun::class_orders_cleared`): the menu,
    /// if it shows, shows in full again, its stamp and group kept, as
    /// `_ShowEscortMenu` (@0x5664a) writes 32 to the counter and draws,
    /// with no stamp and no sound. A fade under way starts again from 32,
    /// its timeout still running from the old stamp. A shut menu stays
    /// shut. [`EscortMenu::run_steps`] calls it after the step's drop.
    fn reshow(&mut self) {
        if let Some(shown) = &mut self.open {
            shown.reshow();
        }
    }

    /// Closes the menu at once.
    pub fn close(&mut self) {
        self.open = None;
    }

    /// The menu's frame at `now`, after `steps` flight steps, with an
    /// escort `flying` or not (`_HandlePlayer` @0x6df4a-0x6e052): whether
    /// it closed. On a frame whose `flying` differs from the last frame's
    /// (its first frame shown included) and none is flying, the group
    /// becomes All Ships and the stamp 0, so the fade starts at once
    /// (@0x6dfb3-0x6dfcb). Then, past its timeout, it drops a level for
    /// each step, one a `_HandlePlayer` frame (@0x6dfec-0x6dff9), and
    /// closes once it reaches 0 (@0x6e007).
    pub fn update(&mut self, now: Duration, steps: u32, flying: bool) -> bool {
        let Some(shown) = &mut self.open else {
            return false;
        };
        if shown.flying != Some(flying) {
            shown.flying = Some(flying);
            if !flying {
                shown.since = None;
                shown.group = EscortGroup::All;
            }
        }
        if steps == 0 || !shown.timed_out(now) {
            return false;
        }
        let drop = u8::try_from(steps).unwrap_or(u8::MAX);
        shown.level = shown.level.saturating_sub(drop);
        if shown.level > 0 {
            return false;
        }
        self.open = None;
        true
    }

    /// The menu's frame at `now` over a tick's `steps` flight steps, with
    /// an escort `flying` or not, `cleared` saying, step by step, whether
    /// that step cleared an escort class's kept order
    /// (`nova_sim::flight_frame::StepRun::class_orders_cleared`; a step it
    /// says nothing of cleared none): whether the menu closed.
    ///
    /// The frame's `flying` is taken first, as [`EscortMenu::update`] takes
    /// it, steps or none. Then each step runs as a `_HandlePlayer` frame
    /// does: the fade drops a level (@0x6dfec), and once that step's
    /// class-order bookkeeping (@0x6e2e8) cleared an order, the menu shows
    /// in full again ([`EscortMenu::reshow`]), so every clearing step of
    /// the tick shows it again, not only the last. A fade that reached 0
    /// has shut the menu, and a later clear leaves it shut (`jle`
    /// @0x6e386, @0x6e444).
    pub fn run_steps(
        &mut self,
        now: Duration,
        steps: u32,
        cleared: impl IntoIterator<Item = bool>,
        flying: bool,
    ) -> bool {
        self.update(now, 0, flying);
        let mut cleared = cleared.into_iter();
        for _ in 0..steps {
            if self.update(now, 1, flying) {
                return true;
            }
            if cleared.next().unwrap_or(false) {
                self.reshow();
            }
        }
        false
    }

    /// Draws the menu while it shows (see the module docs) with the
    /// fleet's classes `rows`, in `colors`, at its level, its text measured
    /// by `metrics` (without them, the title starts at the frame's left and
    /// an order at the right inset): as the floating map is drawn
    /// ([`floating::draw`]), the border faded and the rest clipped to the
    /// frame and faded, so its black inside ORs in nothing.
    pub fn draw(
        &self,
        list: &mut DrawList,
        rows: &[ClassRow; 4],
        colors: EscortMenuColors,
        metrics: Option<&dyn TextMetrics>,
    ) {
        let Some(shown) = self.open else {
            return;
        };
        floating::draw(list, frame(), shown.level, colors.border, |list| {
            draw_inside(list, shown.group, rows, colors, metrics);
        });
    }
}

/// The menu's inside with `selected` selected (see the module docs): its
/// title, rows, orders and bar, black where the original's frame image
/// is.
fn draw_inside(
    list: &mut DrawList,
    selected: EscortGroup,
    rows: &[ClassRow; 4],
    colors: EscortMenuColors,
    metrics: Option<&dyn TextMetrics>,
) {
    let frame = frame();
    let (left, top, right) = (frame.min.x, frame.min.y, frame.max.x);
    let width = |text: &str| {
        metrics.map_or(0.0, |metrics| {
            metrics.width(Font::Geneva, MENU_TEXT_SIZE, text)
        })
    };
    let title_x = if metrics.is_some() {
        left + (MENU_WIDTH - width(ESCORT_COMMANDS)) / 2.0
    } else {
        left
    };
    list.text(
        ESCORT_COMMANDS,
        Point::new(title_x, top + TITLE_BASELINE - MENU_TEXT_SIZE),
        MENU_TEXT_SIZE,
        None,
        Color::WHITE,
    );
    for (n, group) in EscortGroup::ALL.into_iter().enumerate() {
        let step = ROW_GAP * n as f32;
        if group == selected {
            fill_rect(list, row_rect(n), colors.hilite);
        }
        let row = match group {
            EscortGroup::All => None,
            EscortGroup::Class(class) => rows.iter().find(|row| row.class == class),
        };
        let present = row.is_none_or(|row| row.present);
        let color = if present { Color::WHITE } else { ABSENT_ROW };
        let text_top = top + ROW_BASELINE + step - MENU_TEXT_SIZE;
        list.text(
            format!("{}) {}", n + 1, group.menu_label()),
            Point::new(left + TEXT_INSET, text_top),
            MENU_TEXT_SIZE,
            None,
            color,
        );
        if let Some(row) = row.filter(|row| row.present) {
            let label = order_label(row.order);
            list.text(
                label,
                Point::new(right - TEXT_INSET - width(label), text_top),
                MENU_TEXT_SIZE,
                None,
                Color::WHITE,
            );
        }
    }
}

/// What the flight says of a command that changed something
/// (`_IssueNewEscortCommand` @0x660d5): "New escort orders assigned:  "
/// (two spaces), the group's name, a space, and what its escorts now do;
/// Attack with no target to report says "will attack.". A command given
/// while a jump is under way says what they will do instead
/// ([`EscortCommand::will`], @0x66461-0x664e5).
#[must_use]
pub fn escort_command_message(commanded: &Commanded) -> String {
    let doing = if commanded.underway {
        commanded.command.will()
    } else if commanded.command == EscortCommand::Attack && !commanded.targeted {
        WILL_ATTACK
    } else {
        commanded.command.doing()
    };
    format!("{NEW_ORDERS}{} {doing}", commanded.group.message_form())
}

/// `STR#` 137 #29-#38 of `Nova-DF.rsrc`: the numbers one to ten in
/// words, which `_StrcatOrdinalNumber` @0x8ab4 writes counts in.
pub use nova_sim::player_info::NUMBER_WORDS;

/// What the flight says of `count` fighters abandoned as the player
/// jumped (`_HandlePlayer` @0x6c3c4-0x6c42b): in brackets, the count in
/// words up to ten, its first letter capitalised, and in digits above
/// that, then `STR#` 2002 #164 or #165. The original adds it to its
/// arrival line, which the flight does not show yet.
#[must_use]
pub fn fighters_abandoned_message(count: u32) -> String {
    let words = usize::try_from(count)
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|index| NUMBER_WORDS.get(index));
    let number = words.map_or_else(
        || count.to_string(),
        |word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        },
    );
    let noun = if count == 1 {
        ABANDONED_ONE
    } else {
        ABANDONED_MANY
    };
    format!("({number} {noun})")
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use nova_sim::{EscortClass, EscortOrder};

    /// The fleet's rows: a warship escort holding, a freighter in
    /// formation, and no fighters or medium ships.
    fn rows() -> [ClassRow; 4] {
        [
            ClassRow {
                class: EscortClass::Fighter,
                present: false,
                order: None,
            },
            ClassRow {
                class: EscortClass::Medium,
                present: false,
                order: None,
            },
            ClassRow {
                class: EscortClass::Warship,
                present: true,
                order: Some(EscortOrder::Hold),
            },
            ClassRow {
                class: EscortClass::Freighter,
                present: true,
                order: None,
            },
        ]
    }

    /// No escorts at all.
    fn none() -> [ClassRow; 4] {
        rows().map(|row| ClassRow {
            present: false,
            ..row
        })
    }

    const AT: Duration = Duration::from_secs(100);

    /// `ticks` sixtieths after `at`.
    fn after(at: Duration, ticks: u32) -> Duration {
        at + nova_sim::mission::screens::mac_ticks(ticks)
    }

    /// A menu opened at [`AT`] with an escort flying, its first frame run.
    fn opened() -> EscortMenu {
        let mut menu = EscortMenu::default();
        assert_eq!(menu.toggle(AT, &rows()), Toggled::Opened);
        assert!(!menu.update(AT, 1, true));
        menu
    }

    #[test]
    fn opening_shows_it_in_full() {
        let menu = opened();
        assert_eq!(menu.level(), Some(FULL_LEVEL));
        assert_eq!(FULL_LEVEL, 32);
        assert_eq!(EscortMenu::default().level(), None, "shut");
    }

    #[test]
    fn its_timeout_passes_only_once_more_than_480_ticks_have() {
        // `_TickCount > stamp + 0x1e0` (@0x6dfd5-0x6dfea).
        let mut menu = opened();
        assert!(!menu.update(after(AT, 480), 1, true));
        assert_eq!(menu.level(), Some(32), "480 ticks: not yet");
        assert!(!menu.update(after(AT, 481), 1, true));
        assert_eq!(menu.level(), Some(31), "481: the fade's first step");
        assert_eq!(MENU_TIMEOUT, Duration::from_secs(8), "480 ticks");
    }

    #[test]
    fn past_its_timeout_it_drops_a_level_a_step_and_closes_at_0() {
        let mut menu = opened();
        let late = after(AT, 600);
        assert!(!menu.update(late, 0, true), "no step, no drop");
        assert_eq!(menu.level(), Some(32));
        for level in (1..32).rev() {
            assert!(!menu.update(late, 1, true));
            assert_eq!(menu.level(), Some(level));
            assert!(menu.is_open(), "it shows while it fades");
        }
        assert!(menu.update(late, 1, true), "the 32nd step closes it");
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        assert!(!menu.update(late, 1, true), "closed once");
    }

    #[test]
    fn a_frame_of_several_steps_drops_a_level_for_each() {
        let mut menu = opened();
        let late = after(AT, 600);
        assert!(!menu.update(late, 3, true));
        assert_eq!(menu.level(), Some(29));
        assert!(menu.update(late, 40, true), "past 0: closed");
    }

    #[test]
    fn a_present_groups_key_during_a_fade_shows_it_in_full_again() {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 10, true);
        assert_eq!(menu.level(), Some(22));
        assert!(menu.select(3, late, &rows()), "4: warships");
        assert_eq!(menu.level(), Some(32));
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Warship));
        menu.update(after(late, 480), 1, true);
        assert_eq!(menu.level(), Some(32), "restamped");
        menu.update(after(late, 481), 1, true);
        assert_eq!(menu.level(), Some(31));
    }

    #[test]
    fn an_absent_classs_key_restamps_and_keeps_the_level() {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 10, true);
        assert!(!menu.select(1, late, &rows()), "2: no fighters");
        assert_eq!(menu.level(), Some(22), "held where it was");
        assert_eq!(menu.group(), EscortGroup::All, "the group kept");
        menu.update(after(late, 480), 5, true);
        assert_eq!(menu.level(), Some(22), "until another timeout");
        menu.update(after(late, 481), 1, true);
        assert_eq!(menu.level(), Some(21), "then it resumes");
    }

    #[test]
    fn a_command_during_a_fade_shows_it_in_full_again() {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 10, true);
        menu.touch(late);
        assert_eq!(menu.level(), Some(32));
        menu.update(after(late, 480), 1, true);
        assert_eq!(menu.level(), Some(32), "restamped");
        let mut shut = EscortMenu::default();
        shut.touch(AT);
        assert!(!shut.is_open(), "a command does not open it");
    }

    #[test]
    fn a_reshow_during_a_fade_shows_it_in_full_keeping_its_stamp() {
        // `_ShowEscortMenu` (@0x5664a) writes 32 and draws, and stamps
        // nothing: the next step past the old timeout drops a level again.
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 10, true);
        assert!(menu.select(3, late, &rows()), "4: warships");
        menu.update(after(late, 481), 10, true);
        assert_eq!(menu.level(), Some(22));
        menu.reshow();
        assert_eq!(menu.level(), Some(32));
        assert_eq!(
            menu.group(),
            EscortGroup::Class(EscortClass::Warship),
            "the group kept"
        );
        assert!(!menu.update(after(late, 481), 1, true));
        assert_eq!(menu.level(), Some(31), "the old stamp's timeout");
    }

    #[test]
    fn a_reshow_of_a_shut_menu_leaves_it_shut() {
        let mut menu = EscortMenu::default();
        menu.reshow();
        assert!(!menu.is_open());
        let mut menu = opened();
        menu.close();
        menu.reshow();
        assert!(!menu.is_open());
    }

    /// A menu opened at [`AT`], past its timeout and faded to `level`, and
    /// the time it faded at.
    fn fading_at(level: u8) -> (EscortMenu, Duration) {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, u32::from(FULL_LEVEL - level), true);
        assert_eq!(menu.level(), Some(level));
        (menu, late)
    }

    #[test]
    fn steps_with_no_clear_drop_a_level_each() {
        let (mut menu, late) = fading_at(22);
        assert!(!menu.run_steps(late, 3, [false, false, false], true));
        assert_eq!(menu.level(), Some(19));
        assert!(!menu.run_steps(late, 2, [], true), "no report: no clears");
        assert_eq!(menu.level(), Some(17));
    }

    #[test]
    fn a_clear_shows_it_in_full_after_its_own_steps_drop() {
        // Each `_HandlePlayer` fades (@0x6dfec) before its class-order
        // bookkeeping (@0x6e2e8) shows the menu again.
        for (cleared, level) in [
            ([true, false, false], 30),
            ([false, true, false], 31),
            ([false, false, true], 32),
        ] {
            let (mut menu, late) = fading_at(22);
            assert!(!menu.run_steps(late, 3, cleared, true));
            assert_eq!(menu.level(), Some(level), "{cleared:?}");
        }
    }

    #[test]
    fn two_clears_in_one_frame_each_show_it_in_full() {
        // At 2: step 1 drops to 1 and shows it again, steps 2 and 3 drop
        // to 30, step 3's clear shows it again.
        let (mut menu, late) = fading_at(2);
        assert!(!menu.run_steps(late, 3, [true, false, true], true));
        assert_eq!(menu.level(), Some(32));
        let (mut menu, late) = fading_at(2);
        assert!(!menu.run_steps(late, 3, [true, false, false], true));
        assert_eq!(menu.level(), Some(30), "the first clear alone");
    }

    #[test]
    fn a_clear_after_the_fade_reached_0_leaves_it_shut() {
        // A fade that reached 0 has shut the menu, which the bookkeeping
        // leaves shut (`jle` @0x6e386, @0x6e444).
        let (mut menu, late) = fading_at(1);
        assert!(menu.run_steps(late, 2, [true, true], true), "it closed");
        assert!(!menu.is_open());
        let (mut menu, late) = fading_at(2);
        assert!(menu.run_steps(late, 3, [false, false, true], true));
        assert!(!menu.is_open(), "shut on step 2, before step 3's clear");
    }

    #[test]
    fn a_clear_shows_a_menu_not_yet_timed_out_in_full() {
        // A press of a class the player has none of restamps it and keeps
        // its level.
        let (mut menu, late) = fading_at(22);
        assert!(!menu.select(1, late, &rows()), "2: no fighters");
        assert!(!menu.run_steps(late, 2, [false, true], true));
        assert_eq!(menu.level(), Some(32), "no drop, and in full again");
    }

    #[test]
    fn steps_take_the_frames_flying_first() {
        // On a frame none is flying, and one was, the fade starts at once
        // and the group becomes All Ships, before the steps' drops.
        let mut menu = opened();
        assert!(menu.select(3, after(AT, 1), &rows()), "4: warships");
        assert!(!menu.run_steps(after(AT, 2), 0, [], false));
        assert_eq!(menu.group(), EscortGroup::All, "no step, all the same");
        assert!(!menu.run_steps(after(AT, 2), 2, [], false));
        assert_eq!(menu.level(), Some(30));
    }

    #[test]
    fn a_shut_menu_runs_no_steps() {
        let mut menu = EscortMenu::default();
        assert!(!menu.run_steps(AT, 2, [true, false], true));
        assert!(!menu.is_open());
    }

    #[test]
    fn e_during_a_fade_closes_it_at_once() {
        let mut menu = opened();
        menu.update(after(AT, 600), 10, true);
        assert_eq!(menu.toggle(after(AT, 600), &rows()), Toggled::Closed);
        assert!(!menu.is_open());
    }

    #[test]
    fn with_none_flying_the_group_becomes_all_and_the_fade_starts_at_once() {
        // @0x6dfb3-0x6dfcb: the stamp 0 and the group -1.
        let mut menu = opened();
        menu.select(3, AT, &rows());
        assert!(!menu.update(after(AT, 1), 1, false));
        assert_eq!(menu.group(), EscortGroup::All);
        assert_eq!(menu.level(), Some(31), "no wait");
    }

    #[test]
    fn with_none_flying_on_its_first_frame_it_fades_from_that_step() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        assert!(!menu.update(AT, 1, false));
        assert_eq!(menu.level(), Some(31));
    }

    #[test]
    fn the_fade_does_not_restart_while_none_stays_flying() {
        let mut menu = opened();
        menu.update(after(AT, 1), 1, false);
        menu.update(after(AT, 2), 1, false);
        assert_eq!(menu.level(), Some(30));
        assert!(menu.select(0, after(AT, 2), &rows()), "1: all ships");
        assert_eq!(menu.level(), Some(32), "shown again with none flying");
        menu.update(after(AT, 3), 1, false);
        assert_eq!(menu.level(), Some(32), "stamped by the press");
        menu.update(after(AT, 483), 1, false);
        assert_eq!(menu.level(), Some(31), "fading again once timed out");
    }

    #[test]
    fn a_ship_flying_again_does_not_stop_a_fade() {
        let mut menu = opened();
        menu.update(after(AT, 1), 1, false);
        menu.update(after(AT, 2), 1, true);
        assert_eq!(menu.level(), Some(30), "the stamp stays 0");
    }

    #[test]
    fn e_opens_the_menu_on_all_ships_and_closes_it() {
        let mut menu = EscortMenu::default();
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        assert_eq!(menu.toggle(AT, &rows()), Toggled::Opened);
        assert!(menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        assert_eq!(menu.toggle(AT, &rows()), Toggled::Closed);
        assert!(!menu.is_open());
        assert_eq!(MENU_KEY, Key::Char('e'));
    }

    #[test]
    fn with_no_escorts_e_does_not_open_it() {
        let mut menu = EscortMenu::default();
        assert_eq!(menu.toggle(AT, &none()), Toggled::NoEscorts);
        assert!(!menu.is_open());
        assert_eq!(NO_ESCORTS, "You don't have any escorts.");
    }

    #[test]
    fn keys_1_to_5_select_a_class_present_and_ignore_one_absent() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        assert!(menu.select(3, AT, &rows()), "4: warships");
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Warship));
        assert!(!menu.select(1, AT, &rows()), "2: no fighters");
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Warship));
        assert!(menu.select(4, AT, &rows()));
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Freighter));
        assert!(!menu.select(2, AT, &rows()), "3: no medium ships");
        assert!(menu.select(0, AT, &rows()), "1: all, always");
        assert_eq!(menu.group(), EscortGroup::All);
        assert!(!menu.select(5, AT, &rows()), "no sixth key");
        assert_eq!(
            GROUP_KEYS,
            ['1', '2', '3', '4', '5'].map(Key::Char),
            "hard-coded"
        );
    }

    #[test]
    fn while_shut_keys_1_to_5_do_nothing() {
        let mut menu = EscortMenu::default();
        for n in 0..5 {
            assert!(!menu.select(n, AT, &rows()));
        }
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
    }

    #[test]
    fn reopening_selects_all_ships_again() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        menu.select(3, AT, &rows());
        menu.close();
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        menu.toggle(AT, &rows());
        assert_eq!(menu.group(), EscortGroup::All);
    }

    // The row clicks.

    /// Row `n`'s top-left corner, from 0.
    fn row_corner(n: usize) -> Point {
        Point::new(
            MENU_AT.x + BAR_INSET,
            MENU_AT.y + BAR_TOP + ROW_GAP * n as f32,
        )
    }

    #[test]
    fn a_click_on_a_rows_top_left_selects_it_and_its_right_and_bottom_edges_dont() {
        for (n, group) in EscortGroup::ALL.into_iter().enumerate().skip(1) {
            let corner = row_corner(n);
            let mut menu = opened();
            assert!(menu.click(corner, AT), "row {n}'s corner");
            assert_eq!(menu.group(), group, "row {n}");
            let right = Point::new(MENU_AT.x + MENU_WIDTH - BAR_INSET, corner.y);
            let bottom = Point::new(corner.x, MENU_AT.y + BAR_BOTTOM + ROW_GAP * n as f32);
            for edge in [right, bottom] {
                let mut menu = opened();
                assert!(menu.click(edge, AT), "in the frame: taken");
                assert_eq!(menu.group(), EscortGroup::All, "row {n}: {edge:?} is out");
            }
        }
    }

    #[test]
    fn a_click_between_rows_is_taken_and_selects_nothing() {
        let mut menu = opened();
        let between = Point::new(row_corner(1).x + 10.0, MENU_AT.y + BAR_BOTTOM + 1.0);
        assert!(menu.click(between, AT));
        assert_eq!(menu.group(), EscortGroup::All);
    }

    #[test]
    fn a_click_on_the_frames_right_or_bottom_edge_is_not_taken() {
        let mut menu = opened();
        let right = Point::new(MENU_AT.x + MENU_WIDTH, MENU_AT.y + 50.0);
        let bottom = Point::new(MENU_AT.x + 50.0, MENU_AT.y + MENU_HEIGHT);
        assert!(!menu.click(right, AT));
        assert!(!menu.click(bottom, AT));
        assert!(menu.click(MENU_AT, AT), "its top-left corner is in");
        let mut shut = EscortMenu::default();
        assert!(!shut.click(MENU_AT, AT), "shut, it takes none");
    }

    #[test]
    fn a_click_on_an_absent_classs_row_selects_it() {
        // No test of the player's ships (@0x6a0fc-0x6a119).
        let mut menu = opened();
        assert!(menu.click(row_corner(1), AT), "2: no fighters");
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Fighter));
    }

    #[test]
    fn a_click_on_the_selected_row_leaves_the_stamp() {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 2, true);
        assert!(menu.click(row_corner(0), late), "all ships, selected");
        assert_eq!(menu.level(), Some(30), "nothing changed");
        menu.update(late, 1, true);
        assert_eq!(menu.level(), Some(29), "still timed out");
    }

    #[test]
    fn a_click_on_another_row_during_a_fade_shows_it_in_full() {
        let mut menu = opened();
        let late = after(AT, 600);
        menu.update(late, 10, true);
        assert!(menu.click(row_corner(4), late));
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Freighter));
        assert_eq!(menu.level(), Some(32));
        menu.update(after(late, 480), 1, true);
        assert_eq!(menu.level(), Some(32), "stamped");
    }

    /// Five units wide a character.
    struct Fixed;

    impl TextMetrics for Fixed {
        fn width(&self, _font: Font, _size: f32, text: &str) -> f32 {
            5.0 * text.chars().count() as f32
        }

        fn line_height(&self, _font: Font, size: f32) -> f32 {
            size
        }
    }

    const COLORS: EscortMenuColors = EscortMenuColors {
        border: Color::from_rgb24(0x0011_2233),
        hilite: Color::from_rgb24(0x0044_5566),
    };

    /// Text drawn clipped to the menu's frame, as the floating map's
    /// pass draws it ([`floating::clip_and_fade`]).
    fn text(text: &str, x: f32, y: f32, color: Color) -> DrawCommand {
        DrawCommand::Clipped {
            clip: frame(),
            command: Box::new(DrawCommand::Text {
                text: text.to_owned(),
                font: Font::Geneva,
                origin: Point::new(x, y),
                size: MENU_TEXT_SIZE,
                wrap_width: None,
                color,
            }),
        }
    }

    fn line(from: (f32, f32), to: (f32, f32), width: f32, color: Color) -> DrawCommand {
        DrawCommand::Line {
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            width,
            color,
        }
    }

    /// What the menu of [`rows`], warships selected, draws at `level`.
    fn drawn_at(level: u8) -> Vec<DrawCommand> {
        let mut menu = opened();
        menu.select(3, AT, &rows());
        let late = after(AT, 600);
        menu.update(late, u32::from(FULL_LEVEL - level), true);
        assert_eq!(menu.level(), Some(level));
        let mut list = DrawList::new();
        menu.draw(&mut list, &rows(), COLORS, Some(&Fixed));
        list.iter().cloned().collect()
    }

    /// `color` at alpha `alpha`.
    fn alpha(color: Color, alpha: u8) -> Color {
        Color { a: alpha, ..color }
    }

    #[test]
    fn the_menu_in_full_draws_its_frame_title_rows_orders_and_bar_and_no_black() {
        // `_BlitPixieFloatingRectDrawProc` ORs the frame's image in: its
        // black inside ORs in nothing (`_FloaterDrawCallback`
        // @0x38c0f-0x38c5b).
        let white = Color::WHITE;
        let expected = vec![
            line((15.0, 150.0), (175.0, 150.0), 1.0, COLORS.border),
            line((175.0, 150.0), (175.0, 270.0), 1.0, COLORS.border),
            line((175.0, 270.0), (15.0, 270.0), 1.0, COLORS.border),
            line((15.0, 270.0), (15.0, 150.0), 1.0, COLORS.border),
            text("Escort Commands", 57.5, 155.0, white),
            text("1) All Ships", 22.0, 180.0, white),
            text("2) Fighters", 22.0, 198.0, ABSENT_ROW),
            text("3) Medium Ships", 22.0, 216.0, ABSENT_ROW),
            line((19.0, 240.5), (171.0, 240.5), 13.0, COLORS.hilite),
            text("4) Warships", 22.0, 234.0, white),
            text("Hold Position", 103.0, 234.0, white),
            text("5) Freighters", 22.0, 252.0, white),
            text("Formation", 123.0, 252.0, white),
        ];
        assert_eq!(drawn_at(FULL_LEVEL), expected);
        assert_eq!(
            (MENU_AT, MENU_WIDTH, MENU_HEIGHT, ROW_GAP),
            (Point::new(15.0, 150.0), 160.0, 120.0, 18.0)
        );
    }

    #[test]
    fn at_level_16_every_colours_alpha_is_half_rounded() {
        let half = 128;
        let white = alpha(Color::WHITE, half);
        let expected = vec![
            line(
                (15.0, 150.0),
                (175.0, 150.0),
                1.0,
                alpha(COLORS.border, half),
            ),
            line(
                (175.0, 150.0),
                (175.0, 270.0),
                1.0,
                alpha(COLORS.border, half),
            ),
            line(
                (175.0, 270.0),
                (15.0, 270.0),
                1.0,
                alpha(COLORS.border, half),
            ),
            line(
                (15.0, 270.0),
                (15.0, 150.0),
                1.0,
                alpha(COLORS.border, half),
            ),
            text("Escort Commands", 57.5, 155.0, white),
            text("1) All Ships", 22.0, 180.0, white),
            text("2) Fighters", 22.0, 198.0, alpha(ABSENT_ROW, half)),
            text("3) Medium Ships", 22.0, 216.0, alpha(ABSENT_ROW, half)),
            line(
                (19.0, 240.5),
                (171.0, 240.5),
                13.0,
                alpha(COLORS.hilite, half),
            ),
            text("4) Warships", 22.0, 234.0, white),
            text("Hold Position", 103.0, 234.0, white),
            text("5) Freighters", 22.0, 252.0, white),
            text("Formation", 123.0, 252.0, white),
        ];
        assert_eq!(drawn_at(16), expected);
        let level_1 = drawn_at(1);
        assert!(
            level_1.iter().all(|command| match command {
                DrawCommand::Line { color, .. } => color.a == 8,
                DrawCommand::Clipped { command, .. } =>
                    matches!(**command, DrawCommand::Text { color, .. } if color.a == 8),
                _ => false,
            }),
            "255 × 1/32, rounded: {level_1:?}"
        );
    }

    #[test]
    fn without_metrics_the_title_starts_at_the_left_and_the_orders_at_the_inset() {
        let menu = opened();
        let mut list = DrawList::new();
        menu.draw(&mut list, &rows(), COLORS, None);
        let texts: Vec<_> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Clipped { command, .. } => match &**command {
                    DrawCommand::Text { text, origin, .. } => Some((text.clone(), origin.x)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(texts[0], ("Escort Commands".to_owned(), 15.0));
        assert_eq!(texts[5], ("Hold Position".to_owned(), 168.0));
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Line { color, width, .. } if *color == COLORS.hilite && *width == 13.0
            )),
            "All Ships' bar"
        );
    }

    #[test]
    fn the_shut_menu_draws_nothing() {
        let mut list = DrawList::new();
        EscortMenu::default().draw(&mut list, &rows(), COLORS, Some(&Fixed));
        assert!(list.is_empty());
        let mut faded = opened();
        assert!(faded.update(after(AT, 600), 32, true), "faded out");
        faded.draw(&mut list, &rows(), COLORS, Some(&Fixed));
        assert!(list.is_empty(), "at level 0");
    }

    #[test]
    fn the_stock_colours_are_cölr_128s() {
        assert_eq!(EscortMenuColors::STOCK.border, Color::from_rgb24(789_516));
        assert_eq!(EscortMenuColors::STOCK.hilite, Color::from_rgb24(8_388_608));
    }

    fn commanded(group: EscortGroup, command: EscortCommand, targeted: bool) -> String {
        escort_command_message(&Commanded {
            group,
            command,
            targeted,
            underway: false,
        })
    }

    #[test]
    fn a_command_is_told_as_the_original_tells_it() {
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Attack, true),
            "New escort orders assigned:  All ships attacking target."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Warship),
                EscortCommand::Hold,
                false
            ),
            "New escort orders assigned:  Warships holding position."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Freighter),
                EscortCommand::Recall,
                false
            ),
            "New escort orders assigned:  Freighters returning to formation."
        );
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Attack, false),
            "New escort orders assigned:  All ships will attack."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Medium),
                EscortCommand::Defend,
                false
            ),
            "New escort orders assigned:  Medium ships defending."
        );
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Dock, false),
            "New escort orders assigned:  All ships returning to hangar."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Fighter),
                EscortCommand::Dock,
                false
            ),
            "New escort orders assigned:  Fighters returning to hangar."
        );
    }

    #[test]
    fn a_command_under_way_is_told_with_the_will_strings() {
        let under_way = |group, command, targeted| {
            escort_command_message(&Commanded {
                group,
                command,
                targeted,
                underway: true,
            })
        };
        let freighters = EscortGroup::Class(EscortClass::Freighter);
        for (group, command, targeted, said) in [
            (
                EscortGroup::All,
                EscortCommand::Hold,
                false,
                "All ships will hold position.",
            ),
            (
                freighters,
                EscortCommand::Recall,
                false,
                "Freighters will return to formation.",
            ),
            (
                EscortGroup::All,
                EscortCommand::Defend,
                false,
                "All ships will defend.",
            ),
            (
                EscortGroup::All,
                EscortCommand::Dock,
                false,
                "All ships will return to hangar.",
            ),
            (
                EscortGroup::All,
                EscortCommand::Attack,
                false,
                "All ships will attack.",
            ),
            (
                EscortGroup::All,
                EscortCommand::Attack,
                true,
                "All ships will attack.",
            ),
        ] {
            assert_eq!(
                under_way(group, command, targeted),
                format!("New escort orders assigned:  {said}")
            );
        }
    }

    #[test]
    fn fighters_abandoned_are_counted_in_words_up_to_ten() {
        assert_eq!(fighters_abandoned_message(1), "(One fighter abandoned)");
        assert_eq!(fighters_abandoned_message(2), "(Two fighters abandoned)");
        assert_eq!(fighters_abandoned_message(10), "(Ten fighters abandoned)");
        assert_eq!(fighters_abandoned_message(11), "(11 fighters abandoned)");
        assert_eq!(fighters_abandoned_message(0), "(0 fighters abandoned)");
        assert_eq!(
            NUMBER_WORDS,
            [
                "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"
            ]
        );
        for (n, word) in (1..=10).zip(NUMBER_WORDS) {
            let message = fighters_abandoned_message(n);
            assert!(
                message.to_lowercase().starts_with(&format!("({word} ")),
                "{message}"
            );
        }
    }
}
