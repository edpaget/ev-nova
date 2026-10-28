//! Test helpers for the [`FlightSim`] port: [`FakeFlight`], a scripted
//! sim that records what it's asked, and [`check_contract`], what every
//! sim must honour.
//!
//! Available to this crate's tests and, through the `fixture` feature, to
//! other crates' tests. A component test of the view runs on a
//! [`FakeFlight`]; `tests/it/flight_seam.rs` runs [`check_contract`] on a
//! real [`SessionFlight`](crate::flight::SessionFlight).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use nova_sim::brackets::{NavBrackets, TargetBrackets};
use nova_sim::catalog::SystemId;
use nova_sim::flight_frame::{
    Answer, DrawnBoxes, FlightCommand, FlightEvent, FlightFrame, FlightOpening, FloatingLinks,
    FrameAsteroid, FrameBeam, FrameBox, FrameHud, FrameShip, Gates, MapKey, StepInput, StepReport,
    StepRun,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::navigation::NavMode;
use nova_sim::pilot_panel::PilotPanel;
use nova_sim::status_cargo::StatusCargo;
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{ClassRow, Gauge, HailView, PlunderView, RadarGear, Reserves, Take};
use nova_sim::{
    EscortClass, Hover, MapMarks, MissionId, PlayerCommand, SavedNav, ShipId, ShipRef, StellarId,
    StellarPick, Vec2, WeaponId,
};
use nova_sim::{JumpReadiness, PlayerFate};

use nova_sim::mission::screens::OFFER_GAP_TICKS;
use nova_sim::{
    Direction, Good, HireList, JunkId, Lot, Market, MissionInfo, News, Order, OutfitId, Outfitter,
    PlayerInfo, ShipNaming, ShipRefusal, Shipyard,
};

use nova_sim::DeskReading;

use super::{CommSource, FlightSim, MapSource, NameSource, PilotSource, PortSource};
use crate::devtools::{DevDesk, EditRefusal, PilotEdit};

/// A scripted [`FlightSim`]: it shows the frame it was given, records
/// each command in the order sent, answers each from a queue
/// ([`Answer::Refused`] once the queue is empty), hands out the events
/// queued for it on [`FlightSim::drain`], and reports each frame's steps
/// from a queue (no step run once the queue is empty), recording each
/// call's input and step count. Its frames take every gate but those it
/// is told to refuse ([`FakeFlight::refuse`]). It answers [`MapSource`]
/// from [`FakeMap`]
/// ([`FakeFlight::map_mut`]), and records the map's questions whose
/// arguments matter ([`FakeFlight::asked`]). It answers the asks
/// ([`FlightCommand::is_ask`]) from what it is told for each, without the
/// answer queue and outside [`FakeFlight::applied`], so the cursor's hover
/// and a dialog's description never shift the answers queued for the
/// commands a test is about, recording each in [`FakeFlight::asks`]: the
/// hover as told ([`FakeFlight::set_hover`]), the coming traffic from
/// its list ([`FakeFlight::set_coming_traffic`]), and each description's
/// load with the next text queued ([`FakeFlight::queue_description`]),
/// or, once the queue is empty, as its raw text unchanged, resolving
/// nothing, failed or not, as a failed flight still loads one. A new ask
/// is one field and setter for its answer, and one arm in
/// `FakeFlight::answer_ask`; it is recorded with the rest. It lists the
/// weapons from its list ([`FakeFlight::set_weapons`]), failed or not. It
/// answers
/// [`CommSource`] from [`FakeComm`] ([`FakeFlight::comm_mut`]),
/// [`PortSource`] from [`FakePort`] ([`FakeFlight::port_mut`]),
/// [`NameSource`] from [`FakeNames`] ([`FakeFlight::names_mut`]) and
/// [`PilotSource`] from [`FakePilot`] ([`FakeFlight::pilot_mut`]). It
/// answers the message line's commands ([`FlightCommand::CommShown`],
/// [`FlightCommand::CommAcknowledged`] and [`FlightCommand::CommTicked`])
/// and the settings ([`FlightCommand::SetWindowWidth`],
/// [`FlightCommand::SetDoubleTimeKey`], [`FlightCommand::SetSoundQuiet`])
/// done without taking an answer from the queue, as these follow-ups
/// ride along with the commands a test is about. Told to fail
/// ([`FakeFlight::fail`]), its opening says why ([`FakeFlight::opening`])
/// and it refuses every command but a description's load, still
/// recording each. It is the developer
/// tools' desk too ([`DevDesk`]), reading as scripted and recording each
/// edit ([`FakeFlight::desk_mut`], [`FakeFlight::edits`]). It counts the
/// frames it fills ([`FakeFlight::fills`]). One with nothing scripted
/// meets [`check_contract`].
#[derive(Clone, Debug, Default)]
pub struct FakeFlight {
    frame: FlightFrame,
    applied: Vec<FlightCommand>,
    answers: VecDeque<(Answer, Option<FlightFrame>, Vec<FlightEvent>)>,
    events: Vec<FlightEvent>,
    reports: VecDeque<StepReport>,
    stepped: Vec<(StepInput, u32)>,
    refused: Vec<PlayerCommand>,
    map: FakeMap,
    asked: RefCell<Vec<MapAsk>>,
    asks: Vec<FlightCommand>,
    hover: Option<Hover>,
    coming: Vec<ShipId>,
    weapons: Vec<WeaponId>,
    descriptions: VecDeque<String>,
    comm: FakeComm,
    port: FakePort,
    names: FakeNames,
    pilot: FakePilot,
    failure: Option<String>,
    desk: FakeDesk,
    edits: Vec<PilotEdit>,
    fills: Cell<usize>,
}

/// What a [`FakeFlight`] answers as the developer tools' desk
/// ([`DevDesk`]). The default has no reading and takes every edit.
#[derive(Clone, Debug, Default)]
pub struct FakeDesk {
    /// [`DevDesk::reading`]'s answer.
    pub reading: Option<DeskReading>,
    /// What every edit is refused with while set; taken otherwise.
    pub refusal: Option<EditRefusal>,
}

/// What a [`FakeFlight`] answers the [`CommSource`] questions with, each
/// field the answer to the question of its name. The default answers
/// nothing, as a [`CommSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakeComm {
    /// [`CommSource::hailing`]'s answer.
    pub hailing: Option<HailView>,
    /// [`CommSource::boarding`]'s answer.
    pub boarding: Option<PlunderView>,
}

/// What a [`FakeFlight`] answers the [`PortSource`] questions with, each
/// field the answer to the question of its name; a `&mut` read hands back
/// a clone of its field. The default answers nothing open, as a
/// [`PortSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakePort {
    /// [`PortSource::market`]'s answer.
    pub market: Option<Market>,
    /// [`PortSource::recharge_enabled`]'s answer.
    pub recharge_enabled: bool,
    /// [`PortSource::news`]'s answer.
    pub news: Option<News>,
    /// [`PortSource::news_picture`]'s answer.
    pub news_picture: Option<i16>,
    /// [`PortSource::mission_info`]'s answer; nothing listed when `None`.
    pub mission_info: Option<MissionInfo>,
    /// [`PortSource::seen_intro`]'s answer.
    pub seen_intro: bool,
    /// The missions whose offer dialog [`PortSource::refusable_offer_dialog`]
    /// says has a refuse button.
    pub refusable: Vec<MissionId>,
    /// [`PortSource::outfitter`]'s answer.
    pub outfitter: Option<Outfitter>,
    /// [`PortSource::shipyard`]'s answer.
    pub shipyard: Option<Shipyard>,
    /// [`PortSource::escorts_for_hire`]'s answer.
    pub hire_list: Option<HireList>,
    /// [`PortSource::ship_naming`]'s answer, whatever the class; no
    /// shipyard when `None`.
    pub ship_naming: Option<Result<ShipNaming, ShipRefusal>>,
    /// [`PortSource::offer_gap`]'s answer; [`OFFER_GAP_TICKS`] when `None`.
    pub offer_gap: Option<u32>,
    /// The offers [`PortSource::offer_text`] fills, each with the text it
    /// gives whatever the raw text; none for a mission not listed.
    pub offer_texts: Vec<(MissionId, String)>,
    /// [`PortSource::can_hire_escorts`]'s answer.
    pub can_hire_escorts: bool,
}

/// What a [`FakeFlight`] answers the [`NameSource`] questions with, each
/// field the answer to the question of its name, a list of what it names
/// for the questions that take one. The default names nothing, as a
/// [`NameSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakeNames {
    /// The ship classes [`NameSource::ship_name`] names, with their names.
    pub ship_names: Vec<(ShipId, String)>,
    /// The ship classes [`NameSource::ship_long_name`] names, with their
    /// long names.
    pub ship_long_names: Vec<(ShipId, String)>,
    /// The outfits [`NameSource::outfit_name`] names, with their names.
    pub outfit_names: Vec<(OutfitId, String)>,
    /// The outfits [`NameSource::outfit_names`] names, with their
    /// lower-case names and plurals.
    pub outfit_lc_names: Vec<(OutfitId, (String, String))>,
    /// The goods [`NameSource::good_name`] names, with their names.
    pub good_names: Vec<(Good, String)>,
    /// The `jünk`s [`NameSource::junk_lc_name`] names, with their
    /// lower-case names.
    pub junk_lc_names: Vec<(JunkId, String)>,
    /// [`NameSource::cargo_name`]'s `STR#` 4001, by index.
    pub cargo_names: Vec<String>,
    /// [`NameSource::player_ship_name`]'s answer.
    pub player_ship_name: Option<String>,
    /// [`NameSource::long_date`]'s answer.
    pub long_date: String,
    /// The texts [`NameSource::plain_text`] fills, each with what it
    /// gives; any other comes back unchanged.
    pub plain_texts: Vec<(String, String)>,
}

/// What a [`FakeFlight`] answers the [`PilotSource`] questions with, each
/// field the answer to the question of its name. The default shows none,
/// as a [`PilotSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakePilot {
    /// [`PilotSource::player_info`]'s answer.
    pub player_info: Option<PlayerInfo>,
    /// [`PilotSource::pilot_panel`]'s answer.
    pub pilot_panel: Option<PilotPanel>,
}

/// The name `names` gives `id`, if it lists one.
fn named<'a, K: PartialEq, V>(names: &'a [(K, V)], id: &K) -> Option<&'a V> {
    names
        .iter()
        .find(|(listed, _)| listed == id)
        .map(|(_, name)| name)
}

/// What a [`FakeFlight`] answers the [`MapSource`] questions with, each
/// field the answer to the question of its name. The default answers
/// nothing, as a [`MapSource`]'s defaults do.
#[derive(Clone, Debug, Default)]
pub struct FakeMap {
    /// [`MapSource::map_drawing`]'s answer, whatever it is asked.
    pub drawing: Option<MapDrawing>,
    /// [`MapSource::map_panel`]'s answer, whatever it is asked.
    pub panel: Option<MapPanel>,
    /// [`MapSource::map_border_sources`]'s answer.
    pub border_sources: Option<BorderSources>,
    /// [`MapSource::find_map_system`]'s answer, whatever the query.
    pub search: Option<SearchOutcome>,
    /// [`MapSource::map_mission_systems`]'s answer.
    pub mission_systems: Vec<SystemId>,
    /// [`MapSource::map_preselection`]'s answer.
    pub map_preselection: Option<SystemId>,
    /// [`MapSource::floating_preselection`]'s answer.
    pub floating_preselection: Option<SystemId>,
    /// [`MapSource::hypergate_cycle`]'s answer.
    pub hypergate_cycle: Vec<Option<SystemId>>,
    /// The marks each of [`MapSource::offer_map_marks`],
    /// [`MapSource::computer_map_marks`] and
    /// [`MapSource::mission_info_map_marks`] answers, whatever it is
    /// asked; [`MapMarks::NONE`] when `None`.
    pub marks: Option<MapMarks>,
    /// [`MapSource::map_nav`]'s answer.
    pub map_nav: Option<SavedNav>,
    /// [`MapSource::floating_map_places`]'s answer.
    pub floating: FloatingLinks,
    /// The stellars [`MapSource::stellar_name`] names, with their names.
    pub stellar_names: Vec<(StellarId, String)>,
    /// The systems [`MapSource::has_explored`] says are explored.
    pub explored: Vec<SystemId>,
    /// [`MapSource::galaxy`]'s answer.
    pub galaxy: Option<OverlaidGalaxy>,
}

/// A [`MapSource`] question a [`FakeFlight`] was asked whose arguments a
/// test may check, in the order asked ([`FakeFlight::asked`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapAsk {
    /// [`MapSource::map_drawing`], with its arguments.
    Drawing {
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
        /// The mission systems.
        missions: Vec<SystemId>,
        /// The opener's marks.
        marks: MapMarks,
    },
    /// [`MapSource::map_panel`], with its arguments.
    Panel {
        /// The system selected.
        selected: Option<SystemId>,
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
    },
    /// [`MapSource::find_map_system`], with its query.
    Find(String),
    /// [`MapSource::offer_map_marks`], for its mission.
    OfferMarks(MissionId),
    /// [`MapSource::computer_map_marks`], for its row.
    ComputerMarks(Option<MissionId>),
    /// [`MapSource::mission_info_map_marks`], for its slot.
    MissionInfoMarks(Option<u8>),
}

impl FakeFlight {
    /// A sim with nothing scripted: the default frame, every command
    /// refused, nothing to drain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Shows `frame` from now on.
    pub fn set_frame(&mut self, frame: FlightFrame) {
        self.frame = frame;
    }

    /// The frame it shows, to change in place.
    pub fn frame_mut(&mut self) -> &mut FlightFrame {
        &mut self.frame
    }

    /// Answers every hover ask ([`FlightCommand::Hover`]) with `hover`
    /// from now on.
    pub fn set_hover(&mut self, hover: Option<Hover>) {
        self.hover = hover;
    }

    /// The asks sent ([`FlightCommand::is_ask`]), in order, each as it was
    /// sent: kept apart from [`FakeFlight::applied`].
    #[must_use]
    pub fn asks(&self) -> &[FlightCommand] {
        &self.asks
    }

    /// Answers the coming traffic ([`FlightCommand::ComingTraffic`]) with
    /// `ships` from now on.
    pub fn set_coming_traffic(&mut self, ships: Vec<ShipId>) {
        self.coming = ships;
    }

    /// Lists `weapons` in its opening from now on
    /// ([`FakeFlight::opening`]).
    pub fn set_weapons(&mut self, weapons: Vec<WeaponId>) {
        self.weapons = weapons;
    }

    /// Loads the next description not yet loaded
    /// ([`FlightCommand::LoadDescription`]) as `text`.
    pub fn queue_description(&mut self, text: &str) {
        self.descriptions.push_back(text.to_owned());
    }

    /// Answers the next command not yet answered with `answer`: the
    /// message line's commands ([`FlightCommand::CommShown`],
    /// [`FlightCommand::CommAcknowledged`], [`FlightCommand::CommTicked`])
    /// take none, and are answered done, so the answers a test queues line
    /// up with the commands it is about.
    pub fn queue_answer(&mut self, answer: Answer) {
        self.answers.push_back((answer, None, Vec::new()));
    }

    /// Answers the next command not yet answered with `answer`, as
    /// [`FakeFlight::queue_answer`] does, and shows `frame` from then on,
    /// as a command that changes the flight leaves it.
    pub fn queue_answer_showing(&mut self, answer: Answer, frame: FlightFrame) {
        self.answers.push_back((answer, Some(frame), Vec::new()));
    }

    /// Answers the next command not yet answered with `answer` and shows
    /// `frame`, as [`FakeFlight::queue_answer_showing`] does, and hands
    /// out `events` on the drains after it, as a command raises what it
    /// tells: none is drained before the command.
    pub fn queue_answer_telling(
        &mut self,
        answer: Answer,
        frame: FlightFrame,
        events: Vec<FlightEvent>,
    ) {
        self.answers.push_back((answer, Some(frame), events));
    }

    /// Hands out `event` on the next drain.
    pub fn queue_event(&mut self, event: FlightEvent) {
        self.events.push(event);
    }

    /// Reports `report` for the next call of [`FlightSim::step`] not yet
    /// reported, whatever its step count.
    pub fn queue_report(&mut self, report: StepReport) {
        self.reports.push_back(report);
    }

    /// The commands applied, in the order sent.
    #[must_use]
    pub fn applied(&self) -> &[FlightCommand] {
        &self.applied
    }

    /// How many times [`FlightSim::step`] was called: once a frame.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.stepped.len()
    }

    /// How many steps the calls of [`FlightSim::step`] asked for, all
    /// told.
    #[must_use]
    pub fn steps_asked(&self) -> u32 {
        self.stepped.iter().map(|&(_, steps)| steps).sum()
    }

    /// Each call's input to [`FlightSim::step`], in order.
    #[must_use]
    pub fn inputs(&self) -> Vec<StepInput> {
        self.stepped.iter().map(|&(input, _)| input).collect()
    }

    /// Each call of [`FlightSim::step`], its input and its step count, in
    /// order.
    #[must_use]
    pub fn stepped_calls(&self) -> &[(StepInput, u32)] {
        &self.stepped
    }

    /// Refuses the gate `gate` from now on in every frame it fills
    /// ([`FlightFrame::gates`]), whatever frame is scripted later.
    pub fn refuse(&mut self, gate: PlayerCommand) {
        self.refused.push(gate);
    }

    /// Takes the gate `gate` again, as the frame scripted sets it.
    pub fn take(&mut self, gate: PlayerCommand) {
        self.refused.retain(|&refused| refused != gate);
    }

    /// What the map's questions are answered with, to script.
    pub fn map_mut(&mut self) -> &mut FakeMap {
        &mut self.map
    }

    /// What the comm's questions are answered with, to script.
    pub fn comm_mut(&mut self) -> &mut FakeComm {
        &mut self.comm
    }

    /// What the spaceport's questions are answered with, to script.
    pub fn port_mut(&mut self) -> &mut FakePort {
        &mut self.port
    }

    /// What the wording's names and date are answered with, to script.
    pub fn names_mut(&mut self) -> &mut FakeNames {
        &mut self.names
    }

    /// What the pilot's screens are answered with, to script.
    pub fn pilot_mut(&mut self) -> &mut FakePilot {
        &mut self.pilot
    }

    /// The map's questions asked whose arguments matter, in the order
    /// asked ([`MapAsk`]).
    #[must_use]
    pub fn asked(&self) -> Vec<MapAsk> {
        self.asked.borrow().clone()
    }

    /// Fails for `reason` from now on, as a flight that could not start:
    /// its opening gives it ([`FakeFlight::opening`]) and every command but
    /// a description's load is refused. The frame, the events and the
    /// answers stay as scripted, for a test of [`check_contract`]'s clause
    /// on a failed flight.
    pub fn fail(&mut self, reason: &str) {
        self.failure = Some(reason.to_owned());
    }

    /// What the view is handed as it opens, as scripted
    /// ([`FlightOpening`]): the weapons it lists
    /// ([`FakeFlight::set_weapons`]) and why it failed
    /// ([`FakeFlight::fail`]), for `FlightView::over` and
    /// [`check_contract`].
    #[must_use]
    pub fn opening(&self) -> FlightOpening {
        FlightOpening {
            weapons: self.weapons.clone(),
            names: nova_sim::flight_frame::Names::default(),
            failure: self.failure.clone(),
        }
    }

    /// What the desk answers, to script.
    pub fn desk_mut(&mut self) -> &mut FakeDesk {
        &mut self.desk
    }

    /// The desk's edits, in the order made, taken or refused.
    #[must_use]
    pub fn edits(&self) -> &[PilotEdit] {
        &self.edits
    }

    /// How many frames it has filled ([`FlightSim::frame`]): what the
    /// view's fills per tick, per draw and per key are counted by.
    #[must_use]
    pub fn fills(&self) -> usize {
        self.fills.get()
    }

    /// Notes `ask`.
    fn ask(&self, ask: MapAsk) {
        self.asked.borrow_mut().push(ask);
    }

    /// Answers the ask `ask` from what it is told for it, recording it
    /// ([`FakeFlight::asks`]): one arm for each ask. A failed fake refuses
    /// them but a description's load, as a failed flight does.
    fn answer_ask(&mut self, ask: FlightCommand) -> Answer {
        let answer = match &ask {
            FlightCommand::LoadDescription { raw, .. } => Answer::Description(
                self.descriptions
                    .pop_front()
                    .unwrap_or_else(|| raw.clone().unwrap_or_default()),
            ),
            _ if self.failure.is_some() => Answer::Refused,
            FlightCommand::Hover { .. } => Answer::Hovered(self.hover),
            FlightCommand::ComingTraffic => Answer::ComingTraffic(self.coming.clone()),
            _ => Answer::Refused,
        };
        self.asks.push(ask);
        answer
    }
}

impl FlightSim for FakeFlight {
    /// The frame scripted, each gate refused ([`FakeFlight::refuse`])
    /// refused in it whatever the frame set.
    fn frame(&self, into: &mut FlightFrame) {
        self.fills.set(self.fills.get() + 1);
        into.clone_from(&self.frame);
        for &gate in &self.refused {
            into.gates = into.gates.refusing(gate);
        }
    }

    fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport) {
        self.stepped.push((*input, steps));
        match self.reports.pop_front() {
            Some(report) => into.clone_from(&report),
            None => into.clear(),
        }
    }

    fn apply(&mut self, command: FlightCommand) -> Answer {
        if command.is_ask() {
            return self.answer_ask(command);
        }
        let rides_along = matches!(
            command,
            FlightCommand::CommShown(_)
                | FlightCommand::CommAcknowledged
                | FlightCommand::CommTicked
                | FlightCommand::SetWindowWidth(_)
                | FlightCommand::SetDoubleTimeKey(_)
                | FlightCommand::SetSoundQuiet(_)
        );
        self.applied.push(command);
        if self.failure.is_some() {
            return Answer::Refused;
        }
        if rides_along {
            return Answer::Done;
        }
        let Some((answer, frame, mut events)) = self.answers.pop_front() else {
            return Answer::Refused;
        };
        if let Some(frame) = frame {
            self.frame = frame;
        }
        self.events.append(&mut events);
        answer
    }

    fn drain(&mut self, into: &mut Vec<FlightEvent>) {
        into.append(&mut self.events);
    }
}

impl DevDesk for FakeFlight {
    fn reading(&self) -> Option<DeskReading> {
        self.desk.reading.clone()
    }

    fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal> {
        self.edits.push(edit);
        self.desk.refusal.map_or(Ok(()), Err)
    }
}

impl MapSource for FakeFlight {
    fn map_drawing(
        &self,
        hypergate: Option<&[SystemId]>,
        missions: &[SystemId],
        marks: MapMarks,
    ) -> Option<MapDrawing> {
        self.ask(MapAsk::Drawing {
            hypergate: hypergate.map(<[SystemId]>::to_vec),
            missions: missions.to_vec(),
            marks,
        });
        self.map.drawing.clone()
    }

    fn map_panel(
        &self,
        selected: Option<SystemId>,
        hypergate: Option<&[SystemId]>,
    ) -> Option<MapPanel> {
        self.ask(MapAsk::Panel {
            selected,
            hypergate: hypergate.map(<[SystemId]>::to_vec),
        });
        self.map.panel.clone()
    }

    fn map_border_sources(&self) -> Option<BorderSources> {
        self.map.border_sources.clone()
    }

    fn find_map_system(&self, query: &str) -> Option<SearchOutcome> {
        self.ask(MapAsk::Find(query.to_owned()));
        self.map.search
    }

    fn map_mission_systems(&self) -> Vec<SystemId> {
        self.map.mission_systems.clone()
    }

    fn map_preselection(&self) -> Option<SystemId> {
        self.map.map_preselection
    }

    fn floating_preselection(&self) -> Option<SystemId> {
        self.map.floating_preselection
    }

    fn hypergate_cycle(&self) -> Vec<Option<SystemId>> {
        self.map.hypergate_cycle.clone()
    }

    fn offer_map_marks(&self, mission: MissionId) -> MapMarks {
        self.ask(MapAsk::OfferMarks(mission));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn computer_map_marks(&self, selected: Option<MissionId>) -> MapMarks {
        self.ask(MapAsk::ComputerMarks(selected));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn mission_info_map_marks(&self, slot: Option<u8>) -> MapMarks {
        self.ask(MapAsk::MissionInfoMarks(slot));
        self.map.marks.unwrap_or(MapMarks::NONE)
    }

    fn map_nav(&self) -> Option<SavedNav> {
        self.map.map_nav
    }

    fn floating_map_places(&self) -> FloatingLinks {
        self.map.floating.clone()
    }

    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        self.map
            .stellar_names
            .iter()
            .find(|(id, _)| *id == stellar)
            .map(|(_, name)| name.clone())
    }

    fn has_explored(&self, system: SystemId) -> bool {
        self.map.explored.contains(&system)
    }

    fn galaxy(&self) -> Option<&OverlaidGalaxy> {
        self.map.galaxy.as_ref()
    }
}

impl CommSource for FakeFlight {
    fn hailing(&self) -> Option<HailView> {
        self.comm.hailing.clone()
    }

    fn boarding(&self) -> Option<PlunderView> {
        self.comm.boarding
    }
}

impl PortSource for FakeFlight {
    fn market(&self) -> Option<Market> {
        self.port.market.clone()
    }

    fn recharge_enabled(&self) -> bool {
        self.port.recharge_enabled
    }

    fn news(&self) -> Option<&News> {
        self.port.news.as_ref()
    }

    fn news_picture(&self) -> Option<i16> {
        self.port.news_picture
    }

    fn mission_info(&self) -> MissionInfo {
        self.port.mission_info.clone().unwrap_or_default()
    }

    fn seen_intro(&self) -> bool {
        self.port.seen_intro
    }

    fn refusable_offer_dialog(&self, mission: MissionId) -> bool {
        self.port.refusable.contains(&mission)
    }

    fn offer_text(&self, mission: MissionId, raw: &str) -> Option<String> {
        let _ = raw;
        named(&self.port.offer_texts, &mission).cloned()
    }

    fn outfitter(&mut self) -> Option<Outfitter> {
        self.port.outfitter.clone()
    }

    fn shipyard(&mut self) -> Option<Shipyard> {
        self.port.shipyard.clone()
    }

    fn escorts_for_hire(&mut self) -> Option<HireList> {
        self.port.hire_list.clone()
    }

    fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        let _ = ship;
        self.port
            .ship_naming
            .clone()
            .unwrap_or(Err(ShipRefusal::NoShipyard))
    }

    fn offer_gap(&mut self) -> u32 {
        self.port.offer_gap.unwrap_or(OFFER_GAP_TICKS)
    }

    fn can_hire_escorts(&self) -> bool {
        self.port.can_hire_escorts
    }
}

impl NameSource for FakeFlight {
    fn ship_name(&self, ship: ShipId) -> Option<&str> {
        named(&self.names.ship_names, &ship).map(String::as_str)
    }

    fn ship_long_name(&self, ship: ShipId) -> Option<&str> {
        named(&self.names.ship_long_names, &ship).map(String::as_str)
    }

    fn outfit_name(&self, outfit: OutfitId) -> Option<&str> {
        named(&self.names.outfit_names, &outfit).map(String::as_str)
    }

    fn outfit_names(&self, outfit: OutfitId) -> Option<(&str, &str)> {
        named(&self.names.outfit_lc_names, &outfit).map(|(name, plural)| (&**name, &**plural))
    }

    fn good_name(&self, good: Good) -> Option<&str> {
        named(&self.names.good_names, &good).map(String::as_str)
    }

    fn junk_lc_name(&self, junk: JunkId) -> Option<&str> {
        named(&self.names.junk_lc_names, &junk).map(String::as_str)
    }

    fn cargo_name(&self, cargo: u8) -> Option<String> {
        self.names.cargo_names.get(usize::from(cargo)).cloned()
    }

    fn player_ship_name(&self) -> Option<&str> {
        self.names.player_ship_name.as_deref()
    }

    fn long_date(&self) -> String {
        self.names.long_date.clone()
    }

    fn plain_text(&self, text: &str) -> String {
        self.names
            .plain_texts
            .iter()
            .find(|(raw, _)| raw == text)
            .map_or_else(|| text.to_owned(), |(_, filled)| filled.clone())
    }
}

impl PilotSource for FakeFlight {
    fn player_info(&self) -> Option<PlayerInfo> {
        self.pilot.player_info.clone()
    }

    fn pilot_panel(&self) -> Option<PilotPanel> {
        self.pilot.pilot_panel.clone()
    }
}

/// A command every sim refuses: a stellar key for a slot no system has.
#[must_use]
pub fn refused_command() -> FlightCommand {
    FlightCommand::SelectStellar(StellarPick::Slot(usize::MAX))
}

/// A frame unlike `frame` in every field, to fill over: a fill that
/// leaves a field unwritten then shows in the comparison. The
/// destructuring names every field, so a field added to [`FlightFrame`]
/// doesn't compile until it is changed here.
#[allow(clippy::too_many_lines)] // a few lines for each of the frame's fields
pub(crate) fn unlike(frame: &FlightFrame) -> FlightFrame {
    let FlightFrame {
        nav_mode,
        system,
        course_shown,
        course_plotted,
        landed,
        jump_under_way,
        warping,
        warp_end_due,
        fate,
        hold_expired,
        leave_due,
        jump_readiness,
        texts_queued,
        gates,
        map_key,
        ships,
        drawn,
        shots,
        beams,
        cargo_boxes,
        asteroids,
        stellars,
        effective_murk,
        system_murk,
        system_background,
        double_time,
        hyper_gamma,
        traffic_ships,
        hud,
        target,
        death_descs,
        player_ship_name,
        long_date,
        explored,
    } = frame;
    let other_system = SystemId(if system.0 == i16::MAX { 0 } else { i16::MAX });
    FlightFrame {
        nav_mode: unlike_nav(*nav_mode),
        system: other_system,
        course_shown: if course_shown.is_empty() {
            vec![other_system]
        } else {
            Vec::new()
        },
        course_plotted: !course_plotted,
        landed: if landed.is_some() {
            None
        } else {
            Some(StellarId(128))
        },
        jump_under_way: !jump_under_way,
        warping: !warping,
        warp_end_due: !warp_end_due,
        fate: if *fate == PlayerFate::Flying {
            PlayerFate::GameOver
        } else {
            PlayerFate::Flying
        },
        hold_expired: !hold_expired,
        leave_due: !leave_due,
        jump_readiness: if *jump_readiness == JumpReadiness::Blocked {
            JumpReadiness::Clear
        } else {
            JumpReadiness::Blocked
        },
        texts_queued: texts_queued.wrapping_add(1),
        gates: if *gates == Gates::EVERY {
            Gates::NONE
        } else {
            Gates::EVERY
        },
        map_key: MapKey {
            system: if map_key.system == other_system {
                SystemId(other_system.0 - 1)
            } else {
                other_system
            },
            nav: unlike_nav(map_key.nav),
            ..map_key.clone()
        },
        ships: one_or_none(ships, FrameShip::default),
        drawn: one_or_none(drawn, || (ShipRef::Player, nova_sim::ShipLayer::Ships)),
        shots: one_or_none(shots, record::shot),
        beams: one_or_none(beams, || FrameBeam {
            firer: ShipRef::Player,
            start: Vec2::ZERO,
            end: Vec2::ZERO,
            weapon: nova_sim::WeaponId(128),
            fade: 0,
        }),
        cargo_boxes: one_or_none(cargo_boxes, || FrameBox {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            frame: 0,
            fade: 0,
            graphic: 0,
        }),
        asteroids: one_or_none(asteroids, || FrameAsteroid {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            frame: 0.0,
            kind: 0,
        }),
        stellars: one_or_none(stellars, || record::stellar(StellarId(128))),
        effective_murk: effective_murk.wrapping_add(1),
        system_murk: system_murk.wrapping_add(1),
        system_background: if system_background.is_some() {
            None
        } else {
            Some(0)
        },
        double_time: !double_time,
        hyper_gamma: hyper_gamma + 1.0,
        traffic_ships: one_or_none(traffic_ships, || ShipId(128)),
        hud: unlike_hud(hud),
        target: if target.is_some() {
            None
        } else {
            Some(record::target(0))
        },
        death_descs: one_or_none(death_descs, || 128),
        player_ship_name: toggle(player_ship_name.as_ref(), "Unlike".to_owned()),
        long_date: if long_date.is_empty() {
            "Unlike".to_owned()
        } else {
            String::new()
        },
        explored: one_or_none(explored, || SystemId(128)),
    }
}

/// A HUD unlike `hud` in every field. The destructuring names every
/// field, so a field added to [`FrameHud`] doesn't compile until it is
/// changed here.
fn unlike_hud(hud: &FrameHud) -> FrameHud {
    let FrameHud {
        reserves,
        cargo,
        secondary,
        secondary_line_count,
        radar_gear,
        interference,
        interface_government,
        nav_brackets,
        target_brackets,
        red_alert,
        escort_rows,
        escorts_flying,
    } = hud;
    let gauge = |gauge: Gauge| Gauge::full(gauge.max + 1.0);
    FrameHud {
        reserves: Reserves {
            shield: gauge(reserves.shield),
            armor: gauge(reserves.armor),
            fuel: gauge(reserves.fuel),
        },
        cargo: StatusCargo {
            cash: cargo.cash.wrapping_add(1),
            ..cargo.clone()
        },
        secondary: toggle(secondary.as_ref(), nova_sim::WeaponId(128)),
        secondary_line_count: toggle(secondary_line_count.as_ref(), 1),
        radar_gear: RadarGear {
            iff: !radar_gear.iff,
            ..*radar_gear
        },
        interference: u8::from(*interference == 0),
        interface_government: toggle(interface_government.as_ref(), nova_sim::GovtId(128)),
        nav_brackets: toggle(
            nav_brackets.as_ref(),
            NavBrackets {
                stellar: StellarId(128),
                position: Vec2::ZERO,
                expand: 0.0,
                dominated: false,
            },
        ),
        target_brackets: toggle(
            target_brackets.as_ref(),
            TargetBrackets {
                npc: nova_sim::NpcId(0),
                expand: 0.0,
                standing: nova_sim::brackets::Standing::Neutral,
            },
        ),
        red_alert: toggle(red_alert.as_ref(), nova_sim::AlertIcon::Red),
        escort_rows: escort_rows.map(|row| ClassRow {
            present: !row.present,
            ..row
        }),
        escorts_flying: !escorts_flying,
    }
}

/// None when `value` is some, else `some`.
fn toggle<T>(value: Option<&T>, some: T) -> Option<T> {
    if value.is_some() { None } else { Some(some) }
}

/// None when `records` holds any, else one of `record`'s.
fn one_or_none<T>(records: &[T], record: impl FnOnce() -> T) -> Vec<T> {
    if records.is_empty() {
        vec![record()]
    } else {
        Vec::new()
    }
}

/// Small record builders for frames a test scripts: a ship, a shot, a
/// stellar.
pub mod record {
    use nova_sim::combat::projectile::ShotId;
    use nova_sim::flight_frame::{FrameShip, FrameShot, FrameStellar, FrameTarget};
    use nova_sim::{
        DrawnSheet, NpcId, ShipId, ShipRef, StellarId, StellarIff, TargetStatus, Vec2, WeaponId,
    };

    /// The player's ship, of class `class`, at `at` facing `heading`,
    /// drawn in full and boxed by its own frames.
    #[must_use]
    pub fn player(class: i16, at: Vec2, heading: f32) -> FrameShip {
        FrameShip {
            class: ShipId(class),
            position: at,
            heading,
            framed: Some(ShipId(class)),
            ..FrameShip::default()
        }
    }

    /// NPC `id`, of class `class`, at `at` facing `heading`, drawn in
    /// full, boxed by its own frames, on the radar.
    #[must_use]
    pub fn npc(id: u32, class: i16, at: Vec2, heading: f32) -> FrameShip {
        FrameShip {
            ship: ShipRef::Npc(NpcId(id)),
            class: ShipId(class),
            position: at,
            heading,
            framed: Some(ShipId(class)),
            on_radar: true,
            ..FrameShip::default()
        }
    }

    /// A shot of weapon 128 at the system's centre, at rest.
    #[must_use]
    pub fn shot() -> FrameShot {
        FrameShot {
            id: ShotId(0),
            weapon: WeaponId(128),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 0.0,
            age: 0,
            spin_timer: 0,
            layer: nova_sim::combat::shot_layer::ShotLayer::Shots,
        }
    }

    /// NPC `id` targeted, named "Target", with no subtitle, at full
    /// shield and no label.
    #[must_use]
    pub fn target(id: u32) -> FrameTarget {
        FrameTarget {
            npc: NpcId(id),
            name: "Target".to_owned(),
            subtitle: None,
            status: TargetStatus::Shield(100),
            tag: None,
        }
    }

    /// Stellar `id`, shown, whole, on its `Type` sheet, no hypergate,
    /// unlandable on the radar.
    #[must_use]
    pub fn stellar(id: StellarId) -> FrameStellar {
        FrameStellar {
            id,
            shown: true,
            destroyed: false,
            drawn: DrawnSheet::Type,
            counted: DrawnSheet::Type,
            frozen: false,
            counts_none: false,
            gate_frame: None,
            iff: Some(StellarIff::Unlandable),
        }
    }
}

/// A nav mode other than `nav`.
fn unlike_nav(nav: NavMode) -> NavMode {
    if nav == NavMode::Off {
        NavMode::Hyperspace(Some(SystemId(i16::MAX)))
    } else {
        NavMode::Off
    }
}

/// Checks the port's contract against `sim`:
///
/// - **drain empties**: after one drain, a second adds nothing;
/// - **a drain changes no frame**: the frame filled after the drains
///   equals the frame filled before them, so the view keeps its frame
///   across a drain;
/// - **a frame fills the same twice**: two fills with no step between,
///   the second over a frame unlike the first in every field (the gates
///   and the death descs among them), give equal frames, so a fill that
///   skips a field fails;
/// - **an ask changes no frame**: each ask ([`FlightCommand::is_ask`]: a
///   hover at the player's ship with no boxes, the coming traffic and a
///   description's load) leaves the frame filled after it equal to the
///   frame before, so the view fills none after one;
/// - **a refusal changes nothing**: [`refused_command`] answers a
///   refusal, and the frame after it equals the frame before;
/// - **no step asked, none run**: a frame's steps asked for none fill the
///   report, over one holding a step, with no step and no stop;
/// - **the player's record first, once**: a frame's ships begin with the
///   player's, and hold no other;
/// - **every ship drawn has a record**: each of a frame's ships drawn is
///   among its ships;
/// - **no ship hovered without boxes**: a hover with no ship's box finds
///   no ship and not the player's;
/// - **a click on nothing is refused**: a click in space far from
///   anything, with no boxes, is refused, and the frame after it equals
///   the frame before;
/// - **a target has a record**: a frame's target is among its ships;
/// - **target brackets only round the target**: a frame has target
///   brackets only with a target, and they name it;
/// - **interference within 100**: a frame's interference is at most 100;
/// - **only a shown stellar has an IFF**: each stellar not shown has
///   none;
/// - **the escort rows in their order**: a frame's escort rows are of
///   each [`EscortClass`] in [`EscortClass::ALL`]'s order;
/// - **escorts fly only with a row present**: a frame with an escort
///   flying ([`FrameHud::escorts_flying`]) has at least one of its escort
///   rows present, as every escort flying leads a class;
/// - **a weapon select waits for the step**: Weapon Select
///   ([`FlightCommand::SelectWeapon`]) is answered done or refused, never
///   with anything else, and the frame after it equals the frame before,
///   as the press is taken by the next step;
/// - **the travel facts agree**: a frame's warp end is due only while the
///   jump itself is under way ([`FlightFrame::warping`]), the jump itself
///   only while a jump is under way, a leave is due only while landed, and
///   the hold has expired only with the fate
///   [`PlayerFate::Holding`]` { ticks_left: 0 }`, [`PlayerFate::Fading`] or
///   [`PlayerFate::GameOver`]: an expired hold reads `Holding { ticks_left:
///   0 }` until the fade begins ([`FlightCommand::BeginFade`]);
/// - **no arrival before the warp's end**: with the warp's end not due,
///   [`FlightCommand::Arrive`] is refused, and the frame after it equals
///   the frame before;
/// - **the fade waits for the hold**: before the hold has expired,
///   [`FlightCommand::BeginFade`] and [`FlightCommand::EndPlay`] are
///   refused, and the frame after them equals the frame before;
/// - **a jump's answer is its kind**: [`FlightCommand::Jump`] answers
///   [`Answer::Jumped`], [`Answer::JumpRefused`] or [`Answer::Refused`]; a
///   refusal leaves [`FlightFrame::jump_under_way`] as it was (a disabled
///   ship's J is refused mid-jump too), and after [`Answer::Jumped`] a
///   jump is under way. Checked last, as a jump begun changes the flight;
/// - **no plunder without a boarding**: with no boarding under way
///   ([`CommSource::boarding`] none), a take in the plunder dialog
///   ([`FlightCommand::Plunder`]) is refused, and the frame after it
///   equals the frame before. Checked before the jump;
/// - **the port answers only while landed**: in flight
///   ([`FlightFrame::landed`] none), [`PortSource::market`],
///   [`PortSource::news`] and [`PortSource::news_picture`] are none and
///   [`PortSource::recharge_enabled`] is false;
/// - **no trade without a market**: with no exchange
///   ([`PortSource::market`] none), a [`FlightCommand::Trade`] is not
///   answered [`Answer::Traded`]`(Ok(_))`, and the frame after it equals
///   the frame before. Checked before the jump.
///
/// - **a failed flight shows nothing**: a flight whose `opening` says why
///   it could not start ([`FlightOpening::failure`]) lists no weapons in
///   it ([`FlightOpening::weapons`]), fills the default frame, refuses
///   every command but a description's load
///   ([`FlightCommand::LoadDescription`]), which it answers with
///   [`Answer::Description`], and drains nothing. Checked first.
///
/// The error says which clause failed.
pub fn check_contract(sim: &mut impl FlightSim, opening: &FlightOpening) -> Result<(), String> {
    let mut undrained = FlightFrame::default();
    sim.frame(&mut undrained);
    let mut drained = Vec::new();
    sim.drain(&mut drained);
    let first = drained.len();
    sim.drain(&mut drained);
    if drained.len() != first {
        return Err(format!(
            "drain empties: a second drain added {:?}",
            &drained[first..]
        ));
    }
    let mut after_drains = FlightFrame::default();
    sim.frame(&mut after_drains);
    if after_drains != undrained {
        return Err(format!(
            "a drain changes no frame: the frame was {undrained:?}, then {after_drains:?}"
        ));
    }
    check_failed(sim, opening, &drained)?;

    let mut once = FlightFrame::default();
    sim.frame(&mut once);
    let mut twice = unlike(&once);
    sim.frame(&mut twice);
    if twice != once {
        return Err(format!(
            "a frame fills the same twice: {once:?}, then {twice:?}"
        ));
    }
    check_asks(sim, &once)?;

    let answer = sim.apply(refused_command());
    if !answer.is_refused() {
        return Err(format!(
            "a refusal changes nothing: {:?} answered {answer:?}",
            refused_command()
        ));
    }
    let mut after = unlike(&once);
    sim.frame(&mut after);
    if after != once {
        return Err(format!(
            "a refusal changes nothing: the frame was {once:?}, then {after:?}"
        ));
    }

    let mut report = StepReport {
        steps: vec![StepRun::default()],
        stopped: true,
    };
    sim.step(&StepInput::default(), 0, &mut report);
    if report != StepReport::default() {
        return Err(format!(
            "no step asked, none run: the report was {report:?}"
        ));
    }
    check_world(sim, &once)?;
    check_hud(&once)?;
    check_weapon_select(sim, &once)?;
    check_travel(sim, &once)?;
    check_plunder(sim, &once)?;
    check_port(sim, &once)?;
    check_jump(sim, &once)
}

/// [`check_contract`]'s clause on a flight whose `opening` says it could
/// not start, `drained` what its first drain gave: the opening lists no
/// weapons, and the flight fills the default frame, refuses every command
/// (a setting, which a flight that started always takes, among them) but
/// a description's load, which it answers with a text, and drains
/// nothing.
fn check_failed(
    sim: &mut impl FlightSim,
    opening: &FlightOpening,
    drained: &[FlightEvent],
) -> Result<(), String> {
    let Some(reason) = opening.failure.clone() else {
        return Ok(());
    };
    let clause = "a failed flight shows nothing";
    if !drained.is_empty() {
        return Err(format!(
            "{clause}: failed ({reason}), it drained {drained:?}"
        ));
    }
    let mut frame = unlike(&FlightFrame::default());
    sim.frame(&mut frame);
    if frame != FlightFrame::default() {
        return Err(format!("{clause}: failed ({reason}), it filled {frame:?}"));
    }
    let answer = sim.apply(FlightCommand::SetWindowWidth(1024));
    if !answer.is_refused() {
        return Err(format!(
            "{clause}: failed ({reason}), a setting was answered {answer:?}"
        ));
    }
    let answer = sim.apply(FlightCommand::LoadDescription {
        raw: Some("A dësc.".to_owned()),
        movie: None,
    });
    if !matches!(answer, Answer::Description(_)) {
        return Err(format!(
            "{clause}: failed ({reason}), a description's load was answered {answer:?}"
        ));
    }
    if !opening.weapons.is_empty() {
        return Err(format!(
            "{clause}: failed ({reason}), it listed weapons {:?}",
            opening.weapons
        ));
    }
    Ok(())
}

/// [`check_contract`]'s ask clause, `once` the frame filled before: each
/// ask leaves the frame as it was.
fn check_asks(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let asks = [
        FlightCommand::Hover {
            at: once.player().position,
            drawn: DrawnBoxes::default(),
        },
        FlightCommand::ComingTraffic,
        FlightCommand::LoadDescription {
            raw: Some("A dësc.".to_owned()),
            movie: None,
        },
    ];
    for ask in asks {
        let sent = format!("{ask:?}");
        let _ = sim.apply(ask);
        let mut after = unlike(once);
        sim.frame(&mut after);
        if after != *once {
            return Err(format!(
                "an ask changes no frame: {sent} took the frame from {once:?} to {after:?}"
            ));
        }
    }
    Ok(())
}

/// The trade [`check_contract`]'s market clause sends: a ton of the first
/// row's food.
fn contract_trade() -> FlightCommand {
    FlightCommand::Trade(Order {
        row: 0,
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::Click,
    })
}

/// [`check_contract`]'s clauses on the port, `once` the frame filled
/// before.
fn check_port(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    if !once.docked() {
        let clause = "the port answers only while landed";
        if let Some(market) = sim.market() {
            return Err(format!("{clause}: an exchange in flight, {market:?}"));
        }
        if let Some(news) = sim.news() {
            return Err(format!("{clause}: the bar's news in flight, {news:?}"));
        }
        if let Some(picture) = sim.news_picture() {
            return Err(format!("{clause}: a news picture in flight, {picture}"));
        }
        if sim.recharge_enabled() {
            return Err(format!("{clause}: Recharge enabled in flight"));
        }
    }
    if sim.market().is_some() {
        return Ok(());
    }
    let clause = "no trade without a market";
    let sent = format!("{:?}", contract_trade());
    let answer = sim.apply(contract_trade());
    if matches!(answer, Answer::Traded(Ok(_))) {
        return Err(format!("{clause}: {sent} answered {answer:?}"));
    }
    let mut after = unlike(once);
    sim.frame(&mut after);
    if after != *once {
        return Err(format!("{clause}: the frame was {once:?}, then {after:?}"));
    }
    Ok(())
}

/// [`check_contract`]'s plunder clause, `once` the frame filled before.
fn check_plunder(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    if sim.boarding().is_some() {
        return Ok(());
    }
    refused_unchanged(
        sim,
        once,
        FlightCommand::Plunder(Take::Cargo),
        "no plunder without a boarding",
    )
}

/// [`check_contract`]'s clauses on the travel facts, the arrival and the
/// fade, `once` the frame filled before.
fn check_travel(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let facts = |what: &str| Err(format!("the travel facts agree: {what}"));
    if once.warp_end_due && !once.warping {
        return facts("the warp's end is due with no warp");
    }
    if once.warping && !once.jump_under_way {
        return facts("a warp with no jump under way");
    }
    if once.leave_due && !once.docked() {
        return facts("a leave due in flight");
    }
    if once.hold_expired
        && !matches!(
            once.fate,
            PlayerFate::Holding { ticks_left: 0 } | PlayerFate::Fading | PlayerFate::GameOver
        )
    {
        return facts(&format!("the hold expired with the fate {:?}", once.fate));
    }
    if !once.warp_end_due {
        refused_unchanged(
            sim,
            once,
            FlightCommand::Arrive,
            "no arrival before the warp's end",
        )?;
    }
    if !once.hold_expired {
        for command in [FlightCommand::BeginFade, FlightCommand::EndPlay] {
            refused_unchanged(sim, once, command, "the fade waits for the hold")?;
        }
    }
    Ok(())
}

/// Sends `command`, which `clause` says must be refused, and checks that
/// it is, and that the frame after it equals `once`.
fn refused_unchanged(
    sim: &mut impl FlightSim,
    once: &FlightFrame,
    command: FlightCommand,
    clause: &str,
) -> Result<(), String> {
    let sent = format!("{command:?}");
    let answer = sim.apply(command);
    if !answer.is_refused() {
        return Err(format!("{clause}: {sent} answered {answer:?}"));
    }
    let mut after = unlike(once);
    sim.frame(&mut after);
    if after != *once {
        return Err(format!("{clause}: the frame was {once:?}, then {after:?}"));
    }
    Ok(())
}

/// [`check_contract`]'s jump clause, `once` the frame filled before.
fn check_jump(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let answer = sim.apply(FlightCommand::Jump);
    let mut after = unlike(once);
    sim.frame(&mut after);
    let clause = "a jump's answer is its kind";
    match answer {
        Answer::Jumped(_) if !after.jump_under_way => {
            Err(format!("{clause}: {answer:?} with no jump under way"))
        }
        Answer::JumpRefused { .. } | Answer::Refused
            if after.jump_under_way != once.jump_under_way =>
        {
            Err(format!(
                "{clause}: {answer:?} took the jump under way from {} to {}",
                once.jump_under_way, after.jump_under_way
            ))
        }
        Answer::Jumped(_) | Answer::JumpRefused { .. } | Answer::Refused => Ok(()),
        _ => Err(format!("{clause}: it answered {answer:?}")),
    }
}

/// [`check_contract`]'s clauses on the HUD and the target in `once`, the
/// frame filled before.
fn check_hud(once: &FlightFrame) -> Result<(), String> {
    if let Some(target) = &once.target
        && once.ship(ShipRef::Npc(target.npc)).is_none()
    {
        return Err(format!("a target has a record: {:?} has none", target.npc));
    }
    if let Some(brackets) = &once.hud.target_brackets
        && once.target.as_ref().map(|target| target.npc) != Some(brackets.npc)
    {
        return Err(format!(
            "target brackets only round the target: round {:?}, the target {:?}",
            brackets.npc,
            once.target.as_ref().map(|target| target.npc)
        ));
    }
    if once.hud.interference > 100 {
        return Err(format!(
            "interference within 100: it was {}",
            once.hud.interference
        ));
    }
    if let Some(stellar) = once
        .stellars
        .iter()
        .find(|stellar| !stellar.shown && stellar.iff.is_some())
    {
        return Err(format!(
            "only a shown stellar has an IFF: {:?} isn't shown, and has {:?}",
            stellar.id, stellar.iff
        ));
    }
    if !once
        .hud
        .escort_rows
        .iter()
        .map(|row| row.class)
        .eq(EscortClass::ALL)
    {
        return Err(format!(
            "the escort rows in their order: {:?}",
            once.hud.escort_rows.map(|row| row.class)
        ));
    }
    if once.hud.escorts_flying && !once.hud.escort_rows.iter().any(|row| row.present) {
        return Err(format!(
            "escorts fly only with a row present: none of {:?} is",
            once.hud.escort_rows
        ));
    }
    Ok(())
}

/// [`check_contract`]'s weapon select clause, `once` the frame filled
/// before.
fn check_weapon_select(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let answer = sim.apply(FlightCommand::SelectWeapon { backwards: false });
    if !matches!(answer, Answer::Done | Answer::Refused) {
        return Err(format!(
            "a weapon select waits for the step: it answered {answer:?}"
        ));
    }
    let mut after = unlike(once);
    sim.frame(&mut after);
    if after != *once {
        return Err(format!(
            "a weapon select waits for the step: the frame was {once:?}, then {after:?}"
        ));
    }
    Ok(())
}

/// [`check_contract`]'s clauses on the world: the ships' records and
/// those drawn, the hover and a click in space, `once` the frame filled
/// before.
fn check_world(sim: &mut impl FlightSim, once: &FlightFrame) -> Result<(), String> {
    let players = once
        .ships
        .iter()
        .filter(|record| record.ship == ShipRef::Player)
        .count();
    let first = once.ships.first().map(|record| record.ship);
    if players != 1 || first != Some(ShipRef::Player) {
        return Err(format!(
            "the player's record first, once: the ships were {:?}",
            once.ships
                .iter()
                .map(|record| record.ship)
                .collect::<Vec<_>>()
        ));
    }
    if let Some((stray, _)) = once
        .drawn
        .iter()
        .find(|(ship, _)| once.ship(*ship).is_none())
    {
        return Err(format!(
            "every ship drawn has a record: {stray:?} is drawn with none"
        ));
    }
    let at = once.player().position;
    let hover = FlightCommand::Hover {
        at,
        drawn: DrawnBoxes::default(),
    };
    if let Answer::Hovered(Some(hover @ (Hover::Ship(_) | Hover::Player))) = sim.apply(hover) {
        return Err(format!(
            "no ship hovered without boxes: {hover:?} at {at:?}"
        ));
    }
    let far = Vec2::new(1.0e9, 1.0e9);
    let click = FlightCommand::ClickInSpace {
        at: far,
        drawn: DrawnBoxes::default(),
    };
    let answer = sim.apply(click);
    if !answer.is_refused() {
        return Err(format!(
            "a click on nothing is refused: a click at {far:?} answered {answer:?}"
        ));
    }
    let mut after = unlike(once);
    sim.frame(&mut after);
    if after != *once {
        return Err(format!(
            "a click on nothing is refused: the frame was {once:?}, then {after:?}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use nova_sim::{DescMovie, SimSound};

    use super::*;

    #[test]
    fn the_players_record_faces_its_heading_and_is_boxed_by_its_own_frames() {
        let ship = record::player(129, Vec2::new(3.0, 4.0), 90.0);
        assert_eq!(ship.ship, ShipRef::Player);
        assert_eq!(ship.class, ShipId(129));
        assert_eq!(ship.position, Vec2::new(3.0, 4.0));
        assert_eq!(ship.heading.to_bits(), 90.0_f32.to_bits());
        assert_eq!(ship.framed, Some(ShipId(129)));
    }

    /// [`check_contract`] on `fake`, with the opening it scripts.
    fn contract(fake: &mut FakeFlight) -> Result<(), String> {
        let opening = fake.opening();
        check_contract(fake, &opening)
    }

    fn stellar(id: i16) -> FlightFrame {
        FlightFrame {
            nav_mode: NavMode::Stellar(Some(StellarId(id))),
            ..FlightFrame::default()
        }
    }

    #[test]
    fn a_fake_with_nothing_scripted_meets_the_contract() {
        assert_eq!(contract(&mut FakeFlight::new()), Ok(()));
    }

    #[test]
    fn a_failed_fake_with_nothing_else_scripted_meets_the_contract() {
        let mut fake = FakeFlight::new();
        fake.fail("no chär");
        assert_eq!(contract(&mut fake), Ok(()));
    }

    #[test]
    fn a_failed_flight_that_shows_a_frame_or_drains_breaks_the_contract() {
        let mut showing = FakeFlight::new();
        showing.fail("no chär");
        showing.set_frame(stellar(128));
        let clause = contract(&mut showing).expect_err("broken");
        assert!(
            clause.starts_with("a failed flight shows nothing"),
            "{clause}"
        );
        assert!(clause.contains("filled"), "{clause}");
        let mut draining = FakeFlight::new();
        draining.fail("no chär");
        draining.queue_event(FlightEvent::Sound(SimSound::NavSelected));
        let clause = contract(&mut draining).expect_err("broken");
        assert!(
            clause.starts_with("a failed flight shows nothing"),
            "{clause}"
        );
        assert!(clause.contains("drained"), "{clause}");
    }

    /// A flight that failed but takes the settings, or, when its `0` is
    /// false, refuses every command, a description's load among them.
    struct TakesSettings(bool);

    impl FlightSim for TakesSettings {
        fn frame(&self, into: &mut FlightFrame) {
            *into = FlightFrame::default();
        }

        fn step(&mut self, _input: &StepInput, _steps: u32, into: &mut StepReport) {
            into.clear();
        }

        fn apply(&mut self, command: FlightCommand) -> Answer {
            if self.0 && matches!(command, FlightCommand::SetWindowWidth(_)) {
                Answer::Done
            } else {
                Answer::Refused
            }
        }

        fn drain(&mut self, _into: &mut Vec<FlightEvent>) {}
    }

    impl MapSource for TakesSettings {}

    impl CommSource for TakesSettings {}

    impl PortSource for TakesSettings {}

    impl NameSource for TakesSettings {}

    impl PilotSource for TakesSettings {}

    #[test]
    fn a_failed_flight_that_takes_a_command_breaks_the_contract() {
        let clause = check_contract(&mut TakesSettings(true), &FlightOpening::failed("no chär"))
            .expect_err("broken");
        assert!(
            clause.starts_with("a failed flight shows nothing"),
            "{clause}"
        );
        assert!(clause.contains("answered Done"), "{clause}");
    }

    #[test]
    fn a_failed_flight_that_refuses_a_descriptions_load_breaks_the_contract() {
        let clause = check_contract(&mut TakesSettings(false), &FlightOpening::failed("no chär"))
            .expect_err("broken");
        assert!(
            clause.starts_with("a failed flight shows nothing"),
            "{clause}"
        );
        assert!(
            clause.contains("a description's load was answered Refused"),
            "{clause}"
        );
    }

    #[test]
    fn a_failed_flight_that_lists_weapons_breaks_the_contract() {
        let mut fake = FakeFlight::new();
        fake.fail("no chär");
        fake.set_weapons(vec![WeaponId(128)]);
        let clause = contract(&mut fake).expect_err("broken");
        assert!(
            clause.starts_with("a failed flight shows nothing"),
            "{clause}"
        );
        assert!(clause.contains("128"), "{clause}");
    }

    #[test]
    fn a_failed_fake_says_why_and_refuses_every_command_recording_it() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.opening().failure, None);
        fake.queue_answer(Answer::Done);
        fake.fail("no chär");
        assert_eq!(fake.opening(), FlightOpening::failed("no chär"));
        for command in [
            FlightCommand::CueCourse,
            FlightCommand::CommTicked,
            FlightCommand::SetSoundQuiet(true),
        ] {
            assert_eq!(fake.apply(command.clone()), Answer::Refused, "{command:?}");
        }
        assert_eq!(fake.applied().len(), 3);
    }

    #[test]
    fn the_fake_tells_an_answers_events_only_once_the_command_is_answered() {
        let mut fake = FakeFlight::new();
        let told = FlightEvent::Sound(SimSound::NavSelected);
        fake.queue_answer_telling(Answer::Done, stellar(128), vec![told.clone()]);
        let mut drained = Vec::new();
        fake.drain(&mut drained);
        assert_eq!(drained, [], "not before the command");
        assert_eq!(fake.apply(refused_command()), Answer::Done);
        let mut shown = FlightFrame::default();
        fake.frame(&mut shown);
        assert_eq!(shown, stellar(128));
        fake.drain(&mut drained);
        assert_eq!(drained, [told], "after it");
    }

    #[test]
    fn the_fake_answers_the_settings_done_without_taking_an_answer() {
        let mut fake = FakeFlight::new();
        fake.queue_answer(Answer::Stellar(StellarId(130)));
        for command in [
            FlightCommand::SetWindowWidth(1430),
            FlightCommand::SetDoubleTimeKey(true),
            FlightCommand::SetSoundQuiet(false),
        ] {
            assert_eq!(fake.apply(command.clone()), Answer::Done, "{command:?}");
        }
        assert_eq!(
            fake.apply(FlightCommand::SelectStellar(StellarPick::Nearest)),
            Answer::Stellar(StellarId(130)),
            "the answer queued is the next command's"
        );
    }

    #[test]
    fn the_fake_desk_reads_as_scripted_and_records_each_edit() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.reading(), None);
        assert_eq!(fake.edit(PilotEdit::Credits(5)), Ok(()));
        let reading = DeskReading {
            name: "Ada".to_owned(),
            credits: 1000,
            reserves: Reserves::default(),
            date: nova_sim::GameDate::new(1177, 6, 23).expect("a date"),
            date_text: "June 23, 1177".to_owned(),
            system: SystemId(130),
            landed: None,
            bits: Vec::new(),
        };
        fake.desk_mut().reading = Some(reading.clone());
        fake.desk_mut().refusal = Some(EditRefusal::InFlight);
        assert_eq!(fake.reading(), Some(reading));
        let to = PilotEdit::MoveTo {
            system: SystemId(131),
            stellar: StellarId(140),
        };
        assert_eq!(fake.edit(to), Err(EditRefusal::InFlight));
        assert_eq!(fake.edits(), [PilotEdit::Credits(5), to]);
    }

    #[test]
    fn the_fake_fills_the_plain_texts_and_answers_the_hire_as_scripted() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.plain_text("Hi <PN>."), "Hi <PN>.");
        assert!(!fake.can_hire_escorts());
        fake.names_mut()
            .plain_texts
            .push(("Hi <PN>.".to_owned(), "Hi Ada.".to_owned()));
        fake.port_mut().can_hire_escorts = true;
        assert_eq!(fake.plain_text("Hi <PN>."), "Hi Ada.");
        assert_eq!(fake.plain_text("Bye."), "Bye.");
        assert!(fake.can_hire_escorts());
    }

    #[test]
    fn a_fake_showing_a_frame_meets_the_contract() {
        let mut fake = FakeFlight::new();
        fake.set_frame(stellar(128));
        fake.queue_event(FlightEvent::Sound(SimSound::NavSelected));
        assert_eq!(contract(&mut fake), Ok(()));
    }

    #[test]
    fn the_fake_shows_the_frame_it_was_given() {
        let mut fake = FakeFlight::new();
        fake.set_frame(stellar(128));
        let mut frame = FlightFrame::default();
        assert_eq!(fake.fills(), 0);
        fake.frame(&mut frame);
        assert_eq!(frame, stellar(128));
        fake.frame(&mut frame);
        assert_eq!(fake.fills(), 2, "each fill counted");
    }

    #[test]
    fn the_fake_records_commands_in_order_and_answers_from_its_queue() {
        let mut fake = FakeFlight::new();
        fake.queue_answer(Answer::Stellar(StellarId(130)));
        fake.queue_answer(Answer::Stellar(StellarId(129)));
        let commands = [
            FlightCommand::SelectStellar(StellarPick::Slot(1)),
            FlightCommand::SelectStellar(StellarPick::Nearest),
            FlightCommand::SelectStellar(StellarPick::Slot(0)),
        ];
        let answers: Vec<_> = commands
            .iter()
            .map(|command| fake.apply(command.clone()))
            .collect();
        assert_eq!(
            answers,
            [
                Answer::Stellar(StellarId(130)),
                Answer::Stellar(StellarId(129)),
                Answer::Refused,
            ]
        );
        assert_eq!(fake.applied(), commands);
    }

    #[test]
    fn the_fake_hands_out_its_events_once_after_what_was_drained() {
        let mut fake = FakeFlight::new();
        let sounds = [SimSound::NavSelected, SimSound::HyperSelected];
        for sound in sounds {
            fake.queue_event(FlightEvent::Sound(sound));
        }
        let kept = FlightEvent::Sound(SimSound::NavSelected);
        let mut drained = vec![kept.clone()];
        fake.drain(&mut drained);
        assert_eq!(
            drained,
            [
                kept,
                FlightEvent::Sound(SimSound::NavSelected),
                FlightEvent::Sound(SimSound::HyperSelected),
            ]
        );
        let mut again = Vec::new();
        fake.drain(&mut again);
        assert_eq!(again, []);
    }

    #[test]
    fn the_fake_reports_steps_from_its_queue_and_counts_them() {
        let mut fake = FakeFlight::new();
        let scripted = StepReport {
            steps: vec![StepRun::default(); 2],
            stopped: true,
        };
        fake.queue_report(scripted.clone());
        assert_eq!(fake.steps(), 0);
        let held = StepInput {
            cloak: true,
            ..StepInput::default()
        };
        let mut report = StepReport::default();
        fake.step(&held, 2, &mut report);
        assert_eq!(report, scripted, "copied into the caller's report");
        for steps in [3, 0] {
            fake.step(&StepInput::default(), steps, &mut report);
            assert_eq!(report, StepReport::default(), "none run once empty");
        }
        assert_eq!(fake.steps(), 3, "the calls");
        assert_eq!(fake.steps_asked(), 5, "the steps asked");
        assert_eq!(
            fake.inputs(),
            [held, StepInput::default(), StepInput::default()]
        );
    }

    #[test]
    fn the_fake_takes_every_gate_but_those_refused() {
        let mut fake = FakeFlight::new();
        let gates = |fake: &FakeFlight| {
            let mut frame = FlightFrame::default();
            fake.frame(&mut frame);
            frame.gates
        };
        assert_eq!(gates(&fake), Gates::EVERY);
        fake.refuse(PlayerCommand::Nav);
        let refused = Gates::EVERY.refusing(PlayerCommand::Nav);
        assert_eq!(gates(&fake), refused, "in the frame");
        fake.set_frame(stellar(128));
        assert_eq!(gates(&fake), refused, "whatever frame is scripted later");
        fake.take(PlayerCommand::Nav);
        assert_eq!(gates(&fake), Gates::EVERY);
        fake.frame_mut().gates = Gates::NONE;
        assert_eq!(gates(&fake), Gates::NONE, "as the frame sets them");
    }

    #[test]
    fn the_fake_answers_nothing_on_the_map_until_scripted() {
        let fake = FakeFlight::new();
        assert_eq!(fake.map_drawing(None, &[], MapMarks::NONE), None);
        assert_eq!(fake.map_panel(None, None), None);
        assert_eq!(fake.map_border_sources(), None);
        assert_eq!(fake.find_map_system("sol"), None);
        assert_eq!(fake.offer_map_marks(MissionId(128)), MapMarks::NONE);
        assert_eq!(fake.map_nav(), None);
        assert_eq!(fake.stellar_name(StellarId(128)), None);
        assert!(!fake.has_explored(SystemId(130)));
        assert_eq!(fake.galaxy(), None);
    }

    #[test]
    fn the_fake_answers_the_map_as_scripted() {
        let mut fake = FakeFlight::new();
        let marks = MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: false,
        };
        let saved = SavedNav::of(NavMode::Off);
        let floating = FloatingLinks {
            current: nova_sim::Vec2::new(1.0, 2.0),
            links: vec![(SystemId(131), nova_sim::Vec2::new(3.0, 4.0))],
        };
        let map = fake.map_mut();
        map.search = Some(SearchOutcome::Found(SystemId(131)));
        map.border_sources = Some(BorderSources::default());
        map.mission_systems = vec![SystemId(132)];
        map.map_preselection = Some(SystemId(131));
        map.floating_preselection = Some(SystemId(132));
        map.hypergate_cycle = vec![None, Some(SystemId(131))];
        map.marks = Some(marks);
        map.map_nav = Some(saved);
        map.floating = floating.clone();
        map.stellar_names = vec![(StellarId(128), "Earth".to_owned())];
        map.explored = vec![SystemId(130)];
        let galaxy = OverlaidGalaxy::new(Vec::new(), nova_sim::story::GalaxyOverlay::default());
        map.galaxy = Some(galaxy.clone());
        assert_eq!(
            fake.find_map_system("x"),
            Some(SearchOutcome::Found(SystemId(131)))
        );
        assert_eq!(fake.map_border_sources(), Some(BorderSources::default()));
        assert_eq!(fake.map_mission_systems(), [SystemId(132)]);
        assert_eq!(fake.map_preselection(), Some(SystemId(131)));
        assert_eq!(fake.floating_preselection(), Some(SystemId(132)));
        assert_eq!(fake.hypergate_cycle(), [None, Some(SystemId(131))]);
        assert_eq!(fake.offer_map_marks(MissionId(1)), marks);
        assert_eq!(fake.computer_map_marks(None), marks);
        assert_eq!(fake.mission_info_map_marks(Some(0)), marks);
        assert_eq!(fake.map_nav(), Some(saved));
        assert_eq!(fake.floating_map_places(), floating);
        assert_eq!(fake.stellar_name(StellarId(128)).as_deref(), Some("Earth"));
        assert_eq!(fake.stellar_name(StellarId(129)), None);
        assert!(fake.has_explored(SystemId(130)));
        assert!(!fake.has_explored(SystemId(131)));
        assert_eq!(fake.galaxy(), Some(&galaxy));
    }

    #[test]
    fn the_fake_records_the_maps_questions_whose_arguments_matter() {
        let fake = FakeFlight::new();
        let marks = MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: true,
        };
        fake.map_drawing(Some(&[SystemId(131)]), &[SystemId(132)], marks);
        fake.map_panel(Some(SystemId(130)), None);
        fake.find_map_system("alpha");
        fake.offer_map_marks(MissionId(128));
        fake.computer_map_marks(Some(MissionId(129)));
        fake.mission_info_map_marks(Some(2));
        fake.map_preselection();
        assert_eq!(
            fake.asked(),
            [
                MapAsk::Drawing {
                    hypergate: Some(vec![SystemId(131)]),
                    missions: vec![SystemId(132)],
                    marks,
                },
                MapAsk::Panel {
                    selected: Some(SystemId(130)),
                    hypergate: None,
                },
                MapAsk::Find("alpha".to_owned()),
                MapAsk::OfferMarks(MissionId(128)),
                MapAsk::ComputerMarks(Some(MissionId(129))),
                MapAsk::MissionInfoMarks(Some(2)),
            ]
        );
    }

    /// What a broken sim gets wrong.
    #[derive(Clone, Copy, Debug)]
    enum Fault {
        /// Every drain hands out another event.
        NeverEmpties,
        /// A drain changes the frame.
        DrainChangesTheFrame,
        /// An ask changes the frame.
        AskChangesTheFrame,
        /// A fill leaves the gates as they were.
        SkipsTheGates,
        /// A fill leaves the death descs as they were.
        SkipsTheDeathDescs,
        /// A fill leaves the nav mode as it was.
        SkipsAField,
        /// The refused command is answered.
        AnswersTheRefusal,
        /// The refused command changes the frame.
        RefusalChangesTheFrame,
        /// A frame asked for no step reports one.
        StepsUnasked,
        /// The frame's ships begin with an NPC's.
        NpcFirst,
        /// The frame draws a ship it has no record of.
        DrawsAStray,
        /// The hover finds the player's ship with no boxes.
        HoversWithoutBoxes,
        /// The click on nothing is answered.
        AnswersTheClick,
        /// The click on nothing changes the frame.
        ClickChangesTheFrame,
        /// The frame targets a ship it has no record of.
        TargetsAStray,
        /// The frame brackets a ship with no target.
        BracketsWithoutATarget,
        /// The frame brackets another ship than its target.
        BracketsAnother,
        /// The frame's interference is over 100.
        InterferenceOver100,
        /// A stellar not shown has an IFF.
        UnshownIff,
        /// The escort rows are out of order.
        RowsOutOfOrder,
        /// An escort flies with no row present.
        FliesWithNoRow,
        /// A take with no boarding is answered.
        PlundersNothing,
        /// A take with no boarding changes the frame.
        PlunderChangesTheFrame,
        /// An exchange in flight.
        MarketInFlight,
        /// The bar's news in flight.
        NewsInFlight,
        /// A news picture in flight.
        NewsPictureInFlight,
        /// Recharge enabled in flight.
        RechargeInFlight,
        /// A trade with no market goes through.
        TradesWithoutAMarket,
        /// A trade with no market changes the frame.
        TradeChangesTheFrame,
        /// Weapon Select is answered with a stellar.
        AnswersTheSelect,
        /// Weapon Select changes the frame.
        SelectChangesTheFrame,
        /// The warp's end is due with no warp.
        WarpEndsUnwarped,
        /// A warp with no jump under way.
        WarpsWithoutAJump,
        /// A leave is due in flight.
        LeavesInFlight,
        /// The hold has expired with the player flying.
        ExpiresFlying,
        /// The arrival is answered before the warp's end.
        ArrivesEarly,
        /// The arrival refused changes the frame.
        ArrivalChangesTheFrame,
        /// The fade begins before the hold has expired.
        FadesEarly,
        /// Play ends before the hold has expired.
        EndsEarly,
        /// The jump is answered with a stellar.
        AnswersTheJump,
        /// A refused jump puts a jump under way.
        RefusalJumps,
        /// A jump answered as begun leaves none under way.
        JumpsNowhere,
    }

    /// A [`FakeFlight`] with one [`Fault`].
    struct Broken {
        fake: FakeFlight,
        fault: Fault,
    }

    impl Broken {
        fn new(fault: Fault) -> Self {
            let mut fake = FakeFlight::new();
            fake.set_frame(stellar(128));
            Self { fake, fault }
        }
    }

    impl FlightSim for Broken {
        fn frame(&self, into: &mut FlightFrame) {
            let (gates, descs) = (into.gates, into.death_descs.clone());
            if !matches!(self.fault, Fault::SkipsAField) {
                self.fake.frame(into);
            }
            match self.fault {
                Fault::SkipsTheGates => into.gates = gates,
                Fault::SkipsTheDeathDescs => into.death_descs = descs,
                _ => {}
            }
            let npc = ShipRef::Npc(nova_sim::NpcId(7));
            let brackets = |id| nova_sim::brackets::TargetBrackets {
                npc: nova_sim::NpcId(id),
                expand: 0.0,
                standing: nova_sim::brackets::Standing::Neutral,
            };
            match self.fault {
                Fault::NpcFirst => into.ships.insert(0, record::npc(7, 129, Vec2::ZERO, 0.0)),
                Fault::DrawsAStray => into.drawn.push((npc, nova_sim::ShipLayer::Ships)),
                Fault::TargetsAStray => into.target = Some(record::target(7)),
                Fault::BracketsWithoutATarget => into.hud.target_brackets = Some(brackets(7)),
                Fault::BracketsAnother => {
                    into.ships.push(record::npc(7, 129, Vec2::ZERO, 0.0));
                    into.target = Some(record::target(7));
                    into.hud.target_brackets = Some(brackets(8));
                }
                Fault::InterferenceOver100 => into.hud.interference = 101,
                Fault::UnshownIff => into.stellars.push(nova_sim::flight_frame::FrameStellar {
                    shown: false,
                    ..record::stellar(StellarId(129))
                }),
                Fault::RowsOutOfOrder => into.hud.escort_rows.swap(0, 1),
                Fault::FliesWithNoRow => into.hud.escorts_flying = true,
                Fault::WarpEndsUnwarped => {
                    into.warp_end_due = true;
                    into.jump_under_way = true;
                }
                Fault::WarpsWithoutAJump => into.warping = true,
                Fault::LeavesInFlight => into.leave_due = true,
                Fault::ExpiresFlying => into.hold_expired = true,
                _ => {}
            }
        }

        fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport) {
            self.fake.step(input, steps, into);
            if matches!(self.fault, Fault::StepsUnasked) {
                into.steps.push(StepRun::default());
            }
        }

        fn apply(&mut self, command: FlightCommand) -> Answer {
            let click = matches!(command, FlightCommand::ClickInSpace { .. });
            let select = matches!(command, FlightCommand::SelectWeapon { .. });
            let arrive = matches!(command, FlightCommand::Arrive);
            let fade = matches!(command, FlightCommand::BeginFade);
            let end = matches!(command, FlightCommand::EndPlay);
            let jump = matches!(command, FlightCommand::Jump);
            let plunder = matches!(command, FlightCommand::Plunder(_));
            let trade = matches!(command, FlightCommand::Trade(_));
            let hover = matches!(command, FlightCommand::Hover { .. });
            if matches!(self.fault, Fault::AskChangesTheFrame) && command.is_ask() {
                self.fake.set_frame(stellar(129));
            }
            match self.fault {
                Fault::HoversWithoutBoxes if hover => Answer::Hovered(Some(Hover::Player)),
                Fault::TradesWithoutAMarket if trade => Answer::Traded(Ok(1)),
                Fault::TradeChangesTheFrame if trade => {
                    self.fake.set_frame(stellar(129));
                    Answer::Traded(Err(nova_sim::TradeRefusal::NoMarket))
                }
                Fault::PlundersNothing if plunder => Answer::Plundered {
                    taken: nova_sim::Taken::Nothing,
                    open: false,
                },
                Fault::PlunderChangesTheFrame if plunder => {
                    self.fake.set_frame(stellar(129));
                    Answer::Refused
                }
                Fault::ArrivesEarly if arrive => Answer::Arrived {
                    system: SystemId(131),
                    combat: Vec::new(),
                },
                Fault::ArrivalChangesTheFrame if arrive => {
                    self.fake.set_frame(stellar(129));
                    Answer::Refused
                }
                Fault::FadesEarly if fade => Answer::Done,
                Fault::EndsEarly if end => Answer::Done,
                Fault::AnswersTheJump if jump => Answer::Stellar(StellarId(128)),
                Fault::RefusalJumps if jump => {
                    self.fake.frame_mut().jump_under_way = true;
                    Answer::JumpRefused {
                        refusal: nova_sim::JumpRefusal::NoDestination,
                        told: false,
                    }
                }
                Fault::JumpsNowhere if jump => Answer::Jumped(SystemId(131)),
                Fault::AnswersTheSelect if select => Answer::Stellar(StellarId(128)),
                Fault::SelectChangesTheFrame if select => {
                    self.fake.set_frame(stellar(129));
                    Answer::Done
                }
                Fault::AnswersTheRefusal => Answer::Stellar(StellarId(128)),
                Fault::RefusalChangesTheFrame if !command.is_ask() => {
                    self.fake.set_frame(stellar(129));
                    self.fake.apply(command)
                }
                Fault::AnswersTheClick if click => Answer::InSpace(Hover::Player),
                Fault::ClickChangesTheFrame if click => {
                    self.fake.set_frame(stellar(129));
                    self.fake.apply(command)
                }
                _ => self.fake.apply(command),
            }
        }

        fn drain(&mut self, into: &mut Vec<FlightEvent>) {
            if matches!(self.fault, Fault::DrainChangesTheFrame) {
                self.fake.set_frame(stellar(129));
            }
            if matches!(self.fault, Fault::NeverEmpties) {
                self.fake
                    .queue_event(FlightEvent::Sound(SimSound::NavSelected));
            }
            self.fake.drain(into);
        }
    }

    impl MapSource for Broken {}

    impl NameSource for Broken {}

    impl PilotSource for Broken {}

    impl CommSource for Broken {
        fn boarding(&self) -> Option<PlunderView> {
            self.fake.boarding()
        }
    }

    impl PortSource for Broken {
        fn market(&self) -> Option<Market> {
            matches!(self.fault, Fault::MarketInFlight).then(Market::default)
        }

        fn news(&self) -> Option<&News> {
            static NEWS: News = News {
                commercial: String::new(),
                story: String::new(),
            };
            matches!(self.fault, Fault::NewsInFlight).then_some(&NEWS)
        }

        fn news_picture(&self) -> Option<i16> {
            matches!(self.fault, Fault::NewsPictureInFlight).then_some(9000)
        }

        fn recharge_enabled(&self) -> bool {
            matches!(self.fault, Fault::RechargeInFlight)
        }
    }

    /// The clause `fault` breaks, as `check_contract` names it.
    fn clause(fault: Fault) -> String {
        check_contract(&mut Broken::new(fault), &FlightOpening::default()).expect_err("broken")
    }

    #[test]
    fn a_drain_that_never_empties_breaks_the_contract() {
        assert!(clause(Fault::NeverEmpties).starts_with("drain empties"));
    }

    #[test]
    fn a_fill_that_skips_a_field_breaks_the_contract() {
        assert!(clause(Fault::SkipsAField).starts_with("a frame fills the same twice"));
    }

    #[test]
    fn a_fill_that_skips_the_gates_or_the_death_descs_breaks_the_contract() {
        for fault in [Fault::SkipsTheGates, Fault::SkipsTheDeathDescs] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("a frame fills the same twice"),
                "{fault:?}: {clause}"
            );
        }
    }

    #[test]
    fn a_drain_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::DrainChangesTheFrame);
        assert!(clause.starts_with("a drain changes no frame"), "{clause}");
    }

    #[test]
    fn an_ask_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::AskChangesTheFrame);
        assert!(clause.starts_with("an ask changes no frame"), "{clause}");
        assert!(clause.contains("Hover"), "the first ask: {clause}");
    }

    #[test]
    fn an_answered_refusal_breaks_the_contract() {
        let clause = clause(Fault::AnswersTheRefusal);
        assert!(clause.starts_with("a refusal changes nothing"), "{clause}");
        assert!(clause.contains("answered"), "{clause}");
    }

    #[test]
    fn a_refusal_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::RefusalChangesTheFrame);
        assert!(clause.starts_with("a refusal changes nothing"), "{clause}");
        assert!(clause.contains("the frame was"), "{clause}");
    }

    #[test]
    fn a_step_unasked_breaks_the_contract() {
        assert!(clause(Fault::StepsUnasked).starts_with("no step asked, none run"));
    }

    #[test]
    fn an_npc_first_breaks_the_contract() {
        assert!(clause(Fault::NpcFirst).starts_with("the player's record first, once"));
    }

    #[test]
    fn a_second_player_record_breaks_the_contract() {
        let mut fake = FakeFlight::new();
        fake.frame_mut().ships.push(FrameShip::default());
        let clause = contract(&mut fake).expect_err("broken");
        assert!(
            clause.starts_with("the player's record first, once"),
            "{clause}"
        );
    }

    #[test]
    fn a_ship_drawn_with_no_record_breaks_the_contract() {
        assert!(clause(Fault::DrawsAStray).starts_with("every ship drawn has a record"));
    }

    #[test]
    fn a_hover_without_boxes_breaks_the_contract() {
        assert!(clause(Fault::HoversWithoutBoxes).starts_with("no ship hovered without boxes"));
    }

    #[test]
    fn an_answered_click_on_nothing_breaks_the_contract() {
        let clause = clause(Fault::AnswersTheClick);
        assert!(
            clause.starts_with("a click on nothing is refused"),
            "{clause}"
        );
        assert!(clause.contains("answered"), "{clause}");
    }

    #[test]
    fn a_click_on_nothing_that_changes_the_frame_breaks_the_contract() {
        let clause = clause(Fault::ClickChangesTheFrame);
        assert!(
            clause.starts_with("a click on nothing is refused"),
            "{clause}"
        );
        assert!(clause.contains("the frame was"), "{clause}");
    }

    #[test]
    fn a_target_with_no_record_breaks_the_contract() {
        assert!(clause(Fault::TargetsAStray).starts_with("a target has a record"));
    }

    #[test]
    fn target_brackets_without_a_target_or_round_another_break_the_contract() {
        for fault in [Fault::BracketsWithoutATarget, Fault::BracketsAnother] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("target brackets only round the target"),
                "{fault:?}: {clause}"
            );
        }
    }

    #[test]
    fn interference_over_100_breaks_the_contract() {
        assert!(clause(Fault::InterferenceOver100).starts_with("interference within 100"));
    }

    #[test]
    fn an_iff_on_a_stellar_not_shown_breaks_the_contract() {
        assert!(clause(Fault::UnshownIff).starts_with("only a shown stellar has an IFF"));
    }

    #[test]
    fn escort_rows_out_of_order_break_the_contract() {
        assert!(clause(Fault::RowsOutOfOrder).starts_with("the escort rows in their order"));
    }

    #[test]
    fn an_escort_flying_with_no_row_present_breaks_the_contract() {
        assert!(clause(Fault::FliesWithNoRow).starts_with("escorts fly only with a row present"));
    }

    #[test]
    fn a_take_with_no_boarding_answered_or_changing_the_frame_breaks_the_contract() {
        for fault in [Fault::PlundersNothing, Fault::PlunderChangesTheFrame] {
            assert!(
                clause(fault).starts_with("no plunder without a boarding"),
                "{fault:?}: {}",
                clause(fault)
            );
        }
    }

    #[test]
    fn a_port_answering_in_flight_breaks_the_contract() {
        for fault in [
            Fault::MarketInFlight,
            Fault::NewsInFlight,
            Fault::NewsPictureInFlight,
            Fault::RechargeInFlight,
        ] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("the port answers only while landed"),
                "{fault:?}: {clause}"
            );
        }
    }

    #[test]
    fn a_trade_with_no_market_answered_or_changing_the_frame_breaks_the_contract() {
        for fault in [Fault::TradesWithoutAMarket, Fault::TradeChangesTheFrame] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("no trade without a market"),
                "{fault:?}: {clause}"
            );
        }
    }

    #[test]
    fn a_fake_docked_with_its_port_open_meets_the_contract() {
        let mut docked = FakeFlight::new();
        docked.set_frame(FlightFrame {
            landed: Some(StellarId(128)),
            ..stellar(128)
        });
        let port = docked.port_mut();
        port.market = Some(Market::default());
        port.news = Some(News::default());
        port.news_picture = Some(9000);
        port.recharge_enabled = true;
        assert_eq!(
            contract(&mut docked),
            Ok(()),
            "docked, with an exchange, which may trade as it will"
        );
    }

    #[test]
    fn the_fake_names_nothing_until_scripted_then_as_scripted() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.ship_name(ShipId(129)), None);
        assert_eq!(fake.long_date(), "");
        let names = fake.names_mut();
        names.ship_names = vec![(ShipId(129), "Viper".to_owned())];
        names.ship_long_names = vec![(ShipId(129), "Federation Viper".to_owned())];
        names.outfit_names = vec![(OutfitId(128), "Blaster".to_owned())];
        names.outfit_lc_names =
            vec![(OutfitId(128), ("blaster".to_owned(), "blasters".to_owned()))];
        names.good_names = vec![(Good::Commodity(0), "Food".to_owned())];
        names.junk_lc_names = vec![(JunkId(150), "bio-weapons".to_owned())];
        names.cargo_names = vec!["Food".to_owned(), "*Passengers".to_owned()];
        names.player_ship_name = Some("Kestrel".to_owned());
        names.long_date = "June 23rd, 1177 NC".to_owned();
        assert_eq!(fake.ship_name(ShipId(129)), Some("Viper"));
        assert_eq!(fake.ship_name(ShipId(130)), None, "not listed");
        assert_eq!(fake.ship_long_name(ShipId(129)), Some("Federation Viper"));
        assert_eq!(fake.outfit_name(OutfitId(128)), Some("Blaster"));
        assert_eq!(
            fake.outfit_names(OutfitId(128)),
            Some(("blaster", "blasters"))
        );
        assert_eq!(fake.good_name(Good::Commodity(0)), Some("Food"));
        assert_eq!(fake.good_name(Good::Commodity(1)), None);
        assert_eq!(fake.junk_lc_name(JunkId(150)), Some("bio-weapons"));
        assert_eq!(fake.cargo_name(1).as_deref(), Some("*Passengers"));
        assert_eq!(fake.cargo_name(2), None);
        assert_eq!(fake.player_ship_name(), Some("Kestrel"));
        assert_eq!(fake.long_date(), "June 23rd, 1177 NC");
    }

    #[test]
    fn the_fake_answers_nothing_of_the_port_until_scripted_then_as_scripted() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.market(), None);
        assert_eq!(fake.mission_info(), MissionInfo::default());
        assert_eq!(fake.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
        assert_eq!(fake.offer_gap(), OFFER_GAP_TICKS);
        assert!(!fake.refusable_offer_dialog(MissionId(130)));
        assert_eq!(fake.offer_text(MissionId(130), "Run <PAY>"), None);
        let port = fake.port_mut();
        port.offer_texts = vec![(MissionId(130), "Run 1,000".to_owned())];
        port.market = Some(Market::default());
        port.recharge_enabled = true;
        port.news = Some(News::default());
        port.news_picture = Some(9000);
        port.seen_intro = true;
        port.refusable = vec![MissionId(130)];
        port.outfitter = Some(Outfitter::default());
        port.offer_gap = Some(45);
        let info = MissionInfo {
            rows: Vec::new(),
            today: "Today.".to_owned(),
        };
        port.mission_info = Some(info.clone());
        assert_eq!(fake.market(), Some(Market::default()));
        assert!(fake.recharge_enabled());
        assert_eq!(fake.news(), Some(&News::default()));
        assert_eq!(fake.news_picture(), Some(9000));
        assert!(fake.seen_intro());
        assert!(fake.refusable_offer_dialog(MissionId(130)));
        assert!(!fake.refusable_offer_dialog(MissionId(131)));
        assert_eq!(
            fake.offer_text(MissionId(130), "Run <PAY>").as_deref(),
            Some("Run 1,000")
        );
        assert_eq!(fake.offer_text(MissionId(131), "Run"), None, "not offered");
        assert_eq!(fake.outfitter(), Some(Outfitter::default()));
        assert_eq!(fake.mission_info(), info);
        assert_eq!(fake.offer_gap(), 45);
        assert_eq!(fake.shipyard(), None);
        assert_eq!(fake.escorts_for_hire(), None);
    }

    #[test]
    fn a_fake_boarding_or_with_an_escort_flying_meets_the_contract() {
        let mut flying = FakeFlight::new();
        let mut frame = stellar(128);
        frame.hud.escorts_flying = true;
        frame.hud.escort_rows[0].present = true;
        flying.set_frame(frame);
        assert_eq!(contract(&mut flying), Ok(()), "a row present");
        let mut aboard = FakeFlight::new();
        aboard.set_frame(stellar(128));
        aboard.comm_mut().boarding = Some(boarded());
        assert_eq!(
            contract(&mut aboard),
            Ok(()),
            "a boarding under way may answer a take as it will"
        );
    }

    /// What is on board NPC 0, a ship 129.
    fn boarded() -> PlunderView {
        PlunderView {
            npc: nova_sim::NpcId(0),
            ship: ShipId(129),
            credits: 100,
            cargo: None,
            ammo: None,
            fuel: 0,
            odds: 0,
        }
    }

    #[test]
    fn the_fake_answers_nothing_of_the_comm_until_scripted_then_as_scripted() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.hailing(), None);
        assert_eq!(fake.boarding(), None);
        fake.comm_mut().boarding = Some(boarded());
        assert_eq!(fake.boarding(), Some(boarded()));
        let hail = nova_sim::HailView {
            npc: nova_sim::NpcId(0),
            ship: ShipId(129),
            first: None,
            reply: "Hello.".to_owned(),
            comm_name: String::new(),
            govt_name: None,
            hostile: false,
            options: Vec::new(),
            asking: None,
            pay_me: true,
            escort: None,
            portrait: None,
        };
        fake.comm_mut().hailing = Some(hail.clone());
        assert_eq!(fake.hailing(), Some(hail));
    }

    #[test]
    fn a_weapon_select_answered_or_changing_the_frame_breaks_the_contract() {
        let answered = clause(Fault::AnswersTheSelect);
        assert!(
            answered.starts_with("a weapon select waits for the step"),
            "{answered}"
        );
        assert!(answered.contains("answered"), "{answered}");
        let changed = clause(Fault::SelectChangesTheFrame);
        assert!(
            changed.starts_with("a weapon select waits for the step"),
            "{changed}"
        );
        assert!(changed.contains("the frame was"), "{changed}");
    }

    #[test]
    fn travel_facts_that_disagree_break_the_contract() {
        for (fault, what) in [
            (
                Fault::WarpEndsUnwarped,
                "the warp's end is due with no warp",
            ),
            (Fault::WarpsWithoutAJump, "a warp with no jump under way"),
            (Fault::LeavesInFlight, "a leave due in flight"),
            (
                Fault::ExpiresFlying,
                "the hold expired with the fate Flying",
            ),
        ] {
            let clause = clause(fault);
            assert_eq!(
                clause,
                format!("the travel facts agree: {what}"),
                "{fault:?}"
            );
        }
    }

    #[test]
    fn an_arrival_before_the_warps_end_breaks_the_contract() {
        let answered = clause(Fault::ArrivesEarly);
        assert!(
            answered.starts_with("no arrival before the warp's end"),
            "{answered}"
        );
        assert!(answered.contains("answered"), "{answered}");
        let changed = clause(Fault::ArrivalChangesTheFrame);
        assert!(
            changed.starts_with("no arrival before the warp's end"),
            "{changed}"
        );
        assert!(changed.contains("the frame was"), "{changed}");
    }

    #[test]
    fn a_fade_or_an_end_before_the_hold_breaks_the_contract() {
        for (fault, command) in [
            (Fault::FadesEarly, "BeginFade"),
            (Fault::EndsEarly, "EndPlay"),
        ] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("the fade waits for the hold"),
                "{fault:?}: {clause}"
            );
            assert!(clause.contains(command), "{fault:?}: {clause}");
        }
    }

    #[test]
    fn a_jump_answered_otherwise_or_against_its_kind_breaks_the_contract() {
        for (fault, says) in [
            (Fault::AnswersTheJump, "it answered"),
            (
                Fault::RefusalJumps,
                "took the jump under way from false to true",
            ),
            (Fault::JumpsNowhere, "with no jump under way"),
        ] {
            let clause = clause(fault);
            assert!(
                clause.starts_with("a jump's answer is its kind"),
                "{fault:?}: {clause}"
            );
            assert!(clause.contains(says), "{fault:?}: {clause}");
        }
    }

    #[test]
    fn a_fake_through_a_jump_a_hold_and_a_leave_meets_the_contract() {
        let warping = FlightFrame {
            jump_under_way: true,
            warping: true,
            warp_end_due: true,
            ..stellar(128)
        };
        let leaving = FlightFrame {
            landed: Some(StellarId(128)),
            leave_due: true,
            ..stellar(128)
        };
        let expired = |fate| FlightFrame {
            fate,
            hold_expired: true,
            ..stellar(128)
        };
        for frame in [
            warping,
            leaving,
            expired(PlayerFate::Holding { ticks_left: 0 }),
            expired(PlayerFate::Fading),
            expired(PlayerFate::GameOver),
        ] {
            let mut fake = FakeFlight::new();
            fake.set_frame(frame.clone());
            assert_eq!(contract(&mut fake), Ok(()), "{frame:?}");
        }
    }

    #[test]
    fn the_fake_shows_the_frame_an_answer_leaves() {
        let mut fake = FakeFlight::new();
        fake.queue_answer_showing(Answer::Jumped(SystemId(131)), stellar(129));
        fake.queue_answer(Answer::Done);
        assert_eq!(
            fake.apply(FlightCommand::Jump),
            Answer::Jumped(SystemId(131))
        );
        let mut frame = FlightFrame::default();
        fake.frame(&mut frame);
        assert_eq!(frame, stellar(129), "shown from the answer on");
        assert_eq!(fake.apply(FlightCommand::Land), Answer::Done);
        fake.frame(&mut frame);
        assert_eq!(frame, stellar(129), "kept by an answer with no frame");
    }

    #[test]
    fn the_fake_answers_the_message_line_done_without_its_queue() {
        let mut fake = FakeFlight::new();
        fake.queue_answer(Answer::Jumped(SystemId(131)));
        let line = [
            FlightCommand::CommShown(240),
            FlightCommand::CommAcknowledged,
            FlightCommand::CommTicked,
        ];
        for command in line.clone() {
            assert_eq!(fake.apply(command.clone()), Answer::Done, "{command:?}");
        }
        assert_eq!(
            fake.apply(FlightCommand::Jump),
            Answer::Jumped(SystemId(131))
        );
        assert_eq!(fake.applied(), [&line[..], &[FlightCommand::Jump]].concat());
    }

    #[test]
    fn the_fake_opens_listing_the_weapons_it_is_given_and_why_it_failed() {
        let mut fake = FakeFlight::new();
        assert_eq!(fake.opening(), FlightOpening::default(), "none, started");
        fake.set_weapons(vec![WeaponId(128)]);
        assert_eq!(fake.opening().weapons, [WeaponId(128)]);
        fake.fail("no chär");
        assert_eq!(fake.opening().failure.as_deref(), Some("no chär"));
    }

    #[test]
    fn a_fake_showing_a_target_its_brackets_and_iffs_meets_the_contract() {
        let mut fake = FakeFlight::new();
        let mut frame = stellar(128);
        frame.ships.push(record::npc(7, 129, Vec2::ZERO, 0.0));
        frame.target = Some(record::target(7));
        frame.hud.target_brackets = Some(nova_sim::brackets::TargetBrackets {
            npc: nova_sim::NpcId(7),
            expand: 0.0,
            standing: nova_sim::brackets::Standing::Neutral,
        });
        frame.hud.interference = 100;
        frame.stellars = vec![record::stellar(StellarId(128))];
        fake.set_frame(frame);
        assert_eq!(contract(&mut fake), Ok(()));
    }

    #[test]
    fn the_fake_loads_descriptions_as_queued_then_as_their_raw_text_recording_each() {
        let mut fake = FakeFlight::new();
        fake.queue_description("scripted");
        let movie = DescMovie {
            file: "Intro.mov".to_owned(),
            after: true,
            double_size: false,
            cinematic: false,
        };
        let load = |raw: Option<&str>, movie: Option<DescMovie>| FlightCommand::LoadDescription {
            raw: raw.map(str::to_owned),
            movie,
        };
        let plain = "{b5 \"set\" \"clear\"}";
        let loads = [
            load(Some("raw"), Some(movie)),
            load(Some(plain), None),
            load(None, None),
        ];
        let answers: Vec<Answer> = loads.iter().map(|ask| fake.apply(ask.clone())).collect();
        assert_eq!(
            answers,
            [
                Answer::Description("scripted".to_owned()),
                Answer::Description(plain.to_owned()),
                Answer::Description(String::new()),
            ],
            "queued, then the raw text unchanged, and a missing one empty"
        );
        assert_eq!(fake.asks(), loads, "each recorded");
        assert_eq!(fake.applied(), [], "apart from the commands");
        fake.fail("no chär");
        assert_eq!(
            fake.apply(load(Some(plain), None)),
            Answer::Description(plain.to_owned()),
            "a failed fake still loads one"
        );
    }

    /// The hover at `at` with `drawn`, as the command carries it.
    fn hover_at(at: Vec2, drawn: &DrawnBoxes) -> FlightCommand {
        FlightCommand::Hover {
            at,
            drawn: drawn.clone(),
        }
    }

    #[test]
    fn the_fake_answers_the_hover_as_told_and_records_each_ask() {
        let mut fake = FakeFlight::new();
        let boxes = DrawnBoxes {
            player: Some(nova_sim::DrawnBox {
                centre: Vec2::ZERO,
                width: 10,
                height: 10,
            }),
            npcs: Vec::new(),
        };
        let first = hover_at(Vec2::new(1.0, 2.0), &boxes);
        assert_eq!(fake.apply(first.clone()), Answer::Hovered(None));
        fake.set_hover(Some(Hover::Stellar(StellarId(128))));
        let second = hover_at(Vec2::ZERO, &DrawnBoxes::default());
        assert_eq!(
            fake.apply(second.clone()),
            Answer::Hovered(Some(Hover::Stellar(StellarId(128))))
        );
        assert_eq!(fake.asks(), [first, second]);
    }

    #[test]
    fn the_fake_brings_the_coming_traffic_it_is_given() {
        let mut fake = FakeFlight::new();
        assert_eq!(
            fake.apply(FlightCommand::ComingTraffic),
            Answer::ComingTraffic(Vec::new())
        );
        fake.set_coming_traffic(vec![ShipId(129)]);
        assert_eq!(
            fake.apply(FlightCommand::ComingTraffic),
            Answer::ComingTraffic(vec![ShipId(129)])
        );
    }

    #[test]
    fn the_fakes_asks_take_no_queued_answer_and_are_not_applied() {
        let mut fake = FakeFlight::new();
        fake.queue_answer(Answer::Stellar(StellarId(130)));
        fake.apply(hover_at(Vec2::ZERO, &DrawnBoxes::default()));
        fake.apply(FlightCommand::ComingTraffic);
        assert_eq!(fake.applied(), [], "asks are kept apart");
        assert_eq!(fake.asks().len(), 2);
        assert_eq!(
            fake.apply(FlightCommand::SelectStellar(StellarPick::Nearest)),
            Answer::Stellar(StellarId(130)),
            "the answer queued is the next command's"
        );
        fake.fail("no chär");
        assert_eq!(
            fake.apply(FlightCommand::ComingTraffic),
            Answer::Refused,
            "a failed fake refuses them"
        );
        assert_eq!(fake.asks().len(), 3, "still recorded");
    }

    #[test]
    #[allow(clippy::too_many_lines)] // one assertion for each of the frame's fields
    fn a_fill_over_an_unlike_frame_changes_every_field() {
        let busy = FlightFrame {
            ships: Vec::new(),
            drawn: Vec::new(),
            shots: vec![record::shot()],
            stellars: vec![record::stellar(StellarId(130))],
            effective_murk: u8::MAX,
            system_background: Some(0x10),
            double_time: true,
            traffic_ships: vec![ShipId(130)],
            system: SystemId(i16::MAX),
            course_shown: vec![SystemId(131)],
            course_plotted: true,
            landed: Some(StellarId(128)),
            jump_under_way: true,
            warping: true,
            warp_end_due: true,
            fate: PlayerFate::Fading,
            hold_expired: true,
            leave_due: true,
            jump_readiness: JumpReadiness::Underway,
            texts_queued: u64::MAX,
            gates: Gates::EVERY.refusing(PlayerCommand::Map),
            death_descs: vec![130],
            player_ship_name: Some("Kestrel".to_owned()),
            long_date: "June 23rd, 1177 NC".to_owned(),
            explored: vec![SystemId(130)],
            map_key: MapKey {
                system: SystemId(i16::MAX),
                nav: NavMode::Stellar(None),
                ..MapKey::default()
            },
            ..stellar(128)
        };
        for frame in [FlightFrame::default(), stellar(128), busy] {
            let other = unlike(&frame);
            let FlightFrame {
                nav_mode,
                system,
                course_shown,
                course_plotted,
                landed,
                jump_under_way,
                warping,
                warp_end_due,
                fate,
                hold_expired,
                leave_due,
                jump_readiness,
                texts_queued,
                gates,
                map_key,
                ships,
                drawn,
                shots,
                beams,
                cargo_boxes,
                asteroids,
                stellars,
                effective_murk,
                system_murk,
                system_background,
                double_time,
                hyper_gamma,
                traffic_ships,
                hud,
                target,
                death_descs,
                player_ship_name,
                long_date,
                explored,
            } = &frame;
            assert_unlike_hud(&other.hud, hud);
            assert_ne!(other.target, *target);
            assert_ne!(other.ships, *ships);
            assert_ne!(other.drawn, *drawn);
            assert_ne!(other.shots, *shots);
            assert_ne!(other.beams, *beams);
            assert_ne!(other.cargo_boxes, *cargo_boxes);
            assert_ne!(other.asteroids, *asteroids);
            assert_ne!(other.stellars, *stellars);
            assert_ne!(other.effective_murk, *effective_murk);
            assert_ne!(other.system_murk, *system_murk);
            assert_ne!(other.system_background, *system_background);
            assert_ne!(other.double_time, *double_time);
            assert_ne!(other.hyper_gamma.to_bits(), hyper_gamma.to_bits());
            assert_ne!(other.traffic_ships, *traffic_ships);
            assert_ne!(other.nav_mode, *nav_mode);
            assert_ne!(other.system, *system);
            assert_ne!(other.course_shown, *course_shown);
            assert_ne!(other.course_plotted, *course_plotted);
            assert_ne!(other.landed, *landed);
            assert_ne!(other.jump_under_way, *jump_under_way);
            assert_ne!(other.warping, *warping);
            assert_ne!(other.warp_end_due, *warp_end_due);
            assert_ne!(other.fate, *fate);
            assert_ne!(other.hold_expired, *hold_expired);
            assert_ne!(other.leave_due, *leave_due);
            assert_ne!(other.jump_readiness, *jump_readiness);
            assert_ne!(other.texts_queued, *texts_queued);
            assert_ne!(other.gates, *gates);
            assert_ne!(other.death_descs, *death_descs);
            assert_ne!(other.player_ship_name, *player_ship_name);
            assert_ne!(other.long_date, *long_date);
            assert_ne!(other.explored, *explored);
            assert_ne!(other.map_key, *map_key);
            assert_ne!(other.map_key.system, map_key.system);
            assert_ne!(other.map_key.nav, map_key.nav);
        }
    }

    /// Asserts `other` is unlike `hud` in every field.
    fn assert_unlike_hud(other: &FrameHud, hud: &FrameHud) {
        let FrameHud {
            reserves,
            cargo,
            secondary,
            secondary_line_count,
            radar_gear,
            interference,
            interface_government,
            nav_brackets,
            target_brackets,
            red_alert,
            escort_rows,
            escorts_flying,
        } = hud;
        assert_ne!(other.reserves, *reserves);
        assert_ne!(other.cargo, *cargo);
        assert_ne!(other.secondary, *secondary);
        assert_ne!(other.secondary_line_count, *secondary_line_count);
        assert_ne!(other.radar_gear, *radar_gear);
        assert_ne!(other.interference, *interference);
        assert_ne!(other.interface_government, *interface_government);
        assert_ne!(other.nav_brackets, *nav_brackets);
        assert_ne!(other.target_brackets, *target_brackets);
        assert_ne!(other.red_alert, *red_alert);
        assert_ne!(other.escort_rows, *escort_rows);
        assert_ne!(other.escorts_flying, *escorts_flying);
    }

    #[test]
    fn an_unlike_hud_changes_every_field_full_or_empty() {
        let full = unlike_hud(&FrameHud::default());
        for hud in [FrameHud::default(), full.clone(), unlike_hud(&full)] {
            assert_unlike_hud(&unlike_hud(&hud), &hud);
        }
    }

    #[test]
    fn an_unlike_maps_key_moves_off_the_system_unlike_takes() {
        // The key already on the system the frame's unlike moves to steps
        // one below it.
        for (system, key_system, moved) in [(0, i16::MAX, i16::MAX - 1), (i16::MAX, 0, -1)] {
            let frame = FlightFrame {
                system: SystemId(system),
                map_key: MapKey {
                    system: SystemId(key_system),
                    ..MapKey::default()
                },
                ..FlightFrame::default()
            };
            let other = unlike(&frame);
            assert_eq!(other.map_key.system, SystemId(moved));
            assert_ne!(other.map_key.system, frame.map_key.system);
        }
    }
}
