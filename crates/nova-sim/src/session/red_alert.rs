//! The session's Red Alert (see [`red_alert`](crate::red_alert)): each
//! flight tick checks the threat on a due frame, unless the player is
//! destroyed (the death path returns before the check, @0x68838-0x6888c),
//! sounding [`SimSound::RedAlert`] on a new threat unless the escape pod
//! is counting down (+0xc908 > 0, @0x68cd3-0x68cdb), and steps the icon on
//! every tick. A dying player out of the pod still hears it: the gate is
//! the pod countdown, and `_IsPlayerThreatened` (@0x82442) has no dying
//! test. Landing and hyperspace arrival clear the icon.
//!
//! In double time the check runs on both passes, on the same frame
//! (`_HandlePlayer` @0x68c79), but the blink steps on the advance pass
//! alone, as `_HandleAlertSprite` runs only there (`_DoPlayGameWork`
//! @0x44518). An alert raised on the advance pass keeps the single-speed
//! pattern. The second pass alerts only for a threat that came during
//! the first, and that alert shows from the next tick: `cïcn` 18000 for
//! 5 ticks, then each frame by turns every 5, 150 ticks in all.
//!
//! The original checks in `_HandlePlayer`, before the AI moves its ships
//! that frame, while ours reads the NPCs as the tick finds them, after
//! the last tick's traffic moved. The answer is at most one tick apart
//! inside the 60-frame check, and no frame is drawn between, so the lag
//! is never seen.

use super::Session;
use crate::combat::hull::Condition;
use crate::red_alert::{AlertIcon, alert_check_due};
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;

impl Session {
    /// Whether any ship in the system threatens the player
    /// (`_IsPlayerThreatened` @0x82442-0x82482): any NPC for which
    /// [`Npc::threatens_player_among`] the system's NPCs holds, so a ship
    /// attacking one the player leads counts (@0x7f572-0x7f590), nothing
    /// else.
    ///
    /// [`Npc::threatens_player_among`]: crate::traffic::npc::Npc::threatens_player_among
    #[must_use]
    pub fn player_threatened(&self) -> bool {
        let npcs = self.npcs();
        npcs.iter().any(|n| n.threatens_player_among(npcs))
    }

    /// The Red Alert icon frame shown, if any.
    #[must_use]
    pub fn red_alert_icon(&self) -> Option<AlertIcon> {
        self.red_alert.icon()
    }

    /// Says whether the player's sound effects are quiet: off, or at
    /// level 1 or less, which by [`RuleKey::AlertBlink`]'s engine reading
    /// is when a Red Alert blinks its icon. Not quiet until it says, as
    /// the default preferences play sound at full level.
    pub fn set_sound_quiet(&mut self, quiet: bool) {
        self.setup.sound_quiet = quiet;
    }

    /// The Red Alert's check on `frame`, which runs on both of double
    /// time's passes (see the module docs).
    pub(super) fn red_alert_check(&mut self, frame: i16) {
        if self.condition != Condition::Destroyed && alert_check_due(frame) {
            let blinks = self.setup.sound_quiet
                || self.setup.rules.source_for(RuleKey::AlertBlink) != RuleSource::Engine;
            if self
                .red_alert
                .check(self.player_threatened(), self.pod.is_some(), blinks)
            {
                self.sounds.push(SimSound::RedAlert);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::flight::Controls;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{gated, wormholes};
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, begin_jump_now, catalog, fly_out, land_now, npc};
    use crate::traffic::npc::NpcId;
    use crate::{AlertIcon, catalog::SystemId};

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog).expect("starts")
    }

    /// Adds an NPC attacking the player and gives its number.
    fn attacker(session: &mut Session) -> NpcId {
        session.traffic.add_npc(crate::traffic::npc::Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..npc(0, ShipStats::default())
        })
    }

    fn calm(session: &mut Session, id: NpcId) {
        let npc = session
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == id);
        npc.expect("there").goal = Goal::Idle;
    }

    /// Ticks once, giving the Red Alerts it sounded.
    fn alerts(session: &mut Session) -> usize {
        session.tick(Controls::default());
        session
            .take_sounds()
            .iter()
            .filter(|&&sound| sound == SimSound::RedAlert)
            .count()
    }

    /// Ticks `ticks` times, giving the Red Alerts sounded.
    fn alerts_over(session: &mut Session, ticks: usize) -> usize {
        (0..ticks).map(|_| alerts(session)).sum()
    }

    /// The icon after each of `ticks` ticks.
    fn icons(session: &mut Session, ticks: usize) -> Vec<Option<AlertIcon>> {
        (0..ticks)
            .map(|_| {
                session.tick(Controls::default());
                session.red_alert_icon()
            })
            .collect()
    }

    /// The pattern from the trigger tick: 18000, then 18001 and 18000 by
    /// turns every 5 ticks, 146 ticks shown.
    fn pattern(tick: usize) -> Option<AlertIcon> {
        match tick {
            146.. => None,
            1.. if ((tick - 1) / 5).is_multiple_of(2) => Some(AlertIcon::Dark),
            _ => Some(AlertIcon::Red),
        }
    }

    /// A threatened session, quiet, at frame 1: its tick at frame 60 is
    /// the 60th.
    fn threatened(catalog: &FakePilotCatalog) -> Session {
        let mut session = session(catalog);
        session.set_sound_quiet(true);
        attacker(&mut session);
        session.frame = 1;
        session
    }

    #[test]
    fn the_player_is_threatened_by_any_npc_that_threatens_it() {
        let mut session = session(&catalog());
        assert!(!session.player_threatened());
        session.traffic.add_npc(npc(0, ShipStats::default()));
        assert!(!session.player_threatened());
        attacker(&mut session);
        assert!(session.player_threatened());
    }

    #[test]
    fn a_ship_attacking_one_the_player_leads_sounds_the_red_alert() {
        // `_IsThreatToPlayer`'s led clause (@0x7f572-0x7f590).
        let mut session = session(&catalog());
        session.set_sound_quiet(true);
        let escort = session.traffic.add_npc(crate::traffic::npc::Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            ..npc(0, ShipStats::default())
        });
        let stranger = session.traffic.add_npc(npc(0, ShipStats::default()));
        let hunter = session.traffic.add_npc(crate::traffic::npc::Npc {
            goal: Goal::Attack(ShipRef::Npc(stranger)),
            ..npc(0, ShipStats::default())
        });
        assert!(!session.player_threatened(), "a stranger's foe");
        let foe = session
            .traffic
            .npcs_mut()
            .iter_mut()
            .find(|npc| npc.id == hunter)
            .expect("there");
        foe.goal = Goal::Attack(ShipRef::Npc(escort));
        assert!(session.player_threatened());
        session.frame = 1;
        assert_eq!(alerts_over(&mut session, 60), 1);
    }

    #[test]
    fn a_threat_sounds_the_red_alert_on_the_next_check_frame_only() {
        let mut session = threatened(&catalog());
        session.take_sounds();
        assert_eq!(alerts_over(&mut session, 59), 0, "frames 1 to 59");
        session.tick(Controls::default());
        assert_eq!(session.take_sounds(), [SimSound::RedAlert], "frame 60");
        assert_eq!(alerts_over(&mut session, 60), 0, "frames 61 to 120");
    }

    #[test]
    fn the_red_alert_icon_follows_the_pattern_when_sound_is_quiet() {
        let mut session = threatened(&catalog());
        assert!(icons(&mut session, 59).iter().all(Option::is_none));
        let shown = icons(&mut session, 200);
        let expected: Vec<_> = (0..200).map(pattern).collect();
        assert_eq!(shown, expected);
    }

    #[test]
    fn a_loud_player_hears_the_alert_without_the_icon() {
        let mut session = threatened(&catalog());
        session.set_sound_quiet(false);
        assert_eq!(alerts_over(&mut session, 60), 1);
        assert!(icons(&mut session, 200).iter().all(Option::is_none));
    }

    #[test]
    fn the_alert_blink_rule_shows_the_icon_at_any_volume() {
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::AlertBlink, RuleSource::Bible);
        let mut session = session(&catalog).with_rules(rules);
        attacker(&mut session);
        session.frame = 1;
        assert_eq!(alerts_over(&mut session, 60), 1);
        assert_eq!(session.red_alert_icon(), Some(AlertIcon::Red));
    }

    #[test]
    fn staying_threatened_sounds_once() {
        let mut session = threatened(&catalog());
        assert_eq!(alerts_over(&mut session, 600), 1);
    }

    #[test]
    fn no_threat_no_alert() {
        let mut session = session(&catalog());
        session.set_sound_quiet(true);
        session.frame = 1;
        for _ in 0..600 {
            assert_eq!(alerts(&mut session), 0);
            assert_eq!(session.red_alert_icon(), None);
        }
    }

    #[test]
    fn a_check_without_a_threat_re_arms_it() {
        let mut session = threatened(&catalog());
        let id = session.npcs()[0].id;
        assert_eq!(alerts_over(&mut session, 60), 1, "frame 60");
        calm(&mut session, id);
        assert_eq!(alerts_over(&mut session, 60), 0, "frame 120 finds none");
        session.traffic.npcs_mut()[0].goal = Goal::Attack(ShipRef::Player);
        assert_eq!(alerts_over(&mut session, 60), 1, "frame 180 finds it");
    }

    #[test]
    fn a_threat_between_two_checks_is_never_seen() {
        let mut session = session(&catalog());
        session.frame = 1;
        assert_eq!(alerts_over(&mut session, 60), 0, "frames 1 to 60");
        let id = attacker(&mut session);
        assert_eq!(alerts_over(&mut session, 59), 0, "frames 61 to 119");
        calm(&mut session, id);
        assert_eq!(alerts_over(&mut session, 600), 0);
    }

    #[test]
    fn the_check_after_frame_1020_falls_five_ticks_later_at_0() {
        let mut session = threatened(&catalog());
        session.frame = 1021;
        assert_eq!(alerts_over(&mut session, 4), 0, "1021 to 1024");
        assert_eq!(alerts(&mut session), 1, "0");
    }

    #[test]
    fn the_red_alert_is_silent_while_the_pod_counts_down() {
        let mut session = threatened(&catalog());
        session.pod = Some(crate::POD_TICKS);
        assert_eq!(alerts_over(&mut session, 60), 0);
        assert_eq!(session.red_alert_icon(), None);
        session.pod = None;
        assert_eq!(alerts_over(&mut session, 120), 0, "the edge is spent");
    }

    // Double time.

    /// Runs the second pass on the frame the last tick ran on.
    fn again(session: &mut Session) {
        session.tick_again(
            Controls::default(),
            &crate::ai::NovaAi::default(),
            crate::Rules::default(),
            &mut NeverFires,
        );
    }

    /// Ticks once in double time, giving the Red Alerts each pass sounded.
    fn doubled_alerts(session: &mut Session) -> (usize, usize) {
        let advance = alerts(session);
        again(session);
        let sounded = session.take_sounds();
        (
            advance,
            sounded.iter().filter(|&&s| s == SimSound::RedAlert).count(),
        )
    }

    /// The icon after each of `ticks` ticks in double time.
    fn doubled_icons(session: &mut Session, ticks: usize) -> Vec<Option<AlertIcon>> {
        (0..ticks)
            .map(|_| {
                session.tick(Controls::default());
                again(session);
                session.red_alert_icon()
            })
            .collect()
    }

    #[test]
    fn the_second_pass_checks_on_the_same_frame_and_shows_from_the_next_tick() {
        let mut session = session(&catalog());
        session.set_sound_quiet(true);
        session.frame = 1;
        for _ in 0..59 {
            assert_eq!(doubled_alerts(&mut session), (0, 0));
        }
        assert_eq!(alerts(&mut session), 0, "frame 60 finds no threat");
        attacker(&mut session);
        again(&mut session);
        assert_eq!(
            session.take_sounds(),
            [SimSound::RedAlert],
            "frame 60 again"
        );
        assert_eq!(session.red_alert_icon(), None, "no step on the second pass");
        let shown = doubled_icons(&mut session, 200);
        let expected: Vec<_> = (1..=200)
            .map(|tick: usize| match tick {
                151.. => None,
                _ if ((tick - 1) / 5).is_multiple_of(2) => Some(AlertIcon::Red),
                _ => Some(AlertIcon::Dark),
            })
            .collect();
        assert_eq!(shown, expected);
        let count = |icon| shown.iter().filter(|&&shown| shown == Some(icon)).count();
        assert_eq!((count(AlertIcon::Red), count(AlertIcon::Dark)), (75, 75));
    }

    #[test]
    fn an_advance_pass_alert_keeps_the_146_tick_pattern_in_double_time() {
        let mut session = threatened(&catalog());
        assert!(doubled_icons(&mut session, 59).iter().all(Option::is_none));
        let shown = doubled_icons(&mut session, 200);
        let expected: Vec<_> = (0..200).map(pattern).collect();
        assert_eq!(shown, expected);
    }

    #[test]
    fn a_standing_threat_does_not_alert_again_on_the_second_pass() {
        let mut session = threatened(&catalog());
        session.take_sounds();
        let sounded: Vec<(usize, usize)> = (0..600).map(|_| doubled_alerts(&mut session)).collect();
        assert_eq!(sounded[59], (1, 0), "frame 60's advance pass");
        assert_eq!(
            sounded.iter().map(|(a, b)| a + b).sum::<usize>(),
            1,
            "once in all"
        );
    }

    #[test]
    fn the_second_pass_never_steps_the_blink() {
        let mut session = threatened(&catalog());
        assert_eq!(alerts_over(&mut session, 64), 1, "to frame 64");
        session.tick(Controls::default());
        assert_eq!(session.pass_frame, 65, "a step frame");
        let blink = session.red_alert;
        again(&mut session);
        assert_eq!(session.red_alert, blink);
    }

    #[test]
    fn a_dying_player_out_of_the_pod_hears_the_red_alert() {
        let mut session = threatened(&catalog());
        session.condition = Condition::Dying {
            ticks_left: 500,
            starting: false,
        };
        assert_eq!(alerts_over(&mut session, 60), 1);
        assert_eq!(session.red_alert_icon(), Some(AlertIcon::Red));
    }

    #[test]
    fn a_destroyed_player_is_not_checked() {
        let mut session = threatened(&catalog());
        session.condition = Condition::Destroyed;
        assert_eq!(alerts_over(&mut session, 60), 0);
        session.condition = Condition::Intact;
        assert_eq!(
            alerts_over(&mut session, 60),
            1,
            "never checked, so the threat is still new"
        );
    }

    #[test]
    fn a_destroyed_player_still_sees_the_blink_run_out() {
        let mut session = threatened(&catalog());
        assert_eq!(alerts_over(&mut session, 60), 1);
        session.condition = Condition::Destroyed;
        let shown = icons(&mut session, 200);
        let expected: Vec<_> = (1..201).map(pattern).collect();
        assert_eq!(shown, expected);
    }

    #[test]
    fn landing_clears_the_icon_but_not_the_threat() {
        let catalog = catalog();
        let mut session = threatened(&catalog);
        assert_eq!(alerts_over(&mut session, 60), 1);
        assert!(session.red_alert_icon().is_some());
        land_now(&mut session).expect("lands");
        assert_eq!(session.red_alert_icon(), None);
        session.take_off(&catalog, &mut NeverFires);
        attacker(&mut session);
        assert_eq!(alerts_over(&mut session, 300), 0, "still threatened");
        assert_eq!(session.red_alert_icon(), None);
    }

    #[test]
    fn hyperspace_arrival_clears_the_icon_but_not_the_threat() {
        let catalog = catalog();
        let mut session = session(&catalog);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        let mut session = alerted(session);
        assert!(session.red_alert_icon().is_some(), "in hyperspace");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_eq!(session.red_alert_icon(), None);
        attacker(&mut session);
        session.take_sounds();
        assert_eq!(alerts_over(&mut session, 300), 0, "still threatened");
        assert_eq!(session.red_alert_icon(), None);
    }

    /// `session` threatened and quiet at frame 1, ticked to its alert.
    fn alerted(mut session: Session) -> Session {
        session.set_sound_quiet(true);
        attacker(&mut session);
        session.frame = 1;
        assert_eq!(alerts_over(&mut session, 60), 1);
        session
    }

    /// Presses L, runs the dock approach and presses L again, giving the
    /// gate entry it awaits.
    fn press_land_twice(session: &mut Session) {
        session.land().expect("clearance");
        session.dock_approach();
        assert!(matches!(
            session.land(),
            Ok(crate::session::LandPress::AtGate { .. })
        ));
    }

    #[test]
    fn a_hypergate_keeps_the_icon_running() {
        let catalog = gated();
        let mut session = alerted(session(&catalog));
        press_land_twice(&mut session);
        let entered = session.enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires);
        assert_eq!(entered, Ok(SystemId(131)));
        assert_eq!(session.red_alert_icon(), Some(AlertIcon::Red));
        assert_eq!(icons(&mut session, 1), [Some(AlertIcon::Dark)]);
    }

    #[test]
    fn a_wormhole_keeps_the_icon_running() {
        let catalog = wormholes();
        let mut session = alerted(session(&catalog));
        press_land_twice(&mut session);
        let mut chance = crate::testkit::Scripted::rolling(&[0]);
        assert_eq!(
            session.enter_wormhole(&catalog, &mut chance),
            Ok(SystemId(131))
        );
        assert_eq!(session.red_alert_icon(), Some(AlertIcon::Red));
        assert_eq!(icons(&mut session, 1), [Some(AlertIcon::Dark)]);
    }
}
