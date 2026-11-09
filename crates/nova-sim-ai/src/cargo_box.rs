//! The cargo boxes a jettison leaves behind the player's ship and its
//! trader escorts, and the scoopable ones a destroyed asteroid drops: how
//! many each ship drops, where each box starts, how it drifts, spins and
//! fades, and when it is gone. Where they are drawn, and with which
//! `spïn`, is the view's.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`, `_JettisonCargo` (@0x3dffd)
//! drops the boxes once it has decided what goes (@0x3e1c2-0x3e2e7),
//! `_SpawnBox` (@0x3dd74-0x3dffc) places each, and `_HandleBoxes`
//! (@0x2df85-0x2e29e) steps them once a frame from `_DoPlayGameWork`
//! (@0x444f0), whatever the player's state.
//!
//! - **The shares** (@0x3e1f5-0x3e27e). The player always drops, its
//!   share the whole total jettisoned (`nova_sim::mission::cargo::Jettisoned::total`); each
//!   escort `_TotalFleetHolds` would count (parent the player, AI type 6,
//!   not a launched fighter, `InherentAI` 2 or less) drops the goods
//!   jettisoned times its `Holds` over the fleet's ([`escort_share`]).
//! - **The count** ([`box_count`], @0x3e282-0x3e2b5): none for a share
//!   of none or less, else `trunc(share / 5)`, at least 1 and at most 12.
//!   None while landed (0x3b5060).
//! - **The spawn** ([`CargoBoxes::spawn`]): the first free slot of 64,
//!   none when all are taken (see "The slots"); the ship's position and velocity; a life of
//!   `Rand(90) + 180` ticks; frame `Rand(36)`; a spin of −1, 0, 0 or +1
//!   by `Rand(4)`; graphic 0, holding nothing and not scoopable. It is
//!   moved half the ship's sprite width straight behind it (32 wide
//!   without a sprite, `_GetSpriteXSize` @0x26fc), and kicked
//!   `(Rand(40) + 30) / 100` px a tick from `Rand(30) − Rand(15)` degrees
//!   off straight behind, both along the whole-degree heading.
//! - **The scoopable box** ([`CargoBoxes::spawn_scoopable`],
//!   `_SpawnScoopableBox` @0x3e3c5-0x3e54f, called only by
//!   `_DestroyAsteroid` @0xae73): the first free slot of 64, none when all
//!   are taken (see "The slots"); where the asteroid was, from rest; a life of `Rand(200) +
//!   300` ticks; frame `Rand(36)`; a spin of −1, 0, 0 or +1 by `Rand(4)`;
//!   scoopable (+0x24), holding the asteroid's `YieldType` (+0x22), of
//!   the graphic given, 1-4 (+0x1e); kicked `(Rand(81) + 60) × 0.2 ×
//!   0.01` px a tick along `Rand(360)`, 0.12-0.28. A jettisoned box is
//!   written not scoopable (`_SpawnBox` @0x3de76).
//! - **The step** ([`CargoBoxes::step`]): a box whose life is below 0 is
//!   gone (life −2.0, @0x2dfa4-0x2dfee); otherwise its float frame turns
//!   by its spin × the frame's gain, wrapped into `[0, 36)` by two loops,
//!   its life falls by the gain and it moves by its velocity × the gain
//!   (`_gSpeedMult` @0x2e000, 0x2e06c; @0x2dff3-0x2e0bf), each added in
//!   doubles. It is drawn at its truncated frame ([`CargoBox::drawn_frame`],
//!   @0x2e1ae). Over its last 32 ticks it fades ([`CargoBox::fade`],
//!   @0x2e119-0x2e157). The gain is the session's, handed in as a value.
//!   No frozen gate: the boxes step under `_PrepareAllSprites` too.
//!
//! The session clears them on every system change and every landing,
//! where the original sets `_killBoxes` for the next `_HandleBoxes`.
//! Boxes are never saved.
//!
//! Spawning the scoopable boxes is
//! rdm:phase/asteroids/phase-2-shoot-and-break-asteroids's; their pickup
//! is [`scoop`](crate::scoop)'s ([`CargoBoxes::take_touching`]); the
//! miners that steer for them are
//! [`mine_flight`](crate::traffic::mine_flight)'s; their sprite murk
//! (`_HandleSpriteMurkiness` @0x2e181) is the view's
//! (`nova_view::flight::murk`,
//! rdm:phase/play-feel-4/phase-17-murk-other-sprites); the player's wrap
//! moves them ([`CargoBoxes::carry`]).
//!
//! # The slots
//!
//! `_boxData` is a table of 64 slots of 0x28 bytes (`_InitArrays`
//! @0x1ba8d-0x1ba99, `NewPtr(0xa00)`, never cleared). A spawn takes the
//! first slot whose life is below 0 (`_SpawnBox` @0x3dd94,
//! `_SpawnScoopableBox` @0x3e3f8), so a box spawned after an earlier one
//! went lands in the freed lower slot, and the boxes are drawn, collided
//! and listed in slot order ([`CargoBoxes::boxes`], a
//! [`SlotPool`](crate::slot_pool::SlotPool)). A spawn that finds no free
//! slot is dropped before it draws anything. The 64 slots are a resource
//! limit, so the engine keeps them only under `CombatCaps`' engine reading
//! ([`CargoBoxes::set_combat_caps`]); by default the table grows past 64
//! at its end, still first-free, as `slot_pool`'s docs describe, and the
//! slot bytes grow with it. Only a spawn writes a slot's
//! scoopable byte +0x24 and its contents +0x22 (`_SpawnBox` +0x24 = 0
//! @0x3de78; `_SpawnScoopableBox` +0x24 = 1 @0x3e482, +0x22 the yield
//! @0x3e48d); expiry, `_killBoxes` (`_HandleBoxes` @0x2dfb1-0x2dfee) and a
//! scoop (@0x3711c) set only the life. So a freed slot keeps its last
//! bytes ([`SlotBytes`]) until the next spawn into it, which is what a
//! scooping ship reads through an escape pod, whose sprite takes it for
//! box slot 0 ([`CargoBoxes::slot_zero`], [`CargoBoxes::take_slot_zero`];
//! see [`scoop`](crate::scoop)). The engine keeps them for the session's
//! life. A slot no spawn has written holds uninitialised memory in the
//! original (`NewPtr`, not `NewPtrClear`), which can't be replicated: the
//! engine reads it as not scoopable, holding nothing.

use crate::chance::Chance;
use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::player_wrap::PlayerWrap;
use crate::rulebook::RuleSource;
use crate::scoop::{BoxFrames, Scooping};
use crate::slot_pool::SlotPool;
use crate::speed_mult::SpeedMult;

/// The most boxes in space at once under `CombatCaps`' engine reading:
/// the table's 64 slots ([`CargoBoxes::set_combat_caps`]).
pub const MAX_BOXES: usize = 64;

/// The most boxes one ship drops in one jettison (@0x3e2b5).
pub const MAX_DROPPED: u32 = 12;

/// The tons each box dropped stands for (5.0 @0xdd6f0).
pub const TONS_PER_BOX: f64 = 5.0;

/// The frames of a box's spin (36.0 @0xdd670).
pub const FRAMES: u16 = 36;

/// The life below which a box fades, in ticks (32.0 @0xdd654).
pub const FADE_LIFE: f32 = 32.0;

/// The strongest fade (@0x2e152).
pub const MAX_FADE: u8 = 31;

/// The width `_GetSpriteXSize` gives a ship with no sprite (@0x26fc).
pub const NO_SPRITE_WIDTH: f32 = 32.0;

/// The sides of a scoopable box's life draw: `Rand(200) + 300` ticks.
const SCOOPABLE_LIFE_SIDES: u16 = 200;

/// The least life of a scoopable box, in ticks.
const SCOOPABLE_LIFE_BASE: u16 = 300;

/// The sides of a scoopable box's kick draw: `(Rand(81) + 60) × 0.2 ×
/// 0.01` pixels a tick.
const SCOOPABLE_KICK_SIDES: u16 = 81;

/// The least of a scoopable box's kick draw.
const SCOOPABLE_KICK_BASE: u16 = 60;

/// One cargo box in space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CargoBox {
    /// Where it is, in pixels from the system's centre.
    pub position: Vec2,
    /// How far it moves a pass at full gain: the move is this × the
    /// frame's gain.
    pub velocity: Vec2,
    /// The ticks left before it is gone: each pass takes the frame's gain
    /// off it.
    pub life: f32,
    /// Its spin frame, in `[0, 36)`, a float as the original keeps it
    /// (+0x18): it gains its spin × the frame's gain a pass. It is drawn
    /// at [`CargoBox::drawn_frame`].
    pub frame: f32,
    /// How many frames it turns a pass at full gain, −1, 0 or +1 (+0x1a, an
    /// `i16`): the turn is this × the frame's gain.
    pub spin: i8,
    /// Which box graphic it is drawn with, `spïn` 500 + graphic: 0 for a
    /// jettisoned box, 1-4 for an asteroid's.
    pub graphic: u16,
    /// Whether it can be scooped up: only an asteroid's box.
    pub scoopable: bool,
    /// What it holds: an asteroid's `YieldType`, a commodity 0-5 or a
    /// `jünk` 1000-1127, as a scoop reads it ([`haul`](crate::scoop::haul)),
    /// though any value is kept; none for a jettisoned box.
    pub contents: Option<i16>,
}

impl CargoBox {
    /// The frame it is drawn at, its spin frame truncated (`cvttss2si`
    /// @0x2e1ae, handed to `_SWSetCurrentFrameIndex`).
    #[must_use]
    pub fn drawn_frame(&self) -> u16 {
        // In [0, 36) once wrapped; a NaN or a far value saturates.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let frame = self.frame.trunc() as u16;
        frame
    }

    /// How faded it is, 0 (not at all) to [`MAX_FADE`]: none with
    /// [`FADE_LIFE`] ticks or more left, else `min(trunc(32 − life), 31)`
    /// (@0x2e119-0x2e157).
    #[must_use]
    pub fn fade(&self) -> u8 {
        if self.life >= FADE_LIFE {
            return 0;
        }
        // In 0..=33 at most, the life below 32 and never below -1.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let fade = (FADE_LIFE - self.life).trunc() as u8;
        fade.min(MAX_FADE)
    }
}

/// Whether any of `boxes` is scoopable: the original's
/// `_miningBoxesActive` (see [`CargoBoxes::any_scoopable`]).
#[must_use]
pub fn any_scoopable(boxes: &[CargoBox]) -> bool {
    boxes.iter().any(|cargo_box| cargo_box.scoopable)
}

/// How many boxes a ship whose share of a jettison is `share` tons drops
/// (@0x3e282-0x3e2b5): none for a share of none or less, else
/// `trunc(share / 5)`, raised to 1 and capped at [`MAX_DROPPED`].
#[must_use]
pub fn box_count(share: f64) -> u32 {
    if share.is_nan() || share <= 0.0 {
        return 0;
    }
    let boxes = (share / TONS_PER_BOX).trunc().min(f64::from(MAX_DROPPED));
    // Whole, in 0..=12.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let boxes = boxes as u32;
    boxes.max(1)
}

/// An escort's share of a jettison of `goods` tons of goods, its ship's
/// `Holds` `holds` of the fleet's `fleet_holds` (@0x3e253-0x3e27e): the
/// goods times `holds` over `fleet_holds`. The mission cargo is not
/// shared out. None when the fleet has no holds.
#[must_use]
pub fn escort_share(goods: i64, holds: i16, fleet_holds: u32) -> f64 {
    if fleet_holds == 0 {
        return 0.0;
    }
    // Tons of cargo, far inside f64's exact range.
    #[allow(clippy::cast_precision_loss)]
    let goods = goods as f64;
    goods * (f64::from(holds) / f64::from(fleet_holds))
}

/// What a box slot's last spawn wrote to it, kept once its box is gone
/// (see "The slots" in the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotBytes {
    /// Its scoopable byte, +0x24.
    pub scoopable: bool,
    /// What it holds, +0x22 ([`CargoBox::contents`]).
    pub contents: Option<i16>,
}

/// The cargo boxes in space, by slot, at most [`MAX_BOXES`] under
/// `CombatCaps`' engine reading (see "The slots" in the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CargoBoxes {
    /// The live boxes, by slot.
    boxes: SlotPool<CargoBox>,
    /// Each slot's last bytes, by slot; a slot past the end was never
    /// written.
    bytes: Vec<SlotBytes>,
    /// The most boxes the table holds: [`MAX_BOXES`] under `CombatCaps`'
    /// engine reading, none by default ([`CargoBoxes::set_combat_caps`]).
    cap: Option<usize>,
}

impl CargoBoxes {
    /// Caps the table as `source` reads `RuleKey::CombatCaps`: at
    /// [`MAX_BOXES`] by the engine, a box finding no slot dropped before
    /// its draws; uncapped by the other reading, the default, the table
    /// growing past 64 at its end, still first-free. The cap lasts the
    /// session: [`CargoBoxes::clear`] keeps it.
    pub fn set_combat_caps(&mut self, source: RuleSource) {
        self.cap = (source == RuleSource::Engine).then_some(MAX_BOXES);
    }

    /// Carries each box within `wrap`'s reach: `_WrapPlayerPosition`'s loop
    /// of `_boxData` (@0x6811f-0x681a1), which leaves its velocity.
    pub fn carry(&mut self, wrap: &PlayerWrap) {
        for cargo in self.boxes.iter_mut() {
            wrap.carry(&mut cargo.position);
        }
    }

    /// The boxes in space, in slot order.
    #[must_use]
    pub fn boxes(&self) -> &[CargoBox] {
        self.boxes.as_slice()
    }

    /// Whether the table holds all its cap allows: never by default.
    fn is_full(&self) -> bool {
        self.cap.is_some_and(|cap| self.boxes.len() >= cap)
    }

    /// Puts `cargo_box` in the first free slot and writes the slot's bytes
    /// from it; nothing when the cap leaves no slot free.
    fn insert(&mut self, cargo_box: CargoBox) {
        let Some(slot) = self.boxes.insert(cargo_box, self.cap) else {
            return;
        };
        if self.bytes.len() <= slot {
            self.bytes.resize(slot + 1, SlotBytes::default());
        }
        self.bytes[slot] = SlotBytes {
            scoopable: cargo_box.scoopable,
            contents: cargo_box.contents,
        };
    }

    /// Box slot 0's last bytes, its box live or not: what a scooping ship
    /// reads through an escape pod (`_ShipCollisionHandler`
    /// @0x370ea-0x370ff).
    #[must_use]
    pub fn slot_zero(&self) -> SlotBytes {
        self.bytes.first().copied().unwrap_or_default()
    }

    /// Takes box slot 0 as the scoop branch does through an escape pod
    /// (@0x3711c, life −1): frees its live box if there is one, and gives
    /// its last bytes ([`CargoBoxes::slot_zero`]). The box leaves the table
    /// at once, as [`CargoBoxes::take_touching`]'s does.
    pub fn take_slot_zero(&mut self) -> SlotBytes {
        if self.boxes.slot_of(0) == Some(0) {
            self.boxes.remove(0);
        }
        self.slot_zero()
    }

    /// Drops [`box_count`]`(share)` boxes behind `ship`, its sprite
    /// `sprite_width` wide (none without one), drawing on `chance`.
    pub fn drop_behind(
        &mut self,
        ship: &ShipState,
        sprite_width: Option<f32>,
        share: f64,
        chance: &mut (impl Chance + ?Sized),
    ) {
        for _ in 0..box_count(share) {
            self.spawn(ship, sprite_width, chance);
        }
    }

    /// Spawns one box behind `ship` as `_SpawnBox` does (see the module
    /// docs), drawing on `chance`; nothing, and no draw, when the cap
    /// leaves no slot free.
    pub fn spawn(
        &mut self,
        ship: &ShipState,
        sprite_width: Option<f32>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.is_full() {
            return;
        }
        let life = f32::from(chance.roll(90) + 180);
        let frame = f32::from(chance.roll(FRAMES));
        let spin = spin_of(chance.roll(4));
        // The whole-degree heading, in [0, 360).
        #[allow(clippy::cast_possible_truncation)]
        let heading = ship.heading.trunc() as i32;
        let behind = (sprite_width.unwrap_or(NO_SPRITE_WIDTH) * 0.5).trunc();
        let position = ship.position + accel(reversed(heading), behind);
        let speed = f32::from(chance.roll(40) + 30) / 100.0;
        let off = i32::from(chance.roll(30)) - i32::from(chance.roll(15));
        let velocity = ship.velocity + accel(reversed(heading + off), speed);
        self.insert(CargoBox {
            position,
            velocity,
            life,
            frame,
            spin,
            graphic: 0,
            scoopable: false,
            contents: None,
        });
    }

    /// Spawns one scoopable box at `at`, holding `contents`, of `graphic`,
    /// as `_SpawnScoopableBox` does (see the module docs), drawing on
    /// `chance`; nothing, and no draw, when the cap leaves no slot free.
    pub fn spawn_scoopable(
        &mut self,
        at: Vec2,
        contents: Option<i16>,
        graphic: u16,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.is_full() {
            return;
        }
        let life = f32::from(chance.roll(SCOOPABLE_LIFE_SIDES) + SCOOPABLE_LIFE_BASE);
        let frame = f32::from(chance.roll(FRAMES));
        let spin = spin_of(chance.roll(4));
        let angle = i32::from(chance.roll(360));
        let kick = f64::from(chance.roll(SCOOPABLE_KICK_SIDES) + SCOOPABLE_KICK_BASE) * 0.2 * 0.01;
        // A third of a pixel at most.
        #[allow(clippy::cast_possible_truncation)]
        let velocity = accel(angle, kick as f32);
        self.insert(CargoBox {
            position: at,
            velocity,
            life,
            frame,
            spin,
            graphic,
            scoopable: true,
            contents,
        });
    }

    /// Steps every box a pass at the frame's `gain` as `_HandleBoxes`
    /// does: one whose life is below 0 is gone (`ucomiss life, 0` / `ja`
    /// @0x2dfb1-0x2dfb5, so a NaN life is kept), whatever the gain; the
    /// others turn by their spin × the gain, wrapped ([`CargoBox::frame`],
    /// @0x2dff3-0x2e06a), age by the gain and move by their velocity × the
    /// gain (@0x2e06c-0x2e0bf), each added in doubles.
    pub fn step(&mut self, gain: SpeedMult) {
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        self.boxes.retain_mut(|cargo_box| !(cargo_box.life < 0.0));
        for cargo_box in self.boxes.iter_mut() {
            let turned = gain.add_times(cargo_box.frame, f32::from(cargo_box.spin));
            cargo_box.frame = wrapped(turned);
            cargo_box.life = gain.add_times(cargo_box.life, -1.0);
            cargo_box.position = crate::flight::moved(cargo_box.position, cargo_box.velocity, gain);
        }
    }

    /// Whether any box is scoopable: the original's `_miningBoxesActive`,
    /// which gates the scoop's collision (see [`scoop`](crate::scoop)) and
    /// a miner's state 17 (see [`miner`](crate::ai::miner)).
    #[must_use]
    pub fn any_scoopable(&self) -> bool {
        any_scoopable(self.boxes.as_slice())
    }

    /// Takes the first scoopable box, in slot order, that `scooping`
    /// touches, the boxes' graphics' frames `frames`
    /// ([`Scooping::touches`]); none when no scoopable box does. A
    /// jettisoned box is never taken.
    ///
    /// The box leaves the table at once. The original sets its life to −1
    /// and hides its sprite (@0x37119-0x3712e), and the next
    /// `_HandleBoxes` kills it, so no frame is drawn with it either way.
    pub fn take_touching(&mut self, scooping: &Scooping, frames: &BoxFrames) -> Option<CargoBox> {
        let index = self
            .boxes
            .iter()
            .position(|cargo_box| cargo_box.scoopable && scooping.touches(cargo_box, frames))?;
        Some(self.boxes.remove(index))
    }

    /// Empties the table: every box is gone, each slot keeping its last
    /// bytes (see "The slots" in the module docs).
    pub fn clear(&mut self) {
        self.boxes.clear();
    }
}

/// `frame` wrapped into `[0, 36)` as `_HandleBoxes` wraps it, by two
/// float loops (@0x2e01e-0x2e06a): while it is 36.0 or more it loses 36.0
/// (`ucomiss 36, f` / `jae`), then while it is below 0.0 it gains 36.0
/// (`ucomiss f, 0` / `ja`). A NaN takes neither loop, so it stays
/// unwrapped.
fn wrapped(mut frame: f32) -> f32 {
    let frames = f32::from(FRAMES);
    while frame >= frames {
        frame -= frames;
    }
    while frame < 0.0 {
        frame += frames;
    }
    frame
}

/// A box's spin by its `Rand(4)` draw `roll`: −1, 0, 0 or +1.
fn spin_of(roll: u16) -> i8 {
    match roll {
        0 => -1,
        3 => 1,
        _ => 0,
    }
}

/// The heading `angle` turned straight round: `angle + 180`, which
/// [`accel`] takes mod 360.
fn reversed(angle: i32) -> i32 {
    angle + 180
}

/// `_Accel`'s push (@0x2ade6): `magnitude` along `angle` whole degrees,
/// taken mod 360 as the original's `(angle) mod 360` is, every angle
/// here being no less than 0.
#[must_use]
pub fn accel(angle: i32, magnitude: f32) -> Vec2 {
    // In 0..360.
    #[allow(clippy::cast_precision_loss)]
    let angle = angle.rem_euclid(360) as f32;
    facing(angle) * magnitude
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::scoop::BoxFrames;
    use nova_sim_base::fixture::Scripted;

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    /// A table holding `boxes`, in slots 0 on.
    fn of(boxes: &[CargoBox]) -> CargoBoxes {
        let mut table = CargoBoxes::default();
        for &cargo_box in boxes {
            table.insert(cargo_box);
        }
        table
    }

    #[test]
    fn the_count_is_a_box_a_5_tons_at_least_1_and_at_most_12() {
        for (share, count) in [
            (0.0, 0),
            (-3.0, 0),
            (f64::NAN, 0),
            (0.4, 1),
            (4.9, 1),
            (5.0, 1),
            (10.0, 2),
            (59.9, 11),
            (60.0, 12),
            (1000.0, 12),
            (f64::INFINITY, 12),
        ] {
            assert_eq!(box_count(share), count, "{share}");
        }
    }

    #[test]
    fn an_escort_shares_the_goods_by_its_holds_of_the_fleets() {
        assert_eq!(escort_share(30, 50, 150), 10.0);
        assert_eq!(escort_share(0, 50, 150), 0.0);
        assert_eq!(escort_share(30, -50, 150), -10.0, "a negative Holds too");
        assert_eq!(escort_share(30, 50, 0), 0.0, "no fleet holds, no share");
    }

    #[test]
    fn a_wrap_carries_the_boxes_within_its_reach_and_leaves_their_velocities() {
        use crate::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
        let player = Vec2::new(WRAP_EDGE + 10.0, 0.0);
        let wrap = PlayerWrap::of(player).expect("past the edge");
        let near = player + Vec2::new(-100.0, 40.0);
        let far = player + Vec2::new(-WRAP_EDGE * 2.0, 0.0);
        let at = |position| CargoBox {
            position,
            velocity: Vec2::new(0.5, -1.5),
            life: 10.0,
            frame: 0.0,
            spin: 0,
            graphic: 0,
            scoopable: false,
            contents: None,
        };
        let mut boxes = of(&[at(near), at(far)]);
        boxes.carry(&wrap);
        let moved = at(near + Vec2::new(-WRAP_SHIFT, 0.0));
        assert_eq!(boxes.boxes(), [moved, at(far)]);
    }

    fn heading(heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(100.0, 200.0),
            velocity: Vec2::new(1.0, -2.0),
            heading,
        }
    }

    /// A spawn's rolls, in `_SpawnBox`'s order: life, frame, spin, then
    /// the kick's speed, its first and its second angle draw.
    fn rolls(life: u16, frame: u16, spin: u16, speed: u16, add: u16, sub: u16) -> Scripted {
        Scripted::rolling(&[life, frame, spin, speed, add, sub])
    }

    fn spawned(ship: &ShipState, width: Option<f32>, chance: &mut Scripted) -> CargoBox {
        let mut boxes = CargoBoxes::default();
        boxes.spawn(ship, width, chance);
        assert_eq!(boxes.boxes().len(), 1);
        boxes.boxes()[0]
    }

    #[test]
    fn a_box_rolls_its_life_frame_and_spin_over_their_ranges() {
        let ship = heading(0.0);
        for (life, frame, spin, expected) in [
            (0, 0, 0, (180.0, 0.0, -1)),
            (89, 35, 1, (269.0, 35.0, 0)),
            (10, 7, 2, (190.0, 7.0, 0)),
            (0, 0, 3, (180.0, 0.0, 1)),
        ] {
            let mut chance = rolls(life, frame, spin, 0, 0, 0);
            let made = spawned(&ship, Some(40.0), &mut chance);
            assert_eq!((made.life, made.frame, made.spin), expected);
            assert_eq!(made.graphic, 0);
            assert!(!made.scoopable, "a jettisoned box is not scoopable");
            assert_eq!(made.contents, None);
            assert_eq!(
                chance.sides_asked,
                [90, 36, 4, 40, 30, 15],
                "the ranges each draw decides"
            );
        }
    }

    #[test]
    fn a_box_starts_half_the_sprite_behind_the_ship() {
        let mut chance = rolls(0, 0, 0, 0, 0, 0);
        // Facing up: behind is down.
        let made = spawned(&heading(0.0), Some(41.0), &mut chance);
        assert!(
            close(made.position, Vec2::new(100.0, 220.0)),
            "{:?}: half of 41 truncated",
            made.position
        );
        let mut chance = rolls(0, 0, 0, 0, 0, 0);
        // Facing right, its heading's fraction dropped: behind is left.
        let made = spawned(&heading(90.7), Some(40.0), &mut chance);
        assert!(
            close(made.position, Vec2::new(80.0, 200.0)),
            "{:?}",
            made.position
        );
        let mut chance = rolls(0, 0, 0, 0, 0, 0);
        let made = spawned(&heading(270.0), None, &mut chance);
        assert!(
            close(made.position, Vec2::new(116.0, 200.0)),
            "{:?}: 32 wide without a sprite",
            made.position
        );
    }

    #[test]
    fn a_box_is_kicked_from_0_30_to_0_69_px_a_tick_off_straight_behind() {
        // Facing up: straight behind is 180 degrees, down.
        let ship = heading(0.0);
        let mut chance = rolls(0, 0, 0, 0, 0, 0);
        let made = spawned(&ship, Some(40.0), &mut chance);
        let kick = made.velocity - ship.velocity;
        assert!(close(kick, Vec2::new(0.0, 0.3)), "{kick:?}");
        let mut chance = rolls(0, 0, 0, 39, 0, 0);
        let made = spawned(&ship, Some(40.0), &mut chance);
        let kick = made.velocity - ship.velocity;
        assert!(close(kick, Vec2::new(0.0, 0.69)), "{kick:?}");
        // Rand(30) turns it clockwise, Rand(15) back: 29 and -14 degrees.
        let mut chance = rolls(0, 0, 0, 39, 29, 0);
        let made = spawned(&ship, Some(40.0), &mut chance);
        let kick = made.velocity - ship.velocity;
        assert!(close(kick, facing(209.0) * 0.69), "{kick:?}");
        let mut chance = rolls(0, 0, 0, 39, 0, 14);
        let made = spawned(&ship, Some(40.0), &mut chance);
        let kick = made.velocity - ship.velocity;
        assert!(close(kick, facing(166.0) * 0.69), "{kick:?}");
        // Wrapped past 360.
        let mut chance = rolls(0, 0, 0, 39, 29, 0);
        let made = spawned(&heading(359.5), Some(40.0), &mut chance);
        let kick = made.velocity - Vec2::new(1.0, -2.0);
        assert!(close(kick, facing(208.0) * 0.69), "{kick:?}");
    }

    /// A scoopable box's rolls, in `_SpawnScoopableBox`'s order: life,
    /// frame, spin, then the kick's direction and speed.
    fn scooped(life: u16, frame: u16, spin: u16, angle: u16, speed: u16) -> CargoBox {
        let mut boxes = CargoBoxes::default();
        let mut chance = Scripted::rolling(&[life, frame, spin, angle, speed]);
        boxes.spawn_scoopable(Vec2::new(30.0, -40.0), Some(1003), 3, &mut chance);
        assert_eq!(chance.sides_asked, [200, 36, 4, 360, 81]);
        assert_eq!(boxes.boxes().len(), 1);
        boxes.boxes()[0]
    }

    #[test]
    fn a_scoopable_box_holds_its_yield_and_graphic_where_the_asteroid_was() {
        let made = scooped(0, 7, 1, 0, 0);
        assert!(made.scoopable);
        assert_eq!((made.contents, made.graphic), (Some(1003), 3));
        assert_eq!(made.position, Vec2::new(30.0, -40.0));
        assert_eq!(made.frame, 7.0);
    }

    #[test]
    fn a_scoopable_box_lives_300_to_499_ticks_and_spins_by_its_draw() {
        assert_eq!(scooped(0, 0, 0, 0, 0).life, 300.0);
        assert_eq!(scooped(199, 0, 0, 0, 0).life, 499.0);
        let spins: Vec<i8> = (0..4).map(|roll| scooped(0, 0, roll, 0, 0).spin).collect();
        assert_eq!(spins, [-1, 0, 0, 1]);
    }

    #[test]
    fn a_scoopable_box_is_kicked_0_12_to_0_28_px_a_tick_any_way_from_rest() {
        let slow = scooped(0, 0, 0, 0, 0);
        assert!(close(slow.velocity, Vec2::new(0.0, -0.12)), "{slow:?}");
        let fast = scooped(0, 0, 0, 90, 80);
        assert!(close(fast.velocity, Vec2::new(0.28, 0.0)), "{fast:?}");
        let back = scooped(0, 0, 0, 359, 80);
        assert!(close(back.velocity, facing(359.0) * 0.28), "{back:?}");
    }

    /// A table under `CombatCaps`' engine reading: at most [`MAX_BOXES`].
    fn capped() -> CargoBoxes {
        let mut boxes = CargoBoxes::default();
        boxes.set_combat_caps(RuleSource::Engine);
        boxes
    }

    #[test]
    fn under_the_caps_a_65th_scoopable_box_is_dropped() {
        let mut boxes = capped();
        for _ in 0..MAX_BOXES + 3 {
            boxes.spawn_scoopable(Vec2::ZERO, None, 1, &mut Scripted::default());
        }
        assert_eq!(boxes.boxes().len(), MAX_BOXES);
    }

    #[test]
    fn under_the_caps_a_65th_box_is_dropped() {
        let mut boxes = capped();
        for _ in 0..MAX_BOXES + 3 {
            boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        }
        assert_eq!(boxes.boxes().len(), MAX_BOXES);
    }

    #[test]
    fn under_the_caps_a_box_finding_no_slot_draws_nothing() {
        let mut boxes = capped();
        for _ in 0..MAX_BOXES {
            boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        }
        let mut chance = Scripted::default();
        boxes.spawn(&heading(0.0), None, &mut chance);
        boxes.spawn_scoopable(Vec2::ZERO, None, 1, &mut chance);
        assert!(chance.sides_asked.is_empty(), "{:?}", chance.sides_asked);
    }

    #[test]
    fn by_default_a_65th_box_is_kept_first_free() {
        let count = i16::try_from(MAX_BOXES).unwrap() + 1;
        let mut boxes = CargoBoxes::default();
        for held in 0..count {
            boxes.spawn_scoopable(
                Vec2::new(f32::from(held), 0.0),
                Some(held),
                1,
                &mut Scripted::default(),
            );
        }
        assert_eq!(contents(&boxes), (0..count).map(Some).collect::<Vec<_>>());
        assert_eq!(
            boxes.bytes[MAX_BOXES],
            SlotBytes {
                scoopable: true,
                contents: Some(count - 1)
            },
            "slot 64's bytes are written"
        );
        let taken = boxes.take_touching(
            &Scooping::disc(Vec2::new(1.0, 0.0), 0.1),
            &BoxFrames::default(),
        );
        assert_eq!(taken.map(|made| made.contents), Some(Some(1)));
        boxes.spawn_scoopable(Vec2::ZERO, Some(500), 1, &mut Scripted::default());
        let listed = contents(&boxes);
        assert_eq!(listed.len(), MAX_BOXES + 1);
        assert_eq!(
            listed[..3],
            [Some(0), Some(500), Some(2)],
            "the freed slot 1"
        );
        assert_eq!(listed[MAX_BOXES], Some(count - 1));
    }

    #[test]
    fn by_default_a_jettison_keeps_boxes_past_64() {
        let mut boxes = CargoBoxes::default();
        for _ in 0..MAX_BOXES + 3 {
            boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        }
        assert_eq!(boxes.boxes().len(), MAX_BOXES + 3);
    }

    #[test]
    fn only_the_engine_reading_caps_the_table_and_clearing_keeps_it() {
        let mut boxes = capped();
        assert_eq!(boxes.cap, Some(MAX_BOXES));
        boxes.clear();
        assert_eq!(boxes.cap, Some(MAX_BOXES), "clearing keeps the cap");
        boxes.set_combat_caps(RuleSource::Bible);
        assert_eq!(boxes.cap, None);
    }

    #[test]
    fn a_ship_drops_as_many_boxes_as_its_share_counts() {
        for (share, count) in [(0.0, 0), (1.0, 1), (20.0, 4), (100.0, 12)] {
            let mut boxes = CargoBoxes::default();
            boxes.drop_behind(&heading(0.0), None, share, &mut Scripted::default());
            assert_eq!(boxes.boxes().len(), count, "{share}");
        }
    }

    fn one(life: f32, frame: f32, spin: i8) -> CargoBoxes {
        of(&[CargoBox {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(0.5, -1.5),
            life,
            frame,
            spin,
            graphic: 0,
            scoopable: false,
            contents: None,
        }])
    }

    #[test]
    fn a_step_turns_ages_and_moves_each_box() {
        let mut boxes = one(100.0, 7.0, 1);
        boxes.step(SpeedMult::FULL);
        let stepped = boxes.boxes()[0];
        assert_eq!(stepped.frame, 8.0);
        assert_eq!(stepped.life, 99.0);
        assert_eq!(stepped.position, Vec2::new(10.5, 18.5));
        assert_eq!(stepped.velocity, Vec2::new(0.5, -1.5));
    }

    #[test]
    fn the_frame_wraps_both_ways_and_a_still_box_keeps_it() {
        let mut boxes = one(100.0, 35.0, 1);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes()[0].frame, 0.0);
        let mut boxes = one(100.0, 0.0, -1);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes()[0].frame, 35.0);
        let mut boxes = one(100.0, 12.0, 0);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes()[0].frame, 12.0);
    }

    #[test]
    fn a_box_is_gone_on_the_step_after_its_life_falls_below_0() {
        let mut boxes = one(0.5, 0.0, 0);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes()[0].life, -0.5, "still shown");
        boxes.step(SpeedMult::FULL);
        assert!(boxes.boxes().is_empty());
        let mut boxes = one(0.0, 0.0, 0);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes().len(), 1, "a life of exactly 0 steps on");
    }

    const HALF: SpeedMult = SpeedMult::new(0.5);

    #[test]
    fn at_half_gain_a_box_spins_half_a_frame_a_step_wrapping_both_ways() {
        let mut boxes = one(100.0, 35.0, 1);
        boxes.step(HALF);
        assert_eq!(boxes.boxes()[0].frame, 35.5);
        assert_eq!(boxes.boxes()[0].drawn_frame(), 35, "drawn truncated");
        boxes.step(HALF);
        assert_eq!(boxes.boxes()[0].frame, 0.0, "36.0 wraps to 0");
        let mut back = one(100.0, 0.0, -1);
        back.step(HALF);
        assert_eq!(back.boxes()[0].frame, 35.5, "-0.5 wraps to 35.5");
    }

    #[test]
    fn a_box_ages_and_drifts_by_the_gain() {
        let mut half = one(100.0, 7.0, 1);
        half.step(HALF);
        let stepped = half.boxes()[0];
        assert_eq!(stepped.life, 99.5);
        assert_eq!(stepped.position, Vec2::new(10.25, 19.25));
        let mut held = one(100.0, 7.0, 1);
        held.step(SpeedMult::ZERO);
        assert_eq!(held, one(100.0, 7.0, 1), "held still at 0");
    }

    #[test]
    fn at_zero_gain_a_box_below_life_0_is_still_gone() {
        let mut boxes = one(-0.5, 0.0, 0);
        boxes.step(SpeedMult::ZERO);
        assert!(boxes.boxes().is_empty(), "the gone test is unscaled");
    }

    #[test]
    fn a_box_fades_by_its_fractional_life_at_half_gain() {
        let mut boxes = one(11.0, 0.0, 0);
        boxes.step(HALF);
        // trunc(32 - 10.5) = 21.
        assert_eq!(boxes.boxes()[0].fade(), 21);
    }

    #[test]
    fn a_box_with_a_nan_life_is_kept_and_a_nan_frame_stays_unwrapped() {
        let mut boxes = one(f32::NAN, 0.0, 0);
        boxes.step(SpeedMult::FULL);
        assert_eq!(boxes.boxes().len(), 1, "a NaN life is kept");
        let mut boxes = one(100.0, f32::NAN, 1);
        boxes.step(SpeedMult::FULL);
        assert!(boxes.boxes()[0].frame.is_nan(), "neither loop runs");
    }

    #[test]
    fn a_box_lasts_its_life_and_one_tick_more() {
        let mut boxes = CargoBoxes::default();
        boxes.spawn(&heading(0.0), None, &mut rolls(0, 0, 0, 0, 0, 0));
        for _ in 0..181 {
            boxes.step(SpeedMult::FULL);
        }
        assert_eq!(boxes.boxes().len(), 1);
        boxes.step(SpeedMult::FULL);
        assert!(boxes.boxes().is_empty());
    }

    #[test]
    fn a_box_fades_over_its_last_32_ticks() {
        for (life, fade) in [
            (100.0, 0),
            (32.0, 0),
            (31.9, 0),
            (31.0, 1),
            (16.5, 15),
            (1.0, 31),
            (0.5, 31),
            (-0.5, 31),
        ] {
            assert_eq!(one(life, 0.0, 0).boxes()[0].fade(), fade, "{life}");
        }
    }

    /// A table of a jettisoned box at the origin, then scoopable boxes
    /// of graphic 1 holding each of `contents` at each of `at`.
    fn with_scoopable(at: &[(f32, f32)], contents: &[i16]) -> CargoBoxes {
        let mut boxes = of(&[CargoBox {
            position: Vec2::ZERO,
            ..one(100.0, 0.0, 0).boxes()[0]
        }]);
        for (&(x, y), &held) in at.iter().zip(contents) {
            boxes.spawn_scoopable(Vec2::new(x, y), Some(held), 1, &mut Scripted::default());
        }
        boxes
    }

    #[test]
    fn only_a_scoopable_box_makes_the_table_scoopable() {
        assert!(!CargoBoxes::default().any_scoopable());
        assert!(!one(100.0, 0.0, 0).any_scoopable(), "a jettisoned box");
        assert!(with_scoopable(&[(50.0, 0.0)], &[1]).any_scoopable());
    }

    #[test]
    fn a_disc_takes_the_first_scoopable_box_it_touches_and_leaves_the_rest() {
        let frames = BoxFrames::default();
        let mut boxes = with_scoopable(&[(90.0, 0.0), (5.0, 0.0), (-5.0, 0.0)], &[0, 1, 2]);
        let taken = boxes.take_touching(&Scooping::disc(Vec2::ZERO, 10.0), &frames);
        assert_eq!(taken.map(|made| made.contents), Some(Some(1)));
        assert_eq!(
            boxes
                .boxes()
                .iter()
                .map(|made| made.contents)
                .collect::<Vec<_>>(),
            [None, Some(0), Some(2)],
            "the jettisoned box under the disc is left, and the order kept"
        );
        let taken = boxes.take_touching(&Scooping::disc(Vec2::ZERO, 10.0), &frames);
        assert_eq!(taken.map(|made| made.contents), Some(Some(2)));
        assert_eq!(
            boxes.take_touching(&Scooping::disc(Vec2::ZERO, 10.0), &frames),
            None
        );
        assert_eq!(boxes.boxes().len(), 2);
    }

    #[test]
    fn a_disc_touches_a_box_by_its_graphics_frame() {
        // Graphic 1 is 40 across, drawn up and left of the box's point: the
        // box at 65 is 25 from its frame's left edge.
        let frames = BoxFrames::new([None, Some((40, 40)), None, None, None]);
        let mut boxes = with_scoopable(&[(65.0, 0.0)], &[3]);
        assert_eq!(
            boxes.take_touching(&Scooping::disc(Vec2::ZERO, 24.0), &frames),
            None
        );
        assert!(
            boxes
                .take_touching(&Scooping::disc(Vec2::ZERO, 25.0), &frames)
                .is_some()
        );
    }

    #[test]
    fn clearing_empties_the_table() {
        let mut boxes = one(100.0, 0.0, 0);
        boxes.clear();
        assert!(boxes.boxes().is_empty());
    }

    /// Scoopable boxes of graphic 1 at `x` along the axis, each holding
    /// its index, in spawn order.
    fn in_a_row(xs: &[f32]) -> CargoBoxes {
        let mut boxes = CargoBoxes::default();
        for (held, &x) in (0..).zip(xs) {
            boxes.spawn_scoopable(Vec2::new(x, 0.0), Some(held), 1, &mut Scripted::default());
        }
        boxes
    }

    fn contents(boxes: &CargoBoxes) -> Vec<Option<i16>> {
        boxes.boxes().iter().map(|made| made.contents).collect()
    }

    #[test]
    fn a_box_takes_a_freed_lower_slot_and_is_listed_before_an_older_one() {
        let frames = BoxFrames::default();
        let mut boxes = in_a_row(&[0.0, 100.0, 200.0]);
        let taken = boxes.take_touching(&Scooping::disc(Vec2::ZERO, 1.0), &frames);
        assert_eq!(taken.map(|made| made.contents), Some(Some(0)));
        boxes.spawn_scoopable(Vec2::new(300.0, 0.0), Some(7), 1, &mut Scripted::default());
        assert_eq!(
            contents(&boxes),
            [Some(7), Some(1), Some(2)],
            "slot 0 first"
        );
        boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        assert_eq!(boxes.boxes().len(), 4);
        assert_eq!(boxes.boxes()[3].contents, None, "the end, slot 3");
    }

    #[test]
    fn a_box_lost_to_its_life_frees_its_slot_for_the_next() {
        let mut boxes = one(-0.5, 0.0, 0);
        boxes.spawn_scoopable(Vec2::ZERO, Some(4), 1, &mut Scripted::default());
        boxes.step(SpeedMult::FULL);
        assert_eq!(contents(&boxes), [Some(4)]);
        boxes.spawn_scoopable(Vec2::ZERO, Some(5), 1, &mut Scripted::default());
        assert_eq!(contents(&boxes), [Some(5), Some(4)], "slot 0 again");
    }

    #[test]
    fn the_scoop_takes_the_first_box_it_touches_in_slot_order() {
        let frames = BoxFrames::default();
        let mut boxes = in_a_row(&[0.0, 100.0, 200.0]);
        let disc = Scooping::disc(Vec2::ZERO, 1.0);
        boxes.take_touching(&disc, &frames);
        // Into slot 0, under the disc, beside the older box in slot 1.
        boxes.spawn_scoopable(Vec2::ZERO, Some(8), 1, &mut Scripted::default());
        boxes
            .boxes
            .iter_mut()
            .for_each(|made| made.position = Vec2::ZERO);
        let taken = boxes.take_touching(&disc, &frames);
        assert_eq!(taken.map(|made| made.contents), Some(Some(8)));
    }

    #[test]
    fn order_0x15_breaks_a_tie_to_the_lower_slot() {
        let frames = BoxFrames::default();
        let mut boxes = in_a_row(&[0.0, 100.0]);
        boxes.take_touching(&Scooping::disc(Vec2::ZERO, 1.0), &frames);
        // Spawned last, into slot 0, as far from the ship as slot 1's.
        boxes.spawn_scoopable(Vec2::new(-100.0, 0.0), Some(9), 1, &mut Scripted::default());
        let nearest = crate::ai::mine_orders::nearest_box(Vec2::ZERO, boxes.boxes());
        assert_eq!(nearest, Some(Vec2::new(-100.0, 0.0)));
    }

    #[test]
    fn a_freed_slot_keeps_its_scoopable_byte_and_contents() {
        let frames = BoxFrames::default();
        let mut boxes = in_a_row(&[0.0, 100.0]);
        boxes.take_touching(&Scooping::disc(Vec2::ZERO, 1.0), &frames);
        assert_eq!(
            boxes.slot_zero(),
            SlotBytes {
                scoopable: true,
                contents: Some(0)
            }
        );
        let mut expired = in_a_row(&[0.0]);
        expired.boxes.iter_mut().for_each(|made| made.life = -1.0);
        expired.step(SpeedMult::FULL);
        assert!(expired.boxes().is_empty());
        assert_eq!(expired.slot_zero().contents, Some(0), "expiry keeps it");
        let mut cleared = in_a_row(&[0.0]);
        cleared.clear();
        assert!(cleared.boxes().is_empty());
        assert_eq!(
            cleared.slot_zero(),
            SlotBytes {
                scoopable: true,
                contents: Some(0)
            },
            "a clear keeps it"
        );
    }

    #[test]
    fn a_jettison_into_slot_0_writes_it_not_scoopable() {
        let mut boxes = in_a_row(&[0.0]);
        boxes.clear();
        boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        assert_eq!(
            boxes.slot_zero(),
            SlotBytes {
                scoopable: false,
                contents: None
            }
        );
    }

    #[test]
    fn a_slot_no_spawn_wrote_is_not_scoopable() {
        assert_eq!(
            CargoBoxes::default().slot_zero(),
            SlotBytes {
                scoopable: false,
                contents: None
            }
        );
    }

    #[test]
    fn taking_slot_0_frees_its_live_box_and_gives_its_bytes() {
        let mut boxes = in_a_row(&[0.0, 100.0]);
        let taken = boxes.take_slot_zero();
        assert_eq!(
            taken,
            SlotBytes {
                scoopable: true,
                contents: Some(0)
            }
        );
        assert_eq!(contents(&boxes), [Some(1)], "box 0 gone, box 1 kept");
        let stale = boxes.clone();
        assert_eq!(boxes.take_slot_zero(), taken, "dead: its last bytes");
        assert_eq!(boxes, stale, "and nothing else changes");
    }

    #[test]
    fn taking_slot_0_leaves_a_box_in_slot_1() {
        let frames = BoxFrames::default();
        let mut boxes = in_a_row(&[0.0, 100.0]);
        boxes.take_touching(&Scooping::disc(Vec2::ZERO, 1.0), &frames);
        boxes.take_slot_zero();
        assert_eq!(contents(&boxes), [Some(1)]);
    }
}
