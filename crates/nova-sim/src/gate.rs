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
//!   ([`hypergate_destinations`]), and once it closes takes the first
//!   link, in slot order, whose system is the one picked
//!   ([`hypergate_exit`], @0x6389d-0x63999). No pick, or one no link
//!   leads to, cancels the jump.
//! - **A wormhole picks at random** ([`wormhole_exit`]).
//!   `_PlayerEnterWormhole` (@0x64005): with links, a uniform pick among
//!   the slots whose stellar is in a system (`Rand(8)` until one is,
//!   @0x640a3-0x640f6), none failing. Without, among every other wormhole
//!   in another system (`Rand(2048)` until one is, @0x641b8-0x641eb),
//!   failing when none of them is itself unlinked (@0x640fb-0x64186). A
//!   pick that has links does not roll again: the ship comes out of the
//!   wormhole it entered (@0x641f8 to @0x64236-0x64245), the engine's
//!   [`WormholeRule`]. The Bible says the ship "will end up at another
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
//! A stellar's system is the lowest-ID system that lists it, and a link
//! to a stellar no system lists leads nowhere: the original's
//! `_FindSystemFromStellar` with every system active, since the map
//! models no system Visibility.
//!
//! Not modelled: the `dësc` QuickTime movie a gate may play
//! (`_PlayQTMovieFromDesc`), AI ships using or coming out of gates
//! (`_AIMakeShipEmergeFromHyperGate`, the `gövt` flags 0x20-0x80), the
//! mission and ship-spawn hooks on the shared arrival path, system
//! Visibility, the `spöb` `Fee`, and the failure sound for a wormhole with
//! no exit.

use crate::catalog::{GateSite, StellarId, SystemId};
use crate::chance::Chance;
use crate::flight::{ShipState, facing};
use crate::hyperspace::StarMap;

/// The `spöb` `Flags2` bit of a hypergate.
pub const HYPERGATE: u16 = 0x1000;
/// The `spöb` `Flags2` bit of a wormhole.
pub const WORMHOLE: u16 = 0x2000;
/// The fraction of its top speed a ship leaves a gate at: the original's
/// 0.5 (the float at 0xdd694).
pub const EXIT_SPEED_FRACTION: f32 = 0.5;

/// The first stellar ID a link counts for, and how many there can be: the
/// original's `_StellarNumHyperLinks` (@0x812a) counts a slot whose
/// stellar index (ID - 128) is below 0x800.
const FIRST_STELLAR: i16 = 128;
const STELLARS: i16 = 0x800;

/// What a stellar a ship can enter leads through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateKind {
    /// A hypergate: the player picks among its linked gates.
    Hypergate,
    /// A wormhole: it leads somewhere at random.
    Wormhole,
}

impl GateKind {
    /// What a stellar with `Flags2` `flags2` is: a hypergate when it has
    /// the hypergate bit (tested first, as the original does), a wormhole
    /// when it has the wormhole bit, and otherwise no gate.
    #[must_use]
    pub fn of(flags2: u16) -> Option<Self> {
        if flags2 & HYPERGATE != 0 {
            Some(Self::Hypergate)
        } else if flags2 & WORMHOLE != 0 {
            Some(Self::Wormhole)
        } else {
            None
        }
    }
}

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

/// The systems a hypergate with `links` offers on the map: each linked
/// gate's system, in slot order, each once.
#[must_use]
pub fn hypergate_destinations(links: &[Option<StellarId>; 8], sites: &[GateSite]) -> Vec<SystemId> {
    let mut systems = Vec::new();
    for exit in exits(links, sites) {
        if !systems.contains(&exit.system) {
            systems.push(exit.system);
        }
    }
    systems
}

/// The gate a hypergate with `links` leads to when the player picks
/// `chosen`: the first link, in slot order, whose system is `chosen` or
/// sits at its map position on `map`, as `_FindActiveCoLocatedSystem`
/// resolves a system. `None` when no link leads there.
#[must_use]
pub fn hypergate_exit<'a>(
    links: &[Option<StellarId>; 8],
    sites: &'a [GateSite],
    chosen: SystemId,
    map: &StarMap,
) -> Option<&'a GateSite> {
    let at = map.position(chosen);
    exits(links, sites)
        .into_iter()
        .find(|exit| exit.system == chosen || (at.is_some() && map.position(exit.system) == at))
}

/// The wormhole a ship entering wormhole `here`, in `current_system`,
/// comes out of, rolled on `chance`; `None` when it leads nowhere (see the
/// module docs).
///
/// With links, `chance` rolls over the slots whose stellar some system
/// lists, in slot order, repeats counting. Without, the candidates are
/// every other wormhole among `sites`, by ascending ID, in a system other
/// than `current_system`, and there is no exit unless one of them is
/// unlinked: by [`WormholeRule::Engine`] `chance` rolls over all of them,
/// and a linked pick gives `here`; by [`WormholeRule::UnlinkedOnly`] over
/// the unlinked ones only.
pub fn wormhole_exit<'a>(
    here: StellarId,
    current_system: SystemId,
    sites: &'a [GateSite],
    rule: WormholeRule,
    chance: &mut (impl Chance + ?Sized),
) -> Option<&'a GateSite> {
    let entered = site(sites, here)?;
    if has_links(&entered.links) {
        return pick(&exits(&entered.links, sites), chance).copied();
    }
    let candidates: Vec<&GateSite> = sites
        .iter()
        .filter(|site| site.id != here && site.flags2 & WORMHOLE != 0)
        .filter(|site| site.system != current_system)
        .collect();
    let unlinked: Vec<&GateSite> = candidates
        .iter()
        .copied()
        .filter(|site| !has_links(&site.links))
        .collect();
    if unlinked.is_empty() {
        return None;
    }
    match rule {
        WormholeRule::Engine => {
            let picked = pick(&candidates, chance)?;
            Some(if has_links(&picked.links) {
                entered
            } else {
                picked
            })
        }
        WormholeRule::UnlinkedOnly => pick(&unlinked, chance).copied(),
    }
}

/// One of `choices`, rolled on `chance`; `None` when there are none.
fn pick<'c, T>(choices: &'c [T], chance: &mut (impl Chance + ?Sized)) -> Option<&'c T> {
    let sides = u16::try_from(choices.len()).unwrap_or(u16::MAX);
    if sides == 0 {
        return None;
    }
    choices.get(usize::from(chance.roll(sides)))
}

/// A ship of top speed `max_speed` coming out of `site`: at its centre,
/// heading its exit angle when that is 0 to 359 and otherwise one rolled on
/// `chance`, moving along that heading at [`EXIT_SPEED_FRACTION`] of its
/// top speed.
pub fn emerge(site: &GateSite, max_speed: f32, chance: &mut (impl Chance + ?Sized)) -> ShipState {
    let heading = match u16::try_from(site.exit_angle) {
        Ok(angle) if angle < 360 => angle,
        _ => chance.roll(360),
    };
    let heading = f32::from(heading);
    ShipState {
        position: site.position,
        velocity: facing(heading) * (EXIT_SPEED_FRACTION * max_speed),
        heading,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;
    use crate::testkit::{Scripted, star};

    /// The links in `ids`, in slots from the first, the rest unused.
    fn links(ids: &[i16]) -> [Option<StellarId>; 8] {
        let mut slots = [None; 8];
        for (slot, &id) in slots.iter_mut().zip(ids) {
            *slot = Some(StellarId(id));
        }
        slots
    }

    /// Stellar `id` in `system` at (`id`, 0), with `flags2` and links to
    /// `linked`, heading ships out on 90°.
    fn gate_site(id: i16, system: i16, flags2: u16, linked: &[i16]) -> GateSite {
        GateSite {
            id: StellarId(id),
            system: SystemId(system),
            position: Vec2::new(f32::from(id), 0.0),
            flags2,
            links: links(linked),
            exit_angle: 90,
        }
    }

    fn wormhole(id: i16, system: i16, linked: &[i16]) -> GateSite {
        gate_site(id, system, WORMHOLE, linked)
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
            gate_site(1400, 128, HYPERGATE, &[]),
            gate_site(1405, 129, HYPERGATE, &[]),
            gate_site(1413, 298, HYPERGATE, &[]),
            gate_site(1414, 298, HYPERGATE, &[]),
        ];
        let mut slots = links(&[1413, 999, 1405]);
        slots[4] = Some(StellarId(1414));
        slots[6] = Some(StellarId(1405));
        assert_eq!(
            hypergate_destinations(&slots, &sites),
            [SystemId(298), SystemId(129)],
            "an unlisted stellar skipped, and each system once"
        );
        assert_eq!(hypergate_destinations(&[None; 8], &sites), []);
    }

    #[test]
    fn a_hypergate_leads_to_the_first_linked_gate_in_the_system_picked() {
        let sites = [
            gate_site(1405, 129, HYPERGATE, &[]),
            gate_site(1413, 298, HYPERGATE, &[]),
            gate_site(1414, 298, HYPERGATE, &[]),
            gate_site(1418, 483, HYPERGATE, &[]),
        ];
        // 541 shares Dani's (298's) position; 483 sits apart.
        let map = StarMap::new(vec![
            star(129, (0.0, 0.0), &[]),
            star(298, (100.0, 100.0), &[]),
            star(483, (100.0, 0.0), &[]),
            star(541, (100.0, 100.0), &[]),
            star(600, (200.0, 0.0), &[]),
        ]);
        let slots = links(&[1405, 1414, 1413, 1418]);
        let exit = |chosen| hypergate_exit(&slots, &sites, SystemId(chosen), &map).map(|s| s.id.0);
        assert_eq!(exit(129), Some(1405));
        assert_eq!(exit(298), Some(1414), "the first slot there wins");
        assert_eq!(exit(541), Some(1414), "a system at its position");
        assert_eq!(exit(483), Some(1418));
        assert_eq!(exit(600), None, "no link leads there");
        assert_eq!(exit(999), None, "not on the map");
    }

    #[test]
    fn a_gate_off_the_map_matches_only_its_own_system() {
        let sites = [gate_site(1405, 700, HYPERGATE, &[])];
        let map = StarMap::new(vec![star(129, (0.0, 0.0), &[])]);
        let slots = links(&[1405]);
        assert_eq!(
            hypergate_exit(&slots, &sites, SystemId(700), &map).map(|s| s.id.0),
            Some(1405)
        );
        assert_eq!(hypergate_exit(&slots, &sites, SystemId(701), &map), None);
    }

    #[test]
    fn a_linked_wormhole_rolls_over_its_usable_slots() {
        let sites = [
            wormhole(500, 130, &[510, 999, 520, 510]),
            wormhole(510, 131, &[]),
            wormhole(520, 132, &[]),
        ];
        for (roll, expected) in [(0, 510), (1, 520), (2, 510)] {
            let mut chance = Scripted::rolling(&[roll]);
            let exit = wormhole_exit(
                StellarId(500),
                SystemId(130),
                &sites,
                WormholeRule::Engine,
                &mut chance,
            );
            assert_eq!(exit.map(|s| s.id.0), Some(expected), "{roll}");
            assert_eq!(chance.sides_asked, [3], "repeats count");
        }
    }

    #[test]
    fn a_linked_wormhole_whose_links_lead_nowhere_has_no_exit() {
        let sites = [wormhole(500, 130, &[999]), wormhole(510, 131, &[])];
        let mut chance = Scripted::default();
        let exit = wormhole_exit(
            StellarId(500),
            SystemId(130),
            &sites,
            WormholeRule::Engine,
            &mut chance,
        );
        assert_eq!(exit, None);
        assert!(chance.sides_asked.is_empty(), "nothing rolled");
    }

    /// An unlinked wormhole 500 in system 130, with another (505) and a
    /// hypergate (506) there too; elsewhere unlinked wormholes 510 and
    /// 530, linked one 520 between them, and a planet 540.
    fn unlinked_network() -> Vec<GateSite> {
        vec![
            wormhole(500, 130, &[]),
            wormhole(505, 130, &[]),
            gate_site(506, 130, HYPERGATE, &[]),
            wormhole(510, 131, &[]),
            wormhole(520, 132, &[510]),
            wormhole(530, 133, &[]),
            gate_site(540, 134, 0, &[]),
        ]
    }

    fn unlinked_exit(sites: &[GateSite], rule: WormholeRule, roll: u16) -> (Option<i16>, Vec<u16>) {
        let mut chance = Scripted::rolling(&[roll]);
        let exit = wormhole_exit(StellarId(500), SystemId(130), sites, rule, &mut chance);
        (exit.map(|site| site.id.0), chance.sides_asked)
    }

    #[test]
    fn by_the_engine_an_unlinked_wormhole_rolls_over_every_wormhole_elsewhere() {
        let sites = unlinked_network();
        let engine = |roll| unlinked_exit(&sites, WormholeRule::Engine, roll);
        assert_eq!(engine(0), (Some(510), vec![3]));
        assert_eq!(engine(2), (Some(530), vec![3]));
        assert_eq!(
            engine(1),
            (Some(500), vec![3]),
            "a linked pick leaves the ship where it entered"
        );
    }

    #[test]
    fn by_the_bible_an_unlinked_wormhole_rolls_over_the_unlinked_ones_only() {
        let sites = unlinked_network();
        let bible = |roll| unlinked_exit(&sites, WormholeRule::UnlinkedOnly, roll);
        assert_eq!(bible(0), (Some(510), vec![2]));
        assert_eq!(bible(1), (Some(530), vec![2]));
    }

    #[test]
    fn an_unlinked_wormhole_with_only_linked_ones_elsewhere_has_no_exit() {
        let sites = [
            wormhole(500, 130, &[]),
            wormhole(505, 130, &[]),
            wormhole(520, 132, &[500]),
        ];
        for rule in [WormholeRule::Engine, WormholeRule::UnlinkedOnly] {
            assert_eq!(unlinked_exit(&sites, rule, 0), (None, vec![]), "{rule:?}");
        }
    }

    #[test]
    fn a_wormhole_no_system_lists_has_no_exit() {
        let sites = [wormhole(510, 131, &[])];
        assert_eq!(
            unlinked_exit(&sites, WormholeRule::Engine, 0),
            (None, vec![])
        );
    }

    fn emerging(exit_angle: i16, rolls: &[u16]) -> (ShipState, Vec<u16>) {
        let site = GateSite {
            position: Vec2::new(-400.0, -500.0),
            exit_angle,
            ..gate_site(1405, 129, HYPERGATE, &[])
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
}
