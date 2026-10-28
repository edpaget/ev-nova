//! [`FlightView`]'s construction: the generic read of a flight as it
//! opens, the view's halves of the edge's setup and of entering play, the
//! view's own builders and settings, and a test's view over a session it
//! built.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Duration;

#[cfg(any(test, feature = "fixture"))]
use nova_sim::SimCatalog;
use nova_sim::flight_frame::FlightCommand;
use nova_sim::{ALERT_CICN, FixedStep, HashedRolls, Rulebook};

use crate::flight::cadence::HudCadence;
use crate::flight::catalog::{CombatLooks, Looks, ShipSprites, StatusBars};
use crate::flight::cursor::HoverCursor;
use crate::flight::effects::Effects;
use crate::flight::escorts::{EscortMenu, EscortMenuLooks};
use crate::flight::floating::FloatingMap;
use crate::flight::hud::{self};
#[cfg(any(test, feature = "fixture"))]
use crate::flight::sim_adapter::SessionFlight;
use crate::flight::sim_port::FlightSim;
use crate::galaxy::{GalaxyCatalog, GalaxyMap};
use crate::system::camera::VIEW_SIZE;
use crate::system::catalog::SystemCatalog;
use crate::system::kept_frames::KeptFrames;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::DialogTemplate;
use crate::{Diagnostic, SoundPrefs};

use super::{
    BarFor, FlightOverlays, FlightView, Metrics, SharedChance, opening_scene, window_width,
};

/// The view's construction over any flight: [`FlightView::over`] reads it
/// as it opens, through the [`FlightSim`] port, and the edge builds the
/// flight it is handed (`crate::flight::open_flight`). Only the view's own
/// catalogs are asked for: the weapons whose looks are read are the
/// flight's list ([`FlightSim::weapons`]), not a catalog's.
impl<
    C: CombatLooks + SystemCatalog + ShipSprites + StatusBars + GalaxyCatalog + EscortMenuLooks,
    S: FlightSim,
> FlightView<C, S>
{
    /// The view of `sim`, a flight built at the edge, over `catalog`, the
    /// handle the flight shares, shown by `rules` and wording its picks on
    /// `words_chance`, which the edge hands the flight too, so the
    /// wording's picks draw on the flight's own sequence.
    ///
    /// It tells the flight the view's width first
    /// ([`FlightCommand::SetWindowWidth`]), before any frame is read. Then
    /// it reads the flight's first frame: the scene the frame opens on,
    /// the player's sheet and the status bar for the frame's interface
    /// government, and the course map: its nebulae and borders as `rules`
    /// say, the galaxy the flight answers
    /// ([`MapSource::galaxy`](crate::flight::sim_port::MapSource::galaxy)), the
    /// course shown and the current system, centred. The looks of every
    /// weapon the flight lists ([`FlightSim::weapons`]) are read, and what
    /// of them, or of the Red Alert's icon, could not be. A flight that failed to start
    /// ([`FlightSim::failure`]) shows only why: no scene, sheet, bar,
    /// galaxy or looks. No flight starts landed, so flight's first frame
    /// redraws the bar, as `_PlayGame`'s `_RedrawStatus` (@0x457d8) does.
    #[allow(clippy::too_many_lines)] // one line for each of the screen's fields
    pub fn over(catalog: Rc<C>, mut sim: S, rules: &Rulebook, words_chance: SharedChance) -> Self {
        // Sized as the view starts, before anything reads its field.
        sim.apply(FlightCommand::SetWindowWidth(window_width(VIEW_SIZE)));
        let mut frame = nova_sim::flight_frame::FlightFrame::default();
        sim.frame(&mut frame);
        let failure = sim.failure().map(str::to_owned);
        let (scene, sheet, sheet_ship, status_bar) = match &failure {
            None => (
                Some(opening_scene(&*catalog, &frame)),
                catalog.ship_sheet(frame.player().class),
                Some(frame.player().class),
                hud::choose_status_bar(&*catalog, frame.hud.interface_government),
            ),
            Some(reason) => (None, Err(reason.clone()), None, Err(reason.clone())),
        };
        let bar_government = failure
            .is_none()
            .then_some(BarFor(frame.hud.interface_government));
        let mut map = GalaxyMap::course(&*catalog);
        map.set_nebula_pick(rules.source_for(nova_sim::RuleKey::NebulaPicture));
        map.set_border_rules(
            rules.source_for(nova_sim::RuleKey::MapBorderPace),
            rules.source_for(nova_sim::RuleKey::MapFindBorders),
        );
        // A flight that failed to start shows no galaxy.
        if let Some(galaxy) = sim.galaxy() {
            map.show_galaxy(galaxy);
            map.show_course(frame.system, &frame.course_shown);
            // Entering the game draws the floating map, and a new or
            // loaded pilot sets `_mapScroll`, to the current system
            // (`_EnterGameFromMainScreen` @0x19faa, `_DoNewPilot`
            // @0x19054, `_LoadPilotData` @0x7537b).
            map.recentre();
        }
        let looks = match &failure {
            None => Looks::read(&*catalog, sim.weapons()),
            Some(_) => Looks::default(),
        };
        let escort_colors = catalog.escort_menu_colors();
        let alert_width = catalog.icon_size(ALERT_CICN).map(|(width, _)| width);
        let mut unread_looks: Vec<_> = looks
            .problems()
            .into_iter()
            .map(Diagnostic::Unreadable)
            .collect();
        if failure.is_none() && alert_width.is_none() {
            unread_looks.push(Diagnostic::Unreadable(format!(
                "no cïcn {ALERT_CICN}: the Red Alert icon is not drawn"
            )));
        }
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
                presentation: super::stepping::Presentation::of(rules),
                words_chance,
            },
        )
    }
}

/// The view's halves of the edge's setup and of entering play, and its
/// own settings and builders, over any flight.
impl<C: SystemCatalog + ShipSprites + CombatLooks, S: FlightSim> FlightView<C, S> {
    /// Shows the flight as the edge left it once it set the session up
    /// after the view's opening read, as the builders' follow-ups did: the
    /// scene's stellars drawn again by the session's rules, which say how
    /// a destroyed stellar with no `DeadType` sheet draws, but not laid
    /// out again (`with_rules`'), and, when the run's slot sprites were
    /// carried over (`slots_carried`), the NPC sheets their frames ask for
    /// (`with_slot_sprites`'). The kept frame is filled afresh, the cargo
    /// area's labels with it (`with_strings`').
    pub fn show_set_up(&mut self, slots_carried: bool) {
        self.show_destroyed();
        if slots_carried {
            self.read_npc_sheets();
        }
    }

    /// Begins a new pilot's game, `begin` the flight's half (a
    /// `SessionFlight`'s `begin`, on the flight's chance), which says
    /// whether it began: for a pilot just created, before its first save.
    /// The ship, which the beginning puts at (50, 50), is drawn there from
    /// the first frame, not on its way from where the view was read, and
    /// the stars are scattered afresh about it when the flight is next
    /// caught up with, as `_ResetGameData` scatters them (@0x1df5e). The
    /// kept frame is filled afresh after `begin`, as after any change
    /// behind the port ([`FlightView::change_sim`]). A flight that begins
    /// nothing changes nothing else here.
    pub fn begin(&mut self, begin: impl FnOnce(&mut S) -> bool) {
        if self.change_sim(begin) {
            self.place_ship();
            self.stars_due = true;
        }
    }

    /// Enters play from the main screen, `enter` the flight's half (a
    /// `SessionFlight`'s `enter_game`), which says whether it entered: the
    /// stars are scattered afresh when the flight is next caught up with,
    /// as `_EnterGameFromMainScreen` scatters them (@0x19f7c). The kept
    /// frame is filled afresh after `enter` ([`FlightView::change_sim`]).
    /// A flight that enters nothing changes nothing else here.
    pub fn enter_game(&mut self, enter: impl FnOnce(&mut S) -> bool) {
        if self.change_sim(enter) {
            self.stars_due = true;
        }
    }
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The flight the view shows: the edge's way to the adapter's own API
    /// (a `SessionFlight`'s saving and entering play, the pilot it saves),
    /// which the view keeps off the port.
    #[must_use]
    pub fn sim(&self) -> &S {
        &self.sim
    }

    /// Changes the flight the view shows with `change`, through the
    /// adapter's own API ([`FlightView::sim`]): the edge's setup, a
    /// pilot's resume and openings, and a test's change behind the port.
    /// Gives what `change` gives. The kept frame is filled afresh after
    /// it, as after a command ([`FlightView::send`]) or a step, so no read
    /// of the view's finds it stale; what the change raised the view
    /// takes at its next catch-up ([`FlightView::resync`]).
    pub fn change_sim<T>(&mut self, change: impl FnOnce(&mut S) -> T) -> T {
        let changed = change(&mut self.sim);
        self.refresh_frame();
        changed
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
    /// [`FlightCommand::SetSoundQuiet`]).
    #[must_use]
    pub fn with_sound_prefs(mut self, prefs: SoundPrefs) -> Self {
        self.set_sound_prefs(prefs);
        self
    }

    /// Sets the player's sound preferences, as
    /// [`FlightView::with_sound_prefs`] does: the view keeps them, and the
    /// flight is told whether the sound is quiet
    /// ([`FlightCommand::SetSoundQuiet`]), which the Red Alert's blink in
    /// the HUD reads, so it is sent as any command is
    /// ([`FlightView::send`]), the kept frame filled afresh after it.
    pub fn set_sound_prefs(&mut self, prefs: SoundPrefs) {
        self.sound_prefs = prefs;
        self.send(FlightCommand::SetSoundQuiet(prefs.alert_quiet()));
    }

    /// The flight with double time on or off, as `on` says (see the
    /// module docs).
    #[must_use]
    pub fn with_double_time(mut self, on: bool) -> Self {
        self.set_double_time(on);
        self
    }

    /// Turns double time on or off, as `on` says, for the flight flown
    /// now and every one after it: the view keeps it, and the flight is
    /// told the key's state ([`FlightCommand::SetDoubleTimeKey`]), which
    /// it latches at once, sent as any command is ([`FlightView::send`]),
    /// the kept frame filled afresh after it.
    pub fn set_double_time(&mut self, on: bool) {
        self.double_time = on;
        self.send(FlightCommand::SetDoubleTimeKey(on));
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

/// The view of a [`SessionFlight`] that a test built itself, for the
/// tests whose session a builder put into the state they need.
#[cfg(any(test, feature = "fixture"))]
impl<
    C: SimCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks
        + 'static,
> FlightView<C, SessionFlight<C>>
{
    /// The view of `sim` over `catalog`, read as the edge reads a flight
    /// ([`FlightView::over`]) and shown by `rules`, the rulebook its
    /// session was built with (`nova_sim::fixture::SessionBuilder`), so
    /// the view and the session read one: for a test whose session a
    /// builder put into the state it needs, passed as
    /// `builder.build().into()` for Nova's collaborators
    /// ([`SessionFlight::new`]). The catalog is shared with the flight
    /// first, and the wording picks on the flight's chance.
    ///
    /// Wiring-only: for the integration tests that wire a real session
    /// behind the port (`flight_seam`, the stock tests, the edge's
    /// checks). View code and component tests never call it; they open
    /// the view over `FakeFlight`.
    pub fn from_session(catalog: C, mut sim: SessionFlight<C>, rules: &Rulebook) -> Self {
        let catalog = Rc::new(catalog);
        sim.share_catalog(Rc::clone(&catalog) as Rc<dyn SimCatalog>);
        let chance = sim.chance.clone();
        Self::over(catalog, sim, rules, chance)
    }
}

/// What reading a flight gives the view as it opens, besides the catalog
/// and the flight: the course map, the scene, the player's sheet and bar,
/// the flight's first frame, the looks and what of them could not be read,
/// the escort menu's colours, the alert icon's width, the presentation
/// rules and the wording's chance.
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
    words_chance: SharedChance,
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
            words_chance: opening.words_chance,
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
    /// port gives none yet, nor looks; the wording's picks rolled on a
    /// chance that never fires.
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
                words_chance: SharedChance::default(),
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
    /// government's target code, from the kept frame. The
    /// player's own type is read with its ship, so it is left out while
    /// its slot holds its own frames.
    ///
    /// The traffic types' target cards not read yet are asked for in one
    /// batch ([`CombatLooks::target_cards`]), and none when all are read.
    pub(super) fn read_npc_sheets(&mut self) {
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

#[cfg(test)]
mod edge_checks;
