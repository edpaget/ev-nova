//! [`FlightView`]'s land key, gates and course map's close, over the
//! [`FlightSim`] port: the press sends [`FlightCommand::Land`] and says
//! what the flight answers, the gates open and enter by
//! [`FlightCommand::OpenHypergate`], [`FlightCommand::EnterWormhole`] and
//! [`FlightCommand::EnterHypergate`], and the map's close runs whole here,
//! its hypergate entry among it. The flight's messages are worded by the
//! flight's names ([`FlightView::word`]).

use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::landing::LandOutcome;
use nova_sim::{CombatEvent, GateKind, GateRefusal, LandPress};

use crate::Input;
use crate::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use crate::flight::jump::JumpEffect;
use crate::flight::sim_port::FlightSim;
use crate::system::catalog::SystemCatalog;

use super::{
    FlightView, GATE_REFUSAL_TICKS, HYPERGATE_CANCELLED, MAP_KEY, gate_refusal_message,
    landing_refusal_ticks, refusal_message, stellar_name,
};

impl<C: SystemCatalog + ShipSprites + CombatLooks + StatusBars, S: FlightSim> FlightView<C, S> {
    /// Presses the land key ([`FlightCommand::Land`]): requests clearance,
    /// lands, enters a gate, or shows why not. What the press said (the
    /// request's line, or "No response."), shows first, so a
    /// refusal in the same press replaces it. A landing hands the router
    /// the stellar ([`FlightView::take_landing`]) and lets go of what the
    /// view keeps of the fight; over a cleared hypergate the course map
    /// opens offering its links ([`FlightCommand::OpenHypergate`]), and
    /// over a wormhole the ship passes through at once
    /// ([`FlightCommand::EnterWormhole`]). A refusal names its stellar from
    /// the scene, or by the opening's name for one the scene doesn't hold
    /// ([`Names::stellar`](nova_sim::flight_frame::Names::stellar)),
    /// and says the gate's line when the flight says the stellar is one
    /// ([`Answer::LandRefused`]'s `gate`). Each command is sent under its
    /// gate ([`FlightView::send_gated`]), so a press the flight doesn't take
    /// sends nothing and says nothing. A flight that failed refuses the
    /// press.
    pub(super) fn land(&mut self) {
        let pressed = self.send_gated(FlightCommand::Land);
        if pressed.is_refused() {
            // Not taken (`FlightView::send_gated`), or a flight that failed:
            // nothing was pressed.
            return;
        }
        self.say_messages();
        match pressed {
            Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(stellar))) => {
                self.pending_landing = Some(stellar);
                self.blank_message();
                self.effects.clear();
                self.flashes.clear();
                self.held_alts.get_mut().clear();
                self.glows.clear();
                self.shot_looks.clear();
                self.redraw_status();
            }
            Answer::LandPress(LandPress::AtGate {
                kind: GateKind::Hypergate,
                ..
            }) => {
                // Refused, a hypergate without links or no entry pending
                // (`GateRefusal::NoLinks`, `NotAtGate`), says nothing
                // (`FlightView::gate_refused`).
                if let Answer::Offered(offered) = self.send_gated(FlightCommand::OpenHypergate) {
                    self.map.offer_gates(offered);
                    self.open_map();
                }
            }
            Answer::LandPress(LandPress::AtGate {
                kind: GateKind::Wormhole,
                ..
            }) => match self.send_gated(FlightCommand::EnterWormhole) {
                Answer::Arrived { combat, .. } => {
                    self.came_through(JumpEffect::flash(), &combat);
                }
                Answer::GateRefused(refusal) => self.gate_refused(refusal),
                _ => {}
            },
            Answer::LandRefused { refusal, gate } => {
                // A stellar the scene doesn't hold, such as one another
                // system owns, by its spöb name (`stellar_name`).
                let name = refusal
                    .stellar()
                    .and_then(|stellar| {
                        stellar_name(
                            self.scene.as_ref(),
                            |id| self.names.stellar(id).map(str::to_owned),
                            stellar,
                        )
                    })
                    .unwrap_or_default();
                if let Some(text) = refusal_message(&refusal, gate, &name) {
                    self.say_for(text, landing_refusal_ticks(&refusal));
                }
            }
            _ => {}
        }
    }

    /// Shows what the original says when entering a gate is refused, if
    /// anything: a cancelled hypergate jump, or a wormhole leading
    /// nowhere. A hypergate without links says nothing.
    ///
    /// A wormhole leading nowhere still runs the dock request's tail
    /// (`nova_sim::Session::enter_wormhole`): the view catches up with
    /// the flight's events once ([`FlightView::catch_up`]), which lays the
    /// system it stays in out afresh, its old ships, shots, smoke and
    /// effects let go and its new ships' sheets read; then shuts the
    /// escort menu (@0x6705a) and says why. The beep is the flight's.
    fn gate_refused(&mut self, refusal: GateRefusal) {
        match refusal {
            GateRefusal::Cancelled => self.say_for(HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS),
            GateRefusal::NoExit => {
                self.catch_up();
                self.shut_escort_menu();
                self.say_for(gate_refusal_message(GateKind::Wormhole), GATE_REFUSAL_TICKS);
            }
            GateRefusal::NoLinks | GateRefusal::NotAtGate => {}
        }
    }

    /// Plays `effect` for the ship having come out of a gate, `combat` the
    /// fight's events its arrival left pending, caught up with as an
    /// arrival is ([`FlightView::arrived`]); the flight keys
    /// are let go.
    fn came_through(&mut self, effect: JumpEffect, combat: &[CombatEvent]) {
        self.held.clear();
        self.arrived(combat);
        self.fade_in = Some(effect);
    }

    /// Closes the course map, abandoning any gesture on it, and goes back
    /// to flight ([`FlightView::close_map_then`]'s close, docked or in
    /// flight, or the hypergate map's redraw). Closing the hypergate map
    /// enters the hypergate for the system picked on it
    /// ([`FlightCommand::EnterHypergate`]): the ship comes out there, and
    /// the new system fades in from white ([`JumpEffect::emerging`], or
    /// flashes white with the Hyperspace Effects preference off), its
    /// arrival said; with no pick, the jump is cancelled
    /// ([`HYPERGATE_CANCELLED`]). The hypergate map closes silently:
    /// `_PlayerEnterHypergate` plays no beep around it (@0x6385b). See
    /// [`FlightView::close_map_then`] for the course map's close.
    pub fn close_map(&mut self) {
        self.close_map_then(|view, choice| {
            match view.send(FlightCommand::EnterHypergate(choice)) {
                Answer::Arrived { combat, .. } => {
                    let effect = JumpEffect::emerging(view.hyperspace_effects);
                    view.came_through(effect, &combat);
                }
                Answer::GateRefused(refusal) => view.gate_refused(refusal),
                _ => {}
            }
        });
    }

    /// The map's input, the rest of `_MapFilter` (@0x13c63-0x147ab) that
    /// the map leaves to its owner ([`FlightView::map_input_then`]), its
    /// close [`FlightView::close_map`]:
    ///
    /// - A key-down (not a repeat) of a key the filter doesn't take
    ///   (`FlightView::map_filter_takes`) while [`MAP_KEY`] is held
    ///   closes the map, as the map key's close reads the key held
    ///   (@0x144db-0x14587). M's own press closes it before this, in
    ///   flight and in port.
    /// - A press on it that hits systems is the flight's click
    ///   (`FlightView::answer_map_press`), and C or Clear Route clears
    ///   the route (`FlightView::clear_route`).
    /// - Tab or `\` cycles the selection (`FlightView::cycle_map`).
    /// - Return, or a click on Done, closes the map: with the close beep
    ///   and the cue in flight, entering or cancelling the hypergate
    ///   silently, and putting back the bar's nav in port, as M does.
    /// - A text Find confirmed is searched as the in-flight map's Find
    ///   searches
    ///   ([`ScreenAsk::Find`](nova_sim::flight_frame::ScreenAsk::Find)),
    ///   and the map shows what it found
    ///   ([`GalaxyMap::show_search`](crate::galaxy::GalaxyMap::show_search)).
    /// - The map's beeps (Space's and Find's) join the flight's.
    ///
    /// While Find is open every input goes to the map, which gives it to
    /// Find: a key with [`MAP_KEY`] held types, and closes nothing.
    pub(super) fn map_input(&mut self, input: &Input) {
        self.map_input_then(input, Self::close_map);
    }

    /// The input to the map opened over a screen: in port
    /// ([`FlightView::open_bar_map`], `FlightView::open_port_screen_map`),
    /// or by a mission dialog docked or in flight
    /// (`FlightView::open_dialog_map`): a press of [`MAP_KEY`] closes it
    /// ([`FlightView::close_map`]), unless Find is open, where it
    /// types, and anything else goes to the map
    /// ([`FlightView::map_input`]), so flight's own keys and cursor stay
    /// out of the port.
    pub fn port_map_input(&mut self, input: &Input) {
        let closing = Input::Key {
            key: MAP_KEY,
            pressed: true,
            repeat: false,
        };
        if *input == closing && !self.map.finding() {
            self.close_map();
        } else {
            self.map_input(input);
        }
    }
}
