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
