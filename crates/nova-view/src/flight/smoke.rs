//! A shot's smoke trail: the puffs `_HandleShot` leaves at a smoking
//! shot's centre on each smoke frame (@0x35cc5-0x35d37), as `_SpawnSmoke`
//! (@0x3f927-0x3fa04) and `_HandleSmoke` (@0x2e29f-0x2e5a6) keep and
//! draw them.
//!
//! - A smoke frame is a pass whose frame counter is a multiple of 3: the
//!   byte at `*0x3b555c` `_HandleShot` tests @0x35cc5-0x35ccd, which
//!   `_PlayGame` sets before each `_DoPlayGameWork` (@0x45b7d-0x45bbe;
//!   `nova_sim::Session::smoke_frame`). A live shot of a smoking weapon
//!   leaves no puff on the other frames.
//! - The Smoke Trails preference off (the original's inverted
//!   `SmokeTrails` byte set) makes `_SpawnSmoke` return at once
//!   (@0x3f93d-0x3f951), so no shot leaves a puff; the puffs already left
//!   go on ageing, as `_HandleSmoke` does not read it.
//!
//! - A shot smokes ([`smoke_kind`]) under `Flags` 0x0200 (kind 1) or
//!   else 0x0400 (kind 0), the kind 2 more under 0x0800 (@0x35cd9-0x35d0c).
//! - A puff needs its weapon's smoke set's sprite: only sets 0 and 1 have
//!   one, from `cicn` 1000 + 8 × set (`_LoadSprites` @0x21855-0x21907,
//!   [`SmokeSheet`]), and none without its `cicn`s, so a puff of any other
//!   set is dropped. Under the original's caps
//!   ([`RuleKey::CombatCaps`](nova_sim::RuleKey::CombatCaps)'s engine
//!   reading) it takes the first of [`MAX_PUFFS`] slots free, and a full
//!   pool drops it; without them any number is kept, each still taking
//!   the first free slot. `_smokeLayer` draws the puffs in slot order
//!   (`_InitSprites` adds them once, @0x1fe8f, and nothing moves them),
//!   so a new puff in a freed lower slot draws under an older one in a
//!   higher slot ([`SmokePuffs::puffs`], [`SmokePuffs::draw`]); under
//!   [`Blend::Or`](crate::draw::Blend::Or) the order rarely shows.
//! - Each frame a puff is shown at full `AddOver`, [`Blend::Or`](crate::draw::Blend::Or)
//!   at [`lights_tint`]`(32)` (+0xa8 and its channels 0x20,
//!   @0x2e377-0x2e3b3), murked by where it is ([`SeenMurk::at`],
//!   @0x2e367), and then ages by [`PUFF_AGE_STEP`]; its frame is then set
//!   from its age ([`smoke_frame`], @0x2e3f9-0x2e499), and a puff past its
//!   last frame is freed. It never moves.
//! - It is placed at whole pixels from the player by its current frame's
//!   own size ([`SmokeSheet::size`], [`Camera::sprite_centre`],
//!   @0x2e4ac-0x2e52d), a master of `cicn`s having a size a frame.
//! - A system change frees every puff (@0x2e2d3, @0x2e56e-0x2e59a), as
//!   the effects are cleared.
//!
//! Stock has no `cicn` 1000-1015 and no weapon of `Flags` 0x0200 or
//! 0x0400, so stock never smokes.
//!
//! The puffs are drawn in their sprite layer, `_smokeLayer` 0x3b52d0
//! (`_SpawnSmoke` @0x3f957-0x3f9d4 takes a `_smokeData` slot): over the
//! cargo boxes and the escape pods, under every shot and flying ship
//! ([`SpriteLayer::Smoke`](nova_sim::sprite_layer::SpriteLayer::Smoke)).
//!
//! Left out: the translucency `_HandleSmoke` works out and overwrites at
//! once (@0x2e308-0x2e33f), which is never seen; and the frozen frame
//! (@0x35ccf, @0x2e3bc), which the engine never has.

use nova_sim::Vec2;
use nova_sim::player_wrap::PlayerWrap;

use super::catalog::SmokeSheet;
use super::murk::SeenMurk;
use super::slot_pool::SlotPool;
use crate::draw::lights_tint;
use crate::system::camera::Camera;
use crate::{Blend, DrawList, ImageKey, Point};

/// `Flags`: the shot leaves a big smoke trail, kind 1.
pub const BIG_SMOKE: u16 = 0x0200;
/// `Flags`: the shot leaves a small smoke trail, kind 0.
pub const SMALL_SMOKE: u16 = 0x0400;
/// `Flags`: the smoke lingers: its kind 2 more, at half speed.
pub const LINGERING_SMOKE: u16 = 0x0800;

/// The most puffs kept under the caps (64 slots of 0x18 at 0x3b535c).
pub const MAX_PUFFS: usize = 64;

/// What a puff ages a tick (the double 0.25 @0xdd130).
pub const PUFF_AGE_STEP: f32 = 0.25;

/// The kind of smoke a shot of `Flags` `flags` leaves, if any: 1 under
/// [`BIG_SMOKE`], else 0 under [`SMALL_SMOKE`], 2 more under
/// [`LINGERING_SMOKE`] (@0x35cd9-0x35d0c).
#[must_use]
pub fn smoke_kind(flags: u16) -> Option<u8> {
    let kind = if flags & BIG_SMOKE != 0 {
        1
    } else if flags & SMALL_SMOKE != 0 {
        0
    } else {
        return None;
    };
    Some(if flags & LINGERING_SMOKE == 0 {
        kind
    } else {
        kind + 2
    })
}

/// The frame of a puff of `kind` at `age`, or none once it is past its
/// last (@0x2e3f9-0x2e499): the index is its age's whole part, halved
/// for kinds 2 and 3; kinds 1 and 3 show 0, 1, 2, then 2, 1, 0 and end;
/// kinds 0 and 2 show 0 to 7 and end.
#[must_use]
pub fn smoke_frame(kind: u8, age: f32) -> Option<u8> {
    if age < 0.0 {
        return None;
    }
    // A puff ages a quarter a tick: its index is small.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = age.trunc() as u32;
    let index = if kind >= 2 { whole / 2 } else { whole };
    let frame = if kind % 2 == 1 {
        match index {
            0..=2 => index,
            3..=5 => 5 - index,
            _ => return None,
        }
    } else if index <= 7 {
        index
    } else {
        return None;
    };
    u8::try_from(frame).ok()
}

/// A puff of smoke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    /// Where it is, in world units.
    pub at: Vec2,
    /// Its kind ([`smoke_kind`]).
    pub kind: u8,
    /// Its set's sprite.
    pub sheet: SmokeSheet,
    /// How old it is, a quarter a tick.
    pub age: f32,
}

impl Puff {
    /// The frame it shows ([`smoke_frame`]).
    #[must_use]
    pub fn frame(&self) -> u8 {
        smoke_frame(self.kind, self.age).unwrap_or(0)
    }
}

/// The puffs of smoke.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmokePuffs {
    puffs: SlotPool<Puff>,
}

impl SmokePuffs {
    /// Puts a puff of `kind` at `at`, of the set whose sprite is `sheet`,
    /// as `_SpawnSmoke` does: dropped without a sprite, or under the
    /// `caps` when [`MAX_PUFFS`] are kept.
    pub fn spawn(&mut self, at: Vec2, kind: u8, sheet: Option<SmokeSheet>, caps: bool) {
        let Some(sheet) = sheet else {
            return;
        };
        let puff = Puff {
            at,
            kind,
            sheet,
            age: 0.0,
        };
        self.puffs.insert(puff, caps.then_some(MAX_PUFFS));
    }

    /// Carries each puff within `wrap`'s reach: `_WrapPlayerPosition`'s
    /// loop of `_smokeData` (@0x681a7-0x68229).
    pub fn carry(&mut self, wrap: &PlayerWrap) {
        for puff in self.puffs.iter_mut() {
            wrap.carry(&mut puff.at);
        }
    }

    /// Ages every puff a tick, freeing each past its last frame, as
    /// `_HandleSmoke` does after it shows them.
    pub fn step(&mut self) {
        self.puffs.retain_mut(|puff| {
            puff.age += PUFF_AGE_STEP;
            smoke_frame(puff.kind, puff.age).is_some()
        });
    }

    /// Draws each puff where `camera` places it, at whole pixels by its
    /// current frame's own size ([`Camera::sprite_centre`], `_HandleSmoke`
    /// @0x2e4ac-0x2e52d), at its frame, added at full and murked as the
    /// player sees it, `seen`.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, seen: SeenMurk) {
        for puff in self.puffs.iter() {
            let frame = puff.frame();
            let image = ImageKey::icon(puff.sheet.icon(frame));
            let at = Point::new(puff.at.x, puff.at.y);
            let at = camera.sprite_centre(at, puff.sheet.size(frame));
            list.mixed_sprite(
                image,
                at,
                lights_tint(32),
                Blend::Or,
                Some(seen.at(puff.at)),
            );
        }
    }

    /// The puffs kept, in slot order (see the module docs).
    #[must_use]
    pub fn puffs(&self) -> Vec<Puff> {
        self.puffs.to_vec()
    }

    /// Frees every puff.
    pub fn clear(&mut self) {
        self.puffs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Color;
    use crate::Colorize;
    use crate::DrawCommand;
    use crate::system::camera::Anchor;

    const SET_0: SmokeSheet = SmokeSheet {
        first_icon: 1000,
        sizes: [(0, 0); 8],
    };

    #[test]
    fn a_wrap_carries_the_puffs_within_its_reach() {
        use nova_sim::player_wrap::{WRAP_EDGE, WRAP_SHIFT};
        let player = Vec2::new(WRAP_EDGE + 10.0, 0.0);
        let wrap = PlayerWrap::of(player).expect("past the edge");
        let mut puffs = SmokePuffs::default();
        let (near, far) = (
            player - Vec2::new(100.0, 0.0),
            player - Vec2::new(WRAP_EDGE * 2.0, 0.0),
        );
        puffs.spawn(near, 0, Some(SET_0), false);
        puffs.spawn(far, 0, Some(SET_0), false);
        puffs.carry(&wrap);
        let at: Vec<Vec2> = puffs.puffs().iter().map(|puff| puff.at).collect();
        assert_eq!(at, [near - Vec2::new(WRAP_SHIFT, 0.0), far]);
    }

    #[test]
    fn a_shots_smoke_is_its_kind_by_its_flags() {
        assert_eq!(smoke_kind(0), None);
        assert_eq!(smoke_kind(SMALL_SMOKE), Some(0));
        assert_eq!(smoke_kind(BIG_SMOKE), Some(1));
        assert_eq!(smoke_kind(BIG_SMOKE | SMALL_SMOKE), Some(1), "big first");
        assert_eq!(smoke_kind(SMALL_SMOKE | LINGERING_SMOKE), Some(2));
        assert_eq!(smoke_kind(BIG_SMOKE | LINGERING_SMOKE), Some(3));
        assert_eq!(smoke_kind(LINGERING_SMOKE), None, "lingering alone");
    }

    /// The frames a puff of `kind` shows, a tick at a time, from its
    /// spawn's step until it is freed.
    fn shown(kind: u8) -> Vec<u8> {
        let mut puffs = SmokePuffs::default();
        puffs.spawn(Vec2::ZERO, kind, Some(SET_0), false);
        let mut frames = Vec::new();
        loop {
            puffs.step();
            let Some(puff) = puffs.puffs().first().copied() else {
                return frames;
            };
            frames.push(puff.frame());
        }
    }

    /// `frame` held for `ticks` ticks.
    fn held(frames: &[u8], ticks: usize) -> Vec<u8> {
        frames
            .iter()
            .flat_map(|&frame| std::iter::repeat_n(frame, ticks))
            .collect()
    }

    #[test]
    fn each_kind_shows_its_frames_and_ends() {
        // Shown from age 0.25 (the spawn's own step) to its last frame:
        // ages 8, 6, 16 and 12 end kinds 0 to 3, 32, 24, 64 and 48 steps
        // after the spawn.
        let mut small = held(&[0, 1, 2, 3, 4, 5, 6, 7], 4);
        small.remove(0);
        assert_eq!(shown(0), small, "0 to 7");
        let mut big = held(&[0, 1, 2, 2, 1, 0], 4);
        big.remove(0);
        assert_eq!(shown(1), big, "0, 1, 2, then back");
        let mut lingering = held(&[0, 1, 2, 3, 4, 5, 6, 7], 8);
        lingering.remove(0);
        assert_eq!(shown(2), lingering, "at half speed");
        let mut big_lingering = held(&[0, 1, 2, 2, 1, 0], 8);
        big_lingering.remove(0);
        assert_eq!(shown(3), big_lingering);
        assert_eq!(
            [0, 1, 2, 3].map(|kind| shown(kind).len() + 1),
            [32, 24, 64, 48],
            "steps from its spawn to its freeing"
        );
    }

    #[test]
    fn a_frame_is_none_before_its_age_starts_or_past_its_last() {
        assert_eq!(smoke_frame(0, -1.0), None);
        assert_eq!(smoke_frame(0, 0.0), Some(0), "a fresh puff");
        assert_eq!(smoke_frame(0, 7.99), Some(7));
        assert_eq!(smoke_frame(0, 8.0), None);
        assert_eq!(smoke_frame(1, 5.99), Some(0));
        assert_eq!(smoke_frame(1, 6.0), None);
        assert_eq!(smoke_frame(3, 3.5), Some(1));
    }

    #[test]
    fn a_full_pool_drops_a_puff_under_the_caps_and_not_without() {
        let mut capped = SmokePuffs::default();
        let mut uncapped = SmokePuffs::default();
        for _ in 0..70 {
            capped.spawn(Vec2::ZERO, 0, Some(SET_0), true);
            uncapped.spawn(Vec2::ZERO, 0, Some(SET_0), false);
        }
        assert_eq!(capped.puffs().len(), MAX_PUFFS);
        assert_eq!(uncapped.puffs().len(), 70);
        for _ in 0..32 {
            capped.step();
        }
        capped.spawn(Vec2::ZERO, 0, Some(SET_0), true);
        assert_eq!(capped.puffs().len(), 1, "the freed slots taken again");
    }

    #[test]
    fn a_new_puff_takes_a_freed_lower_slot_and_draws_under_an_older_one() {
        let mut puffs = SmokePuffs::default();
        puffs.spawn(Vec2::new(10.0, 0.0), 1, Some(SET_0), false);
        puffs.spawn(Vec2::new(20.0, 0.0), 0, Some(SET_0), false);
        for _ in 0..24 {
            puffs.step();
        }
        let xs =
            |puffs: &SmokePuffs| -> Vec<f32> { puffs.puffs().iter().map(|p| p.at.x).collect() };
        assert_eq!(xs(&puffs), [20.0], "the kind-1 puff ended at age 6");
        puffs.spawn(Vec2::new(30.0, 0.0), 0, Some(SET_0), false);
        assert_eq!(xs(&puffs), [30.0, 20.0], "in slot order");
        let camera = Camera::centred_on(Point::default());
        let mut list = DrawList::new();
        let clear = SeenMurk {
            murk: 0,
            ..seen_from(Vec2::ZERO)
        };
        puffs.draw(&mut list, &camera, clear);
        let drawn: Vec<f32> = list
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { center, .. } => Some(center.x),
                _ => None,
            })
            .collect();
        let screen = |x| camera.whole_point(Point::new(x, 0.0)).x;
        assert_eq!(drawn, [screen(30.0), screen(20.0)], "the new one first");
    }

    #[test]
    fn a_puff_is_placed_at_whole_pixels_by_its_current_frames_size() {
        let mut sizes = [(0, 0); 8];
        sizes[0] = (7, 9);
        sizes[1] = (8, 6);
        let sheet = SmokeSheet { sizes, ..SET_0 };
        let mut puffs = SmokePuffs::default();
        puffs.spawn(Vec2::new(10.75, -20.25), 0, Some(sheet), false);
        let camera = Camera::centred_on(Point::new(0.25, -0.5)).with_anchor(Anchor::PlayAreaCentre);
        let clear = SeenMurk {
            murk: 0,
            ..seen_from(Vec2::ZERO)
        };
        let centre = |puffs: &SmokePuffs| {
            let mut list = DrawList::new();
            puffs.draw(&mut list, &camera, clear);
            list.iter()
                .find_map(|command| match *command {
                    DrawCommand::Sprite { center, .. } => Some(center),
                    _ => None,
                })
                .expect("drawn")
        };
        // (10.5, -19.75) truncates to (10, -19), so (425, 365) less (3, 4)
        // is the top left (422, 361).
        assert_eq!(centre(&puffs), Point::new(425.5, 365.5));
        for _ in 0..4 {
            puffs.step();
        }
        assert_eq!(puffs.puffs()[0].frame(), 1);
        // (425, 365) less (4, 3).
        assert_eq!(centre(&puffs), Point::new(425.0, 365.0));
    }

    #[test]
    fn a_set_without_a_sprite_drops_its_puff() {
        let mut puffs = SmokePuffs::default();
        puffs.spawn(Vec2::ZERO, 0, None, false);
        assert_eq!(puffs.puffs(), []);
    }

    /// The murk at its worst as seen from `viewer`.
    fn seen_from(viewer: Vec2) -> SeenMurk {
        SeenMurk {
            viewer,
            murk: 100,
            colour: 0x1081,
        }
    }

    #[test]
    fn a_puff_is_added_at_full_murked_at_its_frames_cicn_where_it_is() {
        let mut puffs = SmokePuffs::default();
        let set_1 = SmokeSheet {
            first_icon: 1008,
            ..SET_0
        };
        puffs.spawn(Vec2::new(0.0, 160.0), 0, Some(set_1), false);
        for _ in 0..6 {
            puffs.step();
        }
        let camera = Camera::centred_on(Point::new(0.0, 0.0));
        let mut list = DrawList::new();
        puffs.draw(&mut list, &camera, seen_from(Vec2::ZERO));
        let level = seen_from(Vec2::ZERO).at(Vec2::new(0.0, 160.0)).level;
        assert_eq!(level, 30);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Colorized {
                image: ImageKey::icon(1009),
                center: camera.whole_point(Point::new(0.0, 160.0)),
                tint: lights_tint(32),
                blend: crate::Blend::Or,
                colorize: Colorize::toward_pixel(level, 0x1081),
            }],
            "age 1.5: frame 1"
        );
        let mut clear = DrawList::new();
        puffs.draw(&mut clear, &camera, seen_from(Vec2::new(0.0, 160.0)));
        let tints: Vec<Color> = clear
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Sprite { tint, .. } => Some(tint),
                _ => None,
            })
            .collect();
        assert_eq!(tints, [lights_tint(32)], "no murk beside the player");
        puffs.clear();
        assert_eq!(puffs.puffs(), []);
    }
}
