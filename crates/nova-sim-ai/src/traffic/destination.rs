//! The stellars an NPC picks a destination among: the rules of
//! `_SelectRandomStellarDest` (@0x805de-0x80ad2 in the `EV Nova`
//! executable), shared by the idle block's pick
//! ([`ai::idle`](crate::ai::idle)) and the emerge pick
//! ([`emerge`](super::emerge)).
//!
//! Evidence: disassembly of the Mac OS X `EV Nova` (i386, with symbols),
//! counting the ship as argument 0.
//!
//! - **The calls.** `_SelectRandomStellarDest(ship, arg1, arg2)` takes
//!   arg1 as a byte at -0xa9 and arg2 at -0xaa (@0x805ea-0x805fb).
//!   `_PickEmergeStellar` calls it with (ship, `Rand(3) == 0`, 0) (@0x7fc7);
//!   the idle blocks with (ship, 0, 0) (`_WimpyTraderAI` @0x8b0ea,
//!   `_BraveTraderAI` @0x8b554, `_WarshipAI` @0x8b906, @0x8ba4a and
//!   @0x8c025, `_PirateWarshipAI` @0x8c490); and `_InterceptorAI`'s
//!   patrol with (ship, 0, 1) (@0x8d027-0x8d03a).
//! - **Each stellar.** One is *ok* when it is landable
//!   (`_StellarIsLandable` @0x8074e, [`is_landable`]: a destroyed one only
//!   when only landable destroyed, `Flags` 0x80, an intact one only
//!   without it), and its `xPos` and `yPos` are each at most 999, signed,
//!   with no lower bound (@0x80781-0x80797). It is an *enemy's* when both
//!   the ship and the stellar have a government and they are enemies
//!   (@0x807e2-0x80816, [`Governments::npc_enemies`]). It is
//!   *uninhabited* with `Flags` 0x20 and no gate bit (`Flags2` 0x3000,
//!   @0x807c8-0x807dd). A hypergate has `Flags2` 0x1000, a wormhole 0x2000
//!   (@0x8084c-0x80887); a *plain* stellar has neither.
//! - **The switches** are the ship's government's `Flags2`
//!   (@0x80601-0x8065d): [`AVOIDS_HYPERGATES`], [`PREFERS_HYPERGATES`] and
//!   [`PREFERS_WORMHOLES`]. An independent ship has none.
//! - **The branches**, among the ok stellars that are no enemy's:
//!   - arg2 set, *plain only* ([`Admit::PlainOnly`], @0x808fa-0x80942):
//!     none unless one is plain and inhabited; then every plain one. That
//!     pick does not test for an uninhabited stellar (@0x80911-0x80940),
//!     so it can land on one though the count needed an inhabited one
//!     (@0x808b6), as
//!     [`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)
//!     says. It reads no switch.
//!   - with [`PREFERS_WORMHOLES`] and an ok wormhole, among the ok
//!     wormholes (@0x80947-0x80984);
//!   - else with [`PREFERS_HYPERGATES`] and an ok hypergate, among the ok
//!     hypergates (@0x80989-0x809c6);
//!   - else among those that are not uninhabited, leaving out wormholes
//!     without [`PREFERS_WORMHOLES`] and hypergates with
//!     [`AVOIDS_HYPERGATES`] (@0x809cb-0x80a2e, @0x80a30-0x80a9f).
//!
//!   Each is a `Rand(16)` rejection loop over the slots: a uniform pick
//!   among the candidates, which its caller makes with one `below` draw in
//!   the system's stellar order. It returns the slot's stellar, or -1 with
//!   none (@0x80aa1-0x80ac8).
//! - **arg1 changes nothing**, so it is no parameter here. Its branch
//!   (@0x809cb) runs only without [`AVOIDS_HYPERGATES`], and then its test
//!   is the default branch's. Its candidates are too: without
//!   [`PREFERS_WORMHOLES`] both leave out wormholes, and with it the
//!   wormhole branch has already taken any ok wormhole. Only arg2 changes
//!   the pick: no argument admits hypergates, the default branch admits
//!   them unless the government avoids them, and arg2 leaves out every
//!   gate.
//!
//! `_SelectNearestAnyStellar` (@0x7fb00-0x7fc7b), the miner's shuttle pick
//! ([`select_nearest_any_stellar`]), shares only the enemy test: it walks
//! the 16 nav slots, skips an empty one, an enemy's stellar
//! (@0x7fb46-0x7fb95), a gate (in-memory `spöb` +0x34, `Flags2`, & 0x3000,
//! @0x7fbc4-0x7fbcb) and the stellar it is told to leave out (@0x7fbdb),
//! and takes the least squared distance from the ship, the first on a tie
//! (@0x7fc16-0x7fc34). It tests no landability, destruction, distance from
//! the centre or habitation.
//!
//! Left out, of no effect here:
//!
//! - A stellar listed in two nav slots weighs double in the `Rand(16)`
//!   loop (@0x806c1-0x808f4). The engine's sites are distinct stellars.

use crate::catalog::{GovtId, LandingSite, StellarId};
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::govt::{AVOIDS_HYPERGATES, Governments, PREFERS_HYPERGATES, PREFERS_WORMHOLES};
use crate::landing::{StellarFlags, is_landable};

/// How far a stellar may be from the centre, on each axis, to be picked
/// (999, @0x80781-0x8078f): an upper bound only.
pub const MAX_COORDINATE: f32 = 999.0;

/// Which stellars a pick admits: `_SelectRandomStellarDest`'s arg2 (see
/// the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Admit {
    /// arg2 0: by the government's switches, gates included.
    #[default]
    All,
    /// arg2 1: plain stellars only, never a gate, and none unless one is
    /// inhabited.
    PlainOnly {
        /// Whether an uninhabited one may be picked
        /// ([`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)'s
        /// engine reading).
        uninhabited: bool,
    },
}

/// The stellars among `sites` that `_SelectRandomStellarDest` picks among
/// for a ship of `govt`, admitting `admit`, in order (see the module
/// docs).
#[must_use]
pub fn candidates<'s>(
    govt: Option<GovtId>,
    sites: &'s [LandingSite],
    govts: &Governments,
    admit: Admit,
) -> Vec<&'s LandingSite> {
    let ok: Vec<&LandingSite> = sites
        .iter()
        .filter(|site| {
            is_landable(site)
                && site.position.x <= MAX_COORDINATE
                && site.position.y <= MAX_COORDINATE
                && !govts.npc_enemies(govt, site.govt)
        })
        .collect();
    let inhabited = |site: &LandingSite| site.flags & StellarFlags::UNINHABITED == 0;
    if let Admit::PlainOnly { uninhabited } = admit {
        let plain: Vec<&LandingSite> = ok
            .into_iter()
            .filter(|site| GateKind::of(site.flags2).is_none())
            .collect();
        if !plain.iter().any(|site| inhabited(site)) {
            return Vec::new();
        }
        return plain
            .into_iter()
            .filter(|site| uninhabited || inhabited(site))
            .collect();
    }
    let of_kind = |kind| -> Vec<&'s LandingSite> {
        ok.iter()
            .copied()
            .filter(|site| GateKind::of(site.flags2) == Some(kind))
            .collect()
    };
    let wormholes = govts.flag2(govt, PREFERS_WORMHOLES);
    if wormholes {
        let found = of_kind(GateKind::Wormhole);
        if !found.is_empty() {
            return found;
        }
    }
    if govts.flag2(govt, PREFERS_HYPERGATES) {
        let found = of_kind(GateKind::Hypergate);
        if !found.is_empty() {
            return found;
        }
    }
    let no_hypergates = govts.flag2(govt, AVOIDS_HYPERGATES);
    ok.into_iter()
        .filter(|site| match GateKind::of(site.flags2) {
            None => inhabited(site),
            Some(GateKind::Wormhole) => wormholes,
            Some(GateKind::Hypergate) => !no_hypergates,
        })
        .collect()
}

/// The stellar among `sites` nearest `at`, as `_SelectNearestAnyStellar`
/// picks it for a ship of `govt` (see the module docs): no enemy's, no
/// hypergate or wormhole, and not `exclude`; landable or not, destroyed
/// or not, anywhere. The first of `sites` wins a tie. None when no
/// stellar is left.
#[must_use]
pub fn select_nearest_any_stellar(
    at: Vec2,
    govt: Option<GovtId>,
    sites: &[LandingSite],
    govts: &Governments,
    exclude: Option<StellarId>,
) -> Option<StellarId> {
    let mut nearest: Option<(f32, StellarId)> = None;
    for site in sites {
        if govts.npc_enemies(govt, site.govt)
            || GateKind::of(site.flags2).is_some()
            || Some(site.id) == exclude
        {
            continue;
        }
        let off = site.position - at;
        let distance = off.x * off.x + off.y * off.y;
        if nearest.is_none_or(|(least, _)| distance < least) {
            nearest = Some((distance, site.id));
        }
    }
    nearest.map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use nova_sim_catalog::fixture::{govt, planet};

    const SHIP: Option<GovtId> = Some(GovtId(128));
    const FOE: Option<GovtId> = Some(GovtId(129));

    /// Government 128 (class 1) at war with 129 (class 2). 140 prefers
    /// hypergates, 141 wormholes, 142 avoids hypergates, 143 prefers both.
    fn govts() -> Governments {
        let with = |id, flags2| GovtRecord { flags2, ..govt(id) };
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                enemies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
            with(140, PREFERS_HYPERGATES),
            with(141, PREFERS_WORMHOLES),
            with(142, AVOIDS_HYPERGATES),
            with(143, PREFERS_HYPERGATES | PREFERS_WORMHOLES),
        ])
    }

    fn hypergate(id: i16) -> LandingSite {
        LandingSite {
            flags2: HYPERGATE,
            ..planet(id, 300.0, 0.0)
        }
    }

    fn wormhole(id: i16) -> LandingSite {
        LandingSite {
            flags2: WORMHOLE,
            ..planet(id, -300.0, 0.0)
        }
    }

    fn uninhabited(site: LandingSite) -> LandingSite {
        LandingSite {
            flags: site.flags | StellarFlags::UNINHABITED,
            ..site
        }
    }

    /// The IDs of the candidates among `sites` for a ship of `govt`.
    fn ids(govt: Option<GovtId>, sites: &[LandingSite], admit: Admit) -> Vec<i16> {
        candidates(govt, sites, &govts(), admit)
            .into_iter()
            .map(|site| site.id.0)
            .collect()
    }

    const PLAIN: Admit = Admit::PlainOnly { uninhabited: true };
    const INHABITED: Admit = Admit::PlainOnly { uninhabited: false };

    #[test]
    fn a_stellar_beyond_999_on_either_axis_is_out_but_none_is_too_low() {
        let at = |id, x, y| planet(id, x, y);
        let sites = [
            at(128, 1000.0, 0.0),
            at(129, 0.0, 1000.0),
            at(130, 999.0, 999.0),
            at(131, -5000.0, -5000.0),
        ];
        for admit in [Admit::All, PLAIN] {
            assert_eq!(ids(None, &sites, admit), [130, 131], "{admit:?}");
        }
    }

    #[test]
    fn an_uninhabited_planet_is_out_but_an_uninhabited_hypergate_is_kept() {
        let sites = [
            uninhabited(planet(128, 0.0, 0.0)),
            uninhabited(hypergate(129)),
            planet(130, 0.0, 0.0),
        ];
        assert_eq!(ids(None, &sites, Admit::All), [129, 130]);
    }

    #[test]
    fn an_enemys_stellar_is_out_but_only_between_two_governments() {
        let of = |govt, site| LandingSite { govt, ..site };
        let sites = [
            of(FOE, planet(128, 0.0, 0.0)),
            of(None, planet(129, 0.0, 0.0)),
            of(SHIP, planet(130, 0.0, 0.0)),
        ];
        assert_eq!(ids(SHIP, &sites, Admit::All), [129, 130]);
        assert_eq!(ids(SHIP, &sites, PLAIN), [129, 130]);
        assert_eq!(
            ids(None, &sites, Admit::All),
            [128, 129, 130],
            "an independent ship"
        );
        assert_eq!(ids(FOE, &sites, Admit::All), [128, 129]);
    }

    #[test]
    fn a_governments_switches_choose_among_the_gates() {
        let sites = [planet(128, 0.0, 0.0), hypergate(129), wormhole(130)];
        let pick = |govt| ids(Some(GovtId(govt)), &sites, Admit::All);
        assert_eq!(pick(141), [130], "wormholes only");
        assert_eq!(pick(143), [130], "wormholes first");
        assert_eq!(pick(140), [129], "hypergates only");
        assert_eq!(pick(142), [128], "no hypergate, no wormhole");
        assert_eq!(pick(128), [128, 129], "the rest, no wormhole");
        assert_eq!(ids(None, &sites, Admit::All), [128, 129]);
        // With no ok gate of the kind preferred, the rest.
        let no_wormhole = [planet(128, 0.0, 0.0), hypergate(129)];
        assert_eq!(ids(Some(GovtId(141)), &no_wormhole, Admit::All), [128, 129]);
        let no_hypergate = [planet(128, 0.0, 0.0), wormhole(130)];
        assert_eq!(ids(Some(GovtId(140)), &no_hypergate, Admit::All), [128]);
    }

    #[test]
    fn plain_only_never_admits_a_gate_whatever_the_switches() {
        let sites = [planet(128, 0.0, 0.0), hypergate(129), wormhole(130)];
        for govt in [
            None,
            Some(GovtId(140)),
            Some(GovtId(141)),
            Some(GovtId(143)),
        ] {
            assert_eq!(ids(govt, &sites, PLAIN), [128], "{govt:?}");
            assert_eq!(ids(govt, &sites, INHABITED), [128], "{govt:?}");
        }
    }

    #[test]
    fn plain_only_needs_an_inhabited_plain_stellar() {
        let lone = [uninhabited(planet(128, 0.0, 0.0)), hypergate(129)];
        assert!(ids(None, &lone, PLAIN).is_empty());
        assert!(ids(None, &lone, INHABITED).is_empty());
        let both = [uninhabited(planet(128, 0.0, 0.0)), planet(129, 0.0, 0.0)];
        assert_eq!(ids(None, &both, PLAIN), [128, 129]);
        assert_eq!(ids(None, &both, INHABITED), [129]);
        let far = [planet(128, 2000.0, 0.0), uninhabited(planet(129, 0.0, 0.0))];
        assert!(
            ids(None, &far, PLAIN).is_empty(),
            "the inhabited one is out"
        );
    }

    /// The ID `_SelectNearestAnyStellar` picks among `sites` for a ship
    /// of `govt` at `at`, leaving out `exclude`.
    fn nearest(
        at: Vec2,
        govt: Option<GovtId>,
        sites: &[LandingSite],
        exclude: Option<i16>,
    ) -> Option<i16> {
        select_nearest_any_stellar(at, govt, sites, &govts(), exclude.map(StellarId)).map(|id| id.0)
    }

    #[test]
    fn the_nearest_any_stellar_is_the_least_distance_from_the_ship() {
        let sites = [
            planet(128, 500.0, 0.0),
            planet(129, 0.0, -200.0),
            planet(130, -300.0, 300.0),
        ];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &sites, None), Some(129));
        assert_eq!(
            nearest(Vec2::new(-300.0, 250.0), SHIP, &sites, None),
            Some(130)
        );
        assert_eq!(
            nearest(Vec2::new(5000.0, 0.0), None, &sites, None),
            Some(128)
        );
        let behind = [planet(128, 0.0, -100.0), planet(129, 0.0, 50.0)];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &behind, None), Some(129), "by y");
        let behind = [planet(128, -100.0, 0.0), planet(129, 50.0, 0.0)];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &behind, None), Some(129), "by x");
    }

    #[test]
    fn the_nearest_any_stellar_skips_an_enemys_a_gate_and_the_one_left_out() {
        let of = |govt, site| LandingSite { govt, ..site };
        let sites = [
            of(FOE, planet(128, 10.0, 0.0)),
            LandingSite {
                position: Vec2::new(20.0, 0.0),
                ..hypergate(129)
            },
            LandingSite {
                position: Vec2::new(30.0, 0.0),
                ..wormhole(130)
            },
            planet(131, 40.0, 0.0),
            planet(132, 50.0, 0.0),
        ];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &sites, None), Some(131));
        assert_eq!(nearest(Vec2::ZERO, SHIP, &sites, Some(131)), Some(132));
        assert_eq!(
            nearest(Vec2::ZERO, None, &sites, None),
            Some(128),
            "an independent ship has no enemies"
        );
        let unruled = [of(None, planet(128, 10.0, 0.0)), planet(131, 40.0, 0.0)];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &unruled, None), Some(128));
    }

    #[test]
    fn the_nearest_any_stellar_takes_one_unlandable_destroyed_uninhabited_or_far_off() {
        let unlandable = LandingSite {
            flags: 0,
            ..planet(128, 10.0, 0.0)
        };
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[unlandable], None), Some(128));
        let destroyed = LandingSite {
            destroyed: true,
            ..planet(129, 10.0, 0.0)
        };
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[destroyed], None), Some(129));
        let empty = uninhabited(planet(130, 10.0, 0.0));
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[empty], None), Some(130));
        let far = planet(131, 5000.0, 0.0);
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[far], None), Some(131));
    }

    #[test]
    fn the_nearest_any_stellar_breaks_a_tie_to_the_first_and_is_none_with_no_candidate() {
        let sites = [planet(128, 100.0, 0.0), planet(129, -100.0, 0.0)];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &sites, None), Some(128));
        let sites = [planet(129, -100.0, 0.0), planet(128, 100.0, 0.0)];
        assert_eq!(nearest(Vec2::ZERO, SHIP, &sites, None), Some(129));
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[], None), None);
        assert_eq!(nearest(Vec2::ZERO, SHIP, &[hypergate(129)], None), None);
        assert_eq!(
            nearest(Vec2::ZERO, SHIP, &[planet(128, 0.0, 0.0)], Some(128)),
            None
        );
    }
}
