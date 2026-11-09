//! What J says when the gate autopilot has no gate to fly to
//! (rdm:phase/play-the-sample/phase-10-new-game-autopilot-hud).
//!
//! Under the other reading of [`RuleKey::GateAutopilot`], J with a system
//! selected in nav mode 3 engages the gate autopilot when a gate the
//! player's system shows serves the lane there (see the `gate_pilot`
//! module). With no such gate J is the engine's, and the engine's J is
//! refused as [`JumpRefusal::NoDestination`]. The original's J tells that
//! refusal nothing with a mode-3 selection: no words, no beep
//! (`_HandlePlayer` @0x6b964-0x6b9a3; see [`Session::jump_refusal_told`]).
//!
//! By the other reading of [`RuleKey::NoGateRefusal`] the session says
//! why, as [`SimMessage::NoGateLeadsThere`], with no sound: the original's
//! branch plays none, and nothing gives a reason to add one. The answer
//! stays `JumpRefused { NoDestination, told: false }`, since a told
//! refusal beeps and shows the no-selection line. The key is read only
//! under the gate autopilot's other reading.

use crate::SimMessage;
use crate::hyperspace::JumpRefusal;
use crate::navigation::NavMode;
use crate::rulebook::{RuleKey, RuleSource};

use super::Session;

impl Session {
    /// Says [`SimMessage::NoGateLeadsThere`] when J's `refusal` is for want
    /// of a destination while a system is selected in nav mode 3, under
    /// the other readings of both [`RuleKey::GateAutopilot`] and
    /// [`RuleKey::NoGateRefusal`] (see the module docs).
    pub(super) fn say_no_gate_refusal(&mut self, refusal: JumpRefusal) {
        let rules = &self.setup.rules;
        if refusal == JumpRefusal::NoDestination
            && matches!(self.nav_mode(), NavMode::Hyperspace(Some(_)))
            && rules.source_for(RuleKey::GateAutopilot) == RuleSource::Bible
            && rules.source_for(RuleKey::NoGateRefusal) == RuleSource::Bible
        {
            self.space.say(&mut self.out, SimMessage::NoGateLeadsThere);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::flight_frame::{Answer, FlightCommand};
    use crate::session::gate_pilot::tests::laned_by;
    use crate::sound::SimSound;
    use crate::testkit::VERGE;

    /// A new pilot by [`laned_by`] `autopilot`, `no_gate` for
    /// [`RuleKey::NoGateRefusal`], with the gates to verge (301, 302)
    /// gone and `nav` set, its sounds taken.
    fn ungated(autopilot: RuleSource, no_gate: RuleSource, nav: NavMode) -> Session {
        let mut session = laned_by(autopilot);
        session.setup.rules = session
            .setup
            .rules
            .with_override(RuleKey::NoGateRefusal, no_gate);
        session
            .chart
            .sites
            .retain(|site| ![301, 302].contains(&site.id.0));
        session.chart.set_nav(nav);
        session.take_sounds();
        session
    }

    fn selecting(to: SystemId) -> NavMode {
        NavMode::Hyperspace(Some(to))
    }

    fn jump(session: &mut Session) -> Answer {
        crate::session::apply::tests::apply(session, FlightCommand::Jump)
    }

    const REFUSED: Answer = Answer::JumpRefused {
        refusal: JumpRefusal::NoDestination,
        told: false,
    };

    #[test]
    fn by_the_engine_j_no_gate_serves_says_nothing_and_beeps_nothing() {
        let mut session = ungated(RuleSource::Bible, RuleSource::Engine, selecting(VERGE));
        assert_eq!(jump(&mut session), REFUSED);
        assert_eq!(session.take_messages(), []);
        assert!(!session.take_sounds().contains(&SimSound::Refused));
    }

    #[test]
    fn by_the_other_reading_j_no_gate_serves_says_no_jump_point_leads_there() {
        let mut session = ungated(RuleSource::Bible, RuleSource::Bible, selecting(VERGE));
        assert_eq!(jump(&mut session), REFUSED);
        assert_eq!(session.take_messages(), [SimMessage::NoGateLeadsThere]);
        assert!(!session.take_sounds().contains(&SimSound::Refused));
        assert!(!session.gate_pilot_engaged());
    }

    #[test]
    fn the_line_needs_the_gate_autopilots_other_reading_and_a_selection() {
        let mut engine_pilot = ungated(RuleSource::Engine, RuleSource::Bible, selecting(VERGE));
        assert_eq!(jump(&mut engine_pilot), REFUSED);
        assert_eq!(engine_pilot.take_messages(), []);

        let mut unselected = ungated(
            RuleSource::Bible,
            RuleSource::Bible,
            NavMode::Hyperspace(None),
        );
        assert!(matches!(
            jump(&mut unselected),
            Answer::JumpRefused {
                refusal: JumpRefusal::NoDestination,
                ..
            }
        ));
        assert_eq!(unselected.take_messages(), []);

        let mut gated = laned_by(RuleSource::Bible);
        gated.setup.rules = gated
            .setup
            .rules
            .with_override(RuleKey::NoGateRefusal, RuleSource::Bible);
        gated.chart.set_nav(selecting(VERGE));
        assert_eq!(jump(&mut gated), Answer::GatePilot(VERGE));
        assert_eq!(gated.take_messages(), []);
    }

    #[test]
    fn only_a_refusal_for_want_of_a_destination_says_the_line() {
        let mut session = ungated(RuleSource::Bible, RuleSource::Bible, selecting(VERGE));
        session.say_no_gate_refusal(JumpRefusal::Tractored);
        session.say_no_gate_refusal(JumpRefusal::Landed);
        assert_eq!(session.take_messages(), []);
        session.say_no_gate_refusal(JumpRefusal::NoDestination);
        assert_eq!(session.take_messages(), [SimMessage::NoGateLeadsThere]);
    }
}
