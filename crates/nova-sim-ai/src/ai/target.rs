//! Whom a warship or interceptor attacks: the player, as its government's
//! relations and the player's legal record say, or an NPC.
//!
//! The rules are the original's (`_SelectWarshipTarget` @0x89d5e,
//! `_WarshipAI` @0x8b7ab, `_InterceptorAI` @0x8cb57 and
//! `_HighLevelAIHandler`'s drops @0x8e246-0x8e3f6 in the `EV Nova`
//! executable). G is the ship's government, S the system's, and L the
//! player's legal record in the system (its record with S; none in an
//! independent system).
//!
//! **Hostility to the player** ([`hostile_to_player`], the legal table
//! @0x8a61a-0x8a90d), the player being in the system and not breaking up
//! (@0x8a6f8):
//!
//! - A G that blesses the player (rank `Flags` 0x0100,
//!   [`Surroundings::blessed`]): never (@0x8a6a8-0x8a701).
//! - `Flags` 0x0040: never (@0x8a720-0x8a72d).
//! - Otherwise only while the player is within [`HUNT_REACH`] times the
//!   ship's aggression on each axis (@0x8a68d-0x8a6c8), by the legal table,
//!   with G's own `CrimeTol` (T):
//!
//!   | S | attacks when |
//!   |---|---|
//!   | G | L < -T |
//!   | an enemy of G | L > T |
//!   | allied with G | L < -1.5 T ([`ALLIED_TOLERANCE`], @0xdd640) |
//!   | neutral, or none | only with `Flags` 0x0002: L < -2 T |
//!
//! Neither `Flags` 0x0004 nor 0x0001 is read there: 0x0004 acts only
//! through the government's word (`_WarshipAI` @0x8b7b0, `_InterceptorAI`
//! @0x8cb5c; see [`warship`](crate::ai::warship)), and a xenophobe never
//! reaches the table (step 5 below). The blessing is the deciding ship's
//! own government's, as `nova_sim::Session::blessing` takes it under
//! either reading of `rank_govt_reach` and `blessing_refresh`; another
//! government's blessing changes nothing here.
//!
//! **The target** ([`select_target`], and [`pick_target`] with whether
//! the pick turns the ship on the player): these steps in order. A ship
//! still jumping in is never one, nor one destroyed; the player is always
//! seen here (its cloak, and NPCs', are cloaking-fidelity's).
//!
//! 1. **Keep** the live ship it attacks or flees from.
//! 2. **A grudge** (`_SelectWarshipTarget` @0x89ebc-0x89f20): a person
//!    whose `Flags` has 0x0001 and who holds a grudge against the player
//!    (see `nova_sim::person`) picks the player, while it is in
//!    the system and not breaking up, unless it spares it. It reads no
//!    blessing: a blessing government's person with a grudge still takes
//!    the player. It turns on the player through `_AIMakeShipAttackPlayer`
//!    (the tail call @0x89f20), so its person may say its attack quote
//!    ([`Pick::turns_on_player`]).
//! 3. **A mission ship of `ShipBehav` 0** (@0x89f25-0x89f84, see
//!    [`mission_hunt`]): while the player is in the system, the player,
//!    or none when it is disabled or breaking up and the ship has no
//!    destroying weapons (as `_WarshipAI` drops such a target in the same think,
//!    @0x8bc8b-0x8bca9); no later step runs, so such a hunter never turns
//!    on another ship while the player is here, and a paid-off one takes
//!    the player again. It reads no blessing either. It reaches the same
//!    `_AIMakeShipAttackPlayer` call (@0x89f20), whose quote gate a
//!    mission ship always fails (+0xc8d2, @0x89c71). The `ShipBehav` 1
//!    branch (@0x89f86-0x89fef) never runs (see the session's
//!    `mission_escorts` module).
//! 4. **Out of ammunition** (@0x89ff4-0x8a006): a ship with no weapon,
//!    or every weapon out of rounds or fuel
//!    ([`AmmoLeft::Dry`](crate::ai::fire::AmmoLeft::Dry)), takes no new
//!    target. **A ship of no government** takes none either: the original
//!    jumps past every later step (@0x8a052 → @0x8ae6a) to the AI-type-6
//!    search, which no warship or interceptor runs (@0x8ae70).
//! 5. **A xenophobe** (`Flags` 0x0001, @0x8a06f-0x8a413) runs the
//!    candidate pass ([`candidates`](crate::ai::candidates), the strength filter and the
//!    nearest, no heaviest) over the player and its NPC enemies. None of
//!    them is breaking up, disabled, or in the escape pod (class 0x2ff,
//!    [`ESCAPE_POD_SHIP`]; @0x8a094-0x8a0f1). The player is one unless
//!    `Flags` 0x0040 (@0x8a123), S is G and L is above none
//!    (@0x8a189-0x8a19a), or G blesses it (@0x8a1ae); `Flags` 0x0004 isn't
//!    read. An NPC is one when it is not one the ship leads (@0x8a204), not
//!    allied with it (@0x8a250), and an enemy (`_IsShipMyEnemy`
//!    @0x8a270, [`Governments::npc_enemies`](crate::govt::Governments::npc_enemies)).
//!    A pick of the player turns the ship on it (@0x8a40e). The step
//!    **doesn't end the search**: it goes on to the IFF and blessing
//!    clears and step 7 (@0x8a9c9-0x8a9f4), skipping step 6, the legal
//!    table and the roll, its player candidate carried over. Step 7's pick
//!    wins; step 5's is kept only when step 7 has none, which happens only
//!    when an IFF scrambler cleared a player step 5 picked (@0x8aa13): an
//!    IFF-marked xenophobe with no NPC enemy in view still takes the
//!    player.
//! 6. **Help allies** (@0x8a418-0x8a60b): the ship T that the first other
//!    ship of a government allied with its own (its own included) attacks
//!    or flees from (goals 3 and 4, @0x8a494-0x8a4a2: never one
//!    plundering), when its own friend strength times its government's
//!    `MaxOdds` is no less than T's (@0x8a5ab). T may be the player, but
//!    never a ship it spares (below), nor the ship itself; it is any ship
//!    seen (@0x8a538), so a downed or breaking-up T is taken and then
//!    dropped at once (below). This is how police come to a trader's
//!    defence. While G blesses the player, an ally whose T is the player
//!    or a ship the player leads is passed over, and the search goes on to
//!    the next ally (@0x8a4e4-0x8a50e, then @0x8a5de). It sets the target
//!    and goal 4 itself (@0x8a5b0-0x8a5c1) and makes no
//!    `_AIMakeShipAttackPlayer` call, even when T is the player, so no
//!    attack quote comes of it.
//! 7. **The hunt** (@0x8a90d-0x8ad36): the player is a candidate when it
//!    is hostile ([`hostile_to_player`]), or when **the enemy-class
//!    roll** makes it one (@0x8a90d-0x8a9c7): for a ship not led by the
//!    player, with no mission mark nor aux mark (+0xc8d2, +0x8a), while
//!    the government the player's ship type fights as
//!    ([`PlayerSide::combat_govt`](crate::ai::PlayerSide::combat_govt)) is
//!    at war with G, a draw below 50 (`Rand(50)` @0x8a993) of 0 with the
//!    player not breaking up. A xenophobe's candidate is step 5's instead.
//!    Then a G that an IFF scrambler marked ([`Surroundings::iff_marked`],
//!    @0x8aa13), or one that blesses the player (@0x8aa35), clears it.
//!    Every NPC enemy that is seen is a candidate (@0x8aa5e-0x8aade), one
//!    downed or breaking up only when the ship has destroying weapons
//!    ([`fire::destroys`]: `_IsDisabled` holds of a breaking-up ship, and
//!    there is no dying test), but never one it spares (below; the
//!    original has no such test, see the note). The candidate pass
//!    ([`candidates`](crate::ai::candidates)) then takes the nearest, of the heaviest class when
//!    the ship has escorts (@0x8ac24-0x8ad07). A pick of the player turns
//!    the ship on it (@0x8ad31).
//! 8. **Threats** (`_ExtendedIsThreatToShip` asked @0x8adb6), only when
//!    steps 5 and 7 gave none (@0x8ad3c): the nearest NPC attacking,
//!    sniping at or fleeing from it or a ship it leads
//!    ([`Npc::threatens_npc_among`]), other than one it spares. The
//!    player is never one here: the original asks of it while the ship has
//!    no target, so `_IsThreatToPlayer` of the ship is false, and the
//!    blessing `_ExtendedIsThreatToShip` reads there (@0x81a89) cannot
//!    change the answer. A blessing government's ship still turns on an
//!    NPC threat. The original's second threat search, for a ship of AI
//!    type 6 (@0x8ae70, @0x8af1d), never runs: only AI types 3 and 4 reach
//!    this function (`_AIDispatch` @0x8ffc2-0x9000a).
//!
//! **A target is dropped** ([`dropped`], @0x8bc73, @0x8e3ef) when it is
//! gone or breaking up; disabled while the ship has no destroying
//! weapons; or a ship it spares. The hunt drops a fresh pick so too, in
//! the same think (see [`warship::hunt`](crate::ai::warship::hunt)).
//!
//! **A ship it spares** ([`spares`]): an NPC of its own fleet, or an ally
//! as [`TargetRules::led_ally`] reads
//! [`RuleKey::LedAllyAttack`]. By
//! the engine, `_HighLevelAIHandler`'s state-4 drop (@0x8e277-0x8e2e4):
//! an NPC allied with it when it has a government and neither it nor
//! that NPC is led by the player, so an allied ship the player leads is
//! attacked on, and a ship of no government spares no ally. By the other
//! reading, every NPC allied with it, an independent by an independent
//! too.
//!
//! **A ship the player paid off** ([`Npc::spared`], see
//! [`hail`](crate::hail)) spares the player too: no step gives the
//! player (a `ShipBehav` 0 mission ship aside, step 3), it keeps no
//! player target, and drops one. The original makes
//! such a ship a wimpy trader that leaves, which never targets the
//! player unprovoked; this holds for any behaviour that asks here.
//!
//! Steps 6, 7 and 8 never give a ship it spares. The original checks
//! none of them there, and `_ExtendedIsThreatToShip` @0x81a27 excludes
//! only a ship the asker leads (@0x81a5a), not an ally, so its police,
//! seeing an allied trader that a stray police shot provoked, turn on
//! their own kind until the drop at @0x8e2af clears the goal a frame
//! later, and then take it up again. Here they never do. The engine's
//! own-fleet sparing is also wider than the original's fleet drops
//! (@0x8e2e9-0x8e399), which need the attacker to have a leader.
//!
//! **Left out.** `_ShipVisibleToShip` for an NPC's cloak in steps 5-8
//! (@0x8a0d2, @0x8a140, @0x8a538, @0x8a6df, @0x8a748, @0x8a9ba, @0x8aa7b,
//! @0x8ad92) is rdm:phase/cloaking-fidelity/phase-4-npc-cloak-visibility's:
//! here an NPC is seen once it has arrived. Stellar defenders (+0x8c,
//! @0x8a21e) are rdm:task/planet-domination's, and the nag ship
//! (+0xc8d0 = 0x3ff, @0x8a228) has no engine counterpart. The same-system
//! test (@0x8a214) and the +0x88 ≤ 4 gate of step 6 (@0x8a41e) always
//! hold here. The original's 16-bit strengths and 32-bit distances are
//! not replicated.

use crate::ai::candidates::{has_escorts, nearest, within_odds};
use crate::ai::fire::AmmoLeft;
use crate::ai::odds::friend_strength;
use crate::ai::{Surroundings, fire};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::{Condition, ESCAPE_POD_SHIP};
use crate::govt::{NEVER_ATTACKS_PLAYER, NOSY};
#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;
use crate::traffic::GRUDGE;
use crate::traffic::ShipBehav;
use crate::traffic::npc::Npc;

/// How far, in pixels on either axis per point of aggression, a warship
/// hunts a player its government wants.
pub const HUNT_REACH: f32 = 600.0;
/// How many times its tolerance a government's allies' warships put up
/// with.
pub const ALLIED_TOLERANCE: f32 = 1.5;
/// How many times its tolerance a nosy government's warships put up with
/// elsewhere.
pub const NOSY_TOLERANCE: f32 = 2.0;
/// The sides of the enemy-class roll: 1 in 50 turns a ship on a player
/// flying its enemy's ship type (`Rand(50)` @0x8a98c-0x8a99b).
pub const ENEMY_CLASS_SIDES: u32 = 50;

/// The rulebook's entries target selection follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TargetRules {
    /// Which allies a ship spares ([`RuleKey::LedAllyAttack`], see
    /// [`spares`]).
    pub led_ally: RuleSource,
    /// How a friend of no shield capacity counts in a friend strength
    /// ([`RuleKey::FriendShieldCarry`], see
    /// [`odds`](crate::ai::odds)).
    pub friend_carry: RuleSource,
}

/// Whether `npc`, a warship or interceptor not xenophobic, finds the
/// player hostile among `around` (see the module docs).
#[must_use]
pub fn hostile_to_player(npc: &Npc, around: &Surroundings) -> bool {
    let Some(player) = around.player.filter(|_| around.live(ShipRef::Player)) else {
        return false;
    };
    let own = npc.govt;
    let govts = around.govts;
    if around.blessed(own) || govts.flag(own, NEVER_ATTACKS_PLAYER) {
        return false;
    }
    let reach = HUNT_REACH * f32::from(npc.aggression);
    let off = player.state.position - npc.state.position;
    if off.x.abs() > reach || off.y.abs() > reach {
        return false;
    }
    let system = around.system_govt;
    let record = f32::from(around.record);
    let tolerance = f32::from(govts.crime_tol(own));
    if system == own {
        record < -tolerance
    } else if system.is_some() && govts.enemies(own, system) {
        record > tolerance
    } else if system.is_some() && govts.allies(own, system) {
        record < -ALLIED_TOLERANCE * tolerance
    } else {
        govts.flag(own, NOSY) && record < -NOSY_TOLERANCE * tolerance
    }
}

/// Whether a xenophobic `npc` takes the player as a candidate among
/// `around` (step 5, @0x8a094-0x8a1bf): the player intact, not in the
/// escape pod, and none of `Flags` 0x0040, the player in good standing in
/// the ship's own system, or the ship's government blessing it.
fn xenophobe_hunts_player(npc: &Npc, around: &Surroundings) -> bool {
    let Some(player) = around.player else {
        return false;
    };
    player.condition == Condition::Intact
        && player.ship != ESCAPE_POD_SHIP
        && !around.govts.flag(npc.govt, NEVER_ATTACKS_PLAYER)
        && !(around.system_govt == npc.govt && around.record > 0)
        && !around.blessed(npc.govt)
}

/// Whether the enemy-class roll makes the player `npc`'s candidate among
/// `around`, drawing on `chance` only when its gates hold (see the module
/// docs' step 7, @0x8a90d-0x8a9c7).
fn enemy_class_roll(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> bool {
    let Some(player) = around.player else {
        return false;
    };
    let gated = !npc.escorts_player()
        && npc.govt.is_some()
        && npc.mission.is_none()
        && npc.aux.is_none()
        && player.combat_govt.is_some()
        && around.govts.enemies(npc.govt, player.combat_govt);
    gated && chance.below(ENEMY_CLASS_SIDES) == 0 && around.live(ShipRef::Player)
}

/// Whether `ship` is in the system to be seen as a candidate: there,
/// not destroyed and not still jumping in (`_ShipVisibleToShip`, the
/// active slot +0xb8). The player always is while it is there.
fn present(ship: ShipRef, around: &Surroundings) -> bool {
    match ship {
        ShipRef::Player => around.player.is_some(),
        ShipRef::Npc(id) => around
            .npc(id)
            .is_some_and(|npc| npc.arrived() && npc.condition.active()),
    }
}

/// Whether `npc`, a mission's special ship of `ShipBehav` 0, attacks the
/// player among `around`: none when it is no such ship or the player is
/// not in the system; else whether it attacks, which it does unless the
/// player is disabled or breaking up and it has no destroying weapons
/// ([`fire::destroys`]).
///
/// The handler gives up (state 2, target -1, @0x8d6be-0x8d6e9) when
/// `_IsDisabled(player)` holds and `_AIHasDestroyingWeapons` @0x8d693
/// does not. `_IsDisabled` @0x2ce6 compares the player's armour with a
/// share of its capacity (@0x2d79-0x2dcc), so it holds of a breaking-up
/// player too, whose armour is gone. With destroying weapons it goes on
/// attacking a breaking-up player: `_ShipVisibleToShip` @0x971c has no
/// dying test, and `_AIMakeShipAttackPlayer` @0x89c3e reads `_IsDying`
/// only for a person's attack quote (@0x89ca7).
///
/// The one rule behind both `_SelectWarshipTarget` @0x89f25-0x89f84 and
/// `_HighLevelAIHandler` @0x8d628-0x8d6ea, which the AI's dispatch reads
/// on every think and [`select_target`] reads as its step 3. A ship the
/// player paid off is no exception: neither site reads the bribe, whose
/// leave (`_AIMakeShipLeave`, state 2, @0x961eb) is no state the handler
/// skips (8, 11 and 21), so the ship takes the player again on its next
/// think. That is the engine's default; [`RuleKey::HunterBribe`] can
/// honour the bribe instead, and only `NovaAi`'s dispatch reads it: a
/// paid-off ship is a wimpy trader, which never asks [`select_target`],
/// so this rule needs no key. A cloaked or gate-emerging player is no
/// matter: the engine has neither, so the player is always visible to it.
#[must_use]
pub fn mission_hunt(npc: &Npc, around: &Surroundings) -> Option<bool> {
    npc.mission
        .filter(|mark| mark.behav == ShipBehav::AttackPlayer)?;
    let player = around.player.as_ref()?;
    Some(
        !matches!(
            player.condition,
            Condition::Disabled | Condition::Dying { .. }
        ) || fire::destroys(npc),
    )
}

/// The ship `npc` attacks among `around` by `rules`, keeping `current`
/// while it is live and drawing the enemy-class roll on `chance` (see the
/// module docs): [`pick_target`]'s ship.
#[must_use]
pub fn select_target(
    npc: &Npc,
    current: Option<ShipRef>,
    around: &Surroundings,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Option<ShipRef> {
    pick_target(npc, current, around, rules, chance).map(|pick| pick.ship)
}

/// The ship a warship or interceptor picks to attack ([`pick_target`]),
/// and whether the pick turns it on the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pick {
    /// The ship it attacks.
    pub ship: ShipRef,
    /// Whether `_SelectWarshipTarget` calls `_AIMakeShipAttackPlayer` for
    /// this pick: the player picked by a grudge (step 2, the tail call
    /// @0x89f20), a `ShipBehav` 0 mission ship (step 3, the same call),
    /// a xenophobe (step 5, @0x8a40e) or the hunt (step 7, @0x8ad31). A
    /// xenophobe whose step-5 pick of the player step 7 overrides has made
    /// that call all the same, so its pick of an NPC turns it on the
    /// player too: the quote's gate reads the tick's starting state, so
    /// the original's two calls are one here. The player kept (step 1) or
    /// helped against (step 6, which sets the target and goal 4 itself,
    /// @0x8a5b0-0x8a5ca) is no such call.
    pub turns_on_player: bool,
}

/// [`select_target`]'s pick, with whether it turns `npc` on the player
/// (see [`Pick`] and the module docs).
#[must_use]
pub fn pick_target(
    npc: &Npc,
    current: Option<ShipRef>,
    around: &Surroundings,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Option<Pick> {
    let kept = |ship| Pick {
        ship,
        turns_on_player: false,
    };
    // A pick that calls `_AIMakeShipAttackPlayer` when it is the player.
    let called = |ship| Pick {
        ship,
        turns_on_player: ship == ShipRef::Player,
    };
    let paid_off = |ship| ship == ShipRef::Player && npc.spared;
    if let Some(ship) = current.filter(|&ship| around.live(ship) && !paid_off(ship)) {
        return Some(kept(ship));
    }
    let grudging = npc
        .person
        .is_some_and(|person| person.grudge && person.flags & GRUDGE != 0);
    if grudging && !npc.spared && around.live(ShipRef::Player) {
        return Some(called(ShipRef::Player));
    }
    if let Some(attacks) = mission_hunt(npc, around) {
        return attacks.then_some(called(ShipRef::Player));
    }
    if fire::ammo_left(npc) == AmmoLeft::Dry || npc.govt.is_none() {
        return None;
    }
    let govts = around.govts;
    let carry = rules.friend_carry;
    let others = || {
        around
            .by_slot()
            .into_iter()
            .filter(move |other| other.id != npc.id && present(ShipRef::Npc(other.id), around))
    };
    let player = ShipRef::Player;
    let (mut player_candidate, xenophobe_pick) = if govts.xenophobic(npc.govt) {
        let enemies = others()
            .filter(|other| {
                other.condition == Condition::Intact
                    && other.leader != Some(npc.id)
                    && other.ship != ESCAPE_POD_SHIP
                    && !govts.allies(npc.govt, other.govt)
                    && govts.npc_enemies(npc.govt, other.govt)
            })
            .map(|other| ShipRef::Npc(other.id));
        let hunts = xenophobe_hunts_player(npc, around) && !npc.spared;
        let candidates = within_odds(
            npc,
            hunts.then_some(player).into_iter().chain(enemies),
            around,
            carry,
        );
        (
            candidates.contains(&player),
            nearest(npc, &candidates, around, false).map(called),
        )
    } else {
        if let Some(foe) = helped(npc, around, rules) {
            return Some(kept(foe));
        }
        let rolled = enemy_class_roll(npc, around, chance);
        let wanted = hostile_to_player(npc, around) || rolled;
        (wanted && !npc.spared, None)
    };
    if around.iff_marked(npc.govt) || around.blessed(npc.govt) {
        player_candidate = false;
    }
    let destroys = fire::destroys(npc);
    let enemies = others()
        .filter(|other| {
            (other.condition == Condition::Intact || destroys)
                && govts.npc_enemies(npc.govt, other.govt)
                && !spares(npc, ShipRef::Npc(other.id), around, rules)
        })
        .map(|other| ShipRef::Npc(other.id));
    let candidates = within_odds(
        npc,
        player_candidate
            .then_some(player)
            .into_iter()
            .chain(enemies),
        around,
        carry,
    );
    if let Some(target) = nearest(npc, &candidates, around, has_escorts(npc, around)) {
        let called_before = xenophobe_pick.is_some_and(|pick| pick.turns_on_player);
        return Some(Pick {
            turns_on_player: called(target).turns_on_player || called_before,
            ..called(target)
        });
    }
    if xenophobe_pick.is_some() {
        return xenophobe_pick;
    }
    let threats = others()
        .filter(|other| other.threatens_npc_among(npc.id, around.npcs))
        .map(|other| ShipRef::Npc(other.id))
        .filter(|&ship| !spares(npc, ship, around, rules))
        .collect::<Vec<_>>();
    nearest(npc, &threats, around, false).map(kept)
}

/// Step 6, help allies (see the module docs): the quarry of the first
/// ally fighting whom `npc` is strong enough to take on, among `around`
/// by `rules`.
fn helped(npc: &Npc, around: &Surroundings, rules: TargetRules) -> Option<ShipRef> {
    let govts = around.govts;
    let me = ShipRef::Npc(npc.id);
    let carry = rules.friend_carry;
    let own = friend_strength(me, around, carry) * govts.max_odds(npc.govt);
    let blessed = around.blessed(npc.govt);
    around
        .by_slot()
        .into_iter()
        .filter(|ally| {
            ally.id != npc.id
                && around.live(ShipRef::Npc(ally.id))
                && ally.condition == Condition::Intact
                && ally.govt.is_some()
                && govts.allies(npc.govt, ally.govt)
        })
        .find_map(|ally| {
            // Goals 3 and 4 alone (@0x8a494-0x8a4a2): a plunder (0xd) is
            // no fight to help in.
            let foe = ally
                .goal
                .quarry()
                .filter(|_| ally.goal.plundering().is_none())?;
            if blessed && leads_or_is_player(foe, around) {
                return None;
            }
            (foe != me
                && present(foe, around)
                && !spares(npc, foe, around, rules)
                && own >= friend_strength(foe, around, carry))
            .then_some(foe)
        })
}

/// Whether `ship` is the player or a ship the player leads, among
/// `around` (the quarry's leader +0x9a is 0, @0x8a4e4).
fn leads_or_is_player(ship: ShipRef, around: &Surroundings) -> bool {
    match ship {
        ShipRef::Player => true,
        ShipRef::Npc(id) => around.npc(id).is_some_and(Npc::escorts_player),
    }
}

/// Whether `npc` spares `ship` among `around` by `rules`: the player once
/// it has paid to be spared, or an NPC of its own fleet or an ally it
/// spares (see the module docs).
#[must_use]
pub fn spares(npc: &Npc, ship: ShipRef, around: &Surroundings, rules: TargetRules) -> bool {
    match ship {
        ShipRef::Player => npc.spared,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| {
            other.fleet() == npc.fleet() || spares_ally(npc, other, around, rules.led_ally)
        }),
    }
}

/// Whether `npc` spares `other` as an ally by `led_ally`'s reading of
/// [`RuleKey::LedAllyAttack`] among `around`: by the engine, when it has a
/// government and neither it nor `other` is led by the player
/// (`_HighLevelAIHandler` @0x8e277-0x8e2e4, @0x8e077-0x8e0c3); by the other
/// reading, whenever they are allied.
#[must_use]
pub fn spares_ally(npc: &Npc, other: &Npc, around: &Surroundings, led_ally: RuleSource) -> bool {
    let allied = around.govts.allies(npc.govt, other.govt);
    match led_ally {
        RuleSource::Engine => {
            allied && npc.govt.is_some() && !npc.escorts_player() && !other.escorts_player()
        }
        RuleSource::Bible => allied,
    }
}

/// Whether `npc` drops `target` among `around` by `rules` (see the module
/// docs).
#[must_use]
pub fn dropped(npc: &Npc, target: ShipRef, around: &Surroundings, rules: TargetRules) -> bool {
    if !around.live(target) {
        return true;
    }
    if around.condition_of(target) == Some(Condition::Disabled) && !fire::destroys(npc) {
        return true;
    }
    spares(npc, target, around, rules)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::Blessing;
    use crate::govt::ALWAYS_ATTACKS_PLAYER;
    use nova_sim_base::fixture::Draws;

    /// [`pick_target`] by the engine's rules, the enemy-class roll never
    /// coming up 0.
    fn pick(npc: &Npc, current: Option<ShipRef>, around: &Surroundings) -> Option<Pick> {
        pick_target(
            npc,
            current,
            around,
            TargetRules::default(),
            &mut Draws::of(&[]),
        )
    }

    /// [`pick`]'s ship.
    fn select(npc: &Npc, current: Option<ShipRef>, around: &Surroundings) -> Option<ShipRef> {
        pick(npc, current, around).map(|pick| pick.ship)
    }

    /// [`dropped`] by the engine's rules.
    fn drops(npc: &Npc, target: ShipRef, around: &Surroundings) -> bool {
        dropped(npc, target, around, TargetRules::default())
    }
    use crate::ai::fixture::blessing_of;
    use crate::ai::{Goal, PlayerSide};
    use crate::catalog::{GovtId, GovtRecord, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::HullSpec;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::govt::{Governments, XENOPHOBIC};
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::traffic::npc::{Mode, NpcId};
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::{govt, weapon};

    const ME: GovtId = GovtId(140);
    const ALLY: GovtId = GovtId(141);
    const ENEMY: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);
    const RIVAL: GovtId = GovtId(145);

    /// The ship's government 140 (class 1, `CrimeTol` 6, `MaxOdds` 100,
    /// with `flags`), its ally 141, its enemies 142 and 145 (not allied
    /// with each other), a neutral 143 and a xenophobe 144 allied with
    /// it.
    fn govts(flags: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
                crime_tol: 6,
                classes: [1, -1, -1, -1],
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            GovtRecord {
                classes: [4, -1, -1, -1],
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                allies: [1, -1, -1, -1],
                ..govt(144)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [5, -1, -1, -1],
                ..govt(145)
            },
        ])
    }

    /// NPC `id` of `govt` at (`x`, `y`), of strength 100, aggression 2,
    /// armed with a harmless gun of unlimited ammunition (so it is not
    /// dry, and holds no destroying weapon).
    fn ship(id: u32, govt: GovtId, x: f32, y: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.armament = Armament::new([(WeaponSpec::new(&weapon(127)), 1)]);
        npc.govt = Some(govt);
        npc.state.position = Vec2::new(x, y);
        npc.hull.strength = 100.0;
        npc.aggression = 2;
        npc
    }

    /// The player at (`x`, `y`), of strength 100.
    fn player(x: f32, y: f32) -> PlayerSide {
        PlayerSide {
            state: ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            },
            condition: Condition::Intact,
            reserves: Reserves::full(100.0, 100.0, 100.0),
            hull: HullSpec {
                strength: 100.0,
                ..HullSpec::default()
            },
            handling: ShipStats::new(FAST, &[]).handling,
            rating: 0,
            yardstick: 0,
            untargetable: false,
            blessing: Blessing::NONE,
            ship: crate::catalog::ShipId(128),
            combat_govt: None,
            target: None,
            class: crate::catalog::EscortClass::Freighter,
            boarded: false,
        }
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    const P: ShipRef = ShipRef::Player;

    /// Whether ship 1 of govt 140 (of `flags`) at the centre is hostile
    /// to the player at (`x`, `y`) with record `record` in a system of
    /// `system`.
    fn hostile(flags: u16, system: Option<GovtId>, record: i16, at: (f32, f32)) -> bool {
        hostile_blessed(flags, system, record, at, Blessing::NONE)
    }

    /// [`hostile`], the player blessed by `blessing`.
    fn hostile_blessed(
        flags: u16,
        system: Option<GovtId>,
        record: i16,
        (x, y): (f32, f32),
        blessing: Blessing,
    ) -> bool {
        let govts = govts(flags);
        let npcs = [ship(1, ME, 0.0, 0.0)];
        let around = Surroundings {
            player: Some(PlayerSide {
                blessing,
                ..player(x, y)
            }),
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &npcs)
        };
        hostile_to_player(&npcs[0], &around)
    }

    const NEAR: (f32, f32) = (0.0, -100.0);

    #[test]
    fn at_home_a_warship_attacks_a_record_below_minus_its_tolerance() {
        assert!(!hostile(0, Some(ME), -6, NEAR));
        assert!(hostile(0, Some(ME), -7, NEAR));
        assert!(!hostile(0, Some(ME), 30, NEAR));
    }

    #[test]
    fn in_an_enemys_system_a_warship_attacks_a_record_above_its_tolerance() {
        assert!(!hostile(0, Some(ENEMY), 6, NEAR));
        assert!(hostile(0, Some(ENEMY), 7, NEAR));
        assert!(!hostile(0, Some(ENEMY), -30, NEAR));
    }

    #[test]
    fn in_an_allys_system_a_warship_attacks_below_one_and_a_half_times_its_tolerance() {
        assert!(!hostile(0, Some(ALLY), -9, NEAR));
        assert!(hostile(0, Some(ALLY), -10, NEAR));
        assert_eq!(ALLIED_TOLERANCE, 1.5);
    }

    #[test]
    fn elsewhere_only_a_nosy_warship_attacks_below_twice_its_tolerance() {
        for system in [Some(NEUTRAL), None] {
            assert!(!hostile(0, system, -1000, NEAR), "{system:?}");
            assert!(!hostile(NOSY, system, -12, NEAR), "{system:?}");
            assert!(hostile(NOSY, system, -13, NEAR), "{system:?}");
        }
        assert_eq!(NOSY_TOLERANCE, 2.0);
    }

    #[test]
    fn a_warship_hunts_a_wanted_player_only_within_600_per_point_of_aggression() {
        // Aggression 2: 1200 on either axis.
        assert!(hostile(0, Some(ME), -7, (1200.0, -1200.0)));
        assert!(!hostile(0, Some(ME), -7, (1200.5, 0.0)));
        assert!(!hostile(0, Some(ME), -7, (0.0, -1200.5)));
        let govts = govts(0);
        let mut npcs = [ship(1, ME, 0.0, 0.0)];
        let at = |npcs: &[Npc], x: f32| {
            let around = Surroundings {
                player: Some(player(x, 0.0)),
                govts: &govts,
                system_govt: Some(ME),
                record: -7,
                ..Surroundings::new(&[], npcs)
            };
            hostile_to_player(&npcs[0], &around)
        };
        npcs[0].aggression = 3;
        assert!(at(&npcs, 1800.0));
        assert!(!at(&npcs, 1800.5));
        npcs[0].aggression = 0;
        assert!(!at(&npcs, 1.0), "aggression 0 never hunts");
        assert_eq!(HUNT_REACH, 600.0);
    }

    #[test]
    fn the_hunt_is_measured_from_where_the_warship_is() {
        let govts = govts(0);
        let npcs = [ship(1, ME, 1000.0, -1000.0)];
        let at = |x: f32, y: f32| {
            let around = Surroundings {
                player: Some(player(1000.0 + x, -1000.0 + y)),
                govts: &govts,
                system_govt: Some(ME),
                record: -7,
                ..Surroundings::new(&[], &npcs)
            };
            hostile_to_player(&npcs[0], &around)
        };
        assert!(at(1200.0, -1200.0));
        assert!(!at(1200.5, 0.0));
        assert!(!at(0.0, 1200.5));
    }

    /// A xenophobic ship (NPC 1, govt 144 with `flags` besides) choosing
    /// between the player 100 above and a rival 300 to the right, with
    /// record `record` in a system of `system`.
    fn xenophobe_choice(flags: u16, system: Option<GovtId>, record: i16) -> Option<ShipRef> {
        xenophobe_choice_blessed(flags, system, record, Blessing::NONE)
    }

    /// [`xenophobe_choice`], the player blessed by `blessing`.
    fn xenophobe_choice_blessed(
        flags: u16,
        system: Option<GovtId>,
        record: i16,
        blessing: Blessing,
    ) -> Option<ShipRef> {
        xenophobe_pick(flags, system, record, blessing).map(|pick| pick.ship)
    }

    /// [`xenophobe_choice_blessed`], as [`pick_target`] gives it.
    fn xenophobe_pick(
        flags: u16,
        system: Option<GovtId>,
        record: i16,
        blessing: Blessing,
    ) -> Option<Pick> {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | flags,
                ..govt(144)
            },
            govt(145),
        ]);
        let npcs = [ship(1, XENO, 0.0, 0.0), ship(2, RIVAL, 300.0, 0.0)];
        let around = Surroundings {
            player: Some(PlayerSide {
                blessing,
                ..player(0.0, -100.0)
            }),
            govts: &govts,
            system_govt: system,
            record,
            ..Surroundings::new(&[], &npcs)
        };
        pick(&npcs[0], None, &around)
    }

    #[test]
    fn a_xenophobe_hunts_the_player_unless_told_not_to_or_at_home_in_good_standing() {
        assert_eq!(xenophobe_choice(0, Some(ME), 1000), Some(P), "elsewhere");
        assert_eq!(
            xenophobe_choice(0, Some(XENO), 0),
            Some(P),
            "at home, no better"
        );
        assert_eq!(xenophobe_choice(0, Some(XENO), -5), Some(P));
        assert_eq!(
            xenophobe_choice(0, Some(XENO), 1),
            Some(n(2)),
            "at home, in good standing"
        );
        assert_eq!(
            xenophobe_choice(NEVER_ATTACKS_PLAYER, Some(ME), 0),
            Some(n(2)),
            "never the player"
        );
        assert_eq!(
            xenophobe_choice(NEVER_ATTACKS_PLAYER | ALWAYS_ATTACKS_PLAYER, Some(ME), 0),
            Some(n(2)),
            "0x0004 isn't read: 0x0040 still holds"
        );
    }

    #[test]
    fn a_0x0004_xenophobe_at_home_with_the_player_in_good_standing_doesnt_take_it() {
        // crons-news-ranks-fidelity 32: `_SelectWarshipTarget` never reads
        // 0x0004 (@0x8a06f-0x8a413).
        assert_eq!(
            xenophobe_choice(ALWAYS_ATTACKS_PLAYER, Some(XENO), 1),
            Some(n(2))
        );
        assert_eq!(
            xenophobe_choice(ALWAYS_ATTACKS_PLAYER, Some(XENO), 0),
            Some(P),
            "not in good standing"
        );
    }

    #[test]
    fn a_0x0004_warship_at_home_with_the_player_in_good_standing_out_of_reach_takes_no_player() {
        // crons-news-ranks-fidelity 32: the legal table reads no 0x0004.
        let far = (5000.0, 5000.0);
        assert!(!hostile(ALWAYS_ATTACKS_PLAYER, Some(ME), 30, far));
        assert!(!hostile(ALWAYS_ATTACKS_PLAYER, Some(ME), 30, NEAR));
        let npcs = [me()];
        assert_eq!(chosen(ALWAYS_ATTACKS_PLAYER, &npcs, None, 30, far), None);
        assert_eq!(
            chosen(ALWAYS_ATTACKS_PLAYER, &npcs, None, -7, NEAR),
            Some(P),
            "wanted and in reach, by the table"
        );
    }

    #[test]
    fn a_spared_ship_never_takes_the_player_but_still_its_npc_enemies() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | ALWAYS_ATTACKS_PLAYER,
                crime_tol: 6,
                ..govt(144)
            },
            govt(145),
        ]);
        let mut spared = ship(1, XENO, 0.0, 0.0);
        spared.spared = true;
        let rival = ship(2, RIVAL, 300.0, 0.0);
        let choose = |npcs: &[Npc], current| {
            let around = Surroundings {
                player: Some(player(0.0, -100.0)),
                govts: &govts,
                system_govt: Some(XENO),
                record: -100,
                ..Surroundings::new(&[], npcs)
            };
            select(&npcs[0], current, &around)
        };
        let both = [spared.clone(), rival.clone()];
        assert_eq!(choose(&both, Some(P)), Some(n(2)), "the player not kept");
        assert_eq!(choose(&both, None), Some(n(2)), "an NPC enemy still");
        assert_eq!(choose(&[spared.clone()], Some(P)), None, "never the player");
        let unspared = [ship(1, XENO, 0.0, 0.0), rival];
        assert_eq!(choose(&unspared, None), Some(P), "as before");
        assert_eq!(choose(&unspared[..1], Some(P)), Some(P));
    }

    #[test]
    fn a_spared_warship_neither_hunts_nor_helps_against_the_player_and_drops_it() {
        let govts = govts(ALWAYS_ATTACKS_PLAYER);
        let mut spared = ship(1, ME, 0.0, 0.0);
        spared.spared = true;
        let mut kin = ship(2, ME, 50.0, 0.0);
        kin.goal = Goal::Attack(P);
        let npcs = [spared.clone(), kin];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: Some(ME),
            record: -100,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), None);
        assert!(drops(&npcs[0], P, &around));
        let mut unspared = npcs.clone();
        unspared[0].spared = false;
        let around = Surroundings {
            npcs: &unspared,
            ..around
        };
        assert_eq!(select(&unspared[0], None, &around), Some(P));
        assert!(!drops(&unspared[0], P, &around));
    }

    #[test]
    fn a_xenophobe_never_hunts_a_player_breaking_up() {
        let govts = Governments::new([
            GovtRecord {
                flags: XENOPHOBIC | ALWAYS_ATTACKS_PLAYER,
                ..govt(144)
            },
            govt(145),
        ]);
        let npcs = [ship(1, XENO, 0.0, 0.0), ship(2, RIVAL, 300.0, 0.0)];
        let around = Surroundings {
            player: Some(PlayerSide {
                condition: Condition::Dying {
                    ticks_left: 3,
                    starting: false,
                },
                ..player(0.0, -100.0)
            }),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), Some(n(2)));
    }

    #[test]
    fn max_odds_scale_the_ships_strength_against_its_candidates() {
        // MaxOdds 200: a ship of 100 takes on up to 200.
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                max_odds: 200,
                ..govt(140)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..govt(142)
            },
        ]);
        let mut brute = ship(2, ENEMY, 100.0, 0.0);
        brute.hull.strength = 200.0;
        let npcs = [me(), brute];
        let around = Surroundings {
            player: Some(player(0.0, -5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), Some(n(2)));
        let mut stronger = npcs.clone();
        stronger[1].hull.strength = 201.0;
        let around = Surroundings {
            player: Some(player(0.0, -5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &stronger)
        };
        assert_eq!(select(&stronger[0], None, &around), None);
    }

    #[test]
    fn flags_0x0040_never_find_the_player_hostile_and_0x0004_changes_nothing() {
        let far = (5000.0, 5000.0);
        assert!(!hostile(ALWAYS_ATTACKS_PLAYER, Some(ME), 1000, far));
        assert!(!hostile(NEVER_ATTACKS_PLAYER, Some(ME), -1000, NEAR));
        assert!(!hostile(
            NEVER_ATTACKS_PLAYER | ALWAYS_ATTACKS_PLAYER,
            Some(ME),
            -1000,
            NEAR
        ));
        assert!(!hostile(
            NEVER_ATTACKS_PLAYER | XENOPHOBIC,
            Some(ENEMY),
            0,
            NEAR
        ));
    }

    #[test]
    fn no_player_or_one_breaking_up_is_never_attacked() {
        let govts = govts(0);
        let npcs = [ship(1, ME, 0.0, 0.0)];
        let mut around = Surroundings {
            govts: &govts,
            system_govt: Some(ME),
            record: -100,
            ..Surroundings::new(&[], &npcs)
        };
        assert!(!hostile_to_player(&npcs[0], &around));
        around.player = Some(PlayerSide {
            condition: Condition::Dying {
                ticks_left: 2,
                starting: false,
            },
            ..player(0.0, -100.0)
        });
        assert!(!hostile_to_player(&npcs[0], &around));
        around.player = Some(PlayerSide {
            condition: Condition::Disabled,
            ..player(0.0, -100.0)
        });
        assert!(
            hostile_to_player(&npcs[0], &around),
            "disabled is no excuse"
        );
    }

    /// The ship (NPC 1 at the centre) chooses among `npcs` (it first) and
    /// the player at (`x`, `y`), wanted at home or not as `record` says.
    fn chosen(
        flags: u16,
        npcs: &[Npc],
        current: Option<ShipRef>,
        record: i16,
        at: (f32, f32),
    ) -> Option<ShipRef> {
        chosen_pick(flags, npcs, current, record, at).map(|pick| pick.ship)
    }

    /// [`chosen`], as [`pick_target`] gives it.
    fn chosen_pick(
        flags: u16,
        npcs: &[Npc],
        current: Option<ShipRef>,
        record: i16,
        (x, y): (f32, f32),
    ) -> Option<Pick> {
        let govts = govts(flags);
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            system_govt: Some(ME),
            record,
            ..Surroundings::new(&[], npcs)
        };
        pick(&npcs[0], current, &around)
    }

    fn me() -> Npc {
        ship(1, ME, 0.0, 0.0)
    }

    #[test]
    fn a_live_target_is_kept() {
        let npcs = [
            me(),
            ship(2, ENEMY, 10.0, 0.0),
            ship(3, NEUTRAL, 500.0, 0.0),
        ];
        assert_eq!(chosen(0, &npcs, Some(n(3)), 0, NEAR), Some(n(3)));
        assert_eq!(chosen(0, &npcs, Some(n(9)), 0, NEAR), Some(n(2)), "gone");
        let mut dying = npcs.clone();
        dying[2].condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert_eq!(chosen(0, &dying, Some(n(3)), 0, NEAR), Some(n(2)));
    }

    #[test]
    fn a_xenophobe_takes_the_nearest_of_the_player_and_its_npc_enemies() {
        let npcs = [
            me(),
            ship(2, ALLY, 50.0, 0.0),
            ship(3, ENEMY, 150.0, 0.0),
            ship(4, XENO, 20.0, 0.0),
        ];
        // The xenophobe is NPC 1 here: govt 144, at war with everyone it
        // is not allied with, and allied with 140 (NPC 4).
        let mut xeno = npcs.clone();
        xeno[0].govt = Some(XENO);
        xeno[3].govt = Some(ME);
        assert_eq!(
            chosen(0, &xeno, None, 0, (0.0, -100.0)),
            Some(n(2)),
            "the nearest enemy"
        );
        assert_eq!(
            chosen(0, &xeno, None, 0, (0.0, -10.0)),
            Some(P),
            "the player nearer"
        );
        let mut fleet = xeno.clone();
        fleet[1].leader = Some(NpcId(1));
        assert_eq!(
            chosen(0, &fleet, None, 0, (0.0, -1000.0)),
            Some(n(3)),
            "not its own fleet"
        );
    }

    #[test]
    fn a_ship_comes_to_the_help_of_its_first_ally_in_a_fight() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Flee(n(4));
        let mut kin = ship(3, ME, 2000.0, 0.0);
        kin.goal = Goal::Attack(P);
        let pirate = ship(4, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), trader, kin, pirate];
        assert_eq!(
            chosen(0, &npcs, None, 0, (0.0, -100.0)),
            Some(n(4)),
            "the first"
        );
        let mut later = npcs.clone();
        later[1].goal = Goal::Idle;
        assert_eq!(
            chosen(0, &later, None, 0, (0.0, -100.0)),
            Some(P),
            "its own kind"
        );
    }

    #[test]
    fn a_ship_never_helps_an_ally_plundering_only_one_attacking_sniping_or_fleeing() {
        // The ally's goal must be 3 or 4 (`sub 3; cmp 1` @0x8a494-0x8a4a2):
        // goal 0xd is neither.
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        let pirate = ship(4, NEUTRAL, 3000.0, 0.0);
        for (goal, helped) in [
            (Goal::Attack(n(4)), Some(n(4))),
            (Goal::Snipe(n(4)), Some(n(4))),
            (Goal::Flee(n(4)), Some(n(4))),
            (
                Goal::Plunder {
                    ship: n(4),
                    hopeless: false,
                },
                None,
            ),
        ] {
            trader.goal = goal;
            let npcs = [me(), trader.clone(), pirate.clone()];
            assert_eq!(chosen(0, &npcs, None, 0, (0.0, -100.0)), helped, "{goal:?}");
        }
    }

    #[test]
    fn a_ship_helps_only_with_its_strength_times_max_odds_no_less_than_the_foes() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Attack(n(3));
        trader.hull.strength = 0.0;
        let mut pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        // The ship and its ally the trader: 100 and none.
        pirate.hull.strength = 100.0;
        let npcs = [me(), trader, pirate.clone()];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)), "even");
        let mut stronger = npcs.clone();
        stronger[2].hull.strength = 101.0;
        assert_eq!(chosen(0, &stronger, None, 0, NEAR), None, "too strong");
    }

    #[test]
    fn a_ship_never_helps_an_independent_or_a_downed_ally() {
        let mut independent = ship(2, ALLY, 1000.0, 0.0);
        independent.govt = None;
        independent.goal = Goal::Attack(n(3));
        let pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), independent, pirate];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), None);
        let mut disabled = npcs.clone();
        disabled[1].govt = Some(ALLY);
        disabled[1].condition = Condition::Disabled;
        assert_eq!(chosen(0, &disabled, None, 0, NEAR), None);
        let mut arriving = disabled.clone();
        arriving[1].condition = Condition::Intact;
        arriving[1].mode = Mode::JumpingIn { tick: 0 };
        assert_eq!(chosen(0, &arriving, None, 0, NEAR), None);
        let mut gone_foe = arriving.clone();
        gone_foe[1].mode = Mode::Flying;
        gone_foe[1].goal = Goal::Attack(n(9));
        assert_eq!(chosen(0, &gone_foe, None, 0, NEAR), None);
        let mut at_me = gone_foe;
        at_me[1].goal = Goal::Attack(n(1));
        assert_eq!(
            chosen(0, &at_me, None, 0, NEAR),
            None,
            "no help against itself, and an ally is no threat"
        );
    }

    #[test]
    fn a_ship_never_helps_an_ally_against_its_own_kind_an_ally_or_its_fleet() {
        // NPC 2, an allied trader, flees from or fights NPC 3; NPC 4, a
        // later ally, fights the pirate NPC 5.
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Flee(n(3));
        let foe = ship(3, ME, 2000.0, 0.0);
        let mut later = ship(4, ALLY, 1500.0, 0.0);
        later.goal = Goal::Attack(n(5));
        let pirate = ship(5, NEUTRAL, 3000.0, 0.0);
        let npcs = [me(), trader, foe, later, pirate];
        assert_eq!(
            chosen(0, &npcs, None, 0, NEAR),
            Some(n(5)),
            "not its own kind, but the next ally's foe"
        );
        let mut fought = npcs.clone();
        fought[1].goal = Goal::Attack(n(3));
        assert_eq!(chosen(0, &fought, None, 0, NEAR), Some(n(5)), "fought");
        let mut allied = npcs.clone();
        allied[1].govt = Some(ME);
        allied[2].govt = Some(ALLY);
        assert_eq!(chosen(0, &allied, None, 0, NEAR), Some(n(5)), "an ally");
        let mut escort = npcs.clone();
        escort[2].govt = Some(NEUTRAL);
        escort[2].leader = Some(NpcId(1));
        assert_eq!(chosen(0, &escort, None, 0, NEAR), Some(n(5)), "its fleet");
        let mut alone = npcs.clone();
        alone[3].goal = Goal::Idle;
        assert_eq!(chosen(0, &alone, None, 0, NEAR), None, "nothing else");
    }

    #[test]
    fn an_ally_or_its_fleet_fighting_the_ship_is_no_threat_to_turn_on() {
        let mut ally = ship(2, ALLY, 10.0, 0.0);
        ally.goal = Goal::Attack(n(1));
        let mut kin = ship(3, ME, 20.0, 0.0);
        kin.goal = Goal::Flee(n(1));
        let mut escort = ship(4, NEUTRAL, 30.0, 0.0);
        escort.leader = Some(NpcId(1));
        escort.goal = Goal::Snipe(n(1));
        let mut stranger = ship(5, NEUTRAL, 900.0, 0.0);
        stranger.goal = Goal::Attack(n(1));
        let npcs = [me(), ally, kin, escort, stranger];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(5)));
        assert_eq!(chosen(0, &npcs[..4], None, 0, NEAR), None);
    }

    #[test]
    fn the_nearest_of_a_wanted_player_and_the_npc_enemies_is_attacked() {
        let npcs = [me(), ship(2, ENEMY, 300.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        assert_eq!(
            chosen(0, &npcs, None, 0, NEAR),
            Some(n(3)),
            "the player is clean"
        );
        assert_eq!(
            chosen(0, &npcs, None, -7, NEAR),
            Some(P),
            "wanted, and nearest"
        );
        assert_eq!(
            chosen(0, &npcs, None, -7, (0.0, -200.0)),
            Some(P),
            "a tie: the player"
        );
        let tie = [me(), ship(2, ENEMY, 200.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        assert_eq!(chosen(0, &tie, None, 0, NEAR), Some(n(2)), "the earlier");
    }

    #[test]
    fn the_nearest_is_measured_from_where_the_ship_is() {
        let npcs = [
            ship(1, ME, 1000.0, 0.0),
            ship(2, ENEMY, 900.0, 0.0),
            ship(3, RIVAL, 0.0, 0.0),
        ];
        assert_eq!(
            chosen(0, &npcs, None, 0, (5000.0, 5000.0)),
            Some(n(2)),
            "100 away, not 1000"
        );
    }

    #[test]
    fn a_candidate_stronger_than_the_ship_times_its_odds_is_dropped() {
        let mut brute = ship(2, ENEMY, 100.0, 0.0);
        brute.hull.strength = 100.0;
        let npcs = [me(), brute, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(2)), "even");
        let mut stronger = npcs.clone();
        stronger[1].hull.strength = 101.0;
        assert_eq!(chosen(0, &stronger, None, 0, NEAR), Some(n(3)));
    }

    #[test]
    fn a_disabled_enemy_is_skipped_without_destroying_weapons() {
        let mut wreck = ship(2, ENEMY, 100.0, 0.0);
        wreck.condition = Condition::Disabled;
        let npcs = [me(), wreck, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        let mut armed = npcs.clone();
        armed[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(chosen(0, &armed, None, 0, NEAR), Some(n(2)));
    }

    #[test]
    fn the_dying_and_the_arriving_are_never_candidates() {
        let mut dying = ship(2, ENEMY, 100.0, 0.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let mut arriving = ship(3, ENEMY, 150.0, 0.0);
        arriving.mode = Mode::JumpingIn { tick: 0 };
        let mut emerging = ship(5, ENEMY, 120.0, 0.0);
        emerging.mode = Mode::Emerging { ticks_left: 3 };
        let npcs = [me(), dying, arriving, emerging, ship(4, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(4)));
        assert_eq!(chosen(0, &npcs[..4], None, 0, NEAR), None);
    }

    #[test]
    fn with_nothing_else_a_ship_turns_on_the_nearest_threat() {
        let mut far = ship(2, NEUTRAL, 900.0, 0.0);
        far.goal = Goal::Flee(n(1));
        let mut near = ship(3, NEUTRAL, 300.0, 0.0);
        near.goal = Goal::Attack(n(1));
        let mut inspecting = ship(4, NEUTRAL, 10.0, 0.0);
        inspecting.goal = Goal::Inspect(n(1));
        let npcs = [me(), far, near, inspecting];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        assert_eq!(chosen(0, &npcs[..1], None, 0, NEAR), None);
        let mut idle = npcs.clone();
        idle[1].goal = Goal::Idle;
        idle[2].goal = Goal::Idle;
        assert_eq!(chosen(0, &idle, None, 0, NEAR), None);
    }

    #[test]
    fn a_fleet_leader_turns_on_a_ship_attacking_one_it_leads() {
        let mut escort = ship(2, NEUTRAL, 50.0, 0.0);
        escort.leader = Some(NpcId(1));
        let mut raider = ship(3, NEUTRAL, 600.0, 0.0);
        raider.goal = Goal::Attack(n(2));
        let mut sniper = ship(4, NEUTRAL, 300.0, 0.0);
        sniper.goal = Goal::Snipe(n(2));
        let npcs = [me(), escort, raider, sniper];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(4)), "the nearest");
        let mut unled = npcs.clone();
        unled[1].leader = None;
        assert_eq!(chosen(0, &unled, None, 0, NEAR), None, "led by nobody");
        let mut others = npcs.clone();
        others[1].leader = Some(NpcId(9));
        assert_eq!(chosen(0, &others, None, 0, NEAR), None, "led by another");
    }

    #[test]
    fn a_target_is_dropped_once_gone_downed_harmlessly_allied_or_of_the_fleet() {
        fn around<'a>(npcs: &'a [Npc], govts: &'a Governments) -> Surroundings<'a> {
            Surroundings {
                player: Some(player(0.0, -100.0)),
                govts,
                ..Surroundings::new(&[], npcs)
            }
        }
        let govts = govts(0);
        let mut wreck = ship(2, ENEMY, 100.0, 0.0);
        wreck.condition = Condition::Disabled;
        let mut dying = ship(3, ENEMY, 100.0, 0.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let ally = ship(4, ALLY, 100.0, 0.0);
        let kin = ship(5, ME, 100.0, 0.0);
        let mut escort = ship(6, NEUTRAL, 100.0, 0.0);
        escort.leader = Some(NpcId(1));
        let enemy = ship(7, ENEMY, 100.0, 0.0);
        let mut npcs = vec![me(), wreck, dying, ally, kin, escort, enemy];
        for (target, drop) in [
            (n(2), true),
            (n(3), true),
            (n(4), true),
            (n(5), true),
            (n(6), true),
            (n(7), false),
            (n(9), true),
            (P, false),
        ] {
            assert_eq!(
                drops(&npcs[0], target, &around(&npcs, &govts)),
                drop,
                "{target:?}"
            );
        }
        npcs[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert!(
            !drops(&npcs[0], n(2), &around(&npcs, &govts)),
            "it can finish it off"
        );
    }

    // Persons.

    /// The target of ship 1 of govt 140, a person of `flags` with a
    /// grudge or not, spared or not, keeping `current`, with an enemy (NPC
    /// 2) 50 below it, and the player far off with a clean record at home,
    /// when it is in the system.
    fn grudge_target(
        flags: u16,
        grudge: bool,
        spared: bool,
        current: Option<ShipRef>,
        player_here: bool,
    ) -> Option<ShipRef> {
        grudge_target_blessed(flags, grudge, spared, current, player_here, Blessing::NONE)
    }

    /// [`grudge_target`], the player blessed by `blessing`.
    fn grudge_target_blessed(
        flags: u16,
        grudge: bool,
        spared: bool,
        current: Option<ShipRef>,
        player_here: bool,
        blessing: Blessing,
    ) -> Option<ShipRef> {
        grudge_pick(flags, grudge, spared, current, player_here, blessing).map(|pick| pick.ship)
    }

    /// [`grudge_target_blessed`], as [`pick_target`] gives it.
    fn grudge_pick(
        flags: u16,
        grudge: bool,
        spared: bool,
        current: Option<ShipRef>,
        player_here: bool,
        blessing: Blessing,
    ) -> Option<Pick> {
        let govts = govts(0);
        let mut grudger = ship(1, ME, 0.0, 0.0);
        grudger.spared = spared;
        grudger.person = Some(crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(510),
            flags,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge,
            quoted: false,
            quoted_at: None,
        });
        let npcs = [grudger, ship(2, ENEMY, 0.0, 50.0)];
        let around = Surroundings {
            player: player_here.then_some(PlayerSide {
                blessing,
                ..player(0.0, -1000.0)
            }),
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        pick(&npcs[0], current, &around)
    }

    #[test]
    fn a_person_holding_a_grudge_picks_the_player_before_any_other() {
        let grudge = crate::traffic::GRUDGE;
        assert_eq!(grudge_target(grudge, true, false, None, true), Some(P));
        assert_eq!(
            grudge_target(grudge, false, false, None, true),
            Some(n(2)),
            "no grudge"
        );
        assert_eq!(
            grudge_target(0, true, false, None, true),
            Some(n(2)),
            "no 0x0001"
        );
        assert_eq!(
            grudge_target(grudge, true, true, None, true),
            Some(n(2)),
            "spared"
        );
        assert_eq!(
            grudge_target(grudge, true, false, Some(n(2)), true),
            Some(n(2)),
            "it keeps the ship it fights"
        );
        assert_eq!(
            grudge_target(grudge, true, false, None, false),
            Some(n(2)),
            "no player in the system"
        );
    }

    // Out of ammunition.

    /// [`me`] with no weapon.
    fn dry() -> Npc {
        Npc {
            armament: Armament::default(),
            ..me()
        }
    }

    #[test]
    fn a_ship_out_of_ammunition_takes_no_new_target() {
        let npcs = [dry(), ship(2, ENEMY, 100.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, -7, NEAR), None);
        let armed = [me(), ship(2, ENEMY, 100.0, 0.0)];
        assert_eq!(chosen(0, &armed, None, -7, NEAR), Some(P));
    }

    #[test]
    fn a_ship_out_of_ammunition_keeps_the_ship_it_fights() {
        let npcs = [dry(), ship(2, ENEMY, 100.0, 0.0)];
        assert_eq!(chosen(0, &npcs, Some(n(2)), 0, NEAR), Some(n(2)));
    }

    #[test]
    fn a_person_out_of_ammunition_still_hunts_the_player_it_holds_a_grudge_against() {
        let govts = govts(0);
        let mut grudger = dry();
        grudger.person = Some(crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(510),
            flags: crate::traffic::GRUDGE,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: true,
            quoted: false,
            quoted_at: None,
        });
        let npcs = [grudger];
        let around = Surroundings {
            player: Some(player(0.0, -1000.0)),
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), Some(P));
    }

    // Mission ships.

    /// The target of ship 1 of govt 140, with `ShipBehav` 0, destroying
    /// weapons or not, spared or not, with an enemy (NPC 2) 50 below it
    /// and, when it is in the system, the player far off in good standing
    /// at home, in `condition`.
    fn hunter_target(
        destroying: bool,
        spared: bool,
        player_here: bool,
        condition: Condition,
    ) -> Option<ShipRef> {
        hunter_target_blessed(destroying, spared, player_here, condition, Blessing::NONE)
    }

    /// [`hunter_target`], the player blessed by `blessing`.
    fn hunter_target_blessed(
        destroying: bool,
        spared: bool,
        player_here: bool,
        condition: Condition,
        blessing: Blessing,
    ) -> Option<ShipRef> {
        hunter_pick(destroying, spared, player_here, condition, blessing).map(|pick| pick.ship)
    }

    /// [`hunter_target_blessed`], as [`pick_target`] gives it.
    fn hunter_pick(
        destroying: bool,
        spared: bool,
        player_here: bool,
        condition: Condition,
        blessing: Blessing,
    ) -> Option<Pick> {
        let govts = govts(0);
        let mut hunter = me();
        hunter.spared = spared;
        hunter.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        });
        if destroying {
            hunter.armament = Armament::new([(
                WeaponSpec::new(&WeaponRecord {
                    mass_dmg: 1,
                    ..weapon(128)
                }),
                1,
            )]);
        }
        let npcs = [hunter, ship(2, ENEMY, 0.0, 50.0)];
        let mut at = player(0.0, -1000.0);
        at.condition = condition;
        at.blessing = blessing;
        let around = Surroundings {
            player: player_here.then_some(at),
            govts: &govts,
            system_govt: Some(ME),
            record: 30,
            ..Surroundings::new(&[], &npcs)
        };
        pick(&npcs[0], None, &around)
    }

    #[test]
    fn a_ship_behav_0_ship_takes_the_player_before_a_nearer_enemy() {
        assert_eq!(
            hunter_target(false, false, true, Condition::Intact),
            Some(P)
        );
    }

    #[test]
    fn a_ship_behav_0_ship_without_destroying_weapons_takes_nothing_while_the_player_is_disabled() {
        assert_eq!(hunter_target(false, false, true, Condition::Disabled), None);
        assert_eq!(
            hunter_target(true, false, true, Condition::Disabled),
            Some(P),
            "it can destroy the player"
        );
    }

    #[test]
    fn a_ship_behav_0_ship_without_destroying_weapons_takes_nothing_while_the_player_breaks_up() {
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        assert_eq!(hunter_target(false, false, true, dying), None);
        assert_eq!(
            hunter_target(true, false, true, dying),
            Some(P),
            "it can destroy the player"
        );
    }

    #[test]
    fn a_ship_behav_0_ship_falls_through_with_no_player_in_the_system() {
        assert_eq!(
            hunter_target(false, false, false, Condition::Intact),
            Some(n(2))
        );
    }

    #[test]
    fn a_paid_off_ship_behav_0_ship_still_takes_the_player() {
        // Neither @0x89f25 nor `_HighLevelAIHandler` @0x8d628 reads the
        // bribe; the bribe's leave (state 2, @0x961eb) is no state it skips.
        assert_eq!(hunter_target(false, true, true, Condition::Intact), Some(P));
    }

    #[test]
    fn a_ship_behav_0_ship_out_of_ammunition_still_takes_the_player() {
        let govts = govts(0);
        let mut hunter = dry();
        hunter.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        });
        let npcs = [hunter, ship(2, ENEMY, 0.0, 50.0)];
        let around = Surroundings {
            player: Some(player(0.0, -1000.0)),
            govts: &govts,
            system_govt: Some(ME),
            record: 30,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), Some(P));
    }

    // The blessing.

    /// [`chosen`], the player blessed by `blessing`.
    fn chosen_blessed(
        npcs: &[Npc],
        record: i16,
        (x, y): (f32, f32),
        blessing: Blessing,
    ) -> Option<ShipRef> {
        let govts = govts(0);
        let around = Surroundings {
            player: Some(PlayerSide {
                blessing,
                ..player(x, y)
            }),
            govts: &govts,
            system_govt: Some(ME),
            record,
            ..Surroundings::new(&[], npcs)
        };
        select(&npcs[0], None, &around)
    }

    #[test]
    fn a_blessed_player_is_no_xenophobes_candidate() {
        for flags in [0, ALWAYS_ATTACKS_PLAYER] {
            assert_eq!(
                xenophobe_choice(flags, Some(ME), 1000),
                Some(P),
                "{flags:#x}: unblessed"
            );
            assert_eq!(
                xenophobe_choice_blessed(flags, Some(ME), 1000, blessing_of(XENO)),
                Some(n(2)),
                "{flags:#x}: the nearest NPC enemy instead"
            );
        }
    }

    #[test]
    fn helping_allies_passes_over_an_ally_fighting_a_blessed_player_or_a_ship_the_player_leads() {
        let mut first = ship(2, ALLY, 1000.0, 0.0);
        first.goal = Goal::Attack(P);
        let mut second = ship(3, ALLY, 1500.0, 0.0);
        second.goal = Goal::Attack(n(4));
        let pirate = ship(4, NEUTRAL, 3000.0, 0.0);
        let mut escort = ship(5, NEUTRAL, 2000.0, 0.0);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let npcs = [
            me(),
            first.clone(),
            second.clone(),
            pirate.clone(),
            escort.clone(),
        ];
        let mut at_escort = npcs.clone();
        at_escort[1].goal = Goal::Attack(n(5));
        for (npcs, quarry, case) in [
            (&npcs, P, "the player"),
            (&at_escort, n(5), "a ship the player leads"),
        ] {
            assert_eq!(
                chosen_blessed(npcs, 0, NEAR, Blessing::NONE),
                Some(quarry),
                "{case}: unblessed, the first ally's quarry"
            );
            assert_eq!(
                chosen_blessed(npcs, 0, NEAR, blessing_of(ME)),
                Some(n(4)),
                "{case}: the second ally's"
            );
        }
    }

    #[test]
    fn a_blessing_government_never_finds_the_player_hostile() {
        for (flags, system, record, at, case) in [
            (0, Some(ME), -1000, NEAR, "at home, wanted"),
            (0, Some(ENEMY), 1000, NEAR, "in an enemy's system"),
            (NOSY, Some(NEUTRAL), -1000, NEAR, "nosy, elsewhere"),
        ] {
            assert!(
                hostile_blessed(flags, system, record, at, Blessing::NONE),
                "{case}: unblessed"
            );
            assert!(
                !hostile_blessed(flags, system, record, at, blessing_of(ME)),
                "{case}"
            );
        }
        let npcs = [me()];
        assert_eq!(
            chosen_blessed(&npcs, -1000, NEAR, Blessing::NONE),
            Some(P),
            "unblessed"
        );
        assert_eq!(chosen_blessed(&npcs, -1000, NEAR, blessing_of(ME)), None);
    }

    #[test]
    fn only_the_deciding_ships_own_government_blesses() {
        let other = blessing_of(ALLY);
        assert!(hostile_blessed(0, Some(ME), -1000, NEAR, other));
        assert_eq!(
            xenophobe_choice_blessed(0, Some(ME), 1000, blessing_of(ME)),
            Some(P),
            "the xenophobe's own is 144"
        );
        let mut first = ship(2, ALLY, 1000.0, 0.0);
        first.goal = Goal::Attack(P);
        let npcs = [me(), first];
        assert_eq!(chosen_blessed(&npcs, 0, NEAR, other), Some(P), "helping");
        assert_eq!(chosen_blessed(&npcs[..1], -1000, NEAR, other), Some(P));
    }

    #[test]
    fn a_grudge_or_a_shipbehav_0_hunter_takes_a_blessed_player() {
        let blessed = blessing_of(ME);
        assert_eq!(
            grudge_target_blessed(crate::traffic::GRUDGE, true, false, None, true, blessed),
            Some(P)
        );
        assert_eq!(
            hunter_target_blessed(false, false, true, Condition::Intact, blessed),
            Some(P)
        );
    }

    #[test]
    fn a_blessing_leaves_the_threat_step_its_npc_threat() {
        let mut stranger = ship(2, NEUTRAL, 900.0, 0.0);
        stranger.goal = Goal::Attack(n(1));
        let npcs = [me(), stranger];
        assert_eq!(chosen_blessed(&npcs, 0, NEAR, blessing_of(ME)), Some(n(2)));
    }

    // The quote call (`_AIMakeShipAttackPlayer`).

    #[test]
    fn a_grudge_a_xenophobe_and_the_hunt_turn_on_the_player_with_the_quote_call_but_helping_an_ally_does_not()
     {
        let turns = |ship| {
            Some(Pick {
                ship,
                turns_on_player: true,
            })
        };
        let keeps = |ship| {
            Some(Pick {
                ship,
                turns_on_player: false,
            })
        };
        let none = Blessing::NONE;
        let grudge = crate::traffic::GRUDGE;
        assert_eq!(
            grudge_pick(grudge, true, false, None, true, none),
            turns(P),
            "a grudge (@0x89f20)"
        );
        assert_eq!(
            hunter_pick(false, false, true, Condition::Intact, none),
            turns(P),
            "a ShipBehav 0 mission ship (@0x89f20)"
        );
        assert_eq!(
            xenophobe_pick(0, Some(ME), 1000, none),
            turns(P),
            "a xenophobe's nearest (@0x8a40e)"
        );
        let wanted = [me(), ship(2, ENEMY, 300.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        assert_eq!(
            chosen_pick(0, &wanted, None, -7, NEAR),
            turns(P),
            "the hunt's nearest (@0x8ad31)"
        );
        let mut kin = ship(3, ME, 2000.0, 0.0);
        kin.goal = Goal::Attack(P);
        let helping = [
            me(),
            ship(2, ALLY, 1000.0, 0.0),
            kin,
            ship(4, NEUTRAL, 3000.0, 0.0),
        ];
        assert_eq!(
            chosen_pick(0, &helping, None, 0, NEAR),
            keeps(P),
            "helping an ally against the player (@0x8a5b0-0x8a5ca)"
        );
        assert_eq!(chosen_pick(0, &wanted, Some(P), -7, NEAR), keeps(P), "kept");
        assert_eq!(
            chosen_pick(0, &wanted, None, 0, NEAR),
            keeps(n(3)),
            "the hunt's NPC"
        );
        assert_eq!(
            xenophobe_pick(0, Some(XENO), 1, none),
            keeps(n(2)),
            "a xenophobe's NPC"
        );
        assert_eq!(
            grudge_pick(grudge, false, false, None, true, none),
            keeps(n(2)),
            "no grudge"
        );
        let mut threat = ship(2, NEUTRAL, 900.0, 0.0);
        threat.goal = Goal::Attack(n(1));
        assert_eq!(
            chosen_pick(0, &[me(), threat], None, 0, NEAR),
            keeps(n(2)),
            "a threat"
        );
    }

    // The xenophobe step (5).

    /// A xenophobe (NPC 1 of govt 144, at war with all but 140) at the
    /// centre among `others`, the player at (`x`, `y`) out of good standing
    /// at home, by `rules`, its draws `draws`.
    fn xenophobe_among(
        others: &[Npc],
        (x, y): (f32, f32),
        player: impl FnOnce(PlayerSide) -> PlayerSide,
        iff: &std::collections::BTreeSet<GovtId>,
    ) -> Option<Pick> {
        let govts = govts(0);
        let mut npcs = vec![ship(1, XENO, 0.0, 0.0)];
        npcs.extend_from_slice(others);
        let around = Surroundings {
            player: Some(player(self::player(x, y))),
            govts: &govts,
            system_govt: Some(XENO),
            record: 0,
            iff_marked: iff,
            ..Surroundings::new(&[], &npcs)
        };
        pick(&npcs[0], None, &around)
    }

    fn unmarked() -> std::collections::BTreeSet<GovtId> {
        std::collections::BTreeSet::new()
    }

    #[test]
    fn a_xenophobes_npc_candidates_are_intact_enemies_it_doesnt_lead_and_isnt_allied_with() {
        let far = (0.0, -2000.0);
        let none = &unmarked();
        let mut disabled = ship(2, ENEMY, 50.0, 0.0);
        disabled.condition = Condition::Disabled;
        let mut escort = ship(3, ENEMY, 60.0, 0.0);
        escort.leader = Some(NpcId(1));
        let ally = ship(4, ME, 70.0, 0.0);
        let others = [disabled, escort, ally];
        assert_eq!(
            xenophobe_among(&others, far, |p| p, none).map(|pick| pick.ship),
            Some(P),
            "none of them, so the player far off"
        );
        let enemy = ship(5, ENEMY, 900.0, 0.0);
        let mut more = others.to_vec();
        more.push(enemy);
        assert_eq!(
            xenophobe_among(&more, far, |p| p, none).map(|pick| pick.ship),
            Some(n(5))
        );
    }

    #[test]
    fn a_xenophobes_candidate_stronger_than_it_times_its_odds_is_dropped() {
        let none = &unmarked();
        let mut brute = ship(2, ENEMY, 50.0, 0.0);
        brute.hull.strength = 101.0;
        let pick = xenophobe_among(&[brute.clone()], (0.0, -2000.0), |p| p, none);
        assert_eq!(pick.map(|pick| pick.ship), Some(P));
        brute.hull.strength = 100.0;
        let pick = xenophobe_among(&[brute], (0.0, -2000.0), |p| p, none);
        assert_eq!(pick.map(|pick| pick.ship), Some(n(2)));
    }

    #[test]
    fn a_xenophobe_never_takes_a_disabled_player_or_one_in_the_escape_pod() {
        let none = &unmarked();
        let near = (0.0, -10.0);
        let pirate = ship(2, ENEMY, 900.0, 0.0);
        let disabled = |p: PlayerSide| PlayerSide {
            condition: Condition::Disabled,
            ..p
        };
        let pod = |p: PlayerSide| PlayerSide {
            ship: ESCAPE_POD_SHIP,
            ..p
        };
        assert_eq!(
            xenophobe_among(std::slice::from_ref(&pirate), near, |p| p, none).map(|pick| pick.ship),
            Some(P)
        );
        assert_eq!(
            xenophobe_among(std::slice::from_ref(&pirate), near, disabled, none)
                .map(|pick| pick.ship),
            Some(n(2))
        );
        assert_eq!(
            xenophobe_among(&[pirate], near, pod, none).map(|pick| pick.ship),
            Some(n(2))
        );
    }

    /// A weapon that destroys (`MassDmg` 1).
    fn destroying() -> Armament {
        Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )])
    }

    #[test]
    fn a_xenophobes_step_7_pick_wins_over_its_step_5_player_and_the_call_stands() {
        // A disabled enemy is no step-5 candidate, but step 7 takes it for
        // a ship with destroying weapons, nearer than the player.
        let govts = govts(0);
        let mut xeno = ship(1, XENO, 0.0, 0.0);
        xeno.armament = destroying();
        let mut wreck = ship(2, ENEMY, 50.0, 0.0);
        wreck.condition = Condition::Disabled;
        let npcs = [xeno, wreck];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            pick(&npcs[0], None, &around),
            Some(Pick {
                ship: n(2),
                turns_on_player: true
            }),
            "@0x8a40e called for the player before step 7 took the wreck"
        );
    }

    #[test]
    fn an_iff_marked_xenophobe_keeps_its_step_5_player_only_when_step_7_has_none() {
        let marked: std::collections::BTreeSet<GovtId> = [XENO].into();
        let turns = |ship| {
            Some(Pick {
                ship,
                turns_on_player: true,
            })
        };
        assert_eq!(
            xenophobe_among(&[], (0.0, -100.0), |p| p, &marked),
            turns(P),
            "only the player in view: @0x8aa13 clears the candidate, not +0x70"
        );
        let pirate = ship(2, ENEMY, 900.0, 0.0);
        assert_eq!(
            xenophobe_among(std::slice::from_ref(&pirate), (0.0, -100.0), |p| p, &marked),
            turns(n(2)),
            "an NPC enemy in view: step 7 takes it"
        );
        assert_eq!(
            xenophobe_among(&[pirate], (0.0, -100.0), |p| p, &unmarked()),
            turns(P),
            "unmarked, the nearer player"
        );
    }

    #[test]
    fn an_iff_marked_warship_never_takes_the_player_but_still_its_npc_enemies() {
        let govts = govts(0);
        let marked: std::collections::BTreeSet<GovtId> = [ME].into();
        let npcs = [me(), ship(2, ENEMY, 300.0, 0.0)];
        let choose = |npcs: &[Npc], iff| {
            let around = Surroundings {
                player: Some(PlayerSide {
                    combat_govt: Some(RIVAL),
                    ..player(0.0, -100.0)
                }),
                govts: &govts,
                system_govt: Some(ME),
                record: -100,
                iff_marked: iff,
                ..Surroundings::new(&[], npcs)
            };
            pick_target(
                &npcs[0],
                None,
                &around,
                TargetRules::default(),
                &mut Draws::of(&[0]),
            )
            .map(|pick| pick.ship)
        };
        let none = unmarked();
        assert_eq!(choose(&npcs, &none), Some(P), "wanted, and rolled");
        assert_eq!(choose(&npcs, &marked), Some(n(2)));
        assert_eq!(choose(&npcs[..1], &marked), None);
    }

    // The enemy-class roll.

    /// What ship 1 of govt 140 picks, with an enemy too strong for it (NPC
    /// 2, of govt 145) far off, the
    /// player at home far out of reach in good standing, flying a ship of
    /// `combat_govt`, the ship changed by `edit`, drawing `draws`: the pick,
    /// and the ranges asked.
    fn rolled(
        combat_govt: Option<GovtId>,
        edit: impl FnOnce(&mut Npc),
        draws: &[u32],
    ) -> (Option<ShipRef>, Vec<u32>) {
        let govts = govts(0);
        let mut roller = me();
        edit(&mut roller);
        let mut brute = ship(2, RIVAL, 9000.0, 0.0);
        brute.hull.strength = 10_000.0;
        let npcs = [roller, brute];
        let around = Surroundings {
            player: Some(PlayerSide {
                combat_govt,
                ..player(0.0, -5000.0)
            }),
            govts: &govts,
            system_govt: Some(ME),
            record: 30,
            ..Surroundings::new(&[], &npcs)
        };
        let mut chance = Draws::of(draws);
        let pick = select_target(&npcs[0], None, &around, TargetRules::default(), &mut chance);
        (pick, chance.asked)
    }

    #[test]
    fn the_enemy_class_roll_makes_the_player_a_candidate_on_a_0_below_50() {
        assert_eq!(rolled(Some(ENEMY), |_| (), &[0]), (Some(P), vec![50]));
        assert_eq!(rolled(Some(ENEMY), |_| (), &[1]), (None, vec![50]));
        assert_eq!(rolled(Some(ENEMY), |_| (), &[49]), (None, vec![50]));
        assert_eq!(ENEMY_CLASS_SIDES, 50);
    }

    #[test]
    fn the_enemy_class_roll_skips_a_dying_player_for_a_live_npc_enemy() {
        // The live test follows the draw (@0x8a9a2): a 0 still makes no
        // candidate of a player breaking up, and a farther enemy is taken.
        // The enemy is of govt 145, not the player's combat government, so
        // it adds nothing to the player's friend strength.
        let govts = govts(0);
        let npcs = [me(), ship(2, RIVAL, 0.0, 6000.0)];
        let around = Surroundings {
            player: Some(PlayerSide {
                combat_govt: Some(ENEMY),
                condition: Condition::Dying {
                    ticks_left: 3,
                    starting: false,
                },
                ..player(0.0, -5000.0)
            }),
            govts: &govts,
            system_govt: Some(ME),
            record: 30,
            ..Surroundings::new(&[], &npcs)
        };
        let mut chance = Draws::of(&[0]);
        let pick = select_target(&npcs[0], None, &around, TargetRules::default(), &mut chance);
        assert_eq!(chance.asked, vec![50]);
        assert_eq!(pick, Some(n(2)));
    }

    #[test]
    fn the_enemy_class_roll_isnt_asked_without_each_of_its_gates() {
        let led = |npc: &mut Npc| {
            npc.escort = Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            });
        };
        let mission = |npc: &mut Npc| {
            npc.mission = Some(crate::traffic::npc::MissionMark {
                slot: 0,
                behav: crate::traffic::ShipBehav::Own,
                formation: None,
                order: None,
            });
        };
        let aux = |npc: &mut Npc| npc.aux = Some(0);
        for (combat, change, case) in [
            (Some(ENEMY), &led as &dyn Fn(&mut Npc), "led by the player"),
            (Some(ENEMY), &mission, "mission-marked"),
            (Some(ENEMY), &aux, "aux-marked (mission-ships-fidelity 31)"),
            (None, &|_: &mut Npc| (), "no combat government"),
            (Some(NEUTRAL), &|_: &mut Npc| (), "not an enemy's"),
        ] {
            let (pick, asked) = rolled(combat, change, &[0]);
            assert_eq!(asked, Vec::<u32>::new(), "{case}");
            assert_eq!(pick, None, "{case}");
        }
        let (_, asked) = rolled(Some(ENEMY), |npc| npc.govt = Some(XENO), &[0]);
        assert_eq!(asked, Vec::<u32>::new(), "a xenophobe skips it");
    }

    #[test]
    fn an_aux_marked_ship_otherwise_picks_as_any_warship() {
        let govts = govts(0);
        let mut aux = me();
        aux.aux = Some(2);
        let npcs = [aux, ship(2, ENEMY, 300.0, 0.0), ship(3, RIVAL, 0.0, 200.0)];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: Some(ME),
            record: -7,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            select(&npcs[0], None, &around),
            Some(P),
            "the hunt's nearest"
        );
        let clean = Surroundings {
            record: 0,
            ..around
        };
        assert_eq!(select(&npcs[0], None, &clean), Some(n(3)));
    }

    // Steps 5-8 for a ship of no government.

    #[test]
    fn an_independent_warship_takes_no_target_past_step_4() {
        let govts = govts(0);
        let mut independent = me();
        independent.govt = None;
        let mut threat = ship(2, NEUTRAL, 300.0, 0.0);
        threat.goal = Goal::Attack(n(1));
        let mut enemy = ship(3, ENEMY, 100.0, 0.0);
        enemy.govt = None;
        let npcs = [independent, threat, enemy];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            record: -100,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(select(&npcs[0], None, &around), None);
        assert_eq!(
            select(&npcs[0], Some(n(3)), &around),
            Some(n(3)),
            "it keeps one"
        );
    }

    // The hunt (step 7).

    #[test]
    fn with_escorts_the_heaviest_class_wins_over_a_nearer_lighter_one() {
        let mut light = ship(2, ENEMY, 100.0, 0.0);
        light.hull.mass = 50.0;
        let mut heavy = ship(3, RIVAL, 900.0, 0.0);
        heavy.hull.mass = 400.0;
        let mut escort = ship(4, ME, 0.0, 50.0);
        escort.leader = Some(NpcId(1));
        let alone = [me(), light.clone(), heavy.clone()];
        assert_eq!(chosen(0, &alone, None, 0, NEAR), Some(n(2)), "the nearest");
        let led = [me(), light, heavy, escort];
        assert_eq!(chosen(0, &led, None, 0, NEAR), Some(n(3)), "the heaviest");
    }

    #[test]
    fn candidates_closer_than_a_whole_pixel_apart_tie_to_the_earlier() {
        let npcs = [me(), ship(2, ENEMY, 300.9, 0.0), ship(3, RIVAL, 0.0, 300.1)];
        assert_eq!(chosen(0, &npcs, None, 0, (0.0, -5000.0)), Some(n(2)));
        let swapped = [me(), ship(3, RIVAL, 0.0, 300.1), ship(2, ENEMY, 300.9, 0.0)];
        assert_eq!(chosen(0, &swapped, None, 0, (0.0, -5000.0)), Some(n(3)));
        let mut by_slot = npcs.clone();
        by_slot[1].berth = 5;
        by_slot[2].berth = 4;
        assert_eq!(
            chosen(0, &by_slot, None, 0, (0.0, -5000.0)),
            Some(n(3)),
            "the earlier slot"
        );
    }

    #[test]
    fn a_breaking_up_enemy_is_a_candidate_for_a_ship_with_destroying_weapons() {
        let mut dying = ship(2, ENEMY, 100.0, 0.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [me(), dying, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(
            chosen(0, &npcs, None, 0, NEAR),
            Some(n(3)),
            "no destroying weapons"
        );
        let mut armed = npcs.clone();
        armed[0].armament = destroying();
        assert_eq!(
            chosen(0, &armed, None, 0, NEAR),
            Some(n(2)),
            "then dropped by the hunt"
        );
    }

    #[test]
    fn the_hunt_never_takes_a_ship_it_spares() {
        // An enemy government's ship the ship leads.
        let mut escort = ship(2, ENEMY, 100.0, 0.0);
        escort.leader = Some(NpcId(1));
        let npcs = [me(), escort, ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
    }

    #[test]
    fn help_takes_a_breaking_up_or_downed_quarry_the_hunt_then_drops() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Attack(n(3));
        let mut pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        pirate.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [me(), trader, pirate];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        let mut downed = npcs.clone();
        downed[2].condition = Condition::Disabled;
        assert_eq!(chosen(0, &downed, None, 0, NEAR), Some(n(3)));
    }

    // Allies the player leads (`led_ally_attack`).

    /// Whether ship 1 of govt 140 drops `target` (NPC 2) by `led_ally`.
    fn drops_ally(npc: &Npc, target: &Npc, led_ally: RuleSource) -> bool {
        let govts = govts(0);
        let npcs = [npc.clone(), target.clone()];
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let rules = TargetRules {
            led_ally,
            ..TargetRules::default()
        };
        dropped(&npcs[0], n(2), &around, rules)
    }

    fn player_led(mut npc: Npc) -> Npc {
        npc.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        npc
    }

    #[test]
    fn by_default_a_ship_attacks_on_an_ally_the_player_leads_and_drops_a_stranger_ally() {
        let led = player_led(ship(2, ALLY, 100.0, 0.0));
        let stranger = ship(2, ALLY, 100.0, 0.0);
        assert!(!drops_ally(&me(), &led, RuleSource::Engine));
        assert!(drops_ally(&me(), &stranger, RuleSource::Engine));
        assert!(
            drops_ally(&me(), &led, RuleSource::Bible),
            "the other reading"
        );
        assert!(drops_ally(&me(), &stranger, RuleSource::Bible));
        assert!(
            !drops_ally(&player_led(me()), &stranger, RuleSource::Engine),
            "a ship the player leads spares no ally"
        );
        let enemy = ship(2, ENEMY, 100.0, 0.0);
        assert!(!drops_ally(&me(), &enemy, RuleSource::Engine));
    }

    #[test]
    fn by_default_a_ship_of_no_government_spares_no_ally() {
        let mut independent = me();
        independent.govt = None;
        let mut other = ship(2, ALLY, 100.0, 0.0);
        other.govt = None;
        assert!(!drops_ally(&independent, &other, RuleSource::Engine));
        assert!(drops_ally(&independent, &other, RuleSource::Bible));
        let mut kin = other.clone();
        kin.leader = Some(NpcId(1));
        assert!(
            drops_ally(&independent, &kin, RuleSource::Engine),
            "its own fleet still"
        );
    }

    #[test]
    fn by_default_help_comes_against_an_allied_ship_the_player_leads() {
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Attack(n(3));
        let escort = player_led(ship(3, ALLY, 3000.0, 0.0));
        let npcs = [me(), trader, escort];
        let govts = govts(0);
        let around = Surroundings {
            player: Some(player(0.0, -100.0)),
            govts: &govts,
            system_govt: Some(ME),
            ..Surroundings::new(&[], &npcs)
        };
        let by = |led_ally| {
            let rules = TargetRules {
                led_ally,
                ..TargetRules::default()
            };
            select_target(&npcs[0], None, &around, rules, &mut Draws::of(&[]))
        };
        assert_eq!(by(RuleSource::Engine), Some(n(3)));
        assert_eq!(by(RuleSource::Bible), None);
    }

    #[test]
    fn an_arriving_or_jumping_in_enemy_is_no_candidate() {
        let mut arriving = ship(2, ENEMY, 100.0, 0.0);
        arriving.mode = Mode::JumpingIn { tick: 0 };
        let npcs = [me(), arriving.clone(), ship(3, RIVAL, 900.0, 0.0)];
        assert_eq!(chosen(0, &npcs, None, 0, NEAR), Some(n(3)));
        let mut xeno = npcs.clone();
        xeno[0].govt = Some(XENO);
        assert_eq!(
            chosen(0, &xeno, None, 0, (0.0, -5000.0)),
            Some(n(3)),
            "nor a xenophobe's"
        );
    }

    #[test]
    fn the_xenophobe_steps_own_pick_decides_whether_it_turns_on_the_player() {
        // Step 7 takes the wreck nearest, for destroying weapons; the
        // call is step 5's, made when its own nearest is the player.
        let picked = |near: Npc| {
            let govts = govts(0);
            let mut xeno = ship(1, XENO, 0.0, 0.0);
            xeno.armament = destroying();
            let mut wreck = ship(2, RIVAL, 10.0, 0.0);
            wreck.condition = Condition::Disabled;
            let npcs = [xeno, wreck, near];
            let around = Surroundings {
                player: Some(player(0.0, -100.0)),
                govts: &govts,
                system_govt: Some(ME),
                ..Surroundings::new(&[], &npcs)
            };
            pick(&npcs[0], None, &around)
        };
        let wreck = |turns_on_player| {
            Some(Pick {
                ship: n(2),
                turns_on_player,
            })
        };
        assert_eq!(
            picked(ship(3, ENEMY, 0.0, 30.0)),
            wreck(false),
            "an enemy nearer than the player: no call"
        );
        let mut escort = ship(3, ENEMY, 0.0, 30.0);
        escort.leader = Some(NpcId(1));
        assert_eq!(picked(escort), wreck(true), "its own escort is none");
        let mut pod = ship(3, ENEMY, 0.0, 30.0);
        pod.ship = ESCAPE_POD_SHIP;
        assert_eq!(picked(pod), wreck(true), "class 0x2ff is none");
    }

    #[test]
    fn help_scales_the_ships_strength_by_its_max_odds() {
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                max_odds: 200,
                ..govt(140)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..govt(141)
            },
        ]);
        let mut trader = ship(2, ALLY, 1000.0, 0.0);
        trader.goal = Goal::Attack(n(3));
        trader.hull.strength = 0.0;
        let mut pirate = ship(3, NEUTRAL, 3000.0, 0.0);
        pirate.hull.strength = 150.0;
        let npcs = [me(), trader, pirate];
        let around = Surroundings {
            player: Some(player(0.0, -5000.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            select(&npcs[0], None, &around),
            Some(n(3)),
            "100 x 2 against 150"
        );
    }
}
