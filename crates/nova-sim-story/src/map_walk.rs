//! The exploration walk a map grant, the take-off and a jump's
//! re-explore run, as `_AutoSetExploration` (0xdaba-0xdbe1) and
//! `_RecursiveAutoExplore` (0xd942-0xdab9) run it, over a [`WalkSite`]
//! (crons-news-ranks-fidelity phase 45). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! **`_AutoSetExploration(system, reach, level)`**
//! ([`auto_set_exploration`]) zeroes the 0x800-byte visited array at
//! 0xde93c (@0xdac3-0xdae0), writes `reach` to the word 0xde944 points at
//! (@0xdae2-0xdae9), calls `_RecursiveAutoExplore(system, 0, level)`
//! (@0xdb01), then runs the known pass, with no clear (@0xdb06-0xdbd4).
//!
//! **`_RecursiveAutoExplore(system, depth, level)`**
//! ([`recursive_auto_explore`]), in order:
//!
//! 1. it returns when the reach is below `depth` (@0xd967-0xd96a) or the
//!    system is visited already (@0xd979-0xd97d);
//! 2. it marks the system visited (@0xd983);
//! 3. it explores the system's nebulae (`_ExploreNebulaeFromSystem`
//!    @0xd98a), whose `OnExplore`s run at once and may grant a map: a
//!    whole nested walk;
//! 4. it raises the system's explored level (+0x90) to `level`
//!    (@0xd9a6-0xd9af), then its head's (@0xd9b6-0xda10);
//! 5. it walks the system's 16 Con slots in order, each resolved to the
//!    visible system at its place (@0xda17-0xdaac), calling itself on each
//!    at `depth + 1` with the same `level` (@0xdaa3).
//!
//! It is depth first, so a system first reached the long way round, too
//! deep to go on from, is never entered again by a shorter way, and the
//! systems beyond it within N jumps are missed.
//!
//! **The globals.** Each frame reads the reach from the global at its own
//! entry only (`movl 0xde944,%eax; cmpw %dx,(%eax)` @0xd95a-0xd967); its
//! `depth`, `level` and system are its own (-0x1c, -0x1e, -0x1a(%ebp)).
//! So a map granted by an `OnExplore` inside the walk, whose
//! `_AutoSetExploration` clears the visited array (@0xdada) and writes its
//! own reach, changes the outer walk: every later child of every pending
//! outer frame is tested against the inner grant's reach, and the outer
//! walk re-enters any system it had visited that the inner walk did not
//! reach. A re-entered system's nebulae are explored again, which runs no
//! `OnExplore` (each nebula's +0x9 is set, @0x50c6), its level raised
//! again and its links walked again. The inner grant's known pass runs
//! inside the outer walk, and the outer one's again at its end. So:
//!
//! - an inhabited-independent or class map (reach 0) granted inside an
//!   N-jump walk stops every pending outer frame, as each child is at
//!   depth 1 or more;
//! - an N-jump map granted inside a reach-0 walk (a per-system map's, the
//!   take-off's or a jump's re-explore) makes that system's frame walk
//!   its neighbours out to N, at the outer walk's level, around that
//!   system rather than the player's (when an `OnExplore` moved the
//!   player first).
//!
//! The visited array is shared with `_RecursivePenaltySlap` (@0x983c) and
//! `_RecursiveEscapeStellarSearch` (@0xd10b, @0xd263) too; no set
//! operator reaches either, so neither interleaves with a walk.
//!
//! The walk reads the reach and the visited set through the
//! [`WalkSite`] on every call, never a copy, so a nested
//! [`auto_set_exploration`] reaches the outer frames as the globals do.
//! Each frame reads its links once, after its own exploration, where the
//! original re-reads the system table per slot (@0xda1b-0xda29): no
//! `OnExplore` changes the links, which only a refresh rewrites. The
//! 16-bit depth and reach are not kept.
//!
//! **By the Bible** ([`breadth_first`]; the other reading of
//! [`RuleKey::MapExplore`](crate::RuleKey::MapExplore)), every system
//! within N jumps is explored ("how many jumps away from the current
//! system to explore"), breadth first, over the same site and the same
//! shared state: each system is visited as it is taken from the queue,
//! the reach read then.

use std::collections::VecDeque;

use crate::catalog::SystemId;
use crate::pilot::ExploredLevel;

/// What the walk needs of the galaxy and the player: the original's
/// visited array and reach globals, a system's nebula explorations, its
/// explored level, its links and the known pass.
pub trait WalkSite {
    /// Clears the visited set and sets the reach to `reach`
    /// (`_AutoSetExploration` @0xdada-0xdae9).
    fn begin(&mut self, reach: u16);
    /// The reach, as the last [`WalkSite::begin`] set it.
    fn reach(&self) -> u16;
    /// Marks `system` visited: whether it was not already.
    fn visit(&mut self, system: SystemId) -> bool;
    /// Explores `system`'s nebulae, their `OnExplore`s at once: one may
    /// grant a map, a whole nested walk over this site.
    fn explore(&mut self, system: SystemId);
    /// Raises `system`'s explored level, and its place's head's, to
    /// `level` when lower.
    fn raise(&mut self, system: SystemId, level: ExploredLevel);
    /// The systems `system`'s links reach, in the order the walk meets
    /// them.
    fn links(&self, system: SystemId) -> Vec<SystemId>;
    /// The known pass, with no clear.
    fn known_pass(&mut self);
}

/// `_AutoSetExploration` (see the module docs): the walk from `from`
/// reaching `reach` jumps, raising each system to `level`, then the known
/// pass.
pub fn auto_set_exploration<S: WalkSite + ?Sized>(
    site: &mut S,
    from: SystemId,
    reach: u16,
    level: ExploredLevel,
) {
    site.begin(reach);
    recursive_auto_explore(site, from, 0, level);
    site.known_pass();
}

/// `_RecursiveAutoExplore` (see the module docs): `system`, `depth` jumps
/// out, and on from it, reading the reach and the visited set from
/// `site` as it goes.
pub fn recursive_auto_explore<S: WalkSite + ?Sized>(
    site: &mut S,
    system: SystemId,
    depth: u32,
    level: ExploredLevel,
) {
    if u32::from(site.reach()) < depth || !site.visit(system) {
        return;
    }
    site.explore(system);
    site.raise(system, level);
    for next in site.links(system) {
        recursive_auto_explore(site, next, depth + 1, level);
    }
}

/// The Bible's walk (see the module docs): every system within `reach`
/// jumps of `from`, breadth first, each visited, explored and raised to
/// `level` as it is taken from the queue, then the known pass.
pub fn breadth_first<S: WalkSite + ?Sized>(
    site: &mut S,
    from: SystemId,
    reach: u16,
    level: ExploredLevel,
) {
    site.begin(reach);
    let mut queue = VecDeque::from([(from, 0_u32)]);
    while let Some((system, depth)) = queue.pop_front() {
        if u32::from(site.reach()) < depth || !site.visit(system) {
            continue;
        }
        site.explore(system);
        site.raise(system, level);
        queue.extend(site.links(system).into_iter().map(|next| (next, depth + 1)));
    }
    site.known_pass();
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::hyperspace::{HyperlinkRule, StarMap};
    use nova_sim_catalog::fixture::star;

    /// What the [`Site`] saw, in order.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Call {
        Begin(u16),
        Explore(i16),
        Raise(i16),
        Known,
    }

    /// A walk over `map`, recording its calls; exploring a system of
    /// `grants` the first time runs a nested walk from its origin, as a
    /// nebula's `OnExplore` granting a map would.
    struct Site {
        map: StarMap,
        hyperlinks: HyperlinkRule,
        visited: BTreeSet<SystemId>,
        reach: u16,
        calls: Vec<Call>,
        grants: BTreeMap<i16, (i16, u16)>,
        bible: bool,
    }

    impl Site {
        fn new(map: StarMap) -> Self {
            Self {
                map,
                hyperlinks: HyperlinkRule::Engine,
                visited: BTreeSet::new(),
                reach: 0,
                calls: Vec::new(),
                grants: BTreeMap::new(),
                bible: false,
            }
        }

        /// Exploring `at` grants a map of `reach` jumps from `from`.
        fn granting(mut self, at: i16, from: i16, reach: u16) -> Self {
            self.grants.insert(at, (from, reach));
            self
        }

        fn walk(&mut self, from: i16, reach: u16) {
            if self.bible {
                breadth_first(self, SystemId(from), reach, ExploredLevel::Charted);
            } else {
                auto_set_exploration(self, SystemId(from), reach, ExploredLevel::Charted);
            }
        }

        fn explored(&self) -> Vec<i16> {
            self.calls
                .iter()
                .filter_map(|call| match call {
                    Call::Explore(system) => Some(*system),
                    _ => None,
                })
                .collect()
        }

        fn raised(&self) -> BTreeSet<SystemId> {
            self.calls
                .iter()
                .filter_map(|call| match call {
                    Call::Raise(system) => Some(SystemId(*system)),
                    _ => None,
                })
                .collect()
        }
    }

    impl WalkSite for Site {
        fn begin(&mut self, reach: u16) {
            self.calls.push(Call::Begin(reach));
            self.visited.clear();
            self.reach = reach;
        }

        fn reach(&self) -> u16 {
            self.reach
        }

        fn visit(&mut self, system: SystemId) -> bool {
            self.visited.insert(system)
        }

        fn explore(&mut self, system: SystemId) {
            self.calls.push(Call::Explore(system.0));
            if let Some((from, reach)) = self.grants.remove(&system.0) {
                self.walk(from, reach);
            }
        }

        fn raise(&mut self, system: SystemId, level: ExploredLevel) {
            assert_eq!(level, ExploredLevel::Charted);
            self.calls.push(Call::Raise(system.0));
        }

        fn links(&self, system: SystemId) -> Vec<SystemId> {
            self.map.ordered_jumps(system, self.hyperlinks)
        }

        fn known_pass(&mut self) {
            self.calls.push(Call::Known);
        }
    }

    fn ids(list: &[i16]) -> BTreeSet<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// 128 - 129 - 130 - 131 in a line, each linking the next and back.
    fn line() -> StarMap {
        StarMap::new(vec![
            star(128, (0.0, 0.0), &[129]),
            star(129, (1.0, 0.0), &[128, 130]),
            star(130, (2.0, 0.0), &[129, 131]),
            star(131, (3.0, 0.0), &[130]),
        ])
    }

    /// The systems a walk of `reach` from `from` raises, by the engine or
    /// the Bible.
    fn reached(map: StarMap, from: i16, reach: u16, bible: bool) -> BTreeSet<SystemId> {
        let mut site = Site {
            bible,
            ..Site::new(map)
        };
        site.walk(from, reach);
        site.raised()
    }

    #[test]
    fn a_walk_of_n_jumps_raises_the_line_that_far_both_ways() {
        for bible in [false, true] {
            assert_eq!(reached(line(), 128, 1, bible), ids(&[128, 129]));
            assert_eq!(reached(line(), 128, 2, bible), ids(&[128, 129, 130]));
            assert_eq!(reached(line(), 130, 1, bible), ids(&[129, 130, 131]));
            assert_eq!(reached(line(), 129, 9, bible), ids(&[128, 129, 130, 131]));
            assert_eq!(reached(line(), 129, 0, bible), ids(&[129]));
        }
    }

    /// A (128) lists B (129) then C (130); B lists A and C; C lists A, B
    /// and D (131); D lists C.
    fn diamond() -> StarMap {
        StarMap::new(vec![
            star(128, (0.0, 0.0), &[129, 130]),
            star(129, (1.0, 0.0), &[128, 130]),
            star(130, (1.0, 1.0), &[128, 129, 131]),
            star(131, (2.0, 1.0), &[130]),
        ])
    }

    #[test]
    fn the_engines_depth_first_walk_misses_a_system_the_bibles_reaches() {
        // The engine goes A, B (1), C (2, too deep to go on), and never
        // enters C again from A at 1, so D, two jumps from A, is missed.
        assert_eq!(reached(diamond(), 128, 2, false), ids(&[128, 129, 130]));
        assert_eq!(reached(diamond(), 128, 2, true), ids(&[128, 129, 130, 131]));
        assert_eq!(
            reached(diamond(), 128, 3, false),
            ids(&[128, 129, 130, 131])
        );
    }

    #[test]
    fn the_walk_follows_the_links_the_site_gives() {
        // 129 lists 128, which does not list it back.
        let map = || {
            StarMap::new(vec![
                star(128, (0.0, 0.0), &[]),
                star(129, (1.0, 0.0), &[128]),
            ])
        };
        for bible in [false, true] {
            let mut site = Site {
                bible,
                ..Site::new(map())
            };
            site.walk(128, 1);
            assert_eq!(site.raised(), ids(&[128]), "{bible}");
            let mut site = Site {
                bible,
                hyperlinks: HyperlinkRule::BothWays,
                ..Site::new(map())
            };
            site.walk(128, 1);
            assert_eq!(site.raised(), ids(&[128, 129]), "{bible}");
        }
    }

    #[test]
    fn each_system_is_explored_then_raised_before_its_neighbours_are_walked() {
        let mut site = Site::new(line());
        site.walk(128, 2);
        assert_eq!(
            site.calls,
            [
                Call::Begin(2),
                Call::Explore(128),
                Call::Raise(128),
                Call::Explore(129),
                Call::Raise(129),
                Call::Explore(130),
                Call::Raise(130),
                Call::Known,
            ]
        );
    }

    #[test]
    fn a_map_granted_inside_the_walk_sets_the_reach_its_outer_frames_read() {
        // A 3-jump walk from 128; exploring 129 grants a 1-jump map from
        // 128. The inner walk explores 128 and 129 again (the visited set
        // cleared); the outer 129 frame then tests 130 against a reach of
        // 1, so 130 and 131 are never reached.
        let mut site = Site::new(line()).granting(129, 128, 1);
        site.walk(128, 3);
        assert_eq!(
            site.calls,
            [
                Call::Begin(3),
                Call::Explore(128),
                Call::Raise(128),
                Call::Explore(129),
                Call::Begin(1),
                Call::Explore(128),
                Call::Raise(128),
                Call::Explore(129),
                Call::Raise(129),
                Call::Known,
                Call::Raise(129),
                Call::Known,
            ]
        );
        assert_eq!(site.raised(), ids(&[128, 129]));
    }

    /// 130 - 131 and 140 - 141, each pair linking both ways.
    fn pairs() -> StarMap {
        StarMap::new(vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (1.0, 0.0), &[130]),
            star(140, (5.0, 0.0), &[141]),
            star(141, (6.0, 0.0), &[140]),
        ])
    }

    #[test]
    fn a_map_granted_inside_a_reach_0_walk_extends_it_and_clears_its_visits() {
        // A reach-0 walk over 140 (an independent map's); exploring 140
        // grants a 2-jump map from 130. The 140 frame then walks 141 at
        // depth 1 under a reach of 2, and 140 again at 2, its visit wiped.
        let mut site = Site::new(pairs()).granting(140, 130, 2);
        site.walk(140, 0);
        assert_eq!(site.explored(), [140, 130, 131, 141, 140]);
        assert_eq!(site.raised(), ids(&[130, 131, 140, 141]));
    }

    #[test]
    fn a_reach_0_map_granted_inside_a_walk_stops_its_outer_frames() {
        // The 3-jump walk from 128; exploring 129 runs a reach-0 walk over
        // 131, whose visit alone is left.
        let mut site = Site::new(line()).granting(129, 131, 0);
        site.walk(128, 3);
        assert_eq!(site.explored(), [128, 129, 131]);
        assert_eq!(site.raised(), ids(&[128, 129, 131]));
    }

    #[test]
    fn the_bibles_walk_shares_the_reach_and_visits_too() {
        let mut site = Site {
            bible: true,
            ..Site::new(line()).granting(129, 128, 1)
        };
        site.walk(128, 3);
        assert_eq!(site.explored(), [128, 129, 128, 129]);
        assert_eq!(site.raised(), ids(&[128, 129]));
    }
}
