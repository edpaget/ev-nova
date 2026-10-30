//! The flight view's tests. Its component tests run on [`FakeFlight`],
//! each module over scripted frames ([`frames`]), answers and command
//! assertions. [`FakeCatalog`], its data builders and the fakes the
//! adapter's checks share come from `flight::fake_catalog`. This module
//! holds the rest of what the component tests share: the fake-based
//! helpers ([`Faked`], [`faked`], [`sent`], the `faked_map_*` helpers,
//! ...) and the helpers generic over any [`FlightSim`]. No test here
//! starts a real `nova_sim::Session` or imports a real-session type: the
//! adapter's checks are `sim_adapter`'s, the edge's are
//! `build::edge_checks`, and the integration set is
//! `tests/it/flight_seam.rs` and the stock tests
//! (rdm:phase/view-sim-seam/phase-15-integration-test-set).

use std::cell::RefCell;
use std::num::NonZeroU16;

use crate::system::camera::{VIEW_CENTER, VIEW_SIZE};
use nova_sim::flight_frame::GateFade;
use nova_sim::landing::StellarFlags;
use nova_sim::{
    LandingSite, OutfitId, Reserves, ShipId, ShipStats, SimSound, SystemId, TICK, Vec2,
};

use super::*;
use crate::devtools::{PilotDesk, PilotEdit};
use crate::draw::crossed_box;
use crate::draw::fill_command;
use crate::draw::lights_tint;
use crate::flight::cadence::{NAV_PERIOD_TICKS, TARGET_PERIOD_TICKS};
use crate::flight::catalog::ShipSprites;
use crate::flight::catalog::{
    Blink, BoomLook, GovtId, LayerSheet, StarSheet, StatusBarLayout, WeaponLook,
};
use crate::flight::escorts::{GROUP_KEYS, MENU_KEY, NO_ESCORTS};
use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
use crate::flight::jump::JumpPhase;
use crate::flight::radar::RadarStellar;
use crate::flight::radar::{self, IRect, RadarColor, RadarMark};
use crate::flight::sim_port::FlightSim;
use crate::flight::sim_port::fixture::{FakeFlight, record};
use crate::flight::sprite::rotation_frame;
use crate::flight::target::Tag;
use crate::flight::weapons;
use crate::galaxy::MapMode;
use crate::sound::Sound;
use crate::spaceport::PortDialog;
use crate::system::scene::PLACEHOLDER;
use crate::system::{scene, starfield};
use crate::ui::mission_info::MISSION_INFO_KEY;
use crate::ui::player_info::PLAYER_INFO_KEY;
use crate::{DrawList, ImageKey, Input, MouseButton, Screen, ScreenAction};
use nova_sim::ShipIff;
use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightFrame, MapClick, MapOpen, MapScreen, MarksFrom, Names,
    ScreenAnswer, ScreenAsk,
};
use nova_sim::galaxy_map::GalaxyShown;
use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
use nova_sim::{
    AbortError, AcceptError, Assigned, Assignment, CombatCatalog, ComputerRefusal, Controls,
    FighterNote, MissionId, MissionShown, NotOffered, PilotCatalog, PlayerCommand, Rulebook,
    StellarIff, StellarPick, Take, Taken, Turn, VenueOffer,
};

use crate::system::catalog::{StellarContents, StellarId, StellarSheet};
use crate::{Blend, DrawCommand, Font};
use nova_sim::{BoardRefusal, DockKind, MissionBoardLine};
use nova_sim::{GLOW_CRUISE, glow_level};
use nova_sim::{RuleKey, RuleSource};

mod anchor;
mod boarding;
mod cloak;
mod combat;
mod comm_line;
mod controls;
mod dialog_map;
mod escorts;
mod faked_board;
mod faked_combat;
mod faked_desk;
mod faked_docked;
mod faked_escorts;
mod faked_flight;
mod faked_gamble;
mod faked_hail;
mod faked_hud;
mod faked_keys;
mod faked_landing;
mod faked_missions;
mod faked_open;
mod faked_player;
mod faked_screen;
mod faked_shops;
mod faked_travel;
mod faked_wording;
mod faked_world;
mod flying;
mod frames;
mod hyperspace;
mod kept_frame;
mod landing;
mod layers;
mod map_borders;
mod map_find;
mod maps;
mod missions;
mod nav;
mod outputs;
mod painted_ships;
mod port_screen_map;
mod ship_drawing;
mod shots;
mod stars;
mod stellar_looks;
mod stellars;
mod stepping;
mod widescreen;
mod world_layers;

use crate::flight::fake_catalog::*;
// The fixture's `look`, not the view's.
use crate::flight::fake_catalog::look;
use frames::*;

/// Where flight draws the player's ship in Nova's own 1024 x 768 by the
/// original's reading of [`RuleKey::FlightAnchor`]: the play area's
/// centre, `_screenCenter`, (trunc((1024 − 194) / 2), 384).
const PLAYER_AT: Point = Point::new(415.0, 384.0);

/// The planet comm dialog open on stellar `id`, "Earth", greeting.
fn planet_view(id: i16) -> nova_sim::planet_comm::PlanetCommView {
    use nova_sim::planet_comm::{PlanetCommButton, PlanetCommView};
    PlanetCommView {
        stellar: nova_sim::StellarId(id),
        name: "Earth".to_owned(),
        type_label: "[Earth/Luna System]".to_owned(),
        reply: "Channel open to Earth.".to_owned(),
        second: PlanetCommButton::Greetings,
        third: Some(PlanetCommButton::DemandTribute),
        status: None,
        asking: None,
    }
}

fn held(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: true,
    }
}

/// The second half of a tick: `TICK / 2` rounds down, so this is a
/// nanosecond longer.
const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

fn drawn(view: &FlightView<FakeCatalog, impl FlightSim>) -> DrawList {
    let mut list = DrawList::new();
    view.draw(&mut list);
    list
}

fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: Font::Geneva,
        origin,
        size,
        wrap_width: None,
        color,
    }
}

/// The stars' sheet, `rlëD` 700 of 16 frames of 5 × 5, as stock's.
fn star_sheet() -> StarSheet {
    StarSheet {
        image_id: 700,
        frames: NonZeroU16::new(16).expect("non-zero"),
        size: (5, 5),
    }
}

/// [`catalog`] with the stars' sheet, so flight draws its stars.
fn starry() -> FakeCatalog {
    FakeCatalog {
        star_sheet: Ok(star_sheet()),
        ..catalog()
    }
}

/// Whether `command` is one of the stars [`star_sheet`] draws.
fn is_star(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::Sprite {
            image,
            blend: Blend::Or,
            ..
        } if image.id == 700
    )
}

/// The sprites drawn, the stars left out.
fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
    list.iter()
        .filter(|command| !is_star(command))
        .filter_map(|command| match unclipped(command) {
            DrawCommand::Sprite { image, center, .. } => Some((*image, *center)),
            _ => None,
        })
        .collect()
}

fn lines(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|c| matches!(c, DrawCommand::Line { .. }))
        .cloned()
        .collect()
}

/// The stock radar, with the bar at (830, 0): centred on (926, 96).
const STOCK_RADAR: IRect = IRect::new(838, 8, 1014, 184);

/// What the radar shows this frame, as marks on the stock radar.
fn radar_marks(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<RadarMark> {
    radar::marks(
        STOCK_RADAR,
        &view.radar_shown(&view.frame_now()),
        view.hud_cadence.blink_on(),
    )
}

/// The ships' blips on the radar this frame: its pixels but the
/// player's, the last.
fn blips(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<RadarMark> {
    let mut marks: Vec<RadarMark> = radar_marks(view)
        .into_iter()
        .filter(|mark| matches!(mark, RadarMark::Pixel { .. }))
        .collect();
    marks.pop();
    marks
}

const PLAYER_PIXEL: RadarMark = RadarMark::Pixel {
    x: 926,
    y: 96,
    color: RadarColor::Bright,
};

fn texts(list: &DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The port's title, and its help line, on.
const TITLED: FlightOverlays = FlightOverlays {
    title: true,
    help: true,
    cheats: false,
};

/// [`layered`], with the lights blinking by `blink`.
fn blinking(blink: Blink) -> FakeCatalog {
    FakeCatalog {
        sheet: Ok(ShipSheet {
            glow: Some(layer(2100, 36)),
            lights: Some(layer(2200, 12)),
            ..sheet()
        }),
        hulls: vec![lit_hull(hull_of(128, &[]), blink)],
        ..catalog()
    }
}

/// `hull` with a lights sprite blinking `blink`, as its `shän` names one,
/// which the session's display steps (`nova_sim::combat::ship_lights`).
fn lit_hull(hull: nova_sim::HullRecord, blink: Blink) -> nova_sim::HullRecord {
    let display = hull.display.unwrap_or(nova_sim::catalog::ShipDisplay {
        base_sets: 1,
        ..nova_sim::catalog::ShipDisplay::default()
    });
    nova_sim::HullRecord {
        display: Some(nova_sim::catalog::ShipDisplay {
            blink,
            lights_image: true,
            ..display
        }),
        ..hull
    }
}

/// `ïntf` 129: [`layout`] in another shield colour, over `PICT` 705.
fn bar_129() -> StatusBarLayout {
    StatusBarLayout {
        shield_color: Color::rgba(0, 255, 255, 255),
        status_bkgnd: 705,
        ..layout()
    }
}

/// `catalog` where ship type `ship` (given a record if it has none)
/// belongs to gövt 150, whose `Interface` is `interface`, and with
/// `ïntf` 129 ([`bar_129`]).
fn governed(mut catalog: FakeCatalog, ship: i16, interface: i16) -> FakeCatalog {
    if !catalog.ships.iter().any(|record| record.id == ShipId(ship)) {
        let mut record = shipbuying().ships[0].clone();
        record.id = ShipId(ship);
        catalog.ships.push(record);
    }
    for record in &mut catalog.ships {
        if record.id == ShipId(ship) {
            record.inherent_govt = Some(GovtId(150));
        }
    }
    catalog.interfaces = vec![(150, interface)];
    catalog.other_bars = vec![(129, bar_129())];
    catalog
}
const LAND: Key = Key::Char('l');

/// Where the message's first line is drawn: the message box's
/// top-left.
fn message_at() -> Point {
    hud::message_box(VIEW_SIZE).min
}

/// The message `text` as a flight without metrics draws it.
fn message_drawn(text: &str) -> DrawCommand {
    DrawCommand::Text {
        text: text.to_owned(),
        font: hud::MESSAGE_FONT,
        origin: at(25.0, 724.0),
        size: 12.0,
        wrap_width: Some(755.0),
        color: Color::WHITE,
    }
}

const JUMP: Key = Key::Char('j');

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// The fade quad's place in the list and its colour, if drawn.
fn fade(list: &DrawList) -> Option<(usize, Color)> {
    list.iter().enumerate().find_map(|(at, c)| match *c {
        DrawCommand::Line {
            from, width, color, ..
        } if from == Point::new(0.0, 384.0) && width == 768.0 => Some((at, color)),
        _ => None,
    })
}

/// Where the HUD starts: its status bar picture.
fn hud_at(list: &DrawList) -> usize {
    list.iter()
        .position(
            |c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)),
        )
        .expect("the HUD")
}

use nova_sim::{Chance, Direction, Good, Lot, Order};

const BUY_FOOD: Order = Order {
    row: 0,
    good: Good::Commodity(0),
    direction: Direction::Buy,
    lot: Lot::Click,
};

/// The texts drawn in the nav area of the port's status bar
/// ([`FlightView::draw_status_bar`]), as [`nav`] reads flight's.
fn port_nav(view: &FlightView<FakeCatalog, impl FlightSim>) -> Vec<String> {
    let area = layout().nav.offset(at(830.0, 0.0));
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    bar.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if area.contains(*origin) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The nav area naming `name` in mode 2.
fn naming(name: &str) -> Vec<String> {
    vec![hud::NAV_STELLAR.to_owned(), name.to_owned()]
}

/// A view over a scripted flight ([`FakeFlight`]), for the component
/// tests of the code on the port.
type Faked = FlightView<FakeCatalog, FakeFlight>;

/// A view over a flight scripted to show `frame`, read from [`catalog`].
fn faked(frame: FlightFrame) -> Faked {
    faked_in(catalog(), frame)
}

/// A view over a flight scripted to show `frame`, read from `catalog`,
/// the flight naming nothing.
fn faked_in(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
    let mut fake = FakeFlight::new();
    fake.set_frame(frame);
    scripted_over(catalog, fake, &Rulebook::default())
}

/// The view of `fake` over `catalog` by `rules`
/// ([`FlightView::scripted`]), opening on the names `fake`'s opening
/// names ([`FakeFlight::set_names`]).
fn scripted_over(catalog: FakeCatalog, fake: FakeFlight, rules: &Rulebook) -> Faked {
    let names = fake.opening().names;
    FlightView::scripted(catalog, fake, names, rules)
}

/// Changes the names `view` opened on with `change`, as an opening naming
/// them would have: for a test whose view a shared helper built before
/// the test says what the flight names. The view reads the opening once,
/// so the table it kept is the one to change.
fn renamed(view: &mut Faked, change: impl FnOnce(&mut Names)) {
    let mut names = (*view.names).clone();
    change(&mut names);
    view.names = Rc::new(names);
}

/// The names of the stellars of `stellars`, each with its name, and
/// nothing else.
fn stellar_names(stellars: &[(i16, &str)]) -> Names {
    let mut names = Names::default();
    for &(id, name) in stellars {
        names.stellars.insert(StellarId(id), name.to_owned());
    }
    names
}

/// The commands `view` sent its flight, in order.
fn sent(view: &Faked) -> &[FlightCommand] {
    view.sim.applied()
}

/// Gives `view`'s course map `input` as flight's map input does, and
/// says whether it closed the map: the close is the session's, so a
/// scripted flight only notes it.
fn faked_map_input(view: &mut Faked, input: &Input) -> bool {
    let closed = std::cell::Cell::new(false);
    view.map_input_then(input, |_| closed.set(true));
    closed.get()
}

/// Presses and releases `k` on `view`'s open course map, as flight's map
/// input gives them, and says whether the press closed it.
fn faked_map_tap(view: &mut Faked, k: Key) -> bool {
    let closed = faked_map_input(view, &key(k, true));
    faked_map_input(view, &key(k, false));
    closed
}

/// Clicks the left button at `at` on `view`'s open course map, and says
/// whether the click closed it.
fn faked_map_click(view: &mut Faked, at: Point) -> bool {
    let button = |pressed| Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed,
        at,
    };
    let by_press = faked_map_input(view, &button(true));
    let by_release = faked_map_input(view, &button(false));
    by_press || by_release
}

/// Shift-clicks at `at` on `view`'s open course map.
fn faked_map_shift_click(view: &mut Faked, at: Point) {
    faked_map_input(view, &key(Key::Shift, true));
    faked_map_click(view, at);
    faked_map_input(view, &key(Key::Shift, false));
}

/// Where system `id` is on `view`'s course map, brought into item 3
/// first, as [`on_map`] finds it on a session's.
fn faked_on_map(view: &mut Faked, id: i16) -> Point {
    view.map.centre_on_system(SystemId(id));
    let system = view.map.model().system(SystemId(id)).expect("on the map");
    view.map.transform().world_to_screen(system.position())
}

/// System `id` as a flight's drawing frames it in `code`, with no
/// mission arrows.
fn drawn_system(
    id: i16,
    code: nova_sim::iff::SystemColorCode,
) -> nova_sim::galaxy_map::DrawnSystem {
    nova_sim::galaxy_map::DrawnSystem {
        id: SystemId(id),
        code,
        red_arrow: false,
        green_arrow: false,
    }
}

/// The course map's drawing in Sol (130) with the nav off, as a session
/// over [`catalog`] draws it: Sol, uninhabited and the one system named,
/// and its link to Alpha Centauri (131), unexplored.
fn sol_drawing() -> nova_sim::galaxy_map::MapDrawing {
    use nova_sim::iff::SystemColorCode;
    let (sol, alpha) = (SystemId(130), SystemId(131));
    nova_sim::galaxy_map::MapDrawing {
        hypergate: false,
        course: Vec::new(),
        current_links: vec![(sol, alpha)],
        links: vec![nova_sim::galaxy_map::MapLink {
            from: sol,
            to: alpha,
            selected: false,
        }],
        mode3_link: None,
        systems: vec![
            drawn_system(130, SystemColorCode::Uninhabited),
            drawn_system(131, SystemColorCode::Unexplored),
        ],
        current: sol,
        gate_arrows: Vec::new(),
        named: vec![sol],
    }
}

/// What a scripted flight in `frame` answers the course map's opening
/// ([`ScreenAsk::MapOpening`]): `frame`'s nav noted, no marks, no border
/// sources, `preselection` selected and no mission systems.
fn map_opening(frame: &FlightFrame, preselection: Option<i16>) -> ScreenAnswer {
    opening_with(frame.nav_mode, preselection, nova_sim::MapMarks::NONE)
}

/// What a scripted flight answers the course map's opening with the nav
/// `nav` as it opens: that nav noted, `marks`, no border sources,
/// `preselection` selected and no mission systems.
fn opening_with(
    nav: NavMode,
    preselection: Option<i16>,
    marks: nova_sim::MapMarks,
) -> ScreenAnswer {
    ScreenAnswer::MapOpening(MapOpen {
        nav: nova_sim::SavedNav::of(nav),
        marks,
        borders: nova_sim::map_borders::BorderSources::default(),
        preselection: preselection.map(SystemId),
        missions: Vec::new(),
    })
}

/// A panel that says nothing: an empty strip and column, no words.
fn blank_panel() -> nova_sim::map_panel::MapPanel {
    use nova_sim::map_panel::{Column, ColumnBody, MapPanel, PanelWords, Ports, Strip};
    MapPanel {
        strip: Strip::Info {
            ports: Ports::None,
            hazards: String::new(),
            date: String::new(),
        },
        column: Column {
            title: String::new(),
            body: ColumnBody::Unknown,
        },
        words: PanelWords::default(),
    }
}

/// What a scripted flight answers a course map's redraw
/// ([`ScreenAsk::Map`]): `drawing`, marking its gate arrows known as a
/// session does (none off the hypergate map), no border sources and
/// `panel`.
fn redrawn_with(
    drawing: nova_sim::galaxy_map::MapDrawing,
    panel: nova_sim::map_panel::MapPanel,
) -> ScreenAnswer {
    ScreenAnswer::Map(Box::new(MapScreen {
        marks_known: drawing.gate_arrows.clone(),
        drawing,
        borders: nova_sim::map_borders::BorderSources::default(),
        panel,
    }))
}

/// [`redrawn_with`] `drawing` and a [`blank_panel`].
fn redrawn(drawing: nova_sim::galaxy_map::MapDrawing) -> ScreenAnswer {
    redrawn_with(drawing, blank_panel())
}

/// Queues on `view`'s flight the answers to the course map's opening,
/// [`map_opening`] over its frame preselecting `preselection`, and to its
/// first redraw, [`redrawn`] `drawing`: the two screen asks a map makes
/// as it opens.
fn queue_map(
    view: &mut Faked,
    preselection: Option<i16>,
    drawing: nova_sim::galaxy_map::MapDrawing,
) {
    let opening = map_opening(&view.frame, preselection);
    view.sim.queue_screen(opening);
    view.sim.queue_screen(redrawn(drawing));
}

/// What a scripted flight answers the floating map's show
/// ([`ScreenAsk::FloatingMap`]): `drawing`, selecting `preselection`.
fn floating_shown(
    drawing: nova_sim::galaxy_map::MapDrawing,
    preselection: Option<i16>,
) -> ScreenAnswer {
    ScreenAnswer::FloatingMap(Box::new(nova_sim::flight_frame::FloatingMap {
        drawing,
        preselection: preselection.map(SystemId),
    }))
}

/// [`faked`] in `frame`, its flight answering the course map's opening by
/// preselecting Sol and its first redraw with [`sol_drawing`]
/// ([`queue_map`]).
fn faked_sol(frame: FlightFrame) -> Faked {
    let mut view = faked(frame);
    queue_map(&mut view, Some(130), sol_drawing());
    view
}

#[test]
fn a_view_over_a_scripted_flight_shows_its_frame() {
    let mut view = faked(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        jump_readiness: nova_sim::JumpReadiness::Clear,
        ..in_sol()
    });
    faked_hud::changes(&mut view, |frame| frame.explored = vec![SystemId(131)]);
    let scene = view.scene.as_ref().expect("the frame's scene");
    assert_eq!(scene.name(), "Sol", "the frame's system");
    assert_eq!(
        view.nav_now(),
        NavDisplay::Hyperspace {
            destination: hud::NavDestination::Named("Alpha Centauri".to_owned()),
            readiness: nova_sim::JumpReadiness::Clear,
        }
    );
}

#[test]
fn a_scripted_flights_map_sends_its_cycle_and_reports_its_close() {
    let mut view = faked(in_sol());
    view.open_map();
    assert!(!faked_map_input(&mut view, &key(Key::Tab, true)));
    assert_eq!(sent(&view), [FlightCommand::CycleMapSelection]);
    assert!(faked_map_input(&mut view, &key(Key::Enter, true)));
}

/// The nav area showing Alpha Centauri (131), unexplored, in mode 3.
fn cueing_alpha() -> Vec<String> {
    vec![
        hud::NAV_HYPERSPACE.to_owned(),
        hud::NAV_UNEXPLORED.to_owned(),
    ]
}
use nova_sim::mission::screens::SPACEPORT;

/// Which of the bar's parts show as they are now: the gauges, the
/// nav area, the target panel and the radar.
fn retaken(view: &FlightView<FakeCatalog, impl FlightSim>) -> [bool; 4] {
    [
        view.reserves_shown(&view.frame_now()) == view.reserves(),
        view.nav_shown(&view.frame_now()) == view.nav_now(),
        *view.target_shown(&view.frame_now()) == view.target_now(&view.frame_now()),
        view.radar_scene == Some(view.radar_now(&view.frame_now())),
    ]
}

use nova_sim::NpcId;

use nova_sim::{BoomId, Condition, PlayerFate, WeaponId};

use crate::flight::brackets::{self, Standing};
use crate::flight::effects::{DEBRIS_COLOR, Effects};
use crate::flight::target::{self, NO_SECONDARY, NO_TARGET};
use crate::sound::CombatSound;

/// The first command matching `wanted`'s position in `list`.
fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
    list.iter()
        .position(wanted)
        .unwrap_or_else(|| panic!("not drawn: {list:?}"))
}

fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
    move |command| matches!(unclipped(command), DrawCommand::Sprite { image, .. } if image.id == id)
}

/// Whether `command` is the escort menu's border's top edge, the first
/// thing the menu draws: its black inside ORs in nothing, so none is
/// drawn.
fn escort_menu_top(command: &DrawCommand) -> bool {
    use crate::flight::escorts::{MENU_AT, MENU_WIDTH};
    matches!(command, DrawCommand::Line { from, to, .. }
        if *from == MENU_AT && *to == Point::new(MENU_AT.x + MENU_WIDTH, MENU_AT.y))
}

/// `command`, or what it draws when it is [`DrawCommand::Clipped`]: the
/// cursor is clipped to the play area.
fn unclipped(command: &DrawCommand) -> &DrawCommand {
    match command {
        DrawCommand::Clipped { command, .. } => unclipped(command),
        command => command,
    }
}

/// Whether a command draws a `cïcn` in `ids`.
fn icon_in(ids: std::ops::RangeInclusive<i16>) -> impl Fn(&DrawCommand) -> bool {
    move |command| {
        matches!(
            command,
            DrawCommand::Picture { image, .. }
                if image.kind == crate::ImageKind::Icon && ids.contains(&image.id)
        )
    }
}

/// The bracket `cïcn`s drawn (10000-10023), in order.
fn bracket_icons(list: &DrawList) -> Vec<i16> {
    list.iter()
        .filter(|command| icon_in(10000..=10023)(command))
        .map(|command| match command {
            DrawCommand::Picture { image, .. } => image.id,
            _ => unreachable!("a picture"),
        })
        .collect()
}

/// What the target panel's status line shows this frame, if there is
/// a target.
fn status_shown(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<nova_sim::TargetStatus> {
    view.target_shown(&view.frame_now())
        .as_ref()
        .as_ref()
        .map(|panel| panel.status)
}

/// What the target panel's status line would show of the target now.
fn status_now(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<nova_sim::TargetStatus> {
    view.target_now(&view.frame_now()).map(|panel| panel.status)
}

use nova_sim::HailRefusal;

use nova_sim::EscortClass;

const HELLO: &str = "Message Buoy: hello.";

/// The interface sounds `view` holds, taken.
fn ui_sounds(view: &mut FlightView<FakeCatalog, impl FlightSim>) -> Vec<UiSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}
use nova_sim::hire::DEFECTED_ONE;

use nova_sim::MissionTextKind;

/// The background of the status bar `view` shows.
fn bar_shown(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<i16> {
    view.status_bar
        .as_ref()
        .ok()
        .map(|bar| bar.layout.status_bkgnd)
}
/// Stellar `id` with `Strength` 100 and `DeadTime` 5, so that `Y`
/// destroys it, and `Flags2` `flags2`.
fn life(id: i16, flags2: u16) -> nova_sim::StellarLife {
    nova_sim::StellarLife {
        id: StellarId(id),
        strength: 100,
        dead_time: 5,
        flags2,
        explosion: None,
        on_destroy: nova_sim::Script::default(),
        on_regen: nova_sim::Script::default(),
    }
}

/// The four corners from `cïcn` `first` round `centre`, half-size
/// `half`: top-left, top-right, bottom-right, bottom-left.
fn corners_at(first: i16, centre: Point, half: f32) -> Vec<(i16, Point)> {
    let (near, far) = (-half - 16.0, half);
    vec![
        (first, at(centre.x + near, centre.y + near)),
        (first + 1, at(centre.x + far, centre.y + near)),
        (first + 2, at(centre.x + far, centre.y + far)),
        (first + 3, at(centre.x + near, centre.y + far)),
    ]
}

/// The engine tick `view` has reached.
fn now_tick(view: &FlightView<FakeCatalog, impl FlightSim>) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).expect("in range")
}
const HYPER_SELECT: Key = Key::Char('\\');

use nova_sim::GateKind;

/// The course map's sounds `view` holds, taken with the rest.
fn map_sounds(view: &mut FlightView<FakeCatalog, impl FlightSim>) -> Vec<UiSound> {
    ui_sounds(view)
        .into_iter()
        .filter(|sound| matches!(sound, UiSound::MapOpened | UiSound::MapClosed))
        .collect()
}

/// Where on screen `view` draws NPC `id`.
fn npc_on_screen<C, S: FlightSim>(view: &FlightView<C, S>, id: NpcId) -> Point {
    view.camera()
        .world_to_screen(view.shown_npc(&npc_record(view, id)).0)
}

/// NPC `id`'s record in `view`'s kept frame.
fn npc_record<C, S: crate::flight::sim_port::FlightSim>(
    view: &FlightView<C, S>,
    id: NpcId,
) -> nova_sim::flight_frame::FrameShip {
    *view.frame.ship(ShipRef::Npc(id)).expect("in the frame")
}

/// The index in `list` of the first sprite centred on `at`, or of a
/// crossed box's diagonal centred on it.
fn drawn_at(list: &DrawList, at: Point) -> usize {
    let near = |p: Point| (p.x - at.x).abs() < 0.01 && (p.y - at.y).abs() < 0.01;
    list.iter()
        .position(|command| match *command {
            DrawCommand::Sprite { center, .. } => near(center),
            DrawCommand::Line { from, to, .. } => near(Point::new(
                f32::midpoint(from.x, to.x),
                f32::midpoint(from.y, to.y),
            )),
            _ => false,
        })
        .unwrap_or_else(|| panic!("nothing drawn at {at:?}"))
}
/// Quiet sound preferences: effects at level 1.
const QUIET: SoundPrefs = SoundPrefs {
    sound: true,
    music: true,
    effects_level: 1,
    music_level: 7,
};

use crate::flight::floating;
use crate::galaxy::map::{
    BRIGHT_GREEN, CLEARED, CYAN, DARK_GREY, DENIED, DIM_GREEN, HOSTILE, LESS_DARK_GREY, LIGHT_GREY,
};
use crate::geometry::Bounds;
use nova_sim::mission::screens::mac_ticks;

/// The floating map's border lines in `list`: those in the
/// `FloatingMap` colour on the frame's edges, whatever their alpha.
fn floating_border(list: &DrawList) -> Vec<DrawCommand> {
    let border = MENU_COLORS.border;
    list.iter()
        .filter(|command| {
            matches!(command, DrawCommand::Line { from, color, .. }
                if (color.r, color.g, color.b) == (border.r, border.g, border.b)
                    && from.x >= 558.0)
        })
        .cloned()
        .collect()
}

/// The alpha the floating map's border is drawn at, if it is drawn.
fn floating_alpha(view: &FlightView<FakeCatalog, impl FlightSim>) -> Option<u8> {
    floating_border(&drawn(view))
        .first()
        .map(|command| match command {
            DrawCommand::Line { color, .. } => color.a,
            _ => unreachable!(),
        })
}
