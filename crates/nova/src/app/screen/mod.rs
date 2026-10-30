//! The screen router: every screen the app can show, as one [`Screen`](nova_view::Screen).
//!
//! # The main menu and pilots
//!
//! The game opens on the main menu ([`AppScreen::with_pilots`]), which
//! holds a game in progress, as the original's main screen holds a loaded
//! pilot (`_gameInProgress`; see `game`). It opens holding the last
//! pilot saved or opened, as `_OpenLastPilotFile` loads it at launch
//! ([`AppScreen::open_last_pilot`], see `launch`), or holding nothing,
//! silently, when that pilot can't be opened. New Pilot asks for a name in
//! "Create a new pilot:" (the interface file's, or a built-in one),
//! refusing a name already saved, then creates the pilot from the first
//! `chär`, saves it and holds it, not flying; with a game held it first
//! asks `STR#` 140 #8 in the Yes/No dialog, and No leaves the game as it
//! was. Open Pilot lists the saved pilots and holds the one chosen, its
//! explored systems' nebulae swept and its missions' auxiliary ships
//! restarted, as `_MainScreenChoice(1)` does; one of another version
//! closes the list and shows `STR#` 140 #34 over the menu, and any other
//! that can't be opened closes it with only a warning, the game held left
//! as it was. A pilot whose load had to fall back for data the save
//! references and the game data lacks ([`nova_sim::Session::load`], under
//! the router's rules) shows the note `STR#` 140 #52 over the menu.
//! Enter Ship (E, or a click) enters play with the game held, in space
//! over the stellar it last landed on, with no spaceport opened, as
//! `_EnterGameFromMainScreen` does (see `entry`): the main screen fades to
//! black, a pilot that hasn't seen the intro screen sees it (by
//! [`RuleKey::IntroForReopenedPilot`](nova_sim::RuleKey::IntroForReopenedPilot),
//! the last New Pilot's in this run, or the first `chär`'s), and flight
//! shows with a frozen, key-ignoring first frame and fades in from black;
//! with no game, or a pilot dying, it plays snd 153 and stays. Escape on the menu does
//! nothing. Quit quits after [`QUIT_DELAY`], and each of the menu's keys
//! first waits for its click to play (see `menu_input`); a click on Enter
//! Ship, Preferences or About does what E, P or A does, and a held P, A
//! or X repeats as a press. X's text and the About text read the pilot
//! held, or the start-up pilot with none. The menu is the original's main
//! screen, drawn from the game data's art, or a stand-in without it
//! ([`MainMenu`]). Closing the window on the menu asks `STR#` 140 #10 in
//! the Yes/No dialog and quits only on Yes; closing it anywhere else, or
//! Command-Q anywhere, quits at once (see `quit_question`), but while the
//! menu appears, below. Escape in flight, by the engine's reading of
//! [`RuleKey::EscapeToMainScreen`](nova_sim::RuleKey::EscapeToMainScreen),
//! saves the pilot and goes back to the menu, which holds it, so Enter
//! Ship resumes it; by the other reading it does nothing, as in the
//! original. Each flight put away or held leaves its ship slots'
//! draw-layer history, which every later flight of the run starts from,
//! as the original's sprite layers outlive a pilot
//! ([`nova_sim::ship_layer`]). A new pilot's neighbour pass reads what
//! the game held, else the last flight put away, left of its galaxy
//! refreshes ([`nova_sim::LastRefresh`]), or, with neither, the
//! launch's, as `_DoNewPilot` reads the visible flags the run's last
//! refresh wrote (@0x18fc5); entering play from the menu takes the held
//! game's afresh ([`FlightView::enter_game`]).
//!
//! A pilot with a name is saved through the [`PilotKeeper`] on creation,
//! on landing, on taking off, after each change made in the spaceport
//! (`AppScreen::transact`), on going back to the menu and when the app
//! quits in play ([`Screen::quit`](nova_view::Screen::quit)); never on an
//! open, nor when the app quits with the game only held; each failure is a warning
//! ([`Screen::take_warnings`](nova_view::Screen::take_warnings)). Tab on the menu goes to the developer's
//! sides below, where F flies a fresh pilot without a name, never saved,
//! and Escape at the top goes back to the menu. A router without a main
//! menu ([`start_screen`] alone) opens on the ship browser, and Escape at
//! the top quits.
//!
//! A pilot whose ship is breaking up or destroyed is never saved
//! ([`SessionFlight::may_save`](nova_view::flight::SessionFlight::may_save)): not on going back to the menu, not when the
//! app quits, and not when a save comes due, which is dropped. The
//! original saves only on creation, landing and pod rescue
//! (`_SavePilotFile` @0x76876), so a dead pilot reopens as it was at its
//! last landing. A Strict Play pilot whose ship is breaking up or
//! destroyed, outside the escape pod, is deleted instead as play ends
//! ([`SessionFlight::deletes_pilot_on_ending_play`](nova_view::flight::SessionFlight::deletes_pilot_on_ending_play)): on the death's return
//! to the menu, on Escape's put-away and when the app quits, as
//! `_EnterGameFromMainScreen` deletes it (@0x1a4ab-0x1a4e1); a failed
//! delete is a warning. Once the player's ship has exploded, the scene runs on
//! through the hold; when it expires, the router shows the `dësc` of
//! each of a sabotaged player's outfits over the stopped flight, one
//! after another, and begins flight's fade to black as the last closes,
//! or with none on the same tick. When the fade is over the router
//! returns to the main menu as it appears (below), holding the dead
//! pilot as Escape's return holds a live one (Enter Ship refuses it, New
//! Pilot asks #8), or, without a menu, drops flight and returns to the
//! side flight was entered from, which fades in from black over
//! [`DEATH_FADE_FOR`](nova_view::fade::DEATH_FADE_FOR), taking input under the fade.
//!
//! The main menu appears as the original's main screen does
//! ([`Appear`](nova_view::menu::appear::Appear), see `appear`): at
//! start-up it fades in from black over a second, its slides playing
//! after, the clicks and plain keys made meanwhile dropped and its
//! Command keys and the window's close kept for when it ends; on each
//! return from play, by the engine's reading of `main_screen_return_fade`,
//! the screen stays black while the primary button is held and through
//! the second's fade, the main screen appearing when it ends, the keys
//! typed meanwhile reaching it after and its pointer events dropped, as
//! `_EnterGameFromMainScreen` (0x1a471) brings the main screen back. By
//! the rule's other reading the return fades in over the menu, which
//! takes input under the fade.
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
//! click sets the destination and Tab or `\` cycles the hyperspace
//! selection; Escape, Return (or M) closes it, back into flight.
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
//! I on a person's offer over flight opens it over the offer, as
//! `_OfferOneMission`'s item 7 does whoever offers (@0xa32b6): with no
//! map (`_DoMissionInfoDialog` @0x9e3c6), the keys let go first
//! (@0xa32f4-0xa3344), and the offer's `dësc` loaded again as it closes
//! (@0xa335c). With no mission shown (the count @0xa32c1-0xa32ee) it
//! does nothing, and says nothing.
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
//! Nova's own until another is given; by the other reading of the
//! `pd_hostility` rule, their point defence engages only the missiles the
//! router's point-defence rule calls hostile
//! ([`AppScreen::with_point_defence_rule`]), by governments' allegiance
//! until another is given, and by default every missile fired at them;
//! the player's crimes are judged by the
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
//! through [`Screen::take_diagnostics`](nova_view::Screen::take_diagnostics).
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
//! capture that awaits its assignment opens the assignment dialog over
//! the plunder dialog, which stays drawn under it, and the choice goes
//! through flight. Use As My Ship then opens the rename, the Text Input
//! dialog asking the ship's new name, whose OK or Cancel goes through
//! flight (see the `capture_naming` module); Use As Escort, or the
//! rename answered, closes the plunder dialog. The pilot is saved after
//! the input that captured, assigned or named, as after a change in the
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
//! reported once through [`Screen::take_prefs`](nova_view::Screen::take_prefs) for the app to play
//! and save. Flight follows the Hyperspace Effects and Smoke Trails
//! preferences: it is built with them, and a change reaches a flight
//! already built. Weapon Effects, as in the original, takes effect at the
//! next launch: every flight is built with the value the game launched
//! with, and the dialog draws the original's relaunch notice while the box
//! differs from it; by `nova_sim::RuleKey::WeaponEffectsNextLaunch`'s other
//! reading it follows the box at once, as Smoke Trails does. OK, Return or
//! Escape closes it.
//! Its close redraws nothing of the status bar, as the original's menu
//! command `pref` calls `_DoPrefsDialog` alone (@0xcccf1); only the `pref`
//! Apple Event's handler redraws (@0x56195).
//!
//! # Display shapes
//!
//! Flight, and every dialog or map over it, is drawn widened to the
//! window's aspect ratio ([`ViewShape::Widened`](nova_view::ViewShape));
//! the main menu, the developer's sides and the spaceport, with anything
//! over it, the bar's course map included, stay classic, 1024 x 768. The
//! router keeps the widened size the app tells it and hands it to every
//! flight. Over widened flight, the dialogs (About, Preferences, player
//! info, haggle, comm, the mission texts and info, the offer, plunder and
//! assignment) are drawn in their 1024 x 768 frame centred in the view
//! ([`classic_offset`](nova_view::view_shape::classic_offset)), where the
//! original's `_CenterWindow` puts a dialog in a window that wide, and take
//! the pointer moved back to match. Flight places its own course map so.
//!
//! The router reports every live screen's sounds through
//! [`Screen::take_sounds`](nova_view::Screen::take_sounds), and what it shows through
//! [`Screen::now_showing`](nova_view::Screen::now_showing). The spaceport and the two dialogs are dropped
//! as they close, so their sounds (the click that closed them) are kept
//! first.

use std::rc::Rc;
use std::time::Duration;

use nova_data::GameData;
use nova_sim::ship_layer::SlotSprites;
use nova_sim::story::sprite_pool::StellarSpritePool;
use nova_sim::{
    Behaviour, BoardingRule, ControlBits, DisableRule, HailOptions, HireTerms, LegalCode,
    PersonRules, PilotKeeper, PilotStore, PointDefenceRule, Rulebook,
};
use nova_sim::{KeptBerths, LastRefresh};
pub use nova_view::Showing;
use nova_view::fade::ScreenFade;
use nova_view::flight::{FlightOverlays, FlightView, SharedChance};
use nova_view::menu::appear::Appear;
use nova_view::menu::intro::CharacterIntro;
use nova_view::menu::{MainMenu, MenuChoice, PilotList};
use nova_view::movie::Movies;
use nova_view::ships::ShipBrowser;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::offer::OfferDialog;
use nova_view::text::TextMetrics;
use nova_view::ui::mission_dialogs::MissionDialogs;
use nova_view::ui::new_pilot::NewPilotDialog;
use nova_view::ui::note::NoteDialog;
use nova_view::ui::player_info::PlayerInfoScreen;
use nova_view::ui::text_input::TextInputDialog;
use nova_view::ui::{
    AssignmentDialog, CommDialog, DescDialog, DialogResources, HaggleDialog, PlunderDialog,
    PrefsDialog, YesNoDialog,
};
use nova_view::{Color, Input, Key, MenuCommand, Navigator, Point, Prefs, Sound};
use nova_view::{FullscreenKind, Scaling};

mod appear;
mod build;
mod capture_naming;
mod christening;
mod entry;
mod escort_dialog;
mod flight;
mod game;
mod launch;
mod menu;
mod menu_commands;
mod overlays;
mod pilot_panel;
mod port_dialogs;
mod quit_question;
mod routing;
mod screen_impl;
mod ship_name_dialog;

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

/// The key a menu command's Command chord presses: Q for Quit (Command-Q,
/// see [`AppScreen::route`]) and [`PREFERENCES_KEY`] for Preferences…;
/// About has none. The one table of the game's menu chords: the router's
/// Command-Q, its Preferences key and the entry's held chords read it
/// ([`chord_command`]), and the macOS menu bar's key equivalents are
/// checked against it (`platform::menu_bar`).
#[must_use]
pub const fn menu_chord(command: MenuCommand) -> Option<Key> {
    match command {
        MenuCommand::Quit => Some(Key::Char('q')),
        MenuCommand::Preferences => Some(PREFERENCES_KEY),
        MenuCommand::About => None,
    }
}

/// The menu command whose chord presses `key` ([`menu_chord`]), if any.
#[must_use]
pub fn chord_command(key: Key) -> Option<MenuCommand> {
    MenuCommand::ALL
        .into_iter()
        .find(|&command| menu_chord(command) == Some(key))
}

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

/// What a main-menu key does once its click has played, or a click on
/// a button at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuAction {
    /// One of the six buttons, by its key (N, O, Q, E, P or A) or a
    /// click: the key and the click reach the same arm of
    /// `_MainScreenChoice`, so the same result.
    Choice(MenuChoice),
    /// X: `dësc` 32766.
    Text,
}

/// What the main menu waits on, as `_MainScreenKey` @0x1aba0 spins
/// before its choice runs.
///
/// The original reads no event during the spin: what arrives meanwhile
/// stays in the OS queue for whatever the choice opens. So while the
/// menu waits, the router queues every input (and each letting go of the
/// pointer or the keys, and each close of the window, in its place) and
/// replays them in order once
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
    /// action runs once it has ([`Screen::keyed_click_ended`](nova_view::Screen::keyed_click_ended)).
    Click(MenuAction),
}

/// An input held while the main menu waits, or a letting go in its
/// place.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Held {
    Input(Input),
    CancelPointer,
    ReleaseKeys,
    /// The window's close: the original's close-box click waits in the
    /// queue for what the choice leaves showing (see `quit_question`).
    Close,
}

/// Every screen the app can show, and which side it is showing. The router
/// forwards input, ticks and drawing to the side shown, switches sides on
/// a Tab press (not on its key repeats) and decides what an Escape press
/// does.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // Each a setting or a state the router keeps.
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
    /// Flight, once entered, reading the same game data: the flight
    /// shown, or hidden behind the developer's sides.
    flight: Option<FlightView<Rc<GameData>>>,
    /// The game in progress the main menu holds, not flying: a pilot
    /// created or opened, or back from play (see `game`).
    game: Option<FlightView<Rc<GameData>>>,
    /// The ship slots' draw-layer and cloned-frame history the last
    /// flight put away left, which each new flight starts from: the
    /// original's sprites outlive a pilot, `_InitSprites` running once at
    /// launch (see [`nova_sim::ship_layer`]).
    slot_sprites: SlotSprites,
    /// The stellar sprite pool the last flight put away left, which each
    /// new flight carries on: the original never clears a stellar's sprite
    /// pointer, so a stellar keeps the slot it last took for the run (see
    /// [`nova_sim::story::sprite_pool`]).
    stellar_sprites: StellarSpritePool,
    /// What the berths of the last flight put away keep, which each new
    /// flight's traffic starts from: the original's ship table outlives a
    /// pilot, `_InitArrays` taking it once at launch (see
    /// [`nova_sim::traffic::kept_berths`]). Empty at launch, as the
    /// table's fresh pages read 0.
    kept_berths: KeptBerths,
    /// What the galaxy refreshes of the last flight put away left
    /// hidden, which a new pilot's neighbour pass reads when no game is
    /// held ([`AppScreen::run_refresh`]); `None` at launch, when the
    /// launch's own refresh is read.
    last_refresh: Option<LastRefresh>,
    /// The comm dialog's release flag the last flight put away left,
    /// which each new flight starts from: the original's
    /// `_releaseCommEscort` (the byte at 0x31b250, behind 0xdef2c) is an
    /// app-wide global no pilot load touches (see
    /// [`nova_sim::CarryOver::release_comm_escort`]). Clear at launch, as
    /// its BSS reads 0; it is never saved, so a quit drops it.
    release_comm_escort: bool,
    /// The course map's zoom the last flight put away left, which each
    /// new flight's map opens at: the original's `_mapZoom` is set once,
    /// by its data, and never reset by a new or loaded pilot.
    course_map_zoom: Option<f64>,
    /// Whether the course map shows the borders: the player's
    /// `ShowMapBorders` as the run started, then as the last flight put
    /// away left it, so a change made docked lasts for the run.
    map_borders: bool,
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
    /// The Weapon Effects the game launched with, the one every flight
    /// takes while a change waits for the next launch
    /// (`nova_sim::RuleKey::WeaponEffectsNextLaunch`'s engine reading):
    /// set by [`AppScreen::with_prefs`], which `main` calls once at
    /// startup.
    launch_weapon_effects: bool,
    /// The Preferences dialog, while it is open.
    preferences: Option<PrefsDialog>,
    /// The preferences chosen since they were last taken, if they changed.
    prefs_change: Option<Prefs>,
    /// The fullscreen kind picked in the Preferences dialog since it was
    /// last taken.
    display_choice: Option<FullscreenKind>,
    /// The Scaling list, as offered at startup (native and Super-xBR
    /// before), for every Preferences dialog.
    scaling: Vec<Scaling>,
    /// The main menu and where pilots are kept, once given.
    menu: Option<Menu>,
    /// The New Pilot dialog, while it is open over the main menu.
    new_pilot: Option<NewPilotDialog>,
    /// The new pilot's ship christening, while it is open over the main
    /// menu.
    christening: Option<christening::Christening>,
    /// The saved pilots' list, while it is open over the main menu.
    open_pilot: Option<PilotList>,
    /// The note that an opened pilot's load fell back (`STR#` 140 #52),
    /// while it is open over the main menu, its flight built behind it.
    load_note: Option<NoteDialog>,
    /// Whether the #52 note open over the main menu hides the game held
    /// from the pilot panel: no game was held before the Open (see
    /// `pilot_panel`).
    note_hides_game: bool,
    /// The question asked over the main menu, and what for, while it is
    /// open: the window's close's (`STR#` 140 #10) or New Pilot's over a
    /// game in progress (#8).
    question: Option<(quit_question::Question, YesNoDialog)>,
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
    /// Use As My Ship's rename, the Text Input dialog over the plunder
    /// dialog, while the ship captured awaits its name.
    capture_naming: Option<TextInputDialog>,
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
    /// The escort dialog, while a fleet escort hailed is talked to.
    escort_dialog: Option<nova_view::ui::escort_dialog::EscortDialog>,
    /// Whether the port's Leave waits for the report of the escorts sold
    /// and upgraded, shown over the spaceport, to close (see the
    /// `escort_dialog` module).
    leave_held: bool,
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
    /// another is given ([`AppScreen::with_movie_player`], which `main`
    /// does with the FFmpeg decoder), on as the preferences say.
    movies: Movies,
    /// Whether the player has asked the system for keyboard access to
    /// every control, asked each time Preferences opens:
    /// [`NoKeyboardAccessSetting`](crate::app::NoKeyboardAccessSetting)'s
    /// unless another is given ([`AppScreen::with_keyboard_access`]).
    keyboard_access: Rc<dyn crate::app::KeyboardAccess>,
    /// The fade back from black over the developer's side shown after a
    /// death without a main menu, while it plays.
    fade_in: Option<ScreenFade>,
    /// How the main menu appears, at start-up and on a return from play,
    /// while it does.
    appear: Option<Appear>,
    /// Play being entered from the main screen, while it is (see
    /// `entry`).
    entry: Option<entry::Entering>,
    /// The intro of the last pilot New Pilot made in this run, the
    /// original's intro globals (`_GetCharResourceIntroIDs` @0x193ad);
    /// none at launch (`_InitMainScreen` @0x16101-0x1612d).
    new_pilot_intro: Option<CharacterIntro>,
    /// Whether the primary mouse button is down, as the last pointer
    /// input routed or cancel put it: a return from play waits for it to
    /// be let go of (`StillDown`).
    button_down: bool,
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
    /// Whether a quit is due, for [`Screen::take_quit`](nova_view::Screen::take_quit).
    quit_due: bool,
    /// Whether double time is on: Caps Lock's presses toggle it, on any
    /// side, and every flight is told. Never saved, as the original's
    /// `_doubleTime` is not.
    double_time: bool,
    /// The widened view's size, as the app last told it
    /// ([`Screen::set_widened_size`](nova_view::Screen::set_widened_size)):
    /// every flight, built or to be built, draws at it.
    widened: (f32, f32),
    /// Where the pointer is, in the classic 1024 x 768 frame, as the last
    /// pointer input routed put it (`None` once it has left the game).
    /// The main menu is told it before each of its ticks, wherever the
    /// moves went meanwhile ([`AppScreen::tell_menu_pointer`]).
    pointer: Option<Point>,
}

/// Nova's own 1024 x 768, which every screen but flight draws in.
const CLASSIC_VIEW: (f32, f32) = (
    nova_view::view_shape::CLASSIC_SIZE.0 as f32,
    nova_view::view_shape::CLASSIC_SIZE.1 as f32,
);

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

#[cfg(test)]
pub(super) mod tests;
