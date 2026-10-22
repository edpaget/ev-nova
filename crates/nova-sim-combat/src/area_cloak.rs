//! Area cloaks: an escort in close formation with its cloaked leader
//! rides the leader's area cloak, as `_HasCloak`'s area branch has it.
//!
//! # The original
//!
//! In `EV Nova.app/Contents/MacOS/EV Nova`:
//!
//! - **`_HasAreaCloak`** (@0x3d49-0x3e24, [`CloakGear::area`]): a ship
//!   carries an area cloak when a cloak mod (`ModType` 17) of an item it
//!   holds has `ModVal` bit [`AREA_CLOAK`] (0x1000). The player's items are
//!   the outfits it owns (@0x3d85-0x3daa), an NPC's its ship type's
//!   default items (@0x3dc1-0x3e0f). No stock outfit has the bit.
//! - **`_HasCloak`** (@0x95f2-0x971b, [`has_cloak`]): a ship has a cloak
//!   when it [`rides`] its leader's area cloak, or else by a cloak of its
//!   own (@0x96c1-0x970b). An NPC rides when it has a leader (+0x9a, -1
//!   for none, @0x964c), keeps close formation with it (the stored
//!   sub-goal +0xc8ca == 0xc, `_IsShipInCloseForm` @0x82f54-0x82f69), the
//!   leader is cloaked (`_ShipIsCloaked` inlined at @0x9676-0x96b1, its 24
//!   test subsumed as [`Cloak::is_cloaked`](crate::cloak::Cloak::is_cloaked)
//!   does), and the leader carries an area cloak (@0x96b8). The branch tests
//!   no AI type. The player (@0x95fd-0x9647) has no leader, so it never
//!   rides: a player's area cloak covers its escorts only.
//!
//! `_HasCloak` has two callers. `_ShipVisibleToShip` (@0x9788-0x97a2) lets
//! a cloaked leader be seen by its escort that has a cloak, so an escort in
//! close formation sees its area-cloaked leader, the player included
//! (`nova_sim_ai::ai::sight`). `_AIHandleCloaking` (@0x833f6) uncloaks a
//! ship with no cloak (@0x833f3-0x8341d), so an escort riding an area cloak
//! runs the cloak think, whose parent rule (@0x83633-0x8366d) cloaks it
//! ([`cloak::think`](crate::cloak::think)).
//!
//! # The hold
//!
//! `_HandleShip`'s frame hold (@0x3413d-0x3415f, [`holds`]) uncloaks a
//! cloaked ship that can't cloak by `_ShipCanCloak`, which reads only the
//! ship's own items. So, by the engine's reading of
//! [`RuleKey::AreaCloakHold`](crate::RuleKey::AreaCloakHold), an escort
//! riding an area cloak with no cloak of its own is uncloaked the frame it
//! reads cloaked, and never stays cloaked. By the Bible's ("ships in
//! formation with a ship carrying this cloaking device will also be
//! cloaked") the hold keeps it while it rides.
//!
//! # Which close formation each reader reads
//!
//! `_AIDispatch` runs the cloak think (@0x8fcaa) before the high level
//! (@0x901e5), which picks goal 0xa's sub-goal and stores it. So the think,
//! and sight (the high level's sight of its leader @0x8ed5f among it), read
//! the sub-goal the last frame's flight stored
//! (`nova_sim_ai::traffic::Npc::close_form`). `_DoPlayGameWork` runs
//! `_AIDispatch` (@0x4429d) before `_HandleShip` (@0x444ad), so the hold
//! reads the sub-goal this frame's flight picks. `_AIMakeShipJumpIn`
//! (@0x82ebb) writes the goal but not the sub-goal, so the stored one
//! survives a glide in.
//!
//! # Not modelled
//!
//! `_HasAreaCloak`'s player cache (`*0x3b5110`; the stale test
//! @0x3d5c-0x3d7a, the stores @0x3d85 and @0x3e16, reset by
//! `_ResetPlayerPrecalcedValues` @0xc443-0xc448): the engine works the flag
//! out fresh from the owned outfits ([`CloakGear::player`]), which reads the
//! same while every change to them reaches a reset before the next frame.
//! No stock outfit has the bit, so a stale cache can't show in stock play.
//! The observer nag-ship branch of `_ShipVisibleToShip` (@0x973a) is
//! recorded in `nova_sim_ai::ai::sight`: the engine has no nag ship.

#[cfg(doc)]
use crate::cloak::{AREA_CLOAK, CloakGear};
use crate::rulebook::RuleSource;

/// Whether an NPC rides its leader's area cloak (`_HasCloak`
/// @0x964c-0x96bf): it keeps close formation with the leader
/// (`close_form`), the leader is cloaked (`leader_cloaked`) and carries an
/// area cloak (`leader_area`). The caller passes `false` for all three
/// with no leader (@0x964c).
#[must_use]
pub fn rides(close_form: bool, leader_cloaked: bool, leader_area: bool) -> bool {
    close_form && leader_cloaked && leader_area
}

/// Whether a ship has a cloak, as `_HasCloak` (@0x95f2-0x971b) asks: it
/// [`rides`] its leader's area cloak, or else it has a cloak of its own
/// (`device`, @0x96c1-0x970b). The player's branch (@0x95fd-0x9647) is
/// `device` alone, as it has no leader.
#[must_use]
pub fn has_cloak(device: bool, rides: bool) -> bool {
    rides || device
}

/// Whether `_HandleShip`'s frame hold (@0x3413d-0x3415f) keeps a cloaked
/// ship that `can_cloak` by its own gear and `rides` an area cloak, under
/// `hold` ([`RuleKey::AreaCloakHold`](crate::RuleKey::AreaCloakHold)): by
/// the engine only a ship that can cloak itself, by the Bible a ship
/// riding too.
#[must_use]
pub fn holds(can_cloak: bool, rides: bool, hold: RuleSource) -> bool {
    match hold {
        RuleSource::Engine => can_cloak,
        RuleSource::Bible => can_cloak || rides,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_escort_rides_its_leaders_area_cloak_only_in_close_formation_with_the_leader_cloaked() {
        for close_form in [false, true] {
            for leader_cloaked in [false, true] {
                for leader_area in [false, true] {
                    assert_eq!(
                        rides(close_form, leader_cloaked, leader_area),
                        close_form && leader_cloaked && leader_area,
                        "{close_form} {leader_cloaked} {leader_area}"
                    );
                }
            }
        }
        assert!(rides(true, true, true));
        assert!(!rides(false, true, true));
        assert!(!rides(true, false, true));
        assert!(!rides(true, true, false));
    }

    #[test]
    fn a_ship_has_a_cloak_by_its_own_device_or_by_riding() {
        assert!(!has_cloak(false, false));
        assert!(has_cloak(true, false));
        assert!(has_cloak(false, true));
        assert!(has_cloak(true, true));
    }

    #[test]
    fn by_the_engine_the_hold_keeps_only_a_ship_that_can_cloak_itself() {
        assert!(holds(true, false, RuleSource::Engine));
        assert!(holds(true, true, RuleSource::Engine));
        assert!(!holds(false, true, RuleSource::Engine));
        assert!(!holds(false, false, RuleSource::Engine));
    }

    #[test]
    fn by_the_bible_the_hold_keeps_a_ship_riding_an_area_cloak() {
        assert!(holds(false, true, RuleSource::Bible));
        assert!(holds(true, false, RuleSource::Bible));
        assert!(holds(true, true, RuleSource::Bible));
        assert!(!holds(false, false, RuleSource::Bible));
    }
}
