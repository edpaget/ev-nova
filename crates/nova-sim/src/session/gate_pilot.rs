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
//! **Each step** ([`Session::pilot_controls`]) it re-targets (the hop,
//! the gate, the selection) and its controls replace the player's for
//! both of double time's passes (see the `step` module), so every flight
//! rule applies to it as to a player. They fly at the keys' full amounts
//! ([`Session::step_controls`]), as it holds its keys fully: the view
//! sends none while no key or stick is held, so the frame's own amounts
//! would fly them at none. A stick that disengages it flies that step
//! at its own amount. The step clears its ask for L and
//! sets it again when the core's pilot asks; the frame shows it after
//! the frame's steps (`FlightFrame::gate_pilot`,
//! [`Session::gate_pilot_shown`]).
//!
//! **Whose L is whose** ([`Session::press_land_key`]). The session
//! decides, and `FlightCommand::Land` stays a unit variant: a `Land` that
//! arrives while the step's ask stands is the autopilot's own press. It
//! acts as L does, uses up the ask, and the autopilot stays engaged
//! unless the dock request refuses it finally. Any other `Land` is the
//! player's: it acts as L does, and the autopilot lets go. A player's L
//! read in the same frame the autopilot asked counts as the autopilot's;
//! it does what the autopilot's press would have done, though the
//! player's path may open the hypergate map where the autopilot's enters
//! at once (the view's). The view sends the autopilot's press for it,
//! without L's latch, and enters the hypergate for its system without the
//! map; the view never says who pressed.
//!
//! **Cloaked.** The core's pilot never asks for L while the ship is
//! cloaked, as the press would beep (`_ShipIsCloaked` @0x66a8e): the
//! autopilot keeps steering to the gate and holds over it until the
//! cloak is off, so engaging or flying while cloaked makes no press.
//!
//! **Letting go.** It disengages, with no sound, on any of these:
//!
//! - a steering key held: thrust, a turn key, reverse, face or the
//!   afterburner. The player's controls fly that step;
//! - the player's L, a `Land` with no ask standing;
//! - a landing, a jump under way, the ship disabled, breaking up or
//!   destroyed, or the escape pod;
//! - the course cleared: it flies along the course, and the course is
//!   empty without an exit;
//! - no gate the system shows serves the hop (the course is kept);
//! - the course done: a hypergate's exit with no course left, one off the
//!   course (which restarts it), or any exit while it flies to J's
//!   selection alone ([`Session::gate_pilot_after_exit`]). Under
//!   [`RuleKey::GateCourse`]'s engine reading every exit restarts the
//!   course, so the first exit is the end;
//! - a final refusal of its own press, one it can't fly through:
//!   `NotLandable`, `Denied`, `CannotPayFee`, `Cloaked` or `Disabled`.
//!   `TooFar` and `TooFast` only mean "keep flying", and the pilot never
//!   asks for a press they would answer;
//! - a hypergate's entry refused (`Cancelled`, `NoLinks`), or a
//!   wormhole's entry, whatever it comes to.
//!
//! **After an exit along the course** [`RuleKey::GateCourse`]'s other
//! reading has dropped the hop and cued the next, and the autopilot flies
//! on toward it, reselecting its gate at the next step.
//!
//! **Not saved.** It lives on the player's ship and no save keeps it, so
//! a load (`RuleKey::InFlightSave`'s reading saves in flight) flies with
//! none, as it drops held keys.

use crate::catalog::{LandingSite, StellarId, SystemId};
use crate::combat::hull::Condition;
use crate::flight::{Amounts, Controls};
use crate::flight_frame::{GatePilotShown, StepInput};
use crate::gate_pilot::{self, Craft, Gate};
use crate::geometry::Vec2;
use crate::landing::{LandingRefusal, is_landable};
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::rulebook::{RuleKey, RuleSource};

use super::{Chart, LandPress, Session, Statics};

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

    /// [`Session::pilot_controls`] for a step under `input`, holding the
    /// amounts its controls fly at: the keys' full ones for the
    /// autopilot's, which holds its keys fully, else the frame's own
    /// (see the module docs).
    pub(super) fn step_controls(&mut self, input: &StepInput) -> Controls {
        let controls = self.pilot_controls(input.controls);
        let amounts = if self.gate_pilot_engaged() {
            Amounts::FULL
        } else {
            input.amounts
        };
        self.ship.hold_amounts(amounts);
        controls
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

    /// The land key, telling the gate autopilot's press from the
    /// player's (see the module docs): [`Session::land_requesting_by`]'s
    /// press, traffic control scanning by `law` (the `dock_scan` module),
    /// and whether it made a dock request.
    pub(crate) fn press_land_key(
        &mut self,
        law: &dyn crate::legal::LegalCode,
    ) -> (Result<LandPress, LandingRefusal>, bool) {
        let own = self.ship.gate_pilot.is_some_and(|pilot| pilot.asked_land);
        let press = self.land_requesting_by(Some(law));
        let ended = !own || press.0.is_err_and(ends_on);
        if ended {
            self.ship.gate_pilot = None;
        } else if let Some(pilot) = self.ship.gate_pilot.as_mut() {
            pilot.asked_land = false;
        }
        press
    }

    /// After a hypergate's entry, `entered` or refused: the autopilot
    /// flies on to the course's next hop, or is done (see the module
    /// docs).
    pub(super) fn gate_pilot_after_exit(&mut self, entered: bool) {
        let Some(pilot) = self.ship.gate_pilot.as_mut() else {
            return;
        };
        let next = self.chart.course_shown(&self.pilot).first().copied();
        match next {
            Some(hop) if entered && pilot.along_course => {
                pilot.to = hop;
                pilot.asked_land = false;
            }
            _ => self.ship.gate_pilot = None,
        }
    }

    /// The gate autopilot as the frame shows it, `None` while it isn't
    /// engaged.
    #[must_use]
    pub fn gate_pilot_shown(&self) -> Option<GatePilotShown> {
        self.ship.gate_pilot.map(|pilot| GatePilotShown {
            to: pilot.to,
            press_land: pilot.asked_land,
        })
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

/// Whether the dock request's `refusal` of the autopilot's own press
/// ends the autopilot: a refusal it can't fly through.
fn ends_on(refusal: LandingRefusal) -> bool {
    matches!(
        refusal,
        LandingRefusal::NotLandable { .. }
            | LandingRefusal::Denied { .. }
            | LandingRefusal::CannotPayFee { .. }
            | LandingRefusal::Cloaked
            | LandingRefusal::Disabled
    )
}

impl Chart {
    /// The gate the autopilot takes toward `hop` from a ship at `at`: of
    /// the gates serving the lane from the player's system to `hop`, the
    /// nearest the system shows, can be landed on and still holds, by
    /// squared distance, then the lower ID (see the module docs). The
    /// holder term is defensive: in flight the shown sites and the lanes
    /// already agree with the holder, and they differ only while the
    /// owners are left unset, mid-jump or landed, where the autopilot
    /// doesn't fly.
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
pub(in crate::session) mod tests {
    use super::*;
    use crate::flight::{Amounts, Face, Turn};
    use crate::flight_frame::{Answer, FlightCommand, StepInput};
    use crate::hyperspace::{JumpReadiness, JumpRefusal};
    use crate::rulebook::Rulebook;
    use crate::sound::SimSound;
    use crate::testkit::{DRIFT, HEARTH, VERGE, laned_stable};

    /// A new pilot over [`laned_stable`], in drift over gate 300, the links the
    /// gate lanes, by `autopilot` for [`RuleKey::GateAutopilot`] and the
    /// other reading of [`RuleKey::GateCourse`].
    pub(in crate::session) fn laned_by(autopilot: RuleSource) -> Session {
        let rules = Rulebook::default()
            .with_override(RuleKey::GateLanes, RuleSource::Bible)
            .with_override(RuleKey::GateCourse, RuleSource::Bible)
            .with_override(RuleKey::GateAutopilot, autopilot);
        Session::start_ruled(&laned_stable(), &rules).expect("starts")
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
        step_under(
            session,
            &StepInput {
                double_time_key: double,
                ..StepInput::default()
            },
        );
    }

    /// One frame's step of `session` under `input`, in a window 1024 wide.
    fn step_under(session: &mut Session, input: &StepInput) {
        use crate::flight_frame::StepReport;
        use crate::session::StepCollaborators;
        let catalog = laned_stable();
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = crate::chance::NeverFires;
        let mut collab = StepCollaborators {
            catalog: &catalog,
            behaviour: &behaviour,
            rules: crate::combat::Rules::default(),
            chance: &mut chance,
        };
        let input = StepInput {
            window_width: 1024,
            ..*input
        };
        session.step(&input, 1, &mut collab, &mut StepReport::default());
    }

    /// No amounts: what the view sends with no key or stick held.
    const NO_AMOUNTS: Amounts = Amounts {
        turn: 0.0,
        thrust: 0.0,
    };

    /// The thrust `session`'s second step under `input` adds, facing up
    /// at full gain: the first's is clamped to the box a run's first tick
    /// leaves, none.
    fn second_thrust(mut session: Session, input: &StepInput) -> f32 {
        session.ship.player.heading = 0.0;
        session.at_gain(crate::speed_mult::SpeedMult::FULL);
        step_under(&mut session, input);
        let before = session.ship.player.velocity.y;
        step_under(&mut session, input);
        before - session.ship.player.velocity.y
    }

    /// The view holds no amounts while no key or stick is held, as the
    /// autopilot flies; its controls fly at the keys' full amounts all the
    /// same, as it holds them fully.
    #[test]
    fn the_autopilots_controls_fly_at_full_amounts_whatever_the_frame_holds() {
        let unheld = StepInput {
            amounts: NO_AMOUNTS,
            ..StepInput::default()
        };
        let thrust = second_thrust(engaged(), &unheld);
        assert!(thrust > 0.0, "it thrusts toward 301");
        let full = second_thrust(engaged(), &StepInput::default());
        assert!((thrust - full).abs() < 1e-6, "{thrust} unheld, {full} full");
    }

    /// A stick that disengages the autopilot flies that step at its own
    /// amount, as it does with the autopilot off.
    #[test]
    fn a_stick_that_disengages_it_flies_at_its_own_amount() {
        let half = StepInput {
            controls: Controls {
                thrust: true,
                ..Controls::default()
            },
            amounts: Amounts {
                turn: 0.0,
                thrust: 0.5,
            },
            ..StepInput::default()
        };
        let mut session = engaged();
        // Engaged on the first step's autopilot, it lets go on the
        // second's stick.
        session.ship.player.heading = 0.0;
        session.at_gain(crate::speed_mult::SpeedMult::FULL);
        step_under(&mut session, &StepInput::default());
        let before = session.ship.player.velocity.y;
        step_under(&mut session, &half);
        assert!(!session.gate_pilot_engaged());
        let thrust = before - session.ship.player.velocity.y;
        let off = second_thrust(autopiloted(), &half);
        assert!(off > 0.0);
        assert!((thrust - off).abs() < 1e-6, "{thrust} engaged, {off} off");
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
        let mut landed = selecting(autopiloted(), VERGE);
        landed.mode = super::super::Mode::Landed(StellarId(300));
        assert!(!landed.gate_pilot_would_engage(), "landed");
    }

    /// A hyperspace jump to verge under way.
    fn jumping() -> super::super::Mode {
        super::super::Mode::Jumping(crate::session::Jump::Hyperspace { to: VERGE, warp: 1 })
    }

    #[test]
    fn j_with_a_jump_under_way_does_not_engage() {
        let mut session = selecting(autopiloted(), VERGE);
        session.mode = jumping();
        assert!(!session.gate_pilot_would_engage());
        assert!(!matches!(jump(&mut session), Answer::GatePilot(_)));
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn a_jump_under_way_disengages() {
        let mut session = engaged();
        session.mode = jumping();
        assert_eq!(
            session.pilot_controls(Controls::default()),
            Controls::default()
        );
        assert!(!session.gate_pilot_engaged());
    }

    /// [`autopiloted`] with drift's gates unowned (drift made visible by
    /// the hidden move, `Some(true)`), the rescues forgotten but for
    /// `kept`'s, and the rebuild awaited: the owners left unset, as only
    /// mid-jump or landed (the `owners_unset` module). The gates not kept
    /// stay among the sites and the lane, but drift doesn't hold them.
    fn owners_unset_but(kept: &[StellarId]) -> Session {
        let mut session = autopiloted();
        let mut overlay = session.chart.galaxy.overlay().clone();
        overlay.set_visible(DRIFT, Some(true));
        session.chart.galaxy.set_overlay(overlay);
        session.chart.galaxy.forget_rescues();
        let listed: Vec<_> = session
            .statics
            .listed_stellars
            .iter()
            .filter(|stellar| kept.contains(&stellar.id))
            .cloned()
            .collect();
        session.chart.galaxy.keep_rescues(&listed, DRIFT);
        session.chart.galaxy.await_rebuild();
        let lane = session.chart.galaxy.lanes().gates(DRIFT, VERGE);
        for gate in [GATE_TO_VERGE, StellarId(302)] {
            assert!(lane.contains(&gate), "{gate:?} in the lane");
            assert!(
                session.chart.sites.iter().any(|site| site.id == gate),
                "{gate:?} among the sites"
            );
            assert_eq!(
                session
                    .chart
                    .held_here(&session.pilot, &session.statics, gate),
                kept.contains(&gate),
                "{gate:?} held"
            );
        }
        session
    }

    /// The gate `serving_gate` picks toward verge from the player's ship.
    /// Called directly: the owners are left unset only mid-jump or landed,
    /// where `gate_pilot_flies` stops the session from asking, and in flight
    /// the sites and lanes already agree with the holder, so the holder term
    /// is pinned here, below the session.
    fn served(session: &Session) -> Option<StellarId> {
        session
            .chart
            .serving_gate(
                &session.pilot,
                &session.statics,
                VERGE,
                session.ship.player.position,
            )
            .map(|site| site.id)
    }

    /// The pick passes over the nearer 301, which drift doesn't hold (see
    /// [`served`] for why this asks `serving_gate` directly).
    #[test]
    fn serving_gate_passes_over_a_gate_the_system_does_not_hold() {
        let session = owners_unset_but(&[StellarId(302)]);
        assert_eq!(served(&session), Some(StellarId(302)));
    }

    /// With neither gate held, none serves (see [`served`] for why this
    /// asks `serving_gate` directly).
    #[test]
    fn serving_gate_with_neither_gate_held_gives_none() {
        let session = owners_unset_but(&[]);
        assert_eq!(served(&session), None);
    }

    // Over a stock-shaped galaxy with Con links, by `GateLanes`' engine
    // reading.

    /// [`gated`](crate::session::tests::gated) with a Con link from 130 to
    /// 133 too, which no gate serves, by the gate autopilot's other
    /// reading, its ship flown out of the jump zone.
    fn gated_session() -> (crate::testkit::FakePilotCatalog, Session) {
        gated_by(RuleSource::Bible)
    }

    /// [`gated_session`] by `autopilot` for [`RuleKey::GateAutopilot`].
    fn gated_by(autopilot: RuleSource) -> (crate::testkit::FakePilotCatalog, Session) {
        let mut catalog = crate::session::tests::gated();
        for star in &mut catalog.star_map {
            if star.id == SystemId(130) {
                star.links = vec![SystemId(131), SystemId(133)];
            }
        }
        catalog.systems.push(SystemId(133));
        let rules = Rulebook::default().with_override(RuleKey::GateAutopilot, autopilot);
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
    fn by_the_engine_j_toward_a_con_neighbour_a_gate_serves_still_jumps() {
        let (_catalog, session) = gated_by(RuleSource::Engine);
        let mut session = selecting(session, SystemId(131));
        assert_eq!(jump(&mut session), Answer::Jumped(SystemId(131)));
        assert!(!session.gate_pilot_engaged());
        assert!(session.mode.jump().is_some(), "a jump under way");
    }

    #[test]
    fn by_con_links_j_toward_a_neighbour_a_gate_serves_engages_the_autopilot() {
        let (_catalog, session) = gated_session();
        let mut session = selecting(session, SystemId(131));
        assert_eq!(jump(&mut session), Answer::GatePilot(SystemId(131)));
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(300))));
        assert_eq!(session.preparing_jump(), None);
    }

    // The frame, whose L is whose, and the exits.

    /// [`along_the_course`] with the course `hops` from drift and
    /// [`RuleKey::GateCourse`] by `course`, J pressed toward the first.
    fn coursed(course: RuleSource, hops: &[SystemId]) -> Session {
        let rules = Rulebook::default()
            .with_override(RuleKey::GateLanes, RuleSource::Bible)
            .with_override(RuleKey::GateCourse, course)
            .with_override(RuleKey::GateAutopilot, RuleSource::Bible);
        let mut session = Session::start_ruled(&laned_stable(), &rules).expect("starts");
        session.pilot.course = hops.to_vec();
        session.pilot.course_start = Some(DRIFT);
        session.cue_course();
        session.take_sounds();
        assert_eq!(jump(&mut session), Answer::GatePilot(hops[0]));
        session
    }

    fn shown(session: &Session) -> Option<GatePilotShown> {
        let mut frame = crate::flight_frame::FlightFrame::default();
        session.fill_frame(&mut frame);
        frame.gate_pilot
    }

    fn land(session: &mut Session) -> Answer {
        crate::session::apply::tests::apply(session, FlightCommand::Land)
    }

    /// Runs `command` on `session` with [`laned_stable`] shared.
    fn apply_laned(session: &mut Session, command: FlightCommand) -> Answer {
        session.apply(
            command,
            &mut crate::session::apply::ApplyCollaborators {
                catalog: Some(&laned_stable()),
                chance: &mut crate::chance::NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &crate::hail::HailOptions::default(),
            },
        )
    }

    /// `session`, over drift's gate 300 along the course, through its
    /// autopilot's two presses to the entry awaiting its pick.
    fn pressed_through(session: &mut Session) {
        session.pilot_controls(Controls::default());
        assert!(matches!(
            land(session),
            Answer::LandPress(LandPress::Outcome(
                crate::landing::LandOutcome::Requested { .. }
            ))
        ));
        session.dock_approach();
        session.pilot_controls(Controls::default());
        assert_eq!(
            land(session),
            Answer::LandPress(LandPress::AtGate {
                stellar: StellarId(300),
                kind: crate::gate::GateKind::Hypergate,
            })
        );
    }

    #[test]
    fn the_frame_shows_the_autopilot_and_press_land_only_on_a_step_it_asks() {
        let mut far = engaged();
        assert_eq!(
            shown(&far),
            Some(GatePilotShown {
                to: VERGE,
                press_land: false,
            })
        );
        far.pilot_controls(Controls::default());
        assert_eq!(shown(&far).map(|shown| shown.press_land), Some(false));
        let mut over = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        over.pilot_controls(Controls::default());
        assert_eq!(
            shown(&over),
            Some(GatePilotShown {
                to: HEARTH,
                press_land: true,
            })
        );
        assert_eq!(shown(&autopiloted()), None);
    }

    #[test]
    fn the_autopilots_own_presses_request_then_enter_and_keep_it_engaged() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        session.pilot_controls(Controls::default());
        let requested = land(&mut session);
        assert!(
            matches!(
                requested,
                Answer::LandPress(LandPress::Outcome(
                    crate::landing::LandOutcome::Requested { .. }
                ))
            ),
            "{requested:?}"
        );
        assert!(session.gate_pilot_engaged());
        assert_eq!(shown(&session).map(|shown| shown.press_land), Some(false));
        session.dock_approach();
        session.pilot_controls(Controls::default());
        assert_eq!(
            land(&mut session),
            Answer::LandPress(LandPress::AtGate {
                stellar: StellarId(300),
                kind: crate::gate::GateKind::Hypergate,
            })
        );
        assert!(session.gate_pilot_engaged());
    }

    #[test]
    fn an_ask_lasts_one_step_and_a_land_with_none_standing_is_the_players() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        session.pilot_controls(Controls::default());
        land(&mut session);
        // The request stands near: this step asks nothing.
        session.pilot_controls(Controls::default());
        assert_eq!(shown(&session).map(|shown| shown.press_land), Some(false));
        let before = session.port.dock;
        let answer = land(&mut session);
        assert!(!session.gate_pilot_engaged(), "the player's L lets go");
        assert!(
            matches!(
                answer,
                Answer::LandRefused {
                    refusal: LandingRefusal::TooFar { .. },
                    ..
                }
            ),
            "{answer:?}"
        );
        assert_eq!(session.port.dock, before, "acts as L does");
    }

    #[test]
    fn a_players_land_with_no_ask_ever_lets_go() {
        let mut session = engaged();
        session.pilot_controls(Controls::default());
        land(&mut session);
        assert!(!session.gate_pilot_engaged());
    }

    /// [`coursed`] toward hearth with drift's gate 300 changed by
    /// `change`, as shown and as listed, its first step asking for L.
    fn asking_at(change: impl Fn(&mut crate::catalog::LandingSite)) -> Session {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        let chart = &mut session.chart;
        for site in chart.sites.iter_mut().chain(&mut chart.listed) {
            if site.id == StellarId(300) {
                change(site);
            }
        }
        session.pilot_controls(Controls::default());
        assert_eq!(shown(&session).map(|shown| shown.press_land), Some(true));
        session
    }

    #[test]
    fn a_denied_press_disengages() {
        let mut session = asking_at(|site| site.min_status = 100);
        let answer = land(&mut session);
        assert!(
            matches!(
                answer,
                Answer::LandRefused {
                    refusal: LandingRefusal::Denied { .. },
                    ..
                }
            ),
            "{answer:?}"
        );
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn a_fee_the_pilot_cannot_pay_disengages() {
        let mut session = asking_at(|site| site.fee = 1_000_000);
        session.pilot.cash = 0;
        let answer = land(&mut session);
        assert!(
            matches!(
                answer,
                Answer::LandRefused {
                    refusal: LandingRefusal::CannotPayFee { .. },
                    ..
                }
            ),
            "{answer:?}"
        );
        assert!(!session.gate_pilot_engaged());
    }

    const CLOAKED: crate::cloak::Cloak = crate::cloak::Cloak {
        fade: crate::cloak::FADE_FULL,
        direction: 0,
    };

    #[test]
    fn engaging_or_flying_while_cloaked_makes_no_press_and_keeps_steering() {
        let mut session = selecting(autopiloted(), HEARTH);
        session.ship.cloak.cloak = CLOAKED;
        assert_eq!(jump(&mut session), Answer::GatePilot(HEARTH));
        session.pilot_controls(Controls::default());
        assert!(session.gate_pilot_engaged(), "keeps flying");
        assert_eq!(shown(&session).map(|shown| shown.press_land), Some(false));
        let mut flying = engaged();
        flying.ship.cloak.cloak = CLOAKED;
        flying.ship.player.heading = 0.0;
        assert_eq!(
            flying.pilot_controls(Controls::default()),
            Controls {
                thrust: true,
                ..Controls::default()
            },
            "steers on"
        );
        assert!(!flying.ship.gate_pilot.is_some_and(|pilot| pilot.asked_land));
    }

    #[test]
    fn a_final_refusal_of_its_own_press_disengages_and_a_keep_flying_one_does_not() {
        for (refusal, engaged_after) in [
            (
                LandingRefusal::NotLandable {
                    stellar: StellarId(300),
                    station: false,
                },
                false,
            ),
            (LandingRefusal::Cloaked, false),
            (LandingRefusal::Disabled, false),
            (
                LandingRefusal::TooFar {
                    stellar: StellarId(300),
                    station: false,
                },
                true,
            ),
            (
                LandingRefusal::TooFast {
                    stellar: StellarId(300),
                    station: false,
                    speed: 1.0,
                },
                true,
            ),
        ] {
            assert_eq!(!ends_on(refusal), engaged_after, "{refusal:?}");
        }
    }

    #[test]
    fn after_an_exit_along_the_course_it_flies_on_to_the_next_hops_gate() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        pressed_through(&mut session);
        assert!(matches!(
            apply_laned(&mut session, FlightCommand::EnterHypergate(Some(HEARTH))),
            Answer::Arrived { system: HEARTH, .. }
        ));
        assert!(session.gate_pilot_engaged());
        assert_eq!(shown(&session).map(|shown| shown.to), Some(VERGE));
        session.pilot_controls(Controls::default());
        // Out of 310 at (-1200, 0): 312 at (0, 1300) is nearer than 311.
        assert_eq!(session.nav_mode(), NavMode::Stellar(Some(StellarId(312))));
    }

    #[test]
    fn the_last_hops_exit_disengages_with_the_course_empty_and_the_nav_off() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH]);
        pressed_through(&mut session);
        apply_laned(&mut session, FlightCommand::EnterHypergate(Some(HEARTH)));
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.course(), []);
        assert_eq!(session.nav_mode(), NavMode::Off);
    }

    #[test]
    fn by_the_engines_gate_course_the_first_exit_disengages() {
        let mut session = coursed(RuleSource::Engine, &[HEARTH, VERGE]);
        pressed_through(&mut session);
        apply_laned(&mut session, FlightCommand::EnterHypergate(Some(HEARTH)));
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn an_exit_toward_its_selection_alone_is_done() {
        let mut session = selecting(autopiloted(), HEARTH);
        assert_eq!(jump(&mut session), Answer::GatePilot(HEARTH));
        pressed_through(&mut session);
        apply_laned(&mut session, FlightCommand::EnterHypergate(Some(HEARTH)));
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn a_cancelled_entry_disengages() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        pressed_through(&mut session);
        assert!(matches!(
            apply_laned(&mut session, FlightCommand::EnterHypergate(None)),
            Answer::GateRefused(_)
        ));
        assert!(!session.gate_pilot_engaged());
        assert_eq!(session.course(), [HEARTH, VERGE]);
    }

    #[test]
    fn an_entry_with_none_pending_leaves_it_engaged() {
        let mut session = coursed(RuleSource::Bible, &[HEARTH, VERGE]);
        apply_laned(&mut session, FlightCommand::EnterHypergate(Some(HEARTH)));
        assert!(session.gate_pilot_engaged());
    }

    #[test]
    fn a_wormholes_entry_disengages() {
        let holes = crate::session::tests::wormholes();
        let mut session = crate::session::tests::at_wormhole(&holes);
        session.ship.gate_pilot = Some(GatePilot {
            to: SystemId(131),
            along_course: false,
            asked_land: false,
        });
        let _ = session.enter_wormhole(&holes, &mut crate::chance::NeverFires);
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn stepped_from_drift_it_asks_to_request_then_to_enter_within_a_cap() {
        let mut session = engaged();
        session.at_gain(crate::speed_mult::SpeedMult::FULL);
        let mut asked = 0;
        for _ in 0..3_000 {
            step(&mut session, false);
            assert!(session.gate_pilot_engaged());
            if shown(&session).is_some_and(|shown| shown.press_land) {
                asked += 1;
                let answer = land(&mut session);
                if let Answer::LandPress(LandPress::AtGate { stellar, .. }) = answer {
                    assert_eq!(stellar, GATE_TO_VERGE);
                    assert_eq!(asked, 2, "one press to request, one to enter");
                    assert!(session.gate_pilot_engaged());
                    return;
                }
                assert!(
                    matches!(
                        answer,
                        Answer::LandPress(LandPress::Outcome(
                            crate::landing::LandOutcome::Requested { .. }
                        ))
                    ),
                    "{answer:?}"
                );
            }
        }
        panic!("never asked to enter");
    }
}
