//! The flight screen's player-facing message text.

use nova_sim::hire::{DEFECTED_ONE, DEFECTED_SOME};
use nova_sim::mission::screens::NO_ACTIVE_MISSIONS;
use nova_sim::text_tags::comma_number;
use nova_sim::{
    Assigned, BoardRefusal, Chance, CommNote, DockKind, GateKind, Good, HailRefusal, Help,
    JumpRefusal, LandingRefusal, OutfitId, SelfDestructLine, Session, ShipId, SimMessage,
    StellarId, Taken,
};
use nova_sim::{MissionBoardLine, PayNote, RuleSource, TagNames, TutorialHint};

use crate::Key;
use crate::flight::escorts::NUMBER_WORDS;
use crate::spaceport::credits::{TON, TONS};
use crate::system::scene::SystemScene;
use crate::ui::comm::grouped;

use super::{JUMP_KEY, LAND_KEY, MAP_KEY};

/// How many ticks each message stays on the message line: the frames its
/// caller passes `_DisplayComm` (@0x90201), which counts them down once
/// a frame in `_HandleComm` (@0x301b9). The original's callers and their
/// times are recorded in rdm phase hud-fidelity/phase-6-message-line;
/// those the engine does not raise yet are rdm phase
/// hud-fidelity/phase-12-flight-messages-the-original-raises's. The
/// session's own messages carry their time ([`SimMessage::ticks`]), as
/// do the ships' comm notes and the hail quotes it raises
/// ([`CommNote::ticks`](nova_sim::CommNote::ticks),
/// [`PersonQuote::ticks`](nova_sim::PersonQuote::ticks)).
///
/// [`MESSAGE_TICKS`], 240, is the original's common time: "No
/// response." (`_HandlePlayerCommunication` @0x626c8), "You have no
/// active missions." (`_HandlePlayer` @0x705a8), the plunder dialog's
/// lines (`_DoPlunderDialog` @0x936df-0x9498d) and
/// the abandoned fighters on arrival (@0x6c46d). It is also the time of
/// the messages the original does not have ([`TAKE_OFF_FIRST`],
/// [`REPELLED`]) and of the defection notice, which the original
/// shows in a dialog (`_DoEscortPayment` @0x40301).
pub const MESSAGE_TICKS: u16 = 240;
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
/// A landing denied or not possible: 250 (`_HandlePlayerDockRequest`
/// @0x67ef3; the not-landable line @0x67cda-0x67e5c).
pub const LANDING_TICKS: u16 = 250;
/// A landing refused as too far or too fast, at a gate as at a station
/// or a planet: 360 (`_HandlePlayerDockRequest` @0x677c2-0x67838,
/// @0x6787f).
pub const TOO_FAR_TICKS: u16 = 360;
/// A hypergate jump cancelled, or a wormhole leading nowhere: 250
/// (`_PlayerEnterHypergate` @0x6397e, `_PlayerEnterWormhole` @0x646a3).
pub const GATE_REFUSAL_TICKS: u16 = 250;
/// A jump refused as too close, held by a tractor beam or with no
/// destination: 360 (`_HandlePlayer` @0x6b931, @0x6ba00).
pub const JUMP_REFUSAL_TICKS: u16 = 360;
/// The capture's old ship retained or lost: 360 (`_DoShipCapture`
/// @0x41aa1).
pub const OLD_SHIP_TICKS: u16 = 360;

/// How long the message for `refusal` stays: [`JUMP_REFUSAL_TICKS`] too
/// close, held by a tractor beam or with no destination, 240 short of fuel (`_HandlePlayer`
/// @0x6baac), and [`MESSAGE_TICKS`] for the engine's own refusals. A
/// disabled ship's refusal shows no message ([`jump_refusal_message`]),
/// so its time is never used.
#[must_use]
pub fn jump_refusal_ticks(refusal: &JumpRefusal) -> u16 {
    match refusal {
        JumpRefusal::TooClose { .. } | JumpRefusal::Tractored | JumpRefusal::NoDestination => {
            JUMP_REFUSAL_TICKS
        }
        JumpRefusal::NoFuel { .. } | JumpRefusal::Landed | JumpRefusal::Disabled => MESSAGE_TICKS,
    }
}

/// How long the message for `refusal` stays: [`TOO_FAR_TICKS`] too far
/// or too fast, [`LANDING_TICKS`] denied or not landable, and
/// [`MESSAGE_TICKS`] for the silent ones. A disabled ship's refusal, and
/// one where the land key makes no dock request (jumping, no stellars,
/// nothing landable), shows no message ([`refusal_message`]), so its
/// time is never used.
#[must_use]
pub fn landing_refusal_ticks(refusal: &LandingRefusal) -> u16 {
    match refusal {
        LandingRefusal::TooFar { .. } | LandingRefusal::TooFast { .. } => TOO_FAR_TICKS,
        LandingRefusal::Denied { .. } | LandingRefusal::NotLandable { .. } => LANDING_TICKS,
        LandingRefusal::Jumping
        | LandingRefusal::Disabled
        | LandingRefusal::NoStellars
        | LandingRefusal::NothingLandable => MESSAGE_TICKS,
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

/// `STR#` 2002 #284: a mission failed as its cargo was jettisoned, as
/// one of its special ships was lost, or as the player's ship was
/// disabled or broke up.
pub const MISSION_FAILED: &str = "Mission failed.";
/// `STR#` 2002 #287: the player's ship was newly disabled (`_DamageShip`
/// @0x3b181).
pub const SHIP_DISABLED: &str = "Ship disabled.";
/// `STR#` 2002 #288: the player's ship newly started to break up
/// (`_DamageShip` @0x3b3b9).
pub const SHIP_DESTROYED: &str = "Ship destroyed.";
/// `STR#` 2002 #289: everything was jettisoned.
pub const CARGO_JETTISONED: &str = "Cargo jettisoned.";
/// `STR#` 2002 #290: all but the mission cargo was jettisoned.
pub const NON_MISSION_CARGO_JETTISONED: &str = "Non-mission cargo jettisoned.";
/// `STR#` 2002 #127: a disabled former escort repaired itself and
/// rejoined the fleet (`_HandleShip` @0x33b5a).
pub const ESCORT_REPAIRED: &str = "Escort repaired.";
/// `STR#` 2002 #128: a disabled former fighter did (`_HandleShip`
/// @0x33b3b).
pub const FIGHTER_REPAIRED: &str = "Fighter repaired.";
/// `STR#` 2002 #37: the disabled player's Repair Droids repaired it
/// (`_HandlePlayer` @0x68bf0-0x68c3f).
pub const PROPULSION_REPAIRED: &str = "Propulsion systems repaired.";
/// `STR#` 2002 #168: a former escort a pirate took (`_AIPlunderShipContents`
/// @0x8482c).
pub const ESCORT: &str = "Escort";
/// `STR#` 2002 #169: a former fighter a pirate took (@0x84822).
pub const FIGHTER: &str = "Fighter";
/// `STR#` 2002 #373: what a pirate stole (@0x84665).
pub const CARGO: &str = "cargo";
/// `STR#` 2002 #374: how a plunder's lines end (@0x84719, @0x848c9).
pub const STOLEN: &str = "stolen!";
/// `STR#` 2002 #391: between the tons and the cargo (@0x8463f).
pub const OF: &str = "of";
/// `STR#` 2002 #392: between the cargo and the credits (@0x84698).
pub const AND: &str = "and";

/// What the message line says as a pirate steals `tons` of cargo and
/// `credits`, as `_AIPlunderShipContents` builds it (@0x84593-0x84731):
/// the tons, "ton" at exactly 1 and "tons" otherwise, "of cargo", then
/// with credits too "and", the credits, "credit" at 1 and "credits"
/// otherwise, then "stolen!"; each number as `_StrcatCommaNumber` writes
/// it, from a million as `millions` reads
/// ([`RuleKey::PayMillions`](nova_sim::RuleKey::PayMillions)).
#[must_use]
pub fn cargo_stolen_message(tons: u32, credits: i64, millions: RuleSource) -> String {
    let mut words = Vec::new();
    if tons > 0 {
        let unit = if tons == 1 { TON } else { TONS };
        words.extend([comma_number(i64::from(tons), millions), unit.to_owned()]);
        words.extend([OF, CARGO].map(str::to_owned));
        if credits > 0 {
            words.push(AND.to_owned());
        }
    }
    if credits > 0 {
        let unit = if credits > 1 { "credits" } else { "credit" };
        words.extend([comma_number(credits, millions), unit.to_owned()]);
    }
    words.push(STOLEN.to_owned());
    words.join(" ")
}

/// What the message line says of a plunder's `message`, the numbers'
/// millions as `session`'s rules read them; none for any other message.
fn stolen_text(message: &SimMessage, session: &Session) -> Option<String> {
    let millions = session.rules().source_for(nova_sim::RuleKey::PayMillions);
    match message {
        SimMessage::CargoStolen { tons, credits } => {
            Some(cargo_stolen_message(*tons, *credits, millions))
        }
        SimMessage::ShipStolen { fighter } => Some(ship_stolen_message(*fighter)),
        _ => None,
    }
}

/// What the message line says as a pirate takes a former escort, or one
/// the player leads, into its fleet (@0x84815-0x848dc): "Escort stolen!",
/// or "Fighter stolen!" for a former fighter.
#[must_use]
pub fn ship_stolen_message(fighter: bool) -> String {
    let ship = if fighter { FIGHTER } else { ESCORT };
    format!("{ship} {STOLEN}")
}
/// `STR#` 2002 #52: the player ejected into an escape ship, whose long
/// name follows (`_HandlePlayer` @0x6f595-0x6f5a8).
pub const ESCAPING_IN: &str = "Escaping in";
/// `STR#` 2002 #38: a sabotage kill outfit went off with none named
/// (`_HandlePlayer` @0x69645-0x6965e).
pub const BOMB_EXPLODED_ONBOARD: &str = "A bomb has exploded onboard your ship!";
/// The `l33t` mode's words for a kill outfit with no outfit named, the
/// string at 0xd9962 (`_HandlePlayer` @0x6962d-0x6963e).
pub const BOMB_SET_UP: &str = "Someone set up us the bomb!";
/// `STR#` 2002 #35: a disabled player's pre-jump collapsed
/// (`_HandlePlayer` `_GetIndString` @0x68a04).
pub const JUMP_COLLAPSED: &str = "Ship disabled - hyperspace field collapsed.";
/// `STR#` 2002 #385: what each of the self-destruct's lines but the last
/// begins with (`_HandlePlayer` @0x6fe3f, @0x6ff8a, @0x700ea).
pub const SELF_DESTRUCT_LEAD: &str = "Self-destruct";
/// `STR#` 2002 #386: the self-destruct began (@0x6fe65).
pub const SEQUENCE_INITIATED: &str = "sequence initiated.";
/// `STR#` 2002 #387: the self-destruct was let go (@0x70110).
pub const SEQUENCE_CANCELLED: &str = "sequence cancelled.";
/// `STR#` 2002 #395: the countdown line's "in" (@0x6ffb0).
pub const SELF_DESTRUCT_IN: &str = "in";
/// `STR#` 2002 #388: "second", which the original never says; the
/// countdown's last line says it only by the other reading of
/// `nova_sim::RuleKey::SelfDestructSecond`.
pub const SECOND: &str = "second";
/// `STR#` 2002 #389: the countdown line's "seconds" (@0x70014).
pub const SECONDS: &str = "seconds";
/// `STR#` 2002 #390: the self-destruct's last words (@0x70203).
pub const HAVE_A_NICE_DAY: &str = "Have a nice day.";
/// `STR#` 2002 #39: what comes before a sabotage outfit that went off.
pub const YOUR: &str = "Your";
/// `STR#` 2002 #40: what follows one sabotage outfit that went off.
pub const HAS_EXPLODED: &str = "has exploded!";
/// `STR#` 2002 #41: what follows several.
pub const HAVE_EXPLODED: &str = "have exploded!";

/// What the message line says as sabotage outfits go off, as
/// `_HandlePlayer` builds it (@0x6940b-0x69578, @0x69701-0x69871): "Your",
/// a space, `lc_name` and "has exploded!" for `one`, else `lc_plural`
/// and "have exploded!".
#[must_use]
pub fn exploded_message(one: bool, lc_name: &str, lc_plural: &str) -> String {
    if one {
        format!("{YOUR} {lc_name} {HAS_EXPLODED}")
    } else {
        format!("{YOUR} {lc_plural} {HAVE_EXPLODED}")
    }
}

/// `STR#` 2002 #286: a scan found a mission's cargo and failed it.
pub const MISSION_SCANNED: &str = "Your ship has been scanned - mission failed.";
/// `STR#` 2002 #285: a mission's deadline ran out in flight and failed it.
pub const TIME_LIMIT_EXCEEDED: &str = "Time limit exceeded - mission failed.";
/// `STR#` 2002 #375, as the strings spell it: what comes before an
/// illegal outfit a scan finds.
pub const POSSESSION_OF: &str = "your posession of";
/// `STR#` 2002 #376: what comes before the smuggled cargo.
pub const ATTEMPT_TO_SMUGGLE: &str = "your attempt to smuggle";
/// `STR#` 2002 #377: a mission's smuggled cargo, unnamed.
pub const ILLEGAL_CARGO: &str = "illegal cargo";
/// `STR#` 2002 #378: what follows the smuggled cargo.
pub const THROUGH_THIS_SYSTEM: &str = "through this system";
/// `STR#` 2002 #379: how a scan's notice ends, before its fine.
pub const HAS_BEEN_DETECTED: &str = "has been detected";
/// `STR#` 2002 #380: what comes before the fine.
pub const FINED: &str = "You have been fined";

/// `detected`, a scan's notice, with the fine after it as `_ScanPlayer`
/// adds it: none for no fine, else " You have been fined `fine`
/// credits." ("credit" for 1), the fine's thousands grouped
/// (@0x7e934-0x7ea09).
fn with_fine(detected: String, fine: i64) -> String {
    if fine == 0 {
        return detected;
    }
    let credits = if fine > 1 { "credits" } else { "credit" };
    format!("{detected} {FINED} {} {credits}.", grouped(fine))
}

/// What the message line says when a scan finds smuggling, as
/// `_ScanPlayer` builds it (@0x7e7ce-0x7ea9b): "`scanner`:  `ship`, ...
/// has been detected.", then with a fine " You have been fined `fine`
/// credits." ("credit" for 1), the fine's thousands grouped. The
/// scanner is named by the drawn ship name of the mission whose cargo
/// the scan found, when it has one, whatever ship scanned, and otherwise
/// by its ship class (@0x7e7ce-0x7e7ef; see [`SimMessage::Smuggling`]).
#[must_use]
pub fn smuggling_message(scanner: &str, ship: &str, fine: i64) -> String {
    illegal_junk_message(scanner, ship, ILLEGAL_CARGO, fine)
}

/// What the message line says when a scan finds an illegal `jünk` named
/// `lc_name`, as `_ScanPlayer` builds it (@0x7ec29-0x7ee3b):
/// "`scanner`:  `ship`, your attempt to smuggle `lc_name` through this
/// system has been detected.", and the fine as [`smuggling_message`]
/// adds it. The scanner is named by its person's name when it is one,
/// else by its ship class (see [`SimMessage::IllegalJunk`]).
#[must_use]
pub fn illegal_junk_message(scanner: &str, ship: &str, lc_name: &str, fine: i64) -> String {
    with_fine(
        format!(
            "{scanner}:  {ship}, {ATTEMPT_TO_SMUGGLE} {lc_name} {THROUGH_THIS_SYSTEM} \
             {HAS_BEEN_DETECTED}."
        ),
        fine,
    )
}

/// What the message line says when a scan finds an illegal outfit, as
/// `_ScanPlayer` builds it (@0x7f0dd-0x7f2d7): "`scanner`:  `ship`, your
/// posession of", then for `one` owned [`ARTICLE_AN`] or [`ARTICLE_A`]
/// by `lc_name`'s first letter ([`article`]) and `lc_name`, else
/// `lc_plural` with no count, then "has been detected.", and the fine
/// as [`smuggling_message`] adds it.
#[must_use]
pub fn illegal_outfit_message(
    scanner: &str,
    ship: &str,
    one: bool,
    lc_name: &str,
    lc_plural: &str,
    fine: i64,
) -> String {
    let named = if one {
        format!("{} {lc_name}", article(lc_name))
    } else {
        lc_plural.to_owned()
    };
    with_fine(
        format!("{scanner}:  {ship}, {POSSESSION_OF} {named} {HAS_BEEN_DETECTED}."),
        fine,
    )
}

/// The ejected player's line: "Escaping in", a space, the escape ship's
/// long name and a full stop (`_HandlePlayer` @0x6f5ad-0x6f5f3).
fn escaping_in_text(ship: ShipId, session: &Session) -> String {
    let name = session.ship_long_name(ship).unwrap_or_default();
    format!("{ESCAPING_IN} {name}.")
}

/// The self-destruct's lines (`_HandlePlayer` block Z): each but the last
/// is "Self-destruct" (#385) and a space before its own words.
fn self_destruct_text(line: SelfDestructLine) -> String {
    match line {
        SelfDestructLine::Initiated => format!("{SELF_DESTRUCT_LEAD} {SEQUENCE_INITIATED}"),
        SelfDestructLine::In { seconds, singular } => {
            let unit = if singular { SECOND } else { SECONDS };
            format!("{SELF_DESTRUCT_LEAD} {SELF_DESTRUCT_IN} {seconds} {unit}.")
        }
        SelfDestructLine::Cancelled => format!("{SELF_DESTRUCT_LEAD} {SEQUENCE_CANCELLED}"),
        SelfDestructLine::NiceDay => HAVE_A_NICE_DAY.to_owned(),
    }
}

/// What the message line says of `message`, a session's message other
/// than an arrival's, the names read from `session`: none for an
/// arrival's, which [`arrival_lead`] leads the arrival message with, and
/// for the take-off greeting, the land hint, the dock request's line and
/// the dock approach's greeting, which [`message_text`] words with the
/// stellar's name.
pub(super) fn sim_message_text(message: &SimMessage, session: &Session) -> Option<String> {
    let text = match message {
        SimMessage::Arrived(_)
        | SimMessage::Buoy(_)
        | SimMessage::ExitedHypergate(_)
        | SimMessage::PassedWormhole(_)
        | SimMessage::TookOff { .. }
        | SimMessage::DockRequested { .. }
        | SimMessage::DockCleared { .. }
        | SimMessage::Tutorial(TutorialHint::Land { .. }) => {
            return None;
        }
        SimMessage::NoResponse => NO_RESPONSE,
        SimMessage::Tutorial(TutorialHint::Hyperspace) => {
            return Some(hyperspace_hint(
                &key_label(MAP_KEY).unwrap_or_default(),
                &key_label(JUMP_KEY).unwrap_or_default(),
            ));
        }
        SimMessage::Tutorial(TutorialHint::FarEnough) => FAR_ENOUGH,
        SimMessage::Tutorial(TutorialHint::NothingOutHere) => NOTHING_OUT_HERE,
        SimMessage::MissionFailed
        | SimMessage::MissionShipLost
        | SimMessage::DownedMissionFailed => MISSION_FAILED,
        SimMessage::ShipDisabled => SHIP_DISABLED,
        SimMessage::ShipDestroyed => SHIP_DESTROYED,
        SimMessage::CargoJettisoned => CARGO_JETTISONED,
        SimMessage::NonMissionCargoJettisoned => NON_MISSION_CARGO_JETTISONED,
        SimMessage::MissionScanned => MISSION_SCANNED,
        SimMessage::DeadlineMissed => TIME_LIMIT_EXCEEDED,
        SimMessage::NoActiveMissions => NO_ACTIVE_MISSIONS,
        SimMessage::EscortRepaired => ESCORT_REPAIRED,
        SimMessage::FighterRepaired => FIGHTER_REPAIRED,
        SimMessage::PropulsionRepaired => PROPULSION_REPAIRED,
        SimMessage::CargoStolen { .. } | SimMessage::ShipStolen { .. } => {
            return stolen_text(message, session);
        }
        SimMessage::FleetArrivalQuote(text)
        | SimMessage::CallForHelp(text)
        | SimMessage::ScanHail(text)
        | SimMessage::ReinforcementsApproaching(text) => return Some(text.clone()),
        SimMessage::JumpCollapsed => JUMP_COLLAPSED,
        SimMessage::SelfDestruct(line) => return Some(self_destruct_text(*line)),
        SimMessage::EscapingIn { ship } => return Some(escaping_in_text(*ship, session)),
        SimMessage::Sabotaged { outfit: None } => BOMB_EXPLODED_ONBOARD,
        SimMessage::BombSetUp => BOMB_SET_UP,
        SimMessage::Sabotaged {
            outfit: Some((outfit, one)),
        }
        | SimMessage::BombExploded { outfit, one } => {
            let (lc_name, lc_plural) = session.outfit_names(*outfit).unwrap_or_default();
            return Some(exploded_message(*one, lc_name, lc_plural));
        }
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
        SimMessage::IllegalJunk {
            scanner,
            person,
            junk,
            fine,
        } => {
            return Some(illegal_junk_message(
                scanner_name(person.as_deref(), *scanner, session),
                session.pilot().ship_name().unwrap_or_default(),
                session.junk_lc_name(*junk).unwrap_or_default(),
                *fine,
            ));
        }
        SimMessage::IllegalOutfit {
            scanner,
            person,
            outfit,
            one,
            fine,
        } => {
            let (lc_name, lc_plural) = session.outfit_names(*outfit).unwrap_or_default();
            return Some(illegal_outfit_message(
                scanner_name(person.as_deref(), *scanner, session),
                session.pilot().ship_name().unwrap_or_default(),
                *one,
                lc_name,
                lc_plural,
                *fine,
            ));
        }
    };
    Some(text.to_owned())
}

/// What a scan's contraband notice calls the scanner: its person's name
/// when it is one, else its ship class's name (`_ScanPlayer`
/// @0x7ebd1-0x7ec0c, @0x7f024-0x7f05f).
fn scanner_name<'a>(person: Option<&'a str>, scanner: ShipId, session: &'a Session) -> &'a str {
    person
        .or_else(|| session.ship_name(scanner))
        .unwrap_or_default()
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
pub(super) fn mission_board_message(line: MissionBoardLine, session: &Session) -> String {
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

/// `STR#` 2002 #29.
pub const NO_DESTINATION: &str =
    "You have to select a destination before you can start a hyperspace jump.";
/// `STR#` 2002 #42.
pub const TOO_CLOSE: &str =
    "Can't initiate hyperspace jump - not yet far enough away from system center.";
/// `STR#` 2002 #36, shown in place of #42 while another ship holds the
/// player in a tractor beam (`_HandlePlayer` @0x6b8a4-0x6b8dc).
pub const TRACTORED: &str = "Your ship is unable to enter hyperspace while held by a tractor beam.";
/// `STR#` 2002 #10.
pub const NO_FUEL: &str = "Insufficient energy for hyperspace jump.";
/// Not the original's, which never flies a landed ship: worded after
/// `STR#` 2002 #42 and #73 ("Disengage cloaking device first.").
pub const TAKE_OFF_FIRST: &str = "Can't initiate hyperspace jump - take off first.";

/// `STR#` 2002 #22: the land hint's lead for a station.
pub const TUTORIAL_DOCK: &str = "Welcome to Nova - it would be a good idea to start by docking at";
/// `STR#` 2002 #23: the land hint's lead for a planet.
pub const TUTORIAL_LAND: &str = "Welcome to Nova - it would be a good idea to start by landing on";
/// `STR#` 2002 #24: after the stellar's name in the land hint.
pub const TUTORIAL_LOOK_AROUND: &str = "and having a look around. Hit '";
/// `STR#` 2002 #25: after the land key's name.
pub const TUTORIAL_REQUEST: &str = "' to request landing clearance, then hit it again to land.";
/// `STR#` 2002 #26: the hyperspace hint's lead.
pub const TUTORIAL_HYPERSPACE: &str = "Now you should probably hyperspace to another system. Hit '";
/// `STR#` 2002 #27: after the map key's name.
pub const TUTORIAL_MAP: &str =
    "' to access the map, select a nearby system, then move outward and hit '";
/// `STR#` 2002 #28: after the jump key's name.
pub const TUTORIAL_JUMP: &str = "' to begin your jump.";
/// `STR#` 2002 #30.
pub const FAR_ENOUGH: &str =
    "You're well beyond safe hyperspace range - you can initiate hyperjump at any time.";
/// `STR#` 2002 #31.
pub const NOTHING_OUT_HERE: &str = "There's nothing to find out here - if you want to get to another system, you'll have to access the map, set a hyperdrive destination, and start your jump.";
/// `STR#` 2002 #55-#59: the take-off greeting's leads, one rolled
/// `Rand(5)` (`_PlayerLandOnStellar` @0x63604).
pub const TAKE_OFF_LEADS: [&str; 5] = [
    "Launching from",
    "Blasting off from",
    "Taking off from",
    "Leaving",
    "Departing",
];
/// `STR#` 2002 #60: between the stellar and the date in the greeting.
pub const TAKE_OFF_ON: &str = "on";

/// How a tutorial hint names `key`, as the original names its keys from
/// `STR#` 129 (`_HandlePlayer` @0x70b01-0x70b1b, @0x70c3d-0x70cb2): a
/// letter key uppercased, "L". The hints name only [`LAND_KEY`],
/// [`MAP_KEY`] and [`JUMP_KEY`], all letters; the other keys are named
/// once keys can be rebound, so for now they have no name.
#[must_use]
pub fn key_label(key: Key) -> Option<String> {
    match key {
        Key::Char(letter) => Some(letter.to_uppercase().collect()),
        _ => None,
    }
}

/// What the message line says as the ship takes off past the tutorial
/// from the stellar named `stellar` on `date`, led by
/// [`TAKE_OFF_LEADS`]`[lead]`, as `_PlayerLandOnStellar` builds it
/// (@0x635f7-0x63732): "Taking off from Earth on 23 June 1177."
#[must_use]
pub fn take_off_greeting(lead: u16, stellar: &str, date: &str) -> String {
    let lead = TAKE_OFF_LEADS[usize::from(lead) % TAKE_OFF_LEADS.len()];
    format!("{lead} {stellar} {TAKE_OFF_ON} {date}.")
}

/// The tutorial's land hint for the stellar named `stellar`, a `station`
/// or not, naming the land key `key`, as `_HandlePlayer` builds it
/// (@0x70a31-0x70b9a): [`TUTORIAL_DOCK`] or [`TUTORIAL_LAND`], the name,
/// [`TUTORIAL_LOOK_AROUND`], the key and [`TUTORIAL_REQUEST`].
#[must_use]
pub fn land_hint(station: bool, stellar: &str, key: &str) -> String {
    let lead = if station {
        TUTORIAL_DOCK
    } else {
        TUTORIAL_LAND
    };
    format!("{lead} {stellar} {TUTORIAL_LOOK_AROUND}{key}{TUTORIAL_REQUEST}")
}

/// The tutorial's hyperspace hint, naming the map key `map` and the jump
/// key `jump`, as `_HandlePlayer` builds it (@0x70c3d-0x70d33).
#[must_use]
pub fn hyperspace_hint(map: &str, jump: &str) -> String {
    format!("{TUTORIAL_HYPERSPACE}{map}{TUTORIAL_MAP}{jump}{TUTORIAL_JUMP}")
}

/// Texts to say, each with its ticks.
pub(super) type Said = Vec<(String, u16)>;

/// The name of stellar `id` in `scene`, if it shows it: empty otherwise.
pub(super) fn stellar_name(scene: Option<&SystemScene>, id: StellarId) -> &str {
    scene
        .and_then(|scene| scene.stellars().iter().find(|named| named.id == id))
        .map_or("", |named| named.name.as_str())
}

/// What the message line says of `message`, the names read from
/// `session` and the stellars' from `scene`: as [`sim_message_text`]
/// says, and the take-off greeting, the land hint, the dock request's
/// line and the dock approach's greeting, which name a stellar, the
/// line's and the greeting's picks rolled on `chance`. None only for an
/// arrival's.
pub(super) fn message_text(
    message: &SimMessage,
    session: &Session,
    scene: Option<&SystemScene>,
    chance: &mut dyn Chance,
) -> Option<String> {
    let ship = || session.pilot().ship_name().unwrap_or_default();
    match message {
        SimMessage::DockRequested { stellar, kind } => Some(request_line(
            *kind,
            stellar_name(scene, *stellar),
            ship(),
            RequestPicks::roll(*kind, chance),
        )),
        SimMessage::DockCleared { stellar, kind } => Some(approach_greeting(
            *kind,
            stellar_name(scene, *stellar),
            ship(),
            GreetingPicks::roll(*kind, chance),
        )),
        SimMessage::TookOff { stellar, lead } => Some(take_off_greeting(
            *lead,
            stellar_name(scene, *stellar),
            &session.date_text(),
        )),
        SimMessage::Tutorial(TutorialHint::Land { stellar, station }) => Some(land_hint(
            *station,
            stellar_name(scene, *stellar),
            &key_label(LAND_KEY).unwrap_or_default(),
        )),
        _ => sim_message_text(message, session),
    }
}

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
/// fighters-abandoned suffix follows it as a note of its own
/// ([`FlightView::arrived`](super::FlightView::arrived)), and a system's message buoy replaces the
/// whole line ([`SimMessage::Buoy`]).
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
pub(super) fn arrival_lead(message: &SimMessage) -> Option<&'static str> {
    match message {
        SimMessage::Arrived(_) => Some(JUMPING_INTO),
        SimMessage::ExitedHypergate(_) => Some(EXITING_HYPERGATE),
        SimMessage::PassedWormhole(_) => Some(PASSING_WORMHOLE),
        SimMessage::MissionFailed
        | SimMessage::MissionShipLost
        | SimMessage::ShipDisabled
        | SimMessage::ShipDestroyed
        | SimMessage::DownedMissionFailed
        | SimMessage::CargoJettisoned
        | SimMessage::NonMissionCargoJettisoned
        | SimMessage::MissionScanned
        | SimMessage::DeadlineMissed
        | SimMessage::NoActiveMissions
        | SimMessage::EscortRepaired
        | SimMessage::FighterRepaired
        | SimMessage::PropulsionRepaired
        | SimMessage::CargoStolen { .. }
        | SimMessage::ShipStolen { .. }
        | SimMessage::FleetArrivalQuote(_)
        | SimMessage::CallForHelp(_)
        | SimMessage::ScanHail(_)
        | SimMessage::ReinforcementsApproaching(_)
        | SimMessage::JumpCollapsed
        | SimMessage::SelfDestruct(_)
        | SimMessage::EscapingIn { .. }
        | SimMessage::Sabotaged { .. }
        | SimMessage::BombSetUp
        | SimMessage::BombExploded { .. }
        | SimMessage::Smuggling { .. }
        | SimMessage::IllegalJunk { .. }
        | SimMessage::IllegalOutfit { .. }
        | SimMessage::Buoy(_)
        | SimMessage::TookOff { .. }
        | SimMessage::DockRequested { .. }
        | SimMessage::NoResponse
        | SimMessage::DockCleared { .. }
        | SimMessage::Tutorial(_) => None,
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
/// `STR#` 2002 #65: too far to enter a hypergate (@0x67802-0x67838).
pub const TOO_FAR_HYPERGATE: &str = "You're too far away to enter this hypergate.";
/// `STR#` 2002 #66: too far to enter a wormhole (@0x67802-0x67838).
pub const TOO_FAR_WORMHOLE: &str = "You're too far away to enter this wormhole.";
/// `STR#` 2002 #67.
pub const TOO_FAR_STATION: &str = "You're too far away to dock at this station.";
/// `STR#` 2002 #68.
pub const TOO_FAR_PLANET: &str = "You're too far away to land on this planet.";
/// `STR#` 2002 #71.
pub const TOO_FAST_STATION: &str = "You're moving too fast to dock at this station.";
/// `STR#` 2002 #72.
pub const TOO_FAST_PLANET: &str = "You're moving too fast to land on this planet.";
/// `STR#` 2002 #69: too fast to enter a hypergate (@0x677c2-0x67800).
pub const TOO_FAST_HYPERGATE: &str = "You're moving too fast to enter this hypergate.";
/// `STR#` 2002 #70: too fast to enter a wormhole (@0x677c2-0x67800).
pub const TOO_FAST_WORMHOLE: &str = "You're moving too fast to enter this wormhole.";
/// `STR#` 2002 #87: the not-landable line's verb at a station, ending
/// in a space (@0x67d4c).
pub const DOCK_AT: &str = "dock at ";
/// `STR#` 2002 #88: the not-landable line's verb at a planet, ending in
/// a space (@0x67d56).
pub const LAND_ON: &str = "land on ";
/// `STR#` 2002 #82.
pub const DOCKING_DENIED: &str = "Docking request denied.";
/// `STR#` 2002 #83.
pub const LANDING_DENIED: &str = "Landing request denied.";
/// `STR#` 2002 #89: why a ship cannot dock at a station.
pub const HOSTILE_STATION: &str = "The station's hull integrity is too unstable.";
/// `STR#` 2002 #90: why a ship cannot land on a planet.
pub const HOSTILE_PLANET: &str = "The planet's environment is too hostile.";

/// `STR#` 2002 #76.
pub const DOCKMASTER_READS_YOU: &str = "dockmaster reads you";
/// `STR#` 2002 #77.
pub const DOCKING_REQUEST_RECEIVED: &str = "Docking request received";
/// `STR#` 2002 #78.
pub const TRAFFIC_CONTROL_READS_YOU: &str = "traffic control reads you";
/// `STR#` 2002 #79.
pub const LANDING_REQUEST_RECEIVED: &str = "Landing request received";
/// `STR#` 2002 #91.
pub const CLEARED_FOR_HYPERGATE: &str = "Cleared for hypergate entry";
/// `STR#` 2002 #92, its trailing space the stock string's own.
pub const YOURE_CLEARED_FOR_HYPERGATE: &str = "you're cleared for hypergate entry. ";
/// `STR#` 2002 #93, its trailing space the stock string's own.
pub const YOU_ARE_CLEARED_FOR_HYPERGATE: &str = "You are cleared for hypergate entry. ";
/// `STR#` 2002 #94.
pub const CLEARED_TO_DOCK_LEAD: &str = "Cleared to dock";
/// `STR#` 2002 #95.
pub const CLEARED_TO_DOCK: &str = "you're cleared to dock.";
/// `STR#` 2002 #96.
pub const YOU_ARE_CLEARED_TO_DOCK: &str = "You are cleared to dock.";
/// `STR#` 2002 #97.
pub const CLEARED_TO_LAND_LEAD: &str = "Cleared to land";
/// `STR#` 2002 #98.
pub const CLEARED_TO_LAND: &str = "you're cleared to land.";
/// `STR#` 2002 #99.
pub const YOU_ARE_CLEARED_TO_LAND: &str = "You are cleared to land.";
/// `STR#` 2002 #100.
pub const COMMENCE_FINAL_APPROACH: &str = "Commence final approach.";
/// `STR#` 2002 #101.
pub const WELCOME_TO: &str = "Welcome to";

/// The picks that word the dock request's line ([`request_line`]): the
/// rolls the original makes for it (`_HandlePlayerDockRequest`
/// @0x67a8b-0x67bda).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestPicks {
    /// The opening's roll: `Rand(2)` at a hypergate, 0 for
    /// [`HYPERGATE_ENERGIZED`] (#74) and otherwise [`HYPERGATE_ONLINE`]
    /// (#75); `Rand(3)` elsewhere, 0 for the stellar's name and "…
    /// reads you" (#76, #78) and otherwise "… request received" (#77,
    /// #79).
    pub opening: u16,
    /// `Rand(2)`: 0 adds ", " and the ship's name.
    pub named: u16,
}

impl RequestPicks {
    /// The picks for a stellar of `kind`, rolled on `chance`.
    pub fn roll(kind: DockKind, chance: &mut (impl Chance + ?Sized)) -> Self {
        let sides = if kind == DockKind::Hypergate { 2 } else { 3 };
        let opening = chance.roll(sides);
        Self {
            opening,
            named: chance.roll(2),
        }
    }
}

/// The picks that word the dock approach's greeting
/// ([`approach_greeting`]): the rolls the original makes for it
/// (`_HandlePlayerDockApproach` @0x649de, @0x64ac1, @0x64bbf, @0x64cc6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GreetingPicks {
    /// `Rand(3)`, which of the three "cleared" forms.
    pub opening: u16,
    /// `Rand(2)`: 0 for [`COMMENCE_FINAL_APPROACH`], otherwise
    /// [`WELCOME_TO`] and the stellar's name. A hypergate's greeting rolls
    /// none and has no second sentence.
    pub welcome: u16,
}

impl GreetingPicks {
    /// The picks for a stellar of `kind`, rolled on `chance`.
    pub fn roll(kind: DockKind, chance: &mut (impl Chance + ?Sized)) -> Self {
        let opening = chance.roll(3);
        let welcome = if kind == DockKind::Hypergate {
            0
        } else {
            chance.roll(2)
        };
        Self { opening, welcome }
    }
}

/// The dock request's line at the stellar named `stellar`, of `kind`, for
/// the ship named `ship`, as `_HandlePlayerDockRequest` builds it
/// (@0x67a8b-0x67c05): the opening by `picks`, then ", " and the ship's
/// name when `picks.named` is 0, then ". " and [`BEGIN_APPROACH`]. For
/// example "Earth traffic control reads you, Kestrel. Begin initial
/// approach."
#[must_use]
pub fn request_line(kind: DockKind, stellar: &str, ship: &str, picks: RequestPicks) -> String {
    let opening = match kind {
        DockKind::Hypergate if picks.opening == 0 => HYPERGATE_ENERGIZED.to_owned(),
        DockKind::Hypergate => HYPERGATE_ONLINE.to_owned(),
        DockKind::Station if picks.opening == 0 => format!("{stellar} {DOCKMASTER_READS_YOU}"),
        DockKind::Station => DOCKING_REQUEST_RECEIVED.to_owned(),
        DockKind::Planet if picks.opening == 0 => format!("{stellar} {TRAFFIC_CONTROL_READS_YOU}"),
        DockKind::Planet => LANDING_REQUEST_RECEIVED.to_owned(),
    };
    let named = if picks.named == 0 {
        format!(", {ship}")
    } else {
        String::new()
    };
    format!("{opening}{named}. {BEGIN_APPROACH}")
}

/// The dock approach's greeting at the stellar named `stellar`, of `kind`,
/// for the ship named `ship`, as `_HandlePlayerDockApproach` builds it
/// (@0x649cd-0x64d6c), trailing spaces included. By `picks.opening`: 0
/// gives #91/#94/#97, ", ", the ship's name and ". "; 1 the ship's name,
/// ", " and #92/#95/#98; 2 #93/#96/#99. A station's or a planet's then
/// gets " " (@0x64cb9); a hypergate's #92/#93 end in their own space, and
/// its greeting ends there (@0x64a59, @0x64e4d). Otherwise `picks.welcome` 0 adds
/// [`COMMENCE_FINAL_APPROACH`], and any other [`WELCOME_TO`], " ", the
/// stellar's name and ". ". For example "Kestrel, you're cleared to land.
/// Welcome to Earth. ". The fee line that may follow is
/// rdm:task/landing-fee's.
#[must_use]
pub fn approach_greeting(
    kind: DockKind,
    stellar: &str,
    ship: &str,
    picks: GreetingPicks,
) -> String {
    let (lead, cleared, you_are, space) = match kind {
        DockKind::Hypergate => (
            CLEARED_FOR_HYPERGATE,
            YOURE_CLEARED_FOR_HYPERGATE,
            YOU_ARE_CLEARED_FOR_HYPERGATE,
            "",
        ),
        DockKind::Station => (
            CLEARED_TO_DOCK_LEAD,
            CLEARED_TO_DOCK,
            YOU_ARE_CLEARED_TO_DOCK,
            " ",
        ),
        DockKind::Planet => (
            CLEARED_TO_LAND_LEAD,
            CLEARED_TO_LAND,
            YOU_ARE_CLEARED_TO_LAND,
            " ",
        ),
    };
    let opening = match picks.opening {
        0 => format!("{lead}, {ship}. "),
        1 => format!("{ship}, {cleared}{space}"),
        _ => format!("{you_are}{space}"),
    };
    if kind == DockKind::Hypergate {
        opening
    } else if picks.welcome == 0 {
        format!("{opening}{COMMENCE_FINAL_APPROACH}")
    } else {
        format!("{opening}{WELCOME_TO} {stellar}. ")
    }
}

/// What the player is told when `refusal` stops a landing at a stellar
/// that is the `gate` kind, if any, named `name`: the original's words
/// for it, as `_HandlePlayerDockRequest` picks them; nothing for a
/// disabled ship, which it refuses silently (`_IsDisabled` @0x66af7, the
/// bare epilogue 0x67eef), and nothing either when the land key makes no
/// dock request: jumping (@0x6a8cb → 0x6a95d), or nothing selected and
/// nothing to select (@0x6a94c).
///
/// - Too far and too fast (@0x677c2-0x67838): a hypergate's or a
///   wormhole's own string, tested before the station flag, else the
///   station's or the planet's.
/// - Not landable (@0x67cda-0x67e2b): a gate's [`gate_refusal_message`];
///   else [`UNABLE_TO`], " ", [`DOCK_AT`] or [`LAND_ON`], " " again
///   (@0x67d94, so two spaces before the name), the name, ". " and
///   [`HOSTILE_STATION`] or [`HOSTILE_PLANET`].
/// - Denied (@0x67ea3-0x67eea): [`HYPERGATE_DENIED`] at a hypergate only;
///   a wormhole says the station's or the planet's.
#[must_use]
pub fn refusal_message(
    refusal: &LandingRefusal,
    gate: Option<GateKind>,
    name: &str,
) -> Option<String> {
    let pick = |station: bool, at_station: &'static str, on_planet: &'static str| {
        if station { at_station } else { on_planet }
    };
    let by_gate = |station, hypergate, wormhole, at_station, on_planet| match gate {
        Some(GateKind::Hypergate) => hypergate,
        Some(GateKind::Wormhole) => wormhole,
        None => pick(station, at_station, on_planet),
    };
    let text = match *refusal {
        LandingRefusal::Jumping
        | LandingRefusal::Disabled
        | LandingRefusal::NoStellars
        | LandingRefusal::NothingLandable => return None,
        LandingRefusal::TooFar { station, .. } => by_gate(
            station,
            TOO_FAR_HYPERGATE,
            TOO_FAR_WORMHOLE,
            TOO_FAR_STATION,
            TOO_FAR_PLANET,
        ),
        LandingRefusal::TooFast { station, .. } => by_gate(
            station,
            TOO_FAST_HYPERGATE,
            TOO_FAST_WORMHOLE,
            TOO_FAST_STATION,
            TOO_FAST_PLANET,
        ),
        LandingRefusal::NotLandable { station, .. } => {
            if let Some(kind) = gate {
                return Some(gate_refusal_message(kind));
            }
            let verb = pick(station, DOCK_AT, LAND_ON);
            let why = pick(station, HOSTILE_STATION, HOSTILE_PLANET);
            return Some(format!("{UNABLE_TO} {verb} {name}. {why}"));
        }
        LandingRefusal::Denied { station, .. } => {
            if gate == Some(GateKind::Hypergate) {
                HYPERGATE_DENIED
            } else {
                pick(station, DOCKING_DENIED, LANDING_DENIED)
            }
        }
    };
    Some(text.to_owned())
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
        // A miss, or a take with nothing on board, says nothing: it only
        // beeps (`_DoPlunderDialog` @0x93ad2, @0x93cb8, @0x93d2f, @0x94256).
        Taken::Captured | Taken::Aborted | Taken::Nothing | Taken::Missed => return None,
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

/// The article before one `lc_name`, as `_DoPlunderDialog` and
/// `_ScanPlayer` pick it (@0x7f115-0x7f150): [`ARTICLE_AN`] when its first
/// letter, lowercased, is a vowel, else [`ARTICLE_A`].
#[must_use]
pub fn article(lc_name: &str) -> &'static str {
    let vowel = lc_name
        .chars()
        .next()
        .is_some_and(|first| "aeiou".contains(first.to_ascii_lowercase()));
    if vowel { ARTICLE_AN } else { ARTICLE_A }
}

/// What the player is told of a grant of `count` outfits named
/// `lc_name`, or `lc_plural` for more than one, as `_DoPlunderDialog`
/// says it (@0x9322b-0x933ce): [`RETRIEVED`]; for one, [`ARTICLE_AN`]
/// when the name's first letter, lowercased, is a vowel, else
/// [`ARTICLE_A`]; for two to ten the count in words ([`NUMBER_WORDS`]),
/// and above that in digits; the name; and [`FROM_THIS_SHIP`].
#[must_use]
pub fn grant_message(count: u16, lc_name: &str, lc_plural: &str) -> String {
    let (number, name) = if count == 1 {
        (article(lc_name).to_owned(), lc_name)
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
/// plunder dialog ([`FlightView::plunder_shown`](super::FlightView::plunder_shown)) and the messages
/// ([`FlightView::plunder`](super::FlightView::plunder)).
pub(super) fn names(
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
pub(super) fn pay_said(notes: &[PayNote]) -> Vec<(String, u16)> {
    pay_notes_message(notes)
        .into_iter()
        .map(|text| (text, MESSAGE_TICKS))
        .collect()
}

/// What the player is told when `refusal` stops a jump: the original's
/// words for it, or none for a disabled ship, whose J the original
/// ignores (`_HandlePlayer` @0x6b0f5 → 0x6bada).
#[must_use]
pub fn jump_refusal_message(refusal: &JumpRefusal) -> Option<&'static str> {
    match refusal {
        JumpRefusal::NoDestination => Some(NO_DESTINATION),
        JumpRefusal::TooClose { .. } => Some(TOO_CLOSE),
        JumpRefusal::Tractored => Some(TRACTORED),
        JumpRefusal::NoFuel { .. } => Some(NO_FUEL),
        JumpRefusal::Landed => Some(TAKE_OFF_FIRST),
        JumpRefusal::Disabled => None,
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::NpcId;

    use super::*;

    #[test]
    fn a_plunders_cargo_line_reads_as_the_original() {
        let engine = RuleSource::Engine;
        assert_eq!(
            cargo_stolen_message(12, 0, engine),
            "12 tons of cargo stolen!"
        );
        assert_eq!(
            cargo_stolen_message(1, 1, engine),
            "1 ton of cargo and 1 credit stolen!"
        );
        assert_eq!(
            cargo_stolen_message(0, 1500, engine),
            "1,500 credits stolen!"
        );
        assert_eq!(
            cargo_stolen_message(2, 1_234_567, engine),
            "2 tons of cargo and 1.23M credits stolen!"
        );
        assert_eq!(
            cargo_stolen_message(0, 1_234_567, RuleSource::Bible),
            "1,234,567 credits stolen!"
        );
        assert_eq!(
            cargo_stolen_message(1200, 0, engine),
            "1,200 tons of cargo stolen!"
        );
    }

    #[test]
    fn a_stolen_ships_line_names_an_escort_or_a_fighter() {
        assert_eq!(ship_stolen_message(false), "Escort stolen!");
        assert_eq!(ship_stolen_message(true), "Fighter stolen!");
    }

    #[test]
    fn every_pick_of_the_request_line_reads_as_the_original() {
        let line = |kind, opening, named| {
            request_line(kind, "Earth", "Kestrel", RequestPicks { opening, named })
        };
        for (kind, opening, named, text) in [
            (
                DockKind::Planet,
                0,
                0,
                "Earth traffic control reads you, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Planet,
                0,
                1,
                "Earth traffic control reads you. Begin initial approach.",
            ),
            (
                DockKind::Planet,
                1,
                0,
                "Landing request received, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Planet,
                2,
                1,
                "Landing request received. Begin initial approach.",
            ),
            (
                DockKind::Station,
                0,
                0,
                "Earth dockmaster reads you, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Station,
                1,
                1,
                "Docking request received. Begin initial approach.",
            ),
            (
                DockKind::Station,
                2,
                0,
                "Docking request received, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Hypergate,
                0,
                0,
                "Hypergate is energized, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Hypergate,
                1,
                0,
                "Hypergate is online, Kestrel. Begin initial approach.",
            ),
            (
                DockKind::Hypergate,
                1,
                1,
                "Hypergate is online. Begin initial approach.",
            ),
        ] {
            assert_eq!(
                line(kind, opening, named),
                text,
                "{kind:?} {opening} {named}"
            );
        }
    }

    #[test]
    fn every_pick_of_the_greeting_reads_as_the_original() {
        let greeting = |kind, opening, welcome| {
            approach_greeting(kind, "Earth", "Kestrel", GreetingPicks { opening, welcome })
        };
        for (kind, opening, welcome, text) in [
            (
                DockKind::Planet,
                0,
                0,
                "Cleared to land, Kestrel. Commence final approach.",
            ),
            (
                DockKind::Planet,
                1,
                1,
                "Kestrel, you're cleared to land. Welcome to Earth. ",
            ),
            (
                DockKind::Planet,
                2,
                0,
                "You are cleared to land. Commence final approach.",
            ),
            (
                DockKind::Planet,
                0,
                1,
                "Cleared to land, Kestrel. Welcome to Earth. ",
            ),
            (
                DockKind::Station,
                0,
                0,
                "Cleared to dock, Kestrel. Commence final approach.",
            ),
            (
                DockKind::Station,
                1,
                0,
                "Kestrel, you're cleared to dock. Commence final approach.",
            ),
            (
                DockKind::Station,
                2,
                1,
                "You are cleared to dock. Welcome to Earth. ",
            ),
            (
                DockKind::Hypergate,
                0,
                0,
                "Cleared for hypergate entry, Kestrel. ",
            ),
            (
                DockKind::Hypergate,
                1,
                1,
                "Kestrel, you're cleared for hypergate entry. ",
            ),
            (
                DockKind::Hypergate,
                2,
                1,
                "You are cleared for hypergate entry. ",
            ),
        ] {
            assert_eq!(
                greeting(kind, opening, welcome),
                text,
                "{kind:?} {opening} {welcome}"
            );
        }
    }

    #[test]
    fn the_words_are_the_strings_the_original_reads() {
        assert_eq!(
            [
                DOCKMASTER_READS_YOU,
                DOCKING_REQUEST_RECEIVED,
                TRAFFIC_CONTROL_READS_YOU,
                LANDING_REQUEST_RECEIVED,
                BEGIN_APPROACH,
                CLEARED_FOR_HYPERGATE,
                YOURE_CLEARED_FOR_HYPERGATE,
                YOU_ARE_CLEARED_FOR_HYPERGATE,
                CLEARED_TO_DOCK_LEAD,
                CLEARED_TO_DOCK,
                YOU_ARE_CLEARED_TO_DOCK,
                CLEARED_TO_LAND_LEAD,
                CLEARED_TO_LAND,
                YOU_ARE_CLEARED_TO_LAND,
                COMMENCE_FINAL_APPROACH,
                WELCOME_TO,
            ],
            [
                "dockmaster reads you",
                "Docking request received",
                "traffic control reads you",
                "Landing request received",
                "Begin initial approach.",
                "Cleared for hypergate entry",
                "you're cleared for hypergate entry. ",
                "You are cleared for hypergate entry. ",
                "Cleared to dock",
                "you're cleared to dock.",
                "You are cleared to dock.",
                "Cleared to land",
                "you're cleared to land.",
                "You are cleared to land.",
                "Commence final approach.",
                "Welcome to",
            ]
        );
    }

    #[test]
    fn each_refusal_and_assignment_lasts_its_callers_ticks() {
        let stellar = StellarId(128);
        let landing = [
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
            (JumpRefusal::Tractored, 360),
            (JumpRefusal::NoFuel { fuel: 1.0 }, 240),
            (JumpRefusal::Landed, 240),
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
            [240, 240, 250, 360, 250, 500, 250, 360, 250, 360, 360]
        );
    }

    #[test]
    fn every_refusal_has_its_string() {
        let stellar = StellarId(128);
        let cases = [
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
            assert_eq!(
                refusal_message(&refusal, None, "Earth").as_deref(),
                Some(text),
                "{refusal:?}"
            );
        }
        assert_eq!(
            refusal_message(&LandingRefusal::Disabled, None, "Earth"),
            None,
            "silent (@0x66af7, 0x67eef)"
        );
        for (refusal, at) in [
            (LandingRefusal::Jumping, "@0x6a95d"),
            (LandingRefusal::NoStellars, "@0x6a94c"),
            (LandingRefusal::NothingLandable, "@0x6a94c"),
        ] {
            assert_eq!(
                refusal_message(&refusal, None, "Earth"),
                None,
                "no dock request, silent ({at})"
            );
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
            ]
        );
    }

    #[test]
    fn too_far_at_a_gate_says_its_own_words() {
        let stellar = StellarId(128);
        for station in [true, false] {
            let refusal = LandingRefusal::TooFar { stellar, station };
            assert_eq!(
                refusal_message(&refusal, Some(GateKind::Hypergate), "x").as_deref(),
                Some(TOO_FAR_HYPERGATE)
            );
            assert_eq!(
                refusal_message(&refusal, Some(GateKind::Wormhole), "x").as_deref(),
                Some(TOO_FAR_WORMHOLE)
            );
            let plain = if station {
                TOO_FAR_STATION
            } else {
                TOO_FAR_PLANET
            };
            assert_eq!(refusal_message(&refusal, None, "x").as_deref(), Some(plain));
        }
        assert_eq!(
            [TOO_FAR_HYPERGATE, TOO_FAR_WORMHOLE],
            [
                "You're too far away to enter this hypergate.",
                "You're too far away to enter this wormhole.",
            ]
        );
    }

    #[test]
    fn too_fast_at_a_gate_says_its_own_words() {
        let stellar = StellarId(128);
        for station in [true, false] {
            let refusal = LandingRefusal::TooFast {
                stellar,
                station,
                speed: 2.0,
            };
            assert_eq!(
                refusal_message(&refusal, Some(GateKind::Hypergate), "x").as_deref(),
                Some(TOO_FAST_HYPERGATE)
            );
            assert_eq!(
                refusal_message(&refusal, Some(GateKind::Wormhole), "x").as_deref(),
                Some(TOO_FAST_WORMHOLE)
            );
            let plain = if station {
                TOO_FAST_STATION
            } else {
                TOO_FAST_PLANET
            };
            assert_eq!(refusal_message(&refusal, None, "x").as_deref(), Some(plain));
        }
        assert_eq!(
            [TOO_FAST_HYPERGATE, TOO_FAST_WORMHOLE],
            [
                "You're moving too fast to enter this hypergate.",
                "You're moving too fast to enter this wormhole.",
            ]
        );
    }

    #[test]
    fn not_landable_at_a_station_or_planet_leads_with_its_name() {
        let stellar = StellarId(128);
        let station = LandingRefusal::NotLandable {
            stellar,
            station: true,
        };
        let planet = LandingRefusal::NotLandable {
            stellar,
            station: false,
        };
        assert_eq!(
            refusal_message(&station, None, "Earth Station").as_deref(),
            Some(
                "Your ship is unable to dock at  Earth Station. \
                 The station's hull integrity is too unstable."
            )
        );
        assert_eq!(
            refusal_message(&planet, None, "Earth").as_deref(),
            Some("Your ship is unable to land on  Earth. The planet's environment is too hostile.")
        );
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            for refusal in [&station, &planet] {
                assert_eq!(
                    refusal_message(refusal, Some(kind), "Earth"),
                    Some(gate_refusal_message(kind))
                );
            }
        }
        assert_eq!([DOCK_AT, LAND_ON], ["dock at ", "land on "]);
    }

    #[test]
    fn denied_at_a_hypergate_says_hypergate_usage_denied() {
        let stellar = StellarId(128);
        for station in [true, false] {
            let refusal = LandingRefusal::Denied {
                stellar,
                station,
                min_status: 1,
            };
            assert_eq!(
                refusal_message(&refusal, Some(GateKind::Hypergate), "x").as_deref(),
                Some(HYPERGATE_DENIED)
            );
            let plain = if station {
                DOCKING_DENIED
            } else {
                LANDING_DENIED
            };
            for gate in [Some(GateKind::Wormhole), None] {
                assert_eq!(refusal_message(&refusal, gate, "x").as_deref(), Some(plain));
            }
        }
    }

    #[test]
    fn every_jump_refusal_has_its_string() {
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoDestination),
            Some(NO_DESTINATION)
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::TooClose { distance: 1.0 }),
            Some(TOO_CLOSE)
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::NoFuel { fuel: 1.0 }),
            Some(NO_FUEL)
        );
        assert_eq!(
            jump_refusal_message(&JumpRefusal::Landed),
            Some(TAKE_OFF_FIRST)
        );
        assert_eq!(jump_refusal_message(&JumpRefusal::Disabled), None);
        assert_eq!(
            jump_refusal_message(&JumpRefusal::Tractored),
            Some(TRACTORED)
        );
        assert_eq!(
            [
                NO_DESTINATION,
                TOO_CLOSE,
                TRACTORED,
                NO_FUEL,
                TAKE_OFF_FIRST
            ],
            [
                "You have to select a destination before you can start a hyperspace jump.",
                "Can't initiate hyperspace jump - not yet far enough away from system center.",
                "Your ship is unable to enter hyperspace while held by a tractor beam.",
                "Insufficient energy for hyperspace jump.",
                "Can't initiate hyperspace jump - take off first.",
            ]
        );
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

    #[test]
    fn an_illegal_junk_notice_names_it_in_place_of_illegal_cargo() {
        assert_eq!(
            illegal_junk_message("Viper", "Kestrel", "bio-weapons", 0),
            "Viper:  Kestrel, your attempt to smuggle bio-weapons through this system has \
             been detected."
        );
        assert_eq!(
            illegal_junk_message("Viper", "Kestrel", "bio-weapons", 1),
            "Viper:  Kestrel, your attempt to smuggle bio-weapons through this system has \
             been detected. You have been fined 1 credit."
        );
        assert_eq!(
            illegal_junk_message("Viper", "Kestrel", "water", 4500),
            "Viper:  Kestrel, your attempt to smuggle water through this system has been \
             detected. You have been fined 4,500 credits."
        );
    }

    #[test]
    fn an_illegal_outfit_notice_names_one_with_its_article_and_more_in_the_plural() {
        let notice = |one, lc_name, fine| {
            illegal_outfit_message("Viper", "Kestrel", one, lc_name, "blasters", fine)
        };
        assert_eq!(
            notice(true, "blaster", 0),
            "Viper:  Kestrel, your posession of a blaster has been detected."
        );
        assert_eq!(
            notice(true, "Ion cannon", 2),
            "Viper:  Kestrel, your posession of an Ion cannon has been detected. You have \
             been fined 2 credits.",
            "the first letter lowercased"
        );
        for vowel in ["a", "e", "i", "o", "u"] {
            assert!(notice(true, vowel, 0).contains(" an "), "{vowel}");
        }
        assert_eq!(
            notice(false, "blaster", 1),
            "Viper:  Kestrel, your posession of blasters has been detected. You have been \
             fined 1 credit.",
            "no count"
        );
    }

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
}
