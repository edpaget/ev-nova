//! [`FleetEscortAi`]: how a ship with an NPC parent flies and fights:
//! a fleet's escort (`_HyperSpawnFleet` @0x421dc/0x421ef gives it AI type
//! 6 and its lead as parent), a ship a pirate took into its fleet
//! (`_AIPlunderShipContents` @0x84946-0x849cd, the same) and an NPC
//! carrier's fighter (AI type 5). The original flies each by `_EscortAI`
//! (@0x838d2-0x83f5d) under its parent (+0x9a), here [`Npc::leader`]; the
//! engine has no AI types 5 and 6, so a ship with a leader stands for 6 and
//! one with a [`carrier`](Npc::carrier) for 5. A fighter's leader is its
//! carrier (`_LaunchFighter` @0x3d866-0x3d870). [`NovaAi`](crate::ai::NovaAi)
//! routes them here by the rulebook's
//! [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi).
//!
//! **The think** ([`decide_by`]), `_EscortAI` for an NPC parent, in its
//! order:
//!
//! 1. A count wait (goal 0x16, @0x838e4-0x838ef) or an assist (goals 9
//!    and 0xf under a parent other than the player, @0x838f5-0x83911) is
//!    kept.
//! 2. **Lost parent** (@0x83917-0x8397b): its lead gone from the system or
//!    not intact (or, for a fighter, its carrier), it idles (goal 0). The
//!    session takes it out of the fleet with its class's `InherentAI`
//!    (`nova_sim`'s `lost_lead` module), a stand-in for
//!    `_AICheckParentExists`, which runs first on every frame for an NPC
//!    parent (`_DoPlayGameWork` @0x440c6-0x44101) and is
//!    rdm:phase/escort-fleet-fidelity/phase-7-leader-succession's.
//! 3. **Target validity** (@0x83a9b-0x83ad2,
//!    [`valid_target`](escort::valid_target)): a target gone is dropped,
//!    and a disabled one unless its stored word ([`Npc::order_word`],
//!    @0x83ac8) is Attack.
//! 4. **By its order** (+0xc90a, [`order_of`]). Under a lead that is the
//!    player's escort (@0x83b36-0x83b7f), the lead's Defend or Attack
//!    while the lead's goal isn't 0xa ([`Goal::flies_in_form`]), else
//!    Return: the player's escort carrier's fighters fight with it and
//!    dock once it keeps formation or holds any other order; it writes
//!    that order as its word (@0x83b76, @0x83b7f;
//!    [`Decision::order_word`]), which its next target check reads. Under
//!    any other lead, its stored word, the order the lead last issued it
//!    every eighth frame (`_AIIssueEscortOrders`,
//!    [`escort_orders`](crate::ai::escort_orders)), -1 (formation) until
//!    the first:
//!    - **Defend** (@0x83c56-0x83d4e): it keeps its target within
//!      [`DEFEND_KEEP_SQ`] of its lead, inclusive (@0x83cbc), else attacks
//!      the nearest threat to the lead within [`DEFEND_RADIUS`] of it
//!      (`_FindNearestThreatToParent(ship, 550)` @0x83ce7,
//!      [`nearest_threat`]); with none it keeps formation (goal 0xa).
//!    - **Attack** (@0x83d53-0x83e54): it keeps any target, else attacks
//!      the nearest threat to the lead at any distance; with none, it keeps
//!      formation.
//!    - **Hold** (@0x83e59-0x83e74): it stops and waits ([`Goal::Wait`],
//!      goal 6), targeting nothing, running its point defence.
//!    - **Return** (3) for a fighter (AI type 5, @0x83e79-0x83ea3): it
//!      docks with its carrier ([`Goal::Dock`]), targeting nothing.
//!    - **Any other order** (none, or Return for an escort, AI type 6;
//!      @0x83ea8-0x83f5d): it keeps formation on its lead
//!      ([`Goal::Follow`]), guarding against its target while some weapon
//!      it carries reaches it (`_AIInGunRange(ship, target, -1)`,
//!      [`fire::in_reach`]), the player included, else one drawn among the
//!      ships threatening its lead (`_AIFightThreatToParent` @0x81de6,
//!      [`fight_threats`]): one `below(n)` draw, none with no threat. A
//!      guard that is its lead is dropped (@0x83f0d-0x83f1a).
//!
//!    Defend, Attack and formation end its AI wait (+0x4c = -1.0,
//!    @0x83c6b-0x83c73, @0x83d62-0x83d6a, @0x83ea8-0x83eb0,
//!    [`Decision::ends_wait`]): a ship a pirate took ends its 150-frame
//!    wait at its first think. Hold and Return don't. No branch plays a
//!    sound or rolls `Rand(3)` under an NPC parent: the voice is the
//!    player's escorts'.
//!
//! **Its fire** ([`Behaviour::trigger`]). In formation, the think runs
//! `_AIFireGun(ship, 0)` and `_AIFireTurret` on its guard (@0x83f3c,
//! @0x83f4e), whose finds become its pick (+0x72) and raise its fire flag
//! (+0xba). `_LowLevelAIHandler`, which `_AIDispatch` reaches after every
//! think (@0x901f4), clears the flag as it starts (@0x851f6), so neither
//! fires: the gun never fires from formation. Its substates'
//! `_AIEscortFireUnprovoked` ([`escort::unprovoked`]) then fires the best
//! turret at the guard, with its point defence. A gun-only escort fires
//! nothing, keeping its gun find as its pick. Attacking, it fires as any
//! ship does ([`fire::trigger`]), launching its fighters (the high level's
//! launch is every type's); holding, it runs its point defence alone.
//!
//! **Its target** is what it attacks, or its guard unless
//! [`escort::unprovoked`] drops it; holding or docking, none; a kept goal
//! or the lost-parent branch writes none, so it keeps the one it had. In
//! its gate it fires unprovoked ([`escort::fire_unprovoked`], substate 0,
//! @0x85236), and a hit makes it take its attacker as any ship does
//! ([`react::victim`]): it is no Good Samaritan, which is AI types 3 and 4
//! only (@0x8289f-0x828bb). Keeping the hit's attacker by order is
//! rdm:phase/escort-fleet-fidelity/phase-9-escorts-answer-hits-and-run-dry's.
//!
//! **Not built here**, each a later phase's:
//! - jumping with the parent (@0x839fd-0x83a92) and goal 0xb
//!   (@0x83be3-0x83c51), with every +0x50 write: phase 8's;
//! - out of ammunition (@0x83b88-0x83bda): phase 9's;
//! - formation flight on slots, steering at the guard: phase 6's; until
//!   then [`Goal::Follow`] flies within the follow distance of the lead;
//! - the player's death and `_AICheckParentExists`: phase 7's;
//! - the voice: phase 10's.
//!
//! The thinks read the tick's start, so an escort whose lead thinks later
//! in the same tick reads the lead's last goal, not this tick's, and slot
//! order isn't matched: a one-tick lag that crosses no boundary the
//! player sees.

use crate::ai::fire::FireRules;
use crate::ai::{Behaviour, Decision, GateWait, Goal, Reaction, Surroundings, escort, fire, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::escort::{
    DEFEND_KEEP_SQ, DEFEND_RADIUS, EscortOrder, Parent, fight_threats, nearest_threat,
};
use crate::fleet::OrderWord;
use crate::traffic::npc::Npc;

/// The AI of a ship with an NPC parent (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FleetEscortAi {
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

/// The order `npc` follows under its NPC parent among `around`
/// (`_EscortAI` @0x83b36-0x83b7f): under a lead that is the player's
/// escort, the lead's Defend or Attack while its goal isn't 0xa
/// ([`Goal::flies_in_form`]), else Return (3), which docks a fighter;
/// under any other lead, its stored word ([`Npc::order_word`]), which its
/// lead issues ([`escort_orders`](crate::ai::escort_orders)).
#[must_use]
pub fn order_of(npc: &Npc, around: &Surroundings) -> Option<EscortOrder> {
    let Some(lead) = players_escort_lead(npc, around) else {
        return npc.order_word.order();
    };
    Some(match lead.escort_order() {
        Some(order @ (EscortOrder::Defend | EscortOrder::Attack)) if !lead.goal.flies_in_form() => {
            order
        }
        _ => EscortOrder::Dock,
    })
}

/// `npc`'s lead among `around` while that lead is the player's escort.
fn players_escort_lead<'a>(npc: &Npc, around: &Surroundings<'a>) -> Option<&'a Npc> {
    npc.leader
        .and_then(|id| around.npc(id))
        .filter(|lead| lead.escorts_player())
}

/// What `npc`, under its NPC parent and following `order`, decides among
/// `around`, drawing any guard on `chance`: `_EscortAI`'s think (see the
/// module docs). Its target check reads its stored word
/// ([`Npc::order_word`], @0x83ac8), and under a lead that is the player's
/// escort it writes `order` as its word after the check (@0x83b76,
/// @0x83b7f; [`Decision::order_word`]).
pub fn decide_by(
    npc: &Npc,
    order: Option<EscortOrder>,
    around: &Surroundings,
    chance: &mut dyn Chance,
) -> Decision {
    if matches!(npc.goal, Goal::CountWait | Goal::Assist(_)) {
        return goal_of(npc.goal, false);
    }
    let Some(lead) = parent(npc, around) else {
        return goal_of(Goal::Idle, false);
    };
    let target = escort::valid_target(npc, npc.order_word.order(), around);
    let writes = lead.escorts_player().then(|| OrderWord::of(order));
    let goal = |goal: Goal, ends_wait: bool| Decision {
        order_word: writes,
        ..goal_of(goal, ends_wait)
    };
    let follow = |guard: Option<ShipRef>| Goal::Follow {
        lead: lead.id,
        guard,
    };
    let attack_or_follow =
        |threat: Option<ShipRef>| threat.map_or_else(|| follow(None), Goal::Attack);
    match (order, npc.carrier) {
        (Some(EscortOrder::Defend), _) => {
            let near = |ship: &ShipRef| {
                around.state_of(*ship).is_some_and(|at| {
                    let off = at.position - lead.state.position;
                    off.x * off.x + off.y * off.y <= DEFEND_KEEP_SQ
                })
            };
            let kept = target
                .filter(near)
                .or_else(|| nearest_threat(npc, Parent::Npc(lead), around, Some(DEFEND_RADIUS)));
            goal(attack_or_follow(kept), true)
        }
        (Some(EscortOrder::Attack), _) => {
            let kept = target.or_else(|| nearest_threat(npc, Parent::Npc(lead), around, None));
            goal(attack_or_follow(kept), true)
        }
        (Some(EscortOrder::Hold), _) => goal(Goal::Wait, false),
        (Some(EscortOrder::Dock), Some(bay)) => goal(Goal::Dock(bay.ship), false),
        _ => {
            let in_reach = |ship: &ShipRef| {
                around
                    .state_of(*ship)
                    .is_some_and(|at| fire::in_reach(npc, &at))
            };
            let guard = target
                .filter(in_reach)
                .or_else(|| draw(npc, lead, around, chance));
            goal(
                follow(guard.filter(|&ship| ship != ShipRef::Npc(lead.id))),
                true,
            )
        }
    }
}

/// A decision of `goal`, ending its AI wait as `ends_wait` says.
fn goal_of(goal: Goal, ends_wait: bool) -> Decision {
    Decision {
        goal,
        ends_wait,
        ..Decision::default()
    }
}

/// `npc`'s lead among `around`, while it is there and intact, and, for a
/// fighter, its carrier too: `_EscortAI`'s lost-parent test
/// (@0x83917-0x8397b), with `_AICheckParentExists`'s disabled parent
/// (see the module docs).
fn parent<'a>(npc: &Npc, around: &Surroundings<'a>) -> Option<&'a Npc> {
    let intact = |ship: ShipRef| around.condition_of(ship) == Some(Condition::Intact);
    if let Some(bay) = npc.carrier
        && !intact(bay.ship)
    {
        return None;
    }
    npc.leader
        .and_then(|id| around.npc(id))
        .filter(|lead| lead.condition == Condition::Intact)
}

/// One of the ships threatening `lead` among `around` that `npc` may
/// fight, drawn on `chance` (`_AIFightThreatToParent` @0x81de6, a uniform
/// draw where the original rolls `Rand(64)` until a slot passes); none,
/// drawing nothing, with none.
fn draw(npc: &Npc, lead: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Option<ShipRef> {
    let threats = fight_threats(npc, Parent::Npc(lead), around);
    if threats.is_empty() {
        return None;
    }
    let count = u32::try_from(threats.len()).unwrap_or(u32::MAX);
    threats.get(chance.below(count) as usize).copied()
}

impl Behaviour for FleetEscortAi {
    /// [`FleetEscortAi::decision`]'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// [`decide_by`] under [`order_of`].
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        decide_by(npc, order_of(npc, around), around, chance)
    }

    /// Attacking or sniping, [`fire::trigger`]; in formation with a guard,
    /// the think's gun find (and its turret find, which is the low level's,
    /// on the same guard) kept as its pick, and the low level's
    /// [`escort::unprovoked`] fired, with its point defence; a disabled
    /// escort runs no think (`_AIDispatch` @0x8fece-0x8ff13). Holding, its
    /// point defence (`_AIFireTurret` with no target, @0x83e74). Anything
    /// else, nothing.
    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        match npc.goal {
            Goal::Attack(_) | Goal::Snipe(_) => fire::trigger(npc, around, self.fire),
            Goal::Follow {
                guard: Some(guard), ..
            } => {
                let low = escort::unprovoked(npc, Some(guard), around, self.fire);
                if low.drops_target {
                    return low.trigger;
                }
                let gun = if npc.condition == Condition::Intact {
                    fire::gun_find(npc, guard, around, self.fire)
                } else {
                    None
                };
                Trigger {
                    keep: low.trigger.only.or(gun),
                    ..low.trigger
                }
            }
            Goal::Wait => Trigger {
                point_defence: true,
                ..Trigger::default()
            },
            _ => Trigger::default(),
        }
    }

    /// What it attacks; in formation its guard, unless
    /// [`escort::unprovoked`] drops it (+0x70 = -1, @0x80e9d); holding,
    /// docking or keeping formation with no guard, none; idle (its lead
    /// lost) or keeping a count wait or an assist, the target it has, as
    /// those branches write none.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        match npc.goal {
            Goal::Attack(ship) | Goal::Snipe(ship) => Some(ship),
            Goal::Follow { guard, .. } => {
                guard.filter(|_| !escort::unprovoked(npc, guard, around, self.fire).drops_target)
            }
            Goal::Idle | Goal::CountWait | Goal::Assist(_) => npc.target,
            _ => None,
        }
    }

    /// [`escort::fire_unprovoked`], as for every escort or fighter (AI type
    /// above 4, @0x80e59).
    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        escort::fire_unprovoked(npc, around, self.fire)
    }

    /// [`react::victim`]: `_DamageShip`'s reaction block reads no AI type,
    /// and `_DoGoodSamaritan` takes AI 3 and 4 alone (@0x8289f-0x828bb).
    /// Its next think may set another target by its order; keeping the
    /// hit's attacker is
    /// rdm:phase/escort-fleet-fidelity/phase-9-escorts-answer-hits-and-run-dry's.
    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::victim(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, govts, n, player, ship};
    use crate::ai::{Help, escort, fire, react};
    use crate::bay::Carrier;
    use crate::catalog::{EscortClass, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::fleet::OrderWord;
    use crate::traffic::npc::{AiType, NpcId};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    /// The lead, NPC 1.
    const LEAD: NpcId = NpcId(1);

    /// The orders a test runs through: none (-1), then each order.
    const ORDERS: [Option<EscortOrder>; 5] = [
        None,
        Some(EscortOrder::Defend),
        Some(EscortOrder::Attack),
        Some(EscortOrder::Hold),
        Some(EscortOrder::Dock),
    ];

    /// The lead, a medium pirate warship at the centre with `goal`.
    fn lead(goal: Goal) -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.class = EscortClass::Medium;
        npc.goal = goal;
        npc
    }

    /// A turret (140) reaching 300 pixels, and a gun (141) as far: 332
    /// with the margin, as its class's stock weapons reach.
    fn armed() -> Armament {
        Armament::new([(turret(), 1), (gun(), 1)])
    }

    fn turret() -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 4,
            speed: 1000,
            count: 30,
            energy_dmg: 5,
            ..weapon(140)
        })
    }

    fn gun() -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            speed: 1000,
            count: 30,
            energy_dmg: 5,
            ..weapon(141)
        })
    }

    /// The escort, NPC 2, a medium warship at (0, 50) led by the lead,
    /// armed with [`armed`], idle.
    fn escort() -> Npc {
        let mut npc = ship(2, PIRATES, AiType::Warship, 0.0, 50.0);
        npc.class = EscortClass::Medium;
        npc.leader = Some(LEAD);
        npc.armament = armed();
        npc.hull.stock_reach = Some(332.0);
        npc
    }

    /// `npc` targeting `target`.
    fn targeting(mut npc: Npc, target: ShipRef) -> Npc {
        npc.target = Some(target);
        npc
    }

    /// The escort as a fighter of `carrier`.
    fn fighter(carrier: ShipRef) -> Npc {
        Npc {
            class: EscortClass::Fighter,
            carrier: Some(Carrier {
                ship: carrier,
                window: 100.0,
                reach: 40.0,
            }),
            ..escort()
        }
    }

    /// NPC `id`, a medium trader warship at (`x`, `y`) attacking
    /// `target`.
    fn raider(id: u32, x: f32, y: f32, target: ShipRef) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc.goal = Goal::Attack(target);
        npc
    }

    /// NPC `id`, a medium trader at (`x`, `y`), idle.
    fn bystander(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc
    }

    /// NPC `id` at (`x`, `y`) led by the lead, attacking `target`.
    fn sibling(id: u32, x: f32, y: f32, target: ShipRef) -> Npc {
        let mut npc = raider(id, x, y, target);
        npc.leader = Some(LEAD);
        npc
    }

    /// What `npcs[1]` decides under `order`, stored as its word too, among
    /// `npcs`, the player at (`x`, `y`), drawing on `draws`; and the bounds
    /// of the draws asked.
    fn decided_with(
        npcs: &[Npc],
        order: Option<EscortOrder>,
        (x, y): (f32, f32),
        draws: &[u32],
    ) -> (Decision, Vec<u32>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut chance = Draws::of(draws);
        let stored = Npc {
            order_word: OrderWord::of(order),
            ..npcs[1].clone()
        };
        let decision = decide_by(&stored, order, &around, &mut chance);
        (decision, chance.asked)
    }

    /// [`decided_with`] with the player far off, at (0, 5000).
    fn decided(npcs: &[Npc], order: Option<EscortOrder>, draws: &[u32]) -> (Goal, Vec<u32>) {
        let (decision, asked) = decided_with(npcs, order, (0.0, 5000.0), draws);
        (decision.goal, asked)
    }

    /// Keeping formation on the lead, guarding `guard`.
    fn follow(guard: Option<ShipRef>) -> Goal {
        Goal::Follow { lead: LEAD, guard }
    }

    #[test]
    fn it_keeps_a_count_wait_or_an_assist_whatever_its_order() {
        // `_EscortAI` returns at goal 0x16 (@0x838e4-0x838ef), and under a
        // parent other than the player at goal 9 or 0xf (@0x838f5-0x83911).
        for goal in [
            Goal::CountWait,
            Goal::Assist(Help::Refuel),
            Goal::Assist(Help::Repair),
        ] {
            let mut kept = escort();
            kept.goal = goal;
            let npcs = [lead(Goal::Idle), kept, raider(3, 0.0, 100.0, n(1))];
            for order in ORDERS {
                let (decision, asked) = decided_with(&npcs, order, (0.0, 300.0), &[0]);
                assert_eq!(decision.goal, goal, "{goal:?} {order:?}");
                assert!(!decision.ends_wait, "{goal:?} {order:?}");
                assert!(asked.is_empty(), "{goal:?} {order:?}");
            }
        }
    }

    #[test]
    fn with_its_lead_gone_or_not_intact_it_idles_drawing_nothing() {
        // The lost-parent branch (@0x83917-0x8397b): goal 0; the session
        // clears the lead and gives it its class's `InherentAI`.
        let threat = raider(3, 0.0, 100.0, n(2));
        let mut disabled = lead(Goal::Idle);
        disabled.condition = Condition::Disabled;
        let mut dying = lead(Goal::Idle);
        dying.condition = Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
        let gone = bystander(9, 3000.0, 0.0);
        for first in [gone, disabled, dying] {
            let npcs = [first.clone(), escort(), threat.clone()];
            for order in ORDERS {
                let (decision, asked) = decided_with(&npcs, order, (0.0, 300.0), &[0]);
                assert_eq!(decision.goal, Goal::Idle, "{:?} {order:?}", first.condition);
                assert!(!decision.ends_wait);
                assert!(asked.is_empty());
            }
        }
        // A fighter whose carrier is gone, its lead still there.
        let npcs = [lead(Goal::Idle), fighter(n(4)), threat];
        assert_eq!(
            decided(&npcs, Some(EscortOrder::Dock), &[0]),
            (Goal::Idle, vec![])
        );
    }

    #[test]
    fn defending_it_keeps_its_target_within_about_639_pixels_of_its_lead() {
        // 408375 is between 639² (408321) and 640² (409600), inclusive
        // (@0x83cbc), measured from the lead.
        let defend = Some(EscortOrder::Defend);
        let fighting = targeting(escort(), n(3));
        let npcs = [lead(Goal::Idle), fighting.clone(), bystander(3, 0.0, 639.0)];
        assert_eq!(decided(&npcs, defend, &[]), (Goal::Attack(n(3)), vec![]));
        let npcs = [lead(Goal::Idle), fighting.clone(), bystander(3, 0.0, 640.0)];
        assert_eq!(decided(&npcs, defend, &[]), (follow(None), vec![]));
        let npcs = [lead(Goal::Idle), fighting.clone(), bystander(3, 640.0, 0.0)];
        assert_eq!(
            decided(&npcs, defend, &[]),
            (follow(None), vec![]),
            "across too"
        );
        // 451² x 2 = 406802 is within, and 452² x 2 = 408608 is not.
        let npcs = [
            lead(Goal::Idle),
            fighting.clone(),
            bystander(3, 451.0, 451.0),
        ];
        assert_eq!(decided(&npcs, defend, &[]), (Goal::Attack(n(3)), vec![]));
        let npcs = [
            lead(Goal::Idle),
            fighting.clone(),
            bystander(3, 452.0, -452.0),
        ];
        assert_eq!(decided(&npcs, defend, &[]), (follow(None), vec![]));
        let mut moved = lead(Goal::Idle);
        moved.state.position = crate::geometry::Vec2::new(1000.0, 0.0);
        let npcs = [moved, fighting, bystander(3, 1000.0, 639.0)];
        assert_eq!(
            decided(&npcs, defend, &[]),
            (Goal::Attack(n(3)), vec![]),
            "from the lead, not the escort"
        );
    }

    #[test]
    fn defending_it_takes_the_nearest_threat_within_550_of_its_lead() {
        let defend = Some(EscortOrder::Defend);
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 550.0, n(1))];
        assert_eq!(decided(&npcs, defend, &[]), (Goal::Attack(n(3)), vec![]));
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 551.0, n(1))];
        assert_eq!(decided(&npcs, defend, &[]), (follow(None), vec![]));
        let npcs = [
            lead(Goal::Idle),
            escort(),
            raider(3, 0.0, 400.0, n(1)),
            raider(4, 0.0, -300.0, n(1)),
        ];
        assert_eq!(
            decided(&npcs, defend, &[]),
            (Goal::Attack(n(4)), vec![]),
            "the nearer"
        );
        // A disabled target is dropped at the think, and another taken.
        let mut down = raider(3, 0.0, 100.0, n(1));
        down.condition = Condition::Disabled;
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            down,
            raider(4, 0.0, -300.0, n(1)),
        ];
        assert_eq!(decided(&npcs, defend, &[]), (Goal::Attack(n(4)), vec![]));
    }

    #[test]
    fn attacking_it_keeps_its_target_disabled_or_not_else_the_nearest_threat_anywhere() {
        let attack = Some(EscortOrder::Attack);
        let mut down = bystander(3, 5000.0, 0.0);
        down.condition = Condition::Disabled;
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            down,
            raider(4, 10.0, 0.0, n(1)),
        ];
        assert_eq!(decided(&npcs, attack, &[]), (Goal::Attack(n(3)), vec![]));
        let npcs = [
            lead(Goal::Idle),
            escort(),
            raider(3, 5000.0, 0.0, n(1)),
            raider(4, 3000.0, 0.0, n(1)),
        ];
        assert_eq!(decided(&npcs, attack, &[]), (Goal::Attack(n(4)), vec![]));
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(9)),
            bystander(3, 10.0, 0.0),
        ];
        assert_eq!(
            decided(&npcs, attack, &[]),
            (follow(None), vec![]),
            "its target gone, and no threat"
        );
    }

    #[test]
    fn holding_it_waits() {
        // Target and nav -1, goal 6 (@0x83e59-0x83e74).
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            raider(3, 0.0, 100.0, n(1)),
        ];
        let (decision, asked) = decided_with(&npcs, Some(EscortOrder::Hold), (0.0, 0.0), &[0]);
        assert_eq!((decision.goal, asked), (Goal::Wait, vec![]));
        assert!(!decision.ends_wait);
    }

    #[test]
    fn returning_a_fighter_docks_with_its_carrier_and_an_escort_forms() {
        // AI type 5's Return docks (@0x83e79-0x83ea3); AI type 6's falls
        // to the formation block (@0x83ea8).
        let dock = Some(EscortOrder::Dock);
        let mut carrier = bystander(4, 0.0, -100.0);
        carrier.leader = Some(LEAD);
        let npcs = [
            lead(Goal::Idle),
            targeting(fighter(n(4)), n(3)),
            raider(3, 0.0, 100.0, n(1)),
            carrier,
        ];
        let (decision, asked) = decided_with(&npcs, dock, (0.0, 5000.0), &[0]);
        assert_eq!((decision.goal, asked), (Goal::Dock(n(4)), vec![]));
        assert!(!decision.ends_wait);
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 100.0, n(1))];
        assert_eq!(decided(&npcs, dock, &[0]), (follow(Some(n(3))), vec![1]));
        assert_eq!(decided(&npcs, dock, &[0]), decided(&npcs, None, &[0]));
    }

    #[test]
    fn in_formation_it_keeps_a_guard_in_reach_the_player_too() {
        // `_AIInGunRange(ship, target, -1)` (@0x83eb3-0x83ee4): the turret
        // and gun reach 332 from the escort at (0, 50).
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            bystander(3, 0.0, 382.0),
            raider(4, 0.0, 60.0, n(1)),
        ];
        assert_eq!(decided(&npcs, None, &[]), (follow(Some(n(3))), vec![]));
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            bystander(3, 0.0, 383.0),
            raider(4, 0.0, 60.0, n(1)),
        ];
        assert_eq!(
            decided(&npcs, None, &[0]),
            (follow(Some(n(4))), vec![1]),
            "beyond it: drawn among the threats"
        );
        let npcs = [lead(Goal::Idle), targeting(escort(), ShipRef::Player)];
        assert_eq!(
            decided_with(&npcs, None, (0.0, 382.0), &[]).0.goal,
            follow(Some(ShipRef::Player)),
            "the player, threat or not"
        );
        assert_eq!(
            decided_with(&npcs, None, (0.0, 383.0), &[]).0.goal,
            follow(None)
        );
    }

    #[test]
    fn in_formation_it_draws_a_guard_once_among_the_threats_and_not_with_none() {
        let npcs = [
            lead(Goal::Idle),
            escort(),
            raider(3, 0.0, 100.0, n(1)),
            bystander(4, 0.0, 50.0),
            raider(5, 4000.0, 0.0, n(1)),
        ];
        assert_eq!(decided(&npcs, None, &[0]), (follow(Some(n(3))), vec![2]));
        assert_eq!(
            decided(&npcs, None, &[1]),
            (follow(Some(n(5))), vec![2]),
            "at any distance"
        );
        let npcs = [lead(Goal::Idle), escort(), bystander(3, 0.0, 100.0)];
        assert_eq!(decided(&npcs, None, &[0]), (follow(None), vec![]));
    }

    #[test]
    fn in_formation_it_never_guards_its_lead() {
        // A target equal to the parent is dropped after the keep
        // (@0x83f0d-0x83f1a), with no draw.
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(1)),
            raider(3, 0.0, 100.0, n(1)),
        ];
        assert_eq!(decided(&npcs, None, &[0]), (follow(None), vec![]));
    }

    #[test]
    fn a_carriers_fighter_turns_on_the_only_ship_attacking_its_carrier_or_another_of_its_fighters()
    {
        // PLS 31: `_ExtendedIsThreatToShip(parent, ship)` counts a ship
        // attacking one the parent leads (@0x81ab2-0x81b59).
        let carrier = || lead(Goal::Idle);
        let other_fighter = || sibling(4, 0.0, -200.0, n(9));
        for target in [n(1), n(4)] {
            let npcs = [
                carrier(),
                fighter(n(1)),
                raider(3, 0.0, 300.0, target),
                other_fighter(),
                bystander(5, 0.0, 30.0),
            ];
            for order in [EscortOrder::Defend, EscortOrder::Attack] {
                assert_eq!(
                    decided(&npcs, Some(order), &[]),
                    (Goal::Attack(n(3)), vec![]),
                    "{target:?} {order:?}"
                );
            }
            assert_eq!(
                decided(&npcs, None, &[0]),
                (follow(Some(n(3))), vec![1]),
                "{target:?}"
            );
        }
    }

    #[test]
    fn a_fighter_of_the_same_carrier_attacking_the_carrier_is_no_threat() {
        let npcs = [
            lead(Goal::Idle),
            fighter(n(1)),
            sibling(4, 0.0, 100.0, n(1)),
        ];
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            assert_eq!(decided(&npcs, Some(order), &[]), (follow(None), vec![]));
        }
        assert_eq!(decided(&npcs, None, &[0]), (follow(None), vec![]));
    }

    #[test]
    fn an_npc_escort_whose_lead_is_hostile_to_the_player_counts_the_player_as_a_threat() {
        // `_IsShipANearbyThreatToParent` with slot 0 (@0x81a89-0x81a9c), and
        // `_AIFightThreatToParent`'s slot 0 (@0x81e83-0x81e9d).
        let npcs = [lead(Goal::Attack(ShipRef::Player)), escort()];
        let (decision, asked) = decided_with(&npcs, Some(EscortOrder::Defend), (0.0, 500.0), &[]);
        assert_eq!(
            (decision.goal, asked),
            (Goal::Attack(ShipRef::Player), vec![])
        );
        let (decision, asked) = decided_with(&npcs, None, (0.0, 500.0), &[0]);
        assert_eq!(
            (decision.goal, asked),
            (follow(Some(ShipRef::Player)), vec![1])
        );
        let calm = [lead(Goal::Idle), escort()];
        let (decision, asked) = decided_with(&calm, None, (0.0, 500.0), &[0]);
        assert_eq!((decision.goal, asked), (follow(None), vec![]));
    }

    #[test]
    fn defend_attack_and_formation_end_the_ai_wait_and_hold_and_docking_dont() {
        // +0x4c = -1.0 at @0x83c6b-0x83c73, @0x83d62-0x83d6a and
        // @0x83ea8-0x83eb0; none in the hold or the return.
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 100.0, n(1))];
        for order in [
            None,
            Some(EscortOrder::Defend),
            Some(EscortOrder::Attack),
            Some(EscortOrder::Dock),
        ] {
            assert!(
                decided_with(&npcs, order, (0.0, 5000.0), &[0]).0.ends_wait,
                "{order:?}"
            );
        }
        let npcs = [lead(Goal::Idle), escort()];
        assert!(decided_with(&npcs, None, (0.0, 5000.0), &[]).0.ends_wait);
        assert!(
            !decided_with(&npcs, Some(EscortOrder::Hold), (0.0, 5000.0), &[])
                .0
                .ends_wait
        );
        let npcs = [lead(Goal::Idle), fighter(n(1))];
        assert!(
            !decided_with(&npcs, Some(EscortOrder::Dock), (0.0, 5000.0), &[])
                .0
                .ends_wait
        );
    }

    /// The order a fighter of the lead, NPC 1, takes when the lead is the
    /// player's escort following `order` with `goal`.
    fn players_escorts_fighter_order(
        order: Option<EscortOrder>,
        goal: Goal,
    ) -> Option<EscortOrder> {
        let mut escort = lead(goal);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order,
        });
        let npcs = [escort, fighter(n(1))];
        order_of(&npcs[1], &Surroundings::new(&[], &npcs))
    }

    #[test]
    fn a_fighter_of_the_players_escort_takes_its_defend_or_attack_out_of_formation_else_return() {
        // `_EscortAI` @0x83b36-0x83b7f: a parent whose own parent is the
        // player; its order 1 or 2 unless its goal is 0xa, else 3.
        let fighting = Goal::Attack(n(3));
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            assert_eq!(
                players_escorts_fighter_order(Some(order), fighting),
                Some(order)
            );
            for idle in [Goal::Idle, Goal::Wait] {
                assert_eq!(
                    players_escorts_fighter_order(Some(order), idle),
                    Some(order),
                    "goal {idle:?} isn't 0xa"
                );
            }
            for in_form in [
                Goal::Formation { guard: None },
                Goal::Follow {
                    lead: NpcId(9),
                    guard: None,
                },
                Goal::Hold,
            ] {
                assert_eq!(
                    players_escorts_fighter_order(Some(order), in_form),
                    Some(EscortOrder::Dock),
                    "goal 0xa: {in_form:?}"
                );
            }
        }
        for order in [None, Some(EscortOrder::Hold), Some(EscortOrder::Dock)] {
            assert_eq!(
                players_escorts_fighter_order(order, fighting),
                Some(EscortOrder::Dock),
                "{order:?}"
            );
        }
        // Under any other lead, its stored word, whatever the lead does.
        for word in [-1, 0, 1, 2, 3, 4, 5] {
            let mut stored = fighter(n(1));
            stored.order_word = OrderWord(word);
            for goal in [fighting, Goal::Idle] {
                let npcs = [lead(goal), stored.clone()];
                assert_eq!(
                    order_of(&npcs[1], &Surroundings::new(&[], &npcs)),
                    OrderWord(word).order(),
                    "{word} {goal:?}"
                );
            }
        }
    }

    /// What the escort, or a fighter of the lead when `fighter`, holding
    /// `word`, decides under a lead attacking NPC 3, which attacks it from
    /// 500 off; the lead escorts the player under the order `players` gives,
    /// if any.
    fn decision_holding(word: i16, fighter: bool, players: Option<EscortOrder>) -> Decision {
        let mut first = lead(Goal::Attack(n(3)));
        if let Some(order) = players {
            first.escort = Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: Some(order),
            });
        }
        let mut second = if fighter {
            self::fighter(n(1))
        } else {
            escort()
        };
        second.order_word = OrderWord(word);
        let npcs = [first, second, raider(3, 0.0, 500.0, n(1))];
        let govts = govts(0, 0);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        FleetEscortAi::default().decision(&npcs[1], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn it_decides_by_its_stored_word() {
        // `_EscortAI` reads +0xc90a (@0x83c56): -1 and 0 form, 3 docks a
        // fighter and forms an escort, 1 and 2 fight.
        for word in [-1, 0] {
            let decision = decision_holding(word, false, None);
            assert_eq!(decision.goal, follow(Some(n(3))), "{word}");
            assert!(decision.ends_wait);
            assert_eq!(decision.order_word, None, "an NPC lead's ship writes none");
        }
        assert_eq!(decision_holding(3, true, None).goal, Goal::Dock(n(1)));
        assert_eq!(decision_holding(3, false, None).goal, follow(Some(n(3))));
        for word in [1, 2] {
            let decision = decision_holding(word, false, None);
            assert_eq!(decision.goal, Goal::Attack(n(3)), "{word}");
            assert!(decision.ends_wait);
        }
        assert_eq!(decision_holding(4, false, None).goal, Goal::Wait);
        let ai = FleetEscortAi::default();
        let mut stored = escort();
        stored.order_word = OrderWord(2);
        let npcs = [lead(Goal::Idle), stored, raider(3, 0.0, 3000.0, n(1))];
        let govts = govts(0, 0);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            ai.decide(&npcs[1], &around, &mut Draws::of(&[0])),
            Goal::Attack(n(3)),
            "decide is the decision's goal"
        );
    }

    #[test]
    fn a_fighter_of_the_players_escort_flies_its_parents_order_and_writes_it_as_its_word() {
        // @0x83b36-0x83b7f: the parent's Attack, written to its own +0xc90a.
        let decision = decision_holding(-1, true, Some(EscortOrder::Attack));
        assert_eq!(decision.goal, Goal::Attack(n(3)));
        assert_eq!(decision.order_word, Some(OrderWord(2)));
        let decision = decision_holding(2, true, Some(EscortOrder::Hold));
        assert_eq!(decision.goal, Goal::Dock(n(1)), "Return: it docks");
        assert_eq!(decision.order_word, Some(OrderWord(3)));
    }

    #[test]
    fn its_target_check_reads_its_stored_word_not_its_order() {
        // Step 5 (@0x83ac8) comes before the escort-fighter write
        // (@0x83b36): a disabled target is kept only under a stored Attack.
        let mut down = bystander(3, 0.0, 600.0);
        down.condition = Condition::Disabled;
        let decide = |word: i16, order: EscortOrder| {
            let mut stored = targeting(escort(), n(3));
            stored.order_word = OrderWord(word);
            let npcs = [lead(Goal::Idle), stored, down.clone()];
            let govts = govts(0, 0);
            let around = Surroundings {
                govts: &govts,
                ..Surroundings::new(&[], &npcs)
            };
            decide_by(&npcs[1], Some(order), &around, &mut Draws::of(&[])).goal
        };
        assert_eq!(decide(2, EscortOrder::Defend), Goal::Attack(n(3)), "kept");
        assert_eq!(decide(1, EscortOrder::Attack), follow(None), "dropped");
    }

    /// What `ai` holds for `npcs[1]` once it decided `goal`: its trigger
    /// and target, the player at (0, 5000).
    fn command(npcs: &[Npc], goal: Goal) -> (Trigger, Option<ShipRef>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut npc = npcs[1].clone();
        npc.goal = goal;
        let ai = FleetEscortAi::default();
        (ai.trigger(&npc, &around), ai.target(&npc, &around))
    }

    /// A turret pick `weapon` fired, with the point defence.
    fn fired(weapon: i16) -> Trigger {
        Trigger {
            only: Some(WeaponId(weapon)),
            keep: Some(WeaponId(weapon)),
            point_defence: true,
            ..Trigger::default()
        }
    }

    const DEFENCE: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: None,
    };

    #[test]
    fn in_formation_it_fires_its_turret_pick_at_its_guard_with_its_point_defence() {
        // The think's `_AIFireGun(ship, 0)` and `_AIFireTurret` leave their
        // picks (@0x83f3c-0x83f4e), `_LowLevelAIHandler` clears the flag
        // (@0x851f6), and `_AIEscortFireUnprovoked` fires the turret pick
        // (@0x80e4f-0x80ea8).
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, -50.0, n(1))];
        assert_eq!(command(&npcs, follow(Some(n(3)))), (fired(140), Some(n(3))));
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 1000.0, n(1))];
        assert_eq!(
            command(&npcs, follow(Some(n(3)))),
            (DEFENCE, Some(n(3))),
            "beyond every weapon: point defence alone"
        );
    }

    #[test]
    fn a_gun_only_escort_in_formation_fires_nothing_keeping_its_gun_find() {
        let mut gunner = escort();
        gunner.armament = Armament::new([(gun(), 1)]);
        let npcs = [lead(Goal::Idle), gunner, raider(3, 0.0, -50.0, n(1))];
        assert_eq!(
            command(&npcs, follow(Some(n(3)))),
            (
                Trigger {
                    keep: Some(WeaponId(141)),
                    ..DEFENCE
                },
                Some(n(3))
            )
        );
        // A disabled escort runs no think and no point defence.
        let mut down = npcs.clone();
        down[1].condition = Condition::Disabled;
        assert_eq!(
            command(&down, follow(Some(n(3)))),
            (Trigger::default(), Some(n(3)))
        );
    }

    #[test]
    fn in_formation_it_drops_a_guard_disabled_or_gone_and_fires_nothing() {
        let mut down = raider(3, 0.0, -50.0, n(1));
        down.condition = Condition::Disabled;
        let npcs = [lead(Goal::Idle), escort(), down];
        assert_eq!(
            command(&npcs, follow(Some(n(3)))),
            (Trigger::default(), None)
        );
        let npcs = [lead(Goal::Idle), escort()];
        assert_eq!(
            command(&npcs, follow(Some(n(3)))),
            (Trigger::default(), None)
        );
        assert_eq!(command(&npcs, follow(None)), (Trigger::default(), None));
    }

    #[test]
    fn attacking_it_fires_as_any_ship_does_launching_its_fighters() {
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, -50.0, n(1))];
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let mut fighting = npcs[1].clone();
        fighting.goal = Goal::Attack(n(3));
        let (trigger, target) = command(&npcs, Goal::Attack(n(3)));
        assert_eq!(
            trigger,
            fire::trigger(&fighting, &around, FireRules::default())
        );
        assert!(trigger.bays, "its high level's launch");
        assert_eq!(target, Some(n(3)));
        assert_eq!(command(&npcs, Goal::Snipe(n(3))).1, Some(n(3)));
    }

    #[test]
    fn holding_it_runs_its_point_defence_and_docking_or_idle_nothing() {
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            raider(3, 0.0, -50.0, n(1)),
        ];
        assert_eq!(command(&npcs, Goal::Wait), (DEFENCE, None));
        assert_eq!(command(&npcs, Goal::Dock(n(1))), (Trigger::default(), None));
        assert_eq!(
            command(&npcs, Goal::Idle),
            (Trigger::default(), Some(n(3))),
            "the lost-parent branch writes no target"
        );
        assert_eq!(
            command(&npcs, Goal::CountWait),
            (Trigger::default(), Some(n(3)))
        );
    }

    #[test]
    fn waiting_in_its_gate_it_fires_unprovoked_and_hit_it_answers_as_a_victim() {
        let npcs = [
            lead(Goal::Idle),
            targeting(escort(), n(3)),
            raider(3, 0.0, -50.0, n(1)),
        ];
        let around = Surroundings::new(&[], &npcs);
        let ai = FleetEscortAi::default();
        assert_eq!(
            ai.gate_wait(&npcs[1], &around),
            escort::fire_unprovoked(&npcs[1], &around, FireRules::default())
        );
        assert!(ai.gate_wait(&npcs[1], &around).trigger.point_defence);
        for strike in [
            crate::combat::fixture::strike(n(2), n(3)),
            crate::combat::fixture::strike(n(3), ShipRef::Player),
        ] {
            assert_eq!(
                ai.react(&npcs[1], &strike, &around),
                react::victim(&npcs[1], &strike, &around)
            );
        }
        assert_eq!(
            ai.react(
                &npcs[1],
                &crate::combat::fixture::strike(n(3), ShipRef::Player),
                &around
            ),
            Reaction::default(),
            "no Good Samaritan"
        );
    }
}
