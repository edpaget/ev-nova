//! A shot's smoke trail: the puffs `_HandleShot` leaves at a smoking
//! shot's centre every live tick (@0x35cc5-0x35d37), as `_SpawnSmoke`
//! (@0x3f927-0x3fa04) and `_HandleSmoke` (@0x2e29f-0x2e5a6) keep and
//! draw them.
//!
//! - A shot smokes ([`smoke_kind`]) under `Flags` 0x0200 (kind 1) or
//!   else 0x0400 (kind 0), the kind 2 more under 0x0800 (@0x35cd9-0x35d0c).
//! - A puff needs its weapon's smoke set's sprite: only sets 0 and 1 have
//!   one, from `cicn` 1000 + 8 × set (`_LoadSprites` @0x21855-0x21907,
//!   [`SmokeSheet`]), and none without its `cicn`s, so a puff of any other
//!   set is dropped. Under the original's caps
//!   ([`RuleKey::CombatCaps`](nova_sim::RuleKey::CombatCaps)'s engine
//!   reading) it takes the first of [`MAX_PUFFS`] slots free, and a full
//!   pool drops it; without them any number is kept.
//! - Each frame a puff is shown at full `AddOver`, [`Blend::Or`](crate::draw::Blend::Or)
//!   at [`lights_tint`]`(32)` (+0xa8 and its channels 0x20,
//!   @0x2e377-0x2e3b3), murked by where it is ([`sprite_murk`],
//!   @0x2e367), and then ages by [`PUFF_AGE_STEP`]; its frame is then set
//!   from its age ([`smoke_frame`], @0x2e3f9-0x2e499), and a puff past its
//!   last frame is freed. It never moves.
//! - A system change frees every puff (@0x2e2d3, @0x2e56e-0x2e59a), as
//!   the effects are cleared.
//!
//! Stock has no `cicn` 1000-1015 and no weapon of `Flags` 0x0200 or
//! 0x0400, so stock never smokes.
//!
//! Left out: the translucency `_HandleSmoke` works out and overwrites at
//! once (@0x2e308-0x2e33f), which is never seen; the `SmokeTrails`
//! preference gate (@0x35cc5-0x35ccd, @0x3f93d-0x3f951), which the engine
//! has no preference for yet
//! (rdm:phase/weapons-fidelity/phase-35-left-out-of-phase-24); the frozen
//! frame (@0x35ccf, @0x2e3bc), which the engine never has; and the puffs'
//! sprite layer, which rdm:task/shots-drawn-in-their-sprite-layers owns:
//! the view draws them with the other effects.

use nova_sim::Vec2;

use super::catalog::SmokeSheet;
use super::murk::{murk_tint, sprite_murk};
use crate::draw::lights_tint;
use crate::system::camera::Camera;
use crate::{DrawList, ImageKey, Point};

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
    puffs: Vec<Puff>,
}

impl SmokePuffs {
    /// Puts a puff of `kind` at `at`, of the set whose sprite is `sheet`,
    /// as `_SpawnSmoke` does: dropped without a sprite, or under the
    /// `caps` when [`MAX_PUFFS`] are kept.
    pub fn spawn(&mut self, at: Vec2, kind: u8, sheet: Option<SmokeSheet>, caps: bool) {
        let Some(sheet) = sheet else {
            return;
        };
        if caps && self.puffs.len() >= MAX_PUFFS {
            return;
        }
        self.puffs.push(Puff {
            at,
            kind,
            sheet,
            age: 0.0,
        });
    }

    /// Ages every puff a tick, freeing each past its last frame, as
    /// `_HandleSmoke` does after it shows them.
    pub fn step(&mut self) {
        self.puffs.retain_mut(|puff| {
            puff.age += PUFF_AGE_STEP;
            smoke_frame(puff.kind, puff.age).is_some()
        });
    }

    /// Draws each puff where `camera` shows it, at its frame, added at
    /// full and murked as seen from the player's ship at `viewer` in a
    /// system of effective murk `murk`.
    pub fn draw(&self, list: &mut DrawList, camera: &Camera, viewer: Vec2, murk: u8) {
        for puff in &self.puffs {
            let image = ImageKey::icon(puff.sheet.icon(puff.frame()));
            let at = camera.world_to_screen(Point::new(puff.at.x, puff.at.y));
            let level = sprite_murk(puff.at, viewer, murk);
            list.or_sprite(image, at, murk_tint(lights_tint(32), level));
        }
    }

    /// The puffs kept.
    #[must_use]
    pub fn puffs(&self) -> &[Puff] {
        &self.puffs
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
    use crate::DrawCommand;

    const SET_0: SmokeSheet = SmokeSheet { first_icon: 1000 };

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
            let Some(puff) = puffs.puffs().first() else {
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
    fn a_set_without_a_sprite_drops_its_puff() {
        let mut puffs = SmokePuffs::default();
        puffs.spawn(Vec2::ZERO, 0, None, false);
        assert_eq!(puffs.puffs(), []);
    }

    #[test]
    fn a_puff_is_added_at_full_murked_at_its_frames_cicn_where_it_is() {
        let mut puffs = SmokePuffs::default();
        let set_1 = SmokeSheet { first_icon: 1008 };
        puffs.spawn(Vec2::new(0.0, 160.0), 0, Some(set_1), false);
        for _ in 0..6 {
            puffs.step();
        }
        let camera = Camera::centred_on(Point::new(0.0, 0.0));
        let mut list = DrawList::new();
        puffs.draw(&mut list, &camera, Vec2::ZERO, 100);
        let level = sprite_murk(Vec2::new(0.0, 160.0), Vec2::ZERO, 100);
        assert_eq!(level, 30);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Sprite {
                image: ImageKey::icon(1009),
                center: camera.world_to_screen(Point::new(0.0, 160.0)),
                tint: murk_tint(lights_tint(32), level),
                blend: crate::Blend::Or,
            }],
            "age 1.5: frame 1"
        );
        let mut clear = DrawList::new();
        puffs.draw(&mut clear, &camera, Vec2::new(0.0, 160.0), 100);
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
