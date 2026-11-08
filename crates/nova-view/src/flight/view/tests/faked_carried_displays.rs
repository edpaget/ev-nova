//! A kept escort's weapon glow and shield flash carried into its new NPC
//! through a jump's arrival, a gate's exit and a take-off, on
//! [`FakeFlight`]: the view applies the carry the flight reports
//! ([`FlightEvent::DisplaysCarried`]), stepping each once per prepare's
//! gain. Which prepares and gains are the session's rules, pinned by its
//! own tests (`nova_sim`'s `display_carry` module).

use nova_sim::flight_frame::{CarriedDisplay, FlightEvent};
use nova_sim::landing::LandOutcome;
use nova_sim::{GateKind, Gauge, LandPress, NpcId, SessionEvent, SpeedMult};

use super::faked_landing::at_gate;
use super::*;
use crate::flight::flash::ShieldFlash;
use crate::flight::weapon_glow::WeaponGlow;

/// The escort's NPC before the change.
const OLD: ShipRef = ShipRef::Npc(NpcId(4));

/// Its NPC after it.
const NEW: ShipRef = ShipRef::Npc(NpcId(9));

/// A frame of Sol holding the escort as NPC `id`, of ship 129, 20 of its
/// 30 shield left.
fn with_escort(id: u32) -> FlightFrame {
    let mut frame = sol();
    frame.ships.push(FrameShip {
        shield: Gauge {
            now: 20.0,
            max: 30.0,
        },
        ..record::npc(id, 129, Vec2::new(100.0, 0.0), 0.0)
    });
    frame
}

/// A view of the escort as [`OLD`], its shield just hit and its weapon
/// glow just lit.
fn lit() -> Faked {
    let mut view = faked(with_escort(4));
    let mut flash = ShieldFlash::default();
    flash.hit(false);
    view.flashes.insert(OLD, flash);
    let mut glow = WeaponGlow::default();
    glow.light();
    view.glows.insert(OLD, glow);
    view
}

/// The flight's report that the escort went from [`OLD`] to [`NEW`],
/// owed `prepares`.
fn carried(prepares: Vec<SpeedMult>) -> FlightEvent {
    FlightEvent::DisplaysCarried(vec![CarriedDisplay {
        from: NpcId(4),
        to: NpcId(9),
        prepares,
    }])
}

/// The flash and glow [`lit`] lit, stepped once for each of `gains` as
/// `view`'s display of [`NEW`] steps them.
fn stepped(view: &Faked, gains: &[SpeedMult]) -> (ShieldFlash, WeaponGlow) {
    let sheet = view
        .npc_sheets
        .get(&ShipId(129))
        .and_then(|sheet| sheet.as_ref().ok());
    let has_layer = sheet.is_some_and(|sheet| sheet.shield.is_some());
    let decay = sheet.map_or(0.0, ShipSheet::weapon_decay);
    let mut flash = ShieldFlash::default();
    flash.hit(false);
    let mut glow = WeaponGlow::default();
    glow.light();
    for &gain in gains {
        flash.step(20.0, 30.0, has_layer, gain);
        glow.step(decay, gain);
    }
    (flash, glow)
}

/// Checks `view` shows the escort's flash and glow as [`NEW`], stepped
/// once for each of `gains`, and nothing as [`OLD`].
fn shows_carried(view: &mut Faked, gains: &[SpeedMult]) {
    let (flash, glow) = stepped(view, gains);
    assert_eq!(view.flashes.get(&NEW), Some(&flash));
    assert_eq!(view.flashes.get(&OLD), None);
    assert!(glow.drawn().is_some(), "shown");
    assert_eq!(view.glows.take(NEW), Some(glow));
    assert_eq!(view.glows.take(OLD), None);
}

#[test]
fn a_jumps_arrival_shows_the_escorts_glow_and_flash_stepped_by_its_prepare() {
    let mut view = lit();
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        ..with_escort(9)
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim.queue_event(carried(vec![SpeedMult::FULL]));
    view.arrived(&[]);
    shows_carried(&mut view, &[SpeedMult::FULL]);
}

#[test]
fn a_gate_exit_shows_them_stepped_by_each_prepare() {
    let mut view = lit();
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    let gains = [SpeedMult::FULL; 3];
    view.sim.queue_answer_telling(
        Answer::Arrived {
            system: SystemId(131),
            combat: Vec::new(),
        },
        FlightFrame {
            system: SystemId(131),
            ..with_escort(9)
        },
        vec![
            FlightEvent::Session(SessionEvent::SystemChanged),
            carried(gains.to_vec()),
        ],
    );
    view.land();
    shows_carried(&mut view, &gains);
}

#[test]
fn a_take_off_shows_them_as_the_landing_left_them() {
    let mut view = lit();
    view.sim
        .queue_answer(Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(
            StellarId(128),
        ))));
    view.land();
    assert!(view.flashes.is_empty(), "let go as the player lands");
    let gains = [SpeedMult::ZERO; 2];
    view.sim.queue_answer_telling(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: nova_sim::Settled::default(),
        }),
        with_escort(9),
        vec![carried(gains.to_vec())],
    );
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.glows.drawn(NEW), Some(32), "as lit: gain 0 drops none");
    shows_carried(&mut view, &gains);
}

#[test]
fn a_change_that_carries_nothing_lets_go_of_the_held() {
    let mut view = lit();
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        ..with_escort(9)
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim
        .queue_event(FlightEvent::DisplaysCarried(Vec::new()));
    view.arrived(&[]);
    assert!(view.flashes.is_empty());
    assert_eq!(
        view.held_displays,
        crate::flight::carried_displays::HeldDisplays::default()
    );
}
