//! Hyperspace: routes along the hyperlinks between systems, whether the
//! ship can jump, and where it arrives.
//!
//! - The [`StarMap`] is every system's map position and hyperlinks. A link
//!   joins its two systems both ways, whichever of them lists it (as the
//!   galaxy map draws it); a system's link to itself, or to a system that
//!   does not exist, is no link. [`StarMap::route`] is the fewest jumps
//!   from one system to another.
//! - [`check_jump`] gives the next system on the route, or the first
//!   [`JumpRefusal`] that applies, in this order: there is no destination,
//!   the ship is nearer the system's centre than its jump distance
//!   ([`MIN_JUMP_DISTANCE`] standard, the Bible's "Jump Distance 1000
//!   pixels", which outfits can move), or it has less than [`JUMP_FUEL`].
//! - [`jump_bearing`] is the heading the ship turns to before it jumps:
//!   the map's bearing from the system it is in to the next.
//! - [`arrival`] places the ship in the system it jumps to: its jump
//!   distance (at least the standard one) plus [`ARRIVAL_MARGIN`] from the
//!   centre, on the side facing the system it came from, at rest and
//!   facing the centre, so it can jump on at once.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::catalog::{GovtId, StarSystem, SystemId};
use crate::flight::{ShipState, heading_of};
use crate::geometry::Vec2;

/// How far from the system's centre, in pixels, a ship must be to jump
/// unless its outfits say otherwise (the Bible: "Jump Distance 1000
/// pixels"): the no-jump zone's standard radius.
pub const MIN_JUMP_DISTANCE: f32 = 1000.0;
/// The fuel a jump uses (the Bible: "100 is one jump").
pub const JUMP_FUEL: f32 = 100.0;

/// How many whole jumps `fuel` holds: none for no fuel or less.
#[must_use]
pub fn max_jumps(fuel: f32) -> u32 {
    // `as` saturates a float into an integer: a negative becomes 0.
    (fuel / JUMP_FUEL).floor() as u32
}
/// How far outside its no-jump zone, in pixels, a ship arrives, so that
/// rounding in the bearing it arrives on can never leave it inside.
pub const ARRIVAL_MARGIN: f32 = 1.0;
/// How far from the centre of the system it jumps to a ship with the
/// standard jump distance arrives: just outside the no-jump zone, where it
/// can jump out again.
pub const ARRIVAL_DISTANCE: f32 = MIN_JUMP_DISTANCE + ARRIVAL_MARGIN;
/// How many days pass in a jump.
pub const DAYS_PER_JUMP: u32 = 1;

/// Why no route can be plotted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// The destination is the system the ship is in.
    AlreadyThere,
    /// The start or the destination is not on the map.
    Unknown,
    /// No chain of hyperlinks reaches the destination.
    Unreachable,
}

/// Why the ship cannot jump.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpRefusal {
    /// The ship has landed: it must take off first. The
    /// [`Session`](crate::Session) gives this before [`check_jump`] is
    /// asked.
    Landed,
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
}

/// One system on the map.
#[derive(Clone, Debug, PartialEq)]
struct Node {
    position: Vec2,
    govt: Option<GovtId>,
    /// Its neighbours, by ascending ID.
    links: BTreeSet<SystemId>,
}

/// Every system's map position and the hyperlinks between them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StarMap {
    nodes: BTreeMap<SystemId, Node>,
}

impl StarMap {
    /// The map of `systems`: each link both ways, without links to itself
    /// or to a system that is not among them. Where two systems share an
    /// ID, the last wins.
    #[must_use]
    pub fn new(systems: Vec<StarSystem>) -> Self {
        let known: BTreeSet<SystemId> = systems.iter().map(|system| system.id).collect();
        let mut links = Vec::new();
        let mut nodes = BTreeMap::new();
        for system in systems {
            let from = system.id;
            links.extend(
                system
                    .links
                    .into_iter()
                    .filter(|to| *to != from && known.contains(to))
                    .map(|to| (from, to)),
            );
            let node = Node {
                position: system.position,
                govt: system.govt,
                links: BTreeSet::new(),
            };
            nodes.insert(from, node);
        }
        for (a, b) in links {
            for (from, to) in [(a, b), (b, a)] {
                if let Some(node) = nodes.get_mut(&from) {
                    node.links.insert(to);
                }
            }
        }
        Self { nodes }
    }

    /// System `id`'s map position, if it is on the map.
    #[must_use]
    pub fn position(&self, id: SystemId) -> Option<Vec2> {
        self.nodes.get(&id).map(|node| node.position)
    }

    /// System `id`'s controlling government: `None` when it is
    /// independent or not on the map.
    #[must_use]
    pub fn govt(&self, id: SystemId) -> Option<GovtId> {
        self.nodes.get(&id).and_then(|node| node.govt)
    }

    /// The fewest jumps from `from` to `to`: each system jumped to, in
    /// order, ending at `to`. Of routes equally short, the first found
    /// exploring each system's neighbours by ascending ID wins.
    pub fn route(&self, from: SystemId, to: SystemId) -> Result<Vec<SystemId>, RouteError> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return Err(RouteError::Unknown);
        }
        if from == to {
            return Err(RouteError::AlreadyThere);
        }
        // Each system reached, and the one it was reached from.
        let mut came_from = BTreeMap::from([(from, from)]);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            for &next in &self.nodes[&at].links {
                if came_from.contains_key(&next) {
                    continue;
                }
                came_from.insert(next, at);
                if next == to {
                    let mut route = vec![to];
                    let mut step = at;
                    while step != from {
                        route.push(step);
                        step = came_from[&step];
                    }
                    route.reverse();
                    return Ok(route);
                }
                queue.push_back(next);
            }
        }
        Err(RouteError::Unreachable)
    }
}

/// Whether `player`, holding `fuel`, can jump to `next`, the next system
/// on its route, when it must be `min_distance` from the centre to jump
/// (exactly that far is far enough): `next`, or the first refusal that
/// applies.
pub fn check_jump(
    player: &ShipState,
    fuel: f32,
    next: Option<SystemId>,
    min_distance: f32,
) -> Result<SystemId, JumpRefusal> {
    let next = next.ok_or(JumpRefusal::NoDestination)?;
    let distance = player.position.length();
    if distance < min_distance {
        return Err(JumpRefusal::TooClose { distance });
    }
    if fuel < JUMP_FUEL {
        return Err(JumpRefusal::NoFuel { fuel });
    }
    Ok(next)
}

/// The heading a ship jumping from the system at map position `from` to the
/// one at `to` turns to before it jumps: the map's bearing from one to the
/// other, as [`arrival`] reads the map. `None` when they are at the same
/// position, which gives nothing to turn to.
#[must_use]
pub fn jump_bearing(from: Vec2, to: Vec2) -> Option<f32> {
    let way = to - from;
    (way != Vec2::ZERO).then(|| heading_of(way))
}

/// Where a ship that must be `jump_distance` from the centre to jump
/// arrives, jumping from the system at map position `from` to the one at
/// `to`: [`ARRIVAL_MARGIN`] outside that distance, or [`ARRIVAL_DISTANCE`]
/// when that is further, on the side facing `from`, at rest and facing the
/// centre, as it drops out of hyperspace. It can jump on at once. Two
/// systems at the same map position give no side, so it arrives from
/// below, facing up.
#[must_use]
pub fn arrival(from: Vec2, to: Vec2, jump_distance: f32) -> ShipState {
    let away = from - to;
    let length = away.length();
    let outward = if length > 0.0 {
        Vec2::new(away.x / length, away.y / length)
    } else {
        Vec2::new(0.0, 1.0)
    };
    let inward = outward * -1.0;
    ShipState {
        position: outward * ARRIVAL_DISTANCE.max(jump_distance + ARRIVAL_MARGIN),
        velocity: Vec2::ZERO,
        heading: heading_of(inward),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

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

    fn system(id: i16, links: &[i16]) -> StarSystem {
        StarSystem {
            id: SystemId(id),
            position: Vec2::new(f32::from(id), 0.0),
            links: links.iter().copied().map(SystemId).collect(),
            govt: None,
        }
    }

    fn ids(route: &[i16]) -> Vec<SystemId> {
        route.iter().copied().map(SystemId).collect()
    }

    fn route(map: &StarMap, from: i16, to: i16) -> Result<Vec<SystemId>, RouteError> {
        map.route(SystemId(from), SystemId(to))
    }

    // Routes.

    #[test]
    fn a_neighbour_is_one_jump() {
        let map = StarMap::new(vec![system(1, &[2]), system(2, &[1])]);
        assert_eq!(route(&map, 1, 2), Ok(ids(&[2])));
        assert_eq!(route(&map, 2, 1), Ok(ids(&[1])));
    }

    #[test]
    fn a_system_two_links_away_is_two_jumps_ending_there() {
        let map = StarMap::new(vec![system(1, &[2]), system(2, &[3]), system(3, &[])]);
        assert_eq!(route(&map, 1, 3), Ok(ids(&[2, 3])));
        assert_eq!(route(&map, 3, 1), Ok(ids(&[2, 1])));
    }

    #[test]
    fn the_route_is_the_fewest_jumps() {
        // 1-2-3-4-5 the long way, and 1-6-5 the short.
        let map = StarMap::new(vec![
            system(1, &[2, 6]),
            system(2, &[3]),
            system(3, &[4]),
            system(4, &[5]),
            system(5, &[]),
            system(6, &[5]),
        ]);
        assert_eq!(route(&map, 1, 5), Ok(ids(&[6, 5])));
        assert_eq!(route(&map, 2, 4), Ok(ids(&[3, 4])));
        assert_eq!(route(&map, 3, 5), Ok(ids(&[4, 5])));
        assert_eq!(
            route(&map, 3, 6),
            Ok(ids(&[2, 1, 6])),
            "a tie: 2 is lower than 4"
        );
    }

    #[test]
    fn of_equally_short_routes_the_one_through_lower_ids_wins() {
        let diamond = |links_from_1: &[i16]| {
            StarMap::new(vec![
                system(1, links_from_1),
                system(2, &[4]),
                system(3, &[4]),
                system(4, &[]),
            ])
        };
        assert_eq!(route(&diamond(&[2, 3]), 1, 4), Ok(ids(&[2, 4])));
        assert_eq!(route(&diamond(&[3, 2]), 1, 4), Ok(ids(&[2, 4])));
        assert_eq!(route(&diamond(&[3, 2]), 4, 1), Ok(ids(&[2, 1])));
    }

    #[test]
    fn a_one_way_link_is_used_both_ways() {
        let map = StarMap::new(vec![system(1, &[]), system(2, &[1]), system(3, &[2])]);
        assert_eq!(route(&map, 1, 3), Ok(ids(&[2, 3])));
        assert_eq!(route(&map, 3, 1), Ok(ids(&[2, 1])));
    }

    #[test]
    fn a_link_to_itself_or_to_a_missing_system_is_no_link() {
        let map = StarMap::new(vec![system(1, &[1, 99]), system(2, &[2])]);
        assert_eq!(route(&map, 1, 2), Err(RouteError::Unreachable));
        assert_eq!(route(&map, 1, 99), Err(RouteError::Unknown));
        let map = StarMap::new(vec![system(1, &[1, 99, 2]), system(2, &[])]);
        assert_eq!(route(&map, 2, 1), Ok(ids(&[1])));
    }

    #[test]
    fn the_destination_cannot_be_where_the_ship_is() {
        let map = StarMap::new(vec![system(1, &[2]), system(2, &[])]);
        assert_eq!(route(&map, 1, 1), Err(RouteError::AlreadyThere));
    }

    #[test]
    fn a_system_not_on_the_map_has_no_route() {
        let map = StarMap::new(vec![system(1, &[2]), system(2, &[])]);
        assert_eq!(route(&map, 1, 7), Err(RouteError::Unknown));
        assert_eq!(route(&map, 7, 1), Err(RouteError::Unknown));
        assert_eq!(route(&map, 7, 7), Err(RouteError::Unknown));
    }

    #[test]
    fn an_unlinked_system_is_unreachable() {
        let map = StarMap::new(vec![
            system(1, &[2]),
            system(2, &[]),
            system(3, &[4]),
            system(4, &[]),
        ]);
        assert_eq!(route(&map, 1, 4), Err(RouteError::Unreachable));
        assert_eq!(route(&map, 4, 2), Err(RouteError::Unreachable));
    }

    #[test]
    fn a_systems_position_is_its_map_position() {
        let map = StarMap::new(vec![StarSystem {
            id: SystemId(128),
            position: Vec2::new(-150.0, 75.0),
            links: Vec::new(),
            govt: None,
        }]);
        assert_eq!(map.position(SystemId(128)), Some(Vec2::new(-150.0, 75.0)));
        assert_eq!(map.position(SystemId(129)), None);
        assert_eq!(StarMap::default().position(SystemId(128)), None);
    }

    #[test]
    fn a_systems_government_is_its_sÿsts() {
        let map = StarMap::new(vec![
            StarSystem {
                govt: Some(GovtId(140)),
                ..system(128, &[129])
            },
            system(129, &[128]),
        ]);
        assert_eq!(map.govt(SystemId(128)), Some(GovtId(140)));
        assert_eq!(map.govt(SystemId(129)), None, "independent");
        assert_eq!(map.govt(SystemId(130)), None, "not on the map");
    }

    // Jumping.

    /// Whether a ship at (`x`, `y`) holding `fuel` can jump to `next`.
    fn ship(x: f32, y: f32, fuel: f32) -> (ShipState, f32) {
        (
            ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            },
            fuel,
        )
    }

    const NEXT: Option<SystemId> = Some(SystemId(129));

    fn check_ship(
        (player, fuel): (ShipState, f32),
        next: Option<SystemId>,
    ) -> Result<SystemId, JumpRefusal> {
        check_jump(&player, fuel, next, MIN_JUMP_DISTANCE)
    }

    #[test]
    fn the_ship_must_be_as_far_out_as_the_minimum_it_is_given() {
        let (player, fuel) = ship(0.0, 600.0, 300.0);
        assert_eq!(check_jump(&player, fuel, NEXT, 500.0), Ok(SystemId(129)));
        assert_eq!(check_jump(&player, fuel, NEXT, 600.0), Ok(SystemId(129)));
        assert_eq!(
            check_jump(&player, fuel, NEXT, 600.1),
            Err(JumpRefusal::TooClose { distance: 600.0 })
        );
        assert_eq!(
            check_jump(&player, fuel, NEXT, 1000.0),
            Err(JumpRefusal::TooClose { distance: 600.0 })
        );
        let (centre, _) = ship(0.0, 0.0, 300.0);
        assert_eq!(check_jump(&centre, fuel, NEXT, 0.0), Ok(SystemId(129)));
    }

    #[test]
    fn far_enough_out_with_fuel_the_ship_jumps_to_the_next_system() {
        assert_eq!(
            check_ship(ship(0.0, -1000.0, 300.0), NEXT),
            Ok(SystemId(129))
        );
        assert_eq!(
            check_ship(ship(600.0, 800.0, 100.0), NEXT),
            Ok(SystemId(129))
        );
        assert_eq!(
            check_ship(ship(1000.1, 0.0, 100.0), NEXT),
            Ok(SystemId(129))
        );
    }

    #[test]
    fn without_a_destination_there_is_no_jump_wherever_the_ship_is() {
        assert_eq!(
            check_ship(ship(0.0, 0.0, 0.0), None),
            Err(JumpRefusal::NoDestination)
        );
        assert_eq!(
            check_ship(ship(0.0, 5000.0, 300.0), None),
            Err(JumpRefusal::NoDestination)
        );
    }

    #[test]
    fn just_inside_the_minimum_distance_is_too_close() {
        assert_eq!(
            check_ship(ship(0.0, -999.9, 300.0), NEXT),
            Err(JumpRefusal::TooClose { distance: 999.9 })
        );
        assert_eq!(
            check_ship(ship(300.0, 400.0, 0.0), NEXT),
            Err(JumpRefusal::TooClose { distance: 500.0 }),
            "before the fuel"
        );
    }

    #[test]
    fn less_than_a_jumps_fuel_is_no_fuel() {
        assert_eq!(
            check_ship(ship(0.0, 1000.0, 99.9), NEXT),
            Err(JumpRefusal::NoFuel { fuel: 99.9 })
        );
        assert_eq!(
            check_ship(ship(0.0, 1000.0, 0.0), NEXT),
            Err(JumpRefusal::NoFuel { fuel: 0.0 })
        );
    }

    #[test]
    fn the_named_values_are_pinned() {
        assert_eq!(MIN_JUMP_DISTANCE, 1000.0);
        assert_eq!(JUMP_FUEL, 100.0);
        assert_eq!(ARRIVAL_MARGIN, 1.0);
        assert_eq!(ARRIVAL_DISTANCE, 1001.0);
        assert_eq!(DAYS_PER_JUMP, 1);
    }

    // The bearing.

    #[test]
    fn the_jump_bearing_faces_the_next_system_on_the_map() {
        let sol = Vec2::new(20.0, -30.0);
        assert_eq!(
            jump_bearing(sol, Vec2::new(620.0, -30.0)),
            Some(90.0),
            "east"
        );
        assert_eq!(
            jump_bearing(sol, Vec2::new(20.0, -500.0)),
            Some(0.0),
            "north"
        );
        assert_eq!(
            jump_bearing(sol, Vec2::new(20.0, 70.0)),
            Some(180.0),
            "south"
        );
        let south_west = jump_bearing(sol, Vec2::new(-80.0, 70.0)).expect("a bearing");
        assert!((south_west - 225.0).abs() < 1e-3, "{south_west}");
        assert_eq!(jump_bearing(sol, sol), None, "nowhere to turn to");
    }

    // Arriving.

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn from_the_east_the_ship_arrives_at_the_east_edge_facing_west() {
        let arrived = arrival(
            Vec2::new(600.0, 0.0),
            Vec2::new(0.0, 0.0),
            MIN_JUMP_DISTANCE,
        );
        assert_eq!(arrived.position, Vec2::new(1001.0, 0.0));
        assert_eq!(arrived.velocity, Vec2::ZERO);
        assert_eq!(arrived.heading, 270.0);
    }

    #[test]
    fn from_the_north_west_the_ship_arrives_at_the_north_west_edge() {
        let arrived = arrival(
            Vec2::new(100.0, 50.0),
            Vec2::new(400.0, 350.0),
            MIN_JUMP_DISTANCE,
        );
        let diagonal = ARRIVAL_DISTANCE / 2.0_f32.sqrt();
        assert!(
            close(arrived.position, Vec2::new(-diagonal, -diagonal)),
            "{arrived:?}"
        );
        assert_eq!(arrived.velocity, Vec2::ZERO);
        assert!((arrived.heading - 135.0).abs() < 1e-3, "{arrived:?}");
        assert!((arrived.position.length() - ARRIVAL_DISTANCE).abs() < 1e-3);
    }

    #[test]
    fn from_the_same_map_position_the_ship_arrives_from_below_facing_up() {
        let here = Vec2::new(20.0, -30.0);
        let arrived = arrival(here, here, MIN_JUMP_DISTANCE);
        assert_eq!(arrived.position, Vec2::new(0.0, 1001.0));
        assert_eq!(arrived.velocity, Vec2::ZERO);
        assert_eq!(arrived.heading, 0.0);
    }

    #[test]
    fn the_ship_arrives_at_rest_facing_the_centre() {
        let arrived = arrival(Vec2::new(0.0, -10.0), Vec2::ZERO, MIN_JUMP_DISTANCE);
        assert_eq!(arrived.position, Vec2::new(0.0, -1001.0));
        assert_eq!(arrived.velocity, Vec2::ZERO);
        assert_eq!(arrived.heading, 180.0, "facing down, to the centre");
    }

    #[test]
    fn a_raised_jump_distance_moves_the_arrival_out_and_a_lowered_one_does_not() {
        let from = Vec2::new(600.0, 0.0);
        let raised = arrival(from, Vec2::ZERO, 1250.0);
        assert_eq!(raised.position, Vec2::new(1250.0 + ARRIVAL_MARGIN, 0.0));
        let lowered = arrival(from, Vec2::ZERO, 500.0);
        assert_eq!(lowered.position, Vec2::new(ARRIVAL_DISTANCE, 0.0));
        let none = arrival(from, Vec2::ZERO, 0.0);
        assert_eq!(none.position, Vec2::new(ARRIVAL_DISTANCE, 0.0));
    }

    #[test]
    fn from_any_bearing_the_ship_arrives_where_it_can_jump() {
        // Before the margin, several of these bearings (and the first odd
        // pair) left the ship at 999.99994, inside the zone.
        let around = (0..48u8).map(|step| {
            let angle = (f32::from(step) * 7.5).to_radians();
            (
                Vec2::new(500.0 * angle.cos(), 500.0 * angle.sin()),
                Vec2::ZERO,
            )
        });
        let odd = [
            (Vec2::new(100.0, 50.0), Vec2::new(400.0, 350.0)),
            (Vec2::new(-37.0, 211.0), Vec2::new(13.0, -5.0)),
        ];
        for (from, to) in around.chain(odd) {
            for jump_distance in [MIN_JUMP_DISTANCE, 1250.0, 500.0, 0.0] {
                let arrived = arrival(from, to, jump_distance);
                assert_eq!(
                    check_jump(&arrived, JUMP_FUEL, NEXT, jump_distance),
                    Ok(SystemId(129)),
                    "{from:?} to {to:?} at {jump_distance}: {arrived:?}"
                );
            }
        }
    }
}
