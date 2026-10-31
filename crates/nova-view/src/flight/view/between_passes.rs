//! A double-time frame, drawn between its two passes, as `_PlayGame` draws
//! it: `_DoPlayGameWork(1)` (@0x45bc1-0x45bc8), the draw
//! (`_SWProcessSpriteWorld` @0x45dff, `_SWAnimateSpriteWorld` @0x45e21,
//! `_QDFlushPortBuffer` @0x45e82), then `_DoPlayGameWork(0)` (@0x45f20-0x45f31).
//! Nothing draws the world between that second pass and the next
//! iteration's first, so what the second pass changes shows on the next
//! frame, or never if that frame's first pass replaces it.
//!
//! **Both passes still run in one call** (`nova_sim::Session::step`), before
//! the player's next input, and the report carries the flight as it stood
//! between them ([`StepRun::between_passes`]). The view keeps that frame
//! apart ([`BetweenPasses`]) and draws it ([`FlightView::drawn`]). The kept
//! frame ([`FlightView::frame`](FlightView#structfield.frame)) stays the
//! flight as it stands for every logic read, so a draw site missed would
//! show a pass's error, never a stale gate.
//!
//! **The hold's life.** It is taken from the frame's last step
//! ([`FlightView::step_frame`]) and let go, its second pass replayed, at the
//! next frame with a step or a stop ([`FlightView::release_between_passes`]),
//! so a tick with no step, the open map and a background pause go on
//! showing it. A command leaves it: what it changed shows on the next step,
//! as the original's key reads act on the next pass. The three places that
//! seat the ship afresh drop it unreplayed (`FlightView::lay_out`,
//! [`FlightView::place_ship`] and a Use As My Ship takeover,
//! `FlightView::assigned`). A hold is not drawn docked or once the hold
//! after the player's explosion has run out, as neither reaches a step to
//! let it go ([`shows_hold`]).
//!
//! **The view's own effects wait with it.** The last step's second pass is
//! not replayed that frame: its wrap and its effects, flashes, glows, shot
//! looks and stars run as the hold goes, before the next report's first
//! pass. So they never disagree with the frame shown, across a wrap too.
//! Earlier steps of a many-step frame replay in full: nothing draws them.
//! A second pass's quotes, comm and last line stay on their own frame; a
//! line the original draws a frame later is left out, as the phase body
//! says (rdm:phase/play-feel-6/phase-9-double-time-draws-between-its-passes).
//!
//! **The smoothing.** A double-time frame's target is the first pass's
//! state, reached at alpha 1, but its alpha spans two passes, as the
//! original's image moves two passes' distance a frame (`_PlayGame`
//! @0x45d81-0x45d87): [`FlightView::shown_alpha`]. The stars keep the raw
//! alpha, as their scroll is doubled over the step already. On the step a
//! press changes the mode the ship is drawn up to a pass off for a few
//! milliseconds; the original's own switch moves the picture on that frame.
//!
//! **The arrival waits for the frame** ([`FlightView::holding_between_passes`]):
//! a jump's fallback end on a second pass draws the frame with the warp-ups
//! halted and the warp-out playing (`_HandlePlayer` @0x6c62a-0x6c706,
//! @0x6bbb8-0x6bbd9), and the arrival comes on the next step, which stops
//! before it runs. An end on the first pass waits for nothing: the original
//! settles it inside that pass, before the draw, so the frame held already
//! says so and the ship arrives in the same tick, its layout dropping the
//! hold. A second pass's sounds and white fade do not wait, as
//! `PlaySound` and `_FadeWhiteIn` (@0x68404) start at once, so the jump's
//! effect reads the kept frame.
//!
//! **Ordering against the original.** A held pass's combat sounds are
//! raised as its effects step, so they play up to a step late, where the
//! original's `PlaySound` (`_HandleShot` @0x44385, `_HandleExplods`
//! @0x444e1) comes straight after the draw. That crosses a frame, so it is
//! filed with the rest of what this left out
//! (rdm:phase/play-feel-6-followups/phase-5-second-pass-lines-sounds-and-rescue).

use nova_sim::PersonQuote;
use nova_sim::flight_frame::{FlightFrame, PassReport, StepRun};

use crate::flight::sim_port::FlightSim;
use crate::system::camera::Anchor;

use super::FlightView;

/// What a double-time frame holds until the next step (see the module
/// docs).
#[derive(Clone, Debug)]
pub(super) struct BetweenPasses {
    /// The flight as it stood between the passes.
    frame: Box<FlightFrame>,
    /// The second pass, to replay as the hold goes.
    second: Option<PassReport>,
}

impl BetweenPasses {
    /// What `run` holds for the frame after it, taken out of its report:
    /// the frame between its passes, when it has one, and its second pass.
    pub(super) fn taken_from(run: &mut StepRun) -> Option<Self> {
        let frame = run.between_passes.take()?;
        let second = if run.passes.len() > 1 {
            run.passes.pop()
        } else {
            None
        };
        Some(Self { frame, second })
    }

    /// The quotes the held pass said, which stay on their own frame.
    pub(super) fn quotes(&self) -> &[PersonQuote] {
        self.second.as_ref().map_or(&[], |pass| &pass.quotes)
    }
}

/// What is drawn: `held`'s frame, or else `frame`, the kept one, as
/// [`shows_hold`] says. For a caller holding a field of the view mutably;
/// the others read [`FlightView::drawn`].
pub(super) fn drawn<'a>(
    held: Option<&'a BetweenPasses>,
    frame: &'a FlightFrame,
) -> &'a FlightFrame {
    held.filter(|_| shows_hold(frame))
        .map_or(frame, |held| &held.frame)
}

/// Whether a hold is drawn over the kept `frame`: not docked or with the
/// hold after the player's explosion run out, whose paths reach no step
/// to let it go.
fn shows_hold(frame: &FlightFrame) -> bool {
    !frame.docked() && !frame.hold_expired
}

impl<C, S: FlightSim> FlightView<C, S> {
    /// The frame to draw: everything `Screen::draw` reaches reads it, as
    /// do the HUD's snapshots and what the steps write into the scene.
    /// Logic reads the kept frame (see the module docs).
    pub(super) fn drawn(&self) -> &FlightFrame {
        drawn(self.between_passes.as_ref(), &self.frame)
    }

    /// Whether a double-time frame is drawn between its passes: an arrival
    /// that frame doesn't show yet waits for it to go (see the module
    /// docs).
    pub(super) fn holding_between_passes(&self) -> bool {
        self.between_passes.is_some() && shows_hold(&self.frame)
    }

    /// How far the display is from the place each ship had a step ago to
    /// the frame drawn. On a double-time frame, `2 × alpha − 1`: at alpha 1
    /// the first pass's state, at ½ where the step began, and at 0 twice
    /// its pass short of that state, about the last frame's, drawn
    /// extrapolated, so the ships run evenly over two passes a frame. The
    /// display's own alpha is [`FlightView::alpha`].
    pub(super) fn shown_alpha(&self) -> f32 {
        if self.holding_between_passes() {
            2.0f32.mul_add(self.alpha, -1.0)
        } else {
            self.alpha
        }
    }

    /// Lets the hold go, the ship drawn at `anchor`, and replays its second
    /// pass, its wrap first, as [`FlightView::step_frame`] replays any (see
    /// the module docs). Nothing held does nothing.
    pub(super) fn release_between_passes(&mut self, anchor: Anchor) {
        let held = self.between_passes.take();
        if let Some(pass) = held.and_then(|held| held.second) {
            if let Some(wrap) = pass.wrap {
                self.carry_wrap(&wrap);
            }
            self.step_effects(&pass, None, anchor);
        }
    }
}
