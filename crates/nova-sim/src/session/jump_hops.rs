//! A jump's hop loop: the systems a multi-jump passes through on its
//! way, each written as the player's system and explored at once
//! (`_HandlePlayer` @0x6bd67-0x6bdfc, in `EV Nova.app`'s `EV Nova`,
//! i386).
//!
//! The chain is made only when the course's first hop is the system the
//! jump took, or stands at its place; it makes as many hops as
//! [`hops_per_jump`] gives for the ship's multi-jump total and the
//! session's [`MultiJumpRule`], and stops early where the course ends.
//! Under [`MultiJumpRule::PerHop`] a further hop is made only when the
//! fuel, less the hops already owed, pays for it.
//!
//! Each system passed is the visible system that stands for its hop
//! (@0x6bd67-0x6bdc1), written as the player's (@0x6bdd4) and explored at
//! once (@0x6bddb), as the `jump_exploration` module says, then spooled
//! off the course (`_SpoolNextHyperRouteDest` @0x6bde0), so a move an
//! `OnExplore` or a day makes (see the `jump_moves` module) rewrites the
//! course the original holds there, without the hops passed.
//!
//! **The course's step.** The arrival's tail (@0x6c472-0x6c4c8) spools
//! once more. Off the multi-jump branch it first clears a course whose
//! next hop stands elsewhere than the player's system, as the moves left
//! it. So by [`RuleKey::RouteRewrite`]'s engine reading the entry drops
//! the arrival's hop on the multi-jump branch, and otherwise steps as an
//! arrival off the course does (see the `course` module); by the other
//! reading it drops the hop made, if any ([`Session::jump_course_hops`]).

use super::Session;
use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::hyperspace::{JUMP_FUEL, MultiJumpRule, hops_per_jump};
#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;

/// What a jump's hop loop made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Chain {
    /// Whether the jump made its hops along the course: its first hop
    /// the course's next (@0x6bd5b-0x6bd83). The hops passed on the way
    /// are spooled off as they are made, so how many there were matters
    /// to nothing after.
    pub(super) on_course: bool,
    /// The hops whose fuel the jump takes (@0x6be31) and whose days pass
    /// (@0x6c144): one, or by `PerHop` every hop.
    pub(super) jumps: u16,
    /// The system the last leg left.
    pub(super) from: SystemId,
    /// The last hop's stand-in, where the last leg arrives.
    pub(super) at: SystemId,
}

impl Session {
    /// Makes the hops of the jump from `departure`, bound first for
    /// `first`, along the course, writing and exploring each system passed
    /// on the way, reading `catalog` and drawing on `chance` (see the
    /// module docs). The last hop is left to the caller, which writes and
    /// explores the arrival.
    pub(super) fn hop_along<C>(
        &mut self,
        catalog: &C,
        chance: &mut dyn Chance,
        departure: SystemId,
        first: SystemId,
    ) -> Chain
    where
        C: PilotCatalog + TrafficCatalog + ?Sized,
    {
        let mut made = Chain {
            on_course: false,
            jumps: 1,
            from: departure,
            at: first,
        };
        let on_course =
            self.pilot.course.first().is_some_and(|&hop| {
                hop == first || self.chart.galaxy.same_place(hop) == Some(first)
            });
        if !on_course {
            return made;
        }
        made.on_course = true;
        for _ in 1..hops_per_jump(self.ship.fit.stats.multi_jump, self.setup.multi_jump) {
            // The hop after the one being made: the hops passed are
            // spooled off already.
            let next = self.pilot.course.get(1);
            let Some(next) = next.and_then(|&hop| self.chart.galaxy.same_place(hop)) else {
                break;
            };
            // By `PerHop` a further hop is made only if the fuel, less the
            // hops' already owed, pays for it too; its fuel is taken with
            // the first's, its days pass with the first's.
            if self.setup.multi_jump == MultiJumpRule::PerHop {
                let owed = JUMP_FUEL * f32::from(made.jumps);
                if self.pilot.reserves.fuel.now - owed < JUMP_FUEL {
                    break;
                }
                made.jumps += 1;
            }
            // Each system passed on the way, written as the player's
            // (@0x6bdd4) and explored at once (@0x6bddb).
            self.chart
                .write_jump_system(&mut self.pilot, &self.setup, made.at);
            self.explore_now(catalog, chance, made.at);
            // Spooled off the course (@0x6bde0).
            let at = made.at;
            self.chart
                .course_after_arrival(&mut self.pilot, &self.setup, 1, at);
            (made.from, made.at) = (made.at, next);
        }
        made
    }

    /// The hops of the course the jump's entry steps over
    /// ([`Entry::Jump`](super::entry::Entry::Jump)), the jump made
    /// `on_course` or not, on the `multi_jump` branch or not (see the
    /// module docs): the hops passed are spooled already, so at most the
    /// arrival's; none off the multi-jump branch by the engine's reading,
    /// whose entry clears a course standing elsewhere and else drops its
    /// next hop.
    pub(super) fn jump_course_hops(&self, on_course: bool, multi_jump: bool) -> usize {
        match self.setup.route_rewrite() {
            RuleSource::Engine if !multi_jump => 0,
            RuleSource::Engine | RuleSource::Bible => usize::from(on_course),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::hyperspace::{JUMP_FUEL, MultiJumpRule};
    use crate::session::tests::{bound_for_134, chained};
    use crate::testkit::{begin_jump_now, catalog};

    #[test]
    fn by_per_hop_each_hop_made_owes_its_own_fuel_and_days() {
        // Along 131, 132, 134 with a multi-jump of 2: by `PerHop` every
        // hop the fuel pays for, each with its fuel and days; by the
        // engine two hops for one jump's.
        for (rule, to, owed) in [
            (MultiJumpRule::PerHop, 134, 3_u16),
            (MultiJumpRule::Engine, 132, 1),
        ] {
            let catalog = chained(catalog(), 2);
            let mut session = bound_for_134(&catalog, rule);
            let (fuel, mut date) = (session.reserves().fuel.now, session.date());
            begin_jump_now(&mut session).expect("jumps");
            assert_eq!(
                session.arrive(&catalog, &mut NeverFires),
                Some(SystemId(to)),
                "{rule:?}"
            );
            assert_eq!(
                session.reserves().fuel.now,
                fuel - f32::from(owed) * JUMP_FUEL,
                "{rule:?}"
            );
            for _ in 0..owed {
                date = date.next_day();
            }
            assert_eq!(session.date(), date, "{rule:?}");
        }
    }
}
