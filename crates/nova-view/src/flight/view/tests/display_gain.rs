//! The frame's gain the view steps each ship's shield flash and weapon
//! glow by: the pass's [`PassReport::speed_mult`], `_HandleShipDisplay`'s
//! `_gSpeedMult` (@0x2c964, @0x2c7ed), which `step_effects` hands to
//! `show_combat`, `step_flashes` and [`WeaponGlows::step`]. Each expected
//! value is the flash's or glow's own step at the same gain.

use nova_sim::SpeedMult;
use nova_sim::flight_frame::PassReport;

use super::stepping::{NPC, fight, frame, hit, one_step, stepping};
use super::*;
use crate::flight::flash::ShieldFlash;
use crate::flight::weapon_glow::WeaponGlows;

const HALF: SpeedMult = SpeedMult::new(0.5);

/// A pass at half gain raising `combat`.
fn at_half(combat: Vec<CombatEvent>) -> PassReport {
    PassReport {
        speed_mult: HALF,
        ..super::stepping::fight(combat)
    }
}

#[test]
fn a_hits_flash_steps_at_its_passs_gain() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![at_half(vec![hit(NPC, false)])]));
    frame(&mut view, one_step(vec![at_half(vec![])]));
    let npc = fight(vec![]).ships[1];
    let has_layer = view
        .npc_sheets
        .get(&npc.class)
        .and_then(|sheet| sheet.as_ref().ok())
        .is_some_and(|sheet| sheet.shield.is_some());
    let base = npc.class_shield.unwrap_or(npc.shield.max);
    let mut expected = ShieldFlash::default();
    expected.hit(false);
    for _ in 0..2 {
        expected.step(npc.shield.now, base, has_layer, HALF);
    }
    assert_eq!(view.flashes.get(&NPC), Some(&expected));
    assert_eq!(expected.shown(), 31.5, "half a level down");
}

#[test]
fn a_firings_glow_falls_at_its_passs_gain() {
    let mut view = stepping(armed(1, &[BLASTER]));
    view.sheet = Ok(ShipSheet {
        weap_decay: 30,
        ..crate::flight::fake_catalog::sheet()
    });
    let decay = view.sheet.as_ref().map(ShipSheet::weapon_decay).ok();
    let lit = CombatEvent::WeaponGlow {
        ship: ShipRef::Player,
        late: false,
    };
    frame(&mut view, one_step(vec![at_half(vec![lit])]));
    frame(&mut view, one_step(vec![at_half(vec![])]));
    let mut expected = WeaponGlows::default();
    expected.step(&[(ShipRef::Player, false)], |_| decay, HALF);
    expected.step(&[], |_| decay, HALF);
    assert_eq!(
        view.glows.drawn(ShipRef::Player),
        expected.drawn(ShipRef::Player)
    );
    let mut full = WeaponGlows::default();
    full.step(&[(ShipRef::Player, false)], |_| decay, SpeedMult::FULL);
    full.step(&[], |_| decay, SpeedMult::FULL);
    assert_ne!(
        expected.drawn(ShipRef::Player),
        full.drawn(ShipRef::Player),
        "the gain shows"
    );
}
