//! A system's asteroid field: the 16 slots it holds, how an asteroid
//! spawns at the field's edge or in place, how it drifts and spins, and
//! when it is culled. Where they are drawn, and with which `spïn`, is the
//! view's.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`, `_asteroidData` holds 16
//! slots of 0x24 bytes: position (+4/+8), velocity (+0xC/+0x10), frame
//! (+0x14), spin (+0x18), strength (+0x1C), type (+0x1E, `röid` 128 +
//! type) and active (+0x20). `_InitObjects` clears them (@0x1cc6d) and
//! nothing saves them.
//!
//! - **The types** ([`AsteroidType`]): `_LoadObjectData`
//!   (@0x7dcaa-0x7dec3) keeps each `röid`'s strength, at least 1, and its
//!   spin, `SpinRate` × 0.01 frames a tick. Its master sprite is `spïn`
//!   800 + type (`_LoadSprites` @0x20bb7), `xTiles` × `yTiles` frames.
//! - **The spawn** ([`Asteroids::spawn`], `_SpawnAsteroid` @0x3a233):
//!   nothing when the system's `Asteroids` is 0 or less, its `AstTypes`
//!   is 0, or as many are active as `Asteroids` says (@0x3a25c-0x3a29b);
//!   otherwise the first free slot (@0x3a319-0x3a74c) takes a type whose
//!   bit `AstTypes` sets (@0x3a642-0x3a68a), a frame `Rand(frames)`
//!   (@0x3a69a), a spin of the type's × `(Rand(41) + 80)` / 100, negated
//!   on `Rand(2)` 0 (@0x3a6c4-0x3a721), and the type's strength
//!   (@0x3a738). Placed in the field ([`Origin::Placed`],
//!   @0x3a53d-0x3a627), it starts `Rand(W) − W/2` and `Rand(H) − H/2`
//!   off the player, `W` and `H` being `_screenCenter` + 128 (half the
//!   play area, whole, plus 128), moving `(Rand(400) − 200)` / 100 a tick
//!   on each axis. At the edge ([`Origin::Edge`], @0x3a351-0x3a538) it
//!   starts on the ring [`Ring`] says, drifting toward the player.
//! - **The placed field** ([`Asteroids::place_field`], `_CreateAsteroids`
//!   @0x3fa05): `Asteroids` 0 or less kills the field (`_killAsteroids`
//!   @0x3fbb7); otherwise it spawns in place `Asteroids` times
//!   (@0x3fa45-0x3fa75), then places every active slot afresh
//!   (@0x3fa77-0x3fbaf).
//! - **The step** ([`Asteroids::step`], `_HandleAsteroids` @0x361ac):
//!   each active asteroid moves by its velocity and turns by its spin
//!   (@0x36300-0x3636f), its frame wrapped once, up by the frame count
//!   below 0 or down by it at or past it (@0x363b2-0x363f6). It is
//!   culled, its slot freed, when its sprite's left edge on screen is
//!   past the play area's width + 32 or before −2 × its width − 32, or
//!   its top likewise with the height (@0x364e4-0x3661b): asteroids
//!   never wrap round.
//!
//! The randomness is the engine's (see the `chance` module): a type is
//! one roll over the set bits of `AstTypes`, where the original rolls
//! `Rand(16)` until it lands on one, which picks as evenly and can never
//! loop. A spawn draws its type first, then its place and velocity
//! (one axis, then the other), its frame, its spin's size and its sign.
//! No sound plays in any of these routines.
//!
//! Left out:
//!
//! - The slot-0 refill while `_miningShipsActive` is set
//!   (@0x361b5-0x3624b), which moves the first active asteroid into slot
//!   0: rdm:phase/asteroids/phase-5-ai-mining.
//! - The cull of an asteroid whose strength is −32000 or less, the
//!   destroyed marker (@0x362c4):
//!   rdm:phase/asteroids/phase-2-shoot-and-break-asteroids, which adds
//!   destruction.
//! - `_HandleSpriteMurkiness` (@0x36446): the engine draws no sprite
//!   murky (the cargo boxes leave it out too).
//! - The `_gameFrozen` skip (@0x36300-0x3636f) and the frozen game's call
//!   from `_PrepareAllSprites` (@0x3a7dd): the engine has no frozen game
//!   (see the `combat` module), and a paused session doesn't tick.
//! - The largest master's width and height (@0x3624d-0x362a2), which
//!   only dead writes read, and the repositioning writes of the cull.
//! - The anti-piracy block (@0x3a2a1-0x3a2f9), which writes the
//!   registration parity `_SpawnNagShip` (@0x607a) reads: never ported.

use crate::catalog::AsteroidTypeRecord;
use crate::chance::Chance;
use crate::cloak::Area;
use crate::geometry::Vec2;

/// The most asteroids in a field at once: `_asteroidData`'s 16 slots.
pub const SLOTS: usize = 16;

/// How many asteroid types there are: `röid` 128-143, one bit each of
/// `AstTypes`.
pub const KINDS: usize = 16;

/// How far past the play area's right and bottom edges, and before its
/// left and top edges less twice the sprite, an asteroid may go before
/// it is culled (@0x364e4-0x3661b).
pub const CULL_MARGIN: f32 = 32.0;

/// What `_screenCenter` is widened by for a spawn's spread and ring
/// (@0x3a3a1, @0x3a553).
const SPREAD_MARGIN: i32 = 128;

/// The edge ring's share of the widened screen centre (@0x3a3c9).
const RING_SCALE: f32 = 0.7;

/// The edge ring's least radius (@0x3a3e1).
const MIN_RING: f32 = 2.0;

/// The sides of an edge spawn's speed draw: `Rand(200)` hundredths of a
/// pixel a tick (@0x3a42b).
const EDGE_SPEED_SIDES: u16 = 200;

/// The sides of a placed spawn's velocity draw, `Rand(400) − 200`
/// hundredths (@0x3a5b9).
const PLACED_SPEED_SIDES: u16 = 400;

/// The sides of the spin's size draw: `Rand(41) + 80` hundredths of the
/// type's spin (@0x3a6c4).
const SPIN_SIDES: u16 = 41;

/// The least of the spin's size draw, in hundredths (@0x3a6cf).
const SPIN_BASE: u16 = 80;

/// One asteroid type as the field uses it, from its `röid` and its
/// master sprite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsteroidType {
    /// Its strength: `Strength`, at least 1.
    pub strength: i16,
    /// Its spin, in frames a tick: `SpinRate` / 100.
    pub spin: f32,
    /// Its master sprite's frame count.
    pub frames: u16,
    /// Its master sprite's frame width and height: none when the sheet
    /// does not read.
    pub size: (f32, f32),
}

impl AsteroidType {
    /// A type with no `röid`: strength 1, no spin, no frames, no size.
    pub const MISSING: Self = Self {
        strength: 1,
        spin: 0.0,
        frames: 0,
        size: (0.0, 0.0),
    };

    /// The type `record` reads as (`_LoadObjectData` @0x7dcaa-0x7dec3).
    #[must_use]
    pub fn from_record(record: &AsteroidTypeRecord) -> Self {
        Self {
            strength: record.strength.max(1),
            spin: f32::from(record.spin_rate) * 0.01,
            frames: record.frames,
            size: record
                .frame_size
                .map_or((0.0, 0.0), |(width, height)| (width as f32, height as f32)),
        }
    }

    /// Every type, by kind, from `records`: [`AsteroidType::MISSING`] for
    /// a kind with no record, and a record of a kind past the last left
    /// out.
    #[must_use]
    pub fn table(records: &[AsteroidTypeRecord]) -> [Self; KINDS] {
        let mut table = [Self::MISSING; KINDS];
        for record in records {
            if let Some(kind) = table.get_mut(usize::from(record.kind)) {
                *kind = Self::from_record(record);
            }
        }
        table
    }
}

/// A system's asteroid field: its `Asteroids` and `AstTypes`, raw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldSpec {
    /// Its `Asteroids`: how many it holds at most.
    pub count: i16,
    /// Its `AstTypes`: bit `n` for `röid` 128 + `n`.
    pub types: u16,
}

impl FieldSpec {
    /// No field: what a system the catalog doesn't list holds.
    pub const NONE: Self = Self { count: 0, types: 0 };
}

/// Where an edge spawn starts, as
/// [`RuleKey::AsteroidRespawnRing`](crate::RuleKey::AsteroidRespawnRing)
/// reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ring {
    /// The original's ring (@0x3a351-0x3a538): `r` = 0.7 × the larger of
    /// `_screenCenter` + 128 on each axis, at least 2; on each axis a
    /// side drawn with `Rand(2)`, then `r + Rand(trunc(r / 2))` out on
    /// it. On a wide play area `r` is less than half of it, so an
    /// asteroid can appear on screen.
    Original,
    /// Just off screen, inside the cull: on each axis a side drawn, then
    /// on the far side `ceil(edge + half) + Rand(32)` out, on the near
    /// side `floor(edge + half − size) − Rand(trunc(size) + 32)`, `half`
    /// being half its sprite, whole. Its sprite's edge on screen, which
    /// the cull reads, then lies from the play area's extent to 32 past
    /// it on the far side, and from −size to −2 × size − 32 on the near
    /// side: off screen, but never culled. A spawn comes before the step,
    /// and the step only moves it toward the player, so it survives its
    /// first cull and drifts on screen.
    Outside,
}

/// How a spawn places an asteroid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// In the field about the player, as `_CreateAsteroids` places it
    /// (`_SpawnAsteroid(false)`).
    Placed,
    /// At the field's edge, on this ring, drifting toward the player
    /// (`_SpawnAsteroid(true)`).
    Edge(Ring),
}

/// One asteroid in the field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Asteroid {
    /// Where it is, in pixels from the system's centre.
    pub position: Vec2,
    /// How far it moves each tick.
    pub velocity: Vec2,
    /// Its frame: drawn at its whole part.
    pub frame: f32,
    /// How many frames it turns each tick.
    pub spin: f32,
    /// Its strength.
    pub strength: i16,
    /// Its type, 0-15: `röid` 128 + `kind`, drawn with `spïn` 800 +
    /// `kind`.
    pub kind: u8,
}

/// The asteroid field: [`SLOTS`] slots, each holding an asteroid or
/// free.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asteroids {
    slots: [Option<Asteroid>; SLOTS],
}

impl Asteroids {
    /// The asteroids in the field, in slot order.
    pub fn iter(&self) -> impl Iterator<Item = &Asteroid> {
        self.slots.iter().flatten()
    }

    /// How many asteroids the field holds.
    #[must_use]
    pub fn active(&self) -> usize {
        self.iter().count()
    }

    /// Empties the field.
    pub fn clear(&mut self) {
        self.slots = [None; SLOTS];
    }

    /// The lowest free slot, if any (@0x3a319).
    #[must_use]
    pub fn first_free(&self) -> Option<usize> {
        self.slots.iter().position(Option::is_none)
    }

    /// Spawns one asteroid of `spec`'s field from `origin` about `player`
    /// in `area`, the play area about the player, with the types
    /// `types`, drawing on `chance`: `_SpawnAsteroid` (see the module
    /// docs). Nothing under the guards.
    pub fn spawn(
        &mut self,
        origin: Origin,
        spec: FieldSpec,
        types: &[AsteroidType; KINDS],
        player: Vec2,
        area: Area,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if spec.count <= 0 || spec.types == 0 {
            return;
        }
        if self.active() >= usize::try_from(spec.count).unwrap_or(0) {
            return;
        }
        let Some(slot) = self.first_free() else {
            return;
        };
        let kind = pick_kind(spec.types, chance);
        let kind_type = type_of(types, kind);
        let (offset, velocity) = match origin {
            Origin::Placed => placed(area, chance),
            Origin::Edge(Ring::Original) => {
                let r = ring_radius(area);
                let (x, vx) = ring_axis(r, chance);
                let (y, vy) = ring_axis(r, chance);
                (Vec2::new(x, y), Vec2::new(vx, vy))
            }
            Origin::Edge(Ring::Outside) => {
                let (width, height) = kind_type.size;
                let (x, vx) = outside_axis(area.left, area.right, width, chance);
                let (y, vy) = outside_axis(area.top, area.bottom, height, chance);
                (Vec2::new(x, y), Vec2::new(vx, vy))
            }
        };
        let frame = f32::from(chance.roll(kind_type.frames));
        let size = f32::from(chance.roll(SPIN_SIDES) + SPIN_BASE) * 0.01;
        let spin = kind_type.spin * size;
        let spin = if chance.roll(2) == 0 { -spin } else { spin };
        self.slots[slot] = Some(Asteroid {
            position: player + offset,
            velocity,
            frame,
            spin,
            strength: kind_type.strength,
            kind,
        });
    }

    /// Places `spec`'s field about `player` in `area`, with the types
    /// `types`, drawing on `chance`: `_CreateAsteroids` (see the module
    /// docs). An asteroid the field held already is placed afresh with
    /// the new ones.
    pub fn place_field(
        &mut self,
        spec: FieldSpec,
        types: &[AsteroidType; KINDS],
        player: Vec2,
        area: Area,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if spec.count <= 0 {
            self.clear();
            return;
        }
        // Past the slots, every spawn finds the field full.
        let spawns = usize::try_from(spec.count).unwrap_or(0).min(SLOTS);
        for _ in 0..spawns {
            self.spawn(Origin::Placed, spec, types, player, area, chance);
        }
        for asteroid in self.slots.iter_mut().flatten() {
            let (offset, velocity) = placed(area, chance);
            asteroid.position = player + offset;
            asteroid.velocity = velocity;
        }
    }

    /// Steps every asteroid one tick about `player` in `area`, with the
    /// types `types`: `_HandleAsteroids` (see the module docs).
    pub fn step(&mut self, types: &[AsteroidType; KINDS], player: Vec2, area: Area) {
        for slot in &mut self.slots {
            let Some(asteroid) = slot else {
                continue;
            };
            let kind_type = type_of(types, asteroid.kind);
            asteroid.position = asteroid.position + asteroid.velocity;
            asteroid.frame = wrapped(asteroid.frame + asteroid.spin, kind_type.frames);
            if culled(asteroid.position - player, kind_type.size, area) {
                *slot = None;
            }
        }
    }
}

/// `kind`'s type in `types`: the missing type past the last.
fn type_of(types: &[AsteroidType; KINDS], kind: u8) -> AsteroidType {
    types
        .get(usize::from(kind))
        .copied()
        .unwrap_or(AsteroidType::MISSING)
}

/// One of the kinds `types` sets a bit for, drawn on `chance` with one
/// roll over the set bits; `types` is never 0.
fn pick_kind(types: u16, chance: &mut (impl Chance + ?Sized)) -> u8 {
    let pick = chance.roll(types.count_ones() as u16);
    (0..KINDS as u8)
        .filter(|&kind| types & (1 << kind) != 0)
        .nth(usize::from(pick))
        .unwrap_or(types.trailing_zeros() as u8)
}

/// `_screenCenter`: half the play area's width and height, whole
/// (`_SetupWind` @0x1b738).
fn screen_centre(area: Area) -> (i32, i32) {
    (
        ((area.right - area.left) / 2.0).trunc() as i32,
        ((area.bottom - area.top) / 2.0).trunc() as i32,
    )
}

/// The sides of a draw over `extent` whole pixels: none below 0, and at
/// most a roll's.
fn sides(extent: i32) -> u16 {
    u16::try_from(extent.max(0)).unwrap_or(u16::MAX)
}

/// A placed spawn's offset from the player and velocity in `area`,
/// drawn on `chance` (@0x3a53d-0x3a627, @0x3fa77-0x3fbaf).
fn placed(area: Area, chance: &mut (impl Chance + ?Sized)) -> (Vec2, Vec2) {
    let (cx, cy) = screen_centre(area);
    let mut spread = |extent: i32| {
        let extent = extent + SPREAD_MARGIN;
        (i32::from(chance.roll(sides(extent))) - extent / 2) as f32
    };
    let offset = Vec2::new(spread(cx), spread(cy));
    let mut speed =
        || (f32::from(chance.roll(PLACED_SPEED_SIDES)) - f32::from(PLACED_SPEED_SIDES / 2)) * 0.01;
    let velocity = Vec2::new(speed(), speed());
    (offset, velocity)
}

/// The original ring's radius in `area` (@0x3a3a1-0x3a3e1).
fn ring_radius(area: Area) -> f32 {
    let (cx, cy) = screen_centre(area);
    (RING_SCALE * (cx.max(cy) + SPREAD_MARGIN) as f32).max(MIN_RING)
}

/// An edge spawn's speed toward the player, drawn on `chance`.
fn edge_speed(chance: &mut (impl Chance + ?Sized)) -> f32 {
    f32::from(chance.roll(EDGE_SPEED_SIDES)) * 0.01
}

/// One axis of a spawn on the original ring of radius `r`: its offset
/// from the player and its velocity, drawn on `chance`.
fn ring_axis(r: f32, chance: &mut (impl Chance + ?Sized)) -> (f32, f32) {
    let far = chance.roll(2) == 0;
    let out = r + f32::from(chance.roll(sides((r / 2.0).trunc() as i32)));
    let speed = edge_speed(chance);
    if far { (out, -speed) } else { (-out, speed) }
}

/// One axis of a spawn just outside the play area from `low` to `high`
/// for a sprite `size` across: its offset from the player and its
/// velocity, drawn on `chance` (see [`Ring::Outside`]).
fn outside_axis(low: f32, high: f32, size: f32, chance: &mut (impl Chance + ?Sized)) -> (f32, f32) {
    let half = (size / 2.0).trunc();
    let far = chance.roll(2) == 0;
    let out = if far {
        (high + half).ceil() + f32::from(chance.roll(CULL_MARGIN as u16))
    } else {
        let jitter = sides(size.trunc() as i32 + CULL_MARGIN as i32);
        (low + half - size).floor() - f32::from(chance.roll(jitter))
    };
    let speed = edge_speed(chance);
    if far { (out, -speed) } else { (out, speed) }
}

/// `frame` wrapped once into a sheet of `frames`: up by `frames` below
/// 0, down by it at or past it; as it is with no frames.
fn wrapped(frame: f32, frames: u16) -> f32 {
    let frames = f32::from(frames);
    if frames <= 0.0 {
        frame
    } else if frame < 0.0 {
        frame + frames
    } else if frame >= frames {
        frame - frames
    } else {
        frame
    }
}

/// Whether an asteroid at `offset` from the player, its sprite `size`,
/// is culled from `area` (@0x364df-0x3661b).
fn culled(offset: Vec2, size: (f32, f32), area: Area) -> bool {
    axis_culled(offset.x, size.0, area.left, area.right)
        || axis_culled(offset.y, size.1, area.top, area.bottom)
}

/// Whether a sprite `size` across at `offset` from the player is past
/// the cull on the axis the play area spans from `low` to `high`: its
/// edge on screen past the extent + 32, or before −2 × `size` − 32.
fn axis_culled(offset: f32, size: f32, low: f32, high: f32) -> bool {
    let edge = offset.trunc() - (size / 2.0).trunc() - low;
    edge > (high - low) + CULL_MARGIN || edge < -2.0 * size - CULL_MARGIN
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::cloak::DEFAULT_PLAY_AREA;
    use crate::testkit::Scripted;

    /// Every roll its last outcome.
    struct Last;

    impl Chance for Last {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            n.saturating_sub(1)
        }

        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    const PLAYER: Vec2 = Vec2 { x: 100.0, y: 200.0 };

    /// The original's smallest play area, 606 x 600: `_screenCenter`
    /// (303, 300).
    const AREA: Area = DEFAULT_PLAY_AREA;

    /// `r` on [`AREA`]: 0.7 × (303 + 128).
    const R: f32 = 0.7 * 431.0;

    /// Kind `k` of strength 10 × (`k` + 1), `SpinRate` 100 + 10 × `k`, 36
    /// frames of 40 x 30.
    fn types() -> [AsteroidType; KINDS] {
        std::array::from_fn(|kind| AsteroidType {
            strength: 10 * (kind as i16 + 1),
            spin: (100.0 + 10.0 * kind as f32) * 0.01,
            frames: 36,
            size: (40.0, 30.0),
        })
    }

    fn spec(count: i16, types: u16) -> FieldSpec {
        FieldSpec { count, types }
    }

    fn spawned(origin: Origin, spec: FieldSpec, chance: &mut impl Chance) -> Asteroid {
        let mut field = Asteroids::default();
        field.spawn(origin, spec, &types(), PLAYER, AREA, chance);
        assert_eq!(field.active(), 1);
        *field.iter().next().expect("one")
    }

    const ORIGINAL: Origin = Origin::Edge(Ring::Original);
    const OUTSIDE: Origin = Origin::Edge(Ring::Outside);

    #[test]
    fn a_type_takes_its_strength_at_least_1_and_its_spin_in_hundredths() {
        for (strength, kept) in [(0, 1), (-5, 1), (1, 1), (120, 120)] {
            let record = AsteroidTypeRecord {
                kind: 3,
                strength,
                spin_rate: 120,
                frames: 36,
                frame_size: Some((48, 40)),
            };
            assert_eq!(
                AsteroidType::from_record(&record),
                AsteroidType {
                    strength: kept,
                    // 120 × 0.01, as the original multiplies.
                    spin: 120.0 * 0.01,
                    frames: 36,
                    size: (48.0, 40.0),
                },
                "{strength}"
            );
        }
        let unread = AsteroidTypeRecord {
            spin_rate: -50,
            frames: 0,
            frame_size: None,
            ..AsteroidTypeRecord::default()
        };
        assert_eq!(
            AsteroidType::from_record(&unread),
            AsteroidType {
                strength: 1,
                spin: -50.0 * 0.01,
                frames: 0,
                size: (0.0, 0.0),
            }
        );
    }

    #[test]
    fn the_table_holds_each_kind_read_and_the_missing_type_for_the_rest() {
        let record = |kind| AsteroidTypeRecord {
            kind,
            strength: 7,
            spin_rate: 100,
            frames: 4,
            frame_size: Some((10, 10)),
        };
        let table = AsteroidType::table(&[record(4), record(0), record(16), record(200)]);
        assert_eq!(table[0], AsteroidType::from_record(&record(0)));
        assert_eq!(table[4], AsteroidType::from_record(&record(4)));
        for kind in [1, 2, 3, 5, 15] {
            assert_eq!(table[kind], AsteroidType::MISSING, "{kind}");
        }
    }

    #[test]
    fn nothing_spawns_without_a_count_or_types() {
        for spec in [spec(0, 1), spec(-1, 1), spec(4, 0)] {
            let mut field = Asteroids::default();
            field.spawn(ORIGINAL, spec, &types(), PLAYER, AREA, &mut NeverFires);
            assert_eq!(field.active(), 0, "{spec:?}");
        }
    }

    #[test]
    fn spawns_stop_once_the_count_is_reached() {
        let mut field = Asteroids::default();
        for _ in 0..5 {
            field.spawn(
                ORIGINAL,
                spec(3, 1),
                &types(),
                PLAYER,
                AREA,
                &mut NeverFires,
            );
        }
        assert_eq!(field.active(), 3);
    }

    #[test]
    fn never_more_than_16_spawn() {
        let mut field = Asteroids::default();
        for _ in 0..20 {
            field.spawn(
                ORIGINAL,
                spec(20, 1),
                &types(),
                PLAYER,
                AREA,
                &mut NeverFires,
            );
        }
        assert_eq!(field.active(), SLOTS);
        assert_eq!(field.first_free(), None);
        field.spawn(
            ORIGINAL,
            spec(20, 1),
            &types(),
            PLAYER,
            AREA,
            &mut NeverFires,
        );
        assert_eq!(field.active(), SLOTS);
    }

    /// A placed spawn's rolls: the type, x, y, vx, vy, the frame, the
    /// spin's size and its sign.
    fn placed_rolls(kind: u16, frame: u16, size: u16, sign: u16) -> Scripted {
        Scripted::rolling(&[kind, 0, 0, 0, 0, frame, size, sign])
    }

    #[test]
    fn the_type_is_one_of_the_bits_ast_types_sets() {
        for (types, roll, kind) in [
            (0x0011, 0, 0),
            (0x0011, 1, 4),
            (0x8000, 0, 15),
            (0x8421, 2, 10),
            (0x8421, 3, 15),
        ] {
            let mut chance = placed_rolls(roll, 0, 0, 0);
            let made = spawned(Origin::Placed, spec(4, types), &mut chance);
            assert_eq!(made.kind, kind, "{types:#06x} {roll}");
            assert_eq!(
                chance.sides_asked[0],
                types.count_ones() as u16,
                "one roll over the set bits"
            );
        }
    }

    #[test]
    fn a_spawn_takes_its_types_strength_a_frame_in_range_and_a_spin_of_80_to_120_percent() {
        let mut chance = placed_rolls(1, 35, 40, 1);
        let made = spawned(Origin::Placed, spec(4, 0x0011), &mut chance);
        assert_eq!(made.kind, 4);
        assert_eq!(made.strength, 50);
        assert_eq!(made.frame, 35.0);
        assert!((made.spin - 1.4 * 1.2).abs() < 1e-6, "{}", made.spin);
        assert_eq!(chance.sides_asked[5..], [36, 41, 2]);
        let mut chance = placed_rolls(0, 0, 0, 0);
        let made = spawned(Origin::Placed, spec(4, 0x0011), &mut chance);
        assert_eq!(made.strength, 10);
        assert_eq!(made.frame, 0.0);
        assert!((made.spin + 0.8).abs() < 1e-6, "negated: {}", made.spin);
    }

    #[test]
    fn a_type_with_no_frames_spawns_at_frame_0() {
        let mut table = types();
        table[0] = AsteroidType::MISSING;
        let mut field = Asteroids::default();
        field.spawn(ORIGINAL, spec(1, 1), &table, PLAYER, AREA, &mut Last);
        let made = field.iter().next().expect("spawned");
        assert_eq!((made.frame, made.spin, made.strength), (0.0, 0.0, 1));
    }

    /// An edge spawn's rolls: the type, then each axis's side, jitter and
    /// speed, then the frame, the spin's size and its sign.
    fn edge_rolls(x: [u16; 3], y: [u16; 3]) -> Scripted {
        Scripted::rolling(&[0, x[0], x[1], x[2], y[0], y[1], y[2], 0, 0, 0])
    }

    #[test]
    fn an_edge_spawn_on_the_original_ring_lies_at_least_r_out_on_both_axes() {
        let jitter = (R / 2.0).trunc() as u16;
        for side in [0, 1] {
            for extra in [0, jitter - 1] {
                let mut chance = edge_rolls([side, extra, 50], [1 - side, extra, 120]);
                let made = spawned(ORIGINAL, spec(4, 1), &mut chance);
                let off = made.position - PLAYER;
                assert!(
                    (off.x.abs() - (R + f32::from(extra))).abs() < 1e-3,
                    "{side} {extra}: {off:?}"
                );
                assert!(
                    (off.y.abs() - (R + f32::from(extra))).abs() < 1e-3,
                    "{side} {extra}: {off:?}"
                );
                assert!(off.x.abs() >= R && off.y.abs() >= R);
                // Toward the player on both axes.
                assert!(made.velocity.x * off.x < 0.0, "{made:?}");
                assert!(made.velocity.y * off.y < 0.0, "{made:?}");
                assert!((made.velocity.x.abs() - 0.5).abs() < 1e-6);
                assert!((made.velocity.y.abs() - 1.2).abs() < 1e-6);
                assert_eq!(chance.sides_asked[1..7], [2, jitter, 200, 2, jitter, 200]);
            }
        }
    }

    #[test]
    fn the_original_ring_is_at_least_2_out() {
        let tiny = Area::centred(Vec2::ZERO, 0.0, 0.0);
        let mut field = Asteroids::default();
        field.spawn(
            ORIGINAL,
            spec(1, 1),
            &types(),
            PLAYER,
            tiny,
            &mut NeverFires,
        );
        let off = field.iter().next().expect("spawned").position - PLAYER;
        assert!((off.x - 0.7 * 128.0).abs() < 1e-3, "{off:?}");
        let mut field = Asteroids::default();
        let mut chance = Scripted::rolling(&[0, 1, 0, 0, 1, 0, 0]);
        let negative = Area {
            left: 400.0,
            top: 400.0,
            right: -400.0,
            bottom: -400.0,
        };
        field.spawn(
            ORIGINAL,
            spec(1, 1),
            &types(),
            PLAYER,
            negative,
            &mut chance,
        );
        let off = field.iter().next().expect("spawned").position - PLAYER;
        assert_eq!(off, Vec2::new(-MIN_RING, -MIN_RING), "{off:?}");
    }

    fn one(position: Vec2, velocity: Vec2, frame: f32, spin: f32) -> Asteroids {
        let mut field = Asteroids::default();
        field.slots[0] = Some(Asteroid {
            position,
            velocity,
            frame,
            spin,
            strength: 10,
            kind: 0,
        });
        field
    }

    fn first(field: &Asteroids) -> Asteroid {
        *field.iter().next().expect("one")
    }

    #[test]
    fn a_step_moves_each_asteroid_by_its_velocity_and_turns_it_by_its_spin() {
        let mut field = one(PLAYER, Vec2::new(0.5, -1.5), 3.0, 0.75);
        field.step(&types(), PLAYER, AREA);
        let stepped = first(&field);
        assert_eq!(stepped.position, PLAYER + Vec2::new(0.5, -1.5));
        assert_eq!(stepped.velocity, Vec2::new(0.5, -1.5));
        assert_eq!(stepped.frame, 3.75);
        field.step(&types(), PLAYER, AREA);
        assert_eq!(first(&field).position, PLAYER + Vec2::new(1.0, -3.0));
        assert_eq!(first(&field).frame, 4.5);
    }

    #[test]
    fn the_frame_wraps_once_at_either_end() {
        let mut table = types();
        table[0].frames = 10;
        for (frame, spin, wrapped) in [
            (9.5, 1.0, 0.5),
            (0.25, -1.0, 9.25),
            (9.0, 0.5, 9.5),
            (0.0, 0.0, 0.0),
            // Past a whole sheet, it wraps only once.
            (0.0, 25.0, 15.0),
            (0.0, -25.0, -15.0),
        ] {
            let mut field = one(PLAYER, Vec2::ZERO, frame, spin);
            field.step(&table, PLAYER, AREA);
            assert_eq!(first(&field).frame, wrapped, "{frame} {spin}");
        }
        table[0].frames = 0;
        let mut field = one(PLAYER, Vec2::ZERO, 5.0, 2.0);
        field.step(&table, PLAYER, AREA);
        assert_eq!(first(&field).frame, 7.0, "no frames, no wrap");
    }

    /// Whether an asteroid of kind 0 (40 x 30) standing at `off` from the
    /// player survives a step.
    fn survives(off: Vec2) -> bool {
        let mut field = one(PLAYER + off, Vec2::ZERO, 0.0, 0.0);
        field.step(&types(), PLAYER, AREA);
        field.active() == 1
    }

    #[test]
    fn an_asteroid_past_the_cull_on_any_side_is_culled() {
        // Its left edge on screen is trunc(x) − 20 + 303: past 606 + 32,
        // or before −80 − 32, it goes; its top is trunc(y) − 15 + 300:
        // past 600 + 32, or before −60 − 32.
        for (off, kept) in [
            (Vec2::new(355.0, 0.0), true),
            (Vec2::new(355.9, 0.0), true),
            (Vec2::new(356.0, 0.0), false),
            (Vec2::new(-395.0, 0.0), true),
            (Vec2::new(-395.9, 0.0), true),
            (Vec2::new(-396.0, 0.0), false),
            (Vec2::new(0.0, 347.0), true),
            (Vec2::new(0.0, 348.0), false),
            (Vec2::new(0.0, -377.0), true),
            (Vec2::new(0.0, -378.0), false),
        ] {
            assert_eq!(survives(off), kept, "{off:?}");
        }
    }

    #[test]
    fn a_culled_slot_is_refilled_by_the_next_spawn() {
        let mut field = Asteroids::default();
        for _ in 0..3 {
            field.spawn(
                Origin::Placed,
                spec(3, 1),
                &types(),
                PLAYER,
                AREA,
                &mut NeverFires,
            );
        }
        field.slots[0].as_mut().expect("spawned").position = PLAYER + Vec2::new(1000.0, 0.0);
        field.step(&types(), PLAYER, AREA);
        assert_eq!(field.active(), 2);
        assert_eq!(field.first_free(), Some(0));
        field.spawn(
            ORIGINAL,
            spec(3, 1),
            &types(),
            PLAYER,
            AREA,
            &mut NeverFires,
        );
        assert_eq!(field.active(), 3);
        assert!(field.slots[0].is_some(), "the lowest free slot");
    }

    #[test]
    fn a_field_of_none_or_fewer_is_emptied() {
        for count in [0, -1] {
            let mut field = one(PLAYER, Vec2::ZERO, 0.0, 0.0);
            field.place_field(spec(count, 1), &types(), PLAYER, AREA, &mut NeverFires);
            assert_eq!(field.active(), 0, "{count}");
        }
    }

    #[test]
    fn a_placed_field_holds_its_count_within_the_spread_about_the_player() {
        // W = 303 + 128 = 431, H = 300 + 128 = 428.
        for (bounds, mut chance) in [
            (
                (Vec2::new(-215.0, -214.0), Vec2::new(-2.0, -2.0)),
                Box::new(NeverFires) as Box<dyn Chance>,
            ),
            (
                (Vec2::new(215.0, 213.0), Vec2::new(1.99, 1.99)),
                Box::new(Last),
            ),
        ] {
            let mut field = Asteroids::default();
            field.place_field(spec(5, 0x0003), &types(), PLAYER, AREA, &mut *chance);
            assert_eq!(field.active(), 5);
            for made in field.iter() {
                let off = made.position - PLAYER;
                assert!((off - bounds.0).length() < 1e-3, "{off:?}");
                assert!((made.velocity - bounds.1).length() < 1e-5, "{made:?}");
            }
        }
    }

    #[test]
    fn a_placed_field_places_an_asteroid_it_held_afresh() {
        let mut field = one(
            PLAYER + Vec2::new(900.0, 0.0),
            Vec2::new(5.0, 5.0),
            2.0,
            1.0,
        );
        field.place_field(spec(2, 0x0002), &types(), PLAYER, AREA, &mut NeverFires);
        assert_eq!(field.active(), 2);
        let held = first(&field);
        assert_eq!(held.kind, 0, "its type kept");
        assert_eq!(held.position - PLAYER, Vec2::new(-215.0, -214.0));
        assert_eq!(held.velocity, Vec2::new(-2.0, -2.0));
    }

    /// The left (or top) edge of an asteroid `size` across at `offset`
    /// from the player, on screen in the play area from `low`: what the
    /// cull reads.
    fn edge(offset: f32, size: f32, low: f32) -> f32 {
        offset.trunc() - (size / 2.0).trunc() - low
    }

    #[test]
    fn an_outside_spawn_lies_off_screen_inside_the_cull_on_each_side() {
        let width = AREA.right - AREA.left;
        let height = AREA.bottom - AREA.top;
        let (w, h) = (40.0, 30.0);
        for (side, jitter, y_jitter) in [(0, 0, 0), (0, 31, 31), (1, 0, 0), (1, 71, 61)] {
            let mut chance = edge_rolls([side, jitter, 0], [side, y_jitter, 0]);
            let made = spawned(OUTSIDE, spec(4, 1), &mut chance);
            let off = made.position - PLAYER;
            let left = edge(off.x, w, AREA.left);
            let top = edge(off.y, h, AREA.top);
            if side == 0 {
                assert!(
                    (width..=width + 32.0).contains(&left),
                    "{side} {jitter}: {left}"
                );
                assert!(
                    (height..=height + 32.0).contains(&top),
                    "{side} {jitter}: {top}"
                );
            } else {
                assert!(
                    (-2.0 * w - 32.0..=-w).contains(&left),
                    "{side} {jitter}: {left}"
                );
                assert!(
                    (-2.0 * h - 32.0..=-h).contains(&top),
                    "{side} {jitter}: {top}"
                );
            }
            assert_eq!(
                chance.sides_asked[1..7],
                if side == 0 {
                    [2, 32, 200, 2, 32, 200]
                } else {
                    [2, 72, 200, 2, 62, 200]
                }
            );
            // Still, it survives the cull.
            let mut field = Asteroids::default();
            field.slots[0] = Some(made);
            field.step(&types(), PLAYER, AREA);
            assert_eq!(field.active(), 1, "{side} {jitter}");
        }
    }

    #[test]
    fn an_outside_spawn_survives_its_first_tick_and_drifts_on_screen() {
        let width = AREA.right - AREA.left;
        for side in [0, 1] {
            let mut chance = edge_rolls([side, 31, 199], [side, 0, 199]);
            let mut field = Asteroids::default();
            field.spawn(OUTSIDE, spec(1, 1), &types(), PLAYER, AREA, &mut chance);
            field.step(&types(), PLAYER, AREA);
            assert_eq!(field.active(), 1, "its first tick");
            for _ in 0..60 {
                field.step(&types(), PLAYER, AREA);
            }
            let made = first(&field);
            let left = edge(made.position.x - PLAYER.x, 40.0, AREA.left);
            assert!(left < width && left > -40.0, "{side}: on screen at {left}");
        }
    }

    #[test]
    fn an_outside_spawn_on_a_wide_play_area_is_still_off_screen() {
        let wide = Area::centred(Vec2::ZERO, 1800.0, 1100.0);
        for side in [0, 1] {
            let mut chance = edge_rolls([side, 0, 0], [side, 0, 0]);
            let mut field = Asteroids::default();
            field.spawn(OUTSIDE, spec(1, 1), &types(), PLAYER, wide, &mut chance);
            let off = first(&field).position - PLAYER;
            assert!(off.x.abs() >= 900.0 + 20.0, "{off:?}");
            assert!(off.y.abs() >= 550.0 + 15.0, "{off:?}");
            field.step(&types(), PLAYER, wide);
            assert_eq!(field.active(), 1);
        }
    }

    #[test]
    fn an_edge_spawn_on_the_original_ring_can_land_on_a_wide_screen() {
        let wide = Area::centred(Vec2::ZERO, 1800.0, 1100.0);
        let mut field = Asteroids::default();
        field.spawn(
            ORIGINAL,
            spec(1, 1),
            &types(),
            PLAYER,
            wide,
            &mut NeverFires,
        );
        let off = first(&field).position - PLAYER;
        // r = 0.7 × (900 + 128) = 719.6, inside the 900 half-width.
        assert!(off.x.abs() < 900.0, "{off:?}");
    }

    #[test]
    fn clearing_empties_the_field() {
        let mut field = one(PLAYER, Vec2::ZERO, 0.0, 0.0);
        field.clear();
        assert_eq!(field.active(), 0);
        assert_eq!(field.first_free(), Some(0));
    }
}
