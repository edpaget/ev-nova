//! Hypergates and wormholes: stellars that move a ship to another system
//! without fuel and without the minimum jump distance, and where each one
//! leads.
//!
//! Evidence: disassembly of the Mac OS X `EV Nova` (i386, with symbols),
//! and the stock data.
//!
//! - **Entering is landing.** `_HandlePlayerDockRequest` runs the same
//!   checks for a gate as for a planet (the can-land flag, clearance, the
//!   distance and speed checks @0x66beb-0x66cfd). Only where it would call
//!   `_PlayerLandOnStellar` does it branch on `Flags2` ([`GateKind::of`]):
//!   0x1000 enters a hypergate (@0x66ed1), tested first, and 0x2000 a
//!   wormhole (@0x66f0d). So L requests clearance and L again enters, as
//!   the [`landing`](crate::landing) rules say.
//! - **A hypergate lets the player pick.** `_PlayerEnterHypergate`
//!   (@0x637bf) returns at once, silently, when the gate has no links
//!   (`_StellarNumHyperLinks` = 0, [`has_links`]). Otherwise it opens the
//!   galaxy map offering the systems of the linked gates
//!   ([`hypergate_offers`]), and once it closes takes the first link, in
//!   slot order, whose system is the one picked ([`hypergate_exit`],
//!   @0x6389d-0x63999). No pick, or one no link leads to, cancels the
//!   jump.
//! - **A wormhole picks at random** ([`wormhole_exit`]).
//!   `_PlayerEnterWormhole` (@0x64005): with links, a uniform pick among
//!   the slots whose stellar has an owner (`Rand(8)` until one has,
//!   @0x640a3-0x640f0), none failing (@0x64048-0x6409d, @0x6461c).
//!   Without, among every other active wormhole whose owner is another
//!   system than the player's (`Rand(2048)` until one is,
//!   @0x6410e-0x64176, @0x641b8-0x6421c), failing when none of them is
//!   itself unlinked. A pick that has links does not roll again: the ship
//!   comes out of the wormhole it entered, in the player's system
//!   (@0x641f8 to @0x64236-0x64245), the engine's [`WormholeRule`]. The Bible says the ship "will end up at another
//!   random wormhole which also has no defined hyper links", which
//!   [`WormholeRule::UnlinkedOnly`] reads.
//! - **Arrival** ([`emerge`]), identical in both: the ship is placed at the
//!   destination stellar's own position (@0x63a7f-0x63aa6), heading its
//!   `CustSndID` when that is 0-359 and a random heading otherwise
//!   (@0x63aab-0x63adb), and leaves at half its top speed
//!   (`_Accel(heading, ShipMaxSpeed * 0.5)`, @0x63aee-0x63b2f). No fuel is
//!   taken and no day passes: neither function, nor the caller's shared
//!   path, calls `_IncrementGameTime`, which an ordinary jump calls once a
//!   day. That is the engine's [`GateArrivalRule`]; the phase's first
//!   reading, an ordinary jump's arrival and days, is
//!   [`GateArrivalRule::LikeJump`].
//!
//! **Where a gate is.** The gate rules take the system a stellar is in
//! from the session, resolved from every system listing it:
//!
//! - A hypergate's link resolves owner-first: its owner when it has one,
//!   the player's system holding one it lists that no system owns, else
//!   `_FindSystemFromStellar`
//!   (`nova_sim::story::OverlaidGalaxy::system_of`),
//!   then the visible system standing for it (`_FindActiveCoLocatedSystem`,
//!   -1 for -1, @0x4be8). `_DrawMap` (@0xf1f8-0xf21a), `_DoSystemMap`
//!   (@0x12ff2-0x13011) and `_PlayerEnterHypergate` (@0x638cd-0x638ff)
//!   each resolve so: a link with no system visible at its place is
//!   neither drawn nor taken, and the exit is the first link resolving to
//!   the system picked (+0x6c). Only `_MapFilter` (@0x13d87-0x13d98), the
//!   map's Tab / `\` cycle through the hypergate's destinations
//!   ([`hypergate_cycle`], [`next_gate_pick`]), calls
//!   `_FindSystemFromStellar` directly, unresolved by place.
//! - A wormhole reads the owner alone, never `_FindSystemFromStellar`
//!   (`nova_sim::story::OverlaidGalaxy::holder`),
//!   and the ship comes out in the exit's owner (@0x642a7).
//!
//! The exit is then entered as every arrival is (see the
//! `nova_sim::story` module's "Galaxy overlay").
//!
//! A mission's special ships following the player (`ShipSyst` -6) come
//! out of the exit gate after it, or are lost, as the `gövt` `Flags2`
//! bits 0x0020-0x0080 and the odds say (the session's `gate_follow`
//! module, [`RuleKey::GateFollow`](crate::RuleKey::GateFollow)).
//!
//! AI ships coming out of gates (`_AIMakeShipEmergeFromHyperGate`, the
//! `gövt` `Flags2` 0x20-0x80) are modelled in the traffic
//! ([`emerge`](crate::traffic::emerge)), on this module's
//! [`exit_heading`].
//!
//! Every ship the player leads comes out of the exit gate after it,
//! waiting `Rand(20) + 15` ticks and bursting at 15 (loop C,
//! `_HandlePlayerDockRequest` @0x674ef-0x675c8), as
//! [`RuleKey::GateEscorts`](crate::RuleKey::GateEscorts) says: see the
//! session's `gate_escorts` module.
//!
//! Not modelled: the `dësc` QuickTime movie a gate may play
//! (`_PlayQTMovieFromDesc`), AI ships entering gates, the ship-spawn
//! hooks on the shared arrival path (`_SpawnBountyHunter`), the `spöb`
//! `Fee`, and the failure sound for a wormhole with no exit.

use crate::catalog::{GateSite, StellarId, SystemId};
use crate::chance::Chance;
use crate::flight::{ShipState, facing};
use crate::hyperspace::StarMap;
pub use crate::landing::{GateKind, HYPERGATE, WORMHOLE};

/// The fraction of its top speed a ship leaves a gate at: the original's
/// 0.5 (the float at 0xdd694).
pub const EXIT_SPEED_FRACTION: f32 = 0.5;

/// The first stellar ID a link counts for, and how many there can be: the
/// original's `_StellarNumHyperLinks` (@0x812a) counts a slot whose
/// stellar index (ID - 128) is below 0x800.
const FIRST_STELLAR: i16 = 128;
const STELLARS: i16 = 0x800;

/// Why entering a hypergate or wormhole took the ship nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateRefusal {
    /// No entry is pending: the land key has not just been pressed over a
    /// gate of that kind, cleared, or a tick has passed since.
    NotAtGate,
    /// The hypergate has no links (`_StellarNumHyperLinks` is 0): the
    /// original says nothing.
    NoLinks,
    /// No system was picked on the map, or none the hypergate's links lead
    /// to (`STR#` 2002 #50, "Hypergate jump cancelled.").
    Cancelled,
    /// The wormhole leads nowhere (`STR#` 2002 #84 and #86, "the radiation
    /// levels are too extreme").
    NoExit,
}

/// Where a ship comes out of a hypergate or wormhole.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GateArrivalRule {
    /// The original engine's reading (`_PlayerEnterHypergate` @0x639f7,
    /// `_PlayerEnterWormhole` @0x642a1): at the destination gate, heading
    /// its `CustSndID` angle (random outside 0-359) at half its top speed
    /// ([`emerge`]), with no day passing.
    #[default]
    Engine,
    /// The phase's first reading: where an ordinary jump between the two
    /// systems arrives, with the days a jump takes passing; still without
    /// fuel.
    LikeJump,
}

/// Which wormhole a wormhole without links leads to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WormholeRule {
    /// The original engine's reading: a random pick among every other
    /// wormhole in another system, and a pick that has links leaves the
    /// ship at the wormhole it entered (@0x641f8).
    #[default]
    Engine,
    /// The Bible's reading: a random pick among the other unlinked
    /// wormholes in other systems only.
    UnlinkedOnly,
}

/// Whether `link` is a link at all: a stellar ID the original counts
/// (`_StellarNumHyperLinks`).
fn counts(link: StellarId) -> bool {
    (FIRST_STELLAR..FIRST_STELLAR + STELLARS).contains(&link.0)
}

/// Whether a stellar with `links` has any, as `_StellarNumHyperLinks`
/// counts them: a hypergate without leads nowhere, and a wormhole without
/// leads to another wormhole.
#[must_use]
pub fn has_links(links: &[Option<StellarId>; 8]) -> bool {
    links.iter().flatten().any(|&link| counts(link))
}

/// The site `id` names among `sites`: a stellar some system lists.
fn site(sites: &[GateSite], id: StellarId) -> Option<&GateSite> {
    sites.iter().find(|site| site.id == id)
}

/// The sites `links` lead to, in slot order, repeats kept: each used slot
/// whose stellar some system lists.
fn exits<'a>(links: &[Option<StellarId>; 8], sites: &'a [GateSite]) -> Vec<&'a GateSite> {
    links
        .iter()
        .flatten()
        .filter_map(|&link| site(sites, link))
        .collect()
}

/// The systems a hypergate with `links` offers on the overlaid `map`: each
/// linked gate's system, as `system_of` resolves it owner-first
/// (`nova_sim::story::OverlaidGalaxy::system_of`),
/// then as the visible system standing for it
/// ([`Places::stand_in`](crate::hyperspace::Places::stand_in)), in slot
/// order, each once. A link to a stellar no system lists, one that
/// resolves to no system, and one with no system visible at its place
/// offer nothing, as `_DrawMap` resolves each link and skips -1
/// (@0xf1f8-0xf21a).
#[must_use]
pub fn hypergate_offers(
    links: &[Option<StellarId>; 8],
    sites: &[GateSite],
    system_of: impl Fn(StellarId) -> Option<SystemId>,
    map: &StarMap,
) -> Vec<SystemId> {
    let mut offers = Vec::new();
    for exit in exits(links, sites) {
        if let Some(offer) = system_of(exit.id).and_then(|system| map.places().stand_in(system))
            && !offers.contains(&offer)
        {
            offers.push(offer);
        }
    }
    offers
}

/// The gate a hypergate with `links` leads to when the player picks
/// `chosen`: the first link, in slot order, whose gate's system, as
/// `system_of` resolves it owner-first, has `chosen` standing for it on
/// the overlaid `map`, as `_PlayerEnterHypergate` compares each link's
/// `_FindActiveCoLocatedSystem` with the system picked
/// (@0x638cd-0x638ff). `None` when no link leads there.
#[must_use]
pub fn hypergate_exit<'a>(
    links: &[Option<StellarId>; 8],
    sites: &'a [GateSite],
    chosen: SystemId,
    system_of: impl Fn(StellarId) -> Option<SystemId>,
    map: &StarMap,
) -> Option<&'a GateSite> {
    exits(links, sites).into_iter().find(|exit| {
        system_of(exit.id).and_then(|system| map.places().stand_in(system)) == Some(chosen)
    })
}

/// The destinations the hypergate map's Tab / `\` cycles through for a
/// hypergate with `links`, as `_MapFilter` lists them (@0x13d58-0x13daa):
/// each link that counts, in slot order, as `find` gives its system,
/// `_FindSystemFromStellar` unresolved by place
/// (`nova_sim::story::OverlaidGalaxy::find_system_from_stellar`);
/// a link no system lists gives `None`, and stays in the list.
#[must_use]
pub fn hypergate_cycle(
    links: &[Option<StellarId>; 8],
    find: impl Fn(StellarId) -> Option<SystemId>,
) -> Vec<Option<SystemId>> {
    links
        .iter()
        .flatten()
        .filter(|&&link| counts(link))
        .map(|&link| find(link))
        .collect()
}

/// The hypergate map's next pick from `selected`, the map's selection, on
/// Tab or `\`, as `_MapFilter` steps through the `cycle`
/// ([`hypergate_cycle`], @0x13db5-0x13f0a): from the first entry equal
/// to `selected` (`None` matching a `None` entry, as −1 matches −1), the
/// one before it with Shift (`back`), the first wrapping to the last, and
/// otherwise the one after it, the last wrapping to the first; from a
/// selection not in the cycle, the first entry either way. So a
/// destination listed twice keeps a plain press on its second copy. With
/// an empty cycle, `None`: nothing changes (@0x13dac).
#[must_use]
pub fn next_gate_pick(
    cycle: &[Option<SystemId>],
    selected: Option<SystemId>,
    back: bool,
) -> Option<Option<SystemId>> {
    let last = cycle.len().checked_sub(1)?;
    let next = match cycle.iter().position(|&entry| entry == selected) {
        None => 0,
        Some(at) if back => at.checked_sub(1).unwrap_or(last),
        Some(at) => (at + 1) % cycle.len(),
    };
    Some(cycle[next])
}

/// Where a ship comes out of a wormhole ([`wormhole_exit`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WormholeExit<'a> {
    /// The wormhole it comes out of.
    pub site: &'a GateSite,
    /// The system it comes out in.
    pub system: SystemId,
}

/// The wormhole a ship entering wormhole `here`, with the player in
/// `player_system`, comes out of, and the system it comes out in, rolled
/// on `chance`; `None` when it leads nowhere (see the module docs).
/// `owner_of` gives a stellar's owner, the player's system holding one it
/// lists that no system owns
/// (`nova_sim::story::OverlaidGalaxy::holder`):
/// `_PlayerEnterWormhole` reads only the owner (@0x64005-0x642a7).
///
/// With links, `chance` rolls over the slots whose stellar some system
/// lists and has an owner, in slot order, repeats counting, and the ship
/// comes out in that owner (@0x640a3-0x640f0, @0x642a7); none such means
/// no exit (@0x64048-0x6409d, @0x6461c). Without, the candidates are
/// every other wormhole among `sites`, by ascending ID, with an owner that
/// is not `player_system` (@0x6410e-0x64176), and there is no exit unless
/// one of them is unlinked: by [`WormholeRule::Engine`] `chance` rolls
/// over all of them, and a linked pick gives `here` in `player_system`
/// (@0x64236-0x64245); by [`WormholeRule::UnlinkedOnly`] over the
/// unlinked ones only.
pub fn wormhole_exit<'a>(
    here: StellarId,
    player_system: SystemId,
    sites: &'a [GateSite],
    owner_of: impl Fn(StellarId) -> Option<SystemId>,
    rule: WormholeRule,
    chance: &mut (impl Chance + ?Sized),
) -> Option<WormholeExit<'a>> {
    let (entered, choices) = wormhole_choices(here, player_system, sites, owner_of)?;
    let (candidates, unlinked) = match choices {
        WormholeChoices::Linked(owned) => return pick(&owned, chance).copied(),
        WormholeChoices::Unlinked {
            candidates,
            unlinked,
        } => (candidates, unlinked),
    };
    match rule {
        WormholeRule::Engine => {
            let picked = *pick(&candidates, chance)?;
            Some(if has_links(&picked.site.links) {
                WormholeExit {
                    site: entered,
                    system: player_system,
                }
            } else {
                picked
            })
        }
        WormholeRule::UnlinkedOnly => pick(&unlinked, chance).copied(),
    }
}

/// Whether a ship entering wormhole `here`, with the player in
/// `player_system`, comes out anywhere: whether [`wormhole_exit`] gives an
/// exit, by either [`WormholeRule`], read without rolling. The exit's
/// roll only picks among its candidates, so whether there is one doesn't
/// depend on it. The view asks it before the wormhole's movie, which the
/// original plays only past the no-exit refusal
/// (`_PlayerEnterWormhole` @0x6409d, @0x641a6-0x641b3, before the movie
/// @0x6427d).
pub fn wormhole_leads_anywhere(
    here: StellarId,
    player_system: SystemId,
    sites: &[GateSite],
    owner_of: impl Fn(StellarId) -> Option<SystemId>,
) -> bool {
    wormhole_choices(here, player_system, sites, owner_of).is_some()
}

/// What a wormhole entered rolls over ([`wormhole_choices`]).
enum WormholeChoices<'a> {
    /// A linked wormhole's owned exits, in slot order, repeats kept.
    Linked(Vec<WormholeExit<'a>>),
    /// An unlinked wormhole's: every other wormhole elsewhere, and the
    /// unlinked ones among them.
    Unlinked {
        candidates: Vec<WormholeExit<'a>>,
        unlinked: Vec<WormholeExit<'a>>,
    },
}

/// The wormhole `here` as listed, and what [`wormhole_exit`] rolls over
/// from it (see there); `None` when it leads nowhere: no system lists it,
/// none of its links has an owner, or, unlinked, no unlinked wormhole is
/// owned elsewhere.
fn wormhole_choices<'a>(
    here: StellarId,
    player_system: SystemId,
    sites: &'a [GateSite],
    owner_of: impl Fn(StellarId) -> Option<SystemId>,
) -> Option<(&'a GateSite, WormholeChoices<'a>)> {
    let entered = site(sites, here)?;
    let out = |site: &'a GateSite| owner_of(site.id).map(|system| WormholeExit { site, system });
    if has_links(&entered.links) {
        let owned: Vec<WormholeExit<'a>> = exits(&entered.links, sites)
            .into_iter()
            .filter_map(out)
            .collect();
        return (!owned.is_empty()).then_some((entered, WormholeChoices::Linked(owned)));
    }
    let candidates: Vec<WormholeExit<'a>> = sites
        .iter()
        .filter(|site| site.id != here && site.flags2 & WORMHOLE != 0)
        .filter_map(out)
        .filter(|exit| exit.system != player_system)
        .collect();
    let unlinked: Vec<WormholeExit<'a>> = candidates
        .iter()
        .copied()
        .filter(|exit| !has_links(&exit.site.links))
        .collect();
    (!unlinked.is_empty()).then_some((
        entered,
        WormholeChoices::Unlinked {
            candidates,
            unlinked,
        },
    ))
}

/// One of `choices`, rolled on `chance`; `None` when there are none.
fn pick<'c, T>(choices: &'c [T], chance: &mut (impl Chance + ?Sized)) -> Option<&'c T> {
    let sides = u16::try_from(choices.len()).unwrap_or(u16::MAX);
    if sides == 0 {
        return None;
    }
    choices.get(usize::from(chance.roll(sides)))
}

/// The heading a ship comes out of a gate whose `CustSndID` is
/// `exit_angle` on: that angle when it is 0 to 359, and otherwise one
/// rolled on `chance` (`Rand(360)`). The player's arrival (@0x63aab-0x63adb)
/// and an NPC's (`_AIMakeShipEmergeFromHyperGate` @0x8957b-0x895a5) read it
/// alike.
pub fn exit_heading(exit_angle: i16, chance: &mut (impl Chance + ?Sized)) -> f32 {
    let heading = match u16::try_from(exit_angle) {
        Ok(angle) if angle < 360 => angle,
        _ => chance.roll(360),
    };
    f32::from(heading)
}

/// A ship of top speed `max_speed` coming out of `site`: at its centre,
/// heading its [`exit_heading`], moving along that heading at
/// [`EXIT_SPEED_FRACTION`] of its top speed.
pub fn emerge(site: &GateSite, max_speed: f32, chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let heading = exit_heading(site.exit_angle, chance);
    ShipState {
        position: site.position,
        velocity: facing(heading) * (EXIT_SPEED_FRACTION * max_speed),
        heading,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::geometry::Vec2;
    use crate::hyperspace::CoLocated;
    use nova_sim_base::fixture::Scripted;
    use nova_sim_catalog::fixture::star;

    #[test]
    fn the_next_gate_pick_steps_on_and_shift_steps_back_each_wrapping() {
        let (a, b) = (Some(SystemId(131)), Some(SystemId(132)));
        let cycle = [a, None, b];
        assert_eq!(next_gate_pick(&cycle, a, false), Some(None));
        assert_eq!(
            next_gate_pick(&cycle, None, false),
            Some(b),
            "-1 is found too"
        );
        assert_eq!(next_gate_pick(&cycle, b, false), Some(a), "wraps");
        assert_eq!(next_gate_pick(&cycle, b, true), Some(None));
        assert_eq!(next_gate_pick(&cycle, a, true), Some(b), "wraps back");
    }

    #[test]
    fn the_next_gate_pick_from_a_selection_not_listed_is_the_first_either_way() {
        let cycle = [Some(SystemId(131)), Some(SystemId(132))];
        for back in [false, true] {
            assert_eq!(
                next_gate_pick(&cycle, Some(SystemId(130)), back),
                Some(Some(SystemId(131))),
                "{back}"
            );
            assert_eq!(
                next_gate_pick(&cycle, None, back),
                Some(Some(SystemId(131))),
                "{back}"
            );
        }
    }

    #[test]
    fn the_next_gate_pick_steps_on_from_the_first_copy_of_a_repeat() {
        let (a, b) = (Some(SystemId(131)), Some(SystemId(132)));
        assert_eq!(
            next_gate_pick(&[a, a, b], a, false),
            Some(a),
            "stuck on 131"
        );
        assert_eq!(next_gate_pick(&[b, a, a], a, true), Some(b));
    }

    #[test]
    fn the_hypergate_cycle_lists_each_counted_link_in_slot_order_unresolved_or_not() {
        let mut links = [None; 8];
        links[0] = Some(StellarId(130));
        links[2] = Some(StellarId(127));
        links[3] = Some(StellarId(129));
        links[5] = Some(StellarId(131));
        let find = |link: StellarId| (link.0 != 129).then_some(SystemId(link.0 + 100));
        assert_eq!(
            hypergate_cycle(&links, find),
            [Some(SystemId(230)), None, Some(SystemId(231))],
            "127 doesn't count; 129 no system lists"
        );
        assert_eq!(hypergate_cycle(&[None; 8], find), []);
    }

    #[test]
    fn no_next_gate_pick_from_an_empty_cycle() {
        for back in [false, true] {
            assert_eq!(next_gate_pick(&[], Some(SystemId(131)), back), None);
        }
    }

    /// The links in `ids`, in slots from the first, the rest unused.
    fn links(ids: &[i16]) -> [Option<StellarId>; 8] {
        let mut slots = [None; 8];
        for (slot, &id) in slots.iter_mut().zip(ids) {
            *slot = Some(StellarId(id));
        }
        slots
    }

    /// Stellar `id` at (`id`, 0), with `flags2` and links to `linked`,
    /// heading ships out on 90°.
    fn gate_site(id: i16, flags2: u16, linked: &[i16]) -> GateSite {
        GateSite {
            id: StellarId(id),
            position: Vec2::new(f32::from(id), 0.0),
            flags2,
            links: links(linked),
            exit_angle: 90,
        }
    }

    fn wormhole(id: i16, linked: &[i16]) -> GateSite {
        gate_site(id, WORMHOLE, linked)
    }

    /// A resolver giving each stellar of `systems` its system, and any
    /// other none.
    fn resolver(systems: &[(i16, i16)]) -> impl Fn(StellarId) -> Option<SystemId> + use<> {
        let systems: BTreeMap<i16, i16> = systems.iter().copied().collect();
        move |id| systems.get(&id.0).map(|&system| SystemId(system))
    }

    #[test]
    fn the_named_values() {
        assert_eq!((HYPERGATE, WORMHOLE), (0x1000, 0x2000));
        assert_eq!(EXIT_SPEED_FRACTION, 0.5);
        assert_eq!(GateArrivalRule::default(), GateArrivalRule::Engine);
        assert_eq!(WormholeRule::default(), WormholeRule::Engine);
    }

    #[test]
    fn a_gates_kind_is_its_flags2_hypergate_first() {
        assert_eq!(GateKind::of(0x1000), Some(GateKind::Hypergate));
        assert_eq!(GateKind::of(0x1200), Some(GateKind::Hypergate));
        assert_eq!(GateKind::of(0x2000), Some(GateKind::Wormhole));
        assert_eq!(GateKind::of(0x2200), Some(GateKind::Wormhole));
        assert_eq!(GateKind::of(0x3000), Some(GateKind::Hypergate), "both");
        assert_eq!(GateKind::of(0x0200), None);
        assert_eq!(GateKind::of(0xCFFF), None);
    }

    #[test]
    fn links_count_from_stellar_128_for_2048_stellars() {
        assert!(!has_links(&[None; 8]));
        assert!(has_links(&links(&[128])));
        assert!(has_links(&links(&[0, 2175])));
        assert!(!has_links(&links(&[0, 127, 2176, -2])));
    }

    #[test]
    fn a_hypergate_offers_its_linked_gates_systems_in_slot_order_once_each() {
        let sites = [
            gate_site(1400, HYPERGATE, &[]),
            gate_site(1405, HYPERGATE, &[]),
            gate_site(1413, HYPERGATE, &[]),
            gate_site(1414, HYPERGATE, &[]),
        ];
        let system_of = resolver(&[(1400, 128), (1405, 129), (1413, 298), (1414, 298)]);
        let map = StarMap::new(vec![
            star(128, (0.0, 0.0), &[]),
            star(129, (10.0, 0.0), &[]),
            star(298, (20.0, 0.0), &[]),
        ]);
        let mut slots = links(&[1413, 999, 1405]);
        slots[4] = Some(StellarId(1414));
        slots[6] = Some(StellarId(1405));
        assert_eq!(
            hypergate_offers(&slots, &sites, &system_of, &map),
            [SystemId(298), SystemId(129)],
            "an unlisted stellar skipped, and each system once"
        );
        assert_eq!(hypergate_offers(&[None; 8], &sites, &system_of, &map), []);
    }

    /// 1405 in 129, 1413 in 298 and 1414 in 541, which share a place, and
    /// 1418 in 483, alone; 298 and 483 hidden.
    fn hidden_listers() -> (
        [GateSite; 4],
        impl Fn(StellarId) -> Option<SystemId>,
        StarMap,
    ) {
        let sites = [
            gate_site(1405, HYPERGATE, &[]),
            gate_site(1413, HYPERGATE, &[]),
            gate_site(1414, HYPERGATE, &[]),
            gate_site(1418, HYPERGATE, &[]),
        ];
        let system_of = resolver(&[(1405, 129), (1413, 298), (1414, 541), (1418, 483)]);
        let map = StarMap::with_hidden(
            vec![
                star(129, (0.0, 0.0), &[]),
                star(298, (100.0, 100.0), &[]),
                star(483, (100.0, 0.0), &[]),
                star(541, (100.0, 100.0), &[]),
            ],
            &[SystemId(298), SystemId(483)].into(),
            CoLocated::Lowest,
        );
        (sites, system_of, map)
    }

    #[test]
    fn hypergate_offers_resolve_each_link_through_the_resolver_then_by_place() {
        let (sites, system_of, map) = hidden_listers();
        let offers = |ids: &[i16]| hypergate_offers(&links(ids), &sites, &system_of, &map);
        assert_eq!(
            offers(&[1413, 1405, 1418, 1414]),
            [SystemId(541), SystemId(129)],
            "298 by 541, once; 483 by none (@0xf20f-0xf21a)"
        );
        assert_eq!(offers(&[]), []);
        let shadowed = resolver(&[(1414, 541)]);
        let every = StarMap::new(vec![
            star(298, (100.0, 100.0), &[]),
            star(541, (100.0, 100.0), &[]),
        ]);
        assert_eq!(
            hypergate_offers(&links(&[1414, 1405]), &sites, &shadowed, &every),
            [SystemId(298)],
            "541 shadowed by 298 offers 298; 1405 resolves to none"
        );
    }

    #[test]
    fn hypergate_exit_takes_the_first_link_whose_resolved_place_is_the_pick() {
        let sites = [
            gate_site(1405, HYPERGATE, &[]),
            gate_site(1413, HYPERGATE, &[]),
            gate_site(1414, HYPERGATE, &[]),
            gate_site(1418, HYPERGATE, &[]),
        ];
        let system_of = resolver(&[(1405, 129), (1413, 298), (1414, 541), (1418, 483)]);
        // 541 shares Dani's (298's) position; 483 sits apart.
        let map = StarMap::new(vec![
            star(129, (0.0, 0.0), &[]),
            star(298, (100.0, 100.0), &[]),
            star(483, (100.0, 0.0), &[]),
            star(541, (100.0, 100.0), &[]),
            star(600, (200.0, 0.0), &[]),
        ]);
        let slots = links(&[1405, 1414, 1413, 1418]);
        let exit = |chosen| {
            hypergate_exit(&slots, &sites, SystemId(chosen), &system_of, &map).map(|s| s.id.0)
        };
        assert_eq!(exit(129), Some(1405));
        assert_eq!(
            exit(298),
            Some(1414),
            "the first slot whose place is 298: 541 shadowed by it"
        );
        assert_eq!(exit(541), None, "shadowed, never what a link resolves to");
        assert_eq!(exit(483), Some(1418));
        assert_eq!(exit(600), None, "no link leads there");
        assert_eq!(exit(999), None, "not on the map");
    }

    #[test]
    fn a_hypergate_leads_through_a_hidden_listers_gate_to_the_system_at_its_place() {
        let (sites, system_of, map) = hidden_listers();
        let slots = links(&[1413, 1418, 1405]);
        let exit = |chosen| {
            hypergate_exit(&slots, &sites, SystemId(chosen), &system_of, &map).map(|s| s.id.0)
        };
        assert_eq!(exit(541), Some(1413), "standing for 298 (@0x638ed)");
        assert_eq!(exit(298), None, "hidden, never the pick's compare");
        assert_eq!(exit(483), None, "no system visible at its place");
        assert_eq!(exit(129), Some(1405));
    }

    #[test]
    fn a_gate_off_the_map_matches_only_its_own_system() {
        let sites = [gate_site(1405, HYPERGATE, &[])];
        let map = StarMap::new(vec![star(129, (0.0, 0.0), &[])]);
        let slots = links(&[1405]);
        let off_the_map = resolver(&[(1405, 700)]);
        let exit = |chosen, system_of: &dyn Fn(StellarId) -> Option<SystemId>| {
            hypergate_exit(&slots, &sites, SystemId(chosen), system_of, &map).map(|s| s.id.0)
        };
        assert_eq!(exit(700, &off_the_map), Some(1405));
        assert_eq!(exit(701, &off_the_map), None);
        assert_eq!(
            hypergate_offers(&slots, &sites, &off_the_map, &map),
            [SystemId(700)]
        );
        let unresolved = resolver(&[]);
        assert_eq!(exit(129, &unresolved), None, "a link to no system");
        assert_eq!(hypergate_offers(&slots, &sites, &unresolved, &map), []);
    }

    /// The exit's stellar and system.
    fn exit_of(exit: Option<WormholeExit<'_>>) -> Option<(i16, i16)> {
        exit.map(|exit| (exit.site.id.0, exit.system.0))
    }

    #[test]
    fn a_linked_wormhole_rolls_over_its_usable_slots() {
        let sites = [
            wormhole(500, &[510, 999, 520, 510]),
            wormhole(510, &[]),
            wormhole(520, &[]),
        ];
        let owner_of = resolver(&[(500, 130), (510, 131), (520, 132)]);
        for (roll, expected) in [(0, (510, 131)), (1, (520, 132)), (2, (510, 131))] {
            let mut chance = Scripted::rolling(&[roll]);
            let exit = wormhole_exit(
                StellarId(500),
                SystemId(130),
                &sites,
                &owner_of,
                WormholeRule::Engine,
                &mut chance,
            );
            assert_eq!(exit_of(exit), Some(expected), "{roll}");
            assert_eq!(chance.sides_asked, [3], "repeats count");
        }
    }

    #[test]
    fn wormhole_links_lead_only_to_linked_stellars_with_an_owner() {
        let sites = [
            wormhole(500, &[510, 520]),
            wormhole(510, &[]),
            wormhole(520, &[]),
        ];
        let owner_of = resolver(&[(500, 130), (520, 132)]);
        let mut chance = Scripted::rolling(&[0]);
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            &sites,
            &owner_of,
            WormholeRule::Engine,
            &mut chance,
        );
        assert_eq!(exit_of(exit), Some((520, 132)), "510 has no owner");
        assert_eq!(chance.sides_asked, [1]);
        let unowned = resolver(&[(500, 130)]);
        let mut chance = Scripted::default();
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            &sites,
            &unowned,
            WormholeRule::Engine,
            &mut chance,
        );
        assert_eq!(exit_of(exit), None, "no linked owner at all");
        assert!(chance.sides_asked.is_empty(), "nothing rolled");
    }

    #[test]
    fn a_wormhole_comes_out_in_its_exits_owner() {
        let sites = [wormhole(500, &[510]), wormhole(510, &[])];
        let owner_of = resolver(&[(510, 140)]);
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            &sites,
            &owner_of,
            WormholeRule::Engine,
            &mut Scripted::rolling(&[0]),
        );
        assert_eq!(
            exit_of(exit),
            Some((510, 140)),
            "the owner, as the roll found it"
        );
    }

    #[test]
    fn a_linked_wormhole_whose_links_lead_nowhere_has_no_exit() {
        let sites = [wormhole(500, &[999]), wormhole(510, &[])];
        let owner_of = resolver(&[(500, 130), (510, 131), (999, 132)]);
        let mut chance = Scripted::default();
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            &sites,
            &owner_of,
            WormholeRule::Engine,
            &mut chance,
        );
        assert_eq!(exit_of(exit), None, "999 is no listed stellar");
        assert!(chance.sides_asked.is_empty(), "nothing rolled");
    }

    /// An unlinked wormhole 500 in system 130, with another (505) and a
    /// hypergate (506) there too; elsewhere unlinked wormholes 510 and
    /// 530, linked one 520 between them, and a planet 540.
    fn unlinked_network() -> Vec<GateSite> {
        vec![
            wormhole(500, &[]),
            wormhole(505, &[]),
            gate_site(506, HYPERGATE, &[]),
            wormhole(510, &[]),
            wormhole(520, &[510]),
            wormhole(530, &[]),
            gate_site(540, 0, &[]),
        ]
    }

    /// [`unlinked_network`]'s owners.
    fn network_owners() -> impl Fn(StellarId) -> Option<SystemId> {
        resolver(&[
            (500, 130),
            (505, 130),
            (506, 130),
            (510, 131),
            (520, 132),
            (530, 133),
            (540, 134),
        ])
    }

    fn unlinked_exit(
        sites: &[GateSite],
        owner_of: &dyn Fn(StellarId) -> Option<SystemId>,
        rule: WormholeRule,
        roll: u16,
    ) -> (Option<(i16, i16)>, Vec<u16>) {
        let mut chance = Scripted::rolling(&[roll]);
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            sites,
            owner_of,
            rule,
            &mut chance,
        );
        (exit_of(exit), chance.sides_asked)
    }

    #[test]
    fn by_the_engine_an_unlinked_wormhole_rolls_over_every_wormhole_elsewhere() {
        let (sites, owners) = (unlinked_network(), network_owners());
        let engine = |roll| unlinked_exit(&sites, &owners, WormholeRule::Engine, roll);
        assert_eq!(engine(0), (Some((510, 131)), vec![3]));
        assert_eq!(engine(2), (Some((530, 133)), vec![3]));
    }

    #[test]
    fn by_the_engine_a_linked_unlinked_pick_stays_at_the_entered_wormhole_in_the_players_system() {
        let (sites, owners) = (unlinked_network(), network_owners());
        let mut chance = Scripted::rolling(&[2]);
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(150),
            &sites,
            &owners,
            WormholeRule::Engine,
            &mut chance,
        );
        assert_eq!(
            exit_of(exit),
            Some((500, 150)),
            "520 is linked: the ship stays where it entered, in the player's system"
        );
        assert_eq!(chance.sides_asked, [4], "505, 510, 520 and 530, by ID");
    }

    #[test]
    fn by_the_bible_an_unlinked_wormhole_rolls_over_the_unlinked_ones_only() {
        let (sites, owners) = (unlinked_network(), network_owners());
        let bible = |roll| unlinked_exit(&sites, &owners, WormholeRule::UnlinkedOnly, roll);
        assert_eq!(bible(0), (Some((510, 131)), vec![2]));
        assert_eq!(bible(1), (Some((530, 133)), vec![2]));
    }

    #[test]
    fn unlinked_wormhole_candidates_need_an_owner_other_than_the_players_system() {
        let sites = unlinked_network();
        let owners = resolver(&[(500, 130), (510, 130), (520, 132), (530, 140)]);
        let engine = |roll| unlinked_exit(&sites, &owners, WormholeRule::Engine, roll);
        assert_eq!(
            engine(1),
            (Some((530, 140)), vec![2]),
            "510 is the player's system's; 505 has no owner"
        );
        assert_eq!(engine(0), (Some((500, 130)), vec![2]), "520, linked");
        let none_elsewhere = resolver(&[(500, 130), (510, 130), (530, 130)]);
        assert_eq!(
            unlinked_exit(&sites, &none_elsewhere, WormholeRule::Engine, 0),
            (None, vec![])
        );
    }

    #[test]
    fn an_unlinked_wormhole_with_only_linked_ones_elsewhere_has_no_exit() {
        let sites = [
            wormhole(500, &[]),
            wormhole(505, &[]),
            wormhole(520, &[500]),
        ];
        let owners = resolver(&[(500, 130), (505, 130), (520, 132)]);
        for rule in [WormholeRule::Engine, WormholeRule::UnlinkedOnly] {
            assert_eq!(
                unlinked_exit(&sites, &owners, rule, 0),
                (None, vec![]),
                "{rule:?}"
            );
        }
    }

    #[test]
    fn a_wormhole_leads_anywhere_exactly_when_it_has_an_exit_by_either_rule() {
        type Owners = Box<dyn Fn(StellarId) -> Option<SystemId>>;
        let network = unlinked_network();
        let linked = [
            wormhole(500, &[510, 520]),
            wormhole(510, &[]),
            wormhole(520, &[]),
        ];
        let only_linked_elsewhere = [
            wormhole(500, &[]),
            wormhole(505, &[]),
            wormhole(520, &[500]),
        ];
        let cases: [(&[GateSite], Owners, bool); 6] = [
            (&network, Box::new(network_owners()), true),
            (
                &network,
                Box::new(resolver(&[(500, 130), (510, 130), (530, 130)])),
                false,
            ),
            (&linked, Box::new(resolver(&[(500, 130), (520, 132)])), true),
            (&linked, Box::new(resolver(&[(500, 130)])), false),
            (
                &only_linked_elsewhere,
                Box::new(resolver(&[(500, 130), (505, 130), (520, 132)])),
                false,
            ),
            (&network[3..4], Box::new(resolver(&[(510, 131)])), false),
        ];
        for (at, (sites, owners, leads)) in cases.iter().enumerate() {
            assert_eq!(
                wormhole_leads_anywhere(StellarId(500), SystemId(130), sites, owners),
                *leads,
                "case {at}"
            );
            for rule in [WormholeRule::Engine, WormholeRule::UnlinkedOnly] {
                let exit = wormhole_exit(
                    StellarId(500),
                    SystemId(130),
                    sites,
                    owners,
                    rule,
                    &mut Scripted::rolling(&[0]),
                );
                assert_eq!(exit.is_some(), *leads, "case {at}, {rule:?}");
            }
        }
    }

    #[test]
    fn a_wormhole_no_system_lists_has_no_exit() {
        let sites = [wormhole(510, &[])];
        let owners = resolver(&[(510, 131)]);
        assert_eq!(
            unlinked_exit(&sites, &owners, WormholeRule::Engine, 0),
            (None, vec![])
        );
    }

    fn emerging(exit_angle: i16, rolls: &[u16]) -> (ShipState, Vec<u16>) {
        let site = GateSite {
            position: Vec2::new(-400.0, -500.0),
            exit_angle,
            ..gate_site(1405, HYPERGATE, &[])
        };
        let mut chance = Scripted::rolling(rolls);
        (emerge(&site, 6.0, &mut chance), chance.sides_asked)
    }

    #[test]
    fn a_ship_emerges_at_the_gate_on_its_angle_at_half_its_top_speed() {
        let (ship, asked) = emerging(90, &[]);
        assert_eq!(ship.position, Vec2::new(-400.0, -500.0));
        assert_eq!(ship.heading, 90.0);
        assert!((ship.velocity - Vec2::new(3.0, 0.0)).length() < 1e-5);
        assert!(asked.is_empty(), "no roll");
        for angle in [0, 359] {
            let (ship, asked) = emerging(angle, &[7]);
            assert_eq!(ship.heading, f32::from(angle));
            assert!(asked.is_empty());
        }
        let (ship, _) = emerging(180, &[]);
        assert!((ship.velocity - Vec2::new(0.0, 3.0)).length() < 1e-5);
    }

    #[test]
    fn an_angle_outside_0_to_359_rolls_the_heading() {
        for angle in [360, -1, i16::MIN, i16::MAX] {
            let (ship, asked) = emerging(angle, &[270]);
            assert_eq!(ship.heading, 270.0, "{angle}");
            assert_eq!(asked, [360], "{angle}");
            assert!((ship.velocity - Vec2::new(-3.0, 0.0)).length() < 1e-5);
        }
    }

    #[test]
    fn the_exit_heading_is_the_angle_from_0_to_359_and_rolled_otherwise() {
        for angle in [0, 90, 359] {
            let mut chance = Scripted::rolling(&[7]);
            assert_eq!(exit_heading(angle, &mut chance), f32::from(angle));
            assert!(chance.sides_asked.is_empty(), "{angle}: no roll");
        }
        for angle in [360, -1, i16::MIN, i16::MAX] {
            let mut chance = Scripted::rolling(&[123]);
            assert_eq!(exit_heading(angle, &mut chance), 123.0, "{angle}");
            assert_eq!(chance.sides_asked, [360], "{angle}");
        }
    }
}
