//! The ships the player leads at a hypergate or wormhole exit, as the
//! engine reads [`RuleKey::GateEscorts`] ([`Session::lead_out_of_gate`]).
//!
//! A hypergate exit and a wormhole exit take one path in
//! `_HandlePlayerDockRequest`. In its shared tail, loop C
//! (@0x674ef-0x675c8) takes every active ship whose leader (+0x9a) is the
//! player, 0: hired escort, fighter or mission escort. For each it calls
//! `_AIMakeShipEmergeFromHyperGate` (@0x89518), then puts it at the exit
//! stellar's position, gives it the player's heading (+0x44), and sets its
//! wait (+0x4c) to `Rand(20) + 15` ticks. `_AIMakeShipEmergeFromHyperGate`
//! clears its target (+0x70 = -1) and, for leader 0, starts its burst at
//! 15 rather than 30 (@0x895aa-0x895bd; see
//! [`emerge`](crate::traffic::emerge)). Loop C follows loop B and
//! `_EnterMoreShips`, so it takes the ships the arrival brought in, the
//! followers included. Neither makes a sound.
//!
//! `_SetupShipsInSystem` has snapped the hired escorts and fighters into
//! formation before (@0x42c94-0x42ca1); loop C moves them to the gate.
//! Only a take-off reaches the snap of every led ship
//! (`_SetupShipsInSystem(sys, 1)` @0x674ad-0x674c2).
//!
//! The goal is left as it is: [`Goal::Formation`](crate::ai::Goal)
//! resumes after the burst, as `_EscortAI` does once goal 21 ends;
//! [`Mode::Emerging`] stands in for goal 21. The remembered stellar
//! (+0x92) is not modelled, as for any emerging ship.
//!
//! By the other reading each led ship is snapped onto its slot beside the
//! player.

use super::Session;
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::gate::GateArrivalRule;
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::npc::{Mode, NpcId};

/// `Rand(20)`: the spread of a led ship's wait in the gate (@0x675a3).
const LEAD_OUT_DRAW: u32 = 20;
/// The least a led ship waits in the gate (@0x675b0).
const LEAD_OUT_LEAST: u32 = 15;

impl Session {
    /// Loop C (`_HandlePlayerDockRequest` @0x674ef-0x675c8): at a gate
    /// exit at `exit`, every ship the player leads, in ship-table order,
    /// comes out of the exit gate after it: there at rest with the
    /// player's heading, no target, waiting `Rand(20) + 15` ticks drawn on
    /// `chance`, then bursting at 15 ([`Npc::led_out`](crate::traffic::npc::Npc::led_out)).
    ///
    /// Under [`GateArrivalRule::LikeJump`] the player did not come out of
    /// a gate, so there is no gate for them to come out of: they are left
    /// as a jump's arrival leaves them. By the other reading of
    /// [`RuleKey::GateEscorts`] each is snapped onto its slot.
    pub(super) fn lead_out_of_gate(&mut self, exit: Vec2, chance: &mut (impl Chance + ?Sized)) {
        if self.gate_arrival == GateArrivalRule::LikeJump {
            return;
        }
        let led: Vec<NpcId> = self.player_led().map(|npc| npc.id).collect();
        if self.rules.source_for(RuleKey::GateEscorts) != RuleSource::Engine {
            for id in led {
                self.snap(id);
                if let Some(npc) = self.npc_mut(id) {
                    npc.mode = Mode::Flying;
                }
            }
            return;
        }
        let heading = self.player.heading;
        for id in led {
            let wait = chance.below(LEAD_OUT_DRAW) + LEAD_OUT_LEAST;
            if let Some(npc) = self.npc_mut(id) {
                npc.state = ShipState {
                    position: exit,
                    velocity: Vec2::ZERO,
                    heading,
                };
                npc.target = None;
                npc.mode = Mode::Emerging { ticks_left: wait };
                npc.led_out = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::gate::GateKind;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::rulebook::Rulebook;
    use crate::session::gate_follow::tests::{exit_gate_raising, exit_of, following, gated};
    use crate::session::mission_ships::tests::{accepted, jump_to, take_off, tick, ticks};
    use crate::testkit::{Draws, FakePilotCatalog};
    use crate::traffic::emerge::LED_BURST_TICK;
    use crate::traffic::npc::Npc;

    /// A hired escort of ship 200.
    fn hired() -> Escort {
        Escort {
            ship: ShipId(200),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// A catalog whose mission 140's two ships escort the player
    /// (`ShipBehav` 1, `ShipSyst` -6), from the start.
    fn catalog() -> FakePilotCatalog {
        gated(following(1, 0), 0)
    }

    /// In flight in 130 with a hired escort and mission 140's escorts,
    /// under `rules`.
    fn flying(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = accepted(catalog).with_rules(rules);
        session.pilot.escorts.push(hired());
        take_off(&mut session, catalog);
        session
    }

    /// The ships the player leads.
    fn led(session: &Session) -> Vec<Npc> {
        session.player_led().cloned().collect()
    }

    /// Whether `npc` is on its slot beside the player, with its velocity.
    fn on_slot(session: &Session, npc: &Npc) -> bool {
        let formation = npc.formation().expect("a slot");
        npc.state
            == ShipState {
                position: formation.position(&session.player),
                ..session.player
            }
    }

    #[test]
    fn every_ship_the_player_leads_comes_out_of_either_exit_gate_with_its_heading() {
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = catalog();
            let mut session = flying(&catalog, Rulebook::default());
            exit_gate_raising(&mut session, &catalog, kind, &mut NeverFires);
            let led = led(&session);
            assert_eq!(led.len(), 3, "{kind:?}: the hired escort and two");
            assert!(led.iter().any(Npc::mission_escort), "{kind:?}");
            assert!(led.iter().any(Npc::is_players), "{kind:?}");
            for npc in led {
                let expected = ShipState {
                    position: exit_of(kind),
                    velocity: Vec2::ZERO,
                    heading: session.player.heading,
                };
                assert_eq!(npc.state, expected, "{kind:?}");
                assert_eq!(npc.mode, Mode::Emerging { ticks_left: 34 }, "{kind:?}");
                assert_eq!(npc.target, None, "{kind:?}");
                assert!(npc.led_out, "{kind:?}");
            }
        }
    }

    #[test]
    fn each_led_ship_waits_rand_20_plus_15() {
        let catalog = catalog();
        let mut session = flying(&catalog, Rulebook::default());
        exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        let ids: Vec<NpcId> = session.player_led().map(|npc| npc.id).collect();
        let mut draws = Draws::of(&[0, 19, 7]);
        session.lead_out_of_gate(exit_of(GateKind::Hypergate), &mut draws);
        assert_eq!(draws.asked, [LEAD_OUT_DRAW; 3], "one Rand(20) a ship");
        let mut waits: Vec<u32> = ids
            .iter()
            .map(|&id| match session.npc(id).map(|npc| npc.mode) {
                Some(Mode::Emerging { ticks_left }) => ticks_left,
                other => panic!("{other:?}"),
            })
            .collect();
        waits.sort_unstable();
        assert_eq!(waits, [15, 22, 34]);
    }

    #[test]
    fn a_led_ship_holds_at_the_gate_then_bursts_at_15() {
        let catalog = catalog();
        let mut session = flying(&catalog, Rulebook::default());
        exit_gate_raising(&mut session, &catalog, GateKind::Wormhole, &mut NeverFires);
        let ids: Vec<NpcId> = session.player_led().map(|npc| npc.id).collect();
        let heading = session.player.heading;
        let exit = exit_of(GateKind::Wormhole);
        ticks(&mut session, &catalog, 33);
        for &id in &ids {
            let npc = session.npc(id).expect("here");
            assert!(npc.emerging(), "still waiting");
            assert_eq!(npc.state.position, exit, "held at the exit");
        }
        tick(&mut session, &catalog);
        for &id in &ids {
            let npc = session.npc(id).expect("here");
            assert_eq!(
                npc.mode,
                Mode::LeavingGate {
                    tick: LED_BURST_TICK
                }
            );
        }
        tick(&mut session, &catalog);
        for &id in &ids {
            let npc = session.npc(id).expect("here");
            let moved = npc.state.position - exit;
            let along = crate::flight::facing(heading) * 15.0;
            assert!((moved - along).length() < 1e-3, "{moved:?}");
        }
    }

    #[test]
    fn the_other_gate_escorts_reading_snaps_every_led_ship_onto_its_slot() {
        let catalog = catalog();
        let rules = Rulebook::default().with_override(RuleKey::GateEscorts, RuleSource::Bible);
        let mut session = flying(&catalog, rules);
        exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        let led = led(&session);
        assert_eq!(led.len(), 3);
        for npc in &led {
            assert!(on_slot(&session, npc), "{:?}", npc.state);
            assert_eq!(npc.mode, Mode::Flying);
            assert!(!npc.led_out);
        }
    }

    #[test]
    fn like_a_jump_the_led_ships_are_left_as_a_jumps_arrival_leaves_them() {
        let catalog = catalog();
        let mut session =
            flying(&catalog, Rulebook::default()).with_gate_arrival(GateArrivalRule::LikeJump);
        exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        let hired = session.fleet[0].expect("placed");
        let npc = session.npc(hired).expect("here").clone();
        assert!(on_slot(&session, &npc), "{:?}", npc.state);
        assert!(!npc.emerging());
        assert!(!npc.led_out);
    }

    #[test]
    fn a_take_off_and_a_jump_still_place_the_escorts_on_their_slots() {
        let catalog = catalog();
        let session = flying(&catalog, Rulebook::default());
        let led = led(&session);
        assert_eq!(led.len(), 3, "the hired escort and two");
        for npc in &led {
            assert!(on_slot(&session, npc), "take-off: {:?}", npc.state);
            assert!(!npc.emerging());
        }
        let mut session = session;
        jump_to(&mut session, &catalog, 131);
        assert_eq!(session.pilot.system, SystemId(131));
        let hired = session.fleet[0].expect("placed");
        let npc = session.npc(hired).expect("here").clone();
        assert!(on_slot(&session, &npc), "jump: {:?}", npc.state);
        assert!(!npc.emerging());
    }
}
