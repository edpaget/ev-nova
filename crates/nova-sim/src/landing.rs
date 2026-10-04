//! Landing: what the land key does, whether the player's ship can land on
//! a stellar, and what the stellar offers once it has.
//!
//! The land key ([`land_or_select`]) first requests clearance, then
//! lands, as the EV Nova manual says: "select the stellar object you
//! intend to land on ... Then request landing clearance by pressing the L
//! key. You'll be given clearance to land, then press the L key again",
//! "or just press the L key to request clearance on the nearest planet in
//! the system" (and `STR#` 2002 #25: "to request landing clearance, then
//! hit it again to land"). So L, with the navigation target:
//!
//! - none, or one no longer in the system: selects the nearest stellar
//!   that [`is_landable`] ([`nearest_landable`]), station or planet, and
//!   gives its [`Clearance`]. It never lands, even with the ship at rest
//!   over it. With stellars but none landable it selects nothing and
//!   refuses with [`LandingRefusal::NotLandable`] for the nearest; with no
//!   stellars, [`LandingRefusal::NoStellars`].
//! - a stellar that cannot be landed on (only Tab selects one): refuses
//!   with [`LandingRefusal::NotLandable`], wherever the ship is, before
//!   any check of range or speed (`STR#` 2002 #84/#88/#90, "Your ship is
//!   unable to land on ... The planet's environment is too hostile.").
//! - a landable stellar, selected by Tab or an earlier L: lands on it if
//!   [`check_landing`] allows. A refusal keeps the target, and the player
//!   tries again (the manual: "If you're too far away, or if you're moving
//!   too fast ... your ship's computer will beep at you and you can try
//!   again").
//!
//! The ship lands on its target when it is over it, within its
//! [`landing_radius`], and no faster than [`LANDING_SPEED`].
//! [`check_landing`] gives the stellar, or the first [`LandingRefusal`]
//! that applies, in this order:
//!
//! 1. [`LandingRefusal::TooFar`]: the ship is not over the target.
//! 2. [`LandingRefusal::NotLandable`]: the stellar lacks the can-land flag,
//!    or can be landed on only once destroyed (nothing is destroyed yet).
//! 3. [`LandingRefusal::Denied`]: the pilot's legal record with the
//!    stellar's government, or its system's when it has none, is below the
//!    stellar's `MinStatus`, which the Bible says uninhabited stellars
//!    ignore.
//! 4. [`LandingRefusal::TooFast`]: the ship is moving faster than
//!    [`LANDING_SPEED`].
//!
//! Each refusal says whether the stellar is a station, which picks the
//! original's "dock at this station" or "land on this planet" wording.

use crate::catalog::{GovtId, LandingSite, StellarId};
use crate::flight::ShipState;
use crate::geometry::Vec2;

/// The fastest a ship can land, in pixels a tick (30 pixels a second): a
/// third of an average ship's top speed. An average engine brakes to it
/// from top speed in about 20 ticks, and at it a ship takes more than two
/// seconds to cross even a small stellar, so a landing ship is visibly
/// parked over the stellar, as Nova asks.
pub const LANDING_SPEED: f32 = 1.0;

/// The landing radius of a stellar with no sprite: half the 64-pixel
/// placeholder box the views draw for it, so it matches what is drawn.
pub const UNKNOWN_STELLAR_RADIUS: f32 = 32.0;

/// The `spöb` `Flags` bits landing reads (the Bible).
#[derive(Clone, Copy, Debug)]
pub struct StellarFlags;

impl StellarFlags {
    /// Can land or dock here.
    pub const CAN_LAND: u32 = 0x01;
    /// Has a commodity exchange.
    pub const TRADE_CENTER: u32 = 0x02;
    /// Has an outfitter.
    pub const OUTFITTER: u32 = 0x04;
    /// Has a shipyard.
    pub const SHIPYARD: u32 = 0x08;
    /// Is a station, not a planet.
    pub const STATION: u32 = 0x10;
    /// Is uninhabited: no services, and `MinStatus` is ignored.
    pub const UNINHABITED: u32 = 0x20;
    /// Has a bar.
    pub const BAR: u32 = 0x40;
    /// Can be landed on only once it has been destroyed.
    pub const ONLY_WHEN_DESTROYED: u32 = 0x80;
}

/// Why the ship cannot land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LandingRefusal {
    /// The ship is jumping through hyperspace. The
    /// [`Session`](crate::Session) gives this before [`check_landing`] is
    /// asked.
    Jumping,
    /// The system has no stellars.
    NoStellars,
    /// The ship is not over the stellar it would land on.
    TooFar {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
    },
    /// The stellar cannot be landed on.
    NotLandable {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
    },
    /// The pilot's legal record is too low.
    Denied {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
        /// Its `MinStatus`.
        min_status: i16,
    },
    /// The ship is moving too fast.
    TooFast {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
        /// The ship's speed, in pixels a tick.
        speed: f32,
    },
}

/// How close the ship's centre must be to `site`'s centre to be over it:
/// half the larger side of its sprite's frame, which is what the player
/// sees as its disc, or [`UNKNOWN_STELLAR_RADIUS`] without a sprite.
#[must_use]
pub fn landing_radius(site: &LandingSite) -> f32 {
    site.frame_size
        .map_or(UNKNOWN_STELLAR_RADIUS, |(width, height)| {
            width.max(height) as f32 / 2.0
        })
}

/// The reply to a request for clearance, the land key's first press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clearance {
    /// Traffic control (a station's dockmaster) clears the ship.
    Granted,
    /// The stellar is uninhabited, so no one answers ("no traffic
    /// control", says the Bible), and the ship is cleared.
    NoTrafficControl,
    /// The pilot's legal record is below the stellar's `MinStatus`.
    Denied,
}

/// What the land key did, when it was not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandOutcome {
    /// It selected the stellar as the navigation target and requested
    /// clearance there; it did not land.
    Selected {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
        /// The reply.
        clearance: Clearance,
    },
    /// It landed on the stellar.
    Landed(StellarId),
}

/// What the land key does with the navigation target `target`, for the
/// ship `player` among the system's stellars `sites` (in navigation
/// order), in a system governed by `system_govt`, where `record` gives the
/// pilot's legal record with a government: see the module docs.
pub fn land_or_select(
    target: Option<StellarId>,
    player: &ShipState,
    sites: &[LandingSite],
    system_govt: Option<GovtId>,
    record: impl Fn(GovtId) -> i16,
) -> Result<LandOutcome, LandingRefusal> {
    let target = target.and_then(|id| sites.iter().find(|site| site.id == id));
    if let Some(site) = target {
        if !is_landable(site) {
            return Err(not_landable(site));
        }
        return check_landing(player, site, system_govt, record).map(LandOutcome::Landed);
    }
    let Some(site) = nearest_landable_site(player.position, sites) else {
        return Err(
            nearest(player.position, sites.iter()).map_or(LandingRefusal::NoStellars, not_landable)
        );
    };
    let clearance = if has(site, StellarFlags::UNINHABITED) {
        Clearance::NoTrafficControl
    } else if landing_record(site, system_govt, record) < site.min_status {
        Clearance::Denied
    } else {
        Clearance::Granted
    };
    Ok(LandOutcome::Selected {
        stellar: site.id,
        station: is_station(site),
        clearance,
    })
}

/// Whether `site` can be landed on at all, by its flags alone: it has the
/// can-land flag and is not landable only once destroyed. The pilot's
/// record does not count, so a stellar that would deny clearance is
/// landable.
#[must_use]
pub fn is_landable(site: &LandingSite) -> bool {
    has(site, StellarFlags::CAN_LAND) && !has(site, StellarFlags::ONLY_WHEN_DESTROYED)
}

/// The landable stellar among `sites` whose centre is nearest `position`,
/// the earlier in `sites` of any equally near; `None` when none is
/// landable.
#[must_use]
pub fn nearest_landable(position: Vec2, sites: &[LandingSite]) -> Option<StellarId> {
    nearest_landable_site(position, sites).map(|site| site.id)
}

/// The site [`nearest_landable`] names: the one the land key selects when
/// there is no target.
fn nearest_landable_site(position: Vec2, sites: &[LandingSite]) -> Option<&LandingSite> {
    nearest(position, sites.iter().filter(|site| is_landable(site)))
}

/// The one of `sites` whose centre is nearest `position`, the first of
/// any equally near.
fn nearest<'a>(
    position: Vec2,
    sites: impl Iterator<Item = &'a LandingSite>,
) -> Option<&'a LandingSite> {
    let distance = |site: &LandingSite| (site.position - position).length();
    sites.min_by(|a, b| distance(a).total_cmp(&distance(b)))
}

fn not_landable(site: &LandingSite) -> LandingRefusal {
    LandingRefusal::NotLandable {
        stellar: site.id,
        station: is_station(site),
    }
}

fn has(site: &LandingSite, flag: u32) -> bool {
    site.flags & flag != 0
}

/// Whether `player` lands on `site`, in a system governed by
/// `system_govt`: the stellar, or the first refusal that applies (see the
/// module docs). `record` gives the pilot's legal record with a
/// government; see [`landing_record`] for which one landing reads.
pub fn check_landing(
    player: &ShipState,
    site: &LandingSite,
    system_govt: Option<GovtId>,
    record: impl Fn(GovtId) -> i16,
) -> Result<StellarId, LandingRefusal> {
    let (stellar, station) = (site.id, is_station(site));
    if (site.position - player.position).length() > landing_radius(site) {
        return Err(LandingRefusal::TooFar { stellar, station });
    }
    if !is_landable(site) {
        return Err(not_landable(site));
    }
    if !has(site, StellarFlags::UNINHABITED)
        && landing_record(site, system_govt, record) < site.min_status
    {
        return Err(LandingRefusal::Denied {
            stellar,
            station,
            min_status: site.min_status,
        });
    }
    let speed = player.velocity.length();
    if speed > LANDING_SPEED {
        return Err(LandingRefusal::TooFast {
            stellar,
            station,
            speed,
        });
    }
    Ok(stellar)
}

/// The legal record that `site`'s `MinStatus` is checked against, in a
/// system governed by `system_govt`, where `record` gives the pilot's
/// record with a government.
///
/// The Bible's `spöb` `MinStatus` is "the point on your record in the
/// current system" below which you are denied landing clearance, and the
/// `chär` `Status1-4` are "the player's legal status in systems owned by
/// that government". The `spöb` `Govt` is "the stellar's government" and
/// the `sÿst` `Govt` "the controlling govt" (-1, independent, is `None` in
/// both). The Bible does not say which of the two the record belongs to
/// when they differ, so two decisions:
///
/// - The stellar's own `Govt` wins, and a stellar without one falls back
///   to its system's (68 stock stellars, such as Spacedock VI, differ
///   from their system's government; 58 others have none of their own).
/// - With no government on either, the record is 0, a clean record. The
///   Bible's "independent counts as govt 128" rule scales the displayed
///   status label, not a record, so it does not apply here.
///
/// Out of scope here: `chär` status spreading to allies and enemies, the
/// `gövt` `InitialRec`, and the rank flag that grants landing regardless
/// of `MinStatus`.
fn landing_record(
    site: &LandingSite,
    system_govt: Option<GovtId>,
    record: impl Fn(GovtId) -> i16,
) -> i16 {
    site.govt.or(system_govt).map_or(0, record)
}

fn is_station(site: &LandingSite) -> bool {
    has(site, StellarFlags::STATION)
}

/// A spaceport service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Service {
    /// The commodity exchange.
    TradeCenter,
    /// The outfitter.
    Outfitter,
    /// The shipyard.
    Shipyard,
    /// The bar.
    Bar,
    /// The mission computer.
    MissionBbs,
}

/// The services a stellar with `flags` offers, in [`Service`] order. The
/// trade center, outfitter, shipyard and bar each have their flag; there
/// is no flag for the mission BBS, which the original offers on every
/// inhabited stellar. An uninhabited stellar offers nothing.
#[must_use]
pub fn services(flags: u32) -> Vec<Service> {
    if flags & StellarFlags::UNINHABITED != 0 {
        return Vec::new();
    }
    [
        (StellarFlags::TRADE_CENTER, Service::TradeCenter),
        (StellarFlags::OUTFITTER, Service::Outfitter),
        (StellarFlags::SHIPYARD, Service::Shipyard),
        (StellarFlags::BAR, Service::Bar),
    ]
    .into_iter()
    .filter(|(flag, _)| flags & flag != 0)
    .map(|(_, service)| service)
    .chain([Service::MissionBbs])
    .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// A government for the system around the stellars whose record tests
    /// only care about the record itself.
    const GOVT: Option<GovtId> = Some(GovtId(128));

    /// A landable planet at (`x`, `y`), 100 x 60 (radius 50), anyone may
    /// land on.
    fn site(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((100, 60)),
            flags: StellarFlags::CAN_LAND,
            min_status: -32767,
            landing_sound: None,
            tech_level: 0,
            special_tech: [0; 8],
            govt: None,
        }
    }

    fn with_flags(flags: u32) -> LandingSite {
        LandingSite {
            flags,
            ..site(128, 0.0, 0.0)
        }
    }

    /// A ship at (`x`, `y`) moving at (`vx`, `vy`).
    fn ship(x: f32, y: f32, vx: f32, vy: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(vx, vy),
            ..ShipState::default()
        }
    }

    fn parked() -> ShipState {
        ship(0.0, 0.0, 0.0, 0.0)
    }

    #[test]
    fn the_named_values() {
        assert_eq!(LANDING_SPEED, 1.0);
        assert_eq!(UNKNOWN_STELLAR_RADIUS, 32.0);
        assert_eq!(
            [
                StellarFlags::CAN_LAND,
                StellarFlags::TRADE_CENTER,
                StellarFlags::OUTFITTER,
                StellarFlags::SHIPYARD,
                StellarFlags::STATION,
                StellarFlags::UNINHABITED,
                StellarFlags::BAR,
                StellarFlags::ONLY_WHEN_DESTROYED,
            ],
            [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80]
        );
    }

    #[test]
    fn the_radius_is_half_the_larger_side_of_the_frame() {
        assert_eq!(landing_radius(&site(128, 0.0, 0.0)), 50.0);
        let tall = LandingSite {
            frame_size: Some((140, 185)),
            ..site(128, 0.0, 0.0)
        };
        assert_eq!(landing_radius(&tall), 92.5);
        let wide = LandingSite {
            frame_size: Some((140, 85)),
            ..site(128, 0.0, 0.0)
        };
        assert_eq!(landing_radius(&wide), 70.0);
        let unknown = LandingSite {
            frame_size: None,
            ..site(128, 0.0, 0.0)
        };
        assert_eq!(landing_radius(&unknown), UNKNOWN_STELLAR_RADIUS);
    }

    #[test]
    fn landable_means_can_land_and_not_only_once_destroyed() {
        use StellarFlags as F;
        for (flags, landable) in [
            (0, false),
            (F::STATION, false),
            (F::CAN_LAND, true),
            (F::CAN_LAND | F::STATION, true),
            (F::CAN_LAND | F::UNINHABITED, true),
            (F::CAN_LAND | F::ONLY_WHEN_DESTROYED, false),
            (F::ONLY_WHEN_DESTROYED, false),
        ] {
            assert_eq!(is_landable(&with_flags(flags)), landable, "{flags:#x}");
        }
        // The record does not count: a stellar that would deny is landable.
        let strict = LandingSite {
            min_status: 32767,
            ..with_flags(F::CAN_LAND)
        };
        assert!(is_landable(&strict));
    }

    #[test]
    fn the_nearest_landable_stellar_skips_nearer_ones_that_cannot_be_landed_on() {
        let sites = [
            site(128, 500.0, 0.0),
            LandingSite {
                flags: 0,
                ..site(129, 0.0, -10.0)
            },
            LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
                ..site(130, 0.0, 20.0)
            },
            LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
                ..site(131, -300.0, 0.0)
            },
        ];
        assert_eq!(
            nearest_landable(Vec2::ZERO, &sites),
            Some(StellarId(131)),
            "a station counts"
        );
        assert_eq!(
            nearest_landable(Vec2::new(450.0, 0.0), &sites),
            Some(StellarId(128))
        );
    }

    #[test]
    fn with_none_landable_there_is_no_nearest_landable_stellar() {
        assert_eq!(nearest_landable(Vec2::ZERO, &[]), None);
        assert_eq!(nearest_landable(Vec2::ZERO, &[with_flags(0)]), None);
    }

    #[test]
    fn equally_near_stellars_go_to_the_earlier_in_nav_order() {
        let sites = [site(131, 0.0, 100.0), site(128, 0.0, -100.0)];
        assert_eq!(nearest_landable(Vec2::ZERO, &sites), Some(StellarId(131)));
        let sites = [site(128, 0.0, -100.0), site(131, 0.0, 100.0)];
        assert_eq!(nearest_landable(Vec2::ZERO, &sites), Some(StellarId(128)));
    }

    #[test]
    fn a_slow_ship_over_a_landable_stellar_lands_on_it() {
        let target = site(128, 10.0, -20.0);
        assert_eq!(
            check_landing(&ship(5.0, -15.0, 0.5, -0.5), &target, None, |_| 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn too_far_names_the_target() {
        assert_eq!(
            check_landing(&parked(), &site(129, 0.0, -300.0), None, |_| 0),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
        let station = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
            ..site(131, 0.0, 51.0)
        };
        assert_eq!(
            check_landing(&parked(), &station, None, |_| 0),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(131),
                station: true,
            })
        );
    }

    #[test]
    fn exactly_at_the_radius_is_over_it_and_just_beyond_is_not() {
        let target = site(128, 0.0, 0.0);
        assert_eq!(
            check_landing(&ship(30.0, 40.0, 0.0, 0.0), &target, None, |_| 0),
            Ok(StellarId(128))
        );
        assert!(matches!(
            check_landing(&ship(30.0, 40.01, 0.0, 0.0), &target, None, |_| 0),
            Err(LandingRefusal::TooFar { .. })
        ));
    }

    #[test]
    fn a_ship_over_the_target_lands_on_it_even_where_stellars_overlap() {
        // 129's centre is nearer, but 128 is the target and the ship is
        // within its radius too.
        let target = site(128, 40.0, 0.0);
        assert_eq!(
            check_landing(&parked(), &target, None, |_| 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn a_stellar_without_the_can_land_flag_is_not_landable() {
        for (flags, station) in [(0, false), (StellarFlags::STATION, true)] {
            assert_eq!(
                check_landing(&parked(), &with_flags(flags), None, |_| 0),
                Err(LandingRefusal::NotLandable {
                    stellar: StellarId(128),
                    station,
                })
            );
        }
    }

    #[test]
    fn a_stellar_landable_only_once_destroyed_is_not_landable() {
        let flags = StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED;
        assert_eq!(
            check_landing(&parked(), &with_flags(flags), None, |_| 0),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
    }

    fn needing(min_status: i16, flags: u32) -> LandingSite {
        LandingSite {
            flags,
            min_status,
            ..site(128, 0.0, 0.0)
        }
    }

    #[test]
    fn a_record_below_min_status_is_denied() {
        let can_land = StellarFlags::CAN_LAND;
        for min_status in [1, 32767] {
            assert_eq!(
                check_landing(&parked(), &needing(min_status, can_land), GOVT, |_| 0),
                Err(LandingRefusal::Denied {
                    stellar: StellarId(128),
                    station: false,
                    min_status,
                })
            );
        }
        let station = can_land | StellarFlags::STATION;
        assert_eq!(
            check_landing(&parked(), &needing(5, station), GOVT, |_| 4),
            Err(LandingRefusal::Denied {
                stellar: StellarId(128),
                station: true,
                min_status: 5,
            })
        );
    }

    #[test]
    fn a_record_at_or_above_min_status_lands() {
        let can_land = StellarFlags::CAN_LAND;
        for (min_status, record) in [(-32767, 0), (0, 0), (5, 5), (5, 6), (-3, -3)] {
            assert_eq!(
                check_landing(&parked(), &needing(min_status, can_land), GOVT, |_| record),
                Ok(StellarId(128)),
                "{min_status} {record}"
            );
        }
    }

    #[test]
    fn an_uninhabited_stellar_ignores_min_status() {
        let flags = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        assert_eq!(
            check_landing(&parked(), &needing(32767, flags), None, |_| 0),
            Ok(StellarId(128))
        );
        // Even under a government whose record of the pilot is far below.
        let governed = LandingSite {
            govt: Some(A),
            ..needing(32767, flags)
        };
        assert_eq!(
            check_landing(&parked(), &governed, Some(B), |_| -32768),
            Ok(StellarId(128))
        );
    }

    /// Two governments, and a record of the pilot that is 10 with `A` and
    /// 9 with `B`, either side of a `MinStatus` of 10.
    const A: GovtId = GovtId(140);
    const B: GovtId = GovtId(150);

    fn record(govt: GovtId) -> i16 {
        match govt {
            A => 10,
            B => 9,
            _ => panic!("no record asked of {govt:?}"),
        }
    }

    fn governed_by(govt: Option<GovtId>) -> LandingSite {
        LandingSite {
            govt,
            ..needing(10, StellarFlags::CAN_LAND)
        }
    }

    fn denied() -> Result<StellarId, LandingRefusal> {
        Err(LandingRefusal::Denied {
            stellar: StellarId(128),
            station: false,
            min_status: 10,
        })
    }

    #[test]
    fn a_stellars_own_government_keeps_the_record_landing_reads() {
        let land = |site, system| check_landing(&parked(), &governed_by(site), system, record);
        assert_eq!(land(Some(A), Some(B)), Ok(StellarId(128)));
        assert_eq!(land(Some(B), Some(A)), denied());
        assert_eq!(land(Some(A), None), Ok(StellarId(128)));
        assert_eq!(land(Some(B), None), denied());
    }

    #[test]
    fn a_stellar_without_a_government_falls_back_to_its_systems() {
        let land = |system| check_landing(&parked(), &governed_by(None), system, record);
        assert_eq!(land(Some(A)), Ok(StellarId(128)));
        assert_eq!(land(Some(B)), denied());
    }

    #[test]
    fn with_no_government_anywhere_the_record_is_0() {
        let never = |govt| panic!("no record asked of {govt:?}");
        let needs = |min_status| LandingSite {
            govt: None,
            ..needing(min_status, StellarFlags::CAN_LAND)
        };
        assert_eq!(
            check_landing(&parked(), &needs(0), None, never),
            Ok(StellarId(128))
        );
        assert_eq!(
            check_landing(&parked(), &needs(1), None, never),
            Err(LandingRefusal::Denied {
                stellar: StellarId(128),
                station: false,
                min_status: 1,
            })
        );
    }

    #[test]
    fn a_ship_faster_than_the_landing_speed_is_too_fast() {
        let target = site(128, 0.0, 0.0);
        let fast = ship(0.0, 0.0, 0.6, 0.81);
        let speed = fast.velocity.length();
        assert!(speed > LANDING_SPEED, "{speed}");
        assert_eq!(
            check_landing(&fast, &target, None, |_| 0),
            Err(LandingRefusal::TooFast {
                stellar: StellarId(128),
                station: false,
                speed,
            })
        );
        let station = with_flags(StellarFlags::CAN_LAND | StellarFlags::STATION);
        assert!(matches!(
            check_landing(&fast, &station, None, |_| 0),
            Err(LandingRefusal::TooFast { station: true, .. })
        ));
    }

    #[test]
    fn exactly_the_landing_speed_lands() {
        let target = site(128, 0.0, 0.0);
        assert_eq!(
            check_landing(&ship(0.0, 0.0, 0.0, 1.0), &target, None, |_| 0),
            Ok(StellarId(128))
        );
        assert_eq!(
            check_landing(&ship(0.0, 0.0, -1.0, 0.0), &target, None, |_| 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn each_refusal_applies_before_the_next() {
        // Not landable, denied and too fast at once: not landable.
        let fast = ship(0.0, 0.0, 5.0, 0.0);
        assert!(matches!(
            check_landing(&fast, &needing(10, 0), None, |_| 0),
            Err(LandingRefusal::NotLandable { .. })
        ));
        // Denied and too fast: denied.
        assert!(matches!(
            check_landing(&fast, &needing(10, StellarFlags::CAN_LAND), None, |_| 0),
            Err(LandingRefusal::Denied { .. })
        ));
        // Too far comes first of all.
        let far = ship(0.0, 500.0, 5.0, 0.0);
        assert!(matches!(
            check_landing(&far, &needing(10, 0), None, |_| 0),
            Err(LandingRefusal::TooFar { .. })
        ));
    }

    /// Planet 128 at the centre, and planet 129 and station 130 farther
    /// out: what L reads in most of the rule's tests.
    fn system() -> [LandingSite; 3] {
        [
            site(129, 0.0, -300.0),
            site(128, 0.0, 0.0),
            LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
                ..site(130, 400.0, 0.0)
            },
        ]
    }

    /// L pressed with `target` selected, for a ship `player` among
    /// `sites`, with a clean record and no government.
    fn press(
        target: Option<i16>,
        player: &ShipState,
        sites: &[LandingSite],
    ) -> Result<LandOutcome, LandingRefusal> {
        land_or_select(target.map(StellarId), player, sites, None, |_| 0)
    }

    fn selected(stellar: i16, station: bool, clearance: Clearance) -> LandOutcome {
        LandOutcome::Selected {
            stellar: StellarId(stellar),
            station,
            clearance,
        }
    }

    #[test]
    fn with_no_target_l_selects_the_nearest_landable_stellar_and_does_not_land() {
        // Even at rest over it.
        assert_eq!(
            press(None, &parked(), &system()),
            Ok(selected(128, false, Clearance::Granted))
        );
        assert_eq!(
            press(None, &ship(380.0, 0.0, 9.0, 0.0), &system()),
            Ok(selected(130, true, Clearance::Granted))
        );
    }

    #[test]
    fn a_target_no_longer_in_the_system_is_as_good_as_none() {
        assert_eq!(
            press(Some(140), &parked(), &system()),
            Ok(selected(128, false, Clearance::Granted))
        );
    }

    #[test]
    fn with_a_landable_target_l_lands_on_it_not_on_the_nearest() {
        let over_129 = ship(0.0, -280.0, 0.0, 0.5);
        assert_eq!(
            press(Some(129), &over_129, &system()),
            Ok(LandOutcome::Landed(StellarId(129)))
        );
        // Over 128, with 129 selected: too far from 129.
        assert_eq!(
            press(Some(129), &parked(), &system()),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
    }

    #[test]
    fn with_a_landable_target_too_fast_or_denied_refuses() {
        assert!(matches!(
            press(Some(128), &ship(0.0, 0.0, 3.0, 0.0), &system()),
            Err(LandingRefusal::TooFast {
                stellar: StellarId(128),
                ..
            })
        ));
        let strict = [needing(5, StellarFlags::CAN_LAND)];
        assert_eq!(
            land_or_select(Some(StellarId(128)), &parked(), &strict, GOVT, |_| 4),
            Err(LandingRefusal::Denied {
                stellar: StellarId(128),
                station: false,
                min_status: 5,
            })
        );
    }

    #[test]
    fn a_target_that_cannot_be_landed_on_refuses_before_range_or_speed() {
        let sites = [
            site(128, 0.0, 0.0),
            LandingSite {
                flags: StellarFlags::STATION,
                ..site(129, 5000.0, 5000.0)
            },
        ];
        // Far from it and fast, and over a landable one: still too hostile.
        let far_and_fast = ship(0.0, 0.0, 9.0, 0.0);
        assert_eq!(
            press(Some(129), &far_and_fast, &sites),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(129),
                station: true,
            })
        );
        let only_when_destroyed = [LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
            ..site(131, -900.0, 0.0)
        }];
        assert_eq!(
            press(Some(131), &parked(), &only_when_destroyed),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(131),
                station: false,
            })
        );
    }

    #[test]
    fn with_none_landable_l_says_so_of_the_nearest_and_selects_nothing() {
        let sites = [
            LandingSite {
                flags: 0,
                ..site(128, 0.0, 300.0)
            },
            LandingSite {
                flags: StellarFlags::STATION,
                ..site(129, 0.0, -200.0)
            },
        ];
        assert_eq!(
            press(None, &parked(), &sites),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(129),
                station: true,
            })
        );
    }

    #[test]
    fn with_no_stellars_l_refuses() {
        assert_eq!(press(None, &parked(), &[]), Err(LandingRefusal::NoStellars));
        assert_eq!(
            press(Some(128), &parked(), &[]),
            Err(LandingRefusal::NoStellars)
        );
    }

    #[test]
    fn the_clearance_given_follows_the_record_and_traffic_control() {
        let strict = [needing(5, StellarFlags::CAN_LAND)];
        let select = |record| land_or_select(None, &parked(), &strict, GOVT, move |_| record);
        assert_eq!(select(4), Ok(selected(128, false, Clearance::Denied)));
        assert_eq!(select(5), Ok(selected(128, false, Clearance::Granted)));
        let uninhabited = [needing(
            32767,
            StellarFlags::CAN_LAND | StellarFlags::UNINHABITED | StellarFlags::STATION,
        )];
        assert_eq!(
            land_or_select(None, &parked(), &uninhabited, GOVT, |_| -32768),
            Ok(selected(128, true, Clearance::NoTrafficControl))
        );
        // The record read is the stellar's government's.
        let governed = [LandingSite {
            govt: Some(B),
            ..needing(10, StellarFlags::CAN_LAND)
        }];
        assert_eq!(
            land_or_select(None, &parked(), &governed, Some(A), record),
            Ok(selected(128, false, Clearance::Denied))
        );
    }

    #[test]
    fn each_service_flag_offers_its_service_and_inhabited_stellars_a_bbs() {
        use Service::{Bar, MissionBbs, Outfitter, Shipyard, TradeCenter};
        assert_eq!(services(0), [MissionBbs]);
        assert_eq!(services(StellarFlags::CAN_LAND), [MissionBbs]);
        assert_eq!(
            services(StellarFlags::TRADE_CENTER),
            [TradeCenter, MissionBbs]
        );
        assert_eq!(services(StellarFlags::OUTFITTER), [Outfitter, MissionBbs]);
        assert_eq!(services(StellarFlags::SHIPYARD), [Shipyard, MissionBbs]);
        assert_eq!(services(StellarFlags::BAR), [Bar, MissionBbs]);
        assert_eq!(
            services(0xFF & !StellarFlags::UNINHABITED),
            [TradeCenter, Outfitter, Shipyard, Bar, MissionBbs]
        );
    }

    #[test]
    fn an_uninhabited_stellar_offers_nothing() {
        assert_eq!(services(StellarFlags::UNINHABITED), []);
        assert_eq!(services(0xFF), []);
    }
}
