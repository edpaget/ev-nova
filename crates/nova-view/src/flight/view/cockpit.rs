//! [`FlightView`]'s cockpit: shown positions, the cursor, drawing ships,
//! controls and keys, the floating map, escorts, the combat display and the
//! status bar.

use nova_sim::Drawn as ShipDrawn;
use nova_sim::PlayerCommand;
use nova_sim::{
    ClassRow, Condition, Controls, EscortCommand, Face, NavMode, Npc, Reserves, Session, ShipRef,
    ShipState, StellarPick, SystemId, TargetPick, Turn, Vec2, glow_level, lights_level,
};

use crate::draw::{crossed_box, lights_tint};
use crate::flight::alert;
use crate::flight::asteroids::AsteroidShown;
use crate::flight::boxes::BoxShown;
use crate::flight::brackets;
use crate::flight::effects::Effects;
use crate::flight::escorts::{
    EscortMenu, GROUP_KEYS, MENU_KEY, NO_ESCORTS, Toggled, escort_command_message,
};
use crate::flight::flash::ShieldFlash;
use crate::flight::floating::{self, Hit};
use crate::flight::hud::{self, HudState, STATUS_BAR_WIDTH, StatusBar};
use crate::flight::murk::{self, murk_tint, sprite_murk};
use crate::flight::sprite::rotation_frame;
use crate::flight::target::{self};
use crate::flight::weapon_glow::npc_glow_level;
use crate::flight::weapons::{self, BeamShown, ShotShown};
use nova_sim::sprite_layer::SpriteLayer;

use super::sprites::{Sprite, sprite_stack};
use crate::galaxy::MapMarks;
use crate::galaxy::map::MapPicture;
use crate::system::camera::{Camera, VIEW_SIZE};
use crate::system::scene::{PLACEHOLDER, PLACEHOLDER_SIZE, SystemScene};
use crate::time::{sixtieths, ticks};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::view_shape::classic_offset;
use crate::{Color, DrawList, ImageKey, Input, Key, Point, UiSound};

use super::{
    AFTERBURNER_KEY, BOARD_KEY, DrawnNow, EJECT_KEY, ESCORT_ATTACK_KEY, ESCORT_DEFEND_KEY,
    ESCORT_HOLD_KEY, ESCORT_RECALL_KEY, ESCORT_TICKS, FACE_TARGET_KEY, FLIGHT_KEYS, FlightView,
    HAIL_KEY, HELP, HELP_AT, HYPER_SELECT_KEY, HYPERSPACE_MODE_KEY, JETTISON_KEY, JUMP_KEY,
    LAND_KEY, MAP_KEY, MENU_CLOSE_KEY, MESSAGE_GAP, NAV_OFF_KEY, NEAREST_KEY, NEAREST_STELLAR_KEY,
    OVERLAY_SIZE, SELECT_KEY, STELLAR_SLOT_KEYS, TARGET_KEY, ViewShips, ZOOM_IN_KEY,
    ZOOM_IN_KEYPAD_KEY, ZOOM_OUT_KEY, cloak_alpha, drawn_size, npc_tint, shown_heading,
    shown_position,
};

impl<C> FlightView<C> {
    /// The session's system, as laid out, if the session started.
    #[must_use]
    pub fn scene(&self) -> Option<&SystemScene> {
        self.scene.as_ref()
    }

    /// The HUD's status bar, or why it cannot be shown.
    pub fn status_bar(&self) -> Result<&StatusBar, &str> {
        self.status_bar.as_ref().map_err(String::as_str)
    }

    /// How far the display is between the last two steps, in `[0, 1)`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Where the ship is drawn: `alpha` of the way from where it was a step
    /// ago to where it is.
    #[must_use]
    pub fn shown_position(&self) -> Point {
        shown_position(&self.previous, &self.current(), self.alpha)
    }

    /// Which way the ship is drawn facing: `alpha` of the way, the short way
    /// round, from its heading a step ago to its heading now.
    #[must_use]
    pub fn shown_heading(&self) -> f32 {
        shown_heading(&self.previous, &self.current(), self.alpha)
    }

    /// Where `npc` is drawn and which way it faces: `alpha` of the way from
    /// how it was a step ago, as the player's ship; where it is, when it
    /// was not there a step ago.
    pub(super) fn shown_npc(&self, npc: &Npc) -> (Point, f32) {
        let from = self.npc_previous.get(&npc.id).unwrap_or(&npc.state);
        (
            shown_position(from, &npc.state, self.alpha),
            shown_heading(from, &npc.state, self.alpha),
        )
    }

    /// The NPCs, each with where it is drawn and which way it faces.
    pub(super) fn shown_npcs(&self) -> Vec<(&Npc, Point, f32)> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .npcs()
            .iter()
            .map(|npc| {
                let (at, heading) = self.shown_npc(npc);
                (npc, at, heading)
            })
            .collect()
    }

    /// Runs the cursor's `steps` flight steps (see the `cursor` module),
    /// hovering as the pointer is over something targetable now
    /// ([`FlightView::hovering`]), once the session's steps have run and
    /// the ships are where the frame draws them. A key pressed since the
    /// last step, or a flight key held, fades it. While the jump's streak
    /// and fade-out hold flight no step runs, and the cursor holds, which
    /// no player sees under the streak.
    pub(super) fn step_cursor(&mut self, steps: u32) {
        if steps == 0 {
            return;
        }
        let frames = self.looks.cursor_sheet().map(|sheet| sheet.frames);
        let hovering = self.hovering();
        let held = !self.held.is_empty();
        for _ in 0..steps {
            self.cursor.step(hovering, frames, held);
        }
    }

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

    /// What `input` tells the cursor: a pointer move or button press shows
    /// it where the pointer is, and the pointer leaving the window hides
    /// it; a key pressed fades it at the next flight step (none runs while
    /// the map is open, and the M that opened it has faded it already).
    pub(super) fn cursor_input(&mut self, input: &Input) {
        match *input {
            Input::PointerMoved(at)
            | Input::PointerButton {
                pressed: true, at, ..
            } => {
                self.cursor.pointer(at);
            }
            Input::PointerLeft => self.cursor.left(),
            Input::Key { pressed: true, .. } => self.cursor.key(),
            _ => {}
        }
    }

    /// Whether the pointer, while the cursor shows, is over something the
    /// player can target ([`Session::hover`]), the ships boxed where they
    /// are drawn ([`ViewShips`]).
    fn hovering(&self) -> bool {
        let (Ok(session), Some(at)) = (&self.session, self.cursor.shown_at()) else {
            return false;
        };
        let world = self.camera().screen_to_world(at);
        let ships = ViewShips {
            view: self,
            session,
        };
        session.hover(Vec2::new(world.x, world.y), &ships).is_some()
    }

    /// Draws the ships and the shots, bottom first, in their layers as
    /// the session stacks them ([`sprites`](super::sprites)): the ships as
    /// [`Session::draw_layers`] gives them (see [`nova_sim::ship_layer`]),
    /// each layer in the order the session's rules overlap them, and the
    /// shots among them by their layers ([`nova_sim::sprite_layer`]).
    /// Without a session it draws the player alone. The order and each
    /// cloak's level are read from the session as the frame is drawn, so
    /// nothing waits past it.
    pub(super) fn draw_sprites(&self, list: &mut DrawList, camera: &Camera) {
        let player = camera.world_to_screen(self.shown_position());
        let Ok(session) = &self.session else {
            self.draw_ship(list, player, ShipDrawn::Full);
            return;
        };
        let npcs = self.shown_npcs();
        let fade = session
            .rules()
            .source_for(nova_sim::RuleKey::TranslucentShotFade);
        let stacked = sprite_stack(session.draw_layers(), self.shown_shots(camera));
        for sprite in stacked {
            match sprite {
                Sprite::Ship(ShipRef::Player) => {
                    self.draw_ship(list, player, session.player_fate().row().drawn);
                }
                Sprite::Ship(ShipRef::Npc(id)) => {
                    if let Some(&(npc, at, heading)) = npcs.iter().find(|(npc, ..)| npc.id == id) {
                        self.draw_npc(list, session, npc, camera.world_to_screen(at), heading);
                    }
                }
                Sprite::Shot(shot) => weapons::draw_shot(list, &shot, &self.looks, fade),
            }
        }
    }

    /// Draws `npc` at `at` facing `heading` with its own ship's sprite, or a
    /// crossed box when its sheet cannot be read, at the alpha its cloak
    /// leaves ([`cloak_alpha`]); not at all when its cloak hides it. One
    /// waiting at the gate it comes out of is hidden, then fades in over
    /// its last ticks there ([`npc_tint`]). Its weapon glow is drawn over
    /// it while lit ([`weapon_glow`](crate::flight::weapon_glow)), held
    /// below its murk ([`npc_glow_level`]) and murked.
    fn draw_npc(&self, list: &mut DrawList, session: &Session, npc: &Npc, at: Point, heading: f32) {
        let Some(cloak) = cloak_alpha(session.cloak_shown(npc)) else {
            return;
        };
        let Some(tint) = npc_tint(npc) else {
            return;
        };
        let alpha = (u16::from(tint.a) * u16::from(cloak) + 127) / 255;
        let tint = Color {
            a: u8::try_from(alpha).unwrap_or(u8::MAX),
            ..tint
        };
        match self.npc_sheets.get(&npc.ship) {
            Some(Ok(sheet)) => {
                let frame = rotation_frame(heading, sheet.rotations);
                list.sprite(ImageKey::sprite(sheet.image_id, frame), at, tint);
                let glow = sheet.weapon.zip(self.glows.drawn(ShipRef::Npc(npc.id)));
                if let Some((layer, level)) = glow {
                    let viewer = session.player().position;
                    let murk = sprite_murk(npc.state.position, viewer, session.effective_murk());
                    let tint = murk_tint(lights_tint(npc_glow_level(level, murk)), murk);
                    let layer_frame = frame % layer.frames.get();
                    list.or_sprite(ImageKey::sprite(layer.image_id, layer_frame), at, tint);
                }
            }
            _ => crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER),
        }
    }

    /// The camera, on the ship as drawn, in the view's size.
    #[must_use]
    pub fn camera(&self) -> Camera {
        Camera::centred_on_in(self.shown_position(), self.view)
    }

    /// The view's size in logical units: Nova's 1024 x 768 until
    /// [`FlightView::set_view_size`] widens it.
    #[must_use]
    pub fn view_size(&self) -> (f32, f32) {
        self.view
    }

    /// Sets the view's size, `view` logical units: the app's widened size
    /// ([`Screen::set_widened_size`](crate::Screen::set_widened_size)).
    /// The camera, the HUD's places, the play area the session is told
    /// at the next steps, the starfield and the fades all take it from
    /// the next frame on (see the module docs). Nova's own 1024 x 768
    /// draws as flight always has.
    pub fn set_view_size(&mut self, view: (f32, f32)) {
        self.view = view;
    }

    /// How far right the course map's 1024 x 768 frame is drawn, and its
    /// clicks taken: centred in the widened view over flight
    /// ([`classic_offset`]), and at 0 over the spaceport, which is
    /// classic, where the bar and the docked mission dialogs open it.
    pub(super) fn map_offset(&self) -> Point {
        let in_port = self
            .session
            .as_ref()
            .is_ok_and(|session| session.landed().is_some());
        let x = if in_port {
            0.0
        } else {
            classic_offset(self.view.0)
        };
        Point::new(x, 0.0)
    }

    /// The frame of the ship's sheet drawn, or `None` without a sheet.
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let sheet = self.sheet.as_ref().ok()?;
        Some(rotation_frame(self.shown_heading(), sheet.rotations))
    }

    /// What the shield, armour and fuel gauges show this frame: while
    /// docked, by [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence)'s engine reading, the snapshot
    /// taken at their last redraw, as the original's port draws them
    /// only at its redraws (`_statusRedraw[1]` and `[4]`): an outfitter
    /// order or a ship purchase under the same `ïntf` leaves them as they
    /// were (@0x5c59d, @0x5d324). In flight the same while the texts of
    /// a person's offer answered over flight show
    /// ([`FlightView::answer_in_flight`]), as the text dialog's modal loop
    /// draws no bar. Otherwise in flight, by the other reading, or before
    /// the first redraw, the gauges as they are now
    /// ([`FlightView::reserves`]): flight's every frame marks them as
    /// they change.
    pub(super) fn reserves_shown(&self) -> Reserves {
        let docked = self
            .session
            .as_ref()
            .is_ok_and(|session| session.landed().is_some());
        match self.reserves_scene {
            Some(held) if (docked || !self.bar_steps.is_empty()) && self.hud_waits() => held,
            _ => self.reserves(),
        }
    }

    /// The ship's shield, armour and fuel, or none for a session that
    /// never started.
    pub(super) fn reserves(&self) -> Reserves {
        self.session
            .as_ref()
            .map(Session::reserves)
            .unwrap_or_default()
    }

    /// The player's ship now, or where it was for a session that never
    /// started.
    fn current(&self) -> ShipState {
        self.session
            .as_ref()
            .map_or(self.previous, |session| *session.player())
    }

    /// The controls for the keys held.
    pub(super) fn controls(&self) -> Controls {
        let holding = |key| self.held.contains(&key);
        let turn = match (holding(Key::Left), holding(Key::Right)) {
            (true, false) => Turn::Left,
            (false, true) => Turn::Right,
            _ => Turn::None,
        };
        let face = match (holding(FACE_TARGET_KEY), holding(Key::Alt)) {
            (false, _) => Face::Off,
            (true, false) => Face::Target,
            (true, true) => Face::Stellar,
        };
        Controls {
            thrust: holding(Key::Up),
            turn,
            reverse: holding(Key::Down),
            face,
            afterburner: holding(AFTERBURNER_KEY),
        }
    }

    /// Whether the session takes `command` now ([`Session::takes`]); a
    /// screen whose session failed to start takes every one, as it has
    /// nothing to refuse them with.
    pub(super) fn takes(&self, command: PlayerCommand) -> bool {
        self.session
            .as_ref()
            .map_or(true, |session| session.takes(command))
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
            SELECT_KEY => PlayerCommand::SelectWeapon,
            NEAREST_STELLAR_KEY | NAV_OFF_KEY | ZOOM_OUT_KEY | ZOOM_IN_KEY | ZOOM_IN_KEYPAD_KEY => {
                PlayerCommand::Nav
            }
            _ if STELLAR_SLOT_KEYS.iter().any(|keys| keys.contains(&key)) => PlayerCommand::Nav,
            _ => return None,
        };
        Some(command)
    }

    /// A press of `key`, if it works the stellar navigation target. 1-4,
    /// like F1-F4, select the stellar in their slot
    /// ([`STELLAR_SLOT_KEYS`]); while the escort menu is open, 1-5 pick
    /// its groups instead and never reach here ([`FlightView::escort_key`]).
    /// F5 selects the nearest ([`NEAREST_STELLAR_KEY`]); Nav Off turns the
    /// nav system off, or with Option held lets go of the ship target
    /// ([`NAV_OFF_KEY`]).
    pub(super) fn nav_key(&mut self, key: Key) {
        let slot = STELLAR_SLOT_KEYS
            .iter()
            .position(|keys| keys.contains(&key));
        match slot {
            Some(slot) => self.select_stellar(StellarPick::Slot(slot)),
            _ if key == NEAREST_STELLAR_KEY => self.select_stellar(StellarPick::Nearest),
            _ if key == NAV_OFF_KEY => self.press_nav_block(key),
            _ => {}
        }
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
    pub(super) fn hyperspace_key(&mut self, key: Key) {
        if self.session.is_err() {
            return;
        }
        if key == HYPER_SELECT_KEY {
            self.select_latch = false;
            self.select_frame(true);
            if self
                .session
                .as_ref()
                .is_ok_and(|session| matches!(session.nav_mode(), NavMode::Hyperspace(_)))
            {
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

    /// Whether a jump is under way, the pre-jump stage included: the
    /// original's +0x50 > 0, which its key blocks test.
    pub(super) fn jump_under_way(&self) -> bool {
        self.jump.is_some()
            || self
                .session
                .as_ref()
                .is_ok_and(|session| session.preparing_jump().is_some())
    }

    /// One frame of Option-\` and the Nav Off / Hyperspace Mode block,
    /// with \` (`nav_off`), Option and H down or not, as `_HandlePlayer`
    /// runs them (@0x69a17-0x69baa) on [`FlightView::nav_latch`]:
    ///
    /// - With \` and Option down and the latch clear, the latch set and
    ///   the ship target let go ([`Session::clear_target`], @0x69a17-0x69a90),
    ///   during a jump too.
    /// - Then, with \` without Option or H down outside a jump
    ///   (@0x69aa1-0x69ad4), and the latch clear, the latch set and the
    ///   block run ([`Session::nav_mode_keys`]); otherwise, the latch
    ///   cleared (@0x69baa).
    ///
    /// So Option-\` held lets go of a target picked meanwhile on the next
    /// frame, unless H is down; Option let go with \` still down is a
    /// fresh Nav Off; H and \` block each other while one is held; and
    /// either held through a jump's end acts after it.
    pub(super) fn nav_keys_frame(&mut self, nav_off: bool, option: bool, h: bool) {
        let jumping = self.jump_under_way();
        let Ok(session) = &mut self.session else {
            return;
        };
        if nav_off && option && !self.nav_latch {
            self.nav_latch = true;
            session.clear_target();
        }
        if ((nav_off && !option) || h) && !jumping {
            if !self.nav_latch {
                self.nav_latch = true;
                session.nav_mode_keys(nav_off, h);
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
        if let Ok(session) = &mut self.session {
            session.select_next_system();
        }
    }

    /// Makes the stellar `pick` finds the navigation target, if any.
    pub(super) fn select_stellar(&mut self, pick: StellarPick) {
        if let Ok(session) = &mut self.session {
            session.select_stellar(pick);
        }
    }

    /// A click at screen point `at`, as the original's in space
    /// (`_HandlePlayer` @0x6a000-0x6a764): one in the status panel, the
    /// right [`STATUS_BAR_WIDTH`] (0xc2) of the view at any width, x ≥
    /// width − 194 (@0x6a046-0x6a065), radar included, is
    /// no click in space and does nothing (@0x6a065, @0x6a36d); nor does
    /// one in the open escort menu's frame, which the original's menu
    /// branch takes and ends (@0x6a060-0x6a12b; its row selection is left
    /// out, task escorts-follow-up). Any other goes to
    /// [`Session::click_in_space`], the ships boxed where they are drawn
    /// ([`ViewShips`]): a ship under it is targeted, during a jump too, and
    /// otherwise the stellar under it selected.
    pub(super) fn click_in_space(&mut self, at: Point) {
        if at.x >= self.view.0 - STATUS_BAR_WIDTH || self.escort_menu.holds_click(at) {
            return;
        }
        let world = self.camera().screen_to_world(at);
        let Ok(session) = &self.session else {
            return;
        };
        let ships = ViewShips {
            view: self,
            session,
        };
        let drawn = DrawnNow::of(&ships);
        if let Ok(session) = &mut self.session {
            session.click_in_space(Vec2::new(world.x, world.y), &drawn);
        }
    }

    /// The floating map's clock: the flight's elapsed time in Mac ticks
    /// (see the [`floating`](crate::flight::floating) module's trade-offs).
    fn mac_now(&self) -> u128 {
        sixtieths(self.elapsed)
    }

    /// Shows the floating map, restarting its clock, as
    /// `_ShowFloatingMap` does.
    ///
    /// Each show takes the map's drawing afresh, as `_ShowFloatingMap`
    /// draws it into the frame then (`_DrawMapInFloatingFrame`): the
    /// session's drawing off the hypergate map (`_hypergateMapMode` = 0,
    /// @0x114cb) with the mission systems taken now (@0x114ea), and the
    /// system selected now ([`Session::floating_preselection`],
    /// @0x1147b-0x114c8), whose name alone is drawn above a zoom of 1.1.
    /// It marks no green system, with multiple arrows allowed
    /// ([`MapMarks::NONE`](nova_sim::MapMarks::NONE)): it shows only from
    /// `_HandlePlayer`, never under a mission dialog's map.
    /// The frame shows that drawing until the next show: its course and
    /// mode-3 link are those of the last show, not live, as the
    /// original's frame image is drawn only then (at a show, Hyper Select,
    /// a click and a zoom, each of which shows it again here).
    fn show_floating_map(&mut self) {
        self.floating.show(self.mac_now());
        // `_DrawMapInFloatingFrame` sets `_mapScroll` to the current
        // system as it draws (@0x11338-0x1136d), so the course map opens
        // there next.
        self.map.recentre();
        if let Ok(session) = &self.session {
            self.floating.preselect(session.floating_preselection());
            let missions = session.map_mission_systems();
            let drawing = session.map_drawing(None, &missions, nova_sim::MapMarks::NONE);
            self.floating_picture = Some(MapPicture::of(&drawing));
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
        let session = self.session.as_ref();
        let jumping =
            self.jump.is_some() || session.is_ok_and(|session| session.preparing_jump().is_some());
        let mode_3 =
            session.is_ok_and(|session| matches!(session.nav_mode(), NavMode::Hyperspace(_)));
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
    /// or a link's box selects through the session
    /// ([`Session::select_on_floating_map`]) and shows the map again
    /// (@0x6a735); a miss is swallowed (@0x6a342). Whether it took it.
    /// During a jump the original passes the click on (@0x6a145), where a
    /// click in space selects nothing either; here the session refuses the
    /// hit, so the click does nothing and the map isn't shown again.
    pub(super) fn click_floating_map(&mut self, at: Point) -> bool {
        let frame = floating::frame(self.view);
        if !self.floating.is_up() || !floating::contains(frame, at) {
            return false;
        }
        let Ok(session) = &mut self.session else {
            return true;
        };
        let galaxy = session.galaxy();
        let point = |id| {
            galaxy
                .position(id)
                .map_or_else(Point::default, |at| Point::new(at.x, at.y))
        };
        let links: Vec<(SystemId, Point)> = session
            .floating_map_links()
            .into_iter()
            .map(|link| (link, point(link)))
            .collect();
        let current = point(session.system());
        let picked = match floating::hit(&self.floating, frame, at, current, &links) {
            Hit::Centre => Some(None),
            Hit::Link(link) => Some(Some(link)),
            Hit::Miss | Hit::Outside => None,
        };
        if let Some(link) = picked
            && session.select_on_floating_map(link)
        {
            self.show_floating_map();
        }
        true
    }

    /// A zoom key's press: while the floating map is up and no key the
    /// flight holds is down (`_quickKeyCheckResult` and the modifiers,
    /// @0x70f4e-0x70fb5), `-` steps it out and `=` or keypad `+` in. One
    /// that changes the scale shows the map again and raises
    /// [`UiSound::FloatingMapZoomed`] (`_beepSnd[0]`, @0x7103a, @0x710bc);
    /// one at a limit does nothing.
    pub(super) fn zoom_floating_map(&mut self, key: Key) {
        if !self.floating.is_up() || !self.held.is_empty() {
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

    /// Draws the floating map while it is up: centred on the current
    /// system at its zoom, the drawing its last show took
    /// ([`FlightView::show_floating_map`]) through the course map's
    /// [`GalaxyMap::draw_marks`](crate::galaxy::GalaxyMap::draw_marks), with no brackets or second names, as
    /// `_DrawMapInFloatingFrame` runs `_DrawMap` alone, in its frame
    /// bordered in `cölr` `FloatingMap`, clipped and faded to its level
    /// ([`floating::draw`]).
    pub(super) fn draw_floating_map(&self, list: &mut DrawList) {
        let Some(level) = self.floating.level(self.mac_now()) else {
            return;
        };
        let Ok(session) = &self.session else {
            return;
        };
        let current = session.system();
        let at = session
            .galaxy()
            .position(current)
            .map_or_else(Point::default, |at| Point::new(at.x, at.y));
        let frame = floating::frame(self.view);
        let transform = floating::transform(frame, at, self.floating.zoom());
        let Some(picture) = &self.floating_picture else {
            return;
        };
        let marks = MapMarks {
            picture,
            selected: self.floating.preselection(),
        };
        floating::draw(list, frame, level, self.escort_colors.border, |raw| {
            self.map.draw_marks(raw, &transform, &marks);
        });
    }

    /// The escort menu's class rows ([`Session::escort_menu`]); none for
    /// a session that failed.
    pub(super) fn escort_rows(&self) -> [ClassRow; 4] {
        self.session.as_ref().map_or_else(
            |_| {
                nova_sim::EscortClass::ALL.map(|class| ClassRow {
                    class,
                    present: false,
                    order: None,
                })
            },
            Session::escort_menu,
        )
    }

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

    /// Whether a press of `key` is taken while a jump is under way, the
    /// pre-jump stage included, with Option held (`option`) and the escort
    /// menu open (`menu_open`) or not. `_HandlePlayer` runs its key
    /// handling on every frame of a jump but the arrival (@0x6c625), and
    /// these keys' blocks read no jump state (+0x50):
    ///
    /// - Target Select, Tab, with Shift and Option ([`TARGET_KEY`],
    ///   @0x69f1c-0x69ffb), and Closest Target, R and Option-R
    ///   ([`NEAREST_KEY`], @0x6a9b7-0x6aae7);
    /// - Weapon Select, W and Option-W ([`SELECT_KEY`], @0x6ac12-0x6adbf);
    /// - Option-\` ([`NAV_OFF_KEY`] with Option, @0x69a17-0x69a90), which
    ///   lets go of the ship target;
    /// - Board, B ([`BOARD_KEY`], @0x6a98b-0x6a9a8);
    /// - Hail, Y ([`HAIL_KEY`], @0x703cc-0x70405), which the session then
    ///   refuses, as `_HandlePlayerCommunication` does (@0x61f87);
    /// - Jettison, Command-K and Command-Option-K ([`JETTISON_KEY`],
    ///   @0x6fd54-0x6fdaf);
    /// - the escort menu's keys: E, Return, F, D, V and C, and 1-5 while the
    ///   menu is open (@0x6dd00-0x6e452);
    /// - the floating map's zoom keys, `-`, `=` and the keypad's `+`
    ///   ([`ZOOM_OUT_KEY`], [`ZOOM_IN_KEY`], [`ZOOM_IN_KEYPAD_KEY`],
    ///   @0x70f4e-0x71110).
    ///
    /// Every other key is ignored, as the original gates it on +0x50 ≤ 0:
    /// \` without Option and H (@0x69ad4), \\ (@0x69be7), 1-4 with the
    /// menu shut, F1-F4 and F5 (@0x69d2e), M (@0x6a7a2), L (@0x6a8df),
    /// I (@0x70424), and J, which changes nothing during a jump
    /// (@0x6b0da-0x6b10d). P is refused apart
    /// ([`FlightView::refuse_player_info`]); the flight keys are tracked
    /// as held whatever this says, so H, \` and `\` dropped here act after
    /// the jump as their latches let them, and J, whose latch the jump
    /// sets, does not ([`FlightView::held_keys_frame`]).
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
    /// whatever it shows, closes the escort menu (`_HandlePlayer`
    /// @0x6dd00-0x6dd6a) and hides the floating map (@0x6dd6a-0x6dd75),
    /// during a jump too.
    fn acknowledge(&mut self) {
        self.acknowledge_message();
        self.escort_menu.close();
        self.floating.hide();
    }

    /// Gives the escorts `command`, through the session, as the menu says
    /// which; says what it changed, and keeps the menu open for it.
    fn command_escorts(&mut self, command: EscortCommand) {
        let Ok(session) = &mut self.session else {
            return;
        };
        if let Some(commanded) = session.command_escorts(self.escort_menu.group(), command) {
            self.escort_menu.touch(self.elapsed);
            self.say_for(escort_command_message(&commanded), ESCORT_TICKS);
        }
    }

    /// Closes the escort menu once it has timed out or the fleet is empty
    /// ([`EscortMenu::update`]), every frame, the streak's included, as
    /// the original checks it on every frame of a jump (@0x6df59).
    pub(super) fn update_escort_menu(&mut self) {
        let rows = self.escort_rows();
        let flying = self.session.as_ref().is_ok_and(Session::escorts_flying);
        self.escort_menu.update(self.elapsed, &rows, flying);
    }

    /// The escort menu, shut or open.
    #[must_use]
    pub fn escort_menu(&self) -> &EscortMenu {
        &self.escort_menu
    }

    /// Shuts the escort menu, as play is entered from the main screen
    /// (`escortMenuOnscreen` = 0, `_EnterGameFromMainScreen` @0x1a39e).
    pub fn shut_escort_menu(&mut self) {
        self.escort_menu.close();
    }

    /// Picks the target as `pick` says.
    pub(super) fn select_target(&mut self, pick: TargetPick) {
        if let Ok(session) = &mut self.session {
            session.select_target(pick);
        }
    }

    /// The fight's explosions, debris and sounds.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// `ship`'s shield-flash timer as the last frame showed it
    /// ([`ShieldFlash::shown`]): 32 on the frame of a hit, then a frame
    /// less each frame; none once it has run out, or with no hit. Once the
    /// shän shield layer is drawn (rdm:phase/ship-handling-fidelity/phase-10),
    /// the layer shows it at [`ShieldFlash::step`]'s level.
    #[must_use]
    pub fn shield_flash(&self, ship: ShipRef) -> Option<f32> {
        self.flashes
            .get(&ship)
            .map(ShieldFlash::shown)
            .filter(|&shown| shown > 0.0)
    }

    /// The shots in flight, each where it is drawn: `alpha` of the way
    /// from where it was a step ago, as the ships are; each with the
    /// index its spin started on ([`SpinStarts`](crate::flight::spin::SpinStarts))
    /// and its murk where the session holds it, seen from the player's
    /// ship as the session holds it, in the player's effective murk
    /// ([`murk::sprite_murk`], [`Session::effective_murk`]), as
    /// `_HandleShot` murks it once a pass after it moves (@0x35c48).
    pub(super) fn shown_shots(&self, camera: &Camera) -> Vec<ShotShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        let behind = 1.0 - self.alpha;
        let viewer = session.player().position;
        let murk = session.effective_murk();
        session
            .shots()
            .iter()
            .map(|shot| ShotShown {
                id: shot.id,
                spin_start: self.shot_looks.spin_starts.start(shot.id),
                murk: murk::sprite_murk(shot.position, viewer, murk),
                at: camera.world_to_screen(Point::new(
                    shot.velocity.x.mul_add(-behind, shot.position.x),
                    shot.velocity.y.mul_add(-behind, shot.position.y),
                )),
                heading: shot.heading,
                age: shot.age,
                weapon: shot.weapon.id,
                layer: SpriteLayer::from(shot.layer),
            })
            .collect()
    }

    /// The cargo boxes in space, each where it is drawn: `alpha` of the
    /// way from where it was a step ago, as the shots are.
    pub(super) fn shown_boxes(&self, camera: &Camera) -> Vec<BoxShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        let behind = 1.0 - self.alpha;
        session
            .cargo_boxes()
            .iter()
            .map(|cargo_box| BoxShown {
                at: camera.world_to_screen(Point::new(
                    cargo_box.velocity.x.mul_add(-behind, cargo_box.position.x),
                    cargo_box.velocity.y.mul_add(-behind, cargo_box.position.y),
                )),
                frame: cargo_box.frame,
                fade: cargo_box.fade(),
                graphic: cargo_box.graphic,
            })
            .collect()
    }

    /// The asteroids in the field, each where it is drawn: `alpha` of the
    /// way from where it was a step ago, as the cargo boxes are, at its
    /// whole frame.
    pub(super) fn shown_asteroids(&self, camera: &Camera) -> Vec<AsteroidShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        let behind = 1.0 - self.alpha;
        session
            .asteroids()
            .map(|asteroid| AsteroidShown {
                at: camera.world_to_screen(Point::new(
                    asteroid.velocity.x.mul_add(-behind, asteroid.position.x),
                    asteroid.velocity.y.mul_add(-behind, asteroid.position.y),
                )),
                // Its whole part; a frame below 0, which the single wrap
                // can leave, shows the first.
                frame: asteroid.frame as u16,
                kind: asteroid.kind,
            })
            .collect()
    }

    /// The beams being fired, each moved with its firer as it is drawn.
    pub(super) fn shown_beams(&self, camera: &Camera) -> Vec<BeamShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .beams()
            .iter()
            .map(|beam| {
                let (dx, dy) = self.drawn_off(session, beam.firer);
                let shown = |at: Vec2| camera.world_to_screen(Point::new(at.x + dx, at.y + dy));
                BeamShown {
                    start: shown(beam.start),
                    end: shown(beam.end),
                    weapon: beam.weapon.id,
                    fade: beam.fade,
                }
            })
            .collect()
    }

    /// How far `ship` is drawn from where it is.
    fn drawn_off(&self, session: &Session, ship: ShipRef) -> (f32, f32) {
        let (shown, now) = match ship {
            ShipRef::Player => (self.shown_position(), session.player().position),
            ShipRef::Npc(id) => match session.npcs().iter().find(|npc| npc.id == id) {
                Some(npc) => (self.shown_npc(npc).0, npc.state.position),
                None => return (0.0, 0.0),
            },
        };
        (shown.x - now.x, shown.y - now.y)
    }

    /// Draws the brackets round the nav target, then those round the
    /// ship target, the session's ([`Session::nav_brackets`],
    /// [`Session::target_brackets`]), each centred where its sprite is
    /// drawn (see the `brackets` module). A stellar whose sheet can't be
    /// read is bracketed as one with no sprite; a ship's as its crossed
    /// box.
    pub(super) fn draw_brackets(&self, list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
        let Ok(session) = &self.session else {
            return;
        };
        if let Some(nav) = session.nav_brackets()
            && let Some(stellar) = scene.stellars().iter().find(|s| s.id == nav.stellar)
        {
            let sprite = stellar.drawn().as_ref().ok();
            let sprite = sprite.map(|sheet| (sheet.frame_width, sheet.frame_height));
            let at = camera.world_to_screen(stellar.position);
            brackets::draw_nav(list, at, sprite, nav.expand, nav.dominated);
        }
        if let Some(target) = session.target_brackets()
            && let Some(npc) = session.target()
        {
            let (at, _) = self.shown_npc(npc);
            let sprite = drawn_size(self.npc_sheets.get(&npc.ship));
            let at = camera.world_to_screen(at);
            brackets::draw_ship(list, at, Some(sprite), target.expand, target.standing);
        }
    }

    /// Draws the Red Alert icon the session shows, if any, over the
    /// brackets on the interface layer (see the `alert` module).
    pub(super) fn draw_red_alert(&self, list: &mut DrawList) {
        let icon = self.session.as_ref().ok().and_then(Session::red_alert_icon);
        if let (Some(icon), Some(width)) = (icon, self.alert_width) {
            alert::draw_red_alert(list, icon, width, self.view.0);
        }
    }

    /// Draws the status bar's target panel ([`FlightView::target_shown`])
    /// and secondary weapon line. The weapon line follows every frame: the
    /// original redraws it (`_statusRedraw[3]`) on every event that
    /// changes it, so drawing it each frame shows the same.
    fn draw_combat_hud(&self, list: &mut DrawList, bar: &StatusBar, view_width: f32) {
        let Ok(session) = &self.session else {
            return;
        };
        let origin = hud::bar_origin(bar, view_width);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let shown = self.target_shown();
        target::draw_target_panel(list, &bar.layout, origin, shown.as_ref().as_ref(), metrics);
        let line = session.secondary().map(|id| {
            let look = self.looks.weapon(id);
            let name = look.map_or_else(|| format!("wëap {}", id.0), |look| look.name.clone());
            target::secondary_text(&name, session.secondary_line_count())
        });
        target::draw_secondary(list, &bar.layout, origin, line.as_deref(), metrics);
    }

    /// Draws the player's ship at `at`, as much of it as `drawn` says
    /// (see [`nova_sim::player_fate`]): its sprite, engine glow, running
    /// lights and weapon glow ([`weapon_glow`](crate::flight::weapon_glow),
    /// never murked), or, when its sheet cannot be read, a crossed box
    /// and the reason. The box stands in for the sprite and a missing
    /// sheet has no glow or lights, so only a ship drawn in full has one.
    fn draw_ship(&self, list: &mut DrawList, at: Point, drawn: ShipDrawn) {
        if drawn == ShipDrawn::Hidden {
            return;
        }
        match &self.sheet {
            Ok(sheet) => {
                let frame = rotation_frame(self.shown_heading(), sheet.rotations);
                if drawn == ShipDrawn::Full {
                    list.sprite(ImageKey::sprite(sheet.image_id, frame), at, Color::WHITE);
                }
                let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
                let base = self.session.as_ref().map_or(0, Session::engine_glow);
                let glow = sheet
                    .glow
                    .zip(glow_level(base, tick, &self.glow_rolls).map(lights_tint));
                let level = lights_level(&sheet.blink, tick, &self.blink_rolls);
                let lights = sheet.lights.zip(level.map(lights_tint));
                let weapon = sheet
                    .weapon
                    .zip(self.glows.drawn(ShipRef::Player).map(lights_tint));
                for (layer, tint) in [glow, lights, weapon].into_iter().flatten() {
                    let layer_frame = frame % layer.frames.get();
                    list.or_sprite(ImageKey::sprite(layer.image_id, layer_frame), at, tint);
                }
            }
            Err(_) if drawn != ShipDrawn::Full => {}
            Err(reason) => {
                crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    at.x - PLACEHOLDER_SIZE / 2.0,
                    at.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {reason}"),
                    below,
                    OVERLAY_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// Draws the HUD's status bar alone, as flight draws it: in port, the
    /// spaceport draws over it, as the original's port dialog sits beside
    /// the bar and its shops cover the bar's left edge (`_DoPortDialog`
    /// @0x5fbca draws the radar blank, as ours does while landed).
    /// Without a status bar it draws nothing: flight says why, and the
    /// spaceport stays clean. The spaceport is Nova's 1024 x 768, however
    /// wide flight's view is, so the bar is drawn at that width's right
    /// edge.
    pub fn draw_status_bar(&self, list: &mut DrawList) {
        if let Ok(bar) = &self.status_bar {
            self.draw_bar(list, bar, VIEW_SIZE.0);
        }
    }

    /// Draws `bar` showing the HUD against the right edge of a view
    /// `view_width` wide: the radar, gauges and nav area as
    /// shown ([`FlightView::radar_shown`], [`FlightView::reserves_shown`],
    /// [`FlightView::nav_shown`]), the cargo area, then the target panel
    /// and the secondary weapon's line.
    pub(super) fn draw_bar(&self, list: &mut DrawList, bar: &StatusBar, view_width: f32) {
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let radar = self.radar_shown();
        let cargo = self.session.as_ref().ok().map(Session::status_cargo);
        let state = HudState {
            radar: &radar,
            blink_on: self.hud_cadence.blink_on(),
            reserves: self.reserves_shown(),
            nav: self.nav_shown(),
            cargo: cargo.as_ref(),
            disabled: self
                .session
                .as_ref()
                .is_ok_and(|session| session.player_condition() == Condition::Disabled),
        };
        hud::draw(list, bar, &state, metrics, view_width);
        self.draw_combat_hud(list, bar, view_width);
    }

    /// Draws the [`HELP`] line when its overlay is on.
    pub(super) fn draw_help(&self, list: &mut DrawList) {
        if self.overlays.help {
            list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
        }
    }
}
