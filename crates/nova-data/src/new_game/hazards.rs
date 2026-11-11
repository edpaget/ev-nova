//! A system's `hazards`, mapped to the `sÿst` fields the engine reads.
//!
//! Two words have a field:
//!
//! - `nebula` sets `Murk` to [`NEBULA_MURK`]: the engine's fog, which dims
//!   the stars and the sky and fades the sprites toward the background by
//!   their distance (`nova_view::flight::stars`, `nova_view::flight::murk`).
//! - `sensor-dead` sets `Interference` to [`SENSOR_DEAD_INTERFERENCE`]:
//!   static on the radar (`nova_view::flight::radar`), and the jamming of
//!   homing shots, which the new game's unguided weapons never fire. The
//!   format's "radar shows only what is in visual range" has no engine
//!   field, so this is a partial reading.
//!
//! Both values are placeholders: the format has no word for a hazard's
//! strength. The two fields are independent, so a system naming both
//! gets both, and a word named twice counts once.
//!
//! The other four words, `radiation`, `ion-storm`, `solar-flares` and
//! `gravity-well`, are not read: no `sÿst` field carries them, and the
//! loader reports nothing for them. They stay readable on the loaded
//! `Galaxy`.

use nova_galaxy_format::model::Hazard;

/// The `Murk` a `nebula` sets, a placeholder: above the engine's low-murk
/// threshold (7), so it loads under both readings of `LowMurk`, and well
/// short of the most the stars dim to (33 and above).
pub const NEBULA_MURK: i16 = 20;

/// The `Interference` `sensor-dead` sets, a placeholder: the most, which
/// puts static on the radar every frame.
pub const SENSOR_DEAD_INTERFERENCE: i16 = 100;

/// What a system's hazards set on its `sÿst`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    /// The system's `Murk`.
    pub murk: i16,
    /// The system's `Interference`.
    pub interference: i16,
}

/// What `hazards` set on a system's `sÿst`; nothing for none.
///
/// The match over [`Hazard`] is exhaustive on purpose: a word the format
/// adds must be decided here, as a field or as "not read" with its
/// reason in the module docs, not dropped by a wildcard.
#[must_use]
pub fn effects(hazards: &[Hazard]) -> Effects {
    hazards
        .iter()
        .fold(Effects::default(), |effects, hazard| match hazard {
            Hazard::Nebula => Effects {
                murk: NEBULA_MURK,
                ..effects
            },
            Hazard::SensorDead => Effects {
                interference: SENSOR_DEAD_INTERFERENCE,
                ..effects
            },
            Hazard::Radiation | Hazard::IonStorm | Hazard::SolarFlares | Hazard::GravityWell => {
                effects
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_hazard_sets_nothing() {
        assert_eq!(effects(&[]), Effects::default());
    }

    #[test]
    fn a_nebula_sets_the_murk_alone() {
        assert_eq!(
            effects(&[Hazard::Nebula]),
            Effects {
                murk: NEBULA_MURK,
                interference: 0
            }
        );
    }

    #[test]
    fn sensor_dead_sets_the_interference_alone() {
        assert_eq!(
            effects(&[Hazard::SensorDead]),
            Effects {
                murk: 0,
                interference: SENSOR_DEAD_INTERFERENCE
            }
        );
    }

    #[test]
    fn both_set_both_in_either_order() {
        let both = Effects {
            murk: NEBULA_MURK,
            interference: SENSOR_DEAD_INTERFERENCE,
        };
        assert_eq!(effects(&[Hazard::Nebula, Hazard::SensorDead]), both);
        assert_eq!(effects(&[Hazard::SensorDead, Hazard::Nebula]), both);
    }

    #[test]
    fn a_word_named_twice_counts_once() {
        assert_eq!(
            effects(&[Hazard::Nebula, Hazard::Nebula]),
            effects(&[Hazard::Nebula])
        );
        assert_eq!(
            effects(&[Hazard::SensorDead, Hazard::SensorDead]),
            effects(&[Hazard::SensorDead])
        );
    }

    #[test]
    fn the_unread_words_set_nothing_beside_a_read_one() {
        for word in [
            Hazard::Radiation,
            Hazard::IonStorm,
            Hazard::SolarFlares,
            Hazard::GravityWell,
        ] {
            assert_eq!(effects(&[word]), Effects::default(), "{word:?}");
            assert_eq!(
                effects(&[word, Hazard::Nebula, word]),
                effects(&[Hazard::Nebula]),
                "{word:?} beside a nebula"
            );
        }
    }
}
