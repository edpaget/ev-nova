//! A kept escort's weapon glow and shield flash, carried into its new NPC
//! (rdm rendering phase 15). All addresses are in `EV Nova.app`'s
//! `EV Nova` (i386).
//!
//! # The original
//!
//! `_HandleShipDisplay` steps each shown ship's weapon glow (+0xc8e8,
//! @0x2c7cd-0x2c816) and shield flash (+0xc8f0, @0x2c954-0x2c975) at the
//! gain of the moment, so every `_PrepareAllSprites` steps them once more.
//! Every way into a system resets the player's and every spawn zeroes a
//! new NPC's, but `_RespawnEscort` (@0x3d4b8-0x3d62c, from
//! `_SetupShipsInSystem` @0x42c79) writes neither, and neither does
//! `_DeleteAllComputerShips(0)`, which keeps the escort's slot whole
//! (@0x3bf7e-0x3bfb6). So a kept escort goes on from where its last
//! flight display left both, stepped once by each prepare that displays
//! its slot:
//!
//! - a jump's arrival: the one @0x6c59c, at the frame's gain;
//! - a hypergate's or wormhole's exit: the tail's two (@0x6764e,
//!   @0x67658) at the frame's gain, and the entry's own (@0x63e91,
//!   @0x64743) first when the exit is in the system gone in, as only then
//!   does the old slot pass the display's system test (@0x2b7d8-0x2b7ed);
//! - a wormhole with no exit: the tail's two, at the frame's gain;
//! - a take-off: the tail's two, at the gain of 0 `_PlayerLandOnStellar`
//!   wrote (@0x6375e-0x63769).
//!
//! A loaded pilot's escorts are new (`_InitObjects` zeroes every slot),
//! and a mission escort is never respawned (its spawn zeroes both), so
//! neither carries anything.
//!
//! # Here
//!
//! The glows and the flashes live in the flight view, keyed by ship, and
//! the view steps them by each pass's gain; an escort placed afresh is a
//! new NPC. So the session reports, at each placement of the fleet
//! (`Session::enter_escorts`), which new NPC each kept escort is and the
//! gain of every prepare that displayed it since its last flight display,
//! as the old NPC or the new ([`CarriedDisplay`]); the view moves the old
//! NPC's glow and flash onto the new one and steps them once for each.
//! [`DisplayCarry`] keeps the gains each prepare noted
//! (`Session::display_slots`), forgets them at each flight pass's ship
//! loop, which the view steps every glow and flash at, and is drained as
//! [`FlightEvent::DisplaysCarried`](crate::flight_frame::FlightEvent::DisplaysCarried)
//! by `Session::drain_events`, empty when the placement kept none, so the
//! view knows when to let go of what it held through the change.

use std::collections::BTreeMap;

use crate::SpeedMult;
use crate::flight_frame::CarriedDisplay;
use crate::traffic::npc::NpcId;

use super::Session;

/// What the view is owed of the kept escorts' displays (see the module
/// docs); never saved, as every entry into flight places the fleet
/// afresh.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct DisplayCarry {
    /// The gain of each prepare that displayed an NPC since the last
    /// flight pass, by NPC, in order.
    prepared: BTreeMap<NpcId, Vec<SpeedMult>>,
    /// Each kept escort's NPCs since the view last took them, the first
    /// its old one, the last its new one, any between those it was given
    /// by a placement since; some, maybe empty, once the fleet was placed.
    placed: Option<Vec<Vec<NpcId>>>,
}

impl DisplayCarry {
    /// NPC `id` was displayed by a `_PrepareAllSprites` at `gain`.
    pub(super) fn prepared(&mut self, id: NpcId, gain: SpeedMult) {
        self.prepared.entry(id).or_default().push(gain);
    }

    /// A flight pass's ship loop displays every ship, as the view steps
    /// every glow and flash at it: the prepares before it are spent.
    pub(super) fn flown(&mut self) {
        self.prepared.clear();
    }

    /// The fleet is placed afresh, keeping the escorts `kept`, each its
    /// old NPC and its new. A kept escort placed again before the view
    /// took the first is carried from its first NPC.
    pub(super) fn placed(&mut self, kept: impl IntoIterator<Item = (NpcId, NpcId)>) {
        let placed = self.placed.get_or_insert_with(Vec::new);
        for (from, to) in kept {
            match placed.iter_mut().find(|npcs| npcs.last() == Some(&from)) {
                Some(npcs) => npcs.push(to),
                None => placed.push(vec![from, to]),
            }
        }
    }

    /// Each kept escort's carry since the last take, if the fleet was
    /// placed since: each of its NPCs' prepares, the old one's first. The
    /// prepares noted are spent with it.
    pub(super) fn take(&mut self) -> Option<Vec<CarriedDisplay>> {
        let placed = self.placed.take()?;
        let prepared = std::mem::take(&mut self.prepared);
        Some(
            placed
                .into_iter()
                .filter_map(|npcs| {
                    Some(CarriedDisplay {
                        from: *npcs.first()?,
                        to: *npcs.last()?,
                        prepares: npcs
                            .iter()
                            .filter_map(|id| prepared.get(id))
                            .flatten()
                            .copied()
                            .collect(),
                    })
                })
                .collect(),
        )
    }
}

impl Session {
    /// Each kept escort's display carried into its new NPC since the last
    /// take, if the fleet was placed since (see the module docs); taking
    /// it leaves none.
    pub(super) fn take_carried_displays(&mut self) -> Option<Vec<CarriedDisplay>> {
        self.space.escorts.carry.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use crate::chance::NeverFires;
    use crate::flight_frame::FlightEvent;
    use crate::gate::GateKind;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::display_gain::tests::HALF;
    use crate::session::tests::fleet_escort;
    use crate::testkit::{FAST, begin_jump_now, far_129_reachable, fly_out, land_now, ship};

    /// The NPC of the first escort of `session`'s fleet.
    fn escort_id(session: &Session) -> NpcId {
        session.space.escorts.fleet[0].expect("placed")
    }

    /// Checks the carry `session` reports for the fleet's first escort:
    /// its old NPC `from` into its new one, with `prepares`.
    fn carried(session: &mut Session, from: NpcId, prepares: Vec<SpeedMult>) {
        let to = escort_id(session);
        assert_ne!(from, to, "a new NPC");
        let carried = session.take_carried_displays().expect("placed");
        assert_eq!(
            carried.iter().find(|carried| carried.from == from),
            Some(&CarriedDisplay { from, to, prepares })
        );
    }

    #[test]
    fn a_jumps_arrival_carries_each_kept_escort_with_its_one_prepare_at_the_frames_gain() {
        // `_SetupShipsInSystem` @0x6c1f9, then `_PrepareAllSprites`
        // @0x6c59c at the frame's gain.
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        let from = escort_id(&session);
        session.at_gain(HALF);
        session.take_carried_displays();
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        carried(&mut session, from, vec![HALF]);
    }

    #[test]
    fn a_gate_exit_carries_it_with_two_prepares_or_three_in_the_system_gone_in() {
        // The tail's two (@0x6764e, @0x67658), and the entry's own
        // (@0x63e91) first when the old slot passes the display's system
        // test (@0x2b7d8-0x2b7ed): with the exit in the system gone in.
        use crate::session::gate_escorts::tests::{exit_into_130, flying, gated_into_130};
        use crate::session::gate_follow::tests::exit_gate_raising;
        for (into_130, prepares) in [(true, 3), (false, 2)] {
            let catalog = gated_into_130();
            let mut session = flying(&catalog, Rulebook::default());
            let from = escort_id(&session);
            session.at_gain(HALF);
            session.take_carried_displays();
            if into_130 {
                exit_into_130(&mut session, &catalog);
            } else {
                exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
            }
            carried(&mut session, from, vec![HALF; prepares]);
        }
    }

    #[test]
    fn a_wormholes_exit_carries_it_with_two_prepares() {
        use crate::session::gate_escorts::tests::{flying, gated_into_130};
        use crate::session::gate_follow::tests::exit_gate_raising;
        let catalog = gated_into_130();
        let mut session = flying(&catalog, Rulebook::default());
        let from = escort_id(&session);
        session.at_gain(HALF);
        session.take_carried_displays();
        exit_gate_raising(&mut session, &catalog, GateKind::Wormhole, &mut NeverFires);
        carried(&mut session, from, vec![HALF; 2]);
    }

    #[test]
    fn a_wormhole_with_no_exit_carries_it_with_two_prepares() {
        // The tail's two (@0x6764e, @0x67658), at the frame's gain.
        use crate::session::wormhole_no_exit::tests::{fought, press_over, refuse};
        let catalog = fought(crate::session::tests::blaster());
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.escorts = vec![fleet_escort(129)];
        session.populate(&catalog, &mut NeverFires);
        press_over(&mut session);
        let from = escort_id(&session);
        session.at_gain(HALF);
        session.take_carried_displays();
        refuse(&mut session, &catalog);
        carried(&mut session, from, vec![HALF; 2]);
    }

    #[test]
    fn a_take_off_carries_it_with_two_prepares_at_gain_0() {
        // The tail's two at the gain `_PlayerLandOnStellar` zeroed
        // (@0x6375e-0x63769), by the engine's reading of
        // `RuleKey::SpeedMultRamp`; at full gain by the other.
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = far_129_reachable(fleeted());
        for (ramp, gain) in [
            (RuleSource::Engine, SpeedMult::ZERO),
            (RuleSource::Bible, SpeedMult::FULL),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::SpeedMultRamp, ramp);
            let mut session = flying(&catalog, RuleSource::Engine).with_rules(rules);
            land_now(&mut session).expect("lands");
            let from = escort_id(&session);
            session.take_carried_displays();
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            carried(&mut session, from, vec![gain; 2]);
        }
    }

    #[test]
    fn a_placement_that_keeps_no_escort_still_reports_an_empty_carry() {
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_carried_displays();
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.take_carried_displays(), Some(Vec::new()));
    }

    #[test]
    fn a_loaded_pilots_escorts_carry_nothing() {
        // `_InitObjects` zeroes every slot (@0x1c4c7, @0x1c4d4): the
        // escorts `_LoadPilotData` spawns are new.
        let mut catalog = crate::testkit::catalog();
        catalog.ship_records.push(ship(129, FAST));
        let mut pilot = crate::pilot::Pilot::new(&catalog, "").expect("a pilot");
        pilot.escorts.push(fleet_escort(129));
        let mut session = Session::load(&catalog, pilot, &Rulebook::default()).expect("loads");
        session.open_saved(crate::session::SavedOpening::AtLaunch, &mut NeverFires);
        // The entry sets the ships up, placing them (see the
        // `priming_passes` module).
        crate::testkit::enter_game(&mut session, &catalog, HALF, &mut NeverFires);
        escort_id(&session);
        assert_eq!(session.take_carried_displays(), Some(Vec::new()));
    }

    #[test]
    fn nothing_is_carried_until_the_fleet_is_placed() {
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.take_carried_displays();
        session.at_gain(HALF);
        session.prepare_all_sprites(&mut NeverFires);
        assert_eq!(session.take_carried_displays(), None);
        // The prepare is still owed to the placement that follows.
        let from = escort_id(&session);
        session.at_gain(SpeedMult::FULL);
        session.populate(&catalog, &mut NeverFires);
        carried(&mut session, from, vec![HALF, SpeedMult::FULL]);
    }

    #[test]
    fn the_carry_drains_once_as_displays_carried() {
        use crate::session::escorts::tests::{fleeted, flying};
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let mut drained = Vec::new();
        session.drain_events(&mut drained);
        let fleet = |session: &Session| -> Vec<NpcId> {
            session
                .space
                .escorts
                .fleet
                .iter()
                .flatten()
                .copied()
                .collect()
        };
        let from = fleet(&session);
        session.populate(&catalog, &mut NeverFires);
        let gain = session.speed_mult();
        let expected: Vec<CarriedDisplay> = from
            .into_iter()
            .zip(fleet(&session))
            .map(|(from, to)| CarriedDisplay {
                from,
                to,
                prepares: vec![gain],
            })
            .collect();
        assert!(!expected.is_empty());
        let mut drained = Vec::new();
        session.drain_events(&mut drained);
        assert_eq!(
            drained.last(),
            Some(&FlightEvent::DisplaysCarried(expected)),
            "last of the stream"
        );
        let mut again = Vec::new();
        session.drain_events(&mut again);
        assert!(
            !again
                .iter()
                .any(|event| matches!(event, FlightEvent::DisplaysCarried(_))),
            "once"
        );
    }

    #[test]
    fn a_flight_pass_spends_the_prepares_before_it() {
        // The view steps every glow and flash at each pass, so only the
        // prepares since the escort's last flight display are owed.
        use crate::session::escorts::tests::{fleeted, flying, tick};
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.take_carried_displays();
        session.prepare_all_sprites(&mut NeverFires);
        tick(&mut session, &catalog);
        let from = escort_id(&session);
        session.take_carried_displays();
        session.populate(&catalog, &mut NeverFires);
        let gain = session.speed_mult();
        carried(&mut session, from, vec![gain]);
    }

    #[test]
    fn the_old_npcs_prepares_come_before_the_new_ones() {
        let (a, b, c) = (NpcId(1), NpcId(2), NpcId(3));
        let mut carry = DisplayCarry::default();
        carry.prepared(a, HALF);
        carry.prepared(c, SpeedMult::ZERO);
        carry.placed([(a, b)]);
        carry.prepared(b, SpeedMult::FULL);
        assert_eq!(
            carry.take(),
            Some(vec![CarriedDisplay {
                from: a,
                to: b,
                prepares: vec![HALF, SpeedMult::FULL],
            }])
        );
        assert_eq!(carry.take(), None, "taken");
        carry.placed([(b, c)]);
        assert_eq!(
            carry.take(),
            Some(vec![CarriedDisplay {
                from: b,
                to: c,
                prepares: Vec::new(),
            }]),
            "the prepares spent with the take"
        );
    }

    #[test]
    fn a_second_placement_before_the_take_carries_from_the_first_npc() {
        let (a, b, c) = (NpcId(1), NpcId(2), NpcId(3));
        let mut carry = DisplayCarry::default();
        carry.prepared(a, HALF);
        carry.placed([(a, b)]);
        carry.prepared(b, SpeedMult::FULL);
        carry.placed([(b, c)]);
        carry.prepared(c, SpeedMult::ZERO);
        assert_eq!(
            carry.take(),
            Some(vec![CarriedDisplay {
                from: a,
                to: c,
                prepares: vec![HALF, SpeedMult::FULL, SpeedMult::ZERO],
            }])
        );
    }
}
