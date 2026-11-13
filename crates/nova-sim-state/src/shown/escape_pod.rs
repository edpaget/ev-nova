//! An escape pod adrift ([`Pod`]), from `nova_sim_ai::escape_pod`, which
//! re-exports it and keeps the pool of pods, their spawning and their
//! scooping.

use nova_sim_base::flight::moved;
use nova_sim_base::geometry::Vec2;
use nova_sim_base::speed_mult::SpeedMult;

/// An escape pod adrift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pod {
    /// Where it is, in pixels from the system's centre.
    pub position: Vec2,
    /// How far it moves a pass at full gain: the move is this × the
    /// frame's gain.
    pub velocity: Vec2,
    /// The ticks it has left, a float as the original keeps it (+0x14):
    /// each pass takes the frame's gain off it.
    pub life: f32,
    /// The frame of shän 895's sheet it shows.
    pub frame: u16,
    /// Whether its sprite is shown: from its spawn until a step leaves
    /// its life at 0 or less, or a scoop hides it, and never again.
    pub shown: bool,
}

impl Pod {
    /// One step at the frame's `gain`: it drifts by its velocity × the
    /// gain, and the gain comes off its life, each added in doubles
    /// (`_HandleEscapePods` @0x39e8d-0x39ee0, @0x39fcc-0x39fe5); then a
    /// life of 0 or less hides it (@0x3a02d-0x3a046). `pub` for
    /// `nova_sim_ai::escape_pod`'s `EscapePods`, which steps its pods.
    pub fn step(&mut self, gain: SpeedMult) {
        self.position = moved(self.position, self.velocity, gain);
        self.life = gain.add_times(self.life, -1.0);
        // `jbe`: a NaN life hides it too.
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !(self.life > 0.0) {
            self.shown = false;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// A pod shown at (10, 20), drifting (2, -1) a pass, with `life`.
    fn pod(life: f32) -> Pod {
        Pod {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(2.0, -1.0),
            life,
            frame: 3,
            shown: true,
        }
    }

    /// `pod` after one step at `gain`.
    fn stepped(mut pod: Pod, gain: SpeedMult) -> Pod {
        pod.step(gain);
        pod
    }

    #[test]
    fn a_pod_drifts_by_its_velocity_and_ages_a_tick_at_full_gain() {
        let pod = stepped(pod(10.0), SpeedMult::FULL);
        assert_eq!(pod.position, Vec2::new(12.0, 19.0));
        assert_eq!(pod.life, 9.0);
        assert!(pod.shown);
        assert_eq!(pod.frame, 3, "its frame is left");
        assert_eq!(pod.velocity, Vec2::new(2.0, -1.0));
    }

    #[test]
    fn at_half_gain_a_pod_drifts_and_ages_by_half() {
        let pod = stepped(pod(10.0), SpeedMult::new(0.5));
        assert_eq!(pod.position, Vec2::new(11.0, 19.5));
        assert_eq!(pod.life, 9.5);
        assert!(pod.shown);
        assert_eq!(stepped(self::pod(10.0), SpeedMult::ZERO), self::pod(10.0));
    }

    #[test]
    fn a_life_of_none_or_less_hides_it_and_it_stays_hidden() {
        assert!(stepped(pod(1.5), SpeedMult::FULL).shown, "0.5 left");
        assert!(!stepped(pod(1.0), SpeedMult::FULL).shown, "at 0");
        assert!(
            !stepped(pod(0.5), SpeedMult::new(0.5)).shown,
            "at 0, half gain"
        );
        assert!(!stepped(pod(0.25), SpeedMult::FULL).shown, "below 0");
        let hidden = Pod {
            shown: false,
            ..pod(50.0)
        };
        assert!(!stepped(hidden, SpeedMult::FULL).shown, "never shown again");
    }

    #[test]
    fn a_nan_life_hides_it() {
        let pod = stepped(pod(f32::NAN), SpeedMult::FULL);
        assert!(pod.life.is_nan());
        assert!(!pod.shown);
    }
}
