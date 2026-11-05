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

use crate::flight::escorts::{EscortMenu, NO_ESCORTS, Toggled, escort_command_message};
use crate::flight::floating::{self, Hit};
use crate::flight::hud::NavDisplay;
use crate::flight::hud::{self, HudState, StatusBar};
use crate::flight::radar::RadarScene;
use crate::flight::target::{self, TargetPanel};

use crate::flight::sim_port::FlightSim;
use crate::galaxy::MapMarks;
use crate::galaxy::map::MapPicture;
use crate::keymap::{Command, EscortMenuCommand, FlightCommand as F, PlayCommand};
use crate::system::camera::VIEW_SIZE;
use crate::time::sixtieths;
use crate::{DrawList, Input, Point, UiSound};
use nova_sim::flight_frame::FlightCommand;

use super::slot_block::{self, Column};
use super::{ESCORT_TICKS, FlightView, window_width};

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

    /// The [`PlayerCommand`] a press of `command` sends, as
    /// [`Screen::input`](crate::Screen::input) dispatches it; `None` for
    /// one that sends none.
    pub(super) fn player_command(command: Command) -> Option<PlayerCommand> {
        let sent = match command {
            Command::Flight(F::LandDock) => PlayerCommand::Land,
            Command::Flight(F::Board) => PlayerCommand::Board,
            Command::Flight(F::Communicate | F::HailStellar) => PlayerCommand::Hail,
            Command::Play(PlayCommand::GalaxyMap) => PlayerCommand::Map,
            Command::Flight(F::HyperJump) => PlayerCommand::Jump,
            Command::Flight(F::JettisonCargo | F::JettisonNonMission) => PlayerCommand::Jettison,
            Command::Flight(F::Eject) => PlayerCommand::Eject,
            Command::Play(PlayCommand::MissionInfo) => PlayerCommand::MissionInfo,
            Command::Play(PlayCommand::PlayerInfo) => PlayerCommand::PlayerInfo,
            Command::Flight(F::ShowFramerate) => PlayerCommand::FrameRate,
            Command::Flight(F::HyperSelect) => PlayerCommand::SelectSystem,
            Command::Flight(
                F::TargetSelect
                | F::TargetSelectBack
                | F::TargetEscort
                | F::TargetEscortBack
                | F::ClosestTarget
                | F::ClosestShip,
            ) => PlayerCommand::Target,
            Command::Flight(
                F::EscortMenu
                | F::Acknowledge
                | F::AttackTarget
                | F::DefendMe
                | F::HoldPosition
                | F::Recall
                | F::RecallToHangar,
            )
            | Command::EscortMenu(_) => PlayerCommand::Escort,
            Command::Flight(F::SelectSecondary | F::SelectSecondaryBack) => {
                PlayerCommand::SelectWeapon
            }
            Command::Flight(F::WeaponSafety) => PlayerCommand::DeselectWeapon,
            Command::Flight(
                F::HyperspaceMode
                | F::NavOff
                | F::ClearTarget
                | F::StellarSlot1
                | F::StellarSlot2
                | F::StellarSlot3
                | F::StellarSlot4
                | F::NearestStellar
                | F::ZoomOut
                | F::ZoomIn,
            ) => PlayerCommand::Nav,
            _ => return None,
        };
        Some(sent)
    }

    /// Whether a press of `command` is taken while a jump is under way,
    /// the pre-jump stage included. `_HandlePlayer` runs its key handling
    /// on every frame of a jump but the arrival (@0x6c625), and these
    /// keys' blocks read no jump state (+0x50):
    ///
    /// - Target Select, with Shift and Option ([`F::TargetSelect`] and its
    ///   variants, @0x69f1c-0x69ffb), and Closest Target, with Option
    ///   ([`F::ClosestTarget`], [`F::ClosestShip`], @0x6a9b7-0x6aae7);
    /// - Select Secondary, with Option ([`F::SelectSecondary`],
    ///   [`F::SelectSecondaryBack`], @0x6ac12-0x6adbf). Weapon Safety
    ///   ([`F::WeaponSafety`], @0x6adc2-0x6ae1b) reads no jump state
    ///   either; it is held, not pressed, so no press filter applies to
    ///   it;
    /// - Nav Off with Option ([`F::ClearTarget`], @0x69a17-0x69a90), which
    ///   lets go of the ship target;
    /// - Board ([`F::Board`], @0x6a98b-0x6a9a8);
    /// - Jettison, with Command and with Command and Option
    ///   ([`F::JettisonCargo`], [`F::JettisonNonMission`],
    ///   @0x6fd54-0x6fdaf);
    /// - the escort menu's keys: the menu key, Acknowledge, the four
    ///   escort commands, and the menu's groups while it shows, fading out
    ///   too (@0x6dd00-0x6e452);
    /// - the floating map's zooms ([`F::ZoomOut`], [`F::ZoomIn`],
    ///   @0x70f4e-0x71110);
    /// - Show Framerate ([`F::ShowFramerate`], @0x6e454-0x6e4bd).
    ///
    /// Every other command is ignored, as the original gates it on +0x50
    /// ≤ 0: Nav Off without Option and Hyperspace Mode (@0x69ad4), Hyper
    /// Select (@0x69be7), the stellar slots and the nearest stellar
    /// (@0x69d2e; held through the jump's end, one pressed during it acts
    /// after it, see [`FlightView::slot_keys_frame`]), and Hyper Jump,
    /// which changes nothing during a jump (@0x6b0da-0x6b10d). The map
    /// key, Land, Player Info, Communicate and Mission Info never get
    /// here: each press is kept for the pass that reads it, so P's and Y's
    /// refusals land on the pass, not on the press, and L and I held wait
    /// past the jump (the `faded_keys` module). The flight keys are
    /// tracked as held whatever this says, so Hyperspace Mode, Nav Off and
    /// Hyper Select dropped here act after the jump as their latches let
    /// them, and Hyper Jump, whose latch the jump sets, does not
    /// ([`FlightView::held_keys_frame`]). A click on the escort menu is no
    /// key press, so this filter never sees it: the menu takes it during a
    /// jump too, as its branch reads no +0x50 (@0x6a000-0x6a12b,
    /// [`FlightView::click_escort_menu`]).
    pub(super) const fn taken_in_jump(command: Command) -> bool {
        matches!(
            command,
            Command::Flight(
                F::TargetSelect
                    | F::TargetSelectBack
                    | F::TargetEscort
                    | F::TargetEscortBack
                    | F::ClosestTarget
                    | F::ClosestShip
                    | F::SelectSecondary
                    | F::SelectSecondaryBack
                    | F::ClearTarget
                    | F::Board
                    | F::JettisonCargo
                    | F::JettisonNonMission
                    | F::EscortMenu
                    | F::Acknowledge
                    | F::AttackTarget
                    | F::DefendMe
                    | F::HoldPosition
                    | F::Recall
                    | F::RecallToHangar
                    | F::ZoomOut
                    | F::ZoomIn
                    | F::ShowFramerate
            ) | Command::EscortMenu(_)
        )
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

    /// A press of `command`, if it is one of the escort menu's: the menu
    /// key, Acknowledge, the groups while the menu is open, and the four
    /// escort commands (Recall with Option, Return to Hangar). Whether it
    /// was one.
    pub(super) fn escort_command(&mut self, command: Command) -> bool {
        match command {
            Command::Flight(F::EscortMenu) => self.toggle_escort_menu(),
            Command::Flight(F::Acknowledge) => self.acknowledge(),
            Command::Flight(F::AttackTarget) => self.command_escorts(EscortCommand::Attack),
            Command::Flight(F::DefendMe) => self.command_escorts(EscortCommand::Defend),
            Command::Flight(F::HoldPosition) => self.command_escorts(EscortCommand::Hold),
            Command::Flight(F::Recall) => self.command_escorts(EscortCommand::Recall),
            Command::Flight(F::RecallToHangar) => self.command_escorts(EscortCommand::Dock),
            Command::EscortMenu(group) if self.escort_menu.is_open() => {
                let n = EscortMenuCommand::ALL
                    .iter()
                    .position(|&each| each == group)
                    .unwrap_or_default();
                self.select_escort_group(n);
            }
            _ => return false,
        }
        true
    }

    /// E: opens or closes the escort menu, or says the player has no
    /// escorts. Opening it clears [`NO_ESCORTS`] from the message line,
    /// and only that (`_HandlePlayer` @0x6dedb-0x6df29).
    fn toggle_escort_menu(&mut self) {
        let rows = self.escort_rows();
        let toggled = self.escort_menu.toggle(self.elapsed, &rows);
        self.sync_escort_keys();
        match toggled {
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
            self.sync_escort_keys();
            self.shut_escort_menu_by_key();
        }
        self.floating.hide();
    }

    /// The beep of the escort menu's close by Return or by its fade's end:
    /// [`UiSound::EscortMenuToggled`] if the menu key ([`F::EscortMenu`])
    /// is held, as the close block reads the key live (@0x6e01e-0x6e04d),
    /// and none otherwise.
    fn shut_escort_menu_by_key(&mut self) {
        if self.held(F::EscortMenu) {
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
    /// ran ([`EscortMenu::run_steps`]): one level of its fade for each, as
    /// `_HandlePlayer` drops one a frame, every frame, the warp's
    /// included, as the original runs it on every frame of a jump
    /// (@0x6df4a), with the escorts flying as the kept frame shows them
    /// ([`FrameHud::escorts_flying`](nova_sim::flight_frame::FrameHud::escorts_flying)),
    /// and shown in full again after each step of the tick's report that
    /// cleared an escort class's kept order
    /// (`nova_sim::flight_frame::StepRun::class_orders_cleared`):
    /// `Screen::tick` calls it right after the frame's steps, which fill
    /// the kept frame afresh. The fade's end beeps as Return's close does
    /// ([`FlightView::shut_escort_menu_by_key`]).
    ///
    /// In double time the original fades a level on each pass:
    /// `_DoPlayGameWork` calls `_HandlePlayer` on both (@0x43e19) with no
    /// pass argument, and nothing gates the fade (@0x6df4a-0x6e052) on the
    /// pass. Ours drops one a step, the re-show after the step's drop:
    /// rdm:phase/flight-controls/phase-7-handle-player-double-fade-and-arrival-orders.
    pub(super) fn update_escort_menu(&mut self, steps: u32) {
        let flying = self.frame.hud.escorts_flying;
        let cleared = self
            .step_report
            .steps
            .iter()
            .map(|run| run.class_orders_cleared);
        let shut = self
            .escort_menu
            .run_steps(self.elapsed, steps, cleared, flying);
        self.sync_escort_keys();
        if shut {
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
        self.sync_escort_keys();
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

    /// Draws the status bar's target panel, showing `shown`, and the
    /// secondary weapon line, of `frame`'s secondary
    /// ([`FrameHud::secondary`](nova_sim::flight_frame::FrameHud::secondary), [`FrameHud::secondary_line_count`](nova_sim::flight_frame::FrameHud::secondary_line_count)). The
    /// weapon line follows every frame: the original redraws it
    /// (`_statusRedraw[3]`) on every event that changes it, so drawing it
    /// each frame shows the same.
    fn draw_combat_hud(
        &self,
        list: &mut DrawList,
        (bar, view_width): (&StatusBar, f32),
        (frame, shown): (&FlightFrame, Option<&TargetPanel>),
    ) {
        let origin = hud::bar_origin(bar, view_width);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        target::draw_target_panel(list, &bar.layout, origin, shown, metrics);
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
    /// disabled, then the target panel as shown
    /// ([`FlightView::target_shown`]) and the secondary weapon's line.
    pub(super) fn draw_bar(
        &self,
        list: &mut DrawList,
        (bar, view_width): (&StatusBar, f32),
        frame: &FlightFrame,
    ) {
        let radar = self.radar_shown(frame);
        let target = self.target_shown(frame);
        let parts = BarParts {
            radar: &radar,
            reserves: self.reserves_shown(frame),
            nav: self.nav_shown(frame),
            target: target.as_ref().as_ref(),
        };
        self.draw_bar_of(list, (bar, view_width), frame, parts);
    }

    /// Draws `bar` against the right edge of a view `view_width` wide,
    /// showing `parts` and the rest of `frame`'s HUD: the cargo area, the
    /// fuel gauge left out while the player is disabled, and the
    /// secondary weapon's line.
    pub(super) fn draw_bar_of(
        &self,
        list: &mut DrawList,
        (bar, view_width): (&StatusBar, f32),
        frame: &FlightFrame,
        parts: BarParts<'_>,
    ) {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let state = HudState {
            radar: parts.radar,
            blink_on: self.hud_cadence.blink_on(),
            reserves: parts.reserves,
            nav: parts.nav,
            cargo: Some(&frame.hud.cargo),
            disabled: frame.player().condition == Condition::Disabled,
        };
        hud::draw(list, bar, &state, metrics, view_width);
        self.draw_combat_hud(list, (bar, view_width), (frame, parts.target));
    }
}

/// What a status bar shows that is read apart from the frame it draws
/// ([`FlightView::draw_bar_of`]): the radar, the gauges, the nav area and
/// the target panel.
pub(super) struct BarParts<'a> {
    /// What the radar shows.
    pub(super) radar: &'a RadarScene,
    /// The shield, armour and fuel.
    pub(super) reserves: Reserves,
    /// What the nav area shows.
    pub(super) nav: NavDisplay,
    /// What the target panel shows, if anything.
    pub(super) target: Option<&'a TargetPanel>,
}

/// The nav keys, the stellar slot keys and the floating map, over the
/// [`FlightSim`] port: they read the kept frame
/// ([`FlightView::frame`](FlightView#structfield.frame)) and send their
/// commands through it, so any
/// flight drives them, a scripted one in the component tests.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Tracks a flight key `input` presses or lets go of as held
    /// ([`FlightView::is_flight_key`]): flight's reader follows it, so
    /// the commands it holds are held while it is down. A repeat holds a
    /// key too, as a repeat is the first flight sees of a key whose press
    /// went elsewhere, such as one down as focus comes back: the original
    /// polls `GetKeys` and reads such a key down. Two exceptions:
    ///
    /// - A repeat of the land or mission info key is no fresh down: they
    ///   act on being held (the `faded_keys` module), so one taken for a
    ///   press would read again once a refusal let go of the key.
    /// - A repeat of a key whose commands a `_SetAllKeyCmdFlags` site
    ///   latched while it was down ([`FlightView::latched_down`]) is
    ///   dropped: its latch waits for the release, the key reading down
    ///   meanwhile past it ([`FlightView::reads_down`]). That release, or
    ///   a fresh press, which the OS sends only after a release, ends the
    ///   wait.
    pub(super) fn track_held(&mut self, input: &Input) {
        let Input::Key {
            pressed, repeat, ..
        } = *input
        else {
            return;
        };
        self.sync_escort_keys();
        if !self.is_flight_key(input) {
            return;
        }
        let waits_held = |command: &Command| {
            matches!(
                command,
                Command::Flight(F::LandDock) | Command::Play(PlayCommand::MissionInfo)
            )
        };
        if repeat && self.keys.bound_to(input).iter().any(waits_held) {
            return;
        }
        let meant = self.keys.meanings(input);
        if repeat {
            if meant
                .iter()
                .any(|command| self.latched_down.contains(command))
            {
                return;
            }
        } else if pressed {
            self.latched_down.retain(|command| !meant.contains(command));
        }
        if !pressed {
            self.after_poll.retain(|kept| !kept.same_key(input));
        }
        self.keys.follow(input);
        self.latched_down.retain(|&command| self.keys.held(command));
    }

    /// Lets go of the flight keys and the mouse button: flight is giving
    /// up its input, to a dialog over it or by the router's
    /// [`Screen::release_keys`](crate::Screen::release_keys), so their
    /// releases will never reach it. The one place that does: the
    /// original reads both live (`_KeyCheck` @0xa19b-0xa1dc, `_Button`
    /// @0x3803b), and ours has only the events flight is given. So the
    /// router, which sees the input under the dialog, gives back what is
    /// still down as the last dialog over flight closes
    /// ([`FlightView::take_back_live_input`]); a path that misses it
    /// leaves the keys let go, never stuck down. The paths that keep the
    /// input in flight (a gate's exit, the course map opened over a
    /// screen) let go of the held keys alone ([`FlightView::let_go_of_held`]), as their releases still come here,
    /// and the course map opened in flight and a gate's exit keep the
    /// keys read with no latch, B and L among them
    /// ([`FlightView::open_map_as`], `FlightView::came_through`). The
    /// latched keys flight saw down are forgotten too, as their releases
    /// go elsewhere: the next hand-back says again which are down. The
    /// latches themselves are left as they are, as a dialog runs no pass. It marks no `_SetAllKeyCmdFlags` site,
    /// so a dialog it alone opens closes by the latches as they were
    /// (the `dialog_latch` module); a site's dialog opens through
    /// [`FlightView::let_go_into_dialog`].
    pub(super) fn let_go_of_input(&mut self) {
        self.keys.let_go();
        self.latched_down.clear();
        self.after_poll.clear();
        self.faded_keys.forget();
        self.cursor.button(false);
    }

    /// Lets go of the keys of the held commands alone, as the course map
    /// opened over a screen does, whose releases still come here: a key
    /// whose commands a close latched stays down, waiting for its release
    /// ([`FlightView::latched_down`]).
    pub(super) fn let_go_of_held(&mut self) {
        for press in self.keys.down() {
            let latched = self
                .keys
                .meanings(&press)
                .iter()
                .any(|command| self.latched_down.contains(command));
            if !latched {
                self.keys.follow(&press.released());
            }
        }
        self.latched_down.retain(|&command| self.keys.held(command));
    }

    /// One frame of the keys `_HandlePlayer` polls with a latch before the
    /// Board and jump keys, with those read down, a latched one held past
    /// a close included ([`FlightView::reads_down`]), in its order:
    /// Option-\` and the Nav Off / Hyperspace Mode block
    /// ([`FlightView::nav_keys_frame`]), Hyper Select
    /// ([`FlightView::select_frame`]), then the stellar slot keys
    /// ([`FlightView::slot_keys_frame`]), the first two while the flight
    /// takes their command: the head of
    /// [`FlightView::held_keys_frame`]'s frame. So a latched key holds
    /// the latch it shares: a latched H keeps a fresh \` from acting
    /// (@0x69ae6), a latched slot key a fresh one from picking (@0x69e15).
    ///
    /// Option is read with Nav Off's key: [`F::ClearTarget`] down is Nav
    /// Off's key down with Option, so Option let go of with it still down
    /// is a fresh Nav Off, as the original reads Option on every frame.
    pub(super) fn nav_held_frame(&mut self) {
        let option = self.reads_down(F::ClearTarget);
        let nav_off = option || self.reads_down(F::NavOff);
        let h = self.reads_down(F::HyperspaceMode);
        let select = self.reads_down(F::HyperSelect);
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

    /// A press of `command`, if it works the stellar navigation target.
    /// The stellar slots, 1-4 or their F-key twins, select the stellar in
    /// their slot ([`F::StellarSlot1`] and on); while the escort menu is
    /// open, 1-5 pick its groups instead and never reach here
    /// ([`FlightView::escort_command`]). The nearest stellar, F5 or Command-5,
    /// selects the nearest ([`F::NearestStellar`]); each of these goes to
    /// the slot block's next frame ([`FlightView::slot_keys_frame`]), by
    /// its key's `column`, its binding or its fixed twin. Nav Off turns
    /// the nav system off, and with Option held ([`F::ClearTarget`]) lets
    /// go of the ship target.
    pub(super) fn nav_command(&mut self, command: Command, column: Column) {
        if slot_block::is_block_command(command) {
            let down = self.slot_columns_down();
            self.slot_block
                .press(command, column, |other, at| down.contains(&(other, at)));
        } else if let Command::Flight(pressed @ (F::NavOff | F::ClearTarget)) = command {
            self.press_nav_block(pressed);
        }
    }

    /// One frame of the stellar slot keys' block, with the commands held
    /// and pressed since the last ([`slot_block`]): skipped
    /// while a jump is under way (@0x69d2b-0x69d32) or the session refuses
    /// the nav keys, otherwise at most one stellar picked
    /// ([`FlightView::select_stellar`]).
    pub(super) fn slot_keys_frame(&mut self) {
        let polled = self.slot_block_polled();
        let down = self.slot_columns_down();
        let picked = self
            .slot_block
            .frame(|command, at| down.contains(&(command, at)), polled);
        if let Some(pick) = picked {
            self.select_stellar(pick);
        }
    }

    /// The slot block's commands read down, each with the column of each
    /// key holding it: its binding's or its fixed twin's.
    fn slot_columns_down(&self) -> Vec<(Command, Column)> {
        let presses = self.keys.down();
        self.commands_down()
            .filter(|&command| slot_block::is_block_command(command))
            .flat_map(|command| {
                presses
                    .iter()
                    .filter(move |press| self.keys.meanings(press).contains(&command))
                    .map(move |press| (command, self.column_of(press, command)))
            })
            .collect()
    }

    /// Which column the key of `input` holds `command` by: its fixed twin
    /// ([`CommandReader::is_twin`](crate::keymap::CommandReader::is_twin))
    /// or its binding.
    pub(super) fn column_of(&self, input: &Input, command: Command) -> Column {
        if self.keys.is_twin(input, command) {
            Column::Twin
        } else {
            Column::Bound
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
        let down = self.slot_columns_down();
        self.slot_block.queued(
            |command, at| down.contains(&(command, at)),
            self.slot_block_polled(),
        )
    }

    /// A press of Hyper Select ([`F::HyperSelect`]) or Hyperspace Mode
    /// ([`F::HyperspaceMode`]). The press's own frame shows the floating
    /// map: Hyperspace Mode's always (presses are taken only outside a
    /// jump), and Hyper Select's when it leaves the mode at 3, unless it
    /// found no system. A tap released before the next tick still shows
    /// it, as the frame's key poll would see it down.
    ///
    /// Each runs its key's routine with the key counted as down, its latch
    /// first cleared as the release before the press clears it: Hyper
    /// Select's always, Hyperspace Mode's unless Nav Off without Option is
    /// still down, when the latch holds and H only shows the map
    /// (@0x69b8e-0x69ba3).
    ///
    /// A flight that failed to start shows nothing here: its frame's nav is
    /// off, and its floating map has no drawing.
    pub(super) fn hyperspace_command(&mut self, pressed: F) {
        if pressed == F::HyperSelect {
            self.select_latch = false;
            self.select_frame(true);
            if matches!(self.frame.nav_mode, NavMode::Hyperspace(_)) {
                self.show_floating_map();
            }
        } else {
            self.press_nav_block(pressed);
            self.show_floating_map();
        }
    }

    /// A fresh press of Nav Off, with Option or not ([`F::NavOff`],
    /// [`F::ClearTarget`]), or Hyperspace Mode ([`F::HyperspaceMode`]):
    /// the OS sends one only after a release, which clears the block's
    /// latch unless the block's other keys still hold it (Hyperspace Mode
    /// for Nav Off; Nav Off without Option for Hyperspace Mode). Then the
    /// block runs with the key down ([`FlightView::nav_keys_frame`]).
    fn press_nav_block(&mut self, pressed: F) {
        let nav_off_pressed = matches!(pressed, F::NavOff | F::ClearTarget);
        let option = if nav_off_pressed {
            pressed == F::ClearTarget
        } else {
            self.reads_down(F::ClearTarget)
        };
        let nav_off_down = self.reads_down(F::NavOff) || self.reads_down(F::ClearTarget);
        let h_down = self.reads_down(F::HyperspaceMode);
        let others = if nav_off_pressed {
            h_down
        } else {
            nav_off_down && !option
        };
        let (nav_off, h) = (
            nav_off_pressed || nav_off_down,
            pressed == F::HyperspaceMode || h_down,
        );
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
    /// it again at the same tick. H and `\` held past a close's latch
    /// still show it, as the latched branches do (@0x69b8e-0x69ba3,
    /// @0x69cdd-0x69ce7).
    pub(super) fn floating_frame(&mut self) {
        let jumping = self.jump_under_way();
        let mode_3 = matches!(self.frame.nav_mode, NavMode::Hyperspace(_));
        let h = self.reads_down(F::HyperspaceMode) && !jumping && self.takes(PlayerCommand::Nav);
        let backslash =
            self.reads_down(F::HyperSelect) && mode_3 && self.takes(PlayerCommand::SelectSystem);
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
    /// flight reads is down (`_quickKeyCheckResult` and the modifiers,
    /// @0x70f4e-0x70fb5), a latched one held past a close included, as
    /// `_QuickKeyCheck` sets the result for any key it reads down
    /// (@0xa145-0xa14c), Zoom Out steps it out and Zoom In in. The eject,
    /// jettison and self-destruct keys held alone don't count:
    /// `_HandlePlayer` checks each only with Command down (@0x6fdc8,
    /// @0x6f2a5), so alone they hold no command and never set
    /// `_quickKeyCheckResult`, and Command itself holds the zoom back, as
    /// any modifier does ([`CommandReader::modifier_down`](crate::keymap::CommandReader::modifier_down)).
    /// One that changes the scale shows the map again and raises
    /// [`UiSound::FloatingMapZoomed`] (`_beepSnd[0]`, @0x7103a, @0x710bc);
    /// one at a limit does nothing.
    pub(super) fn zoom_floating(&mut self, pressed: F) {
        let holds = self.keys.modifier_down()
            || self.commands_down().any(|down| {
                self.slot_polled(down, true).unwrap_or(!matches!(
                    down,
                    Command::Flight(
                        F::SelfDestruct | F::Eject | F::JettisonCargo | F::JettisonNonMission
                    )
                ))
            });
        if !self.floating.is_up() || holds {
            return;
        }
        let moved = if pressed == F::ZoomOut {
            self.floating.zoom_out()
        } else {
            self.floating.zoom_in()
        };
        if moved {
            self.show_floating_map();
            self.ui_sounds.push(UiSound::FloatingMapZoomed);
        }
    }

    /// For a held command of the stellar slot block or the escort menu's
    /// groups, whether `_HandlePlayer` polls its key this frame and so
    /// finds it down; `None` for any other. The slot block polls its
    /// commands always but during a jump (@0x69d2b-0x69d32), the escort
    /// menu hiding 1-5 from it (@0xa106-0xa124) as flight's reader reads
    /// them as the groups then. With `escort_loop`, the escort menu's loop
    /// counts too: it polls 1-5 while the menu shows, during a jump too
    /// (@0x6df4a-0x6e186), setting `_quickKeyCheckResult`
    /// (@0xa18a-0xa191) but not the cursor's fade.
    pub(super) fn slot_polled(&self, command: Command, escort_loop: bool) -> Option<bool> {
        if let Command::EscortMenu(_) = command {
            return Some(escort_loop);
        }
        slot_block::is_block_command(command).then(|| !self.jump_under_way())
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
        let current = self.drawn().floating.current;
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

    /// A press of Target Select (`pressed`, [`F::TargetSelect`] or a
    /// variant): the next ship, the previous with Shift, each among the
    /// player's escorts with Option ([`FlightCommand::SelectTarget`]).
    pub(super) fn pick_target(&mut self, pressed: F) {
        self.select_target(match pressed {
            F::TargetEscort => TargetPick::NextEscort,
            F::TargetSelectBack => TargetPick::Previous,
            F::TargetEscortBack => TargetPick::PreviousEscort,
            _ => TargetPick::Next,
        });
    }

    /// A press of Closest Target (`pressed`): the nearest threat, or the
    /// nearest ship with Option ([`F::ClosestShip`],
    /// [`FlightCommand::SelectTarget`]).
    pub(super) fn pick_nearest(&mut self, pressed: F) {
        self.select_target(if pressed == F::ClosestShip {
            TargetPick::Nearest
        } else {
            TargetPick::NearestThreat
        });
    }

    /// A press of Select Secondary (`pressed`), backwards with Option
    /// ([`F::SelectSecondaryBack`], [`FlightCommand::SelectWeapon`]).
    /// Option is read at the press, the original reading it at the pass
    /// (@0x6acf6): the press is taken on the next step, before a frame is
    /// drawn.
    pub(super) fn select_weapon(&mut self, pressed: F) {
        let backwards = pressed == F::SelectSecondaryBack;
        self.send(FlightCommand::SelectWeapon { backwards });
    }
}
