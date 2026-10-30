//! The original's AI type word (ship +0x88), as its readers compare it
//! ([`Npc::original_ai_type`]).
//!
//! The engine routes a led ship by its leader or carrier and keeps its
//! own [`AiType`] (1-4), where the original writes a word of its own over
//! +0x88. The readers that compare that word (`> 2`, `<= 2`, `> 4`,
//! `!= 4`, `3..=4`) read the predicate, so each writes the binary's own
//! comparison.
//!
//! **The writers.**
//!
//! - 5, a carried fighter ([`FIGHTER_AI`]): `_LaunchFighter` @0x3d771,
//!   `_LoadPilotData` @0x75b45, the player's captures @0x65b3b/@0x65e6e.
//! - 6, an escort ([`ESCORT_AI`]): `_SpawnEscort` @0x3fed1,
//!   `_HyperSpawnFleet` @0x421dc, `_SetupShipsInSystem` @0x430b6,
//!   `_AICheckParentExists` @0x89491, `_AIPlunderShipContents` @0x8495c,
//!   `_AIDispatch` @0x8fc19 (a mission escort), and the captures
//!   (`_DoShipCapture` @0x41425/@0x41480, `_HandlePlayerBoardAttempt`
//!   @0x65bb1, `_DoPlunderDialog` @0x9461f).
//! - 1, a bribe ([`BRIBED_AI`]): `_DoCommDialog` @0x961f8 writes +0x88
//!   only, keeping the parent +0x9a, and nothing but a capture writes it
//!   again until the ship leaves (the disabled reset @0x8fece-0x8ff13
//!   leaves it alone). The player's capture makes it the player's escort,
//!   read first; a pirate's take (`_AIPlunderShipContents` @0x8495c)
//!   clears [`Npc::spared`], the bribe's only state.
//! - -1, no AI ([`NO_AI`]): the player's eject hulk (`_HandlePlayer`
//!   @0x6f31f).
//!
//! **The disabled former escort.** `_AIDispatch`'s reset drops a disabled
//! escort's parent and leaves +0x88 at 6 until its first think once whole,
//! where `_EscortAI`'s lost parent (@0x83917-0x8397b) writes its class's
//! `InherentAI` (@0x83c4a); a miner (`shïp` `Flags3` & 3, @0x8ff76-0x8ff8d)
//! never reaches that write and keeps 6 for its stay. The engine's reset
//! takes it out of its fleet at once ([`Npc::leave_fleet`]) and marks it
//! ([`Npc::kept_escort_ai`], see
//! [`disabled_reset`](super::disabled_reset)), and
//! [`Npc::settle_original_ai_type`] clears the mark at that first think.
//!
//! **The readers.** The hail's bribe and plunder class
//! (`_DoCommDialog` @0x95790, @0x9585b) and escort shortcut (@0x962ee,
//! @0x96564), the government hail's string (`_LoadAdvice` @0x91a0d), a
//! brave trader's distress call (`_BraveTraderAI` @0x8b707), a trader
//! leader's orders (`_AIIssueEscortOrders` @0x89888), the escorts' victory
//! voice (`_HandleShipHit` @0x36cd8), the interceptor's inspection pick
//! (`_InterceptorAI` @0x8cd4e, @0x8cdfe), the Good Samaritan
//! (`_DoGoodSamaritan` @0x8289f), and the plunder candidate and lost-sight
//! search (`_AIFindShipToPlunder` @0x7fdad, `_HighLevelAIHandler`
//! @0x8d748).
//!
//! Nothing here plays a sound or draws a random number.

use super::npc::{AiType, Npc};
use crate::ai::miner;

/// The original's AI type word of a ship with no AI (+0x88 -1).
pub const NO_AI: i16 = -1;
/// The original's AI type word of a bribed ship (`_DoCommDialog` @0x961f8).
pub const BRIBED_AI: i16 = 1;
/// The original's AI type word of a carried fighter (`_LaunchFighter`
/// @0x3d771).
pub const FIGHTER_AI: i16 = 5;
/// The original's AI type word of an escort (`_SpawnEscort` @0x3fed1,
/// `_HyperSpawnFleet` @0x421dc, and the rest in the module docs).
pub const ESCORT_AI: i16 = 6;

impl AiType {
    /// Its word, 1-4.
    #[must_use]
    pub fn raw(self) -> i16 {
        match self {
            Self::WimpyTrader => 1,
            Self::BraveTrader => 2,
            Self::Warship => 3,
            Self::Interceptor => 4,
        }
    }
}

impl Npc {
    /// The original's AI type (+0x88), as its readers compare it (see the
    /// module docs): [`NO_AI`] with no AI; for the player's escort, its
    /// launched fighter [`FIGHTER_AI`] and any other [`ESCORT_AI`] (a
    /// capture writes 6 over an earlier bribe); a bribed ship
    /// [`BRIBED_AI`], ahead of a leader or carrier it keeps (a pirate's
    /// take clears [`Npc::spared`] as it writes 6, @0x8495c); a carried
    /// fighter [`FIGHTER_AI`]; a ship
    /// with an NPC leader, or a former escort that keeps its word,
    /// [`ESCORT_AI`]; otherwise its own type's word.
    #[must_use]
    pub fn original_ai_type(&self) -> i16 {
        if self.no_ai {
            NO_AI
        } else if self.escorts_player() {
            if self.carrier.is_some() {
                FIGHTER_AI
            } else {
                ESCORT_AI
            }
        } else if self.spared {
            BRIBED_AI
        } else if self.carrier.is_some() {
            FIGHTER_AI
        } else if self.leader.is_some() || self.kept_escort_ai {
            ESCORT_AI
        } else {
            self.ai_type.raw()
        }
    }

    /// Its think's settling of [`Npc::kept_escort_ai`]: the mark cleared
    /// once it thinks whole, where `_EscortAI`'s lost parent writes its
    /// class's `InherentAI` (@0x83c4a); kept while it is disabled, and for
    /// its stay if it thinks as a miner ([`miner::thinks_for`],
    /// @0x8ff76-0x8ff8d), which never reaches that write.
    ///
    /// **Ordering.** The original clears it inside the ship's own
    /// dispatch, in slot order; the traffic's tick settles it after the
    /// tick's decisions. So a higher slot's reader in that same tick reads
    /// 6 a tick late: only a brave trader whose target's first think once
    /// whole falls in that tick, whose distress call is drawn on the
    /// message line. Filed in `play-feel-5-followups`.
    pub fn settle_original_ai_type(&mut self) {
        if !self.condition.disabled() && !miner::thinks_for(self) {
            self.kept_escort_ai = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::ShipBehav;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{MissionMark, NpcId};

    fn ship(ai_type: AiType) -> Npc {
        Npc {
            ai_type,
            ..npc(1, ShipStats::default())
        }
    }

    fn slot() -> FormationSlot {
        FormationSlot {
            slot: 2,
            ships: 3,
            spacing: 40.0,
        }
    }

    fn duty() -> EscortDuty {
        EscortDuty {
            formation: slot(),
            order: None,
        }
    }

    fn carrier() -> Carrier {
        Carrier {
            ship: ShipRef::Npc(NpcId(0)),
            window: 100.0,
            reach: 40.0,
        }
    }

    #[test]
    fn a_ship_with_no_ai_reads_minus_one() {
        let hulk = Npc {
            no_ai: true,
            leader: Some(NpcId(0)),
            ..ship(AiType::Warship)
        };
        assert_eq!(hulk.original_ai_type(), -1);
    }

    #[test]
    fn an_unled_ship_reads_its_own_type() {
        for (ai_type, raw) in [
            (AiType::WimpyTrader, 1),
            (AiType::BraveTrader, 2),
            (AiType::Warship, 3),
            (AiType::Interceptor, 4),
        ] {
            assert_eq!(ai_type.raw(), raw);
            assert_eq!(ship(ai_type).original_ai_type(), raw, "{ai_type:?}");
        }
    }

    #[test]
    fn a_led_ship_reads_six_and_a_carried_fighter_five() {
        let led = Npc {
            leader: Some(NpcId(0)),
            ..ship(AiType::WimpyTrader)
        };
        assert_eq!(led.original_ai_type(), 6);
        let fighter = Npc {
            carrier: Some(carrier()),
            leader: Some(NpcId(0)),
            ..ship(AiType::BraveTrader)
        };
        assert_eq!(fighter.original_ai_type(), 5);
        let leaderless = Npc {
            carrier: Some(carrier()),
            ..ship(AiType::BraveTrader)
        };
        assert_eq!(leaderless.original_ai_type(), 5, "its leader dropped");
    }

    #[test]
    fn the_players_escort_reads_six_and_its_launched_fighter_five() {
        let hired = Npc {
            escort: Some(duty()),
            ..ship(AiType::WimpyTrader)
        };
        assert_eq!(hired.original_ai_type(), 6);
        let fighter = Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Player,
                ..carrier()
            }),
            ..hired
        };
        assert_eq!(fighter.original_ai_type(), 5);
        let mission = Npc {
            mission: Some(MissionMark {
                slot: 1,
                behav: ShipBehav::EscortPlayer,
                formation: Some(slot()),
                order: None,
            }),
            ..ship(AiType::WimpyTrader)
        };
        assert_eq!(mission.original_ai_type(), 6, "a mission escort");
    }

    #[test]
    fn a_bribed_led_ship_reads_one_until_captured() {
        let spared = Npc {
            spared: true,
            leader: Some(NpcId(0)),
            ..ship(AiType::Warship)
        };
        assert_eq!(spared.original_ai_type(), 1);
        let fighter = Npc {
            carrier: Some(carrier()),
            kept_escort_ai: true,
            ..spared.clone()
        };
        assert_eq!(fighter.original_ai_type(), 1);
        let captured = Npc {
            escort: Some(duty()),
            ..spared
        };
        assert_eq!(captured.original_ai_type(), 6, "the capture writes 6");
    }

    #[test]
    fn a_marked_former_escort_reads_six() {
        let marked = Npc {
            kept_escort_ai: true,
            ..ship(AiType::WimpyTrader)
        };
        assert_eq!(marked.original_ai_type(), 6);
    }

    #[test]
    fn the_mark_clears_at_a_whole_ships_think_only() {
        let marked = Npc {
            kept_escort_ai: true,
            ..ship(AiType::WimpyTrader)
        };
        let mut whole = marked.clone();
        whole.settle_original_ai_type();
        assert!(!whole.kept_escort_ai);
        let mut disabled = Npc {
            condition: Condition::Disabled,
            ..marked.clone()
        };
        disabled.settle_original_ai_type();
        assert!(disabled.kept_escort_ai);
        let mut miner = Npc {
            miner: miner::MINER_BITS,
            ..marked
        };
        miner.settle_original_ai_type();
        assert!(miner.kept_escort_ai, "a miner keeps it for its stay");
    }
}
