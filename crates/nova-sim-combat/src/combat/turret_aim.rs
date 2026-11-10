//! Where each ship's turrets last aimed ([`TurretAim`]): a readout for
//! the view, which draws a ship's turret outfits facing what they fire at
//! (`nova_view::flight::mounted_outfits`).
//!
//! - A turret (guidance 4) records the heading its shot is aimed along
//!   from its exit, before its inaccuracy: the fight's own lead angle
//!   ([`Pointing::whole_from`]), so a turret that leads its target faces
//!   where it leads.
//! - A turreted beam (3) records its bearing as it is launched, from its
//!   exit, the same way.
//! - No other guidance records one: a fixed gun faces the nose, a
//!   quadrant turret (7, 8) is not a turning mount in the new game, and
//!   point defence (9, 10) fires at missiles, which the new game maps
//!   none of. A turret that doesn't fire (a blind spot, no room) records
//!   nothing.
//!
//! The fight only records aims, in a side buffer it never reads
//! ([`Combat::take_turret_aims`](super::Combat::take_turret_aims)), drained
//! by the session after the fight as the stray steps are. Nothing in the
//! simulation reads them, and no [`CombatEvent`](super::CombatEvent) carries
//! them, so stock play and its event lists are unchanged. The original
//! keeps no turret facing; this has no counterpart there. It plays no
//! sound.

use crate::flight::ShipState;
use crate::geometry::Vec2;

use super::aim::Pointing;
use super::armament::Launch;
use super::weapon::Guidance;
use super::{Combat, ShipRef};

impl Combat {
    /// Records the aim `ship` fired `launch` along from `from` moved to
    /// its exit `at`, if its guidance records one ([`aim`]).
    pub(super) fn note_turret_aim(
        &mut self,
        ship: ShipRef,
        launch: &Launch,
        from: &ShipState,
        at: Vec2,
    ) {
        let leaving = ShipState {
            position: at,
            ..*from
        };
        self.turret_aims.extend(aim(
            ship,
            launch.weapon.guidance,
            &launch.pointing,
            &leaving,
        ));
    }
}

/// One ship's turret aim: the world heading, in degrees, its turret shot
/// or turreted beam was aimed along as it left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurretAim {
    /// The ship that fired.
    pub ship: ShipRef,
    /// The heading it was aimed along, in degrees, before its inaccuracy.
    pub heading: f32,
}

/// Whether a weapon of `guidance` records its aim (see the module docs).
#[must_use]
pub fn records(guidance: Guidance) -> bool {
    matches!(guidance, Guidance::Turret | Guidance::TurretBeam)
}

/// The aim `ship` records firing a weapon of `guidance` pointed as
/// `pointing` says from `leaving`, its exit: the pointing's whole degree
/// from there, before inaccuracy. None for a guidance that records none.
#[must_use]
pub fn aim(
    ship: ShipRef,
    guidance: Guidance,
    pointing: &Pointing,
    leaving: &ShipState,
) -> Option<TurretAim> {
    records(guidance).then(|| TurretAim {
        ship,
        heading: f32::from(pointing.whole_from(leaving)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_turret_and_a_turreted_beam_record_their_aim() {
        for raw in [4, 3] {
            assert!(records(Guidance::decode(raw)), "{raw}");
        }
        for raw in [-1, 0, 1, 5, 6, 7, 8, 9, 10] {
            assert!(!records(Guidance::decode(raw)), "{raw}");
        }
        assert!(!records(Guidance::FighterBay));
    }

    #[test]
    fn the_aim_is_the_pointings_whole_degree_from_the_exit() {
        let leaving = ShipState {
            position: Vec2::new(10.0, 0.0),
            ..ShipState::default()
        };
        let pointing = Pointing::At(Vec2::new(10.0, -50.0));
        let recorded = aim(ShipRef::Player, Guidance::Turret, &pointing, &leaving);
        assert_eq!(
            recorded,
            Some(TurretAim {
                ship: ShipRef::Player,
                heading: f32::from(pointing.whole_from(&leaving)),
            })
        );
        assert_eq!(
            aim(ShipRef::Player, Guidance::Unguided, &pointing, &leaving),
            None
        );
    }
}
