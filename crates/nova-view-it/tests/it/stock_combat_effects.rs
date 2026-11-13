//! The fight's effects over the stock data: the Hellhound's explosion,
//! as `wëap` 230's record gives it, scatters 5 of `bööm` 129 and 23 of `bööm` 128
//! about its main explosion through the stock looks; the stock shots'
//! fades and trails, the graviton beam's zig-zag and a weapon glow layer
//! (rdm weapons-fidelity phase 24); a dying ship's escape pod draws on
//! shän 895's sheet of 36 rotations; and the stars are `spïn` 700's 16
//! frames of 5 × 5. Skips, passing, when `NOVA_DATA` is
//! unset.

use crate::common;

use nova_data::GameData;
use nova_sim_frame::catalog::Explosion;
use nova_sim_frame::combat::shot_fade::at_full_gain;
use nova_sim_frame::{
    BoomId, CombatCatalog, CombatEvent, NeverFires, NpcId, RuleSource, ShipId, ShipRef, Vec2,
    WeaponId,
};
use nova_view::flight::Looks;
use nova_view::flight::ShipSprites;
use nova_view::flight::WeaponLook;
use nova_view::flight::effects::{Effects, LoopedSounds, Scene};
use nova_view::flight::lightning::{StepRolls, zig_zag};
use nova_view::flight::murk::SeenMurk;
use nova_view::flight::shot_fade::{ShotShade, shot_shade};
use nova_view::flight::weapons::TRANSLUCENT;
use nova_view::{DrawCommand, DrawList, Point};

#[test]
fn the_stock_hellhounds_explosion_scatters_5_small_and_23_large_extras() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    // The explosion and blast radius `nova_sim::combat::weapon::WeaponSpec::new`
    // reads from the record, as nova-sim-combat's tests pin.
    let hellhound = data
        .weapons()
        .into_iter()
        .find(|record| record.id == WeaponId(230))
        .expect("the Hellhound");
    let explosion = Explosion::decode(hellhound.explod_type).expect("an explosion");
    let blast_radius = f32::from(hellhound.blast_radius.max(0));
    let looks = Looks::read(&data, [WeaponId(230)]);
    let mut effects = Effects::default();
    let scene = Scene {
        player: Point::new(0.0, 0.0),
        debris: true,
        caps: false,
        looped: LoopedSounds::Anywhere,
    };
    let event = CombatEvent::Exploded {
        at: Vec2::new(100.0, 50.0),
        explosion,
        size: blast_radius,
    };
    effects.apply(&event, &scene, &looks, &mut NeverFires);
    let count = |boom| {
        effects
            .explosions()
            .iter()
            .filter(|shown| shown.boom == boom)
            .count()
    };
    assert_eq!(count(BoomId(129)), 5);
    assert_eq!(count(BoomId(128)), 23);
    let main = *effects.explosions().last().expect("the main one");
    assert_eq!(main.boom, explosion.boom);
    assert_eq!(main.at, Point::new(100.0, 50.0));
    assert_eq!(effects.explosions().len(), 29);
}

/// The stock looks of `ids`.
fn stock_looks(data: &GameData, ids: &[i16]) -> Looks {
    Looks::read(data, ids.iter().copied().map(WeaponId))
}

/// How a shot of `look` is drawn `age` full frames old
/// ([`at_full_gain`]), by `rule`.
fn shade_at(look: &WeaponLook, age: u32, rule: RuleSource) -> ShotShade {
    let count = u32::try_from(look.count).expect("a Count");
    let (life, fade) = at_full_gain(count, look.shot_falloff, age);
    shot_shade(look, fade, life, rule)
}

#[test]
fn stock_weapon_156_trails_and_fades_by_its_look() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let looks = stock_looks(&data, &[156]);
    let look = looks.weapon(WeaponId(156)).expect("wëap 156");
    assert_eq!(look.count, 110);
    assert_eq!(look.shot_falloff, 2);
    assert_eq!(look.flags3 & TRANSLUCENT, TRANSLUCENT);
    assert_eq!(look.particles, 2);
    assert_eq!(look.part_life, (10, 24));
    assert_eq!(look.part_color, 0x0010_40FF);
    let engine = RuleSource::Engine;
    assert_eq!(shade_at(look, 78, engine), ShotShade::Opaque);
    assert_eq!(shade_at(look, 79, engine), ShotShade::Additive(32));
    assert_eq!(shade_at(look, 95, engine), ShotShade::Additive(4));
}

#[test]
fn a_stock_143_is_opaque_before_its_fade_by_the_engine_and_added_by_the_other_reading() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let looks = stock_looks(&data, &[143]);
    let look = looks.weapon(WeaponId(143)).expect("wëap 143");
    assert_eq!((look.count, look.shot_falloff), (50, 2));
    // Life 40 at age 10: before its window; life 20 at age 30: 11 ticks
    // into it, 32 − 22.
    assert_eq!(shade_at(look, 10, RuleSource::Engine), ShotShade::Opaque);
    assert_eq!(
        shade_at(look, 10, RuleSource::Bible),
        ShotShade::Additive(32)
    );
    for rule in [RuleSource::Engine, RuleSource::Bible] {
        assert_eq!(
            shade_at(look, 30, rule),
            ShotShade::Additive(10),
            "{rule:?}"
        );
    }
}

#[test]
fn the_stock_graviton_beam_draws_nothing_under_100_pixels_by_the_engine() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let looks = stock_looks(&data, &[168]);
    let look = looks.weapon(WeaponId(168)).expect("wëap 168");
    assert_eq!(
        (look.li_density, look.li_amplitude),
        (2, 4),
        "LiDensity 1 loads as 2"
    );
    let (from, to) = (Point::new(0.0, 0.0), Point::new(99.0, 0.0));
    let mut rolls = StepRolls::new(1);
    assert_eq!(
        zig_zag(from, to, look, 0, RuleSource::Engine, &mut rolls),
        []
    );
    assert_eq!(
        zig_zag(from, to, look, 0, RuleSource::Bible, &mut rolls).len(),
        1,
        "one straight line by the other reading"
    );
    let far = Point::new(150.0, 0.0);
    assert_eq!(
        zig_zag(from, far, look, 0, RuleSource::Engine, &mut rolls).len(),
        2,
        "3 steps at 150: 2 segments a line wide"
    );
}

#[test]
fn stock_shan_141s_weapon_glow_is_rled_1826_falling_by_its_decay() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sheet = data.ship_sheet(ShipId(141)).expect("shän 141");
    let weapon = sheet.weapon.expect("a WeapImageID");
    assert_eq!(weapon.image, nova_view::SheetImage::Rled(1826));
    assert_eq!(sheet.weap_decay, 5);
    assert!((sheet.weapon_decay() - 1.665).abs() < 1e-6, "5 × 0.333");
}

#[test]
fn a_stock_escape_pod_draws_on_shan_895s_sheet_of_36_rotations() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let looks = Looks::read(&data, []);
    let sheet = *looks.pod_sheet().expect("shän 895's sheet reads");
    assert_eq!(sheet.frames.get(), 36);
    let mut effects = Effects::default();
    let scene = Scene {
        player: Point::new(0.0, 0.0),
        debris: false,
        caps: true,
        looped: LoopedSounds::Anywhere,
    };
    let event = CombatEvent::PodEjected {
        ship: ShipRef::Npc(NpcId(0)),
        at: Vec2::new(40.0, 0.0),
        velocity: Vec2::new(1.0, 0.0),
    };
    effects.apply(&event, &scene, &looks, &mut NeverFires);
    let heard: Vec<i16> = effects
        .take_sounds()
        .iter()
        .map(|sound| sound.sound.0)
        .collect();
    assert_eq!(heard, [372]);
    // The simulation sets it adrift; the view draws it from the frame.
    let pod = nova_sim_frame::escape_pod::Pod {
        position: Vec2::new(40.0, 0.0),
        velocity: Vec2::new(1.0, 0.0),
        life: 150.0,
        // A pod moving right faces frame 9 of the 36, by
        // `nova_sim::escape_pod::pod_frame`, which nova-sim-ai pins.
        frame: 9,
        shown: true,
    };
    let mut list = DrawList::new();
    let camera = nova_view::system::camera::Camera::centred_on(Point::new(0.0, 0.0));
    let clear = SeenMurk {
        viewer: Vec2::ZERO,
        murk: 0,
        colour: 0,
    };
    nova_view::flight::pods::draw_pods(&mut list, &camera, &looks, clear, &[pod]);
    let drawn = list
        .into_iter()
        .filter(|command| {
            matches!(command, DrawCommand::Sprite { image, .. } if *image == sheet.image.frame(image.frame))
        })
        .count();
    assert_eq!(drawn, 1);
}

#[test]
fn the_stock_stars_are_spin_700s_16_frames_of_5_by_5() {
    use nova_view::flight::CombatLooks;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let sheet = data.star_sheet().expect("spïn 700 reads");
    assert_eq!(
        (sheet.image, sheet.frames.get(), sheet.size),
        (nova_view::SheetImage::Rled(700), 16, (5, 5)),
        "rlëD 700, 4 × 4 tiles of 5 × 5"
    );
}
