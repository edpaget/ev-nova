//! Hailing a ship: the comm dialog's conversation, its options, and what
//! they make the ship say and do.
//!
//! The player hails its target (see `nova_sim::Session::hail`);
//! the ship answers with a ship comm string ([`reply`]) chosen by its
//! [`Attitude`] ([`like`]), and a [`Conversation`] rolls its variant,
//! mood, price and advice once ([`deal`]). The dialog lists the
//! [`HailOption`]s that apply, Nova's four by default ([`nova`]:
//! Greetings, Request Assistance and Beg For Mercy, and Release for the
//! player's escort). Each option sees the
//! conversation as a read-only [`Hail`] and answers a press with an
//! [`Answer`]: the [`Reply`] the ship says, a [`Deed`] it does, and a
//! price it asks ([`Ask`]), which the player may haggle over. The session
//! turns replies into words and does the deeds, so an option needs no
//! catalog and no session code; adding one means registering it in
//! [`HailOptions`] at the edge. Hailing a ship the player leads (a hired
//! or captured escort, a launched fighter, or a mission escort), the
//! dialog also shows its status ([`EscortStatus`]), a hired escort's
//! daily pay among it, as the original's comm dialog does for its
//! "Hired Escort", until Release is pressed.
//!
//! **What a ship's government and type make of hails** ([`Dispositions`],
//! `_HandlePlayerCommunication` @0x61f48 and `_DoCommDialog`
//! @0x9576d-0x9586c in the `EV Nova` executable), from its `gövt`'s
//! `Flags` and `Flags2`:
//!
//! - *mute* (no hail is answered, "No response."): `Flags` 0x0400 of its
//!   government or of its ship type's inherent government;
//! - *greedy* (a higher price, see [`deal`]): `Flags` 0x8000;
//! - *bribable*: an independent ship always; a trader when its
//!   government has `Flags` 0x2000; a warship or interceptor when it has
//!   0x0200;
//! - *untalkative* (Greetings alone, "No response."): `Flags2` 0x0001;
//! - *quiet* (see [`RuleKey::QuietHails`](crate::RuleKey::QuietHails)):
//!   `Flags2` 0x0008 of its government or its inherent government;
//! - *free help* ("Roadside Assistance"): `Flags2` 0x0010, or a rank of
//!   the pilot's with `Flags` 0x0800 that reaches the ship's government;
//! - *battle help*: a rank of the pilot's with `Flags` 0x0400 that
//!   reaches the ship's government (see [`favour`](crate::ai::favour),
//!   by [`RuleKey::RankGovtReach`](crate::RuleKey::RankGovtReach));
//! - *plunderer*: a warship or interceptor of `Flags` 0x1000;
//! - *xenophobe*: `Flags` 0x0001.
//!
//! The Bible agrees on each, or is read the engine's way without a stated
//! disagreement: 0x0010's "always … for free" means for free, and the
//! ship must still like the player and be free.

pub mod advice;
pub mod assist;
pub mod battle;
pub mod deal;
pub mod like;
pub mod nova;
pub mod reply;

use std::fmt::Debug;
use std::rc::Rc;

use crate::ai::threatened::player_threatened_by_enemy_of;
use crate::ai::{Favour, Goal, Surroundings};
use crate::catalog::{GovtId, ShipId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::govt::{Governments, XENOPHOBIC};
use crate::rulebook::Rulebook;
use crate::traffic::npc::{Npc, NpcId};

pub use crate::ai::Help;
pub use deal::{Conversation, Haggle, Mood, Settled};
pub use like::{Attitude, attitude, likes_player};
pub use nova::{BegForMercy, Greetings, JoinFleet, Release, RequestAssistance};
pub use reply::{MissionHail, Reply};

/// `Flags`: its ships answer no hail.
pub const MUTE: u16 = 0x0400;
/// `Flags`: its warships and interceptors take bribes.
pub const BRIBABLE_WARSHIPS: u16 = 0x0200;
/// `Flags`: its warships and interceptors plunder.
pub const PLUNDERS: u16 = 0x1000;
/// `Flags`: its traders take bribes.
pub const BRIBABLE_TRADERS: u16 = 0x2000;
/// `Flags`: its ships ask a larger price.
pub const GREEDY: u16 = 0x8000;
/// `Flags2`: its ships say nothing when hailed.
pub const UNTALKATIVE: u16 = 0x0001;
/// `Flags2`: its ships send no distress calls and answer no greetings
/// (the Bible; see [`RuleKey::QuietHails`](crate::RuleKey::QuietHails)).
pub const QUIET: u16 = 0x0008;
/// `Flags2`: "Roadside Assistance": its ships help for free.
pub const FREE_HELP: u16 = 0x0010;

/// What a ship's government and type make of hails (see the module
/// docs).
// Each is its own flag of the government's, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dispositions {
    /// It answers no hail.
    pub mute: bool,
    /// It asks a larger price.
    pub greedy: bool,
    /// It takes a bribe.
    pub bribable: bool,
    /// It says nothing.
    pub untalkative: bool,
    /// Its government is quiet.
    pub quiet: bool,
    /// It helps for free: its government's `Flags2` 0x0010, or a rank's
    /// [`HELPS_FREE`](crate::ai::HELPS_FREE).
    pub free_help: bool,
    /// It fights a threat to the player for free: a rank's
    /// [`HELPS_IN_BATTLE`](crate::ai::HELPS_IN_BATTLE).
    pub battle_help: bool,
    /// It is a warship or interceptor that plunders.
    pub plunderer: bool,
    /// Its government is xenophobic.
    pub xenophobe: bool,
}

impl Dispositions {
    /// What `npc`, of a ship type whose inherent government is
    /// `inherent_govt`, makes of hails, by `govts`, the pilot's ranks
    /// granting it `favour` (see the module docs).
    #[must_use]
    pub fn of(
        npc: &Npc,
        inherent_govt: Option<GovtId>,
        govts: &Governments,
        favour: Favour,
    ) -> Self {
        let govt = npc.govt;
        let fights = !npc.ai_type.trades();
        let bribable = match govt {
            None => true,
            Some(_) if fights => govts.flag(govt, BRIBABLE_WARSHIPS),
            Some(_) => govts.flag(govt, BRIBABLE_TRADERS),
        };
        Self {
            mute: govts.flag(govt, MUTE) || govts.flag(inherent_govt, MUTE),
            greedy: govts.flag(govt, GREEDY),
            bribable,
            untalkative: govts.flag2(govt, UNTALKATIVE),
            quiet: govts.flag2(govt, QUIET) || govts.flag2(inherent_govt, QUIET),
            free_help: govts.flag2(govt, FREE_HELP) || favour.free_help,
            battle_help: favour.battle_help,
            plunderer: fights && govts.flag(govt, PLUNDERS),
            xenophobe: govts.flag(govt, XENOPHOBIC),
        }
    }
}

/// The conversation as an option sees it: everything Nova's options
/// decide by, read-only.
// Each flag is its own fact about the conversation, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug)]
pub struct Hail<'a> {
    /// The ship hailed: its AI type, government, `InfoTypes`, goal and
    /// condition among the rest.
    pub npc: &'a Npc,
    /// How it feels about the player.
    pub attitude: Attitude,
    /// The conversation's variant (see [`deal`]).
    pub variant: u8,
    /// The `InfoTypes` bit Greetings' advice follows, if any.
    pub advice: Option<u16>,
    /// The ship's mood.
    pub mood: f32,
    /// What its government and type make of hails.
    pub dispositions: Dispositions,
    /// Whether it is busy: fighting, assisting, keeping formation, holding
    /// ([`Goal::Hold`], goal 10 to `_AIIsShipBusy` @0x82fac-0x82fe4),
    /// waiting ([`Goal::Wait`], goal 6), or under attack.
    pub busy: bool,
    /// Whether it is already assisting the player: refuelling or repairing
    /// it ([`Goal::Assist`]) with the player as its target, and not
    /// disabled (`_IsRefuellingPlayer` @0x820fd, `_IsRepairingPlayer`
    /// @0x8213a). An assister a hit turned on an NPC is busy, not this.
    pub assisting_player: bool,
    /// Whether any NPC threatens the player, or a ship the player leads
    /// ([`Npc::threatens_player_among`]): `_IsPlayerThreatened` @0x8245c,
    /// read by Request Assistance (`_DoCommDialog` @0x96310).
    pub player_threatened: bool,
    /// Whether an NPC other than the ship hailed threatens the player as
    /// `_ExtendedIsThreatToPlayer` asks it ([`Npc::threatens_player_extended`],
    /// as the player sees it) and is of a government
    /// [`Governments::npc_enemies`] finds an enemy of the ship's
    /// (`_IsPlayerThreatenedByMyEnemy` @0x82483-0x824f5,
    /// [`player_threatened_by_enemy_of`], see [`Hail::new`]): what rank
    /// battle help reads (@0x96328).
    pub threatened_by_enemy: bool,
    /// Every NPC in the system, the ship hailed among them: what a helper
    /// picks its target from (see [`battle`]).
    pub npcs: &'a [Npc],
    /// The last ships of the berths vacated with their records intact
    /// ([`Traffic::vacated`](crate::traffic::Traffic::vacated)), which
    /// only the pick of rank battle help and of a paid helper reads
    /// ([`Hail::vacated_threats`]). The session sets it; none otherwise.
    pub vacated: &'a [Npc],
    /// The help the player needs, if any.
    pub need: Option<Help>,
    /// The comm quote (an entry in `STR#` 7100) Greetings says in place
    /// of a friendly ship's advice: a person's, under the engine's
    /// reading of [`RuleKey::CommQuote`](crate::RuleKey::CommQuote); none
    /// otherwise.
    pub quote: Option<u16>,
    /// Whether the ship hailed is a person who may join the player's
    /// fleet: its record allows it
    /// (`nova_sim::person::offers_to_join`), its
    /// `LinkMission` is available from a ship, and the fleet has room. The
    /// session sets it; none otherwise.
    pub joins: bool,
    /// The mission of the ship hailed, when it is a mission ship: its
    /// slot's `ShipGoal` and its mark's `ShipBehav`. The session sets it;
    /// none otherwise.
    pub mission: Option<MissionHail>,
    /// The governments, which a helper's pick of an enemy threat reads
    /// under [`RuleKey::BattleHelpTarget`](crate::RuleKey::BattleHelpTarget)'s
    /// other reading (see [`battle`]).
    pub govts: &'a Governments,
}

impl Hail<'_> {
    /// Whether the ship hailed is the player's escort.
    #[must_use]
    pub fn escort(&self) -> bool {
        self.npc.escort.is_some()
    }

    /// Whether the ship hailed is of the original's AI type above 4, which
    /// Request Assistance answers as an escort: "just an escort" when busy
    /// (`_DoCommDialog` @0x962ee-0x96304), free help at any threat
    /// (@0x964f8-0x9650f), and no refuel or repair (@0x96564-0x9656c).
    /// The writers of +0x88 above 4 are a carried fighter's launch (5,
    /// `_LaunchFighter` @0x3d771) and an escort's (6): a fleet's escorts
    /// (`_HyperSpawnFleet` @0x421dc, `_SetupShipsInSystem` @0x430b6,
    /// `_AICheckParentExists` @0x89491), a pirate's taken ship
    /// (`_AIPlunderShipContents` @0x8495c), a mission escort (`_AIDispatch`
    /// @0x8fc19) and the player's own (`_SpawnEscort` @0x3fed1, captures).
    /// So: a ship that [escorts the player](Npc::escorts_player), escorts
    /// a fleet lead ([`Npc::leader`]), or was launched from a carrier
    /// ([`Npc::carrier`]); not one the player has paid off, which the
    /// bribe made a wimpy trader (@0x961f8). AI types above 4 from data
    /// are folded into the wimpy trader at the spawn
    /// ([`AiType::from_raw`](crate::traffic::npc::AiType::from_raw)).
    #[must_use]
    pub fn escort_ai(&self) -> bool {
        let npc = self.npc;
        !npc.spared && (npc.escorts_player() || npc.leader.is_some() || npc.carrier.is_some())
    }

    /// Whether the ship hailed is a fighter out of a bay (see
    /// [`bay`](crate::bay)).
    #[must_use]
    pub fn carried(&self) -> bool {
        self.npc.carrier.is_some()
    }
}

impl<'a> Hail<'a> {
    /// Every NPC but the ship hailed that threatens the player
    /// ([`Npc::threatens_player`]), in the system's order: the ships
    /// `_AIFightEnemyThreatToPlayer` (@0x82c9f-0x82d11) picks among, by
    /// its inline test (+0x70 == 0, AI state 3 or 4, @0x82cda-0x82cef), so
    /// not a ship attacking one the player leads. Rank battle help and a
    /// paid helper pick from these ([`battle`]), and, as the routine has
    /// no active test (+0xb8), from the [`Hail::vacated_threats`] too.
    pub fn threats_to_player(&self) -> impl Iterator<Item = &'a Npc> + use<'a> {
        let me = self.npc.id;
        self.npcs
            .iter()
            .filter(move |other| other.id != me && other.threatens_player())
    }

    /// The [`Hail::vacated`] ships that pass the same inline test
    /// ([`Npc::threatens_player`]: +0x70 == 0, AI state 3 or 4, intact
    /// standing for `_IsDisabled` and `_IsDying` on the record as it was
    /// left), in the order recorded: the inactive slots
    /// `_AIFightEnemyThreatToPlayer` still counts (@0x82c9f-0x82d99; see
    /// [`traffic::vacated`](crate::traffic::vacated)). A berth that the
    /// ship hailed or any NPC in the system holds is not one: a ship
    /// taking a berth drops its record anyway.
    pub fn vacated_threats(&self) -> impl Iterator<Item = &'a Npc> + use<'a> {
        let npcs = self.npcs;
        let mine = self.npc.berth;
        self.vacated.iter().filter(move |gone| {
            gone.berth != mine
                && gone.threatens_player()
                && npcs.iter().all(|npc| npc.berth != gone.berth)
        })
    }
}

impl<'a> Hail<'a> {
    /// The hail of `npc` among `around`, of `dispositions`, in
    /// conversation `talk`, the player needing `need`, of `mission` when
    /// it is a mission ship ([`Hail::mission`]). It is busy while
    /// it attacks, snipes at, flees from, plunders or assists a ship, keeps
    /// formation (goals 10 and 12), stops and waits (goal 6, which
    /// `_AIIsShipBusy` @0x82fac-0x82fe4 finds busy as it finds every goal
    /// but 0, 1, 2, 7 and 0x14), waits or drifts as a pirate (goals 0x16
    /// and 0xe), or an
    /// NPC attacks or snipes at it
    /// (`_AIIsShipBusy` @0x82fac, `_IsShipThreatened` @0x82068); the
    /// player is threatened while any NPC threatens it.
    ///
    /// It is threatened by the ship's enemy
    /// (`_IsPlayerThreatenedByMyEnemy` @0x82483-0x824f5) when another NPC
    /// threatens it whose government [`Governments::npc_enemies`] finds
    /// an enemy of the ship's, as
    /// [`player_threatened_by_enemy_of`] asks it: the original loops over
    /// slots 1-63, an active one (@0x824a0), not the ship (@0x824ac), that
    /// `_ExtendedIsThreatToPlayer` holds for (@0x824b8,
    /// [`Npc::threatens_player_extended`], read whole with the player's
    /// sight) and `_IsShipMyEnemy` (@0x8196c) finds the ship's enemy
    /// (@0x824cf). Two branches are left out: the extended test's stellar
    /// defender (+0x8c, @0x81bd3), as the engine has no defence ships
    /// (rdm:task/planet-domination), and `_IsShipMyEnemy`'s test that the
    /// other is not the shareware nag ship (`+0xc8d0 != 0x3ff` @0x819a4,
    /// `_SpawnNagShip` @0x61b5), as the engine has none. Nothing plays a
    /// sound.
    #[must_use]
    pub fn new(
        npc: &'a Npc,
        around: &Surroundings<'a>,
        dispositions: Dispositions,
        talk: &Conversation,
        need: Option<Help>,
        mission: Option<MissionHail>,
    ) -> Self {
        let me = ShipRef::Npc(npc.id);
        // `_IsRefuellingPlayer` @0x820fd and `_IsRepairingPlayer`
        // @0x8213a: active, not `_IsDisabled`, target 0, goal 9 or 0xf. A
        // hailed ship is active.
        let assisting_player = matches!(npc.goal, Goal::Assist(_))
            && npc.target == Some(ShipRef::Player)
            && !npc.condition.disabled();
        let attacked = around
            .npcs
            .iter()
            .any(|other| other.id != npc.id && other.goal.attacking() == Some(me));
        Self {
            npc,
            attitude: attitude(npc, around, mission),
            variant: talk.variant,
            advice: talk.advice,
            mood: talk.mood,
            dispositions,
            busy: npc.goal.fights()
                || matches!(npc.goal, Goal::Assist(_))
                || matches!(
                    npc.goal,
                    Goal::Formation { .. }
                        | Goal::Hold
                        | Goal::Wait
                        | Goal::CountWait
                        | Goal::Drift
                )
                || attacked,
            assisting_player,
            player_threatened: around
                .npcs
                .iter()
                .any(|other| other.threatens_player_among(around.npcs)),
            threatened_by_enemy: player_threatened_by_enemy_of(npc, around),
            npcs: around.npcs,
            vacated: &[],
            need,
            quote: None,
            joins: false,
            mission,
            govts: around.govts,
        }
    }
}

/// What a ship does at an option's press, or once the player pays or
/// declines its price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deed {
    /// It spares the player: it leaves, never to fight again.
    Spare,
    /// It attacks the player.
    Attack,
    /// It flies over to help the player.
    Help(Help),
    /// The player's escort leaves the fleet, and the system, once the
    /// channel closes.
    Release,
    /// The person hailed joins the player's fleet, as itself, at once.
    Join,
    /// It fights a threat to the player, as [`Defence`] says.
    Defend(Defence),
}

/// How a ship sent to fight a threat to the player does it (see
/// [`battle`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Defence {
    /// The NPC it attacks, its target set and its AI state attack
    /// (+0x70, +0xc8c8 = 4); none when the pick found no threat, which
    /// clears its target (+0x70 = -1) and changes nothing else
    /// (`_AIFightEnemyThreatToPlayer` @0x82daa, `_AIFightThreatToPlayer`
    /// @0x8195e).
    pub threat: Option<NpcId>,
    /// Whether it then calls for reinforcements, forced past their
    /// strength test (`_AICallForReinforcements(ship, 1)` @0x9636d):
    /// rank battle help alone does, whatever the pick found.
    pub reinforce: bool,
    /// Whether its travel stellar is dropped (+0x6c = -1, `Npc::travel_set`
    /// false) as it takes a threat: `_AIFightThreatToPlayer` does
    /// (@0x81949), `_AIFightEnemyThreatToPlayer` doesn't.
    pub drop_travel: bool,
}

/// A price a ship asks, and what it says and does as the haggling ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ask {
    /// Its reply and deed when the player pays.
    pub paid: (Reply, Option<Deed>),
    /// Its reply and deed when the player declines.
    pub declined: (Reply, Option<Deed>),
    /// Its reply when the player's cash is short.
    pub short: Reply,
}

/// How a ship answers an option's press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    /// What it says just before `reply` in the same press, drawn and then
    /// replaced, as a free helper's price text is (`_DoCommDialog`
    /// @0x965aa-0x965af, then @0x965e3).
    pub first: Option<Reply>,
    /// What it says; none leaves the reply as it was.
    pub reply: Option<Reply>,
    /// What it does at once, if anything.
    pub deed: Option<Deed>,
    /// The price it asks, if any.
    pub ask: Option<Ask>,
}

impl Answer {
    /// It says `reply`, and does nothing.
    #[must_use]
    pub fn say(reply: Reply) -> Self {
        Self {
            reply: Some(reply),
            ..Self::default()
        }
    }
}

/// Why a hail went unanswered (`_HandlePlayerCommunication` @0x61f48).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HailRefusal {
    /// There is nothing to hail: no ship target and no navigation
    /// stellar, or Option held and no navigation stellar (@0x6206c), or
    /// the player is landed, breaking up, or braking for or making a jump
    /// (@0x61f87). Nothing is said.
    NoTarget,
    /// "No response." (`STR#` 2002 #53): the target is disabled, even
    /// while still jumping in (@0x62610-0x62644), or mute (see
    /// [`Dispositions`]), or an escape pod, the player's escort or not
    /// ([`ESCAPE_POD_SHIP`](crate::combat::hull::ESCAPE_POD_SHIP),
    /// @0x62145); or the navigation stellar hailed, with no ship target
    /// or Option held, does not answer
    /// ([`landing::answers_hail`](crate::landing::answers_hail)) or is
    /// not shown in the system (@0x6261c).
    NoResponse,
    /// "Unable to send hail - target ship is entering hyperspace."
    /// (`STR#` 2002 #54): the target is still jumping in and not disabled
    /// (@0x6264e-0x62676), mute or not.
    InHyperspace,
}

/// One of the comm dialog's buttons, an option that applies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HailButton {
    /// Its label.
    pub label: String,
    /// Its hotkey, if it has one.
    pub key: Option<char>,
}

/// The comm dialog's contents: the ship hailed, its reply, and the
/// options that apply.
#[derive(Clone, Debug, PartialEq)]
pub struct HailView {
    /// The NPC hailed.
    pub npc: NpcId,
    /// Its ship type.
    pub ship: ShipId,
    /// What it said just before `reply` in the last press, if anything: a
    /// free helper's price text. The original draws it and replaces it in
    /// the same click (`_DoCommDialog` @0x965aa-0x965af, then @0x965e3),
    /// so a panel may show `reply` alone.
    pub first: Option<String>,
    /// What it last said.
    pub reply: String,
    /// Its ship type's `CommName`.
    pub comm_name: String,
    /// Its government's `CommName`; none for an independent.
    pub govt_name: Option<String>,
    /// Whether it is hostile to the player.
    pub hostile: bool,
    /// The options that apply, in order: the buttons above Close Channel.
    pub options: Vec<HailButton>,
    /// The price it asks while the player haggles, if any.
    pub asking: Option<i64>,
    /// Whether the haggle dialog says "Pay me" (a ship), rather than "Pay
    /// us" (a planet, which is not hailed here).
    pub pay_me: bool,
    /// What it is as a ship the player leads (its escorts, carried
    /// fighters and mission escorts); none for a ship it does not lead,
    /// and none once Release is pressed in this hail.
    pub escort: Option<EscortStatus>,
    /// The `PICT` shown in place of its ship type's picture: a person's
    /// `HailPict` (`_DoCommDialog` @0x95a77-0x95aa5); none for any other
    /// ship.
    pub portrait: Option<i16>,
}

/// What the comm dialog shows of a ship the player leads hailed
/// (`_CommDialogUpdate` @0x90688-0x90704): "Hired Escort" for a hired
/// escort, or "Escort" for any other (a captured escort, a launched
/// fighter, a mission escort). A fleet escort hailed opens the escort
/// dialog instead (`nova_sim::escort_dialog`), which shows a hired
/// escort's pay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortStatus {
    /// Whether it is a hired fleet escort (+0xbb).
    pub hired: bool,
}

/// The word after a pay amount: `_creditString` ("credit") or
/// `_creditsString` ("credits").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditUnit {
    /// "credit", the singular.
    Credit,
    /// "credits", every other amount.
    Credits,
}

/// A hired escort's daily pay as its pay line shows it: the whole
/// credits drawn and the unit after them, which the core decides
/// (`_EscortDialogUpdate` @0x94f9c-0x9500d).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownWage {
    /// The amount drawn, which may be negative.
    pub credits: i64,
    /// "credit" or "credits".
    pub unit: CreditUnit,
}

impl ShownWage {
    /// A kept whole-credit `wage` shown as itself: "credit" at exactly 1.
    #[must_use]
    pub fn kept(wage: i64) -> Self {
        Self {
            credits: wage,
            unit: if wage == 1 {
                CreditUnit::Credit
            } else {
                CreditUnit::Credits
            },
        }
    }
}

/// How long an assisting ship's comm note shows: 240 (`_HighLevelAIHandler`
/// @0x8f6a1, `_LowLevelAIHandler` @0x86ef4), the one count the session
/// sets the message line to and the flight shows it for.
pub const NOTE_TICKS: u16 = 240;

/// What an NPC assisting the player has done, for the flight's message
/// line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommNote {
    /// The NPC that helped.
    pub from: NpcId,
    /// Its ship type's `CommName`.
    pub comm_name: String,
    /// The player's ship's name, which the line names after the help
    /// (`*0x3b5210`, `_HighLevelAIHandler` @0x8f62a, `_LowLevelAIHandler`
    /// @0x86e7d).
    pub ship_name: String,
    /// The help it gave.
    pub done: Help,
}

impl CommNote {
    /// How long the note shows: [`NOTE_TICKS`], whatever the help.
    #[must_use]
    pub fn ticks(&self) -> u16 {
        NOTE_TICKS
    }
}

/// One of the comm dialog's options, a button with its hotkey: an open
/// set, registered at the edge in [`HailOptions`].
pub trait HailOption: Debug {
    /// Its button's label.
    fn label(&self) -> String;

    /// Its hotkey, if it has one.
    fn key(&self) -> Option<char> {
        None
    }

    /// Whether the dialog lists it for `hail`.
    fn applies(&self, hail: &Hail) -> bool;

    /// How the ship answers its press, rolling any choice on `chance`.
    fn press(&self, hail: &Hail, chance: &mut dyn Chance) -> Answer;

    /// Whether its press makes the person hailed join the fleet
    /// ([`Deed::Join`]): a hail that lists one opens the comm dialog in
    /// place of the person's mission offer (see the session's
    /// `person_offers`). None does by default.
    fn joins(&self) -> bool {
        false
    }
}

/// The comm dialog's options, in the order it lists them.
#[derive(Clone, Debug)]
pub struct HailOptions {
    options: Vec<Rc<dyn HailOption>>,
}

impl Default for HailOptions {
    /// Nova's four, by the engine.
    fn default() -> Self {
        Self::nova(&Rulebook::default())
    }
}

impl HailOptions {
    /// No options at all.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            options: Vec::new(),
        }
    }

    /// Nova's five, Greetings, Request Assistance, Beg For Mercy, Release
    /// and Use As Escort, as `rulebook` chooses: the first three's
    /// [`RuleKey::QuietHails`](crate::RuleKey::QuietHails), Greetings'
    /// [`RuleKey::LongAdvice`](crate::RuleKey::LongAdvice), Request
    /// Assistance's [`RuleKey::BattleHelpTarget`](crate::RuleKey::BattleHelpTarget),
    /// [`RuleKey::BraveTraderBattleHelp`](crate::RuleKey::BraveTraderBattleHelp)
    /// and [`RuleKey::BattleHelpGoneSlots`](crate::RuleKey::BattleHelpGoneSlots),
    /// and Use As Escort's [`RuleKey::PersonJoin`](crate::RuleKey::PersonJoin)
    /// entries. Release is for the player's escorts and Use As Escort for
    /// the rest, so the two are never listed together.
    #[must_use]
    pub fn nova(rulebook: &Rulebook) -> Self {
        Self::empty()
            .with(Rc::new(Greetings::from_rulebook(rulebook)))
            .with(Rc::new(RequestAssistance::from_rulebook(rulebook)))
            .with(Rc::new(BegForMercy::from_rulebook(rulebook)))
            .with(Rc::new(Release))
            .with(Rc::new(JoinFleet::from_rulebook(rulebook)))
    }

    /// These options with `option` after them.
    #[must_use]
    pub fn with(mut self, option: Rc<dyn HailOption>) -> Self {
        self.options.push(option);
        self
    }

    /// Whether an option that applies to `hail` makes the person join
    /// the fleet ([`HailOption::joins`]).
    #[must_use]
    pub fn lists_join(&self, hail: &Hail) -> bool {
        self.listed(hail).iter().any(|option| option.joins())
    }

    /// The options that apply to `hail`, in order.
    #[must_use]
    pub fn listed(&self, hail: &Hail) -> Vec<&dyn HailOption> {
        self.options
            .iter()
            .map(|option| &**option)
            .filter(|option| option.applies(hail))
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod fixture {
    use super::*;
    use crate::stats::ShipStats;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::FAST;

    /// NPC 1, a warship of no government, idle.
    pub(crate) fn ship() -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.ai_type = AiType::Warship;
        npc
    }

    /// A hail of `npc`, friendly, talkative and in variant 0, with an
    /// even mood, no advice, nothing busy and nothing needed.
    pub(crate) fn hail(npc: &Npc) -> Hail<'_> {
        Hail {
            npc,
            attitude: Attitude::Friendly,
            variant: 0,
            advice: None,
            mood: 1.0,
            dispositions: Dispositions::default(),
            busy: false,
            assisting_player: false,
            player_threatened: false,
            threatened_by_enemy: false,
            npcs: &[],
            vacated: &[],
            need: None,
            quote: None,
            joins: false,
            mission: None,
            govts: &NO_GOVERNMENTS,
        }
    }

    /// No governments at all.
    static NO_GOVERNMENTS: Governments = Governments::empty();
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::fixture::{hail, ship};
    use super::*;
    use crate::ai::{self, Favour, HELPS_FREE, HELPS_IN_BATTLE};
    use crate::catalog::{GovtRecord, RankId, RankRecord};
    use crate::rulebook::{RuleKey, RuleSource};
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_catalog::fixture::govt;
    use std::collections::BTreeSet;

    #[test]
    fn a_kept_wage_shows_as_itself_singular_only_at_one() {
        let shown = |credits, unit| ShownWage { credits, unit };
        assert_eq!(ShownWage::kept(1), shown(1, CreditUnit::Credit));
        assert_eq!(ShownWage::kept(0), shown(0, CreditUnit::Credits));
        assert_eq!(ShownWage::kept(2), shown(2, CreditUnit::Credits));
        assert_eq!(ShownWage::kept(175), shown(175, CreditUnit::Credits));
    }

    #[test]
    fn a_comm_note_shows_for_240_ticks_whatever_the_help() {
        for done in [Help::Refuel, Help::Repair] {
            let note = CommNote {
                from: NpcId(0),
                comm_name: String::new(),
                ship_name: String::new(),
                done,
            };
            assert_eq!(note.ticks(), 240, "{done:?}");
        }
    }

    const G: GovtId = GovtId(140);
    const INHERENT: GovtId = GovtId(141);

    /// Government 140 of `flags` and `flags2`, and 141 of the `inherent`
    /// flags and flags2.
    fn govts(flags: u16, flags2: u16, inherent: (u16, u16)) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                flags2,
                ..govt(140)
            },
            GovtRecord {
                flags: inherent.0,
                flags2: inherent.1,
                ..govt(141)
            },
        ])
    }

    /// What a ship of `ai_type` and `govt`, its type inheriting 141,
    /// makes of hails among `govts`.
    fn of(ai_type: AiType, govt: Option<GovtId>, govts: &Governments) -> Dispositions {
        let mut npc = ship();
        npc.ai_type = ai_type;
        npc.govt = govt;
        Dispositions::of(&npc, Some(INHERENT), govts, Favour::default())
    }

    /// Every flag on the inherent government, none on its own.
    fn inherited() -> Governments {
        govts(
            0,
            0,
            (
                GREEDY | XENOPHOBIC | BRIBABLE_TRADERS | BRIBABLE_WARSHIPS | PLUNDERS,
                UNTALKATIVE | FREE_HELP,
            ),
        )
    }

    #[test]
    fn a_plain_government_is_talkative_and_takes_no_bribes() {
        let govts = govts(0, 0, (0, 0));
        assert_eq!(
            of(AiType::Warship, Some(G), &govts),
            Dispositions::default()
        );
    }

    #[test]
    fn its_government_or_its_inherent_one_makes_it_mute_or_quiet() {
        for (flags, flags2, inherent) in [(MUTE, 0, (0, QUIET)), (0, QUIET, (MUTE, 0))] {
            let govts = govts(flags, flags2, inherent);
            let made = of(AiType::Warship, Some(G), &govts);
            assert!(made.mute && made.quiet, "{made:?}");
            assert!(!made.untalkative && !made.free_help, "{made:?}");
        }
        let govts = govts(0, 0, (MUTE, QUIET));
        let mut npc = ship();
        npc.govt = Some(G);
        assert_eq!(
            Dispositions::of(&npc, None, &govts, Favour::default()),
            Dispositions::default(),
            "no inherent government"
        );
    }

    #[test]
    fn only_its_own_government_makes_it_greedy_untalkative_helpful_or_xenophobic() {
        let govts = govts(GREEDY | XENOPHOBIC, UNTALKATIVE | FREE_HELP, (0, 0));
        let made = of(AiType::WimpyTrader, Some(G), &govts);
        assert!(made.greedy && made.xenophobe, "{made:?}");
        assert!(made.untalkative && made.free_help, "{made:?}");
        assert!(!made.mute && !made.quiet, "{made:?}");
        assert_eq!(
            of(AiType::WimpyTrader, Some(G), &inherited()),
            Dispositions::default()
        );
    }

    #[test]
    fn an_independent_ship_always_takes_a_bribe() {
        let govts = govts(0, 0, (0, 0));
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            assert!(of(ai_type, None, &govts).bribable, "{ai_type:?}");
        }
    }

    #[test]
    fn traders_and_warships_take_bribes_by_their_own_flags() {
        let traders = govts(BRIBABLE_TRADERS, 0, (0, 0));
        let warships = govts(BRIBABLE_WARSHIPS, 0, (0, 0));
        for trader in [AiType::WimpyTrader, AiType::BraveTrader] {
            assert!(of(trader, Some(G), &traders).bribable, "{trader:?}");
            assert!(!of(trader, Some(G), &warships).bribable, "{trader:?}");
        }
        for fighter in [AiType::Warship, AiType::Interceptor] {
            assert!(of(fighter, Some(G), &warships).bribable, "{fighter:?}");
            assert!(!of(fighter, Some(G), &traders).bribable, "{fighter:?}");
        }
    }

    #[test]
    fn only_a_warship_or_interceptor_plunders() {
        let govts = govts(PLUNDERS, 0, (0, 0));
        assert!(of(AiType::Warship, Some(G), &govts).plunderer);
        assert!(of(AiType::Interceptor, Some(G), &govts).plunderer);
        assert!(!of(AiType::BraveTrader, Some(G), &govts).plunderer);
        assert!(!of(AiType::Warship, Some(G), &inherited()).plunderer);
    }

    #[test]
    fn rank_favour_gives_battle_help_and_ors_free_help_beside_the_government_flag() {
        let mut npc = ship();
        npc.govt = Some(G);
        let plain = govts(0, 0, (0, 0));
        let battle = Dispositions::of(
            &npc,
            None,
            &plain,
            Favour {
                battle_help: true,
                free_help: false,
            },
        );
        assert!(battle.battle_help && !battle.free_help, "{battle:?}");
        let free = Dispositions::of(
            &npc,
            None,
            &plain,
            Favour {
                battle_help: false,
                free_help: true,
            },
        );
        assert!(free.free_help && !free.battle_help, "{free:?}");
        let flagged = Dispositions::of(&npc, None, &govts(0, FREE_HELP, (0, 0)), Favour::default());
        assert!(flagged.free_help && !flagged.battle_help, "{flagged:?}");
    }

    #[test]
    fn the_rank_dispositions_reach_an_allied_and_not_an_unallied_ship_under_both_readings() {
        // G (140, class 1, allied with class 2), A (141, class 2) and U
        // (142, class 3).
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
        ]);
        let rank_of = |id, govt| RankRecord {
            affil_govt: Some(GovtId(govt)),
            flags: HELPS_IN_BATTLE | HELPS_FREE,
            ..nova_sim_catalog::fixture::rank(id)
        };
        let records = [rank_of(130, 141), rank_of(131, 140)];
        let made = |active: &[i16], ship_govt: i16, reach| {
            let mut npc = ship();
            npc.govt = Some(GovtId(ship_govt));
            let active: BTreeSet<RankId> = active.iter().copied().map(RankId).collect();
            let favour = ai::favour(&records, &active, npc.govt, &govts, reach);
            let made = Dispositions::of(&npc, None, &govts, favour);
            (made.battle_help, made.free_help)
        };
        let (engine, bible) = (RuleSource::Engine, RuleSource::Bible);
        assert_eq!(made(&[130], 140, engine), (true, true), "allied");
        assert_eq!(made(&[130], 142, engine), (false, false), "unallied");
        assert_eq!(made(&[130], 140, bible), (false, false), "allied");
        assert_eq!(made(&[130], 142, bible), (false, false), "unallied");
        assert_eq!(made(&[131], 140, bible), (true, true), "its own");
    }

    #[test]
    fn the_government_flag_gives_free_help_through_the_rank_aware_dispositions_with_the_price_text()
    {
        let govts = govts(0, FREE_HELP, (0, 0));
        let mut npc = ship();
        npc.govt = Some(G);
        let favour = ai::favour(&[], &BTreeSet::new(), npc.govt, &govts, RuleSource::Engine);
        let dispositions = Dispositions::of(&npc, None, &govts, favour);
        let needy = Hail {
            dispositions,
            need: Some(Help::Repair),
            ..hail(&npc)
        };
        let answer = RequestAssistance::default().press(&needy, &mut crate::NeverFires);
        assert_eq!(answer.first, Some(Reply::Comm(reply::HELP_FOR_PAY)));
        assert_eq!(answer.reply, Some(Reply::Comm(reply::ON_MY_WAY)));
        assert_eq!(answer.ask, None);
        assert_eq!(answer.deed, Some(Deed::Help(Help::Repair)));
    }

    /// NPC `id` of government 140, flying `goal`.
    fn flying(id: u32, goal: Goal) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, crate::stats::ShipStats::default());
        npc.govt = Some(G);
        npc.goal = goal;
        npc
    }

    /// No flags on 140 or 141.
    fn plain() -> Governments {
        govts(0, 0, (0, 0))
    }

    /// The hail of `npcs[0]` among `npcs` and `govts`, in a system of 140
    /// with the player's record `record`, its conversation `talk`.
    fn hailing<'a>(
        npcs: &'a [Npc],
        govts: &'a Governments,
        record: i16,
        talk: &Conversation,
    ) -> Hail<'a> {
        let around = Surroundings {
            govts,
            system_govt: Some(G),
            record,
            ..Surroundings::new(&[], npcs)
        };
        Hail::new(
            &npcs[0],
            &around,
            Dispositions::default(),
            talk,
            Some(Help::Refuel),
            None,
        )
    }

    fn talk() -> Conversation {
        let mut chance = nova_sim_base::fixture::Draws::of(&[3, 40, 1, 1, 0, 1]);
        Conversation::open(100_000, false, 0xC000, &mut chance)
    }

    #[test]
    fn a_hail_carries_the_conversations_variant_mood_and_advice() {
        let plain = plain();
        let npcs = [flying(1, Goal::Idle)];
        let talk = talk();
        let hail = hailing(&npcs, &plain, 0, &talk);
        assert_eq!(hail.npc.id, NpcId(1));
        assert_eq!(hail.variant, 3);
        assert_eq!(hail.mood, talk.mood);
        assert_eq!(hail.advice, Some(0x8000));
        assert_eq!(hail.need, Some(Help::Refuel));
        assert_eq!(hail.dispositions, Dispositions::default());
    }

    #[test]
    fn a_hail_reads_the_ships_attitude() {
        let plain = plain();
        let npcs = [flying(1, Goal::Idle)];
        assert_eq!(
            hailing(&npcs, &plain, 0, &talk()).attitude,
            Attitude::Friendly
        );
        assert_eq!(
            hailing(&npcs, &plain, -100, &talk()).attitude,
            Attitude::Unfriendly
        );
        let hunting = [flying(1, Goal::Attack(ShipRef::Player))];
        assert_eq!(
            hailing(&hunting, &plain, 0, &talk()).attitude,
            Attitude::Hostile
        );
    }

    #[test]
    fn a_waiting_ship_is_busy() {
        // Goal 6 is none of `_AIIsShipBusy`'s 0, 1, 2, 7 and 0x14
        // (@0x82fac-0x82fe4).
        let npcs = [flying(1, Goal::Wait)];
        assert!(hailing(&npcs, &plain(), 0, &talk()).busy);
    }

    #[test]
    fn a_waiting_miner_is_busy_when_hailed() {
        // A miner of `Flags3` 0x0001 with no asteroid in slot 0 stops and
        // waits (state 6, `_MinerAI` @0x8b2d1).
        let mut zephyr = flying(1, Goal::Idle);
        zephyr.miner = crate::ai::miner::MINES;
        let alone = [zephyr.clone()];
        zephyr.goal = crate::ai::miner::decide(&zephyr, &Surroundings::new(&[], &alone));
        let npcs = [zephyr];
        assert!(hailing(&npcs, &plain(), 0, &talk()).busy);
    }

    #[test]
    fn a_held_ship_is_busy() {
        let npcs = [flying(1, Goal::Hold)];
        assert!(hailing(&npcs, &plain(), 0, &talk()).busy);
    }

    #[test]
    fn a_ship_fighting_assisting_or_attacked_is_busy() {
        let plain = plain();
        for goal in [
            Goal::Attack(ShipRef::Npc(NpcId(5))),
            Goal::Snipe(ShipRef::Npc(NpcId(5))),
            Goal::Flee(ShipRef::Npc(NpcId(5))),
            Goal::Assist(Help::Repair),
            Goal::Formation { guard: None },
            Goal::Formation {
                guard: Some(ShipRef::Npc(NpcId(5))),
            },
            Goal::Plunder {
                ship: ShipRef::Npc(NpcId(5)),
                hopeless: false,
            },
            Goal::CountWait,
            Goal::Drift,
        ] {
            let npcs = [flying(1, goal)];
            assert!(hailing(&npcs, &plain, 0, &talk()).busy, "{goal:?}");
        }
        for goal in [
            Goal::Idle,
            Goal::JumpOut,
            Goal::Inspect(ShipRef::Npc(NpcId(5))),
        ] {
            let npcs = [flying(1, goal)];
            assert!(!hailing(&npcs, &plain, 0, &talk()).busy, "{goal:?}");
        }
        for goal in [
            Goal::Attack(ShipRef::Npc(NpcId(1))),
            Goal::Snipe(ShipRef::Npc(NpcId(1))),
        ] {
            let npcs = [flying(1, Goal::Idle), flying(2, goal)];
            assert!(
                hailing(&npcs, &plain, 0, &talk()).busy,
                "attacked: {goal:?}"
            );
        }
        for goal in [
            Goal::Flee(ShipRef::Npc(NpcId(1))),
            Goal::Attack(ShipRef::Npc(NpcId(3))),
        ] {
            let npcs = [flying(1, Goal::Idle), flying(2, goal)];
            assert!(!hailing(&npcs, &plain, 0, &talk()).busy, "{goal:?}");
        }
    }

    #[test]
    fn a_refueller_retargeted_at_an_npc_is_busy_but_not_assisting_the_player() {
        let plain = plain();
        for help in [Help::Refuel, Help::Repair] {
            let mut helper = flying(1, Goal::Assist(help));
            helper.target = Some(ShipRef::Npc(NpcId(2)));
            let npcs = [helper, flying(2, Goal::Idle)];
            let hail = hailing(&npcs, &plain, 0, &talk());
            assert!(hail.busy, "{help:?}: goals 9 and 0xf, `_AIIsShipBusy`");
            assert!(!hail.assisting_player, "{help:?}: target 0, @0x8211c");
            let mut none = flying(1, Goal::Assist(help));
            none.target = None;
            let npcs = [none];
            assert!(!hailing(&npcs, &plain, 0, &talk()).assisting_player);
        }
    }

    #[test]
    fn a_busy_refueller_retargeted_at_an_npc_answers_busy_not_on_my_way() {
        use crate::hail::nova::RequestAssistance;
        use crate::hail::reply::{BUSY, ON_MY_WAY};
        let plain = plain();
        let press = |target: ShipRef| {
            let mut helper = flying(1, Goal::Assist(Help::Refuel));
            helper.target = Some(target);
            let npcs = [helper, flying(2, Goal::Idle)];
            let hail = hailing(&npcs, &plain, 0, &talk());
            RequestAssistance::default().press(&hail, &mut nova_sim_base::fixture::Draws::of(&[]))
        };
        assert_eq!(
            press(ShipRef::Npc(NpcId(2))),
            Answer::say(Reply::Comm(BUSY)),
            "0x10 for AI type 4 or below, @0x962f8"
        );
        assert_eq!(
            press(ShipRef::Player),
            Answer::say(Reply::Comm(ON_MY_WAY)),
            "0x1d, @0x962da"
        );
    }

    #[test]
    fn a_disabled_assister_is_not_assisting_the_player() {
        use crate::combat::hull::Condition;
        let plain = plain();
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 5,
                starting: false,
            },
        ] {
            let mut helper = flying(1, Goal::Assist(Help::Repair));
            helper.target = Some(ShipRef::Player);
            helper.condition = condition;
            let npcs = [helper];
            let hail = hailing(&npcs, &plain, 0, &talk());
            assert!(!hail.assisting_player, "{condition:?}: @0x82113");
            assert!(hail.busy);
        }
    }

    #[test]
    fn a_hail_knows_whether_the_ship_assists_the_player_and_whether_the_player_is_threatened() {
        let plain = plain();
        let mut helper = flying(1, Goal::Assist(Help::Refuel));
        helper.target = Some(ShipRef::Player);
        let npcs = [helper];
        let hail = hailing(&npcs, &plain, 0, &talk());
        assert!(hail.assisting_player);
        assert!(!hail.player_threatened);
        let npcs = [
            flying(1, Goal::Idle),
            flying(2, Goal::Snipe(ShipRef::Player)),
        ];
        let hail = hailing(&npcs, &plain, 0, &talk());
        assert!(!hail.assisting_player);
        assert!(hail.player_threatened);
        let hunting = [flying(1, Goal::Attack(ShipRef::Player))];
        assert!(
            hailing(&hunting, &plain, 0, &talk()).player_threatened,
            "itself"
        );
        let mut escort = flying(1, Goal::Idle);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let npcs = [escort, flying(2, Goal::Attack(ShipRef::Npc(NpcId(1))))];
        assert!(
            hailing(&npcs, &plain, 0, &talk()).player_threatened,
            "a ship attacking one the player leads"
        );
        let npcs = [
            flying(1, Goal::Idle),
            flying(2, Goal::Attack(ShipRef::Npc(NpcId(1)))),
        ];
        assert!(
            !hailing(&npcs, &plain, 0, &talk()).player_threatened,
            "a ship attacking one nobody leads"
        );
    }

    /// 140 (class 1, enemies with class 2, allied with class 3), 142
    /// (class 2), 143 (class 3) and 144 (class 4).
    fn rivals() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                allies: [3, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..govt(143)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(144)
            },
        ])
    }

    /// NPC `id` of `govt`, flying `goal`.
    fn of_govt(id: u32, govt: Option<i16>, goal: Goal) -> Npc {
        Npc {
            govt: govt.map(GovtId),
            ..flying(id, goal)
        }
    }

    #[test]
    fn a_hail_knows_whether_an_enemy_of_the_ship_threatens_the_player() {
        let rivals = rivals();
        let threatened = |other: Npc| {
            let npcs = [flying(1, Goal::Idle), other];
            let hail = hailing(&npcs, &rivals, 0, &talk());
            (hail.player_threatened, hail.threatened_by_enemy)
        };
        let sniping = Goal::Snipe(ShipRef::Player);
        assert_eq!(threatened(of_govt(2, Some(142), sniping)), (true, true));
        assert_eq!(
            threatened(of_govt(2, Some(142), Goal::Attack(ShipRef::Player))),
            (true, true)
        );
        for govt in [Some(143), Some(144), Some(140), None] {
            assert_eq!(
                threatened(of_govt(2, govt, sniping)),
                (true, false),
                "{govt:?}"
            );
        }
        for goal in [Goal::Idle, Goal::Attack(ShipRef::Npc(NpcId(1)))] {
            assert_eq!(
                threatened(of_govt(2, Some(142), goal)),
                (false, false),
                "{goal:?}"
            );
        }
        let npcs = [of_govt(1, Some(142), Goal::Attack(ShipRef::Player))];
        let hail = hailing(&npcs, &rivals, 0, &talk());
        assert!(
            hail.player_threatened && !hail.threatened_by_enemy,
            "itself"
        );
        let mut independent = flying(1, Goal::Idle);
        independent.govt = None;
        let npcs = [independent, of_govt(2, Some(142), sniping)];
        assert!(
            !hailing(&npcs, &rivals, 0, &talk()).threatened_by_enemy,
            "an independent has no enemy"
        );
    }

    /// Whether an enemy of NPC 1 (`npcs[0]`, of 140) threatens the player
    /// among `npcs`, the player scanning for cloaks when `scanner`.
    fn by_enemy(npcs: &[Npc], scanner: bool) -> bool {
        let rivals = rivals();
        let around = Surroundings {
            govts: &rivals,
            system_govt: Some(G),
            cloak_scanner: scanner,
            ..Surroundings::new(&[], npcs)
        };
        Hail::new(
            &npcs[0],
            &around,
            Dispositions::default(),
            &talk(),
            None,
            None,
        )
        .threatened_by_enemy
    }

    /// NPC `id` hired by the player.
    fn hired(id: u32) -> Npc {
        Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            ..of_govt(id, Some(143), Goal::Idle)
        }
    }

    #[test]
    fn rank_battle_help_counts_an_enemy_attacking_a_ship_the_player_leads() {
        let raider = of_govt(2, Some(142), Goal::Attack(ShipRef::Npc(NpcId(3))));
        assert!(by_enemy(&[flying(1, Goal::Idle), raider, hired(3)], false));
    }

    #[test]
    fn rank_battle_help_counts_an_enemy_whose_hit_target_is_the_player() {
        let mut hit = of_govt(2, Some(142), Goal::Idle);
        hit.target = Some(ShipRef::Player);
        assert!(by_enemy(&[flying(1, Goal::Idle), hit.clone()], false));
        hit.ai_wait = 5;
        assert!(
            !by_enemy(&[flying(1, Goal::Idle), hit], false),
            "its wait running"
        );
    }

    #[test]
    fn rank_battle_help_counts_an_enemy_lead_whose_escort_attacks_the_player() {
        // The lead needs a target of its own (+0x70 != -1, @0x81bad).
        let mut lead = of_govt(2, Some(142), Goal::Idle);
        lead.target = Some(ShipRef::Npc(NpcId(9)));
        let mut escort = of_govt(3, Some(143), Goal::Attack(ShipRef::Player));
        escort.leader = Some(NpcId(2));
        assert!(by_enemy(&[flying(1, Goal::Idle), lead, escort], false));
    }

    #[test]
    fn rank_battle_help_counts_a_cloaked_enemy_only_while_the_player_sees_it() {
        let mut cloaked = of_govt(2, Some(142), Goal::Attack(ShipRef::Player));
        cloaked.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        let npcs = [flying(1, Goal::Idle), cloaked];
        assert!(!by_enemy(&npcs, false), "unseen");
        assert!(by_enemy(&npcs, true), "scanned on screen");
    }

    #[test]
    fn rank_battle_help_never_counts_the_players_own_escort() {
        let mut escort = hired(2);
        escort.govt = Some(GovtId(142));
        escort.goal = Goal::Attack(ShipRef::Player);
        assert!(!by_enemy(&[flying(1, Goal::Idle), escort], false));
    }

    #[test]
    fn the_threats_to_the_player_are_every_other_npc_that_threatens_it() {
        let rivals = rivals();
        let npcs = [
            of_govt(1, Some(142), Goal::Attack(ShipRef::Player)),
            of_govt(2, Some(142), Goal::Snipe(ShipRef::Player)),
            of_govt(3, Some(143), Goal::Idle),
            of_govt(4, None, Goal::Flee(ShipRef::Player)),
        ];
        let hail = hailing(&npcs, &rivals, 0, &talk());
        let ids: Vec<NpcId> = hail.threats_to_player().map(|npc| npc.id).collect();
        assert_eq!(ids, [NpcId(2), NpcId(4)]);
    }

    #[test]
    fn the_flags_are_the_originals() {
        assert_eq!(
            [MUTE, BRIBABLE_WARSHIPS, PLUNDERS, BRIBABLE_TRADERS, GREEDY],
            [0x0400, 0x0200, 0x1000, 0x2000, 0x8000]
        );
        assert_eq!([UNTALKATIVE, QUIET, FREE_HELP], [0x0001, 0x0008, 0x0010]);
    }

    /// An option that applies while the ship is busy, and says `STR#`
    /// 9000 #1.
    #[derive(Debug)]
    struct WhileBusy;

    impl HailOption for WhileBusy {
        fn label(&self) -> String {
            "Busy?".to_owned()
        }

        fn applies(&self, hail: &Hail) -> bool {
            hail.busy
        }

        fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
            Answer::say(Reply::Line {
                list: 9000,
                index: 1,
            })
        }
    }

    fn labels(options: &HailOptions, hail: &Hail) -> Vec<String> {
        options
            .listed(hail)
            .iter()
            .map(|option| option.label())
            .collect()
    }

    #[test]
    fn nova_lists_greetings_then_the_middle_option_that_applies_and_release_for_an_escort() {
        let options = HailOptions::nova(&Rulebook::default());
        let npc = ship();
        let friendly = hail(&npc);
        assert_eq!(
            labels(&options, &friendly),
            ["Greetings", "Request Assistance"]
        );
        let hostile = Hail {
            attitude: Attitude::Hostile,
            ..friendly
        };
        assert_eq!(labels(&options, &hostile), ["Greetings", "Beg For Mercy"]);
        let untalkative = Hail {
            dispositions: Dispositions {
                untalkative: true,
                ..Dispositions::default()
            },
            ..hostile
        };
        assert_eq!(labels(&options, &untalkative), ["Greetings"]);
        let mut escort = ship();
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        assert_eq!(
            labels(&options, &hail(&escort)),
            ["Release"],
            "the escort's"
        );
        assert_eq!(labels(&HailOptions::default(), &hail(&escort)), ["Release"]);
    }

    #[test]
    fn nova_lists_use_as_escort_for_a_joining_person_under_person_join_alone() {
        let npc = ship();
        let joining = Hail {
            joins: true,
            ..hail(&npc)
        };
        let keys = |options: &HailOptions| {
            options
                .listed(&joining)
                .iter()
                .map(|option| (option.label(), option.key()))
                .collect::<Vec<_>>()
        };
        let bible = HailOptions::nova(
            &Rulebook::default().with_override(RuleKey::PersonJoin, RuleSource::Bible),
        );
        assert_eq!(
            keys(&bible),
            [
                ("Greetings".to_owned(), Some('G')),
                ("Request Assistance".to_owned(), Some('R')),
                ("Use As Escort".to_owned(), Some('U')),
            ]
        );
        assert_eq!(
            labels(&HailOptions::nova(&Rulebook::default()), &joining),
            ["Greetings", "Request Assistance"],
            "by the engine"
        );
        assert!(!hail(&npc).joins, "the fixture's hail may not join");
        assert!(bible.lists_join(&joining), "Use As Escort joins");
        assert!(!bible.lists_join(&hail(&npc)), "nothing to join");
        assert!(
            !HailOptions::nova(&Rulebook::default()).lists_join(&joining),
            "by the engine"
        );
    }

    #[test]
    fn at_most_one_middle_option_applies_and_exactly_one_to_a_talkative_ship() {
        let options = HailOptions::default();
        let npc = ship();
        for attitude in [Attitude::Friendly, Attitude::Unfriendly, Attitude::Hostile] {
            for untalkative in [false, true] {
                let hail = Hail {
                    attitude,
                    dispositions: Dispositions {
                        untalkative,
                        ..Dispositions::default()
                    },
                    ..hail(&npc)
                };
                let middle = options.listed(&hail).len() - 1;
                let expected = usize::from(!untalkative);
                assert_eq!(middle, expected, "{attitude:?} {untalkative}");
            }
        }
    }

    #[test]
    fn an_option_registered_with_is_listed_after_exactly_when_it_applies() {
        let options = HailOptions::default().with(Rc::new(WhileBusy));
        let npc = ship();
        let idle = hail(&npc);
        assert_eq!(labels(&options, &idle), ["Greetings", "Request Assistance"]);
        let busy = Hail { busy: true, ..idle };
        assert_eq!(
            labels(&options, &busy),
            ["Greetings", "Request Assistance", "Busy?"]
        );
        let listed = options.listed(&busy);
        assert_eq!(listed[2].key(), None, "no hotkey by default");
        assert_eq!(
            listed[2].press(&busy, &mut crate::NeverFires),
            Answer::say(Reply::Line {
                list: 9000,
                index: 1
            })
        );
        assert!(HailOptions::empty().listed(&busy).is_empty());
    }

    #[test]
    fn the_rulebook_reaches_each_options_quiet_hails() {
        let npc = ship();
        let quiet = Hail {
            dispositions: Dispositions {
                quiet: true,
                ..Dispositions::default()
            },
            need: Some(Help::Refuel),
            ..hail(&npc)
        };
        let hostile = Hail {
            attitude: Attitude::Hostile,
            dispositions: Dispositions {
                bribable: true,
                ..quiet.dispositions
            },
            ..quiet
        };
        let press = |options: &HailOptions, hail: &Hail, at: usize| {
            options.listed(hail)[at].press(hail, &mut crate::NeverFires)
        };
        let engine = HailOptions::default();
        assert_ne!(press(&engine, &quiet, 0), Answer::say(Reply::Comm(1)));
        assert_eq!(press(&engine, &quiet, 1), Answer::default());
        assert_eq!(press(&engine, &hostile, 1), Answer::default());
        let bible = HailOptions::nova(
            &Rulebook::default().with_override(RuleKey::QuietHails, RuleSource::Bible),
        );
        assert_eq!(press(&bible, &quiet, 0), Answer::say(Reply::Comm(1)));
        assert_ne!(press(&bible, &quiet, 1), Answer::default());
        assert_ne!(press(&bible, &hostile, 1), Answer::default());
    }

    #[test]
    fn the_rulebook_reaches_greetings_long_advice() {
        let npc = ship();
        let greeting = |rulebook: &Rulebook| {
            let options = HailOptions::nova(rulebook);
            let friendly = hail(&npc);
            options.listed(&friendly)[0].press(&friendly, &mut crate::NeverFires)
        };
        let advice = |long_advice| {
            Answer::say(Reply::Advice {
                list: reply::GREETINGS.0,
                index: reply::GREETINGS.1,
                greet_if_blank: false,
                patch: None,
                long_advice,
            })
        };
        assert_eq!(greeting(&Rulebook::default()), advice(RuleSource::Engine));
        assert_eq!(
            greeting(&Rulebook::default().with_override(RuleKey::LongAdvice, RuleSource::Bible)),
            advice(RuleSource::Bible)
        );
    }
}
