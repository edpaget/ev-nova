//! The ships drawn as `_HandleShipDisplay` draws them (rdm
//! ship-handling-fidelity phase 10): their banked, folded and animated
//! sets, the NPCs' engine glow, running lights and murk, lights out while
//! disabled, the alt layer, the shield layer's flash and the hull's
//! `BaseTransp`.

use super::*;
use crate::flight::murk::{murk_tint, sprite_murk};
use crate::flight::ship_layers::{lights_murk, npc_glow_rolls};
use crate::flight::weapon_glow::npc_glow_level;
use nova_sim::catalog::{ShipDisplay, ShipExits};
use nova_sim::cloak::CloakShown;
use nova_sim::combat::ship_frames::{ANIMATED, BANKS};

/// A `shän` display of `flags`, `sets` base sets and `AnimDelay` `delay`,
/// its lights sprite steady.
fn display(flags: u16, sets: i16, delay: i16) -> ShipDisplay {
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
fn drawn_of(view: &View, image: i16) -> Vec<(u16, Color, bool)> {
    drawn(view)
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
fn frames_of(view: &View, image: i16) -> Vec<u16> {
    drawn_of(view, image)
        .into_iter()
        .map(|(frame, ..)| frame)
        .collect()
}

/// The flight's tick now, which the glow and lights roll by.
fn now(view: &View) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).unwrap_or(u64::MAX)
}

/// Every NPC heads out to jump.
#[derive(Debug)]
struct Leaving;

impl Behaviour for Leaving {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::JumpOut
    }
}

/// [`trafficked`] with NPC ship 129 of `npc` (72 frames a turn) and the
/// player's 128 of `player` (36), the NPC's sheet `sheet` when given.
fn shown(player: ShipDisplay, npc: ShipDisplay, sheet: Option<ShipSheet>) -> FakeCatalog {
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
fn npc_sheet() -> ShipSheet {
    ShipSheet {
        image_id: 2001,
        rotations: NonZeroU16::new(72).expect("non-zero"),
        glow: Some(layer(2101, 72)),
        lights: Some(layer(2201, 72)),
        ..sheet()
    }
}

/// `catalog`'s flight, its one NPC placed at (100, -100) facing
/// `heading`, flown as `behaviour` says, then `draws` drawn.
fn flying_with(
    catalog: FakeCatalog,
    heading: u32,
    behaviour: Rc<dyn Behaviour>,
    draws: &[u32],
) -> View {
    let script: Vec<u32> = placed(850, 650, heading)
        .into_iter()
        .chain(draws.iter().copied())
        .collect();
    let (_, chance) = scripted(&script);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(behaviour);
    view.tick(TICK);
    view
}

fn npc_of(view: &View) -> nova_sim::Npc {
    view.session().expect("flying").npcs()[0].clone()
}

#[test]
fn the_player_holding_right_shows_its_banked_set_past_6_and_set_0_let_go() {
    let catalog = shown(display(BANKS, 3, 0), display(0, 1, 0), None);
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    view.input(&key(Key::Right, true));
    let mut sets = Vec::new();
    for _ in 0..8 {
        view.tick(TICK);
        let frame = frames_of(&view, 2000)[0];
        assert_eq!(Some(frame), view.frame());
        sets.push(frame / 36);
    }
    assert_eq!(sets, [0, 0, 0, 0, 0, 0, 2, 2], "banked right past 6: set 2");
    view.input(&key(Key::Right, false));
    view.tick(TICK);
    assert_eq!(frames_of(&view, 2000)[0] / 36, 0, "let go: set 0");
    view.input(&key(Key::Left, true));
    ticks(&mut view, 20);
    assert_eq!(frames_of(&view, 2000)[0] / 36, 1, "banked left: set 1");
}

#[test]
fn a_turning_npc_shows_its_banked_set_past_4() {
    let catalog = shown(display(0, 1, 0), display(BANKS, 3, 0), Some(npc_sheet()));
    // Facing west at (100, -100), it turns right toward the north-east
    // to run out.
    let mut view = flying_with(catalog, 270, Rc::new(Leaving), &[]);
    let mut sets = Vec::new();
    for _ in 0..6 {
        view.tick(TICK);
        let npc = npc_of(&view);
        let frames = frames_of(&view, 2001);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0] / 72, npc.display.drawn.set);
        sets.push(npc.display.drawn.set);
    }
    assert_eq!(sets, [0, 0, 0, 0, 2, 2], "turning right, banked past 4");
}

#[test]
fn an_npcs_glow_brightens_under_thrust_and_fades_coasting_murked() {
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet()));
    catalog.sol_murk = 30;
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    let glows = |view: &mut View, n: usize| {
        (0..n)
            .map(|_| {
                view.tick(TICK);
                npc_of(view).display.glow
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        glows(&mut view, 3),
        [31, 30, 29],
        "at rest, coasting from 32"
    );
    let npc = npc_of(&view);
    let session = view.session().expect("flying");
    let murk = sprite_murk(
        npc.state.position,
        session.player().position,
        session.effective_murk(),
    );
    assert!(murk > 0, "a murky system");
    let level = glow_level(
        nova_sim::glow::drawn_base(npc.display.glow),
        now(&view),
        &npc_glow_rolls(npc.id),
    )
    .expect("lit");
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
    // Running out, it thrusts.
    view.behaviour = Rc::new(Leaving);
    let rising = glows(&mut view, 60);
    assert!(
        rising.windows(2).any(|pair| pair[1] > pair[0]),
        "{rising:?}"
    );
    assert_eq!(rising.iter().max(), Some(&28), "falling from 28 at first");
    let up_to = rising.iter().skip(5).max();
    assert_eq!(up_to, Some(&24), "thrust raises it to cruise: {rising:?}");
}

#[test]
fn a_ship_coasting_to_none_shows_no_glow() {
    let mut view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet())),
        0,
        Rc::new(Still),
        &[],
    );
    ticks(&mut view, 40);
    assert_eq!(npc_of(&view).display.glow, 0);
    assert_eq!(drawn_of(&view, 2101), [], "hidden at 1 or less");
}

#[test]
fn an_asteroid_miner_unfolds_to_fire_and_refolds_45_mac_ticks_after() {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.hulls[0] = shown_hull(128, display(0x00C2, 6, 5), 36, &[BLASTER]);
    let mut view = fighting(catalog, &[]);
    let set = |view: &View| view.frame().expect("a sheet") / 36;
    // A new pilot's ship folds up from set 0, a set every 6 frames.
    ticks(&mut view, 40);
    assert_eq!(set(&view), 5, "folded");
    view.input(&key(FIRE_KEY, true));
    let mut fired_at = None;
    let mut sets = Vec::new();
    for tick in 0..40 {
        view.tick(TICK);
        sets.push(set(&view));
        if shots_of(&view, BLASTER) > 0 {
            fired_at = Some(tick);
            break;
        }
    }
    assert_eq!(
        sets[..31],
        [
            5, 5, 5, 5, 5, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1,
            0, 0
        ][..],
        "no shot while folded: it unfolds a set every 6 frames"
    );
    assert_eq!(fired_at, Some(30), "then it fires at set 0");
    view.input(&key(FIRE_KEY, false));
    let mut sets = Vec::new();
    for _ in 0..30 {
        view.tick(TICK);
        sets.push(set(&view));
    }
    // The last shot's tick holds +0xa8; 23 ticks on, 46 Mac ticks have
    // passed, and the fold starts; 6 frames on it shows set 1.
    let first_up = sets.iter().position(|&set| set > 0).expect("refolds");
    assert_eq!(first_up, 22 + 6, "{sets:?}");
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
fn lights_drawn(view: &View) -> (Option<Color>, Option<Color>) {
    let tint = |image| drawn_of(view, image).first().map(|&(_, tint, _)| tint);
    (tint(2200), tint(2201))
}

/// Sets the player's and the NPC's conditions in `view` to `to`, the
/// player held there.
fn both(view: &mut View, to: Condition) {
    condition(view, to);
    view.disable_rule = if to == Condition::Disabled {
        Rc::new(Disabling)
    } else {
        Rc::new(nova_sim::NovaDisable)
    };
    let id = npc_of(view).id;
    if let Ok(session) = &mut view.session {
        if to == Condition::Disabled {
            nova_sim::fixture::disable(session, id);
        } else {
            nova_sim::fixture::repair(session, id);
        }
    }
}

#[test]
fn a_disabled_0x0040_ships_lights_go_out_and_climb_back_from_nothing() {
    let lit = ShipDisplay {
        blink: TRIANGLE,
        ..display(0x0040, 1, 0)
    };
    let mut catalog = shown(lit, lit, Some(npc_sheet()));
    catalog.sheet = Ok(ShipSheet {
        lights: Some(layer(2200, 36)),
        ..sheet()
    });
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    ticks(&mut view, 29);
    // The player's lights stepped once more, before the first frame; the
    // NPC's from the tick it came in.
    let rising = (Some(lights_tint(23)), Some(lights_tint(22)));
    assert_eq!(lights_drawn(&view), rising, "rising, intact");
    both(&mut view, Condition::Disabled);
    view.tick(TICK);
    assert_eq!(lights_drawn(&view), (None, None), "both out");
    ticks(&mut view, 4);
    assert_eq!(lights_drawn(&view), (None, None), "still out");
    both(&mut view, Condition::Intact);
    view.tick(TICK);
    assert_eq!(lights_drawn(&view), (None, None), "0.75: dark for a step");
    view.tick(TICK);
    let one = Some(lights_tint(1));
    assert_eq!(lights_drawn(&view), (one, one), "climbing from nothing");
}

#[test]
fn a_plug_in_shield_image_flashes_on_a_hit_and_fades() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 1;
    catalog.weapons[0].explod_type = -1;
    catalog.npc_sheet = Some(ShipSheet {
        shield: Some(layer(2501, 72)),
        ..npc_sheet()
    });
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    let npc = ShipRef::Npc(npc_of(&view).id);
    assert_eq!(drawn_of(&view, 2501), [], "unhit");
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    for _ in 0..20 {
        if view.shield_flash(npc).is_some() {
            break;
        }
        view.tick(TICK);
    }
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
    view.tick(TICK);
    assert_eq!(drawn_of(&view, 2501)[0].1, lights_tint(level - 1), "fading");
    ticks(&mut view, 40);
    assert_eq!(view.shield_flash(npc), None);
    assert_eq!(drawn_of(&view, 2501), [], "run out");
}

#[test]
fn an_animated_npc_cycles_its_sets() {
    let catalog = shown(display(0, 1, 0), display(ANIMATED, 3, 0), Some(npc_sheet()));
    // Its spawn's anim set rolled 0.
    let mut view = flying_with(catalog, 90, Rc::new(Still), &[0]);
    let mut sets = vec![frames_of(&view, 2001)[0] / 72];
    for _ in 0..4 {
        view.tick(TICK);
        sets.push(frames_of(&view, 2001)[0] / 72);
    }
    assert_eq!(sets, [1, 2, 0, 1, 2], "a set a frame at AnimDelay 0");
}

#[test]
fn the_alt_layer_is_drawn_at_its_own_set_with_the_hulls_tint() {
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
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    let hull = drawn_of(&view, 2000)[0];
    assert_eq!(
        hull.1.a,
        crate::flight::ship_layers::base_alpha(10),
        "BaseTransp 10"
    );
    let alt_sets = |view: &mut View, n: usize| {
        (0..n)
            .map(|_| {
                view.tick(TICK);
                let alt = drawn_of(view, 2400);
                assert_eq!(alt.len(), 1);
                assert!(!alt[0].2, "drawn plainly");
                assert_eq!(alt[0].1, drawn_of(view, 2000)[0].1, "with the hull's tint");
                alt[0].0 / 36
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        alt_sets(&mut view, 7),
        [0, 0, 0, 0, 1, 1, 1],
        "every 6 frames"
    );
}

/// Each sprite of `image` drawn: its frame and centre.
fn placed_of(view: &View, image: i16) -> Vec<(u16, Point)> {
    sprites(&drawn(view))
        .into_iter()
        .filter(|(key, _)| key.id == image)
        .map(|(key, at)| (key.frame, at))
        .collect()
}

/// A flight whose NPC is of 0x0020 with a two-set plug-in alt layer
/// (`rlëD` 2401), the held alt read by `reading`.
fn holding_alt(reading: RuleSource) -> View {
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
    flying_with(catalog, 0, Rc::new(Still), &[])
        .with_rules(Rulebook::default().with_override(RuleKey::DisabledAltHeld, reading))
}

#[test]
fn a_disabled_0x0020_npcs_alt_stays_where_it_was_drawn_as_the_player_flies_on() {
    let mut view = holding_alt(RuleSource::Engine);
    view.tick(TICK);
    let before = placed_of(&view, 2401);
    assert_eq!(before.len(), 1, "drawn live");
    assert_eq!(before[0].1, placed_of(&view, 2001)[0].1, "on its hull");
    let id = npc_of(&view).id;
    if let Ok(session) = &mut view.session {
        nova_sim::fixture::disable(session, id);
    }
    view.input(&key(Key::Up, true));
    ticks(&mut view, 10);
    let hull = placed_of(&view, 2001)[0].1;
    assert_ne!(hull, before[0].1, "the view moved on");
    assert_eq!(placed_of(&view, 2401), before, "its alt held on screen");
    // Repaired, it is drawn live on its hull again.
    if let Ok(session) = &mut view.session {
        nova_sim::fixture::repair(session, id);
    }
    view.tick(TICK);
    let hull = placed_of(&view, 2001)[0].1;
    assert_eq!(placed_of(&view, 2401)[0].1, hull);
}

#[test]
fn by_the_other_reading_a_disabled_0x0020_npcs_alt_is_hidden() {
    let mut view = holding_alt(RuleSource::Bible);
    view.tick(TICK);
    assert_eq!(placed_of(&view, 2401).len(), 1);
    let id = npc_of(&view).id;
    if let Ok(session) = &mut view.session {
        nova_sim::fixture::disable(session, id);
    }
    view.tick(TICK);
    assert_eq!(placed_of(&view, 2401), [], "hidden");
    assert_eq!(placed_of(&view, 2001).len(), 1, "its hull still drawn");
}

/// The hull of `npc` as `view` draws it: its frame's tint, if drawn.
fn npc_hull(view: &View, npc: &nova_sim::Npc) -> Option<Color> {
    let session = view.session().expect("flying");
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, npc, VIEW_CENTER, npc.state.heading);
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
    let view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(sheet)),
        0,
        Rc::new(Still),
        &[],
    );
    let base = crate::flight::ship_layers::base_alpha(10);
    let mut npc = npc_of(&view);
    assert_eq!(
        npc_hull(&view, &npc).map(|tint| tint.a),
        Some(base),
        "opaque: 22/32"
    );
    // Waiting at its gate, fading in, its fade's alpha in place of it.
    for ticks_left in [1, 3] {
        npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left };
        let faded = npc_tint(&npc).expect("shown").a;
        assert_ne!(faded, base, "{ticks_left}");
        assert_eq!(
            npc_hull(&view, &npc).map(|tint| tint.a),
            Some(faded),
            "{ticks_left}"
        );
    }
    npc.mode = nova_sim::traffic::npc::Mode::EnteringGate {
        gate: StellarId(128),
        ticks_left: 8,
    };
    let entering = npc_tint(&npc).expect("shown").a;
    assert_ne!(entering, base);
    assert_eq!(npc_hull(&view, &npc).map(|tint| tint.a), Some(entering));
    // Fully cloaked, its cloak decides: hidden, or, led by the player,
    // faint at its full fade's translucency.
    npc.mode = nova_sim::traffic::npc::Mode::Flying;
    npc.cloak = CLOAKED;
    assert_eq!(npc_hull(&view, &npc), None);
    npc.escort = Some(nova_sim::escort::EscortDuty {
        formation: nova_sim::escort::FormationSlot::default(),
        order: None,
    });
    let session = view.session().expect("flying");
    let shown = session.cloak_shown(&npc);
    assert_eq!(shown, CloakShown::Translucent(0x1e));
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
        0,
        Rc::new(Still),
        &[],
    );
    let mut npc = npc_of(&view);
    npc.cloak = nova_sim::cloak::Cloak {
        fade: 25.0,
        direction: 1,
    };
    let session = view.session().expect("flying");
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, &npc, VIEW_CENTER, npc.state.heading);
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
    npc.cloak = CLOAKED;
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, &npc, VIEW_CENTER, npc.state.heading);
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
        0,
        Rc::new(Still),
        &[],
    );
    let mut npc = npc_of(&view);
    view.glows
        .step(&[(ShipRef::Npc(npc.id), false)], |_| Some(1.0));
    let level = view.glows.drawn(ShipRef::Npc(npc.id)).expect("lit");
    npc.cloak = CLOAKED;
    let session = view.session().expect("flying");
    assert_eq!(session.cloak_shown(&npc), CloakShown::Hidden);
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, &npc, VIEW_CENTER, npc.state.heading);
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
    npc.cloak = nova_sim::cloak::Cloak::default();
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 100 };
    assert_eq!(npc_tint(&npc), None);
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, &npc, VIEW_CENTER, npc.state.heading);
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

/// `npc` drawn by `view` with `tint`, at `at` from the player.
fn npc_drawn_ionized(
    view: &View,
    npc: &nova_sim::Npc,
    tint: Option<nova_sim::combat::ion_tint::IonTint>,
    at: nova_sim::Vec2,
) -> (DrawList, u8) {
    let session = view.session().expect("flying");
    let mut npc = npc.clone();
    npc.state.position = session.player().position + at;
    npc.display.ion_tint = tint;
    let murk = sprite_murk(
        npc.state.position,
        session.player().position,
        session.effective_murk(),
    );
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, &npc, VIEW_CENTER, npc.state.heading);
    (list, murk)
}

const ION: nova_sim::combat::ion_tint::IonTint = nova_sim::combat::ion_tint::IonTint {
    level: 16,
    colour: 0x0034_c2ff,
};

/// Has `ship`'s shield flash in `view` on a hit's first frame at a full
/// shield: the level its layer is drawn at.
fn flashing(view: &mut View, ship: ShipRef) -> u8 {
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
fn murky_ionizing(rules: Option<nova_sim::Rulebook>) -> View {
    let mut catalog = shown(
        display(0, 1, 0),
        ShipDisplay {
            alt_sets: 1,
            alt_image: true,
            ..display(0, 1, 0)
        },
        Some(ionized_sheet()),
    );
    catalog.sol_murk = 30;
    let view = flying_with(catalog, 0, Rc::new(Still), &[]);
    match rules {
        Some(rules) => view.with_rules(rules),
        None => view,
    }
}

#[test]
fn an_ionized_npc_draws_its_hull_and_layers_colorized_in_its_ion_colour_at_its_unmurked_tint() {
    let mut view = murky_ionizing(None);
    let mut npc = npc_of(&view);
    npc.display.drawn.alt = nova_sim::combat::ship_frames::Alt::Drawn(0);
    let shield = flashing(&mut view, ShipRef::Npc(npc.id));
    let (list, murk) =
        npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
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
fn an_npc_below_a_third_ionized_draws_as_before() {
    let mut view = murky_ionizing(None);
    let npc = npc_of(&view);
    let shield = flashing(&mut view, ShipRef::Npc(npc.id));
    let at = nova_sim::Vec2::new(100.0, -100.0);
    let (plain, murk) = npc_drawn_ionized(&view, &npc, None, at);
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
    let npc = npc_of(&view);
    let (list, murk) =
        npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(300.0, -300.0));
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
    let npc = npc_of(&view);
    let (list, murk) =
        npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(300.0, -300.0));
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
    let mut npc = npc_of(&view);
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
    let (list, _) = npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
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
    let mut npc = npc_of(&view);
    lit_at_28(&mut view, &npc);
    npc.mode = nova_sim::traffic::npc::Mode::EnteringGate {
        gate: nova_sim::StellarId(130),
        ticks_left: 4,
    };
    let (list, _) = npc_drawn_ionized(&view, &npc, None, nova_sim::Vec2::new(100.0, -100.0));
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
    let mut npc = npc_of(&view);
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
    let (list, murk) = npc_drawn_ionized(&view, &npc, None, nova_sim::Vec2::new(100.0, -100.0));
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
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet()));
    catalog.fields = ShipFields {
        ionize_max: 400,
        deionize: 1,
        ..FIELDS
    };
    catalog.sheet = Ok(ShipSheet {
        glow: Some(layer(2100, 36)),
        shield: Some(layer(2300, 36)),
        ..sheet()
    });
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    let session = view.session.as_mut().expect("flying");
    nova_sim::fixture::set_player_ion(
        session,
        nova_sim::combat::ion::Ion {
            charge: 200.0,
            colour: 0x0000_80ff,
            ..nova_sim::combat::ion::Ion::default()
        },
    );
    view.tick(TICK);
    let tint = view
        .session()
        .expect("flying")
        .player_ion_tint()
        .expect("glowing");
    assert!((14..=18).contains(&tint.level), "{tint:?}");
    let list = drawn(&view);
    let hull = colorized_of(&list, 2000);
    assert_eq!(hull.len(), 1);
    assert_eq!(hull[0].2, crate::Colorize::toward(tint.level, 0x0000_80ff));
    assert_eq!(hull[0].0, Color::WHITE, "never murked");
    assert!(colorized_of(&list, 2100).is_empty(), "its glow plain");
    let shield = flashing(&mut view, ShipRef::Player);
    let list = drawn(&view);
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
fn murky_with(sheet: ShipSheet) -> View {
    let mut catalog = shown(display(0, 1, 0), display(0, 1, 0), Some(sheet));
    catalog.sol_murk = 30;
    flying_with(catalog, 0, Rc::new(Still), &[])
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
fn lit_at_28(view: &mut View, npc: &nova_sim::Npc) {
    let ship = ShipRef::Npc(npc.id);
    view.glows.step(&[(ship, false)], |_| Some(4.0));
    view.glows.step(&[], |_| Some(4.0));
    assert_eq!(view.glows.drawn(ship), Some(28));
}

#[test]
fn an_ionized_npcs_lit_weapon_glow_draws_its_solid_runs_plainly_right_after_it() {
    use crate::ImageKind::{Rled, RledSolid};
    use crate::draw::Blend;
    let mut view = murky_with(solid_sheet(true, false));
    let npc = npc_of(&view);
    lit_at_28(&mut view, &npc);
    let (list, _) = npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
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
    let npc = npc_of(&view);
    lit_at_28(&mut view, &npc);
    let (list, _) = npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    let drawn = commands_of(&list, 1930);
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert_eq!(drawn[0].1, crate::ImageKind::Rled);
}

/// The player's hull drawn now: where, its tint and its blend.
fn player_hull(view: &View) -> Option<(Point, Color, crate::draw::Blend)> {
    drawn(view).iter().find_map(|command| match *command {
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
    // faint (@0x2d2c0-0x2d648), as an NPC's.
    let mut view = flight();
    let (centre, _, _) = player_hull(&view).expect("drawn");
    let set = |view: &mut View, cloak| {
        let Ok(session) = &mut view.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_cloak(session, cloak);
    };
    set(&mut view, CLOAKED);
    let (_, tint, _) = player_hull(&view).expect("drawn faint");
    assert_eq!(tint, crate::draw::translucent_tint(4, 30), "at 4 over 30");
    set(
        &mut view,
        nova_sim::cloak::Cloak {
            fade: 25.0,
            direction: 1,
        },
    );
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
    let npc = npc_of(&view);
    let (list, _) = npc_drawn_ionized(&view, &npc, Some(ION), nova_sim::Vec2::new(100.0, -100.0));
    assert_eq!(commands_of(&list, 1930), []);
}

#[test]
fn a_translucent_hull_whose_sheet_holds_runs_draws_them_after_it() {
    use crate::ImageKind::{Rled, RledSolid};
    use crate::draw::Blend;
    let view = murky_with(solid_sheet(false, true));
    let npc = npc_of(&view);
    let (list, murk) = npc_drawn_ionized(&view, &npc, None, nova_sim::Vec2::new(100.0, -100.0));
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
