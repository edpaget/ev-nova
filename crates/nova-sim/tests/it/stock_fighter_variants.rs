//! Fighter variants over the stock data: the Fed Viper's and Fed
//! Anaconda's variants share their sprites, so a bay of the base takes
//! them. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_sim::ShipId;
use nova_sim::combat::armament::Arsenal;

#[test]
fn the_fed_vipers_and_anacondas_variants_draw_their_sprites() {
    let Some(data) = common::opened() else {
        return;
    };
    let arsenal = Arsenal::read(&data);
    let base = |ship: i16| arsenal.sprite_base(ShipId(ship)).0;
    assert_eq!(base(144), 144, "the base is its own");
    assert_eq!(base(223), 144);
    assert_eq!(base(224), 144);
    assert_eq!(base(225), 145);
    assert_eq!(base(145), 145);
}
