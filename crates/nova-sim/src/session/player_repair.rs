//! The disabled player repairing itself, and the repair inhibit that holds
//! it back: `_HandlePlayer`'s countdown and repair (@0x68b09-0x68c6c), its
//! armour regeneration's inhibit test (@0x6d03e-0x6d049), and
//! `_ShipCanSelfRepair` (@0x9134) for the player. It is the player's
//! counterpart of the `self_repair` module's NPCs.
//!
//! **The inhibit** ([`RepairInhibit`], `_playerRepairInhibit` 0x3b50b4) is
//! armed at 300 as the player is newly disabled (`_DamageShip`
//! @0x3b186-0x3b18d, see the `player_downed` module), a one-hit breakup
//! included, and turned off by every reset ([`Session::reset_precalced`],
//! `_ResetPlayerPrecalcedValues` @0xc998). While it holds, the player's
//! armour does not regenerate ([`Session::player_armor_regen`], which the
//! fight's [`Fighter::armor_regen`](crate::combat::Fighter::armor_regen)
//! takes) and its gear does not repair it.
//!
//! **Each pass that runs `_HandlePlayer`'s work before the fight** (both
//! passes in double time, see the `double_time` module),
//! [`Session::player_repair_tick`] first counts the inhibit down a tick
//! (@0x68b09-0x68b2c). Then, while the player is disabled and the inhibit
//! does not hold (@0x68b30-0x68b40), it draws `Rand(`[`REPAIR_ODDS`]`)` and
//! repairs on 0, when the player is not breaking up, its armour is above
//! 0 and it owns a repair system ([`can_self_repair`], [`has_gear`] over
//! the outfits it owns, @0x919f-0x91e3). The draw comes before the gear
//! test, as the original's does. The armour goes to [`repaired_armor`]: a
//! third of what it holds plus one, a tenth plus one for a tough hull
//! (`Flags` 0x0010, @0x68b6e). The condition is decided again at once with
//! the fight's [`DisableRule`](crate::combat::hull::DisableRule), as for
//! an NPC, so the tick's scans and fight see the player intact. The player
//! is told "Propulsion systems repaired." ([`SimMessage::PropulsionRepaired`],
//! `STR#` 2002 #37, for 250 ticks, `_DisplayComm` @0x68c3f), and then
//! `_beepSnd[4]` plays ([`SimSound::PropulsionRepaired`], `snd ` 154,
//! @0x68c44-0x68c67).
//!
//! When the player is not disabled, the original's block runs
//! `_HandleShipPointDefense` instead (@0x68c6e-0x68c74), which the fight
//! already runs for every ship.
//!
//! **Landed or jumping**, where the fight stands still, the countdown
//! pauses with it. No one can see it: no regeneration happens there, and
//! an arrival or a take-off resets the inhibit anyway.
//!
//! *Ordering against the original.* On the frame of a repair the original
//! skips the shield's regeneration, as it tests the disabled flag it took
//! at the frame's start (-0x2cb, @0x6cfa2); the engine's fight sees the
//! player intact and regenerates one tick's shield rate. One tick's rate
//! is not something the player can see, so it is not filed.
//!
//! **Left out.** The cheat that writes the inhibit (@0x6ebdf), as the
//! engine has no cheats.

use super::Session;
use crate::chance::Chance;
use crate::combat::Rules;
use crate::combat::hull::Condition;
use crate::message::SimMessage;
use crate::self_repair::{REPAIR_ODDS, RepairInhibit, can_self_repair, has_gear, repaired_armor};
use crate::sound::SimSound;

impl Session {
    /// Counts the repair inhibit down a tick, then lets a disabled player
    /// whose inhibit has run out repair itself, drawing on `chance`, its
    /// condition decided again by `rules` (see the module docs).
    pub(super) fn player_repair_tick(&mut self, rules: Rules, chance: &mut dyn Chance) {
        self.repair_inhibit.tick();
        if self.condition != Condition::Disabled || self.repair_inhibit.holds() {
            return;
        }
        let drawn = chance.below(REPAIR_ODDS);
        let gear = has_gear(&self.pilot.outfits, &self.outfits);
        let armor = &mut self.pilot.reserves.armor;
        if !can_self_repair(drawn, self.condition, armor.now, gear) {
            return;
        }
        armor.now = repaired_armor(armor.max, self.hull.tough);
        self.condition = if rules.disable.disabled(*armor, &self.hull) {
            Condition::Disabled
        } else {
            Condition::Intact
        };
        self.say(SimMessage::PropulsionRepaired);
        self.sounds.push(SimSound::PropulsionRepaired);
    }

    /// The armour the player regenerates a fight tick: none while the
    /// repair inhibit holds (@0x6d03e-0x6d049), else its ship's rate.
    pub(super) fn player_armor_regen(&self) -> f32 {
        if self.repair_inhibit.holds() {
            0.0
        } else {
            self.stats.armor_regen
        }
    }

    /// Arms the repair inhibit, as the player is newly disabled
    /// (`_DamageShip` @0x3b186-0x3b18d).
    pub(super) fn arm_repair_inhibit(&mut self) {
        self.repair_inhibit = RepairInhibit::armed();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{HullRecord, OutfitId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::hull::{DisableRule, HullSpec, TOUGH};
    use crate::reserves::Gauge;
    use crate::self_repair::{MOD_REPAIR, REPAIR_INHIBIT_TICKS};
    use crate::session::tests::{DISABLING, Firing, armed, facing_an_npc};
    use crate::testkit::{Draws, FakePilotCatalog, catalog, hull, outfit};
    use crate::traffic::npc::NpcId;

    /// Repair Droids.
    const DROIDS: OutfitId = OutfitId(437);

    /// The player's ship (128: 30 shield, 45 armour), its hull of `flags`,
    /// Repair Droids among the outfits.
    fn repairing(flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            outfits: vec![outfit(437, &[(MOD_REPAIR, 0)])],
            hulls: vec![HullRecord { flags, ..hull(128) }],
            ..catalog()
        }
    }

    /// A session of `catalog`, the player owning Repair Droids when
    /// `droids`, disabled by its armour at 3 of 45 with no hit, so the
    /// inhibit is off, and its sounds and messages taken.
    fn disabled(catalog: &FakePilotCatalog, droids: bool) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        if droids {
            session.pilot.outfits.insert(DROIDS, 1);
        }
        session.pilot.reserves.armor.now = 3.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.condition, Condition::Disabled);
        assert!(!session.repair_inhibit.holds());
        let _ = (session.take_sounds(), session.take_messages());
        session
    }

    /// One fight tick on `draws`, ships disabled by `rules`: what it asked.
    fn tick_on(session: &mut Session, rules: Rules, draws: &[u32]) -> Vec<u32> {
        let mut chance = Draws::of(draws);
        session.tick_combat(rules, &mut chance);
        chance.asked
    }

    fn armor(session: &Session) -> f32 {
        session.pilot.reserves.armor.now
    }

    #[test]
    fn a_disabled_player_with_repair_droids_repairs_on_a_draw_of_0() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, true);
        assert_eq!(tick_on(&mut session, Rules::default(), &[0]), [REPAIR_ODDS]);
        assert_eq!(armor(&session), repaired_armor(45.0, false));
        assert!((armor(&session) - 15.9985).abs() < 1e-4, "a third plus one");
        assert_eq!(session.condition, Condition::Intact);
        assert_eq!(session.take_messages(), [SimMessage::PropulsionRepaired]);
        assert_eq!(session.take_sounds(), [SimSound::PropulsionRepaired]);
    }

    #[test]
    fn a_tough_hull_is_repaired_to_a_tenth_plus_one() {
        let catalog = repairing(TOUGH);
        let mut session = disabled(&catalog, true);
        tick_on(&mut session, Rules::default(), &[0]);
        assert_eq!(armor(&session), repaired_armor(45.0, true));
        assert!((armor(&session) - 5.5).abs() < 1e-4);
        assert_eq!(session.take_messages(), [SimMessage::PropulsionRepaired]);
    }

    #[test]
    fn a_repaired_player_still_disabled_by_the_rule_stays_disabled() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, true);
        assert_eq!(tick_on(&mut session, DISABLING, &[0]), [REPAIR_ODDS]);
        assert_eq!(armor(&session), repaired_armor(45.0, false));
        assert_eq!(session.condition, Condition::Disabled);
        assert_eq!(session.take_messages(), [SimMessage::PropulsionRepaired]);
        assert_eq!(
            tick_on(&mut session, DISABLING, &[1]),
            [REPAIR_ODDS],
            "and draws again"
        );
    }

    #[test]
    fn any_other_draw_repairs_nothing() {
        let catalog = repairing(0);
        for draw in [1, 499] {
            let mut session = disabled(&catalog, true);
            assert_eq!(
                tick_on(&mut session, Rules::default(), &[draw]),
                [REPAIR_ODDS]
            );
            assert_eq!(armor(&session), 3.0, "{draw}");
            assert_eq!(session.condition, Condition::Disabled);
            assert_eq!(
                (session.take_messages(), session.take_sounds()),
                (vec![], vec![])
            );
        }
    }

    #[test]
    fn without_the_gear_the_player_still_draws_but_is_never_repaired() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, false);
        assert_eq!(tick_on(&mut session, Rules::default(), &[0]), [REPAIR_ODDS]);
        assert_eq!(armor(&session), 3.0);
        assert_eq!(session.condition, Condition::Disabled);
        assert_eq!(session.take_messages(), []);
        let mut session = disabled(&catalog, true);
        session.pilot.outfits.insert(DROIDS, 0);
        tick_on(&mut session, Rules::default(), &[0]);
        assert_eq!(armor(&session), 3.0, "owning none of them");
    }

    #[test]
    fn while_the_inhibit_holds_nothing_is_drawn_or_repaired() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, true);
        session.arm_repair_inhibit();
        let mut chance = Draws::of(&[0]);
        for tick in 0..REPAIR_INHIBIT_TICKS {
            session.tick_combat(Rules::default(), &mut chance);
            assert!(chance.asked.is_empty(), "tick {tick}");
        }
        assert_eq!(armor(&session), 3.0);
        assert_eq!(session.condition, Condition::Disabled);
        session.tick_combat(Rules::default(), &mut chance);
        assert_eq!(chance.asked, [REPAIR_ODDS], "on the 301st");
        assert_eq!(armor(&session), repaired_armor(45.0, false));
        assert_eq!(session.take_messages(), [SimMessage::PropulsionRepaired]);
    }

    #[test]
    fn a_dying_player_or_one_with_no_armour_is_never_repaired() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, true);
        session.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        assert!(tick_on(&mut session, Rules::default(), &[0]).is_empty());
        assert!(matches!(session.condition, Condition::Dying { .. }));
        assert_eq!(armor(&session), 3.0);
        for none in [0.0, -1.0] {
            let mut session = disabled(&catalog, true);
            session.pilot.reserves.armor.now = none;
            assert_eq!(tick_on(&mut session, Rules::default(), &[0]), [REPAIR_ODDS]);
            assert_eq!(armor(&session), none);
            assert!(
                !session
                    .take_messages()
                    .contains(&SimMessage::PropulsionRepaired)
            );
        }
    }

    #[test]
    fn an_intact_player_draws_nothing() {
        let catalog = repairing(0);
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.outfits.insert(DROIDS, 1);
        session.pilot.reserves.armor.now = 30.0;
        assert!(tick_on(&mut session, Rules::default(), &[0]).is_empty());
        assert_eq!(armor(&session), 30.0);
        assert_eq!(session.condition, Condition::Intact);
    }

    #[test]
    fn a_disabled_player_regenerates_no_shield_inhibited_or_not() {
        let catalog = repairing(0);
        let mut session = disabled(&catalog, false);
        session.stats.shield_regen = 1.0;
        session.stats.armor_regen = 1.0;
        session.pilot.reserves.shield.now = 5.0;
        session.arm_repair_inhibit();
        for _ in 0..REPAIR_INHIBIT_TICKS + 20 {
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(session.condition, Condition::Disabled);
            assert_eq!(session.pilot.reserves.shield.now, 5.0);
            assert_eq!(armor(&session), 3.0);
        }
        assert!(!session.repair_inhibit.holds(), "it has lapsed");
    }

    /// An intact player regenerating a point of armour a tick, its armour
    /// at 30 of 45.
    fn regenerating(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.stats.armor_regen = 1.0;
        session.pilot.reserves.armor.now = 30.0;
        session
    }

    #[test]
    fn an_armed_inhibit_holds_back_armour_regeneration_for_300_ticks() {
        let catalog = repairing(0);
        let mut session = regenerating(&catalog);
        session.arm_repair_inhibit();
        for tick in 0..REPAIR_INHIBIT_TICKS {
            session.tick_combat(Rules::default(), &mut NeverFires);
            assert_eq!(armor(&session), 30.0, "tick {tick}");
        }
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(armor(&session), 31.0, "on the 301st");
    }

    #[test]
    fn in_double_time_both_passes_count_the_inhibit_down() {
        let catalog = repairing(0);
        let mut session = regenerating(&catalog);
        session.arm_repair_inhibit();
        let pass = |session: &mut Session| {
            session.tick_combat(Rules::default(), &mut NeverFires);
            session.tick_again(
                crate::flight::Controls::default(),
                &crate::ai::NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
        };
        for _ in 0..REPAIR_INHIBIT_TICKS / 2 {
            pass(&mut session);
        }
        assert_eq!(armor(&session), 30.0, "300 passes held");
        pass(&mut session);
        assert_eq!(armor(&session), 32.0, "a point on each pass after");
    }

    #[test]
    fn a_player_never_disabled_regenerates_armour_at_once() {
        let catalog = repairing(0);
        let mut session = regenerating(&catalog);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(armor(&session), 31.0);
    }

    #[test]
    fn a_reset_turns_the_inhibit_off() {
        let catalog = repairing(0);
        let mut session = regenerating(&catalog);
        session.arm_repair_inhibit();
        session.jettison_cargo(true, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(armor(&session), 31.0);
    }

    /// Disables a ship below this much armour.
    #[derive(Debug)]
    struct Below(f32);

    impl DisableRule for Below {
        fn disabled(&self, armor: Gauge, _hull: &HullSpec) -> bool {
            armor.now < self.0
        }
    }

    #[test]
    fn a_player_disabled_by_a_hit_regenerates_no_armour_for_300_ticks() {
        // The NPC 100 above the player fires a blaster that passes through
        // shields for a point of armour, disabling the player below 10.
        let mut catalog = armed();
        catalog.weapons[0] = WeaponRecord {
            mass_dmg: 1,
            energy_dmg: 0,
            flags: 0x0020,
            ..catalog.weapons[0]
        };
        let below = Below(10.0);
        let rules = Rules {
            disable: &below,
            ..Rules::default()
        };
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.reserves.armor.now = 10.6;
        session.tick_traffic(&catalog, &Firing, &mut NeverFires);
        let mut ticks = 0;
        while session.condition == Condition::Intact {
            assert!(ticks < 30, "the player was never hit");
            session.tick_combat(rules, &mut NeverFires);
            ticks += 1;
        }
        assert_eq!(session.take_messages(), [SimMessage::ShipDisabled]);
        // The NPC leaves, and the player's armour is put back, so the
        // next tick finds it intact, regenerating a point a tick.
        session.traffic.remove(NpcId(0));
        session.pilot.reserves.armor.now = 40.0;
        session.stats.armor_regen = 1.0;
        for tick in 0..REPAIR_INHIBIT_TICKS {
            session.tick_combat(rules, &mut NeverFires);
            assert!(armor(&session) <= 40.0, "tick {tick}");
        }
        assert_eq!(session.condition, Condition::Intact);
        let before = armor(&session);
        session.tick_combat(rules, &mut NeverFires);
        assert_eq!(armor(&session), before + 1.0, "on the 301st");
    }
}
