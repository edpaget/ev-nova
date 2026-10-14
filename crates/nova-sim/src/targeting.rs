//! The player's target: which NPC a target command picks.
//!
//! The rules are the original's (`_HandlePlayer` and the searches it calls
//! in the `EV Nova` executable):
//!
//! - A ship is targetable ([`targetable`]) while it is intact or
//!   disabled: a ship breaking up or destroyed is not (`_IsDying`). Target
//!   Select skips the escort class (the player's hired escorts and the
//!   ships they lead, see below), and Closest Target every ship the
//!   player leads, mission escorts too ([`nearest`]).
//!   A cloaked ship ([`Cloak::is_cloaked`](crate::cloak::Cloak::is_cloaked))
//!   other than the player's escort is not targetable, and a target that
//!   cloaks is let go, unless the player can target cloaked ships (a cloak
//!   scanner's `ModVal` 0x0008, `_ShipCanTargetCloakedShips` @0x2466;
//!   `_HandlePlayer` @0x686bf-0x686ee, the searches @0x6a440-0x6a460, a
//!   mouse click @0x3810b-0x38130): the original asks the cloak itself,
//!   not whether the player sees the ship. A ship whose type is
//!   untargetable (`shïp` `Flags2` 0x0004,
//!   [`CloakGear::untargetable`](crate::cloak::CloakGear::untargetable))
//!   is no candidate for any search unless the player can target
//!   untargetable ships (a cloak scanner's `ModVal` 0x0004,
//!   [`Scanner::untargetable`], `_ShipCanTargetUntargetableShips`
//!   @0x238f; `_FindNearestShipToPlayer` @0x935e,
//!   `_FindNearestThreatToPlayer` @0x950f, `_FindPrevShipInSystem`
//!   @0xa451 and @0xa561, `_FindNextShipInSystem` @0xa825 and @0xa936).
//!   Escort Select has no exception for it, as it has for a cloak. A
//!   target already held is not let go for it: the lost-target check
//!   (`_HandlePlayer` @0x686a4-0x686ee) never reads the bit, so
//!   [`targetable`] does not either. A click on a ship (`_HandlePlayer`
//!   @0x6a480-0x6a4a3) applies the same gate, but ship clicks are not
//!   modelled (rdm task nav-selection-follow-up). A
//!   ship waiting at the hypergate or wormhole it comes out of
//!   ([`Npc::emerging`]) is not targetable
//!   (`_AIShipIsEmergingFromHypergate` @0x7e2e8, read by
//!   `_FindNearestShipToPlayer` @0x9330 and the next and previous
//!   searches @0xa42d-0xa918, `_FindNextShipInSystem` @0xa801), and a
//!   target emerging is let go (`_HandlePlayer` @0x686a4); once it bursts
//!   out of the gate it is a candidate again.
//! - Target Select (Tab, `_FindNextShipInSystem` @0xa5b1, its caller
//!   `_HandlePlayer` @0x69f1c-0x69ffb) walks the ship table up, in
//!   [`Npc::berth`] order (a tie, which the original's table cannot
//!   have, going to the NPC that appeared first), from the one after the
//!   target, or from the first with none ([`next`]). Shift-Tab
//!   (`_FindPrevShipInSystem` @0xa1dd, `_QuickKeyCheck(0x38)` @0x69f74)
//!   walks it down, from the one before the target, or from the last with
//!   none ([`previous`]). Neither wraps: past the end the target becomes
//!   none, and the next press starts again from the other end. A target
//!   no longer in the system counts as none.
//! - Escort Select (Option-Tab, Shift-Option-Tab back; `_KeyCheck(0x3a)`
//!   @0xa5c8) walks the escort class alone, the same way ([`next_escort`],
//!   [`previous_escort`]), and Target Select every other ship
//!   (@0xa755-0xa854). The escort class (the original's `E`,
//!   @0xa6c1-0xa728) is the player's hired escorts and the ships with no
//!   mission mark that one leads, such as a hired escort's fighter
//!   (`_LaunchFighter` @0x3d866-0x3d870); a mission escort, and a fighter
//!   it launched, are Target Select's. A cloaked ship is skipped unless
//!   the player can target cloaked ships, or the search is Escort Select,
//!   which takes a cloaked ship of the escort class.
//! - Closest Target (R, @0x6a9ab-0x6aadb) picks the candidate nearest the
//!   player by squared distance (@0x9368-0x93ba), with no limit; only a
//!   strictly nearer ship replaces the best, so a tie goes to the earlier.
//!   Option-R takes any ship (`_FindNearestShipToPlayer` @0x923f) and plain
//!   R only the threats (`_FindNearestThreatToPlayer` @0x93f9, with
//!   `_IsThreatToPlayer` @0x7f501: an intact ship attacking, sniping at or
//!   fleeing from the player or a ship he leads, @0x7f572-0x7f590,
//!   [`Npc::threatens_player_or_led`]), so [`nearest`] takes the filter.

use crate::cloak::Scanner;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
#[cfg(doc)]
use crate::traffic::npc::Mode;
use crate::traffic::npc::{Npc, NpcId};

/// How a target command picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetPick {
    /// The candidate nearest the player.
    Nearest,
    /// The candidate nearest the player that threatens it or a ship it
    /// leads (`_IsThreatToPlayer` @0x7f572-0x7f590,
    /// [`Npc::threatens_player_or_led`]).
    NearestThreat,
    /// The next candidate after the target ([`next`]): Tab.
    Next,
    /// The next ship of the escort class after the target (the player's
    /// hired escorts and the unmarked ships they lead, such as their
    /// fighters; [`next_escort`]), cloaked or not: Option-Tab.
    NextEscort,
    /// The candidate before the target ([`previous`]): Shift-Tab.
    Previous,
    /// The ship of the escort class before the target
    /// ([`previous_escort`]), cloaked or not: Shift-Option-Tab.
    PreviousEscort,
}

/// Whether `npc` is there to be targeted at all, whatever its cloak: it
/// is intact or disabled, and not emerging from a gate
/// ([`Mode::Emerging`]). Every search and the lost-target check start
/// from this, each adding its own cloak exception.
fn alive_and_present(npc: &Npc) -> bool {
    matches!(npc.condition, Condition::Intact | Condition::Disabled) && !npc.emerging()
}

/// Whether `npc` can be targeted by a player that can target cloaked
/// ships (`cloaked_ok`) or not: it is alive and present (`alive_and_present`),
/// and not cloaked unless `cloaked_ok` or it escorts the player.
#[must_use]
pub fn targetable(npc: &Npc, cloaked_ok: bool) -> bool {
    alive_and_present(npc) && (cloaked_ok || npc.escorts_player() || !npc.cloak.is_cloaked())
}

/// Whether a player of `scanner` may pick `npc` for its type: unless
/// `scanner` targets untargetable ships, `npc`'s type must not be
/// untargetable (see the module docs).
fn type_targetable(npc: &Npc, scanner: Scanner) -> bool {
    scanner.untargetable || !npc.cloak_gear.untargetable()
}

/// Whether `npc` is a candidate for Target Select (`escorts` false) or
/// Escort Select (`escorts` true) among `npcs` for a player of
/// `scanner`: intact or disabled, not emerging from a gate
/// ([`Mode::Emerging`]), not cloaked unless `scanner` targets cloaked
/// ships or the search is Escort Select, of a type it may pick
/// ([`type_targetable`], with no exception for Escort Select), and in the
/// escort class ([`escort_class`]) just when the search is Escort Select
/// (`_FindNextShipInSystem` @0xa755-0xa854).
fn cycles(npc: &Npc, npcs: &[Npc], scanner: Scanner, escorts: bool) -> bool {
    alive_and_present(npc)
        && (scanner.targets || escorts || !npc.cloak.is_cloaked())
        && type_targetable(npc, scanner)
        && escort_class(npc, npcs) == escorts
}

/// Whether `npc` is in the escort class among `npcs` (the original's
/// `E`, `_FindNextShipInSystem` @0xa6c1-0xa728): one of the player's hired
/// escorts, or a ship with no mission mark led by one, such as a hired
/// escort's fighter. A mission escort is not, nor a ship it leads: a
/// mission escort never carries an [`EscortDuty`](crate::escort::EscortDuty).
fn escort_class(npc: &Npc, npcs: &[Npc]) -> bool {
    npc.escort.is_some()
        || (npc.mission.is_none()
            && npc.leader.is_some_and(|leader| {
                npcs.iter()
                    .any(|ship| ship.id == leader && ship.escort.is_some())
            }))
}

/// The ship among `npcs` that passes `filter` nearest `from`, by squared
/// distance, a tie going to the earlier; none when none qualifies. Cloaked
/// ships are candidates when `scanner` targets them, and ships of an
/// untargetable type when it targets those. Every ship the player leads
/// ([`Npc::escorts_player`], [`Session::player_led`](crate::Session::player_led))
/// is skipped, mission escorts too (`_FindNearestShipToPlayer` @0x92a6,
/// `_FindNearestThreatToPlayer` @0x94db), so [`targetable`]'s exception
/// for a cloaked escort never applies here: the original skips the
/// escorts (@0x92a6) before it asks about the cloak (@0x92d6).
#[must_use]
pub fn nearest(
    npcs: &[Npc],
    from: Vec2,
    scanner: Scanner,
    filter: impl Fn(&Npc) -> bool,
) -> Option<NpcId> {
    let mut best: Option<(f32, NpcId)> = None;
    for npc in npcs
        .iter()
        .filter(|npc| nearest_candidate(npc, scanner) && filter(npc))
    {
        let off = npc.state.position - from;
        let distance = off.x * off.x + off.y * off.y;
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, npc.id));
        }
    }
    best.map(|(_, id)| id)
}

/// Whether `npc` is a candidate for Closest Target for a player of
/// `scanner`: targetable, as `scanner` says of cloaked ships, of a type
/// it may pick ([`type_targetable`]), and not led by the player.
fn nearest_candidate(npc: &Npc, scanner: Scanner) -> bool {
    targetable(npc, scanner.targets) && type_targetable(npc, scanner) && !npc.escorts_player()
}

/// The first candidate among `npcs` after `current` in ship-table order,
/// or the first candidate when there is no target; none past the last.
/// Cloaked ships, and ships of an untargetable type, are candidates when
/// `scanner` targets them.
#[must_use]
pub fn next(npcs: &[Npc], current: Option<NpcId>, scanner: Scanner) -> Option<NpcId> {
    walk(npcs, current, Walk::Up, |npc| {
        cycles(npc, npcs, scanner, false)
    })
}

/// The last candidate among `npcs` before `current` in ship-table order,
/// or the last candidate when there is no target; none past the first.
/// Cloaked ships, and ships of an untargetable type, are candidates when
/// `scanner` targets them.
#[must_use]
pub fn previous(npcs: &[Npc], current: Option<NpcId>, scanner: Scanner) -> Option<NpcId> {
    walk(npcs, current, Walk::Down, |npc| {
        cycles(npc, npcs, scanner, false)
    })
}

/// The first ship of the escort class among `npcs` after `current` in
/// ship-table order: the player's hired escorts and the ships with no
/// mission mark they lead, such as their fighters (`escort_class`); the
/// first when there is no target, none past the last. A cloaked ship of
/// the class is a candidate whether or not the player can target cloaked
/// ships; one of an untargetable type only when `scanner` targets those.
#[must_use]
pub fn next_escort(npcs: &[Npc], current: Option<NpcId>, scanner: Scanner) -> Option<NpcId> {
    walk(npcs, current, Walk::Up, |npc| {
        cycles(npc, npcs, scanner, true)
    })
}

/// The last ship of the escort class among `npcs` before `current` in
/// ship-table order: the player's hired escorts and the ships with no
/// mission mark they lead, such as their fighters (`escort_class`); the
/// last when there is no target, none past the first. A cloaked ship of
/// the class is a candidate whether or not the player can target cloaked
/// ships; one of an untargetable type only when `scanner` targets those.
#[must_use]
pub fn previous_escort(npcs: &[Npc], current: Option<NpcId>, scanner: Scanner) -> Option<NpcId> {
    walk(npcs, current, Walk::Down, |npc| {
        cycles(npc, npcs, scanner, true)
    })
}

/// Which way a search walks the ship table.
#[derive(Clone, Copy)]
enum Walk {
    /// Up the table, as `_FindNextShipInSystem` does.
    Up,
    /// Down the table, as `_FindPrevShipInSystem` does.
    Down,
}

/// The nearest NPC among `npcs` passing `filter` beyond `current` in
/// ship-table order (its [`Npc::berth`], a tie going to the one that
/// appeared first), the way `way` goes; or the nearest the end it starts
/// from when there is no target or the target is no longer among `npcs`.
fn walk(
    npcs: &[Npc],
    current: Option<NpcId>,
    way: Walk,
    filter: impl Fn(&Npc) -> bool,
) -> Option<NpcId> {
    let from = current
        .and_then(|current| npcs.iter().find(|npc| npc.id == current))
        .map(table_order);
    let beyond = |npc: &&Npc| {
        from.is_none_or(|from| match way {
            Walk::Up => table_order(npc) > from,
            Walk::Down => table_order(npc) < from,
        })
    };
    let ahead = npcs.iter().filter(|npc| filter(npc)).filter(beyond);
    match way {
        Walk::Up => ahead.min_by_key(|npc| table_order(npc)),
        Walk::Down => ahead.max_by_key(|npc| table_order(npc)),
    }
    .map(|npc| npc.id)
}

/// Where `npc` stands in the ship table: by its berth, then by its number.
fn table_order(npc: &Npc) -> (u32, NpcId) {
    (npc.berth, npc.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloak::{CloakGear, UNTARGETABLE};
    use crate::stats::ShipStats;
    use crate::testkit::npc;

    /// A player with no cloak scanner.
    const BLIND: Scanner = Scanner::NONE;
    /// A player whose scanner targets cloaked ships alone.
    const SEES_CLOAKED: Scanner = Scanner {
        targets: true,
        ..Scanner::NONE
    };
    /// A player whose scanner targets ships of an untargetable type alone.
    const SEES_UNTARGETABLE: Scanner = Scanner {
        untargetable: true,
        ..Scanner::NONE
    };

    /// `npc`, of an untargetable ship type ([`UNTARGETABLE`]).
    fn untargetable(mut npc: Npc) -> Npc {
        npc.cloak_gear = CloakGear {
            flags2: UNTARGETABLE,
            ..CloakGear::default()
        };
        npc
    }

    /// NPC `id` at (`x`, `y`), in `condition`.
    fn at(id: u32, x: f32, y: f32, condition: Condition) -> Npc {
        let mut npc = npc(id, ShipStats::default());
        npc.state.position = Vec2::new(x, y);
        npc.condition = condition;
        npc
    }

    /// [`nearest`] for a player that cannot target cloaked ships.
    fn nearest_(npcs: &[Npc], from: Vec2, filter: impl Fn(&Npc) -> bool) -> Option<NpcId> {
        nearest(npcs, from, BLIND, filter)
    }

    fn intact(id: u32, x: f32, y: f32) -> Npc {
        at(id, x, y, Condition::Intact)
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 5,
        starting: false,
    };

    #[test]
    fn intact_and_disabled_ships_are_targetable_and_dying_or_destroyed_ones_are_not() {
        let of = |condition| targetable(&at(0, 0.0, 0.0, condition), false);
        assert!(of(Condition::Intact));
        assert!(of(Condition::Disabled));
        assert!(!of(DYING));
        assert!(!of(Condition::Destroyed));
    }

    #[test]
    fn a_ship_emerging_from_a_gate_is_neither_targetable_nor_a_candidate() {
        let mut emerging = intact(1, 10.0, 0.0);
        emerging.mode = crate::traffic::npc::Mode::Emerging { ticks_left: 9 };
        assert!(!targetable(&emerging, false));
        let mut arriving = intact(1, 10.0, 0.0);
        arriving.mode = crate::traffic::npc::Mode::JumpingIn { ticks_left: 9 };
        assert!(targetable(&arriving, false), "a ship jumping in still is");
        let npcs = [emerging, intact(2, 300.0, 0.0)];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs[..1], None, BLIND), None);
    }

    #[test]
    fn the_nearest_is_by_squared_distance() {
        let npcs = [
            intact(1, 300.0, 0.0),
            intact(2, -100.0, 120.0),
            intact(3, 0.0, -200.0),
        ];
        let any = |_: &Npc| true;
        assert_eq!(nearest_(&npcs, Vec2::ZERO, any), Some(NpcId(2)));
        assert_eq!(nearest_(&npcs, Vec2::new(250.0, 0.0), any), Some(NpcId(1)));
        assert_eq!(nearest_(&npcs, Vec2::new(0.0, -150.0), any), Some(NpcId(3)));
    }

    #[test]
    fn a_tie_goes_to_the_earlier_npc() {
        let npcs = [
            intact(4, 0.0, 100.0),
            intact(5, 100.0, 0.0),
            intact(6, -100.0, 0.0),
        ];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(4)));
        let later = [intact(7, 0.0, 200.0), npcs[1].clone(), npcs[2].clone()];
        assert_eq!(nearest_(&later, Vec2::ZERO, |_| true), Some(NpcId(5)));
    }

    #[test]
    fn the_nearest_may_be_disabled_but_never_dying_or_destroyed() {
        let npcs = [
            at(1, 10.0, 0.0, DYING),
            at(2, 20.0, 0.0, Condition::Destroyed),
            at(3, 30.0, 0.0, Condition::Disabled),
            intact(4, 40.0, 0.0),
        ];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(3)));
        assert_eq!(nearest_(&npcs[..2], Vec2::ZERO, |_| true), None);
    }

    #[test]
    fn a_ship_waiting_at_its_gate_is_skipped_but_not_one_bursting_out() {
        use crate::traffic::npc::Mode;
        let mut waiting = intact(1, 10.0, 0.0);
        waiting.mode = Mode::Emerging { ticks_left: 30 };
        let mut leaving = intact(2, 20.0, 0.0);
        leaving.mode = Mode::LeavingGate { tick: 1 };
        let npcs = [waiting, leaving, intact(3, 30.0, 0.0)];
        assert!(!cycles(&npcs[0], &npcs, BLIND, false));
        assert!(cycles(&npcs[1], &npcs, BLIND, false));
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        assert_eq!(nearest_(&npcs[..1], Vec2::ZERO, |_| true), None);
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs[..1], None, BLIND), None);
    }

    #[test]
    fn the_nearest_passes_the_filter() {
        let npcs = [intact(1, 10.0, 0.0), intact(2, 20.0, 0.0)];
        assert_eq!(
            nearest_(&npcs, Vec2::ZERO, |npc| npc.id != NpcId(1)),
            Some(NpcId(2))
        );
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| false), None);
        assert_eq!(nearest_(&[], Vec2::ZERO, |_| true), None);
    }

    #[test]
    fn next_walks_the_npcs_in_order_then_gives_none() {
        let npcs = [
            intact(2, 0.0, 0.0),
            intact(5, 0.0, 0.0),
            intact(9, 0.0, 0.0),
        ];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = next(&npcs, current, BLIND);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(2)),
                Some(NpcId(5)),
                Some(NpcId(9)),
                None,
                Some(NpcId(2))
            ]
        );
        assert_eq!(next(&[], None, BLIND), None);
    }

    #[test]
    fn a_target_no_longer_in_the_system_walks_from_the_end() {
        let npcs = [intact(2, 0.0, 0.0), intact(9, 0.0, 0.0)];
        assert_eq!(next(&npcs, Some(NpcId(5)), BLIND), Some(NpcId(2)));
        assert_eq!(previous(&npcs, Some(NpcId(5)), BLIND), Some(NpcId(9)));
        assert_eq!(next(&npcs, Some(NpcId(10)), BLIND), Some(NpcId(2)));
    }

    #[test]
    fn previous_walks_the_berths_down_from_the_last_then_gives_none() {
        let npcs = [berthed(1, 2), berthed(2, 3), berthed(3, 1)];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = previous(&npcs, current, BLIND);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(2)),
                Some(NpcId(1)),
                Some(NpcId(3)),
                None,
                Some(NpcId(2))
            ]
        );
        assert_eq!(previous(&[], None, BLIND), None);
        let tied = [berthed(4, 1), berthed(6, 1)];
        assert_eq!(previous(&tied, None, BLIND), Some(NpcId(6)));
        assert_eq!(previous(&tied, Some(NpcId(6)), BLIND), Some(NpcId(4)));
    }

    #[test]
    fn previous_escort_walks_the_escorts_down() {
        let npcs = [
            berthed_escort(1, 1),
            berthed(2, 2),
            berthed_escort(4, 3),
            berthed(5, 4),
            berthed_escort(7, 5),
        ];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = previous_escort(&npcs, current, BLIND);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(7)),
                Some(NpcId(4)),
                Some(NpcId(1)),
                None,
                Some(NpcId(7))
            ]
        );
        assert_eq!(
            previous_escort(&npcs, Some(NpcId(5)), BLIND),
            Some(NpcId(4)),
            "before a non-escort"
        );
        assert_eq!(previous_escort(&npcs[1..2], None, BLIND), None);
    }

    #[test]
    fn next_skips_the_dying_and_takes_the_disabled() {
        let npcs = [
            at(1, 0.0, 0.0, DYING),
            at(2, 0.0, 0.0, Condition::Disabled),
            at(3, 0.0, 0.0, Condition::Destroyed),
            intact(4, 0.0, 0.0),
        ];
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(2)), BLIND), Some(NpcId(4)));
        assert_eq!(next(&npcs[..1], None, BLIND), None);
    }

    /// NPC `id` at (`x`, `y`), the player's escort.
    fn escort(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = intact(id, x, y);
        npc.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order: None,
        });
        npc
    }

    /// NPC `id` at (`x`, `y`), a mission escort (`ShipBehav` 1) that has
    /// joined the player.
    fn mission_escort(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = intact(id, x, y);
        npc.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot::default()),
        });
        npc
    }

    #[test]
    fn the_nearest_passes_over_a_mission_escort_for_the_next_nearest_ship() {
        let npcs = [mission_escort(1, 5.0, 0.0), intact(2, 10.0, 0.0)];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        assert_eq!(nearest_(&npcs[..1], Vec2::ZERO, |_| true), None);
        assert_eq!(
            next(&npcs, None, BLIND),
            Some(NpcId(1)),
            "Tab still takes a mission escort"
        );
    }

    #[test]
    fn a_disabled_or_cloaked_mission_escort_is_never_the_nearest() {
        let mut disabled = mission_escort(1, 5.0, 0.0);
        disabled.condition = Condition::Disabled;
        let npcs = [disabled, intact(2, 10.0, 0.0)];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        let mut hidden = mission_escort(1, 5.0, 0.0);
        hidden.cloak = cloaked(1, 0.0, 0.0).cloak;
        let npcs = [hidden, intact(2, 10.0, 0.0)];
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, SEES_CLOAKED, |_| true),
            Some(NpcId(2))
        );
    }

    #[test]
    fn the_nearest_and_next_skip_the_players_escorts() {
        let npcs = [
            escort(1, 10.0, 0.0),
            intact(2, 20.0, 0.0),
            escort(3, 5.0, 0.0),
        ];
        assert_eq!(nearest_(&npcs, Vec2::ZERO, |_| true), Some(NpcId(2)));
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(2)), BLIND), None);
        assert_eq!(nearest_(&[escort(1, 0.0, 0.0)], Vec2::ZERO, |_| true), None);
        assert!(
            targetable(&npcs[0], false),
            "an escort targeted stays targeted"
        );
    }

    #[test]
    fn next_escort_walks_the_escorts_alone_in_order_then_gives_none() {
        let npcs = [
            escort(1, 0.0, 0.0),
            intact(2, 0.0, 0.0),
            escort(4, 0.0, 0.0),
            intact(5, 0.0, 0.0),
            escort(7, 0.0, 0.0),
        ];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..5 {
            current = next_escort(&npcs, current, BLIND);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [
                Some(NpcId(1)),
                Some(NpcId(4)),
                Some(NpcId(7)),
                None,
                Some(NpcId(1))
            ]
        );
        assert_eq!(
            next_escort(&npcs, Some(NpcId(2)), BLIND),
            Some(NpcId(4)),
            "after a non-escort"
        );
        assert_eq!(next_escort(&npcs[1..2], None, BLIND), None);
    }

    /// NPC `id` at ship-table `berth`, intact.
    fn berthed(id: u32, berth: u32) -> Npc {
        let mut npc = intact(id, 0.0, 0.0);
        npc.berth = berth;
        npc
    }

    /// NPC `id` at ship-table `berth`, the player's escort.
    fn berthed_escort(id: u32, berth: u32) -> Npc {
        let mut npc = escort(id, 0.0, 0.0);
        npc.berth = berth;
        npc
    }

    #[test]
    fn next_and_next_escort_reach_a_lower_berth_before_a_lower_id() {
        let npcs = [berthed(2, 3), berthed(5, 1)];
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(5)));
        assert_eq!(next(&npcs, Some(NpcId(5)), BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs, Some(NpcId(2)), BLIND), None);
        let escorts = [berthed_escort(2, 3), berthed_escort(5, 1)];
        assert_eq!(next_escort(&escorts, None, BLIND), Some(NpcId(5)));
        assert_eq!(next_escort(&escorts, Some(NpcId(5)), BLIND), Some(NpcId(2)));
        assert_eq!(next_escort(&escorts, Some(NpcId(2)), BLIND), None);
    }

    /// NPC `id` at ship-table `berth`, a fighter `leader` launched: led
    /// by it, with no duty of its own (as `join_carrier` makes one).
    fn fighter_of(id: u32, berth: u32, leader: u32) -> Npc {
        let mut npc = berthed(id, berth);
        npc.leader = Some(NpcId(leader));
        npc
    }

    #[test]
    fn a_hired_escorts_fighter_is_option_tabs_and_not_tabs() {
        let npcs = [berthed_escort(1, 1), fighter_of(2, 2, 1), berthed(3, 3)];
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(3)));
        assert_eq!(next(&npcs, Some(NpcId(3)), BLIND), None);
        assert_eq!(previous(&npcs, None, BLIND), Some(NpcId(3)));
        assert_eq!(previous(&npcs, Some(NpcId(3)), BLIND), None);
        assert_eq!(next_escort(&npcs, Some(NpcId(1)), BLIND), Some(NpcId(2)));
        assert_eq!(previous_escort(&npcs, None, BLIND), Some(NpcId(2)));
    }

    #[test]
    fn a_fighter_led_by_a_mission_escort_and_a_mission_ship_led_by_an_escort_are_tabs() {
        let mut ours = mission_escort(1, 0.0, 0.0);
        ours.berth = 1;
        let mut aboard = fighter_of(4, 4, 3);
        aboard.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::mission::ships::ShipBehav::Own,
            formation: None,
        });
        let npcs = [ours, fighter_of(2, 2, 1), berthed_escort(3, 3), aboard];
        let mut walked = Vec::new();
        let mut current = None;
        for _ in 0..4 {
            current = next(&npcs, current, BLIND);
            walked.push(current);
        }
        assert_eq!(
            walked,
            [Some(NpcId(1)), Some(NpcId(2)), Some(NpcId(4)), None]
        );
        assert_eq!(next_escort(&npcs, None, BLIND), Some(NpcId(3)));
        assert_eq!(next_escort(&npcs, Some(NpcId(3)), BLIND), None);
        let led_by_traffic = [berthed(5, 1), fighter_of(6, 2, 5)];
        assert_eq!(next(&led_by_traffic, Some(NpcId(5)), BLIND), Some(NpcId(6)));
        assert_eq!(next_escort(&led_by_traffic, None, BLIND), None);
    }

    #[test]
    fn tab_passes_over_a_cloaked_mission_escort_without_the_scanner_and_option_tab_takes_a_cloaked_escorts_fighter()
     {
        let mut ours = mission_escort(1, 0.0, 0.0);
        ours.cloak = cloaked(1, 0.0, 0.0).cloak;
        let npcs = [ours, berthed(2, 2)];
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(previous(&npcs, Some(NpcId(2)), BLIND), None);
        assert_eq!(next(&npcs, None, SEES_CLOAKED), Some(NpcId(1)));
        let mut hidden = fighter_of(4, 2, 3);
        hidden.cloak = cloaked(4, 0.0, 0.0).cloak;
        let escorts = [berthed_escort(3, 1), hidden];
        assert_eq!(next_escort(&escorts, Some(NpcId(3)), BLIND), Some(NpcId(4)));
        assert_eq!(next(&escorts, None, BLIND), None);
    }

    /// [`intact`], cloaked.
    fn cloaked(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = intact(id, x, y);
        npc.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        npc
    }

    #[test]
    fn a_cloaked_ship_is_targetable_only_by_a_player_that_can_target_cloaked_ships() {
        let hidden = cloaked(1, 10.0, 0.0);
        assert!(!targetable(&hidden, false));
        assert!(targetable(&hidden, true));
        let npcs = [hidden, intact(2, 20.0, 0.0)];
        assert_eq!(nearest(&npcs, Vec2::ZERO, BLIND, |_| true), Some(NpcId(2)));
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, SEES_CLOAKED, |_| true),
            Some(NpcId(1))
        );
        assert_eq!(next(&npcs, None, BLIND), Some(NpcId(2)));
        assert_eq!(next(&npcs, None, SEES_CLOAKED), Some(NpcId(1)));
        let mut fading = intact(3, 0.0, 0.0);
        fading.cloak = crate::cloak::Cloak {
            fade: 16.0,
            direction: 1,
        };
        assert!(targetable(&fading, false), "not cloaked yet");
    }

    #[test]
    fn the_players_cloaked_escort_stays_targetable() {
        let mut ours = escort(1, 0.0, 0.0);
        ours.cloak = cloaked(1, 0.0, 0.0).cloak;
        assert!(targetable(&ours, false));
        assert_eq!(next_escort(&[ours], None, BLIND), Some(NpcId(1)));
    }

    #[test]
    fn closest_target_passes_over_an_untargetable_ship_without_the_scanner() {
        let npcs = [untargetable(intact(1, 10.0, 0.0)), intact(2, 20.0, 0.0)];
        let any = |_: &Npc| true;
        assert_eq!(nearest(&npcs, Vec2::ZERO, BLIND, any), Some(NpcId(2)));
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, SEES_CLOAKED, any),
            Some(NpcId(2)),
            "the cloaked-ship bit is not the untargetable one"
        );
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, SEES_UNTARGETABLE, any),
            Some(NpcId(1))
        );
        assert!(
            targetable(&npcs[0], false),
            "a held target is not let go for its type"
        );
    }

    #[test]
    fn the_threat_search_passes_over_an_untargetable_threat_without_the_scanner() {
        let npcs = [
            untargetable(intact(1, 10.0, 0.0)),
            intact(2, 20.0, 0.0),
            intact(3, 30.0, 0.0),
        ];
        let threat = |npc: &Npc| npc.id != NpcId(2);
        assert_eq!(nearest(&npcs, Vec2::ZERO, BLIND, threat), Some(NpcId(3)));
        assert_eq!(
            nearest(&npcs, Vec2::ZERO, SEES_UNTARGETABLE, threat),
            Some(NpcId(1))
        );
    }

    #[test]
    fn target_select_passes_over_an_untargetable_ship_both_ways_without_the_scanner() {
        let npcs = [berthed(1, 1), untargetable(berthed(2, 2)), berthed(3, 3)];
        assert_eq!(next(&npcs, Some(NpcId(1)), BLIND), Some(NpcId(3)));
        assert_eq!(previous(&npcs, Some(NpcId(3)), BLIND), Some(NpcId(1)));
        assert_eq!(next(&npcs, Some(NpcId(1)), SEES_CLOAKED), Some(NpcId(3)));
        assert_eq!(
            next(&npcs, Some(NpcId(1)), SEES_UNTARGETABLE),
            Some(NpcId(2))
        );
        assert_eq!(
            previous(&npcs, Some(NpcId(3)), SEES_UNTARGETABLE),
            Some(NpcId(2))
        );
    }

    #[test]
    fn escort_select_passes_over_an_untargetable_escort_without_the_scanner() {
        let lone = [untargetable(berthed_escort(1, 1))];
        assert_eq!(next_escort(&lone, None, BLIND), None);
        assert_eq!(previous_escort(&lone, None, BLIND), None);
        assert_eq!(next_escort(&lone, None, SEES_UNTARGETABLE), Some(NpcId(1)));
        assert_eq!(
            previous_escort(&lone, None, SEES_UNTARGETABLE),
            Some(NpcId(1))
        );
        let mut cloaked_too = untargetable(berthed_escort(1, 1));
        cloaked_too.cloak = cloaked(1, 0.0, 0.0).cloak;
        assert_eq!(
            next_escort(&[cloaked_too.clone()], None, SEES_UNTARGETABLE),
            Some(NpcId(1)),
            "a cloaked escort needs no cloak bit"
        );
        assert_eq!(next_escort(&[cloaked_too], None, SEES_CLOAKED), None);
    }
}
