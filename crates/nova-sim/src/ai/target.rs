//! Whom a warship or interceptor attacks: the player, as its government's
//! relations and the player's legal record say, or an NPC.
//!
//! The rules are the original's (`_SelectWarshipTarget` @0x89d5e,
//! `_WarshipAI` @0x8b7ab and `_InterceptorAI` @0x8cb57 in the `EV Nova`
//! executable). G is the ship's government, S the system's, and L the
//! player's legal record in the system (its record with S; none in an
//! independent system).
//!
//! **Hostility to the player** ([`hostile_to_player`]), the player being
//! in the system and not breaking up:
//!
//! - `Flags` 0x0004: always; else `Flags` 0x0040: never.
//! - A xenophobic G: unless S is G and L is above none.
//! - Otherwise only while the player is within [`HUNT_REACH`] times the
//!   ship's aggression on each axis (@0x8a61a), by the legal table, with
//!   G's own `CrimeTol` (T):
//!
//!   | S | attacks when |
//!   |---|---|
//!   | G | L < -T |
//!   | an enemy of G | L > T |
//!   | allied with G | L < -1.5 T ([`ALLIED_TOLERANCE`], @0xdd640) |
//!   | neutral, or none | only with `Flags` 0x0002: L < -2 T |
//!
//! The player is never blessed (no ranks yet), carries no IFF scrambler,
//! and its ship has no combat government, so the original's 1-in-50
//! chance against a player flying an enemy's ship is not modelled.
//!
//! **The target** ([`select_target`]): these steps in order, the first
//! that gives one winning; "nearest" is by squared distance, a tie to the
//! player and then the earlier NPC, and a ship breaking up or still
//! jumping in is never one.
//!
//! 1. **Keep** the live ship it attacks or flees from.
//! 2. **A grudge** (`_SelectWarshipTarget` @0x89ebc-0x89f20): a person
//!    whose `Flags` has 0x0001 and who holds a grudge against the player
//!    (see [`person`](crate::person)) picks the player, while it is in
//!    the system and not breaking up, unless it spares it.
//! 3. **A xenophobe**: the nearest of the player (unless `Flags` 0x0040,
//!    or S is G and L is above none; always with 0x0004) and every NPC
//!    enemy not allied with it and not of its own fleet.
//! 4. **Help allies** (@0x8a418): the ship T that the first other ship of
//!    a government allied with its own (its own included) attacks or
//!    flees from, when its own friend strength times its government's
//!    `MaxOdds` is no less than T's. T may be the player, but never a
//!    ship it spares (below). This is how police come to a trader's
//!    defence.
//! 5. **The player** when it is hostile, and **every NPC enemy**: the
//!    nearest of them, each dropped when its friend strength is above the
//!    ship's own times its `MaxOdds`, and a disabled NPC skipped unless
//!    the ship has destroying weapons. The original's heaviest candidate
//!    for a ship with escorts is a placeholder: nearest always.
//! 6. **Threats**: the nearest NPC attacking, sniping at or fleeing from
//!    it, other than one it spares.
//!
//! **A target is dropped** ([`dropped`], @0x8bc73, @0x8e3ef) when it is
//! gone or breaking up; disabled while the ship has no destroying
//! weapons; or a ship it spares: an NPC allied with it (the player's
//! escorts aside, which do not exist yet; @0x8e2af) or of its own fleet.
//!
//! **A ship the player paid off** ([`Npc::spared`], see
//! [`hail`](crate::hail)) spares the player too: no step gives the
//! player, it keeps no player target, and drops one. The original makes
//! such a ship a wimpy trader that leaves, which never targets the
//! player unprovoked; this holds for any behaviour that asks here.
//!
//! Steps 4 and 6 never give a ship it spares. The original checks
//! neither there (nor does `_ExtendedIsThreatToShip` @0x81a27), so its
//! police, seeing an allied trader that a stray police shot provoked,
//! turn on their own kind until the drop at @0x8e2af clears the goal a
//! frame later, and then take it up again. Here they never do.

use crate::ai::odds::friend_strength;
use crate::ai::{Surroundings, fire};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::govt::{ALWAYS_ATTACKS_PLAYER, NEVER_ATTACKS_PLAYER, NOSY};
use crate::person::GRUDGE;
use crate::traffic::npc::Npc;

/// How far, in pixels on either axis per point of aggression, a warship
/// hunts a player its government wants.
pub const HUNT_REACH: f32 = 600.0;
/// How many times its tolerance a government's allies' warships put up
/// with.
pub const ALLIED_TOLERANCE: f32 = 1.5;
/// How many times its tolerance a nosy government's warships put up with
/// elsewhere.
pub const NOSY_TOLERANCE: f32 = 2.0;

/// Whether `npc`, a warship or interceptor, is hostile to the player
/// among `around` (see the module docs).
#[must_use]
pub fn hostile_to_player(npc: &Npc, around: &Surroundings) -> bool {
    let Some(player) = around.player.filter(|_| around.live(ShipRef::Player)) else {
        return false;
    };
    let govts = around.govts;
    let own = npc.govt;
    if govts.flag(own, ALWAYS_ATTACKS_PLAYER) {
        return true;
    }
    if govts.flag(own, NEVER_ATTACKS_PLAYER) {
        return false;
    }
    let system = around.system_govt;
    let record = f32::from(around.record);
    if govts.xenophobic(own) {
        return !(system == own && record > 0.0);
    }
    let reach = HUNT_REACH * f32::from(npc.aggression);
    let off = player.state.position - npc.state.position;
    if off.x.abs() > reach || off.y.abs() > reach {
        return false;
    }
    let tolerance = f32::from(govts.crime_tol(own));
    if system == own {
        record < -tolerance
    } else if system.is_some() && govts.enemies(own, system) {
        record > tolerance
    } else if system.is_some() && govts.allies(own, system) {
        record < -ALLIED_TOLERANCE * tolerance
    } else {
        govts.flag(own, NOSY) && record < -NOSY_TOLERANCE * tolerance
    }
}

/// Whether a xenophobic `npc` takes the player as a candidate among
/// `around`: unless `Flags` 0x0040 (and not 0x0004), or the player is in
/// good standing in the ship's own system.
fn xenophobe_hunts_player(npc: &Npc, around: &Surroundings) -> bool {
    let govts = around.govts;
    around.live(ShipRef::Player)
        && (govts.flag(npc.govt, ALWAYS_ATTACKS_PLAYER)
            || !(govts.flag(npc.govt, NEVER_ATTACKS_PLAYER)
                || (around.system_govt == npc.govt && around.record > 0)))
}

/// The nearest of `candidates` to `npc` among `around`, by squared
/// distance, a tie to the earlier.
fn nearest(
    npc: &Npc,
    candidates: impl Iterator<Item = ShipRef>,
    around: &Surroundings,
) -> Option<ShipRef> {
    let mut best: Option<(f32, ShipRef)> = None;
    for ship in candidates {
        let Some(state) = around.state_of(ship) else {
            continue;
        };
        let off = state.position - npc.state.position;
        let distance = off.x * off.x + off.y * off.y;
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, ship));
        }
    }
    best.map(|(_, ship)| ship)
}

/// The ship `npc` attacks among `around`, keeping `current` while it is
/// live (see the module docs).
#[must_use]
pub fn select_target(
    npc: &Npc,
    current: Option<ShipRef>,
    around: &Surroundings,
) -> Option<ShipRef> {
    let paid_off = |ship| ship == ShipRef::Player && npc.spared;
    if let Some(kept) = current.filter(|&ship| around.live(ship) && !paid_off(ship)) {
        return Some(kept);
    }
    let grudging = npc
        .person
        .is_some_and(|person| person.grudge && person.flags & GRUDGE != 0);
    if grudging && !npc.spared && around.live(ShipRef::Player) {
        return Some(ShipRef::Player);
    }
    let govts = around.govts;
    let me = ShipRef::Npc(npc.id);
    let others = || {
        around
            .npcs
            .iter()
            .filter(move |other| other.id != npc.id && around.live(ShipRef::Npc(other.id)))
    };
    if govts.xenophobic(npc.govt) {
        let player =
            (xenophobe_hunts_player(npc, around) && !npc.spared).then_some(ShipRef::Player);
        let enemies = others()
            .filter(|other| {
                govts.npc_enemies(npc.govt, other.govt)
                    && !govts.allies(npc.govt, other.govt)
                    && other.fleet() != npc.fleet()
            })
            .map(|other| ShipRef::Npc(other.id));
        if let Some(target) = nearest(npc, player.into_iter().chain(enemies), around) {
            return Some(target);
        }
    }
    let own = friend_strength(me, around) * govts.max_odds(npc.govt);
    let helped = others()
        .filter(|ally| {
            ally.condition == Condition::Intact
                && ally.govt.is_some()
                && govts.allies(npc.govt, ally.govt)
        })
        .find_map(|ally| {
            let foe = ally.goal.quarry()?;
            (foe != me
                && around.live(foe)
                && !spares(npc, foe, around)
                && own >= friend_strength(foe, around))
            .then_some(foe)
        });
    if helped.is_some() {
        return helped;
    }
    let player = (hostile_to_player(npc, around) && !npc.spared).then_some(ShipRef::Player);
    let destroys = fire::destroys(npc);
    let enemies = others()
        .filter(|other| {
            govts.npc_enemies(npc.govt, other.govt)
                && (other.condition == Condition::Intact || destroys)
        })
        .map(|other| ShipRef::Npc(other.id));
    let candidates = player
        .into_iter()
        .chain(enemies)
        .filter(|&ship| friend_strength(ship, around) <= own);
    if let Some(target) = nearest(npc, candidates, around) {
        return Some(target);
    }
    let threats = others()
        .filter(|other| other.threatens(me))
        .map(|other| ShipRef::Npc(other.id))
        .filter(|&ship| !spares(npc, ship, around));
    nearest(npc, threats, around)
}

/// Whether `npc` spares `ship` among `around`: the player once it has
/// paid to be spared, or an NPC allied with it or of its own fleet (see
/// the module docs).
fn spares(npc: &Npc, ship: ShipRef, around: &Surroundings) -> bool {
    match ship {
        ShipRef::Player => npc.spared,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| {
            around.govts.allies(npc.govt, other.govt) || other.fleet() == npc.fleet()
        }),
    }
}

/// Whether `npc` drops `target` among `around` (see the module docs).
#[must_use]
pub fn dropped(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
    if !around.live(target) {
        return true;
    }
    if around.condition_of(target) == Some(Condition::Disabled) && !fire::destroys(npc) {
        return true;
    }
    spares(npc, target, around)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Goal, PlayerSide};
    use crate::catalog::{GovtId, GovtRecord, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::HullSpec;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::govt::{Governments, XENOPHOBIC};
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, govt, weapon};
    use crate::traffic::npc::{Mode, NpcId};

    const ME: GovtId = GovtId(140);
    const ALLY: GovtId = GovtId(141);
    const ENEMY: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);
    const RIVAL: GovtId = GovtId(145);

    /// The ship's government 140 (class 1, `CrimeTol` 6, `MaxOdds` 100,
    /// with `flags`), its ally 141, its enemies 142 and 145 (not allied
    /// with each other), a neutral 143 and a xenophobe 144 allied with
    /// it.
    fn govts(flags: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                allies: [1, -1, -1, -1],
                ..govt(144)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [5, -1, -1, -1],
                ..govt(145)
            },
        ])
    }

    /// NPC `id` of `govt` at (`x`, `y`), of strength 100, aggression 2.
    fn ship(id: u32, govt: GovtId, x: f32, y: f32) -> Npc {
        let mut npc = crate::testkit::npc(id, ShipStats::new(FAST, &[]));
        npc.govt = Some(govt);
        npc.state.position = Vec2::new(x, y);
        npc.hull.strength = 100.0;
        npc.aggression = 2;
        npc
    }

    /// The player at (`x`, `y`), of strength 100.
    fn player(x: f32, y: f32) -> PlayerSide {
        PlayerSide {
            state: ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            },
            condition: Condition::Intact,
            reserves: Reserves::full(100.0, 100.0, 100.0),
            hull: HullSpec {
                strength: 100.0,
                ..HullSpec::default()
            },
            handling: ShipStats::new(FAST, &[]).handling,
        }
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    const P: ShipRef = ShipRef::Player;

    /// Whether ship 1 of govt 140 (of `flags`) at the centre is hostile
    /// to the player at (`x`, `y`) with record `record` in a system of
    /// `system`.
    fn hostile(flags: u16, system: Option<GovtId>, record: i16, (x, y): (f32, f32)) -> bool {
        let govts = govts(flags);
        let npcs = [ship(1, ME, 0.0, 0.0)];
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &npcs)
        };
        hostile_to_player(&npcs[0], &around)
    }

    const NEAR: (f32, f32) = (0.0, -100.0);

    #[test]
    fn at_home_a_warship_attacks_a_record_below_minus_its_tolerance() {
        assert!(!hostile(0, Some(ME), -6, NEAR));
        assert!(hostile(0, Some(ME), -7, NEAR));
        assert!(!hostile(0, Some(ME), 30, NEAR));
    }

    #[test]
    fn in_an_enemys_system_a_warship_attacks_a_record_above_its_tolerance() {
        assert!(!hostile(0, Some(ENEMY), 6, NEAR));
        assert!(hostile(0, Some(ENEMY), 7, NEAR));
        assert!(!hostile(0, Some(ENEMY), -30, NEAR));
    }

    #[test]
    fn in_an_allys_system_a_warship_attacks_below_one_and_a_half_times_its_tolerance() {
        assert!(!hostile(0, Some(ALLY), -9, NEAR));
        assert!(hostile(0, Some(ALLY), -10, NEAR));
        assert_eq!(ALLIED_TOLERANCE, 1.5);
    }

    #[test]
    fn elsewhere_only_a_nosy_warship_attacks_below_twice_its_tolerance() {
        for system in [Some(NEUTRAL), None] {
            assert!(!hostile(0, system, -1000, NEAR), "{system:?}");
            assert!(!hostile(NOSY, system, -12, NEAR), "{system:?}");
            assert!(hostile(NOSY, system, -13, NEAR), "{system:?}");
        }
        assert_eq!(NOSY_TOLERANCE, 2.0);
    }

    #[test]
    fn a_warship_hunts_a_wanted_player_only_within_600_per_point_of_aggression() {
        // Aggression 2: 1200 on either axis.
        assert!(hostile(0, Some(ME), -7, (1200.0, -1200.0)));
        assert!(!hostile(0, Some(ME), -7, (1200.5, 0.0)));
        assert!(!hostile(0, Some(ME), -7, (0.0, -1200.5)));
        let govts = govts(0);
        let mut npcs = [ship(1, ME, 0.0, 0.0)];
        let at = |npcs: &[Npc], x: f32| {
            let around = Surroundings {
                player: Some(player(x, 0.0)),
                govts: &govts,
                system_govt: Some(ME),
                record: -7,
                ..Surroundings::new(&[], npcs)
            };
            hostile_to_player(&npcs[0], &around)
        };
        npcs[0].aggression = 3;
        assert!(at(&npcs, 1800.0));
        assert!(!at(&npcs, 1800.5));
        npcs[0].aggression = 0;
        assert!(!at(&npcs, 1.0), "aggression 0 never hunts");
        assert_eq!(HUNT_REACH, 600.0);
    }

    #[test]
    fn the_hunt_is_measured_from_where_the_warship_is() {
        let govts = govts(0);
        let npcs = [ship(1, ME, 1000.0, -1000.0)];
        let at = |x: f32, y: f32| {
            let around = Surroundings {
                player: Some(player(1000.0 + x, -1000.0 + y)),
                govts: &govts,
                system_govt: Some(ME),
                record: -7,
                ..Surroundings::new(&[], &npcs)
            };
            hostile_to_player(&npcs[0], &around)
        };
        assert!(at(1200.0, -1200.0));
        assert!(!at(1200.5, 0.0));
        assert!(!at(0.0, 1200.5));
    }

    /// A xenophobic ship (NPC 1, govt 144 with `flags` besides) choosing
    /// between the player 100 above and a rival 300 to the right, with
    /// record `record` in a system of `system`.
    fn xenophobe_choice(flags: u16, system: Option<GovtId>, record: i16) -> Option<ShipRef> {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | flags,
                ..govt(144)
            },
            govt(145),
        ]);
        let npcs = [ship(1, XENO, 0.0, 0.0), ship(2, RIVAL, 300.0, 0.0)];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &npcs)
        };
        select_target(&npcs[0], None, &around)
    }

    #[test]
    fn a_xenophobe_hunts_the_player_unless_told_not_to_or_at_home_in_good_standing() {
        assert_eq!(xenophobe_choice(0, Some(ME), 1000), Some(P), "elsewhere");
        assert_eq!(
            xenophobe_choice(0, Some(XENO), 0),
            Some(P),
            "at home, no better"
        );
        assert_eq!(xenophobe_choice(0, Some(XENO), -5), Some(P));
        assert_eq!(
            xenophobe_choice(0, Some(XENO), 1),
            Some(n(2)),
            "at home, in good standing"
        );
        assert_eq!(
            xenophobe_choice(NEVER_ATTACKS_PLAYER, Some(ME), 0),
            Some(n(2)),
            "never the player"
        );
        assert_eq!(
            xenophobe_choice(NEVER_ATTACKS_PLAYER | ALWAYS_ATTACKS_PLAYER, Some(XENO), 1),
            Some(P),
            "always wins"
        );
    }

    #[test]
    fn a_spared_ship_never_takes_the_player_but_still_its_npc_enemies() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | ALWAYS_ATTACKS_PLAYER,
                crime_tol: 6,
                ..govt(144)
            },
            govt(145),
        ]);
        let mut spared = ship(1, XENO, 0.0, 0.0);
        spared.spared = true;
        let rival = ship(2, RIVAL, 300.0, 0.0);
        let choose = |npcs: &[Npc], current| {
            let around = Surroundings {
                player: Some(player(0.0, -100.0)),
                govts: &govts,
                system_govt: Some(XENO),
                record: -100,
                ..Surroundings::new(&[], npcs)
            };
            select_target(&npcs[0], current, &around)
        };
        let both = [spared.clone(), rival.clone()];
        assert_eq!(choose(&both, Some(P)), Some(n(2)), "the player not kept");
        assert_eq!(choose(&both, None), Some(n(2)), "an NPC enemy still");
        assert_eq!(choose(&[spared.clone()], Some(P)), None, "never the player");
        let unspared = [ship(1, XENO, 0.0, 0.0), rival];
        assert_eq!(choose(&unspared, None), Some(P), "as before");
        assert_eq!(choose(&unspared[..1], Some(P)), Some(P));
    }

    #[test]
    fn a_spared_warship_neither_hunts_nor_helps_against_the_player_and_drops_it() {
        let govts = govts(ALWAYS_ATTACKS_PLAYER);
        let mut spared = ship(1, ME, 0.0, 0.0);
        spared.spared = true;
        let mut kin = ship(2, ME, 50.0, 0.0);
        kin.goal = Goal::Attack(P);
        let npcs = [spared.clone(), kin];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: Some(ME),
            record: -100,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select_target(&npcs[0], None, &around), None);
        assert!(dropped(&npcs[0], P, &around));
        let mut unspared = npcs.clone();
        unspared[0].spared = false;
        let around = Surroundings {
            npcs: &unspared,
            ..around
        };
        assert_eq!(select_target(&unspared[0], None, &around), Some(P));
        assert!(!dropped(&unspared[0], P, &around));
    }

    #[test]
    fn a_xenophobe_never_hunts_a_player_breaking_up() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | ALWAYS_ATTACKS_PLAYER,
                ..govt(144)
            },
            govt(145),
        ]);
        let npcs = [ship(1, XENO, 0.0, 0.0), ship(2, RIVAL, 300.0, 0.0)];
        let around = Surroundings {
            player: Some(PlayerSide {
                condition: Condition::Dying { ticks_left: 3 },
                ..player(0.0, -100.0)
            }),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select_target(&npcs[0], None, &around), Some(n(2)));
    }

    #[test]
    fn max_odds_scale_the_ships_strength_against_its_candidates() {
        // MaxOdds 200: a ship of 100 takes on up to 200.
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                max_odds: 200,
                ..govt(140)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..govt(142)
            },
        ]);
        let mut brute = ship(2, ENEMY, 100.0, 0.0);
        brute.hull.strength = 200.0;
        let npcs = [me(), brute];
        let around = Surroundings {
            player: Some(player(0.0, -5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select_target(&npcs[0], None, &around), Some(n(2)));
        let mut stronger = npcs.clone();
        stronger[1].hull.strength = 200.5;
        let around = Surroundings {
            player: Some(player(0.0, -5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &stronger)
        };
        assert_eq!(select_target(&stronger[0], None, &around), None);
    }

    #[test]
    fn flags_0x0004_always_attack_the_player_and_0x0040_never() {
        let far = (5000.0, 5000.0);
        assert!(hostile(ALWAYS_ATTACKS_PLAYER, Some(ME), 1000, far));
        assert!(!hostile(NEVER_ATTACKS_PLAYER, Some(ME), -1000, NEAR));
        assert!(!hostile(
            NEVER_ATTACKS_PLAYER | XENOPHOBIC,
            Some(ENEMY),
            0,
            NEAR
        ));
    }

    #[test]
    fn a_xenophobe_spares_only_a_player_in_good_standing_at_home() {
        let far = (5000.0, 5000.0);
        assert!(!hostile(XENOPHOBIC, Some(ME), 1, far));
        assert!(hostile(XENOPHOBIC, Some(ME), 0, far));
        assert!(hostile(XENOPHOBIC, Some(ENEMY), 1000, far));
        assert!(hostile(XENOPHOBIC, None, 1000, far));
    }

    #[test]
    fn no_player_or_one_breaking_up_is_never_attacked() {
        let govts = govts(ALWAYS_ATTACKS_PLAYER);
        let npcs = [ship(1, ME, 0.0, 0.0)];
        let mut around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert!(!hostile_to_player(&npcs[0], &around));
        around.player = Some(PlayerSide {
            condition: Condition::Dying { ticks_left: 2 },
            ..player(0.0, -100.0)
        });
        assert!(!hostile_to_player(&npcs[0], &around));
        around.player = Some(PlayerSide {
            condition: Condition::Disabled,
            ..player(0.0, -100.0)
        });
        assert!(
            hostile_to_player(&npcs[0], &around),
            "disabled is no excuse"
        );
    }

    /// The ship (NPC 1 at the centre) chooses among `npcs` (it first) and
    /// the player at (`x`, `y`), wanted at home or not as `record` says.
    fn chosen(
        flags: u16,
        npcs: &[Npc],
        current: Option<ShipRef>,
        record: i16,
        (x, y): (f32, f32),
    ) -> Option<ShipRef> {
        let govts = govts(flags);
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            system_govt: Some(ME),
            record,
            ..Surroundings::new(&[], npcs)
        };
        select_target(&npcs[0], current, &around)
    }

    fn me() -> Npc {
        ship(1, ME, 0.0, 0.0)
    }

    #[test]
    fn a_live_target_is_kept() {
        let npcs = [
            me(),
            ship(2, ENEMY, 10.0, 0.0),
            ship(3, NEUTRAL, 500.0, 0.0),
        ];
        assert_eq!(chosen(0, &npcs, Some(n(3)), 0, NEAR), Some(n(3)));
        assert_eq!(chosen(0, &npcs, Some(n(9)), 0, NEAR), Some(n(2)), "gone");
        let mut dying = npcs.clone();
        dying[2].condition = Condition::Dying { ticks_left: 3 };
        assert_eq!(chosen(0, &dying, Some(n(3)), 0, NEAR), Some(n(2)));
    }

    #[test]
    fn a_xenophobe_takes_the_nearest_of_the_player_and_its_npc_enemies() {
        let npcs = [
            me(),
            ship(2, ALLY, 50.0, 0.0),
            ship(3, ENEMY, 150.0, 0.0),
            ship(4, XENO, 20.0, 0.0),
        ];
        // The xenophobe is NPC 1 here: govt 144, at war with everyone it
        // is not allied with, and allied with 140 (NPC 4).
        let mut xeno = npcs.clone();
        xeno[0].govt = Some(XENO);
        xeno[3].govt = Some(ME);
        assert_eq!(
            chosen(0, &xeno, None, 0, (0.0, -100.0)),
            Some(n(2)),
            "the nearest enemy"
        );
        assert_eq!(
            chosen(0, &xeno, None, 0, (0.0, -10.0)),
            Some(P),
            "the player nearer"
        );
        let mut fleet = xeno.clone();
        fleet[1].leader = Some(NpcId(1));
        assert_eq!(
            chosen(0, &fleet, None, 0, (0.0, -1000.0)),
            Some(n(3)),
            "not its own fleet"
        );
    }

    #[test]
    fn a_ship_comes_to_the_help_of_its_first_ally_in_a_fight() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Flee(n(4));
        let mut kin = ship(3, ME, 2000.0, 0.0);
        kin.goal = Goal::Attack(P);
        let pirate = ship(4, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), trader, kin, pirate];
        assert_eq!(
            chosen(0, &npcs, None, 0, (0.0, -100.0)),
            Some(n(4)),
            "the first"
        );
        let mut later = npcs.clone();
        later[1].goal = Goal::Idle;
        assert_eq!(
            chosen(0, &later, None, 0, (0.0, -100.0)),
            Some(P),
            "its own kind"
        );
    }

    #[test]
    fn a_ship_helps_only_with_its_strength_times_max_odds_no_less_than_the_foes() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Attack(n(3));
        trader.hull.strength = 0.0;
        let mut pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        // The ship and its ally the trader: 100 and none.
        pirate.hull.strength = 100.0;
        let npcs = [me(), trader, pirate.clone()];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)), "even");
        let mut stronger = npcs.clone();
        stronger[2].hull.strength = 100.5;
        assert_eq!(chosen(0, &stronger, None, 0, NEAR), None, "too strong");
    }

    #[test]
    fn a_ship_never_helps_an_independent_or_a_downed_ally() {
        let mut independent = ship(2, ALLY, 1000.0, 0.0);
        independent.govt = None;
        independent.goal = Goal::Attack(n(3));
        let pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), independent, pirate];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), None);
        let mut disabled = npcs.clone();
        disabled[1].govt = Some(ALLY);
        disabled[1].condition = Condition::Disabled;
        assert_eq!(chosen(0, &disabled, None, 0, NEAR), None);
        let mut arriving = disabled.clone();
        arriving[1].condition = Condition::Intact;
        arriving[1].mode = Mode::JumpingIn { ticks_left: 3 };
        assert_eq!(chosen(0, &arriving, None, 0, NEAR), None);
        let mut gone_foe = arriving.clone();
        gone_foe[1].mode = Mode::Flying;
        gone_foe[1].goal = Goal::Attack(n(9));
        assert_eq!(chosen(0, &gone_foe, None, 0, NEAR), None);
        let mut at_me = gone_foe;
        at_me[1].goal = Goal::Attack(n(1));
        assert_eq!(
            chosen(0, &at_me, None, 0, NEAR),
            None,
            "no help against itself, and an ally is no threat"
        );
    }

    #[test]
    fn a_ship_never_helps_an_ally_against_its_own_kind_an_ally_or_its_fleet() {
        // NPC 2, an allied trader, flees from or fights NPC 3; NPC 4, a
        // later ally, fights the pirate NPC 5.
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Flee(n(3));
        let foe = ship(3, ME, 2000.0, 0.0);
        let mut later = ship(4, ALLY, 1500.0, 0.0);
        later.goal = Goal::Attack(n(5));
        let pirate = ship(5, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), trader, foe, later, pirate];
        assert_eq!(
            chosen(0, &npcs, None, 0, NEAR),
            Some(n(5)),
            "not its own kind, but the next ally's foe"
        );
        let mut fought = npcs.clone();
        fought[1].goal = Goal::Attack(n(3));
        assert_eq!(chosen(0, &fought, None, 0, NEAR), Some(n(5)), "fought");
        let mut allied = npcs.clone();
        allied[1].govt = Some(ME);
        allied[2].govt = Some(ALLY);
        assert_eq!(chosen(0, &allied, None, 0, NEAR), Some(n(5)), "an ally");
        let mut escort = npcs.clone();
        escort[2].govt = Some(NEUTRAL);
        escort[2].leader = Some(NpcId(1));
        assert_eq!(chosen(0, &escort, None, 0, NEAR), Some(n(5)), "its fleet");
        let mut alone = npcs.clone();
        alone[3].goal = Goal::Idle;
        assert_eq!(chosen(0, &alone, None, 0, NEAR), None, "nothing else");
    }

    #[test]
    fn an_ally_or_its_fleet_fighting_the_ship_is_no_threat_to_turn_on() {
        let mut ally = ship(2, ALLY, 10.0, 0.0);
        ally.goal = Goal::Attack(n(1));
        let mut kin = ship(3, ME, 20.0, 0.0);
        kin.goal = Goal::Flee(n(1));
        let mut escort = ship(4, NEUTRAL, 30.0, 0.0);
        escort.leader = Some(NpcId(1));
        escort.goal = Goal::Snipe(n(1));
        let mut stranger = ship(5, NEUTRAL, 900.0, 0.0);
        stranger.goal = Goal::Attack(n(1));
        let npcs = [me(), ally, kin, escort, stranger];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(5)));
        assert_eq!(chosen(0, &npcs[..4], None, 0, NEAR), None);
    }

    #[test]
    fn the_nearest_of_a_wanted_player_and_the_npc_enemies_is_attacked() {
        let npcs = [me(), ship(2, ENEMY, 300.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        assert_eq!(
            chosen(0, &npcs, None, 0, NEAR),
            Some(n(3)),
            "the player is clean"
        );
        assert_eq!(
            chosen(0, &npcs, None, -7, NEAR),
            Some(P),
            "wanted, and nearest"
        );
        assert_eq!(
            chosen(0, &npcs, None, -7, (0.0, -200.0)),
            Some(P),
            "a tie: the player"
        );
        let tie = [me(), ship(2, ENEMY, 200.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        assert_eq!(chosen(0, &tie, None, 0, NEAR), Some(n(2)), "the earlier");
    }

    #[test]
    fn a_candidate_stronger_than_the_ship_times_its_odds_is_dropped() {
        let mut brute = ship(2, ENEMY, 100.0, 0.0);
        brute.hull.strength = 100.0;
        let npcs = [me(), brute, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(2)), "even");
        let mut stronger = npcs.clone();
        stronger[1].hull.strength = 100.5;
        assert_eq!(chosen(0, &stronger, None, 0, NEAR), Some(n(3)));
    }

    #[test]
    fn a_disabled_enemy_is_skipped_without_destroying_weapons() {
        let mut wreck = ship(2, ENEMY, 100.0, 0.0);
        wreck.condition = Condition::Disabled;
        let npcs = [me(), wreck, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        let mut armed = npcs.clone();
        armed[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(chosen(0, &armed, None, 0, NEAR), Some(n(2)));
    }

    #[test]
    fn the_dying_and_the_arriving_are_never_candidates() {
        let mut dying = ship(2, ENEMY, 100.0, 0.0);
        dying.condition = Condition::Dying { ticks_left: 3 };
        let mut arriving = ship(3, ENEMY, 150.0, 0.0);
        arriving.mode = Mode::JumpingIn { ticks_left: 3 };
        let npcs = [me(), dying, arriving, ship(4, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(4)));
        assert_eq!(chosen(0, &npcs[..3], None, 0, NEAR), None);
    }

    #[test]
    fn with_nothing_else_a_ship_turns_on_the_nearest_threat() {
        let mut far = ship(2, NEUTRAL, 900.0, 0.0);
        far.goal = Goal::Flee(n(1));
        let mut near = ship(3, NEUTRAL, 300.0, 0.0);
        near.goal = Goal::Attack(n(1));
        let mut inspecting = ship(4, NEUTRAL, 10.0, 0.0);
        inspecting.goal = Goal::Inspect(n(1));
        let npcs = [me(), far, near, inspecting];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        assert_eq!(chosen(0, &npcs[..1], None, 0, NEAR), None);
        let mut idle = npcs.clone();
        idle[1].goal = Goal::Idle;
        idle[2].goal = Goal::Idle;
        assert_eq!(chosen(0, &idle, None, 0, NEAR), None);
    }

    #[test]
    fn a_target_is_dropped_once_gone_downed_harmlessly_allied_or_of_the_fleet() {
        fn around<'a>(npcs: &'a [Npc], govts: &'a Governments) -> Surroundings<'a> {
            Surroundings {
                player: Some(player(0.0, -100.0)),
                govts,
                ..Surroundings::new(&[], npcs)
            }
        }
        let govts = govts(0);
        let mut wreck = ship(2, ENEMY, 100.0, 0.0);
        wreck.condition = Condition::Disabled;
        let mut dying = ship(3, ENEMY, 100.0, 0.0);
        dying.condition = Condition::Dying { ticks_left: 3 };
        let ally = ship(4, ALLY, 100.0, 0.0);
        let kin = ship(5, ME, 100.0, 0.0);
        let mut escort = ship(6, NEUTRAL, 100.0, 0.0);
        escort.leader = Some(NpcId(1));
        let enemy = ship(7, ENEMY, 100.0, 0.0);
        let mut npcs = vec![me(), wreck, dying, ally, kin, escort, enemy];
        for (target, drop) in [
            (n(2), true),
            (n(3), true),
            (n(4), true),
            (n(5), true),
            (n(6), true),
            (n(7), false),
            (n(9), true),
            (P, false),
        ] {
            assert_eq!(
                dropped(&npcs[0], target, &around(&npcs, &govts)),
                drop,
                "{target:?}"
            );
        }
        npcs[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert!(
            !dropped(&npcs[0], n(2), &around(&npcs, &govts)),
            "it can finish it off"
        );
    }

    // Persons.

    /// The target of ship 1 of govt 140, a person of `flags` with a
    /// grudge or not, spared or not, keeping `current`, with an enemy (NPC
    /// 2) 50 below it, and the player far off with a clean record at home,
    /// when it is in the system.
    fn grudge_target(
        flags: u16,
        grudge: bool,
        spared: bool,
        current: Option<ShipRef>,
        player_here: bool,
    ) -> Option<ShipRef> {
        let govts = govts(0);
        let mut grudger = ship(1, ME, 0.0, 0.0);
        grudger.spared = spared;
        grudger.person = Some(crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(510),
            flags,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge,
            quoted: false,
            quoted_at: None,
        });
        let npcs = [grudger, ship(2, ENEMY, 0.0, 50.0)];
        let around = Surroundings {
            player: player_here.then(|| player(0.0, -1000.0)),
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        select_target(&npcs[0], current, &around)
    }

    #[test]
    fn a_person_holding_a_grudge_picks_the_player_before_any_other() {
        let grudge = crate::person::GRUDGE;
        assert_eq!(grudge_target(grudge, true, false, None, true), Some(P));
        assert_eq!(
            grudge_target(grudge, false, false, None, true),
            Some(n(2)),
            "no grudge"
        );
        assert_eq!(
            grudge_target(0, true, false, None, true),
            Some(n(2)),
            "no 0x0001"
        );
        assert_eq!(
            grudge_target(grudge, true, true, None, true),
            Some(n(2)),
            "spared"
        );
        assert_eq!(
            grudge_target(grudge, true, false, Some(n(2)), true),
            Some(n(2)),
            "it keeps the ship it fights"
        );
        assert_eq!(
            grudge_target(grudge, true, false, None, false),
            Some(n(2)),
            "no player in the system"
        );
    }
}
