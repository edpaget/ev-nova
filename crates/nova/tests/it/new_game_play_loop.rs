//! The new game's fixture (`nova_data::new_game::fixture::folder`) played
//! from New Pilot to the end of its storyline in one test, through the
//! app as `main` builds it with no interface file
//! (`crate::new_game_port::app_over`): the placeholder dialogs, the
//! start-up appear run, the new game's rulebook and the Ink dialog loader.
//! Each frame asserts that nothing failed but a missing picture and that
//! nothing was warned of (`Harness::frame`). Where its sibling
//! `new_game_port_loop` plays the port, this plays the seams between every
//! phase that pins its own piece: one pilot, Ada, through the app's own
//! inputs.
//!
//! 1. **New Pilot**, flying in Hearth, clear: Hearth's first stellar
//!    hailed (1, Y), then a ship (Tab, Y), each reply drawn on its comm
//!    dialog over its backing, over the flight, and closed.
//! 2. **Hearth Dock**, the landing's sound raised, its description drawn
//!    on its backed spaceport; the dock job, grain bought and sold back,
//!    the racked hauler's and the turret ring's descriptions read on the
//!    backed shipyard and outfitter, the hull bought, the ring and a
//!    second mass driver fitted, grain bought to hold: the port loop's
//!    steps, in its order.
//! 3. **The side road** to the Verge, in a nebula: the passage spends the
//!    hull's gate fuel, and the flight is murky. The plate delivered at
//!    Verge Camp, its news shown once; the Recharge at Verge Workshop
//!    refills the fare.
//! 4. **The Spur**, the Verge's dead end, by its road: the passage's fare
//!    spent, and the raider its traffic flies there met as it is, no ship
//!    placed. It is targeted with R (and Tab, once it is disabled and no
//!    threat), fought with Space held down and destroyed: the mass
//!    driver's sound and the raider's explosion raised. The road home
//!    goes through on what the tank still holds.
//! 5. **The road taken**: down at Verge Workshop until the side road has
//!    closed, then Verge Camp, the wardens' toll refused and the road's
//!    offer taken; off the camp the blockade's guards hold the road's
//!    gate and are fought the same way, in the nebula, until every one is
//!    destroyed, and the road is the Guild's.
//! 6. **The storyline is finished**: every `dock-work` mission ended but
//!    the one nothing calls for, and the plate's news heard and read, its
//!    log shown once in the run.
//! 7. **Saved and loaded**: down at Verge Workshop, off it, which fills
//!    the shield and armour, and down again; the window closed, which
//!    saves the pilot, and the pilot opened again in a new app over the
//!    same store: the pilot saved is the pilot before saving, and loads
//!    equal but for the course's start, which the load clears.
//!
//! The fights are won by the fixture's numbers, read nowhere here: the
//! racked hauler's own hull and shields (`ships/racked-hauler.md`). The
//! test never holds the player's ship up, places a ship, nor weakens a
//! hostile: it reads the session only to decide which key to press and
//! to assert. The one state put in rather than played is the approach to
//! a stellar before L, as in every app test (`land_at`,
//! `land_at_verge_camp`), and none is put in while a fight is on.
//!
//! The app's chances stay the harness's `NeverFires`, as the port loop's:
//! every draw decides the same way whatever order it comes in, so the run
//! is the same each time without tying it to the order of draws. Every
//! id, count, text, fare and sound is read from the opened data, the
//! fixture's files and the session, none retyped.

use std::collections::BTreeSet;

use nova::app::{Control, Showing, WindowEvent};
use nova_audio::AudioCommand;
use nova_data::new_game::NewGame;
use nova_data::new_game::fixture::folder;
use nova_data::records::desc::Desc;
use nova_data::records::dude::Dude;
use nova_data::records::govt::Govt;
use nova_data::records::govt::flags::ALWAYS_ATTACKS_PLAYER;
use nova_data::records::mission::Mission;
use nova_data::records::ship::Ship;
use nova_data::records::system::System;
use nova_data::{GameData, Registered, outfit_desc_id};
use nova_sim::catalog::Explosion;
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::sound::SimSound;
use nova_sim::storylines::state::MissionState;
use nova_sim::{
    Condition, Gauge, NpcId, Pilot, PlayerFate, Service, ShipRef, SoundId, StellarId, SystemId,
    Vec2,
};
use nova_view::Key;
use nova_view::flight::CombatLooks;
use nova_view::spaceport::conversation::CLOSE_HINT;
use nova_view::spaceport::layout::{LEAVE_ITEM, service_item};
use nova_view::spaceport::outfitter::DONE_ITEM as OUTFITTER_DONE_ITEM;
use nova_view::spaceport::shipyard::DONE_ITEM as SHIPYARD_DONE_ITEM;
use nova_view::ui::Dialog;
use nova_view::ui::comm::{CLOSE_ITEM as COMM_CLOSE_ITEM, REPLY_ITEM};
use nova_view::ui::planet_comm::CLOSE_ITEM as PLANET_COMM_CLOSE_ITEM;
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{drawn, land_at, new_pilot_flying, opened_from, resumed_in};
use crate::new_game_dialog_backing::{
    assert_backed_between, assert_backed_under, commands, first_text,
};
use crate::new_game_news::headline;
use crate::new_game_port::{app_over, centre_of, click_port_item, pilot, spaceport};
use crate::new_game_port_loop::{
    assert_same, buy_and_sell_back_grain, buy_grain_to_hold, buy_the_racked_hauler,
    deliver_the_plate, fit_the_turret_ring_and_a_mass_driver, flight_session, fly_the_autopilot,
    fly_to_the_verge, land_at_hearth_dock, land_at_verge_camp, outfit, plot_course_to, racked_fare,
    recharge_at_verge_workshop, ship, system, take_off, take_the_dock_job,
};
use crate::new_game_sounds::{cue, mass_driver_sound, resolved, samples_of};
use crate::new_game_travel::{fixture, gates, nova_of, side_road_closing_day};
use crate::pilots::{Harness, saved, texts};

/// The most frames the raider's fight may take: about twice the 1,092
/// frames it was measured to take.
const RAIDER_FIGHT_FRAMES: u32 = 2_200;

/// The most frames the guards' fight may take: about twice the 2,531
/// frames it was measured to take.
const GUARDS_FIGHT_FRAMES: u32 = 5_000;

/// The frames a fight waits after pressing R or Tab before it presses
/// again: the flight takes a press at its next step.
const KEY_FRAMES: u32 = 2;

/// The most times the pilot takes off from Verge Workshop and lands again
/// to wait for the side road to close.
const WAITING_VISITS: u32 = 10;

#[test]
fn a_new_pilot_plays_the_fixture_through_its_storyline_and_the_pilot_saved_after_it_loads_equal() {
    let store = MemoryPilots::new();
    let data = opened_from(&folder());
    let game_ids = fixture();
    let mut heard = Heard::default();
    // 1. New Pilot, flying in Hearth, hailing.
    let mut game = new_pilot_flying(app_over(opened_from(&folder()), &store));
    let start = pilot(&game);
    assert_eq!(system_murk(&game), murk_of(&data, system("hearth")));
    assert_eq!(system_murk(&game), 0, "Hearth is clear");
    // The stellar first: Y hails the ship once Tab has targeted one.
    hail_the_first_stellar(&mut game);
    hail_a_ship(&mut game);
    // 2. Hearth Dock: the landing's sound, the port's description.
    heard.take(&game);
    let mark = heard.mark();
    land_at_hearth_dock(&mut game);
    heard.take(&game);
    assert_landing_heard(&heard, mark, &data, &game_ids);
    read_the_port(&mut game, &data, "hearth-dock");
    take_the_dock_job(&mut game);
    buy_and_sell_back_grain(&mut game);
    read_the_racked_hauler(&mut game, &data);
    buy_the_racked_hauler(&mut game);
    read_the_turret_ring(&mut game, &data);
    fit_the_turret_ring_and_a_mass_driver(&mut game);
    buy_grain_to_hold(&mut game);
    // 3. The side road to the Verge, in its nebula; the plate, the news
    // once, the Recharge.
    take_off(&mut game, &start);
    fly_to_the_verge(&mut game);
    let verge_murk = murk_of(&data, system("verge"));
    assert!(verge_murk > 0, "the Verge's nebula: {verge_murk}");
    assert_eq!(system_murk(&game), verge_murk, "the flight is murky");
    // The delivery's step draws the news log over the camp, its one
    // showing: every later landing asserts it opens no more.
    deliver_the_plate(&mut game);
    recharge_at_verge_workshop(&mut game);
    // 4. The Spur: the raider its traffic flies, fought, and home again.
    let tank = pilot(&game).reserves.fuel;
    assert_eq!(tank, Gauge::full(tank.max), "refilled");
    click_port_item(&mut game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the workshop");
    fly_the_road(&mut game, "verge/spur-road", 1);
    let raiders = arrive_among_the_raiders(&mut game, &data);
    heard.take(&game);
    let mark = heard.mark();
    let raider_fight = fight(&mut game, &raiders, RAIDER_FIGHT_FRAMES, true);
    heard.take(&game);
    assert!(raider_fight.fired, "the player's shots flew");
    assert!(
        raider_fight.turned,
        "the raiders' word turned them on the player"
    );
    assert_weapon_and_explosion_heard(&heard, mark, &data, &game_ids, &raider_fight.hulls);
    fly_the_road(&mut game, "spur/verge-road", 2);
    // 5. Down at the workshop until the side road closes, then the road.
    wait_for_the_side_road(&mut game);
    take_the_road(&mut game);
    let guards = off_the_camp_among_the_guards(&mut game, &data, &game_ids);
    assert_eq!(system_murk(&game), verge_murk, "in the Verge's nebula");
    let guards_fight = fight(&mut game, &guards, GUARDS_FIGHT_FRAMES, false);
    assert!(guards_fight.fired, "the player's shots flew");
    assert!(
        guards_fight.murky,
        "the guards' fight is in the Verge's nebula"
    );
    assert_the_road_is_the_guild_s(&game);
    // 6. The storyline is finished, its news shown once.
    assert_the_storyline_finished(&game, &data);
    // 7. Down at the workshop, off and down again, saved and loaded.
    repair_at_verge_workshop(&mut game);
    save_and_load(game, &store);
}

/// Step 7a: down at Verge Workshop, off it and down again, no landing
/// opening the news log. The take-off fills the shield and armour the
/// guards cost, as the original's does (`nova_sim`'s `land_on_stellar`
/// module): the landing leaves them as they were, and so would the save,
/// which the load then fills.
fn repair_at_verge_workshop(game: &mut Harness) {
    land_at(game, "verge-workshop");
    assert_no_news_log(game, "the workshop");
    click_port_item(game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the workshop");
    let off = pilot(game).reserves;
    assert_eq!(
        (off.shield, off.armor),
        (Gauge::full(off.shield.max), Gauge::full(off.armor.max)),
        "the take-off fills them"
    );
    land_at(game, "verge-workshop");
    assert_no_news_log(game, "the workshop again");
}

/// The plate's news, its id, read from the delivery's release.
fn plate_news() -> String {
    let news = opened_from(&folder()).storylines().missions["dock-job"]
        .on_success
        .news
        .clone();
    let [id] = news.as_slice() else {
        panic!("the delivery releases one news item: {news:?}");
    };
    id.clone()
}

/// Asserts that the next frame doesn't draw the plate's news headline:
/// no news log opened over the landing at `place`.
#[track_caller]
fn assert_no_news_log(game: &mut Harness, place: &str) {
    let page = drawn(game);
    assert!(
        !page.contains(&headline(&plate_news())),
        "{place}: no news log: {page:?}"
    );
}

/// `system`'s murk, as its `sÿst` in the opened data gives it.
fn murk_of(data: &GameData, system: SystemId) -> i16 {
    record::<System>(data, system.0).murk
}

/// The murk the flight frame draws the system under.
fn system_murk(game: &Harness) -> i16 {
    let mut frame = FlightFrame::default();
    flight_session(game).fill_frame(&mut frame);
    frame.system_murk
}

/// `id`'s record of type `T` in `data`.
fn record<T: Registered + Clone>(data: &GameData, id: i16) -> T {
    data.get::<T>(id)
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

/// Asserts that the next frame draws `text`, a `dësc`'s words, however its
/// lines wrap.
#[track_caller]
fn assert_draws(game: &mut Harness, text: &str, what: &str) {
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let page = words(&texts(&game.frame()).join(" "));
    assert!(page.contains(&words(text)), "{what}: {text:?} in {page:?}");
}

/// Step 1b: Tab targets the nearest ship of Hearth's traffic and Y hails
/// it: its reply is drawn on the comm dialog, over its backing, over the
/// flight. The dialog's Close closes it.
fn hail_a_ship(game: &mut Harness) {
    game.press(KeyCode::Tab);
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm, "the hail opens the comm");
    let comm = game.app.screen().comm().expect("the comm dialog");
    let reply = comm
        .dialog()
        .text(REPLY_ITEM)
        .expect("the reply item")
        .lines()
        .join(" ");
    assert!(!reply.trim().is_empty(), "the ship says something");
    assert_backed_between(&commands(game), comm.dialog(), &first_text(comm));
    let close = item_centre(comm.dialog(), COMM_CLOSE_ITEM);
    let page = texts(&game.frame()).join(" ");
    assert!(page.contains(reply.trim()), "{reply:?} in {page:?}");
    game.click(close);
    game.frame();
    assert_eq!(game.showing(), Showing::Flight, "closed");
}

/// Step 1a: 1 selects Hearth's first stellar and Y hails it: its
/// greeting, its name following, is drawn on the planet comm dialog,
/// backed over the flight. The dialog's Close closes it.
fn hail_the_first_stellar(game: &mut Harness) {
    game.press(KeyCode::Digit1);
    game.frame();
    game.press(KeyCode::KeyY);
    assert_eq!(game.showing(), Showing::Comm, "the hail opens the comm");
    let comm = game.app.screen().planet_comm().expect("the planet comm");
    let view = comm.view().clone();
    let named = format!("{}.", view.name);
    assert!(
        view.reply.len() > named.len() && view.reply.ends_with(&named),
        "a greeting, then the name: {:?}",
        view.reply
    );
    assert_backed_between(&commands(game), comm.dialog(), &first_text(comm));
    let close = item_centre(comm.dialog(), PLANET_COMM_CLOSE_ITEM);
    let page = texts(&game.frame()).join(" ");
    assert!(page.contains(&view.reply), "{:?} in {page:?}", view.reply);
    game.click(close);
    game.frame();
    assert_eq!(game.showing(), Showing::Flight, "closed");
}

/// The centre of `dialog`'s item `item`.
fn item_centre(dialog: &Dialog, item: usize) -> nova_view::geometry::Point {
    dialog.item_bounds(item).expect("an item").center()
}

/// Step 2a: the stellar `id`'s spaceport is backed under its text, and
/// draws the stellar's own `dësc`, as the opened data holds it.
fn read_the_port(game: &mut Harness, data: &GameData, id: &str) {
    let stellar = StellarId(nova_of(&fixture().ids.stellars, id));
    let port = spaceport(game);
    let dialog = port.dialog().expect("laid out");
    assert_backed_under(&commands(game), dialog, &first_text(port));
    let desc = record::<Desc>(data, stellar.0);
    assert_draws(game, desc.text.as_str(), id);
}

/// Step 2b: before the purchase, the racked hauler's cell clicked at the
/// shipyard draws its `dësc` on the backed shipyard, which Done closes.
fn read_the_racked_hauler(game: &mut Harness, data: &GameData) {
    let racked = ship("racked-hauler");
    click_port_item(game, service_item(Service::Shipyard));
    game.frame();
    let shipyard = spaceport(game).open_shipyard().expect("the shipyard");
    let index = (shipyard.shipyard().rows.iter())
        .position(|row| row.id == racked)
        .expect("listed");
    let cell = shipyard.cell_bounds(index).expect("a cell").center();
    game.click(cell);
    let text = data
        .ship_description(racked)
        .expect("described")
        .expect("decoded")
        .record
        .text
        .to_string();
    assert_draws(game, &text, "the racked hauler");
    let shipyard = spaceport(game).open_shipyard().expect("still open");
    let dialog = shipyard.dialog().expect("laid out");
    assert_backed_under(&commands(game), dialog, &first_text(shipyard));
    let done = centre_of(shipyard.dialog(), SHIPYARD_DONE_ITEM);
    game.click(done);
    game.frame();
    assert!(spaceport(game).open_shipyard().is_none(), "closed");
}

/// Step 2c: before the fitting, the turret ring's cell clicked at the
/// outfitter draws its `dësc` on the backed outfitter, which Done closes.
fn read_the_turret_ring(game: &mut Harness, data: &GameData) {
    let ring = outfit("turret-ring");
    click_port_item(game, service_item(Service::Outfitter));
    game.frame();
    let outfitter = spaceport(game).open_outfitter().expect("the outfitter");
    let index = (outfitter.outfitter().rows.iter())
        .position(|row| row.id == ring)
        .expect("listed");
    let cell = outfitter.cell_bounds(index).expect("a cell").center();
    game.click(cell);
    let desc = outfit_desc_id(nova_data::OutfitId(ring.0)).expect("an outfit's dësc");
    let text = record::<Desc>(data, desc).text.to_string();
    assert_draws(game, &text, "the turret ring");
    let outfitter = spaceport(game).open_outfitter().expect("still open");
    let dialog = outfitter.dialog().expect("laid out");
    assert_backed_under(&commands(game), dialog, &first_text(outfitter));
    let done = centre_of(outfitter.dialog(), OUTFITTER_DONE_ITEM);
    game.click(done);
    game.frame();
    assert!(spaceport(game).open_outfitter().is_none(), "closed");
}

/// The sounds the app asked its audio to play, gathered across the run:
/// every `Harness::sounds` drains the shared log, so this is the one
/// reader.
#[derive(Default)]
struct Heard(Vec<SoundId>);

impl Heard {
    /// Gathers what `game` played since the last call.
    fn take(&mut self, game: &Harness) {
        self.0.extend(
            game.sounds()
                .into_iter()
                .filter_map(|command| match command {
                    AudioCommand::Play { sound, .. }
                    | AudioCommand::PlayUnlessPlaying { sound, .. } => Some(sound),
                    _ => None,
                }),
        );
    }

    /// How many have been gathered: a mark for [`Heard::since`].
    fn mark(&self) -> usize {
        self.0.len()
    }

    /// Whether `sound` was played after `mark`.
    fn since(&self, mark: usize, sound: SoundId) -> bool {
        self.0[mark..].contains(&sound)
    }
}

/// A sound role `game.md` fills.
#[derive(Clone, Copy, Debug)]
enum Role {
    Landing,
    Explosion,
}

/// The clip `game.md` names for `role`.
fn role_clip(game: &NewGame, role: Role) -> String {
    let roles = &game.galaxy.game.as_ref().expect("game.md").data.sounds;
    let named = match role {
        Role::Landing => &roles.landing,
        Role::Explosion => &roles.explosion,
    };
    named
        .as_ref()
        .map_or_else(|| panic!("the fixture fills {role:?}"), ToString::to_string)
}

/// Criterion 3, the landing: its cue played after `mark`, and it resolves
/// to the clip `game.md` names as the landing's.
fn assert_landing_heard(heard: &Heard, mark: usize, data: &GameData, game: &NewGame) {
    let landed = cue(SimSound::Landed);
    assert!(heard.since(mark, landed), "the landing's snd {}", landed.0);
    let clip = role_clip(game, Role::Landing);
    assert_eq!(resolved(data, landed), Some(samples_of(game, &clip)));
}

/// Criterion 3, the weapon and the explosion: after `mark`, the mass
/// driver's sound played as it fired, and the explosion each of `hulls`,
/// the hulls destroyed, sets off, which resolves to the clip `game.md`
/// names as the explosion's.
fn assert_weapon_and_explosion_heard(
    heard: &Heard,
    mark: usize,
    data: &GameData,
    game: &NewGame,
    hulls: &[nova_sim::ShipId],
) {
    let fired = mass_driver_sound(data, game);
    assert!(
        heard.since(mark, fired),
        "the mass driver's snd {}",
        fired.0
    );
    let clip = role_clip(game, Role::Explosion);
    for hull in hulls {
        let blast = Explosion::decode(record::<Ship>(data, hull.0).explode2).expect("an explosion");
        let boom = data
            .boom_look(blast.boom)
            .expect("a bööm")
            .expect("its look")
            .sound
            .expect("heard");
        assert!(
            heard.since(mark, boom),
            "{hull:?}'s explosion, snd {}",
            boom.0
        );
        assert_eq!(resolved(data, boom), Some(samples_of(game, &clip)));
    }
}

/// The far end of the jump point `road` (`system/point`): its partner's
/// system, and its partner's gate, where the road comes out.
fn far_end_of(road: &str) -> (SystemId, Vec2) {
    let game = fixture();
    let near = StellarId(nova_of(&game.ids.gates, road));
    let (_, system, exit) = gates(&game)
        .into_iter()
        .find(|candidate| candidate.stellar == near)
        .expect("the road's gate")
        .partner;
    (system, exit)
}

/// The course through the jump point `road`, clicked on the map and flown
/// on the gate autopilot with J: it comes out at the road's far gate,
/// on the same day, its tank the full one less `fares` of the racked
/// hauler's fares, never empty.
fn fly_the_road(game: &mut Harness, road: &str, fares: u8) {
    let before = pilot(game);
    let (to, exit) = far_end_of(road);
    plot_course_to(game, to);
    game.press(KeyCode::KeyJ);
    fly_the_autopilot(game);
    let arrived = pilot(game);
    assert_eq!(arrived.system, to, "came out through {road}");
    assert_eq!(flight_session(game).player().position, exit);
    assert_eq!(arrived.story.day, before.story.day, "a gate passes no day");
    let fuel = arrived.reserves.fuel;
    assert_eq!(
        fuel,
        Gauge {
            now: fuel.max - f32::from(fares) * racked_fare(),
            ..fuel
        },
        "{road}: the fares spent"
    );
    assert!(fuel.now > 0.0, "fuel left");
}

/// Step 4a: on the first frame in the Spur, its traffic is there as its
/// `sÿst` names it: at least one ship flies a hull of one of its `düde`s,
/// under that `düde`'s government, with no mission and no person. They
/// are the ones fought, and the only ships there.
fn arrive_among_the_raiders(game: &mut Harness, data: &GameData) -> Vec<NpcId> {
    let spur = record::<System>(data, system("spur").0);
    let fleets: Vec<(Option<i16>, Vec<i16>)> = (spur.dude_types.iter())
        .filter(|&&dude| dude >= 0)
        .map(|&dude| {
            let dude = record::<Dude>(data, dude);
            let hulls = dude.ship_type.iter().flatten().map(|ship| ship.0).collect();
            (dude.govt.map(|govt| govt.0), hulls)
        })
        .collect();
    assert!(!fleets.is_empty(), "the Spur names its traffic");
    let session = flight_session(game);
    let raiders: Vec<NpcId> = (session.npcs().iter())
        .filter(|npc| {
            npc.mission.is_none()
                && npc.person.is_none()
                && fleets.iter().any(|(govt, hulls)| {
                    npc.govt.map(|govt| govt.0) == *govt && hulls.contains(&npc.ship.0)
                })
        })
        .map(|npc| npc.id)
        .collect();
    assert!(
        !raiders.is_empty(),
        "the Spur's traffic: {:?}",
        session.npcs()
    );
    for npc in session.npcs() {
        let govt = npc.govt.map(|govt| record::<Govt>(data, govt.0));
        assert!(
            govt.is_some_and(|govt| govt.flags.0 & ALWAYS_ATTACKS_PLAYER != 0),
            "{:?}: hostile, its government attacking the player on sight",
            npc.id
        );
    }
    assert_eq!(
        session.npcs().len(),
        raiders.len(),
        "nothing else flies there"
    );
    raiders
}

/// What a fight showed.
struct Fought {
    /// The hulls destroyed, in the order they went.
    hulls: Vec<nova_sim::ShipId>,
    /// Whether a shot of the player's was ever in flight.
    fired: bool,
    /// Whether one of the hostiles ever targeted the player.
    turned: bool,
    /// Whether the system stayed under its murk throughout.
    murky: bool,
}

/// Holds Space down and targets with R, then Tab, until every one of
/// `hostiles` has broken up and gone, for at most `frames` frames. Each
/// frame the player is still flying, and every ship seen breaking up is
/// one of `hostiles`; when `closed`, no ship comes into the system
/// meanwhile either.
fn fight(game: &mut Harness, hostiles: &[NpcId], frames: u32, closed: bool) -> Fought {
    let murk = system_murk(game);
    let mut fought = Fought {
        hulls: Vec::new(),
        fired: false,
        turned: false,
        murky: true,
    };
    let mut dying: BTreeSet<NpcId> = BTreeSet::new();
    let mut tabs = 0;
    let mut tried_r = false;
    let mut wait = 0;
    hold_fire(game, true);
    for _ in 0..frames {
        let session = flight_session(game);
        let here: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        let left: Vec<NpcId> = (hostiles.iter().copied())
            .filter(|id| here.contains(id))
            .collect();
        if left.is_empty() {
            hold_fire(game, false);
            assert_eq!(
                dying,
                hostiles.iter().copied().collect(),
                "each broke up before it went"
            );
            return fought;
        }
        // One breaking up can't be targeted: it is only waited for.
        let standing = left.iter().any(|id| !dying.contains(id));
        // A press is taken at the flight's next step, which may come a
        // frame later: each waits out `KEY_FRAMES` before the next.
        if !standing || session.target().is_some_and(|npc| left.contains(&npc.id)) {
            (tried_r, tabs, wait) = (false, 0, 0);
        } else if wait > 0 {
            wait -= 1;
        } else if tried_r {
            // A disabled hostile is no threat: Tab cycles to it.
            tabs += 1;
            assert!(tabs <= here.len(), "Tab never reached a hostile");
            game.press(KeyCode::Tab);
            wait = KEY_FRAMES;
        } else {
            tried_r = true;
            game.press(KeyCode::KeyR);
            wait = KEY_FRAMES;
        }
        game.frame();
        let session = flight_session(game);
        assert_eq!(
            session.player_fate(),
            PlayerFate::Flying,
            "the player flies"
        );
        fought.fired |= session
            .shots()
            .iter()
            .any(|shot| shot.firer == ShipRef::Player);
        fought.murky &= system_murk(game) == murk;
        for npc in session.npcs() {
            if closed {
                assert!(here.contains(&npc.id), "{:?} came in", npc.id);
            }
            if matches!(npc.condition, Condition::Dying { .. }) {
                assert!(hostiles.contains(&npc.id), "{:?} broke up", npc.id);
                if dying.insert(npc.id) {
                    fought.hulls.push(npc.ship);
                }
            }
            fought.turned |= hostiles.contains(&npc.id) && npc.target == Some(ShipRef::Player);
        }
    }
    let session = flight_session(game);
    panic!(
        "the fight goes on after {frames} frames: the player {:?}, the ships {:?}",
        session.reserves(),
        (session.npcs().iter())
            .map(|npc| (npc.id, npc.reserves, npc.condition))
            .collect::<Vec<_>>()
    );
}

/// Presses (`held`) or lets go of the fire key.
fn hold_fire(game: &mut Harness, held: bool) {
    let event = WindowEvent::Key {
        key: Key::Space,
        pressed: held,
        repeat: false,
    };
    assert_eq!(game.handle(event), Control::Continue);
}

/// Step 5a: down at Verge Workshop, and off and down there again while
/// the side road is still open. No landing opens the news log: the news
/// heard there was read at the camp.
fn wait_for_the_side_road(game: &mut Harness) {
    land_at(game, "verge-workshop");
    assert_no_news_log(game, "the workshop");
    let mut visits = 0;
    while pilot(game).story.day < side_road_closing_day() {
        visits += 1;
        assert!(visits <= WAITING_VISITS, "the side road never closes");
        click_port_item(game, LEAVE_ITEM);
        land_at(game, "verge-workshop");
        assert_no_news_log(game, "the workshop, waiting");
    }
}

/// Step 5b: off the workshop and down at Verge Camp: the wardens' toll
/// offered and refused, the road's offer taken, then the camp's greeting
/// closed, and no news log: the plate's news heard there is read.
fn take_the_road(game: &mut Harness) {
    click_port_item(game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the workshop");
    land_at_verge_camp(game);
    game.press(KeyCode::Digit2);
    game.press(KeyCode::Enter);
    assert_eq!(
        pilot(game).story.mission("warden-toll"),
        Some(MissionState::Declined),
        "the toll refused"
    );
    game.press(KeyCode::Digit1);
    game.press(KeyCode::Enter);
    assert_eq!(
        pilot(game).story.mission("clear-the-road"),
        Some(MissionState::Active {
            cargo: 0,
            met: false
        }),
        "the road taken on"
    );
    let page = drawn(game);
    assert!(
        page.contains(&CLOSE_HINT.to_owned()),
        "the greeting: {page:?}"
    );
    game.press(KeyCode::Enter);
    let page = drawn(game);
    assert!(!page.contains(&CLOSE_HINT.to_owned()), "closed: {page:?}");
    assert_no_news_log(game, "the camp");
    let verge = pilot(game).system;
    let heard = flight_session(game).heard_news(verge);
    let item = heard.iter().find(|item| item.id == plate_news());
    assert!(item.is_some_and(|item| item.read), "heard, read: {heard:?}");
}

/// Step 5c: off the camp, the guards the road's blockade holds at its
/// gate: the ships marked with a mission, as many as its `mïsn`'s
/// `ShipCount`, read from the opened data.
fn off_the_camp_among_the_guards(
    game: &mut Harness,
    data: &GameData,
    game_ids: &NewGame,
) -> Vec<NpcId> {
    click_port_item(game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the camp");
    let guards: Vec<NpcId> = (flight_session(game).npcs().iter())
        .filter(|npc| npc.mission.is_some())
        .map(|npc| npc.id)
        .collect();
    let blockade = nova_of(&game_ids.ids.blockades, "clear-the-road");
    let count = record::<Mission>(data, blockade).ship_count;
    assert_eq!(
        guards.len(),
        usize::try_from(count).expect("a count"),
        "the blockade's guards"
    );
    guards
}

/// Step 5d: every guard destroyed, the road is taken and is the Guild's.
fn assert_the_road_is_the_guild_s(game: &Harness) {
    let road = pilot(game);
    assert_eq!(
        road.story.mission("clear-the-road"),
        Some(MissionState::Done)
    );
    let taken = opened_from(&folder()).storylines().missions["clear-the-road"]
        .on_success
        .points
        .clone();
    assert!(!taken.is_empty(), "the road's success changes its point");
    for change in &taken {
        assert_eq!(
            road.story.points[&change.point].owner, change.owner,
            "{}",
            change.point
        );
    }
}

/// Step 6: every mission of the dock job's storyline has ended, but those
/// nothing in the fixture calls for, which never started; and the only
/// news heard is the plate's, read.
fn assert_the_storyline_finished(game: &Harness, data: &GameData) {
    let storylines = data.storylines();
    let line = storylines.missions["dock-job"]
        .storyline
        .as_ref()
        .expect("the dock job's storyline")
        .id
        .clone();
    let set: BTreeSet<&String> = (storylines.missions.values())
        .flat_map(|mission| {
            [
                &mission.on_offer,
                &mission.on_accept,
                &mission.on_decline,
                &mission.on_success,
            ]
        })
        .flat_map(|effects| &effects.flags)
        .collect();
    let finished = pilot(game);
    let mut ended = 0;
    for (id, mission) in &storylines.missions {
        if mission.storyline.as_ref().is_none_or(|of| of.id != line) {
            continue;
        }
        let state = finished.story.mission(id);
        if mission
            .requires
            .flags
            .iter()
            .any(|flag| !set.contains(flag))
        {
            assert_eq!(state, None, "{id}: nothing calls for it");
            continue;
        }
        ended += 1;
        assert!(
            matches!(state, Some(MissionState::Done | MissionState::Declined)),
            "{id}: {state:?}"
        );
    }
    assert!(ended > 1, "the storyline's missions");
    // The side road's news broke at Hearth and never reaches the Verge.
    let heard: Vec<String> = (flight_session(game).heard_news(finished.system).into_iter())
        .map(|item| item.id)
        .collect();
    assert_eq!(heard, [plate_news()], "the one news item heard here");
    assert_eq!(
        finished.story.read,
        BTreeSet::from([plate_news()]),
        "and read"
    );
}

/// Step 7: saved as the window closes, the pilot is the pilot before
/// saving, field by field, and loads so in a new app over the same store
/// but for the course's start, which the load clears.
fn save_and_load(mut game: Harness, store: &MemoryPilots) {
    let before = pilot(&game);
    assert_eq!(before.ship, ship("racked-hauler"), "the bought hull");
    assert_eq!(before.owned(outfit("dock-pass")), 0, "the lent pass back");
    // The spur road's exit, the last gate the pilot came out of, restarted
    // the course at the Verge; only the load clears it.
    assert_eq!(
        (before.course.as_slice(), before.course_start),
        ([].as_slice(), Some(system("verge"))),
        "the course restarted at the Verge"
    );
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_same(&saved(store, "Ada"), &before, "saved");
    let mut resumed = resumed_in(app_over(opened_from(&folder()), store));
    resumed.frame();
    let cleared = Pilot {
        course_start: None,
        ..before
    };
    assert_same(&pilot(&resumed), &cleared, "loaded");
}
