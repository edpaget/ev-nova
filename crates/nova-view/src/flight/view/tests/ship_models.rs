//! Ships drawn as 3D models (rdm:phase/engine-3d/phase-10-sample-in-3d):
//! a class with a hull model in the flight's table
//! ([`FlightView::with_world_models`]) draws it and its outfits on their
//! mounts in its outline's place, turned by its shown heading and the
//! frame's hull turn, its turrets facing its turret facing; a sheet, the
//! stock look's included, wins; a ship a cloak, a gate or its
//! `BaseTransp` fades draws its model at its [`model_alpha`], jittered as
//! its cloak jitters its hull, or nothing where its hull draws nothing;
//! and no table, or an empty one, draws as before. On [`FakeFlight`], with a
//! hand-built table. The commands expected are built with
//! [`mounted_outfits`] and [`ship_orientation`], whose own tests pin how
//! they place and turn.

use std::collections::BTreeMap;
use std::rc::Rc;

use nova_data::store::{Footing, Mount, Mounts, Shown};
use nova_sim::cloak::CloakShown;
use nova_sim::combat::hull_turn::HullTurn;
use nova_sim::flight_frame::GateFade;
use nova_sim::{NpcId, OutfitId};

use super::faked_world::{world, world_list};
use super::*;
use crate::flight::cloak_fade::{CloakDraw, cloak_rolls, player_cloak_rolls};
use crate::flight::model_fade::model_alpha;
use crate::flight::ship_layers::base_alpha;
use crate::flight::world_models::{ShipModel, WorldModels};
use crate::flight::{mounted_outfits, ship_orientation};
use crate::model::{ModelKey, ModelPose};
use crate::outline::{Outline, OutlineTriangle};

const PLAYER_HULL: ModelKey = ModelKey(1);
pub(super) const NPC_HULL: ModelKey = ModelKey(2);
const TURRET: ModelKey = ModelKey(3);
const GUN: ModelKey = ModelKey(4);
/// The kit that adds a turret bed.
pub(super) const BED: OutfitId = OutfitId(200);
/// How many logical units a model unit spans.
const SCALE: f32 = 1.5;

/// A dart 10 long from its centre to its nose, so 20 across as it turns.
fn dart() -> Outline {
    Outline {
        triangles: vec![OutlineTriangle {
            corners: [at(0.0, 10.0), at(3.0, -4.0), at(-3.0, -4.0)],
            color: Color::rgba(255, 0, 0, 255),
        }],
    }
}

/// The dart's frame: 20 by 20.
pub(super) const DART: (u32, u32) = (20, 20);

fn mount(hardpoint: &str, turns: bool, asset: &str) -> Mount {
    Mount {
        hardpoint: hardpoint.to_owned(),
        at: [0.0, if turns { -1.0 } else { 2.0 }, 0.5],
        footing: Footing::Surface,
        turns,
        shows: Some(Shown {
            asset: asset.to_owned(),
            lift: 0.25,
        }),
    }
}

/// Ship 128, the player's, with a fixed gun on its nose and a turret on
/// the [`BED`] kit; ship 130, an NPC's, built with the bed.
pub(super) fn table() -> WorldModels {
    let model = |key| ShipModel {
        key,
        scale: SCALE,
        base_transp: 0,
        radius: 2.0,
    };
    WorldModels {
        ships: BTreeMap::from([
            (ShipId(128), model(PLAYER_HULL)),
            (ShipId(130), model(NPC_HULL)),
        ]),
        built_kits: BTreeMap::from([(ShipId(130), vec![BED])]),
        mounts: Mounts {
            hulls: BTreeMap::from([(ShipId(128), vec![mount("nose", false, "gun")])]),
            kits: BTreeMap::from([
                ((ShipId(128), BED), vec![mount("bed", true, "turret-mount")]),
                ((ShipId(130), BED), vec![mount("bed", true, "turret-mount")]),
            ]),
        },
        outfits: BTreeMap::from([("turret-mount".to_owned(), TURRET), ("gun".to_owned(), GUN)]),
        stellars: BTreeMap::new(),
    }
}

/// The fake catalog with ship 128's sheet unreadable and the dart as the
/// outline of 128 and 130, with ship 130's traffic.
fn sheetless() -> FakeCatalog {
    FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        outlines: vec![(128, dart()), (130, dart())],
        ..trafficked(&[130], 1, 130, 3)
    }
}

/// The player at the centre facing `heading` and NPC 0, of ship 130, at
/// (100, -100) facing 90°, as `edit` leaves them, under `turn`.
pub(super) fn frame(
    heading: f32,
    turn: HullTurn,
    edit: impl FnOnce(&mut [FrameShip]),
) -> FlightFrame {
    let mut ships = vec![
        the_player_at(Vec2::ZERO, heading),
        record::npc(0, 130, Vec2::new(100.0, -100.0), 90.0),
    ];
    edit(&mut ships);
    FlightFrame {
        hull_turn: turn,
        ..in_sol_with(ships)
    }
}

/// `frame`'s view, drawing `models`.
pub(super) fn view_of(frame: FlightFrame, models: WorldModels) -> Faked {
    world(sheetless(), frame).with_world_models(Rc::new(models))
}

/// The 3D commands `list` draws, in order.
pub(super) fn models(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|c| matches!(c, DrawCommand::Model { .. } | DrawCommand::Mounted { .. }))
        .cloned()
        .collect()
}

fn triangles(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .filter(|c| matches!(c, DrawCommand::Triangle { .. }))
        .cloned()
        .collect()
}

/// The commands `ship`, drawn at `centre` facing `heading` under `turn`
/// as `hull` with `kits`, gives by the table.
pub(super) fn expected(
    ship: &FrameShip,
    (centre, heading, turn): (Point, f32, HullTurn),
    hull: ModelKey,
    kits: &[OutfitId],
) -> Vec<DrawCommand> {
    let table = table();
    let pose = ModelPose {
        center: centre,
        orientation: ship_orientation(&ship.frames, heading, turn),
        scale: SCALE,
    };
    let mounts = table.mounts(ship.class, kits);
    let facings = vec![ship.turret_facing; mounts.len()];
    mounted_outfits(
        hull,
        pose,
        ship.frames.hull_angle(heading, turn),
        &mounts,
        &facings,
        |asset| table.outfit(asset),
    )
}

/// The NPC's commands, as `view` draws them opaque by the table.
fn expected_npc(view: &Faked, turn: HullTurn) -> Vec<DrawCommand> {
    let npc = *view.drawn().npcs().next().expect("the npc");
    let (at, heading) = view.shown_npc(&npc);
    expected(
        &npc,
        (view.camera().sprite_centre(at, DART), heading, turn),
        NPC_HULL,
        &[BED],
    )
}

/// The player's commands, as `view` draws them opaque by the table.
pub(super) fn expected_player(view: &Faked, turn: HullTurn) -> Vec<DrawCommand> {
    expected(
        &view.drawn().player(),
        (
            view.camera().sprite_centre(view.shown_position(), DART),
            view.shown_heading(),
            turn,
        ),
        PLAYER_HULL,
        &view.drawn().player_kits,
    )
}

/// The player's commands and the NPC's, as `view` draws them by the table.
pub(super) fn expected_both(view: &Faked, turn: HullTurn) -> Vec<DrawCommand> {
    let mut all = expected_npc(view, turn);
    all.extend(expected_player(view, turn));
    all
}

#[test]
fn a_class_with_a_model_draws_it_and_its_outfits_in_its_outlines_place() {
    let view = view_of(frame(30.0, HullTurn::Continuous, |_| {}), table());
    let list = world_list(&view);
    assert_eq!(models(&list), expected_both(&view, HullTurn::Continuous));
    assert_eq!(triangles(&list), [], "no outline");
    // The NPC first, as the frame stacks it, then the player.
    let hulls: Vec<ModelKey> = models(&list)
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Model { model, .. } => Some(*model),
            _ => None,
        })
        .collect();
    assert_eq!(hulls, [NPC_HULL, PLAYER_HULL]);
}

#[test]
fn the_players_mounts_follow_the_kits_it_carries_and_an_npcs_its_class_built() {
    let parts = |list: &DrawList| -> Vec<ModelKey> {
        models(list)
            .iter()
            .filter_map(|c| match c {
                DrawCommand::Mounted { model, .. } => Some(*model),
                _ => None,
            })
            .collect()
    };
    let bare = view_of(frame(0.0, HullTurn::Continuous, |_| {}), table());
    // The NPC's bed, then the player's nose gun alone.
    assert_eq!(parts(&world_list(&bare)), [TURRET, GUN]);
    let fitted_frame = FlightFrame {
        player_kits: vec![BED],
        ..frame(0.0, HullTurn::Continuous, |_| {})
    };
    let fitted = view_of(fitted_frame, table());
    let list = world_list(&fitted);
    assert_eq!(parts(&list), [TURRET, GUN, TURRET]);
    assert_eq!(models(&list), expected_both(&fitted, HullTurn::Continuous));
}

/// The turn of each turret `list` draws.
pub(super) fn turret_turns(list: &DrawList) -> Vec<f32> {
    models(list)
        .iter()
        .filter_map(|c| match c {
            DrawCommand::Mounted { model, mounting } if *model == TURRET => Some(mounting.turn),
            _ => None,
        })
        .collect()
}

#[test]
fn a_turret_faces_its_ships_turret_facing_or_the_nose_with_none() {
    let nose = view_of(frame(0.0, HullTurn::Continuous, |_| {}), table());
    assert_eq!(turret_turns(&world_list(&nose)), [0.0]);
    let faced = view_of(
        frame(0.0, HullTurn::Continuous, |ships| {
            ships[1].turret_facing = Some(200.0);
        }),
        table(),
    );
    let list = world_list(&faced);
    // The NPC stands at 90°: its turret turns by 200 − 90 on its hull.
    assert_eq!(turret_turns(&list), [(200.0_f32 - 90.0).to_radians()]);
    assert_eq!(models(&list), expected_both(&faced, HullTurn::Continuous));
}

/// The player's hull orientation as `view` draws it.
fn player_orientation(view: &Faked) -> crate::model::Orientation {
    let list = world_list(view);
    models(&list)
        .iter()
        .find_map(|c| match c {
            DrawCommand::Model { model, pose, .. } if *model == PLAYER_HULL => {
                Some(pose.orientation)
            }
            _ => None,
        })
        .expect("the player's hull")
}

#[test]
fn the_hull_turns_within_a_sprite_frame_only_when_it_turns_continuously() {
    // Headings 1° and 4° stand on one frame of 36.
    for (turn, moves) in [(HullTurn::Continuous, true), (HullTurn::ByFrame, false)] {
        let at = |heading| player_orientation(&view_of(frame(heading, turn, |_| {}), table()));
        assert_eq!(at(1.0) != at(4.0), moves, "{turn:?}");
    }
}

#[test]
fn a_class_with_no_model_draws_its_outline_as_before() {
    let mut npc_only = table();
    npc_only.ships.remove(&ShipId(130));
    let view = view_of(frame(30.0, HullTurn::Continuous, |_| {}), npc_only);
    let list = world_list(&view);
    let npc = *view.drawn().npcs().next().expect("the npc");
    let (at, heading) = view.shown_npc(&npc);
    let mut outline = DrawList::new();
    crate::outline::draw(
        &mut outline,
        &dart(),
        view.camera().sprite_centre(at, DART),
        heading,
        1.0,
    );
    assert_eq!(
        triangles(&list),
        outline.iter().cloned().collect::<Vec<_>>()
    );
    let hulls = models(&list)
        .into_iter()
        .filter(|c| matches!(c, DrawCommand::Model { .. }))
        .count();
    assert_eq!(hulls, 1, "the player's alone");
}

/// Answers the fake catalog's own sheet for ship 130 alone.
struct StockLook;

impl ShipSprites for StockLook {
    fn ship_sheet(&self, id: ShipId) -> Result<ShipSheet, String> {
        (id == ShipId(130))
            .then(crate::flight::fake_catalog::sheet)
            .ok_or_else(|| "no look".to_owned())
    }

    fn ship_outline(&self, _: ShipId) -> Option<Outline> {
        None
    }
}

#[test]
fn a_catalog_sheet_or_the_stock_look_wins_over_a_model() {
    // The player's sheet reads in the plain catalog.
    let sheeted = world(
        FakeCatalog {
            outlines: vec![(130, dart())],
            ..trafficked(&[130], 1, 130, 3)
        },
        frame(0.0, HullTurn::Continuous, |_| {}),
    )
    .with_world_models(Rc::new(table()));
    let hulls: Vec<_> = models(&world_list(&sheeted))
        .into_iter()
        .filter(|c| matches!(c, DrawCommand::Model { .. }))
        .collect();
    assert!(
        matches!(
            hulls[..],
            [DrawCommand::Model {
                model: NPC_HULL,
                ..
            }]
        ),
        "the player draws its sheet: {hulls:?}"
    );
    let looked = view_of(frame(0.0, HullTurn::Continuous, |_| {}), table())
        .with_ship_looks(Rc::new(StockLook));
    let hulls: Vec<_> = models(&world_list(&looked))
        .into_iter()
        .filter(|c| matches!(c, DrawCommand::Model { .. }))
        .collect();
    assert!(
        matches!(
            hulls[..],
            [DrawCommand::Model {
                model: PLAYER_HULL,
                ..
            }]
        ),
        "the NPC draws the stock look: {hulls:?}"
    );
}

/// `commands` jittered by `offset` and faded to `alpha`: the hull's centre
/// moved and its alpha set, its parts following it.
pub(super) fn faded(commands: Vec<DrawCommand>, offset: Point, alpha: f32) -> Vec<DrawCommand> {
    commands
        .into_iter()
        .map(|command| command.translated(offset).at_alpha(alpha))
        .collect()
}

/// The tick `view` rolls its cloaks' jitter at.
fn tick(view: &Faked) -> u64 {
    u64::try_from(crate::time::ticks(view.elapsed)).expect("a tick")
}

/// The jitter NPC 0's cloak, shown as `cloak`, takes in `view`.
pub(super) fn npc_jitter(view: &Faked, cloak: CloakShown) -> Point {
    CloakDraw::of(cloak, None, tick(view), &cloak_rolls(NpcId(0)))
        .map_or(at(0.0, 0.0), |c| c.offset)
}

/// The jitter the player's cloak, shown as `cloak`, takes in `view`.
fn player_jitter(view: &Faked, cloak: CloakShown) -> Point {
    CloakDraw::of(cloak, None, tick(view), &player_cloak_rolls()).map_or(at(0.0, 0.0), |c| c.offset)
}

/// The alpha a model of `BaseTransp` 0 draws at, as `cloak` and `gate` fade
/// it ([`model_alpha`], whose own tests pin it).
pub(super) fn alpha_of(cloak: CloakShown, gate: GateFade) -> f32 {
    model_alpha(cloak, None, gate, 0).expect("drawn")
}

#[test]
fn an_npc_fading_under_a_cloak_draws_its_model_and_outfits_jittered_at_its_alpha() {
    let cloak = CloakShown::Fading(21.5);
    let view = view_of(
        frame(0.0, HullTurn::Continuous, |ships| ships[1].cloak = cloak),
        table(),
    );
    let jitter = npc_jitter(&view, cloak);
    assert_ne!(jitter, at(0.0, 0.0), "it jitters");
    let alpha = alpha_of(cloak, GateFade::None);
    assert!(alpha < 1.0);
    let mut want = faded(expected_npc(&view, HullTurn::Continuous), jitter, alpha);
    want.extend(expected_player(&view, HullTurn::Continuous));
    let list = world_list(&view);
    assert_eq!(models(&list), want);
    assert_eq!(triangles(&list), [], "no outline");
}

#[test]
fn a_seen_full_fade_npc_and_a_cloaked_player_draw_their_models_at_their_alphas() {
    let (faint, fading) = (CloakShown::Translucent(0x1e), CloakShown::Fading(12.0));
    let view = view_of(
        frame(0.0, HullTurn::Continuous, |ships| {
            ships[1].cloak = faint;
            ships[0].cloak = fading;
        }),
        table(),
    );
    let mut want = faded(
        expected_npc(&view, HullTurn::Continuous),
        npc_jitter(&view, faint),
        alpha_of(faint, GateFade::None),
    );
    want.extend(faded(
        expected_player(&view, HullTurn::Continuous),
        player_jitter(&view, fading),
        alpha_of(fading, GateFade::None),
    ));
    assert_eq!(models(&world_list(&view)), want);
}

/// A paint whose channels fade apart, so its share is not the unpainted one.
const PAINT: nova_sim::Rgb15 = nova_sim::Rgb15 { r: 31, g: 16, b: 4 };

/// The alpha a model of `BaseTransp` 0 painted [`PAINT`] draws at under
/// `cloak`, and that it is not the unpainted alpha.
fn painted_alpha(cloak: CloakShown) -> f32 {
    let alpha = model_alpha(cloak, Some(PAINT), GateFade::None, 0).expect("drawn");
    assert_ne!(alpha, alpha_of(cloak, GateFade::None), "paint changes it");
    alpha
}

#[test]
fn a_painted_npc_fading_under_a_cloak_draws_its_model_at_its_painted_alpha() {
    let cloak = CloakShown::Fading(21.5);
    let view = view_of(
        frame(0.0, HullTurn::Continuous, |ships| {
            ships[1].cloak = cloak;
            ships[1].paint = Some(PAINT);
        }),
        table(),
    );
    let jitter = CloakDraw::of(cloak, Some(PAINT), tick(&view), &cloak_rolls(NpcId(0)))
        .expect("cloaked")
        .offset;
    let mut want = faded(
        expected_npc(&view, HullTurn::Continuous),
        jitter,
        painted_alpha(cloak),
    );
    want.extend(expected_player(&view, HullTurn::Continuous));
    assert_eq!(models(&world_list(&view)), want);
}

#[test]
fn a_painted_player_fading_under_a_cloak_draws_its_model_at_its_painted_alpha() {
    let cloak = CloakShown::Fading(12.0);
    let view = view_of(
        frame(0.0, HullTurn::Continuous, |ships| {
            ships[0].cloak = cloak;
            ships[0].paint = Some(PAINT);
        }),
        table(),
    );
    let jitter = CloakDraw::of(cloak, Some(PAINT), tick(&view), &player_cloak_rolls())
        .expect("cloaked")
        .offset;
    let mut want = expected_npc(&view, HullTurn::Continuous);
    want.extend(faded(
        expected_player(&view, HullTurn::Continuous),
        jitter,
        painted_alpha(cloak),
    ));
    assert_eq!(models(&world_list(&view)), want);
}

#[test]
fn a_hidden_npc_and_one_waiting_at_its_gate_draw_no_model_and_no_outline() {
    type Edit = fn(&mut [FrameShip]);
    let edits: [(&str, Edit); 3] = [
        ("hidden", |ships| ships[1].cloak = CloakShown::Hidden),
        ("waiting", |ships| {
            ships[1].gate = GateFade::Emerging { ticks_left: 100.0 };
        }),
        ("unseen at its base", |ships| ships[1].class = ShipId(131)),
    ];
    for (why, edit) in edits {
        let mut models_table = table();
        models_table.ships.insert(
            ShipId(131),
            ShipModel {
                key: NPC_HULL,
                scale: SCALE,
                base_transp: 32,
                radius: 2.0,
            },
        );
        let catalog = FakeCatalog {
            outlines: vec![(128, dart()), (130, dart()), (131, dart())],
            ..sheetless()
        };
        let view = world(catalog, frame(0.0, HullTurn::Continuous, edit))
            .with_world_models(Rc::new(models_table));
        let list = world_list(&view);
        assert_eq!(
            models(&list),
            expected_player(&view, HullTurn::Continuous),
            "{why}"
        );
        assert_eq!(triangles(&list), [], "{why}: no outline");
    }
}

#[test]
fn an_npc_entering_or_coming_out_of_a_gate_draws_at_the_gates_alpha() {
    for gate in [
        GateFade::Entering { ticks_left: 9.0 },
        GateFade::Emerging { ticks_left: 3.0 },
    ] {
        let view = view_of(
            frame(0.0, HullTurn::Continuous, |ships| ships[1].gate = gate),
            table(),
        );
        let alpha = alpha_of(CloakShown::Opaque, gate);
        assert!(alpha < 1.0, "{gate:?}");
        let mut want = faded(
            expected_npc(&view, HullTurn::Continuous),
            at(0.0, 0.0),
            alpha,
        );
        want.extend(expected_player(&view, HullTurn::Continuous));
        assert_eq!(models(&world_list(&view)), want, "{gate:?}");
    }
}

#[test]
fn a_base_transparent_class_draws_its_model_at_its_base_alpha() {
    let mut models_table = table();
    for model in models_table.ships.values_mut() {
        model.base_transp = 10;
    }
    let view = view_of(frame(0.0, HullTurn::Continuous, |_| {}), models_table);
    let alpha = f32::from(base_alpha(10)) / 255.0;
    assert_eq!(
        model_alpha(CloakShown::Opaque, None, GateFade::None, 10),
        Some(alpha)
    );
    let mut want = faded(
        expected_npc(&view, HullTurn::Continuous),
        at(0.0, 0.0),
        alpha,
    );
    want.extend(faded(
        expected_player(&view, HullTurn::Continuous),
        at(0.0, 0.0),
        alpha,
    ));
    assert_eq!(models(&world_list(&view)), want);
}

#[test]
fn an_opaque_ship_draws_its_model_whole() {
    let view = view_of(frame(0.0, HullTurn::Continuous, |_| {}), table());
    let list = world_list(&view);
    assert_eq!(models(&list), expected_both(&view, HullTurn::Continuous));
    assert!(
        models(&list)
            .iter()
            .all(|c| !matches!(c, DrawCommand::Model { alpha, .. } if *alpha != 1.0)),
        "every hull at 1"
    );
}

#[test]
fn a_player_not_drawn_in_full_draws_no_model() {
    for shown in [nova_sim::Drawn::Hidden, nova_sim::Drawn::SpriteHidden] {
        let view = view_of(
            frame(0.0, HullTurn::Continuous, |ships| ships[0].shown = shown),
            table(),
        );
        assert_eq!(
            models(&world_list(&view)),
            expected_npc(&view, HullTurn::Continuous),
            "{shown:?}"
        );
    }
}

#[test]
fn no_table_and_an_empty_table_draw_the_same_list_as_before() {
    let before = world_list(&world(
        sheetless(),
        frame(30.0, HullTurn::Continuous, |_| {}),
    ));
    let empty = world_list(&view_of(
        frame(30.0, HullTurn::Continuous, |_| {}),
        WorldModels::default(),
    ));
    assert_eq!(
        empty.iter().cloned().collect::<Vec<_>>(),
        before.iter().cloned().collect::<Vec<_>>()
    );
    assert_eq!(models(&before), []);
}
