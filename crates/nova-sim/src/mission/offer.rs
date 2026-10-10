//! The offers of one visit: the roll table `AvailRandom` reads
//! ([`RollTable`]), each offer's resolved values ([`Offer`], [`resolve`])
//! and the per-visit state that keeps them ([`Offers`]). See the
//! [module docs](super) for the original's order.

use crate::catalog::{MissionId, MissionRecord, StellarId, SystemId};
use crate::chance::Chance;
use crate::date::GameDate;

use super::select::{World, select_stellar};
use super::{AvailFacts, Venue, Visit, availability};

/// How many rolls the table holds: one for each mission index, 0-999
/// (`_missionRandom`, 0x20f240).
pub const ROLLED: usize = 1000;

/// The lowest `mïsn` ID: mission index 0.
const FIRST_MISSION: i32 = 128;

/// The visit's rolls, one for each mission index (`mïsn` ID - 128), each
/// 1 to 100 once drawn, 0 before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollTable {
    rolls: Vec<u16>,
}

impl Default for RollTable {
    fn default() -> Self {
        Self {
            rolls: vec![0; ROLLED],
        }
    }
}

impl RollTable {
    /// Draws every roll afresh, in index order: `Rand(100) + 1` each
    /// (`_HandlePlayerDockRequest` @0x66d3f-0x66d60).
    pub fn draw(&mut self, chance: &mut dyn Chance) {
        for roll in &mut self.rolls {
            *roll = chance.roll(100).saturating_add(1);
        }
    }

    /// This table with mission `id`'s roll set to `roll`.
    #[cfg(test)]
    pub(crate) fn with(mut self, id: MissionId, roll: u16) -> Self {
        let index = usize::try_from(i32::from(id.0) - FIRST_MISSION).expect("a mission index");
        self.rolls[index] = roll;
        self
    }

    /// Mission `id`'s roll: `None` for an ID outside 128-1127, which the
    /// table has no roll for.
    #[must_use]
    pub fn roll(&self, id: MissionId) -> Option<u16> {
        let index = usize::try_from(i32::from(id.0) - FIRST_MISSION).ok()?;
        self.rolls.get(index).copied()
    }
}

/// A stellar an offer names, with the system it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stop {
    /// The stellar.
    pub stellar: StellarId,
    /// The lowest-ID visible system that lists it, `None` when none does
    /// (a fixed stellar of a hidden system, or none at all).
    pub system: Option<SystemId>,
}

impl Stop {
    /// `stellar`, in the system `world` gives it.
    fn of(stellar: StellarId, world: &World) -> Self {
        Self {
            stellar,
            system: world.system_of(stellar),
        }
    }
}

/// A mission offered on a visit, with the values the visit rolled for it
/// (`_briefMissionInfo`, 0x35d800).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The mission.
    pub mission: MissionId,
    /// Where it is offered.
    pub venue: Venue,
    /// Where it goes: none for `TravelStel` -1.
    pub destination: Option<Stop>,
    /// Where it is completed: the destination for `ReturnStel` -1.
    pub return_to: Option<Stop>,
    /// The commodity or `jünk` it carries, raw: none for a `CargoType`
    /// outside 0-999 but 1000.
    pub cargo_type: Option<i16>,
    /// The tons it carries, drawn even with no cargo.
    pub cargo_tons: i64,
    /// Its pay: `PayVal`, raw.
    pub pay: i64,
    /// The last day it may be completed: none for a `TimeLimit` of 0 or
    /// below.
    pub deadline: Option<GameDate>,
}

/// `CargoType`: a random commodity.
const RANDOM_CARGO: i16 = 1000;
/// The standard commodities a random cargo is drawn from.
const COMMODITIES: u16 = 6;
/// `CargoQty`: none.
const NO_TONS: i16 = -1;

/// `today` plus `days` days.
pub(super) fn after(today: GameDate, days: i16) -> GameDate {
    (0..days).fold(today, |date, _| date.next_day())
}

/// `mission`'s values for an offer at `venue`, from `origin`, the stellar
/// docked at, in `world` on `today`, drawing on `chance` in the original's
/// order (see the module docs; `_RandomizeOneMission` @0xa14d1).
pub fn resolve(
    mission: &MissionRecord,
    venue: Venue,
    origin: StellarId,
    world: &World,
    today: GameDate,
    chance: &mut dyn Chance,
) -> Offer {
    let origin = Some(origin);
    let destination = match mission.travel_stel {
        -1 => None,
        code => select_stellar(code, origin, None, world, chance),
    };
    let return_to = match mission.return_stel {
        -1 => destination,
        code => select_stellar(code, origin, destination, world, chance),
    };
    let cargo_type = match mission.cargo_type {
        RANDOM_CARGO => i16::try_from(chance.roll(COMMODITIES)).ok(),
        cargo @ 0..RANDOM_CARGO => Some(cargo),
        _ => None,
    };
    let cargo_tons = match mission.cargo_qty {
        NO_TONS => 0,
        tons @ 0.. => i64::from(tons),
        range => {
            let width = range.unsigned_abs();
            i64::from(chance.below(u32::from(width))) + i64::from(width / 2)
        }
    };
    Offer {
        mission: mission.id,
        venue,
        destination: destination.map(|stellar| Stop::of(stellar, world)),
        return_to: return_to.map(|stellar| Stop::of(stellar, world)),
        cargo_type,
        cargo_tons,
        pay: i64::from(mission.pay_val),
        deadline: (mission.time_limit > 0).then(|| after(today, mission.time_limit)),
    }
}

/// The offers of the visit under way, and the stellar they were rolled
/// at: the caller's state, never saved (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Offers {
    rolled_at: Option<StellarId>,
    table: RollTable,
    computer: Vec<Offer>,
    outside: Vec<Offer>,
}

impl Offers {
    /// Rolls the offers at the stellar `visit` docks at, unless they were
    /// rolled there last, when it does nothing and draws nothing: draws
    /// the table, then offers `missions` (by ascending ID) as
    /// [`Offers::reoffer`] does, and keeps the stellar.
    pub fn roll(&mut self, missions: &[MissionRecord], visit: &Visit, chance: &mut dyn Chance) {
        let stellar = visit.docked.id;
        if self.rolled_at == Some(stellar) {
            return;
        }
        self.table.draw(chance);
        self.reoffer(missions, visit, chance);
        self.rolled_at = Some(stellar);
    }

    /// Offers `missions` (by ascending ID) again without drawing the
    /// table, as a mission's success does (@0xa1b04): the mission
    /// computer's pass, then the others', each listing every mission
    /// available at its venue and resolving each listed one in turn.
    pub fn reoffer(&mut self, missions: &[MissionRecord], visit: &Visit, chance: &mut dyn Chance) {
        let checks = availability();
        for venue in Venue::ORDER {
            let listed: Vec<&MissionRecord> = missions
                .iter()
                .filter(|mission| {
                    let facts = AvailFacts {
                        visit,
                        mission,
                        venue,
                        rolls: &self.table,
                    };
                    checks.first_refusal(&facts, chance).is_ok()
                })
                .collect();
            let offers = listed
                .into_iter()
                .map(|mission| {
                    resolve(
                        mission,
                        venue,
                        visit.docked.id,
                        &visit.world,
                        visit.today,
                        chance,
                    )
                })
                .collect();
            match venue {
                Venue::Computer => self.computer = offers,
                Venue::Outside => self.outside = offers,
            }
        }
    }

    /// Forgets the stellar the offers were rolled at, so the next landing
    /// rolls them afresh, wherever it is; the offers stay until then.
    pub fn forget(&mut self) {
        self.rolled_at = None;
    }

    /// Empties both venues' offers, keeping the stellar they were rolled
    /// at, as a landed `M` or `N` does (@0x1584b, @0x15949).
    pub fn empty(&mut self) {
        self.computer.clear();
        self.outside.clear();
    }

    /// The offers at `venue`, in the order rolled.
    #[must_use]
    pub fn at(&self, venue: Venue) -> &[Offer] {
        match venue {
            Venue::Computer => &self.computer,
            Venue::Outside => &self.outside,
        }
    }

    /// The visit's roll table.
    #[must_use]
    pub fn table(&self) -> &RollTable {
        &self.table
    }

    /// The stellar the offers were last rolled at, if they are kept.
    #[must_use]
    pub fn rolled_at(&self) -> Option<StellarId> {
        self.rolled_at
    }
}
