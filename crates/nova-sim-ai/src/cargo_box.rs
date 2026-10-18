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
//!   none when all are taken; the ship's position and velocity; a life of
//!   `Rand(90) + 180` ticks; frame `Rand(36)`; a spin of −1, 0, 0 or +1
//!   by `Rand(4)`; graphic 0, holding nothing and not scoopable. It is
//!   moved half the ship's sprite width straight behind it (32 wide
//!   without a sprite, `_GetSpriteXSize` @0x26fc), and kicked
//!   `(Rand(40) + 30) / 100` px a tick from `Rand(30) − Rand(15)` degrees
//!   off straight behind, both along the whole-degree heading.
//! - **The scoopable box** ([`CargoBoxes::spawn_scoopable`],
//!   `_SpawnScoopableBox` @0x3e3c5-0x3e54f, called only by
//!   `_DestroyAsteroid` @0xae73): the first free slot of 64, none when all
//!   are taken; where the asteroid was, from rest; a life of `Rand(200) +
//!   300` ticks; frame `Rand(36)`; a spin of −1, 0, 0 or +1 by `Rand(4)`;
//!   scoopable (+0x24), holding the asteroid's `YieldType` (+0x22), of
//!   the graphic given, 1-4 (+0x1e); kicked `(Rand(81) + 60) × 0.2 ×
//!   0.01` px a tick along `Rand(360)`, 0.12-0.28. A jettisoned box is
//!   written not scoopable (`_SpawnBox` @0x3de76).
//! - **The step** ([`CargoBoxes::step`]): a box whose life is below 0 is
//!   gone (life −2.0, @0x2dfa4-0x2dfee); otherwise its frame turns by its
//!   spin, wrapped into `[0, 36)`, its life falls by 1 and it moves by its
//!   velocity (@0x2dff3-0x2e0bf). Over its last 32 ticks it fades
//!   ([`CargoBox::fade`], @0x2e119-0x2e157).
//!
//! The session clears them on every system change and every landing,
//! where the original sets `_killBoxes` for the next `_HandleBoxes`.
//! Boxes are never saved.
//!
//! Spawning the scoopable boxes is
//! rdm:phase/asteroids/phase-2-shoot-and-break-asteroids's; their pickup
//! is [`scoop`](crate::scoop)'s ([`CargoBoxes::take_touching`]). Left
//! out: the AI that steers for them
//! (rdm:phase/asteroids/phase-5-ai-mining); the player's wrap shift; and
//! the sprite murk, which the view draws for shots alone so far
//! (rdm:task/murk-other-sprites).

use crate::chance::Chance;
use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::scoop::{BoxFrames, touches};

/// The most boxes in space at once: the table's 64 slots.
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
    /// How far it moves each tick.
    pub velocity: Vec2,
    /// The ticks left before it is gone.
    pub life: f32,
    /// Its frame, in `[0, 36)`.
    pub frame: u16,
    /// How many frames it turns each tick: −1, 0 or +1.
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

/// The cargo boxes in space, at most [`MAX_BOXES`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CargoBoxes {
    boxes: Vec<CargoBox>,
}

impl CargoBoxes {
    /// The boxes in space.
    #[must_use]
    pub fn boxes(&self) -> &[CargoBox] {
        &self.boxes
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
    /// docs), drawing on `chance`; nothing when the table is full.
    pub fn spawn(
        &mut self,
        ship: &ShipState,
        sprite_width: Option<f32>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.boxes.len() >= MAX_BOXES {
            return;
        }
        let life = f32::from(chance.roll(90) + 180);
        let frame = chance.roll(FRAMES);
        let spin = spin_of(chance.roll(4));
        // The whole-degree heading, in [0, 360).
        #[allow(clippy::cast_possible_truncation)]
        let heading = ship.heading.trunc() as i32;
        let behind = (sprite_width.unwrap_or(NO_SPRITE_WIDTH) * 0.5).trunc();
        let position = ship.position + accel(reversed(heading), behind);
        let speed = f32::from(chance.roll(40) + 30) / 100.0;
        let off = i32::from(chance.roll(30)) - i32::from(chance.roll(15));
        let velocity = ship.velocity + accel(reversed(heading + off), speed);
        self.boxes.push(CargoBox {
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
    /// `chance`; nothing when the table is full.
    pub fn spawn_scoopable(
        &mut self,
        at: Vec2,
        contents: Option<i16>,
        graphic: u16,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.boxes.len() >= MAX_BOXES {
            return;
        }
        let life = f32::from(chance.roll(SCOOPABLE_LIFE_SIDES) + SCOOPABLE_LIFE_BASE);
        let frame = chance.roll(FRAMES);
        let spin = spin_of(chance.roll(4));
        let angle = i32::from(chance.roll(360));
        let kick = f64::from(chance.roll(SCOOPABLE_KICK_SIDES) + SCOOPABLE_KICK_BASE) * 0.2 * 0.01;
        // A third of a pixel at most.
        #[allow(clippy::cast_possible_truncation)]
        let velocity = accel(angle, kick as f32);
        self.boxes.push(CargoBox {
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

    /// Steps every box one tick as `_HandleBoxes` does: one whose life is
    /// below 0 is gone; the others turn, age and move.
    pub fn step(&mut self) {
        self.boxes.retain(|cargo_box| cargo_box.life >= 0.0);
        for cargo_box in &mut self.boxes {
            cargo_box.frame =
                (cargo_box.frame + FRAMES).wrapping_add_signed(cargo_box.spin.into()) % FRAMES;
            cargo_box.life -= 1.0;
            cargo_box.position = cargo_box.position + cargo_box.velocity;
        }
    }

    /// Whether any box is scoopable: the original's `_miningBoxesActive`,
    /// which gates the scoop's collision (see [`scoop`](crate::scoop)).
    #[must_use]
    pub fn any_scoopable(&self) -> bool {
        self.boxes.iter().any(|cargo_box| cargo_box.scoopable)
    }

    /// Takes the first scoopable box, in table order, that a ship's disc,
    /// `radius` about `at`, touches by its graphic's frame in `frames`
    /// ([`touches`]); none when no scoopable box does. A jettisoned box is
    /// never taken.
    ///
    /// The box leaves the table at once. The original sets its life to −1
    /// and hides its sprite (@0x37119-0x3712e), and the next
    /// `_HandleBoxes` kills it, so no frame is drawn with it either way.
    pub fn take_touching(&mut self, at: Vec2, radius: f32, frames: &BoxFrames) -> Option<CargoBox> {
        let index = self.boxes.iter().position(|cargo_box| {
            cargo_box.scoopable
                && touches(
                    at,
                    radius,
                    cargo_box.position,
                    frames.half(cargo_box.graphic),
                )
        })?;
        Some(self.boxes.remove(index))
    }

    /// Empties the table: every box is gone.
    pub fn clear(&mut self) {
        self.boxes.clear();
    }
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
            (0, 0, 0, (180.0, 0, -1)),
            (89, 35, 1, (269.0, 35, 0)),
            (10, 7, 2, (190.0, 7, 0)),
            (0, 0, 3, (180.0, 0, 1)),
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
        assert_eq!(made.frame, 7);
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

    #[test]
    fn a_full_table_spawns_no_scoopable_box() {
        let mut boxes = CargoBoxes::default();
        for _ in 0..MAX_BOXES + 3 {
            boxes.spawn_scoopable(Vec2::ZERO, None, 1, &mut Scripted::default());
        }
        assert_eq!(boxes.boxes().len(), MAX_BOXES);
    }

    #[test]
    fn a_full_table_spawns_nothing() {
        let mut boxes = CargoBoxes::default();
        for _ in 0..MAX_BOXES + 3 {
            boxes.spawn(&heading(0.0), None, &mut Scripted::default());
        }
        assert_eq!(boxes.boxes().len(), MAX_BOXES);
    }

    #[test]
    fn a_ship_drops_as_many_boxes_as_its_share_counts() {
        for (share, count) in [(0.0, 0), (1.0, 1), (20.0, 4), (100.0, 12)] {
            let mut boxes = CargoBoxes::default();
            boxes.drop_behind(&heading(0.0), None, share, &mut Scripted::default());
            assert_eq!(boxes.boxes().len(), count, "{share}");
        }
    }

    fn one(life: f32, frame: u16, spin: i8) -> CargoBoxes {
        CargoBoxes {
            boxes: vec![CargoBox {
                position: Vec2::new(10.0, 20.0),
                velocity: Vec2::new(0.5, -1.5),
                life,
                frame,
                spin,
                graphic: 0,
                scoopable: false,
                contents: None,
            }],
        }
    }

    #[test]
    fn a_step_turns_ages_and_moves_each_box() {
        let mut boxes = one(100.0, 7, 1);
        boxes.step();
        let stepped = boxes.boxes()[0];
        assert_eq!(stepped.frame, 8);
        assert_eq!(stepped.life, 99.0);
        assert_eq!(stepped.position, Vec2::new(10.5, 18.5));
        assert_eq!(stepped.velocity, Vec2::new(0.5, -1.5));
    }

    #[test]
    fn the_frame_wraps_both_ways_and_a_still_box_keeps_it() {
        let mut boxes = one(100.0, 35, 1);
        boxes.step();
        assert_eq!(boxes.boxes()[0].frame, 0);
        let mut boxes = one(100.0, 0, -1);
        boxes.step();
        assert_eq!(boxes.boxes()[0].frame, 35);
        let mut boxes = one(100.0, 12, 0);
        boxes.step();
        assert_eq!(boxes.boxes()[0].frame, 12);
    }

    #[test]
    fn a_box_is_gone_on_the_step_after_its_life_falls_below_0() {
        let mut boxes = one(0.5, 0, 0);
        boxes.step();
        assert_eq!(boxes.boxes()[0].life, -0.5, "still shown");
        boxes.step();
        assert!(boxes.boxes().is_empty());
        let mut boxes = one(0.0, 0, 0);
        boxes.step();
        assert_eq!(boxes.boxes().len(), 1, "a life of exactly 0 steps on");
    }

    #[test]
    fn a_box_lasts_its_life_and_one_tick_more() {
        let mut boxes = CargoBoxes::default();
        boxes.spawn(&heading(0.0), None, &mut rolls(0, 0, 0, 0, 0, 0));
        for _ in 0..181 {
            boxes.step();
        }
        assert_eq!(boxes.boxes().len(), 1);
        boxes.step();
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
            assert_eq!(one(life, 0, 0).boxes()[0].fade(), fade, "{life}");
        }
    }

    /// A table of a jettisoned box at the origin, then scoopable boxes
    /// of graphic 1 holding each of `contents` at each of `at`.
    fn with_scoopable(at: &[(f32, f32)], contents: &[i16]) -> CargoBoxes {
        let mut boxes = one(100.0, 0, 0);
        boxes.boxes[0].position = Vec2::ZERO;
        for (&(x, y), &held) in at.iter().zip(contents) {
            boxes.spawn_scoopable(Vec2::new(x, y), Some(held), 1, &mut Scripted::default());
        }
        boxes
    }

    #[test]
    fn only_a_scoopable_box_makes_the_table_scoopable() {
        assert!(!CargoBoxes::default().any_scoopable());
        assert!(!one(100.0, 0, 0).any_scoopable(), "a jettisoned box");
        assert!(with_scoopable(&[(50.0, 0.0)], &[1]).any_scoopable());
    }

    #[test]
    fn a_disc_takes_the_first_scoopable_box_it_touches_and_leaves_the_rest() {
        let frames = BoxFrames::default();
        let mut boxes = with_scoopable(&[(90.0, 0.0), (5.0, 0.0), (-5.0, 0.0)], &[0, 1, 2]);
        let taken = boxes.take_touching(Vec2::ZERO, 10.0, &frames);
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
        let taken = boxes.take_touching(Vec2::ZERO, 10.0, &frames);
        assert_eq!(taken.map(|made| made.contents), Some(Some(2)));
        assert_eq!(boxes.take_touching(Vec2::ZERO, 10.0, &frames), None);
        assert_eq!(boxes.boxes().len(), 2);
    }

    #[test]
    fn a_disc_touches_a_box_by_its_graphics_frame() {
        // Graphic 1 is 40 across: the box at 45 is 25 from its edge.
        let frames = BoxFrames::new([None, Some((40, 40)), None, None, None]);
        let mut boxes = with_scoopable(&[(45.0, 0.0)], &[3]);
        assert_eq!(boxes.take_touching(Vec2::ZERO, 24.0, &frames), None);
        assert!(boxes.take_touching(Vec2::ZERO, 25.0, &frames).is_some());
    }

    #[test]
    fn clearing_empties_the_table() {
        let mut boxes = one(100.0, 0, 0);
        boxes.clear();
        assert!(boxes.boxes().is_empty());
    }
}
