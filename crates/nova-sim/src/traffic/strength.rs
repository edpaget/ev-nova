//! The escort-strength timer: a fleet that comes for a player whose
//! escorts carry a lot of cargo.
//!
//! The original keeps a countdown in a word global (0x3b5304) that four
//! places touch, and nothing else, not `M`/`N`, nor the developer moves,
//! nor a save:
//!
//! - `_ResetPlayer` (@0x1d73f-0x1d74a) disarms it (0xffff), as
//!   [`StrengthTimer::default`] is.
//! - A take-off, and a hypergate or wormhole exit, which share its path
//!   (`_HandlePlayerDockRequest` @0x66f40), arm it to `Rand(30) + 30`
//!   (@0x6706f-0x67084), after every day of the stay and before the
//!   system is set up. A hyperspace arrival arms it so too
//!   (@0x6c00c-0x6c021), before the visit's roll table, the jump's days
//!   and the setup ([`seed`]).
//! - `_EnterMoreShips`, once a frame (here a traffic tick), after the
//!   arrival roll and `_SpawnNagShip` (the shareware nag, out of scope),
//!   reads it (@0x43d1a-0x43dd7): at 0 it disarms it and fires, and
//!   otherwise counts it down. Armed at n, it fires on the (n + 1)-th tick
//!   after, whatever the system's `AvgShips`, and once only: only the
//!   16-bit wrap after about 65535 ticks could fire it again, and word
//!   widths are not copied ([`StrengthTimer`]).
//!
//! Firing sums the `Holds` of the class of each of the player's escorts
//! (ships 1-63, active, parent the player, AI type 6, no mission ship,
//! @0x43d2c-0x43d87) whose class's `InherentAI` is at most 2, a trader;
//! the player's fighters fly as AI type 5 and never count
//! ([`escort_tons_in_system`]). The class's `Holds` is the class record's (runtime
//! +0, `shïp` field 0x00), made positive when it is negative
//! (`_LoadObjectData` @0x7a6ad-0x7a6b8), not the cargo space fitted.
//! Above 199 the fleet of slot 255 comes, else above 49 the fleet of slot
//! 254, else nothing (@0x43d89-0x43dcf, [`fleet_for`]); slot i is `flët`
//! 128 + i (`_LoadObjectData` @0x7d0eb-0x7d335). It comes by
//! `_HyperSpawnFleet`, its lead flying as an interceptor (argument 3, 4)
//! in place of its own `InherentAI`, and only when the fleet exists and
//! its `AppearOn` holds (@0x41cbb-0x41cda).
//!
//! The stock data has no `flët` 382 or 383 (its fleets are 128-255), so
//! in the stock game the timer brings nothing: it is a hook for plug-ins.

use crate::catalog::{FleetId, ShipRecord};
use crate::chance::Chance;
use crate::market::{self, EscortHolds};
use crate::traffic::npc::Npc;
use crate::traffic::spawn::FIRST_FLEET_ID;

/// The least the timer is armed to: `Rand(30) + 30`.
pub const SEED_BASE: u32 = 30;
/// How many values above [`SEED_BASE`] the timer may be armed to.
pub const SEED_SPREAD: u32 = 30;
/// The most cargo the escorts may carry and bring nothing (`cmpw $0x31`).
pub const STRONG_HOLDS: i64 = 49;
/// The most cargo the escorts may carry and bring only
/// [`STRONG_FLEET`] (`cmpw $0xc7`).
pub const STRONGER_HOLDS: i64 = 199;
/// The fleet of slot 254, which escorts carrying above [`STRONG_HOLDS`]
/// bring.
pub const STRONG_FLEET: FleetId = FleetId(FIRST_FLEET_ID + 254);
/// The fleet of slot 255, which escorts carrying above
/// [`STRONGER_HOLDS`] bring.
pub const STRONGER_FLEET: FleetId = FleetId(FIRST_FLEET_ID + 255);

/// What the timer is armed to: `Rand(30) + 30`, 30 to 59 ticks.
pub fn seed(chance: &mut (impl Chance + ?Sized)) -> u32 {
    SEED_BASE + chance.below(SEED_SPREAD)
}

/// The fleet that escorts carrying `holds` tons bring, if any.
#[must_use]
pub fn fleet_for(holds: i64) -> Option<FleetId> {
    if holds > STRONGER_HOLDS {
        Some(STRONGER_FLEET)
    } else if holds > STRONG_HOLDS {
        Some(STRONG_FLEET)
    } else {
        None
    }
}

/// The tons the player's escorts in the system that count carry: the
/// class `Holds` of each of `npcs` that is the player's and no fighter,
/// whose class, in `ships`, is a trader (`InherentAI` at most
/// [`FREIGHTER_AI`](crate::escort::FREIGHTER_AI)), summed as the fleet's
/// cargo space sums them ([`market::escort_tons`]); one whose class has no
/// record counts none.
#[must_use]
pub fn escort_tons_in_system(npcs: &[Npc], ships: &[ShipRecord]) -> i64 {
    market::escort_tons(
        npcs.iter()
            .filter(|npc| npc.is_players())
            .filter_map(|npc| {
                let record = ships.iter().find(|record| record.id == npc.ship)?;
                Some(EscortHolds {
                    holds: record.fields.holds,
                    inherent_ai: record.inherent_ai,
                    carried: npc.carrier.is_some(),
                })
            }),
    )
}

/// The countdown: disarmed, or the ticks left before it fires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StrengthTimer(Option<u32>);

impl StrengthTimer {
    /// Arms it to a [`seed`] drawn on `chance`.
    pub fn arm(&mut self, chance: &mut (impl Chance + ?Sized)) {
        self.0 = Some(seed(chance));
    }

    /// Counts it down a tick, and says whether it fires: once, on the
    /// tick after it reaches 0, which disarms it.
    pub fn tick(&mut self) -> bool {
        match self.0 {
            Some(0) => {
                self.0 = None;
                true
            }
            Some(left) => {
                self.0 = Some(left - 1);
                false
            }
            None => false,
        }
    }

    /// Whether it is armed.
    #[must_use]
    pub fn armed(self) -> bool {
        self.0.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::catalog::ShipId;
    use crate::combat::ShipRef;
    use crate::escort::EscortDuty;
    use crate::handling::ShipFields;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, npc, ship};

    #[test]
    fn the_named_values() {
        assert_eq!((SEED_BASE, SEED_SPREAD), (30, 30));
        assert_eq!((STRONG_HOLDS, STRONGER_HOLDS), (49, 199));
        assert_eq!((STRONG_FLEET, STRONGER_FLEET), (FleetId(382), FleetId(383)));
    }

    #[test]
    fn the_seed_draws_below_30_and_adds_30() {
        let mut low = Draws::of(&[0]);
        assert_eq!(seed(&mut low), 30);
        assert_eq!(low.asked, [30]);
        let mut high = Draws::of(&[29]);
        assert_eq!(seed(&mut high), 59);
        assert_eq!(high.asked, [30]);
    }

    #[test]
    fn above_49_tons_bring_382_and_above_199_bring_383() {
        for holds in [i64::MIN, -1, 0, 1, 49] {
            assert_eq!(fleet_for(holds), None, "{holds}");
        }
        for holds in [50, 120, 199] {
            assert_eq!(fleet_for(holds), Some(FleetId(382)), "{holds}");
        }
        for holds in [200, 5000, i64::MAX] {
            assert_eq!(fleet_for(holds), Some(FleetId(383)), "{holds}");
        }
    }

    #[test]
    fn armed_at_n_it_fires_once_on_the_tick_after_n() {
        for n in [0, 1, 29] {
            let mut timer = StrengthTimer::default();
            timer.arm(&mut Draws::of(&[n]));
            assert!(timer.armed());
            for tick in 0..30 + n {
                assert!(!timer.tick(), "armed at {n}, tick {tick}");
            }
            assert!(timer.tick(), "armed at {n}, fires");
            assert!(!timer.armed(), "fired, disarmed");
            for _ in 0..1000 {
                assert!(!timer.tick(), "once only");
            }
        }
    }

    #[test]
    fn disarmed_it_never_fires() {
        let mut timer = StrengthTimer::default();
        assert!(!timer.armed());
        for _ in 0..1000 {
            assert!(!timer.tick());
        }
    }

    #[test]
    fn arming_again_starts_the_countdown_afresh() {
        let mut timer = StrengthTimer::default();
        timer.arm(&mut Draws::of(&[0]));
        for _ in 0..20 {
            timer.tick();
        }
        timer.arm(&mut Draws::of(&[0]));
        for _ in 0..30 {
            assert!(!timer.tick());
        }
        assert!(timer.tick());
    }

    /// Ship class `id` carrying `holds` tons, of `InherentAI`
    /// `inherent_ai`.
    fn class(id: i16, holds: i16, inherent_ai: i16) -> ShipRecord {
        ShipRecord {
            inherent_ai,
            ..ship(id, ShipFields { holds, ..FAST })
        }
    }

    /// An NPC of ship `id`, the player's escort when `escort`.
    fn flying(n: u32, id: i16, escort: bool) -> Npc {
        Npc {
            ship: ShipId(id),
            escort: escort.then_some(EscortDuty {
                slot: 2,
                ships: 2,
                spacing: 30.0,
                order: None,
            }),
            ..npc(n, ShipStats::new(FAST, &[]))
        }
    }

    #[test]
    fn the_escorts_of_trader_classes_count_their_classes_holds() {
        let ships = [
            class(128, 10, 1),
            class(129, 20, 2),
            class(130, 40, 0),
            class(131, 80, -1),
        ];
        let npcs: Vec<Npc> = (0..4).map(|n| flying(n, 128 + n as i16, true)).collect();
        assert_eq!(escort_tons_in_system(&npcs, &ships), 150);
        assert_eq!(escort_tons_in_system(&npcs[1..2], &ships), 20);
    }

    #[test]
    fn two_escorts_of_one_class_count_it_twice() {
        let ships = [class(128, 30, 1)];
        let npcs = [flying(0, 128, true), flying(1, 128, true)];
        assert_eq!(escort_tons_in_system(&npcs, &ships), 60);
    }

    #[test]
    fn a_class_of_negative_holds_counts_them_made_positive() {
        let ships = [class(128, -70, 1)];
        assert_eq!(escort_tons_in_system(&[flying(0, 128, true)], &ships), 70);
    }

    #[test]
    fn warships_fighters_traffic_and_classes_without_a_record_count_none() {
        let ships = [class(128, 100, 3), class(129, 100, 4), class(130, 100, 1)];
        let fighter = Npc {
            carrier: Some(Carrier {
                ship: ShipRef::Player,
                window: 10.0,
                reach: 10.0,
            }),
            ..flying(2, 130, true)
        };
        let npcs = [
            flying(0, 128, true),
            flying(1, 129, true),
            fighter,
            flying(3, 130, false),
            flying(4, 140, true),
        ];
        assert_eq!(escort_tons_in_system(&npcs, &ships), 0);
    }
}
