//! The player's escorts: their classes, the standing orders the player
//! gives them, and the commands that set those orders.
//!
//! The values are the original's (`_LoadObjectData`, `_HandlePlayer`
//! and `_IssueNewEscortCommand` in the `EV Nova` executable).
//!
//! **Classes** ([`EscortClass::of`], `_LoadObjectData` @0x7a45f-0x7a473):
//! a ship's class is its `shïp` `EscortType`, 0 Fighter, 1 Medium, 2
//! Warship and 3 Freighter. Any other value (the Bible names only -1,
//! "figured out at runtime") is replaced: a ship of `InherentAI` 2 or
//! less is a Freighter; otherwise one of `Mass` 49 or less a Fighter, of
//! 199 or less a Medium ship, and any heavier a Warship. Every stock
//! `shïp` has 0-3.
//!
//! **Orders** ([`EscortOrder`]): Defend, Attack and Hold Position; an
//! escort with none ([`None`]) keeps formation, the original's
//! "Formation". The player commands ([`EscortCommand`]) the whole fleet
//! or one class ([`EscortGroup`]): F attacks the player's target, D
//! defends the player, V holds position and C recalls the escorts to
//! formation. Option-C is Return to Hangar (command 3, `_HandlePlayer`
//! @0x6e207, `STR#` 2002 #148 and #155): the carried fighters it reaches
//! fly back to dock, and any other escort goes back to formation, the
//! message then saying "returning to formation." (`_IssueNewEscortCommand`
//! @0x6619b-0x661d4). Each escort keeps its own order, saved with the pilot. The
//! original keeps four class orders, saves them, and resets them to
//! Formation on every system entry (`_RespawnEscort` @0x3d546,
//! `_HandlePlayer` @0x6e3bb-0x6e44b); here that reset is the engine's
//! reading of the rulebook's
//! [`RuleKey::EscortOrders`](crate::RuleKey::EscortOrders), and the
//! other reading keeps the orders (see the session).
//!
//! **Formation** ([`number_formation`], [`spacing`], [`slot_offset`], [`slot_position`];
//! `_AICalcFormationPositions` @0x85040, `_AICalcShipFormPos` @0x84cb8):
//! the player leads, in slot 1, and the ships it leads take slots 2, 3, …
//! in ship-table order (their berths): its hired escorts and its mission
//! escorts together (a `ShipBehav` 1 mission's ships, which hold a
//! [`FormationSlot`] of their own and no [`EscortDuty`]; see the
//! session's `mission_escorts` module), each a number of spacings
//! behind and beside the player along its heading. The spacing is six tenths of the widest sprite,
//! held between 24 and 60 pixels. An escort within [`KEEP_FORMATION`] of
//! the player matches the player's velocity and moves with it, nudged
//! back towards its slot (`_AIMaintainFormation` @0x84f04); farther off
//! it flies toward the player (see
//! [`formation_flight`](crate::traffic::formation_flight)). A ship
//! it leads that attacks with a swarm leader forms on that ship instead,
//! so it leaves the player's formation and the rest close up
//! ([`swarm::formation_parent`](crate::ai::swarm::formation_parent); see
//! the session's `swarm_formation` module).
//!
//! **Threats** ([`threats`], [`threat_score`], [`best_threat`];
//! `_IsShipANearbyThreatToParent` @0x836a4): an escort fights the ships
//! threatening the player or a ship the player leads, scored by distance,
//! a ship of another class counting half, and a fighter against a warship
//! escort a quarter; a
//! defending one looks within [`DEFEND_RADIUS`] of the player and keeps
//! its target within [`DEFEND_KEEP_SQ`] (see
//! [`ai::escort`](crate::ai::escort)). The same searches serve an escort
//! with an NPC parent ([`Parent`], [`fight_threats`], [`threat_score_to`],
//! [`player_threat_score`], [`nearest_threat`]): the ships threatening the
//! lead or a ship it leads, and the player when the lead threatens it
//! (see [`ai::fleet_escort`](crate::ai::fleet_escort)).
//!
//! The strings are `STR#` 2002's: #51, #133, #134 and #154 the constants
//! below, the labels each type gives, #155-#159 what a command's message
//! says the escorts are doing ([`EscortCommand::doing`]) and #150-#154
//! what they will do, said of a command given while a jump is under way
//! ([`EscortCommand::will`], `_IssueNewEscortCommand` @0x66461-0x664e5).

use crate::ai::Surroundings;
pub use crate::catalog::{EscortClass, FIGHTER_MASS, FREIGHTER_AI, MEDIUM_MASS};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
pub use crate::fleet::EscortOrder;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::traffic::npc::{Npc, NpcId};
use crate::trig::{accel, whole};

/// `STR#` 2002 #51: E with no escorts.
pub const NO_ESCORTS: &str = "You don't have any escorts.";
/// `STR#` 2002 #133: the escort menu's title.
pub const ESCORT_COMMANDS: &str = "Escort Commands";
/// `STR#` 2002 #134: what a command's message starts with (two spaces
/// at its end).
pub const NEW_ORDERS: &str = "New escort orders assigned:  ";
/// `STR#` 2002 #154: Attack's message with no target to copy.
pub const WILL_ATTACK: &str = "will attack.";

/// The escort menu's label for `order` (`STR#` 2002 #145-#149): none
/// is "Formation".
#[must_use]
pub const fn order_label(order: Option<EscortOrder>) -> &'static str {
    match order {
        None => "Formation",
        Some(EscortOrder::Defend) => "Defend",
        Some(EscortOrder::Attack) => "Attack",
        Some(EscortOrder::Dock) => "Return to Hangar",
        Some(EscortOrder::Hold) => "Hold Position",
    }
}

/// A command the player gives its escorts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortCommand {
    /// F: attack the player's target.
    Attack,
    /// D: defend the player.
    Defend,
    /// V: hold position.
    Hold,
    /// C: back to formation.
    Recall,
    /// Option-C: Return to Hangar, for the carried fighters; the other
    /// escorts it reaches go back to formation.
    Dock,
}

impl EscortCommand {
    /// The standing order it gives: none for a recall.
    #[must_use]
    pub const fn order(self) -> Option<EscortOrder> {
        match self {
            Self::Attack => Some(EscortOrder::Attack),
            Self::Defend => Some(EscortOrder::Defend),
            Self::Hold => Some(EscortOrder::Hold),
            Self::Dock => Some(EscortOrder::Dock),
            Self::Recall => None,
        }
    }

    /// What its message says the escorts do (`STR#` 2002 #155-#159).
    #[must_use]
    pub const fn doing(self) -> &'static str {
        match self {
            Self::Attack => "attacking target.",
            Self::Defend => "defending.",
            Self::Hold => "holding position.",
            Self::Recall => "returning to formation.",
            Self::Dock => "returning to hangar.",
        }
    }

    /// What its message says the escorts will do when it is given while
    /// a jump is under way (`STR#` 2002 #150-#154,
    /// `_IssueNewEscortCommand` @0x66461-0x664e0). Attack says
    /// [`WILL_ATTACK`] whether or not the escorts took a target.
    #[must_use]
    pub const fn will(self) -> &'static str {
        match self {
            Self::Attack => WILL_ATTACK,
            Self::Defend => "will defend.",
            Self::Hold => "will hold position.",
            Self::Recall => "will return to formation.",
            Self::Dock => "will return to hangar.",
        }
    }
}

/// The escorts a command goes to: every one, or one class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortGroup {
    /// Every escort.
    All,
    /// The escorts of one class.
    Class(EscortClass),
}

impl EscortGroup {
    /// Every group, in the escort menu's order (keys 1-5).
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Class(EscortClass::Fighter),
        Self::Class(EscortClass::Medium),
        Self::Class(EscortClass::Warship),
        Self::Class(EscortClass::Freighter),
    ];

    /// Whether an escort of `class` is in it.
    #[must_use]
    pub fn holds(self, class: EscortClass) -> bool {
        match self {
            Self::All => true,
            Self::Class(own) => own == class,
        }
    }

    /// Its row in the escort menu (`STR#` 2002 #144, #140-#143).
    #[must_use]
    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::All => "All Ships",
            Self::Class(class) => class.menu_label(),
        }
    }

    /// What a command's message calls it (`STR#` 2002 #139, #135-#138).
    #[must_use]
    pub const fn message_form(self) -> &'static str {
        match self {
            Self::All => "All ships",
            Self::Class(class) => class.message_form(),
        }
    }
}

/// What a command to the escorts changed, for its message: the group
/// it went to, the command, whether Attack had a target to report, and
/// whether a jump was under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commanded {
    /// The escorts it went to.
    pub group: EscortGroup,
    /// The command.
    pub command: EscortCommand,
    /// Whether Attack had the player's target to report ("attacking
    /// target.").
    pub targeted: bool,
    /// Whether it was given while a jump was under way, the pre-jump
    /// stage included (the original's ship +0x50 > 0, @0x66466): its
    /// message then says what the escorts will do ([`EscortCommand::will`]).
    pub underway: bool,
}

/// One class's row in the escort menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassRow {
    /// The class.
    pub class: EscortClass,
    /// Whether the player has an escort of it.
    pub present: bool,
    /// The standing order of its first escort; none for formation, or
    /// with no escort of it.
    pub order: Option<EscortOrder>,
}

/// An escort's duty in the system: its place in the formation and the
/// standing order it follows. An NPC carries one while it escorts the
/// player; it is never saved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EscortDuty {
    /// Its place in the formation: slot 2, 3, … by its place in the
    /// fleet, the player being slot 1.
    pub formation: FormationSlot,
    /// Its standing order; none keeps formation.
    pub order: Option<EscortOrder>,
}

/// A place in the player's formation, held by a hired escort (its
/// [`EscortDuty`]) or a mission escort (its mission's mark): its slot,
/// how many ships the formation holds, and its spacing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormationSlot {
    /// Its slot in the formation ([`slot_offset`]): 2, 3, …, the player
    /// being slot 1.
    pub slot: u8,
    /// How many ships the formation holds, the player included.
    pub ships: u8,
    /// The formation's spacing, in pixels ([`spacing`]).
    pub spacing: f32,
}

impl FormationSlot {
    /// Where it is behind a leader at `leader` ([`slot_position`]).
    #[must_use]
    pub fn position(&self, leader: &ShipState) -> Vec2 {
        slot_position(leader, self.slot, self.ships, self.spacing)
    }
}

/// How near its leader, in pixels on each axis, inclusive, an escort
/// keeps close formation (300.0 @0xdd95c; goal 0xa's substate 0xc, see
/// [`formation_flight`](crate::traffic::formation_flight)); by the port's
/// earlier formation, how near its slot.
pub const KEEP_FORMATION: f32 = 300.0;
/// How near its leader, in pixels on each axis, inclusive, an escort
/// closes in; farther off it approaches at full speed (600.0 @0xdda54;
/// substates 0xb and 9). By the port's earlier formation, how near its
/// slot it closes on it.
pub const APPROACH_FORMATION: f32 = 600.0;
/// How far off its slot, in pixels on an axis, an escort in close
/// formation is nudged back from: from 8 off, inclusive (8.0 @0xdd044,
/// `_AIMaintainFormation`); by the port's earlier formation, beyond it.
pub const FORMATION_SLACK: f32 = 8.0;
/// How many ticks' acceleration the nudge back to the slot is (10.0
/// @0xdd65c): the whole of it, which can carry it past the slot; by the
/// port's earlier formation, at most that, never past it.
pub const FORMATION_NUDGE: f32 = 10.0;

/// The share of the widest sprite the formation is spaced by (0.6
/// @0xdda70).
pub const SPACING_SHARE: f32 = 0.6;
/// The least spacing, in pixels.
pub const MIN_SPACING: f32 = 24.0;
/// The most spacing, in pixels.
pub const MAX_SPACING: f32 = 60.0;
/// How wide a leader with no sprite counts, in pixels.
pub const NO_SPRITE: f32 = 64.0;
/// The highest slot with a place of its own: any later one sits on the
/// leader.
pub const LAST_SLOT: u8 = 21;

/// The spacing of a formation led by a ship `leader` pixels wide (none
/// without a sprite, counting as [`NO_SPRITE`]) with followers
/// `followers` wide (`_AICalcFormationPositions` @0x85040): the widest
/// times [`SPACING_SHARE`], truncated, held between [`MIN_SPACING`] and
/// [`MAX_SPACING`].
#[must_use]
pub fn spacing(leader: Option<f32>, followers: &[Option<f32>]) -> f32 {
    let widest = followers
        .iter()
        .flatten()
        .fold(leader.unwrap_or(NO_SPRITE), |widest, &width| {
            widest.max(width)
        });
    (widest * SPACING_SHARE)
        .trunc()
        .clamp(MIN_SPACING, MAX_SPACING)
}

/// The slots of one formation: its parent `parent_width` pixels wide
/// (none without a sprite) and its `followers`, every ship whose
/// formation parent it is (`_AICalcFormationPositions`
/// @0x85137-0x851cc). The followers take slots 2, 3, … in ship-table
/// order, by berth (ships at one berth in the order given), out of
/// `ships`, the followers and the parent, all spaced alike by the widest
/// sprite among them ([`spacing`]). Which ships follow, and the parent's
/// width, are the caller's: the player's formation is the session's,
/// an NPC's the traffic's ([`fleet_formation`](crate::traffic::fleet_formation)).
#[must_use]
pub fn number_formation<'a>(
    parent_width: Option<f32>,
    followers: impl IntoIterator<Item = &'a Npc>,
) -> Vec<(NpcId, FormationSlot)> {
    let mut followers: Vec<&Npc> = followers.into_iter().collect();
    followers.sort_by_key(|npc| npc.berth);
    let ships = u8::try_from(followers.len() + 1).unwrap_or(u8::MAX);
    let widths: Vec<Option<f32>> = followers.iter().map(|npc| npc.hull.sprite_size).collect();
    let spacing = spacing(parent_width, &widths);
    followers
        .iter()
        .enumerate()
        .map(|(place, npc)| {
            let slot = FormationSlot {
                slot: u8::try_from(place + 2).unwrap_or(u8::MAX),
                ships,
                spacing,
            };
            (npc.id, slot)
        })
        .collect()
}

/// Where slot `slot` of a formation of `ships` sits, in spacings: how far
/// behind the leader, and how far to its right (negative to its left)
/// (`_AICalcShipFormPos` @0x84cb8). Slot 1 is the leader; slots 4-8
/// depend on whether `ships` is even; slots past [`LAST_SLOT`] sit on the
/// leader.
#[must_use]
pub fn slot_offset(slot: u8, ships: u8) -> (f32, f32) {
    const EVEN: [(f32, f32); 5] = [(2.0, 0.0), (2.0, -2.0), (2.0, 2.0), (3.0, -1.0), (3.0, 1.0)];
    const ODD: [(f32, f32); 5] = [(2.0, -2.0), (2.0, 2.0), (3.0, -1.0), (3.0, 1.0), (2.0, 0.0)];
    const LATER: [(f32, f32); 13] = [
        (3.0, -3.0),
        (3.0, 3.0),
        (4.0, 0.0),
        (4.0, -2.0),
        (4.0, 2.0),
        (4.0, -4.0),
        (4.0, 4.0),
        (5.0, -1.0),
        (5.0, 1.0),
        (5.0, -3.0),
        (5.0, 3.0),
        (5.0, -5.0),
        (5.0, 5.0),
    ];
    match slot {
        2 => (1.0, -1.0),
        3 => (1.0, 1.0),
        4..=8 if ships.is_multiple_of(2) => EVEN[usize::from(slot - 4)],
        4..=8 => ODD[usize::from(slot - 4)],
        9..=LAST_SLOT => LATER[usize::from(slot - 9)],
        _ => (0.0, 0.0),
    }
}

/// Where slot `slot` of a formation of `ships` spaced `spacing` apart
/// is, behind a leader at `leader` along its heading; its right is the
/// heading turned 90 degrees clockwise (`_AICalcShipFormPos`
/// @0x84e64-0x84ef7). Both pushes go by whole degrees and the table sine
/// and cosine ([`trig::accel`](crate::trig::accel)): behind along
/// trunc(heading), to the right along trunc(heading + 90.0) % 360 (90.0
/// @0xdd4d8, C's signed remainder), summed from nothing and then added
/// to the leader's position, in that order.
#[must_use]
pub fn slot_position(leader: &ShipState, slot: u8, ships: u8, spacing: f32) -> Vec2 {
    let (behind, right) = slot_offset(slot, ships);
    let side = whole(leader.heading + 90.0) % 360;
    accel(whole(leader.heading), -behind * spacing) + accel(side, right * spacing) + leader.position
}

/// The radius around the player, in pixels, a defending escort takes a
/// new threat within (550 @0x83c56).
pub const DEFEND_RADIUS: f32 = 550.0;
/// The distance squared from the player within which a defending escort
/// keeps its target (408375.0 @0xdda28, about 639 pixels).
pub const DEFEND_KEEP_SQ: f32 = 408_375.0;

/// The ships among `around` that threaten the player, which `escort` may
/// fight (`_AIFightThreatToParent` @0x81de6, with the player the parent):
/// any but `escort` itself that `_ExtendedIsThreatToPlayer` @0x81b70
/// holds for (@0x81e5c, @0x81e96, @0x81f29, @0x81f7d), read whole as
/// [`Npc::threatens_player_extended`] with the player's sight of it
/// ([`Surroundings::seen_by_player`]). So a ship the player can't see is
/// none, nor is a fellow escort, hired or a mission escort (@0x81b9f); a
/// ship attacking the player or a ship it leads, one whose hit's target
/// is the player, and a lead whose escort fights the player are. The
/// stellar-defender clause (+0x8c, @0x81bd3) is left out, as the engine
/// has no defence ships (rdm:task/planet-domination).
/// `_AIFightThreatToParent` makes no sight test of its own.
pub fn threats<'a>(
    escort: &'a Npc,
    around: &'a Surroundings<'_>,
) -> impl Iterator<Item = &'a Npc> + 'a {
    around
        .npcs
        .iter()
        .filter(move |candidate| candidate.id != escort.id && around.threatens_player(candidate))
}

/// The ship an escort guards, its parent (+0x9a): the player, where it
/// is, or an NPC lead. The threat searches read it
/// ([`fight_threats`], [`threat_score_to`], [`nearest_threat`]).
#[derive(Clone, Copy, Debug)]
pub enum Parent<'a> {
    /// The player, at this place.
    Player(&'a ShipState),
    /// An NPC lead: a fleet's lead, a pirate that took the escort in, or
    /// a fighter's lead.
    Npc(&'a Npc),
}

impl Parent<'_> {
    /// Where it is.
    fn at(self) -> Vec2 {
        match self {
            Self::Player(state) => state.position,
            Self::Npc(lead) => lead.state.position,
        }
    }
}

/// The ships among `around` that threaten `parent`, which `escort` may
/// fight (`_AIFightThreatToParent` @0x81de6-0x81fae, whole), in the
/// original's slot order, the player first:
///
/// - With the player the parent, the NPCs [`threats`] yields.
/// - With an NPC lead, the player when it is in the system, not disabled,
///   and `_ExtendedIsThreatToPlayer(lead)` holds (@0x81e83-0x81e9d,
///   [`Surroundings::threatens_player`]: the player's sight of the lead,
///   no blessing); and every NPC but `escort` and the lead that threatens
///   the lead by `_ExtendedIsThreatToShip(lead, ship)` (@0x81e74,
///   [`Npc::threatens_npc_among`], which fails a disabled ship).
///
/// It makes no sight test of its own. The test of a ship's system (+0x74)
/// is not modelled, as every ship the engine holds is in the player's
/// system; the count's missing active test can't matter, as both threat
/// tests fail an inactive slot (@0x81a33, @0x81b82).
#[must_use]
pub fn fight_threats(escort: &Npc, parent: Parent, around: &Surroundings) -> Vec<ShipRef> {
    match parent {
        Parent::Player(_) => threats(escort, around)
            .map(|threat| ShipRef::Npc(threat.id))
            .collect(),
        Parent::Npc(lead) => {
            let player = around
                .player
                .filter(|player| player.condition == Condition::Intact)
                .filter(|_| around.threatens_player(lead))
                .map(|_| ShipRef::Player);
            player
                .into_iter()
                .chain(
                    around
                        .npcs
                        .iter()
                        .filter(|candidate| {
                            candidate.id != escort.id
                                && candidate.id != lead.id
                                && candidate.threatens_npc_among(lead.id, around.npcs)
                        })
                        .map(|threat| ShipRef::Npc(threat.id)),
                )
                .collect()
        }
    }
}

/// How strongly `escort` takes `candidate` for a threat to the player at
/// `player`, among `around` (`_IsShipANearbyThreatToParent` @0x836a4); the
/// lowest wins, and none is no threat: [`threat_score_to`] with the player
/// the parent.
#[must_use]
pub fn threat_score(
    escort: &Npc,
    candidate: &Npc,
    around: &Surroundings,
    player: &ShipState,
    radius: Option<f32>,
) -> Option<f32> {
    threat_score_to(escort, candidate, Parent::Player(player), around, radius)
}

/// How strongly `escort` takes the NPC `candidate` for a threat to
/// `parent` among `around` (`_IsShipANearbyThreatToParent`
/// @0x836a4-0x83860); the lowest wins, and none is no threat. The
/// candidate must be other than the escort, one the escort sees
/// (`_ShipVisibleToShip(candidate, escort)` @0x8370e,
/// [`Surroundings::sees`]), and threaten the parent (@0x83735-0x83768):
/// the player as [`threats`] asks it (`_ExtendedIsThreatToPlayer`
/// @0x83744), an NPC lead, never the lead itself, by
/// `_ExtendedIsThreatToShip(lead, candidate)`
/// ([`Npc::threatens_npc_among`]). The score is `scored`'s.
///
/// The tests of the candidate's own parent and of a sibling (@0x836f1)
/// coincide with the threat tests' leader exclusions (@0x81a5a,
/// @0x81b9f), and the test of a disabled candidate by its own order word
/// (@0x83727) is dead, as both threat tests fail a disabled ship
/// (@0x81a43, @0x81b92): none is built.
#[must_use]
pub fn threat_score_to(
    escort: &Npc,
    candidate: &Npc,
    parent: Parent,
    around: &Surroundings,
    radius: Option<f32>,
) -> Option<f32> {
    if candidate.id == escort.id || !around.sees(escort, ShipRef::Npc(candidate.id)) {
        return None;
    }
    let threatens = match parent {
        Parent::Player(_) => around.threatens_player(candidate),
        Parent::Npc(lead) => {
            candidate.id != lead.id && candidate.threatens_npc_among(lead.id, around.npcs)
        }
    };
    if !threatens {
        return None;
    }
    scored(
        escort,
        candidate.state.position,
        candidate.class,
        parent.at(),
        radius,
    )
}

/// How strongly `escort` takes the player for a threat to its NPC `lead`
/// among `around` (`_IsShipANearbyThreatToParent` with slot 0 the
/// candidate): none with no player in the system, one the escort doesn't
/// see ([`Surroundings::sees`], @0x8370e, tested before the disabled test),
/// a disabled one, one the
/// lead's government blesses with peace, or one the lead doesn't threaten
/// by the narrow `_IsThreatToPlayer(lead)` (`_ExtendedIsThreatToShip`
/// @0x81a89-0x81a9c, [`Npc::threatens_player_among`]). The
/// score is `scored`'s, the class the player's
/// ([`PlayerSide::class`](crate::ai::PlayerSide::class)).
#[must_use]
pub fn player_threat_score(
    escort: &Npc,
    lead: &Npc,
    around: &Surroundings,
    radius: Option<f32>,
) -> Option<f32> {
    let player = around.player?;
    if !around.sees(escort, ShipRef::Player)
        || player.condition != Condition::Intact
        || around.blessed(lead.govt)
        || !lead.threatens_player_among(around.npcs)
    {
        return None;
    }
    scored(
        escort,
        player.state.position,
        player.class,
        lead.state.position,
        radius,
    )
}

/// The score of a threat at `at`, of class `class`, to `escort`, its parent
/// at `parent` (@0x837a5-0x83850): the distance squared from the escort;
/// with a `radius` around the parent, the threat must be within it,
/// inclusive, and its distance squared from the parent is added. It is
/// halved for a threat of another class than the escort's, and halved
/// again for a fighter against a warship escort. The original truncates
/// the squares to integers and holds the score at least 1; the engine
/// keeps floats, and the floor changes no pick.
fn scored(
    escort: &Npc,
    at: Vec2,
    class: EscortClass,
    parent: Vec2,
    radius: Option<f32>,
) -> Option<f32> {
    let squared = |off: Vec2| off.x * off.x + off.y * off.y;
    let mut score = squared(at - escort.state.position);
    if let Some(radius) = radius {
        let from_parent = squared(at - parent);
        if from_parent > radius * radius {
            return None;
        }
        score += from_parent;
    }
    if class != escort.class {
        score /= 2.0;
        if class == EscortClass::Fighter && escort.class == EscortClass::Warship {
            score /= 2.0;
        }
    }
    Some(score)
}

/// The threat to the player at `player` among `around` that `escort` takes:
/// [`nearest_threat`] with the player the parent. Only candidates
/// [`threat_score`] accepts count, so both the escort's sight of a
/// candidate ([`Surroundings::sees`]) and the player's
/// ([`Surroundings::seen_by_player`]) gate the pick.
#[must_use]
pub fn best_threat(
    escort: &Npc,
    around: &Surroundings,
    player: &ShipState,
    radius: Option<f32>,
) -> Option<NpcId> {
    match nearest_threat(escort, Parent::Player(player), around, radius) {
        Some(ShipRef::Npc(id)) => Some(id),
        _ => None,
    }
}

/// The threat to `parent` among `around` that `escort` takes
/// (`_FindNearestThreatToParent` @0x83861-0x838d1): the lowest score, the
/// earliest slot of a tie, as only a strictly lower score replaces the
/// pick. Slot 0, the player, comes first ([`player_threat_score`], with
/// an NPC lead alone), then the NPCs by slot ([`Surroundings::by_slot`],
/// [`threat_score_to`]).
#[must_use]
pub fn nearest_threat(
    escort: &Npc,
    parent: Parent,
    around: &Surroundings,
    radius: Option<f32>,
) -> Option<ShipRef> {
    let player = match parent {
        Parent::Player(_) => None,
        Parent::Npc(lead) => {
            player_threat_score(escort, lead, around, radius).map(|score| (ShipRef::Player, score))
        }
    };
    let npcs = around.by_slot().into_iter().filter_map(|candidate| {
        threat_score_to(escort, candidate, parent, around, radius)
            .map(|score| (ShipRef::Npc(candidate.id), score))
    });
    let mut best: Option<(ShipRef, f32)> = None;
    for (ship, score) in player.into_iter().chain(npcs) {
        if best.is_none_or(|(_, lowest)| score < lowest) {
            best = Some((ship, score));
        }
    }
    best.map(|(ship, _)| ship)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_class_of_0_to_3_is_taken_as_it_is_whatever_the_ai_or_mass() {
        for (raw, class) in [
            (0, EscortClass::Fighter),
            (1, EscortClass::Medium),
            (2, EscortClass::Warship),
            (3, EscortClass::Freighter),
        ] {
            for (ai, mass) in [(1, 10), (2, 500), (3, 10), (4, 500)] {
                assert_eq!(EscortClass::of(raw, ai, mass), class, "{raw} {ai} {mass}");
            }
        }
    }

    #[test]
    fn any_other_class_is_worked_out_from_the_ai_then_the_mass() {
        for raw in [-1, 4, i16::MIN, i16::MAX] {
            assert_eq!(EscortClass::of(raw, 2, 10), EscortClass::Freighter);
            assert_eq!(EscortClass::of(raw, 1, 500), EscortClass::Freighter);
            assert_eq!(EscortClass::of(raw, 3, 49), EscortClass::Fighter);
            assert_eq!(EscortClass::of(raw, 3, 50), EscortClass::Medium);
            assert_eq!(EscortClass::of(raw, 4, 199), EscortClass::Medium);
            assert_eq!(EscortClass::of(raw, 3, 200), EscortClass::Warship);
        }
        assert_eq!((FIGHTER_MASS, MEDIUM_MASS, FREIGHTER_AI), (49, 199, 2));
    }

    #[test]
    fn each_command_gives_its_order_and_recall_none() {
        assert_eq!(EscortCommand::Attack.order(), Some(EscortOrder::Attack));
        assert_eq!(EscortCommand::Defend.order(), Some(EscortOrder::Defend));
        assert_eq!(EscortCommand::Hold.order(), Some(EscortOrder::Hold));
        assert_eq!(EscortCommand::Recall.order(), None);
        assert_eq!(EscortCommand::Dock.order(), Some(EscortOrder::Dock));
    }

    #[test]
    fn the_labels_are_their_strings() {
        assert_eq!(
            EscortGroup::ALL.map(EscortGroup::menu_label),
            [
                "All Ships",
                "Fighters",
                "Medium Ships",
                "Warships",
                "Freighters"
            ]
        );
        assert_eq!(
            EscortGroup::ALL.map(EscortGroup::message_form),
            [
                "All ships",
                "Fighters",
                "Medium ships",
                "Warships",
                "Freighters"
            ]
        );
        assert_eq!(
            [
                None,
                Some(EscortOrder::Defend),
                Some(EscortOrder::Attack),
                Some(EscortOrder::Hold),
                Some(EscortOrder::Dock)
            ]
            .map(order_label),
            [
                "Formation",
                "Defend",
                "Attack",
                "Hold Position",
                "Return to Hangar"
            ]
        );
        assert_eq!(
            [
                EscortCommand::Attack,
                EscortCommand::Defend,
                EscortCommand::Hold,
                EscortCommand::Recall,
                EscortCommand::Dock
            ]
            .map(EscortCommand::doing),
            [
                "attacking target.",
                "defending.",
                "holding position.",
                "returning to formation.",
                "returning to hangar."
            ]
        );
        assert_eq!(NEW_ORDERS, "New escort orders assigned:  ");
        assert_eq!(WILL_ATTACK, "will attack.");
        assert_eq!(ESCORT_COMMANDS, "Escort Commands");
        assert_eq!(NO_ESCORTS, "You don't have any escorts.");
    }

    #[test]
    fn the_will_strings_are_str_2002s_150_to_154() {
        assert_eq!(
            [
                EscortCommand::Dock,
                EscortCommand::Recall,
                EscortCommand::Hold,
                EscortCommand::Defend,
                EscortCommand::Attack
            ]
            .map(EscortCommand::will),
            [
                "will return to hangar.",
                "will return to formation.",
                "will hold position.",
                "will defend.",
                WILL_ATTACK
            ]
        );
    }

    #[test]
    fn the_spacing_is_six_tenths_of_the_widest_sprite_held_between_24_and_60() {
        assert_eq!(spacing(Some(30.0), &[]), 24.0);
        assert_eq!(spacing(Some(50.0), &[]), 30.0);
        assert_eq!(spacing(Some(120.0), &[]), 60.0);
        assert_eq!(spacing(None, &[]), 38.0, "trunc(64 x 0.6)");
        assert_eq!(spacing(Some(30.0), &[Some(55.0), None, Some(41.0)]), 33.0);
        assert_eq!(spacing(None, &[Some(70.0)]), 42.0, "the widest follower");
        assert_eq!(
            (SPACING_SHARE, MIN_SPACING, MAX_SPACING, NO_SPRITE),
            (0.6, 24.0, 60.0, 64.0)
        );
    }

    /// A follower numbered `id`, at berth `berth`, `width` wide.
    fn follower(id: u32, berth: u32, width: Option<f32>) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, crate::stats::ShipStats::default());
        npc.berth = berth;
        npc.hull.sprite_size = width;
        npc
    }

    #[test]
    fn a_formation_numbers_its_followers_from_2_by_berth_out_of_them_and_the_parent() {
        let ships = [
            follower(1, 9, None),
            follower(2, 6, None),
            follower(3, 7, None),
            follower(4, 6, None),
        ];
        let slot = |slot| FormationSlot {
            slot,
            ships: 5,
            spacing: 38.0,
        };
        assert_eq!(
            number_formation(None, &ships),
            [
                (NpcId(2), slot(2)),
                (NpcId(4), slot(3)),
                (NpcId(3), slot(4)),
                (NpcId(1), slot(5)),
            ],
            "by berth; one berth's in the order given"
        );
        assert!(number_formation(None, &[]).is_empty());
    }

    #[test]
    fn a_formation_is_spaced_by_the_widest_sprite_the_parents_included() {
        let spacing_of = |parent: Option<f32>, widths: &[Option<f32>]| {
            let ships: Vec<Npc> = (1..)
                .zip(widths)
                .map(|(n, &width)| follower(n, n, width))
                .collect();
            let slots = number_formation(parent, &ships);
            assert_eq!(slots.len(), widths.len());
            assert!(
                slots
                    .iter()
                    .all(|(_, slot)| slot.spacing == slots[0].1.spacing)
            );
            slots[0].1.spacing
        };
        assert_eq!(
            spacing_of(None, &[Some(40.0), Some(70.0)]),
            42.0,
            "70 × 0.6"
        );
        assert_eq!(spacing_of(None, &[Some(20.0)]), 38.0, "64 with no sprite");
        assert_eq!(
            spacing_of(Some(50.0), &[Some(20.0), None]),
            30.0,
            "the parent's"
        );
        assert_eq!(spacing_of(Some(30.0), &[Some(20.0), Some(30.0)]), 24.0);
        assert_eq!(spacing_of(Some(30.0), &[Some(120.0)]), 60.0);
    }

    #[test]
    fn each_slot_sits_behind_and_beside_the_leader_as_the_original_places_it() {
        let even = [
            (1, (0.0, 0.0)),
            (2, (1.0, -1.0)),
            (3, (1.0, 1.0)),
            (4, (2.0, 0.0)),
            (5, (2.0, -2.0)),
            (6, (2.0, 2.0)),
            (7, (3.0, -1.0)),
            (8, (3.0, 1.0)),
        ];
        let odd = [
            (1, (0.0, 0.0)),
            (2, (1.0, -1.0)),
            (3, (1.0, 1.0)),
            (4, (2.0, -2.0)),
            (5, (2.0, 2.0)),
            (6, (3.0, -1.0)),
            (7, (3.0, 1.0)),
            (8, (2.0, 0.0)),
        ];
        for (slot, at) in even {
            assert_eq!(slot_offset(slot, 8), at, "slot {slot} of 8");
        }
        for (slot, at) in odd {
            assert_eq!(slot_offset(slot, 7), at, "slot {slot} of 7");
        }
        let later = [
            (9, (3.0, -3.0)),
            (10, (3.0, 3.0)),
            (11, (4.0, 0.0)),
            (12, (4.0, -2.0)),
            (13, (4.0, 2.0)),
            (14, (4.0, -4.0)),
            (15, (4.0, 4.0)),
            (16, (5.0, -1.0)),
            (17, (5.0, 1.0)),
            (18, (5.0, -3.0)),
            (19, (5.0, 3.0)),
            (20, (5.0, -5.0)),
            (21, (5.0, 5.0)),
        ];
        for (slot, at) in later {
            assert_eq!(slot_offset(slot, 22), at, "slot {slot}");
            assert_eq!(slot_offset(slot, 21), at, "slot {slot}, the same for odd");
        }
        for slot in [0, 22, 30, u8::MAX] {
            assert_eq!(
                slot_offset(slot, 40),
                (0.0, 0.0),
                "slot {slot} on the leader"
            );
        }
        assert_eq!(LAST_SLOT, 21);
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn a_slot_turns_with_its_leader() {
        let up = ShipState {
            position: Vec2::new(100.0, 50.0),
            ..ShipState::default()
        };
        let at = slot_position(&up, 2, 7, 30.0);
        assert!(near(at, Vec2::new(70.0, 80.0)), "behind and left: {at:?}");
        let at = slot_position(&up, 3, 7, 30.0);
        assert!(near(at, Vec2::new(130.0, 80.0)), "behind and right: {at:?}");
        let right = ShipState {
            heading: 90.0,
            ..up
        };
        let at = slot_position(&right, 2, 7, 30.0);
        assert!(near(at, Vec2::new(70.0, 20.0)), "turned 90: {at:?}");
        let at = slot_position(&right, 4, 7, 30.0);
        assert!(
            near(at, Vec2::new(40.0, -10.0)),
            "two behind, two left: {at:?}"
        );
        assert!(near(slot_position(&right, 1, 7, 30.0), right.position));
    }

    #[test]
    fn a_slot_sits_on_the_leaders_whole_degree_heading() {
        let whole = ShipState {
            position: Vec2::new(100.0, 50.0),
            heading: 45.0,
            ..ShipState::default()
        };
        let fractional = ShipState {
            heading: 45.9,
            ..whole
        };
        for slot in [2, 4] {
            assert_eq!(
                slot_position(&fractional, slot, 6, 40.0),
                slot_position(&whole, slot, 6, 40.0),
                "slot {slot}"
            );
        }
    }

    #[test]
    fn a_slots_side_push_turns_by_the_whole_degree_wrapped() {
        let leader = ShipState {
            heading: 270.5,
            ..ShipState::default()
        };
        // Slot 3 is one behind and one to the right: the right is
        // trunc(270.5 + 90) % 360 = 0, straight up.
        let at = slot_position(&leader, 3, 6, 40.0);
        let expected = crate::trig::accel(270, -40.0) + crate::trig::accel(0, 40.0);
        assert_eq!(at, expected + leader.position);
    }

    #[test]
    fn a_slot_is_the_two_table_pushes_then_the_leaders_place() {
        let leader = ShipState {
            position: Vec2::new(-1234.567, 891.25),
            heading: 123.75,
            ..ShipState::default()
        };
        // Slot 9 is three behind and three to the left.
        let at = slot_position(&leader, 9, 12, 37.0);
        let pushes = crate::trig::accel(123, -3.0 * 37.0) + crate::trig::accel(213, -3.0 * 37.0);
        assert_eq!(at, pushes + leader.position);
    }

    /// NPC `id` of `class` at (`x`, `y`), attacking the player when
    /// `threat`.
    fn ship(id: u32, class: EscortClass, x: f32, y: f32, threat: bool) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, crate::stats::ShipStats::default());
        npc.class = class;
        npc.state.position = Vec2::new(x, y);
        if threat {
            npc.goal = crate::ai::Goal::Attack(crate::combat::ShipRef::Player);
        }
        npc
    }

    const MEDIUM: EscortClass = EscortClass::Medium;

    /// Surroundings of no ships.
    fn none() -> Surroundings<'static> {
        Surroundings::new(&[], &[])
    }

    /// Surroundings of `npcs`, the player at the centre and scanning for
    /// no cloaks.
    fn among(npcs: &[Npc]) -> Surroundings<'_> {
        Surroundings::new(&[], npcs)
    }

    #[test]
    fn with_no_radius_the_nearer_threat_wins() {
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let npcs = [
            ship(2, MEDIUM, 300.0, 0.0, true),
            ship(3, MEDIUM, 0.0, -200.0, true),
            ship(4, MEDIUM, 0.0, 100.0, false),
        ];
        assert_eq!(
            threat_score(&escort, &npcs[0], &none(), &player, None),
            Some(90_000.0)
        );
        assert_eq!(
            threat_score(&escort, &npcs[1], &none(), &player, None),
            Some(40_000.0)
        );
        assert_eq!(
            best_threat(&escort, &among(&npcs), &player, None),
            Some(NpcId(3))
        );
        let far = ShipState {
            position: Vec2::new(5000.0, 0.0),
            ..ShipState::default()
        };
        assert_eq!(
            best_threat(&escort, &among(&npcs), &far, None),
            Some(NpcId(3)),
            "anywhere"
        );
    }

    #[test]
    fn with_a_radius_the_players_distance_is_added_and_one_beyond_it_is_out() {
        let player = ShipState {
            position: Vec2::new(100.0, 0.0),
            ..ShipState::default()
        };
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let inside = ship(2, MEDIUM, 650.0, 0.0, true);
        assert_eq!(
            threat_score(&escort, &inside, &none(), &player, Some(DEFEND_RADIUS)),
            Some(650.0 * 650.0 + 550.0 * 550.0)
        );
        let outside = ship(3, MEDIUM, 651.0, 0.0, true);
        assert_eq!(
            threat_score(&escort, &outside, &none(), &player, Some(DEFEND_RADIUS)),
            None
        );
        assert_eq!(
            best_threat(
                &escort,
                &among(&[outside.clone(), inside]),
                &player,
                Some(DEFEND_RADIUS)
            ),
            Some(NpcId(2))
        );
        assert_eq!(
            best_threat(&escort, &among(&[outside]), &player, Some(DEFEND_RADIUS)),
            None
        );
        assert_eq!((DEFEND_RADIUS, DEFEND_KEEP_SQ), (550.0, 408_375.0));
    }

    #[test]
    fn another_class_counts_half_and_a_fighter_against_a_warship_a_quarter() {
        let player = ShipState::default();
        let medium = ship(1, MEDIUM, 0.0, 0.0, false);
        let same = ship(2, MEDIUM, 100.0, 0.0, true);
        let other = ship(3, EscortClass::Freighter, 140.0, 0.0, true);
        assert_eq!(
            threat_score(&medium, &other, &none(), &player, None),
            Some(9800.0)
        );
        assert_eq!(
            best_threat(&medium, &among(&[same.clone(), other]), &player, None),
            Some(NpcId(3))
        );
        let other = ship(3, EscortClass::Freighter, 142.0, 0.0, true);
        assert_eq!(
            best_threat(&medium, &among(&[same, other]), &player, None),
            Some(NpcId(2))
        );
        let warship = ship(1, EscortClass::Warship, 0.0, 0.0, false);
        let fighter = ship(4, EscortClass::Fighter, 199.0, 0.0, true);
        let medium_threat = ship(5, MEDIUM, 141.0, 0.0, true);
        let warship_threat = ship(6, EscortClass::Warship, 100.0, 0.0, true);
        assert_eq!(
            threat_score(&warship, &fighter, &none(), &player, None),
            Some(199.0 * 199.0 / 4.0)
        );
        assert_eq!(
            best_threat(
                &warship,
                &among(&[warship_threat.clone(), fighter]),
                &player,
                None
            ),
            Some(NpcId(4)),
            "up to four times"
        );
        let fighter = ship(4, EscortClass::Fighter, 201.0, 0.0, true);
        assert_eq!(
            best_threat(
                &warship,
                &among(&[warship_threat.clone(), fighter]),
                &player,
                None
            ),
            Some(NpcId(6))
        );
        assert_eq!(
            best_threat(
                &warship,
                &among(&[warship_threat, medium_threat]),
                &player,
                None
            ),
            Some(NpcId(5)),
            "a medium ship counts half"
        );
        let fighter_escort = ship(1, EscortClass::Fighter, 0.0, 0.0, false);
        let warship_threat = ship(6, EscortClass::Warship, 100.0, 0.0, true);
        assert_eq!(
            threat_score(&fighter_escort, &warship_threat, &none(), &player, None),
            Some(5000.0),
            "not the other way round"
        );
    }

    #[test]
    fn the_scores_are_measured_from_wherever_the_escort_and_player_are() {
        let player = ShipState {
            position: Vec2::new(-50.0, 30.0),
            ..ShipState::default()
        };
        let escort = ship(1, MEDIUM, 100.0, -40.0, false);
        let candidate = ship(2, MEDIUM, 130.0, 0.0, true);
        assert_eq!(
            threat_score(&escort, &candidate, &none(), &player, None),
            Some(2500.0)
        );
        assert_eq!(
            threat_score(&escort, &candidate, &none(), &player, Some(DEFEND_RADIUS)),
            Some(2500.0 + 180.0 * 180.0 + 30.0 * 30.0)
        );
        let warship = ship(1, EscortClass::Warship, 100.0, -40.0, false);
        assert_eq!(
            threat_score(&warship, &candidate, &none(), &player, None),
            Some(1250.0),
            "a medium ship against a warship: half"
        );
    }

    #[test]
    fn a_tie_goes_to_the_earlier_threat() {
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let npcs = [
            ship(2, MEDIUM, 0.0, 100.0, true),
            ship(3, MEDIUM, 100.0, 0.0, true),
        ];
        assert_eq!(
            best_threat(&escort, &among(&npcs), &player, None),
            Some(NpcId(2))
        );
    }

    #[test]
    fn fellow_escorts_itself_and_non_threats_are_never_candidates() {
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, true);
        let mut fellow = ship(2, MEDIUM, 10.0, 0.0, true);
        fellow.escort = Some(EscortDuty {
            formation: FormationSlot {
                slot: 3,
                ships: 3,
                spacing: 30.0,
            },
            order: None,
        });
        let calm = ship(3, MEDIUM, 10.0, 0.0, false);
        let mut mission_escort = ship(5, MEDIUM, 5.0, 0.0, true);
        mission_escort.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::traffic::ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
        });
        for candidate in [&escort, &fellow, &calm, &mission_escort] {
            assert_eq!(
                threat_score(&escort, candidate, &none(), &player, None),
                None
            );
        }
        let npcs = [
            escort.clone(),
            fellow,
            calm,
            mission_escort,
            ship(4, MEDIUM, 900.0, 0.0, true),
        ];
        assert_eq!(
            best_threat(&escort, &among(&npcs), &player, None),
            Some(NpcId(4))
        );
        assert_eq!(
            threats(&escort, &among(&npcs))
                .map(|npc| npc.id)
                .collect::<Vec<_>>(),
            [NpcId(4)]
        );
    }

    #[test]
    fn a_ship_attacking_a_fellow_escort_is_a_threat_to_defend_against() {
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let mut fellow = ship(2, MEDIUM, 10.0, 0.0, false);
        fellow.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        let mut raider = ship(3, MEDIUM, 20.0, 0.0, false);
        raider.goal = crate::ai::Goal::Attack(crate::combat::ShipRef::Npc(NpcId(2)));
        let mut turncoat = fellow.clone();
        turncoat.id = NpcId(4);
        turncoat.goal = raider.goal;
        let npcs = [escort.clone(), fellow, raider, turncoat];
        assert_eq!(
            best_threat(&escort, &among(&npcs), &player, None),
            Some(NpcId(3))
        );
        assert_eq!(
            threats(&escort, &among(&npcs))
                .map(|npc| npc.id)
                .collect::<Vec<_>>(),
            [NpcId(3)],
            "never a fellow escort, whatever it attacks"
        );
        let unled = [
            escort.clone(),
            ship(2, MEDIUM, 10.0, 0.0, false),
            npcs[2].clone(),
        ];
        assert_eq!(best_threat(&escort, &among(&unled), &player, None), None);
        assert_eq!(threats(&escort, &among(&unled)).count(), 0);
    }

    fn cloaked(mut npc: Npc) -> Npc {
        npc.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        npc
    }

    /// The ids `escort` takes for threats among `around`: those
    /// [`threats`] yields, and [`best_threat`]'s pick.
    fn taken(escort: &Npc, around: &Surroundings) -> (Vec<NpcId>, Option<NpcId>) {
        (
            threats(escort, around).map(|npc| npc.id).collect(),
            best_threat(escort, around, &ShipState::default(), None),
        )
    }

    #[test]
    fn a_cloaked_attacker_the_player_cannot_see_is_no_threat_to_fight() {
        // The escort scans, so only the player's sight decides.
        let mut escort = ship(1, MEDIUM, 0.0, 0.0, false);
        escort.cloak_gear.scanner.on_screen = true;
        let npcs = [escort.clone(), cloaked(ship(2, MEDIUM, 50.0, 0.0, true))];
        assert_eq!(taken(&escort, &among(&npcs)), (vec![], None), "unseen");
        let scanning = Surroundings {
            cloak_scanner: true,
            ..among(&npcs)
        };
        assert_eq!(
            taken(&escort, &scanning),
            (vec![NpcId(2)], Some(NpcId(2))),
            "the player scanning on screen"
        );
    }

    #[test]
    fn a_lead_whose_escort_attacks_the_player_and_a_hit_aimed_at_it_are_threats() {
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let mut lead = ship(2, MEDIUM, 100.0, 0.0, false);
        lead.target = Some(crate::combat::ShipRef::Npc(NpcId(9)));
        let mut its_escort = ship(3, MEDIUM, 900.0, 0.0, true);
        its_escort.leader = Some(NpcId(2));
        let mut hit = ship(4, MEDIUM, 50.0, 0.0, false);
        hit.target = Some(crate::combat::ShipRef::Player);
        let npcs = [escort.clone(), lead, its_escort, hit];
        assert_eq!(
            taken(&escort, &among(&npcs)),
            (vec![NpcId(2), NpcId(3), NpcId(4)], Some(NpcId(4)))
        );
        let no_hit = [npcs[0].clone(), npcs[1].clone(), npcs[2].clone()];
        assert_eq!(
            best_threat(&escort, &among(&no_hit), &ShipState::default(), None),
            Some(NpcId(2)),
            "the lead, nearer than its escort"
        );
    }

    #[test]
    fn the_escort_takes_no_nearby_threat_it_cannot_see_itself() {
        let mut escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let npcs = [escort.clone(), cloaked(ship(2, MEDIUM, 50.0, 0.0, true))];
        let seen_by_player = Surroundings {
            cloak_scanner: true,
            ..among(&npcs)
        };
        let player = ShipState::default();
        assert_eq!(
            threat_score(&escort, &npcs[1], &seen_by_player, &player, None),
            None,
            "the escort scans for no cloaks"
        );
        assert_eq!(best_threat(&escort, &seen_by_player, &player, None), None);
        escort.cloak_gear.scanner.on_screen = true;
        assert_eq!(
            threat_score(&escort, &npcs[1], &seen_by_player, &player, None),
            Some(2500.0)
        );
        assert_eq!(
            best_threat(&escort, &seen_by_player, &player, None),
            Some(NpcId(2))
        );
    }

    #[test]
    fn a_tie_goes_to_the_lower_berth_whatever_the_order_held() {
        // `_FindNearestThreatToParent` walks slots 0-63, a strictly lower
        // score replacing the pick (@0x83861-0x838d1).
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, false);
        let mut later = ship(2, MEDIUM, 0.0, 100.0, true);
        later.berth = 7;
        let mut earlier = ship(3, MEDIUM, 100.0, 0.0, true);
        earlier.berth = 4;
        let npcs = [escort.clone(), later, earlier];
        assert_eq!(
            best_threat(&escort, &among(&npcs), &player, None),
            Some(NpcId(3))
        );
    }

    /// The NPC lead, NPC 10.
    const LEAD: NpcId = NpcId(10);

    /// The lead at (`x`, `y`), idle.
    fn lead_at(x: f32, y: f32) -> Npc {
        ship(10, MEDIUM, x, y, false)
    }

    /// NPC `id` at (`x`, `y`) led by the lead.
    fn led(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, MEDIUM, x, y, false);
        npc.leader = Some(LEAD);
        npc
    }

    /// NPC `id` at (`x`, `y`) attacking `target`.
    fn attacking(id: u32, x: f32, y: f32, target: ShipRef) -> Npc {
        let mut npc = ship(id, MEDIUM, x, y, false);
        npc.goal = crate::ai::Goal::Attack(target);
        npc
    }

    /// `npcs` with the player at (`x`, `y`), as `player` changes it.
    fn with_player(
        npcs: &[Npc],
        (x, y): (f32, f32),
        player: impl Fn(&mut crate::ai::PlayerSide),
    ) -> Surroundings<'_> {
        let mut side = crate::ai::fixture::player(x, y);
        player(&mut side);
        Surroundings {
            player: Some(side),
            ..among(npcs)
        }
    }

    #[test]
    fn the_nearest_threat_to_an_npc_lead_attacks_it_or_a_ship_it_leads() {
        // `_IsShipANearbyThreatToParent` with an NPC parent asks
        // `_ExtendedIsThreatToShip(parent, candidate)` (@0x83735-0x83768).
        let lead = lead_at(0.0, 0.0);
        let escort = led(1, 0.0, 0.0);
        let npcs = [
            escort.clone(),
            lead.clone(),
            led(2, 500.0, 0.0),
            attacking(3, 300.0, 0.0, ShipRef::Npc(LEAD)),
            attacking(4, 0.0, 200.0, ShipRef::Npc(NpcId(2))),
            ship(5, MEDIUM, 10.0, 0.0, false),
            attacking(6, 20.0, 0.0, ShipRef::Player),
        ];
        let around = among(&npcs);
        let parent = Parent::Npc(&lead);
        assert_eq!(
            threat_score_to(&escort, &npcs[3], parent, &around, None),
            Some(90_000.0)
        );
        assert_eq!(
            threat_score_to(&escort, &npcs[4], parent, &around, None),
            Some(40_000.0)
        );
        for calm in [&npcs[5], &npcs[6]] {
            assert_eq!(
                threat_score_to(&escort, calm, parent, &around, None),
                None,
                "{:?}",
                calm.id
            );
        }
        assert_eq!(
            nearest_threat(&escort, parent, &around, None),
            Some(ShipRef::Npc(NpcId(4)))
        );
        let far = lead_at(5000.0, 0.0);
        let mut moved = npcs.clone();
        moved[1] = far.clone();
        assert_eq!(
            nearest_threat(&escort, Parent::Npc(&far), &among(&moved), None),
            Some(ShipRef::Npc(NpcId(4))),
            "anywhere, measured from the escort"
        );
    }

    #[test]
    fn with_a_radius_the_npc_leads_distance_is_added_and_one_beyond_it_is_out() {
        let lead = lead_at(100.0, 0.0);
        let escort = led(1, 0.0, 0.0);
        let inside = attacking(2, 650.0, 0.0, ShipRef::Npc(LEAD));
        let outside = attacking(3, 651.0, 0.0, ShipRef::Npc(LEAD));
        let npcs = [
            escort.clone(),
            lead.clone(),
            outside.clone(),
            inside.clone(),
        ];
        let around = among(&npcs);
        let parent = Parent::Npc(&lead);
        assert_eq!(
            threat_score_to(&escort, &inside, parent, &around, Some(DEFEND_RADIUS)),
            Some(650.0 * 650.0 + 550.0 * 550.0)
        );
        assert_eq!(
            threat_score_to(&escort, &outside, parent, &around, Some(DEFEND_RADIUS)),
            None
        );
        assert_eq!(
            nearest_threat(&escort, parent, &around, Some(DEFEND_RADIUS)),
            Some(ShipRef::Npc(NpcId(2)))
        );
        let npcs = [escort.clone(), lead.clone(), outside];
        assert_eq!(
            nearest_threat(&escort, parent, &among(&npcs), Some(DEFEND_RADIUS)),
            None
        );
    }

    #[test]
    fn neither_the_lead_the_escort_nor_a_ship_the_lead_leads_is_a_threat_to_it() {
        // The sibling and leader tests (@0x836f1) coincide with
        // `_ExtendedIsThreatToShip`'s own leader test (@0x81a5a).
        // The lead attacks a ship it leads, which by `_ExtendedIsThreatToShip`
        // alone would make it a threat to itself.
        let mut lead = lead_at(0.0, 0.0);
        lead.goal = crate::ai::Goal::Attack(ShipRef::Npc(NpcId(2)));
        let mut escort = ship(1, MEDIUM, 0.0, 0.0, false);
        escort.goal = crate::ai::Goal::Attack(ShipRef::Npc(LEAD));
        let mut turncoat = led(2, 10.0, 0.0);
        turncoat.goal = crate::ai::Goal::Attack(ShipRef::Npc(LEAD));
        let npcs = [escort.clone(), lead.clone(), turncoat];
        let around = among(&npcs);
        let parent = Parent::Npc(&lead);
        for candidate in &npcs {
            assert_eq!(
                threat_score_to(&escort, candidate, parent, &around, None),
                None,
                "{:?}",
                candidate.id
            );
        }
        assert_eq!(nearest_threat(&escort, parent, &around, None), None);
        assert_eq!(fight_threats(&escort, parent, &around), []);
    }

    #[test]
    fn a_cloaked_threat_to_an_npc_lead_the_escort_cannot_see_is_not_taken() {
        let lead = lead_at(0.0, 0.0);
        let mut escort = led(1, 0.0, 0.0);
        let npcs = [
            escort.clone(),
            lead.clone(),
            cloaked(attacking(2, 50.0, 0.0, ShipRef::Npc(LEAD))),
        ];
        let around = among(&npcs);
        assert_eq!(
            nearest_threat(&escort, Parent::Npc(&lead), &around, None),
            None
        );
        escort.cloak_gear.scanner.on_screen = true;
        assert_eq!(
            nearest_threat(&escort, Parent::Npc(&lead), &around, None),
            Some(ShipRef::Npc(NpcId(2)))
        );
    }

    #[test]
    fn an_npc_escort_takes_no_cloaked_player_it_cannot_see_for_a_threat_to_its_lead() {
        // `_ShipVisibleToShip(player, escort)` @0x8370e, before `_IsDisabled`.
        let mut lead = lead_at(0.0, 0.0);
        lead.goal = crate::ai::Goal::Attack(ShipRef::Player);
        let escort = led(1, 0.0, 0.0);
        let npcs = [escort.clone(), lead.clone()];
        let cloaked = with_player(&npcs, (0.0, 150.0), |player| {
            *player = crate::ai::fixture::cloaked_player(*player);
        });
        assert_eq!(player_threat_score(&escort, &lead, &cloaked, None), None);
        let scanning = crate::ai::fixture::scanning(escort);
        assert!(
            player_threat_score(&scanning, &lead, &cloaked, None).is_some(),
            "scanned within 200"
        );
    }

    #[test]
    fn the_player_is_a_nearby_threat_to_an_npc_lead_that_threatens_it() {
        // Slot 0 as a candidate: not disabled, not blessed by the lead's
        // government, and the lead `_IsThreatToPlayer` (@0x81a89-0x81a9c).
        let mut lead = lead_at(0.0, 0.0);
        lead.goal = crate::ai::Goal::Attack(ShipRef::Player);
        let mut class = led(1, 0.0, 0.0);
        class.class = EscortClass::Freighter;
        let npcs = [class.clone(), lead.clone()];
        let parent = Parent::Npc(&lead);
        let around = with_player(&npcs, (0.0, 300.0), |_| {});
        assert_eq!(
            nearest_threat(&class, parent, &around, None),
            Some(ShipRef::Player)
        );
        assert_eq!(
            player_threat_score(&class, &lead, &around, Some(DEFEND_RADIUS)),
            Some(2.0 * 90_000.0),
            "the player's class is the escort's: no halving"
        );
        let medium = led(1, 0.0, 0.0);
        assert_eq!(
            player_threat_score(&medium, &lead, &around, None),
            Some(45_000.0),
            "another class: half"
        );
        let blessed = with_player(&npcs, (0.0, 300.0), |player| {
            player.blessing = crate::ai::fixture::blessing_of(crate::ai::fixture::PIRATES);
        });
        let mut pirate_lead = lead.clone();
        pirate_lead.govt = Some(crate::ai::fixture::PIRATES);
        let pirate_npcs = [class.clone(), pirate_lead.clone()];
        let blessed = Surroundings {
            npcs: &pirate_npcs,
            ..blessed
        };
        assert_eq!(
            nearest_threat(&class, Parent::Npc(&pirate_lead), &blessed, None),
            None,
            "blessed by the lead's government"
        );
        let disabled = with_player(&npcs, (0.0, 300.0), |player| {
            player.condition = crate::combat::hull::Condition::Disabled;
        });
        assert_eq!(nearest_threat(&class, parent, &disabled, None), None);
        let idle = lead_at(0.0, 0.0);
        let npcs = [class.clone(), idle.clone()];
        let around = with_player(&npcs, (0.0, 300.0), |_| {});
        assert_eq!(
            nearest_threat(&class, Parent::Npc(&idle), &around, None),
            None,
            "a lead that doesn't threaten the player"
        );
    }

    #[test]
    fn a_tie_with_the_player_goes_to_the_player_then_to_the_lower_berth() {
        let mut lead = lead_at(0.0, 0.0);
        lead.goal = crate::ai::Goal::Attack(ShipRef::Player);
        let escort = led(1, 0.0, 0.0);
        let mut later = attacking(2, 100.0, 0.0, ShipRef::Npc(LEAD));
        later.berth = 9;
        let mut earlier = attacking(3, -100.0, 0.0, ShipRef::Npc(LEAD));
        earlier.berth = 5;
        let npcs = [escort.clone(), lead.clone(), later, earlier];
        let around = with_player(&npcs, (0.0, 100.0), |player| {
            player.class = MEDIUM;
        });
        let parent = Parent::Npc(&lead);
        assert_eq!(
            nearest_threat(&escort, parent, &around, None),
            Some(ShipRef::Player)
        );
        let around = among(&npcs);
        assert_eq!(
            nearest_threat(&escort, parent, &around, None),
            Some(ShipRef::Npc(NpcId(3)))
        );
    }

    #[test]
    fn an_npc_leads_fight_threats_include_the_player_whenever_it_threatens_the_player() {
        // `_AIFightThreatToParent`'s slot 0 asks
        // `_ExtendedIsThreatToPlayer(parent)` (@0x81e83-0x81e9d): the
        // player's sight of the lead, no blessing.
        let mut lead = lead_at(0.0, 0.0);
        lead.goal = crate::ai::Goal::Attack(ShipRef::Player);
        lead.govt = Some(crate::ai::fixture::PIRATES);
        let escort = led(1, 0.0, 0.0);
        let mut down = attacking(4, 30.0, 0.0, ShipRef::Npc(LEAD));
        down.condition = crate::combat::hull::Condition::Disabled;
        let npcs = [
            escort.clone(),
            lead.clone(),
            attacking(2, 5000.0, 0.0, ShipRef::Npc(LEAD)),
            ship(3, MEDIUM, 10.0, 0.0, false),
            down,
        ];
        let parent = Parent::Npc(&lead);
        let around = with_player(&npcs, (0.0, 300.0), |_| {});
        assert_eq!(
            fight_threats(&escort, parent, &around),
            [ShipRef::Player, ShipRef::Npc(NpcId(2))]
        );
        let blessed = with_player(&npcs, (0.0, 300.0), |player| {
            player.blessing = crate::ai::fixture::blessing_of(crate::ai::fixture::PIRATES);
        });
        assert_eq!(
            fight_threats(&escort, parent, &blessed),
            [ShipRef::Player, ShipRef::Npc(NpcId(2))],
            "blessed or not"
        );
        let disabled = with_player(&npcs, (0.0, 300.0), |player| {
            player.condition = crate::combat::hull::Condition::Disabled;
        });
        assert_eq!(
            fight_threats(&escort, parent, &disabled),
            [ShipRef::Npc(NpcId(2))]
        );
        assert_eq!(
            fight_threats(&escort, parent, &among(&npcs)),
            [ShipRef::Npc(NpcId(2))],
            "no player in the system"
        );
        let unseen = [escort.clone(), cloaked(lead.clone())];
        let around = with_player(&unseen, (0.0, 300.0), |_| {});
        assert_eq!(
            fight_threats(&escort, Parent::Npc(&unseen[1]), &around),
            [],
            "the player doesn't see the lead"
        );
        assert_eq!(
            fight_threats(
                &escort,
                Parent::Player(&ShipState::default()),
                &among(&npcs)
            ),
            [ShipRef::Npc(LEAD)],
            "with the player the parent, the threats to the player"
        );
    }

    #[test]
    fn the_groups_hold_their_classes() {
        for class in EscortClass::ALL {
            assert!(EscortGroup::All.holds(class));
            for other in EscortClass::ALL {
                assert_eq!(EscortGroup::Class(class).holds(other), class == other);
            }
        }
        assert_eq!(
            EscortGroup::ALL[1..],
            EscortClass::ALL.map(EscortGroup::Class)
        );
    }
}
