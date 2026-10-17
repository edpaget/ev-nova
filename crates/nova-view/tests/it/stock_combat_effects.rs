//! The fight's effects over the stock data: the Hellhound's explosion,
//! as the fight raises it, scatters 5 of `bööm` 129 and 23 of `bööm` 128
//! about its main explosion through the stock looks. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::combat::weapon::WeaponSpec;
use nova_sim::{BoomId, CombatCatalog, CombatEvent, NeverFires, Vec2, WeaponId};
use nova_view::Point;
use nova_view::flight::Looks;
use nova_view::flight::effects::{Effects, LoopedSounds, Scene};

#[test]
fn the_stock_hellhounds_explosion_scatters_5_small_and_23_large_extras() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let hellhound = data
        .weapons()
        .iter()
        .map(WeaponSpec::new)
        .find(|spec| spec.id == WeaponId(230))
        .expect("the Hellhound");
    let explosion = hellhound.explosion.expect("an explosion");
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
        size: hellhound.blast_radius,
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
    let main = effects.explosions().last().expect("the main one");
    assert_eq!(main.boom, explosion.boom);
    assert_eq!(main.at, Point::new(100.0, 50.0));
    assert_eq!(effects.explosions().len(), 29);
}
