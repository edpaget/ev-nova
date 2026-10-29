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
//!    ([`nearest_stellar`], `_FindNearestStellar`), station or planet, a
//!    hypergate or wormhole counting only strictly within 500 pixels by
//!    the engine's reading of
//!    [`RuleKey::GatePickRange`](crate::RuleKey::GatePickRange)
//!    (@0x2838-0x2853), which the press selects ([`DockPress::selected`])
//!    and goes on with (@0x6a8ed-0x6a956); a pick it can't land on is
//!    selected and then refused at step 2. The `nova_sim::Session` passes
//!    as `sites` every active stellar its system lists, owned there or
//!    not, which are what its selection holds, so a held pick another
//!    system owns is found and refused again (stellar-fidelity phase 14).
//!    With
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
//! 3. **The fee** (@0x669b4-0x66a80): when a fee is due ([`fee_due`])
//!    and it is more than the pilot's cash, signed (@0x669d0-0x669d6, so
//!    a pilot with exactly the fee pays it), it refuses with
//!    [`LandingRefusal::CannotPayFee`], silently, and drops the request
//!    (@0x67cc1-0x67cd0). It comes before the cloak, the disabled ship's
//!    silent press, the uninhabited stellar and the clearance, so it
//!    refuses the first press as well as the landing one. The charge
//!    ([`pay_fee`]) is the `nova_sim::Session`'s, on landing and on
//!    entering a hypergate or wormhole (@0x66db9-0x66df5).
//! 4. **Uninhabited** (@0x66b51-0x66bcd): the request becomes the
//!    stellar's at [`CLEARED`], "No response." is said
//!    ([`DockSaid::NoResponse`]), and the press goes on to 5, so it lands
//!    in the same press when it can.
//! 5. **The attempt** (@0x66bd2-0x67800): when the request is the
//!    stellar's, it lands if [`check_landing`] allows, and the request
//!    ends. Otherwise the refusal keeps the request and the target, and
//!    the player tries again (the manual: "If you're too far away, or if
//!    you're moving too fast ... your ship's computer will beep at you and
//!    you can try again").
//! 6. **The request** (@0x6788f-0x67c4b, @0x67ea3-0x67ef9): otherwise
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
//! 2. [`LandingRefusal::TooFast`]: the ship is moving too fast, by
//!    [`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed). By the
//!    engine's reading, unless |vx| and |vy| are each at most
//!    [`PER_AXIS_LANDING_SPEED`], 0.75, compared in single precision and
//!    inclusive, a NaN on either axis refused (`_FloatAbs` of +0x20 and
//!    +0x24 @0x66beb-0x66c1d, `ucomiss` against the float at 0xdd658
//!    @0x66c3c-0x66c66). By the other reading, the port's earlier check:
//!    the overall speed above [`LANDING_SPEED`], 1.0.
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
//! - The cloak refusal (@0x66a85-0x66ae9) and the silent press of a
//!   disabled or dying ship (@0x66aee-0x66b14) are the
//!   `nova_sim::Session`'s, which refuses them after the press's
//!   landability and fee with [`LandingRefusal::Cloaked`] (STR# 2002 #73,
//!   beeping `snd ` 153 at the shared tail @0x67e8f) and
//!   [`LandingRefusal::Disabled`], the cloak first.
//! - The player's AI wait (+0x4c, tested @0x66c29-0x66c36 and zeroed
//!   @0x66b6d): the player's +0x4c is never above 0, so the test always
//!   passes, and the engine keeps none.
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
//!   keeps it loaded, or a `Type` whose sheet cannot be read; and,
//!   whatever its master, a stellar whose owner is not the player's
//!   system (@0xacb9-0xacc1, before the master tests), which the dock
//!   request refuses as not landable before it reads the reach, so only
//!   NPC gate entry meets it;
//! - [`OUT_OF_RANGE_FRAME_WIDTH`], 32, for a loaded master whose `spïn`
//!   gives no frame count ([`TypeWidth::OutOfRange`]);
//!
//! and the reach is [`NO_FRAME_REACH`], 75, whatever the width, when the
//! stellar's sprite slot shows no frame. The session keeps which masters
//! are loaded and which slots are empty (see the `story` module's "Built
//! by phase 10"). By the other reading, the port's earlier circle: within
//! [`landing_radius`] of the centre.
//!
//! **NPCs land by their own rule** ([`npc_over`]), whichever reading of
//! `LandingDistance` holds, following
//! [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding). By the engine's
//! reading, `_HighLevelAIHandler` state 1's box (@0x8d90d-0x8da07): the
//! ship's offset from the stellar's centre, `(float)stellar − ship`
//! truncated to whole pixels on each axis (`cvttss2si`, then a 16-bit
//! `cwde` @0x8d929, @0x8d94b, not replicated), each at most the
//! half-width [`npc_box`], inclusive: [`NPC_BOX`], 40 (@0x8d90f), or
//! below a turn rate of [`NPC_BOX_TURN`] (`_ShipTurnRate`, the float
//! @0xdd044) trunc((9 − turn) × 8 + 32) in doubles (@0xdda90, @0xdda98,
//! @0xdd578; a 16-bit `movsx` @0x8d9b7, not replicated). So a ship 40.9
//! pixels off on each axis is over the stellar, and one 41.0 off on
//! either is not. A turn that is not a number keeps 40 (`jp`), and an
//! offset that is not a number truncates to 0x80000000, whose low word 0
//! is inside. By the other reading, the port's earlier circle: within
//! [`landing_radius`]. The speed it must be below, the brake and the
//! approach are the NPC flight's (`nova_sim::traffic::land_flight`).
//!
//! The quarter of the width, inclusive (@0x8dbdf), is state 20's box for
//! entering a hypergate or wormhole, not a landing ([`stellar_width`],
//! see `nova_sim::traffic::gate_entry`).
//! The player's speed gate is per axis in the original, built by
//! stellar-fidelity phase 6 (item 2 above).

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
///
/// It is now the other reading of
/// [`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed) for the player
/// and of [`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding) for an NPC.
pub const LANDING_SPEED: f32 = 1.0;

/// The fastest the player can land on each axis by the original, in
/// pixels a tick: the single-precision float 0.75 at 0xdd658 (bytes
/// `0000403f`, not a double), compared inclusively with |vx| and |vy| by
/// `_HandlePlayerDockRequest` (@0x66c3c-0x66c66).
pub const PER_AXIS_LANDING_SPEED: f32 = 0.75;

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
    /// check, the fee and the cloak ([`Self::Cloaked`])
    /// for a disabled ship, and before anything for a breaking-up or
    /// destroyed one (see `nova_sim::Session::land`).
    Disabled,
    /// The ship is cloaked: "Disengage cloaking device first." (STR#
    /// 2002 #73, `_DisplayComm` for 250 ticks @0x66ade), then the shared
    /// tail's beep (`_ShipIsCloaked` @0x66a8e → 0x67e67). It comes after
    /// the landability check (@0x66977-0x669ae) and the fee, and before
    /// the disabled test (@0x66aee) and the reach, speed, approach and
    /// clearance (@0x66c23 on).
    Cloaked,
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
    /// The pilot can't pay the stellar's landing `Fee`
    /// (`_HandlePlayerDockRequest` @0x669b4-0x66a80): a fee is due
    /// ([`fee_due`]) and is more than the pilot's cash. It says `STR#`
    /// 2002 #61 "You don't have enough", " ", `_creditsCString`
    /// "credits", " " and #62 "to pay the hypergate fee." at a hypergate,
    /// else #63 "to pay the docking fee." at a station, else #64 "to pay
    /// the landing fee.", for 250 ticks (`_DisplayComm` @0x67c88-0x67cbc).
    /// The dock request is dropped (@0x67cc1-0x67cd0). Silent: it plays
    /// no `_beepSnd`.
    CannotPayFee {
        /// The stellar.
        stellar: StellarId,
        /// Whether it is a station.
        station: bool,
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
    /// before `_IsDisabled`, so a disabled ship's beeps too), a cloaked
    /// ship (@0x66a8e → 0x67e67), out of
    /// reach (@0x66cfd, @0x676aa), or pressed again before the approach
    /// clears or moving too fast (@0x66c23-0x66c66, @0x676fe). The
    /// others are silent of it: [`Self::Jumping`], [`Self::NoStellars`]
    /// and [`Self::NothingLandable`] never reach the dock request
    /// (`_HandlePlayer` @0x6a8cb, @0x6a90f, and `_FindNearestStellar`
    /// giving -1 @0x6a94c), [`Self::Disabled`] goes to its bare epilogue
    /// (0x67eef), and [`Self::Denied`] plays `_beepSnd[1]` instead
    /// (@0x67ec6). [`Self::CannotPayFee`] plays nothing (@0x669b4-0x66a80).
    ///
    /// A [`Self::NotLandable`] from a press with no target beeps too: the
    /// press selects the nearest stellar with the can-land flag
    /// ([`nearest_stellar`]) and makes its dock request, which refuses it
    /// as the original's does (@0x66977 -> 0x67cda).
    #[must_use]
    pub fn beeps(&self) -> bool {
        matches!(
            self,
            Self::NotLandable { .. } | Self::Cloaked | Self::TooFar { .. } | Self::TooFast { .. }
        )
    }

    /// The stellar the refusal names, if any: the one the press was
    /// refused at, out of reach, too fast, not landable, denied or
    /// unpaid. The refusals before any stellar is picked, and the
    /// disabled and cloaked ship's, name none.
    #[must_use]
    pub fn stellar(&self) -> Option<StellarId> {
        match *self {
            Self::TooFar { stellar, .. }
            | Self::TooFast { stellar, .. }
            | Self::NotLandable { stellar, .. }
            | Self::Denied { stellar, .. }
            | Self::CannotPayFee { stellar, .. } => Some(stellar),
            Self::Jumping
            | Self::NoStellars
            | Self::NothingLandable
            | Self::Disabled
            | Self::Cloaked => None,
        }
    }
}

/// The disc a stellar's frame of `(width, height)` shows: half its larger
/// side. The landing radius takes it from the `Type` sheet's frame
/// ([`landing_radius`]); the shots take it from the frame the stellar
/// draws, which may be its sprite slot's when no `Type` sheet can be read
/// (`nova_sim::Session`'s stellar targets).
#[must_use]
pub fn frame_radius((width, height): (u32, u32)) -> f32 {
    width.max(height) as f32 / 2.0
}

/// The [`frame_radius`] of `site`'s `Type` sheet's frame, which is what
/// the player sees as its disc, or [`UNKNOWN_STELLAR_RADIUS`] without a
/// sprite: the landing radius of [`RuleKey::LandingDistance`]'s other
/// reading and the radius NPCs land within.
///
/// [`RuleKey::LandingDistance`]: crate::RuleKey::LandingDistance
#[must_use]
pub fn landing_radius(site: &LandingSite) -> f32 {
    site.frame_size.map_or(UNKNOWN_STELLAR_RADIUS, frame_radius)
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

/// The half-width of an NPC's landing box from a turn rate of
/// [`NPC_BOX_TURN`] up, in pixels (`mov $0x28, %ebx`, @0x8d90f).
pub const NPC_BOX: f32 = 40.0;
/// The turn rate below which an NPC's landing box widens (the float
/// @0xdd044).
pub const NPC_BOX_TURN: f32 = 8.0;
/// The turn rate the widening counts down from (the double @0xdda90).
pub const NPC_BOX_TOP_TURN: f64 = 9.0;
/// The pixels the box widens by for each degree of turn below
/// [`NPC_BOX_TOP_TURN`] (the double @0xdda98).
pub const NPC_BOX_PER_TURN: f64 = 8.0;
/// The widened box's base half-width (the double @0xdd578).
pub const NPC_BOX_BASE: f64 = 32.0;

/// The half-width of the box an NPC turning at `turn_rate` lands in
/// (`_HighLevelAIHandler` state 1, @0x8d90f-0x8d9a8): [`NPC_BOX`], or
/// below a turn of [`NPC_BOX_TURN`] trunc((9 − turn) × 8 + 32), in
/// doubles. A turn that is not a number keeps [`NPC_BOX`], as the
/// comparison's `jp` does.
#[must_use]
pub fn npc_box(turn_rate: f32) -> f32 {
    if turn_rate < NPC_BOX_TURN {
        // A multiply then an add, as the original's `mulsd`/`addsd`.
        let half = (NPC_BOX_TOP_TURN - f64::from(turn_rate)) * NPC_BOX_PER_TURN + NPC_BOX_BASE;
        // Truncated, as `cvttsd2si` does; its 16-bit `movsx` is not
        // replicated.
        #[allow(clippy::cast_possible_truncation)]
        let half = half.trunc() as f32;
        half
    } else {
        NPC_BOX
    }
}

/// Whether an NPC at `position`, turning at `turn_rate`, is over `site`
/// under `source`
/// ([`RuleKey::NpcLanding`](crate::RuleKey::NpcLanding)). By the engine's
/// reading, `_HighLevelAIHandler` state 1's box (@0x8d90d-0x8da07): its
/// offset from the centre truncated to whole pixels on each axis, each at
/// most [`npc_box`] of its turn, inclusive; an offset that is not a number
/// reads as 0, inside. By the other reading, the port's earlier circle:
/// within [`landing_radius`] of the centre, whatever the turn. The one
/// place either region is decided: the traffic's landing flight asks it by
/// both readings, for the landing and for the unfold.
#[must_use]
pub fn npc_over(site: &LandingSite, position: Vec2, turn_rate: f32, source: RuleSource) -> bool {
    if source == RuleSource::Bible {
        return (site.position - position).length() <= landing_radius(site);
    }
    let half = npc_box(turn_rate);
    // `cvttss2si`'s truncation; Rust gives 0 for NaN as the original's
    // 0x80000000 does in its low word. The `cwde` back from 16 bits is
    // not replicated.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    let whole = |offset: f32| (offset as i32) as f32;
    let within = |offset: f32| whole(offset).abs() <= half;
    within(site.position.x - position.x) && within(site.position.y - position.y)
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
/// ship `player`, its pilot holding `cash`, among the system's stellars
/// `sites` (in navigation order), where `cleared` says whether traffic
/// control clears the pilot to request docking at an inhabited stellar
/// ([`cleared`]), `held` says whether the player's system still holds a
/// stellar, owned and active (its owner word, +0x14, is the player's
/// system), and `reach` gives a stellar's [`LandingReach`], the speed read
/// by `speed` ([`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed))
/// and the untargeted pick's gate limit by `gates`
/// ([`RuleKey::GatePickRange`](crate::RuleKey::GatePickRange)): see the
/// module docs.
// One argument per thing the original's dock request reads.
#[allow(clippy::too_many_arguments)]
pub fn land_or_select(
    target: Option<&LandingSite>,
    dock: Option<DockRequest>,
    player: &ShipState,
    cash: i64,
    sites: &[LandingSite],
    cleared: impl Fn(&LandingSite) -> bool,
    held: impl Fn(&LandingSite) -> bool,
    reach: impl Fn(&LandingSite) -> LandingReach,
    speed: RuleSource,
    gates: RuleSource,
) -> DockPress {
    let refused = |refusal| DockPress {
        selected: None,
        dock,
        said: None,
        outcome: Err(refusal),
    };
    let (site, selected) = match target {
        Some(site) => (site, None),
        None => match nearest_stellar_site(player.position, sites, gates) {
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
    if fee_due(site).is_some_and(|fee| fee > cash) {
        return DockPress {
            selected,
            dock: None,
            said: None,
            outcome: Err(LandingRefusal::CannotPayFee { stellar, station }),
        };
    }
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
        let attempt = check_landing(player, site, request.permission, reach(site), speed);
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

/// The landing fee `site` charges, when one is due: its `Fee` when that is
/// above 0 and it is not dominated, else `None`. `_HandlePlayerDockRequest`
/// tests the same for its refusal (`Fee` @0x669b7-0x669b9, the domination
/// byte +0x46 @0x669bf-0x669c3) and its charge (@0x66dd1-0x66dd5), and
/// `_HandlePlayerDockApproach` for the greeting's fee line
/// (@0x64d87-0x64d96).
///
/// The engine knows only the always-dominated `Flags2`
/// [`DOMINATED`](crate::market::DOMINATED); domination by Demand Tribute
/// (the runtime byte) is rdm:task/planet-domination's, and this is the one
/// reader to switch when it lands.
#[must_use]
pub fn fee_due(site: &LandingSite) -> Option<i64> {
    (site.fee > 0 && site.flags2 & crate::market::DOMINATED == 0).then_some(i64::from(site.fee))
}

/// The cash a pilot holding `cash` has left after paying `fee`: their
/// difference, and 0 when that is negative (`subl` @0x66de3, the clamp
/// @0x66deb-0x66df5). The press refuses a pilot who can't pay first
/// ([`LandingRefusal::CannotPayFee`]), so the clamp is never met from it.
#[must_use]
pub fn pay_fee(cash: i64, fee: i64) -> i64 {
    (cash - fee).max(0)
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

/// The squared distance, 250000.0 (500 pixels), a hypergate or wormhole
/// must be strictly within for [`nearest_stellar`] to count it by the
/// engine's reading of
/// [`RuleKey::GatePickRange`](crate::RuleKey::GatePickRange): the float at
/// 0xdd050, compared by `ucomiss` @0x284a.
pub const GATE_PICK_RANGE_SQUARED: f32 = 250_000.0;

/// The stellar among `sites` (in navigation order) whose centre is nearest
/// `position`, as `_FindNearestStellar` (@0x276c-0x2888) picks it, the
/// gate limit read by `gates`
/// ([`RuleKey::GatePickRange`](crate::RuleKey::GatePickRange)). The whole
/// function:
///
/// - F1: it starts with no best, its best distance -1.0 (@0x2777-0x2785).
/// - F2, F3: it walks the system's sixteen `NavDefs` slots, skipping an
///   empty one and one whose stellar is not active (+0x44,
///   @0x278a-0x27c7). Which stellars those are is the caller's: `sites`
///   holds them (see `nova_sim::Session`'s pick candidates).
/// - F4: it skips a stellar without the can-land flag (`Flags` 0x01,
///   @0x27cd). Being destroyed and `Flags` 0x80 are not read, so a
///   destroyed stellar counts, and so does an intact one landable only
///   once destroyed.
/// - F5: it reads no owner (+0x14), so a stellar another visible system
///   owns counts. Refusing it is the caller's.
/// - F6: the distance is squared, `dx·dx + dy·dy` in single precision
///   (@0x27d7-0x2844), never rooted.
/// - F7: by the engine's reading, a hypergate or wormhole (`Flags2` &
///   0x3000, @0x2838-0x2841) counts only when that is strictly below
///   [`GATE_PICK_RANGE_SQUARED`] (@0x284a-0x2853); every other stellar
///   counts at any distance (@0x2848). By the other reading, a gate counts
///   at any distance.
/// - F8: the nearest wins, the earlier of any at an equal squared distance
///   (`ja` @0x285a-0x285d).
/// - F9: `None` when none counts (@0x287d).
///
/// A NaN distance, which would stick as the best (@0x285a-0x2868), can't
/// arise: positions are never NaN.
#[must_use]
pub fn nearest_stellar(
    position: Vec2,
    sites: &[LandingSite],
    gates: RuleSource,
) -> Option<StellarId> {
    nearest_stellar_site(position, sites, gates).map(|site| site.id)
}

/// The stellar [`nearest_stellar`] (`_FindNearestStellar`) names, and whether it is a station
/// (`spöb` `Flags` 0x10), as the tutorial's land hint words it
/// (`_HandlePlayer` @0x709e8-0x70a31; see `nova_sim::tutorial`).
#[must_use]
pub fn nearest_stellar_station(
    position: Vec2,
    sites: &[LandingSite],
    gates: RuleSource,
) -> Option<(StellarId, bool)> {
    nearest_stellar_site(position, sites, gates).map(|site| (site.id, is_station(site)))
}

/// The site [`nearest_stellar`] names: the one the land key selects when
/// there is no target.
fn nearest_stellar_site(
    position: Vec2,
    sites: &[LandingSite],
    gates: RuleSource,
) -> Option<&LandingSite> {
    let squared = |site: &LandingSite| (site.position - position).length_squared();
    let in_range = |site: &&LandingSite| {
        gates != RuleSource::Engine
            || GateKind::of(site.flags2).is_none()
            || squared(site) < GATE_PICK_RANGE_SQUARED
    };
    nearest(
        position,
        sites
            .iter()
            .filter(|site| has(site, StellarFlags::CAN_LAND))
            .filter(in_range),
    )
}

/// The one of `sites` whose centre is nearest `position` by squared
/// distance, the first of any equally near.
fn nearest<'a>(
    position: Vec2,
    sites: impl Iterator<Item = &'a LandingSite>,
) -> Option<&'a LandingSite> {
    let squared = |site: &LandingSite| (site.position - position).length_squared();
    sites.min_by(|a, b| squared(a).total_cmp(&squared(b)))
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
/// as is a ship outside the reach; then a ship too fast by `reading`
/// ([`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed)) is
/// [`LandingRefusal::TooFast`]: by the engine, |vx| or |vy| above
/// [`PER_AXIS_LANDING_SPEED`] or not a number (@0x66c3c-0x66c66); by the
/// other reading, the overall speed above [`LANDING_SPEED`].
pub fn check_landing(
    player: &ShipState,
    site: &LandingSite,
    permission: i16,
    reach: LandingReach,
    reading: RuleSource,
) -> Result<StellarId, LandingRefusal> {
    let (stellar, station) = (site.id, is_station(site));
    if permission <= REQUESTED_NEAR || !reach.holds(site.position - player.position) {
        return Err(LandingRefusal::TooFar { stellar, station });
    }
    if too_fast(player.velocity, reading) {
        let speed = player.velocity.length();
        return Err(LandingRefusal::TooFast {
            stellar,
            station,
            speed,
        });
    }
    Ok(stellar)
}

/// Whether a ship moving at `velocity` is too fast to land by `reading`
/// (see [`check_landing`]). By the engine, written so that a NaN axis
/// fails both compares and refuses, as the `jp` @0x66c51 and the
/// unordered `jb` @0x66c66 do.
fn too_fast(velocity: Vec2, reading: RuleSource) -> bool {
    match reading {
        RuleSource::Engine => {
            !(velocity.x.abs() <= PER_AXIS_LANDING_SPEED
                && velocity.y.abs() <= PER_AXIS_LANDING_SPEED)
        }
        RuleSource::Bible => velocity.length() > LANDING_SPEED,
    }
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

/// The services a stellar with `flags` offers, in [`Service`] order.
///
/// The trade center, outfitter, shipyard and bar each follow their own
/// flag alone, uninhabited or not, as `_DrawPlanetButtons` enables them
/// (@0x29802-0x29821) and `_DoPortDialog` answers them (@0x5fe71,
/// @0x5feb5, @0x5ff2c, @0x60115). There is no flag for the mission BBS:
/// the original offers it wherever `_canDoMissions` holds, which is "not
/// uninhabited" (@0x5fab6, @0x4f069). No stock stellar that is
/// uninhabited carries a service flag, so the difference shows only in a
/// plug-in.
#[must_use]
pub fn services(flags: u32) -> Vec<Service> {
    let missions = (flags & StellarFlags::UNINHABITED == 0).then_some(Service::MissionBbs);
    [
        (StellarFlags::TRADE_CENTER, Service::TradeCenter),
        (StellarFlags::OUTFITTER, Service::Outfitter),
        (StellarFlags::SHIPYARD, Service::Shipyard),
        (StellarFlags::BAR, Service::Bar),
    ]
    .into_iter()
    .filter(|(flag, _)| flags & flag != 0)
    .map(|(_, service)| service)
    .chain(missions)
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

    /// Cash enough for any fee the tests set.
    const RICH: i64 = 1_000_000;

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
            dead_counts_frames: true,
            exit_angle: -1,
            tribute: 0,
            gravity: 0,
            fee: 0,
            animation: crate::catalog::SiteAnimation::default(),
            defense_dude: None,
            def_count: 0,
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
        check_by(player, site, RuleSource::Engine)
    }

    /// [`check`] with the speed read by `reading`
    /// ([`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed)).
    fn check_by(
        player: &ShipState,
        site: &LandingSite,
        reading: RuleSource,
    ) -> Result<StellarId, LandingRefusal> {
        let press = land_or_select(
            Some(site),
            Some(DockRequest {
                stellar: site.id,
                permission: CLEARED,
            }),
            player,
            RICH,
            std::slice::from_ref(site),
            |_| panic!("traffic control is asked"),
            |_| true,
            engine_reach,
            reading,
            RuleSource::Engine,
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
            RICH,
            std::slice::from_ref(site),
            |site| landing_record(site, system_govt, &record) >= site.min_status,
            |_| true,
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
        );
        press.outcome.map(|outcome| match outcome {
            LandOutcome::Landed(stellar) | LandOutcome::Requested { stellar, .. } => stellar,
        })
    }

    #[test]
    fn a_refusal_names_the_stellar_it_was_refused_at_or_none() {
        let id = StellarId(130);
        for named in [
            LandingRefusal::TooFar {
                stellar: id,
                station: false,
            },
            LandingRefusal::TooFast {
                stellar: id,
                station: true,
                speed: 9.0,
            },
            LandingRefusal::NotLandable {
                stellar: id,
                station: false,
            },
            LandingRefusal::Denied {
                stellar: id,
                station: false,
                min_status: 0,
            },
            LandingRefusal::CannotPayFee {
                stellar: id,
                station: false,
            },
        ] {
            assert_eq!(named.stellar(), Some(id), "{named:?}");
        }
        for unnamed in [
            LandingRefusal::Jumping,
            LandingRefusal::NoStellars,
            LandingRefusal::NothingLandable,
            LandingRefusal::Disabled,
            LandingRefusal::Cloaked,
        ] {
            assert_eq!(unnamed.stellar(), None, "{unnamed:?}");
        }
    }

    #[test]
    fn a_stellar_the_players_system_does_not_hold_is_not_landable_before_any_reach() {
        // Over it, at rest and cleared: only the owner test refuses
        // (@0x6699a-0x669a4), before the uninhabited, request, speed and
        // reach tests.
        let sites = system();
        let uninhabited = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..sites[1]
        };
        for site in [sites[1], uninhabited] {
            let press = land_or_select(
                Some(&site),
                request(128, CLEARED),
                &parked(),
                RICH,
                std::slice::from_ref(&site),
                |_| panic!("traffic control is asked"),
                |_| false,
                |_| panic!("the reach is read"),
                RuleSource::Engine,
                RuleSource::Engine,
            );
            assert_eq!(
                press,
                DockPress {
                    selected: None,
                    dock: request(128, CLEARED),
                    said: None,
                    outcome: Err(LandingRefusal::NotLandable {
                        stellar: StellarId(128),
                        station: false,
                    }),
                }
            );
        }
        let picked = land_or_select(
            None,
            None,
            &parked(),
            RICH,
            &sites,
            |_| true,
            |site| site.id != StellarId(128),
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
        );
        assert_eq!(
            picked.selected,
            Some(StellarId(128)),
            "selected all the same"
        );
        assert!(matches!(
            picked.outcome,
            Err(LandingRefusal::NotLandable { .. })
        ));
    }

    #[test]
    fn the_named_values() {
        assert_eq!(LANDING_SPEED, 1.0);
        assert_eq!(PER_AXIS_LANDING_SPEED, 0.75);
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
    fn a_frame_disc_is_half_the_frame_s_larger_side() {
        assert_eq!(frame_radius((100, 60)), 50.0);
        assert_eq!(frame_radius((140, 185)), 92.5);
        assert_eq!(frame_radius((141, 85)), 70.5);
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
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            Some(StellarId(128))
        );
        assert_eq!(
            nearest_stellar(Vec2::new(0.0, 450.0), &sites, RuleSource::Engine),
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
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            Some(StellarId(130)),
            "landable only once destroyed, and intact"
        );
        assert_eq!(
            nearest_stellar(Vec2::new(450.0, 0.0), &sites, RuleSource::Engine),
            Some(StellarId(128))
        );
        assert_eq!(
            nearest_stellar_station(Vec2::new(-250.0, 0.0), &sites, RuleSource::Engine),
            Some((StellarId(131), true)),
            "a station counts, and is said to be one"
        );
        assert_eq!(
            nearest_stellar_station(Vec2::new(450.0, 0.0), &sites, RuleSource::Engine),
            Some((StellarId(128), false))
        );
        assert_eq!(
            nearest_stellar_station(Vec2::ZERO, &[], RuleSource::Engine),
            None
        );
    }

    #[test]
    fn with_no_can_land_flag_there_is_no_nearest_stellar() {
        assert_eq!(nearest_stellar(Vec2::ZERO, &[], RuleSource::Engine), None);
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &[with_flags(0)], RuleSource::Engine),
            None
        );
        assert_eq!(
            nearest_stellar(
                Vec2::ZERO,
                &[with_flags(StellarFlags::ONLY_WHEN_DESTROYED)],
                RuleSource::Engine
            ),
            None
        );
    }

    /// The strict comparison at @0x285a-0x285d.
    #[test]
    fn equally_near_stellars_go_to_the_earlier_in_nav_order() {
        let sites = [site(131, 0.0, 100.0), site(128, 0.0, -100.0)];
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            Some(StellarId(131))
        );
        let sites = [site(128, 0.0, -100.0), site(131, 0.0, 100.0)];
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            Some(StellarId(128))
        );
    }

    /// A wormhole at (`x`, `y`) as the stock ones are flagged: can-land
    /// and uninhabited (`Flags` 0x21), `Flags2` 0x2000.
    fn wormhole(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            flags2: WORMHOLE,
            ..site(id, x, y)
        }
    }

    /// A hypergate at (`x`, `y`): can-land (`Flags` 0x01), `Flags2` 0x1000.
    fn hypergate(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags2: HYPERGATE,
            ..site(id, x, y)
        }
    }

    /// The gates of the gate-limit tests, each built at (`x`, `y`).
    const GATES: [fn(i16, f32, f32) -> LandingSite; 2] = [wormhole, hypergate];

    /// `_FindNearestStellar` counts a gate (`Flags2` & 0x3000) only when
    /// its squared distance is strictly below 250000.0 (`ucomiss` against
    /// 0xdd050, `jae` @0x284a-0x2853): exactly 500 px away it does not
    /// count, and the planet 900 px away is picked.
    #[test]
    fn by_the_engine_a_gate_exactly_500_away_is_not_picked() {
        for gate in GATES {
            let sites = [gate(130, 300.0, 400.0), site(128, 900.0, 0.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
                Some(StellarId(128)),
                "{:?}",
                sites[0].flags2
            );
        }
    }

    /// d² = 300² + 399² = 249201, below 250000.0: the gate counts.
    #[test]
    fn by_the_engine_a_gate_just_inside_500_is_picked() {
        for gate in GATES {
            let sites = [site(128, 900.0, 0.0), gate(130, 300.0, 399.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
                Some(StellarId(130)),
                "{:?}",
                sites[1].flags2
            );
            assert_eq!(
                nearest_stellar_station(Vec2::ZERO, &sites, RuleSource::Engine),
                Some((StellarId(130), false))
            );
        }
    }

    /// With only gates 500 px or more away there is no pick, and an
    /// untargeted L selects nothing and refuses in silence.
    #[test]
    fn by_the_engine_only_far_gates_give_no_pick_and_l_refuses_nothing_landable() {
        let sites = [wormhole(130, 300.0, 400.0), hypergate(131, -600.0, 0.0)];
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            None
        );
        assert_eq!(
            nearest_stellar_station(Vec2::ZERO, &sites, RuleSource::Engine),
            None
        );
        let press = land_or_select(
            None,
            None,
            &parked(),
            RICH,
            &sites,
            |_| panic!("traffic control is asked"),
            |_| true,
            |_| panic!("the reach is read"),
            RuleSource::Engine,
            RuleSource::Engine,
        );
        assert_eq!(
            press,
            DockPress {
                selected: None,
                dock: None,
                said: None,
                outcome: Err(LandingRefusal::NothingLandable),
            }
        );
        assert!(!LandingRefusal::NothingLandable.beeps(), "silent");
    }

    /// The other reading counts a gate at any distance, the port's
    /// earlier pick.
    #[test]
    fn by_the_other_reading_a_gate_counts_at_any_distance() {
        for gate in GATES {
            let sites = [gate(130, 300.0, 400.0), site(128, 900.0, 0.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &sites, RuleSource::Bible),
                Some(StellarId(130))
            );
            let far = [site(128, 9000.0, 0.0), gate(130, 5000.0, 0.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &far, RuleSource::Bible),
                Some(StellarId(130))
            );
            assert_eq!(
                nearest_stellar_station(Vec2::ZERO, &far, RuleSource::Bible),
                Some((StellarId(130), false))
            );
        }
    }

    /// The limit is the gates' alone (`je 0x2855` @0x2848): an ordinary
    /// stellar counts at any distance by either reading.
    #[test]
    fn a_planet_far_away_counts_by_either_reading() {
        let sites = [site(128, 5000.0, 0.0), wormhole(130, 4000.0, 0.0)];
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Engine),
            Some(StellarId(128))
        );
        assert_eq!(
            nearest_stellar(
                Vec2::new(-3000.0, 0.0),
                &[site(128, 3000.0, 4000.0)],
                RuleSource::Engine
            ),
            Some(StellarId(128))
        );
        assert_eq!(
            nearest_stellar(Vec2::ZERO, &sites, RuleSource::Bible),
            Some(StellarId(130))
        );
    }

    /// The choice is on d² (F6, F8), the earlier winning only an equal
    /// d²: (300, 400) and (500, 0) are both 250000 from the origin, and
    /// (0, 499) is nearer than both. Nothing orders two stellars
    /// differently by d² than by its root (the root is monotonic), so
    /// only the ties are shown.
    #[test]
    fn of_two_stellars_the_earlier_wins_only_an_equal_squared_distance() {
        for gates in [RuleSource::Engine, RuleSource::Bible] {
            let tied = [site(128, 300.0, 400.0), site(129, 500.0, 0.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &tied, gates),
                Some(StellarId(128))
            );
            let tied = [site(129, 500.0, 0.0), site(128, 300.0, 400.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &tied, gates),
                Some(StellarId(129))
            );
            let nearer = [site(128, 300.0, 400.0), site(129, 0.0, 499.0)];
            assert_eq!(
                nearest_stellar(Vec2::ZERO, &nearer, gates),
                Some(StellarId(129))
            );
        }
    }

    /// The limit is measured from the ship, wherever it is.
    #[test]
    fn by_the_engine_the_gate_limit_is_measured_from_the_ship() {
        let from = Vec2::new(1000.0, -400.0);
        for gate in GATES {
            let sites = [site(128, 5000.0, 0.0), gate(130, 1100.0, -400.0)];
            assert_eq!(
                nearest_stellar(from, &sites, RuleSource::Engine),
                Some(StellarId(130))
            );
            let sites = [site(128, 5000.0, 0.0), gate(130, 1300.0, 0.0)];
            assert_eq!(
                nearest_stellar(from, &sites, RuleSource::Engine),
                Some(StellarId(128)),
                "exactly 500 away"
            );
        }
    }

    /// An untargeted L selects as the pick does, by each reading.
    #[test]
    fn untargeted_l_selects_by_the_gate_reading() {
        let sites = [wormhole(130, 300.0, 400.0), site(128, 900.0, 0.0)];
        let pressed = |gates| {
            land_or_select(
                None,
                None,
                &parked(),
                RICH,
                &sites,
                |_| true,
                |_| true,
                engine_reach,
                RuleSource::Engine,
                gates,
            )
            .selected
        };
        assert_eq!(pressed(RuleSource::Engine), Some(StellarId(128)));
        assert_eq!(pressed(RuleSource::Bible), Some(StellarId(130)));
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
        let at = |x, y| {
            check_landing(
                &ship(x, y, 0.0, 0.0),
                &target,
                CLEARED,
                circle,
                RuleSource::Engine,
            )
        };
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
        let fast = |y| {
            check_landing(
                &ship(0.0, y, 3.0, 0.0),
                &target,
                CLEARED,
                reach,
                RuleSource::Engine,
            )
        };
        assert!(matches!(fast(174.0), Err(LandingRefusal::TooFast { .. })));
        assert!(matches!(fast(175.0), Err(LandingRefusal::TooFar { .. })));
        // Outside today's 50 circle but inside the box: too fast, not too
        // far.
        assert!(matches!(fast(60.0), Err(LandingRefusal::TooFast { .. })));
        let circle = LandingReach::Circle(50.0);
        let fast = |y| {
            check_landing(
                &ship(0.0, y, 3.0, 0.0),
                &target,
                CLEARED,
                circle,
                RuleSource::Engine,
            )
        };
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
    fn an_npcs_box_is_40_from_a_turn_of_8_and_widens_below_it() {
        assert_eq!(NPC_BOX, 40.0);
        assert_eq!(NPC_BOX_TURN, 8.0);
        assert_eq!(NPC_BOX_TOP_TURN, 9.0);
        assert_eq!(NPC_BOX_PER_TURN, 8.0);
        assert_eq!(NPC_BOX_BASE, 32.0);
        for (turn, half) in [
            (8.0, 40.0),
            (12.0, 40.0),
            (7.99, 40.0),
            (7.5, 44.0),
            (7.0, 48.0),
            (1.0, 96.0),
            (f32::NAN, 40.0),
        ] {
            assert_eq!(npc_box(turn), half, "{turn}");
        }
    }

    #[test]
    fn by_the_engine_an_npc_is_over_a_stellar_within_its_box_in_whole_pixels() {
        let planet = site(128, 100.0, -50.0);
        let over = |dx: f32, dy: f32, turn| {
            npc_over(
                &planet,
                Vec2::new(100.0 + dx, -50.0 + dy),
                turn,
                RuleSource::Engine,
            )
        };
        for (dx, dy) in [(40.9, 40.9), (-40.9, -40.9), (40.9, -40.9), (0.0, 0.0)] {
            assert!(over(dx, dy, 8.0), "{dx} {dy}");
        }
        for (dx, dy) in [(41.0, 0.0), (-41.0, 0.0), (0.0, 41.0), (0.0, -41.0)] {
            assert!(!over(dx, dy, 8.0), "{dx} {dy}");
        }
        assert!(over(48.9, -48.9, 7.0));
        assert!(!over(49.0, 0.0, 7.0));
        assert!(!over(0.0, 49.0, 7.0));
        // Far beyond any 16-bit offset: outside, not wrapped.
        assert!(!over(1.0e9, 0.0, 8.0));
    }

    #[test]
    fn by_the_engine_an_npc_at_no_position_is_over_the_stellar() {
        let planet = site(128, 0.0, 0.0);
        for position in [
            Vec2::new(f32::NAN, f32::NAN),
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(0.0, f32::NAN),
        ] {
            assert!(npc_over(&planet, position, 8.0, RuleSource::Engine));
        }
    }

    #[test]
    fn by_the_bible_an_npc_is_over_a_stellar_within_its_landing_radius() {
        // 100 x 60: a radius of 50.
        let planet = site(128, 0.0, 0.0);
        let over = |x: f32, y: f32| npc_over(&planet, Vec2::new(x, y), 8.0, RuleSource::Bible);
        assert!(over(30.0, 40.0));
        assert!(over(-50.0, 0.0));
        assert!(!over(30.0, 40.01));
        assert!(!over(40.0, 40.0), "inside the box of 40");
        // A turn rate makes no difference to the circle.
        assert!(npc_over(
            &planet,
            Vec2::new(48.0, 0.0),
            1.0,
            RuleSource::Bible
        ));
        assert!(!npc_over(
            &planet,
            Vec2::new(60.0, 0.0),
            1.0,
            RuleSource::Bible
        ));
        // Measured from the stellar's centre, wherever it is.
        let off_centre = site(128, 500.0, -300.0);
        let at = |x: f32, y: f32| Vec2::new(500.0 + x, -300.0 + y);
        assert!(npc_over(
            &off_centre,
            at(30.0, 40.0),
            8.0,
            RuleSource::Bible
        ));
        assert!(!npc_over(
            &off_centre,
            at(30.0, 40.01),
            8.0,
            RuleSource::Bible
        ));
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

    /// [`TooFast`](LandingRefusal::TooFast) at stellar 128, a planet,
    /// for a ship moving at `velocity`.
    fn too_fast(velocity: Vec2) -> Result<StellarId, LandingRefusal> {
        Err(LandingRefusal::TooFast {
            stellar: StellarId(128),
            station: false,
            speed: velocity.length(),
        })
    }

    /// [`check_by`] over stellar 128 for a parked ship moving at
    /// (`vx`, `vy`), by `reading`.
    fn moving(vx: f32, vy: f32, reading: RuleSource) -> Result<StellarId, LandingRefusal> {
        check_by(&ship(0.0, 0.0, vx, vy), &site(128, 0.0, 0.0), reading)
    }

    #[test]
    fn each_axis_at_most_three_quarters_lands_by_the_engine() {
        let overall = Vec2::new(0.7, 0.7).length();
        assert!(overall > 0.98 && overall < LANDING_SPEED, "{overall}");
        assert_eq!(moving(0.7, 0.7, RuleSource::Engine), Ok(StellarId(128)));
        assert_eq!(
            moving(0.8, 0.0, RuleSource::Engine),
            too_fast(Vec2::new(0.8, 0.0))
        );
        assert_eq!(
            moving(0.0, -0.8, RuleSource::Engine),
            too_fast(Vec2::new(0.0, -0.8))
        );
        // The default helper reads the engine.
        assert_eq!(
            check(&ship(0.0, 0.0, 0.8, 0.0), &site(128, 0.0, 0.0)),
            too_fast(Vec2::new(0.8, 0.0))
        );
    }

    #[test]
    fn three_quarters_on_an_axis_is_inclusive_by_the_engine() {
        assert_eq!(PER_AXIS_LANDING_SPEED, 0.75);
        assert_eq!(moving(0.75, -0.75, RuleSource::Engine), Ok(StellarId(128)));
        assert_eq!(moving(-0.75, 0.75, RuleSource::Engine), Ok(StellarId(128)));
        let over = 0.75_f32.next_up();
        assert_eq!(
            moving(over, 0.0, RuleSource::Engine),
            too_fast(Vec2::new(over, 0.0))
        );
        assert_eq!(
            moving(0.0, -over, RuleSource::Engine),
            too_fast(Vec2::new(0.0, -over))
        );
        // The readings differ both ways.
        assert_eq!(moving(0.74, 0.74, RuleSource::Engine), Ok(StellarId(128)));
        assert_eq!(
            moving(0.74, 0.74, RuleSource::Bible),
            too_fast(Vec2::new(0.74, 0.74))
        );
    }

    #[test]
    fn a_velocity_that_is_not_a_number_is_too_fast_by_the_engine() {
        for (vx, vy) in [(f32::NAN, 0.0), (0.0, f32::NAN)] {
            assert!(
                matches!(
                    moving(vx, vy, RuleSource::Engine),
                    Err(LandingRefusal::TooFast { .. })
                ),
                "{vx} {vy}"
            );
            // The port's earlier check lets it land, as it did.
            assert_eq!(moving(vx, vy, RuleSource::Bible), Ok(StellarId(128)));
        }
    }

    #[test]
    fn the_other_reading_checks_the_overall_speed() {
        let bible = RuleSource::Bible;
        assert_eq!(moving(0.7, 0.7, bible), Ok(StellarId(128)));
        assert_eq!(moving(0.8, 0.0, bible), Ok(StellarId(128)));
        assert_eq!(moving(0.0, 1.0, bible), Ok(StellarId(128)));
        assert_eq!(moving(-1.0, 0.0, bible), Ok(StellarId(128)));
        assert_eq!(
            moving(0.0, 1.0_f32.next_up(), bible),
            too_fast(Vec2::new(0.0, 1.0_f32.next_up()))
        );
        assert_eq!(moving(0.6, 0.81, bible), too_fast(Vec2::new(0.6, 0.81)));
    }

    #[test]
    fn too_far_comes_before_too_fast_by_either_reading() {
        let target = site(128, 0.0, 0.0);
        for reading in RuleSource::ALL {
            let outside = ship(0.0, 500.0, 0.8, 0.0);
            assert!(
                matches!(
                    check_landing(&outside, &target, CLEARED, engine_reach(&target), reading),
                    Err(LandingRefusal::TooFar { .. })
                ),
                "{reading:?}"
            );
            let over = ship(0.0, 0.0, 0.8, 0.0);
            assert!(
                matches!(
                    check_landing(
                        &over,
                        &target,
                        REQUESTED_NEAR,
                        engine_reach(&target),
                        reading
                    ),
                    Err(LandingRefusal::TooFar { .. })
                ),
                "{reading:?}"
            );
        }
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
            RICH,
            sites,
            |_| clear,
            |_| true,
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
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
            RICH,
            &sites,
            |_| panic!("traffic control is asked"),
            |site| sites.contains(site),
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
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
            RICH,
            &sites,
            |_| panic!("traffic control is asked"),
            |site| site.id != StellarId(130),
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
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
            RICH,
            &sites,
            |_| panic!("traffic control is asked"),
            |site| site.id != StellarId(128),
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
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
    fn an_uninhabited_stellar_offers_its_flagged_services_but_no_bbs() {
        use Service::{Bar, Outfitter, Shipyard, TradeCenter};
        let uninhabited = StellarFlags::UNINHABITED;
        assert_eq!(services(uninhabited), []);
        assert_eq!(
            services(uninhabited | StellarFlags::TRADE_CENTER | StellarFlags::BAR),
            [TradeCenter, Bar]
        );
        assert_eq!(services(0xFF), [TradeCenter, Outfitter, Shipyard, Bar]);
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
            // No `PlaySound` on the fee refusal's path (@0x669b4-0x66a80,
            // `_DisplayComm` @0x67c88-0x67cbc).
            LandingRefusal::CannotPayFee { stellar, station },
        ];
        for refusal in silent {
            assert!(!refusal.beeps(), "{refusal:?}");
        }
    }

    /// `site` with this `Fee`.
    fn charging(fee: i32, site: LandingSite) -> LandingSite {
        LandingSite { fee, ..site }
    }

    #[test]
    fn a_fee_is_due_only_when_above_0_at_a_stellar_not_dominated() {
        // `Fee > 0` (@0x669b7-0x669b9) and the domination byte clear
        // (@0x669bf-0x669c3), as the charge (@0x66dd1-0x66dd5) and the
        // greeting (@0x64d87-0x64d96) read them.
        let planet = site(128, 0.0, 0.0);
        assert_eq!(fee_due(&charging(0, planet)), None);
        assert_eq!(fee_due(&charging(-5, planet)), None);
        assert_eq!(fee_due(&charging(100, planet)), Some(100));
        let dominated = LandingSite {
            flags2: crate::market::DOMINATED,
            ..charging(100, planet)
        };
        assert_eq!(fee_due(&dominated), None);
    }

    #[test]
    fn paying_a_fee_takes_it_from_the_cash_and_never_below_0() {
        // `subl` @0x66de3, then a negative cash set to 0 @0x66deb-0x66df5.
        assert_eq!(pay_fee(500, 100), 400);
        assert_eq!(pay_fee(100, 100), 0);
        assert_eq!(pay_fee(50, 100), 0);
    }

    /// L pressed by a ship parked over `site`, the system's one stellar,
    /// with `cash`, `site` selected and `dock` requested, traffic control
    /// clearing as `clear` says.
    fn pay(
        site: &LandingSite,
        cash: i64,
        dock: Option<DockRequest>,
        clear: impl Fn(&LandingSite) -> bool,
    ) -> DockPress {
        land_or_select(
            Some(site),
            dock,
            &parked(),
            cash,
            std::slice::from_ref(site),
            clear,
            |_| true,
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
        )
    }

    #[test]
    fn a_player_who_cannot_pay_the_fee_is_refused_and_the_request_dropped() {
        // `Fee > cash`, signed (@0x669d0-0x669d6): the request is dropped
        // (`_dockTarget`/`_dockPermission` = -1 @0x67cc1-0x67cd0).
        let site = charging(100, site(128, 0.0, 0.0));
        let press = pay(&site, 99, request(128, CLEARED), |_| {
            panic!("traffic control is asked")
        });
        assert_eq!(
            press,
            DockPress {
                selected: None,
                dock: None,
                said: None,
                outcome: Err(LandingRefusal::CannotPayFee {
                    stellar: StellarId(128),
                    station: false,
                }),
            }
        );
    }

    #[test]
    fn a_player_with_exactly_the_fee_lands() {
        let site = charging(100, site(128, 0.0, 0.0));
        let press = pay(&site, 100, request(128, CLEARED), |_| true);
        assert_eq!(press.outcome, landed(128));
        assert_eq!(press.dock, None);
    }

    #[test]
    fn the_fee_refuses_the_first_press_before_traffic_control_is_asked() {
        // @0x669b4 comes before the clearance block (@0x67903).
        let site = charging(100, site(128, 0.0, 0.0));
        let press = land_or_select(
            None,
            None,
            &parked(),
            0,
            std::slice::from_ref(&site),
            |_| panic!("traffic control is asked"),
            |_| true,
            |_| panic!("the reach is read"),
            RuleSource::Engine,
            RuleSource::Engine,
        );
        assert_eq!(
            press,
            DockPress {
                selected: Some(StellarId(128)),
                dock: None,
                said: None,
                outcome: Err(LandingRefusal::CannotPayFee {
                    stellar: StellarId(128),
                    station: false,
                }),
            }
        );
    }

    #[test]
    fn the_fee_refuses_an_uninhabited_stellar_before_no_response() {
        // @0x669b4 comes before the uninhabited test (@0x66b51).
        let site = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..charging(100, site(128, 0.0, 0.0))
        };
        let press = pay(&site, 0, None, |_| true);
        assert_eq!(press.said, None);
        assert_eq!(press.dock, None);
        assert!(matches!(
            press.outcome,
            Err(LandingRefusal::CannotPayFee { .. })
        ));
    }

    #[test]
    fn a_stellar_that_is_not_landable_is_refused_so_before_its_fee() {
        let not_landable = charging(100, with_flags(0));
        assert!(matches!(
            pay(&not_landable, 0, None, |_| true).outcome,
            Err(LandingRefusal::NotLandable { .. })
        ));
        let site = charging(100, site(128, 0.0, 0.0));
        let unheld = land_or_select(
            Some(&site),
            None,
            &parked(),
            0,
            std::slice::from_ref(&site),
            |_| true,
            |_| false,
            engine_reach,
            RuleSource::Engine,
            RuleSource::Engine,
        );
        assert!(matches!(
            unheld.outcome,
            Err(LandingRefusal::NotLandable { .. })
        ));
    }

    #[test]
    fn no_fee_due_refuses_nobody() {
        let planet = site(128, 0.0, 0.0);
        let dominated = LandingSite {
            flags2: crate::market::DOMINATED,
            ..charging(100, planet)
        };
        for site in [dominated, charging(0, planet), charging(-5, planet)] {
            let press = pay(&site, 0, request(128, CLEARED), |_| true);
            assert_eq!(press.outcome, landed(128), "{site:?}");
        }
    }

    #[test]
    fn a_station_that_cannot_be_paid_says_so() {
        let station = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::STATION,
            ..charging(100, site(128, 0.0, 0.0))
        };
        assert_eq!(
            pay(&station, 0, None, |_| true).outcome,
            Err(LandingRefusal::CannotPayFee {
                stellar: StellarId(128),
                station: true,
            })
        );
    }

    #[test]
    fn a_cloaked_refusal_beeps() {
        // `_ShipIsCloaked` @0x66a8e jumps to the shared tail 0x67e67,
        // which plays `_beepSnd[3]` (@0x67e8f).
        assert!(LandingRefusal::Cloaked.beeps());
    }
}
