//! [`FlightView`] as a [`Screen`].

use std::time::Duration;

use nova_sim::PlayerCommand;
use nova_sim::Steps;

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;
use crate::ui::catalog::{DescriptionGraphics, DescriptionSource};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{
    Diagnostic, DrawList, Input, Key, MouseButton, Screen, ScreenAction, Sound, ViewShape,
};

use super::{
    BOARD_KEY, FRAME_RATE_KEY, FlightView, HAIL_KEY, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY,
    JETTISON_KEY, JUMP_KEY, LAND_KEY, MAP_KEY, NEAREST_KEY, SELECT_KEY, TARGET_KEY, ZOOM_IN_KEY,
    ZOOM_IN_KEYPAD_KEY, ZOOM_OUT_KEY,
};

impl<
    C: CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks
        + DescriptionSource
        + DescriptionGraphics,
    S: FlightSim,
> Screen for FlightView<C, S>
{
    /// Never quits: Escape is the router's. While a jump is under way,
    /// the pre-jump stage included, M, L, P, Y and I are kept for the pass
    /// that reads them (the `faded_keys` module), and any other press
    /// goes on only when [`FlightView::taken_during_jump`] takes it; a
    /// click in space selects nothing then, as [`Session::select_stellar`](nova_sim::Session::select_stellar)
    /// refuses during a jump (@0x6a145, @0x6a5ef). Otherwise, and of the
    /// keys taken during a jump, each key's [`PlayerCommand`] is asked of
    /// the session ([`Session::takes`](nova_sim::Session::takes)) and a refused one does nothing,
    /// saying nothing: while the player's ship breaks up the map, landing
    /// and mission info (`_HandlePlayer`'s breakup-timer gates at 0x6a7b3,
    /// 0x6a8cb and 0x7044b), and once it has exploded every command (the
    /// dead branch, 0x6870d-0x6888c); and for
    /// [`TAKEOFF_GRACE_TICKS`](nova_sim::TAKEOFF_GRACE_TICKS) after a
    /// take-off the map and mission info (`_frameCounter` below 0,
    /// @0x6a7de, @0x7045e). The session refuses some taken commands
    /// itself, P with its beep among them. The flight keys are still
    /// tracked as held, so one held at arrival acts on the next tick, as
    /// the original reads them live, H, \`, `\` and J as their latches
    /// let them ([`FlightView::held_keys_frame`]; see the module docs), L
    /// and I too ([`FlightView::faded_keys_frame`]).
    fn input(&mut self, input: &Input) -> ScreenAction {
        self.take_input(input)
    }

    /// First catches the screen up with what the session has told since
    /// (see the module docs), map open or not. Then runs the simulation's
    /// steps for `dt` with the keys held ([`FlightView::run_steps`]),
    /// whose first act, steps or none, settles the asteroid field of a
    /// session started or loaded in flight, so the first frame drawn shows
    /// it (see `nova_sim::Session::step`), and
    /// advances the stellars' animations, each hypergate drawing the frame
    /// the session stepped it to ([`show_gate_frames`](super::show_gate_frames)). While the map is
    /// open nothing moves. The session flies on through the whole jump, its
    /// warp included: the jump's effect starts once the jump has begun,
    /// fades to white from the first step the session's `_hyperGamma` is
    /// above 0 ([`FlightFrame::hyper_gamma`](nova_sim::flight_frame::FlightFrame::hyper_gamma)), and the ship arrives on the step
    /// the session says the warp has ended ([`FlightFrame::warp_end_due`](nova_sim::flight_frame::FlightFrame::warp_end_due)),
    /// whose steps stop there, before the frame is drawn, or, on a
    /// double time second pass, on the next step, after the frame between
    /// its passes is drawn ([`FlightView::holding_between_passes`]). From then on the
    /// rest of the effect, the fade-in or the white flash, plays over the
    /// new system; that plays on with the map open too, as the original's
    /// display fade does. A jump the session gave up instead is settled at
    /// the top of the next tick ([`FlightView::settle_given_up_jump`]). The
    /// escort menu times out on every step the map is shut.
    ///
    /// Each step runs the session's advance pass, then has the NPCs answer
    /// the fight's hits and the persons say their quotes, so a ship hit in
    /// its gate is drawn on the frame of the hit.
    ///
    /// Then, unless the map is open, the HUD's timer moves on, refreshing
    /// the radar when it is due ([`FlightView::advance_hud`]), still
    /// before the frame is drawn.
    ///
    /// The keys kept through a jump are read at the head of the keys'
    /// frame and right after the steps (the `faded_keys` module): a key
    /// read at the head that opens something ends the tick, as no step
    /// runs under it.
    fn tick(&mut self, dt: Duration) {
        self.tick_then(dt);
    }

    /// The scene and its HUD, then the fades. With the course map open,
    /// the map's dialog over the paused scene as it would be drawn shut,
    /// less the cursor, which the map hides, its 1024 x 768
    /// frame centred in a widened view ([`FlightView::map_offset`]); in
    /// port, where the bar opens it, the dialog alone, where the classic
    /// spaceport has it, for the app to draw over the spaceport. These are
    /// [`FlightView::draw_under_dialog_map`] and
    /// [`FlightView::draw_dialog_map`], which the app draws apart when a
    /// mission dialog goes between them.
    fn draw(&self, list: &mut DrawList) {
        if self.draw_movie(list, |list| {
            if self.draw_scene(list) {
                self.draw_fades(list);
            }
        }) {
            return;
        }
        if self.map_open {
            self.draw_under_dialog_map(list);
            self.draw_dialog_map(list);
            return;
        }
        if self.draw_scene(list) {
            self.draw_fades(list);
        }
    }

    /// Abandons any gesture on the course map, and forgets where the
    /// pointer was: flight stops getting pointer moves while it is hidden
    /// or covered, so the cursor sprite is not drawn (and nothing is
    /// hovered) at that stale point until the next move, which the app
    /// sends when flight shows again.
    fn cancel_pointer(&mut self) {
        self.map.cancel_pointer();
        self.cursor.left();
    }

    /// Lets go of every flight key: the ship stops thrusting and turning,
    /// and coasts. [`MAP_KEY`] counts as let go too, so a key-down the
    /// course map doesn't take no longer closes it, and a map waiting for
    /// M's release closes ([`FlightView::close_map`]): ours can't read the
    /// keys unfocused, as `_KeyCheck` reads `GetKeys` live
    /// (@0xa19b-0xa1dc), so losing them counts as the release. So it does
    /// for a mouse button, which `_Button` reads live (@0x3803b). What the
    /// wait queued goes with them (the `map_wait` module).
    fn release_keys(&mut self) {
        self.let_go_of_input();
        self.map.release_keys();
        self.map_key_held = false;
        self.wait_keys.clear();
        if self.map_closing {
            self.close_map();
        }
    }

    /// The frame-rate readout starts afresh, "Calculating FPS..." on the
    /// next frame while it is shown, as `_SwitchedIn` zeroes its counters
    /// (@0x513cc-0x513dc).
    fn focus_gained(&mut self) {
        self.switched_in();
    }

    /// The flight halts both warp-ups, as `_SwitchedOut` does
    /// (@0x4f3a3-0x4f3e0; `FlightCommand::SwitchedOut`). A flight that
    /// failed to start refuses it, as any command.
    fn focus_lost(&mut self) {
        self.send(nova_sim::flight_frame::FlightCommand::SwitchedOut);
    }

    /// The session's sounds (thrust, landing, taking off and jumping),
    /// then the interface's (the escort menu's, the floating map's zooms,
    /// the player info refused, the course map's beeps), then the fight's.
    /// The course map has no buttons that sound.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.drained_sounds()
    }

    /// Flight hides the system's pointer over the window for its cursor
    /// sprite (`_HandleMouse` @0x38007-0x38017 hides it inside the
    /// window), unless the map is open, the flight never started or the
    /// cursor's sheet could not be read.
    fn hides_pointer(&self) -> bool {
        if self.playing() {
            return self.movie_hides_pointer();
        }
        !self.map_open && self.failure.is_none() && self.looks.cursor_sheet().is_some()
    }

    /// What could not be read of the looks, then the session's
    /// diagnostics, each once.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.drained_diagnostics()
    }

    /// Flight widens to the window's aspect ratio
    /// ([`ViewShape::Widened`]), showing more space.
    fn view_shape(&self) -> ViewShape {
        ViewShape::Widened
    }

    /// Takes the widened size as the view's ([`FlightView::set_view_size`]).
    fn set_widened_size(&mut self, size: (f32, f32)) {
        self.set_view_size(size);
    }
}

impl<
    C: CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks
        + DescriptionSource
        + DescriptionGraphics,
    S: FlightSim,
> FlightView<C, S>
{
    /// [`Screen::tick`]'s work. The kept frame needs no fill at its end:
    /// the catch-up, the steps and each command fill it where they change
    /// the flight.
    fn tick_then(&mut self, dt: Duration) {
        if self.playing() {
            self.movie_tick(dt);
            return;
        }
        self.elapse_frame_rate(dt);
        self.catch_up();
        self.fade_frame(dt);
        if self.map_open {
            // The map's own frame: its borders' null events, none while
            // it waits for M's release (`_BusyIdle` runs none).
            if !self.map_closing {
                self.map.tick(dt);
            }
            return;
        }
        if self.death_frame(dt) {
            // `_PlayGame` goes on counting and showing while the player
            // is dead, the fade to black included.
            self.frame_rate_frame();
            return;
        }
        self.step_stellars(dt);
        if self.faded_keys_frame() {
            return;
        }
        self.held_keys_frame();
        self.floating_frame();
        let Steps { steps, alpha } = self.clock.advance(dt);
        let keyed = self.cursor_keyed();
        let jumping = self.jump_under_way();
        self.run_steps(steps);
        self.settle_faded_keys(jumping);
        // After the lines the steps relayed, as `_PlayGame`'s readout
        // comes after `_DoPlayGameWork`.
        self.frame_rate_frame();
        self.update_escort_menu(steps);
        self.alpha = alpha;
        self.step_cursor(steps, keyed);
        self.start_jump_effect();
        if let Some(effect) = &mut self.jump {
            effect.advance(dt);
        }
        self.arrival_frame();
        self.advance_hud();
    }

    /// `press`, if a jump under way lets it through and the flight takes
    /// its command.
    pub(super) fn press_taken(&self, press: Option<Key>, jumping: bool) -> Option<Key> {
        let press = if jumping {
            let option = self.held.contains(&Key::Alt);
            let menu_open = self.escort_menu.is_open();
            press.filter(|&key| Self::taken_during_jump(key, option, menu_open))
        } else {
            press
        };
        press.filter(|&key| {
            self.command_of(key)
                .is_none_or(|command| self.takes(command))
        })
    }

    /// [`Screen::input`]'s work. The kept frame needs no fill at its end:
    /// each command the input sends fills it ([`FlightView::send`]).
    fn take_input(&mut self, input: &Input) -> ScreenAction {
        if self.playing() {
            self.movie_input(input);
            return ScreenAction::None;
        }
        self.cursor_input(input);
        let press = match *input {
            Input::Key {
                key,
                pressed: true,
                repeat: false,
            } => Some(key),
            _ => None,
        };
        let jumping = self.jump_under_way();
        if jumping && self.stop_during_jump(press, input) {
            return ScreenAction::None;
        }
        if self.map_open {
            if self.plain_map_open() {
                self.track_unlatched(input);
            }
            // M types into Find while it is open.
            if press == Some(MAP_KEY) && !self.map.finding() {
                // M's own close reads M held, so it waits for the release.
                self.map_key_held = true;
                self.close_map();
            } else {
                self.map_input(input);
            }
            return ScreenAction::None;
        }
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        } = *input
            && self.takes(PlayerCommand::Nav)
            && !self.click_escort_menu(at)
            && !self.click_floating_map(at)
        {
            self.click_in_space(at);
        }
        match self.press_taken(press, jumping) {
            Some(LAND_KEY) => {
                self.press_land();
                // The hypergate's map takes its release, as M's does.
                if self.map_open {
                    return ScreenAction::None;
                }
            }
            Some(BOARD_KEY) => {
                // A fresh press follows a release, which clears the latch.
                self.board_latch = false;
                self.board_frame(true);
                if self.pending_boarding {
                    return ScreenAction::None;
                }
            }
            Some(HAIL_KEY) => {
                self.hail();
                if self.hail_opened() {
                    return ScreenAction::None;
                }
            }
            Some(MAP_KEY) => {
                self.open_map_by_key();
                return ScreenAction::None;
            }
            Some(JUMP_KEY) => {
                // A fresh press follows a release, which clears the latch.
                self.jump_latch = false;
                self.jump_frame(true);
            }
            Some(JETTISON_KEY) => self.jettison(),
            Some(MISSION_INFO_KEY) => {
                self.press_mission_info();
                if self.pending_mission_info.is_some() {
                    return ScreenAction::None;
                }
            }
            Some(FRAME_RATE_KEY) => self.toggle_frame_rate(),
            Some(PLAYER_INFO_KEY) => {
                let action = self.ask_player_info();
                if !self.pending_player_info {
                    self.track_held(input);
                }
                return action;
            }
            Some(key @ (HYPER_SELECT_KEY | HYPERSPACE_MODE_KEY)) => self.hyperspace_key(key),
            Some(key @ (ZOOM_OUT_KEY | ZOOM_IN_KEY | ZOOM_IN_KEYPAD_KEY)) => {
                self.zoom_floating_map(key);
            }
            Some(TARGET_KEY) => self.press_target_key(),
            Some(key) if self.escort_key(key) => {}
            Some(NEAREST_KEY) => self.press_nearest_key(),
            Some(SELECT_KEY) => self.press_weapon_select(),
            Some(key) => self.nav_key(key),
            None => {}
        }
        self.track_held(input);
        ScreenAction::None
    }

    /// Draws the scene and its HUD, the fades left out: whether there was
    /// a scene to draw, or only why flight could not start. The sprite
    /// world goes first, in `_SetupSpriteWorld`'s order
    /// ([`FlightView::draw_world`]), the brackets, the cursor, the icons,
    /// the floating map and the escort menu among it, then the overlay
    /// over it: the title, the help and the status bar.
    fn draw_scene(&self, list: &mut DrawList) -> bool {
        #[cfg(test)]
        self.check_frame_kept();
        self.draw_flight(list, || self.failure.clone().unwrap_or_default())
    }

    /// What goes under the dialogs a map opened from and under the map:
    /// the paused scene and its HUD, or nothing in port, where the app
    /// draws the spaceport instead. The original leaves the last frame
    /// drawn in `_gWind` behind each dialog and behind the map.
    pub fn draw_under_dialog_map(&self, list: &mut DrawList) {
        if !self.frame.docked() {
            self.draw_scene(list);
        }
    }

    /// What goes over the dialogs a map opened from: the open map, whose
    /// window opens in front of theirs (`_DoSystemMap` @0x125b7-0x125c9),
    /// then the fades.
    pub fn draw_dialog_map(&self, list: &mut DrawList) {
        self.draw_open_map(list);
        self.draw_fades(list);
    }
}
