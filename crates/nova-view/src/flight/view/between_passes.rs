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
//! seat the ship afresh drop it unreplayed ([`FlightView::drop_between_passes`]
//! from `FlightView::lay_out`, [`FlightView::place_ship`] and a Use As My
//! Ship takeover, `FlightView::assigned`). A hold is not drawn docked or once the hold
//! after the player's explosion has run out, as neither reaches a step to
//! let it go ([`shows_hold`]).
//!
//! **The view's own effects wait with it.** The last step's second pass is
//! not replayed that frame: its wrap and its effects, flashes, glows, shot
//! looks and stars run as the hold goes, before the next report's first
//! pass. Its sounds are the exception (see below). So they never disagree
//! with the frame shown, across a wrap too. Earlier steps of a many-step
//! frame replay in full: nothing draws them.
//!
//! **Its lines wait with it.** `_DisplayComm` fills the slot at once but
//! draws its line into the background and work frames (@0x90294-0x90390),
//! which reach the screen at the next frame's draw (`_FloaterDrawCallback`
//! @0x38b52-0x38b90). So a line the second pass raised shows on the frame
//! after, unless that frame's first pass raises one of its own. The
//! session holds them apart (`StepRun::second_pass_lines`), the frame's own
//! last line being the first pass's. The hold words them as it is taken
//! ([`FlightView::hold_lines`]), against the frame its own messages are
//! worded against, and relays them once ([`FlightView::relay_held_lines`]):
//! at the release, before the next report's lines; at a drop; or before
//! any other write to the slot, a line said between the frames among
//! them. The original reads a press on the next first pass, after the
//! held line was drawn, so the press's line stands over it, and a cut or
//! a clear acts on the line the held one left.
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
//! before it runs. That arrival runs the tick's steps the stop left unrun
//! in the new system, so the new system's first frame is after its first
//! pass there, as the original's frame 98 is (@0x45bc8), held between its
//! passes in double time. An end on the first pass waits for nothing: the original
//! settles it inside that pass, before the draw, so the frame held already
//! says so and the ship arrives in the same tick, its layout dropping the
//! hold. A second pass's sounds and white fade do not wait, as
//! `PlaySound` and `_FadeWhiteIn` (@0x68404) start at once, so the jump's
//! effect reads the kept frame.
//!
//! **A held pass is heard on its own frame.** Its combat sounds are made as
//! the hold is taken ([`FlightView::hear_held_pass`]), because the
//! original's `PlaySound` comes straight after the draw (`_HandleShot`
//! @0x44385, `_HandleShip` @0x44483, `_HandleShipDisplay` @0x444ad,
//! `_CreateExplosion` @0x3f91a, all on both passes). That covers the
//! looped sounds' countdown, the breakup rolls and each event's sound.
//! Its sprites, the breakups among them, are set off at the release, with
//! the rest of its effects (see the `effects` module's "Heard, then
//! shown"). A hold dropped unreplayed drops its breakups, and its sounds
//! have played, as the original's have.

use nova_sim::flight_frame::{FlightFrame, PassReport, RaisedLine, StepRun};

use crate::flight::effects::Breakups;
use crate::flight::sim_port::FlightSim;
use crate::system::camera::Anchor;

use super::{FlightView, Said, dying};

/// What a double-time frame holds until the next step (see the module
/// docs).
#[derive(Clone, Debug)]
pub(super) struct BetweenPasses {
    /// The flight as it stood between the passes.
    frame: Box<FlightFrame>,
    /// The second pass, to replay as the hold goes.
    second: Option<PassReport>,
    /// The second pass's breakups, once its sounds are heard
    /// ([`FlightView::hear_held_pass`]), to set off as the hold goes.
    heard: Option<Breakups>,
    /// The lines the second pass raised ([`StepRun::second_pass_lines`]),
    /// worded as the hold was taken, to relay as it goes or before the
    /// slot is next written ([`FlightView::relay_held_lines`]).
    said: Said,
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
        Some(Self {
            frame,
            second,
            heard: None,
            said: Said::new(),
        })
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
        self.relay_held_lines();
        let Some(held) = self.between_passes.take() else {
            return;
        };
        if let Some(pass) = held.second {
            if let Some(wrap) = pass.wrap {
                self.carry_wrap(&wrap);
            }
            self.step_effects(&pass, None, anchor, held.heard);
        }
    }

    /// Drops the hold unreplayed, where the ship is seated afresh, its
    /// lines relayed first (see the module docs).
    pub(super) fn drop_between_passes(&mut self) {
        self.relay_held_lines();
        self.between_passes = None;
    }

    /// Words the lines `lines` a second pass held, against the kept frame
    /// and scene ([`FlightView::said_of`], an arrival's lead kept for its
    /// arrival), for the hold to relay on the frame after. With no hold to
    /// keep them they are relayed at once.
    pub(super) fn hold_lines(&mut self, lines: Vec<RaisedLine>) {
        let said: Said = lines
            .into_iter()
            .filter_map(|line| self.said_of(line))
            .collect();
        match &mut self.between_passes {
            Some(held) => held.said.extend(said),
            None => {
                for (text, ticks) in said {
                    self.relay(text, ticks);
                }
            }
        }
    }

    /// Relays the lines the hold keeps, in the order raised, once: at its
    /// release, at its drop, and before any write to the slot but the
    /// frame's own relays, so a line said between the frames, which the
    /// original's next first pass reads after the held one, stands over
    /// it (see the module docs).
    pub(super) fn relay_held_lines(&mut self) {
        let Some(held) = &mut self.between_passes else {
            return;
        };
        let said = std::mem::take(&mut held.said);
        for (text, ticks) in said {
            self.relay(text, ticks);
        }
    }

    /// Makes the sounds of `held`'s second pass now, as the original's
    /// `PlaySound` comes straight after the frame's draw, and keeps its
    /// breakups on the hold for the release to set off (see the module
    /// docs). Each sound is heard from where the pass left the player.
    pub(super) fn hear_held_pass(&mut self, held: &mut BetweenPasses) {
        let Some(pass) = &held.second else {
            return;
        };
        let dying = dying(&pass.ships, &self.sheet, &self.npc_sheets);
        let scene = self.presentation.scene(pass.player_position);
        let chance = &mut self.effects_chance;
        let breakups = self.effects.hear_step(&dying, &scene, &self.looks, chance);
        for event in &pass.combat {
            self.effects.hear_event(event, &scene, &self.looks);
        }
        held.heard = Some(breakups);
    }
}
