//! The new pilot's first-flight tutorial: the original's `_newbieFlag`
//! (0x31696c, an `i16` never saved), and the hints, the take-off greeting
//! and the "No destination" refusal it gates.
//!
//! **The flag's writes.**
//! - Every flight start sets it to [`Tutorial::SEASONED`], 0x7fff
//!   (`_ResetPlayer` @0x1d7cf): a pilot opened or created, and the
//!   restarts. Here every [`Session`](crate::Session) starts so.
//! - A new pilot then gets [`Tutorial::NEW_PILOT`], −3 (`_DoNewPilot`
//!   @0x19254), here [`Session::begin`](crate::Session::begin). So only a
//!   pilot created in this run is a newbie; saved and reopened, it is not.
//! - A take-off from a pilot at 2 or below sets −1, and above 2 greets
//!   instead ([`Tutorial::take_off`], `_PlayerLandOnStellar`
//!   @0x635e8-0x63745).
//! - The jump's warp stage beginning, as the ship stops braking
//!   (`_HandlePlayer` @0x6b603), and the hyperspace arrival (@0x6c002) set
//!   [`Tutorial::SEASONED`] ([`Tutorial::jumped`]).
//! - The hint block moves it through −2, 0, 1 and 2 ([`Tutorial::check`]).
//!
//! A newbie is a pilot whose flag is 2 or below ([`Tutorial::is_newbie`],
//! @0x635ed, @0x6b969).
//!
//! **The hint block** (`_HandlePlayer` @0x7093d-0x70ea2) runs on flight
//! frames where the flag is 1 or below and the frame counter is a multiple
//! of [`HINT_FRAMES`] (signed, @0x7094d-0x70981), reading the ship's
//! squared distance from the system's centre (@0x7098f-0x709a7):
//! 1. At −3 (@0x709b2), the nearest stellar (`_FindNearestStellar`
//!    @0x709e8), if any, gives [`TutorialHint::Land`] (@0x70a1c-0x70b9a);
//!    then the flag is −2 either way (@0x70baa).
//! 2. At −1 (@0x70bb5), a nearest stellar gives [`TutorialHint::Hyperspace`]
//!    (@0x70c12-0x70d33); then the flag is 0 either way (@0x70d3e).
//! 3. Past [`FAR_ENOUGH_SQUARED`] (strictly), the flag 0 or below and the
//!    ship intact: [`TutorialHint::FarEnough`] (@0x70dab-0x70dea), and the
//!    flag is 1 (@0x70def).
//! 4. Otherwise, past [`NOTHING_OUT_HERE_SQUARED`], the flag 1 and the ship
//!    intact: [`TutorialHint::NothingOutHere`] (@0x70e59-0x70e98), and the
//!    flag is 2 (@0x70e9d).
//!
//! So steps 1 or 2 and then 3 can all run in one frame, and the −2 that
//! step 1 leaves passes step 3's test. Each hint beeps `snd ` 154 and shows
//! for 512 ticks; the session raises them as messages and sounds.
//!
//! The original shows each hint only when its `STR#` 2002 string is not
//! empty (@0x709c8, @0x709d1, @0x70bc5, @0x70d7b, @0x70e2d); the engine's
//! strings are constants, never empty, so that gate is left out.

use crate::catalog::StellarId;

/// How many frames apart the hint checks fall (@0x7094d-0x70981).
pub const HINT_FRAMES: i16 = 60;

/// The squared distance from the centre past which the ship is told it is
/// far enough to jump: 2,000,000 (`0xdd974`, about 1,414 px).
pub const FAR_ENOUGH_SQUARED: f32 = 2_000_000.0;

/// The squared distance from the centre past which the ship is told there
/// is nothing out here: 10,000,000 (`0xdd978`, about 3,162 px).
pub const NOTHING_OUT_HERE_SQUARED: f32 = 10_000_000.0;

/// A tutorial hint, in the order of the hint block's steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TutorialHint {
    /// Land on the nearest stellar (with the can-land flag): `STR#` 2002 #22 for a station
    /// (`spöb` `Flags` 0x10, @0x70a31) or #23, then #24 and #25 around the
    /// land key's name.
    Land {
        /// The stellar to land on.
        stellar: StellarId,
        /// Whether it is a station, docked at rather than landed on.
        station: bool,
    },
    /// Hyperspace to another system: `STR#` 2002 #26-#28 around the map
    /// and jump keys' names.
    Hyperspace,
    /// The ship is far enough out to jump: `STR#` 2002 #30.
    FarEnough,
    /// There is nothing to find this far out: `STR#` 2002 #31.
    NothingOutHere,
}

/// The original's `_newbieFlag`: how far through the first-flight tutorial
/// the pilot is (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tutorial(i16);

impl Tutorial {
    /// A pilot just created: −3 (`_DoNewPilot` @0x19254).
    pub const NEW_PILOT: Self = Self(-3);
    /// A pilot past the tutorial: 0x7fff (`_ResetPlayer` @0x1d7cf, the
    /// first jump @0x6b603, @0x6c002).
    pub const SEASONED: Self = Self(0x7fff);

    /// The flag's value.
    #[must_use]
    pub const fn flag(self) -> i16 {
        self.0
    }

    /// Whether the pilot is still a newbie: the flag 2 or below (@0x635ed,
    /// @0x6b969).
    #[must_use]
    pub const fn is_newbie(self) -> bool {
        self.0 <= 2
    }

    /// The hint block on `frame` (see the module docs), with the ship
    /// `intact` or not, `distance_squared` from the centre, and `nearest`
    /// finding the nearest stellar and whether it is a station:
    /// the hints it gives, in order.
    pub fn check(
        &mut self,
        frame: i16,
        intact: bool,
        distance_squared: f32,
        nearest: impl Fn() -> Option<(StellarId, bool)>,
    ) -> Vec<TutorialHint> {
        let mut hints = Vec::new();
        if self.0 > 1 || frame % HINT_FRAMES != 0 {
            return hints;
        }
        match self.0 {
            -3 => {
                if let Some((stellar, station)) = nearest() {
                    hints.push(TutorialHint::Land { stellar, station });
                }
                self.0 = -2;
            }
            -1 => {
                if nearest().is_some() {
                    hints.push(TutorialHint::Hyperspace);
                }
                self.0 = 0;
            }
            _ => {}
        }
        if distance_squared > FAR_ENOUGH_SQUARED && self.0 <= 0 && intact {
            hints.push(TutorialHint::FarEnough);
            self.0 = 1;
        } else if distance_squared > NOTHING_OUT_HERE_SQUARED && self.0 == 1 && intact {
            hints.push(TutorialHint::NothingOutHere);
            self.0 = 2;
        }
        hints
    }

    /// A take-off with no `Q` message: whether it greets, above 2; at 2 or
    /// below the flag becomes −1 instead, for the hyperspace hint
    /// (`_PlayerLandOnStellar` @0x635e8-0x63745).
    pub fn take_off(&mut self) -> bool {
        if self.0 > 2 {
            return true;
        }
        self.0 = -1;
        false
    }

    /// A jump began, or the ship arrived from one: the pilot is past the
    /// tutorial (@0x6b603, @0x6c002).
    pub fn jumped(&mut self) {
        *self = Self::SEASONED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARTH: StellarId = StellarId(128);

    /// Finds Earth, a planet.
    fn earth() -> impl Fn() -> Option<(StellarId, bool)> {
        || Some((EARTH, false))
    }

    fn none() -> Option<(StellarId, bool)> {
        None
    }

    const LAND: TutorialHint = TutorialHint::Land {
        stellar: EARTH,
        station: false,
    };

    #[test]
    fn take_off_greets_above_two_and_otherwise_sets_minus_one() {
        for flag in [3, Tutorial::SEASONED.flag()] {
            let mut tutorial = Tutorial(flag);
            assert!(tutorial.take_off(), "{flag}");
            assert_eq!(tutorial.flag(), flag, "{flag} stays put");
        }
        for flag in [-3, -2, 0, 2] {
            let mut tutorial = Tutorial(flag);
            assert!(!tutorial.take_off(), "{flag}");
            assert_eq!(tutorial.flag(), -1, "{flag}");
        }
    }

    #[test]
    fn a_newbie_is_two_or_below() {
        assert!(Tutorial::NEW_PILOT.is_newbie());
        assert!(Tutorial(2).is_newbie());
        assert!(!Tutorial(3).is_newbie());
        assert!(!Tutorial::SEASONED.is_newbie());
    }

    #[test]
    fn jumping_makes_the_pilot_seasoned() {
        let mut tutorial = Tutorial::NEW_PILOT;
        tutorial.jumped();
        assert_eq!(tutorial, Tutorial::SEASONED);
    }

    #[test]
    fn the_land_hint_comes_on_a_sixtieth_frame() {
        for frame in [59, -14, 61] {
            let mut tutorial = Tutorial::NEW_PILOT;
            assert_eq!(tutorial.check(frame, true, 0.0, earth()), [], "{frame}");
            assert_eq!(tutorial, Tutorial::NEW_PILOT, "{frame}");
        }
        for frame in [0, 60, -60] {
            let mut tutorial = Tutorial::NEW_PILOT;
            assert_eq!(tutorial.check(frame, true, 0.0, earth()), [LAND], "{frame}");
            assert_eq!(tutorial.flag(), -2, "{frame}");
        }
        let mut tutorial = Tutorial::NEW_PILOT;
        let station = || Some((EARTH, true));
        assert_eq!(
            tutorial.check(60, true, 0.0, station),
            [TutorialHint::Land {
                stellar: EARTH,
                station: true
            }]
        );
    }

    #[test]
    fn the_land_hint_moves_on_without_a_stellar() {
        let mut tutorial = Tutorial::NEW_PILOT;
        assert_eq!(tutorial.check(60, true, 0.0, none), []);
        assert_eq!(tutorial.flag(), -2);
        assert_eq!(tutorial.check(120, true, 0.0, earth()), [], "−2 waits");
        assert_eq!(tutorial.flag(), -2);
    }

    #[test]
    fn the_hyperspace_hint_follows_minus_one() {
        let mut tutorial = Tutorial(-1);
        assert_eq!(
            tutorial.check(60, true, 0.0, earth()),
            [TutorialHint::Hyperspace]
        );
        assert_eq!(tutorial.flag(), 0);
        let mut tutorial = Tutorial(-1);
        assert_eq!(tutorial.check(60, true, 0.0, none), []);
        assert_eq!(tutorial.flag(), 0, "moves on without a stellar");
    }

    #[test]
    fn far_enough_needs_over_two_million_squared_and_an_intact_ship() {
        for flag in [0, -2] {
            let mut tutorial = Tutorial(flag);
            assert_eq!(tutorial.check(60, true, 2_000_000.0, none), [], "{flag}");
            assert_eq!(tutorial.flag(), flag);
            assert_eq!(tutorial.check(60, false, 2_000_001.0, none), [], "{flag}");
            assert_eq!(tutorial.flag(), flag);
            assert_eq!(
                tutorial.check(60, true, 2_000_001.0, none),
                [TutorialHint::FarEnough],
                "{flag}"
            );
            assert_eq!(tutorial.flag(), 1);
        }
    }

    #[test]
    fn nothing_out_here_needs_stage_one_and_over_ten_million_squared() {
        let mut tutorial = Tutorial(1);
        assert_eq!(tutorial.check(60, true, 10_000_000.0, none), []);
        assert_eq!(tutorial.check(60, false, 10_000_001.0, none), []);
        assert_eq!(tutorial.flag(), 1);
        assert_eq!(
            tutorial.check(60, true, 10_000_001.0, none),
            [TutorialHint::NothingOutHere]
        );
        assert_eq!(tutorial.flag(), 2);
        let mut tutorial = Tutorial(0);
        assert_eq!(
            tutorial.check(60, true, 10_000_001.0, none),
            [TutorialHint::FarEnough],
            "never followed by #31 in one frame"
        );
        assert_eq!(tutorial.flag(), 1);
    }

    #[test]
    fn land_and_far_enough_in_one_frame() {
        let mut tutorial = Tutorial::NEW_PILOT;
        assert_eq!(
            tutorial.check(60, true, 2_000_001.0, earth()),
            [LAND, TutorialHint::FarEnough]
        );
        assert_eq!(tutorial.flag(), 1);
        let mut tutorial = Tutorial(-1);
        assert_eq!(
            tutorial.check(60, true, 2_000_001.0, earth()),
            [TutorialHint::Hyperspace, TutorialHint::FarEnough]
        );
        assert_eq!(tutorial.flag(), 1);
    }

    #[test]
    fn two_and_seasoned_check_nothing() {
        for flag in [2, Tutorial::SEASONED.flag()] {
            let mut tutorial = Tutorial(flag);
            assert_eq!(
                tutorial.check(60, true, 10_000_001.0, earth()),
                [],
                "{flag}"
            );
            assert_eq!(tutorial.flag(), flag);
        }
    }
}
