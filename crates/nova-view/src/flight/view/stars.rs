//! The flight view's starfield: the original's ([`Starfield`]) by the
//! engine's reading of
//! [`RuleKey::FlightStarfield`],
//! the port's procedural dots ([`starfield::draw`]) by the other.
//!
//! The view drives the original's as `_PlayGame` does:
//!
//! - **Scattering.** The stars are scattered, then placed once, as
//!   `_PrepareAllSprites` places them (@0x3a7e2), at the first catch-up
//!   after the flight is built, play is entered from the main screen
//!   (`_EnterGameFromMainScreen` @0x19f7c) or a new game begins
//!   (`_ResetGameData` @0x1df5e), and at each
//!   [`SessionEvent::StarfieldReset`]: a jump's
//!   arrival, a gate's exit and the escape pod's rescue. The negative
//!   murk's clears after the scattering (`_EnterGameFromMainScreen`
//!   @0x19f86-0x19fa5, the gates') are the scattering's own hiding. No
//!   landing or take-off touches them: the landing's clear
//!   (`_HandlePlayerDockRequest` @0x670b8-0x670c3) only repeats what the
//!   scattering into the system did, as murk does not change within it.
//! - **Each step** (`_PlayGame` @0x45b6a-0x45f31). After the first pass
//!   (`_DoPlayGameWork(1)`, whose `_HandleStars` places them), the stars
//!   are snapshot for the frame, then scrolled by the ship's move over the
//!   pass, doubled in double time (`_ScrollStarfield` @0x45d59-0x45d96);
//!   in double time the second pass (`_DoPlayGameWork(0)` @0x45f31) wraps
//!   and places them again, undrawn until the next step's snapshot.
//! - **Drawing.** First in the world, under the message line and the
//!   stellars; without the stars' sheet, none.
//!
//! The Parallax Starfield preference reads as on, the original's default
//! (`_LoadPrefs` @0xcecee-0xced11): the Preferences dialog shows it greyed
//! out. Honouring it is rdm:phase/play-feel/phase-12-parallax-starfield-preference.
//!
//! [`SessionEvent::StarfieldReset`]: nova_sim::SessionEvent::StarfieldReset

use nova_sim::{Chance, RuleKey, RuleSource, Session, Vec2};

use crate::DrawList;
use crate::flight::catalog::Looks;
use crate::flight::stars::{StarLook, Starfield, Surroundings};
use crate::system::camera::{Anchor, Camera, STATUS_BAR_WIDTH};
use crate::system::starfield;

/// The Parallax Starfield preference as the engine reads it: on, the
/// original's default (see the module docs).
pub(super) const PARALLAX_STARFIELD: bool = true;

/// Whether `session` draws the original's starfield, by its rules'
/// reading of [`RuleKey::FlightStarfield`], with the stars' sheet read
/// into `looks`: without it no star is drawn, so none is scattered or
/// stepped, and the effects' chance draws nothing for them.
pub(super) fn original(session: &Session, looks: &Looks) -> bool {
    session.rules().source_for(RuleKey::FlightStarfield) == RuleSource::Engine
        && looks.star_sheet().is_some()
}

/// Where `session`'s stars are handled in a view `view` logical units in
/// size drawn about `anchor`: the play area left of the status bar, the
/// player's ship, the anchor in whole pixels, the system's murk as loaded
/// and the stars' look from `looks`.
pub(super) fn surroundings(
    session: &Session,
    view: (f32, f32),
    anchor: Anchor,
    looks: &Looks,
) -> Surroundings {
    // A view's logical units: within an i32's range.
    #[allow(clippy::cast_possible_truncation)]
    let whole = |units: f32| units.trunc() as i32;
    let point = anchor.point(view);
    Surroundings {
        area: (whole(view.0 - STATUS_BAR_WIDTH), whole(view.1)),
        ship: session.player().position,
        anchor: (whole(point.x), whole(point.y)),
        murk: session.system_murk(),
        look: StarLook::of(looks.star_sheet()),
    }
}

/// Scatters `stars` about the ship and places them once, for the frame to
/// draw (see the module docs), drawing on `chance`.
pub(super) fn reset(stars: &mut Starfield, at: &Surroundings, chance: &mut dyn Chance) {
    stars.randomize(at, PARALLAX_STARFIELD, chance);
    stars.handle(at, chance);
    stars.snapshot();
}

/// Steps `stars` after a pass of `session`, in a view `view` logical
/// units in size drawn about `anchor`, the stars' look from `looks`, when
/// it draws the original's starfield (see the module docs): wraps and
/// places them, drawing on `chance`; and after the first pass, `first`
/// the ship's position before it, snapshots them for the frame and
/// scrolls them by the ship's move over the pass, twice over in double
/// time.
pub(super) fn step(
    stars: &mut Starfield,
    session: &Session,
    (view, anchor, looks): ((f32, f32), Anchor, &Looks),
    first: Option<Vec2>,
    chance: &mut dyn Chance,
) {
    if !original(session, looks) {
        return;
    }
    let at = surroundings(session, view, anchor, looks);
    stars.handle(&at, chance);
    if let Some(from) = first {
        stars.snapshot();
        let times = if session.double_time() { 2.0 } else { 1.0 };
        stars.scroll((from.x - at.ship.x) * times, (from.y - at.ship.y) * times);
    }
}

/// Draws the starfield by `source`'s reading of
/// [`RuleKey::FlightStarfield`]: `stars` from `looks`' sheet, `alpha` of
/// the way between their last two snapshots, by the engine's; the
/// procedural dots through `camera` by the other.
pub(super) fn draw(
    list: &mut DrawList,
    (stars, looks): (&Starfield, &Looks),
    source: RuleSource,
    camera: &Camera,
    alpha: f32,
) {
    match source {
        RuleSource::Engine => {
            if let Some(sheet) = looks.star_sheet() {
                stars.draw(list, sheet, alpha);
            }
        }
        RuleSource::Bible => starfield::draw(list, camera),
    }
}
