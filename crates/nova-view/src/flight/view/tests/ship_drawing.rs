//! The ships drawn as `_HandleShipDisplay` draws them (rdm
//! ship-handling-fidelity phase 10): their banked, folded and animated
//! sets, the NPCs' engine glow, running lights and murk, lights out while
//! disabled, the alt layer, the shield layer's flash and the hull's
//! `BaseTransp`.

use super::*;
use crate::flight::murk::{murk_tint, sprite_murk};
use crate::flight::ship_layers::{lights_murk, npc_rolls};
use crate::flight::weapon_glow::npc_glow_level;
use nova_sim::catalog::{ShipDisplay, ShipExits};
use nova_sim::combat::ship_frames::{ANIMATED, BANKS};

/// A `shän` display of `flags`, `sets` base sets and `AnimDelay` `delay`.
fn display(flags: u16, sets: i16, delay: i16) -> ShipDisplay {
    ShipDisplay {
        flags,
        base_sets: sets,
        anim_delay: delay,
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
        &npc_rolls(npc.id).0,
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

#[test]
fn a_disabled_0x0040_ships_lights_go_out() {
    let lit_sheet = ShipSheet {
        lights: Some(layer(2200, 36)),
        flags: 0x0040,
        ..sheet()
    };
    let mut catalog = shown(
        display(0x0040, 1, 0),
        display(0x0040, 1, 0),
        Some(ShipSheet {
            flags: 0x0040,
            ..npc_sheet()
        }),
    );
    catalog.sheet = Ok(lit_sheet);
    let mut view = flying_with(catalog, 0, Rc::new(Still), &[]);
    view.tick(TICK);
    assert_eq!(drawn_of(&view, 2200).len(), 1, "the player's, intact");
    assert_eq!(drawn_of(&view, 2201).len(), 1, "the NPC's, intact");
    let id = npc_of(&view).id;
    if let Ok(session) = &mut view.session {
        nova_sim::fixture::disable(session, id);
    }
    view.tick(TICK);
    condition(&mut view, Condition::Disabled);
    assert_eq!(drawn_of(&view, 2200), [], "the player's out");
    assert_eq!(drawn_of(&view, 2201), [], "the NPC's out");
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

#[test]
fn each_npc_is_noted_on_the_tick_it_was_first_seen_until_it_leaves() {
    let mut view = flying_with(
        shown(display(0, 1, 0), display(0, 1, 0), Some(npc_sheet())),
        0,
        Rc::new(Still),
        &[],
    );
    let id = npc_of(&view).id;
    let first = view.appeared.get(&id).copied().expect("noted");
    ticks(&mut view, 3);
    assert_eq!(
        view.appeared.get(&id),
        Some(&first),
        "kept from its first tick"
    );
    let gone = NpcId(99);
    view.appeared.insert(gone, 0);
    view.tick(TICK);
    assert_eq!(view.appeared.get(&gone), None, "forgotten once gone");
}

/// The hull of `npc` as `view` draws it: its frame's tint, if drawn.
fn npc_hull(view: &View, npc: &nova_sim::Npc) -> Option<Color> {
    let session = view.session().expect("flying");
    let mut list = DrawList::new();
    view.draw_npc(&mut list, session, npc, VIEW_CENTER, npc.state.heading);
    list.iter().find_map(|command| match *command {
        DrawCommand::Sprite { image, tint, .. } if image.id == 2001 => Some(tint),
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
    let cloaked = cloak_alpha(shown).expect("seen");
    assert_ne!(cloaked, base);
    assert_eq!(npc_hull(&view, &npc).map(|tint| tint.a), Some(cloaked));
}
