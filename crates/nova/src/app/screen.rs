//! The screen router: every screen the app can show, as one [`Screen`].
//!
//! # The main menu and pilots
//!
//! The game opens on the main menu ([`AppScreen::with_pilots`]): New
//! Pilot asks for a name in "Create a new pilot:" (the interface file's,
//! or a built-in one), refusing a name already saved, then creates the
//! pilot from the first `chär`, saves it and flies it. Open Pilot lists
//! the saved pilots and flies the one chosen, from the spaceport of the
//! stellar it last landed on when it is docked. Quit quits. Escape in
//! flight goes back to the menu, saving the pilot and putting it away.
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
//! L in flight requests clearance at a stellar, and a second L, over it,
//! lands and shows its spaceport ([`SpaceportView`]), laid out by the
//! interface file's "Spaceport" dialog when the router has dialogs;
//! without them the spaceport says why. The spaceport takes every input:
//! Leave, Return and Escape take off, back into flight at the stellar,
//! and Tab, F and I do nothing.
//!
//! At a trade center, the spaceport's Trade Center opens the session's
//! exchange, laid out by the interface file's "Trade" dialog. There B buys
//! and S sells a ton of the selected good, and with Alt held the most;
//! the router makes each trade through the session, hands the exchange as
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
//! and the escorts' standing orders are reset, or kept, on entering a
//! system as the router says ([`AppScreen::with_escort_orders`]), reset
//! (the engine's) until another is given. Each flight hires escorts by
//! the router's hiring rules ([`AppScreen::with_hire_require`],
//! [`AppScreen::with_take_off_pay`], [`AppScreen::with_escort_wage`],
//! [`AppScreen::with_hire_terms`], [`AppScreen::with_control_bits`]),
//! the engine's and Nova's until others are given. Each flight's
//! outfitter and shipyard read `BuyRandom` as the router says
//! ([`AppScreen::with_buy_random`]), the engine's until another is given.
//! Each flight's held tribbles and perishable `jünk` grow and decay as
//! the router says ([`AppScreen::with_junk_flags`]), the engine's until
//! another is given. Each flight's outfitter refuses a launcher's sale
//! for its ammunition as the router says
//! ([`AppScreen::with_launcher_sale`]), the engine's until another is
//! given. Each flight's `ModType` 27 outfits raise their targets' `Max`
//! as the router says ([`AppScreen::with_raised_max`]), the engine's
//! until another is given. Each flight's exchange prices an active
//! `öops` event's commodity as the router says
//! ([`AppScreen::with_event_price`]), the engine's until another is given.
//! Each flight's ship purchase keeps the cargo as the router says
//! ([`AppScreen::with_purchase_cargo`]), the engine's until another is
//! given. Each flight's exchange trades a `jünk` of negative or zero
//! price as the router says ([`AppScreen::with_junk_price`]), and trades
//! each `jünk` row the ways it says ([`AppScreen::with_junk_trade`]), and
//! moves as many tons a plain trade as it says
//! ([`AppScreen::with_trade_lot`]), the engine's until others are given. Each flight's outfitter sells a map
//! or clean-record outfit as the router says
//! ([`AppScreen::with_outfit_limit`]), the engine's until another is
//! given; each time the spaceport's outfitter opens, the flight is told
//! ([`FlightView::open_outfitter`]) and the outfitter shown afresh.
//! Each flight's persons
//! appear as the router's rules say ([`AppScreen::with_person_rules`])
//! and say their comm quotes as it says ([`AppScreen::with_comm_quote`]),
//! the engine's and Nova's until others are given;
//! [`AppScreen::with_rulebook`] sets every rule from one rulebook.
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
use nova_sim::{
    Allegiance, Behaviour, BoardingRule, ControlBits, DisableRule, HailOptions, HailView,
    HireTerms, LegalCode, NoControlBits, NovaAi, NovaBoarding, NovaDisable, NovaHire, NovaLaw,
    NovaPersons, PersonRules, Pilot, PilotKeeper, PilotStore, PointDefenceRule, RuleKey,
    RuleSource, Rulebook, Take, Taken, pilot_key,
};
pub use nova_view::Showing;
use nova_view::flight::{FlightView, SharedChance};
use nova_view::galaxy::GalaxyMap;
use nova_view::menu::{MainMenu, MenuChoice, PilotList, PilotListOutcome};
use nova_view::ships::ShipBrowser;
use nova_view::spaceport::SpaceportView;
use nova_view::spaceport::bar::BAR_DIALOG;
use nova_view::spaceport::layout::SPACEPORT_DIALOG;
use nova_view::spaceport::outfitter::OUTFIT_DIALOG;
use nova_view::spaceport::shipyard::{SHIP_INFO_DIALOG, SHIPYARD_DIALOG};
use nova_view::spaceport::trade::TRADE_DIALOG;
use nova_view::spaceport::{OutfitterCatalog, ShipyardCatalog};
use nova_view::system::SystemView;
use nova_view::text::TextMetrics;
use nova_view::ui::comm::{COMM_DIALOG, HAGGLE_DIALOG};
use nova_view::ui::desc::DESC_DIALOG;
use nova_view::ui::new_pilot::{NAME_TAKEN, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome};
use nova_view::ui::plunder::{ASSIGNMENT_DIALOG, PLUNDER_DIALOG};
use nova_view::ui::prefs::PREFS_DIALOG;
use nova_view::ui::text_input::TEXT_INPUT_DIALOG;
use nova_view::ui::{
    AssignmentDialog, CommDialog, CommPress, DescDialog, DescriptionSource, DialogResources,
    HaggleDialog, PlunderDialog, PlunderShown, PrefsDialog,
};
use nova_view::{
    Color, Diagnostic, DrawList, Input, Key, Navigator, Point, Prefs, Screen, ScreenAction, Sound,
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

/// The About text's `dësc`.
pub const ABOUT_TEXT: i16 = 32767;

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
    /// What dialogs are built from, once given.
    dialogs: Option<Dialogs>,
    /// The About dialog, while it is open.
    about: Option<DescDialog>,
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
    /// Whether each flight's escorts' standing orders are reset on
    /// entering a system.
    escort_orders: RuleSource,
    /// What a fighter each flight's player launches does first.
    fighter_launch: RuleSource,
    /// What becomes of each flight's fighters out as the player leaves a
    /// system.
    fighter_recall: RuleSource,
    /// Whether an unmet `Require` refuses a hire in each flight.
    hire_require: RuleSource,
    /// How `BuyRandom` reads in each flight.
    buy_random: RuleSource,
    /// How held tribbles and perishable `jünk` grow and decay in each
    /// flight.
    junk_flags: RuleSource,
    /// When each flight's outfitter refuses a launcher's sale for its
    /// ammunition.
    launcher_sale: RuleSource,
    /// How each flight's `ModType` 27 outfits raise their targets' `Max`.
    raised_max: RuleSource,
    /// How each flight's exchange prices an active `öops` event's
    /// commodity.
    event_price: RuleSource,
    /// What cargo each flight's ship purchase keeps.
    purchase_cargo: RuleSource,
    /// How each flight's exchange trades a `jünk` of negative or zero
    /// price.
    junk_price: RuleSource,
    /// Which ways each flight's exchange trades a `jünk` row.
    junk_trade: RuleSource,
    /// How many tons a plain trade on each flight's exchange moves.
    trade_lot: RuleSource,
    /// Whether each flight's outfitter sells a map or clean-record outfit
    /// only once an opening.
    outfit_limit: RuleSource,
    /// Whether each take-off pays each flight's hired escorts a day.
    take_off_pay: RuleSource,
    /// Which wage each flight's hired escorts are paid.
    escort_wage: RuleSource,
    /// The fee and wage of a hire in each flight.
    hire_terms: Rc<dyn HireTerms>,
    /// How persons appear in each flight.
    person_rules: Rc<dyn PersonRules>,
    /// When each flight's persons say their comm quotes.
    comm_quote: RuleSource,
    /// The control-bit test a ship for hire's `Availability` goes
    /// through in each flight.
    control_bits: Rc<dyn ControlBits>,
    /// The comm dialog, while a hail is under way.
    comm: Option<CommDialog>,
    /// The haggle dialog, over the comm dialog, while a price is asked.
    haggle: Option<HaggleDialog>,
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
            dialogs: None,
            about: None,
            spaceport: None,
            sounds: Vec::new(),
            prefs: Prefs::default(),
            preferences: None,
            prefs_change: None,
            menu: None,
            new_pilot: None,
            open_pilot: None,
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
            escort_orders: RuleSource::Engine,
            fighter_launch: RuleSource::Engine,
            fighter_recall: RuleSource::Engine,
            hire_require: RuleSource::Engine,
            buy_random: RuleSource::Engine,
            junk_flags: RuleSource::Engine,
            launcher_sale: RuleSource::Engine,
            raised_max: RuleSource::Engine,
            event_price: RuleSource::Engine,
            purchase_cargo: RuleSource::Engine,
            junk_price: RuleSource::Engine,
            junk_trade: RuleSource::Engine,
            trade_lot: RuleSource::Engine,
            outfit_limit: RuleSource::Engine,
            take_off_pay: RuleSource::Engine,
            escort_wage: RuleSource::Engine,
            hire_terms: Rc::new(NovaHire::default()),
            control_bits: Rc::new(NoControlBits),
            person_rules: Rc::new(NovaPersons::default()),
            comm_quote: RuleSource::Engine,
            comm: None,
            haggle: None,
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

    /// The router with each flight's escorts' standing orders reset, or
    /// kept, on entering a system as `source` says
    /// ([`FlightView::with_escort_orders`]); the engine's reset until
    /// another is given.
    #[must_use]
    pub fn with_escort_orders(self, source: RuleSource) -> Self {
        Self {
            escort_orders: source,
            ..self
        }
    }

    /// The router with the fighters each flight's player launches doing
    /// first as `source` says ([`FlightView::with_fighter_launch`]); the
    /// engine's until another is given.
    #[must_use]
    pub fn with_fighter_launch(self, source: RuleSource) -> Self {
        Self {
            fighter_launch: source,
            ..self
        }
    }

    /// The router with each flight's fighters out, as the player leaves a
    /// system, following `source` ([`FlightView::with_fighter_recall`]);
    /// the engine's until another is given.
    #[must_use]
    pub fn with_fighter_recall(self, source: RuleSource) -> Self {
        Self {
            fighter_recall: source,
            ..self
        }
    }

    /// The router with an unmet `Require` refusing a hire in each flight,
    /// or not, as `source` says ([`FlightView::with_hire_require`]); the
    /// engine's (not) until another is given.
    #[must_use]
    pub fn with_hire_require(self, source: RuleSource) -> Self {
        Self {
            hire_require: source,
            ..self
        }
    }

    /// The router with `BuyRandom` read in each flight as `source` says
    /// ([`FlightView::with_buy_random`]); the engine's until another is
    /// given.
    #[must_use]
    pub fn with_buy_random(self, source: RuleSource) -> Self {
        Self {
            buy_random: source,
            ..self
        }
    }

    /// The router with held tribbles and perishable `jünk` growing and
    /// decaying in each flight as `source` says
    /// ([`FlightView::with_junk_flags`]); the engine's until another is
    /// given.
    #[must_use]
    pub fn with_junk_flags(self, source: RuleSource) -> Self {
        Self {
            junk_flags: source,
            ..self
        }
    }

    /// The router with each flight's outfitter refusing a launcher's sale
    /// for its ammunition as `source` says
    /// ([`FlightView::with_launcher_sale`]); the engine's until another is
    /// given.
    #[must_use]
    pub fn with_launcher_sale(self, source: RuleSource) -> Self {
        Self {
            launcher_sale: source,
            ..self
        }
    }

    /// The router with each flight's `ModType` 27 outfits raising their
    /// targets' `Max` as `source` says ([`FlightView::with_raised_max`]);
    /// the engine's until another is given.
    #[must_use]
    pub fn with_raised_max(self, source: RuleSource) -> Self {
        Self {
            raised_max: source,
            ..self
        }
    }

    /// The router with each flight's exchange pricing an active `öops`
    /// event's commodity as `source` says
    /// ([`FlightView::with_event_price`]); the engine's until another is
    /// given.
    #[must_use]
    pub fn with_event_price(self, source: RuleSource) -> Self {
        Self {
            event_price: source,
            ..self
        }
    }

    /// The router with each flight's ship purchase keeping the cargo as
    /// `source` says ([`FlightView::with_purchase_cargo`]); the engine's
    /// until another is given.
    #[must_use]
    pub fn with_purchase_cargo(self, source: RuleSource) -> Self {
        Self {
            purchase_cargo: source,
            ..self
        }
    }

    /// The router with each flight's exchange trading a `jünk` of
    /// negative or zero price as `source` says
    /// ([`FlightView::with_junk_price`]); the engine's until another is
    /// given.
    #[must_use]
    pub fn with_junk_price(self, source: RuleSource) -> Self {
        Self {
            junk_price: source,
            ..self
        }
    }

    /// The router with each flight's exchange trading each `jünk` row the
    /// ways `source` says ([`FlightView::with_junk_trade`]); the engine's
    /// until another is given.
    #[must_use]
    pub fn with_junk_trade(self, source: RuleSource) -> Self {
        Self {
            junk_trade: source,
            ..self
        }
    }

    /// The router with a plain trade on each flight's exchange moving as
    /// many tons as `source` says ([`FlightView::with_trade_lot`]); the
    /// engine's (up to 10 a click) until another is given.
    #[must_use]
    pub fn with_trade_lot(self, source: RuleSource) -> Self {
        Self {
            trade_lot: source,
            ..self
        }
    }

    /// The router with each flight's outfitter selling a map or
    /// clean-record outfit as `source` says
    /// ([`FlightView::with_outfit_limit`]); the engine's (once an opening)
    /// until another is given.
    #[must_use]
    pub fn with_outfit_limit(self, source: RuleSource) -> Self {
        Self {
            outfit_limit: source,
            ..self
        }
    }

    /// The router with each take-off paying each flight's hired escorts a
    /// day's wages, or not, as `source` says
    /// ([`FlightView::with_take_off_pay`]); the engine's (it does) until
    /// another is given.
    #[must_use]
    pub fn with_take_off_pay(self, source: RuleSource) -> Self {
        Self {
            take_off_pay: source,
            ..self
        }
    }

    /// The router with each flight's hired escorts paid the wage `source`
    /// says ([`FlightView::with_escort_wage`]); the engine's until
    /// another is given.
    #[must_use]
    pub fn with_escort_wage(self, source: RuleSource) -> Self {
        Self {
            escort_wage: source,
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

    /// The router with `bits` testing a ship for hire's `Availability` in
    /// each flight ([`FlightView::with_control_bits`]); none hold false
    /// until others are given.
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

    /// The router with each flight's persons saying their comm quotes as
    /// `source` says ([`FlightView::with_comm_quote`]); the engine's until
    /// another is given.
    #[must_use]
    pub fn with_comm_quote(self, source: RuleSource) -> Self {
        Self {
            comm_quote: source,
            ..self
        }
    }

    /// The router with Nova's rules, each disputed one as `rulebook`
    /// chooses: the NPCs' behaviour, disabling, point defence, the law,
    /// boarding, hailing, the escorts' and fighters' rules, hiring, the
    /// shops' `BuyRandom`, the tribbles and perishables, the cargo a ship
    /// purchase keeps, and the persons' rules. This is the edge where
    /// every [`RuleKey`] meets its setting.
    #[must_use]
    pub fn with_rulebook(self, rulebook: &Rulebook) -> Self {
        self.with_behaviour(Rc::new(NovaAi::from_rulebook(rulebook)))
            .with_disable_rule(Rc::new(NovaDisable))
            .with_point_defence_rule(Rc::new(Allegiance))
            .with_law(Rc::new(NovaLaw::from_rulebook(rulebook)))
            .with_boarding_rule(Rc::new(NovaBoarding::from_rulebook(rulebook)))
            .with_hail_options(HailOptions::nova(rulebook))
            .with_escort_orders(rulebook.source_for(RuleKey::EscortOrders))
            .with_fighter_launch(rulebook.source_for(RuleKey::FighterLaunch))
            .with_fighter_recall(rulebook.source_for(RuleKey::FighterRecall))
            .with_hire_require(rulebook.source_for(RuleKey::HireRequire))
            .with_buy_random(rulebook.source_for(RuleKey::BuyRandom))
            .with_junk_flags(rulebook.source_for(RuleKey::JunkFlags))
            .with_launcher_sale(rulebook.source_for(RuleKey::LauncherSale))
            .with_raised_max(rulebook.source_for(RuleKey::RaisedMax))
            .with_event_price(rulebook.source_for(RuleKey::EventPrice))
            .with_purchase_cargo(rulebook.source_for(RuleKey::PurchaseCargo))
            .with_junk_price(rulebook.source_for(RuleKey::JunkPrice))
            .with_junk_trade(rulebook.source_for(RuleKey::JunkTrade))
            .with_trade_lot(rulebook.source_for(RuleKey::TradeLot))
            .with_outfit_limit(rulebook.source_for(RuleKey::OutfitLimit))
            .with_take_off_pay(rulebook.source_for(RuleKey::TakeOffPay))
            .with_escort_wage(rulebook.source_for(RuleKey::EscortWage))
            .with_hire_terms(Rc::new(NovaHire::from_rulebook(rulebook)))
            .with_control_bits(Rc::new(NoControlBits))
            .with_person_rules(Rc::new(NovaPersons::from_rulebook(rulebook)))
            .with_comm_quote(rulebook.source_for(RuleKey::CommQuote))
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
    /// crimes as its rules say, and laying out its HUD's text with the
    /// router's metrics, if it has any.
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
            .with_escort_orders(self.escort_orders)
            .with_fighter_launch(self.fighter_launch)
            .with_fighter_recall(self.fighter_recall)
            .with_hire_require(self.hire_require)
            .with_buy_random(self.buy_random)
            .with_junk_flags(self.junk_flags)
            .with_launcher_sale(self.launcher_sale)
            .with_raised_max(self.raised_max)
            .with_event_price(self.event_price)
            .with_purchase_cargo(self.purchase_cargo)
            .with_junk_price(self.junk_price)
            .with_junk_trade(self.junk_trade)
            .with_trade_lot(self.trade_lot)
            .with_outfit_limit(self.outfit_limit)
            .with_take_off_pay(self.take_off_pay)
            .with_escort_wage(self.escort_wage)
            .with_hire_terms(Rc::clone(&self.hire_terms))
            .with_control_bits(Rc::clone(&self.control_bits))
            .with_person_rules(Rc::clone(&self.person_rules))
            .with_comm_quote(self.comm_quote)
            .with_hyperspace_effects(self.prefs.hyperspace_effects);
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

    /// Changes the pilot with `change` while the ship is landed, as a
    /// spaceport screen does, and says whether it did; the pilot is saved
    /// after the next input or tick.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.flight
            .as_mut()
            .is_some_and(|flight| flight.transact(change))
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
        Self { prefs, ..self }
    }

    /// The player's preferences, as last chosen.
    #[must_use]
    pub fn prefs(&self) -> Prefs {
        self.prefs
    }

    /// The About dialog, while it is open.
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
            return Showing::About;
        }
        if self.preferences.is_some() {
            return Showing::Preferences;
        }
        if self.haggle.is_some() {
            return Showing::Haggle;
        }
        if self.comm.is_some() {
            return Showing::Comm;
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
        match self.side {
            Side::MainMenu => Showing::MainMenu,
            Side::Ships => Showing::ShipBrowser,
            Side::Galaxy if self.galaxy.system().is_some() => Showing::System,
            Side::Galaxy => Showing::GalaxyMap,
            Side::Flight if self.flight.as_ref().is_some_and(FlightView::map_open) => {
                Showing::FlightMap
            }
            Side::Flight => Showing::Flight,
            Side::Spaceport => Showing::Spaceport,
        }
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

    fn shown(&self) -> &dyn Screen {
        match self.side {
            Side::MainMenu => &self.menu.as_ref().expect(MENU).screen,
            Side::Ships => &self.ships,
            Side::Galaxy => &self.galaxy,
            Side::Flight => self.flight.as_ref().expect(ENTERED),
            Side::Spaceport => self.spaceport.as_ref().expect(LANDED),
        }
    }

    fn shown_mut(&mut self) -> &mut dyn Screen {
        match self.side {
            Side::MainMenu => &mut self.menu.as_mut().expect(MENU).screen,
            Side::Ships => &mut self.ships,
            Side::Galaxy => &mut self.galaxy,
            Side::Flight => self.flight.as_mut().expect(ENTERED),
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
        let Some(dialogs) = &self.dialogs else {
            return;
        };
        match about_dialog(dialogs, &self.data) {
            Ok(dialog) => {
                let below = self.shown_mut();
                below.cancel_pointer();
                below.release_keys();
                self.about = Some(dialog);
            }
            Err(reason) => eprintln!("nova: cannot show the About text: {reason}"),
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

    /// The Preferences dialog's input: each change is kept, to be taken
    /// once, and the dialog closes once OK is activated.
    fn preferences_input(&mut self, input: &Input) -> ScreenAction {
        if let Some(dialog) = &mut self.preferences {
            dialog.input(input);
            if let Some(prefs) = dialog.take_change() {
                self.prefs = prefs;
                self.prefs_change = Some(prefs);
                if let Some(flight) = &mut self.flight {
                    flight.set_hyperspace_effects(prefs.hyperspace_effects);
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
    /// Preferences dialog, the New Pilot dialog or the saved pilots' list.
    fn overlay_mut(&mut self) -> Option<&mut dyn Screen> {
        if let Some(about) = &mut self.about {
            return Some(about);
        }
        if let Some(dialog) = &mut self.preferences {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.haggle {
            return Some(dialog);
        }
        if let Some(dialog) = &mut self.comm {
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
        self.open_pilot.as_mut().map(|list| list as &mut dyn Screen)
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
    /// (where Tab selects the navigation target). When it lands, the
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
                    self.switch_to(Side::MainMenu);
                    self.save_pilot();
                    self.flight = None;
                    self.spaceport = None;
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
        ScreenAction::None
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
    /// exchange and outfitter when it has them.
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
            spaceport = spaceport.with_trade(trade, market);
        }
        if let Some(outfitter) = self.flight.as_mut().and_then(FlightView::outfitter) {
            let template = template(OUTFIT_DIALOG).map(|(template, _)| template);
            let art: Rc<dyn OutfitterCatalog> = Rc::clone(&self.data) as Rc<dyn OutfitterCatalog>;
            spaceport = spaceport.with_outfitter(template, outfitter, art);
        }
        let template_of = |id| template(id).map(|(template, _)| template);
        if let Some(shipyard) = self.flight.as_mut().and_then(FlightView::shipyard) {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport.with_shipyard(
                template_of(SHIPYARD_DIALOG),
                template_of(SHIP_INFO_DIALOG),
                template_of(TEXT_INPUT_DIALOG),
                shipyard,
                art,
            );
        }
        if let Some(pilot) = self.flight.as_ref().and_then(FlightView::pilot) {
            let art: Rc<dyn ShipyardCatalog> = Rc::clone(&self.data) as Rc<dyn ShipyardCatalog>;
            spaceport = spaceport.with_bar(
                template_of(BAR_DIALOG),
                template_of(SHIPYARD_DIALOG),
                template_of(SHIP_INFO_DIALOG),
                art,
                pilot.escort_count() < MAX_ESCORTS,
            );
        }
        self.spaceport = Some(spaceport);
        self.switch_to(Side::Spaceport);
    }

    /// Flies `pilot`, from the main menu, in place of any flight; Escape
    /// goes back to the menu. A pilot docked at a stellar resumes in its
    /// spaceport.
    fn start_flight(&mut self, pilot: Pilot) {
        self.spaceport = None;
        let mut flight = self.flight(|data| FlightView::with_pilot(data, pilot));
        let landing = flight.take_landing();
        self.flight = Some(flight);
        self.return_to = Side::MainMenu;
        self.switch_to(Side::Flight);
        if let Some(stellar) = landing {
            self.show_spaceport(stellar);
        }
    }

    /// Where pilots are kept, if anywhere.
    fn keeper(&self) -> Option<Rc<PilotKeeper<Box<dyn PilotStore>>>> {
        self.menu.as_ref().and_then(|menu| menu.pilots.clone())
    }

    /// Saves the pilot flying, when it has a name and there is somewhere to
    /// keep it; a failure becomes a warning. Pilots without a name, which
    /// the developer's F flies, are never saved.
    fn save_pilot(&mut self) {
        let (Some(flight), Some(keeper)) = (&self.flight, self.keeper()) else {
            return;
        };
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

    /// Saves the pilot if flight says a save is due: it has landed, taken
    /// off or changed in the spaceport.
    fn save_if_due(&mut self) {
        if self.flight.as_mut().is_some_and(FlightView::take_save_due) {
            self.save_pilot();
        }
    }

    /// The main menu's input: its choice opens the New Pilot dialog or the
    /// saved pilots' list over it, or quits. Tab goes to the ship browser.
    fn menu_input(&mut self, input: &Input) -> ScreenAction {
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
        let menu = self.menu.as_mut().expect(MENU);
        menu.screen.input(input);
        match menu.screen.take_choice() {
            Some(MenuChoice::NewPilot) => self.open_new_pilot(),
            Some(MenuChoice::OpenPilot) => self.open_pilot_list(),
            Some(MenuChoice::Quit) => return ScreenAction::Quit,
            None => {}
        }
        ScreenAction::None
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
    /// can be saved under, is refused; any other creates the pilot, saves
    /// it at once (so Open Pilot lists it before it lands) and flies it.
    fn new_pilot_input(&mut self, input: &Input) -> ScreenAction {
        let dialog = self.new_pilot.as_mut().expect("open");
        dialog.input(input);
        let outcome = dialog.take_outcome();
        match outcome {
            Some(NewPilotOutcome::Cancel) => self.close_new_pilot(),
            Some(NewPilotOutcome::Create(name)) => {
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
                        if let Some(keeper) = keeper
                            && let Err(warning) = keeper.save(&pilot)
                        {
                            self.warnings.push(warning);
                        }
                        self.close_new_pilot();
                        self.start_flight(pilot);
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
    /// warning.
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
                        self.start_flight(pilot);
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

    /// Closes the saved pilots' list, keeping its sounds.
    fn close_pilot_list(&mut self) {
        if let Some(mut list) = self.open_pilot.take() {
            self.sounds.extend(list.take_sounds());
        }
    }

    /// The spaceport's input, all of it: the outfitter opening tells the
    /// flight and shows its outfitter afresh; an order on its exchange trades,
    /// and one in its outfitter buys or sells, and the exchange and the
    /// outfitter as they then are go back to it. A ship asked for in its
    /// shipyard opens the prompt for its name, when it can be bought; the
    /// ship named there is bought, and one declined there draws its roll
    /// again when the shipyard's list is next built, changing nothing
    /// else. Once it is left, the ship takes off
    /// and flight shows.
    fn spaceport_input(&mut self, input: &Input) -> ScreenAction {
        let spaceport = self.spaceport.as_mut().expect(LANDED);
        spaceport.input(input);
        // A new opening clears what lasts one, and the outfitter it opened
        // on was the cached one: it shows the flight's afresh.
        if spaceport.take_outfitter_opened() {
            let flight = self.flight.as_mut().expect(ENTERED);
            flight.open_outfitter();
            if let Some(outfitter) = flight.outfitter() {
                spaceport.set_outfitter(outfitter);
            }
        }
        let trade = spaceport.take_trade();
        let outfit = spaceport.take_outfit();
        let ship = spaceport.take_ship();
        let recharge = spaceport.take_recharge();
        let hire = spaceport.take_hire();
        if spaceport.take_hire_request()
            && let Some(list) = self.flight.as_mut().expect(ENTERED).escorts_for_hire()
        {
            spaceport.open_hire(list);
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
        if trade.is_some() || outfit.is_some() || ship.is_some() || recharge || hire.is_some() {
            let flight = self.flight.as_mut().expect(ENTERED);
            // A refused order changes nothing; the screen greys what it can.
            if let Some(order) = trade {
                let _ = flight.trade(order);
            }
            if let Some(order) = outfit {
                let _ = flight.outfit(order);
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
        if spaceport.left() {
            self.sounds.extend(spaceport.take_sounds());
            self.switch_to(Side::Flight);
            self.spaceport = None;
            self.flight.as_mut().expect(ENTERED).take_off();
        }
        ScreenAction::None
    }

    /// Routes one input, as [`Screen::input`] describes.
    fn route(&mut self, input: &Input) -> ScreenAction {
        if self.about.is_some() {
            return self.about_input(input);
        }
        if self.preferences.is_some() {
            return self.preferences_input(input);
        }
        if self.haggle.is_some() {
            return self.haggle_input(input);
        }
        if self.comm.is_some() {
            return self.comm_input(input);
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
        // A ship's name is typed into its prompt, P and all.
        if self
            .spaceport
            .as_ref()
            .and_then(SpaceportView::open_shipyard)
            .is_some_and(|shipyard| shipyard.naming().is_some())
        {
            return self.spaceport_input(input);
        }
        if let Input::Key {
            key: Key::Char('p'),
            pressed,
            repeat,
        } = *input
            && self.dialogs.is_some()
        {
            if pressed && !repeat {
                self.open_preferences();
            }
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

/// "Desc Dialog" showing the About text, from the dialogs and the game
/// data.
fn about_dialog(dialogs: &Dialogs, data: &GameData) -> Result<DescDialog, String> {
    let template = dialogs.resources.dialog_template(DESC_DIALOG)?;
    let text = data.description(ABOUT_TEXT)?;
    Ok(DescDialog::new(
        &template,
        &text,
        data.button_style(),
        Rc::clone(&dialogs.metrics),
    ))
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
    /// targets the next ship, and F and I do nothing.
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
    /// pilot flies it, and Escape in flight goes back to the menu, saving
    /// the pilot and putting it away. Quit, or Escape, quits. I and P work
    /// on the menu, and Tab goes to the ship browser; on the ship browser
    /// and the galaxy map, Escape goes back to the menu instead of
    /// quitting.
    ///
    /// After every input, a pilot with a name is saved when flight says a
    /// save is due: on landing, on taking off, and after a change in the
    /// spaceport ([`AppScreen::transact`]).
    fn input(&mut self, input: &Input) -> ScreenAction {
        let action = self.route(input);
        self.save_if_due();
        action
    }

    /// Only the side shown ticks; a hidden one is paused. While an overlay
    /// (the About dialog, the Preferences dialog, the New Pilot dialog or
    /// the saved pilots' list) is open, only it ticks: flight pauses under
    /// the preferences, as in the original. A save that is due is made.
    fn tick(&mut self, dt: Duration) {
        match self.overlay_mut() {
            Some(overlay) => overlay.tick(dt),
            None => self.shown_mut().tick(dt),
        }
        self.save_if_due();
    }

    /// The side shown, then the hint (not over flight, which has its own
    /// help line, or the spaceport), then the overlay open: the New Pilot
    /// dialog or the saved pilots' list, then the About dialog or the
    /// Preferences dialog.
    fn draw(&self, list: &mut DrawList) {
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
        if let Some(dialog) = &self.plunder {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.assignment {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.comm {
            dialog.draw(list);
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
    }

    fn cancel_pointer(&mut self) {
        match self.overlay_mut() {
            Some(overlay) => overlay.cancel_pointer(),
            None => self.shown_mut().cancel_pointer(),
        }
    }

    fn release_keys(&mut self) {
        match self.overlay_mut() {
            Some(overlay) => overlay.release_keys(),
            None => self.shown_mut().release_keys(),
        }
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

    /// Saves the pilot flying, in flight or in the spaceport, when it has
    /// a name.
    fn quit(&mut self) {
        self.save_pilot();
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
mod tests {
    use std::io;
    use std::path::Path;
    use std::rc::Rc;

    use nova_data::graphics::RLED;
    use nova_data::graphics::fixture::RledBuilder;
    use nova_data::records::character::Character;
    use nova_data::records::desc::Desc;
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
    /// names the same `rlëD`. The only `chär` starts in ship 128, an
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
        let mut anim = vec![0; ShipAnim::SIZE.expect("fixed")];
        anim[0x00..0x02].copy_from_slice(&1000_i16.to_be_bytes());
        anim[0x04..0x06].copy_from_slice(&1_i16.to_be_bytes());
        anim[0x34..0x36].copy_from_slice(&4_i16.to_be_bytes());
        let sheet = (0..4)
            .fold(RledBuilder::new(1, 1), |sheet, _| {
                sheet.frame(|f| f.line().pixels(&[0x7C00]))
            })
            .build();
        let mut ship = vec![0; Ship::SIZE.expect("fixed")];
        let mut average = ship.clone();
        for (at, value) in [(0x04, 300_i16), (0x06, 300), (0x08, 10)] {
            average[at..at + 2].copy_from_slice(&value.to_be_bytes());
        }
        let mut character = vec![0; Character::SIZE.expect("fixed")];
        let mut stellar = landable();
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
            let mut booster = vec![0; Outfit::SIZE.expect("fixed")];
            // For sale every day: `BuyRandom` 100.
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
            fork = fork.resource(Outfit::TYPE, 128, Some(b"Booster"), &booster);
        }
        if shipbuying {
            stellar[0x09] |= 0x08;
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
        }
        character[0x04..0x06].copy_from_slice(&128_i16.to_be_bytes());
        character[0x06..0x08].copy_from_slice(&128_i16.to_be_bytes());
        for slot in 1..4 {
            let at = 0x06 + 2 * slot;
            character[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
        }
        let mut spin = vec![0; Spin::SIZE.expect("fixed")];
        spin[0..2].copy_from_slice(&1000_i16.to_be_bytes());
        let fork = fork
            .resource(Ship::TYPE, 129, Some(b"Second"), &ship)
            .resource(Ship::TYPE, 128, Some(b"First"), &average)
            .resource(Character::TYPE, 128, Some(b"Pilot"), &character)
            .resource(ShipAnim::TYPE, 128, None, &anim)
            .resource(ShipAnim::TYPE, 129, None, &anim)
            .resource(RLED, 1000, None, &sheet)
            .resource(System::TYPE, 128, Some(b"Alpha"), &system(0, &[128]))
            .resource(System::TYPE, 129, Some(b"Beta"), &system(300, &[]))
            .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &stellar)
            .resource(Spin::TYPE, 1000, None, &spin)
            .resource(Desc::TYPE, ABOUT_TEXT, None, &about_text())
            .build()
            .bytes;
        let file = OneFile(fork);
        let data = GameData::load(&file, &file, Path::new("/data"), None).expect("opens");
        Rc::new(data)
    }

    /// A `spöb` at (0, 0) that can be landed on: a planet with a bar.
    fn landable() -> Vec<u8> {
        let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
        bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
        bytes
    }

    /// The About text's `dësc`: ten short lines.
    fn about_text() -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..10)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\r")
            .into_bytes();
        bytes.push(0);
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
    /// text show; and "Spaceport" (1000), [`spaceport_template`].
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
            if id != DESC_DIALOG {
                return Err(format!("no DLOG {id}"));
            }
            Ok(DialogTemplate {
                bounds: Bounds::at(Point::new(0.0, 0.0), 200.0, 100.0),
                placement: Placement::Fixed,
                items: vec![
                    item(100.0, 70.0, 99.0, 25.0, ItemSpec::User),
                    item(10.0, 150.0, 20.0, 20.0, ItemSpec::Picture(1431)),
                    item(10.0, 10.0, 100.0, 36.0, ItemSpec::User),
                ],
            })
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

    /// Presses L twice in flight: the first requests clearance, the
    /// second lands.
    fn land_twice(screen: &mut AppScreen) {
        for pressed in [true, false, true] {
            assert_eq!(screen.input(&key(LAND, pressed)), ScreenAction::None);
        }
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
    fn leaving_the_spaceport_by_leave_keeps_its_click_and_the_take_off() {
        let mut screen = with_dialogs(data());
        land(&mut screen);
        assert_eq!(
            screen.take_sounds(),
            [Sound::Sim(SimSound::Landed {
                stellar_sound: None
            })]
        );
        let leave = spaceport(&screen)
            .dialog()
            .expect("laid out")
            .item_bounds(LEAVE_ITEM)
            .expect("Leave")
            .center();
        assert_eq!(
            press_and_release(&mut screen, leave),
            [vec![DOWN], vec![UP, Sound::Sim(SimSound::TookOff)]]
        );
        assert!(screen.spaceport_view().is_none(), "gone");
    }

    #[test]
    fn closing_the_about_text_by_done_keeps_its_click() {
        let mut screen = with_dialogs(data());
        open_about(&mut screen);
        let done = about(&screen)
            .dialog()
            .item_bounds(1)
            .expect("Done")
            .center();
        assert_eq!(press_and_release(&mut screen, done), [vec![DOWN], vec![UP]]);
        assert!(screen.about().is_none(), "closed");
    }

    // The Preferences dialog.

    const PREFS: Key = Key::Char('p');

    fn open_prefs(screen: &mut AppScreen) {
        assert_eq!(screen.input(&key(PREFS, true)), ScreenAction::None);
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
        }
    }

    #[test]
    fn p_opens_the_preferences_over_ships_galaxy_flight_its_map_and_the_spaceport() {
        let mut screen = with_dialogs(data());
        assert!(screen.preferences().is_none());
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::ShipBrowser);

        screen.input(&key(Key::Tab, true));
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::GalaxyMap);

        fly(&mut screen);
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Flight, "Escape only closes it");

        screen.input(&key(MAP, true));
        open_prefs(&mut screen);
        screen.input(&key(Key::Enter, true));
        assert_eq!(screen.showing(), Showing::FlightMap);
        screen.input(&key(Key::Escape, true));

        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        open_prefs(&mut screen);
        screen.input(&key(Key::Escape, true));
        assert_eq!(screen.showing(), Showing::Spaceport, "still landed");
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
        screen.input(&key(PREFS, true));
        assert_eq!(screen.showing(), Showing::Flight);
        assert!(screen.preferences().is_none());
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
    fn closing_by_ok_keeps_its_click() {
        let mut screen = with_dialogs(data());
        open_prefs(&mut screen);
        let ok = prefs_dialog(&screen)
            .dialog()
            .item_bounds(1)
            .expect("OK")
            .center();
        assert_eq!(press_and_release(&mut screen, ok), [vec![DOWN], vec![UP]]);
        assert!(screen.preferences().is_none(), "closed");
    }

    #[test]
    fn the_flight_help_offers_p() {
        assert!(nova_view::flight::view::HELP.contains("P: preferences"));
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
    fn menu(store: &MemoryPilots) -> AppScreen {
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
    fn create(screen: &mut AppScreen, name: &str) {
        assert_eq!(choose(screen, MenuChoice::NewPilot), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::NewPilot);
        type_text(screen, name);
        screen.input(&key(Key::Enter, true));
    }

    fn pilot(screen: &AppScreen) -> &nova_sim::Pilot {
        flight(screen).pilot().expect("flying")
    }

    fn saved(store: &MemoryPilots, key: &str) -> nova_sim::Pilot {
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
    fn quit_or_escape_on_the_main_menu_quits() {
        let store = MemoryPilots::new();
        let mut screen = menu(&store);
        assert_eq!(choose(&mut screen, MenuChoice::Quit), ScreenAction::Quit);
        let mut screen = menu(&store);
        assert_eq!(screen.input(&held(Key::Escape)), ScreenAction::None);
        assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
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
        assert_eq!(screen.showing(), Showing::Spaceport);
        assert_eq!(spaceport(&screen).stellar(), nova_sim::StellarId(128));
        assert_eq!(*pilot(&screen), landed);
        assert!(screen.pilot_list().is_none());
        assert_eq!(screen.take_sounds(), [], "no landing sound");
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
            [vec![DOWN], vec![UP]]
        );
        assert!(screen.pilot_list().is_none(), "closed");
        let at = screen
            .main_menu()
            .expect("a main menu")
            .button(MenuChoice::NewPilot)
            .rect
            .center();
        press_and_release(&mut screen, at);
        let ok = screen
            .new_pilot()
            .expect("open")
            .dialog()
            .item_bounds(2)
            .expect("Cancel")
            .center();
        assert_eq!(press_and_release(&mut screen, ok), [vec![DOWN], vec![UP]]);
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
    /// Cancel (2) and the Full Name field (8); the rest parked.
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
        let mut screen = AppScreen::new(trading_data())
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
        // A max sale and a max buy, then a click on Buy with no space.
        screen.input(&key(Key::Alt, true));
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).held(FOOD), 0, "everything held");
        assert_eq!(pilot(&screen).cash(), 1000);
        screen.input(&key(Key::Char('b'), true));
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
    use nova_view::spaceport::outfitter::{BUY_ITEM as OUTFIT_BUY_ITEM, OUTFIT_DIALOG};

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
        let mut screen = AppScreen::new(outfitting_data())
            .with_dialogs(Rc::new(OutfitDialogs), Rc::new(MonoMetrics))
            .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics));
        create(&mut screen, "Ada");
        land_twice(&mut screen);
        assert_eq!(screen.showing(), Showing::Spaceport);
        screen
    }

    const BOOSTER: OutfitId = OutfitId(128);

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
        // S sells one back for half.
        screen.input(&key(Key::Char('s'), true));
        assert_eq!(pilot(&screen).owned(BOOSTER), 1);
        assert_eq!(pilot(&screen).cash(), 250);
        // Escape closes the outfitter; the spaceport stays.
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(&screen).open_outfitter().is_none());
        assert_eq!(screen.showing(), Showing::Spaceport);
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
        let mut screen = AppScreen::new(shipbuying_data())
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
        let mut screen = landed_shipyard(&store);
        click_port_item(&mut screen, 9);
        press_b(&mut screen);
        screen.input(&key(Key::Enter, true));
        let writes = store.writes();
        let before = pilot(&screen).clone();
        // Ship 129 again: 200 and a 400 trade-in do not cover 800.
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
        // Once it is closed, P opens Preferences again.
        click_cancel(&mut screen);
        assert!(name_prompt(&screen).is_none(), "declined");
        screen.input(&key(Key::Char('p'), true));
        assert!(screen.preferences().is_some(), "P is Preferences again");
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
        assert!(!lists_second(&screen), "the new roll missed");
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

    /// [`boarding_data_with`] no cash.
    fn boarding_data() -> Rc<GameData> {
        boarding_data_with(0)
    }

    /// The first `chär` flies ship 128 ("First": average, a crew of 10)
    /// in system 128 with `cash`, whose one `düde`, at 100 %, flies ship
    /// 129 ("Trader": a crew of 3, 100 armour, 20 holds, 300 fuel, `Cost`
    /// 150,000) for food and money (`Booty` 0x0041), independent. Food is
    /// commodity 0.
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
        for (k, pressed) in [
            (Key::Tab, true),
            (Key::Tab, false),
            (Key::Char('b'), true),
            (Key::Char('b'), false),
        ] {
            screen.input(&key(k, pressed));
        }
    }

    /// The router over [`boarding_data`] with dialogs, in the developer's
    /// flight, boarded.
    fn boarded() -> AppScreen {
        let mut screen = boardable(with_dialogs(boarding_data()));
        fly(&mut screen);
        board(&mut screen);
        assert_eq!(screen.showing(), Showing::Plunder);
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
        assert_eq!(
            sounds,
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
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
        for (k, pressed) in [
            (Key::Tab, true),
            (Key::Tab, false),
            (Key::Char('y'), true),
            (Key::Char('y'), false),
        ] {
            screen.input(&key(k, pressed));
        }
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
        assert_eq!(
            sounds,
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
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
    fn the_routers_escort_orders_rule_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_escort_orders(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.escort_orders(), source);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        assert_eq!(
            flight(&screen).session().expect("flying").escort_orders(),
            RuleSource::Engine,
            "the engine's by default"
        );
    }

    #[test]
    fn the_routers_fighter_rules_reach_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_fighter_launch(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.fighter_launch(), source);
            assert_eq!(session.fighter_recall(), RuleSource::Engine);
            let mut screen = AppScreen::new(data()).with_fighter_recall(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.fighter_recall(), source);
            assert_eq!(session.fighter_launch(), RuleSource::Engine);
        }
    }

    #[test]
    fn the_routers_hiring_rules_reach_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_hire_require(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.hire_require(), source);
            assert_eq!(session.take_off_pay(), RuleSource::Engine);
            assert_eq!(session.escort_wage(), RuleSource::Engine);
            let mut screen = AppScreen::new(data()).with_take_off_pay(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.take_off_pay(), source);
            assert_eq!(session.hire_require(), RuleSource::Engine);
            let mut screen = AppScreen::new(data()).with_escort_wage(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.escort_wage(), source);
            assert_eq!(session.take_off_pay(), RuleSource::Engine);
        }
    }

    #[test]
    fn the_routers_buy_random_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_buy_random(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.buy_random(), source);
            assert_eq!(session.hire_require(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.buy_random(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_junk_flags_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_junk_flags(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.junk_flags(), source);
            assert_eq!(session.buy_random(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.junk_flags(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_launcher_sale_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_launcher_sale(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.launcher_sale(), source);
            assert_eq!(session.junk_flags(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.launcher_sale(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_purchase_cargo_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_purchase_cargo(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.purchase_cargo(), source);
            assert_eq!(session.event_price(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.purchase_cargo(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_junk_price_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_junk_price(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.junk_price(), source);
            assert_eq!(session.purchase_cargo(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.junk_price(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_junk_trade_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_junk_trade(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.junk_trade(), source);
            assert_eq!(session.junk_price(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.junk_trade(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_trade_lot_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_trade_lot(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.trade_lot(), source);
            assert_eq!(session.junk_trade(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.trade_lot(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_outfit_limit_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_outfit_limit(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.outfit_limit(), source);
            assert_eq!(session.raised_max(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.outfit_limit(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_event_price_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_event_price(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.event_price(), source);
            assert_eq!(session.raised_max(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.event_price(), RuleSource::Engine);
    }

    #[test]
    fn the_routers_raised_max_reaches_every_flight() {
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_raised_max(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.raised_max(), source);
            assert_eq!(session.launcher_sale(), RuleSource::Engine);
        }
        let mut screen = AppScreen::new(data());
        fly(&mut screen);
        let session = flight(&screen).session().expect("flying");
        assert_eq!(session.raised_max(), RuleSource::Engine);
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
        assert_eq!(session.comm_quote(), RuleSource::Engine);
        for source in RuleSource::ALL {
            let mut screen = AppScreen::new(data()).with_comm_quote(source);
            fly(&mut screen);
            let session = flight(&screen).session().expect("flying");
            assert_eq!(session.comm_quote(), source);
        }
    }

    /// The rule-bearing parts of `screen`, each by name, as they print.
    fn rules_of(screen: &AppScreen) -> Vec<(&'static str, String)> {
        vec![
            ("behaviour", format!("{:?}", screen.behaviour)),
            ("disable_rule", format!("{:?}", screen.disable_rule)),
            ("defence_rule", format!("{:?}", screen.defence_rule)),
            ("law", format!("{:?}", screen.law)),
            ("boarding_rule", format!("{:?}", screen.boarding_rule)),
            ("hail_options", format!("{:?}", screen.hail_options)),
            ("escort_orders", format!("{:?}", screen.escort_orders)),
            ("fighter_launch", format!("{:?}", screen.fighter_launch)),
            ("fighter_recall", format!("{:?}", screen.fighter_recall)),
            ("hire_require", format!("{:?}", screen.hire_require)),
            ("buy_random", format!("{:?}", screen.buy_random)),
            ("junk_flags", format!("{:?}", screen.junk_flags)),
            ("launcher_sale", format!("{:?}", screen.launcher_sale)),
            ("raised_max", format!("{:?}", screen.raised_max)),
            ("event_price", format!("{:?}", screen.event_price)),
            ("purchase_cargo", format!("{:?}", screen.purchase_cargo)),
            ("junk_price", format!("{:?}", screen.junk_price)),
            ("junk_trade", format!("{:?}", screen.junk_trade)),
            ("trade_lot", format!("{:?}", screen.trade_lot)),
            ("outfit_limit", format!("{:?}", screen.outfit_limit)),
            ("take_off_pay", format!("{:?}", screen.take_off_pay)),
            ("escort_wage", format!("{:?}", screen.escort_wage)),
            ("hire_terms", format!("{:?}", screen.hire_terms)),
            ("control_bits", format!("{:?}", screen.control_bits)),
            ("person_rules", format!("{:?}", screen.person_rules)),
            ("comm_quote", format!("{:?}", screen.comm_quote)),
        ]
    }

    /// The setting each rule's key reaches.
    fn setting_of(key: RuleKey) -> &'static str {
        match key {
            RuleKey::CrimeGains => "law",
            RuleKey::EmptyBooty
            | RuleKey::CrewlessCapture
            | RuleKey::PersonCredits
            | RuleKey::GrantCount
            | RuleKey::GrantMax => "boarding_rule",
            RuleKey::PiracyPolice | RuleKey::EscortAi | RuleKey::PersonCoward => "behaviour",
            RuleKey::QuietHails | RuleKey::LongAdvice | RuleKey::PersonJoin => "hail_options",
            RuleKey::EscortOrders => "escort_orders",
            RuleKey::FighterLaunch => "fighter_launch",
            RuleKey::FighterRecall => "fighter_recall",
            RuleKey::HireRequire => "hire_require",
            RuleKey::BuyRandom => "buy_random",
            RuleKey::JunkFlags => "junk_flags",
            RuleKey::LauncherSale => "launcher_sale",
            RuleKey::RaisedMax => "raised_max",
            RuleKey::EventPrice => "event_price",
            RuleKey::PurchaseCargo => "purchase_cargo",
            RuleKey::JunkPrice => "junk_price",
            RuleKey::JunkTrade => "junk_trade",
            RuleKey::TradeLot => "trade_lot",
            RuleKey::OutfitLimit => "outfit_limit",
            RuleKey::TakeOffPay => "take_off_pay",
            RuleKey::HireFee => "hire_terms",
            RuleKey::EscortWage => "escort_wage",
            RuleKey::PersonOdds
            | RuleKey::SystemPersons
            | RuleKey::LinkSystSlip
            | RuleKey::ShieldMod => "person_rules",
            RuleKey::CommQuote => "comm_quote",
        }
    }

    #[test]
    fn the_rulebook_routes_each_rule_to_its_own_setting() {
        let engine = rules_of(&AppScreen::new(data()).with_rulebook(&Rulebook::default()));
        assert_eq!(
            engine,
            rules_of(&AppScreen::new(data())),
            "the engine's are the router's defaults"
        );
        for key in RuleKey::ALL {
            let rulebook = Rulebook::default().with_override(key, RuleSource::Bible);
            let bible = rules_of(&AppScreen::new(data()).with_rulebook(&rulebook));
            let changed: Vec<&str> = engine
                .iter()
                .zip(&bible)
                .filter(|(was, now)| was.1 != now.1)
                .map(|(was, _)| was.0)
                .collect();
            assert_eq!(changed, [setting_of(key)], "{key:?}");
        }
        let all =
            rules_of(&AppScreen::new(data()).with_rulebook(&Rulebook::new(RuleSource::Bible)));
        assert!(all.contains(&("comm_quote", "Bible".to_owned())), "{all:?}");
        // The joining and grant rules reach the hail options' Use As
        // Escort and the boarding rule's two fields.
        let joined = Rulebook::default()
            .with_override(RuleKey::PersonJoin, RuleSource::Bible)
            .with_override(RuleKey::GrantCount, RuleSource::Bible)
            .with_override(RuleKey::GrantMax, RuleSource::Bible);
        let routed = rules_of(&AppScreen::new(data()).with_rulebook(&joined));
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
        assert!(boarding.contains("grant_max: Bible"), "{boarding}");
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
}
