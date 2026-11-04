//! The fight's tests of the shield flash on hits past a ship's standing
//! (rdm play-feel-9 phase 4): `_DamageShip`'s write of the victim's
//! +0xc8f0 = 32.0 (@0x3bef9-0x3bf0c) runs after the reaction block's skip
//! target, so every path past the entry guards (@0x3a86b-0x3a88f)
//! reaches it, whether the hit is answered or not. A hit on a breaking-up
//! or disabled ship, the hit that takes a ship's last armour and a
//! ship's hit on itself (the reaction's self-hit skip, @0x3b411-0x3b4c8)
//! each raise their [`CombatEvent::Hit`] all the same.

use super::*;

/// The events as A at the centre fires `record` at `b`, at (100, 0): its
/// shot's 6th move meets it.
fn hits_on_b(record: WeaponRecord, b: Ship) -> Vec<CombatEvent> {
    events_on_move(record, vec![b], 6)
}

/// The ships among `events` whose hit flashes their shield: a hit that
/// doesn't pass shields, of A's weapon 128, where its shot met B's edge.
fn flashed(events: &[CombatEvent]) -> Vec<ShipRef> {
    raised(events)
        .into_iter()
        .filter(|&(_, weapon, at, passes)| {
            weapon == Some(WeaponId(128)) && within(at, Vec2::new(84.0, 0.0)) && !passes
        })
        .map(|hit| hit.0)
        .collect()
}

#[test]
fn a_hit_on_a_breaking_up_ship_raises_a_hit_that_flashes_it() {
    let mut dying = Ship::at(2, 100.0, 0.0);
    dying.reserves.armor.now = -1.0;
    dying.condition = Condition::Dying {
        ticks_left: 50,
        starting: false,
    };
    let events = hits_on_b(blaster(), dying);
    assert_eq!(flashed(&events), [B], "{events:?}");
}

#[test]
fn a_hit_on_a_disabled_ship_raises_a_hit_that_flashes_it() {
    let mut disabled = Ship::at(2, 100.0, 0.0);
    disabled.reserves.armor.now = 6.0;
    disabled.condition = Condition::Disabled;
    let events = hits_on_b(blaster(), disabled);
    assert_eq!(flashed(&events), [B], "{events:?}");
}

#[test]
fn the_hit_that_takes_a_ships_last_armour_raises_its_hit_too() {
    let shell = WeaponRecord {
        mass_dmg: 50,
        ..blaster()
    };
    let events = hits_on_b(shell, Ship::at(2, 100.0, 0.0));
    assert_eq!(flashed(&events), [B], "{events:?}");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, CombatEvent::BreakingUp { ship: B, .. })),
        "it breaks up: {events:?}"
    );
}

#[test]
fn a_ships_hit_on_itself_raises_a_hit() {
    let mut combat = Combat::default();
    // Met at (10, 0): the player in its own blast's square.
    let mut rocks = Rocks::of([rock(0, 20.0, 0.0)]);
    let (_, events) = shot_at_rocks(
        &mut combat,
        ShipRef::Player,
        blasting(0, 0),
        Vec::new(),
        &mut rocks,
        1,
    );
    let ships: Vec<ShipRef> = raised(&events).iter().map(|hit| hit.0).collect();
    assert_eq!(ships, [ShipRef::Player], "{events:?}");
}
