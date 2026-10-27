//! The session's first-flight tutorial (see [`tutorial`](crate::tutorial)):
//! each flight tick runs the hint block on its frame, unless the player is
//! destroyed, raising each hint as [`SimMessage::Tutorial`] with
//! [`SimSound::TutorialHint`]; and the jump key's refusals are told, or
//! not, as the tutorial and [`RuleKey::NewbieFuelRefusal`] say
//! ([`Session::jump_refusal_told`]).
//!
//! The original runs the hint block in `_HandlePlayer` after the jump key
//! (@0x7093d), and ours in the tick before the controls; the hints are
//! at most one tick apart from the original's, and no frame is drawn
//! between, so the lag is never seen.

use super::Session;
use crate::combat::hull::Condition;
use crate::hyperspace::JumpRefusal;
use crate::landing::nearest_stellar_station;
use crate::message::SimMessage;
use crate::navigation::NavMode;
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;
use crate::tutorial::Tutorial;

impl Session {
    /// How far through the first-flight tutorial the pilot is.
    #[must_use]
    pub fn tutorial(&self) -> Tutorial {
        self.tutorial
    }

    /// Whether the player is told of `refusal` when J gives it: the words
    /// and, for those [`check_jump`](crate::hyperspace::check_jump) gives,
    /// [`SimSound::Refused`] (`_HandlePlayer` @0x6b86d-0x6bab7).
    ///
    /// - The engine's own refusal, landed, is always told.
    /// - Disabled (or dying) is never told: the original's J skips the
    ///   jump block for a disabled player (`_HandlePlayer` @0x6b0f5 →
    ///   0x6bada), saying and beeping nothing.
    /// - Too close is always told (@0x6b86d-0x6b93c), and so is the
    ///   tractor's hold, on the same path (#36 in place of #42,
    ///   @0x6b8a4-0x6b8dc).
    /// - No destination is told only to a newbie, and never in nav mode 3
    ///   with a selection (@0x6b964-0x6b9a3): a seasoned pilot's J with
    ///   nothing selected is silent.
    /// - No fuel is told to a seasoned pilot (@0x6ba72-0x6bab7). A newbie,
    ///   whose J has a selection, is told nothing by the engine
    ///   (@0x6b995-0x6b9a3), and told as any pilot by the other reading of
    ///   [`RuleKey::NewbieFuelRefusal`].
    #[must_use]
    pub(crate) fn jump_refusal_told(&self, refusal: JumpRefusal) -> bool {
        let newbie = self.tutorial.is_newbie();
        match refusal {
            JumpRefusal::Landed | JumpRefusal::TooClose { .. } | JumpRefusal::Tractored => true,
            JumpRefusal::Disabled => false,
            JumpRefusal::NoDestination => {
                newbie && !matches!(self.nav_mode(), NavMode::Hyperspace(Some(_)))
            }
            JumpRefusal::NoFuel { .. } => {
                !newbie
                    || self.setup.rules.source_for(RuleKey::NewbieFuelRefusal) == RuleSource::Bible
            }
        }
    }

    /// The tutorial's part of a flight tick on `frame` (see the module
    /// docs): the hint block, with the ship intact or not, its squared
    /// distance from the centre and the nearest stellar
    /// (`_FindNearestStellar`, the one F5 selects).
    pub(super) fn tutorial_tick(&mut self, frame: i16) {
        if self.ship.condition == Condition::Destroyed {
            return;
        }
        let position = self.ship.player.position;
        let distance_squared = position.x * position.x + position.y * position.y;
        let sites = &self.chart.sites;
        let gates = self.setup.rules.source_for(RuleKey::GatePickRange);
        let hints = self.tutorial.check(
            frame,
            self.ship.condition == Condition::Intact,
            distance_squared,
            || nearest_stellar_station(position, sites, gates),
        );
        for hint in hints {
            self.space.say(&mut self.out, SimMessage::Tutorial(hint));
            self.out.sounds.push(SimSound::TutorialHint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{StellarId, SystemId};
    use crate::chance::{Chance, NeverFires};

    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::Rulebook;
    use crate::testkit::work;
    use crate::testkit::{FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, land_now};
    use crate::tutorial::TutorialHint;

    fn seasoned(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    fn new_pilot(catalog: &FakePilotCatalog) -> Session {
        let mut session = seasoned(catalog);
        session.begin(catalog, &mut Scripted::default());
        session.take_messages();
        session.take_sounds();
        session
    }

    /// Ticks once, giving the messages and sounds the tick raised.
    fn tick(session: &mut Session) -> (Vec<SimMessage>, Vec<SimSound>) {
        work(session, &crate::testkit::catalog(), Controls::default());
        (session.take_messages(), session.take_sounds())
    }

    /// Rolls 2 on five sides and 0 on any other: the greeting's lead.
    struct LeadTwo;

    impl Chance for LeadTwo {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            if n == 5 { 2 } else { 0 }
        }
    }

    #[test]
    fn a_flight_start_is_seasoned_and_begin_makes_a_new_pilot() {
        let catalog = catalog();
        let session = seasoned(&catalog);
        assert_eq!(session.tutorial(), Tutorial::SEASONED);
        assert_eq!(new_pilot(&catalog).tutorial(), Tutorial::NEW_PILOT);
    }

    #[test]
    fn a_new_pilots_first_tick_raises_the_land_hint_with_its_beep() {
        let catalog = catalog();
        let mut session = new_pilot(&catalog);
        let hint = SimMessage::Tutorial(TutorialHint::Land {
            stellar: StellarId(128),
            station: false,
        });
        assert_eq!(
            tick(&mut session),
            (vec![hint], vec![SimSound::TutorialHint])
        );
        assert_eq!(session.tutorial().flag(), -2);
        for _ in 0..200 {
            assert_eq!(tick(&mut session), (vec![], vec![]));
        }
    }

    /// The land hint names `_FindNearestStellar`'s pick, gate limit and
    /// all: by the engine, a wormhole 650 from the new pilot's (50, 50) is
    /// passed over for a planet 950 away; by the other reading it is
    /// named.
    #[test]
    fn the_land_hint_passes_over_a_far_gate_by_the_engine() {
        let mut catalog = catalog();
        catalog.sites[0].1 = vec![
            crate::session::stellar_picks::tests::wormhole(150, 50.0, 700.0),
            crate::testkit::planet(128, 50.0, 1000.0),
        ];
        for (rules, named) in [
            (Rulebook::default(), 128),
            (
                Rulebook::default().with_override(RuleKey::GatePickRange, RuleSource::Bible),
                150,
            ),
        ] {
            let mut session = seasoned(&catalog).with_rules(rules);
            session.begin(&catalog, &mut Scripted::default());
            session.take_messages();
            session.take_sounds();
            let hint = SimMessage::Tutorial(TutorialHint::Land {
                stellar: StellarId(named),
                station: false,
            });
            assert_eq!(tick(&mut session).0, vec![hint], "{rules:?}");
        }
    }

    #[test]
    fn far_enough_out_a_new_pilot_is_told_so() {
        let catalog = catalog();
        let mut session = new_pilot(&catalog);
        tick(&mut session);
        session.ship.player.position = Vec2::new(1500.0, 0.0);
        session.space.frame = 60;
        assert_eq!(
            tick(&mut session),
            (
                vec![SimMessage::Tutorial(TutorialHint::FarEnough)],
                vec![SimSound::TutorialHint]
            )
        );
        session.ship.player.position = Vec2::new(0.0, 3200.0);
        session.space.frame = 120;
        session.ship.condition = Condition::Disabled;
        assert_eq!(tick(&mut session).0, [], "not while disabled");
        session.ship.condition = Condition::Intact;
        session.space.frame = 120;
        assert_eq!(
            tick(&mut session).0,
            [SimMessage::Tutorial(TutorialHint::NothingOutHere)]
        );
    }

    #[test]
    fn a_seasoned_pilot_gets_no_hints() {
        let catalog = catalog();
        let mut session = seasoned(&catalog);
        session.take_sounds();
        session.ship.player.position = Vec2::new(0.0, 5000.0);
        for _ in 0..200 {
            assert_eq!(tick(&mut session).0, []);
        }
    }

    #[test]
    fn no_hint_while_destroyed() {
        let catalog = catalog();
        let mut session = new_pilot(&catalog);
        session.ship.condition = Condition::Destroyed;
        for _ in 0..200 {
            assert_eq!(tick(&mut session), (vec![], vec![]));
        }
        assert_eq!(session.tutorial(), Tutorial::NEW_PILOT);
    }

    #[test]
    fn a_seasoned_take_off_greets_with_the_rolled_lead() {
        let catalog = catalog();
        let mut session = seasoned(&catalog);
        land_now(&mut session).expect("lands");
        session.take_messages();
        assert_eq!(
            session.take_off(&catalog, &mut LeadTwo),
            Some(StellarId(128))
        );
        assert_eq!(
            session.take_messages(),
            [SimMessage::TookOff {
                stellar: StellarId(128),
                lead: 2
            }]
        );
        assert_eq!(session.tutorial(), Tutorial::SEASONED);
    }

    #[test]
    fn a_new_pilots_take_off_does_not_greet() {
        let catalog = catalog();
        let mut session = new_pilot(&catalog);
        land_now(&mut session).expect("lands");
        session.take_off(&catalog, &mut LeadTwo);
        assert_eq!(session.take_messages(), []);
        assert_eq!(session.tutorial().flag(), -1);
        session.take_sounds();
        session.space.frame = 0;
        assert_eq!(
            tick(&mut session),
            (
                vec![SimMessage::Tutorial(TutorialHint::Hyperspace)],
                vec![SimSound::TutorialHint]
            ),
            "the hyperspace hint follows"
        );
    }

    #[test]
    fn the_first_jump_makes_the_pilot_seasoned() {
        let catalog = catalog();
        let mut session = new_pilot(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        assert!(session.tutorial().is_newbie(), "until the jump begins");
        begin_jump_now(&mut session, &catalog).expect("jumps");
        assert_eq!(session.tutorial(), Tutorial::SEASONED);
        assert!(!session.jump_refusal_told(JumpRefusal::NoDestination));
    }

    #[test]
    fn a_hyperspace_arrival_makes_the_pilot_seasoned() {
        let catalog = catalog();
        let mut session = seasoned(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.tutorial = Tutorial::NEW_PILOT;
        session.arrive(&catalog, &mut NeverFires);
        assert_eq!(session.tutorial(), Tutorial::SEASONED);
    }

    #[test]
    fn jump_refusal_told_matrix() {
        let catalog = catalog();
        let too_close = JumpRefusal::TooClose { distance: 0.0 };
        let no_fuel = JumpRefusal::NoFuel { fuel: 0.0 };
        let no_destination = JumpRefusal::NoDestination;
        // (newbie, selected, rule, refusal, told)
        let engine = RuleSource::Engine;
        let bible = RuleSource::Bible;
        for (newbie, selected, rule, refusal, told) in [
            (false, false, engine, no_destination, false),
            (false, true, engine, no_destination, false),
            (true, false, engine, no_destination, true),
            (true, true, engine, no_destination, false),
            (false, true, engine, too_close, true),
            (true, true, engine, too_close, true),
            (false, true, engine, JumpRefusal::Tractored, true),
            (true, true, engine, JumpRefusal::Tractored, true),
            (false, true, engine, no_fuel, true),
            (false, true, bible, no_fuel, true),
            (true, true, engine, no_fuel, false),
            (true, true, bible, no_fuel, true),
            (true, false, engine, JumpRefusal::Landed, true),
            (false, false, engine, JumpRefusal::Disabled, false),
            (true, false, engine, JumpRefusal::Disabled, false),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::NewbieFuelRefusal, rule);
            let mut session = seasoned(&catalog).with_rules(rules);
            if newbie {
                session.tutorial = Tutorial::NEW_PILOT;
            }
            if selected {
                session.plot_course(SystemId(131)).expect("a route");
                assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(SystemId(131))));
            }
            let case = format!("{newbie} {selected} {rule:?} {refusal:?}");
            assert_eq!(session.jump_refusal_told(refusal), told, "{case}");
        }
        // A disabled or dying player's J is refused as disabled, untold.
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 100,
                starting: false,
            },
        ] {
            let mut session = seasoned(&catalog);
            session.tutorial = Tutorial::NEW_PILOT;
            session.ship.condition = condition;
            let refusal = session.begin_jump().expect_err("refused");
            assert_eq!(refusal, JumpRefusal::Disabled, "{condition:?}");
            assert!(!session.jump_refusal_told(refusal), "{condition:?}");
        }
    }

    /// Presses J on `session` and gives what it gave and the sounds.
    fn press_j(session: &mut Session) -> (Result<SystemId, JumpRefusal>, Vec<SimSound>) {
        session.take_sounds();
        (session.begin_jump(), session.take_sounds())
    }

    #[test]
    fn j_beeps_its_refusals_exactly_when_they_are_told() {
        let catalog = catalog();
        let refused = vec![SimSound::Refused];
        // Nothing selected.
        let mut session = seasoned(&catalog);
        assert_eq!(
            press_j(&mut session),
            (Err(JumpRefusal::NoDestination), vec![])
        );
        session.tutorial = Tutorial::NEW_PILOT;
        assert_eq!(
            press_j(&mut session),
            (Err(JumpRefusal::NoDestination), refused.clone())
        );
        // Too close.
        session.plot_course(SystemId(131)).expect("a route");
        assert_eq!(
            press_j(&mut session),
            (
                Err(JumpRefusal::TooClose { distance: 0.0 }),
                refused.clone()
            )
        );
        // No fuel, by each reading.
        for (rule, newbie_sounds) in [
            (RuleSource::Engine, vec![]),
            (RuleSource::Bible, refused.clone()),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::NewbieFuelRefusal, rule);
            let mut session = seasoned(&catalog).with_rules(rules);
            session.plot_course(SystemId(131)).expect("a route");
            session.pilot.reserves.fuel.now = 0.0;
            let no_fuel = Err(JumpRefusal::NoFuel { fuel: 0.0 });
            assert_eq!(press_j(&mut session), (no_fuel, refused.clone()));
            session.tutorial = Tutorial::NEW_PILOT;
            assert_eq!(press_j(&mut session), (no_fuel, newbie_sounds), "{rule:?}");
        }
        // The engine's own refusals.
        let mut session = seasoned(&catalog);
        session.ship.condition = Condition::Disabled;
        assert_eq!(press_j(&mut session), (Err(JumpRefusal::Disabled), vec![]));
        session.ship.condition = Condition::Dying {
            ticks_left: 100,
            starting: false,
        };
        assert_eq!(press_j(&mut session), (Err(JumpRefusal::Disabled), vec![]));
        let mut session = seasoned(&catalog);
        land_now(&mut session).expect("lands");
        assert_eq!(press_j(&mut session), (Err(JumpRefusal::Landed), vec![]));
    }
}
