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
//! worded against, and relays them once: at the next frame's steps,
//! before their lines ([`FlightView::release_held_lines`]); or before a
//! drop of the hold or any other write to the slot
//! ([`FlightView::relay_held_lines`]), a line said between the frames
//! among them. The original reads a press on the next first pass, after
//! the held line was drawn, so the press's line stands over it, and a cut
//! or a clear acts on the line the held one left. A write the frame makes
//! after its own steps leaves them held, as the original made it before
//! the second pass raised them: the frame-rate readout (`_PlayGame`
//! @0x45bcd-0x45d57, between the first pass and the draw), the keys kept
//! through a jump, read on the pass that collapses it, and an arrival's
//! line, raised inside its pass's `_HandlePlayer` (@0x6bbb8-0x6c625).
//! An arrival on the second pass holds its line among them, and its
//! layout says it first and relays the rest after it, as the rest of the
//! original's pass raises them after the arrival.
//!
//! **The smoothing.** A double-time frame's target is the first pass's
//! state, reached at alpha 1, but its alpha spans two passes, as the
//! original's image moves two passes' distance a frame (`_PlayGame`
//! @0x45d81-0x45d87): [`FlightView::shown_alpha`]. The stars keep the raw
//! alpha, as their scroll is doubled over the step already. On the step a
//! press changes the mode the ship is drawn up to a pass off for a few
//! milliseconds; the original's own switch moves the picture on that frame.
//!
//! **The arrival waits for the frame** ([`BetweenPasses`]'s arrival). A
//! jump's warp ended on the second pass settles its arrival inside that
//! pass (`nova_sim::flight_frame::StepRun::arrival`), after the original
//! drew frame 97 between the passes (@0x45dff-0x45e82). So the frame
//! between shows the old system, and the hold carries the arrival. Its
//! frame is presented at the tick's end ([`FlightView::present_held_arrival`]);
//! the first catch-up after that, at the next tick's top or a press after
//! the draw, lays the new system out (`FlightView::arrive_held`), replays
//! the second pass in the new scene, its sounds heard already, and relays
//! the held lines after the arrival's line. Until then the catch-up leaves
//! the flight's events where they are. So the white of an arrival without
//! the fades shows on the next frame presented, as the original's white
//! follows frame 97's flush, and the frame after shows the new system
//! (`_RefreshSAT` @0x6c605). An arrival on the first pass lays out where
//! the steps' replay reaches it ([`FlightView::arrive_in_step`]), and its
//! frame between shows the new system. A second pass's sounds and white
//! fade do not wait, as `PlaySound` and `_FadeWhiteIn` (@0x68404) start
//! at once: the fight's events before the arrival are heard as the hold
//! is taken, in the system left, and drawn nowhere.
//!
//! **Its escort-menu block runs as the hold goes.** The second pass's
//! `_HandlePlayer` fades the escort menu a level and may redraw it
//! (`nova_sim::flight_frame::PassReport::escort_menu`); the frame between
//! the passes shows the menu after the first pass, and the second pass's
//! block runs, at the time the hold was taken, as the hold is let go or
//! dropped ([`FlightView::run_held_escort_menu`]), before the next
//! report's passes. It runs on a drop too, as it is the flight's state,
//! not an effect. It leaves a redraw a press between the frames owes (a
//! command, a group selected) to the next report's passes: the flight ran
//! the held pass before the press, so its rows don't carry what the press
//! changed.
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

use std::time::Duration;

use nova_sim::flight_frame::{FlightFrame, PassExit, PassReport, RaisedLine, StepRun};

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
    /// The flight's time as the hold was taken, which the second pass's
    /// escort-menu block runs at as the hold goes
    /// ([`FlightView::run_held_escort_menu`]).
    at: Duration,
    /// The jump's arrival the second pass settled, laid out at the first
    /// catch-up after the frame between is presented
    /// (`FlightView::arrive_held`; see the module docs).
    arrival: Option<PassExit>,
    /// Whether the frame between has been presented
    /// ([`FlightView::present_held_arrival`]).
    presented: bool,
}

impl BetweenPasses {
    /// What `run` holds for the frame after it, taken out of its report
    /// at the flight's time `at`: the frame between its passes, when it
    /// has one, and its second pass.
    pub(super) fn taken_from(run: &mut StepRun, at: Duration) -> Option<Self> {
        let frame = run.between_passes.take()?;
        let second = if run.passes.len() > 1 {
            run.passes.pop()
        } else {
            None
        };
        let arrival = run
            .arrival
            .clone()
            .filter(|exit| second.is_some() && exit.pass == 1);
        Some(Self {
            frame,
            second,
            heard: None,
            at,
            arrival,
            presented: false,
        })
    }

    /// Takes the second pass out of the hold: a rescue's on it, which
    /// waits with the rescue's screen (see the `rescue` module).
    pub(super) fn take_second(&mut self) -> Option<PassReport> {
        self.second.take()
    }

    /// Takes the arrival the second pass settled, once the frame between
    /// has been presented; `None` without one or while it waits for its
    /// frame.
    pub(super) fn take_presented_arrival(&mut self) -> Option<PassExit> {
        let presented = self.presented;
        self.arrival.take_if(|_| presented)
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

    /// Whether the hold carries the jump's arrival its second pass
    /// settled, laid out at the frame after (see the module docs).
    pub(super) fn holds_arrival(&self) -> bool {
        self.between_passes
            .as_ref()
            .is_some_and(|held| held.arrival.is_some())
    }

    /// Whether the hold carries an arrival whose frame between is not yet
    /// presented: the catch-up leaves the flight's events for it.
    pub(super) fn arrival_waits_for_its_frame(&self) -> bool {
        self.between_passes
            .as_ref()
            .is_some_and(|held| held.arrival.is_some() && !held.presented)
    }

    /// Marks the frame between as presented, as `Screen::tick` does at its
    /// end ([`FlightView::present_jump_effect`]): an arrival its second
    /// pass settled is laid out at the next catch-up.
    pub(super) fn present_held_arrival(&mut self) {
        if let Some(held) = &mut self.between_passes {
            held.presented = true;
        }
    }

    /// Whether a double-time frame is drawn between its passes (see the
    /// module docs).
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
        let Some(held) = self.between_passes.take() else {
            return;
        };
        self.replay_held(held, anchor, false);
    }

    /// Replays `held`'s second pass, if it has one, the ship drawn at
    /// `anchor`: its wrap first, its effects with its breakups as they
    /// were heard, then its escort-menu block at the time the hold was
    /// taken ([`FlightView::run_held_escort_menu`]). On the pass a jump's
    /// `arrival` settled in, its explosions step once more in the new
    /// scene before the menu's block
    /// ([`FlightView::step_arrival_explosions`]). Both a release
    /// ([`FlightView::release_between_passes`]) and a held arrival
    /// (`FlightView::arrive_held`) replay by it.
    pub(super) fn replay_held(&mut self, held: BetweenPasses, anchor: Anchor, arrival: bool) {
        let Some(pass) = held.second else {
            return;
        };
        if let Some(wrap) = pass.wrap {
            self.carry_wrap(&wrap);
        }
        self.step_effects(&pass, None, anchor, held.heard);
        if arrival {
            self.step_arrival_explosions(&pass);
        }
        self.run_held_escort_menu(held.at, pass.escort_menu);
    }

    /// Drops the hold unreplayed, where the ship is seated afresh, its
    /// lines relayed first (see the module docs). Its second pass's
    /// escort-menu block still runs, as it is the flight's state, not an
    /// effect ([`FlightView::run_held_escort_menu`]).
    pub(super) fn drop_between_passes(&mut self) {
        self.relay_held_lines();
        if let Some(held) = self.between_passes.take()
            && let Some(pass) = held.second
        {
            self.run_held_escort_menu(held.at, pass.escort_menu);
        }
    }

    /// Runs the escort-menu block of a held second pass, `menu`, at `at`,
    /// the time the hold was taken (see the module docs): its fade's drop
    /// and its redraws ([`EscortMenu::run_held_pass`](crate::flight::escorts::EscortMenu::run_held_pass)),
    /// before the next report's passes, beeping as the fade's end beeps.
    /// A redraw a press owes is left for the next report's passes, as the
    /// held pass's rows predate the press.
    pub(super) fn run_held_escort_menu(
        &mut self,
        at: Duration,
        menu: Option<nova_sim::flight_frame::EscortMenuPass>,
    ) {
        if self.escort_menu.run_held_pass(at, menu) {
            self.shut_escort_menu_by_key();
        }
    }

    /// Words the lines `lines` a second pass held, against the kept frame
    /// and scene ([`FlightView::said_of`], an arrival's lead kept for its
    /// arrival), for the frame after ([`FlightView::held_lines`](FlightView#structfield.held_lines)).
    pub(super) fn hold_lines(&mut self, lines: Vec<RaisedLine>) {
        let said: Said = lines
            .into_iter()
            .filter_map(|line| self.said_of(line))
            .collect();
        self.held_lines.extend(said);
    }

    /// Relays the lines a second pass held before a write to the slot
    /// (see the module docs): a drop of the hold, or any write but the
    /// frame's own relays, so a line said between the frames, which the
    /// original's next first pass reads after the held one, stands over
    /// it. A write the frame makes after its steps
    /// ([`FlightView::writing_in_frame`](FlightView#structfield.writing_in_frame))
    /// leaves them held: the original said it before the second pass
    /// raised them, so they stand over it on the frame after.
    pub(super) fn relay_held_lines(&mut self) {
        if !self.writing_in_frame {
            self.release_held_lines();
        }
    }

    /// Relays the lines a second pass held, in the order raised, once,
    /// whatever writes the frame: at the next frame's steps, before their
    /// lines (see the module docs).
    pub(super) fn release_held_lines(&mut self) {
        for (text, ticks) in std::mem::take(&mut self.held_lines) {
            self.relay(text, ticks);
        }
    }

    /// Makes the sounds of `held`'s second pass now, as the original's
    /// `PlaySound` comes straight after the frame's draw, and keeps its
    /// breakups on the hold for the release to set off (see the module
    /// docs). Each sound is heard from where the pass left the player.
    pub(super) fn hear_held_pass(&mut self, held: &mut BetweenPasses) {
        if let Some(arrival) = &held.arrival {
            // The fight before the arrival, in the system it left.
            let scene = self.presentation.scene(held.frame.player().position);
            for event in &arrival.before {
                self.effects.hear_event(event, &scene, &self.looks);
            }
        }
        let Some(pass) = &held.second else {
            return;
        };
        let dying = dying(
            &pass.ships,
            (&self.sheet, self.outline.as_ref()),
            &self.npc_sheets,
            &self.npc_outlines,
        );
        let scene = self.presentation.scene(pass.player_position);
        let chance = &mut self.effects_chance;
        let breakups = self.effects.hear_step(&dying, &scene, &self.looks, chance);
        for event in &pass.combat {
            self.effects.hear_event(event, &scene, &self.looks);
        }
        held.heard = Some(breakups);
    }
}
