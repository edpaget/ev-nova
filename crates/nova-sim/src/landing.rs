//! Landing: whether the player's ship can land on a stellar, and what the
//! stellar offers once it has.
//!
//! The ship lands on the stellar it is over, the one whose centre is
//! nearest among those within their [`landing_radius`], when it is no
//! faster than [`LANDING_SPEED`]. [`check_landing`] gives the stellar, or
//! the first [`LandingRefusal`] that applies, in this order:
//!
//! 1. [`LandingRefusal::NoStellars`]: the system has none.
//! 2. [`LandingRefusal::TooFar`]: the ship is over none; the reason names
//!    the nearest.
//! 3. [`LandingRefusal::NotLandable`]: the stellar lacks the can-land flag,
//!    or can be landed on only once destroyed (nothing is destroyed yet).
//! 4. [`LandingRefusal::Denied`]: the pilot's legal record is below the
//!    stellar's `MinStatus`, which the Bible says uninhabited stellars
//!    ignore.
//! 5. [`LandingRefusal::TooFast`]: the ship is moving faster than
//!    [`LANDING_SPEED`].
//!
//! Each refusal says whether the stellar is a station, which picks the
//! original's "dock at this station" or "land on this planet" wording.

use crate::catalog::{LandingSite, StellarId};
use crate::flight::ShipState;

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
    /// The ship is over no stellar.
    TooFar {
        /// The stellar whose centre is nearest.
        nearest: StellarId,
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

/// The stellar `player` lands on among `sites`, with the pilot's legal
/// `record`, or the first refusal that applies (see the module docs).
pub fn check_landing(
    player: &ShipState,
    sites: &[LandingSite],
    record: i16,
) -> Result<StellarId, LandingRefusal> {
    let distance = |site: &LandingSite| (site.position - player.position).length();
    let nearest = |candidates: &mut dyn Iterator<Item = &LandingSite>| {
        candidates
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
            .copied()
    };
    let Some(closest) = nearest(&mut sites.iter()) else {
        return Err(LandingRefusal::NoStellars);
    };
    let over = nearest(
        &mut sites
            .iter()
            .filter(|site| distance(site) <= landing_radius(site)),
    );
    let Some(site) = over else {
        return Err(LandingRefusal::TooFar {
            nearest: closest.id,
            station: is_station(&closest),
        });
    };
    let (stellar, station) = (site.id, is_station(&site));
    let has = |flag| site.flags & flag != 0;
    if !has(StellarFlags::CAN_LAND) || has(StellarFlags::ONLY_WHEN_DESTROYED) {
        return Err(LandingRefusal::NotLandable { stellar, station });
    }
    if !has(StellarFlags::UNINHABITED) && record < site.min_status {
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

fn is_station(site: &LandingSite) -> bool {
    site.flags & StellarFlags::STATION != 0
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
    fn a_slow_ship_over_a_landable_stellar_lands_on_it() {
        let sites = [site(128, 10.0, -20.0)];
        assert_eq!(
            check_landing(&ship(5.0, -15.0, 0.5, -0.5), &sites, 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn no_stellars_refuses() {
        assert_eq!(
            check_landing(&parked(), &[], 0),
            Err(LandingRefusal::NoStellars)
        );
    }

    #[test]
    fn too_far_names_the_nearest_stellar() {
        let sites = [
            site(128, 500.0, 0.0),
            site(129, 0.0, -300.0),
            site(130, -400.0, 0.0),
        ];
        assert_eq!(
            check_landing(&parked(), &sites, 0),
            Err(LandingRefusal::TooFar {
                nearest: StellarId(129),
                station: false,
            })
        );
        let station = [LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
            ..site(131, 0.0, 51.0)
        }];
        assert_eq!(
            check_landing(&parked(), &station, 0),
            Err(LandingRefusal::TooFar {
                nearest: StellarId(131),
                station: true,
            })
        );
    }

    #[test]
    fn exactly_at_the_radius_is_over_it_and_just_beyond_is_not() {
        let sites = [site(128, 0.0, 0.0)];
        assert_eq!(
            check_landing(&ship(30.0, 40.0, 0.0, 0.0), &sites, 0),
            Ok(StellarId(128))
        );
        assert!(matches!(
            check_landing(&ship(30.0, 40.01, 0.0, 0.0), &sites, 0),
            Err(LandingRefusal::TooFar { .. })
        ));
    }

    #[test]
    fn the_nearest_centre_wins_when_stellars_overlap() {
        let sites = [
            site(128, 40.0, 0.0),
            site(129, -20.0, 0.0),
            site(130, 0.0, 30.0),
        ];
        assert_eq!(check_landing(&parked(), &sites, 0), Ok(StellarId(129)));
        // The nearest wins even when it refuses and a farther one would not.
        let unlandable = LandingSite {
            flags: 0,
            ..site(129, 0.0, 0.0)
        };
        let sites = [site(128, 40.0, 0.0), unlandable];
        assert_eq!(
            check_landing(&parked(), &sites, 0),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(129),
                station: false,
            })
        );
    }

    #[test]
    fn a_stellar_without_the_can_land_flag_is_not_landable() {
        for (flags, station) in [(0, false), (StellarFlags::STATION, true)] {
            assert_eq!(
                check_landing(&parked(), &[with_flags(flags)], 0),
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
            check_landing(&parked(), &[with_flags(flags)], 0),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
    }

    fn needing(min_status: i16, flags: u32) -> [LandingSite; 1] {
        [LandingSite {
            flags,
            min_status,
            ..site(128, 0.0, 0.0)
        }]
    }

    #[test]
    fn a_record_below_min_status_is_denied() {
        let can_land = StellarFlags::CAN_LAND;
        for min_status in [1, 32767] {
            assert_eq!(
                check_landing(&parked(), &needing(min_status, can_land), 0),
                Err(LandingRefusal::Denied {
                    stellar: StellarId(128),
                    station: false,
                    min_status,
                })
            );
        }
        let station = can_land | StellarFlags::STATION;
        assert_eq!(
            check_landing(&parked(), &needing(5, station), 4),
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
                check_landing(&parked(), &needing(min_status, can_land), record),
                Ok(StellarId(128)),
                "{min_status} {record}"
            );
        }
    }

    #[test]
    fn an_uninhabited_stellar_ignores_min_status() {
        let flags = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        assert_eq!(
            check_landing(&parked(), &needing(32767, flags), 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn a_ship_faster_than_the_landing_speed_is_too_fast() {
        let sites = [site(128, 0.0, 0.0)];
        let fast = ship(0.0, 0.0, 0.6, 0.81);
        let speed = fast.velocity.length();
        assert!(speed > LANDING_SPEED, "{speed}");
        assert_eq!(
            check_landing(&fast, &sites, 0),
            Err(LandingRefusal::TooFast {
                stellar: StellarId(128),
                station: false,
                speed,
            })
        );
        let station = [with_flags(StellarFlags::CAN_LAND | StellarFlags::STATION)];
        assert!(matches!(
            check_landing(&fast, &station, 0),
            Err(LandingRefusal::TooFast { station: true, .. })
        ));
    }

    #[test]
    fn exactly_the_landing_speed_lands() {
        let sites = [site(128, 0.0, 0.0)];
        assert_eq!(
            check_landing(&ship(0.0, 0.0, 0.0, 1.0), &sites, 0),
            Ok(StellarId(128))
        );
        assert_eq!(
            check_landing(&ship(0.0, 0.0, -1.0, 0.0), &sites, 0),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn each_refusal_applies_before_the_next() {
        // Not landable, denied and too fast at once: not landable.
        let fast = ship(0.0, 0.0, 5.0, 0.0);
        assert!(matches!(
            check_landing(&fast, &needing(10, 0), 0),
            Err(LandingRefusal::NotLandable { .. })
        ));
        // Denied and too fast: denied.
        assert!(matches!(
            check_landing(&fast, &needing(10, StellarFlags::CAN_LAND), 0),
            Err(LandingRefusal::Denied { .. })
        ));
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
