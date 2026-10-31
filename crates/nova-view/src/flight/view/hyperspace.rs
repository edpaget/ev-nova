//! [`FlightView`]'s jump and arrival, over the [`FlightSim`] port: J's
//! frame and the jump it sends ([`FlightCommand::Jump`]), the jump's
//! effect read from the frame (the jump under way, the warp, the
//! `_hyperGamma` and the warp's end), and the arrival it sends
//! ([`FlightCommand::Arrive`]) and catches up with, its pending
//! explosions from the answer ([`Answer::Arrived`]). The arrival's words
//! name what the flight names ([`FlightView::word`], the date
//! [`FlightFrame::long_date`](nova_sim::flight_frame::FlightFrame::long_date)).

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::{CombatEvent, Condition, SimMessage};

use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::jump::JumpEffect;
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::{
    FAR_ENOUGH, FlightView, NOTHING_OUT_HERE, TOO_CLOSE, TRACTORED, arrival_lead,
    arrival_message_with, jump_refusal_message, jump_refusal_ticks,
};

impl<C, S: FlightSim> FlightView<C, S> {
    /// Begins a jump ([`FlightCommand::Jump`]), or shows why not when the
    /// player is told ([`Answer::JumpRefused`]'s `told`,
    /// `nova_sim::Session::jump_refusal_told`); the refusal's beep is the
    /// flight's, drained with its sounds. Starting it clears the message
    /// line only when it says [`TOO_CLOSE`], [`TRACTORED`], [`FAR_ENOUGH`]
    /// or [`NOTHING_OUT_HERE`] (`_HandlePlayer` @0x6b231-0x6b3d9, #36 at
    /// @0x6b2ac-0x6b30d). The original's second jump path clears them
    /// again (@0x6d601-0x6d7b1), once the pilot is past the tutorial,
    /// which the arrival line replaces anyway. The keys held stay held:
    /// the flight flies the jump without them, and a steering, thrust or
    /// fire key still held at arrival acts then, as the original reads
    /// those live. J held through the jump does nothing after it: its
    /// latch is set on every frame of the jump ([`FlightView::jump_frame`]).
    fn jump(&mut self) {
        self.settle_given_up_jump();
        match self.send_gated(FlightCommand::Jump) {
            Answer::Jumped(_) => {
                for text in [TOO_CLOSE, TRACTORED, FAR_ENOUGH, NOTHING_OUT_HERE] {
                    self.clear_message(text);
                }
                self.start_jump_effect();
            }
            Answer::JumpRefused {
                refusal,
                told: true,
            } => {
                if let Some(text) = jump_refusal_message(&refusal) {
                    self.say_for(text, jump_refusal_ticks(&refusal));
                }
            }
            _ => {}
        }
    }

    /// One frame of the jump key, with J down or not, as `_HandlePlayer`
    /// runs it (@0x6b0da-0x6badf) on [`FlightView::jump_latch`]: during a
    /// jump, the latch set (@0x6bad3), so J held through a jump does
    /// nothing after it; with J down and the player intact, a jump begun
    /// or refused when the latch is clear (@0x6b86d, @0x6b955), and the
    /// latch set; otherwise, J up or the player disabled or breaking up
    /// (`_IsDisabled`), the latch cleared (@0x6badf). So J held into a
    /// disable, or first pressed while disabled, acts on the first frame
    /// the player is intact again. The jump under way and the player's
    /// condition are read from one frame filled afresh. While the flight
    /// refuses the jump's gate ([`FlightView::takes_command`]) the frame
    /// does nothing, the latch left as it was. A flight that failed
    /// refuses the jump ([`FlightView::jump`]).
    pub(super) fn jump_frame(&mut self, down: bool) {
        if !self.takes_command(&FlightCommand::Jump) {
            return;
        }
        if self.frame.jump_under_way {
            self.jump_latch = true;
            return;
        }
        let intact = self.frame.player().condition == Condition::Intact;
        if down && intact {
            if !self.jump_latch {
                self.jump();
            }
            self.jump_latch = true;
        } else {
            self.jump_latch = false;
        }
    }

    /// Starts the jump's effect once the frame says the jump itself has
    /// begun ([`FlightFrame::warping`]), or lights the screen, unless it is
    /// playing already; and starts its fade to white once the frame's
    /// `_hyperGamma` is above 0 ([`FlightFrame::hyper_gamma`]), as the kept
    /// frame holds them after the steps or the jump just begun: a second
    /// pass's fade starts at once, as `_FadeWhiteIn` is called inside the
    /// pass (`_HandlePlayer` @0x68404), like a sound, not drawn with the
    /// frame.
    ///
    /// [`FlightFrame::warping`]: nova_sim::flight_frame::FlightFrame::warping
    /// [`FlightFrame::hyper_gamma`]: nova_sim::flight_frame::FlightFrame::hyper_gamma
    pub(super) fn start_jump_effect(&mut self) {
        // `_hyperGamma` goes back to -1 on the first pass after the
        // arrival, so it lights only a jump still under way.
        let lit = self.frame.jump_under_way && self.frame.hyper_gamma > 0.0;
        if self.jump.is_none() && (self.frame.warping || lit) {
            self.jump = Some(JumpEffect::warp(self.hyperspace_effects));
        }
        if lit && let Some(effect) = &mut self.jump {
            effect.light();
        }
    }

    /// Settles the jump's effect once the flight has given its jump up
    /// before arrival (its warp collapsed under a disabled player, or cut
    /// by a sabotage kill outfit): lit, the screen fades back from white;
    /// unlit, the effect is dropped ([`JumpEffect::given_up`]).
    ///
    /// It runs at the top of a tick, before the keys, and before a jump
    /// begins ([`FlightView::jump`]), as `_HandlePlayer` calls
    /// `_FadeWhiteOut` at the top of its first pass with +0x50 no longer
    /// above 0 (@0x6840b-0x6841f): the frame drawn after the cancelling
    /// tick still shows the fade to white where it had got, as the
    /// original's asynchronous `CGDisplayFade` runs on under it.
    pub(super) fn settle_given_up_jump(&mut self) {
        if self.jump.is_none() || self.jump_under_way() {
            return;
        }
        if let Some(fade) = self.jump.take().and_then(JumpEffect::given_up) {
            self.fade_in = Some(fade);
        }
    }

    /// A tick's fades, as `Screen::tick` runs them after the frame's
    /// catch-up: the arrival's fade-in moves on by `dt`, and goes once it
    /// is done; then a jump the flight gave up is settled
    /// ([`FlightView::settle_given_up_jump`]).
    pub(super) fn fade_frame(&mut self, dt: std::time::Duration) {
        if let Some(effect) = &mut self.fade_in {
            effect.advance(dt);
            if effect.done() {
                self.fade_in = None;
            }
        }
        self.settle_given_up_jump();
    }

    /// The jump's effect, while it plays: the warp and its fade to white
    /// until the ship arrives, or else the arrival's fade-in (or flash).
    #[must_use]
    pub fn jump_effect(&self) -> Option<&JumpEffect> {
        self.jump.as_ref().or(self.fade_in.as_ref())
    }

    /// Shows the explosions the jump left pending, `combat` as the
    /// arrival's answer carries them ([`Answer::Arrived`]), a crön's `Y`
    /// on its days among them, in the scene just laid out, so they are on
    /// the arrival's first frame: the original keeps its explod table
    /// through the arrival (only `_ResetPlayer` empties it, @0x1d7fb), and
    /// its `_PrepareAllSprites` advances them once (@0x6c59c,
    /// `_HandleExplods` @0x3a7ce) before the frame's own pass, which the
    /// steps' order from spawn to draw stands for. A `Y`'s carries no
    /// sound of its own (see
    /// [`Effects::apply`](crate::flight::effects::Effects::apply)).
    fn show_arrival_explosions(&mut self, combat: &[CombatEvent]) {
        let player = self.player_place().position;
        let scene = self.presentation.scene(player);
        let chance = &mut self.effects_chance;
        for event in combat {
            self.effects.apply(event, &scene, &self.looks, chance);
        }
        if !combat.is_empty() {
            self.effects.step(&[], &scene, &self.looks, chance);
        }
    }
}

/// The arrival, which lays the new system out ([`FlightView::catch_up`]).
impl<C: SystemCatalog + ShipSprites + CombatLooks + StatusBars, S: FlightSim> FlightView<C, S> {
    /// A tick's arrival, as `Screen::tick` runs it after the steps and the
    /// effect's advance: on the frame that says the warp has ended
    /// ([`FlightFrame::warp_end_due`](nova_sim::flight_frame::FlightFrame::warp_end_due),
    /// read from the kept frame, which the steps and the effect's start
    /// have just filled), the ship arrives ([`FlightView::arrive`]), and
    /// the rest of the effect, the fade-in or the white flash, plays over
    /// the new system as the arrival's fade-in. A double-time frame drawn
    /// between its passes comes first
    /// ([`FlightView::holding_between_passes`]): that frame shows the old
    /// system, and the arrival follows on the next step, which stops
    /// before it runs.
    pub(super) fn arrival_frame(&mut self) {
        if !self.frame.warp_end_due || self.holding_between_passes() {
            return;
        }
        self.arrive();
        self.fade_in = self
            .jump
            .take()
            .map(|mut effect| {
                effect.arrive();
                effect
            })
            .filter(|effect| !effect.done());
    }

    /// Ends the jump ([`FlightCommand::Arrive`]): the ship arrives in the
    /// next system, caught up with as [`FlightView::arrived`] says.
    pub(super) fn arrive(&mut self) {
        if let Answer::Arrived { combat, .. } = self.send(FlightCommand::Arrive) {
            self.arrived(&combat);
        }
    }

    /// Catches up with the ship having arrived in a system, `combat` the
    /// fight's events its arrival left pending
    /// ([`FlightView::catch_up`], which lays it out from the flight's
    /// event, drawn from where the ship arrives), and shows the flight's
    /// arrival message, led as the original leads it, naming the scene
    /// laid out and the flight's date as the frame the catch-up refilled
    /// holds it
    /// ([`FlightFrame::long_date`](nova_sim::flight_frame::FlightFrame::long_date)),
    /// or the system's message buoy in its place ([`SimMessage::Buoy`],
    /// its text as the flight gives it), followed by how many fighters
    /// were abandoned, the line's own tail (`_HandlePlayer`
    /// @0x6c3c4-0x6c42b), then the flight's other messages
    /// ([`FlightView::word`]) and how many hired escorts defected unpaid,
    /// if any. A buoy's arrival carries no
    /// fighter notes: the flight drops them, as the buoy replaces the
    /// arrival line's suffix too. The escort menu shuts at once and
    /// silently, at a jump's arrival (`_HandlePlayer` @0x6c50e) and a
    /// gate's exit (`_HandlePlayerDockRequest` @0x6705a) alike.
    pub(super) fn arrived(&mut self, combat: &[CombatEvent]) {
        self.catch_up();
        self.shut_escort_menu();
        self.show_arrival_explosions(combat);
        // Every arrival sets `_mapScroll` to the system arrived in: a
        // jump's (`_HandlePlayer` @0x6c073), a hypergate's
        // (`_PlayerEnterHypergate` @0x63a5d) and a wormhole's
        // (`_PlayerEnterWormhole` @0x64307).
        self.map.recentre();
        let (fighters, pay, drained) = self.arrival_outputs();
        let date = self.frame.long_date.clone();
        let mut messages = std::mem::take(&mut self.pending_leads);
        messages.extend(drained);
        let worded: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| Some((self.text_of(message)?, message.ticks())))
            .collect();
        self.blank_message();
        let Some(scene) = &self.scene else {
            return;
        };
        let stellars = !scene.stellars().is_empty();
        let chance = &mut self.words_chance;
        let mut said: Vec<(String, u16)> = messages
            .iter()
            .filter_map(|message| {
                let text = match message {
                    SimMessage::Buoy(text) => text.clone(),
                    _ => arrival_message_with(
                        arrival_lead(message, chance)?,
                        scene.name(),
                        &date,
                        stellars,
                    ),
                };
                Some((text, message.ticks()))
            })
            .next_back()
            .into_iter()
            .collect();
        said.extend(fighters);
        said.extend(worded);
        said.extend(pay);
        self.say_all(&said);
    }
}
