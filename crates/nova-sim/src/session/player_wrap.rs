//! The player's wrap, `_WrapPlayerPosition` (@0x67efa-0x6838f): a ship past
//! the edge of the system is carried back, and what stands near it comes
//! along ([`player_wrap`](crate::player_wrap)).
//!
//! `_HandlePlayer` calls it at @0x6864d, after the tractor block
//! (@0x68436-0x6862b) and before the lost-target check (@0x68652), so here
//! [`Session::wrap_player_position`] runs between the screen's white and
//! that check (`Session::before_move`), on both of double time's passes.
//! The guard at @0x68647-0x6864b runs it only while the player's +0x50 is
//! not above 0, so never in the pre-jump's brake (1.0) or the warp
//! ([`PlayerShip::warp_stage`]). Nothing earlier in `_HandlePlayer` returns,
//! so it runs for a dying, destroyed or podded player too. Landed, no pass
//! runs. The routine plays no sound.
//!
//! Each table is carried by its owner: the ship slots and boxes
//! (`Traffic::carry`, `CargoBoxes::carry`), the shots, beams and kept slot
//! positions (`Combat::carry`), and the view's own explosions, pods and
//! smoke, which it carries from [`PassReport::wrap`]. Stellars, asteroids,
//! stars, particles and every velocity stay, as no code in the routine reads
//! them. So every asteroid on screen is culled on the wrap's pass
//! (`Asteroids::step`) and the edge spawn refills the field, the stars
//! black out until each has stepped back, and a pursuer more than 15,000
//! behind is left there.
//!
//! The rule is [`RuleKey::PlayerWrap`](crate::RuleKey::PlayerWrap): by the
//! other reading nothing wraps.
//!
//! [`PassReport::wrap`]: crate::flight_frame::PassReport::wrap

use super::{Jump, PlayerShip, Session, Space};
use crate::combat::ShipRef;
use crate::player_wrap::PlayerWrap;
use crate::rulebook::{RuleKey, RuleSource};

impl PlayerShip {
    /// Moves the player if it wraps, as `rule` reads
    /// [`RuleKey::PlayerWrap`](crate::RuleKey::PlayerWrap), `jump` under way
    /// (see the module docs): the wrap, if it did.
    fn wrap(&mut self, jump: Option<Jump>, rule: RuleSource) -> Option<PlayerWrap> {
        if rule != RuleSource::Engine || self.warp_stage(jump) > 0.0 {
            return None;
        }
        let wrap = PlayerWrap::of(self.player.position)?;
        self.player.position = wrap.carried(self.player.position);
        Some(wrap)
    }
}

impl Space {
    /// Carries the ships, boxes and fight by `wrap` (see the module docs).
    fn carry(&mut self, wrap: &PlayerWrap) {
        let left = self.traffic.carry(wrap);
        self.boxes.carry(wrap);
        self.combat
            .carry(wrap, left.into_iter().map(ShipRef::Npc).collect());
    }
}

impl Session {
    /// `_WrapPlayerPosition`, at the head of each pass (see the module
    /// docs). It is here as it moves the player and the space about it,
    /// and notes the wrap for the pass's report.
    pub(super) fn wrap_player_position(&mut self) {
        let rule = self.setup.rules.source_for(RuleKey::PlayerWrap);
        let Some(wrap) = self.ship.wrap(self.mode.jump(), rule) else {
            return;
        };
        self.space.carry(&wrap);
        self.out.player_wrap = Some(wrap);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::asteroid::Asteroid;
    use crate::catalog::SystemId;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::tests::{FIRE, OUT, armed, bound_for_131, facing_an_npc};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, npc, work, work_with};
    use crate::traffic::npc::{Npc, NpcId};

    use super::{Jump, Session};

    /// Every NPC idles.
    #[derive(Debug)]
    struct Idling;

    impl Behaviour for Idling {
        fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            Goal::Idle
        }
    }

    const PAST: Vec2 = Vec2::new(WRAP_EDGE + 1.0, 20.0);
    const SHIFT: Vec2 = Vec2::new(-WRAP_SHIFT, 0.0);

    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(session.landed(), None);
        // The first pass populates the system and sets the speed box.
        pass(&mut session, catalog);
        session
    }

    fn pass(session: &mut Session, catalog: &FakePilotCatalog) {
        work(session, catalog, Controls::default());
    }

    fn again(session: &mut Session) {
        let behaviour = crate::ai::NovaAi::default();
        session.tick_again(
            Controls::default(),
            &behaviour,
            Rules::default(),
            &mut NeverFires,
        );
    }

    #[test]
    fn a_ship_past_the_edge_comes_back_on_the_next_pass_its_velocity_and_heading_kept() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.ship.player.position = PAST;
        session.ship.player.velocity = Vec2::new(4.0, 0.0);
        session.ship.player.heading = 90.0;
        pass(&mut session, &catalog);
        let player = *session.player();
        assert_eq!(player.position, PAST + SHIFT + Vec2::new(4.0, 0.0));
        assert_eq!(player.velocity, Vec2::new(4.0, 0.0));
        assert_eq!(player.heading, 90.0);
    }

    #[test]
    fn a_ship_inside_the_edge_stays() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        let inside = Vec2::new(WRAP_EDGE - 100.0, 100.0 - WRAP_EDGE);
        session.ship.player.position = inside;
        pass(&mut session, &catalog);
        assert_eq!(session.player().position, inside);
    }

    #[test]
    fn the_ships_within_reach_keep_their_offset_and_those_beyond_stay() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.ship.player.position = PAST;
        let (near, far) = (
            PAST + Vec2::new(-200.0, 50.0),
            PAST + Vec2::new(-WRAP_EDGE * 2.0, 0.0),
        );
        for at in [near, far] {
            let mut ship = npc(0, ShipStats::new(FAST, &[]));
            ship.state.position = at;
            session.space.traffic.add_npc(ship);
        }
        pass(&mut session, &catalog);
        let places: Vec<Vec2> = session.npcs().iter().map(|n| n.state.position).collect();
        assert_eq!(places, [near + SHIFT, far]);
        assert_eq!(
            places[0] - session.player().position,
            near - PAST,
            "its offset kept"
        );
    }

    #[test]
    fn the_boxes_near_it_come_along_and_those_beyond_stay() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.ship.player.position = PAST;
        let (near, far) = (
            PAST + Vec2::new(-200.0, 50.0),
            PAST + Vec2::new(-WRAP_EDGE * 2.0, 0.0),
        );
        for at in [near, far] {
            let boxes = &mut session.space.boxes;
            boxes.spawn_scoopable(at, None, 1, &mut NeverFires);
        }
        let before = session.cargo_boxes().to_vec();
        pass(&mut session, &catalog);
        let after = session.cargo_boxes();
        assert_eq!(
            after[0].position,
            before[0].position + SHIFT + before[0].velocity
        );
        assert_eq!(after[1].position, before[1].position + before[1].velocity);
    }

    #[test]
    fn a_shot_near_it_comes_along() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 90);
        pass(&mut session, &catalog);
        session.ship.player.position = Vec2::new(WRAP_EDGE - 1.0, 0.0);
        session.ship.player.velocity = Vec2::new(5.0, 0.0);
        session.hold_trigger(FIRE);
        pass(&mut session, &catalog);
        session.hold_fire(false, false);
        let shot = session.shots()[0];
        assert!(session.player().position.x > WRAP_EDGE, "past the edge");
        pass(&mut session, &catalog);
        let moved = session.shots()[0];
        assert_eq!(moved.position, shot.position + SHIFT + shot.velocity);
    }

    #[test]
    fn the_stellars_stay() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        let places = |session: &Session| -> Vec<Vec2> {
            session
                .chart
                .sites
                .iter()
                .map(|site| site.position)
                .collect()
        };
        let before = places(&session);
        assert!(!before.is_empty());
        session.ship.player.position = PAST;
        pass(&mut session, &catalog);
        assert_eq!(places(&session), before);
    }

    /// Puts an asteroid a little ahead of the ship.
    fn rock_ahead(session: &mut Session) {
        let position = session.player().position + Vec2::new(50.0, 0.0);
        let rock = Asteroid {
            position,
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 100,
            kind: 0,
        };
        session.space.asteroids.place(rock).expect("a free slot");
    }

    #[test]
    fn an_asteroid_on_screen_stays_in_the_world_and_goes_on_the_wraps_pass() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.set_window_width(640);
        session.ship.player.position = PAST;
        rock_ahead(&mut session);
        pass(&mut session, &catalog);
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn on_double_times_second_pass_too() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.set_window_width(640);
        session.ship.warp.double_time = true;
        session.ship.player.position = PAST;
        rock_ahead(&mut session);
        again(&mut session);
        assert_eq!(session.player().position, PAST + SHIFT);
        assert_eq!(session.asteroids().count(), 0);
    }

    #[test]
    fn no_wrap_while_the_stage_is_above_0() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        let to = SystemId(131);
        let brake = Jump::PreJump {
            to,
            bearing: None,
            warp: None,
        };
        let warp = Jump::Hyperspace { to, warp: 3 };
        let mut wraps = |jump| {
            session.ship.player.position = PAST;
            session.ship.wrap(jump, RuleSource::Engine).is_some()
        };
        assert!(wraps(None), "out of a jump");
        assert!(!wraps(Some(brake)), "the brake's 1.0");
        assert!(wraps(Some(warp)), "a stage of 0");
        session.start_warp();
        session.ship.player.position = PAST;
        let staged = session.ship.wrap(Some(warp), RuleSource::Engine);
        assert_eq!(staged, None, "the warp's stage");
    }

    #[test]
    fn no_wrap_as_a_jump_begins() {
        let catalog = catalog();
        let mut session = bound_for_131(&catalog, OUT, Vec2::ZERO, 90.0);
        session.begin_jump().expect("jumps");
        session.ship.player.position = PAST;
        pass(&mut session, &catalog);
        assert!(session.player().position.x > 0.0, "the pre-jump's brake");
    }

    #[test]
    fn a_wrap_for_a_disabled_dying_or_destroyed_player() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 100,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            let catalog = catalog();
            let mut session = flying(&catalog);
            session.ship.condition = condition;
            session.ship.player.position = PAST;
            pass(&mut session, &catalog);
            assert_eq!(session.player().position, PAST + SHIFT, "{condition:?}");
        }
    }

    #[test]
    fn a_wrap_for_a_player_in_the_pod() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        session.pod = Some(100);
        session.ship.player.position = PAST;
        pass(&mut session, &catalog);
        assert!(session.player().position.x < 0.0);
    }

    #[test]
    fn no_wrap_while_landed() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        crate::testkit::land_now(&mut session).expect("lands");
        assert!(session.landed().is_some());
        session.ship.player.position = PAST;
        pass(&mut session, &catalog);
        assert_eq!(session.player().position, PAST);
    }

    #[test]
    fn by_the_other_reading_nothing_wraps() {
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::PlayerWrap, RuleSource::Bible);
        let mut session = flying(&catalog).with_rules(rules);
        session.ship.player.position = PAST;
        session
            .space
            .traffic
            .add_npc(npc(0, ShipStats::new(FAST, &[])));
        pass(&mut session, &catalog);
        assert_eq!(session.player().position, PAST);
        assert_eq!(session.out.player_wrap, None);
        assert_eq!(session.npcs()[0].state.position, Vec2::ZERO);
    }

    #[test]
    fn the_lost_target_check_runs_on_the_wraps_pass() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.select_target(crate::targeting::TargetPick::Next);
        session.space.traffic.npcs_mut()[0].cloak = crate::cloak::Cloak {
            fade: 40.0,
            direction: 0,
        };
        session.ship.player.position = PAST;
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &Idling,
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(session.player().position, PAST + SHIFT);
        assert_eq!(session.chart.target, None::<NpcId>);
    }

    #[test]
    fn the_wrap_raises_no_sound() {
        let catalog = catalog();
        let mut session = flying(&catalog);
        let quiet = {
            pass(&mut session, &catalog);
            session.take_sounds()
        };
        session.ship.player.position = PAST;
        pass(&mut session, &catalog);
        assert_eq!(session.take_sounds(), quiet);
    }
}
