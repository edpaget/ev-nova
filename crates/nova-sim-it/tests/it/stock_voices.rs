//! The escorts' voices over the stock data (rdm escort-fleet-fidelity
//! phase 10): the clips `_LoadSounds` counts for each voice set and
//! message (@0x1c03f-0x1c0b3), the governments' `VoiceType`s, and the
//! ships of `Flags2` 0x0010 that speak none. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use nova_sim::combat::hull::HullSpec;
use nova_sim::voice::{GovtVoice, VoiceMessage, VoiceVariant};
use nova_sim::{CombatCatalog, GovtId, PilotCatalog, ShipId};

#[test]
fn the_stock_voice_sets_hold_the_clips_load_sounds_counts() {
    let Some(data) = common::opened() else {
        return;
    };
    let voices = data.voices();
    // [acknowledge, target, victory] for sets 0-7: Civ_Ind, Federation,
    // Rebel, Polaris, Auroran, Pirate, Wild Geese, and none.
    let counts: Vec<[usize; 3]> = (0..8)
        .map(|set| VoiceMessage::ALL.map(|message| voices.table.count(set, message)))
        .collect();
    assert_eq!(
        counts,
        [
            [7, 5, 3],
            [7, 5, 5],
            [9, 5, 3],
            [5, 3, 5],
            [8, 8, 8],
            [8, 8, 8],
            [8, 8, 8],
            [0, 0, 0],
        ]
    );
}

#[test]
fn the_stock_governments_voices_are_their_voice_types() {
    let Some(data) = common::opened() else {
        return;
    };
    let voices = data.voices();
    assert_eq!(
        voices.govts.get(&GovtId(128)),
        Some(&GovtVoice {
            set: Some(1),
            forced: None
        })
    );
    assert_eq!(
        voices.govts.get(&GovtId(182)),
        Some(&GovtVoice {
            set: Some(4),
            forced: Some(VoiceVariant::Even)
        })
    );
    let forced_even: Vec<GovtId> = voices
        .govts
        .iter()
        .filter(|(_, voice)| voice.forced == Some(VoiceVariant::Even))
        .map(|(&id, _)| id)
        .collect();
    assert_eq!(forced_even, [GovtId(182), GovtId(188), GovtId(189)]);
    assert_eq!(voices.set_of(None), Some(0), "no attributes government");
}

#[test]
fn the_stock_ships_of_flags2_0x0010_are_the_drones_and_the_vell_os_craft() {
    let Some(data) = common::opened() else {
        return;
    };
    let mute: Vec<ShipId> = data
        .hulls()
        .iter()
        .filter(|record| HullSpec::new(record).mute)
        .map(|record| record.id)
        .collect();
    assert_eq!(mute, [130, 173, 174, 175, 381, 382, 383, 393].map(ShipId));
}
