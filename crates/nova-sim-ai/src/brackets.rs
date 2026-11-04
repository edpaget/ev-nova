//! The corner brackets round the nav target and the ship target: when
//! they show, how far out they start and how fast they fly in, and which
//! colour a ship target's take. Where they are drawn, and with which
//! `cïcn`, is the view's.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`, `_DoPlayGameWork` calls
//! `_HandlePlayer` (@0x43e19), where selections happen, then
//! `_HandleNavTargetSprites` (@0x444fa), `_HandleWeapTargetSprites`
//! (@0x444ff) and `_HandleAlertSprite` (@0x4451e); `_PlayGame` draws the
//! sprites after that (`_SWAnimateSpriteWorld` @0x45e21).
//!
//! - **The fly-in** ([`FlyIn`]). A selection sets the expand to
//!   [`FLY_IN_START`] (0x43800000). Both bracket routines step it
//!   *before* they place the corners (nav @0x2fcd5-0x2fd0d before
//!   @0x2fe46, ship @0x2ff35-0x2ff6d before @0x300ed): while it is above
//!   0 it falls by 60.0 (0xdd5b0) x `_gSpeedMult` (@0x2fce7, @0x2ff47),
//!   in doubles, then is clamped at 0. So at full gain the 256 is never
//!   drawn: the drawn sequence is 196, 136, 76, 16, 0, settling on the
//!   fifth pass, the selection's own counted; at half gain it is 226,
//!   196, … 16, 0, settling on the ninth. The gain is the session's,
//!   handed in as a value; no frozen gate stops it.
//! - **Nav brackets** (`_HandleNavTargetSprites` @0x2fcaf-0x2ff19) show
//!   only in nav mode 2 with a stellar target (@0x2fcbf-0x2fccf);
//!   otherwise all four are hidden and the expand set to 0 on every pass
//!   (@0x2fee5-0x2ff10). They take the dominated frames when the
//!   stellar's +0x46 byte is set (@0x2fd40-0x2fd76), the byte
//!   `_ColorCodeStellar` reads ([`NavBrackets::dominated`]).
//! - **Ship brackets** (`_HandleWeapTargetSprites` @0x2ff1a-0x301b8) are
//!   hidden, the expand set to 0, with no target (@0x30184-0x301af). Their
//!   frame ([`standing`], @0x2ffac-0x30021) tests a disabled ship, then
//!   the player's escort, then a threat.
//! - **A new ship target** (Tab / Shift-Tab @0x69fd4, a click @0x6a347
//!   on a change @0x6a51c, R / Option-R @0x6aa77 on a change @0x6aa28)
//!   reaches a tail that sets the expand to 256 when the target's sprite
//!   is on screen ([`sprite_in_area`], `_SWIsSpriteInRect`), and to 0
//!   otherwise (@0x6a5c0-0x6a5e4, @0x6aa95-0x6aab6). A pick that leaves
//!   the target as it was writes nothing.
//!
//! Neither routine plays a sound: the selection sounds are
//! `_HandlePlayer`'s.
//!
//! Never reached: the defender case of the ship frame (+0x8c != −1 with
//! the player as parent, @0x2ffd3), as no ship the player leads defends
//! a stellar: a defence ship spawns with no leader (-1), a pirate's take
//! leaves it led by the pirate, and the player's capture clears its
//! +0x8c. Left out:
//! `_SWIsSpriteInRect`'s per-frame insets
//! (frame +0x2e-+0x34), as ours has no trimmed frame bounds and uses the
//! whole sprite.

use crate::catalog::StellarId;
use crate::cloak::Area;
use crate::geometry::Vec2;
use crate::speed_mult::SpeedMult;
use crate::traffic::npc::{Npc, NpcId};

/// How far out a selection starts the brackets (0x43800000).
pub const FLY_IN_START: f32 = 256.0;

/// How far the brackets fly in a pass at full gain (−60.0 at 0xdd5b0):
/// the step is this × the frame's gain ([`FlyIn::step`]).
pub const FLY_IN_STEP: f32 = 60.0;

/// How far out a set of brackets is, flying in from a selection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlyIn {
    expand: f32,
    /// Started since the last step: the original steps the start in the
    /// selection's own pass before drawing it.
    fresh: bool,
}

impl FlyIn {
    /// Starts the fly-in from [`FLY_IN_START`].
    pub fn start(&mut self) {
        *self = Self {
            expand: FLY_IN_START,
            fresh: true,
        };
    }

    /// Settles the brackets at once.
    pub fn settle(&mut self) {
        *self = Self::default();
    }

    /// One pass at the frame's `gain`: while the expand is above 0 the
    /// brackets fly in by [`FLY_IN_STEP`] × the gain, in doubles, then an
    /// expand below 0 is clamped at 0 (`ucomiss 0, e` / `jbe`, `ucomiss e,
    /// 0` / `jbe`, @0x2fcd5-0x2fd0d, @0x2ff35-0x2ff6d): a NaN expand is
    /// held.
    pub fn step(&mut self, gain: SpeedMult) {
        if self.expand > 0.0 {
            self.expand = gain.add_times(self.expand, -FLY_IN_STEP);
        }
        if self.expand < 0.0 {
            self.expand = 0.0;
        }
        self.fresh = false;
    }

    /// How far out the brackets are drawn.
    ///
    /// The original takes a selection in `_HandlePlayer` and steps the
    /// expand in the same pass before drawing it. Ours takes a key between
    /// ticks, so a frame drawn before the next tick shows what that pass
    /// would draw: a fresh start is drawn already stepped once, at `next`,
    /// the gain that pass will run at (196 at full gain). The next tick
    /// steps it there, so the frames from the key are 196, 196, 136, …;
    /// the extra frame never shows a value the original doesn't. After a
    /// take-off's zero, the next pass is held at 0, so a selection then is
    /// drawn at 256, as the original's pass at gain 0 draws it.
    #[must_use]
    pub fn expand(&self, next: SpeedMult) -> f32 {
        if self.fresh {
            let mut stepped = *self;
            stepped.step(next);
            stepped.expand
        } else {
            self.expand
        }
    }
}

/// How a ship target stands towards the player, for its brackets' frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// A threat to the player: red.
    Hostile,
    /// Any other ship: yellow.
    Neutral,
    /// The player's escort, or a ship one leads: green.
    Escort,
    /// Disabled or breaking up: grey.
    Disabled,
}

/// How `npc`, led by `leader` (the ship its [`leader`](Npc::leader)
/// names, if it is in the system), among `npcs`, stands for its
/// brackets, in `_HandleWeapTargetSprites`'s order (@0x2ffac-0x30021):
/// disabled (`_IsDisabled`, as `nova_sim::iff::ship_iff`
/// reads it, [`Npc::is_disabled`]: not a stellar's defence ship breaking
/// up); else the player's escort or a ship one leads, the
/// parent/grandparent rule of `_ColorCodeShip` (@0x4b46-0x4b8f); else a
/// threat to the player or a ship the player leads (`_IsThreatToPlayer`
/// @0x2ffe2, [`Npc::threatens_player_among`]); else neutral. Unlike `_ColorCodeShip`, which
/// tests the threat first (@0x4b27), an escort attacking the player is
/// still an escort here.
#[must_use]
pub fn standing(npc: &Npc, leader: Option<&Npc>, npcs: &[Npc]) -> Standing {
    if npc.is_disabled() {
        Standing::Disabled
    } else if npc.escorts_player() || leader.is_some_and(Npc::escorts_player) {
        Standing::Escort
    } else if npc.threatens_player_among(npcs) {
        Standing::Hostile
    } else {
        Standing::Neutral
    }
}

/// Whether a sprite `width` x `height` centred on `at` is in `area`
/// (`_SWIsSpriteInRect` @0xac282): whether the two strictly overlap, so a
/// sprite only touching an edge is not.
#[must_use]
pub fn sprite_in_area(area: Area, at: Vec2, width: f32, height: f32) -> bool {
    Area::centred(at, width, height).overlaps(area)
}

/// The brackets round the nav target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavBrackets {
    /// The stellar bracketed.
    pub stellar: StellarId,
    /// The stellar's spöb position (+0x4/+0x6), where the brackets are
    /// drawn whether its system shows it or not (`_HandleNavTargetSprites`
    /// @0x2fd9f-0x2fdda).
    pub position: Vec2,
    /// How far out they are ([`FlyIn::expand`]).
    pub expand: f32,
    /// Whether the stellar is dominated: the green frames.
    pub dominated: bool,
}

/// The brackets round the ship target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetBrackets {
    /// The ship bracketed.
    pub npc: NpcId,
    /// How far out they are ([`FlyIn::expand`]).
    pub expand: f32,
    /// How it stands towards the player: the frames.
    pub standing: Standing,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;

    /// The expand drawn after each of `steps` steps.
    fn steps(fly_in: &mut FlyIn, steps: usize) -> Vec<f32> {
        (0..steps)
            .map(|_| {
                fly_in.step(SpeedMult::FULL);
                fly_in.expand(SpeedMult::FULL)
            })
            .collect()
    }

    #[test]
    fn the_fly_in_draws_196_136_76_16_then_0_after_a_start() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        assert_eq!(
            fly_in.expand(SpeedMult::FULL),
            196.0,
            "drawn before the next step"
        );
        assert_eq!(steps(&mut fly_in, 6), [196.0, 136.0, 76.0, 16.0, 0.0, 0.0]);
        assert_eq!((FLY_IN_START, FLY_IN_STEP), (256.0, 60.0));
    }

    #[test]
    fn a_step_from_0_stays_0_and_settle_is_0_at_once() {
        let mut fly_in = FlyIn::default();
        assert_eq!(fly_in.expand(SpeedMult::FULL), 0.0);
        assert_eq!(steps(&mut fly_in, 2), [0.0, 0.0]);
        fly_in.start();
        fly_in.step(SpeedMult::FULL);
        fly_in.settle();
        assert_eq!(fly_in.expand(SpeedMult::FULL), 0.0);
        assert_eq!(fly_in, FlyIn::default());
        fly_in.start();
        fly_in.settle();
        assert_eq!(fly_in.expand(SpeedMult::FULL), 0.0, "a fresh start settled");
    }

    #[test]
    fn a_restart_part_way_flies_in_from_256_again() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        steps(&mut fly_in, 3);
        fly_in.start();
        assert_eq!(fly_in.expand(SpeedMult::FULL), 196.0);
        assert_eq!(steps(&mut fly_in, 2), [196.0, 136.0]);
    }

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// The expand drawn after each of `steps` steps at `gain`, the next
    /// gain `gain` too.
    fn steps_at(fly_in: &mut FlyIn, steps: usize, gain: SpeedMult) -> Vec<f32> {
        (0..steps)
            .map(|_| {
                fly_in.step(gain);
                fly_in.expand(gain)
            })
            .collect()
    }

    #[test]
    fn at_half_gain_the_fly_in_draws_226_196_and_on_settling_on_the_ninth_pass() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        let mut drawn = vec![fly_in.expand(HALF)];
        drawn.extend(steps_at(&mut fly_in, 9, HALF));
        assert_eq!(
            drawn,
            [
                226.0, 226.0, 196.0, 166.0, 136.0, 106.0, 76.0, 46.0, 16.0, 0.0
            ]
        );
    }

    #[test]
    fn at_zero_gain_the_fly_in_holds_and_a_settled_one_stays_settled() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        assert_eq!(fly_in.expand(SpeedMult::ZERO), 256.0, "drawn at 256");
        assert_eq!(steps_at(&mut fly_in, 2, SpeedMult::ZERO), [256.0, 256.0]);
        let mut settled = FlyIn::default();
        assert_eq!(steps_at(&mut settled, 1, HALF), [0.0], "from 0 it stays 0");
    }

    #[test]
    fn a_fresh_starts_expand_is_the_expand_after_one_step_at_the_next_gain() {
        for next in [HALF, SpeedMult::ZERO, SpeedMult::FULL] {
            let mut fresh = FlyIn::default();
            fresh.start();
            let mut stepped = fresh;
            stepped.step(next);
            assert_eq!(fresh.expand(next), stepped.expand(next), "{next:?}");
        }
    }

    #[test]
    fn a_nan_expand_is_held_and_a_negative_one_clamped() {
        let mut nan = FlyIn {
            expand: f32::NAN,
            fresh: false,
        };
        nan.step(SpeedMult::FULL);
        assert!(nan.expand(SpeedMult::FULL).is_nan(), "held");
        let mut below = FlyIn {
            expand: -5.0,
            fresh: false,
        };
        below.step(SpeedMult::FULL);
        assert_eq!(below.expand(SpeedMult::FULL), 0.0, "clamped");
    }

    #[test]
    fn a_settled_fly_in_takes_no_step_even_at_a_nan_gain() {
        // `ucomiss 0, e` / `jbe` skips the step at 0, so no gain reaches it.
        let mut settled = FlyIn::default();
        settled.step(SpeedMult::new(f64::NAN));
        assert_eq!(settled.expand(SpeedMult::FULL), 0.0);
    }

    #[test]
    fn a_full_gain_step_drops_a_start_by_fly_in_step() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        fly_in.step(SpeedMult::FULL);
        assert_eq!(fly_in.expand(SpeedMult::FULL), FLY_IN_START - FLY_IN_STEP);
    }

    fn hired() -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..npc(1, ShipStats::default())
        }
    }

    fn attacking(npc: Npc) -> Npc {
        Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..npc
        }
    }

    fn in_condition(npc: Npc, condition: Condition) -> Npc {
        Npc { condition, ..npc }
    }

    #[test]
    fn a_defence_ship_breaking_up_is_not_bracketed_disabled() {
        let guard = crate::traffic::fixture::breaking_up_guard(npc(1, ShipStats::default()));
        assert_eq!(standing(&guard, None, &[]), Standing::Neutral, "@0x2d6f");
    }

    #[test]
    fn standing_is_disabled_first_then_escort_then_threat_then_neutral() {
        let idle = npc(2, ShipStats::default());
        let fighter = Npc {
            leader: Some(NpcId(1)),
            ..npc(2, ShipStats::default())
        };
        let dying = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            for ship in [hired(), attacking(hired()), attacking(idle.clone())] {
                assert_eq!(
                    standing(&in_condition(ship, condition), None, &[]),
                    Standing::Disabled,
                    "{condition:?}"
                );
            }
        }
        assert_eq!(standing(&hired(), None, &[]), Standing::Escort);
        assert_eq!(
            standing(&attacking(hired()), None, &[]),
            Standing::Escort,
            "an escort before a threat, unlike the IFF"
        );
        assert_eq!(standing(&fighter, Some(&hired()), &[]), Standing::Escort);
        assert_eq!(
            standing(&attacking(fighter.clone()), Some(&hired()), &[]),
            Standing::Escort,
            "an escort's fighter attacking the player"
        );
        assert_eq!(
            standing(&attacking(idle.clone()), None, &[]),
            Standing::Hostile
        );
        assert_eq!(standing(&idle, None, &[]), Standing::Neutral);
        let led = npc(1, ShipStats::default());
        assert_eq!(
            standing(&fighter, Some(&led), &[]),
            Standing::Neutral,
            "led by a ship that escorts no one"
        );
        assert_eq!(
            standing(&attacking(fighter), Some(&led), &[]),
            Standing::Hostile
        );
        let attacking_hired = Npc {
            goal: Goal::Attack(ShipRef::Npc(NpcId(1))),
            ..idle.clone()
        };
        assert_eq!(
            standing(&attacking_hired, None, &[hired()]),
            Standing::Hostile,
            "attacking a ship the player leads"
        );
        assert_eq!(
            standing(&attacking_hired, None, &[npc(1, ShipStats::default())]),
            Standing::Neutral,
            "attacking a ship nobody leads"
        );
    }

    /// The area from (0, 0) to (100, 50).
    const AREA: Area = Area {
        left: 0.0,
        top: 0.0,
        right: 100.0,
        bottom: 50.0,
    };

    #[test]
    fn a_sprite_is_in_the_area_only_when_it_strictly_overlaps() {
        let inside = |x: f32, y: f32| sprite_in_area(AREA, Vec2::new(x, y), 20.0, 10.0);
        assert!(inside(50.0, 25.0));
        // Touching each edge from outside, then 1 in.
        assert!(!inside(-10.0, 25.0), "left");
        assert!(inside(-9.0, 25.0));
        assert!(!inside(110.0, 25.0), "right");
        assert!(inside(109.0, 25.0));
        assert!(!inside(50.0, -5.0), "top");
        assert!(inside(50.0, -4.0));
        assert!(!inside(50.0, 55.0), "bottom");
        assert!(inside(50.0, 54.0));
        let point = |x: f32, y: f32| sprite_in_area(AREA, Vec2::new(x, y), 0.0, 0.0);
        assert!(point(50.0, 25.0), "a 0 x 0 sprite inside");
        assert!(!point(0.0, 25.0), "on the left edge");
        assert!(!point(100.0, 25.0), "on the right edge");
        assert!(!point(50.0, 0.0), "on the top edge");
        assert!(!point(50.0, 50.0), "on the bottom edge");
    }
}
