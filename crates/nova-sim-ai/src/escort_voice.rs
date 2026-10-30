//! Who among the player's escorts speaks, and when: the three callers of
//! `_SpeakVoiceMessage` (see `nova_sim::voice` for the clips and the
//! channel).
//!
//! - **A command** (`_IssueNewEscortCommand` @0x660d5-0x66690): the
//!   speaker is a draw among the command's group in slot order from the
//!   first ship it changed on ([`command_speaker`]). A returning fighter
//!   ([`returning_fighter`]) lets go of its target under any command but
//!   Return to Hangar, and counts as changed.
//! - **A new target** (`_EscortAI` @0x83cdc-0x83e34): a Defend or Attack
//!   think that takes a new threat ([`VoiceCue::NewThreat`]) calls it 1 in
//!   3, for a fighter- or medium-class ship that isn't mute
//!   ([`calls_target`]), while the voice is idle. Every other think of a
//!   ship the player leads is a [`VoiceCue::Thought`], at which a ship
//!   with no variant rolls one (@0x83980-0x839f6).
//! - **A victory** (`_HandleShipHit` @0x36c7f-0x36d45): as a shot's direct
//!   hit starts a ship dying, the highest berth among the ships attacking
//!   it speaks ([`victory_speaker`]).
//!
//! A ship of `Flags2` 0x0010 ([`HullSpec::mute`]) says none of the three,
//! but a returning fighter takes the command's voice with none chosen
//! yet, mute or not.
//!
//! [`HullSpec::mute`]: crate::combat::hull::HullSpec::mute

use crate::ai::Goal;
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::escort::EscortClass;
use crate::traffic::npc::Npc;
use crate::voice::VoiceVariant;

/// What a think of a ship the player leads tells the voice
/// ([`Decision::voice`](crate::ai::Decision::voice)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceCue {
    /// An `_EscortAI` think past its lost parent: a ship with no variant
    /// rolls one (@0x83980-0x839f6).
    Thought,
    /// A Defend or Attack think that took a new threat
    /// (`_FindNearestThreatToParent` found one, @0x83cdc-0x83d4e,
    /// @0x83d73-0x83df7): the 1-in-3 target call's roll
    /// (@0x83df9-0x83e34), and the variant's as for a
    /// [`Thought`](Self::Thought).
    NewThreat,
}

/// Who would speak: a ship's class, whose attributes government picks the
/// voice set, and its variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speaker {
    /// Its ship class.
    pub ship: ShipId,
    /// Its variant; none while unset.
    pub voice: Option<VoiceVariant>,
}

/// One ship of a command's group in `_IssueNewEscortCommand`'s slot loop
/// (@0x66125-0x663a8).
// Each flag is one of the slot loop's tests, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Voter {
    /// Its slot, which orders the loop.
    pub berth: u32,
    /// Whether the command changed it: its order (@0x6617c-0x6622c), its
    /// target (@0x66233-0x66273), or its return as a fighter
    /// (@0x6627a-0x662dc).
    pub changed: bool,
    /// Whether its order changed and it isn't mute, which lets the
    /// command speak at all (@0x661e3, @0x66212).
    pub speaks: bool,
    /// Whether it is a fighter flying home ([`returning_fighter`]) under a
    /// command other than Return to Hangar.
    pub returning: bool,
    /// Whether its class speaks no voice ([`HullSpec::mute`]).
    ///
    /// [`HullSpec::mute`]: crate::combat::hull::HullSpec::mute
    pub mute: bool,
    /// Who it is, as a speaker.
    pub speaker: Speaker,
}

/// Who acknowledges a command among `voters`, in berth order, drawing on
/// `chance` (`_IssueNewEscortCommand` @0x6627a-0x663a8). The changes
/// accumulate through the loop. A returning fighter takes the voice
/// outright with none chosen yet, mute or not (@0x662de-0x66321), else on
/// a first draw of 0, else goes to the normal test (@0x662c4-0x662dc). In
/// the normal test (@0x6632a-0x663a8), once some voter has changed, every
/// voter that isn't mute takes the voice with none chosen, else on a draw
/// of 0. None when no voter takes it. Whether the command speaks at all is
/// the caller's ([`Voter::speaks`]).
pub fn command_speaker(voters: &[Voter], chance: &mut (impl Chance + ?Sized)) -> Option<Speaker> {
    let mut changed = false;
    let mut chosen: Option<Speaker> = None;
    for voter in voters {
        changed |= voter.changed;
        if voter.returning && (chosen.is_none() || chance.below(2) == 0) {
            chosen = Some(voter.speaker);
            continue;
        }
        if changed && !voter.mute && (chosen.is_none() || chance.below(2) == 0) {
            chosen = Some(voter.speaker);
        }
    }
    chosen
}

/// Whether `npc` is a fighter flying home to its carrier,
/// `_AIIsFighterReturning` (@0x82e54): carried (AI type 5) and docking
/// (goal 5).
#[must_use]
pub fn returning_fighter(npc: &Npc) -> bool {
    npc.carrier.is_some() && matches!(npc.goal, Goal::Dock(_))
}

/// Whether `npc` calls a new target (`_EscortAI` @0x83df9-0x83e34): its
/// class's `EscortType` 1 or less, a fighter or a medium ship, and not
/// mute.
#[must_use]
pub fn calls_target(npc: &Npc) -> bool {
    matches!(npc.class, EscortClass::Fighter | EscortClass::Medium) && !npc.hull.mute
}

/// Who among `npcs`, the ships the player leads, says the victory over
/// `victim`, a shot's direct hit having started it dying (`_HandleShipHit`
/// @0x36c7f-0x36d45): the highest berth among those active, not the
/// victim, targeting it, attacking, not mute, and of the original's AI
/// type above 2 ([`Npc::original_ai_type`], @0x36cd8): every ship the
/// player leads is a carried fighter (5) or an escort (6), whatever its
/// class; a fighter one of them launched is an NPC's carried fighter (5).
/// Every such ship speaks in slot order, so the last one's message
/// stands.
pub fn victory_speaker<'a>(
    npcs: impl IntoIterator<Item = &'a Npc>,
    victim: ShipRef,
) -> Option<&'a Npc> {
    npcs.into_iter()
        .filter(|npc| {
            npc.condition.active()
                && ShipRef::Npc(npc.id) != victim
                && npc.target == Some(victim)
                && matches!(npc.goal, Goal::Attack(_))
                && !npc.hull.mute
                && npc.original_ai_type() > 2
        })
        .max_by_key(|npc| npc.berth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::combat::hull::Condition;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, MissionMark, NpcId};
    use nova_sim_base::fixture::{Draws, FAST};

    fn speaker(ship: i16) -> Speaker {
        Speaker {
            ship: ShipId(ship),
            voice: None,
        }
    }

    /// A voter at `berth`, of ship class `berth`, changed as `changed`
    /// says.
    fn voter(berth: u32, changed: bool) -> Voter {
        Voter {
            berth,
            changed,
            speaks: changed,
            returning: false,
            mute: false,
            speaker: speaker(i16::try_from(berth).unwrap_or(0)),
        }
    }

    fn mute(voter: Voter) -> Voter {
        Voter {
            mute: true,
            speaks: false,
            ..voter
        }
    }

    fn returning(voter: Voter) -> Voter {
        Voter {
            returning: true,
            changed: true,
            ..voter
        }
    }

    fn spoken(voters: &[Voter], draws: &[u32]) -> Option<ShipId> {
        command_speaker(voters, &mut Draws::of(draws)).map(|speaker| speaker.ship)
    }

    #[test]
    fn a_voter_before_the_first_change_never_speaks() {
        assert_eq!(
            spoken(&[voter(1, false), voter(2, true)], &[0, 0]),
            Some(ShipId(2))
        );
        assert_eq!(spoken(&[voter(1, false)], &[0]), None);
    }

    #[test]
    fn the_first_changed_voter_speaks_with_none_chosen_and_no_draw() {
        let mut draws = Draws::of(&[]);
        let chosen = command_speaker(&[voter(1, true)], &mut draws);
        assert_eq!(chosen, Some(speaker(1)));
        assert!(draws.asked.is_empty(), "no draw needed");
    }

    #[test]
    fn a_later_voter_takes_over_on_a_0_and_not_on_a_1() {
        let voters = [voter(1, true), voter(2, false), voter(3, true)];
        assert_eq!(spoken(&voters, &[1, 0]), Some(ShipId(3)));
        assert_eq!(
            spoken(&voters, &[0, 1]),
            Some(ShipId(2)),
            "unchanged, it still takes it"
        );
        assert_eq!(spoken(&voters, &[1, 1]), Some(ShipId(1)));
    }

    #[test]
    fn a_mute_voter_never_takes_over() {
        assert_eq!(
            spoken(&[voter(1, true), mute(voter(2, true))], &[0]),
            Some(ShipId(1))
        );
        assert_eq!(
            spoken(&[mute(voter(1, true)), voter(2, false)], &[0]),
            Some(ShipId(2)),
            "a mute first change leaves it to the next"
        );
        assert_eq!(spoken(&[mute(voter(1, true))], &[0]), None);
    }

    #[test]
    fn a_returning_fighter_with_no_voice_chosen_takes_it_even_mute() {
        assert_eq!(
            spoken(&[mute(returning(voter(1, false)))], &[]),
            Some(ShipId(1))
        );
    }

    #[test]
    fn a_returning_fighter_takes_over_on_its_first_draw_of_0_or_else_its_normal_test() {
        let voters = [voter(1, true), returning(voter(2, false))];
        assert_eq!(spoken(&voters, &[0]), Some(ShipId(2)), "first draw 0");
        assert_eq!(spoken(&voters, &[1, 0]), Some(ShipId(2)), "1, then 0");
        assert_eq!(spoken(&voters, &[1, 1]), Some(ShipId(1)), "1, then 1");
        let mute = [voter(1, true), mute(returning(voter(2, false)))];
        assert_eq!(
            spoken(&mute, &[1, 0]),
            Some(ShipId(1)),
            "mute: no normal test"
        );
    }

    #[test]
    fn no_voter_speaking_gives_none() {
        assert_eq!(spoken(&[], &[]), None);
        assert_eq!(spoken(&[voter(1, false), voter(2, false)], &[0, 0]), None);
    }

    /// Ship `id` at berth `berth`, a warship attacking `target`.
    fn attacker(id: u32, berth: u32, target: ShipRef) -> Npc {
        let mut ship = npc(id, crate::stats::ShipStats::new(FAST, &[]));
        ship.berth = berth;
        ship.ai_type = AiType::Warship;
        ship.goal = Goal::Attack(target);
        ship.target = Some(target);
        ship
    }

    const VICTIM: ShipRef = ShipRef::Npc(NpcId(9));

    fn victor(npcs: &[Npc]) -> Option<NpcId> {
        victory_speaker(npcs, VICTIM).map(|npc| npc.id)
    }

    #[test]
    fn the_highest_berth_attacking_the_victim_says_the_victory() {
        let npcs = [
            attacker(1, 4, VICTIM),
            attacker(2, 7, VICTIM),
            attacker(3, 5, VICTIM),
        ];
        assert_eq!(victor(&npcs), Some(NpcId(2)));
    }

    #[test]
    fn a_trader_one_not_attacking_one_on_another_or_a_mute_one_says_nothing() {
        let mut trader = attacker(1, 9, VICTIM);
        trader.ai_type = AiType::BraveTrader;
        assert_eq!(victor(&[trader.clone()]), None, "a trader");
        let carried = Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..trader.clone()
        };
        assert_eq!(
            victor(&[carried]),
            Some(NpcId(1)),
            "a carried fighter, AI 5"
        );
        let mission = Npc {
            mission: Some(MissionMark {
                order: None,
                slot: 0,
                behav: crate::traffic::ShipBehav::EscortPlayer,
                formation: Some(crate::escort::FormationSlot::default()),
            }),
            ..trader
        };
        assert_eq!(victor(&[mission]), Some(NpcId(1)), "a mission escort, AI 6");
        let mut forming = attacker(1, 9, VICTIM);
        forming.goal = Goal::Formation {
            guard: Some(VICTIM),
        };
        assert_eq!(victor(&[forming]), None, "not attacking");
        let other = attacker(1, 9, ShipRef::Npc(NpcId(8)));
        assert_eq!(victor(&[other]), None, "on another ship");
        let mut quiet = attacker(1, 9, VICTIM);
        quiet.hull.mute = true;
        assert_eq!(victor(&[quiet]), None, "mute");
        let mut gone = attacker(1, 9, VICTIM);
        gone.condition = Condition::Destroyed;
        assert_eq!(victor(&[gone]), None, "inactive");
        let mut itself = attacker(9, 9, VICTIM);
        itself.target = Some(VICTIM);
        assert_eq!(victor(&[itself]), None, "the victim itself");
    }

    #[test]
    fn a_hired_or_captured_escort_of_a_trader_class_says_the_victory() {
        // Every ship the player leads is the original's AI type 5 or 6.
        let mut hired = attacker(1, 9, VICTIM);
        hired.ai_type = AiType::WimpyTrader;
        hired.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        assert_eq!(victor(&[hired]), Some(NpcId(1)));
    }

    #[test]
    fn a_fighter_or_medium_ship_that_isnt_mute_calls_its_target() {
        let mut ship = npc(1, crate::stats::ShipStats::new(FAST, &[]));
        for (class, calls) in [
            (EscortClass::Fighter, true),
            (EscortClass::Medium, true),
            (EscortClass::Warship, false),
            (EscortClass::Freighter, false),
        ] {
            ship.class = class;
            assert_eq!(calls_target(&ship), calls, "{class:?}");
        }
        ship.class = EscortClass::Fighter;
        ship.hull.mute = true;
        assert!(!calls_target(&ship), "mute");
    }

    #[test]
    fn a_carried_fighter_docking_is_returning() {
        let mut ship = npc(1, crate::stats::ShipStats::new(FAST, &[]));
        ship.goal = Goal::Dock(ShipRef::Player);
        assert!(!returning_fighter(&ship), "not carried");
        ship.carrier = Some(Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        assert!(returning_fighter(&ship));
        ship.goal = Goal::Formation { guard: None };
        assert!(!returning_fighter(&ship), "not docking");
    }
}
