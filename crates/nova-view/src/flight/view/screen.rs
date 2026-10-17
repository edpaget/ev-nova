//! [`FlightView`] as a [`Screen`].

use std::time::Duration;

use nova_sim::PlayerCommand;
use nova_sim::{
    AsteroidCatalog, CombatCatalog, CommCatalog, PilotCatalog, Session, Steps, TargetPick,
    TrafficCatalog,
};

use crate::flight::alert;
use crate::flight::asteroids::{self};
use crate::flight::boxes::{self};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::hud::{self};
use crate::flight::jump::JumpPhase;
use crate::flight::weapons::{self};
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{self};
use crate::system::starfield;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{Color, Diagnostic, DrawList, Input, Key, MouseButton, Screen, ScreenAction, Sound};

use super::{
    BOARD_KEY, FlightView, HAIL_KEY, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY, JETTISON_KEY, JUMP_KEY,
    LAND_KEY, MAP_KEY, NEAREST_KEY, SELECT_KEY, TARGET_KEY, TITLE, TITLE_SIZE, ZOOM_IN_KEY,
    ZOOM_IN_KEYPAD_KEY, ZOOM_OUT_KEY,
};

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + AsteroidCatalog
        + CommCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks,
> Screen for FlightView<C>
{
    /// Never quits: Escape is the router's. While a jump is under way,
    /// the pre-jump stage included, P is refused with its beep, and a press
    /// goes on only when [`FlightView::taken_during_jump`] takes it; a
    /// click in space selects nothing then, as [`Session::select_stellar`]
    /// refuses during a jump (@0x6a145, @0x6a5ef). Otherwise, and of the
    /// keys taken during a jump, each key's [`PlayerCommand`] is asked of
    /// the session ([`Session::takes`]) and a refused one does nothing,
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
    /// let them ([`FlightView::held_keys_frame`]; see the module docs).
    fn input(&mut self, input: &Input) -> ScreenAction {
        self.cursor_input(input);
        let press = match *input {
            Input::Key {
                key,
                pressed: true,
                repeat: false,
            } => Some(key),
            _ => None,
        };
        let jumping = self.jump.is_some() || self.preparing_jump();
        if jumping && press == Some(PLAYER_INFO_KEY) {
            return self.refuse_player_info();
        }
        if self.map_open {
            // M types into Find while it is open.
            if press == Some(MAP_KEY) && !self.map.finding() {
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
            && !self.click_floating_map(at)
        {
            self.click_in_space(at);
        }
        let press = if jumping {
            let option = self.held.contains(&Key::Alt);
            let menu_open = self.escort_menu.is_open();
            press.filter(|&key| Self::taken_during_jump(key, option, menu_open))
        } else {
            press
        };
        let press = press.filter(|&key| {
            self.command_of(key)
                .is_none_or(|command| self.takes(command))
        });
        match press {
            Some(LAND_KEY) => self.land(),
            Some(BOARD_KEY) => {
                self.board();
                if self.pending_boarding {
                    return ScreenAction::None;
                }
            }
            Some(HAIL_KEY) => {
                self.hail();
                if self.pending_hail.is_some() {
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
            Some(MISSION_INFO_KEY) => self.ask_mission_info(),
            Some(PLAYER_INFO_KEY) => return self.ask_player_info(),
            Some(key @ (HYPER_SELECT_KEY | HYPERSPACE_MODE_KEY)) => self.hyperspace_key(key),
            Some(key @ (ZOOM_OUT_KEY | ZOOM_IN_KEY | ZOOM_IN_KEYPAD_KEY)) => {
                self.zoom_floating_map(key);
            }
            Some(TARGET_KEY) => self.select_target(
                match (
                    self.held.contains(&Key::Shift),
                    self.held.contains(&Key::Alt),
                ) {
                    (false, false) => TargetPick::Next,
                    (false, true) => TargetPick::NextEscort,
                    (true, false) => TargetPick::Previous,
                    (true, true) => TargetPick::PreviousEscort,
                },
            ),
            Some(key) if self.escort_key(key) => {}
            Some(NEAREST_KEY) => self.select_target(if self.held.contains(&Key::Alt) {
                TargetPick::Nearest
            } else {
                TargetPick::NearestThreat
            }),
            Some(SELECT_KEY) => {
                let backwards = self.held.contains(&Key::Alt);
                if let Ok(session) = &mut self.session {
                    session.select_secondary(backwards);
                }
            }
            Some(key) => self.nav_key(key),
            None => {}
        }
        self.track_held(input);
        ScreenAction::None
    }

    /// First settles the asteroid field of a session started or loaded in
    /// flight ([`Session::settle_asteroid_field`]), so the first frame
    /// drawn shows it, and catches the screen up with what the session
    /// has told since (see the module docs), map open or not. Then runs the simulation's
    /// steps for `dt` with the keys held ([`FlightView::run_steps`]), and
    /// advances the stellars' animations, each hypergate drawing the frame
    /// the session stepped it to ([`show_gate_frames`](super::show_gate_frames)). While the map is
    /// open nothing moves. During the pre-jump stage the session flies on,
    /// and the streak starts on the step the session begins the jump,
    /// which is the last step run. While the stars streak and the old
    /// system fades out the jump's effect moves, and the session runs its
    /// steps only while its warp is in its lead-in
    /// ([`Session::in_warp_lead_in`]): the ship stays where it is, but the
    /// traffic flies on and the scans are taken, as the original still
    /// scans a warping player then. The ship arrives when the effect says,
    /// after the last step. From then on the session flies again while the
    /// rest of the effect, the fade-in or the white flash, plays over it;
    /// that plays on with the map open too, as the original's display fade
    /// does. The escort menu times out on every step the map is shut, the
    /// streak's included.
    ///
    /// Each step runs the session's tick, the traffic's, the assistance's
    /// and the fight's, then has the NPCs answer the fight's hits
    /// ([`Session::answer_strikes`]) and the persons say their quotes, so
    /// a ship hit in its gate is drawn on the frame of the hit.
    ///
    /// Then, unless the map is open, the HUD's timer moves on, refreshing
    /// the radar when it is due ([`FlightView::advance_hud`]), still
    /// before the frame is drawn.
    fn tick(&mut self, dt: Duration) {
        if let Ok(session) = &mut self.session {
            session.settle_asteroid_field(&mut self.chance);
        }
        self.catch_up();
        if let Some(effect) = &mut self.fade_in {
            effect.advance(dt);
            if effect.done() {
                self.fade_in = None;
            }
        }
        if self.map_open {
            return;
        }
        if self.hold_expired() {
            self.advance_death_fade(dt);
            return;
        }
        self.elapsed += dt;
        self.held_keys_frame();
        self.floating_frame();
        if self.jump.is_some() {
            let leading_in = self.session.as_ref().is_ok_and(Session::in_warp_lead_in);
            if leading_in {
                let Steps { steps, alpha } = self.clock.advance(dt);
                self.run_steps(steps, Session::in_warp_lead_in);
                self.alpha = alpha;
            }
            if let Some(effect) = &mut self.jump
                && effect.advance(dt)
            {
                self.arrive();
                self.fade_in = self.jump.take().filter(|effect| !effect.done());
            }
            self.update_escort_menu();
            self.advance_hud();
            return;
        }
        let Steps { steps, alpha } = self.clock.advance(dt);
        self.run_steps(steps, |session| {
            session.jumping().is_none() && !session.hold_expired()
        });
        self.update_escort_menu();
        self.alpha = alpha;
        self.step_cursor(steps);
        self.start_streak_if_jumping();
        self.advance_hud();
    }

    fn draw(&self, list: &mut DrawList) {
        if self.map_open {
            self.map.draw(list);
            self.draw_fades(list);
            return;
        }
        let Some(scene) = &self.scene else {
            let reason = self.session().err().unwrap_or_default();
            list.text(
                format!("Cannot start flight: {reason}"),
                TITLE,
                TITLE_SIZE,
                None,
                Color::ERROR,
            );
            self.draw_help(list);
            return;
        };
        let camera = self.camera();
        match self.jump.map(|effect| (effect, effect.phase())) {
            Some((effect, JumpPhase::Streak(_) | JumpPhase::FadeOut(_))) => {
                starfield::draw_streaked(list, &camera, effect.direction(), effect.streak_length());
            }
            _ => starfield::draw(list, &camera),
        }
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        if let Some(message) = self.message() {
            hud::draw_message(list, message, metrics);
        }
        scene::draw_stellars(list, scene, &camera, self.elapsed);
        asteroids::draw_asteroids(list, &self.shown_asteroids(&camera), &self.looks);
        boxes::draw_boxes(list, &self.shown_boxes(&camera), &self.looks);
        let beams = self.shown_beams(&camera);
        weapons::draw_beams(list, &beams, &self.looks, true);
        self.draw_ships(list, &camera);
        weapons::draw_shots(list, &self.shown_shots(&camera), &self.looks);
        weapons::draw_beams(list, &beams, &self.looks, false);
        self.effects.draw(list, &camera, &self.looks);
        self.draw_brackets(list, scene, &camera);
        self.draw_red_alert(list);
        if self.session.as_ref().is_ok_and(Session::double_time) {
            alert::draw_double_time(list);
        }
        // `_PostDrawCallback`'s order: the floating map over the sprites
        // and icons, under the escort menu (@0x38b95-0x38c72).
        self.draw_floating_map(list);
        self.escort_menu
            .draw(list, &self.escort_rows(), self.escort_colors, metrics);
        if self.overlays.title {
            list.text(
                format!("{} (sÿst {})", scene.name(), scene.id().0),
                TITLE,
                TITLE_SIZE,
                None,
                Color::WHITE,
            );
        }
        self.draw_help(list);
        match &self.status_bar {
            Ok(bar) => self.draw_bar(list, bar),
            Err(reason) => hud::draw_unavailable(list, reason),
        }
        self.cursor.draw(list, &self.looks);
        self.draw_fades(list);
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
    /// course map doesn't take no longer closes it.
    fn release_keys(&mut self) {
        self.held.clear();
        self.map.release_keys();
        self.map_key_held = false;
    }

    /// The session's sounds (thrust, landing, taking off and jumping),
    /// then the interface's (the escort menu's, the floating map's zooms,
    /// the player info refused, the course map's beeps), then the fight's.
    /// The course map has no buttons that sound.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds: Vec<Sound> = self
            .session
            .as_mut()
            .map(|session| session.take_sounds().into_iter().map(Sound::Sim).collect())
            .unwrap_or_default();
        sounds.extend(self.ui_sounds.drain(..).map(Sound::Ui));
        sounds.extend(self.effects.take_sounds().into_iter().map(Sound::Combat));
        sounds
    }

    /// Flight hides the system's pointer over the window for its cursor
    /// sprite (`_HandleMouse` @0x38007-0x38017 hides it inside the
    /// window), unless the map is open, the flight never started or the
    /// cursor's sheet could not be read.
    fn hides_pointer(&self) -> bool {
        !self.map_open && self.session.is_ok() && self.looks.cursor_sheet().is_some()
    }

    /// What could not be read of the looks, then the session's
    /// diagnostics, each once.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diagnostics = std::mem::take(&mut self.unread_looks);
        if let Ok(session) = &mut self.session {
            let sim = session.take_diagnostics().into_iter();
            diagnostics.extend(sim.map(Diagnostic::Sim));
        }
        diagnostics
    }
}
