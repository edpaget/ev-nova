//! A pirate's travel stellar: the original's +0x6c, which
//! `_PirateWarshipAI`'s copy of the idle block reads (@0x8c472-0x8c477 in
//! the `EV Nova` executable, [`idle_block::pirate_idle`]): while it is -1 a
//! pirate with nothing to fight picks a stellar; while it is set it leaves
//! or waits without picking. The think writes nothing to it first, so a
//! pirate keeps it through its fights and the drops that end them.
//!
//! The engine keeps it as [`Npc::travel`], the stellar it names or none
//! for -1, and a decision says how its think writes it
//! ([`Decision::travel`], [`TravelWrite`]); the traffic [`take`]s the
//! write as it takes the goal. It is read ([`held`]) as the stellar of the
//! ship's goal while it is bound for a stellar or a gate (goals 1 and 0x14
//! carry it), else as the gate it comes out of ([`Npc::emerges_from`], the
//! emerge's write @0x8954a), else as the field; [`is_set`] asks only
//! whether that is a stellar.
//!
//! **One number, read two ways.** +0x6c is one 16-bit word, and its
//! meaning depends on its writer: a stellar's table index (the `spöb` ID
//! less 128: a pick, a gate, a landing, a stellar attacker's quarry, a
//! defender's stellar), a ship's slot (an escort's leader, the player's 0
//! included; a fighter's carrier; a plunder's victim; a repair's target)
//! or, on the player in mode 3, a Con's slot. `_PirateWarshipAI` asks only
//! whether it is -1 (@0x8c472-0x8c477); `_MinerAI` takes the number for a
//! stellar's index with no bounds test (@0x8b36c-0x8b3ae, goal 1's read
//! @0x8d8d7-0x8d8e8), so a slot names the stellar of ID 128 + slot
//! ([`of_slot`]), a real record that usually belongs to another system.
//! The field holds each slot so ([`slot_of`]).
//!
//! Its writes a pirate passes through:
//!
//! - **Set**: the pick (@0x8c495; [`take`] sets it as a decision leaves a
//!   `Land` or `EnterGate` goal), and a plunder of a disabled ship, the
//!   victim's slot (`_HighLevelAIHandler` @0x8e8c9).
//! - **Cleared** (-1): a spawn (`_InitObjects` @0x1c3c1, `_RandomShipSpawn`
//!   @0x3c444, `_GenericRandomShipSpawn` @0x3c9e2); the count wait
//!   (@0x8c70d, see [`board_wait`](crate::traffic::board_wait)); the plunder
//!   (@0x8c7ae); the dry drop (@0x8c887); a plunder of an intact ship, on
//!   every frame (@0x8ea50); the lost-sight search of a ship above the
//!   traders (@0x8d833, see [`lost_sight`](super::lost_sight)); the end of a
//!   refuel (@0x8f6b6-0x8f6ce) or a repair (@0x8f717-0x8f732,
//!   `_LowLevelAIHandler` @0x86f09-0x86f31) and a plunder's give-up
//!   (@0x8e88f-0x8e8b9); a capture's drop of the ships that target its
//!   victim (`_AIPlunderShipContents` @0x84872-0x8488d); and the player's
//!   boarding's or capture's drop of the ships after its ship
//!   (`_AIShipBoardedByPlayer` @0x897e6-0x8980f, `_AIShipCapturedByPlayer`
//!   @0x896c5-0x896fa); and every victim the player's hit turns on it
//!   (`_AIMakeShipAttackPlayer` @0x89d47, from `_DamageShip` @0x3be69;
//!   the traffic's reaction), an assister included; and the end of a
//!   held ship's burst (`_AIMakeShipStopJumping` @0x82eaa, see
//!   [`held_burst`](crate::traffic::held_burst)). The
//!   `_AIMakeShipStopRefuelling` that follows it (@0x3bec7, its -1
//!   @0x833ba) never acts, as it needs goal 9 or 0xf and finds goal 4.
//! - **Kept**: the head drop (@0x8c32a-0x8c330), the disabled-warship drop
//!   (@0x8c839-0x8c842), an attack, and every goal the think keeps.
//!
//! The emerge's gate is folded in only when read: the burst start's -1
//! (@0x8dd93) clears [`Npc::emerges_from`] already, and a ship emerges only
//! as it spawns, when the flag is clear.
//!
//! Not written, as each would clear a flag already clear: the count wait's
//! end (@0x8d486), the drift (@0x8eb8b) and a plunder whose target has gone
//! (@0x8eb45). Every path into them has cleared it: the count wait, the
//! plunder, or a plunder's every-frame write followed by the head drop.
//!
//! Nothing here plays a sound. +0x6c is 16-bit there; not replicated.
//!
//! **A fleet's.** A ship with a leader holds its leader's slot here, the
//! player's 0 included, and any slot reads as set. So one that leaves its
//! fleet carries it into the idle block of the pirate it may become. Its
//! writes:
//!
//! - `_EscortAI`'s order block, whoever the parent ([`order_block`]);
//! - `_EscortAI`'s entry into goal 0xb, the lead's copied (@0x83a77-0x83a7c,
//!   [`copied`], see [`fleet_escort`](super::fleet_escort));
//! - goal 0xb's check and sub 0xd, copied or cleared, the leader the
//!   player or an NPC ([`fly_jump`](crate::traffic::warp_out::fly_jump));
//!   the player's +0x6c is its nav selection, a stellar in mode 2 or a Con
//!   slot in mode 3 (the session's `Chart::held_selection`), which
//!   `_EscortAI`'s entry copies too ([`copying`]);
//! - the cancels of a follow, -1 (@0x90118, @0x90196), and a fighter's
//!   turn-back, its carrier's slot (@0x8d621;
//!   [`jump_with_leader`](crate::traffic::jump_with_leader));
//! - a lead entering a gate, which gives its followers the gate (@0x8dd30);
//! - a successor taking over its lead's (@0x893b0-0x893e1,
//!   [`succession`](crate::traffic::succession));
//! - Use As My Ship on a lead, -1 for each ship it led (`_DoShipCapture`
//!   @0x41353-0x41398, [`Npc::lose_captured_lead`]); Use As Escort writes
//!   no follower's (@0x9461f-0x947a5, @0x89693-0x897a0);
//! - a mission escort's join: its first think's formation, the player's
//!   slot, as the session writes that formation.
//!
//! A follower keeps its own when its lead is lost, and so does a disabled
//! one left alone, until the disabled reset clears it. A ship the player
//! leads holds the player's slot through a mission's end, as
//! `_AIMakeShipLeave` (@0x7e2b0) doesn't write it: so a pirate mission
//! escort released in formation leaves, and one held picks a stellar.
//! `_AIFightThreatToParent`'s writes (@0x81f59, @0x81f8a) aren't modelled:
//! its one caller overwrites them (@0x83ef4, @0x83f09).
//!
//! **The other AI types'.** A successor under its lead's government reads
//! whatever the lead's think left, so each type writes its own:
//!
//! - the idle block of the traders and the warship, -1 then the pick
//!   ([`idle_block::pick_write`]); the warship's leave with no way to jump
//!   (@0x8c00c, @0x8c02a) and its fully-dry drop (@0x8c117);
//! - `_MinerAI`'s, which reads the number too (see
//!   [`miner`](super::miner)): -1 disabled (@0x8b232), mining with no rock
//!   (@0x8b2cb) and while its wait runs (@0x8b45d), and the nearest pick
//!   (@0x8b386);
//! - `_InterceptorAI`'s patrol pick, through its `Land` (@0x8d132), and
//!   the lost-sight drop's -1 (@0x8d3df,
//!   [`lost_sight::after_inspection`](super::lost_sight::after_inspection));
//!   goal 7's -1 as it scans the player (@0x8f369; the traffic's
//!   `take_goals`), kept for any other ship (@0x8f391);
//! - goal 0xf's repair, its target's slot (@0x8f74c, the traffic's
//!   `plunder` high level).
//!
//! **The session's.** A write that replaces a goal outside [`take`] leaves
//! a taken stellar goal's stellar held, as the original's +0x6c is left
//! unless the replacing routine writes it: the scan's find writes -1
//! (`_AIMakeShipAttackPlayer` @0x89d47), a ship falling in with the player
//! the player's slot (`_AIMakeEscortFlyInForm` @0x89bac), and the eject's
//! threat walk moves a threat's slot 0 onto the hulk's (@0x6f6ff-0x6f70d).
//! The releases of the ships the player leads and the pod's drop of the
//! player write none, as the original's don't.

#[cfg(doc)]
use crate::ai::{Decision, idle_block};
use crate::ai::{Goal, Surroundings};
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::escort::EscortOrder;
use crate::traffic::npc::Npc;

/// How a think writes its ship's travel stellar ([`Decision::travel`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TravelWrite {
    /// It writes none: the value is kept, or a stellar goal's written (see
    /// [`take`]).
    #[default]
    Keep,
    /// It writes this stellar, or the slot it stands for ([`of_slot`]).
    Set(StellarId),
    /// It writes -1.
    Clear,
}

/// The stellar a ship's or a Con's `slot` names when +0x6c holds it and a
/// reader takes it for a stellar's table index: the `spöb` of ID
/// 128 + `slot` (the table is indexed by ID less 128, @0x8d8d7-0x8d8e8).
#[must_use]
pub const fn of_slot(slot: u32) -> StellarId {
    // A ship's slot is below 64 and a Con's below 16, so the sum fits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    StellarId(128 + slot as i16)
}

/// The slot of `ship` as +0x6c holds it ([`of_slot`]): the player's 0, an
/// NPC's berth, among the system's ships or those gone from it; none for a
/// ship in neither. A gone ship's record is found by its id whatever tick
/// freed it, as it stands until a ship takes its berth: what is read is
/// only the slot's number, which the original copies from +0x70 the same
/// however long ago the slot was freed (@0x8f74c, @0x8e8c9). Its callers
/// pass a ship in the system or one the reader's target still holds.
#[must_use]
pub fn slot_of(ship: ShipRef, around: &Surroundings) -> Option<StellarId> {
    match ship {
        ShipRef::Player => Some(of_slot(0)),
        ShipRef::Npc(id) => around
            .npcs
            .iter()
            .chain(around.gone.iter().map(|gone| &gone.ship))
            .find(|npc| npc.id == id)
            .map(|npc| of_slot(npc.berth)),
    }
}

/// The value `npc`'s +0x6c holds as it is read: the stellar of its goal
/// (goals 1 and 0x14 carry it), else the gate it comes out of
/// ([`Npc::emerges_from`], the emerge's write @0x8954a), else
/// [`Npc::travel`].
#[must_use]
pub fn held(npc: &Npc) -> Option<StellarId> {
    stellar_of(npc.goal).or(npc.emerges_from).or(npc.travel)
}

/// Whether `npc`'s travel stellar (+0x6c) is set, so a pirate's idle block
/// picks none (see the module docs).
#[must_use]
pub fn is_set(npc: &Npc) -> bool {
    held(npc).is_some()
}

/// Takes `write` into `npc`'s value as it takes `goal`, before its goal is
/// replaced: set or cleared as `write` says, or else the stellar of the
/// goal it takes, else of the goal it leaves, else what it held.
pub fn take(npc: &mut Npc, write: TravelWrite, goal: Goal) {
    npc.travel = match write {
        TravelWrite::Set(stellar) => Some(stellar),
        TravelWrite::Clear => None,
        TravelWrite::Keep => stellar_of(goal).or(stellar_of(npc.goal)).or(npc.travel),
    };
}

/// The write that copies `lead`'s travel stellar as [`held`] reads it.
#[must_use]
pub fn copied(lead: &Npc) -> TravelWrite {
    copying(held(lead))
}

/// The write that copies a held value: `Set` of it, or `Clear` for -1.
#[must_use]
pub fn copying(value: Option<StellarId>) -> TravelWrite {
    value.map_or(TravelWrite::Clear, TravelWrite::Set)
}

/// How `_EscortAI`'s order block writes +0x6c as it decides `goal` under
/// `order`, `new_threat` when the block took a threat rather than keeping
/// its target, for every parent, `parent` the parent's slot (the player's
/// 0, an NPC lead's berth):
///
/// - Attack, a new threat: -1 (@0x83d83). A kept target writes none
///   (@0x83d6d), nor does Defend while it fights (@0x83cd6, @0x83e4b).
/// - Hold: -1 (@0x83e65).
/// - Defend or Attack with nothing to fight, a fighter's Return and
///   formation: the leader's slot (@0x83dac, @0x83e9f, @0x83f09).
///
/// The high level repeats the leader's slot every frame in goals 5, 0xa
/// and 0xc (@0x8ec04, @0x8ed4b, @0x8ef68). The think's write stands for
/// those: a lag of one think behind a -1 written between thinks, which
/// crosses no boundary the player sees.
#[must_use]
pub fn order_block(
    goal: Goal,
    order: Option<EscortOrder>,
    new_threat: bool,
    parent: u32,
) -> TravelWrite {
    match goal {
        Goal::Attack(_) if new_threat && order == Some(EscortOrder::Attack) => TravelWrite::Clear,
        Goal::Attack(_) => TravelWrite::Keep,
        Goal::Wait => TravelWrite::Clear,
        _ => TravelWrite::Set(of_slot(parent)),
    }
}

/// The stellar `goal` carries in +0x6c: a landing's (goal 1) or a gate's
/// entered (0x14).
fn stellar_of(goal: Goal) -> Option<StellarId> {
    match goal {
        Goal::Land(stellar) | Goal::EnterGate(stellar) => Some(stellar),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::ai::{Behaviour, Decision, Surroundings};
    use crate::catalog::StellarId;
    use crate::chance::Chance;
    use crate::combat::ShipRef;
    use crate::stats::ShipStats;
    use crate::traffic::Traffic;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::{Draws, FAST};

    const LAND: Goal = Goal::Land(StellarId(128));
    const GATE: Goal = Goal::EnterGate(StellarId(129));
    /// A stellar held in the field.
    const HELD: StellarId = StellarId(140);
    /// Another, written.
    const WRITTEN: StellarId = StellarId(141);

    fn ship() -> Npc {
        npc(1, ShipStats::new(FAST, &[]))
    }

    fn holding(goal: Goal, travel: Option<StellarId>) -> Npc {
        Npc {
            goal,
            travel,
            ..ship()
        }
    }

    /// The value `npc` holds once it takes `write` with `goal`.
    fn taken(mut npc: Npc, write: TravelWrite, goal: Goal) -> Option<StellarId> {
        take(&mut npc, write, goal);
        npc.travel
    }

    #[test]
    fn keep_holds_the_value_and_a_stellar_goal_writes_it() {
        let keep = TravelWrite::Keep;
        assert_eq!(taken(holding(Goal::Idle, None), keep, Goal::JumpOut), None);
        assert_eq!(
            taken(holding(Goal::Idle, Some(HELD)), keep, Goal::JumpOut),
            Some(HELD)
        );
    }

    #[test]
    fn a_taken_stellar_goal_writes_its_stellar() {
        let keep = TravelWrite::Keep;
        assert_eq!(
            taken(holding(Goal::Idle, None), keep, LAND),
            Some(StellarId(128))
        );
        assert_eq!(
            taken(holding(Goal::Wait, Some(HELD)), keep, GATE),
            Some(StellarId(129)),
            "over the value held"
        );
        assert_eq!(
            taken(holding(GATE, None), keep, LAND),
            Some(StellarId(128)),
            "the goal taken before the goal left"
        );
    }

    #[test]
    fn a_left_stellar_goal_keeps_its_stellar() {
        let keep = TravelWrite::Keep;
        assert_eq!(
            taken(holding(LAND, None), keep, Goal::Idle),
            Some(StellarId(128))
        );
        assert_eq!(
            taken(holding(GATE, Some(HELD)), keep, Goal::Wait),
            Some(StellarId(129)),
            "over the value held"
        );
    }

    #[test]
    fn set_and_clear_overrule_the_goal() {
        assert_eq!(
            taken(
                holding(Goal::Idle, Some(HELD)),
                TravelWrite::Set(WRITTEN),
                Goal::Idle
            ),
            Some(WRITTEN)
        );
        assert_eq!(
            taken(holding(Goal::Idle, None), TravelWrite::Set(WRITTEN), LAND),
            Some(WRITTEN)
        );
        assert_eq!(
            taken(holding(LAND, Some(HELD)), TravelWrite::Clear, LAND),
            None
        );
        assert_eq!(
            taken(holding(Goal::Idle, Some(HELD)), TravelWrite::Clear, GATE),
            None
        );
    }

    #[test]
    fn held_reads_the_goal_then_the_gate_then_the_field() {
        let emerging = |goal, travel| Npc {
            emerges_from: Some(StellarId(130)),
            ..holding(goal, travel)
        };
        assert_eq!(held(&holding(LAND, Some(HELD))), Some(StellarId(128)));
        assert_eq!(held(&emerging(GATE, Some(HELD))), Some(StellarId(129)));
        assert_eq!(
            held(&emerging(Goal::Idle, Some(HELD))),
            Some(StellarId(130))
        );
        assert_eq!(held(&holding(Goal::Idle, Some(HELD))), Some(HELD));
        assert_eq!(held(&holding(Goal::Wait, None)), None);
    }

    #[test]
    fn a_slot_names_the_stellar_of_id_128_up() {
        assert_eq!(of_slot(0), StellarId(128));
        assert_eq!(of_slot(5), StellarId(133));
        assert_eq!(of_slot(63), StellarId(191));
    }

    #[test]
    fn a_ships_slot_is_the_players_0_or_an_npcs_berth_here_or_gone() {
        use crate::traffic::gone::{Freed, GoneShips};
        let here = Npc {
            id: NpcId(2),
            berth: 7,
            ..ship()
        };
        let left = Npc {
            id: NpcId(3),
            berth: 9,
            ..ship()
        };
        let mut gone = GoneShips::default();
        gone.record(left, Freed::InDispatch);
        let npcs = [ship(), here];
        let around = Surroundings {
            gone: gone.ships(),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(slot_of(ShipRef::Player, &around), Some(StellarId(128)));
        assert_eq!(slot_of(ShipRef::Npc(NpcId(2)), &around), Some(of_slot(7)));
        assert_eq!(slot_of(ShipRef::Npc(NpcId(3)), &around), Some(of_slot(9)));
        assert_eq!(slot_of(ShipRef::Npc(NpcId(8)), &around), None);
    }

    #[test]
    fn copying_sets_a_held_value_and_clears_for_none() {
        assert_eq!(copying(Some(HELD)), TravelWrite::Set(HELD));
        assert_eq!(copying(None), TravelWrite::Clear);
        assert_eq!(
            copied(&holding(LAND, None)),
            TravelWrite::Set(StellarId(128))
        );
    }

    #[test]
    fn the_order_block_sets_keeps_or_clears_the_travel_stellar() {
        // The parent's slot, berth 3 or the player's 0.
        const PARENT: TravelWrite = TravelWrite::Set(StellarId(131));
        const ATTACK: Goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
        const FORM: Goal = Goal::Formation { guard: None };
        let follow = Goal::Follow {
            lead: NpcId(2),
            guard: None,
        };
        let attack = Some(EscortOrder::Attack);
        let defend = Some(EscortOrder::Defend);
        for (goal, order, new_threat, write) in [
            (ATTACK, attack, true, TravelWrite::Clear),
            (ATTACK, attack, false, TravelWrite::Keep),
            (ATTACK, defend, true, TravelWrite::Keep),
            (ATTACK, defend, false, TravelWrite::Keep),
            (ATTACK, None, true, TravelWrite::Keep),
            (
                Goal::Wait,
                Some(EscortOrder::Hold),
                false,
                TravelWrite::Clear,
            ),
            (FORM, None, false, PARENT),
            (FORM, attack, false, PARENT),
            (follow, defend, false, PARENT),
            (
                Goal::Dock(ShipRef::Player),
                Some(EscortOrder::Dock),
                false,
                PARENT,
            ),
        ] {
            assert_eq!(
                order_block(goal, order, new_threat, 3),
                write,
                "{goal:?} {order:?} {new_threat}"
            );
        }
        assert_eq!(
            order_block(FORM, None, false, 0),
            TravelWrite::Set(StellarId(128)),
            "the player's"
        );
    }

    #[test]
    fn the_emerges_gate_and_a_stellar_goal_read_as_set() {
        assert!(!is_set(&ship()));
        assert!(is_set(&holding(Goal::Idle, Some(HELD))));
        assert!(is_set(&holding(LAND, None)));
        assert!(is_set(&holding(GATE, None)));
        let emerging = Npc {
            emerges_from: Some(StellarId(129)),
            ..ship()
        };
        assert!(is_set(&emerging));
        assert!(!is_set(&holding(Goal::Wait, None)));
        assert!(!is_set(&holding(Goal::JumpOut, None)));
    }

    /// Gives its queued decisions in turn, then idles.
    #[derive(Debug)]
    struct Scripted(RefCell<Vec<Decision>>);

    impl Scripted {
        fn of(decisions: &[Decision]) -> Self {
            Self(RefCell::new(decisions.iter().rev().copied().collect()))
        }
    }

    impl Behaviour for Scripted {
        fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
            self.decision(npc, around, chance).goal
        }

        fn decision(&self, _: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Decision {
            self.0.borrow_mut().pop().unwrap_or_default()
        }
    }

    fn of(goal: Goal) -> Decision {
        Decision {
            goal,
            ..Decision::default()
        }
    }

    /// The flag after each of `decisions`, one a tick, of a pirate in the
    /// traffic.
    fn flags(decisions: &[Decision]) -> Vec<bool> {
        let mut traffic = Traffic::with_interval(1);
        traffic.add_npc(ship());
        let script = Scripted::of(decisions);
        decisions
            .iter()
            .map(|_| {
                traffic.tick(&script, &[], &mut Draws::of(&[0; 8]));
                traffic.npcs()[0].travel_set()
            })
            .collect()
    }

    #[test]
    fn in_traffic_a_pirate_bound_for_a_stellar_keeps_it_set_through_a_fight() {
        let fight = of(Goal::Attack(ShipRef::Npc(NpcId(2))));
        assert_eq!(
            flags(&[of(LAND), fight, of(Goal::Idle)]),
            [true, true, true],
            "set by the landing, kept through the fight and its drop"
        );
        assert_eq!(flags(&[fight, of(Goal::Idle)]), [false, false], "no pick");
        let cleared = Decision {
            travel: TravelWrite::Clear,
            ..of(Goal::Idle)
        };
        assert_eq!(flags(&[of(LAND), fight, cleared]), [true, true, false]);
    }
}
