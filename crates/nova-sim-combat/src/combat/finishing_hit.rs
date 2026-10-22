//! The hits that start a ship dying, which an escort's victory voice reads
//! (`_HandleShipHit` @0x36c7f-0x36d45): a shot's direct hit, by contact
//! or by proximity, on a ship not dying before it (`_IsDying` @0x36ad5)
//! and dying after it (@0x36c85). A beam's hit (`_HandleBeams` →
//! `_DamageShip` @0x316a1), a blast's (@0x36ec0, @0x36106), a shockwave's
//! (@0x2da89) and a missile lock, which takes no armour, never start one
//! ([`Combat::take_landed`](super::Combat::take_landed)).

use super::ShipRef;

/// A hit that started its ship dying, with the index of its strike among
/// those it was taken with, so the session reads the ships the player
/// leads as they were when it landed (see `nova_sim::escort_voice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishingHit {
    /// The ship it started dying.
    pub ship: ShipRef,
    /// The index of its strike.
    pub strike: usize,
}
