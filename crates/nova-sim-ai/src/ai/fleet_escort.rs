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
//! 2. **Lost parent** (@0x83917-0x8397b): with no lead, or its lead gone
//!    from the system or not intact, it idles (goal 0) and leaves its fleet
//!    with its class's `InherentAI` (@0x83c4a, [`Decision::leaves_fleet`]),
//!    its target kept. A fighter's lead is its carrier. The succession
//!    (`_AICheckParentExists`,
//!    [`traffic::succession`](crate::traffic::succession)) runs before
//!    every think and gives every ship of a lost lead another lead or
//!    none, so this meets only a ship with no leader: one the disabled
//!    reset left ([`traffic::disabled_reset`](crate::traffic::disabled_reset)),
//!    whole again, such as a former fighter, whose AI type 5 lasts until
//!    this think. It writes +0x50 = -1.0 (@0x83974; the traffic gives up
//!    the stage, see [`jump_with_leader`](crate::traffic::jump_with_leader)).
//! 3. **Target validity** (@0x83a9b-0x83ad2,
//!    [`valid_target`](escort::valid_target)): a target gone is dropped,
//!    and a disabled one unless its stored word ([`Npc::order_word`],
//!    @0x83ac8) is Attack.
//! 4. **Out of ammunition** (@0x83b88-0x83bda,
//!    [`out_of_ammo::escort_word`]): on Defend or Attack with `Flags2`
//!    0x0080, a fighter short of any ammunition takes Return (word 3) and
//!    any other ship fully dry formation (word 0). It writes the word under
//!    any lead ([`Decision::order_word`]): under one the player doesn't
//!    lead it stands as [`Npc::order_word`] until that lead's next order
//!    frame. It runs before the jump's return, so a ship taking its
//!    leader's jump writes it too.
//! 5. **By its order** (+0xc90a, [`order_of`], then the fallback's
//!    word if it wrote one). Under a lead that is the player's escort
//!    (@0x83b36-0x83b7f), the lead's Defend or Attack
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
//!      [`DEFEND_KEEP_SQ`](crate::escort::DEFEND_KEEP_SQ) of its lead,
//!      inclusive (@0x83cbc; [`escort::kept_by_order`]), else attacks
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
//! ([`react::victim`]), which its next think keeps by its order as any
//! target ([`escort::kept_by_order`]): it is no Good Samaritan, which is
//! AI types 3 and 4 only (@0x8289f-0x828bb).
//!
//! **Jumping with the parent** (@0x839fd-0x83a92, @0x83be3-0x83c51):
//! after the lost parent, a lead that draws its followers
//! ([`jump_with_leader::draws_followers`](crate::traffic::jump_with_leader::draws_followers):
//! sub 4 or 0xd, goal 2's brake, or +0x50 above 0) puts it on goal 0xb:
//! its target dropped, following the lead's jump
//! ([`Decision::follows_jump`]), or leaving its fleet when it flies
//! inertialess. The order word is still written under a lead that is the
//! player's escort; no order block runs, so its AI wait is kept.
//! Defend, Attack and formation write +0x50 = -1.0 beside +0x4c,
//! whoever the parent ([`Decision::ends_wait`]).
//!
//! **Not built here**, each a later phase's:
//! - formation flight on slots, steering at the guard: phase 6's; until
//!   then [`Goal::Follow`] flies within the follow distance of the lead;
//! - the voice: phase 10's.
//!
//! The thinks read the tick's start, so an escort whose lead thinks later
//! in the same tick reads the lead's last goal, not this tick's, and slot
//! order isn't matched: a one-tick lag that crosses no boundary the
//! player sees.

use crate::ai::fire::FireRules;
use crate::ai::regroup::Kept;
use crate::ai::{
    Behaviour, Decision, GateWait, Goal, Reaction, Surroundings, escort, fire, out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::escort::{DEFEND_RADIUS, EscortOrder, Parent, fight_threats, nearest_threat};
use crate::fleet::OrderWord;
use crate::traffic::jump_with_leader;
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
///
/// The lead's order is the one after the lead's own out-of-ammunition
/// fallback ([`out_of_ammo::escort_order`], @0x83b56): an escort carrier
/// fully dry keeps formation, so it sends its fighters home. The original
/// reads the word the lead's last think left; this works it out from the
/// lead's state at the fighter's think. The lag is at most one think
/// interval and crosses no boundary the player sees.
#[must_use]
pub fn order_of(npc: &Npc, around: &Surroundings) -> Option<EscortOrder> {
    let Some(lead) = players_escort_lead(npc, around) else {
        return npc.order_word.order();
    };
    Some(match out_of_ammo::escort_order(lead, lead.escort_order()) {
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
        return Decision {
            leaves_fleet: true,
            ..goal_of(Goal::Idle, false)
        };
    };
    let target = escort::valid_target(npc, npc.order_word.order(), around);
    // The fallback writes its word under any lead (@0x83bc9, @0x83bda);
    // under one the player doesn't lead, the lead's next order frame
    // overwrites it.
    let fallback = out_of_ammo::escort_word(npc, order);
    let order = fallback.map_or(order, OrderWord::order);
    let writes = fallback.or_else(|| lead.escorts_player().then(|| OrderWord::of(order)));
    if jump_with_leader::draws_followers(lead) {
        // The entry (@0x839fd-0x83a92), then goal 0xb's block
        // (@0x83be3): no order block runs.
        return Decision {
            goal: Goal::Follow {
                lead: lead.id,
                guard: None,
            },
            clears_target: true,
            follows_jump: true,
            order_word: writes,
            ..Decision::default()
        };
    }
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
        (Some(fighting @ (EscortOrder::Defend | EscortOrder::Attack)), _) => {
            let radius = (fighting == EscortOrder::Defend).then_some(DEFEND_RADIUS);
            let kept = escort::kept_by_order(order, target, lead.state.position, around)
                .or_else(|| nearest_threat(npc, Parent::Npc(lead), around, radius));
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

/// `npc`'s lead among `around`, while it is there and intact:
/// `_EscortAI`'s lost-parent test (@0x83917-0x8397b), its leader alone, as
/// a fighter's leader is its carrier. With the succession run first it
/// meets only a ship with no leader: one the disabled reset left
/// (`traffic::disabled_reset`), whole again (see the module docs).
fn parent<'a>(npc: &Npc, around: &Surroundings<'a>) -> Option<&'a Npc> {
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
            Goal::Wait => escort::hold_trigger(npc),
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
    /// above 4, @0x80e59). In the regroup pause it fires the substate it
    /// kept ([`regroup`](crate::ai::regroup)): a fight's passes at its
    /// target without the high level's launch
    /// ([`fire::low_level_trigger`]), else unprovoked (subs 0, 1, 9, 0xb
    /// and 0xc).
    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        match npc.goal {
            Goal::Regroup {
                kept: Kept::Attack(_) | Kept::Snipe(_),
            } => {
                let flying = Npc {
                    goal: npc.flown_goal(),
                    ..npc.clone()
                };
                GateWait {
                    trigger: fire::low_level_trigger(&flying, around, self.fire),
                    drops_target: false,
                }
            }
            _ => escort::fire_unprovoked(npc, around, self.fire),
        }
    }

    /// [`react::victim`]: `_DamageShip`'s reaction block reads no AI type,
    /// and `_DoGoodSamaritan` takes AI 3 and 4 alone (@0x8289f-0x828bb).
    /// Its next think keeps the hit's attacker by its order as any target
    /// ([`escort::kept_by_order`]), or sets another.
    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::victim(npc, strike, around)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{PIRATES, TRADERS, govts, n, player, ship};
    use crate::ai::{Help, escort, fire, react};
    use crate::bay::Carrier;
    use crate::catalog::{EscortClass, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::fleet::OrderWord;
    use crate::traffic::npc::{AiType, Mode, NpcId};
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
    fn with_its_lead_gone_or_not_intact_it_idles_and_leaves_its_fleet_drawing_nothing() {
        // The lost-parent branch (@0x83917-0x8397b): parent -1, goal 0 and
        // its class's `InherentAI` (@0x83c4a), its target kept.
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
                assert!(decision.leaves_fleet, "{:?} {order:?}", first.condition);
                assert!(!decision.ends_wait);
                assert!(!decision.clears_target);
                assert!(asked.is_empty());
            }
        }
        // A fighter the disabled reset left with no leader, whole again:
        // its carrier is read through its leader alone.
        let mut orphan = fighter(n(1));
        orphan.leader = None;
        let npcs = [lead(Goal::Idle), orphan, threat.clone()];
        let (decision, _) = decided_with(&npcs, Some(EscortOrder::Dock), (0.0, 300.0), &[0]);
        assert_eq!((decision.goal, decision.leaves_fleet), (Goal::Idle, true));
        // With its lead there and intact it stays.
        let npcs = [lead(Goal::Idle), fighter(n(1)), threat];
        let (decision, _) = decided_with(&npcs, Some(EscortOrder::Dock), (0.0, 300.0), &[0]);
        assert_eq!(
            (decision.goal, decision.leaves_fleet),
            (Goal::Dock(n(1)), false)
        );
    }

    /// What the escort, targeting NPC 3 under Attack, decides under a lead
    /// in goal 2 at the centre that `setup` set.
    fn entering(setup: impl Fn(&mut Npc)) -> (Decision, Vec<u32>) {
        let mut first = lead(Goal::JumpOut);
        setup(&mut first);
        let npcs = [
            first,
            targeting(escort(), n(3)),
            raider(3, 0.0, 100.0, n(1)),
        ];
        decided_with(&npcs, Some(EscortOrder::Attack), (0.0, 5000.0), &[0])
    }

    #[test]
    fn it_follows_a_lead_on_a_jump_braking_for_one_or_under_way() {
        // @0x839fd-0x83a92: sub 4 or 0xd, goal 2 sub 1, or +0x50 above 0.
        use crate::traffic::warp_out::WarpGoal;
        let far_off =
            |lead: &mut Npc| lead.state.position = crate::geometry::Vec2::new(3000.0, 0.0);
        for (name, setup) in [
            (
                "sub 4",
                &(|lead: &mut Npc| lead.begin_own_jump()) as &dyn Fn(&mut Npc),
            ),
            ("sub 0xd", &|lead: &mut Npc| {
                lead.goal = Goal::Formation { guard: None };
                lead.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
            }),
            ("goal 2 sub 1", &|lead: &mut Npc| {
                far_off(lead);
                lead.state.velocity = crate::geometry::Vec2::new(2.0, 0.0);
            }),
            ("+0x50 above 0", &|lead: &mut Npc| {
                lead.goal = Goal::Idle;
                lead.warp.stage = 0.5;
            }),
        ] {
            let (decision, asked) = entering(setup);
            assert_eq!(decision.goal, follow(None), "{name}");
            assert!(decision.clears_target, "target -1 (@0x83a71): {name}");
            assert!(decision.follows_jump, "{name}");
            assert!(!decision.ends_wait, "no order block runs: {name}");
            assert_eq!(decision.order_word, None, "{name}");
            assert!(asked.is_empty(), "{name}");
        }
    }

    #[test]
    fn it_keeps_formation_with_a_lead_running_out_inside_1000_or_at_rest() {
        // Goal 2 sub 3 draws no one: its escort fights on until it brakes.
        for setup in [
            &(|_: &mut Npc| {}) as &dyn Fn(&mut Npc),
            &|lead: &mut Npc| {
                lead.goal = Goal::Idle;
                lead.warp.stage = -1.0;
            },
        ] {
            let (decision, _) = entering(setup);
            assert_eq!(decision.goal, Goal::Attack(n(3)));
            assert!(!decision.follows_jump);
        }
    }

    #[test]
    fn a_fighter_of_the_players_escort_on_its_jump_follows_and_still_writes_its_order() {
        // The order word (@0x83b36-0x83b7f) runs after the entry.
        let mut first = lead(Goal::Formation { guard: None });
        first.warp.goal = Some(crate::traffic::warp_out::WarpGoal::Follow(ShipRef::Player));
        first.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: Some(EscortOrder::Attack),
        });
        let npcs = [first, fighter(n(1)), raider(3, 0.0, 100.0, n(1))];
        let govts = govts(0, 0);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let decision = FleetEscortAi::default().decision(&npcs[1], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, follow(None));
        assert!(decision.follows_jump);
        assert_eq!(decision.order_word, Some(OrderWord(3)), "Return: in form");
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
    fn under_an_npc_lead_it_cues_no_voice_even_taking_a_threat() {
        // `_EscortAI`'s roll and 1-in-3 call are for a ship the player
        // leads (+0x9a == 0, @0x83980, @0x83df9).
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, 300.0, n(1))];
        for order in ORDERS {
            let (decision, _) = decided_with(&npcs, order, (0.0, 5000.0), &[]);
            assert_eq!(decision.voice, None, "{order:?}");
        }
        let (decision, _) = decided_with(&npcs, Some(EscortOrder::Defend), (0.0, 5000.0), &[]);
        assert_eq!(decision.goal, Goal::Attack(n(3)), "it took the threat");
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

    /// `npc` short of ammunition as `left` says, its hull of `Flags2`
    /// 0x0080.
    fn short(npc: Npc, left: AmmoLeft) -> Npc {
        crate::ai::fixture::short_of_ammo(npc, left, true)
    }

    #[test]
    fn a_carriers_fighter_on_attack_out_of_ammunition_docks_and_writes_word_3() {
        // `_EscortAI`'s fallback, for every parent: AI type 5 takes word 3
        // (@0x83bc9), and the Return block docks it, +0x4c kept
        // (@0x83e79-0x83ea3).
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            for left in [AmmoLeft::OnlyUnlimited, AmmoLeft::Dry] {
                let dry = short(targeting(fighter(n(1)), n(3)), left);
                let npcs = [lead(Goal::Attack(n(3))), dry, raider(3, 0.0, 100.0, n(1))];
                let (decision, _) = decided_with(&npcs, Some(order), (0.0, 5000.0), &[0]);
                assert_eq!(decision.goal, Goal::Dock(n(1)), "{order:?} {left:?}");
                assert_eq!(
                    decision.order_word,
                    Some(OrderWord(3)),
                    "{order:?} {left:?}"
                );
                assert!(!decision.ends_wait, "{order:?} {left:?}");
            }
        }
    }

    #[test]
    fn an_escort_with_no_firepower_follows_and_writes_word_0() {
        // Any other type takes word 0 fully dry (@0x83bda), and the
        // formation block ends its wait (@0x83ea8-0x83eb0); with unlimited
        // weapons left it keeps its order and writes nothing.
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            let dry = short(targeting(escort(), n(3)), AmmoLeft::Dry);
            let npcs = [lead(Goal::Attack(n(3))), dry, raider(3, 0.0, 100.0, n(1))];
            let (decision, _) = decided_with(&npcs, Some(order), (0.0, 5000.0), &[0]);
            assert!(
                matches!(decision.goal, Goal::Follow { lead: LEAD, .. }),
                "{order:?}: {:?}",
                decision.goal
            );
            assert_eq!(decision.order_word, Some(OrderWord(0)), "{order:?}");
            assert!(decision.ends_wait, "{order:?}");
            let unlimited = short(targeting(escort(), n(3)), AmmoLeft::OnlyUnlimited);
            let npcs = [
                lead(Goal::Attack(n(3))),
                unlimited,
                raider(3, 0.0, 100.0, n(1)),
            ];
            let (decision, _) = decided_with(&npcs, Some(order), (0.0, 5000.0), &[0]);
            assert_eq!(decision.goal, Goal::Attack(n(3)), "{order:?}");
            assert_eq!(decision.order_word, None, "{order:?}");
        }
    }

    #[test]
    fn on_its_leaders_jump_the_fallback_still_writes_its_word() {
        // The fallback (@0x83b88-0x83bda) runs before goal 0xb's return
        // (@0x83be3).
        let mut first = lead(Goal::JumpOut);
        first.begin_own_jump();
        for (npc, word) in [(fighter(n(1)), 3), (escort(), 0)] {
            let npcs = [
                first.clone(),
                short(targeting(npc, n(3)), AmmoLeft::Dry),
                raider(3, 0.0, 100.0, n(1)),
            ];
            let (decision, _) = decided_with(&npcs, Some(EscortOrder::Attack), (0.0, 5000.0), &[0]);
            assert!(decision.follows_jump, "{word}");
            assert_eq!(decision.goal, follow(None), "{word}");
            assert_eq!(decision.order_word, Some(OrderWord(word)));
            assert!(!decision.ends_wait, "{word}");
        }
    }

    /// The lead as the player's escort following `order`, attacking NPC 3.
    fn players_escort(order: EscortOrder) -> Npc {
        let mut first = lead(Goal::Attack(n(3)));
        first.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: Some(order),
        });
        first
    }

    #[test]
    fn a_fighter_of_the_players_escort_writes_the_fallback_over_its_leads_order() {
        // @0x83b36-0x83b7f writes the lead's Attack, then the fallback
        // overwrites it with 3 (@0x83bc9).
        let npcs = [
            players_escort(EscortOrder::Attack),
            short(fighter(n(1)), AmmoLeft::Dry),
            raider(3, 0.0, 500.0, n(1)),
        ];
        let govts = govts(0, 0);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let decision = FleetEscortAi::default().decision(&npcs[1], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Dock(n(1)));
        assert_eq!(decision.order_word, Some(OrderWord(3)));
    }

    #[test]
    fn a_fighter_of_a_players_escort_out_of_ammunition_returns_home() {
        // The fighter reads its lead's word (@0x83b56), which the lead's
        // fallback wrote: 0 for an escort fully dry, so Return (@0x83b7f).
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            let order_with = |left: AmmoLeft| {
                let npcs = [
                    short(players_escort(order), left),
                    fighter(n(1)),
                    raider(3, 0.0, 500.0, n(1)),
                ];
                order_of(&npcs[1], &Surroundings::new(&[], &npcs))
            };
            assert_eq!(
                order_with(AmmoLeft::Dry),
                Some(EscortOrder::Dock),
                "{order:?}"
            );
            assert_eq!(
                order_with(AmmoLeft::OnlyUnlimited),
                Some(order),
                "{order:?}: unlimited weapons left"
            );
            assert_eq!(order_with(AmmoLeft::Armed), Some(order), "{order:?}");
        }
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

    /// What `npcs[1]` holds in the regroup pause, keeping `kept`, its
    /// target `target`, the player at (0, 5000).
    fn paused(npcs: &[Npc], kept: Kept, target: Option<ShipRef>) -> GateWait {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut npc = npcs[1].clone();
        npc.goal = Goal::Regroup { kept };
        npc.target = target;
        FleetEscortAi::default().gate_wait(&npc, &around)
    }

    #[test]
    fn paused_in_a_kept_fight_it_fires_its_passes_at_its_target_launching_nothing() {
        // The pause skips `_HighLevelAIHandler` and its `_AILaunchFighter`
        // (@0x8e6fa); `_LowLevelAIHandler` runs the fight's substate.
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, -50.0, n(1))];
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        for (kept, goal) in [
            (Kept::Attack(n(3)), Goal::Attack(n(3))),
            (Kept::Snipe(n(3)), Goal::Snipe(n(3))),
        ] {
            let mut fighting = npcs[1].clone();
            fighting.goal = goal;
            fighting.target = Some(n(3));
            let full = fire::trigger(&fighting, &around, FireRules::default());
            let wait = paused(&npcs, kept, Some(n(3)));
            assert_eq!(
                wait,
                GateWait {
                    trigger: Trigger {
                        bays: false,
                        ..full
                    },
                    drops_target: false,
                },
                "{kept:?}"
            );
            assert!(wait.trigger.only.is_some(), "{kept:?}: its passes fire");
        }
        assert!(
            fire::trigger(
                &Npc {
                    goal: Goal::Attack(n(3)),
                    ..npcs[1].clone()
                },
                &around,
                FireRules::default()
            )
            .bays,
            "the high level's launch, which the pause skips"
        );
    }

    #[test]
    fn paused_in_any_other_substate_it_fires_unprovoked_at_its_target() {
        let npcs = [lead(Goal::Idle), escort(), raider(3, 0.0, -50.0, n(1))];
        for kept in [
            Kept::Formation { guard: Some(n(3)) },
            Kept::Coast,
            Kept::Stop,
        ] {
            assert_eq!(
                paused(&npcs, kept, Some(n(3))),
                GateWait {
                    trigger: fired(140),
                    drops_target: false
                },
                "{kept:?}"
            );
        }
        let mut down = raider(3, 0.0, -50.0, n(1));
        down.condition = Condition::Disabled;
        let npcs = [lead(Goal::Idle), escort(), down];
        assert_eq!(
            paused(&npcs, Kept::Formation { guard: Some(n(3)) }, Some(n(3))),
            GateWait {
                trigger: Trigger::default(),
                drops_target: true
            },
            "a disabled guard is dropped"
        );
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

    #[test]
    fn holding_at_its_gate_it_runs_its_point_defence_through_the_wait() {
        // Hold tail-calls `_AIFireTurret` with no target (@0x83e5f,
        // @0x83f4e), point defence unless disabled (@0x80afa-0x80b0d),
        // beside the wait's `_AIEscortFireUnprovoked`, which does nothing
        // with no target (@0x8521e-0x85236). A ship waiting in its gate
        // (goal 21) runs no think, whatever its goal from before.
        let ai = FleetEscortAi::default();
        let in_gate = |mode: Mode, goal: Goal| {
            let mut npc = escort();
            npc.mode = mode;
            npc.goal = goal;
            let npcs = [lead(Goal::Idle), npc, raider(3, 0.0, -50.0, n(1))];
            ai.gate_wait(&npcs[1], &Surroundings::new(&[], &npcs))
        };
        let held = Mode::HeldAtGate { ticks_left: 20 };
        assert!(
            in_gate(held, Goal::Wait).trigger.point_defence,
            "held, holding"
        );
        assert_eq!(
            in_gate(Mode::Emerging { ticks_left: 20 }, Goal::Wait),
            GateWait::default(),
            "waiting in its gate"
        );
        assert_eq!(
            in_gate(held, Goal::Attack(ShipRef::Player)),
            GateWait::default(),
            "held, attacking with no target"
        );
    }
}
