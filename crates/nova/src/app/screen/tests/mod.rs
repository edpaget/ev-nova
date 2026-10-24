use std::io;
use std::path::Path;
use std::rc::Rc;

use nova_data::graphics::RLED;
use nova_data::graphics::fixture::RledBuilder;
use nova_data::records::character::Character;
use nova_data::records::desc::Desc;
use nova_data::records::mission::Mission;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::string_list::StrList;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, Record, SystemId};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader};
use nova_sim::{RuleKey, RuleSource};
use nova_view::galaxy::GalaxyMap;
use nova_view::galaxy::map::ENTER_BUTTON;
use nova_view::geometry::Bounds;
use nova_view::menu::fixture::main_screen_resources;
use nova_view::ships::{ShipBrowser, ShipId};
use nova_view::sound::SimSound;
use nova_view::spaceport::layout::LEAVE_ITEM;
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
use nova_view::{DrawCommand, Font, Key, MouseButton, Point, Sound, UiSound};

use super::*;

use super::port_dialogs::start_up_words;
use nova_sim::{
    Allegiance, NovaAi, NovaBits, NovaBoarding, NovaDisable, NovaHire, NovaLaw, NovaPersons,
    OfferPrompt, Session,
};
use nova_view::movie::MoviePlayer;
use nova_view::spaceport::bar::{BAR_DIALOG, PICTURE_BAR_DIALOG};
use nova_view::spaceport::layout::SPACEPORT_DIALOG;
use nova_view::spaceport::missions::{INFO_DIALOG, MissionInfoScreen};
use nova_view::spaceport::offer::{OFFER_DIALOG, PICTURE_OFFER_DIALOG};
use nova_view::spaceport::shipyard::{SHIP_INFO_DIALOG, SHIPYARD_DIALOG};
use nova_view::ui::DescriptionSource;
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::DescShown;
use nova_view::ui::player_info::{PLAYER_INFO_DIALOG, YES_NO_DIALOG};
use nova_view::ui::prefs::PREFS_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::{DrawList, Screen, ScreenAction};

use nova_data::graphics::PICT;
use nova_data::graphics::fixture::{DirectBits, PictBuilder};
use nova_data::records::dude::Dude;
use nova_data::records::govt::Govt;
use nova_data::records::interface::Interface;
use nova_data::records::person::Person;
use nova_data::records::weapon::Weapon;
use nova_sim::Good;
use nova_sim::MissionId;
use nova_sim::OutfitId;
use nova_sim::Reply;
use nova_sim::fixture::MemoryPilots;
use nova_sim::hail::{Answer, Ask, Hail, HailOption};
use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};
use nova_sim::reserves::Gauge;
use nova_sim::{Chance, Goal, HullSpec, Npc, Surroundings};
use nova_sim::{PilotKeeper, PilotStore};
use nova_view::devtools::{PilotDesk, PilotEdit};
use nova_view::menu::MenuChoice;
use nova_view::spaceport::TradeScreen;
use nova_view::spaceport::bar::BarScreen;
use nova_view::spaceport::outfitter::{
    BUY_ITEM as OUTFIT_BUY_ITEM, NEGATIVE_FREE_MASS, OUTFIT_DIALOG, SELL_ITEM as OUTFIT_SELL_ITEM,
};
use nova_view::spaceport::trade::{BUY_ITEM, TRADE_DIALOG};
use nova_view::spaceport::{OutfitterScreen, ShipyardScreen};
use nova_view::ui::Dialog;
use nova_view::ui::comm::{CLOSE_ITEM, COMM_DIALOG, HAGGLE_DIALOG, LOWER_ITEM};
use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG};
use nova_view::ui::note::{LOAD_FALLBACK_NOTE, NOTE_DIALOG};
use nova_view::ui::player_info::{PLAYER_INFO_KEY, PlayerInfoButton, Tab};
use nova_view::ui::plunder::{
    ASSIGNMENT_DIALOG, ESCORT_ITEM, MY_SHIP_ITEM, PLUNDER_DIALOG, PLUNDER_PICTURE,
};
use std::cell::RefCell;
use std::collections::VecDeque;

mod appear;
mod christening;
mod death;
mod dialog_maps;
mod encounters;
mod entry;
mod flight;
mod game;
mod launch;
mod map_find;
mod menu;
mod menu_commands;
mod missions;
mod new_pilot;
mod overlays;
mod port;
mod quit_question;
mod widescreen;

/// One data file, `/data/Nova Data`, holding a fork.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

/// An independent `sÿst` at (`x`, 0) with no hyperlinks and these
/// stellars.
fn system(x: i16, stellars: &[i16]) -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    bytes[0..2].copy_from_slice(&x.to_be_bytes());
    for slot in 0..32 {
        bytes[0x04 + 2 * slot..0x06 + 2 * slot].copy_from_slice(&(-1_i16).to_be_bytes());
    }
    for (slot, id) in stellars.iter().enumerate() {
        bytes[0x24 + 2 * slot..0x26 + 2 * slot].copy_from_slice(&id.to_be_bytes());
    }
    bytes[0x66..0x68].copy_from_slice(&(-1_i16).to_be_bytes());
    bytes
}

/// Ships 129 and 128, each with a `shän` naming a 4-frame `rlëD` (one
/// set of 4 rotations), and systems 128 and 129, 300 apart. System 128
/// holds stellar 128 at (0, 0), a planet with a bar, whose `spïn` 1000
/// names the same `rlëD`; system 129 holds stellar 129, Beta Prime,
/// another such planet. The only `chär` starts in ship 128, an
/// average ship, in system 128, over the planet.
fn data() -> Rc<GameData> {
    game_data(false, false, false)
}

/// [`data`], where the planet is also a trade center trading food at
/// 75 (`STR#` 4000 and 4004), the `chär` holds 1000 credits and ship
/// 128 holds 10 tons.
fn trading_data() -> Rc<GameData> {
    game_data(true, false, false)
}

/// [`trading_data`], where the planet, of tech level 1, is also an
/// outfitter selling `oütf` 128, a speed booster (+300) of 2 tons for
/// 500 credits, up to 3, and ship 128 has 10 tons free.
fn outfitting_data() -> Rc<GameData> {
    game_data(true, true, false)
}

/// [`outfitting_data`], where the planet is also a shipyard selling
/// ship 129, twice as fast with 20 tons of cargo space and 12 free,
/// for 800 credits, carrying a booster; ship 128 costs nothing.
fn shipbuying_data() -> Rc<GameData> {
    game_data(true, true, true)
}

/// A `STR#` of `strings`.
fn str_list(strings: &[&str]) -> Vec<u8> {
    let mut bytes = u16::try_from(strings.len())
        .expect("few")
        .to_be_bytes()
        .to_vec();
    for string in strings {
        bytes.push(u8::try_from(string.len()).expect("short"));
        bytes.extend(string.as_bytes());
    }
    bytes
}

fn game_data(trading: bool, outfitting: bool, shipbuying: bool) -> Rc<GameData> {
    game_data_selling(trading, outfitting, shipbuying, 100)
}

/// [`game_data`], ship 129 for sale on a `BuyRandom` of `buy_random`.
fn game_data_selling(
    trading: bool,
    outfitting: bool,
    shipbuying: bool,
    buy_random: i16,
) -> Rc<GameData> {
    game_data_with(trading, outfitting, shipbuying, buy_random, b"", b"")
}

/// [`data`], where the `chär`'s `OnStart` is `on_start`.
fn starting_data(on_start: &[u8]) -> Rc<GameData> {
    game_data_starting(false, false, false, on_start, b"")
}

/// [`data`], where Alpha (128) links Beta (129), which is visible
/// until bit 5 is set (`Visibility` `!b5`), and the `chär`'s `OnStart`
/// sets bit 5: a new pilot hides Beta as its game begins.
fn hiding_beta_data() -> Rc<GameData> {
    let mut built = game_data_fork(false, false, false, 100, b"b5", b"", None).build();
    let alpha = built.layout.data_entry(System::TYPE, 128) + 4;
    built.put_u16(alpha + 0x04, 129);
    let beta = built.layout.data_entry(System::TYPE, 129) + 4;
    for (at, byte) in b"!b5".iter().enumerate() {
        built.put_u8(beta + 0x96 + at, *byte);
    }
    loaded_bytes(built.bytes)
}

/// Whether the pilot of `flight` has explored Beta (129).
fn explored_beta(flight: &FlightView<Rc<GameData>>) -> bool {
    flight
        .pilot()
        .expect("a pilot")
        .has_explored(nova_sim::SystemId(129))
}

/// [`outfitting_data`], where the booster's `OnPurchase` is
/// `on_purchase`, and `STR#` 25048 holds "Off you go, <PSN>."
fn hooked_outfitting_data(on_purchase: &[u8]) -> Rc<GameData> {
    game_data_starting(true, true, false, b"", on_purchase)
}

/// [`game_data`], where the `chär`'s `OnStart` is `on_start` and the
/// booster's `OnPurchase` is `on_purchase`.
fn game_data_starting(
    trading: bool,
    outfitting: bool,
    shipbuying: bool,
    on_start: &[u8],
    on_purchase: &[u8],
) -> Rc<GameData> {
    game_data_with(trading, outfitting, shipbuying, 100, on_start, on_purchase)
}

/// [`outfitting_data`], where the booster is a pod of -20 tons, and
/// the outfitter also sells `oütf` 129, a hold of 25 tons for 100
/// credits: with both owned, 5 tons are free, and selling the pod
/// would leave -15.
fn pod_data() -> Rc<GameData> {
    game_data_full(true, true, false, 100, b"", b"", Some(500))
}

/// [`pod_data`], the pod 450 credits: the 1000 credits buy two pods
/// and the hold, leaving 25 tons free, so one pod sold leaves 5 and a
/// second would leave -15.
fn two_pod_data() -> Rc<GameData> {
    game_data_full(true, true, false, 100, b"", b"", Some(450))
}

/// [`game_data`], ship 129 for sale on a `BuyRandom` of `buy_random`,
/// the `chär`'s `OnStart` `on_start` and the booster's `OnPurchase`
/// `on_purchase`.
fn game_data_with(
    trading: bool,
    outfitting: bool,
    shipbuying: bool,
    buy_random: i16,
    on_start: &[u8],
    on_purchase: &[u8],
) -> Rc<GameData> {
    game_data_full(
        trading,
        outfitting,
        shipbuying,
        buy_random,
        on_start,
        on_purchase,
        None,
    )
}

/// [`game_data_with`], the booster a pod of -20 tons priced `pod` and
/// a 25-ton hold (`oütf` 129) for sale too when `pod` is some
/// ([`pod_data`]).
#[allow(clippy::fn_params_excessive_bools)] // A fixture's switches.
fn game_data_full(
    trading: bool,
    outfitting: bool,
    shipbuying: bool,
    buy_random: i16,
    on_start: &[u8],
    on_purchase: &[u8],
    pod: Option<i32>,
) -> Rc<GameData> {
    loaded(&game_data_fork(
        trading,
        outfitting,
        shipbuying,
        buy_random,
        on_start,
        on_purchase,
        pod,
    ))
}

/// Alpha Prime's `CustSndID`, its ambient sound: Port Kane's in the
/// stock data.
const AMBIENT: i16 = 10_032;

/// Beta Prime's `CustSndID`, another ambient sound.
const BETA_AMBIENT: i16 = 10_033;

/// The resources of [`game_data_full`], before they are loaded.
#[allow(clippy::fn_params_excessive_bools)] // A fixture's switches.
fn game_data_fork(
    trading: bool,
    outfitting: bool,
    shipbuying: bool,
    buy_random: i16,
    on_start: &[u8],
    on_purchase: &[u8],
    pod: Option<i32>,
) -> ForkBuilder {
    let anim = ship_anim();
    let sheet = (0..4)
        .fold(RledBuilder::new(1, 1), |sheet, _| {
            sheet.frame(|f| f.line().pixels(&[0x7C00]))
        })
        .build();
    let mut average = vec![0; Ship::SIZE.expect("fixed")];
    for (at, value) in [(0x04, 300_i16), (0x06, 300), (0x08, 10)] {
        average[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }
    let mut character = vec![0; Character::SIZE.expect("fixed")];
    character[0x32..0x32 + on_start.len()].copy_from_slice(on_start);
    let mut stellar = landable();
    stellar[0x1A..0x1C].copy_from_slice(&AMBIENT.to_be_bytes());
    let mut beta = landable();
    beta[0x1A..0x1C].copy_from_slice(&BETA_AMBIENT.to_be_bytes());
    let mut fork = ForkBuilder::new();
    if trading {
        average[0x00..0x02].copy_from_slice(&10_i16.to_be_bytes());
        character[0x00..0x04].copy_from_slice(&1000_i32.to_be_bytes());
        stellar[0x06..0x0A].copy_from_slice(&0x2000_0043_u32.to_be_bytes());
        fork = fork
            .resource(StrList::TYPE, 4000, None, &str_list(&["Food"]))
            .resource(StrList::TYPE, 4004, None, &str_list(&["75"]));
    }
    if outfitting {
        stellar[0x09] |= 0x04;
        stellar[0x0C..0x0E].copy_from_slice(&1_i16.to_be_bytes());
        average[0x0C..0x0E].copy_from_slice(&10_i16.to_be_bytes());
        let mut booster = booster_record(on_purchase);
        if !on_purchase.is_empty() {
            let leave = str_list(&["Off you go, <PSN>."]);
            fork = fork.resource(StrList::TYPE, 25048, None, &leave);
        }
        if let Some(price) = pod {
            booster[0x02..0x04].copy_from_slice(&(-20_i16).to_be_bytes());
            booster[0x0E..0x12].copy_from_slice(&price.to_be_bytes());
            let mut hold = booster_record(b"");
            hold[0x02..0x04].copy_from_slice(&25_i16.to_be_bytes());
            hold[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
            fork = fork.resource(Outfit::TYPE, 129, Some(b"Hold"), &hold);
        }
        fork = fork.resource(Outfit::TYPE, 128, Some(b"Booster"), &booster);
    }
    let ship = if shipbuying {
        stellar[0x09] |= 0x08;
        let mut prompts = vec![""; 119];
        prompts.push("Please name your new");
        fork = fork.resource(StrList::TYPE, 2002, None, &str_list(&prompts));
        second_ship(buy_random)
    } else {
        vec![0; Ship::SIZE.expect("fixed")]
    };
    character[0x04..0x06].copy_from_slice(&128_i16.to_be_bytes());
    character[0x06..0x08].copy_from_slice(&128_i16.to_be_bytes());
    for slot in 1..4 {
        let at = 0x06 + 2 * slot;
        character[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
    }
    let mut spin = vec![0; Spin::SIZE.expect("fixed")];
    spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
    with_main_screen(fork)
        .resource(Ship::TYPE, 129, Some(b"Second"), &ship)
        .resource(Ship::TYPE, 128, Some(b"First"), &average)
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
        .resource(ShipAnim::TYPE, 128, None, &anim)
        .resource(ShipAnim::TYPE, 129, None, &anim)
        .resource(RLED, 1000, None, &sheet)
        .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128]))
        .resource(System::TYPE, 129, Some(b"Beta"), &system(300, &[129]))
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar)
        .resource(Stellar::TYPE, 129, Some(b"Beta Prime"), &beta)
        .resource(Spin::TYPE, 1000, None, &spin)
        .resource(Spin::TYPE, 650, None, &spin)
        .resource(Desc::TYPE, ABOUT_TEXT, None, &about_text())
        .resource(Desc::TYPE, MENU_X_TEXT, None, &x_text())
        .resource(PICT, 8518, None, &upper_cap_picture())
}

/// The player info's upper cap's size in [`game_data_fork`]: not the
/// stock 413 x 40, so a layout from the stock sizes shows.
const UPPER_CAP_SIZE: (i16, i16) = (300, 30);

/// A grey `PICT` [`UPPER_CAP_SIZE`]: the player info's upper cap
/// (`PICT` 8518), the only one of its background pictures the fixture
/// data holds.
fn upper_cap_picture() -> Vec<u8> {
    let (width, height) = UPPER_CAP_SIZE;
    let bounds = [0, 0, height, width];
    let count = usize::try_from(width * height).expect("positive");
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &vec![0x4210; count]))
        .end()
        .build()
}

/// `fork` with the main menu's art, so the menu draws the original's
/// screen.
fn with_main_screen(fork: ForkBuilder) -> ForkBuilder {
    main_screen_resources()
        .into_iter()
        .fold(fork, |fork, (ty, id, data)| {
            fork.resource(ty, id, None, &data)
        })
}

/// The game data of `fork`'s resources.
fn loaded(fork: &ForkBuilder) -> Rc<GameData> {
    loaded_bytes(fork.build().bytes)
}

/// The game data of a built fork's `bytes`.
fn loaded_bytes(bytes: Vec<u8>) -> Rc<GameData> {
    let file = OneFile(bytes);
    let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
    Rc::new(data)
}

/// [`outfitting_data`], where the outfitter also sells a license
/// (`oütf` 130, 100 credits, contributing bit 0x2) and `oütf` 131,
/// hidden until bit 0x2 is contributed (`Flags` 0x0100, `Require`
/// 0x2).
fn licensing_data() -> Rc<GameData> {
    let mut license = booster_record(b"");
    license[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
    license[0x1E..0x26].copy_from_slice(&2_u64.to_be_bytes());
    let mut licensed = booster_record(b"");
    licensed[0x0C..0x0E].copy_from_slice(&0x0100_u16.to_be_bytes());
    licensed[0x26..0x2E].copy_from_slice(&2_u64.to_be_bytes());
    loaded(
        &game_data_fork(true, true, false, 100, b"", b"", None)
            .resource(Outfit::TYPE, 130, Some(b"License"), &license)
            .resource(Outfit::TYPE, 131, Some(b"Licensed"), &licensed),
    )
}

/// [`outfitting_data`], where the booster, the outfitter's only row,
/// hides itself once bought: flagged 0x0010 (taken away after
/// purchase) and 0x4000 (hidden unless available), its `Availability`
/// `!b7` and its `OnPurchase` `b7`.
fn self_hiding_data() -> Rc<GameData> {
    let mut built = game_data_fork(true, true, false, 100, b"", b"b7", None).build();
    let booster = built.layout.data_entry(Outfit::TYPE, 128) + 4;
    built.put_u16(booster + 0x0C, 0x4010);
    built.bytes[booster + 0x2E..booster + 0x31].copy_from_slice(b"!b7");
    loaded_bytes(built.bytes)
}

/// The `shän` every fixture ship shares: one 1 x 1 frame of `rlë8`
/// 1000.
fn ship_anim() -> Vec<u8> {
    let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
    anim[0x00..0x02].copy_from_slice(&1000_i16.to_be_bytes());
    anim[0x04..0x06].copy_from_slice(&1_i16.to_be_bytes());
    anim[0x34..0x36].copy_from_slice(&4_i16.to_be_bytes());
    anim
}

/// `shïp` 129, "Second": 800 credits, for sale on a `BuyRandom` of
/// `buy_random`, carrying a booster.
fn second_ship(buy_random: i16) -> Vec<u8> {
    let mut ship = vec![0; Ship::SIZE.expect("fixed")];
    for (at, value) in [
        (0x00, 20_i16),
        (0x04, 300),
        (0x06, 600),
        (0x08, 10),
        (0x0C, 12),
        (0x2E, 1),
        (0x388, buy_random),
    ] {
        ship[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }
    ship[0x30..0x34].copy_from_slice(&800_i32.to_be_bytes());
    for (slot, (id, count)) in [(128_i16, 1_i16), (-1, 0), (-1, 0), (-1, 0)]
        .into_iter()
        .enumerate()
    {
        let at = 0x4E + 2 * slot;
        ship[at..at + 2].copy_from_slice(&id.to_be_bytes());
        ship[at + 8..at + 10].copy_from_slice(&count.to_be_bytes());
        ship[0x370 + 2 * slot..0x372 + 2 * slot].copy_from_slice(&(-1_i16).to_be_bytes());
    }
    ship[0x5CE..0x5D4].copy_from_slice(b"Second");
    ship[0x62E..0x63D].copy_from_slice(b"Second Shipyard");
    ship
}

/// [`shipbuying_data`], where the shipyard also sells `shïp` 130,
/// "Third", [`second_ship`] at 5,000 credits: more than the pilot's
/// cash and any trade-in cover.
fn dear_shipbuying_data() -> Rc<GameData> {
    let mut third = second_ship(100);
    third[0x30..0x34].copy_from_slice(&5000_i32.to_be_bytes());
    third[0x5CE..0x5D4].copy_from_slice(b"Third\0");
    loaded(
        &game_data_fork(true, true, true, 100, b"", b"", None)
            .resource(Ship::TYPE, 130, Some(b"Third"), &third)
            .resource(ShipAnim::TYPE, 130, None, &ship_anim()),
    )
}

/// The booster `oütf` of [`game_data_with`]: for sale every day
/// (`BuyRandom` 100), 500 credits, its `OnPurchase` `on_purchase`.
fn booster_record(on_purchase: &[u8]) -> Vec<u8> {
    let mut booster = vec![0; Outfit::SIZE.expect("fixed")];
    for (at, value) in [
        (0x02, 2_i16),
        (0x04, 1),
        (0x06, 8),
        (0x08, 300),
        (0x0A, 3),
        (0x3F0, 100),
    ] {
        booster[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }
    booster[0x0E..0x12].copy_from_slice(&500_i32.to_be_bytes());
    booster[0x32B..0x332].copy_from_slice(b"Booster");
    booster[0x12D..0x12D + on_purchase.len()].copy_from_slice(on_purchase);
    booster
}

/// A `spöb` at (0, 0) that can be landed on: a planet with a bar.
fn landable() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    bytes
}

/// The About text's `dësc`: ten short lines, the third holding a pilot
/// and a mission tag that the briefing munge would replace, the fourth
/// conditional text that adds " him" for a male pilot.
fn about_text() -> Vec<u8> {
    let mut bytes: Vec<u8> = (0..10)
        .map(|n| match n {
            2 => "line 2 <PN> <DST>".to_owned(),
            3 => "line 3{G \" him\" \"\"}".to_owned(),
            n => format!("line {n}"),
        })
        .collect::<Vec<_>>()
        .join("\r")
        .into_bytes();
    bytes.push(0);
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

/// The main menu's X text's `dësc`: two short lines, the second naming
/// the pilot, the ship type and a mission's destination.
fn x_text() -> Vec<u8> {
    let mut bytes = b"x 0\rx <PN>1 <PST> <DST>\0".to_vec();
    bytes.extend([0xFF; 2]);
    bytes.extend([0; 34]);
    bytes
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

/// A key held down past the OS key-repeat delay: another press, marked
/// as a repeat.
fn held(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: true,
    }
}

fn drawn(screen: &impl Screen) -> DrawList {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list
}

/// Clicks the left button on system `id`'s dot.
fn click_system(screen: &mut AppScreen, id: i16) {
    let map = screen.galaxy_map();
    let position = map
        .model()
        .system(SystemId(id))
        .expect("a system")
        .position();
    let at = map.transform().world_to_screen(position);
    for pressed in [true, false] {
        let input = Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        assert_eq!(screen.input(&input), ScreenAction::None);
    }
}

#[test]
fn the_app_starts_on_the_ship_browser_at_the_first_ship() {
    let screen = start_screen(data());
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
    assert_eq!(screen.galaxy_map().selected(), None);
    assert_eq!(screen.galaxy_map().model().systems().len(), 2);
}

#[test]
fn a_tab_press_switches_screens_and_its_release_does_nothing() {
    use Showing::{GalaxyMap as Map, ShipBrowser as Ships};
    let mut screen = AppScreen::new(data());
    let mut showing = Vec::new();
    for input in [
        key(Key::Tab, true),
        key(Key::Tab, false),
        key(Key::Tab, true),
        key(Key::Tab, false),
        key(Key::Tab, true),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        showing.push(screen.showing());
    }
    assert_eq!(showing, [Map, Map, Ships, Ships, Map]);
}

#[test]
fn holding_tab_switches_once() {
    use Showing::{GalaxyMap as Map, ShipBrowser as Ships};
    let mut screen = AppScreen::new(data());
    let mut showing = Vec::new();
    for input in [
        key(Key::Tab, true),
        held(Key::Tab),
        held(Key::Tab),
        held(Key::Tab),
        key(Key::Tab, false),
        key(Key::Tab, true),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        showing.push(screen.showing());
    }
    assert_eq!(showing, [Map, Map, Map, Map, Map, Ships]);
}

#[test]
fn a_held_tab_reaches_neither_screen() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    let map = screen.galaxy_map();
    let view = *map.view().expect("the viewer");
    let at = map.transform().world_to_screen(
        map.model()
            .system(SystemId(129))
            .expect("a system")
            .position(),
    );
    let left = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };

    // A drag on the map survives a held Tab: no switch, so no cancel.
    screen.input(&left(true));
    screen.input(&held(Key::Tab));
    screen.input(&Input::PointerMoved(Point::new(at.x + 50.0, at.y)));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_ne!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "the drag goes on"
    );
}

#[test]
fn held_keys_reach_the_screen_shown() {
    let mut screen = AppScreen::new(data());
    screen.input(&held(Key::Right));
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
    screen.input(&key(Key::Tab, true));
    let fitted = *screen.galaxy_map().view().expect("the viewer");
    screen.input(&held(Key::Right));
    assert_ne!(
        *screen.galaxy_map().view().expect("the viewer"),
        fitted,
        "the map pans"
    );
}

#[test]
fn keys_reach_only_the_screen_shown() {
    let mut screen = AppScreen::new(data());
    let fitted = *screen.galaxy_map().view().expect("the viewer");
    assert_eq!(screen.input(&key(Key::Right, true)), ScreenAction::None);
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        fitted,
        "the map stays put"
    );

    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Right, true));
    assert_ne!(
        *screen.galaxy_map().view().expect("the viewer"),
        fitted,
        "the map pans"
    );
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
}

#[test]
fn each_screen_keeps_its_state_across_a_round_trip() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Tab, true));
    click_system(&mut screen, 129);
    screen.input(&key(Key::Char('='), true));
    let (view, selected) = (
        *screen.galaxy_map().view().expect("the viewer"),
        screen.galaxy_map().selected(),
    );
    assert_eq!(selected, Some(SystemId(129)));

    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
    screen.input(&key(Key::Tab, true));
    assert_eq!(*screen.galaxy_map().view().expect("the viewer"), view);
    assert_eq!(screen.galaxy_map().selected(), selected);
}

#[test]
fn switching_away_cancels_a_drag_whose_release_the_other_screen_gets() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    let map = screen.galaxy_map();
    let view = *map.view().expect("the viewer");
    let at = map.transform().world_to_screen(
        map.model()
            .system(SystemId(129))
            .expect("a system")
            .position(),
    );
    let left = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };

    screen.input(&left(true));
    screen.input(&key(Key::Tab, true));
    screen.input(&left(false));
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    screen.input(&Input::PointerMoved(Point::new(at.x + 50.0, at.y)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "no drag"
    );
    assert_eq!(screen.galaxy_map().selected(), None);
}

#[test]
fn an_escape_press_quits_from_either_screen() {
    let mut screen = AppScreen::new(data());
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
}

#[test]
fn an_escape_repeat_or_release_does_nothing() {
    let mut screen = AppScreen::new(data());
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
}

/// Opens the map, selects system `id` and enters it with Return.
fn enter(screen: &mut AppScreen, id: i16) {
    screen.input(&key(Key::Tab, true));
    click_system(screen, id);
    assert_eq!(screen.input(&key(Key::Enter, true)), ScreenAction::None);
}

fn camera(screen: &AppScreen) -> Point {
    screen
        .system_view()
        .expect("a system open")
        .camera()
        .center()
}

#[test]
fn return_on_a_selected_system_shows_it() {
    let mut screen = AppScreen::new(data());
    assert_eq!(screen.system_view().map(|v| v.scene().id()), None);
    enter(&mut screen, 128);
    assert_eq!(screen.showing(), Showing::System);
    let view = screen.system_view().expect("a system open");
    assert_eq!(view.scene().id(), SystemId(128));
    assert_eq!(view.scene().stellars().len(), 1);
    assert_eq!(
        screen.navigator().system().map(|v| v.scene().id()),
        Some(SystemId(128))
    );
}

#[test]
fn the_enter_button_shows_the_selected_system() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    click_system(&mut screen, 129);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ENTER_BUTTON.center(),
        });
    }
    assert_eq!(screen.showing(), Showing::System);
}

#[test]
fn escape_goes_back_to_the_map_then_quits() {
    let mut screen = AppScreen::new(data());
    enter(&mut screen, 128);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_eq!(screen.galaxy_map().selected(), Some(SystemId(128)));
    assert!(screen.system_view().is_none());
    // Holding Escape after going back does not quit.
    assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
}

#[test]
fn tab_from_a_system_goes_to_the_ships_and_back_to_it_as_it_was() {
    let mut screen = AppScreen::new(data());
    enter(&mut screen, 128);
    screen.input(&key(Key::Right, true));
    screen.tick(Duration::from_millis(250));
    screen.input(&key(Key::Right, false));
    let moved = camera(&screen);
    assert_eq!(moved, Point::new(240.0, 0.0));

    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    screen.input(&key(Key::Tab, true));
    assert_eq!(screen.showing(), Showing::System);
    assert_eq!(camera(&screen), moved);
}

#[test]
fn a_tab_switch_lets_go_of_the_keys_held_in_a_system() {
    let mut screen = AppScreen::new(data());
    enter(&mut screen, 128);
    screen.input(&key(Key::Down, true));
    screen.input(&key(Key::Tab, true));
    // Down's release goes to the ship browser.
    screen.input(&key(Key::Down, false));
    screen.input(&key(Key::Tab, true));
    screen.tick(Duration::from_millis(250));
    assert_eq!(camera(&screen), Point::new(0.0, 0.0), "no drift");
}

#[test]
fn releasing_the_keys_reaches_the_screen_shown() {
    let mut screen = AppScreen::new(data());
    enter(&mut screen, 128);
    screen.input(&key(Key::Up, true));
    screen.release_keys();
    screen.tick(Duration::from_millis(250));
    assert_eq!(camera(&screen), Point::new(0.0, 0.0));
}

#[test]
fn cancelling_the_pointer_reaches_the_screen_shown() {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    let view = *screen.galaxy_map().view().expect("the viewer");
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: Point::new(100.0, 100.0),
    });
    screen.cancel_pointer();
    screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "no drag"
    );
}

#[test]
fn the_hint_is_drawn_over_the_system() {
    let data = data();
    let mut screen = AppScreen::new(Rc::clone(&data));
    enter(&mut screen, 128);
    let mut expected = drawn(screen.system_view().expect("open"));
    expected.push(hint());
    assert_eq!(drawn(&screen), expected);
}

#[test]
fn only_the_screen_shown_ticks() {
    let mut screen = AppScreen::new(data());
    let tick = Duration::from_secs(1) / 30;
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), Some(1));
    screen.input(&key(Key::Tab, true));
    screen.tick(tick);
    assert_eq!(
        screen.ship_browser().frame(),
        Some(1),
        "paused while hidden"
    );
    screen.input(&key(Key::Tab, true));
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), Some(2));

    // Nor while a system is shown.
    enter(&mut screen, 128);
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), Some(2));
    screen.input(&key(Key::Tab, true));
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), Some(3));
}

/// The hint line of a router with no dialogs.
fn hint() -> DrawCommand {
    DrawCommand::Text {
        text: HINT_WITHOUT_DIALOGS.to_owned(),
        font: Font::Geneva,
        origin: HINT_AT,
        size: HINT_SIZE,
        wrap_width: None,
        color: HINT_COLOR,
    }
}

#[test]
fn it_draws_the_screen_shown_then_the_hint() {
    let data = data();
    let mut screen = AppScreen::new(Rc::clone(&data));
    let mut ships = ShipBrowser::new(Rc::clone(&data));
    let mut expected = drawn(&ships);
    expected.push(hint());
    assert_eq!(drawn(&screen), expected);

    screen.input(&key(Key::Tab, true));
    let mut expected = drawn(&GalaxyMap::new(&data));
    expected.push(hint());
    assert_eq!(drawn(&screen), expected);

    screen.input(&key(Key::Tab, true));
    ships.tick(Duration::from_millis(100));
    screen.tick(Duration::from_millis(100));
    let mut expected = drawn(&ships);
    expected.push(hint());
    assert_eq!(drawn(&screen), expected);
    assert_eq!(
        HINT,
        "Tab: ships / galaxy map   F: fly   I: about   P: preferences"
    );
    assert_eq!((HINT_AT, HINT_SIZE), (Point::new(16.0, 8.0), 14.0));
}

// Flight.

/// Enters flight with an F press and checks it shows.
fn fly(screen: &mut AppScreen) {
    assert_eq!(screen.input(&key(Key::Char('f'), true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::Flight);
}

fn flight(screen: &AppScreen) -> &FlightView<Rc<GameData>> {
    screen.flight_view().expect("flight entered")
}

fn ship(screen: &AppScreen) -> nova_sim::ShipState {
    *flight(screen).session().expect("flying").player()
}

const TICK: Duration = nova_sim::TICK;

const MAP: Key = Key::Char('m');

// The About dialog.

/// Dialog templates by ID: "Desc Dialog" (3003), 200 x 100 at (0, 0),
/// with Done (1) and a 100 x 36 text box (3): three lines of the About
/// text show; the note alert (3000), 200 x 100, with OK (1) and a
/// text box (3); "Spaceport" (1000), [`spaceport_template`]; and "Text
/// Input" (3001), [`text_input_template`].
struct Dialogs;

impl DialogResources for Dialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        let item = |x, y, w, h, kind| ItemTemplate {
            bounds: Bounds::at(Point::new(x, y), w, h),
            enabled: true,
            kind,
        };
        if id == SPACEPORT_DIALOG {
            return Ok(spaceport_template());
        }
        if id == PREFS_DIALOG {
            return Ok(prefs_template());
        }
        if id == NEW_PILOT_DIALOG {
            return Ok(new_pilot_template());
        }
        if id == TEXT_INPUT_DIALOG {
            return Ok(text_input_template());
        }
        if id == PLAYER_INFO_DIALOG {
            return Ok(player_info_template());
        }
        if id == YES_NO_DIALOG {
            return Ok(yes_no_template());
        }
        if id == NOTE_DIALOG {
            return Ok(DialogTemplate {
                bounds: Bounds::at(Point::new(0.0, 0.0), 200.0, 100.0),
                placement: Placement::Fixed,
                items: vec![
                    item(100.0, 70.0, 70.0, 20.0, ItemSpec::User),
                    item(10.0, 150.0, 20.0, 20.0, ItemSpec::Picture(1431)),
                    item(10.0, 10.0, 180.0, 50.0, ItemSpec::User),
                ],
            });
        }
        if id != DESC_DIALOG && id != PICTURE_DESC_DIALOG {
            return Err(format!("no DLOG {id}"));
        }
        // The picture variant is wider.
        let width = if id == DESC_DIALOG { 200.0 } else { 300.0 };
        Ok(DialogTemplate {
            bounds: Bounds::at(Point::new(0.0, 0.0), width, 100.0),
            placement: Placement::Fixed,
            items: vec![
                item(100.0, 70.0, 99.0, 25.0, ItemSpec::User),
                item(10.0, 150.0, 20.0, 20.0, ItemSpec::Picture(1431)),
                item(10.0, 10.0, 100.0, 36.0, ItemSpec::User),
            ],
        })
    }
}

/// "Player Info" (1017), as stock lays it out: 413 x 227, centred,
/// with Done (1), the tabs (2-5), the content box (6) and Jettison
/// (7).
fn player_info_template() -> DialogTemplate {
    let item = |x, y, w, h| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 413.0, 227.0),
        placement: Placement::Center,
        items: vec![
            item(293.0, 195.0, 99.0, 25.0),
            item(7.0, 8.0, 99.0, 25.0),
            item(107.0, 8.0, 99.0, 25.0),
            item(207.0, 8.0, 99.0, 25.0),
            item(307.0, 8.0, 99.0, 25.0),
            item(4.0, 40.0, 405.0, 141.0),
            item(60.0, 195.0, 150.0, 25.0),
        ],
    }
}

/// `YesNo` (3002), as stock lays it out: 355 x 109, centred, with OK
/// (1), the question (3) and Cancel (5).
fn yes_no_template() -> DialogTemplate {
    let item = |x, y, w, h, kind| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind,
    };
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 355.0, 109.0),
        placement: Placement::Center,
        items: vec![
            item(263.0, 75.0, 70.0, 20.0, ItemSpec::Button("OK".into())),
            item(277.0, 131.0, 68.0, 30.0, ItemSpec::Picture(129)),
            item(52.0, 5.0, 295.0, 50.0, ItemSpec::StaticText(String::new())),
            item(7.0, 5.0, 32.0, 32.0, ItemSpec::Picture(130)),
            item(181.0, 75.0, 70.0, 20.0, ItemSpec::Button("Cancel".into())),
        ],
    }
}

/// "new prefs dialog" (4003), smaller: 300 x 260 at (0, 0), with OK
/// (1), the Scaling list's row (2), the effects volume (4 to 7), the
/// Music (8) and Sound (20) check boxes, Key Settings (16), a greyed
/// check box (9) and the Hyperspace Effects check box (21).
fn prefs_template() -> DialogTemplate {
    let item = |x, y, w, h, kind| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind,
    };
    let mut items: Vec<ItemTemplate> = (0..21)
        .map(|_| item(0.0, 300.0, 10.0, 10.0, ItemSpec::User))
        .collect();
    items[0] = item(200.0, 230.0, 70.0, 20.0, ItemSpec::Button("OK".into()));
    items[1] = item(150.0, 40.0, 100.0, 18.0, ItemSpec::User);
    items[3] = item(
        150.0,
        140.0,
        100.0,
        16.0,
        ItemSpec::StaticText("Sound Volume:".into()),
    );
    items[4] = item(
        170.0,
        160.0,
        100.0,
        16.0,
        ItemSpec::StaticText("Static Text".into()),
    );
    items[5] = item(150.0, 167.0, 11.0, 9.0, ItemSpec::Picture(135));
    items[6] = item(150.0, 158.0, 11.0, 9.0, ItemSpec::Picture(134));
    items[7] = item(
        150.0,
        20.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("Intro Music".into()),
    );
    items[8] = item(
        10.0,
        20.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("QuickTime Movies".into()),
    );
    items[9] = item(
        10.0,
        40.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("Smoke Trails".into()),
    );
    items[14] = item(
        10.0,
        60.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("Weapon Effects".into()),
    );
    items[15] = item(
        20.0,
        230.0,
        120.0,
        20.0,
        ItemSpec::Button("Key Settings".into()),
    );
    items[19] = item(
        150.0,
        60.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("Ambient Sounds".into()),
    );
    items[20] = item(
        150.0,
        80.0,
        100.0,
        18.0,
        ItemSpec::CheckBox("Hyperspace Effects".into()),
    );
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 300.0, 260.0),
        placement: Placement::Fixed,
        items,
    }
}

/// No dialogs at all.
struct NoDialogs;

impl DialogResources for NoDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        Err(format!("no DLOG {id}"))
    }
}

fn with_dialogs(data: Rc<GameData>) -> AppScreen {
    AppScreen::new(data).with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
}

fn open_about(screen: &mut AppScreen) {
    assert_eq!(screen.input(&key(Key::Char('i'), true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::About);
}

fn about(screen: &AppScreen) -> &DescDialog {
    screen.about().expect("the About dialog is open")
}

// Landing.

/// "Spaceport", smaller: 400 x 300 at (0, 0), its twelve items in a
/// row of 30 x 20 boxes, so each service and Leave (12) has a place.
fn spaceport_template() -> DialogTemplate {
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 400.0, 300.0),
        placement: Placement::Fixed,
        items: (0..12_u8)
            .map(|n| ItemTemplate {
                bounds: Bounds::at(Point::new(f32::from(n) * 32.0, 250.0), 30.0, 20.0),
                enabled: true,
                kind: ItemSpec::User,
            })
            .collect(),
    }
}

const LAND: Key = Key::Char('l');

/// Presses L twice in flight: the first requests clearance, a tick
/// lets the dock approach clear it, and the second lands.
fn land_twice(screen: &mut AppScreen) {
    for pressed in [true, false] {
        assert_eq!(screen.input(&key(LAND, pressed)), ScreenAction::None);
    }
    screen.tick(Duration::from_millis(50));
    assert_eq!(screen.input(&key(LAND, true)), ScreenAction::None);
}

/// Enters flight and lands with two L presses.
fn land(screen: &mut AppScreen) {
    fly(screen);
    land_twice(screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
}

fn spaceport(screen: &AppScreen) -> &SpaceportView {
    screen.spaceport_view().expect("landed")
}

// Sounds.

const DOWN: Sound = Sound::Ui(UiSound::ButtonDown);

const UP: Sound = Sound::Ui(UiSound::ButtonUp);

fn press_and_release(screen: &mut AppScreen, at: Point) -> Vec<Vec<Sound>> {
    [true, false]
        .into_iter()
        .map(|pressed| {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            screen.take_sounds()
        })
        .collect()
}

fn halted() -> Sound {
    Sound::Ui(UiSound::PortAmbientHalted(nova_sim::SoundId(AMBIENT)))
}

// The Preferences dialog.

const PREFS: Key = Key::Char('p');

/// Presses Command+comma, letting go of Command once it is done.
fn command_comma(screen: &mut AppScreen) {
    screen.input(&key(Key::Command, true));
    assert_eq!(
        screen.input(&key(PREFERENCES_KEY, true)),
        ScreenAction::None
    );
    screen.input(&key(PREFERENCES_KEY, false));
    screen.input(&key(Key::Command, false));
}

/// Opens the Preferences as the player does on the side shown: P on
/// the menu, the ship browser and the galaxy viewer, Command+comma in
/// flight and the spaceport.
fn open_prefs(screen: &mut AppScreen) {
    if matches!(
        screen.showing(),
        Showing::Flight | Showing::FlightMap | Showing::Spaceport
    ) {
        command_comma(screen);
    } else {
        assert_eq!(screen.input(&key(PREFS, true)), ScreenAction::None);
        // The menu's P opens them once its click has played.
        screen.keyed_click_ended();
    }
    assert_eq!(screen.showing(), Showing::Preferences);
    assert_eq!(screen.now_showing(), Some(Showing::Preferences));
}

fn prefs_dialog(screen: &AppScreen) -> &PrefsDialog {
    screen
        .preferences()
        .expect("the Preferences dialog is open")
}

/// Clicks the left button at `at`.
fn click_at(screen: &mut AppScreen, at: Point) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

// The player info screen.

/// Steps flight through the take-off grace, which refuses P for
/// [`nova_sim::TAKEOFF_GRACE_TICKS`] after a take-off.
fn past_the_takeoff_grace(screen: &mut AppScreen) {
    for _ in 0..nova_sim::TAKEOFF_GRACE_TICKS {
        screen.tick(TICK);
    }
}

// The main menu and pilots.

fn keeper(store: &MemoryPilots) -> PilotKeeper<Box<dyn PilotStore>> {
    PilotKeeper::new(Box::new(store.clone()))
}

/// The router with the main menu over `store`, and no dialogs, past its
/// start-up appear ([`appeared`]).
pub(crate) fn menu(store: &MemoryPilots) -> AppScreen {
    appeared(AppScreen::new(data()).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics)))
}

/// Escape from flight, back to the main menu, past the return's appear
/// ([`returned`]).
pub(crate) fn escape_to_menu(screen: &mut AppScreen) {
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    returned(screen);
}

/// `screen` once the main screen it returned to has appeared: the
/// second's black ticked out, then a click off every button, which
/// finishes the slides silently.
pub(crate) fn returned(screen: &mut AppScreen) {
    screen.tick(nova_view::fade::DEATH_FADE_FOR);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(1.0, 1.0),
        });
    }
}

/// Builds on past the start-up appear, for chaining ([`appeared`]).
pub(crate) trait Appeared {
    /// The router once its main screen has appeared.
    #[must_use]
    fn appeared(self) -> Self;
}

impl Appeared for AppScreen {
    fn appeared(self) -> Self {
        appeared(self)
    }
}

/// `screen` once its main screen has appeared: the second's fade from
/// black ticked out, then a click off every button, which finishes the
/// slides silently, and the sounds taken.
pub(crate) fn appeared(mut screen: AppScreen) -> AppScreen {
    returned(&mut screen);
    screen.take_sounds();
    screen
}

fn choose(screen: &mut AppScreen, choice: MenuChoice) -> ScreenAction {
    let at = screen.main_menu().expect("a main menu").press_point(choice);
    let mut action = ScreenAction::None;
    for pressed in [true, false] {
        action = screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    action
}

fn type_text(screen: &mut AppScreen, text: &str) {
    for c in text.chars() {
        screen.input(&Input::Text(c));
    }
}

/// Creates a pilot named `name` from the main menu and enters play with
/// it: [`create_held`], then a click on Enter Ship once the menu holds it
/// ([`enter_ship`]).
pub(crate) fn create(screen: &mut AppScreen, name: &str) {
    create_held(screen, name);
    if screen.showing() == Showing::MainMenu {
        enter_ship(screen);
    }
}

/// Creates a pilot named `name` from the main menu, which then holds it:
/// New Pilot (answering yes to `STR#` 140 #8 when a pilot is held already
/// and the router can ask), the name, Return.
pub(crate) fn create_held(screen: &mut AppScreen, name: &str) {
    assert_eq!(choose(screen, MenuChoice::NewPilot), ScreenAction::None);
    if screen.showing() == Showing::Question {
        screen.input(&key(Key::Enter, true));
    }
    assert_eq!(screen.showing(), Showing::NewPilot);
    type_text(screen, name);
    screen.input(&key(Key::Enter, true));
    christen(screen);
}

/// Confirms the new pilot's ship christening with its default, Return,
/// when it is open.
pub(crate) fn christen(screen: &mut AppScreen) {
    if screen.showing() == Showing::Christening {
        screen.input(&key(Key::Enter, true));
    }
}

/// Opens the first saved pilot from the main menu's list, which then
/// holds it: Open Pilot, Return.
pub(crate) fn open_held(screen: &mut AppScreen) {
    choose(screen, MenuChoice::OpenPilot);
    assert_eq!(screen.showing(), Showing::OpenPilot);
    screen.input(&key(Key::Enter, true));
}

/// Enters play with the pilot the main menu holds: a click on Enter Ship,
/// then the entry stepped through ([`through_entry`]).
pub(crate) fn enter_ship(screen: &mut AppScreen) {
    choose(screen, MenuChoice::EnterShip);
    through_entry(screen);
    assert_eq!(screen.showing(), Showing::Flight);
}

/// Steps play's entry through: ticks of a tenth of a second until the
/// router no longer enters play, the skip key tapped while the intro's
/// pictures show and Return while its text does. Bounded, so an entry
/// that never ends fails.
pub(crate) fn through_entry(screen: &mut AppScreen) {
    for _ in 0..1000 {
        if !screen.entering() {
            return;
        }
        let tap = match screen.showing() {
            Showing::Intro => Some(nova_view::menu::intro::SKIP_KEY),
            Showing::IntroText => Some(Key::Enter),
            _ => None,
        };
        if let Some(tapped) = tap {
            screen.input(&key(tapped, true));
            screen.input(&key(tapped, false));
        }
        screen.tick(std::time::Duration::from_millis(100));
    }
    panic!("play is still being entered");
}

fn pilot(screen: &AppScreen) -> &nova_sim::Pilot {
    flight(screen).pilot().expect("flying")
}

/// `pilot` as its last save has it, from before entering play marked
/// the intro seen.
fn unseen(pilot: &nova_sim::Pilot) -> nova_sim::Pilot {
    nova_sim::Pilot {
        seen_intro: false,
        ..pilot.clone()
    }
}

pub(crate) fn saved(store: &MemoryPilots, key: &str) -> nova_sim::Pilot {
    nova_sim::save::decode(&store.text(key).expect("saved")).expect("a pilot")
}

// O's guard.

/// Creates a pilot named `name` from the main menu with Strict Play
/// ticked and enters play with it: New Pilot, a click on the Strict Play
/// box, the name, Return, then Enter Ship. The router needs the
/// interface file's dialogs.
fn create_strict(screen: &mut AppScreen, name: &str) {
    choose(screen, MenuChoice::NewPilot);
    if screen.showing() == Showing::Question {
        screen.input(&key(Key::Enter, true));
    }
    let at = screen
        .new_pilot()
        .expect("open")
        .dialog()
        .item_bounds(nova_view::ui::new_pilot::STRICT_PLAY_ITEM)
        .expect("the Strict Play box")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    type_text(screen, name);
    screen.input(&key(Key::Enter, true));
    christen(screen);
    if screen.showing() == Showing::MainMenu {
        enter_ship(screen);
    }
}

// The note on a load that fell back.

/// "Create a new pilot:", smaller: 326 x 213 at (0, 0), with OK (1),
/// Cancel (2), Strict Play (4) and the Full Name and Nickname fields (8,
/// 9); the rest parked.
fn new_pilot_template() -> DialogTemplate {
    let item = |x, y, w, h, kind| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind,
    };
    let mut items: Vec<ItemTemplate> = (0..14)
        .map(|_| item(0.0, 300.0, 10.0, 10.0, ItemSpec::User))
        .collect();
    items[0] = item(238.0, 183.0, 70.0, 20.0, ItemSpec::Button("OK".into()));
    items[1] = item(156.0, 183.0, 70.0, 20.0, ItemSpec::Button("Cancel".into()));
    items[3] = item(
        59.0,
        122.0,
        95.0,
        17.0,
        ItemSpec::CheckBox("Strict Play".into()),
    );
    items[7] = item(
        140.0,
        30.0,
        170.0,
        16.0,
        ItemSpec::EditText("Edit Text".into()),
    );
    items[8] = item(
        140.0,
        53.0,
        170.0,
        16.0,
        ItemSpec::EditText("Edit Text".into()),
    );
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 326.0, 213.0),
        placement: Placement::Fixed,
        items,
    }
}

// The Trade Center.

/// "Trade", smaller: 400 x 300 at (0, 0), with Done (1), eight rows (4
/// to 11), Buy (13) and Sell (14).
fn trade_template() -> DialogTemplate {
    let item = |x: f32, y: f32| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), 30.0, 12.0),
        enabled: true,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..15).map(|_| item(0.0, 400.0)).collect();
    items[0] = item(300.0, 250.0);
    for row in 0..8_u8 {
        items[3 + usize::from(row)] = item(10.0, 20.0 + 12.0 * f32::from(row));
    }
    items[12] = item(100.0, 250.0);
    items[13] = item(200.0, 250.0);
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 400.0, 300.0),
        placement: Placement::Fixed,
        items,
    }
}

/// [`Dialogs`], with "Trade" too.
struct TradeDialogs;

impl DialogResources for TradeDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == TRADE_DIALOG {
            return Ok(trade_template());
        }
        Dialogs.dialog_template(id)
    }
}

/// The router over [`trading_data`] with the trade dialog, keeping
/// pilots in `store`, flying a new pilot named Ada, landed.
fn landed_trader(store: &MemoryPilots) -> AppScreen {
    landed_trader_by(store, RuleSource::Engine)
}

/// [`landed_trader`], Option on Buy or Sell doing as `trade_count`
/// says.
fn landed_trader_by(store: &MemoryPilots, trade_count: RuleSource) -> AppScreen {
    let mut screen = AppScreen::new(trading_data())
        .with_rules(Rulebook::default().with_override(RuleKey::TradeCount, trade_count))
        .with_dialogs(Rc::new(TradeDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

/// Clicks spaceport item `number`.
fn click_port_item(screen: &mut AppScreen, number: usize) {
    let at = spaceport(screen)
        .dialog()
        .expect("laid out")
        .item_bounds(number)
        .expect("an item")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

const FOOD: Good = Good::Commodity(0);

// The Outfitter.

/// "Outfit", smaller: 400 x 300 at (0, 0), with Done (1), Sell (4),
/// the grid (5), the description (6), Buy (7), the picture (8) and the
/// info box (9).
fn outfit_template() -> DialogTemplate {
    let item = |x: f32, y: f32, w: f32, h: f32| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..11).map(|_| item(0.0, 400.0, 1.0, 1.0)).collect();
    items[0] = item(300.0, 250.0, 30.0, 12.0);
    items[3] = item(200.0, 250.0, 30.0, 12.0);
    items[4] = item(0.0, 0.0, 200.0, 200.0);
    items[5] = item(200.0, 0.0, 100.0, 100.0);
    items[6] = item(100.0, 250.0, 30.0, 12.0);
    items[7] = item(300.0, 0.0, 100.0, 100.0);
    items[8] = item(300.0, 100.0, 100.0, 100.0);
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 400.0, 300.0),
        placement: Placement::Fixed,
        items,
    }
}

/// [`Dialogs`], with "Outfit" too.
struct OutfitDialogs;

impl DialogResources for OutfitDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == OUTFIT_DIALOG {
            return Ok(outfit_template());
        }
        Dialogs.dialog_template(id)
    }
}

/// [`landed_outfitter`], over `data`.
fn landed_outfitter_over(store: &MemoryPilots, data: Rc<GameData>) -> AppScreen {
    let mut screen = AppScreen::new(data)
        .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

/// The trade and outfit dialogs.
struct ShopDialogs;

impl DialogResources for ShopDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        match id {
            TRADE_DIALOG => Ok(trade_template()),
            OUTFIT_DIALOG => Ok(outfit_template()),
            _ => Dialogs.dialog_template(id),
        }
    }
}

// The resets past the holds: the player info opening and the hire
// screen closing.

/// The shop dialogs, the offer, the bar and its hire screen.
struct PastTheHoldsDialogs;

impl DialogResources for PastTheHoldsDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        match id {
            OFFER_DIALOG => Ok(offer_template()),
            BAR_DIALOG => Ok(bar_template()),
            SHIPYARD_DIALOG => Ok(shipyard_template()),
            _ => ShopDialogs.dialog_template(id),
        }
    }
}

/// "Bar", smaller: 263 x 185 at (0, 0), with Leave (1), Hire Escort
/// (5) and the text (7).
fn bar_template() -> DialogTemplate {
    let item = |x: f32, y: f32, w: f32, h: f32| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: false,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..10).map(|_| item(0.0, 300.0, 1.0, 1.0)).collect();
    items[0] = item(156.0, 154.0, 99.0, 26.0);
    items[4] = item(6.0, 125.0, 146.0, 26.0);
    items[6] = item(16.0, 10.0, 230.0, 106.0);
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 263.0, 185.0),
        placement: Placement::Fixed,
        items,
    }
}

/// [`Dialogs`], with "Trade" and "Outfit" too.
struct EveryDialog;

impl DialogResources for EveryDialog {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == TRADE_DIALOG {
            return Ok(trade_template());
        }
        OutfitDialogs.dialog_template(id)
    }
}

// The Shipyard.

/// "Shipyard", smaller: 400 x 300 at (0, 0), with Done (1), the grid
/// (5), the description (6), Buy Ship (7), the picture (8), the info
/// box (9) and Info (10).
fn shipyard_template() -> DialogTemplate {
    let item = |x: f32, y: f32, w: f32, h: f32| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..13).map(|_| item(0.0, 400.0, 1.0, 1.0)).collect();
    items[0] = item(300.0, 250.0, 30.0, 12.0);
    items[4] = item(0.0, 0.0, 200.0, 200.0);
    items[5] = item(200.0, 0.0, 100.0, 100.0);
    items[6] = item(100.0, 250.0, 30.0, 12.0);
    items[7] = item(300.0, 0.0, 100.0, 100.0);
    items[8] = item(300.0, 100.0, 100.0, 100.0);
    items[9] = item(200.0, 250.0, 30.0, 12.0);
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 400.0, 300.0),
        placement: Placement::Fixed,
        items,
    }
}

/// "Text Input", as stock lays it out: 360 x 138, centred, with OK
/// (1), the prompt (3), the field (5) and Cancel (6).
fn text_input_template() -> DialogTemplate {
    let item = |x: f32, y: f32, w: f32, h: f32, kind| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind,
    };
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 360.0, 138.0),
        placement: Placement::Center,
        items: vec![
            item(252.0, 106.0, 70.0, 20.0, ItemSpec::Button("OK".into())),
            item(7.0, 147.0, 32.0, 32.0, ItemSpec::Picture(129)),
            item(52.0, 5.0, 295.0, 50.0, ItemSpec::StaticText(String::new())),
            item(7.0, 5.0, 32.0, 32.0, ItemSpec::Picture(130)),
            item(91.0, 64.0, 200.0, 16.0, ItemSpec::EditText(String::new())),
            item(170.0, 106.0, 70.0, 20.0, ItemSpec::Button("Cancel".into())),
        ],
    }
}

/// [`EveryDialog`], with "Shipyard", "Shipyard Info" and "Text Input"
/// too.
struct ShipyardDialogs;

impl DialogResources for ShipyardDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == SHIPYARD_DIALOG || id == SHIP_INFO_DIALOG {
            return Ok(shipyard_template());
        }
        if id == TEXT_INPUT_DIALOG {
            return Ok(text_input_template());
        }
        EveryDialog.dialog_template(id)
    }
}

/// The router over [`shipbuying_data`] with every dialog, keeping
/// pilots in `store`, flying a new pilot named Ada, landed.
fn landed_shipyard(store: &MemoryPilots) -> AppScreen {
    landed_shipyard_of(shipbuying_data(), store)
}

/// Selects the open shipyard's first cell, as Right picks it from none:
/// by the engine the shipyard opens with nothing selected
/// (`_DoShipyardDialog` @0x5e6d8-0x5e705, `_ShipyardFilter`
/// @0x60734-0x607b7).
fn select_first_ship(screen: &mut AppScreen) {
    screen.input(&key(Key::Right, true));
    screen.input(&key(Key::Right, false));
}

/// [`landed_shipyard`], on `data`.
fn landed_shipyard_of(data: Rc<GameData>, store: &MemoryPilots) -> AppScreen {
    let mut screen = AppScreen::new(data)
        .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

// Boarding.

/// Every NPC idles.
#[derive(Debug)]
struct Idle;

impl Behaviour for Idle {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        Goal::Idle
    }
}

// Hailing.

/// Ticks once, targets the trader with Tab and hails it with Y.
fn hail(screen: &mut AppScreen) {
    screen.tick(TICK);
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    // Tab's beep, so a test hears only what follows.
    screen.take_sounds();
    screen.input(&key(Key::Char('y'), true));
    screen.input(&key(Key::Char('y'), false));
}

fn cash(screen: &AppScreen) -> i64 {
    flight(screen).pilot().expect("a pilot").cash()
}

// Pilot edits.

/// Makes `edit` through the router's pilot desk, as the developer
/// tools do, then lets the router catch up.
fn edit(screen: &mut AppScreen, edit: PilotEdit) {
    screen
        .pilot_desk()
        .expect("flying")
        .edit(edit)
        .expect("made");
    screen.after_pilot_edit();
}

// Mission offers at every venue.

/// A `dësc` of `text`.
fn desc_of(text: &str) -> Vec<u8> {
    pictured_desc_of(text, -1)
}

/// A `dësc` of `text` naming the movie `file`, of `Flags` `flags`.
fn filmed_desc_of(text: &str, file: &str, flags: u16) -> Vec<u8> {
    let mut desc = desc_of(text);
    let at = text.len() + 3;
    desc[at..at + file.len()].copy_from_slice(file.as_bytes());
    desc[at + 32..at + 34].copy_from_slice(&flags.to_be_bytes());
    desc
}

/// A `dësc` of `text` whose `Graphic` is `graphic`.
fn pictured_desc_of(text: &str, graphic: i16) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(graphic.to_be_bytes());
    bytes.extend([0; 34]);
    bytes
}

/// A refusable `mïsn` offered at Alpha Prime's venue `loc` on every
/// visit, going nowhere, with no text but its offer's, paying -50,100
/// (below -50,000, so its accept takes 100 credits, and the cash shown
/// changes).
fn venue_mission(loc: i16) -> Vec<u8> {
    venue_mission_at(loc, 128)
}

/// [`venue_mission`], available at stellar `stellar`.
fn venue_mission_at(loc: i16, stellar: i16) -> Vec<u8> {
    let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
    let put = |bytes: &mut Vec<u8>, at: usize, values: &[i16]| {
        for (i, value) in values.iter().enumerate() {
            bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
        }
    };
    put(
        &mut bytes,
        0x00,
        &[stellar, 0, loc, 0, 0, 100, -1, -1, -1, -1],
    );
    bytes[0x1C..0x20].copy_from_slice(&(-50_100_i32).to_be_bytes());
    put(&mut bytes, 0x20, &[0, 0, -1, -1, 0, -1, 0, -1, 0]);
    put(&mut bytes, 0x32, &[-1, -1, -1, -1, -1, -1, -1, 0, 1, -1]);
    put(&mut bytes, 0x58, &[-1]);
    bytes
}

/// [`shipbuying_data`] with missions `ids` at venue `loc`, each
/// refusable, paying -50,100 so an accept takes 100 credits, with
/// the offer text "Job `id`." (`dësc` `id` + 3872).
fn venue_data_of(loc: i16, ids: &[i16]) -> Rc<GameData> {
    let fork = ids.iter().fold(
        game_data_fork(true, true, true, 100, b"", b"", None),
        |fork, &id| {
            fork.resource(Mission::TYPE, id, Some(b"Venue job"), &venue_mission(loc))
                .resource(
                    Desc::TYPE,
                    id + 3872,
                    Some(b"Offer"),
                    &desc_of(&format!("Job {id}.")),
                )
        },
    );
    loaded(&fork)
}

/// [`shipbuying_data`] with one mission, 130, at venue `loc`: see
/// [`venue_data_of`]. At the outfitter its pay below -50,000 makes
/// the accept change the cash the outfitter shows.
fn venue_data(loc: i16) -> Rc<GameData> {
    venue_data_of(loc, &[130])
}

/// The offer dialog's shape, smaller: 300 x 200, fixed at (0, 0), with
/// accept (1), refuse (2), the text (3) and the one button (6).
fn offer_template() -> DialogTemplate {
    let item = |x: f32, y: f32, w: f32, h: f32| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: false,
        kind: ItemSpec::User,
    };
    let mut items: Vec<ItemTemplate> = (0..6).map(|_| item(0.0, 300.0, 1.0, 1.0)).collect();
    items[0] = item(200.0, 170.0, 90.0, 25.0);
    items[1] = item(100.0, 170.0, 90.0, 25.0);
    items[2] = item(10.0, 10.0, 280.0, 150.0);
    items[5] = item(150.0, 170.0, 90.0, 25.0);
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 300.0, 200.0),
        placement: Placement::Fixed,
        items,
    }
}

/// [`ShipyardDialogs`], with the offer dialog too.
struct VenueDialogs;

impl DialogResources for VenueDialogs {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == OFFER_DIALOG {
            return Ok(offer_template());
        }
        ShipyardDialogs.dialog_template(id)
    }
}

/// The router over `data` with every dialog, keeping pilots in
/// `store`, flying a new pilot named Ada, landed.
fn landed_at_venues(store: &MemoryPilots, data: Rc<GameData>) -> AppScreen {
    let mut screen = AppScreen::new(data)
        .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared();
    create(&mut screen, "Ada");
    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    screen
}

fn offered_mission(dialog: Option<&OfferDialog>) -> Option<MissionId> {
    dialog.map(|dialog| dialog.prompt().mission)
}

fn active(screen: &AppScreen) -> Vec<MissionId> {
    pilot(screen)
        .missions()
        .iter()
        .map(|(_, active)| active.mission)
        .collect()
}

/// A movie player logging each start and stop, whose movies end on
/// their second advance.
#[derive(Default)]
struct Reel {
    log: std::cell::RefCell<Vec<String>>,
    advanced: std::cell::Cell<u32>,
}

impl MoviePlayer for Reel {
    fn start(&self, file: &str) -> Option<(u32, u32)> {
        self.log.borrow_mut().push(format!("start {file}"));
        self.advanced.set(0);
        Some((160, 120))
    }

    fn advance(&self, _dt: Duration) -> bool {
        self.advanced.set(self.advanced.get() + 1);
        self.advanced.get() < 2
    }

    fn stop(&self) {
        self.log.borrow_mut().push("stop".to_owned());
    }

    fn draw_frame(&self, _rect: Bounds, _list: &mut DrawList) {}
}

fn reeled(reel: &Reel) -> Vec<String> {
    reel.log.borrow().clone()
}

// The status bar in port.

/// Writes `values` big-endian into `bytes` from `at`.
fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (n, value) in values.iter().enumerate() {
        bytes[at + 2 * n..at + 2 * n + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// Writes `values` big-endian into `bytes` from `at`.
fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (n, value) in values.iter().enumerate() {
        bytes[at + 4 * n..at + 4 * n + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// The fuel gauge's full colour in [`bar_interface`].
const BAR_FUEL: u32 = 0x00FF_FF00;

/// `ïntf` 128, as `tests/it/flight.rs` has it: the radar at (8, 8) to
/// (184, 184), green and dim green, the shield, armour and fuel
/// gauges 149 wide from x 35, over `PICT` 700.
fn bar_interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[0x0000_FF00, 0x0000_8000]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[0x0000_00FF]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[0x00FF_0000]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[BAR_FUEL, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    put_i16s(&mut bytes, 0x48, &[300, 8, 315, 184]);
    put_i16s(&mut bytes, 0x50, &[330, 8, 442, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12]);
    put_i16s(&mut bytes, 0xA4, &[700]);
    bytes
}

/// A grey 194 x 16 `PICT`: the status bar's background, cut short.
fn bar_picture() -> Vec<u8> {
    let bounds = [0, 0, 16, 194];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &[0x4210; 194 * 16]))
        .end()
        .build()
}

/// `fork` with the status bar: [`bar_interface`] over
/// [`bar_picture`].
fn barred(fork: ForkBuilder) -> ForkBuilder {
    fork.resource(Interface::TYPE, 128, None, &bar_interface())
        .resource(PICT, 700, None, &bar_picture())
}

/// The mission info screen's shape, smaller: 300 x 150, fixed at
/// (0, 0), with Done (1), the list (2), the heading (3), the text (4),
/// Abort (5) and the date (7).
fn mission_info_template() -> DialogTemplate {
    let item = |x, y, w, h| ItemTemplate {
        bounds: Bounds::at(Point::new(x, y), w, h),
        enabled: true,
        kind: ItemSpec::User,
    };
    DialogTemplate {
        bounds: Bounds::at(Point::new(0.0, 0.0), 300.0, 150.0),
        placement: Placement::Fixed,
        items: vec![
            item(190.0, 120.0, 99.0, 25.0),
            item(10.0, 24.0, 100.0, 84.0),
            item(10.0, 1.0, 190.0, 12.0),
            item(120.0, 26.0, 170.0, 80.0),
            item(60.0, 120.0, 99.0, 25.0),
            item(0.0, 300.0, 1.0, 1.0),
            item(200.0, 4.0, 90.0, 12.0),
        ],
    }
}

/// How wide the fuel gauge's full colour is painted, at y 234 to 241
/// in [`bar_interface`]'s yellow.
fn fuel_width(list: &DrawList) -> Option<f32> {
    list.iter().find_map(|command| match *command {
        DrawCommand::Line {
            from, to, color, ..
        } if color == Color::from_rgb24(BAR_FUEL) && (234.0..241.0).contains(&from.y) => {
            Some(to.x - from.x)
        }
        _ => None,
    })
}

/// [`VenueDialogs`] with the mission info screen and the bar.
struct InfoOverEveryOpener;

impl DialogResources for InfoOverEveryOpener {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        match id {
            INFO_DIALOG => Ok(mission_info_template()),
            BAR_DIALOG => Ok(bar_template()),
            _ => VenueDialogs.dialog_template(id),
        }
    }
}
