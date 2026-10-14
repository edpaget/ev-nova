//! A ship's top speed in flight: the original's `_ShipMaxSpeed`
//! (@0x36f2-0x386d), branch by branch, and where the engine has each one.
//!
//! 1. **The player** (+0x86 is 0, @0x3717-0x3721): the class's `Speed`
//!    / 100 plus `ModVal` / 100 for each `ModType` 8 outfit owned, at
//!    least 0, cached in 0x3b5058 (@0x3727-0x37b3). The engine has this
//!    in [`ShipStats`](crate::stats::ShipStats) (`MORE_SPEED`) and
//!    [`Handling`](crate::handling::Handling), which is never negative.
//!    The cache changes nothing the player can see. The player's branch
//!    skips steps 2-5.
//! 2. **An NPC of `shïp` `Flags` 0x0400** gets 0 at once (@0x37b5-0x37bf).
//!    Left out, to
//!    rdm:phase/system-traffic-fidelity/phase-17-npc-skill-and-speed-factors.
//! 3. **Any other NPC** starts from the class's bare `Speed`, without its
//!    outfits, × its skill rolled at spawn (`_RandomSkillLevel`
//!    @0x6922-0x6984; @0x37c5-0x37d2). Left out, to the same phase: the
//!    engine flies an NPC at its own handling, outfits counted.
//! 4. **× its government's `SkillMult` / 100** (@0x37cb-0x37ee). Left out,
//!    to the same phase.
//! 5. **× 0.333** (@0xdd0c8) while a tractor beam pulls the ship
//!    (+0xc8dc, @0x37f4-0x3815). Left out: the engine has no tractor
//!    beams. Owner: rdm:task/weapon-flags-follow-up.
//! 6. **× 2** for the shareware nag ship (+0xc8d0 0x3ff, @0x381c-0x382a).
//!    Left out: the nag is out of scope.
//! 7. **× 1.5** (@0xdd078) for the player, and for every ship whose
//!    leader (+0x9a) is the player, while the pilot does not play Strict
//!    Play (@0x382e-0x384f). Ported here as [`led_speed`], under
//!    [`RuleKey::NonStrictSpeed`]. The session flies the player at
//!    [`Session::top_speed`](crate::Session::top_speed), and traffic flies
//!    each ship the player leads at
//!    [`Npc::top_speed`](crate::traffic::npc::Npc::top_speed).
//! 8. **At least 0** (@0x3857): the engine's handling is never negative.
//!
//! Nothing in `_ShipMaxSpeed` plays a sound.

use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// `_ShipMaxSpeed`'s factor for a pilot without Strict Play (@0xdd078).
pub const NON_STRICT_SPEED: f32 = 1.5;

/// The factor on the top speed of the player and of every ship it leads:
/// [`NON_STRICT_SPEED`] under the engine's reading of
/// [`RuleKey::NonStrictSpeed`] for a pilot who does not play Strict Play,
/// and 1.0 otherwise.
#[must_use]
pub fn led_speed(rules: &Rulebook, strict_play: bool) -> f32 {
    if !strict_play && rules.source_for(RuleKey::NonStrictSpeed) == RuleSource::Engine {
        NON_STRICT_SPEED
    } else {
        1.0
    }
}

#[cfg(test)]
// The factors are exact constants, so an exact comparison is meant.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_pilot_without_strict_play_flies_half_as_fast_again_by_the_engine() {
        assert_eq!(led_speed(&Rulebook::default(), false), 1.5);
    }

    #[test]
    fn a_strict_play_pilot_flies_at_its_own_top_speed() {
        assert_eq!(led_speed(&Rulebook::default(), true), 1.0);
    }

    #[test]
    fn the_other_non_strict_speed_reading_gives_no_factor() {
        let rules = Rulebook::default().with_override(RuleKey::NonStrictSpeed, RuleSource::Bible);
        assert_eq!(led_speed(&rules, false), 1.0);
        assert_eq!(led_speed(&rules, true), 1.0);
    }
}
