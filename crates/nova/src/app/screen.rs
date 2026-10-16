//! The screen router: every screen the app can show, as one [`Screen`].
//!
//! # The main menu and pilots
//!
//! The game opens on the main menu ([`AppScreen::with_pilots`]): New
//! Pilot asks for a name in "Create a new pilot:" (the interface file's,
//! or a built-in one), refusing a name already saved, then creates the
//! pilot from the first `chär`, saves it and flies it. Open Pilot lists
//! the saved pilots and flies the one chosen, in space over the stellar
//! it last landed on, with no spaceport opened. A pilot whose load had to
//! fall back for data the save references and the game data lacks
//! ([`nova_sim::Session::load`], under the router's rules) is flown only
//! once the note `STR#` 140 #52 over the menu is dismissed; its flight,
//! built behind the note, doesn't tick meanwhile, and quitting with the
//! note open saves the repaired pilot. Quit quits after
//! [`QUIT_DELAY`], and each of the menu's keys first waits for its click
//! to play (see `menu_input`). Escape in
//! flight goes back to the menu, saving the pilot and putting it away.
//! Each flight put away leaves its ship slots' draw-layer history, which
//! every later flight of the run starts from, as the original's sprite
//! layers outlive a pilot ([`nova_sim::ship_layer`]).
//!
//! A pilot with a name is saved through the [`PilotKeeper`] on landing,
//! on taking off, after each change made in the spaceport
//! ([`AppScreen::transact`]), on going back to the menu and when the app
//! quits ([`Screen::quit`]); each failure is a warning
//! ([`Screen::take_warnings`]). Tab on the menu goes to the developer's
//! sides below, where F flies a fresh pilot without a name, never saved,
//! and Escape at the top goes back to the menu. A router without a main
//! menu ([`start_screen`] alone) opens on the ship browser, and Escape at
//! the top quits.
//!
//! A pilot whose ship is breaking up or destroyed is never saved
//! ([`FlightView::may_save`]): not on going back to the menu, not when the
//! app quits, and not when a save comes due, which is dropped. The
//! original saves only on creation, landing and pod rescue
//! (`_SavePilotFile` @0x76876), so a dead pilot reopens as it was at its
//! last landing. A Strict Play pilot whose ship is breaking up or
//! destroyed, outside the escape pod, is deleted instead as play ends
//! ([`FlightView::deletes_pilot_on_ending_play`]): on the death's return
//! to the menu, on Escape's put-away and when the app quits, as
//! `_EnterGameFromMainScreen` deletes it (@0x1a4ab-0x1a4e1); a failed
//! delete is a warning. Once the player's ship has exploded, the scene runs on
//! through the hold; when it expires, the router shows the `dësc` of
//! each of a sabotaged player's outfits over the stopped flight, one
//! after another, and begins flight's fade to black as the last closes,
//! or with none on the same tick. When the fade is over the router drops flight, as
//! Escape's put-away does, and returns to the main menu (or, without one,
//! the side flight was entered from), which fades in from black over
//! [`DEATH_FADE_FOR`], taking input under the fade, as the original's
//! `_EnterGameFromMainScreen` (0x1a471) fades the main screen in.
//!
//! # The developer's sides
//!
//! The router keeps two sides alive, the ship browser and the
//! [`Navigator`] (the galaxy map and the system opened from it), and shows
//! one at a time. Tab switches sides, so each keeps its state (the
//! selected ship; the map's view and selection, and the open system with
//! its camera) while hidden. The router also decides what Escape does:
//! back one level, from a system to the map, and quit at the top.
//!
//! F, from either side, enters flight: the [`FlightView`], built the first
//! time and kept, so flight resumes where it left off. In flight the
//! arrow keys fly the ship, Space and Control fire, W selects the
//! secondary weapon, Tab and R pick the target, and Escape goes back to the
//! screen flight was entered from; it never quits.
//!
//! M and J are flight's own keys. M opens flight's course map, where a
//! click sets the destination; Escape (or M) closes it, back into flight.
//! J jumps along the course. The Tab side's galaxy map stays the
//! developer's viewer, which enters systems; play plots courses on the map
//! opened from flight, which has no "Enter system" button.
//!
//! Caps Lock, pressed anywhere, turns double time on or off for flight
//! (see [`FlightView::set_double_time`]); the router keeps it across
//! screens and pilots, and saves nothing of it.
//!
//! L in flight requests clearance at a stellar, and a second L, over it,
//! lands and shows its spaceport ([`SpaceportView`]), laid out by the
//! interface file's "Spaceport" dialog when the router has dialogs;
//! without them the spaceport says why. The spaceport takes every input:
//! Leave, Return and Escape take off, back into flight at the stellar,
//! Tab and F do nothing, and I opens the mission info screen. A `Q` run
//! while landed (a mission's `OnSuccess` at the landing, an `OnAccept`,
//! a purchase's hook) leaves the same way, once the texts and dialogs
//! shown over the spaceport have closed: the landing's texts show over
//! the spaceport first, then its offer, and the input that closes the
//! last of them takes off, the `Q`'s message the take-off line. A screen
//! open over the spaceport (the exchange, the outfitter, the shipyard,
//! the bar and its hire screen, the mission computer) closes with it, as
//! the original's filters close theirs.
//!
//! At a trade center, the spaceport's Trade Center opens the session's
//! exchange, laid out by the interface file's "Trade" dialog. There a
//! plain B buys and S sells up to ten tons of the selected good (a ton by
//! the `trade_lot` rule's other reading), and with Alt held the quantity
//! dialog (`DLOG` 1003, or its fallback) asks how many, up to
//! the engine's maximum (the `trade_quotient` rule says how it divides);
//! by the `trade_count` rule's other reading, Alt trades that maximum at
//! once. The router makes each trade through the session, hands the exchange as
//! it now is back to the screen, and saves the pilot after the input.
//!
//! At an outfitter, the spaceport's Outfitter opens the session's
//! outfitter, laid out by the interface file's "Outfit" dialog, each
//! outfit's picture and description read from the game data. There B buys
//! and S sells one of the selected outfit; the router makes each order
//! through the session, whose stats change with it, hands the outfitter as
//! it now is back to the screen, and saves the pilot after the input.
//!
//! At a shipyard, the spaceport's Shipyard opens the session's shipyard,
//! laid out by the interface file's "Shipyard" dialog, with its "Shipyard
//! Info" panel, each ship's picture and description read from the game
//! data. There B buys the selected ship, trading in the one flown; the
//! router makes the purchase through the session, which flies the new
//! ship from then on, hands the exchange, the outfitter and the shipyard
//! as they now are back to the screen (any order changes the cash, and a
//! ship the cargo space, the free mass and the outfits), and saves the
//! pilot after the input.
//!
//! At a bar, the spaceport's Bar opens the stellar's bar, laid out by the
//! interface file's bar dialog (`DLOG` 1013), its hire screen by
//! "Shipyard" and "Shipyard Info"; Hire Escort is greyed while the fleet
//! is full. Its Hire Escort asks the session for the ships for hire, the
//! day's rolls drawn on the router's chance, and the router opens them on
//! the hire screen; H there hires the selected ship through the session,
//! and the router builds the list afresh (which draws the hired class's
//! roll again), closes the hire screen back to the bar, hands the
//! exchange, the outfitter and the shipyard back with the cash as it now
//! is, and saves the pilot after the input.
//!
//! The spaceport's Mission BBS opens the session's mission computer,
//! laid out by the interface file's `DLOG` 1006, or shows why it cannot
//! in "Desc Dialog" (every slot taken, or nothing available). Its Accept
//! accepts the selected mission through the session, on the router's
//! chance, and the computer is shown again as the session now has it,
//! closing silently once nothing is available. The mission texts the
//! session queues (a mission's `BriefText`, `CompText`, `FailText` and
//! `RefuseText`) show one after another in "Desc Dialog" over the
//! spaceport, after each input and tick. Those queued in flight (by an
//! `S`, such as one a deadline missed on the take-off runs through the
//! mission's `OnFailure`, or by a person's offer answered over flight)
//! show the same way over flight, which pauses under them, as
//! `_ShowTextDialog` shows them.
//!
//! I, in the spaceport, the bar or the mission computer, opens the
//! mission info screen (`DLOG` 1012) over whatever is open, when an
//! active mission is shown there; its Abort aborts the selected mission
//! through the session, and the screen shows the missions left, closing
//! once none is. I in flight opens it the same way over flight, which
//! pauses under it; with no mission shown there, flight says "You have
//! no active missions." on its message line, as it says "Time limit
//! exceeded - mission failed." when a deadline runs out in flight.
//!
//! Missions are offered at five venues as the original's
//! `_OfferMissionFromPort` offers them: the spaceport asks once, as it
//! opens on landing, before anything can open over it; the trade center,
//! outfitter and shipyard ask as they open; the bar a quarter second
//! after it opens. The router answers each with the session's next offer
//! at that venue, which enters it, shown in the offer dialog (`DLOG`
//! 1016); its choice accepts or refuses the mission through the session,
//! the texts it queued show, and the venue open asks again after a gap
//! drawn on the router's chance (the spaceport never does). A venue is
//! entered only as it is offered at, so an offer refused for want of a
//! slot stays suppressed until another venue but the spaceport is: going
//! back to the spaceport and into the bar again keeps it, opening a shop
//! or landing again and opening the bar forgets it.
//!
//! Where fuel is sold, the spaceport's Recharge fills the tank through the
//! session; the router tells the screen how it went (a refusal says why in
//! the description box), hands the exchange, the outfitter and the
//! shipyard back with the cash as it now is, and saves the pilot after the
//! input.
//!
//! Each day a jump takes rolls the planetary events, and the NPC traffic
//! its rolls, on the router's source of chance
//! ([`AppScreen::with_chance`]), which never fires until one is given, so
//! the developer's flights stay the same each time; each flight's
//! explosions and debris roll on another
//! ([`AppScreen::with_effects_chance`]), the same until one is given. A
//! flight's HUD lays out its text with the router's metrics, the
//! dialogs' or the main menu's. Every flight's NPCs
//! decide as the router's behaviour says ([`AppScreen::with_behaviour`]),
//! Nova's combat AI until another is given, and their ships are
//! disabled as the router's rule says ([`AppScreen::with_disable_rule`]),
//! Nova's own until another is given; their point defence engages the
//! missiles the router's point-defence rule calls hostile
//! ([`AppScreen::with_point_defence_rule`]), by governments' allegiance
//! until another is given; the player's crimes are judged by the
//! router's law ([`AppScreen::with_law`]), Nova's until another is given;
//! escorts are hired by the router's hire terms and control bits
//! ([`AppScreen::with_hire_terms`], [`AppScreen::with_control_bits`]),
//! and persons appear as the router's person rules say
//! ([`AppScreen::with_person_rules`]), Nova's until others are given.
//! Every other disputed rule, each flight reads from the router's rule
//! set ([`AppScreen::with_rules`]), every rule the engine's until another
//! is given; [`AppScreen::with_rulebook`] sets the rule set from one
//! rulebook and builds each of the strategies above from it. Each time
//! the spaceport's outfitter opens, the flight is told
//! ([`FlightView::open_outfitter`]) and the outfitter shown afresh; each
//! time it closes, or the bar's hire screen closes, the flight is told
//! ([`FlightView::close_outfitter`], [`FlightView::close_hire`]) and the
//! shops shown afresh, as each resets the session. The player info screen
//! opens on what [`FlightView::open_player_info`] gives, which resets the
//! session first.
//! The flight's diagnostics about game
//! data it could not read or the simulation does not handle yet come
//! through [`Screen::take_diagnostics`].
//!
//! B, in flight, boards the target ([`FlightView`]'s boarding, by the
//! router's [`BoardingRule`], [`AppScreen::with_boarding_rule`], Nova's
//! until another is given). A boarding opens the plunder dialog over
//! flight, which pauses under it as under the Preferences dialog: the
//! interface file's "Plunder" dialog when the router has dialogs, or a
//! built-in one (with a warning when the interface file has none); with
//! no metrics to lay it out the boarding is let go, with a warning. Each
//! take goes through flight, which says what it took, and the dialog
//! shows what is still on board, or closes once the boarding is over. A
//! capture that awaits its assignment opens the assignment dialog the
//! same way, and the choice goes through flight; the pilot is saved after
//! the input that captured or assigned, as after a change in the
//! spaceport.
//!
//! I, outside flight and the spaceport, opens the About text in the game's "Desc Dialog"
//! over the screen shown, when the router was given the interface file's
//! dialogs ([`AppScreen::with_dialogs`]). The dialog is modal: it takes
//! every input until Done (Return, Escape or a click) closes it.
//!
//! P, on any side, opens the Preferences dialog ("new prefs dialog") over
//! the screen shown when the router has dialogs. Like the About dialog it
//! is modal, and flight pauses under it. It opens on the player's
//! preferences ([`AppScreen::with_prefs`]), and each change is
//! reported once through [`Screen::take_prefs`] for the app to play
//! and save. Flight follows the Hyperspace Effects preference: it is
//! built with it, and a change reaches a flight already built. OK, Return or Escape closes it.
//! Its close redraws nothing of the status bar, as the original's menu
//! command `pref` calls `_DoPrefsDialog` alone (@0xcccf1); only the `pref`
//! Apple Event's handler redraws (@0x56195).
//!
//! The router reports every live screen's sounds through
//! [`Screen::take_sounds`], and what it shows through
//! [`Screen::now_showing`]. The spaceport and the two dialogs are dropped
//! as they close, so their sounds (the click that closed them) are kept
//! first.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_sim::board::MAX_ESCORTS;
use nova_sim::ship_layer::SlotSprites;
use nova_sim::{
    Allegiance, Behaviour, BoardingRule, CommCatalog, ControlBits, DisableRule, HailOptions,
    HailView, HireTerms, LegalCode, MissionInfo, NovaAi, NovaBits, NovaBoarding, NovaDisable,
    NovaHire, NovaLaw, NovaPersons, OfferPrompt, PersonRules, Pilot, PilotKeeper, PilotStore,
    PointDefenceRule, RuleKey, Rulebook, Session, Take, Taken, VenueOffer, pilot_key, text_tags,
};
pub use nova_view::Showing;
use nova_view::devtools::SessionDesk;
use nova_view::fade::{DEATH_FADE_FOR, ScreenFade};
use nova_view::flight::{FlightOverlays, FlightView, SharedChance};
use nova_view::galaxy::GalaxyMap;
use nova_view::menu::{Chosen, ChosenBy, MainMenu, MenuChoice, PilotList, PilotListOutcome};
use nova_view::movie::{MoviePlayer, Movies};
use nova_view::ships::ShipBrowser;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::bar::{BAR_DIALOG, BarTemplates, PICTURE_BAR_DIALOG};
use nova_view::spaceport::layout::SPACEPORT_DIALOG;
use nova_view::spaceport::missions::{COMPUTER_DIALOG, INFO_DIALOG, MissionInfoScreen};
use nova_view::spaceport::news::NEWS_DIALOG;
use nova_view::spaceport::offer::{
    OFFER_DIALOG, OfferChoice, OfferDialog, OfferTemplates, PICTURE_OFFER_DIALOG,
};
use nova_view::spaceport::outfitter::OUTFIT_DIALOG;
use nova_view::spaceport::shipyard::{
    InfoTemplates, SHIP_INFO_DIALOG, SHIP_PICTURE_INFO_DIALOG, SHIPYARD_DIALOG,
};
use nova_view::spaceport::trade::TRADE_DIALOG;
use nova_view::spaceport::{OutfitterCatalog, ShipyardCatalog};
use nova_view::system::SystemView;
use nova_view::text::TextMetrics;
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::ui::comm::{COMM_DIALOG, HAGGLE_DIALOG};
use nova_view::ui::desc::{DESC_DIALOG, DescTemplates, PICTURE_DESC_DIALOG};
use nova_view::ui::mission_dialogs::{DescShown, MissionDialogs};
use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome};
use nova_view::ui::note::{LOAD_FALLBACK_NOTE, NOTE_DIALOG, NoteDialog};
use nova_view::ui::player_info::{PLAYER_INFO_DIALOG, PlayerInfoScreen, YES_NO_DIALOG};
use nova_view::ui::plunder::{ASSIGNMENT_DIALOG, PLUNDER_DIALOG};
use nova_view::ui::prefs::PREFS_DIALOG;
use nova_view::ui::quantity::QUANTITY_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::ui::{
    AssignmentDialog, CommDialog, CommPress, DescDialog, DescriptionSource, DialogResources,
    HaggleDialog, PlunderDialog, PlunderShown, PrefsDialog,
};
use nova_view::{
    Color, Diagnostic, DrawList, Input, Key, Navigator, Point, Prefs, Screen, ScreenAction, Sound,
    UiSound,
};

/// The hint the router draws over every screen, where it goes, its size and
/// its colour. Every screen leaves that corner free. The hint offers I
/// and P only when the router has dialogs, and so they do something;
/// without them it draws [`HINT_WITHOUT_DIALOGS`].
pub const HINT: &str = "Tab: ships / galaxy map   F: fly   I: about   P: preferences";
pub const HINT_WITHOUT_DIALOGS: &str = "Tab: ships / galaxy map   F: fly";
/// The hint the router draws over the main menu, with dialogs and without.
pub const MENU_HINT: &str = "Tab: ships / galaxy map   I: about   P: preferences";
pub const MENU_HINT_WITHOUT_DIALOGS: &str = "Tab: ships / galaxy map";
pub const HINT_AT: Point = Point::new(16.0, 8.0);
pub const HINT_SIZE: f32 = 14.0;
pub const HINT_COLOR: Color = Color::DIM;

/// The key that, with Command held, opens the Preferences on every side:
/// the Mac's Command+comma.
pub const PREFERENCES_KEY: Key = Key::Char(',');

/// The About text's `dësc`.
pub const ABOUT_TEXT: i16 = 32767;

/// The `dësc` the main menu's X shows (`_MainScreenKey` @0x1ac7e,
/// `_LoadDescription(0x7ffe)`).
pub const MENU_X_TEXT: i16 = 32766;

/// How long the main menu's Quit waits after its click before the app
/// quits: `_MainScreenChoice(2)` calls `Delay(15)` (@0x1a9f0-0x1a9fb)
/// before it sends itself `quit`, which lets 601 sound.
pub const QUIT_DELAY: Duration = nova_sim::mission::screens::mac_ticks(15);

/// Why the spaceport cannot be laid out when the router has no dialogs.
pub const NO_INTERFACE: &str = "no interface file";

/// What the New Pilot dialog says of a name no pilot file can be saved
/// under, such as `..`.
pub const UNUSABLE_NAME: &str = "That name can't be used for a pilot file.";

/// The main menu, the two sides Tab switches between, and flight,
/// entered from the menu or from either side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    MainMenu,
    Ships,
    Galaxy,
    Flight,
    Spaceport,
}

/// What a main-menu key does once its click has played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    /// The menu's own choice: New Pilot, Open Pilot or Quit.
    Choice(MenuChoice),
    /// E: Enter Ship, which with no pilot loaded is refused.
    Enter,
    /// P: the Preferences.
    Preferences,
    /// A: the About text.
    About,
    /// X: `dësc` 32766.
    Text,
}

/// What the main menu waits on, as `_MainScreenKey` @0x1aba0 spins
/// before its choice runs.
///
/// The original reads no event during the spin: what arrives meanwhile
/// stays in the OS queue for whatever the choice opens. So while the
/// menu waits, the router queues every input (and each letting go of the
/// pointer or the keys, in its place) and replays them in order once
/// the choice has run. The engine notices the click's end at its next
/// audio feed, an input or a frame, not in a spin; the choice still runs
/// in the pass that plays 601, before the next frame is drawn, so no
/// frame shows the wait differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuHold {
    /// E was typed with the physical E key held: the original spins on
    /// `_QuickKeyCheck(0x0e)` (@0x1ac32-0x1ac57) until it is released,
    /// and only then clicks.
    EHeld,
    /// A keyed click plays (`_PlayMainScreenClickSound` @0x177af); the
    /// action runs once it has ([`Screen::keyed_click_ended`]).
    Click(MenuAction),
}

/// An input held while the main menu waits, or a letting go in its
/// place.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Held {
    Input(Input),
    CancelPointer,
    ReleaseKeys,
}

/// Every screen the app can show, and which side it is showing. The router
/// forwards input, ticks and drawing to the side shown, switches sides on
/// a Tab press (not on its key repeats) and decides what an Escape press
/// does.
#[derive(Clone, Debug)]
pub struct AppScreen {
    side: Side,
    /// The side flight was last entered from, which Escape goes back to.
    return_to: Side,
    /// The game data, which flight reads when it is first entered.
    data: Rc<GameData>,
    /// The ship browser, reading the game data the renderer draws from.
    ships: ShipBrowser<Rc<GameData>>,
    /// The galaxy map and any system opened from it, reading the same game
    /// data.
    galaxy: Navigator<Rc<GameData>>,
    /// Flight, once entered, reading the same game data.
    flight: Option<FlightView<Rc<GameData>>>,
    /// The ship slots' draw-layer and cloned-frame history the last
    /// flight put away left, which each new flight starts from: the
    /// original's sprites outlive a pilot, `_InitSprites` running once at
    /// launch (see [`nova_sim::ship_layer`]).
    slot_sprites: SlotSprites,
    /// What dialogs are built from, once given.
    dialogs: Option<Dialogs>,
    /// The About dialog, or the main menu's X text, while it is open.
    about: Option<DescDialog>,
    /// The `dësc` the open [`about`](Self::about) dialog shows.
    about_text: i16,
    /// The spaceport, while the ship is landed.
    spaceport: Option<SpaceportView>,
    /// The sounds of screens closed since the sounds were last taken.
    sounds: Vec<Sound>,
    /// The player's preferences, as last chosen.
    prefs: Prefs,
    /// The Preferences dialog, while it is open.
    preferences: Option<PrefsDialog>,
    /// The preferences chosen since they were last taken, if they changed.
    prefs_change: Option<Prefs>,
    /// The main menu and where pilots are kept, once given.
    menu: Option<Menu>,
    /// The New Pilot dialog, while it is open over the main menu.
    new_pilot: Option<NewPilotDialog>,
    /// The saved pilots' list, while it is open over the main menu.
    open_pilot: Option<PilotList>,
    /// The note that an opened pilot's load fell back (`STR#` 140 #52),
    /// while it is open over the main menu, its flight built behind it.
    load_note: Option<NoteDialog>,
    /// The warnings since they were last taken.
    warnings: Vec<String>,
    /// What each flight rolls each day's events and its traffic on.
    chance: SharedChance,
    /// What each flight's explosions and debris are rolled on.
    effects_chance: SharedChance,
    /// How each flight's NPCs decide.
    behaviour: Rc<dyn Behaviour>,
    /// When each flight's ships are disabled.
    disable_rule: Rc<dyn DisableRule>,
    /// Which missiles each flight's point defence engages.
    defence_rule: Rc<dyn PointDefenceRule>,
    /// What each flight's player's crimes do to its legal record.
    law: Rc<dyn LegalCode>,
    /// Who repels boarders, the capture odds and the capture roll, in
    /// each flight.
    boarding_rule: Rc<dyn BoardingRule>,
    /// The plunder dialog, while a boarding is under way.
    plunder: Option<PlunderDialog>,
    /// The captured-ship assignment dialog, while a capture awaits it.
    assignment: Option<AssignmentDialog>,
    /// The options each flight's comm dialog lists.
    hail_options: HailOptions,
    /// How each disputed rule reads in each flight.
    rules: Rulebook,
    /// The port's own overlays each flight draws.
    flight_overlays: FlightOverlays,
    /// The fee and wage of a hire in each flight.
    hire_terms: Rc<dyn HireTerms>,
    /// How persons appear in each flight.
    person_rules: Rc<dyn PersonRules>,
    /// The control-bit test a ship for hire's `Availability` goes
    /// through in each flight.
    control_bits: Rc<dyn ControlBits>,
    /// The comm dialog, while a hail is under way.
    comm: Option<CommDialog>,
    /// The haggle dialog, over the comm dialog, while a price is asked.
    haggle: Option<HaggleDialog>,
    /// The offer dialog of a person hailed or boarded, over flight, until
    /// it is answered; or, answered and not hidden, drawn under the
    /// mission dialogs its answer opened until they close
    /// ([`AppScreen::settle_offer`]). While mission dialogs are open over
    /// it, it is neither ticked nor given input, so its movie waits for
    /// theirs (`_PlayQTMovie` is modal).
    offer: Option<OfferDialog>,
    /// The mission texts and the mission info screen over flight, while
    /// one is open.
    missions: Option<MissionDialogs>,
    /// The movies the mission texts and offers play, and whether they
    /// play: [`NoMoviePlayer`](crate::movie::NoMoviePlayer)'s unless
    /// another is given ([`AppScreen::with_movie_player`]), on as the
    /// preferences say.
    movies: Movies,
    /// The fade back from black over the side shown after a death, while
    /// it plays.
    fade_in: Option<ScreenFade>,
    /// The player info screen over flight, while it is open.
    player_info: Option<PlayerInfoScreen>,
    /// Whether Command is held, for Command+comma.
    command_held: bool,
    /// Whether the physical S, L and E keys are held: O does nothing
    /// while S or L is (`_KeyCheck(0x01)`, `_KeyCheck(0x25)`
    /// @0x1abd3-0x1abf5), and E waits for E's release.
    keys_held: MenuKeys,
    /// What the main menu waits on, if anything.
    menu_hold: Option<MenuHold>,
    /// What arrived while the main menu waited, in order.
    held: Vec<Held>,
    /// What is left of Quit's delay, while it runs ([`QUIT_DELAY`]).
    quit_in: Option<Duration>,
    /// Whether a quit is due, for [`Screen::take_quit`].
    quit_due: bool,
    /// Whether double time is on: Caps Lock's presses toggle it, on any
    /// side, and every flight is told. Never saved, as the original's
    /// `_doubleTime` is not.
    double_time: bool,
}

/// The physical keys the main menu reads as held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MenuKeys {
    s: bool,
    l: bool,
    e: bool,
}

/// The main menu, the metrics its screens' text is laid out by when there
/// are no dialogs, and where pilots are kept (if anywhere).
#[derive(Clone)]
struct Menu {
    screen: MainMenu,
    metrics: Rc<dyn TextMetrics>,
    pilots: Option<Rc<PilotKeeper<Box<dyn PilotStore>>>>,
}

impl std::fmt::Debug for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menu")
            .field("screen", &self.screen)
            .field(
                "pilots",
                &self.pilots.as_ref().map(|keeper| keeper.store().location()),
            )
            .finish_non_exhaustive()
    }
}

/// The interface file's dialogs and the metrics their text is laid out by.
#[derive(Clone)]
struct Dialogs {
    resources: Rc<dyn DialogResources>,
    metrics: Rc<dyn TextMetrics>,
}

impl std::fmt::Debug for Dialogs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialogs").finish_non_exhaustive()
    }
}

impl AppScreen {
    /// Every screen over `data`, showing the ship browser.
    #[must_use]
    pub fn new(data: Rc<GameData>) -> Self {
        Self {
            side: Side::Ships,
            return_to: Side::Ships,
            galaxy: Navigator::new(Rc::clone(&data)),
            ships: ShipBrowser::new(Rc::clone(&data)),
            data,
            flight: None,
            slot_sprites: SlotSprites::default(),
            dialogs: None,
            about: None,
            about_text: ABOUT_TEXT,
            spaceport: None,
            sounds: Vec::new(),
            prefs: Prefs::default(),
            preferences: None,
            prefs_change: None,
            menu: None,
            new_pilot: None,
            open_pilot: None,
            load_note: None,
            warnings: Vec::new(),
            chance: SharedChance::default(),
            effects_chance: SharedChance::default(),
            behaviour: Rc::new(NovaAi::default()),
            disable_rule: Rc::new(NovaDisable),
            defence_rule: Rc::new(Allegiance),
            law: Rc::new(NovaLaw::default()),
            boarding_rule: Rc::new(NovaBoarding::default()),
            plunder: None,
            assignment: None,
            hail_options: HailOptions::default(),
            rules: Rulebook::default(),
            flight_overlays: FlightOverlays::default(),
            hire_terms: Rc::new(NovaHire::default()),
            control_bits: Rc::new(NovaBits),
            person_rules: Rc::new(NovaPersons::default()),
            comm: None,
            haggle: None,
            offer: None,
            missions: None,
            movies: Movies::new(
                Rc::new(crate::movie::NoMoviePlayer),
                Prefs::default().quicktime_movies,
            ),
            fade_in: None,
            player_info: None,
            command_held: false,
            keys_held: MenuKeys::default(),
            menu_hold: None,
            held: Vec::new(),
            quit_in: None,
            quit_due: false,
            double_time: false,
        }
    }

    /// The router with each flight rolling each day's events and its
    /// traffic on `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The router with each flight's explosions and debris rolled on
    /// `chance`, apart from the simulation's.
    #[must_use]
    pub fn with_effects_chance(self, effects_chance: SharedChance) -> Self {
        Self {
            effects_chance,
            ..self
        }
    }

    /// The router with each flight's NPCs deciding as `behaviour` says.
    #[must_use]
    pub fn with_behaviour(self, behaviour: Rc<dyn Behaviour>) -> Self {
        Self { behaviour, ..self }
    }

    /// The router with each flight's ships disabled as `rule` says.
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            disable_rule,
            ..self
        }
    }

    /// The router with each flight's point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            defence_rule,
            ..self
        }
    }

    /// The router with each flight's player's crimes judged by `law`.
    #[must_use]
    pub fn with_law(self, law: Rc<dyn LegalCode>) -> Self {
        Self { law, ..self }
    }

    /// The router with each flight's boardings by `rule`: who repels
    /// boarders, the capture odds and the capture roll.
    #[must_use]
    pub fn with_boarding_rule(self, boarding_rule: Rc<dyn BoardingRule>) -> Self {
        Self {
            boarding_rule,
            ..self
        }
    }

    /// The router with each flight's comm dialog listing `hail_options`.
    #[must_use]
    pub fn with_hail_options(self, hail_options: HailOptions) -> Self {
        Self {
            hail_options,
            ..self
        }
    }

    /// The router with `terms` giving the fee and wage of a hire in each
    /// flight ([`FlightView::with_hire_terms`]); Nova's until others are
    /// given.
    #[must_use]
    pub fn with_hire_terms(self, hire_terms: Rc<dyn HireTerms>) -> Self {
        Self { hire_terms, ..self }
    }

    /// The router with `bits` testing a ship for hire's `Availability` and
    /// a person's `ActiveOn` against the pilot in each flight
    /// ([`FlightView::with_control_bits`]); Nova's
    /// ([`NovaBits`](nova_sim::NovaBits)) until others are given.
    #[must_use]
    pub fn with_control_bits(self, control_bits: Rc<dyn ControlBits>) -> Self {
        Self {
            control_bits,
            ..self
        }
    }

    /// The router with `rules` deciding how persons appear in each flight
    /// ([`FlightView::with_person_rules`]); Nova's until others are given.
    #[must_use]
    pub fn with_person_rules(self, person_rules: Rc<dyn PersonRules>) -> Self {
        Self {
            person_rules,
            ..self
        }
    }

    /// The router with each flight's disputed rules reading as `rules`
    /// says ([`FlightView::with_rules`]): every rule the engine's until
    /// others are given. Only the rule set changes: the strategies the
    /// router hands each flight (its behaviour, law, boarding, hail
    /// options, hire terms and persons) stay as they are, so a rule one
    /// of them decides changes only through [`AppScreen::with_rulebook`].
    #[must_use]
    pub fn with_rules(self, rules: Rulebook) -> Self {
        Self { rules, ..self }
    }

    /// The router with each flight drawing the port's own `overlays`, the
    /// title and the help line ([`FlightView::with_overlays`]): both off
    /// until others are given. They are developer settings, not rules.
    #[must_use]
    pub fn with_flight_overlays(self, flight_overlays: FlightOverlays) -> Self {
        Self {
            flight_overlays,
            ..self
        }
    }

    /// The router with Nova's rules, each disputed one as `rulebook`
    /// chooses: the strategies each flight decides by (the NPCs'
    /// behaviour, disabling, point defence, the law, boarding, hailing,
    /// the hire terms, the control bits and the persons), built from it,
    /// and the rule set every flight reads the rest from
    /// ([`AppScreen::with_rules`]). This is the edge where every
    /// [`RuleKey`](nova_sim::RuleKey) meets its setting.
    #[must_use]
    pub fn with_rulebook(self, rulebook: &Rulebook) -> Self {
        self.with_behaviour(Rc::new(NovaAi::from_rulebook(rulebook)))
            .with_disable_rule(Rc::new(NovaDisable))
            .with_point_defence_rule(Rc::new(Allegiance))
            .with_law(Rc::new(NovaLaw::from_rulebook(rulebook)))
            .with_boarding_rule(Rc::new(NovaBoarding::from_rulebook(rulebook)))
            .with_hail_options(HailOptions::nova(rulebook))
            .with_hire_terms(Rc::new(NovaHire::from_rulebook(rulebook)))
            .with_control_bits(Rc::new(NovaBits))
            .with_person_rules(Rc::new(NovaPersons::from_rulebook(rulebook)))
            .with_rules(*rulebook)
    }

    /// The comm dialog, while a hail is under way.
    #[must_use]
    pub fn comm(&self) -> Option<&CommDialog> {
        self.comm.as_ref()
    }

    /// The haggle dialog, while a price is asked.
    #[must_use]
    pub fn haggle(&self) -> Option<&HaggleDialog> {
        self.haggle.as_ref()
    }

    /// The offer dialog of a person hailed or boarded, while it is open.
    #[must_use]
    pub fn offer(&self) -> Option<&OfferDialog> {
        self.offer.as_ref()
    }

    /// The player info screen open over flight, if it is.
    #[must_use]
    pub fn flight_player_info(&self) -> Option<&PlayerInfoScreen> {
        self.player_info.as_ref()
    }

    /// The mission text shown over flight, if any.
    #[must_use]
    pub fn flight_mission_text(&self) -> Option<&DescDialog> {
        self.missions.as_ref()?.text()
    }

    /// The mission info screen open over flight, if it is.
    #[must_use]
    pub fn flight_mission_info(&self) -> Option<&MissionInfoScreen> {
        self.missions.as_ref()?.info()
    }

    /// The plunder dialog, while a boarding is under way.
    #[must_use]
    pub fn plunder(&self) -> Option<&PlunderDialog> {
        self.plunder.as_ref()
    }

    /// The captured-ship assignment dialog, while a capture awaits it.
    #[must_use]
    pub fn assignment(&self) -> Option<&AssignmentDialog> {
        self.assignment.as_ref()
    }

    /// A flight over the game data, `new` from it, rolling on the router's
    /// chance, its effects on the router's effects chance, deciding as its
    /// behaviour says, disabling ships, engaging missiles and judging
    /// crimes as its rules say, naming its ship from the game data's
    /// string lists, and laying out its HUD's text with the router's
    /// metrics, if it has any.
    fn flight(
        &self,
        new: impl FnOnce(Rc<GameData>) -> FlightView<Rc<GameData>>,
    ) -> FlightView<Rc<GameData>> {
        let flight = new(Rc::clone(&self.data))
            .with_chance(self.chance.clone())
            .with_effects_chance(self.effects_chance.clone())
            .with_behaviour(Rc::clone(&self.behaviour))
            .with_disable_rule(Rc::clone(&self.disable_rule))
            .with_point_defence_rule(Rc::clone(&self.defence_rule))
            .with_law(Rc::clone(&self.law))
            .with_boarding_rule(Rc::clone(&self.boarding_rule))
            .with_hail_options(self.hail_options.clone())
            .with_rules(self.rules)
            .with_slot_sprites(self.slot_sprites.clone())
            .with_overlays(self.flight_overlays)
            .with_hire_terms(Rc::clone(&self.hire_terms))
            .with_control_bits(Rc::clone(&self.control_bits))
            .with_person_rules(Rc::clone(&self.person_rules))
            .with_strings(Rc::clone(&self.data) as Rc<dyn CommCatalog>)
            .with_hyperspace_effects(self.prefs.hyperspace_effects)
            .with_sound_prefs(self.prefs.sound)
            .with_double_time(self.double_time);
        match self.metrics() {
            Some(metrics) => flight.with_metrics(metrics),
            None => flight,
        }
    }

    /// The metrics the router's text is laid out by: the dialogs', or the
    /// main menu's, if either was given.
    fn metrics(&self) -> Option<Rc<dyn TextMetrics>> {
        let dialogs = self.dialogs.as_ref().map(|dialogs| &dialogs.metrics);
        let menu = self.menu.as_ref().map(|menu| &menu.metrics);
        dialogs.or(menu).map(Rc::clone)
    }

    /// The router with the main menu, which it now opens on: New Pilot
    /// and Open Pilot save and open pilots through `pilots`, and its
    /// screens lay out their text with `metrics` when there are no
    /// dialogs. With no `pilots` (nowhere to keep them), New Pilot still
    /// flies, saving nothing, and Open Pilot lists nothing.
    #[must_use]
    pub fn with_pilots(
        self,
        pilots: Option<PilotKeeper<Box<dyn PilotStore>>>,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let screen = MainMenu::new(self.data.button_style(), Rc::clone(&metrics));
        Self {
            side: Side::MainMenu,
            menu: Some(Menu {
                screen,
                metrics,
                pilots: pilots.map(Rc::new),
            }),
            ..self
        }
    }

    /// The main menu, once given.
    #[must_use]
    pub fn main_menu(&self) -> Option<&MainMenu> {
        self.menu.as_ref().map(|menu| &menu.screen)
    }

    /// The New Pilot dialog, while it is open.
    #[must_use]
    pub fn new_pilot(&self) -> Option<&NewPilotDialog> {
        self.new_pilot.as_ref()
    }

    /// The saved pilots' list, while it is open.
    #[must_use]
    pub fn pilot_list(&self) -> Option<&PilotList> {
        self.open_pilot.as_ref()
    }

    /// The note that an opened pilot's load fell back, while it is open
    /// over the main menu.
    #[must_use]
    pub fn load_note(&self) -> Option<&NoteDialog> {
        self.load_note.as_ref()
    }

    /// Changes the pilot with `change` while the ship is landed, as a
    /// spaceport screen does, and says whether it did; the pilot is saved
    /// after the next input or tick.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.flight
            .as_mut()
            .is_some_and(|flight| flight.transact(change))
    }

    /// The pilot flying, as the developer tools' desk; `None` before
    /// flight is entered and when the flight could not start.
    pub fn pilot_desk(&mut self) -> Option<SessionDesk<'_, Rc<GameData>>> {
        self.flight.as_mut()?.pilot_desk()
    }

    /// Catches up with an edit made through the pilot desk: flight catches
    /// up at once with what the session told of it ([`FlightView::resync`],
    /// which its next frame would do anyway), laying out the system the
    /// pilot moved to or drawing the ship at the stellar it moved to; the
    /// open spaceport is rebuilt for the stellar the pilot is now landed
    /// on when it moved, settling at once so the session rolls the new
    /// stellar's offers and the spaceport asks its landing offer before
    /// the next frame, and otherwise shows the exchange, outfitter and
    /// shipyard as they now are; then the pilot is saved if a save is due,
    /// as after any change.
    pub fn after_pilot_edit(&mut self) {
        let Some(flight) = self.flight.as_mut() else {
            return;
        };
        flight.resync();
        let landed = flight.session().ok().and_then(Session::landed);
        if let Some(spaceport) = &mut self.spaceport {
            match landed {
                Some(stellar) if stellar != spaceport.stellar() => {
                    self.show_spaceport(stellar);
                    self.settle_script();
                }
                _ => refresh_spaceport(spaceport, flight),
            }
        }
        self.save_if_due();
    }

    /// The router with dialogs: I opens the About text in a dialog built
    /// from `dialogs`, its text laid out by `metrics`. Without dialogs, I
    /// does nothing.
    #[must_use]
    pub fn with_dialogs(
        self,
        dialogs: Rc<dyn DialogResources>,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        Self {
            dialogs: Some(Dialogs {
                resources: dialogs,
                metrics,
            }),
            ..self
        }
    }

    /// The router with the player's preferences, `prefs`, which the
    /// Preferences dialog opens on.
    #[must_use]
    pub fn with_prefs(self, prefs: Prefs) -> Self {
        self.movies.set_on(prefs.quicktime_movies);
        Self { prefs, ..self }
    }

    /// The router playing the mission texts' and offers' movies through
    /// `player`, while the preferences have them on.
    #[must_use]
    pub fn with_movie_player(self, player: Rc<dyn MoviePlayer>) -> Self {
        let movies = Movies::new(player, self.prefs.quicktime_movies);
        Self { movies, ..self }
    }

    /// The player's preferences, as last chosen.
    #[must_use]
    pub fn prefs(&self) -> Prefs {
        self.prefs
    }

    /// The About dialog, or the main menu's X text, while it is open.
    #[must_use]
    pub fn about(&self) -> Option<&DescDialog> {
        self.about.as_ref()
    }

    /// The Preferences dialog, while it is open.
    #[must_use]
    pub fn preferences(&self) -> Option<&PrefsDialog> {
        self.preferences.as_ref()
    }

    /// Which screen is showing.
    #[must_use]
    pub fn showing(&self) -> Showing {
        if self.about.is_some() {
            // The X text is no About box: it leaves the music playing.
            return if self.about_text == ABOUT_TEXT {
                Showing::About
            } else {
                Showing::Note
            };
        }
        if self.preferences.is_some() {
            return Showing::Preferences;
        }
        if self.player_info.is_some() {
            return Showing::PlayerInfo;
        }
        if self.haggle.is_some() {
            return Showing::Haggle;
        }
        if self.comm.is_some() {
            return Showing::Comm;
        }
        if let Some(missions) = &self.missions {
            if missions.playing() {
                return Showing::Movie;
            }
            return if missions.text().is_some() {
                Showing::MissionText
            } else {
                Showing::MissionInfo
            };
        }
        if let Some(offer) = &self.offer {
            return if offer.playing() {
                Showing::Movie
            } else {
                Showing::Offer
            };
        }
        if self.plunder.is_some() {
            return Showing::Plunder;
        }
        if self.assignment.is_some() {
            return Showing::Assignment;
        }
        if self.new_pilot.is_some() {
            return Showing::NewPilot;
        }
        if self.open_pilot.is_some() {
            return Showing::OpenPilot;
        }
        if self.load_note.is_some() {
            return Showing::Note;
        }
        match self.side {
            Side::MainMenu => Showing::MainMenu,
            Side::Ships => Showing::ShipBrowser,
            Side::Galaxy if self.galaxy.system().is_some() => Showing::System,
            Side::Galaxy => Showing::GalaxyMap,
            Side::Flight if self.flight.as_ref().is_some_and(FlightView::map_open) => {
                Showing::FlightMap
            }
            Side::Flight => Showing::Flight,
            Side::Spaceport if self.port_map_open() => Showing::FlightMap,
            Side::Spaceport => Showing::Spaceport,
        }
    }

    /// The input to flight's course map open over the spaceport, taking
    /// it all, and true; false, taking nothing, with no map open there. An
    /// Escape press closes it ([`FlightView::close_map`]), as in flight,
    /// and anything else goes to it ([`FlightView::port_map_input`]).
    fn port_map_input(&mut self, input: &Input) -> bool {
        if !self.port_map_open() {
            return false;
        }
        let flight = self.flight.as_mut().expect(ENTERED);
        let escape = Input::Key {
            key: Key::Escape,
            pressed: true,
            repeat: false,
        };
        if *input == escape {
            flight.close_map();
        } else {
            flight.port_map_input(input);
        }
        true
    }

    /// Whether flight's course map is open over the spaceport: the bar's
    /// map ([`FlightView::open_bar_map`]).
    fn port_map_open(&self) -> bool {
        self.side == Side::Spaceport && self.flight.as_ref().is_some_and(FlightView::map_open)
    }

    /// The ship browser, shown or not.
    #[must_use]
    pub fn ship_browser(&self) -> &ShipBrowser<Rc<GameData>> {
        &self.ships
    }

    /// The galaxy map, shown or not.
    #[must_use]
    pub fn galaxy_map(&self) -> &GalaxyMap {
        self.galaxy.map()
    }

    /// The open system's view, shown or not, if a system is open.
    #[must_use]
    pub fn system_view(&self) -> Option<&SystemView> {
        self.galaxy.system()
    }

    /// The flight screen, shown or not, once flight has been entered.
    #[must_use]
    pub fn flight_view(&self) -> Option<&FlightView<Rc<GameData>>> {
        self.flight.as_ref()
    }

    /// The spaceport, while the ship is landed.
    #[must_use]
    pub fn spaceport_view(&self) -> Option<&SpaceportView> {
        self.spaceport.as_ref()
    }

    /// The galaxy side: the map and any open system.
    #[must_use]
    pub fn navigator(&self) -> &Navigator<Rc<GameData>> {
        &self.galaxy
    }

    /// The screen of the side shown: flight's while its course map is
    /// open over the spaceport ([`AppScreen::port_map_open`]), so the
    /// spaceport neither ticks nor draws beneath it, paused as under any
    /// modal dialog. The bar's offer wait pausing under it too is ours'
    /// existing reading
    /// (rdm:phase/missions-fidelity/phase-15-offer-timer-under-nested-dialogs).
    fn shown(&self) -> &dyn Screen {
        match self.side {
            Side::MainMenu => &self.menu.as_ref().expect(MENU).screen,
            Side::Ships => &self.ships,
            Side::Galaxy => &self.galaxy,
            Side::Flight => self.flight.as_ref().expect(ENTERED),
            Side::Spaceport if self.port_map_open() => self.flight.as_ref().expect(ENTERED),
            Side::Spaceport => self.spaceport.as_ref().expect(LANDED),
        }
    }

    /// The screen of the side shown, as [`AppScreen::shown`] gives it.
    fn shown_mut(&mut self) -> &mut dyn Screen {
        let port_map = self.port_map_open();
        match self.side {
            Side::MainMenu => &mut self.menu.as_mut().expect(MENU).screen,
            Side::Ships => &mut self.ships,
            Side::Galaxy => &mut self.galaxy,
            Side::Flight => self.flight.as_mut().expect(ENTERED),
            Side::Spaceport if port_map => self.flight.as_mut().expect(ENTERED),
            Side::Spaceport => self.spaceport.as_mut().expect(LANDED),
        }
    }

    /// Hides the side shown, first cancelling its pointer gesture and
    /// letting go of its keys (their releases will go elsewhere), and shows
    /// `side`.
    fn switch_to(&mut self, side: Side) {
        let hidden = self.shown_mut();
        hidden.cancel_pointer();
        hidden.release_keys();
        self.side = side;
    }

    /// Shows flight, building it the first time, and remembers the side to
    /// go back to.
    fn enter_flight(&mut self) {
        if self.flight.is_none() {
            self.flight = Some(self.flight(FlightView::new));
        }
        self.return_to = self.side;
        self.switch_to(Side::Flight);
    }

    /// Opens the About dialog over the side shown, first cancelling its
    /// pointer gesture and letting go of its keys (their releases will go
    /// to the dialog). With no dialogs, nothing opens; when the dialog
    /// cannot be built, nothing opens and the reason goes to stderr.
    fn open_about(&mut self) {
        self.open_text(ABOUT_TEXT);
    }

    /// Opens "Desc Dialog" on `dësc` `id` over the side shown, as
    /// [`open_about`](Self::open_about) opens the About text. The About
    /// text and the main menu's X text ([`MENU_X_TEXT`]) are first loaded
    /// and munged as before any pilot is loaded ([`start_up_words`]), as
    /// `_DoAboutBox` and `_MainScreenKey` do before `_ShowTextDialog`.
    fn open_text(&mut self, id: i16) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let rules = nova_sim::text_parse::ParseRules::from_rulebook(&self.rules);
        let dialog = self.data.description(id).and_then(|text| {
            let text = if id == MENU_X_TEXT || id == ABOUT_TEXT {
                start_up_words(&text, &*self.data, rules)
            } else {
                text
            };
            desc_dialog(dialogs, &self.data, &text)
        });
        match dialog {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.about = Some(dialog);
                self.about_text = id;
            }
            Err(reason) => eprintln!("nova: cannot show dësc {id}: {reason}"),
        }
    }

    /// Opens the Preferences dialog over the side shown, on the player's
    /// preferences, first cancelling the side's pointer gesture and
    /// letting go of its keys. With no dialogs, nothing opens; when the
    /// dialog cannot be built, nothing opens and the reason goes to stderr.
    fn open_preferences(&mut self) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let dialog = dialogs
            .resources
            .dialog_template(PREFS_DIALOG)
            .and_then(|template| {
                PrefsDialog::new(
                    &template,
                    self.prefs,
                    self.data.button_style(),
                    Rc::clone(&dialogs.metrics),
                )
            });
        match dialog {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.preferences = Some(dialog);
            }
            Err(reason) => eprintln!("nova: cannot show the preferences: {reason}"),
        }
    }

    /// Turns double time on or off on a Caps Lock press, not on its
    /// repeats nor its release, and tells flight, if built; a flight
    /// built later starts with it ([`FlightView::with_double_time`]).
    ///
    /// The original follows Caps Lock's lock state (`_QuickKeyCheck(0x39)`
    /// @0x463c3); ours toggles on each press the platform reports, an
    /// accepted difference. It drifts from the lock when Caps Lock is
    /// pressed while the window is not focused, or is already on at
    /// launch. On macOS winit 0.30.13 reports no Caps Lock key at all, so
    /// there double time never comes on.
    fn double_time_key(&mut self, input: &Input) {
        if let Input::Key {
            key: Key::CapsLock,
            pressed: true,
            repeat: false,
        } = *input
        {
            self.double_time = !self.double_time;
            if let Some(flight) = &mut self.flight {
                flight.set_double_time(self.double_time);
            }
        }
    }

    /// The Preferences dialog's input: each change is kept, to be taken
    /// once, and the dialog closes once OK is activated.
    fn preferences_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(dialog) = &mut self.preferences {
            dialog.input(input);
            if let Some(prefs) = dialog.take_change() {
                self.prefs = prefs;
                self.prefs_change = Some(prefs);
                self.movies.set_on(prefs.quicktime_movies);
                if let Some(flight) = &mut self.flight {
                    flight.set_hyperspace_effects(prefs.hyperspace_effects);
                    flight.set_sound_prefs(prefs.sound);
                }
            }
            if dialog.closed() {
                self.sounds.extend(dialog.take_sounds());
                self.preferences = None;
            }
        }
        ScreenAction::None
    }

    /// The overlay open over the side shown, if any: the About dialog, the
    /// Preferences dialog, the New Pilot dialog, the saved pilots' list or
    /// the load's note.
    fn overlay_mut(&mut self) -> Option<&mut dyn Screen> {
        if let Some(about) = &mut self.about {
            return Some(about);
        }
        if let Some(dialog) = &mut self.preferences {
            return Some(dialog);
        }
        if let Some(screen) = &mut self.player_info {
            return Some(screen);
        }
        if let Some(dialog) = &mut self.haggle {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.comm {
            return Some(dialog);
        }
        if let Some(dialogs) = &mut self.missions {
            return Some(dialogs);
        }
        if let Some(dialog) = &mut self.offer {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.plunder {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.assignment {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.new_pilot {
            return Some(dialog);
        }
        if let Some(list) = &mut self.open_pilot {
            return Some(list);
        }
        self.load_note.as_mut().map(|note| note as &mut dyn Screen)
    }

    /// The About dialog's input; it closes once Done is activated.
    fn about_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(about) = &mut self.about {
            about.input(input);
            if about.closed() {
                self.sounds.extend(about.take_sounds());
                self.about = None;
            }
        }
        ScreenAction::None
    }

    /// Flight's input: an Escape press closes flight's map when it is
    /// open, and otherwise goes back; everything else goes to flight
    /// (where L, the stellar slot keys and a click select the navigation
    /// target). When it lands, the
    /// spaceport shows.
    fn flight_input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: Key::Escape,
            pressed,
            repeat,
        } = *input
        {
            if pressed && !repeat {
                let flight = self.flight.as_mut().expect(ENTERED);
                if flight.map_open() {
                    flight.close_map();
                } else if self.return_to == Side::MainMenu {
                    // The pilot is put away: saved, and flight dropped, so
                    // the developer's F flies a fresh, unnamed pilot.
                    self.end_flight(Side::MainMenu);
                } else {
                    self.switch_to(self.return_to);
                }
            }
            return ScreenAction::None;
        }
        let flight = self.flight.as_mut().expect(ENTERED);
        flight.input(input);
        if let Some(stellar) = flight.take_landing() {
            self.show_spaceport(stellar);
        }
        if let Some(shown) = self.flight.as_mut().and_then(FlightView::take_boarding) {
            self.open_plunder(&shown);
        }
        if let Some(view) = self.flight.as_mut().and_then(FlightView::take_hail) {
            self.open_comm(&view);
        }
        if let Some(prompt) = self.flight.as_mut().and_then(FlightView::take_offer) {
            self.open_offer(prompt);
        }
        if self
            .flight
            .as_mut()
            .is_some_and(FlightView::take_player_info_request)
        {
            self.open_flight_player_info();
        }
        if let Some(info) = self
            .flight
            .as_mut()
            .and_then(FlightView::take_mission_info_request)
            && let Some(missions) = self.flight_missions()
        {
            missions.open_info(info);
            self.load_flight_info_text();
        }
        self.drop_closed_missions();
        ScreenAction::None
    }

    /// The mission dialogs over flight, built from the interface file's
    /// "Desc Dialog" and mission info screen the first time they are
    /// needed; none without dialogs.
    fn flight_missions(&mut self) -> Option<&mut MissionDialogs> {
        if self.missions.is_none() {
            let dialogs = self.dialogs.as_ref()?;
            self.missions = Some(
                MissionDialogs::new(
                    DescTemplates {
                        plain: dialogs.resources.dialog_template(DESC_DIALOG),
                        picture: dialogs.resources.dialog_template(PICTURE_DESC_DIALOG),
                    },
                    dialogs.resources.dialog_template(INFO_DIALOG),
                    Rc::clone(&dialogs.metrics),
                    self.data.button_style(),
                )
                .with_movies(self.movies.clone()),
            );
        }
        self.missions.as_mut()
    }

    /// Tells flight of each mission text over flight closed or dropped
    /// since it was last told ([`MissionDialogs::take_texts_closed`]), so
    /// it lets go of the bar a person's offer answered over flight holds
    /// under them ([`FlightView::close_mission_text`]); then drops the
    /// mission dialogs over flight once nothing is open, keeping their
    /// sounds, and begins the death's fade if the hold has expired
    /// ([`AppScreen::fade_once_dialogs_close`]).
    ///
    /// What was pressed under the last movie is dropped: flight reads its
    /// keys as held, and the router lets go of them as the dialogs open.
    fn drop_closed_missions(&mut self) {
        if let (Some(missions), Some(flight)) = (&mut self.missions, &mut self.flight) {
            for _ in 0..missions.take_texts_closed() {
                flight.close_mission_text();
            }
        }
        if let Some(missions) = &mut self.missions
            && !missions.is_open()
        {
            missions.take_unclaimed();
            self.sounds.extend(missions.take_sounds());
            self.missions = None;
            // The offer dialog left drawn under them is disposed of
            // (`DisposeDialogNoFlash` @0xa361e).
            if self.offer.as_ref().is_some_and(OfferDialog::answered) {
                self.offer = None;
            }
            self.fade_once_dialogs_close();
        }
    }

    /// The input of the mission dialogs over flight: an Abort on the
    /// mission info screen goes through flight, and the screen shows the
    /// missions left. Its close, laid out, redraws the whole status bar
    /// ([`FlightView::refresh_status`]), as `_DoMissionInfoDialog` ends in
    /// `_RefreshSAT` (@0x9e769). They are dropped once nothing is open.
    fn missions_input(&mut self, input: &Input) -> ScreenAction {
        let missions = self.missions.as_mut().expect("open");
        missions.input(input);
        abort_and_refresh(missions, self.flight.as_mut());
        self.load_flight_info_text();
        if self
            .missions
            .as_mut()
            .is_some_and(MissionDialogs::take_info_closed_laid_out)
            && let Some(flight) = &mut self.flight
        {
            flight.refresh_status();
        }
        self.drop_closed_missions();
        ScreenAction::None
    }

    /// Opens the player info screen over flight, which pauses, and over
    /// any offer dialog open there, on what flight gives as the dialog
    /// opens ([`FlightView::open_player_info`], which resets the session
    /// first; nothing opens in a jump or while the ship breaks up): laid out by
    /// the interface file's `DLOG` 1017 and its Yes/No dialog, 3002, or
    /// saying why not. The screen below lets go of its keys. With no
    /// dialogs, nothing opens, as the mission info does not.
    fn open_flight_player_info(&mut self) {
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        let Some(info) = self.flight.as_mut().and_then(FlightView::open_player_info) else {
            return;
        };
        let layout = dialogs
            .resources
            .dialog_template(PLAYER_INFO_DIALOG)
            .map(|template| (template, Rc::clone(&dialogs.metrics)));
        let confirm = dialogs.resources.dialog_template(YES_NO_DIALOG);
        self.cancel_pointer();
        self.release_keys();
        self.player_info = Some(PlayerInfoScreen::new(
            layout,
            confirm,
            info,
            self.data.button_style(),
        ));
    }

    /// The player info screen's input over flight: a confirmed Jettison
    /// jettisons everything through flight. Once it closes, the screen
    /// below lets go of its keys, and, laid out, it redraws the whole
    /// status bar ([`FlightView::refresh_status`]), as
    /// `_DoPlayerInfoDialog` ends in `_RefreshSAT` (@0x5a725).
    fn player_info_input(&mut self, input: &Input) -> ScreenAction {
        let screen = self.player_info.as_mut().expect("open");
        screen.input(input);
        let jettison = screen.take_jettison();
        let closed = screen.closed();
        let laid_out = screen.problem().is_none();
        if jettison && let Some(flight) = &mut self.flight {
            flight.jettison_all();
        }
        if closed {
            self.player_info = None;
            self.release_keys();
            if laid_out && let Some(flight) = &mut self.flight {
                flight.refresh_status();
            }
        }
        ScreenAction::None
    }

    /// Opens the offer dialog on `prompt`, a person's mission offered on
    /// its hail or boarding, over flight, which pauses: the interface
    /// file's, in the variant its graphic picks
    /// ([`OfferTemplates::layout`]), or, without it or the metrics to lay
    /// it out, the dialog's own note that Return accepts it.
    fn open_offer(&mut self, prompt: OfferPrompt) {
        let layout = match (&self.dialogs, self.metrics()) {
            (Some(dialogs), Some(metrics)) => OfferTemplates {
                plain: dialogs.resources.dialog_template(OFFER_DIALOG),
                picture: dialogs.resources.dialog_template(PICTURE_OFFER_DIALOG),
            }
            .layout(prompt.graphic)
            .map(|template| (template, metrics)),
            _ => Err(NO_INTERFACE.to_owned()),
        };
        self.offer = Some(
            OfferDialog::new(layout, prompt, self.data.button_style()).with_movies(&self.movies),
        );
    }

    /// The offer dialog's input, then what it asked for
    /// ([`AppScreen::settle_offer`]).
    fn offer_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(dialog) = &mut self.offer {
            dialog.input(input);
        }
        self.settle_offer();
        ScreenAction::None
    }

    /// Acts on what the offer dialog over flight asked for, after an input
    /// or a tick (the keys pressed under its movie reach it as the movie
    /// ends, on a tick): the player info screen opens over it; or the
    /// choice goes through flight, and the dialog closes, keeping its
    /// sounds, and flight resumes. The texts the answer queued show over
    /// flight at once, the offer's after-movie among them where the
    /// session queued it.
    ///
    /// The dialog is hidden for an accept from the dialog with a refuse
    /// button (`_OfferOneMission` @0xa2df3) and for a refusal that showed
    /// a `RefuseText` (@0xa2e8e). Otherwise, the one-button dialog's accept
    /// (@0xa2f52-0xa2ff7) or a refusal with none, it stays drawn, answered
    /// and inert, under the mission dialogs the answer opened, until they
    /// have all closed (`DisposeDialogNoFlash` @0xa361e,
    /// [`AppScreen::drop_closed_missions`]).
    fn settle_offer(&mut self) {
        let Some(dialog) = &mut self.offer else {
            return;
        };
        if dialog.take_player_info_request() {
            self.open_flight_player_info();
            return;
        }
        let Some(choice) = dialog.take_choice() else {
            return;
        };
        let mission = dialog.prompt().mission;
        let two_buttons = dialog.prompt().labels.refuse.is_some();
        self.sounds.extend(dialog.take_sounds());
        let flight = self.flight.as_mut().expect(ENTERED);
        let hidden = match choice {
            OfferChoice::Accept => {
                let _ = flight.accept_mission(mission);
                two_buttons
            }
            OfferChoice::Refuse => flight
                .refuse_mission(mission)
                .is_ok_and(|shown| shown.offer_hidden),
        };
        if hidden {
            self.offer = None;
        }
        self.show_mission_texts();
        if self.missions.is_none() {
            self.offer = None;
        }
    }

    /// Opens the comm dialog on `view` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the hail is hung up, with a warning.
    fn open_comm(&mut self, view: &HailView) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.hang_up();
            }
            self.warnings
                .push("nova: cannot show the comm dialog: no metrics to lay it out".to_owned());
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(COMM_DIALOG)
                .and_then(|template| CommDialog::new(&template, view, style, Rc::clone(&metrics)))
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in comm dialog: {reason}"));
                }
                CommDialog::fallback(view, style, metrics)
            }
        };
        self.comm = Some(dialog);
    }

    /// The comm dialog's input: each option pressed goes through flight,
    /// and the dialog shows what came of it, opening the haggle dialog
    /// over it when a price is asked; Close Channel hangs up and resumes
    /// flight, keeping the dialog's sounds, as does a press with nothing
    /// to show (the ship gone).
    fn comm_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.comm.as_mut().expect("open");
        dialog.input(input);
        let Some(press) = dialog.take_press() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        let shown = match press {
            CommPress::Option(place) => flight.answer(place),
            CommPress::Close => None,
        };
        if let Some(view) = shown {
            if let Some(dialog) = &mut self.comm {
                dialog.set_hail(&view);
            }
            if let Some(price) = view.asking {
                self.open_haggle(price, view.pay_me);
            }
        } else {
            flight.hang_up();
            if let Some(mut dialog) = self.comm.take() {
                self.sounds.extend(dialog.take_sounds());
            }
        }
        ScreenAction::None
    }

    /// Opens the haggle dialog asking `price` over the comm dialog: the
    /// interface file's, or the built-in one without it.
    fn open_haggle(&mut self, price: i64, pay_me: bool) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(HAGGLE_DIALOG)
                .and_then(|template| {
                    HaggleDialog::new(&template, price, pay_me, style, Rc::clone(&metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in haggle dialog: {reason}"));
                }
                HaggleDialog::fallback(price, pay_me, style, metrics)
            }
        };
        self.haggle = Some(dialog);
    }

    /// The haggle dialog's input: the choice goes through flight; a price
    /// lowered is shown, and otherwise the dialog closes, keeping its
    /// sounds, and the comm dialog shows what came of it.
    fn haggle_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.haggle.as_mut().expect("open");
        dialog.input(input);
        let Some(choice) = dialog.take_choice() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        let shown = flight.haggle(choice);
        if let Some(price) = shown.as_ref().and_then(|view| view.asking) {
            let pay_me = shown.as_ref().is_none_or(|view| view.pay_me);
            if let Some(dialog) = &mut self.haggle {
                dialog.set_asking(price, pay_me);
            }
            return ScreenAction::None;
        }
        if let Some(mut dialog) = self.haggle.take() {
            self.sounds.extend(dialog.take_sounds());
        }
        if let Some(view) = shown.or_else(|| flight.hailing()) {
            if let Some(dialog) = &mut self.comm {
                dialog.set_hail(&view);
            }
        } else {
            flight.hang_up();
            if let Some(mut dialog) = self.comm.take() {
                self.sounds.extend(dialog.take_sounds());
            }
        }
        ScreenAction::None
    }

    /// Opens the plunder dialog on `shown` over flight, which pauses: the
    /// interface file's, or the built-in one without it (with a warning
    /// when the interface file has none). With no metrics to lay it out,
    /// the boarding is let go, with a warning.
    fn open_plunder(&mut self, shown: &PlunderShown) {
        let Some(metrics) = self.metrics() else {
            if let Some(flight) = &mut self.flight {
                flight.plunder(Take::Abort);
            }
            self.warnings
                .push("nova: cannot show the plunder dialog: no metrics to lay it out".to_owned());
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(PLUNDER_DIALOG)
                .and_then(|template| {
                    PlunderDialog::new(&template, shown, style, Rc::clone(&metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings
                        .push(format!("nova: using the built-in plunder dialog: {reason}"));
                }
                PlunderDialog::fallback(shown, style, metrics)
            }
        };
        self.plunder = Some(dialog);
    }

    /// The plunder dialog's input: each press goes through flight, and
    /// the dialog shows what is still on board, or closes once the
    /// boarding is over; a capture awaiting its assignment opens the
    /// assignment dialog.
    fn plunder_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.plunder.as_mut().expect("open");
        dialog.input(input);
        let Some(take) = dialog.take_take() else {
            return ScreenAction::None;
        };
        let flight = self.flight.as_mut().expect(ENTERED);
        let taken = flight.plunder(take);
        match flight.plunder_shown() {
            Some(shown) => {
                if let Some(dialog) = &mut self.plunder {
                    dialog.set_plunder(&shown);
                }
            }
            None => {
                if let Some(mut dialog) = self.plunder.take() {
                    self.sounds.extend(dialog.take_sounds());
                }
            }
        }
        if taken == Taken::Captured {
            self.open_assignment();
        }
        ScreenAction::None
    }

    /// Opens the captured-ship assignment dialog over flight: the
    /// interface file's, or the built-in one without it.
    fn open_assignment(&mut self) {
        let Some(metrics) = self.metrics() else {
            return;
        };
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(ASSIGNMENT_DIALOG)
                .and_then(|template| AssignmentDialog::new(&template, style, Rc::clone(&metrics)))
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in assignment dialog: {reason}"
                    ));
                }
                AssignmentDialog::fallback(style, metrics)
            }
        };
        self.assignment = Some(dialog);
    }

    /// The assignment dialog's input: the choice goes through flight, and
    /// the dialog closes, keeping its sounds.
    fn assignment_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.assignment.as_mut().expect("open");
        dialog.input(input);
        if let Some(choice) = dialog.take_choice() {
            self.sounds.extend(dialog.take_sounds());
            self.assignment = None;
            self.flight.as_mut().expect(ENTERED).assign(choice);
        }
        ScreenAction::None
    }

    /// Shows the spaceport of `stellar`, landed on, laid out by the
    /// "Spaceport" dialog when the router has dialogs, with the session's
    /// exchange and outfitter when it has them, its ambient sound's waits
    /// drawn on the router's chance. One already open, for another
    /// stellar the pilot desk moved to, is dropped first
    /// ([`AppScreen::drop_spaceport`]).
    fn show_spaceport(&mut self, stellar: nova_sim::StellarId) {
        let template = |id| match &self.dialogs {
            Some(dialogs) => dialogs
                .resources
                .dialog_template(id)
                .map(|template| (template, Rc::clone(&dialogs.metrics))),
            None => Err(NO_INTERFACE.to_owned()),
        };
        let mut spaceport =
            SpaceportView::new(self.data.as_ref(), stellar, template(SPACEPORT_DIALOG));
        if let Some(market) = self.flight.as_ref().and_then(FlightView::market) {
            let trade = template(TRADE_DIALOG).map(|(template, _)| template);
            let quantity = template(QUANTITY_DIALOG).map(|(template, _)| template);
            spaceport = spaceport.with_trade(trade, quantity, market);
        }
        if let Some(outfitter) = self.flight.as_mut().and_then(FlightView::outfitter) {
            let outfit = template(OUTFIT_DIALOG).map(|(template, _)| template);
            let quantity = template(QUANTITY_DIALOG).map(|(template, _)| template);
            let art: Rc<dyn OutfitterCatalog> = Rc::clone(&self.data) as Rc<dyn OutfitterCatalog>;
            let alert = template(DESC_DIALOG).map(|(template, _)| template);
            spaceport = spaceport.with_outfitter(outfit, quantity, alert, outfitter, art);
        }
        let template_of = |id| template(id).map(|(template, _)| template);
        let info_templates = || InfoTemplates {
            plain: template_of(SHIP_INFO_DIALOG),
            picture: template_of(SHIP_PICTURE_INFO_DIALOG),
        };
        if let Some(shipyard) = self.flight.as_mut().and_then(FlightView::shipyard) {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport.with_shipyard(
                template_of(SHIPYARD_DIALOG),
                info_templates(),
                template_of(TEXT_INPUT_DIALOG),
                shipyard,
                art,
            );
        }
        if let Some(pilot) = self.flight.as_ref().and_then(FlightView::pilot) {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport
                .with_bar(
                    BarTemplates {
                        plain: template_of(BAR_DIALOG),
                        picture: template_of(PICTURE_BAR_DIALOG),
                    },
                    template_of(NEWS_DIALOG),
                    template_of(SHIPYARD_DIALOG),
                    info_templates(),
                    art,
                    pilot.escort_count() < MAX_ESCORTS,
                )
                .with_missions(
                    template_of(COMPUTER_DIALOG),
                    template_of(INFO_DIALOG),
                    OfferTemplates {
                        plain: template_of(OFFER_DIALOG),
                        picture: template_of(PICTURE_OFFER_DIALOG),
                    },
                    DescTemplates {
                        plain: template_of(DESC_DIALOG),
                        picture: template_of(PICTURE_DESC_DIALOG),
                    },
                );
        }
        spaceport = spaceport
            .with_player_info(template_of(PLAYER_INFO_DIALOG), template_of(YES_NO_DIALOG))
            .with_offer_clock(self.rules.source_for(RuleKey::OfferClock))
            .with_movies(self.movies.clone())
            .with_chance(self.chance.clone());
        self.drop_spaceport();
        self.spaceport = Some(spaceport);
        self.serve_descriptions();
        self.switch_to(Side::Spaceport);
    }

    /// Flies `pilot`, from the main menu, in place of any flight; Escape
    /// goes back to the menu. A pilot last landed on a stellar resumes in
    /// flight over it ([`FlightView::with_pilot`]).
    fn start_flight(&mut self, pilot: Pilot) {
        self.build_flight(pilot, false);
        self.enter_built_flight();
    }

    /// Builds `pilot`'s flight in place of any flight, without showing
    /// it: one `loaded` from a save reads its load's fallbacks from the
    /// router's rules ([`FlightView::loaded`]), a new one is flown as it
    /// is.
    fn build_flight(&mut self, pilot: Pilot, loaded: bool) {
        self.drop_spaceport();
        self.put_away_flight();
        let rules = self.rules;
        let flight = self.flight(|data| {
            if loaded {
                FlightView::loaded(data, pilot, rules)
            } else {
                FlightView::with_pilot(data, pilot)
            }
        });
        self.flight = Some(flight);
    }

    /// Shows the flight just built, from the main menu, which Escape goes
    /// back to: in space, as `_EnterGameFromMainScreen` and `_PlayGame`
    /// land nothing.
    fn enter_built_flight(&mut self) {
        self.return_to = Side::MainMenu;
        self.switch_to(Side::Flight);
    }

    /// Puts the pilot away and shows `side`: saves it (unless it may not
    /// be saved), or deletes a strict pilot dying
    /// ([`AppScreen::put_away_pilot`]), before anything can show the menu
    /// or list the pilots, and drops flight, the spaceport and any dialog
    /// open over flight, so nothing is left to re-enter.
    fn end_flight(&mut self, side: Side) {
        self.put_away_pilot();
        self.switch_to(side);
        self.put_away_flight();
        self.drop_spaceport();
        self.plunder = None;
        self.assignment = None;
        self.comm = None;
        self.haggle = None;
        self.offer = None;
        self.missions = None;
        self.player_info = None;
    }

    /// The seam at the expiry of the hold after the player's death, before
    /// the original's `gPause = 1` (0x68838): the `dësc` of each of a
    /// sabotaged player's outfits shows over the stopped flight, one
    /// after another (`_HandlePlayer` @0x68739-0x68832,
    /// [`FlightView::sabotage_death_descs`]), and the fade
    /// ([`FlightView::begin_fade`]) begins once the last closes
    /// ([`AppScreen::fade_once_dialogs_close`]). With none to show, no
    /// dialogs or no "Desc Dialog", it begins at once, on the same tick.
    ///
    /// Each is loaded through flight as `_LoadDescription` (@0x687c9)
    /// loads it ([`FlightView::load_description`]). A `dësc` that cannot
    /// be read is skipped with a warning; what the original does with a missing one
    /// was not checked. Mission texts the death's tick queued share the
    /// queue, after these, and the fade waits for them too.
    ///
    /// The dialogs open in the tick the hold expires, before its frame is
    /// drawn, and the fade begins on the input that closes the last, so
    /// no boundary the player sees lies between them and the original's
    /// order.
    fn at_hold_expiry(&mut self) {
        let Some(flight) = &mut self.flight else {
            return;
        };
        let mut texts = Vec::new();
        for id in flight.sabotage_death_descs() {
            match flight.load_description(id) {
                Ok(text) => texts.push(DescShown {
                    text,
                    graphic: self.data.description_graphic(id),
                    movie: self.data.description_movie(id),
                }),
                Err(reason) => self.warnings.push(format!(
                    "nova: cannot show the sabotage dësc {id}: {reason}"
                )),
            }
        }
        if !texts.is_empty() {
            self.show_over_flight(texts);
        }
        self.fade_once_dialogs_close();
    }

    /// Shows the escape pod's rescue's `dësc` `id` over flight, as
    /// `_HandlePlayer`'s `_ShowTextDialog` does (@0x68e80-0x68eac), in the
    /// variant its `Graphic` picks, loaded through flight as
    /// `_LoadDescription` (@0x68e90) loads it
    /// ([`FlightView::load_description`]); one that cannot be read is a
    /// warning. The rescue settled before it, and flight goes on once it closes: no
    /// fade follows, unlike the death's.
    fn show_rescue_desc(&mut self, id: i16) {
        let loaded = match &mut self.flight {
            Some(flight) => flight.load_description(id),
            None => self.data.description(id),
        };
        match loaded {
            Ok(text) => {
                let graphic = self.data.description_graphic(id);
                self.show_over_flight(vec![DescShown {
                    text,
                    graphic,
                    movie: self.data.description_movie(id),
                }]);
            }
            Err(reason) => self
                .warnings
                .push(format!("nova: cannot show the rescue dësc {id}: {reason}")),
        }
    }

    /// Begins the death's fade once no dialog is open over flight: at
    /// the hold's expiry with none to show, or as the last closes.
    /// Before the hold has expired [`FlightView::begin_fade`] does
    /// nothing, so a dialog closing in live flight leaves flight as it is.
    fn fade_once_dialogs_close(&mut self) {
        if self.missions.is_none()
            && let Some(flight) = &mut self.flight
        {
            flight.begin_fade();
        }
    }

    /// Takes what flight's tick ended: at the hold's expiry, the seam
    /// ([`AppScreen::at_hold_expiry`]); once the death's fade is over, the
    /// return to the main menu (or the side flight was entered from), which
    /// fades in from black.
    fn after_flight_tick(&mut self) {
        if self
            .flight
            .as_mut()
            .is_some_and(FlightView::take_hold_expired)
        {
            self.at_hold_expiry();
        }
        if let Some(id) = self.flight.as_mut().and_then(FlightView::take_rescue_desc) {
            self.show_rescue_desc(id);
        }
        if self.flight.as_ref().is_some_and(FlightView::game_over) {
            let side = if self.menu.is_some() {
                Side::MainMenu
            } else {
                self.return_to
            };
            self.end_flight(side);
            self.fade_in = Some(ScreenFade::from_black(DEATH_FADE_FOR));
        }
    }

    /// Moves the fade back from black on by `dt`, letting go of it once it
    /// is over.
    fn advance_fade_in(&mut self, dt: Duration) {
        if let Some(fade) = &mut self.fade_in {
            fade.advance(dt);
            if fade.done() {
                self.fade_in = None;
            }
        }
    }

    /// Drops the flight, if any, keeping its ship slots' draw-layer and
    /// cloned-frame history for the next flight in this run.
    fn put_away_flight(&mut self) {
        if let Some(slot_sprites) = self
            .flight
            .take()
            .as_ref()
            .and_then(FlightView::slot_sprites)
        {
            self.slot_sprites = slot_sprites.clone();
        }
    }

    /// Where pilots are kept, if anywhere.
    fn keeper(&self) -> Option<Rc<PilotKeeper<Box<dyn PilotStore>>>> {
        self.menu.as_ref().and_then(|menu| menu.pilots.clone())
    }

    /// Puts the pilot flying away as play ends: deletes it, when there
    /// is somewhere it is kept and it is a strict pilot dying outside the
    /// escape pod ([`FlightView::deletes_pilot_on_ending_play`]), as
    /// `_EnterGameFromMainScreen` does once `_PlayGame` returns
    /// (@0x1a4ab-0x1a4e1), and otherwise saves it
    /// ([`AppScreen::save_pilot`]). A failed delete becomes a warning.
    /// Nothing writes the pilot back afterwards: a dying pilot may not be
    /// saved.
    fn put_away_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
        if !flight.deletes_pilot_on_ending_play() {
            self.save_pilot();
            return;
        }
        let Some(pilot) = flight.pilot() else {
            return;
        };
        if let Err(warning) = keeper.delete(pilot.name()) {
            self.warnings.push(warning);
        }
    }

    /// Saves the pilot flying, when it has a name, there is somewhere to
    /// keep it and it may be saved ([`FlightView::may_save`]: not while its
    /// ship breaks up or after it is destroyed); a failure becomes a
    /// warning. Pilots without a name, which the developer's F flies, are
    /// never saved.
    fn save_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
        if !flight.may_save() {
            return;
        }
        let Some(pilot) = flight.pilot() else {
            return;
        };
        if pilot_key(pilot.name()).is_none() {
            return;
        }
        if let Err(warning) = keeper.save(pilot) {
            self.warnings.push(warning);
        }
    }

    /// Settles what the set expressions run in flight or in the spaceport
    /// queued ([`FlightView::settle_script`]); a move while landed leaves
    /// the spaceport open as it is. The mission texts queued show over
    /// it, and then the venues' offers are served: a landing rolls the
    /// visit's offers as it settles, so the spaceport's offer is asked for
    /// here, after the texts the landing queued, as `_DoPortDialog` offers
    /// after `_MissionLandCheck` (@0x5fbeb-0x5fc37), and before any input
    /// can open a service. Last, a `Q` run while landed leaves, once
    /// nothing shown holds it ([`AppScreen::leave_if_due`]).
    fn settle_script(&mut self) {
        if let Some(flight) = self.flight.as_mut() {
            flight.settle_script();
        }
        self.show_mission_texts();
        self.serve_offers();
        self.leave_if_due();
    }

    /// Leaves the spaceport for a `Q` run while landed
    /// ([`FlightView::leave_due`]), as its Leave does
    /// ([`AppScreen::leave_port`]), unless a text or dialog shown over it
    /// holds the leave ([`SpaceportView::holds_leave`]): the router keeps
    /// only that gate, and the session carries the leave out
    /// ([`FlightView::leave_if_due`]). The original's
    /// port filters leave on their next call while `_absquatulateStr` is
    /// set (`_PortFilter` @0x4f047-0x4f063, which chooses Leave,
    /// @0x4f1ff), and the texts and dialogs over them never read it. As
    /// this runs after every input and tick, the input that closes the
    /// last of them leaves before a frame draws the spaceport bare. With
    /// no spaceport open, the ship leaves at once.
    fn leave_if_due(&mut self) {
        if !self.flight.as_ref().is_some_and(FlightView::leave_due) {
            return;
        }
        if self
            .spaceport
            .as_ref()
            .is_some_and(SpaceportView::holds_leave)
        {
            return;
        }
        self.leave_port_by(FlightView::leave_if_due);
    }

    /// Leaves the spaceport by its Leave ([`FlightView::take_off`]).
    fn leave_port(&mut self) {
        self.leave_port_by(FlightView::take_off);
    }

    /// Leaves the spaceport: it closes and the ship takes off by `leave`,
    /// the one path for the port's Leave and a `Q`'s, as `_DoPortDialog`
    /// has one (@0x5fcd3, @0x601a0-0x6025e). The session settles the
    /// take-off's events (the stay's `DayPassed`, `Explored`, `TookOff`)
    /// at once itself ([`Session::leave_port`](nova_sim::Session::leave_port)),
    /// and the mission texts they queue show over flight, so on either
    /// path they are settled before the take-off's save and its first
    /// frame, as `_PlayerLandOnStellar` runs them inside the take-off.
    fn leave_port_by(
        &mut self,
        leave: impl FnOnce(&mut FlightView<Rc<GameData>>) -> Option<nova_sim::StellarId>,
    ) {
        self.close_spaceport();
        if let Some(flight) = self.flight.as_mut() {
            let _ = leave(flight);
        }
        self.show_mission_texts();
    }

    /// Shows the mission texts the session queued: over the spaceport, if
    /// it is open, even when a `Q` has made a leave due, as the leave
    /// waits for them ([`AppScreen::leave_if_due`]); and otherwise, in
    /// flight, over flight, which pauses
    /// under them, as `_ShowTextDialog` (@0x5a752) shows them (a
    /// `BriefText` an `S` accepted, the texts of a person's offer
    /// accepted or refused over flight), first cancelling flight's
    /// pointer gesture and letting go of its keys (their releases will go
    /// to the texts), as any dialog over flight does; flight hears of each
    /// as it closes or is dropped ([`AppScreen::drop_closed_missions`]).
    /// Without dialogs, those in flight are dropped, as the spaceport
    /// drops them without its template, and flight hears of each text
    /// dropped at once, as the mission dialogs count a dropped text
    /// closed: the offer dialog still shows without dialogs, by its
    /// fallback layout, so a person's offer answered over flight holds
    /// the bar under its texts until then
    /// ([`FlightView::close_mission_text`]). On any other side they stay
    /// queued in the session.
    /// An offer's movies come in the places the session queued them.
    /// Over the spaceport, the texts it closed or dropped are told to
    /// flight first and last (`close_mission_texts`).
    fn show_mission_texts(&mut self) {
        if let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) {
            close_mission_texts(spaceport, flight);
            spaceport.show_mission_texts(flight.take_mission_texts());
            close_mission_texts(spaceport, flight);
            return;
        }
        if self.side != Side::Flight {
            return;
        }
        let Some(texts) = self
            .flight
            .as_mut()
            .map(FlightView::take_mission_texts)
            .filter(|texts| !texts.is_empty())
        else {
            return;
        };
        if self.flight_missions().is_none() {
            let flight = self.flight.as_mut().expect("texts came from flight");
            for _ in texts
                .iter()
                .filter(|shown| matches!(shown, nova_sim::MissionShown::Text(_)))
            {
                flight.close_mission_text();
            }
            return;
        }
        self.over_flight(|missions| missions.show_texts(texts));
    }

    /// Shows `texts`, each a text and its `dësc`'s `Graphic`, over flight
    /// in "Desc Dialog" of the variant each picks, one after another, as
    /// [`AppScreen::over_flight`] shows them. Without dialogs they are
    /// dropped.
    fn show_over_flight(&mut self, texts: Vec<DescShown>) {
        self.over_flight(|missions| missions.show_descs(texts));
    }

    /// Queues what `show` queues on the mission dialogs over flight, first
    /// cancelling flight's pointer gesture and letting go of its keys
    /// (their releases will go to the dialogs), as any dialog over flight
    /// does. Without dialogs nothing is queued.
    fn over_flight(&mut self, show: impl FnOnce(&mut MissionDialogs)) {
        if self.flight_missions().is_none() {
            return;
        }
        let below = self.shown_mut();
        below.cancel_pointer();
        below.release_keys();
        if let Some(missions) = &mut self.missions {
            show(missions);
        }
        self.drop_closed_missions();
    }

    /// Closes the spaceport, if it is open, and shows flight, letting go
    /// of the spaceport's keys, then drops it
    /// ([`AppScreen::drop_spaceport`]).
    fn close_spaceport(&mut self) {
        if self.spaceport.is_none() {
            return;
        }
        self.switch_to(Side::Flight);
        self.drop_spaceport();
    }

    /// Drops the spaceport, if it is open, keeping its sounds and its
    /// ambient sound's halt ([`SpaceportView::close_sounds`]): every way
    /// the spaceport goes comes through here, Leave, a script's take-off,
    /// the pilot put away or replaced, or the pilot desk moving it, as
    /// `_DoPortDialog` halts `_planetSnd` however its loop ends
    /// (@0x601d2-0x601eb). A move while landed keeps the spaceport open,
    /// and its sound with it.
    fn drop_spaceport(&mut self) {
        if let Some(mut spaceport) = self.spaceport.take() {
            self.sounds.extend(spaceport.close_sounds());
        }
    }

    /// Saves the pilot if flight says a save is due: it has landed, taken
    /// off or changed in the spaceport.
    fn save_if_due(&mut self) {
        if self.flight.as_mut().is_some_and(FlightView::take_save_due) {
            self.save_pilot();
        }
    }

    /// The main menu's input, as `_MainScreenKey` @0x1aba0 and
    /// `_MainScreenMouse` read it.
    ///
    /// A click on New Pilot or Open Pilot opens the New Pilot dialog or
    /// the saved pilots' list over the menu at once (its sounds have
    /// played, under the pointer). A click on Quit waits out
    /// [`QUIT_DELAY`] and then quits ([`Screen::take_quit`]), as
    /// `_MainScreenMouse` (@0x1aea4-0x1aead) reaches the same
    /// `_MainScreenChoice(2)` as the key. The engine's Escape quits at
    /// once, silently. Tab goes to the ship browser.
    ///
    /// Each key clicks ([`UiSound::ButtonKeyed`]) and its choice waits
    /// for the click to end ([`MenuHold`]): typed N, O and Q (the menu's
    /// own), and, with dialogs, A (the About text, @0x1ac70) and X
    /// (`dësc` 32766, @0x1ac7e, munged as before any pilot). P is the
    /// router's ([`preferences_key`](Self::preferences_key)).
    ///
    /// - O does nothing while the physical S or L key is held
    ///   (@0x1abd3-0x1abf5). No other code reads those two keys, so the
    ///   guard's purpose is unknown; it is built as read.
    /// - Typed E with the physical E key held waits for its release
    ///   (@0x1ac32-0x1ac57) before it clicks; typed with the E key up
    ///   (another layout) it clicks at once. Then it is Enter Ship
    ///   (`_MainScreenChoice(3)`), which this menu, never holding a
    ///   loaded pilot, always refuses: [`UiSound::EnterRefused`]. Choice
    ///   3's entering the game and its dying-pilot refusal are left out
    ///   (rdm:task/main-screen-loaded-pilot).
    ///
    /// With Command held, the typed keys and A and X go to the
    /// original's menus and do nothing here.
    fn menu_input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key {
            key: key @ Key::Char('a' | 'x'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
            && !self.command_held
        {
            if pressed && !repeat {
                self.click_then(if key == Key::Char('a') {
                    MenuAction::About
                } else {
                    MenuAction::Text
                });
            }
            return ScreenAction::None;
        }
        if let Input::Key {
            key: Key::Tab,
            pressed,
            repeat,
        } = *input
        {
            if pressed && !repeat {
                self.switch_to(Side::Ships);
            }
            return ScreenAction::None;
        }
        // A Command key goes to the original's menus (`MenuKey`, offered
        // first by `_DoMainScreen` @0x1b259-0x1b2bb), never to
        // `_MainScreenKey`, so it neither clicks nor chooses.
        if matches!(input, Input::Text(_)) && self.command_held {
            return ScreenAction::None;
        }
        match *input {
            Input::Text('e' | 'E') => {
                if self.keys_held.e {
                    self.menu_hold = Some(MenuHold::EHeld);
                } else {
                    self.click_then(MenuAction::Enter);
                }
                return ScreenAction::None;
            }
            Input::Text('o' | 'O') if self.keys_held.s || self.keys_held.l => {
                return ScreenAction::None;
            }
            _ => {}
        }
        let menu = self.menu.as_mut().expect(MENU);
        menu.screen.input(input);
        match menu.screen.take_choice() {
            Some(Chosen {
                choice,
                by: ChosenBy::Shortcut,
            }) => self.menu_hold = Some(MenuHold::Click(MenuAction::Choice(choice))),
            Some(Chosen {
                choice: MenuChoice::Quit,
                by: ChosenBy::Escape,
            }) => return ScreenAction::Quit,
            Some(Chosen { choice, .. }) => self.run_menu_action(MenuAction::Choice(choice)),
            None => {}
        }
        ScreenAction::None
    }

    /// Clicks a main-menu key ([`UiSound::ButtonKeyed`]) and holds
    /// `action` until the click has played.
    fn click_then(&mut self, action: MenuAction) {
        self.sounds.push(Sound::Ui(UiSound::ButtonKeyed));
        self.menu_hold = Some(MenuHold::Click(action));
    }

    /// Runs a main-menu `action`, as `_MainScreenChoice` @0x1a72a does.
    fn run_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Choice(MenuChoice::NewPilot) => self.open_new_pilot(),
            MenuAction::Choice(MenuChoice::OpenPilot) => self.open_pilot_list(),
            MenuAction::Choice(MenuChoice::Quit) => self.quit_in = Some(QUIT_DELAY),
            // `_MainScreenChoice(3)` with no game in progress beeps
            // `_beepSnd[3]` and redraws (@0x1aa89-0x1aac2).
            MenuAction::Enter => self.sounds.push(Sound::Ui(UiSound::EnterRefused)),
            MenuAction::Preferences => self.open_preferences(),
            MenuAction::About => self.open_text(ABOUT_TEXT),
            MenuAction::Text => self.open_text(MENU_X_TEXT),
        }
    }

    /// An input that arrives while the main menu waits ([`MenuHold`]):
    /// queued for the replay, but for E's own while it is held. E's
    /// release clicks and turns the wait into the click's; E's repeats
    /// and their text are dropped (whether the OS queues autoKeys during
    /// the original's spin is not read, and replaying them would type E
    /// again).
    fn hold_input(&mut self, hold: MenuHold, input: &Input) {
        if hold == MenuHold::EHeld {
            match *input {
                Input::Key {
                    key: Key::Char('e'),
                    pressed: false,
                    ..
                } => {
                    self.e_released();
                    return;
                }
                Input::Key {
                    key: Key::Char('e'),
                    pressed: true,
                    ..
                }
                | Input::Text('e' | 'E') => return,
                _ => {}
            }
        }
        self.held.push(Held::Input(*input));
    }

    /// E is let go of while E's wait holds: it clicks, and Enter Ship
    /// waits on the click.
    fn e_released(&mut self) {
        self.keys_held.e = false;
        self.click_then(MenuAction::Enter);
    }

    /// Opens the New Pilot dialog: the interface file's, or the built-in
    /// one without it (with a warning when the interface file has none).
    fn open_new_pilot(&mut self) {
        let style = self.data.button_style();
        let built = self.dialogs.as_ref().map(|dialogs| {
            dialogs
                .resources
                .dialog_template(NEW_PILOT_DIALOG)
                .and_then(|template| {
                    NewPilotDialog::new(&template, style, Rc::clone(&dialogs.metrics))
                })
        });
        let dialog = match built {
            Some(Ok(dialog)) => dialog,
            other => {
                if let Some(Err(reason)) = other {
                    self.warnings.push(format!(
                        "nova: using the built-in New Pilot dialog: {reason}"
                    ));
                }
                let metrics = Rc::clone(&self.menu.as_ref().expect(MENU).metrics);
                NewPilotDialog::fallback(style, metrics)
            }
        };
        // Chosen by a click let go of on its button, the menu below holds
        // no press and no keys to let go of.
        self.new_pilot = Some(dialog);
    }

    /// Opens the saved pilots' list; when they cannot be listed, it shows
    /// why, and so does a warning.
    fn open_pilot_list(&mut self) {
        let (keys, error) = match self.keeper().map(|keeper| keeper.list()) {
            Some(Ok(keys)) => (keys, None),
            Some(Err(error)) => (Vec::new(), Some(error)),
            None => (Vec::new(), None),
        };
        let metrics = Rc::clone(&self.menu.as_ref().expect(MENU).metrics);
        let mut list = PilotList::new(keys, self.data.button_style(), metrics);
        if let Some(error) = error {
            list.show_error(&error);
            self.warnings.push(error);
        }
        self.open_pilot = Some(list);
    }

    /// The New Pilot dialog's input. A name already saved, or one no file
    /// can be saved under, is refused; any other creates the pilot with the
    /// gender and Strict Play chosen and flies it, begins its game (the `chär`'s `OnStart`,
    /// [`FlightView::begin`]) and saves it at once, with what that did (so
    /// Open Pilot lists it before it lands), as `_DoNewPilot` runs
    /// `OnStart` before the first save.
    fn new_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.new_pilot.as_mut().expect("open");
        dialog.input(input);
        let outcome = dialog.take_outcome();
        match outcome {
            Some(NewPilotOutcome::Cancel) => self.close_new_pilot(),
            Some(NewPilotOutcome::Create {
                name,
                gender,
                strict_play,
            }) => {
                let keeper = self.keeper();
                let refusal = if pilot_key(&name).is_none() {
                    Some(UNUSABLE_NAME.to_owned())
                } else if keeper.as_ref().is_some_and(|keeper| keeper.exists(&name)) {
                    Some(NAME_TAKEN.to_owned())
                } else {
                    None
                };
                if let Some(refusal) = refusal {
                    self.new_pilot.as_mut().expect("open").refuse(&refusal);
                    return ScreenAction::None;
                }
                match Pilot::new(self.data.as_ref(), &name) {
                    Ok(pilot) => {
                        self.close_new_pilot();
                        self.start_flight(pilot.with_gender(gender).with_strict_play(strict_play));
                        if let Some(flight) = &mut self.flight {
                            flight.begin();
                            flight.take_save_due();
                        }
                        self.save_pilot();
                    }
                    Err(error) => {
                        let why = error.to_string();
                        self.new_pilot.as_mut().expect("open").refuse(&why);
                        self.warnings
                            .push(format!("nova: cannot create a pilot: {why}"));
                    }
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// Closes the New Pilot dialog, keeping its sounds.
    fn close_new_pilot(&mut self) {
        if let Some(mut dialog) = self.new_pilot.take() {
            self.sounds.extend(dialog.take_sounds());
        }
    }

    /// The saved pilots' list's input: the pilot chosen is opened and
    /// flown; one that cannot be opened says why in the list, and in a
    /// warning. One whose load fell back ([`FlightView::load_repaired`])
    /// has its flight built, faced and kept behind the note `STR#` 140 #52
    /// over the main menu, as `_MainScreenChoice` shows it through
    /// `_EVNoteAlert` over the main screen (@0x1a817-0x1a85b), so the note
    /// is seen before any flight frame; flight shows once it is dismissed
    /// ([`AppScreen::load_note_input`]). Without dialogs, or with no
    /// note to build, the note is a warning and flight shows at once.
    fn open_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let list = self.open_pilot.as_mut().expect("open");
        list.input(input);
        match list.take_outcome() {
            Some(PilotListOutcome::Cancel) => self.close_pilot_list(),
            Some(PilotListOutcome::Open(key)) => {
                let opened = match self.keeper() {
                    Some(keeper) => keeper.open(&key),
                    None => Err(format!(
                        "nova: there is nowhere to open the pilot {key} from"
                    )),
                };
                match opened {
                    Ok(pilot) => {
                        self.close_pilot_list();
                        self.build_flight(pilot, true);
                        // The load's heading, before any frame is drawn.
                        if let Some(flight) = &mut self.flight {
                            flight.resume();
                        }
                        if self.flight.as_ref().is_some_and(FlightView::load_repaired) {
                            match self.load_note_dialog() {
                                Some(note) => {
                                    let menu = self.shown_mut();
                                    menu.cancel_pointer();
                                    menu.release_keys();
                                    self.load_note = Some(note);
                                    return ScreenAction::None;
                                }
                                None => self.warnings.push(LOAD_FALLBACK_NOTE.to_owned()),
                            }
                        }
                        self.enter_built_flight();
                    }
                    Err(error) => {
                        self.open_pilot.as_mut().expect("open").show_error(&error);
                        self.warnings.push(error);
                    }
                }
            }
            None => {}
        }
        ScreenAction::None
    }

    /// The note `STR#` 140 #52 in the note alert (`DLOG` 3000), if the
    /// router has dialogs and the alert can be built.
    fn load_note_dialog(&self) -> Option<NoteDialog> {
        let dialogs = self.dialogs.as_ref()?;
        let template = dialogs.resources.dialog_template(NOTE_DIALOG).ok()?;
        Some(NoteDialog::new(
            &template,
            LOAD_FALLBACK_NOTE,
            self.data.button_style(),
            Rc::clone(&dialogs.metrics),
        ))
    }

    /// The load's note's input, all of it: once OK is activated, the note
    /// closes, keeping its sounds, and the flight built behind it shows.
    fn load_note_input(&mut self, input: &Input) -> ScreenAction {
        let note = self.load_note.as_mut().expect("open");
        note.input(input);
        if note.closed() {
            if let Some(mut note) = self.load_note.take() {
                self.sounds.extend(note.take_sounds());
            }
            self.enter_built_flight();
        }
        ScreenAction::None
    }

    /// Closes the saved pilots' list, keeping its sounds.
    fn close_pilot_list(&mut self) {
        if let Some(mut list) = self.open_pilot.take() {
            self.sounds.extend(list.take_sounds());
        }
    }

    /// The spaceport's input, all of it: the outfitter opening tells the
    /// flight and shows its outfitter afresh; the bar opening has the
    /// flight draw the visit's news, which the bar's News then shows; an order on its exchange trades,
    /// and one in its outfitter buys or sells, and the exchange and the
    /// outfitter as they then are go back to it. A ship asked for in its
    /// shipyard opens the prompt for its name, when it can be bought; the
    /// ship named there is bought, and one declined there draws its roll
    /// again when the shipyard's list is next built, changing nothing
    /// else. Once it is left, the ship takes off
    /// and flight shows. Otherwise the input ends by redrawing the status
    /// bar when a closing the original ends in `_RefreshSAT` came
    /// (`refresh_status_bar`). Mission BBS and Hire Escort are answered
    /// with the port dialogs' openings (`open_port_dialogs`).
    ///
    /// The bar's map key opens flight's course map over the spaceport
    /// ([`FlightView::open_bar_map`]). While it is open it takes every
    /// input: an Escape press closes it ([`FlightView::close_map`]), as in
    /// flight, and anything else goes to it ([`FlightView::port_map_input`],
    /// where M closes it too).
    fn spaceport_input(&mut self, input: &Input) -> ScreenAction {
        if self.port_map_input(input) {
            return ScreenAction::None;
        }
        let spaceport = self.spaceport.as_mut().expect(LANDED);
        spaceport.input(input);
        close_mission_texts(spaceport, self.flight.as_mut().expect(ENTERED));
        open_bar_map(spaceport, self.flight.as_mut().expect(ENTERED));
        // A new opening clears what lasts one, and the outfitter it opened
        // on was the cached one: it shows the flight's afresh, or, listing
        // nothing, says so and never opens, the port's `_DrawStatus` after
        // it still drawing a marked nav.
        open_outfitter(spaceport, self.flight.as_mut().expect(ENTERED));
        let trade = spaceport.take_trade();
        let outfit = spaceport.take_outfit();
        let ship = spaceport.take_ship();
        let recharge = spaceport.take_recharge();
        let hire = spaceport.take_hire();
        let accepted = spaceport.take_mission_accept();
        open_port_dialogs(spaceport, self.flight.as_mut().expect(ENTERED));
        if spaceport.take_mission_info_request() {
            let info = self.flight.as_ref().expect(ENTERED).mission_info();
            spaceport.open_mission_info(info);
        }
        // An abort frees the slot, and the info goes back, closing once
        // nothing is left; a refusal (greyed) changes nothing.
        abort_and_refresh(spaceport, self.flight.as_mut());
        // The bar draws the visit's news as it opens, before News can
        // show it; News shows what was drawn.
        if spaceport.take_bar_opened() {
            self.flight.as_mut().expect(ENTERED).open_bar();
        }
        if spaceport.take_news_request() {
            let flight = self.flight.as_ref().expect(ENTERED);
            if let (Some(news), Some(picture)) = (flight.news(), flight.news_picture()) {
                spaceport.open_news(news, picture);
            }
        }
        if let Some(ship) = spaceport.take_ship_request()
            && let Ok(naming) = self.flight.as_mut().expect(ENTERED).ship_naming(ship)
        {
            spaceport.open_ship_naming(&naming);
        }
        // The list is kept for the visit, as the original keeps it; the
        // roll is drawn again when it is next built.
        if let Some(ship) = spaceport.take_declined_ship() {
            self.flight.as_mut().expect(ENTERED).decline_ship(ship);
        }
        if trade.is_some()
            || outfit.is_some()
            || ship.is_some()
            || recharge
            || hire.is_some()
            || accepted.is_some()
        {
            let flight = self.flight.as_mut().expect(ENTERED);
            // A refused order changes nothing; the screen greys what it can.
            if let Some(order) = trade {
                let _ = flight.trade(order);
            }
            // A plain click's count of 1 is exactly one order; the
            // quantity dialog's Cancel orders none, which still runs what
            // follows the original's loop once its button's check passed:
            // the buy's reset and either's status draw. A sale
            // refused in the sell loop shows its words over the
            // outfitter, at whatever pass it is refused; the sales before
            // it stand. Any other refusal changes nothing. Then the
            // outfitter follows what the order left its selection at: an
            // order that changed the listing resets it, or closes the
            // outfitter when nothing is listed.
            if let Some((order, count)) = outfit {
                let run = flight.outfit_order(order, count);
                if let Some(alert) = &run.alert {
                    spaceport.alert_outfit(alert);
                }
                spaceport.outfit_ordered(run.after);
            }
            if let Some(order) = ship {
                let _ = flight.buy_ship(order.ship, &order.name);
            }
            // A hire draws the class's roll again, so the list is built
            // afresh; the hire screen closes back to the bar.
            if let Some(ship) = hire {
                let _ = flight.hire(ship);
                if let Some(list) = flight.escorts_for_hire() {
                    spaceport.set_hire(&list);
                }
            }
            // An accept shows its texts, then the computer again, as the
            // original closes it and opens it again; one left with nothing
            // available closes silently.
            if let Some(mission) = accepted {
                let _ = flight.accept_mission(mission);
                spaceport.show_mission_texts(flight.take_mission_texts());
                spaceport.set_mission_computer(flight.open_mission_computer());
            }
            // A refused refill says why in the spaceport.
            if recharge {
                match flight.recharge() {
                    Ok(_) => spaceport.recharged(),
                    Err(refusal) => spaceport.refuse_recharge(refusal),
                }
            }
            // Each changes the cash; an outfit the cargo space and the
            // trade-in; and a ship the cargo space, the free mass and the
            // outfits: so all three go back.
            refresh_spaceport(spaceport, flight);
        }
        self.serve_closings();
        self.serve_mission_screens();
        self.serve_descriptions();
        let spaceport = self.spaceport.as_mut().expect(LANDED);
        if spaceport.left() {
            self.leave_port();
            return ScreenAction::None;
        }
        self.serve_offers();
        self.serve_player_info();
        if let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) {
            refresh_status_bar(spaceport, flight);
        }
        ScreenAction::None
    }

    /// The texts the spaceport's mission computer and mission info screen
    /// have shown selected, loaded through flight as the original loads a
    /// selected row's (`_MissionFilter` @0x9f9bd, `_MissionInfoFilter`
    /// @0x9d89f) and shown while still selected. The screens themselves
    /// open as they are asked for, earlier in the input
    /// (`open_port_dialogs`).
    fn serve_mission_screens(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if let Some(mission) = spaceport.take_computer_text_request()
            && let Some(row) = flight.show_computer_row(mission)
        {
            spaceport.show_computer_row(&row);
        }
        load_info_text(spaceport, flight);
        if let Some(mission) = spaceport.take_offer_reload() {
            flight.reload_offer_desc(mission);
        }
    }

    /// The `dësc`s the spaceport's screens have shown, each loaded
    /// through flight as `_LoadDescription` loads one
    /// ([`FlightView::load_description`]) and shown in its place, in the
    /// input that showed it, so no frame draws the raw read. One that
    /// cannot be read keeps the raw read's blank.
    fn serve_descriptions(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        while let Some(id) = spaceport.take_description_request() {
            if let Ok(text) = flight.load_description(id) {
                spaceport.show_description(id, &text);
            }
        }
    }

    /// The mission info screen over flight's selected text, loaded
    /// through flight as [`AppScreen::serve_mission_screens`] loads the
    /// spaceport's.
    fn load_flight_info_text(&mut self) {
        let (Some(missions), Some(flight)) = (&mut self.missions, &mut self.flight) else {
            return;
        };
        load_info_text(missions, flight);
    }

    /// The outfitter and the hire screen closing, however they closed this
    /// input (an outfitter's Done or Escape, an order that left nothing
    /// listed, or the input that closed that order's alert; a hire screen
    /// after its hire, on Cancel, or a bar with nothing for hire), each
    /// read once its order or hire went through: flight is told
    /// ([`FlightView::close_outfitter`], [`FlightView::close_hire`]), whose
    /// reset can trim the cargo, and the shops are handed back. Then a port
    /// dialog that closed redraws what its close draws and restores the
    /// nav saved as it opened (`close_port_dialogs`).
    fn serve_closings(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        let outfitter = spaceport.take_outfitter_closed();
        let hire = spaceport.take_hire_closed();
        if outfitter {
            flight.close_outfitter();
        }
        if hire {
            flight.close_hire();
        }
        if outfitter || hire {
            refresh_spaceport(spaceport, flight);
        }
        close_port_dialogs(spaceport, flight);
    }

    /// The spaceport's player info screen: asked for, it opens on what
    /// flight gives as the dialog opens ([`FlightView::open_player_info`]),
    /// whose reset can trim the cargo, so the shops are handed back; its
    /// Jettison drops everything through flight, which changes the
    /// exchange, so the shops are handed back, and the session makes a
    /// save due.
    fn serve_player_info(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if spaceport.take_player_info_request()
            && let Some(info) = flight.open_player_info()
        {
            spaceport.open_player_info(info);
            refresh_spaceport(spaceport, flight);
        }
        if spaceport.take_player_info_jettison() {
            flight.jettison_all();
            refresh_spaceport(spaceport, flight);
        }
    }

    /// The venues' mission offers (`_OfferMissionFromPort` @0xa369e): a
    /// venue's ask answered with the session's next offer there, which
    /// enters the venue, and a choice made through the session, each
    /// answer showing the texts it queued and rearming the venue with a
    /// gap drawn on the router's chance (the spaceport never asks again);
    /// an accept changes the cash, so the shops are handed back.
    fn serve_offers(&mut self) {
        let (Some(spaceport), Some(flight)) = (&mut self.spaceport, &mut self.flight) else {
            return;
        };
        if spaceport.take_offer_restart() {
            spaceport.rearm_offers(flight.offer_gap());
        }
        if let Some(venue) = spaceport.take_offer_request() {
            match flight.next_offer(venue) {
                Some(VenueOffer::Prompt(prompt)) => spaceport.open_offer(prompt),
                Some(VenueOffer::Accepted { .. }) => {
                    spaceport.show_mission_texts(flight.take_mission_texts());
                    spaceport.rearm_offers(flight.offer_gap());
                    refresh_spaceport(spaceport, flight);
                }
                None => spaceport.rearm_offers(flight.offer_gap()),
            }
        }
        if let Some((mission, choice)) = spaceport.take_offer_choice() {
            match choice {
                OfferChoice::Accept => {
                    let _ = flight.accept_mission(mission);
                }
                OfferChoice::Refuse => {
                    // A `RefuseText` hides the dialog (@0xa2e8e); without
                    // one it stays drawn under what the refusal shows.
                    if flight
                        .refuse_mission(mission)
                        .is_ok_and(|shown| shown.offer_hidden)
                    {
                        spaceport.release_answered_offer();
                    }
                }
            }
            spaceport.show_mission_texts(flight.take_mission_texts());
            spaceport.rearm_offers(flight.offer_gap());
            refresh_spaceport(spaceport, flight);
        }
    }

    /// Whether `input` is a Preferences key, which the router takes, with
    /// dialogs to open them by: Command+comma on every side, as the
    /// original's app menu opens them (`_PrefAEVTHandler` @0x56181), and
    /// P on the main menu, the ship browser and the galaxy viewer, away
    /// from flight and the spaceport, where P is the player info key. A
    /// press opens them; its repeats and release do nothing.
    fn preferences_key(&mut self, input: &Input) -> bool {
        let Input::Key {
            key,
            pressed,
            repeat,
        } = *input
        else {
            return false;
        };
        let away = matches!(self.side, Side::MainMenu | Side::Ships | Side::Galaxy);
        let taken =
            (key == PREFERENCES_KEY && self.command_held) || (key == Key::Char('p') && away);
        if !taken || self.dialogs.is_none() {
            return false;
        }
        if pressed && !repeat {
            // `_MainScreenKey`'s P (@0x1ac59) clicks first; a Command key
            // goes to the original's menus (`MenuKey`), which do not.
            // It opens once the click has played.
            if self.side == Side::MainMenu && key == Key::Char('p') && !self.command_held {
                self.click_then(MenuAction::Preferences);
            } else {
                self.open_preferences();
            }
        }
        true
    }

    /// Whether the spaceport takes every input, the Preferences keys
    /// included: while a ship's name is typed into its prompt, P and all;
    /// and while a movie plays in front, as `_PlayQTMovie`'s modal loop
    /// takes no menu events (`WaitNextEvent` with mask 0 @0x55e4e).
    fn spaceport_takes_every_key(&self) -> bool {
        let Some(spaceport) = &self.spaceport else {
            return false;
        };
        let naming = spaceport
            .open_shipyard()
            .is_some_and(|shipyard| shipyard.naming().is_some());
        naming || (self.side == Side::Spaceport && spaceport.playing())
    }

    /// Routes one input, as [`Screen::input`] describes.
    fn route(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key { key, pressed, .. } = *input {
            match key {
                Key::Command => self.command_held = pressed,
                Key::Char('s') => self.keys_held.s = pressed,
                Key::Char('l') => self.keys_held.l = pressed,
                Key::Char('e') => self.keys_held.e = pressed,
                _ => {}
            }
        }
        if self.about.is_some() {
            return self.about_input(input);
        }
        if self.preferences.is_some() {
            return self.preferences_input(input);
        }
        if self.player_info.is_some() {
            return self.player_info_input(input);
        }
        if self.haggle.is_some() {
            return self.haggle_input(input);
        }
        if self.comm.is_some() {
            return self.comm_input(input);
        }
        if self.missions.is_some() {
            return self.missions_input(input);
        }
        if self.offer.is_some() {
            return self.offer_input(input);
        }
        if self.plunder.is_some() {
            return self.plunder_input(input);
        }
        if self.assignment.is_some() {
            return self.assignment_input(input);
        }
        if self.new_pilot.is_some() {
            return self.new_pilot_input(input);
        }
        if self.open_pilot.is_some() {
            return self.open_pilot_input(input);
        }
        if self.load_note.is_some() {
            return self.load_note_input(input);
        }
        if self.spaceport_takes_every_key() {
            return self.spaceport_input(input);
        }
        if self.preferences_key(input) {
            return ScreenAction::None;
        }
        match self.side {
            Side::Flight => return self.flight_input(input),
            Side::Spaceport => return self.spaceport_input(input),
            Side::MainMenu | Side::Ships | Side::Galaxy => {}
        }
        if let Input::Key {
            key: Key::Char('i'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
        {
            if pressed && !repeat {
                self.open_about();
            }
            return ScreenAction::None;
        }
        if self.side == Side::MainMenu {
            return self.menu_input(input);
        }
        if let Input::Key {
            key: key @ (Key::Tab | Key::Escape | Key::Char('f')),
            pressed,
            repeat,
        } = *input
        {
            if !pressed || repeat {
                return ScreenAction::None;
            }
            match key {
                Key::Escape if self.showing() == Showing::System => {
                    return self.galaxy.input(input);
                }
                Key::Escape if self.menu.is_some() => self.switch_to(Side::MainMenu),
                Key::Escape => return ScreenAction::Quit,
                Key::Tab if self.side == Side::Ships => self.switch_to(Side::Galaxy),
                Key::Tab => self.switch_to(Side::Ships),
                _ => self.enter_flight(),
            }
            return ScreenAction::None;
        }
        self.shown_mut().input(input)
    }
}

/// Shows `spaceport`'s outfitter as `flight` has it, once it has opened
/// ([`SpaceportView::take_outfitter_opened`]): `flight` clears what lasts
/// one opening first. Listing nothing, the outfitter says so and never
/// opens ([`SpaceportView::set_opened_outfitter`]), and the port's
/// `_DrawStatus` after it still draws a marked nav
/// ([`FlightView::empty_outfitter_returned`]), before any port dialog's
/// opening is told.
fn open_outfitter(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if !spaceport.take_outfitter_opened() {
        return;
    }
    flight.open_outfitter();
    if let Some(outfitter) = flight.outfitter() {
        spaceport.set_opened_outfitter(outfitter);
    }
    if spaceport.take_empty_outfitter() {
        flight.empty_outfitter_returned();
    }
}

/// Opens `flight`'s course map over `spaceport` when the bar's map key
/// asked for it (`_BarFilter` item 8, `_DoSystemMap` @0x492b2), the
/// spaceport letting go of its gesture and keys as it is covered.
fn open_bar_map(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if spaceport.take_map_request() {
        spaceport.cancel_pointer();
        spaceport.release_keys();
        flight.open_bar_map();
    }
}

/// Answers `spaceport`'s Mission BBS with `flight`'s mission computer and
/// the bar's Hire Escort with the ships for hire, then tells `flight` of
/// the port dialog that has opened ([`FlightView::open_port_dialog`]),
/// which saves the nav before any order or offer run inside it, in the
/// input that opened it.
fn open_port_dialogs(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if spaceport.take_mission_computer_request() {
        let computer = flight.open_mission_computer();
        spaceport.open_mission_computer_after(computer, flight.loaded_after_movie());
    }
    if spaceport.take_hire_request()
        && let Some(list) = flight.escorts_for_hire()
    {
        spaceport.open_hire(list);
    }
    if spaceport.take_port_dialog_opened().is_some() {
        flight.open_port_dialog();
    }
}

/// Tells `flight` of the port dialog of `spaceport` that has closed
/// ([`FlightView::close_port_dialog`]): its `_RefreshSAT`, then the nav's
/// restore, as each of the original's dialogs closes, the bar's hire
/// screen among them; after the input's last order, and before any frame
/// is drawn. A mission computer that closed under mission texts is told
/// once they have closed.
fn close_port_dialogs(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if let Some(dialog) = spaceport.take_port_dialog_closed() {
        flight.close_port_dialog(dialog);
    }
}

/// Tells `flight` of each mission text the spaceport has closed since it
/// was last told, as it closed or was dropped
/// ([`SpaceportView::take_texts_closed`]): flight redraws the bar as the
/// original's caller of each text does as it returns
/// ([`FlightView::close_mission_text`]). It runs after the spaceport's
/// input, before the closings that wait for the texts redraw, and as the
/// texts queued since are shown, after every input and tick, so a text
/// closed on a movie's tick, or dropped, redraws before the next frame.
fn close_mission_texts(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    for _ in 0..spaceport.take_texts_closed() {
        flight.close_mission_text();
    }
}

/// Redraws `flight`'s whole status bar ([`FlightView::refresh_status`])
/// when a closing of `spaceport` that the original ends in `_RefreshSAT`
/// has come ([`SpaceportView::take_status_refresh`]): the bar, News, a
/// ship description, player info or mission info. It runs at the end of
/// the spaceport's input, after the input's last order (an abort's
/// mission info, a jettison), and before any frame is drawn.
fn refresh_status_bar(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if spaceport.take_status_refresh() {
        flight.refresh_status();
    }
}

/// Shows `spaceport` the exchange, outfitter and shipyard as `flight`
/// now has them.
fn refresh_spaceport(spaceport: &mut SpaceportView, flight: &mut FlightView<Rc<GameData>>) {
    if let Some(market) = flight.market() {
        spaceport.set_market(market);
    }
    if let Some(outfitter) = flight.outfitter() {
        spaceport.set_outfitter(outfitter);
    }
    if let Some(shipyard) = flight.shipyard() {
        spaceport.set_shipyard(shipyard);
    }
}

/// An open mission info screen, as whatever holds one shows it: the
/// spaceport, or the mission dialogs over flight. Its Abort and the text
/// it asks for are answered through flight
/// ([`abort_and_refresh`], [`load_info_text`]).
trait MissionInfoHolder {
    /// The slot the screen asked to abort since this was last taken,
    /// once.
    fn take_abort(&mut self) -> Option<u8>;
    /// After an abort, the screen shows `info`, closing once nothing is
    /// left.
    fn set_info(&mut self, info: MissionInfo);
    /// The slot whose `QuickBrief` the screen asked for since this was
    /// last taken, once.
    fn take_info_text_request(&mut self) -> Option<u8>;
    /// Shows `text`, slot `slot`'s, while that slot is selected.
    fn show_info_text(&mut self, slot: u8, text: &str);
}

impl MissionInfoHolder for SpaceportView {
    fn take_abort(&mut self) -> Option<u8> {
        self.take_mission_abort()
    }

    fn set_info(&mut self, info: MissionInfo) {
        self.set_mission_info(info);
    }

    fn take_info_text_request(&mut self) -> Option<u8> {
        SpaceportView::take_info_text_request(self)
    }

    fn show_info_text(&mut self, slot: u8, text: &str) {
        SpaceportView::show_info_text(self, slot, text);
    }
}

impl MissionInfoHolder for MissionDialogs {
    fn take_abort(&mut self) -> Option<u8> {
        MissionDialogs::take_abort(self)
    }

    fn set_info(&mut self, info: MissionInfo) {
        MissionDialogs::set_info(self, info);
    }

    fn take_info_text_request(&mut self) -> Option<u8> {
        MissionDialogs::take_info_text_request(self)
    }

    fn show_info_text(&mut self, slot: u8, text: &str) {
        MissionDialogs::show_info_text(self, slot, text);
    }
}

/// An Abort `holder`'s mission info screen asked for frees the slot
/// through `flight`, and the screen shows the missions left, closing once
/// nothing is left; a refusal (greyed) changes nothing. Flight is needed
/// only once an Abort was asked for.
fn abort_and_refresh(
    holder: &mut impl MissionInfoHolder,
    flight: Option<&mut FlightView<Rc<GameData>>>,
) {
    if let Some(slot) = holder.take_abort() {
        let flight = flight.expect(ENTERED);
        let _ = flight.abort_mission(slot);
        holder.set_info(flight.mission_info());
    }
}

/// The text `holder`'s mission info screen has shown selected, loaded
/// through `flight` as the original loads a selected row's
/// (`_MissionInfoFilter` @0x9d89f) and shown while still selected.
fn load_info_text(holder: &mut impl MissionInfoHolder, flight: &mut FlightView<Rc<GameData>>) {
    if let Some(slot) = holder.take_info_text_request()
        && let Some(text) = flight.show_info_row(slot)
    {
        holder.show_info_text(slot, &text);
    }
}

/// "Desc Dialog" showing `text`, from the dialogs, in the game data's
/// button style.
fn desc_dialog(dialogs: &Dialogs, data: &GameData, text: &str) -> Result<DescDialog, String> {
    let template = dialogs.resources.dialog_template(DESC_DIALOG)?;
    Ok(DescDialog::new(
        &template,
        text,
        data.button_style(),
        Rc::clone(&dialogs.metrics),
    ))
}

/// The about text or the main menu's X text `raw` as the original loads
/// and munges it at start-up: `_LoadDescription` (about @0x16021, X
/// @0x1ac9d) resolves its conditional text by `rules` for
/// [`StartUpPilot`](nova_sim::control::StartUpPilot), with no rank
/// governments and no draws, as there is no flight; then
/// `_MungeBriefing(0, -1)` (about @0x16035, X @0x1acb1) fills its tags
/// from the zeroed globals ([`text_tags::expand_before_any_pilot`]).
/// After a flight the original reads the last pilot's globals instead
/// (no `_ResetPlayer` after `_PlayGame`); the engine holds no pilot
/// outside flight, which rdm:phase/main-screen/phase-3-game-in-progress
/// builds.
fn start_up_words(
    raw: &str,
    data: &(impl nova_sim::PilotCatalog + ?Sized),
    rules: nova_sim::text_parse::ParseRules,
) -> String {
    let resolved =
        nova_sim::text_parse::conditional_text(raw, &nova_sim::control::StartUpPilot, rules);
    text_tags::expand_before_any_pilot(&resolved, data)
}

/// Flight shows only once it has been built.
const ENTERED: &str = "flight is built when it is entered";

/// The spaceport shows only once the ship has landed.
const LANDED: &str = "the spaceport is built when the ship lands";

/// The main menu shows only once the router has one.
const MENU: &str = "the main menu side is shown only with a main menu";

/// The screen the app opens on: every screen over `data`, showing the ship
/// browser; [`AppScreen::with_pilots`] makes it open on the main menu.
#[must_use]
pub fn start_screen(data: Rc<GameData>) -> AppScreen {
    AppScreen::new(data)
}

impl Screen for AppScreen {
    /// A Tab press switches sides, first cancelling any pointer gesture on
    /// the side it hides and letting go of the keys it holds (their
    /// releases will now go to the other side). Holding Tab switches once:
    /// its key repeats are consumed without switching, as is its release.
    ///
    /// An Escape press goes back from an open system to the map, and
    /// otherwise quits; its repeats and release are consumed, so holding
    /// Escape goes back once and never quits. An F press enters flight,
    /// cancelling and letting go on the side it hides as Tab does; holding
    /// F enters once. Everything else, repeats included, goes to the side
    /// shown.
    ///
    /// In flight, an Escape press closes flight's map when it is open, and
    /// otherwise goes back to the side flight was entered from, letting go
    /// of the keys held in flight; it never quits, and its repeats and
    /// release are consumed. Everything else goes to flight, where Tab
    /// targets the next ship and F does nothing; I opens the mission info
    /// screen over flight, or, with no active mission listed, says "You
    /// have no active missions." on the message line.
    ///
    /// In the spaceport, every event goes to it; leaving it takes off,
    /// back into flight, letting go of its keys.
    ///
    /// With dialogs, an I press outside flight opens the About dialog
    /// over the side shown, cancelling and letting go on it as Tab does.
    /// A P press, on any side (flight, its map and the spaceport
    /// included), opens the Preferences dialog the same way; P reaches the
    /// router before flight. Their repeats and releases are consumed.
    /// While either dialog is open, every event goes to it alone (Escape
    /// closes it and never quits).
    ///
    /// With a main menu ([`AppScreen::with_pilots`]), the router opens on
    /// it. Its New Pilot opens the New Pilot dialog and Open Pilot the
    /// saved pilots' list over it; while either is open, every event goes
    /// to it first, so typing a name never opens a dialog. Choosing a
    /// pilot flies it; one whose load fell back first shows its note over
    /// the menu, which takes every event until Return, Escape or OK
    /// dismisses it and flight shows. Escape in flight goes back to the menu, saving
    /// the pilot and putting it away. Escape quits at once; Quit quits
    /// after [`QUIT_DELAY`] ([`Screen::take_quit`]), and its key first
    /// waits for its click, as every main-menu key does
    /// ([`Screen::keyed_click_ended`]): input meanwhile is held and
    /// replayed after, and input during the delay is dropped. I and P work
    /// on the menu, and Tab goes to the ship browser; on the ship browser
    /// and the galaxy map, Escape goes back to the menu instead of
    /// quitting.
    ///
    /// Before anything else sees it, a Caps Lock press, on any side and
    /// under any overlay, turns double time on or off for flight, built or
    /// to be built ([`AppScreen::double_time_key`]); it then goes on as
    /// any other key does.
    ///
    /// After every input, what the set expressions it ran queued is
    /// settled ([`FlightView::settle_script`]), and then a pilot with a
    /// name is saved when flight says a save is due: on landing, on taking
    /// off, after a change in the spaceport ([`AppScreen::transact`]) and
    /// after a move by set operator.
    fn input(&mut self, input: &Input) -> ScreenAction {
        // The original is inside Quit's `Delay`, then exits.
        if self.quit_in.is_some() || self.quit_due {
            return ScreenAction::None;
        }
        if let Some(hold) = self.menu_hold {
            self.hold_input(hold, input);
            return ScreenAction::None;
        }
        self.double_time_key(input);
        let action = self.route(input);
        self.settle_script();
        self.save_if_due();
        action
    }

    /// Only the side shown ticks; a hidden one is paused. While an overlay
    /// (the About dialog, the Preferences dialog, the New Pilot dialog or
    /// the saved pilots' list) is open, only it ticks: flight pauses under
    /// the preferences, as in the original. What the offer dialog over
    /// flight asked for is acted on, and the mission dialogs over flight
    /// dropped once closed, on the tick a movie of theirs ends. A fade back from black after a
    /// death moves on. Then what flight's tick ended is taken: the seam at
    /// the hold's expiry, and the return to the main menu once the death's
    /// fade is over. What the set expressions queued is settled, and a
    /// save that is due is made.
    fn tick(&mut self, dt: Duration) {
        if let Some(left) = self.quit_in {
            match left.checked_sub(dt) {
                Some(left) if !left.is_zero() => self.quit_in = Some(left),
                _ => {
                    self.quit_in = None;
                    self.quit_due = true;
                }
            }
        }
        match self.overlay_mut() {
            Some(overlay) => {
                overlay.tick(dt);
                // The spaceport isn't ticked under it, so says so: the
                // original's port window comes back to the front once the
                // overlay closes.
                if let Some(spaceport) = &mut self.spaceport {
                    spaceport.cover();
                }
            }
            None => self.shown_mut().tick(dt),
        }
        self.settle_offer();
        self.drop_closed_missions();
        self.advance_fade_in(dt);
        self.after_flight_tick();
        self.settle_script();
        self.save_if_due();
    }

    /// The side shown, then the hint (not over flight, which has its own
    /// help line, or the spaceport), then the overlay open: the New Pilot
    /// dialog, the saved pilots' list or the load's note, then the About
    /// dialog or the Preferences dialog; then the fade back from black
    /// after a death, over everything. Under the spaceport, flight's status
    /// bar is drawn at the right edge ([`FlightView::draw_status_bar`]), as
    /// the original's port dialog leaves it beside the dialog and its shops
    /// cover its left edge (`_DoPortDialog` @0x5fbca).
    fn draw(&self, list: &mut DrawList) {
        if self.side == Side::Spaceport && !self.port_map_open() {
            self.flight.as_ref().expect(ENTERED).draw_status_bar(list);
        }
        self.shown().draw(list);
        let hint = match (self.side, self.dialogs.is_some()) {
            (Side::Ships | Side::Galaxy, true) => Some(HINT),
            (Side::Ships | Side::Galaxy, false) => Some(HINT_WITHOUT_DIALOGS),
            (Side::MainMenu, true) => Some(MENU_HINT),
            (Side::MainMenu, false) => Some(MENU_HINT_WITHOUT_DIALOGS),
            (Side::Flight | Side::Spaceport, _) => None,
        };
        if let Some(hint) = hint {
            list.text(hint, HINT_AT, HINT_SIZE, None, HINT_COLOR);
        }
        if let Some(dialog) = &self.new_pilot {
            dialog.draw(list);
        }
        if let Some(pilots) = &self.open_pilot {
            pilots.draw(list);
        }
        if let Some(note) = &self.load_note {
            note.draw(list);
        }
        if let Some(dialog) = &self.plunder {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.assignment {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.comm {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.offer {
            dialog.draw(list);
        }
        if let Some(screen) = &self.player_info {
            screen.draw(list);
        }
        if let Some(dialogs) = &self.missions {
            dialogs.draw(list);
        }
        if let Some(dialog) = &self.haggle {
            dialog.draw(list);
        }
        if let Some(about) = &self.about {
            about.draw(list);
        }
        if let Some(dialog) = &self.preferences {
            dialog.draw(list);
        }
        if let Some(fade) = &self.fade_in {
            fade.draw(list);
        }
    }

    /// While the main menu waits, the cancel waits too, in its place
    /// among the inputs.
    fn cancel_pointer(&mut self) {
        if self.menu_hold.is_some() {
            self.held.push(Held::CancelPointer);
            return;
        }
        match self.overlay_mut() {
            Some(overlay) => overlay.cancel_pointer(),
            None => self.shown_mut().cancel_pointer(),
        }
    }

    /// While the main menu waits, the letting go waits too, in its place
    /// among the inputs; while E's wait holds, it counts as E's release.
    fn release_keys(&mut self) {
        if let Some(hold) = self.menu_hold {
            self.held.push(Held::ReleaseKeys);
            if hold == MenuHold::EHeld {
                self.e_released();
            }
            return;
        }
        self.command_held = false;
        self.keys_held = MenuKeys::default();
        match self.overlay_mut() {
            Some(overlay) => overlay.release_keys(),
            None => self.shown_mut().release_keys(),
        }
    }

    /// Runs the main-menu action held for its keyed click, if any (E's
    /// wait is left alone), then replays what arrived meanwhile, in
    /// order, through [`Screen::input`]: a replayed key that holds again
    /// queues the rest. A quit a replayed input asks is kept for
    /// [`Screen::take_quit`].
    fn keyed_click_ended(&mut self) {
        let Some(MenuHold::Click(action)) = self.menu_hold else {
            return;
        };
        self.menu_hold = None;
        self.run_menu_action(action);
        for held in std::mem::take(&mut self.held) {
            match held {
                Held::Input(input) => {
                    if self.input(&input) == ScreenAction::Quit {
                        self.quit_due = true;
                    }
                }
                Held::CancelPointer => self.cancel_pointer(),
                Held::ReleaseKeys => self.release_keys(),
            }
        }
    }

    /// Whether Quit's delay has run out, or a replayed input quit.
    fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.quit_due)
    }

    /// The sounds of the screens closed since, then of every live screen.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds = std::mem::take(&mut self.sounds);
        sounds.extend(self.ships.take_sounds());
        sounds.extend(self.galaxy.take_sounds());
        let open = [
            self.menu
                .as_mut()
                .map(|menu| &mut menu.screen as &mut dyn Screen),
            self.new_pilot
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.open_pilot.as_mut().map(|list| list as &mut dyn Screen),
            self.load_note.as_mut().map(|note| note as &mut dyn Screen),
            self.about.as_mut().map(|about| about as &mut dyn Screen),
            self.preferences
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.plunder
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.assignment
                .as_mut()
                .map(|dialog| dialog as &mut dyn Screen),
            self.comm.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.haggle.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.offer.as_mut().map(|dialog| dialog as &mut dyn Screen),
            self.missions
                .as_mut()
                .map(|dialogs| dialogs as &mut dyn Screen),
            self.spaceport.as_mut().map(|port| port as &mut dyn Screen),
            self.flight.as_mut().map(|flight| flight as &mut dyn Screen),
        ];
        for screen in open.into_iter().flatten() {
            sounds.extend(screen.take_sounds());
        }
        sounds
    }

    /// The preferences chosen in the Preferences dialog, once after each
    /// change.
    fn take_prefs(&mut self) -> Option<Prefs> {
        self.prefs_change.take()
    }

    fn now_showing(&self) -> Option<Showing> {
        Some(self.showing())
    }

    /// Flight's ([`FlightView`]'s cursor), while it shows with nothing
    /// over it; and while a cinematic movie plays in front, over flight or
    /// the spaceport (`_HideTheCursor` @0x55a73), never under a dialog
    /// opened over it.
    fn hides_pointer(&self) -> bool {
        match self.showing() {
            Showing::Movie => match &self.missions {
                Some(missions) => missions.hides_pointer(),
                None => self.offer.as_ref().is_some_and(Screen::hides_pointer),
            },
            Showing::Flight | Showing::Spaceport => self.shown().hides_pointer(),
            _ => false,
        }
    }

    /// Puts the pilot flying away, in flight or in the spaceport, when it
    /// has a name: saves it, or deletes a strict pilot dying
    /// ([`AppScreen::put_away_pilot`]).
    fn quit(&mut self) {
        self.put_away_pilot();
    }

    /// The warnings since they were last taken: saves that failed, pilots
    /// that could not be listed or opened, and a New Pilot dialog missing
    /// from the interface file.
    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The flight's diagnostics, once flight has been entered.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.flight
            .as_mut()
            .map(Screen::take_diagnostics)
            .unwrap_or_default()
    }
}

#[cfg(test)]
pub(super) mod tests {
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
    use nova_view::ships::{ShipBrowser, ShipId};
    use nova_view::sound::SimSound;
    use nova_view::spaceport::layout::LEAVE_ITEM;
    use nova_view::text::fixture::MonoMetrics;
    use nova_view::ui::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
    use nova_view::{DrawCommand, Font, Key, MouseButton, Point, Sound, UiSound};

    use super::*;

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
        fork.resource(Ship::TYPE, 129, Some(b"Second"), &ship)
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
    /// and a mission tag that the briefing munge would replace.
    fn about_text() -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..10)
            .map(|n| match n {
                2 => "line 2 <PN> <DST>".to_owned(),
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
        let at = map.view().world_to_screen(position);
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
        let view = *map.view();
        let at = map.view().world_to_screen(
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
        assert_ne!(*screen.galaxy_map().view(), view, "the drag goes on");
    }

    #[test]
    fn held_keys_reach_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        screen.input(&held(Key::Right));
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        screen.input(&key(Key::Tab, true));
        let fitted = *screen.galaxy_map().view();
        screen.input(&held(Key::Right));
        assert_ne!(*screen.galaxy_map().view(), fitted, "the map pans");
    }

    #[test]
    fn keys_reach_only_the_screen_shown() {
        let mut screen = AppScreen::new(data());
        let fitted = *screen.galaxy_map().view();
        assert_eq!(screen.input(&key(Key::Right, true)), ScreenAction::None);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        assert_eq!(*screen.galaxy_map().view(), fitted, "the map stays put");

        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Right, true));
        assert_ne!(*screen.galaxy_map().view(), fitted, "the map pans");
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
    }

    #[test]
    fn each_screen_keeps_its_state_across_a_round_trip() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Right, true));
        screen.input(&key(Key::Tab, true));
        click_system(&mut screen, 129);
        screen.input(&key(Key::Char('='), true));
        let (view, selected) = (*screen.galaxy_map().view(), screen.galaxy_map().selected());
        assert_eq!(selected, Some(SystemId(129)));

        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(129)));
        screen.input(&key(Key::Tab, true));
        assert_eq!(*screen.galaxy_map().view(), view);
        assert_eq!(screen.galaxy_map().selected(), selected);
    }

    #[test]
    fn switching_away_cancels_a_drag_whose_release_the_other_screen_gets() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let map = screen.galaxy_map();
        let view = *map.view();
        let at = map.view().world_to_screen(
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
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
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
        let view = *screen.galaxy_map().view();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: Point::new(100.0, 100.0),
        });
        screen.cancel_pointer();
        screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
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

    #[test]
    fn f_enters_flight_from_each_screen_and_escape_goes_back_to_it() {
        let mut screen = AppScreen::new(data());
        assert!(screen.flight_view().is_none(), "not until entered");
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.system(), SystemId(128));
        assert_eq!(session.ship(), nova_sim::ShipId(128));
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        screen.input(&key(Key::Tab, true));
        fly(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);

        click_system(&mut screen, 128);
        screen.input(&key(Key::Enter, true));
        fly(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::System);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn an_f_repeat_or_release_does_not_enter_flight() {
        let mut screen = AppScreen::new(data());
        assert_eq!(screen.input(&held(Key::Char('f'))), ScreenAction::None);
        assert_eq!(
            screen.input(&key(Key::Char('f'), false)),
            ScreenAction::None
        );
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.flight_view().is_none());
    }

    #[test]
    fn escape_in_flight_never_quits_and_its_repeats_and_release_do_nothing() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        // Holding Escape after leaving does not quit either.
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn tab_in_flight_targets_ships_not_stellars_and_stays_in_flight() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let target = |screen: &AppScreen| flight(screen).session().expect("flying").nav_target();
        assert_eq!(target(&screen), None);
        for input in [key(Key::Tab, true), held(Key::Tab), key(Key::Tab, false)] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::Flight);
        }
        assert_eq!(target(&screen), None, "Tab is the original's Target Select");
    }

    #[test]
    fn the_flight_keys_fly_the_ship_and_flight_resumes_where_it_left_off() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 10);
        let flown = ship(&screen);
        assert!(flown.position.y < 0.0, "{flown:?}");
        assert_eq!(
            screen.ship_browser().selected(),
            Some(ShipId(128)),
            "the keys went to flight"
        );
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Char('f'), true));
        assert_eq!(ship(&screen), flown, "the same flight");
    }

    #[test]
    fn only_flight_with_nothing_over_it_hides_the_system_pointer() {
        let mut screen = with_dialogs(data());
        assert!(!screen.hides_pointer(), "the ship browser");
        fly(&mut screen);
        assert!(screen.hides_pointer(), "flight");
        open_prefs(&mut screen);
        assert!(!screen.hides_pointer(), "the preferences over it");
        screen.input(&key(Key::Escape, true));
        assert!(screen.hides_pointer(), "flight again");
    }

    #[test]
    fn entering_and_leaving_flight_let_go_of_the_keys_held() {
        let mut screen = AppScreen::new(data());
        enter(&mut screen, 128);
        screen.input(&key(Key::Right, true));
        fly(&mut screen);
        // Right's release goes to flight.
        screen.input(&key(Key::Right, false));
        screen.input(&key(Key::Escape, true));
        screen.tick(Duration::from_millis(250));
        assert_eq!(camera(&screen), Point::new(0.0, 0.0), "no drift");

        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 5);
        screen.input(&key(Key::Escape, true));
        // Up's release goes to the system view.
        screen.input(&key(Key::Up, false));
        fly(&mut screen);
        let coasting = ship(&screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
    }

    #[test]
    fn entering_flight_cancels_a_drag_on_the_map() {
        let mut screen = AppScreen::new(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(100.0, 100.0),
        };
        screen.input(&left(true));
        fly(&mut screen);
        screen.input(&left(false));
        screen.input(&key(Key::Escape, true));
        screen.input(&Input::PointerMoved(Point::new(150.0, 100.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "no drag");
    }

    #[test]
    fn only_flight_ticks_while_it_is_shown() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK);
        assert_eq!(screen.ship_browser().frame(), Some(0), "paused");
        let moving = ship(&screen);
        assert_ne!(moving, nova_sim::ShipState::default());
        screen.input(&key(Key::Escape, true));
        screen.tick(TICK * 3);
        assert_eq!(ship(&screen), moving, "paused while hidden");
        assert_eq!(screen.ship_browser().frame(), Some(3));
    }

    #[test]
    fn releasing_the_keys_reaches_flight() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.release_keys();
        screen.tick(TICK * 3);
        assert_eq!(ship(&screen), nova_sim::ShipState::default());
    }

    const MAP: Key = Key::Char('m');

    /// Enters flight and opens its map with an M press.
    fn open_flight_map(screen: &mut AppScreen) {
        fly(screen);
        assert_eq!(screen.input(&key(MAP, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::FlightMap);
    }

    #[test]
    fn the_flight_maps_beeps_come_through_the_router_escape_closing_it_too() {
        let mut screen = AppScreen::new(data());
        open_flight_map(&mut screen);
        let ui = |sounds: Vec<Sound>| -> Vec<Sound> {
            sounds
                .into_iter()
                .filter(|sound| matches!(sound, Sound::Ui(_)))
                .collect()
        };
        assert_eq!(
            ui(screen.take_sounds()),
            [Sound::Ui(UiSound::MapOpened)],
            "opened"
        );
        screen.input(&key(MAP, false));
        screen.input(&key(Key::Escape, true));
        assert!(!flight(&screen).map_open());
        assert_eq!(
            ui(screen.take_sounds()),
            [Sound::Ui(UiSound::MapClosed)],
            "Escape closed it"
        );
    }

    #[test]
    fn m_in_flight_opens_its_map_and_escape_closes_it_back_into_flight() {
        let mut screen = AppScreen::new(data());
        open_flight_map(&mut screen);
        assert!(flight(&screen).map_open());
        assert_eq!(screen.input(&key(MAP, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::FlightMap);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(
            screen.showing(),
            Showing::FlightMap,
            "a repeat does nothing"
        );
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(!flight(&screen).map_open());
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight, "still flying");
        // Escape in flight leaves it, as before.
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn m_closes_the_flight_map_too() {
        let mut screen = AppScreen::new(data());
        open_flight_map(&mut screen);
        screen.input(&key(MAP, false));
        screen.input(&key(MAP, true));
        assert_eq!(screen.showing(), Showing::Flight);
    }

    #[test]
    fn tab_f_and_i_do_nothing_on_the_flight_map() {
        let mut screen = with_dialogs(data());
        open_flight_map(&mut screen);
        for k in [Key::Tab, Key::Char('f'), Key::Char('i')] {
            assert_eq!(screen.input(&key(k, true)), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::FlightMap, "{k:?}");
        }
        assert!(screen.about().is_none());
    }

    #[test]
    fn the_flight_map_is_drawn_without_the_hint() {
        let mut screen = AppScreen::new(data());
        open_flight_map(&mut screen);
        let mut map = DrawList::new();
        flight(&screen).course_map().draw(&mut map);
        assert_eq!(drawn(&screen), map);
    }

    #[test]
    fn flight_is_drawn_without_the_hint() {
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        screen.tick(TICK);
        assert_eq!(drawn(&screen), drawn(flight(&screen)));
        screen.input(&key(Key::Escape, true));
        let list = drawn(&screen);
        assert_eq!(list.iter().last(), Some(&hint()));
    }

    // The About dialog.

    /// Dialog templates by ID: "Desc Dialog" (3003), 200 x 100 at (0, 0),
    /// with Done (1) and a 100 x 36 text box (3): three lines of the About
    /// text show; the note alert (3000), 200 x 100, with OK (1) and a
    /// text box (3); and "Spaceport" (1000), [`spaceport_template`].
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
    /// (1), the effects volume (4 to 7), the Music (8) and Sound (20)
    /// check boxes, Key Settings (16), a greyed check box (9) and the
    /// Hyperspace Effects check box (21).
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
            ItemSpec::CheckBox("Smoke Trails".into()),
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

    #[test]
    fn i_opens_the_about_text_over_the_side_shown() {
        let data = data();
        let mut screen = with_dialogs(Rc::clone(&data));
        assert!(screen.about().is_none());
        let below = drawn(&screen);
        open_about(&mut screen);
        assert_eq!(about(&screen).lines()[..2], ["line 0", "line 1"]);
        assert_eq!(about(&screen).lines().len(), 10);
        // The side, the hint, then the dialog.
        let list = drawn(&screen);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let below: Vec<&DrawCommand> = below.iter().collect();
        assert_eq!(commands[..below.len()], below[..]);
        assert_eq!(
            drawn(about(&screen)).iter().collect::<Vec<_>>(),
            commands[below.len()..]
        );

        // From the galaxy map too.
        let mut screen = with_dialogs(data);
        screen.input(&key(Key::Tab, true));
        open_about(&mut screen);
    }

    #[test]
    fn the_about_text_reads_its_conditional_text_for_a_pilot_not_yet_loaded() {
        assert_eq!(
            start_up_words(
                "By {G \"him\" \"her\"}{b1 \", set\" \"\"}{P30 \".\" \"!\"} <PN>",
                &*data(),
                nova_sim::text_parse::ParseRules::default()
            ),
            "By her. ",
            "conditional text first, then the tags (no pilot's name)"
        );
    }

    #[test]
    fn the_about_text_follows_the_apps_rulebook() {
        let about_only = |text: &str| {
            loaded(&ForkBuilder::new().resource(Desc::TYPE, ABOUT_TEXT, None, &desc_of(text)))
        };
        let mut screen = with_dialogs(about_only("Hi {x there"));
        open_about(&mut screen);
        assert_eq!(about(&screen).lines(), ["Hi "], "the engine's reading");
        let mut screen = with_dialogs(about_only("Hi {x there")).with_rules(
            Rulebook::default().with_override(RuleKey::StrayTextBrace, RuleSource::Bible),
        );
        open_about(&mut screen);
        assert_eq!(
            about(&screen).lines(),
            ["Hi {x there"],
            "the stray brace copied as written"
        );
    }

    /// Game data holding only `dësc` `id` reading `text` and shïp 128,
    /// "First", which `<PST>` names before any pilot.
    fn start_up_text_only(id: i16, text: &str) -> Rc<GameData> {
        loaded(
            &ForkBuilder::new()
                .resource(Desc::TYPE, id, None, &desc_of(text))
                .resource(
                    Ship::TYPE,
                    128,
                    Some(b"First"),
                    &vec![0; Ship::SIZE.expect("fixed")],
                ),
        )
    }

    /// The lines main menu X shows over `data`, under `rules`.
    fn x_lines(data: Rc<GameData>, rules: Rulebook) -> Vec<String> {
        let store = MemoryPilots::new();
        let mut screen = with_dialogs(data)
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
            .with_rules(rules);
        screen.input(&key(Key::Char('x'), true));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::Note);
        about(&screen).lines().to_vec()
    }

    #[test]
    fn the_about_text_fills_its_tags_as_before_any_pilot() {
        let mut screen = with_dialogs(start_up_text_only(
            ABOUT_TEXT,
            "Registered to: <PN>, a <PST>, {G \"he\" \"she\"}.",
        ));
        open_about(&mut screen);
        assert_eq!(
            about(&screen).lines().join(" "),
            "Registered to: , a First, she.",
            "no pilot's name, ship 128's class, the gender's second reading"
        );
    }

    #[test]
    fn the_x_text_resolves_its_conditional_text_for_a_pilot_not_yet_loaded() {
        assert_eq!(
            x_lines(
                start_up_text_only(MENU_X_TEXT, "Hi {G \"sir\" \"ma'am\"} <PST>"),
                Rulebook::default()
            ),
            ["Hi ma'am First"]
        );
    }

    #[test]
    fn the_x_text_follows_the_apps_rulebook() {
        let x_only = || start_up_text_only(MENU_X_TEXT, "Hi {x there");
        assert_eq!(
            x_lines(x_only(), Rulebook::default()),
            ["Hi "],
            "the engine's reading"
        );
        assert_eq!(
            x_lines(
                x_only(),
                Rulebook::default().with_override(RuleKey::StrayTextBrace, RuleSource::Bible)
            ),
            ["Hi {x there"],
            "the stray brace copied as written"
        );
    }

    #[test]
    fn the_about_dialog_uses_the_game_s_button_style() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let list = drawn(about(&screen));
        let done = list
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text, font, size, ..
                } if text == "Done" => Some((*font, *size)),
                _ => None,
            })
            .expect("the Done label");
        assert_eq!(done, (Font::Charcoal, 12.0), "stock, as there is no cölr");
    }

    #[test]
    fn without_dialogs_i_does_nothing() {
        let mut screen = AppScreen::new(data());
        let before = drawn(&screen);
        assert_eq!(screen.input(&key(Key::Char('i'), true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.about().is_none());
        assert_eq!(drawn(&screen), before);
    }

    /// The text of the last thing `screen` draws: the hint.
    fn hint_text(screen: &AppScreen) -> String {
        match drawn(screen).iter().last() {
            Some(DrawCommand::Text { text, .. }) => text.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_hint_offers_i_only_with_dialogs() {
        let data = data();
        let without = hint_text(&AppScreen::new(Rc::clone(&data)));
        assert_eq!(without, "Tab: ships / galaxy map   F: fly");
        assert!(!without.contains("I:"), "{without}");
        assert_eq!(
            hint_text(&with_dialogs(data)),
            "Tab: ships / galaxy map   F: fly   I: about   P: preferences"
        );
    }

    #[test]
    fn a_missing_dialog_or_text_opens_nothing() {
        let mut screen =
            AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
        screen.input(&key(Key::Char('i'), true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        let file = OneFile(ForkBuilder::new().build().bytes);
        let empty = Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"));
        let mut screen = with_dialogs(empty);
        screen.input(&key(Key::Char('i'), true));
        assert!(screen.about().is_none());
    }

    #[test]
    fn an_i_repeat_or_release_opens_nothing() {
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Char('i'), false));
        screen.input(&held(Key::Char('i')));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn i_does_nothing_in_flight() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Char('i'), true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.about().is_none());
    }

    #[test]
    fn while_open_input_reaches_only_the_dialog() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        for input in [
            key(Key::Right, true),
            key(Key::Tab, true),
            key(Key::Char('f'), true),
            key(Key::Char('i'), true),
        ] {
            assert_eq!(screen.input(&input), ScreenAction::None);
        }
        assert_eq!(screen.showing(), Showing::About);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
        assert!(screen.flight_view().is_none());
        screen.input(&key(Key::Down, true));
        screen.input(&held(Key::Down));
        assert_eq!(about(&screen).first_line(), 2);
    }

    #[test]
    fn escape_closes_the_dialog_and_never_quits() {
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Tab, true));
        open_about(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert!(screen.about().is_none());
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn return_or_a_click_on_done_closes_the_dialog() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: done,
            });
        }
        assert_eq!(screen.showing(), Showing::ShipBrowser);
    }

    #[test]
    fn only_the_dialog_ticks_while_it_is_open() {
        let mut screen = with_dialogs(data());
        let tick = Duration::from_millis(100);
        screen.tick(tick);
        let frame = screen.ship_browser().frame();
        open_about(&mut screen);
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), frame, "paused below");
        screen.input(&key(Key::Escape, true));
        screen.tick(tick);
        assert_ne!(screen.ship_browser().frame(), frame);
    }

    #[test]
    fn opening_cancels_a_drag_and_lets_go_of_the_keys_below() {
        let mut screen = with_dialogs(data());
        enter(&mut screen, 128);
        let camera_at = camera(&screen);
        screen.input(&key(Key::Char('d'), true));
        open_about(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::System);
        screen.tick(Duration::from_secs(1));
        assert_eq!(camera(&screen), camera_at, "D was let go");

        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        let at = Point::new(500.0, 400.0);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        open_about(&mut screen);
        screen.input(&key(Key::Escape, true));
        screen.input(&Input::PointerMoved(Point::new(600.0, 400.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "the drag was cancelled");
    }

    #[test]
    fn debug_shows_whether_there_are_dialogs() {
        let debug = format!("{:?}", with_dialogs(data()));
        assert!(debug.contains("dialogs: Some(Dialogs { .. })"), "{debug}");
        let debug = format!("{:?}", AppScreen::new(data()));
        assert!(debug.contains("dialogs: None"), "{debug}");
    }

    #[test]
    fn cancelling_and_releasing_reach_the_dialog_while_open() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        screen.input(&button(true));
        screen.release_keys();
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(screen.showing(), Showing::About, "the click was abandoned");
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

    #[test]
    fn l_over_the_planet_shows_its_spaceport() {
        let mut screen = with_dialogs(data());
        assert!(screen.spaceport_view().is_none());
        land(&mut screen);
        let port = spaceport(&screen);
        assert_eq!(port.stellar(), nova_sim::StellarId(128));
        assert_eq!(port.problem(), None);
        assert_eq!(
            port.offered(),
            [nova_sim::Service::Bar, nova_sim::Service::MissionBbs]
        );
        assert_eq!(
            flight(&screen).session().expect("flying").landed(),
            Some(nova_sim::StellarId(128))
        );
        // Drawn alone: no hint.
        assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    }

    #[test]
    fn landing_lets_go_of_the_flight_keys() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        // Up's release goes to the spaceport, and Leave takes off.
        screen.input(&key(Key::Up, false));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen), nova_sim::ShipState::default(), "no thrust");
    }

    #[test]
    fn leave_takes_off_back_into_flight_at_the_planet() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        let leave = spaceport(&screen)
            .dialog()
            .expect("laid out")
            .item_bounds(LEAVE_ITEM)
            .expect("Leave")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: leave,
            });
        }
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.spaceport_view().is_none());
        assert_eq!(flight(&screen).session().expect("flying").landed(), None);
        assert_eq!(ship(&screen), nova_sim::ShipState::default());
        // It flies again.
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 3);
        assert!(ship(&screen).position.y < 0.0);
    }

    #[test]
    fn escape_in_the_spaceport_leaves_and_never_quits() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight);
    }

    #[test]
    fn tab_f_and_i_do_nothing_in_the_spaceport() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        for k in [Key::Tab, Key::Char('f'), Key::Char('i'), LAND] {
            assert_eq!(screen.input(&key(k, true)), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::Spaceport, "{k:?}");
        }
        assert!(screen.about().is_none());
    }

    #[test]
    fn without_dialogs_the_spaceport_says_why_and_escape_still_leaves() {
        let mut screen = AppScreen::new(data());
        land(&mut screen);
        assert_eq!(spaceport(&screen).problem(), Some(NO_INTERFACE));
        let texts: Vec<String> = drawn(&screen)
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            texts,
            [
                "Cannot show the spaceport: no interface file. Press Return or Escape to take off."
                    .to_owned()
            ]
        );
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
    }

    #[test]
    fn a_missing_spaceport_dialog_says_why() {
        let mut screen =
            AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
        land(&mut screen);
        assert_eq!(spaceport(&screen).problem(), Some("no DLOG 1000"));
    }

    #[test]
    fn only_the_spaceport_ticks_and_takes_pointer_and_key_resets() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        let docked = ship(&screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen), docked);
        let leave = spaceport(&screen)
            .dialog()
            .expect("laid out")
            .item_bounds(LEAVE_ITEM)
            .expect("Leave")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: leave,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.release_keys();
        screen.input(&button(false));
        assert_eq!(
            screen.showing(),
            Showing::Spaceport,
            "the click was abandoned"
        );
    }

    #[test]
    fn a_refused_landing_stays_in_flight() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 30);
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.spaceport_view().is_none());
        assert!(flight(&screen).message().is_some());
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

    #[test]
    fn the_router_says_what_it_shows() {
        let mut screen = with_dialogs(data());
        assert_eq!(screen.now_showing(), Some(Showing::ShipBrowser));
        land(&mut screen);
        assert_eq!(screen.now_showing(), Some(Showing::Spaceport));
    }

    #[test]
    fn flights_sounds_come_through_the_router() {
        let mut screen = with_dialogs(data());
        assert_eq!(screen.take_sounds(), []);
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 2);
        assert_eq!(screen.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
        screen.input(&key(Key::Up, false));
        screen.tick(TICK * 2);
        assert_eq!(screen.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
        assert_eq!(screen.take_sounds(), [], "taken");
    }

    #[test]
    fn leaving_the_spaceport_by_leave_sounds_only_the_take_off() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        assert_eq!(
            screen.take_sounds(),
            [
                Sound::Sim(SimSound::TrafficControl),
                Sound::Sim(SimSound::Landed)
            ],
            "the greeting, then the landing"
        );
        let leave = spaceport(&screen)
            .dialog()
            .expect("laid out")
            .item_bounds(LEAVE_ITEM)
            .expect("Leave")
            .center();
        assert_eq!(
            press_and_release(&mut screen, leave),
            [vec![], vec![halted(), Sound::Sim(SimSound::TookOff)]],
            "the ambient sound halts with the spaceport"
        );
        assert!(screen.spaceport_view().is_none(), "gone");
    }

    fn ambient() -> Sound {
        Sound::Ui(UiSound::PortAmbient {
            sound: nova_sim::SoundId(AMBIENT),
            until_stopped: false,
        })
    }

    fn halted() -> Sound {
        Sound::Ui(UiSound::PortAmbientHalted(nova_sim::SoundId(AMBIENT)))
    }

    fn beta_ambient() -> Sound {
        Sound::Ui(UiSound::PortAmbient {
            sound: nova_sim::SoundId(BETA_AMBIENT),
            until_stopped: false,
        })
    }

    /// No UI puts a landed pilot away: flight's Escape and a game over
    /// both come from flight. So the put-away is driven through
    /// [`AppScreen::end_flight`] itself.
    #[test]
    fn putting_a_landed_pilot_away_halts_the_ambient_sound() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        screen.end_flight(Side::MainMenu);
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert!(screen.spaceport_view().is_none(), "gone");
        let sounds = screen.take_sounds();
        assert!(sounds.contains(&halted()), "{sounds:?}");
    }

    #[test]
    fn a_pilot_desk_move_to_another_stellar_halts_the_old_sound_before_the_new_one_starts() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        edit(
            &mut screen,
            PilotEdit::MoveTo {
                system: SystemId(129),
                stellar: nova_sim::StellarId(129),
            },
        );
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
        // The new spaceport's offer is served after its first frame
        // ([`AppScreen::tick`]), which it covers, so its sound starts on
        // the second.
        let sounds = [
            screen.take_sounds(),
            frame_sounds(&mut screen, TICK),
            frame_sounds(&mut screen, TICK),
        ]
        .concat();
        let halt = sounds.iter().position(|&sound| sound == halted());
        let started = sounds.iter().position(|&sound| sound == beta_ambient());
        assert!(
            halt.is_some() && started.is_some() && halt < started,
            "Alpha Prime's halt, then Beta Prime's sound: {sounds:?}"
        );
        assert!(!sounds.contains(&ambient()), "{sounds:?}");
    }

    #[test]
    fn a_pilot_desk_edit_that_stays_landed_keeps_the_ambient_sound_and_its_wait() {
        use nova_sim::mission::screens::mac_ticks;
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(FirstOutcome));
        let mut screen = with_dialogs(data()).with_chance(SharedChance::new(shared));
        land(&mut screen);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        assert_eq!(frame_sounds(&mut screen, mac_ticks(240)), []);
        edit(&mut screen, PilotEdit::Credits(5000));
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(128));
        let sounds = screen.take_sounds();
        assert!(!sounds.contains(&halted()), "{sounds:?}");
        assert_eq!(
            frame_sounds(&mut screen, mac_ticks(120)),
            [],
            "not started again"
        );
        assert_eq!(
            frame_sounds(&mut screen, mac_ticks(120)),
            [ambient()],
            "480 ticks after the first, the wait carried on"
        );
    }

    /// No UI replaces a docked pilot's flight: the menu that opens or
    /// creates a pilot is reached only once flight is put away. So the
    /// replacement is driven through [`AppScreen::build_flight`] itself,
    /// and its sounds read from the router's queue, as the spaceport side
    /// is left showing with no spaceport to show.
    #[test]
    fn replacing_a_docked_pilot_halts_the_ambient_sound() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        let bea = nova_sim::Pilot::new(screen.data.as_ref(), "Bea").expect("a pilot");
        screen.build_flight(bea, false);
        assert!(screen.spaceport_view().is_none(), "gone");
        let sounds = std::mem::take(&mut screen.sounds);
        assert!(sounds.contains(&halted()), "{sounds:?}");
    }

    /// One frame of `dt`'s sounds.
    fn frame_sounds(screen: &mut AppScreen, dt: Duration) -> Vec<Sound> {
        screen.tick(dt);
        screen.take_sounds()
    }

    #[test]
    fn the_stellars_ambient_sound_starts_on_the_spaceports_first_frame_not_at_touchdown() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        let landing = screen.take_sounds();
        assert!(!landing.contains(&ambient()), "{landing:?}");
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        assert_eq!(frame_sounds(&mut screen, TICK), []);
    }

    /// The landing's input settles its texts and the spaceport's offer
    /// ([`AppScreen::settle_script`]) before the spaceport's first frame,
    /// so nothing holds the sound back there.
    #[test]
    fn with_a_pilot_the_ambient_sound_starts_once_the_landings_offer_is_served() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, data());
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
    }

    #[test]
    fn the_ambient_sounds_waits_are_drawn_on_the_routers_chance() {
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(FirstOutcome));
        let mut screen = with_dialogs(data()).with_chance(SharedChance::new(shared));
        land(&mut screen);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        assert_eq!(
            frame_sounds(&mut screen, nova_sim::mission::screens::mac_ticks(480)),
            [ambient()],
            "480 ticks, as the first outcome draws"
        );
    }

    /// A source whose every draw is the first outcome.
    struct FirstOutcome;

    impl Chance for FirstOutcome {
        fn fires(&mut self, _percent: u8) -> bool {
            true
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
        }
    }

    #[test]
    fn the_preferences_closing_over_the_spaceport_rearms_the_ambient_sound() {
        use nova_sim::mission::screens::mac_ticks;
        let mut screen = with_dialogs(data());
        land(&mut screen);
        screen.take_sounds();
        assert_eq!(frame_sounds(&mut screen, TICK), [ambient()]);
        assert_eq!(frame_sounds(&mut screen, mac_ticks(900)), []);
        open_prefs(&mut screen);
        assert_eq!(frame_sounds(&mut screen, mac_ticks(900)), [], "paused");
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Escape, false));
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen.take_sounds();
        assert_eq!(
            frame_sounds(&mut screen, mac_ticks(100)),
            [],
            "the wait began again"
        );
        assert_eq!(frame_sounds(&mut screen, mac_ticks(959)), [ambient()]);
    }

    #[test]
    /// `_TrackTextButtons` @0x26bb0 plays nothing under the pointer.
    fn closing_the_about_text_by_done_is_silent() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        assert_eq!(press_and_release(&mut screen, done), [vec![], vec![]]);
        assert!(screen.about().is_none(), "closed");
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

    fn quiet() -> Prefs {
        Prefs {
            sound: nova_view::SoundPrefs {
                sound: true,
                music: false,
                effects_level: 3,
                music_level: 5,
            },
            hyperspace_effects: true,
            quicktime_movies: true,
        }
    }

    #[test]
    fn p_opens_the_preferences_over_ships_and_galaxy() {
        let mut screen = with_dialogs(data());
        assert!(screen.preferences().is_none());
        screen.input(&key(PREFS, true));
        assert_eq!(screen.showing(), Showing::Preferences);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        screen.input(&key(Key::Tab, true));
        screen.input(&key(PREFS, true));
        assert_eq!(screen.showing(), Showing::Preferences);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::GalaxyMap);
    }

    #[test]
    fn command_comma_opens_the_preferences_on_every_side() {
        let mut screen = with_dialogs(data());
        command_comma(&mut screen);
        assert_eq!(screen.showing(), Showing::Preferences, "the ship browser");
        screen.input(&key(Key::Escape, true));

        fly(&mut screen);
        command_comma(&mut screen);
        assert_eq!(screen.showing(), Showing::Preferences, "flight");
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "Escape only closes it");

        screen.input(&key(MAP, true));
        command_comma(&mut screen);
        assert_eq!(screen.showing(), Showing::Preferences, "over the map");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::FlightMap);
        screen.input(&key(Key::Escape, true));

        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        command_comma(&mut screen);
        assert_eq!(screen.showing(), Showing::Preferences, "the spaceport");
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Spaceport, "still landed");
    }

    #[test]
    fn a_comma_without_command_or_after_it_is_let_go_opens_nothing() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(PREFERENCES_KEY, true));
        assert_eq!(screen.showing(), Showing::Flight, "no Command");
        screen.input(&key(Key::Command, true));
        screen.input(&held(PREFERENCES_KEY));
        assert_eq!(screen.showing(), Showing::Flight, "a repeat");
        screen.input(&key(Key::Command, false));
        screen.input(&key(PREFERENCES_KEY, true));
        assert_eq!(screen.showing(), Showing::Flight, "Command let go");
        screen.input(&key(Key::Command, true));
        screen.release_keys();
        screen.input(&key(PREFERENCES_KEY, true));
        assert_eq!(screen.showing(), Showing::Flight, "the window lost focus");
    }

    #[test]
    fn the_dialog_opens_on_the_prefs_the_router_was_given() {
        let mut screen = with_dialogs(data()).with_prefs(quiet());
        assert_eq!(screen.prefs(), quiet());
        open_prefs(&mut screen);
        assert_eq!(prefs_dialog(&screen).prefs(), quiet());
        assert_eq!(AppScreen::new(data()).prefs(), Prefs::default());
    }

    #[test]
    fn each_change_comes_out_once_and_the_dialog_reopens_on_it() {
        let mut screen = with_dialogs(data()).with_prefs(quiet());
        assert_eq!(screen.take_prefs(), None);
        open_prefs(&mut screen);
        let music = prefs_dialog(&screen).music().rect().center();
        click_at(&mut screen, music);
        let changed = Prefs {
            sound: nova_view::SoundPrefs {
                music: true,
                ..quiet().sound
            },
            ..quiet()
        };
        assert_eq!(screen.take_prefs(), Some(changed));
        assert_eq!(screen.take_prefs(), None, "once");
        assert_eq!(screen.prefs(), changed);
        let up = prefs_dialog(&screen).effects_volume().rects().up.center();
        click_at(&mut screen, up);
        screen.input(&key(Key::Enter, true));
        assert!(screen.preferences().is_none(), "closed");
        let louder = Prefs {
            sound: nova_view::SoundPrefs {
                effects_level: 4,
                ..changed.sound
            },
            ..changed
        };
        assert_eq!(screen.take_prefs(), Some(louder));
        open_prefs(&mut screen);
        assert_eq!(prefs_dialog(&screen).prefs(), louder);
    }

    fn no_effects() -> Prefs {
        Prefs {
            hyperspace_effects: false,
            ..quiet()
        }
    }

    fn flight_effects(screen: &AppScreen) -> bool {
        screen.flight_view().expect("flying").hyperspace_effects()
    }

    #[test]
    fn flight_is_built_with_the_routers_hyperspace_effects() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        assert!(flight_effects(&screen), "on by default");
        let mut screen = with_dialogs(data()).with_prefs(no_effects());
        fly(&mut screen);
        assert!(!flight_effects(&screen));
    }

    #[test]
    fn a_change_in_the_dialog_reaches_flight_already_built() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        open_prefs(&mut screen);
        let point = prefs_dialog(&screen).hyperspace_effects().rect().center();
        click_at(&mut screen, point);
        assert!(!flight_effects(&screen));
        assert_eq!(
            screen.take_prefs(),
            Some(Prefs {
                hyperspace_effects: false,
                ..Prefs::default()
            })
        );
        assert_eq!(screen.take_prefs(), None, "once");
        click_at(&mut screen, point);
        assert!(flight_effects(&screen), "back on");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(flight_effects(&screen));
    }

    fn alert_quiet(screen: &AppScreen) -> bool {
        screen.flight_view().expect("flying").alert_quiet()
    }

    #[test]
    fn flight_is_built_with_the_routers_sound_prefs() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        assert!(!alert_quiet(&screen), "loud by default");
        let mut screen = with_dialogs(data()).with_prefs(Prefs {
            sound: nova_view::SoundPrefs {
                effects_level: 1,
                ..quiet().sound
            },
            ..quiet()
        });
        fly(&mut screen);
        assert!(alert_quiet(&screen));
    }

    #[test]
    fn a_sound_change_in_the_dialog_reaches_flight_already_built() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        open_prefs(&mut screen);
        let point = prefs_dialog(&screen).sound().rect().center();
        click_at(&mut screen, point);
        assert!(alert_quiet(&screen), "sound off");
        click_at(&mut screen, point);
        assert!(!alert_quiet(&screen), "back on");
    }

    #[test]
    fn a_change_before_flight_is_built_reaches_it_when_it_is() {
        let mut screen = with_dialogs(data());
        open_prefs(&mut screen);
        let point = prefs_dialog(&screen).hyperspace_effects().rect().center();
        click_at(&mut screen, point);
        screen.input(&key(Key::Escape, true));
        fly(&mut screen);
        assert!(!flight_effects(&screen));
    }

    #[test]
    fn a_p_repeat_or_release_opens_nothing_and_reaches_nothing() {
        let mut screen = with_dialogs(data());
        for input in [held(PREFS), key(PREFS, false)] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::ShipBrowser, "{input:?}");
        }
        fly(&mut screen);
        for input in [held(PREFS), key(PREFS, false)] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::Flight, "{input:?}");
        }
        open_prefs(&mut screen);
        screen.input(&key(PREFS, false));
        screen.input(&held(PREFS));
        assert_eq!(screen.showing(), Showing::Preferences, "still open");
    }

    #[test]
    fn without_dialogs_p_does_nothing() {
        let mut screen = AppScreen::new(data());
        let before = drawn(&screen);
        assert_eq!(screen.input(&key(PREFS, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert_eq!(drawn(&screen), before);
        fly(&mut screen);
        command_comma(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.preferences().is_none());
        screen.input(&key(PREFS, true));
        assert_eq!(screen.showing(), Showing::Flight, "P is the player info");
        assert!(
            screen.flight_player_info().is_none(),
            "no dialogs to show it"
        );
    }

    #[test]
    fn a_missing_preferences_dialog_opens_nothing() {
        let mut screen =
            AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
        screen.input(&key(PREFS, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.preferences().is_none());
    }

    #[test]
    fn flight_is_paused_and_lets_go_of_its_keys_while_the_dialog_is_open() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK);
        let moving = ship(&screen);
        open_prefs(&mut screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen), moving, "paused");
        // Up's release goes to the dialog.
        screen.input(&key(Key::Up, false));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        let coasting = ship(&screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
    }

    #[test]
    fn while_the_preferences_are_open_input_reaches_only_them() {
        let mut screen = with_dialogs(data());
        open_prefs(&mut screen);
        for input in [
            key(Key::Right, true),
            key(Key::Char('f'), true),
            key(Key::Char('i'), true),
            key(PREFS, true),
        ] {
            assert_eq!(screen.input(&input), ScreenAction::None);
        }
        assert_eq!(screen.showing(), Showing::Preferences);
        assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
        assert!(screen.flight_view().is_none() && screen.about().is_none());
        screen.input(&key(Key::Tab, true));
        assert_eq!(
            prefs_dialog(&screen).focus(),
            Some(nova_view::ui::prefs::Focus::Music),
            "Tab moves the dialog's focus"
        );
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn the_dialog_is_drawn_over_the_side_and_the_hint() {
        let mut screen = with_dialogs(data());
        let below = drawn(&screen);
        open_prefs(&mut screen);
        let list = drawn(&screen);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let below: Vec<&DrawCommand> = below.iter().collect();
        assert_eq!(commands[..below.len()], below[..]);
        assert_eq!(
            drawn(prefs_dialog(&screen)).iter().collect::<Vec<_>>(),
            commands[below.len()..]
        );
    }

    #[test]
    fn only_the_dialog_ticks_while_it_is_open_over_the_ships() {
        let mut screen = with_dialogs(data());
        let tick = Duration::from_millis(100);
        screen.tick(tick);
        let frame = screen.ship_browser().frame();
        open_prefs(&mut screen);
        screen.tick(tick);
        assert_eq!(screen.ship_browser().frame(), frame, "paused below");
        screen.input(&key(Key::Escape, true));
        screen.tick(tick);
        assert_ne!(screen.ship_browser().frame(), frame);
    }

    #[test]
    fn opening_cancels_a_drag_below() {
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::Tab, true));
        let view = *screen.galaxy_map().view();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: Point::new(500.0, 400.0),
        });
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        screen.input(&Input::PointerMoved(Point::new(600.0, 400.0)));
        assert_eq!(*screen.galaxy_map().view(), view, "the drag was cancelled");
    }

    #[test]
    fn cancelling_and_releasing_reach_the_preferences_while_open() {
        let mut screen = with_dialogs(data());
        open_prefs(&mut screen);
        let ok = prefs_dialog(&screen)
            .dialog()
            .item_bounds(1)
            .expect("OK")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        screen.input(&button(true));
        screen.release_keys();
        screen.cancel_pointer();
        screen.input(&button(false));
        assert_eq!(
            screen.showing(),
            Showing::Preferences,
            "the click was abandoned"
        );
    }

    #[test]
    fn closing_by_ok_is_silent() {
        let mut screen = with_dialogs(data());
        open_prefs(&mut screen);
        let ok = prefs_dialog(&screen)
            .dialog()
            .item_bounds(1)
            .expect("OK")
            .center();
        assert_eq!(press_and_release(&mut screen, ok), [vec![], vec![]]);
        assert!(screen.preferences().is_none(), "closed");
    }

    // The player info screen.

    use nova_view::ui::player_info::{PLAYER_INFO_KEY, PlayerInfoButton, Tab};

    /// Clicks `button` on the player info screen open over flight, or
    /// else over the spaceport.
    fn click_player_info(screen: &mut AppScreen, button: PlayerInfoButton) {
        let open = screen
            .flight_player_info()
            .or_else(|| screen.spaceport_view()?.player_info())
            .expect("the player info is open");
        let at = open.button_bounds(button).expect("laid out").center();
        click_at(screen, at);
    }

    #[test]
    fn p_in_flight_opens_the_player_info_over_the_paused_flight() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        screen.input(&key(Key::Up, true));
        screen.tick(TICK);
        let moving = ship(&screen);
        screen.input(&key(PLAYER_INFO_KEY, true));
        assert_eq!(screen.showing(), Showing::PlayerInfo);
        assert_eq!(screen.now_showing(), Some(Showing::PlayerInfo));
        let open = screen.flight_player_info().expect("open");
        assert_eq!(open.problem(), None, "laid out by DLOG 1017");
        assert_eq!(open.info().credits, pilot(&screen).cash());
        let list = drawn(&screen);
        let over = drawn(open);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            commands[commands.len() - over.iter().count()..],
            over.iter().collect::<Vec<_>>()[..],
            "drawn over flight"
        );
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen), moving, "paused");
        screen.input(&key(PLAYER_INFO_KEY, false));
        screen.input(&key(Key::Up, false));
        screen.input(&key(Key::Tab, true));
        assert_eq!(
            screen.flight_player_info().map(PlayerInfoScreen::tab),
            Some(Tab::Cargo),
            "it takes the input"
        );
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "Escape closes it alone");
        let coasting = ship(&screen);
        screen.tick(TICK * 5);
        assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
    }

    #[test]
    fn p_opens_the_player_info_over_the_spaceport_and_its_dialogs() {
        let store = MemoryPilots::new();
        let check = |screen: &mut AppScreen, what: &str| {
            screen.input(&key(PLAYER_INFO_KEY, true));
            let open = spaceport(screen).player_info().expect(what);
            assert_eq!(open.problem(), None, "{what}");
            assert_eq!(screen.showing(), Showing::Spaceport, "{what}");
            screen.input(&key(Key::Escape, true));
            assert!(spaceport(screen).player_info().is_none(), "{what}");
        };
        let mut screen = landed_trader(&store);
        check(&mut screen, "the spaceport");
        click_port_item(&mut screen, 7);
        check(&mut screen, "the trade center");
        assert!(spaceport(&screen).open_trade().is_some(), "still trading");
        let mut screen = landed_shipyard(&MemoryPilots::new());
        click_port_item(&mut screen, 9);
        check(&mut screen, "the shipyard");
        screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
        assert!(
            spaceport(&screen)
                .open_shipyard()
                .and_then(ShipyardScreen::info_panel)
                .is_some()
        );
        check(&mut screen, "its Info panel");
        assert!(
            spaceport(&screen)
                .open_shipyard()
                .and_then(ShipyardScreen::info_panel)
                .is_some(),
            "back to the panel"
        );
        let mut screen = landed_at_venues(&MemoryPilots::new(), venue_data(SPACEPORT));
        assert!(spaceport(&screen).offer_dialog().is_some());
        check(&mut screen, "the spaceport's offer");
        assert!(spaceport(&screen).offer_dialog().is_some(), "still offered");
    }

    #[test]
    fn confirming_jettison_empties_the_hold_and_closes_it_landed_or_flying() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 7);
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).held(FOOD), 10);
        let writes = store.writes();
        screen.input(&key(PLAYER_INFO_KEY, true));
        click_player_info(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
        click_player_info(&mut screen, PlayerInfoButton::Jettison);
        screen.input(&key(Key::Enter, true));
        assert!(spaceport(&screen).player_info().is_none(), "closed");
        assert_eq!(pilot(&screen).held(FOOD), 0, "jettisoned");
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(open.market().row(FOOD).map(|row| row.held), Some(0));
        assert_eq!(store.writes(), writes + 1, "saved");
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "taken off");
        assert_eq!(pilot(&screen).held(FOOD), 3, "the cash left buys 3");
        past_the_takeoff_grace(&mut screen);
        screen.input(&key(PLAYER_INFO_KEY, true));
        click_player_info(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
        click_player_info(&mut screen, PlayerInfoButton::Jettison);
        screen.input(&key(Key::Enter, true));
        assert!(screen.flight_player_info().is_none(), "closed");
        assert_eq!(pilot(&screen).held(FOOD), 0, "jettisoned in flight");
    }

    /// Steps flight through the take-off grace, which refuses P for
    /// [`nova_sim::TAKEOFF_GRACE_TICKS`] after a take-off.
    fn past_the_takeoff_grace(screen: &mut AppScreen) {
        for _ in 0..nova_sim::TAKEOFF_GRACE_TICKS {
            screen.tick(TICK);
        }
    }

    #[test]
    fn the_flight_help_offers_p_and_command_comma() {
        assert!(nova_view::flight::view::HELP.contains("P: player info"));
        assert!(nova_view::flight::view::HELP.contains("Cmd-,: preferences"));
    }

    // The main menu and pilots.

    use nova_sim::fixture::MemoryPilots;
    use nova_sim::{PilotKeeper, PilotStore};
    use nova_view::menu::MenuChoice;
    use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG};

    fn keeper(store: &MemoryPilots) -> PilotKeeper<Box<dyn PilotStore>> {
        PilotKeeper::new(Box::new(store.clone()))
    }

    /// The router with the main menu over `store`, and no dialogs.
    pub(crate) fn menu(store: &MemoryPilots) -> AppScreen {
        AppScreen::new(data()).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
    }

    /// The router with the main menu over `store`, and dialogs.
    fn menu_with_dialogs(store: &MemoryPilots) -> AppScreen {
        with_dialogs(data()).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
    }

    fn choose(screen: &mut AppScreen, choice: MenuChoice) -> ScreenAction {
        let at = screen
            .main_menu()
            .expect("a main menu")
            .button(choice)
            .rect
            .center();
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

    /// Creates a pilot named `name` from the main menu: New Pilot, the
    /// name, Return.
    pub(crate) fn create(screen: &mut AppScreen, name: &str) {
        assert_eq!(choose(screen, MenuChoice::NewPilot), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::NewPilot);
        type_text(screen, name);
        screen.input(&key(Key::Enter, true));
    }

    fn pilot(screen: &AppScreen) -> &nova_sim::Pilot {
        flight(screen).pilot().expect("flying")
    }

    pub(crate) fn saved(store: &MemoryPilots, key: &str) -> nova_sim::Pilot {
        nova_sim::save::decode(&store.text(key).expect("saved")).expect("a pilot")
    }

    #[test]
    fn with_pilots_the_app_opens_on_the_main_menu() {
        let store = MemoryPilots::new();
        let screen = menu(&store);
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(screen.now_showing(), Some(Showing::MainMenu));
        assert!(screen.flight_view().is_none());
        let mut list = DrawList::new();
        screen.main_menu().expect("a main menu").draw(&mut list);
        let mut all = drawn(&screen);
        assert_eq!(
            all.iter().take(list.len()).collect::<Vec<_>>(),
            list.iter().collect::<Vec<_>>()
        );
        assert!(matches!(
            all.iter().last(),
            Some(DrawCommand::Text { text, .. }) if text == MENU_HINT_WITHOUT_DIALOGS
        ));
        all = drawn(&menu_with_dialogs(&store));
        assert!(matches!(
            all.iter().last(),
            Some(DrawCommand::Text { text, .. }) if text == MENU_HINT
        ));
    }

    #[test]
    fn quit_waits_out_the_quit_delay_and_escape_quits_at_once() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        ui_sounds(&mut screen);
        assert_eq!(choose(&mut screen, MenuChoice::Quit), ScreenAction::None);
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::ButtonDown, UiSound::ButtonUp],
            "601 already played, on the release"
        );
        assert!(!screen.take_quit());
        screen.tick(QUIT_DELAY.saturating_sub(Duration::from_millis(1)));
        assert!(!screen.take_quit(), "still waiting");
        screen.tick(Duration::from_millis(1));
        assert!(screen.take_quit(), "the delay is over");
        assert!(!screen.take_quit(), "once");

        let mut screen = menu(&store);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
    }

    #[test]
    fn quits_delay_is_fifteen_ticks() {
        assert_eq!(QUIT_DELAY, nova_sim::mission::screens::mac_ticks(15));
    }

    #[test]
    fn input_during_the_quit_delay_does_nothing() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        choose(&mut screen, MenuChoice::Quit);
        ui_sounds(&mut screen);
        let new_pilot = screen
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        click_at(&mut screen, new_pilot);
        screen.input(&Input::Text('o'));
        screen.input(&key(Key::Char('p'), true));
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(ui_sounds(&mut screen), [], "nothing clicked");
        screen.tick(QUIT_DELAY);
        assert!(screen.take_quit());
    }

    #[test]
    fn keyed_q_quits_only_after_its_click_and_the_quit_delay() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        ui_sounds(&mut screen);
        assert_eq!(screen.input(&Input::Text('q')), ScreenAction::None);
        assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed]);
        screen.tick(QUIT_DELAY * 2);
        assert!(!screen.take_quit(), "the click still plays");
        screen.keyed_click_ended();
        assert!(!screen.take_quit(), "601 has only started");
        screen.tick(QUIT_DELAY.saturating_sub(Duration::from_millis(1)));
        assert!(!screen.take_quit());
        screen.tick(Duration::from_millis(1));
        assert!(screen.take_quit());
        assert!(!screen.take_quit(), "once");
    }

    #[test]
    fn an_escape_typed_during_a_keyed_click_quits_once_it_is_replayed() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        // E is refused with no pilot: the replay reaches the menu.
        screen.input(&Input::Text('e'));
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert!(!screen.take_quit(), "held for the click");
        screen.keyed_click_ended();
        assert!(screen.take_quit(), "the replayed Escape quit");
        assert!(!screen.take_quit(), "once");
    }

    #[test]
    fn input_while_a_replayed_quit_is_untaken_does_nothing() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        // E is refused with no pilot: the replay reaches the menu.
        screen.input(&Input::Text('e'));
        screen.input(&key(Key::Escape, true));
        screen.keyed_click_ended();
        ui_sounds(&mut screen);
        assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.input(&Input::Text('q')), ScreenAction::None);
        assert_eq!(ui_sounds(&mut screen), [], "nothing clicked");
        assert!(screen.take_quit());
    }

    /// The interface sounds `screen` reports.
    fn ui_sounds(screen: &mut AppScreen) -> Vec<UiSound> {
        screen
            .take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Ui(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn keyed_n_o_p_a_and_x_click_then_open_their_screen_once_the_click_ends() {
        let store = MemoryPilots::new();
        for (press, opens) in [
            (Input::Text('n'), Showing::NewPilot),
            (Input::Text('O'), Showing::OpenPilot),
            (key(Key::Char('p'), true), Showing::Preferences),
            (key(Key::Char('a'), true), Showing::About),
            (key(Key::Char('x'), true), Showing::Note),
        ] {
            let mut screen = menu_with_dialogs(&store);
            ui_sounds(&mut screen);
            assert_eq!(screen.input(&press), ScreenAction::None);
            assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "{press:?}");
            screen.tick(Duration::from_secs(1));
            assert_eq!(screen.showing(), Showing::MainMenu, "{press:?} waits");
            screen.keyed_click_ended();
            assert_eq!(screen.showing(), opens, "{press:?}");
            screen.keyed_click_ended();
            assert_eq!(ui_sounds(&mut screen), [], "{press:?} once");
        }
    }

    #[test]
    fn a_and_x_show_the_about_text_and_desc_32766_both_munged_before_any_pilot() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        screen.input(&key(Key::Char('a'), true));
        screen.keyed_click_ended();
        assert_eq!(about(&screen).lines()[..2], ["line 0", "line 1"]);
        assert_eq!(
            about(&screen).lines()[2],
            "line 2  [Error]",
            "no pilot's name, no mission, as _DoAboutBox munges with index -1"
        );
        screen.input(&key(Key::Char('a'), false));
        screen.input(&key(Key::Enter, true));
        screen.input(&key(Key::Enter, false));
        assert_eq!(screen.showing(), Showing::MainMenu);

        screen.input(&key(Key::Char('x'), true));
        screen.keyed_click_ended();
        assert_eq!(
            screen.showing(),
            Showing::Note,
            "a text over the menu, not the About box"
        );
        assert_eq!(
            about(&screen).lines(),
            ["x 0", "x 1 First [Error]"],
            "no pilot's name, ship 128's class, no mission"
        );
        screen.input(&key(Key::Char('x'), false));
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
    }

    #[test]
    fn input_during_a_keyed_click_is_replayed_in_order_once_it_ends() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        screen.input(&Input::Text('n'));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&Input::Text('b'));
        screen.input(&key(Key::Char('b'), false));
        assert_eq!(screen.showing(), Showing::MainMenu);
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::NewPilot);
        assert_eq!(screen.new_pilot().expect("open").name(), "b");
    }

    #[test]
    fn letting_go_during_a_keyed_click_happens_in_its_place_in_the_replay() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        // E with the E key not held (another layout) clicks at once.
        screen.input(&Input::Text('e'));
        screen.input(&key(Key::Command, true));
        screen.release_keys();
        screen.input(&Input::Text('n'));
        assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed]);
        screen.keyed_click_ended();
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::EnterRefused, UiSound::ButtonKeyed],
            "Command let go before the N, which holds again"
        );
        assert_eq!(screen.showing(), Showing::MainMenu, "N's click plays");
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::NewPilot);
    }

    #[test]
    fn a_pointer_cancelled_during_a_keyed_click_is_cancelled_in_its_place() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        let at = screen
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        screen.input(&Input::Text('e'));
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        ui_sounds(&mut screen);
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::MainMenu, "cancelled, no click");
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::EnterRefused, UiSound::ButtonDown]
        );
    }

    // E.

    #[test]
    fn e_held_waits_for_its_release_to_click_then_refuses_with_no_pilot() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        screen.input(&key(Key::Char('e'), true));
        screen.input(&Input::Text('e'));
        assert_eq!(ui_sounds(&mut screen), [], "no click while E is held");
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), [], "no click hold yet");
        screen.input(&key(Key::Char('e'), false));
        assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "on release");
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), [UiSound::EnterRefused]);
        assert_eq!(screen.showing(), Showing::MainMenu);
    }

    #[test]
    fn typed_e_without_the_e_key_held_clicks_at_once() {
        let store = MemoryPilots::new();
        for typed in ['e', 'E'] {
            let mut screen = menu(&store);
            ui_sounds(&mut screen);
            screen.input(&Input::Text(typed));
            assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed], "{typed}");
            screen.keyed_click_ended();
            assert_eq!(ui_sounds(&mut screen), [UiSound::EnterRefused], "{typed}");
        }
    }

    #[test]
    fn while_e_is_held_its_repeats_are_dropped_and_the_rest_waits() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        screen.input(&key(Key::Char('e'), true));
        screen.input(&Input::Text('e'));
        for _ in 0..3 {
            screen.input(&held(Key::Char('e')));
            screen.input(&Input::Text('e'));
        }
        screen.input(&Input::Text('n'));
        ui_sounds(&mut screen);
        screen.release_keys();
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::ButtonKeyed],
            "letting go of the keys releases E"
        );
        assert_eq!(screen.showing(), Showing::MainMenu, "N waits");
        screen.keyed_click_ended();
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::EnterRefused, UiSound::ButtonKeyed],
            "one refusal, then N clicks"
        );
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::NewPilot);
    }

    #[test]
    fn command_e_does_nothing() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        ui_sounds(&mut screen);
        screen.input(&key(Key::Command, true));
        screen.input(&key(Key::Char('e'), true));
        screen.input(&Input::Text('e'));
        screen.input(&key(Key::Char('e'), false));
        screen.keyed_click_ended();
        assert_eq!(ui_sounds(&mut screen), []);
    }

    // O's guard.

    #[test]
    fn o_does_nothing_while_s_or_l_is_held() {
        let store = MemoryPilots::new();
        for guard in [Key::Char('s'), Key::Char('l')] {
            let mut screen = menu_with_dialogs(&store);
            ui_sounds(&mut screen);
            screen.input(&key(guard, true));
            screen.input(&Input::Text('o'));
            screen.keyed_click_ended();
            assert_eq!(ui_sounds(&mut screen), [], "{guard:?}");
            assert_eq!(screen.showing(), Showing::MainMenu, "{guard:?}");
            screen.input(&key(guard, false));
            screen.input(&Input::Text('o'));
            screen.keyed_click_ended();
            assert_eq!(screen.showing(), Showing::OpenPilot, "{guard:?} let go");

            let mut screen = menu_with_dialogs(&store);
            screen.input(&key(guard, true));
            screen.release_keys();
            screen.input(&Input::Text('O'));
            screen.keyed_click_ended();
            assert_eq!(screen.showing(), Showing::OpenPilot, "{guard:?} released");
        }
    }

    #[test]
    fn command_n_o_and_q_on_the_main_menu_neither_click_nor_choose() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        screen.input(&key(Key::Command, true));
        for typed in ['n', 'o', 'q', 'N'] {
            assert_eq!(screen.input(&Input::Text(typed)), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::MainMenu, "Command {typed}");
            assert_eq!(ui_sounds(&mut screen), [], "Command {typed}");
        }
        screen.input(&key(Key::Command, false));
        screen.input(&Input::Text('n'));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), Showing::NewPilot, "without Command");
    }

    #[test]
    fn the_main_menus_engine_keys_repeats_and_releases_are_silent() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        ui_sounds(&mut screen);
        for k in [Key::Char('p'), Key::Char('a'), Key::Char('x')] {
            screen.input(&held(k));
            screen.input(&key(k, false));
            assert_eq!(screen.showing(), Showing::MainMenu, "{k:?}");
            assert_eq!(ui_sounds(&mut screen), [], "{k:?}");
        }
        screen.input(&key(Key::Char('i'), true));
        assert_eq!(screen.showing(), Showing::About);
        assert_eq!(ui_sounds(&mut screen), [], "the engine's About key");
        screen.input(&key(Key::Enter, true));
        ui_sounds(&mut screen);

        // Command keys go to the original's menus, which do not click.
        screen.input(&key(Key::Command, true));
        screen.input(&key(Key::Char('p'), true));
        assert_eq!(screen.showing(), Showing::Preferences);
        assert_eq!(ui_sounds(&mut screen), [], "Command+P");
        screen.input(&key(Key::Escape, true));
        for k in [Key::Char('a'), Key::Char('x')] {
            screen.input(&key(k, true));
            assert_eq!(screen.showing(), Showing::MainMenu, "Command {k:?}");
        }
        screen.input(&key(Key::Command, false));
        assert_eq!(ui_sounds(&mut screen), []);

        // P away from the main menu is no main-screen key.
        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Char('p'), true));
        assert_eq!(screen.showing(), Showing::Preferences);
        assert_eq!(ui_sounds(&mut screen), [], "the ship browser");
    }

    #[test]
    fn without_dialogs_a_and_x_open_nothing_and_make_no_sound() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        ui_sounds(&mut screen);
        for k in [Key::Char('a'), Key::Char('x')] {
            screen.input(&key(k, true));
            assert_eq!(screen.showing(), Showing::MainMenu, "{k:?}");
        }
        assert_eq!(ui_sounds(&mut screen), []);
    }

    #[test]
    fn tab_goes_from_the_menu_to_the_developer_sides_and_escape_comes_back() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        screen.input(&key(Key::Char('f'), true));
        assert_eq!(screen.showing(), Showing::MainMenu, "F is a developer key");
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::MainMenu);
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::MainMenu);
        screen.input(&key(Key::Tab, true));
        enter(&mut screen, 128);
        assert_eq!(screen.showing(), Showing::System);
        screen.input(&key(Key::Escape, true));
        assert_eq!(
            screen.showing(),
            Showing::GalaxyMap,
            "back to the map first"
        );
    }

    /// Slot history with berth 1 in the escorts' layer and holding
    /// `shïp` 140's frames, as a flight that led a ship of that type there
    /// leaves it.
    fn led_berth_1() -> SlotSprites {
        let mut sprites = SlotSprites::default();
        sprites.layers.put(
            nova_sim::ship_layer::Slot::Berth(1),
            nova_sim::ship_layer::ShipLayer::Led,
        );
        sprites
            .frames
            .clone_frames(nova_sim::ship_layer::Slot::Berth(1), nova_sim::ShipId(140));
        sprites
    }

    /// Gives the flight shown `sprites` as its history, as its ticks would.
    fn lay_out(screen: &mut AppScreen, sprites: SlotSprites) {
        screen.flight = screen
            .flight
            .take()
            .map(|flight| flight.with_slot_sprites(sprites));
    }

    #[test]
    fn a_pilot_loaded_after_another_starts_from_the_layers_it_left() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        assert_eq!(
            AppScreen::new(data()).slot_sprites,
            SlotSprites::default(),
            "a run starts from the launch order"
        );
        create(&mut screen, "Ada");
        lay_out(&mut screen, led_berth_1());
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert!(screen.flight_view().is_none(), "put away");
        create(&mut screen, "Bea");
        assert_eq!(pilot(&screen).name(), "Bea");
        assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
    }

    #[test]
    fn a_pilot_replaced_while_flying_hands_on_its_layers() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        lay_out(&mut screen, led_berth_1());
        let bea = nova_sim::Pilot::new(screen.data.as_ref(), "Bea").expect("a pilot");
        screen.start_flight(bea);
        assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
    }

    #[test]
    fn the_developers_flight_after_a_pilot_starts_from_the_layers_it_left() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        lay_out(&mut screen, led_berth_1());
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Char('f'), true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(flight(&screen).slot_sprites(), Some(&led_berth_1()));
    }

    #[test]
    fn new_pilot_asks_for_a_name_then_flies_and_saves_the_pilot() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(pilot(&screen).name(), "Ada");
        assert_eq!(store.keys(), ["Ada"], "saved at once");
        assert_eq!(saved(&store, "Ada"), *pilot(&screen));
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    /// One flight tick's time.
    const FLIGHT_TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

    #[test]
    fn a_new_pilot_is_shown_the_land_hint() {
        use nova_view::flight::view::{TUTORIAL_DOCK, TUTORIAL_LAND};
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        assert_eq!(flight(&screen).message(), None, "before the first tick");
        screen.tick(FLIGHT_TICK);
        let shown = flight(&screen).message().expect("the land hint");
        assert!(
            shown.starts_with(TUTORIAL_LAND) || shown.starts_with(TUTORIAL_DOCK),
            "{shown}"
        );
        assert!(shown.contains("Hit 'L'"), "{shown}");
    }

    #[test]
    fn an_opened_pilot_is_never_shown_the_tutorial() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        for _ in 0..120 {
            screen.tick(FLIGHT_TICK);
            assert_eq!(flight(&screen).message(), None);
        }
    }

    #[test]
    fn a_new_pilot_runs_the_chärs_on_start_once_and_is_saved_with_what_it_did() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(starting_data(b"^b5 X129"))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        let bit_5 = nova_sim::Bit::new(5).expect("a bit");
        assert!(pilot(&screen).control_bit(bit_5), "toggled once");
        assert!(pilot(&screen).has_explored(nova_sim::SystemId(129)));
        assert_eq!(saved(&store, "Ada"), *pilot(&screen), "saved with them");
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
        // Opening the pilot runs no OnStart: a second toggle would clear it.
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = AppScreen::new(starting_data(b"^b5 X129"))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(pilot(&screen).control_bit(bit_5));
    }

    #[test]
    fn a_new_pilot_flies_and_is_saved_with_the_gender_chosen() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::NewPilot);
        let at = screen
            .new_pilot()
            .expect("open")
            .dialog()
            .item_bounds(nova_view::ui::new_pilot::GENDER_ITEM)
            .expect("the gender control")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
        type_text(&mut screen, "Ada");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(pilot(&screen).gender(), nova_sim::Gender::Female);
        assert_eq!(saved(&store, "Ada").gender(), nova_sim::Gender::Female);
        let mut screen = menu(&store);
        create(&mut screen, "Bob");
        assert_eq!(pilot(&screen).gender(), nova_sim::Gender::Male);
    }

    /// Creates a pilot named `name` from the main menu with Strict Play
    /// ticked: New Pilot, a click on the Strict Play box, the name,
    /// Return. The router needs the interface file's dialogs.
    fn create_strict(screen: &mut AppScreen, name: &str) {
        choose(screen, MenuChoice::NewPilot);
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
    }

    #[test]
    fn a_new_pilot_made_with_strict_play_ticked_plays_strict() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        create_strict(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(pilot(&screen).strict_play());
        assert!(saved(&store, "Ada").strict_play());
        let mut screen = menu_with_dialogs(&store);
        create(&mut screen, "Bob");
        assert!(!pilot(&screen).strict_play());
    }

    #[test]
    fn the_router_passes_the_overlays_to_each_flight() {
        let on = FlightOverlays {
            title: true,
            help: true,
        };
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        assert_eq!(
            screen.flight_view().expect("flying").overlays(),
            FlightOverlays::default(),
            "off by default"
        );
        let mut screen = with_dialogs(data()).with_flight_overlays(on);
        fly(&mut screen);
        assert_eq!(screen.flight_view().expect("flying").overlays(), on);
        let store = MemoryPilots::new();
        let mut screen = menu(&store).with_flight_overlays(on);
        create(&mut screen, "Ada");
        assert_eq!(
            screen.flight_view().expect("flying").overlays(),
            on,
            "a new pilot's flight"
        );
    }

    fn flight_double_time(screen: &AppScreen) -> bool {
        screen.flight_view().expect("flying").double_time()
    }

    #[test]
    fn caps_lock_presses_toggle_double_time_and_releases_and_repeats_do_not() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        assert!(!flight_double_time(&screen), "off to begin with");
        screen.input(&key(Key::CapsLock, true));
        assert!(flight_double_time(&screen), "on");
        screen.input(&held(Key::CapsLock));
        screen.input(&key(Key::CapsLock, false));
        assert!(flight_double_time(&screen), "a repeat and a release");
        screen.input(&key(Key::CapsLock, true));
        assert!(!flight_double_time(&screen), "off");
    }

    #[test]
    fn a_toggle_under_an_overlay_reaches_the_flight_beneath() {
        let mut screen = with_dialogs(data());
        fly(&mut screen);
        open_prefs(&mut screen);
        screen.input(&key(Key::CapsLock, true));
        assert!(flight_double_time(&screen));
    }

    #[test]
    fn a_toggle_made_on_another_screen_reaches_a_flight_built_later() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        screen.input(&key(Key::CapsLock, true));
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(flight_double_time(&screen), "a new pilot's flight");
        let mut screen = with_dialogs(data());
        screen.input(&key(Key::CapsLock, true));
        fly(&mut screen);
        assert!(flight_double_time(&screen), "the developer's flight");
    }

    #[test]
    fn a_new_pilot_flies_with_the_routers_hyperspace_effects() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store).with_prefs(Prefs {
            hyperspace_effects: false,
            ..Prefs::default()
        });
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(!screen.flight_view().expect("flying").hyperspace_effects());
    }

    #[test]
    fn typing_a_name_never_opens_the_dialogs_under_it() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        choose(&mut screen, MenuChoice::NewPilot);
        for c in ['p', 'i', 'f'] {
            screen.input(&key(Key::Char(c), true));
            screen.input(&Input::Text(c));
        }
        assert_eq!(screen.showing(), Showing::NewPilot);
        assert_eq!(screen.new_pilot().expect("open").name(), "pif");
        screen.input(&key(Key::Backspace, true));
        assert_eq!(screen.new_pilot().expect("open").name(), "pi");
    }

    #[test]
    fn the_new_pilot_dialog_comes_from_the_interface_file_when_there_is_one() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        choose(&mut screen, MenuChoice::NewPilot);
        let field = screen.new_pilot().expect("open").field().rect();
        assert_eq!(field, new_pilot_template().items[7].bounds);
        let mut without = menu(&store);
        choose(&mut without, MenuChoice::NewPilot);
        assert_eq!(without.showing(), Showing::NewPilot, "the built-in one");
        let mut missing = AppScreen::new(data())
            .with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        choose(&mut missing, MenuChoice::NewPilot);
        assert_eq!(missing.showing(), Showing::NewPilot, "the built-in one");
        assert_eq!(
            missing.take_warnings(),
            ["nova: using the built-in New Pilot dialog: no DLOG 3102"]
        );
    }

    #[test]
    fn cancelling_new_pilot_goes_back_to_the_menu() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::NewPilot);
        type_text(&mut screen, "Ada");
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert!(screen.new_pilot().is_none());
        assert_eq!(store.writes(), 0);
    }

    #[test]
    fn a_name_already_saved_is_refused_whatever_its_case() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        let writes = store.writes();
        create(&mut screen, "ada");
        assert_eq!(screen.showing(), Showing::NewPilot);
        assert_eq!(
            screen.new_pilot().expect("open").refusal(),
            Some(NAME_TAKEN)
        );
        assert_eq!(store.writes(), writes);
    }

    #[test]
    fn a_name_that_cannot_be_a_file_is_refused() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "..");
        assert_eq!(screen.showing(), Showing::NewPilot);
        assert_eq!(
            screen.new_pilot().expect("open").refusal(),
            Some(UNUSABLE_NAME)
        );
        assert_eq!(store.writes(), 0);
    }

    #[test]
    fn open_pilot_lists_the_saved_pilots_and_resumes_the_chosen_one_where_it_landed() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        let landed = pilot(&screen).clone();
        let docked_at = ship(&screen).position;
        assert_eq!(landed.stellar(), Some(nova_sim::StellarId(128)));
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = menu(&store);
        assert_eq!(
            choose(&mut screen, MenuChoice::OpenPilot),
            ScreenAction::None
        );
        assert_eq!(screen.showing(), Showing::OpenPilot);
        assert_eq!(screen.pilot_list().expect("open").keys(), ["Ada"]);
        screen.take_sounds();
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "in space, no spaceport");
        assert!(screen.spaceport.is_none());
        assert!(undocked(&screen));
        assert_eq!(ship(&screen).position, docked_at, "over its stellar");
        assert_eq!(*pilot(&screen), landed);
        assert!(screen.pilot_list().is_none());
        assert_eq!(screen.take_sounds(), [], "no landing sound");
    }

    // The note on a load that fell back.

    use nova_view::ui::note::{LOAD_FALLBACK_NOTE, NOTE_DIALOG};

    /// Rewrites `key`'s save in `store` with `change` over its JSON.
    fn rewrite_save(
        store: &MemoryPilots,
        key: &str,
        change: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>),
    ) {
        let mut save: serde_json::Value =
            serde_json::from_str(&store.text(key).expect("saved")).expect("JSON");
        let fields = save.as_object_mut().expect("an object");
        assert!(fields.contains_key("stellar") && fields.contains_key("system"));
        change(fields);
        store.put(key, &save.to_string());
    }

    /// Ada, created over `store` and landed on stellar 128 in system 128,
    /// saved as if last landed on stellar 999 in system 129: a stellar no
    /// system lists.
    fn save_unlisted_ada(store: &MemoryPilots) {
        let mut screen = menu_with_dialogs(store);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(128)));
        assert_eq!(screen.quit_and_reopen(store), Showing::MainMenu);
        rewrite_save(store, "Ada", |fields| {
            fields.insert("stellar".into(), 999.into());
            fields.insert("system".into(), 129.into());
        });
    }

    /// Opens the first saved pilot from the main menu's list.
    fn open_first(screen: &mut AppScreen) {
        choose(screen, MenuChoice::OpenPilot);
        assert_eq!(screen.showing(), Showing::OpenPilot);
        screen.input(&key(Key::Enter, true));
    }

    fn flown_system(screen: &AppScreen) -> nova_sim::SystemId {
        flight(screen).session().expect("flying").system()
    }

    fn undocked(screen: &AppScreen) -> bool {
        flight(screen).session().expect("flying").landed().is_none()
    }

    #[test]
    fn an_opened_pilot_whose_stellar_no_system_lists_shows_the_note_over_the_menu_then_flies() {
        let store = MemoryPilots::new();
        save_unlisted_ada(&store);
        let mut screen = menu_with_dialogs(&store);
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Note);
        assert_eq!(screen.now_showing(), Some(Showing::Note));
        assert!(screen.pilot_list().is_none());
        assert_eq!(
            screen.load_note().map(NoteDialog::text),
            Some(LOAD_FALLBACK_NOTE)
        );
        assert!(screen.flight_view().is_some(), "built behind the note");
        assert_eq!(flown_system(&screen), nova_sim::SystemId(129));
        assert!(undocked(&screen));
        let mut note = DrawList::new();
        screen.load_note().expect("open").draw(&mut note);
        let all = drawn(&screen);
        assert_eq!(
            all.iter().skip(all.len() - note.len()).collect::<Vec<_>>(),
            note.iter().collect::<Vec<_>>(),
            "the note is drawn last, over the menu"
        );
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.load_note().is_none());
        assert_eq!(flown_system(&screen), nova_sim::SystemId(129));
        // Back on the menu and open again: the repaired save loads plainly.
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight, "the note shows once");
    }

    #[test]
    fn under_the_engines_reading_the_noted_pilot_flies_in_the_lowest_system() {
        let store = MemoryPilots::new();
        save_unlisted_ada(&store);
        let mut screen = menu_with_dialogs(&store).with_rules(
            Rulebook::default().with_override(RuleKey::UnlistedStellar, RuleSource::Engine),
        );
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Note);
        assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
        assert!(undocked(&screen));
    }

    #[test]
    fn a_normal_open_never_shows_the_note() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = menu_with_dialogs(&store);
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(undocked(&screen));
        assert!(screen.load_note().is_none());
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        create(&mut screen, "Bob");
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = menu_with_dialogs(&store);
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.load_note().is_none());
    }

    #[test]
    fn a_pilot_whose_saved_system_is_gone_notes_and_flies_in_the_lowest_that_exists() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        create(&mut screen, "Ada");
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        rewrite_save(&store, "Ada", |fields| {
            fields.insert("system".into(), 300.into());
            fields.insert("stellar".into(), serde_json::Value::Null);
        });
        let mut screen = menu_with_dialogs(&store);
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Note);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(flown_system(&screen), nova_sim::SystemId(128));
        assert!(undocked(&screen));
    }

    #[test]
    fn without_dialogs_a_repaired_open_warns_and_flies() {
        let store = MemoryPilots::new();
        save_unlisted_ada(&store);
        let mut screen = menu(&store);
        screen.take_warnings();
        open_first(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.load_note().is_none());
        assert_eq!(screen.take_warnings(), [LOAD_FALLBACK_NOTE]);
    }

    #[test]
    fn the_note_takes_every_event() {
        let store = MemoryPilots::new();
        save_unlisted_ada(&store);
        let mut screen = menu_with_dialogs(&store);
        open_first(&mut screen);
        for pressed in [true, false] {
            for typed in [Key::Char('f'), Key::Tab, Key::Char('i'), Key::Char('l')] {
                assert_eq!(screen.input(&key(typed, pressed)), ScreenAction::None);
            }
        }
        assert_eq!(screen.showing(), Showing::Note);
        screen.tick(TICK);
        screen.release_keys();
        screen.cancel_pointer();
        assert_eq!(screen.showing(), Showing::Note);
        let ok = screen
            .load_note()
            .expect("open")
            .dialog()
            .item_bounds(nova_view::ui::note::OK_ITEM)
            .expect("OK")
            .center();
        screen.take_sounds();
        let click = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        };
        screen.input(&click(true));
        assert_eq!(screen.take_sounds(), []);
        screen.input(&click(false));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(screen.take_sounds(), [], "OK is silent as it closes");
    }

    /// Never fires, and draws the last outcome: a heading of 359.
    struct LastOutcome;

    impl Chance for LastOutcome {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n - 1
        }
    }

    fn last_outcome() -> SharedChance {
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(LastOutcome));
        SharedChance::new(shared)
    }

    #[test]
    #[allow(clippy::float_cmp)] // A whole degree, exactly as drawn.
    fn open_pilot_faces_the_pilot_on_a_drawn_heading() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.quit_and_reopen(&store), Showing::MainMenu);
        let mut screen = menu(&store).with_chance(last_outcome());
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(ship(&screen).heading, 359.0);
    }

    #[test]
    #[allow(clippy::float_cmp)] // Facing up, exactly.
    fn a_new_pilot_starts_facing_up_whatever_the_chance() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store).with_chance(last_outcome());
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(ship(&screen).heading, 0.0);
    }

    #[test]
    fn cancelling_open_pilot_goes_back_to_the_menu() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        assert_eq!(
            screen.pilot_list().expect("open").keys(),
            Vec::<String>::new()
        );
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
    }

    #[test]
    fn a_pilot_that_cannot_be_opened_says_why_in_the_list() {
        let store = MemoryPilots::new();
        store.put("Broken", "not a save");
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::OpenPilot);
        let error = screen
            .pilot_list()
            .expect("open")
            .error()
            .expect("an error")
            .to_owned();
        assert!(
            error.starts_with("nova: the pilot Broken in memory: "),
            "{error}"
        );
        assert_eq!(screen.take_warnings(), [error]);
    }

    #[test]
    fn pilots_that_cannot_be_listed_say_why() {
        let store = MemoryPilots::new();
        store.fail_reads(true);
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        let message = "nova: cannot list the pilots in memory (the disk is unreadable)";
        assert_eq!(screen.pilot_list().expect("open").error(), Some(message));
        assert_eq!(screen.take_warnings(), [message]);
    }

    #[test]
    fn without_a_store_new_pilot_flies_unsaved_and_open_pilot_lists_nothing() {
        let mut screen = AppScreen::new(data()).with_pilots(None, Rc::new(MonoMetrics));
        assert_eq!(screen.showing(), Showing::MainMenu);
        choose(&mut screen, MenuChoice::OpenPilot);
        assert_eq!(
            screen.pilot_list().expect("open").keys(),
            Vec::<String>::new()
        );
        screen.input(&key(Key::Escape, true));
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        land_twice(&mut screen);
        screen.quit();
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn landing_taking_off_and_a_transaction_each_save_the_pilot() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        assert_eq!(store.writes(), 1, "created");
        land_twice(&mut screen);
        assert_eq!(store.writes(), 2, "landed");
        assert_eq!(
            saved(&store, "Ada").stellar(),
            Some(nova_sim::StellarId(128))
        );
        assert!(screen.transact(|pilot| pilot.set_cash(4_321)));
        screen.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert_eq!(store.writes(), 3, "a spaceport transaction");
        assert_eq!(saved(&store, "Ada").cash(), 4_321);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(store.writes(), 4, "took off");
        screen.tick(TICK);
        screen.input(&key(Key::Up, true));
        assert_eq!(store.writes(), 4, "flying saves nothing");
    }

    #[test]
    fn escape_to_the_menu_and_quitting_save_the_pilot() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        screen.input(&key(Key::Up, true));
        screen.tick(TICK * 10);
        let writes = store.writes();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(store.writes(), writes + 1);
        assert!(screen.flight_view().is_none(), "the pilot is put away");
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        land_twice(&mut screen);
        let writes = store.writes();
        screen.quit();
        assert_eq!(store.writes(), writes + 1, "quitting saves");
    }

    #[test]
    fn a_failed_save_is_a_warning() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        store.fail_writes(true);
        land_twice(&mut screen);
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot save the pilot Ada in memory (the disk is full)"]
        );
        assert_eq!(screen.take_warnings(), Vec::<String>::new(), "taken");
    }

    #[test]
    fn the_developer_flight_is_a_fresh_unnamed_pilot_never_saved() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        screen.input(&key(Key::Escape, true));
        let ada = store.text("Ada");
        let writes = store.writes();
        screen.input(&key(Key::Tab, true));
        fly(&mut screen);
        assert_eq!(pilot(&screen).name(), "");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        screen.quit();
        assert_eq!(store.writes(), writes, "nothing saved");
        assert_eq!(store.text("Ada"), ada);
        assert_eq!(store.keys(), ["Ada"]);
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn i_and_p_work_on_the_main_menu() {
        let store = MemoryPilots::new();
        let mut screen = menu_with_dialogs(&store);
        open_about(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
    }

    #[test]
    fn the_menus_sounds_come_through_the_router() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        let at = screen
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::OpenPilot)
            .rect
            .center();
        assert_eq!(press_and_release(&mut screen, at), [vec![DOWN], vec![UP]]);
        let cancel = screen
            .pilot_list()
            .expect("open")
            .cancel_button()
            .rect
            .center();
        assert_eq!(
            press_and_release(&mut screen, cancel),
            [vec![], vec![]],
            "the pilot list's Cancel is silent"
        );
        assert!(screen.pilot_list().is_none(), "closed");
        let at = screen
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        assert_eq!(press_and_release(&mut screen, at), [vec![DOWN], vec![UP]]);
        let ok = screen
            .new_pilot()
            .expect("open")
            .dialog()
            .item_bounds(2)
            .expect("Cancel")
            .center();
        assert_eq!(
            press_and_release(&mut screen, ok),
            [vec![], vec![]],
            "the new pilot dialog's Cancel is silent"
        );
        assert!(screen.new_pilot().is_none(), "cancelled");
    }

    #[test]
    fn the_overlays_take_pointer_cancels_and_key_releases() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        let cancel = screen
            .pilot_list()
            .expect("open")
            .cancel_button()
            .rect
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: cancel,
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.release_keys();
        screen.input(&button(false));
        assert_eq!(
            screen.showing(),
            Showing::OpenPilot,
            "the click was abandoned"
        );
        screen.tick(TICK);
        assert_eq!(screen.showing(), Showing::OpenPilot);
    }

    /// "Create a new pilot:", smaller: 326 x 213 at (0, 0), with OK (1),
    /// Cancel (2), Strict Play (4) and the Full Name field (8); the rest
    /// parked.
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
        DialogTemplate {
            bounds: Bounds::at(Point::new(0.0, 0.0), 326.0, 213.0),
            placement: Placement::Fixed,
            items,
        }
    }

    #[test]
    fn a_tab_repeat_or_release_on_the_menu_does_nothing() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        for input in [held(Key::Tab), key(Key::Tab, false)] {
            assert_eq!(screen.input(&input), ScreenAction::None);
            assert_eq!(screen.showing(), Showing::MainMenu, "{input:?}");
        }
    }

    #[test]
    fn debug_shows_the_menu_and_where_pilots_are_kept() {
        let store = MemoryPilots::new();
        let debug = format!("{:?}", menu(&store));
        assert!(
            debug.contains("menu: Some(Menu { screen: MainMenu {")
                && debug.contains(r#"pilots: Some("memory"), .. })"#),
            "{debug}"
        );
    }

    impl AppScreen {
        /// Quits as the app would, and says what a new router over `store`
        /// shows.
        fn quit_and_reopen(&mut self, store: &MemoryPilots) -> Showing {
            self.quit();
            menu(store).showing()
        }
    }

    // The Trade Center.

    use nova_sim::Good;
    use nova_view::spaceport::TradeScreen;
    use nova_view::spaceport::trade::{BUY_ITEM, TRADE_DIALOG};

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
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
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

    #[test]
    fn the_trade_center_opens_the_stellars_exchange() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        assert!(
            spaceport(&screen)
                .offered()
                .contains(&nova_sim::Service::TradeCenter)
        );
        click_port_item(&mut screen, 7);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(open.problem(), None);
        assert_eq!(open.market().row(FOOD).map(|row| row.price), Some(75));
        assert_eq!((open.market().cash, open.market().free), (1000, 10));
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    }

    #[test]
    fn an_order_trades_refreshes_the_exchange_and_saves_the_pilot() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 7);
        let writes = store.writes();
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).cash(), 250, "a click buys up to 10 tons");
        assert_eq!(pilot(&screen).held(FOOD), 10);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(open.market().row(FOOD).map(|row| row.held), Some(10));
        assert_eq!(open.market().cash, 250);
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        assert_eq!(saved(&store, "Ada").held(FOOD), 10);
        // A max sale and a max buy, the quantity dialog's default each
        // confirmed, then a click on Buy with no space.
        screen.input(&key(Key::Alt, true));
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).held(FOOD), 10, "the dialog asks first");
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).held(FOOD), 0, "everything held");
        assert_eq!(pilot(&screen).cash(), 1000);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Enter, true));
        screen.input(&key(Key::Alt, false));
        assert_eq!(pilot(&screen).held(FOOD), 10, "the hold is full");
        assert_eq!(pilot(&screen).cash(), 250);
        let writes = store.writes();
        let buy = spaceport(&screen)
            .open_trade()
            .and_then(|open| open.dialog())
            .and_then(|dialog| dialog.item_bounds(BUY_ITEM))
            .expect("Buy")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: buy,
            });
        }
        assert_eq!(pilot(&screen).held(FOOD), 10, "greyed");
        assert_eq!(store.writes(), writes, "nothing to save");
        // Escape closes the exchange; the spaceport stays.
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_trade().is_none());
        assert_eq!(screen.showing(), Showing::Spaceport);
    }

    #[test]
    fn alt_b_asks_for_a_count_and_the_count_typed_is_bought_and_saved() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 7);
        let writes = store.writes();
        screen.input(&key(Key::Alt, true));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&Input::Text('b'));
        screen.input(&key(Key::Alt, false));
        let asking = spaceport(&screen)
            .open_trade()
            .and_then(TradeScreen::quantity)
            .expect("asking");
        assert_eq!(asking.field().text(), "10", "min(1000 / 75, 10 free)");
        assert_eq!(store.writes(), writes, "nothing traded yet");
        screen.input(&key(Key::Char('3'), true));
        screen.input(&Input::Text('3'));
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).held(FOOD), 3);
        assert_eq!(pilot(&screen).cash(), 1000 - 3 * 75);
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        assert_eq!(saved(&store, "Ada").held(FOOD), 3);
        assert!(spaceport(&screen).open_trade().is_some(), "still trading");
    }

    #[test]
    fn by_the_other_count_reading_alt_b_buys_the_most_at_once() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader_by(&store, RuleSource::Bible);
        click_port_item(&mut screen, 7);
        screen.input(&key(Key::Alt, true));
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).held(FOOD), 10);
    }

    #[test]
    fn without_the_trade_dialog_the_trade_center_says_why() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(trading_data())
            .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 7);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(open.problem(), Some("no DLOG 1001"));
    }

    #[test]
    fn a_stellar_without_a_trade_center_offers_no_exchange() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        click_port_item(&mut screen, 7);
        assert!(spaceport(&screen).open_trade().is_none());
        assert!(spaceport(&screen).open_service().is_none());
    }

    // The Outfitter.

    use nova_sim::OutfitId;
    use nova_view::spaceport::outfitter::{
        BUY_ITEM as OUTFIT_BUY_ITEM, NEGATIVE_FREE_MASS, OUTFIT_DIALOG,
        SELL_ITEM as OUTFIT_SELL_ITEM,
    };

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

    /// The router over [`outfitting_data`] with the outfit dialog, keeping
    /// pilots in `store`, flying a new pilot named Ada, landed.
    fn landed_outfitter(store: &MemoryPilots) -> AppScreen {
        landed_outfitter_over(store, outfitting_data())
    }

    /// [`landed_outfitter`], over `data`.
    fn landed_outfitter_over(store: &MemoryPilots, data: Rc<GameData>) -> AppScreen {
        let mut screen = AppScreen::new(data)
            .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen
    }

    const BOOSTER: OutfitId = OutfitId(128);

    /// [`data`] with the status bar ([`barred`]), its planet an
    /// outfitter selling nothing, as no `oütf` exists.
    fn empty_outfitter_data() -> Rc<GameData> {
        let mut built = barred(game_data_fork(false, false, false, 100, b"", b"", None)).build();
        let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
        built.bytes[stellar + 0x09] |= 0x04;
        loaded_bytes(built.bytes)
    }

    #[test]
    fn an_empty_outfitter_shows_its_text_and_restores_nothing() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, empty_outfitter_data());
        assert!(
            spaceport(&screen)
                .offered()
                .contains(&nova_sim::Service::Outfitter)
        );
        let bar = bar_drawn(&screen);
        click_port_item(&mut screen, 8);
        assert!(
            spaceport(&screen).open_outfitter().is_none(),
            "never opened"
        );
        assert_eq!(
            spaceport(&screen)
                .mission_text()
                .map(|text| text.lines().join(" ")),
            Some(nova_view::spaceport::outfitter::NO_ITEMS.to_owned())
        );
        assert_eq!(bar_drawn(&screen), bar, "the status bar as it was");
        screen.input(&key(Key::Enter, true));
        assert!(
            spaceport(&screen).mission_text().is_none(),
            "the text closed"
        );
        assert_eq!(screen.showing(), Showing::Spaceport);
    }

    /// [`data`] with the status bar ([`barred`]), its planet, of tech
    /// level 1, a shipyard ([`shipbuying_data`]) and an outfitter selling
    /// nothing, as no `oütf` exists. Ship 129 carries nothing, is listed
    /// only while `!b7` (`Flags3` 0x0100), and its `OnPurchase` `M129 b7` moves the pilot to
    /// Beta Prime and empties the shipyard.
    fn empty_shops_after_a_move_data() -> Rc<GameData> {
        let mut built = barred(game_data_fork(true, false, true, 100, b"", b"", None)).build();
        let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
        built.bytes[stellar + 0x09] |= 0x04;
        built.put_u16(stellar + 0x0C, 1);
        let ship = built.layout.data_entry(Ship::TYPE, 129) + 4;
        built.put_u16(ship + 0x4E, 0xFFFF);
        built.put_u16(ship + 0x56, 0);
        built.put_u16(ship + 0x726, 0x0100);
        built.bytes[ship + 0x6C..ship + 0x6F].copy_from_slice(b"!b7");
        built.bytes[ship + 0x26A..ship + 0x271].copy_from_slice(b"M129 b7");
        loaded_bytes(built.bytes)
    }

    /// The stellar named in the status bar's nav area.
    fn nav_named(screen: &AppScreen) -> Vec<String> {
        bar_drawn(screen)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } if text.ends_with(" Prime") => Some(text),
                _ => None,
            })
            .collect()
    }

    /// The shipyard's restore after a landed `M` leaves the nav marked
    /// but undrawn; an empty shipyard draws nothing after it, while an
    /// empty outfitter's `_DrawStatus` (@0x5ff09-0x5ff10) draws it.
    #[test]
    fn an_empty_outfitter_draws_the_nav_a_shipyard_left_marked_and_an_empty_shipyard_does_not() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard_of(empty_shops_after_a_move_data(), &store);
        assert_eq!(nav_named(&screen), ["Alpha Prime"], "landed");
        click_port_item(&mut screen, 9);
        press_b(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
        assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(129)));
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_shipyard().is_none(), "closed");
        assert_eq!(nav_named(&screen), ["Beta Prime"], "the move held");

        click_port_item(&mut screen, 9);
        assert!(spaceport(&screen).open_shipyard().is_none(), "empty");
        assert_eq!(
            spaceport(&screen)
                .mission_text()
                .map(|text| text.lines().join(" ")),
            Some(nova_view::spaceport::shipyard::NO_SHIPS.to_owned())
        );
        screen.input(&key(Key::Enter, true));
        assert_eq!(
            nav_named(&screen),
            ["Beta Prime"],
            "no draw after the shipyard"
        );

        click_port_item(&mut screen, 8);
        assert!(
            spaceport(&screen).open_outfitter().is_none(),
            "never opened"
        );
        assert_eq!(nav_named(&screen), ["Alpha Prime"], "the restore drawn");
    }

    #[test]
    fn the_outfitter_opens_the_stellars_outfitter() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter(&store);
        assert!(
            spaceport(&screen)
                .offered()
                .contains(&nova_sim::Service::Outfitter)
        );
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.problem(), None);
        let booster = open.outfitter().row(BOOSTER).expect("listed");
        assert_eq!((booster.price, booster.mass), (500, 2));
        assert_eq!(
            (open.outfitter().cash, open.outfitter().free_mass),
            (1000, 10)
        );
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    }

    /// [`landed_outfitter`], with Nova's rules as `rulebook` chooses.
    fn landed_outfitter_by(store: &MemoryPilots, rulebook: &Rulebook) -> AppScreen {
        let mut screen = AppScreen::new(outfitting_data())
            .with_rulebook(rulebook)
            .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen
    }

    #[test]
    fn an_empty_outfitter_cell_clicked_follows_the_grid_selection_rule() {
        let other = Rulebook::default().with_override(RuleKey::GridSelection, RuleSource::Bible);
        for (rulebook, kept) in [(Rulebook::default(), false), (other, true)] {
            let store = MemoryPilots::new();
            let mut screen = landed_outfitter_by(&store, &rulebook);
            click_port_item(&mut screen, 8);
            let open = spaceport(&screen).open_outfitter().expect("outfitting");
            let first = open.selected_outfit();
            assert!(first.is_some(), "the first cell");
            let empty = open
                .cell_bounds(open.outfitter().rows.len())
                .expect("a cell")
                .center();
            click_at(&mut screen, empty);
            let open = spaceport(&screen).open_outfitter().expect("outfitting");
            assert_eq!(
                open.selected_outfit(),
                first.filter(|_| kept),
                "kept: {kept}"
            );
        }
    }

    const LICENSE: OutfitId = OutfitId(130);
    const LICENSED: OutfitId = OutfitId(131);

    #[test]
    fn an_outfit_purchase_that_relists_resets_the_outfitters_selection() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, licensing_data());
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().row(LICENSED), None, "hidden");
        screen.input(&key(Key::Right, true));
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.selected_outfit(), Some(LICENSE));
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).owned(LICENSE), 1);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert!(open.outfitter().row(LICENSED).is_some(), "relisted");
        assert_eq!(open.selected(), None);
        assert_eq!(open.selected_outfit(), Some(BOOSTER));
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "Buy answers for 128");
        assert_eq!(pilot(&screen).owned(LICENSE), 1);
        assert_eq!(pilot(&screen).cash(), 400);
    }

    #[test]
    fn an_outfit_purchase_that_leaves_nothing_listed_closes_the_outfitter() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, self_hiding_data());
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().rows.len(), 1);
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).cash(), 500, "bought");
        assert_eq!(pilot(&screen).owned(BOOSTER), 0, "taken away");
        assert!(spaceport(&screen).open_outfitter().is_none(), "closed");
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert!(!spaceport(&screen).left());
    }

    /// [`outfitting_data`], the booster made a cargo pod: 10 tons more
    /// cargo space (`ModType` 2) for 100 credits.
    fn cargo_pod_data() -> Rc<GameData> {
        let mut built = game_data_fork(true, true, false, 100, b"", b"", None).build();
        let pod = built.layout.data_entry(Outfit::TYPE, 128) + 4;
        built.put_u16(pod + 0x06, 2);
        built.put_u16(pod + 0x08, 10);
        built.bytes[pod + 0x0E..pod + 0x12].copy_from_slice(&100_i32.to_be_bytes());
        loaded_bytes(built.bytes)
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

    /// The router over [`cargo_pod_data`], landed, by the other reading of
    /// [`RuleKey::OutfitSaleCargo`]: Ada buys the pod (20 tons), then 12
    /// tons of food with the 900 credits left, a lot of 10 and then 2,
    /// and sells the pod in the
    /// outfitter, which stays open, keeping the 12 tons in her 10.
    fn twelve_tons_in_ten(store: &MemoryPilots) -> AppScreen {
        let rulebook =
            Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Bible);
        let mut screen = AppScreen::new(cargo_pod_data())
            .with_rulebook(&rulebook)
            .with_dialogs(Rc::new(ShopDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the pod bought");
        click_port_item(&mut screen, 7);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(pilot(&screen).held(FOOD), 12);
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the pod sold");
        assert!(spaceport(&screen).open_outfitter().is_some());
        assert_eq!(pilot(&screen).held(FOOD), 12, "kept past the holds");
        screen
    }

    #[test]
    fn closing_the_outfitter_trims_the_cargo_past_the_holds() {
        let mut screen = twelve_tons_in_ten(&MemoryPilots::new());
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert_eq!(pilot(&screen).held(FOOD), 10, "@0x5db10");
        click_port_item(&mut screen, 7);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(
            open.market().row(FOOD).map(|row| row.held),
            Some(10),
            "the shops shown afresh"
        );
    }

    // The resets past the holds: the player info opening and the hire
    // screen closing.

    use nova_view::spaceport::bar::BarScreen;

    /// The mission cargo [`past_the_holds`] carries: 9 tons, never cut.
    const MISSION_TONS: u32 = 9;

    /// [`cargo_pod_data`] whose outfitter offers mission 130 (see
    /// [`venue_mission`]) carrying [`MISSION_TONS`] of cargo aboard from
    /// its accept, and whose first ship class is for hire in the bar
    /// every day (`HireRandom` 100).
    fn past_the_holds_data() -> Rc<GameData> {
        let mut mission = venue_mission(OUTFITTER);
        let tons = i16::try_from(MISSION_TONS).expect("a few tons");
        mission[0x10..0x12].copy_from_slice(&0_i16.to_be_bytes());
        mission[0x12..0x14].copy_from_slice(&tons.to_be_bytes());
        let fork = game_data_fork(true, true, false, 100, b"", b"", None)
            .resource(Mission::TYPE, 130, Some(b"Cargo job"), &mission)
            .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job 130."));
        let mut built = fork.build();
        let pod = built.layout.data_entry(Outfit::TYPE, 128) + 4;
        built.put_u16(pod + 0x06, 2);
        built.put_u16(pod + 0x08, 10);
        built.bytes[pod + 0x0E..pod + 0x12].copy_from_slice(&100_i32.to_be_bytes());
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x38A, 100);
        loaded_bytes(built.bytes)
    }

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

    /// The router over [`past_the_holds_data`], landed, by the other
    /// reading of [`RuleKey::OutfitSaleCargo`]: Ada accepts the
    /// outfitter's mission ([`MISSION_TONS`] aboard), buys the pod (20
    /// tons), then 10 tons of food, and sells the pod, keeping 19 tons in
    /// her 10. The outfitter's close trims the food to trunc(10 x 10 /
    /// 19) = 5, leaving 14 tons in 10, as the mission cargo is never cut:
    /// each reset after it trims the food again.
    fn past_the_holds(store: &MemoryPilots) -> AppScreen {
        let rulebook =
            Rulebook::default().with_override(RuleKey::OutfitSaleCargo, RuleSource::Bible);
        let mut screen = AppScreen::new(past_the_holds_data())
            .with_rulebook(&rulebook)
            .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(offered_mission(open.offer_dialog()), Some(MissionId(130)));
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)]);
        assert_eq!(pilot(&screen).total_cargo(), i64::from(MISSION_TONS));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the pod bought");
        click_port_item(&mut screen, 7);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(pilot(&screen).held(FOOD), 10);
        click_port_item(&mut screen, 8);
        assert!(
            spaceport(&screen)
                .open_outfitter()
                .is_some_and(|open| open.offer_dialog().is_none()),
            "no offer over the sale"
        );
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the pod sold");
        assert_eq!(pilot(&screen).held(FOOD), 10, "kept past the holds");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert_eq!(pilot(&screen).held(FOOD), 5, "the close's trim");
        screen
    }

    /// The trade center's food row as the spaceport shows it.
    fn food_shown(screen: &AppScreen) -> Option<u32> {
        spaceport(screen)
            .open_trade()
            .and_then(|open| open.market().row(FOOD))
            .map(|row| row.held)
    }

    #[test]
    fn p_over_the_trade_center_resets_and_shows_the_cargo_trimmed() {
        let mut screen = past_the_holds(&MemoryPilots::new());
        click_port_item(&mut screen, 7);
        assert_eq!(food_shown(&screen), Some(5));
        screen.input(&key(PLAYER_INFO_KEY, true));
        assert!(spaceport(&screen).player_info().is_some(), "open");
        assert_eq!(pilot(&screen).held(FOOD), 3, "trunc(5 x 10 / 14) @0x5a06a");
        assert_eq!(food_shown(&screen), Some(3), "the shops shown afresh");
        let shown = spaceport(&screen)
            .player_info()
            .and_then(|open| open.info().cargo.clone())
            .expect("cargo shown");
        // The cargo line counts every ton aboard: 3 of food and the
        // mission's 9, not the 14 before the reset.
        assert!(shown.contains("12"), "the info built after: {shown}");
    }

    #[test]
    fn cancelling_the_hire_screen_resets_and_the_shops_show_it() {
        let mut screen = past_the_holds(&MemoryPilots::new());
        click_port_item(&mut screen, 10);
        assert!(spaceport(&screen).open_bar().is_some(), "in the bar");
        screen.input(&key(Key::Char('h'), true));
        assert!(
            spaceport(&screen)
                .open_bar()
                .and_then(BarScreen::hire_screen)
                .is_some(),
            "for hire"
        );
        assert_eq!(pilot(&screen).held(FOOD), 5, "opening resets nothing");
        screen.input(&key(Key::Escape, true));
        assert!(
            spaceport(&screen)
                .open_bar()
                .is_some_and(|bar| bar.hire_screen().is_none()),
            "cancelled back to the bar"
        );
        assert_eq!(pilot(&screen).held(FOOD), 3, "trunc(5 x 10 / 14) @0x49235");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_bar().is_none(), "left the bar");
        click_port_item(&mut screen, 7);
        assert_eq!(food_shown(&screen), Some(3), "the shops shown afresh");
    }

    /// [`data`] with system 128 linked to 129, landed at Alpha Prime's
    /// bar, the bar open.
    fn in_the_bar() -> AppScreen {
        let mut built = game_data_fork(false, false, false, 100, b"", b"", None).build();
        let alpha = built.layout.data_entry(System::TYPE, 128) + 4;
        built.put_u16(alpha + 0x04, 129);
        let mut screen = AppScreen::new(loaded_bytes(built.bytes))
            .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics));
        land(&mut screen);
        click_port_item(&mut screen, 10);
        assert!(spaceport(&screen).open_bar().is_some(), "in the bar");
        screen
    }

    #[test]
    fn the_bars_map_opens_over_the_spaceport_and_its_close_puts_back_the_nav() {
        let mut screen = in_the_bar();
        let nav = |screen: &AppScreen| {
            flight(screen)
                .session()
                .ok()
                .map(nova_sim::Session::nav_mode)
        };
        let landed = Some(nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(128))));
        assert_eq!(nav(&screen), landed);
        screen.input(&key(MAP, true));
        assert_eq!(screen.showing(), Showing::FlightMap);
        assert!(spaceport(&screen).open_bar().is_some(), "the bar beneath");
        let mut list = DrawList::new();
        flight(&screen).course_map().draw(&mut list);
        assert_eq!(
            drawn(&screen).iter().cloned().collect::<Vec<_>>(),
            list.iter().cloned().collect::<Vec<_>>(),
            "the map drawn alone"
        );
        screen.input(&key(MAP, false));
        let map = flight(&screen).course_map();
        let beta = map
            .view()
            .world_to_screen(map.model().system(SystemId(129)).expect("Beta").position());
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: beta,
        };
        screen.input(&left(true));
        screen.cancel_pointer();
        screen.input(&left(false));
        assert_eq!(nav(&screen), landed, "the map's click abandoned");
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: beta,
            });
        }
        assert_eq!(
            nav(&screen),
            Some(nova_sim::NavMode::Hyperspace(Some(SystemId(129)))),
            "plotted and cued"
        );
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert!(spaceport(&screen).open_bar().is_some(), "still in the bar");
        assert_eq!(nav(&screen), landed, "the bar's nav put back");
        assert_eq!(
            flight(&screen)
                .session()
                .map(|session| session.course().to_vec()),
            Ok(vec![SystemId(129)]),
            "the course kept"
        );
        screen.input(&key(MAP, true));
        assert_eq!(screen.showing(), Showing::FlightMap, "again");
        screen.input(&key(MAP, false));
        screen.input(&key(MAP, true));
        assert_eq!(screen.showing(), Showing::Spaceport, "M closes it too");
    }

    #[test]
    fn p_in_flight_resets_before_the_info_is_built() {
        let mut screen = past_the_holds(&MemoryPilots::new());
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "taken off");
        past_the_takeoff_grace(&mut screen);
        let held = pilot(&screen).held(FOOD);
        let trimmed = held * 10 / (held + MISSION_TONS);
        assert!(trimmed < held, "{held} still past the holds");
        screen.input(&key(PLAYER_INFO_KEY, true));
        let shown = screen
            .flight_player_info()
            .and_then(|open| open.info().cargo.clone())
            .expect("open, showing the cargo");
        assert_eq!(pilot(&screen).held(FOOD), trimmed, "@0x5a06a");
        let total = (trimmed + MISSION_TONS).to_string();
        assert!(shown.contains(&total), "the info built after: {shown}");
    }

    #[test]
    fn an_outfit_order_changes_the_pilot_refreshes_the_outfitter_and_saves() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter(&store);
        click_port_item(&mut screen, 8);
        let speed = flight(&screen)
            .session()
            .expect("flying")
            .handling()
            .max_speed;
        let writes = store.writes();
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1);
        assert_eq!(pilot(&screen).cash(), 500);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(1));
        assert_eq!(open.outfitter().free_mass, 8);
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        assert_eq!(saved(&store, "Ada").owned(BOOSTER), 1);
        let faster = flight(&screen)
            .session()
            .expect("flying")
            .handling()
            .max_speed;
        assert!((faster - speed - 3.0).abs() < 1e-6, "{speed} to {faster}");
        // Another, then a click on Buy with too little left to pay.
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).cash(), 0);
        let writes = store.writes();
        let buy = spaceport(&screen)
            .open_outfitter()
            .and_then(|open| open.dialog())
            .and_then(|dialog| dialog.item_bounds(OUTFIT_BUY_ITEM))
            .expect("Buy")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: buy,
            });
        }
        assert_eq!(pilot(&screen).owned(BOOSTER), 2, "greyed");
        assert_eq!(store.writes(), writes, "nothing to save");
        // S sells one back, bought since the outfitter opened: in full.
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1);
        assert_eq!(pilot(&screen).cash(), 500);
        // Escape closes the outfitter; the spaceport stays.
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert_eq!(screen.showing(), Showing::Spaceport);
    }

    #[test]
    fn a_sale_refused_in_the_loop_opens_its_words_over_the_outfitter() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, pod_data());
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Right, true));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Left, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1);
        assert_eq!(pilot(&screen).owned(OutfitId(129)), 1);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().free_mass, 5);
        assert!(open.alert_dialog().is_none());
        let (cash, writes) = (pilot(&screen).cash(), store.writes());
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "nothing sold");
        assert_eq!(pilot(&screen).cash(), cash);
        assert_eq!(store.writes(), writes, "nothing to save");
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        let alert = open.alert_dialog().expect("the words in DLOG 3003");
        assert_eq!(alert.lines().join(" "), NEGATIVE_FREE_MASS);
        screen.input(&key(Key::Escape, true));
        let open = spaceport(&screen)
            .open_outfitter()
            .expect("still outfitting");
        assert!(open.alert_dialog().is_none(), "Escape closed the words");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert_eq!(screen.showing(), Showing::Spaceport);
    }

    #[test]
    fn a_counted_sale_refused_part_way_keeps_its_sales_and_opens_its_words() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, two_pod_data());
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Right, true));
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Left, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 2);
        assert_eq!(pilot(&screen).owned(OutfitId(129)), 1);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().free_mass, 25);
        let refund = open.outfitter().refund(BOOSTER).expect("a refund");
        let (cash, writes) = (pilot(&screen).cash(), store.writes());
        // Alt-S, as a Mac sends Option-S, then a count of 2.
        screen.input(&key(Key::Alt, true));
        screen.input(&key(Key::Char('s'), true));
        screen.input(&Input::Text('ß'));
        screen.input(&key(Key::Alt, false));
        screen.input(&key(Key::Char('2'), true));
        screen.input(&Input::Text('2'));
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "one sold, then refused");
        assert_eq!(pilot(&screen).cash(), cash + refund);
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        let alert = open.alert_dialog().expect("the words in DLOG 3003");
        assert_eq!(alert.lines().join(" "), NEGATIVE_FREE_MASS);
        screen.input(&key(Key::Escape, true));
        let open = spaceport(&screen)
            .open_outfitter()
            .expect("still outfitting");
        assert!(open.alert_dialog().is_none(), "Escape closed the words");
    }

    #[test]
    fn alt_on_buy_asks_for_a_count_and_buys_the_count_typed() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter(&store);
        click_port_item(&mut screen, 8);
        let writes = store.writes();
        screen.input(&key(Key::Alt, true));
        let buy = spaceport(&screen)
            .open_outfitter()
            .and_then(|open| open.dialog())
            .and_then(|dialog| dialog.item_bounds(OUTFIT_BUY_ITEM))
            .expect("Buy")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: buy,
            });
        }
        screen.input(&key(Key::Alt, false));
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        let asking = open.quantity().expect("asking");
        assert_eq!(asking.field().text(), "2", "1000 / 500");
        assert_eq!(pilot(&screen).owned(BOOSTER), 0, "the dialog asks first");
        screen.input(&key(Key::Char('2'), true));
        screen.input(&Input::Text('2'));
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 2);
        assert_eq!(pilot(&screen).cash(), 0, "two prices");
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert!(open.quantity().is_none(), "closed");
        assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(2));
        assert_eq!(store.writes(), writes + 1, "saved after the input");
    }

    #[test]
    fn a_move_by_a_purchase_hook_keeps_the_spaceport_open_and_saves_the_pilot_moved() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"M129"));
        click_port_item(&mut screen, 8);
        let writes = store.writes();
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(screen.showing(), Showing::Spaceport, "still landed");
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(128));
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().row(BOOSTER).map(|row| row.owned), Some(1));
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.landed(), Some(nova_sim::StellarId(128)));
        assert!(session.market().is_some(), "the port's exchange still");
        assert_eq!(pilot(&screen).system(), nova_sim::SystemId(129));
        assert_eq!(pilot(&screen).stellar(), Some(nova_sim::StellarId(129)));
        assert_eq!(store.writes(), writes + 1, "saved once, after the move");
        assert_eq!(saved(&store, "Ada").system(), nova_sim::SystemId(129));
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        let scene = flight(&screen)
            .scene()
            .map(nova_view::system::SystemScene::id);
        assert_eq!(scene, Some(nova_sim::SystemId(129)), "taken off there");
    }

    #[test]
    fn a_q_by_a_purchase_hook_closes_the_spaceport_takes_off_and_saves() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"Q25048"));
        click_port_item(&mut screen, 8);
        let writes = store.writes();
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.spaceport_view().is_none());
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "bought first");
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.landed(), None);
        assert_eq!(flight(&screen).message(), Some("Off you go, <PSN>."));
        assert_eq!(store.writes(), writes + 1, "saved once, after the input");
        assert_eq!(saved(&store, "Ada").owned(BOOSTER), 1);
        let sounds = screen.take_sounds();
        let took_off = sounds
            .iter()
            .position(|&sound| sound == Sound::Sim(nova_sim::SimSound::TookOff));
        assert!(took_off.is_some(), "{sounds:?}");
        assert!(
            sounds
                .iter()
                .position(|&sound| sound == halted())
                .is_some_and(|halt| Some(halt) < took_off),
            "the ambient sound halts with the spaceport: {sounds:?}"
        );
        assert!(
            !sounds.contains(&Sound::Sim(nova_sim::SimSound::ScriptMessage)),
            "no beep on the take-off"
        );
        screen.input(&key(Key::Char('l'), true));
        assert_eq!(screen.showing(), Showing::Flight, "flight takes the keys");
    }

    #[test]
    fn a_sound_by_a_purchase_hook_plays_after_the_take_off() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, hooked_outfitting_data(b"P300"));
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        screen.tick(Duration::from_millis(100));
        let mission = Sound::Sim(SimSound::Script {
            sound: nova_sim::SoundId(300),
            exclusive: true,
        });
        assert!(!screen.take_sounds().contains(&mission), "landed");
        screen.input(&key(Key::Escape, true));
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        screen.take_sounds();
        screen.tick(Duration::from_millis(100));
        assert_eq!(
            screen
                .take_sounds()
                .into_iter()
                .filter(|&sound| sound == mission)
                .count(),
            1
        );
    }

    #[test]
    fn a_new_pilot_moved_by_its_on_start_flies_there_and_is_saved_there() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(starting_data(b"M129"))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(pilot(&screen).system(), nova_sim::SystemId(129));
        let scene = flight(&screen)
            .scene()
            .map(nova_view::system::SystemScene::id);
        assert_eq!(scene, Some(nova_sim::SystemId(129)));
        assert_eq!(saved(&store, "Ada").system(), nova_sim::SystemId(129));
    }

    #[test]
    fn without_the_outfit_dialog_the_outfitter_says_why() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(outfitting_data())
            .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.problem(), Some("no DLOG 1002"));
    }

    #[test]
    fn a_stellar_without_an_outfitter_offers_none() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 8);
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert!(spaceport(&screen).open_service().is_none());
    }

    #[test]
    fn an_order_in_one_refreshes_the_other_for_when_it_opens() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(outfitting_data())
            .with_dialogs(Rc::new(EveryDialog), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 7);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!(open.market().cash, 500, "the outfit was paid for");
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).held(FOOD), 6, "500 at 75");
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().cash, 50, "the food was paid for");
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

    /// [`landed_shipyard`], on `data`.
    fn landed_shipyard_of(data: Rc<GameData>, store: &MemoryPilots) -> AppScreen {
        let mut screen = AppScreen::new(data)
            .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen
    }

    #[test]
    fn the_shipyard_opens_the_stellars_shipyard() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard(&store);
        assert!(
            spaceport(&screen)
                .offered()
                .contains(&nova_sim::Service::Shipyard)
        );
        click_port_item(&mut screen, 9);
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        assert_eq!(open.problem(), None);
        let second = open.shipyard().row(ShipId(129)).expect("listed");
        assert_eq!(second.price, 800);
        assert_eq!(open.shipyard().row(ShipId(128)), None, "BuyRandom 0");
        assert_eq!((open.shipyard().cash, open.shipyard().trade_in), (1000, 0));
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    }

    /// Presses B, as the platform sends it: the key, then its character.
    fn press_b(screen: &mut AppScreen) {
        screen.input(&key(Key::Char('b'), true));
        screen.input(&Input::Text('b'));
        screen.input(&key(Key::Char('b'), false));
    }

    /// The name prompt open over the shipyard, if it is.
    fn name_prompt(screen: &AppScreen) -> Option<&nova_view::ui::TextInputDialog> {
        spaceport(screen)
            .open_shipyard()
            .and_then(nova_view::spaceport::ShipyardScreen::naming)
    }

    #[test]
    fn a_ship_order_changes_the_pilots_ship_refreshes_the_shops_and_saves() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard(&store);
        click_port_item(&mut screen, 9);
        let writes = store.writes();
        press_b(&mut screen);
        assert_eq!(pilot(&screen).ship(), ShipId(128), "named first");
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).ship(), ShipId(129));
        assert_eq!(pilot(&screen).cash(), 200);
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "its default booster");
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        assert_eq!(saved(&store, "Ada").ship(), ShipId(129));
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        assert_eq!(open.shipyard().current, ShipId(129));
        assert_eq!(open.shipyard().cash, 200);
        let speed = flight(&screen)
            .session()
            .expect("flying")
            .handling()
            .max_speed;
        assert!(
            (speed - 9.0).abs() < 1e-6,
            "600 and the booster's 300: {speed}"
        );
        // The outfitter and the exchange read the new ship.
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(open.outfitter().cash, 200);
        assert_eq!(open.outfitter().free_mass, 12);
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 7);
        let open = spaceport(&screen).open_trade().expect("trading");
        assert_eq!((open.market().cash, open.market().capacity), (200, 20));
        assert_eq!(screen.showing(), Showing::Spaceport);
    }

    #[test]
    fn a_refused_ship_order_changes_nothing_and_saves_nothing() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard_of(dear_shipbuying_data(), &store);
        click_port_item(&mut screen, 9);
        press_b(&mut screen);
        screen.input(&key(Key::Enter, true));
        let writes = store.writes();
        let before = pilot(&screen).clone();
        // Ship 130: 200 and a 400 trade-in do not cover 5,000.
        screen.input(&key(Key::Right, true));
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        assert_eq!(
            open.shipyard().can(ShipId(130)),
            Err(nova_sim::ShipRefusal::CannotAfford)
        );
        press_b(&mut screen);
        assert!(name_prompt(&screen).is_none(), "nothing to name");
        screen.input(&key(Key::Enter, true));
        assert_eq!(*pilot(&screen), before);
        assert_eq!(store.writes(), writes, "nothing to save");
    }

    #[test]
    fn buying_a_ship_asks_its_name_and_keeps_the_name_confirmed() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard(&store);
        click_port_item(&mut screen, 9);
        assert_eq!(pilot(&screen).ship_name(), Some("First"));
        let writes = store.writes();
        press_b(&mut screen);
        let prompt = name_prompt(&screen).expect("the name prompt");
        let default = prompt.field().text().to_owned();
        let digits = default.strip_prefix("Second ").expect("the class's name");
        assert_eq!(digits.len(), 3, "{default}");
        assert!(
            digits.chars().all(|digit| ('1'..='9').contains(&digit)),
            "{default}"
        );
        assert!(prompt.field().selected(), "the B typed nothing");
        let texts: Vec<String> = drawn(&screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|text| text.contains("Please name your new Second Shipyard:")),
            "{texts:?}"
        );
        assert_eq!(store.writes(), writes, "nothing bought yet");
        let message = flight(&screen).message().map(str::to_owned);
        type_text(&mut screen, "The Kestrel");
        screen.input(&key(Key::Enter, true));
        assert!(name_prompt(&screen).is_none(), "closed");
        assert_eq!(pilot(&screen).ship(), ShipId(129));
        assert_eq!(pilot(&screen).ship_name(), Some("Kestrel"));
        assert_eq!(store.writes(), writes + 1, "saved after the input");
        assert_eq!(saved(&store, "Ada").ship_name(), Some("Kestrel"));
        assert_eq!(
            flight(&screen).message().map(str::to_owned),
            message,
            "no message follows"
        );
        assert!(spaceport(&screen).open_shipyard().is_some());
    }

    #[test]
    fn typing_a_ship_name_never_opens_the_dialogs_under_it() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard(&store);
        click_port_item(&mut screen, 9);
        press_b(&mut screen);
        for c in ['p', 'i', 'f'] {
            screen.input(&key(Key::Char(c), true));
            screen.input(&Input::Text(c));
            screen.input(&key(Key::Char(c), false));
        }
        assert!(screen.preferences().is_none(), "no Preferences");
        assert_eq!(screen.showing(), Showing::Spaceport);
        let prompt = name_prompt(&screen).expect("the name prompt");
        assert_eq!(prompt.field().text(), "pif");
        // Once it is closed, P opens the player info.
        click_cancel(&mut screen);
        assert!(name_prompt(&screen).is_none(), "declined");
        screen.input(&key(Key::Char('p'), true));
        assert!(screen.preferences().is_none());
        assert!(
            spaceport(&screen).player_info().is_some(),
            "P is the player info"
        );
    }

    #[test]
    fn cancelling_the_name_prompt_buys_nothing_and_saves_nothing() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard(&store);
        click_port_item(&mut screen, 9);
        let writes = store.writes();
        let before = pilot(&screen).clone();
        press_b(&mut screen);
        click_cancel(&mut screen);
        assert!(name_prompt(&screen).is_none(), "closed");
        assert_eq!(*pilot(&screen), before);
        assert_eq!(store.writes(), writes, "nothing to save");
        assert!(spaceport(&screen).open_shipyard().is_some());
    }

    /// Clicks the open name prompt's Cancel.
    fn click_cancel(screen: &mut AppScreen) {
        let cancel = name_prompt(screen)
            .expect("the name prompt")
            .dialog()
            .item_bounds(6)
            .expect("Cancel")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: cancel,
            });
        }
    }

    /// The `BuyRandom` ship 129 sells on in [`rolling_shipyard`]: a
    /// chance no other draw asks.
    const ROLLED: u8 = 37;

    /// Answers each [`ROLLED`] % draw from its script, then misses, and
    /// counts them; every other chance misses, as [`nova_sim::NeverFires`]
    /// does.
    #[derive(Default)]
    struct BuyRolls {
        script: VecDeque<bool>,
        asked: usize,
    }

    impl Chance for BuyRolls {
        fn fires(&mut self, percent: u8) -> bool {
            if percent != ROLLED {
                return false;
            }
            self.asked += 1;
            self.script.pop_front().unwrap_or(false)
        }

        fn below(&mut self, n: u32) -> u32 {
            n.saturating_sub(1)
        }

        fn roll(&mut self, _sides: u16) -> u16 {
            0
        }
    }

    /// [`landed_shipyard`], ship 129 for sale on a chance of [`ROLLED`] %
    /// drawn from `script`, and the draws.
    fn rolling_shipyard(
        store: &MemoryPilots,
        script: &[bool],
    ) -> (AppScreen, Rc<RefCell<BuyRolls>>) {
        let rolls = Rc::new(RefCell::new(BuyRolls {
            script: script.iter().copied().collect(),
            asked: 0,
        }));
        let shared: Rc<RefCell<dyn Chance>> = rolls.clone();
        let mut screen = AppScreen::new(game_data_selling(true, true, true, i16::from(ROLLED)))
            .with_dialogs(Rc::new(ShipyardDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
            .with_chance(SharedChance::new(shared));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        (screen, rolls)
    }

    fn lists_second(screen: &AppScreen) -> bool {
        let open = spaceport(screen).open_shipyard().expect("shipbuying");
        open.shipyard().row(ShipId(129)).is_some()
    }

    #[test]
    fn a_ship_declined_is_named_again_on_the_list_built_at_opening() {
        let store = MemoryPilots::new();
        let (mut screen, rolls) = rolling_shipyard(&store, &[true, false]);
        click_port_item(&mut screen, 9);
        assert!(lists_second(&screen));
        assert_eq!(rolls.borrow().asked, 1, "drawn as the list was built");
        press_b(&mut screen);
        click_cancel(&mut screen);
        assert!(name_prompt(&screen).is_none(), "closed");
        press_b(&mut screen);
        assert!(name_prompt(&screen).is_some(), "the prompt opens again");
        assert_eq!(rolls.borrow().asked, 1, "nothing drawn again yet");
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).ship(), ShipId(129), "bought");
    }

    #[test]
    fn a_ship_declined_draws_its_roll_again_when_the_list_is_next_built() {
        let store = MemoryPilots::new();
        let (mut screen, rolls) = rolling_shipyard(&store, &[true, false]);
        click_port_item(&mut screen, 9);
        press_b(&mut screen);
        click_cancel(&mut screen);
        assert!(lists_second(&screen), "the list is kept for the visit");
        assert_eq!(rolls.borrow().asked, 1);
        // An order builds the list again.
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 8);
        screen.input(&key(Key::Char('b'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1, "the outfit was bought");
        assert_eq!(rolls.borrow().asked, 2, "drawn again");
        screen.input(&key(Key::Escape, true));
        click_port_item(&mut screen, 9);
        assert!(
            spaceport(&screen).open_shipyard().is_none(),
            "the new roll missed: nothing listed, it never opens"
        );
    }

    #[test]
    fn a_stellar_without_a_shipyard_offers_none() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter(&store);
        click_port_item(&mut screen, 9);
        assert!(spaceport(&screen).open_shipyard().is_none());
        assert!(spaceport(&screen).open_service().is_none());
    }

    #[test]
    fn without_the_shipyard_dialog_the_shipyard_says_why() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(shipbuying_data())
            .with_dialogs(Rc::new(EveryDialog), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 9);
        let open = spaceport(&screen).open_shipyard().expect("shipbuying");
        assert_eq!(open.problem(), Some("no DLOG 1004"));
    }

    // Boarding.

    use std::cell::RefCell;
    use std::collections::VecDeque;

    use nova_data::records::dude::Dude;
    use nova_sim::reserves::Gauge;
    use nova_sim::{Chance, Goal, HullSpec, Npc, Surroundings};
    use nova_view::ui::plunder::{
        ASSIGNMENT_DIALOG, ESCORT_ITEM, MY_SHIP_ITEM, PLUNDER_DIALOG, PLUNDER_PICTURE,
    };

    /// Every ship that holds armour is disabled: the trader (100), never
    /// the player (none).
    #[derive(Debug)]
    struct Downs;

    impl DisableRule for Downs {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            true
        }
    }

    /// Every NPC idles.
    #[derive(Debug)]
    struct Idle;

    impl Behaviour for Idle {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    /// Never repels, gives odds of 75, and always captures; a person
    /// carries its `Credits`, and grants nothing.
    #[derive(Debug)]
    struct Sure;

    impl BoardingRule for Sure {
        fn repels(&self, _booty: u16) -> bool {
            false
        }

        fn capture_odds(
            &self,
            _crew: &nova_sim::board::CaptureCrew,
            _chance: &mut dyn Chance,
        ) -> u8 {
            75
        }

        fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
            true
        }

        fn person_credits(&self, credits: i32, _chance: &mut dyn Chance) -> i64 {
            i64::from(credits)
        }

        fn grant(
            &self,
            _grant: &nova_sim::grant::PersonGrant,
            _stock: &[nova_sim::grant::GrantStock],
            _free_mass: i64,
            _grant_max: nova_sim::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<nova_sim::grant::Granted> {
            None
        }
    }

    /// Draws its script, then the last outcome.
    struct Script(VecDeque<u32>);

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.0.pop_front().unwrap_or(n - 1)
        }
    }

    /// The traffic's setup draws: the trader under the player at the
    /// centre, facing up; then the last outcomes.
    fn under_the_player() -> SharedChance {
        scripted_under_the_player().1
    }

    /// [`under_the_player`]'s chance, with its script, to draw more from.
    fn scripted_under_the_player() -> (Rc<RefCell<Script>>, SharedChance) {
        let script = Rc::new(RefCell::new(Script([6, 6, 0, 0, 750, 750, 0, 0].into())));
        let shared: Rc<RefCell<dyn Chance>> = script.clone();
        (script, SharedChance::new(shared))
    }

    /// The draws a new pilot's flight makes: the offers' roll table its
    /// beginning draws, each the last outcome, then the traffic's setup,
    /// the trader under a new pilot's ship at (50, 50), where
    /// [`nova_sim::Session::begin`] puts it, facing up; then the last
    /// outcomes.
    fn under_a_new_pilot() -> SharedChance {
        let table = std::iter::repeat_n(99, nova_sim::mission::ROLLED);
        let setup = [6, 6, 0, 0, 800, 800, 0, 0];
        let script: Rc<RefCell<dyn Chance>> =
            Rc::new(RefCell::new(Script(table.chain(setup).collect())));
        SharedChance::new(script)
    }

    /// [`boarding_data_with`] no cash.
    fn boarding_data() -> Rc<GameData> {
        boarding_data_with(0)
    }

    /// The first `chär` flies ship 128 ("First": average, a crew of 10)
    /// in system 128 with `cash`, whose one `düde`, at 100 %, flies ship
    /// 129 ("Trader": a crew of 3, 100 armour, 20 holds, 300 fuel, `Cost`
    /// 150,000, and no speed, so it stays where setup places it) for food
    /// and money (`Booty` 0x0041), independent. Food is commodity 0.
    fn boarding_data_with(cash: i32) -> Rc<GameData> {
        let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
            for (i, value) in values.iter().enumerate() {
                bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
            }
        };
        let ship = |holds, armor, crew| {
            let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
            put(&mut bytes, 0x00, &[holds, 0, 300, 300, 10, 300, 0, armor]);
            put(&mut bytes, 0x12, &[-1; 4]);
            put(&mut bytes, 0x42, &[1, crew]);
            put(&mut bytes, 0x4E, &[-1; 4]);
            put(&mut bytes, 0x370, &[-1; 4]);
            put(&mut bytes, 0x6CE, &[-1; 4]);
            bytes
        };
        let mut trader = ship(20, 100, 3);
        put(&mut trader, 0x06, &[0]);
        trader[0x30..0x34].copy_from_slice(&150_000_i32.to_be_bytes());
        let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
        put(&mut anim, 0x00, &[1000, 0, 1]);
        put(&mut anim, 0x34, &[4]);
        let sheet = (0..4)
            .fold(RledBuilder::new(1, 1), |sheet, _| {
                sheet.frame(|f| f.line().pixels(&[0x7C00]))
            })
            .build();
        let mut character = vec![0; Character::SIZE.expect("fixed")];
        character[0x00..0x04].copy_from_slice(&cash.to_be_bytes());
        put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
        put(&mut character, 0x0E, &[-1; 4]);
        let mut system = vec![0; System::SIZE.expect("fixed")];
        put(&mut system, 0x04, &[-1; 32]);
        put(&mut system, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
        put(&mut system, 0x54, &[100]);
        put(&mut system, 0x64, &[1, -1]);
        let mut dude = vec![0; Dude::SIZE.expect("fixed")];
        put(&mut dude, 0x00, &[1, -1, 0x0041]);
        put(&mut dude, 0x08, &[-1; 16]);
        put(&mut dude, 0x08, &[129]);
        put(&mut dude, 0x28, &[100]);
        let fork = ForkBuilder::new()
            .resource(Ship::TYPE, 128, Some(b"First"), &ship(0, 0, 10))
            .resource(Ship::TYPE, 129, Some(b"Trader"), &trader)
            .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system)
            .resource(Dude::TYPE, 128, Some(b"Traders"), &dude)
            .resource(StrList::TYPE, 4000, None, &str_list(&["Food"]))
            .resource(StrList::TYPE, 4004, None, &str_list(&["75"]))
            .resource(
                StrList::TYPE,
                3000,
                None,
                &str_list(&as_strs(&comm_strings())),
            )
            .resource(StrList::TYPE, 2002, None, &str_list(&as_strs(&messages())))
            .resource(
                StrList::TYPE,
                9000,
                None,
                &str_list(&["Pay up.", "Paid.", "Declined.", "Short."]),
            )
            .build()
            .bytes;
        let file = OneFile(fork);
        Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
    }

    /// The ship comm strings: "Channel open." in every variant of the
    /// first group, and "c<n>" for each other.
    fn comm_strings() -> Vec<String> {
        (1..=200)
            .map(|n| {
                if n <= 5 {
                    "Channel open.".to_owned()
                } else {
                    format!("c{n}")
                }
            })
            .collect()
    }

    /// The game's messages: "Greetings." at #175, "m<n>" for each other.
    fn messages() -> Vec<String> {
        (1..=200)
            .map(|n| {
                if n == 175 {
                    "Greetings.".to_owned()
                } else {
                    format!("m{n}")
                }
            })
            .collect()
    }

    fn as_strs(strings: &[String]) -> Vec<&str> {
        strings.iter().map(String::as_str).collect()
    }

    /// `screen` over [`boarding_data`], its trader under the player,
    /// idling and disabled at once.
    fn boardable(screen: AppScreen) -> AppScreen {
        screen
            .with_chance(under_the_player())
            .with_behaviour(Rc::new(Idle))
            .with_disable_rule(Rc::new(Downs))
    }

    /// Ticks once (the system is set up and the trader disabled), targets
    /// the trader with Tab and boards it with B.
    fn board(screen: &mut AppScreen) {
        screen.tick(TICK);
        screen.input(&key(Key::Tab, true));
        screen.input(&key(Key::Tab, false));
        // Tab's beep, so a test hears only what follows.
        screen.take_sounds();
        screen.input(&key(Key::Char('b'), true));
        screen.input(&key(Key::Char('b'), false));
    }

    /// The router over [`boarding_data`] with dialogs, in the developer's
    /// flight, boarded, its sounds taken.
    fn boarded() -> AppScreen {
        let mut screen = boardable(with_dialogs(boarding_data()));
        fly(&mut screen);
        let _ = screen.take_sounds();
        board(&mut screen);
        assert_eq!(screen.showing(), Showing::Plunder);
        assert!(
            screen
                .take_sounds()
                .contains(&Sound::Sim(SimSound::Boarded)),
            "the airlock, as the dialog opens"
        );
        screen
    }

    fn plunder_click(screen: &mut AppScreen, item: usize) -> Vec<Sound> {
        let at = screen
            .plunder()
            .expect("the plunder dialog is open")
            .dialog()
            .item_bounds(item)
            .expect("an item")
            .center();
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(screen.take_sounds());
        }
        sounds
    }

    fn assignment_click(screen: &mut AppScreen, item: usize) {
        let at = screen
            .assignment()
            .expect("the assignment dialog is open")
            .dialog()
            .item_bounds(item)
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

    #[test]
    fn b_on_a_disabled_target_opens_the_plunder_dialog_over_paused_flight() {
        let mut screen = boarded();
        let dialog = screen.plunder().expect("open");
        assert_eq!(dialog.lines()[2], "Cargo: 19 tons of Food");
        assert_eq!(dialog.lines()[4], "Capture Odds: 28%");
        let alpha = flight(&screen).alpha();
        screen.tick(TICK / 2);
        assert_eq!(
            flight(&screen).alpha().to_bits(),
            alpha.to_bits(),
            "flight is paused"
        );
        let list = drawn(&screen);
        let texts: Vec<String> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Capture Ship"), "{texts:?}");
        assert!(
            texts.iter().position(|text| text == "Capture Ship")
                > texts.iter().position(|text| text.contains("Alpha")),
            "over flight"
        );
    }

    #[test]
    fn each_take_goes_through_the_flight_and_refreshes_the_dialog() {
        let mut screen = boarded();
        let cash = flight(&screen).pilot().expect("a pilot").cash();
        let sounds = plunder_click(&mut screen, 3);
        assert_eq!(sounds, []);
        assert_eq!(
            flight(&screen).pilot().expect("a pilot").cash(),
            cash + 5750
        );
        assert_eq!(
            flight(&screen).message(),
            Some("You stole all the 5750 credits from this ship.")
        );
        assert_eq!(screen.showing(), Showing::Plunder, "still open");
        plunder_click(&mut screen, 3);
        assert_eq!(
            flight(&screen).pilot().expect("a pilot").cash(),
            cash + 5750,
            "greyed: taken once"
        );
        plunder_click(&mut screen, 2);
        let dialog = screen.plunder().expect("open");
        assert_eq!(dialog.lines()[2], "Cargo: N/A", "refreshed");
    }

    #[test]
    fn abort_closes_the_dialog_back_into_flight() {
        let mut screen = boarded();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.plunder().is_none());
        assert_eq!(flight(&screen).boarding(), None);
        screen.input(&key(Key::Escape, false));
        assert_eq!(
            screen.showing(),
            Showing::Flight,
            "the release is the dialog's"
        );
    }

    #[test]
    fn the_plunder_dialog_comes_from_the_interface_file_when_there_is_one() {
        /// "Plunder" (1011), as stock, and nothing else.
        struct Plunder;

        impl DialogResources for Plunder {
            fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
                if id != PLUNDER_DIALOG {
                    return Err(format!("no DLOG {id}"));
                }
                let user = |x, y, w, h, enabled| ItemTemplate {
                    bounds: Bounds::at(Point::new(x, y), w, h),
                    enabled,
                    kind: ItemSpec::User,
                };
                Ok(DialogTemplate {
                    bounds: Bounds::at(Point::new(0.0, 0.0), 309.0, 198.0),
                    placement: Placement::Fixed,
                    items: vec![
                        user(91.0, 166.0, 126.0, 25.0, true),
                        user(110.0, 110.0, 89.0, 25.0, true),
                        user(35.0, 138.0, 89.0, 25.0, true),
                        user(204.0, 110.0, 89.0, 25.0, true),
                        user(11.0, 7.0, 287.0, 96.0, false),
                        user(16.0, 110.0, 89.0, 25.0, true),
                        user(129.0, 138.0, 146.0, 25.0, true),
                    ],
                })
            }
        }

        let mut screen = boardable(
            AppScreen::new(boarding_data()).with_dialogs(Rc::new(Plunder), Rc::new(MonoMetrics)),
        );
        fly(&mut screen);
        board(&mut screen);
        let pictures: Vec<_> = drawn(&screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StretchedPicture {
                    image, top_left, ..
                } => Some((*image, *top_left)),
                _ => None,
            })
            .collect();
        assert!(
            pictures.contains(&(PLUNDER_PICTURE, Point::new(0.0, 0.0))),
            "{pictures:?}"
        );
    }

    #[test]
    fn without_any_metrics_a_boarding_is_let_go() {
        let mut screen = boardable(AppScreen::new(boarding_data()));
        fly(&mut screen);
        board(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(flight(&screen).boarding(), None, "aborted");
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot show the plunder dialog: no metrics to lay it out"]
        );
    }

    /// The router over [`boarding_data`] with dialogs and pilots kept in
    /// `store`, capturing surely, flying the new pilot "Ada", boarded and
    /// captured.
    fn captured(store: &MemoryPilots) -> AppScreen {
        let mut screen = boardable(
            with_dialogs(boarding_data()).with_pilots(Some(keeper(store)), Rc::new(MonoMetrics)),
        )
        .with_chance(under_a_new_pilot())
        .with_boarding_rule(Rc::new(Sure));
        create(&mut screen, "Ada");
        board(&mut screen);
        assert_eq!(screen.showing(), Showing::Plunder);
        plunder_click(&mut screen, 7);
        assert_eq!(screen.showing(), Showing::Assignment);
        assert!(screen.plunder().is_none());
        screen
    }

    #[test]
    fn a_capture_asks_its_assignment_and_use_as_escort_joins_the_fleet_and_saves() {
        let store = MemoryPilots::new();
        let mut screen = captured(&store);
        let alpha = flight(&screen).alpha();
        screen.tick(TICK / 2);
        assert_eq!(
            flight(&screen).alpha().to_bits(),
            alpha.to_bits(),
            "flight is paused"
        );
        assignment_click(&mut screen, ESCORT_ITEM);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(
            flight(&screen).message(),
            Some("You assigned this ship to your fleet of escorts.")
        );
        assert_eq!(pilot(&screen).escorts().len(), 1);
        assert_eq!(saved(&store, "Ada").escorts().len(), 1, "saved");
        assert_eq!((PLUNDER_DIALOG, ASSIGNMENT_DIALOG), (1011, 1018));
    }

    #[test]
    fn use_as_my_ship_flies_the_captured_ship() {
        let store = MemoryPilots::new();
        let mut screen = captured(&store);
        assignment_click(&mut screen, MY_SHIP_ITEM);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(129));
        assert_eq!(
            flight(&screen).message(),
            Some("You retained your old ship as an escort.")
        );
        assert_eq!(saved(&store, "Ada").ship(), nova_sim::ShipId(129));
    }

    #[test]
    fn the_overlays_take_the_pointer_cancels_and_the_key_releases() {
        let store = MemoryPilots::new();
        let mut screen = captured(&store);
        let at = screen
            .assignment()
            .expect("open")
            .dialog()
            .item_bounds(ESCORT_ITEM)
            .expect("an item")
            .center();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        screen.cancel_pointer();
        screen.release_keys();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        assert_eq!(screen.showing(), Showing::Assignment, "abandoned");
        let mut screen = boarded();
        let at = screen
            .plunder()
            .expect("open")
            .dialog()
            .item_bounds(3)
            .expect("an item")
            .center();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        screen.cancel_pointer();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at,
        });
        assert_eq!(
            flight(&screen).boarding().map(|view| view.credits),
            Some(5750),
            "abandoned"
        );
    }

    // Hailing.

    use nova_sim::Reply;
    use nova_sim::hail::{Answer, Ask, Hail, HailOption};
    use nova_view::ui::comm::{CLOSE_ITEM, COMM_DIALOG, HAGGLE_DIALOG, LOWER_ITEM};

    /// An option that asks a price, saying `STR#` 9000's strings: "Pay
    /// up." asked, "Paid." paid, "Declined." declined and "Short." short.
    #[derive(Debug)]
    struct Sell;

    impl HailOption for Sell {
        fn label(&self) -> String {
            "Sell".to_owned()
        }

        fn key(&self) -> Option<char> {
            Some('S')
        }

        fn applies(&self, _hail: &Hail) -> bool {
            true
        }

        fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
            let line = |index| Reply::Line { list: 9000, index };
            Answer {
                ask: Some(Ask {
                    paid: (line(2), None),
                    declined: (line(3), None),
                    short: line(4),
                }),
                ..Answer::say(line(1))
            }
        }
    }

    /// `screen` over [`boarding_data`], its trader under the player,
    /// idling and intact, and Nova's options with [`Sell`].
    fn hailable(screen: AppScreen) -> AppScreen {
        screen
            .with_chance(under_the_player())
            .with_behaviour(Rc::new(Idle))
            .with_hail_options(HailOptions::default().with(Rc::new(Sell)))
    }

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

    /// The router over [`boarding_data`] with dialogs, in the developer's
    /// flight, hailing the trader.
    fn hailing() -> AppScreen {
        let mut screen = hailable(with_dialogs(boarding_data()));
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Comm);
        screen
    }

    fn tap(screen: &mut AppScreen, k: Key) {
        screen.input(&key(k, true));
        screen.input(&key(k, false));
    }

    fn reply(screen: &AppScreen) -> String {
        flight(screen).hailing().expect("hailing").reply
    }

    #[test]
    fn y_opens_the_comm_dialog_over_paused_flight() {
        let screen = &mut hailing();
        assert_eq!(reply(screen), "Channel open.");
        let alpha = flight(screen).alpha();
        screen.tick(TICK / 2);
        assert_eq!(
            flight(screen).alpha().to_bits(),
            alpha.to_bits(),
            "flight is paused"
        );
        let texts: Vec<String> = drawn(screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        for label in ["Greetings", "Request Assistance", "Sell", "Close Channel"] {
            assert!(texts.iter().any(|text| text == label), "{label}: {texts:?}");
        }
        assert!(
            texts.iter().position(|text| text == "Close Channel")
                > texts.iter().position(|text| text.contains("Alpha")),
            "over flight"
        );
        assert!(screen.comm().is_some());
    }

    #[test]
    fn each_press_goes_through_the_flight_and_refreshes_the_dialog() {
        let screen = &mut hailing();
        tap(screen, Key::Char('g'));
        assert_eq!(screen.showing(), Showing::Comm, "still open");
        assert_eq!(reply(screen), "Greetings.", "the trader's advice: none");
        let shown: Vec<String> = drawn(screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(shown.iter().any(|text| text == "Greetings."), "{shown:?}");
    }

    #[test]
    fn a_price_asked_opens_the_haggle_dialog_whose_choice_settles_it() {
        let screen = &mut hailing();
        tap(screen, Key::Char('s'));
        assert_eq!(screen.showing(), Showing::Haggle);
        assert_eq!(reply(screen), "Pay up.");
        let dialog = screen.haggle().expect("open");
        assert_eq!(dialog.lines(), ["Pay me 1,000 credits."]);
        tap(screen, Key::Escape);
        assert_eq!(screen.showing(), Showing::Haggle, "Escape does nothing");
        tap(screen, Key::Enter);
        assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
        assert!(screen.haggle().is_none());
        assert_eq!(reply(screen), "Short.", "no cash");
    }

    /// The router as [`hailing`] gives it, the pilot holding 10,000
    /// credits, so the trader's price is 3,000 (3,600 at a mood of 1.2,
    /// at most a third of the cash, in thousands), and Sell pressed with
    /// the haggle roll `roll`.
    fn haggling(roll: u32) -> AppScreen {
        let (script, chance) = scripted_under_the_player();
        let mut screen = hailable(with_dialogs(boarding_data_with(10_000))).with_chance(chance);
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Comm);
        script.borrow_mut().0 = [roll].into();
        tap(&mut screen, Key::Char('s'));
        assert_eq!(screen.showing(), Showing::Haggle);
        let dialog = screen.haggle().expect("open");
        assert_eq!(dialog.lines(), ["Pay me 3,000 credits."]);
        screen
    }

    fn cash(screen: &AppScreen) -> i64 {
        flight(screen).pilot().expect("a pilot").cash()
    }

    #[test]
    fn a_won_haggle_keeps_the_haggle_dialog_open_at_the_lowered_price() {
        let screen = &mut haggling(35);
        tap(screen, Key::Char('l'));
        assert_eq!(screen.showing(), Showing::Haggle, "still haggling");
        let dialog = screen.haggle().expect("open");
        assert_eq!(
            dialog.lines(),
            ["Pay me 2,200 credits."],
            "0.75 of 3,000, in hundreds, of the ship"
        );
        assert_eq!(reply(screen), "Pay up.", "unchanged");
        assert_eq!(cash(screen), 10_000);
        tap(screen, Key::Enter);
        assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
        assert!(screen.haggle().is_none());
        assert_eq!(reply(screen), "Paid.");
        assert_eq!(cash(screen), 7_800, "the lowered price paid");
    }

    #[test]
    fn a_lost_haggle_declines_and_the_comm_dialog_takes_the_next_press() {
        let screen = &mut haggling(36);
        let at = screen
            .haggle()
            .expect("open")
            .dialog()
            .item_bounds(LOWER_ITEM)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
        assert_eq!(screen.showing(), Showing::Comm, "back to the comm dialog");
        assert!(screen.haggle().is_none());
        assert_eq!(reply(screen), "Declined.");
        assert_eq!(cash(screen), 10_000, "nothing paid");
        tap(screen, Key::Char('g'));
        assert_eq!(reply(screen), "Greetings.", "not stuck");
        tap(screen, Key::Char('s'));
        assert_eq!(screen.showing(), Showing::Haggle);
        let dialog = screen.haggle().expect("open");
        assert_eq!(dialog.lines(), ["Pay me 4,000 credits."], "raised");
    }

    #[test]
    fn close_channel_hangs_up_and_resumes_flight() {
        let screen = &mut hailing();
        let at = screen
            .comm()
            .expect("open")
            .dialog()
            .item_bounds(CLOSE_ITEM)
            .expect("an item")
            .center();
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(screen.take_sounds());
        }
        assert_eq!(sounds, []);
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.comm().is_none());
        assert_eq!(flight(screen).hailing(), None, "hung up");
        let alpha = flight(screen).alpha();
        screen.tick(TICK / 2);
        assert_ne!(flight(screen).alpha().to_bits(), alpha.to_bits(), "resumed");
    }

    #[test]
    fn escape_closes_the_channel_too() {
        let screen = &mut hailing();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight);
        screen.input(&key(Key::Escape, false));
        assert_eq!(
            screen.showing(),
            Showing::Flight,
            "the release is the dialog's"
        );
    }

    #[test]
    fn the_comm_dialogs_come_from_the_interface_file_when_there_is_one() {
        /// "Comm" (1007) and "Haggle" (1008), as the built-in ones, fixed.
        struct Comm;

        impl DialogResources for Comm {
            fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
                let user = |x, y, w, h, enabled| ItemTemplate {
                    bounds: Bounds::at(Point::new(x, y), w, h),
                    enabled,
                    kind: ItemSpec::User,
                };
                match id {
                    COMM_DIALOG => Ok(DialogTemplate {
                        bounds: Bounds::at(Point::new(0.0, 0.0), 423.0, 215.0),
                        placement: Placement::Fixed,
                        items: vec![
                            user(21.0, 181.0, 166.0, 26.0, true),
                            user(21.0, 153.0, 166.0, 26.0, true),
                            user(21.0, 125.0, 166.0, 26.0, true),
                            user(46.0, 241.0, 200.0, 25.0, true),
                            user(7.0, 320.0, 200.0, 25.0, true),
                            user(199.0, 335.0, 200.0, 25.0, true),
                            user(178.0, 261.0, 200.0, 25.0, true),
                            user(178.0, 289.0, 200.0, 25.0, true),
                            user(34.0, 299.0, 112.0, 16.0, false),
                            user(11.0, 8.0, 192.0, 58.0, false),
                            user(216.0, 7.0, 200.0, 200.0, false),
                            user(40.0, 73.0, 134.0, 46.0, false),
                        ],
                    }),
                    HAGGLE_DIALOG => Ok(DialogTemplate {
                        bounds: Bounds::at(Point::new(0.0, 0.0), 262.0, 107.0),
                        placement: Placement::Fixed,
                        items: vec![
                            user(58.0, 74.0, 146.0, 26.0, true),
                            user(58.0, 39.0, 146.0, 26.0, true),
                            user(7.0, 6.0, 248.0, 25.0, false),
                        ],
                    }),
                    _ => Err(format!("no DLOG {id}")),
                }
            }
        }

        let mut screen = hailable(
            AppScreen::new(boarding_data()).with_dialogs(Rc::new(Comm), Rc::new(MonoMetrics)),
        );
        fly(&mut screen);
        hail(&mut screen);
        let pictures = |screen: &AppScreen| -> Vec<_> {
            drawn(screen)
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::StretchedPicture {
                        image, top_left, ..
                    } => Some((*image, *top_left)),
                    _ => None,
                })
                .collect()
        };
        assert!(
            pictures(&screen).contains(&(nova_view::ui::comm::COMM_PICTURE, Point::new(0.0, 0.0))),
            "{:?}",
            pictures(&screen)
        );
        tap(&mut screen, Key::Char('s'));
        assert!(
            pictures(&screen)
                .contains(&(nova_view::ui::comm::HAGGLE_PICTURE, Point::new(0.0, 0.0))),
            "{:?}",
            pictures(&screen)
        );
        assert!(screen.take_warnings().is_empty());
    }

    #[test]
    fn without_any_metrics_a_hail_is_hung_up() {
        let mut screen = hailable(AppScreen::new(boarding_data()));
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(flight(&screen).hailing(), None, "hung up");
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot show the comm dialog: no metrics to lay it out"]
        );
    }

    #[test]
    fn every_rule_reaches_every_flight() {
        let every = RuleKey::ALL
            .into_iter()
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        let reached = |screen: &AppScreen| {
            let rules = *flight(screen).session().expect("flying").rules();
            assert_eq!(rules, every);
            for key in RuleKey::ALL {
                assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
            }
        };
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        assert_eq!(
            flight(&screen).session().map(Session::rules),
            Ok(&Rulebook::default()),
            "the engine's by default"
        );
        let mut screen = AppScreen::new(data()).with_rulebook(&every);
        fly(&mut screen);
        reached(&screen);
        let store = MemoryPilots::new();
        let mut screen = menu(&store).with_rulebook(&every);
        create(&mut screen, "Ada");
        reached(&screen);
    }

    #[test]
    fn the_routers_persons_rules_reach_every_flight() {
        let rules = nova_sim::NovaPersons {
            slots: RuleSource::Bible,
            ..nova_sim::NovaPersons::default()
        };
        let mut screen = AppScreen::new(data()).with_person_rules(Rc::new(rules));
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(
            format!("{:?}", session.person_rules()),
            format!("{rules:?}")
        );
    }

    #[test]
    fn each_flight_names_its_ship_from_the_game_datas_string_lists() {
        let mut screen = AppScreen::new(trading_data());
        fly(&mut screen);
        let strings = flight(&screen).session().expect("flying").strings();
        assert_eq!(strings.string_list(4000), ["Food"]);
    }

    /// The rule-bearing parts of `screen`, each by name, as they print:
    /// its rule set and the strategies it hands each flight.
    fn rules_of(screen: &AppScreen) -> Vec<(&'static str, String)> {
        vec![
            ("rules", format!("{:?}", screen.rules)),
            ("behaviour", format!("{:?}", screen.behaviour)),
            ("disable_rule", format!("{:?}", screen.disable_rule)),
            ("defence_rule", format!("{:?}", screen.defence_rule)),
            ("law", format!("{:?}", screen.law)),
            ("boarding_rule", format!("{:?}", screen.boarding_rule)),
            ("hail_options", format!("{:?}", screen.hail_options)),
            ("hire_terms", format!("{:?}", screen.hire_terms)),
            ("control_bits", format!("{:?}", screen.control_bits)),
            ("person_rules", format!("{:?}", screen.person_rules)),
        ]
    }

    /// What [`rules_of`] must find once the router is given `rulebook`:
    /// the rule set, and each strategy built from it, by name, as they
    /// print.
    fn ports_of(rulebook: &Rulebook) -> Vec<(&'static str, String)> {
        vec![
            ("rules", format!("{rulebook:?}")),
            (
                "behaviour",
                format!("{:?}", NovaAi::from_rulebook(rulebook)),
            ),
            ("disable_rule", format!("{NovaDisable:?}")),
            ("defence_rule", format!("{Allegiance:?}")),
            ("law", format!("{:?}", NovaLaw::from_rulebook(rulebook))),
            (
                "boarding_rule",
                format!("{:?}", NovaBoarding::from_rulebook(rulebook)),
            ),
            ("hail_options", format!("{:?}", HailOptions::nova(rulebook))),
            (
                "hire_terms",
                format!("{:?}", NovaHire::from_rulebook(rulebook)),
            ),
            ("control_bits", format!("{NovaBits:?}")),
            (
                "person_rules",
                format!("{:?}", NovaPersons::from_rulebook(rulebook)),
            ),
        ]
    }

    #[test]
    fn the_rulebook_routes_each_rule_to_its_own_setting() {
        assert_eq!(
            rules_of(&AppScreen::new(data())),
            ports_of(&Rulebook::default()),
            "the engine's are the router's defaults"
        );
        let every = RuleKey::ALL
            .into_iter()
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        let books = RuleKey::ALL
            .into_iter()
            .map(|key| Rulebook::default().with_override(key, RuleSource::Bible))
            .chain([every]);
        for rulebook in books {
            assert_eq!(
                rules_of(&AppScreen::new(data()).with_rulebook(&rulebook)),
                ports_of(&rulebook),
                "{rulebook:?}"
            );
        }
        // The joining and grant rules reach the hail options' Use As
        // Escort, the boarding rule's count and the rule set's `Max`.
        let joined = Rulebook::default()
            .with_override(RuleKey::PersonJoin, RuleSource::Bible)
            .with_override(RuleKey::GrantCount, RuleSource::Bible)
            .with_override(RuleKey::GrantMax, RuleSource::Bible);
        let screen = AppScreen::new(data()).with_rulebook(&joined);
        let routed = rules_of(&screen);
        let setting = |name: &str| {
            routed
                .iter()
                .find(|(setting, _)| *setting == name)
                .map(|(_, value)| value.clone())
                .unwrap_or_default()
        };
        assert!(
            setting("hail_options").contains("JoinFleet { rule: Bible }"),
            "{routed:?}"
        );
        let boarding = setting("boarding_rule");
        assert!(boarding.contains("grant_count: Bible"), "{boarding}");
        assert!(!boarding.contains("grant_max"), "{boarding}");
        assert_eq!(
            screen.rules.source_for(RuleKey::GrantMax),
            RuleSource::Bible
        );
    }

    #[test]
    fn the_routers_hail_options_reach_every_flight() {
        let mut screen = with_dialogs(boarding_data())
            .with_chance(under_the_player())
            .with_behaviour(Rc::new(Idle))
            .with_hail_options(HailOptions::empty());
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(flight(&screen).hailing().expect("hailing").options, []);
    }

    // Pilot edits.

    use nova_view::devtools::{PilotDesk, PilotEdit};

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

    #[test]
    fn the_pilot_desk_is_there_only_in_flight() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        assert!(screen.pilot_desk().is_none());
        create(&mut screen, "Ada");
        let desk = screen.pilot_desk().expect("flying");
        assert_eq!(desk.sheet().expect("a pilot").name, "Ada");
    }

    #[test]
    fn a_credits_edit_is_saved_and_the_open_exchange_shows_it() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 7);
        let writes = store.writes();
        edit(&mut screen, PilotEdit::Credits(5000));
        assert_eq!(store.writes(), writes + 1);
        assert_eq!(saved(&store, "Ada").cash(), 5000);
        let open = spaceport(&screen).open_trade().expect("still trading");
        assert_eq!(open.market().cash, 5000);
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn an_edit_in_flight_is_saved_too() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        create(&mut screen, "Ada");
        edit(&mut screen, PilotEdit::Credits(42));
        assert_eq!(saved(&store, "Ada").cash(), 42);
        assert_eq!(screen.showing(), Showing::Flight);
    }

    #[test]
    fn a_move_rebuilds_the_spaceport_for_the_new_stellar() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        edit(
            &mut screen,
            PilotEdit::MoveTo {
                system: SystemId(129),
                stellar: nova_sim::StellarId(129),
            },
        );
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(
            flight(&screen)
                .scene()
                .map(nova_view::system::scene::SystemScene::id),
            Some(SystemId(129))
        );
        let saved = saved(&store, "Ada");
        assert_eq!(saved.system(), SystemId(129));
        assert_eq!(saved.stellar(), Some(nova_sim::StellarId(129)));
        assert!(
            !spaceport(&screen)
                .offered()
                .contains(&nova_sim::Service::TradeCenter),
            "Beta Prime has no exchange"
        );
    }

    #[test]
    fn a_move_to_the_stellar_landed_on_keeps_the_spaceport_open() {
        let store = MemoryPilots::new();
        let mut screen = landed_trader(&store);
        click_port_item(&mut screen, 7);
        edit(
            &mut screen,
            PilotEdit::MoveTo {
                system: SystemId(128),
                stellar: nova_sim::StellarId(128),
            },
        );
        assert!(spaceport(&screen).open_trade().is_some(), "not rebuilt");
    }

    #[test]
    fn an_unnamed_pilots_edit_is_not_saved() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        screen.input(&key(Key::Tab, true));
        fly(&mut screen);
        edit(&mut screen, PilotEdit::Credits(42));
        assert_eq!(pilot(&screen).cash(), 42);
        assert_eq!(store.writes(), 0);
        assert_eq!(store.keys(), Vec::<String>::new());
    }

    // Mission offers at every venue.

    use nova_sim::MissionId;
    use nova_sim::mission::screens::{BAR, OUTFITTER, SHIPYARD, SPACEPORT, TRADE_CENTER};
    use nova_view::spaceport::{OutfitterScreen, ShipyardScreen};

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
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
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

    #[test]
    fn landing_offers_a_spaceport_mission_over_the_spaceport() {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, venue_data(SPACEPORT));
        let offer = spaceport(&screen)
            .offer_dialog()
            .expect("offered on landing");
        assert_eq!(offer.prompt().mission, MissionId(130));
        assert_eq!(offer.prompt().text, "Job 130.");
        screen.input(&key(Key::Enter, true));
        assert!(spaceport(&screen).offer_dialog().is_none());
        assert_eq!(active(&screen), [MissionId(130)]);
        assert_eq!(pilot(&screen).cash(), 900, "its pay took 100");
    }

    #[test]
    fn a_pilot_desk_move_offers_the_new_stellars_spaceport_mission() {
        let fork = game_data_fork(true, true, true, 100, b"", b"", None)
            .resource(
                Mission::TYPE,
                131,
                Some(b"Venue job"),
                &venue_mission_at(SPACEPORT, 129),
            )
            .resource(Desc::TYPE, 131 + 3872, Some(b"Offer"), &desc_of("Job 131."));
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, loaded(&fork));
        assert!(spaceport(&screen).offer_dialog().is_none(), "none at 128");
        edit(
            &mut screen,
            PilotEdit::MoveTo {
                system: SystemId(129),
                stellar: nova_sim::StellarId(129),
            },
        );
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(129));
        assert_eq!(
            offered_mission(spaceport(&screen).offer_dialog()),
            Some(MissionId(131)),
            "asked before the next frame"
        );
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(131)]);
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

    /// The router landed at Alpha Prime, whose spaceport offers mission
    /// 130 of `Flags` `flags`, its offer `dësc` of `text` naming the movie
    /// "Offer.mov" played as `movie` says, with movies played through a
    /// new [`Reel`] and `prefs`.
    fn landed_filmed(flags: u16, text: &str, movie: u16, prefs: Prefs) -> (AppScreen, Rc<Reel>) {
        let (mut screen, reel) = filmed(flags, text, movie, prefs);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        (screen, reel)
    }

    /// [`landed_filmed`], before any pilot is made.
    fn filmed(flags: u16, text: &str, movie: u16, prefs: Prefs) -> (AppScreen, Rc<Reel>) {
        let mut mission = venue_mission(SPACEPORT);
        mission[0x50..0x52].copy_from_slice(&flags.to_be_bytes());
        let offer = filmed_desc_of(text, "Offer.mov", movie);
        let fork = game_data_fork(true, true, true, 100, b"", b"", None)
            .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
            .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &offer);
        let reel = Rc::new(Reel::default());
        let store = MemoryPilots::new();
        let screen = AppScreen::new(loaded(&fork))
            .with_prefs(prefs)
            .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
            .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        (screen, reel)
    }

    /// Opens the Preferences, clicks QuickTime Movies and closes them.
    fn toggle_movies(screen: &mut AppScreen) {
        open_prefs(screen);
        let was = prefs_dialog(screen).quicktime_movies().on();
        let point = prefs_dialog(screen).quicktime_movies().rect().center();
        click_at(screen, point);
        assert_eq!(prefs_dialog(screen).quicktime_movies().on(), !was);
        screen.input(&key(Key::Enter, true));
        assert!(screen.preferences().is_none(), "closed");
        assert_eq!(screen.prefs().quicktime_movies, !was);
    }

    #[test]
    fn quicktime_movies_turned_off_in_the_dialog_stop_every_movie() {
        for movie in [0, 0x0001] {
            let (mut screen, reel) = filmed(0, "Job.", movie, Prefs::default());
            toggle_movies(&mut screen);
            create(&mut screen, "Ada");
            land_twice(&mut screen);
            assert!(spaceport(&screen).offer_dialog().is_some(), "{movie}");
            screen.input(&key(Key::Enter, true));
            assert_eq!(active(&screen), [MissionId(130)], "{movie}");
            assert!(reeled(&reel).is_empty(), "{movie}: {:?}", reeled(&reel));
        }
    }

    #[test]
    fn quicktime_movies_turned_on_in_the_dialog_play_again() {
        let off = Prefs {
            quicktime_movies: false,
            ..Prefs::default()
        };
        let (mut screen, reel) = filmed(0, "Job.", 0, off);
        toggle_movies(&mut screen);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.tick(Duration::from_millis(16));
        assert_eq!(reeled(&reel), ["start Offer.mov"], "before the dialog");
        let (mut screen, reel) = filmed(0, "Job.", 0x0001, off);
        toggle_movies(&mut screen);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(reeled(&reel), ["start Offer.mov"], "after the answer");
    }

    fn reeled(reel: &Reel) -> Vec<String> {
        reel.log.borrow().clone()
    }

    #[test]
    fn the_spaceports_offer_plays_its_after_movie_once_answered() {
        let (mut screen, reel) = landed_filmed(0, "Job.", 0x0001, Prefs::default());
        assert!(spaceport(&screen).offer_dialog().is_some());
        assert!(reeled(&reel).is_empty(), "not before the dialog");
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)]);
        assert_eq!(reeled(&reel), ["start Offer.mov"], "after the answer");
        screen.tick(Duration::from_millis(16));
        screen.tick(Duration::from_millis(16));
        assert_eq!(reeled(&reel), ["start Offer.mov", "stop"]);
    }

    #[test]
    fn the_spaceports_offer_plays_its_before_movie_and_a_refusal_after_it_plays_none() {
        let (mut screen, reel) = landed_filmed(0, "Job.", 0, Prefs::default());
        assert!(reeled(&reel).is_empty(), "staged until the spaceport ticks");
        screen.tick(Duration::from_millis(16));
        assert_eq!(reeled(&reel), ["start Offer.mov"]);
        screen.tick(Duration::from_millis(16));
        screen.input(&key(Key::Char('n'), true));
        assert_eq!(active(&screen), []);
        assert_eq!(reeled(&reel), ["start Offer.mov", "stop"], "none after");
    }

    /// The router landed at Alpha Prime, whose spaceport offers mission
    /// 130 of the `mïsn` bytes `mission`, its offer `dësc` "Job." naming
    /// "Offer.mov" played after, with the `dësc`s `descs` too, movies
    /// played through a new [`Reel`].
    fn landed_offering(mission: &[u8], descs: &[(i16, Vec<u8>)]) -> (AppScreen, Rc<Reel>) {
        let mut fork = game_data_fork(true, true, true, 100, b"", b"", None)
            .resource(Mission::TYPE, 130, Some(b"Venue job"), mission)
            .resource(
                Desc::TYPE,
                130 + 3872,
                Some(b"Offer"),
                &filmed_desc_of("Job.", "Offer.mov", 0x0001),
            );
        for (id, desc) in descs {
            fork = fork.resource(Desc::TYPE, *id, None, desc);
        }
        let reel = Rc::new(Reel::default());
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(loaded(&fork))
            .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
            .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        (screen, reel)
    }

    fn ticked(screen: &mut AppScreen, ticks: usize) {
        for _ in 0..ticks {
            screen.tick(Duration::from_millis(16));
        }
    }

    #[test]
    fn a_no_room_text_replays_the_spaceport_offers_after_movie() {
        let mut mission = venue_mission(SPACEPORT);
        mission[0x10..0x12].copy_from_slice(&0_i16.to_be_bytes());
        mission[0x12..0x14].copy_from_slice(&10_000_i16.to_be_bytes());
        let (mut screen, reel) = landed_offering(&mission, &[]);
        assert!(spaceport(&screen).offer_dialog().is_some());
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [], "no room for the cargo");
        assert!(
            spaceport(&screen).mission_text().is_some(),
            "the no-room text"
        );
        assert!(reeled(&reel).is_empty(), "after it, not before");
        screen.input(&key(Key::Enter, true));
        ticked(&mut screen, 6);
        assert_eq!(
            reeled(&reel),
            ["start Offer.mov", "stop", "start Offer.mov", "stop"],
            "the STR# text's after-check reads the offer's (@0x5b40b), and the offer's \
             after-check plays it again (@0xa3580)"
        );
    }

    #[test]
    fn a_refuse_texts_after_movie_plays_twice_and_the_offers_not_at_all() {
        let mut mission = venue_mission(SPACEPORT);
        mission[0x58..0x5A].copy_from_slice(&300_i16.to_be_bytes());
        let (mut screen, reel) = landed_offering(
            &mission,
            &[(300, filmed_desc_of("No.", "Refuse.mov", 0x0001))],
        );
        screen.input(&key(Key::Char('n'), true));
        assert!(
            spaceport(&screen).mission_text().is_some(),
            "the RefuseText"
        );
        assert!(
            spaceport(&screen).offer_dialog().is_none(),
            "the offer hidden for it (@0xa2e8e)"
        );
        screen.input(&key(Key::Enter, true));
        ticked(&mut screen, 6);
        assert_eq!(
            reeled(&reel),
            ["start Refuse.mov", "stop", "start Refuse.mov", "stop"],
            "nothing reloads the offer after the refuse text (@0xa2eb5-0xa2f4d)"
        );
    }

    fn texts_drawn(screen: &AppScreen) -> Vec<String> {
        drawn(screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_refused_spaceport_offer_without_a_refuse_text_stays_drawn_under_its_after_movie() {
        let (mut screen, reel) = landed_offering(&venue_mission(SPACEPORT), &[]);
        assert!(texts_drawn(&screen).contains(&"Job.".to_owned()));
        screen.input(&key(Key::Char('n'), true));
        assert_eq!(reeled(&reel), ["start Offer.mov"]);
        assert!(
            texts_drawn(&screen).contains(&"Job.".to_owned()),
            "not hidden (@0xa2e70-0xa2e78): under its after-movie until @0xa361e"
        );
        ticked(&mut screen, 3);
        assert_eq!(reeled(&reel), ["start Offer.mov", "stop"]);
        assert!(
            !texts_drawn(&screen).contains(&"Job.".to_owned()),
            "disposed"
        );
    }

    #[test]
    fn a_refuse_text_hides_the_spaceport_offer() {
        let mut mission = venue_mission(SPACEPORT);
        mission[0x58..0x5A].copy_from_slice(&300_i16.to_be_bytes());
        let (mut screen, _) = landed_offering(&mission, &[(300, desc_of("No."))]);
        screen.input(&key(Key::Char('n'), true));
        let shown = texts_drawn(&screen);
        assert!(
            !shown.contains(&"Job.".to_owned()),
            "hidden for its RefuseText (@0xa2e8e): {shown:?}"
        );
    }

    #[test]
    fn the_mission_computers_refusal_plays_the_last_descs_after_movie() {
        let fork = game_data_fork(true, true, true, 100, b"", b"", None).resource(
            Desc::TYPE,
            128,
            None,
            &filmed_desc_of("A port.", "Port.mov", 0x0001),
        );
        let reel = Rc::new(Reel::default());
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(loaded(&fork))
            .with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>)
            .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 11);
        assert!(
            spaceport(&screen).mission_text().is_some(),
            "nothing available here"
        );
        assert!(reeled(&reel).is_empty());
        screen.input(&key(Key::Enter, true));
        assert_eq!(
            reeled(&reel),
            ["start Port.mov"],
            "the stellar's dësc was the last loaded (@0x5f96b), and the refusal's \
             after-check reads it (@0xa3956, @0x5b40b)"
        );
    }

    #[test]
    fn a_spaceport_offer_accepted_at_once_plays_its_movie() {
        let (screen, reel) = landed_filmed(0x0004, "", 0x0004, Prefs::default());
        assert_eq!(active(&screen), [MissionId(130)]);
        assert_eq!(reeled(&reel), ["start Offer.mov"]);
        assert!(!screen.hides_pointer(), "during the fade out");
    }

    #[test]
    fn a_cinematic_movie_over_the_spaceport_hides_the_pointer() {
        let (mut screen, _) = landed_filmed(0x0004, "", 0x0004, Prefs::default());
        screen.tick(Duration::from_secs(1));
        assert!(screen.hides_pointer());
    }

    #[test]
    fn command_comma_over_a_spaceport_movie_opens_no_preferences_and_the_movie_plays_on() {
        let (mut screen, reel) = landed_filmed(0, "Job.", 0x0004, Prefs::default());
        screen.tick(Duration::from_secs(1));
        assert!(screen.hides_pointer(), "the cinematic movie plays");
        command_comma(&mut screen);
        assert!(screen.preferences().is_none());
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert!(screen.hides_pointer(), "still hidden");
        for _ in 0..8 {
            screen.tick(Duration::from_secs(1));
        }
        assert_eq!(reeled(&reel), ["start Offer.mov", "stop"], "played on");
        assert!(
            spaceport(&screen)
                .offer_dialog()
                .is_some_and(|offer| !offer.playing()),
            "the offer shows"
        );
    }

    #[test]
    fn an_overlay_over_a_spaceport_movie_shows_the_pointer() {
        let (mut screen, _) = landed_filmed(0, "Job.", 0x0004, Prefs::default());
        screen.tick(Duration::from_secs(1));
        assert!(screen.hides_pointer());
        screen.open_preferences();
        assert_eq!(screen.showing(), Showing::Preferences);
        assert!(!screen.hides_pointer());
    }

    #[test]
    fn with_movies_off_in_the_preferences_none_plays() {
        let off = Prefs {
            quicktime_movies: false,
            ..Prefs::default()
        };
        let (mut screen, reel) = landed_filmed(0, "Job.", 0x0001, off);
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)]);
        assert!(reeled(&reel).is_empty());
    }

    /// Opens shop `item` at a stellar offering mission 130 at `loc`, and
    /// gives the offer open over it.
    fn shop_offer(loc: i16, item: usize) -> Option<MissionId> {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, venue_data(loc));
        assert!(
            spaceport(&screen).offer_dialog().is_none(),
            "not on landing"
        );
        click_port_item(&mut screen, item);
        let spaceport = spaceport(&screen);
        let opened = match loc {
            TRADE_CENTER => spaceport.open_trade().and_then(TradeScreen::offer_dialog),
            OUTFITTER => spaceport
                .open_outfitter()
                .and_then(OutfitterScreen::offer_dialog),
            _ => spaceport
                .open_shipyard()
                .and_then(ShipyardScreen::offer_dialog),
        };
        offered_mission(opened)
    }

    #[test]
    fn entering_the_trade_center_offers_its_mission_at_once() {
        assert_eq!(shop_offer(TRADE_CENTER, 7), Some(MissionId(130)));
    }

    #[test]
    fn entering_the_outfitter_offers_its_mission_at_once() {
        assert_eq!(shop_offer(OUTFITTER, 8), Some(MissionId(130)));
    }

    /// [`shipbuying_data`] with an outfitter selling nothing, as no `oütf`
    /// exists, and mission 130 at the outfitter ([`venue_mission`]),
    /// unrefusable and with no offer text, so an offer of it is its
    /// accept at once.
    fn empty_outfitter_offering_data() -> Rc<GameData> {
        let mut mission = venue_mission(OUTFITTER);
        mission[0x50..0x52].copy_from_slice(&0x0004_u16.to_be_bytes());
        let mut built = game_data_fork(true, false, true, 100, b"", b"", None)
            .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
            .build();
        let stellar = built.layout.data_entry(Stellar::TYPE, 128) + 4;
        built.bytes[stellar + 0x09] |= 0x04;
        loaded_bytes(built.bytes)
    }

    /// An outfitter listing nothing never opens, so it offers nothing
    /// (`_DoOutfitDialog` @0x5bbf7 → @0x5dbd6 skips its offer): its
    /// mission, accepted at once on any offer, stays unaccepted.
    #[test]
    fn an_empty_outfitter_offers_nothing() {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, empty_outfitter_offering_data());
        click_port_item(&mut screen, 8);
        assert!(
            spaceport(&screen).open_outfitter().is_none(),
            "never opened"
        );
        assert_eq!(
            spaceport(&screen)
                .mission_text()
                .map(|text| text.lines().join(" ")),
            Some(nova_view::spaceport::outfitter::NO_ITEMS.to_owned())
        );
        screen.input(&key(Key::Enter, true));
        screen.tick(Duration::from_secs(5));
        assert!(spaceport(&screen).offer_dialog().is_none());
        assert_eq!(active(&screen), [], "nothing offered, nothing accepted");
        assert_eq!(pilot(&screen).cash(), 1000);
    }

    #[test]
    fn entering_the_shipyard_offers_its_mission_at_once() {
        assert_eq!(shop_offer(SHIPYARD, 9), Some(MissionId(130)));
    }

    #[test]
    fn an_accept_at_the_outfitter_hands_the_outfitter_back() {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, venue_data(OUTFITTER));
        click_port_item(&mut screen, 8);
        let open = spaceport(&screen).open_outfitter().expect("outfitting");
        assert_eq!(offered_mission(open.offer_dialog()), Some(MissionId(130)));
        assert_eq!(open.outfitter().cash, 1000);
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)]);
        let open = spaceport(&screen)
            .open_outfitter()
            .expect("still outfitting");
        assert!(open.offer_dialog().is_none());
        assert_eq!(open.outfitter().cash, 900, "the cash shown changes");
    }

    /// The trade center's offer at a stellar offering missions 130-132
    /// there, with the offer timer on `clock`: 130 refused, 131 left open
    /// 5 s and refused, and a moment after: the offer open then.
    fn offer_a_moment_after_a_long_offer(clock: RuleSource) -> Option<MissionId> {
        let store = MemoryPilots::new();
        let rulebook = Rulebook::default().with_override(RuleKey::OfferClock, clock);
        let mut screen = AppScreen::new(venue_data_of(TRADE_CENTER, &[130, 131, 132]))
            .with_rulebook(&rulebook)
            .with_dialogs(Rc::new(VenueDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 7);
        let offered = |screen: &AppScreen| {
            offered_mission(
                spaceport(screen)
                    .open_trade()
                    .and_then(TradeScreen::offer_dialog),
            )
        };
        assert_eq!(offered(&screen), Some(MissionId(130)));
        screen.input(&key(Key::Char('n'), true));
        screen.tick(Duration::from_secs(2));
        screen.tick(Duration::from_secs(2));
        assert_eq!(offered(&screen), Some(MissionId(131)), "{clock:?}");
        screen.tick(Duration::from_secs(5));
        screen.input(&key(Key::Char('n'), true));
        screen.tick(Duration::from_millis(1));
        screen.tick(Duration::from_millis(1));
        offered(&screen)
    }

    #[test]
    fn the_routers_offer_clock_reaches_the_venues() {
        assert_eq!(
            offer_a_moment_after_a_long_offer(RuleSource::Engine),
            Some(MissionId(132)),
            "the gap ran from the ask"
        );
        assert_eq!(
            offer_a_moment_after_a_long_offer(RuleSource::Bible),
            None,
            "the gap runs from the answer"
        );
    }

    #[test]
    fn the_bar_offers_again_after_its_hire_screen_restarts_its_wait() {
        let store = MemoryPilots::new();
        let fork = [130, 131].into_iter().fold(
            game_data_fork(true, true, false, 100, b"", b"", None),
            |fork, id: i16| {
                fork.resource(Mission::TYPE, id, Some(b"Bar job"), &venue_mission(BAR))
                    .resource(
                        Desc::TYPE,
                        id + 3872,
                        Some(b"Offer"),
                        &desc_of(&format!("Job {id}.")),
                    )
            },
        );
        let mut built = fork.build();
        // The first ship is for hire, as in `past_the_holds_data`.
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x38A, 100);
        let mut screen = AppScreen::new(loaded_bytes(built.bytes))
            .with_dialogs(Rc::new(PastTheHoldsDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 10);
        let offered = |screen: &AppScreen| {
            offered_mission(
                spaceport(screen)
                    .open_bar()
                    .and_then(BarScreen::offer_dialog),
            )
        };
        screen.tick(Duration::from_millis(300));
        screen.tick(Duration::from_millis(300));
        assert_eq!(offered(&screen), Some(MissionId(130)));
        screen.input(&key(Key::Char('n'), true));
        screen.input(&key(Key::Char('h'), true));
        assert!(
            spaceport(&screen)
                .open_bar()
                .and_then(BarScreen::hire_screen)
                .is_some(),
            "for hire"
        );
        screen.tick(Duration::from_millis(1));
        screen.input(&key(Key::Escape, true));
        screen.tick(Duration::from_secs(2));
        screen.tick(Duration::from_secs(2));
        assert_eq!(offered(&screen), Some(MissionId(131)), "rearmed");
    }

    #[test]
    fn the_spaceport_does_not_offer_again_on_returning_from_a_service() {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, venue_data_of(SPACEPORT, &[130, 131]));
        assert_eq!(
            offered_mission(spaceport(&screen).offer_dialog()),
            Some(MissionId(130)),
            "one offer as it lands"
        );
        screen.input(&key(Key::Char('n'), true));
        assert!(spaceport(&screen).offer_dialog().is_none(), "refused");
        screen.tick(Duration::from_secs(5));
        assert!(spaceport(&screen).offer_dialog().is_none(), "131 waits");
        click_port_item(&mut screen, 7);
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_trade().is_none(), "back");
        screen.tick(Duration::from_secs(5));
        assert!(
            spaceport(&screen).offer_dialog().is_none(),
            "nothing on returning"
        );
    }

    // The status bar in port.

    use nova_data::graphics::PICT;
    use nova_data::graphics::fixture::{DirectBits, PictBuilder};
    use nova_data::records::interface::Interface;

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

    /// `ïntf` 128, as `tests/flight.rs` has it: the radar at (8, 8) to
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

    /// [`outfitting_data`] with the status bar ([`barred`]), ship 128
    /// carrying 300 fuel, and the outfitter also selling `oütf` 132, IFF
    /// (`ModType` 14), and `oütf` 133, a fuel tank (`ModType` 12) of 100,
    /// each 100 credits and weighing nothing.
    fn barred_outfitting_data() -> Rc<GameData> {
        barred_data(false)
    }

    /// [`barred_outfitting_data`], where the planet is also a shipyard
    /// ([`shipbuying_data`]) whose ship 129 carries IFF (`oütf` 132)
    /// beside its booster.
    fn barred_shipbuying_data() -> Rc<GameData> {
        barred_data(true)
    }

    /// [`barred_outfitting_data`], or [`barred_shipbuying_data`] when
    /// `shipbuying`.
    fn barred_data(shipbuying: bool) -> Rc<GameData> {
        let sold = |mod_type: i16, mod_val: i16| {
            let mut outfit = booster_record(b"");
            put_i16s(&mut outfit, 0x02, &[0]);
            put_i16s(&mut outfit, 0x06, &[mod_type, mod_val]);
            outfit[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
            outfit
        };
        let fork = barred(game_data_fork(true, true, shipbuying, 100, b"", b"", None))
            .resource(Outfit::TYPE, 132, Some(b"IFF"), &sold(14, 0))
            .resource(Outfit::TYPE, 133, Some(b"Tank"), &sold(12, 100));
        let mut built = fork.build();
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x0A, 300);
        if shipbuying {
            let second = built.layout.data_entry(Ship::TYPE, 129) + 4;
            built.put_u16(second + 0x50, 132);
            built.put_u16(second + 0x58, 1);
        }
        loaded_bytes(built.bytes)
    }

    /// The status bar's background in port: `PICT` 700 against the right
    /// edge.
    fn bar_background() -> DrawCommand {
        DrawCommand::Picture {
            image: nova_view::ImageKey::picture(700),
            top_left: Point::new(830.0, 0.0),
        }
    }

    /// What flight's status bar alone draws ([`FlightView::draw_status_bar`]).
    fn bar_drawn(screen: &AppScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        flight(screen).draw_status_bar(&mut list);
        list.iter().cloned().collect()
    }

    /// The 1-pixel dots inside the radar, (838, 8) to (1014, 184), as
    /// their pixels.
    fn radar_dots(list: &DrawList) -> Vec<(f32, f32)> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot {
                    center, size: 1.0, ..
                } if (838.0..1014.0).contains(&center.x) && (8.0..184.0).contains(&center.y) => {
                    Some((center.x - 0.5, center.y - 0.5))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn in_port_the_status_bar_shows_beside_the_spaceport_with_a_blank_radar() {
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(barred_outfitting_data())
            .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        assert!(
            radar_dots(&drawn(&screen)).len() > 1,
            "in flight the planet is on radar too"
        );
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        let list = drawn(&screen);
        let bar = bar_drawn(&screen);
        assert_eq!(bar.first(), Some(&bar_background()));
        let mut port = drawn(spaceport(&screen))
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            port.first(),
            Some(&DrawCommand::Picture {
                image: nova_view::spaceport::layout::BACKGROUND,
                top_left: spaceport(&screen).dialog().expect("laid out").bounds().min,
            })
        );
        let mut under = bar.clone();
        under.append(&mut port);
        assert_eq!(list.iter().cloned().collect::<Vec<_>>(), under);
        assert_eq!(radar_dots(&list), [(926.0, 96.0)], "the player alone");
    }

    #[test]
    fn the_outfitter_draws_over_the_status_bar() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
        click_port_item(&mut screen, 8);
        assert!(spaceport(&screen).open_outfitter().is_some());
        let mut under = bar_drawn(&screen);
        assert_eq!(under.first(), Some(&bar_background()));
        under.extend(drawn(spaceport(&screen)).iter().cloned());
        assert_eq!(drawn(&screen).iter().cloned().collect::<Vec<_>>(), under);
    }

    #[test]
    fn the_status_bar_is_drawn_in_port_and_flight_only() {
        let mut screen = AppScreen::new(barred_outfitting_data());
        let background = bar_background();
        fly(&mut screen);
        assert!(drawn(&screen).iter().any(|c| *c == background), "flight");
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(!drawn(&screen).iter().any(|c| *c == background), "ships");
        screen.input(&key(Key::Tab, true));
        assert_eq!(screen.showing(), Showing::GalaxyMap);
        assert!(!drawn(&screen).iter().any(|c| *c == background), "galaxy");
    }

    #[test]
    fn a_port_without_a_bar_draws_no_reason() {
        let store = MemoryPilots::new();
        let screen = landed_outfitter(&store);
        assert!(flight(&screen).status_bar().is_err(), "no ïntf");
        let texts: Vec<String> = drawn(&screen)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !texts.iter().any(|text| text.starts_with("Status bar")),
            "{texts:?}"
        );
        assert_eq!(drawn(&screen), drawn(spaceport(&screen)));
    }

    const IFF: OutfitId = OutfitId(132);

    /// Clicks at `at` with the left button.
    fn click_on(screen: &mut AppScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Selects `outfit`'s cell in the outfitter open, then clicks its
    /// item `item`.
    fn click_for_outfit(screen: &mut AppScreen, outfit: OutfitId, item: usize) {
        let rows = spaceport(screen)
            .open_outfitter()
            .expect("outfitting")
            .outfitter()
            .rows
            .len();
        for _ in 0..rows {
            let open = spaceport(screen).open_outfitter().expect("outfitting");
            if open.selected_outfit() == Some(outfit) {
                break;
            }
            screen.input(&key(Key::Right, true));
        }
        let open = spaceport(screen).open_outfitter().expect("outfitting");
        assert_eq!(open.selected_outfit(), Some(outfit));
        let at = open
            .dialog()
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("the item")
            .center();
        click_on(screen, at);
    }

    /// Whether `list` paints the radar, (838, 8) to (1014, 184), black.
    fn radar_black(list: &DrawList) -> bool {
        list.iter().any(|command| {
            matches!(*command, DrawCommand::Line { from, to, color, .. }
                if color == Color::rgba(0, 0, 0, 255)
                    && from.x >= 838.0 && to.x <= 1014.0
                    && (8.0..184.0).contains(&from.y))
        })
    }

    /// Whether `list` draws the player cyan at the radar's centre.
    fn player_cyan(list: &DrawList) -> bool {
        list.iter().any(|command| {
            *command
                == DrawCommand::Dot {
                    center: Point::new(926.5, 96.5),
                    size: 1.0,
                    color: Color::rgba(0, 255, 255, 255),
                }
        })
    }

    #[test]
    fn an_iff_bought_in_port_turns_the_bars_radar_black_at_once_and_one_sold_clears_it() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
        click_port_item(&mut screen, 8);
        assert!(!radar_black(&drawn(&screen)), "without IFF");
        click_for_outfit(&mut screen, IFF, OUTFIT_BUY_ITEM);
        assert_eq!(pilot(&screen).owned(IFF), 1);
        let list = drawn(&screen);
        assert!(radar_black(&list), "bought");
        assert!(player_cyan(&list), "bought");
        click_for_outfit(&mut screen, IFF, OUTFIT_SELL_ITEM);
        assert_eq!(pilot(&screen).owned(IFF), 0);
        let list = drawn(&screen);
        assert!(!radar_black(&list), "sold");
        assert!(!player_cyan(&list), "sold");
        assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
    }

    #[test]
    fn a_ship_bought_carrying_iff_turns_the_bars_radar_black_at_once() {
        let store = MemoryPilots::new();
        let mut screen = landed_shipyard_of(barred_shipbuying_data(), &store);
        click_port_item(&mut screen, 9);
        assert!(!radar_black(&drawn(&screen)), "without IFF");
        press_b(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen).ship(), ShipId(129));
        assert_eq!(pilot(&screen).owned(IFF), 1, "its default IFF");
        assert!(spaceport(&screen).open_shipyard().is_some(), "still open");
        let list = drawn(&screen);
        assert!(radar_black(&list));
        assert!(player_cyan(&list));
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

    /// Flies `screen` up, the thrust held, until the radar, refreshed
    /// every 8 ticks, has just refreshed, then 7 ticks more: it holds the
    /// ship as it was 7 ticks back.
    fn stale_radar(screen: &mut AppScreen) {
        screen.input(&key(Key::Up, true));
        for _ in 0..200 {
            screen.tick(TICK);
        }
        let mut before = radar_dots(&drawn(screen));
        for _ in 0..8 {
            screen.tick(TICK);
            let after = radar_dots(&drawn(screen));
            if after != before {
                break;
            }
            before = after;
        }
        for _ in 0..7 {
            screen.tick(TICK);
        }
    }

    #[test]
    fn player_infos_and_mission_infos_close_over_flight_redraw_the_whole_bar() {
        type Close = fn(&mut AppScreen);
        let rows: [(&str, Close); 2] = [
            ("player info (@0x5a725)", |screen| {
                screen.input(&key(PLAYER_INFO_KEY, true));
                screen.input(&key(PLAYER_INFO_KEY, false));
                assert!(screen.flight_player_info().is_some(), "open");
                screen.input(&key(Key::Escape, true));
                assert!(screen.flight_player_info().is_none(), "closed");
            }),
            ("mission info (@0x9e769)", |screen| {
                let mut missions = MissionDialogs::new(
                    DescTemplates {
                        plain: Err("no DLOG 3003".to_owned()),
                        picture: Err("no DLOG 3004".to_owned()),
                    },
                    Ok(mission_info_template()),
                    Rc::new(MonoMetrics),
                    screen.data.button_style(),
                );
                missions.open_info(nova_sim::MissionInfo {
                    rows: vec![nova_sim::InfoRow {
                        slot: 0,
                        name: "Run".to_owned(),
                        failed: false,
                        text: "Go.".to_owned(),
                        abortable: true,
                    }],
                    today: String::new(),
                });
                assert!(missions.info().is_some(), "open");
                screen.missions = Some(missions);
                screen.input(&key(Key::Escape, true));
                assert!(screen.missions.is_none(), "closed");
            }),
        ];
        for (what, close) in rows {
            let mut screen = AppScreen::new(barred_outfitting_data())
                .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics));
            fly(&mut screen);
            stale_radar(&mut screen);
            let held = radar_dots(&drawn(&screen));
            close(&mut screen);
            let shown = radar_dots(&drawn(&screen));
            screen.flight.as_mut().expect("flying").refresh_status();
            let now = radar_dots(&drawn(&screen));
            assert_ne!(held, now, "{what}: the radar held what it drew before");
            assert_eq!(shown, now, "{what}: redrawn as it closed");
        }
    }

    #[test]
    fn player_infos_close_over_flight_without_its_dialog_redraws_nothing() {
        /// [`Dialogs`] without "Player Info" (1017): the original, failing
        /// to get it, neither shows the dialog nor redraws (@0x5a0a5).
        struct NoPlayerInfo;

        impl DialogResources for NoPlayerInfo {
            fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
                if id == PLAYER_INFO_DIALOG {
                    return Err(format!("no DLOG {id}"));
                }
                Dialogs.dialog_template(id)
            }
        }

        let mut screen = AppScreen::new(barred_outfitting_data())
            .with_dialogs(Rc::new(NoPlayerInfo), Rc::new(MonoMetrics));
        fly(&mut screen);
        stale_radar(&mut screen);
        let held = radar_dots(&drawn(&screen));
        screen.input(&key(PLAYER_INFO_KEY, true));
        screen.input(&key(PLAYER_INFO_KEY, false));
        let info = screen.flight_player_info().expect("open");
        assert!(info.problem().is_some(), "not laid out");
        screen.input(&key(Key::Escape, true));
        assert!(screen.flight_player_info().is_none(), "closed");
        assert_eq!(radar_dots(&drawn(&screen)), held, "not redrawn");
        screen.flight.as_mut().expect("flying").refresh_status();
        assert_ne!(radar_dots(&drawn(&screen)), held, "stale as it closed");
    }

    const TANK: OutfitId = OutfitId(133);

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

    /// [`barred_outfitting_data`]'s ship and tank, where Alpha Prime's
    /// spaceport offers mission 130 ([`venue_mission`]), whose `OnAccept`
    /// is `G133`, the tank, and whose `BriefText` is `dësc` 5000, "Go.".
    fn barred_briefing_data() -> Rc<GameData> {
        let mut tank = booster_record(b"");
        put_i16s(&mut tank, 0x02, &[0]);
        put_i16s(&mut tank, 0x06, &[12, 100]);
        tank[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
        let mut mission = venue_mission(SPACEPORT);
        put_i16s(&mut mission, 0x34, &[5000]);
        mission[0x15B..0x15F].copy_from_slice(b"G133");
        let fork = barred(game_data_fork(true, true, true, 100, b"", b"", None))
            .resource(Outfit::TYPE, 133, Some(b"Tank"), &tank)
            .resource(Mission::TYPE, 130, Some(b"Venue job"), &mission)
            .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job."))
            .resource(Desc::TYPE, 5000, Some(b"Brief"), &desc_of("Go."));
        let mut built = fork.build();
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x0A, 300);
        loaded_bytes(built.bytes)
    }

    #[test]
    fn a_spaceport_brief_text_shows_over_the_pre_accept_bar_and_its_done_redraws_it() {
        let store = MemoryPilots::new();
        let mut screen = landed_at_venues(&store, barred_briefing_data());
        assert!(spaceport(&screen).offer_dialog().is_some(), "offered");
        let full = fuel_width(&drawn(&screen)).expect("some fuel");
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        assert_eq!(fuel_width(&drawn(&screen)), Some(full), "held");
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)]);
        assert!(
            spaceport(&screen).mission_text().is_some(),
            "the brief shows"
        );
        let half = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(
            half < full,
            "the dialog's redraw before the accept: {half} of {full}"
        );
        screen.input(&key(Key::Enter, true));
        assert!(spaceport(&screen).mission_text().is_none(), "closed");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(
            width > half,
            "its close redraws OnAccept's tank, fuel and all: {width}"
        );
        screen.flight.as_mut().expect("flying").refresh_status();
        assert_eq!(fuel_width(&drawn(&screen)), Some(width), "as it is now");
    }

    #[test]
    fn mission_info_closing_over_the_outfitter_redraws_the_bar_held_under_it() {
        /// [`VenueDialogs`] with the mission info screen.
        struct InfoDialogs;

        impl DialogResources for InfoDialogs {
            fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
                if id == INFO_DIALOG {
                    return Ok(mission_info_template());
                }
                VenueDialogs.dialog_template(id)
            }
        }

        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(barred_briefing_data())
            .with_dialogs(Rc::new(InfoDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.input(&key(Key::Enter, true));
        screen.input(&key(Key::Enter, true));
        assert_eq!(active(&screen), [MissionId(130)], "accepted");
        assert!(spaceport(&screen).mission_text().is_none(), "brief closed");
        let full = fuel_width(&drawn(&screen)).expect("some fuel");
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        click_port_item(&mut screen, 8);
        assert!(spaceport(&screen).open_outfitter().is_some(), "open");
        assert_eq!(fuel_width(&drawn(&screen)), Some(full), "held");
        screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
        screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
        assert!(spaceport(&screen).mission_info().is_some(), "info open");
        assert_eq!(fuel_width(&drawn(&screen)), Some(full), "not as it opens");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).mission_info().is_none(), "info closed");
        assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width < full, "redrawn as it closes (@0x9e769): {width}");
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

    /// [`barred`] [`shipbuying_data`], its first ship for hire, where
    /// Alpha Prime's spaceport offers mission 130, whose `OnAccept`,
    /// `S131`, starts mission 131 too, each one's `OnAbort` `M129`, a
    /// landed move that selects Beta Prime; and its bar offers mission
    /// 129, listed before 130 so the accept's shift (`RuleKey::OfferShift`)
    /// leaves it.
    fn aborting_to_beta_data() -> Rc<GameData> {
        let mut job = venue_mission(SPACEPORT);
        job[0x15B..0x15F].copy_from_slice(b"S131");
        job[0x557..0x55B].copy_from_slice(b"M129");
        let mut next = venue_mission(SPACEPORT);
        next[0x5C..0x5E].copy_from_slice(b"b9");
        next[0x557..0x55B].copy_from_slice(b"M129");
        let fork = barred(game_data_fork(true, true, true, 100, b"", b"", None))
            .resource(Mission::TYPE, 130, Some(b"Venue job"), &job)
            .resource(Mission::TYPE, 131, Some(b"Next job"), &next)
            .resource(Mission::TYPE, 129, Some(b"Bar job"), &venue_mission(BAR))
            .resource(Desc::TYPE, 130 + 3872, Some(b"Offer"), &desc_of("Job."))
            .resource(Desc::TYPE, 129 + 3872, Some(b"Offer"), &desc_of("Bar job."));
        let mut built = fork.build();
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x38A, 100);
        loaded_bytes(built.bytes)
    }

    /// Whether `screen` draws "Beta Prime", which only the nav area can.
    fn names_beta(screen: &AppScreen) -> bool {
        drawn(screen).iter().any(
            |command| matches!(command, DrawCommand::Text { text, .. } if text == "Beta Prime"),
        )
    }

    /// A screen mission info opens over: what it is, how to open it on
    /// the spaceport, and whether it is open.
    type InfoOpener = (&'static str, fn(&mut AppScreen), fn(&SpaceportView) -> bool);

    /// Each screen mission info opens over that redraws the bar as it
    /// closes (@0x9e769), as [`aborting_to_beta_data`] lays them out.
    const INFO_OPENERS: [InfoOpener; 7] = [
        (
            "the exchange",
            |screen| click_port_item(screen, 7),
            |port| port.open_trade().is_some(),
        ),
        (
            "the outfitter",
            |screen| click_port_item(screen, 8),
            |port| port.open_outfitter().is_some(),
        ),
        (
            "the shipyard",
            |screen| click_port_item(screen, 9),
            |port| port.open_shipyard().is_some(),
        ),
        (
            "the shipyard's description",
            |screen| {
                click_port_item(screen, 9);
                screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
            },
            |port| {
                port.open_shipyard()
                    .and_then(ShipyardScreen::info_panel)
                    .is_some()
            },
        ),
        (
            "the hire screen",
            |screen| {
                click_port_item(screen, 10);
                screen.input(&key(Key::Char('h'), true));
            },
            |port| port.open_bar().and_then(BarScreen::hire_screen).is_some(),
        ),
        (
            "the hire screen's description",
            |screen| {
                click_port_item(screen, 10);
                screen.input(&key(Key::Char('h'), true));
                screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
            },
            |port| {
                port.open_bar()
                    .and_then(BarScreen::hire_screen)
                    .and_then(nova_view::spaceport::HireScreen::info_panel)
                    .is_some()
            },
        ),
        (
            "an offer over the bar",
            |screen| {
                click_port_item(screen, 10);
                screen.tick(Duration::from_millis(300));
                screen.tick(Duration::from_millis(300));
                assert_eq!(
                    offered_mission(
                        spaceport(screen)
                            .open_bar()
                            .and_then(BarScreen::offer_dialog)
                    ),
                    Some(MissionId(129))
                );
            },
            |port| port.open_bar().and_then(BarScreen::offer_dialog).is_some(),
        ),
    ];

    #[test]
    fn mission_info_closing_over_each_opener_shows_the_nav_its_abort_wrote() {
        for (what, open, still_open) in INFO_OPENERS {
            let store = MemoryPilots::new();
            let mut screen = AppScreen::new(aborting_to_beta_data())
                .with_dialogs(Rc::new(InfoOverEveryOpener), Rc::new(MonoMetrics))
                .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
            create(&mut screen, "Ada");
            land_twice(&mut screen);
            screen.input(&key(Key::Enter, true));
            assert_eq!(active(&screen).len(), 2, "{what}: both accepted");
            open(&mut screen);
            assert!(still_open(spaceport(&screen)), "{what}: open");
            screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, true));
            screen.input(&key(nova_view::ui::mission_info::MISSION_INFO_KEY, false));
            let abort = spaceport(&screen)
                .mission_info()
                .and_then(MissionInfoScreen::dialog)
                .and_then(|dialog| dialog.item_bounds(nova_view::spaceport::missions::ABORT_ITEM))
                .expect("Abort")
                .center();
            click_on(&mut screen, abort);
            assert_eq!(active(&screen).len(), 1, "{what}: one aborted");
            assert_eq!(
                flight(&screen).session().map(Session::nav_mode),
                Ok(nova_sim::NavMode::Stellar(Some(nova_sim::StellarId(129)))),
                "{what}: its M129 selected Beta Prime"
            );
            assert!(
                spaceport(&screen).mission_info().is_some(),
                "{what}: info open"
            );
            assert!(!names_beta(&screen), "{what}: held under it");
            screen.input(&key(Key::Escape, true));
            assert!(
                spaceport(&screen).mission_info().is_none(),
                "{what}: closed"
            );
            assert!(still_open(spaceport(&screen)), "{what}: still open");
            assert!(
                names_beta(&screen),
                "{what}: redrawn as it closes (@0x9e769)"
            );
        }
    }

    #[test]
    fn a_fuel_tank_bought_in_port_leaves_the_bars_fuel_gauge_until_the_outfitter_closes() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, barred_outfitting_data());
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        assert_eq!(fuel_width(&drawn(&screen)), Some(149.0), "landing's");
        click_port_item(&mut screen, 8);
        click_for_outfit(&mut screen, TANK, OUTFIT_BUY_ITEM);
        assert_eq!(pilot(&screen).owned(TANK), 1);
        assert_eq!(
            fuel_width(&drawn(&screen)),
            Some(149.0),
            "held while the outfitter is open"
        );
        let done = spaceport(&screen)
            .open_outfitter()
            .and_then(|open| open.dialog())
            .and_then(|dialog| dialog.item_bounds(1))
            .expect("Done")
            .center();
        click_on(&mut screen, done);
        assert!(spaceport(&screen).open_outfitter().is_none(), "closed");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width < 149.0, "redrawn as it closes: {width}");
    }

    use nova_data::records::govt::Govt;

    const UPGRADE: OutfitId = OutfitId(134);

    /// [`barred_outfitting_data`], where the outfitter also sells `oütf`
    /// 134, an upgrade of no mod for 100 credits whose `OnPurchase` is
    /// `H129`, and ship 129, carrying 300 fuel, is under gövt 128, whose
    /// `Interface` is `ïntf` 129: [`bar_interface`] again.
    fn barred_upgrading_data() -> Rc<GameData> {
        let mut upgrade = booster_record(b"H129");
        put_i16s(&mut upgrade, 0x02, &[0]);
        put_i16s(&mut upgrade, 0x06, &[0, 0]);
        upgrade[0x0E..0x12].copy_from_slice(&100_i32.to_be_bytes());
        let mut govt = vec![0; Govt::SIZE.expect("fixed")];
        put_i16s(&mut govt, 0xAC, &[129]);
        let fork = barred(game_data_fork(true, true, false, 100, b"", b"", None))
            .resource(Outfit::TYPE, 134, Some(b"Upgrade"), &upgrade)
            .resource(Govt::TYPE, 128, Some(b"Rebels"), &govt)
            .resource(Interface::TYPE, 129, None, &bar_interface());
        let mut built = fork.build();
        let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
        built.put_u16(first + 0x0A, 300);
        let second = built.layout.data_entry(Ship::TYPE, 129) + 4;
        built.put_u16(second + 0x0A, 300);
        built.put_u16(second + 0x48, 128);
        loaded_bytes(built.bytes)
    }

    #[test]
    fn an_upgrade_bought_in_port_onto_another_bar_redraws_its_fuel_gauge_at_once() {
        let store = MemoryPilots::new();
        let mut screen = landed_outfitter_over(&store, barred_upgrading_data());
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        assert_eq!(fuel_width(&drawn(&screen)), Some(149.0), "landing's");
        click_port_item(&mut screen, 8);
        click_for_outfit(&mut screen, UPGRADE, OUTFIT_BUY_ITEM);
        assert_eq!(pilot(&screen).ship(), ShipId(129), "upgraded");
        assert!(spaceport(&screen).open_outfitter().is_some(), "still open");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width < 149.0, "redrawn at once: {width}");
    }

    // Death.

    use nova_data::records::weapon::Weapon;

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    struct Firing;

    impl Behaviour for Firing {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
                only: None,
                turrets_only: false,
                bays: false,
                point_defence: false,
            }
        }
    }

    /// Disables no ship: the player breaks up once its armour is gone.
    #[derive(Debug)]
    struct Never;

    impl DisableRule for Never {
        fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
            false
        }
    }

    /// The first `chär` flies ship 128 ("First": no shield, 10 armour, a
    /// `DeathDelay` of 30, so it breaks up for 88 ticks) from system 128,
    /// over the planet Alpha Prime (128) at (0, 0) with a bar. The
    /// system's one `düde`, at 100 %, flies ship 129 ("Killer": 100
    /// armour and no speed, so it stays where setup places it), carrying
    /// one `wëap` 128: unguided, every 5 ticks, 20 pixels a tick for 30,
    /// 1000 mass damage through shields, so one shot breaks the player up.
    fn deadly_data() -> Rc<GameData> {
        deadly_data_with(&[])
    }

    /// A sabotage outfit: its `oütf` ID, the `dësc` its `ModType` 47
    /// slot names, and that `dësc`'s text, if it has one.
    type Saboteur<'a> = (i16, i16, Option<&'a str>);

    /// The sabotage `dësc` that shows a picture, and its `Graphic`: the
    /// others show none.
    const PICTURED_SABOTAGE: (i16, i16) = (5000, 300);

    /// [`deadly_data`], the player's ship carrying one of each of
    /// `saboteurs` as its default items, their `dësc`s pictured as
    /// [`PICTURED_SABOTAGE`] says.
    fn deadly_data_with(saboteurs: &[Saboteur]) -> Rc<GameData> {
        let items: Vec<(i16, [i16; 3])> = saboteurs
            .iter()
            .map(|&(outfit, desc, _)| (outfit, [nova_sim::SABOTAGE_KILL, desc, 1]))
            .collect();
        let fork =
            saboteurs
                .iter()
                .fold(deadly_fork(&items), |fork, &(_, desc, text)| match text {
                    Some(text) => {
                        let graphic = if desc == PICTURED_SABOTAGE.0 {
                            PICTURED_SABOTAGE.1
                        } else {
                            -1
                        };
                        fork.resource(Desc::TYPE, desc, None, &pictured_desc_of(text, graphic))
                    }
                    None => fork,
                });
        loaded(&fork)
    }

    /// [`deadly_data`]'s fork, the player's ship carrying one of each of
    /// `items` as its default items: each an `oütf` ID with its first
    /// `ModType`, `ModVal` and the field after them.
    fn deadly_fork(items: &[(i16, [i16; 3])]) -> ForkBuilder {
        let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
            for (i, value) in values.iter().enumerate() {
                bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
            }
        };
        let ship = |armor, weapon| {
            let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
            put(&mut bytes, 0x00, &[0, 0, 300, 300, 10, 300, 0, armor]);
            put(&mut bytes, 0x12, &[weapon, -1, -1, -1]);
            put(&mut bytes, 0x1A, &[1, 0, 0, 0]);
            put(&mut bytes, 0x42, &[1, 10]);
            put(&mut bytes, 0x4E, &[-1; 4]);
            put(&mut bytes, 0x370, &[-1; 4]);
            put(&mut bytes, 0x6CE, &[-1; 4]);
            bytes
        };
        let mut player = ship(10, -1);
        put(&mut player, 0x34, &[30]);
        for (slot, &(outfit, _)) in items.iter().enumerate() {
            put(&mut player, 0x4E + 2 * slot, &[outfit]);
            put(&mut player, 0x56 + 2 * slot, &[1]);
        }
        let mut killer = ship(100, 128);
        put(&mut killer, 0x06, &[0]);
        let mut weapon = vec![0; Weapon::SIZE.expect("fixed")];
        put(&mut weapon, 0x00, &[5, 30, 1000, 0, -1, 2000, -1, 0, 0, -1]);
        put(&mut weapon, 0x16, &[-1]);
        put(&mut weapon, 0x1C, &[0x0020]);
        let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
        put(&mut anim, 0x00, &[1000, 0, 1]);
        put(&mut anim, 0x34, &[4]);
        let sheet = (0..4)
            .fold(RledBuilder::new(1, 1), |sheet, _| {
                sheet.frame(|f| f.line().pixels(&[0x7C00]))
            })
            .build();
        let mut character = vec![0; Character::SIZE.expect("fixed")];
        put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
        put(&mut character, 0x0E, &[-1; 4]);
        let mut system = system(0, &[128]);
        put(&mut system, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
        put(&mut system, 0x54, &[100]);
        put(&mut system, 0x64, &[1]);
        let mut dude = vec![0; Dude::SIZE.expect("fixed")];
        put(&mut dude, 0x00, &[1, -1]);
        put(&mut dude, 0x08, &[-1; 16]);
        put(&mut dude, 0x08, &[129]);
        put(&mut dude, 0x28, &[100]);
        let mut spin = vec![0; Spin::SIZE.expect("fixed")];
        spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
        let fork = items
            .iter()
            .fold(ForkBuilder::new(), |fork, &(outfit, mods)| {
                let mut record = vec![0; Outfit::SIZE.expect("fixed")];
                put(&mut record, 0x06, &mods);
                fork.resource(Outfit::TYPE, outfit, Some(b"Bomb"), &record)
            });
        fork.resource(Ship::TYPE, 128, Some(b"First"), &player)
            .resource(Ship::TYPE, 129, Some(b"Killer"), &killer)
            .resource(Weapon::TYPE, 128, Some(b"Blaster"), &weapon)
            .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system)
            .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &landable())
            .resource(Spin::TYPE, 1000, None, &spin)
            .resource(Dude::TYPE, 128, Some(b"Killers"), &dude)
    }

    /// The rescue's `dësc` text in [`pod_data`].
    const RESCUE_TEXT: &str = "After several weeks of sensory deprivation";

    /// The rescue's `dësc`'s `Graphic` in [`pod_data`].
    const RESCUE_GRAPHIC: i16 = 400;

    /// [`deadly_data`] where the player's ship carries an escape pod
    /// (`oütf` 200, `ModType` 11) and auto-eject (201, `ModType` 20), with
    /// the pod's class (`shïp` 895) and, when `desc` gives its text, the
    /// rescue's `dësc` 13999.
    fn escape_pod_data(desc: Option<&str>) -> Rc<GameData> {
        let fork = escape_pod_fork();
        match desc {
            Some(text) => loaded(&fork.resource(
                Desc::TYPE,
                13999,
                None,
                &pictured_desc_of(text, RESCUE_GRAPHIC),
            )),
            None => loaded(&fork),
        }
    }

    /// [`escape_pod_data`]'s fork, with no rescue `dësc`.
    fn escape_pod_fork() -> ForkBuilder {
        let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
            for (i, value) in values.iter().enumerate() {
                bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
            }
        };
        let mut pod = vec![0; Ship::SIZE.expect("fixed")];
        put(&mut pod, 0x00, &[0, 0, 300, 300, 10, 300, 0, 10]);
        put(&mut pod, 0x12, &[-1; 4]);
        put(&mut pod, 0x4E, &[-1; 4]);
        put(&mut pod, 0x370, &[-1; 4]);
        put(&mut pod, 0x6CE, &[-1; 4]);
        deadly_fork(&[(200, [11, 0, 1]), (201, [20, 0, 1])]).resource(
            Ship::TYPE,
            895,
            Some(b"Escape Pod"),
            &pod,
        )
    }

    /// The router over [`escape_pod_data`] (with its `dësc` of the text
    /// `desc` gives, if any), flying with the killer above the player,
    /// until the player is in the escape pod.
    fn ejected(store: &MemoryPilots, desc: Option<&str>) -> AppScreen {
        ejected_by(store, desc, create)
    }

    /// [`ejected`], the pilot made by `create`.
    fn ejected_by(
        store: &MemoryPilots,
        desc: Option<&str>,
        create: fn(&mut AppScreen, &str),
    ) -> AppScreen {
        ejected_from(store, with_dialogs(escape_pod_data(desc)), create)
    }

    /// `screen`, over [`escape_pod_data`] or its like, flying with the
    /// killer above the player, the pilot made by `create`, until the
    /// player is in the escape pod.
    fn ejected_from(
        store: &MemoryPilots,
        screen: AppScreen,
        create: fn(&mut AppScreen, &str),
    ) -> AppScreen {
        let mut screen = screen
            .with_chance(above_the_player())
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Never))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.input(&key(Key::Escape, true));
        tick_until(&mut screen, 240, |screen| {
            matches!(fate(screen), nova_sim::PlayerFate::Ejected { .. })
        });
        screen
    }

    #[test]
    fn a_rescue_desc_that_cannot_be_read_is_skipped_with_a_warning() {
        let store = MemoryPilots::new();
        let mut screen = ejected(&store, None);
        tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            fate(screen) == nova_sim::PlayerFate::Flying
        });
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot show the rescue dësc 13999: no dësc 13999"]
        );
    }

    #[test]
    fn the_rescue_shows_its_desc_over_flight_and_stops_stepping() {
        let store = MemoryPilots::new();
        let mut screen = ejected(&store, Some(RESCUE_TEXT));
        let ticks = tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(ticks, nova_sim::POD_TICKS, "on the countdown's last tick");
        assert_eq!(shown_text(&screen).as_deref(), Some(RESCUE_TEXT));
        assert_eq!(
            shown_picture(&screen),
            Some((300.0, Some(RESCUE_GRAPHIC))),
            "DLOG 3004, its graphic shown"
        );
        assert_eq!(fate(&screen), nova_sim::PlayerFate::Flying);
        let at = ship(&screen);
        screen.tick(Duration::from_secs(2));
        assert_eq!(ship(&screen), at, "nothing moves under the dialog");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "flying on");
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn the_rescues_desc_plays_its_movie_over_flight() {
        let store = MemoryPilots::new();
        let data = loaded(&escape_pod_fork().resource(
            Desc::TYPE,
            13999,
            None,
            &filmed_desc_of(RESCUE_TEXT, "Rescue.mov", 0),
        ));
        let reel = Rc::new(Reel::default());
        let screen = with_dialogs(data).with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>);
        let mut screen = ejected_from(&store, screen, create);
        assert!(reeled(&reel).is_empty());
        tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            screen.showing() != Showing::Flight
        });
        assert_eq!(screen.showing(), Showing::Movie, "before its text");
        assert_eq!(reeled(&reel), ["start Rescue.mov"]);
        assert_eq!(fate(&screen), nova_sim::PlayerFate::Flying);
        tick_until(&mut screen, 10, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(reeled(&reel), ["start Rescue.mov", "stop"]);
        assert_eq!(shown_text(&screen).as_deref(), Some(RESCUE_TEXT));
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "flying on");
    }

    #[test]
    fn the_rescue_desc_reads_its_conditional_text_through_flight() {
        let store = MemoryPilots::new();
        let mut screen = ejected(&store, Some("Rescued, {G \"sir\" \"ma'am\"}."));
        tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(
            shown_text(&screen).as_deref(),
            Some("Rescued, sir."),
            "loaded as _LoadDescription loads it (@0x68e90)"
        );
    }

    #[test]
    fn a_strict_pilot_rescued_from_the_pod_is_saved_before_the_rescue_is_drawn() {
        let store = MemoryPilots::new();
        let mut screen = ejected_by(&store, Some(RESCUE_TEXT), create_strict);
        let before = store.writes();
        tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(store.writes(), before + 1, "saved on the rescue's tick");
        let saved = saved(&store, "Ada");
        assert_eq!(saved.stellar(), Some(nova_sim::StellarId(128)));
        assert_eq!(saved.system(), nova_sim::SystemId(128));
    }

    #[test]
    fn a_pilot_rescued_from_the_pod_without_strict_play_is_not_saved() {
        let store = MemoryPilots::new();
        let mut screen = ejected_by(&store, Some(RESCUE_TEXT), create);
        let before = store.writes();
        tick_until(&mut screen, nova_sim::POD_TICKS + 1, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(store.writes(), before, "no save on the rescue's tick");
    }

    /// Draws the last outcome of every roll but the traffic's placement:
    /// its two 1500-wide draws place a ship at (0, -100), 100 pixels
    /// above the player at the centre, and its heading draw faces it down
    /// at the player. However many rolls landing and taking off make
    /// first, the killer is placed there.
    #[derive(Default)]
    struct AboveThePlayer {
        placed: usize,
    }

    impl Chance for AboveThePlayer {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            match n {
                1500 => {
                    self.placed += 1;
                    if self.placed % 2 == 1 { 750 } else { 650 }
                }
                360 => 180,
                _ => n - 1,
            }
        }
    }

    fn above_the_player() -> SharedChance {
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(AboveThePlayer::default()));
        SharedChance::new(shared)
    }

    /// The router over [`deadly_data`] with the main menu over `store`,
    /// dialogs, the killer placed above the player and firing, and no ship
    /// disabled.
    fn deadly_menu(store: &MemoryPilots) -> AppScreen {
        with_dialogs(deadly_data())
            .with_chance(above_the_player())
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Never))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
    }

    fn fate(screen: &AppScreen) -> nova_sim::PlayerFate {
        flight(screen).session().expect("flying").player_fate()
    }

    /// Ticks `screen` one sim tick at a time until `done`, at most `limit`
    /// ticks, and says how many it took.
    fn tick_until(screen: &mut AppScreen, limit: u32, done: impl Fn(&AppScreen) -> bool) -> u32 {
        for n in 1..=limit {
            screen.tick(TICK);
            if done(screen) {
                return n;
            }
        }
        panic!("not within {limit} ticks: {:?}", screen.showing())
    }

    /// Ticks `screen` until the player's ship breaks up.
    fn until_breaking_up(screen: &mut AppScreen) {
        tick_until(screen, 120, |screen| {
            matches!(fate(screen), nova_sim::PlayerFate::BreakingUp { .. })
        });
    }

    /// Ticks `screen` until the player's ship has exploded.
    fn until_exploded(screen: &mut AppScreen) {
        tick_until(screen, 240, |screen| {
            fate(screen) == nova_sim::PlayerFate::Exploded
        });
    }

    /// The alpha of the black covering the whole screen, if the last
    /// thing `screen` draws is one.
    #[allow(clippy::float_cmp)] // The screen's height, exactly as drawn.
    fn black_over(screen: &AppScreen) -> Option<u8> {
        match drawn(screen).iter().last() {
            Some(DrawCommand::Line { width, color, .. })
                if *width == 768.0 && (color.r, color.g, color.b) == (0, 0, 0) =>
            {
                Some(color.a)
            }
            _ => None,
        }
    }

    /// A pilot, "Ada", created on [`deadly_menu`] over `store`, landed and
    /// taken off again, before the killer is in the system.
    fn landed_and_took_off(store: &MemoryPilots) -> AppScreen {
        landed_and_took_off_by(store, create)
    }

    /// [`landed_and_took_off`] with a Strict Play pilot.
    fn strict_landed_and_took_off(store: &MemoryPilots) -> AppScreen {
        landed_and_took_off_by(store, create_strict)
    }

    /// [`landed_and_took_off`], the pilot made by `create`.
    fn landed_and_took_off_by(store: &MemoryPilots, create: fn(&mut AppScreen, &str)) -> AppScreen {
        let mut screen = deadly_menu(store);
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "taken off");
        screen
    }

    /// [`landed_and_took_off`], then the killer breaks its ship up.
    fn landed_took_off_and_breaking_up(store: &MemoryPilots) -> AppScreen {
        let mut screen = landed_and_took_off(store);
        until_breaking_up(&mut screen);
        screen
    }

    #[test]
    fn a_pilot_that_landed_took_off_and_died_is_saved_by_the_landing_and_take_off_only() {
        let store = MemoryPilots::new();
        let mut screen = landed_and_took_off(&store);
        let took_off = pilot(&screen).clone();
        until_exploded(&mut screen);
        tick_until(&mut screen, 400, |screen| {
            screen.showing() == Showing::MainMenu
        });
        let written = store.written();
        assert_eq!(written.len(), 3, "creation, landing, take-off: {written:?}");
        assert!(written.iter().all(|(key, _)| key == "Ada"));
        let last = nova_sim::save::decode(&written[2].1).expect("a pilot");
        assert_eq!(last, took_off, "the take-off's save is the last");
        assert_eq!(store.deleted(), Vec::<String>::new(), "not strict: kept");
        assert_eq!(store.keys(), ["Ada"]);
        let mut screen = deadly_menu(&store);
        choose(&mut screen, MenuChoice::OpenPilot);
        screen.input(&key(Key::Enter, true));
        assert_eq!(pilot(&screen), &last, "reopened as at the take-off");
    }

    #[test]
    fn a_dead_pilot_holds_fades_and_returns_to_the_main_menu() {
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        until_exploded(&mut screen);
        for tick in 1..=nova_sim::HOLD_TICKS {
            screen.tick(TICK);
            assert_eq!(screen.showing(), Showing::Flight, "dead tick {tick}");
            assert_eq!(black_over(&screen), None, "dead tick {tick}");
        }
        screen.tick(TICK);
        assert_eq!(
            fate(&screen),
            nova_sim::PlayerFate::Fading,
            "on the expiry's tick, straight through the seam"
        );
        assert_eq!(screen.showing(), Showing::Flight, "no dialog");
        screen.tick(Duration::from_millis(500));
        assert_eq!(black_over(&screen), Some(128));
        assert_eq!(screen.showing(), Showing::Flight);
        screen.tick(Duration::from_millis(500));
        assert_eq!(screen.showing(), Showing::MainMenu, "back to the menu");
        assert_eq!(black_over(&screen), Some(255), "from black");
        screen.tick(Duration::from_millis(500));
        assert_eq!(black_over(&screen), Some(128), "fading in");
        screen.tick(Duration::from_millis(500));
        assert_eq!(black_over(&screen), None, "faded in");
        assert_eq!(
            choose(&mut screen, MenuChoice::NewPilot),
            ScreenAction::None
        );
        assert_eq!(screen.showing(), Showing::NewPilot, "the menu takes input");
    }

    #[test]
    fn the_main_menu_takes_input_under_its_fade_in() {
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        until_exploded(&mut screen);
        tick_until(&mut screen, 400, |screen| {
            screen.showing() == Showing::MainMenu
        });
        assert!(black_over(&screen).is_some());
        assert_eq!(
            choose(&mut screen, MenuChoice::NewPilot),
            ScreenAction::None
        );
        assert_eq!(screen.showing(), Showing::NewPilot);
    }

    #[test]
    fn after_the_return_no_flight_is_left_to_re_enter() {
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        until_exploded(&mut screen);
        tick_until(&mut screen, 400, |screen| {
            screen.showing() == Showing::MainMenu
        });
        assert!(screen.flight_view().is_none());
        assert!(screen.spaceport_view().is_none());
        screen.quit();
        assert_eq!(store.writes(), 3, "nothing left to save");
    }

    #[test]
    fn without_a_main_menu_a_dead_pilot_returns_to_the_side_flight_was_entered_from() {
        let mut screen = with_dialogs(deadly_data())
            .with_chance(above_the_player())
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Never));
        fly(&mut screen);
        until_exploded(&mut screen);
        tick_until(&mut screen, 400, |screen| {
            screen.showing() != Showing::Flight
        });
        assert_eq!(screen.showing(), Showing::ShipBrowser);
        assert!(screen.flight_view().is_none());
        assert_eq!(black_over(&screen), Some(255), "from black");
    }

    #[test]
    fn escape_during_the_breakup_writes_no_save_and_drops_flight() {
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        let writes = store.writes();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(store.writes(), writes, "no save");
        assert!(screen.flight_view().is_none(), "flight dropped");
        assert!(screen.spaceport_view().is_none());
    }

    #[test]
    fn quitting_during_the_breakup_or_the_hold_writes_no_save() {
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        let writes = store.writes();
        screen.quit();
        assert_eq!(store.writes(), writes, "breaking up");
        until_exploded(&mut screen);
        screen.tick(TICK);
        assert!(matches!(
            fate(&screen),
            nova_sim::PlayerFate::Holding { .. }
        ));
        screen.quit();
        assert_eq!(store.writes(), writes, "holding");
    }

    #[test]
    fn a_save_due_while_dying_is_not_written() {
        use nova_view::devtools::{PilotDesk, PilotEdit};
        let store = MemoryPilots::new();
        let mut screen = landed_took_off_and_breaking_up(&store);
        let writes = store.writes();
        screen
            .pilot_desk()
            .expect("flying")
            .edit(PilotEdit::Credits(5000))
            .expect("edits in flight");
        screen.tick(TICK);
        assert_eq!(store.writes(), writes, "due, but not written");
        screen.after_pilot_edit();
        assert_eq!(store.writes(), writes, "nor after the edit");
    }

    /// The strict pilot's death, ticked from the explosion to the main
    /// menu.
    fn until_the_main_menu(screen: &mut AppScreen) {
        tick_until(screen, 400, |screen| screen.showing() == Showing::MainMenu);
    }

    #[test]
    fn a_strict_pilot_destroyed_is_deleted_before_the_main_menu_shows() {
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        assert!(pilot(&screen).strict_play());
        let writes = store.writes();
        until_exploded(&mut screen);
        tick_until(&mut screen, 400, |screen| {
            let shown = screen.showing() == Showing::MainMenu;
            assert_eq!(
                store.deleted().is_empty(),
                !shown,
                "deleted as the menu shows, not before"
            );
            shown
        });
        assert_eq!(store.deleted(), ["Ada"]);
        assert_eq!(store.keys(), Vec::<String>::new());
        assert_eq!(store.writes(), writes, "nothing written after the take-off");
        choose(&mut screen, MenuChoice::OpenPilot);
        assert_eq!(
            screen.pilot_list().expect("open").keys(),
            Vec::<String>::new()
        );
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn escape_while_a_strict_pilot_breaks_up_deletes_it() {
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        until_breaking_up(&mut screen);
        let writes = store.writes();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(store.deleted(), ["Ada"]);
        assert_eq!(store.keys(), Vec::<String>::new());
        screen.quit();
        assert_eq!(store.writes(), writes, "nothing written");
        assert_eq!(store.keys(), Vec::<String>::new(), "nor written back");
    }

    #[test]
    fn quitting_while_a_strict_pilot_breaks_up_or_holds_deletes_it() {
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        until_breaking_up(&mut screen);
        let writes = store.writes();
        screen.quit();
        assert_eq!(store.deleted(), ["Ada"], "breaking up");
        assert_eq!(store.keys(), Vec::<String>::new());
        assert_eq!(store.writes(), writes);

        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        let writes = store.writes();
        until_exploded(&mut screen);
        screen.tick(TICK);
        assert!(matches!(
            fate(&screen),
            nova_sim::PlayerFate::Holding { .. }
        ));
        screen.quit();
        assert_eq!(store.deleted(), ["Ada"], "holding");
        assert_eq!(store.keys(), Vec::<String>::new());
        assert_eq!(store.writes(), writes);
    }

    #[test]
    fn a_save_due_while_a_strict_pilot_dies_writes_nothing_after_the_delete() {
        use nova_view::devtools::{PilotDesk, PilotEdit};
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        until_breaking_up(&mut screen);
        let writes = store.writes();
        screen
            .pilot_desk()
            .expect("flying")
            .edit(PilotEdit::Credits(5000))
            .expect("edits in flight");
        screen.tick(TICK);
        screen.after_pilot_edit();
        until_exploded(&mut screen);
        until_the_main_menu(&mut screen);
        assert_eq!(store.deleted(), ["Ada"]);
        assert_eq!(store.writes(), writes, "nothing written");
        assert_eq!(store.keys(), Vec::<String>::new());
    }

    #[test]
    fn a_strict_pilot_put_away_flying_is_saved_not_deleted() {
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        let writes = store.writes();
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::MainMenu);
        assert_eq!(store.writes(), writes + 1, "saved");
        assert_eq!(store.deleted(), Vec::<String>::new());
        assert!(saved(&store, "Ada").strict_play());
    }

    #[test]
    fn a_failed_delete_becomes_a_warning() {
        let store = MemoryPilots::new();
        let mut screen = strict_landed_and_took_off(&store);
        store.fail_deletes(true);
        until_exploded(&mut screen);
        until_the_main_menu(&mut screen);
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot delete the pilot Ada in memory (the disk is locked)"]
        );
        assert_eq!(store.keys(), ["Ada"], "still there");
    }

    // Sabotage.

    /// The text of the mission text shown over flight, if any.
    fn shown_text(screen: &AppScreen) -> Option<String> {
        screen
            .flight_mission_text()
            .map(|text| text.lines().join(" "))
    }

    /// The width of the text dialog shown over flight, and its picture.
    fn shown_picture(screen: &AppScreen) -> Option<(f32, Option<i16>)> {
        screen
            .flight_mission_text()
            .map(|text| (text.dialog().bounds().width(), text.picture()))
    }

    /// [`landed_and_took_off`] over [`deadly_data_with`] `saboteurs`.
    fn sabotaged_and_took_off(store: &MemoryPilots, saboteurs: &[Saboteur]) -> AppScreen {
        took_off_over(store, with_dialogs(deadly_data_with(saboteurs)))
    }

    /// `screen`, over [`deadly_data`] or its like, landed and taken off
    /// with the killer above the player.
    fn took_off_over(store: &MemoryPilots, screen: AppScreen) -> AppScreen {
        let mut screen = screen
            .with_chance(above_the_player())
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Never))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "taken off");
        screen
    }

    /// Two sabotage outfits, 128 and 129, naming `dësc`s 5000 and 5001.
    const TWO_SABOTEURS: [Saboteur; 2] = [
        (128, 5000, Some("Your ship was sabotaged.")),
        (129, 5001, Some("Twice over.")),
    ];

    #[test]
    fn a_sabotage_desc_reads_its_conditional_text_through_flight() {
        let store = MemoryPilots::new();
        let mut screen = sabotaged_and_took_off(
            &store,
            &[(128, 5000, Some("Sabotaged, {G \"sir\" \"ma'am\"}."))],
        );
        until_exploded(&mut screen);
        tick_until(&mut screen, nova_sim::HOLD_TICKS + 2, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(
            shown_text(&screen).as_deref(),
            Some("Sabotaged, sir."),
            "loaded as _LoadDescription loads it (@0x687c9)"
        );
    }

    #[test]
    fn a_sabotaged_pilot_sees_each_outfits_desc_after_the_hold_and_then_fades() {
        let store = MemoryPilots::new();
        let mut screen = sabotaged_and_took_off(&store, &TWO_SABOTEURS);
        until_exploded(&mut screen);
        for tick in 1..=nova_sim::HOLD_TICKS {
            screen.tick(TICK);
            assert_eq!(screen.showing(), Showing::Flight, "dead tick {tick}");
        }
        screen.tick(TICK);
        assert_eq!(
            screen.showing(),
            Showing::MissionText,
            "on the expiry's tick"
        );
        assert_eq!(
            shown_text(&screen).as_deref(),
            Some("Your ship was sabotaged.")
        );
        assert_eq!(
            shown_picture(&screen),
            Some((300.0, Some(PICTURED_SABOTAGE.1))),
            "DLOG 3004, its graphic shown"
        );
        let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
        assert_eq!(fate(&screen), held, "not fading under the dialog");
        assert!(flight(&screen).session().expect("flying").hold_expired());
        screen.tick(Duration::from_secs(2));
        assert_eq!(fate(&screen), held, "nothing moves under the dialog");
        assert_eq!(screen.showing(), Showing::MissionText);
        assert_eq!(black_over(&screen), None);
        screen.input(&key(Key::Enter, true));
        assert_eq!(shown_text(&screen).as_deref(), Some("Twice over."));
        assert_eq!(shown_picture(&screen), Some((200.0, None)), "no graphic");
        assert_eq!(fate(&screen), held, "one dialog left");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "the last closed");
        assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
        screen.tick(Duration::from_millis(500));
        assert_eq!(black_over(&screen), Some(128));
        screen.tick(Duration::from_millis(500));
        assert_eq!(screen.showing(), Showing::MainMenu, "back to the menu");
        assert_eq!(screen.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn the_sabotage_dialogs_let_go_of_flights_keys() {
        let store = MemoryPilots::new();
        let mut screen = sabotaged_and_took_off(&store, &TWO_SABOTEURS);
        until_exploded(&mut screen);
        for _ in 1..=nova_sim::HOLD_TICKS {
            screen.tick(TICK);
        }
        screen.input(&key(Key::Up, true));
        assert!(flight(&screen).holds_keys(), "held before the expiry");
        screen.tick(TICK);
        assert_eq!(screen.showing(), Showing::MissionText);
        assert!(!flight(&screen).holds_keys(), "let go");
    }

    #[test]
    fn a_sabotage_desc_that_cannot_be_read_is_skipped_with_a_warning() {
        let store = MemoryPilots::new();
        let mut screen = sabotaged_and_took_off(
            &store,
            &[(128, 5000, None), (129, 5001, Some("Twice over."))],
        );
        until_exploded(&mut screen);
        tick_until(&mut screen, nova_sim::HOLD_TICKS + 1, |screen| {
            screen.showing() != Showing::Flight
        });
        assert_eq!(shown_text(&screen).as_deref(), Some("Twice over."));
        assert_eq!(
            screen.take_warnings(),
            ["nova: cannot show the sabotage dësc 5000: no dësc 5000"]
        );
        screen.input(&key(Key::Enter, true));
        assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
    }

    #[test]
    fn with_no_sabotage_desc_to_read_the_fade_begins_on_the_expirys_tick() {
        let store = MemoryPilots::new();
        let mut screen = sabotaged_and_took_off(&store, &[(128, 5000, None)]);
        until_exploded(&mut screen);
        for _ in 0..=nova_sim::HOLD_TICKS {
            screen.tick(TICK);
        }
        assert_eq!(screen.showing(), Showing::Flight, "no dialog");
        assert_eq!(fate(&screen), nova_sim::PlayerFate::Fading);
        assert_eq!(screen.take_warnings().len(), 1);
    }

    /// The router sabotaged by one outfit, 128, whose `dësc` 5000,
    /// "Sabotaged.", names the movie "Sabotage.mov" of `Flags` `flags`,
    /// its movies played through a new [`Reel`], taken off and ticked
    /// until the player's ship has exploded.
    fn filmed_sabotage(store: &MemoryPilots, flags: u16) -> (AppScreen, Rc<Reel>) {
        let data = loaded(
            &deadly_fork(&[(128, [nova_sim::SABOTAGE_KILL, 5000, 1])]).resource(
                Desc::TYPE,
                5000,
                None,
                &filmed_desc_of("Sabotaged.", "Sabotage.mov", flags),
            ),
        );
        let reel = Rc::new(Reel::default());
        let screen = with_dialogs(data).with_movie_player(Rc::clone(&reel) as Rc<dyn MoviePlayer>);
        let mut screen = took_off_over(store, screen);
        until_exploded(&mut screen);
        (screen, reel)
    }

    #[test]
    fn a_sabotage_descs_movie_starts_as_the_hold_expires_and_the_fade_waits_for_it_and_its_text() {
        let store = MemoryPilots::new();
        let (mut screen, reel) = filmed_sabotage(&store, 0);
        for _ in 1..=nova_sim::HOLD_TICKS {
            screen.tick(TICK);
        }
        assert!(reeled(&reel).is_empty(), "not before the expiry");
        screen.tick(TICK);
        assert_eq!(
            reeled(&reel),
            ["start Sabotage.mov"],
            "on the expiry's tick"
        );
        assert_eq!(screen.showing(), Showing::Movie, "before its text");
        let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
        assert_eq!(fate(&screen), held, "not fading under the movie");
        tick_until(&mut screen, 10, |screen| {
            screen.showing() == Showing::MissionText
        });
        assert_eq!(reeled(&reel), ["start Sabotage.mov", "stop"]);
        assert_eq!(shown_text(&screen).as_deref(), Some("Sabotaged."));
        assert_eq!(fate(&screen), held, "nor under the text");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(
            fate(&screen),
            nova_sim::PlayerFate::Fading,
            "once both close"
        );
    }

    #[test]
    fn a_sabotage_descs_after_movie_holds_the_fade_until_the_tick_it_ends() {
        let store = MemoryPilots::new();
        let (mut screen, reel) = filmed_sabotage(&store, 0x0001);
        tick_until(&mut screen, nova_sim::HOLD_TICKS + 2, |screen| {
            screen.showing() != Showing::Flight
        });
        assert_eq!(shown_text(&screen).as_deref(), Some("Sabotaged."));
        assert!(reeled(&reel).is_empty(), "after its text");
        screen.input(&key(Key::Enter, true));
        assert_eq!(reeled(&reel), ["start Sabotage.mov"]);
        assert_eq!(screen.showing(), Showing::Movie);
        let held = nova_sim::PlayerFate::Holding { ticks_left: 0 };
        for _ in 0..10 {
            if reeled(&reel).len() > 1 {
                break;
            }
            assert_eq!(fate(&screen), held, "not while it plays");
            screen.tick(TICK);
        }
        assert_eq!(reeled(&reel), ["start Sabotage.mov", "stop"]);
        assert_eq!(screen.showing(), Showing::Flight);
        assert_eq!(
            fate(&screen),
            nova_sim::PlayerFate::Fading,
            "on the tick the movie ended"
        );
    }

    // The picture variants, as the router lays them out.

    use nova_data::records::person::Person;
    use nova_view::ui::Dialog;

    /// A `dësc` `Graphic` above 127, so the picture variants show.
    const SHOWN_GRAPHIC: i16 = 5008;

    /// The picture variant of [`offer_template`], told apart by its size:
    /// 400 x 250, its graphic (8) at (290, 10).
    fn picture_offer_template() -> DialogTemplate {
        let mut template = offer_template();
        template.bounds = Bounds::at(Point::new(0.0, 0.0), 400.0, 250.0);
        template.items.resize(8, template.items[3].clone());
        template.items[7].bounds = Bounds::at(Point::new(290.0, 10.0), 100.0, 100.0);
        template
    }

    /// The picture variant of [`bar_template`], told apart by its size:
    /// 266 x 306, its picture (8) above the text.
    fn picture_bar_template() -> DialogTemplate {
        let mut template = bar_template();
        template.bounds = Bounds::at(Point::new(0.0, 0.0), 266.0, 306.0);
        template.items[7].bounds = Bounds::at(Point::new(16.0, 120.0), 230.0, 100.0);
        template
    }

    /// [`PastTheHoldsDialogs`], with the picture variants of the offer
    /// (1020) and the bar (1021) too.
    struct PictureDialogs;

    impl DialogResources for PictureDialogs {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            match id {
                PICTURE_OFFER_DIALOG => Ok(picture_offer_template()),
                PICTURE_BAR_DIALOG => Ok(picture_bar_template()),
                _ => PastTheHoldsDialogs.dialog_template(id),
            }
        }
    }

    /// A 2 x 2 `PICT` of one colour.
    fn small_pict() -> Vec<u8> {
        let frame = [0, 0, 2, 2];
        PictBuilder::new(frame)
            .direct_bits(&DirectBits::rgb888(frame, &[[90, 90, 90]; 4]))
            .end()
            .build()
    }

    /// The first `chär` flies ship 128 in system 128, whose one Person
    /// slot, at 100 %, holds `përs` 128, independent, in ship 129, of
    /// `Flags` 0x0040, so hailing it offers its `LinkMission` 128 (offered
    /// from a ship on every visit) in place of the comm dialog; the
    /// mission's offer text, `dësc` 4000, has `Graphic` [`SHOWN_GRAPHIC`].
    fn person_offer_data() -> Rc<GameData> {
        loaded(&person_offer_fork(|_| {}))
    }

    /// [`person_offer_data`]'s fork, its `mïsn` 128 fitted by `fit`.
    fn person_offer_fork(fit: impl FnOnce(&mut Vec<u8>)) -> ForkBuilder {
        let put = |bytes: &mut [u8], at: usize, values: &[i16]| {
            for (i, value) in values.iter().enumerate() {
                bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
            }
        };
        let ship = |crew| {
            let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
            put(&mut bytes, 0x00, &[0, 0, 300, 300, 10, 300, 0, 100]);
            put(&mut bytes, 0x12, &[-1; 4]);
            put(&mut bytes, 0x42, &[1, crew]);
            put(&mut bytes, 0x4E, &[-1; 4]);
            put(&mut bytes, 0x370, &[-1; 4]);
            put(&mut bytes, 0x6CE, &[-1; 4]);
            bytes
        };
        let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
        put(&mut anim, 0x00, &[1000, 0, 1]);
        put(&mut anim, 0x34, &[4]);
        let sheet = (0..4)
            .fold(RledBuilder::new(1, 1), |sheet, _| {
                sheet.frame(|f| f.line().pixels(&[0x7C00]))
            })
            .build();
        let mut character = vec![0; Character::SIZE.expect("fixed")];
        put(&mut character, 0x04, &[128, 128, -1, -1, -1]);
        put(&mut character, 0x0E, &[-1; 4]);
        let mut system = vec![0; System::SIZE.expect("fixed")];
        put(&mut system, 0x04, &[-1; 32]);
        put(&mut system, 0x44, &[-1; 8]);
        put(&mut system, 0x64, &[0, -1]);
        put(&mut system, 0x6E, &[128, -1, -1, -1, -1, -1, -1, -1]);
        put(&mut system, 0x7E, &[100]);
        let mut person = vec![0; Person::SIZE.expect("fixed")];
        put(&mut person, 0x00, &[-2, -1, 3, 2, 0, 129]);
        put(&mut person, 0x0C, &[-1; 4]);
        put(&mut person, 0x2A, &[-1, -1, -1, 128]);
        person[0x32..0x34].copy_from_slice(&0x0040_u16.to_be_bytes());
        let mut mission = vec![0; Mission::SIZE.expect("fixed")];
        put(&mut mission, 0x04, &[2]);
        put(&mut mission, 0x0A, &[100]);
        put(&mut mission, 0x34, &[-1]);
        put(&mut mission, 0x58, &[-1]);
        fit(&mut mission);
        ForkBuilder::new()
            .resource(Ship::TYPE, 128, Some(b"First"), &ship(10))
            .resource(Ship::TYPE, 129, Some(b"Hauler"), &ship(3))
            .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system)
            .resource(Person::TYPE, 128, Some(b"Merchant"), &person)
            .resource(Mission::TYPE, 128, Some(b"Ride"), &mission)
            .resource(
                Desc::TYPE,
                128 + 3872,
                Some(b"Offer"),
                &pictured_desc_of("Ride along.", SHOWN_GRAPHIC),
            )
    }

    /// [`person_offer_data`] under the status bar ([`barred`]), where
    /// `mïsn` 128's `BriefText` is `dësc` 5000, "Go.", and its `OnAccept`
    /// is `G133`, a fuel tank of 100.
    fn briefing_person_offer_data() -> Rc<GameData> {
        let mut tank = booster_record(b"");
        put_i16s(&mut tank, 0x02, &[0]);
        put_i16s(&mut tank, 0x06, &[12, 100]);
        let fork = person_offer_fork(|mission| {
            put_i16s(mission, 0x34, &[5000]);
            mission[0x15B..0x15F].copy_from_slice(b"G133");
        })
        .resource(Outfit::TYPE, 133, Some(b"Tank"), &tank)
        .resource(Desc::TYPE, 5000, Some(b"Brief"), &desc_of("Go."));
        loaded(&barred(fork))
    }

    /// [`PictureDialogs`] without "Desc Dialog" (3003, 3004): mission
    /// texts are dropped as they come.
    struct NoTextDialogs;

    impl DialogResources for NoTextDialogs {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id == DESC_DIALOG || id == PICTURE_DESC_DIALOG {
                return Err(format!("no DLOG {id}"));
            }
            PictureDialogs.dialog_template(id)
        }
    }

    /// The router over [`briefing_person_offer_data`] with `dialogs`, in
    /// the developer's flight, the tank half full, hailing the merchant:
    /// its offer open.
    fn hailing_briefer(dialogs: Rc<dyn DialogResources>) -> AppScreen {
        let mut screen = AppScreen::new(briefing_person_offer_data())
            .with_dialogs(dialogs, Rc::new(MonoMetrics))
            .with_behaviour(Rc::new(Idle));
        fly(&mut screen);
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Offer);
        screen
    }

    #[test]
    fn a_persons_brief_shows_over_flights_pre_accept_bar_until_it_closes() {
        let mut screen = hailing_briefer(Rc::new(PictureDialogs));
        let half = fuel_width(&drawn(&screen)).expect("some fuel");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::MissionText, "the brief");
        assert_eq!(
            fuel_width(&drawn(&screen)),
            Some(half),
            "the bar from before OnAccept's tank"
        );
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "closed");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width > half, "live again, the tank's fuel too: {width}");
    }

    #[test]
    fn a_persons_brief_dropped_over_flight_lets_go_of_the_bar() {
        let mut screen = hailing_briefer(Rc::new(NoTextDialogs));
        let half = fuel_width(&drawn(&screen)).expect("some fuel");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "the brief dropped");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width > half, "live: {width}");
    }

    /// Whether `screen` draws `text`.
    fn draws_text(screen: &AppScreen, text: &str) -> bool {
        drawn(screen)
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text: t, .. } if t == text))
    }

    #[test]
    fn without_dialogs_a_persons_brief_dropped_over_flight_lets_go_of_the_bar() {
        let mut screen = AppScreen::new(briefing_person_offer_data()).with_behaviour(Rc::new(Idle));
        fly(&mut screen);
        edit(
            &mut screen,
            PilotEdit::Reserve(nova_sim::Reserve::Fuel, 150.0),
        );
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Offer, "its fallback layout");
        let half = fuel_width(&drawn(&screen)).expect("some fuel");
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::Flight, "the brief dropped");
        assert_eq!(active(&screen), [MissionId(128)], "accepted");
        let width = fuel_width(&drawn(&screen)).expect("some fuel");
        assert!(width > half, "the gauges live: {width}");
        let hyperspace = nova_view::flight::hud::NAV_HYPERSPACE;
        assert!(!draws_text(&screen, hyperspace), "not yet in mode 3");
        // H, flight's hyperspace mode key.
        screen.input(&key(Key::Char('h'), true));
        screen.input(&key(Key::Char('h'), false));
        assert!(draws_text(&screen, hyperspace), "the nav live");
    }

    #[test]
    fn a_persons_offer_with_a_graphic_above_127_opens_over_flight_in_1020() {
        let mut screen = AppScreen::new(person_offer_data())
            .with_dialogs(Rc::new(PictureDialogs), Rc::new(MonoMetrics))
            .with_behaviour(Rc::new(Idle));
        fly(&mut screen);
        hail(&mut screen);
        assert_eq!(screen.showing(), Showing::Offer, "over the paused flight");
        let dialog = screen.offer().expect("open");
        assert_eq!(dialog.prompt().mission, MissionId(128));
        assert_eq!(dialog.prompt().graphic, Some(SHOWN_GRAPHIC));
        assert_eq!(dialog.problem(), None);
        assert_eq!(
            dialog.dialog().map(Dialog::bounds),
            Some(picture_offer_template().bounds),
            "laid out by DLOG 1020"
        );
        assert!(
            screen
                .take_sounds()
                .contains(&Sound::Sim(SimSound::HailOffer)),
            "the hail's sound (@0x6223d)"
        );
    }

    #[test]
    fn a_bar_desc_whose_graphic_is_above_127_and_whose_pict_exists_opens_in_1021() {
        let fork = game_data_fork(true, true, true, 100, b"", b"", None)
            .resource(
                Desc::TYPE,
                10_000,
                None,
                &pictured_desc_of("A smoky bar.", SHOWN_GRAPHIC),
            )
            .resource(PICT, SHOWN_GRAPHIC, None, &small_pict());
        let store = MemoryPilots::new();
        let mut screen = AppScreen::new(loaded(&fork))
            .with_dialogs(Rc::new(PictureDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        click_port_item(&mut screen, 10);
        let bar = spaceport(&screen).open_bar().expect("in the bar");
        assert_eq!(bar.problem(), None);
        assert_eq!(
            bar.dialog().map(Dialog::bounds),
            Some(picture_bar_template().bounds),
            "laid out by DLOG 1021"
        );
    }
}
