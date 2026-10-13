//! The flight screen: the player's ship flying from its starting system,
//! over the parallax starfield and among the system's stellars, with the
//! camera following the ship, and jumping through hyperspace along the
//! course plotted on the galaxy map.
//!
//! The screen owns a [`nova_sim::Session`], flying a new pilot
//! ([`FlightView::new`]) or a given one ([`FlightView::with_pilot`]), and
//! its catalog. It reads, once
//! when it is built: the session's system, through the [`SystemCatalog`]
//! port, the ship's sprite sheet, through the [`ShipSprites`] port, the
//! HUD's status bar for the player's government, through the
//! [`StatusBars`] port, the galaxy for its course map, through the
//! [`GalaxyCatalog`] port, and the look of every weapon and explosion type,
//! through the [`CombatLooks`] port. After that it reads only when the
//! session tells it entered a system afresh: that system, and when the
//! player's ship changes class: its sprite sheet, and when the system's NPC
//! traffic is populated: its traffic, through the [`TrafficCatalog`]
//! port, and the target cards and codes of the ship types and governments
//! it brings. Drawing and input never read anything.
//!
//! The session populates the system's NPC traffic when it says to (see
//! [`Session::tick_traffic`]), and as the ship takes off
//! ([`Session::take_off`]), rolled on the screen's [`SharedChance`].
//! Each time, the sprite sheet of each ship type the traffic can spawn is
//! read, once per type for the life of the screen. After each step of the
//! player's ship, the traffic takes a step,
//! its NPCs deciding as the screen's [`Behaviour`] says
//! ([`FlightView::with_behaviour`]; [`NovaAi`] by default), and then the
//! fight ([`Session::tick_combat`]), its ships disabled as the screen's
//! [`DisableRule`] says ([`FlightView::with_disable_rule`];
//! [`NovaDisable`] by default), their point defence engaging the
//! missiles its [`PointDefenceRule`] calls hostile
//! ([`FlightView::with_point_defence_rule`]; [`Allegiance`] by default),
//! and the player's crimes judged by its [`LegalCode`]
//! ([`FlightView::with_law`]; [`NovaLaw`] by default). What could not be read of the looks
//! (each once, when the screen is built), then the session's diagnostics
//! about game data it does not handle yet, pass through
//! [`Screen::take_diagnostics`] for the app to write out. Each NPC is
//! drawn with its own ship's sprite, after the stellars and before the
//! player, smoothed between its last two steps as the player's ship is
//! (a crossed box when its sheet cannot be read), and as a dim blip on the
//! radar. One waiting at the hypergate or wormhole it comes out of is not
//! drawn until its last 16 ticks there, over which it fades in; it is on
//! the radar all along, as the original's radar does not ask.
//!
//! Each step's fight events are drained into the [`Effects`]: its
//! explosions, debris and sounds, rolled on their own [`SharedChance`]
//! ([`FlightView::with_effects_chance`]), never on the simulation's. The
//! beams live before the fight's step are passed along, so a looped
//! weapon is heard once a beam. The fight is drawn over the stellars:
//! beams that go under the ships, then the ships, then the shots (each
//! smoothed as the ships are) and the other beams, then the explosions and
//! debris, then the brackets round the target ([`weapons`],
//! [`effects`](super::effects), [`target`]). The HUD shows the target
//! panel and the secondary weapon's line, its text laid out by the
//! screen's [`TextMetrics`] ([`FlightView::with_metrics`]). Landing and
//! arriving clear the effects.
//!
//! Each day a jump takes rolls the planetary events on the screen's
//! [`SharedChance`] ([`FlightView::with_chance`]); without one, nothing
//! random happens. Landed at a trade center, the session's exchange can be
//! read ([`FlightView::market`]) and traded on ([`FlightView::trade`]);
//! landed at an outfitter, so can its outfitter ([`FlightView::outfitter`],
//! [`FlightView::outfit`]); and landed at a shipyard, a new ship can be
//! bought ([`FlightView::shipyard`], [`FlightView::buy_ship`]), whose
//! sprite sheet is read then.
//!
//! The screen redraws from what the session tells it
//! ([`SessionEvent`]), never from which of the session's methods ran or
//! what it gave. Each frame first takes the session's events, so a change
//! made outside the screen's own methods (an edit through the pilot desk,
//! a set expression run by a control bit) is drawn by the next frame; each
//! of the screen's own methods that drives the session takes them last,
//! so the change is drawn at once. A change of ship, after a purchase, a
//! capture, or a `C`, `E` or `H` set operator, reads the player's sprite
//! sheet afresh when the class is another than the one it was read for,
//! once; a system entered afresh, by a jump, a gate, a move or a
//! relocation, is laid out; and a move to another stellar of the system
//! shown draws the ship there.
//!
//! The HUD is drawn over everything but a jump's fade: the status bar against the
//! right edge, its radar showing the stellars and ships around the ship
//! as drawn, its bars the session's shield, armour and fuel, and its nav
//! area the navigation target or else the course. Without a status bar it
//! says why, and flight goes on. By [`RuleKey::HudCadence`]'s engine
//! reading the radar shows what it held at its last refresh, every 8
//! ticks ([`HudCadence`]) or when the original's `_RedrawStatus` would
//! redraw it (a new system, landing, take-off, or closing the map, the
//! comm dialog or a mission dialog); by the other, every frame. The nav
//! area likewise holds its text and colours between refreshes every 15
//! ticks, and when `_RedrawStatus` would redraw it, but shows a change of
//! selection at once. The target panel likewise holds what it showed
//! between its redraws every 45 ticks, at the next radar refresh once the
//! target's shield or ship type changes, and when `_RedrawStatus` would
//! redraw it or the plunder dialog closes, but shows a change of target
//! at once ([`retake_target`]). Its target blinks bright on the
//! same 8 ticks either way, and it shows no stellars or ships while the
//! ship is landed. The player's outfits give it IFF colours and a density
//! scanner ([`Session::radar_gear`]); far from the system's centre an
//! arrow points back to it; and each tick the interference in the system
//! ([`Session::interference`]) may fill it with static, rolled on the
//! effects' chance.
//!
//! Each [`Screen::tick`] runs the simulation's fixed-step clock: the
//! frame's time becomes whole steps of 1/30 s, each flown with the keys
//! held, and what is left over is how far the display is between the last
//! two steps. The ship, and the camera with it, are drawn that far from
//! the step before the last towards the last, so motion is smooth at any
//! frame rate while the simulation itself never depends on it.
//!
//! The ship is drawn as its sheet's rotation frame for the heading shown,
//! then, at the same centre and on the same frame (modulo the layer's
//! frame count), its engine glow and its running lights. The glow is
//! drawn at the level [`nova_sim::glow_level`] gives for the session's
//! glow base ([`Session::engine_glow`]) at the flight's time in ticks: it
//! fades in over 24 ticks of thrust, flickers, and fades out over 24
//! ticks after Up is released, and landing and beginning a jump put it
//! out. The lights are drawn at the level [`nova_sim::lights_level`]
//! gives for the sheet's blink at the flight's time in ticks. Each is
//! drawn at [`lights_tint`] of its level, or not at all while it is hidden
//! or off. Both are combined by OR with what is beneath
//! ([`Blend::Or`](crate::Blend::Or)) at every level, scaled by level/32,
//! like the original's `_BlitPixieRLETranslucent` (0xc1568) below full and
//! `_BlitPixieRLEAddOver` (0xc24bf) at full: see [`lights_tint`] for the
//! full record. The flight's time
//! stops while the course map is open, so the lights blink and the glow
//! flickers in game time. Random blinking rolls on [`HashedRolls`] from
//! seed 0, and the glow's flicker on [`HashedRolls`] from seed 1. A layer
//! that cannot be shown is left out silently.
//!
//! Input, the original's default keys:
//!
//! - Up thrusts, Left and Right turn, and Down turns to face against the
//!   ship's motion, while held: a press (or its key repeats) holds the key
//!   and its release lets it go. Left and Right together cancel, and
//!   either overrides Down.
//! - L, once a press (its repeats do nothing), requests clearance, then
//!   lands, as [`landing`](nova_sim::landing) says: with no navigation
//!   target it selects the nearest landable stellar and shows the reply
//!   ([`clearance_message`]); with one it lands there. The router takes
//!   the landing ([`FlightView::take_landing`]) and shows the spaceport.
//!   The reply, or why a landing was refused, shows on the message line
//!   in the original's words (`STR#` 2002), for [`LANDING_TICKS`] or
//!   [`landing_refusal_ticks`].
//!   Over a hypergate or wormhole, the second L enters it instead
//!   ([`nova_sim::gate`]): a hypergate opens the course map as the
//!   hypergate map ([`MapMode::Hypergate`]), offering its linked
//!   systems, and closing the map (M, or the router's Escape) enters it
//!   for the one picked, or cancels; a wormhole passes the ship through at
//!   once. The ship comes out in the new system, which is read and laid
//!   out, the message line says so ([`arrival_message_with`]), and the
//!   screen fades in from white out of a hypergate
//!   ([`JumpEffect::emerging`]) or flashes white out of a wormhole
//!   ([`JumpEffect::flash`]). Clearance at a hypergate, and why a gate
//!   cannot be entered, are in the gate's own words
//!   ([`hypergate_clearance_message`], [`gate_refusal_message`]); the
//!   hypergate's two clearance replies are picked by a roll on the
//!   flight's chance.
//! - M (a press, not its repeats) opens the course map, a [`GalaxyMap`]
//!   in [`MapMode::Course`](crate::galaxy::MapMode::Course), and lets go
//!   of the flight keys. While it is open flight is paused, as in the
//!   original, only the map is drawn and every input goes to it, except
//!   that M closes it ([`FlightView::close_map`] closes it too, for the
//!   router's Escape). A system clicked on the map becomes the
//!   destination: the session plots the course there and the map shows
//!   it. The map shows the systems the pilot has explored, and the rest
//!   unexplored. It draws the session's galaxy ([`Session::galaxy`]):
//!   the catalog's geometry with only the visible systems, links and
//!   active nebulae, each system coloured by its government there, rebuilt
//!   on the frame after each change to it; and the course as it is flown
//!   ([`Session::course_shown`]), each hop the visible system standing
//!   for it.
//! - The HUD's nav area shows the navigation target, or else the next
//!   system on the course (see [`hud`](super::hud)). As in the original
//!   (`_HandlePlayer` @0x68390; see [`navigation`](nova_sim::navigation)),
//!   1-4 and F1-F4 ([`STELLAR_SLOT_KEYS`], presses) select the stellar in
//!   that slot of the system's nav defaults, 1-4 only while the escort
//!   menu is shut; F5 ([`NEAREST_STELLAR_KEY`]) the nearest landable one,
//!   as L does with none; a left click in space the stellar under it
//!   (not in the status panel, the right [`STATUS_BAR_WIDTH`] of the
//!   screen, where the radar takes no clicks); and backquote, Nav Off
//!   ([`NAV_OFF_KEY`]), clears it. The original has no stellar cycle key:
//!   Tab is its Target Select, cycling the ships.
//! - `\` (a press, not its repeats), the original's Hyper Select, plots
//!   a one-jump course to the next system linked with this one, as
//!   [`Session::select_next_system`] says, and clears the navigation
//!   target, so the nav area shows that system and J jumps there.
//! - J (a press) begins the pre-jump turn and slow-down towards the next
//!   system on the course when the session allows a jump, and otherwise
//!   says why in the original's words (`STR#` 2002), as a refused landing
//!   does. Once J is accepted the keys are let go and ignored while the
//!   session flies the ship round to the jump's bearing
//!   ([`Session::preparing_jump`]). The stars streak when the session says
//!   the jump has begun ([`Session::jumping`]), at once for a ship already
//!   facing that way and slow enough. The jump plays its [`JumpEffect`]:
//!   the session waits while the stars streak and the screen, HUD
//!   included, fades to white, as the original's whole-display fade does;
//!   then the ship arrives at full white, the new system is read and laid
//!   out, and the message line says so in the original's words
//!   ([`arrival_message`]). Flight resumes as the new system fades in
//!   from white: the keys work again and the session flies, J included,
//!   as under the original's asynchronous display fade. Only the streak
//!   and the fade-out ignore the keys. A multi-jump plays one
//!   effect, toward the first system, and the scene loaded is the last
//!   system it passes. With the Hyperspace Effects preference off
//!   ([`FlightView::with_hyperspace_effects`],
//!   [`FlightView::set_hyperspace_effects`]) a jump skips the fades, as
//!   the original's does: the stars streak, the ship arrives as the streak
//!   ends, and the arrival frame shows solid white once, with flight
//!   going on under it.
//! - Space fires the primary weapons and Control the secondary selected,
//!   while held. W (a press) selects the next secondary weapon, and with
//!   Alt (Option) the one before. Tab (a press) targets the next ship in
//!   turn, R the nearest threat, and Alt-R (Option-R) the nearest ship
//!   ([`TargetPick`]). The original's Shift-Tab, back through the ships,
//!   is not bound: there is no Shift key yet.
//! - B (a press) boards the target when the session allows it
//!   ([`Session::board`]), by the screen's [`BoardingRule`]
//!   ([`FlightView::with_boarding_rule`]; [`NovaBoarding`] by default),
//!   and otherwise says why in the original's words (`STR#` 2002
//!   #130-132), or nothing when there is no target or the player does not
//!   face it. A boarding lets go of the flight keys and is held for the
//!   router ([`FlightView::take_boarding`]), which opens the plunder
//!   dialog over the paused flight; each press there goes through
//!   [`FlightView::plunder`], and a capture's assignment through
//!   [`FlightView::assign`], each saying what it did as a message. After
//!   "Use As My Ship" the ship is drawn where it is, not on its way from
//!   the old one, and the session tells the change of ship, so the new
//!   ship's sprite sheet is read, once. Boarding a
//!   person that grants outfits says what it retrieved
//!   ([`grant_message`]) for [`GRANT_TICKS`]. A person of `Flags`
//!   0x0200 whose mission is available offers it in place of the plunder
//!   ([`Boarding::Offered`]), held for the router as a hail's offer is.
//! - Y (a press) hails the target ([`Session::open_hail`]): a person
//!   whose mission is available offers it in place of the comm dialog,
//!   held for the router ([`FlightView::take_offer`]), which opens the
//!   offer dialog over the paused flight and answers it through
//!   [`FlightView::accept_mission`] or [`FlightView::refuse_mission`]; a
//!   mission that cannot be refused, with no offer text, is accepted at
//!   once and shows nothing. Any other hail opens the comm dialog,
//!   listing the screen's [`HailOptions`]
//!   ([`FlightView::with_hail_options`]; Nova's by default), its replies
//!   read through the [`CommCatalog`] port, and otherwise says why in the
//!   original's words (`STR#` 2002 #53-54), or nothing with no target. A
//!   hail answered lets go of the flight keys and is held for the router
//!   ([`FlightView::take_hail`]), which opens the comm dialog over the
//!   paused flight; each press there goes through [`FlightView::answer`],
//!   each haggle through [`FlightView::haggle`], and closing the channel
//!   through [`FlightView::hang_up`]. Each step, the NPCs assisting the
//!   player give their help ([`Session::tick_assistance`], by the screen's
//!   [`DisableRule`]), and each says when it is done, as a message
//!   ([`comm_message`]).
//! - I ([`MISSION_INFO_KEY`], a press), outside the map and a jump, asks
//!   the router for the mission info screen over the paused flight
//!   ([`FlightView::take_mission_info_request`]), letting go of the
//!   flight keys, when the session shows an active mission there; with
//!   none it says "You have no active missions." (`STR#` 2002 #354).
//! - The player's escorts fly beside it (see [`Session`]), their ship
//!   types' sprite sheets read with the traffic's. F, D, V and C (presses)
//!   command them ([`Session::command_escorts`]): attack the player's
//!   target, defend the player, hold position, and recall them to
//!   formation; Option-C is Return to Hangar, sending the fighters out of
//!   the player's bays home to dock and the others back to formation.
//!   A command goes to the group selected in the escort menu, or to every
//!   escort while it is shut, and says what it changed
//!   ([`escort_command_message`]); one that changed nothing says nothing.
//!   E opens and closes the escort menu ([`EscortMenu`]), or says "You
//!   don't have any escorts." with none; while it is open, 1-5 select
//!   its group (rather than a stellar) and Return closes it. The menu is drawn over the flight,
//!   under the HUD, in the colours the [`EscortMenuLooks`] port gives,
//!   read when the screen is built. Option-Tab targets the next escort
//!   ([`TargetPick::NextEscort`]), as for a hail. The escorts' standing
//!   orders reset, or not, on entering a system as the session's rule
//!   set says ([`FlightView::with_rules`]).
//! - The player's fighters launch from its bays as its secondary weapon
//!   (W selects a bay, Ctrl launches), and the fighter types every bay in
//!   the system launches have their sheets read with the traffic's. What
//!   a fighter launched does first, and what becomes of the fighters out
//!   as the player leaves a system, follow the session's rule set
//!   ([`FlightView::with_rules`]); fighters abandoned in a jump
//!   are told on arrival ([`fighters_abandoned_message`]).
//! - Landed at an outfitter or a shipyard, the router asks the flight
//!   for its list ([`FlightView::outfitter`], [`FlightView::shipyard`])
//!   and trades through it ([`FlightView::outfit`],
//!   [`FlightView::outfit_counted`], [`FlightView::buy_ship`]), each item's roll for the day drawn on the
//!   flight's chance; the router tells the flight each time the
//!   outfitter opens ([`FlightView::open_outfitter`]). Where the shops,
//!   the exchange and the tribbles and perishables in flight follow a
//!   disputed rule, they read it from the session's rule set
//!   ([`FlightView::with_rules`]).
//! - Landed at a bar, the router asks the flight for the ships for hire
//!   and hires them ([`FlightView::escorts_for_hire`],
//!   [`FlightView::hire`]), the day's rolls drawn on the flight's chance;
//!   the session's hire terms and control bits are set on the screen
//!   ([`FlightView::with_hire_terms`], [`FlightView::with_control_bits`]),
//!   and its disputed hiring rules come from its rule set
//!   ([`FlightView::with_rules`]). Hired escorts who defect unpaid
//!   on arrival or at take-off are told ([`defection_message`]).
//! - Persons appear as the session's rules say
//!   ([`FlightView::with_person_rules`]); the target panel shows a
//!   person's own name and subtitle, a person hailed says its comm quote
//!   as the session's rule set says ([`FlightView::with_rules`]), and
//!   each step a person may
//!   say its hail quote ([`Session::tick_quotes`], right after the
//!   traffic's tick), shown in the message line for
//!   [`HAIL_QUOTE_TICKS`], the last one said winning.
//! - Return ([`MENU_CLOSE_KEY`], the original's `ackComm`) clears the
//!   message line and closes the escort menu.
//! - Escape belongs to the app's router, which closes the map or leaves
//!   flight. The screen never quits.
//!
//! The message line is drawn as `_DisplayComm` (@0x90201) draws it: one
//! slot, a new message replacing the old and its time, in the rect
//! `_InitSprites` sets up ([`hud::message_box`]), wrapped, in the system
//! font at 12 pt in white ([`hud::draw_message`]). It is drawn over the
//! starfield and under everything else, as the original draws it on the
//! sprite world's background. Each message stays for as many ticks of
//! flight as its original caller asks ([`MESSAGE_TICKS`] and the
//! constants after it, [`SimMessage::ticks`]). The dialogs drawn over
//! flight (the comm, offer and plunder dialogs, the map) leave it as it
//! is, and it shows again afterwards for the ticks it had left, as
//! `_RedrawComm` (@0x903a0) redraws it. Starting a jump clears it only
//! when it says [`TOO_CLOSE`] (`_HandlePlayer` @0x6b231-0x6b2a7), and
//! opening the escort menu only when it says [`NO_ESCORTS`]
//! (@0x6dedb-0x6df29). The system's title and the [`HELP`] line are the
//! port's own, drawn only when turned on ([`FlightOverlays`],
//! [`FlightView::with_overlays`]).

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use nova_sim::hire::{DEFECTED_ONE, DEFECTED_SOME};
use nova_sim::mission::offer_gap;
use nova_sim::mission::screens::{NO_ACTIVE_MISSIONS, mac_ticks};
use nova_sim::order_loop::{OrderRun, Stopped};
use nova_sim::traffic::emerge::{FULL_LEVEL, emerge_level};
use nova_sim::traffic::npc::Mode;
use nova_sim::{
    ALERT_CICN, Allegiance, Assigned, Assignment, Behaviour, BoardRefusal, Boarding, BoardingRule,
    Chance, ClassRow, Clearance, CombatCatalog, CommCatalog, CommNote, Condition, Controls,
    DisableRule, EscortCommand, FighterNote, FixedStep, GateKind, GateRefusal, Good, GovtId,
    Haggle, HailOptions, HailRefusal, HailView, HashedRolls, Help, JumpRefusal, LandOutcome,
    LandPress, LandingRefusal, LegalCode, Market, NeverFires, NovaAi, NovaBoarding, NovaDisable,
    NovaLaw, Npc, NpcId, Order, OutfitId, OutfitOrder, OutfitRefusal, OutfitRun, Outfitter, Pilot,
    PilotCatalog, PlunderView, PointDefenceRule, RechargeRefusal, Reselection, Reserves, RuleKey,
    RuleSource, Rulebook, Rules, Session, ShipId, ShipNaming, ShipPurchase, ShipRef, ShipRefusal,
    ShipState, Shipyard, SimMessage, StartError, StellarId, StellarPick, Steps, SystemId, Take,
    Taken, TargetPick, TargetTag, TradeRefusal, TrafficCatalog, Turn, Vec2, flight::normalized,
    flight::shortest_turn, glow_level, lights_level,
};
use nova_sim::{
    AbortError, AcceptError, ComputerRefusal, ControlBits, Hailed, HireList, HireRefusal,
    HireTerms, Hired, MissionBoardLine, MissionComputer, MissionId, MissionInfo, MissionText,
    NotOffered, OfferPrompt, PayNote, PersonRules, SessionEvent, StellarIff, TagNames, VenueOffer,
};

use super::alert;
use super::brackets;
use super::cadence::{Drawn, HudCadence, retake_target};
use super::catalog::{CombatLooks, Looks, ShipSheet, ShipSprites, StatusBars, TargetCard};
use super::effects::{Dying, Effects, Scene};
use super::escorts::{
    EscortMenu, EscortMenuColors, EscortMenuLooks, GROUP_KEYS, MENU_KEY, NO_ESCORTS, NUMBER_WORDS,
    Toggled, escort_command_message, fighters_abandoned_message,
};
use super::hud::{self, HudState, NavDestination, NavDisplay, STATUS_BAR_WIDTH, StatusBar};
use super::jump::{JumpEffect, JumpPhase};
use super::radar::{self, RadarScene, RadarShip, RadarStellar};
use super::sprite::rotation_frame;
use super::target::{self, Tag, TargetPanel};
use super::weapons::{self, BeamShown, ShotShown};
use crate::devtools::SessionDesk;
use crate::draw::{crossed_box, lights_tint};
use crate::galaxy::{GalaxyCatalog, GalaxyMap, MapMode};
use crate::spaceport::bar::MISSION_INFO_KEY;
use crate::system::camera::{Camera, VIEW_SIZE};
use crate::system::catalog::SystemCatalog;
use crate::system::scene::{self, PLACEHOLDER, PLACEHOLDER_SIZE, StellarLook, SystemScene};
use crate::system::starfield;
use crate::text::TextMetrics;
use crate::time::ticks;
use crate::ui::PlunderShown;
use crate::ui::comm::grouped;
use crate::{
    Color, Diagnostic, DrawList, ImageKey, Input, Key, MouseButton, Point, Screen, ScreenAction,
    Sound, SoundPrefs,
};

/// The developer overlays: the system's title and the help line.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// How far below the ship's placeholder the reason goes.
const MESSAGE_GAP: f32 = 22.0;
/// The help line.
pub const HELP: &str = "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: preferences   Esc: leave";
/// The port's own overlays on the flight screen, which the original does
/// not draw: developer settings, both off by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlightOverlays {
    /// The system's name and `sÿst` ID at the top left.
    pub title: bool,
    /// The [`HELP`] line of keys at the bottom.
    pub help: bool,
}

/// How many ticks each message stays on the message line: the frames its
/// caller passes `_DisplayComm` (@0x90201), which counts them down once
/// a frame in `_HandleComm` (@0x301b9). The original's callers and their
/// times are recorded in rdm phase hud-fidelity/phase-6-message-line;
/// those the engine does not raise yet are rdm phase
/// hud-fidelity/phase-12-flight-messages-the-original-raises's. The
/// session's own messages carry their time ([`SimMessage::ticks`]).
///
/// [`MESSAGE_TICKS`], 240, is the original's common time: "No
/// response." (`_HandlePlayerCommunication` @0x626c8), "You have no
/// active missions." (`_HandlePlayer` @0x705a8), the ships' comm notes
/// (`_HighLevelAIHandler` @0x8f6a1, `_LowLevelAIHandler` @0x86ef4),
/// the plunder dialog's lines (`_DoPlunderDialog` @0x936df-0x9498d) and
/// the abandoned fighters on arrival (@0x6c46d). It is also the time of
/// the messages the original does not have ([`TAKE_OFF_FIRST`],
/// [`JUMP_DISABLED`], [`IN_HYPERSPACE`], [`LAND_DISABLED`], [`REPELLED`],
/// [`NO_STELLARS`] on L) and of the defection notice, which the original
/// shows in a dialog (`_DoEscortPayment` @0x40301).
pub const MESSAGE_TICKS: u16 = 240;
/// A person's hail quote: 420 (0x1a4, `_ShowPersonHailQuote` @0x4464d).
pub const HAIL_QUOTE_TICKS: u16 = 420;
/// A grant's message: 240 (0xf0, `_DoPlunderDialog` @0x9342c).
pub const GRANT_TICKS: u16 = 240;
/// A mission ship boarded on the assist path, refuelled, or its cargo
/// retrieved: 250 (0xfa, `_HandlePlayerBoardAttempt` @0x65538, @0x656be).
pub const BOARD_LINE_TICKS: u16 = 250;
/// Why a boarding was refused: 360 (`_HandlePlayerBoardAttempt`
/// @0x6604a).
pub const BOARD_REFUSAL_TICKS: u16 = 360;
/// An escort command, and "You don't have any escorts.": 250
/// (`_IssueNewEscortCommand` @0x665b9, `_HandlePlayer` @0x6de5a).
pub const ESCORT_TICKS: u16 = 250;
/// A `Q` operator's message, in flight or at take-off: 500
/// (`_PlayGame` @0x465a8, `_PlayerLandOnStellar` @0x635c4).
pub const SCRIPT_MESSAGE_TICKS: u16 = 500;
/// Clearance's reply, and a landing denied or not possible: 250
/// (`_HandlePlayerDockRequest` @0x67c46, @0x67cbc, @0x67e67).
pub const LANDING_TICKS: u16 = 250;
/// A landing refused as too far or too fast: 360
/// (`_HandlePlayerDockRequest` @0x6787f).
pub const TOO_FAR_TICKS: u16 = 360;
/// A hypergate jump cancelled, or a wormhole leading nowhere: 250
/// (`_PlayerEnterHypergate` @0x6397e, `_PlayerEnterWormhole` @0x646a3).
pub const GATE_REFUSAL_TICKS: u16 = 250;
/// A jump refused as too close or with no destination: 360
/// (`_HandlePlayer` @0x6b931, @0x6ba00).
pub const JUMP_REFUSAL_TICKS: u16 = 360;
/// The capture's old ship retained or lost: 360 (`_DoShipCapture`
/// @0x41aa1).
pub const OLD_SHIP_TICKS: u16 = 360;

/// How long the message for `refusal` stays: [`JUMP_REFUSAL_TICKS`] too
/// close or with no destination, 240 short of fuel (`_HandlePlayer`
/// @0x6baac), and [`MESSAGE_TICKS`] for the engine's own refusals.
#[must_use]
pub fn jump_refusal_ticks(refusal: &JumpRefusal) -> u16 {
    match refusal {
        JumpRefusal::TooClose { .. } | JumpRefusal::NoDestination => JUMP_REFUSAL_TICKS,
        JumpRefusal::NoFuel { .. } | JumpRefusal::Landed | JumpRefusal::Disabled => MESSAGE_TICKS,
    }
}

/// How long the message for `refusal` stays: [`TOO_FAR_TICKS`] too far
/// or too fast, [`LANDING_TICKS`] denied or not landable, and
/// [`MESSAGE_TICKS`] for the engine's own refusals.
#[must_use]
pub fn landing_refusal_ticks(refusal: &LandingRefusal) -> u16 {
    match refusal {
        LandingRefusal::TooFar { .. } | LandingRefusal::TooFast { .. } => TOO_FAR_TICKS,
        LandingRefusal::Denied { .. } | LandingRefusal::NotLandable { .. } => LANDING_TICKS,
        LandingRefusal::Jumping | LandingRefusal::Disabled | LandingRefusal::NoStellars => {
            MESSAGE_TICKS
        }
    }
}

/// How long the message for `assigned` stays: [`MESSAGE_TICKS`] for a
/// capture made an escort (`_DoPlunderDialog` @0x9482a), and
/// [`OLD_SHIP_TICKS`] for the old ship after "Use As My Ship".
#[must_use]
pub fn assigned_ticks(assigned: Assigned) -> u16 {
    match assigned {
        Assigned::Escort => MESSAGE_TICKS,
        Assigned::MyShip | Assigned::Abandoned => OLD_SHIP_TICKS,
    }
}

/// The keys flight holds: the original's defaults, Alt, which turns the
/// weapon select key back, and Command, which the jettison key needs.
const FLIGHT_KEYS: [Key; 8] = [
    Key::Up,
    Key::Left,
    Key::Right,
    Key::Down,
    FIRE_KEY,
    SECONDARY_KEY,
    Key::Alt,
    Key::Command,
];

/// The primary fire key: the original's default (`fireKey0`, `_loadKeys`
/// @0xcd5f8).
pub const FIRE_KEY: Key = Key::Space;
/// The secondary fire key: the original's default (`fireKey1`).
pub const SECONDARY_KEY: Key = Key::Control;
/// The secondary weapon select key: the original's default
/// (`weapSelect`); with Alt (Option) it selects the one before.
pub const SELECT_KEY: Key = Key::Char('w');
/// The target select key: the original's default (`targSel`), which
/// picks the next ship in turn.
pub const TARGET_KEY: Key = Key::Tab;
/// The closest target key: the original's default (`closeTarg`), which
/// picks the nearest threat, and with Alt (Option) the nearest ship.
pub const NEAREST_KEY: Key = Key::Char('r');

/// The land key: the original's default (`STR#` 129, and `STR#` 2002
/// #25).
pub const LAND_KEY: Key = Key::Char('l');
/// The galaxy map key: the original's documented default (`Keys.nib`'s
/// `mapKey`; `STR#` 2002 #26-28 name a map key).
pub const MAP_KEY: Key = Key::Char('m');
/// The hyperspace jump key: the original's documented default
/// (`Keys.nib`'s `jumpKey`).
pub const JUMP_KEY: Key = Key::Char('j');
/// The board key: the original's default (`Keys.nib`'s `boardKey`), one
/// attempt a press.
pub const BOARD_KEY: Key = Key::Char('b');
/// The hail key: the original's default (`Keys.nib`'s `hailKey`), one
/// hail a press.
pub const HAIL_KEY: Key = Key::Char('y');
/// The jettison key: the original's default (`dumpCargo`, `_keyPrefs`
/// +0x1e, keycode 0x28, `_loadKeys` @0xce33d-0xce360), which jettisons
/// only with Command held (`_HandlePlayer` @0x6fd54-0x6fdaf): everything,
/// or with Option (Alt) also held, all but the mission cargo.
pub const JETTISON_KEY: Key = Key::Char('k');
/// The escorts' attack key: the original's default (`Keys.nib`'s
/// `escortCmdKey0`, "Attack Target").
pub const ESCORT_ATTACK_KEY: Key = Key::Char('f');
/// The escorts' defend key (`escortCmdKey1`, "Defend Me").
pub const ESCORT_DEFEND_KEY: Key = Key::Char('d');
/// The escorts' hold key (`escortCmdKey2`, "Hold Position").
pub const ESCORT_HOLD_KEY: Key = Key::Char('v');
/// The escorts' recall key (`escortCmdKey3`, "Recall: (option = dock)").
pub const ESCORT_RECALL_KEY: Key = Key::Char('c');
/// What closes the escort menu besides its own key: Return
/// (`ackComm`).
pub const MENU_CLOSE_KEY: Key = Key::Enter;

/// `STR#` 2002 #284: a mission failed as its cargo was jettisoned, or as
/// one of its special ships was lost.
pub const MISSION_FAILED: &str = "Mission failed.";
/// `STR#` 2002 #289: everything was jettisoned.
pub const CARGO_JETTISONED: &str = "Cargo jettisoned.";
/// `STR#` 2002 #290: all but the mission cargo was jettisoned.
pub const NON_MISSION_CARGO_JETTISONED: &str = "Non-mission cargo jettisoned.";

/// `STR#` 2002 #286: a scan found a mission's cargo and failed it.
pub const MISSION_SCANNED: &str = "Your ship has been scanned - mission failed.";
/// `STR#` 2002 #285: a mission's deadline ran out in flight and failed it.
pub const TIME_LIMIT_EXCEEDED: &str = "Time limit exceeded - mission failed.";
/// `STR#` 2002 #376-#379, a space between each: what follows the
/// scanner's and the ship's names when a scan finds smuggling.
pub const SMUGGLING_DETECTED: &str =
    "your attempt to smuggle illegal cargo through this system has been detected";
/// `STR#` 2002 #380: what comes before the fine.
pub const FINED: &str = "You have been fined";

/// What the message line says when a scan finds smuggling, as
/// `_ScanPlayer` builds it (@0x7e7ce-0x7ea9b): "`scanner`:  `ship`, ...
/// has been detected.", then with a fine " You have been fined `fine`
/// credits." ("credit" for 1), the fine's thousands grouped. The
/// scanner is named by the drawn ship name of the mission whose cargo
/// the scan found, when it has one, whatever ship scanned, and otherwise
/// by its ship class (@0x7e7ce-0x7e7ef; see [`SimMessage::Smuggling`]).
#[must_use]
pub fn smuggling_message(scanner: &str, ship: &str, fine: i64) -> String {
    let detected = format!("{scanner}:  {ship}, {SMUGGLING_DETECTED}.");
    if fine == 0 {
        return detected;
    }
    let credits = if fine > 1 { "credits" } else { "credit" };
    format!("{detected} {FINED} {} {credits}.", grouped(fine))
}

/// What the message line says of `message`, a session's message other
/// than an arrival's, the names read from `session`: none for an
/// arrival's, which [`arrival_lead`] leads the arrival message with.
fn sim_message_text(message: &SimMessage, session: &Session) -> Option<String> {
    let text = match message {
        SimMessage::Arrived(_) | SimMessage::ExitedHypergate(_) | SimMessage::PassedWormhole(_) => {
            return None;
        }
        SimMessage::MissionFailed | SimMessage::MissionShipLost => MISSION_FAILED,
        SimMessage::CargoJettisoned => CARGO_JETTISONED,
        SimMessage::NonMissionCargoJettisoned => NON_MISSION_CARGO_JETTISONED,
        SimMessage::MissionScanned => MISSION_SCANNED,
        SimMessage::DeadlineMissed => TIME_LIMIT_EXCEEDED,
        SimMessage::Smuggling {
            scanner,
            ship_name,
            fine,
        } => {
            return Some(smuggling_message(
                ship_name
                    .as_deref()
                    .or_else(|| session.ship_name(*scanner))
                    .unwrap_or_default(),
                session.pilot().ship_name().unwrap_or_default(),
                *fine,
            ));
        }
    };
    Some(text.to_owned())
}

/// `STR#` 2002 #53.
pub const NO_RESPONSE: &str = "No response.";
/// `STR#` 2002 #54.
pub const HAIL_IN_HYPERSPACE: &str = "Unable to send hail - target ship is entering hyperspace.";
/// `STR#` 2002 #3, without the ship's name the original adds.
pub const ENERGY_TRANSFER: &str = "Energy transfer complete";
/// `STR#` 2002 #384.
pub const REPAIRS_COMPLETE: &str = "Repairs complete";

/// What the player is told when `refusal` stops a hail: the original's
/// words for it, or nothing when there is no ship to hail.
#[must_use]
pub fn hail_refusal_message(refusal: HailRefusal) -> Option<&'static str> {
    match refusal {
        HailRefusal::NoTarget => None,
        HailRefusal::NoResponse => Some(NO_RESPONSE),
        HailRefusal::InHyperspace => Some(HAIL_IN_HYPERSPACE),
    }
}

/// `STR#` 2002 #126: a mission's special ship boarded on the assist
/// path.
pub const TARGET_BOARDED: &str = "Target ship has been boarded.";

/// What the comm line says as a mission's special ship of `Flags` 0x0008
/// is boarded on the assist path, as `_HandlePlayerBoardAttempt` builds
/// it (@0x655ae-0x65663): "`ship`:  Energy transfer complete,
/// `player_ship`."
#[must_use]
pub fn refuelled_message(ship: &str, player_ship: &str) -> String {
    format!("{ship}:  {ENERGY_TRANSFER}, {player_ship}.")
}

/// `STR#` 2002 #107: the article before a cargo's name as a boarding
/// picks it up.
pub const THE: &str = "the";

/// What the comm line says as a `PickupMode` 2 mission's cargo, its
/// `STR#` 4001 name `cargo`, is picked up by boarding its special ship,
/// as `_HandlePlayerBoardAttempt` builds it (@0x653fb-0x65543):
/// [`RETRIEVED`] and a space (two in all), then [`THE`] and a space
/// unless the name starts with `*`, the name without its `*`, a space,
/// and [`FROM_THIS_SHIP`].
#[must_use]
pub fn retrieved_message(cargo: &str) -> String {
    match cargo.strip_prefix('*') {
        Some(name) => format!("{RETRIEVED} {name} {FROM_THIS_SHIP}"),
        None => format!("{RETRIEVED} {THE} {cargo} {FROM_THIS_SHIP}"),
    }
}

/// What the comm line says of `line`, the names read from `session`.
fn mission_board_message(line: MissionBoardLine, session: &Session) -> String {
    match line {
        MissionBoardLine::Boarded => TARGET_BOARDED.to_owned(),
        MissionBoardLine::Refuelled { ship } => refuelled_message(
            session.ship_name(ship).unwrap_or_default(),
            session.pilot().ship_name().unwrap_or_default(),
        ),
        MissionBoardLine::Retrieved { cargo } => retrieved_message(
            &cargo
                .and_then(|cargo| u8::try_from(cargo).ok())
                .and_then(|cargo| session.cargo_name(cargo))
                .unwrap_or_default(),
        ),
    }
}

/// What the player is told when a ship it asked for help is done:
/// "<ship>:  Energy transfer complete." or "<ship>:  Repairs complete."
/// (the original names the player's ship after the energy transfer; it
/// has no name here).
#[must_use]
pub fn comm_message(note: &CommNote) -> String {
    let done = match note.done {
        Help::Refuel => ENERGY_TRANSFER,
        Help::Repair => REPAIRS_COMPLETE,
    };
    format!("{}:  {done}.", note.comm_name)
}
/// The stellar slot keys: each pair selects the stellar in that slot of
/// the system's navigation defaults, from the first. The original reads
/// keys 1-4 (Mac keycodes 0x12-0x15) and F1-F4 (`_loadKeys` @0xce6c8
/// hard-codes 0x7a, 0x78, 0x63, 0x76 at +0x34..+0x3a) in `_HandlePlayer`
/// @0x69cf6-0x69f17. While the escort menu is open, 1-4 pick its groups
/// instead ([`GROUP_KEYS`]); F1-F4 always pick a stellar.
pub const STELLAR_SLOT_KEYS: [[Key; 2]; 4] = [
    [Key::Char('1'), Key::Function(1)],
    [Key::Char('2'), Key::Function(2)],
    [Key::Char('3'), Key::Function(3)],
    [Key::Char('4'), Key::Function(4)],
];
/// The nearest stellar key: F5 (Mac keycode 0x60), which the original
/// reads beside the slot keys (@0x69cf6-0x69f17) and which selects the
/// stellar its land key would with nothing selected. Its Cmd-5 twin has
/// no key here.
pub const NEAREST_STELLAR_KEY: Key = Key::Function(5);
/// The Nav Off key, which clears the navigation target: the original's
/// default, backquote (`_loadKeys` @0xcd5f8 `navOff` 0x32, handled in
/// `_HandlePlayer` @0x69a17-0x69b63), the only key that clears it. While
/// the dev overlay is built in, backquote toggles the overlay instead and
/// never reaches flight.
pub const NAV_OFF_KEY: Key = Key::Char('`');

/// The Hyper Select key, which cycles the hyperspace destination through
/// the systems the current one links to: the original's default, `\`
/// (`Keys.nib`'s `hyperSel`, "Hyper Select:", default keycode 0x2a in
/// `_loadKeys`). H is the original's separate Hyperspace Mode key.
pub const HYPER_SELECT_KEY: Key = Key::Char('\\');

/// `STR#` 2002 #29.
pub const NO_DESTINATION: &str =
    "You have to select a destination before you can start a hyperspace jump.";
/// `STR#` 2002 #42.
pub const TOO_CLOSE: &str =
    "Can't initiate hyperspace jump - not yet far enough away from system center.";
/// `STR#` 2002 #10.
pub const NO_FUEL: &str = "Insufficient energy for hyperspace jump.";
/// Not the original's, which never flies a landed ship: worded after
/// `STR#` 2002 #42 and #73 ("Disengage cloaking device first.").
pub const TAKE_OFF_FIRST: &str = "Can't initiate hyperspace jump - take off first.";
/// Not the original's, whose disabled ship takes no keys: worded after
/// `STR#` 2002 #42.
pub const JUMP_DISABLED: &str = "Can't initiate hyperspace jump - your ship is disabled.";

/// `STR#` 2002 #44. The original picks #43, #44 or #45 ("Entering the",
/// "Jumping into the", "Arriving in the") at random; this port always
/// says #44, matching none of its random rolls.
pub const JUMPING_INTO: &str = "Jumping into the";
/// `STR#` 2002 #48.
pub const SYSTEM_ON: &str = "system on";

/// `STR#` 2002 #46: coming out of a hypergate.
pub const EXITING_HYPERGATE: &str = "Exiting hypergate in the";
/// `STR#` 2002 #47: passing through a wormhole.
pub const PASSING_WORMHOLE: &str = "Passing through a wormhole into the";

/// What the message line says on arriving from a jump in the system
/// named `system` on `date`, as the original's `HandlePlayer` builds it,
/// adding [`NO_STELLARS`] when the system has no stellars: the
/// [`JUMPING_INTO`] reading of [`arrival_message_with`]. The original's
/// fighters-abandoned suffix and message-buoy override are left out.
#[must_use]
pub fn arrival_message(system: &str, date: &str, stellars: bool) -> String {
    arrival_message_with(JUMPING_INTO, system, date, stellars)
}

/// What the message line says on arriving in the system named `system` on
/// `date`, led by `lead`: [`JUMPING_INTO`] from a jump, and
/// [`EXITING_HYPERGATE`] or [`PASSING_WORMHOLE`] through a gate, as
/// `_PlayerEnterHypergate` (@0x63c45-0x63dbe) and `_PlayerEnterWormhole`
/// (@0x6444c-0x6457d) build it; [`NO_STELLARS`] follows when the system
/// has no stellars.
#[must_use]
pub fn arrival_message_with(lead: &str, system: &str, date: &str, stellars: bool) -> String {
    let mut text = format!("{lead} {system} {SYSTEM_ON} {date}.");
    if !stellars {
        text.push(' ');
        text.push_str(NO_STELLARS);
    }
    text
}

/// The lead of the message line on the arrival `message` raises: none for
/// a message of another kind.
fn arrival_lead(message: &SimMessage) -> Option<&'static str> {
    match message {
        SimMessage::Arrived(_) => Some(JUMPING_INTO),
        SimMessage::ExitedHypergate(_) => Some(EXITING_HYPERGATE),
        SimMessage::PassedWormhole(_) => Some(PASSING_WORMHOLE),
        SimMessage::MissionFailed
        | SimMessage::MissionShipLost
        | SimMessage::CargoJettisoned
        | SimMessage::NonMissionCargoJettisoned
        | SimMessage::MissionScanned
        | SimMessage::DeadlineMissed
        | SimMessage::Smuggling { .. } => None,
    }
}

/// `STR#` 2002 #50: no system picked on the hypergate map, or none the
/// gate leads to.
pub const HYPERGATE_CANCELLED: &str = "Hypergate jump cancelled.";
/// `STR#` 2002 #74.
pub const HYPERGATE_ENERGIZED: &str = "Hypergate is energized";
/// `STR#` 2002 #75.
pub const HYPERGATE_ONLINE: &str = "Hypergate is online";
/// `STR#` 2002 #80.
pub const BEGIN_APPROACH: &str = "Begin initial approach.";
/// `STR#` 2002 #81: clearance denied at a hypergate
/// (`_HandlePlayerDockRequest` @0x67ee0-0x67eea).
pub const HYPERGATE_DENIED: &str = "Hypergate usage denied.";
/// `STR#` 2002 #84.
pub const UNABLE_TO: &str = "Your ship is unable to";
/// `STR#` 2002 #85.
pub const HYPERGATE_OFFLINE: &str = "enter this hypergate - it is offline.";
/// `STR#` 2002 #86.
pub const WORMHOLE_TOO_HOT: &str = "enter this wormhole - the radiation levels are too extreme.";

/// What the player is told when L requests clearance at a hypergate and is
/// cleared: [`HYPERGATE_ENERGIZED`] when `energized`, otherwise
/// [`HYPERGATE_ONLINE`] (the original rolls `Rand(2)`, @0x67a8b-0x67aa5),
/// then ". " and [`BEGIN_APPROACH`] (@0x67b78-0x67c05). The original adds
/// the pilot's name after the first half on another roll; that is left
/// out.
#[must_use]
pub fn hypergate_clearance_message(energized: bool) -> String {
    let first = if energized {
        HYPERGATE_ENERGIZED
    } else {
        HYPERGATE_ONLINE
    };
    format!("{first}. {BEGIN_APPROACH}")
}

/// Why the ship cannot enter a gate of `kind`, as the original says it
/// (@0x67cda-0x67d3c): [`UNABLE_TO`] then [`HYPERGATE_OFFLINE`] or
/// [`WORMHOLE_TOO_HOT`]. A gate that cannot be landed on says it, and so
/// does a wormhole that leads nowhere.
#[must_use]
pub fn gate_refusal_message(kind: GateKind) -> String {
    let why = match kind {
        GateKind::Hypergate => HYPERGATE_OFFLINE,
        GateKind::Wormhole => WORMHOLE_TOO_HOT,
    };
    format!("{UNABLE_TO} {why}")
}

/// `STR#` 2002 #49.
pub const NO_STELLARS: &str = "No stellar objects present.";
/// `STR#` 2002 #67.
pub const TOO_FAR_STATION: &str = "You're too far away to dock at this station.";
/// `STR#` 2002 #68.
pub const TOO_FAR_PLANET: &str = "You're too far away to land on this planet.";
/// `STR#` 2002 #71.
pub const TOO_FAST_STATION: &str = "You're moving too fast to dock at this station.";
/// `STR#` 2002 #72.
pub const TOO_FAST_PLANET: &str = "You're moving too fast to land on this planet.";
/// `STR#` 2002 #82.
pub const DOCKING_DENIED: &str = "Docking request denied.";
/// `STR#` 2002 #83.
pub const LANDING_DENIED: &str = "Landing request denied.";
/// `STR#` 2002 #89: why a ship cannot dock at a station.
pub const HOSTILE_STATION: &str = "The station's hull integrity is too unstable.";
/// `STR#` 2002 #90: why a ship cannot land on a planet.
pub const HOSTILE_PLANET: &str = "The planet's environment is too hostile.";
/// Not the original's, which takes no keys during a jump: worded after
/// `STR#` 2002 #54 ("Unable to send hail - target ship is entering
/// hyperspace.").
pub const IN_HYPERSPACE: &str = "Unable to land - your ship is in hyperspace.";
/// Not the original's, whose disabled ship takes no keys: worded as
/// [`IN_HYPERSPACE`] is.
pub const LAND_DISABLED: &str = "Unable to land - your ship is disabled.";

/// `STR#` 2002 #76.
pub const DOCKMASTER_READS_YOU: &str = "dockmaster reads you";
/// `STR#` 2002 #78.
pub const TRAFFIC_CONTROL_READS_YOU: &str = "traffic control reads you";
/// `STR#` 2002 #95.
pub const CLEARED_TO_DOCK: &str = "you're cleared to dock.";
/// `STR#` 2002 #96.
pub const YOU_ARE_CLEARED_TO_DOCK: &str = "You are cleared to dock.";
/// `STR#` 2002 #98.
pub const CLEARED_TO_LAND: &str = "you're cleared to land.";
/// `STR#` 2002 #99.
pub const YOU_ARE_CLEARED_TO_LAND: &str = "You are cleared to land.";

/// What the player is told when L requests clearance at the stellar
/// `name`, a station or a planet, and `clearance` is the reply. Only the
/// pieces are the original's (`STR#` 2002); how they are joined is a
/// reconstruction:
///
/// - granted: "{name} traffic control reads you, you're cleared to land."
///   (#78, #98), or at a station "{name} dockmaster reads you, you're
///   cleared to dock." (#76, #95);
/// - uninhabited, with no traffic control to answer: "You are cleared to
///   land." (#99) or "You are cleared to dock." (#96);
/// - denied: "Landing request denied." (#83) or "Docking request denied."
///   (#82).
#[must_use]
pub fn clearance_message(name: &str, station: bool, clearance: Clearance) -> String {
    let (reads_you, cleared, you_are_cleared, denied) = if station {
        (
            DOCKMASTER_READS_YOU,
            CLEARED_TO_DOCK,
            YOU_ARE_CLEARED_TO_DOCK,
            DOCKING_DENIED,
        )
    } else {
        (
            TRAFFIC_CONTROL_READS_YOU,
            CLEARED_TO_LAND,
            YOU_ARE_CLEARED_TO_LAND,
            LANDING_DENIED,
        )
    };
    match clearance {
        Clearance::Granted => format!("{name} {reads_you}, {cleared}"),
        Clearance::NoTrafficControl => you_are_cleared.to_owned(),
        Clearance::Denied => denied.to_owned(),
    }
}

/// What the player is told when `refusal` stops a landing: the original's
/// words for it, for a station or a planet.
#[must_use]
pub fn refusal_message(refusal: &LandingRefusal) -> &'static str {
    let pick = |station: bool, at_station, on_planet| {
        if station { at_station } else { on_planet }
    };
    match *refusal {
        LandingRefusal::Jumping => IN_HYPERSPACE,
        LandingRefusal::Disabled => LAND_DISABLED,
        LandingRefusal::NoStellars => NO_STELLARS,
        LandingRefusal::TooFar { station, .. } => pick(station, TOO_FAR_STATION, TOO_FAR_PLANET),
        LandingRefusal::NotLandable { station, .. } => {
            pick(station, HOSTILE_STATION, HOSTILE_PLANET)
        }
        LandingRefusal::Denied { station, .. } => pick(station, DOCKING_DENIED, LANDING_DENIED),
        LandingRefusal::TooFast { station, .. } => pick(station, TOO_FAST_STATION, TOO_FAST_PLANET),
    }
}

/// Where a ship is drawn `alpha` of the way from `from` to `to`.
fn shown_position(from: &ShipState, to: &ShipState, alpha: f32) -> Point {
    let (from, to) = (from.position, to.position);
    Point::new(
        (to.x - from.x).mul_add(alpha, from.x),
        (to.y - from.y).mul_add(alpha, from.y),
    )
}

/// Which way a ship is drawn facing `alpha` of the way, the short way
/// round, from `from` to `to`.
fn shown_heading(from: &ShipState, to: &ShipState, alpha: f32) -> f32 {
    normalized(shortest_turn(from.heading, to.heading).mul_add(alpha, from.heading))
}

/// How `npc`'s sprite is tinted: `None` while it waits hidden at the gate
/// it comes out of; white with the alpha of its [`emerge_level`] over
/// its last [`FADE_TICKS`](nova_sim::traffic::emerge::FADE_TICKS) there;
/// plain white otherwise.
///
/// The original (`_HandleShipDisplay` @0x2c090-0x2c18e) fades the hull in
/// at 16-bit depth by a destination factor of twice the ticks left and a
/// bias of four times them, at most 32. That translucent blit was not
/// traced; this draws the ship's alpha rising from none to full, as the
/// cloak's fade does. An NPC gets no screen flash, as the player's own
/// gate arrival does (see [`jump`](super::jump)).
fn npc_tint(npc: &Npc) -> Option<Color> {
    let Mode::Emerging { ticks_left } = npc.mode else {
        return Some(Color::WHITE);
    };
    let level = u16::from(emerge_level(ticks_left)?);
    let full = u16::from(FULL_LEVEL);
    let alpha = u8::try_from((level * 255 + full / 2) / full).unwrap_or(u8::MAX);
    Some(Color::rgba(255, 255, 255, alpha))
}

/// `STR#` 2002 #130: the target cannot be boarded.
pub const CANT_BOARD: &str = "You can't board this ship.";
/// `STR#` 2002 #131: the player is not over the target.
pub const NOT_CLOSE_ENOUGH: &str = "You're not close enough to board this ship.";
/// `STR#` 2002 #132: the player moves too fast against the target.
pub const TOO_FAST_TO_BOARD: &str = "You're moving too fast to board this ship.";
/// The Bible's words for a crew that repels boarders (`düde` `Booty` 0);
/// the original has no such string.
pub const REPELLED: &str = "You were repelled while attempting to board this ship.";
/// `STR#` 2002 #113: the self-destruct went off.
pub const SELF_DESTRUCT: &str = "Oops! You tripped this ship's security self-destruct mechanism.";
/// `STR#` 2002 #114: the hold had no room for the cargo.
pub const NO_CARGO_STORED: &str =
    "You couldn't store any of the cargo you plundered from this ship.";
/// `STR#` 2002 #117: no ammunition could be taken.
pub const NO_AMMO_STORED: &str = "You couldn't store any of the ammo you plundered from this ship.";
/// `STR#` 2002 #6: the tank had no room for the energy.
pub const NO_ENERGY_STORED: &str =
    "You couldn't store any of the energy you transferred from this ship.";
/// `STR#` 2002 #5: all the energy was stored, and the tank is not full.
pub const ALL_ENERGY_STORED: &str =
    "You transferred all of this ship's energy to your reactors and batteries.";
/// `STR#` 2002 #4: the energy filled the tank.
pub const ENERGY_FILLED: &str =
    "You filled your reactors and batteries with energy from this ship.";
/// `STR#` 2002 #125: the capture failed.
pub const CAPTURE_FAILED: &str = "Your attempt to capture this ship was unsuccessful.";
/// `STR#` 2002 #124: the fleet is full.
pub const FLEET_FULL: &str = "You already have the maximum possible number of escorts.";
/// `STR#` 2002 #123: the ship captured joined the fleet.
pub const ASSIGNED_ESCORT: &str = "You assigned this ship to your fleet of escorts.";
/// `STR#` 2002 #304: the old ship joined the fleet after "Use As My Ship".
pub const RETAINED_OLD_SHIP: &str = "You retained your old ship as an escort.";
/// `STR#` 2002 #305: no slot was free for the old ship.
pub const LOST_OLD_SHIP: &str = "You were unable to retain your old ship as an escort.";

/// What the player is told when `refusal` stops a boarding: the
/// original's words for it, or nothing.
#[must_use]
pub fn board_refusal_message(refusal: BoardRefusal) -> Option<&'static str> {
    match refusal {
        BoardRefusal::CantBoard => Some(CANT_BOARD),
        BoardRefusal::TooFar => Some(NOT_CLOSE_ENOUGH),
        BoardRefusal::TooFast => Some(TOO_FAST_TO_BOARD),
        BoardRefusal::NoTarget | BoardRefusal::Misaligned => None,
    }
}

/// What the player is told of a press in the plunder dialog that did
/// `taken`, in the original's words (`_DoPlunderDialog`): the cargo's
/// `good` and the ammunition's `outfit` by name, and the energy by
/// whether it filled the tank; nothing for a capture awaiting its
/// assignment, an abort or a press that did nothing.
#[must_use]
pub fn plunder_message(taken: Taken, good: Option<&str>, outfit: Option<&str>) -> Option<String> {
    let text = match taken {
        Taken::Cargo { stored: 0, .. } => NO_CARGO_STORED.to_owned(),
        Taken::Cargo { stored, .. } => {
            let tons = if stored == 1 { "ton" } else { "tons" };
            let good = good.unwrap_or("cargo");
            format!("You salvaged {stored} {tons} of {good} from this ship.")
        }
        Taken::Credits(credits) => format!("You stole all the {credits} credits from this ship."),
        Taken::Ammo { count: 0, .. } => NO_AMMO_STORED.to_owned(),
        Taken::Ammo { count, .. } => {
            let outfit = outfit.unwrap_or("rounds");
            format!("You salvaged {count} {outfit} from this ship.")
        }
        Taken::Energy { stored: 0, .. } => NO_ENERGY_STORED.to_owned(),
        Taken::Energy { full: true, .. } => ENERGY_FILLED.to_owned(),
        Taken::Energy { .. } => ALL_ENERGY_STORED.to_owned(),
        Taken::Tripped => SELF_DESTRUCT.to_owned(),
        Taken::CaptureFailed => CAPTURE_FAILED.to_owned(),
        Taken::FleetFull => FLEET_FULL.to_owned(),
        Taken::Escorted => ASSIGNED_ESCORT.to_owned(),
        Taken::Captured | Taken::Aborted | Taken::Nothing => return None,
    };
    Some(text)
}

/// `STR#` 2002 #106: a grant's message starts so.
pub const RETRIEVED: &str = "You retrieved ";
/// `STR#` 2002 #393: the article before one outfit of a consonant.
pub const ARTICLE_A: &str = "a";
/// `STR#` 2002 #394: the article before one outfit of a vowel.
pub const ARTICLE_AN: &str = "an";
/// `STR#` 2002 #108: a grant's message ends so.
pub const FROM_THIS_SHIP: &str = "from this ship.";

/// What the player is told of a grant of `count` outfits named
/// `lc_name`, or `lc_plural` for more than one, as `_DoPlunderDialog`
/// says it (@0x9322b-0x933ce): [`RETRIEVED`]; for one, [`ARTICLE_AN`]
/// when the name's first letter, lowercased, is a vowel, else
/// [`ARTICLE_A`]; for two to ten the count in words ([`NUMBER_WORDS`]),
/// and above that in digits; the name; and [`FROM_THIS_SHIP`].
#[must_use]
pub fn grant_message(count: u16, lc_name: &str, lc_plural: &str) -> String {
    let (number, name) = if count == 1 {
        let vowel = lc_name
            .chars()
            .next()
            .is_some_and(|first| "aeiou".contains(first.to_ascii_lowercase()));
        let article = if vowel { ARTICLE_AN } else { ARTICLE_A };
        (article.to_owned(), lc_name)
    } else {
        let words = usize::from(count)
            .checked_sub(1)
            .and_then(|index| NUMBER_WORDS.get(index));
        let number = words.map_or_else(|| count.to_string(), |word| (*word).to_owned());
        (number, lc_plural)
    };
    format!("{RETRIEVED}{number} {name} {FROM_THIS_SHIP}")
}

/// The names `session` gives `good` and `outfit`: one rule for the
/// plunder dialog ([`FlightView::plunder_shown`]) and the messages
/// ([`FlightView::plunder`]).
fn names(
    session: &Session,
    good: Option<Good>,
    outfit: Option<OutfitId>,
) -> (Option<&str>, Option<&str>) {
    (
        good.and_then(|good| session.good_name(good)),
        outfit.and_then(|outfit| session.outfit_name(outfit)),
    )
}

/// What the player is told of an assignment that did `assigned`.
#[must_use]
pub fn assigned_message(assigned: Assigned) -> &'static str {
    match assigned {
        Assigned::Escort => ASSIGNED_ESCORT,
        Assigned::MyShip => RETAINED_OLD_SHIP,
        Assigned::Abandoned => LOST_OLD_SHIP,
    }
}

/// What the player is told when `count` hired escorts defected for want
/// of pay: `STR#` 2002 #302 for one, #303 for more.
#[must_use]
pub fn defection_message(count: u32) -> &'static str {
    if count == 1 {
        DEFECTED_ONE
    } else {
        DEFECTED_SOME
    }
}

/// What the player is told of each of `notes`, in order.
#[must_use]
pub fn pay_notes_message(notes: &[PayNote]) -> Vec<String> {
    notes
        .iter()
        .map(|&PayNote::Defected(count)| defection_message(count).to_owned())
        .collect()
}

/// What the player is told of each of `notes`, in order, each for
/// [`MESSAGE_TICKS`].
fn pay_said(notes: &[PayNote]) -> Vec<(String, u16)> {
    pay_notes_message(notes)
        .into_iter()
        .map(|text| (text, MESSAGE_TICKS))
        .collect()
}

/// What the player is told when `refusal` stops a jump: the original's
/// words for it.
#[must_use]
pub fn jump_refusal_message(refusal: &JumpRefusal) -> &'static str {
    match refusal {
        JumpRefusal::NoDestination => NO_DESTINATION,
        JumpRefusal::TooClose { .. } => TOO_CLOSE,
        JumpRefusal::NoFuel { .. } => NO_FUEL,
        JumpRefusal::Landed => TAKE_OFF_FIRST,
        JumpRefusal::Disabled => JUMP_DISABLED,
    }
}

/// A [`Chance`] shared by whoever holds a copy: the app's one source of
/// randomness, handed to each flight. The default never fires and always
/// rolls the first outcome.
#[derive(Clone)]
pub struct SharedChance(Rc<RefCell<dyn Chance>>);

impl SharedChance {
    /// The source `chance`, shared.
    #[must_use]
    pub fn new(chance: Rc<RefCell<dyn Chance>>) -> Self {
        Self(chance)
    }
}

impl Default for SharedChance {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(NeverFires)))
    }
}

impl std::fmt::Debug for SharedChance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Chance")
    }
}

impl Chance for SharedChance {
    fn fires(&mut self, percent: u8) -> bool {
        self.0.borrow_mut().fires(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.borrow_mut().below(n)
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.0.borrow_mut().roll(sides)
    }
}

/// The player's ship in flight, reading the game data from `C`.
#[derive(Clone, Debug)]
pub struct FlightView<C> {
    /// What the screen reads: when it is built, and on each arrival.
    catalog: C,
    /// The flight, or why it could not start.
    session: Result<Session, String>,
    /// The session's system, laid out; `None` when the session failed.
    scene: Option<SystemScene>,
    /// The player ship's sheet, or why it cannot be shown.
    sheet: Result<ShipSheet, String>,
    /// The class `sheet` and `status_bar` were read for; `None` when the
    /// session failed.
    sheet_ship: Option<ShipId>,
    /// The HUD's status bar, or why it cannot be shown.
    status_bar: Result<StatusBar, String>,
    /// Turns frame times into simulation steps.
    clock: FixedStep,
    /// The player's ship as it was a step before the session's.
    previous: ShipState,
    /// How far the display is from `previous` to the session's ship.
    alpha: f32,
    /// Time since the view opened, which drives the stellars' animations
    /// and the running lights' blinking. It stops while the map is open.
    elapsed: Duration,
    /// The flight keys held down.
    held: HashSet<Key>,
    /// The stellar landed on, until the router takes it.
    pending_landing: Option<StellarId>,
    /// The message line's one slot (`_DisplayComm`'s 0x3b565c): a new
    /// message replaces the old and its time. Its ticks count the
    /// flight's, so they stop while the map is open and while the router
    /// holds flight under a dialog, as `_HandleComm` (@0x301b9) counts
    /// only in the flight loop. The original's `_HandlePlayerDockRequest`
    /// counts one tick more just before landing (@0x66da5), which no
    /// player sees.
    message: Option<Message>,
    /// The port's own overlays drawn.
    overlays: FlightOverlays,
    /// Whether a boarding has opened that the router has not taken.
    pending_boarding: bool,
    /// The hail just answered, for the router.
    pending_hail: Option<HailView>,
    /// The mission info [`MISSION_INFO_KEY`] asked the screen be opened
    /// on, for the router.
    pending_mission_info: Option<MissionInfo>,
    /// The mission a person hailed or boarded just offered, for the
    /// router.
    pending_offer: Option<OfferPrompt>,
    /// The session's arrival messages taken before their arrival is
    /// caught up with, which leads them.
    pending_leads: Vec<SimMessage>,
    /// The options the comm dialog lists.
    hail_options: HailOptions,
    /// Who repels boarders, the capture odds and the capture roll.
    boarding_rule: Rc<dyn BoardingRule>,
    /// The course map, shown or not.
    map: GalaxyMap,
    /// Whether the course map is shown.
    map_open: bool,
    /// The jump's effect, while it holds flight: the streak and the
    /// fade-out, until the ship arrives.
    jump: Option<JumpEffect>,
    /// The rest of an arrival's effect, once the ship has arrived: the
    /// fade-in from white, or the white flash. Flight goes on under it.
    fade_in: Option<JumpEffect>,
    /// Whether jumps play their white fades, as the Hyperspace Effects
    /// preference says.
    hyperspace_effects: bool,
    /// The player's sound preferences, which say whether the Red Alert
    /// blinks its icon ([`SoundPrefs::alert_quiet`]), as the session is
    /// told.
    sound_prefs: SoundPrefs,
    /// `cïcn` 18000's width, which places the Red Alert icon, or `None`
    /// when it cannot be read, and the icon is not drawn.
    alert_width: Option<u32>,
    /// What each day's events and the traffic are rolled on.
    chance: SharedChance,
    /// What random running lights roll on.
    blink_rolls: HashedRolls,
    /// What the engine glow's flicker rolls on: seed 1, so it never
    /// mirrors random-mode lights, which roll on seed 0.
    glow_rolls: HashedRolls,
    /// How the NPCs decide.
    behaviour: Rc<dyn Behaviour>,
    /// When a ship in the fight is disabled.
    disable_rule: Rc<dyn DisableRule>,
    /// Which missiles the ships' point defence engages.
    defence_rule: Rc<dyn PointDefenceRule>,
    /// What the player's crimes do to its legal record.
    law: Rc<dyn LegalCode>,
    /// Each NPC ship type's sheet, or why it cannot be shown, read once.
    npc_sheets: BTreeMap<ShipId, Result<ShipSheet, String>>,
    /// Each NPC as it was a step before the session's.
    npc_previous: BTreeMap<NpcId, ShipState>,
    /// The weapons' and explosions' looks, read once.
    looks: Looks,
    /// What could not be read of the looks, until it is taken.
    unread_looks: Vec<Diagnostic>,
    /// Each NPC ship type's target card, read once.
    cards: BTreeMap<ShipId, TargetCard>,
    /// Each NPC government's target code, read once.
    codes: BTreeMap<GovtId, Option<String>>,
    /// The fight's explosions, debris and sounds.
    effects: Effects,
    /// What the effects are rolled on.
    effects_chance: SharedChance,
    /// What the HUD's text is measured by, if anything.
    metrics: Option<Metrics>,
    /// The escort menu, shut or open.
    escort_menu: EscortMenu,
    /// The escort menu's colours, read once.
    escort_colors: EscortMenuColors,
    /// When the HUD refreshes, and the radar target's blink.
    hud_cadence: HudCadence,
    /// What the radar showed at its last refresh, which it shows until the
    /// next under [`RuleKey::HudCadence`]'s engine reading; `None` before
    /// the first.
    radar_scene: Option<RadarScene>,
    /// What the nav area showed at its last refresh, and the nav mode it
    /// showed it for, which it shows until the next under
    /// [`RuleKey::HudCadence`]'s engine reading; `None` before the first.
    nav_scene: Option<(NavMode, NavDisplay)>,
    /// What the target panel showed at its last redraw, and what it drew
    /// it for, which it shows until the next under
    /// [`RuleKey::HudCadence`]'s engine reading; `None` before the first.
    target_scene: Option<(Option<Drawn>, Option<TargetPanel>)>,
    /// The radar's static this tick, a `ppat` ID, rolled on the effects'
    /// chance ([`radar::static_frame`]); `None` without.
    radar_static: Option<i16>,
}

/// A message on the message line.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Message {
    /// What it says.
    text: String,
    /// The flight's tick it was said on.
    said_at: u128,
    /// How many ticks it stays.
    ticks: u16,
}

/// What [`FlightView::catch_up`] redrew, from the session's events.
// Each flag is one kind of event, folded on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Redrawn {
    /// The ship changed: its sprite sheet followed.
    ship: bool,
    /// A system was entered afresh: it was laid out.
    system: bool,
    /// The ship was put at another stellar of the system shown.
    stellar: bool,
    /// The galaxy overlay changed: the course map was rebuilt.
    galaxy: bool,
    /// A rebuild point changed which stellars shown are dead since it:
    /// the stellars were drawn again.
    rebuilt: bool,
}

/// Draws each stellar of `scene` as `session` holds it: by whether its
/// galaxy holds it destroyed, its `DeadType` sheet or its `Type` sheet, as
/// the original's per-frame sprite tick picks them (story-runtime phase
/// 7), and, by the session's reading of [`RuleKey::DeadTypeFallback`],
/// graphic type 0's sheet for one with no `DeadType` sheet that has been
/// dead since the last rebuild point (story-runtime phase 9).
fn show_destroyed(scene: &mut SystemScene, session: &Session) {
    scene.set_destroyed(
        |id| StellarLook {
            destroyed: session.galaxy().destroyed(id),
            dead_since_rebuild: session.dead_since_rebuild(id),
        },
        session.rules().source_for(RuleKey::DeadTypeFallback),
    );
}

/// Text metrics, shared.
#[derive(Clone)]
struct Metrics(Rc<dyn TextMetrics>);

impl std::fmt::Debug for Metrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Metrics")
    }
}

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
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
    /// pilot docked at a stellar resumes landed there: the landing is
    /// reported once ([`FlightView::take_landing`]), with no sound, so the
    /// router shows the spaceport.
    pub fn with_pilot(catalog: C, pilot: Pilot) -> Self {
        let session = Session::fly(&catalog, pilot);
        Self::flying(catalog, session)
    }

    /// Begins a new pilot's game as [`Session::begin`] does, on the
    /// flight's chance, and shows on the course map any system it
    /// explored: for a pilot just created, before its first save. A
    /// session that failed begins nothing.
    pub fn begin(&mut self) {
        if let Ok(session) = &mut self.session {
            session.begin(&self.catalog, &mut self.chance);
            self.map.show_explored(session.pilot().explored());
        }
    }

    fn flying(catalog: C, session: Result<Session, StartError>) -> Self {
        let session = session.map_err(|err| err.to_string());
        let (scene, sheet, sheet_ship, status_bar) = match &session {
            Ok(session) => (
                Some({
                    let mut scene = SystemScene::load(&catalog, session.system());
                    show_destroyed(&mut scene, session);
                    scene
                }),
                catalog.ship_sheet(session.ship()),
                Some(session.ship()),
                hud::choose_status_bar(&catalog, session.interface_government()),
            ),
            Err(reason) => (None, Err(reason.clone()), None, Err(reason.clone())),
        };
        let previous = session
            .as_ref()
            .map(|session| *session.player())
            .unwrap_or_default();
        let mut map = GalaxyMap::course(&catalog);
        if let Ok(session) = &session {
            map.show_galaxy(session.galaxy());
            map.show_course(session.system(), &session.course_shown());
            map.show_explored(session.pilot().explored());
        }
        let pending_landing = session.as_ref().ok().and_then(Session::landed);
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
            jump: None,
            fade_in: None,
            hyperspace_effects: true,
            sound_prefs: SoundPrefs::default(),
            alert_width,
            session,
            scene,
            sheet,
            sheet_ship,
            status_bar,
            clock: FixedStep::new(),
            previous,
            alpha: 0.0,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
            pending_landing,
            message: None,
            overlays: FlightOverlays::default(),
            pending_boarding: false,
            pending_hail: None,
            pending_mission_info: None,
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
            metrics: None,
            escort_menu: EscortMenu::default(),
            escort_colors,
            hud_cadence: HudCadence::default(),
            radar_scene: None,
            nav_scene: None,
            target_scene: None,
            radar_static: None,
        }
    }

    /// The flight with every disputed rule reading as `rules` says
    /// ([`Session::with_rules`]). The stellars of the scene shown are
    /// drawn again by the new rules, which say how a destroyed stellar
    /// with no `DeadType` sheet draws, but not laid out again.
    #[must_use]
    pub fn with_rules(self, rules: Rulebook) -> Self {
        let mut view = Self {
            session: self.session.map(|session| session.with_rules(rules)),
            ..self
        };
        view.show_destroyed();
        view
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

    /// The flight with the HUD's text measured by `metrics`.
    #[must_use]
    pub fn with_metrics(self, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            metrics: Some(Metrics(metrics)),
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

    /// Hails the target ([`Session::open_hail`]), or says why not: a hail
    /// answered is held for the router ([`FlightView::take_hail`]), and a
    /// mission offered in its place too ([`FlightView::take_offer`]),
    /// each letting go of the flight keys; one accepted at once shows
    /// nothing.
    fn hail(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.open_hail(&self.catalog, &self.hail_options, &mut self.chance) {
            Ok(Hailed::Comm(view)) => {
                self.pending_hail = Some(view);
                self.held.clear();
            }
            Ok(Hailed::Offer(offer)) => self.hold_offer(offer),
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

    /// The mission info [`MISSION_INFO_KEY`] asked for the mission info
    /// screen on, once: the router opens the screen on it over the paused
    /// flight.
    pub fn take_mission_info_request(&mut self) -> Option<MissionInfo> {
        self.pending_mission_info.take()
    }

    /// Presses [`MISSION_INFO_KEY`], as `_HandlePlayer` does
    /// (@0x70405-0x705a8): with an active mission shown, asks the router
    /// for the mission info screen, letting go of the flight keys; with
    /// none, says #354 on the message line.
    fn ask_mission_info(&mut self) {
        let info = self.mission_info();
        if info.rows.is_empty() {
            self.say_for(NO_ACTIVE_MISSIONS, MESSAGE_TICKS);
        } else {
            self.pending_mission_info = Some(info);
            self.held.clear();
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
    /// not been read yet.
    fn read_npc_sheets(&mut self) {
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

    /// Covers the screen in the arrival's fade-in, then in the jump's
    /// fade-out, as far as each shows.
    fn draw_fades(&self, list: &mut DrawList) {
        for effect in [&self.fade_in, &self.jump].into_iter().flatten() {
            effect.draw_fade(list);
        }
    }

    /// Lets go of the flight keys and shows the course map.
    fn open_map(&mut self) {
        self.held.clear();
        self.map_open = true;
    }

    /// The map's input; a destination clicked on it becomes the session's
    /// course, which the map then shows.
    fn map_input(&mut self, input: &Input) {
        self.map.input(input);
        let Some(destination) = self.map.take_destination() else {
            return;
        };
        if let Ok(session) = &mut self.session {
            // A failed plot clears the course, which the map then shows.
            let _ = session.plot_course(destination);
            self.map
                .show_course(session.system(), &session.course_shown());
        }
    }

    /// Begins a jump, or shows why not. Starting it clears the message
    /// line only when it says [`TOO_CLOSE`] (`_HandlePlayer`
    /// @0x6b231-0x6b2a7, @0x6d601-0x6d67f).
    fn jump(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.begin_jump() {
            Ok(_) => {
                self.held.clear();
                self.clear_message(TOO_CLOSE);
                self.start_streak_if_jumping();
            }
            Err(refusal) => {
                self.say_for(jump_refusal_message(&refusal), jump_refusal_ticks(&refusal));
            }
        }
    }

    /// Starts the jump's streak, from the current system towards the next,
    /// once the session says the jump has begun, unless it is playing
    /// already.
    fn start_streak_if_jumping(&mut self) {
        let Ok(session) = &self.session else {
            return;
        };
        if let (None, Some(next)) = (&self.jump, session.jumping()) {
            let position = |id| session.star_map().position(id).unwrap_or_default();
            self.jump = Some(
                JumpEffect::toward(position(session.system()), position(next))
                    .with_fades(self.hyperspace_effects),
            );
        }
    }

    /// Whether the ship is braking and turning before a jump.
    fn preparing_jump(&self) -> bool {
        self.session
            .as_ref()
            .is_ok_and(|session| session.preparing_jump().is_some())
    }

    /// Ends the jump: the ship arrives in the next system, caught up with
    /// as [`FlightView::arrived`] says.
    fn arrive(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        if session.arrive(&self.catalog, &mut self.chance).is_some() {
            self.arrived();
        }
    }

    /// Catches up with the ship having arrived in a system
    /// ([`FlightView::catch_up`], which lays it out from the session's
    /// event, drawn from where the ship arrives), and shows the session's
    /// arrival message in the original's words, naming the scene laid
    /// out, followed by how many fighters were abandoned and how many
    /// hired escorts defected unpaid, if any, the fighters first.
    fn arrived(&mut self) {
        self.catch_up();
        let Ok(session) = &mut self.session else {
            return;
        };
        let notes = session.take_fighter_notes();
        let pay = session.take_pay_notes();
        let date = session.date_text();
        let mut messages = std::mem::take(&mut self.pending_leads);
        messages.extend(session.take_messages());
        let worded: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| Some((sim_message_text(message, session)?, message.ticks())))
            .collect();
        self.message = None;
        let Some(scene) = &self.scene else {
            return;
        };
        let mut said: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| Some((arrival_lead(message)?, message.ticks())))
            .next_back()
            .map(|(lead, ticks)| {
                let stellars = !scene.stellars().is_empty();
                let text = arrival_message_with(lead, scene.name(), &date, stellars);
                (text, ticks)
            })
            .into_iter()
            .collect();
        said.extend(worded);
        said.extend(notes.into_iter().map(|FighterNote::Abandoned(count)| {
            (fighters_abandoned_message(count), MESSAGE_TICKS)
        }));
        said.extend(pay_said(&pay));
        self.say_all(&said);
    }

    /// Catches the screen up with what the session has told since this
    /// was last called ([`Session::take_events`]), and gives what it
    /// redrew. Several events of a kind ask for one reaction, decided by
    /// the events alone and never by what the session was last asked to
    /// do: a change of ship reads the player's sprite sheet afresh
    /// ([`FlightView::follow_ship`]); a system entered afresh is laid out
    /// ([`FlightView::lay_out`]); otherwise a move to another stellar of
    /// the system shown draws the ship there ([`FlightView::place_ship`]);
    /// and a change to the galaxy draws the session's galaxy on the course
    /// map ([`GalaxyMap::show_galaxy`]) with the course as it is flown.
    ///
    /// The only caller of [`Session::take_events`]: each frame calls it
    /// first, so a change made outside the view's own methods (a
    /// developer tools' edit, a set expression a control bit runs) is
    /// drawn by the next frame; and each of the view's methods that drives
    /// the session between frames calls it last, so its caller sees the
    /// screen caught up at once. Taking the events again finds none.
    fn catch_up(&mut self) -> Redrawn {
        let Ok(session) = &mut self.session else {
            return Redrawn::default();
        };
        let mut redrawn = Redrawn::default();
        for event in session.take_events() {
            match event {
                SessionEvent::ShipChanged => redrawn.ship = true,
                SessionEvent::SystemChanged => redrawn.system = true,
                SessionEvent::StellarChanged => redrawn.stellar = true,
                SessionEvent::GalaxyChanged => redrawn.galaxy = true,
                SessionEvent::StellarsRebuilt => redrawn.rebuilt = true,
            }
        }
        if redrawn.galaxy {
            self.map.show_galaxy(session.galaxy());
            self.map
                .show_course(session.system(), &session.course_shown());
        }
        let system = session.system();
        if redrawn.ship {
            self.follow_ship();
        }
        if redrawn.system {
            self.lay_out(system);
        } else if redrawn.stellar {
            self.place_ship();
        }
        if redrawn.galaxy || redrawn.system || redrawn.rebuilt {
            self.show_destroyed();
        }
        if redrawn.system {
            self.redraw_status();
        }
        redrawn
    }

    /// Draws each stellar of the scene shown as the session holds it
    /// ([`show_destroyed`]). The scene is not laid out again.
    fn show_destroyed(&mut self) {
        if let (Ok(session), Some(scene)) = (&self.session, &mut self.scene) {
            show_destroyed(scene, session);
        }
    }

    /// Lays out `system`, the session's: its scene, read from the catalog,
    /// the course map's course and explored systems, and the ship drawn
    /// from where it is, the last system's NPCs and effects let go.
    fn lay_out(&mut self, system: SystemId) {
        let Ok(session) = &self.session else {
            return;
        };
        self.scene = Some(SystemScene::load(&self.catalog, system));
        self.map.show_course(system, &session.course_shown());
        self.map.show_explored(session.pilot().explored());
        self.previous = *session.player();
        self.alpha = 0.0;
        self.npc_previous.clear();
        self.effects.clear();
        self.read_npc_sheets();
    }

    /// The pilot flying, as the developer tools' desk: its edits reach
    /// the session, moves read the catalog, and the course map's galaxy
    /// names the places. `None` when the session failed.
    pub fn pilot_desk(&mut self) -> Option<SessionDesk<'_, C>> {
        let session = self.session.as_mut().ok()?;
        Some(SessionDesk::new(session, &self.catalog, self.map.model()))
    }

    /// Draws the ship where the session has it, not on its way from where
    /// it was, and shows the session's course again on the course map:
    /// after a move to another stellar of the system shown, and a
    /// take-off.
    fn place_ship(&mut self) {
        let Ok(session) = &self.session else {
            return;
        };
        self.map
            .show_course(session.system(), &session.course_shown());
        self.previous = *session.player();
        self.alpha = 0.0;
    }

    /// Catches the screen up with an edit made through the pilot desk now,
    /// rather than on the next frame, from the session's events (see the
    /// module docs): a move to another system is laid out as an arrival
    /// is, with no message, and one to another stellar of the system shown
    /// draws the ship there, the course map showing the session's course
    /// again. Nothing is read when nothing moved.
    pub fn resync(&mut self) {
        self.catch_up();
    }

    /// Takes off from the stellar landed on, and gives it; `None` when the
    /// ship has not landed. The next frame draws the ship where it is, at
    /// the stellar, not on its way from where it was, in the system a move
    /// made while landed went to, laid out afresh as the session tells
    /// ([`SessionEvent`]); it shows no message
    /// from before the landing, but says how many hired escorts defected
    /// for want of the take-off's pay ([`defection_message`]); the
    /// session has populated the system's traffic afresh
    /// ([`Session::take_off`]), so no frame draws the ships from before
    /// the landing. Leaving passes the stay's days, their
    /// events rolled on the flight's chance.
    pub fn take_off(&mut self) -> Option<StellarId> {
        let session = self.session.as_mut().ok()?;
        let stellar = session.take_off(&self.catalog, &mut self.chance)?;
        let said = self.took_off();
        self.say_all(&said);
        Some(stellar)
    }

    /// Catches the screen up with the session having taken off: the ship
    /// drawn where it is, in the system laid out afresh if the session
    /// tells a move while landed changed it, the traffic populated afresh
    /// drawn where it is with its ship types' sheets read, no message from
    /// before kept; gives what paying the escorts for the take-off has to
    /// say, each with its ticks.
    ///
    /// The original keeps the slot and replaces it with its take-off
    /// greeting (`_PlayerLandOnStellar` @0x63732), which the engine does
    /// not raise yet (rdm phase
    /// hud-fidelity/phase-12-flight-messages-the-original-raises); a
    /// message from before the landing would be further from what the
    /// player sees than none.
    fn took_off(&mut self) -> Vec<(String, u16)> {
        let pay = self
            .session
            .as_mut()
            .map(Session::take_pay_notes)
            .unwrap_or_default();
        self.catch_up();
        self.place_ship();
        self.read_npc_sheets();
        self.message = None;
        self.redraw_status();
        pay_said(&pay)
    }

    /// Shows the texts of `said`, joined by two spaces, when there is
    /// anything, for the ticks of the first: the others are appended to
    /// it, as `_HandlePlayer` appends the abandoned fighters to the
    /// arrival line (@0x6c3c4).
    fn say_all(&mut self, said: &[(String, u16)]) {
        let Some(&(_, ticks)) = said.first() else {
            return;
        };
        let texts: Vec<&str> = said.iter().map(|(text, _)| text.as_str()).collect();
        self.say_for(texts.join("  "), ticks);
    }

    /// Settles what the set expressions run since queued, as
    /// [`Session::settle_script`] does, reading the catalog and drawing on
    /// the flight's chance, and gives the stellar a `Q` made the ship take
    /// off from, for the router to close its spaceport. A take-off is
    /// caught up with as [`FlightView::take_off`] is, the `Q`'s message
    /// shown before what the take-off's pay has to say. Otherwise the
    /// screen catches up with what the session told ([`SessionEvent`]): a
    /// move in flight lays out the system the ship is in afresh, even the
    /// same one, as the original kills its explosions and smoke, and how
    /// many fighters were abandoned, if any, is shown after a `Q`'s
    /// message; a move while landed shows once the ship takes off; and a
    /// `C`, `E` or `H` reads the player's sprite sheet afresh. Then the
    /// session's messages raised as it settled show at once, as a
    /// deadline missed on the take-off does, before a text dialog can
    /// pause flight; an arrival's waits for the arrival as on a tick.
    pub fn settle_script(&mut self) -> Option<StellarId> {
        let session = self.session.as_mut().ok()?;
        let settled = session.settle_script(&self.catalog, &mut self.chance);
        let flying = session.landed().is_none();
        let mut said: Vec<(String, u16)> = settled
            .message
            .into_iter()
            .map(|text| (text, SCRIPT_MESSAGE_TICKS))
            .collect();
        if settled.took_off.is_some() {
            said.extend(self.took_off());
        } else if self.catch_up().system && flying {
            let notes = self
                .session
                .as_mut()
                .map(Session::take_fighter_notes)
                .unwrap_or_default();
            said.extend(notes.into_iter().map(|FighterNote::Abandoned(count)| {
                (fighters_abandoned_message(count), MESSAGE_TICKS)
            }));
        }
        self.say_all(&said);
        self.say_messages();
        settled.took_off
    }

    /// Plays `effect` for the ship having come out of a gate, caught up
    /// with as an arrival is ([`FlightView::arrived`]).
    fn came_through(&mut self, effect: JumpEffect) {
        self.held.clear();
        self.arrived();
        self.fade_in = Some(effect);
    }

    /// Presses the land key: requests clearance and shows the reply,
    /// lands, enters a gate, or shows why not. Over a cleared hypergate
    /// the course map opens offering its links, and over a wormhole the
    /// ship passes through at once.
    fn land(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.land() {
            Ok(LandPress::Outcome(LandOutcome::Selected {
                stellar,
                station,
                clearance,
            })) => {
                let hypergate = session.gate_kind(stellar) == Some(GateKind::Hypergate);
                let text = match clearance {
                    Clearance::Granted if hypergate => {
                        hypergate_clearance_message(self.chance.roll(2) == 0)
                    }
                    Clearance::Denied if hypergate => HYPERGATE_DENIED.to_owned(),
                    _ => {
                        let name = self.scene.as_ref().and_then(|scene| {
                            let named = scene.stellars().iter().find(|named| named.id == stellar);
                            named.map(|named| named.name.as_str())
                        });
                        clearance_message(name.unwrap_or_default(), station, clearance)
                    }
                };
                self.say_for(text, LANDING_TICKS);
            }
            Ok(LandPress::Outcome(LandOutcome::Landed(stellar))) => {
                self.pending_landing = Some(stellar);
                self.message = None;
                self.effects.clear();
                self.redraw_status();
            }
            Ok(LandPress::AtGate {
                kind: GateKind::Hypergate,
                ..
            }) => match session.open_hypergate(&self.catalog) {
                Ok(offered) => {
                    self.map.offer_gates(offered);
                    self.open_map();
                }
                Err(refusal) => self.gate_refused(Some(refusal)),
            },
            Ok(LandPress::AtGate {
                kind: GateKind::Wormhole,
                ..
            }) => match session.enter_wormhole(&self.catalog, &mut self.chance) {
                Ok(_) => self.came_through(JumpEffect::flash()),
                Err(refusal) => self.gate_refused(Some(refusal)),
            },
            Err(refusal) => {
                let gate = match refusal {
                    LandingRefusal::NotLandable { stellar, .. } => session.gate_kind(stellar),
                    _ => None,
                };
                let text = gate.map_or_else(
                    || refusal_message(&refusal).to_owned(),
                    gate_refusal_message,
                );
                self.say_for(text, landing_refusal_ticks(&refusal));
            }
        }
    }

    /// Shows what the original says when entering a gate is refused, if
    /// anything: a cancelled hypergate jump, or a wormhole leading
    /// nowhere. A hypergate without links says nothing.
    fn gate_refused(&mut self, refusal: Option<GateRefusal>) {
        match refusal {
            Some(GateRefusal::Cancelled) => self.say_for(HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS),
            Some(GateRefusal::NoExit) => {
                self.say_for(gate_refusal_message(GateKind::Wormhole), GATE_REFUSAL_TICKS);
            }
            Some(GateRefusal::NoLinks | GateRefusal::NotAtGate) | None => {}
        }
    }

    /// Closes the course map, abandoning any gesture on it, and goes back
    /// to flight. Closing the hypergate map enters the hypergate for the
    /// system picked on it: the ship comes out there, and the new system
    /// fades in from white ([`JumpEffect::emerging`], or flashes white
    /// with the Hyperspace Effects preference off); with no pick, the jump
    /// is cancelled ([`HYPERGATE_CANCELLED`]).
    pub fn close_map(&mut self) {
        self.map.cancel_pointer();
        self.map.release_keys();
        self.map_open = false;
        self.redraw_status();
        if self.map.mode() != MapMode::Hypergate {
            return;
        }
        let choice = self.map.take_gate_choice();
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.enter_hypergate(choice, &self.catalog, &mut self.chance) {
            Ok(_) => {
                let effect = JumpEffect::emerging(self.hyperspace_effects);
                self.came_through(effect);
            }
            Err(refusal) => self.gate_refused(Some(refusal)),
        }
    }

    /// Buys a ship named `name` as [`Session::buy_ship`] does, it and its
    /// hooks drawing on the flight's chance, and catches up with the
    /// change of ship it tells ([`SessionEvent::ShipChanged`]): the sprite
    /// sheet of the class the session then flies (the one bought, or the
    /// one its hooks changed it to) is read, once, so the new hull is
    /// drawn once it takes off; a refused purchase tells nothing and
    /// reads nothing, and a session that failed has no shipyard.
    pub fn buy_ship(&mut self, ship: ShipId, name: &str) -> Result<ShipPurchase, ShipRefusal> {
        let session = self.session.as_mut().map_err(|_| ShipRefusal::NoShipyard)?;
        let bought = session.buy_ship(ship, name, &mut self.chance)?;
        self.catch_up();
        Ok(bought)
    }

    /// Assigns the ship captured, as [`Session::assign`] does, and says
    /// what it did; after "Use As My Ship" it is drawn where it is, not on
    /// its way from the old ship, and the change of ship the session tells
    /// is caught up with ([`SessionEvent::ShipChanged`]), reading the new
    /// ship's sprite sheet, once. `None` when no capture awaits its
    /// assignment.
    pub fn assign(&mut self, choice: Assignment) -> Option<Assigned> {
        let session = self.session.as_mut().ok()?;
        let assigned = session.assign(choice, &mut self.chance)?;
        if assigned == Assigned::MyShip {
            self.previous = *session.player();
            self.alpha = 0.0;
        }
        self.catch_up();
        self.say_for(assigned_message(assigned), assigned_ticks(assigned));
        Some(assigned)
    }

    /// Reads the player's sprite sheet and status bar afresh when the
    /// session flies a class other than the one they were read for, so a
    /// ship bought, captured or changed by a set expression is drawn under
    /// its own bar; called only when the session tells the ship changed
    /// ([`FlightView::catch_up`]), which it may tell of the class it was.
    /// A sheet that cannot be read is kept against its class too, so it is
    /// not tried again. The bar is chosen again as `_LoadIntfResource`
    /// does ([`hud::rechoose_status_bar`]); the original returns at once
    /// for the bar already loaded, and asking only on a new class does the
    /// same.
    ///
    /// The original loads the bar inside the purchase, capture or set
    /// expression. Ours waits for the next `catch_up`, on the next frame or
    /// in the view call that drove the change, which is still before any
    /// frame is drawn, so the player never sees the old bar on the new
    /// ship; a purchase while landed is chosen before flight is drawn
    /// again.
    fn follow_ship(&mut self) {
        let Ok(session) = &self.session else { return };
        let ship = session.ship();
        if self.sheet_ship != Some(ship) {
            self.sheet = self.catalog.ship_sheet(ship);
            self.status_bar = hud::rechoose_status_bar(
                &self.catalog,
                session.interface_government(),
                self.status_bar.as_ref().ok(),
            );
            self.sheet_ship = Some(ship);
        }
    }
}

impl<C> FlightView<C> {
    /// What the radar shows now ([`RadarScene`]): the player's ship and
    /// the other ships where they are drawn, the scene's stellars in nav
    /// order with the shape and sprite width the radar reads, and blank
    /// while the ship is landed (`_playerIsLanded`); each with its IFF
    /// code, the player's IFF and density scanner, this tick's static
    /// ([`FlightView::advance_hud`]), and whether a hyperspace route is
    /// shown ([`route_shown`]).
    fn radar_now(&self) -> RadarScene {
        // Without a session flight shows only why, never the radar.
        let Ok(session) = &self.session else {
            return RadarScene::default();
        };
        let stellars = self.scene.as_ref().map_or_else(Vec::new, |scene| {
            scene
                .stellars()
                .iter()
                .map(|stellar| RadarStellar {
                    position: stellar.position,
                    square: stellar.radar_square,
                    sprite_width: stellar.drawn().as_ref().ok().map(|sheet| sheet.frame_width),
                    iff: session
                        .stellar_iff(stellar.id)
                        .unwrap_or(StellarIff::Unlandable),
                })
                .collect()
        });
        let gear = session.radar_gear();
        RadarScene {
            player: self.shown_position(),
            stellars,
            ships: radar_ships(session, &self.shown_npcs()),
            blank: session.landed().is_some(),
            iff: gear.iff,
            density_scanner: gear.density_scanner,
            static_pattern: self.radar_static,
            route_shown: route_shown(session).is_some(),
        }
    }

    /// Redraws the HUD at once, as the original's `_RedrawStatus`
    /// (@0x4cac0) does: the radar's snapshot is taken afresh, but the
    /// refresh timer and the target's blink are left as they are.
    ///
    /// The original calls it at flight start (`_PlayGame` @0x457d8, ours
    /// the first frame, which shows the radar as it is), on landing and
    /// take-off (`_PlayerLandOnStellar`), on arrival in a system
    /// (`_HandlePlayer`; ours whenever a system is laid out), on closing
    /// the map (`_DoSystemMap`), on entering a hypergate
    /// (`_PlayerEnterHypergate`, ours as the map closes) and on closing
    /// the comm dialog ([`FlightView::hang_up`]), the mission dialog
    /// ([`FlightView::accept_mission`], [`FlightView::refuse_mission`]) and
    /// the text dialog ([`FlightView::take_mission_texts`]). The shipyard
    /// dialog is the spaceport's, while the radar is blank, and taking off
    /// redraws it.
    fn redraw_status(&mut self) {
        self.radar_scene = Some(self.radar_now());
        self.retake_nav();
        self.retake_target();
    }

    /// What the target panel is drawn for now: the target, its ship type
    /// and its shield; none without a target.
    fn target_drawn(&self) -> Option<Drawn> {
        let npc = self.session.as_ref().ok()?.target()?;
        Some(Drawn {
            npc: npc.id,
            ship: npc.ship,
            shield: npc.reserves.shield.now,
        })
    }

    /// What the target panel shows of the target now (see
    /// [`TargetPanel`]); none without a target.
    fn target_now(&self) -> Option<TargetPanel> {
        let session = self.session.as_ref().ok()?;
        let npc = session.target()?;
        let card = self.cards.get(&npc.ship);
        let tag = session.target_tag(npc).and_then(|tag| match tag {
            TargetTag::Code(govt) => self.codes.get(&govt).cloned().flatten().map(Tag::Code),
            TargetTag::Escort => Some(Tag::Escort),
            TargetTag::Fighter => Some(Tag::Fighter),
        });
        Some(TargetPanel {
            name: session.npc_name(npc).unwrap_or_default().to_owned(),
            subtitle: session
                .npc_subtitle(npc)
                .map(str::to_owned)
                .or_else(|| card.map(|card| card.subtitle.clone()))
                .unwrap_or_default(),
            picture: card.and_then(|card| card.picture),
            status: session.target_status(npc),
            tag,
        })
    }

    /// Takes the target panel's snapshot afresh.
    fn retake_target(&mut self) {
        self.target_scene = Some((self.target_drawn(), self.target_now()));
    }

    /// What the target panel shows this frame: by
    /// [`RuleKey::HudCadence`]'s engine reading, the snapshot taken at its
    /// last redraw while it is of the target the player has now; by the
    /// other, before the first redraw, or once the target has changed,
    /// the panel as it is now ([`FlightView::target_now`]).
    fn target_shown(&self) -> Cow<'_, Option<TargetPanel>> {
        match &self.target_scene {
            Some((drawn, shown))
                if self.hud_waits()
                    && drawn.map(|drawn| drawn.npc) == self.target_drawn().map(|now| now.npc) =>
            {
                Cow::Borrowed(shown)
            }
            _ => Cow::Owned(self.target_now()),
        }
    }

    /// Takes the nav area's snapshot afresh.
    fn retake_nav(&mut self) {
        let mode = self.session.as_ref().map_or(NavMode::Off, nav_mode);
        self.nav_scene = Some((mode, self.nav_now()));
    }

    /// What the HUD's nav area shows now, for the [`nav_mode`] (see
    /// [`NavDisplay`]): the selected stellar, by its name in the scene
    /// ("No Destination" for one not in it); otherwise the next system on
    /// the course, by its name on the map if the pilot has explored it,
    /// with whether the ship can jump there; otherwise "Nav System Off".
    fn nav_now(&self) -> NavDisplay {
        let Ok(session) = &self.session else {
            return NavDisplay::Off;
        };
        match nav_mode(session) {
            NavMode::Off => NavDisplay::Off,
            NavMode::Stellar(target) => NavDisplay::Stellar(
                self.scene
                    .as_ref()
                    .and_then(|scene| scene.stellars().iter().find(|s| s.id == target))
                    .map(|stellar| stellar.name.clone()),
            ),
            NavMode::Hyperspace(next) => NavDisplay::Hyperspace {
                destination: next.map_or(NavDestination::None, |next| {
                    session
                        .pilot()
                        .has_explored(next)
                        .then(|| self.map.model().system(next))
                        .flatten()
                        .map_or(NavDestination::Unexplored, |system| {
                            NavDestination::Named(system.entry.name.clone())
                        })
                }),
                readiness: session.jump_readiness(),
            },
        }
    }

    /// Rolls this tick's radar static against the interference in the
    /// player's system ([`radar::static_frame`], on the effects' chance,
    /// never the simulation's), as `_HandleStatus` (@0x494d7) does every
    /// frame; then moves the HUD's timer to the view's elapsed ticks, and
    /// takes the radar's and the nav area's snapshots afresh when each
    /// refreshes ([`HudCadence::advance`]). The nav area's is also taken
    /// afresh whenever the nav mode has changed since it was taken, as
    /// each of the original's selection changes marks the area for
    /// redrawing at once (`_statusRedraw[0]`, e.g. @0x69b39, @0x69efa,
    /// @0x6a919, @0xe34d, @0x1379b). This is settled before the frame is
    /// drawn, and [`FlightView::nav_shown`] shows a changed mode at once
    /// anyway, so no boundary is crossed. The target panel's is taken
    /// afresh as [`retake_target`] says: on a change of target, on its
    /// 45-tick redraw, and on a radar refresh once its shield or ship type
    /// has changed. The crossing test may settle the 45-tick redraw a
    /// frame after the original's exact `% 45` frame would, which the
    /// player never sees.
    ///
    /// The original rolls the threshold in `_HandleStatus` and picks the
    /// pattern as `_DrawStatusRadar` draws; ours rolls both here, once a
    /// tick, which changes no decision the player can see.
    fn advance_hud(&mut self) {
        if let Ok(session) = &self.session {
            self.radar_static =
                radar::static_frame(session.interference(), &mut self.effects_chance);
        }
        let now = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
        let refresh = self.hud_cadence.advance(now);
        if refresh.radar {
            self.radar_scene = Some(self.radar_now());
        }
        if refresh.nav || !self.nav_mode_shown() {
            self.retake_nav();
        }
        let shown = self.target_scene.as_ref().and_then(|(drawn, _)| *drawn);
        if self.target_scene.is_none()
            || retake_target(shown.as_ref(), self.target_drawn().as_ref(), refresh)
        {
            self.retake_target();
        }
    }

    /// Whether the nav area's snapshot was taken for the nav mode the
    /// player is in now.
    fn nav_mode_shown(&self) -> bool {
        let mode = self.session.as_ref().map_or(NavMode::Off, nav_mode);
        self.nav_scene
            .as_ref()
            .is_some_and(|(shown, _)| *shown == mode)
    }

    /// What the nav area shows this frame: by [`RuleKey::HudCadence`]'s
    /// engine reading, the snapshot taken at its last refresh, text and
    /// colours both; by the other, before the first refresh, or once the
    /// nav mode has changed since, the nav area as it is now
    /// ([`FlightView::nav_now`]).
    fn nav_shown(&self) -> NavDisplay {
        match &self.nav_scene {
            Some((_, shown)) if self.hud_waits() && self.nav_mode_shown() => shown.clone(),
            _ => self.nav_now(),
        }
    }

    /// Whether the HUD's content waits for its refreshes: the engine's
    /// reading of [`RuleKey::HudCadence`].
    fn hud_waits(&self) -> bool {
        self.session.as_ref().map_or(RuleSource::Engine, |session| {
            session.rules().source_for(RuleKey::HudCadence)
        }) == RuleSource::Engine
    }

    /// What the radar shows this frame: by [`RuleKey::HudCadence`]'s
    /// engine reading, the snapshot taken at its last refresh; by the
    /// other, or before the first refresh, the radar as it is now
    /// ([`FlightView::radar_now`]).
    fn radar_shown(&self) -> Cow<'_, RadarScene> {
        match &self.radar_scene {
            Some(shown) if self.hud_waits() => Cow::Borrowed(shown),
            _ => Cow::Owned(self.radar_now()),
        }
    }

    /// Whether the course map is shown.
    #[must_use]
    pub fn map_open(&self) -> bool {
        self.map_open
    }

    /// The course map, shown or not.
    #[must_use]
    pub fn course_map(&self) -> &GalaxyMap {
        &self.map
    }

    /// The jump's effect, while it plays: the streak and the fade-out
    /// that hold flight, or else the arrival's fade-in (or flash) that
    /// flight goes on under.
    #[must_use]
    pub fn jump_effect(&self) -> Option<&JumpEffect> {
        self.jump.as_ref().or(self.fade_in.as_ref())
    }

    /// The stellar the ship has just landed on, once: the router takes it
    /// to show the spaceport.
    pub fn take_landing(&mut self) -> Option<StellarId> {
        self.pending_landing.take()
    }

    /// The message on screen, if any: the slot's until its ticks have
    /// passed.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        let message = self.message.as_ref()?;
        let shown = ticks(self.elapsed) - message.said_at;
        (shown < u128::from(message.ticks)).then_some(message.text.as_str())
    }

    /// Shows `text` as the message, from now, for `ticks` of flight,
    /// replacing whatever was shown, as `_DisplayComm` does
    /// (@0x9024b-0x90256).
    fn say_for(&mut self, text: impl Into<String>, ticks: u16) {
        self.message = Some(Message {
            text: text.into(),
            said_at: crate::time::ticks(self.elapsed),
            ticks,
        });
    }

    /// Clears the message line when it holds `text`, as the original
    /// clears one message by name (`_HandlePlayer` @0x6b231-0x6b2a7,
    /// @0x6dedb-0x6df29).
    fn clear_message(&mut self, text: &str) {
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.text == text)
        {
            self.message = None;
        }
    }

    /// Holds `offer`'s prompt for the router, letting go of the flight
    /// keys; a mission accepted at once shows nothing here, its texts
    /// queued in the session for the router to show over flight.
    fn hold_offer(&mut self, offer: VenueOffer) {
        if let VenueOffer::Prompt(prompt) = offer {
            self.pending_offer = Some(prompt);
            self.held.clear();
        }
    }

    /// Boards the target, or says why not: a boarding that opens is held
    /// for the router ([`FlightView::take_boarding`]), and a mission
    /// offered in its place too ([`FlightView::take_offer`]), each letting
    /// go of the flight keys.
    fn board(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        match session.board(&*self.law, &*self.boarding_rule, &mut self.chance) {
            Ok(Boarding::Opened(_)) => {
                self.pending_boarding = true;
                self.held.clear();
                if let Some(granted) = session.take_grant() {
                    let (name, plural) = session.outfit_names(granted.outfit).unwrap_or_default();
                    let said = grant_message(granted.count, name, plural);
                    self.say_for(said, GRANT_TICKS);
                }
            }
            Ok(Boarding::Repelled) => self.say_for(REPELLED, MESSAGE_TICKS),
            Ok(Boarding::Offered(offer)) => self.hold_offer(offer),
            Ok(Boarding::MissionShip(line)) => {
                if let Some(line) = line {
                    let said = mission_board_message(line, session);
                    self.say_for(said, BOARD_LINE_TICKS);
                }
            }
            Err(refusal) => {
                if let Some(text) = board_refusal_message(refusal) {
                    self.say_for(text, BOARD_REFUSAL_TICKS);
                }
            }
        }
    }

    /// Ends the hail under way, if any ([`Session::hang_up`]), and
    /// refreshes the HUD as the original's comm dialog does on closing
    /// ([`FlightView::redraw_status`]).
    pub fn hang_up(&mut self) {
        if let Ok(session) = &mut self.session {
            session.hang_up();
        }
        self.redraw_status();
    }

    /// What is on board the ship just boarded, named, once: the router
    /// takes it to open the plunder dialog.
    pub fn take_boarding(&mut self) -> Option<PlunderShown> {
        if !std::mem::take(&mut self.pending_boarding) {
            return None;
        }
        self.plunder_shown()
    }

    /// What is on board the ship being boarded, while the plunder dialog
    /// is open ([`Session::boarding`]).
    #[must_use]
    pub fn boarding(&self) -> Option<PlunderView> {
        self.session.as_ref().ok()?.boarding()
    }

    /// What the plunder dialog shows while it is open: [`Self::boarding`]
    /// with its cargo's good and its ammunition's outfit named, as
    /// [`Self::plunder`]'s messages name them.
    #[must_use]
    pub fn plunder_shown(&self) -> Option<PlunderShown> {
        let session = self.session.as_ref().ok()?;
        let view = session.boarding()?;
        let (good, outfit) = names(
            session,
            view.cargo.map(|(good, _)| good),
            view.ammo.map(|(outfit, _)| outfit),
        );
        Some(PlunderShown {
            view,
            good: good.map(str::to_owned),
            outfit: outfit.map(str::to_owned),
        })
    }

    /// Presses `take` in the plunder dialog, through the session
    /// ([`Session::plunder`]), and says what it did. The target panel is
    /// taken afresh, as `_HandlePlayer` redraws it once the dialog closes
    /// (@0x6eb97, @0x6eda6).
    pub fn plunder(&mut self, take: Take) -> Taken {
        let Ok(session) = &mut self.session else {
            return Taken::Nothing;
        };
        let taken = session.plunder(take, &*self.boarding_rule, &mut self.chance);
        let (good, outfit) = match taken {
            Taken::Cargo { good, .. } => names(session, Some(good), None),
            Taken::Ammo { outfit, .. } => names(session, None, Some(outfit)),
            _ => (None, None),
        };
        if let Some(text) = plunder_message(taken, good, outfit) {
            self.say_for(text, MESSAGE_TICKS);
        }
        self.retake_target();
        taken
    }

    /// Shows the session's messages since they were last taken, each in
    /// turn, in the original's words; an arrival's waits for its arrival
    /// to be caught up with ([`FlightView::arrived`]).
    fn say_messages(&mut self) {
        let Ok(session) = &mut self.session else {
            return;
        };
        let mut said = Vec::new();
        for message in session.take_messages() {
            match sim_message_text(&message, session) {
                Some(text) => said.push((text, message.ticks())),
                None => self.pending_leads.push(message),
            }
        }
        for (text, ticks) in said {
            self.say_for(text, ticks);
        }
    }

    /// Presses the jettison key: with Command held, jettisons the cargo,
    /// and with Option (Alt) too, all but the mission cargo (see
    /// [`Session::jettison_cargo`]), saying what went.
    fn jettison(&mut self) {
        if !self.held.contains(&Key::Command) {
            return;
        }
        let all = !self.held.contains(&Key::Alt);
        if let Ok(session) = &mut self.session {
            session.jettison_cargo(all, &mut self.chance);
        }
        self.say_messages();
    }

    /// The flight, or why it could not start.
    pub fn session(&self) -> Result<&Session, &str> {
        self.session.as_ref().map_err(String::as_str)
    }

    /// The pilot flying, if the flight started.
    #[must_use]
    pub fn pilot(&self) -> Option<&Pilot> {
        self.session.as_ref().ok().map(Session::pilot)
    }

    /// Changes the pilot with `change` while landed, as
    /// [`Session::transact`] does, and says whether it did.
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.session
            .as_mut()
            .is_ok_and(|session| session.transact(change))
    }

    /// The exchange of the stellar landed on, as [`Session::market`] gives
    /// it; none for a session that failed.
    #[must_use]
    pub fn market(&self) -> Option<Market> {
        self.session.as_ref().ok()?.market()
    }

    /// Trades as [`Session::trade`] does; a session that failed has no
    /// exchange.
    pub fn trade(&mut self, order: Order) -> Result<u32, TradeRefusal> {
        match &mut self.session {
            Ok(session) => session.trade(order),
            Err(_) => Err(TradeRefusal::NoMarket),
        }
    }

    /// The outfitter of the stellar landed on, as [`Session::outfitter`]
    /// gives it, the day's rolls drawn on the flight's chance; none for a
    /// session that failed.
    pub fn outfitter(&mut self) -> Option<Outfitter> {
        self.session.as_mut().ok()?.outfitter(&mut self.chance)
    }

    /// The outfitter opens, as [`Session::open_outfitter`] has it; a
    /// session that failed has none.
    pub fn open_outfitter(&mut self) {
        if let Ok(session) = &mut self.session {
            session.open_outfitter();
        }
    }

    /// Buys or sells an outfit as [`Session::outfit`] does, it and its
    /// hook drawing on the flight's chance; a session that failed has no
    /// outfitter.
    pub fn outfit(&mut self, order: OutfitOrder) -> Result<(), OutfitRefusal> {
        match &mut self.session {
            Ok(session) => session.outfit(order, &mut self.chance),
            Err(_) => Err(OutfitRefusal::NoOutfitter),
        }
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_counted`] does, on the flight's chance, giving
    /// how many went through; a session that failed has no outfitter.
    pub fn outfit_counted(&mut self, order: OutfitOrder, count: u32) -> Result<u32, OutfitRefusal> {
        match &mut self.session {
            Ok(session) => session.outfit_counted(order, count, &mut self.chance),
            Err(_) => Err(OutfitRefusal::NoOutfitter),
        }
    }

    /// Buys or sells up to `count` of an outfit as
    /// [`Session::outfit_order`] does, on the flight's chance, giving the
    /// whole run and its alert; a session that failed has no outfitter,
    /// refused at the first pass with no alert.
    pub fn outfit_order(&mut self, order: OutfitOrder, count: u32) -> OutfitRun {
        match &mut self.session {
            Ok(session) => session.outfit_order(order, count, &mut self.chance),
            Err(_) => OutfitRun {
                run: OrderRun {
                    done: 0,
                    stopped: Some(Stopped {
                        pass: 0,
                        refusal: OutfitRefusal::NoOutfitter,
                    }),
                },
                alert: None,
                after: Reselection::Kept,
            },
        }
    }

    /// Recharges as [`Session::recharge`] does; a session that failed
    /// sells no fuel.
    pub fn recharge(&mut self) -> Result<i64, RechargeRefusal> {
        match &mut self.session {
            Ok(session) => session.recharge(),
            Err(_) => Err(RechargeRefusal::NoFuel),
        }
    }

    /// The ships for hire in the bar of the stellar landed on, as
    /// [`Session::escorts_for_hire`] gives them, the day's rolls drawn on
    /// the flight's chance; none for a session that failed.
    pub fn escorts_for_hire(&mut self) -> Option<HireList> {
        self.session
            .as_mut()
            .ok()?
            .escorts_for_hire(&mut self.chance)
    }

    /// Hires a ship as [`Session::hire`] does, on the flight's chance; a
    /// session that failed has no bar.
    pub fn hire(&mut self, ship: ShipId) -> Result<Hired, HireRefusal> {
        match &mut self.session {
            Ok(session) => session.hire(ship, &mut self.chance),
            Err(_) => Err(HireRefusal::NoBar),
        }
    }

    /// The mission computer of the stellar landed on, as
    /// [`Session::mission_computer`] gives it; a session that failed has
    /// nothing available.
    ///
    /// # Errors
    ///
    /// The [`ComputerRefusal`] that applies.
    pub fn mission_computer(&self) -> Result<MissionComputer, ComputerRefusal> {
        match &self.session {
            Ok(session) => session.mission_computer(),
            Err(_) => Err(ComputerRefusal::NoneAvailable),
        }
    }

    /// Accepts `mission` as [`Session::accept_mission`] does, on the
    /// flight's chance: the slot it went into. The HUD is redrawn, as the
    /// original's mission dialog does on closing
    /// ([`FlightView::redraw_status`]).
    ///
    /// # Errors
    ///
    /// The [`AcceptError`] that applies; a session that failed offers
    /// nothing.
    pub fn accept_mission(&mut self, mission: MissionId) -> Result<u8, AcceptError> {
        let accepted = match &mut self.session {
            Ok(session) => session.accept_mission(mission, &mut self.chance),
            Err(_) => Err(AcceptError::NotOffered),
        };
        self.redraw_status();
        accepted
    }

    /// Refuses `mission` as [`Session::refuse_mission`] does, on the
    /// flight's chance. The HUD is redrawn, as on accepting.
    ///
    /// # Errors
    ///
    /// [`NotOffered`] when it is not offered; a session that failed offers
    /// nothing.
    pub fn refuse_mission(&mut self, mission: MissionId) -> Result<(), NotOffered> {
        let refused = match &mut self.session {
            Ok(session) => session.refuse_mission(mission, &mut self.chance),
            Err(_) => Err(NotOffered),
        };
        self.redraw_status();
        refused
    }

    /// The mission info screen's view, as [`Session::mission_info`] gives
    /// it; nothing listed for a session that failed.
    pub fn mission_info(&self) -> MissionInfo {
        match &self.session {
            Ok(session) => session.mission_info(),
            Err(_) => MissionInfo {
                rows: Vec::new(),
                today: String::new(),
            },
        }
    }

    /// Aborts the mission in `slot` as [`Session::abort_mission`] does, on
    /// the flight's chance.
    ///
    /// # Errors
    ///
    /// The [`AbortError`] that applies; a session that failed has none
    /// active.
    pub fn abort_mission(&mut self, slot: u8) -> Result<(), AbortError> {
        match &mut self.session {
            Ok(session) => session.abort_mission(slot, &mut self.chance),
            Err(_) => Err(AbortError::NotActive),
        }
    }

    /// The next offer at venue `loc`, as [`Session::next_offer`] gives it,
    /// on the flight's chance; none for a session that failed.
    pub fn next_offer(&mut self, loc: i16) -> Option<VenueOffer> {
        self.session
            .as_mut()
            .ok()?
            .next_offer(loc, &mut self.chance)
    }

    /// Enters venue `loc` for its offers, as
    /// [`Session::enter_offer_venue`] does.
    pub fn enter_offer_venue(&mut self, loc: i16) {
        if let Ok(session) = &mut self.session {
            session.enter_offer_venue(loc);
        }
    }

    /// The wait before a venue's next offer, drawn on the flight's chance
    /// ([`offer_gap`]).
    pub fn offer_gap(&mut self) -> Duration {
        mac_ticks(offer_gap(&mut self.chance))
    }

    /// The mission texts to show since they were last taken, as
    /// [`Session::take_mission_texts`] gives them; none for a session that
    /// failed. When there are any, the HUD is redrawn
    /// ([`FlightView::redraw_status`]): the original's text dialog redraws
    /// it on closing, and flight does not tick under the dialog, so
    /// redrawing as the texts are taken shows the same.
    pub fn take_mission_texts(&mut self) -> Vec<MissionText> {
        let texts = self
            .session
            .as_mut()
            .map(Session::take_mission_texts)
            .unwrap_or_default();
        if !texts.is_empty() {
            self.redraw_status();
        }
        texts
    }

    /// The shipyard of the stellar landed on, as [`Session::shipyard`]
    /// gives it, the day's rolls drawn on the flight's chance; none for a
    /// session that failed.
    pub fn shipyard(&mut self) -> Option<Shipyard> {
        self.session.as_mut().ok()?.shipyard(&mut self.chance)
    }

    /// The prompt for naming a ship of class `ship` before it is bought,
    /// as [`Session::ship_naming`] gives it, on the flight's chance; a
    /// session that failed has no shipyard.
    pub fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        let session = self.session.as_mut().map_err(|_| ShipRefusal::NoShipyard)?;
        session.ship_naming(ship, &mut self.chance)
    }

    /// Declines to buy a ship of class `ship`, as [`Session::decline_ship`]
    /// does; nothing for a session that failed.
    pub fn decline_ship(&mut self, ship: ShipId) {
        if let Ok(session) = &mut self.session {
            session.decline_ship(ship);
        }
    }

    /// Whether the pilot should be saved, as [`Session::take_save_due`]
    /// says; taking it clears it.
    pub fn take_save_due(&mut self) -> bool {
        self.session.as_mut().is_ok_and(Session::take_save_due)
    }

    /// The session's system, as laid out, if the session started.
    #[must_use]
    pub fn scene(&self) -> Option<&SystemScene> {
        self.scene.as_ref()
    }

    /// The HUD's status bar, or why it cannot be shown.
    pub fn status_bar(&self) -> Result<&StatusBar, &str> {
        self.status_bar.as_ref().map_err(String::as_str)
    }

    /// How far the display is between the last two steps, in `[0, 1)`.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Where the ship is drawn: `alpha` of the way from where it was a step
    /// ago to where it is.
    #[must_use]
    pub fn shown_position(&self) -> Point {
        shown_position(&self.previous, &self.current(), self.alpha)
    }

    /// Which way the ship is drawn facing: `alpha` of the way, the short way
    /// round, from its heading a step ago to its heading now.
    #[must_use]
    pub fn shown_heading(&self) -> f32 {
        shown_heading(&self.previous, &self.current(), self.alpha)
    }

    /// Where `npc` is drawn and which way it faces: `alpha` of the way from
    /// how it was a step ago, as the player's ship; where it is, when it
    /// was not there a step ago.
    fn shown_npc(&self, npc: &Npc) -> (Point, f32) {
        let from = self.npc_previous.get(&npc.id).unwrap_or(&npc.state);
        (
            shown_position(from, &npc.state, self.alpha),
            shown_heading(from, &npc.state, self.alpha),
        )
    }

    /// The NPCs, each with where it is drawn and which way it faces.
    fn shown_npcs(&self) -> Vec<(&Npc, Point, f32)> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .npcs()
            .iter()
            .map(|npc| {
                let (at, heading) = self.shown_npc(npc);
                (npc, at, heading)
            })
            .collect()
    }

    /// Draws each NPC with its own ship's sprite, or a crossed box when its
    /// sheet cannot be read, but for one the player cannot see for its
    /// cloak ([`sprite_drawn`]). One waiting at the gate it comes out of
    /// is hidden, then fades in over its last ticks there ([`npc_tint`]).
    fn draw_npcs(&self, list: &mut DrawList, camera: &Camera) {
        let Ok(session) = &self.session else {
            return;
        };
        for (npc, at, heading) in self.shown_npcs() {
            if !sprite_drawn(session, npc) {
                continue;
            }
            let Some(tint) = npc_tint(npc) else {
                continue;
            };
            let at = camera.world_to_screen(at);
            match self.npc_sheets.get(&npc.ship) {
                Some(Ok(sheet)) => {
                    let frame = rotation_frame(heading, sheet.rotations);
                    list.sprite(ImageKey::sprite(sheet.image_id, frame), at, tint);
                }
                _ => crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER),
            }
        }
    }

    /// The camera, on the ship as drawn.
    #[must_use]
    pub fn camera(&self) -> Camera {
        Camera::centred_on(self.shown_position())
    }

    /// The frame of the ship's sheet drawn, or `None` without a sheet.
    #[must_use]
    pub fn frame(&self) -> Option<u16> {
        let sheet = self.sheet.as_ref().ok()?;
        Some(rotation_frame(self.shown_heading(), sheet.rotations))
    }

    /// Today's date as the HUD shows it, or none for a session that never
    /// started.
    fn date_text(&self) -> String {
        self.session
            .as_ref()
            .map(Session::date_text)
            .unwrap_or_default()
    }

    /// The ship's shield, armour and fuel, or none for a session that
    /// never started.
    fn reserves(&self) -> Reserves {
        self.session
            .as_ref()
            .map(Session::reserves)
            .unwrap_or_default()
    }

    /// The player's ship now, or where it was for a session that never
    /// started.
    fn current(&self) -> ShipState {
        self.session
            .as_ref()
            .map_or(self.previous, |session| *session.player())
    }

    /// The controls for the keys held.
    fn controls(&self) -> Controls {
        let holding = |key| self.held.contains(&key);
        let turn = match (holding(Key::Left), holding(Key::Right)) {
            (true, false) => Turn::Left,
            (false, true) => Turn::Right,
            _ => Turn::None,
        };
        Controls {
            thrust: holding(Key::Up),
            turn,
            reverse: holding(Key::Down),
        }
    }

    /// A press of `key`, if it picks an escort group or works the stellar
    /// navigation target. While the escort menu is open, 1-5 pick its
    /// groups ([`GROUP_KEYS`]); otherwise 1-4, like F1-F4 always, select
    /// the stellar in their slot ([`STELLAR_SLOT_KEYS`]). F5 selects the
    /// nearest ([`NEAREST_STELLAR_KEY`]) and Nav Off clears it
    /// ([`NAV_OFF_KEY`]).
    fn nav_key(&mut self, key: Key) {
        let group = GROUP_KEYS.iter().position(|&group| group == key);
        let slot = STELLAR_SLOT_KEYS
            .iter()
            .position(|keys| keys.contains(&key));
        match (group, slot) {
            (Some(n), _) if self.escort_menu.is_open() => {
                let rows = self.escort_rows();
                self.escort_menu.select(n, self.elapsed, &rows);
            }
            (_, Some(slot)) => self.select_stellar(StellarPick::Slot(slot)),
            _ if key == NEAREST_STELLAR_KEY => self.select_stellar(StellarPick::Nearest),
            _ if key == NAV_OFF_KEY => {
                if let Ok(session) = &mut self.session {
                    session.clear_nav_target();
                }
            }
            _ => {}
        }
    }

    /// Makes the stellar `pick` finds the navigation target, if any.
    fn select_stellar(&mut self, pick: StellarPick) {
        if let Ok(session) = &mut self.session {
            session.select_stellar(pick);
        }
    }

    /// A click at screen point `at`, as the original's in space
    /// (`_HandlePlayer` @0x6a000-0x6a717): one in the status panel, the
    /// right [`STATUS_BAR_WIDTH`] (0xc2) of the screen, radar included, is
    /// no click in space and does nothing; any other selects the stellar
    /// under it ([`StellarPick::At`]).
    fn click_in_space(&mut self, at: Point) {
        if at.x >= VIEW_SIZE.0 - STATUS_BAR_WIDTH {
            return;
        }
        let world = self.camera().screen_to_world(at);
        self.select_stellar(StellarPick::At(Vec2::new(world.x, world.y)));
    }

    /// The escort menu's class rows ([`Session::escort_menu`]); none for
    /// a session that failed.
    fn escort_rows(&self) -> [ClassRow; 4] {
        self.session.as_ref().map_or_else(
            |_| {
                nova_sim::EscortClass::ALL.map(|class| ClassRow {
                    class,
                    present: false,
                    order: None,
                })
            },
            Session::escort_menu,
        )
    }

    /// E: opens or closes the escort menu, or says the player has no
    /// escorts. Opening it clears [`NO_ESCORTS`] from the message line,
    /// and only that (`_HandlePlayer` @0x6dedb-0x6df29).
    fn toggle_escort_menu(&mut self) {
        let rows = self.escort_rows();
        match self.escort_menu.toggle(self.elapsed, &rows) {
            Toggled::NoEscorts => self.say_for(NO_ESCORTS, ESCORT_TICKS),
            Toggled::Opened => self.clear_message(NO_ESCORTS),
            Toggled::Closed => {}
        }
    }

    /// Return (`ackComm`, `_keyPrefs` +0xc): clears the message line,
    /// whatever it shows, and closes the escort menu (`_HandlePlayer`
    /// @0x6dd00-0x6dd6a).
    fn acknowledge(&mut self) {
        self.message = None;
        self.escort_menu.close();
    }

    /// Gives the escorts `command`, through the session, as the menu says
    /// which; says what it changed, and keeps the menu open for it.
    fn command_escorts(&mut self, command: EscortCommand) {
        let Ok(session) = &mut self.session else {
            return;
        };
        if let Some(commanded) = session.command_escorts(self.escort_menu.group(), command) {
            self.escort_menu.touch(self.elapsed);
            self.say_for(escort_command_message(&commanded), ESCORT_TICKS);
        }
    }

    /// The escort menu, shut or open.
    #[must_use]
    pub fn escort_menu(&self) -> &EscortMenu {
        &self.escort_menu
    }

    /// Picks the target as `pick` says.
    fn select_target(&mut self, pick: TargetPick) {
        if let Ok(session) = &mut self.session {
            session.select_target(pick);
        }
    }

    /// The fight's explosions, debris and sounds.
    #[must_use]
    pub fn effects(&self) -> &Effects {
        &self.effects
    }

    /// The shots in flight, each where it is drawn: `alpha` of the way
    /// from where it was a step ago, as the ships are.
    fn shown_shots(&self, camera: &Camera) -> Vec<ShotShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        let behind = 1.0 - self.alpha;
        session
            .shots()
            .iter()
            .map(|shot| ShotShown {
                at: camera.world_to_screen(Point::new(
                    shot.velocity.x.mul_add(-behind, shot.position.x),
                    shot.velocity.y.mul_add(-behind, shot.position.y),
                )),
                heading: shot.heading,
                age: shot.age,
                weapon: shot.weapon.id,
            })
            .collect()
    }

    /// The beams being fired, each moved with its firer as it is drawn.
    fn shown_beams(&self, camera: &Camera) -> Vec<BeamShown> {
        let Ok(session) = &self.session else {
            return Vec::new();
        };
        session
            .beams()
            .iter()
            .map(|beam| {
                let (dx, dy) = self.drawn_off(session, beam.firer);
                let shown = |at: Vec2| camera.world_to_screen(Point::new(at.x + dx, at.y + dy));
                BeamShown {
                    start: shown(beam.start),
                    end: shown(beam.end),
                    weapon: beam.weapon.id,
                }
            })
            .collect()
    }

    /// How far `ship` is drawn from where it is.
    fn drawn_off(&self, session: &Session, ship: ShipRef) -> (f32, f32) {
        let (shown, now) = match ship {
            ShipRef::Player => (self.shown_position(), session.player().position),
            ShipRef::Npc(id) => match session.npcs().iter().find(|npc| npc.id == id) {
                Some(npc) => (self.shown_npc(npc).0, npc.state.position),
                None => return (0.0, 0.0),
            },
        };
        (shown.x - now.x, shown.y - now.y)
    }

    /// Draws the brackets round the nav target, then those round the
    /// ship target, the session's ([`Session::nav_brackets`],
    /// [`Session::target_brackets`]), each centred where its sprite is
    /// drawn (see the `brackets` module). A stellar whose sheet can't be
    /// read is bracketed as one with no sprite; a ship's as its crossed
    /// box.
    fn draw_brackets(&self, list: &mut DrawList, scene: &SystemScene, camera: &Camera) {
        let Ok(session) = &self.session else {
            return;
        };
        if let Some(nav) = session.nav_brackets()
            && let Some(stellar) = scene.stellars().iter().find(|s| s.id == nav.stellar)
        {
            let sprite = stellar.drawn().as_ref().ok();
            let sprite = sprite.map(|sheet| (sheet.frame_width, sheet.frame_height));
            let at = camera.world_to_screen(stellar.position);
            brackets::draw_nav(list, at, sprite, nav.expand, nav.dominated);
        }
        if let Some(target) = session.target_brackets()
            && let Some(npc) = session.target()
        {
            let (at, _) = self.shown_npc(npc);
            let sprite = match self.npc_sheets.get(&npc.ship) {
                Some(Ok(sheet)) => (sheet.frame_width, sheet.frame_height),
                _ => (PLACEHOLDER_SIZE as u32, PLACEHOLDER_SIZE as u32),
            };
            let at = camera.world_to_screen(at);
            brackets::draw_ship(list, at, Some(sprite), target.expand, target.standing);
        }
    }

    /// Draws the Red Alert icon the session shows, if any, over the
    /// brackets on the interface layer (see the `alert` module).
    fn draw_red_alert(&self, list: &mut DrawList) {
        let icon = self.session.as_ref().ok().and_then(Session::red_alert_icon);
        if let (Some(icon), Some(width)) = (icon, self.alert_width) {
            alert::draw_red_alert(list, icon, width);
        }
    }

    /// Draws the status bar's target panel ([`FlightView::target_shown`])
    /// and secondary weapon line. The weapon line follows every frame: the
    /// original redraws it (`_statusRedraw[3]`) on every event that
    /// changes it, so drawing it each frame shows the same.
    fn draw_combat_hud(&self, list: &mut DrawList, bar: &StatusBar) {
        let Ok(session) = &self.session else {
            return;
        };
        let origin = hud::bar_origin(bar);
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        let shown = self.target_shown();
        target::draw_target_panel(list, &bar.layout, origin, shown.as_ref().as_ref(), metrics);
        let line = session.secondary().map(|id| {
            let look = self.looks.weapon(id);
            let name = look.map_or_else(|| format!("wëap {}", id.0), |look| look.name.clone());
            target::secondary_text(&name, session.secondary_line_count())
        });
        target::draw_secondary(list, &bar.layout, origin, line.as_deref(), metrics);
    }

    fn draw_ship(&self, list: &mut DrawList, at: Point) {
        match &self.sheet {
            Ok(sheet) => {
                let frame = rotation_frame(self.shown_heading(), sheet.rotations);
                list.sprite(ImageKey::sprite(sheet.image_id, frame), at, Color::WHITE);
                let tick = u64::try_from(ticks(self.elapsed)).unwrap_or(u64::MAX);
                let base = self.session.as_ref().map_or(0, Session::engine_glow);
                let glow = sheet
                    .glow
                    .zip(glow_level(base, tick, &self.glow_rolls).map(lights_tint));
                let level = lights_level(&sheet.blink, tick, &self.blink_rolls);
                let lights = sheet.lights.zip(level.map(lights_tint));
                for (layer, tint) in [glow, lights].into_iter().flatten() {
                    let layer_frame = frame % layer.frames.get();
                    list.or_sprite(ImageKey::sprite(layer.image_id, layer_frame), at, tint);
                }
            }
            Err(reason) => {
                crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    at.x - PLACEHOLDER_SIZE / 2.0,
                    at.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {reason}"),
                    below,
                    OVERLAY_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// Draws the [`HELP`] line when its overlay is on.
    fn draw_help(&self, list: &mut DrawList) {
        if self.overlays.help {
            list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
        }
    }
}

impl<
    C: PilotCatalog
        + TrafficCatalog
        + CombatCatalog
        + CommCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks,
> Screen for FlightView<C>
{
    /// Never quits: Escape is the router's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.jump.is_some() || self.preparing_jump() {
            return ScreenAction::None;
        }
        let press = match *input {
            Input::Key {
                key,
                pressed: true,
                repeat: false,
            } => Some(key),
            _ => None,
        };
        if self.map_open {
            if press == Some(MAP_KEY) {
                self.close_map();
            } else {
                self.map_input(input);
            }
            return ScreenAction::None;
        }
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        } = *input
        {
            self.click_in_space(at);
        }
        match press {
            Some(LAND_KEY) => self.land(),
            Some(BOARD_KEY) => {
                self.board();
                if self.pending_boarding {
                    return ScreenAction::None;
                }
            }
            Some(HAIL_KEY) => {
                self.hail();
                if self.pending_hail.is_some() {
                    return ScreenAction::None;
                }
            }
            Some(MAP_KEY) => {
                self.open_map();
                return ScreenAction::None;
            }
            Some(JUMP_KEY) => {
                self.jump();
                return ScreenAction::None;
            }
            Some(JETTISON_KEY) => self.jettison(),
            Some(MISSION_INFO_KEY) => self.ask_mission_info(),
            Some(HYPER_SELECT_KEY) => {
                if let Ok(session) = &mut self.session {
                    session.select_next_system();
                }
                return ScreenAction::None;
            }
            Some(TARGET_KEY) => self.select_target(if self.held.contains(&Key::Alt) {
                TargetPick::NextEscort
            } else {
                TargetPick::Next
            }),
            Some(MENU_KEY) => self.toggle_escort_menu(),
            Some(MENU_CLOSE_KEY) => self.acknowledge(),
            Some(ESCORT_ATTACK_KEY) => self.command_escorts(EscortCommand::Attack),
            Some(ESCORT_DEFEND_KEY) => self.command_escorts(EscortCommand::Defend),
            Some(ESCORT_HOLD_KEY) => self.command_escorts(EscortCommand::Hold),
            Some(ESCORT_RECALL_KEY) => self.command_escorts(if self.held.contains(&Key::Alt) {
                EscortCommand::Dock
            } else {
                EscortCommand::Recall
            }),
            Some(NEAREST_KEY) => self.select_target(if self.held.contains(&Key::Alt) {
                TargetPick::Nearest
            } else {
                TargetPick::NearestThreat
            }),
            Some(SELECT_KEY) => {
                let backwards = self.held.contains(&Key::Alt);
                if let Ok(session) = &mut self.session {
                    session.select_secondary(backwards);
                }
            }
            Some(key) => self.nav_key(key),
            None => {}
        }
        if let Input::Key { key, pressed, .. } = *input
            && FLIGHT_KEYS.contains(&key)
        {
            if pressed {
                self.held.insert(key);
            } else {
                self.held.remove(&key);
            }
        }
        ScreenAction::None
    }

    /// First catches the screen up with what the session has told since
    /// (see the module docs), map open or not. Then runs the simulation's
    /// steps for `dt` with the keys held, and advances the stellars'
    /// animations. While the map is open nothing moves. During
    /// the pre-jump stage the session flies on, and the streak starts on
    /// the step the session begins the jump, which is the last step run.
    /// While the stars streak and the old system fades out only the
    /// jump's effect moves, and the ship arrives when the effect says.
    /// From then on the session flies again while the rest of the effect,
    /// the fade-in or the white flash, plays over it; that plays on with
    /// the map open too, as the original's display fade does.
    ///
    /// Then, unless the map is open, the HUD's timer moves on, refreshing
    /// the radar when it is due ([`FlightView::advance_hud`]), still
    /// before the frame is drawn.
    fn tick(&mut self, dt: Duration) {
        self.catch_up();
        if let Some(effect) = &mut self.fade_in {
            effect.advance(dt);
            if effect.done() {
                self.fade_in = None;
            }
        }
        if self.map_open {
            return;
        }
        if let Some(effect) = &mut self.jump {
            self.elapsed += dt;
            if effect.advance(dt) {
                self.arrive();
                self.fade_in = self.jump.take().filter(|effect| !effect.done());
            }
            self.advance_hud();
            return;
        }
        self.elapsed += dt;
        let Steps { steps, alpha } = self.clock.advance(dt);
        let controls = self.controls();
        let fire = (
            self.held.contains(&FIRE_KEY),
            self.held.contains(&SECONDARY_KEY),
        );
        let mut notes = Vec::new();
        let mut quotes = Vec::new();
        if let Ok(session) = &mut self.session {
            session.set_play_area(play_area());
            session.hold_fire(fire.0, fire.1);
            for _ in 0..steps {
                self.previous = *session.player();
                self.npc_previous = session
                    .npcs()
                    .iter()
                    .map(|npc| (npc.id, npc.state))
                    .collect();
                session.tick(controls);
                session.tick_traffic(&self.catalog, &*self.behaviour, &mut self.chance);
                session.tick_quotes(&self.catalog, &mut self.chance);
                quotes.extend(session.take_quotes());
                session.tick_assistance(&*self.disable_rule);
                notes.extend(session.take_comm());
                let rules = Rules {
                    disable: &*self.disable_rule,
                    defence: &*self.defence_rule,
                    law: &*self.law,
                };
                session.tick_combat(rules, &mut self.chance);
                let player = point(session.player().position);
                let dying = dying(session, &self.sheet, &self.npc_sheets);
                let chance = &mut self.effects_chance;
                self.effects.step(&dying, player, &self.looks, chance);
                let scene = Scene { player };
                for event in session.take_combat_events() {
                    self.effects.apply(&event, &scene, &self.looks, chance);
                }
                if session.jumping().is_some() {
                    break;
                }
            }
        }
        for note in &notes {
            self.say_for(comm_message(note), MESSAGE_TICKS);
        }
        for quote in quotes {
            self.say_for(quote.text, HAIL_QUOTE_TICKS);
        }
        self.say_messages();
        // The session may have populated its system afresh.
        self.read_npc_sheets();
        let rows = self.escort_rows();
        self.escort_menu.update(self.elapsed, &rows);
        self.alpha = alpha;
        self.start_streak_if_jumping();
        self.advance_hud();
    }

    fn draw(&self, list: &mut DrawList) {
        if self.map_open {
            self.map.draw(list);
            self.draw_fades(list);
            return;
        }
        let Some(scene) = &self.scene else {
            let reason = self.session().err().unwrap_or_default();
            list.text(
                format!("Cannot start flight: {reason}"),
                TITLE,
                TITLE_SIZE,
                None,
                Color::ERROR,
            );
            self.draw_help(list);
            return;
        };
        let camera = self.camera();
        match self.jump.map(|effect| (effect, effect.phase())) {
            Some((effect, JumpPhase::Streak(_) | JumpPhase::FadeOut(_))) => {
                starfield::draw_streaked(list, &camera, effect.direction(), effect.streak_length());
            }
            _ => starfield::draw(list, &camera),
        }
        let metrics = self.metrics.as_ref().map(|metrics| &*metrics.0);
        if let Some(message) = self.message() {
            hud::draw_message(list, message, metrics);
        }
        scene::draw_stellars(list, scene, &camera, self.elapsed);
        let beams = self.shown_beams(&camera);
        weapons::draw_beams(list, &beams, &self.looks, true);
        self.draw_npcs(list, &camera);
        self.draw_ship(list, camera.world_to_screen(self.shown_position()));
        weapons::draw_shots(list, &self.shown_shots(&camera), &self.looks);
        weapons::draw_beams(list, &beams, &self.looks, false);
        self.effects.draw(list, &camera, &self.looks);
        self.draw_brackets(list, scene, &camera);
        self.draw_red_alert(list);
        self.escort_menu
            .draw(list, &self.escort_rows(), self.escort_colors, metrics);
        if self.overlays.title {
            list.text(
                format!("{} (sÿst {})", scene.name(), scene.id().0),
                TITLE,
                TITLE_SIZE,
                None,
                Color::WHITE,
            );
        }
        self.draw_help(list);
        match &self.status_bar {
            Ok(bar) => {
                let radar = self.radar_shown();
                let date = self.date_text();
                let cargo = self.session.as_ref().ok().map(Session::status_cargo);
                let state = HudState {
                    radar: &radar,
                    blink_on: self.hud_cadence.blink_on(),
                    reserves: self.reserves(),
                    nav: self.nav_shown(),
                    cargo: cargo.as_ref(),
                    date: &date,
                    disabled: self
                        .session
                        .as_ref()
                        .is_ok_and(|session| session.player_condition() == Condition::Disabled),
                };
                hud::draw(list, bar, &state, metrics);
                self.draw_combat_hud(list, bar);
            }
            Err(reason) => hud::draw_unavailable(list, reason),
        }
        self.draw_fades(list);
    }

    /// Abandons any gesture on the course map.
    fn cancel_pointer(&mut self) {
        self.map.cancel_pointer();
    }

    /// Lets go of every flight key: the ship stops thrusting and turning,
    /// and coasts.
    fn release_keys(&mut self) {
        self.held.clear();
        self.map.release_keys();
    }

    /// The session's sounds (thrust, landing, taking off and jumping),
    /// then the fight's. The course map has no buttons that sound.
    fn take_sounds(&mut self) -> Vec<Sound> {
        let mut sounds: Vec<Sound> = self
            .session
            .as_mut()
            .map(|session| session.take_sounds().into_iter().map(Sound::Sim).collect())
            .unwrap_or_default();
        sounds.extend(self.effects.take_sounds().into_iter().map(Sound::Combat));
        sounds
    }

    /// What could not be read of the looks, then the session's
    /// diagnostics, each once.
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        let mut diagnostics = std::mem::take(&mut self.unread_looks);
        if let Ok(session) = &mut self.session {
            let sim = session.take_diagnostics().into_iter();
            diagnostics.extend(sim.map(Diagnostic::Sim));
        }
        diagnostics
    }
}

/// The simulation's `v` as a view point.
fn point(v: Vec2) -> Point {
    Point::new(v.x, v.y)
}

/// The ships in `session` breaking up, each with its sprite's width from
/// `player`'s sheet or the NPCs' `sheets` (a placeholder's without one).
fn dying(
    session: &Session,
    player: &Result<ShipSheet, String>,
    sheets: &BTreeMap<ShipId, Result<ShipSheet, String>>,
) -> Vec<Dying> {
    let width = |sheet: Option<&Result<ShipSheet, String>>| match sheet {
        Some(Ok(sheet)) => sheet.frame_width as f32,
        _ => PLACEHOLDER_SIZE,
    };
    let breaking = |condition, at: Vec2, sprite_width, explosion| match condition {
        Condition::Dying { ticks_left } => Some(Dying {
            at: point(at),
            sprite_width,
            explosion,
            ticks_left,
        }),
        _ => None,
    };
    let me = breaking(
        session.player_condition(),
        session.player().position,
        width(Some(player)),
        session.hull().breakup,
    );
    let npcs = session.npcs().iter().filter_map(|npc| {
        breaking(
            npc.condition,
            npc.state.position,
            width(sheets.get(&npc.ship)),
            npc.hull.breakup,
        )
    });
    me.into_iter().chain(npcs).collect()
}

/// Whether `npc`'s sprite may be drawn for its cloak: not while the
/// player cannot see past it ([`Session::sees_past_cloak`]). A ship
/// waiting at the gate it comes out of is hidden, then faded in, by its
/// tint ([`npc_tint`]), as `_HandleShipDisplay` (@0x2c090-0x2c18e) fades
/// it. The radar still shows it, as `_DrawStatusRadar` does not ask.
fn sprite_drawn(session: &Session, npc: &Npc) -> bool {
    session.sees_past_cloak(npc)
}

/// Each of `shown` the radar shows, where it is drawn, whether it is the
/// player's target, its IFF code and whether it is heavy: those
/// [`Session::on_radar`] says, a cloaked ship only when its cloak shows
/// there or the player scans for cloaked ships.
fn radar_ships(session: &Session, shown: &[(&Npc, Point, f32)]) -> Vec<RadarShip> {
    let target = session.target().map(|npc| npc.id);
    shown
        .iter()
        .filter(|(npc, _, _)| session.on_radar(npc))
        .map(|&(npc, at, _)| RadarShip {
            position: at,
            targeted: target == Some(npc.id),
            iff: session.ship_iff(npc),
            heavy: npc.hull.mass > HEAVY_MASS,
        })
        .collect()
}

/// The class `Mass` a ship must be above to show as a square under a
/// density scanner (`cmpw $0x63` @0x49bb8-0x49c2f).
const HEAVY_MASS: f32 = 99.0;

/// The player's nav mode, the original's player +0x6a, as the engine
/// derives it: a selected stellar is mode 2, else a plotted course mode 3,
/// else mode −1, the nav system off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavMode {
    /// Mode −1: nothing selected, no course.
    Off,
    /// Mode 2: the selected stellar.
    Stellar(StellarId),
    /// Mode 3: the next hop shown ([`route_shown`]), `None` when the first
    /// hop has no visible system at its place.
    Hyperspace(Option<SystemId>),
}

/// The player's [`NavMode`] in `session`.
///
/// A course whose first hop has no system shown at its place is mode 3
/// with no destination ("No Destination", the original's +0x6c == −1).
/// The original keeps the hop's stale link index there and names the
/// hidden system; ours does not reveal a hidden system. Its other mode
/// changes that the engine has no state for (Nav Off keeping the route,
/// Hyper Select with nothing to cue) are rdm phase
/// hud-fidelity/phase-11-track-nav-mode's.
fn nav_mode(session: &Session) -> NavMode {
    if let Some(target) = session.nav_target() {
        NavMode::Stellar(target)
    } else if session.course().is_empty() {
        NavMode::Off
    } else {
        NavMode::Hyperspace(route_shown(session))
    }
}

/// The next system on the hyperspace route the nav area shows, if it
/// shows one: no stellar is selected and a course is shown (the
/// original's nav mode 3). The nav area ([`FlightView::nav_now`]) and
/// the radar's centre arrow ([`RadarScene::route_shown`]) both read it.
fn route_shown(session: &Session) -> Option<SystemId> {
    if session.nav_target().is_some() {
        return None;
    }
    session.course_shown().first().copied()
}

/// The play area about the ship, in world units: the screen left of the
/// status bar, the ship drawn at the screen's centre. The session reads it to
/// tell whether a ship is on screen.
fn play_area() -> nova_sim::cloak::Area {
    let camera = Camera::centred_on(Point::default());
    let top_left = camera.screen_to_world(Point::default());
    let bottom_right =
        camera.screen_to_world(Point::new(VIEW_SIZE.0 - STATUS_BAR_WIDTH, VIEW_SIZE.1));
    nova_sim::cloak::Area {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::num::NonZeroU16;

    use crate::system::camera::VIEW_CENTER;
    use nova_sim::landing::{LandingRefusal, StellarFlags};
    use nova_sim::{
        CharacterStart, CommodityStrings, DateAffixes, DisasterRecord, GateSite, Handling,
        JunkRecord, LandingSite, OutfitId, OutfitRecord, Reserves, ShipFields, ShipId, ShipRecord,
        ShipStats, SimDiagnostic, SimSound, SoundId, StarSystem, StartDate, StartError, SystemId,
        TICK, Vec2, step,
    };

    use super::*;
    use crate::devtools::{PilotDesk, PilotEdit};
    use crate::draw::lights_tint;
    use crate::flight::cadence::{NAV_PERIOD_TICKS, TARGET_PERIOD_TICKS};
    use crate::flight::catalog::{
        Blink, BoomLook, GovtId, LayerSheet, StatusBarLayout, TargetCard, WeaponLook,
    };
    use crate::flight::hud::{self, HudState, NavDisplay, StatusBar};
    use crate::flight::radar::{self, IRect, RadarColor, RadarMark};
    use crate::galaxy::{Galaxy, MapMode, SystemEntry};
    use crate::sound::Sound;
    use nova_sim::ShipIff;

    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::{Blend, DrawCommand, Font};
    use nova_sim::hyperspace::{JumpRefusal, MIN_JUMP_DISTANCE};
    use nova_sim::{BlinkChance, GLOW_CRUISE, HashedRolls, glow_level};
    use nova_sim::{RuleKey, RuleSource};

    /// The first `chär` flies ship 128 (an average ship that turns 3° a
    /// tick, with a 36-rotation, 40 x 40 sheet, `rlëD` 2000) from system
    /// 130, Sol: Earth at (0, -600) and Moon at (300, -200), which animates
    /// a frame a tick. On the map Sol is at (0, 0), linked both ways to Alpha
    /// Centauri (131) at (600, 0), which holds Proxima at its centre;
    /// Barnard (132) at (0, 600) is linked to nothing, unless `onward`.
    /// Records the systems read.
    struct FakeCatalog {
        character: Result<CharacterStart, StartError>,
        fields: ShipFields,
        sheet: Result<ShipSheet, String>,
        /// `ïntf` 128: stock-like, or why it cannot be read.
        bar: Result<StatusBarLayout, String>,
        /// `cicn` 18000's size: 24 x 24, by default.
        alert_icon: Option<(u32, u32)>,
        /// Each `gövt`'s `Interface`: none, by default.
        interfaces: Vec<(i16, i16)>,
        /// The `ïntf`s past 128: none, by default.
        other_bars: Vec<(i16, StatusBarLayout)>,
        /// The `ïntf`s asked for.
        bars_asked: RefCell<Vec<i16>>,
        /// System 130's landing sites: Earth and Moon, by default.
        sites: Vec<LandingSite>,
        /// System 130's `NavDefs` slots on the map: none, by default.
        sol_navs: Vec<Option<(u32, u16)>>,
        /// System 130's `Interference`: none, by default.
        sol_interference: i16,
        /// The stellars Sol lists after Earth and the Moon: none, by
        /// default.
        sol_more: Vec<StellarContents>,
        systems_read: RefCell<Vec<SystemId>>,
        /// The commodities: none, by default.
        commodities: CommodityStrings,
        /// The planetary events: none, by default.
        disasters: Vec<DisasterRecord>,
        /// The outfits: none, by default.
        outfits: Vec<OutfitRecord>,
        /// The ship classes the shipyard reads: none, by default.
        ships: Vec<ShipRecord>,
        /// The ships whose sheets were asked for.
        sheets_asked: RefCell<Vec<ShipId>>,
        /// Each system's traffic: none, by default.
        traffic: Vec<(SystemId, nova_sim::SystemTraffic)>,
        /// The düdes: none, by default.
        dudes: Vec<(nova_sim::DudeId, nova_sim::DudeRecord)>,
        /// The weapons: none, by default.
        weapons: Vec<nova_sim::WeaponRecord>,
        /// The ship types' combat fields: none, by default.
        hulls: Vec<nova_sim::HullRecord>,
        /// The governments: none, by default.
        govts: Vec<nova_sim::GovtRecord>,
        /// The weapons' looks: none, by default.
        looks: Vec<(i16, Result<WeaponLook, String>)>,
        /// The explosions' looks: none, by default.
        booms: Vec<(i16, Result<BoomLook, String>)>,
        /// The ship types' target cards: none, by default.
        cards: Vec<(i16, TargetCard)>,
        /// The governments' target codes: none, by default.
        codes: Vec<(i16, String)>,
        /// The string lists: none, by default.
        strings: Vec<(i16, Vec<String>)>,
        /// The persons: none, by default.
        persons: Vec<nova_sim::PersonRecord>,
        /// Ship 128's default items: none, by default.
        defaults: Vec<(OutfitId, u16)>,
        /// Whether Alpha Centauri links on to Barnard, which has no
        /// stellars: not by default.
        onward: bool,
        /// Whether Sol lists Barnard too, after Alpha Centauri, so that
        /// Hyper Select has two systems to cycle through: not by default.
        fan: bool,
        /// Every stellar a gate may lead to: none, by default.
        gates: Vec<GateSite>,
        /// Whether Alpha Reborn (133) exists, at Alpha Centauri's place,
        /// with no links and Proxima Reborn (141) at its centre: not by
        /// default.
        reborn: bool,
        /// The stellars that can be destroyed and regenerated: none, by
        /// default.
        lives: Vec<nova_sim::StellarLife>,
        /// The missions: none, by default.
        missions: Vec<nova_sim::catalog::MissionRecord>,
        /// What the mission screens show of each: none, by default.
        mission_texts: Vec<nova_sim::catalog::MissionTexts>,
    }

    /// No strings unless a test sets them.
    impl nova_sim::CommCatalog for FakeCatalog {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.strings
                .iter()
                .find(|(list, _)| *list == id)
                .map_or_else(Vec::new, |(_, strings)| strings.clone())
        }
    }

    type View = FlightView<FakeCatalog>;

    const FIELDS: ShipFields = ShipFields {
        speed: 300,
        accel: 300,
        maneuver: 30,
        shield: 40,
        armor: 60,
        fuel: 250,
        fuel_regen: 0,
        holds: 0,
        mass: 15,
        free_mass: 8,
        contribute: 1,
        shield_rech: 0,
        armor_rech: 0,
        flags2: 0,
    };

    fn sheet() -> ShipSheet {
        ShipSheet {
            image_id: 2000,
            rotations: NonZeroU16::new(36).expect("non-zero"),
            frame_width: 40,
            frame_height: 40,
            glow: None,
            lights: None,
            blink: Blink::STEADY,
        }
    }

    fn catalog() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [None, Some(SystemId(130)), None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                ..CharacterStart::default()
            }),
            fields: FIELDS,
            sheet: Ok(sheet()),
            bar: Ok(layout()),
            alert_icon: Some((24, 24)),
            interfaces: Vec::new(),
            other_bars: Vec::new(),
            bars_asked: RefCell::default(),
            sites: vec![
                site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
            systems_read: RefCell::default(),
            commodities: CommodityStrings::default(),
            disasters: Vec::new(),
            outfits: Vec::new(),
            ships: Vec::new(),
            sheets_asked: RefCell::default(),
            traffic: Vec::new(),
            dudes: Vec::new(),
            weapons: Vec::new(),
            hulls: Vec::new(),
            govts: Vec::new(),
            looks: Vec::new(),
            booms: Vec::new(),
            cards: Vec::new(),
            codes: Vec::new(),
            strings: Vec::new(),
            persons: Vec::new(),
            defaults: Vec::new(),
            onward: false,
            fan: false,
            reborn: false,
            sol_navs: Vec::new(),
            sol_interference: 0,
            sol_more: Vec::new(),
            gates: Vec::new(),
            lives: Vec::new(),
            missions: Vec::new(),
            mission_texts: Vec::new(),
        }
    }

    /// Stellar `id` at `(x, y)`, 20 x 20 (radius 10), with `flags`.
    fn site(id: i16, (x, y): (f32, f32), flags: u32) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((20, 20)),
            flags,
            min_status: 0,
            landing_sound: None,
            tech_level: 1,
            special_tech: [0; 8],
            govt: None,
            flags2: 0,
            destroyed: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            exit_angle: -1,
        }
    }

    /// The start's date, with stock's affixes.
    const DATE: &str = "June 23, 1177 NC";

    /// Stock `ïntf` 128's areas, with background `PICT` 700.
    fn layout() -> StatusBarLayout {
        let rect = |left, top, right, bottom| crate::geometry::Bounds {
            min: at(left, top),
            max: at(right, bottom),
        };
        StatusBarLayout {
            radar: rect(8.0, 8.0, 184.0, 184.0),
            shield: rect(35.0, 199.0, 184.0, 206.0),
            armor: rect(35.0, 216.0, 184.0, 223.0),
            fuel: rect(35.0, 234.0, 184.0, 241.0),
            nav: rect(8.0, 254.0, 184.0, 286.0),
            weap: rect(8.0, 300.0, 184.0, 315.0),
            targ: rect(8.0, 330.0, 184.0, 442.0),
            cargo: rect(8.0, 458.0, 184.0, 552.0),
            bright_text: Color::WHITE,
            dim_text: Color::DIM,
            bright_radar: Color::rgba(0, 255, 0, 255),
            dim_radar: Color::rgba(0, 128, 0, 255),
            shield_color: Color::rgba(0, 0, 255, 255),
            armor_color: Color::rgba(255, 0, 0, 255),
            fuel_full: Color::rgba(255, 255, 0, 255),
            fuel_partial: Color::rgba(128, 128, 0, 255),
            font: Font::Geneva,
            font_size: 12.0,
            subtitle_size: 10.0,
            status_bkgnd: 700,
        }
    }

    impl PilotCatalog for FakeCatalog {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            self.character.clone()
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            assert_eq!(id, ShipId(128));
            Ok(self.fields)
        }

        fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
            assert_eq!(id, ShipId(128));
            self.defaults.clone()
        }

        fn outfits(&self) -> Vec<OutfitRecord> {
            self.outfits.clone()
        }

        fn ships(&self) -> Vec<ShipRecord> {
            self.ships.clone()
        }

        /// Sol and Alpha Centauri; Barnard, which has no stellars, only
        /// when Alpha Centauri links on to it.
        fn system_exists(&self, id: SystemId) -> bool {
            matches!(id.0, 130 | 131)
                || (id.0 == 132 && self.onward)
                || (id.0 == 133 && self.reborn)
        }

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            match system.0 {
                130 => self.sites.clone(),
                131 => vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)],
                132 if self.onward => Vec::new(),
                133 if self.reborn => vec![site(141, (0.0, 0.0), StellarFlags::CAN_LAND)],
                other => panic!("asked for sÿst {other}'s sites"),
            }
        }

        fn star_map(&self) -> Vec<StarSystem> {
            let star = |id, (x, y), links: &[i16]| StarSystem {
                id: SystemId(id),
                position: Vec2::new(x, y),
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
                stellars: Vec::new(),
                interference: 0,
            };
            let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
            let sol: &[i16] = if self.fan { &[131, 132] } else { &[131] };
            let mut systems = vec![
                StarSystem {
                    stellars: self.sol_navs.clone(),
                    interference: self.sol_interference,
                    ..star(130, (0.0, 0.0), sol)
                },
                star(131, (600.0, 0.0), onward),
                star(132, (0.0, 600.0), &[]),
            ];
            if self.reborn {
                systems.push(star(133, (600.0, 0.0), &[]));
            }
            systems
        }

        fn commodity_strings(&self) -> CommodityStrings {
            self.commodities.clone()
        }

        fn junk(&self) -> Vec<JunkRecord> {
            Vec::new()
        }

        fn disasters(&self) -> Vec<DisasterRecord> {
            self.disasters.clone()
        }

        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            Vec::new()
        }

        /// Stock's: no prefix, and " NC".
        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: String::new(),
                suffix: " NC".to_owned(),
            }
        }

        fn gate_sites(&self) -> Vec<GateSite> {
            self.gates.clone()
        }

        fn stellar_lives(&self) -> Vec<nova_sim::StellarLife> {
            self.lives.clone()
        }

        fn fallback_frame_size(&self) -> Option<(u32, u32)> {
            Some((100, 100))
        }

        fn missions(&self) -> Vec<nova_sim::catalog::MissionRecord> {
            self.missions.clone()
        }

        fn mission_texts(&self) -> Vec<nova_sim::catalog::MissionTexts> {
            self.mission_texts.clone()
        }

        fn listed_stellars(&self) -> Vec<nova_sim::catalog::ListedStellar> {
            Vec::new()
        }

        fn place_names(&self) -> nova_sim::catalog::PlaceNames {
            nova_sim::catalog::PlaceNames::default()
        }

        fn stellar_marks(&self) -> Vec<nova_sim::catalog::StellarMark> {
            Vec::new()
        }
    }

    /// The weapons, ship types' combat fields and governments given.
    impl CombatCatalog for FakeCatalog {
        fn weapons(&self) -> Vec<nova_sim::WeaponRecord> {
            self.weapons.clone()
        }

        fn hulls(&self) -> Vec<nova_sim::HullRecord> {
            self.hulls.clone()
        }

        fn governments(&self) -> Vec<nova_sim::GovtRecord> {
            self.govts.clone()
        }
    }

    /// The traffic, düdes and persons given; no fleets.
    impl TrafficCatalog for FakeCatalog {
        fn system_traffic(&self, id: SystemId) -> Option<nova_sim::SystemTraffic> {
            self.traffic
                .iter()
                .find(|(system, _)| *system == id)
                .map(|(_, traffic)| *traffic)
        }

        fn dude(&self, id: nova_sim::DudeId) -> Option<nova_sim::DudeRecord> {
            self.dudes
                .iter()
                .find(|(dude, _)| *dude == id)
                .map(|(_, record)| record.clone())
        }

        fn fleets(&self) -> Vec<nova_sim::FleetRecord> {
            Vec::new()
        }

        fn persons(&self) -> Vec<nova_sim::PersonRecord> {
            self.persons.clone()
        }
    }

    fn stellar(id: i16, name: &str, (x, y): (i16, i16), frames: u16) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: name.to_owned(),
            x,
            y,
            sprite: Ok(StellarSheet {
                image_id: 1000 + id,
                frames: NonZeroU16::new(frames).expect("non-zero"),
                frame_width: 20,
                frame_height: 20,
            }),
            dead_sprite: None,
            animation: AnimationData {
                delay: 1,
                frame0_bias: 0,
                only_when_destroyed: false,
            },
            radar_square: false,
        }
    }

    /// Graphic type 0's sheet (`spïn` 1000) in every system: one 40 x 40
    /// frame.
    const TYPE_ZERO: i16 = 3000;

    /// `stellar`, drawing `DeadType` sheet 2000 + `id` (one 30 x 30 frame)
    /// while destroyed.
    fn dying(id: i16, name: &str, at: (i16, i16), frames: u16) -> StellarContents {
        StellarContents {
            dead_sprite: Some(Ok(StellarSheet {
                image_id: 2000 + id,
                frames: NonZeroU16::MIN,
                frame_width: 30,
                frame_height: 30,
            })),
            ..stellar(id, name, at, frames)
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            self.systems_read.borrow_mut().push(id);
            let (name, stellars) = match id.0 {
                130 => (
                    "Sol",
                    [
                        dying(128, "Earth", (0, -600), 1),
                        stellar(129, "Moon", (300, -200), 4),
                    ]
                    .into_iter()
                    .chain(self.sol_more.iter().cloned())
                    .collect(),
                ),
                131 => ("Alpha Centauri", vec![dying(140, "Proxima", (0, 0), 1)]),
                132 if self.onward => ("Barnard", Vec::new()),
                133 if self.reborn => (
                    "Alpha Reborn",
                    vec![stellar(141, "Proxima Reborn", (0, 0), 1)],
                ),
                other => panic!("asked for sÿst {other}"),
            };
            SystemContents {
                id,
                name: name.to_owned(),
                stellars,
                type_zero_sprite: Ok(StellarSheet {
                    image_id: TYPE_ZERO,
                    frames: NonZeroU16::MIN,
                    frame_width: 40,
                    frame_height: 40,
                }),
                problems: Vec::new(),
            }
        }
    }

    impl GalaxyCatalog for FakeCatalog {
        fn galaxy(&self) -> Galaxy {
            let entry = |id, name: &str, (x, y), links: &[i16]| SystemEntry {
                id: SystemId(id),
                name: name.to_owned(),
                x,
                y,
                links: links.iter().copied().map(SystemId).collect(),
                govt: None,
                stellars: Vec::new(),
            };
            let onward: &[i16] = if self.onward { &[130, 132] } else { &[130] };
            let sol: &[i16] = if self.fan { &[131, 132] } else { &[131] };
            let mut systems = vec![
                entry(130, "Sol", (0, 0), sol),
                entry(131, "Alpha Centauri", (600, 0), onward),
                entry(132, "Barnard", (0, 600), &[]),
            ];
            if self.reborn {
                systems.push(entry(133, "Alpha Reborn", (600, 0), &[]));
            }
            Galaxy {
                systems,
                ..Galaxy::default()
            }
        }
    }

    impl ShipSprites for FakeCatalog {
        /// Ship 128's sheet is [`FakeCatalog::sheet`]; ship 129's is
        /// `rlëD` 2001's, with 72 rotations, glow `rlëD` 2101 and lights
        /// `rlëD` 2201; ship 130's cannot be read.
        fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
            self.sheets_asked.borrow_mut().push(id);
            match id.0 {
                128 => self.sheet.clone(),
                129 => Ok(ShipSheet {
                    image_id: 2001,
                    rotations: NonZeroU16::new(72).expect("non-zero"),
                    glow: Some(layer(2101, 72)),
                    lights: Some(layer(2201, 72)),
                    ..sheet()
                }),
                130 => Err("no shän 130".to_owned()),
                other => panic!("asked for shïp {other}'s sheet"),
            }
        }
    }

    /// The looks, cards and codes given; any other weapon cannot be read,
    /// and there is no other explosion type.
    impl CombatLooks for FakeCatalog {
        fn weapon_look(&self, id: nova_sim::WeaponId) -> Result<WeaponLook, String> {
            self.looks
                .iter()
                .find(|(weapon, _)| *weapon == id.0)
                .map_or_else(
                    || Err(format!("no wëap {}", id.0)),
                    |(_, look)| look.clone(),
                )
        }

        fn boom_look(&self, id: nova_sim::BoomId) -> Option<Result<BoomLook, String>> {
            self.booms
                .iter()
                .find(|(boom, _)| *boom == id.0)
                .map(|(_, look)| look.clone())
        }

        fn target_card(&self, ship: ShipId) -> TargetCard {
            self.cards
                .iter()
                .find(|(id, _)| *id == ship.0)
                .map(|(_, card)| card.clone())
                .unwrap_or_default()
        }

        fn target_code(&self, govt: GovtId) -> Option<String> {
            self.codes
                .iter()
                .find(|(id, _)| *id == govt.0)
                .map(|(_, code)| code.clone())
        }
    }

    /// The escort menu in its own colours, so tests can tell them.
    impl EscortMenuLooks for FakeCatalog {
        fn escort_menu_colors(&self) -> EscortMenuColors {
            MENU_COLORS
        }
    }

    const MENU_COLORS: EscortMenuColors = EscortMenuColors {
        border: Color::from_rgb24(0x0012_3456),
        hilite: Color::from_rgb24(0x0065_4321),
    };

    /// `ïntf` 128 and the others listed; every government listed has an
    /// `Interface`, and the others have no record. `PICT` 700 (128's
    /// background), 705 and 128 are 194 x 767, and there are no others.
    impl StatusBars for FakeCatalog {
        fn government_interface(&self, id: GovtId) -> Result<Option<i16>, String> {
            Ok(self
                .interfaces
                .iter()
                .find(|(govt, _)| *govt == id.0)
                .map(|&(_, interface)| interface))
        }

        fn status_bar(&self, id: i16) -> Result<StatusBarLayout, String> {
            self.bars_asked.borrow_mut().push(id);
            if id == 128 {
                return self.bar.clone();
            }
            self.other_bars
                .iter()
                .find(|(bar, _)| *bar == id)
                .map(|(_, layout)| layout.clone())
                .ok_or_else(|| format!("no ïntf {id}"))
        }

        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            [700, 705, 128].contains(&id).then_some((194, 767))
        }

        fn icon_size(&self, id: i16) -> Option<(u32, u32)> {
            (id == ALERT_CICN).then_some(self.alert_icon).flatten()
        }
    }

    fn flight() -> View {
        FlightView::new(catalog())
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn player(view: &View) -> ShipState {
        *view.session().expect("flying").player()
    }

    fn handling() -> Handling {
        ShipStats::new(FIELDS, &[]).handling
    }

    /// The ship as it starts: at rest at the centre, facing up.
    fn start() -> ShipState {
        ShipState::default()
    }

    fn reserves(view: &View) -> Reserves {
        view.session().expect("flying").reserves()
    }

    /// `state` after `ticks` steps under `controls`.
    fn stepped(mut state: ShipState, controls: Controls, ticks: u32) -> ShipState {
        for _ in 0..ticks {
            step(&mut state, &handling(), controls);
        }
        state
    }

    /// The second half of a tick: `TICK / 2` rounds down, so this is a
    /// nanosecond longer.
    const REST_OF_TICK: Duration = Duration::from_nanos(16_666_667);

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
    };

    fn ticks(view: &mut View, n: u32) {
        for _ in 0..n {
            view.tick(TICK);
        }
    }

    // Building.

    #[test]
    fn it_starts_the_session_in_its_system_at_rest() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(session.ship(), ShipId(128));
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(session.handling(), handling());
        assert_eq!(*session.player(), start());
        let scene = view.scene().expect("a scene");
        assert_eq!(scene.id(), SystemId(130));
        assert_eq!(scene.stellars().len(), 2);
        assert_eq!(view.camera().center(), at(0.0, 0.0));
        assert_eq!(view.shown_position(), at(0.0, 0.0));
        assert_eq!(view.frame(), Some(0));
    }

    #[test]
    fn a_session_that_cannot_start_says_why_and_draws_nothing_else() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(failed);
        assert_eq!(view.session().err(), Some("no chär to start from"));
        assert!(view.scene().is_none());
        assert_eq!(view.frame(), None);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(
            drawn(&view).iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Text {
                text: "Cannot start flight: no chär to start from".to_owned(),
                font: Font::Geneva,
                origin: TITLE,
                size: TITLE_SIZE,
                wrap_width: None,
                color: Color::ERROR,
            },]
        );
    }

    #[test]
    fn the_unstarted_flight_shows_help_only_when_turned_on() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let view = FlightView::new(failed).with_overlays(FlightOverlays {
            title: false,
            help: true,
        });
        let list = drawn(&view);
        let help = overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM);
        assert_eq!(list.iter().last(), Some(&help));
        assert_eq!(list.iter().count(), 2, "the error line and the help");
    }

    // Flying.

    #[test]
    fn holding_up_thrusts_the_ship_up_and_the_camera_follows() {
        let mut view = flight();
        assert_eq!(view.input(&key(Key::Up, true)), ScreenAction::None);
        view.tick(Duration::from_secs(1));
        let expected = stepped(start(), THRUST, 30);
        assert_eq!(player(&view), expected);
        assert!(expected.position.y < -40.0, "{expected:?}");
        assert_eq!(expected.position.x, 0.0);
        assert_eq!(view.camera().center(), view.shown_position());
        let shown = view.shown_position();
        // Drawn at most a step behind.
        let behind = shown.y - expected.position.y;
        assert!((0.0..=3.0 + 1e-4).contains(&behind), "{shown:?}");
    }

    #[test]
    fn releasing_up_coasts() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Up, false));
        let coasting = player(&view);
        ticks(&mut view, 10);
        let later = player(&view);
        assert_eq!(later.velocity, coasting.velocity);
        assert_eq!(later, stepped(coasting, Controls::default(), 10));
    }

    #[test]
    fn left_and_right_turn_the_ship_and_its_frame() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 11);
        // 30° after 10 steps, drawn a step behind.
        assert_eq!(player(&view).heading, 33.0);
        assert_eq!(view.shown_heading(), 30.0);
        assert_eq!(view.frame(), Some(3));
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Left, true));
        ticks(&mut view, 21);
        assert_eq!(player(&view).heading, 330.0);
        assert_eq!(view.frame(), Some(33));
        assert_eq!(player(&view).velocity, Vec2::ZERO, "turning alone");
    }

    #[test]
    fn left_and_right_together_cancel() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.input(&key(Key::Right, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view).heading, 0.0);
    }

    #[test]
    fn down_turns_the_ship_against_its_motion() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        ticks(&mut view, 70);
        assert!(
            (player(&view).heading - 180.0).abs() < 1e-3,
            "{:?}",
            player(&view)
        );
        assert_eq!(view.frame(), Some(18));
        // Left or Right overrides it.
        view.input(&key(Key::Left, true));
        ticks(&mut view, 1);
        assert!((player(&view).heading - 177.0).abs() < 1e-3);
    }

    #[test]
    fn repeats_hold_a_key_and_releasing_the_keys_stops_the_thrust() {
        let mut view = flight();
        view.input(&held(Key::Up));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        view.release_keys();
        let coasting = player(&view);
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(coasting, Controls::default(), 5));
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut view = flight();
        view.input(&key(Key::Up, false));
        ticks(&mut view, 3);
        assert_eq!(player(&view), start());
    }

    #[test]
    fn other_input_changes_nothing_and_never_quits() {
        let mut view = flight();
        let others = [
            key(Key::Escape, true),
            key(Key::Space, true),
            key(Key::Tab, true),
            key(Key::Space, true),
            key(Key::Enter, true),
            key(Key::Char('w'), true),
            key(Key::Char('w'), true),
            key(Key::Other, true),
            Input::PointerMoved(at(1.0, 2.0)),
            Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed: true,
                at: at(1.0, 2.0),
            },
        ];
        for input in others {
            assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
        }
        ticks(&mut view, 5);
        assert_eq!(player(&view), start());
    }

    // Interpolation.

    #[test]
    fn the_ship_is_drawn_alpha_of_the_way_from_the_last_step_to_this_one() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        let first = stepped(start(), THRUST, 1);
        assert_eq!(player(&view), first);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(0.0, 0.0), "a step behind");
        view.tick(TICK / 2);
        assert!((view.alpha() - 0.5).abs() < 1e-6, "{}", view.alpha());
        assert_eq!(player(&view), first, "no step yet");
        let half = view.shown_position();
        assert!((half.y - first.position.y / 2.0).abs() < 1e-6, "{half:?}");
        assert_eq!(half.x, 0.0);
        assert_eq!(view.camera().center(), half);
        // The rest of the tick: one more step, and back to its start.
        view.tick(REST_OF_TICK);
        let second = stepped(first, THRUST, 1);
        assert_eq!(player(&view), second);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(
            view.shown_position(),
            at(first.position.x, first.position.y)
        );
    }

    #[test]
    fn mid_flight_the_ship_is_drawn_between_its_last_two_positions() {
        let mut view = flight();
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 20);
        let before = player(&view);
        ticks(&mut view, 1);
        let after = player(&view);
        view.tick(TICK / 4);
        let shown = view.shown_position();
        let expected = |from: f32, to: f32| from + (to - from) * view.alpha();
        assert!(
            before.position.x > 1.0 && before.position.y < -1.0,
            "{before:?}"
        );
        assert!((shown.x - expected(before.position.x, after.position.x)).abs() < 1e-4);
        assert!((shown.y - expected(before.position.y, after.position.y)).abs() < 1e-4);
    }

    #[test]
    fn the_heading_is_drawn_the_short_way_round_across_0() {
        let mut view = flight();
        view.input(&key(Key::Left, true));
        view.tick(TICK);
        assert_eq!(player(&view).heading, 357.0);
        view.tick(TICK / 2);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
        view.input(&key(Key::Left, false));
        view.input(&key(Key::Right, true));
        view.tick(REST_OF_TICK);
        view.tick(TICK / 2);
        // From 357 to 0: half way is 358.5 again, not 178.5.
        assert_eq!(player(&view).heading, 0.0);
        assert!(
            (view.shown_heading() - 358.5).abs() < 1e-3,
            "{}",
            view.shown_heading()
        );
    }

    // Drawing.

    fn drawn(view: &View) -> DrawList {
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

    fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        list.iter()
            .filter_map(|command| match command {
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

    /// The radar's pixels drawn, in either radar colour, row by row as
    /// drawn.
    fn dots_on_radar(list: &DrawList) -> Vec<(i32, i32)> {
        let layout = layout();
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Dot {
                    center,
                    size: 1.0,
                    color,
                } if color == layout.dim_radar || color == layout.bright_radar => {
                    Some(((center.x - 0.5) as i32, (center.y - 0.5) as i32))
                }
                _ => None,
            })
            .collect()
    }

    /// The stock radar, with the bar at (830, 0): centred on (926, 96).
    const STOCK_RADAR: IRect = IRect::new(838, 8, 1014, 184);

    /// What the radar shows this frame, as marks on the stock radar.
    fn radar_marks(view: &View) -> Vec<RadarMark> {
        radar::marks(
            STOCK_RADAR,
            &view.radar_shown(),
            view.hud_cadence.blink_on(),
        )
    }

    /// The ships' blips on the radar this frame: its pixels but the
    /// player's, the last.
    fn blips(view: &View) -> Vec<RadarMark> {
        let mut marks: Vec<RadarMark> = radar_marks(view)
            .into_iter()
            .filter(|mark| matches!(mark, RadarMark::Pixel { .. }))
            .collect();
        marks.pop();
        marks
    }

    /// The 2×2 oval a small stellar at radar pixel (`x`, `y`) draws.
    fn small_oval(x: i32, y: i32) -> RadarMark {
        RadarMark::Oval {
            rect: IRect::around(x, y, 1),
            color: RadarColor::Dim,
        }
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
    };

    #[test]
    fn the_title_and_help_are_not_drawn_by_default() {
        assert_eq!(
            FlightOverlays::default(),
            FlightOverlays {
                title: false,
                help: false
            }
        );
        assert_eq!(flight().overlays(), FlightOverlays::default());
        assert_eq!(flight().with_overlays(TITLED).overlays(), TITLED);
        let texts = texts(&drawn(&flight()));
        assert!(!texts.contains(&"Sol (sÿst 130)".to_owned()), "{texts:?}");
        assert!(!texts.contains(&HELP.to_owned()), "{texts:?}");
    }

    /// The flight's draw list's commands up to the HUD's first, whose
    /// status bar background is the first picture.
    fn before_the_hud(list: &DrawList) -> Vec<DrawCommand> {
        list.iter()
            .take_while(|c| !matches!(c, DrawCommand::Picture { .. }))
            .cloned()
            .collect()
    }

    #[test]
    fn the_title_draws_when_turned_on() {
        let titled = FlightOverlays {
            title: true,
            help: false,
        };
        let list = drawn(&flight().with_overlays(titled));
        let before = before_the_hud(&list);
        assert_eq!(
            before.last(),
            Some(&overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE)),
            "over the flight, under the HUD"
        );
        assert!(!texts(&list).contains(&HELP.to_owned()));
    }

    #[test]
    fn the_help_line_draws_when_turned_on() {
        let helped = FlightOverlays {
            title: false,
            help: true,
        };
        let list = drawn(&flight().with_overlays(helped));
        let before = before_the_hud(&list);
        assert_eq!(
            before.last(),
            Some(&overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM)),
            "over the flight, under the HUD"
        );
        assert!(!texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
        let both = drawn(&flight().with_overlays(TITLED));
        let before = before_the_hud(&both);
        assert_eq!(
            before[before.len() - 2..],
            [
                overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE),
                overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM),
            ],
            "the title, then the help"
        );
    }

    #[test]
    fn it_draws_the_stars_the_stellars_the_ship_then_the_hud() {
        let view = flight();
        let list = drawn(&view);
        let mut expected = DrawList::new();
        starfield::draw(&mut expected, &view.camera());
        scene::draw_stellars(
            &mut expected,
            view.scene().expect("a scene"),
            &view.camera(),
            Duration::ZERO,
        );
        expected.sprite(ImageKey::sprite(2000, 0), VIEW_CENTER, Color::WHITE);
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &RadarScene {
                    player: at(0.0, 0.0),
                    stellars: vec![
                        RadarStellar {
                            position: at(0.0, -600.0),
                            square: false,
                            sprite_width: Some(20),
                            iff: StellarIff::Unlandable,
                        },
                        RadarStellar {
                            position: at(300.0, -200.0),
                            square: false,
                            sprite_width: Some(20),
                            iff: StellarIff::Unlandable,
                        },
                    ],
                    ..RadarScene::default()
                },
                blink_on: false,
                reserves: ShipStats::new(FIELDS, &[]).full(),
                nav: NavDisplay::Off,
                cargo: Some(&view.session().expect("flying").status_cargo()),
                date: DATE,
                disabled: false,
            },
            None,
        );
        let bar = view.status_bar().expect("a status bar");
        let origin = hud::bar_origin(bar);
        target::draw_target_panel(&mut expected, &bar.layout, origin, None, None);
        target::draw_secondary(&mut expected, &bar.layout, origin, None, None);
        assert_eq!(list, expected);
        assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 0), at(812.0, 184.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ]
        );
        assert_eq!(
            HELP,
            "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: preferences   Esc: leave"
        );
        assert_eq!((TITLE, HELP_AT), (at(16.0, 32.0), at(16.0, 744.0)));
    }

    #[test]
    fn the_ship_stays_at_the_screens_centre_as_the_stellars_go_by() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        view.tick(Duration::from_millis(1500));
        let list = drawn(&view);
        let shown = sprites(&list);
        let camera = view.camera();
        assert_eq!(
            shown[2],
            (
                ImageKey::sprite(2000, view.frame().expect("a sheet")),
                VIEW_CENTER
            )
        );
        assert_ne!(view.frame(), Some(0));
        assert_eq!(shown[0].1, camera.world_to_screen(at(0.0, -600.0)));
        assert_ne!(shown[0].1, at(512.0, -216.0), "the stellars moved");
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &camera);
        assert!(
            list.iter().take(stars.len()).eq(stars.iter()),
            "stars under the camera"
        );
    }

    #[test]
    fn the_stellars_animate_with_time() {
        let mut view = flight();
        ticks(&mut view, 1);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 1));
        ticks(&mut view, 2);
        assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 3));
    }

    #[test]
    fn a_ship_without_a_sheet_is_a_placeholder_with_the_reason() {
        let sheetless = FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..catalog()
        };
        let view = FlightView::new(sheetless);
        assert_eq!(view.frame(), None);
        let list = drawn(&view);
        assert_eq!(sprites(&list).len(), 2, "the stellars only");
        let mut expected = DrawList::new();
        crossed_box(&mut expected, VIEW_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
        assert_eq!(
            lines(&list)[..expected.len()],
            *lines(&expected),
            "then the HUD's"
        );
        let reason = list
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite")))
            .cloned();
        assert_eq!(
            reason,
            Some(overlay(
                "Sprite unavailable: no shän 128 for shïp 128",
                at(
                    VIEW_CENTER.x - PLACEHOLDER_SIZE / 2.0,
                    VIEW_CENTER.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP
                ),
                OVERLAY_SIZE,
                Color::ERROR
            ))
        );
        assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
    }

    // The engine glow and running lights.

    fn layer(image_id: i16, frames: u16) -> LayerSheet {
        LayerSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
        }
    }

    /// [`sheet`] with a 36-frame glow, `rlëD` 2100, and 12-frame lights,
    /// `rlëD` 2200.
    fn layered() -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The ship's sprites drawn: those from `rlëD`s 2000 to 2299.
    fn ship_sprites(view: &View) -> Vec<(ImageKey, Point)> {
        sprites(&drawn(view))
            .into_iter()
            .filter(|(image, _)| (2000..2300).contains(&image.id))
            .collect()
    }

    #[test]
    fn a_coasting_ship_draws_its_lights_over_it_and_no_glow() {
        let mut view = FlightView::new(layered());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
                (ImageKey::sprite(2200, 0), VIEW_CENTER),
            ]
        );
        view.input(&key(Key::Up, true));
        assert_eq!(ship_sprites(&view).len(), 2, "held, but not yet flown");
        ticks(&mut view, 3);
        view.input(&key(Key::Up, false));
        // As many ticks coasting as thrusting: the glow has faded out.
        ticks(&mut view, 3);
        let session = view.session().expect("flying");
        assert!(!session.thrusting());
        assert_eq!(session.engine_glow(), 0);
        assert_eq!(
            ship_sprites(&view)
                .iter()
                .map(|(image, _)| image.id)
                .collect::<Vec<_>>(),
            [2000, 2200],
            "coasting"
        );
    }

    #[test]
    fn a_thrusting_ship_draws_its_glow_between_it_and_its_lights() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        // A base of 6: drawn at every roll.
        ticks(&mut view, 6);
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, 0), centre),
                (ImageKey::sprite(2100, 0), centre),
                (ImageKey::sprite(2200, 0), centre),
            ]
        );
    }

    /// The ship's sprites' image ids and blends, as [`ship_sprites`].
    fn ship_blends(view: &View) -> Vec<(i16, Blend)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, blend, .. } if (2000..2300).contains(&image.id) => {
                    Some((image.id, *blend))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_thrusting_ships_glow_and_lights_or_onto_its_normal_sprite() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or), (2200, Blend::Or),]
        );
        assert!(
            !drawn(&view).iter().any(|command| matches!(
                command,
                DrawCommand::Sprite {
                    blend: Blend::Additive,
                    ..
                }
            )),
            "nothing in a thrusting, lit frame is added"
        );
        view.input(&key(Key::Up, false));
        ticks(&mut view, 6);
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2200, Blend::Or)],
            "coasting"
        );
    }

    #[test]
    fn the_layers_turn_with_the_ship_between_steps() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        view.input(&key(Key::Right, true));
        // 50 steps and a half: the display is between two steps.
        ticks(&mut view, 50);
        view.tick(TICK / 2);
        assert_ne!(view.shown_heading(), player(&view).heading, "between");
        let frame = view.frame().expect("a sheet");
        assert_eq!(frame, 15, "{}", view.shown_heading());
        let centre = view.camera().world_to_screen(view.shown_position());
        assert_eq!(
            ship_sprites(&view),
            [
                (ImageKey::sprite(2000, frame), centre),
                (ImageKey::sprite(2100, frame), centre),
                (ImageKey::sprite(2200, frame % 12), centre),
            ]
        );
    }

    #[test]
    fn landing_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "landed");
    }

    #[test]
    fn beginning_a_jump_while_thrusting_puts_the_glow_out() {
        let mut view = FlightView::new(layered());
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 6);
        assert_eq!(ship_sprites(&view).len(), 3, "glowing");
        jump_now(&mut view);
        assert_eq!(view.session().expect("flying").engine_glow(), 0);
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000, 2200], "jumping");
    }

    #[test]
    fn a_ship_without_layers_draws_only_its_sprite() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert!(view.session().expect("flying").thrusting());
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2000]);
    }

    #[test]
    fn a_ship_without_a_sheet_draws_no_layers() {
        let mut view = FlightView::new(FakeCatalog {
            sheet: Err("no shän 128 for shïp 128".to_owned()),
            ..layered()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(ship_sprites(&view), []);
    }

    /// Every command drawing the glow, `rlëD` 2100, whatever its blend.
    fn glow_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2100),
            )
            .cloned()
            .collect()
    }

    /// The glow commands for a base of `base` at `tick`: the glow drawn by
    /// OR at the level [`glow_level`] gives over the ship's centre, on its
    /// frame, or none when it is hidden.
    fn glow_for(view: &View, base: u8, tick: u64) -> Vec<DrawCommand> {
        let frame = view.frame().expect("a sheet") % 36;
        let center = view.camera().world_to_screen(view.shown_position());
        glow_level(base, tick, &view.glow_rolls)
            .map(|level| DrawCommand::Sprite {
                image: ImageKey::sprite(2100, frame),
                center,
                tint: lights_tint(level),
                blend: Blend::Or,
            })
            .into_iter()
            .collect()
    }

    #[test]
    fn the_glow_ramps_in_and_flickers_at_the_glow_function_level() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        let mut cruising = Vec::new();
        for n in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = n.min(GLOW_CRUISE);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, u64::from(n));
            assert_eq!(glow_commands(&view), want, "tick {n}");
            if n >= 6 {
                assert_eq!(want.len(), 1, "a base of 6 or more always shows");
            }
            if n >= 24 {
                cruising.extend(want);
            }
        }
        cruising.dedup();
        assert!(cruising.len() >= 2, "it flickers: {cruising:?}");
    }

    #[test]
    fn releasing_up_fades_the_glow_out() {
        let mut view = FlightView::new(layered());
        view.input(&key(Key::Up, true));
        ticks(&mut view, 30);
        assert_eq!(view.session().expect("flying").engine_glow(), GLOW_CRUISE);
        view.input(&key(Key::Up, false));
        for k in 1..=40_u8 {
            ticks(&mut view, 1);
            let base = GLOW_CRUISE.saturating_sub(k);
            assert_eq!(view.session().expect("flying").engine_glow(), base);
            let want = glow_for(&view, base, 30 + u64::from(k));
            assert_eq!(glow_commands(&view), want, "{k} ticks after");
            if k == 3 {
                let level = glow_level(21, 33, &view.glow_rolls).expect("still drawn");
                let alpha = lights_tint(level).a;
                assert!(alpha < 255 && alpha >= lights_tint(17).a, "{alpha}");
                assert_eq!(want.len(), 1, "fading, not out");
            }
            if k >= 24 {
                assert_eq!(want, [], "out {k} ticks after");
            }
        }
    }

    // The running lights' blinking.

    /// The Shuttle's blink: a double flash every 40 ticks, lit at ticks
    /// 1-3 and 10-12.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// [`layered`], with the lights blinking by `blink`.
    fn blinking(blink: Blink) -> FakeCatalog {
        FakeCatalog {
            sheet: Ok(ShipSheet {
                glow: Some(layer(2100, 36)),
                lights: Some(layer(2200, 12)),
                blink,
                ..sheet()
            }),
            ..catalog()
        }
    }

    /// The tints the lights, `rlëD` 2200, are drawn by OR with.
    fn lights_tints(view: &View) -> Vec<Color> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    tint,
                    blend: Blend::Or,
                    ..
                } if image.id == 2200 => Some(*tint),
                _ => None,
            })
            .collect()
    }

    /// The lights' alphas drawn: one, or none when they are off.
    fn lights_alphas(view: &View) -> Vec<u8> {
        lights_tints(view).iter().map(|tint| tint.a).collect()
    }

    #[test]
    fn the_shuttles_lights_blink_with_game_time() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 0: off");
        assert_eq!(ship_sprites(&view)[0].0.id, 2000, "the ship is drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_tints(&view), [Color::WHITE], "tick 1: on");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4: off");
        ticks(&mut view, 6);
        assert_eq!(lights_alphas(&view), [255], "tick 10: on again");
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 13: the gap");
    }

    #[test]
    fn a_pulsing_light_is_ored_at_its_level() {
        // The stock triangle: 10 to 31.
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let mut view = FlightView::new(blinking(triangle));
        assert_eq!(lights_alphas(&view), [0_u8; 0], "0.75: not drawn");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [8], "level 1");
        ticks(&mut view, 40);
        assert_eq!(lights_alphas(&view), [247], "level 31");
    }

    /// Every command drawing the lights, `rlëD` 2200, whatever its blend.
    fn lights_commands(view: &View) -> Vec<DrawCommand> {
        drawn(view)
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2200),
            )
            .cloned()
            .collect()
    }

    /// The lights drawn by OR with `tint` over the coasting ship's centre.
    fn lights_at(tint: Color) -> DrawCommand {
        DrawCommand::Sprite {
            image: ImageKey::sprite(2200, 0),
            center: VIEW_CENTER,
            tint,
            blend: Blend::Or,
        }
    }

    #[test]
    fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
        // The stock triangle, 10 to 31, never full: the original draws
        // every level with its translucent blit, which ORs in the lights
        // scaled by level/32 (see `lights_tint`).
        let triangle = Blink {
            mode: 2,
            a: 10,
            b: 75,
            c: 32,
            d: 75,
        };
        let rolls = HashedRolls::new(0);
        let mut view = FlightView::new(blinking(triangle));
        let mut levels = Vec::new();
        for tick in 0..=101 {
            let level = lights_level(&triangle, tick, &rolls);
            let want: Vec<_> = level
                .map(|n| lights_at(lights_tint(n)))
                .into_iter()
                .collect();
            assert_eq!(lights_commands(&view), want, "tick {tick}");
            levels.extend(level);
            ticks(&mut view, 1);
        }
        assert!(levels.contains(&10), "the lowest level is drawn");
        assert!(levels.contains(&31), "the highest level is drawn");

        // A steady light is full: ORed whole.
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_commands(&view), [lights_at(Color::WHITE)]);
    }

    #[test]
    fn steady_lights_are_ored_at_full_from_the_start() {
        let view = FlightView::new(blinking(Blink::STEADY));
        assert_eq!(lights_tints(&view), [Color::WHITE]);
    }

    #[test]
    fn the_glow_still_shows_while_the_lights_are_off() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        view.input(&key(Key::Up, true));
        // Tick 13: the Shuttle's lights are dark, and a base of 13 always
        // shows the glow.
        ticks(&mut view, 13);
        assert!(view.session().expect("flying").thrusting());
        assert_eq!(
            ship_blends(&view),
            [(2000, Blend::Normal), (2100, Blend::Or)]
        );
        let glow = drawn(&view).iter().find_map(|command| match command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 2100 => Some(*tint),
            _ => None,
        });
        let level = glow_level(13, 13, &view.glow_rolls).expect("drawn");
        assert_eq!(glow, Some(lights_tint(level)));
    }

    #[test]
    fn the_blink_pauses_while_the_map_is_open() {
        let mut view = FlightView::new(blinking(SHUTTLE));
        ticks(&mut view, 3);
        assert_eq!(lights_alphas(&view), [255], "tick 3");
        view.input(&key(MAP_KEY, true));
        assert!(view.map_open());
        // Tick 103 would be dark.
        ticks(&mut view, 100);
        view.close_map();
        assert_eq!(lights_alphas(&view), [255], "still tick 3");
        ticks(&mut view, 1);
        assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4");
    }

    #[test]
    fn random_lights_roll_on_hashed_rolls() {
        // A=5 B=12 C=3: one of 8 levels from 5, every 4 ticks.
        let random = Blink {
            mode: 3,
            a: 5,
            b: 12,
            c: 3,
            d: 0,
        };
        let rolls = HashedRolls::new(0);
        let expected = |change| lights_tint(5 + rolls.roll(change, 8) as u8).a;
        let mut view = FlightView::new(blinking(random));
        assert_eq!(lights_alphas(&view), [expected(0)]);
        ticks(&mut view, 4);
        assert_eq!(lights_alphas(&view), [expected(1)]);
    }

    // The HUD.

    #[test]
    fn a_new_pilot_shows_the_default_status_bar() {
        let view = flight();
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: layout(),
                background: Some(hud::Background {
                    id: 700,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
    }

    #[test]
    fn the_hud_shows_the_status_bar_full_bars_and_the_stellars_on_radar() {
        let view = flight();
        let list = drawn(&view);
        assert!(
            list.iter().any(|c| *c
                == DrawCommand::Picture {
                    image: ImageKey::picture(700),
                    top_left: at(830.0, 0.0),
                }),
            "{list:?}"
        );
        // The shïp's shield 40, armour 60 and fuel 250: all of it in the
        // full colour, then the half jump past the two whole ones (from
        // 119.2 pixels) over it.
        let width = 149.0;
        let bar = |top: f32, from: f32, to: f32, color| DrawCommand::Line {
            from: at(865.0 + from, top + 3.5),
            to: at(865.0 + to, top + 3.5),
            width: 7.0,
            color,
        };
        let layout = layout();
        assert_eq!(
            lines(&list),
            [
                bar(199.0, 0.0, width, layout.shield_color),
                bar(216.0, 0.0, width, layout.armor_color),
                bar(234.0, 0.0, width, layout.fuel_full),
                bar(234.0, 119.0, width, layout.fuel_partial),
            ]
        );
        // Earth, 600 up, is 18.75 pixels up the radar, at 77.25; the Moon,
        // (300, -200), at (935.375, 89.75): small ovals, dim, then the
        // player, bright.
        assert_eq!(
            dots_on_radar(&list),
            [
                (925, 76),
                (926, 76),
                (925, 77),
                (926, 77),
                (934, 88),
                (935, 88),
                (934, 89),
                (935, 89),
                (926, 96)
            ]
        );
        assert_eq!(
            radar_marks(&view),
            [small_oval(926, 77), small_oval(935, 89), PLAYER_PIXEL]
        );
        // No system name in the nav area (the dev title still names it).
        assert!(!texts(&list).contains(&"Sol".to_owned()), "{list:?}");
    }

    #[test]
    fn a_stellars_radar_shape_and_sprite_width_reach_the_radar() {
        // `w` x `h` frames: the radar reads the width, never the height.
        let sized = |id, at, (w, h): (u32, u32), square| StellarContents {
            sprite: Ok(StellarSheet {
                image_id: 1000 + id,
                frames: NonZeroU16::MIN,
                frame_width: w,
                frame_height: h,
            }),
            radar_square: square,
            ..stellar(id, "Extra", at, 1)
        };
        let view = FlightView::new(FakeCatalog {
            sol_more: vec![
                sized(150, (640, 0), (300, 300), true),
                sized(151, (-640, 320), (120, 60), false),
                sized(152, (0, 640), (250, 150), false),
            ],
            ..catalog()
        });
        let dim = RadarColor::Dim;
        // A station is a 2×2 square whatever its size; past 89 pixels wide
        // an oval grows 2 a side, past 199, 3.
        assert_eq!(
            radar_marks(&view),
            [
                small_oval(926, 77),
                small_oval(935, 89),
                RadarMark::Square {
                    rect: IRect::around(946, 96, 1),
                    color: dim,
                },
                RadarMark::Oval {
                    rect: IRect::around(906, 106, 2),
                    color: dim,
                },
                RadarMark::Oval {
                    rect: IRect::around(926, 116, 3),
                    color: dim,
                },
                PLAYER_PIXEL,
            ]
        );
    }

    /// The radar's player position: where it was when the radar last
    /// refreshed, or now.
    fn radar_player(view: &View) -> Point {
        view.radar_shown().player
    }

    #[test]
    fn by_the_engine_the_radar_holds_between_refreshes_eight_ticks_apart() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        let refreshed = view.shown_position();
        assert_eq!(radar_player(&view), refreshed, "refreshed on the first");
        for tick in 2..=8 {
            ticks(&mut view, 1);
            assert_eq!(radar_player(&view), refreshed, "held on tick {tick}");
        }
        assert_ne!(view.shown_position(), refreshed, "the ship moved");
        ticks(&mut view, 1);
        assert_eq!(radar_player(&view), view.shown_position(), "the 9th");
        assert_eq!(
            radar_marks(&view)[0],
            small_oval(
                926,
                radar::radar_offset(-600.0, view.shown_position().y, 96)
            )
        );
    }

    /// `view`'s flight with [`RuleKey::HudCadence`] read the Bible's way:
    /// the HUD every frame.
    fn every_frame(view: View) -> View {
        view.with_rules(Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible))
    }

    #[test]
    fn by_the_other_reading_the_radar_follows_the_ship_every_frame() {
        let mut view = every_frame(flight());
        let before = dots_on_radar(&drawn(&view));
        view.input(&key(Key::Up, true));
        for _ in 0..45 {
            ticks(&mut view, 1);
            assert_eq!(radar_player(&view), view.shown_position());
        }
        view.tick(TICK / 2);
        let shown = view.shown_position();
        assert!(shown.y < -10.0, "{shown:?}");
        assert_eq!(radar_player(&view), shown, "between ticks too");
        let after = dots_on_radar(&drawn(&view));
        let y = |world| radar::radar_offset(world, shown.y, 96);
        assert_eq!(
            radar_marks(&view),
            [
                small_oval(926, y(-600.0)),
                small_oval(935, y(-200.0)),
                PLAYER_PIXEL
            ]
        );
        assert!(after[0].1 > before[0].1, "{before:?} {after:?}");
    }

    #[test]
    fn while_landed_the_radar_shows_only_the_player_until_take_off() {
        for every in [false, true] {
            let view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
            let mut view = if every { every_frame(view) } else { view };
            ticks(&mut view, 3);
            let flying = radar_marks(&view);
            assert_eq!(flying.len(), 3, "the Sun, the Moon and the player");
            land_now(&mut view);
            assert!(view.session().expect("flying").landed().is_some());
            assert!(view.radar_shown().blank, "at once, as _RedrawStatus");
            assert_eq!(radar_marks(&view), [PLAYER_PIXEL]);
            let list = drawn(&view);
            assert_eq!(dots_on_radar(&list), [(926, 96)]);
            view.take_off().expect("took off");
            assert!(!view.radar_shown().blank, "at once on take-off");
            assert_eq!(radar_marks(&view).len(), 3);
        }
    }

    #[test]
    fn a_new_system_shows_on_the_radar_at_once_leaving_the_timer_and_blink() {
        let mut view = landed_view();
        ticks(&mut view, 3);
        let cadence = view.hud_cadence;
        move_to(&mut view, 131, 140);
        view.resync();
        assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
        let shown = view.radar_shown();
        assert_eq!(shown.player, view.shown_position());
        let stellars: Vec<Point> = shown.stellars.iter().map(|s| s.position).collect();
        let scene: Vec<Point> = view
            .scene()
            .expect("a scene")
            .stellars()
            .iter()
            .map(|s| s.position)
            .collect();
        assert_eq!(stellars, scene, "Alpha Centauri's");
    }

    #[test]
    fn an_arrival_shows_the_new_systems_stellars_at_once() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = view.session().expect("flying").system();
        for _ in 0..1000 {
            let cadence = view.hud_cadence;
            view.tick(TICK);
            if view.session().expect("flying").system() != leaving {
                let now = u64::try_from(crate::time::ticks(view.elapsed)).expect("in range");
                let mut probe = cadence;
                assert!(!probe.advance(now).radar, "not a refresh tick");
                assert_eq!(view.hud_cadence.blink_on(), cadence.blink_on());
                break;
            }
        }
        assert_ne!(view.session().expect("flying").system(), leaving);
        let stellars: Vec<Point> = view
            .radar_shown()
            .stellars
            .iter()
            .map(|s| s.position)
            .collect();
        assert_eq!(stellars, [at(0.0, 0.0)], "Alpha Centauri's");
        assert_eq!(view.radar_shown().player, view.shown_position());
    }

    #[test]
    fn the_targets_blip_blinks_bright_every_eight_ticks_under_both_readings() {
        for every in [false, true] {
            let (_, chance) = scripted(&placed(850, 650, 90));
            let view = FlightView::new(trafficked(&[130], 1, 129, 3))
                .with_chance(chance)
                .with_behaviour(Rc::new(Still));
            let mut view = if every { every_frame(view) } else { view };
            view.tick(TICK);
            tap(&mut view, TARGET_KEY);
            assert!(view.session().expect("flying").target().is_some());
            ticks(&mut view, 7);
            let mut seen = Vec::new();
            for _ in 0..24 {
                ticks(&mut view, 1);
                let [RadarMark::Pixel { color, .. }] = blips(&view)[..] else {
                    panic!("one blip: {:?}", blips(&view));
                };
                seen.push(color);
            }
            let mut expected = vec![RadarColor::Dim; 8];
            expected.extend([RadarColor::Bright; 8]);
            expected.extend([RadarColor::Dim; 8]);
            assert_eq!(seen, expected, "every frame: {every}");
            let list = drawn(&view);
            let bright = list.iter().filter(|command| {
                matches!(command, DrawCommand::Dot { color, .. } if *color == layout().bright_radar)
            });
            assert_eq!(bright.count(), 1, "the player alone, the target dim");
        }
    }

    #[test]
    fn the_bars_show_the_sessions_reserves() {
        let view = flight();
        let reserves = reserves(&view);
        assert_eq!(reserves, ShipStats::new(FIELDS, &[]).full());
        let mut expected = DrawList::new();
        hud::draw(
            &mut expected,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &RadarScene::default(),
                blink_on: false,
                reserves,
                nav: NavDisplay::Off,
                cargo: None,
                date: DATE,
                disabled: false,
            },
            None,
        );
        assert_eq!(lines(&drawn(&view)), lines(&expected));
    }

    #[test]
    fn an_unreadable_status_bar_says_why_and_flight_goes_on() {
        let barless = FakeCatalog {
            bar: Err("no ïntf 128".to_owned()),
            ..catalog()
        };
        let mut view = FlightView::new(barless);
        assert_eq!(view.status_bar(), Err("no ïntf 128"));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 5);
        assert_eq!(player(&view), stepped(start(), THRUST, 5));
        let list = drawn(&view);
        let mut reason = DrawList::new();
        hud::draw_unavailable(&mut reason, "no ïntf 128");
        assert_eq!(list.iter().last(), reason.iter().next());
        assert_eq!(lines(&list), [], "no bars");
        assert_eq!(dots_on_radar(&list), [], "no radar");
        assert_eq!(sprites(&list).len(), 3, "stellars and ship");
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

    #[test]
    fn a_ship_type_whose_government_has_its_own_interface_shows_that_bar() {
        let view = FlightView::new(governed(catalog(), 128, 129));
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: bar_129(),
                background: Some(hud::Background {
                    id: 705,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
        let list = drawn(&view);
        assert!(
            list.iter().any(|c| *c
                == DrawCommand::Picture {
                    image: ImageKey::picture(705),
                    top_left: at(830.0, 0.0),
                }),
            "{list:?}"
        );
        for interface in [127, 0, -1] {
            let view = FlightView::new(governed(catalog(), 128, interface));
            assert_eq!(
                view.status_bar().map(|bar| &bar.layout),
                Ok(&layout()),
                "{interface}"
            );
        }
    }

    #[test]
    fn a_session_that_cannot_start_has_no_status_bar() {
        let failed = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let view = FlightView::new(failed);
        assert_eq!(view.status_bar(), Err("no chär to start from"));
    }

    // Frame rate.

    /// Flies a script for three seconds of frames at `fps`, each redraw at
    /// `n / fps` seconds to the nanosecond: thrust for a second, turn
    /// right for half a second, reverse for a second, then coast.
    fn fly(fps: u64) -> ShipState {
        let mut view = flight();
        let script = [
            (0, Key::Up, true),
            (2, Key::Up, false),
            (2, Key::Right, true),
            (3, Key::Right, false),
            (3, Key::Down, true),
            (5, Key::Down, false),
        ];
        let mut last = 0;
        for frame in 1..=3 * fps {
            let half_second = (frame - 1) * 2 / fps;
            if (frame - 1) * 2 % fps == 0 {
                for &(_, k, pressed) in script.iter().filter(|(at, ..)| *at == half_second) {
                    view.input(&key(k, pressed));
                }
            }
            let now = frame * 1_000_000_000 / fps;
            view.tick(Duration::from_nanos(now - last));
            last = now;
        }
        player(&view)
    }

    #[test]
    fn the_same_keys_fly_the_same_at_30_60_and_120_fps() {
        let at_30 = fly(30);
        assert!(at_30.heading > 0.0, "{at_30:?}");
        assert!(at_30.position.y < 0.0, "{at_30:?}");
        assert_eq!(fly(60), at_30);
        assert_eq!(fly(120), at_30);
    }

    // Landing.

    /// The fake catalog with system 130 holding just `sites`.
    fn flight_among(sites: Vec<LandingSite>) -> View {
        FlightView::new(FakeCatalog { sites, ..catalog() })
    }

    const LAND: Key = Key::Char('l');

    /// The message drawn at its place, if any.
    fn message(view: &View) -> Option<DrawCommand> {
        drawn(view)
            .iter()
            .find(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
            .cloned()
    }

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

    /// Asserts the message line says `text` now, for `n` ticks of flight
    /// in all, and not after.
    #[track_caller]
    fn assert_lasts(view: &mut View, text: &str, n: u16) {
        assert_eq!(view.message(), Some(text));
        ticks(view, u32::from(n) - 1);
        assert_eq!(
            view.message(),
            Some(text),
            "still shown after {} ticks",
            n - 1
        );
        ticks(view, 1);
        assert_eq!(view.message(), None, "gone after {n} ticks");
    }

    /// L pressed twice: clearance, then landing.
    fn land_now(view: &mut View) {
        tap(view, LAND);
        tap(view, LAND);
    }

    #[test]
    fn l_requests_clearance_and_a_second_l_lands() {
        // The Moon (129), in the scene, over the ship.
        let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(view.take_landing(), None, "cleared, not landed");
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        let cleared = "Moon traffic control reads you, you're cleared to land.";
        assert_eq!(view.message(), Some(cleared));
        assert_eq!(message(&view), Some(message_drawn(cleared)));
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the HUD shows it");
        assert_eq!(view.take_sounds(), [], "no sound for clearance");

        view.input(&key(LAND, false));
        assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(129))
        );
        assert_eq!(view.take_landing(), Some(StellarId(129)));
        assert_eq!(view.take_landing(), None, "taken");
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(LAND_KEY, LAND);
    }

    #[test]
    fn every_clearance_has_its_message() {
        use Clearance::{Denied, Granted, NoTrafficControl};
        let cases = [
            (
                false,
                Granted,
                "Earth traffic control reads you, you're cleared to land.",
            ),
            (
                true,
                Granted,
                "Earth dockmaster reads you, you're cleared to dock.",
            ),
            (false, NoTrafficControl, "You are cleared to land."),
            (true, NoTrafficControl, "You are cleared to dock."),
            (false, Denied, "Landing request denied."),
            (true, Denied, "Docking request denied."),
        ];
        for (station, clearance, text) in cases {
            assert_eq!(
                clearance_message("Earth", station, clearance),
                text,
                "{station} {clearance:?}"
            );
        }
        assert_eq!(
            [
                TRAFFIC_CONTROL_READS_YOU,
                DOCKMASTER_READS_YOU,
                CLEARED_TO_LAND,
                CLEARED_TO_DOCK,
                YOU_ARE_CLEARED_TO_LAND,
                YOU_ARE_CLEARED_TO_DOCK,
            ],
            [
                "traffic control reads you",
                "dockmaster reads you",
                "you're cleared to land.",
                "you're cleared to dock.",
                "You are cleared to land.",
                "You are cleared to dock.",
            ]
        );
    }

    #[test]
    fn the_clearance_names_the_stellar_selected_and_says_how_it_answered() {
        let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        let mut view = flight_among(vec![site(128, (0.0, 0.0), uninhabited)]);
        tap(&mut view, LAND);
        assert_lasts(&mut view, "You are cleared to land.", 250);
        let strict = LandingSite {
            min_status: 1,
            ..site(
                129,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::STATION,
            )
        };
        let mut view = flight_among(vec![strict]);
        tap(&mut view, LAND);
        assert_eq!(nav_target(&view), Some(StellarId(129)), "selected anyway");
        assert_lasts(&mut view, "Docking request denied.", 250);
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(
            view.message(),
            Some("Moon traffic control reads you, you're cleared to land."),
            "the nearer of Sol's two"
        );
        assert_lasts(
            &mut view,
            "Moon traffic control reads you, you're cleared to land.",
            LANDING_TICKS,
        );
    }

    #[test]
    fn a_repeat_or_release_of_l_does_nothing() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        view.input(&held(LAND));
        view.input(&key(LAND, false));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(nav_target(&view), None, "nothing selected");
        let mut far = flight();
        far.input(&held(LAND));
        assert_eq!(far.message(), None, "no refusal either");
    }

    #[test]
    fn each_refusal_says_why_in_the_originals_words() {
        // L, then L again: the first requests clearance where it can.
        let refused = |sites: Vec<LandingSite>| {
            let mut view = flight_among(sites);
            tap(&mut view, LAND);
            tap(&mut view, LAND);
            assert_eq!(view.take_landing(), None);
            view.message().map(str::to_owned)
        };
        let centre = |flags, min_status| LandingSite {
            min_status,
            ..site(140, (0.0, 0.0), flags)
        };
        let station = StellarFlags::CAN_LAND | StellarFlags::STATION;
        assert_eq!(
            refused(Vec::new()).as_deref(),
            Some("No stellar objects present.")
        );
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), StellarFlags::CAN_LAND)]).as_deref(),
            Some("You're too far away to land on this planet.")
        );
        assert_eq!(
            refused(vec![site(140, (0.0, -600.0), station)]).as_deref(),
            Some("You're too far away to dock at this station.")
        );
        assert_eq!(
            refused(vec![centre(0, 0)]).as_deref(),
            Some("The planet's environment is too hostile.")
        );
        assert_eq!(
            refused(vec![centre(StellarFlags::CAN_LAND, 1)]).as_deref(),
            Some("Landing request denied.")
        );
        assert_eq!(
            refused(vec![centre(station, 32767)]).as_deref(),
            Some("Docking request denied.")
        );
    }

    #[test]
    fn a_ship_moving_too_fast_is_told_so() {
        let big = |flags| LandingSite {
            frame_size: Some((400, 400)),
            ..site(140, (0.0, 0.0), flags)
        };
        for (flags, expected) in [
            (
                StellarFlags::CAN_LAND,
                "You're moving too fast to land on this planet.",
            ),
            (
                StellarFlags::CAN_LAND | StellarFlags::STATION,
                "You're moving too fast to dock at this station.",
            ),
        ] {
            let mut view = flight_among(vec![big(flags)]);
            view.input(&key(Key::Up, true));
            ticks(&mut view, 15);
            land_now(&mut view);
            assert_eq!(view.take_landing(), None);
            assert_eq!(view.message(), Some(expected));
            assert_eq!(nav_target(&view), Some(StellarId(140)), "kept: try again");
        }
    }

    #[test]
    fn each_refusal_and_assignment_lasts_its_callers_ticks() {
        let stellar = StellarId(128);
        let landing = [
            (LandingRefusal::Jumping, 240),
            (LandingRefusal::Disabled, 240),
            (LandingRefusal::NoStellars, 240),
            (
                LandingRefusal::TooFar {
                    stellar,
                    station: false,
                },
                360,
            ),
            (
                LandingRefusal::TooFast {
                    stellar,
                    station: true,
                    speed: 2.0,
                },
                360,
            ),
            (
                LandingRefusal::Denied {
                    stellar,
                    station: false,
                    min_status: 1,
                },
                250,
            ),
            (
                LandingRefusal::NotLandable {
                    stellar,
                    station: false,
                },
                250,
            ),
        ];
        for (refusal, ticks) in landing {
            assert_eq!(landing_refusal_ticks(&refusal), ticks, "{refusal:?}");
        }
        let jump = [
            (JumpRefusal::NoDestination, 360),
            (JumpRefusal::TooClose { distance: 1.0 }, 360),
            (JumpRefusal::NoFuel { fuel: 1.0 }, 240),
            (JumpRefusal::Landed, 240),
            (JumpRefusal::Disabled, 240),
        ];
        for (refusal, ticks) in jump {
            assert_eq!(jump_refusal_ticks(&refusal), ticks, "{refusal:?}");
        }
        assert_eq!(assigned_ticks(Assigned::Escort), 240);
        assert_eq!(assigned_ticks(Assigned::MyShip), 360);
        assert_eq!(assigned_ticks(Assigned::Abandoned), 360);
        assert_eq!(
            [
                MESSAGE_TICKS,
                HAIL_QUOTE_TICKS,
                GRANT_TICKS,
                BOARD_LINE_TICKS,
                BOARD_REFUSAL_TICKS,
                ESCORT_TICKS,
                SCRIPT_MESSAGE_TICKS,
                LANDING_TICKS,
                TOO_FAR_TICKS,
                GATE_REFUSAL_TICKS,
                JUMP_REFUSAL_TICKS,
                OLD_SHIP_TICKS,
            ],
            [240, 420, 240, 250, 360, 250, 500, 250, 360, 250, 360, 360]
        );
    }

    #[test]
    fn a_new_message_replaces_the_old_and_its_time() {
        let mut view = flight();
        view.say_for("A", 420);
        ticks(&mut view, 10);
        view.say_for("B", 240);
        assert_lasts(&mut view, "B", 240);
    }

    #[test]
    fn a_message_waits_out_the_map() {
        let mut view = flight();
        view.say_for("A", 240);
        ticks(&mut view, 40);
        tap(&mut view, MAP_KEY);
        assert!(view.map_open());
        ticks(&mut view, 300);
        tap(&mut view, MAP_KEY);
        assert!(!view.map_open());
        assert_lasts(&mut view, "A", 200);
    }

    #[test]
    fn starting_a_jump_clears_only_too_close() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, JUMP);
        assert_eq!(view.message(), Some(TOO_CLOSE));
        fly_out(&mut view);
        assert_eq!(view.message(), Some(TOO_CLOSE), "still shown");
        tap(&mut view, JUMP);
        assert!(view.preparing_jump(), "accepted");
        assert_eq!(view.message(), None, "#42 cleared");

        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, JUMP);
        assert!(view.preparing_jump(), "accepted");
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
    }

    #[test]
    fn every_refusal_has_its_string() {
        let stellar = StellarId(128);
        let cases = [
            (LandingRefusal::Jumping, IN_HYPERSPACE),
            (LandingRefusal::Disabled, LAND_DISABLED),
            (LandingRefusal::NoStellars, NO_STELLARS),
            (
                LandingRefusal::TooFar {
                    stellar,
                    station: true,
                },
                TOO_FAR_STATION,
            ),
            (
                LandingRefusal::TooFar {
                    stellar,
                    station: false,
                },
                TOO_FAR_PLANET,
            ),
            (
                LandingRefusal::NotLandable {
                    stellar,
                    station: true,
                },
                HOSTILE_STATION,
            ),
            (
                LandingRefusal::NotLandable {
                    stellar,
                    station: false,
                },
                HOSTILE_PLANET,
            ),
            (
                LandingRefusal::Denied {
                    stellar,
                    station: true,
                    min_status: 1,
                },
                DOCKING_DENIED,
            ),
            (
                LandingRefusal::Denied {
                    stellar,
                    station: false,
                    min_status: 1,
                },
                LANDING_DENIED,
            ),
            (
                LandingRefusal::TooFast {
                    stellar,
                    station: true,
                    speed: 2.0,
                },
                TOO_FAST_STATION,
            ),
            (
                LandingRefusal::TooFast {
                    stellar,
                    station: false,
                    speed: 2.0,
                },
                TOO_FAST_PLANET,
            ),
        ];
        for (refusal, text) in cases {
            assert_eq!(refusal_message(&refusal), text, "{refusal:?}");
        }
        assert_eq!(
            [
                NO_STELLARS,
                TOO_FAR_STATION,
                TOO_FAR_PLANET,
                TOO_FAST_STATION,
                TOO_FAST_PLANET,
                DOCKING_DENIED,
                LANDING_DENIED,
                HOSTILE_STATION,
                HOSTILE_PLANET,
                IN_HYPERSPACE,
            ],
            [
                "No stellar objects present.",
                "You're too far away to dock at this station.",
                "You're too far away to land on this planet.",
                "You're moving too fast to dock at this station.",
                "You're moving too fast to land on this planet.",
                "Docking request denied.",
                "Landing request denied.",
                "The station's hull integrity is too unstable.",
                "The planet's environment is too hostile.",
                "Unable to land - your ship is in hyperspace.",
            ]
        );
    }

    #[test]
    fn the_refusal_is_drawn_until_its_ticks_have_passed() {
        let mut view = flight();
        ticks(&mut view, 3);
        land_now(&mut view);
        let shown = Some(message_drawn(TOO_FAR_PLANET));
        assert_eq!(message(&view), shown);
        ticks(&mut view, 359);
        assert_eq!(message(&view), shown);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        ticks(&mut view, 1);
        assert_eq!(message(&view), None);
        assert_eq!(view.message(), None);
        assert_eq!(TOO_FAR_TICKS, 360);
    }

    #[test]
    fn the_message_draws_over_the_starfield_and_under_the_ships() {
        let mut view = flight();
        land_now(&mut view);
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &view.camera());
        let stars: Vec<&DrawCommand> = stars.iter().collect();
        assert!(!stars.is_empty());
        assert_eq!(commands[..stars.len()], stars[..], "the starfield first");
        assert_eq!(
            commands[stars.len()],
            &message_drawn(TOO_FAR_PLANET),
            "then the message"
        );
        let sprite = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Sprite { .. }))
            .expect("the player's sprite");
        assert!(stars.len() < sprite, "the ship over it");
    }

    #[test]
    fn a_long_message_wraps_in_the_box_by_the_metrics() {
        let mut view = flight().with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
        land_now(&mut view);
        let text = vec!["abcd"; 60].join(" ");
        view.say_for(text.clone(), MESSAGE_TICKS);
        let mut expected = DrawList::new();
        hud::draw_message(
            &mut expected,
            &text,
            Some(&crate::text::fixture::MonoMetrics),
        );
        let list = drawn(&view);
        let commands: Vec<&DrawCommand> = list.iter().collect();
        let first = commands
            .iter()
            .position(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
            .expect("the message");
        let expected: Vec<&DrawCommand> = expected.iter().collect();
        assert_eq!(expected.len(), 2);
        assert_eq!(commands[first..first + 2], expected[..]);
    }

    #[test]
    fn a_new_refusal_shows_afresh() {
        let mut view = flight();
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        view.input(&key(LAND, true));
        ticks(&mut view, 300);
        view.input(&key(LAND, false));
        view.input(&key(LAND, true));
        ticks(&mut view, 100);
        assert_eq!(
            view.message(),
            Some(TOO_FAR_PLANET),
            "360 ticks from the second"
        );
    }

    #[test]
    fn taking_off_draws_the_ship_at_the_stellar_on_the_next_frame() {
        let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(view.session().expect("flying").landed(), None);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), at(6.0, -8.0));
        assert_eq!(view.camera().center(), at(6.0, -8.0));
        assert_eq!(view.take_off(), None, "not landed");
        // It flies on from there.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 2);
        assert!(player(&view).position.y < -8.0, "{:?}", player(&view));
    }

    #[test]
    fn a_refusal_is_forgotten_once_the_ship_lands() {
        // 20 wide: within 35 on each axis is over it.
        let mut view = flight_among(vec![site(140, (0.0, -40.0), StellarFlags::CAN_LAND)]);
        land_now(&mut view);
        assert_eq!(view.message(), Some(TOO_FAR_PLANET));
        // Nudge the ship and drift over the planet.
        view.input(&key(Key::Up, true));
        ticks(&mut view, 1);
        view.input(&key(Key::Up, false));
        for _ in 0..100 {
            if player(&view).position.y <= -6.0 {
                break;
            }
            ticks(&mut view, 1);
        }
        assert!(player(&view).position.y <= -6.0, "{:?}", player(&view));
        assert_eq!(view.message(), Some(TOO_FAR_PLANET), "still on screen");
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        assert_eq!(view.message(), None);
        assert_eq!(message(&view), None);
        assert_eq!(view.take_off(), Some(StellarId(140)));
        assert_eq!(view.message(), None, "not back after take-off");
    }

    #[test]
    fn landing_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(LAND, true));
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_off(), None);
    }

    #[test]
    fn l_over_a_stellar_sounds_the_landing_with_its_own_sound() {
        let mut view = flight_among(vec![LandingSite {
            landing_sound: Some(SoundId(10_032)),
            ..site(140, (6.0, -8.0), StellarFlags::CAN_LAND)
        }]);
        assert_eq!(view.take_sounds(), []);
        tap(&mut view, LAND);
        assert_eq!(view.take_sounds(), [], "clearance is silent");
        tap(&mut view, LAND);
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Landed {
                stellar_sound: Some(SoundId(10_032))
            })]
        );
        assert_eq!(view.take_sounds(), [], "taken");
        view.take_off();
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TookOff)]);
    }

    #[test]
    fn thrust_sounds_as_it_starts_and_stops() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
        view.input(&key(Key::Up, false));
        ticks(&mut view, 1);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
    }

    #[test]
    fn a_flight_that_never_started_makes_no_sounds() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(view.take_sounds(), []);
    }

    // Hyperspace.

    const MAP: Key = Key::Char('m');
    const JUMP: Key = Key::Char('j');

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Presses and releases `k`.
    fn tap(view: &mut View, k: Key) {
        view.input(&key(k, true));
        view.input(&key(k, false));
    }

    /// Where system `id` is on the course map.
    fn on_map(view: &View, id: i16) -> Point {
        let map = view.course_map();
        let system = map.model().system(SystemId(id)).expect("on the map");
        map.view().world_to_screen(system.position())
    }

    /// Clicks the left button at `at`.
    fn click(view: &mut View, at: Point) {
        for pressed in [true, false] {
            view.input(&Input::PointerButton {
                button: crate::MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    /// Opens the map, picks system `id` and closes the map again.
    fn plot(view: &mut View, id: i16) {
        tap(view, MAP);
        let at = on_map(view, id);
        click(view, at);
        tap(view, MAP);
        assert!(!view.map_open());
    }

    /// Thrusts straight up until the ship is at least the minimum jump
    /// distance from the centre, then lets go of Up.
    fn fly_out(view: &mut View) {
        view.input(&key(Key::Up, true));
        for _ in 0..2000 {
            if player(view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        view.input(&key(Key::Up, false));
        assert!(
            player(view).position.length() >= MIN_JUMP_DISTANCE,
            "{:?}",
            player(view)
        );
    }

    #[test]
    fn the_course_map_starts_on_the_current_system_with_no_course() {
        let view = flight();
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        assert_eq!(view.course_map().route(), []);
        assert_eq!(view.jump_effect(), None);
        assert_eq!((MAP_KEY, JUMP_KEY), (MAP, JUMP));
    }

    #[test]
    fn m_opens_the_map_over_everything_and_lets_go_of_the_flight_keys() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        assert_eq!(view.input(&key(MAP, true)), ScreenAction::None);
        assert!(view.map_open());
        let mut map = DrawList::new();
        view.course_map().draw(&mut map);
        assert_eq!(drawn(&view), map, "only the map");
        // Holding M, or letting it go, keeps it open.
        view.input(&held(MAP));
        view.input(&key(MAP, false));
        assert!(view.map_open());
        view.input(&key(MAP, true));
        assert!(!view.map_open());
        ticks(&mut view, 5);
        assert_eq!(player(&view), start(), "Up was let go");
    }

    #[test]
    fn while_the_map_is_open_flight_is_paused_and_keys_go_to_the_map() {
        let mut view = flight();
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        let flying = player(&view);
        tap(&mut view, MAP);
        let fitted = *view.course_map().view();
        ticks(&mut view, 30);
        assert_eq!(player(&view), flying, "paused");
        view.input(&key(Key::Left, true));
        assert_ne!(*view.course_map().view(), fitted, "Left pans the map");
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.jump_effect(), None);
        tap(&mut view, MAP);
        ticks(&mut view, 1);
        assert_eq!(player(&view).heading, 0.0, "Left went to the map");
    }

    #[test]
    fn a_click_on_a_system_plots_the_course_there_and_the_map_shows_it() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert!(view.map_open());
        let course = [SystemId(131)];
        assert_eq!(view.session().expect("flying").course(), course);
        assert_eq!(view.course_map().route(), course);
        assert_eq!(view.course_map().current(), Some(SystemId(130)));
        let barnard = on_map(&view, 132);
        click(&mut view, barnard);
        assert_eq!(view.session().expect("flying").course(), []);
        assert_eq!(view.course_map().route(), []);
        assert!(
            texts(&drawn(&view)).contains(&"No hyperspace route".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    #[test]
    fn closing_the_map_abandons_its_drag() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: true,
            at: alpha,
        });
        view.close_map();
        assert!(!view.map_open());
        tap(&mut view, MAP);
        view.input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: false,
            at: alpha,
        });
        assert_eq!(view.course_map().selected(), None, "no click");
        assert_eq!(view.session().expect("flying").course(), []);
    }

    #[test]
    fn j_says_why_a_jump_is_refused_in_the_originals_words() {
        let mut view = flight();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(NO_DESTINATION));
        assert_eq!(message(&view), Some(message_drawn(NO_DESTINATION)));
        assert_lasts(&mut view, NO_DESTINATION, 360);
        plot(&mut view, 131);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_lasts(&mut view, TOO_CLOSE, 360);

        let mut dry = FlightView::new(FakeCatalog {
            fields: ShipFields { fuel: 50, ..FIELDS },
            ..catalog()
        });
        plot(&mut dry, 131);
        fly_out(&mut dry);
        dry.input(&key(JUMP, true));
        assert_eq!(dry.jump_effect(), None);
        assert_lasts(&mut dry, NO_FUEL, 240);
    }

    #[test]
    fn a_repeat_or_release_of_j_does_nothing() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&held(JUMP));
        view.input(&key(JUMP, false));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
    }

    #[test]
    fn every_jump_refusal_has_its_string() {
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoDestination),
            NO_DESTINATION
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::TooClose { distance: 1.0 }),
            TOO_CLOSE
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoFuel { fuel: 1.0 }),
            NO_FUEL
        );
        assert_eq!(jump_refusal_message(&JumpRefusal::Landed), TAKE_OFF_FIRST);
        assert_eq!(jump_refusal_message(&JumpRefusal::Disabled), JUMP_DISABLED);
        assert_eq!(
            [LAND_DISABLED, JUMP_DISABLED],
            [
                "Unable to land - your ship is disabled.",
                "Can't initiate hyperspace jump - your ship is disabled."
            ]
        );
        assert_eq!(
            [NO_DESTINATION, TOO_CLOSE, NO_FUEL, TAKE_OFF_FIRST],
            [
                "You have to select a destination before you can start a hyperspace jump.",
                "Can't initiate hyperspace jump - not yet far enough away from system center.",
                "Insufficient energy for hyperspace jump.",
                "Can't initiate hyperspace jump - take off first.",
            ]
        );
    }

    #[test]
    fn j_while_landed_is_refused_until_take_off() {
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
        plot(&mut view, 131);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(140)));
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.session().expect("flying").jumping(), None);
        assert_eq!(view.message(), Some(TAKE_OFF_FIRST));
        view.take_off();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(TOO_CLOSE), "flying again");
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
            .position(|c| matches!(c, DrawCommand::Picture { image, .. } if *image == ImageKey::picture(700)))
            .expect("the HUD")
    }

    /// Where the ship's sprite is drawn.
    fn ship_at(list: &DrawList) -> usize {
        list.iter()
            .position(|c| matches!(c, DrawCommand::Sprite { image, .. } if image.id == 2000))
            .expect("the ship")
    }

    #[test]
    fn a_jump_streaks_the_stars_then_fades_out_arrives_and_fades_in() {
        let mut view = flight().with_overlays(TITLED);
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = player(&view);
        let fuel_leaving = reserves(&view).fuel.now;
        assert_eq!(view.message(), None);
        let effect = view.jump_effect().expect("jumping");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to Alpha Centauri");
        assert_eq!(
            view.session().expect("flying").jumping(),
            Some(SystemId(131))
        );

        // The stars streak; nothing fades.
        view.tick(ms(500));
        assert_eq!(player(&view), leaving, "frozen");
        let list = drawn(&view);
        let mut streaks = DrawList::new();
        starfield::draw_streaked(&mut streaks, &view.camera(), at(1.0, 0.0), 256.0);
        assert!(!streaks.is_empty());
        assert!(list.iter().take(streaks.len()).eq(streaks.iter()));
        assert_eq!(fade(&list), None);
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));

        // The old system fades out to white, HUD and all.
        view.tick(ms(625));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color, Color::rgba(255, 255, 255, 64));
        assert!(ship_at(&list) < hud_at(&list) && hud_at(&list) < at_fade);
        assert_eq!(at_fade, list.len() - 1, "the fade is drawn last");
        assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
        view.tick(ms(125));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(128), "growing");
        assert_eq!(view.session().expect("flying").system(), SystemId(130));

        // Past 1.5 s it arrives, and the new system fades in.
        view.tick(ms(450));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.jumping(), None);
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(130), SystemId(131)]
        );
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.course_map().route(), []);
        let arrived = player(&view);
        assert_eq!(arrived.position, Vec2::new(-1001.0, 0.0));
        assert_eq!(arrived.velocity, Vec2::ZERO, "at rest");
        assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
        assert_eq!(view.shown_position(), at(-1001.0, 0.0));
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("a fade");
        assert_eq!(color.a, 221);
        assert!(texts(&list).contains(&"Alpha Centauri (sÿst 131)".to_owned()));
        let mut hud = DrawList::new();
        hud::draw(
            &mut hud,
            view.status_bar().expect("a status bar"),
            &HudState {
                radar: &view.radar_shown(),
                blink_on: view.hud_cadence.blink_on(),
                reserves: reserves(&view),
                nav: NavDisplay::Off,
                cargo: Some(&view.session().expect("flying").status_cargo()),
                date: "June 24, 1177 NC",
                disabled: false,
            },
            None,
        );
        assert!(
            list.iter()
                .skip(hud_at(&list))
                .take(hud.len())
                .eq(hud.iter()),
            "the HUD, with a jump's fuel less and a day on"
        );
        assert_eq!(
            (at_fade, hud_at(&list) + hud.len() + 2),
            (list.len() - 1, at_fade),
            "then the target and weapon, and the fade over everything"
        );
        view.tick(ms(100));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(204), "shrinking");
        assert_eq!(player(&view), arrived, "at rest, no keys held");

        // Then plain flight.
        view.tick(ms(1200));
        assert_eq!(view.jump_effect(), None);
        let list = drawn(&view);
        assert_eq!(fade(&list), None);
        let mut stars = DrawList::new();
        starfield::draw(&mut stars, &view.camera());
        assert!(list.iter().take(stars.len()).eq(stars.iter()));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        assert_ne!(player(&view), arrived, "it flies again");
    }

    #[test]
    fn hyperspace_effects_default_on() {
        let mut view = flight();
        assert!(view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert_eq!(fade(&drawn(&view)).map(|f| f.1.a), Some(128), "fading out");
    }

    /// Whether `list` draws the fade quad translucent: part of a fade.
    fn translucent_fade(list: &DrawList) -> bool {
        fade(list).is_some_and(|(_, color)| color.a < u8::MAX)
    }

    #[test]
    fn with_effects_off_a_jump_streaks_then_arrives_without_fading() {
        let mut view = flight().with_hyperspace_effects(false);
        assert!(!view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let leaving = player(&view);

        // The stars streak, with no fade over them.
        view.tick(ms(500));
        assert_eq!(player(&view), leaving, "frozen");
        let list = drawn(&view);
        let mut streaks = DrawList::new();
        starfield::draw_streaked(&mut streaks, &view.camera(), at(1.0, 0.0), 256.0);
        assert!(list.iter().take(streaks.len()).eq(streaks.iter()));
        assert_eq!(fade(&list), None);
        view.tick(ms(499));
        assert_eq!(fade(&drawn(&view)), None);
        assert_eq!(view.session().expect("flying").system(), SystemId(130));

        // As the streak ends, the ship arrives and the screen is white once.
        view.tick(ms(1));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.jumping(), None);
        let list = drawn(&view);
        assert_eq!(
            fade(&list),
            Some((list.len() - 1, Color::WHITE)),
            "opaque white over everything"
        );
        assert!(view.jump_effect().is_some(), "flashing");
        assert_eq!(player(&view).position, Vec2::new(-1001.0, 0.0));

        // Then plain flight, with no fade in.
        view.tick(ms(33));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(fade(&drawn(&view)), None);
    }

    #[test]
    fn set_hyperspace_effects_changes_the_next_jump() {
        let mut view = flight().with_hyperspace_effects(false);
        view.set_hyperspace_effects(true);
        assert!(view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert!(translucent_fade(&drawn(&view)), "fading");

        let mut view = flight();
        view.set_hyperspace_effects(false);
        assert!(!view.hyperspace_effects());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let mut faded = false;
        for _ in 0..200 {
            view.tick(TICK);
            faded |= translucent_fade(&drawn(&view));
        }
        assert!(!faded, "never fades");
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    /// Ship 128 carrying the stock Multi-Jumping Organ (`oütf` 275,
    /// multi-jump 10), with Alpha Centauri linked on to Barnard.
    fn multi_jumping() -> FakeCatalog {
        FakeCatalog {
            outfits: vec![OutfitRecord {
                id: OutfitId(275),
                name: "Multi-Jumping Organ".to_owned(),
                short_name: "Multi-Jumping Organ".to_owned(),
                disp_weight: 0,
                mass: 0,
                tech_level: 1,
                max: 1,
                flags: 0,
                cost: 0,
                mods: [(32, 10), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "multi-jumping organ".to_owned(),
                lc_plural: "multi-jumping organs".to_owned(),
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            }],
            defaults: vec![(OutfitId(275), 1)],
            onward: true,
            ..catalog()
        }
    }

    #[test]
    fn a_multi_jump_plays_one_effect_and_loads_only_the_final_system() {
        let mut view = FlightView::new(multi_jumping()).with_overlays(TITLED);
        plot(&mut view, 132);
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131), SystemId(132)]
        );
        fly_out(&mut view);
        jump_now(&mut view);
        let fuel_leaving = reserves(&view).fuel.now;
        let effect = view.jump_effect().expect("jumping");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to the first hop");

        view.tick(ms(1550));
        let session = view.session().expect("flying");
        assert_eq!(session.system(), SystemId(132));
        assert_eq!(session.course(), []);
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(130), SystemId(132)],
            "Alpha Centauri is never loaded"
        );
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
        assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
        assert!(texts(&drawn(&view)).contains(&"Barnard (sÿst 132)".to_owned()));
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Barnard system on June 24, 1177 NC. No stellar objects present."
            ),
            "names the final system"
        );
        assert!(view.jump_effect().is_some(), "fading in");

        view.tick(ms(1450));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.session().expect("flying").jumping(), None);
        for _ in 0..30 {
            view.tick(TICK);
            assert_eq!(view.jump_effect(), None, "no second effect");
        }
        assert_eq!(view.session().expect("flying").system(), SystemId(132));
    }

    /// Presses J and ticks until the streak begins, as the session says
    /// the jump has begun.
    fn jump_now(view: &mut View) {
        view.input(&key(JUMP, true));
        for _ in 0..1000 {
            if view.jump_effect().is_some() {
                return;
            }
            view.tick(TICK);
        }
        panic!("the jump never began: {:?}", player(view));
    }

    /// Ticks until the ship has arrived in another system and the jump's
    /// effect is over.
    fn arrive_now(view: &mut View) {
        let leaving = view.session().expect("flying").system();
        for _ in 0..1000 {
            let arrived = view.session().expect("flying").system() != leaving;
            if arrived && view.jump_effect().is_none() {
                return;
            }
            view.tick(TICK);
        }
        panic!("never arrived: {:?}", player(view));
    }

    #[test]
    fn the_streak_waits_for_the_session_to_begin_the_jump() {
        // fly_out leaves the ship racing north, facing it; Alpha Centauri
        // is east.
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None, "not on J");
        assert_eq!(view.message(), None);
        let session = view.session().expect("flying");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.jumping(), None);
        let pressed = player(&view);
        view.tick(TICK);
        assert_ne!(player(&view), pressed, "the session keeps ticking");
        let mut waited = 1;
        while view.session().expect("flying").jumping().is_none() {
            assert_eq!(view.jump_effect(), None, "tick {waited}");
            view.tick(TICK);
            waited += 1;
            assert!(waited < 1000, "the jump never began");
        }
        let effect = view.jump_effect().expect("the streak, on that tick");
        assert_eq!(effect.direction(), at(1.0, 0.0), "east, to Alpha Centauri");
        assert_eq!(player(&view).heading, 90.0, "facing it");
        assert!(waited > 30, "braked and turned first: {waited}");
    }

    #[test]
    fn a_ship_ready_to_jump_streaks_on_j() {
        // Turned east at rest, then drifting out slowly that way: facing
        // Alpha Centauri and slow enough, it needs no pre-jump stage.
        let mut view = flight();
        plot(&mut view, 131);
        view.input(&key(Key::Right, true));
        ticks(&mut view, 30);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 10);
        view.input(&key(Key::Up, false));
        for _ in 0..2000 {
            if player(&view).position.length() >= MIN_JUMP_DISTANCE {
                break;
            }
            view.tick(TICK);
        }
        let ship = player(&view);
        assert_eq!(ship.heading, 90.0, "{ship:?}");
        assert!(ship.position.length() >= MIN_JUMP_DISTANCE, "{ship:?}");
        view.input(&key(JUMP, true));
        let effect = view.jump_effect().expect("streaking on J");
        assert_eq!(effect.direction(), at(1.0, 0.0));
        view.tick(TICK);
        assert_eq!(player(&view), ship, "frozen at once");
    }

    #[test]
    fn keys_are_ignored_while_the_ship_turns_to_jump() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        let pressed = player(&view);
        let mut alongside = flight();
        plot(&mut alongside, 131);
        fly_out(&mut alongside);
        alongside.input(&key(JUMP, true));
        for k in [Key::Up, Key::Left, Key::Down, MAP, LAND, JUMP, Key::Tab] {
            assert_eq!(view.input(&key(k, true)), ScreenAction::None);
        }
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        for _ in 0..10 {
            view.tick(TICK);
            alongside.tick(TICK);
        }
        assert_eq!(player(&view), player(&alongside), "flown alike");
        assert_ne!(player(&view), pressed, "the session flies on");
        assert_eq!(view.jump_effect(), None);
        let session = view.session().expect("flying");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)));
        assert_eq!(session.nav_target(), None);
        assert_eq!(view.take_landing(), None);
    }

    #[test]
    fn keys_are_ignored_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(Key::Up, true));
        jump_now(&mut view);
        view.tick(ms(300));
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Left, true));
        view.input(&key(MAP, true));
        view.input(&key(LAND, true));
        view.input(&key(JUMP, true));
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        view.tick(ms(2700));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.take_landing(), None);
        let arrived = player(&view);
        ticks(&mut view, 5);
        assert_eq!(
            player(&view),
            stepped(arrived, Controls::default(), 5),
            "neither thrusting nor turning"
        );
    }

    #[test]
    fn keys_are_ignored_while_the_old_system_fades_out() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1250));
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeOut(_))
        ));
        for k in [Key::Up, Key::Left, MAP, LAND, JUMP] {
            assert_eq!(view.input(&key(k, true)), ScreenAction::None);
        }
        assert!(!view.map_open());
        assert_eq!(view.message(), None);
        assert_eq!(view.take_landing(), None);
        arrive_now(&mut view);
        let arrived = player(&view);
        ticks(&mut view, 5);
        assert_eq!(
            player(&view),
            stepped(arrived, Controls::default(), 5),
            "neither thrusting nor turning"
        );
        assert_eq!(view.session().expect("flying").preparing_jump(), None);
    }

    /// Jumps from Sol to Alpha Centauri, on a course on to Barnard, and
    /// stops 50 ms into the arrival's fade-in.
    fn fading_in_on_course_to_barnard() -> View {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        plot(&mut view, 132);
        fly_out(&mut view);
        jump_now(&mut view);
        view.tick(ms(1550));
        assert_eq!(system_of(&view), SystemId(131));
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(_))
        ));
        view
    }

    /// The fade's alpha as drawn, if it is.
    fn fade_alpha(view: &View) -> Option<u8> {
        fade(&drawn(view)).map(|(_, color)| color.a)
    }

    #[test]
    fn j_during_the_arrival_fade_in_begins_the_next_jump() {
        let mut view = fading_in_on_course_to_barnard();
        let arrival = view.message().expect("the arrival message").to_owned();
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), Some(arrival.as_str()), "not refused");
        let session = view.session().expect("flying");
        assert!(
            session.preparing_jump() == Some(SystemId(132))
                || session.jumping() == Some(SystemId(132)),
            "{:?} {:?}",
            session.preparing_jump(),
            session.jumping()
        );
        let pressed = fade_alpha(&view).expect("still fading in");
        view.tick(TICK);
        let fading = fade_alpha(&view).expect("the old fade plays on");
        assert!(fading < pressed, "{fading} < {pressed}");
        for _ in 0..1000 {
            if view.session().expect("flying").jumping().is_some() {
                break;
            }
            view.tick(TICK);
        }
        assert_eq!(
            view.session().expect("flying").jumping(),
            Some(SystemId(132))
        );
        let effect = view.jump_effect().expect("the next jump's streak");
        assert!(matches!(effect.phase(), JumpPhase::Streak(_)));
        let toward = JumpEffect::toward(Vec2::new(600.0, 0.0), Vec2::new(0.0, 600.0));
        assert_eq!(
            effect.direction(),
            toward.direction(),
            "south-west, to Barnard"
        );
        arrive_now(&mut view);
        assert_eq!(system_of(&view), SystemId(132));
    }

    #[test]
    fn the_ship_flies_under_the_arrival_fade_in() {
        let mut view = fading_in_on_course_to_barnard();
        let arrived = player(&view);
        let before = fade_alpha(&view).expect("fading in");
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        let after = fade_alpha(&view).expect("still fading in");
        assert!(after < before, "{after} < {before}");
    }

    #[test]
    fn the_arrival_fade_plays_on_and_shows_over_the_map() {
        let mut view = fading_in_on_course_to_barnard();
        tap(&mut view, MAP);
        assert!(view.map_open(), "the map opens under the fade");
        let list = drawn(&view);
        let (at_fade, color) = fade(&list).expect("the fade, over the map");
        assert_eq!(at_fade, list.len() - 1, "drawn last");
        let before = color.a;
        view.tick(ms(100));
        let after = fade_alpha(&view).expect("fading on with the map open");
        assert!(after < before, "{after} < {before}");
        view.tick(crate::flight::jump::FADE_IN_FOR);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.jump_effect(), None, "the fade played out");
        assert_eq!(fade(&drawn(&view)), None);
    }

    #[test]
    fn arrival_message_reads_as_the_original() {
        assert_eq!(
            arrival_message("Sol", "June 23, 1177 NC", true),
            "Jumping into the Sol system on June 23, 1177 NC."
        );
    }

    #[test]
    fn arrival_message_in_a_system_without_stellars_says_so() {
        assert_eq!(
            arrival_message("Sol", "June 23, 1177 NC", false),
            "Jumping into the Sol system on June 23, 1177 NC. No stellar objects present."
        );
    }

    #[test]
    fn the_arrival_strings_are_the_originals() {
        assert_eq!(JUMPING_INTO, "Jumping into the");
        assert_eq!(SYSTEM_ON, "system on");
    }

    #[test]
    fn arriving_shows_the_jump_message_on_the_message_line() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let session = view.session().expect("flying");
        let leaving = session.system();
        for _ in 0..1000 {
            if view.session().expect("flying").system() != leaving {
                break;
            }
            view.tick(TICK);
        }
        let date = view.session().expect("flying").date_text();
        let name = view.scene().expect("arrived").name().to_owned();
        let text = arrival_message(&name, &date, true);
        assert!(text.contains("Alpha Centauri"), "{text}");
        assert_eq!(view.message(), Some(text.as_str()));
        assert_eq!(message(&view), Some(message_drawn(&text)));
        assert_lasts(&mut view, &text, 240);
        assert_eq!(message(&view), None);
    }

    #[test]
    fn a_refusal_shown_before_the_jump_is_replaced_by_the_arrival() {
        let mut view = flight();
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        view.input(&key(JUMP, false));
        assert_eq!(view.message(), Some(NO_DESTINATION));
        plot(&mut view, 131);
        assert_eq!(view.message(), Some(NO_DESTINATION), "still on screen");
        view.input(&key(JUMP, true));
        assert_eq!(
            view.message(),
            Some(NO_DESTINATION),
            "starting the jump clears only #42"
        );
        jump_now(&mut view);
        arrive_now(&mut view);
        let shown = view.message().expect("the arrival message");
        assert!(shown.starts_with(JUMPING_INTO), "{shown}");
    }

    #[test]
    fn map_and_jump_in_a_flight_that_never_started_do_nothing_much() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.course_map().current(), None);
        view.input(&key(JUMP, true));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(view.message(), None);
        tap(&mut view, MAP);
        assert!(view.map_open());
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        assert_eq!(view.course_map().route(), []);
    }

    #[test]
    fn the_stellars_go_on_animating_while_the_jump_plays() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let moon = |view: &View| sprites(&drawn(view))[1].0.frame;
        let before = moon(&view);
        view.tick(TICK);
        assert_eq!(moon(&view), (before + 1) % 4, "a frame a tick");
        view.tick(TICK * 2);
        assert_eq!(moon(&view), (before + 3) % 4);
        assert!(view.jump_effect().is_some());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_the_open_map() {
        let mut view = flight();
        tap(&mut view, MAP);
        let alpha = on_map(&view, 131);
        let left = |pressed| Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at: alpha,
        };
        view.input(&left(true));
        view.cancel_pointer();
        view.input(&left(false));
        assert_eq!(view.course_map().selected(), None, "no click");
        assert_eq!(view.session().expect("flying").course(), []);
        assert!(view.map_open());
    }

    // The pilot.

    /// A pilot named `name` docked at Earth: landed over a planet 128 at
    /// the centre, then flown in the stock catalog, where Earth is at
    /// (0, -600).
    fn docked_pilot(name: &str) -> Pilot {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let pilot = Pilot::new(&centred, name).expect("starts");
        let mut view = FlightView::with_pilot(centred, pilot);
        land_now(&mut view);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        view.pilot().expect("flying").clone()
    }

    // Pilot edits.

    /// A view of Ada docked at Earth, the landing taken, with the systems
    /// read so far forgotten.
    fn landed_view() -> View {
        let mut view = FlightView::with_pilot(catalog(), docked_pilot("Ada"));
        view.take_landing();
        view.catalog().systems_read.borrow_mut().clear();
        view
    }

    fn move_to(view: &mut View, system: i16, stellar: i16) {
        let mut desk = view.pilot_desk().expect("flying");
        desk.edit(PilotEdit::MoveTo {
            system: SystemId(system),
            stellar: StellarId(stellar),
        })
        .expect("moves");
    }

    #[test]
    fn a_session_that_failed_has_no_pilot_desk() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert!(view.pilot_desk().is_none());
        view.resync();
        assert!(view.scene().is_none());
    }

    #[test]
    fn the_pilot_desk_reads_the_session_and_names_from_the_map() {
        let mut view = landed_view();
        let desk = view.pilot_desk().expect("flying");
        let sheet = desk.sheet().expect("a pilot");
        assert_eq!(sheet.name, "Ada");
        assert_eq!(sheet.system.name, "Sol");
        assert_eq!(desk.systems().len(), 3);
    }

    #[test]
    fn after_a_move_resync_lays_out_the_new_system() {
        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.resync();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.course_map().route(), []);
        assert!(
            view.course_map()
                .explored()
                .is_some_and(|explored| explored.contains(&SystemId(131)))
        );
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert_eq!(view.message(), None, "no arrival message");
        view.resync();
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(131)],
            "nothing moved, nothing read"
        );
    }

    #[test]
    fn after_a_move_within_the_system_resync_drops_the_course_and_reads_nothing() {
        let mut view = landed_view();
        plot(&mut view, 131);
        assert_eq!(view.course_map().route(), [SystemId(131)]);
        view.tick(TICK / 2);
        assert!(view.alpha() > 0.0, "between steps");
        move_to(&mut view, 130, 129);
        view.resync();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(view.course_map().route(), []);
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.alpha(), 0.0);
        assert_eq!(view.shown_position(), Point::new(300.0, -200.0));
    }

    #[test]
    fn a_resync_with_nothing_moved_reads_nothing() {
        let mut view = landed_view();
        view.resync();
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    }

    #[test]
    fn a_pilot_docked_at_a_stellar_resumes_landed_there_silently() {
        let pilot = docked_pilot("Ada");
        assert_eq!(pilot.stellar(), Some(StellarId(128)));
        let mut view = FlightView::with_pilot(catalog(), pilot.clone());
        assert_eq!(view.pilot(), Some(&pilot));
        assert_eq!(
            view.take_landing(),
            Some(StellarId(128)),
            "the router lands"
        );
        assert_eq!(view.take_landing(), None, "once");
        assert_eq!(player(&view).position, Vec2::new(0.0, -600.0));
        assert_eq!(view.take_sounds(), [], "no landing sound");
        assert!(!view.take_save_due());
        assert_eq!(view.take_off(), Some(StellarId(128)));
    }

    #[test]
    fn a_pilot_in_flight_resumes_at_the_centre() {
        let pilot = Pilot::new(&catalog(), "Bob").expect("starts");
        let mut view = FlightView::with_pilot(catalog(), pilot);
        assert_eq!(view.take_landing(), None);
        assert_eq!(player(&view), start());
        assert_eq!(view.pilot().map(Pilot::name), Some("Bob"));
    }

    #[test]
    fn a_pilot_whose_system_is_gone_cannot_fly() {
        let mut pilot = docked_pilot("Ada");
        pilot.explore(SystemId(132));
        let text = nova_sim::save::encode(&pilot).replace("\"system\": 130", "\"system\": 132");
        let moved = nova_sim::save::decode(&text).expect("a pilot");
        let view = FlightView::with_pilot(catalog(), moved);
        assert_eq!(
            view.session().err(),
            Some("the pilot's system, sÿst 132, does not exist")
        );
        assert_eq!(view.pilot(), None);
    }

    #[test]
    fn landing_taking_off_and_transactions_make_a_save_due() {
        let centred = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..catalog()
        };
        let mut view = FlightView::new(centred);
        assert!(!view.take_save_due());
        assert!(!view.transact(|pilot| pilot.set_cash(5)), "in flight");
        land_now(&mut view);
        assert!(view.take_save_due(), "landed");
        assert!(view.transact(|pilot| pilot.set_cash(5)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(5));
        assert!(view.take_save_due(), "a transaction");
        assert!(!view.take_save_due(), "taken");
        view.take_off();
        assert!(view.take_save_due(), "took off");
    }

    #[test]
    fn a_view_that_cannot_fly_has_no_pilot_and_saves_nothing() {
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        let mut view = FlightView::new(broken);
        assert_eq!(view.pilot(), None);
        assert!(!view.transact(|pilot| pilot.set_cash(5)));
        assert!(!view.take_save_due());
    }

    #[test]
    fn the_course_map_shows_the_explored_systems_and_arriving_explores() {
        let mut view = flight();
        let explored = |view: &View| {
            view.course_map()
                .explored()
                .map(|set| set.iter().copied().collect::<Vec<_>>())
        };
        assert_eq!(explored(&view), Some(vec![SystemId(130)]));
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(explored(&view), Some(vec![SystemId(130), SystemId(131)]));
    }

    // The exchange and the day's chances.

    use nova_sim::{Chance, Direction, DisasterId, Good, Lot, Order, TradeRefusal};

    /// Earth at the centre, a trade center trading food at 75, the first
    /// `chär` holding 1000 credits, and its ship 10 tons.
    fn trading() -> FakeCatalog {
        let flags = StellarFlags::CAN_LAND | StellarFlags::TRADE_CENTER | 2 << 28;
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            fields: ShipFields {
                holds: 10,
                ..FIELDS
            },
            sites: vec![site(128, (0.0, 0.0), flags)],
            commodities: CommodityStrings {
                names: vec!["Food".to_owned()],
                name_patches: Default::default(),
                base_prices: vec!["75".to_owned()],
                price_patches: Default::default(),
            },
            ..catalog()
        }
    }

    const BUY_FOOD: Order = Order {
        row: 0,
        good: Good::Commodity(0),
        direction: Direction::Buy,
        lot: Lot::Click,
    };

    #[test]
    fn the_exchange_is_the_sessions_and_a_trade_goes_through_it() {
        let mut view = FlightView::new(trading());
        assert_eq!(view.market(), None, "in flight");
        assert_eq!(view.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
        land_now(&mut view);
        view.take_save_due();
        let market = view.market().expect("landed at a trade center");
        assert_eq!(
            market.row(Good::Commodity(0)).map(|row| row.price),
            Some(75)
        );
        assert_eq!(view.trade(BUY_FOOD), Ok(10), "a click buys up to 10");
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 10 * 75));
        assert!(view.take_save_due(), "a trade");
        assert_eq!(
            view.market()
                .and_then(|m| m.row(Good::Commodity(0)).map(|row| row.held)),
            Some(10)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..trading()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.market(), None);
        assert_eq!(broken.trade(BUY_FOOD), Err(TradeRefusal::NoMarket));
    }

    use nova_sim::order_loop::{OrderRun, Stopped};
    use nova_sim::{OutfitOrder, OutfitRefusal, OutfitRun, Reselection};

    /// Earth at the centre, an outfitter selling a fuel tank (+100 fuel,
    /// a ton, 1000 credits), the first `chär` holding 1000 credits.
    fn outfitting() -> FakeCatalog {
        FakeCatalog {
            character: Ok(CharacterStart {
                cash: 1000,
                ..catalog().character.expect("a chär")
            }),
            sites: vec![site(
                128,
                (0.0, 0.0),
                StellarFlags::CAN_LAND | StellarFlags::OUTFITTER,
            )],
            outfits: vec![OutfitRecord {
                id: OutfitId(200),
                name: "Fuel Tank".to_owned(),
                short_name: "Fuel Tank".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 5,
                flags: 0,
                cost: 1000,
                mods: [(12, 100), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "fuel tank".to_owned(),
                lc_plural: "fuel tanks".to_owned(),
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            }],
            ..catalog()
        }
    }

    const BUY_TANK: OutfitOrder = OutfitOrder {
        outfit: OutfitId(200),
        direction: Direction::Buy,
    };

    #[test]
    fn the_outfitter_is_the_sessions_and_an_order_goes_through_it() {
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.outfitter(), None, "in flight");
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
        land_now(&mut view);
        view.take_save_due();
        let outfitter = view.outfitter().expect("landed at an outfitter");
        assert_eq!(
            outfitter.row(OutfitId(200)).map(|row| row.price),
            Some(1000)
        );
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.pilot().map(Pilot::cash), Some(0));
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(
            view.outfitter()
                .and_then(|o| o.row(OutfitId(200)).map(|row| row.owned)),
            Some(1)
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "the tank");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.outfitter(), None);
        assert_eq!(broken.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
    }

    #[test]
    fn a_counted_order_goes_through_the_session() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 3000,
            ..catalog.character.expect("a chär")
        });
        let mut view = FlightView::new(catalog);
        assert_eq!(
            view.outfit_counted(BUY_TANK, 2),
            Err(OutfitRefusal::NoOutfitter),
            "in flight"
        );
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.outfit_counted(BUY_TANK, 2), Ok(2));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000));
        assert!(view.take_save_due(), "a purchase");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(
            broken.outfit_counted(BUY_TANK, 2),
            Err(OutfitRefusal::NoOutfitter)
        );
    }

    #[test]
    fn an_outfit_order_goes_through_the_session_with_its_run() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 3000,
            ..catalog.character.expect("a chär")
        });
        let refused = |refusal| OutfitRun {
            run: OrderRun {
                done: 0,
                stopped: Some(Stopped { pass: 0, refusal }),
            },
            alert: None,
            after: Reselection::Kept,
        };
        let mut view = FlightView::new(catalog);
        assert_eq!(
            view.outfit_order(BUY_TANK, 2),
            refused(OutfitRefusal::NoOutfitter),
            "in flight"
        );
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(
            view.outfit_order(BUY_TANK, 4),
            OutfitRun {
                run: OrderRun {
                    done: 3,
                    stopped: Some(Stopped {
                        pass: 3,
                        refusal: OutfitRefusal::CannotAfford,
                    }),
                },
                alert: None,
                after: Reselection::Kept,
            }
        );
        assert_eq!(view.pilot().map(Pilot::cash), Some(0));
        assert!(view.take_save_due(), "a purchase");
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(
            broken.outfit_order(BUY_TANK, 2),
            refused(OutfitRefusal::NoOutfitter)
        );
    }

    /// [`outfitting`] with 10,000 credits, a -10-ton pod (201) and an
    /// outfit (202) of `heavy` tons, each 100 credits.
    fn pod_outfitting(heavy: i16) -> FakeCatalog {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 10_000,
            ..catalog.character.expect("a chär")
        });
        let tank = catalog.outfits[0].clone();
        for (id, mass) in [(201, -10), (202, heavy)] {
            catalog.outfits.push(OutfitRecord {
                id: OutfitId(id),
                mass,
                cost: 100,
                lc_name: "pod".to_owned(),
                lc_plural: "pods".to_owned(),
                ..tank.clone()
            });
        }
        catalog
    }

    #[test]
    fn a_counted_sale_refused_part_way_goes_through_the_session_with_its_alert() {
        let pod = OutfitId(201);
        let mut probe = FlightView::new(pod_outfitting(0));
        land_now(&mut probe);
        let free = probe.outfitter().expect("open").free_mass;
        // Three pods and the heavy outfit leave ten tons free: one pod
        // sold leaves none, and the next is refused at pass 1.
        let heavy = i16::try_from(free).expect("small") + 20;
        let mut view = FlightView::new(pod_outfitting(heavy));
        land_now(&mut view);
        let buy = |outfit| OutfitOrder {
            outfit,
            direction: Direction::Buy,
        };
        assert_eq!(view.outfit_order(buy(pod), 3).run.done, 3);
        assert_eq!(view.outfit_order(buy(OutfitId(202)), 1).run.done, 1);
        assert_eq!(view.outfitter().expect("open").free_mass, 10);
        view.take_save_due();
        let sell = OutfitOrder {
            outfit: pod,
            direction: Direction::Sell,
        };
        let order = view.outfit_order(sell, 3);
        assert_eq!(
            order.run,
            OrderRun {
                done: 1,
                stopped: Some(Stopped {
                    pass: 1,
                    refusal: OutfitRefusal::NegativeFreeMass,
                }),
            }
        );
        let alert = order.alert.expect("an alert");
        assert_eq!(alert.refusal, OutfitRefusal::NegativeFreeMass);
        assert!(alert.plural, "a pass remains");
        assert_eq!(view.pilot().map(|pilot| pilot.owned(pod)), Some(2));
        assert!(view.take_save_due(), "a sale");
    }

    #[test]
    fn opening_the_outfitter_lifts_the_sessions_once_an_opening_limit() {
        let mut catalog = outfitting();
        catalog.character = Ok(CharacterStart {
            cash: 5000,
            ..catalog.character.expect("a chär")
        });
        catalog.outfits[0].mods[0] = (nova_sim::outfitter::EXPLORES_MAP, 5);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::BoughtThisOpening));
        view.open_outfitter();
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        FlightView::new(broken).open_outfitter();
    }

    #[test]
    fn recharging_goes_through_the_session() {
        use nova_sim::RechargeRefusal;
        let mut view = FlightView::new(outfitting());
        assert_eq!(view.recharge(), Err(RechargeRefusal::NoFuel), "in flight");
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.recharge(), Err(RechargeRefusal::Full));
        assert!(!view.take_save_due());
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..outfitting()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.recharge(), Err(RechargeRefusal::NoFuel));
    }

    /// Fires every time, and records each percent it is asked; each draw
    /// is 0, and records its `n`.
    #[derive(Default)]
    struct Always {
        asked: Vec<u8>,
        drawn: Vec<u32>,
    }

    impl Chance for Always {
        fn fires(&mut self, percent: u8) -> bool {
            self.asked.push(percent);
            true
        }

        fn below(&mut self, n: u32) -> u32 {
            self.drawn.push(n);
            0
        }

        /// The last of the outcomes.
        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    #[test]
    fn a_shared_chance_draws_from_its_source() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut chance = SharedChance::new(shared);
        assert_eq!(chance.below(7), 0);
        assert_eq!(chance.below(500), 0);
        assert_eq!(always.borrow().drawn, [7, 500]);
        assert_eq!(SharedChance::default().below(256), 255, "never fires");
    }

    /// A food surplus at Proxima, 35 % a day.
    fn eventful() -> FakeCatalog {
        FakeCatalog {
            disasters: vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 140,
                commodity: 0,
                price_delta: -15,
                duration: 30,
                freq: 35,
                activate_on: nova_sim::Test::default(),
            }],
            ..catalog()
        }
    }

    /// Jumps to Alpha Centauri and arrives.
    fn jump_to_alpha(view: &mut View) {
        plot(view, 131);
        fly_out(view);
        view.input(&key(JUMP, true));
        arrive_now(view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
    }

    #[test]
    fn a_jump_rolls_the_days_events_on_the_chance_given() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(eventful()).with_chance(SharedChance::new(shared));
        jump_to_alpha(&mut view);
        assert_eq!(always.borrow().asked, [35]);
        let events: Vec<_> = view.pilot().expect("a pilot").events().collect();
        assert_eq!(events, [(DisasterId(128), 30)]);
        assert_eq!(format!("{:?}", SharedChance::default()), "Chance");
    }

    #[test]
    fn leaving_rolls_the_stays_days_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::with_pilot(eventful(), docked_pilot("Ada"))
            .with_chance(SharedChance::new(shared));
        view.take_landing();
        let landed = view.session().expect("docked").date();
        let shown = view.date_text();
        view.take_off().expect("took off");
        assert_eq!(always.borrow().asked, [35], "the take-off's day");
        assert_eq!(view.session().expect("flying").date(), landed.next_day());
        assert_ne!(view.date_text(), shown, "the HUD's date moves on");
    }

    #[test]
    fn without_a_chance_given_nothing_random_happens() {
        let mut view = FlightView::new(eventful());
        jump_to_alpha(&mut view);
        assert_eq!(view.pilot().expect("a pilot").events().count(), 0);
        let mut shared = SharedChance::default();
        assert!(!shared.fires(100));
        assert_eq!(shared.roll(8), 0);
    }

    #[test]
    fn a_shared_chance_rolls_on_its_source() {
        let shared: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        assert_eq!(SharedChance::new(shared).roll(360), 359);
    }

    use nova_sim::ShipRefusal;

    /// [`outfitting`], where Earth is a shipyard too, selling ship 129 for
    /// 900 credits: twice as fast, with a fuel tank.
    fn shipbuying() -> FakeCatalog {
        let mut earth = site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::OUTFITTER | StellarFlags::SHIPYARD,
        );
        earth.tech_level = 1;
        FakeCatalog {
            sites: vec![earth],
            ships: vec![ShipRecord {
                id: ShipId(129),
                name: "Fast".to_owned(),
                short_name: "Fast".to_owned(),
                long_name: String::new(),
                fields: ShipFields {
                    speed: 600,
                    ..FIELDS
                },
                defaults: vec![(OutfitId(200), 1)],
                cost: 900,
                tech_level: 1,
                buy_random: 100,
                hire_random: 0,
                require: 0,
                availability: nova_sim::Test::default(),
                appear_on: nova_sim::Test::default(),
                flags3: 0,
                disp_weight: 0,
                max_gun: 0,
                max_tur: 0,
                length: 0,
                crew: 0,
                inherent_ai: 1,
                comm_name: String::new(),
                inherent_govt: None,
                combat_govt: None,
                escort_type: -1,
                on_capture: nova_sim::Script::default(),
                on_purchase: nova_sim::Script::default(),
                on_retire: nova_sim::Script::default(),
            }],
            ..outfitting()
        }
    }

    #[test]
    fn the_shipyard_is_the_sessions_and_a_purchase_reloads_the_ships_sheet() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(view.shipyard(), None, "in flight");
        assert_eq!(
            view.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NoShipyard)
        );
        land_now(&mut view);
        view.take_save_due();
        let shipyard = view.shipyard().expect("landed at a shipyard");
        assert_eq!(shipyard.row(ShipId(129)).map(|row| row.price), Some(900));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        assert_eq!(
            view.buy_ship(ShipId(999), "Kestrel"),
            Err(ShipRefusal::NotListed),
            "refused"
        );
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        let bought = view.buy_ship(ShipId(129), "The Kestrel").expect("bought");
        assert_eq!(bought.price, 900);
        assert_eq!(
            view.pilot().and_then(Pilot::ship_name),
            Some("Kestrel"),
            "named as confirmed"
        );
        assert!(view.take_save_due(), "a purchase");
        assert_eq!(view.pilot().map(Pilot::ship), Some(ShipId(129)));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 900));
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        assert_eq!(view.reserves().fuel.max, 350.0, "its tank");
        // Off again, the new hull is drawn, at the new speed.
        view.settle_script();
        view.take_off().expect("took off");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "read once"
        );
        let mut list = DrawList::new();
        view.draw(&mut list);
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Sprite { image, .. } if image.id == 2001
            )),
            "the new sheet"
        );
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "and its lights");
        assert_eq!(
            view.session().map(|session| session.handling().max_speed),
            Ok(6.0)
        );
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..shipbuying()
        };
        let mut broken = FlightView::new(broken);
        assert_eq!(broken.shipyard(), None);
        assert_eq!(
            broken.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NoShipyard)
        );
    }

    /// [`shipbuying`], with the fuel tank and ship 129 each for sale on a
    /// `BuyRandom` of 50.
    fn rolling() -> FakeCatalog {
        let mut catalog = shipbuying();
        catalog.outfits[0].buy_random = 50;
        catalog.ships[0].buy_random = 50;
        catalog
    }

    #[test]
    fn the_outfitter_and_shipyard_roll_on_the_flights_chance() {
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::default());
        land_now(&mut view);
        let outfitter = view.outfitter().expect("an outfitter");
        assert!(outfitter.row(OutfitId(200)).is_none(), "off today");
        let shipyard = view.shipyard().expect("a shipyard");
        assert!(shipyard.row(ShipId(129)).is_none(), "off today");
        assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NotListed));
        assert_eq!(
            view.buy_ship(ShipId(129), "Kestrel"),
            Err(ShipRefusal::NotListed)
        );
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
        land_now(&mut view);
        always.borrow_mut().asked.clear();
        let outfitter = view.outfitter().expect("an outfitter");
        assert!(outfitter.row(OutfitId(200)).is_some(), "on today");
        let shipyard = view.shipyard().expect("a shipyard");
        assert!(shipyard.row(ShipId(129)).is_some(), "on today");
        assert_eq!(always.borrow().asked, [50, 50]);
        assert!(view.buy_ship(ShipId(129), "Kestrel").is_ok());
    }

    #[test]
    fn the_naming_is_the_sessions_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(shipbuying()).with_chance(SharedChance::new(shared));
        assert_eq!(view.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
        land_now(&mut view);
        view.take_save_due();
        assert_eq!(view.ship_naming(ShipId(999)), Err(ShipRefusal::NotListed));
        let naming = view.ship_naming(ShipId(129)).expect("a naming");
        assert_eq!(naming.ship, ShipId(129));
        assert_eq!(naming.prompt, "Please name your new : ");
        assert_eq!(naming.default, "Fast 999", "each digit the last of nine");
        assert!(!view.take_save_due());
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..shipbuying()
        });
        assert_eq!(
            broken.ship_naming(ShipId(129)),
            Err(ShipRefusal::NoShipyard)
        );
        broken.decline_ship(ShipId(129));
    }

    #[test]
    fn declining_a_ship_draws_its_roll_again_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(rolling()).with_chance(SharedChance::new(shared));
        land_now(&mut view);
        view.take_save_due();
        view.shipyard().expect("a shipyard");
        always.borrow_mut().asked.clear();
        view.shipyard().expect("a shipyard");
        assert!(always.borrow().asked.is_empty(), "kept");
        view.decline_ship(ShipId(129));
        assert!(!view.take_save_due());
        view.shipyard().expect("a shipyard");
        assert_eq!(always.borrow().asked, [50], "drawn again");
    }

    #[test]
    fn every_rule_reaches_the_session_through_the_flight() {
        let every = RuleKey::ALL
            .into_iter()
            .fold(Rulebook::default(), |book, key| {
                book.with_override(key, RuleSource::Bible)
            });
        assert_eq!(
            flight().session().map(Session::rules),
            Ok(&Rulebook::default())
        );
        let view = flight().with_rules(every);
        let rules = view.session().expect("flying").rules();
        assert_eq!(rules, &every);
        for key in RuleKey::ALL {
            assert_eq!(rules.source_for(key), RuleSource::Bible, "{key:?}");
        }
    }

    // The set-expression hooks.

    /// Rolls the second outcome of every roll, so `R(a b)` takes `a`.
    #[derive(Debug)]
    struct RollsOne;

    impl Chance for RollsOne {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n.min(1)
        }
    }

    fn rolling_one() -> SharedChance {
        SharedChance::new(Rc::new(RefCell::new(RollsOne)))
    }

    fn bit_set(view: &View, n: u16) -> bool {
        let bit = nova_sim::Bit::new(n).expect("a bit");
        view.pilot().is_some_and(|pilot| pilot.control_bit(bit))
    }

    #[test]
    fn an_outfits_on_purchase_runs_on_the_flights_chance() {
        let mut catalog = outfitting();
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("R(b1 b2)");
        let mut view = FlightView::new(catalog).with_chance(rolling_one());
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert!(bit_set(&view, 1));
        assert!(!bit_set(&view, 2));
    }

    #[test]
    fn a_ships_on_purchase_runs_on_the_flights_chance() {
        let mut catalog = shipbuying();
        catalog.ships[0].on_purchase = nova_sim::Script::parse("R(b3 b4)");
        let mut view = FlightView::new(catalog).with_chance(rolling_one());
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert!(bit_set(&view, 3));
        assert!(!bit_set(&view, 4));
    }

    #[test]
    fn a_ships_on_purchase_h_reads_only_the_final_class_sheet_once() {
        let mut catalog = shipbuying();
        catalog.ships[0].on_purchase = nova_sim::Script::parse("H130");
        let other = ShipRecord {
            id: ShipId(130),
            ..catalog.ships[0].clone()
        };
        catalog.ships.push(other);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(130)));
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(130)]
        );
        view.settle_script();
        view.take_off().expect("took off");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(130)],
            "a failed read is not tried again"
        );
        assert_eq!(view.frame(), None, "no sheet for 130");
    }

    /// The first `chär`'s `OnStart` is `on_start`.
    fn starting_with(on_start: &str) -> FakeCatalog {
        let base = catalog();
        FakeCatalog {
            character: Ok(CharacterStart {
                on_start: nova_sim::Script::parse(on_start),
                ..base.character.clone().expect("a chär")
            }),
            ..base
        }
    }

    #[test]
    fn beginning_runs_on_start_once_and_the_map_shows_what_it_explored() {
        let catalog = starting_with("R(b5 b6) ^b7 X131");
        let pilot = Pilot::new(&catalog, "Ada").expect("starts");
        let mut view = FlightView::with_pilot(catalog, pilot).with_chance(rolling_one());
        assert!(!bit_set(&view, 7), "flying runs nothing");
        view.begin();
        assert!(bit_set(&view, 5), "on the flight's chance");
        assert!(bit_set(&view, 7), "once");
        let explored = view
            .course_map()
            .explored()
            .map(|set| set.iter().copied().collect::<Vec<_>>());
        assert_eq!(explored, Some(vec![SystemId(130), SystemId(131)]));
        let broken = FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..starting_with("b5")
        };
        let mut broken = FlightView::new(broken);
        broken.begin();
        assert_eq!(broken.pilot(), None);
    }
    // Traffic.

    use std::collections::VecDeque;

    use nova_sim::{Goal, NpcId};

    /// Draws its script, then the last outcome, so no roll fires;
    /// records each `n` asked.
    #[derive(Default)]
    struct Script {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Chance for Script {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().unwrap_or(n - 1)
        }
    }

    fn scripted(draws: &[u32]) -> (Rc<RefCell<Script>>, SharedChance) {
        let script = Rc::new(RefCell::new(Script {
            draws: draws.iter().copied().collect(),
            asked: Vec::new(),
        }));
        let shared: Rc<RefCell<dyn Chance>> = script.clone();
        (script, SharedChance::new(shared))
    }

    /// One setup pass placing the düde's ship at (`x` - 750, `y` - 750)
    /// facing `heading`, of aggression 0.
    fn placed(x: u32, y: u32, heading: u32) -> [u32; 8] {
        [6, 6, 0, 0, x, y, heading, 2]
    }

    /// Decides nothing: every NPC idles.
    #[derive(Debug)]
    struct Still;

    impl Behaviour for Still {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }
    }

    /// [`catalog`] with `avg` ships on average in each of `systems`, all of
    /// düde 128's ship `ship` with AI `ai_type`; ships 129 and 130 are
    /// records the traffic can fly.
    fn trafficked(systems: &[i16], avg: i16, ship: i16, ai_type: i16) -> FakeCatalog {
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        let record = |id| ShipRecord {
            id: ShipId(id),
            name: format!("Ship {id}"),
            short_name: String::new(),
            long_name: String::new(),
            fields: ShipFields {
                speed: 600,
                ..FIELDS
            },
            defaults: Vec::new(),
            cost: 1,
            tech_level: 1,
            buy_random: 100,
            hire_random: 0,
            require: 0,
            availability: nova_sim::Test::default(),
            appear_on: nova_sim::Test::default(),
            flags3: 0,
            disp_weight: 0,
            max_gun: 0,
            max_tur: 0,
            length: 0,
            crew: 0,
            inherent_ai: 1,
            comm_name: String::new(),
            inherent_govt: None,
            combat_govt: None,
            escort_type: -1,
            on_capture: nova_sim::Script::default(),
            on_purchase: nova_sim::Script::default(),
            on_retire: nova_sim::Script::default(),
        };
        FakeCatalog {
            traffic: systems
                .iter()
                .map(|&id| {
                    (
                        SystemId(id),
                        nova_sim::SystemTraffic {
                            dude_types,
                            avg_ships: avg,
                            persons: Default::default(),
                        },
                    )
                })
                .collect(),
            dudes: vec![(
                nova_sim::DudeId(128),
                nova_sim::DudeRecord {
                    ai_type,
                    govt: None,
                    ships: vec![(ShipId(ship), 1)],
                    booty: 0,
                    info_types: 0,
                },
            )],
            ships: vec![record(129), record(130)],
            ..catalog()
        }
    }

    fn npc_ids(view: &View) -> Vec<NpcId> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.id).collect()
    }

    fn npc_states(view: &View) -> Vec<ShipState> {
        let session = view.session().expect("flying");
        session.npcs().iter().map(|npc| npc.state).collect()
    }

    #[test]
    fn the_traffic_is_populated_when_the_flight_starts() {
        let (script, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        assert_eq!(npc_ids(&view), [], "not until the first tick");
        view.tick(TICK);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_eq!((npc.ship, npc.goal), (ShipId(129), Goal::Idle));
        assert_eq!(npc.state.position, Vec2::new(100.0, -100.0));
        assert_eq!(&script.borrow().asked[..7], [7, 7, 100, 1, 1500, 1500, 360]);
        ticks(&mut view, 3);
        assert_eq!(npc_ids(&view), [NpcId(0)], "populated once");
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(npc.goal, Goal::Idle, "deciding as the behaviour given says");
        assert_eq!(
            npc.state.position,
            Vec2::new(118.0, -100.0),
            "gliding right at its top speed, 6 a tick"
        );
    }

    #[test]
    fn an_npc_is_drawn_with_its_own_sprite_where_it_is_facing_its_heading() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let frame = rotation_frame(90.0, NonZeroU16::new(72).expect("non-zero"));
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
                (ImageKey::sprite(1129, 1), at(812.0, 184.0)),
                (ImageKey::sprite(2001, frame), at(612.0, 284.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ],
            "after the stellars, before the player"
        );
    }

    #[test]
    fn a_ship_waiting_at_its_gate_is_drawn_by_its_tint_and_any_ship_by_its_cloak() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let mut npc = session.npcs()[0].clone();
        assert!(sprite_drawn(session, &npc), "flying");
        npc.mode = nova_sim::traffic::npc::Mode::JumpingIn { ticks_left: 3 };
        assert!(sprite_drawn(session, &npc), "jumping in");
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
        assert!(sprite_drawn(session, &npc), "fading in at its gate");
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 30 };
        assert!(sprite_drawn(session, &npc));
        assert_eq!(npc_tint(&npc), None, "hidden at its gate");
        npc.cloak = CLOAKED;
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
        assert!(!sprite_drawn(session, &npc), "cloaked at its gate");
    }

    /// A full cloak.
    const CLOAKED: nova_sim::cloak::Cloak = nova_sim::cloak::Cloak {
        fade: nova_sim::cloak::FADE_FULL,
        direction: 0,
    };

    #[test]
    fn a_ship_the_player_cannot_see_for_its_cloak_is_not_drawn() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let mut npc = session.npcs()[0].clone();
        npc.cloak = CLOAKED;
        assert!(!sprite_drawn(session, &npc), "cloaked");
        npc.cloak.direction = -1;
        npc.cloak.fade = 8.0;
        assert!(sprite_drawn(session, &npc), "faded out enough to be seen");
    }

    #[test]
    fn a_cloaked_ship_is_on_the_radar_only_when_the_session_says() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        let npc = session.npcs()[0].clone();
        let blip = RadarShip {
            position: at(100.0, -100.0),
            targeted: false,
            iff: ShipIff::Other,
            heavy: false,
        };
        let at_blip = at(100.0, -100.0);
        assert_eq!(radar_ships(session, &[(&npc, at_blip, 0.0)]), [blip]);
        let mut cloaked = npc.clone();
        cloaked.cloak = CLOAKED;
        assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), []);
        cloaked.cloak_gear.on_radar = true;
        assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), [blip]);
    }

    #[test]
    fn the_play_area_is_the_screen_left_of_the_status_bar_about_the_ship() {
        let area = play_area();
        assert_eq!(
            area,
            nova_sim::cloak::Area {
                left: -512.0,
                top: -384.0,
                right: 1024.0 - 194.0 - 512.0,
                bottom: 384.0,
            }
        );
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert_eq!(view.session().expect("flying").play_area(), area, "told");
    }

    #[test]
    fn an_npc_is_a_dim_blip_on_the_radar() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        // (100, -100) is 3.125 pixels each way: (929.125, 92.875).
        let blip = RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Dim,
        };
        assert_eq!(blips(&view), [blip]);
        let list = drawn(&view);
        assert!(
            list.iter().any(|command| *command
                == DrawCommand::Dot {
                    center: at(929.5, 92.5),
                    size: 1.0,
                    color: layout().dim_radar,
                }),
            "{list:?}"
        );
    }

    /// The tints the NPC sprite (ship 129's sheet, 2001) is drawn with.
    fn npc_tints(list: &DrawList) -> Vec<Color> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, tint, .. } if image.id == 2001 => Some(*tint),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_npc_coming_out_of_a_gate_is_hidden_then_fades_in_but_always_on_the_radar() {
        let mut catalog = trafficked(&[130], 1, 129, 3);
        catalog.sites.push(LandingSite {
            flags2: nova_sim::gate::HYPERGATE,
            exit_angle: 90,
            ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
        });
        // Setup brings nobody (an empty person roll); then an arrival
        // comes out of the gate, the third of the stellars.
        let (script, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        ticks(&mut view, 2);
        assert_eq!(&script.borrow().asked[1..9], [500, 7, 7, 100, 1, 360, 3, 3]);
        let mode = |view: &View| view.session().expect("flying").npcs()[0].mode;
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 59 });
        ticks(&mut view, 19);
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 40 });
        let list = drawn(&view);
        assert_eq!(npc_tints(&list), [], "hidden");
        assert_eq!(blips(&view).len(), 1, "on the radar all the same");
        ticks(&mut view, 32);
        assert_eq!(mode(&view), Mode::Emerging { ticks_left: 8 });
        let list = drawn(&view);
        assert_eq!(
            npc_tints(&list),
            [Color::rgba(255, 255, 255, 128)],
            "half in"
        );
        assert_eq!(blips(&view).len(), 1);
        ticks(&mut view, 8);
        assert_eq!(mode(&view), Mode::LeavingGate { tick: 0 });
        assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE]);
    }

    #[test]
    fn an_npc_is_drawn_between_its_last_two_steps() {
        // At (0, 100) facing up, it heads for Earth, straight up.
        let mut draws = placed(750, 850, 0).to_vec();
        draws.push(0);
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked(&[130], 1, 129, 1)).with_chance(chance);
        // The first step sets the system up; the NPC moves from the next.
        view.tick(TICK);
        assert_eq!(npc_states(&view)[0].position, Vec2::new(0.0, 100.0));
        view.tick(TICK + TICK / 2);
        let npc = &view.session().expect("flying").npcs()[0];
        assert_eq!(
            npc.goal,
            Goal::Land(StellarId(128)),
            "Nova's AI, by default: a trader's business"
        );
        let moved = npc.state.position.y - 100.0;
        assert!(moved < 0.0, "{npc:?}");
        let alpha = view.alpha();
        assert!(alpha > 0.4, "{alpha}");
        let shown = sprites(&drawn(&view))[2].1;
        let expected = view
            .camera()
            .world_to_screen(at(0.0, moved.mul_add(alpha, 100.0)));
        assert!(
            (shown.y - expected.y).abs() < 1e-3,
            "{shown:?} {expected:?}"
        );
        assert_eq!(shown.x, expected.x);
    }

    #[test]
    fn an_npc_whose_sheet_cannot_be_read_is_a_crossed_box() {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(trafficked(&[130], 1, 130, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let list = drawn(&view);
        let mut boxed = DrawList::new();
        crossed_box(&mut boxed, at(612.0, 284.0), PLACEHOLDER_SIZE, PLACEHOLDER);
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
        assert_eq!(sprites(&list).len(), 3, "two stellars and the player");
    }

    #[test]
    fn each_ship_types_sheet_is_read_once() {
        let mut view = FlightView::new(trafficked(&[130, 131], 3, 129, 1));
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        ticks(&mut view, 5);
        assert_eq!(npc_ids(&view).len(), 3);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        jump_to_alpha(&mut view);
        assert!(!npc_ids(&view).is_empty());
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "not again"
        );
    }

    #[test]
    fn arriving_repopulates_from_the_new_system() {
        let mut view = FlightView::new(trafficked(&[131], 2, 129, 1));
        view.tick(TICK);
        assert_eq!(npc_ids(&view), []);
        jump_to_alpha(&mut view);
        assert_eq!(npc_ids(&view).len(), 2);
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "read on arrival"
        );
    }

    #[test]
    fn npcs_stand_still_while_the_map_is_open_or_the_ship_is_landed() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        ticks(&mut view, 3);
        let before = npc_states(&view);
        assert_eq!(before.len(), 2);
        tap(&mut view, MAP);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "the map is open");
        tap(&mut view, MAP);
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        let before = npc_states(&view);
        ticks(&mut view, 10);
        assert_eq!(npc_states(&view), before, "landed");
    }

    #[test]
    fn taking_off_repopulates_the_system() {
        let catalog = FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 2, 129, 1)
        };
        let mut view = FlightView::new(catalog);
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(0), NpcId(1)]);
        land_now(&mut view);
        view.take_off().expect("took off");
        assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "before any tick");
        view.tick(TICK);
        assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "populated once");
    }

    /// [`trafficked`] with one ship 129 of AI 3 in 130, over planet 128
    /// at the centre.
    fn trafficked_over_128() -> FakeCatalog {
        FakeCatalog {
            sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
            ..trafficked(&[130], 1, 129, 3)
        }
    }

    /// Whether `list` draws a sprite at world point `world`.
    fn sprite_at(view: &View, list: &DrawList, world: Vec2) -> bool {
        let screen = view.camera().world_to_screen(point(world));
        sprites(list).iter().any(|&(_, center)| center == screen)
    }

    #[test]
    fn the_first_frame_after_take_off_draws_none_of_the_ships_from_before_the_landing() {
        let mut draws = placed(850, 650, 90).to_vec();
        // The take-off's escort-strength seed, then its population.
        draws.push(0);
        draws.extend(placed(650, 850, 90));
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked_over_128())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let before = npc_states(&view)[0].position;
        assert_eq!(before, Vec2::new(100.0, -100.0));
        land_now(&mut view);
        view.take_off().expect("took off");
        let after = npc_states(&view)[0].position;
        assert_eq!(after, Vec2::new(-100.0, 100.0));

        let list = drawn(&view);
        assert!(!sprite_at(&view, &list, before), "with no tick");
        assert!(sprite_at(&view, &list, after), "with no tick");

        let states = npc_states(&view);
        view.tick(TICK / 4);
        assert_eq!(npc_states(&view), states, "no step");
        let list = drawn(&view);
        assert!(!sprite_at(&view, &list, before), "after a frame of no step");
        assert!(sprite_at(&view, &list, after), "after a frame of no step");
    }

    #[test]
    fn a_ship_type_first_flown_at_take_off_is_drawn_with_its_sprite_at_once() {
        // The take-off's escort-strength seed, then its population.
        let mut draws = vec![0];
        draws.extend(placed(650, 850, 90));
        let (_, chance) = scripted(&draws);
        let mut view = FlightView::new(trafficked_over_128())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        land_now(&mut view);
        assert!(view.take_landing().is_some(), "landed before any traffic");
        view.take_off().expect("took off");
        let after = npc_states(&view)[0].position;
        let list = drawn(&view);
        let mut boxed = DrawList::new();
        crossed_box(
            &mut boxed,
            view.camera().world_to_screen(point(after)),
            PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            !commands.windows(expected.len()).any(|run| run == expected),
            "no placeholder"
        );
        assert!(sprite_at(&view, &list, after), "its own sprite");
    }

    // Combat.

    /// Says no ship is disabled, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Counting {
        asked: std::cell::Cell<usize>,
    }

    impl DisableRule for Counting {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    #[test]
    fn the_fight_asks_the_disable_rule_given_each_step_in_flight() {
        let rule = Rc::new(Counting::default());
        let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)])
            .with_disable_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(rule.asked.get(), 1, "the player, the only ship");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3);
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while landed");
    }

    /// Every NPC idles and holds its trigger.
    #[derive(Debug)]
    struct Firing;

    impl Behaviour for Firing {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
                only: None,
                turrets_only: false,
                bays: false,
            }
        }
    }

    #[test]
    fn the_sessions_diagnostics_come_through_the_screen() {
        let mining = nova_sim::WeaponRecord {
            id: nova_sim::WeaponId(181),
            reload: 20,
            count: 12,
            mass_dmg: 2,
            energy_dmg: 2,
            guidance: -1,
            speed: 1000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 5,
            explod_type: -1,
            prox_radius: 5,
            blast_radius: 6,
            flags: 0,
            seeker: 0,
            flags2: 0x8000,
            flags3: 0,
            decay: 0,
            beam_length: 0,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
            max_ammo: 0,
            prox_safety: 0,
        };
        let hull = nova_sim::HullRecord {
            id: ShipId(129),
            flags: 0,
            death_delay: 0,
            explode1: -1,
            explode2: -1,
            mass: 0,
            weapons: vec![nova_sim::StockWeapon {
                weapon: nova_sim::WeaponId(181),
                count: 1,
                ammo: 0,
            }],
            size: None,
            height: None,
            strength: 0,
        };
        let (_, chance) = scripted(&placed(850, 650, 90));
        let catalog = FakeCatalog {
            weapons: vec![mining],
            hulls: vec![hull],
            looks: vec![(181, Ok(WeaponLook::default()))],
            ..trafficked(&[130], 1, 129, 3)
        };
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Firing));
        assert_eq!(view.take_diagnostics(), []);
        ticks(&mut view, 3);
        assert_eq!(
            view.take_diagnostics(),
            [Diagnostic::Sim(SimDiagnostic::UnimplementedWeaponFlag {
                weapon: nova_sim::WeaponId(181),
                field: nova_sim::combat::flags::FlagField::Flags2,
                bit: 0x8000
            })]
        );
        ticks(&mut view, 30);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    #[test]
    fn a_look_whose_sheet_cannot_be_read_still_sounds_and_is_reported_once() {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.looks[0].1 = Ok(WeaponLook {
            sheet: Some(Err("no spïn 3005".to_owned())),
            ..look("Blaster", 208, None)
        });
        catalog.booms.push((
            129,
            Ok(BoomLook {
                sheet: Err("no spïn 401".to_owned()),
                advance: 1.0,
                sound: None,
            }),
        ));
        let mut view = fighting(catalog, &[]);
        assert_eq!(
            view.take_diagnostics(),
            [
                Diagnostic::Unreadable("wëap 128: no spïn 3005".to_owned()),
                Diagnostic::Unreadable("no wëap 150".to_owned()),
                Diagnostic::Unreadable("bööm 129: no spïn 401".to_owned()),
            ],
            "and the unseen weapon, which has no look at all"
        );
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let sounds: Vec<_> = combat_sounds(&mut view).iter().map(|s| s.sound).collect();
        assert_eq!(sounds, [nova_sim::SoundId(208)], "the blaster is heard");
        ticks(&mut view, 3);
        assert_eq!(view.take_diagnostics(), [], "once");
    }

    // Combat controls and display.

    use std::num::NonZeroU16 as Frames;

    use nova_sim::{BoomId, Condition, WeaponId};

    use crate::flight::brackets::{self, Standing};
    use crate::flight::catalog::EffectSheet;
    use crate::flight::effects::{DEBRIS_COLOR, Effects};
    use crate::flight::target::{self, NO_SECONDARY, NO_TARGET};
    use crate::flight::weapons::{BEAM_UNDER_SHIPS, translucent};
    use crate::sound::CombatSound;

    const BLASTER: WeaponId = WeaponId(128);
    const ROCKET: WeaponId = WeaponId(140);
    const MISSILE: WeaponId = WeaponId(141);
    const TORCH: WeaponId = WeaponId(142);
    const UNDER: WeaponId = WeaponId(146);
    const OVER: WeaponId = WeaponId(147);
    const UNSEEN: WeaponId = WeaponId(150);

    /// `wëap` `id` firing every tick, 20 pixels a tick for 30 ticks, doing
    /// no damage, with `flags`, `guidance` and `explod_type`.
    fn gun(id: WeaponId, flags: u16, guidance: i16, explod_type: i16) -> nova_sim::WeaponRecord {
        nova_sim::WeaponRecord {
            id,
            reload: 0,
            count: 30,
            mass_dmg: 0,
            energy_dmg: 0,
            guidance,
            speed: 2000,
            ammo_type: -1,
            inaccuracy: 0,
            impact: 0,
            explod_type,
            prox_radius: 0,
            blast_radius: 0,
            flags,
            seeker: 0,
            flags2: 0,
            flags3: 0,
            decay: 0,
            beam_length: 50,
            burst_count: 0,
            burst_reload: 0,
            guided_turn: 0,
            durability: 0,
            sub_count: 0,
            sub_type: None,
            sub_theta: 0,
            sub_limit: 0,
            max_ammo: 0,
            prox_safety: 0,
        }
    }

    /// Ship `id` carrying one of each of `weapons`, gone at once in
    /// `bööm` 128.
    fn hull_of(id: i16, weapons: &[WeaponId]) -> nova_sim::HullRecord {
        nova_sim::HullRecord {
            id: ShipId(id),
            flags: 0,
            death_delay: 0,
            explode1: -1,
            explode2: 0,
            mass: 0,
            weapons: weapons
                .iter()
                .map(|&weapon| nova_sim::StockWeapon {
                    weapon,
                    count: 1,
                    ammo: 0,
                })
                .collect(),
            size: None,
            height: None,
            strength: 0,
        }
    }

    /// `oütf` `id`, "Gun `id`", holding a `weapon`: a ton, 1000 credits.
    fn holding(id: i16, weapon: WeaponId) -> OutfitRecord {
        OutfitRecord {
            id: OutfitId(id),
            name: format!("Gun {id}"),
            short_name: format!("Gun {id}"),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 1000,
            mods: [
                (nova_sim::combat::armament::MOD_WEAPON, weapon.0),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: format!("gun {id}"),
            lc_plural: format!("guns {id}"),
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }
    }

    fn effect_sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            image_id,
            frames: Frames::new(frames).expect("non-zero"),
        }
    }

    /// A look named `name` sounding `snd ` `sound`, its shots on `rlëD`
    /// `image` of 36 frames, if any.
    fn look(name: &str, sound: i16, image: Option<i16>) -> WeaponLook {
        WeaponLook {
            name: name.to_owned(),
            sheet: image.map(|image| Ok(effect_sheet(image, 36))),
            sound: Some(nova_sim::SoundId(sound)),
            ..WeaponLook::default()
        }
    }

    /// [`trafficked`] (ships of type 129, warships) where the player's
    /// ship carries `weapons`, held by outfits 250 on, in that order; the
    /// blaster, rockets, missiles and torch have looks, and `bööm` 128 shows `rlëD` 400's 3 frames at a frame
    /// a step, sounding `snd ` 302. Ship 129 is a "Light Transport" with
    /// picture 3001, and govt 140's code is "Fed.".
    fn armed(avg: i16, weapons: &[WeaponId]) -> FakeCatalog {
        let mut catalog = trafficked(&[130], avg, 129, 3);
        catalog.dudes[0].1.govt = Some(GovtId(140));
        FakeCatalog {
            weapons: vec![
                gun(BLASTER, 0, -1, 0),
                gun(ROCKET, 0x0002, -1, -1),
                gun(MISSILE, 0x0002, -1, -1),
                gun(TORCH, 0x0002, -1, -1),
                gun(UNDER, 0, 0, -1),
                gun(OVER, 0, 0, -1),
                gun(UNSEEN, 0, -1, -1),
            ],
            hulls: vec![hull_of(128, weapons), hull_of(129, &[])],
            outfits: (250..)
                .zip(weapons)
                .map(|(id, &weapon)| holding(id, weapon))
                .collect(),
            looks: vec![
                (128, Ok(look("Blaster", 208, Some(3500)))),
                (140, Ok(look("Rocket", 209, Some(3501)))),
                (141, Ok(look("Missile", 210, None))),
                (142, Ok(look("Torch", 211, None))),
                (
                    146,
                    Ok(WeaponLook {
                        flags2: BEAM_UNDER_SHIPS,
                        beam_width: 1,
                        beam_color: 0x0000_00FF,
                        ..WeaponLook::default()
                    }),
                ),
                (
                    147,
                    Ok(WeaponLook {
                        beam_width: 1,
                        beam_color: 0x0000_FF00,
                        ..WeaponLook::default()
                    }),
                ),
            ],
            booms: vec![(
                128,
                Ok(BoomLook {
                    sheet: Ok(effect_sheet(400, 3)),
                    advance: 1.0,
                    sound: Some(nova_sim::SoundId(302)),
                }),
            )],
            cards: vec![(
                129,
                TargetCard {
                    subtitle: "Light Transport".to_owned(),
                    picture: Some(3001),
                },
            )],
            codes: vec![(140, "Fed.".to_owned())],
            ..catalog
        }
    }

    /// `catalog`'s flight with its NPCs idling, placed by `draws`, after
    /// the step that populates the system.
    fn fighting(catalog: FakeCatalog, draws: &[u32]) -> View {
        let (_, chance) = scripted(draws);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    /// NPC 0 at (100, -100) and NPC 1 at (-50, 0), nearer the player.
    fn two_npcs() -> Vec<u32> {
        [placed(850, 650, 90), placed(700, 750, 0)].concat()
    }

    fn target_of(view: &View) -> Option<NpcId> {
        view.session().expect("flying").target().map(|npc| npc.id)
    }

    /// Taps R while holding Alt (Option).
    fn alt_tap(view: &mut View, k: Key) {
        view.input(&key(Key::Alt, true));
        tap(view, k);
        view.input(&key(Key::Alt, false));
    }

    #[test]
    fn tab_targets_the_next_npc_and_alt_r_the_nearest() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        assert_eq!(target_of(&view), None);
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        view.input(&held(Key::Tab));
        assert_eq!(target_of(&view), Some(NpcId(0)), "a repeat does nothing");
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), None, "past the last");
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        tap(&mut view, Key::Tab);
        view.input(&key(Key::Alt, true));
        view.input(&held(NEAREST_KEY));
        assert_eq!(target_of(&view), None, "a repeat does nothing");
        assert_eq!((TARGET_KEY, NEAREST_KEY), (Key::Tab, Key::Char('r')));
    }

    /// NPC 0 attacks the player; the rest idle.
    #[derive(Debug)]
    struct Threatening;

    impl Behaviour for Threatening {
        fn decide(
            &self,
            npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            if npc.id == NpcId(0) {
                Goal::Attack(ShipRef::Player)
            } else {
                Goal::Idle
            }
        }
    }

    #[test]
    fn r_targets_the_nearest_threat_over_a_nearer_ship_and_brackets_it_red() {
        let mut view = fighting(armed(2, &[]), &two_npcs()).with_behaviour(Rc::new(Threatening));
        tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), None, "no threat yet: unchanged");
        view.tick(TICK);
        tap(&mut view, NEAREST_KEY);
        assert_eq!(
            target_of(&view),
            Some(NpcId(0)),
            "the threat, though farther"
        );
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10008, 10009, 10010, 10011], "red");
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)), "Alt-R: the nearer");
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10012, 10013, 10014, 10015], "yellow");
    }

    /// How many of the shots in flight are of `weapon`.
    fn shots_of(view: &View, weapon: WeaponId) -> usize {
        let session = view.session().expect("flying");
        session
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == weapon)
            .count()
    }

    #[test]
    fn space_fires_the_primaries_and_control_the_secondary_while_held() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        tap(&mut view, SELECT_KEY);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 2);
        view.input(&held(FIRE_KEY));
        ticks(&mut view, 1);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 0));
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 2);
        assert_eq!(shots_of(&view, BLASTER), 3, "let go");
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 2));
        view.input(&key(SECONDARY_KEY, false));
        ticks(&mut view, 1);
        assert_eq!(shots_of(&view, ROCKET), 2, "let go");
        assert_eq!((FIRE_KEY, SECONDARY_KEY), (Key::Space, Key::Control));
    }

    #[test]
    fn letting_go_of_the_keys_lets_go_of_the_fire_keys() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        tap(&mut view, SELECT_KEY);
        view.input(&key(FIRE_KEY, true));
        view.input(&key(SECONDARY_KEY, true));
        ticks(&mut view, 1);
        view.release_keys();
        ticks(&mut view, 2);
        assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (1, 1));
    }

    fn secondary_of(view: &View) -> Option<WeaponId> {
        view.session().expect("flying").secondary()
    }

    #[test]
    fn w_selects_the_next_secondary_and_alt_w_the_one_before() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE, TORCH]), &[]);
        assert_eq!(secondary_of(&view), None);
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE));
        view.input(&held(SELECT_KEY));
        assert_eq!(secondary_of(&view), Some(MISSILE), "a repeat does nothing");
        view.input(&key(Key::Alt, true));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(TORCH), "wrapping");
        view.input(&key(Key::Alt, false));
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        view.input(&key(Key::Alt, true));
        view.release_keys();
        tap(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE), "Alt let go");
        assert_eq!(SELECT_KEY, Key::Char('w'));
    }

    #[test]
    fn the_combat_keys_do_nothing_while_the_map_is_open() {
        let mut view = fighting(armed(2, &[BLASTER, ROCKET, MISSILE]), &two_npcs());
        tap(&mut view, MAP);
        for k in [Key::Tab, NEAREST_KEY, SELECT_KEY, FIRE_KEY, SECONDARY_KEY] {
            view.input(&key(k, true));
        }
        tap(&mut view, MAP);
        ticks(&mut view, 2);
        assert_eq!(target_of(&view), None);
        assert_eq!(secondary_of(&view), None);
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    #[test]
    fn the_combat_keys_do_nothing_while_a_jump_plays() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[]);
        tap(&mut view, SELECT_KEY);
        plot(&mut view, 131);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        for k in [NEAREST_KEY, SELECT_KEY, FIRE_KEY] {
            view.input(&key(k, true));
        }
        view.tick(ms(2000));
        assert_eq!(view.jump_effect(), None);
        ticks(&mut view, 2);
        assert_eq!(secondary_of(&view), Some(ROCKET));
        assert_eq!(view.session().expect("flying").shots(), []);
    }

    /// The combat sounds taken.
    fn combat_sounds(view: &mut View) -> Vec<CombatSound> {
        view.take_sounds()
            .into_iter()
            .filter_map(|sound| match sound {
                Sound::Combat(sound) => Some(sound),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_steps_fight_reaches_the_effects() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK * 3);
        assert_eq!(shots_of(&view, BLASTER), 3, "three steps");
        let fired = CombatSound {
            sound: nova_sim::SoundId(208),
            offset: (0, 0),
        };
        assert_eq!(combat_sounds(&mut view), [fired; 3]);
        assert_eq!(combat_sounds(&mut view), [], "taken");
    }

    #[test]
    fn the_sounds_are_the_sessions_then_the_fights() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        assert_eq!(
            view.take_sounds(),
            [
                Sound::Sim(SimSound::ThrustStarted),
                Sound::Combat(CombatSound {
                    sound: nova_sim::SoundId(208),
                    offset: (0, 0),
                })
            ]
        );
    }

    /// The first command matching `wanted`'s position in `list`.
    fn first(list: &DrawList, wanted: impl Fn(&DrawCommand) -> bool) -> usize {
        list.iter()
            .position(wanted)
            .unwrap_or_else(|| panic!("not drawn: {list:?}"))
    }

    fn sprite_of(id: i16) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == id)
    }

    fn line_in(color: Color) -> impl Fn(&DrawCommand) -> bool {
        move |command| matches!(command, DrawCommand::Line { color: c, .. } if *c == color)
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

    /// The flight with an NPC 100 pixels above the player, the player
    /// firing its blaster (which explodes where it hits) and two beams, one
    /// drawn under the ships and one over them, at it.
    fn firing_at_an_npc() -> View {
        let catalog = armed(1, &[BLASTER, UNDER, OVER]);
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view
    }

    #[test]
    fn the_fight_is_drawn_between_the_stellars_and_the_hud_in_order() {
        let view = firing_at_an_npc().with_overlays(TITLED);
        let list = drawn(&view);
        let stellar = first(&list, sprite_of(1128));
        let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
        let npc = first(&list, sprite_of(2001));
        let ship = first(&list, sprite_of(2000));
        let shot = first(&list, sprite_of(3500));
        let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
        let explosion = first(&list, sprite_of(400));
        let brackets = first(&list, icon_in(10008..=10023));
        let title = first(
            &list,
            |c| matches!(c, DrawCommand::Text { origin, .. } if *origin == TITLE),
        );
        let hud = hud_at(&list);
        let order = [
            stellar, under, npc, ship, shot, over, explosion, brackets, title, hud,
        ];
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
        let DrawCommand::Sprite { tint, .. } = list.iter().nth(explosion).cloned().expect("drawn")
        else {
            panic!("a sprite")
        };
        assert_eq!(tint, translucent());
    }

    #[test]
    fn a_shot_is_drawn_with_its_frame_where_it_flies() {
        let mut view = fighting(armed(0, &[BLASTER]), &[]);
        // Turned 45 degrees right, so the shot flies across and up.
        view.input(&key(Key::Right, true));
        ticks(&mut view, 15);
        view.input(&key(Key::Right, false));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let shot = session.shots()[0];
        let alpha = view.alpha();
        let shown = at(
            shot.position.x - shot.velocity.x * (1.0 - alpha),
            shot.position.y - shot.velocity.y * (1.0 - alpha),
        );
        let list = drawn(&view);
        let drawn_at = sprites(&list)
            .into_iter()
            .find(|(image, _)| image.id == 3500)
            .expect("the shot");
        assert_eq!(drawn_at.0, ImageKey::sprite(3500, 4), "heading 45");
        let expected = view.camera().world_to_screen(shown);
        assert!(
            (drawn_at.1.x - expected.x).abs() < 1e-3 && (drawn_at.1.y - expected.y).abs() < 1e-3,
            "{drawn_at:?} {expected:?}"
        );
    }

    /// Every NPC heads for planet 128 and holds its trigger.
    #[derive(Debug)]
    struct LandingFiring;

    impl Behaviour for LandingFiring {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Land(StellarId(128))
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
                only: None,
                turrets_only: false,
                bays: false,
            }
        }
    }

    /// Where the beams in `color` start, on screen: the lines a pixel wide,
    /// not the HUD's bars.
    fn beam_starts(list: &DrawList, color: Color) -> Vec<Point> {
        list.iter()
            .filter_map(|command| match *command {
                DrawCommand::Line {
                    from,
                    color: c,
                    width,
                    ..
                } if c == color && width == 1.0 => Some(from),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_beam_is_drawn_from_its_firer_where_the_firer_is_drawn() {
        let mut catalog = armed(1, &[OVER]);
        catalog.hulls[1] = hull_of(129, &[UNDER]);
        let (_, chance) = scripted(&placed(850, 650, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(LandingFiring));
        view.tick(TICK);
        view.input(&key(Key::Up, true));
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 5);
        view.tick(TICK / 2);
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        assert_ne!(
            view.npc_previous[&npc.id].position, npc.state.position,
            "moving"
        );
        assert_ne!(view.previous.position, session.player().position, "moving");
        let list = drawn(&view);
        let camera = view.camera();
        let player = camera.world_to_screen(view.shown_position());
        let over = beam_starts(&list, Color::rgba(0, 255, 0, 255));
        assert!(!over.is_empty());
        assert!(
            over.iter().all(|&start| start == player),
            "the player's, from the player as drawn: {over:?}"
        );
        let (npc_shown, _) = view.shown_npc(npc);
        let under = beam_starts(&list, Color::rgba(0, 0, 255, 255));
        assert!(!under.is_empty());
        let expected = camera.world_to_screen(npc_shown);
        assert!(
            under
                .iter()
                .all(|start| (start.x - expected.x).abs() < 1e-3
                    && (start.y - expected.y).abs() < 1e-3),
            "the NPC's, from the NPC as drawn: {under:?} {expected:?}"
        );
    }

    #[test]
    fn a_ship_breaking_up_goes_off_about_where_it_is() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[1].death_delay = 30;
        catalog.hulls[1].explode1 = 0;
        // 100 above, facing away: it flies on up the line of fire.
        let mut view = fighting(catalog, &placed(750, 650, 0));
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        let mut breaking = None;
        for _ in 0..40 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Some(npc) = session.npcs().first()
                && let Condition::Dying { ticks_left } = npc.condition
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(npc.state.position - npc.state.velocity);
                break;
            }
        }
        // Where it was before the tick's flight step moved it on.
        let at = breaking.expect("an explosion as it breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of its 40-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 9.0, at.y + 9.0)
        );
    }

    #[test]
    fn the_players_ship_breaking_up_goes_off_about_where_it_is() {
        // The NPC, straight above the player and facing it, fires a
        // blaster that kills at a hit; the player's ship, 80 pixels wide
        // (the NPC's 40), takes 30 ticks to break up.
        let mut catalog = armed(1, &[]);
        catalog.sheet = Ok(ShipSheet {
            frame_width: 80,
            ..sheet()
        });
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        catalog.hulls[0].death_delay = 30;
        catalog.hulls[0].explode1 = 0;
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
        let mut breaking = None;
        for _ in 0..40 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            if let Condition::Dying { ticks_left } = session.player_condition()
                && ticks_left < 19
                && !view.effects().explosions().is_empty()
            {
                breaking = Some(session.player().position);
                break;
            }
        }
        let at = breaking.expect("an explosion as the player's ship breaks up");
        // Never firing, the effects' chance draws the last outcome: a
        // quarter of the player's 80-pixel sprite less one out, each way.
        assert_eq!(
            view.effects().explosions()[0].at,
            Point::new(at.x + 19.0, at.y + 19.0)
        );
    }

    #[test]
    fn a_shot_whose_look_cannot_be_read_is_a_crossed_box() {
        let mut view = fighting(armed(0, &[UNSEEN]), &[]);
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        let list = drawn(&view);
        let shot = view.session().expect("flying").shots()[0];
        let mut boxed = DrawList::new();
        // A whole step on, it is drawn a step behind, where it left.
        assert_eq!(view.alpha(), 0.0);
        let left = shot.position - shot.velocity;
        let centre = view.camera().world_to_screen(at(left.x, left.y));
        crossed_box(
            &mut boxed,
            centre,
            weapons::SHOT_PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = boxed.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn a_destroyed_npc_explodes_and_scatters_debris_on_the_frames_after() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        // Drifting up, so the explosion is heard from where the player is.
        view.input(&key(Key::Up, true));
        view.tick(TICK);
        view.input(&key(Key::Up, false));
        view.input(&key(FIRE_KEY, true));
        let mut destroyed = false;
        for _ in 0..10 {
            view.tick(TICK);
            if view.session().expect("flying").npcs().is_empty() {
                destroyed = true;
                break;
            }
        }
        assert!(destroyed, "{:?}", view.session().expect("flying").npcs());
        assert_eq!(target_of(&view), None);
        let effects = view.effects();
        assert_eq!(effects.explosions().len(), 1);
        assert_eq!(effects.explosions()[0].boom, BoomId(128));
        assert_eq!(effects.debris().len(), 16);
        let list = drawn(&view);
        let explosion = first(&list, sprite_of(400));
        assert_eq!(
            list.iter()
                .skip(explosion)
                .filter(|c| matches!(c, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR))
                .count(),
            16,
            "drawn after the explosion"
        );
        let player = view.session().expect("flying").player().position;
        assert!(player.y < 0.0, "{player:?}");
        let boom = view.effects().explosions()[0].at;
        let heard = CombatSound {
            sound: nova_sim::SoundId(302),
            offset: (
                (boom.x - player.x).round() as i32,
                (boom.y - player.y).round() as i32,
            ),
        };
        assert!(
            combat_sounds(&mut view).contains(&heard),
            "the explosion is heard: {heard:?}"
        );
    }

    /// [`armed`] with a blaster whose shots detonate after a tick.
    fn detonating() -> FakeCatalog {
        let mut catalog = armed(0, &[BLASTER]);
        catalog.weapons[0].flags = 0x8000;
        catalog.weapons[0].count = 1;
        catalog
    }

    /// Fires one detonating shot, which explodes.
    fn detonate(view: &mut View) {
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        assert_eq!(view.effects().explosions().len(), 1);
    }

    #[test]
    fn landing_clears_the_effects() {
        let mut catalog = detonating();
        catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        let mut view = fighting(catalog, &[]);
        detonate(&mut view);
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        assert_eq!(*view.effects(), Effects::default());
    }

    #[test]
    fn arriving_clears_the_effects() {
        let mut view = fighting(detonating(), &[]);
        plot(&mut view, 131);
        fly_out(&mut view);
        detonate(&mut view);
        jump_now(&mut view);
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(131));
        assert_eq!(*view.effects(), Effects::default());
    }

    #[test]
    fn the_target_is_bracketed_and_its_panel_shows_it() {
        let view = firing_at_an_npc();
        let session = view.session().expect("flying");
        let npc = &session.npcs()[0];
        let list = drawn(&view);
        let (shown, _) = view.shown_npc(npc);
        let Some(Ok(sheet)) = view.npc_sheets.get(&npc.ship) else {
            panic!("a sheet");
        };
        let mut expected = DrawList::new();
        brackets::draw_ship(
            &mut expected,
            view.camera().world_to_screen(shown),
            Some((sheet.frame_width, sheet.frame_height)),
            0.0,
            Standing::Neutral,
        );
        let commands: Vec<_> = list.iter().cloned().collect();
        let expected: Vec<_> = expected.iter().cloned().collect();
        assert!(
            commands.windows(4).any(|run| run == expected),
            "{commands:?}"
        );
        let bar = view.status_bar().expect("a status bar");
        let mut panel = DrawList::new();
        target::draw_target_panel(
            &mut panel,
            &bar.layout,
            hud::bar_origin(bar),
            Some(&TargetPanel {
                name: "Ship 129".to_owned(),
                subtitle: "Light Transport".to_owned(),
                picture: Some(3001),
                status: nova_sim::TargetStatus::Shield(100),
                tag: Some(Tag::Code("Fed.".to_owned())),
            }),
            None,
        );
        let expected: Vec<_> = panel.iter().cloned().collect();
        assert!(
            commands.windows(expected.len()).any(|run| run == expected),
            "{commands:?}"
        );
    }

    #[test]
    fn the_nav_brackets_draw_under_the_ship_brackets() {
        let mut view = firing_at_an_npc();
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        let list = drawn(&view);
        let nav = first(&list, icon_in(10000..=10007));
        let ship = first(&list, icon_in(10008..=10023));
        let npc = first(&list, sprite_of(2001));
        assert!(npc < nav && nav + 4 == ship, "{npc} {nav} {ship}");
        assert_eq!(
            bracket_icons(&list),
            [10000, 10001, 10002, 10003, 10012, 10013, 10014, 10015]
        );
    }

    #[test]
    fn a_disabled_targets_brackets_are_grey() {
        let catalog = armed(1, &[]);
        let mut view =
            fighting(catalog, &placed(750, 650, 180)).with_disable_rule(Rc::new(Disabling));
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        let list = drawn(&view);
        assert_eq!(bracket_icons(&list), [10020, 10021, 10022, 10023]);
        assert!(texts(&list).contains(&target::DISABLED.to_owned()));
    }

    /// The ship bracket `cïcn`s drawn (10008-10023), and where.
    fn ship_bracket_corners(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(10008..=10023)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    /// Where `view` draws its target's sprite.
    fn target_on_screen(view: &View) -> Point {
        let session = view.session().expect("flying");
        let npc = session.target().expect("a target");
        view.camera().world_to_screen(view.shown_npc(npc).0)
    }

    #[test]
    fn targeting_a_ship_on_screen_flies_its_brackets_in_then_settles_them() {
        let mut view = fighting(armed(1, &[]), &placed(750, 650, 180));
        assert_eq!(ship_bracket_corners(&view), []);
        tap(&mut view, Key::Tab);
        // Ship 129, 40 x 40: half 20; neutral, from 10012.
        assert_eq!(
            ship_bracket_corners(&view),
            corners_at(10012, target_on_screen(&view), 20.0 + 196.0),
            "a frame drawn before the next tick"
        );
        for expand in [196.0, 136.0, 76.0, 16.0, 0.0, 0.0] {
            view.tick(TICK);
            assert_eq!(
                ship_bracket_corners(&view),
                corners_at(10012, target_on_screen(&view), 20.0 + expand),
                "{expand}"
            );
        }
    }

    #[test]
    fn a_target_whose_sheet_cannot_be_read_is_bracketed_round_its_crossed_box() {
        let mut view = fighting(trafficked(&[130], 1, 130, 3), &placed(850, 650, 90));
        tap(&mut view, Key::Tab);
        ticks(&mut view, 5);
        // Its 64 x 64 box: half 32.
        assert_eq!(
            ship_bracket_corners(&view),
            corners_at(10012, target_on_screen(&view), PLACEHOLDER_SIZE / 2.0)
        );
    }

    /// What the target panel's status line shows this frame, if there is
    /// a target.
    fn status_shown(view: &View) -> Option<nova_sim::TargetStatus> {
        view.target_shown()
            .as_ref()
            .as_ref()
            .map(|panel| panel.status)
    }

    /// What the target panel's status line would show of the target now.
    fn status_now(view: &View) -> Option<nova_sim::TargetStatus> {
        view.target_now().map(|panel| panel.status)
    }

    /// An unarmed flight targeting its one NPC, intact, its panel drawn
    /// two ticks after a timed redraw.
    fn targeting_after_a_redraw(view: View) -> View {
        let mut view = view;
        while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        assert_eq!(status_shown(&view), status_now(&view));
        assert!(
            matches!(status_shown(&view), Some(nova_sim::TargetStatus::Shield(_))),
            "{:?}",
            status_shown(&view)
        );
        view
    }

    #[test]
    fn the_target_panel_holds_a_disabling_until_its_next_timed_redraw() {
        let view = fighting(armed(1, &[]), &placed(750, 650, 180));
        let view = targeting_after_a_redraw(view);
        let held = status_shown(&view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
        while !now_tick(&view).is_multiple_of(TARGET_PERIOD_TICKS) {
            assert_eq!(status_shown(&view), held, "tick {}", now_tick(&view));
            assert!(!texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
            view.tick(TICK);
        }
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
        assert!(texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
    }

    #[test]
    fn read_the_bibles_way_the_target_panel_follows_every_frame() {
        let view = every_frame(fighting(armed(1, &[]), &placed(750, 650, 180)));
        let view = targeting_after_a_redraw(view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_ne!(now_tick(&view) % TARGET_PERIOD_TICKS, 0);
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn redrawing_the_status_retakes_the_target_panel() {
        let view = fighting(armed(1, &[]), &placed(750, 650, 180));
        let view = targeting_after_a_redraw(view);
        let mut view = view.with_disable_rule(Rc::new(Disabling));
        view.tick(TICK);
        assert_ne!(status_shown(&view), status_now(&view), "held");
        view.redraw_status();
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn a_change_of_target_shows_at_once() {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        tap(&mut view, Key::Tab);
        view.tick(TICK);
        let drawn_for = |view: &View| {
            view.target_scene
                .as_ref()
                .and_then(|(drawn, _)| drawn.map(|drawn| drawn.npc))
        };
        assert_eq!(drawn_for(&view), Some(NpcId(0)));
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(drawn_for(&view), Some(NpcId(0)), "not yet retaken");
        assert_eq!(*view.target_shown(), view.target_now(), "shown at once");
        view.tick(TICK);
        assert_eq!(drawn_for(&view), Some(NpcId(1)), "retaken");
        tap(&mut view, Key::Tab);
        assert_eq!(target_of(&view), None);
        assert_eq!(*view.target_shown(), None, "lost at once");
        let list = texts(&drawn(&view));
        assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
    }

    #[test]
    fn a_falling_shield_shows_at_the_radar_refreshes_only() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].energy_dmg = 1;
        let mut view = fighting(catalog, &placed(750, 650, 180));
        tap(&mut view, Key::Tab);
        view.input(&key(FIRE_KEY, true));
        let mut shown = status_shown(&view);
        let mut changes = 0;
        let mut held = 0;
        for _ in 0..240 {
            let (blink, before) = (view.hud_cadence.blink_on(), now_tick(&view));
            view.tick(TICK);
            let radar = view.hud_cadence.blink_on() != blink;
            let timed = now_tick(&view) / TARGET_PERIOD_TICKS > before / TARGET_PERIOD_TICKS;
            let now = status_shown(&view);
            if now != shown {
                changes += 1;
                assert!(radar || timed, "tick {}: {now:?}", now_tick(&view));
                assert_eq!(now, status_now(&view), "tick {}", now_tick(&view));
            }
            if now != status_now(&view) {
                held += 1;
            }
            shown = now;
            if target_of(&view).is_none() {
                break;
            }
        }
        view.input(&key(FIRE_KEY, false));
        assert!(changes > 1, "the shield fell: {changes}");
        assert!(held > 0, "and was held between refreshes");
    }

    #[test]
    fn a_disabled_player_has_no_fuel_gauge() {
        let mut catalog = armed(1, &[]);
        catalog.hulls[1] = hull_of(129, &[BLASTER]);
        let mut view = fighting(catalog, &placed(750, 650, 180))
            .with_behaviour(Rc::new(Firing))
            .with_disable_rule(Rc::new(Disabling));
        let colors = |view: &View| -> Vec<Color> {
            lines(&drawn(view))
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Line { color, .. } => Some(color),
                    _ => None,
                })
                .collect()
        };
        let layout = layout();
        assert!(colors(&view).contains(&layout.fuel_full), "intact");
        for _ in 0..40 {
            if view.session().expect("flying").player_condition() == Condition::Disabled {
                break;
            }
            view.tick(TICK);
        }
        assert_eq!(
            view.session().expect("flying").player_condition(),
            Condition::Disabled
        );
        let shown = colors(&view);
        assert!(!shown.contains(&layout.fuel_full), "{shown:?}");
        assert!(!shown.contains(&layout.fuel_partial), "{shown:?}");
        assert!(shown.contains(&layout.armor_color), "{shown:?}");
    }

    /// Disables every ship.
    #[derive(Debug)]
    struct Disabling;

    impl DisableRule for Disabling {
        fn disabled(&self, _armor: nova_sim::Gauge, _hull: &nova_sim::HullSpec) -> bool {
            true
        }
    }

    #[test]
    fn the_hud_shows_no_target_and_the_secondary_weapon() {
        let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
        let list = texts(&drawn(&view));
        assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
        assert!(list.contains(&NO_SECONDARY.to_owned()), "{list:?}");
        tap(&mut view, SELECT_KEY);
        let list = texts(&drawn(&view));
        assert!(list.contains(&"Rocket".to_owned()), "{list:?}");
        let unarmed = texts(&drawn(&fighting(armed(0, &[BLASTER]), &[])));
        assert!(unarmed.contains(&NO_SECONDARY.to_owned()), "{unarmed:?}");
    }

    #[test]
    fn the_panel_is_laid_out_by_the_metrics_given() {
        let view = fighting(armed(0, &[BLASTER]), &[])
            .with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
        let list = drawn(&view);
        let origin = list
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, origin, .. } if text == NO_TARGET => Some(*origin),
                _ => None,
            })
            .expect("No Target");
        // Centred: 9 characters of 6 in the 176 across from 838.
        assert_eq!(origin.x, 899.0);
    }

    #[test]
    fn the_effects_roll_on_their_own_chance() {
        let mut catalog = armed(1, &[BLASTER]);
        catalog.weapons[0].mass_dmg = 1000;
        catalog.weapons[0].flags = 0x0020;
        catalog.weapons[0].explod_type = -1;
        let (_, chance) = scripted(&placed(750, 650, 180));
        let (effects, effects_chance) = scripted(&[]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_effects_chance(effects_chance);
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 10);
        assert!(view.session().expect("flying").npcs().is_empty());
        assert_eq!(
            effects.borrow().asked,
            [360, 101, 31].repeat(16),
            "the debris, drawn on the effects' own chance"
        );
    }

    #[test]
    fn a_looped_weapon_is_heard_again_only_once_its_sound_has_played_out() {
        let mut catalog = armed(0, &[UNDER]);
        catalog.weapons[4].reload = 0;
        catalog.weapons[4].count = 5;
        catalog.looks[4].1 = Ok(WeaponLook {
            flags: weapons::LOOPED_SOUND,
            sound: Some(nova_sim::SoundId(220)),
            sound_ticks: Some(3),
            ..WeaponLook::default()
        });
        let mut view = fighting(catalog, &[]);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 4);
        assert_eq!(
            view.session().expect("flying").beams().len(),
            4,
            "a beam fired each step"
        );
        assert_eq!(combat_sounds(&mut view).len(), 2, "on the first and fourth");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 1);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view), [], "still playing");
        view.input(&key(FIRE_KEY, false));
        ticks(&mut view, 3);
        view.input(&key(FIRE_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(combat_sounds(&mut view).len(), 1, "played out");
    }

    #[test]
    fn the_metrics_debug_as_their_name() {
        let metrics = Metrics(Rc::new(crate::text::fixture::MonoMetrics));
        assert_eq!(format!("{metrics:?}"), "Metrics");
    }

    #[test]
    fn the_help_line_names_the_combat_keys() {
        for name in ["Space", "Ctrl", "W", "Tab", "R:"] {
            assert!(HELP.contains(name), "{name}: {HELP}");
        }
    }

    // Point defence.

    /// Says no missile is hostile, counting the times it is asked.
    #[derive(Debug, Default)]
    struct Unalarmed {
        asked: std::cell::Cell<usize>,
    }

    impl nova_sim::PointDefenceRule for Unalarmed {
        fn hostile(
            &self,
            _defender: nova_sim::combat::defence::Side,
            _firer: nova_sim::combat::defence::Side,
            _govts: &nova_sim::Governments,
        ) -> bool {
            self.asked.set(self.asked.get() + 1);
            false
        }
    }

    /// Every NPC idles, targets the player and holds its trigger.
    #[derive(Debug)]
    struct Attacking;

    impl Behaviour for Attacking {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            Goal::Idle
        }

        fn trigger(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> nova_sim::Trigger {
            nova_sim::Trigger {
                primary: true,
                secondary: None,
                only: None,
                turrets_only: false,
                bays: false,
            }
        }

        fn target(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
        ) -> Option<ShipRef> {
            Some(ShipRef::Player)
        }
    }

    const QUAD: WeaponId = WeaponId(133);
    const IR: WeaponId = WeaponId(134);

    /// [`armed`] with the player, over a planet at the centre, carrying a
    /// point-defence turret (held by outfit 250), and its traffic a
    /// missile it fires once, 5 pixels a tick.
    fn defending() -> FakeCatalog {
        let mut catalog = armed(1, &[QUAD]);
        catalog.sites = vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)];
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 5,
            count: 12,
            mass_dmg: 1,
            energy_dmg: 4,
            ..gun(QUAD, 0, 9, -1)
        });
        catalog.weapons.push(nova_sim::WeaponRecord {
            reload: 1000,
            count: 200,
            speed: 500,
            ..gun(IR, 0, 1, -1)
        });
        catalog.hulls = vec![hull_of(128, &[QUAD]), hull_of(129, &[IR])];
        catalog
    }

    /// `catalog`'s flight with its NPC attacking from 200 pixels above
    /// the player, after the step that populates the system.
    fn attacked(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&placed(750, 550, 180));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Attacking));
        view.tick(TICK);
        view
    }

    #[test]
    fn the_fight_asks_the_point_defence_rule_given_each_step_in_flight() {
        let rule = Rc::new(Unalarmed::default());
        let mut view = attacked(defending()).with_point_defence_rule(rule.clone());
        view.tick(TICK);
        assert_eq!(shots_of(&view, IR), 1, "fired at the player");
        assert_eq!(rule.asked.get(), 1, "of the one missile");
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3);
        assert_eq!(shots_of(&view, QUAD), 0, "never hostile");
        view.input(&key(MAP_KEY, true));
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while the map is open");
        view.input(&key(MAP_KEY, true));
        land_now(&mut view);
        assert!(view.take_landing().is_some());
        ticks(&mut view, 2);
        assert_eq!(rule.asked.get(), 3, "not while landed");
    }

    #[test]
    fn a_missile_shot_down_is_drawn_exploding_and_unheard() {
        let mut view = attacked(defending());
        let mut sounds = Vec::new();
        let mut exploded = false;
        for _ in 0..12 {
            view.tick(TICK);
            sounds.extend(combat_sounds(&mut view));
            let explosion = drawn(&view).iter().any(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 400),
            );
            if explosion {
                exploded = true;
                break;
            }
        }
        assert!(exploded, "bööm 128 drawn");
        assert_eq!(shots_of(&view, IR), 0, "shot down");
        assert!(
            sounds
                .iter()
                .all(|sound| sound.sound != nova_sim::SoundId(302)),
            "{sounds:?}"
        );
    }

    /// [`armed`] with a blaster doing 10 energy and 10 mass damage, and
    /// govt 140 that costs 3 to disable a ship of.
    fn damaging() -> FakeCatalog {
        let catalog = armed(1, &[BLASTER]);
        FakeCatalog {
            weapons: vec![nova_sim::WeaponRecord {
                reload: 5,
                mass_dmg: 10,
                energy_dmg: 10,
                ..gun(BLASTER, 0, -1, -1)
            }],
            govts: vec![nova_sim::GovtRecord {
                id: GovtId(140),
                flags: 0,
                flags2: 0,
                crime_tol: 0,
                initial_rec: 0,
                penalties: nova_sim::Penalties {
                    disable: 3,
                    ..nova_sim::Penalties::default()
                },
                max_odds: 100,
                classes: [-1; 4],
                allies: [-1; 4],
                enemies: [-1; 4],
                comm_name: String::new(),
                scan_mask: 0,
                scan_fine: 0,
            }],
            ..catalog
        }
    }

    #[test]
    fn by_default_npcs_fight_by_novas_ai_and_crimes_by_novas_law() {
        // The warship 100 above, facing away: it flies on up the line of
        // fire.
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::new(damaging()).with_chance(chance);
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        let mut answered = false;
        for _ in 0..120 {
            view.tick(TICK);
            let session = view.session().expect("flying");
            answered |= session
                .npcs()
                .first()
                .is_some_and(|npc| npc.goal == Goal::Attack(ShipRef::Player));
            if session
                .npcs()
                .first()
                .is_some_and(|npc| npc.condition == Condition::Disabled)
            {
                break;
            }
        }
        assert!(answered, "the warship hit turned on the player");
        let session = view.session().expect("flying");
        assert_eq!(session.npcs()[0].condition, Condition::Disabled);
        assert_eq!(session.pilot().legal_record(GovtId(140)), -3);
    }

    /// Records each crime, and changes nothing.
    #[derive(Debug, Default)]
    struct Witness {
        seen: RefCell<Vec<nova_sim::Crime>>,
    }

    impl nova_sim::LegalCode for Witness {
        fn penalties(
            &self,
            crime: nova_sim::Crime,
            _victim: Option<GovtId>,
            _govts: &nova_sim::Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push(crime);
            Vec::new()
        }
    }

    #[test]
    fn the_flight_convicts_by_the_law_it_is_given() {
        let witness = Rc::new(Witness::default());
        // 100 above, facing away: it flies on up the line of fire.
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::new(damaging())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_law(witness.clone());
        view.tick(TICK);
        view.input(&key(FIRE_KEY, true));
        for _ in 0..120 {
            view.tick(TICK);
            if view.session().expect("flying").npcs()[0].condition == Condition::Disabled {
                break;
            }
        }
        assert_eq!(*witness.seen.borrow(), [nova_sim::Crime::Disable]);
        let session = view.session().expect("flying");
        assert_eq!(session.pilot().legal_record(GovtId(140)), 0, "as it says");
    }

    // Boarding.

    use nova_sim::HullSpec;
    use nova_sim::reserves::Gauge;

    /// Disables every ship of some strength: the NPCs (ship 129, of
    /// strength 5), never the player (ship 128, of none).
    #[derive(Debug)]
    struct DisablesTheStrong;

    impl DisableRule for DisablesTheStrong {
        fn disabled(&self, _armor: Gauge, hull: &HullSpec) -> bool {
            hull.strength > 0.0
        }
    }

    /// Never repels, always gives odds of 75 and always captures; a person
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

    /// Always repels boarders; never asked anything more.
    #[derive(Debug)]
    struct Repels;

    impl BoardingRule for Repels {
        fn repels(&self, _booty: u16) -> bool {
            true
        }

        fn capture_odds(
            &self,
            _crew: &nova_sim::board::CaptureCrew,
            _chance: &mut dyn Chance,
        ) -> u8 {
            unreachable!("repelled")
        }

        fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
            unreachable!("repelled")
        }

        fn person_credits(&self, _credits: i32, _chance: &mut dyn Chance) -> i64 {
            unreachable!("repelled")
        }

        fn grant(
            &self,
            _grant: &nova_sim::grant::PersonGrant,
            _stock: &[nova_sim::grant::GrantStock],
            _free_mass: i64,
            _grant_max: nova_sim::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<nova_sim::grant::Granted> {
            unreachable!("repelled")
        }
    }

    /// [`trafficked`] with its ship 129 (of strength 5, a crew of 3 and a
    /// `Cost` of 150,000, and no speed, so it starts at rest) carrying
    /// money (`Booty` 0x0040), and the player's ship 128 a crew of 10.
    fn boardable() -> FakeCatalog {
        let mut catalog = trafficked(&[130], 1, 129, 1);
        catalog.dudes[0].1.booty = 0x0040;
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.crew = 3;
                record.cost = 150_000;
            }
        }
        let mut player = catalog.ships[0].clone();
        player.id = ShipId(128);
        player.crew = 10;
        catalog.ships[0].fields.speed = 0;
        catalog.ships.push(player);
        catalog.hulls = vec![nova_sim::HullRecord {
            strength: 5,
            ..hull_of(129, &[])
        }];
        catalog
    }

    /// [`boardable`]'s flight with its one NPC placed at (`x` - 750,
    /// `y` - 750) facing `heading`, disabled by [`DisablesTheStrong`] on
    /// the first tick unless `intact`, and targeted.
    fn beside(x: u32, y: u32, heading: u32, intact: bool) -> View {
        let (_, chance) = scripted(&placed(x, y, heading));
        let mut view = FlightView::new(boardable())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        if !intact {
            view = view.with_disable_rule(Rc::new(DisablesTheStrong));
        }
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        let session = view.session().expect("flying");
        assert_eq!(session.target().map(|npc| npc.id), Some(NpcId(0)));
        view
    }

    #[test]
    fn b_on_a_disabled_aligned_slow_target_reports_a_boarding_once() {
        let mut view = beside(750, 750, 0, false);
        assert_eq!(view.take_boarding(), None);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!((boarding.npc, boarding.ship), (NpcId(0), ShipId(129)));
        assert!(boarding.credits >= 1000, "{boarding:?}");
        assert_eq!(view.take_boarding(), None, "once");
        assert_eq!(view.boarding(), Some(boarding), "still under way");
        assert_eq!(view.message(), None);
        let session = view.session().expect("flying");
        assert!(session.npcs()[0].boarded);
        assert_eq!(BOARD_KEY, Key::Char('b'));
    }

    #[test]
    fn each_refused_boarding_says_why_in_the_originals_words() {
        let mut intact = beside(750, 750, 0, true);
        tap(&mut intact, BOARD_KEY);
        assert_eq!(intact.take_boarding(), None);
        assert_lasts(&mut intact, CANT_BOARD, 360);
        let mut far = beside(800, 750, 0, false);
        tap(&mut far, BOARD_KEY);
        assert_eq!(far.message(), Some(NOT_CLOSE_ENOUGH));
        let mut fast = beside(750, 750, 0, false);
        fast.input(&key(Key::Up, true));
        ticks(&mut fast, 10);
        fast.input(&key(Key::Up, false));
        tap(&mut fast, BOARD_KEY);
        assert_eq!(fast.message(), Some(TOO_FAST_TO_BOARD));
        let mut repelled = beside(750, 750, 0, false).with_boarding_rule(Rc::new(Repels));
        tap(&mut repelled, BOARD_KEY);
        assert_eq!(repelled.take_boarding(), None);
        assert_lasts(&mut repelled, REPELLED, MESSAGE_TICKS);
        assert_eq!(MESSAGE_TICKS, 240);
        assert_eq!(
            [CANT_BOARD, NOT_CLOSE_ENOUGH, TOO_FAST_TO_BOARD, REPELLED,],
            [
                "You can't board this ship.",
                "You're not close enough to board this ship.",
                "You're moving too fast to board this ship.",
                "You were repelled while attempting to board this ship.",
            ]
        );
    }

    #[test]
    fn a_misaligned_or_missing_target_says_nothing() {
        let mut aside = beside(750, 750, 90, false);
        tap(&mut aside, BOARD_KEY);
        assert_eq!(aside.message(), None);
        assert_eq!(aside.take_boarding(), None);
        let mut alone = flight();
        tap(&mut alone, BOARD_KEY);
        assert_eq!(alone.message(), None);
        assert_eq!(board_refusal_message(BoardRefusal::NoTarget), None);
        assert_eq!(board_refusal_message(BoardRefusal::Misaligned), None);
    }

    #[test]
    fn boarding_keeps_the_message() {
        let mut view = beside(750, 750, 0, false);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, BOARD_KEY);
        view.take_boarding().expect("boarded");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
        assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
        assert_lasts(&mut view, MISSION_FAILED, 240);
    }

    #[test]
    fn plundering_goes_through_the_session_and_says_what_it_took() {
        let mut view = beside(750, 750, 0, false);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        let cash = view.pilot().expect("a pilot").cash();
        assert_eq!(
            view.plunder(Take::Credits),
            Taken::Credits(boarding.credits)
        );
        assert_eq!(
            view.pilot().expect("a pilot").cash(),
            cash + boarding.credits
        );
        let said = format!(
            "You stole all the {} credits from this ship.",
            boarding.credits
        );
        assert_eq!(view.boarding().map(|now| now.credits), Some(0));
        assert_lasts(&mut view, &said, 240);
        assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
        assert_eq!(view.boarding(), None);
    }

    #[test]
    fn the_plunder_messages_are_the_originals() {
        let food = Some("Food");
        let message = |taken| plunder_message(taken, food, Some("Rockets"));
        let cargo = |stored| Taken::Cargo {
            good: Good::Commodity(0),
            stored,
        };
        assert_eq!(message(cargo(0)).as_deref(), Some(NO_CARGO_STORED));
        assert_eq!(
            message(cargo(1)).as_deref(),
            Some("You salvaged 1 ton of Food from this ship.")
        );
        assert_eq!(
            message(cargo(12)).as_deref(),
            Some("You salvaged 12 tons of Food from this ship.")
        );
        assert_eq!(
            plunder_message(cargo(3), None, None).as_deref(),
            Some("You salvaged 3 tons of cargo from this ship."),
            "a good with no name"
        );
        assert_eq!(
            message(Taken::Credits(5750)).as_deref(),
            Some("You stole all the 5750 credits from this ship.")
        );
        let ammo = |count| Taken::Ammo {
            outfit: OutfitId(310),
            count,
        };
        assert_eq!(message(ammo(0)).as_deref(), Some(NO_AMMO_STORED));
        assert_eq!(
            message(ammo(7)).as_deref(),
            Some("You salvaged 7 Rockets from this ship.")
        );
        let energy = |stored, full| Taken::Energy {
            offered: 170,
            stored,
            full,
        };
        assert_eq!(message(energy(0, true)).as_deref(), Some(NO_ENERGY_STORED));
        assert_eq!(
            message(energy(170, false)).as_deref(),
            Some(ALL_ENERGY_STORED)
        );
        assert_eq!(message(energy(100, true)).as_deref(), Some(ENERGY_FILLED));
        assert_eq!(message(Taken::Tripped).as_deref(), Some(SELF_DESTRUCT));
        assert_eq!(
            message(Taken::CaptureFailed).as_deref(),
            Some(CAPTURE_FAILED)
        );
        assert_eq!(message(Taken::FleetFull).as_deref(), Some(FLEET_FULL));
        assert_eq!(message(Taken::Escorted).as_deref(), Some(ASSIGNED_ESCORT));
        for silent in [Taken::Captured, Taken::Aborted, Taken::Nothing] {
            assert_eq!(message(silent), None, "{silent:?}");
        }
        assert_eq!(
            [
                NO_CARGO_STORED,
                NO_AMMO_STORED,
                NO_ENERGY_STORED,
                ALL_ENERGY_STORED,
                ENERGY_FILLED,
                SELF_DESTRUCT,
                CAPTURE_FAILED,
                FLEET_FULL,
                ASSIGNED_ESCORT,
                RETAINED_OLD_SHIP,
                LOST_OLD_SHIP,
            ],
            [
                "You couldn't store any of the cargo you plundered from this ship.",
                "You couldn't store any of the ammo you plundered from this ship.",
                "You couldn't store any of the energy you transferred from this ship.",
                "You transferred all of this ship's energy to your reactors and batteries.",
                "You filled your reactors and batteries with energy from this ship.",
                "Oops! You tripped this ship's security self-destruct mechanism.",
                "Your attempt to capture this ship was unsuccessful.",
                "You already have the maximum possible number of escorts.",
                "You assigned this ship to your fleet of escorts.",
                "You retained your old ship as an escort.",
                "You were unable to retain your old ship as an escort.",
            ]
        );
        assert_eq!(assigned_message(Assigned::Escort), ASSIGNED_ESCORT);
        assert_eq!(assigned_message(Assigned::MyShip), RETAINED_OLD_SHIP);
        assert_eq!(assigned_message(Assigned::Abandoned), LOST_OLD_SHIP);
    }

    /// [`beside`], boarded by [`Sure`] and captured, awaiting its
    /// assignment.
    fn captured() -> View {
        captured_in(boardable())
    }

    /// [`captured`], from `catalog`, a [`boardable`] one.
    fn captured_in(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&placed(750, 750, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(DisablesTheStrong))
            .with_boarding_rule(Rc::new(Sure));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!(boarding.odds, 75, "by the rule given");
        assert_eq!(view.plunder(Take::Capture), Taken::Captured);
        assert_eq!(view.message(), None, "the assignment dialog says it");
        view
    }

    #[test]
    fn the_target_panel_is_redrawn_once_the_plunder_dialog_closes() {
        let mut view = beside(750, 750, 0, true);
        view.tick(TICK);
        while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        let mut view = view.with_disable_rule(Rc::new(DisablesTheStrong));
        view.tick(TICK);
        assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
        assert_ne!(status_shown(&view), status_now(&view), "held");
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded").view;
        assert_eq!(
            view.plunder(Take::Credits),
            Taken::Credits(boarding.credits)
        );
        assert_eq!(*view.target_shown(), view.target_now(), "redrawn at once");
        assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    }

    #[test]
    fn use_as_escort_says_so() {
        let mut view = captured();
        view.tick(TICK / 2);
        let alpha = view.alpha();
        assert!(alpha > 0.0, "between steps");
        assert_eq!(view.assign(Assignment::Escort), Some(Assigned::Escort));
        assert_eq!(view.alpha(), alpha, "the old ship flies on");
        let pilot = view.pilot().expect("a pilot");
        assert_eq!(pilot.escorts().len(), 1);
        assert!(view.take_save_due());
        assert_lasts(&mut view, ASSIGNED_ESCORT, 240);
    }

    #[test]
    fn use_as_my_ship_flies_and_draws_the_captured_ship() {
        let mut view = captured();
        view.tick(TICK / 2);
        assert!(view.alpha() > 0.0, "between steps");
        let mut asked = view.catalog().sheets_asked.borrow().clone();
        assert_eq!(view.assign(Assignment::MyShip), Some(Assigned::MyShip));
        assert_eq!(view.alpha(), 0.0, "drawn where it is");
        assert_eq!(view.message(), Some(RETAINED_OLD_SHIP));
        assert_eq!(view.session().expect("flying").ship(), ShipId(129));
        asked.push(ShipId(129));
        assert_eq!(*view.catalog().sheets_asked.borrow(), asked, "read once");
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            asked,
            "not again once settled"
        );
        let drawn_player = sprites(&drawn(&view))
            .into_iter()
            .filter(|&(_, center)| center == VIEW_CENTER)
            .map(|(image, _)| image)
            .collect::<Vec<_>>();
        assert_eq!(
            drawn_player,
            [ImageKey::sprite(2001, 0), ImageKey::sprite(2201, 0)],
            "ship 129's sheet, and its lights"
        );
        assert_eq!(view.assign(Assignment::MyShip), None, "nothing awaits");
        assert_lasts(&mut view, RETAINED_OLD_SHIP, 360);
    }

    #[test]
    fn the_plunder_taken_is_named_as_the_session_names_it() {
        let mut catalog = boardable();
        catalog.fields.holds = 20;
        catalog.dudes[0].1.booty = 0x0041;
        catalog.commodities = CommodityStrings {
            names: vec!["Food".to_owned()],
            name_patches: Default::default(),
            base_prices: vec!["75".to_owned()],
            price_patches: Default::default(),
        };
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.fields.holds = 20;
            }
        }
        catalog.weapons = vec![nova_sim::WeaponRecord {
            ammo_type: 12,
            ..gun(ROCKET, 0x0002, -1, -1)
        }];
        let rockets = |id, ammo| nova_sim::HullRecord {
            strength: 5,
            weapons: vec![nova_sim::StockWeapon {
                weapon: ROCKET,
                count: 1,
                ammo,
            }],
            ..hull_of(id, &[])
        };
        catalog.hulls = vec![rockets(128, 0), rockets(129, 2)];
        catalog.hulls[0].strength = 0;
        catalog.outfits = vec![
            holding(250, ROCKET),
            OutfitRecord {
                id: OutfitId(310),
                name: "Rockets".to_owned(),
                short_name: "Rockets".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 10,
                flags: 0,
                cost: 100,
                mods: [
                    (nova_sim::combat::armament::MOD_AMMO, 140),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                item_class: 0,
                lc_name: "rocket".to_owned(),
                lc_plural: "rockets".to_owned(),
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
            },
        ];
        let (_, chance) = scripted(&placed(750, 750, 0));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_disable_rule(Rc::new(DisablesTheStrong));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        let boarding = view.take_boarding().expect("boarded");
        assert_eq!(boarding.view.cargo, Some((Good::Commodity(0), 19)));
        assert_eq!(boarding.view.ammo, Some((OutfitId(310), 2)));
        let named = |shown: &PlunderShown| (shown.good.clone(), shown.outfit.clone());
        assert_eq!(
            named(&boarding),
            (Some("Food".to_owned()), Some("Rockets".to_owned())),
            "the dialog names them as the messages do"
        );
        assert_eq!(view.plunder_shown().as_ref(), Some(&boarding));
        view.plunder(Take::Cargo);
        assert_eq!(
            view.message(),
            Some("You salvaged 19 tons of Food from this ship.")
        );
        let shown = view.plunder_shown().expect("still under way");
        assert_eq!(named(&shown), (None, Some("Rockets".to_owned())));
        view.plunder(Take::Ammo);
        assert_eq!(
            view.message(),
            Some("You salvaged 2 Rockets from this ship.")
        );
        let shown = view.plunder_shown().expect("still under way");
        assert_eq!(named(&shown), (None, None), "nothing left to name");
    }

    #[test]
    fn the_help_line_names_board() {
        assert!(HELP.contains("B: board"), "{HELP}");
    }

    // Hailing.

    use nova_sim::{Haggle, HailOptions, HailRefusal, Help};

    /// [`boardable`]'s ship comm strings: "c<n>" for each, with "Channel
    /// open." in every variant of the first group.
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

    /// [`beside`] with the ship comm strings.
    fn hailable(intact: bool) -> View {
        let mut view = beside(750, 750, 0, intact);
        view.catalog.strings = vec![(3000, comm_strings())];
        view
    }

    #[test]
    fn y_on_a_hailable_target_reports_a_hail_once() {
        let mut view = hailable(true);
        assert_eq!(view.take_hail(), None);
        view.input(&key(Key::Up, true));
        tap(&mut view, HAIL_KEY);
        let hail = view.take_hail().expect("hailed");
        assert_eq!(hail.npc, NpcId(0));
        assert_eq!(hail.reply, "Channel open.");
        assert_eq!(view.take_hail(), None, "once");
        assert_eq!(view.hailing().map(|hail| hail.npc), Some(NpcId(0)));
        assert_eq!(view.message(), None);
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
        assert_eq!(HAIL_KEY, Key::Char('y'));
    }

    #[test]
    fn y_on_a_disabled_target_says_no_response() {
        let mut view = hailable(false);
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None);
        assert_lasts(&mut view, NO_RESPONSE, 240);
        assert_eq!(
            hail_refusal_message(HailRefusal::NoResponse),
            Some("No response.")
        );
        assert_eq!(
            hail_refusal_message(HailRefusal::InHyperspace),
            Some("Unable to send hail - target ship is entering hyperspace.")
        );
        assert_eq!(hail_refusal_message(HailRefusal::NoTarget), None);
        let mut alone = flight();
        tap(&mut alone, HAIL_KEY);
        assert_eq!(alone.take_hail(), None);
        assert_eq!(alone.message(), None, "no target: nothing said");
    }

    #[test]
    fn a_hail_dialog_keeps_the_message() {
        let mut view = hailable(true);
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
        view.hang_up();
        assert_lasts(&mut view, MISSION_FAILED, 240);
    }

    #[test]
    fn answers_haggles_and_hanging_up_go_through_the_session() {
        let mut view = hailable(true);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let answered = view.answer(0).expect("open");
        assert_eq!(answered.npc, NpcId(0));
        assert_eq!(view.haggle(Haggle::Accept), None, "no price asked");
        view.hang_up();
        assert_eq!(view.hailing(), None);
        assert_eq!(view.answer(0), None);
    }

    #[test]
    fn hanging_up_a_hail_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
        let mut view = hailable(true);
        ticks(&mut view, 3);
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let stale = RadarScene::default();
        assert_ne!(view.radar_now(), stale);
        view.radar_scene = Some(stale);
        let cadence = view.hud_cadence;
        view.hang_up();
        assert_eq!(view.hailing(), None);
        assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
        assert_eq!(*view.radar_shown(), view.radar_now(), "as _RedrawStatus");
    }

    /// An option that asks a price, saying nothing.
    #[derive(Debug)]
    struct Sell;

    impl nova_sim::HailOption for Sell {
        fn label(&self) -> String {
            "Sell".to_owned()
        }

        fn applies(&self, _hail: &nova_sim::Hail) -> bool {
            true
        }

        fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
            let line = nova_sim::Reply::Line { list: 1, index: 1 };
            nova_sim::Answer {
                ask: Some(nova_sim::Ask {
                    paid: (line, None),
                    declined: (line, None),
                    short: line,
                }),
                ..nova_sim::Answer::default()
            }
        }
    }

    #[test]
    fn a_price_asked_is_haggled_over_through_the_session() {
        let mut view = hailable(true).with_hail_options(HailOptions::empty().with(Rc::new(Sell)));
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        let asked = view.answer(0).expect("open");
        assert!(asked.asking.is_some(), "{asked:?}");
        let settled = view.haggle(Haggle::Accept).expect("open");
        assert_eq!(settled.asking, None, "settled");
    }

    #[test]
    fn the_screens_hail_options_are_listed() {
        let mut view = hailable(true);
        view = view.with_hail_options(HailOptions::empty());
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail().expect("hailed").options, []);
    }

    #[test]
    fn a_helpers_note_names_it_in_the_originals_words() {
        let note = |done| nova_sim::CommNote {
            from: NpcId(2),
            comm_name: "Cruiser".to_owned(),
            done,
        };
        assert_eq!(
            comm_message(&note(Help::Refuel)),
            "Cruiser:  Energy transfer complete."
        );
        assert_eq!(
            comm_message(&note(Help::Repair)),
            "Cruiser:  Repairs complete."
        );
    }

    /// An option the ship answers by flying over to refuel the player.
    #[derive(Debug)]
    struct Refuel;

    impl nova_sim::HailOption for Refuel {
        fn label(&self) -> String {
            "Refuel".to_owned()
        }

        fn applies(&self, _hail: &nova_sim::Hail) -> bool {
            true
        }

        fn press(&self, _hail: &nova_sim::Hail, _chance: &mut dyn Chance) -> nova_sim::Answer {
            nova_sim::Answer {
                deed: Some(nova_sim::Deed::Help(Help::Refuel)),
                ..nova_sim::Answer::default()
            }
        }
    }

    #[test]
    fn a_helpers_note_shows_for_the_common_time() {
        let mut view = hailable(true).with_hail_options(HailOptions::empty().with(Rc::new(Refuel)));
        tap(&mut view, HAIL_KEY);
        view.take_hail().expect("hailed");
        view.answer(0).expect("open");
        view.hang_up();
        assert_eq!(view.message(), None);
        view.tick(TICK);
        let note = view.message().expect("refuelled").to_owned();
        assert!(note.ends_with(":  Energy transfer complete."), "{note}");
        assert_lasts(&mut view, &note, MESSAGE_TICKS);
    }

    #[test]
    fn the_help_line_names_hail() {
        assert!(HELP.contains("Y: hail"), "{HELP}");
    }

    // Escorts.

    use super::super::escorts::{ABSENT_ROW, MENU_AT, MENU_HEIGHT, MENU_WIDTH};
    use nova_sim::{EscortClass, EscortGroup, EscortOrder};

    /// `pilot`, saved and read back with a fleet of `ships`, each full.
    fn with_escorts(pilot: &Pilot, ships: &[i16]) -> Pilot {
        with_fleet(pilot, ships, false)
    }

    /// `pilot`, saved and read back with a fleet of `ships`, each full,
    /// and fighters out of a bay when `carried`.
    fn with_fleet(pilot: &Pilot, ships: &[i16], carried: bool) -> Pilot {
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
        let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
        save["escorts"] = ships
            .iter()
            .map(|ship| {
                serde_json::json!({
                    "ship": ship,
                    "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                    "order": null,
                    "carried": carried,
                    "wage": null,
                    "person": null
                })
            })
            .collect();
        nova_sim::save::decode(&save.to_string()).expect("a pilot")
    }

    /// [`boardable`] with ship 130 a fighter (`EscortType` 0) holding
    /// no fuel, and two of them out of the player's bays; its traffic
    /// ship (NPC 0) placed 100 above the player. A tick has placed the
    /// fighters (NPCs 1 and 2).
    fn fighters_out() -> View {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 0;
                record.fields.fuel = 0;
            }
        }
        let pilot = with_fleet(
            &Pilot::new(&catalog, "Carrier").expect("starts"),
            &[130, 130],
            true,
        );
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert!(view.session().expect("flying").is_escort(NpcId(2)));
        view
    }

    /// Taps `key` with Alt (Option) held.
    fn option(view: &mut View, key_: Key) {
        view.input(&key(Key::Alt, true));
        tap(view, key_);
        view.input(&key(Key::Alt, false));
    }

    #[test]
    fn option_c_sends_the_fighters_home_and_c_alone_back_to_formation() {
        let mut view = fighters_out();
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Dock); 2]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to hangar.")
        );
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None; 2], "C alone recalls");
        tap(&mut view, MENU_KEY);
        tap(&mut view, GROUP_KEYS[1]);
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  Fighters returning to hangar.")
        );
    }

    #[test]
    fn option_c_sends_an_escort_of_its_own_back_to_formation() {
        let mut view = escorted();
        tap(&mut view, ESCORT_HOLD_KEY);
        option(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to formation.")
        );
    }

    #[test]
    fn the_escort_menu_shows_return_to_hangar_for_fighters_returning() {
        let mut view = fighters_out();
        option(&mut view, ESCORT_RECALL_KEY);
        tap(&mut view, MENU_KEY);
        assert!(
            texts(&drawn(&view)).contains(&"Return to Hangar".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    #[test]
    fn fighters_abandoned_in_a_jump_are_told_on_arrival() {
        let mut view = fighters_out();
        jump_to_alpha(&mut view);
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Alpha Centauri system on June 24, 1177 NC.  (Two fighters abandoned)"
            )
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
    }

    /// [`boardable`] with ship 130, a warship escort (`EscortType` 2,
    /// `InherentAI` 3), whose sheet cannot be read, and the pilot's fleet
    /// one of them; its traffic ship (NPC 0) placed 100 above the player;
    /// all idle. A tick has placed the escort (NPC 1).
    fn escorted() -> View {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 2;
                record.inherent_ai = 3;
            }
        }
        let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(session.is_escort(NpcId(1)), "{:?}", session.npcs());
        view
    }

    #[test]
    fn the_players_escort_is_bracketed_green() {
        let mut view = escorted();
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
    }

    #[test]
    fn the_players_fighter_is_bracketed_green() {
        let mut view = fighters_out();
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(bracket_icons(&drawn(&view)), [10016, 10017, 10018, 10019]);
    }

    /// Where the target panel's label `label` is drawn, if it is: its
    /// top-left.
    fn label_drawn(view: &View, label: &str) -> Option<Point> {
        drawn(view).iter().find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if text == label => Some(*origin),
            _ => None,
        })
    }

    /// Where `view` sets a label `label` against the target panel's
    /// bottom right, measured by `MonoMetrics`: 7 in from the right,
    /// on the baseline 6 above the bottom.
    fn label_place(view: &View, label: &str) -> Point {
        let bar = view.status_bar().expect("a status bar");
        let layout = &bar.layout;
        let area = layout.targ.offset(hud::bar_origin(bar));
        let width = crate::text::TextMetrics::width(
            &crate::text::fixture::MonoMetrics,
            layout.font,
            layout.font_size,
            label,
        );
        at(
            area.max.x - 7.0 - width,
            area.max.y - 6.0 - layout.font_size,
        )
    }

    #[test]
    fn the_target_panel_labels_the_players_escort_and_fighter_and_omits_a_missing_code() {
        let mono = || Rc::new(crate::text::fixture::MonoMetrics);
        let mut view = escorted().with_metrics(mono());
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(
            label_drawn(&view, target::ESCORT),
            Some(label_place(&view, target::ESCORT)),
            "Escort"
        );
        let mut view = fighters_out().with_metrics(mono());
        option(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)));
        assert_eq!(
            label_drawn(&view, target::FIGHTER),
            Some(label_place(&view, target::FIGHTER)),
            "Fighter"
        );
        assert_eq!(label_drawn(&view, target::ESCORT), None);
        // The traffic ship's government, 140, has the code "Fed.".
        let mut view = fighting(armed(1, &[]), &placed(750, 650, 180)).with_metrics(mono());
        tap(&mut view, TARGET_KEY);
        assert_eq!(
            label_drawn(&view, "Fed."),
            Some(label_place(&view, "Fed.")),
            "with a code"
        );
        let uncoded = FakeCatalog {
            codes: Vec::new(),
            ..armed(1, &[])
        };
        let mut view = fighting(uncoded, &placed(750, 650, 180)).with_metrics(mono());
        tap(&mut view, TARGET_KEY);
        assert!(target_of(&view).is_some());
        let bottom = label_place(&view, "").y;
        let at_bottom: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if origin.y == bottom => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(at_bottom, ["Shield: ", "100%"], "no code drawn");
    }

    fn orders(view: &View) -> Vec<Option<EscortOrder>> {
        view.pilot()
            .expect("flying")
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect()
    }

    #[test]
    fn f_d_v_and_c_command_every_escort_and_say_what_changed() {
        let mut view = escorted();
        tap(&mut view, TARGET_KEY);
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(NpcId(0)),
            "Tab skips the escort"
        );
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Attack)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships attacking target.")
        );
        let escort = &view.session().expect("flying").npcs()[1];
        assert_eq!(escort.target, Some(ShipRef::Npc(NpcId(0))));
        tap(&mut view, ESCORT_DEFEND_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Defend)]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships defending.")
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(orders(&view), [Some(EscortOrder::Hold)]);
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(orders(&view), [None]);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships returning to formation.")
        );
        assert_eq!(
            [
                ESCORT_ATTACK_KEY,
                ESCORT_DEFEND_KEY,
                ESCORT_HOLD_KEY,
                ESCORT_RECALL_KEY
            ],
            ['f', 'd', 'v', 'c'].map(Key::Char)
        );
    }

    #[test]
    fn a_command_that_changes_nothing_says_nothing() {
        let mut view = escorted();
        tap(&mut view, ESCORT_RECALL_KEY);
        assert_eq!(view.message(), None, "already in formation");
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_lasts(
            &mut view,
            "New escort orders assigned:  All ships holding position.",
            ESCORT_TICKS,
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(view.message(), None, "held already");
    }

    #[test]
    fn attack_with_no_target_will_attack() {
        let mut view = escorted();
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  All ships will attack.")
        );
    }

    #[test]
    fn with_the_escort_menu_open_1_to_4_pick_groups_not_stellars() {
        let mut view = escorted();
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        tap(&mut view, Key::Char('2'));
        assert_eq!(nav_target(&view), None, "the menu took it");
        tap(&mut view, Key::Function(2));
        assert_eq!(nav_target(&view), Some(StellarId(129)), "F2 never does");
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open());
        tap(&mut view, Key::Char('1'));
        assert_eq!(nav_target(&view), Some(StellarId(128)), "shut: a stellar");
        assert_eq!(view.escort_menu().group(), EscortGroup::All);
    }

    #[test]
    fn return_clears_the_message_and_closes_the_menu() {
        let mut view = escorted();
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept by the menu");
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), None, "acknowledged");
        let mut view = flight();
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_CLOSE_KEY);
        assert_eq!(view.message(), None, "with the menu shut too");
    }

    #[test]
    fn opening_the_escort_menu_clears_no_escorts_only() {
        let mut view = escorted();
        view.say_for(NO_ESCORTS, ESCORT_TICKS);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), None, "#51 cleared");
        view.say_for(NO_ESCORTS, ESCORT_TICKS);
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), Some(NO_ESCORTS), "closing does not clear");
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
    }

    #[test]
    fn e_opens_the_menu_and_its_keys_pick_the_group_commanded() {
        let mut view = escorted();
        tap(&mut view, GROUP_KEYS[3]);
        assert!(!view.escort_menu().is_open(), "1-5 leave a shut menu alone");
        assert_eq!(view.escort_menu().group(), EscortGroup::All);
        tap(&mut view, MENU_KEY);
        assert!(view.escort_menu().is_open());
        tap(&mut view, GROUP_KEYS[1]);
        assert_eq!(view.escort_menu().group(), EscortGroup::All, "no fighters");
        tap(&mut view, GROUP_KEYS[3]);
        assert_eq!(
            view.escort_menu().group(),
            EscortGroup::Class(EscortClass::Warship)
        );
        tap(&mut view, ESCORT_HOLD_KEY);
        assert_eq!(
            view.message(),
            Some("New escort orders assigned:  Warships holding position.")
        );
        tap(&mut view, MENU_CLOSE_KEY);
        assert!(!view.escort_menu().is_open(), "Return closes it");
        tap(&mut view, MENU_KEY);
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open(), "E again closes it");
        assert_eq!(MENU_CLOSE_KEY, Key::Enter);
    }

    #[test]
    fn the_menu_closes_8_seconds_after_its_last_command() {
        let mut view = escorted();
        tap(&mut view, MENU_KEY);
        view.tick(Duration::from_secs(6));
        tap(&mut view, ESCORT_HOLD_KEY);
        view.tick(Duration::from_secs(6));
        assert!(view.escort_menu().is_open(), "the command kept it open");
        view.tick(Duration::from_secs(3));
        assert!(!view.escort_menu().is_open());
    }

    #[test]
    fn e_with_no_escorts_says_so() {
        let mut view = flight();
        tap(&mut view, MENU_KEY);
        assert!(!view.escort_menu().is_open());
        assert_eq!(view.message(), Some("You don't have any escorts."));
        tap(&mut view, ESCORT_ATTACK_KEY);
        assert_eq!(
            view.message(),
            Some("You don't have any escorts."),
            "no command"
        );
        assert_lasts(&mut view, NO_ESCORTS, 250);
    }

    #[test]
    fn option_tab_targets_the_escorts() {
        let mut view = escorted();
        view.input(&key(Key::Alt, true));
        tap(&mut view, TARGET_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(
            view.session().expect("flying").target().map(|npc| npc.id),
            Some(NpcId(1))
        );
    }

    #[test]
    fn the_escorts_sheets_are_read_with_the_traffics() {
        let view = escorted();
        let asked = view.catalog().sheets_asked.borrow();
        assert!(asked.contains(&ShipId(130)), "{asked:?}");
    }

    #[test]
    fn the_open_menu_is_drawn_over_the_flight_under_the_hud() {
        let mut view = escorted().with_overlays(TITLED);
        let mut shut = DrawList::new();
        view.draw(&mut shut);
        tap(&mut view, MENU_KEY);
        let mut open = DrawList::new();
        view.draw(&mut open);
        let commands: Vec<_> = open.iter().collect();
        let title = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Escort Commands"))
            .expect("the menu");
        let frame = commands
            .iter()
            .position(|command| {
                matches!(command, DrawCommand::Line { color, .. } if *color == MENU_COLORS.border)
            })
            .expect("its border");
        let help = commands
            .iter()
            .position(|command| **command == overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM))
            .expect("the help line");
        let player = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Sprite { image, .. } if *image == ImageKey::sprite(2000, 0)))
            .expect("the player");
        assert!(
            player < frame && frame < title && title < help,
            "{player} {frame} {title} {help}"
        );
        assert_eq!(
            open.len() - shut.len(),
            1 + 4 + 1 + 5 + 1 + 1,
            "fill, border, title, rows, bar, order"
        );
        assert!(commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, .. } if text == "2) Fighters" && *color == ABSENT_ROW
        )));
        assert!(commands.iter().any(|command| matches!(
            command,
            DrawCommand::Line { from, .. } if from.x == MENU_AT.x && from.y == MENU_AT.y + MENU_HEIGHT / 2.0
        )), "filled across {MENU_WIDTH}");
    }

    #[test]
    fn the_help_line_names_the_escort_keys() {
        assert!(
            HELP.contains("F/D/V/C: escorts   Alt-C: dock   E: escort menu"),
            "{HELP}"
        );
    }

    // Hiring escorts and paying them.

    use nova_sim::hire::{DEFECTED_ONE, DEFECTED_SOME};
    use nova_sim::{ControlBits, HireRefusal, HireTerms, PayNote};

    #[test]
    fn a_defection_is_said_as_the_original_says_it() {
        assert_eq!(defection_message(1), DEFECTED_ONE);
        assert_eq!(defection_message(2), DEFECTED_SOME);
        assert_eq!(defection_message(5), DEFECTED_SOME);
        assert_eq!(
            pay_notes_message(&[PayNote::Defected(1), PayNote::Defected(3)]),
            vec![DEFECTED_ONE.to_owned(), DEFECTED_SOME.to_owned()]
        );
    }

    /// [`shipbuying`] with its stellar a bar too, and ship 129 (`Cost`
    /// 900) for hire half the days.
    fn hiring() -> FakeCatalog {
        let mut catalog = shipbuying();
        catalog.sites[0].flags |= StellarFlags::BAR;
        for record in &mut catalog.ships {
            if record.id == ShipId(129) {
                record.hire_random = 50;
            }
        }
        catalog
    }

    #[test]
    fn hiring_goes_through_the_session_on_the_flights_chance() {
        let always = Rc::new(RefCell::new(Always::default()));
        let shared: Rc<RefCell<dyn Chance>> = always.clone();
        let mut view = FlightView::new(hiring()).with_chance(SharedChance::new(shared));
        assert_eq!(view.escorts_for_hire(), None, "in flight");
        assert_eq!(view.hire(ShipId(129)), Err(HireRefusal::NoBar));
        land_now(&mut view);
        view.take_save_due();
        let list = view.escorts_for_hire().expect("a bar");
        let row = list.row(ShipId(129)).expect("for hire");
        assert_eq!((row.fee, row.wage), (90, 9));
        assert_eq!(always.borrow().asked, [50]);
        let hired = view.hire(ShipId(129)).expect("hired");
        assert_eq!((hired.fee, hired.wage), (90, 9));
        assert_eq!(view.pilot().map(Pilot::cash), Some(1000 - 90));
        assert_eq!(view.pilot().expect("flying").escorts()[0].wage, Some(9));
        assert!(view.take_save_due());
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..hiring()
        });
        assert_eq!(broken.escorts_for_hire(), None);
        assert_eq!(broken.hire(ShipId(129)), Err(HireRefusal::NoBar));
    }

    // The mission screens.

    use nova_sim::MissionTextKind;
    use nova_sim::catalog::{MissionRecord, MissionShip, MissionTexts};
    use nova_sim::control::{Script as SetScript, Test};

    /// Mission `id`, offered at the mission computer of any stellar on
    /// every visit, to anyone, going nowhere.
    fn errand(id: i16) -> MissionRecord {
        MissionRecord {
            id: MissionId(id),
            avail_stel: -1,
            avail_loc: 0,
            avail_record: 0,
            avail_rating: 0,
            avail_random: 100,
            avail_ship_type: 0,
            avail_bits: Test::default(),
            require: 0,
            flags: 0,
            flags2: 0,
            travel_stel: -1,
            return_stel: -1,
            cargo_type: -1,
            cargo_qty: 0,
            pickup_mode: -1,
            drop_off_mode: -1,
            scan_mask: 0,
            pay_val: 0,
            time_limit: 0,
            disp_weight: 0,
            on_accept: SetScript::default(),
            on_refuse: SetScript::default(),
            on_success: SetScript::default(),
            on_failure: SetScript::default(),
            on_abort: SetScript::default(),
            on_ship_done: SetScript::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: MissionShip::NONE,
            aux: nova_sim::catalog::AuxShip::NONE,
        }
    }

    #[test]
    fn the_mission_screens_go_through_the_session() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130), errand(131)];
        errands.mission_texts = vec![
            MissionTexts {
                brief: Some("Go.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            },
            MissionTexts {
                refuse: Some("Fine.".to_owned()),
                ..MissionTexts::named(MissionId(131), "Walk")
            },
        ];
        let mut view = FlightView::new(errands);
        assert_eq!(
            view.mission_computer(),
            Err(ComputerRefusal::NoneAvailable),
            "in flight"
        );
        assert_eq!(
            view.accept_mission(MissionId(130)),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(view.refuse_mission(MissionId(131)), Err(NotOffered));
        land_now(&mut view);
        view.settle_script();
        let computer = view.mission_computer().expect("opens");
        let names: Vec<_> = computer.rows.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(names, ["Run", "Walk"]);
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        assert_eq!(view.refuse_mission(MissionId(131)), Ok(()));
        let texts: Vec<_> = view
            .take_mission_texts()
            .into_iter()
            .map(|text| (text.kind, text.text))
            .collect();
        assert_eq!(
            texts,
            [
                (MissionTextKind::Brief, "Go.".to_owned()),
                (MissionTextKind::Refuse, "Fine.".to_owned()),
            ]
        );
        assert_eq!(view.take_mission_texts(), [], "taken");
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(
            broken.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
        assert_eq!(
            broken.accept_mission(MissionId(130)),
            Err(AcceptError::NotOffered)
        );
        assert_eq!(broken.refuse_mission(MissionId(130)), Err(NotOffered));
        assert_eq!(broken.take_mission_texts(), []);
    }

    #[test]
    fn closing_a_mission_dialog_or_showing_its_texts_redraws_the_hud() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130)];
        errands.mission_texts = vec![MissionTexts {
            brief: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        let mut view = FlightView::new(errands);
        let stale = RadarScene::default();
        let fresh = view.radar_now();
        assert_ne!(fresh, stale);
        view.radar_scene = Some(stale.clone());
        let _ = view.accept_mission(MissionId(130));
        assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "accepted");
        view.radar_scene = Some(stale.clone());
        let _ = view.refuse_mission(MissionId(130));
        assert_eq!(view.radar_scene.as_ref(), Some(&fresh), "refused");
        view.radar_scene = Some(stale.clone());
        assert_eq!(view.take_mission_texts(), []);
        assert_eq!(view.radar_scene.as_ref(), Some(&stale), "nothing shown");
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        view.radar_scene = Some(stale.clone());
        assert_eq!(view.take_mission_texts().len(), 1);
        assert_eq!(view.radar_scene, Some(view.radar_now()), "the brief shown");
    }

    #[test]
    fn entering_another_venue_through_the_flight_offers_a_suppressed_offer_again() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = (130..146).map(errand).collect();
        errands.missions.push(MissionRecord {
            avail_loc: 1,
            flags: nova_sim::mission::screens::UNREFUSABLE,
            ..errand(150)
        });
        let mut view = FlightView::new(errands);
        land_now(&mut view);
        view.settle_script();
        for id in 130..146 {
            assert!(view.accept_mission(MissionId(id)).is_ok(), "{id}");
        }
        view.enter_offer_venue(1);
        let refused = Some(VenueOffer::Accepted {
            mission: MissionId(150),
            accepted: Err(AcceptError::Refused(
                nova_sim::mission::AcceptRefusal::SlotsFull,
            )),
        });
        assert_eq!(view.next_offer(1), refused);
        assert_eq!(view.next_offer(1), None, "suppressed");
        view.enter_offer_venue(3);
        view.enter_offer_venue(1);
        assert_eq!(view.next_offer(1), refused, "offered again");
    }

    /// A flight from [`trading`]'s Earth, its ship of 10 tons, that
    /// accepted mission 130, carrying 4 tons of food loaded at accept,
    /// `Flags` `flags`, and bought the other 6 tons full of food when
    /// `food`, then took off.
    fn carrying(flags: u16, food: bool) -> View {
        let mut errands = trading();
        errands.missions = vec![MissionRecord {
            cargo_type: 0,
            cargo_qty: 4,
            pickup_mode: 0,
            flags,
            ..errand(130)
        }];
        let mut view = FlightView::new(errands);
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        if food {
            assert_eq!(view.trade(BUY_FOOD), Ok(6), "up to the free space");
        }
        view.take_off().expect("takes off");
        view.settle_script();
        view
    }

    /// Holds Command (and Alt with `option`), taps `k`, and lets go.
    fn command_tap(view: &mut View, k: Key, option: bool) {
        view.input(&key(Key::Command, true));
        if option {
            view.input(&key(Key::Alt, true));
        }
        tap(view, k);
        if option {
            view.input(&key(Key::Alt, false));
        }
        view.input(&key(Key::Command, false));
    }

    fn cargo_aboard(view: &View) -> i64 {
        view.pilot().expect("flying").total_cargo()
    }

    #[test]
    fn command_k_jettisons_the_cargo_failing_the_mission_and_says_so() {
        let mut view = carrying(0, true);
        assert_eq!(cargo_aboard(&view), 10);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(cargo_aboard(&view), 0);
        assert_eq!(view.pilot().expect("flying").missions().iter().count(), 0);
        assert_eq!(view.message(), Some(MISSION_FAILED));
        assert_eq!(JETTISON_KEY, Key::Char('k'));
        assert_eq!(MISSION_FAILED, "Mission failed.");
        assert!(HELP.contains("Cmd-K: jettison"), "{HELP}");
    }

    #[test]
    fn command_option_k_keeps_the_mission_cargo() {
        let mut view = carrying(0, true);
        command_tap(&mut view, JETTISON_KEY, true);
        assert_eq!(cargo_aboard(&view), 4, "the food bought went");
        assert_eq!(view.message(), Some(NON_MISSION_CARGO_JETTISONED));
        assert_eq!(
            NON_MISSION_CARGO_JETTISONED,
            "Non-mission cargo jettisoned."
        );
        let mut view = carrying(0, false);
        command_tap(&mut view, JETTISON_KEY, true);
        assert_eq!(cargo_aboard(&view), 4, "kept");
        assert_eq!(view.message(), None, "nothing else held");
    }

    #[test]
    fn a_hidden_missions_cargo_jettisoned_says_the_cargo_went() {
        let mut view = carrying(0x0400, false);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(cargo_aboard(&view), 0);
        assert_eq!(view.message(), Some(CARGO_JETTISONED));
        assert_eq!(CARGO_JETTISONED, "Cargo jettisoned.");
    }

    #[test]
    fn k_alone_jettisons_nothing() {
        let mut view = carrying(0, false);
        tap(&mut view, JETTISON_KEY);
        assert_eq!(cargo_aboard(&view), 4);
        assert_eq!(view.message(), None);
        view.input(&key(Key::Alt, true));
        tap(&mut view, JETTISON_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(cargo_aboard(&view), 4, "Option alone");
        command_tap(&mut view, Key::Char('j'), false);
        assert_eq!(cargo_aboard(&view), 4, "another key with Command");
    }

    #[test]
    fn each_jettison_message_is_worded_and_the_arrivals_are_not_among_them() {
        let view = flight();
        let session = view.session().expect("flying");
        let text = |message| sim_message_text(&message, session);
        assert_eq!(
            text(SimMessage::MissionFailed).as_deref(),
            Some(MISSION_FAILED)
        );
        assert_eq!(
            text(SimMessage::CargoJettisoned).as_deref(),
            Some(CARGO_JETTISONED)
        );
        assert_eq!(
            text(SimMessage::NonMissionCargoJettisoned).as_deref(),
            Some(NON_MISSION_CARGO_JETTISONED)
        );
        assert_eq!(text(SimMessage::Arrived(SystemId(130))), None);
        assert_eq!(text(SimMessage::ExitedHypergate(SystemId(130))), None);
        assert_eq!(text(SimMessage::PassedWormhole(SystemId(130))), None);
        for message in [
            SimMessage::MissionFailed,
            SimMessage::CargoJettisoned,
            SimMessage::NonMissionCargoJettisoned,
            SimMessage::MissionScanned,
            SimMessage::Smuggling {
                scanner: ShipId(129),
                ship_name: None,
                fine: 0,
            },
        ] {
            assert_eq!(arrival_lead(&message), None, "{message:?}");
        }
    }

    #[test]
    fn a_mission_ship_boarded_on_the_assist_path_says_its_line() {
        assert_eq!(TARGET_BOARDED, "Target ship has been boarded.");
        assert_eq!(
            refuelled_message("Freighter", "Kestrel"),
            "Freighter:  Energy transfer complete, Kestrel."
        );
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            mission_board_message(MissionBoardLine::Boarded, session),
            TARGET_BOARDED
        );
    }

    /// `_HandlePlayerBoardAttempt` @0x653fb-0x65543: #106 ("You
    /// retrieved ", its own trailing space) and a space, #107 and a space
    /// unless the name starts with `*`, the name without it, a space,
    /// #108.
    #[test]
    fn a_pickup_ships_line_names_its_cargo_as_the_original_spaces_it() {
        assert_eq!(THE, "the");
        assert_eq!(
            retrieved_message("Food"),
            "You retrieved  the Food from this ship."
        );
        assert_eq!(
            retrieved_message("*Medical Supplies"),
            "You retrieved  Medical Supplies from this ship."
        );
        assert_eq!(BOARD_LINE_TICKS, 250, "0xfa frames @0x65538");
    }

    #[test]
    fn a_pickup_ships_line_reads_the_cargo_name_from_str_4001() {
        let strings = FakeCatalog {
            strings: vec![(4001, vec!["Food".to_owned(), "*Passengers".to_owned()])],
            ..catalog()
        };
        let view = flight().with_strings(Rc::new(strings));
        let session = view.session().expect("flying");
        let said = |cargo| mission_board_message(MissionBoardLine::Retrieved { cargo }, session);
        assert_eq!(said(Some(0)), "You retrieved  the Food from this ship.");
        assert_eq!(said(Some(1)), "You retrieved  Passengers from this ship.");
        assert_eq!(said(Some(2)), "You retrieved  the  from this ship.");
        assert_eq!(said(None), "You retrieved  the  from this ship.");
        assert_eq!(said(Some(256)), "You retrieved  the  from this ship.");
    }

    #[test]
    fn a_mission_ship_lost_says_str_2002_284() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::MissionShipLost, session).as_deref(),
            Some(MISSION_FAILED)
        );
        assert_eq!(arrival_lead(&SimMessage::MissionShipLost), None);
    }

    #[test]
    fn the_deadline_message_is_str_2002_285() {
        let view = flight();
        let session = view.session().expect("flying");
        assert_eq!(
            sim_message_text(&SimMessage::DeadlineMissed, session).as_deref(),
            Some(TIME_LIMIT_EXCEEDED)
        );
        assert_eq!(arrival_lead(&SimMessage::DeadlineMissed), None);
        assert_eq!(TIME_LIMIT_EXCEEDED, "Time limit exceeded - mission failed.");
    }

    /// Inspects the player on its first decision; then it has looked the
    /// player over, and idles.
    #[derive(Debug, Default)]
    struct Inspector(std::cell::Cell<bool>);

    impl Behaviour for Inspector {
        fn decide(
            &self,
            _npc: &nova_sim::Npc,
            _around: &nova_sim::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            if self.0.replace(true) {
                Goal::Idle
            } else {
                Goal::Inspect(ShipRef::Player)
            }
        }
    }

    /// A flight in Sol with 1200 credits, its ship named Kestrel, with
    /// mission 130's 4 tons
    /// of food (`ScanMask` 0x0004, `Flags` `flags`) aboard in slot 0, and
    /// one interceptor (ship 129) of a government scanning for 0x0004 and
    /// fining `fine`, placed on the ship, that inspects it, its scan
    /// drawing 0.
    fn smuggling(flags: u16, fine: i16) -> View {
        let mut catalog = trafficked(&[130], 1, 129, 4);
        catalog.character = Ok(CharacterStart {
            cash: 1200,
            ..catalog.character.expect("a chär")
        });
        catalog.dudes[0].1.govt = Some(GovtId(140));
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0,
            flags2: 0,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties {
                smuggle: 5,
                ..nova_sim::Penalties::default()
            },
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0x0004,
            scan_fine: fine,
        }];
        catalog.missions = vec![MissionRecord {
            cargo_type: 0,
            cargo_qty: 4,
            pickup_mode: 0,
            scan_mask: 0x0004,
            flags,
            ..errand(130)
        }];
        let pilot = Pilot::new(&catalog, "Ada").expect("a pilot");
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
        save["ship_name"] = serde_json::json!("Kestrel");
        save["missions"] = serde_json::json!([{
            "slot": 0,
            "mission": 130,
            "destination": null,
            "return": null,
            "cargo_type": 0,
            "cargo_tons": 4,
            "pay": 0,
            "days_left": null,
            "deadline": null,
            "reached": false,
            "goals_met": true,
            "aboard": true,
            "failed": null,
            "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0},
            "aux": {"countdown": 0, "arrived": 0, "left": 0},
        }]);
        let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
        let mut draws = placed(750, 750, 0).to_vec();
        draws.push(0);
        let (_, chance) = scripted(&draws);
        FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Inspector::default()))
    }

    #[test]
    fn a_scan_that_fails_a_mission_says_so_on_the_message_line() {
        let mut view = smuggling(0x0020, 500);
        for _ in 0..3 {
            view.tick(TICK);
        }
        let pilot = view.pilot().expect("flying");
        assert_eq!(pilot.missions().iter().count(), 0, "failed at once");
        assert_lasts(&mut view, MISSION_SCANNED, 400);
        assert_eq!(
            MISSION_SCANNED,
            "Your ship has been scanned - mission failed."
        );
    }

    #[test]
    fn a_scan_that_finds_smuggling_names_the_scanner_and_the_ship_and_the_fine() {
        let mut view = smuggling(0, 500);
        assert_eq!(view.pilot().expect("flying").cash(), 1200);
        for _ in 0..3 {
            view.tick(TICK);
        }
        assert_eq!(view.pilot().expect("flying").cash(), 700);
        assert_eq!(
            view.message(),
            Some(
                "Ship 129:  Kestrel, your attempt to smuggle illegal cargo through this \
                 system has been detected. You have been fined 500 credits."
            )
        );
    }

    #[test]
    fn the_smuggling_notice_reads_the_fine_in_the_originals_words() {
        assert_eq!(
            smuggling_message("Viper", "Kestrel", 0),
            "Viper:  Kestrel, your attempt to smuggle illegal cargo through this system has \
             been detected."
        );
        assert_eq!(
            smuggling_message("Viper", "Kestrel", 1),
            "Viper:  Kestrel, your attempt to smuggle illegal cargo through this system has \
             been detected. You have been fined 1 credit."
        );
        assert_eq!(
            smuggling_message("Viper", "Kestrel", 12_300),
            "Viper:  Kestrel, your attempt to smuggle illegal cargo through this system has \
             been detected. You have been fined 12,300 credits."
        );
    }

    /// The found mission's drawn ship name names the scanner when it has
    /// one (`_ScanPlayer` @0x7e7ce); otherwise its ship class does.
    #[test]
    fn the_smuggling_notice_names_the_scanner_by_the_found_missions_ship_name() {
        let view = flight();
        let session = view.session().expect("flying");
        let ship = session.pilot().ship_name().unwrap_or_default();
        let text = |ship_name: Option<&str>| {
            sim_message_text(
                &SimMessage::Smuggling {
                    scanner: ShipId(128),
                    ship_name: ship_name.map(str::to_owned),
                    fine: 500,
                },
                session,
            )
        };
        assert_eq!(
            text(Some("Rover")),
            Some(smuggling_message("Rover", ship, 500))
        );
        assert_eq!(
            text(None),
            Some(smuggling_message(
                session.ship_name(ShipId(128)).unwrap_or_default(),
                ship,
                500
            ))
        );
    }

    #[test]
    fn mission_info_and_aborts_go_through_the_session() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![errand(130)];
        let mut view = FlightView::new(errands);
        assert_eq!(view.mission_info().rows, []);
        assert_eq!(view.abort_mission(0), Err(AbortError::NotActive));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        let slots: Vec<_> = view
            .mission_info()
            .rows
            .iter()
            .map(|row| row.slot)
            .collect();
        assert_eq!(slots, [0]);
        assert_eq!(view.abort_mission(0), Ok(()));
        assert_eq!(view.mission_info().rows, []);
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(broken.mission_info().rows, []);
        assert_eq!(broken.abort_mission(0), Err(AbortError::NotActive));
    }

    #[test]
    fn the_bars_offers_go_through_the_session_on_the_flights_chance() {
        let mut errands = catalog();
        errands.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        errands.missions = vec![MissionRecord {
            avail_loc: 1,
            ..errand(141)
        }];
        let (script, chance) = scripted(&[]);
        let mut view = FlightView::new(errands).with_chance(chance);
        assert_eq!(view.next_offer(1), None, "in flight");
        land_now(&mut view);
        view.settle_script();
        script.borrow_mut().asked.clear();
        script.borrow_mut().draws.push_back(7);
        view.enter_offer_venue(1);
        let Some(VenueOffer::Prompt(prompt)) = view.next_offer(1) else {
            panic!("an offer");
        };
        assert_eq!(prompt.mission, MissionId(141));
        assert_eq!(view.offer_gap(), nova_sim::mission::screens::mac_ticks(37));
        assert_eq!(script.borrow().asked, [30]);
        let mut broken = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        broken.enter_offer_venue(1);
        assert_eq!(broken.next_offer(1), None);
    }

    /// `pilot`, saved and read back with `escorts`, each a ship, whether
    /// it is carried and its wage, each full.
    fn with_paid_fleet(pilot: &Pilot, escorts: &[(i16, bool, Option<i64>)]) -> Pilot {
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(pilot)).expect("JSON");
        let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
        save["escorts"] = escorts
            .iter()
            .map(|&(ship, carried, wage)| {
                serde_json::json!({
                    "ship": ship,
                    "reserves": {"shield": gauge(40.0), "armor": gauge(60.0), "fuel": gauge(250.0)},
                    "order": null,
                    "carried": carried,
                    "wage": wage,
                    "person": null
                })
            })
            .collect();
        nova_sim::save::decode(&save.to_string()).expect("a pilot")
    }

    /// [`boardable`] with ship 130 of `Cost` 10,000 (a wage of 100), and
    /// a pilot holding `cash` whose fleet is one hired ship 130, docked
    /// at stellar 128 when `docked`; its flight, its first tick done.
    fn paying(cash: i64, docked: bool) -> View {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.cost = 10_000;
            }
        }
        let mut pilot = with_paid_fleet(
            &Pilot::new(&catalog, "Payer").expect("starts"),
            &[(130, false, Some(100))],
        );
        pilot.set_cash(cash);
        if docked {
            let text =
                nova_sim::save::encode(&pilot).replace("\"stellar\": null", "\"stellar\": 128");
            pilot = nova_sim::save::decode(&text).expect("a pilot");
        }
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    #[test]
    fn an_escort_left_unpaid_by_a_jump_defects_and_the_flight_says_so() {
        let mut view = paying(50, false);
        assert_eq!(view.pilot().expect("flying").escorts().len(), 1);
        jump_to_alpha(&mut view);
        let said = format!(
            "{}  {DEFECTED_ONE}",
            "Jumping into the Alpha Centauri system on June 24, 1177 NC."
        );
        assert_eq!(view.message(), Some(said.as_str()), "after the arrival");
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        let mut paid = paying(100, false);
        jump_to_alpha(&mut paid);
        assert_eq!(
            paid.message(),
            Some("Jumping into the Alpha Centauri system on June 24, 1177 NC."),
            "the arrival alone"
        );
        assert_eq!(paid.pilot().map(Pilot::cash), Some(0));
    }

    #[test]
    fn by_the_engine_an_escort_left_unpaid_at_take_off_defects_and_the_flight_says_so() {
        let mut view = paying(50, true);
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.message(), Some(DEFECTED_ONE));
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        let mut kept = paying(50, true)
            .with_rules(Rulebook::default().with_override(RuleKey::TakeOffPay, RuleSource::Bible));
        kept.take_off().expect("took off");
        assert_eq!(kept.message(), None);
        assert_eq!(kept.pilot().expect("flying").escorts().len(), 1);
    }

    #[test]
    fn fighters_abandoned_and_escorts_defected_on_one_arrival_are_said_together() {
        let mut catalog = boardable();
        for record in &mut catalog.ships {
            if record.id == ShipId(130) {
                record.escort_type = 0;
                record.fields.fuel = 0;
            }
        }
        let pilot = with_paid_fleet(
            &Pilot::new(&catalog, "Carrier").expect("starts"),
            &[
                (130, true, None),
                (130, true, None),
                (129, false, Some(1500)),
            ],
        );
        let (_, chance) = scripted(&placed(750, 650, 0));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        jump_to_alpha(&mut view);
        assert_eq!(
            view.message(),
            Some(
                "Jumping into the Alpha Centauri system on June 24, 1177 NC.  (Two fighters abandoned)  Due to lack of pay, one of your escorts has defected."
            )
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
    }

    /// Terms of a fee of 7 and a wage of 3 for every ship.
    #[derive(Debug)]
    struct Sevens;

    impl HireTerms for Sevens {
        fn fee(&self, _ship: &ShipRecord, _site: &LandingSite) -> i64 {
            7
        }

        fn charge(&self, _ship: &ShipRecord, _site: &LandingSite, _cash: i64) -> i64 {
            7
        }

        fn wage(&self, _ship: &ShipRecord) -> i64 {
            3
        }
    }

    /// Control bits where nothing holds.
    #[derive(Debug)]
    struct Nothing;

    impl ControlBits for Nothing {
        fn allows(&self, _test: &nova_sim::TestExpr, _pilot: &dyn nova_sim::PilotFacts) -> bool {
            false
        }
    }

    #[test]
    fn the_hire_ports_reach_the_session() {
        let always: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        let mut view = FlightView::new(hiring())
            .with_chance(SharedChance::new(always))
            .with_hire_terms(Rc::new(Sevens))
            .with_control_bits(Rc::new(Nothing));
        land_now(&mut view);
        let list = view.escorts_for_hire().expect("a bar");
        let row = list.row(ShipId(129)).expect("listed: no Flags3 hides it");
        assert_eq!((row.fee, row.wage), (7, 3));
        assert_eq!(row.hire, Err(HireRefusal::NotForHire));
    }

    // Persons.

    /// [`trafficked`] with no traffic of its own but a Person slot at
    /// 100 %, "Ace" (`përs` 600, subtitle "Top Gun"), flying ship 129 and
    /// saying `STR#` 7101 #1, "<OSN>: Prepare to die, <PN>!", now and
    /// then.
    fn peopled() -> FakeCatalog {
        let mut catalog = trafficked(&[130], 0, 129, 3);
        catalog.traffic[0].1.persons[0] = (Some(nova_sim::PersonId(600)), 100);
        catalog.persons = vec![nova_sim::PersonRecord {
            id: nova_sim::PersonId(600),
            name: "Ace".to_owned(),
            link_syst: -2,
            govt: None,
            ai_type: 3,
            aggress: 2,
            coward: 0,
            ship: Some(ShipId(129)),
            weapons: Vec::new(),
            credits: 0,
            shield_mod: 0,
            hail_pict: None,
            comm_quote: -1,
            hail_quote: 1,
            link_mission: None,
            flags: 0,
            active_on: nova_sim::Test::default(),
            subtitle: "Top Gun".to_owned(),
            flags2: 0,
            grant_class: 0,
            grant_count: 0,
            grant_prob: 0,
            mission_ship: None,
        }];
        catalog.strings = vec![(7101, vec!["<OSN>: Prepare to die, <PN>!".to_owned()])];
        catalog
    }

    /// The draws that list Ace and place it 100 above the player, then
    /// the first quote draw landing on 0.
    const ACE: [u32; 5] = [99, 750, 650, 0, 0];

    #[test]
    fn a_person_targeted_shows_its_name_and_subtitle_on_the_panel() {
        let (_, chance) = scripted(&ACE);
        let mut view = FlightView::new(peopled())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, Key::Tab);
        let texts: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Ace"), "{texts:?}");
        assert!(texts.iter().any(|text| text == "Top Gun"), "{texts:?}");
    }

    /// A flight from Sol after accepting mission 140, whose one special
    /// ship, a rescue held disabled, is set up on Earth under the player
    /// at take-off, and which carries 10 tons of food (`STR#` 4001 #1),
    /// picked up by boarding it; the player's ship holds 10 tons. With
    /// `full`, mission 141, of 10 tons loaded at its accept, fills them
    /// first. The ship is targeted.
    fn rescuing(full: bool) -> View {
        let mut hunted = trafficked(&[130], 0, 129, 3);
        hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        hunted.sol_navs = vec![Some((0, 0))];
        hunted.fields.holds = 10;
        for record in &mut hunted.ships {
            record.crew = 3;
        }
        hunted.strings = vec![(4001, vec!["Food".to_owned()])];
        let cargo = |id, pickup_mode| MissionRecord {
            cargo_type: 0,
            cargo_qty: 10,
            pickup_mode,
            ..errand(id)
        };
        hunted.missions = vec![
            MissionRecord {
                ships: MissionShip {
                    count: 1,
                    goal: 5,
                    syst: -1,
                    dude: Some(nova_sim::DudeId(128)),
                    start: -1,
                    ..MissionShip::NONE
                },
                ..cargo(140, 2)
            },
            cargo(141, 0),
        ];
        hunted.mission_texts = vec![
            MissionTexts::named(MissionId(140), "Rescue"),
            MissionTexts::named(MissionId(141), "Haul"),
        ];
        let strings = FakeCatalog {
            strings: hunted.strings.clone(),
            ..catalog()
        };
        let mut view = FlightView::new(hunted)
            .with_strings(Rc::new(strings))
            .with_behaviour(Rc::new(Still));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
        if full {
            assert_eq!(view.accept_mission(MissionId(141)), Ok(1));
        }
        view.settle_script();
        assert!(view.take_off().is_some());
        view.settle_script();
        view.tick(TICK);
        let _ = view.take_mission_texts();
        tap(&mut view, TARGET_KEY);
        let session = view.session().expect("flying");
        assert!(session.target().is_some_and(|npc| npc.mission.is_some()));
        view
    }

    #[test]
    fn b_on_a_pickup_ship_says_the_cargo_retrieved_for_250_frames() {
        let mut view = rescuing(false);
        tap(&mut view, BOARD_KEY);
        let said = "You retrieved  the Food from this ship.";
        assert_eq!(view.message(), Some(said));
        assert_eq!(view.take_boarding(), None, "no plunder");
        assert_eq!(view.take_mission_texts(), [], "no dialog");
        assert_lasts(&mut view, said, BOARD_LINE_TICKS);
    }

    /// The refused pickup's dialog (`_DoMissionCargoPickup` @0x982ff) is
    /// queued as the B press returns, so the router shows it over flight
    /// as it settles that input, and the comm line says #130.
    #[test]
    fn b_on_a_pickup_ship_without_room_shows_the_dialog_at_once_and_says_cant_board() {
        let mut view = rescuing(true);
        tap(&mut view, BOARD_KEY);
        assert_eq!(view.message(), Some(CANT_BOARD));
        let texts: Vec<(MissionTextKind, String)> = view
            .take_mission_texts()
            .into_iter()
            .map(|text| (text.kind, text.text))
            .collect();
        assert_eq!(
            texts,
            [(
                MissionTextKind::NoRoom,
                nova_sim::mission::cargo::NO_SPACE_TO_LOAD.to_owned()
            )]
        );
        let session = view.session().expect("flying");
        assert!(!session.npcs()[0].boarded);
    }

    #[test]
    fn a_mission_ship_targeted_shows_its_missions_name_and_subtitle_and_no_tag() {
        let mut hunted = trafficked(&[130], 0, 129, 3);
        hunted.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
        hunted.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -1,
                dude: Some(nova_sim::DudeId(128)),
                start: 0,
                name_id: Some(7000),
                subtitle: Some(7001),
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        hunted.mission_texts = vec![MissionTexts::named(MissionId(140), "Hunt")];
        let strings = FakeCatalog {
            strings: vec![
                (7000, vec!["Rover".to_owned()]),
                (7001, vec!["Bounty Hunter".to_owned()]),
            ],
            ..catalog()
        };
        let mut view = FlightView::new(hunted)
            .with_strings(Rc::new(strings))
            .with_behaviour(Rc::new(Still));
        land_now(&mut view);
        view.settle_script();
        assert_eq!(view.accept_mission(MissionId(140)), Ok(0));
        view.settle_script();
        assert!(view.take_off().is_some());
        view.settle_script();
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(
            session.npcs().iter().any(|npc| npc.mission.is_some()),
            "the mission ship is in the system"
        );
        tap(&mut view, Key::Tab);
        let texts: Vec<String> = drawn(&view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Rover"), "{texts:?}");
        assert!(
            texts.iter().any(|text| text == "Bounty Hunter"),
            "{texts:?}"
        );
        assert!(!texts.iter().any(|text| text == "Ship 129"), "{texts:?}");
        assert!(
            !texts
                .iter()
                .any(|text| text.trim() == "Sp" || text.trim() == "Df"),
            "no cheats-only tag: {texts:?}"
        );
    }

    #[test]
    fn a_hail_quote_shows_in_the_message_line_for_420_ticks() {
        let (_, chance) = scripted(&ACE);
        let mut view = FlightView::new(peopled())
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        assert_eq!(
            view.message(),
            Some("Ace: Prepare to die, !"),
            "an unnamed pilot"
        );
        assert_lasts(&mut view, "Ace: Prepare to die, !", 420);
        assert_eq!(HAIL_QUOTE_TICKS, 420);
    }

    #[test]
    fn the_persons_rules_reach_the_session() {
        let rules = nova_sim::NovaPersons {
            odds: RuleSource::Bible,
            ..nova_sim::NovaPersons::default()
        };
        let view = flight().with_person_rules(Rc::new(rules));
        assert_eq!(
            format!("{:?}", view.session().expect("flying").person_rules()),
            format!("{rules:?}")
        );
    }

    /// Runs set expression `text` on `view`'s session.
    fn run_set(view: &mut View, text: &str) {
        let expr = nova_sim::SetExpr::parse(text).expect("parses");
        let session = view.session.as_mut().expect("flying");
        session.run_set(&expr, &mut NeverFires);
    }

    #[test]
    fn a_move_in_flight_lays_out_the_system_moved_to_once_settled() {
        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        view.settle_script();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "on Proxima");
        assert_eq!(view.message(), None);
        view.settle_script();
        assert_eq!(
            *view.catalog().systems_read.borrow(),
            [SystemId(131)],
            "nothing more to settle"
        );
    }

    #[test]
    fn an_h_from_a_set_expression_redraws_the_new_hull_once_settled() {
        let mut view = FlightView::new(shipbuying());
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
        run_set(&mut view, "H129");
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)]
        );
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "ship 129's sheet, and its lights");
        assert_eq!(view.frame(), Some(0), "of 72 rotations");
    }

    #[test]
    fn an_outfits_on_purchase_h_redraws_the_new_hull() {
        let mut catalog = shipbuying();
        catalog.outfits[0].on_purchase = nova_sim::Script::parse("H129");
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.session().map(Session::ship), Ok(ShipId(129)));
        view.settle_script();
        view.settle_script();
        assert_eq!(
            *view.catalog().sheets_asked.borrow(),
            [ShipId(128), ShipId(129)],
            "once"
        );
        view.take_off().expect("took off");
        let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
        assert_eq!(ids, [2001, 2201], "the new hull");
    }

    #[test]
    fn settling_reads_no_sheet_when_the_class_is_unchanged() {
        let mut view = FlightView::new(shipbuying());
        view.settle_script();
        ticks(&mut view, 3);
        run_set(&mut view, "b1");
        view.settle_script();
        view.tick(TICK);
        assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    }

    #[test]
    fn a_move_within_the_system_lays_it_out_afresh() {
        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "N130");
        view.settle_script();
        assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(130)]);
    }

    #[test]
    fn a_move_while_landed_is_laid_out_once_the_ship_takes_off() {
        let mut view = landed_view();
        run_set(&mut view, "M131");
        view.settle_script();
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(*view.catalog().systems_read.borrow(), []);
        assert_eq!(view.take_off(), Some(StellarId(128)));
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert!(
            view.course_map()
                .explored()
                .is_some_and(|explored| explored.contains(&SystemId(131))),
            "the take-off explores"
        );
    }

    /// `view` with `STR#` 25048 holding one message.
    fn told(view: View) -> View {
        let strings = FakeCatalog {
            strings: vec![(25048, vec!["Off you go, <PSN>.".to_owned()])],
            ..catalog()
        };
        view.with_strings(Rc::new(strings))
    }

    #[test]
    fn a_q_in_flight_shows_its_message() {
        let mut view = told(flight());
        run_set(&mut view, "Q25048");
        assert_eq!(view.settle_script(), None);
        assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ScriptMessage)]);
        assert_lasts(&mut view, "Off you go, <PSN>.", SCRIPT_MESSAGE_TICKS);
    }

    #[test]
    fn a_landed_q_takes_off_and_shows_its_message() {
        let mut view = told(landed_view());
        run_set(&mut view, "M131 Q25048");
        assert_eq!(view.settle_script(), Some(StellarId(128)));
        let session = view.session().expect("flying");
        assert_eq!(session.landed(), None);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
        assert_eq!(view.settle_script(), None, "settled");
        assert_lasts(&mut view, "Off you go, <PSN>.", 500);
    }

    #[test]
    fn a_landed_q_shows_the_take_offs_pay_after_its_message() {
        let mut view = told(paying(50, true));
        assert_eq!(view.take_landing(), Some(StellarId(128)));
        run_set(&mut view, "Q25048");
        assert_eq!(view.settle_script(), Some(StellarId(128)));
        let said = format!("Off you go, <PSN>.  {DEFECTED_ONE}");
        assert_lasts(&mut view, &said, 500);
    }

    #[test]
    fn fighters_abandoned_in_a_move_are_told_after_a_qs_message() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut view = told(fighters_out()).with_rules(rules);
        run_set(&mut view, "Q25048 M131");
        assert_eq!(view.settle_script(), None);
        assert_eq!(
            view.message(),
            Some("Off you go, <PSN>.  (Two fighters abandoned)")
        );
        assert_eq!(view.pilot().expect("flying").escorts(), []);
        view.message = None;
        assert_eq!(view.settle_script(), None);
        assert_eq!(view.message(), None, "nothing more to tell");
    }

    // Session events.

    /// The ships whose sheets `view` has asked for.
    fn sheets(view: &View) -> Vec<ShipId> {
        view.catalog().sheets_asked.borrow().clone()
    }

    fn systems_read(view: &View) -> Vec<SystemId> {
        view.catalog().systems_read.borrow().clone()
    }

    /// Settles `view`'s session by itself, not through the view.
    fn settle_session(view: &mut View) {
        let session = view.session.as_mut().expect("flying");
        let _ = session.settle_script(&view.catalog, &mut NeverFires);
    }

    #[test]
    fn the_hull_redraws_from_the_event_on_the_next_frame() {
        let mut view = FlightView::new(shipbuying());
        run_set(&mut view, "H129");
        settle_session(&mut view);
        assert_eq!(sheets(&view), [ShipId(128)], "not before the frame");
        view.tick(TICK);
        assert_eq!(
            sheets(&view),
            [ShipId(128), ShipId(129)],
            "a set expression"
        );
        view.tick(TICK);
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "once");

        let mut view = captured();
        let asked = sheets(&view).len();
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut NeverFires),
            Some(Assigned::MyShip)
        );
        assert_eq!(view.sheet_ship, Some(ShipId(128)), "not before the frame");
        view.tick(TICK);
        assert_eq!(view.sheet_ship, Some(ShipId(129)), "a capture");
        let player = sheets(&view)[asked..]
            .iter()
            .filter(|&&ship| ship == ShipId(129))
            .count();
        assert_eq!(player, 1, "read once, beside the old ship's as an escort");

        let mut view = FlightView::new(shipbuying());
        land_now(&mut view);
        let session = view.session.as_mut().expect("flying");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        view.tick(TICK);
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)], "a purchase");
    }

    /// The background of the status bar `view` shows.
    fn bar_shown(view: &View) -> Option<i16> {
        view.status_bar().ok().map(|bar| bar.layout.status_bkgnd)
    }

    fn bars_read(view: &View) -> Vec<i16> {
        view.catalog().bars_asked.borrow().clone()
    }

    #[test]
    fn the_status_bar_follows_the_ship_type_on_the_next_frame() {
        let mut view = FlightView::new(governed(shipbuying(), 129, 129));
        run_set(&mut view, "H129");
        settle_session(&mut view);
        assert_eq!(bar_shown(&view), Some(700), "not before the frame");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a set expression");
        view.tick(TICK);
        assert_eq!(bars_read(&view), [128, 129], "once");

        let mut view = captured_in(governed(boardable(), 129, 129));
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.assign(Assignment::MyShip, &mut NeverFires),
            Some(Assigned::MyShip)
        );
        assert_eq!(bar_shown(&view), Some(700), "not before the frame");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a capture");

        let mut view = FlightView::new(governed(shipbuying(), 129, 129));
        land_now(&mut view);
        let session = view.session.as_mut().expect("flying");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        view.tick(TICK);
        assert_eq!(bar_shown(&view), Some(705), "a purchase");
    }

    #[test]
    fn a_new_ship_types_missing_bar_keeps_the_layout_over_picture_128() {
        let mut view = FlightView::new(governed(shipbuying(), 129, 131));
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert_eq!(bars_read(&view), [128, 131]);
        assert_eq!(
            view.status_bar(),
            Ok(&StatusBar {
                layout: layout(),
                background: Some(hud::Background {
                    id: 128,
                    width: 194.0,
                    height: 767.0,
                }),
            })
        );
    }

    #[test]
    fn the_system_is_laid_out_from_the_event() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.catalog().systems_read.borrow_mut().clear();
        jump_now(&mut view);
        arrive_now(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a jump");

        let mut view = FlightView::new(holed());
        view.catalog().systems_read.borrow_mut().clear();
        land_now(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a wormhole");

        let mut view = flight();
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        settle_session(&mut view);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a set expression");

        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "a relocation");
    }

    #[test]
    fn several_moves_before_a_frame_lay_out_once_the_last_system() {
        let mut view = FlightView::new(FakeCatalog {
            onward: true,
            ..catalog()
        });
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131 M132");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
        assert_eq!(systems_read(&view), [SystemId(132)], "one layout");
    }

    /// A stellar moved to, then a system entered, before one frame: one
    /// layout, of the system entered. (Placing the ship as well after the
    /// layout would change nothing to see: its writes are the layout's.)
    #[test]
    fn a_system_entered_supersedes_a_stellar_moved_to_before_the_frame() {
        let mut view = landed_view();
        move_to(&mut view, 130, 129);
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.take_off(&view.catalog, &mut NeverFires),
            Some(StellarId(129))
        );
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)], "one layout");
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");
    }

    // The galaxy overlay.

    /// Changes `view`'s galaxy by itself, not through the view.
    fn change_galaxy(view: &mut View, change: impl FnOnce(&mut nova_sim::story::GalaxyOverlay)) {
        let session = view.session.as_mut().expect("flying");
        assert!(session.change_galaxy(&view.catalog, change));
    }

    /// Hides system `id`.
    fn hiding(id: i16) -> impl FnOnce(&mut nova_sim::story::GalaxyOverlay) {
        move |overlay| {
            overlay.set_visible(SystemId(id), Some(false));
        }
    }

    /// Whether `view`'s course map draws system `id`.
    fn on_the_course_map(view: &View, id: i16) -> bool {
        view.course_map().model().system(SystemId(id)).is_some()
    }

    #[test]
    fn a_pilot_flown_with_a_galaxy_overlay_shows_it_on_the_course_map() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.change_galaxy(&catalog, hiding(132));
        let view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert!(!on_the_course_map(&view, 132));
        assert!(on_the_course_map(&view, 131));
    }

    #[test]
    fn a_change_to_the_galaxy_is_drawn_on_the_next_frame_and_not_before() {
        let mut view = flight();
        change_galaxy(&mut view, hiding(132));
        assert!(on_the_course_map(&view, 132), "not before the frame");
        view.tick(TICK);
        assert!(!on_the_course_map(&view, 132));
    }

    #[test]
    fn the_course_map_shows_the_course_as_it_is_flown() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let mut view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert_eq!(view.course_map().route(), [SystemId(131)]);
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        assert_eq!(view.course_map().route(), [], "no system at its place");
        let session = view.session.as_ref().expect("flying");
        assert_eq!(session.course(), [SystemId(131)], "kept");
    }

    /// A flight of `reborn` Sol, bound for Alpha Centauri, Alpha Reborn
    /// explored.
    fn bound_for_alpha_with_reborn_explored() -> View {
        let catalog = FakeCatalog {
            reborn: true,
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.plot_course(SystemId(131)).expect("a route");
        let explore = nova_sim::SetExpr::parse("X133").expect("parses");
        session.run_set(&explore, &mut NeverFires);
        FlightView::with_pilot(catalog, session.pilot().clone())
    }

    #[test]
    fn the_nav_area_shows_the_system_standing_for_a_hidden_next_hop() {
        let mut view = bound_for_alpha_with_reborn_explored();
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Reborn"]);
    }

    #[test]
    fn the_nav_area_shows_no_destination_for_a_hop_with_no_system_visible_at_its_place() {
        let mut view = bound_for_alpha_with_reborn_explored();
        change_galaxy(&mut view, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
            overlay.set_visible(SystemId(133), Some(false));
        });
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
        let session = view.session().expect("flying");
        assert_eq!(session.course(), [SystemId(131)], "kept");
    }

    #[test]
    fn several_galaxy_changes_before_a_frame_rebuild_the_map_once() {
        let mut view = flight();
        change_galaxy(&mut view, hiding(132));
        change_galaxy(&mut view, |overlay| {
            overlay.set_visible(SystemId(132), None);
            overlay.set_visible(SystemId(131), Some(false));
        });
        let redrawn = view.catch_up();
        assert!(redrawn.galaxy);
        assert!(on_the_course_map(&view, 132), "the last galaxy");
        assert!(!on_the_course_map(&view, 131));
        assert!(!view.catch_up().galaxy, "once");
    }

    // Destroyed stellars (story-runtime phase 7).

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

    /// A flight in Sol whose Earth (128, `DeadType` sheet 2128), Moon
    /// (129, no different graphic) and Proxima (140, `DeadType` sheet
    /// 2140) can be destroyed.
    fn destructible() -> View {
        FlightView::new(FakeCatalog {
            lives: vec![life(128, 0), life(129, 0), life(140, 0)],
            ..catalog()
        })
    }

    /// Whether stellar `id` is drawn with its `Type` sheet (1000 + `id`)
    /// and whether with its `DeadType` sheet (2000 + `id`), at any of the
    /// first 4 frames.
    fn sheets_drawn(view: &View, id: i16) -> (bool, bool) {
        let images: Vec<ImageKey> = sprites(&drawn(view))
            .into_iter()
            .map(|(image, _)| image)
            .collect();
        let sheet = |image_id| (0..4).any(|f| images.contains(&ImageKey::sprite(image_id, f)));
        (sheet(1000 + id), sheet(2000 + id))
    }

    const TYPE: (bool, bool) = (true, false);
    const DEAD_TYPE: (bool, bool) = (false, true);

    #[test]
    fn a_stellar_destroyed_with_y_is_drawn_with_its_dead_type_from_the_next_frame_and_regenerated_with_u()
     {
        let mut view = destructible();
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        view.catalog().systems_read.borrow_mut().clear();

        run_set(&mut view, "Y128");
        settle_session(&mut view);
        assert_eq!(sheets_drawn(&view, 128), TYPE, "not before the frame");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "from the next frame");
        assert_eq!(systems_read(&view), [], "not laid out again");

        run_set(&mut view, "U128");
        settle_session(&mut view);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "not before the frame");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE, "regenerated");
        assert_eq!(systems_read(&view), []);
    }

    #[test]
    fn y_and_u_before_one_frame_draw_type() {
        let mut view = destructible();
        view.tick(TICK);
        run_set(&mut view, "Y128");
        run_set(&mut view, "U128");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
    }

    #[test]
    fn y_on_a_stellar_of_another_system_changes_nothing_drawn() {
        let mut view = destructible();
        view.tick(TICK);
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "Y140");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars().iter().all(|stellar| !stellar.destroyed));
        assert_eq!(systems_read(&view), []);
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_keeps_its_type() {
        let mut view = destructible();
        run_set(&mut view, "Y129");
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 129), TYPE);
        let scene = view.scene().expect("a scene");
        assert!(scene.stellars()[1].destroyed, "destroyed all the same");
    }

    #[test]
    fn a_system_entered_draws_its_destroyed_stellars_dead() {
        let mut view = destructible();
        run_set(&mut view, "Y140");
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
    }

    #[test]
    fn a_system_entered_after_its_stellar_was_destroyed_draws_it_dead() {
        let mut view = destructible();
        run_set(&mut view, "Y140");
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        run_set(&mut view, "M131");
        settle_session(&mut view);
        let redrawn = view.catch_up();
        assert!(redrawn.system, "laid out");
        assert!(!redrawn.galaxy, "by the system's reaction alone");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(sheets_drawn(&view, 140), DEAD_TYPE);
    }

    #[test]
    fn a_new_pilot_with_a_stellar_that_starts_destroyed_draws_it_dead_from_the_first_frame() {
        let mut view = FlightView::new(FakeCatalog {
            lives: vec![life(128, 0x0040)],
            ..catalog()
        });
        assert_eq!(sheets_drawn(&view, 128), TYPE, "not begun yet");
        view.begin();
        view.tick(TICK);
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE);
    }

    #[test]
    fn a_pilot_flown_with_a_stellar_destroyed_draws_it_dead_at_once() {
        let catalog = FakeCatalog {
            lives: vec![life(128, 0)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let destroy = nova_sim::SetExpr::parse("Y128").expect("parses");
        session.run_set(&destroy, &mut NeverFires);
        let view = FlightView::with_pilot(catalog, session.pilot().clone());
        assert_eq!(sheets_drawn(&view, 128), DEAD_TYPE, "before any frame");
    }

    // A `DeadType` with no sheet after a rebuild point (story-runtime
    // phase 9).

    /// Whether graphic type 0's sheet is drawn, at any of the first 4
    /// frames: Moon (129) is the only stellar that can draw it.
    fn type_zero_drawn(view: &View) -> bool {
        sprites(&drawn(view))
            .into_iter()
            .any(|(image, _)| (0..4).any(|f| image == ImageKey::sprite(TYPE_ZERO, f)))
    }

    /// Moon (129) is drawn with its `Type` sheet, or graphic type 0's.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Moon {
        Type,
        TypeZero,
    }

    fn moon(view: &View) -> Moon {
        match (sheets_drawn(view, 129), type_zero_drawn(view)) {
            (TYPE, false) => Moon::Type,
            ((false, false), true) => Moon::TypeZero,
            other => panic!("the Moon drawn as {other:?}"),
        }
    }

    /// [`destructible`] with `DeadTypeFallback` read as `reading`.
    fn reading(reading: RuleSource) -> View {
        destructible()
            .with_rules(Rulebook::default().with_override(RuleKey::DeadTypeFallback, reading))
    }

    /// The Moon after `Y129` and a frame; then after an in-flight `M130`
    /// and a frame; then `U129` and a frame; then `Y129` and a frame.
    fn moon_through_a_rebuild(mut view: View) -> [Moon; 4] {
        run_set(&mut view, "Y129");
        view.tick(TICK);
        let before = moon(&view);
        run_set(&mut view, "M130");
        settle_session(&mut view);
        view.tick(TICK);
        let rebuilt = moon(&view);
        run_set(&mut view, "U129");
        view.tick(TICK);
        let regenerated = moon(&view);
        run_set(&mut view, "Y129");
        view.tick(TICK);
        [before, rebuilt, regenerated, moon(&view)]
    }

    #[test]
    fn a_stellar_with_no_dead_type_draws_type_zero_from_the_next_rebuild_point_until_seen_whole() {
        assert_eq!(
            moon_through_a_rebuild(destructible()),
            [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type],
            "by default, the engine's"
        );
        assert_eq!(
            moon_through_a_rebuild(reading(RuleSource::Engine)),
            [Moon::Type, Moon::TypeZero, Moon::Type, Moon::Type]
        );
    }

    #[test]
    fn by_the_bible_a_stellar_with_no_dead_type_keeps_its_type_throughout() {
        assert_eq!(
            moon_through_a_rebuild(reading(RuleSource::Bible)),
            [Moon::Type; 4]
        );
    }

    #[test]
    fn a_take_off_that_only_rebuilds_draws_type_zero() {
        let mut view = FlightView::new(FakeCatalog {
            sites: vec![
                site(128, (6.0, -8.0), StellarFlags::CAN_LAND),
                site(129, (300.0, -200.0), StellarFlags::CAN_LAND),
            ],
            lives: vec![life(129, 0)],
            ..catalog()
        });
        land_now(&mut view);
        assert_eq!(
            view.session().expect("flying").landed(),
            Some(StellarId(128))
        );
        run_set(&mut view, "Y129");
        view.tick(TICK);
        assert_eq!(
            moon(&view),
            Moon::Type,
            "destroyed after the landing's rebuild"
        );
        view.catalog().systems_read.borrow_mut().clear();
        view.take_off();
        assert_eq!(moon(&view), Moon::TypeZero, "the take-off's rebuild");
        assert_eq!(systems_read(&view), [], "not laid out again");
    }

    /// A pilot loaded with the Moon destroyed, flown by `rules`.
    fn loaded(rules: Rulebook) -> View {
        let catalog = FakeCatalog {
            lives: vec![life(129, 0)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let destroy = nova_sim::SetExpr::parse("Y129").expect("parses");
        session.run_set(&destroy, &mut NeverFires);
        FlightView::with_pilot(catalog, session.pilot().clone()).with_rules(rules)
    }

    #[test]
    fn a_pilot_loaded_with_a_stellar_destroyed_draws_it_as_its_rules_say_from_the_first_frame() {
        let view = loaded(Rulebook::default());
        assert!(
            view.session()
                .expect("flying")
                .dead_since_rebuild(StellarId(129))
        );
        assert_eq!(moon(&view), Moon::TypeZero, "a load is a rebuild point");
        let bible = Rulebook::default().with_override(RuleKey::DeadTypeFallback, RuleSource::Bible);
        let view = loaded(bible);
        assert_eq!(moon(&view), Moon::Type, "before any event");
        let view = loaded(bible).with_rules(Rulebook::default());
        assert_eq!(moon(&view), Moon::TypeZero, "and back");
    }

    #[test]
    fn a_system_entered_by_a_jump_with_a_stellar_destroyed_draws_type_zero_from_its_first_frame() {
        let mut view = destructible();
        run_set(&mut view, "Y129");
        jump_to_alpha(&mut view);
        plot(&mut view, 130);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        arrive_now(&mut view);
        assert_eq!(view.session().expect("flying").system(), SystemId(130));
        assert_eq!(moon(&view), Moon::TypeZero);
    }

    #[test]
    fn a_frame_with_the_map_open_still_redraws_from_the_events() {
        let mut view = flight();
        tap(&mut view, MAP);
        assert!(view.map_open());
        view.catalog().systems_read.borrow_mut().clear();
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert!(view.map_open(), "still open");
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)]);

        let mut view = FlightView::new(shipbuying());
        tap(&mut view, MAP);
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert!(view.map_open(), "still open");
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
    }

    #[test]
    fn a_frame_during_the_streak_still_redraws_the_hull() {
        let mut view = FlightView::new(shipbuying());
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        run_set(&mut view, "H129");
        view.tick(TICK);
        assert!(view.jump_effect().is_some(), "still streaking");
        assert_eq!(view.session().map(Session::system), Ok(SystemId(130)));
        assert_eq!(sheets(&view), [ShipId(128), ShipId(129)]);
    }

    #[test]
    fn fighters_abandoned_are_told_only_with_the_layout_of_the_move_settled() {
        let rules = Rulebook::default().with_override(RuleKey::MoveArrival, RuleSource::Bible);
        let mut view = fighters_out().with_rules(rules);
        run_set(&mut view, "M131");
        settle_session(&mut view);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(view.settle_script(), None);
        assert_eq!(view.message(), None, "no layout from this settling");
        let session = view.session.as_mut().expect("flying");
        assert_eq!(
            session.take_fighter_notes(),
            [FighterNote::Abandoned(2)],
            "left for the next arrival"
        );
    }

    #[test]
    fn a_desk_move_redraws_with_no_extra_call() {
        let mut view = landed_view();
        move_to(&mut view, 131, 140);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(systems_read(&view), [SystemId(131)]);
        assert_eq!(view.course_map().current(), Some(SystemId(131)));
        assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");

        let mut view = landed_view();
        plot(&mut view, 131);
        move_to(&mut view, 130, 129);
        view.tick(TICK);
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
        assert_eq!(systems_read(&view), [], "the scene kept");
        assert_eq!(view.course_map().route(), [], "the course cleared");
        assert_eq!(
            view.shown_position(),
            Point::new(300.0, -200.0),
            "at the Moon"
        );
    }

    #[test]
    fn the_disable_rule_reaches_the_session_too() {
        let rule = Rc::new(Counting::default());
        let view = flight().with_disable_rule(rule.clone());
        let session = view.session().expect("flying");
        assert!(
            !session
                .disable_rule()
                .disabled(nova_sim::Gauge::full(10.0), &session.hull())
        );
        assert_eq!(rule.asked.get(), 1, "the session asks the rule given");
    }

    #[test]
    fn the_string_lists_reach_the_session() {
        let strings = FakeCatalog {
            strings: vec![(25040, vec!["Kestrel".to_owned()])],
            ..catalog()
        };
        let view = flight().with_strings(Rc::new(strings));
        assert_eq!(
            view.session().expect("flying").strings().string_list(25040),
            ["Kestrel"]
        );
    }

    // Boarding grants.

    #[test]
    fn a_grant_is_said_in_the_originals_words() {
        let part = |count| grant_message(count, "spare part", "spare parts");
        assert_eq!(part(1), "You retrieved a spare part from this ship.");
        assert_eq!(
            grant_message(1, "ion cannon", "ion cannons"),
            "You retrieved an ion cannon from this ship."
        );
        assert_eq!(
            grant_message(1, "Ion cannon", "Ion cannons"),
            "You retrieved an Ion cannon from this ship.",
            "its first letter lowercased for the article"
        );
        assert_eq!(part(2), "You retrieved two spare parts from this ship.");
        assert_eq!(part(10), "You retrieved ten spare parts from this ship.");
        assert_eq!(part(11), "You retrieved 11 spare parts from this ship.");
        assert_eq!(
            grant_message(
                1,
                "Dr Ralph's exploration map",
                "Dr Ralph's exploration maps"
            ),
            "You retrieved a Dr Ralph's exploration map from this ship."
        );
        for vowel in ["a", "e", "i", "o", "u"] {
            assert!(
                grant_message(1, vowel, "").starts_with("You retrieved an "),
                "{vowel}"
            );
        }
        assert_eq!(grant_message(1, "", ""), "You retrieved a  from this ship.");
        assert_eq!(
            [RETRIEVED, ARTICLE_A, ARTICLE_AN, FROM_THIS_SHIP],
            ["You retrieved ", "a", "an", "from this ship."]
        );
        assert_eq!(GRANT_TICKS, 240, "0xf0 @0x9342c");
    }

    /// Grants a person's `GrantCount` of the first outfit it may.
    #[derive(Debug)]
    struct Grants;

    impl BoardingRule for Grants {
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
            grant: &nova_sim::grant::PersonGrant,
            stock: &[nova_sim::grant::GrantStock],
            _free_mass: i64,
            _grant_max: nova_sim::RuleSource,
            _chance: &mut dyn Chance,
        ) -> Option<nova_sim::grant::Granted> {
            let first = nova_sim::grant::candidates(grant.class, stock)
                .first()
                .copied()?;
            Some(nova_sim::grant::Granted {
                outfit: first.outfit,
                count: grant.count,
            })
        }
    }

    /// [`peopled`] with Ace a derelict (government 140), so disabled,
    /// granting `count` spare parts (`oütf` 200, class 7), of no hail
    /// quote; its ship 129 has a crew of 3 and a board reach.
    fn granting(count: i16) -> FakeCatalog {
        let mut catalog = peopled();
        let ace = &mut catalog.persons[0];
        ace.govt = Some(GovtId(140));
        ace.hail_quote = -1;
        ace.grant_class = 7;
        ace.grant_prob = 100;
        ace.grant_count = count;
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0x0800,
            flags2: 0,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties::default(),
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
        }];
        for record in &mut catalog.ships {
            record.crew = 3;
        }
        catalog.hulls = vec![hull_of(129, &[])];
        catalog.outfits = vec![OutfitRecord {
            id: OutfitId(200),
            name: "Spare Part".to_owned(),
            short_name: "Spare Part".to_owned(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 100,
            mods: [(0, 0); 4],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 7,
            lc_name: "spare part".to_owned(),
            lc_plural: "spare parts".to_owned(),
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }];
        catalog
    }

    /// [`granting`]'s flight, by `rule`, with Ace listed and placed on
    /// the player, targeted and boarded.
    fn boarded_ace(count: i16, rule: Rc<dyn BoardingRule>) -> View {
        let (_, chance) = scripted(&[99, 750, 750, 0]);
        let mut view = FlightView::new(granting(count))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_boarding_rule(rule);
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        assert!(view.take_boarding().is_some(), "boarded");
        view
    }

    #[test]
    fn boarding_a_granting_person_says_what_it_retrieved_for_240_ticks() {
        let mut view = boarded_ace(2, Rc::new(Grants));
        let said = "You retrieved two spare parts from this ship.";
        assert_eq!(view.message(), Some(said));
        assert_eq!(view.pilot().expect("a pilot").owned(OutfitId(200)), 2);
        assert_lasts(&mut view, said, GRANT_TICKS);
        let view = boarded_ace(1, Rc::new(Grants));
        assert_eq!(
            view.message(),
            Some("You retrieved a spare part from this ship.")
        );
    }

    #[test]
    fn boarding_a_person_that_grants_nothing_says_nothing() {
        let view = boarded_ace(2, Rc::new(Sure));
        assert_eq!(view.message(), None);
    }

    // Persons' mission offers.

    /// [`peopled`] with Ace of `flags` linked to mission 130, offered
    /// from a ship, its offer text `offer`, refusable unless
    /// `unrefusable`.
    fn offering(flags: u16, offer: Option<&str>, unrefusable: bool) -> FakeCatalog {
        let mut catalog = peopled();
        catalog.persons[0].flags = flags;
        catalog.persons[0].link_mission = Some(130);
        catalog.missions = vec![nova_sim::catalog::MissionRecord {
            id: MissionId(130),
            avail_stel: -1,
            avail_loc: 2,
            avail_record: 0,
            avail_rating: 0,
            avail_random: 100,
            avail_ship_type: 0,
            avail_bits: nova_sim::Test::default(),
            require: 0,
            flags: if unrefusable { 0x0004 } else { 0 },
            flags2: 0,
            travel_stel: -1,
            return_stel: -1,
            cargo_type: -1,
            cargo_qty: 0,
            pickup_mode: -1,
            drop_off_mode: -1,
            scan_mask: 0,
            pay_val: 0,
            time_limit: 0,
            disp_weight: 0,
            on_accept: nova_sim::Script::default(),
            on_refuse: nova_sim::Script::default(),
            on_success: nova_sim::Script::default(),
            on_failure: nova_sim::Script::default(),
            on_abort: nova_sim::Script::default(),
            on_ship_done: nova_sim::Script::default(),
            can_abort: 1,
            comp_govt: None,
            comp_reward: 0,
            date_post_inc: 0,
            ships: nova_sim::catalog::MissionShip::NONE,
            aux: nova_sim::catalog::AuxShip::NONE,
        }];
        catalog.mission_texts = vec![nova_sim::catalog::MissionTexts {
            offer: offer.map(str::to_owned),
            ..nova_sim::catalog::MissionTexts::named(MissionId(130), "Help Ace")
        }];
        catalog
    }

    /// `catalog`'s flight with Ace listed and placed 100 above the
    /// player, targeted.
    fn targeting_ace(catalog: FakeCatalog) -> View {
        let (_, chance) = scripted(&ACE[..4]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        view
    }

    #[test]
    fn y_on_a_person_offering_its_mission_reports_the_offer_once() {
        let mut view = targeting_ace(offering(0, Some("<OSN> needs help."), false));
        assert_eq!(view.take_offer(), None);
        view.input(&key(Key::Up, true));
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None, "no comm dialog");
        let prompt = view.take_offer().expect("offered");
        assert_eq!(prompt.mission, MissionId(130));
        assert_eq!(prompt.text, "Ace needs help.");
        assert_eq!(view.take_offer(), None, "once");
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        assert_eq!(
            view.pilot().expect("a pilot").missions().missions(),
            [MissionId(130)]
        );
    }

    #[test]
    fn i_with_no_mission_says_you_have_no_active_missions() {
        let mut view = flight();
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None);
        assert_lasts(&mut view, NO_ACTIVE_MISSIONS, 240);
    }

    #[test]
    fn i_with_an_active_mission_asks_for_the_info_screen_once() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.accept_mission(MissionId(130)), Ok(0));
        view.input(&key(Key::Up, true));
        tap(&mut view, MISSION_INFO_KEY);
        let info = view.take_mission_info_request().expect("asked");
        assert_eq!(info, view.mission_info());
        assert_eq!(view.take_mission_info_request(), None, "once");
        assert_eq!(view.message(), None);
        view.tick(TICK);
        assert_eq!(
            view.session().expect("flying").player().velocity,
            Vec2::ZERO,
            "the flight keys let go"
        );
    }

    #[test]
    fn i_with_the_map_open_does_nothing() {
        let mut view = flight();
        tap(&mut view, MAP_KEY);
        assert!(view.map_open());
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.message(), None);
        assert_eq!(view.take_mission_info_request(), None);
    }

    #[test]
    fn an_offer_keeps_the_message() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        view.say_for(MISSION_FAILED, 240);
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    }

    #[test]
    fn a_refusal_of_the_offer_goes_through_the_session() {
        let mut view = targeting_ace(offering(0, Some("Help?"), false));
        tap(&mut view, HAIL_KEY);
        view.take_offer().expect("offered");
        assert_eq!(view.refuse_mission(MissionId(130)), Ok(()));
        assert_eq!(
            view.refuse_mission(MissionId(130)),
            Err(NotOffered),
            "answered"
        );
    }

    #[test]
    fn b_on_a_person_offering_on_boarding_reports_the_offer_and_no_plunder() {
        let mut catalog = granting(2);
        let ace = &mut catalog.persons[0];
        ace.flags = 0x0200;
        ace.link_mission = Some(130);
        catalog.missions = offering(0, None, false).missions;
        catalog.mission_texts = offering(0, Some("Aboard."), false).mission_texts;
        let (_, chance) = scripted(&[99, 750, 750, 0]);
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still))
            .with_boarding_rule(Rc::new(Grants));
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        tap(&mut view, BOARD_KEY);
        assert_eq!(view.take_boarding(), None, "no plunder dialog");
        assert_eq!(
            view.take_offer().map(|prompt| prompt.text),
            Some("Aboard.".to_owned())
        );
        assert_eq!(view.message(), None, "nothing granted");
    }

    #[test]
    fn an_offer_accepted_at_once_shows_nothing() {
        let mut view = targeting_ace(offering(0, None, true));
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_offer(), None);
        assert_eq!(view.take_hail(), None);
        assert_eq!(
            view.pilot().expect("a pilot").missions().missions(),
            [MissionId(130)]
        );
    }

    // The navigation target.

    /// Two stellars the ship starts over, at rest: A (128), small and
    /// nearest, then B (129), wide and a little further off, in that nav
    /// order; and C (130), drawn where the status panel covers it.
    fn a_and_b() -> View {
        let wide = LandingSite {
            frame_size: Some((200, 200)),
            ..site(129, (40.0, 0.0), StellarFlags::CAN_LAND)
        };
        flight_among(vec![
            site(128, (10.0, 0.0), StellarFlags::CAN_LAND),
            wide,
            site(130, (330.0, 0.0), StellarFlags::CAN_LAND),
        ])
    }

    /// Where `view` draws the system point `(x, y)`.
    fn on_screen(view: &View, (x, y): (f32, f32)) -> Point {
        view.camera().world_to_screen(at(x, y))
    }

    #[test]
    fn a_slot_key_retargets_and_l_lands_on_the_new_target() {
        for slot_key in STELLAR_SLOT_KEYS[1] {
            let mut view = a_and_b();
            tap(&mut view, LAND);
            assert_eq!(nav_target(&view), Some(StellarId(128)), "{slot_key:?}");
            tap(&mut view, slot_key);
            assert_eq!(nav_target(&view), Some(StellarId(129)), "{slot_key:?}");
            tap(&mut view, LAND);
            assert_eq!(view.take_landing(), Some(StellarId(129)), "{slot_key:?}");
        }
    }

    #[test]
    fn the_slot_keys_are_1_to_4_and_f1_to_f4_in_nav_order() {
        assert_eq!(
            STELLAR_SLOT_KEYS,
            [
                [Key::Char('1'), Key::Function(1)],
                [Key::Char('2'), Key::Function(2)],
                [Key::Char('3'), Key::Function(3)],
                [Key::Char('4'), Key::Function(4)],
            ]
        );
        for (slot, keys) in STELLAR_SLOT_KEYS.iter().enumerate().take(3) {
            for &slot_key in keys {
                let mut view = a_and_b();
                tap(&mut view, slot_key);
                let expected = [128, 129, 130][slot];
                assert_eq!(nav_target(&view), Some(StellarId(expected)), "{slot_key:?}");
            }
        }
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        for &empty in &STELLAR_SLOT_KEYS[3] {
            tap(&mut view, empty);
            assert_eq!(nav_target(&view), Some(StellarId(129)), "empty slot keeps");
        }
    }

    #[test]
    fn a_slot_key_repeat_or_release_does_nothing() {
        let mut view = a_and_b();
        view.input(&held(Key::Char('2')));
        view.input(&key(Key::Char('2'), false));
        assert_eq!(nav_target(&view), None);
    }

    #[test]
    fn a_click_on_a_stellar_retargets_and_l_lands_there() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let b = on_screen(&view, (40.0, 0.0));
        click(&mut view, b);
        assert_eq!(nav_target(&view), Some(StellarId(129)));
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), Some(StellarId(129)));
    }

    #[test]
    fn a_click_picks_the_stellar_above_or_below_the_ship_where_it_is_drawn() {
        let mut view = flight_among(vec![
            site(128, (0.0, 150.0), StellarFlags::CAN_LAND),
            site(129, (0.0, -150.0), StellarFlags::CAN_LAND),
        ]);
        for (id, y) in [(129, -150.0), (128, 150.0), (129, -150.0)] {
            let drawn = on_screen(&view, (0.0, y));
            click(&mut view, drawn);
            assert_eq!(nav_target(&view), Some(StellarId(id)), "{y}");
        }
    }

    #[test]
    fn a_click_on_empty_space_keeps_the_target() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let empty = on_screen(&view, (-300.0, -300.0));
        click(&mut view, empty);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
    }

    #[test]
    fn a_right_click_or_a_release_selects_nothing() {
        let mut view = a_and_b();
        let b = on_screen(&view, (40.0, 0.0));
        for (button, pressed) in [
            (crate::MouseButton::Right, true),
            (crate::MouseButton::Left, false),
        ] {
            view.input(&Input::PointerButton {
                button,
                pressed,
                at: b,
            });
            assert_eq!(nav_target(&view), None, "{button:?} {pressed}");
        }
    }

    #[test]
    fn a_click_on_the_status_panel_keeps_the_target_even_over_a_stellar() {
        let mut view = a_and_b();
        tap(&mut view, LAND);
        let c = on_screen(&view, (330.0, 0.0));
        let edge = VIEW_SIZE.0 - STATUS_BAR_WIDTH;
        assert!(c.x > edge, "{c:?} is under the panel");
        click(&mut view, c);
        assert_eq!(nav_target(&view), Some(StellarId(128)), "over C");
        // C's box reaches 26 pixels either side of it, across the panel's
        // edge, which decides.
        let left_of_c = on_screen(&view, (330.0 - 26.0, 0.0)).x;
        assert!(left_of_c < edge && edge < c.x);
        click(&mut view, at(edge, c.y));
        assert_eq!(nav_target(&view), Some(StellarId(128)), "the edge is panel");
        click(&mut view, at(edge - 0.5, c.y));
        assert_eq!(
            nav_target(&view),
            Some(StellarId(130)),
            "left of it is space"
        );
    }

    #[test]
    fn f5_selects_the_nearest_stellar() {
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        tap(&mut view, NEAREST_STELLAR_KEY);
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        assert_eq!(NEAREST_STELLAR_KEY, Key::Function(5));
    }

    #[test]
    fn nav_off_clears_the_target_and_the_next_l_selects_the_nearest() {
        let mut view = a_and_b();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_target(&view), None);
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), None, "selects, not lands");
        assert_eq!(nav_target(&view), Some(StellarId(128)));
        assert_eq!(NAV_OFF_KEY, Key::Char('`'));
    }

    // The nav target's brackets.

    /// The nav bracket `cïcn`s drawn (10000-10007), and where.
    fn nav_icons(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(10000..=10007)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
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

    #[test]
    fn selecting_a_stellar_draws_cicn_10000_to_10003_flying_in_then_settled() {
        let mut view = flight();
        assert_eq!(nav_icons(&view), []);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        // Earth (128), 20 x 20: half 10.
        let earth = on_screen(&view, (0.0, -600.0));
        assert_eq!(
            nav_icons(&view),
            corners_at(10000, earth, 10.0 + 196.0),
            "a frame drawn before the next tick"
        );
        for expand in [196.0, 136.0, 76.0, 16.0, 0.0, 0.0] {
            view.tick(TICK);
            let earth = on_screen(&view, (0.0, -600.0));
            assert_eq!(
                nav_icons(&view),
                corners_at(10000, earth, 10.0 + expand),
                "{expand}"
            );
        }
    }

    #[test]
    fn no_nav_brackets_without_a_nav_target_or_after_hyper_select() {
        let mut view = flight();
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        assert_eq!(nav_icons(&view).len(), 4);
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_icons(&view), [], "Nav Off");
        view.tick(TICK);
        assert_eq!(nav_icons(&view), []);
        tap(&mut view, STELLAR_SLOT_KEYS[1][0]);
        let moon = on_screen(&view, (300.0, -200.0));
        assert_eq!(
            nav_icons(&view),
            corners_at(10000, moon, 10.0 + 196.0),
            "flies in again"
        );
        tap(&mut view, HYPER_SELECT_KEY);
        assert_eq!(view.session().expect("flying").nav_target(), None);
        assert_eq!(nav_icons(&view), [], "Hyper Select");
    }

    #[test]
    fn a_stellar_whose_sprite_cannot_be_read_is_bracketed_at_half_64() {
        let unreadable = StellarContents {
            sprite: Err("no rlëD".to_owned()),
            ..stellar(131, "Broken", (100, 0), 1)
        };
        let mut sites = catalog().sites;
        sites.push(site(131, (100.0, 0.0), StellarFlags::CAN_LAND));
        let mut view = FlightView::new(FakeCatalog {
            sites,
            sol_more: vec![unreadable],
            ..catalog()
        });
        tap(&mut view, STELLAR_SLOT_KEYS[2][0]);
        ticks(&mut view, 5);
        let broken = on_screen(&view, (100.0, 0.0));
        assert_eq!(nav_icons(&view), corners_at(10000, broken, 64.0));
    }

    #[test]
    fn an_always_dominated_stellar_is_bracketed_green() {
        let mut sites = catalog().sites;
        sites[0].flags2 = nova_sim::market::DOMINATED;
        let mut view = flight_among(sites);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        ticks(&mut view, 5);
        let earth = on_screen(&view, (0.0, -600.0));
        assert_eq!(nav_icons(&view), corners_at(10004, earth, 10.0));
    }

    #[test]
    fn with_the_map_open_a_click_goes_to_the_map_not_the_stellars() {
        let mut view = a_and_b();
        let b = on_screen(&view, (40.0, 0.0));
        tap(&mut view, MAP);
        click(&mut view, b);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(nav_target(&view), None);
    }

    fn nav_target(view: &View) -> Option<StellarId> {
        view.session().expect("flying").nav_target()
    }

    /// The texts drawn in the HUD's nav area, the stock bar's at (830, 0).
    fn nav(view: &View) -> Vec<String> {
        let area = layout().nav.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The texts drawn in the HUD's nav area, as [`nav`] gives them, each
    /// with its colour.
    fn nav_colored(view: &View) -> Vec<(String, Color)> {
        let area = layout().nav.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } if area.contains(*origin) => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    /// The colour the nav area's value, its second line, is drawn in.
    fn nav_value_color(view: &View) -> Color {
        nav_colored(view)[1].1
    }

    /// The texts drawn in the cargo area, at the stock bar's (830, 0),
    /// with where each starts.
    fn cargo(view: &View) -> Vec<(String, Point)> {
        let area = layout().cargo.offset(at(830.0, 0.0));
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } if area.contains(*origin) => {
                    Some((text.clone(), *origin))
                }
                _ => None,
            })
            .collect()
    }

    /// `text` from `x` right of the stock cargo area's left, (8, 458) at
    /// (830, 0), on `baseline` below its top, in Geneva 12.
    fn cargo_line(text: &str, x: f32, baseline: f32) -> (String, Point) {
        (text.to_owned(), at(838.0 + x, 458.0 + baseline - 12.0))
    }

    #[test]
    fn the_hud_lists_the_goods_bought_the_free_space_and_the_cash() {
        let labels = FakeCatalog {
            strings: vec![(4003, vec!["Food:".to_owned(), "Ind:".to_owned()])],
            ..catalog()
        };
        let mut view = FlightView::new(trading()).with_strings(Rc::new(labels));
        assert_eq!(
            cargo(&view),
            [
                cargo_line(hud::CARGO_FREE, 77.0, 12.0),
                cargo_line("10", 110.0, 12.0),
                cargo_line("Credits:", 77.0, 66.0),
                cargo_line("1,000", 87.0, 82.0),
            ],
            "an empty 10-ton hold, and no room for the date"
        );
        land_now(&mut view);
        assert_eq!(view.trade(BUY_FOOD), Ok(10));
        assert_eq!(
            cargo(&view),
            [
                cargo_line("Food:", 3.0, 12.0),
                cargo_line("10", 41.0, 12.0),
                cargo_line(hud::CARGO_FREE, 77.0, 12.0),
                cargo_line("0", 110.0, 12.0),
                cargo_line("Credits:", 77.0, 66.0),
                cargo_line("250", 87.0, 82.0),
            ]
        );
    }

    #[test]
    fn a_cargo_area_with_room_shows_the_date_on_its_last_line() {
        assert_eq!(
            layout().cargo,
            crate::geometry::Bounds {
                min: at(8.0, 458.0),
                max: at(184.0, 552.0),
            }
        );
        let taller = FakeCatalog {
            bar: Ok(StatusBarLayout {
                cargo: crate::geometry::Bounds {
                    min: at(8.0, 440.0),
                    max: at(184.0, 552.0),
                },
                ..layout()
            }),
            ..catalog()
        };
        let mut view = FlightView::new(taller);
        // One line of Geneva 12 (1.2 x 12) above the area's bottom.
        let last_line = at(838.0, 552.0 - 1.2 * 12.0);
        let date = |view: &View| {
            cargo(view)
                .into_iter()
                .filter(|(text, _)| text.ends_with("NC"))
                .collect::<Vec<_>>()
        };
        assert_eq!(date(&view), [(DATE.to_owned(), last_line)]);
        jump_to_alpha(&mut view);
        assert_eq!(date(&view), [("June 24, 1177 NC".to_owned(), last_line)]);
    }

    #[test]
    fn with_nothing_selected_and_no_course_the_hud_says_nav_system_off() {
        assert_eq!(nav(&flight()), [hud::NAV_OFF]);
        assert_eq!(nav(&every_frame(flight())), [hud::NAV_OFF]);
    }

    #[test]
    fn the_nav_area_is_centred_by_the_metrics_given() {
        use crate::text::TextMetrics as _;
        use crate::text::fixture::MonoMetrics;
        let view = flight().with_metrics(Rc::new(MonoMetrics));
        let origin = drawn(&view)
            .iter()
            .find_map(|c| match c {
                DrawCommand::Text { text, origin, .. } if text == hud::NAV_OFF => Some(*origin),
                _ => None,
            })
            .expect("Nav System Off");
        let layout = layout();
        let area = layout.nav.offset(at(830.0, 0.0));
        let width = MonoMetrics.width(layout.font, layout.font_size, hud::NAV_OFF);
        // The stock area's middle is 830 + (8 + 184) / 2 = 926.
        assert_eq!(f32::midpoint(area.min.x, area.max.x), 926.0);
        assert_eq!(
            origin,
            at(
                926.0 - (width / 2.0).trunc(),
                area.min.y + hud::NAV_OFF_BASELINE - layout.font_size
            )
        );
    }

    #[test]
    fn a_stellar_target_not_in_the_scene_is_stellar_navigation_over_no_destination() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert!(nav_target(&view).is_some());
        view.scene = None;
        assert_eq!(view.nav_now(), NavDisplay::Stellar(None));
    }

    #[test]
    fn the_hud_shows_the_selected_stellar_by_name() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the nearest");
    }

    #[test]
    fn a_plotted_jump_shows_hyperspace_and_the_next_system_if_explored() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        jump_to_alpha(&mut view);
        assert_eq!(nav(&view), [hud::NAV_OFF], "arrived");
        plot(&mut view, 130);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Sol"]);
    }

    #[test]
    fn the_destination_is_dim_until_the_ship_is_out_far_enough_to_jump() {
        // Read the Bible's way, the nav area follows every frame.
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        let hyperspace = |value: Color| {
            vec![
                (hud::NAV_HYPERSPACE.to_owned(), Color::DIM),
                (hud::NAV_UNEXPLORED.to_owned(), value),
            ]
        };
        assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
        let clear_iff_out = |view: &View| {
            let out = player(view).position.length() >= MIN_JUMP_DISTANCE;
            let expected = if out { Color::WHITE } else { Color::DIM };
            assert_eq!(nav_value_color(view), expected, "{:?}", player(view));
            out
        };
        view.input(&key(Key::Up, true));
        let mut dim_ticks = 0;
        while !clear_iff_out(&view) {
            dim_ticks += 1;
            assert!(dim_ticks < 2000, "never got out");
            view.tick(TICK);
        }
        assert!(dim_ticks > 0, "it started inside");
        assert_eq!(nav_colored(&view), hyperspace(Color::WHITE));
        // Back inside: Down turns against the motion, then Up thrusts.
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        for _ in 0..90 {
            view.tick(TICK);
            clear_iff_out(&view);
        }
        view.input(&key(Key::Down, false));
        view.input(&key(Key::Up, true));
        let mut out_ticks = 0;
        while clear_iff_out(&view) {
            out_ticks += 1;
            assert!(out_ticks < 2000, "never got back in");
            view.tick(TICK);
        }
        view.release_keys();
        assert_eq!(nav_colored(&view), hyperspace(Color::DIM));
    }

    #[test]
    fn in_a_system_with_no_stellars_the_destination_is_bright_from_the_centre() {
        let mut view = FlightView::new(FakeCatalog {
            sites: Vec::new(),
            ..catalog()
        });
        plot(&mut view, 131);
        assert_eq!(player(&view).position, Vec2::ZERO);
        assert_eq!(nav_value_color(&view), Color::WHITE);
    }

    #[test]
    fn once_j_is_accepted_the_hyperspace_label_turns_bright_too() {
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        fly_out(&mut view);
        tap(&mut view, JUMP);
        assert_eq!(
            nav_colored(&view),
            [
                (hud::NAV_HYPERSPACE.to_owned(), Color::WHITE),
                (hud::NAV_UNEXPLORED.to_owned(), Color::WHITE),
            ]
        );
    }

    /// The engine tick `view` has reached.
    fn now_tick(view: &View) -> u64 {
        u64::try_from(crate::time::ticks(view.elapsed)).expect("in range")
    }

    #[test]
    fn the_nav_area_holds_its_colours_until_its_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        assert_eq!(nav_value_color(&view), Color::DIM, "inside, at once");
        view.input(&key(Key::Up, true));
        let mut expected = Color::DIM;
        let mut held = 0;
        for _ in 0..2000 {
            let before = now_tick(&view);
            view.tick(TICK);
            let now = now_tick(&view);
            let out = player(&view).position.length() >= MIN_JUMP_DISTANCE;
            if now / NAV_PERIOD_TICKS > before / NAV_PERIOD_TICKS {
                expected = if out { Color::WHITE } else { Color::DIM };
            }
            assert_eq!(nav_value_color(&view), expected, "tick {now}");
            if out && expected == Color::DIM {
                held += 1;
            }
            if expected == Color::WHITE {
                break;
            }
        }
        view.release_keys();
        assert_eq!(expected, Color::WHITE, "it got out");
        assert!(
            (1..u32::try_from(NAV_PERIOD_TICKS).expect("small")).contains(&held),
            "held dim {held} ticks after the ship got clear"
        );
    }

    #[test]
    fn the_nav_area_holds_its_text_until_its_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        // Move to just past a refresh.
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        for _ in 1..NAV_PERIOD_TICKS - 1 {
            view.tick(TICK);
            assert_eq!(
                nav(&view),
                [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED],
                "tick {}",
                now_tick(&view)
            );
        }
        view.tick(TICK);
        assert_eq!(now_tick(&view) % NAV_PERIOD_TICKS, 0);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn read_the_bibles_way_the_nav_area_follows_every_frame() {
        let mut view = every_frame(flight());
        plot(&mut view, 131);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn a_selection_change_shows_at_once() {
        let mut view = flight();
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "drawn at once");
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "and kept");
        tap(&mut view, HYPER_SELECT);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        view.tick(TICK);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        assert_ne!(now_tick(&view) % NAV_PERIOD_TICKS, 0, "between refreshes");
    }

    #[test]
    fn redrawing_the_status_retakes_the_nav_area() {
        let mut view = flight();
        plot(&mut view, 131);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        run_set(&mut view, "X131");
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
        view.redraw_status();
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, "Alpha Centauri"]);
    }

    #[test]
    fn once_j_is_accepted_the_label_turns_bright_at_the_next_refresh() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        while now_tick(&view) % NAV_PERIOD_TICKS != 1 {
            view.tick(TICK);
        }
        let label = |view: &View| nav_colored(view)[0].1;
        assert_eq!(label(&view), Color::DIM);
        tap(&mut view, JUMP);
        assert_eq!(label(&view), Color::DIM, "held");
        for _ in 1..NAV_PERIOD_TICKS - 1 {
            view.tick(TICK);
            assert_eq!(label(&view), Color::DIM, "tick {}", now_tick(&view));
        }
        view.tick(TICK);
        assert_eq!(label(&view), Color::WHITE, "tick {}", now_tick(&view));
    }

    #[test]
    fn with_a_target_and_a_course_the_hud_shows_the_target() {
        let mut view = flight();
        plot(&mut view, 131);
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
        assert_eq!(
            view.session().expect("flying").course(),
            [SystemId(131)],
            "the course is kept"
        );
    }

    #[test]
    fn arriving_clears_the_target_and_l_selects_in_the_new_system() {
        let mut view = flight();
        tap(&mut view, LAND);
        jump_to_alpha(&mut view);
        assert_eq!(nav_target(&view), None);
        assert_eq!(nav(&view), [hud::NAV_OFF]);
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Proxima"]);
    }

    #[test]
    fn tab_in_a_flight_that_never_started_does_nothing() {
        let mut view = FlightView::new(FakeCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        });
        assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
        assert!(view.session().is_err());
    }

    // Hyper Select.

    const HYPER_SELECT: Key = Key::Char('\\');

    /// A flight from Sol, which lists Alpha Centauri then Barnard.
    fn fanned() -> View {
        FlightView::new(FakeCatalog {
            fan: true,
            ..catalog()
        })
    }

    fn course(view: &View) -> Vec<SystemId> {
        view.session().expect("flying").course().to_vec()
    }

    #[test]
    fn the_hyper_select_key_plots_the_next_listed_system() {
        let mut view = fanned();
        assert_eq!(HYPER_SELECT_KEY, HYPER_SELECT);
        for expected in [131, 132, 131] {
            assert_eq!(view.input(&key(HYPER_SELECT, true)), ScreenAction::None);
            view.input(&key(HYPER_SELECT, false));
            assert_eq!(course(&view), [SystemId(expected)]);
        }
    }

    #[test]
    fn the_hud_nav_area_shows_the_selected_system() {
        let mut view = flight();
        tap(&mut view, LAND);
        assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
        tap(&mut view, HYPER_SELECT);
        assert_eq!(nav_target(&view), None);
        assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    }

    #[test]
    fn a_repeat_or_release_of_the_hyper_select_key_does_not_cycle() {
        let mut view = fanned();
        view.input(&held(HYPER_SELECT));
        view.input(&key(HYPER_SELECT, false));
        assert_eq!(course(&view), []);
        tap(&mut view, HYPER_SELECT);
        view.input(&held(HYPER_SELECT));
        view.input(&key(HYPER_SELECT, false));
        assert_eq!(course(&view), [SystemId(131)]);
    }

    #[test]
    fn hyper_select_is_ignored_while_a_jump_prepares_or_plays() {
        let mut view = fanned();
        tap(&mut view, HYPER_SELECT);
        fly_out(&mut view);
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump(), "braking and turning");
        tap(&mut view, HYPER_SELECT);
        assert_eq!(course(&view), [SystemId(131)]);
        jump_now(&mut view);
        tap(&mut view, HYPER_SELECT);
        assert_eq!(course(&view), [SystemId(131)]);
    }

    #[test]
    fn hyper_select_with_the_map_open_changes_nothing() {
        let mut view = fanned();
        tap(&mut view, MAP);
        tap(&mut view, HYPER_SELECT);
        assert!(view.map_open());
        assert_eq!(course(&view), []);
    }

    // Hypergates and wormholes.

    use crate::flight::jump::{ARRIVAL_FLASH_FOR, FADE_IN_FOR};
    use nova_sim::GateKind;

    /// Stellar `id` in `system` at (`x`, `y`), with `flags2`, `links` and
    /// `exit_angle`.
    fn gate_site(
        id: i16,
        system: i16,
        (x, y): (f32, f32),
        flags2: u16,
        links: &[i16],
        exit_angle: i16,
    ) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            system: SystemId(system),
            position: Vec2::new(x, y),
            flags2,
            links: slots,
            exit_angle,
        }
    }

    /// Sol holds hypergate 300 (a station) at its centre, over the ship,
    /// linked to 310 in Alpha Centauri at (100, 200), heading ships out on
    /// 90°.
    fn gated() -> FakeCatalog {
        FakeCatalog {
            sites: vec![LandingSite {
                flags2: 0x1200,
                ..site(
                    300,
                    (0.0, 0.0),
                    StellarFlags::CAN_LAND | StellarFlags::STATION,
                )
            }],
            gates: vec![
                gate_site(300, 130, (0.0, 0.0), 0x1200, &[310], 120),
                gate_site(310, 131, (100.0, 200.0), 0x1000, &[300], 90),
            ],
            ..catalog()
        }
    }

    /// Sol holds unlinked wormhole 400 at its centre, over the ship, and
    /// Alpha Centauri unlinked wormhole 410 at (-300, 40).
    fn holed() -> FakeCatalog {
        FakeCatalog {
            sites: vec![LandingSite {
                flags2: 0x2200,
                ..site(400, (0.0, 0.0), StellarFlags::CAN_LAND)
            }],
            gates: vec![
                gate_site(400, 130, (0.0, 0.0), 0x2200, &[], 120),
                gate_site(410, 131, (-300.0, 40.0), 0x2000, &[], 0),
            ],
            ..catalog()
        }
    }

    fn system_of(view: &View) -> SystemId {
        view.session().expect("flying").system()
    }

    #[test]
    fn the_gate_messages_read_as_the_original() {
        assert_eq!(
            arrival_message_with(EXITING_HYPERGATE, "Alpha Centauri", DATE, true),
            "Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC."
        );
        assert_eq!(
            arrival_message_with(PASSING_WORMHOLE, "Barnard", DATE, false),
            "Passing through a wormhole into the Barnard system on June 23, 1177 NC. \
             No stellar objects present."
        );
        assert_eq!(
            arrival_message_with(JUMPING_INTO, "Sol", DATE, true),
            arrival_message("Sol", DATE, true)
        );
        assert_eq!(
            hypergate_clearance_message(true),
            "Hypergate is energized. Begin initial approach."
        );
        assert_eq!(
            hypergate_clearance_message(false),
            "Hypergate is online. Begin initial approach."
        );
        assert_eq!(
            gate_refusal_message(GateKind::Hypergate),
            "Your ship is unable to enter this hypergate - it is offline."
        );
        assert_eq!(
            gate_refusal_message(GateKind::Wormhole),
            "Your ship is unable to enter this wormhole - the radiation levels are too extreme."
        );
        assert_eq!(HYPERGATE_CANCELLED, "Hypergate jump cancelled.");
        assert_eq!(HYPERGATE_DENIED, "Hypergate usage denied.");
    }

    #[test]
    fn hypergate_clearance_is_energized_or_online_by_a_roll() {
        let mut view = FlightView::new(gated());
        tap(&mut view, LAND);
        assert_eq!(
            view.message(),
            Some("Hypergate is energized. Begin initial approach."),
            "the first outcome"
        );
        let last: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
        let mut view = FlightView::new(gated()).with_chance(SharedChance::new(last));
        tap(&mut view, LAND);
        assert_eq!(
            view.message(),
            Some("Hypergate is online. Begin initial approach."),
            "the second outcome"
        );
        assert_lasts(
            &mut view,
            "Hypergate is online. Begin initial approach.",
            250,
        );
    }

    #[test]
    fn a_hypergate_the_pilot_may_not_use_denies_it_in_its_own_words() {
        let mut catalog = gated();
        catalog.sites[0].min_status = 100;
        let mut view = FlightView::new(catalog);
        tap(&mut view, LAND);
        assert_lasts(&mut view, HYPERGATE_DENIED, 250);
    }

    #[test]
    fn an_offline_gate_is_refused_in_its_own_words() {
        for (flags2, kind) in [(0x1000, GateKind::Hypergate), (0x2000, GateKind::Wormhole)] {
            let mut view = flight_among(vec![LandingSite {
                flags2,
                ..site(300, (0.0, 0.0), StellarFlags::STATION)
            }]);
            tap(&mut view, LAND);
            let refusal = gate_refusal_message(kind);
            assert_lasts(&mut view, &refusal, 250);
        }
        let mut planet = flight_among(vec![site(300, (0.0, 0.0), 0)]);
        tap(&mut planet, LAND);
        assert_eq!(planet.message(), Some(HOSTILE_PLANET), "not a gate");
        assert_lasts(&mut planet, HOSTILE_PLANET, 250);
    }

    #[test]
    fn l_twice_at_a_hypergate_opens_the_map_offering_its_links() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        assert!(view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_eq!(view.take_landing(), None, "not landed");
        assert_eq!(system_of(&view), SystemId(130));
    }

    #[test]
    fn closing_the_hypergate_map_on_a_pick_comes_out_of_its_gate() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(player(&view).position, Vec2::new(100.0, 200.0));
        assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Centauri"));
        assert_eq!(
            view.message(),
            Some("Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC.")
        );
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(0.0))
        );
        assert_eq!(view.shown_position(), Point::new(100.0, 200.0));
        view.tick(FADE_IN_FOR);
        assert_eq!(view.jump_effect(), None, "the fade is over");
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(
            view.course_map().current(),
            Some(SystemId(131)),
            "the map follows"
        );
    }

    #[test]
    fn the_ship_flies_under_a_hypergates_fade_in() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeIn(0.0))
        );
        let arrived = player(&view);
        assert_eq!(arrived.position, Vec2::new(100.0, 200.0));
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), stepped(arrived, THRUST, 3));
        assert_ne!(player(&view).position, arrived.position, "it flies");
        assert!(translucent_fade(&drawn(&view)), "still fading in");
    }

    #[test]
    fn a_hypergate_whose_gates_system_is_hidden_is_picked_by_the_one_at_its_place() {
        let mut view = FlightView::new(FakeCatalog {
            reborn: true,
            ..gated()
        });
        change_galaxy(&mut view, hiding(131));
        view.tick(TICK);
        land_now(&mut view);
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        let reborn = on_map(&view, 133);
        click(&mut view, reborn);
        tap(&mut view, MAP);
        assert_eq!(system_of(&view), SystemId(133));
        assert_eq!(player(&view).position, Vec2::new(100.0, 200.0), "at 310");
        assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Reborn"));
    }

    #[test]
    fn escape_on_the_hypergate_map_enters_the_gate_too() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        view.close_map();
        assert_eq!(system_of(&view), SystemId(131));
    }

    #[test]
    fn closing_the_hypergate_map_without_a_pick_cancels() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Course);
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
        assert_eq!(view.jump_effect(), None);
        assert_eq!(nav_target(&view), None);
        // The map is a course map again.
        tap(&mut view, MAP);
        assert_eq!(view.course_map().mode(), MapMode::Course);
        tap(&mut view, MAP);
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED), "nothing new");
        assert_lasts(&mut view, HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS);
    }

    #[test]
    fn closing_the_map_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
        // The course map, closed by its key or as Escape does; then the
        // hypergate map, cancelled and entered (`_PlayerEnterHypergate`).
        for (catalog, pick, by_key) in [
            (catalog(), false, true),
            (catalog(), false, false),
            (gated(), false, true),
            (gated(), true, true),
        ] {
            let gate = !catalog.gates.is_empty();
            let mut view = FlightView::new(catalog);
            ticks(&mut view, 3);
            if gate {
                land_now(&mut view);
                assert_eq!(view.course_map().mode(), MapMode::Hypergate);
            } else {
                tap(&mut view, MAP);
            }
            assert!(view.map_open());
            if pick {
                let alpha = on_map(&view, 131);
                click(&mut view, alpha);
            }
            let stale = RadarScene::default();
            assert_ne!(view.radar_now(), stale);
            view.radar_scene = Some(stale);
            let cadence = view.hud_cadence;
            if by_key {
                tap(&mut view, MAP);
            } else {
                view.close_map();
            }
            let case = format!("gate {gate}, pick {pick}, by key {by_key}");
            assert!(!view.map_open(), "{case}");
            assert_eq!(
                view.hud_cadence, cadence,
                "neither timed nor flipped: {case}"
            );
            assert_eq!(*view.radar_shown(), view.radar_now(), "{case}");
        }
    }

    #[test]
    fn without_hyperspace_effects_a_hypergate_flashes_white() {
        let mut view = FlightView::new(gated()).with_hyperspace_effects(false);
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0))
        );
        view.tick(ARRIVAL_FLASH_FOR);
        assert_eq!(view.jump_effect(), None);
    }

    #[test]
    fn a_hypergate_without_links_does_nothing() {
        let mut catalog = gated();
        catalog.gates[0].links = [None; 8];
        let mut view = FlightView::new(catalog);
        tap(&mut view, LAND);
        let cleared = view.message().map(str::to_owned);
        tap(&mut view, LAND);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), cleared.as_deref(), "nothing said");
        assert_eq!(nav_target(&view), None);
    }

    #[test]
    fn a_hypergate_whose_links_lead_nowhere_opens_an_empty_map_that_cancels() {
        let mut catalog = gated();
        catalog.gates[0].links[0] = Some(StellarId(999));
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert!(view.map_open());
        assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        assert_ne!(view.message(), Some(HYPERGATE_CANCELLED), "not yet");
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    }

    #[test]
    fn l_twice_at_a_wormhole_passes_through_it_with_a_flash() {
        let mut view = FlightView::new(holed());
        land_now(&mut view);
        assert!(!view.map_open());
        assert_eq!(system_of(&view), SystemId(131));
        assert_eq!(player(&view).position, Vec2::new(-300.0, 40.0));
        assert_eq!(
            view.message(),
            Some("Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.")
        );
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0))
        );
        let mut plain = FlightView::new(holed()).with_hyperspace_effects(false);
        land_now(&mut plain);
        assert_eq!(
            plain.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0)),
            "whatever the preference"
        );
        let passed =
            "Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.";
        assert_lasts(&mut plain, passed, 240);
    }

    #[test]
    fn coming_out_of_a_hypergate_says_so_for_360_ticks() {
        let mut view = FlightView::new(gated());
        land_now(&mut view);
        let alpha = on_map(&view, 131);
        click(&mut view, alpha);
        tap(&mut view, MAP);
        let exited = "Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC.";
        assert_lasts(&mut view, exited, 360);
    }

    #[test]
    fn a_wormhole_with_no_exit_says_why() {
        let mut catalog = holed();
        catalog.gates.truncate(1);
        let mut view = FlightView::new(catalog);
        land_now(&mut view);
        assert_eq!(system_of(&view), SystemId(130));
        assert_eq!(view.jump_effect(), None);
        let why =
            "Your ship is unable to enter this wormhole - the radiation levels are too extreme.";
        assert_lasts(&mut view, why, GATE_REFUSAL_TICKS);
    }

    /// The sprites of ship 129's sheet (`rlëD` 2001) drawn.
    fn ship_129_sprites(list: &DrawList) -> usize {
        sprites(list)
            .iter()
            .filter(|(image, _)| image.id == 2001)
            .count()
    }

    #[test]
    fn an_attacker_following_through_a_wormhole_is_not_drawn_while_it_emerges_but_is_on_radar() {
        let mut catalog = FakeCatalog {
            sites: holed().sites,
            gates: holed().gates,
            ..trafficked(&[130, 131], 0, 129, 3)
        };
        catalog.dudes[0].1.govt = Some(GovtId(140));
        catalog.govts = vec![nova_sim::GovtRecord {
            id: GovtId(140),
            flags: 0,
            flags2: nova_sim::govt::ALWAYS_FOLLOWS_THROUGH_WORMHOLE,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties::default(),
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
        }];
        // One ship of düde 128 (ship 129) attacking the player and
        // following it (`ShipSyst` -6).
        catalog.missions = vec![MissionRecord {
            ships: MissionShip {
                count: 1,
                syst: -6,
                dude: Some(nova_sim::DudeId(128)),
                behav: 0,
                start: 0,
                ..MissionShip::NONE
            },
            ..errand(140)
        }];
        let mut view = FlightView::new(catalog).with_behaviour(Rc::new(Still));
        run_set(&mut view, "S140");
        settle_session(&mut view);
        land_now(&mut view);
        assert_eq!(system_of(&view), SystemId(131));
        view.tick(TICK);
        let session = view.session().expect("flying");
        assert!(
            session.npcs().iter().any(Npc::emerging),
            "the attacker emerges after the player"
        );
        let list = drawn(&view);
        assert_eq!(
            sprites(&list),
            [
                (ImageKey::sprite(1140, 0), at(812.0, 344.0)),
                (ImageKey::sprite(2000, 0), VIEW_CENTER),
            ],
            "Proxima and the player, not the ship emerging"
        );
        // Once the radar has refreshed, the ship is on it, still emerging.
        ticks(&mut view, 8);
        let session = view.session().expect("flying");
        assert!(session.npcs().iter().any(Npc::emerging));
        assert_eq!(blips(&view).len(), 1, "on radar while it emerges");
        while view
            .session()
            .expect("flying")
            .npcs()
            .iter()
            .any(Npc::emerging)
        {
            view.tick(TICK);
        }
        assert_eq!(ship_129_sprites(&drawn(&view)), 1, "drawn once out");
    }

    // The radar's IFF, density scanner, centre arrow and static.

    /// IFF, a density scanner and anti-interference 30 as outfits 300-302.
    fn radar_outfits() -> Vec<OutfitRecord> {
        let outfit = |id: i16, mod_type: i16, mod_val: i16| {
            let mut mods = [(0, 0); 4];
            mods[1] = (mod_type, mod_val);
            OutfitRecord {
                id: OutfitId(id),
                name: format!("Outfit {id}"),
                short_name: String::new(),
                disp_weight: 0,
                mass: 0,
                tech_level: 1,
                max: 10,
                flags: 0,
                cost: 1,
                mods,
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: nova_sim::Test::default(),
                on_purchase: nova_sim::Script::default(),
                on_sell: nova_sim::Script::default(),
                item_class: 0,
                lc_name: String::new(),
                lc_plural: String::new(),
            }
        };
        vec![
            outfit(300, nova_sim::iff::MOD_IFF, 0),
            outfit(301, nova_sim::iff::MOD_DENSITY_SCANNER, 0),
            outfit(302, nova_sim::iff::MOD_ANTI_INTERFERENCE, 30),
        ]
    }

    /// `catalog`, its ship carrying `defaults` of [`radar_outfits`].
    fn outfitted(catalog: FakeCatalog, defaults: &[(i16, u16)]) -> FakeCatalog {
        FakeCatalog {
            outfits: radar_outfits(),
            defaults: defaults.iter().map(|&(id, n)| (OutfitId(id), n)).collect(),
            ..catalog
        }
    }

    #[test]
    fn an_iff_outfit_turns_on_the_black_backdrop_and_the_cyan_player() {
        let mut view = FlightView::new(outfitted(catalog(), &[(300, 1)]));
        ticks(&mut view, 1);
        assert!(view.radar_shown().iff);
        let marks = radar_marks(&view);
        assert_eq!(
            marks[0],
            RadarMark::Fill {
                rect: STOCK_RADAR,
                color: RadarColor::Black
            }
        );
        assert!(marks.contains(&RadarMark::Pixel {
            x: 926,
            y: 96,
            color: RadarColor::Cyan
        }));
        // Earth and the Moon, landable to a clean record: yellow.
        let stellars: Vec<StellarIff> = view.radar_shown().stellars.iter().map(|s| s.iff).collect();
        assert_eq!(stellars, [StellarIff::Cleared, StellarIff::Cleared]);
        let list = drawn(&view);
        let black = list.iter().any(|command| {
            matches!(command, DrawCommand::Line { color, .. } if *color == Color::rgba(0, 0, 0, 255))
        });
        assert!(black, "the black fill is drawn");
        for without in [outfitted(catalog(), &[]), outfitted(catalog(), &[(301, 1)])] {
            let mut view = FlightView::new(without);
            ticks(&mut view, 1);
            assert!(!view.radar_shown().iff);
            assert_eq!(radar_marks(&view).last(), Some(&PLAYER_PIXEL));
            assert!(
                !radar_marks(&view)
                    .iter()
                    .any(|m| matches!(m, RadarMark::Fill { .. }))
            );
        }
    }

    /// A trafficked flight with one idle NPC of ship 129, whose class
    /// weighs `mass`, at (100, -100), radar pixel (929, 92); the player
    /// carries `defaults`.
    fn with_heavy_npc(mass: i16, defaults: &[(i16, u16)]) -> View {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let catalog = FakeCatalog {
            hulls: vec![nova_sim::HullRecord {
                mass,
                ..hull_of(129, &[])
            }],
            ..outfitted(trafficked(&[130], 1, 129, 3), defaults)
        };
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        view
    }

    #[test]
    fn a_heavy_npc_shows_as_a_square_only_with_a_density_scanner() {
        let square = RadarMark::Square {
            rect: IRect::around(929, 92, 1),
            color: RadarColor::Dim,
        };
        let dot = RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Dim,
        };
        // The ships' marks: squares and pixels but the player's, last.
        let ships = |view: &View| {
            let mut marks: Vec<RadarMark> = radar_marks(view)
                .into_iter()
                .filter(|mark| matches!(mark, RadarMark::Pixel { .. } | RadarMark::Square { .. }))
                .collect();
            assert_eq!(marks.pop(), Some(PLAYER_PIXEL));
            marks
        };
        assert_eq!(ships(&with_heavy_npc(100, &[(301, 1)])), [square]);
        assert_eq!(ships(&with_heavy_npc(100, &[])), [dot]);
        assert_eq!(ships(&with_heavy_npc(99, &[(301, 1)])), [dot]);
    }

    #[test]
    fn a_ships_iff_code_reaches_the_radar() {
        let view = with_heavy_npc(0, &[(300, 1)]);
        let ships = &view.radar_shown().ships;
        assert_eq!(ships.len(), 1);
        assert_eq!(ships[0].iff, ShipIff::Other);
        assert!(!ships[0].heavy);
        assert!(radar_marks(&view).contains(&RadarMark::Pixel {
            x: 929,
            y: 92,
            color: RadarColor::Blue
        }));
    }

    #[test]
    fn a_plotted_course_hides_the_arrow_until_a_stellar_is_selected() {
        let mut view = flight();
        assert!(!view.radar_now().route_shown, "no course");
        plot(&mut view, 131);
        assert!(view.radar_now().route_shown);
        assert!(view.radar_shown().route_shown, "redrawn as the map closes");
        tap(&mut view, NEAREST_STELLAR_KEY);
        assert!(view.session().expect("flying").nav_target().is_some());
        assert!(!view.radar_now().route_shown, "a stellar selected");
    }

    /// Thrusts straight up until the ship is more than 2700 from the
    /// centre, then lets go of Up.
    fn fly_far(view: &mut View) {
        view.input(&key(Key::Up, true));
        for _ in 0..5000 {
            if player(view).position.length() > 2700.0 {
                break;
            }
            view.tick(TICK);
        }
        view.input(&key(Key::Up, false));
        assert!(player(view).position.length() > 2700.0);
    }

    fn arrow_lines(view: &View) -> usize {
        radar_marks(view)
            .iter()
            .filter(|mark| matches!(mark, RadarMark::Line { .. }))
            .count()
    }

    #[test]
    fn far_from_the_centre_the_arrow_blinks_with_the_target() {
        let mut view = flight();
        assert_eq!(arrow_lines(&view), 0, "near the centre");
        fly_far(&mut view);
        let mut shown = Vec::new();
        for _ in 0..32 {
            ticks(&mut view, 1);
            shown.push((view.hud_cadence.blink_on(), arrow_lines(&view)));
        }
        for &(blink_on, lines) in &shown {
            assert_eq!(lines, if blink_on { 3 } else { 0 }, "{shown:?}");
        }
        let flips = shown
            .windows(2)
            .filter(|pair| pair[0].0 != pair[1].0)
            .count();
        assert_eq!(flips, 4, "every 8 ticks: {shown:?}");
    }

    /// `catalog` with Sol's `Interference` at `level`, its effects rolled
    /// on `draws`.
    fn interfered(level: i16, draws: &[u32]) -> (Rc<RefCell<Script>>, View) {
        let (script, chance) = scripted(draws);
        let catalog = FakeCatalog {
            sol_interference: level,
            ..catalog()
        };
        (script, FlightView::new(catalog).with_effects_chance(chance))
    }

    #[test]
    fn interference_shows_static_at_the_next_refresh() {
        // The threshold roll 0 (1 ≤ 100), then pattern 3.
        let (script, mut view) = interfered(100, &[0, 3]);
        assert_eq!(view.radar_shown().static_pattern, None, "before a tick");
        ticks(&mut view, 1);
        assert_eq!(script.borrow().asked, [100, 10]);
        assert_eq!(view.radar_shown().static_pattern, Some(131));
        assert_eq!(
            radar_marks(&view)[0],
            RadarMark::Pattern {
                rect: STOCK_RADAR,
                id: 131
            }
        );
        let list = drawn(&view);
        assert!(list.iter().any(|command| *command
            == DrawCommand::TiledPicture {
                image: ImageKey::pattern(131),
                area: STOCK_RADAR.to_bounds(),
            }));
    }

    #[test]
    fn without_interference_no_static_shows_and_nothing_is_rolled() {
        let (script, mut view) = interfered(0, &[]);
        for _ in 0..20 {
            ticks(&mut view, 1);
            assert_eq!(view.radar_shown().static_pattern, None);
        }
        assert!(script.borrow().asked.is_empty());
    }

    #[test]
    fn anti_interference_lowers_the_level_static_is_rolled_against() {
        // 100 less 30: a roll of 70 (71 > 70) shows none.
        let (script, chance) = scripted(&[70]);
        let catalog = FakeCatalog {
            sol_interference: 100,
            ..outfitted(catalog(), &[(302, 1)])
        };
        let mut view = FlightView::new(catalog).with_effects_chance(chance);
        ticks(&mut view, 1);
        assert_eq!(script.borrow().asked, [100]);
        assert_eq!(view.radar_shown().static_pattern, None);
    }

    #[test]
    fn the_hud_cadence_and_blink_advance_through_a_jumps_streak() {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        jump_now(&mut view);
        let streaking = |view: &View| {
            matches!(
                view.jump_effect().map(JumpEffect::phase),
                Some(JumpPhase::Streak(_))
            )
        };
        assert!(streaking(&view));
        // Mark the snapshot, then tick through the streak: the radar is
        // taken afresh, and the blink flips, every 8 ticks.
        let marker = RadarScene {
            player: at(-1.0, -1.0),
            ..RadarScene::default()
        };
        let mut refreshes = Vec::new();
        let mut tick = 0;
        while streaking(&view) {
            view.radar_scene = Some(marker.clone());
            let blink = view.hud_cadence.blink_on();
            view.tick(TICK);
            tick += 1;
            let retaken = view.radar_scene.as_ref() != Some(&marker);
            assert_eq!(retaken, view.hud_cadence.blink_on() != blink, "tick {tick}");
            if retaken {
                refreshes.push(tick);
            }
        }
        assert!(tick >= 24, "the streak lasts a second: {tick}");
        assert!(refreshes.len() >= 3, "{refreshes:?}");
        let gaps: Vec<u32> = refreshes.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert!(gaps.iter().all(|&gap| gap == 8), "{refreshes:?}");
    }

    // The Red Alert.

    /// Quiet sound preferences: effects at level 1.
    const QUIET: SoundPrefs = SoundPrefs {
        sound: true,
        music: true,
        effects_level: 1,
        music_level: 7,
    };

    /// Two NPCs, NPC 0 turning on the player at the first tick after the
    /// flight's first (frame 0), sound `prefs`: the Red Alert's check at
    /// frame 60 is the 60th tick from here.
    fn threatened(prefs: SoundPrefs) -> View {
        fighting(armed(2, &[]), &two_npcs())
            .with_behaviour(Rc::new(Threatening))
            .with_sound_prefs(prefs)
    }

    /// The Red Alert `cïcn`s drawn, and where.
    fn icons_drawn(view: &View) -> Vec<(i16, Point)> {
        drawn(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(18000..=18001)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_red_alert_draws_cicn_18000_then_18001_at_the_playfields_top_right() {
        let mut view = threatened(QUIET);
        for tick in 1..60 {
            view.tick(TICK);
            assert_eq!(icons_drawn(&view), [], "tick {tick}: before the check");
        }
        let corner = Point::new(1024.0 - 194.0 - 24.0, 0.0);
        for tick in 0..160_usize {
            view.tick(TICK);
            let expected = match tick {
                146.. => vec![],
                1.. if ((tick - 1) / 5).is_multiple_of(2) => vec![(18001, corner)],
                _ => vec![(18000, corner)],
            };
            assert_eq!(icons_drawn(&view), expected, "tick {tick} of the blink");
        }
    }

    #[test]
    fn no_alert_icon_is_drawn_for_a_loud_player() {
        let mut view = threatened(SoundPrefs::default());
        for tick in 0..300 {
            view.tick(TICK);
            assert_eq!(icons_drawn(&view), [], "tick {tick}");
        }
        assert!(!view.alert_quiet());
    }

    #[test]
    fn the_sound_prefs_reach_the_session_when_set() {
        let mut view = threatened(SoundPrefs::default());
        view.set_sound_prefs(QUIET);
        assert!(view.alert_quiet());
        ticks(&mut view, 60);
        assert_eq!(icons_drawn(&view).len(), 1);
        view.set_sound_prefs(SoundPrefs {
            sound: false,
            ..SoundPrefs::default()
        });
        assert!(view.alert_quiet(), "sound off is quiet");
        view.set_sound_prefs(SoundPrefs::default());
        assert!(!view.alert_quiet());
    }

    #[test]
    fn the_red_alert_sounds_once_through_the_view() {
        let mut view = threatened(QUIET);
        view.take_sounds();
        let mut alerts = 0;
        for _ in 0..300 {
            view.tick(TICK);
            alerts += view
                .take_sounds()
                .iter()
                .filter(|&&sound| sound == Sound::Sim(SimSound::RedAlert))
                .count();
        }
        assert_eq!(alerts, 1);
    }

    #[test]
    fn the_alert_icon_is_drawn_over_the_brackets() {
        let mut view = threatened(QUIET);
        ticks(&mut view, 1);
        tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), Some(NpcId(0)));
        ticks(&mut view, 59);
        let list = drawn(&view);
        let commands: Vec<_> = list.iter().collect();
        let icon = commands
            .iter()
            .position(|command| icon_in(18000..=18001)(command))
            .expect("the icon is drawn");
        assert!(
            icon_in(10011..=10011)(commands[icon - 1]),
            "the brackets' bottom-left corner just before it: {:?}",
            commands[icon - 1]
        );
    }

    #[test]
    fn a_missing_alert_icon_draws_nothing_and_says_so_once() {
        let catalog = FakeCatalog {
            alert_icon: None,
            ..armed(2, &[])
        };
        let (_, chance) = scripted(&two_npcs());
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Threatening))
            .with_sound_prefs(QUIET);
        let missing =
            Diagnostic::Unreadable("no cïcn 18000: the Red Alert icon is not drawn".to_owned());
        let told = |view: &mut View| {
            let diagnostics = view.take_diagnostics();
            diagnostics.iter().filter(|&d| *d == missing).count()
        };
        assert_eq!(told(&mut view), 1);
        let mut blinked = false;
        for _ in 0..200 {
            view.tick(TICK);
            blinked |= view.session().expect("flying").red_alert_icon().is_some();
            assert_eq!(icons_drawn(&view), []);
        }
        assert!(blinked, "the session blinked it");
        assert_eq!(told(&mut view), 0, "once");
        assert_eq!(told(&mut flight()), 0, "the default catalog has it");
    }
}
