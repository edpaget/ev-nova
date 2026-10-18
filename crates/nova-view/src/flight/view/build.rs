//! [`FlightView`]'s construction: reading the session and catalog, the
//! builders, the hail, player and mission info requests, and the
//! preferences.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::PlayerCommand;
use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
use nova_sim::{
    ALERT_CICN, Allegiance, AsteroidCatalog, Behaviour, BoardingRule, CombatCatalog, CommCatalog,
    DisableRule, FixedStep, Haggle, HailOptions, HailView, HashedRolls, LegalCode, NovaAi,
    NovaBoarding, NovaDisable, NovaLaw, Pilot, PilotCatalog, PointDefenceRule, Rulebook, Session,
    StartError, TrafficCatalog,
};
use nova_sim::{
    ControlBits, Hailed, HireTerms, MissionInfo, MissionInfoPress, OfferPrompt, PersonRules,
    PlayerInfo, ship_layer::SlotSprites,
};

use crate::flight::cadence::HudCadence;
use crate::flight::catalog::{CombatLooks, Looks, ShipSprites, StatusBars};
use crate::flight::cursor::HoverCursor;
use crate::flight::effects::Effects;
use crate::flight::escorts::{EscortMenu, EscortMenuLooks};
use crate::flight::floating::FloatingMap;
use crate::flight::hud::{self};
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::camera::VIEW_SIZE;
use crate::system::catalog::SystemCatalog;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::{Diagnostic, Key, ScreenAction, SoundPrefs};

use super::{
    BarFor, FlightOverlays, FlightView, MESSAGE_TICKS, Metrics, SharedChance, Then,
    hail_refusal_message, opening_scene,
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
> FlightView<C>
{
    /// A new, unnamed pilot's flight, read from `catalog`, which the
    /// screen keeps.
    pub fn new(catalog: C) -> Self {
        let session = Session::start(&catalog);
        Self::flying(catalog, session)
    }

    /// `pilot`'s flight, read from `catalog`, which the screen keeps. A
    /// pilot last landed on a stellar resumes in flight over it, at rest,
    /// as [`Session::fly`] places it: nothing lands, so no landing is
    /// reported ([`FlightView::take_landing`]) and nothing sounds.
    pub fn with_pilot(catalog: C, pilot: Pilot) -> Self {
        let session = Session::fly(&catalog, pilot);
        Self::flying(catalog, session)
    }

    /// `pilot`'s flight as [`FlightView::with_pilot`] builds it, its load's
    /// fallbacks reading `rules` ([`Session::load`]): for a pilot opened
    /// from a save, whose repair [`FlightView::load_repaired`] reports.
    pub fn loaded(catalog: C, pilot: Pilot, rules: Rulebook) -> Self {
        let session = Session::load(&catalog, pilot, &rules);
        Self::flying(catalog, session)
    }

    /// Begins a new pilot's game as [`Session::begin`] does, on the
    /// flight's chance: for a pilot just created, before its first save.
    /// The
    /// ship, which the beginning puts at (50, 50), is drawn there from the
    /// first frame, not on its way from where the view was read. A
    /// session that failed begins nothing.
    pub fn begin(&mut self) {
        if let Ok(session) = &mut self.session {
            session.begin(&self.catalog, &mut self.chance);
            self.place_ship();
        }
    }

    /// Resumes a pilot opened from a save as [`Session::resume`] does, on
    /// the flight's chance: its ship faces a drawn heading, as
    /// `_LoadPilotData` faces every pilot it loads (@0x7535b-0x75372), and
    /// the offers' roll table is drawn, as the load's `_ResetPlayer` draws
    /// it (@0x76502, @0x7673e). A session that failed resumes nothing.
    pub fn resume(&mut self) {
        if let Ok(session) = &mut self.session {
            session.resume(&mut self.chance);
        }
    }

    /// Opens a pilot just loaded on the main screen, as Open Pilot's
    /// `_MainScreenChoice(1)` does once it is loaded: every active
    /// mission's auxiliary ships restart
    /// ([`Session::restart_aux_from_main_screen`], @0x1a918-0x1a982) on
    /// the flight's chance. A session that failed opens nothing.
    pub fn open_on_main_screen(&mut self) {
        if let Ok(session) = &mut self.session {
            session.restart_aux_from_main_screen(&mut self.chance);
        }
    }

    /// Opens the last pilot at launch, once it is loaded, as
    /// `_OpenLastPilotFile` and `_DoMainScreen`'s once-a-launch setup do
    /// ([`Session::open_at_launch`]) on the flight's chance: every active
    /// mission's auxiliary ships restart in the unguarded form
    /// (@0x765d4-0x76637), and the player's system is explored (@0x1affe).
    /// A session that failed opens nothing.
    pub fn open_at_launch(&mut self) {
        if let Ok(session) = &mut self.session {
            session.open_at_launch(&mut self.chance);
        }
    }

    /// The flight of `session`, or why it could not start. No session
    /// starts landed, so flight's first frame redraws the bar, as
    /// `_PlayGame`'s `_RedrawStatus` (@0x457d8) does.
    fn flying(catalog: C, session: Result<Session, StartError>) -> Self {
        Self::read(catalog, session)
    }

    /// The flight of `session` as [`FlightView::flying`] reads it.
    #[allow(clippy::too_many_lines)] // one line for each of the screen's fields
    fn read(catalog: C, session: Result<Session, StartError>) -> Self {
        let session = session.map_err(|err| err.to_string());
        let (scene, sheet, sheet_ship, status_bar) = match &session {
            Ok(session) => (
                Some(opening_scene(&catalog, session)),
                catalog.ship_sheet(session.ship()),
                Some(session.ship()),
                hud::choose_status_bar(&catalog, session.interface_government()),
            ),
            Err(reason) => (None, Err(reason.clone()), None, Err(reason.clone())),
        };
        let bar_government = session
            .as_ref()
            .ok()
            .map(|session| BarFor(session.interface_government()));
        let previous = session
            .as_ref()
            .map(|session| *session.player())
            .unwrap_or_default();
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
            map.show_course(session.system(), &session.course_shown());
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
        Self {
            catalog,
            map,
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
            sound_prefs: SoundPrefs::default(),
            alert_width,
            view: VIEW_SIZE,
            double_time: false,
            session,
            scene,
            sheet,
            sheet_ship,
            bar_government,
            status_bar,
            clock: FixedStep::new(),
            previous,
            alpha: 0.0,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
            nav_latch: false,
            select_latch: false,
            jump_latch: false,
            pending_landing: None,
            message: None,
            overlays: FlightOverlays::default(),
            pending_boarding: false,
            pending_hail: None,
            pending_mission_info: None,
            pending_player_info: false,
            pending_offer: None,
            pending_leads: Vec::new(),
            hail_options: HailOptions::default(),
            boarding_rule: Rc::new(NovaBoarding::default()),
            chance: SharedChance::default(),
            blink_rolls: HashedRolls::new(0),
            glow_rolls: HashedRolls::new(1),
            behaviour: Rc::new(NovaAi::default()),
            disable_rule: Rc::new(NovaDisable),
            defence_rule: Rc::new(Allegiance),
            law: Rc::new(NovaLaw::default()),
            npc_sheets: BTreeMap::new(),
            npc_previous: BTreeMap::new(),
            unread_looks,
            looks,
            cards: BTreeMap::new(),
            codes: BTreeMap::new(),
            effects: Effects::default(),
            effects_chance: SharedChance::default(),
            flashes: BTreeMap::new(),
            glows: crate::flight::weapon_glow::WeaponGlows::default(),
            shot_looks: super::shots::ShotLooks::default(),
            metrics: None,
            escort_menu: EscortMenu::default(),
            ui_sounds: Vec::new(),
            floating: FloatingMap::new(),
            escort_colors,
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
            radar_static: None,
            cursor: HoverCursor::default(),
        }
    }

    /// The flight with every disputed rule reading as `rules` says
    /// ([`Session::with_rules`]). The stellars of the scene shown are
    /// drawn again by the new rules, which say how a destroyed stellar
    /// with no `DeadType` sheet draws, but not laid out again; the course
    /// map's nebulae take their picture by the new
    /// [`RuleKey::NebulaPicture`](nova_sim::RuleKey::NebulaPicture), and
    /// its borders their pace and Find's pass by
    /// [`RuleKey::MapBorderPace`](nova_sim::RuleKey::MapBorderPace) and
    /// [`RuleKey::MapFindBorders`](nova_sim::RuleKey::MapFindBorders).
    #[must_use]
    pub fn with_rules(self, rules: Rulebook) -> Self {
        let pick = rules.source_for(nova_sim::RuleKey::NebulaPicture);
        let pace = rules.source_for(nova_sim::RuleKey::MapBorderPace);
        let find = rules.source_for(nova_sim::RuleKey::MapFindBorders);
        let mut view = Self {
            session: self.session.map(|session| session.with_rules(rules)),
            ..self
        };
        view.map.set_nebula_pick(pick);
        view.map.set_border_rules(pace, find);
        view.show_destroyed();
        view
    }

    /// The flight with its ship slots' draw layers and cloned frames
    /// starting from `slot_sprites`, the history an earlier flight of the
    /// same run left ([`Session::with_slot_sprites`]).
    #[must_use]
    pub fn with_slot_sprites(self, slot_sprites: SlotSprites) -> Self {
        let mut view = Self {
            session: self
                .session
                .map(|session| session.with_slot_sprites(slot_sprites)),
            ..self
        };
        view.read_npc_sheets();
        view
    }

    /// The ship slots' draw-layer and cloned-frame history as the flight
    /// leaves it ([`Session::slot_sprites`]), or `None` when its session
    /// never started.
    #[must_use]
    pub fn slot_sprites(&self) -> Option<&SlotSprites> {
        self.session.as_ref().ok().map(Session::slot_sprites)
    }

    /// The flight with `terms` giving the fee and wage of a hire
    /// ([`Session::with_hire_terms`]).
    #[must_use]
    pub fn with_hire_terms(self, terms: Rc<dyn HireTerms>) -> Self {
        Self {
            session: self.session.map(|session| session.with_hire_terms(terms)),
            ..self
        }
    }

    /// The flight with `rules` deciding how persons appear
    /// ([`Session::with_person_rules`]).
    #[must_use]
    pub fn with_person_rules(self, rules: Rc<dyn PersonRules>) -> Self {
        Self {
            session: self.session.map(|session| session.with_person_rules(rules)),
            ..self
        }
    }

    /// The flight with the `T` set operator naming the ship from
    /// `strings`' string lists ([`Session::with_strings`]).
    #[must_use]
    pub fn with_strings(self, strings: Rc<dyn CommCatalog>) -> Self {
        Self {
            session: self.session.map(|session| session.with_strings(strings)),
            ..self
        }
    }

    /// The flight with `bits` testing a ship's `Availability` for hire
    /// ([`Session::with_control_bits`]).
    #[must_use]
    pub fn with_control_bits(self, bits: Rc<dyn ControlBits>) -> Self {
        Self {
            session: self.session.map(|session| session.with_control_bits(bits)),
            ..self
        }
    }

    /// The flight with its explosions and debris rolled on `chance`, apart
    /// from the simulation's.
    #[must_use]
    pub fn with_effects_chance(self, effects_chance: SharedChance) -> Self {
        Self {
            effects_chance,
            ..self
        }
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
        Self { behaviour, ..self }
    }

    /// The flight with its ships disabled as `rule` says: in the fight,
    /// and in its session's changes of ship
    /// ([`Session::with_disable_rule`]).
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            session: self
                .session
                .map(|session| session.with_disable_rule(Rc::clone(&disable_rule))),
            disable_rule,
            ..self
        }
    }

    /// The flight with its ships' point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            defence_rule,
            ..self
        }
    }

    /// The flight with the player's crimes judged by `law`.
    #[must_use]
    pub fn with_law(self, law: Rc<dyn LegalCode>) -> Self {
        Self { law, ..self }
    }

    /// The flight with its boardings by `rule`: who repels boarders, the
    /// capture odds and the capture roll.
    #[must_use]
    pub fn with_boarding_rule(self, boarding_rule: Rc<dyn BoardingRule>) -> Self {
        Self {
            boarding_rule,
            ..self
        }
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

    /// The flight with the comm dialog listing `hail_options`.
    #[must_use]
    pub fn with_hail_options(self, hail_options: HailOptions) -> Self {
        Self {
            hail_options,
            ..self
        }
    }

    /// Hails the target ([`Session::hail_key`]), Option held as Alt is,
    /// or says why not: a hail answered is held for the router
    /// ([`FlightView::take_hail`]), and a mission offered in its place
    /// too ([`FlightView::take_offer`]), each letting go of the flight
    /// keys; one accepted at once shows nothing, and so does a navigation
    /// stellar that answers, as its comm dialog is not built.
    pub(super) fn hail(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        let option = self.held.contains(&Key::Alt);
        match session.hail_key(&self.catalog, &self.hail_options, option, &mut self.chance) {
            Ok(Hailed::Comm(view)) => {
                self.pending_hail = Some(view);
                self.held.clear();
            }
            Ok(Hailed::Offer(offer)) => self.hold_offer(offer),
            // The planet comm dialog (`_DoPlanetCommDialog`) is not built
            // yet: rdm:task/planet-domination opens it here.
            Ok(Hailed::Stellar(_)) => {}
            Err(refusal) => {
                if let Some(text) = hail_refusal_message(refusal) {
                    self.say_for(text, MESSAGE_TICKS);
                }
            }
        }
    }

    /// The hail just answered, once: the router takes it to open the comm
    /// dialog.
    pub fn take_hail(&mut self) -> Option<HailView> {
        self.pending_hail.take()
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen since
    /// this was last taken, once: the router opens it on
    /// [`FlightView::player_info`] over the paused flight.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_player_info)
    }

    /// What the player info screen shows, as [`Session::player_info`]
    /// gives it from the screen's catalog, changing nothing; `None` for a
    /// session that failed. An opening of the screen is
    /// [`FlightView::open_player_info`].
    #[must_use]
    pub fn player_info(&self) -> Option<PlayerInfo> {
        let session = self.session.as_ref().ok()?;
        Some(session.player_info(&self.catalog))
    }

    /// The player info dialog opens, as [`Session::open_player_info`] has
    /// it: the session resets first, so the screen it gives shows the
    /// cargo trimmed; none in a jump or while the ship breaks up, when
    /// nothing opens, or for a session that failed. The router opens the
    /// screen on it, from flight and from the spaceport alike.
    pub fn open_player_info(&mut self) -> Option<PlayerInfo> {
        let session = self.session.as_mut().ok()?;
        session.open_player_info(&self.catalog)
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
        match &mut self.session {
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
            if let Ok(session) = &mut view.session {
                session.jettison_cargo(true, &mut view.chance);
            }
        });
        self.say_messages();
    }

    /// The mission info [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY) asked for the mission info
    /// screen on, once: the router opens the screen on it over the paused
    /// flight.
    pub fn take_mission_info_request(&mut self) -> Option<MissionInfo> {
        self.pending_mission_info.take()
    }

    /// Presses [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), as `_HandlePlayer` does
    /// (@0x70405-0x705a8), the session deciding
    /// ([`Session::press_mission_info`]): with an active mission shown,
    /// asks the router for the mission info screen, letting go of the
    /// flight keys; with none, says the session's #354 on the message
    /// line after its refusal beep; refused, nothing. A flight whose
    /// session never started says #354 alone, with no session to beep.
    pub(super) fn ask_mission_info(&mut self) {
        let Ok(session) = self.session.as_mut() else {
            self.say_for(NO_ACTIVE_MISSIONS, MESSAGE_TICKS);
            return;
        };
        match session.press_mission_info() {
            MissionInfoPress::Refused => {}
            MissionInfoPress::NoActiveMissions => self.say_messages(),
            MissionInfoPress::Opens(info) => {
                self.pending_mission_info = Some(info);
                self.held.clear();
            }
        }
    }

    /// The mission a person hailed or boarded just offered, once: the
    /// router takes it to open the offer dialog over the paused flight,
    /// and answers it through [`FlightView::accept_mission`] or
    /// [`FlightView::refuse_mission`].
    pub fn take_offer(&mut self) -> Option<OfferPrompt> {
        self.pending_offer.take()
    }

    /// The comm dialog's contents while a hail is under way
    /// ([`Session::hailing`]).
    #[must_use]
    pub fn hailing(&self) -> Option<HailView> {
        self.session
            .as_ref()
            .ok()?
            .hailing(&self.catalog, &self.hail_options)
    }

    /// Presses the `pick`th option listed, through the session
    /// ([`Session::answer`]).
    pub fn answer(&mut self, pick: usize) -> Option<HailView> {
        let session = self.session.as_mut().ok()?;
        session.answer(pick, &self.catalog, &self.hail_options, &mut self.chance)
    }

    /// Makes `choice` in the haggle dialog, through the session
    /// ([`Session::haggle`]).
    pub fn haggle(&mut self, choice: Haggle) -> Option<HailView> {
        let session = self.session.as_mut().ok()?;
        session.haggle(choice, &self.catalog, &self.hail_options)
    }

    /// Reads the sheet of each ship type the traffic can spawn that has
    /// not been read yet, and of each type a ship here is boxed by
    /// ([`Session::framed_types`]), such as the stale frames of a hidden
    /// ship's slot.
    pub(super) fn read_npc_sheets(&mut self) {
        let Ok(session) = &self.session else {
            return;
        };
        for ship in session.traffic_ships() {
            self.npc_sheets
                .entry(ship)
                .or_insert_with(|| self.catalog.ship_sheet(ship));
            self.cards
                .entry(ship)
                .or_insert_with(|| self.catalog.target_card(ship));
        }
        for ship in session.framed_types() {
            self.npc_sheets
                .entry(ship)
                .or_insert_with(|| self.catalog.ship_sheet(ship));
        }
        for govt in session.npcs().iter().filter_map(|npc| npc.govt) {
            self.codes
                .entry(govt)
                .or_insert_with(|| self.catalog.target_code(govt));
        }
    }

    /// The flight with each day's events and the traffic rolled on
    /// `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
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
        if let Ok(session) = &mut self.session {
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
        if let Ok(session) = &mut self.session {
            session.set_double_time_key(on);
        }
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

    /// The catalog the screen reads.
    #[must_use]
    pub fn catalog(&self) -> &C {
        &self.catalog
    }
}
