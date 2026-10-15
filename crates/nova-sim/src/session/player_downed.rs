//! The player's ship newly disabled or newly breaking up: `_DamageShip`'s
//! player block (@0x3b0de-0x3b3f1).
//!
//! **Newly disabled** (@0x3b0fb-0x3b1d2): the player is told "Ship
//! disabled." ([`SimMessage::ShipDisabled`], `STR#` 2002 #287) with
//! `_beepSnd[1]` ([`SimSound::ShipDowned`], `snd ` 151), its armour is
//! truncated toward zero (@0x3b193-0x3b1a1), and the first unfailed
//! mission whose `Flags2` holds 0x0004 fails.
//!
//! **Newly breaking up** (@0x3b1fe-0x3b3f1): the beep again, "Ship
//! destroyed." ([`SimMessage::ShipDestroyed`], #288), and the same
//! mission check, from slot 0 again.
//!
//! For the player, `_IsDisabled` holds at armour 0 or below, so a hit
//! that takes an intact player straight to its breakup runs both parts:
//! two beeps, "Ship disabled." and then "Ship destroyed.", which replaces
//! it on the message line, the truncation, and two mission checks. The
//! breakup's check skips the slot the disabling's just failed, so a
//! one-hit breakup can fail two `Flags2` 0x0004 missions.
//!
//! **The mission check** (@0x3b1af-0x3b1d2, @0x3b3be-0x3b3f1) takes the
//! first slot, in order, holding a mission not yet failed whose `Flags2`
//! holds 0x0004 ([`FAILS_IF_DOWNED`]). It beeps ([`SimSound::DownedMissionFailed`],
//! @0x3b04e, @0x3b2f1), fails the mission at once
//! ([`Session::quick_failure`], `_QuickMissionFailure` @0x3b059,
//! @0x3b302), and then says "Mission failed."
//! ([`SimMessage::DownedMissionFailed`], #284, @0x3b0b6, @0x3b353). It
//! fails one slot at most, and a hidden mission (`Flags` 0x0400) fails
//! aloud: nothing tests the hidden flag here.
//!
//! **Left out.** `_playerRepairInhibit` = 300.0 (@0x3b186-0x3b18d) belongs
//! to rdm:phase/ship-handling-fidelity/phase-3-regeneration-and-self-repair.
//! The Captain Hector shareware nag and its `_playerKilledByHector` lock
//! (@0x3b226-0x3b2c9, @0x3b35d-0x3b365) are a roadmap non-goal; with the
//! nag left out, "Ship destroyed." always shows.
//!
//! **Once a tick.** The original tests the player's condition before and
//! after each hit; the engine decides condition once a fight tick
//! (`Combat::update_condition`), so this compares the condition before
//! and after the tick. A hit that disables and a later one in the same
//! tick that breaks the ship up make the same calls in the same order as
//! one hit from intact to breaking up, and the lines settle before the
//! tick's frame is drawn, so the player sees no difference. A hull with
//! `DeathDelay` 0 goes from intact to destroyed in one tick, which counts
//! as newly breaking up too.

use super::Session;
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::message::SimMessage;
use crate::sound::SimSound;

/// `Flags2`: the mission fails when the player's ship is disabled or
/// breaks up (`_DamageShip` @0x3b1bc, @0x3b3db: in-memory +0x5a & 4).
const FAILS_IF_DOWNED: u16 = 0x0004;

impl Session {
    /// Raises the player's "Ship disabled." and "Ship destroyed." lines,
    /// with their beeps, truncates a newly disabled player's armour, and
    /// fails its first `Flags2` 0x0004 mission for each, as the player's
    /// condition `was` before the fight's tick and is now, drawing on
    /// `chance` (see the module docs).
    pub(super) fn player_downed(&mut self, was: Condition, chance: &mut dyn Chance) {
        let now = self.condition;
        if was == Condition::Intact && now != Condition::Intact {
            self.sounds.push(SimSound::ShipDowned);
            self.messages.push(SimMessage::ShipDisabled);
            // ship-handling-fidelity phase 3 arms `_playerRepairInhibit`
            // (300.0) here (@0x3b186-0x3b18d).
            let armor = &mut self.pilot.reserves.armor.now;
            *armor = armor.trunc();
            self.fail_downed_mission(chance);
        }
        let was_up = matches!(was, Condition::Intact | Condition::Disabled);
        let down = matches!(now, Condition::Dying { .. } | Condition::Destroyed);
        if was_up && down {
            self.sounds.push(SimSound::ShipDowned);
            self.messages.push(SimMessage::ShipDestroyed);
            self.fail_downed_mission(chance);
        }
    }

    /// Fails the first slot's mission, in order, not failed already, whose
    /// `Flags2` holds [`FAILS_IF_DOWNED`], hidden or not: the beep, the
    /// quick failure, then "Mission failed." (@0x3b02b-0x3b0b6,
    /// @0x3b2ce-0x3b353), drawing on `chance`.
    fn fail_downed_mission(&mut self, chance: &mut dyn Chance) {
        let found = self
            .pilot
            .missions
            .iter()
            .filter(|(_, active)| active.failed.is_none())
            .map(|(slot, active)| (slot, self.lifecycle_record(active.mission)))
            .find(|(_, record)| record.flags2 & FAILS_IF_DOWNED != 0);
        let Some((slot, record)) = found else {
            return;
        };
        self.sounds.push(SimSound::DownedMissionFailed);
        self.quick_failure(slot, &record, chance);
        self.messages.push(SimMessage::DownedMissionFailed);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::MissionRecord;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::control::{Bit, Script};
    use crate::mission::FailedBy;
    use crate::session::tests::{DISABLING, armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, carrying, mission};

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Mission `id` with `Flags2` `flags2`, unabortable, its `OnFailure`
    /// setting bit `id` - 100.
    fn failing(id: i16, flags2: u16) -> MissionRecord {
        MissionRecord {
            flags2,
            can_abort: 0,
            on_failure: Script::parse(&format!("b{}", id - 100)),
            ..mission(id)
        }
    }

    /// A session of [`armed`] holding `missions` in flight, with no NPC,
    /// its armour at `armor`, as `condition` says, its sounds and messages
    /// taken.
    fn flying(missions: Vec<MissionRecord>, armor: f32, condition: Condition) -> Session {
        flying_in(
            &FakePilotCatalog {
                missions,
                ..armed()
            },
            armor,
            condition,
        )
    }

    fn flying_in(catalog: &FakePilotCatalog, armor: f32, condition: Condition) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.pilot.reserves.armor.now = armor;
        session.condition = condition;
        let _ = session.take_sounds();
        let _ = session.take_messages();
        session
    }

    /// Fills `slot` with mission `id`, failed already when `failed`.
    fn hold(session: &mut Session, slot: u8, id: i16, failed: bool) {
        let mut active = carrying(id, 0);
        if failed {
            active.failed = Some(FailedBy::Quick);
        }
        assert!(session.pilot.missions.put(slot, active));
    }

    /// One fight tick by `rules`: the sounds and the messages it raised.
    fn tick(session: &mut Session, rules: Rules) -> (Vec<SimSound>, Vec<SimMessage>) {
        session.tick_combat(rules, &mut NeverFires);
        (session.take_sounds(), session.take_messages())
    }

    fn failed_in(session: &Session, slot: u8) -> Option<FailedBy> {
        session
            .pilot
            .missions
            .get(slot)
            .and_then(|held| held.failed)
    }

    #[test]
    fn a_newly_disabled_player_says_ship_disabled_with_beep_151() {
        let mut session = flying(vec![], 20.0, Condition::Intact);
        assert_eq!(
            tick(&mut session, DISABLING),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDisabled])
        );
        assert_eq!(session.condition, Condition::Disabled);
    }

    #[test]
    fn an_already_disabled_player_says_nothing_more() {
        let mut session = flying(vec![], 20.0, Condition::Disabled);
        assert_eq!(tick(&mut session, DISABLING), (vec![], vec![]));
        assert_eq!(session.condition, Condition::Disabled);
    }

    #[test]
    fn a_disabled_player_breaking_up_says_ship_destroyed_once() {
        let mut session = flying(vec![], 0.0, Condition::Disabled);
        assert_eq!(
            tick(&mut session, DISABLING),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDestroyed])
        );
        assert!(matches!(session.condition, Condition::Dying { .. }));
        assert_eq!(tick(&mut session, DISABLING), (vec![], vec![]), "still");
        assert!(matches!(session.condition, Condition::Dying { .. }));
    }

    #[test]
    fn an_intact_player_broken_up_at_once_says_both_lines_in_order() {
        let mut session = flying(vec![], 0.0, Condition::Intact);
        assert_eq!(
            tick(&mut session, Rules::default()),
            (
                vec![SimSound::ShipDowned, SimSound::ShipDowned],
                vec![SimMessage::ShipDisabled, SimMessage::ShipDestroyed]
            )
        );
    }

    #[test]
    fn a_death_delay_0_hull_destroyed_at_once_still_says_ship_destroyed() {
        let mut catalog = armed();
        for hull in &mut catalog.hulls {
            hull.death_delay = 0;
        }
        let mut session = flying_in(&catalog, 0.0, Condition::Disabled);
        assert_eq!(
            tick(&mut session, Rules::default()),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDestroyed])
        );
        assert_eq!(session.condition, Condition::Destroyed);
        let mut session = flying_in(&catalog, 0.0, Condition::Intact);
        assert_eq!(
            tick(&mut session, Rules::default()),
            (
                vec![SimSound::ShipDowned, SimSound::ShipDowned],
                vec![SimMessage::ShipDisabled, SimMessage::ShipDestroyed]
            )
        );
        assert_eq!(session.condition, Condition::Destroyed);
    }

    #[test]
    fn the_newly_disabled_players_armour_is_truncated_toward_zero() {
        let mut session = flying(vec![], 7.6, Condition::Intact);
        tick(&mut session, DISABLING);
        assert_eq!(session.pilot.reserves.armor.now, 7.0);
        let mut session = flying(vec![], -3.5, Condition::Intact);
        tick(&mut session, Rules::default());
        assert_eq!(session.pilot.reserves.armor.now, -3.0, "toward zero");
        let mut session = flying(vec![], 7.6, Condition::Disabled);
        tick(&mut session, DISABLING);
        assert_eq!(session.pilot.reserves.armor.now, 7.6, "disabled before");
        let mut session = flying(vec![], -3.5, Condition::Disabled);
        tick(&mut session, Rules::default());
        assert_eq!(session.pilot.reserves.armor.now, -3.5, "only breaking up");
    }

    #[test]
    fn an_npc_disabled_or_broken_up_raises_no_ship_lines() {
        for armor in [1.0, 0.0] {
            let mut session = facing_an_npc(&armed(), 180);
            session.traffic.npcs_mut()[0].reserves.armor.now = armor;
            let _ = session.take_sounds();
            let _ = session.take_messages();
            let (sounds, messages) = tick(&mut session, Rules::default());
            assert_ne!(
                session.npcs()[0].condition,
                Condition::Intact,
                "{armor}: downed"
            );
            assert_eq!(session.condition, Condition::Intact);
            assert_eq!((sounds, messages), (vec![], vec![]), "{armor}");
        }
    }

    #[test]
    fn a_flags2_0x0004_mission_fails_when_the_player_is_disabled() {
        let mut session = flying(vec![failing(140, 0x0004)], 20.0, Condition::Intact);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            tick(&mut session, DISABLING),
            (
                vec![SimSound::ShipDowned, SimSound::DownedMissionFailed],
                vec![SimMessage::ShipDisabled, SimMessage::DownedMissionFailed]
            )
        );
        assert_eq!(failed_in(&session, 0), Some(FailedBy::Quick));
        assert!(session.pilot.control_bit(bit(40)), "its OnFailure ran");
    }

    #[test]
    fn an_abortable_flags2_0x0004_mission_ends_with_no_on_abort() {
        let record = MissionRecord {
            can_abort: 1,
            on_abort: Script::parse("b41"),
            ..failing(140, 0x0004)
        };
        let mut session = flying(vec![record], 20.0, Condition::Intact);
        hold(&mut session, 0, 140, false);
        let (sounds, messages) = tick(&mut session, DISABLING);
        assert_eq!(
            (sounds.len(), messages.last()),
            (2, Some(&SimMessage::DownedMissionFailed))
        );
        assert_eq!(session.pilot.missions.get(0), None, "freed");
        assert!(session.pilot.control_bit(bit(40)), "its OnFailure ran");
        assert!(!session.pilot.control_bit(bit(41)), "no OnAbort");
    }

    #[test]
    fn only_the_first_unfailed_flags2_0x0004_mission_fails() {
        let missions = vec![
            failing(140, 0x000b),
            failing(141, 0x0004),
            failing(143, 0x0004),
            failing(145, 0x0004),
        ];
        let mut session = flying(missions, 20.0, Condition::Intact);
        hold(&mut session, 0, 140, false);
        hold(&mut session, 1, 141, true);
        hold(&mut session, 3, 143, false);
        hold(&mut session, 5, 145, false);
        tick(&mut session, DISABLING);
        assert_eq!(failed_in(&session, 0), None);
        assert_eq!(failed_in(&session, 1), Some(FailedBy::Quick), "as it was");
        assert_eq!(failed_in(&session, 3), Some(FailedBy::Quick));
        assert_eq!(failed_in(&session, 5), None);
        let ran: Vec<u16> = [40, 41, 43, 45]
            .into_iter()
            .filter(|&n| session.pilot.control_bit(bit(n)))
            .collect();
        assert_eq!(ran, [43]);
    }

    #[test]
    fn no_mission_fails_without_the_bit() {
        let missions = vec![failing(140, 0x000b), failing(141, 0)];
        let mut session = flying(missions, 20.0, Condition::Intact);
        hold(&mut session, 0, 140, false);
        hold(&mut session, 1, 141, false);
        assert_eq!(
            tick(&mut session, DISABLING),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDisabled])
        );
        assert_eq!(
            (failed_in(&session, 0), failed_in(&session, 1)),
            (None, None)
        );
    }

    #[test]
    fn a_hidden_flags2_0x0004_mission_still_fails_aloud() {
        let record = MissionRecord {
            flags: 0x0400,
            ..failing(140, 0x0004)
        };
        let mut session = flying(vec![record], 20.0, Condition::Intact);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            tick(&mut session, DISABLING),
            (
                vec![SimSound::ShipDowned, SimSound::DownedMissionFailed],
                vec![SimMessage::ShipDisabled, SimMessage::DownedMissionFailed]
            )
        );
        assert_eq!(failed_in(&session, 0), Some(FailedBy::Quick));
    }

    #[test]
    fn a_flags2_0x0004_mission_fails_when_the_player_breaks_up() {
        let mut session = flying(vec![failing(140, 0x0004)], 0.0, Condition::Disabled);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            tick(&mut session, Rules::default()),
            (
                vec![SimSound::ShipDowned, SimSound::DownedMissionFailed],
                vec![SimMessage::ShipDestroyed, SimMessage::DownedMissionFailed]
            )
        );
        assert_eq!(failed_in(&session, 0), Some(FailedBy::Quick));
    }

    #[test]
    fn a_one_hit_breakup_fails_two_flags2_0x0004_missions() {
        let missions = vec![failing(142, 0x0004), failing(144, 0x0004)];
        let mut session = flying(missions, 0.0, Condition::Intact);
        hold(&mut session, 2, 142, false);
        hold(&mut session, 4, 144, false);
        assert_eq!(
            tick(&mut session, Rules::default()),
            (
                vec![
                    SimSound::ShipDowned,
                    SimSound::DownedMissionFailed,
                    SimSound::ShipDowned,
                    SimSound::DownedMissionFailed,
                ],
                vec![
                    SimMessage::ShipDisabled,
                    SimMessage::DownedMissionFailed,
                    SimMessage::ShipDestroyed,
                    SimMessage::DownedMissionFailed,
                ]
            )
        );
        assert_eq!(failed_in(&session, 2), Some(FailedBy::Quick));
        assert_eq!(failed_in(&session, 4), Some(FailedBy::Quick));
    }
}
