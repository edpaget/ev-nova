//! The flight view's choice of starfield ([`StarfieldKind`]), and the
//! layered sky it draws by
//! [`RuleKey::LayeredStarfield`](nova_sim::RuleKey::LayeredStarfield)'s
//! other reading ([`draw`]; see [`crate::flight::sky`]).
//!
//! The choice is made once, from the two keys' readings, when the view's
//! rules are read ([`Presentation`](super::stepping::Presentation)): the
//! layered sky's reading wins over whatever
//! [`RuleKey::FlightStarfield`](nova_sim::RuleKey::FlightStarfield) reads.
//! Under the layered sky the original's starfield is neither scattered
//! nor stepped, so the effects' chance draws nothing for it.
//!
//! The sky is seen from over the point the camera centres on. A player
//! wrap jumps that point by the wrap's shift, so the view keeps the
//! wraps' sum ([`SkyShift`]) and adds it back: the eye moves only by the
//! ship's own move, and the sky doesn't jump.
//!
//! Through a jump's warp the sky rushes ([`rush`]): its pose carries the
//! ship's move over the step, which the renderer streaks the stars by.
//! The brake and turn before the warp and the arrival after it don't
//! rush. Only the layered reading draws a sky, so the rush is the new
//! game's alone; the original's starfield is left as ported.

use nova_sim::flight_frame::{FlightFrame, ShipPlace};
use nova_sim::player_wrap::PlayerWrap;
use nova_sim::{RuleSource, Vec2};

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
    /// [`RuleKey::FlightStarfield`](nova_sim::RuleKey::FlightStarfield),
    /// `starfield`, and of
    /// [`RuleKey::LayeredStarfield`](nova_sim::RuleKey::LayeredStarfield),
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
/// the wraps taken back by `shift`, drawn at its anchor, rushing by
/// [`rush`] from `previous`: none in a system of negative murk.
pub(super) fn draw(
    list: &mut DrawList,
    frame: &FlightFrame,
    previous: ShipPlace,
    camera: &Camera,
    shift: SkyShift,
) {
    let eye = shift.eye(camera.center());
    let rush = rush(frame, previous);
    if let Some(sky) = sky_pose(frame.system, eye, camera.anchor(), rush, frame.system_murk) {
        list.sky(sky);
    }
}

/// How far the sky rushes in `frame`, the frame drawn, the player's place
/// a step ago `previous`: through the jump's warp
/// ([`FlightFrame::warping`]) the ship's move over the step, zero
/// otherwise, the brake and turn before it and the arrival after it
/// included.
///
/// The move is the whole step's, from `previous` to where `frame` has the
/// ship, the two places the ship is drawn between
/// ([`FlightView::shown_position`](super::FlightView::shown_position)):
/// on a double-time frame between its passes that is the held frame's
/// pass, the one drawn. A wrap carries `previous` with it and an arrival
/// resets it, so the rush is the ship's own move and never a wrap's
/// shift or the jump's leap. The warp moves the ship's position rather
/// than its velocity, so the velocity would miss it.
fn rush(frame: &FlightFrame, previous: ShipPlace) -> Point {
    if !frame.warping {
        return Point::new(0.0, 0.0);
    }
    let now = frame.player().place().position;
    Point::new(now.x - previous.position.x, now.y - previous.position.y)
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
