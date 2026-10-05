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
//! formation.
//!
//! The strings (`STR#` 2002) are the constants below; [`STRINGS`] lists
//! each with its number.

use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::traffic::npc::{Npc, NpcId};

/// `STR#` 2002 #51: E with no escorts.
pub const NO_ESCORTS: &str = "You don't have any escorts.";
/// `STR#` 2002 #133: the escort menu's title.
pub const ESCORT_COMMANDS: &str = "Escort Commands";
/// `STR#` 2002 #134: what a command's message starts with (two spaces
/// at its end).
pub const NEW_ORDERS: &str = "New escort orders assigned:  ";
/// `STR#` 2002 #154: Attack's message with no target to copy.
pub const WILL_ATTACK: &str = "will attack.";

/// Every escort string, with its number in `STR#` 2002.
pub const STRINGS: [(u16, &str); 24] = [
    (51, NO_ESCORTS),
    (133, ESCORT_COMMANDS),
    (134, NEW_ORDERS),
    (135, "Fighters"),
    (136, "Medium ships"),
    (137, "Warships"),
    (138, "Freighters"),
    (139, "All ships"),
    (140, "Fighters"),
    (141, "Medium Ships"),
    (142, "Warships"),
    (143, "Freighters"),
    (144, "All Ships"),
    (145, "Defend"),
    (146, "Attack"),
    (147, "Hold Position"),
    (148, "Return to Hangar"),
    (149, "Formation"),
    (154, WILL_ATTACK),
    (155, "returning to hangar."),
    (156, "returning to formation."),
    (157, "holding position."),
    (158, "defending."),
    (159, "attacking target."),
];

/// String `n` of [`STRINGS`].
const fn string(n: u16) -> &'static str {
    let mut at = 0;
    while at < STRINGS.len() {
        if STRINGS[at].0 == n {
            return STRINGS[at].1;
        }
        at += 1;
    }
    panic!("no such escort string");
}

/// An escort's class, which the escort menu groups it by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscortClass {
    /// 0: a fighter.
    Fighter,
    /// 1: a medium ship.
    Medium,
    /// 2: a warship.
    Warship,
    /// 3: a freighter; the default, the class a ship of no record works
    /// out to (`InherentAI` none).
    #[default]
    Freighter,
}

/// The heaviest `Mass` of a fighter whose class is worked out.
pub const FIGHTER_MASS: i16 = 49;
/// The heaviest `Mass` of a medium ship whose class is worked out.
pub const MEDIUM_MASS: i16 = 199;
/// The highest `InherentAI` of a freighter whose class is worked out.
pub const FREIGHTER_AI: i16 = 2;

impl EscortClass {
    /// Every class, in the escort menu's order.
    pub const ALL: [Self; 4] = [Self::Fighter, Self::Medium, Self::Warship, Self::Freighter];

    /// The class of a `shïp` of `EscortType` `escort_type`, `InherentAI`
    /// `inherent_ai` and `Mass` `mass` (see the module docs).
    #[must_use]
    pub fn of(escort_type: i16, inherent_ai: i16, mass: i16) -> Self {
        match escort_type {
            0 => Self::Fighter,
            1 => Self::Medium,
            2 => Self::Warship,
            3 => Self::Freighter,
            _ if inherent_ai <= FREIGHTER_AI => Self::Freighter,
            _ if mass <= FIGHTER_MASS => Self::Fighter,
            _ if mass <= MEDIUM_MASS => Self::Medium,
            _ => Self::Warship,
        }
    }

    /// Its row in the escort menu (`STR#` 2002 #140-#143).
    #[must_use]
    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::Fighter => string(140),
            Self::Medium => string(141),
            Self::Warship => string(142),
            Self::Freighter => string(143),
        }
    }

    /// What a command's message calls it (`STR#` 2002 #135-#138).
    #[must_use]
    pub const fn message_form(self) -> &'static str {
        match self {
            Self::Fighter => string(135),
            Self::Medium => string(136),
            Self::Warship => string(137),
            Self::Freighter => string(138),
        }
    }
}

/// A standing order an escort follows. An escort with none keeps
/// formation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscortOrder {
    /// 1: it defends the player.
    Defend,
    /// 2: it attacks the player's target.
    Attack,
    /// 4: it holds position.
    Hold,
}

/// The escort menu's label for `order` (`STR#` 2002 #145-#149): none
/// is "Formation".
#[must_use]
pub const fn order_label(order: Option<EscortOrder>) -> &'static str {
    match order {
        None => string(149),
        Some(EscortOrder::Defend) => string(145),
        Some(EscortOrder::Attack) => string(146),
        Some(EscortOrder::Hold) => string(147),
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
}

impl EscortCommand {
    /// The standing order it gives: none for a recall.
    #[must_use]
    pub const fn order(self) -> Option<EscortOrder> {
        match self {
            Self::Attack => Some(EscortOrder::Attack),
            Self::Defend => Some(EscortOrder::Defend),
            Self::Hold => Some(EscortOrder::Hold),
            Self::Recall => None,
        }
    }

    /// What its message says the escorts do (`STR#` 2002 #156-#159).
    #[must_use]
    pub const fn doing(self) -> &'static str {
        match self {
            Self::Attack => string(159),
            Self::Defend => string(158),
            Self::Hold => string(157),
            Self::Recall => string(156),
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
            Self::All => string(144),
            Self::Class(class) => class.menu_label(),
        }
    }

    /// What a command's message calls it (`STR#` 2002 #139, #135-#138).
    #[must_use]
    pub const fn message_form(self) -> &'static str {
        match self {
            Self::All => string(139),
            Self::Class(class) => class.message_form(),
        }
    }
}

/// What a command to the escorts changed, for its message: the group
/// it went to, the command, and whether Attack copied the player's
/// target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commanded {
    /// The escorts it went to.
    pub group: EscortGroup,
    /// The command.
    pub command: EscortCommand,
    /// Whether the escorts took the player's target to attack.
    pub targeted: bool,
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
    /// Its slot in the formation ([`slot_offset`]): 2, 3, … by its place
    /// in the fleet, the player being slot 1.
    pub slot: u8,
    /// How many ships the formation holds, the player included.
    pub ships: u8,
    /// The formation's spacing, in pixels ([`spacing`]).
    pub spacing: f32,
    /// Its standing order; none keeps formation.
    pub order: Option<EscortOrder>,
}

/// How near its slot, in pixels on each axis, an escort keeps formation
/// (300.0 @0xdd95c).
pub const KEEP_FORMATION: f32 = 300.0;
/// How near its slot, in pixels on each axis, an escort approaches it;
/// farther off it flies at full speed (600.0 @0xdda54).
pub const APPROACH_FORMATION: f32 = 600.0;
/// How far off its slot, in pixels on an axis, an escort keeping
/// formation may drift before it is nudged back (8.0 @0xdd044).
pub const FORMATION_SLACK: f32 = 8.0;
/// How many ticks' acceleration the nudge back to the slot is at most
/// (10.0 @0xdd65c).
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
/// heading turned 90 degrees clockwise.
#[must_use]
pub fn slot_position(leader: &ShipState, slot: u8, ships: u8, spacing: f32) -> Vec2 {
    let (behind, right) = slot_offset(slot, ships);
    leader.position
        + facing(leader.heading) * (-behind * spacing)
        + facing(leader.heading + 90.0) * (right * spacing)
}

/// The radius around the player, in pixels, a defending escort takes a
/// new threat within (550 @0x83c56).
pub const DEFEND_RADIUS: f32 = 550.0;
/// The distance squared from the player within which a defending escort
/// keeps its target (408375.0 @0xdda28, about 639 pixels).
pub const DEFEND_KEEP_SQ: f32 = 408_375.0;

/// The ships among `npcs` that threaten the player, which `escort` may
/// fight (`_AIFightThreatToParent` @0x81de6): any that threatens the
/// player ([`Npc::threatens_player`]), but never `escort` itself or a
/// fellow escort.
pub fn threats<'a>(escort: &'a Npc, npcs: &'a [Npc]) -> impl Iterator<Item = &'a Npc> + 'a {
    npcs.iter().filter(move |candidate| {
        candidate.id != escort.id && candidate.escort.is_none() && candidate.threatens_player()
    })
}

/// How strongly `escort` takes `candidate` for a threat to the player at
/// `player` (`_IsShipANearbyThreatToParent` @0x836a4); the lowest wins,
/// and none is no threat (see [`threats`]). The score is the distance
/// squared from the escort; with a `radius` around the player, the
/// candidate must be within it, and its distance squared from the player
/// is added. It is halved for a candidate of another class than the
/// escort's, and halved again for a fighter against a warship escort.
/// The original's visibility test (cloaks) is not modelled.
#[must_use]
pub fn threat_score(
    escort: &Npc,
    candidate: &Npc,
    player: &ShipState,
    radius: Option<f32>,
) -> Option<f32> {
    if candidate.id == escort.id || candidate.escort.is_some() || !candidate.threatens_player() {
        return None;
    }
    let squared = |off: Vec2| off.x * off.x + off.y * off.y;
    let mut score = squared(candidate.state.position - escort.state.position);
    if let Some(radius) = radius {
        let from_player = squared(candidate.state.position - player.position);
        if from_player > radius * radius {
            return None;
        }
        score += from_player;
    }
    if candidate.class != escort.class {
        score /= 2.0;
        if candidate.class == EscortClass::Fighter && escort.class == EscortClass::Warship {
            score /= 2.0;
        }
    }
    Some(score)
}

/// The threat to the player at `player` among `npcs` that `escort` takes
/// ([`threat_score`]): the lowest score, the first of a tie.
#[must_use]
pub fn best_threat(
    escort: &Npc,
    npcs: &[Npc],
    player: &ShipState,
    radius: Option<f32>,
) -> Option<NpcId> {
    let mut best: Option<(NpcId, f32)> = None;
    for candidate in npcs {
        let Some(score) = threat_score(escort, candidate, player, radius) else {
            continue;
        };
        if best.is_none_or(|(_, lowest)| score < lowest) {
            best = Some((candidate.id, score));
        }
    }
    best.map(|(id, _)| id)
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
                Some(EscortOrder::Hold)
            ]
            .map(order_label),
            ["Formation", "Defend", "Attack", "Hold Position"]
        );
        assert_eq!(
            [
                EscortCommand::Attack,
                EscortCommand::Defend,
                EscortCommand::Hold,
                EscortCommand::Recall
            ]
            .map(EscortCommand::doing),
            [
                "attacking target.",
                "defending.",
                "holding position.",
                "returning to formation."
            ]
        );
        assert_eq!(NEW_ORDERS, "New escort orders assigned:  ");
        assert_eq!(WILL_ATTACK, "will attack.");
        assert_eq!(ESCORT_COMMANDS, "Escort Commands");
        assert_eq!(NO_ESCORTS, "You don't have any escorts.");
    }

    #[test]
    fn the_strings_are_numbered_in_order() {
        assert!(STRINGS.windows(2).all(|pair| pair[0].0 < pair[1].0));
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

    /// NPC `id` of `class` at (`x`, `y`), attacking the player when
    /// `threat`.
    fn ship(id: u32, class: EscortClass, x: f32, y: f32, threat: bool) -> Npc {
        let mut npc = crate::testkit::npc(id, crate::stats::ShipStats::default());
        npc.class = class;
        npc.state.position = Vec2::new(x, y);
        if threat {
            npc.goal = crate::ai::Goal::Attack(crate::combat::ShipRef::Player);
        }
        npc
    }

    const MEDIUM: EscortClass = EscortClass::Medium;

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
            threat_score(&escort, &npcs[0], &player, None),
            Some(90_000.0)
        );
        assert_eq!(
            threat_score(&escort, &npcs[1], &player, None),
            Some(40_000.0)
        );
        assert_eq!(best_threat(&escort, &npcs, &player, None), Some(NpcId(3)));
        let far = ShipState {
            position: Vec2::new(5000.0, 0.0),
            ..ShipState::default()
        };
        assert_eq!(
            best_threat(&escort, &npcs, &far, None),
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
            threat_score(&escort, &inside, &player, Some(DEFEND_RADIUS)),
            Some(650.0 * 650.0 + 550.0 * 550.0)
        );
        let outside = ship(3, MEDIUM, 651.0, 0.0, true);
        assert_eq!(
            threat_score(&escort, &outside, &player, Some(DEFEND_RADIUS)),
            None
        );
        assert_eq!(
            best_threat(
                &escort,
                &[outside.clone(), inside],
                &player,
                Some(DEFEND_RADIUS)
            ),
            Some(NpcId(2))
        );
        assert_eq!(
            best_threat(&escort, &[outside], &player, Some(DEFEND_RADIUS)),
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
        assert_eq!(threat_score(&medium, &other, &player, None), Some(9800.0));
        assert_eq!(
            best_threat(&medium, &[same.clone(), other], &player, None),
            Some(NpcId(3))
        );
        let other = ship(3, EscortClass::Freighter, 142.0, 0.0, true);
        assert_eq!(
            best_threat(&medium, &[same, other], &player, None),
            Some(NpcId(2))
        );
        let warship = ship(1, EscortClass::Warship, 0.0, 0.0, false);
        let fighter = ship(4, EscortClass::Fighter, 199.0, 0.0, true);
        let medium_threat = ship(5, MEDIUM, 141.0, 0.0, true);
        let warship_threat = ship(6, EscortClass::Warship, 100.0, 0.0, true);
        assert_eq!(
            threat_score(&warship, &fighter, &player, None),
            Some(199.0 * 199.0 / 4.0)
        );
        assert_eq!(
            best_threat(&warship, &[warship_threat.clone(), fighter], &player, None),
            Some(NpcId(4)),
            "up to four times"
        );
        let fighter = ship(4, EscortClass::Fighter, 201.0, 0.0, true);
        assert_eq!(
            best_threat(&warship, &[warship_threat.clone(), fighter], &player, None),
            Some(NpcId(6))
        );
        assert_eq!(
            best_threat(&warship, &[warship_threat, medium_threat], &player, None),
            Some(NpcId(5)),
            "a medium ship counts half"
        );
        let fighter_escort = ship(1, EscortClass::Fighter, 0.0, 0.0, false);
        let warship_threat = ship(6, EscortClass::Warship, 100.0, 0.0, true);
        assert_eq!(
            threat_score(&fighter_escort, &warship_threat, &player, None),
            Some(5000.0),
            "not the other way round"
        );
    }

    #[test]
    fn fellow_escorts_itself_and_non_threats_are_never_candidates() {
        let player = ShipState::default();
        let escort = ship(1, MEDIUM, 0.0, 0.0, true);
        let mut fellow = ship(2, MEDIUM, 10.0, 0.0, true);
        fellow.escort = Some(EscortDuty {
            slot: 3,
            ships: 3,
            spacing: 30.0,
            order: None,
        });
        let calm = ship(3, MEDIUM, 10.0, 0.0, false);
        for candidate in [&escort, &fellow, &calm] {
            assert_eq!(threat_score(&escort, candidate, &player, None), None);
        }
        let npcs = [
            escort.clone(),
            fellow,
            calm,
            ship(4, MEDIUM, 900.0, 0.0, true),
        ];
        assert_eq!(best_threat(&escort, &npcs, &player, None), Some(NpcId(4)));
        assert_eq!(
            threats(&escort, &npcs)
                .map(|npc| npc.id)
                .collect::<Vec<_>>(),
            [NpcId(4)]
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
