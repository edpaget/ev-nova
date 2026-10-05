//! Ships and governments the AI types' tests share.

use crate::ai::{PlayerSide, Surroundings};
use crate::catalog::{GovtId, GovtRecord, LandingSite};
use crate::combat::ShipRef;
use crate::combat::hull::{Condition, HullSpec};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::reserves::Reserves;
use crate::stats::ShipStats;
use crate::testkit::{FAST, govt, planet};
use crate::traffic::npc::{AiType, Npc, NpcId};

/// Traders: class 1, `CrimeTol` 6.
pub(crate) const TRADERS: GovtId = GovtId(140);
/// Police allied with the traders.
pub(crate) const POLICE: GovtId = GovtId(141);
/// Pirates at war with the traders.
pub(crate) const PIRATES: GovtId = GovtId(142);
/// Nobody's friend or foe.
pub(crate) const NEUTRAL: GovtId = GovtId(143);

/// The four governments, the traders' with `flags` and the police's with
/// `police_flags`.
pub(crate) fn govts(flags: u16, police_flags: u16) -> Governments {
    Governments::new([
        GovtRecord {
            flags,
            crime_tol: 6,
            classes: [1, -1, -1, -1],
            ..govt(140)
        },
        GovtRecord {
            flags: police_flags,
            crime_tol: 6,
            allies: [1, -1, -1, -1],
            classes: [2, -1, -1, -1],
            ..govt(141)
        },
        GovtRecord {
            enemies: [1, -1, -1, -1],
            classes: [3, -1, -1, -1],
            ..govt(142)
        },
        govt(143),
    ])
}

/// NPC `id` of `govt` and `ai_type` at (`x`, `y`), at rest facing up, of
/// strength 100 and aggression 2, its reserves full (30 shield).
pub(crate) fn ship(id: u32, govt: GovtId, ai_type: AiType, x: f32, y: f32) -> Npc {
    let mut npc = crate::testkit::npc(id, ShipStats::new(FAST, &[]));
    npc.govt = Some(govt);
    npc.ai_type = ai_type;
    npc.state.position = Vec2::new(x, y);
    npc.hull.strength = 100.0;
    npc.aggression = 2;
    npc
}

/// The player at (`x`, `y`), at rest, of strength 100.
pub(crate) fn player(x: f32, y: f32) -> PlayerSide {
    PlayerSide {
        state: ShipState {
            position: Vec2::new(x, y),
            ..ShipState::default()
        },
        condition: Condition::Intact,
        reserves: Reserves::full(100.0, 100.0, 100.0),
        hull: HullSpec {
            strength: 100.0,
            ..HullSpec::default()
        },
        handling: ShipStats::new(FAST, &[]).handling,
    }
}

/// A landable planet, 128, far off.
pub(crate) fn sites() -> Vec<LandingSite> {
    vec![planet(128, 3000.0, 3000.0)]
}

/// `npcs` among `sites`, with the player at (`x`, `y`), in a traders'
/// system where the player's record is `record`.
pub(crate) fn around<'a>(
    sites: &'a [LandingSite],
    npcs: &'a [Npc],
    govts: &'a Governments,
    (x, y): (f32, f32),
    record: i16,
) -> Surroundings<'a> {
    Surroundings {
        player: Some(player(x, y)),
        govts,
        system_govt: Some(TRADERS),
        record,
        ..Surroundings::new(sites, npcs)
    }
}

/// NPC `id` as a ship.
pub(crate) fn n(id: u32) -> ShipRef {
    ShipRef::Npc(NpcId(id))
}
