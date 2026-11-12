//! The values the flight view's `FlightSim` port carries across the seam
//! between the view and the session, re-exported from
//! [`nova_sim_frame::flight_frame`], which defines them and documents
//! the seam, with the constructors that read combat's and the ai's state:
//! a shot's, a beam's, a cargo box's and an asteroid's records
//! ([`FrameShotOf`], [`FrameBeamOf`], [`FrameBoxOf`],
//! [`FrameAsteroidOf`]) and an NPC's fade at a gate ([`GateFadeOf`]).
//! Each is a `pub` trait implemented for the frame's type, so
//! `FrameShot::of(..)` and the rest read as they did once the trait is
//! named with a `use`.
//!
//! [`Session::fill_frame`](crate::Session::fill_frame) fills a frame,
//! [`Session::apply`](crate::Session::apply) answers a command and
//! [`Session::screen`](crate::Session::screen) a screen's ask.

pub use nova_sim_frame::flight_frame::*;

use crate::asteroid::Asteroid;
use crate::cargo_box::CargoBox;
use crate::combat::beam::Beam;
use crate::combat::projectile::Shot;
use crate::sprite_mask::RleSheet;
use crate::traffic::npc::Mode;

/// [`FrameShot`]'s constructor from a [`Shot`], combat's.
pub trait FrameShotOf {
    /// `shot`'s record, at the frame it shows on `sheet`, its weapon
    /// `Graphic`'s as the contact passes read it, if read.
    #[must_use]
    fn of(shot: &Shot, sheet: Option<&RleSheet>) -> Self;
}

impl FrameShotOf for FrameShot {
    fn of(shot: &Shot, sheet: Option<&RleSheet>) -> Self {
        Self {
            id: shot.id,
            weapon: shot.weapon.id,
            position: shot.position,
            velocity: shot.velocity,
            heading: shot.heading,
            life: shot.life,
            fade: shot.fade_shown,
            frame: sheet.and_then(|sheet| shot.frame_on(sheet)),
            layer: shot.layer,
        }
    }
}

/// [`FrameBeam`]'s constructor from a [`Beam`], combat's.
pub trait FrameBeamOf {
    /// `beam`'s record.
    #[must_use]
    fn of(beam: &Beam) -> Self;
}

impl FrameBeamOf for FrameBeam {
    fn of(beam: &Beam) -> Self {
        Self {
            firer: beam.firer,
            start: beam.start,
            end: beam.end,
            weapon: beam.weapon.id,
            fade: beam.fade,
        }
    }
}

/// [`FrameBox`]'s constructor from a [`CargoBox`], the ai's.
pub trait FrameBoxOf {
    /// `cargo_box`'s record.
    #[must_use]
    fn of(cargo_box: &CargoBox) -> Self;
}

impl FrameBoxOf for FrameBox {
    fn of(cargo_box: &CargoBox) -> Self {
        Self {
            position: cargo_box.position,
            velocity: cargo_box.velocity,
            frame: cargo_box.drawn_frame(),
            fade: cargo_box.fade(),
            graphic: cargo_box.graphic,
        }
    }
}

/// [`FrameAsteroid`]'s constructor from an [`Asteroid`], combat's.
pub trait FrameAsteroidOf {
    /// `asteroid`'s record.
    #[must_use]
    fn of(asteroid: &Asteroid) -> Self;
}

impl FrameAsteroidOf for FrameAsteroid {
    fn of(asteroid: &Asteroid) -> Self {
        Self {
            position: asteroid.position,
            velocity: asteroid.velocity,
            frame: asteroid.shown_frame(),
            kind: asteroid.kind,
        }
    }
}

/// [`GateFade`]'s constructor from an NPC's [`Mode`], the ai's.
pub trait GateFadeOf {
    /// The fade an NPC in `mode` shows.
    #[must_use]
    fn of(mode: &Mode) -> Self;
}

impl GateFadeOf for GateFade {
    fn of(mode: &Mode) -> Self {
        match *mode {
            Mode::Emerging { ticks_left } => Self::Emerging { ticks_left },
            Mode::EnteringGate { ticks_left, .. } => Self::Entering { ticks_left },
            _ => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::combat::ShipRef;

    #[test]
    fn an_npcs_gate_fade_follows_its_mode() {
        assert_eq!(
            GateFade::of(&Mode::Emerging { ticks_left: 7.0 }),
            GateFade::Emerging { ticks_left: 7.0 }
        );
        let entering = Mode::EnteringGate {
            gate: StellarId(130),
            ticks_left: 5.0,
        };
        assert_eq!(
            GateFade::of(&entering),
            GateFade::Entering { ticks_left: 5.0 }
        );
        assert_eq!(GateFade::of(&Mode::Flying), GateFade::None);
        assert!(GateFade::Entering { ticks_left: 0.0 }.at_gate());
        assert!(GateFade::Emerging { ticks_left: 0.0 }.at_gate());
        assert!(!GateFade::None.at_gate());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_shot_visited_at_half_gain_carries_its_life_and_the_fade_its_visit_read() {
        use crate::combat::projectile::Guide;
        let weapon = crate::combat::weapon::WeaponSpec {
            falloff: -6,
            lifetime: 100,
            ..crate::combat::weapon::WeaponSpec::new(&nova_sim_catalog::fixture::weapon(128))
        };
        let mut shot = Shot::launch(
            weapon,
            ShipRef::Player,
            ShipRef::Player,
            &crate::flight::ShipState::default(),
            0.0,
        );
        let guide = Guide {
            speed_mult: crate::SpeedMult::new(0.5),
            ..Guide::among(&[])
        };
        for _ in 0..2 {
            shot.step(&guide, &mut crate::NeverFires);
        }
        let carried = FrameShot::of(&shot, None);
        assert_eq!((carried.life, carried.fade), (shot.life, shot.fade_shown));
        assert_eq!(carried.fade, 3.0, "half of 6 read on the 2nd visit");
        assert_eq!(carried.life, 99.0);
    }
}
