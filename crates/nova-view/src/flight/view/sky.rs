//! The flight view's choice of starfield ([`StarfieldKind`]), and the
//! layered sky it draws by
//! [`RuleKey::LayeredStarfield`](nova_sim_frame::RuleKey::LayeredStarfield)'s
//! other reading ([`draw`]; see [`crate::flight::sky`]).
//!
//! The choice is made once, from the two keys' readings, when the view's
//! rules are read ([`Presentation`](super::stepping::Presentation)): the
//! layered sky's reading wins over whatever
//! [`RuleKey::FlightStarfield`](nova_sim_frame::RuleKey::FlightStarfield) reads.
//! Under the layered sky the original's starfield is neither scattered
//! nor stepped, so the effects' chance draws nothing for it.
//!
//! The sky is seen from over the point the camera centres on. A player
//! wrap jumps that point by the wrap's shift, so the view keeps the
//! wraps' sum ([`SkyShift`]) and adds it back: the eye moves only by the
//! ship's own move, and the sky doesn't jump.

use nova_sim_frame::flight_frame::FlightFrame;
use nova_sim_frame::player_wrap::PlayerWrap;
use nova_sim_frame::{RuleSource, Vec2};

use crate::DrawList;
use crate::flight::sky::sky_pose;
use crate::geometry::Point;
use crate::system::camera::Camera;

/// Which starfield flight draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StarfieldKind {
    /// The original's ([`crate::flight::stars`]).
    Original,
    /// The port's procedural dots ([`crate::system::starfield`]).
    Procedural,
    /// The layered sky ([`crate::flight::sky`]).
    Layered,
}

impl StarfieldKind {
    /// The starfield the readings of
    /// [`RuleKey::FlightStarfield`](nova_sim_frame::RuleKey::FlightStarfield),
    /// `starfield`, and of
    /// [`RuleKey::LayeredStarfield`](nova_sim_frame::RuleKey::LayeredStarfield),
    /// `layered`, pick: the layered sky by the latter's other reading,
    /// whatever the former reads; otherwise the former's.
    pub(super) fn of(starfield: RuleSource, layered: RuleSource) -> Self {
        match (layered, starfield) {
            (RuleSource::Bible, _) => Self::Layered,
            (RuleSource::Engine, RuleSource::Engine) => Self::Original,
            (RuleSource::Engine, RuleSource::Bible) => Self::Procedural,
        }
    }

    /// The reading of `flight_starfield` the stars are handled by (see the
    /// `stars` module), or `None` for the layered sky, which handles none.
    pub(super) fn stars(self) -> Option<RuleSource> {
        match self {
            Self::Original => Some(RuleSource::Engine),
            Self::Procedural => Some(RuleSource::Bible),
            Self::Layered => None,
        }
    }
}

/// What the player's wraps have shifted the camera's centre by, taken
/// back: added to the centre, it gives an eye that moves only with the
/// ship.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct SkyShift(Vec2);

impl SkyShift {
    /// Takes `wrap`'s shift back, as the view carries it.
    pub(super) fn carry(&mut self, wrap: &PlayerWrap) {
        self.0 = Vec2::new(self.0.x - wrap.delta.x, self.0.y - wrap.delta.y);
    }

    /// The eye over `centre`, the camera's centre.
    pub(super) fn eye(self, centre: Point) -> Point {
        Point::new(centre.x + self.0.x, centre.y + self.0.y)
    }
}

/// Draws `frame`'s system's sky, seen from over `camera`'s centre with
/// the wraps taken back by `shift`, drawn at its anchor: none in a system
/// of negative murk.
pub(super) fn draw(list: &mut DrawList, frame: &FlightFrame, camera: &Camera, shift: SkyShift) {
    let eye = shift.eye(camera.center());
    if let Some(sky) = sky_pose(frame.system, eye, camera.anchor(), frame.system_murk) {
        list.sky(sky);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layered_skys_reading_wins_and_otherwise_the_starfields_decides() {
        use RuleSource::{Bible, Engine};
        assert_eq!(StarfieldKind::of(Engine, Engine), StarfieldKind::Original);
        assert_eq!(StarfieldKind::of(Bible, Engine), StarfieldKind::Procedural);
        assert_eq!(StarfieldKind::of(Engine, Bible), StarfieldKind::Layered);
        assert_eq!(StarfieldKind::of(Bible, Bible), StarfieldKind::Layered);
    }

    #[test]
    fn only_the_original_and_procedural_handle_stars_by_their_reading() {
        assert_eq!(StarfieldKind::Original.stars(), Some(RuleSource::Engine));
        assert_eq!(StarfieldKind::Procedural.stars(), Some(RuleSource::Bible));
        assert_eq!(StarfieldKind::Layered.stars(), None);
    }

    #[test]
    fn a_carried_wrap_is_taken_back_from_the_eye() {
        let wrap = PlayerWrap {
            from: Vec2::new(0.0, 0.0),
            delta: Vec2::new(-100.0, 40.0),
        };
        let mut shift = SkyShift::default();
        assert_eq!(shift.eye(Point::new(5.0, 6.0)), Point::new(5.0, 6.0));
        shift.carry(&wrap);
        assert_eq!(shift.eye(Point::new(5.0, 6.0)), Point::new(105.0, -34.0));
        shift.carry(&wrap);
        assert_eq!(shift.eye(Point::new(0.0, 0.0)), Point::new(200.0, -80.0));
    }
}
