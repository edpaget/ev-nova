//! Helpers for the world's component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-7-world-frame): a view over a scripted
//! frame, read from the [`FakeCatalog`] as a session's view is read, and
//! the world it draws. Kept apart from `tests/mod.rs`, which is large.

use super::*;
use nova_sim::flight_frame::FrameShip;

/// A view over a flight scripted to show `frame`, read from `catalog` by
/// `rules` as a session's view is read: the player's sheet of the frame's
/// class, the looks of each of the catalog's weapons, and the NPCs' sheets
/// and codes the frame asks for.
pub(super) fn world_by(catalog: FakeCatalog, frame: FlightFrame, rules: &Rulebook) -> Faked {
    let looks = Looks::read(&catalog, catalog.weapons().iter().map(|weapon| weapon.id));
    let mut view = FlightView::scripted(catalog, scripted_flight(frame), rules);
    let class = view.frame.player().class;
    view.sheet = view.catalog.ship_sheet(class);
    view.sheet_ship = Some(class);
    view.looks = looks;
    view.read_npc_sheets();
    view
}

/// [`world_by`] by the default rules.
pub(super) fn world(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
    world_by(catalog, frame, &Rulebook::default())
}

/// A fake flight showing `frame`.
fn scripted_flight(frame: FlightFrame) -> FakeFlight {
    let mut fake = FakeFlight::new();
    fake.set_frame(frame);
    fake
}

/// Shows `frame` from now on: the fake's, the view's kept frame filled
/// from it, and the sheets it asks for read, as a step would leave them.
pub(super) fn shows(view: &mut Faked, frame: FlightFrame) {
    view.sim.set_frame(frame);
    view.read_npc_sheets();
}

/// The world `view` draws, its brackets, Red Alert and escorts its
/// frame's ([`FlightFrame::hud`](nova_sim::flight_frame::FlightFrame::hud)).
pub(super) fn world_list(view: &Faked) -> DrawList {
    let mut list = DrawList::new();
    assert!(view.draw_world_scene(&mut list), "a scene to draw");
    list
}

/// The world `view` draws with its flight's frame showing `hud`'s
/// brackets, icon and escort rows from now on.
pub(super) fn world_list_with(
    view: &mut Faked,
    hud: &nova_sim::flight_frame::FrameHud,
) -> DrawList {
    view.sim.frame_mut().hud = hud.clone();
    view.refresh_frame();
    world_list(view)
}

/// The ships' sprites `view` draws (sheets 2000 to 2299).
pub(super) fn faked_ship_sprites(view: &Faked) -> Vec<(ImageKey, Point)> {
    sprites(&world_list(view))
        .into_iter()
        .filter(|(image, _)| (2000..2300).contains(&image.id))
        .collect()
}

/// The player's ship 128 at `at` facing `heading`.
pub(super) fn the_player_at(at: Vec2, heading: f32) -> FrameShip {
    record::player(128, at, heading)
}

/// NPC `id` of ship 129 at `at` facing `heading`.
pub(super) fn npc_129(id: u32, at: Vec2, heading: f32) -> FrameShip {
    record::npc(id, 129, at, heading)
}

/// A frame in Sol (130) holding `ships`, the player's first, each drawn
/// in the ships' layer in their order but the player's, drawn last, and
/// Sol's two stellars shown.
pub(super) fn in_sol_with(ships: Vec<FrameShip>) -> FlightFrame {
    let mut drawn: Vec<_> = ships
        .iter()
        .filter(|ship| ship.ship != ShipRef::Player)
        .map(|ship| (ship.ship, nova_sim::ShipLayer::Ships))
        .collect();
    drawn.push((ShipRef::Player, nova_sim::ShipLayer::Ships));
    FlightFrame {
        ships,
        drawn,
        stellars: vec![
            record::stellar(StellarId(128)),
            record::stellar(StellarId(129)),
        ],
        ..in_sol()
    }
}

/// Shows `events` on `view`'s effects, as a pass raising them would,
/// about the player where the frame has it, by the view's presentation,
/// on its effects' chance.
pub(super) fn raise(view: &mut Faked, events: &[CombatEvent]) {
    let scene = view.presentation.scene(view.frame.player().position);
    let mut chance = view.effects_chance.clone();
    for event in events {
        view.effects.apply(event, &scene, &view.looks, &mut chance);
    }
}

/// A beam of `weapon` fired by `firer` from `start` to `end`, unfaded.
pub(super) fn beam(
    firer: ShipRef,
    weapon: WeaponId,
    start: Vec2,
    end: Vec2,
) -> nova_sim::flight_frame::FrameBeam {
    nova_sim::flight_frame::FrameBeam {
        firer,
        start,
        end,
        weapon,
        fade: 0,
    }
}

/// Shot `id` of `weapon` at `at` facing `heading`, at rest, in the shots'
/// layer.
pub(super) fn shot_of(
    id: u32,
    weapon: WeaponId,
    at: Vec2,
    heading: f32,
) -> nova_sim::flight_frame::FrameShot {
    nova_sim::flight_frame::FrameShot {
        id: nova_sim::combat::projectile::ShotId(id),
        weapon,
        position: at,
        heading,
        ..record::shot()
    }
}

/// An asteroid of kind 0 at `at`, at rest, on its first frame.
pub(super) fn asteroid_at(at: Vec2) -> nova_sim::flight_frame::FrameAsteroid {
    nova_sim::flight_frame::FrameAsteroid {
        position: at,
        velocity: Vec2::ZERO,
        frame: 0.0,
        kind: 0,
    }
}

/// A cargo box of graphic 0 at `at`, at rest, unfaded, on its first
/// frame.
pub(super) fn box_at(at: Vec2) -> nova_sim::flight_frame::FrameBox {
    nova_sim::flight_frame::FrameBox {
        position: at,
        velocity: Vec2::ZERO,
        frame: 0,
        fade: 0,
        graphic: 0,
    }
}

/// A HUD with NPC `id` targeted, its brackets settled round it as a
/// neutral's.
pub(super) fn targeting(id: u32) -> nova_sim::flight_frame::FrameHud {
    nova_sim::flight_frame::FrameHud {
        target_brackets: Some(nova_sim::brackets::TargetBrackets {
            npc: NpcId(id),
            expand: 0.0,
            standing: crate::flight::brackets::Standing::Neutral,
        }),
        ..nova_sim::flight_frame::FrameHud::default()
    }
}

/// Puts `text` on `view`'s message line from now, for `ticks`.
pub(super) fn saying(view: &mut Faked, text: &str, ticks: u16) {
    view.message = Some(super::super::Message {
        text: text.to_owned(),
        said_at: crate::time::ticks(view.elapsed),
        ticks,
    });
}
