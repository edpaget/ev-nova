//! [`FlightView`] as a [`Screen`].

use std::time::Duration;

use nova_sim::PlayerCommand;
use nova_sim::{
    AsteroidCatalog, CombatCatalog, CommCatalog, PilotCatalog, Session, Steps, TargetPick,
    TrafficCatalog,
};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::escorts::EscortMenuLooks;
use crate::flight::hud::{self};
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{
    Color, Diagnostic, DrawList, Input, Key, MouseButton, Screen, ScreenAction, Sound, ViewShape,
};

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
> Screen for FlightView<C, crate::flight::SessionFlight<C>>
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
        let jumping = self.jump_under_way();
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
                // A fresh press follows a release, which clears the latch.
                self.board_latch = false;
                self.board_frame(true);
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
                // Option is read at the press, the original reading it at
                // the pass (@0x6acf6): the press is taken on the next
                // step, before a frame is drawn.
                let backwards = self.held.contains(&Key::Alt);
                if let Ok(session) = &mut self.sim.session {
                    session.press_weapon_select(backwards);
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
    /// open nothing moves. The session flies on through the whole jump, its
    /// warp included: the jump's effect starts once the jump has begun,
    /// fades to white from the first step the session's `_hyperGamma` is
    /// above 0 ([`Session::hyper_gamma`]), and the ship arrives on the step
    /// the session says the warp has ended ([`Session::warp_end_due`]),
    /// whose steps stop there, before the frame is drawn. From then on the
    /// rest of the effect, the fade-in or the white flash, plays over the
    /// new system; that plays on with the map open too, as the original's
    /// display fade does. A jump the session gave up instead is settled at
    /// the top of the next tick ([`FlightView::settle_given_up_jump`]). The
    /// escort menu times out on every step the map is shut.
    ///
    /// Each step runs the session's tick, the traffic's (the assistance's
    /// within it) and the fight's, then has the NPCs answer the fight's hits
    /// ([`Session::answer_strikes`]) and the persons say their quotes, so
    /// a ship hit in its gate is drawn on the frame of the hit.
    ///
    /// Then, unless the map is open, the HUD's timer moves on, refreshing
    /// the radar when it is due ([`FlightView::advance_hud`]), still
    /// before the frame is drawn.
    fn tick(&mut self, dt: Duration) {
        if let Ok(session) = &mut self.sim.session {
            session.settle_asteroid_field(&mut self.sim.chance);
        }
        self.catch_up();
        if let Some(effect) = &mut self.fade_in {
            effect.advance(dt);
            if effect.done() {
                self.fade_in = None;
            }
        }
        self.settle_given_up_jump();
        if self.map_open {
            // The map's own frame: its borders' null events.
            self.map.tick(dt);
            return;
        }
        if self.hold_expired() {
            self.advance_death_fade(dt);
            return;
        }
        self.elapsed += dt;
        self.held_keys_frame();
        self.floating_frame();
        let Steps { steps, alpha } = self.clock.advance(dt);
        self.run_steps(steps, |session| {
            !session.warp_end_due() && !session.hold_expired()
        });
        self.update_escort_menu();
        self.alpha = alpha;
        self.step_cursor(steps);
        self.start_jump_effect();
        if let Some(effect) = &mut self.jump {
            effect.advance(dt);
        }
        if self.sim.session.as_ref().is_ok_and(Session::warp_end_due) {
            self.arrive();
            self.fade_in = self
                .jump
                .take()
                .map(|mut effect| {
                    effect.arrive();
                    effect
                })
                .filter(|effect| !effect.done());
        }
        self.advance_hud();
    }

    /// The scene and its HUD, then the fades. With the course map open,
    /// the map's dialog over the paused scene as it would be drawn shut,
    /// less the cursor, which the map hides, its 1024 x 768
    /// frame centred in a widened view ([`FlightView::map_offset`]); in
    /// port, where the bar opens it, the dialog alone, where the classic
    /// spaceport has it, for the app to draw over the spaceport.
    fn draw(&self, list: &mut DrawList) {
        if self.map_open {
            if !self.frame_now().docked {
                self.draw_scene(list);
            }
            self.draw_open_map(list);
            self.draw_fades(list);
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
        self.drained_sounds()
    }

    /// Flight hides the system's pointer over the window for its cursor
    /// sprite (`_HandleMouse` @0x38007-0x38017 hides it inside the
    /// window), unless the map is open, the flight never started or the
    /// cursor's sheet could not be read.
    fn hides_pointer(&self) -> bool {
        !self.map_open && self.sim.session.is_ok() && self.looks.cursor_sheet().is_some()
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
> FlightView<C, crate::flight::SessionFlight<C>>
{
    /// Draws the scene and its HUD, the fades left out: whether there was
    /// a scene to draw, or only why flight could not start. The sprite
    /// world goes first, in `_SetupSpriteWorld`'s order
    /// ([`FlightView::draw_world`]), the brackets, the cursor, the icons,
    /// the floating map and the escort menu among it, then the overlay
    /// over it: the title, the help and the status bar.
    fn draw_scene(&self, list: &mut DrawList) -> bool {
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
            return false;
        };
        let camera = self.camera();
        list.world(|list| self.draw_world(list, scene, &camera));
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
            Ok(bar) => self.draw_bar(list, bar, self.view.0),
            Err(reason) => hud::draw_unavailable(list, reason, self.view.0),
        }
        true
    }
}
