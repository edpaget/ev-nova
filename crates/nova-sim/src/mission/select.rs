//! Choosing a mission's stellars, as the original's
//! `_SelectMissionStellar` (@0x9a26a) does: what a `TravelStel` or
//! `ReturnStel` code names ([`StellarCode`]), the stellars a random code
//! may land on ([`candidates`]), and the one drawn ([`select_stellar`]),
//! as [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw) reads
//! the original's -2 draw. Whether a stellar is distant from the origin
//! is as [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
//! reads `_StellarsAreDistant` (@0x9cb9-0x9ef6): by the engine it
//! compares the places of the two systems (their co-located chain heads)
//! and their own Con links, and asks that every system at the stellar's
//! place list it, the last alone with no origin, as availability asks.
//! Whether an ally code, 15000-19999, also takes a stellar by the system
//! at its index is as
//! [`mission_ally_index`](crate::RuleKey::MissionAllyIndex) reads the
//! original's test (@0x9a7af-0x9a7c8, @0x9a8e5-0x9a8fa).
//!
//! The galaxy it chooses from is a [`World`]: every active stellar,
//! overlaid ([`stellars_of`]), where every listed stellar is
//! ([`placements`]), the overlaid map and the governments.

use crate::catalog::{GovtId, ListedStellar, StellarId, SystemId};
use crate::chance::Chance;
use crate::gate::GateKind;
use crate::govt::Governments;
use crate::hyperspace::StarMap;
use crate::landing::{StellarFlags, landable};
use crate::rulebook::RuleSource;
use crate::story::{OverlaidGalaxy, Whereabouts};

/// A stellar of a visible system, overlaid, with the system it belongs
/// to: what choosing it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stellar {
    /// The stellar.
    pub id: StellarId,
    /// The system that holds it ([`OverlaidGalaxy::holder`]): its owner,
    /// the lowest-ID visible system that lists it and is not shadowed at
    /// its place, else the player's system when that lists it. This is
    /// the system `_StellarsAreDistant` reads (+0x14).
    pub system: SystemId,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its government, the overlay's if it has one: `None` when it is
    /// independent.
    pub govt: Option<GovtId>,
    /// Whether it is destroyed.
    pub destroyed: bool,
    /// Whether every system at its system's place, hidden or not, lists
    /// it: the co-located chain walk of `_StellarsAreDistant`
    /// (@0x9e60-0x9ee4), over the chain `_LoadObjectData` builds of every
    /// system at a place (@0x783d9-0x78517).
    pub listed_at_place: bool,
    /// The government the system record at its index holds, the `sÿst`
    /// with its ID, which an ally code's test reads beside `_GovtAllies`
    /// (`_SelectMissionStellar` @0x9a7af-0x9a7c8, @0x9a8e5-0x9a8fa; see
    /// `index_govt`): `None` when that slot matches no government.
    pub index_govt: Option<GovtId>,
}

/// The highest system ID whose slot `_LoadObjectData` writes, for a
/// catalog of systems `ids`. It loads `sÿst` 128 + i into slot i, from 0
/// up, until it has loaded as many as `CountResources` counts
/// (@0x77a14, @0x78393-0x783a1) or reached the array's end, 2175
/// (@0x783b3): its highest ID when every one is in 128-2175, else 2175,
/// as one it can never load keeps it going. With no system, only the
/// first slot.
fn written_to(ids: impl IntoIterator<Item = SystemId>) -> SystemId {
    const LAST: SystemId = SystemId(2175);
    let mut highest = SystemId(128);
    for id in ids {
        if !(SystemId(128)..=LAST).contains(&id) {
            return LAST;
        }
        highest = highest.max(id);
    }
    highest
}

/// The government in the system array's slot at stellar `id`'s index,
/// the catalog's `govt` of the system with that ID, as `_LoadObjectData`
/// leaves it and as `_SelectMissionStellar` reads it (+0x8 @0x9a7ba,
/// @0x9a8f2); the loader writes `written_to` and below. Nothing writes a
/// system's +0x8 in play, so no overlay counts, and no visibility test is
/// made of it.
///
/// - A system with that ID: its catalog `Govt`, kept when it is 128-383
///   and otherwise -1 (@0x77c2f-0x77c33, the range check @0x77d6b-0x77d81),
///   so `None` for an independent one or one out of range.
/// - A gap, no system with that ID at or below `written_to`: the sentinel
///   0x8001 (@0x77a92), which matches no government, `None`.
/// - Past `written_to`: a slot the loader never writes, of the array's
///   plain `_NewPtr(0xfe000)` (`_InitObjects` @0x1b7f9-0x1b80d). This
///   rests on that memory being zero in practice, as a fresh allocation
///   of that size is on OS X: government index 0, `GovtId(128)`. Nothing
///   in the original guarantees it.
fn index_govt(
    id: StellarId,
    catalog: impl Fn(SystemId) -> Option<GovtId>,
    written_to: SystemId,
) -> Option<GovtId> {
    let system = SystemId(id.0);
    if system > written_to {
        return Some(GovtId(128));
    }
    catalog(system).filter(|govt| (128..=383).contains(&govt.0))
}

/// Every stellar of `listed` that is active in `galaxy` with the player in
/// `player`, by ascending ID, each in the system that holds it
/// ([`OverlaidGalaxy::active_holder`], the [`OverlaidGalaxy::holder`]
/// while it is visible: its owner, the lowest-ID visible system
/// that lists it and is not shadowed at its place, by
/// `colocated_resolution`, else the player's system when that lists it),
/// with the overlay's government and destroyed state, and whether every
/// system at its system's place lists it ([`Stellar::listed_at_place`]),
/// and the catalog government of the system at its index
/// ([`Stellar::index_govt`]).
///
/// These are `_SelectMissionStellar`'s candidates: every scan of it tests
/// a stellar's active byte (+0x44) before its distance, which
/// `_HandleStellarSystemVisibility` sets for a stellar that exists (+0x45:
/// loaded and listed by a system, `_LoadObjectData` @0x77584-0x7758b,
/// @0x779bf-0x779c6, @0x789ad-0x789c6) whose owner is visible
/// (@0x32c42-0x32cdf), after its rescue gives a stellar with no owner to
/// the player's system when that lists it (@0x32c58-0x32cb3). So a
/// stellar rescued into the player's system is a candidate while that
/// system is visible, and its system for `_StellarsAreDistant` is the
/// rescued holder; a stellar no visible system holds is left out, and
/// `_StellarsAreDistant`'s refusal of a stellar that does not exist
/// (@0x9cea-0x9cee) is never reached.
#[must_use]
pub fn stellars_of(
    galaxy: &OverlaidGalaxy,
    listed: &[ListedStellar],
    player: SystemId,
) -> Vec<Stellar> {
    let map = galaxy.map();
    let written_to = written_to(galaxy.catalog_systems());
    listed
        .iter()
        .filter_map(|stellar| {
            let id = stellar.id;
            let system = galaxy.active_holder(&stellar.systems, player)?;
            Some(Stellar {
                id,
                system,
                flags: stellar.flags,
                flags2: stellar.flags2,
                govt: galaxy.overlay().stellar(id).govt.unwrap_or(stellar.govt),
                destroyed: galaxy.destroyed(id),
                listed_at_place: map
                    .places()
                    .at(system)
                    .all(|at| stellar.systems.contains(&at)),
                index_govt: index_govt(id, |system| galaxy.record_govt(system), written_to),
            })
        })
        .collect()
}

/// Where a listed stellar is, held or not: what a mission's stops, its
/// availability and its aux and mission ships read of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The stellar.
    pub id: StellarId,
    /// Where it is.
    pub at: Whereabouts,
}

/// Where each stellar of `listed` is in `galaxy`, with the player in
/// `player` ([`OverlaidGalaxy::whereabouts`]), by ascending ID: every one,
/// held or not.
#[must_use]
pub fn placements(
    galaxy: &OverlaidGalaxy,
    listed: &[ListedStellar],
    player: SystemId,
) -> Vec<Placed> {
    listed
        .iter()
        .map(|stellar| Placed {
            id: stellar.id,
            at: galaxy.whereabouts(&stellar.systems, player),
        })
        .collect()
}

/// The galaxy a mission's stellars are chosen from.
#[derive(Clone, Copy, Debug)]
pub struct World<'a> {
    /// Every active stellar, by ascending ID ([`stellars_of`]).
    pub stellars: &'a [Stellar],
    /// Where every listed stellar is, by ascending ID ([`placements`]).
    pub placed: &'a [Placed],
    /// The overlaid map: the visible systems and their links.
    pub map: &'a StarMap,
    /// The governments and their relations.
    pub govts: &'a Governments,
    /// How a random -2 is drawn:
    /// [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw), from
    /// the rule set the caller holds ([`select_stellar`]).
    pub draw: RuleSource,
    /// How distance from an origin is told:
    /// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance),
    /// from the rule set the caller holds (see [`candidates`]).
    pub distance: RuleSource,
    /// Whether an ally code also takes a stellar by the system at its
    /// index: [`mission_ally_index`](crate::RuleKey::MissionAllyIndex),
    /// from the rule set the caller holds (see [`Kind::Ally`]).
    pub ally_index: RuleSource,
}

impl World<'_> {
    /// Stellar `id`, when a visible system lists it.
    #[must_use]
    pub fn stellar(&self, id: StellarId) -> Option<&Stellar> {
        self.stellars
            .binary_search_by_key(&id, |stellar| stellar.id)
            .ok()
            .and_then(|at| self.stellars.get(at))
    }

    /// The system that holds stellar `id`, when it is active
    /// ([`Stellar::system`]).
    #[must_use]
    pub fn system_of(&self, id: StellarId) -> Option<SystemId> {
        self.stellar(id).map(|stellar| stellar.system)
    }

    /// Where stellar `id` is ([`placements`]): nowhere, the default, when
    /// no system lists it.
    #[must_use]
    pub fn whereabouts(&self, id: StellarId) -> Whereabouts {
        self.placed
            .binary_search_by_key(&id, |placed| placed.id)
            .map_or_else(|_| Whereabouts::default(), |at| self.placed[at].at)
    }

    /// Whether `stellar` is distant from `origin`, none as availability
    /// asks, as `self.distance` reads
    /// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
    /// (`_StellarsAreDistant` @0x9cb9-0x9ef6).
    ///
    /// By the engine, as the original tells it. From an origin in a
    /// visible system, `stellar` is distant when its system is not at the
    /// origin's place (the same co-located chain head, @0x9d35-0x9d9d),
    /// neither system's own Con links reach a system at the other's place
    /// (@0x9da3-0x9e1c), and every system at its system's place lists it
    /// ([`Stellar::listed_at_place`], the chain walk @0x9e60-0x9ee4). With
    /// no origin only that last test is made (@0x9e20-0x9e5c). From an
    /// origin of no visible system nothing is distant: its system is -1
    /// (@0x9d1d-0x9d24).
    ///
    /// By the other reading, phase 1's test: `stellar` is in another
    /// system, one the map does not link with the origin's either way;
    /// with no origin, or an origin of no visible system, it is not
    /// tested.
    fn distant(&self, origin: Option<StellarId>, stellar: &Stellar) -> bool {
        let from = origin.map(|origin| self.system_of(origin));
        let to = stellar.system;
        match self.distance {
            RuleSource::Engine => match from {
                None => stellar.listed_at_place,
                Some(None) => false,
                Some(Some(from)) => {
                    let places = self.map.places();
                    // Whether one of `a`'s own Con links is at `b`'s place.
                    let reaches = |a: SystemId, b: SystemId| {
                        self.map
                            .listed_links(a)
                            .iter()
                            .any(|&link| places.together(link, b))
                    };
                    !places.together(from, to)
                        && !reaches(from, to)
                        && !reaches(to, from)
                        && stellar.listed_at_place
                }
            },
            RuleSource::Bible => from
                .flatten()
                .is_none_or(|from| from != to && !self.map.neighbours(from).contains(&to)),
        }
    }
}

/// What a `TravelStel` or `ReturnStel` code chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StellarCode {
    /// The origin: -1 and -4 (@0x9a282-0x9a2a5), 1-127 (@0x9a543), and
    /// every code none of the others reads.
    Origin,
    /// That stellar, 128-2175, drawing nothing (@0x9a596).
    Fixed(StellarId),
    /// One of the candidates of a kind, drawn.
    Random(Kind),
}

/// The stellars a random code takes, beside the tests every random code
/// makes (see [`candidates`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// -2: inhabited and not a station (@0x9a2ab-0x9a3e4).
    Inhabited,
    /// -3: uninhabited and not a station (@0x9a3e9-0x9a53d).
    Uninhabited,
    /// 9999-14999: governed by this government, `None` for 9999's
    /// independent, and inhabited unless it is derelict
    /// (@0x9a5a9-0x9a74f).
    Govt(Option<GovtId>),
    /// 15000-19999: of another government, allied with this one,
    /// inhabited (@0x9a755-0x9a99a). By the engine's reading of
    /// [`mission_ally_index`](crate::RuleKey::MissionAllyIndex), a stellar
    /// of another government is also taken when the system at its index
    /// has this one ([`Stellar::index_govt`], @0x9a7af-0x9a7c8,
    /// @0x9a8e5-0x9a8fa), allied or not.
    Ally(GovtId),
    /// 20000-24999: not of this government, an independent included,
    /// inhabited (@0x9a9a0-0x9aaff).
    NotGovt(GovtId),
    /// 25000-29999: of another government, an enemy of this one,
    /// inhabited (@0x9ab01-0x9acf7).
    Enemy(GovtId),
    /// 30000-30999: of another government, sharing a class with this
    /// one, inhabited (@0x9acfd-0x9aeeb).
    SharedClass(GovtId),
    /// 31000-31999: of another government, sharing no class with this
    /// one, inhabited (@0x9aef1-0x9b142).
    NoSharedClass(GovtId),
}

/// What a government code, 9999-31999, names: a relation and the
/// government (`gövt` index + 128) it is to, as both a mission's stellar
/// codes ([`StellarCode`]) and its `ShipSyst` read them. What each
/// relation takes is the reader's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GovtCode {
    /// 9999-14999: this government, `None` for 9999's independent.
    Govt(Option<GovtId>),
    /// 15000-19999: allied with this government.
    Ally(GovtId),
    /// 20000-24999: not this government.
    NotGovt(GovtId),
    /// 25000-29999: an enemy of this government.
    Enemy(GovtId),
    /// 30000-30999: sharing a class with this government.
    SharedClass(GovtId),
    /// 31000-31999: sharing no class with this government.
    NoSharedClass(GovtId),
}

impl GovtCode {
    /// What `code` names; `None` outside 9999-31999.
    #[must_use]
    pub fn of(code: i16) -> Option<Self> {
        // The government `code`'s index from `base` names.
        let govt = |base: i16| GovtId(code - base + 128);
        Some(match code {
            9999 => Self::Govt(None),
            10000..=14999 => Self::Govt(Some(govt(10000))),
            15000..=19999 => Self::Ally(govt(15000)),
            20000..=24999 => Self::NotGovt(govt(20000)),
            25000..=29999 => Self::Enemy(govt(25000)),
            30000..=30999 => Self::SharedClass(govt(30000)),
            31000..=31999 => Self::NoSharedClass(govt(31000)),
            _ => return None,
        })
    }
}

impl From<GovtCode> for Kind {
    fn from(code: GovtCode) -> Self {
        match code {
            GovtCode::Govt(of) => Self::Govt(of),
            GovtCode::Ally(of) => Self::Ally(of),
            GovtCode::NotGovt(of) => Self::NotGovt(of),
            GovtCode::Enemy(of) => Self::Enemy(of),
            GovtCode::SharedClass(of) => Self::SharedClass(of),
            GovtCode::NoSharedClass(of) => Self::NoSharedClass(of),
        }
    }
}

impl StellarCode {
    /// What `code` chooses.
    #[must_use]
    pub fn of(code: i16) -> Self {
        match code {
            -2 => Self::Random(Kind::Inhabited),
            -3 => Self::Random(Kind::Uninhabited),
            128..=2175 => Self::Fixed(StellarId(code)),
            _ => GovtCode::of(code).map_or(Self::Origin, |code| Self::Random(code.into())),
        }
    }
}

/// Whether `stellar` has the `Flags` `flag`.
fn has(stellar: &Stellar, flag: u32) -> bool {
    stellar.flags & flag != 0
}

/// Whether `stellar` is inhabited: its uninhabited flag clear.
fn inhabited(stellar: &Stellar) -> bool {
    !has(stellar, StellarFlags::UNINHABITED)
}

/// Whether `stellar` is "normal landable": landable, and no hypergate or
/// wormhole (`_StellarIsNormalLandable` @0xa9f8).
fn normal_landable(stellar: &Stellar) -> bool {
    landable(stellar.flags, stellar.destroyed) && GateKind::of(stellar.flags2).is_none()
}

impl Kind {
    /// Whether `stellar` is of this kind, the governments as `world`'s
    /// say, an ally code's index test as `world.ally_index` reads
    /// [`mission_ally_index`](crate::RuleKey::MissionAllyIndex).
    fn takes(self, stellar: &Stellar, world: &World) -> bool {
        let govts = world.govts;
        let govt = stellar.govt;
        // Of a government, and another than `of`.
        let other = |of: GovtId| govt.is_some_and(|govt| govt != of);
        match self {
            Self::Inhabited => inhabited(stellar) && !has(stellar, StellarFlags::STATION),
            Self::Uninhabited => !inhabited(stellar) && !has(stellar, StellarFlags::STATION),
            Self::Govt(of) => govt == of && (inhabited(stellar) || govts.derelict(of)),
            Self::Ally(of) => {
                let by_index =
                    world.ally_index == RuleSource::Engine && stellar.index_govt == Some(of);
                other(of) && inhabited(stellar) && (by_index || govts.allies(govt, Some(of)))
            }
            Self::NotGovt(of) => govt != Some(of) && inhabited(stellar),
            Self::Enemy(of) => other(of) && inhabited(stellar) && govts.enemies(govt, Some(of)),
            Self::SharedClass(of) => {
                other(of) && inhabited(stellar) && govts.shared_class(govt, Some(of))
            }
            Self::NoSharedClass(of) => {
                other(of) && inhabited(stellar) && !govts.shared_class(govt, Some(of))
            }
        }
    }
}

/// The stellars a random code of `kind` may choose, by ascending ID,
/// drawing nothing. A candidate is in a visible system (every stellar of
/// the `world` is), is normal landable (landable, and no hypergate or
/// wormhole: `_StellarIsNormalLandable` @0xa9f8), is neither `origin` nor
/// `exclude`, is of the kind, and is distant from `origin` as
/// `world.distance` reads
/// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
/// (`World::distant`). With no `origin`, as availability asks, none is
/// excluded for it, and distance is by the engine only the co-located
/// chain walk (`_StellarsAreDistant` @0x9e20-0x9ee4), by the other
/// reading not tested.
#[must_use]
pub fn candidates(
    kind: Kind,
    origin: Option<StellarId>,
    exclude: Option<StellarId>,
    world: &World,
) -> Vec<StellarId> {
    world
        .stellars
        .iter()
        .filter(|stellar| {
            let id = Some(stellar.id);
            id != origin
                && id != exclude
                && normal_landable(stellar)
                && kind.takes(stellar, world)
                && world.distant(origin, stellar)
        })
        .map(|stellar| stellar.id)
        .collect()
}

/// The stellars a random code of `kind` draws from by the engine's
/// reading of [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw),
/// by ascending ID, drawing nothing: none when its scan finds nothing. For every kind but -2 these
/// are its [`candidates`], as the original's scan and draw make the same
/// tests and the draw tests the distance of the stellar drawn.
///
/// For -2 the scan (@0x9a2c7-0x9a330) is only a gate: it needs a stellar,
/// neither `origin` nor `exclude`, normal landable and inhabited, a
/// station or not, distant from `origin` as `World::distant` tells it,
/// with no origin too (the chain walk alone, @0x9e20-0x9ee4). A stellar
/// of no visible system is never distant (its system is -1:
/// `_StellarsAreDistant` @0x9d0e-0x9d17), so the world's stellars are all
/// it can find; whether an origin of no visible system finds none
/// (@0x9d1d-0x9d24) is as
/// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
/// reads it. Once it finds one, the draw (@0x9a365-0x9a3e2) takes
/// every stellar of the kind, neither `origin` nor `exclude`, normal
/// landable, at any distance: it tests the distance of the scan's first
/// hit, not of the stellar drawn (@0x9a3ce-0x9a3db). Where that leaves
/// none, the original redraws forever; this gives none, so the origin is
/// chosen.
fn drawable(
    kind: Kind,
    origin: Option<StellarId>,
    exclude: Option<StellarId>,
    world: &World,
) -> Vec<StellarId> {
    if kind != Kind::Inhabited {
        return candidates(kind, origin, exclude, world);
    }
    let open = |stellar: &Stellar| {
        let id = Some(stellar.id);
        id != origin && id != exclude && normal_landable(stellar)
    };
    let found = world
        .stellars
        .iter()
        .any(|stellar| open(stellar) && inhabited(stellar) && world.distant(origin, stellar));
    if !found {
        return Vec::new();
    }
    world
        .stellars
        .iter()
        .filter(|stellar| open(stellar) && kind.takes(stellar, world))
        .map(|stellar| stellar.id)
        .collect()
}

/// The stellars a random code of `kind` draws from, as `world.draw` reads
/// [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw): by the
/// engine the original's draw (`drawable`), by the other reading the
/// [`candidates`].
fn pool(
    kind: Kind,
    origin: Option<StellarId>,
    exclude: Option<StellarId>,
    world: &World,
) -> Vec<StellarId> {
    match world.draw {
        RuleSource::Engine => drawable(kind, origin, exclude, world),
        RuleSource::Bible => candidates(kind, origin, exclude, world),
    }
}

/// The stellar `code` chooses from `origin`, never `exclude` when it
/// draws, drawing on `chance`: the origin for a code that names it, that
/// stellar for a fixed code, and for a random one a single
/// `chance.below(n)` over the stellars it draws from, by ascending ID, or
/// the origin, drawing nothing, when there is none. The original redraws
/// `Rand(2048)` until it hits one, which comes to the same odds. What it
/// draws from is as `world.draw` reads
/// [`mission_stellar_draw`](crate::RuleKey::MissionStellarDraw): by the
/// engine the original's draw (`drawable`), which for -2 takes a stellar at any distance once its
/// scan finds a distant one and stops where the original would redraw
/// forever; by the other reading the [`candidates`], each distant from
/// the origin. Every other random code draws the same by either. `None`
/// only when the origin is: as availability asks, with no origin.
pub fn select_stellar(
    code: i16,
    origin: Option<StellarId>,
    exclude: Option<StellarId>,
    world: &World,
    chance: &mut dyn Chance,
) -> Option<StellarId> {
    match StellarCode::of(code) {
        StellarCode::Origin => origin,
        StellarCode::Fixed(stellar) => Some(stellar),
        StellarCode::Random(kind) => {
            let candidates = pool(kind, origin, exclude, world);
            if candidates.is_empty() {
                return origin;
            }
            let n = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
            let drawn = usize::try_from(chance.below(n)).unwrap_or(usize::MAX);
            candidates.get(drawn).copied().or(origin)
        }
    }
}

/// Whether availability finds `code` a stellar with no origin
/// (`_IsMissionAvailable` @0x9b832-0x9b8b5, which refuses when
/// `_SelectMissionStellar` gives -1): a fixed stellar, or a random code
/// whose draw, with no origin, has a stellar to take, the same pool
/// [`select_stellar`] draws from. With no origin, distance is as
/// [`mission_stellar_distance`](crate::RuleKey::MissionStellarDistance)
/// reads it: by the engine, only whether every system at the stellar's
/// place lists it (`_StellarsAreDistant` @0x9e20-0x9ee4); by the other
/// reading, not tested. By the engine's draw a -2 needs that of its
/// gate's stellar only, station or not, and the stellar it takes need not
/// pass it. A code that names the origin finds none. Nothing is drawn.
#[must_use]
pub fn selectable(code: i16, world: &World) -> bool {
    match StellarCode::of(code) {
        StellarCode::Origin => false,
        StellarCode::Fixed(_) => true,
        StellarCode::Random(kind) => !pool(kind, None, None, world).is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::catalog::{GovtRecord, StarSystem};
    use crate::govt::DERELICT;
    use crate::hyperspace::CoLocated;
    use crate::story::{Countdown, GalaxyOverlay};
    use crate::testkit::{Draws, govt, star};

    const FED: GovtId = GovtId(128);
    const CIVVIES: GovtId = GovtId(129);
    const PIRATES: GovtId = GovtId(130);
    const WRECKS: GovtId = GovtId(131);

    /// The Federation (class 1, allied with 13, at war with 9); the
    /// Civvies (class 13); the pirates (class 9); derelict wrecks (class
    /// 1).
    fn govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [13, -1, -1, -1],
                enemies: [9, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [13, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [9, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                flags: DERELICT,
                classes: [1, -1, -1, -1],
                ..govt(131)
            },
        ])
    }

    /// 130 links to 131; 132 and 133 are far from both. 135 is at 130's
    /// place and 136 at 131's, neither linked; 134, at 132's place, lists
    /// 130; 137, far, lists 135.
    fn map() -> StarMap {
        StarMap::new(map_systems())
    }

    /// [`map`]'s systems.
    fn map_systems() -> Vec<StarSystem> {
        vec![
            star(130, (0.0, 0.0), &[131]),
            star(131, (100.0, 0.0), &[]),
            star(132, (200.0, 0.0), &[]),
            star(133, (300.0, 0.0), &[]),
            star(134, (200.0, 0.0), &[130]),
            star(135, (0.0, 0.0), &[]),
            star(136, (100.0, 0.0), &[]),
            star(137, (400.0, 0.0), &[135]),
        ]
    }

    /// Stellar `id` in `system`, whole, with these `Flags` beside landing,
    /// of `govt`.
    fn at(system: i16, id: i16, flags: u32, govt: Option<GovtId>) -> Stellar {
        Stellar {
            id: StellarId(id),
            system: SystemId(system),
            flags: StellarFlags::CAN_LAND | flags,
            flags2: 0,
            govt,
            destroyed: false,
            listed_at_place: true,
            index_govt: None,
        }
    }

    const UNINHABITED: u32 = StellarFlags::UNINHABITED;
    const STATION: u32 = StellarFlags::STATION;

    /// The candidates of `kind` from 128, in system 130, among `stellars`.
    fn of(kind: Kind, stellars: &[Stellar]) -> Vec<i16> {
        far(RuleSource::Engine, kind, Some(128), stellars)
    }

    /// The candidates of `kind` from `origin` among `stellars`, distance
    /// told by `reading`.
    fn far(reading: RuleSource, kind: Kind, origin: Option<i16>, stellars: &[Stellar]) -> Vec<i16> {
        far_on(&map(), reading, kind, origin, stellars)
    }

    /// [`far`] over `map`.
    fn far_on(
        map: &StarMap,
        reading: RuleSource,
        kind: Kind,
        origin: Option<i16>,
        stellars: &[Stellar],
    ) -> Vec<i16> {
        let govts = govts();
        let world = World {
            stellars,
            placed: &[],
            map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: reading,
            ally_index: RuleSource::Engine,
        };
        candidates(kind, origin.map(StellarId), None, &world)
            .into_iter()
            .map(|id| id.0)
            .collect()
    }

    /// The -2 candidates from 128 among `stellars` by the engine's reading
    /// of distance, then by the other.
    fn both(stellars: &[Stellar]) -> (Vec<i16>, Vec<i16>) {
        both_on(&map(), stellars)
    }

    /// [`both`] over `map`.
    fn both_on(map: &StarMap, stellars: &[Stellar]) -> (Vec<i16>, Vec<i16>) {
        let by = |reading| far_on(map, reading, Kind::Inhabited, Some(128), stellars);
        (by(RuleSource::Engine), by(RuleSource::Bible))
    }

    /// The origin, 128 in 130, then `more`.
    fn with_origin(more: &[Stellar]) -> Vec<Stellar> {
        let mut stellars = vec![at(130, 128, 0, Some(FED))];
        stellars.extend_from_slice(more);
        stellars
    }

    #[test]
    fn each_code_names_what_the_original_reads() {
        use Kind::{
            Ally, Enemy, Govt, Inhabited, NoSharedClass, NotGovt, SharedClass, Uninhabited,
        };
        use StellarCode::{Fixed, Origin, Random};
        for (code, chooses) in [
            (-1, Origin),
            (-4, Origin),
            (-5, Origin),
            (0, Origin),
            (1, Origin),
            (127, Origin),
            (-2, Random(Inhabited)),
            (-3, Random(Uninhabited)),
            (128, Fixed(StellarId(128))),
            (2175, Fixed(StellarId(2175))),
            (2176, Origin),
            (9998, Origin),
            (9999, Random(Govt(None))),
            (10000, Random(Govt(Some(GovtId(128))))),
            (14999, Random(Govt(Some(GovtId(5127))))),
            (15000, Random(Ally(GovtId(128)))),
            (19999, Random(Ally(GovtId(5127)))),
            (20000, Random(NotGovt(GovtId(128)))),
            (24999, Random(NotGovt(GovtId(5127)))),
            (25000, Random(Enemy(GovtId(128)))),
            (29999, Random(Enemy(GovtId(5127)))),
            (30000, Random(SharedClass(GovtId(128)))),
            (30999, Random(SharedClass(GovtId(1127)))),
            (31000, Random(NoSharedClass(GovtId(128)))),
            (31999, Random(NoSharedClass(GovtId(1127)))),
            (32000, Origin),
            (i16::MIN, Origin),
        ] {
            assert_eq!(StellarCode::of(code), chooses, "{code}");
        }
    }

    #[test]
    fn a_government_code_names_its_relation_and_government_at_each_range_edge() {
        use GovtCode::{Ally, Enemy, Govt, NoSharedClass, NotGovt, SharedClass};
        for (code, names) in [
            (i16::MIN, None),
            (-2, None),
            (128, None),
            (9998, None),
            (9999, Some(Govt(None))),
            (10000, Some(Govt(Some(GovtId(128))))),
            (14999, Some(Govt(Some(GovtId(5127))))),
            (15000, Some(Ally(GovtId(128)))),
            (19999, Some(Ally(GovtId(5127)))),
            (20000, Some(NotGovt(GovtId(128)))),
            (24999, Some(NotGovt(GovtId(5127)))),
            (25000, Some(Enemy(GovtId(128)))),
            (29999, Some(Enemy(GovtId(5127)))),
            (30000, Some(SharedClass(GovtId(128)))),
            (30999, Some(SharedClass(GovtId(1127)))),
            (31000, Some(NoSharedClass(GovtId(128)))),
            (31999, Some(NoSharedClass(GovtId(1127)))),
            (32000, None),
            (i16::MAX, None),
        ] {
            assert_eq!(GovtCode::of(code), names, "{code}");
        }
    }

    #[test]
    fn minus_2_takes_inhabited_planets_and_minus_3_uninhabited_ones() {
        let stellars = with_origin(&[
            at(132, 150, 0, None),
            at(132, 151, UNINHABITED, None),
            at(132, 152, STATION, None),
            at(132, 153, UNINHABITED | STATION, None),
        ]);
        assert_eq!(of(Kind::Inhabited, &stellars), [150]);
        assert_eq!(of(Kind::Uninhabited, &stellars), [151]);
    }

    #[test]
    fn a_government_code_takes_its_stellars_and_uninhabited_ones_only_when_derelict() {
        let stellars = with_origin(&[
            at(132, 150, 0, Some(FED)),
            at(132, 151, UNINHABITED, Some(FED)),
            at(132, 152, STATION, Some(FED)),
            at(132, 153, 0, Some(CIVVIES)),
            at(132, 154, 0, None),
            at(132, 155, 0, Some(WRECKS)),
            at(132, 156, UNINHABITED, Some(WRECKS)),
            at(132, 157, UNINHABITED, None),
        ]);
        assert_eq!(
            of(Kind::Govt(Some(FED)), &stellars),
            [150, 152],
            "stations too"
        );
        assert_eq!(of(Kind::Govt(None), &stellars), [154], "9999: independent");
        assert_eq!(of(Kind::Govt(Some(WRECKS)), &stellars), [155, 156]);
    }

    #[test]
    fn the_relation_codes_take_other_inhabited_governments_so_related() {
        let stellars = with_origin(&[
            at(132, 150, 0, Some(FED)),
            at(132, 151, 0, Some(CIVVIES)),
            at(132, 152, UNINHABITED, Some(CIVVIES)),
            at(132, 153, 0, Some(PIRATES)),
            at(132, 154, 0, None),
            at(132, 155, 0, Some(WRECKS)),
        ]);
        assert_eq!(
            of(Kind::Ally(FED), &stellars),
            [151],
            "not the Federation itself (@0x9a7a2-0x9a7ad), nor an independent"
        );
        assert_eq!(of(Kind::NotGovt(FED), &stellars), [151, 153, 154, 155]);
        assert_eq!(of(Kind::Enemy(FED), &stellars), [153]);
        assert_eq!(
            of(Kind::SharedClass(FED), &stellars),
            [155],
            "class 1 in slot 1, derelict or not"
        );
        assert_eq!(of(Kind::NoSharedClass(FED), &stellars), [151, 153]);
    }

    #[test]
    fn a_candidate_is_normal_landable_and_distant_from_the_origin() {
        let gate = Stellar {
            flags2: 0x1000,
            ..at(133, 162, 0, None)
        };
        let wormhole = Stellar {
            flags2: 0x2000,
            ..at(133, 163, 0, None)
        };
        let unlandable = Stellar {
            flags: 0,
            ..at(133, 164, 0, None)
        };
        let destroyed = Stellar {
            destroyed: true,
            ..at(133, 165, 0, None)
        };
        let wreck = Stellar {
            destroyed: true,
            ..at(133, 166, StellarFlags::ONLY_WHEN_DESTROYED, None)
        };
        let stellars = with_origin(&[
            at(130, 129, 0, None),
            at(131, 140, 0, None),
            at(132, 150, 0, None),
            at(133, 160, 0, None),
            gate,
            wormhole,
            unlandable,
            destroyed,
            wreck,
        ]);
        assert_eq!(
            of(Kind::Inhabited, &stellars),
            [150, 160, 166],
            "not in the origin's system, nor one linked with it, nor a gate, nor unlandable"
        );
    }

    #[test]
    fn distance_counts_a_link_either_way() {
        let map = StarMap::new(vec![
            star(130, (0.0, 0.0), &[]),
            star(131, (100.0, 0.0), &[130]),
            star(132, (200.0, 0.0), &[]),
        ]);
        let govts = govts();
        let stellars = with_origin(&[at(131, 140, 0, None), at(132, 150, 0, None)]);
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let world = World {
                stellars: &stellars,
                placed: &[],
                map: &map,
                govts: &govts,
                draw: RuleSource::Engine,
                distance: reading,
                ally_index: RuleSource::Engine,
            };
            assert_eq!(
                candidates(Kind::Inhabited, Some(StellarId(128)), None, &world),
                [StellarId(150)],
                "131 lists 130, by {reading:?}"
            );
        }
    }

    #[test]
    fn by_the_engine_a_stellar_at_the_origins_place_is_not_distant() {
        let stellars = with_origin(&[at(135, 160, 0, None), at(133, 170, 0, None)]);
        assert_eq!(
            both(&stellars),
            (vec![170], vec![160, 170]),
            "135 shares 130's chain head (@0x9d3e-0x9d9d)"
        );
    }

    #[test]
    fn by_the_engine_a_link_from_the_origin_to_the_stellars_place_is_not_distant() {
        let stellars = with_origin(&[at(136, 160, 0, None), at(133, 170, 0, None)]);
        assert_eq!(
            both(&stellars),
            (vec![170], vec![160, 170]),
            "130 lists 131, at 136's place (@0x9db4-0x9dde)"
        );
    }

    #[test]
    fn by_the_engine_a_link_from_the_stellars_system_to_the_origins_place_is_not_distant() {
        let stellars = with_origin(&[at(137, 160, 0, None), at(133, 170, 0, None)]);
        let itself = StarMap::with_hidden(map_systems(), &BTreeSet::new(), CoLocated::Itself);
        assert_eq!(
            both_on(&itself, &stellars),
            (vec![170], vec![160, 170]),
            "137 lists 135, at 130's place (@0x9de4-0x9e0e)"
        );
        assert_eq!(
            both(&stellars),
            (vec![170], vec![170]),
            "by the original's resolution 137's link names 130, which 135 \
             shares a place with: a neighbour by the other distance reading too"
        );
    }

    #[test]
    fn only_the_two_systems_own_links_count_for_distance() {
        let stellars = with_origin(&[at(132, 150, 0, None)]);
        assert_eq!(
            both(&stellars),
            (vec![150], vec![150]),
            "134, at 132's place, lists 130, but 132 does not"
        );
    }

    #[test]
    fn by_the_engine_a_stellar_not_every_system_at_its_place_lists_is_not_distant() {
        let half_listed = Stellar {
            listed_at_place: false,
            ..at(132, 150, 0, None)
        };
        let stellars = with_origin(&[half_listed, at(133, 170, 0, None)]);
        assert_eq!(
            both(&stellars),
            (vec![170], vec![150, 170]),
            "the chain walk counts 132 and 134 but finds 150 in one (@0x9e60-0x9ee4)"
        );
    }

    #[test]
    fn with_no_origin_the_engine_tests_only_that_every_system_at_its_place_lists_it() {
        let half_listed = Stellar {
            listed_at_place: false,
            ..at(132, 150, 0, None)
        };
        let stellars = with_origin(&[half_listed, at(135, 160, 0, None)]);
        assert_eq!(
            far(RuleSource::Engine, Kind::Inhabited, None, &stellars),
            [128, 160],
            "no Con slot nor head is read with no origin (@0x9e20-0x9e5c)"
        );
        assert_eq!(
            far(RuleSource::Bible, Kind::Inhabited, None, &stellars),
            [128, 150, 160]
        );
    }

    #[test]
    fn by_the_engine_an_origin_of_no_visible_system_has_no_distant_stellar() {
        let stellars = [
            at(132, 150, 0, Some(FED)),
            at(132, 151, UNINHABITED, Some(FED)),
        ];
        for kind in [Kind::Inhabited, Kind::Uninhabited, Kind::Govt(Some(FED))] {
            assert_eq!(
                far(RuleSource::Engine, kind, Some(128), &stellars),
                Vec::<i16>::new(),
                "{kind:?}: its system is -1 (@0x9d1d-0x9d24)"
            );
        }
    }

    #[test]
    fn by_the_bible_with_no_origin_nothing_is_excluded_for_it_nor_tested_for_distance() {
        let (map, govts) = (map(), govts());
        let stellars = with_origin(&[at(131, 140, 0, None), at(132, 150, 0, None)]);
        let world = World {
            stellars: &stellars,
            placed: &[],
            map: &map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: RuleSource::Bible,
            ally_index: RuleSource::Engine,
        };
        assert_eq!(
            candidates(Kind::Inhabited, None, None, &world),
            [StellarId(128), StellarId(140), StellarId(150)]
        );
        assert_eq!(
            candidates(Kind::Inhabited, None, Some(StellarId(140)), &world),
            [StellarId(128), StellarId(150)],
            "the exclude still is"
        );
    }

    #[test]
    fn by_the_bible_an_origin_no_visible_system_lists_is_not_tested_for_distance() {
        let (map, govts) = (map(), govts());
        let stellars = [at(130, 129, 0, None), at(132, 150, 0, None)];
        let world = World {
            stellars: &stellars,
            placed: &[],
            map: &map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: RuleSource::Bible,
            ally_index: RuleSource::Engine,
        };
        assert_eq!(
            candidates(Kind::Inhabited, Some(StellarId(128)), None, &world),
            [StellarId(129), StellarId(150)]
        );
    }

    /// The world of `stellars` over [`map`] and [`govts`], drawn by the
    /// engine's reading, handed to `f`.
    fn in_world<T>(stellars: &[Stellar], f: impl FnOnce(&World) -> T) -> T {
        by(RuleSource::Engine, stellars, f)
    }

    /// The world of `stellars` over [`map`] and [`govts`], drawn by
    /// `reading`, distance told by the engine, handed to `f`.
    fn by<T>(reading: RuleSource, stellars: &[Stellar], f: impl FnOnce(&World) -> T) -> T {
        by_both(reading, RuleSource::Engine, stellars, f)
    }

    /// The world of `stellars` over [`map`] and [`govts`], drawn by
    /// `draw` and distance told by `distance`, handed to `f`.
    fn by_both<T>(
        draw: RuleSource,
        distance: RuleSource,
        stellars: &[Stellar],
        f: impl FnOnce(&World) -> T,
    ) -> T {
        let (map, govts) = (map(), govts());
        f(&World {
            stellars,
            placed: &[],
            map: &map,
            govts: &govts,
            draw,
            distance,
            ally_index: RuleSource::Engine,
        })
    }

    /// What `code` chooses from 128, never `exclude`, among `stellars` by
    /// `reading`, distance told by the engine, drawing `draws`, with the
    /// bounds asked.
    fn drawn(
        reading: RuleSource,
        code: i16,
        exclude: Option<i16>,
        stellars: &[Stellar],
        draws: &[u32],
    ) -> (Option<i16>, Vec<u32>) {
        drawn_by(reading, RuleSource::Engine, code, exclude, stellars, draws)
    }

    /// What `code` chooses from 128, never `exclude`, among `stellars`,
    /// drawn by `draw` and distance told by `distance`, drawing `draws`,
    /// with the bounds asked.
    fn drawn_by(
        draw: RuleSource,
        distance: RuleSource,
        code: i16,
        exclude: Option<i16>,
        stellars: &[Stellar],
        draws: &[u32],
    ) -> (Option<i16>, Vec<u32>) {
        by_both(draw, distance, stellars, |world| {
            let mut chance = Draws::of(draws);
            let exclude = exclude.map(StellarId);
            let chosen = select_stellar(code, Some(StellarId(128)), exclude, world, &mut chance);
            (chosen.map(|id| id.0), chance.asked)
        })
    }

    #[test]
    fn by_the_engine_minus_2_draws_at_any_distance_once_its_scan_finds_a_distant_stellar() {
        let stellars = with_origin(&[at(131, 140, 0, None), at(132, 150, 0, None)]);
        assert_eq!(
            drawn(RuleSource::Engine, -2, None, &stellars, &[0]),
            (Some(140), vec![2]),
            "the scan finds 150 first, and the draw tests 150's distance, not 140's"
        );
        assert_eq!(
            drawn(RuleSource::Bible, -2, None, &stellars, &[0]),
            (Some(150), vec![1]),
            "only a distant candidate"
        );
    }

    #[test]
    fn by_the_engine_minus_2_may_draw_in_the_origins_own_system_but_never_the_origin_or_exclude() {
        let stellars = with_origin(&[
            at(130, 129, 0, None),
            at(131, 140, 0, None),
            at(132, 150, 0, None),
        ]);
        let engine = |exclude, draw| drawn(RuleSource::Engine, -2, exclude, &stellars, &[draw]);
        assert_eq!(engine(None, 0), (Some(129), vec![3]));
        assert_eq!(engine(Some(129), 0), (Some(140), vec![2]));
        assert_eq!(engine(Some(140), 1), (Some(150), vec![2]));
    }

    #[test]
    fn by_the_engine_minus_2_draws_inhabited_normal_landable_planets() {
        let unlandable = Stellar {
            flags: 0,
            ..at(131, 141, 0, None)
        };
        let gate = Stellar {
            flags2: 0x1000,
            ..at(131, 142, 0, None)
        };
        let stellars = with_origin(&[
            at(131, 140, 0, None),
            unlandable,
            gate,
            at(131, 143, UNINHABITED, None),
            at(131, 144, STATION, None),
            at(132, 150, 0, None),
        ]);
        assert_eq!(
            drawn(RuleSource::Engine, -2, None, &stellars, &[1]),
            (Some(150), vec![2]),
            "140 and 150 only"
        );
    }

    #[test]
    fn by_the_engine_a_distant_station_opens_minus_2s_scan() {
        let stellars = with_origin(&[at(131, 140, 0, None), at(132, 150, STATION, None)]);
        assert_eq!(
            drawn(RuleSource::Engine, -2, None, &stellars, &[0]),
            (Some(140), vec![1]),
            "the scan does not test for a station (@0x9a2c7-0x9a330)"
        );
        assert_eq!(
            drawn(RuleSource::Bible, -2, None, &stellars, &[0]),
            (Some(128), vec![]),
        );
    }

    #[test]
    fn by_the_engine_minus_2_gives_the_origin_when_its_scan_finds_nothing_distant() {
        let unlandable = Stellar {
            flags: 0,
            ..at(132, 151, 0, None)
        };
        let gate = Stellar {
            flags2: 0x1000,
            ..at(132, 152, 0, None)
        };
        let stellars = with_origin(&[
            at(130, 129, 0, None),
            at(131, 140, 0, None),
            at(132, 150, UNINHABITED, None),
            unlandable,
            gate,
            at(133, 160, 0, None),
        ]);
        assert_eq!(
            drawn(RuleSource::Engine, -2, Some(160), &stellars, &[0]),
            (Some(128), vec![]),
            "nothing distant but the exclude: the crash-log path returns the origin (@0x9b02a-0x9b072)"
        );
    }

    #[test]
    fn by_the_engine_an_origin_of_no_visible_system_has_no_distant_stellar_for_minus_2() {
        let stellars = [at(130, 129, 0, None), at(132, 150, 0, None)];
        assert_eq!(
            drawn(RuleSource::Engine, -2, None, &stellars, &[0]),
            (Some(128), vec![]),
            "its system is -1 (@0x9d1d-0x9d24)"
        );
        assert_eq!(
            drawn(RuleSource::Bible, -2, None, &stellars, &[0]),
            (Some(128), vec![]),
            "nothing is distant from it by the engine's distance, whichever draw"
        );
        assert_eq!(
            drawn_by(
                RuleSource::Bible,
                RuleSource::Bible,
                -2,
                None,
                &stellars,
                &[0]
            ),
            (Some(129), vec![2]),
            "phase 1's draw and distance: not tested"
        );
    }

    #[test]
    fn where_the_original_draws_forever_both_readings_give_the_origin_drawing_nothing() {
        // The scan passes the distant station 150, but nothing passes the
        // draw: 150 is a station and 140, the only other planet, is
        // excluded. A loop over `Draws`, which gives its last outcome
        // forever, would never end.
        let stellars = with_origin(&[at(131, 140, 0, None), at(132, 150, STATION, None)]);
        for reading in RuleSource::ALL {
            assert_eq!(
                drawn(reading, -2, Some(140), &stellars, &[]),
                (Some(128), vec![]),
                "{reading:?}"
            );
        }
        let only_a_station = with_origin(&[at(132, 150, STATION, None)]);
        for reading in RuleSource::ALL {
            assert_eq!(
                drawn(reading, -2, None, &only_a_station, &[]),
                (Some(128), vec![]),
                "{reading:?}"
            );
        }
    }

    /// A station alone in 133, and a planet in 132 that 134, at 132's
    /// place, does not list.
    fn station_and_half_listed_planet() -> Vec<Stellar> {
        let half_listed = Stellar {
            listed_at_place: false,
            ..at(132, 150, 0, None)
        };
        vec![half_listed, at(133, 160, STATION, None)]
    }

    #[test]
    fn by_the_engine_with_no_origin_minus_2s_gate_is_chain_tested_and_its_draw_is_not() {
        let stellars = station_and_half_listed_planet();
        let with_no_origin = |draw| {
            by(draw, &stellars, |world| {
                let mut chance = Draws::of(&[0]);
                let chosen = select_stellar(-2, None, None, world, &mut chance);
                (chosen.map(|id| id.0), chance.asked)
            })
        };
        assert_eq!(
            with_no_origin(RuleSource::Engine),
            (Some(150), vec![1]),
            "the station passes the gate (@0x9a2c7-0x9a330); the drawn 150 is not chain-tested (@0x9a3ce-0x9a3db)"
        );
        assert_eq!(
            with_no_origin(RuleSource::Bible),
            (None, vec![]),
            "only a candidate, and 150 fails the chain walk"
        );
        let half_listed_only = [station_and_half_listed_planet()[0]];
        let gate_shut = by(RuleSource::Engine, &half_listed_only, |world| {
            select_stellar(-2, None, None, world, &mut Draws::default())
        });
        assert_eq!(gate_shut, None, "150 alone fails the gate's chain walk");
    }

    #[test]
    fn availability_finds_what_the_draw_would_take_by_each_reading() {
        let stellars = station_and_half_listed_planet();
        assert!(
            by(RuleSource::Engine, &stellars, |world| selectable(-2, world)),
            "the station opens the gate and the draw takes 150"
        );
        assert!(
            !by(RuleSource::Bible, &stellars, |world| selectable(-2, world)),
            "no candidate: 150 fails the chain walk, 160 is a station"
        );
    }

    #[test]
    fn the_other_random_codes_draw_the_same_by_either_reading() {
        let stellars = with_origin(&[
            at(130, 129, UNINHABITED, Some(FED)),
            at(131, 140, UNINHABITED, Some(FED)),
            at(132, 150, UNINHABITED, Some(FED)),
            at(132, 151, 0, Some(FED)),
            at(133, 160, UNINHABITED, Some(FED)),
            at(133, 161, 0, Some(CIVVIES)),
        ]);
        for code in [-3, 10000, 15000, 20000, 25000, 30000, 31000] {
            let draws = |reading| drawn(reading, code, Some(160), &stellars, &[0]);
            assert_eq!(
                draws(RuleSource::Engine),
                draws(RuleSource::Bible),
                "{code}"
            );
        }
        assert_eq!(
            drawn(RuleSource::Engine, -3, None, &stellars, &[0]),
            (Some(150), vec![2]),
            "distant from the origin, as the drawn stellar's distance is tested (@0x9a52c-0x9a536)"
        );
    }

    #[test]
    fn a_random_code_draws_once_over_its_candidates_and_never_the_exclude() {
        let stellars = with_origin(&[
            at(132, 150, 0, None),
            at(132, 151, 0, None),
            at(133, 160, 0, None),
        ]);
        in_world(&stellars, |world| {
            let mut chance = Draws::of(&[1]);
            let origin = Some(StellarId(128));
            assert_eq!(
                select_stellar(-2, origin, Some(StellarId(150)), world, &mut chance),
                Some(StellarId(160)),
                "the second of 151 and 160"
            );
            assert_eq!(chance.asked, [2]);
        });
    }

    #[test]
    fn with_no_candidate_a_random_code_gives_the_origin_drawing_nothing() {
        let stellars = with_origin(&[at(130, 129, 0, None)]);
        in_world(&stellars, |world| {
            let mut chance = Draws::of(&[0]);
            let origin = Some(StellarId(128));
            assert_eq!(select_stellar(-2, origin, None, world, &mut chance), origin);
            assert_eq!(
                select_stellar(-2, None, None, world, &mut chance),
                Some(StellarId(128)),
                "with no origin, the origin itself is a candidate"
            );
            assert_eq!(select_stellar(-3, None, None, world, &mut chance), None);
            assert_eq!(chance.asked, [2], "drawn only with a candidate");
        });
    }

    #[test]
    fn fixed_and_origin_codes_draw_nothing() {
        in_world(&with_origin(&[]), |world| {
            let mut chance = Draws::default();
            let origin = Some(StellarId(128));
            assert_eq!(
                select_stellar(500, origin, None, world, &mut chance),
                Some(StellarId(500)),
                "whether a system lists it or not"
            );
            for code in [-1, -4, 0, 1, 127, 2176, 9998, 32000] {
                assert_eq!(
                    select_stellar(code, origin, None, world, &mut chance),
                    origin
                );
                assert_eq!(select_stellar(code, None, None, world, &mut chance), None);
            }
            assert!(chance.asked.is_empty(), "{:?}", chance.asked);
        });
    }

    #[test]
    fn availability_finds_a_fixed_stellar_or_a_random_code_with_any_candidate() {
        let stellars = with_origin(&[at(131, 140, UNINHABITED, None)]);
        in_world(&stellars, |world| {
            assert!(selectable(128, world));
            assert!(selectable(2175, world));
            assert!(selectable(-2, world), "the docked stellar itself");
            assert!(selectable(-3, world), "next door");
            assert!(!selectable(10001, world), "no Civvies stellar");
            for code in [-1, -4, 1, 127, 2176, 9998] {
                assert!(!selectable(code, world), "{code}");
            }
        });
    }

    #[test]
    fn a_stellar_is_listed_at_its_place_when_every_system_there_lists_it() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(130), Some(false));
        let systems: Vec<StarSystem> = vec![
            star(130, (0.0, 0.0), &[]),
            star(131, (0.0, 0.0), &[]),
            star(132, (200.0, 0.0), &[]),
        ];
        let galaxy = OverlaidGalaxy::new(systems, overlay);
        let listed = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0,
            flags2: 0,
            govt: None,
            min_status: 0,
        };
        let stellars = stellars_of(
            &galaxy,
            &[
                listed(140, &[130, 131]),
                listed(141, &[131]),
                listed(150, &[132]),
            ],
            SystemId(132),
        );
        let at_place: Vec<(i16, i16, bool)> = stellars
            .iter()
            .map(|stellar| (stellar.id.0, stellar.system.0, stellar.listed_at_place))
            .collect();
        assert_eq!(
            at_place,
            [(140, 131, true), (141, 131, false), (150, 132, true)],
            "the hidden 130 shares 131's place and lists 140 only: the chain takes in every system there (@0x78442-0x784e3)"
        );
    }

    #[test]
    fn each_stellar_is_in_its_owner_and_an_unowned_one_is_left_out() {
        let systems: Vec<StarSystem> = vec![
            star(130, (0.0, 0.0), &[]),
            star(131, (0.0, 0.0), &[]),
            star(132, (200.0, 0.0), &[]),
        ];
        let listed = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0,
            flags2: 0,
            govt: None,
            min_status: 0,
        };
        let listed = [
            listed(140, &[130, 131]),
            listed(141, &[131]),
            listed(150, &[131, 132]),
        ];
        let placed = |colocated| {
            let mut galaxy = OverlaidGalaxy::new(systems.clone(), GalaxyOverlay::default());
            galaxy.set_colocated(colocated);
            stellars_of(&galaxy, &listed, SystemId(132))
                .iter()
                .map(|stellar| (stellar.id.0, stellar.system.0))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            placed(CoLocated::Lowest),
            [(140, 130), (150, 132)],
            "131 is shadowed by 130 and claims nothing: 141 has no owner"
        );
        assert_eq!(
            placed(CoLocated::Itself),
            [(140, 130), (141, 131), (150, 131)]
        );
    }

    /// Two visible systems, 130 and 131, share a place, and the player is
    /// in 132, which lists neither 140 nor 141, so `holder`'s rescue
    /// (`_HandleStellarSystemVisibility` @0x32c58-0x32cb3) gives neither to
    /// the player's system. By [`CoLocated::Lowest`] 130 claims 140 and
    /// marks 131 done (`_PropagateMissionBitEffects` @0x9977e-0x997ae), so
    /// 141, which only 131 lists, keeps no owner, is never set active, and
    /// a -2 from 150 can't draw it. By [`CoLocated::Itself`] 131 owns 141
    /// and the draw can take it.
    ///
    /// The rest of the rule is pinned elsewhere: the rescue in a shadowed
    /// player's system by
    /// `a_stellar_rescued_into_the_players_system_is_a_candidate_there`,
    /// the chain taking in a hidden member by
    /// `a_stellar_is_listed_at_its_place_when_every_system_there_lists_it`,
    /// the readers that fall back to `_FindSystemFromStellar` by
    /// `whereabouts_hold_the_rescued_holder_and_what_find_system_from_stellar_gives`
    /// and `the_travel_system_is_the_holders_stand_in_else_find_system_from_stellar`,
    /// and the session's wiring of the rule by
    /// `by_the_engine_a_place_of_two_visible_systems_is_the_lowest_one`.
    #[test]
    fn a_random_draw_takes_a_stellar_only_a_shadowed_system_lists_only_by_the_other_reading() {
        let systems: Vec<StarSystem> = vec![
            star(130, (0.0, 0.0), &[]),
            star(131, (0.0, 0.0), &[]),
            star(132, (200.0, 0.0), &[]),
        ];
        let landable = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: StellarFlags::CAN_LAND,
            flags2: 0,
            govt: None,
            min_status: 0,
        };
        let listed = [
            landable(140, &[130, 131]),
            landable(141, &[131]),
            landable(150, &[132]),
        ];
        let govts = govts();
        let pool_of = |colocated| {
            let mut galaxy = OverlaidGalaxy::new(systems.clone(), GalaxyOverlay::default());
            galaxy.set_colocated(colocated);
            let stellars = stellars_of(&galaxy, &listed, SystemId(132));
            let world = World {
                stellars: &stellars,
                placed: &[],
                map: galaxy.map(),
                govts: &govts,
                draw: RuleSource::Engine,
                distance: RuleSource::Engine,
                ally_index: RuleSource::Engine,
            };
            pool(Kind::Inhabited, Some(StellarId(150)), None, &world)
                .iter()
                .map(|id| id.0)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            pool_of(CoLocated::Lowest),
            [140],
            "131 is shadowed by 130 and claims nothing, and the player's 132 doesn't list 141"
        );
        assert_eq!(
            pool_of(CoLocated::Itself),
            [140, 141],
            "131 stands for itself and owns 141"
        );
    }

    #[test]
    fn the_stellars_of_the_visible_systems_go_to_the_lowest_id_listing_each() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(130), Some(false));
        overlay.set_stellar_govt(StellarId(150), Some(Some(CIVVIES)));
        overlay.set_stellar_govt(StellarId(160), Some(None));
        overlay.set_destroyed(StellarId(140), Some(Countdown { days_left: 3 }));
        let systems: Vec<StarSystem> = vec![
            star(130, (0.0, 0.0), &[]),
            star(131, (100.0, 0.0), &[]),
            star(132, (200.0, 0.0), &[]),
        ];
        let galaxy = OverlaidGalaxy::new(systems, overlay);
        let listed = |id, systems: &[i16]| ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0x21,
            flags2: 0x0010,
            govt: Some(FED),
            min_status: 0,
        };
        let stellars = stellars_of(
            &galaxy,
            &[
                listed(128, &[130]),
                listed(140, &[130, 131, 132]),
                listed(150, &[131]),
                listed(160, &[132, 134]),
                listed(170, &[134]),
            ],
            SystemId(132),
        );
        let stellar = |id, system, govt, destroyed| Stellar {
            id: StellarId(id),
            system: SystemId(system),
            flags: 0x21,
            flags2: 0x0010,
            govt,
            destroyed,
            listed_at_place: true,
            // Past 132, the last system: never written, index 0.
            index_govt: Some(FED),
        };
        assert_eq!(
            stellars,
            [
                stellar(140, 131, Some(FED), true),
                stellar(150, 131, Some(CIVVIES), false),
                stellar(160, 132, None, false),
            ],
            "128 only in the hidden 130, and 170 only in 134, not in the catalog"
        );
        let (map, govts) = (galaxy.map().clone(), Governments::empty());
        let world = World {
            stellars: &stellars,
            placed: &[],
            map: &map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: RuleSource::Engine,
            ally_index: RuleSource::Engine,
        };
        assert_eq!(world.system_of(StellarId(160)), Some(SystemId(132)));
        assert_eq!(world.system_of(StellarId(128)), None, "in a hidden system");
    }

    /// Stellar `id`, of no government, listed by `systems`.
    fn listed(id: i16, systems: &[i16]) -> ListedStellar {
        ListedStellar {
            id: StellarId(id),
            systems: systems.iter().copied().map(SystemId).collect(),
            flags: 0,
            flags2: 0,
            govt: None,
            min_status: 0,
        }
    }

    /// Systems 130 and 131 at one place, 131 hidden when `hidden`, and
    /// 132 elsewhere.
    fn sharing_a_place(hidden: bool) -> OverlaidGalaxy {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(131), hidden.then_some(false));
        OverlaidGalaxy::new(
            vec![
                star(130, (0.0, 0.0), &[]),
                star(131, (0.0, 0.0), &[]),
                star(132, (200.0, 0.0), &[]),
            ],
            overlay,
        )
    }

    #[test]
    fn a_stellar_rescued_into_the_players_system_is_a_candidate_there() {
        let galaxy = sharing_a_place(false);
        let listed = [listed(141, &[131])];
        let systems = |player| {
            stellars_of(&galaxy, &listed, SystemId(player))
                .iter()
                .map(|stellar| (stellar.id.0, stellar.system.0))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            systems(131),
            [(141, 131)],
            "130 shadows 131, so 141 has no owner, and the player's 131 holds it"
        );
        assert_eq!(systems(132), [], "the player elsewhere");
    }

    #[test]
    fn a_stellar_rescued_into_a_hidden_players_system_is_no_candidate() {
        let galaxy = sharing_a_place(true);
        let listed = [listed(141, &[131])];
        assert_eq!(
            stellars_of(&galaxy, &listed, SystemId(131)),
            [],
            "the hidden 131 holds 141, but a stellar is active only while its holder is visible (@0x32cbe-0x32cdf)"
        );
    }

    #[test]
    fn placements_cover_every_listed_stellar_held_or_not() {
        let galaxy = sharing_a_place(false);
        let listed = [
            listed(140, &[130, 131]),
            listed(141, &[131]),
            listed(142, &[131]),
            listed(160, &[134]),
        ];
        let placed = placements(&galaxy, &listed, SystemId(132));
        let whereabouts = |holder: Option<i16>, found: Option<i16>| Whereabouts {
            holder: holder.map(SystemId),
            found: found.map(SystemId),
        };
        let place = |id, at| Placed {
            id: StellarId(id),
            at,
        };
        assert_eq!(
            placed,
            [
                place(140, whereabouts(Some(130), Some(130))),
                place(141, whereabouts(None, Some(131))),
                place(142, whereabouts(None, Some(131))),
                place(160, whereabouts(None, Some(134))),
            ]
        );
        let held = placements(&galaxy, &listed, SystemId(131));
        assert_eq!(
            held[1],
            place(141, whereabouts(Some(131), Some(131))),
            "rescued"
        );

        let (map, govts) = (galaxy.map().clone(), Governments::empty());
        let world = World {
            stellars: &[],
            placed: &placed,
            map: &map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: RuleSource::Engine,
            ally_index: RuleSource::Engine,
        };
        for (at, id) in placed.iter().map(|placed| (placed.at, placed.id)) {
            assert_eq!(world.whereabouts(id), at, "{id:?}");
        }
        assert_eq!(
            world.whereabouts(StellarId(150)),
            Whereabouts::default(),
            "listed by no system"
        );
    }

    /// Stellar `id` in `system`, inhabited, of `govt`, the system at its
    /// index governed by `index_govt`.
    fn indexed(system: i16, id: i16, govt: Option<GovtId>, index_govt: Option<GovtId>) -> Stellar {
        Stellar {
            index_govt,
            ..at(system, id, 0, govt)
        }
    }

    /// The world of `stellars` over [`map`] and [`govts`], an ally code's
    /// index test read by `ally_index`, the draw and distance by the
    /// engine, handed to `f`.
    fn by_ally<T>(ally_index: RuleSource, stellars: &[Stellar], f: impl FnOnce(&World) -> T) -> T {
        let (map, govts) = (map(), govts());
        f(&World {
            stellars,
            placed: &[],
            map: &map,
            govts: &govts,
            draw: RuleSource::Engine,
            distance: RuleSource::Engine,
            ally_index,
        })
    }

    /// The candidates of `kind` from 128 among `stellars`, an ally code's
    /// index test read by `ally_index`.
    fn ally_candidates(ally_index: RuleSource, kind: Kind, stellars: &[Stellar]) -> Vec<i16> {
        by_ally(ally_index, stellars, |world| {
            candidates(kind, Some(StellarId(128)), None, world)
                .into_iter()
                .map(|id| id.0)
                .collect()
        })
    }

    #[test]
    fn an_ally_code_takes_a_stellar_by_the_system_at_its_index_by_the_engine_only() {
        let stellars = with_origin(&[
            indexed(132, 140, Some(PIRATES), Some(FED)),
            indexed(132, 141, Some(CIVVIES), None),
        ]);
        assert_eq!(
            ally_candidates(RuleSource::Engine, Kind::Ally(FED), &stellars),
            [140, 141],
            "the pirates' 140 by the Federation's system at its index (@0x9a7af-0x9a7c8)"
        );
        assert_eq!(
            ally_candidates(RuleSource::Bible, Kind::Ally(FED), &stellars),
            [141],
            "only the Civvies, allied by class"
        );
    }

    #[test]
    fn the_index_test_never_takes_an_independent_or_the_target_itself() {
        let stellars = with_origin(&[
            indexed(132, 140, None, Some(FED)),
            indexed(132, 141, Some(FED), Some(FED)),
        ]);
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                ally_candidates(reading, Kind::Ally(FED), &stellars),
                [] as [i16; 0],
                "refused before the index test (@0x9a794-0x9a7ad): {reading:?}"
            );
        }
    }

    #[test]
    fn the_index_test_needs_the_target_government() {
        let stellars = with_origin(&[indexed(132, 140, Some(PIRATES), Some(CIVVIES))]);
        assert_eq!(
            ally_candidates(RuleSource::Engine, Kind::Ally(FED), &stellars),
            [] as [i16; 0]
        );
    }

    #[test]
    fn the_index_test_still_needs_an_inhabited_stellar() {
        let stellars = with_origin(&[Stellar {
            index_govt: Some(FED),
            ..at(132, 140, UNINHABITED, Some(PIRATES))
        }]);
        assert_eq!(
            ally_candidates(RuleSource::Engine, Kind::Ally(FED), &stellars),
            [] as [i16; 0],
            "@0x9a825"
        );
    }

    #[test]
    fn only_the_ally_kind_reads_the_index_government() {
        let stellars = with_origin(&[
            indexed(132, 140, Some(PIRATES), Some(FED)),
            indexed(132, 141, Some(CIVVIES), Some(FED)),
            indexed(133, 150, Some(PIRATES), Some(CIVVIES)),
            indexed(133, 151, Some(CIVVIES), Some(CIVVIES)),
        ]);
        for kind in [
            Kind::Govt(Some(FED)),
            Kind::Govt(Some(PIRATES)),
            Kind::NotGovt(FED),
            Kind::Enemy(FED),
            Kind::Enemy(CIVVIES),
            Kind::SharedClass(FED),
            Kind::NoSharedClass(FED),
            Kind::NoSharedClass(CIVVIES),
            Kind::Inhabited,
            Kind::Uninhabited,
        ] {
            assert_eq!(
                ally_candidates(RuleSource::Engine, kind, &stellars),
                ally_candidates(RuleSource::Bible, kind, &stellars),
                "{kind:?}"
            );
        }
        assert_eq!(
            ally_candidates(RuleSource::Engine, Kind::Ally(CIVVIES), &stellars),
            [150],
            "the pirates' 150 by the Civvies' system at its index, not 140 by the Federation's"
        );
    }

    #[test]
    fn the_draw_and_availability_take_the_index_stellar() {
        let stellars = with_origin(&[
            indexed(132, 140, Some(PIRATES), Some(FED)),
            indexed(133, 150, Some(PIRATES), None),
        ]);
        let drawn = |reading| {
            by_ally(reading, &stellars, |world| {
                let mut chance = Draws::of(&[0]);
                let chosen = select_stellar(15000, Some(StellarId(128)), None, world, &mut chance);
                (chosen.map(|id| id.0), chance.asked)
            })
        };
        assert_eq!(
            drawn(RuleSource::Engine),
            (Some(140), vec![1]),
            "the draw's test (@0x9a8e5-0x9a8fa)"
        );
        assert_eq!(
            drawn(RuleSource::Bible),
            (Some(128), vec![]),
            "no candidate: the origin, drawing nothing"
        );
        assert!(by_ally(RuleSource::Engine, &stellars, |world| selectable(
            15000, world
        )));
        assert!(!by_ally(RuleSource::Bible, &stellars, |world| selectable(
            15000, world
        )));
    }

    /// The catalog of systems 130 (the Federation's), 131 (independent),
    /// 133 (a `Govt` of 400) and 150 (the pirates'), as `index_govt` reads
    /// it.
    fn catalog_govt(system: SystemId) -> Option<GovtId> {
        match system.0 {
            130 => Some(FED),
            133 => Some(GovtId(400)),
            150 => Some(PIRATES),
            _ => None,
        }
    }

    #[test]
    fn the_government_at_a_stellars_index_is_the_same_id_systems() {
        let at = |id| index_govt(StellarId(id), catalog_govt, SystemId(150));
        assert_eq!(at(130), Some(FED));
        assert_eq!(at(150), Some(PIRATES), "the last system loaded");
        assert_eq!(at(131), None, "independent: -1");
        assert_eq!(
            at(133),
            None,
            "outside 128-383 the loader writes -1 (@0x77d6b-0x77d81)"
        );
        assert_eq!(
            at(140),
            None,
            "a gap keeps the sentinel 0x8001 (@0x77a92), which matches no government"
        );
        assert_eq!(
            at(151),
            Some(GovtId(128)),
            "past the last system loaded the slot is never written (@0x78393-0x783a1) and reads _NewPtr's zeros, index 0"
        );
    }

    #[test]
    fn the_government_index_is_kept_from_128_to_383() {
        let catalog = |system: SystemId| Some(GovtId(system.0));
        let at = |id| index_govt(StellarId(id), catalog, SystemId(2175));
        assert_eq!(at(128), Some(GovtId(128)));
        assert_eq!(at(383), Some(GovtId(383)));
        assert_eq!(at(384), None);
        assert_eq!(
            index_govt(StellarId(140), |_| Some(GovtId(127)), SystemId(2175)),
            None
        );
    }

    #[test]
    fn the_loader_writes_to_its_highest_system_when_it_can_reach_every_one() {
        let ids = |ids: &[i16]| written_to(ids.iter().copied().map(SystemId));
        assert_eq!(ids(&[128, 130, 150]), SystemId(150));
        assert_eq!(ids(&[128, 2175]), SystemId(2175));
        assert_eq!(
            ids(&[128, 127]),
            SystemId(2175),
            "a system below 128 is counted (@0x77a14) but never loaded, so the loop runs to the array's end (@0x783b3)"
        );
        assert_eq!(ids(&[128, 2176]), SystemId(2175), "nor one above 2175");
        assert_eq!(
            ids(&[]),
            SystemId(128),
            "no system: the first slot is written before the count stops the loop"
        );
    }

    #[test]
    fn each_stellar_carries_the_catalog_government_at_its_index() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(140), Some(false));
        overlay.set_govt(SystemId(141), Some(Some(PIRATES)));
        let governed = |id: i16, govt| StarSystem {
            govt,
            ..star(id, (f32::from(id) * 100.0, 0.0), &[])
        };
        let galaxy = OverlaidGalaxy::new(
            vec![
                governed(130, None),
                governed(140, Some(FED)),
                governed(141, Some(CIVVIES)),
                governed(143, None),
            ],
            overlay,
        );
        let stellars = stellars_of(
            &galaxy,
            &[
                listed(140, &[130]),
                listed(141, &[130]),
                listed(142, &[130]),
                listed(144, &[130]),
            ],
            SystemId(130),
        );
        let index_govts: Vec<(i16, Option<GovtId>)> = stellars
            .iter()
            .map(|stellar| (stellar.id.0, stellar.index_govt))
            .collect();
        assert_eq!(
            index_govts,
            [
                (140, Some(FED)),
                (141, Some(CIVVIES)),
                (142, None),
                (144, Some(GovtId(128))),
            ],
            "the hidden 140 counts, 141's overlaid government does not (+0x8 is never written in play), 142 is a gap and 144 past the last system"
        );
    }
}
