//! The comm dialog's view: why a hail went unanswered ([`HailRefusal`]),
//! what the dialog shows ([`HailView`], [`HailButton`], [`EscortStatus`]),
//! a hired escort's pay as it is shown ([`ShownWage`], [`CreditUnit`]) and
//! an assisting ship's note ([`CommNote`]), from `nova_sim_ai::hail`,
//! which re-exports them and keeps the hail, its options and its
//! dispositions.
//!
//! - [`deal`]: a choice in the haggle dialog, from
//!   `nova_sim_ai::hail::deal`.

pub mod deal;

use nova_sim_catalog::catalog::ShipId;

use super::ai::Help;
use super::combat::NpcId;

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
    /// `nova_sim_ai::hail::Dispositions`), or an escape pod, the player's escort or not
    /// ([`ESCAPE_POD_SHIP`](super::combat::hull::ESCAPE_POD_SHIP),
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
