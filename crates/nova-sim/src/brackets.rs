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
//!   0 it falls by 60.0 (0xdd5b0) x `_gSpeedMult`, then is clamped at 0.
//!   So the 256 is never drawn: the drawn sequence is 196, 136, 76, 16,
//!   0, settling on the fifth pass, the selection's own counted.
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
//! Left out: the defender case of the ship frame (+0x8c != −1 with the
//! player as parent, @0x2ffd3), as ours has no ship led by the player
//! that defends a stellar; and `_SWIsSpriteInRect`'s per-frame insets
//! (frame +0x2e-+0x34), as ours has no trimmed frame bounds and uses the
//! whole sprite.

use crate::catalog::StellarId;
use crate::cloak::Area;
use crate::combat::hull::Condition;
use crate::geometry::Vec2;
use crate::traffic::npc::{Npc, NpcId};

/// How far out a selection starts the brackets (0x43800000).
pub const FLY_IN_START: f32 = 256.0;

/// How far the brackets fly in each pass (−60.0 at 0xdd5b0).
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

    /// One pass: the brackets fly in by [`FLY_IN_STEP`], no further than
    /// settled.
    pub fn step(&mut self) {
        self.expand = (self.expand - FLY_IN_STEP).max(0.0);
        self.fresh = false;
    }

    /// How far out the brackets are drawn.
    ///
    /// The original takes a selection in `_HandlePlayer` and steps the
    /// expand in the same pass before drawing it. Ours takes a key between
    /// ticks, so a frame drawn before the next tick shows what that pass
    /// would draw: a fresh start is drawn already stepped once, 196. The
    /// next tick steps it there, so the frames from the key are 196, 196,
    /// 136, …; the extra frame never shows a value the original doesn't.
    #[must_use]
    pub fn expand(&self) -> f32 {
        if self.fresh {
            let mut stepped = *self;
            stepped.step();
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
/// reads it); else the player's escort or a ship one leads, the
/// parent/grandparent rule of `_ColorCodeShip` (@0x4b46-0x4b8f); else a
/// threat to the player or a ship the player leads (`_IsThreatToPlayer`
/// @0x2ffe2, [`Npc::threatens_player_among`]); else neutral. Unlike `_ColorCodeShip`, which
/// tests the threat first (@0x4b27), an escort attacking the player is
/// still an escort here.
#[must_use]
pub fn standing(npc: &Npc, leader: Option<&Npc>, npcs: &[Npc]) -> Standing {
    if matches!(npc.condition, Condition::Disabled | Condition::Dying { .. }) {
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
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;

    /// The expand drawn after each of `steps` steps.
    fn steps(fly_in: &mut FlyIn, steps: usize) -> Vec<f32> {
        (0..steps)
            .map(|_| {
                fly_in.step();
                fly_in.expand()
            })
            .collect()
    }

    #[test]
    fn the_fly_in_draws_196_136_76_16_then_0_after_a_start() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        assert_eq!(fly_in.expand(), 196.0, "drawn before the next step");
        assert_eq!(steps(&mut fly_in, 6), [196.0, 136.0, 76.0, 16.0, 0.0, 0.0]);
        assert_eq!((FLY_IN_START, FLY_IN_STEP), (256.0, 60.0));
    }

    #[test]
    fn a_step_from_0_stays_0_and_settle_is_0_at_once() {
        let mut fly_in = FlyIn::default();
        assert_eq!(fly_in.expand(), 0.0);
        assert_eq!(steps(&mut fly_in, 2), [0.0, 0.0]);
        fly_in.start();
        fly_in.step();
        fly_in.settle();
        assert_eq!(fly_in.expand(), 0.0);
        assert_eq!(fly_in, FlyIn::default());
        fly_in.start();
        fly_in.settle();
        assert_eq!(fly_in.expand(), 0.0, "a fresh start settled");
    }

    #[test]
    fn a_restart_part_way_flies_in_from_256_again() {
        let mut fly_in = FlyIn::default();
        fly_in.start();
        steps(&mut fly_in, 3);
        fly_in.start();
        assert_eq!(fly_in.expand(), 196.0);
        assert_eq!(steps(&mut fly_in, 2), [196.0, 136.0]);
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
