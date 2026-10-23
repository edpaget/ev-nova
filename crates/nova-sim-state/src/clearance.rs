//! The dock request and the dock approach: the original's `_dockTarget`
//! and `_dockPermission` ([`DockRequest`]), which the land key sets
//! (`_HandlePlayerDockRequest`, see [`land_or_select`](crate::landing::land_or_select)) and the approach
//! counts on every flight frame (`_HandlePlayerDockApproach`
//! @0x647a6-0x64fff, [`approach`]). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! **The counter.** A request starts at 0 when it is made away from the
//! stellar, and at [`REQUESTED_NEAR`] when it is made within [`NEAR`] of it
//! on both axes (@0x67a0b-0x67a41). An uninhabited stellar's request
//! starts at [`CLEARED`] (@0x66b51-0x66bcd). The approach, while it
//! [clears](approach_clears) the pilot, adds 1 from [`REQUESTED_NEAR`] up,
//! sets [`CLEARED`] once the ship comes within [`NEAR`], greets the pilot
//! on the frame the counter is [`CLEARED`] exactly, and drops the request
//! once it is past [`EXPIRES_PAST`]. The land key lands only on a counter
//! past [`REQUESTED_NEAR`] (@0x66c13-0x66c23), and a counter at
//! [`CLEARED`] or more clears the next request, `Require` permitting
//! ([`Standing::granted`](crate::landing::Standing::granted), @0x67934-0x6793d).
//!
//! **The approach** (`_HandlePlayerDockApproach`), whole:
//!
//! 1. The dock stellar left the system (@0x647c5-0x647e3): the request is
//!    dropped. The session does this (`nova_sim::Session`'s
//!    approach).
//! 2. No request (@0x647e9-0x647f5): nothing. Ours holds none as `None`.
//! 3. The clearance ([`approach_clears`], @0x6480d-0x648ca), with no
//!    `Require` test anywhere.
//! 4. Cleared: [`approach`]'s count, the same for an inhabited stellar
//!    (@0x648e2-0x64970) and an uninhabited one (@0x64e9e-0x64f3b), and the
//!    greeting for an inhabited one only (@0x64975-0x64e8e), with
//!    `_beepSnd[1]`, `snd ` 151 (@0x649a9).
//! 5. Not cleared (@0x64ea0): nothing, no count and no expiry.
//! 6. The expiry (@0x64fd3-0x64ff0).
//!
//! Left out: the uninhabited greeting (`snd ` 150 @0x64f77, `STR#` 2002
//! #53 for 240 frames @0x64fbd), which needs `_dockTryFlag` set
//! (@0x64f41-0x64f49) where its only caller has it clear
//! (@0x6a970-0x6a978): dead code. The greeting's fee line
//! (@0x64d70-0x64e58) is rdm:task/landing-fee's, as the engine models no
//! fee. The distances' `i16` wraps are not copied.

use crate::catalog::{LandingSite, StellarId};
use crate::geometry::Vec2;
use crate::landing::{HYPERGATE, Standing, StellarFlags, open_by_record};

/// The counter of a request made within [`NEAR`] of the stellar
/// (@0x67a2c), and the least the approach counts up from (@0x648e2).
pub const REQUESTED_NEAR: i16 = 0x2ed;

/// The counter the approach sets within [`NEAR`] (@0x64970), and greets
/// on (@0x64975): the land key lands on more than [`REQUESTED_NEAR`]
/// (@0x66c13), and a request reads this or more as granted (@0x67934).
pub const CLEARED: i16 = 0x2ee;

/// The counter past which the approach drops the request (@0x64fd3).
pub const EXPIRES_PAST: i16 = 0x7ff;

/// How near the ship's centre must be to the stellar's, on each axis, for
/// the request to be silent and the approach to clear it: 249 pixels,
/// the distance truncated (`0xf9`, `setle`, @0x678a7-0x678ee,
/// @0x648f5-0x64967).
pub const NEAR: f32 = 249.0;

/// A request to dock: the original's `_dockTarget` and `_dockPermission`.
/// No request is `None`, the original's `_dockTarget` −1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockRequest {
    /// The stellar requested.
    pub stellar: StellarId,
    /// The counter (see the module docs).
    pub permission: i16,
}

/// What the request's line and the approach's greeting word a stellar
/// as: a hypergate by its `Flags2` 0x1000, else a station by its `Flags`
/// 0x10, else a planet (@0x67a8b-0x67b15, @0x649cd, @0x64ab0). A wormhole
/// is a station or a planet by its `Flags`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockKind {
    /// A hypergate.
    Hypergate,
    /// A station.
    Station,
    /// A planet, or anything else.
    Planet,
}

impl DockKind {
    /// The kind of a stellar of `Flags` `flags` and `Flags2` `flags2`.
    #[must_use]
    pub fn of(flags: u32, flags2: u16) -> Self {
        if flags2 & HYPERGATE != 0 {
            Self::Hypergate
        } else if flags & StellarFlags::STATION != 0 {
            Self::Station
        } else {
            Self::Planet
        }
    }

    /// The kind of `site`.
    #[must_use]
    pub fn of_site(site: &LandingSite) -> Self {
        Self::of(site.flags, site.flags2)
    }
}

/// Whether a ship `offset` from a stellar's centre is near it: within
/// [`NEAR`] on both axes, each distance truncated as the original's
/// `cvttsd2si` does.
#[must_use]
pub fn near(offset: Vec2) -> bool {
    offset.x.trunc().abs() <= NEAR && offset.y.trunc().abs() <= NEAR
}

/// Whether the approach clears the pilot of `standing` at `site`, its
/// request's counter at `permission` (@0x6480d-0x648ca): any of no
/// government, the record passing `MinStatus` (with
/// [`cleared`](crate::landing::cleared)'s sentinels), the stellar dominated, the
/// counter past [`REQUESTED_NEAR`], a mission stellar, or the government
/// blessing the pilot with landing. Unlike the request's clearance, it
/// has no `Require` test, so a blessing clears it under either reading of
/// [`BlessingLanding`](crate::RuleKey::BlessingLanding), and
/// [`Standing::meets_require`](crate::landing::Standing::meets_require) and
/// [`Standing::granted`](crate::landing::Standing::granted) are not read.
#[must_use]
pub fn approach_clears(site: &LandingSite, standing: Standing, permission: i16) -> bool {
    site.govt.is_none()
        || open_by_record(site.min_status, standing.record)
        || site.flags2 & crate::market::DOMINATED != 0
        || permission > REQUESTED_NEAR
        || standing.mission_stellar
        || standing.blessed
}

/// What one frame of the approach did, besides the count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Approach {
    /// The pilot is greeted: the counter is [`CLEARED`] exactly at an
    /// inhabited stellar (@0x64975).
    pub greet: bool,
    /// The request is past [`EXPIRES_PAST`] and is dropped (@0x64fd3).
    pub expired: bool,
}

/// Runs one frame of the approach on `request` at `site`, the ship
/// `offset` from its centre, `clears` saying whether
/// [`approach_clears`] holds: when it does not, nothing changes. When it
/// does, the counter adds 1 from [`REQUESTED_NEAR`] up (@0x648e2-0x648f2),
/// then becomes [`CLEARED`] when the ship is [`near`] and the counter is
/// [`REQUESTED_NEAR`] or less (@0x648f5-0x64970).
#[must_use]
pub fn approach(
    request: &mut DockRequest,
    site: &LandingSite,
    offset: Vec2,
    clears: bool,
) -> Approach {
    if !clears {
        return Approach::default();
    }
    if request.permission >= REQUESTED_NEAR {
        request.permission = request.permission.saturating_add(1);
    }
    if near(offset) && request.permission <= REQUESTED_NEAR {
        request.permission = CLEARED;
    }
    Approach {
        greet: request.permission == CLEARED && site.flags & StellarFlags::UNINHABITED == 0,
        expired: request.permission > EXPIRES_PAST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtId;
    use crate::landing::{WORMHOLE, cleared};
    use crate::market::DOMINATED;
    use crate::rulebook::RuleSource;

    #[test]
    fn the_named_values() {
        assert_eq!(REQUESTED_NEAR, 0x2ed);
        assert_eq!(CLEARED, 0x2ee);
        assert_eq!(EXPIRES_PAST, 0x7ff);
        assert!((NEAR - 249.0).abs() < f32::EPSILON);
    }

    #[test]
    fn near_is_within_249_on_both_axes_truncated() {
        for (x, y) in [
            (249.9, 0.0),
            (-249.9, 0.0),
            (0.0, 249.9),
            (0.0, -249.9),
            (249.0, -249.0),
            (0.0, 0.0),
        ] {
            assert!(near(Vec2::new(x, y)), "({x}, {y})");
        }
        for (x, y) in [
            (250.0, 0.0),
            (-250.0, 0.0),
            (0.0, 250.0),
            (0.0, -250.0),
            (10.0, 300.0),
            (300.0, 10.0),
        ] {
            assert!(!near(Vec2::new(x, y)), "({x}, {y})");
        }
    }

    #[test]
    fn a_hypergate_is_one_by_flags2_before_a_station_by_flags() {
        let station = StellarFlags::STATION;
        assert_eq!(DockKind::of(station, HYPERGATE), DockKind::Hypergate);
        assert_eq!(DockKind::of(0, HYPERGATE), DockKind::Hypergate);
        assert_eq!(DockKind::of(station, 0), DockKind::Station);
        assert_eq!(DockKind::of(StellarFlags::CAN_LAND, 0), DockKind::Planet);
        assert_eq!(DockKind::of(station, WORMHOLE), DockKind::Station);
        assert_eq!(DockKind::of(0, WORMHOLE), DockKind::Planet);
        let site = LandingSite {
            flags: station,
            ..planet()
        };
        assert_eq!(DockKind::of_site(&site), DockKind::Station);
        assert_eq!(DockKind::of_site(&planet()), DockKind::Planet);
    }

    /// An inhabited landable planet at the origin of government 140
    /// needing a record of 10.
    fn planet() -> LandingSite {
        LandingSite {
            id: StellarId(128),
            position: Vec2::ZERO,
            frame_size: Some((100, 100)),
            flags: StellarFlags::CAN_LAND,
            min_status: 10,
            tech_level: 0,
            special_tech: [0; 8],
            govt: Some(GovtId(140)),
            flags2: 0,
            destroyed: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            dead_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            gravity: 0,
            fee: 0,
            animation: crate::catalog::SiteAnimation::default(),
        }
    }

    /// A pilot with a record of 0 who meets no `Require`, of no mission
    /// there, unblessed and with no request granted.
    fn nobody() -> Standing {
        Standing::default()
    }

    #[test]
    fn the_approach_clears_on_any_one_of_its_clauses() {
        assert!(!approach_clears(&planet(), nobody(), 0), "none holds");
        let ungoverned = LandingSite {
            govt: None,
            ..planet()
        };
        assert!(approach_clears(&ungoverned, nobody(), 0), "no government");
        let record = Standing {
            record: 10,
            ..nobody()
        };
        assert!(approach_clears(&planet(), record, 0), "the record");
        let dominated = LandingSite {
            flags2: DOMINATED,
            ..planet()
        };
        assert!(approach_clears(&dominated, nobody(), 0), "dominated");
        assert!(approach_clears(&planet(), nobody(), 0x2ee), "the counter");
        let mission = Standing {
            mission_stellar: true,
            ..nobody()
        };
        assert!(approach_clears(&planet(), mission, 0), "a mission stellar");
        let blessed = Standing {
            blessed: true,
            ..nobody()
        };
        assert!(approach_clears(&planet(), blessed, 0), "blessed");
    }

    #[test]
    fn the_approach_uses_the_requests_record_sentinels() {
        let needing = |min_status| LandingSite {
            min_status,
            ..planet()
        };
        let record = |record| Standing { record, ..nobody() };
        assert!(!approach_clears(&needing(32767), record(32767), 0));
        assert!(approach_clears(&needing(-32767), record(-32768), 0));
        assert!(!approach_clears(&needing(11), record(10), 0));
    }

    #[test]
    fn the_approach_has_no_require_test() {
        let unmet = Standing {
            record: 10,
            meets_require: false,
            ..nobody()
        };
        assert!(approach_clears(&planet(), unmet, 0));
        for source in RuleSource::ALL {
            assert!(!cleared(&planet(), unmet, source), "the request's does");
        }
        let blessed = Standing {
            blessed: true,
            meets_require: false,
            ..nobody()
        };
        assert!(approach_clears(&planet(), blessed, 0));
        assert!(!cleared(&planet(), blessed, RuleSource::Bible));
    }

    #[test]
    fn the_counter_clears_the_approach_only_past_0x2ed() {
        assert!(!approach_clears(&planet(), nobody(), 0x2ed));
        assert!(approach_clears(&planet(), nobody(), 0x2ee));
        let granted = Standing {
            granted: true,
            ..nobody()
        };
        assert!(
            !approach_clears(&planet(), granted, 0),
            "granted is the request's"
        );
    }

    /// The approach on a request of `permission` at `site`, the ship
    /// `offset` away, clearing as `clears` says: the counter after, and
    /// what it did.
    fn run(site: &LandingSite, permission: i16, offset: Vec2, clears: bool) -> (i16, Approach) {
        let mut request = DockRequest {
            stellar: StellarId(128),
            permission,
        };
        let done = approach(&mut request, site, offset, clears);
        assert_eq!(request.stellar, StellarId(128));
        (request.permission, done)
    }

    const FAR: Vec2 = Vec2::new(0.0, 250.0);
    const OVER: Vec2 = Vec2::new(249.9, -249.9);

    fn did(greet: bool, expired: bool) -> Approach {
        Approach { greet, expired }
    }

    #[test]
    fn an_approach_that_does_not_clear_changes_nothing() {
        for permission in [0, 0x2ed, 0x2ee, 0x7ff, 0x800] {
            for offset in [FAR, OVER] {
                assert_eq!(
                    run(&planet(), permission, offset, false),
                    (permission, did(false, false)),
                    "{permission:#x}"
                );
            }
        }
    }

    #[test]
    fn a_far_request_waits_at_0_until_the_ship_comes_near() {
        assert_eq!(run(&planet(), 0, FAR, true), (0, did(false, false)));
        assert_eq!(run(&planet(), 0, OVER, true), (0x2ee, did(true, false)));
    }

    #[test]
    fn a_near_request_counts_up_to_0x2ee_and_greets_wherever_the_ship_is() {
        assert_eq!(run(&planet(), 0x2ed, FAR, true), (0x2ee, did(true, false)));
        assert_eq!(run(&planet(), 0x2ed, OVER, true), (0x2ee, did(true, false)));
    }

    #[test]
    fn past_0x2ee_the_counter_counts_on_and_greets_no_more() {
        assert_eq!(
            run(&planet(), 0x2ee, OVER, true),
            (0x2ef, did(false, false))
        );
        assert_eq!(run(&planet(), 0x2ee, FAR, true), (0x2ef, did(false, false)));
        assert_eq!(run(&planet(), 0x500, FAR, true), (0x501, did(false, false)));
    }

    #[test]
    fn an_uninhabited_stellar_counts_alike_and_never_greets() {
        let uninhabited = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..planet()
        };
        assert_eq!(
            run(&uninhabited, 0x2ed, OVER, true),
            (0x2ee, did(false, false))
        );
        assert_eq!(run(&uninhabited, 0, OVER, true), (0x2ee, did(false, false)));
        assert_eq!(
            run(&uninhabited, 0x2ee, FAR, true),
            (0x2ef, did(false, false))
        );
        assert_eq!(
            run(&uninhabited, 0x7ff, FAR, true),
            (0x800, did(false, true))
        );
    }

    #[test]
    fn past_0x7ff_the_request_expires() {
        assert_eq!(run(&planet(), 0x7fe, FAR, true), (0x7ff, did(false, false)));
        assert_eq!(run(&planet(), 0x7ff, FAR, true), (0x800, did(false, true)));
        assert_eq!(run(&planet(), 0x7ff, OVER, true), (0x800, did(false, true)));
    }
}
