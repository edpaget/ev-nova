//! The offers of one visit: the roll table `AvailRandom` reads
//! ([`RollTable`]), each offer's resolved values ([`Offer`], [`resolve`])
//! and the per-visit state that keeps them ([`Offers`]). See the
//! [module docs](super) for the original's order.

use crate::catalog::{MissionId, MissionRecord, StellarId, SystemId};
use crate::chance::{Chance, NeverFires};
use crate::date::GameDate;

use super::screens::display_order;
use super::select::{World, select_stellar};
use super::{AvailFacts, Venue, Visit, availability};
use crate::rulebook::RuleKey;

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

    /// Sets mission `id`'s roll to `roll`, as an accept sets it to 0
    /// (`_DoMissionAccept` @0xa2196): nothing for an ID outside 128-1127.
    pub fn set(&mut self, id: MissionId, roll: u16) {
        let index = usize::try_from(i32::from(id.0) - FIRST_MISSION).ok();
        if let Some(held) = index.and_then(|index| self.rolls.get_mut(index)) {
            *held = roll;
        }
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
    /// The system `_RandomizeOneMission` stores beside it (the offer
    /// slot's +0x2 for the destination, +0xa for the return): for a
    /// destination its holder's stand-in, else the system
    /// `_FindSystemFromStellar` gives ([`Whereabouts::travel`](crate::story::Whereabouts::travel)); for a
    /// return its holder as it is, else that
    /// ([`Whereabouts::system`](crate::story::Whereabouts::system)). `None` only when no system lists the
    /// stellar.
    ///
    /// The text tags, the aux ships and the mission ships don't read it:
    /// they resolve the stellar live, as the original does.
    pub system: Option<SystemId>,
}

impl Stop {
    /// `stellar` as a destination, in its travel system in `world`
    /// (@0xa15a8-0xa15ca).
    fn travel(stellar: StellarId, world: &World) -> Self {
        Self {
            stellar,
            system: world.whereabouts(stellar).travel(world.map.places()),
        }
    }

    /// `stellar` as a return, in its system in `world`, worked out afresh
    /// even when it is the destination's (@0xa1632-0xa1669).
    fn returning(stellar: StellarId, world: &World) -> Self {
        Self {
            stellar,
            system: world.whereabouts(stellar).system(),
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
/// order (see the module docs; `_RandomizeOneMission` @0xa14d1). With no
/// origin, as a person's offer may have (the -1 is passed on to
/// `_StellarsAreDistant`, @0xa151f-0xa157e), a random code draws among
/// the candidates distant from no origin, as
/// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
/// reads it: by the engine those every system at their place lists (the
/// co-located chain walk), by the other reading every candidate, though a
/// -2 by the engine's
/// [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw) tests
/// only its gate's stellar so; and a code naming the origin finds none
/// ([`select_stellar`]).
pub fn resolve(
    mission: &MissionRecord,
    venue: Venue,
    origin: Option<StellarId>,
    world: &World,
    today: GameDate,
    chance: &mut dyn Chance,
) -> Offer {
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
        destination: destination.map(|stellar| Stop::travel(stellar, world)),
        return_to: return_to.map(|stellar| Stop::returning(stellar, world)),
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
        let stellar = visit.docked.map(|site| site.id);
        if stellar.is_none() || self.rolled_at == stellar {
            return;
        }
        self.table.draw(chance);
        self.reoffer(missions, visit, chance);
        self.rolled_at = stellar;
    }

    /// Offers `missions` (by ascending ID) again without drawing the
    /// table, as a mission's success does (@0xa1b04): the mission
    /// computer's pass, then the others', each listing every mission
    /// available at its venue and resolving each listed one in turn, then
    /// putting the list in [`display_order`] (@0xa1842).
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
                        cached_avail_bits: None,
                    };
                    checks.first_refusal(&facts, chance).is_ok()
                })
                .collect();
            let resolved = listed
                .into_iter()
                .map(|mission| {
                    let offer = resolve(
                        mission,
                        venue,
                        visit.docked.map(|site| site.id),
                        &visit.world,
                        visit.today,
                        chance,
                    );
                    (offer, mission.disp_weight)
                })
                .collect();
            let offers = display_order(resolved, visit.rules.source_for(RuleKey::MissionOrder));
            match venue {
                Venue::Computer => self.computer = offers,
                Venue::Outside => self.outside = offers,
                // Never rolled: [`Venue::ORDER`] leaves it out.
                Venue::Ship => {}
            }
        }
    }

    /// Keeps only the offers still available at their venue on `visit`,
    /// each as it was rolled, the table read and nothing drawn: what an
    /// accept leaves (`_DoMissionAccept` @0xa1ffd-0xa2136, each kept only
    /// if `_IsMissionAvailable(j, 1)` passes). An offer of a mission
    /// `missions` has no record of is dropped.
    pub fn refilter(&mut self, missions: &[MissionRecord], visit: &Visit) {
        let checks = availability();
        let table = &self.table;
        let available = |offer: &Offer| {
            missions
                .iter()
                .find(|mission| mission.id == offer.mission)
                .is_some_and(|mission| {
                    let facts = AvailFacts {
                        visit,
                        mission,
                        venue: offer.venue,
                        rolls: table,
                        cached_avail_bits: None,
                    };
                    checks.first_refusal(&facts, &mut NeverFires).is_ok()
                })
        };
        self.computer.retain(|offer| available(offer));
        self.outside.retain(|offer| available(offer));
    }

    /// Where `mission`'s offer is listed at `venue`, in the order shown:
    /// the index `_OfferMissionFromPort` finds it at (-0x2a(%ebp),
    /// @0xa36f5). `None` when it is not listed there.
    #[must_use]
    pub fn position(&self, venue: Venue, mission: MissionId) -> Option<usize> {
        self.at(venue)
            .iter()
            .position(|offer| offer.mission == mission)
    }

    /// Drops the entry at `index` of `venue`'s offers, whatever mission it
    /// holds, as `_OfferMissionFromPort` shifts the outside list down from
    /// the index it found its offer at (@0xa377d-0xa37a1), and returns the
    /// mission it dropped. Past the end it drops nothing, as the original
    /// then shifts only free (-1) entries.
    pub fn drop_at(&mut self, venue: Venue, index: usize) -> Option<MissionId> {
        let list = match venue {
            Venue::Computer => &mut self.computer,
            Venue::Outside => &mut self.outside,
            Venue::Ship => return None,
        };
        (index < list.len()).then(|| list.remove(index).mission)
    }

    /// Drops the offer of `mission`, at either venue, as an accept with
    /// the offer dialog open does (`_DoMissionAccept` @0xa2018-0xa2136):
    /// whether there was one.
    pub fn drop_offer(&mut self, mission: MissionId) -> bool {
        let before = self.computer.len() + self.outside.len();
        self.computer.retain(|offer| offer.mission != mission);
        self.outside.retain(|offer| offer.mission != mission);
        self.computer.len() + self.outside.len() != before
    }

    /// Sets mission `id`'s roll in the visit's table ([`RollTable::set`]).
    pub fn set_roll(&mut self, id: MissionId, roll: u16) {
        self.table.set(id, roll);
    }

    /// Forgets the stellar the offers were rolled at, so the next landing
    /// rolls them afresh, wherever it is; the offers stay until then.
    pub fn forget(&mut self) {
        self.rolled_at = None;
    }

    /// Forgets the stellar the offers were rolled at and draws the
    /// visit's table afresh on `chance` ([`RollTable::draw`]), as a jump's
    /// arrival does (@0x6bfce, @0x6c040-0x6c066), for the offers from a
    /// ship to read until the next landing; the offers stay until then.
    pub fn redraw(&mut self, chance: &mut dyn Chance) {
        self.forget();
        self.table.draw(chance);
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
            Venue::Ship => &[],
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

    /// Whether a landing at `stellar` rolls the offers afresh
    /// ([`Offers::roll`]): unless they were last rolled there
    /// (@0x66d2b-0x66d3d), when the original calls `_RandomizeMissionData`
    /// and so refreshes the galaxy (@0xa175f).
    #[must_use]
    pub fn due_at(&self, stellar: StellarId) -> bool {
        self.rolled_at != Some(stellar)
    }
}
