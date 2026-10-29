//! [`FlightView`]'s death, over the [`FlightSim`] port: the hold after the
//! player's explosion, read from the frame
//! ([`FlightFrame::hold_expired`], [`FlightFrame::fate`]), the fade to
//! black and the end of play, sent as commands
//! ([`FlightCommand::BeginFade`], [`FlightCommand::EndPlay`]), and the
//! fades drawn over the view.
//!
//! [`FlightFrame::hold_expired`]: nova_sim::flight_frame::FlightFrame::hold_expired
//! [`FlightFrame::fate`]: nova_sim::flight_frame::FlightFrame::fate

use std::time::Duration;

use nova_sim::PlayerFate;
use nova_sim::flight_frame::FlightCommand;

use crate::DrawList;
use crate::fade::{DEATH_FADE_FOR, ScreenFade};
use crate::flight::sim_port::FlightSim;

use super::FlightView;

impl<C, S: FlightSim> FlightView<C, S> {
    /// Covers the view, widened or not, in the arrival's fade-in, then in
    /// the jump's fade-out, then in the death's fade to black, as far as
    /// each shows.
    pub(super) fn draw_fades(&self, list: &mut DrawList) {
        for effect in [&self.fade_in, &self.jump].into_iter().flatten() {
            effect.draw_fade(list, self.view);
        }
        if let Some(fade) = &self.death_fade {
            fade.draw(list, self.view);
        }
    }

    /// A tick's death, as `Screen::tick` runs it after the frame's
    /// catch-up: once the hold after the player's explosion has run out
    /// ([`FlightFrame::hold_expired`](nova_sim::flight_frame::FlightFrame::hold_expired),
    /// read from the kept frame, which the catch-up has just filled),
    /// flight steps no more and only the death's fade moves on, by `dt`
    /// ([`FlightView::advance_death_fade`]). Gives whether the hold had
    /// expired, so the tick ends there.
    pub(super) fn death_frame(&mut self, dt: Duration) -> bool {
        if !self.frame.hold_expired {
            return false;
        }
        self.advance_death_fade(dt);
        true
    }

    /// Whether any flight key is held down.
    #[must_use]
    pub fn holds_keys(&self) -> bool {
        !self.held.is_empty()
    }

    /// The `dësc` IDs a sabotaged player sees as it dies, as the kept
    /// frame holds them
    /// ([`FlightFrame::death_descs`](nova_sim::flight_frame::FlightFrame::death_descs)):
    /// the router asks once the hold has expired, after the step that
    /// expired it filled the frame. None for a flight that failed, whose
    /// frame is the default.
    #[must_use]
    pub fn sabotage_death_descs(&self) -> Vec<i16> {
        self.frame.death_descs.clone()
    }

    /// Begins the fade to black after the hold has expired, over
    /// [`DEATH_FADE_FOR`] ([`FlightCommand::BeginFade`]): the fade starts
    /// once the flight's fate is [`PlayerFate::Fading`]; before the
    /// expiry the flight refuses, and nothing starts.
    pub fn begin_fade(&mut self) {
        self.send(FlightCommand::BeginFade);
        if self.frame.fate == PlayerFate::Fading && self.death_fade.is_none() {
            self.death_fade = Some(ScreenFade::to_black(DEATH_FADE_FOR));
        }
    }

    /// Whether the death's fade is over and play has ended: the router
    /// returns to the main screen. Read from the frame filled afresh, as
    /// the router asks between frames.
    #[must_use]
    pub fn game_over(&self) -> bool {
        self.frame.fate == PlayerFate::GameOver
    }

    /// Moves the death's fade on by `dt`, and ends play once it is over
    /// ([`FlightCommand::EndPlay`]); nothing else moves.
    pub(super) fn advance_death_fade(&mut self, dt: Duration) {
        let Some(fade) = &mut self.death_fade else {
            return;
        };
        fade.advance(dt);
        if fade.done() {
            self.send(FlightCommand::EndPlay);
        }
    }
}
