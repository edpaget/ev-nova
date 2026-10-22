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
//! once (@0x6bddb), as the `jump_exploration` module says.

use super::Session;
use crate::catalog::{PilotCatalog, SystemId, TrafficCatalog};
use crate::chance::Chance;
use crate::hyperspace::{JUMP_FUEL, MultiJumpRule, hops_per_jump};

/// What a jump's hop loop made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Chain {
    /// The hops made along the course: none when the jump left it, one
    /// for a single jump along it.
    pub(super) hops: usize,
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
            hops: 0,
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
        made.hops = 1;
        for _ in 1..hops_per_jump(self.ship.fit.stats.multi_jump, self.setup.multi_jump) {
            let next = self.pilot.course.get(made.hops);
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
            made.hops += 1;
            // Each system passed on the way, written as the player's
            // (@0x6bdd4) and explored at once (@0x6bddb).
            self.chart
                .write_jump_system(&mut self.pilot, &self.setup, made.at);
            self.explore_now(catalog, chance, made.at);
            (made.from, made.at) = (made.at, next);
        }
        made
    }
}
