//! [`FlightView`]'s cockpit: controls and keys, the floating map, escorts,
//! the combat display and the status bar, drawn from the flight's frame
//! (its HUD and target parts). The world itself is drawn from the kept
//! frame (the `drawing` and `world` modules).

use nova_sim::PlayerCommand;
#[cfg(doc)]
use nova_sim::Session;
use nova_sim::flight_frame::{Answer, FlightFrame, ScreenAnswer, ScreenAsk};
use nova_sim::{
    ClassRow, Condition, EscortCommand, NavMode, Reserves, StellarPick, SystemId, TargetPick,
};

use crate::flight::escorts::{
    EscortMenu, GROUP_KEYS, MENU_KEY, NO_ESCORTS, Toggled, escort_command_message,
};
use crate::flight::floating::{self, Hit};
use crate::flight::hud::{self, HudState, StatusBar};
use crate::flight::target::{self};

use crate::flight::sim_port::FlightSim;
use crate::galaxy::MapMarks;
use crate::galaxy::map::MapPicture;
use crate::system::camera::VIEW_SIZE;
use crate::time::sixtieths;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{DrawList, Input, Key, Point, UiSound};
use nova_sim::flight_frame::FlightCommand;

use super::slot_block;
use super::{
    BOARD_KEY, DESELECT_KEY, EJECT_KEY, ESCORT_ATTACK_KEY, ESCORT_DEFEND_KEY, ESCORT_HOLD_KEY,
    ESCORT_RECALL_KEY, ESCORT_TICKS, FLIGHT_KEYS, FlightView, HAIL_KEY, HYPER_SELECT_KEY,
    HYPERSPACE_MODE_KEY, JETTISON_KEY, JUMP_KEY, LAND_KEY, MAP_KEY, MENU_CLOSE_KEY, NAV_OFF_KEY,
    NEAREST_KEY, NEAREST_STELLAR_COMMAND_KEY, NEAREST_STELLAR_KEY, SELECT_KEY, SELF_DESTRUCT_KEY,
    STELLAR_SLOT_KEYS, TARGET_KEY, ZOOM_IN_KEY, ZOOM_IN_KEYPAD_KEY, ZOOM_OUT_KEY, window_width,
};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Sets the view's size, `view` logical units: the app's widened size
    /// ([`Screen::set_widened_size`](crate::Screen::set_widened_size)).
    /// The camera, the HUD's places, the play area the flight is told
    /// at the next steps, the starfield and the fades all take it from
    /// the next frame on (see the module docs). Nova's own 1024 x 768
    /// draws as flight always has. The flight's game window takes its
    /// width at once ([`FlightCommand::SetWindowWidth`]), so a course map
    /// opened before the next step reads its asteroids stored by it.
    pub fn set_view_size(&mut self, view: (f32, f32)) {
        self.view = view;
        self.sim
            .apply(FlightCommand::SetWindowWidth(window_width(view)));
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The HUD's status bar, or why it cannot be shown.
    pub fn status_bar(&self) -> Result<&StatusBar, &str> {
        self.status_bar.as_ref().map_err(String::as_str)
    }

    /// The [`PlayerCommand`] a press of `key` sends, as [`Screen::input`](crate::Screen::input)
    /// dispatches it; `None` for a key that is no command.
    pub(super) fn command_of(&self, key: Key) -> Option<PlayerCommand> {
        let command = match key {
            LAND_KEY => PlayerCommand::Land,
            BOARD_KEY => PlayerCommand::Board,
            HAIL_KEY => PlayerCommand::Hail,
            MAP_KEY => PlayerCommand::Map,
            JUMP_KEY => PlayerCommand::Jump,
            JETTISON_KEY => PlayerCommand::Jettison,
            EJECT_KEY => PlayerCommand::Eject,
            MISSION_INFO_KEY => PlayerCommand::MissionInfo,
            PLAYER_INFO_KEY => PlayerCommand::PlayerInfo,
            HYPER_SELECT_KEY => PlayerCommand::SelectSystem,
            HYPERSPACE_MODE_KEY => PlayerCommand::Nav,
            TARGET_KEY | NEAREST_KEY => PlayerCommand::Target,
            MENU_KEY | MENU_CLOSE_KEY | ESCORT_ATTACK_KEY | ESCORT_DEFEND_KEY | ESCORT_HOLD_KEY
            | ESCORT_RECALL_KEY => PlayerCommand::Escort,
            _ if self.escort_menu.is_open() && GROUP_KEYS.contains(&key) => PlayerCommand::Escort,
            NEAREST_STELLAR_COMMAND_KEY if self.held.contains(&Key::Command) => PlayerCommand::Nav,
            SELECT_KEY => PlayerCommand::SelectWeapon,
            DESELECT_KEY => PlayerCommand::DeselectWeapon,
            NEAREST_STELLAR_KEY | NAV_OFF_KEY | ZOOM_OUT_KEY | ZOOM_IN_KEY | ZOOM_IN_KEYPAD_KEY => {
                PlayerCommand::Nav
            }
            _ if STELLAR_SLOT_KEYS.iter().any(|keys| keys.contains(&key)) => PlayerCommand::Nav,
            _ => return None,
        };
        Some(command)
    }

    /// Whether a press of `key` is taken while a jump is under way, the
    /// pre-jump stage included, with Option held (`option`) and the escort
    /// menu open (`menu_open`) or not. `_HandlePlayer` runs its key
    /// handling on every frame of a jump but the arrival (@0x6c625), and
    /// these keys' blocks read no jump state (+0x50):
    ///
    /// - Target Select, Tab, with Shift and Option ([`TARGET_KEY`],
    ///   @0x69f1c-0x69ffb), and Closest Target, R and Option-R
    ///   ([`NEAREST_KEY`], @0x6a9b7-0x6aae7);
    /// - Weapon Select, W and Option-W ([`SELECT_KEY`], @0x6ac12-0x6adbf).
    ///   Weapon Safety, S ([`DESELECT_KEY`], @0x6adc2-0x6ae1b), reads no
    ///   jump state either; it is held, not pressed, so no press filter
    ///   applies to it;
    /// - Option-\` ([`NAV_OFF_KEY`] with Option, @0x69a17-0x69a90), which
    ///   lets go of the ship target;
    /// - Board, B ([`BOARD_KEY`], @0x6a98b-0x6a9a8);
    /// - Hail, Y ([`HAIL_KEY`], @0x703cc-0x70405), which the session then
    ///   refuses, as `_HandlePlayerCommunication` does (@0x61f87);
    /// - Jettison, Command-K and Command-Option-K ([`JETTISON_KEY`],
    ///   @0x6fd54-0x6fdaf);
    /// - the escort menu's keys: E, Return, F, D, V and C, and 1-5 while the
    ///   menu shows, fading out too (@0x6dd00-0x6e452);
    /// - the floating map's zoom keys, `-`, `=` and the keypad's `+`
    ///   ([`ZOOM_OUT_KEY`], [`ZOOM_IN_KEY`], [`ZOOM_IN_KEYPAD_KEY`],
    ///   @0x70f4e-0x71110).
    ///
    /// Every other key is ignored, as the original gates it on +0x50 ≤ 0:
    /// \` without Option and H (@0x69ad4), \\ (@0x69be7), 1-4 and
    /// Command-5 with the menu shut, F1-F4 and F5 (@0x69d2e; held through
    /// the jump's end, one pressed during it acts after it, see
    /// [`FlightView::slot_keys_frame`]), M (@0x6a7a2), L (@0x6a8df), I
    /// (@0x70424), and J, which changes nothing during a jump
    /// (@0x6b0da-0x6b10d). P is refused apart
    /// ([`FlightView::refuse_player_info`]); the flight keys are tracked
    /// as held whatever this says, so H, \` and `\` dropped here act after
    /// the jump as their latches let them, and J, whose latch the jump
    /// sets, does not ([`FlightView::held_keys_frame`]). A click on the
    /// escort menu is no key press, so this filter never sees it: the menu
    /// takes it during a jump too, as its branch reads no +0x50
    /// (@0x6a000-0x6a12b, [`FlightView::click_escort_menu`]).
    pub(super) fn taken_during_jump(key: Key, option: bool, menu_open: bool) -> bool {
        match key {
            TARGET_KEY | NEAREST_KEY | SELECT_KEY | BOARD_KEY | HAIL_KEY | JETTISON_KEY
            | MENU_KEY | MENU_CLOSE_KEY | ESCORT_ATTACK_KEY | ESCORT_DEFEND_KEY
            | ESCORT_HOLD_KEY | ESCORT_RECALL_KEY | ZOOM_OUT_KEY | ZOOM_IN_KEY
            | ZOOM_IN_KEYPAD_KEY => true,
            NAV_OFF_KEY => option,
            _ => menu_open && GROUP_KEYS.contains(&key),
        }
    }
}

/// The escort menu, over the [`FlightSim`] port: its keys read the fleet
/// from the kept frame
/// ([`FrameHud::escort_rows`](nova_sim::flight_frame::FrameHud::escort_rows)) and
/// send their commands through it.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Selects the escort menu's group `n` (keys 1-5), if the player has
    /// an escort of it.
    fn select_escort_group(&mut self, n: usize) {
        let rows = self.escort_rows();
        if self.escort_menu.select(n, self.elapsed, &rows) {
            self.ui_sounds.push(UiSound::EscortGroupSelected);
        }
    }

    /// A press of `key`, if it is one of the escort menu's: E, Return,
    /// 1-5 while the menu is open, and F, D, V and C (Option-C Return to
    /// Hangar). Whether it was one.
    pub(super) fn escort_key(&mut self, key: Key) -> bool {
        let group = GROUP_KEYS.iter().position(|&group| group == key);
        match key {
            MENU_KEY => self.toggle_escort_menu(),
            MENU_CLOSE_KEY => self.acknowledge(),
            ESCORT_ATTACK_KEY => self.command_escorts(EscortCommand::Attack),
            ESCORT_DEFEND_KEY => self.command_escorts(EscortCommand::Defend),
            ESCORT_HOLD_KEY => self.command_escorts(EscortCommand::Hold),
            ESCORT_RECALL_KEY => self.command_escorts(if self.held.contains(&Key::Alt) {
                EscortCommand::Dock
            } else {
                EscortCommand::Recall
            }),
            _ => match group {
                Some(n) if self.escort_menu.is_open() => self.select_escort_group(n),
                _ => return false,
            },
        }
        true
    }

    /// E: opens or closes the escort menu, or says the player has no
    /// escorts. Opening it clears [`NO_ESCORTS`] from the message line,
    /// and only that (`_HandlePlayer` @0x6dedb-0x6df29).
    fn toggle_escort_menu(&mut self) {
        let rows = self.escort_rows();
        match self.escort_menu.toggle(self.elapsed, &rows) {
            Toggled::NoEscorts => {
                self.say_for(NO_ESCORTS, ESCORT_TICKS);
                self.ui_sounds.push(UiSound::NoEscorts);
            }
            Toggled::Opened => {
                self.clear_message(NO_ESCORTS);
                self.ui_sounds.push(UiSound::EscortMenuToggled);
            }
            Toggled::Closed => self.ui_sounds.push(UiSound::EscortMenuToggled),
        }
    }

    /// Return (`ackComm`, `_keyPrefs` +0xc): clears the message line,
    /// whatever it shows, closes the escort menu at once, fading too
    /// (`_HandlePlayer` @0x6dd00-0x6dd8a), and hides the floating map
    /// (@0x6dd6a-0x6dd75), during a jump too. The menu's close beeps only
    /// while E is held ([`FlightView::shut_escort_menu_by_key`]).
    fn acknowledge(&mut self) {
        self.acknowledge_message();
        if self.escort_menu.is_open() {
            self.escort_menu.close();
            self.shut_escort_menu_by_key();
        }
        self.floating.hide();
    }

    /// The beep of the escort menu's close by Return or by its fade's end:
    /// [`UiSound::EscortMenuToggled`] if E ([`MENU_KEY`]) is held, as the
    /// close block reads the key live (@0x6e01e-0x6e04d), and none
    /// otherwise.
    fn shut_escort_menu_by_key(&mut self) {
        if self.held.contains(&MENU_KEY) {
            self.ui_sounds.push(UiSound::EscortMenuToggled);
        }
    }

    /// Gives the escorts `command`, as the menu says which
    /// ([`FlightCommand::CommandEscorts`], `nova_sim::Session::command_escorts`,
    /// its speaker drawn on the flight's chance), once the flight takes the
    /// escort keys ([`FlightView::send_gated`]); says what it changed, and
    /// keeps the menu open for it. A command that changed no escort says
    /// nothing.
    fn command_escorts(&mut self, command: EscortCommand) {
        let group = self.escort_menu.group();
        if let Answer::Commanded(commanded) =
            self.send_gated(FlightCommand::CommandEscorts { group, command })
        {
            self.escort_menu.touch(self.elapsed);
            self.say_for(escort_command_message(&commanded), ESCORT_TICKS);
        }
    }

    /// Runs the escort menu's frame for the `steps` flight steps the tick
    /// ran ([`EscortMenu::update`]): one level of its fade for each, as
    /// `_HandlePlayer` drops one a frame, every frame, the warp's
    /// included, as the original runs it on every frame of a jump
    /// (@0x6df4a), with the escorts flying as the kept frame shows them
    /// ([`FrameHud::escorts_flying`](nova_sim::flight_frame::FrameHud::escorts_flying)):
    /// `Screen::tick` calls it right after the frame's steps, which fill
    /// the kept frame afresh. The fade's end beeps as Return's close does
    /// ([`FlightView::shut_escort_menu_by_key`]).
    pub(super) fn update_escort_menu(&mut self, steps: u32) {
        let flying = self.frame.hud.escorts_flying;
        if self.escort_menu.update(self.elapsed, steps, flying) {
            self.shut_escort_menu_by_key();
        }
    }

    /// A left click at screen point `at` on the escort menu
    /// ([`EscortMenu::click`]): whether the menu took it, so it goes no
    /// further, to the floating map or to space.
    pub(super) fn click_escort_menu(&mut self, at: Point) -> bool {
        self.escort_menu.click(at, self.elapsed)
    }

    /// The escort menu, shut or open.
    #[must_use]
    pub fn escort_menu(&self) -> &EscortMenu {
        &self.escort_menu
    }

    /// Shuts the escort menu at once and silently, as play is entered
    /// from the main screen (`escortMenuOnscreen` = 0,
    /// `_EnterGameFromMainScreen` @0x1a39e), and as the view catches up
    /// with a take-off, a gate's exit (`_HandlePlayerDockRequest`
    /// @0x6705a) or a jump's arrival (`_HandlePlayer` @0x6c50e).
    pub fn shut_escort_menu(&mut self) {
        self.escort_menu.close();
    }
}

/// The status bar, its gauges and the escort menu's rows, over the
/// [`FlightSim`] port: each reads the flight's frame
/// ([`FlightFrame::hud`], [`FlightFrame::target`]), the kept one, or the
/// one a draw is handed ([`FlightView::draw_bar`]).
impl<C, S: FlightSim> FlightView<C, S> {
    /// What the shield, armour and fuel gauges show this frame, of
    /// `frame`'s: while docked, by
    /// [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine
    /// reading, the snapshot taken at their last redraw, as the original's
    /// port draws them only at its redraws (`_statusRedraw[1]` and `[4]`):
    /// an outfitter order or a ship purchase under the same `ïntf` leaves
    /// them as they were (@0x5c59d, @0x5d324). In flight the same while
    /// the texts of a person's offer answered over flight show
    /// ([`FlightView::answer_in_flight`]), as the text dialog's modal loop
    /// draws no bar. Otherwise in flight, by the other reading, or before
    /// the first redraw, the gauges as they are now
    /// ([`FrameHud::reserves`](nova_sim::flight_frame::FrameHud::reserves)): flight's every frame marks them as they
    /// change.
    pub(super) fn reserves_shown(&self, frame: &FlightFrame) -> Reserves {
        match self.reserves_scene {
            Some(held) if (frame.docked() || !self.bar_steps.is_empty()) && self.hud_waits() => {
                held
            }
            _ => frame.hud.reserves,
        }
    }

    /// The ship's shield, armour and fuel now, as the flight's frame,
    /// filled afresh, holds them
    /// ([`FrameHud::reserves`](nova_sim::flight_frame::FrameHud::reserves));
    /// none for a session that never started. Read afresh, as the docked
    /// calls that change them (a recharge, an outfitter order, a
    /// mission's script) read them right after.
    pub(super) fn reserves(&self) -> Reserves {
        self.frame.hud.reserves
    }

    /// The escort menu's class rows, as the kept frame holds them
    /// ([`FrameHud::escort_rows`](nova_sim::flight_frame::FrameHud::escort_rows)); none present for a session that
    /// failed.
    pub(super) fn escort_rows(&self) -> [ClassRow; 4] {
        self.frame.hud.escort_rows
    }

    /// Draws the status bar's target panel ([`FlightView::target_shown`])
    /// and secondary weapon line, of `frame`'s target and secondary
    /// ([`FrameHud::secondary`](nova_sim::flight_frame::FrameHud::secondary), [`FrameHud::secondary_line_count`](nova_sim::flight_frame::FrameHud::secondary_line_count)). The
    /// weapon line follows every frame: the original redraws it
    /// (`_statusRedraw[3]`) on every event that changes it, so drawing it
    /// each frame shows the same.
    fn draw_combat_hud(
        &self,
        list: &mut DrawList,
        (bar, view_width): (&StatusBar, f32),
        frame: &FlightFrame,
    ) {
        let origin = hud::bar_origin(bar, view_width);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let shown = self.target_shown(frame);
        target::draw_target_panel(list, &bar.layout, origin, shown.as_ref().as_ref(), metrics);
        let line = frame.hud.secondary.map(|id| {
            let look = self.looks.weapon(id);
            let name = look.map_or_else(|| format!("wëap {}", id.0), |look| look.name.clone());
            target::secondary_text(&name, frame.hud.secondary_line_count)
        });
        target::draw_secondary(list, &bar.layout, origin, line.as_deref(), metrics);
    }

    /// Draws the HUD's status bar alone, as flight draws it: in port, the
    /// spaceport draws over it, as the original's port dialog sits beside
    /// the bar and its shops cover the bar's left edge (`_DoPortDialog`
    /// @0x5fbca draws the radar blank, as ours does while landed).
    /// Without a status bar it draws nothing: flight says why, and the
    /// spaceport stays clean. The spaceport is Nova's 1024 x 768, however
    /// wide flight's view is, so the bar is drawn at that width's right
    /// edge.
    ///
    /// It draws the kept frame ([`FlightView::frame`](FlightView#structfield.frame)),
    /// which every change made while docked fills afresh: a command
    /// ([`FlightView::send`]), a change behind the port
    /// ([`FlightView::change_sim`], which the test-only `transact` goes
    /// through), and the pilot desk's edits, which its caller resyncs.
    pub fn draw_status_bar(&self, list: &mut DrawList) {
        if let Ok(bar) = &self.status_bar {
            self.draw_bar(list, (bar, VIEW_SIZE.0), &self.frame);
        }
    }

    /// Draws `bar` showing `frame`'s HUD against the right edge of a view
    /// `view_width` wide: the radar, gauges and nav area as shown
    /// ([`FlightView::radar_shown`], [`FlightView::reserves_shown`],
    /// [`FlightView::nav_shown`]), the cargo area
    /// ([`FrameHud::cargo`](nova_sim::flight_frame::FrameHud::cargo)), the fuel gauge left out while the player is
    /// disabled, then the target panel and the secondary weapon's line.
    pub(super) fn draw_bar(
        &self,
        list: &mut DrawList,
        (bar, view_width): (&StatusBar, f32),
        frame: &FlightFrame,
    ) {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let radar = self.radar_shown(frame);
        let state = HudState {
            radar: &radar,
            blink_on: self.hud_cadence.blink_on(),
            reserves: self.reserves_shown(frame),
            nav: self.nav_shown(frame),
            cargo: Some(&frame.hud.cargo),
            disabled: frame.player().condition == Condition::Disabled,
        };
        hud::draw(list, bar, &state, metrics, view_width);
        self.draw_combat_hud(list, (bar, view_width), frame);
    }
}

/// The nav keys, the stellar slot keys and the floating map, over the
/// [`FlightSim`] port: they read the kept frame
/// ([`FlightView::frame`](FlightView#structfield.frame)) and send their
/// commands through it, so any
/// flight drives them, a scripted one in the component tests.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Tracks a flight key `input` presses or lets go of as held.
    pub(super) fn track_held(&mut self, input: &Input) {
        if let Input::Key { key, pressed, .. } = *input
            && FLIGHT_KEYS.contains(&key)
        {
            if pressed {
                self.held.insert(key);
            } else {
                self.held.remove(&key);
            }
        }
    }

    /// One frame of the keys `_HandlePlayer` polls with a latch before the
    /// Board and jump keys, with those held down, in its order: Option-\`
    /// and the Nav Off / Hyperspace Mode block
    /// ([`FlightView::nav_keys_frame`]), Hyper Select
    /// ([`FlightView::select_frame`]), then the stellar slot keys
    /// ([`FlightView::slot_keys_frame`]), the first two while the flight
    /// takes their command: the head of
    /// [`FlightView::held_keys_frame`]'s frame.
    pub(super) fn nav_held_frame(&mut self) {
        let down = |key: Key| self.held.contains(&key);
        let (nav_off, option, h) = (down(NAV_OFF_KEY), down(Key::Alt), down(HYPERSPACE_MODE_KEY));
        let select = down(HYPER_SELECT_KEY);
        if self.takes(PlayerCommand::Nav) {
            self.nav_keys_frame(nav_off, option, h);
        }
        if self.takes(PlayerCommand::SelectSystem) {
            self.select_frame(select);
        }
        self.slot_keys_frame();
    }

    /// Whether the flight takes `command` now, as the kept frame's gates
    /// say ([`FlightFrame::gates`](nova_sim::flight_frame::FlightFrame::gates),
    /// [`Session::takes`]): the frame the last command, step or change
    /// left, which each of them refills, so a key read mid-input sees the
    /// commands sent before it. A flight that failed to start takes every
    /// one, as its default frame does.
    pub(super) fn takes(&self, command: PlayerCommand) -> bool {
        self.frame.gates.takes(command)
    }

    /// A press of `key`, if it works the stellar navigation target. 1-4,
    /// like F1-F4, select the stellar in their slot
    /// ([`STELLAR_SLOT_KEYS`]); while the escort menu is open, 1-5 pick
    /// its groups instead and never reach here ([`FlightView::escort_key`]).
    /// F5 and Command-5 select the nearest ([`NEAREST_STELLAR_KEY`],
    /// [`NEAREST_STELLAR_COMMAND_KEY`]); each of these goes to the slot
    /// block's next frame ([`FlightView::slot_keys_frame`]). Nav Off turns
    /// the nav system off, or with Option held lets go of the ship target
    /// ([`NAV_OFF_KEY`]).
    pub(super) fn nav_key(&mut self, key: Key) {
        // The 5 alone goes too: the block picks nothing with it.
        if slot_block::is_block_key(key) {
            let menu_open = self.escort_menu.is_open();
            self.slot_block.press(key, &self.held, menu_open);
        } else if key == NAV_OFF_KEY {
            self.press_nav_block(key);
        }
    }

    /// One frame of the stellar slot keys' block, with the keys held and
    /// pressed since the last ([`slot_block`]): skipped
    /// while a jump is under way (@0x69d2b-0x69d32) or the session refuses
    /// the nav keys, otherwise at most one stellar picked
    /// ([`FlightView::select_stellar`]).
    pub(super) fn slot_keys_frame(&mut self) {
        let polled = self.slot_block_polled();
        let menu_open = self.escort_menu.is_open();
        if let Some(pick) = self.slot_block.frame(&self.held, menu_open, polled) {
            self.select_stellar(pick);
        }
    }

    /// Whether the slot block polls its keys this frame: not while a jump
    /// is under way (@0x69d2b-0x69d32) or the session refuses the nav keys.
    fn slot_block_polled(&self) -> bool {
        !self.jump_under_way() && self.takes(PlayerCommand::Nav)
    }

    /// Whether the flight holds input a later frame will act on, for a
    /// test harness to draw frames until it has settled rather than count
    /// them. Every flight key acts at its press, its latch's frame run
    /// then ([`Screen::input`](crate::Screen::input)), but the stellar
    /// slot keys, which go to the slot block's next frame
    /// ([`FlightView::slot_keys_frame`]): so this is the slot block's
    /// queue, its presses and a pick's latch still to clear (the
    /// `slot_block` module's `SlotBlock::queued`). A key held down is not
    /// queued input.
    #[cfg(any(test, feature = "fixture"))]
    pub fn input_queued(&self) -> bool {
        let menu_open = self.escort_menu.is_open();
        self.slot_block
            .queued(&self.held, menu_open, self.slot_block_polled())
    }

    /// A press of Hyper Select ([`HYPER_SELECT_KEY`]) or Hyperspace Mode
    /// ([`HYPERSPACE_MODE_KEY`]). The press's own frame shows the floating
    /// map: H's always (presses are taken only outside a jump), and
    /// `\`'s when it leaves the mode at 3, unless it found no system. A tap released before the next tick
    /// still shows it, as the frame's key poll would see it down.
    ///
    /// Each runs its key's routine with the key counted as down, its latch
    /// first cleared as the release before the press clears it: `\`'s
    /// always, H's unless \` without Option is still down, when the
    /// latch holds and H only shows the map (@0x69b8e-0x69ba3).
    ///
    /// A flight that failed to start shows nothing here: its frame's nav is
    /// off, and its floating map has no drawing.
    pub(super) fn hyperspace_key(&mut self, key: Key) {
        if key == HYPER_SELECT_KEY {
            self.select_latch = false;
            self.select_frame(true);
            if matches!(self.frame.nav_mode, NavMode::Hyperspace(_)) {
                self.show_floating_map();
            }
        } else {
            self.press_nav_block(key);
            self.show_floating_map();
        }
    }

    /// A fresh press of \` or H ([`NAV_OFF_KEY`], [`HYPERSPACE_MODE_KEY`]):
    /// the OS sends one only after a release, which clears the block's
    /// latch unless the block's other keys still hold it (H for \`; \`
    /// without Option for H). Then the block runs with the key down
    /// ([`FlightView::nav_keys_frame`]).
    fn press_nav_block(&mut self, key: Key) {
        let down = |held: Key| key == held || self.held.contains(&held);
        let option = self.held.contains(&Key::Alt);
        let others = if key == NAV_OFF_KEY {
            self.held.contains(&HYPERSPACE_MODE_KEY)
        } else {
            self.held.contains(&NAV_OFF_KEY) && !option
        };
        let (nav_off, h) = (down(NAV_OFF_KEY), down(HYPERSPACE_MODE_KEY));
        if !others {
            self.nav_latch = false;
        }
        self.nav_keys_frame(nav_off, option, h);
    }

    /// Whether the session has a jump under way, the pre-jump stage
    /// included: the original's +0x50 > 0, which its key blocks test live.
    /// A jump given up is over at once, whatever its effect still shows.
    pub(super) fn jump_under_way(&self) -> bool {
        self.frame.jump_under_way
    }

    /// One frame of Option-\` and the Nav Off / Hyperspace Mode block,
    /// with \` (`nav_off`), Option and H down or not, as `_HandlePlayer`
    /// runs them (@0x69a17-0x69baa) on [`FlightView::nav_latch`]:
    ///
    /// - With \` and Option down and the latch clear, the latch set and
    ///   the ship target let go ([`FlightCommand::ClearTarget`],
    ///   [`Session::clear_target`], @0x69a17-0x69a90), during a jump too.
    /// - Then, with \` without Option or H down outside a jump
    ///   (@0x69aa1-0x69ad4), and the latch clear, the latch set and the
    ///   block run ([`FlightCommand::NavModeKeys`],
    ///   [`Session::nav_mode_keys`]); otherwise, the latch cleared
    ///   (@0x69baa).
    ///
    /// So Option-\` held lets go of a target picked meanwhile on the next
    /// frame, unless H is down; Option let go with \` still down is a
    /// fresh Nav Off; H and \` block each other while one is held; and
    /// either held through a jump's end acts after it.
    pub(super) fn nav_keys_frame(&mut self, nav_off: bool, option: bool, h: bool) {
        let jumping = self.jump_under_way();
        if nav_off && option && !self.nav_latch {
            self.nav_latch = true;
            self.send(FlightCommand::ClearTarget);
        }
        if ((nav_off && !option) || h) && !jumping {
            if !self.nav_latch {
                self.nav_latch = true;
                self.send(FlightCommand::NavModeKeys { nav_off, h });
            }
        } else {
            self.nav_latch = false;
        }
    }

    /// One frame of Hyper Select, with `\` down or not, as `_HandlePlayer`
    /// runs it (@0x69bb2-0x69cee) on [`FlightView::select_latch`]: up, the
    /// latch cleared (@0x69cee); down with the latch clear outside a jump,
    /// the latch set (@0x69bf1) and the next linked system selected
    /// ([`Session::select_next_system`]). During a jump the latch is left
    /// as it is (@0x69be7): a `\` held from before stays latched, and one
    /// first pressed during the jump acts after it.
    pub(super) fn select_frame(&mut self, down: bool) {
        if !down {
            self.select_latch = false;
            return;
        }
        if self.select_latch || self.jump_under_way() {
            return;
        }
        self.select_latch = true;
        self.send(FlightCommand::SelectNextSystem);
    }

    /// Makes the stellar `pick` finds the navigation target, if any
    /// ([`FlightCommand::SelectStellar`]).
    pub(super) fn select_stellar(&mut self, pick: StellarPick) {
        self.send(FlightCommand::SelectStellar(pick));
    }

    /// The floating map's clock: the flight's elapsed time in Mac ticks
    /// (see the [`floating`](crate::flight::floating) module's trade-offs).
    fn mac_now(&self) -> u128 {
        sixtieths(self.elapsed)
    }

    /// Shows the floating map, restarting its clock, as
    /// `_ShowFloatingMap` does.
    ///
    /// Each show takes the map's drawing afresh, one answer to the show's
    /// ask ([`ScreenAsk::FloatingMap`]), as `_ShowFloatingMap` draws it
    /// into the frame then (`_DrawMapInFloatingFrame`): the session's
    /// drawing off the hypergate map (`_hypergateMapMode` = 0, @0x114cb)
    /// with the mission systems taken now (@0x114ea), and the system
    /// selected now (`Session::floating_preselection`,
    /// @0x1147b-0x114c8), whose name alone is drawn above a zoom of 1.1.
    /// It marks no green system, with multiple arrows allowed
    /// ([`MapMarks::NONE`](nova_sim::MapMarks::NONE)): it shows only from
    /// `_HandlePlayer`, never under a mission dialog's map.
    /// The frame shows that drawing until the next show: its course and
    /// mode-3 link are those of the last show, not live, as the
    /// original's frame image is drawn only then (at a show, Hyper Select,
    /// a click and a zoom, each of which shows it again here).
    ///
    /// So it asks at every show, and a show comes on every frame while H
    /// or `\` is held ([`FlightView::floating_frame`]): the flight steps
    /// between two held frames, and what it draws can change with no
    /// command from the view (a mission ship destroyed, a jump arrived
    /// at, a script's nav), which the frame's map key doesn't follow. A
    /// flight that answers nothing (one that failed to start) changes
    /// neither the picture nor the preselection.
    fn show_floating_map(&mut self) {
        self.floating.show(self.mac_now());
        // `_DrawMapInFloatingFrame` sets `_mapScroll` to the current
        // system as it draws (@0x11338-0x1136d), so the course map opens
        // there next.
        self.map.recentre();
        if let ScreenAnswer::FloatingMap(shown) = self.sim.screen(ScreenAsk::FloatingMap) {
            self.floating.preselect(shown.preselection);
            self.floating_picture = Some(MapPicture::of(&shown.drawing));
        }
    }

    /// The floating map's work each frame, as `_HandlePlayer` does it
    /// before `_HandleComm`: shown while H is held outside a jump
    /// (@0x69b8e-0x69ba8; its jump gate skips it, @0x69ad4), and while `\`
    /// is held in nav mode 3, jumping or not (@0x69cdd-0x69ce7; its jump
    /// gate jumps straight to the show, @0x69be7), each while the session
    /// takes its command; then hidden once more than 500 ticks have
    /// passed since it was last shown (@0x30262-0x30285). It runs once a
    /// frame, not once a pass: double time's second pass would only show
    /// it again at the same tick.
    pub(super) fn floating_frame(&mut self) {
        let jumping = self.jump_under_way();
        let mode_3 = matches!(self.frame.nav_mode, NavMode::Hyperspace(_));
        let h =
            self.held.contains(&HYPERSPACE_MODE_KEY) && !jumping && self.takes(PlayerCommand::Nav);
        let backslash = self.held.contains(&HYPER_SELECT_KEY)
            && mode_3
            && self.takes(PlayerCommand::SelectSystem);
        if h || backslash {
            self.show_floating_map();
        }
        self.floating.expire(self.mac_now());
    }

    /// A left click at `at`, if the floating map takes it, as
    /// `_HandlePlayer` does (@0x6a130-0x6a762): while the map is up, faded
    /// out or not, inside its frame. A hit on the current system's centre
    /// or a link's box, each at its place on the map (the kept frame's,
    /// [`FlightFrame::floating`](nova_sim::flight_frame::FlightFrame::floating)), selects through the flight
    /// ([`FlightCommand::SelectOnFloatingMap`],
    /// [`Session::select_on_floating_map`]) and shows the map again
    /// (@0x6a735); a miss is swallowed (@0x6a342). Whether it took it.
    /// During a jump the original passes the click on (@0x6a145), where a
    /// click in space selects nothing either; here the session refuses the
    /// hit, so the click does nothing and the map isn't shown again.
    pub(super) fn click_floating_map(&mut self, at: Point) -> bool {
        let frame = floating::frame(self.view);
        if !self.floating.is_up() || !floating::contains(frame, at) {
            return false;
        }
        let places = &self.frame.floating;
        let links: Vec<(SystemId, Point)> = places
            .links
            .iter()
            .map(|&(link, at)| (link, Point::new(at.x, at.y)))
            .collect();
        let current = Point::new(places.current.x, places.current.y);
        let picked = match floating::hit(&self.floating, frame, at, current, &links) {
            Hit::Centre => Some(None),
            Hit::Link(link) => Some(Some(link)),
            Hit::Miss | Hit::Outside => None,
        };
        if let Some(link) = picked
            && !self
                .send(FlightCommand::SelectOnFloatingMap(link))
                .is_refused()
        {
            self.show_floating_map();
        }
        true
    }

    /// A zoom key's press: while the floating map is up and no key the
    /// flight holds is down (`_quickKeyCheckResult` and the modifiers,
    /// @0x70f4e-0x70fb5), `-` steps it out and `=` or keypad `+` in. A `-`
    /// held alone doesn't count: `_HandlePlayer` checks it only with
    /// Command down (@0x6fdc8), so alone it never sets
    /// `_quickKeyCheckResult`. One
    /// that changes the scale shows the map again and raises
    /// [`UiSound::FloatingMapZoomed`] (`_beepSnd[0]`, @0x7103a, @0x710bc);
    /// one at a limit does nothing.
    pub(super) fn zoom_floating_map(&mut self, key: Key) {
        let holds = self.held.iter().any(|&held| {
            self.slot_key_polled(held, true)
                .unwrap_or(held != SELF_DESTRUCT_KEY)
        });
        if !self.floating.is_up() || holds {
            return;
        }
        let moved = if key == ZOOM_OUT_KEY {
            self.floating.zoom_out()
        } else {
            self.floating.zoom_in()
        };
        if moved {
            self.show_floating_map();
            self.ui_sounds.push(UiSound::FloatingMapZoomed);
        }
    }

    /// For a held stellar slot key `key` (1-4, F1-F5 or the 5), whether
    /// `_HandlePlayer` polls it this frame and so finds it down; `None` for
    /// any other key. The slot block polls F1-F5 always, 1-4 with the
    /// escort menu shut and the 5 with the menu shut, Command down and F5
    /// up (@0x69d38-0x69dda, the menu's mask @0xa106-0xa124), and none of
    /// them during a jump (@0x69d2b-0x69d32). With `escort_loop`, the
    /// escort menu's loop counts too: it polls 1-5 while the menu shows,
    /// during a jump too (@0x6df4a-0x6e186), setting
    /// `_quickKeyCheckResult` (@0xa18a-0xa191) but not the cursor's fade.
    pub(super) fn slot_key_polled(&self, key: Key, escort_loop: bool) -> Option<bool> {
        if !slot_block::is_block_key(key) {
            return None;
        }
        let menu_open = self.escort_menu.is_open();
        let block = !self.jump_under_way()
            && slot_block::block_polls(key, |down| self.held.contains(&down), menu_open);
        Some(block || (escort_loop && slot_block::escort_loop_polls(key, menu_open)))
    }

    /// Draws the floating map while it is up: centred on the current
    /// system at its zoom (the kept frame's place for it,
    /// [`FlightFrame::floating`](nova_sim::flight_frame::FlightFrame::floating),
    /// read at every draw, never asked), the drawing its last show took
    /// ([`FlightView::show_floating_map`]) through the course map's
    /// [`GalaxyMap::draw_marks`](crate::galaxy::GalaxyMap::draw_marks), with no brackets or second names, as
    /// `_DrawMapInFloatingFrame` runs `_DrawMap` alone, in its frame
    /// bordered in `cölr` `FloatingMap`, clipped and faded to its level
    /// ([`floating::draw`]).
    pub(super) fn draw_floating_map(&self, list: &mut DrawList) {
        let Some(level) = self.floating.level(self.mac_now()) else {
            return;
        };
        let Some(picture) = &self.floating_picture else {
            return;
        };
        let current = self.frame.floating.current;
        let at = Point::new(current.x, current.y);
        let frame = floating::frame(self.view);
        let transform = floating::transform(frame, at, self.floating.zoom());
        let marks = MapMarks {
            picture,
            selected: self.floating.preselection(),
        };
        floating::draw(list, frame, level, self.escort_colors.border, |raw| {
            self.map.draw_marks(raw, &transform, &marks);
        });
    }

    /// Picks the target as `pick` says ([`FlightCommand::SelectTarget`]).
    pub(super) fn select_target(&mut self, pick: TargetPick) {
        self.send(FlightCommand::SelectTarget(pick));
    }

    /// A press of Target Select ([`TARGET_KEY`]): the next ship, the
    /// previous with Shift, each among the player's escorts with Option
    /// ([`FlightCommand::SelectTarget`]).
    pub(super) fn press_target_key(&mut self) {
        let (shift, option) = (
            self.held.contains(&Key::Shift),
            self.held.contains(&Key::Alt),
        );
        self.select_target(match (shift, option) {
            (false, false) => TargetPick::Next,
            (false, true) => TargetPick::NextEscort,
            (true, false) => TargetPick::Previous,
            (true, true) => TargetPick::PreviousEscort,
        });
    }

    /// A press of Closest Target ([`NEAREST_KEY`]): the nearest threat, or
    /// the nearest ship with Option ([`FlightCommand::SelectTarget`]).
    pub(super) fn press_nearest_key(&mut self) {
        self.select_target(if self.held.contains(&Key::Alt) {
            TargetPick::Nearest
        } else {
            TargetPick::NearestThreat
        });
    }

    /// A press of Weapon Select ([`SELECT_KEY`]), backwards with Option
    /// held ([`FlightCommand::SelectWeapon`]). Option is read at the press,
    /// the original reading it at the pass (@0x6acf6): the press is taken
    /// on the next step, before a frame is drawn.
    pub(super) fn press_weapon_select(&mut self) {
        let backwards = self.held.contains(&Key::Alt);
        self.send(FlightCommand::SelectWeapon { backwards });
    }
}
