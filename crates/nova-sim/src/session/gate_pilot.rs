//! The gate autopilot, by [`RuleKey::GateAutopilot`]'s other reading, the
//! new game's: J flies a plotted course gate by gate, with the player's
//! own controls, by the core's pilot ([`gate_pilot::steer`]).
//!
//! The original has no autopilot that flies to a stellar (see the core's
//! module docs), so none of this has an address of its own; it drives J
//! (`_HandlePlayer` @0x6b11e-0x6b131), the player's step and the land
//! key's dock request (`_HandlePlayerDockRequest` @0x66691-0x67ef9),
//! each as ported.
//!
//! **Engaging.** J, by the other reading, engages it when the player
//! flies (not landed, no jump under way, the ship intact and out of the
//! pod) and a gate the system shows serves the lane to the system
//! selected in nav mode 3 ([`Session::engage_gate_pilot`]): J answers
//! `Answer::GatePilot` with that system and starts no pre-jump. With no
//! such gate J is the engine's (`Session::begin_jump`), so under the gate
//! lanes it is the original's silent refusal, and with a Con link it
//! jumps. A gate wins over a hyperspace jump: where both lead, J engages
//! the autopilot. J while engaged answers the same again and changes
//! nothing, as J during the pre-jump does. The nav area reads the jump
//! clear while J would engage, and under way while it is engaged
//! (`Session::jump_readiness`).
//!
//! **The hop and the gate.** It flies to the hop: J's selection, or,
//! when that was the course's first hop as it is flown, the course's
//! first hop at each step, so a course re-plotted on the map re-routes
//! it. The gate is the nearest by squared distance, then the lower ID, of
//! those that serve the lane from the player's system to the hop
//! (`nova_sim_ai::gate_lanes::GateLanes::gates`) that the system shows,
//! that can be landed on and that the system still holds: a gate the
//! press would refuse as not landable is passed over, as it serves
//! nothing. Each step it selects that gate in nav mode 2 with the
//! brackets' fly-in, as the land key's own pick does, and only when the
//! nav doesn't hold it already, so the dock request stands; a player's
//! stellar key or click is overridden at the next step.
//!
//! **Each step** ([`Session::pilot_controls`]) the autopilot's controls
//! replace the player's for both of double time's passes (see the `step`
//! module), so every flight rule applies to it as to a player.
//!
//! **Letting go.** It disengages, with no sound, on any of these:
//!
//! - a steering key held: thrust, a turn key, reverse, face or the
//!   afterburner. The player's controls fly that step;
//! - a landing, a jump under way, the ship disabled, breaking up or
//!   destroyed, or the escape pod;
//! - the course cleared: it flies along the course, and the course is
//!   empty without an exit;
//! - no gate the system shows serves the hop (the course is kept).
//!
//! **Not saved.** It lives on the player's ship and no save keeps it, so
//! a load (`RuleKey::InFlightSave`'s reading saves in flight) flies with
//! none, as it drops held keys.

use crate::catalog::{LandingSite, StellarId, SystemId};
use crate::combat::hull::Condition;
use crate::flight::Controls;
use crate::gate_pilot::{self, Craft, Gate};
use crate::geometry::Vec2;
use crate::landing::is_landable;
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};

use super::{Chart, Session, Statics};

/// The gate autopilot while it is engaged (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GatePilot {
    /// The system it flies toward: the hop it takes next.
    pub(super) to: SystemId,
    /// Whether it flies along the course: J's selection was the course's
    /// first hop, so each step it takes the course's first hop.
    pub(super) along_course: bool,
    /// Whether this step's pilot asked for L.
    pub(super) asked_land: bool,
}

impl Session {
    /// J by [`RuleKey::GateAutopilot`]'s other reading: the system the
    /// autopilot flies toward, engaging it now if it isn't, or `None` when
    /// it doesn't engage and J is the engine's (see the module docs).
    pub(crate) fn engage_gate_pilot(&mut self) -> Option<SystemId> {
        if let Some(pilot) = self.ship.gate_pilot {
            return Some(pilot.to);
        }
        let (to, gate) = self.gate_pilot_target()?;
        let along_course = self.chart.course_shown(&self.pilot).first() == Some(&to);
        self.ship.gate_pilot = Some(GatePilot {
            to,
            along_course,
            asked_land: false,
        });
        self.chart.select_gate(gate);
        Some(to)
    }

    /// The controls the player's ship flies by this step: the
    /// autopilot's, re-targeted, while it stays engaged, else `controls`,
    /// the player's (see the module docs). It clears the step's ask for L
    /// and sets it again when the pilot asks.
    pub(super) fn pilot_controls(&mut self, controls: Controls) -> Controls {
        let Some(mut pilot) = self.ship.gate_pilot.take() else {
            return controls;
        };
        if controls != Controls::default() || !self.gate_pilot_flies() {
            return controls;
        }
        if pilot.along_course {
            match self.chart.course_shown(&self.pilot).first() {
                Some(&hop) => pilot.to = hop,
                None => return controls,
            }
        }
        let Some(site) = self
            .chart
            .serving_gate(
                &self.pilot,
                &self.statics,
                pilot.to,
                self.ship.player.position,
            )
            .copied()
        else {
            return controls;
        };
        self.chart.select_gate(site.id);
        let held = self.chart.held_here(&self.pilot, &self.statics, site.id);
        let craft = Craft {
            state: self.ship.player,
            inertialess: self
                .ship
                .player_inertialess(&self.setup)
                .then_some(self.ship.player_speed),
            handling: self
                .ship
                .flying_handling(&self.pilot, &self.setup, self.mode.jump()),
            turn: self.setup.flight_rules().player_turn,
            gain: self.speed_mult(),
        };
        let gate = Gate {
            site: &site,
            reach: self.space.landing_reach(&self.setup, &site, held),
            held,
            dock: self.port.dock,
            speed: self.setup.rules.source_for(RuleKey::LandingSpeed),
            cloaked: self.ship.cloak.cloak.is_cloaked(),
        };
        let steer = gate_pilot::steer(&craft, &gate);
        pilot.asked_land = steer.press_land;
        self.ship.gate_pilot = Some(pilot);
        steer.controls
    }

    /// Whether the gate autopilot is engaged.
    pub(crate) fn gate_pilot_engaged(&self) -> bool {
        self.ship.gate_pilot.is_some()
    }

    /// Whether J would engage the gate autopilot now.
    pub(crate) fn gate_pilot_would_engage(&self) -> bool {
        self.gate_pilot_target().is_some()
    }

    /// What J would engage the autopilot toward by
    /// [`RuleKey::GateAutopilot`]'s other reading: the system selected in
    /// nav mode 3 and the gate serving it, while the player flies.
    fn gate_pilot_target(&self) -> Option<(SystemId, StellarId)> {
        if self.setup.rules.source_for(RuleKey::GateAutopilot) == RuleSource::Engine
            || !self.gate_pilot_flies()
        {
            return None;
        }
        let NavMode::Hyperspace(Some(to)) = self.chart.nav_mode() else {
            return None;
        };
        let gate =
            self.chart
                .serving_gate(&self.pilot, &self.statics, to, self.ship.player.position)?;
        Some((to, gate.id))
    }

    /// Whether the player flies as the autopilot can fly it: not landed,
    /// no jump under way, the ship intact and out of the escape pod.
    fn gate_pilot_flies(&self) -> bool {
        self.mode.landed().is_none()
            && self.mode.jump().is_none()
            && self.ship.condition == Condition::Intact
            && self.pod.is_none()
    }
}

impl Chart {
    /// The gate the autopilot takes toward `hop` from a ship at `at`: of
    /// the gates serving the lane from the player's system to `hop`, the
    /// nearest the system shows, can be landed on and still holds, by
    /// squared distance, then the lower ID (see the module docs).
    fn serving_gate(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        hop: SystemId,
        at: Vec2,
    ) -> Option<&LandingSite> {
        let gates = self.galaxy.lanes().gates(pilot.system, hop);
        let squared = |site: &LandingSite| (site.position - at).length_squared();
        self.sites
            .iter()
            .filter(|site| gates.contains(&site.id))
            .filter(|site| is_landable(site) && self.held_here(pilot, statics, site.id))
            .min_by(|a, b| squared(a).total_cmp(&squared(b)).then(a.id.cmp(&b.id)))
    }

    /// Selects `gate` in nav mode 2 with the brackets' fly-in, as the land
    /// key's own pick does, unless the nav holds it already.
    fn select_gate(&mut self, gate: StellarId) {
        if self.nav != NavMode::Stellar(Some(gate)) {
            self.select_pressed(Some(gate));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::{Face, Turn};
    use crate::flight_frame::{Answer, FlightCommand};
    use crate::hyperspace::{JumpReadiness, JumpRefusal};
    use crate::rulebook::Rulebook;
    use crate::sound::SimSound;
    use crate::testkit::{DRIFT, HEARTH, VERGE, laned};

    /// A new pilot over [`laned`], in drift over gate 300, the links the
    /// gate lanes, by `autopilot` for [`RuleKey::GateAutopilot`] and the
    /// other reading of [`RuleKey::GateCourse`].
    fn laned_by(autopilot: RuleSource) -> Session {
        let rules = Rulebook::default()
            .with_override(RuleKey::GateLanes, RuleSource::Bible)
            .with_override(RuleKey::GateCourse, RuleSource::Bible)
            .with_override(RuleKey::GateAutopilot, autopilot);
        Session::start_ruled(&laned(), &rules).expect("starts")
    }

    /// [`laned_by`] the gate autopilot's other reading.
    pub(in crate::session) fn autopiloted() -> Session {
        laned_by(RuleSource::Bible)
    }

    /// `session` with `to` selected in nav mode 3, its sounds taken.
    fn selecting(mut session: Session, to: SystemId) -> Session {
        session.chart.set_nav(NavMode::Hyperspace(Some(to)));
        session.take_sounds();
        session
    }

    fn jump(session: &mut Session) -> Answer {
        crate::session::apply::tests::apply(session, FlightCommand::Jump)
    }

    /// [`autopiloted`], J pressed toward verge.
    fn engaged() -> Session {
        let mut session = selecting(autopiloted(), VERGE);
        assert_eq!(jump(&mut session), Answer::GatePilot(VERGE));
        session
    }

    const GATE_TO_VERGE: StellarId = StellarId(301);

    #[test]
    fn by_the_engine_j_toward_a_lane_neighbour_refuses_silently_and_engages_nothing() {
        let mut session = selecting(laned_by(RuleSource::Engine), VERGE);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        assert_eq!(
            jump(&mut session),
            Answer::JumpRefused {
                refusal: JumpRefusal::NoDestination,
                told: false,
            }
        );
        assert!(!session.take_sounds().contains(&SimSound::Refused));
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(VERGE)));
    }

    #[test]
    fn by_the_other_reading_j_engages_and_selects_the_nearest_serving_gate_with_no_jump() {
        let mut session = selecting(autopiloted(), VERGE);
        assert_eq!(jump(&mut session), Answer::GatePilot(VERGE));
        assert!(session.gate_pilot_engaged());
        // 301 at (0, -1100) is nearer than the back door 302.
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(GATE_TO_VERGE)));
        assert_eq!(session.mode.jump(), None, "no pre-jump");
        assert_eq!(session.preparing_jump(), None);
        assert!(!session.take_sounds().contains(&SimSound::Refused));
    }

    #[test]
    fn j_while_engaged_answers_the_same_and_changes_nothing() {
        let mut session = engaged();
        let before = session.clone();
        assert_eq!(jump(&mut session), Answer::GatePilot(VERGE));
        assert_eq!(session.ship.gate_pilot, before.ship.gate_pilot);
        assert_eq!(session.chart.nav, before.chart.nav);
        assert_eq!(session.mode.jump(), None);
    }

    #[test]
    fn readiness_reads_clear_before_j_and_under_way_after() {
        let mut session = selecting(autopiloted(), VERGE);
        assert_eq!(session.jump_readiness(), JumpReadiness::Clear);
        jump(&mut session);
        assert_eq!(session.jump_readiness(), JumpReadiness::Underway);
    }

    #[test]
    fn a_step_with_no_controls_flies_toward_the_gate() {
        let mut session = engaged();
        let controls = session.pilot_controls(Controls::default());
        assert_ne!(controls, Controls::default());
        assert!(session.gate_pilot_engaged());
        // At rest facing up, 301 straight up: it thrusts.
        session.ship.player.heading = 0.0;
        assert_eq!(
            session.pilot_controls(Controls::default()),
            Controls {
                thrust: true,
                ..Controls::default()
            }
        );
    }

    /// One frame's step of `session` with no controls held, double time's
    /// key as `double`.
    fn step(session: &mut Session, double: bool) {
        use crate::flight_frame::{StepInput, StepReport};
        use crate::session::StepCollaborators;
        let catalog = laned();
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = crate::chance::NeverFires;
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &behaviour,
            rules: crate::combat::Rules::default(),
            chance: &mut chance,
        };
        let input = StepInput {
            double_time_key: double,
            window_width: 1024,
            ..StepInput::default()
        };
        session.step(&input, 1, &mut collab, &mut StepReport::default());
    }

    #[test]
    fn a_step_flies_the_autopilots_controls_on_both_of_double_times_passes() {
        // The second step's thrust: the first's is clamped to the box a
        // run's first tick leaves, none.
        let thrust_of = |double| {
            let mut session = engaged();
            session.ship.player.heading = 0.0;
            session.at_gain(crate::speed_mult::SpeedMult::FULL);
            step(&mut session, double);
            let before = session.ship.player.velocity.y;
            step(&mut session, double);
            assert!(session.gate_pilot_engaged());
            before - session.ship.player.velocity.y
        };
        let single = thrust_of(false);
        assert!(single > 0.0, "it thrusts toward 301");
        let double = thrust_of(true);
        assert!(
            (double - 2.0 * single).abs() < 1e-6,
            "{double} after two passes, {single} after one"
        );
    }

    #[test]
    fn each_steering_key_disengages_and_the_players_controls_fly_that_step() {
        let keys = [
            Controls {
                thrust: true,
                ..Controls::default()
            },
            Controls {
                turn: Turn::Left,
                ..Controls::default()
            },
            Controls {
                turn: Turn::Right,
                ..Controls::default()
            },
            Controls {
                reverse: true,
                ..Controls::default()
            },
            Controls {
                face: Face::Target,
                ..Controls::default()
            },
            Controls {
                face: Face::Stellar,
                ..Controls::default()
            },
            Controls {
                afterburner: true,
                ..Controls::default()
            },
        ];
        for key in keys {
            let mut session = engaged();
            assert_eq!(session.pilot_controls(key), key, "{key:?}");
            assert!(!session.gate_pilot_engaged(), "{key:?}");
        }
    }

    #[test]
    fn a_landing_a_disablement_and_the_pod_each_disengage() {
        let mut landed = engaged();
        landed.mode = super::super::Mode::Landed(StellarId(300));
        landed.pilot_controls(Controls::default());
        assert!(!landed.gate_pilot_engaged(), "landed");
        let mut disabled = engaged();
        disabled.ship.condition = Condition::Disabled;
        assert_eq!(
            disabled.pilot_controls(Controls::default()),
            Controls::default()
        );
        assert!(!disabled.gate_pilot_engaged(), "disabled");
        let mut pod = engaged();
        pod.pod = Some(10);
        pod.pilot_controls(Controls::default());
        assert!(!pod.gate_pilot_engaged(), "pod");
    }

    /// [`autopiloted`] with the course drift → hearth → verge cued, J
    /// pressed.
    fn along_the_course() -> Session {
        let mut session = autopiloted();
        session.pilot.course = vec![HEARTH, VERGE];
        session.pilot.course_start = Some(DRIFT);
        session.cue_course();
        session.take_sounds();
        assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(HEARTH)));
        assert_eq!(jump(&mut session), Answer::GatePilot(HEARTH));
        session
    }

    #[test]
    fn with_the_course_cleared_it_disengages() {
        let mut session = along_the_course();
        session.pilot_controls(Controls::default());
        assert!(session.gate_pilot_engaged());
        session.clear_route();
        session.pilot_controls(Controls::default());
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn with_the_hops_gates_hidden_it_disengages_and_keeps_the_course() {
        let mut session = along_the_course();
        session.pilot.course = vec![VERGE];
        session
            .chart
            .sites
            .retain(|site| ![301, 302].contains(&site.id.0));
        session.pilot_controls(Controls::default());
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.course(), [VERGE]);
    }

    #[test]
    fn a_course_replotted_mid_flight_reroutes_it_to_the_new_first_hops_gate() {
        let mut session = along_the_course();
        session.pilot_controls(Controls::default());
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(300))));
        session.pilot.course = vec![VERGE];
        session.pilot_controls(Controls::default());
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(GATE_TO_VERGE)));
        assert_eq!(session.ship.gate_pilot.map(|pilot| pilot.to), Some(VERGE));
    }

    #[test]
    fn a_selection_off_the_course_flies_to_it_alone() {
        let mut session = autopiloted();
        session.pilot.course = vec![HEARTH, VERGE];
        session.pilot.course_start = Some(DRIFT);
        let mut session = selecting(session, VERGE);
        assert_eq!(jump(&mut session), Answer::GatePilot(VERGE));
        session.pilot_controls(Controls::default());
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(GATE_TO_VERGE)));
        session.clear_route();
        session.pilot_controls(Controls::default());
        assert!(session.gate_pilot_engaged(), "not along the course");
    }

    #[test]
    fn a_stellar_keys_selection_is_reselected_to_the_gate_at_the_next_step() {
        let mut session = engaged();
        session
            .chart
            .set_nav(NavMode::Stellar(Some(StellarId(302))));
        session.pilot_controls(Controls::default());
        assert!(session.gate_pilot_engaged());
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(GATE_TO_VERGE)));
    }

    #[test]
    fn the_selection_is_written_only_when_the_nav_doesnt_hold_the_gate() {
        let mut session = engaged();
        session.chart.nav_fly_in = crate::brackets::FlyIn::default();
        let settled = session.chart.nav_fly_in;
        session.pilot_controls(Controls::default());
        assert_eq!(session.chart.nav_fly_in, settled, "no fresh fly-in");
    }

    #[test]
    fn with_no_shown_gate_serving_the_selection_j_is_the_engines() {
        let mut session = autopiloted();
        session
            .chart
            .sites
            .retain(|site| ![301, 302].contains(&site.id.0));
        let mut session = selecting(session, VERGE);
        assert_eq!(session.jump_readiness(), JumpReadiness::Blocked);
        assert_eq!(
            jump(&mut session),
            Answer::JumpRefused {
                refusal: JumpRefusal::NoDestination,
                told: false,
            }
        );
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn a_gate_that_cannot_be_landed_on_serves_nothing() {
        let mut session = autopiloted();
        for site in &mut session.chart.sites {
            if site.id == GATE_TO_VERGE {
                site.flags = 0;
            }
        }
        let mut session = selecting(session, VERGE);
        assert_eq!(jump(&mut session), Answer::GatePilot(VERGE));
        // The back door, farther, serves the lane instead.
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(302))));
    }

    #[test]
    fn j_with_no_selection_or_landed_or_disabled_does_not_engage() {
        let mut none = autopiloted();
        none.chart.set_nav(NavMode::Hyperspace(None));
        assert!(!none.gate_pilot_would_engage());
        let mut disabled = selecting(autopiloted(), VERGE);
        disabled.ship.condition = Condition::Disabled;
        assert!(!disabled.gate_pilot_would_engage());
        assert!(!matches!(jump(&mut disabled), Answer::GatePilot(_)));
        let mut pod = selecting(autopiloted(), VERGE);
        pod.pod = Some(10);
        assert!(!pod.gate_pilot_would_engage());
        let mut stellar = autopiloted();
        stellar.chart.set_nav(NavMode::Stellar(Some(GATE_TO_VERGE)));
        assert!(!stellar.gate_pilot_would_engage());
    }

    // Over a stock-shaped galaxy with Con links, by `GateLanes`' engine
    // reading.

    /// [`gated`](crate::session::tests::gated) with a Con link from 130 to
    /// 133 too, which no gate serves, by the gate autopilot's other
    /// reading, its ship flown out of the jump zone.
    fn gated_session() -> (crate::testkit::FakePilotCatalog, Session) {
        let mut catalog = crate::session::tests::gated();
        for star in &mut catalog.star_map {
            if star.id == SystemId(130) {
                star.links = vec![SystemId(131), SystemId(133)];
            }
        }
        catalog.systems.push(SystemId(133));
        let rules = Rulebook::default().with_override(RuleKey::GateAutopilot, RuleSource::Bible);
        let mut session = Session::start_ruled(&catalog, &rules).expect("starts");
        crate::testkit::fly_out(&mut session, &catalog);
        session.take_sounds();
        (catalog, session)
    }

    #[test]
    fn by_con_links_j_toward_a_neighbour_no_gate_serves_still_jumps() {
        let (_catalog, session) = gated_session();
        let mut session = selecting(session, SystemId(133));
        assert_eq!(jump(&mut session), Answer::Jumped(SystemId(133)));
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn by_con_links_j_toward_a_neighbour_a_gate_serves_engages_the_autopilot() {
        let (_catalog, session) = gated_session();
        let mut session = selecting(session, SystemId(131));
        assert_eq!(jump(&mut session), Answer::GatePilot(SystemId(131)));
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(300))));
        assert_eq!(session.preparing_jump(), None);
    }
}
