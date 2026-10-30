//! The ships drawn as `_HandleShipDisplay` draws them (rdm
//! ship-handling-fidelity phase 10): their banked, folded and animated
//! sets, the NPCs' engine glow, running lights and murk, lights out while
//! disabled, the alt layer, the shield layer's flash and the hull's
//! `BaseTransp`. On [`FakeFlight`]: each ship's record says what its
//! display picked (its sets, alt, lights, glow base, cloak, ion tint and
//! gate fade), as the session's display steps them (nova-sim-combat's
//! `ship_frames`, `ship_lights` and nova-sim's `glow`, named where a test
//! leans on them); the view draws what the record says.

use super::faked_world::{shows, world_by};
use super::*;
use crate::flight::murk::{murk_tint, sprite_murk};
use crate::flight::ship_layers::{lights_murk, npc_glow_rolls};
use crate::flight::weapon_glow::npc_glow_level;
use nova_sim::catalog::{ShipDisplay, ShipExits};
use nova_sim::cloak::CloakShown;
use nova_sim::combat::ship_frames::{ANIMATED, Alt, BANKS, Drawn, FrameSpec};
use nova_sim::flight_frame::{FrameShip, PassReport, StepReport, StepRun};

/// A `shän` display of `flags`, `sets` base sets and `AnimDelay` `delay`,
/// its lights sprite steady.
pub(super) fn display(flags: u16, sets: i16, delay: i16) -> ShipDisplay {
    ShipDisplay {
        flags,
        base_sets: sets,
        anim_delay: delay,
        lights_image: true,
        ..ShipDisplay::default()
    }
}

/// Ship `id`'s hull of `display`, `frames` a turn, carrying `weapons`.
fn shown_hull(
    id: i16,
    display: ShipDisplay,
    frames: i16,
    weapons: &[WeaponId],
) -> nova_sim::HullRecord {
    nova_sim::HullRecord {
        display: Some(display),
        exits: Some(ShipExits {
            frames,
            ..ShipExits::default()
        }),
        ..hull_of(id, weapons)
    }
}

/// Each sprite of `image` drawn: its frame, tint and whether it is added.
fn drawn_of(view: &Faked, image: i16) -> Vec<(u16, Color, bool)> {
    super::faked_world::world_list(view)
        .iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((key.frame, tint, blend == crate::draw::Blend::Or)),
            _ => None,
        })
        .collect()
}

/// The frames of `image` drawn.
fn frames_of(view: &Faked, image: i16) -> Vec<u16> {
    drawn_of(view, image)
        .into_iter()
        .map(|(frame, ..)| frame)
        .collect()
}

/// The flight's tick now, which the glow and lights roll by.
fn now(view: &Faked) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).unwrap_or(u64::MAX)
}

/// [`trafficked`] with NPC ship 129 of `npc` (72 frames a turn) and the
/// player's 128 of `player` (36), the NPC's sheet `sheet` when given.
pub(super) fn shown(
    player: ShipDisplay,
    npc: ShipDisplay,
    sheet: Option<ShipSheet>,
) -> FakeCatalog {
    FakeCatalog {
        hulls: vec![
            shown_hull(128, player, 36, &[]),
            shown_hull(129, npc, 72, &[]),
        ],
        npc_sheet: sheet,
        ..trafficked(&[130], 1, 129, 3)
    }
}

/// The NPC's sheet: `rlëD` 2001 of 72 rotations, glow 2101, lights 2201.
pub(super) fn npc_sheet() -> ShipSheet {
    ShipSheet {
        image_id: 2001,
        rotations: NonZeroU16::new(72).expect("non-zero"),
        glow: Some(layer(2101, 72)),
        lights: Some(layer(2201, 72)),
        ..sheet()
    }
}

/// A hull's frames, `frames` a turn, with a lights layer.
fn turning_by(frames: u16) -> FrameSpec {
    FrameSpec {
        frames: NonZeroU16::new(frames).expect("non-zero"),
        lights: true,
        ..FrameSpec::NONE
    }
}

/// The player's record in a [`shown`] flight: ship 128 at the centre,
/// facing up, 36 frames a turn, on set 0 with its steady lights at 32, its
/// glow out.
fn player_shown() -> FrameShip {
    FrameShip {
        frames: turning_by(36),
        drawn: Drawn {
            lights: Some(32),
            ..Drawn::default()
        },
        ..the_player_at(Vec2::ZERO, 0.0)
    }
}

/// NPC 0's record in a [`shown`] flight: ship 129 at (100, -100) facing
/// `heading`, 72 frames a turn, on set 0 with its steady lights at 32,
/// its glow at its full 32.
fn npc_shown(heading: f32) -> FrameShip {
    FrameShip {
        frames: turning_by(72),
        drawn: Drawn {
            lights: Some(32),
            ..Drawn::default()
        },
        glow: 32,
        ..npc_129(0, Vec2::new(100.0, -100.0), heading)
    }
}

/// A scripted flight from `catalog` by `rules`: the player and NPC 0 as
/// [`player_shown`] and [`npc_shown`] have them, the NPC facing
/// `heading`, in a system of effective murk `murk`, a tick in.
pub(super) fn shown_by(catalog: FakeCatalog, heading: f32, murk: u8, rules: &Rulebook) -> Faked {
    let mut frame = in_sol_with(vec![player_shown(), npc_shown(heading)]);
    frame.effective_murk = murk;
    let mut view = world_by(catalog, frame, rules);
    view.elapsed = TICK;
    view
}

/// [`shown_by`] Nova's rules, the system clear.
pub(super) fn flying_with(catalog: FakeCatalog, heading: f32) -> Faked {
    shown_by(catalog, heading, 0, &Rulebook::default())
}

/// Shows `view`'s frame with ship `at` (0 the player, 1 the NPC) changed
/// by `change`.
pub(super) fn redrawn(view: &mut Faked, at: usize, change: impl FnOnce(&mut FrameShip)) {
    let mut frame = view.frame.clone();
    change(&mut frame.ships[at]);
    shows(view, frame);
}

/// NPC 0's id.
pub(super) const NPC_0: NpcId = NpcId(0);

#[test]
fn the_player_is_drawn_on_the_set_its_display_picked_banked_or_level() {
    // Which set, and when: the session's display (nova-sim-combat's
    // `holding_right_banks_the_player_onto_set_2_past_6_and_letting_go_levels_it`
    // and `a_banking_ship_draws_set_1_banked_left_and_2_banked_right`).
    let catalog = shown(display(BANKS, 3, 0), display(0, 1, 0), None);
    let mut view = flying_with(catalog, 0.0);
    for set in [0, 2, 0, 1] {
        redrawn(&mut view, 0, |player| player.drawn.set = set);
        let frame = frames_of(&view, 2000)[0];
        assert_eq!(Some(frame), view.frame());
        assert_eq!(frame / 36, set, "set {set}");
    }
}

#[test]
fn an_npc_is_drawn_on_the_set_its_display_picked() {
    // Banked past 4 as it turns: the session's (nova-sim-combat's
    // `a_banking_ship_draws_set_1_banked_left_and_2_banked_right`).
    let catalog = shown(display(0, 1, 0), display(BANKS, 3, 0), Some(npc_sheet()));
    let mut view = flying_with(catalog, 270.0);
    for set in [0, 2, 1] {
        redrawn(&mut view, 1, |npc| npc.drawn.set = set);
        let frames = frames_of(&view, 2001);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0] / 72, set, "set {set}");
    }
}

#[test]
fn an_npcs_glow_is_drawn_at_its_base_held_below_its_murk_and_murked() {
    // The base under thrust and coasting is the session's (nova-sim's
    // `thrust_raises_the_base_to_cruise_and_coasting_lowers_it_to_zero`).
    let catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet()));
    let mut view = shown_by(catalog, 0.0, 30, &Rulebook::default());
    redrawn(&mut view, 1, |npc| npc.glow = 29);
    let npc = view.frame.ships[1];
    let murk = sprite_murk(npc.position, Vec2::ZERO, 30);
    assert!(murk > 0, "a murky system");
    let level = glow_level(29, now(&view), &npc_glow_rolls(NPC_0)).expect("lit");
    let tint = murk_tint(lights_tint(npc_glow_level(level, murk)), murk);
    let frame = frames_of(&view, 2001)[0];
    assert_eq!(
        drawn_of(&view, 2101),
        [(frame, tint, true)],
        "held below its murk, murked"
    );
    let hull = drawn_of(&view, 2001)[0].1;
    assert_eq!(hull, murk_tint(Color::WHITE, murk), "its hull murked too");
    let lights_murk = lights_murk(murk);
    let lights = murk_tint(lights_tint(npc_glow_level(32, lights_murk)), lights_murk);
    assert_eq!(
        drawn_of(&view, 2201),
        [(frame, lights, true)],
        "its steady lights a third as murky"
    );
}

#[test]
fn a_ship_whose_glow_base_is_one_or_none_shows_no_glow() {
    let mut view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet())),
        0.0,
    );
    for base in [1, 0] {
        redrawn(&mut view, 1, |npc| npc.glow = base);
        assert_eq!(drawn_of(&view, 2101), [], "hidden at 1 or less: {base}");
    }
}

/// The stock triangle, 10 to 31.
const TRIANGLE: Blink = Blink {
    mode: 2,
    a: 10,
    b: 75,
    c: 32,
    d: 75,
};

/// The player's and the NPC's lights as drawn: each tint, if drawn.
fn lights_drawn(view: &Faked) -> (Option<Color>, Option<Color>) {
    let tint = |image| drawn_of(view, image).first().map(|&(_, tint, _)| tint);
    (tint(2200), tint(2201))
}

/// Shows `view`'s player and NPC with their lights at `player` and `npc`.
fn lit_at(view: &mut Faked, player: Option<u8>, npc: Option<u8>) {
    redrawn(view, 0, |ship| ship.drawn.lights = player);
    redrawn(view, 1, |ship| ship.drawn.lights = npc);
}

#[test]
fn each_ships_lights_are_drawn_at_the_level_its_display_drew_them_or_not_at_all() {
    // Out while disabled and climbing back from nothing once repaired:
    // the session's (nova-sim-combat's
    // `the_lights_go_out_on_a_disabled_ship_of_0x0040` and
    // `a_put_out_triangle_climbs_back_from_nothing`).
    let lit = ShipDisplay {
        blink: TRIANGLE,
        ..display(0x0040, 1, 0)
    };
    let mut catalog = shown(lit, lit, Some(npc_sheet()));
    catalog.sheet = Ok(ShipSheet {
        lights: Some(layer(2200, 36)),
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    lit_at(&mut view, Some(23), Some(22));
    let rising = (Some(lights_tint(23)), Some(lights_tint(22)));
    assert_eq!(lights_drawn(&view), rising, "rising, intact");
    lit_at(&mut view, None, None);
    assert_eq!(lights_drawn(&view), (None, None), "both out");
    lit_at(&mut view, Some(1), Some(1));
    let one = Some(lights_tint(1));
    assert_eq!(lights_drawn(&view), (one, one), "climbing from nothing");
}

/// Runs a frame of `view` whose one pass raises `combat` with its
/// frame's ships as they stand.
fn pass_raising(view: &mut Faked, combat: Vec<CombatEvent>) {
    let frame = view.frame.clone();
    let pass = PassReport {
        ships: frame.ships.clone(),
        combat,
        player_position: frame.player().position,
        ..PassReport::default()
    };
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(
        1,
        crate::system::camera::Anchor::in_flight(RuleSource::Engine),
    );
}

#[test]
fn a_plug_in_shield_image_flashes_on_a_hit_and_fades() {
    // What the hit does to the shield is the session's; the view flashes
    // the layer from the hit the pass reports.
    let catalog = FakeCatalog {
        npc_sheet: Some(ShipSheet {
            shield: Some(layer(2501, 72)),
            ..npc_sheet()
        }),
        ..armed(1, &[BLASTER])
    };
    let mut view = flying_with(catalog, 180.0);
    let npc = ShipRef::Npc(NPC_0);
    assert_eq!(drawn_of(&view, 2501), [], "unhit");
    redrawn(&mut view, 1, |npc| {
        npc.shield = nova_sim::Gauge {
            now: 39.0,
            max: 40.0,
        };
    });
    let hit = CombatEvent::Hit {
        ship: npc,
        weapon: Some(BLASTER),
        at: Vec2::new(100.0, -100.0),
        passes_shields: false,
        late: false,
    };
    pass_raising(&mut view, vec![hit]);
    let hull = frames_of(&view, 2001)[0];
    let level = view.flashes[&npc].drawn().expect("drawn");
    assert!(
        (30..=32).contains(&level),
        "held to its shield's share: {level}"
    );
    let flashed = drawn_of(&view, 2501);
    assert_eq!(
        flashed,
        [(hull, lights_tint(level), true)],
        "the hull's frame"
    );
    pass_raising(&mut view, Vec::new());
    assert_eq!(drawn_of(&view, 2501)[0].1, lights_tint(level - 1), "fading");
    for _ in 0..40 {
        pass_raising(&mut view, Vec::new());
    }
    assert_eq!(view.shield_flash(npc), None);
    assert_eq!(drawn_of(&view, 2501), [], "run out");
}

#[test]
fn an_animated_npc_is_drawn_on_each_set_its_display_steps_to() {
    // A set a frame at AnimDelay 0: the session's (nova-sim-combat's
    // `an_animated_ship_cycles_its_sets_every_anim_delay`).
    let catalog = shown(display(0, 1, 0), display(ANIMATED, 3, 0), Some(npc_sheet()));
    let mut view = flying_with(catalog, 90.0);
    let mut sets = Vec::new();
    for set in [1, 2, 0, 1, 2] {
        redrawn(&mut view, 1, |npc| npc.drawn.set = set);
        sets.push(frames_of(&view, 2001)[0] / 72);
    }
    assert_eq!(sets, [1, 2, 0, 1, 2]);
}

#[test]
fn the_alt_layer_is_drawn_at_its_own_set_with_the_hulls_tint() {
    // Every 6 frames: the session's (nova-sim-combat's
    // `the_alt_layer_steps_on_its_own_timer_once_a_frame_off_the_0x0008_path`).
    let mut catalog = shown(
        ShipDisplay {
            alt_sets: 6,
            alt_image: true,
            ..display(0x0058, 1, 5)
        },
        display(0, 1, 0),
        None,
    );
    catalog.sheet = Ok(ShipSheet {
        alt: Some(layer(2400, 216)),
        base_transp: 10,
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    let hull = drawn_of(&view, 2000)[0];
    assert_eq!(
        hull.1.a,
        crate::flight::ship_layers::base_alpha(10),
        "BaseTransp 10"
    );
    let mut alt_sets = Vec::new();
    for set in [0, 0, 1] {
        redrawn(&mut view, 0, |player| player.drawn.alt = Alt::Drawn(set));
        let alt = drawn_of(&view, 2400);
        assert_eq!(alt.len(), 1);
        assert!(!alt[0].2, "drawn plainly");
        assert_eq!(alt[0].1, drawn_of(&view, 2000)[0].1, "with the hull's tint");
        alt_sets.push(alt[0].0 / 36);
    }
    assert_eq!(alt_sets, [0, 0, 1]);
}

/// Each sprite of `image` drawn: its frame and centre.
fn placed_of(view: &Faked, image: i16) -> Vec<(u16, Point)> {
    sprites(&super::faked_world::world_list(view))
        .into_iter()
        .filter(|(key, _)| key.id == image)
        .map(|(key, at)| (key.frame, at))
        .collect()
}

/// A flight whose NPC is of 0x0020 with a two-set plug-in alt layer
/// (`rlëD` 2401), drawn on alt set 0.
fn holding_alt() -> Faked {
    let npc = ShipDisplay {
        alt_sets: 2,
        alt_image: true,
        ..display(0x0020, 1, 0)
    };
    let sheet = ShipSheet {
        alt: Some(layer(2401, 144)),
        ..npc_sheet()
    };
    let catalog = shown(display(0, 1, 0), npc, Some(sheet));
    let mut view = flying_with(catalog, 0.0);
    redrawn(&mut view, 1, |npc| npc.drawn.alt = Alt::Drawn(0));
    view
}

#[test]
fn a_disabled_0x0020_npcs_alt_stays_where_it_was_drawn_as_the_player_flies_on() {
    // Held while disabled by the engine's reading: the session's
    // (nova-sim-combat's `a_disabled_0x0020_ship_holds_its_alt_set_and_timer`);
    // the view keeps it where it last drew it.
    let mut view = holding_alt();
    let before = placed_of(&view, 2401);
    assert_eq!(before.len(), 1, "drawn live");
    assert_eq!(before[0].1, placed_of(&view, 2001)[0].1, "on its hull");
    redrawn(&mut view, 1, |npc| npc.drawn.alt = Alt::Held);
    redrawn(&mut view, 0, |player| {
        player.position = Vec2::new(0.0, -60.0);
    });
    view.previous = view.frame.player().place();
    let hull = placed_of(&view, 2001)[0].1;
    assert_ne!(hull, before[0].1, "the view moved on");
    assert_eq!(placed_of(&view, 2401), before, "its alt held on screen");
    // Repaired, it is drawn live on its hull again.
    redrawn(&mut view, 1, |npc| npc.drawn.alt = Alt::Drawn(0));
    let hull = placed_of(&view, 2001)[0].1;
    assert_eq!(placed_of(&view, 2401)[0].1, hull);
}

#[test]
fn an_npc_whose_alt_shows_none_draws_no_alt_layer() {
    // A disabled 0x0020 ship's alt is hidden by the other reading:
    // the session's (nova-sim-combat's
    // `by_the_other_reading_a_disabled_0x0020_ships_alt_layer_is_hidden`).
    let mut view = holding_alt();
    assert_eq!(placed_of(&view, 2401).len(), 1);
    redrawn(&mut view, 1, |npc| {
        npc.drawn.alt = Alt::None;
        npc.condition = Condition::Disabled;
    });
    assert_eq!(placed_of(&view, 2401), [], "hidden");
    assert_eq!(placed_of(&view, 2001).len(), 1, "its hull still drawn");
}

/// The hull of `npc`'s record as `view` draws it: its frame's tint, if
/// drawn.
fn npc_hull(view: &Faked, npc: &FrameShip) -> Option<Color> {
    let mut list = DrawList::new();
    view.draw_npc(&mut list, npc, &|_| VIEW_CENTER, npc.heading);
    list.iter().find_map(|command| match *command {
        DrawCommand::Sprite { image, tint, .. } | DrawCommand::Colorized { image, tint, .. }
            if image.id == 2001 =>
        {
            Some(tint)
        }
        _ => None,
    })
}

#[test]
fn an_npcs_base_transp_gives_way_to_its_gate_fade_and_cloak() {
    let sheet = ShipSheet {
        base_transp: 10,
        ..npc_sheet()
    };
    let view = flying_with(shown(display(0, 1, 0), display(0, 1, 0), Some(sheet)), 0.0);
    let base = crate::flight::ship_layers::base_alpha(10);
    let mut npc = npc_record(&view, NPC_0);
    assert_eq!(
        npc_hull(&view, &npc).map(|tint| tint.a),
        Some(base),
        "opaque: 22/32"
    );
    // Waiting at its gate, fading in, its fade's alpha in place of it.
    for ticks_left in [1, 3] {
        npc.gate = GateFade::Emerging { ticks_left };
        let faded = npc_tint(npc.gate).expect("shown").a;
        assert_ne!(faded, base, "{ticks_left}");
        assert_eq!(
            npc_hull(&view, &npc).map(|tint| tint.a),
            Some(faded),
            "{ticks_left}"
        );
    }
    npc.gate = GateFade::Entering { ticks_left: 8 };
    let entering = npc_tint(npc.gate).expect("shown").a;
    assert_ne!(entering, base);
    assert_eq!(npc_hull(&view, &npc).map(|tint| tint.a), Some(entering));
    // Fully cloaked, its cloak decides: hidden, or, led by the player,
    // faint at its full fade's translucency (the session's: nova-sim's
    // `a_fully_cloaked_ship_is_faint_when_led_or_scanned_at_any_range_and_hidden_otherwise`).
    npc.gate = GateFade::None;
    npc.cloak = CloakShown::Hidden;
    assert_eq!(npc_hull(&view, &npc), None);
    npc.cloak = CloakShown::Translucent(0x1e);
    assert_eq!(
        npc_hull(&view, &npc),
        Some(crate::draw::translucent_tint(4, 30)),
        "at 4 over 30, not its BaseTransp"
    );
}

#[test]
fn a_fading_npc_is_jittered_within_its_reach_and_a_hidden_unlit_one_draws_nothing() {
    let view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet())),
        0.0,
    );
    let mut npc = npc_record(&view, NPC_0);
    npc.cloak = CloakShown::Fading(25.0);
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    let hull = list.iter().find_map(|command| match *command {
        DrawCommand::Sprite {
            image,
            center,
            tint,
            blend,
        } if image.id == 2001 => Some((center, tint, blend)),
        _ => None,
    });
    let (center, tint, blend) = hull.expect("drawn");
    assert_eq!((tint, blend), (lights_tint(7), crate::draw::Blend::Or));
    assert!(
        (center.x - VIEW_CENTER.x).abs() <= 2.0 && (center.y - VIEW_CENTER.y).abs() <= 2.0,
        "{center:?}"
    );
    // A hidden full fade draws no hull, glow or lights.
    npc.cloak = CloakShown::Hidden;
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    assert_eq!(list.iter().count(), 0, "no weapon glow lit");
}

#[test]
fn a_hidden_cloaked_npc_draws_its_lit_weapon_glow_alone_and_one_at_its_gate_nothing() {
    let mut view = flying_with(
        shown(
            display(0, 1, 0),
            display(0, 1, 0),
            Some(ShipSheet {
                weapon: Some(layer(2301, 72)),
                ..npc_sheet()
            }),
        ),
        0.0,
    );
    let mut npc = npc_record(&view, NPC_0);
    view.glows.step(&[(npc.ship, false)], |_| Some(1.0));
    let level = view.glows.drawn(npc.ship).expect("lit");
    npc.cloak = CloakShown::Hidden;
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    let drawn: Vec<_> = list
        .iter()
        .map(|command| match *command {
            DrawCommand::Sprite {
                image,
                center,
                tint,
                blend,
            } => Some((image.id, center, tint, blend)),
            _ => None,
        })
        .collect();
    assert_eq!(
        drawn,
        [Some((
            2301,
            VIEW_CENTER,
            lights_tint(level.min(8)),
            crate::draw::Blend::Or
        ))],
        "the weapon glow alone, unjittered"
    );
    // Waiting at its gate, uncloaked, it draws nothing, lit glow or not.
    npc.cloak = CloakShown::Opaque;
    npc.gate = GateFade::Emerging { ticks_left: 100 };
    assert_eq!(npc_tint(npc.gate), None);
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    assert_eq!(list.iter().count(), 0, "hidden at its gate");
}

/// Each colorized sprite of `image` drawn: its tint, whether it is OR-ed
/// in, and its colorize.
fn colorized_of(list: &DrawList, image: i16) -> Vec<(Color, bool, crate::Colorize)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Colorized {
                image: key,
                tint,
                blend,
                colorize,
                ..
            } if key.id == image => Some((tint, blend == crate::draw::Blend::Or, colorize)),
            _ => None,
        })
        .collect()
}

/// NPC 0's record, changed by `change`, drawn by `view` with `tint`, at
/// `at` from the player: the list, and its murk there.
fn npc_drawn_ionized(
    view: &Faked,
    change: impl FnOnce(&mut FrameShip),
    tint: Option<nova_sim::combat::ion_tint::IonTint>,
    at: nova_sim::Vec2,
) -> (DrawList, u8) {
    let player = view.frame.player().position;
    let mut npc = npc_record(view, NPC_0);
    change(&mut npc);
    npc.position = player + at;
    npc.ion_tint = tint;
    let murk = sprite_murk(npc.position, player, view.frame.effective_murk);
    let mut list = DrawList::new();
    view.draw_npc(&mut list, &npc, &|_| VIEW_CENTER, npc.heading);
    (list, murk)
}

const ION: nova_sim::combat::ion_tint::IonTint = nova_sim::combat::ion_tint::IonTint {
    level: 16,
    colour: 0x0034_c2ff,
};

/// Has `ship`'s shield flash in `view` on a hit's first frame at a full
/// shield: the level its layer is drawn at.
fn flashing(view: &mut Faked, ship: ShipRef) -> u8 {
    let mut flash = crate::flight::flash::ShieldFlash::default();
    flash.hit(false);
    let level = flash.step(100.0, 100.0, true).expect("drawn");
    view.flashes.insert(ship, flash);
    level
}

/// Each sprite of `image` drawn plainly in `list`: its tint and whether
/// it is OR-ed in.
fn plain_of(list: &DrawList, image: i16) -> Vec<(Color, bool)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((tint, blend == crate::draw::Blend::Or)),
            _ => None,
        })
        .collect()
}

/// The NPC's sheet with an alt layer (`rlëD` 2400) and a shield
/// (`rlëD` 2500), `BaseTransp` 10.
fn ionized_sheet() -> ShipSheet {
    ShipSheet {
        alt: Some(layer(2400, 72)),
        shield: Some(layer(2500, 72)),
        base_transp: 10,
        ..npc_sheet()
    }
}

/// A murky flight whose NPC shows an alt layer and its lights.
fn murky_ionizing(rules: Option<nova_sim::Rulebook>) -> Faked {
    let catalog = shown(
        display(0, 1, 0),
        ShipDisplay {
            alt_sets: 1,
            alt_image: true,
            ..display(0, 1, 0)
        },
        Some(ionized_sheet()),
    );
    let rules = rules.unwrap_or_default();
    let mut view = shown_by(catalog, 0.0, 30, &rules);
    redrawn(&mut view, 1, |npc| npc.drawn.alt = Alt::Drawn(0));
    view
}

#[test]
fn an_ionized_npc_draws_its_hull_and_layers_colorized_in_its_ion_colour_at_its_unmurked_tint() {
    let mut view = murky_ionizing(None);
    let shield = flashing(&mut view, ShipRef::Npc(NPC_0));
    let (list, murk) = npc_drawn_ionized(
        &view,
        |npc| npc.drawn.alt = Alt::Drawn(0),
        Some(ION),
        nova_sim::Vec2::new(100.0, -100.0),
    );
    assert!(murk > 0 && murk < 16, "{murk}");
    let colorize = crate::Colorize::toward(16, 0x0034_c2ff);
    let unmurked = Color {
        a: crate::flight::ship_layers::base_alpha(10),
        ..Color::WHITE
    };
    assert_eq!(
        colorized_of(&list, 2001),
        [(unmurked, false, colorize)],
        "the hull"
    );
    assert_eq!(
        colorized_of(&list, 2400),
        [(unmurked, false, colorize)],
        "the alt layer"
    );
    assert_eq!(
        colorized_of(&list, 2500),
        [(lights_tint(shield), true, colorize)],
        "the shield, at its own level unmurked, in the hull's colorize"
    );
    assert_eq!(plain_of(&list, 2500), [], "drawn colorized alone");
    // Its glow and lights copy the hull's colorize, the lights a third as
    // deep, each held below it.
    let glow = colorized_of(&list, 2101);
    assert_eq!(glow.len(), 1);
    assert_eq!(glow[0].2, colorize);
    assert!(glow[0].1);
    let lights = lights_murk(16);
    assert_eq!(
        colorized_of(&list, 2201),
        [(
            lights_tint(npc_glow_level(32, lights)),
            true,
            crate::Colorize {
                level: lights,
                ..colorize
            }
        )]
    );
}

#[test]
fn an_ionized_npcs_glow_is_held_below_its_ion_level() {
    let view = murky_ionizing(None);
    let (list, _) = npc_drawn_ionized(
        &view,
        |npc| npc.glow = 24,
        Some(ION),
        nova_sim::Vec2::new(100.0, -100.0),
    );
    let level = glow_level(24, now(&view), &npc_glow_rolls(NPC_0)).expect("lit");
    assert!(level > npc_glow_level(level, 16), "the hold is exercised");
    assert_eq!(
        colorized_of(&list, 2101),
        [(
            lights_tint(npc_glow_level(level, 16)),
            true,
            crate::Colorize::toward(16, 0x0034_c2ff)
        )]
    );
}

#[test]
fn an_ionized_npcs_weapon_glow_takes_the_hulls_colorize_held_below_it() {
    let mut view = murky_with(ShipSheet {
        weapon: Some(layer(2301, 72)),
        ..ionized_sheet()
    });
    lit_at_28(&mut view, NPC_0);
    let (list, _) = npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    assert!(28 > npc_glow_level(28, 16), "the hold is exercised");
    assert_eq!(
        colorized_of(&list, 2301),
        [(
            lights_tint(npc_glow_level(28, 16)),
            true,
            crate::Colorize::toward(16, 0x0034_c2ff)
        )]
    );
    assert_eq!(plain_of(&list, 2301), []);
}

#[test]
fn an_npc_below_a_third_ionized_draws_as_before() {
    let mut view = murky_ionizing(None);
    let shield = flashing(&mut view, ShipRef::Npc(NPC_0));
    let at = nova_sim::Vec2::new(100.0, -100.0);
    let (plain, murk) = npc_drawn_ionized(&view, |_| {}, None, at);
    assert!(colorized_of(&plain, 2001).is_empty());
    assert!(colorized_of(&plain, 2500).is_empty());
    assert_eq!(
        plain_of(&plain, 2500),
        [(murk_tint(lights_tint(shield), murk), true)],
        "its shield murked"
    );
    let hull = plain.iter().find_map(|command| match *command {
        DrawCommand::Sprite { image, tint, .. } if image.id == 2001 => Some(tint),
        _ => None,
    });
    let unmurked = Color {
        a: crate::flight::ship_layers::base_alpha(10),
        ..Color::WHITE
    };
    assert_eq!(hull, Some(murk_tint(unmurked, murk)), "murked");
}

#[test]
fn a_murkier_ionized_npc_takes_32_less_its_murk_as_a_pixel_by_the_engine() {
    let view = murky_ionizing(None);
    let (list, murk) =
        npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(300.0, -300.0));
    assert!(murk >= 16, "{murk}");
    let unmurked = Color {
        a: crate::flight::ship_layers::base_alpha(10),
        ..Color::WHITE
    };
    let colorize = crate::Colorize::toward_pixel(murk, u16::from(32 - murk));
    assert_eq!(colorize.rgb5, [0, 0, 32 - murk]);
    assert_eq!(colorized_of(&list, 2001), [(unmurked, false, colorize)]);
}

#[test]
fn a_murkier_ionized_npc_takes_its_ion_colour_at_its_murk_by_the_other_reading() {
    let rules = nova_sim::Rulebook::default().with_override(
        nova_sim::RuleKey::IonTintMurkColour,
        nova_sim::RuleSource::Bible,
    );
    let view = murky_ionizing(Some(rules));
    let (list, murk) =
        npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(300.0, -300.0));
    let unmurked = Color {
        a: crate::flight::ship_layers::base_alpha(10),
        ..Color::WHITE
    };
    assert_eq!(
        colorized_of(&list, 2001),
        [(unmurked, false, crate::Colorize::toward(murk, 0x0034_c2ff))]
    );
}

/// The alpha a fade `level` 32nds draws at, rounded to nearest.
fn fade_alpha(level: u16) -> u8 {
    u8::try_from((level * 255 + 16) / 32).expect("at most 255")
}

#[test]
fn an_ionized_npc_at_its_gate_shows_the_gates_white_colorize_not_its_tint() {
    let view = murky_ionizing(None);
    let (list, _) = npc_drawn_ionized(
        &view,
        |npc| npc.gate = GateFade::Emerging { ticks_left: 3 },
        Some(ION),
        nova_sim::Vec2::new(100.0, -100.0),
    );
    // A fade of 26: washed white at min(2 × 6, 32).
    let faded = Color {
        a: fade_alpha(26),
        ..Color::WHITE
    };
    assert_eq!(
        colorized_of(&list, 2001),
        [(faded, false, crate::Colorize::toward_pixel(12, 0xffff))]
    );
}

#[test]
fn an_npc_entering_a_gate_is_colorized_white_and_its_glow_held_out() {
    let mut view = murky_with(ShipSheet {
        weapon: Some(layer(2301, 72)),
        ..ionized_sheet()
    });
    lit_at_28(&mut view, NPC_0);
    let (list, _) = npc_drawn_ionized(
        &view,
        |npc| npc.gate = GateFade::Entering { ticks_left: 4 },
        None,
        nova_sim::Vec2::new(100.0, -100.0),
    );
    let white = crate::Colorize::toward_pixel(32, 0xffff);
    let faded = Color {
        a: fade_alpha(8),
        ..Color::WHITE
    };
    assert_eq!(
        colorized_of(&list, 2001),
        [(faded, false, white)],
        "the hull"
    );
    // The glow and weapon glow copy it and are held to none below it; the
    // lights take a third of it.
    for image in [2101, 2301] {
        assert_eq!(
            colorized_of(&list, image),
            [(lights_tint(0), true, white)],
            "{image}"
        );
    }
    let lights = lights_murk(32);
    assert_eq!(
        colorized_of(&list, 2201),
        [(
            lights_tint(npc_glow_level(32, lights)),
            true,
            crate::Colorize {
                level: lights,
                ..white
            }
        )]
    );
}

#[test]
fn a_murked_npc_at_its_gate_takes_the_gates_colorize_in_place_of_its_murk() {
    let view = murky_ionizing(None);
    let (list, murk) = npc_drawn_ionized(
        &view,
        |npc| npc.gate = GateFade::Emerging { ticks_left: 3 },
        None,
        nova_sim::Vec2::new(100.0, -100.0),
    );
    assert!(murk > 0, "murked");
    let faded = Color {
        a: fade_alpha(26),
        ..Color::WHITE
    };
    assert_eq!(
        colorized_of(&list, 2001),
        [(faded, false, crate::Colorize::toward_pixel(12, 0xffff))]
    );
    assert_eq!(plain_of(&list, 2001), [], "not murked");
}

#[test]
fn an_ionized_player_draws_its_hull_colorized_and_its_glow_plainly() {
    // The tint its charge gives is the session's (nova-sim's
    // `the_players_display_tints_it_and_clears_a_spent_colour`).
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet()));
    catalog.sheet = Ok(ShipSheet {
        glow: Some(layer(2100, 36)),
        shield: Some(layer(2300, 36)),
        weapon: Some(layer(2310, 36)),
        ..sheet()
    });
    let mut view = flying_with(catalog, 0.0);
    lit_at_28_ship(&mut view, ShipRef::Player);
    let tint = nova_sim::combat::ion_tint::IonTint {
        level: 16,
        colour: 0x0000_80ff,
    };
    redrawn(&mut view, 0, |player| {
        player.ion_tint = Some(tint);
        player.glow = 24;
    });
    let list = super::faked_world::world_list(&view);
    let hull = colorized_of(&list, 2000);
    assert_eq!(hull.len(), 1);
    assert_eq!(hull[0].2, crate::Colorize::toward(tint.level, 0x0000_80ff));
    assert_eq!(hull[0].0, Color::WHITE, "never murked");
    assert!(colorized_of(&list, 2100).is_empty(), "its glow plain");
    // Its glow and weapon glow at full strength, unmurked by the ion level,
    // as the original keeps the player's light layers at level 0.
    let level = glow_level(24, now(&view), &view.glow_rolls).expect("lit");
    assert_eq!(
        plain_of(&list, 2100),
        [(lights_tint(level), true)],
        "its glow unmurked"
    );
    assert!(
        colorized_of(&list, 2310).is_empty(),
        "its weapon glow plain"
    );
    assert_eq!(
        plain_of(&list, 2310),
        [(lights_tint(28), true)],
        "its weapon glow unmurked"
    );
    let shield = flashing(&mut view, ShipRef::Player);
    let list = super::faked_world::world_list(&view);
    assert_eq!(
        colorized_of(&list, 2300),
        [(lights_tint(shield), true, hull[0].2)],
        "its shield takes the hull's colorize"
    );
    assert_eq!(plain_of(&list, 2300), [], "drawn colorized alone");
}

// Solid runs (rdm ship-handling-fidelity phase 23, the `solid_runs`
// module).

/// [`ionized_sheet`] with a weapon glow `rlëD` 1930, whose sheet holds
/// solid runs as `weapon` says, and its hull's as `hull` says.
fn solid_sheet(weapon: bool, hull: bool) -> ShipSheet {
    ShipSheet {
        weapon: Some(LayerSheet {
            solid: weapon,
            ..layer(1930, 72)
        }),
        solid: hull,
        ..ionized_sheet()
    }
}

/// A murky flight whose NPC has `sheet`.
fn murky_with(sheet: ShipSheet) -> Faked {
    let catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(sheet));
    shown_by(catalog, 0.0, 30, &Rulebook::default())
}

/// Each command of `image` in `list`, with its place in the list: its
/// kind, frame, whether it is colorized, tint and blend.
fn commands_of(
    list: &DrawList,
    image: i16,
) -> Vec<(
    usize,
    crate::ImageKind,
    u16,
    bool,
    Color,
    crate::draw::Blend,
)> {
    list.iter()
        .enumerate()
        .filter_map(|(at, command)| match *command {
            DrawCommand::Sprite {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((at, key.kind, key.frame, false, tint, blend)),
            DrawCommand::Colorized {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((at, key.kind, key.frame, true, tint, blend)),
            _ => None,
        })
        .collect()
}

/// Lights the NPC's weapon glow in `view`, falling 4 a step for 2 steps:
/// drawn at 28.
fn lit_at_28(view: &mut Faked, id: NpcId) {
    lit_at_28_ship(view, ShipRef::Npc(id));
}

/// Lights `ship`'s weapon glow in `view`: drawn at 28.
fn lit_at_28_ship(view: &mut Faked, ship: ShipRef) {
    view.glows.step(&[(ship, false)], |_| Some(4.0));
    view.glows.step(&[], |_| Some(4.0));
    assert_eq!(view.glows.drawn(ship), Some(28));
}

#[test]
fn an_ionized_npcs_lit_weapon_glow_draws_its_solid_runs_plainly_right_after_it() {
    use crate::ImageKind::{Rled, RledSolid};
    use crate::draw::Blend;
    let mut view = murky_with(solid_sheet(true, false));
    lit_at_28(&mut view, NPC_0);
    let (list, _) = npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    let drawn = commands_of(&list, 1930);
    assert_eq!(drawn.len(), 2, "{drawn:?}");
    let (glow_at, kind, frame, colorized, _, blend) = drawn[0];
    assert_eq!((kind, colorized, blend), (Rled, true, Blend::Or));
    assert_eq!(
        drawn[1],
        (
            glow_at + 1,
            RledSolid,
            frame,
            false,
            Color::WHITE,
            Blend::Normal
        ),
        "right after it, its frame, plain white"
    );
}

#[test]
fn a_weapon_glow_whose_sheet_holds_no_runs_draws_no_overlay() {
    let mut view = murky_with(solid_sheet(false, false));
    lit_at_28(&mut view, NPC_0);
    let (list, _) = npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    let drawn = commands_of(&list, 1930);
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert_eq!(drawn[0].1, crate::ImageKind::Rled);
}

/// The player's hull drawn now: where, its tint and its blend.
fn player_hull(view: &Faked) -> Option<(Point, Color, crate::draw::Blend)> {
    super::faked_world::world_list(view)
        .iter()
        .find_map(|command| match *command {
            DrawCommand::Sprite {
                image,
                center,
                tint,
                blend,
            } if image.id == 2000 => Some((center, tint, blend)),
            _ => None,
        })
}

#[test]
fn the_players_ship_is_drawn_faint_at_a_full_fade_and_jittered_while_fading() {
    // `_HandleShipDisplay` draws the player's full fade at 0x1e
    // (+0x86 0, @0x2d199-0x2d1a1 → 0x2d1be), and its fade jittered and
    // faint (@0x2d2c0-0x2d648), as an NPC's. How its cloak shows is the
    // session's (nova-sim's `the_players_full_fade_is_drawn_faint`).
    let mut view = flying_with(catalog(), 0.0);
    let (centre, _, _) = player_hull(&view).expect("drawn");
    redrawn(&mut view, 0, |player| {
        player.cloak = CloakShown::Translucent(0x1e);
    });
    let (_, tint, _) = player_hull(&view).expect("drawn faint");
    assert_eq!(tint, crate::draw::translucent_tint(4, 30), "at 4 over 30");
    redrawn(&mut view, 0, |player| {
        player.cloak = CloakShown::Fading(25.0);
    });
    let (at, tint, blend) = player_hull(&view).expect("drawn fading");
    assert_eq!((tint, blend), (lights_tint(7), crate::draw::Blend::Or));
    assert!(
        (at.x - centre.x).abs() <= 2.0 && (at.y - centre.y).abs() <= 2.0,
        "{at:?} by {centre:?}"
    );
}

#[test]
fn an_unlit_weapon_glow_draws_neither_it_nor_its_runs() {
    let view = murky_with(solid_sheet(true, false));
    let (list, _) = npc_drawn_ionized(&view, |_| {}, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    assert_eq!(commands_of(&list, 1930), []);
}

#[test]
fn a_translucent_hull_whose_sheet_holds_runs_draws_them_after_it() {
    use crate::ImageKind::{Rled, RledSolid};
    use crate::draw::Blend;
    let view = murky_with(solid_sheet(false, true));
    let (list, murk) = npc_drawn_ionized(&view, |_| {}, None, nova_sim::Vec2::new(100.0, -100.0));
    let drawn = commands_of(&list, 2001);
    let unmurked = Color {
        a: crate::flight::ship_layers::base_alpha(10),
        ..Color::WHITE
    };
    let (hull_at, _, frame, ..) = drawn[0];
    assert_eq!(
        drawn,
        [
            (
                hull_at,
                Rled,
                frame,
                false,
                murk_tint(unmurked, murk),
                Blend::Normal
            ),
            (
                hull_at + 1,
                RledSolid,
                frame,
                false,
                Color::WHITE,
                Blend::Normal
            ),
        ]
    );
}
