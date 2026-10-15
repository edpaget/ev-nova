//! The player's ship newly disabled or newly breaking up: `_DamageShip`'s
//! player block (@0x3b0de-0x3b3f1).
//!
//! **Newly disabled** (@0x3b0fb-0x3b1d2): the player is told "Ship
//! disabled." ([`SimMessage::ShipDisabled`], `STR#` 2002 #287) with
//! `_beepSnd[1]` ([`SimSound::ShipDowned`], `snd ` 151), and the first
//! unfailed mission whose `Flags2` holds 0x0004 fails. Its armour is
//! truncated toward zero (@0x3b193-0x3b1a1) by the fight itself, before
//! it decides the breakup (`Combat::tick_on`), as the original's dying
//! test after the truncation (`_IsDying` @0x3b1e5) reads the truncated
//! armour: an intact player hit down to 0.4 armour is truncated to none
//! and starts to break up in the same tick.
//!
//! **Newly breaking up** (@0x3b1fe-0x3b3f1): the beep again, "Ship
//! destroyed." ([`SimMessage::ShipDestroyed`], #288), and the same
//! mission check, from slot 0 again.
//!
//! For the player, `_IsDisabled` holds at armour 0 or below, so a hit
//! that takes an intact player straight to its breakup runs both parts:
//! two beeps, "Ship disabled." and then "Ship destroyed.", which replaces
//! it on the message line, and two mission checks. The
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
//! **Hits only.** `_DamageShip` takes the condition before and after
//! each hit, so only damage speaks the lines. The engine decides the
//! condition once a fight tick (`Combat::update_condition`), so this runs
//! only on a tick whose strikes hit the player, comparing its condition
//! before and after the tick. A condition that changes with no hit (a
//! ship taken over, armour set by a script or an editor) says nothing,
//! fails nothing, and is not truncated. A hit that disables and a later
//! one in the same tick that breaks the ship up make the same calls in
//! the same order as one hit from intact to breaking up, and the lines
//! settle before the tick's frame is drawn, so the player sees no
//! difference. A hull with `DeathDelay` 0 goes from intact to destroyed
//! in one tick, which counts as newly breaking up too.

use super::Session;
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::message::SimMessage;
use crate::sound::SimSound;

/// `Flags2`: the mission fails when the player's ship is disabled or
/// breaks up (`_DamageShip` @0x3b1bc, @0x3b3db: in-memory +0x5a & 4).
const FAILS_IF_DOWNED: u16 = 0x0004;

impl Session {
    /// Raises the player's "Ship disabled." and "Ship destroyed." lines,
    /// with their beeps, and fails its first `Flags2` 0x0004 mission for
    /// each, as the player's condition `was` before the fight's tick and
    /// is now, when the tick's `strikes` hit the player, drawing on
    /// `chance` (see the module docs).
    pub(super) fn player_downed(
        &mut self,
        was: Condition,
        strikes: &[Strike],
        chance: &mut dyn Chance,
    ) {
        if !strikes.iter().any(|strike| strike.ship == ShipRef::Player) {
            return;
        }
        let now = self.condition;
        if was == Condition::Intact && now != Condition::Intact {
            self.sounds.push(SimSound::ShipDowned);
            self.messages.push(SimMessage::ShipDisabled);
            // ship-handling-fidelity phase 3 arms `_playerRepairInhibit`
            // (300.0) here (@0x3b186-0x3b18d).
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
    use crate::catalog::{MissionRecord, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::hull::{DisableRule, HullSpec};
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::control::{Bit, Script};
    use crate::mission::FailedBy;
    use crate::reserves::Gauge;
    use crate::session::tests::{DISABLING, FIRE, Firing, armed, facing_an_npc};
    use crate::testkit::{FakePilotCatalog, carrying, mission};
    use crate::traffic::npc::NpcId;

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Disables a ship below this much armour.
    #[derive(Debug)]
    struct Below(f32);

    impl DisableRule for Below {
        fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
            armor.now < self.0
        }
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

    /// [`armed`] holding `missions`, its blaster passing through shields
    /// for `mass` armour a hit.
    fn shooting(missions: Vec<MissionRecord>, mass: i16) -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            missions,
            ..armed()
        };
        catalog.weapons[0] = WeaponRecord {
            mass_dmg: mass,
            energy_dmg: 0,
            flags: 0x0020,
            ..catalog.weapons[0]
        };
        catalog
    }

    /// A session of `catalog` with its NPC 100 above the player and
    /// facing it, the player's armour at `armor`, as `condition` says, its
    /// sounds and messages taken.
    fn under_fire(catalog: &FakePilotCatalog, armor: f32, condition: Condition) -> Session {
        let mut session = facing_an_npc(catalog, 180);
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

    /// One tick of the NPC firing at the player, ships disabled below
    /// `below` armour: the sounds and the messages it raised.
    fn tick(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        below: f32,
    ) -> (Vec<SimSound>, Vec<SimMessage>) {
        let below = Below(below);
        session.tick_traffic(catalog, &Firing, &mut NeverFires);
        session.tick_combat(
            Rules {
                disable: &below,
                ..Rules::default()
            },
            &mut NeverFires,
        );
        (session.take_sounds(), session.take_messages())
    }

    /// Ticks as [`tick`] until a shot hits the player: the sounds and the
    /// messages raised on the way.
    fn hit(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        below: f32,
    ) -> (Vec<SimSound>, Vec<SimMessage>) {
        let before = session.pilot.reserves.armor.now;
        let (mut sounds, mut messages) = (Vec::new(), Vec::new());
        for _ in 0..30 {
            let (said, told) = tick(session, catalog, below);
            sounds.extend(said);
            messages.extend(told);
            if session.pilot.reserves.armor.now != before {
                return (sounds, messages);
            }
        }
        panic!("the player was never hit");
    }

    fn armor(session: &Session) -> f32 {
        session.pilot.reserves.armor.now
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
        let catalog = shooting(vec![], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDisabled])
        );
        assert_eq!(session.condition, Condition::Disabled);
    }

    #[test]
    fn an_already_disabled_player_hit_says_nothing_more() {
        let catalog = shooting(vec![], 1);
        let mut session = under_fire(&catalog, 9.6, Condition::Disabled);
        assert_eq!(hit(&mut session, &catalog, 10.0), (vec![], vec![]));
        assert_eq!(session.condition, Condition::Disabled);
    }

    #[test]
    fn a_disabled_player_breaking_up_says_ship_destroyed_once() {
        let catalog = shooting(vec![], 1);
        let mut session = under_fire(&catalog, 0.5, Condition::Disabled);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDestroyed])
        );
        assert!(matches!(session.condition, Condition::Dying { .. }));
        assert_eq!(
            tick(&mut session, &catalog, 10.0),
            (vec![], vec![]),
            "still"
        );
        assert!(matches!(session.condition, Condition::Dying { .. }));
    }

    #[test]
    fn an_intact_player_broken_up_at_a_hit_says_both_lines_in_order() {
        let catalog = shooting(vec![], 20);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (
                vec![SimSound::ShipDowned, SimSound::ShipDowned],
                vec![SimMessage::ShipDisabled, SimMessage::ShipDestroyed]
            )
        );
        assert!(matches!(session.condition, Condition::Dying { .. }));
    }

    #[test]
    fn a_death_delay_0_hull_destroyed_at_a_hit_still_says_ship_destroyed() {
        let mut catalog = shooting(vec![], 1);
        catalog.hulls[0].death_delay = 0;
        let mut session = under_fire(&catalog, 0.5, Condition::Disabled);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (vec![SimSound::ShipDowned], vec![SimMessage::ShipDestroyed])
        );
        assert_eq!(session.condition, Condition::Destroyed);
        catalog.weapons[0].mass_dmg = 20;
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (
                vec![SimSound::ShipDowned, SimSound::ShipDowned],
                vec![SimMessage::ShipDisabled, SimMessage::ShipDestroyed]
            )
        );
        assert_eq!(session.condition, Condition::Destroyed);
    }

    #[test]
    fn the_newly_disabled_players_armour_is_truncated_toward_zero() {
        let catalog = shooting(vec![], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hit(&mut session, &catalog, 10.0);
        assert_eq!(armor(&session), 9.0, "9.6");
        let mut session = under_fire(&catalog, 9.6, Condition::Disabled);
        hit(&mut session, &catalog, 10.0);
        assert!((armor(&session) - 8.6).abs() < 1e-4, "disabled before");
        let mut session = under_fire(&catalog, 0.5, Condition::Disabled);
        hit(&mut session, &catalog, 10.0);
        assert_eq!(armor(&session), -0.5, "only breaking up");
        let catalog = shooting(vec![], 20);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hit(&mut session, &catalog, 10.0);
        assert_eq!(armor(&session), -9.0, "-9.4, toward zero");
    }

    #[test]
    fn a_newly_disabled_player_truncated_to_none_breaks_up_in_the_same_tick() {
        let catalog = shooting(vec![failing(142, 0x0004), failing(144, 0x0004)], 1);
        let mut session = under_fire(&catalog, 1.4, Condition::Intact);
        hold(&mut session, 2, 142, false);
        hold(&mut session, 4, 144, false);
        let _ = session.take_combat_events();
        assert_eq!(
            hit(&mut session, &catalog, 1.0),
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
            ),
            "0.4 truncated to none"
        );
        assert_eq!(armor(&session), 0.0);
        assert_eq!(session.condition, Condition::Dying { ticks_left: 1 });
        assert!(session.take_combat_events().iter().any(|event| matches!(
            event,
            CombatEvent::BreakingUp {
                ship: ShipRef::Player,
                ..
            }
        )));
    }

    /// A session of `catalog`, the player firing at its NPC 100 above it
    /// and facing it, the NPC holding its fire.
    fn firing_at_the_npc(catalog: &FakePilotCatalog) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        session.hold_trigger(FIRE);
        session.target = Some(NpcId(0));
        let _ = session.take_sounds();
        let _ = session.take_messages();
        session
    }

    #[test]
    fn a_hit_on_another_ship_as_the_players_condition_changes_says_nothing() {
        // The fight ticks until the player's first shot hits the NPC; then,
        // afresh, the player's armour drops below the rule's 5 as that
        // tick comes, with no hit on the player.
        let catalog = shooting(vec![failing(140, 0x0004)], 1);
        let below = Below(5.0);
        let rules = Rules {
            disable: &below,
            ..Rules::default()
        };
        let mut session = firing_at_the_npc(&catalog);
        let full = session.npcs()[0].reserves.armor.now;
        let ticks = (1..30)
            .find(|_| {
                session.tick_combat(rules, &mut NeverFires);
                session.npcs()[0].reserves.armor.now != full
            })
            .expect("the NPC is hit");
        let mut session = firing_at_the_npc(&catalog);
        hold(&mut session, 0, 140, false);
        for _ in 1..ticks {
            session.tick_combat(rules, &mut NeverFires);
        }
        session.pilot.reserves.armor.now = 4.6;
        session.tick_combat(rules, &mut NeverFires);
        assert_ne!(session.npcs()[0].reserves.armor.now, full, "hit then");
        assert_eq!(session.condition, Condition::Disabled);
        assert_eq!(
            (session.take_sounds(), session.take_messages()),
            (vec![], vec![])
        );
        assert_eq!(failed_in(&session, 0), None);
        assert_eq!(armor(&session), 4.6, "not truncated");
    }

    #[test]
    fn a_condition_changed_with_no_hit_raises_nothing() {
        // With no NPC to shoot, the rule disables the player, and armour
        // set to none breaks it up: neither is a hit.
        let catalog = FakePilotCatalog {
            missions: vec![failing(140, 0x0004)],
            ..armed()
        };
        let mut session = Session::start(&catalog).expect("starts");
        hold(&mut session, 0, 140, false);
        session.pilot.reserves.armor.now = 7.6;
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(session.condition, Condition::Disabled);
        session.pilot.reserves.armor.now = 0.0;
        session.tick_combat(DISABLING, &mut NeverFires);
        assert!(matches!(session.condition, Condition::Dying { .. }));
        assert_eq!(
            (session.take_sounds(), session.take_messages()),
            (vec![], vec![])
        );
        assert_eq!(failed_in(&session, 0), None);
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.reserves.armor.now = 7.6;
        session.tick_combat(DISABLING, &mut NeverFires);
        assert_eq!(armor(&session), 7.6, "not truncated");
    }

    #[test]
    fn an_npc_downed_by_the_player_raises_no_ship_lines_and_keeps_its_armour() {
        // The player fires at the NPC, 100 above it and facing it, and the
        // NPC holds its fire.
        let catalog = shooting(vec![], 1);
        for armor in [10.6, 0.5] {
            let mut session = facing_an_npc(&catalog, 180);
            session.traffic.npcs_mut()[0].reserves.armor.now = armor;
            session.hold_trigger(FIRE);
            session.target = Some(NpcId(0));
            let _ = session.take_sounds();
            let _ = session.take_messages();
            let below = Below(10.0);
            let rules = Rules {
                disable: &below,
                ..Rules::default()
            };
            let mut heard = (Vec::new(), Vec::new());
            for _ in 0..30 {
                session.tick_combat(rules, &mut NeverFires);
                heard.0.extend(session.take_sounds());
                heard.1.extend(session.take_messages());
                if session.npcs()[0].condition != Condition::Intact {
                    break;
                }
            }
            assert_ne!(session.npcs()[0].condition, Condition::Intact, "{armor}");
            assert_eq!(session.condition, Condition::Intact);
            assert_eq!(heard, (vec![], vec![]), "{armor}");
            if armor > 1.0 {
                assert!((session.npcs()[0].reserves.armor.now - 9.6).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn a_flags2_0x0004_mission_fails_when_the_player_is_disabled() {
        let catalog = shooting(vec![failing(140, 0x0004)], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
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
        let catalog = shooting(vec![record], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 0, 140, false);
        let (sounds, messages) = hit(&mut session, &catalog, 10.0);
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
        let catalog = shooting(missions, 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 0, 140, false);
        hold(&mut session, 1, 141, true);
        hold(&mut session, 3, 143, false);
        hold(&mut session, 5, 145, false);
        hit(&mut session, &catalog, 10.0);
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
        let catalog = shooting(vec![failing(140, 0x000b), failing(141, 0)], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 0, 140, false);
        hold(&mut session, 1, 141, false);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
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
        let catalog = shooting(vec![record], 1);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (
                vec![SimSound::ShipDowned, SimSound::DownedMissionFailed],
                vec![SimMessage::ShipDisabled, SimMessage::DownedMissionFailed]
            )
        );
        assert_eq!(failed_in(&session, 0), Some(FailedBy::Quick));
    }

    #[test]
    fn a_flags2_0x0004_mission_fails_when_the_player_breaks_up() {
        let catalog = shooting(vec![failing(140, 0x0004)], 1);
        let mut session = under_fire(&catalog, 0.5, Condition::Disabled);
        hold(&mut session, 0, 140, false);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
            (
                vec![SimSound::ShipDowned, SimSound::DownedMissionFailed],
                vec![SimMessage::ShipDestroyed, SimMessage::DownedMissionFailed]
            )
        );
        assert_eq!(failed_in(&session, 0), Some(FailedBy::Quick));
    }

    #[test]
    fn a_one_hit_breakup_fails_two_flags2_0x0004_missions() {
        let catalog = shooting(vec![failing(142, 0x0004), failing(144, 0x0004)], 20);
        let mut session = under_fire(&catalog, 10.6, Condition::Intact);
        hold(&mut session, 2, 142, false);
        hold(&mut session, 4, 144, false);
        assert_eq!(
            hit(&mut session, &catalog, 10.0),
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
