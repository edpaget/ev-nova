//! The candidate pass `_SelectWarshipTarget` runs in its xenophobe step and
//! again in its hunt (steps 5 and 7, see [`target`](crate::ai::target)):
//! the strength filter, the heaviest class and the nearest pick.
//!
//! The rules are the original's (`_SelectWarshipTarget` @0x8a2a8-0x8a3f8
//! and @0x8aaf2-0x8ad1f in the `EV Nova` executable).
//!
//! - **The strength filter** (@0x8a34d-0x8a3a8, @0x8ab96-0x8ac22): a
//!   candidate whose friend strength ([`friend_strength`]) is above the
//!   ship's own times its government's `MaxOdds` is dropped.
//! - **The heaviest** (@0x8ac24-0x8ad07, the hunt's alone): a ship with
//!   escorts ([`has_escorts`]) takes only the candidates of the largest
//!   class `Mass` (+0x9ee, [`HullSpec::mass`](crate::combat::hull::HullSpec::mass))
//!   among those left.
//! - **The nearest** (@0x8a3c7-0x8a3f8, @0x8acab-0x8ad1f): by
//!   [`distance`], strictly nearer replacing, so a tie goes to the earlier
//!   slot, the player's (0) first, then the NPCs in slot order
//!   ([`Surroundings::by_slot`]).
//!
//! **Has escorts** is the ship's +0xc0, which `_DoPlayGameWork` sets each
//! frame on the leader of every active ship of AI type above 4 in the
//! system (@0x44064-0x44129). Every ship the original gives a leader is
//! of such a type (a fleet's escorts are AI type 6, `_HyperSpawnFleet`
//! @0x421dc/0x421ef; fighters 5), but the engine's fleet escorts still fly
//! their own type (rdm:phase/escort-fleet-fidelity/phase-3-npc-led-escorts),
//! so it is read here as any ship in the system whose leader is the ship.

use crate::ai::Surroundings;
use crate::ai::odds::friend_strength;
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// The distance the original's nearest pick compares, squared: each axis's
/// offset truncated toward zero, then the squares added
/// (`cvttss2si` @0x8a302, @0x8a324; @0x8ab4b, @0x8ab6d). The original's
/// sum is 32 bits; here it isn't wrapped.
#[must_use]
pub fn distance(from: Vec2, to: Vec2) -> i64 {
    // The offsets of two ships in a system fit an i64 many times over.
    #[allow(clippy::cast_possible_truncation)]
    let axis = |offset: f32| offset.trunc() as i64;
    let (x, y) = (axis(to.x - from.x), axis(to.y - from.y));
    x * x + y * y
}

/// Whether `npc` has escorts among `around`: some ship there whose leader
/// is `npc` (see the module docs).
#[must_use]
pub fn has_escorts(npc: &Npc, around: &Surroundings) -> bool {
    around.npcs.iter().any(|other| other.leader == Some(npc.id))
}

/// The candidates of `candidates`, in slot order, whose friend strength
/// isn't above `npc`'s own times its government's `MaxOdds` among
/// `around`, a shieldless friend counting as `carry` reads
/// [`RuleKey::FriendShieldCarry`](crate::rulebook::RuleKey::FriendShieldCarry).
#[must_use]
pub fn within_odds(
    npc: &Npc,
    candidates: impl IntoIterator<Item = ShipRef>,
    around: &Surroundings,
    carry: RuleSource,
) -> Vec<ShipRef> {
    let own = friend_strength(ShipRef::Npc(npc.id), around, carry);
    let reach = own * around.govts.max_odds(npc.govt);
    candidates
        .into_iter()
        .filter(|&ship| friend_strength(ship, around, carry) <= reach)
        .collect()
}

/// The nearest to `npc` of `candidates`, in slot order, among `around`,
/// only those of the heaviest class among them when `heaviest` (see the
/// module docs).
#[must_use]
pub fn nearest(
    npc: &Npc,
    candidates: &[ShipRef],
    around: &Surroundings,
    heaviest: bool,
) -> Option<ShipRef> {
    let mass_of = |ship: ShipRef| match ship {
        ShipRef::Player => around.player.map(|player| player.hull.mass),
        ShipRef::Npc(id) => around.npc(id).map(|other| other.hull.mass),
    };
    let heaviest = heaviest
        .then(|| {
            candidates
                .iter()
                .filter_map(|&ship| mass_of(ship))
                .reduce(f32::max)
        })
        .flatten();
    let mut best: Option<(i64, ShipRef)> = None;
    for &ship in candidates {
        let Some(state) = around.state_of(ship) else {
            continue;
        };
        if heaviest.is_some_and(|mass| mass_of(ship) != Some(mass)) {
            continue;
        }
        let distance = distance(npc.state.position, state.position);
        if best.is_none_or(|(nearest, _)| distance < nearest) {
            best = Some((distance, ship));
        }
    }
    best.map(|(_, ship)| ship)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::traffic::npc::{AiType, NpcId};

    const P: ShipRef = ShipRef::Player;

    #[test]
    fn the_distance_truncates_each_axis_toward_zero_before_squaring() {
        let at = |x: f32, y: f32| Vec2::new(x, y);
        assert_eq!(distance(at(0.0, 0.0), at(3.9, -4.9)), 9 + 16);
        assert_eq!(distance(at(0.5, 0.5), at(-0.4, 0.4)), 0);
        assert_eq!(distance(at(10.0, 0.0), at(0.0, 0.0)), 100);
        assert_eq!(
            distance(at(5.0, 7.0), at(3.0, 4.0)),
            4 + 9,
            "the offset, not the sum"
        );
        assert_eq!(
            distance(at(0.0, 0.0), at(50_000.0, 50_000.0)),
            5_000_000_000,
            "no 32-bit wrap"
        );
    }

    /// A trader (NPC 1) at the centre and `others`, the player at
    /// (`x`, `y`).
    fn picked(
        others: &[Npc],
        at: (f32, f32),
        candidates: &[ShipRef],
        heaviest: bool,
    ) -> Option<ShipRef> {
        let mut npcs = vec![ship(1, TRADERS, AiType::Warship, 0.0, 0.0)];
        npcs.extend_from_slice(others);
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, at, 0);
        nearest(&npcs[0], candidates, &around, heaviest)
    }

    #[test]
    fn the_nearest_by_whole_pixels_wins_and_a_tie_goes_to_the_earlier() {
        let near = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        let far = ship(3, PIRATES, AiType::Warship, 0.0, 100.9);
        assert_eq!(
            picked(
                &[far.clone(), near.clone()],
                (0.0, -5000.0),
                &[n(3), n(2)],
                false
            ),
            Some(n(3)),
            "100.9 is 100 in whole pixels: a tie, to the earlier"
        );
        assert_eq!(
            picked(&[far, near], (0.0, -5000.0), &[n(2), n(3)], false),
            Some(n(2))
        );
        let pirate = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        assert_eq!(
            picked(&[pirate], (0.0, -100.5), &[P, n(2)], false),
            Some(P),
            "the player first"
        );
        assert_eq!(picked(&[], (0.0, -100.5), &[n(9)], false), None, "gone");
    }

    #[test]
    fn with_heaviest_only_the_heaviest_class_is_picked() {
        let mut light = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        light.hull.mass = 50.0;
        let mut heavy = ship(3, PIRATES, AiType::Warship, 900.0, 0.0);
        heavy.hull.mass = 400.0;
        let mut heavy_far = heavy.clone();
        heavy_far.id = NpcId(4);
        heavy_far.state.position = Vec2::new(950.0, 0.0);
        let others = [light, heavy, heavy_far];
        let all = [n(2), n(3), n(4)];
        assert_eq!(picked(&others, (0.0, -5000.0), &all, false), Some(n(2)));
        assert_eq!(
            picked(&others, (0.0, -5000.0), &all, true),
            Some(n(3)),
            "the nearest of the heaviest"
        );
        assert_eq!(
            picked(&others, (0.0, -5000.0), &[n(2)], true),
            Some(n(2)),
            "among the candidates only"
        );
    }

    #[test]
    fn the_player_is_weighed_by_its_class_mass() {
        let weighed = |npc_mass: f32| {
            let mut light = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
            light.hull.mass = npc_mass;
            let npcs = [ship(1, TRADERS, AiType::Warship, 0.0, 0.0), light];
            let govts = govts(0, 0);
            let sites = sites();
            let mut around = around(&sites, &npcs, &govts, (0.0, -3000.0), 0);
            let mut player = around.player.expect("there");
            player.hull.mass = 60.0;
            around.player = Some(player);
            nearest(&npcs[0], &[P, n(2)], &around, true)
        };
        assert_eq!(weighed(50.0), Some(P), "the player is heavier");
        assert_eq!(weighed(60.0), Some(n(2)), "as heavy, and nearer");
    }

    #[test]
    fn a_ship_has_escorts_when_one_names_it_its_leader() {
        let mut escort = ship(2, TRADERS, AiType::Warship, 100.0, 0.0);
        let npcs = [ship(1, TRADERS, AiType::Warship, 0.0, 0.0), escort.clone()];
        let govts = govts(0, 0);
        let sites = sites();
        assert!(!has_escorts(
            &npcs[0],
            &around(&sites, &npcs, &govts, (0.0, 0.0), 0)
        ));
        escort.leader = Some(NpcId(1));
        let npcs = [npcs[0].clone(), escort];
        assert!(has_escorts(
            &npcs[0],
            &around(&sites, &npcs, &govts, (0.0, 0.0), 0)
        ));
        assert!(!has_escorts(
            &npcs[1],
            &around(&sites, &npcs, &govts, (0.0, 0.0), 0)
        ));
    }

    #[test]
    fn a_candidate_stronger_than_the_ship_times_its_max_odds_is_left_out() {
        // MaxOdds 100 by the fixture's governments: as strong as the ship
        // at most.
        let mut even = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        even.hull.strength = 100.0;
        let mut stronger = ship(3, NEUTRAL, AiType::Warship, 100.0, 0.0);
        stronger.hull.strength = 101.0;
        let npcs = [ship(1, TRADERS, AiType::Warship, 0.0, 0.0), even, stronger];
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            within_odds(&npcs[0], [P, n(2), n(3)], &around, RuleSource::Engine),
            vec![P, n(2)]
        );
        let mut backed = npcs.to_vec();
        backed.push(ship(4, POLICE, AiType::Warship, 50.0, 0.0));
        let around = Surroundings {
            npcs: &backed,
            ..around
        };
        assert_eq!(
            within_odds(&backed[0], [n(3)], &around, RuleSource::Engine),
            vec![n(3)],
            "its ally counts in its strength"
        );
    }
}
