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
//! - **The destruction** ([`Asteroids::destroy`], `_DestroyAsteroid`
//!   @0xadc9-0xb028): a shot that leaves an asteroid's strength below 0
//!   destroys it at once (`_HandleAsteroidHit` @0x3691a, see
//!   [`asteroid_hit`](crate::combat::asteroid_hit)). It drops scoopable
//!   boxes, throws off its particles, explodes and breaks into fragments
//!   ([`Asteroids::spawn_fragment`], `_SpawnSubAsteroid`
//!   @0x3fbc7-0x3fd21), and its slot is freed ([`Remains`]).
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
//! - The cull of an asteroid whose strength is −32000 or less
//!   (@0x362c4): nothing in the original writes that marker to an
//!   asteroid's strength. The compare in `_HandleAsteroids` (@0x362ca) is
//!   its only read, and each −32000 written elsewhere (@0x9ad1, @0xc3d9,
//!   @0x97c64) is to another table. A hit below 0 destroys the asteroid
//!   at once, so the cull is moot.
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
use crate::combat::ParticleBurst;
use crate::combat::weapon::Explosion;
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

/// The width and height `_DestroyAsteroid` takes for an asteroid with
/// no sprite (32, @0xaea7-0xaf3a).
pub const NO_SPRITE_SIZE: f32 = 32.0;

/// The hit radius of an asteroid with no sprite: half of
/// [`NO_SPRITE_SIZE`].
pub const NO_SPRITE_RADIUS: f32 = NO_SPRITE_SIZE / 2.0;

/// What `_screenCenter` is widened by for a spawn's spread and ring
/// (@0x3a3a1, @0x3a553).
const SPREAD_MARGIN: i32 = 128;

/// The sides of a fragment's velocity draw on each axis, `Rand(200) −
/// 100` hundredths of a pixel a tick (@0x3fbc7-0x3fd21).
const FRAGMENT_SPEED_SIDES: u16 = 200;

/// The sides of the box count's draw: `YieldQty × (Rand(101) + 50)`
/// hundredths (@0xadc9-0xae73).
const YIELD_SIDES: u16 = 101;

/// The least of the box count's draw, in hundredths of `YieldQty`.
const YIELD_BASE: u16 = 50;

/// A destroyed asteroid's particles' speed, in pixels a tick
/// (@0xaea7-0xaf3a).
const DEBRIS_SPEED: f32 = 0.2;

/// How far, in percent, a destroyed asteroid's particles' speed varies
/// either way.
const DEBRIS_VARIATION: u32 = 40;

/// A destroyed asteroid's particles' least and most life, in ticks.
const DEBRIS_LIFE: (u32, u32) = (240, 480);

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
    /// The boxes it drops on average as it is destroyed: its `YieldQty`,
    /// none below none.
    pub yield_qty: u16,
    /// What its boxes hold: its `YieldType`, a commodity 0-6 or a `jünk`
    /// 1000-1127; none for any other, or when its `YieldQty` is below
    /// none.
    pub yield_type: Option<i16>,
    /// The kinds it breaks into: its `FragType1` and `FragType2`, each
    /// none unless it names `röid` 128-143 or a kind 0-15.
    pub fragments: [Option<u8>; 2],
    /// The fragments it breaks into on average: its `FragCount`.
    pub frag_count: i16,
    /// The particles it throws off as it is destroyed: its `PartCount`.
    pub part_count: i16,
    /// Their colour, `00RRGGBB`: its `PartColor`.
    pub part_color: u32,
    /// What a shot's `Impact` pushes against: its `Mass`.
    pub mass: i16,
    /// Its explosion as it is destroyed: its `ExplodeType`, if any.
    pub explosion: Option<Explosion>,
}

/// The highest commodity a `YieldType` may name (@0x7dd37-0x7dd7a).
const LAST_COMMODITY_YIELD: i16 = 6;

/// The `jünk` a `YieldType` may name: 1000-1127 (@0x7dd37-0x7dd7a).
const JUNK_YIELDS: std::ops::RangeInclusive<i16> = 1000..=1127;

/// The first `röid`: a `FragType` of 128-143 names its kind as an ID
/// (@0x7de2e-0x7de4c).
const FIRST_ROID: i16 = 128;

impl AsteroidType {
    /// A type with no `röid`: strength 1, no spin, no frames, no size, and
    /// nothing to yield, break into or explode in.
    pub const MISSING: Self = Self {
        strength: 1,
        spin: 0.0,
        frames: 0,
        size: (0.0, 0.0),
        yield_qty: 0,
        yield_type: None,
        fragments: [None, None],
        frag_count: 0,
        part_count: 0,
        part_color: 0,
        mass: 0,
        explosion: None,
    };

    /// The type `record` reads as (`_LoadObjectData` @0x7dcaa-0x7deda):
    /// its strength at least 1; its `YieldType` kept only for 0-6 and
    /// 1000-1127, and both it and its `YieldQty` dropped when the
    /// quantity is below none (@0x7dd37-0x7dd7a); each `FragType` of
    /// 128-143 or 0-15 as its kind (@0x7de2e-0x7de4c).
    #[must_use]
    pub fn from_record(record: &AsteroidTypeRecord) -> Self {
        let kept = record.yield_type;
        let yield_type = ((0..=LAST_COMMODITY_YIELD).contains(&kept)
            || JUNK_YIELDS.contains(&kept))
        .then_some(kept)
        .filter(|_| record.yield_qty >= 0);
        Self {
            strength: record.strength.max(1),
            spin: f32::from(record.spin_rate) * 0.01,
            frames: record.frames,
            size: record
                .frame_size
                .map_or((0.0, 0.0), |(width, height)| (width as f32, height as f32)),
            yield_qty: u16::try_from(record.yield_qty).unwrap_or(0),
            yield_type,
            fragments: record.frag_types.map(fragment_kind),
            frag_count: record.frag_count,
            part_count: record.part_count,
            part_color: record.part_color,
            mass: record.mass,
            explosion: Explosion::decode(record.explode_type),
        }
    }

    /// Its hit radius, in pixels: half the larger of its frame's width
    /// and height, or [`NO_SPRITE_RADIUS`] with no sprite. The original
    /// tests a shot against its sprite pixel by pixel; the engine has no
    /// masks, and takes the disc the sprite fills.
    #[must_use]
    pub fn radius(&self) -> f32 {
        let (width, height) = self.size;
        if width <= 0.0 && height <= 0.0 {
            NO_SPRITE_RADIUS
        } else {
            width.max(height) / 2.0
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

/// What a destroyed asteroid leaves, for the session to put in space
/// (`_DestroyAsteroid` @0xadc9-0xb028, see [`Asteroids::destroy`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Remains {
    /// Where it was.
    pub at: Vec2,
    /// How many scoopable boxes it drops.
    pub boxes: u32,
    /// What they hold: its type's `YieldType`.
    pub contents: Option<i16>,
    /// Their graphic: its kind / 4, plus 1, so 1-4 (@0xae52-0xae5f).
    pub graphic: u16,
    /// The particles it throws off, if its type has a `PartCount`.
    pub particles: Option<ParticleBurst>,
    /// Its type's explosion, if any.
    pub explosion: Option<Explosion>,
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

    /// The asteroids in the field, each with its slot, in slot order.
    pub fn indexed(&self) -> impl Iterator<Item = (usize, &Asteroid)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, asteroid)| Some((slot, asteroid.as_ref()?)))
    }

    /// Puts `asteroid` in the first free slot, and gives the slot; none
    /// when the field is full.
    pub(crate) fn place(&mut self, asteroid: Asteroid) -> Option<usize> {
        let slot = self.first_free()?;
        self.slots[slot] = Some(asteroid);
        Some(slot)
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
        let spin = spun(kind_type, chance);
        self.slots[slot] = Some(Asteroid {
            position: player + offset,
            velocity,
            frame,
            spin,
            strength: kind_type.strength,
            kind,
        });
    }

    /// Spawns a fragment of `kind` at `at`, with the types `types`,
    /// drawing on `chance`: `_SpawnSubAsteroid` (@0x3fbc7-0x3fd21). It
    /// takes the first free slot, and is dropped when there is none; it
    /// moves `(Rand(200) − 100)` hundredths of a pixel a tick on each
    /// axis, its own and not its parent's, from a frame `Rand(frames)`,
    /// spinning and as strong as a spawn of its type.
    pub fn spawn_fragment(
        &mut self,
        at: Vec2,
        kind: u8,
        types: &[AsteroidType; KINDS],
        chance: &mut (impl Chance + ?Sized),
    ) {
        if self.first_free().is_none() {
            return;
        }
        let kind_type = type_of(types, kind);
        let mut speed = || {
            (f32::from(chance.roll(FRAGMENT_SPEED_SIDES)) - f32::from(FRAGMENT_SPEED_SIDES / 2))
                * 0.01
        };
        let velocity = Vec2::new(speed(), speed());
        let frame = f32::from(chance.roll(kind_type.frames));
        let spin = spun(kind_type, chance);
        self.place(Asteroid {
            position: at,
            velocity,
            frame,
            spin,
            strength: kind_type.strength,
            kind,
        });
    }

    /// Destroys the asteroid in `slot`, with the types `types`, drawing on
    /// `chance`: `_DestroyAsteroid` (@0xadc9-0xb028). What it leaves,
    /// none for an empty slot:
    ///
    /// - `trunc(YieldQty × (Rand(101) + 50) × 0.01)` boxes, 50-150 % of
    ///   its `YieldQty`, holding its `YieldType`, of graphic kind / 4 + 1
    ///   (@0xae52-0xae73);
    /// - with a `PartCount`, as many particles at 0.2 a tick ± 40 %,
    ///   living 240-480 ticks, of its `PartColor`, spread over a third of
    ///   its frame's width, whole, or of 32 with no sprite
    ///   (@0xaea7-0xaf3a);
    /// - its explosion (@0xaf7f).
    ///
    /// With a `FragCount`, `FragCount / 2 + Rand(FragCount)` fragments are
    /// spawned where it was ([`Asteroids::spawn_fragment`]), each of
    /// `FragType1` or `FragType2` by `Rand(2)` when both are set, the one
    /// set when one is, and none when neither is (@0xaf8f-0xaffc); its own
    /// slot is still held as they spawn. Then its slot is freed
    /// (@0xb008-0xb01c).
    pub fn destroy(
        &mut self,
        slot: usize,
        types: &[AsteroidType; KINDS],
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Remains> {
        let asteroid = (*self.slots.get(slot)?)?;
        let kind_type = type_of(types, asteroid.kind);
        let at = asteroid.position;
        let boxes = if kind_type.yield_qty > 0 {
            let share =
                u32::from(kind_type.yield_qty) * u32::from(chance.roll(YIELD_SIDES) + YIELD_BASE);
            // Whole, at most 32,767 × 1.5.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                (f64::from(share) * 0.01).trunc() as u32
            }
        } else {
            0
        };
        let particles = u32::try_from(kind_type.part_count)
            .ok()
            .filter(|&count| count > 0)
            .map(|count| {
                let width = if kind_type.size.0 > 0.0 {
                    kind_type.size.0
                } else {
                    NO_SPRITE_SIZE
                };
                ParticleBurst {
                    at,
                    speed: DEBRIS_SPEED,
                    variation: DEBRIS_VARIATION,
                    life: DEBRIS_LIFE,
                    color: kind_type.part_color,
                    count,
                    spread: (width / 3.0).trunc(),
                }
            });
        if let Ok(count) = u16::try_from(kind_type.frag_count)
            && count > 0
        {
            let fragments = count / 2 + chance.roll(count);
            for _ in 0..fragments {
                let kind = match kind_type.fragments {
                    [Some(first), Some(second)] => {
                        if chance.roll(2) == 0 {
                            first
                        } else {
                            second
                        }
                    }
                    [Some(kind), None] | [None, Some(kind)] => kind,
                    [None, None] => break,
                };
                self.spawn_fragment(at, kind, types, chance);
            }
        }
        self.slots[slot] = None;
        Some(Remains {
            at,
            boxes,
            contents: kind_type.yield_type,
            graphic: u16::from(asteroid.kind >> 2) + 1,
            particles,
            explosion: kind_type.explosion,
        })
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

/// The kind a `FragType` of `raw` names: `röid` 128-143, or a kind
/// 0-15 as it is; none for any other (@0x7de2e-0x7de4c).
fn fragment_kind(raw: i16) -> Option<u8> {
    let kind = if raw >= FIRST_ROID {
        raw - FIRST_ROID
    } else {
        raw
    };
    u8::try_from(kind)
        .ok()
        .filter(|&kind| usize::from(kind) < KINDS)
}

/// `kind`'s type in `types`: the missing type past the last.
fn type_of(types: &[AsteroidType; KINDS], kind: u8) -> AsteroidType {
    types
        .get(usize::from(kind))
        .copied()
        .unwrap_or(AsteroidType::MISSING)
}

/// A spawn's spin of `kind_type`'s, drawn on `chance`: its spin ×
/// `(Rand(41) + 80)` / 100, negated on `Rand(2)` 0 (@0x3a6c4-0x3a721,
/// @0x3fbc7-0x3fd21).
fn spun(kind_type: AsteroidType, chance: &mut (impl Chance + ?Sized)) -> f32 {
    let size = f32::from(chance.roll(SPIN_SIDES) + SPIN_BASE) * 0.01;
    let spin = kind_type.spin * size;
    if chance.roll(2) == 0 { -spin } else { spin }
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
            ..AsteroidType::MISSING
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
                ..AsteroidTypeRecord::default()
            };
            assert_eq!(
                AsteroidType::from_record(&record),
                AsteroidType {
                    strength: kept,
                    // 120 × 0.01, as the original multiplies.
                    spin: 120.0 * 0.01,
                    frames: 36,
                    size: (48.0, 40.0),
                    ..AsteroidType::MISSING
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
                ..AsteroidType::MISSING
            }
        );
    }

    /// A type read from a record of `YieldType` `yield_type` and
    /// `YieldQty` `yield_qty`.
    fn yielding(yield_type: i16, yield_qty: i16) -> AsteroidType {
        AsteroidType::from_record(&AsteroidTypeRecord {
            yield_type,
            yield_qty,
            ..AsteroidTypeRecord::default()
        })
    }

    #[test]
    fn a_type_keeps_a_yield_type_of_0_to_6_or_1000_to_1127_and_none_else() {
        for (raw, kept) in [
            (0, Some(0)),
            (6, Some(6)),
            (7, None),
            (999, None),
            (1000, Some(1000)),
            (1127, Some(1127)),
            (1128, None),
            (-1, None),
        ] {
            let made = yielding(raw, 3);
            assert_eq!((made.yield_type, made.yield_qty), (kept, 3), "{raw}");
        }
        assert_eq!(
            yielding(4, 0).yield_type,
            Some(4),
            "kept at a YieldQty of 0"
        );
    }

    #[test]
    fn a_negative_yield_qty_is_none_and_drops_the_yield_type() {
        let made = yielding(4, -3);
        assert_eq!((made.yield_type, made.yield_qty), (None, 0));
    }

    #[test]
    fn a_fragment_type_of_128_to_143_or_0_to_15_names_its_kind() {
        for (raw, kind) in [
            (128, Some(0)),
            (130, Some(2)),
            (143, Some(15)),
            (0, Some(0)),
            (5, Some(5)),
            (15, Some(15)),
            (16, None),
            (127, None),
            (144, None),
            (200, None),
            (-1, None),
        ] {
            let made = AsteroidType::from_record(&AsteroidTypeRecord {
                frag_types: [raw, -1],
                ..AsteroidTypeRecord::default()
            });
            assert_eq!(made.fragments, [kind, None], "{raw}");
            let made = AsteroidType::from_record(&AsteroidTypeRecord {
                frag_types: [-1, raw],
                ..AsteroidTypeRecord::default()
            });
            assert_eq!(made.fragments, [None, kind], "{raw}");
        }
    }

    #[test]
    fn a_type_keeps_its_fragment_count_particles_mass_and_explosion() {
        let made = AsteroidType::from_record(&AsteroidTypeRecord {
            frag_count: 4,
            part_count: 12,
            part_color: 0x00AB_CDEF,
            mass: 50,
            explode_type: 1003,
            ..AsteroidTypeRecord::default()
        });
        assert_eq!((made.frag_count, made.part_count), (4, 12));
        assert_eq!((made.part_color, made.mass), (0x00AB_CDEF, 50));
        assert_eq!(made.explosion, Explosion::decode(1003));
        assert!(made.explosion.is_some());
        let quiet = AsteroidType::from_record(&AsteroidTypeRecord::default());
        assert_eq!(
            quiet.explosion, None,
            "a record's default ExplodeType is -1"
        );
        assert_eq!(quiet.fragments, [None, None], "and its FragTypes");
    }

    #[test]
    fn the_missing_type_yields_breaks_into_and_explodes_into_nothing() {
        let missing = AsteroidType::MISSING;
        assert_eq!((missing.yield_qty, missing.yield_type), (0, None));
        assert_eq!(missing.fragments, [None, None]);
        assert_eq!((missing.frag_count, missing.part_count), (0, 0));
        assert_eq!((missing.mass, missing.explosion), (0, None));
    }

    #[test]
    fn a_types_radius_is_half_its_larger_side_or_16_without_a_sprite() {
        let sized = |size| AsteroidType {
            size,
            ..AsteroidType::MISSING
        };
        assert_eq!(sized((40.0, 30.0)).radius(), 20.0);
        assert_eq!(sized((30.0, 48.0)).radius(), 24.0);
        assert_eq!(sized((0.0, 0.0)).radius(), 16.0);
        assert_eq!(AsteroidType::MISSING.radius(), NO_SPRITE_RADIUS);
        assert_eq!(NO_SPRITE_RADIUS, 16.0);
    }

    #[test]
    fn the_table_holds_each_kind_read_and_the_missing_type_for_the_rest() {
        let record = |kind| AsteroidTypeRecord {
            kind,
            strength: 7,
            spin_rate: 100,
            frames: 4,
            frame_size: Some((10, 10)),
            ..AsteroidTypeRecord::default()
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

    // Destruction.

    /// [`types()`] with kind 4 yielding `YieldQty` 4 of `jünk` 1003,
    /// throwing off 12 orange particles, breaking into 4 fragments of
    /// kinds 1 and 2, exploding as `bööm` 131, and of `Mass` 30.
    fn breakable() -> [AsteroidType; KINDS] {
        let mut table = types();
        table[4] = AsteroidType {
            yield_qty: 4,
            yield_type: Some(1003),
            fragments: [Some(1), Some(2)],
            frag_count: 4,
            part_count: 12,
            part_color: 0x00FF_8000,
            mass: 30,
            explosion: Explosion::decode(3),
            ..table[4]
        };
        table
    }

    fn rock(kind: u8, at: Vec2) -> Asteroid {
        Asteroid {
            position: at,
            velocity: Vec2::new(1.5, -0.5),
            frame: 3.0,
            spin: 0.5,
            strength: -2,
            kind,
        }
    }

    /// A field holding `rock` in slot `slot`.
    fn holding(slot: usize, rock: Asteroid) -> Asteroids {
        let mut field = Asteroids::default();
        field.slots[slot] = Some(rock);
        field
    }

    #[test]
    fn placing_takes_the_first_free_slot_until_the_field_is_full() {
        let mut field = holding(0, rock(0, PLAYER));
        assert_eq!(field.place(rock(1, PLAYER)), Some(1));
        field.slots[0] = None;
        assert_eq!(field.place(rock(2, PLAYER)), Some(0));
        for _ in 2..SLOTS {
            assert!(field.place(rock(3, PLAYER)).is_some());
        }
        assert_eq!(field.place(rock(3, PLAYER)), None, "full");
        assert_eq!(field.active(), SLOTS);
        assert_eq!(field.slots[1].map(|r| r.kind), Some(1));
        assert_eq!(
            field.indexed().next().map(|(slot, r)| (slot, r.kind)),
            Some((0, 2))
        );
    }

    #[test]
    fn a_fragment_takes_the_first_free_slot_at_the_point_given_with_its_own_velocity() {
        let at = Vec2::new(40.0, -60.0);
        for (chance, velocity, frame, spin) in [
            (
                Box::new(NeverFires) as Box<dyn Chance>,
                Vec2::new(-1.0, -1.0),
                0.0,
                -0.8 * types()[2].spin,
            ),
            (
                Box::new(Last),
                Vec2::new(0.99, 0.99),
                35.0,
                1.2 * types()[2].spin,
            ),
        ] {
            let mut chance = chance;
            let mut field = holding(0, rock(4, PLAYER));
            field.spawn_fragment(at, 2, &types(), &mut *chance);
            let made = field.slots[1].expect("slot 1");
            assert_eq!((made.position, made.kind), (at, 2));
            assert!((made.velocity - velocity).length() < 1e-6, "{made:?}");
            assert_eq!(made.frame, frame);
            assert!((made.spin - spin).abs() < 1e-6, "{made:?}");
            assert_eq!(made.strength, types()[2].strength);
        }
    }

    #[test]
    fn a_fragment_is_dropped_when_the_field_is_full() {
        let mut field = Asteroids::default();
        for _ in 0..SLOTS {
            field.place(rock(0, PLAYER));
        }
        field.spawn_fragment(PLAYER, 2, &types(), &mut NeverFires);
        assert!(field.iter().all(|made| made.kind == 0));
    }

    #[test]
    fn destroying_an_empty_slot_leaves_nothing() {
        let mut field = Asteroids::default();
        assert_eq!(field.destroy(3, &breakable(), &mut NeverFires), None);
    }

    #[test]
    fn a_destroyed_asteroid_drops_half_to_one_and_a_half_times_its_yield_qty_in_boxes() {
        for (mut chance, boxes) in [
            (Box::new(NeverFires) as Box<dyn Chance>, 2),
            (Box::new(Last), 6),
        ] {
            let mut field = holding(5, rock(4, PLAYER));
            let remains = field
                .destroy(5, &breakable(), &mut *chance)
                .expect("remains");
            assert_eq!(remains.boxes, boxes);
            assert_eq!(remains.contents, Some(1003));
            assert_eq!(remains.at, PLAYER);
        }
        let mut table = breakable();
        table[4].yield_qty = 0;
        let mut field = holding(5, rock(4, PLAYER));
        let remains = field.destroy(5, &table, &mut Last).expect("remains");
        assert_eq!(remains.boxes, 0);
    }

    #[test]
    fn a_destroyed_asteroids_boxes_are_graphic_its_kind_over_4_plus_1() {
        for (kind, graphic) in [(0, 1), (3, 1), (4, 2), (7, 2), (8, 3), (15, 4)] {
            let mut field = holding(0, rock(kind, PLAYER));
            let remains = field
                .destroy(0, &types(), &mut NeverFires)
                .expect("remains");
            assert_eq!(remains.graphic, graphic, "{kind}");
        }
    }

    #[test]
    fn a_destroyed_asteroid_throws_off_its_particles_a_third_of_its_width_out() {
        let mut field = holding(0, rock(4, PLAYER));
        let remains = field
            .destroy(0, &breakable(), &mut NeverFires)
            .expect("remains");
        assert_eq!(
            remains.particles,
            Some(ParticleBurst {
                at: PLAYER,
                speed: 0.2,
                variation: 40,
                life: (240, 480),
                color: 0x00FF_8000,
                count: 12,
                // A third of 40, whole.
                spread: 13.0,
            })
        );
        let mut table = breakable();
        table[4].size = (0.0, 0.0);
        let mut field = holding(0, rock(4, PLAYER));
        let remains = field.destroy(0, &table, &mut NeverFires).expect("remains");
        assert_eq!(remains.particles.map(|p| p.spread), Some(10.0), "32 / 3");
        table[4].part_count = 0;
        let mut field = holding(0, rock(4, PLAYER));
        let remains = field.destroy(0, &table, &mut NeverFires).expect("remains");
        assert_eq!(remains.particles, None);
    }

    #[test]
    fn a_destroyed_asteroid_explodes_as_its_type_says() {
        let mut field = holding(0, rock(4, PLAYER));
        let remains = field
            .destroy(0, &breakable(), &mut NeverFires)
            .expect("remains");
        assert_eq!(remains.explosion, Explosion::decode(3));
        let mut field = holding(0, rock(3, PLAYER));
        let remains = field
            .destroy(0, &breakable(), &mut NeverFires)
            .expect("remains");
        assert_eq!(remains.explosion, None);
    }

    #[test]
    fn a_destroyed_asteroid_breaks_into_half_its_frag_count_plus_a_draw_of_it() {
        for (mut chance, count, kind) in [
            (Box::new(NeverFires) as Box<dyn Chance>, 2, 1),
            (Box::new(Last), 5, 2),
        ] {
            let mut field = holding(0, rock(4, PLAYER));
            field.destroy(0, &breakable(), &mut *chance);
            assert_eq!(field.active(), count);
            assert!(field.slots[0].is_none(), "the parent's slot freed");
            assert!(field.iter().all(|made| made.kind == kind));
            assert!(field.iter().all(|made| made.position == PLAYER));
        }
    }

    #[test]
    fn a_fragment_is_the_one_type_set_and_none_without_either() {
        for (fragments, kinds) in [
            ([None, Some(7)], vec![7, 7]),
            ([Some(9), None], vec![9, 9]),
            ([None, None], vec![]),
        ] {
            let mut table = breakable();
            table[4].fragments = fragments;
            let mut field = holding(0, rock(4, PLAYER));
            field.destroy(0, &table, &mut NeverFires);
            let made: Vec<u8> = field.iter().map(|made| made.kind).collect();
            assert_eq!(made, kinds, "{fragments:?}");
        }
        let mut table = breakable();
        table[4].frag_count = 0;
        let mut field = holding(0, rock(4, PLAYER));
        field.destroy(0, &table, &mut Last);
        assert_eq!(field.active(), 0, "no FragCount, no fragments");
    }

    #[test]
    fn a_full_field_takes_no_fragments_and_frees_the_parents_slot() {
        let mut field = holding(0, rock(4, PLAYER));
        for _ in 1..SLOTS {
            field.place(rock(0, PLAYER));
        }
        field.destroy(0, &breakable(), &mut Last);
        assert_eq!(field.active(), SLOTS - 1);
        assert_eq!(field.first_free(), Some(0));
        assert!(field.iter().all(|made| made.kind == 0));
    }
}
