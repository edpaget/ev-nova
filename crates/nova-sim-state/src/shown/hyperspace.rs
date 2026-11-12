//! A jump's fuel ([`JUMP_FUEL`], [`max_jumps`]) and whether the ship can
//! jump ([`JumpReadiness`], [`JumpRefusal`]), from
//! `nova_sim_ai::hyperspace`, which re-exports them and keeps the star
//! map, its routes and the jump checks. The links the galaxy map draws
//! ([`system_links`], [`link_pairs`]) are here too, so the view can draw
//! the catalog's links without the star map.

use std::collections::{BTreeMap, BTreeSet};

use nova_sim_base::geometry::Vec2;
use nova_sim_catalog::catalog::{StarSystem, SystemId};

/// The fuel a jump uses (the Bible: "100 is one jump").
pub const JUMP_FUEL: f32 = 100.0;

/// How many whole jumps `fuel` holds: none for no fuel or less.
#[must_use]
pub fn max_jumps(fuel: f32) -> u32 {
    // `as` saturates a float into an integer: a negative becomes 0.
    (fuel / JUMP_FUEL).floor() as u32
}

/// Whether the ship can jump to the next system on its course, as the
/// nav area shows it (`_DrawStatusNav` @0x49f43-0x4a451 draws the
/// destination dim unless the ship is clear or a jump is under way).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpReadiness {
    /// J would be refused now, for any [`JumpRefusal`].
    Blocked,
    /// J would be accepted.
    Clear,
    /// J has been accepted: the ship is braking and turning before the
    /// jump, or is in hyperspace.
    Underway,
}

/// Why the ship cannot jump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpRefusal {
    /// The ship has landed: it must take off first. The
    /// `nova_sim::Session` gives this before `nova_sim_ai::hyperspace::check_jump` is
    /// asked.
    Landed,
    /// The ship is disabled, breaking up or destroyed. The
    /// `nova_sim::Session` gives this before `nova_sim_ai::hyperspace::check_jump` is
    /// asked. The player is never told of it: the original's J skips the
    /// jump block for a disabled ship (`_HandlePlayer` @0x6b0f5 →
    /// 0x6bada), with no message and no beep.
    Disabled,
    /// No destination has been chosen, or it has been reached.
    NoDestination,
    /// The ship is nearer the system's centre than its jump distance.
    TooClose {
        /// How far from the centre it is.
        distance: f32,
    },
    /// The ship has less fuel than [`JUMP_FUEL`].
    NoFuel {
        /// How much it has.
        fuel: f32,
    },
    /// Another ship holds it in a tractor beam (`nova_sim_ai::hyperspace::hold_jump`).
    Tractored,
}

/// `position`'s exact coordinates: a place, which two systems may share.
#[must_use]
pub fn place(position: Vec2) -> (u32, u32) {
    (position.x.to_bits(), position.y.to_bits())
}

/// Every hyperlink among the pairs (from, to) each system `listed`,
/// once, as (lower ID, higher ID), sorted: a link is drawn whichever of
/// its systems lists it, kept from the lower one, and only from a system
/// on the map (`on_map`); a link to itself is none. The one place the
/// drawn links are normalised: `nova_sim_ai::hyperspace::StarMap::links`
/// and [`system_links`] both draw by it.
#[must_use]
pub fn link_pairs(
    listed: impl IntoIterator<Item = (SystemId, SystemId)>,
    on_map: impl Fn(SystemId) -> bool,
) -> Vec<(SystemId, SystemId)> {
    let mut links = BTreeSet::new();
    for (a, b) in listed {
        for (from, to) in [(a, b), (b, a)] {
            if from < to && on_map(from) {
                links.insert((from, to));
            }
        }
    }
    links.into_iter().collect()
}

/// The hyperlinks the galaxy map draws between `systems`, the catalog's
/// map with none hidden ([`link_pairs`]): each system's links, without
/// a link to a system that is not among them, each going to the lowest
/// system at its target's place, as the original's
/// `_FindActiveCoLocatedSystem` (@0x4be0) resolves a Con slot. Where two
/// systems share an ID, the last one's place counts. It is the links of
/// `nova_sim_ai::hyperspace::StarMap::new(systems)`.
#[must_use]
pub fn system_links(systems: &[StarSystem]) -> Vec<(SystemId, SystemId)> {
    let places: BTreeMap<SystemId, (u32, u32)> = systems
        .iter()
        .map(|system| (system.id, place(system.position)))
        .collect();
    // The IDs ascend, so the first at a place is its lowest.
    let mut lowest = BTreeMap::new();
    for (&id, &at) in &places {
        lowest.entry(at).or_insert(id);
    }
    let stand_in = |to| places.get(&to).and_then(|at| lowest.get(at)).copied();
    link_pairs(
        systems.iter().flat_map(|system| {
            system
                .links
                .iter()
                .filter_map(|&to| stand_in(to))
                .map(move |to| (system.id, to))
        }),
        |id| places.contains_key(&id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_catalog::fixture::star;

    // The pinned value is exact.
    #[allow(clippy::float_cmp)]
    #[test]
    fn a_jump_uses_100_fuel() {
        assert_eq!(JUMP_FUEL, 100.0);
    }

    #[test]
    fn the_links_are_each_link_once_lower_id_first_sorted_to_the_lowest_system_at_a_place() {
        let systems = [
            star(5, (0.0, 0.0), &[1]),        // one-way, listed by the higher ID
            star(1, (10.0, 0.0), &[2, 2, 9]), // repeated, and to a missing system
            star(2, (20.0, 0.0), &[1, 2]),    // listed by both, and to itself
            star(3, (30.0, 0.0), &[7]),       // to 7, at 4's place
            star(7, (40.0, 0.0), &[]),
            star(4, (40.0, 0.0), &[]),
            star(6, (60.0, 0.0), &[]),
        ];
        let pair = |a, b| (SystemId(a), SystemId(b));
        assert_eq!(system_links(&systems), [pair(1, 2), pair(1, 5), pair(3, 4)]);
        assert_eq!(system_links(&[]), []);
    }

    #[test]
    fn the_jumps_fuel_holds_are_whole_jumps_of_it() {
        assert_eq!(max_jumps(300.0), 3);
        assert_eq!(max_jumps(199.9), 1);
        assert_eq!(max_jumps(100.0), 1);
        assert_eq!(max_jumps(99.9), 0);
        assert_eq!(max_jumps(0.0), 0);
        assert_eq!(max_jumps(-250.0), 0, "none below none");
        assert_eq!(max_jumps(f32::MAX), u32::MAX, "saturating");
    }
}
