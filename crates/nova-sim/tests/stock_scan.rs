//! The contraband scan over the stock data (`_ScanPlayer` @0x7eabd-0x7f3d7;
//! see `nova_sim::scan`). Skips, passing, when `NOVA_DATA` is unset.
//!
//! The Federation (`gövt` 128) scans for `ScanMask` 0x8000. Bio-weapons
//! (`jünk` 137, 0xf597) and the illegal medium blaster (`oütf` 320) carry
//! that bit; water (`jünk` 134) carries only 0x0800, the bit nearly every
//! stock `jünk` has, so it shields the outfits from the scan by the
//! engine's reading.
//!
//! The warp-up sound, `snd ` 128, gives the warp's lead-in, through which
//! the player is still scanned (`_ScanPlayer` @0x7e562-0x7e633; see
//! `nova_sim::warp`).

mod common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_sim::market::Good;
use nova_sim::scan::{Carried, Contraband, Latches, ScanRules, contraband};
use nova_sim::warp::{SoundLength, held, sound_duration, warp_factor};
use nova_sim::{Governments, GovtId, JunkId, OutfitId, PilotCatalog, ShipId};

/// What the Federation finds on a player holding `junk` (a ton of each)
/// and owning one of each of `outfits`, with 10,000 credits, both latches
/// armed, by the engine.
fn found(data: &GameData, junk: &[i16], outfits: &[i16]) -> Option<Contraband> {
    let govts = Governments::read(data);
    let federation = govts.get(GovtId(128)).expect("the Federation");
    let junk_records = PilotCatalog::junk(data);
    let outfit_records = PilotCatalog::outfits(data);
    let cargo: BTreeMap<Good, u32> = junk.iter().map(|&id| (Good::Junk(JunkId(id)), 1)).collect();
    let owned: BTreeMap<OutfitId, u16> = outfits.iter().map(|&id| (OutfitId(id), 1)).collect();
    let carried = Carried {
        cargo: &cargo,
        junk: &junk_records,
        outfits: &owned,
        outfit_records: &outfit_records,
    };
    contraband(
        federation,
        &carried,
        10_000,
        Latches::ARMED,
        ScanRules::default(),
    )
}

#[test]
fn the_federation_finds_bio_weapons_and_the_illegal_blaster_but_not_water() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let federation = Governments::read(&data)
        .get(GovtId(128))
        .map(|record| record.scan_mask);
    assert_eq!(federation, Some(0x8000));
    assert!(matches!(
        found(&data, &[137], &[]),
        Some(Contraband::Junk {
            id: JunkId(137),
            ..
        })
    ));
    assert_eq!(found(&data, &[134], &[]), None, "water");
    assert!(matches!(
        found(&data, &[], &[320]),
        Some(Contraband::Outfit {
            id: OutfitId(320),
            one: true,
            ..
        })
    ));
    assert_eq!(
        found(&data, &[134], &[320]),
        None,
        "water's 0x0800 latch shadows the outfits"
    );
}

/// The stock warp-up sound, `snd ` 128, runs 134016 frames at 22050 Hz:
/// 364 Mac ticks, the d of `_ScanPlayer`'s jump-timing check, so the
/// Shuttle's lead-in (w 1.3, no jump-speed bit) lasts while t ≤ 0.35 ×
/// 364 / 1.69, 75 Mac ticks.
#[test]
#[allow(clippy::float_cmp)]
fn the_stock_warp_up_sound_lasts_364_ticks_and_the_shuttles_lead_in_75() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sound = data.warp_sound();
    assert_eq!(
        sound,
        Some(SoundLength {
            frames: 134_016,
            rate: 22_050
        })
    );
    let duration = sound_duration(sound);
    assert_eq!(duration, 364);
    let shuttle = PilotCatalog::ships(&data)
        .into_iter()
        .find(|record| record.id == ShipId(128))
        .expect("the Shuttle");
    let w = warp_factor(shuttle.flags);
    assert_eq!(w, 1.3, "no jump-speed bit");
    assert!(!held(Some(74), w, duration, false));
    assert!(held(Some(76), w, duration, false));
}
