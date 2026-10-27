//! Where the player's jump arrives, under
//! [`RuleKey::JumpArrival`](crate::RuleKey::JumpArrival).
//!
//! **By the engine** it is the original's arrival block, `_HandlePlayer`
//! @0x6bc0d-0x6bd20 (see [`jump_arrival`](crate::jump_arrival)): the ship
//! comes in [`ARRIVAL_RADIUS`](crate::jump_arrival::ARRIVAL_RADIUS) from
//! the centre on the departure's side, along the bearing from the system
//! it left to the first hop (@0x6bc1b-0x6bccd) — the one the warp stage
//! turned it to ([`Chart::jump_aim`](super::Chart::jump_aim)), so for a multi-jump the first
//! hop's side, not the last leg's, as the block runs once before the hop
//! loop (@0x6bdb4-0x6bdfc). It keeps the heading the warp left. An
//! inertial ship moves at its top speed ([`Session::top_speed`]) along its
//! whole-degree heading (@0x6bcfa-0x6bd1b); an inertialess one arrives
//! still, its speed along its heading at its top speed (@0x6bce7-0x6bcf5),
//! which its velocity follows from the next tick (see the `inertialess`
//! module). The escorts it leads take its velocity and heading, as
//! `_RespawnEscort` copies them (@0x3d69e-0x3d6d1), then are pushed back
//! from their slots and come 50 faster to glide in (see the
//! `escort_arrival` module).
//!
//! **By the other reading** it is the port's earlier at-rest arrival
//! ([`hyperspace::arrival`](crate::hyperspace::arrival)): just outside the
//! no-jump zone, max(1001, jump distance + 1) out on the last leg's side,
//! facing the centre, its speed none, so it can always jump on at once. A
//! hypergate's like-a-jump exit
//! ([`GateArrivalRule::LikeJump`](crate::gate::GateArrivalRule::LikeJump))
//! reads that arrival whichever reading this is: it is no reading of the
//! original's.
//!
//! **When.** The original's block runs as the jump ends, before the hop
//! loop (@0x6bd67) and every exploration (@0x6bddb, @0x6be12), so the
//! engine's reading places the ship there too
//! ([`Session::jump_arrival_block`]): an `OnExplore`'s or a crön's change
//! of ship class (`C`, `E`, `H`) comes after it and leaves the arrival's
//! speed as the class before set it, and a sound raised on the jump's
//! days is heard from where the ship arrives. The entry then keeps the
//! ship where it is. The other reading has no original and places at the
//! entry ([`Session::jump_entry_placement`]), on the last leg's side,
//! which only the hop loop gives.
//!
//! All of it settles in the arrival's step, before the frame is drawn, as
//! the original's does. The block plays no sound.

use super::Session;
use super::entry::{Placement, ShipPlacement};
use crate::catalog::SystemId;
use crate::hyperspace::arrival;
use crate::jump_arrival::top_speed_arrival;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// The arrival block as the jump from `departure`, bound first for
    /// `first`, ends (see the module docs): the strikes still unanswered
    /// dropped, as the entry's row would drop them (`Reset::Strikes`),
    /// their grudges held first against the departure, its NPCs and the
    /// player as they stand before the ship moves; then, by the engine's
    /// reading, the ship placed ([`Session::jump_arrival_block`]).
    pub(super) fn place_jump_arrival(&mut self, departure: SystemId, first: SystemId) {
        let strikes = std::mem::take(&mut self.space.strikes);
        self.hold_left_grudges(&strikes, departure);
        if let Some(ship) = self.jump_arrival_block(departure, first) {
            self.place(Placement {
                ship,
                stellar: None,
                hold: false,
            });
        }
    }

    /// Where the arrival block (@0x6bc0d-0x6bd20) puts the player's jump
    /// from `departure`, bound first for `first`, as the jump ends and
    /// before its explorations, by the engine's reading; `None` by the
    /// other, which places at the entry (see the module docs).
    pub(super) fn jump_arrival_block(
        &self,
        departure: SystemId,
        first: SystemId,
    ) -> Option<ShipPlacement> {
        if self.setup.rules.source_for(RuleKey::JumpArrival) != RuleSource::Engine {
            return None;
        }
        let map = |id| self.chart.galaxy.position(id).unwrap_or_default();
        let (ship, speed) = top_speed_arrival(
            map(departure),
            map(first),
            self.ship.player.heading,
            self.top_speed(),
            self.ship.player_inertialess(&self.setup),
        );
        Some(ShipPlacement::ArriveAtSpeed { ship, speed })
    }

    /// Where the jump's entry puts the ship, its last leg from `from` to
    /// `at`: where it is by the engine's reading, the arrival block having
    /// placed it; by the other, at rest just outside the no-jump zone on
    /// the last leg's side (see the module docs).
    pub(super) fn jump_entry_placement(&self, from: SystemId, at: SystemId) -> ShipPlacement {
        let map = |id| self.chart.galaxy.position(id).unwrap_or_default();
        match self.setup.rules.source_for(RuleKey::JumpArrival) {
            RuleSource::Engine => ShipPlacement::Keep,
            RuleSource::Bible => ShipPlacement::Arrive(arrival(
                map(from),
                map(at),
                self.ship.fit.stats.jump_distance,
            )),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;

    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::hyperspace::{MIN_JUMP_DISTANCE, MultiJumpRule, arrival};
    use crate::session::Session;
    use crate::session::tests::{bound_for_134, chained};
    use crate::stats::INERTIALESS_HULL;
    use crate::testkit::work;
    use crate::testkit::{FAST, FakePilotCatalog, begin_jump_now, catalog, fly_out, nebula, ship};
    use crate::trig::{accel, whole};
    use crate::{RuleKey, RuleSource, Rulebook};

    /// The other reading: the at-rest arrival.
    fn at_rest() -> Rulebook {
        Rulebook::default().with_override(RuleKey::JumpArrival, RuleSource::Bible)
    }

    /// `catalog`'s session under `rules`, its course plotted to 132 (by
    /// 131, at (600, 0) from 130 at the centre), flown out and jumping.
    fn jumping(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        session.plot_course(SystemId(132)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session
    }

    /// The fake catalog with ship 128's hull inertialess.
    fn inertialess() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: INERTIALESS_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    fn near(a: Vec2, b: Vec2, within: f32) -> bool {
        (a - b).length() <= within
    }

    #[test]
    fn a_jump_arrives_1350_out_on_the_side_it_came_from_at_top_speed() {
        let catalog = catalog();
        let mut session = jumping(&catalog, Rulebook::default());
        let heading = session.player().heading;
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        let player = *session.player();
        assert!(
            near(player.position, Vec2::new(-1350.0, 0.0), 1e-3),
            "on 130's side: {player:?}"
        );
        assert_eq!(player.heading, heading, "the warp's heading");
        let top = session.top_speed();
        assert!(top > 0.0);
        assert_eq!(player.velocity, accel(whole(heading), top));
        assert_eq!(session.player_speed(), player.velocity.length());
    }

    #[test]
    fn the_arrival_keeps_the_heading_the_warp_left() {
        let catalog = catalog();
        let mut session = jumping(&catalog, Rulebook::default());
        session.ship.player.heading = 200.5;
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let player = *session.player();
        assert_eq!(player.heading, 200.5);
        assert_eq!(player.velocity, accel(200, session.top_speed()));
        assert!(near(player.position, Vec2::new(-1350.0, 0.0), 1e-3));
    }

    #[test]
    fn an_inertialess_player_arrives_still_at_top_speed_and_speeds_up_toward_its_heading() {
        let catalog = inertialess();
        let mut session = jumping(&catalog, Rulebook::default());
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        let player = *session.player();
        assert_eq!(player.velocity, Vec2::ZERO);
        assert!(near(player.position, Vec2::new(-1350.0, 0.0), 1e-3));
        let top = session.top_speed();
        assert!(top > 0.0);
        assert_eq!(session.player_speed(), top);
        work(&mut session, &catalog, Controls::default());
        let moved = session.player().velocity;
        assert!(moved.length() > 0.0, "{moved:?}");
        let along = crate::flight::facing(session.player().heading);
        assert!(
            moved.x * along.x + moved.y * along.y > 0.0,
            "toward its heading: {moved:?}"
        );
    }

    #[test]
    fn a_multi_jump_arrives_on_the_first_hops_side() {
        let catalog = chained(catalog(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default());
        begin_jump_now(&mut session, &catalog).expect("jumps");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(132))
        );
        // 130 (0, 0) to 131 (600, 0) is east, so it comes in from the
        // west; the last leg, 131 to 132 (600, 600), would be from the
        // north.
        let position = session.player().position;
        assert!(
            near(position, Vec2::new(-1350.0, 0.0), 1e-3),
            "{position:?}"
        );
    }

    #[test]
    fn under_the_other_reading_the_ship_arrives_at_rest_just_outside_the_zone() {
        for catalog in [catalog(), inertialess()] {
            let mut session = jumping(&catalog, at_rest());
            session.arrive(&catalog, &mut NeverFires).expect("arrives");
            assert_eq!(
                *session.player(),
                arrival(Vec2::ZERO, Vec2::new(600.0, 0.0), MIN_JUMP_DISTANCE)
            );
            assert_eq!(session.player().position, Vec2::new(-1001.0, 0.0));
            assert_eq!(session.player_speed(), 0.0);
        }
        let catalog = chained(catalog(), 2);
        let mut session = bound_for_134(&catalog, MultiJumpRule::default()).with_rules(at_rest());
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert_eq!(
            session.player().position,
            Vec2::new(0.0, -1001.0),
            "the last leg's side"
        );
    }

    /// `base` with the nebula around 131 changing the ship to 130, of
    /// half its top speed, as 131 is explored.
    fn slowed_over_131(base: FakePilotCatalog) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            nebulae: vec![nebula(128, (500, -100, 200, 200), "", "C130")],
            ..base
        };
        let flags2 = catalog
            .ships
            .first()
            .and_then(|(_, fields)| fields.as_ref().ok())
            .map_or(0, |fields| fields.flags2);
        let slow = ShipFields {
            speed: 300,
            flags2,
            ..FAST
        };
        catalog.ships.push((ShipId(130), Ok(slow)));
        catalog.ship_records.push(ship(130, slow));
        catalog
    }

    #[test]
    fn a_jumps_arrival_speed_is_the_class_before_an_on_explores_change_of_ship() {
        // The arrival block (@0x6bc0d-0x6bd20) runs before the arrival's
        // `OnExplore` (@0x6be12), so it reads class 128's top speed.
        let catalog = slowed_over_131(catalog());
        let mut session = jumping(&catalog, Rulebook::default());
        let fast = session.top_speed();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(session.top_speed() < fast, "the OnExplore's class");
        let speed = session.player().velocity.length();
        assert!((speed - fast).abs() < 1e-3, "{speed} against {fast}");

        let catalog = slowed_over_131(inertialess());
        let mut session = jumping(&catalog, Rulebook::default());
        let fast = session.top_speed();
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(session.top_speed() < fast, "the OnExplore's class");
        assert_eq!(session.player_speed(), fast);
        assert!(
            near(session.player().position, Vec2::new(-1350.0, 0.0), 1e-3),
            "placed before the explorations, as before"
        );
    }

    #[test]
    fn straight_after_arriving_the_player_can_jump_on_by_either_reading() {
        for rules in [Rulebook::default(), at_rest()] {
            let catalog = catalog();
            let mut session = jumping(&catalog, rules);
            session.arrive(&catalog, &mut NeverFires).expect("arrives");
            work(&mut session, &catalog, Controls::default());
            assert_eq!(
                session.begin_jump(),
                Ok(SystemId(132)),
                "{:?}",
                session.player()
            );
        }
    }
}
