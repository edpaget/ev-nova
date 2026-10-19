//! An NPC's skill and its government's `SkillMult` on its top speed and
//! thrust, and a still hull's (see the [`skill`](crate::skill) module).
//!
//! **The class's handling** ([`Npc::class_handling`]): `_ShipMaxSpeed`
//! (@0x37b5-0x37ee) and `_ShipAccelRate` (@0x3601-0x363a) take an NPC's
//! class's top speed and thrust (its stats', which the
//! [`RuleKey::NpcDefaultItems`](crate::RuleKey::NpcDefaultItems) reading
//! builds, see [`ShipKind`](crate::traffic::table::ShipKind)), × its
//! skill (ship +0x40), × its government's factor (gövt +0x64, when it
//! has a government), each a float product in that order, and return
//! none at once for a [`STILL_HULL`](crate::skill::STILL_HULL) class.
//! Its turn takes neither (`_ShipTurnRate` @0x32f7). A tractor's third
//! comes after them ([`Npc::own_handling`]), then the player's led × 1.5
//! ([`Npc::top_speed`]), the flight's divisor and the ionization. A
//! skill below none (a `SkillVar` over 100 can roll one) gives none, as
//! both functions floor their result.
//!
//! **The government's factor follows the government.** The original
//! reads gövt +0x64 of the ship's government each time; the engine keeps
//! the factor on the NPC ([`Npc::skill_mult`]), as `SkillMult` is fixed
//! record data, and every write of an NPC's government goes through
//! [`Npc::independent`], [`Npc::take_govt_of`] or [`Npc::join_govt`],
//! which write both. Each builder of an NPC sets them so once it is
//! built: `Traffic::add` joins the spawned ship's government; an escort
//! placed, a fighter launched and the eject's hulk are made independent,
//! a fighter of an NPC carrier then taking its carrier's; and a ship
//! enlisted in place becomes independent, a ship a pirate takes taking
//! the pirate's.
//!
//! **What the skill also feeds, left out.** `_LowLevelAIHandler`'s
//! close formation (substate 0xc, @0x87b4f-0x87bcb) nudges an escort's
//! heading 1° toward its leader's while `0 < |diff| < trunc(25 / skill)`,
//! 25 when the skill is none or less. The engine's close formation isn't
//! a port of that substate, so the nudge is handed to
//! rdm:phase/escort-fleet-fidelity/phase-2-formation-flight.

use crate::catalog::GovtId;
use crate::govt::Governments;
use crate::handling::Handling;
use crate::traffic::npc::Npc;

impl Npc {
    /// Its handling before a tractor's hold (see the module docs): its
    /// stats' top speed and thrust × its skill × its government's factor,
    /// none below none, or none at all for a still hull; its turn its
    /// stats'.
    #[must_use]
    pub fn class_handling(&self) -> Handling {
        let own = self.stats.handling;
        let scaled = |value: f32| {
            if self.still {
                0.0
            } else {
                (value * self.skill * self.skill_mult).max(0.0)
            }
        };
        Handling {
            max_speed: scaled(own.max_speed),
            accel: scaled(own.accel),
            turn_rate: own.turn_rate,
        }
    }

    /// It becomes independent: no government, and so no factor.
    pub fn independent(&mut self) {
        self.govt = None;
        self.skill_mult = 1.0;
    }

    /// It takes `other`'s government, and so its factor.
    pub fn take_govt_of(&mut self, other: &Npc) {
        self.govt = other.govt;
        self.skill_mult = other.skill_mult;
    }

    /// It joins `govt`, taking its factor from `govts`.
    pub fn join_govt(&mut self, govt: Option<GovtId>, govts: &Governments) {
        self.govt = govt;
        self.skill_mult = govts.skill_mult(govt);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::combat::ShipRef;
    use crate::combat::impact::TractorLock;
    use crate::combat::tractor_hold::held_handling;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::govt;

    /// `FAST` (top speed 6, thrust 0.18, turn 3), `IonizeMax` 400, of
    /// `skill` and government factor `mult`.
    fn flying(skill: f32, mult: f32) -> Npc {
        let mut ship = npc(
            1,
            ShipStats::new(
                ShipFields {
                    ionize_max: 400,
                    ..FAST
                },
                &[],
            ),
        );
        ship.skill = skill;
        ship.skill_mult = mult;
        ship
    }

    /// `ship`, held by another ship.
    fn held(mut ship: Npc) -> Npc {
        ship.tractor = Some(TractorLock::by(ShipRef::Npc(NpcId(2))));
        ship
    }

    #[test]
    fn an_npcs_top_speed_and_thrust_take_its_skill_then_its_governments_factor() {
        let ship = flying(0.9, 1.5);
        let own = ship.stats.handling;
        let expected = Handling {
            max_speed: own.max_speed * 0.9 * 1.5,
            accel: own.accel * 0.9 * 1.5,
            turn_rate: own.turn_rate,
        };
        assert_eq!(ship.class_handling(), expected);
        assert_eq!(ship.own_handling(), expected, "not held");
        assert_eq!(flying(1.0, 1.0).own_handling(), own);
    }

    #[test]
    fn a_still_hull_neither_moves_nor_thrusts_at_any_skill_but_turns() {
        let mut ship = flying(1.3, 2.0);
        ship.still = true;
        let own = ship.stats.handling;
        let still = Handling {
            max_speed: 0.0,
            accel: 0.0,
            turn_rate: own.turn_rate,
        };
        assert_eq!(ship.own_handling(), still);
        let held = held(ship);
        assert_eq!(
            held.own_handling(),
            held_handling(still, held.hull.class_turn)
        );
        assert!(held.own_handling().turn_rate < own.turn_rate, "a third");
        assert!(held.own_handling().turn_rate > 0.0);
    }

    #[test]
    fn the_factors_come_before_a_tractors_third() {
        let ship = held(flying(0.8, 2.5));
        let own = ship.stats.handling;
        let third = |value: f64| (value * 0.333) as f32;
        let held = ship.own_handling();
        assert_eq!(held.max_speed, third(f64::from(own.max_speed * 0.8 * 2.5)));
        assert_eq!(held.accel, third(f64::from(own.accel * 0.8 * 2.5) / 2.0));
        assert_eq!(held, held_handling(ship.class_handling(), 0.0));
    }

    #[test]
    fn a_skill_below_none_gives_no_speed_or_thrust() {
        let ship = flying(-0.5, 1.5);
        let own = ship.stats.handling;
        assert_eq!(
            ship.own_handling(),
            Handling {
                max_speed: 0.0,
                accel: 0.0,
                turn_rate: own.turn_rate,
            }
        );
        let held = held(ship).own_handling();
        assert_eq!((held.max_speed, held.accel), (0.0, 0.0));
    }

    #[test]
    fn a_player_led_escorts_top_speed_takes_the_led_factor_after_its_own() {
        let mut ship = flying(1.2, 0.6);
        let own = ship.stats.handling.max_speed;
        assert_eq!(ship.top_speed(1.5), own * 1.2 * 0.6, "not led");
        ship.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        assert_eq!(ship.top_speed(1.5), own * 1.2 * 0.6 * 1.5);
    }

    #[test]
    fn its_ionized_handling_carries_the_factors() {
        let mut ship = flying(0.7, 2.0);
        ship.ion.charge = 200.0;
        let own = ship.stats.handling;
        let slowed = ship.ionized_handling();
        assert_eq!(
            slowed.accel,
            (f64::from(own.accel * 0.7 * 2.0) * 0.5) as f32
        );
        assert_eq!(slowed.max_speed, own.max_speed * 0.7 * 2.0);
    }

    #[test]
    fn each_government_write_keeps_its_factor() {
        let govts = Governments::new([GovtRecord {
            skill_mult: 150,
            ..govt(130)
        }]);
        let mut ship = flying(1.0, 1.0);
        ship.join_govt(Some(crate::catalog::GovtId(130)), &govts);
        assert_eq!((ship.govt, ship.skill_mult), (Some(GovtId(130)), 1.5));
        let mut other = flying(1.0, 1.0);
        other.take_govt_of(&ship);
        assert_eq!((other.govt, other.skill_mult), (Some(GovtId(130)), 1.5));
        ship.independent();
        assert_eq!((ship.govt, ship.skill_mult), (None, 1.0));
        ship.join_govt(Some(GovtId(131)), &govts);
        assert_eq!((ship.govt, ship.skill_mult), (Some(GovtId(131)), 1.0));
    }
}
