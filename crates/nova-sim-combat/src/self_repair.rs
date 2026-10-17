//! A disabled ship repairing itself, and a former escort rejoining the
//! player as it does: `_ShipCanSelfRepair` (@0x9134-0x923e) and the
//! self-repair branch of `_HandleShip` (@0x33a3b-0x33bde).
//!
//! **The gear.** A ship can repair itself when it carries a repair system,
//! an outfit with a `ModType` of [`MOD_REPAIR`] in any of its four mod
//! slots ([`has_gear`]). The player's gear is the outfits it owns
//! (@0x919f-0x91e3); any other ship's is its **class's** `DefaultItems`
//! (@0x91e5-0x922d), never anything it bought. In the stock data only
//! `oütf` 437, Repair Droids, is one, and no `shïp` carries it, so only
//! plug-ins make an NPC repair itself.
//!
//! **The draw.** Each tick a disabled ship with the gear draws
//! `Rand(`[`REPAIR_ODDS`]`)` and repairs itself on 0 ([`can_self_repair`]):
//! the original's n is 500 over the frame's length in 30 Hz ticks
//! (`*0x3b511c`), which is 1 at the engine's fixed rate. A ship breaking
//! up, or one with no armour, never does. The engine tests the gear before
//! it draws, so a ship without it draws nothing.
//!
//! **The repair.** The armour goes to [`repaired_armor`]: a third of what
//! it holds (a tenth for a tough hull) plus one, just above where it is
//! disabled (@0x33a4e-0x33ad4).
//!
//! **The rejoin.** A ship that was the player's escort when it was
//! disabled remembers what it was ([`FormerEscort`], the original's
//! +0xc8de, set by `_DamageShip` @0x3af8f-0x3afce) and, repairing itself,
//! rejoins the fleet as that: a fighter as a carried fighter, a hired
//! escort hired with its wage, a captured one unhired
//! ([`FormerEscort::rejoined`]). The player is told
//! "Fighter repaired." or "Escort repaired." ([`rejoin_message`]).

use std::collections::BTreeMap;

use crate::catalog::{OutfitId, OutfitRecord};
use crate::combat::hull::Condition;
use crate::fleet::Escort;
use crate::message::SimMessage;
use crate::reserves::Reserves;

/// The `ModType` of a repair system (the Bible's 49, 0x31).
pub const MOD_REPAIR: i16 = 49;
/// The sides of the draw a disabled ship makes each tick: it repairs
/// itself on 0 (500.0 @0xdd120 over a frame of one tick).
pub const REPAIR_ODDS: u32 = 500;
/// The share of what it holds a tough hull's armour is repaired to (0.1
/// @0xdd110).
pub const TOUGH_REPAIR_SHARE: f64 = 0.1;
/// The share for any other hull (0.3333 @0xdd5f8).
pub const REPAIR_SHARE: f64 = 0.3333;
/// What the repaired armour adds to the share (1.0 @0xdd0c0).
pub const REPAIR_PLUS: f64 = 1.0;

/// Whether `items`, of `outfits`, hold a repair system: an outfit with a
/// count above 0 that has a mod of [`MOD_REPAIR`] in any slot. The
/// player's `items` are the outfits it owns, an NPC's its class's
/// `DefaultItems` (see the module docs).
#[must_use]
pub fn has_gear(items: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> bool {
    outfits
        .iter()
        .filter(|record| items.get(&record.id).is_some_and(|&count| count > 0))
        .any(|record| record.mods.iter().any(|&(kind, _)| kind == MOD_REPAIR))
}

/// Whether a ship in `condition`, with `armor`, having `drawn` from
/// `Rand(`[`REPAIR_ODDS`]`)` and carrying the gear or not, repairs itself
/// (`_ShipCanSelfRepair`): on a draw of 0, when it is not breaking up or
/// destroyed, its armour is above 0, and it has the gear.
#[must_use]
pub fn can_self_repair(drawn: u32, condition: Condition, armor: f32, gear: bool) -> bool {
    let breaking_up = matches!(condition, Condition::Dying { .. } | Condition::Destroyed);
    drawn == 0 && !breaking_up && armor > 0.0 && gear
}

/// The armour a ship holding `capacity`, its hull `tough` or not, is
/// repaired to: `capacity` × [`TOUGH_REPAIR_SHARE`] or [`REPAIR_SHARE`],
/// plus [`REPAIR_PLUS`], in doubles as the original works it.
#[must_use]
pub fn repaired_armor(capacity: f32, tough: bool) -> f32 {
    let share = if tough {
        TOUGH_REPAIR_SHARE
    } else {
        REPAIR_SHARE
    };
    // The original's double arithmetic, stored back as a float.
    #[allow(clippy::cast_possible_truncation)]
    let armor = f64::from(capacity).mul_add(share, REPAIR_PLUS) as f32;
    armor
}

/// The ticks a newly disabled player's repair inhibit holds after it is
/// armed: `_DamageShip` sets `_playerRepairInhibit` to 300.0
/// (@0x3b186-0x3b18d).
pub const REPAIR_INHIBIT_TICKS: i32 = 300;

/// The player's repair inhibit, `_playerRepairInhibit` (0x3b50b4): while it
/// holds, the player's armour neither regenerates (`_HandlePlayer`
/// @0x6d03e-0x6d049) nor is repaired by its gear (@0x68b3d-0x68b40).
///
/// The original keeps it as a float. `_DamageShip` arms it at 300.0 as the
/// player is newly disabled, `_ResetPlayerPrecalcedValues` turns it off at
/// -1.0 (@0xc998), and each frame of `_HandlePlayer` takes a frame's length
/// in ticks (1 at the engine's rate) off it while it is at 0 or above
/// (@0x68b09-0x68b2c). It holds while it is at 0 or above, so armed it
/// holds through the next [`REPAIR_INHIBIT_TICKS`] ticks and opens on the
/// one after. The count here is whole ticks, which matches the float
/// exactly at the engine's fixed rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairInhibit {
    /// The ticks left, -1 when off.
    left: i32,
}

impl RepairInhibit {
    /// Off: it never holds (the -1.0).
    pub const OFF: Self = Self { left: -1 };

    /// Armed as the player is newly disabled: it holds for the next
    /// [`REPAIR_INHIBIT_TICKS`] ticks.
    #[must_use]
    pub fn armed() -> Self {
        Self {
            left: REPAIR_INHIBIT_TICKS,
        }
    }

    /// Counts down one tick while it is at 0 or above.
    pub fn tick(&mut self) {
        if self.left >= 0 {
            self.left -= 1;
        }
    }

    /// Whether it holds back the player's armour regeneration and repair.
    #[must_use]
    pub fn holds(self) -> bool {
        self.left >= 0
    }
}

impl Default for RepairInhibit {
    fn default() -> Self {
        Self::OFF
    }
}

/// What a former escort was, as `_DamageShip` sets +0xc8de: 0, 1 or 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormerKind {
    /// 0: a fighter the player launched (AI type 5).
    Fighter,
    /// 1: an escort hired in the bar (the hired flag +0xbb).
    Hired,
    /// 2: any other escort: captured, the old ship kept after "Use As My
    /// Ship", or a person enlisted by a hail, which the original never
    /// marks hired.
    Captured,
}

impl FormerKind {
    /// What the escort of `record` is: a carried fighter, else hired
    /// when it has a wage, else captured.
    #[must_use]
    pub fn of(record: &Escort) -> Self {
        if record.carried {
            Self::Fighter
        } else if record.hired() {
            Self::Hired
        } else {
            Self::Captured
        }
    }
}

/// A ship that was the player's escort when it was disabled or started to
/// break up, with the record it left the fleet with (the original's
/// +0xc8de 0 or more). A mission escort never is one: the original's
/// `_DamageShip` leaves a ship with a mission mark out (@0x3af9d).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormerEscort {
    /// The record it had in the fleet.
    pub record: Escort,
}

impl FormerEscort {
    /// What it was.
    #[must_use]
    pub fn kind(&self) -> FormerKind {
        FormerKind::of(&self.record)
    }

    /// Its record as it rejoins the fleet holding `reserves`: its class,
    /// standing order (which neither `_DamageShip` nor the rejoin touch,
    /// +0xc90a), wage and person as they were, a fighter still carried.
    #[must_use]
    pub fn rejoined(&self, reserves: Reserves) -> Escort {
        Escort {
            reserves,
            ..self.record
        }
    }
}

/// What the player is told as a former escort of `kind` rejoins:
/// `STR#` 2002 #128 for a fighter, #127 for any other (@0x33b27-0x33b5a).
#[must_use]
pub fn rejoin_message(kind: FormerKind) -> SimMessage {
    match kind {
        FormerKind::Fighter => SimMessage::FighterRepaired,
        FormerKind::Hired | FormerKind::Captured => SimMessage::EscortRepaired,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{PersonId, ShipId};
    use crate::fleet::EscortOrder;
    use crate::reserves::Gauge;
    use nova_sim_catalog::fixture::outfit;

    /// Repair Droids (437), a repair system in its third slot; a shield
    /// booster (300) with none.
    fn outfits() -> Vec<OutfitRecord> {
        vec![
            outfit(300, &[(4, 10), (-1, 0), (-1, 0), (-1, 0)]),
            outfit(437, &[(1, 0), (2, 0), (MOD_REPAIR, 0), (-1, 0)]),
        ]
    }

    #[test]
    fn a_repair_system_held_is_the_gear_in_any_mod_slot() {
        let outfits = outfits();
        let held = |id: i16, count: u16| BTreeMap::from([(OutfitId(id), count)]);
        assert!(has_gear(&held(437, 1), &outfits));
        assert!(has_gear(&held(437, 3), &outfits));
        assert!(!has_gear(&held(437, 0), &outfits), "a count of none");
        assert!(!has_gear(&held(300, 1), &outfits), "no repair mod");
        assert!(!has_gear(&BTreeMap::new(), &outfits));
        assert!(!has_gear(&held(437, 1), &[]), "no record of it");
        for slot in 0..4 {
            let mut mods = [(-1, 0); 4];
            mods[slot] = (MOD_REPAIR, 0);
            let record = outfit(500, &mods);
            assert!(has_gear(&held(500, 1), &[record]), "slot {slot}");
        }
        assert_eq!(MOD_REPAIR, 0x31);
    }

    #[test]
    fn only_a_draw_of_none_repairs_a_disabled_ship_with_armour_and_gear() {
        assert!(can_self_repair(0, Condition::Disabled, 5.0, true));
        assert!(!can_self_repair(1, Condition::Disabled, 5.0, true));
        assert!(!can_self_repair(499, Condition::Disabled, 5.0, true));
        assert!(!can_self_repair(0, Condition::Disabled, 5.0, false));
        assert!(!can_self_repair(0, Condition::Disabled, 0.0, true));
        assert!(!can_self_repair(0, Condition::Disabled, -1.0, true));
        assert!(can_self_repair(0, Condition::Disabled, 0.01, true));
        assert!(!can_self_repair(
            0,
            Condition::Dying {
                ticks_left: 3,
                starting: false
            },
            5.0,
            true
        ));
        assert!(!can_self_repair(0, Condition::Destroyed, 5.0, true));
        assert!(
            can_self_repair(0, Condition::Intact, 5.0, true),
            "the caller asks only of a disabled ship"
        );
        assert_eq!(REPAIR_ODDS, 500);
    }

    #[test]
    fn an_inhibit_off_never_holds_and_stays_off_as_it_ticks() {
        let mut inhibit = RepairInhibit::OFF;
        for _ in 0..400 {
            assert!(!inhibit.holds());
            inhibit.tick();
        }
        assert_eq!(inhibit, RepairInhibit::OFF);
        assert_eq!(RepairInhibit::default(), RepairInhibit::OFF);
    }

    /// The ticks after arming, in turn, whose `tick` leaves the inhibit
    /// holding, up to `most`.
    fn held_for(mut inhibit: RepairInhibit, most: u32) -> u32 {
        let mut held = 0;
        for _ in 0..most {
            inhibit.tick();
            if !inhibit.holds() {
                return held;
            }
            held += 1;
        }
        held
    }

    #[test]
    fn an_armed_inhibit_holds_for_300_ticks_and_opens_on_the_301st() {
        assert_eq!(REPAIR_INHIBIT_TICKS, 300);
        let armed = RepairInhibit::armed();
        assert!(armed.holds(), "as it is armed");
        assert_eq!(held_for(armed, 1000), 300);
        let mut inhibit = armed;
        for _ in 0..301 {
            inhibit.tick();
        }
        assert!(!inhibit.holds());
        inhibit.tick();
        assert!(!inhibit.holds(), "and stays open");
    }

    #[test]
    fn arming_again_restarts_the_inhibit_from_300() {
        let mut inhibit = RepairInhibit::armed();
        for _ in 0..200 {
            inhibit.tick();
        }
        inhibit = RepairInhibit::armed();
        assert_eq!(held_for(inhibit, 1000), 300);
    }

    #[test]
    fn the_armour_goes_just_above_where_the_hull_is_disabled() {
        assert!((repaired_armor(300.0, false) - 100.99).abs() < 1e-4);
        assert!((repaired_armor(300.0, true) - 31.0).abs() < 1e-4);
        assert_eq!(repaired_armor(0.0, false), 1.0);
        assert_eq!(repaired_armor(0.0, true), 1.0);
        assert_eq!(
            (TOUGH_REPAIR_SHARE, REPAIR_SHARE, REPAIR_PLUS),
            (0.1, 0.3333, 1.0)
        );
    }

    fn record(carried: bool, wage: Option<i64>) -> Escort {
        Escort {
            ship: ShipId(150),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: Some(EscortOrder::Defend),
            carried,
            wage,
            person: Some(PersonId(700)),
        }
    }

    #[test]
    fn a_former_escort_is_a_fighter_hired_or_captured_as_damage_ship_reads_it() {
        assert_eq!(FormerKind::of(&record(true, None)), FormerKind::Fighter);
        assert_eq!(
            FormerKind::of(&record(true, Some(50))),
            FormerKind::Fighter,
            "AI type 5 first"
        );
        assert_eq!(FormerKind::of(&record(false, Some(50))), FormerKind::Hired);
        assert_eq!(
            FormerKind::of(&record(false, Some(0))),
            FormerKind::Hired,
            "a wage of none is still hired"
        );
        assert_eq!(FormerKind::of(&record(false, None)), FormerKind::Captured);
        let former = FormerEscort {
            record: record(false, Some(50)),
        };
        assert_eq!(former.kind(), FormerKind::Hired);
    }

    #[test]
    fn a_former_escort_rejoins_with_its_record_and_the_reserves_it_has() {
        let reserves = Reserves {
            shield: Gauge {
                now: 0.0,
                max: 30.0,
            },
            armor: Gauge {
                now: 16.0,
                max: 45.0,
            },
            fuel: Gauge {
                now: 120.0,
                max: 300.0,
            },
        };
        for (carried, wage) in [(false, Some(50)), (false, None), (true, None)] {
            let former = FormerEscort {
                record: record(carried, wage),
            };
            assert_eq!(
                former.rejoined(reserves),
                Escort {
                    reserves,
                    ..record(carried, wage)
                }
            );
        }
    }

    #[test]
    fn a_fighter_says_fighter_repaired_and_any_other_escort_repaired() {
        assert_eq!(
            rejoin_message(FormerKind::Fighter),
            SimMessage::FighterRepaired
        );
        assert_eq!(
            rejoin_message(FormerKind::Hired),
            SimMessage::EscortRepaired
        );
        assert_eq!(
            rejoin_message(FormerKind::Captured),
            SimMessage::EscortRepaired
        );
    }
}
