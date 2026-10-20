//! The player's auto-turn out of a jump: what its turn block turns toward
//! this tick, from the face-target key and reverse, in `_HandlePlayer`'s
//! order (`EV Nova.app/Contents/MacOS/EV Nova`, i386). The turn itself is
//! [`player_turn`](crate::player_turn)'s.
//!
//! **The face-target key** (keyPrefs +0xe, `_loadKeys`' `autopilot`,
//! default A; @0x6aeba-0x6aff3), held out of a jump (+0x50 ≤ 0,
//! @0x6aed9), aims the ship ([`Face`]):
//!
//! - with a ship target (+0x70): at it, `_CalcInterceptAngle`(player,
//!   target) (@0x6af75-0x6af9e), unless Option is held and a nav
//!   selection stands, when at the selection's stellar
//!   (@0x6af1a-0x6af65);
//! - with no ship target, at the selection's stellar
//!   (@0x6afa4-0x6afea);
//! - with neither, nowhere: the turn keys work.
//!
//! "The selection's stellar" is stellar record [+0x6c]: the key reads
//! the selection as a stellar's index whatever the nav mode. In mode 2
//! that is the stellar selected. In mode 3 it is the selected system's
//! link slot, so the ship faces the stellar whose index is that slot,
//! `spöb` 128 + slot, wherever it is: [`RuleKey::FaceNavSlot`]'s engine
//! reading. Its other reading gives mode 3 no stellar to face. The key
//! has no disabled test: a disabled player's press still aims (+0x68),
//! but the turn block doesn't turn a disabled ship (@0x6c905).
//!
//! **Reverse** (keyPrefs +0x2c, @0x6c72a-0x6c899), held out of a jump by
//! a player not disabled, aims against the velocity when the ship moves
//! at 0.05 or more on either axis
//! ([`reverse_aim`](crate::player_turn::reverse_aim)), over the face
//! key's aim, as it comes later in the frame.
//!
//! Each aim is written to the session's `aim`, the original's +0x68,
//! which the double-time switch reads (see the `double_time` module).
//! With an aim, the turn keys do nothing this tick.
//!
//! Both keys set *0x3b5550, the flight key the cursor fades on, which the
//! view's held keys stand in for. Neither plays a sound.
//!
//! A stellar record's position is its `spöb`'s `xPos` and `yPos`
//! (record +4, @0x6af3f, @0x6afc0): the system's stellar's own, or, for
//! a record no stellar of the system has, its mark (see the `missions`
//! module), read whatever system lists it. A record that didn't load
//! reads as a zeroed one, at 0,0, as the missions' record reading has it.
//!
//! An inertialess ship's reverse (@0x6c770-0x6c7d3) aims at nothing: it
//! brakes the ship's speed instead (see the `inertialess` module).

use super::{Chart, Session, Setup, Statics};
use crate::catalog::StellarId;
use crate::flight::{Controls, Face};
use crate::geometry::Vec2;
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::player_turn::reverse_aim;
use crate::trig::intercept_angle;
use crate::{Condition, RuleKey, RuleSource};

/// The first stellar ID, whose record is index 0.
const FIRST_STELLAR: i16 = 128;

impl Session {
    /// The auto-turn's aim for the player's turn this tick, out of a jump,
    /// under `controls` (see the module docs): the face key's, then
    /// reverse's over it, each written to the session's aim; none when
    /// neither aims, or when the player is disabled, whose ship doesn't
    /// turn.
    pub(super) fn steer(&mut self, controls: Controls) -> Option<i16> {
        let disabled = self.ship.condition != Condition::Intact;
        let mut aim = self.face_aim(controls.face);
        // An inertialess ship's reverse brakes instead, with no aim (see
        // the `inertialess` module).
        if controls.reverse && !disabled && !self.ship.player_inertialess(&self.setup) {
            aim = reverse_aim(self.ship.player.velocity).or(aim);
        }
        if let Some(aim) = aim {
            self.ship.aim = aim;
        }
        aim.filter(|_| !disabled)
    }

    /// Where the face key, held as `face`, aims the player's ship, if
    /// anywhere (see the module docs).
    fn face_aim(&self, face: Face) -> Option<i16> {
        if face == Face::Off {
            return None;
        }
        let from = self.ship.player.position;
        let stellar = self
            .chart
            .face_stellar(&self.pilot, &self.statics, &self.setup);
        let target = self.target().map(|npc| npc.state.position);
        let at = match (target, stellar) {
            (Some(_), Some(stellar)) if face == Face::Stellar => stellar,
            (Some(target), _) => target,
            (None, stellar) => stellar?,
        };
        Some(intercept_angle(from, at))
    }
}

impl Chart {
    /// The position of stellar record [+0x6c], the nav selection read as
    /// a stellar's index whatever the mode: the stellar selected in mode
    /// 2; in mode 3, by [`RuleKey::FaceNavSlot`]'s engine reading, the
    /// stellar whose index is the selected system's link slot, and by its
    /// other reading none; none with no selection (see the module docs
    /// for where the position is read).
    fn face_stellar(&self, pilot: &Pilot, statics: &Statics, setup: &Setup) -> Option<Vec2> {
        let id = match self.nav {
            NavMode::Off | NavMode::Stellar(None) | NavMode::Hyperspace(None) => return None,
            NavMode::Stellar(Some(id)) => id,
            NavMode::Hyperspace(Some(selected)) => {
                if setup.rules.source_for(RuleKey::FaceNavSlot) == RuleSource::Bible {
                    return None;
                }
                let slot = i16::try_from(self.selected_slot(pilot, selected)?).ok()?;
                StellarId(FIRST_STELLAR + slot)
            }
        };
        let here = self.sites.iter().find(|site| site.id == id);
        Some(here.map_or_else(
            || {
                let (x, y) = statics.stellar_marks.place(id).unwrap_or_default();
                Vec2::new(f32::from(x), f32::from(y))
            },
            |site| site.position,
        ))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::Rulebook;
    use crate::catalog::{ShipId, StellarMark, SystemId};
    use crate::flight::Turn;
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, planet, star};

    /// The fake catalog flying a ship of `Maneuver` 25, 2.5° a tick, whose
    /// system 130 links to 131 and then 132, and whose stellar 129 is
    /// 131's, marked at (2000, 0), not 130's.
    fn pegasus() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.ships = vec![(
            ShipId(128),
            Ok(ShipFields {
                maneuver: 25,
                ..FAST
            }),
        )];
        catalog.systems.push(SystemId(132));
        catalog.star_map[0] = star(130, (0.0, 0.0), &[131, 132]);
        catalog.sites = vec![
            (SystemId(130), vec![planet(128, 30.0, -40.0)]),
            (
                SystemId(131),
                vec![planet(140, 0.0, 0.0), planet(129, 2000.0, 0.0)],
            ),
        ];
        catalog.marks.push(StellarMark {
            id: StellarId(129),
            place: (2000, 0),
            name: "Varia".to_owned(),
        });
        catalog
    }

    /// A session of [`pegasus`] at rest at the centre facing `heading`.
    fn flying(catalog: &FakePilotCatalog, heading: f32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.ship.settle_box(&session.pilot, &session.setup);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.ship.player.heading = heading;
        session
    }

    fn keys(turn: Turn) -> Controls {
        Controls {
            turn,
            ..Controls::default()
        }
    }

    fn reverse(turn: Turn) -> Controls {
        Controls {
            turn,
            reverse: true,
            ..Controls::default()
        }
    }

    fn face(face: Face, turn: Turn) -> Controls {
        Controls {
            face,
            turn,
            ..Controls::default()
        }
    }

    /// Adds an NPC at `at`, at rest, and targets it.
    fn target_at(session: &mut Session, at: Vec2) {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.state.position = at;
        let id = session.space.traffic.add_npc(npc);
        session.chart.target = Some(id);
    }

    #[test]
    fn a_maneuver_25_player_turns_2_a_tick() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 90.0);
        assert_eq!(session.handling().turn_rate, 2.5);
        session.tick(keys(Turn::Right));
        assert_eq!(session.ship.player.heading, 92.0);
        session.tick(keys(Turn::Left));
        session.tick(keys(Turn::Left));
        assert_eq!(session.ship.player.heading, 88.0);
    }

    #[test]
    fn by_the_other_reading_the_player_turns_its_full_rate() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 90.0)
            .with_rules(Rulebook::default().with_override(RuleKey::PlayerTurn, RuleSource::Bible));
        session.tick(keys(Turn::Right));
        assert_eq!(session.ship.player.heading, 92.5);
    }

    #[test]
    fn reverse_turns_until_within_a_turn_holding_the_turn_keys_off() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        // Moving east: behind is 270, the short way anticlockwise.
        session.ship.player.velocity = Vec2::new(0.06, 0.0);
        let held = reverse(Turn::Right);
        session.tick(held);
        assert_eq!(
            session.ship.player.heading, 358.0,
            "left, the keys held off"
        );
        assert_eq!(session.ship.aim, 270);
        for _ in 0..60 {
            session.tick(held);
        }
        let stopped = session.ship.player.heading;
        assert!(
            (270.0..=272.0).contains(&stopped),
            "within a turn: {stopped}"
        );
        session.tick(held);
        assert_eq!(
            session.ship.player.heading, stopped,
            "no snap, and no right turn"
        );
    }

    #[test]
    fn reverse_stops_aiming_once_each_axis_is_under_0_05_and_the_turn_keys_work() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        session.ship.aim = 45;
        session.ship.player.velocity = Vec2::new(0.049, -0.049);
        session.tick(reverse(Turn::Right));
        assert_eq!(session.ship.player.heading, 2.0, "the turn key");
        assert_eq!(session.ship.aim, 45, "no aim");
        session.tick(reverse(Turn::None));
        assert_eq!(session.ship.player.heading, 2.0);
    }

    #[test]
    fn the_face_key_faces_the_ship_target() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        target_at(&mut session, Vec2::new(500.0, 0.0));
        session.tick(face(Face::Target, Turn::Left));
        assert_eq!(session.ship.aim, 90);
        assert_eq!(
            session.ship.player.heading, 2.0,
            "toward it, the keys held off"
        );
        for _ in 0..60 {
            session.tick(face(Face::Target, Turn::None));
        }
        assert!((88.0..=90.0).contains(&session.ship.player.heading));
    }

    #[test]
    fn with_option_and_a_stellar_selected_the_face_key_faces_the_stellar() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        target_at(&mut session, Vec2::new(500.0, 0.0));
        session
            .chart
            .set_nav(NavMode::Stellar(Some(StellarId(128))));
        session.tick(face(Face::Stellar, Turn::None));
        assert_eq!(
            session.ship.aim,
            intercept_angle(Vec2::ZERO, Vec2::new(30.0, -40.0))
        );
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(session.ship.aim, 90, "without Option, the target");
        session.chart.set_nav(NavMode::Stellar(None));
        session.tick(face(Face::Stellar, Turn::None));
        assert_eq!(session.ship.aim, 90, "with Option but nothing selected");
    }

    #[test]
    fn with_no_ship_target_the_face_key_faces_the_stellar_selected() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        session
            .chart
            .set_nav(NavMode::Stellar(Some(StellarId(128))));
        for face_key in [Face::Target, Face::Stellar] {
            session.ship.aim = 0;
            session.tick(face(face_key, Turn::None));
            assert_eq!(
                session.ship.aim,
                intercept_angle(session.ship.player.position, Vec2::new(30.0, -40.0))
            );
        }
    }

    #[test]
    fn with_nothing_to_face_the_face_key_aims_nowhere_and_the_turn_keys_work() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        session.ship.aim = 123;
        session.tick(face(Face::Target, Turn::Right));
        assert_eq!(session.ship.aim, 123, "kept");
        assert_eq!(session.ship.player.heading, 2.0);
    }

    #[test]
    fn by_the_engine_in_mode_3_the_face_key_faces_the_stellar_whose_index_is_the_link_slot() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        // 132 is 130's second link, slot 1: stellar 129, 131's, at
        // (2000, 0), as if it stood in this system.
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(132))));
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(session.ship.aim, 90);
        assert_eq!(session.ship.player.heading, 2.0);
        // 131, slot 0: stellar 128, this system's.
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(131))));
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(
            session.ship.aim,
            intercept_angle(session.ship.player.position, Vec2::new(30.0, -40.0))
        );
    }

    #[test]
    fn by_the_other_reading_mode_3_has_no_stellar_to_face() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0)
            .with_rules(Rulebook::default().with_override(RuleKey::FaceNavSlot, RuleSource::Bible));
        session.ship.aim = 7;
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(132))));
        session.tick(face(Face::Target, Turn::Right));
        assert_eq!(session.ship.aim, 7);
        assert_eq!(session.ship.player.heading, 2.0, "the turn key");
        target_at(&mut session, Vec2::new(-500.0, 0.0));
        session.tick(face(Face::Stellar, Turn::None));
        assert_eq!(session.ship.aim, 270, "the ship target, Option or not");
    }

    #[test]
    fn a_record_that_did_not_load_is_faced_at_0_0() {
        let mut catalog = pegasus();
        catalog.marks.clear();
        catalog.unloaded.push(StellarId(129));
        let mut session = flying(&catalog, 0.0);
        session.ship.player.position = Vec2::new(100.0, 0.0);
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(132))));
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(session.ship.aim, 270, "toward the centre");
    }

    #[test]
    fn reverses_aim_beats_the_face_keys() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        target_at(&mut session, Vec2::new(500.0, 0.0));
        session.ship.player.velocity = Vec2::new(0.0, 3.0);
        let both = Controls {
            reverse: true,
            face: Face::Target,
            ..Controls::default()
        };
        session.tick(both);
        assert_eq!(session.ship.aim, 0, "against the motion, not at the target");
        assert_eq!(session.ship.player.heading, 0.0, "already facing it");
    }

    #[test]
    fn reverse_under_0_05_leaves_the_face_keys_aim_and_the_turn_keys_held_off() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        target_at(&mut session, Vec2::new(500.0, 0.0));
        session.ship.aim = 45;
        session.ship.player.velocity = Vec2::new(0.049, -0.049);
        session.tick(Controls {
            reverse: true,
            face: Face::Target,
            turn: Turn::Left,
            ..Controls::default()
        });
        assert_eq!(session.ship.aim, 90, "the target's bearing, not cleared");
        assert_eq!(
            session.ship.player.heading, 2.0,
            "toward the target, the keys held off"
        );
    }

    #[test]
    fn in_a_jump_the_face_key_and_reverse_are_ignored() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        session.ship.player.position = Vec2::new(0.0, 5000.0);
        target_at(&mut session, Vec2::new(-500.0, 5000.0));
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(131))));
        session.begin_jump().expect("jumps");
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(session.ship.aim, 90, "the jump's bearing, not the target");
        assert_eq!(session.ship.player.heading, 2.0, "turning to it");
        session.tick(reverse(Turn::Left));
        assert_eq!(session.ship.aim, 90);
        assert_eq!(session.ship.player.heading, 4.0);
    }

    #[test]
    fn a_jump_to_a_system_at_the_same_map_place_turns_to_0_before_it_begins() {
        let mut catalog = pegasus();
        catalog.star_map[1] = star(131, (0.0, 0.0), &[132]);
        // By the original's co-located reading 130 stands for 131 at their
        // shared place, so the link reaches 131 only by the other one.
        let mut session = flying(&catalog, 90.0).with_rules(
            Rulebook::default().with_override(RuleKey::ColocatedResolution, RuleSource::Bible),
        );
        session.ship.player.position = Vec2::new(0.0, 5000.0);
        session
            .chart
            .set_nav(NavMode::Hyperspace(Some(SystemId(131))));
        session.begin_jump().expect("jumps");
        assert_eq!(
            session.preparing_jump(),
            Some(SystemId(131)),
            "turning first, as _CalcInterceptAngle gives 0"
        );
        let mut ticks = 0;
        while session.jumping().is_none() {
            let before = session.ship.player.heading;
            session.tick(Controls::default());
            ticks += 1;
            assert!(ticks < 100, "never jumped: {:?}", session.ship.player);
            if session.preparing_jump().is_some() {
                assert_eq!(session.ship.aim, 0);
                assert_eq!(
                    session.ship.player.heading,
                    before - 2.0,
                    "2° a tick toward 0"
                );
            }
        }
        let heading = session.ship.player.heading;
        assert!(
            crate::player_turn::settled(
                heading,
                2.5,
                0,
                crate::player_turn::PlayerTurn::WholeDegrees
            ),
            "jumped facing {heading}"
        );
        assert!(ticks > 40, "jumped after {ticks} ticks");
    }

    #[test]
    fn a_disabled_player_pressing_the_face_key_aims_but_does_not_turn() {
        let catalog = pegasus();
        let mut session = flying(&catalog, 0.0);
        target_at(&mut session, Vec2::new(500.0, 0.0));
        session.ship.condition = Condition::Disabled;
        session.tick(face(Face::Target, Turn::None));
        assert_eq!(session.ship.aim, 90);
        assert_eq!(session.ship.player.heading, 0.0);
        session.ship.player.velocity = Vec2::new(0.0, 3.0);
        session.ship.aim = 45;
        session.tick(reverse(Turn::None));
        assert_eq!(session.ship.aim, 45, "reverse is not read while disabled");
    }
}
