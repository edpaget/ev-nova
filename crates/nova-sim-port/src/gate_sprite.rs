//! A hypergate's sprite: which frame of its sheet it shows, opening while a
//! ship holds it open and closing once none does.
//!
//! Evidence: disassembly of the Mac OS X `EV Nova` (i386, with symbols),
//! `_HandleStellarSprites` (@0x2e713-0x2ef2c), its hypergate branch
//! (`Flags2` 0x1000, @0x2e832-0x2ec46).
//!
//! - **When it animates** (@0x2e784-0x2e80a): when its destroyed state is
//!   the one `Flags2` 0x0080 names and its frame count is above 1. It
//!   needs no `AnimDelay` above 0. Otherwise it shows frame 0 and its
//!   state stays frozen (@0x2ee20). While it animates, its timer gains a
//!   tick a frame (`_gSpeedMult`, @0x2e810-0x2e825).
//! - **The open frame** `o` ([`GateAnimation::open_frame`],
//!   @0x2e83c-0x2e864) is its `CustPicID`, or frames / 2 when that is 0
//!   or less or at least frames - 1. Frames 0 to `o` - 1 are the opening,
//!   `o` to frames - 1 the open loop.
//! - **Held open** ([`held_open`], @0x2e867-0x2e9b0): first by the
//!   player's dock stellar, its request's counter above 0
//!   (@0x2e867-0x2e87e; see [`DockRequest`]), `None` standing for
//!   `_dockTarget` -1, which matches no gate; then by an NPC coming out
//!   of it ([`Npc::holds_gate_open`]): waiting in it (goal 0x15), not
//!   bursting (goal 8, +0x6c = -1, @0x8dd8a-0x8dd93); or by one heading in to enter it
//!   (@0x2e8dd-0x2e9a2, [`Npc::approaches_gate`]): an NPC, active and not
//!   `_IsDisabled` (@0x2e884-0x2e8aa), whose goal is 1 or 0x14 with its
//!   travel stellar this one (`_AIShipIsShipGoingToStellar` @0x895d6),
//!   strictly inside a box of half-width 2 × `_GetSpriteXSize` of the
//!   gate's slot sprite on each axis (its drawn frame's width, or 32
//!   with no frame, @0x26d4-0x2701), grown to trunc(× 1.1) once the gate
//!   is open, its frame as it stood before this tick's step
//!   ([`approach_reach`], @0x2e906-0x2e92e). It has no mode test.
//! - **While held** (@0x2e9f8-0x2ebf1) it steps once its timer reaches
//!   `AnimDelay`, or `Frame0Bias` × `AnimDelay` on frame `o` when the bias
//!   is above 1, the timer then starting again. Below `o` it opens a frame.
//!   From `o` it plays its loop: with `Flags2` 0x0001 it shows `o` between
//!   every other frame (@0x2ea77); with 0x0002 it picks each next frame at
//!   random among `o` to frames - 1, drawn again while it is the frame
//!   shown (or `o`, with 0x0001; @0x2eabb-0x2eafb, @0x2eb6d-0x2ebb4), and
//!   otherwise takes them in turn, wrapping to `o` (to `o` + 1 with
//!   0x0001; @0x2eb02-0x2eb39, @0x2ebb6-0x2ebea).
//! - **Released** (@0x2e9b6-0x2e9df, @0x2ebf6-0x2ec46) it steps once its
//!   timer reaches `AnimDelay`, with no bias. Below `o` it closes a frame,
//!   resting on 0. In its loop without 0x0002 it plays the loop out to its
//!   last frame first; then, or at once with 0x0002, it goes to `o` - 1 and
//!   closes from there.
//!
//! With `AnimDelay` 0 or less the threshold is met every tick, so a stock
//! hypergate (42 frames, `AnimDelay` 0) opens over 21 ticks.
//!
//! The engine differs where the original would hang: a random pick with
//! no frame left to pick (frames - `o` is 1, say) spins forever in the
//! original; the engine keeps the next frame it had. `Frame0Bias` ×
//! `AnimDelay` is reckoned wide, with no 16-bit wrap. The original takes
//! ships 1-63 alone (@0x2e884, step 0xc940 to 0x325000); the engine takes
//! every NPC, having no cap on the ship table.
//!
//! The function plays no sound: it calls `_Rand`, `_IsDisabled`, two
//! `_AIShip*` predicates, `_FloatAbs` and sprite calls, and no `PlaySound`
//! or `_beepSnd`.
//!
//! Left out of the hypergate branch, with the reason:
//!
//! - **The sequential branch** (@0x2ec4b-0x2ee05), every other stellar's:
//!   the view's stellar animation.
//! - **Sprite-slot plumbing** (@0x2e713-0x2e7e5, @0x2ee35-0x2ef1a):
//!   visibility, master loading, cloning and location, which the view's
//!   stellar drawing does; `_HandleSpriteMurkiness` (@0x2eea4) is left
//!   out, as the view murks shots alone so far
//!   (rdm:task/murk-other-sprites).

use crate::catalog::{LandingSite, SiteAnimation};
use crate::chance::Chance;
use crate::clearance::DockRequest;
use crate::traffic::npc::Npc;

/// `Flags2` 0x0001: show the open frame between every other frame.
pub const ALTERNATE: u16 = 0x0001;
/// `Flags2` 0x0002: pick the loop's frames at random.
pub const RANDOM: u16 = 0x0002;
/// `Flags2` 0x0080: animate only while destroyed.
pub const ONLY_WHEN_DESTROYED: u16 = 0x0080;

/// A hypergate's animation fields, raw from its `spöb`, and the frame
/// count of the sheet it counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateAnimation {
    /// Its `AnimDelay`, raw.
    pub delay: i16,
    /// Its `Frame0Bias`, raw.
    pub frame0_bias: i16,
    /// Its `CustPicID`, raw: the open frame, when it is one.
    pub open_pic: i16,
    /// Its `Flags2`, raw: [`ALTERNATE`], [`RANDOM`] and
    /// [`ONLY_WHEN_DESTROYED`] are read.
    pub flags2: u16,
    /// The frame count of the sheet it counts.
    pub frames: u16,
}

impl GateAnimation {
    /// `site`'s, counting its `DeadType` sheet's frames while it is
    /// destroyed with one that reads, and its `Type` sheet's otherwise, as
    /// the stellar's sprite slot holds one or the other.
    #[must_use]
    pub fn of(site: &LandingSite) -> Self {
        let SiteAnimation {
            delay,
            frame0_bias,
            open_frame,
            type_frames,
            dead_frames,
        } = site.animation;
        Self {
            delay,
            frame0_bias,
            open_pic: open_frame,
            flags2: site.flags2,
            frames: if site.destroyed && site.dead_graphic.is_some() {
                dead_frames
            } else {
                type_frames
            },
        }
    }

    /// The open frame `o`: its `CustPicID`, or frames / 2 when that is 0
    /// or less or at least frames - 1 (@0x2e83c-0x2e864).
    #[must_use]
    pub fn open_frame(&self) -> u16 {
        let frames = i32::from(self.frames);
        let pic = i32::from(self.open_pic);
        if pic <= 0 || pic >= frames - 1 {
            self.frames / 2
        } else {
            // Between 1 and frames - 2.
            pic as u16
        }
    }

    /// Whether it animates, `destroyed` or not: when its frame count is
    /// above 1 and its destroyed state is the one [`ONLY_WHEN_DESTROYED`]
    /// names (@0x2e784-0x2e80a).
    fn animates(&self, destroyed: bool) -> bool {
        self.frames > 1 && (self.flags2 & ONLY_WHEN_DESTROYED != 0) == destroyed
    }

    fn has(&self, bit: u16) -> bool {
        self.flags2 & bit != 0
    }

    /// The ticks a step waits for, on frame `current`, `held` open or not:
    /// `AnimDelay`, or `Frame0Bias` × `AnimDelay` held on the open frame
    /// with a bias above 1 (@0x2e9f8-0x2ea2c); released, `AnimDelay`
    /// alone (@0x2e9b6-0x2e9df).
    fn threshold(&self, current: u16, held: bool) -> i64 {
        let delay = i64::from(self.delay);
        if held && current == self.open_frame() && self.frame0_bias > 1 {
            i64::from(self.frame0_bias) * delay
        } else {
            delay
        }
    }
}

/// A hypergate's sprite state: the frame shown, the next, and its timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateSprite {
    current: u16,
    next: u16,
    timer: u64,
}

impl Default for GateSprite {
    fn default() -> Self {
        Self::new()
    }
}

impl GateSprite {
    /// A closed gate: frame 0, next 1, its timer at 0 (`_InitObjects`
    /// @0x1c8b9-0x1c8d1).
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: 0,
            next: 1,
            timer: 0,
        }
    }

    /// One tick of `anim`'s gate, `destroyed` or not, `held_open` or not,
    /// its random picks drawn on `chance`.
    pub fn step(
        &mut self,
        anim: &GateAnimation,
        destroyed: bool,
        held_open: bool,
        chance: &mut (impl Chance + ?Sized),
    ) {
        if !anim.animates(destroyed) {
            return;
        }
        self.timer += 1;
        if i128::from(self.timer) < i128::from(anim.threshold(self.current, held_open)) {
            return;
        }
        self.timer = 0;
        if held_open {
            self.open(anim, chance);
        } else {
            self.close(anim);
        }
    }

    /// One step held open (@0x2ea4f-0x2ebf1).
    fn open(&mut self, anim: &GateAnimation, chance: &mut (impl Chance + ?Sized)) {
        let open = anim.open_frame();
        if self.current < open {
            self.current += 1;
            self.next = self.current;
        } else if anim.has(ALTERNATE) {
            if self.current == open {
                self.current = self.next;
                self.next = if anim.has(RANDOM) {
                    self.pick(anim, &[open, self.current], chance)
                } else {
                    self.following(anim, open + 1)
                };
            } else {
                self.current = open;
            }
        } else {
            self.current = self.next;
            self.next = if anim.has(RANDOM) {
                self.pick(anim, &[self.current], chance)
            } else {
                self.following(anim, open)
            };
        }
    }

    /// The frame after the next in turn, wrapping round to `floor` below
    /// the open frame (@0x2eb02-0x2eb39, @0x2ebb6-0x2ebea).
    fn following(&self, anim: &GateAnimation, floor: u16) -> u16 {
        let after = (u32::from(self.next) + 1) % u32::from(anim.frames);
        match u16::try_from(after) {
            Ok(after) if after >= anim.open_frame() => after,
            _ => floor,
        }
    }

    /// A frame of the loop drawn on `chance`, `o` + `Rand(frames - o)`,
    /// drawn again while it is one of `not` (@0x2eabb-0x2eafb,
    /// @0x2eb6d-0x2ebb4); the next frame as it was when every frame of
    /// the loop is one of them, where the original would draw forever.
    fn pick(&self, anim: &GateAnimation, not: &[u16], chance: &mut (impl Chance + ?Sized)) -> u16 {
        let open = anim.open_frame();
        if (open..anim.frames).all(|frame| not.contains(&frame)) {
            return self.next;
        }
        let span = u32::from(anim.frames - open);
        loop {
            // Below frames - o, so the frame is below frames.
            let frame = open + chance.below(span) as u16;
            if !not.contains(&frame) {
                return frame;
            }
        }
    }

    /// One step released (@0x2ebf6-0x2ec46).
    fn close(&mut self, anim: &GateAnimation) {
        let open = anim.open_frame();
        if self.current < open {
            self.current = self.current.saturating_sub(1);
        } else if !anim.has(RANDOM) && self.current < anim.frames - 1 {
            self.current += 1;
        } else {
            self.current = open - 1;
        }
    }

    /// The frame drawn: 0 while it does not animate.
    #[must_use]
    pub fn shown(&self, anim: &GateAnimation, destroyed: bool) -> u16 {
        if anim.animates(destroyed) {
            self.current
        } else {
            0
        }
    }

    /// Whether it is open: on the open frame or past it.
    #[must_use]
    pub fn is_open(&self, anim: &GateAnimation) -> bool {
        self.current >= anim.open_frame()
    }
}

/// Whether `gate` is held open this tick: tested first, by the player's
/// `dock` request when it is for `gate` with its counter above 0
/// (`_dockTarget` and `_dockPermission`, @0x2e867-0x2e87e); then by any
/// of `npcs` coming out of
/// it ([`Npc::holds_gate_open`]; `_HandleStellarSprites`
/// @0x2e884-0x2e8d7), waiting in it (goal 0x15), not bursting (goal 8,
/// +0x6c = -1, @0x8dd8a-0x8dd93), or heading in to land on or enter it strictly
/// inside `reach` of it on both axes ([`Npc::approaches_gate`],
/// @0x2e8dd-0x2e9a2; see [`approach_reach`]).
#[must_use]
pub fn held_open(gate: &LandingSite, reach: f32, dock: Option<DockRequest>, npcs: &[Npc]) -> bool {
    dock.is_some_and(|request| request.stellar == gate.id && request.permission > 0)
        || npcs.iter().any(|npc| {
            npc.holds_gate_open(gate.id) || npc.approaches_gate(gate.id, gate.position, reach)
        })
}

/// How near a ship heading in must be to hold a gate open, on each axis,
/// strictly: twice the width of the sprite the gate's slot draws, grown
/// to trunc(× 1.1) once the gate is `open` (`_HandleStellarSprites`
/// @0x2e906-0x2e92e, the double at 0xdd1b0), the gate's frame as it stood
/// before this tick's step.
#[must_use]
pub fn approach_reach(sprite_width: u32, open: bool) -> f32 {
    let reach = 2.0 * f64::from(sprite_width);
    let reach = if open {
        (reach * APPROACH_GROWTH).trunc()
    } else {
        reach
    };
    // A sprite's width, doubled and grown a tenth: well inside f32's
    // exact integers.
    #[allow(clippy::cast_possible_truncation)]
    let reach = reach as f32;
    reach
}

/// How much farther a ship heading in holds an open gate open: 1.1 (the
/// double at 0xdd1b0).
pub const APPROACH_GROWTH: f64 = 1.1;

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::chance::NeverFires;
    use nova_sim_base::fixture::Draws;

    fn gate(frames: u16, delay: i16) -> GateAnimation {
        GateAnimation {
            delay,
            frame0_bias: 0,
            open_pic: 0,
            flags2: 0,
            frames,
        }
    }

    /// The frames shown over `ticks` ticks, held open as `held` says.
    fn run(
        sprite: &mut GateSprite,
        anim: &GateAnimation,
        ticks: usize,
        held: bool,
        chance: &mut impl Chance,
    ) -> Vec<u16> {
        (0..ticks)
            .map(|_| {
                sprite.step(anim, false, held, chance);
                sprite.shown(anim, false)
            })
            .collect()
    }

    #[test]
    fn the_open_frame_is_cust_pic_id_within_1_to_frames_less_2_and_half_the_frames_otherwise() {
        let with = |open_pic| GateAnimation {
            open_pic,
            ..gate(10, 1)
        };
        assert_eq!(with(1).open_frame(), 1);
        assert_eq!(with(3).open_frame(), 3);
        assert_eq!(with(8).open_frame(), 8);
        for half in [0, -1, 9, 10, 400] {
            assert_eq!(with(half).open_frame(), 5, "{half}");
        }
        assert_eq!(gate(42, 0).open_frame(), 21);
        assert_eq!(gate(7, 0).open_frame(), 3);
    }

    #[test]
    fn a_sites_gate_animation_counts_its_dead_sheet_while_destroyed_with_one() {
        let site = LandingSite {
            flags2: crate::gate::HYPERGATE | RANDOM | ONLY_WHEN_DESTROYED,
            dead_graphic: Some(6),
            animation: SiteAnimation {
                delay: 3,
                frame0_bias: 2,
                open_frame: 9,
                type_frames: 42,
                dead_frames: 8,
            },
            ..nova_sim_catalog::fixture::planet(128, 0.0, 0.0)
        };
        let anim = GateAnimation {
            delay: 3,
            frame0_bias: 2,
            open_pic: 9,
            flags2: crate::gate::HYPERGATE | RANDOM | ONLY_WHEN_DESTROYED,
            frames: 42,
        };
        assert_eq!(GateAnimation::of(&site), anim);
        let dead = LandingSite {
            destroyed: true,
            ..site
        };
        assert_eq!(
            GateAnimation::of(&dead),
            GateAnimation { frames: 8, ..anim }
        );
        let no_dead_sheet = LandingSite {
            dead_graphic: None,
            ..dead
        };
        assert_eq!(GateAnimation::of(&no_dead_sheet), anim);
    }

    #[test]
    fn a_closed_gate_not_held_stays_on_frame_0() {
        let anim = gate(8, 1);
        let mut sprite = GateSprite::new();
        assert_eq!(run(&mut sprite, &anim, 50, false, &mut NeverFires), [0; 50]);
    }

    #[test]
    fn held_it_opens_a_frame_each_delay_then_loops_from_the_open_frame() {
        // 6 frames, o = 3.
        let anim = gate(6, 2);
        let mut sprite = GateSprite::new();
        assert_eq!(
            run(&mut sprite, &anim, 18, true, &mut NeverFires),
            [0, 1, 1, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 3, 3, 4, 4, 5]
        );
    }

    #[test]
    fn a_bias_holds_the_open_frame_that_many_delays_and_no_other() {
        let anim = GateAnimation {
            frame0_bias: 3,
            ..gate(6, 2)
        };
        let mut sprite = GateSprite::new();
        let shown = run(&mut sprite, &anim, 30, true, &mut NeverFires);
        // Opening ignores the bias: 1 at tick 2, 2 at 4, 3 at 6.
        assert_eq!(shown[..6], [0, 1, 1, 2, 2, 3]);
        // On 3 it waits 6 ticks a step: 3 (again) at 12, 4 at 18.
        assert_eq!(shown[6..17], [3; 11]);
        assert_eq!(shown[17..21], [4, 4, 5, 5]);
        // Then back to 3 at 22, held there 6 ticks.
        assert_eq!(shown[21..27], [3; 6]);
        assert_eq!(shown[27], 4);
    }

    #[test]
    fn a_bias_of_1_or_below_holds_the_open_frame_one_delay() {
        for bias in [1, 0, -1] {
            let anim = GateAnimation {
                frame0_bias: bias,
                ..gate(6, 2)
            };
            let mut sprite = GateSprite::new();
            assert_eq!(
                run(&mut sprite, &anim, 12, true, &mut NeverFires),
                [0, 1, 1, 2, 2, 3, 3, 3, 3, 4, 4, 5],
                "{bias}"
            );
        }
    }

    #[test]
    fn a_delay_of_0_or_below_steps_every_tick_as_a_stock_gate_does() {
        for delay in [0, -1] {
            let anim = gate(42, delay);
            let mut sprite = GateSprite::new();
            let shown = run(&mut sprite, &anim, 23, true, &mut NeverFires);
            let opening: Vec<u16> = (1..=21).collect();
            assert_eq!(shown[..21], opening[..], "{delay}");
            assert_eq!(shown[21..], [21, 22], "{delay}");
        }
    }

    #[test]
    fn alternating_it_shows_the_open_frame_between_every_other_and_wraps_past_it() {
        // 6 frames, o = 3, every tick.
        let anim = GateAnimation {
            flags2: ALTERNATE,
            ..gate(6, 1)
        };
        let mut sprite = GateSprite::new();
        assert_eq!(
            run(&mut sprite, &anim, 12, true, &mut NeverFires),
            [1, 2, 3, 3, 4, 3, 5, 3, 4, 3, 5, 3]
        );
    }

    #[test]
    fn random_it_picks_the_next_frame_among_the_loop_drawing_again_on_the_frame_shown() {
        // 8 frames, o = 4: draws below 4.
        let anim = GateAnimation {
            flags2: RANDOM,
            ..gate(8, 1)
        };
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 4, true, &mut NeverFires);
        assert_eq!(sprite.shown(&anim, false), 4);
        // At 4, next 4 (from the opening): shows 4, picks again. 0 is 4,
        // the frame shown, so it draws again: 2 gives 6. At 6 a draw of
        // 2 is 6 again: drawn again, 3 gives 7. At 7, 0 gives 4.
        let mut draws = Draws::of(&[0, 2, 2, 3, 0]);
        assert_eq!(run(&mut sprite, &anim, 3, true, &mut draws), [4, 6, 7]);
        assert_eq!(draws.asked, [4; 5]);
    }

    #[test]
    fn random_and_alternating_it_draws_again_on_the_open_frame_too() {
        let anim = GateAnimation {
            flags2: RANDOM | ALTERNATE,
            ..gate(8, 1)
        };
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 4, true, &mut NeverFires);
        // At 4 (next 4): shows 4, picks: 0 is 4, drawn again; 1 gives 5.
        // Then 5, picking: 1 is 5, the frame shown, drawn again; 0 is 4,
        // again; 3 gives 7. Then 4, then 7, picking: 1 gives 5. Then 4.
        let mut draws = Draws::of(&[0, 1, 1, 0, 3, 1]);
        assert_eq!(
            run(&mut sprite, &anim, 5, true, &mut draws),
            [4, 5, 4, 7, 4]
        );
        assert_eq!(draws.asked, [4; 6]);
    }

    #[test]
    fn a_random_pick_with_no_frame_left_neither_hangs_nor_moves_the_next_frame() {
        // 4 frames with CustPicID 0: o = 2, frames - o = 2. Alternating
        // and random, the loop is {2, 3} less 2 and the frame shown.
        let anim = GateAnimation {
            flags2: RANDOM | ALTERNATE,
            ..gate(4, 1)
        };
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 2, true, &mut NeverFires);
        // At 2, next 2: shows 2, picks among {3}: 3. Then shows 3, and
        // picks among none: no draw.
        let mut draws = Draws::of(&[1]);
        assert_eq!(run(&mut sprite, &anim, 2, true, &mut draws), [2, 3]);
        assert_eq!(draws.asked, [2]);
        // Back to 2, then 3 again: next stayed 3.
        let mut none = Draws::of(&[]);
        assert_eq!(run(&mut sprite, &anim, 4, true, &mut none), [2, 3, 2, 3]);
        assert!(none.asked.is_empty());
        // 3 frames, CustPicID 0: o = 1. Random alone, {1, 2} less the
        // frame shown always leaves one.
        let three = GateAnimation {
            flags2: RANDOM,
            ..gate(3, 1)
        };
        let mut sprite = GateSprite::new();
        let mut draws = Draws::of(&[0, 1, 0]);
        assert_eq!(run(&mut sprite, &three, 4, true, &mut draws), [1, 1, 2, 1]);
    }

    #[test]
    fn released_from_its_loop_it_plays_it_out_then_closes_to_0() {
        let anim = gate(6, 1);
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 5, true, &mut NeverFires);
        assert_eq!(sprite.shown(&anim, false), 4);
        assert_eq!(
            run(&mut sprite, &anim, 7, false, &mut NeverFires),
            [5, 2, 1, 0, 0, 0, 0]
        );
    }

    #[test]
    fn released_random_it_closes_at_once() {
        let anim = GateAnimation {
            flags2: RANDOM,
            ..gate(6, 1)
        };
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 5, true, &mut Draws::of(&[2, 0]));
        assert_eq!(sprite.shown(&anim, false), 5);
        assert_eq!(
            run(&mut sprite, &anim, 4, false, &mut NeverFires),
            [2, 1, 0, 0]
        );
    }

    #[test]
    fn released_it_waits_the_delay_alone_with_no_bias() {
        let anim = GateAnimation {
            frame0_bias: 4,
            ..gate(6, 2)
        };
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 6, true, &mut NeverFires);
        assert_eq!(sprite.shown(&anim, false), 3);
        // On 3 held it would wait 8; released it steps in 2.
        assert_eq!(
            run(&mut sprite, &anim, 4, false, &mut NeverFires),
            [3, 4, 4, 5]
        );
    }

    #[test]
    fn held_again_while_closing_it_opens_from_where_it_is() {
        let anim = gate(8, 1);
        let mut sprite = GateSprite::new();
        run(&mut sprite, &anim, 6, true, &mut NeverFires);
        // 6, 7, then 3 and 2 closing.
        run(&mut sprite, &anim, 4, false, &mut NeverFires);
        assert_eq!(sprite.shown(&anim, false), 2);
        assert_eq!(
            run(&mut sprite, &anim, 4, true, &mut NeverFires),
            [3, 4, 4, 5]
        );
    }

    #[test]
    fn a_gate_that_does_not_animate_shows_0_and_keeps_its_state_frozen() {
        let anim = gate(6, 2);
        let mut sprite = GateSprite::new();
        sprite.step(&anim, false, true, &mut NeverFires);
        // One tick into its delay; destroyed, it does not animate.
        for _ in 0..10 {
            sprite.step(&anim, true, true, &mut NeverFires);
            assert_eq!(sprite.shown(&anim, true), 0);
        }
        // Intact again it picks up where it was: one more tick to frame 1.
        sprite.step(&anim, false, true, &mut NeverFires);
        assert_eq!(sprite.shown(&anim, false), 1);
        // Shown destroyed, it draws 0 whatever its frame.
        assert_eq!(sprite.shown(&anim, true), 0);
        // One that animates only while destroyed animates then alone.
        let ruin = GateAnimation {
            flags2: ONLY_WHEN_DESTROYED,
            ..gate(6, 1)
        };
        let mut sprite = GateSprite::new();
        sprite.step(&ruin, false, true, &mut NeverFires);
        assert_eq!(sprite.shown(&ruin, false), 0);
        sprite.step(&ruin, true, true, &mut NeverFires);
        assert_eq!(sprite.shown(&ruin, true), 1);
        assert_eq!(sprite.shown(&ruin, false), 0);
    }

    #[test]
    fn a_gate_of_1_frame_or_none_never_animates() {
        for frames in [0, 1] {
            let anim = gate(frames, 0);
            let mut sprite = GateSprite::new();
            assert_eq!(
                run(&mut sprite, &anim, 5, true, &mut NeverFires),
                [0; 5],
                "{frames}"
            );
            assert_eq!(sprite, GateSprite::new(), "{frames}");
        }
        // 2 frames animate: o = 1.
        let two = gate(2, 0);
        let mut sprite = GateSprite::new();
        assert_eq!(run(&mut sprite, &two, 1, true, &mut NeverFires), [1]);
    }

    /// Hypergate `id` at (100, -50).
    fn gate_site(id: i16) -> LandingSite {
        LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..nova_sim_catalog::fixture::planet(id, 100.0, -50.0)
        }
    }

    #[test]
    fn the_players_dock_gate_is_held_open_while_its_counter_is_above_0() {
        use crate::clearance::{CLEARED, DockRequest, EXPIRES_PAST, REQUESTED_NEAR};
        let gate = gate_site(130);
        let request = |stellar, permission| {
            Some(DockRequest {
                stellar,
                permission,
            })
        };
        for permission in [REQUESTED_NEAR, CLEARED, 0x2ef, EXPIRES_PAST, 1] {
            assert!(
                held_open(&gate, 0.0, request(gate.id, permission), &[]),
                "{permission:#x}"
            );
        }
        for permission in [0, -1] {
            assert!(!held_open(&gate, 0.0, request(gate.id, permission), &[]));
        }
        assert!(!held_open(&gate, 0.0, None, &[]));
        assert!(!held_open(
            &gate,
            0.0,
            request(StellarId(131), CLEARED),
            &[]
        ));
    }

    #[test]
    fn a_gate_is_held_open_by_any_ship_coming_out_of_it() {
        use crate::stats::ShipStats;
        use crate::traffic::npc::Mode;
        let gate_id = StellarId(130);
        let flying = crate::traffic::fixture::npc(1, ShipStats::default());
        let emerging = Npc {
            mode: Mode::Emerging { ticks_left: 9 },
            emerges_from: Some(gate_id),
            ..crate::traffic::fixture::npc(2, ShipStats::default())
        };
        let (gate, other) = (gate_site(130), gate_site(131));
        assert!(!held_open(&gate, 0.0, None, &[]));
        assert!(!held_open(&gate, 0.0, None, std::slice::from_ref(&flying)));
        assert!(held_open(
            &gate,
            0.0,
            None,
            &[flying.clone(), emerging.clone()]
        ));
        assert!(!held_open(&other, 0.0, None, &[flying, emerging]));
    }

    #[test]
    fn a_gate_is_held_open_by_a_ship_heading_in_strictly_inside_its_reach() {
        use crate::ai::Goal;
        use crate::combat::hull::Condition;
        use crate::geometry::Vec2;
        use crate::stats::ShipStats;
        let gate = gate_site(130);
        let at = |goal, x: f32, y: f32| Npc {
            goal,
            state: crate::flight::ShipState {
                position: Vec2::new(gate.position.x + x, gate.position.y + y),
                ..crate::flight::ShipState::default()
            },
            ..crate::traffic::fixture::npc(1, ShipStats::default())
        };
        let held = |npc: Npc| held_open(&gate, 200.0, None, &[npc]);
        for goal in [Goal::Land(gate.id), Goal::EnterGate(gate.id)] {
            assert!(held(at(goal, 199.9, -199.9)), "{goal:?}");
            assert!(held(at(goal, -199.9, 199.9)), "{goal:?}");
            assert!(!held(at(goal, 200.0, 0.0)), "{goal:?}: not at 2w");
            assert!(!held(at(goal, -200.0, 0.0)), "{goal:?}");
            assert!(!held(at(goal, 0.0, 200.0)), "{goal:?}");
            assert!(!held(at(goal, 0.0, -200.0)), "{goal:?}");
            let disabled = Npc {
                condition: Condition::Disabled,
                ..at(goal, 0.0, 0.0)
            };
            assert!(!held(disabled), "{goal:?}: disabled");
        }
        assert!(
            !held(at(Goal::EnterGate(StellarId(131)), 0.0, 0.0)),
            "another gate"
        );
        assert!(!held(at(Goal::Idle, 0.0, 0.0)), "another goal");
        assert!(!held(at(Goal::JumpOut, 0.0, 0.0)));
    }

    #[test]
    fn the_approach_reach_is_twice_the_sprite_width_grown_a_tenth_once_open() {
        assert_eq!(approach_reach(100, false), 200.0);
        assert_eq!(approach_reach(100, true), 220.0);
        assert_eq!(approach_reach(33, true), 72.0, "72.6");
        assert_eq!(approach_reach(5, true), 11.0);
        assert_eq!(approach_reach(32, false), 64.0);
    }

    #[test]
    fn it_is_open_from_the_open_frame() {
        let anim = gate(6, 1);
        let mut sprite = GateSprite::new();
        let open: Vec<bool> = (0..5)
            .map(|_| {
                sprite.step(&anim, false, true, &mut NeverFires);
                sprite.is_open(&anim)
            })
            .collect();
        assert_eq!(open, [false, false, true, true, true]);
        assert!(!GateSprite::new().is_open(&anim));
    }
}
