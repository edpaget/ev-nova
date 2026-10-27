//! [`FlightView`]'s construction: reading the session and catalog, the
//! builders, the hail, player and mission info requests, and the
//! preferences.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::PlayerCommand;
use nova_sim::{
    ALERT_CICN, AsteroidCatalog, Behaviour, BoardingRule, CombatCatalog, CommCatalog, DisableRule,
    FixedStep, HailOptions, HashedRolls, LegalCode, Pilot, PilotCatalog, PointDefenceRule,
    Rulebook, Session, StartError, TrafficCatalog,
};
use nova_sim::{
    ControlBits, HireTerms, KeptBerths, LastRefresh, PersonRules, PlayerInfo,
    pilot_panel::PilotPanel, ship_layer::SlotSprites, story::sprite_pool::StellarSpritePool,
};

use crate::flight::cadence::HudCadence;
use crate::flight::catalog::{CombatLooks, Looks, ShipSprites, StatusBars};
use crate::flight::cursor::HoverCursor;
use crate::flight::effects::Effects;
use crate::flight::escorts::{EscortMenu, EscortMenuLooks};
use crate::flight::floating::FloatingMap;
use crate::flight::hud::{self};
use crate::flight::sim_adapter::SessionFlight;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::camera::VIEW_SIZE;
use crate::system::catalog::SystemCatalog;
use crate::system::kept_frames::KeptFrames;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::{Diagnostic, ScreenAction, SoundPrefs};

use super::{
    BarFor, FlightOverlays, FlightView, Metrics, SharedChance, Then, opening_scene, window_width,
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
> FlightView<C, crate::flight::SessionFlight<C>>
{
    /// A new, unnamed pilot's flight, read from `catalog`, which the
    /// screen keeps.
    pub fn new(catalog: C) -> Self
    where
        C: 'static,
    {
        let session = Session::start(&catalog);
        Self::flying(catalog, session)
    }

    /// `pilot`'s flight, read from `catalog`, which the screen keeps. A
    /// pilot last landed on a stellar resumes in flight over it, at rest,
    /// as [`Session::fly`] places it: nothing lands, so no landing is
    /// reported ([`FlightView::take_landing`]) and nothing sounds.
    pub fn with_pilot(catalog: C, pilot: Pilot) -> Self
    where
        C: 'static,
    {
        let session = Session::fly(&catalog, pilot);
        Self::flying(catalog, session)
    }

    /// `pilot`'s flight as [`FlightView::with_pilot`] builds it, its load's
    /// fallbacks reading `rules` ([`Session::load`]): for a pilot opened
    /// from a save, whose repair [`FlightView::load_repaired`] reports.
    pub fn loaded(catalog: C, pilot: Pilot, rules: Rulebook) -> Self
    where
        C: 'static,
    {
        let session = Session::load(&catalog, pilot, &rules);
        Self::flying(catalog, session)
    }

    /// The flight of `sim`, read from `catalog` as a loaded session is
    /// read, but reached without the load: for a test whose session a
    /// builder put into the state it needs
    /// (`nova_sim::fixture::SessionBuilder`), passed as
    /// `builder.build().into()` for Nova's collaborators
    /// ([`SessionFlight::new`]).
    #[cfg(any(test, feature = "fixture"))]
    pub fn from_session(catalog: C, sim: SessionFlight<C>) -> Self
    where
        C: 'static,
    {
        Self::read(catalog, sim)
    }

    /// Begins a new pilot's game as [`Session::begin`] does, on the
    /// flight's chance: for a pilot just created, before its first save.
    /// The
    /// ship, which the beginning puts at (50, 50), is drawn there from the
    /// first frame, not on its way from where the view was read, and the
    /// stars are scattered afresh about it when the flight is next caught
    /// up with, as `_ResetGameData` scatters them (@0x1df5e). A session
    /// that failed begins nothing.
    pub fn begin(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.begin(&*self.catalog, &mut self.sim.chance);
            self.place_ship();
            self.stars_due = true;
        }
    }

    /// Enters play from the main screen as [`Session::enter_game`] does:
    /// the refresh a later new pilot reads is taken afresh, and the stars
    /// are scattered afresh when the flight is next caught up with, as
    /// `_EnterGameFromMainScreen` scatters them (@0x19f7c). A session that
    /// failed enters nothing.
    pub fn enter_game(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.enter_game();
            self.stars_due = true;
        }
    }

    /// Resumes a pilot opened from a save as [`Session::resume`] does, on
    /// the flight's chance: its ship faces a drawn heading, as
    /// `_LoadPilotData` faces every pilot it loads (@0x7535b-0x75372), and
    /// the offers' roll table is drawn, as the load's `_ResetPlayer` draws
    /// it (@0x76502, @0x7673e). A session that failed resumes nothing.
    pub fn resume(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.resume(&mut self.sim.chance);
        }
    }

    /// Opens a pilot just loaded on the main screen, as Open Pilot's
    /// `_MainScreenChoice(1)` does once it is loaded: every active
    /// mission's auxiliary ships restart
    /// ([`Session::restart_aux_from_main_screen`], @0x1a918-0x1a982) on
    /// the flight's chance. A session that failed opens nothing.
    pub fn open_on_main_screen(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.restart_aux_from_main_screen(&mut self.sim.chance);
        }
    }

    /// Opens the last pilot at launch, once it is loaded, as
    /// `_OpenLastPilotFile` and `_DoMainScreen`'s once-a-launch setup do
    /// ([`Session::open_at_launch`]) on the flight's chance: every active
    /// mission's auxiliary ships restart in the unguarded form
    /// (@0x765d4-0x76637), and the player's system is explored (@0x1affe).
    /// A session that failed opens nothing.
    pub fn open_at_launch(&mut self) {
        if let Ok(session) = &mut self.sim.session {
            session.open_at_launch(&mut self.sim.chance);
        }
    }

    /// The flight of `session`, or why it could not start. No session
    /// starts landed, so flight's first frame redraws the bar, as
    /// `_PlayGame`'s `_RedrawStatus` (@0x457d8) does.
    fn flying(catalog: C, session: Result<Session, StartError>) -> Self
    where
        C: 'static,
    {
        Self::read(
            catalog,
            SessionFlight::new(session.map_err(|err| err.to_string())),
        )
    }

    /// The flight of `sim` as [`FlightView::flying`] reads it.
    #[allow(clippy::too_many_lines)] // one line for each of the screen's fields
    fn read(catalog: C, mut sim: SessionFlight<C>) -> Self
    where
        C: 'static,
    {
        if let Ok(session) = sim.session_mut() {
            // Sized as the view starts, before anything reads its field.
            session.set_window_width(window_width(VIEW_SIZE));
        }
        let mut frame = nova_sim::flight_frame::FlightFrame::default();
        sim.frame(&mut frame);
        let session = &sim.session;
        let (scene, sheet, sheet_ship, status_bar) = match &session {
            Ok(_) => (
                Some(opening_scene(&catalog, &frame)),
                catalog.ship_sheet(frame.player().class),
                Some(frame.player().class),
                hud::choose_status_bar(&catalog, frame.hud.interface_government),
            ),
            Err(reason) => (None, Err(reason.clone()), None, Err(reason.clone())),
        };
        let bar_government = session
            .as_ref()
            .ok()
            .map(|_| BarFor(frame.hud.interface_government));
        let mut map = GalaxyMap::course(&catalog);
        if let Ok(session) = &session {
            map.set_nebula_pick(session.rules().source_for(nova_sim::RuleKey::NebulaPicture));
            map.set_border_rules(
                session.rules().source_for(nova_sim::RuleKey::MapBorderPace),
                session
                    .rules()
                    .source_for(nova_sim::RuleKey::MapFindBorders),
            );
            map.show_galaxy(session.galaxy());
            map.show_course(frame.system, &frame.course_shown);
            // Entering the game draws the floating map, and a new or
            // loaded pilot sets `_mapScroll`, to the current system
            // (`_EnterGameFromMainScreen` @0x19faa, `_DoNewPilot`
            // @0x19054, `_LoadPilotData` @0x7537b).
            map.recentre();
        }
        let looks = match &session {
            Ok(_) => Looks::read(&catalog, catalog.weapons().iter().map(|weapon| weapon.id)),
            Err(_) => Looks::default(),
        };
        let escort_colors = catalog.escort_menu_colors();
        let alert_width = catalog.icon_size(ALERT_CICN).map(|(width, _)| width);
        let mut unread_looks: Vec<_> = looks
            .problems()
            .into_iter()
            .map(Diagnostic::Unreadable)
            .collect();
        if session.is_ok() && alert_width.is_none() {
            unread_looks.push(Diagnostic::Unreadable(format!(
                "no cïcn {ALERT_CICN}: the Red Alert icon is not drawn"
            )));
        }
        let presentation = super::stepping::Presentation::of(
            session
                .as_ref()
                .map_or(&nova_sim::Rulebook::default(), Session::rules),
        );
        // One handle, the view's and the flight's: the flight's answers
        // that read the catalog read the one the view draws from.
        let catalog = Rc::new(catalog);
        sim.share_catalog(Rc::clone(&catalog) as Rc<dyn nova_sim::SimCatalog>);
        Self::assembled(
            catalog,
            sim,
            Opening {
                map,
                scene,
                sheet,
                sheet_ship,
                status_bar,
                bar_government,
                frame,
                looks,
                unread_looks,
                escort_colors,
                alert_width,
                presentation,
            },
        )
    }

    /// The flight with every disputed rule reading as `rules` says
    /// ([`Session::with_rules`]). The stellars of the scene shown are
    /// drawn again by the new rules, which say how a destroyed stellar
    /// with no `DeadType` sheet draws, but not laid out again; the course
    /// map's nebulae take their picture by the new
    /// [`RuleKey::NebulaPicture`](nova_sim::RuleKey::NebulaPicture), and
    /// its borders their pace and Find's pass by
    /// [`RuleKey::MapBorderPace`](nova_sim::RuleKey::MapBorderPace) and
    /// [`RuleKey::MapFindBorders`](nova_sim::RuleKey::MapFindBorders); the
    /// steps' effects and stars are shown by them too
    /// ([`Presentation`](super::stepping::Presentation)).
    #[must_use]
    pub fn with_rules(mut self, rules: Rulebook) -> Self {
        let pick = rules.source_for(nova_sim::RuleKey::NebulaPicture);
        let pace = rules.source_for(nova_sim::RuleKey::MapBorderPace);
        let find = rules.source_for(nova_sim::RuleKey::MapFindBorders);
        self.presentation = super::stepping::Presentation::of(&rules);
        self.sim.session = self.sim.session.map(|session| session.with_rules(rules));
        self.map.set_nebula_pick(pick);
        self.map.set_border_rules(pace, find);
        self.show_destroyed();
        self
    }

    /// The flight with its ship slots' draw layers and cloned frames
    /// starting from `slot_sprites`, the history an earlier flight of the
    /// same run left ([`Session::with_slot_sprites`]).
    #[must_use]
    pub fn with_slot_sprites(mut self, slot_sprites: SlotSprites) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_slot_sprites(slot_sprites));
        self.read_npc_sheets();
        self
    }

    /// The ship slots' draw-layer and cloned-frame history as the flight
    /// leaves it ([`Session::slot_sprites`]), or `None` when its session
    /// never started.
    #[must_use]
    pub fn slot_sprites(&self) -> Option<&SlotSprites> {
        self.sim.session.as_ref().ok().map(Session::slot_sprites)
    }

    /// The flight with its stellar sprite pool carried from `pool`, the
    /// one an earlier flight of the same run left
    /// ([`Session::carry_stellar_sprite_pool`]): the original never clears
    /// a stellar's sprite pointer between pilots.
    #[must_use]
    pub fn with_stellar_sprite_pool(mut self, pool: StellarSpritePool) -> Self {
        if let Ok(session) = &mut self.sim.session {
            session.carry_stellar_sprite_pool(pool);
        }
        self
    }

    /// The stellar sprite pool as the flight leaves it
    /// ([`Session::stellar_sprite_pool`]), or `None` when its session never
    /// started.
    #[must_use]
    pub fn stellar_sprite_pool(&self) -> Option<&StellarSpritePool> {
        self.sim
            .session
            .as_ref()
            .ok()
            .map(Session::stellar_sprite_pool)
    }

    /// The flight with its traffic's berths starting from `kept`, what an
    /// earlier flight of the same run left ([`Session::with_kept_berths`]).
    #[must_use]
    pub fn with_kept_berths(mut self, kept: KeptBerths) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_kept_berths(kept));
        self
    }

    /// What the flight's traffic's berths keep for the next flight
    /// ([`Session::kept_berths`]), or `None` when its session never
    /// started.
    #[must_use]
    pub fn kept_berths(&self) -> Option<KeptBerths> {
        self.sim.session.as_ref().ok().map(Session::kept_berths)
    }

    /// The flight with a new pilot's neighbour pass reading `carried`,
    /// what an earlier flight of the same run left
    /// ([`Session::with_last_refresh`]).
    #[must_use]
    pub fn with_last_refresh(mut self, carried: LastRefresh) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_last_refresh(carried));
        self
    }

    /// What the flight's last galaxy refresh left hidden, for a new pilot
    /// made later in the run ([`Session::last_refresh`]), or `None` when
    /// its session never started.
    #[must_use]
    pub fn last_refresh(&self) -> Option<LastRefresh> {
        self.sim.session.as_ref().ok().map(Session::last_refresh)
    }

    /// The flight with `terms` giving the fee and wage of a hire
    /// ([`Session::with_hire_terms`]).
    #[must_use]
    pub fn with_hire_terms(mut self, terms: Rc<dyn HireTerms>) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_hire_terms(terms));
        self
    }

    /// The flight with `rules` deciding how persons appear
    /// ([`Session::with_person_rules`]).
    #[must_use]
    pub fn with_person_rules(mut self, rules: Rc<dyn PersonRules>) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_person_rules(rules));
        self
    }

    /// The flight with the `T` set operator naming the ship from
    /// `strings`' string lists ([`Session::with_strings`]).
    #[must_use]
    pub fn with_strings(mut self, strings: Rc<dyn CommCatalog>) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_strings(strings));
        // The cargo area's labels are the strings' (`FrameHud::cargo`).
        self.refresh_frame();
        self
    }

    /// The flight with `bits` testing a ship's `Availability` for hire
    /// ([`Session::with_control_bits`]).
    #[must_use]
    pub fn with_control_bits(mut self, bits: Rc<dyn ControlBits>) -> Self {
        self.sim.session = self
            .sim
            .session
            .map(|session| session.with_control_bits(bits));
        self
    }

    /// The flight with the HUD's text, and the course map's panels',
    /// measured by `metrics`.
    #[must_use]
    pub fn with_metrics(self, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            map: self.map.with_metrics(Rc::clone(&metrics)),
            metrics: Some(Metrics(metrics)),
            ..self
        }
    }

    /// The flight with the course map at `zoom`, the run's: `_mapZoom` is
    /// never reset, so a new or loaded pilot's map opens at the zoom the
    /// last flight's was left at ([`GalaxyMap::with_zoom`]).
    #[must_use]
    pub fn with_course_map_zoom(self, zoom: f64) -> Self {
        Self {
            map: self.map.with_zoom(zoom),
            ..self
        }
    }

    /// The flight with the course map's borders shown or not, as the
    /// player's `ShowMapBorders` setting says
    /// ([`GalaxyMap::show_borders_setting`]).
    #[must_use]
    pub fn with_map_borders(mut self, shown: bool) -> Self {
        self.map.show_borders_setting(shown);
        self
    }

    /// The flight with the course map's Find laid out from `template`
    /// (stock `DLOG` 3005), or its fallback for `None`, and the map's
    /// buttons labelled in `style` ([`GalaxyMap::with_find`]).
    #[must_use]
    pub fn with_find(self, template: Option<DialogTemplate>, style: ButtonStyle) -> Self {
        Self {
            map: self.map.with_find(template, style),
            ..self
        }
    }

    /// The flight with its NPCs deciding as `behaviour` says.
    #[must_use]
    pub fn with_behaviour(self, behaviour: Rc<dyn Behaviour>) -> Self {
        Self {
            sim: self.sim.with_behaviour(behaviour),
            ..self
        }
    }

    /// The flight with its ships disabled as `rule` says: in the fight,
    /// and in its session's changes of ship
    /// ([`Session::with_disable_rule`]).
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            sim: self.sim.with_disable_rule(disable_rule),
            ..self
        }
    }

    /// The flight with its ships' point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            sim: self.sim.with_point_defence_rule(defence_rule),
            ..self
        }
    }

    /// The flight with the player's crimes judged by `law`.
    #[must_use]
    pub fn with_law(self, law: Rc<dyn LegalCode>) -> Self {
        Self {
            sim: self.sim.with_law(law),
            ..self
        }
    }

    /// The flight with its boardings by `rule`: who repels boarders, the
    /// capture odds and the capture roll.
    #[must_use]
    pub fn with_boarding_rule(self, boarding_rule: Rc<dyn BoardingRule>) -> Self {
        Self {
            sim: self.sim.with_boarding_rule(boarding_rule),
            ..self
        }
    }

    /// The flight with the comm dialog listing `hail_options`
    /// ([`SessionFlight::with_hail_options`]).
    #[must_use]
    pub fn with_hail_options(self, hail_options: HailOptions) -> Self {
        Self {
            sim: self.sim.with_hail_options(hail_options),
            ..self
        }
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen since
    /// this was last taken, once: the router opens it on
    /// [`FlightView::player_info`] over the paused flight.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_player_info)
    }

    /// What the player info screen shows, as [`Session::player_info`]
    /// gives it from the screen's catalog, marked with the cheats setting,
    /// changing nothing; `None` for a session that failed. An opening of
    /// the screen is
    /// [`FlightView::open_player_info`].
    #[must_use]
    pub fn player_info(&self) -> Option<PlayerInfo> {
        let session = self.sim.session.as_ref().ok()?;
        Some(self.marked(session.player_info(&*self.catalog)))
    }

    /// What the main screen's pilot panel says about the pilot, as
    /// [`Session::pilot_panel`] gives it from the screen's catalog,
    /// changing nothing; `None` for a session that failed.
    #[must_use]
    pub fn pilot_panel(&self) -> Option<PilotPanel> {
        let session = self.sim.session.as_ref().ok()?;
        Some(session.pilot_panel(&*self.catalog))
    }

    /// The player info dialog opens, as [`Session::open_player_info`] has
    /// it: the session resets first, so the screen it gives shows the
    /// cargo trimmed; none in a jump or while the ship breaks up, when
    /// nothing opens, or for a session that failed. It is marked with the
    /// cheats setting. The router opens the screen on it, from flight and
    /// from the spaceport alike.
    pub fn open_player_info(&mut self) -> Option<PlayerInfo> {
        let session = self.sim.session.as_mut().ok()?;
        let info = session.open_player_info(&*self.catalog)?;
        Some(self.marked(info))
    }

    /// `info` marked with the cheats setting ([`FlightOverlays::cheats`]),
    /// which the session leaves off.
    fn marked(&self, info: PlayerInfo) -> PlayerInfo {
        PlayerInfo {
            cheats: self.overlays.cheats,
            ..info
        }
    }

    /// Presses [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) outside a jump, its [`PlayerCommand`]
    /// taken: when the session takes the press too
    /// ([`Session::press_player_info`]), asks the router for the player
    /// info screen, letting go of the flight keys (`_HandlePlayer`
    /// @0x702bf-0x703c9); when it refuses, with its beep, nothing more.
    pub(super) fn ask_player_info(&mut self) -> ScreenAction {
        if self.press_player_info() {
            self.pending_player_info = true;
            self.held.clear();
        }
        ScreenAction::None
    }

    /// Whether the session takes a press of [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY)
    /// ([`Session::press_player_info`], which beeps a refusal); a failed
    /// session takes it, and the router opens nothing for it.
    fn press_player_info(&mut self) -> bool {
        match &mut self.sim.session {
            Ok(session) => session.press_player_info(),
            Err(_) => true,
        }
    }

    /// A press of [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) during a jump: outside the map, the
    /// session refuses it with its beep ([`Session::press_player_info`],
    /// @0x702f7, @0x7039a-0x703bd), when the player's fate takes it.
    pub(super) fn refuse_player_info(&mut self) -> ScreenAction {
        if !self.map_open && self.takes(PlayerCommand::PlayerInfo) {
            let _ = self.press_player_info();
        }
        ScreenAction::None
    }

    /// Jettisons everything, mission cargo included, as the player info's
    /// Jettison does (`_JettisonCargo(1)` @0x5a66d), saying what went.
    /// Docked, an `OnFailure` `C`, `E` or `H` of a mission it fails
    /// (`_QuickMissionFailure` @0x99d83) that changes the `ïntf` redraws
    /// the whole bar at its operator; the player info's close redraws
    /// after it (@0x5a725).
    pub fn jettison_all(&mut self) {
        self.run_docked(Then::Nothing, |view| {
            if let Ok(session) = &mut view.sim.session {
                session.jettison_cargo(true, &mut view.sim.chance);
            }
        });
        self.say_messages(Self::WORDS);
    }

    /// The flight with each day's events and the traffic rolled on
    /// `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self {
            sim: self.sim.with_chance(chance),
            ..self
        }
    }

    /// The flight with shots leaving their particle trails and smoke or
    /// not, as the Smoke Trails preference is on (the default) or off.
    #[must_use]
    pub fn with_smoke_trails(self, smoke_trails: bool) -> Self {
        Self {
            smoke_trails,
            ..self
        }
    }

    /// Sets whether shots leave their particle trails and smoke from the
    /// next step; the puffs already left go on ageing.
    pub fn set_smoke_trails(&mut self, smoke_trails: bool) {
        self.smoke_trails = smoke_trails;
    }

    /// Whether shots leave their particle trails and smoke.
    #[must_use]
    pub fn smoke_trails(&self) -> bool {
        self.smoke_trails
    }

    /// The flight with ships showing their weapon glow or not, as the
    /// Weapon Effects preference is on (the default) or off. Off, the
    /// glows still light and fade; they are only not drawn, as the
    /// original, without the weapon sprites, still sets and decays them.
    #[must_use]
    pub fn with_weapon_effects(self, weapon_effects: bool) -> Self {
        Self {
            weapon_effects,
            ..self
        }
    }

    /// Sets whether ships show their weapon glow.
    pub fn set_weapon_effects(&mut self, weapon_effects: bool) {
        self.weapon_effects = weapon_effects;
    }

    /// Whether ships show their weapon glow.
    #[must_use]
    pub fn weapon_effects(&self) -> bool {
        self.weapon_effects
    }

    /// The flight with the player's sound preferences `prefs`, which say
    /// whether the Red Alert blinks its icon ([`SoundPrefs::alert_quiet`],
    /// [`Session::set_sound_quiet`]).
    #[must_use]
    pub fn with_sound_prefs(mut self, prefs: SoundPrefs) -> Self {
        self.set_sound_prefs(prefs);
        self
    }

    /// Sets the player's sound preferences, as
    /// [`FlightView::with_sound_prefs`] does.
    pub fn set_sound_prefs(&mut self, prefs: SoundPrefs) {
        self.sound_prefs = prefs;
        if let Ok(session) = &mut self.sim.session {
            session.set_sound_quiet(prefs.alert_quiet());
        }
    }

    /// The flight with double time on or off, as `on` says (see the
    /// module docs).
    #[must_use]
    pub fn with_double_time(mut self, on: bool) -> Self {
        self.set_double_time(on);
        self
    }

    /// Turns double time on or off, as `on` says, for the session flown
    /// now and every one after it.
    pub fn set_double_time(&mut self, on: bool) {
        self.double_time = on;
        if let Ok(session) = &mut self.sim.session {
            session.set_double_time_key(on);
        }
        self.refresh_frame();
    }

    /// Whether double time is on, as the screen was last told.
    #[must_use]
    pub fn double_time(&self) -> bool {
        self.double_time
    }

    /// Whether the player's sound is quiet enough for the Red Alert to
    /// blink its icon, as the sound preferences last said.
    #[must_use]
    pub fn alert_quiet(&self) -> bool {
        self.sound_prefs.alert_quiet()
    }
}

/// What reading a flight gives the view as it opens, besides the catalog
/// and the flight: the course map, the scene, the player's sheet and bar,
/// the flight's first frame, the looks and what of them could not be read,
/// the escort menu's colours and the alert icon's width.
struct Opening {
    map: GalaxyMap,
    scene: Option<crate::system::SystemScene>,
    sheet: Result<crate::flight::ShipSheet, String>,
    sheet_ship: Option<nova_sim::ShipId>,
    status_bar: Result<hud::StatusBar, String>,
    bar_government: Option<BarFor>,
    frame: nova_sim::flight_frame::FlightFrame,
    looks: Looks,
    unread_looks: Vec<Diagnostic>,
    escort_colors: crate::flight::escorts::EscortMenuColors,
    alert_width: Option<u32>,
    presentation: super::stepping::Presentation,
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The flight with its explosions and debris rolled on `chance`, apart
    /// from the simulation's.
    #[must_use]
    pub fn with_effects_chance(self, effects_chance: SharedChance) -> Self {
        Self {
            effects_chance,
            ..self
        }
    }

    /// The flight with each scattering of the stars giving them their own
    /// parallax factors or not, as the Parallax Starfield preference is on
    /// (the default) or off.
    #[must_use]
    pub fn with_parallax_starfield(self, parallax_starfield: bool) -> Self {
        Self {
            parallax_starfield,
            ..self
        }
    }

    /// Sets whether the stars get their own parallax factors. It takes
    /// effect at the next scattering; the stars already scattered keep
    /// their factors.
    pub fn set_parallax_starfield(&mut self, parallax_starfield: bool) {
        self.parallax_starfield = parallax_starfield;
    }

    /// Whether the stars' scatterings give them their own parallax factors.
    #[must_use]
    pub fn parallax_starfield(&self) -> bool {
        self.parallax_starfield
    }

    /// The flight with jumps playing their white fades or not, as the
    /// Hyperspace Effects preference is on (the default) or off.
    #[must_use]
    pub fn with_hyperspace_effects(self, hyperspace_effects: bool) -> Self {
        Self {
            hyperspace_effects,
            ..self
        }
    }

    /// Sets whether the next jump plays its white fades; a jump already
    /// playing keeps its own.
    pub fn set_hyperspace_effects(&mut self, hyperspace_effects: bool) {
        self.hyperspace_effects = hyperspace_effects;
    }

    /// Whether jumps play their white fades.
    #[must_use]
    pub fn hyperspace_effects(&self) -> bool {
        self.hyperspace_effects
    }

    /// The flight drawing the port's own `overlays`, the title and the
    /// help line, which are off by default.
    #[must_use]
    pub fn with_overlays(self, overlays: FlightOverlays) -> Self {
        Self { overlays, ..self }
    }

    /// The port's own overlays the flight draws.
    #[must_use]
    pub fn overlays(&self) -> FlightOverlays {
        self.overlays
    }

    /// The catalog the screen reads.
    #[must_use]
    pub fn catalog(&self) -> &C {
        &self.catalog
    }

    /// The view of `sim` over `catalog`, as `opening` read them, every
    /// other field as a view starts.
    #[allow(clippy::too_many_lines)] // one line for each of the screen's fields
    fn assembled(catalog: Rc<C>, sim: S, opening: Opening) -> Self {
        Self {
            catalog,
            map: opening.map,
            map_open: false,
            map_key_held: false,
            map_opened: None,
            map_missions: Vec::new(),
            map_borders_save: None,
            map_seen: None,
            floating_picture: None,
            jump: None,
            fade_in: None,
            death_fade: None,
            hyperspace_effects: true,
            smoke_trails: true,
            parallax_starfield: true,
            weapon_effects: true,
            sound_prefs: SoundPrefs::default(),
            alert_width: opening.alert_width,
            view: VIEW_SIZE,
            double_time: false,
            sim,
            scene: opening.scene,
            kept_frames: KeptFrames::new(),
            sheet: opening.sheet,
            sheet_ship: opening.sheet_ship,
            bar_government: opening.bar_government,
            status_bar: opening.status_bar,
            clock: FixedStep::new(),
            previous: opening.frame.player().place(),
            alpha: 0.0,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
            nav_latch: false,
            select_latch: false,
            slot_block: super::slot_block::SlotBlock::default(),
            jump_latch: false,
            board_latch: false,
            pending_landing: None,
            message: None,
            overlays: FlightOverlays::default(),
            pending_boarding: false,
            pending_hail: None,
            pending_escort_dialog: None,
            pending_mission_info: None,
            pending_player_info: false,
            pending_offer: None,
            pending_leads: Vec::new(),
            glow_rolls: HashedRolls::new(1),
            npc_sheets: BTreeMap::new(),
            npc_previous: Vec::new(),
            unread_looks: opening.unread_looks,
            looks: opening.looks,
            cards: BTreeMap::new(),
            codes: BTreeMap::new(),
            effects: Effects::default(),
            effects_chance: SharedChance::default(),
            stars: crate::flight::stars::Starfield::new(),
            stars_due: true,
            flashes: BTreeMap::new(),
            held_alts: std::cell::RefCell::default(),
            glows: crate::flight::weapon_glow::WeaponGlows::default(),
            shot_looks: super::shots::ShotLooks::default(),
            metrics: None,
            escort_menu: EscortMenu::default(),
            ui_sounds: Vec::new(),
            floating: FloatingMap::new(),
            escort_colors: opening.escort_colors,
            hud_cadence: HudCadence::default(),
            radar_scene: None,
            nav_scene: None,
            nav_flash: None,
            nav_marked: false,
            nav_texts: 0,
            target_scene: None,
            reserves_scene: None,
            bar_steps: VecDeque::new(),
            bars_staged: 0,
            inbox: super::inbox::Inbox::default(),
            step_report: nova_sim::flight_frame::StepReport::default(),
            presentation: opening.presentation,
            frame: opening.frame,
            radar_static: None,
            cursor: HoverCursor::default(),
        }
    }

    /// A view of `sim`, a scripted flight, over `catalog`, for a component
    /// test: the course map read from the catalog, its nebulae and borders
    /// as `rules` say, showing the frame's course; the scene the frame
    /// opens on, read from the catalog with the stellars it shows, each as
    /// it holds it ([`opening_scene`]); with no sheet or status bar, as the
    /// port gives none yet, nor looks.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn scripted(catalog: C, sim: S, rules: &Rulebook) -> Self
    where
        C: GalaxyCatalog + EscortMenuLooks + SystemCatalog,
    {
        const NO_FLIGHT: &str = "a scripted flight has none";
        let mut map = GalaxyMap::course(&catalog);
        map.set_nebula_pick(rules.source_for(nova_sim::RuleKey::NebulaPicture));
        map.set_border_rules(
            rules.source_for(nova_sim::RuleKey::MapBorderPace),
            rules.source_for(nova_sim::RuleKey::MapFindBorders),
        );
        let mut frame = nova_sim::flight_frame::FlightFrame::default();
        sim.frame(&mut frame);
        map.show_course(frame.system, &frame.course_shown);
        map.recentre();
        let escort_colors = catalog.escort_menu_colors();
        let scene = Some(opening_scene(&catalog, &frame));
        Self::assembled(
            Rc::new(catalog),
            sim,
            Opening {
                map,
                scene,
                sheet: Err(NO_FLIGHT.to_owned()),
                sheet_ship: None,
                status_bar: Err(NO_FLIGHT.to_owned()),
                bar_government: None,
                frame,
                looks: Looks::default(),
                unread_looks: Vec::new(),
                escort_colors,
                alert_width: None,
                presentation: super::stepping::Presentation::of(rules),
            },
        )
    }
}

/// The NPCs' sheets, over the [`FlightSim`] port: read from the catalog as
/// the frame's ships and traffic ask for them, so any flight drives them,
/// a scripted one in the component tests.
impl<C: ShipSprites + CombatLooks, S: FlightSim> FlightView<C, S> {
    /// Reads the sheet of each ship type the traffic can spawn
    /// ([`FlightFrame::traffic_ships`](nova_sim::flight_frame::FlightFrame::traffic_ships))
    /// that has not been read yet, of each type a ship here is boxed by
    /// ([`FlightFrame::framed_types`](nova_sim::flight_frame::FlightFrame::framed_types)),
    /// such as the stale frames of a hidden ship's slot, and each NPC's
    /// government's target code, from the frame filled afresh. The
    /// player's own type is read with its ship, so it is left out while
    /// its slot holds its own frames.
    ///
    /// The traffic types' target cards not read yet are asked for in one
    /// batch ([`CombatLooks::target_cards`]), and none when all are read.
    pub(super) fn read_npc_sheets(&mut self) {
        self.refresh_frame();
        let traffic = self.frame.traffic_ships.clone();
        let framed: Vec<_> = self.frame.framed_types().collect();
        let govts: Vec<_> = self.frame.npcs().filter_map(|npc| npc.govt).collect();
        self.read_traffic_sheets(traffic);
        for ship in framed {
            self.npc_sheets
                .entry(ship)
                .or_insert_with(|| self.catalog.ship_sheet(ship));
        }
        for govt in govts {
            self.codes
                .entry(govt)
                .or_insert_with(|| self.catalog.target_code(govt));
        }
    }

    /// Reads, in the warm-up frame of entering play, the sheets and target
    /// cards of the ship types the system's traffic will bring
    /// ([`FlightSim::coming_traffic`], `nova_sim::Session::coming_traffic_ships`),
    /// so the first step, which fills the system, reads none: their reading
    /// is the one-time cost of a run's first stepped frame, and the warm-up
    /// frame is drawn under black. A flight that failed reads nothing.
    pub fn warm_up_traffic(&mut self) {
        let coming = self.sim.coming_traffic();
        self.read_traffic_sheets(coming);
    }

    /// Whether the sheet and the target card of ship type `ship` are read
    /// already, so a step that brings one reads neither.
    #[must_use]
    pub fn has_read_traffic(&self, ship: nova_sim::ShipId) -> bool {
        self.npc_sheets.contains_key(&ship) && self.cards.contains_key(&ship)
    }

    /// Reads the sheet of each of `traffic` not read yet, and their target
    /// cards not read yet in one batch ([`CombatLooks::target_cards`]),
    /// none when all are read.
    fn read_traffic_sheets(&mut self, traffic: Vec<nova_sim::ShipId>) {
        for &ship in &traffic {
            self.npc_sheets
                .entry(ship)
                .or_insert_with(|| self.catalog.ship_sheet(ship));
        }
        let unread: Vec<_> = traffic
            .into_iter()
            .filter(|ship| !self.cards.contains_key(ship))
            .collect();
        if !unread.is_empty() {
            let cards = self.catalog.target_cards(&unread);
            self.cards.extend(unread.into_iter().zip(cards));
        }
    }
}
