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

#[cfg(test)]
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
