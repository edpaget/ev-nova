//! Runs one of the fight's phases among test ships, each a fresh
//! [`Fighter`], as the session runs each phase over the fighters it builds
//! afresh (`nova_sim::Session::with_fighters`).
//!
//! Each helper runs **one** phase. The order the phases run in within a
//! pass is the session's alone (`nova_sim::Session::pass_after_move`): a
//! test that sets up its state with one phase before running the phase it
//! pins may, but no helper here chains the phases in a pass's order. A
//! test whose assertion depends on that order belongs to the session, on
//! `nova_sim::Session::game_work`.

use super::*;

/// The arena a test's phase is fought in by default: the advance pass at
/// single speed, `arsenal`, `govts` and `rules`, in the default field.
pub(super) fn arena<'a>(
    arsenal: &'a Arsenal,
    govts: &'a Governments,
    rules: Rules<'a>,
) -> Arena<'a> {
    Arena {
        pass: GamePass::Advance { double_time: false },
        arsenal,
        govts,
        rules,
        field: Field::default(),
    }
}

/// Runs `phase` among `ships` in `arena`, drawing on `chance`.
pub(super) fn run_in(
    combat: &mut Combat,
    ships: &mut [Ship],
    arena: Arena<'_>,
    chance: &mut dyn Chance,
    phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
) {
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    phase(combat, &mut fighters, arena, chance);
}

/// Runs `phase` among `ships` by `rules`, with no arsenal or governments,
/// on the advance pass, drawing on [`NeverFires`].
pub(super) fn run_by(
    combat: &mut Combat,
    ships: &mut [Ship],
    rules: Rules<'_>,
    phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    run_in(
        combat,
        ships,
        arena(&arsenal, &govts, rules),
        &mut NeverFires,
        phase,
    );
}

/// Runs `phase` among `ships` on `pass`, by the default rules, with no
/// arsenal or governments, drawing on [`NeverFires`].
pub(super) fn run_on(
    combat: &mut Combat,
    ships: &mut [Ship],
    pass: GamePass,
    phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
) {
    let (arsenal, govts) = (Arsenal::default(), Governments::default());
    let arena = Arena {
        pass,
        ..arena(&arsenal, &govts, Rules::default())
    };
    run_in(combat, ships, arena, &mut NeverFires, phase);
}

/// Runs `phase` among `ships` as [`run_by`] does, with `rule` deciding
/// when a ship is disabled and the other rules the default.
pub(super) fn run(
    combat: &mut Combat,
    ships: &mut [Ship],
    rule: &dyn DisableRule,
    phase: impl FnOnce(&mut Combat, &mut [Fighter], Arena<'_>, &mut dyn Chance),
) {
    let rules = Rules {
        disable: rule,
        ..Rules::default()
    };
    run_by(combat, ships, rules, phase);
}

/// Step 1's trigger: the player fires and its reloads count down
/// ([`Combat::fire_player`]).
pub(super) fn fire_player(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.fire_player(fighters, arena, chance);
    });
}

/// The player's regeneration ([`Combat::regenerate_player`]).
pub(super) fn regenerate_player(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, _| {
        combat.regenerate_player(fighters, arena);
    });
}

/// The player's display step: its death timer's tick, as `_HandlePlayer`
/// runs it (@0x6d7e6), then its display ([`Combat::display_player`]).
pub(super) fn display_player(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    for ship in ships.iter_mut().filter(|ship| ship.id == ShipRef::Player) {
        ship.condition.count_down();
    }
    run(combat, ships, rule, |combat, fighters, arena, _| {
        combat.display_player(fighters, arena);
    });
}

/// The collision passes, with no stellars or asteroids
/// ([`Combat::test_hits`]).
pub(super) fn test_hits(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.test_hits(fighters, arena, Scenery::default(), chance);
    });
}

/// Step 4's point defence and launches: every NPC's
/// ([`Combat::defend_npcs`]), the whole loop at once.
pub(super) fn defend_npcs(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.defend_npcs(fighters, arena, None, false, chance);
    });
}

/// Step 5, `_HandleShot` ([`Combat::fly_shots`]).
pub(super) fn fly_shots(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.fly_shots(fighters, arena, chance);
    });
}

/// Step 6, the ship loop's fight: every NPC's `_HandleShip` and display,
/// by slot ([`Combat::handle_npcs`]).
pub(super) fn handle_npcs(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.handle_npcs(fighters, arena, chance);
    });
}

/// `_HandleBeams` and the late steps, with no stellars
/// ([`Combat::close_tick_in`]).
pub(super) fn close_tick(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
    run(combat, ships, rule, |combat, fighters, arena, chance| {
        combat.close_tick_in(fighters, arena, &mut [], None, chance);
    });
}

/// The phase in which `firer` fires its trigger: the player's trigger
/// ([`fire_player`]) or, for an NPC, the ship loop ([`handle_npcs`]).
pub(super) fn fire(
    combat: &mut Combat,
    ships: &mut [Ship],
    firer: ShipRef,
    rule: &dyn DisableRule,
) {
    if firer == ShipRef::Player {
        fire_player(combat, ships, rule);
    } else {
        handle_npcs(combat, ships, rule);
    }
}
