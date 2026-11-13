//! A jump's fuel ([`JUMP_FUEL`], [`max_jumps`]) and whether the ship can
//! jump ([`JumpReadiness`], [`JumpRefusal`]), from
//! `nova_sim_ai::hyperspace`, which re-exports them and keeps the star
//! map, its routes and the jump checks. Which system stands for a place
//! ([`Places`], [`CoLocated`]) is here too, the one resolver the star map
//! and the links the galaxy map draws ([`system_links`], [`link_pairs`])
//! both read, so the view can draw the catalog's links without the star
//! map.

use std::collections::{BTreeMap, BTreeSet};

use nova_sim_base::geometry::Vec2;
use nova_sim_base::rulebook::RuleSource;
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
/// map with none hidden ([`link_pairs`]): each system's links, each
/// going to the system standing for its target ([`Places::link_target`]
/// by [`CoLocated::Lowest`], so the lowest system at its target's place,
/// and none to a system that is not among them). Where two systems share
/// an ID, the last one's place counts. It is the links of
/// `nova_sim_ai::hyperspace::StarMap::new(systems)`, which resolves its
/// links through the same [`Places`].
#[must_use]
pub fn system_links(systems: &[StarSystem]) -> Vec<(SystemId, SystemId)> {
    let places = Places::new(systems, &BTreeSet::new(), CoLocated::Lowest);
    link_pairs(
        systems.iter().flat_map(|system| {
            system
                .links
                .iter()
                .filter_map(|&to| places.link_target(to))
                .map(move |to| (system.id, to))
        }),
        |id| places.contains(id),
    )
}

/// Which visible system stands for a place two visible systems share, as
/// [`RuleKey::ColocatedResolution`](crate::RuleKey::ColocatedResolution)
/// reads (see the `nova_sim::story` module's "Visibility and
/// governments"). A hidden system stands for the lowest-ID visible system
/// at its place under both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CoLocated {
    /// The original's reading: the lowest-ID visible system at a place
    /// stands for every system there, a visible one too
    /// (`_FindActiveCoLocatedSystem` @0x4c02-0x4c40), so a link reaches
    /// only it, and a visible system it shadows claims no stellar
    /// (`_PropagateMissionBitEffects` @0x996ed-0x997bd). An arrival still
    /// enters a shadowed visible system as it is, by the engine's
    /// [`RuleKey::ArrivalSystem`](crate::RuleKey::ArrivalSystem).
    #[default]
    Lowest,
    /// The other reading, story-runtime phase 4's: a visible system stands
    /// for itself, so a link reaches every visible system at its target's
    /// place.
    Itself,
}

impl From<RuleSource> for CoLocated {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Lowest,
            RuleSource::Bible => Self::Itself,
        }
    }
}

/// Where each system of a galaxy is on the map, which of them the galaxy
/// overlay hides, and so which system stands for each: the one place the
/// original's `_FindActiveCoLocatedSystem` (@0x4be0) is read, by the ai's
/// star map (`nova_sim_ai::hyperspace::StarMap`) and the galaxy map's
/// [`system_links`] alike. A place is an exact map position; where two
/// systems share an ID, the last wins.
///
/// A hidden system stands for the lowest-ID visible system at its place
/// (none when no system there is visible). A visible one stands for the
/// same system by the original's reading, which gives the lowest-ID
/// visible one there even for a visible system (@0x4c02-0x4c40), and for
/// itself by the other: the [`CoLocated`] resolution, which differs only
/// where two systems at one place are both visible. The head of a place
/// is its lowest-ID system, visible or not, as `_LoadObjectData` builds
/// the chain (@0x783d9-0x78517).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Places {
    /// Each system's map position.
    positions: BTreeMap<SystemId, Vec2>,
    /// The systems at each place, by ascending ID.
    by_place: BTreeMap<(u32, u32), Vec<SystemId>>,
    /// The systems hidden.
    hidden: BTreeSet<SystemId>,
    /// Which visible system stands for a place two share.
    colocated: CoLocated,
}

impl Places {
    /// The places of `systems`, with those in `hidden` hidden, a place two
    /// visible systems share resolved as `colocated` says.
    #[must_use]
    pub fn new(systems: &[StarSystem], hidden: &BTreeSet<SystemId>, colocated: CoLocated) -> Self {
        let positions: BTreeMap<SystemId, Vec2> = systems
            .iter()
            .map(|system| (system.id, system.position))
            .collect();
        let mut by_place: BTreeMap<(u32, u32), Vec<SystemId>> = BTreeMap::new();
        for (&id, &position) in &positions {
            by_place.entry(place(position)).or_default().push(id);
        }
        Self {
            positions,
            by_place,
            hidden: hidden.clone(),
            colocated,
        }
    }

    /// Which visible system stands for a place two visible systems share.
    #[must_use]
    pub fn colocated(&self) -> CoLocated {
        self.colocated
    }

    /// Whether `system` is among them.
    #[must_use]
    pub fn contains(&self, system: SystemId) -> bool {
        self.positions.contains_key(&system)
    }

    /// `system`'s map position, hidden or not.
    #[must_use]
    pub fn position(&self, system: SystemId) -> Option<Vec2> {
        self.positions.get(&system).copied()
    }

    /// Whether `system` is visible: unless it is hidden.
    #[must_use]
    pub fn visible(&self, system: SystemId) -> bool {
        !self.hidden.contains(&system)
    }

    /// Whether `a` and `b` are both among them, at the same place.
    #[must_use]
    pub fn together(&self, a: SystemId, b: SystemId) -> bool {
        let at = |id| self.position(id).map(place);
        at(a).is_some() && at(a) == at(b)
    }

    /// Every system at `system`'s place, itself included, by ascending ID;
    /// none when it is not among them.
    pub fn at(&self, system: SystemId) -> impl Iterator<Item = SystemId> + '_ {
        self.position(system)
            .and_then(|position| self.by_place.get(&place(position)))
            .into_iter()
            .flatten()
            .copied()
    }

    /// The visible system that stands for `system`: the lowest-ID visible
    /// system at its place, if any, except that a visible system stands for
    /// itself under [`CoLocated::Itself`]. A visible system at no place
    /// (not among them) stands for itself.
    #[must_use]
    pub fn stand_in(&self, system: SystemId) -> Option<SystemId> {
        if self.visible(system) {
            return Some(
                match self.colocated {
                    CoLocated::Lowest => self.at(system).find(|&id| self.visible(id)),
                    CoLocated::Itself => None,
                }
                .unwrap_or(system),
            );
        }
        self.at(system).find(|&id| self.visible(id))
    }

    /// The system a link to `to` reaches: the one [standing for
    /// it](Self::stand_in), of those among them; nothing for a system
    /// they do not hold, or with none visible at its place
    /// (`_FindActiveCoLocatedSystem` @0x4be0).
    #[must_use]
    pub fn link_target(&self, to: SystemId) -> Option<SystemId> {
        self.contains(to).then(|| self.stand_in(to)).flatten()
    }

    /// The head of `system`'s place: its lowest-ID system, visible or not;
    /// none when it is not among them.
    #[must_use]
    pub fn head(&self, system: SystemId) -> Option<SystemId> {
        self.at(system).next()
    }
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
    fn the_lowest_visible_system_is_the_default_co_located_resolution() {
        assert_eq!(CoLocated::default(), CoLocated::Lowest);
        assert_eq!(CoLocated::from(RuleSource::Engine), CoLocated::Lowest);
        assert_eq!(CoLocated::from(RuleSource::Bible), CoLocated::Itself);
    }

    /// Systems 1, 2, 3 and 4: 2 and 3 share a place, 4 a pixel from it;
    /// 1 lists 2 and 3 lists 1. With 5 also at 2's place.
    fn replaced_systems() -> Vec<StarSystem> {
        vec![
            star(1, (0.0, 0.0), &[2]),
            star(2, (50.0, 50.0), &[]),
            star(3, (50.0, 50.0), &[1]),
            star(4, (50.0, 51.0), &[]),
            star(5, (50.0, 50.0), &[]),
        ]
    }

    /// [`replaced_systems`]' places, with `hidden` hidden, by the
    /// engine's resolution.
    fn places_hiding(hidden: &[i16]) -> Places {
        places_resolving(hidden, CoLocated::Lowest)
    }

    /// [`replaced_systems`]' places, with `hidden` hidden, resolved as
    /// `colocated` says.
    fn places_resolving(hidden: &[i16], colocated: CoLocated) -> Places {
        let hidden = hidden.iter().copied().map(SystemId).collect();
        Places::new(&replaced_systems(), &hidden, colocated)
    }

    #[test]
    fn by_the_engine_every_system_stands_for_the_lowest_visible_one_at_its_place() {
        let places = places_hiding(&[2]);
        let stand_in = |id| places.stand_in(SystemId(id)).map(|id| id.0);
        assert_eq!(stand_in(3), Some(3), "the lowest visible there");
        assert_eq!(stand_in(5), Some(3), "shadowed by 3, though visible");
        assert_eq!(stand_in(2), Some(3), "hidden");
        assert_eq!(stand_in(1), Some(1), "alone at its place");
        assert_eq!(stand_in(99), Some(99), "nothing hides it");
        assert!(places.visible(SystemId(5)), "still visible");
        let places = places_hiding(&[]);
        assert_eq!(places.stand_in(SystemId(5)), Some(SystemId(2)));
        let places = places_hiding(&[2, 3]);
        assert_eq!(places.stand_in(SystemId(2)), Some(SystemId(5)));
        assert_eq!(places.stand_in(SystemId(5)), Some(SystemId(5)));
        let places = places_hiding(&[2, 3, 5]);
        assert_eq!(places.stand_in(SystemId(3)), None, "none visible there");
        let places = places_hiding(&[4, 99]);
        assert_eq!(places.stand_in(SystemId(4)), None, "alone at its place");
        assert_eq!(places.stand_in(SystemId(99)), None, "hidden and nowhere");
    }

    #[test]
    fn a_visible_system_stands_for_itself_and_a_hidden_one_for_the_lowest_visible_there() {
        let places = places_resolving(&[2], CoLocated::Itself);
        let stand_in = |id| places.stand_in(SystemId(id)).map(|id| id.0);
        assert_eq!(stand_in(3), Some(3), "itself, visible");
        assert_eq!(stand_in(5), Some(5), "itself, though 3 is lower");
        assert_eq!(stand_in(2), Some(3), "the lowest visible there");
        assert_eq!(stand_in(99), Some(99), "nothing hides it");
        let places = places_resolving(&[2, 3], CoLocated::Itself);
        assert_eq!(places.stand_in(SystemId(2)), Some(SystemId(5)));
        let places = places_resolving(&[2, 3, 5], CoLocated::Itself);
        assert_eq!(places.stand_in(SystemId(3)), None, "none visible there");
        let places = places_resolving(&[4, 99], CoLocated::Itself);
        assert_eq!(places.stand_in(SystemId(4)), None, "alone at its place");
        assert_eq!(places.stand_in(SystemId(99)), None, "hidden and nowhere");
    }

    #[test]
    fn a_link_reaches_the_system_standing_for_its_target_of_those_held() {
        let places = places_hiding(&[2]);
        let target = |id| places.link_target(SystemId(id)).map(|id| id.0);
        assert_eq!(target(2), Some(3), "hidden: the lowest visible there");
        assert_eq!(target(5), Some(3), "shadowed");
        assert_eq!(target(1), Some(1));
        assert_eq!(target(99), None, "not held, though nothing hides it");
        let places = places_hiding(&[2, 3, 5]);
        assert_eq!(places.link_target(SystemId(2)), None, "none visible there");
    }

    #[test]
    fn the_head_of_a_place_is_its_lowest_system_visible_or_not() {
        let places = places_hiding(&[2, 3, 5]);
        let head = |id| places.head(SystemId(id)).map(|id| id.0);
        assert_eq!(head(5), Some(2));
        assert_eq!(head(2), Some(2));
        assert_eq!(head(1), Some(1), "alone");
        assert_eq!(head(99), None, "nowhere");
    }

    #[test]
    fn places_know_every_systems_position_hidden_or_not() {
        let places = places_hiding(&[2]);
        assert!(places.contains(SystemId(2)));
        assert!(!places.contains(SystemId(99)));
        assert_eq!(places.position(SystemId(2)), Some(Vec2::new(50.0, 50.0)));
        assert_eq!(places.position(SystemId(99)), None);
        assert!(!places.visible(SystemId(2)));
        assert!(places.visible(SystemId(3)));
        assert!(places.visible(SystemId(99)));
        assert!(places.together(SystemId(2), SystemId(5)));
        assert!(!places.together(SystemId(2), SystemId(4)), "a pixel apart");
        assert!(!places.together(SystemId(2), SystemId(99)));
        assert!(!places.together(SystemId(99), SystemId(99)), "nowhere");
        assert_eq!(places.colocated(), CoLocated::Lowest);
        assert_eq!(
            places_resolving(&[], CoLocated::Itself).colocated(),
            CoLocated::Itself
        );
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
