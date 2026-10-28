//! [`FlightView`]'s travel: fades and death, the course map, the jump and
//! its steps, arrival, take-off, landing, the shipyard and outfitter
//! orders.

#[cfg(doc)]
use nova_sim::Session;
use nova_sim::SessionEvent;
use nova_sim::gate::next_gate_pick;

use nova_sim::flight_frame::{
    Answer, FlightCommand, MapClick, MapOpen, MapScreen, ScreenAnswer, ScreenAsk,
};

use crate::devtools::{DevDesk, FlightDesk};
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::sim_port::FlightSim;
use crate::galaxy::{MapCycle, MapMode, PendingPress, PressAnswer};
use crate::system::catalog::SystemCatalog;
use crate::view_shape::classic_offset;
use crate::{DrawList, Input, Key, Point, Screen, UiSound};

use super::dialog_map::{MapOpened, MapOpening};
use super::{BOARD_KEY, FlightView, JUMP_KEY, MAP_KEY, Redrawn};

impl<C, S: FlightSim + DevDesk> FlightView<C, S> {
    /// The pilot flying, as the developer tools' desk: its reading and
    /// edits are the flight's own desk's ([`DevDesk`]), and the course
    /// map's galaxy names the places. `None` for a flight that could not
    /// start ([`FlightView::failure`]).
    ///
    /// The desk's edits change the flight behind the port, and the kept
    /// frame isn't filled afresh after them here: its caller resyncs the
    /// view after every edit ([`FlightView::resync`], whose catch-up fills
    /// it), as the app does (`App::pilot_desk_frame` runs
    /// `AppScreen::after_pilot_edit`, whose first act is the resync).
    pub fn pilot_desk(&mut self) -> Option<FlightDesk<'_, S>> {
        if self.failure.is_some() {
            return None;
        }
        Some(FlightDesk::new(&mut self.sim, self.map.model()))
    }
}

/// The course map, over the [`FlightSim`] port: its opening, drawing,
/// panels, input and Clear Route read the flight through the kept frame
/// ([`FlightView::frame`](FlightView#structfield.frame)) and the screen
/// query ([`FlightSim::screen`]: the opening, each redraw, the panels
/// after a selection change, a cycle and Find, each asked once when it
/// happens, never per frame), and send their commands through it. Its close is generic too
/// ([`FlightView::close_map`], in the `gates` module), the hypergate
/// entry it may run a command ([`FlightCommand::EnterHypergate`]).
/// [`FlightView::map_input_taken`] says when a close is due.
impl<C, S: FlightSim> FlightView<C, S> {
    /// The keys `_HandlePlayer` polls with a latch, once a frame, with
    /// those held down, in its order: Option-\` and the Nav Off /
    /// Hyperspace Mode block ([`FlightView::nav_keys_frame`]), Hyper
    /// Select ([`FlightView::select_frame`]), the stellar slot keys
    /// ([`FlightView::slot_keys_frame`]), the Board key
    /// ([`FlightView::board_frame`]), then the jump key
    /// ([`FlightView::jump_frame`]), each while the flight takes its
    /// command. A key held across a jump's end, a release of Option or a
    /// repair acts here, as a press would, on the first frame its latch
    /// lets it.
    ///
    /// It runs before the tick's steps, as `_HandlePlayer` runs before the
    /// ships' `_HandleShip` (@0x43e19, @0x44483): a repair made in one
    /// tick's steps is acted on at the next, the frame the original sees
    /// it. And it runs before the jump effect's arrival, which is where
    /// the original's arrival frame runs its keys (+0x50 still above 0):
    /// the arrival tick still counts as jumping, and the first tick after
    /// it is the original's first frame to act (@0x6c625).
    pub(super) fn held_keys_frame(&mut self) {
        let down = |key: Key| self.held.contains(&key);
        let (jump, board) = (down(JUMP_KEY), down(BOARD_KEY));
        self.nav_held_frame();
        // The Board key's and the jump's frames ask their own gates
        // (`FlightCommand::gate`).
        self.board_frame(board);
        self.jump_frame(jump);
    }

    /// The course map's close, the hypergate's entry left to
    /// `enter_gate`, handed the system picked on the hypergate map
    /// ([`FlightView::close_map`]): the borders' save, the map shut,
    /// then the course map's close, docked or in flight, or the hypergate
    /// map's redraw before `enter_gate` runs.
    ///
    /// Closing the course map in flight cues the course's next hop
    /// ([`FlightCommand::CueCourse`]), as the original's map does unless
    /// it is the hypergate's (`_DoSystemMap` @0x13c16), and beeps
    /// [`UiSound::MapClosed`], as `_HandlePlayer` does once
    /// `_DoSystemMap` returns (@0x6a85a).
    ///
    /// Docked, its close is `_DoSystemMap`'s in port: the whole bar is
    /// redrawn (`_RefreshSAT` @0x13bc9), showing the nav as the map left
    /// it, and the course is not cued (@0x13be2-0x13c0a skips
    /// `_CueNextHyperRouteDest` in port). Then the selection the map
    /// opened with is put back, the mode kept, with no redraw
    /// ([`FlightCommand::RestoreMapSelection`], `_DoSystemMap` @0x12349,
    /// @0x13bee-0x13c06), undoing a Clear Route. It beeps nothing: the
    /// [`UiSound::MapClosed`] beep is `_HandlePlayer`'s, around the map M
    /// opens in flight. A mission dialog's map beeps in flight neither.
    ///
    /// Then whoever opened it puts back what it saved: when the bar opened
    /// it ([`FlightView::open_bar_map`]), the nav the bar opened with
    /// ([`FlightCommand::RestoreBarNav`], `_DoBarDialog` @0x492b7-0x492d3),
    /// with no redraw, as `_UpdateAllPortDialogs` (@0x492d7) draws no
    /// status, so the nav area shows the map's selection until the port's
    /// next nav redraw, such as the bar's close; a port screen its own
    /// (`FlightView::open_port_screen_map`), with no redraw either: the
    /// exchange and the outfitter the nav they opened with, and the
    /// spaceport, the shipyard, the hire screen and the ship description
    /// the nav as the map opened; a mission dialog its own
    /// (`FlightView::open_dialog_map`): the offer's, the computer's and
    /// the briefing's the nav as the map opened, docked or in flight, the
    /// computer's then redrawing the nav area, and mission info's nothing.
    pub(super) fn close_map_then(
        &mut self,
        enter_gate: impl FnOnce(&mut Self, Option<nova_sim::SystemId>),
    ) {
        // `_SaveMapBordersState` for a change, whatever the map's mode,
        // unless docked (@0x13c21-0x13c3f).
        let docked = self.frame.docked();
        if !docked && self.map.borders_changed_since_open() {
            self.map_borders_save = Some(self.map.borders_shown());
        }
        self.map.cancel_pointer();
        self.map.release_keys();
        self.map.close_find();
        self.map_open = false;
        self.map_key_held = false;
        let opened = self.map_opened.take();
        if self.map.mode() != MapMode::Hypergate {
            if docked {
                self.redraw_status();
                if let Some(nav) = opened.and_then(|opened| opened.nav) {
                    self.send(FlightCommand::RestoreMapSelection(nav));
                }
                self.restore_opener_nav(opened);
                return;
            }
            let by_dialog =
                opened.is_some_and(|opened| matches!(opened.by, MapOpening::Dialog(..)));
            if !by_dialog {
                self.ui_sounds.push(UiSound::MapClosed);
            }
            self.send(FlightCommand::CueCourse);
            self.redraw_status();
            self.restore_opener_nav(opened);
            return;
        }
        self.redraw_status();
        let choice = self.map.take_gate_choice();
        enter_gate(self, choice);
    }

    /// Catches the screen up with what the flight has told since this
    /// was last called ([`FlightEvent::Session`]), and gives what it
    /// redrew. Several events of a kind ask for one reaction, decided by
    /// the events alone and never by what the flight was last asked to
    /// do: a change to the galaxy draws the flight's galaxy on the course
    /// map with the course as it is flown ([`FlightView::catch_up_then`]);
    /// a change of ship reads the player's sprite sheet afresh
    /// (`FlightView::follow_ship`); a system entered afresh is laid out
    /// (`FlightView::lay_out`); otherwise a move to another stellar of
    /// the system shown draws the ship there ([`FlightView::place_ship`]);
    /// and a rebuild point that changed which stellars are shown, with no
    /// system entered, reads the scene again with them
    /// ([`FlightView::redraw_scene_then`], with the status bar's follow
    /// and redraw, [`FlightView::follow_bar`] and
    /// [`FlightView::redraw_status`]).
    ///
    /// The only place the session events are taken: each frame calls it
    /// first, so a change made outside the view's own methods (a
    /// developer tools' edit, a set expression a control bit runs) is
    /// drawn by the next frame; and each of the view's methods that drives
    /// the flight between frames calls it last, so its caller sees the
    /// screen caught up at once. Taking the events again finds none. A
    /// flight that failed drains nothing, and its frame is the default.
    ///
    /// [`FlightEvent::Session`]: nova_sim::flight_frame::FlightEvent::Session
    pub(super) fn catch_up(&mut self) -> Redrawn
    where
        C: SystemCatalog + ShipSprites + CombatLooks + StatusBars,
    {
        self.catch_up_then(|view, redrawn| {
            view.redraw_scene_then(redrawn, Self::follow_bar, Self::redraw_status);
        })
    }

    /// Catches the screen up with an edit made through the pilot desk now,
    /// rather than on the next frame, from the flight's events (see the
    /// module docs): a move to another system is laid out as an arrival
    /// is, with no message, and one to another stellar of the system shown
    /// draws the ship there, the course map showing the flight's course
    /// again. Nothing is read when nothing moved.
    pub fn resync(&mut self)
    where
        C: SystemCatalog + ShipSprites + CombatLooks + StatusBars,
    {
        self.catch_up();
    }

    /// [`FlightView::catch_up`]'s work over the port, with `redraw` the
    /// redraws that still read the session ([`FlightView::redraw_scene_then`]
    /// for a session's flight): drains the flight's session events
    /// ([`FlightEvent::Session`]) and folds them into what they ask for,
    /// several of a kind asking for one reaction. A change to the galaxy
    /// draws the flight's galaxy on the course map, asked afresh
    /// ([`ScreenAsk::Galaxy`],
    /// [`GalaxyMap::show_galaxy`](crate::galaxy::GalaxyMap::show_galaxy))
    /// with the course as it is flown, before anything else is drawn, only
    /// reads coming between. Then `redraw` runs, and last the open map's
    /// drawing follows the galaxy, the explored and known sets, the course
    /// and the nav ([`FlightView::redraw_map`]): the flight is paused while
    /// it is open, so this is rare, a set expression or the developer
    /// tools. Gives what was redrawn.
    ///
    /// [`FlightEvent::Session`]: nova_sim::flight_frame::FlightEvent::Session
    pub(super) fn catch_up_then(&mut self, redraw: impl FnOnce(&mut Self, Redrawn)) -> Redrawn {
        let mut redrawn = Redrawn::default();
        let events = self.drain().take_events();
        // The kept frame afresh before anything redraws from it.
        self.refresh_frame();
        for event in events {
            match event {
                SessionEvent::ShipChanged => redrawn.ship = true,
                SessionEvent::SystemChanged => redrawn.system = true,
                SessionEvent::StellarChanged => redrawn.stellar = true,
                SessionEvent::GalaxyChanged => redrawn.galaxy = true,
                SessionEvent::StellarsRebuilt => redrawn.rebuilt = true,
                SessionEvent::StarfieldReset => redrawn.stars = true,
            }
        }
        if redrawn.galaxy {
            if let ScreenAnswer::Galaxy(galaxy) = self.sim.screen(ScreenAsk::Galaxy) {
                self.map.show_galaxy(&galaxy);
            }
            self.show_course();
        }
        // A redraw changes the flight only through a command, which fills
        // the kept frame afresh, so it needs no fill after it.
        redraw(self, redrawn);
        if self.map_open && (redrawn.galaxy || self.map_seen.as_ref() != Some(&self.frame.map_key))
        {
            self.redraw_map();
        }
        redrawn
    }

    /// Shows the course as it is flown on the course map, from the
    /// current system ([`FlightFrame::course_shown`](nova_sim::flight_frame::FlightFrame::course_shown)).
    pub(super) fn show_course(&mut self) {
        self.map
            .show_course(self.frame.system, &self.frame.course_shown);
    }

    /// The input `input` as the course map takes it: in its own
    /// 1024 x 768 frame, which a widened view over flight draws centred,
    /// moved only across ([`FlightView::map_offset`]).
    pub(super) fn map_frame_input(&self, input: &Input) -> Input {
        let offset = self.map_offset();
        input.translated(Point::new(-offset.x, 0.0))
    }

    /// [`FlightView::map_input`]'s work, with `close` the map's close
    /// ([`FlightView::close_map`] for a session's flight), run at once
    /// when the input closes the map ([`FlightView::map_input_taken`]).
    /// Then a click, a Tab cycle or a gate pick that changed the
    /// selection takes the panels again ([`FlightView::retake_map_panel`]),
    /// as the original's next `_MapDialogUpdate` draws them for its
    /// `_selSystem`. After an input that closed the map the panels are
    /// taken for nothing: the map isn't drawn until it opens again, which
    /// takes them.
    pub(super) fn map_input_then(&mut self, input: &Input, close: impl FnOnce(&mut Self)) {
        let selected = self.map.selected();
        let input = self.map_frame_input(input);
        if self.map_input_taken(&input) {
            close(self);
        }
        if self.map.selected() != selected {
            self.retake_map_panel();
        }
    }

    /// The open course map, drawn into `list`: its 1024 x 768 frame moved
    /// right by [`FlightView::map_offset`].
    pub(super) fn draw_open_map(&self, list: &mut DrawList) {
        let mut map = DrawList::new();
        self.map.draw(&mut map);
        list.extend_translated(&map, self.map_offset());
    }

    /// How far right the course map's 1024 x 768 frame is drawn, and its
    /// clicks taken: centred in the widened view over flight
    /// ([`classic_offset`]), and at 0 over the spaceport, which is
    /// classic, where the bar and the docked mission dialogs open it.
    pub(super) fn map_offset(&self) -> Point {
        let in_port = self.frame.docked();
        let x = if in_port {
            0.0
        } else {
            classic_offset(self.view.0)
        };
        Point::new(x, 0.0)
    }

    /// Lets go of the flight keys and shows the course map, with the
    /// system the original's `_DoSystemMap` selects as it opens
    /// (`Session::map_preselection`, with the rest of its opening,
    /// [`ScreenAsk::MapOpening`]) selected, and the system selected in nav
    /// mode 3 drawn as its link. The hypergate map opens this way too, its
    /// pick still only an offered system.
    ///
    /// Clear Route is enabled off the hypergate map with a course
    /// (`showClearRouteButton`, @0x12396-0x123c3), and never on it.
    pub(super) fn open_map(&mut self) {
        self.open_map_as(MapOpening::Plain);
    }

    /// [`FlightView::open_map`], opened `by` whoever opens it (flight, the
    /// bar, a port screen or a mission dialog), which gives the map its
    /// marks ([`FlightView::open_dialog_map`]) and its close's restore, and
    /// the nav noted as it opens (`_DoSystemMap` @0x12349): all of it, with
    /// the borders, the preselection and the mission systems, one answer
    /// to the opening's ask ([`ScreenAsk::MapOpening`]). A flight that
    /// answers it nothing (one that failed to start) opens the map with no
    /// nav noted, the marks the opener gives itself
    /// ([`MarksFrom::unanswered`](nova_sim::flight_frame::MarksFrom::unanswered)),
    /// no borders shown, nothing preselected and no mission systems.
    pub(super) fn open_map_as(&mut self, by: MapOpening) {
        let marks = by.marks_from();
        let open = match self.sim.screen(ScreenAsk::MapOpening { marks }) {
            ScreenAnswer::MapOpening(open) => Some(open),
            _ => None,
        };
        let (nav, marks, borders, preselection, missions) = match open {
            Some(MapOpen {
                nav,
                marks,
                borders,
                preselection,
                missions,
            }) => (Some(nav), marks, Some(borders), preselection, missions),
            None => (None, marks.unanswered(), None, None, Vec::new()),
        };
        self.map_opened = Some(MapOpened { by, nav, marks });
        self.held.clear();
        self.map_open = true;
        if let Some(sources) = borders {
            self.map.show_border_sources(sources);
        }
        self.map.opened();
        self.map.preselect(preselection);
        self.map_missions = missions;
        self.map
            .set_clear_route(self.map.mode() != MapMode::Hypergate && self.frame.course_plotted);
        self.redraw_map();
    }

    /// Takes the session's drawing onto the course map
    /// ([`Session::map_drawing`]), as `_MapDialogUpdate` runs `_DrawMap`:
    /// on the hypergate map with the gate's offers arrowed, each of which
    /// that draw marks known ([`Session::mark_known`], `_DrawMap`
    /// @0xf457-0xf45f). So the first draw of the hypergate map shows an
    /// offer the player knew nothing of without its oval, and the next one,
    /// at a Tab, a click or the course map opened again before the next
    /// rebuild point, shows it, as the original's next `_MapDialogUpdate`
    /// does. What the drawing was taken from is noted after the mark
    /// ([`MapKey`](nova_sim::flight_frame::MapKey)), so the mark itself
    /// doesn't take it again. The borders and the panels for the system
    /// selected are taken with it, all one answer to the redraw's ask
    /// ([`ScreenAsk::Map`]), the hypergate map's borders counting the
    /// arrows known, as the mark leaves them. A flight that answers it
    /// nothing redraws nothing.
    pub(super) fn redraw_map(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let hypergate = offered.is_some();
        let ScreenAnswer::Map(screen) = self.sim.screen(ScreenAsk::Map {
            hypergate: offered,
            missions: self.map_missions.clone(),
            marks: MapOpened::marks(self.map_opened.as_ref()),
            selected: self.map.selected(),
        }) else {
            return;
        };
        let MapScreen {
            drawing,
            borders,
            panel,
        } = *screen;
        self.map.show_drawing(&drawing);
        if hypergate {
            self.send(FlightCommand::MarkKnown(drawing.gate_arrows));
        }
        self.map.show_border_sources(borders);
        self.map_seen = Some(self.frame.map_key.clone());
        self.map.show_panel(panel);
    }

    /// Takes the session's info panels onto the course map
    /// ([`Session::map_panel`], [`ScreenAsk::MapPanel`]) for the system it
    /// selects, as `_MapDialogUpdate` draws them after `_DrawMap`: on the
    /// hypergate map with the gate's offers, which title a destination.
    pub(super) fn retake_map_panel(&mut self) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        if let ScreenAnswer::MapPanel(panel) = self.sim.screen(ScreenAsk::MapPanel {
            selected: self.map.selected(),
            hypergate: offered,
        }) {
            self.map.show_panel(*panel);
        }
    }

    /// Opens the course map on a press of [`MAP_KEY`] in flight, held from
    /// then on. `_HandlePlayer`'s `_beepSnd[2]` (@0x6a809) is the key's,
    /// not the map's: the hypergate's map opens without it.
    pub(super) fn open_map_by_key(&mut self) {
        self.ui_sounds.push(UiSound::MapOpened);
        self.open_map();
        self.map_key_held = true;
    }

    /// Opens the course map from the bar, as `_DoBarDialog`'s item 8 does
    /// (@0x492a3-0x492b2), over the spaceport: in
    /// [`MapMode::Course`](crate::galaxy::MapMode::Course), as the bar
    /// clears the hypergate flag first (@0x492aa-0x492af). Ours' map is
    /// in that mode whenever it is closed, as the hypergate map's close
    /// takes its pick ([`GalaxyMap::take_gate_choice`](crate::galaxy::GalaxyMap::take_gate_choice)). Its close is
    /// [`FlightView::close_map`]'s in port.
    pub fn open_bar_map(&mut self) {
        self.open_map_as(MapOpening::Bar);
    }

    /// [`FlightView::map_input`]'s input, translated into the map's frame,
    /// before the panels follow the selection: whether it closes the map,
    /// which the caller then does ([`FlightView::close_map`]), at once.
    /// A key-down the filter doesn't take with [`MAP_KEY`] held closes it
    /// before anything else; Return or Done after the rest.
    pub(super) fn map_input_taken(&mut self, input: &Input) -> bool {
        if let Input::Key {
            key: MAP_KEY,
            pressed,
            ..
        } = *input
        {
            self.map_key_held = pressed;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && self.map_key_held
            && !self.map.finding()
            && !self.map_filter_takes(key)
        {
            return true;
        }
        self.map.input(input);
        self.ui_sounds.extend(self.map.take_ui_sounds());
        if let Some(query) = self.map.take_find()
            && let ScreenAnswer::Found(found) = self.sim.screen(ScreenAsk::Find(query))
        {
            self.map.show_search(found);
            self.ui_sounds.extend(self.map.take_ui_sounds());
        }
        if let Some(press) = self.map.take_press() {
            self.answer_map_press(&press);
        }
        if self.map.take_clear_route() {
            self.clear_route();
        }
        if let Some(cycle) = self.map.take_cycle() {
            self.cycle_map(cycle);
        }
        self.map.take_close()
    }

    /// Answers a press on the map that hit systems
    /// ([`GalaxyMap::take_press`](crate::galaxy::GalaxyMap::take_press)) as
    /// `_DoSystemMap` takes item 3 ([`Session::map_click`]): the hypergate
    /// map passes the systems the gate offers, and every map the mission
    /// systems taken as it opened. With no session, or no hit accepted,
    /// the map drags. An accepted press draws the map again, as the
    /// original invalidates its items 2, 3 and 6 (@0x1315c-0x13259), so the
    /// hypergate map's offers show known; the course changed is shown, and
    /// the nav area is redrawn at once when the click wrote it
    /// (`_DrawStatus`), as [`FlightView::cycle_map`] does.
    fn answer_map_press(&mut self, press: &PendingPress) {
        let offered = (self.map.mode() == MapMode::Hypergate).then(|| self.map.offered().to_vec());
        let answer = self.send(FlightCommand::MapClick(MapClick {
            hits: press.hits.clone(),
            shift: press.shift,
            hypergate: offered,
            missions: self.map_missions.clone(),
            green: MapOpened::marks(self.map_opened.as_ref()).green,
        }));
        let clicked = match answer {
            Answer::Clicked(clicked) => Some(clicked),
            _ => None,
        };
        self.map.answer_press(clicked.map(|clicked| PressAnswer {
            selected: clicked.selected,
            gate_pick: clicked.gate_pick,
            clear_route: clicked.clear_route,
        }));
        let Some(clicked) = clicked else {
            return;
        };
        if clicked.course_changed {
            self.show_course();
        }
        self.redraw_map();
        if clicked.nav_redraw {
            self.retake_nav();
        }
    }

    /// Clear Route, from its button or C (`_DoSystemMap` item 8,
    /// @0x12c69-0x12dbf): the session's course cleared and its selection
    /// let go ([`Session::clear_route`]), Clear Route disabled and the
    /// hypergate map's pick dropped
    /// ([`GalaxyMap::route_cleared`](crate::galaxy::GalaxyMap::route_cleared)),
    /// the map drawn again and the nav area redrawn at once (`_DrawStatus`
    /// @0x12dba).
    fn clear_route(&mut self) {
        self.map.route_cleared();
        if !self.send(FlightCommand::ClearRoute).is_refused() {
            self.show_course();
        }
        self.redraw_map();
        self.retake_nav();
    }

    /// Whether `_MapFilter` takes `key` before its map-key close, which a
    /// key it takes never reaches (@0x13cdd-0x144d6): Return, Enter,
    /// Escape, Tab and `\` (off a planet, @0x13d20), Space, B, C, `-`,
    /// `=` and `+` (the keypad's too), the arrows, and F and S. The keys
    /// later phases port are taken already, so none of them closes the
    /// map. A modifier is taken too: a Mac sends no key-down for one
    /// alone, so the filter never sees it.
    fn map_filter_takes(&self, key: Key) -> bool {
        let docked = self.frame.docked();
        match key {
            Key::Shift
            | Key::Control
            | Key::Alt
            | Key::Command
            | Key::CapsLock
            | Key::Enter
            | Key::Escape
            | Key::Space
            | Key::Left
            | Key::Right
            | Key::Up
            | Key::Down => true,
            Key::Tab => !docked,
            Key::Char(c) => match c.to_ascii_lowercase() {
                '\\' => !docked,
                'b' | 'c' | '-' | '=' | '+' | 'f' | 's' => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// A Tab or `\` on the map ([`MapCycle`]), as `_MapFilter` takes it
    /// off a planet (@0x13d0c-0x140d8). On the hypergate map, the map's
    /// selection steps to the next of the pending hypergate's
    /// destinations ([`Session::hypergate_cycle`],
    /// [`next_gate_pick`](nova_sim::gate::next_gate_pick)), back with
    /// Shift, held as it is so a hidden one steps on too; no nav mode is
    /// written and the nav area isn't redrawn, as the original's mode 4 is
    /// never drawn (@0x13dd8-0x13ebd). A pick redraws the map's item 3
    /// ([`GalaxyMap::redraw`](crate::galaxy::GalaxyMap::redraw),
    /// @0x13e6f); with no destination to pick nothing is invalidated
    /// (`jle` @0x13daf). On the course map, the session cycles the
    /// hyperspace selection ([`Session::cycle_map_selection`]); the map
    /// then draws the new selected link and selects the system the
    /// original's `_selSystem` names (@0x1408c-0x140d8,
    /// `Session::map_preselection`), item 3 is redrawn whether a link
    /// was picked or none (@0x14032), and the nav area is redrawn at
    /// once, as the filter runs `_DrawStatus` with nav marked
    /// (`_statusRedraw[0]`, @0x13fb2-0x1406e). Docked, as on the bar's
    /// map, nothing changes.
    fn cycle_map(&mut self, cycle: MapCycle) {
        if self.map.mode() == MapMode::Hypergate {
            let destinations = match self.sim.screen(ScreenAsk::HypergateCycle) {
                ScreenAnswer::HypergateCycle(destinations) => destinations,
                _ => Vec::new(),
            };
            if let Some(pick) = next_gate_pick(&destinations, self.map.selected(), cycle.back) {
                self.map.select_gate(pick);
                self.map.redraw();
            }
            self.redraw_map();
            return;
        }
        if !self.send(FlightCommand::CycleMapSelection).is_refused() {
            let preselection = match self.sim.screen(ScreenAsk::MapPreselection) {
                ScreenAnswer::MapPreselection(preselection) => preselection,
                _ => None,
            };
            self.map.preselect(preselection);
            self.map.redraw();
            self.redraw_map();
            self.retake_nav();
        }
    }
}
