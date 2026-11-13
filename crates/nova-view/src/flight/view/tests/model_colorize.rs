//! A ship drawn as its 3D model takes the colorize its sprite would
//! (rdm:phase/rendering-3d/phase-15-model-colorize-like-its-sprite): the
//! murk the player sees an NPC through, its ion tint over that, a gate's
//! white wash in place of both, and the player's ion tint alone; on the
//! hull's command only, its parts taking it in the renderer. On
//! [`FakeFlight`], with the `ship_models` tests' hand-built table. The
//! colorizes expected are built with [`SeenMurk::at`], [`hull_colorize`]
//! and [`gate_colorize`], whose own tests pin what they give.

use std::rc::Rc;

use nova_sim_frame::cloak::CloakShown;
use nova_sim_frame::combat::hull_turn::HullTurn;
use nova_sim_frame::combat::ion_tint::IonTint;
use nova_sim_frame::flight_frame::GateFade;
use nova_sim_frame::{RuleKey, RuleSource};

use super::faked_world::{world, world_by, world_list};
use super::ship_models::{
    StockLook, alpha_of, dart, expected_both, expected_npc, expected_player, faded, frame, models,
    npc_jitter, player_jitter, sheetless, table,
};
use super::*;
use crate::Colorize;
use crate::flight::gate_colorize::gate_colorize;
use crate::flight::ion_tint::hull_colorize;
use crate::flight::murk::SeenMurk;

/// The colour the frames murk toward: a stock one.
const MURK_COLOUR: u16 = 0x1081;

/// An ion tint deeper than the NPC's murk in a system of murk 30, and
/// shallower than its murk in one of 100.
const ION: IonTint = IonTint {
    level: 16,
    colour: 0x0034_c2ff,
};

const TURN: HullTurn = HullTurn::Continuous;

/// The `ship_models` frame (the player at the centre, NPC 0 at
/// (100, -100)) as `edit` leaves its ships, in a system of effective murk
/// `murk` toward [`MURK_COLOUR`].
fn murky(murk: u8, edit: impl FnOnce(&mut [FrameShip])) -> FlightFrame {
    FlightFrame {
        effective_murk: murk,
        murk_colour: MURK_COLOUR,
        ..frame(0.0, TURN, edit)
    }
}

/// `frame` drawn with every ship as its model, by `rules`.
fn modelled_by(frame: FlightFrame, rules: &Rulebook) -> Faked {
    world_by(sheetless(), frame, rules).with_world_models(Rc::new(table()))
}

/// `frame` drawn with every ship as its model, by the default rules.
fn modelled(frame: FlightFrame) -> Faked {
    modelled_by(frame, &Rulebook::default())
}

/// The murk's colorize of NPC 0 as `view`'s frame has the player see it.
fn npc_murk(view: &Faked) -> Colorize {
    let frame = view.drawn();
    let npc = frame.npcs().next().expect("the npc");
    SeenMurk {
        viewer: frame.player().position,
        murk: frame.effective_murk,
        colour: MURK_COLOUR,
    }
    .at(npc.position)
}

/// `commands` with their hull colorized `colorize`.
fn colorized(commands: Vec<DrawCommand>, colorize: Option<Colorize>) -> Vec<DrawCommand> {
    commands
        .into_iter()
        .map(|command| command.with_colorize(colorize))
        .collect()
}

/// The colorize of each hull model `list` draws, in order: the NPC's,
/// then the player's.
fn hull_colorizes(list: &DrawList) -> Vec<Option<Colorize>> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Model { colorize, .. } => Some(*colorize),
            _ => None,
        })
        .collect()
}

/// The NPC's commands colorized `npc` and the player's `player`, both
/// opaque, as `view` draws them by the table.
fn both(view: &Faked, npc: Option<Colorize>, player: Option<Colorize>) -> Vec<DrawCommand> {
    let mut want = colorized(expected_npc(view, TURN), npc);
    want.extend(colorized(expected_player(view, TURN), player));
    want
}

#[test]
fn a_modelled_npc_in_murk_takes_its_murks_colorize() {
    let view = modelled(murky(30, |_| {}));
    let murk = npc_murk(&view);
    assert!(murk.mixes(), "murked: {murk:?}");
    let list = world_list(&view);
    // The hull alone carries it: its parts are as the table gives them,
    // and the player, never murked, has none.
    assert_eq!(models(&list), both(&view, Some(murk), None));
    assert_eq!(hull_colorizes(&list), [Some(murk), None]);
}

#[test]
fn a_murky_npc_beside_the_player_takes_none() {
    let view = modelled(murky(100, |ships| ships[1].position = ships[0].position));
    assert_eq!(npc_murk(&view).level, 0, "beside it");
    let list = world_list(&view);
    assert_eq!(models(&list), expected_both(&view, TURN));
    assert_eq!(hull_colorizes(&list), [None, None]);
}

#[test]
fn an_ionized_modelled_npc_takes_the_ion_colorize() {
    let view = modelled(murky(0, |ships| ships[1].ion_tint = Some(ION)));
    let ion = hull_colorize(0, Some(ION), RuleSource::Engine);
    assert_eq!(ion, Some(Colorize::toward(ION.level, ION.colour)));
    assert_eq!(models(&world_list(&view)), both(&view, ion, None));
    // In a shallower murk than its tint, the tint still.
    let shallow = modelled(murky(30, |ships| ships[1].ion_tint = Some(ION)));
    assert!(npc_murk(&shallow).level < ION.level);
    assert_eq!(models(&world_list(&shallow)), both(&shallow, ion, None));
}

#[test]
fn an_ionized_modelled_npc_in_deeper_murk_takes_the_colour_its_rule_reads() {
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::IonTintMurkColour, source);
        let view = modelled_by(murky(100, |ships| ships[1].ion_tint = Some(ION)), &rules);
        let murk = npc_murk(&view).level;
        assert!(murk >= ION.level, "{murk}");
        let want = hull_colorize(murk, Some(ION), source);
        assert_ne!(want, Some(npc_murk(&view)), "{source:?}: not the murk's");
        assert_ne!(
            want,
            hull_colorize(0, Some(ION), source),
            "{source:?}: not the unmurked tint"
        );
        assert_eq!(
            models(&world_list(&view)),
            both(&view, want, None),
            "{source:?}"
        );
    }
    assert_ne!(
        hull_colorize(24, Some(ION), RuleSource::Engine),
        hull_colorize(24, Some(ION), RuleSource::Bible),
        "the readings differ"
    );
}

#[test]
fn an_ionized_modelled_player_takes_the_ion_colorize_and_never_the_murk() {
    // The player far from the NPC, so the NPC is murked and it is not.
    let view = modelled(murky(100, |ships| ships[0].ion_tint = Some(ION)));
    let ion = hull_colorize(0, Some(ION), RuleSource::Engine);
    let murk = npc_murk(&view);
    assert!(murk.level >= ION.level, "deeper than the tint");
    assert_eq!(models(&world_list(&view)), both(&view, Some(murk), ion));
    // Unionized in the same murk, it takes none.
    let plain = modelled(murky(100, |_| {}));
    assert_eq!(
        hull_colorizes(&world_list(&plain)),
        [Some(npc_murk(&plain)), None]
    );
}

#[test]
fn a_modelled_npc_at_a_gate_takes_the_gates_wash_in_place_of_murk_and_ion() {
    for gate in [
        GateFade::Entering { ticks_left: 9.0 },
        GateFade::Emerging { ticks_left: 3.0 },
    ] {
        let view = modelled(murky(100, |ships| {
            ships[1].gate = gate;
            ships[1].ion_tint = Some(ION);
        }));
        let wash = gate_colorize(gate).expect("at a gate");
        assert!(wash.mixes(), "{gate:?}");
        assert_ne!(wash, npc_murk(&view), "{gate:?}: not the murk");
        // Faded as the gate fades it, still.
        let alpha = alpha_of(CloakShown::Opaque, gate);
        assert!(alpha < 1.0, "{gate:?}");
        let mut want = colorized(
            faded(expected_npc(&view, TURN), at(0.0, 0.0), alpha),
            Some(wash),
        );
        want.extend(expected_player(&view, TURN));
        assert_eq!(models(&world_list(&view)), want, "{gate:?}");
    }
}

#[test]
fn a_modelled_npc_whose_gate_washes_nothing_takes_no_murk_either() {
    let gate = GateFade::Entering { ticks_left: 16.0 };
    assert_eq!(gate_colorize(gate).map(|wash| wash.level), Some(0));
    let view = modelled(murky(100, |ships| {
        ships[1].gate = gate;
        ships[1].ion_tint = Some(ION);
    }));
    assert!(npc_murk(&view).mixes(), "murky there");
    assert_eq!(hull_colorizes(&world_list(&view)), [None, None]);
}

#[test]
fn a_cloaked_ionized_modelled_ship_keeps_its_colorize() {
    let cloak = CloakShown::Fading(21.5);
    let view = modelled(murky(0, |ships| {
        for ship in ships.iter_mut() {
            ship.cloak = cloak;
            ship.ion_tint = Some(ION);
        }
    }));
    let ion = hull_colorize(0, Some(ION), RuleSource::Engine);
    let alpha = alpha_of(cloak, GateFade::None);
    assert!(alpha < 1.0);
    let mut want = colorized(
        faded(expected_npc(&view, TURN), npc_jitter(&view, cloak), alpha),
        ion,
    );
    want.extend(colorized(
        faded(
            expected_player(&view, TURN),
            player_jitter(&view, cloak),
            alpha,
        ),
        ion,
    ));
    assert_eq!(models(&world_list(&view)), want);
}

/// The colorize of ship sheet 2000's hull sprite as `list` draws it: none
/// when it is drawn plainly.
fn sprite_colorize(list: &DrawList) -> Option<Colorize> {
    list.iter()
        .find_map(|command| match unclipped(command) {
            DrawCommand::Colorized {
                image, colorize, ..
            } if image.id == 2000 => Some(Some(*colorize)),
            DrawCommand::Sprite { image, .. } if image.id == 2000 => Some(None),
            _ => None,
        })
        .expect("the hull's sprite")
}

#[test]
fn a_modelled_ship_takes_the_colorize_its_sprite_takes() {
    type Edit = fn(&mut [FrameShip]);
    let frames: [(&str, u8, Edit); 5] = [
        ("murk", 30, |_| {}),
        ("ion", 0, |ships| {
            ships[0].ion_tint = Some(ION);
            ships[1].ion_tint = Some(ION);
        }),
        ("ion in deeper murk", 100, |ships| {
            ships[0].ion_tint = Some(ION);
            ships[1].ion_tint = Some(ION);
        }),
        ("gate", 100, |ships| {
            ships[1].gate = GateFade::Emerging { ticks_left: 3.0 };
        }),
        ("clear", 0, |_| {}),
    ];
    for (why, murk, edit) in frames {
        let [npc_model, player_model] =
            hull_colorizes(&world_list(&modelled(murky(murk, edit))))[..]
        else {
            panic!("{why}: two hulls")
        };
        // The NPC on the stock look's sheet, the player still its model.
        let npc_sprite = modelled(murky(murk, edit)).with_ship_looks(Rc::new(StockLook));
        assert_eq!(
            sprite_colorize(&world_list(&npc_sprite)),
            npc_model,
            "{why}: the NPC"
        );
        // The player on the catalog's sheet, the NPC still its model.
        let player_sprite = world(
            FakeCatalog {
                outlines: vec![(130, dart())],
                ..trafficked(&[130], 1, 130, 3)
            },
            murky(murk, edit),
        )
        .with_world_models(Rc::new(table()));
        assert_eq!(
            sprite_colorize(&world_list(&player_sprite)),
            player_model,
            "{why}: the player"
        );
    }
}

#[test]
fn an_unmurked_unionized_ships_commands_are_unchanged() {
    let view = modelled(murky(0, |_| {}));
    let list = world_list(&view);
    assert_eq!(models(&list), expected_both(&view, TURN));
    assert_eq!(hull_colorizes(&list), [None, None]);
}
