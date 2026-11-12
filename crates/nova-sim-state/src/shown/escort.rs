//! The escort menu's and the escort commands' view: the commands the
//! player gives ([`EscortCommand`]), the groups they go to
//! ([`EscortGroup`]), what a command changed ([`Commanded`]), a class's
//! menu row ([`ClassRow`]) and the `STR#` 2002 strings they say, from
//! `nova_sim_ai::escort`, which re-exports them and keeps the formation
//! and the threat searches.

use nova_sim_catalog::catalog::EscortClass;

use super::fleet::EscortOrder;

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
    /// Whether a ship of it the player leads is here, disabled or dying too.
    pub present: bool,
    /// The order kept for its class, as `_DrawEscortMenuInFrame` reads it
    /// (@0x56534); none for formation, or with no escort of it.
    pub order: Option<EscortOrder>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
