//! Landing: what the land key does, whether the player's ship can land on
//! a stellar, and what the stellar offers once it has.
//!
//! The land key ([`land_or_select`]) first requests clearance, then
//! lands, as the EV Nova manual says: "select the stellar object you
//! intend to land on ... Then request landing clearance by pressing the L
//! key. You'll be given clearance to land, then press the L key again",
//! "or just press the L key to request clearance on the nearest planet in
//! the system" (and `STR#` 2002 #25: "to request landing clearance, then
//! hit it again to land"). It is `_HandlePlayerDockRequest`
//! (@0x66691-0x67ef9), and the dock request it keeps is the
//! [`clearance`] module's [`DockRequest`]. L, in this
//! order:
//!
//! 1. **The stellar.** The navigation target, the stellar held wherever
//!    it is now, never swapped for another (`_HandlePlayer` @0x6a8ed goes
//!    to the nearest pick only when +0x6c is -1); with none, the nearest
//!    stellar with the can-land flag, destroyed or not
//!    ([`nearest_stellar`], `_FindNearestStellar`), station or planet,
//!    which the press selects ([`DockPress::selected`]) and goes on with
//!    (@0x6a8ed-0x6a956); a pick it can't land on is selected and then
//!    refused at step 2. With
//!    stellars but none with the can-land flag it selects nothing and
//!    refuses with
//!    [`LandingRefusal::NothingLandable`]; with no stellars,
//!    [`LandingRefusal::NoStellars`]. Both are silent: the land key makes
//!    no dock request (@0x6a94c).
//! 2. **Not landable** (@0x6695c-0x669ae): a stellar that cannot be landed
//!    on (`_StellarIsLandable` @0x66977), or that the player's system no
//!    longer holds, its owner (+0x14) not the player's system (+0x74,
//!    @0x6699a-0x669a4) or it inactive (+0x44, @0x669aa), refuses with
//!    [`LandingRefusal::NotLandable`], wherever the ship is (`STR#` 2002
//!    #84/#88/#90, "Your ship is unable to land on ... The planet's
//!    environment is too hostile.", @0x67cda-0x67e8f). The request is
//!    kept. The owner can differ from the player's system between rebuild
//!    points, once a galaxy change has given the stellar to another
//!    system; the caller says whether the system holds it.
//! 3. **Uninhabited** (@0x66b51-0x66bcd): the request becomes the
//!    stellar's at [`CLEARED`], "No response." is said
//!    ([`DockSaid::NoResponse`]), and the press goes on to 4, so it lands
//!    in the same press when it can.
//! 4. **The attempt** (@0x66bd2-0x67800): when the request is the
//!    stellar's, it lands if [`check_landing`] allows, and the request
//!    ends. Otherwise the refusal keeps the request and the target, and
//!    the player tries again (the manual: "If you're too far away, or if
//!    you're moving too fast ... your ship's computer will beep at you and
//!    you can try again").
//! 5. **The request** (@0x6788f-0x67c4b, @0x67ea3-0x67ef9): otherwise
//!    traffic control is asked ([`cleared`]), the old counter granting it
//!    ([`Standing::granted`]). Cleared, the request becomes the stellar's,
//!    at [`REQUESTED_NEAR`] in silence within
//!    [`NEAR`](crate::clearance::NEAR) of it on both axes, else at 0 with
//!    the request's line ([`DockSaid::Requested`]), and the press gives
//!    [`LandOutcome::Requested`]. Not cleared, [`LandingRefusal::Denied`],
//!    and the request is dropped.
//!
//! So a press lands only once the dock approach has counted the request
//! past [`REQUESTED_NEAR`] ([`approach`](crate::clearance::approach)), and
//! traffic control is not asked again. [`check_landing`] gives the
//! stellar, or the first [`LandingRefusal`] that applies, in this order:
//!
//! 1. [`LandingRefusal::TooFar`]: the counter is not past
//!    [`REQUESTED_NEAR`] (@0x66c13-0x66c23), or the ship is not over the
//!    target, by its reach (@0x66c6c-0x66cfd). The same reach and counter
//!    decide whether a ship moving too fast is told it is too far or too
//!    fast, as the original's dock request reads them again for its
//!    refusal (@0x6773a-0x677c0).
//! 2. [`LandingRefusal::TooFast`]: the ship is moving faster than
//!    [`LANDING_SPEED`].
//!
//! **Clearance** ([`cleared`]) is `_HandlePlayerDockRequest`'s clearance
//! block (@0x67903-0x67a05), whole. Its checks run in this order, the pilot's [`Standing`] giving what they
//! read of the pilot:
//!
//! 1. The record: the legal record (by the engine the one in the
//!    player's system; by the Bible the one with the stellar's
//!    government, or its system's when it has none, [`landing_record`])
//!    is at least `MinStatus`, or `MinStatus` is [`ANY_RECORD`]; never
//!    when `MinStatus` is [`NEVER_BY_RECORD`] (@0x67903-0x67921).
//! 2. Domination: the stellar is dominated (@0x6792d). The original's
//!    byte (+0x46) is set by Demand Tribute; the engine knows only the
//!    always-dominated `Flags2` [`DOMINATED`](crate::market::DOMINATED).
//! 3. A request already granted (@0x67934-0x6793d): the dock request's
//!    counter at [`CLEARED`] or more ([`Standing::granted`]), whichever
//!    stellar it is for. The approach sets it ([`clearance`]).
//! 4. `Require`: at a governed stellar, a pilot who does not contribute
//!    every `Require` bit of its government is cleared by none of 1-3
//!    (@0x67940-0x6796b, `_PlayerMeetsRequirements` @0x776f).
//! 5. Mission stellars: otherwise the stellar is an active mission's
//!    travel or return stellar, compared as raw IDs over every slot,
//!    failed ones included (@0x67977-0x679cb).
//! 6. The blessing: otherwise a governed stellar's government blesses the
//!    pilot with landing (`_PlayerBlessedByGovt` which 1, @0x679cd-0x67a05;
//!    see `nova_sim::rank`'s **Blessing**).
//!
//! None of them refuses (@0x67ea3), the same refusal for `Require` as for
//! the record. So by the engine the blessing gets the pilot past both
//! `MinStatus` and `Require`, as a mission stellar does; by the other
//! reading of [`BlessingLanding`](crate::RuleKey::BlessingLanding), the
//! Bible's, past `MinStatus` only. The clearance block plays no sound.
//!
//! The dock approach's own clearance
//! ([`approach_clears`](crate::clearance::approach_clears)) has no
//! `Require` test, so it is not [`cleared`].
//!
//! Left out of the press:
//!
//! - An accepted planet bribe's counter, 0x2ef (`_DoPlanetCommDialog`
//!   @0x97112-0x97125), domination by Demand Tribute (+0x46 written
//!   @0x978fa, cleared @0x9740f) and `_DoPlanetCommDialog`'s blessing
//!   (@0x96c15): rdm:task/planet-domination.
//! - The fee: its refusal (@0x669b4-0x66a80), the approach greeting's fee
//!   line (@0x64d70-0x64e58) and the charge (@0x66db9-0x66df5):
//!   rdm:task/landing-fee. The engine models no fee, and no stock stellar
//!   has one.
//! - The cloak refusal (@0x66a85-0x66ae9) and the silent press of a
//!   disabled or dying ship (@0x66aee-0x66b14), which the
//!   `nova_sim::Session` refuses up front with
//!   [`LandingRefusal::Disabled`]:
//!   rdm:phase/player-death/phase-16-handle-player-left-out-gates.
//! - The cloak refusal (`_ShipIsCloaked` @0x66a8e, STR# 2002 #73),
//!   which beeps `snd ` 153 at the shared tail (@0x67e8f): the engine
//!   has no player cloak (rdm:task/cloaking-device).
//! - The fee refusal (@0x669b4-0x66a80), which is silent
//!   (rdm:task/landing-fee).
//! - The per-axis 0.75 speed gate (@0x66c3c-0x66c66):
//!   rdm:phase/stellar-fidelity/phase-6-per-axis-landing-speed.
//! - The player's AI wait (+0x4c, tested @0x66c2f and zeroed @0x66b6d):
//!   the engine keeps none.
//!
//! Each refusal says whether the stellar is a station, which picks the
//! original's "dock at this station" or "land on this planet" wording.
//!
//! Hypergates and wormholes are requested and landed on the same way;
//! the `nova_sim::Session` enters one instead of docking
//! (`nova_sim::LandPress::AtGate`, see
//! `nova_sim::gate`).
//!
//! **The reach** ([`landing_reach`]) follows
//! [`RuleKey::LandingDistance`](crate::RuleKey::LandingDistance). By the
//! engine's reading, the original's dock request
//! (`_HandlePlayerDockRequest` @0x66cc7-0x66cfd): a box, the ship over
//! the stellar when both its x and its y distance from the centre are
//! strictly below trunc([`REACH_PER_WIDTH`] × the width), the width being
//! `_GetStellarSpriteXSize`'s (@0xac91-0xad2d):
//!
//! - the `Type` frame's width ([`TypeWidth::Frame`]) while the `Type`
//!   master is loaded, the frame's width and not its larger side;
//! - [`UNLOADED_TYPE_WIDTH`], 150, while it is not
//!   ([`TypeWidth::Unloaded`]): a destroyed stellar from a rebuild point
//!   until a flight frame loads it again, unless another stellar shown
//!   keeps it loaded, or a `Type` whose sheet cannot be read;
//! - [`OUT_OF_RANGE_FRAME_WIDTH`], 32, for a loaded master whose `spïn`
//!   gives no frame count ([`TypeWidth::OutOfRange`]);
//!
//! and the reach is [`NO_FRAME_REACH`], 75, whatever the width, when the
//! stellar's sprite slot shows no frame. The session keeps which masters
//! are loaded and which slots are empty (see the `story` module's "Built
//! by phase 10"). By the other reading, the port's earlier circle: within
//! [`landing_radius`] of the centre.
//!
//! NPCs land by their own rule, which stays on the [`landing_radius`]
//! circle under either reading: the original's NPC landing is
//! `_HighLevelAIHandler` state 1, whose box goes by the ship's turn rate
//! (`phase/stellar-fidelity/phase-5-npc-landing-reach`). The quarter of
//! the width, inclusive (@0x8dbdf), is state 20's box for entering a
//! hypergate or wormhole, not a landing ([`stellar_width`], see
//! `nova_sim::traffic::gate_entry`).
//! The original's speed gate is per axis (story-runtime phase 17,
//! per-axis-landing-speed).

use crate::catalog::{GovtId, LandingSite, StellarId};
use crate::clearance::{self, CLEARED, DockRequest, REQUESTED_NEAR};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

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

/// The `spöb` `Flags2` bit of a hypergate.
pub const HYPERGATE: u16 = 0x1000;
/// The `spöb` `Flags2` bit of a wormhole.
pub const WORMHOLE: u16 = 0x2000;

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

/// Why the ship cannot land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LandingRefusal {
    /// The ship is jumping through hyperspace. The
    /// `nova_sim::Session` gives this before [`check_landing`] is
    /// asked.
    Jumping,
    /// The ship is disabled, breaking up or destroyed: refused silently,
    /// the dock request's bare epilogue 0x67eef, after the landability
    /// check (and the fee and cloak, not modelled) for a disabled ship,
    /// and before anything for a breaking-up or destroyed one (see
    /// `nova_sim::Session::land`).
    Disabled,
    /// The system has no stellars, so the land key has nothing to select
    /// and makes no dock request (@0x6a94c): a silent refusal.
    NoStellars,
    /// No stellar was selected, and the system has stellars but none with
    /// the can-land flag, so none the land key would select (`_FindNearestStellar` gave -1, @0x6a94c):
    /// the press makes no dock request, a silent refusal. It names no
    /// stellar, as the original selects none.
    NothingLandable,
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

impl LandingRefusal {
    /// Whether `_HandlePlayerDockRequest` beeps `_beepSnd[3]` (`snd ` 153)
    /// for this refusal: a stellar that can't be landed on (wrong
    /// government or inactive, @0x66977-0x669ae, `PlaySound` @0x67e8f,
    /// before `_IsDisabled`, so a disabled ship's beeps too), out of
    /// reach (@0x66cfd, @0x676aa), or pressed again before the approach
    /// clears or moving too fast (@0x66c23-0x66c66, @0x676fe). The
    /// others are silent of it: [`Self::Jumping`], [`Self::NoStellars`]
    /// and [`Self::NothingLandable`] never reach the dock request
    /// (`_HandlePlayer` @0x6a8cb, @0x6a90f, and `_FindNearestStellar`
    /// giving -1 @0x6a94c), [`Self::Disabled`] goes to its bare epilogue
    /// (0x67eef), and [`Self::Denied`] plays `_beepSnd[1]` instead
    /// (@0x67ec6).
    ///
    /// A [`Self::NotLandable`] from a press with no target beeps too: the
    /// press selects the nearest stellar with the can-land flag
    /// ([`nearest_stellar`]) and makes its dock request, which refuses it
    /// as the original's does (@0x66977 -> 0x67cda).
    #[must_use]
    pub fn beeps(&self) -> bool {
        matches!(
            self,
            Self::NotLandable { .. } | Self::TooFar { .. } | Self::TooFast { .. }
        )
    }
}

/// Half the larger side of `site`'s sprite's frame, which is what the
/// player sees as its disc, or [`UNKNOWN_STELLAR_RADIUS`] without a
/// sprite: the landing radius of [`RuleKey::LandingDistance`]'s other
/// reading, the radius NPCs land within, and the disc the shots hit.
///
/// [`RuleKey::LandingDistance`]: crate::RuleKey::LandingDistance
#[must_use]
pub fn landing_radius(site: &LandingSite) -> f32 {
    site.frame_size
        .map_or(UNKNOWN_STELLAR_RADIUS, |(width, height)| {
            width.max(height) as f32 / 2.0
        })
}

/// The original's reach when the stellar's sprite slot shows no frame
/// (`_HandlePlayerDockRequest` @0x66cc7).
pub const NO_FRAME_REACH: f32 = 75.0;

/// The width `_GetStellarSpriteXSize` gives a stellar whose `Type` master
/// is not loaded (@0xad2d).
pub const UNLOADED_TYPE_WIDTH: u32 = 150;

/// The width `_GetStellarSpriteXSize` gives a loaded `Type` master whose
/// frame index is outside its frame count (@0xacff).
pub const OUT_OF_RANGE_FRAME_WIDTH: u32 = 32;

/// The reach per pixel of width: the double at 0xdd6e8.
pub const REACH_PER_WIDTH: f64 = 1.75;

/// Which width `_GetStellarSpriteXSize` reads of a stellar's `Type` master.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeWidth {
    /// The master is loaded and its frame index is in range: the `Type`
    /// frame's width.
    Frame,
    /// The master is loaded but its frame index is outside its frame
    /// count: [`OUT_OF_RANGE_FRAME_WIDTH`].
    OutOfRange,
    /// The master is not loaded: [`UNLOADED_TYPE_WIDTH`].
    Unloaded,
}

/// What the landing distance reads of a stellar besides its frame size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StellarShown {
    /// Whether its sprite slot shows any frame.
    pub frame: bool,
    /// Which width its `Type` master gives.
    pub type_width: TypeWidth,
}

/// How close the ship's centre must be to a stellar's to land on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LandingReach {
    /// Both the x and the y distance strictly below this: the original's
    /// box.
    Box(f32),
    /// The distance at most this: the other reading's circle,
    /// [`landing_radius`].
    Circle(f32),
}

impl LandingReach {
    /// Whether a ship `offset` from the stellar's centre is over it.
    #[must_use]
    pub fn holds(self, offset: Vec2) -> bool {
        match self {
            Self::Box(reach) => offset.x.abs() < reach && offset.y.abs() < reach,
            Self::Circle(radius) => offset.length() <= radius,
        }
    }
}

/// `site`'s width as `_GetStellarSpriteXSize` gives it (@0xac91-0xad2d),
/// its `Type` master giving `type_width`: the `Type` frame's width,
/// [`OUT_OF_RANGE_FRAME_WIDTH`], or [`UNLOADED_TYPE_WIDTH`] (also for a
/// frame of no size known).
#[must_use]
pub fn stellar_width(site: &LandingSite, type_width: TypeWidth) -> u32 {
    match (type_width, site.frame_size) {
        (TypeWidth::Frame, Some((width, _))) => width,
        (TypeWidth::OutOfRange, _) => OUT_OF_RANGE_FRAME_WIDTH,
        (TypeWidth::Frame | TypeWidth::Unloaded, _) => UNLOADED_TYPE_WIDTH,
    }
}

/// `site`'s landing reach under `source`, given what it shows: see the
/// module docs.
#[must_use]
pub fn landing_reach(site: &LandingSite, shown: StellarShown, source: RuleSource) -> LandingReach {
    if source == RuleSource::Bible {
        return LandingReach::Circle(landing_radius(site));
    }
    if !shown.frame {
        return LandingReach::Box(NO_FRAME_REACH);
    }
    let width = stellar_width(site, shown.type_width);
    // Truncated, as `cvttsd2si` does: rounding, not a word wrap.
    #[allow(clippy::cast_possible_truncation)]
    let reach = (f64::from(width) * REACH_PER_WIDTH).trunc() as f32;
    LandingReach::Box(reach)
}

/// What the land key did, when it was not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandOutcome {
    /// It requested docking at the stellar, and traffic control cleared
    /// the pilot (`_HandlePlayerDockRequest` @0x67a0b): the request's
    /// line is said away from it ([`DockSaid::Requested`]), and nothing
    /// within [`NEAR`](crate::clearance::NEAR).
    Requested {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
    },
    /// It landed on the stellar.
    Landed(StellarId),
}

/// What the land key said, besides its outcome and its refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSaid {
    /// "No response.": the stellar is uninhabited (@0x66b51-0x66bcd). A
    /// refusal in the same press replaces it on the message line.
    NoResponse,
    /// The request's line, with its sound: the request was cleared away
    /// from the stellar (@0x67a41-0x67c46).
    Requested,
}

/// What one press of the land key did ([`land_or_select`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DockPress {
    /// The stellar the press selected, when nothing was selected.
    pub selected: Option<StellarId>,
    /// The dock request after the press.
    pub dock: Option<DockRequest>,
    /// What it said besides its outcome.
    pub said: Option<DockSaid>,
    /// What it did, or why not.
    pub outcome: Result<LandOutcome, LandingRefusal>,
}

/// What the land key does with the navigation target `target` (the
/// stellar held, wherever it is now) and the dock request `dock`, for the
/// ship `player` among the system's stellars `sites` (in navigation
/// order), where `cleared` says whether traffic control clears the pilot
/// to request docking at an inhabited stellar ([`cleared`]), `held` says
/// whether the player's system still holds a stellar, owned and active,
/// and `reach` gives a stellar's [`LandingReach`]: see the module docs.
pub fn land_or_select(
    target: Option<&LandingSite>,
    dock: Option<DockRequest>,
    player: &ShipState,
    sites: &[LandingSite],
    cleared: impl Fn(&LandingSite) -> bool,
    held: impl Fn(&LandingSite) -> bool,
    reach: impl Fn(&LandingSite) -> LandingReach,
) -> DockPress {
    let refused = |refusal| DockPress {
        selected: None,
        dock,
        said: None,
        outcome: Err(refusal),
    };
    let (site, selected) = match target {
        Some(site) => (site, None),
        None => match nearest_stellar_site(player.position, sites) {
            Some(site) => (site, Some(site.id)),
            None if sites.is_empty() => return refused(LandingRefusal::NoStellars),
            None => return refused(LandingRefusal::NothingLandable),
        },
    };
    if !is_landable(site) || !held(site) {
        return DockPress {
            selected,
            dock,
            said: None,
            outcome: Err(not_landable(site)),
        };
    }
    let (stellar, station) = (site.id, is_station(site));
    let (mut dock, mut said) = (dock, None);
    if has(site, StellarFlags::UNINHABITED) {
        dock = Some(DockRequest {
            stellar,
            permission: CLEARED,
        });
        said = Some(DockSaid::NoResponse);
    }
    let pressed = |dock, said, outcome| DockPress {
        selected,
        dock,
        said,
        outcome,
    };
    if let Some(request) = dock.filter(|request| request.stellar == stellar) {
        let attempt = check_landing(player, site, request.permission, reach(site));
        let dock = if attempt.is_ok() { None } else { dock };
        return pressed(dock, said, attempt.map(LandOutcome::Landed));
    }
    if !cleared(site) {
        let denied = LandingRefusal::Denied {
            stellar,
            station,
            min_status: site.min_status,
        };
        return pressed(None, said, Err(denied));
    }
    let near = clearance::near(site.position - player.position);
    let request = DockRequest {
        stellar,
        permission: if near { REQUESTED_NEAR } else { 0 },
    };
    let said = if near {
        said
    } else {
        Some(DockSaid::Requested)
    };
    pressed(
        Some(request),
        said,
        Ok(LandOutcome::Requested { stellar, station }),
    )
}

/// Whether `site` can be landed on at all, by its flags and whether it is
/// destroyed: it has the can-land flag, and it is destroyed exactly when
/// it is landable only once destroyed, as the original's
/// `_StellarIsLandable` reads it (@0xa989-0xa9f7). So a destroyed stellar
/// cannot be landed on, unless it can be landed on only once destroyed.
/// The pilot's record does not count, so a stellar that would deny
/// clearance is landable.
#[must_use]
pub fn is_landable(site: &LandingSite) -> bool {
    landable(site.flags, site.destroyed)
}

/// Whether a stellar of `Flags` `flags`, `destroyed` or not, is landable
/// (see [`is_landable`]).
#[must_use]
pub fn landable(flags: u32, destroyed: bool) -> bool {
    flags & StellarFlags::CAN_LAND != 0
        && destroyed == (flags & StellarFlags::ONLY_WHEN_DESTROYED != 0)
}

/// Whether a pilot may be docked at `site`: it is [landable](is_landable)
/// and is no hypergate or wormhole, which are entered instead.
#[must_use]
pub fn is_dockable(site: &LandingSite) -> bool {
    GateKind::of(site.flags2).is_none() && is_landable(site)
}

/// Whether `site`, the player's navigation target, answers a hail with
/// its comm dialog: it is [dockable](is_dockable) and not
/// [uninhabited](StellarFlags::UNINHABITED), as
/// `_HandlePlayerCommunication` reads it (`_StellarIsLandable` @0x61fda,
/// `Flags` 0x20 @0x61fe5, `Flags2` 0x3000 @0x61fff-0x62006). Its active
/// byte (+0x44 @0x62009) is the session's to test: a stellar shown in the
/// player's system.
#[must_use]
pub fn answers_hail(site: &LandingSite) -> bool {
    is_dockable(site) && !has(site, StellarFlags::UNINHABITED)
}

/// Whether a pilot may be docked at `site` whatever its destroyed state:
/// it has the can-land flag and is no hypergate or wormhole. Being
/// destroyed, or landable only once destroyed, does not count: a landed
/// `M` docks so, as the original reads neither there (see the
/// `script_effects` module of the session).
#[must_use]
pub fn is_dockable_destroyed_or_not(site: &LandingSite) -> bool {
    GateKind::of(site.flags2).is_none() && has(site, StellarFlags::CAN_LAND)
}

/// The stellar among `sites` whose centre is nearest `position`, as
/// `_FindNearestStellar` (@0x276c-0x2888) picks it: it skips only a
/// stellar without the can-land flag (`Flags` 0x01, @0x27cd), so a
/// destroyed one counts, and so does an intact one landable only once
/// destroyed. Inactive stellars are never in `sites`. The earlier in
/// `sites` of any equally near wins (@0x285a-0x285d); `None` when none has
/// the flag. Left out: the hypergate/wormhole 500 px limit
/// (@0x2838-0x2853) and candidates owned by another visible system, both
/// rdm:task/gate-stellar-targeting-fidelity.
#[must_use]
pub fn nearest_stellar(position: Vec2, sites: &[LandingSite]) -> Option<StellarId> {
    nearest_stellar_site(position, sites).map(|site| site.id)
}

/// The stellar [`nearest_stellar`] (`_FindNearestStellar`) names, and whether it is a station
/// (`spöb` `Flags` 0x10), as the tutorial's land hint words it
/// (`_HandlePlayer` @0x709e8-0x70a31; see `nova_sim::tutorial`).
#[must_use]
pub fn nearest_stellar_station(position: Vec2, sites: &[LandingSite]) -> Option<(StellarId, bool)> {
    nearest_stellar_site(position, sites).map(|site| (site.id, is_station(site)))
}

/// The site [`nearest_stellar`] names: the one the land key selects when
/// there is no target.
fn nearest_stellar_site(position: Vec2, sites: &[LandingSite]) -> Option<&LandingSite> {
    nearest(
        position,
        sites
            .iter()
            .filter(|site| has(site, StellarFlags::CAN_LAND)),
    )
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

/// Whether `player` lands on `site`, a landable stellar it has requested
/// with its counter at `permission`, over it within `reach`: the
/// stellar, or the refusal that applies (`_HandlePlayerDockRequest`
/// @0x66c13-0x66cfd, @0x67687-0x67800). A counter not past
/// [`REQUESTED_NEAR`] is [`LandingRefusal::TooFar`] wherever the ship is,
/// as is a ship outside the reach; then a ship faster than
/// [`LANDING_SPEED`] is [`LandingRefusal::TooFast`].
pub fn check_landing(
    player: &ShipState,
    site: &LandingSite,
    permission: i16,
    reach: LandingReach,
) -> Result<StellarId, LandingRefusal> {
    let (stellar, station) = (site.id, is_station(site));
    if permission <= REQUESTED_NEAR || !reach.holds(site.position - player.position) {
        return Err(LandingRefusal::TooFar { stellar, station });
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

/// The `MinStatus` that never clears by record (@0x67915).
pub const NEVER_BY_RECORD: i16 = 32767;

/// The `MinStatus` that clears any record (@0x67909).
pub const ANY_RECORD: i16 = -32767;

/// What traffic control weighs of the pilot at a stellar, besides the
/// stellar itself (`_HandlePlayerDockRequest` @0x67903-0x67a05).
// Each flag is one clause of the clearance, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Standing {
    /// The legal record `MinStatus` is checked against.
    pub record: i16,
    /// Whether the pilot contributes every `Require` bit of the stellar's
    /// government (`_PlayerMeetsRequirements` @0x776f).
    pub meets_require: bool,
    /// Whether the stellar is an active mission's travel or return
    /// stellar (@0x67977-0x679cb).
    pub mission_stellar: bool,
    /// Whether the stellar's government blesses the pilot with landing
    /// (`_PlayerBlessedByGovt` which 1, @0x679cd-0x67a05; see
    /// `nova_sim::rank`'s **Blessing**).
    pub blessed: bool,
    /// Whether the pilot's dock request stands at
    /// [`CLEARED`] or more, whichever stellar
    /// it is for (@0x67934-0x6793d; see [`clearance`]).
    pub granted: bool,
}

/// Whether a stellar of `MinStatus` `min_status` is open to a pilot of
/// legal `record` by its record alone: the record is at least `MinStatus`
/// or `MinStatus` is [`ANY_RECORD`], and never when `MinStatus` is
/// [`NEVER_BY_RECORD`] (`_HandlePlayerDockRequest` @0x67903-0x67921,
/// `_HandlePlayerDockApproach` @0x6481c-0x64843, and the escape pod's
/// search @0xd1de-0xd1fc).
#[must_use]
pub fn open_by_record(min_status: i16, record: i16) -> bool {
    (record >= min_status || min_status == ANY_RECORD) && min_status != NEVER_BY_RECORD
}

/// Whether traffic control clears the pilot of `standing` to land at
/// `site`, an inhabited stellar, as `_HandlePlayerDockRequest`'s
/// clearance block decides (@0x67903-0x67a05), by `blessing`'s reading of
/// [`BlessingLanding`](crate::RuleKey::BlessingLanding): see the module
/// docs.
///
/// The record passes when it is at least `MinStatus` or `MinStatus` is
/// [`ANY_RECORD`], and never when `MinStatus` is [`NEVER_BY_RECORD`]. The
/// Bible's plain comparison differs from these sentinels only for a
/// record of exactly 32767 or -32768, so they have no rule of their own.
#[must_use]
pub fn cleared(site: &LandingSite, standing: Standing, blessing: RuleSource) -> bool {
    let by_record = open_by_record(site.min_status, standing.record);
    let dominated = site.flags2 & crate::market::DOMINATED != 0;
    let governed = site.govt.is_some();
    let require_ok = !governed || standing.meets_require;
    let blessed = governed && standing.blessed;
    let granted = standing.granted;
    match blessing {
        RuleSource::Engine => {
            ((by_record || dominated || granted) && require_ok)
                || standing.mission_stellar
                || blessed
        }
        RuleSource::Bible => {
            ((by_record || dominated || granted || blessed) && require_ok)
                || standing.mission_stellar
        }
    }
}

/// The legal record that `site`'s `MinStatus` is checked against by the
/// Bible (the [`LegalRecords`](crate::RuleKey::LegalRecords) rule's
/// reading of a record with each government), in a system governed by
/// `system_govt`, where `record` gives the pilot's record with a
/// government. By the engine, landing reads the record in the player's
/// system instead, as the original does (see [`legal`](crate::legal)).
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
/// Out of scope here: `chär` status spreading to allies and enemies.
pub fn landing_record(
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
    /// The mission computer, whose view the session gives
    /// (`nova_sim::Session::mission_computer`).
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
            tech_level: 0,
            special_tech: [0; 8],
            govt: None,
            flags2: 0,
            destroyed: false,
            graphic: 0,
            dead_graphic: None,
            dead_frame_size: None,
            type_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            gravity: 0,
            animation: crate::catalog::SiteAnimation::default(),
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

    /// `site`'s reach by the engine's reading, its `Type` master loaded
    /// and a frame shown.
    fn engine_reach(site: &LandingSite) -> LandingReach {
        landing_reach(site, LOADED, RuleSource::Engine)
    }

    /// A stellar whose slot shows a frame and whose `Type` master is
    /// loaded.
    const LOADED: StellarShown = StellarShown {
        frame: true,
        type_width: TypeWidth::Frame,
    };

    /// L pressed by `player` with `site` selected and requested, the
    /// request at [`CLEARED`], by `site`'s [`engine_reach`]: the stellar
    /// landed on, or the refusal. Traffic control is not asked.
    fn check(player: &ShipState, site: &LandingSite) -> Result<StellarId, LandingRefusal> {
        let press = land_or_select(
            Some(site),
            Some(DockRequest {
                stellar: site.id,
                permission: CLEARED,
            }),
            player,
            std::slice::from_ref(site),
            |_| panic!("traffic control is asked"),
            |_| true,
            engine_reach,
        );
        press.outcome.map(|outcome| match outcome {
            LandOutcome::Landed(stellar) => stellar,
            LandOutcome::Requested { .. } => panic!("{outcome:?}"),
        })
    }

    /// L pressed by a ship parked over `site`, the system's one stellar,
    /// with nothing selected and nothing requested, traffic control
    /// clearing by the record `landing_record` reads in a system of
    /// `system_govt` from `record`: the stellar requested (or landed on,
    /// uninhabited), or the refusal.
    fn ask(
        site: &LandingSite,
        system_govt: Option<GovtId>,
        record: impl Fn(GovtId) -> i16,
    ) -> Result<StellarId, LandingRefusal> {
        let press = land_or_select(
            None,
            None,
            &parked(),
            std::slice::from_ref(site),
            |site| landing_record(site, system_govt, &record) >= site.min_status,
            |_| true,
            engine_reach,
        );
        press.outcome.map(|outcome| match outcome {
            LandOutcome::Landed(stellar) | LandOutcome::Requested { stellar, .. } => stellar,
        })
    }

    #[test]
    fn the_named_values() {
        assert_eq!(LANDING_SPEED, 1.0);
        assert_eq!(UNKNOWN_STELLAR_RADIUS, 32.0);
        assert_eq!(NO_FRAME_REACH, 75.0);
        assert_eq!(UNLOADED_TYPE_WIDTH, 150);
        assert_eq!(OUT_OF_RANGE_FRAME_WIDTH, 32);
        assert_eq!(REACH_PER_WIDTH, 1.75);
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
    fn destroyed_a_stellar_is_landable_only_when_it_can_be_landed_on_only_once_destroyed() {
        use StellarFlags as F;
        let destroyed = |flags| LandingSite {
            destroyed: true,
            ..with_flags(flags)
        };
        for (flags, landable) in [
            (0, false),
            (F::CAN_LAND, false),
            (F::CAN_LAND | F::STATION, false),
            (F::CAN_LAND | F::ONLY_WHEN_DESTROYED, true),
            (F::ONLY_WHEN_DESTROYED, false),
        ] {
            assert_eq!(is_landable(&destroyed(flags)), landable, "{flags:#x}");
        }
        assert!(is_dockable(&destroyed(
            F::CAN_LAND | F::ONLY_WHEN_DESTROYED
        )));
        assert!(!is_dockable(&destroyed(F::CAN_LAND)));
        assert_eq!(
            check(&parked(), &destroyed(F::CAN_LAND)),
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(128),
                station: false,
            })
        );
        assert_eq!(
            check(&parked(), &destroyed(F::CAN_LAND | F::ONLY_WHEN_DESTROYED)),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn a_stellar_with_the_can_land_flag_and_no_gate_is_dockable_destroyed_or_not() {
        use StellarFlags as F;
        for destroyed in [false, true] {
            let site = |flags, flags2| LandingSite {
                flags,
                flags2,
                destroyed,
                ..with_flags(0)
            };
            for (flags, flags2, dockable) in [
                (F::CAN_LAND, 0, true),
                (F::CAN_LAND | F::ONLY_WHEN_DESTROYED, 0, true),
                (F::CAN_LAND | F::STATION, 0, true),
                (0, 0, false),
                (F::ONLY_WHEN_DESTROYED, 0, false),
                (F::CAN_LAND, HYPERGATE, false),
                (F::CAN_LAND, WORMHOLE, false),
            ] {
                assert_eq!(
                    is_dockable_destroyed_or_not(&site(flags, flags2)),
                    dockable,
                    "{flags:#x} {flags2:#x} {destroyed}"
                );
            }
        }
    }

    /// `_FindNearestStellar` (@0x276c-0x2888) tests the can-land flag
    /// only, not whether the stellar is destroyed.
    #[test]
    fn the_nearest_stellar_counts_a_destroyed_one_as_find_nearest_stellar_does() {
        let sites = [
            LandingSite {
                destroyed: true,
                ..site(128, 0.0, 10.0)
            },
            site(129, 0.0, 300.0),
            LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
                destroyed: true,
                ..site(130, 0.0, 500.0)
            },
        ];
        assert_eq!(nearest_stellar(Vec2::ZERO, &sites), Some(StellarId(128)));
        assert_eq!(
            nearest_stellar(Vec2::new(0.0, 450.0), &sites),
            Some(StellarId(130))
        );
    }

    #[test]
    fn dockable_means_landable_and_no_gate() {
        let gate = |flags2| LandingSite {
            flags2,
            ..with_flags(StellarFlags::CAN_LAND)
        };
        assert!(is_dockable(&with_flags(StellarFlags::CAN_LAND)));
        assert!(!is_dockable(&with_flags(0)), "not landable");
        assert!(
            !is_dockable(&with_flags(
                StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED
            )),
            "not landable"
        );
        assert!(!is_dockable(&gate(HYPERGATE)), "a hypergate");
        assert!(!is_dockable(&gate(WORMHOLE)), "a wormhole");
    }

    #[test]
    fn a_hail_is_answered_by_a_dockable_inhabited_stellar_alone() {
        let gate = |flags2| LandingSite {
            flags2,
            ..with_flags(StellarFlags::CAN_LAND)
        };
        assert!(answers_hail(&with_flags(StellarFlags::CAN_LAND)));
        assert!(!answers_hail(&with_flags(0)), "not landable");
        assert!(
            !answers_hail(&LandingSite {
                destroyed: true,
                ..with_flags(StellarFlags::CAN_LAND)
            }),
            "destroyed"
        );
        assert!(
            answers_hail(&LandingSite {
                destroyed: true,
                ..with_flags(StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED)
            }),
            "landable only once destroyed, and destroyed"
        );
        assert!(
            !answers_hail(&with_flags(
                StellarFlags::CAN_LAND | StellarFlags::UNINHABITED
            )),
            "uninhabited"
        );
        assert!(!answers_hail(&gate(HYPERGATE)), "a hypergate");
        assert!(!answers_hail(&gate(WORMHOLE)), "a wormhole");
    }

    /// `_FindNearestStellar` (@0x276c-0x2888) skips a stellar without the
    /// can-land flag (@0x27cd), and nothing else for its flags.
    #[test]
    fn the_nearest_stellar_skips_only_those_without_the_can_land_flag() {
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
            nearest_stellar(Vec2::ZERO, &sites),
            Some(StellarId(130)),
            "landable only once destroyed, and intact"
        );
        assert_eq!(
            nearest_stellar(Vec2::new(450.0, 0.0), &sites),
            Some(StellarId(128))
        );
        assert_eq!(
            nearest_stellar_station(Vec2::new(-250.0, 0.0), &sites),
            Some((StellarId(131), true)),
            "a station counts, and is said to be one"
        );
        assert_eq!(
            nearest_stellar_station(Vec2::new(450.0, 0.0), &sites),
            Some((StellarId(128), false))
        );
        assert_eq!(nearest_stellar_station(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn with_no_can_land_flag_there_is_no_nearest_stellar() {
        assert_eq!(nearest_stellar(Vec2::ZERO, &[]), None);
        assert_eq!(nearest_stellar(Vec2::ZERO, &[with_flags(0)]), None);
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &[with_flags(StellarFlags::ONLY_WHEN_DESTROYED)]),
            None
        );
    }

    /// The strict comparison at @0x285a-0x285d.
    #[test]
    fn equally_near_stellars_go_to_the_earlier_in_nav_order() {
        let sites = [site(131, 0.0, 100.0), site(128, 0.0, -100.0)];
        assert_eq!(nearest_stellar(Vec2::ZERO, &sites), Some(StellarId(131)));
        let sites = [site(128, 0.0, -100.0), site(131, 0.0, 100.0)];
        assert_eq!(nearest_stellar(Vec2::ZERO, &sites), Some(StellarId(128)));
    }

    #[test]
    fn a_slow_ship_over_a_landable_stellar_lands_on_it() {
        let target = site(128, 10.0, -20.0);
        assert_eq!(
            check(&ship(5.0, -15.0, 0.5, -0.5), &target),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn too_far_names_the_target() {
        assert_eq!(
            check(&parked(), &site(129, 0.0, -300.0)),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            })
        );
        // 100 wide: exactly on the strict 175 box's edge.
        let station = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
            ..site(131, 0.0, 175.0)
        };
        assert_eq!(
            check(&parked(), &station),
            Err(LandingRefusal::TooFar {
                stellar: StellarId(131),
                station: true,
            })
        );
    }

    #[test]
    fn exactly_at_the_radius_is_over_it_and_just_beyond_is_not() {
        let target = site(128, 0.0, 0.0);
        let circle = LandingReach::Circle(landing_radius(&target));
        let at = |x, y| check_landing(&ship(x, y, 0.0, 0.0), &target, CLEARED, circle);
        assert_eq!(at(30.0, 40.0), Ok(StellarId(128)));
        assert!(matches!(
            at(30.0, 40.01),
            Err(LandingRefusal::TooFar { .. })
        ));
    }

    #[test]
    fn exactly_at_the_box_edge_is_too_far_and_just_inside_lands() {
        // 100 wide: a reach of 175 on each axis.
        let target = site(128, 0.0, 0.0);
        assert_eq!(engine_reach(&target), LandingReach::Box(175.0));
        for (x, y) in [(174.9, 0.0), (0.0, -174.9), (-174.9, 174.9)] {
            assert_eq!(
                check(&ship(x, y, 0.0, 0.0), &target),
                Ok(StellarId(128)),
                "({x}, {y})"
            );
        }
        for (x, y) in [(175.0, 0.0), (0.0, -175.0), (175.0, 175.0), (-175.0, 10.0)] {
            assert!(
                matches!(
                    check(&ship(x, y, 0.0, 0.0), &target),
                    Err(LandingRefusal::TooFar { .. })
                ),
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn the_same_reach_decides_too_fast_and_too_far() {
        let target = site(128, 0.0, 0.0);
        let reach = LandingReach::Box(175.0);
        let fast = |y| check_landing(&ship(0.0, y, 3.0, 0.0), &target, CLEARED, reach);
        assert!(matches!(fast(174.0), Err(LandingRefusal::TooFast { .. })));
        assert!(matches!(fast(175.0), Err(LandingRefusal::TooFar { .. })));
        // Outside today's 50 circle but inside the box: too fast, not too
        // far.
        assert!(matches!(fast(60.0), Err(LandingRefusal::TooFast { .. })));
        let circle = LandingReach::Circle(50.0);
        let fast = |y| check_landing(&ship(0.0, y, 3.0, 0.0), &target, CLEARED, circle);
        assert!(matches!(fast(50.0), Err(LandingRefusal::TooFast { .. })));
        assert!(matches!(fast(60.0), Err(LandingRefusal::TooFar { .. })));
    }

    #[test]
    fn a_box_holds_strictly_inside_on_each_axis() {
        let reach = LandingReach::Box(175.0);
        assert!(reach.holds(Vec2::new(174.9, -174.9)));
        assert!(!reach.holds(Vec2::new(175.0, 0.0)), "strict on x");
        assert!(!reach.holds(Vec2::new(0.0, -175.0)), "strict on y");
        assert!(!reach.holds(Vec2::new(-175.0, 0.0)), "either sign");
        assert!(!reach.holds(Vec2::new(0.0, 175.0)), "either sign");
        assert!(!reach.holds(Vec2::new(10.0, 180.0)), "x inside, y not");
        assert!(!reach.holds(Vec2::new(180.0, 10.0)), "y inside, x not");
        assert!(
            reach.holds(Vec2::new(170.0, 170.0)),
            "a diagonal outside a 175 circle"
        );
    }

    #[test]
    fn a_circle_holds_at_most_its_radius() {
        let reach = LandingReach::Circle(50.0);
        assert!(reach.holds(Vec2::new(30.0, 40.0)));
        assert!(reach.holds(Vec2::new(-30.0, -40.0)));
        assert!(!reach.holds(Vec2::new(30.0, 40.01)));
        assert!(!reach.holds(Vec2::new(40.0, 40.0)), "inside the box of 50");
    }

    fn sized(width: u32, height: u32) -> LandingSite {
        LandingSite {
            frame_size: Some((width, height)),
            ..site(128, 0.0, 0.0)
        }
    }

    fn shown(frame: bool, type_width: TypeWidth) -> StellarShown {
        StellarShown { frame, type_width }
    }

    #[test]
    fn by_the_engine_the_reach_is_the_type_width_times_1_75_truncated() {
        let engine = |site: &LandingSite, shown| landing_reach(site, shown, RuleSource::Engine);
        let box_of = LandingReach::Box;
        assert_eq!(engine(&sized(100, 60), LOADED), box_of(175.0));
        assert_eq!(engine(&sized(15, 15), LOADED), box_of(26.0), "26.25");
        assert_eq!(engine(&sized(140, 85), LOADED), box_of(245.0), "the width");
        assert_eq!(
            engine(&sized(85, 140), LOADED),
            box_of(148.0),
            "not the height"
        );
        assert_eq!(engine(&sized(325, 325), LOADED), box_of(568.0), "568.75");
    }

    #[test]
    fn by_the_engine_an_unloaded_type_is_150_wide_and_out_of_range_32() {
        let engine = |site: &LandingSite, shown| landing_reach(site, shown, RuleSource::Engine);
        let box_of = LandingReach::Box;
        for site in [sized(100, 60), sized(200, 60)] {
            assert_eq!(
                engine(&site, shown(true, TypeWidth::Unloaded)),
                box_of(262.0),
                "262.5"
            );
            assert_eq!(
                engine(&site, shown(true, TypeWidth::OutOfRange)),
                box_of(56.0)
            );
        }
        let unknown = LandingSite {
            frame_size: None,
            ..site(128, 0.0, 0.0)
        };
        assert_eq!(engine(&unknown, LOADED), box_of(262.0));
        assert_eq!(
            engine(&unknown, shown(true, TypeWidth::OutOfRange)),
            box_of(56.0)
        );
    }

    #[test]
    fn a_stellars_width_is_its_type_frames_or_150_unloaded_or_32_out_of_range() {
        assert_eq!(stellar_width(&sized(140, 85), TypeWidth::Frame), 140);
        assert_eq!(stellar_width(&sized(140, 85), TypeWidth::Unloaded), 150);
        assert_eq!(stellar_width(&sized(140, 85), TypeWidth::OutOfRange), 32);
        let unknown = LandingSite {
            frame_size: None,
            ..site(128, 0.0, 0.0)
        };
        assert_eq!(stellar_width(&unknown, TypeWidth::Frame), 150);
        assert_eq!(stellar_width(&unknown, TypeWidth::OutOfRange), 32);
    }

    #[test]
    fn by_the_engine_a_slot_with_no_frame_reaches_75_whatever_the_width() {
        for type_width in [TypeWidth::Frame, TypeWidth::OutOfRange, TypeWidth::Unloaded] {
            for site in [sized(15, 15), sized(100, 60), sized(325, 325)] {
                assert_eq!(
                    landing_reach(&site, shown(false, type_width), RuleSource::Engine),
                    LandingReach::Box(75.0)
                );
            }
        }
    }

    #[test]
    fn by_the_bible_the_reach_is_always_todays_circle() {
        let unknown = LandingSite {
            frame_size: None,
            ..site(128, 0.0, 0.0)
        };
        for (site, radius) in [
            (sized(100, 60), 50.0),
            (sized(140, 185), 92.5),
            (sized(200, 60), 100.0),
            (unknown, 32.0),
        ] {
            for frame in [true, false] {
                for type_width in [TypeWidth::Frame, TypeWidth::OutOfRange, TypeWidth::Unloaded] {
                    assert_eq!(
                        landing_reach(&site, shown(frame, type_width), RuleSource::Bible),
                        LandingReach::Circle(radius),
                        "{:?} {frame} {type_width:?}",
                        site.frame_size
                    );
                }
            }
        }
    }

    #[test]
    fn a_ship_over_the_target_lands_on_it_even_where_stellars_overlap() {
        // 129's centre is nearer, but 128 is the target and the ship is
        // within its radius too.
        let target = site(128, 40.0, 0.0);
        assert_eq!(check(&parked(), &target), Ok(StellarId(128)));
    }

    #[test]
    fn a_stellar_without_the_can_land_flag_is_not_landable() {
        for (flags, station) in [(0, false), (StellarFlags::STATION, true)] {
            assert_eq!(
                check(&parked(), &with_flags(flags)),
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
            check(&parked(), &with_flags(flags)),
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
                ask(&needing(min_status, can_land), GOVT, |_| 0),
                Err(LandingRefusal::Denied {
                    stellar: StellarId(128),
                    station: false,
                    min_status,
                })
            );
        }
        let station = can_land | StellarFlags::STATION;
        assert_eq!(
            ask(&needing(5, station), GOVT, |_| 4),
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
                ask(&needing(min_status, can_land), GOVT, |_| record),
                Ok(StellarId(128)),
                "{min_status} {record}"
            );
        }
    }

    #[test]
    fn an_uninhabited_stellar_ignores_min_status() {
        let flags = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
        assert_eq!(ask(&needing(32767, flags), None, |_| 0), Ok(StellarId(128)));
        // Even under a government whose record of the pilot is far below.
        let governed = LandingSite {
            govt: Some(A),
            ..needing(32767, flags)
        };
        assert_eq!(ask(&governed, Some(B), |_| -32768), Ok(StellarId(128)));
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
        let land = |site, system| ask(&governed_by(site), system, record);
        assert_eq!(land(Some(A), Some(B)), Ok(StellarId(128)));
        assert_eq!(land(Some(B), Some(A)), denied());
        assert_eq!(land(Some(A), None), Ok(StellarId(128)));
        assert_eq!(land(Some(B), None), denied());
    }

    #[test]
    fn a_stellar_without_a_government_falls_back_to_its_systems() {
        let land = |system| ask(&governed_by(None), system, record);
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
        assert_eq!(ask(&needs(0), None, never), Ok(StellarId(128)));
        assert_eq!(
            ask(&needs(1), None, never),
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
            check(&fast, &target),
            Err(LandingRefusal::TooFast {
                stellar: StellarId(128),
                station: false,
                speed,
            })
        );
        let station = with_flags(StellarFlags::CAN_LAND | StellarFlags::STATION);
        assert!(matches!(
            check(&fast, &station),
            Err(LandingRefusal::TooFast { station: true, .. })
        ));
    }

    #[test]
    fn exactly_the_landing_speed_lands() {
        let target = site(128, 0.0, 0.0);
        assert_eq!(
            check(&ship(0.0, 0.0, 0.0, 1.0), &target),
            Ok(StellarId(128))
        );
        assert_eq!(
            check(&ship(0.0, 0.0, -1.0, 0.0), &target),
            Ok(StellarId(128))
        );
    }

    #[test]
    fn each_refusal_applies_before_the_next() {
        // Not landable, too far and too fast at once: not landable.
        let far_and_fast = ship(0.0, 500.0, 5.0, 0.0);
        assert!(matches!(
            check(&far_and_fast, &needing(10, 0)),
            Err(LandingRefusal::NotLandable { .. })
        ));
        // Too far and too fast: too far.
        assert!(matches!(
            check(&far_and_fast, &needing(10, StellarFlags::CAN_LAND)),
            Err(LandingRefusal::TooFar { .. })
        ));
        // Too fast over it: too fast.
        assert!(matches!(
            check(
                &ship(0.0, 0.0, 5.0, 0.0),
                &needing(10, StellarFlags::CAN_LAND)
            ),
            Err(LandingRefusal::TooFast { .. })
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

    /// L pressed with `target` of `sites` selected and `dock` requested,
    /// for a ship `player` among `sites`, traffic control clearing as
    /// `clear` says, and every stellar held by the player's system.
    fn press_with(
        target: Option<i16>,
        dock: Option<DockRequest>,
        player: &ShipState,
        sites: &[LandingSite],
        clear: bool,
    ) -> DockPress {
        let target = target.map(|id| {
            sites
                .iter()
                .find(|site| site.id == StellarId(id))
                .expect("a target among the sites")
        });
        land_or_select(
            target,
            dock,
            player,
            sites,
            |_| clear,
            |_| true,
            engine_reach,
        )
    }

    /// L pressed with `target` selected and nothing requested, for a ship
    /// `player` among `sites`, traffic control clearing.
    fn press(target: Option<i16>, player: &ShipState, sites: &[LandingSite]) -> DockPress {
        press_with(target, None, player, sites, true)
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn request(stellar: i16, permission: i16) -> Option<DockRequest> {
        Some(DockRequest {
            stellar: StellarId(stellar),
            permission,
        })
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn requested(stellar: i16, station: bool) -> Result<LandOutcome, LandingRefusal> {
        Ok(LandOutcome::Requested {
            stellar: StellarId(stellar),
            station,
        })
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn landed(stellar: i16) -> Result<LandOutcome, LandingRefusal> {
        Ok(LandOutcome::Landed(StellarId(stellar)))
    }

    // Wrapped to compare with what a press gives.
    #[allow(clippy::unnecessary_wraps)]
    fn too_far(stellar: i16) -> Result<LandOutcome, LandingRefusal> {
        Err(LandingRefusal::TooFar {
            stellar: StellarId(stellar),
            station: false,
        })
    }

    #[test]
    fn with_no_target_l_selects_the_nearest_stellar_and_requests_it() {
        // Even at rest over it, it does not land.
        assert_eq!(
            press(None, &parked(), &system()),
            DockPress {
                selected: Some(StellarId(128)),
                dock: request(128, 0x2ed),
                said: None,
                outcome: requested(128, false),
            }
        );
        assert_eq!(
            press(None, &ship(380.0, 0.0, 9.0, 0.0), &system()),
            DockPress {
                selected: Some(StellarId(130)),
                dock: request(130, 0x2ed),
                said: None,
                outcome: requested(130, true),
            }
        );
    }

    #[test]
    fn a_held_target_no_longer_in_the_system_is_refused_as_not_landable() {
        // The held stellar is never swapped for the nearest
        // (`_HandlePlayer` @0x6a8ed): its owner and active rechecks
        // (@0x6699a-0x669aa) refuse it.
        let sites = system();
        let gone = site(140, 0.0, 0.0);
        let pressed = land_or_select(
            Some(&gone),
            request(128, 0x2ed),
            &parked(),
            &sites,
            |_| panic!("traffic control is asked"),
            |site| sites.contains(site),
            engine_reach,
        );
        assert_eq!(
            pressed,
            DockPress {
                selected: None,
                dock: request(128, 0x2ed),
                said: None,
                outcome: Err(LandingRefusal::NotLandable {
                    stellar: StellarId(140),
                    station: false,
                }),
            }
        );
    }

    #[test]
    fn a_target_its_system_no_longer_holds_is_refused_as_not_landable_with_a_beep() {
        let sites = system();
        let pressed = land_or_select(
            Some(&sites[2]),
            request(130, CLEARED),
            &ship(400.0, 0.0, 0.0, 0.0),
            &sites,
            |_| panic!("traffic control is asked"),
            |site| site.id != StellarId(130),
            engine_reach,
        );
        let refusal = LandingRefusal::NotLandable {
            stellar: StellarId(130),
            station: true,
        };
        assert_eq!(
            pressed,
            DockPress {
                selected: None,
                dock: request(130, CLEARED),
                said: None,
                outcome: Err(refusal),
            },
            "after the landability check, before the attempt"
        );
        assert!(refusal.beeps());
    }

    #[test]
    fn a_nearest_pick_its_system_no_longer_holds_is_selected_then_refused() {
        let sites = system();
        let pressed = land_or_select(
            None,
            None,
            &parked(),
            &sites,
            |_| panic!("traffic control is asked"),
            |site| site.id != StellarId(128),
            engine_reach,
        );
        assert_eq!(
            pressed,
            DockPress {
                selected: Some(StellarId(128)),
                dock: None,
                said: None,
                outcome: Err(LandingRefusal::NotLandable {
                    stellar: StellarId(128),
                    station: false,
                }),
            }
        );
    }

    #[test]
    fn a_request_away_from_the_stellar_starts_at_0_and_says_its_line() {
        assert_eq!(
            press(Some(129), &parked(), &system()),
            DockPress {
                selected: None,
                dock: request(129, 0),
                said: Some(DockSaid::Requested),
                outcome: requested(129, false),
            }
        );
    }

    #[test]
    fn a_request_is_silent_within_249_on_both_axes() {
        let at = |x, y| [site(128, x, y)];
        for (x, y) in [(249.9, -249.9), (-249.9, 249.9), (0.0, 0.0)] {
            assert_eq!(
                press(Some(128), &parked(), &at(x, y)),
                DockPress {
                    selected: None,
                    dock: request(128, 0x2ed),
                    said: None,
                    outcome: requested(128, false),
                },
                "({x}, {y})"
            );
        }
        for (x, y) in [(250.0, 0.0), (0.0, 250.0), (-250.0, 10.0), (10.0, -250.0)] {
            assert_eq!(
                press(Some(128), &parked(), &at(x, y)),
                DockPress {
                    selected: None,
                    dock: request(128, 0),
                    said: Some(DockSaid::Requested),
                    outcome: requested(128, false),
                },
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn traffic_control_denying_drops_the_request() {
        assert_eq!(
            press_with(None, request(129, 0x500), &parked(), &system(), false),
            DockPress {
                selected: Some(StellarId(128)),
                dock: None,
                said: None,
                outcome: Err(LandingRefusal::Denied {
                    stellar: StellarId(128),
                    station: false,
                    min_status: -32767,
                }),
            }
        );
        let strict = [needing(5, StellarFlags::CAN_LAND | StellarFlags::STATION)];
        assert_eq!(
            press_with(Some(128), None, &ship(0.0, 900.0, 0.0, 0.0), &strict, false).outcome,
            Err(LandingRefusal::Denied {
                stellar: StellarId(128),
                station: true,
                min_status: 5,
            })
        );
    }

    #[test]
    fn a_request_on_another_stellar_is_replaced_by_this_ones() {
        for permission in [0, 0x2ed, 0x2ee, 0x500] {
            let pressed = press_with(
                Some(128),
                request(129, permission),
                &parked(),
                &system(),
                true,
            );
            assert_eq!(pressed.dock, request(128, 0x2ed), "{permission:#x}");
            assert_eq!(pressed.outcome, requested(128, false));
        }
    }

    #[test]
    fn the_requested_stellar_lands_only_on_a_counter_past_0x2ed() {
        let over = |permission| {
            press_with(
                Some(128),
                request(128, permission),
                &parked(),
                &system(),
                false,
            )
        };
        for permission in [0, 0x2ed] {
            assert_eq!(
                over(permission),
                DockPress {
                    selected: None,
                    dock: request(128, permission),
                    said: None,
                    outcome: too_far(128),
                },
                "{permission:#x}: kept, traffic control not asked"
            );
        }
        for permission in [0x2ee, 0x7ff] {
            assert_eq!(
                over(permission),
                DockPress {
                    selected: None,
                    dock: None,
                    said: None,
                    outcome: landed(128),
                },
                "{permission:#x}"
            );
        }
    }

    #[test]
    fn a_requested_stellar_is_too_fast_only_within_the_reach_and_past_0x2ed() {
        let fast = |y, permission| {
            press_with(
                Some(128),
                request(128, permission),
                &ship(0.0, y, 3.0, 0.0),
                &system(),
                true,
            )
        };
        let pressed = fast(174.0, 0x2ee);
        assert!(
            matches!(pressed.outcome, Err(LandingRefusal::TooFast { .. })),
            "{pressed:?}"
        );
        assert_eq!(pressed.dock, request(128, 0x2ee), "kept");
        assert_eq!(fast(175.0, 0x2ee).outcome, too_far(128));
        assert_eq!(fast(0.0, 0x2ed).outcome, too_far(128));
        // At rest, but outside the reach.
        assert_eq!(
            press_with(
                Some(128),
                request(128, 0x2ee),
                &ship(0.0, 175.0, 0.0, 0.0),
                &system(),
                true
            )
            .outcome,
            too_far(128)
        );
    }

    #[test]
    fn with_a_requested_target_l_lands_on_it_not_on_the_nearest() {
        let over_129 = ship(0.0, -280.0, 0.0, 0.5);
        assert_eq!(
            press_with(Some(129), request(129, 0x2ee), &over_129, &system(), true).outcome,
            landed(129)
        );
        // Over 128, with 129 selected: too far from 129.
        assert_eq!(
            press_with(Some(129), request(129, 0x2ee), &parked(), &system(), true).outcome,
            too_far(129)
        );
    }

    /// An uninhabited planet 128 at (`x`, `y`) that would deny clearance.
    fn uninhabited(x: f32, y: f32) -> [LandingSite; 1] {
        [LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..needing(32767, 0)
        }]
        .map(|site| LandingSite {
            position: Vec2::new(x, y),
            ..site
        })
    }

    #[test]
    fn an_uninhabited_stellar_says_no_response_and_lands_in_the_same_press() {
        assert_eq!(
            press_with(None, None, &parked(), &uninhabited(0.0, 0.0), false),
            DockPress {
                selected: Some(StellarId(128)),
                dock: None,
                said: Some(DockSaid::NoResponse),
                outcome: landed(128),
            }
        );
        // Away from it: "No response.", then too far, the request at
        // 0x2ee, whatever it was.
        for dock in [
            None,
            request(128, 0),
            request(128, 0x500),
            request(129, 0x2ee),
        ] {
            assert_eq!(
                press_with(Some(128), dock, &parked(), &uninhabited(0.0, 900.0), false),
                DockPress {
                    selected: None,
                    dock: request(128, 0x2ee),
                    said: Some(DockSaid::NoResponse),
                    outcome: too_far(128),
                },
                "{dock:?}"
            );
        }
    }

    #[test]
    fn a_target_that_cannot_be_landed_on_refuses_before_anything_and_keeps_the_request() {
        let sites = [
            site(128, 0.0, 0.0),
            LandingSite {
                flags: StellarFlags::STATION,
                ..site(129, 5000.0, 5000.0)
            },
            LandingSite {
                flags: StellarFlags::UNINHABITED,
                ..site(132, 0.0, 0.0)
            },
        ];
        // Far from it and fast, and over a landable one: still too hostile.
        let far_and_fast = ship(0.0, 0.0, 9.0, 0.0);
        assert_eq!(
            press_with(Some(129), request(129, 0x2ee), &far_and_fast, &sites, true),
            DockPress {
                selected: None,
                dock: request(129, 0x2ee),
                said: None,
                outcome: Err(LandingRefusal::NotLandable {
                    stellar: StellarId(129),
                    station: true,
                }),
            }
        );
        // Uninhabited, but not landable: no "No response.".
        assert_eq!(
            press_with(Some(132), request(128, 0x2ee), &parked(), &sites, true),
            DockPress {
                selected: None,
                dock: request(128, 0x2ee),
                said: None,
                outcome: Err(LandingRefusal::NotLandable {
                    stellar: StellarId(132),
                    station: false,
                }),
            }
        );
        let only_when_destroyed = [LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::ONLY_WHEN_DESTROYED,
            ..site(131, -900.0, 0.0)
        }];
        assert_eq!(
            press(Some(131), &parked(), &only_when_destroyed).outcome,
            Err(LandingRefusal::NotLandable {
                stellar: StellarId(131),
                station: false,
            })
        );
    }

    /// With nothing selected, `_FindNearestStellar` gives -1, so the land
    /// key makes no dock request and names no stellar (@0x6a94c).
    #[test]
    fn with_none_landable_l_selects_nothing_and_makes_no_request() {
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
            press_with(None, request(128, 0x2ee), &parked(), &sites, true),
            DockPress {
                selected: None,
                dock: request(128, 0x2ee),
                said: None,
                outcome: Err(LandingRefusal::NothingLandable),
            }
        );
    }

    /// With no target the land key selects `_FindNearestStellar`'s pick
    /// (@0x6a8ed-0x6a932), a destroyed one included (@0x276c-0x2888), and
    /// the dock request refuses it as not landable (@0x66977 → 0x67cda).
    #[test]
    fn with_no_target_l_selects_a_destroyed_stellar_and_says_it_cannot_land() {
        let sites = [LandingSite {
            destroyed: true,
            ..site(128, 0.0, 0.0)
        }];
        let refused = Err(LandingRefusal::NotLandable {
            stellar: StellarId(128),
            station: false,
        });
        assert_eq!(
            press(None, &parked(), &sites),
            DockPress {
                selected: Some(StellarId(128)),
                dock: None,
                said: None,
                outcome: refused,
            }
        );
        assert_eq!(
            press_with(None, request(129, 0x500), &parked(), &sites, true),
            DockPress {
                selected: Some(StellarId(128)),
                dock: request(129, 0x500),
                said: None,
                outcome: refused,
            },
            "the request is kept"
        );
    }

    /// No stellars, so nothing to select: no dock request (@0x6a94c).
    #[test]
    fn with_no_stellars_l_refuses() {
        assert_eq!(
            press(None, &parked(), &[]).outcome,
            Err(LandingRefusal::NoStellars)
        );
    }

    // Clearance.

    /// A planet of government 140 needing `min_status`.
    fn governed(min_status: i16) -> LandingSite {
        LandingSite {
            govt: Some(A),
            ..needing(min_status, StellarFlags::CAN_LAND)
        }
    }

    /// A pilot whose record is `record`, who meets `Require`, of no
    /// mission there and unblessed.
    fn standing(record: i16) -> Standing {
        Standing {
            record,
            meets_require: true,
            ..Standing::default()
        }
    }

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;

    #[test]
    fn a_record_at_or_above_min_status_is_cleared() {
        for source in RuleSource::ALL {
            assert!(cleared(&governed(5), standing(5), source));
            assert!(cleared(&governed(5), standing(6), source));
            assert!(!cleared(&governed(5), standing(4), source));
            assert!(cleared(&governed(-3), standing(-3), source));
        }
        assert_eq!(Standing::default().record, 0);
        assert!(!Standing::default().meets_require);
    }

    #[test]
    fn a_min_status_of_32767_never_clears_by_record() {
        for source in RuleSource::ALL {
            assert!(
                !cleared(&governed(32767), standing(32767), source),
                "@0x67915 {source:?}"
            );
            assert!(cleared(&governed(32766), standing(32767), source));
        }
    }

    #[test]
    fn a_min_status_of_minus_32767_clears_any_record() {
        for source in RuleSource::ALL {
            assert!(
                cleared(&governed(-32767), standing(-32768), source),
                "@0x67909 {source:?}"
            );
            assert!(!cleared(&governed(-32766), standing(-32768), source));
        }
    }

    #[test]
    fn a_dominated_stellar_is_cleared_whatever_the_record() {
        let dominated = LandingSite {
            flags2: crate::market::DOMINATED,
            ..governed(32767)
        };
        for source in RuleSource::ALL {
            assert!(cleared(&dominated, standing(-32768), source), "@0x6792d");
        }
    }

    #[test]
    fn an_unmet_require_refuses_a_governed_stellar_even_with_the_record_or_dominated() {
        let unmet = Standing {
            meets_require: false,
            ..standing(10)
        };
        let dominated = LandingSite {
            flags2: crate::market::DOMINATED,
            ..governed(0)
        };
        for source in RuleSource::ALL {
            assert!(!cleared(&governed(0), unmet, source), "@0x67940");
            assert!(!cleared(&dominated, unmet, source));
        }
    }

    #[test]
    fn an_independent_stellar_ignores_require() {
        let unmet = Standing {
            meets_require: false,
            ..standing(10)
        };
        for source in RuleSource::ALL {
            assert!(cleared(&needing(0, StellarFlags::CAN_LAND), unmet, source));
            assert!(!cleared(
                &needing(11, StellarFlags::CAN_LAND),
                unmet,
                source
            ));
        }
    }

    #[test]
    fn a_mission_stellar_is_cleared_past_min_status_and_require() {
        let mission = Standing {
            record: -32768,
            meets_require: false,
            mission_stellar: true,
            blessed: false,
            granted: false,
        };
        for source in RuleSource::ALL {
            assert!(cleared(&governed(32767), mission, source), "@0x67977");
            assert!(cleared(
                &needing(32767, StellarFlags::CAN_LAND),
                mission,
                source
            ));
        }
    }

    /// A pilot whose record is `record`, blessed with landing, meeting
    /// `Require` or not.
    fn blessed(record: i16, meets_require: bool) -> Standing {
        Standing {
            record,
            meets_require,
            mission_stellar: false,
            blessed: true,
            granted: false,
        }
    }

    /// A pilot whose record is `record`, whose dock request is granted,
    /// meeting `Require` or not.
    fn granted(record: i16, meets_require: bool) -> Standing {
        Standing {
            record,
            meets_require,
            granted: true,
            ..Standing::default()
        }
    }

    #[test]
    fn a_granted_request_clears_alone_require_permitting() {
        for source in RuleSource::ALL {
            assert!(
                cleared(&governed(32767), granted(0, true), source),
                "@0x67934 {source:?}"
            );
            assert!(
                !cleared(&governed(32767), granted(0, false), source),
                "@0x67940 {source:?}"
            );
            assert!(
                cleared(
                    &needing(32767, StellarFlags::CAN_LAND),
                    granted(0, false),
                    source
                ),
                "an ungoverned stellar reads no Require {source:?}"
            );
        }
        assert!(!Standing::default().granted);
    }

    #[test]
    fn by_the_engine_a_blessing_clears_past_min_status_and_require() {
        assert!(cleared(&governed(32767), blessed(0, true), ENGINE));
        assert!(
            cleared(&governed(32767), blessed(0, false), ENGINE),
            "@0x679cd"
        );
    }

    #[test]
    fn by_the_bible_a_blessing_clears_past_min_status_but_not_require() {
        assert!(cleared(&governed(32767), blessed(0, true), BIBLE));
        assert!(!cleared(&governed(32767), blessed(0, false), BIBLE));
        assert!(!cleared(&governed(0), blessed(0, false), BIBLE));
    }

    #[test]
    fn an_independent_stellar_is_never_cleared_by_a_blessing() {
        for source in RuleSource::ALL {
            assert!(
                !cleared(
                    &needing(32767, StellarFlags::CAN_LAND),
                    blessed(0, true),
                    source
                ),
                "{source:?}"
            );
        }
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

    #[test]
    fn only_the_dock_requests_not_landable_too_far_and_too_fast_beep() {
        let (stellar, station) = (StellarId(128), false);
        let beeping = [
            LandingRefusal::NotLandable { stellar, station },
            LandingRefusal::TooFar { stellar, station },
            LandingRefusal::TooFast {
                stellar,
                station,
                speed: 2.0,
            },
        ];
        for refusal in beeping {
            assert!(refusal.beeps(), "{refusal:?}");
        }
        let silent = [
            LandingRefusal::Jumping,
            LandingRefusal::Disabled,
            LandingRefusal::NoStellars,
            LandingRefusal::NothingLandable,
            LandingRefusal::Denied {
                stellar,
                station,
                min_status: 10,
            },
        ];
        for refusal in silent {
            assert!(!refusal.beeps(), "{refusal:?}");
        }
    }
}
