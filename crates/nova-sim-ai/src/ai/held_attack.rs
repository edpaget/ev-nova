//! An interceptor's goal 4 (`_InterceptorAI` @0x8c896 in the `EV Nova`
//! executable, then `_HighLevelAIHandler`'s goal-4 block
//! @0x8e246-0x8e731): an attack it holds or its provoked branch writes,
//! taken through the think's tail and the high level's drops in the
//! original's order, with no search in that think ([`interceptor_hunt`]).
//!
//! **The provoked branch** (@0x8cbf6-0x8cc31): with +0x96 above 0, +0x70
//! set and +0x50 at or below 0, every goal but 3, 7, 9 and 0xf becomes 4
//! on +0x70. The gate then closes for goal 4 (@0x8cc6d-0x8cc7d), so no
//! `_SelectWarshipTarget` runs, and no more does it for an unprovoked
//! goal 4. Behind the gate the think's tail runs, in this order:
//!
//! 1. the retreats (@0x8d14d-0x8d2e4), which need a government (@0x8d157)
//!    and goal 4 (@0x8d161), the odds retreat also its government's
//!    `Flags` 0x0100 (@0x8d180): the flight is goal 3 on +0x70;
//! 2. the disabled drop (@0x8d2fa-0x8d33a): a disabled +0x70 with no
//!    destroying weapon, goal 0 and +0x70 -1;
//! 3. the out-of-ammo retreat (@0x8d343-0x8d391): goal 3 on +0x70, which
//!    the disabled drop has just made -1, so the high level stops it
//!    (@0x8dff6 → @0x8e228: goal 0, +0x96 0).
//!
//! Then the high level's slot test makes a gone +0x70 -1 (@0x8d4e4-0x8d50d),
//! and goal 4's block drops the target ([`drop_of`]), each drop with its
//! own writes:
//!
//! | Drop | Address | Goal | +0x70 | +0x96 |
//! |---|---|---|---|---|
//! | Disabled, no destroying weapon (think) | @0x8d2fa-0x8d33a | 0 | -1 | kept |
//! | Gone (the slot test made +0x70 -1) | @0x8e263 → @0x8e731 | 0 | -1 | kept |
//! | Ally (`_GovtAllies`, neither ship led by the player) | @0x8e277-0x8e2db → @0x8fb47 | 0, sub 0 | **kept** | kept |
//! | Its fleet | @0x8e2e9-0x8e399 → @0x8e381 | 0, sub 0 | -1 | kept |
//! | Breaking up (`_IsDying`) | @0x8e3ef → @0x8e714-0x8e726 | 0 | -1 | 0 |
//!
//! The inactive-slot branch (@0x8e3df → @0x8e714) can't be reached, the
//! slot test having come first, nor can goal 4's +0x4c test (@0x8e26d),
//! behind the handler's head gate (@0x8d4b9-0x8d4d2). Undropped, the
//! block flies the attack ([`state4_attack`]).
//!
//! The engine's own drops stand beside these: a player paid off
//! ([`Npc::spared`]; in the original a bribe makes the ship of AI type 1,
//! no interceptor at all) and, by the Bible's reading of
//! [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks),
//! a blessed player ([`releases_blessed`](super::warship::releases_blessed)),
//! each clearing the target as the fleet drop does.
//!
//! **The kept target.** The ally drop keeps +0x70, and provocation never
//! decays ([`provocation`](crate::traffic::provocation)): only a drop
//! zeroes it, and the ally drop doesn't. So each think writes goal 4 on the
//! ally again and the ally drop idles it again, with no search, for as
//! long as the ally stays in its slot; once it leaves, the slot let-go
//! clears +0x70 and the next think searches. An unprovoked interceptor
//! idle with +0x70 set searches, inspects and patrols nothing either
//! (@0x8cc93-0x8ccc9 → @0x8d13f, [`searches_nothing`]): its out-of-ammo
//! retreat alone still runs, fleeing +0x70
//! ([`out_of_ammo::after_patrol`]). The provoked write inside that gate
//! (@0x8ccbe-0x8ccd8) can't be reached: the provoked branch above has
//! already closed the gate.
//!
//! The answer to the question whether a goal-4 pick of
//! [`pick_target`](super::target::pick_target) differs from the gate's
//! skip: its step 1 keeps a live ship held, so they differ only where a
//! drop takes the target, where the engine re-picked in the same think and
//! the original idles that frame. Nothing here plays a sound or draws a
//! random number.

use crate::ai::interceptor::{gate_closed, searches_nothing};
use crate::ai::odds::state4_attack;
use crate::ai::target::{TargetRules, spares_ally};
use crate::ai::warship::{Hunt, Retreat, releases_blessed, retreats};
use crate::ai::{Goal, Surroundings, fire, out_of_ammo};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// A drop of goal 4's target (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    /// Disabled while the ship has no destroying weapon: the think's
    /// drop (@0x8d2fa-0x8d33a).
    Disabled,
    /// Gone from its slot, or not yet arrived (@0x8e263 → @0x8e731).
    Gone,
    /// An ally it spares (@0x8e277-0x8e2db).
    Ally,
    /// A ship of its own fleet (@0x8e2e9-0x8e399).
    Fleet,
    /// The player paid off, or blessed by the Bible's reading: the
    /// engine's own.
    PaidOff,
    /// Breaking up (@0x8e3ef → @0x8e714-0x8e726).
    Dying,
}

impl Drop {
    /// Whether the drop makes the target -1: all but the ally drop.
    #[must_use]
    pub fn clears_target(self) -> bool {
        self != Self::Ally
    }

    /// Whether the drop zeroes the provocation: the breaking-up drop
    /// alone (@0x8e714-0x8e726).
    #[must_use]
    pub fn forgets_provocation(self) -> bool {
        self == Self::Dying
    }
}

/// The high level's drop of `ship`, the target of `npc` in goal 4, among
/// `around`, by `rules` and `always_attacks`, in the original's order:
/// gone, an ally, its fleet, the engine's paid-off or blessed player,
/// breaking up. None when goal 4 flies on.
#[must_use]
pub fn drop_of(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Option<Drop> {
    let jumping_in = match ship {
        ShipRef::Player => false,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| !other.arrived()),
    };
    if jumping_in || !around.condition_of(ship).is_some_and(Condition::active) {
        return Some(Drop::Gone);
    }
    if let ShipRef::Npc(id) = ship
        && let Some(other) = around.npc(id)
    {
        if spares_ally(npc, other, around, rules.led_ally) {
            return Some(Drop::Ally);
        }
        if other.fleet_among(around.npcs) == npc.fleet_among(around.npcs) {
            return Some(Drop::Fleet);
        }
    }
    if (ship == ShipRef::Player && npc.spared)
        || releases_blessed(npc, ship, around, always_attacks)
    {
        return Some(Drop::PaidOff);
    }
    matches!(around.condition_of(ship), Some(Condition::Dying { .. })).then_some(Drop::Dying)
}

/// What an interceptor `npc` does with goal 4 on `ship` among `around`,
/// retreating as `retreat` says and dropping by `rules` and
/// `always_attacks`: the think's tail, then goal 4's high level (see the
/// module docs). Judged as attacking `ship` whatever goal it held, as the
/// provoked branch writes goal 4 first.
#[must_use]
pub fn interceptor_attack(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Hunt {
    let judged = Npc {
        goal: Goal::Attack(ship),
        ..npc.clone()
    };
    if retreats(&judged, around, retreat) {
        return Hunt::of(Goal::Flee(ship));
    }
    let dry = retreat.out_of_ammo && out_of_ammo::retreats(npc);
    if around.condition_of(ship) == Some(Condition::Disabled) && !fire::destroys(npc) {
        // Goal 0, +0x70 -1; dry, goal 3 on -1, which the high level stops
        // (@0x8dff6 → @0x8e228: +0x96 0).
        return Hunt {
            goal: Some(Goal::Idle),
            clears_target: true,
            forgets_provocation: dry,
            ..Hunt::default()
        };
    }
    if dry {
        return Hunt::of(Goal::Flee(ship));
    }
    match drop_of(npc, ship, around, rules, always_attacks) {
        Some(drop) => Hunt {
            goal: Some(Goal::Idle),
            clears_target: drop.clears_target(),
            forgets_provocation: drop.forgets_provocation(),
            ..Hunt::default()
        },
        None => Hunt::of(state4_attack(npc, ship, around)),
    }
}

/// What an interceptor `npc`'s think does among `around` before its
/// search, by `retreat`, `rules` and `always_attacks`, when it holds no
/// word and flies no flight it keeps (see the module docs): provoked with
/// a target, any goal but a flight, an inspection or a help, goal 4 on it
/// ([`interceptor_attack`]); unprovoked in goal 4, the same on the ship
/// it attacks or snipes at; and nothing to fight, searching nothing, idle
/// with a target it kept ([`searches_nothing`]) or behind a closed gate
/// ([`gate_closed`]). None leaves it the hunt's own path: an inspection
/// this think ends, or a flight from a ship no longer here.
#[must_use]
pub fn interceptor_hunt(
    npc: &Npc,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Option<Hunt> {
    let attack = |ship| interceptor_attack(npc, ship, around, retreat, rules, always_attacks);
    let provoked_branch = !matches!(
        npc.goal,
        Goal::Flee(_) | Goal::Inspect(_) | Goal::Assist(_) | Goal::CountWait
    );
    if let Some(ship) = npc.target.filter(|_| npc.provoked > 0.0 && provoked_branch) {
        return Some(attack(ship));
    }
    if let Some(ship) = npc.goal.attacking() {
        return Some(attack(ship));
    }
    (searches_nothing(npc) || gate_closed(npc, around)).then(Hunt::default)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{
        NEUTRAL, PIRATES, POLICE, TRADERS, around, blessing_of, govts, n, ship, short_of_ammo,
        sites,
    };
    use crate::ai::interceptor::INTERCEPTOR_RETREAT;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::govt::{ALWAYS_ATTACKS_PLAYER, INTERCEPTORS_RETREAT};
    use crate::traffic::npc::{AiType, Mode, NpcId};

    const P: ShipRef = ShipRef::Player;

    /// A police interceptor (NPC 1) at the centre, attacking `ship`.
    fn police(ship_ref: ShipRef) -> Npc {
        let mut cop = ship(1, POLICE, AiType::Interceptor, 0.0, 0.0);
        cop.goal = Goal::Attack(ship_ref);
        cop.target = Some(ship_ref);
        cop
    }

    fn dropped_among(npcs: &[Npc], target: ShipRef, rules: TargetRules) -> Option<Drop> {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -1000.0), 0);
        drop_of(&npcs[0], target, &around, rules, RuleSource::Engine)
    }

    fn led_by_player(mut npc: Npc) -> Npc {
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        npc
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    #[test]
    fn a_ship_gone_or_still_jumping_in_is_the_gone_drop() {
        let alone = [police(n(2))];
        assert_eq!(
            dropped_among(&alone, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
        let mut arriving = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        arriving.mode = Mode::JumpingIn { tick: 0 };
        let npcs = [police(n(2)), arriving];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
        let mut destroyed = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        destroyed.condition = Condition::Destroyed;
        let npcs = [police(n(2)), destroyed];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
    }

    #[test]
    fn an_ally_is_dropped_by_either_reading_and_one_the_player_leads_only_by_the_bible() {
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            let rules = TargetRules {
                led_ally,
                ..TargetRules::default()
            };
            let npcs = [police(n(2)), ally.clone()];
            assert_eq!(dropped_among(&npcs, n(2), rules), Some(Drop::Ally));
        }
        let npcs = [police(n(2)), led_by_player(ally)];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            None,
            "the engine's ally test needs neither led by the player"
        );
        let bible = TargetRules {
            led_ally: RuleSource::Bible,
            ..TargetRules::default()
        };
        assert_eq!(dropped_among(&npcs, n(2), bible), Some(Drop::Ally));
    }

    #[test]
    fn a_ship_of_its_fleet_the_paid_off_player_and_a_ship_breaking_up_are_dropped() {
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(1));
        let npcs = [police(n(2)), escort];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Fleet)
        );
        let mut bribed = police(P);
        bribed.spared = true;
        assert_eq!(
            dropped_among(&[bribed], P, TargetRules::default()),
            Some(Drop::PaidOff)
        );
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        let npcs = [police(n(2)), dying];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Dying)
        );
        let npcs = [police(n(2)), ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        assert_eq!(dropped_among(&npcs, n(2), TargetRules::default()), None);
    }

    #[test]
    fn by_the_bible_a_blessed_player_is_dropped_and_by_the_engine_it_is_not() {
        let govts = govts(0, ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        let npcs = [police(P)];
        let mut around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        if let Some(player) = around.player.as_mut() {
            player.blessing = blessing_of(POLICE);
        }
        let rules = TargetRules::default();
        assert_eq!(
            drop_of(&npcs[0], P, &around, rules, RuleSource::Bible),
            Some(Drop::PaidOff)
        );
        assert_eq!(
            drop_of(&npcs[0], P, &around, rules, RuleSource::Engine),
            None
        );
    }

    #[test]
    fn the_ally_drop_comes_before_the_breaking_up_one_so_a_dying_ally_keeps_its_target() {
        let mut ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        ally.condition = DYING;
        let npcs = [police(n(2)), ally];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Ally)
        );
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(1));
        escort.condition = DYING;
        let npcs = [police(n(2)), escort];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Fleet),
            "the fleet drop comes before it too"
        );
    }

    #[test]
    fn each_drop_writes_its_own_target_and_provocation() {
        let writes = |drop: Drop| (drop.clears_target(), drop.forgets_provocation());
        assert_eq!(writes(Drop::Disabled), (true, false));
        assert_eq!(writes(Drop::Gone), (true, false));
        assert_eq!(writes(Drop::Ally), (false, false));
        assert_eq!(writes(Drop::Fleet), (true, false));
        assert_eq!(writes(Drop::PaidOff), (true, false));
        assert_eq!(writes(Drop::Dying), (true, true));
    }

    /// The interceptor `cop`'s goal 4 on NPC 2, `quarry`, by the police
    /// flags `police_flags`.
    fn attacked(cop: &Npc, quarry: &Npc, police_flags: u16) -> Hunt {
        let npcs = [cop.clone(), quarry.clone()];
        let govts = govts(0, police_flags);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        interceptor_attack(
            &npcs[0],
            n(2),
            &around,
            INTERCEPTOR_RETREAT,
            TargetRules::default(),
            RuleSource::Engine,
        )
    }

    fn disabled_pirate() -> Npc {
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Disabled;
        pirate
    }

    #[test]
    fn an_outnumbered_interceptor_retreats_before_its_drops() {
        let mut cop = police(n(2));
        cop.goal = Goal::Wait;
        cop.reserves.shield.now = 1.0;
        cop.odds = 5.0;
        let hunt = attacked(&cop, &disabled_pirate(), INTERCEPTORS_RETREAT);
        assert_eq!(hunt, Hunt::of(Goal::Flee(n(2))), "judged as goal 4");
    }

    #[test]
    fn the_disabled_drop_idles_clearing_the_target_and_dry_it_forgets_its_provocation() {
        let hunt = attacked(&police(n(2)), &disabled_pirate(), 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, false)
        );
        let dry = short_of_ammo(police(n(2)), AmmoLeft::Dry, true);
        let hunt = attacked(&dry, &disabled_pirate(), 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, true),
            "goal 3 on -1, stopped (@0x8e228)"
        );
    }

    #[test]
    fn dry_with_no_disabled_drop_it_flees_before_the_high_levels_drops() {
        let dry = short_of_ammo(police(n(2)), AmmoLeft::Dry, true);
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        assert_eq!(attacked(&dry, &dying, 0), Hunt::of(Goal::Flee(n(2))));
    }

    #[test]
    fn the_high_levels_drops_idle_it_with_their_writes_and_otherwise_it_attacks() {
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        let hunt = attacked(&police(n(2)), &dying, 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, true)
        );
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        let hunt = attacked(&police(n(2)), &ally, 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), false, false)
        );
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(
            attacked(&police(n(2)), &pirate, 0),
            Hunt::of(Goal::Attack(n(2)))
        );
    }
}
