//! Whether a stellar is dominated: the one rule every reader of the
//! original's dominated byte (stellar +0x46) goes through.
//!
//! The byte is set two ways. `_ResetPlayerRecord`, `_DoNewPilot` and
//! `_ResetGameData` set it from the `spöb`'s `Flags2` bit [`DOMINATED`]
//! (0x0020; @0x1dbdf, @0x192c6, @0x1de58), and every rebuild point sets it
//! again for an active such stellar (`_HandleStellarSystemVisibility`
//! @0x32ceb) and never clears it. The planet comm dialog's Demand Tribute
//! sets it at run time (`_DoPlanetCommDialog` @0x978fa) and its Release
//! clears it (@0x9740f); that state is the galaxy overlay's
//! (`nova_sim::story::GalaxyOverlay::set_dominated`), stamped on a
//! [`LandingSite`] as [`LandingSite::dominated_at_run_time`]. A stellar is
//! dominated when either says so: [`dominated_by`], and over a stamped
//! site [`dominated`].
//!
//! The readers, each of which reaches this rule:
//!
//! - the landing fee (`_HandlePlayerDockRequest` @0x669bf, @0x66dd5;
//!   `_HandlePlayerDockApproach` @0x64d92): [`landing::fee_due`](crate::landing::fee_due);
//! - the clearance (`_HandlePlayerDockRequest` @0x6792d):
//!   [`landing::cleared`](crate::landing::cleared);
//! - the dock approach's greeting (`_HandlePlayerDockApproach` @0x64845):
//!   [`clearance::approach`](crate::clearance::approach);
//! - the exchange's markup (`_DoTradeDialog` @0x5dc3a): `nova_sim`'s
//!   `Chart::markup`;
//! - mission availability's `AvailRecord` -32000 and -32001
//!   (`_IsMissionAvailable` @0x9b559, @0x9b578): `nova_sim::mission`;
//! - the stellar guns (`_IsThreatToStellar` @0x9f0c): `nova_sim`'s
//!   `Session::armed_stellars`;
//! - the radar's IFF (`_ColorCodeStellar` @0xaa8c), the nav brackets
//!   (`_HandleNavTargetSprites` @0x2fd58) and the galaxy map's system
//!   colour (`_ColorCodeSystem` @0xd82e): `nova_sim`'s session `radar`,
//!   `brackets` and `map` modules;
//! - the Legal Status string (`_DrawLegalStatusString` @0xd5e6) and the
//!   player info's daily tribute row (`_PlayerInfoDialogUpdate`
//!   @0x53547): `nova_sim`'s session `player_info` module;
//! - the daily tribute (`_DoStellarTimePassage` @0x40880) and the defence
//!   fleet's regrowth (`_IncrementGameTime` @0xb5f6): `nova_sim`'s session
//!   `dominated_stellars` module;
//! - the load's defence ships left (`_LoadPilotData` @0x75d76): `nova_sim`'s
//!   session `stellar_defence` module;
//! - the planet comm dialog (`_DoPlanetCommDialog`, `_PlanetCommFilter`,
//!   `_PlanetCommDialogUpdate`, `_DrawPlanetCommDialogButtons`):
//!   `nova_sim`'s session `planet_comm` module;
//! - the Recharge's free fill (`_DoPortDialog` @0x5fdb9): `nova_sim`'s
//!   session `recharge` module.

use crate::catalog::LandingSite;

/// The `spöb` `Flags2` bit of a stellar that is always dominated by the
/// player: `_ResetPlayerRecord` sets each stellar's dominated byte (+0x46)
/// from it (@0x1dbdb-0x1dbdf), and the exchange prices such a stellar at
/// [`Markup::Dominated`](crate::market::Markup::Dominated).
#[doc(alias = "ALWAYS_DOMINATED")]
pub const DOMINATED: u16 = 0x0020;

/// Whether a stellar of `Flags2` `flags2` is dominated, the player having
/// dominated it at run time exactly when `by_player`: the one rule (see
/// the module docs).
#[must_use]
pub const fn dominated_by(by_player: bool, flags2: u16) -> bool {
    by_player || flags2 & DOMINATED != 0
}

/// Whether `site` is dominated, read through [`dominated_by`] from its
/// stamped run-time domination and its `Flags2`.
#[must_use]
pub const fn dominated(site: &LandingSite) -> bool {
    dominated_by(site.dominated_at_run_time, site.flags2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_catalog::fixture::planet;

    #[test]
    fn a_stellar_is_dominated_by_the_player_or_by_its_flags() {
        assert!(!dominated_by(false, 0));
        assert!(dominated_by(true, 0), "dominated at run time");
        assert!(dominated_by(false, DOMINATED), "always dominated");
        assert!(dominated_by(true, DOMINATED));
        assert!(!dominated_by(false, !DOMINATED), "no other bit counts");
    }

    #[test]
    fn a_site_is_dominated_by_its_stamp_or_its_flags() {
        let site = planet(128, 0.0, 0.0);
        assert!(!dominated(&site));
        assert!(dominated(&LandingSite {
            dominated_at_run_time: true,
            ..site
        }));
        assert!(dominated(&LandingSite {
            flags2: DOMINATED,
            ..site
        }));
    }

    #[test]
    fn the_bit_is_0x0020() {
        assert_eq!(DOMINATED, 0x0020);
    }
}
