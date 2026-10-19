//! How a ship hit answers, and how others come to its help: Nova's
//! [`answer`], which the four AI types share.
//!
//! The rules are the original's (`_DamageShip` @0x3bb20-0x3bed2 and
//! `_DoGoodSamaritan` @0x3bef4 in the `EV Nova` executable). A strike
//! reaches them only once answered ([`Strike::answered`]): `_DamageShip`'s
//! answer flag, which holds back a ship's kin, its fleet and much else,
//! skips the whole block when clear (see
//! [`answer_flag`](crate::ai::answer_flag)).
//!
//! - **The victim**, and the lead of an escort hit, add the hit's nominal
//!   damage to their provocation (+0x96: each of its mass and energy args
//!   above none, [`Strike::nominal`], @0x3bcc0-0x3bcf0, @0x3bd25-0x3bd5d)
//!   and take the attacker as their target, unless the
//!   victim [keeps its target](keeps_target): attacking (goal 4) its
//!   target (+0x70), faced within [`FOCUS_DEGREES`] of the heading it
//!   last wanted (+0x68), with the attacker not nearer than half that
//!   target's distance (dist² above [`FOCUS`] × its target's,
//!   `_DamageShip` @0x3bbf3-0x3bcac). A pass's strikes are answered in
//!   turn (see `traffic::in_turn`), so the target is the one the hits
//!   before it in the pass left, which may be an attacker of an earlier
//!   hit, and +0x68 the heading wanted as the pass began. Then the
//!   hit changes nothing for it or its lead: the original jumps to the
//!   Good Samaritan call (0x3bed2), past +0x96 and the target, the lead's
//!   part (@0x3bcfa-0x3bd79), the turn on the player, the wait cut and
//!   the grudge. A ship hit by the very ship it attacks, facing it, so
//!   keeps at it quietly. The lead has no focus test of its own. Which
//!   test applies is
//!   [`RuleKey::HitFocus`](crate::RuleKey::HitFocus)'s
//!   ([`Surroundings::hit_focus`]): the other reading is the engine's
//!   before weapons-fidelity phase 37. The player's
//!   hit on the victim itself also turns it on the player at once
//!   ([`victim`], `_AIMakeShipAttackPlayer` @0x3be69), whatever it was
//!   doing. Its lead's goal the block never writes, so the lead goes on
//!   to the Good Samaritan test like any other ship (the lead's part
//!   @0x3bd1b-0x3bd79 falls to the call @0x3bed2-0x3bef4, whose goal gate
//!   is goal != 4, @0x828b3): a warship or interceptor lead not attacking
//!   whose gates pass attacks the player, any other takes the target
//!   alone. A ship still
//!   coming into the system answers so too, gliding in, waiting in its
//!   gate or bursting out of it: `_DamageShip` drops the reaction only
//!   while the victim's (or the lead's) hyperspace state +0x50 is above
//!   0 (@0x3b4b0-0x3b4c2, @0x3b97e-0x3b995, @0x3bab7-0x3bae4), which is a
//!   ship jumping out, and an arrival's never is: -1.0 waiting in a gate
//!   (@0x89555), -999.0 gliding in or bursting out (@0x82ece,
//!   @0x8f3c8-0x8f3d9), until `_AIMakeShipStopJumping` zeroes it
//!   (@0x82eb0). (Phase 6 read +0x50 as running through an arrival; it
//!   does not.) Which ships answer is the traffic's to say
//!   ([`RuleKey::ArrivalHits`](crate::RuleKey::ArrivalHits)). A victim
//!   with a wait that answers has it cut to 20 ticks (@0x3be6e-0x3be7c),
//!   which the traffic does
//!   ([`autopilot::HIT_WAIT`](crate::traffic::autopilot::HIT_WAIT)).
//!   A hit by no ship ([`Strike::by`] none, the original's source -1) is
//!   never tested for focus (`si` >= 0, @0x3bbf8-0x3bc03) and draws no
//!   target (`si` <= 63, @0x3bcf0, @0x3bd64), but still provokes the
//!   victim and its lead and cuts the victim's wait (`testw si; jne
//!   0x3be6e`, @0x3bd7e).
//! - **A Good Samaritan**: for a hit by the player on the ship it was
//!   meant for ([`Strike::intended`], @0x3bed2), every warship or
//!   interceptor W not already attacking takes the player as its target
//!   and attacks at once, unless W is an NPC enemy of the victim, or the
//!   victim's government is xenophobic and not W's; the victim's
//!   government is derelict or ignored when attacked (`Flags` 0x0020); or
//!   W is neither allied with the victim nor nosy (`Flags` 0x0002). For
//!   an independent W, only the derelict, 0x0020 and xenophobe tests
//!   apply; for an independent victim, only a nosy W answers. A W whose
//!   government blesses the player with peace (rank `Flags` 0x0100,
//!   [`Surroundings::blessed`]: `_PlayerBlessedByGovt(W's govt, 0)`
//!   @0x829bf) never answers, under either reading of `rank_govt_reach`
//!   and `blessing_refresh`; but the original reads the blessing only for
//!   a governed victim (an independent one jumps past it, @0x82919 to
//!   0x82aa0), so a nosy blessing W still answers an attack on an
//!   independent. (The attacker is always the player here: both callers
//!   pass it, so the original's clause for a ship the player leads,
//!   @0x829a0-0x829b2, never arises.) There is no limit on range. The original calls the same `_DoGoodSamaritan`
//!   when the player boards a ship, after the crime
//!   (`_HandlePlayerBoardAttempt`), so the session queues a strike of no
//!   damage for a boarding (`nova_sim::Session::board`), intended but not
//!   direct, as it never passes `_DamageShip`: it draws no victim's or
//!   lead's reaction, only the Samaritans', the ship boarded and its lead
//!   among them, as the loop @0x82891-0x82ba8 leaves out no slot. The lead
//!   of an escort the player hits is one of those slots too, after its
//!   own reaction. The blessing holds it back alike.
//!   The Bible's "piracy police" are the interceptors alone: "acts as
//!   'piracy police' by attacking any ship that fires on or attempts to
//!   board another, non-enemy ship" (`düde` AI type 4). Which applies is
//!   the [`Rulebook`](crate::Rulebook)'s
//!   [`RuleKey::PiracyPolice`](crate::RuleKey::PiracyPolice) entry,
//!   passed to [`answer`]: by the engine, warships and interceptors
//!   answer; by the Bible, interceptors only.
//!
//! So Federation police answer the player's attack on a Civvies trader,
//! since the Civvies are Federation allies. The original's distress
//! message (`_AICallForHelp`) only tells the player (see
//! [`call_for_help`](super::call_for_help)); the system's reinforcement
//! fleet comes by its own call (`_AICallForReinforcements`, see
//! [`reinforcements`](super::reinforcements)), not by a hit.
//!
//! Left out of the reaction block (weapons-fidelity phase 37 records each):
//!
//! - The defence-ship branches, for a ship defending a stellar (+0x8c),
//!   which the engine doesn't have: the victim's (@0x3bbe2 → 0x3bd8c: it
//!   reacts only when 0.5 × dist² to its stellar exceeds dist² to the
//!   attacker, @0x3be17, and adds 30 × each nominal arg, @0x3be2b-0x3be56)
//!   and the lead's and the grudge's +0x8c tests (@0x3bd1b, @0x3be88).
//!   They belong to rdm task planet-domination.
//! - The jump-out stage (+0x50 above 0): the entry gate (@0x3b46a), the
//!   arriving test (@0x3b97e) and a victim whose leader jumps out
//!   (@0x3bab7-0x3bae4), for rdm phase
//!   weapons-fidelity-cross-roadmap/phase-1-damage-ship-jump-out-stage.
//! - The test of the victim's slot +0x86 (@0x3bcb2), which a player victim
//!   would fail but never reaches: the player is AI type -1 (@0x3b459).
//! - `_AIMakeShipStopRefuelling` and target 0 for a player's hit
//!   (@0x3beba-0x3becc), after the focus test, which rdm phase
//!   combat-ai-fidelity/phase-26-assister-hit owns.
//!
//! Left out of `_DoGoodSamaritan`'s reach (weapons-fidelity phase 38
//! records each):
//!
//! - Disabled ships. The loop tests no slot's condition, so a disabled
//!   warship or interceptor not attacking takes goal 4 and the player
//!   (@0x82b82, @0x82b97), but `_AIDispatch`'s disabled reset
//!   (@0x8fece-0x8ff13) wipes both at its next think, nearly always in the
//!   same frame. The engine has no such reset yet, so a disabled NPC asks
//!   for nothing here (`Traffic::react` takes intact NPCs alone): writing
//!   the goal now would carry it through self-repair. rdm phase
//!   weapons-fidelity-cross-roadmap/phase-2-disabled-ships-take-a-good-samaritans-write
//!   owns it, behind escort-fleet-fidelity's disabled reset.
//! - Slots not in use. `_IsShipMyEnemy` answers "not an enemy" for an
//!   inactive slot (@0x81a16), so a stale one may take the Samaritan's
//!   write, but no code reads an inactive slot (@0x44275) and every spawn
//!   rewrites goal and target (@0x3c3a3, @0x3c43b, @0x3c969, @0x3c9d9,
//!   @0x3d7a3, @0x3dbf7). Record only: the engine keeps no free slots.

use crate::ai::{Goal, Reaction, Surroundings};
use crate::combat::aim::{angle_off, bearing};
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::govt::{DERELICT, IGNORED_WHEN_ATTACKED, NOSY};
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Npc};
use crate::trig;

/// The share of the attacker's distance squared within which a ship
/// keeps attacking its own target (0.25 @0x3bc3b).
pub const FOCUS: f32 = 0.25;
/// How far off its nose, in degrees, a ship keeping its own target must
/// have it.
pub const FOCUS_ANGLE: f32 = 44.0;
/// How far, in whole degrees, the heading a ship struck faces may be off
/// the one it last wanted for it to keep its target (0x2c, `jg`
/// @0x3bc3f).
pub const FOCUS_DEGREES: i16 = 44;

/// How `npc` answers `strike` among `around`, the Good Samaritans being
/// those `piracy_police` says (see the module docs): as the
/// [`victim`] for a direct hit on itself or its escort, then as a Good
/// Samaritan, as `_DamageShip`'s reaction block falls to the call
/// `_DoGoodSamaritan(victim, player)` (@0x3bed2-0x3bef4) and that loop
/// excludes no slot (@0x82891-0x82ba8). A Samaritan takes the player as
/// its target and attacks it, keeping the provocation its victim's
/// reaction gave it.
///
/// So the lead of an escort the player hits, a warship or interceptor
/// not attacking, takes the target and the provocation (@0x3bd1b-0x3bd79)
/// and goes on to attack the player as a Samaritan; under the focus skip
/// (@0x3bcac, straight to 0x3bed2) it is a Samaritan unprovoked. A strike
/// not direct, the boarding's call, draws no victim's reaction, so the
/// ship boarded and its lead answer it as Samaritans or not at all.
///
/// The Samaritan test reads `npc`'s goal from before its reaction, where
/// the original reads it after the block. The two agree: the block never
/// writes a lead's goal, and the victim of the player's hit either keeps
/// its target, attacking, so fails the test, or turns on the player, which
/// is what passing the test gives too.
#[must_use]
pub fn answer(
    npc: &Npc,
    strike: &Strike,
    around: &Surroundings,
    piracy_police: RuleSource,
) -> Reaction {
    let reacted = victim(npc, strike, around);
    let polices = match piracy_police {
        RuleSource::Engine => true,
        RuleSource::Bible => npc.ai_type == AiType::Interceptor,
    };
    let samaritans = strike.by == Some(ShipRef::Player) && strike.intended;
    if samaritans && polices && good_samaritan(npc, strike.ship, around) {
        return Reaction {
            target: Some(ShipRef::Player),
            goal: Some(Goal::Attack(ShipRef::Player)),
            ..reacted
        };
    }
    reacted
}

/// How `npc` answers `strike` among `around` as its victim, or as the
/// lead of the escort it hit, and nothing more: `_DamageShip`'s reaction
/// block for every AI type (@0x3bb7a-0x3bed2 reads none), and all of it
/// for the AI types 5 and 6, the player's escorts and fighters and an NPC
/// carrier's, which are never Good Samaritans (`_DoGoodSamaritan` takes
/// AI 3 and 4 alone, @0x8289f-0x828bb).
///
/// Unless the ship struck [keeps its target](keeps_target), it takes the
/// attacker as its target and the nominal damage as provocation; a hit by
/// no ship provokes it with no target and no focus test; a strike not
/// direct ([`Strike::direct`]) does nothing: the
/// boarding's call, which never passes `_DamageShip`. The player's hit on
/// the ship itself also turns it on the
/// player at once (`Goal::Attack(Player)`): `_AIMakeShipAttackPlayer`
/// (@0x3be66-0x3be69) sets goal 4 on the victim, never on its lead
/// (@0x89d3e), whatever goal it had, a ship assisting the player
/// included (`_AIMakeShipStopRefuelling` after it, @0x3bec7, then finds
/// goal 4 and does nothing). Its travel stellar cleared (+0x6c -1,
/// @0x89d47) the traffic writes ([`Npc::travel_set`]). Its one other
/// change, clearing a jump's sub-goal (4 or 0xd under goals 2, 3 and 0xb,
/// @0x89cf2-0x89d35), the new goal makes here, as the engine has no jump
/// stage; any other sub-goal it keeps, so a ship already attacking keeps
/// its pursuit ([`Npc::pursuit`]).
#[must_use]
pub fn victim(npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
    if !strike.direct || !hits(npc, strike, around) {
        return Reaction::default();
    }
    let provoked = provocation(strike);
    // A hit by no ship is never tested for focus (`si` >= 0,
    // @0x3bbf8-0x3bc03) and draws no target (`si` <= 63, @0x3bcf0,
    // @0x3bd64), but still provokes.
    let Some(by) = strike.by else {
        return Reaction {
            target: None,
            provoked,
            goal: None,
        };
    };
    // The ship struck: `npc` itself, or the escort it leads, both among
    // `around` (`hits` checked it).
    let hit = match strike.ship {
        ShipRef::Npc(id) => around.npc(id).unwrap_or(npc),
        ShipRef::Player => npc,
    };
    if keeps_target(npc, hit, by, around) {
        return Reaction::default();
    }
    let on_itself = strike.ship == ShipRef::Npc(npc.id);
    Reaction {
        target: Some(by),
        provoked,
        goal: (on_itself && by == ShipRef::Player).then_some(Goal::Attack(ShipRef::Player)),
    }
}

/// What `strike` adds to the provocation (+0x96) of its victim and of
/// the victim's lead: each of its nominal mass and energy damage
/// ([`Strike::nominal`], `_DamageShip`'s args +0x14 and +0x18) when above
/// none (@0x3bcc0-0x3bcf0, @0x3bd25-0x3bd5d), not the damage taken. A
/// missile lock's nominal damage is none, so it adds nothing. The args
/// are 16-bit words in the original; this keeps `f32`.
fn provocation(strike: &Strike) -> f32 {
    let positive = |damage: f32| damage.max(0.0);
    positive(strike.nominal.mass) + positive(strike.nominal.energy)
}

/// Whether `strike` hit `npc` itself, or an escort it leads, among
/// `around`.
fn hits(npc: &Npc, strike: &Strike, around: &Surroundings) -> bool {
    match strike.ship {
        ShipRef::Npc(id) => {
            id == npc.id || around.npc(id).is_some_and(|hit| hit.leader == Some(npc.id))
        }
        ShipRef::Player => false,
    }
}

/// Whether `answering`, struck itself (`hit` is `answering`) or the lead
/// of `hit`, the escort struck, keeps its target against `attacker` among
/// `around`, so the hit changes nothing for it (see the module docs), by
/// [`Surroundings::hit_focus`]
/// ([`RuleKey::HitFocus`](crate::RuleKey::HitFocus)). Kin is not read:
/// the answer flag held it back already
/// ([`answer_flag`](crate::ai::answer_flag)). Its mode is not read:
/// `_DamageShip` skips a reaction only while +0x50 is above 0
/// (@0x3b4b0-0x3b4c2), a ship jumping out, never one coming in.
///
/// - By the engine (`_DamageShip` @0x3bbf3-0x3bcac), `hit` is tested,
///   whichever ship answers: it keeps its target when it is attacking
///   ([`Goal::attacking`] some, `_IsShipAttacking` @0x82f6a, which tests
///   the goal word alone), its target T ([`Npc::target`], +0x70, not the
///   goal's quarry) is live, it faces within [`FOCUS_DEGREES`] of the
///   heading it last wanted (`_AngularDifference(trunc(+0x44), +0x68)`),
///   and dist²(`attacker`) > [`FOCUS`] × dist²(T). The two part when an
///   earlier hit of the pass set the target and left the goal. +0x68 is
///   the last body's heading of its frame for its goal
///   ([`Pursuit::frame`](crate::ai::pursuit::Pursuit::frame)), which at
///   the collision passes is the previous tick's, as in the original;
///   while a pass is answered it is that heading as the pass began
///   ([`Surroundings::wanted`]), as no reaction writes +0x68; with no such
///   frame, the whole-degree bearing to T stands in. The original reads
///   T's slot whatever is in it; a dead or departed T is not kept here.
/// - By the other reading, the engine's before weapons-fidelity phase 37,
///   `answering` tests itself: it keeps its target when T is nearer than
///   half the attacker's distance (dist²(T) < [`FOCUS`] × dist²(attacker))
///   and within [`FOCUS_ANGLE`] of its float heading by bearing.
#[must_use]
pub fn keeps_target(answering: &Npc, hit: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    match around.hit_focus {
        RuleSource::Engine => focused(hit, attacker, around),
        RuleSource::Bible => ignores(answering, attacker, around),
    }
}

/// The distance squared from `npc` to `ship` among `around`, and where
/// `ship` is; none for a ship not there.
fn squared(npc: &Npc, ship: ShipRef, around: &Surroundings) -> Option<(f32, Vec2)> {
    around.state_of(ship).map(|state| {
        let off = state.position - npc.state.position;
        (off.x * off.x + off.y * off.y, state.position)
    })
}

/// The original's focus test of `hit`, the ship struck by `attacker`
/// (see [`keeps_target`]).
fn focused(hit: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    let Some(own) = hit
        .goal
        .attacking()
        .and(hit.target)
        .filter(|&own| around.live(own))
    else {
        return false;
    };
    let (Some((target, at)), Some((by, _))) =
        (squared(hit, own, around), squared(hit, attacker, around))
    else {
        return false;
    };
    let wanted = match around.wanted {
        Some(wanted) => wanted.get(&hit.id).copied(),
        None => hit.wanted_heading(),
    }
    .unwrap_or_else(|| trig::intercept_angle(hit.state.position, at));
    trig::angular_difference(trig::whole(hit.state.heading), wanted) <= FOCUS_DEGREES
        && by > FOCUS * target
}

/// The other reading's focus test of `npc` itself (see [`keeps_target`]).
fn ignores(npc: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    let Some(own) = npc.goal.attacking().filter(|&own| around.live(own)) else {
        return false;
    };
    match (squared(npc, own, around), squared(npc, attacker, around)) {
        (Some((near, at)), Some((far, _))) => {
            near < FOCUS * far
                && angle_off(npc.state.heading, bearing(npc.state.position, at)) <= FOCUS_ANGLE
        }
        _ => false,
    }
}

/// Whether `npc` answers the player's attack on `victim` among `around`
/// (see the module docs). The victim itself is not left out, as the loop
/// @0x82891-0x82ba8 leaves out no slot. A direct hit's victim reaches
/// here after its reaction: either it kept its target, so its goal is
/// attacking and the test fails, or it passes and the merge in [`answer`]
/// gives exactly what its reaction did, the player as target and as the
/// goal's quarry. Either way the answer is the original's, where the
/// victim's goal is already 4 by the call and it never passes.
fn good_samaritan(npc: &Npc, victim: ShipRef, around: &Surroundings) -> bool {
    let ShipRef::Npc(victim_id) = victim else {
        return false;
    };
    if npc.ai_type.trades() || npc.goal.attacking().is_some() || around.npc(victim_id).is_none() {
        return false;
    }
    let govts = around.govts;
    let theirs = around.govt_of(victim);
    let own = npc.govt;
    let nosy = govts.flag(own, NOSY);
    if theirs.is_none() {
        return nosy;
    }
    if around.blessed(own) {
        return false;
    }
    if govts.flag(theirs, DERELICT) || govts.flag(theirs, IGNORED_WHEN_ATTACKED) {
        return false;
    }
    if govts.xenophobic(theirs) && theirs != own {
        return false;
    }
    own.is_none() || (!govts.npc_enemies(own, theirs) && (govts.allies(own, theirs) || nosy))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, GovtRecord};
    use crate::geometry::Vec2;
    use crate::govt::{DERELICT, Governments, IGNORED_WHEN_ATTACKED, NOSY, XENOPHOBIC};
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::npc::{AiType, Mode, NpcId};
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::govt;

    const TRADERS: GovtId = GovtId(140);
    const POLICE: GovtId = GovtId(141);
    const PIRATES: GovtId = GovtId(142);
    const NEUTRAL: GovtId = GovtId(143);
    const XENO: GovtId = GovtId(144);

    /// Traders (class 1), police allied with them, pirates at war with
    /// them, a neutral and a xenophobe, with `flags` on the traders and
    /// `nosy` on the neutral.
    fn relations(flags: u16, nosy: u16) -> Governments {
        Governments::new([
            GovtRecord {
                flags,
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
                flags: nosy,
                ..govt(143)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                ..govt(144)
            },
        ])
    }

    /// NPC `id` of `govt` and `ai_type` at (`x`, 0), facing up.
    fn ship(id: u32, govt: Option<GovtId>, ai_type: AiType, x: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]));
        npc.govt = govt;
        npc.ai_type = ai_type;
        npc.state.position = Vec2::new(x, 0.0);
        npc
    }

    fn n(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    use crate::combat::Nominal;
    use crate::combat::fixture::strike;

    fn answered(govts: &Governments, npcs: &[Npc], who: usize, hit: &Strike) -> Reaction {
        answered_by(RuleSource::Engine, govts, npcs, who, hit)
    }

    fn answered_by(
        police: RuleSource,
        govts: &Governments,
        npcs: &[Npc],
        who: usize,
        hit: &Strike,
    ) -> Reaction {
        let around = Surroundings {
            govts,
            ..Surroundings::new(&[], npcs)
        };
        answer(&npcs[who], hit, &around, police)
    }

    /// [`answered`] with the focus test read by `focus`
    /// ([`Surroundings::hit_focus`]).
    fn focused_by(
        focus: RuleSource,
        govts: &Governments,
        npcs: &[Npc],
        who: usize,
        hit: &Strike,
    ) -> Reaction {
        focused_around(focus, govts, npcs, None, who, hit)
    }

    /// [`focused_by`] with `player` in the system.
    fn focused_around(
        focus: RuleSource,
        govts: &Governments,
        npcs: &[Npc],
        player: Option<&crate::ai::PlayerSide>,
        who: usize,
        hit: &Strike,
    ) -> Reaction {
        let around = Surroundings {
            govts,
            hit_focus: focus,
            player: player.copied(),
            ..Surroundings::new(&[], npcs)
        };
        answer(&npcs[who], hit, &around, RuleSource::Engine)
    }

    /// The victim, or its lead, taking its attacker, provoked by the
    /// fixture's nominal 12 mass and 12 energy damage.
    const TAKES: fn(ShipRef) -> Reaction = |attacker| Reaction {
        target: Some(attacker),
        provoked: 24.0,
        goal: None,
    };

    /// The victim of the player's hit: it takes the player as its target
    /// and attacks it at once (`_AIMakeShipAttackPlayer` @0x89d3e).
    const TURNS: Reaction = Reaction {
        target: Some(ShipRef::Player),
        provoked: 24.0,
        goal: Some(Goal::Attack(ShipRef::Player)),
    };

    #[test]
    fn the_players_hit_turns_its_victim_on_the_player_but_not_the_victims_lead() {
        // `_DamageShip` calls `_AIMakeShipAttackPlayer` on the victim
        // (`edi`) alone, once si, the attacker, is the player
        // (@0x3bd7e-0x3be69); the lead takes the target only (@0x3bd79),
        // and this one, a trader, is no Samaritan to attack it after.
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0), escort];
        let on_lead = strike(n(1), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 0, &on_lead), TURNS);
        let on_escort = strike(n(2), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 1, &on_escort), TURNS, "the escort");
        assert_eq!(
            answered(&govts, &npcs, 0, &on_escort),
            TAKES(ShipRef::Player),
            "its lead"
        );
        assert_eq!(
            answered(&govts, &npcs, 0, &strike(n(1), n(2))),
            TAKES(n(2)),
            "an NPC's hit gives no goal"
        );
    }

    #[test]
    fn a_ship_keeping_its_own_target_does_not_turn_on_the_player() {
        // The focus skip jumps past `_AIMakeShipAttackPlayer` to 0x3bed2
        // (@0x3bcac): the player 6 right keeps it (36 > 0.25 × 100), 4
        // right does not (16).
        let govts = relations(0, 0);
        let mut hunter = ship(1, Some(POLICE), AiType::Warship, 0.0);
        hunter.goal = Goal::Attack(n(2));
        hunter.target = Some(n(2));
        let mut quarry = ship(2, Some(PIRATES), AiType::Warship, 0.0);
        quarry.state.position = Vec2::new(0.0, -10.0);
        let npcs = [hunter, quarry];
        let hit = strike(n(1), ShipRef::Player);
        let with_player_at = |x: f32| {
            let around = Surroundings {
                govts: &govts,
                player: Some(crate::ai::fixture::player(x, 0.0)),
                ..Surroundings::new(&[], &npcs)
            };
            victim(&npcs[0], &hit, &around)
        };
        assert_eq!(with_player_at(6.0), Reaction::default());
        assert_eq!(with_player_at(100.0), Reaction::default());
        assert_eq!(with_player_at(4.0), TURNS, "the player too near");
    }

    #[test]
    fn by_the_other_reading_a_ship_keeping_its_own_nearer_target_does_not_turn_on_the_player() {
        let govts = relations(0, 0);
        let mut hunter = ship(1, Some(POLICE), AiType::Warship, 0.0);
        hunter.goal = Goal::Attack(n(2));
        hunter.target = Some(n(2));
        let mut quarry = ship(2, Some(PIRATES), AiType::Warship, 0.0);
        quarry.state.position = Vec2::new(0.0, -10.0);
        let npcs = [hunter, quarry];
        let around = Surroundings {
            govts: &govts,
            player: Some(crate::ai::fixture::player(100.0, 0.0)),
            hit_focus: RuleSource::Bible,
            ..Surroundings::new(&[], &npcs)
        };
        let hit = strike(n(1), ShipRef::Player);
        assert_eq!(victim(&npcs[0], &hit, &around), Reaction::default());
        let mut far = npcs.clone();
        far[1].state.position = Vec2::new(0.0, -60.0);
        let around = Surroundings {
            npcs: &far,
            ..around
        };
        assert_eq!(victim(&far[0], &hit, &around), TURNS, "its target too far");
    }

    #[test]
    fn victim_answers_a_hit_on_the_ship_or_its_escort_and_is_no_good_samaritan() {
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            escort,
            ship(3, Some(POLICE), AiType::Warship, 3000.0),
        ];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let by_pirate = strike(n(1), n(9));
        assert_eq!(victim(&npcs[0], &by_pirate, &around), TAKES(n(9)));
        assert_eq!(
            victim(&npcs[0], &strike(n(2), n(9)), &around),
            TAKES(n(9)),
            "its escort hit"
        );
        let by_player = strike(n(1), ShipRef::Player);
        assert_eq!(victim(&npcs[0], &by_player, &around), TURNS);
        assert_eq!(
            answer(&npcs[2], &by_player, &around, RuleSource::Engine),
            ANSWERS,
            "a Samaritan by `answer`"
        );
        assert_eq!(
            victim(&npcs[2], &by_player, &around),
            Reaction::default(),
            "none by `victim`"
        );
    }

    #[test]
    fn the_victim_takes_its_attacker_and_adds_the_damage() {
        let govts = relations(0, 0);
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(PIRATES), AiType::Warship, 500.0),
        ];
        let hit = strike(n(1), n(2));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(2)));
        assert_eq!(
            answered(&govts, &npcs, 0, &strike(n(1), ShipRef::Player)),
            TURNS
        );
        assert_eq!(
            answered(&govts, &npcs, 1, &hit),
            Reaction::default(),
            "the attacker"
        );
    }

    #[test]
    fn a_hit_by_no_ship_provokes_the_victim_and_its_lead_but_gives_no_target() {
        // +0x96 takes the nominal damage whatever the source
        // (@0x3bcb2-0x3bcf0, the lead @0x3bd25-0x3bd5d); only the target
        // waits for a ship source (`si` <= 63, @0x3bcf0, @0x3bd64), and
        // the focus test asks for one (`si` >= 0, @0x3bbf8-0x3bc03).
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0), escort];
        let provoked = Reaction {
            target: None,
            provoked: 24.0,
            goal: None,
        };
        let by_none = |hit| Strike {
            by: None,
            ..strike(hit, n(1))
        };
        assert_eq!(answered(&govts, &npcs, 0, &by_none(n(1))), provoked);
        assert_eq!(
            answered(&govts, &npcs, 1, &by_none(n(1))),
            Reaction::default(),
            "its escort, not hit"
        );
        assert_eq!(
            answered(&govts, &npcs, 1, &by_none(n(2))),
            provoked,
            "the escort hit"
        );
        assert_eq!(
            answered(&govts, &npcs, 0, &by_none(n(2))),
            provoked,
            "its lead"
        );
        let hunted = hunted(100.0);
        for focus in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                focused_by(focus, &govts, &hunted, 0, &by_none(n(1))),
                provoked,
                "{focus:?}: no focus test for a hit by no ship"
            );
        }
    }

    #[test]
    fn the_victim_and_its_lead_add_each_nominal_damage_above_none() {
        // `_DamageShip` adds the mass and energy args (+0x14, +0x18) to
        // +0x96 each when above 0 (@0x3bcc0-0x3bcf0, @0x3bd25-0x3bd5d),
        // not the damage taken.
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0), escort];
        let provoked = |hit: ShipRef, who: usize, nominal: Nominal, lock: bool| {
            let struck = Strike {
                damage: 3.0,
                nominal,
                lock,
                ..strike(hit, n(9))
            };
            answered(&govts, &npcs, who, &struck).provoked
        };
        let nominal = |mass, energy| Nominal { mass, energy };
        for (hit, who) in [(n(1), 0), (n(2), 1), (n(2), 0)] {
            assert_eq!(provoked(hit, who, nominal(4.0, 10.0), false), 14.0);
            assert_eq!(provoked(hit, who, nominal(-5.0, 10.0), false), 10.0);
            assert_eq!(provoked(hit, who, nominal(4.0, -10.0), false), 4.0);
            assert_eq!(provoked(hit, who, nominal(0.0, 0.0), true), 0.0, "a lock");
        }
    }

    #[test]
    fn the_lead_of_an_escort_hit_takes_its_attacker_too() {
        let govts = relations(0, 0);
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            escort,
            ship(3, Some(PIRATES), AiType::Warship, 500.0),
        ];
        let hit = strike(n(2), n(3));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(3)));
        assert_eq!(answered(&govts, &npcs, 1, &hit), TAKES(n(3)));
    }

    #[test]
    fn an_answered_hit_by_its_own_government_or_fleet_is_taken() {
        // Kin is the answer flag's (`ai::answer_flag`, @0x3b4cd-0x3b4fc):
        // a hit that reaches the reaction was answered, and the reaction
        // block, the lead's part included (@0x3bcfa-0x3bd79), has no kin
        // test of its own.
        let govts = relations(0, 0);
        let mut npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(TRADERS), AiType::Warship, 500.0),
            ship(3, Some(TRADERS), AiType::Warship, 50.0),
        ];
        npcs[2].leader = Some(NpcId(1));
        let hit = strike(n(1), n(2));
        assert_eq!(answered(&govts, &npcs, 0, &hit), TAKES(n(2)));
        assert_eq!(
            answered(&govts, &npcs, 0, &strike(n(3), n(2))),
            TAKES(n(2)),
            "its escort hit by its government"
        );
        npcs[1].govt = Some(NEUTRAL);
        npcs[1].leader = Some(NpcId(1));
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            TAKES(n(2)),
            "by its escort"
        );
        npcs[1].leader = None;
        npcs[0].govt = None;
        npcs[1].govt = None;
        assert_eq!(
            answered(&govts, &npcs, 0, &hit),
            TAKES(n(2)),
            "two independents"
        );
    }

    #[test]
    fn a_ship_gliding_in_waiting_in_its_gate_or_bursting_out_answers_a_hit_as_a_flying_one() {
        // `_DamageShip` drops a reaction only while +0x50 is above 0, a
        // ship jumping out (@0x3b4b0-0x3b4c2, @0x3b97e-0x3b995,
        // @0x3bab7-0x3bae4); an arrival's +0x50 is -1.0 in its gate
        // (@0x89555) and -999.0 gliding or bursting (@0x82ece, @0x8f3c8).
        let govts = relations(0, 0);
        for mode in [
            Mode::JumpingIn { tick: 0 },
            Mode::Emerging { ticks_left: 4 },
            Mode::HeldAtGate { ticks_left: 4 },
            Mode::LeavingGate { tick: 1 },
            Mode::Coasting { ticks_left: 30 },
        ] {
            let mut npcs = [
                ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
                ship(2, Some(PIRATES), AiType::Warship, 500.0),
                ship(3, Some(TRADERS), AiType::WimpyTrader, 900.0),
            ];
            npcs[0].mode = mode;
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(1), n(2))),
                TAKES(n(2)),
                "{mode:?}"
            );
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(1), ShipRef::Player)),
                TURNS,
                "{mode:?}"
            );
            npcs[2].leader = Some(NpcId(1));
            assert_eq!(
                answered(&govts, &npcs, 0, &strike(n(3), n(2))),
                TAKES(n(2)),
                "{mode:?}: its escort hit"
            );
        }
    }

    /// A hunter (1) facing up and attacking a quarry (2) 50 above it, hit
    /// by a pirate (3) `x` to its right.
    fn hunted(x: f32) -> [Npc; 3] {
        let mut hunter = ship(1, Some(POLICE), AiType::Warship, 0.0);
        hunter.goal = Goal::Attack(n(2));
        hunter.target = Some(n(2));
        let mut quarry = ship(2, Some(PIRATES), AiType::Warship, 0.0);
        quarry.state.position = Vec2::new(0.0, -50.0);
        [hunter, quarry, ship(3, Some(PIRATES), AiType::Warship, x)]
    }

    #[test]
    fn a_ship_attacking_keeps_its_target_unless_the_attacker_is_nearer_than_half_its_distance() {
        // `_DamageShip` keeps the target when dist²(attacker) > 0.25 ×
        // dist²(target) (`ucomiss; ja 0x3bed2`, @0x3bc41-0x3bcac): with the
        // target at 50 (0.25 × 2500 = 625), the attacker must come in to 25.
        let govts = relations(0, 0);
        let hit = strike(n(1), n(3));
        let engine = |npcs: &[Npc]| focused_by(RuleSource::Engine, &govts, npcs, 0, &hit);
        assert_eq!(engine(&hunted(100.0)), Reaction::default(), "far off");
        assert_eq!(engine(&hunted(25.1)), Reaction::default(), "kept");
        assert_eq!(engine(&hunted(25.0)), TAKES(n(3)), "at the edge");
        assert_eq!(engine(&hunted(24.9)), TAKES(n(3)), "reacts");
        assert_eq!(
            answered(&govts, &hunted(100.0), 0, &hit),
            Reaction::default()
        );
        assert_eq!(
            focused_by(RuleSource::Bible, &govts, &hunted(100.0), 0, &hit),
            TAKES(n(3)),
            "by the other reading its target is not near enough"
        );
        assert_eq!(
            focused_by(RuleSource::Bible, &govts, &hunted(24.9), 0, &hit),
            TAKES(n(3))
        );
        let mut gone = hunted(100.0);
        gone[0].goal = Goal::Attack(n(9));
        gone[0].target = Some(n(9));
        assert_eq!(engine(&gone), TAKES(n(3)), "its target gone");
        let mut shifted = hunted(100.0);
        for npc in &mut shifted {
            npc.state.position = npc.state.position + Vec2::new(300.0, -200.0);
        }
        assert_eq!(engine(&shifted), Reaction::default(), "measured from it");
        let mut near = hunted(24.9);
        for npc in &mut near {
            npc.state.position = npc.state.position + Vec2::new(300.0, -200.0);
        }
        assert_eq!(engine(&near), TAKES(n(3)), "measured from it, near");
    }

    #[test]
    fn the_focus_measures_against_the_target_a_hit_set_not_the_goals_quarry() {
        // T is the ship in +0x70 (@0x3bbf3), not a quarry stored in the
        // goal: `_IsShipAttacking` tests the goal word alone (@0x82f6a).
        // A hit that set +0x70 to A (@0x3bcf6) leaves goal 4, so the next
        // hit measures against A. Here the goal's quarry (2) is 50 above,
        // the target A (4) 10 above, the attacker (3) 20 right: 400 is
        // above 0.25 × 100 but not above 0.25 × 2500.
        let govts = relations(0, 0);
        let mut npcs = hunted(20.0).to_vec();
        let mut near = ship(4, Some(PIRATES), AiType::Warship, 0.0);
        near.state.position = Vec2::new(0.0, -10.0);
        npcs.push(near);
        npcs[0].target = Some(n(4));
        let hit = strike(n(1), n(3));
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit),
            Reaction::default(),
            "kept against its target"
        );
        npcs[0].target = None;
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit),
            TAKES(n(3)),
            "no target, no keep"
        );
    }

    #[test]
    fn a_ship_hit_by_the_ship_it_attacks_keeps_at_it_quietly() {
        // A == T, so A > 0.25 T: no +0x96, no target, no wait cut.
        let govts = relations(0, 0);
        let mut npcs = hunted(0.0);
        npcs[0].goal = Goal::Attack(n(3));
        npcs[0].target = Some(n(3));
        npcs[2].state.position = Vec2::new(0.0, -50.0);
        let hit = strike(n(1), n(3));
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit),
            Reaction::default()
        );
        assert_eq!(
            focused_by(RuleSource::Bible, &govts, &npcs, 0, &hit),
            TAKES(n(3))
        );
    }

    #[test]
    fn the_lead_of_an_escort_hit_follows_the_escorts_focus() {
        // The lead's part (@0x3bcfa-0x3bd79) has no focus test of its own:
        // the victim's test gates it.
        let govts = relations(0, 0);
        // An idle lead (1); its escort (2) 50 right attacks a quarry (4)
        // 50 above itself; the attacker (3) 100 right of the escort.
        let mut escort = ship(2, Some(POLICE), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        escort.goal = Goal::Attack(n(4));
        escort.target = Some(n(4));
        let mut quarry = ship(4, Some(PIRATES), AiType::Warship, 50.0);
        quarry.state.position = Vec2::new(50.0, -50.0);
        let npcs = [
            ship(1, Some(POLICE), AiType::Warship, 0.0),
            escort,
            ship(3, Some(PIRATES), AiType::Warship, 150.0),
            quarry,
        ];
        let hit = strike(n(2), n(3));
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit),
            Reaction::default(),
            "the lead"
        );
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &npcs, 1, &hit),
            Reaction::default(),
            "the escort"
        );
        assert_eq!(
            focused_by(RuleSource::Bible, &govts, &npcs, 0, &hit),
            TAKES(n(3)),
            "by the other reading the idle lead tests itself"
        );
        // The lead attacking a quarry 10 above itself; its escort idle.
        let mut turned = npcs.clone();
        turned[0].goal = Goal::Attack(n(4));
        turned[0].target = Some(n(4));
        turned[1].goal = Goal::Idle;
        turned[3].state.position = Vec2::new(0.0, -10.0);
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &turned, 0, &hit),
            TAKES(n(3)),
            "the escort keeps nothing"
        );
        assert_eq!(
            focused_by(RuleSource::Bible, &govts, &turned, 0, &hit),
            Reaction::default(),
            "by the other reading the lead keeps its own"
        );
    }

    /// A frame of `goal` whose last body wants `heading`.
    fn wanting(goal: Goal, heading: i16) -> crate::ai::pursuit::Frame {
        use crate::ai::fire::Manoeuvre;
        use crate::ai::pursuit::{Body, Frame};
        Frame {
            goal,
            bodies: [
                Some(Body {
                    manoeuvre: Manoeuvre::Approach,
                    heading: 180,
                }),
                Some(Body {
                    manoeuvre: Manoeuvre::Dogfight,
                    heading,
                }),
            ],
            speed_cap: None,
        }
    }

    #[test]
    fn the_facing_is_read_against_the_heading_its_last_frame_wanted() {
        // +0x68, the heading the low level last wanted (@0x3bc25-0x3bc3f),
        // is the frame's last body's; with no frame for its goal, the
        // bearing to its target stands in.
        let govts = relations(0, 0);
        let hit = strike(n(1), n(3));
        let with_frame = |frame| {
            let mut npcs = hunted(100.0);
            npcs[0].pursuit.frame = Some(frame);
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit)
        };
        let attacking = Goal::Attack(n(2));
        assert_eq!(with_frame(wanting(attacking, 44)), Reaction::default());
        assert_eq!(with_frame(wanting(attacking, 316)), Reaction::default());
        assert_eq!(with_frame(wanting(attacking, 45)), TAKES(n(3)), "45 off");
        assert_eq!(with_frame(wanting(attacking, 315)), TAKES(n(3)));
        assert_eq!(with_frame(wanting(attacking, 90)), TAKES(n(3)));
        assert_eq!(
            with_frame(wanting(Goal::Attack(n(9)), 90)),
            Reaction::default(),
            "a stale frame: the bearing, 0, stands in"
        );
        let mut aside = hunted(100.0);
        aside[1].state.position = Vec2::new(-50.0, -50.0);
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &aside, 0, &hit),
            TAKES(n(3)),
            "no frame, its target 45 off the bearing"
        );
        aside[0].pursuit.frame = Some(wanting(attacking, 0));
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &aside, 0, &hit),
            Reaction::default(),
            "the frame wants it dead ahead"
        );
    }

    #[test]
    fn the_facing_reads_its_heading_in_whole_degrees() {
        // `_AngularDifference(trunc(+0x44), +0x68)` <= 0x2c.
        let govts = relations(0, 0);
        let hit = strike(n(1), n(3));
        let heading = |degrees: f32| {
            let mut npcs = hunted(100.0);
            npcs[0].state.heading = degrees;
            focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit)
        };
        assert_eq!(heading(44.9), Reaction::default());
        assert_eq!(heading(45.0), TAKES(n(3)));
        assert_eq!(heading(316.0), Reaction::default());
        assert_eq!(heading(315.9), TAKES(n(3)));
    }

    #[test]
    fn the_focus_holds_back_only_a_ship_attacking() {
        // `_IsShipAttacking` (@0x3bc18 → @0x82f6a): goal 4.
        let govts = relations(0, 0);
        let hit = strike(n(1), n(3));
        for goal in [
            Goal::Plunder {
                ship: n(2),
                hopeless: false,
            },
            Goal::Flee(n(2)),
            Goal::Idle,
        ] {
            let mut npcs = hunted(100.0);
            npcs[0].goal = goal;
            assert_eq!(
                focused_by(RuleSource::Engine, &govts, &npcs, 0, &hit),
                TAKES(n(3)),
                "{goal:?}"
            );
        }
        let mut sniping = hunted(100.0);
        sniping[0].goal = Goal::Snipe(n(2));
        assert_eq!(
            focused_by(RuleSource::Engine, &govts, &sniping, 0, &hit),
            Reaction::default()
        );
    }

    #[test]
    fn by_the_other_reading_a_ship_keeps_a_target_nearer_than_half_the_attackers_distance_ahead_of_it()
     {
        let govts = relations(0, 0);
        let bible = |govts: &Governments, npcs: &[Npc], who: usize, hit: &Strike| {
            focused_by(RuleSource::Bible, govts, npcs, who, hit)
        };
        // The ship faces up; its target 50 above, the attacker 100 right.
        let mut hunter = ship(1, Some(POLICE), AiType::Warship, 0.0);
        hunter.goal = Goal::Attack(n(2));
        hunter.target = Some(n(2));
        let mut quarry = ship(2, Some(PIRATES), AiType::Warship, 0.0);
        quarry.state.position = Vec2::new(0.0, -49.9);
        let attacker = ship(3, Some(PIRATES), AiType::Warship, 100.0);
        let npcs = [hunter, quarry, attacker];
        let hit = strike(n(1), n(3));
        assert_eq!(bible(&govts, &npcs, 0, &hit), Reaction::default(), "kept");
        let mut farther = npcs.clone();
        farther[1].state.position = Vec2::new(0.0, -50.0);
        assert_eq!(bible(&govts, &farther, 0, &hit), TAKES(n(3)));
        // Near enough, but 45 degrees off the nose; then 1.
        let mut aside = npcs.clone();
        aside[1].state.position = Vec2::new(-30.0, -30.0);
        assert_eq!(bible(&govts, &aside, 0, &hit), TAKES(n(3)));
        aside[0].state.heading = 316.0;
        assert_eq!(bible(&govts, &aside, 0, &hit), Reaction::default());
        let mut gone = npcs.clone();
        gone[0].goal = Goal::Attack(n(9));
        gone[0].target = Some(n(9));
        assert_eq!(
            bible(&govts, &gone, 0, &hit),
            TAKES(n(3)),
            "its target gone"
        );
        let shift = |ships: &[Npc]| -> Vec<Npc> {
            ships
                .iter()
                .cloned()
                .map(|mut npc| {
                    npc.state.position = npc.state.position + Vec2::new(300.0, -200.0);
                    npc
                })
                .collect()
        };
        assert_eq!(
            bible(&govts, &shift(&npcs), 0, &hit),
            Reaction::default(),
            "measured from where it is"
        );
        assert_eq!(bible(&govts, &shift(&farther), 0, &hit), TAKES(n(3)));
        let mut by_its_target = npcs.clone();
        by_its_target[0].goal = Goal::Attack(n(3));
        by_its_target[0].target = Some(n(3));
        assert_eq!(
            bible(&govts, &by_its_target, 0, &hit),
            TAKES(n(3)),
            "hit by the ship it attacks"
        );
        assert_eq!((FOCUS, FOCUS_ANGLE), (0.25, 44.0));
        let mut plundering = npcs.clone();
        plundering[0].goal = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        assert_eq!(
            bible(&govts, &plundering, 0, &hit),
            TAKES(n(3)),
            "a plunder (goal 0xd) is no attack to `_IsShipAttacking` (@0x3bc18)"
        );
    }

    /// A Samaritan W of `govt` and `ai_type` answering the player's hit
    /// on a trader of `victim`.
    fn samaritan(
        govts: &Governments,
        w: Option<GovtId>,
        ai_type: AiType,
        victim: Option<GovtId>,
    ) -> Reaction {
        let npcs = [
            ship(1, victim, AiType::WimpyTrader, 0.0),
            ship(2, w, ai_type, 3000.0),
        ];
        answered(govts, &npcs, 1, &strike(n(1), ShipRef::Player))
    }

    const ANSWERS: Reaction = Reaction {
        target: Some(ShipRef::Player),
        provoked: 0.0,
        goal: Some(Goal::Attack(ShipRef::Player)),
    };

    #[test]
    fn allied_warships_and_interceptors_answer_the_players_attack_anywhere() {
        let govts = relations(0, 0);
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            for w in [Some(POLICE), Some(TRADERS)] {
                assert_eq!(
                    samaritan(&govts, w, ai_type, Some(TRADERS)),
                    ANSWERS,
                    "{w:?} {ai_type:?}"
                );
            }
        }
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            assert_eq!(
                samaritan(&govts, Some(POLICE), ai_type, Some(TRADERS)),
                Reaction::default(),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn no_samaritan_answers_the_players_unintended_hit() {
        let govts = relations(0, 0);
        let npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(POLICE), AiType::Warship, 3000.0),
        ];
        let hit = strike(n(1), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 1, &hit), ANSWERS, "intended");
        let stray = Strike {
            intended: false,
            ..hit
        };
        assert_eq!(
            answered(&govts, &npcs, 1, &stray),
            Reaction::default(),
            "a stray hit on a ship the player wasn't aiming at (@0x3bed2)"
        );
        assert_eq!(
            answered(&govts, &npcs, 0, &stray),
            TURNS,
            "the victim still answers it"
        );
    }

    #[test]
    fn by_the_bible_only_interceptors_police_piracy() {
        let govts = relations(0, 0);
        let answer_of = |police, ai_type| {
            let npcs = [
                ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
                ship(2, Some(POLICE), ai_type, 3000.0),
            ];
            answered_by(police, &govts, &npcs, 1, &strike(n(1), ShipRef::Player))
        };
        assert_eq!(answer_of(RuleSource::Engine, AiType::Warship), ANSWERS);
        assert_eq!(answer_of(RuleSource::Engine, AiType::Interceptor), ANSWERS);
        assert_eq!(answer_of(RuleSource::Bible, AiType::Interceptor), ANSWERS);
        assert_eq!(
            answer_of(RuleSource::Bible, AiType::Warship),
            Reaction::default()
        );
        // A warship hit itself still answers by the Bible.
        let npcs = [
            ship(1, Some(TRADERS), AiType::Warship, 0.0),
            ship(2, Some(PIRATES), AiType::Warship, 500.0),
        ];
        assert_eq!(
            answered_by(RuleSource::Bible, &govts, &npcs, 0, &strike(n(1), n(2))),
            TAKES(n(2))
        );
    }

    #[test]
    fn a_samaritan_already_attacking_or_hit_by_another_keeps_to_its_fight() {
        let govts = relations(0, 0);
        let mut npcs = [
            ship(1, Some(TRADERS), AiType::WimpyTrader, 0.0),
            ship(2, Some(POLICE), AiType::Warship, 3000.0),
        ];
        npcs[1].goal = Goal::Attack(n(9));
        npcs[1].target = Some(n(9));
        let hit = strike(n(1), ShipRef::Player);
        assert_eq!(answered(&govts, &npcs, 1, &hit), Reaction::default());
        npcs[1].goal = Goal::Snipe(n(9));
        assert_eq!(answered(&govts, &npcs, 1, &hit), Reaction::default());
        npcs[1].goal = Goal::Flee(n(9));
        assert_eq!(
            answered(&govts, &npcs, 1, &hit),
            ANSWERS,
            "fleeing is no fight"
        );
        let by_npc = strike(n(1), n(3));
        npcs[1].goal = Goal::Idle;
        assert_eq!(
            answered(&govts, &npcs, 1, &by_npc),
            Reaction::default(),
            "only the player's attacks"
        );
    }

    #[test]
    fn no_samaritan_answers_for_an_enemy_a_xenophobe_a_derelict_or_the_ignored() {
        let govts = relations(0, 0);
        assert_eq!(
            samaritan(&govts, Some(PIRATES), AiType::Warship, Some(TRADERS)),
            Reaction::default(),
            "W at war with the victim"
        );
        assert_eq!(
            samaritan(&govts, Some(POLICE), AiType::Warship, Some(XENO)),
            Reaction::default(),
            "a xenophobic victim"
        );
        assert_eq!(
            samaritan(&govts, Some(XENO), AiType::Warship, Some(XENO)),
            ANSWERS,
            "its own xenophobes"
        );
        for flags in [DERELICT, IGNORED_WHEN_ATTACKED] {
            assert_eq!(
                samaritan(
                    &relations(flags, 0),
                    Some(POLICE),
                    AiType::Warship,
                    Some(TRADERS)
                ),
                Reaction::default(),
                "{flags:#x}"
            );
        }
    }

    #[test]
    fn a_neutral_answers_only_when_nosy() {
        assert_eq!(
            samaritan(
                &relations(0, 0),
                Some(NEUTRAL),
                AiType::Warship,
                Some(TRADERS)
            ),
            Reaction::default()
        );
        assert_eq!(
            samaritan(
                &relations(0, NOSY),
                Some(NEUTRAL),
                AiType::Warship,
                Some(TRADERS)
            ),
            ANSWERS
        );
    }

    #[test]
    fn an_independent_samaritan_minds_only_derelicts_the_ignored_and_xenophobes() {
        let govts = relations(0, 0);
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(TRADERS)),
            ANSWERS
        );
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(PIRATES)),
            ANSWERS,
            "nobody's enemy"
        );
        assert_eq!(
            samaritan(&govts, None, AiType::Warship, Some(XENO)),
            Reaction::default()
        );
        assert_eq!(
            samaritan(
                &relations(IGNORED_WHEN_ATTACKED, 0),
                None,
                AiType::Warship,
                Some(TRADERS)
            ),
            Reaction::default()
        );
    }

    #[test]
    fn for_an_independent_victim_only_the_nosy_answer() {
        assert_eq!(
            samaritan(&relations(0, 0), Some(POLICE), AiType::Warship, None),
            Reaction::default()
        );
        assert_eq!(
            samaritan(&relations(0, NOSY), Some(NEUTRAL), AiType::Warship, None),
            ANSWERS
        );
        assert_eq!(
            samaritan(&relations(0, NOSY), None, AiType::Warship, None),
            Reaction::default(),
            "an independent is not nosy"
        );
    }

    // The boarding's call.

    /// The Good Samaritan call of the player's boarding of `victim`:
    /// intended and answered, of no damage, but not direct, as it never
    /// passes `_DamageShip` (`_DoGoodSamaritan` @0x65ed9).
    fn call(victim: ShipRef) -> Strike {
        Strike {
            direct: false,
            damage: 0.0,
            ..strike(victim, ShipRef::Player)
        }
    }

    /// A trader lead (1) of a warship escort (2) of the traders, and a
    /// police warship (3) far off.
    fn escorted(lead: AiType) -> [Npc; 3] {
        let mut escort = ship(2, Some(TRADERS), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        [
            ship(1, Some(TRADERS), lead, 0.0),
            escort,
            ship(3, Some(POLICE), AiType::Warship, 3000.0),
        ]
    }

    #[test]
    fn a_boarding_call_draws_no_hit_reaction_from_the_ship_or_its_lead() {
        // `_DoGoodSamaritan` writes goal 4 and the target alone, on the
        // slots its gates pass (@0x82b82-0x82b97): no victim's reaction.
        let govts = relations(0, 0);
        let npcs = escorted(AiType::WimpyTrader);
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let boarded = call(n(2));
        assert_eq!(
            victim(&npcs[1], &boarded, &around),
            Reaction::default(),
            "the ship boarded"
        );
        assert_eq!(
            victim(&npcs[0], &boarded, &around),
            Reaction::default(),
            "its lead"
        );
        assert_eq!(
            victim(&npcs[1], &strike(n(2), ShipRef::Player), &around),
            TURNS,
            "a direct hit still turns it"
        );
    }

    #[test]
    fn a_boarding_call_makes_a_warship_lead_and_the_ship_itself_samaritans_and_a_trader_lead_nothing()
     {
        // The loop @0x82891-0x82ba8 excludes no slot: the lead and the
        // ship boarded pass the gates as any other ship does.
        let govts = relations(0, 0);
        let boarded = call(n(2));
        let warship_lead = escorted(AiType::Warship);
        assert_eq!(
            answered(&govts, &warship_lead, 0, &boarded),
            ANSWERS,
            "a warship lead"
        );
        assert_eq!(
            answered(&govts, &warship_lead, 1, &boarded),
            ANSWERS,
            "the warship boarded"
        );
        assert_eq!(
            answered(&govts, &warship_lead, 2, &boarded),
            ANSWERS,
            "an allied police warship"
        );
        assert_eq!(
            answered(&govts, &escorted(AiType::WimpyTrader), 0, &boarded),
            Reaction::default(),
            "a trader lead"
        );
        assert_eq!(
            answered_by(RuleSource::Bible, &govts, &warship_lead, 0, &boarded),
            Reaction::default(),
            "a warship lead, by the Bible"
        );
        assert_eq!(
            answered_by(
                RuleSource::Bible,
                &govts,
                &escorted(AiType::Interceptor),
                0,
                &boarded
            ),
            ANSWERS,
            "an interceptor lead, by the Bible"
        );
    }

    // The lead of an escort the player hits.

    /// The lead of an escort the player hit, answering as a Samaritan as
    /// well as taking the target: provoked by the hit, attacking the
    /// player.
    const LEAD_ANSWERS: Reaction = Reaction {
        target: Some(ShipRef::Player),
        provoked: 24.0,
        goal: Some(Goal::Attack(ShipRef::Player)),
    };

    #[test]
    fn a_warship_or_interceptor_lead_of_an_escort_the_player_hits_takes_it_and_answers_as_a_samaritan()
     {
        // `_DamageShip`'s lead part (@0x3bd1b-0x3bd79) adds +0x96 and sets
        // +0x70 but never the lead's goal, then falls to the call
        // `_DoGoodSamaritan(victim, player)` (@0x3bed2-0x3bef4), whose
        // goal gate (goal != 4, @0x828b3) the lead passes like any slot.
        let govts = relations(0, 0);
        let hit = strike(n(2), ShipRef::Player);
        for lead in [AiType::Warship, AiType::Interceptor] {
            let npcs = escorted(lead);
            assert_eq!(answered(&govts, &npcs, 0, &hit), LEAD_ANSWERS, "{lead:?}");
            assert_eq!(
                answered(&govts, &npcs, 1, &hit),
                TURNS,
                "{lead:?}: the escort"
            );
        }
        assert_eq!(
            answered(&govts, &escorted(AiType::WimpyTrader), 0, &hit),
            TAKES(ShipRef::Player),
            "a trader lead is no Samaritan"
        );
        assert_eq!(
            answered_by(
                RuleSource::Bible,
                &govts,
                &escorted(AiType::Warship),
                0,
                &hit
            ),
            TAKES(ShipRef::Player),
            "a warship lead, by the Bible"
        );
        assert_eq!(
            answered_by(
                RuleSource::Bible,
                &govts,
                &escorted(AiType::Interceptor),
                0,
                &hit
            ),
            LEAD_ANSWERS,
            "an interceptor lead, by the Bible"
        );
    }

    #[test]
    fn a_lead_whose_samaritan_gates_fail_takes_the_target_only() {
        let govts = relations(0, 0);
        let hit = strike(n(2), ShipRef::Player);
        let mut attacking = escorted(AiType::Warship);
        attacking[0].goal = Goal::Attack(n(9));
        attacking[0].target = Some(n(9));
        assert_eq!(
            answered(&govts, &attacking, 0, &hit),
            TAKES(ShipRef::Player),
            "already attacking (@0x828b3)"
        );
        let stray = Strike {
            intended: false,
            ..hit
        };
        assert_eq!(
            answered(&govts, &escorted(AiType::Warship), 0, &stray),
            TAKES(ShipRef::Player),
            "the hit not intended: no call (@0x3bed2)"
        );
        assert_eq!(
            answered(
                &relations(IGNORED_WHEN_ATTACKED, 0),
                &escorted(AiType::Warship),
                0,
                &hit
            ),
            TAKES(ShipRef::Player),
            "the victim's government ignored when attacked"
        );
        let npcs = escorted(AiType::Warship);
        let blessed = |blessing| {
            let around = Surroundings {
                govts: &govts,
                player: Some(crate::ai::PlayerSide {
                    blessing,
                    ..crate::ai::fixture::player(0.0, -100.0)
                }),
                ..Surroundings::new(&[], &npcs)
            };
            answer(&npcs[0], &hit, &around, RuleSource::Engine)
        };
        assert_eq!(blessed(crate::ai::Blessing::NONE), LEAD_ANSWERS);
        assert_eq!(
            blessed(crate::ai::fixture::blessing_of(TRADERS)),
            TAKES(ShipRef::Player),
            "its government blesses the player"
        );
    }

    #[test]
    fn the_lead_of_an_escort_keeping_its_target_against_the_player_still_answers_as_a_samaritan() {
        // The focus skip jumps to 0x3bed2 (@0x3bcac), past the lead's
        // +0x96 and +0x70, but not past the Good Samaritan call.
        let govts = relations(0, 0);
        // An idle lead (1); its escort (2) 50 right attacks a quarry (4)
        // 50 above itself; the player 100 right of the escort.
        let mut escort = ship(2, Some(POLICE), AiType::Warship, 50.0);
        escort.leader = Some(NpcId(1));
        escort.goal = Goal::Attack(n(4));
        escort.target = Some(n(4));
        let mut quarry = ship(4, Some(PIRATES), AiType::Warship, 50.0);
        quarry.state.position = Vec2::new(50.0, -50.0);
        let npcs = [ship(1, Some(POLICE), AiType::Warship, 0.0), escort, quarry];
        let player = crate::ai::fixture::player(150.0, 0.0);
        let hit = strike(n(2), ShipRef::Player);
        let engine = |npcs: &[Npc], who| {
            focused_around(RuleSource::Engine, &govts, npcs, Some(&player), who, &hit)
        };
        let bible = |npcs: &[Npc], who| {
            focused_around(RuleSource::Bible, &govts, npcs, Some(&player), who, &hit)
        };
        assert_eq!(engine(&npcs, 1), Reaction::default(), "the escort keeps it");
        assert_eq!(engine(&npcs, 0), ANSWERS, "the lead, unprovoked");
        assert_eq!(
            bible(&npcs, 0),
            LEAD_ANSWERS,
            "by the other reading the idle lead tests itself"
        );
        // The lead attacking a quarry 10 above itself.
        let mut turned = npcs.clone();
        turned[0].goal = Goal::Attack(n(4));
        turned[0].target = Some(n(4));
        turned[2].state.position = Vec2::new(0.0, -10.0);
        assert_eq!(
            bible(&turned, 0),
            Reaction::default(),
            "by the other reading it keeps its own, and attacking is no Samaritan"
        );
    }

    // The blessing.

    /// [`samaritan`] for a warship W, the player blessed by `blessing`,
    /// its strike of `damage`.
    fn samaritan_blessed(
        govts: &Governments,
        w: Option<GovtId>,
        victim: Option<GovtId>,
        blessing: crate::ai::Blessing,
        damage: f32,
    ) -> Reaction {
        let npcs = [
            ship(1, victim, AiType::WimpyTrader, 0.0),
            ship(2, w, AiType::Warship, 3000.0),
        ];
        let around = Surroundings {
            govts,
            player: Some(crate::ai::PlayerSide {
                blessing,
                ..crate::ai::fixture::player(0.0, -100.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        let hit = Strike {
            damage,
            ..strike(n(1), ShipRef::Player)
        };
        answer(&npcs[1], &hit, &around, RuleSource::Engine)
    }

    #[test]
    fn a_blessing_government_is_no_good_samaritan() {
        let govts = relations(0, 0);
        let blessed = crate::ai::fixture::blessing_of(POLICE);
        let unblessed = crate::ai::Blessing::NONE;
        for (damage, case) in [(12.0, "a shot"), (0.0, "a boarding")] {
            assert_eq!(
                samaritan_blessed(&govts, Some(POLICE), Some(TRADERS), unblessed, damage),
                ANSWERS,
                "{case}: unblessed"
            );
            assert_eq!(
                samaritan_blessed(&govts, Some(POLICE), Some(TRADERS), blessed, damage),
                Reaction::default(),
                "{case}"
            );
            assert_eq!(
                samaritan_blessed(
                    &govts,
                    Some(POLICE),
                    Some(TRADERS),
                    crate::ai::fixture::blessing_of(TRADERS),
                    damage
                ),
                ANSWERS,
                "{case}: the victim's blessing is not W's"
            );
        }
    }

    #[test]
    fn a_nosy_blessing_warship_still_answers_an_attack_on_an_independent() {
        let govts = relations(0, NOSY);
        let blessed = crate::ai::fixture::blessing_of(NEUTRAL);
        assert_eq!(
            samaritan_blessed(&govts, Some(NEUTRAL), None, blessed, 12.0),
            ANSWERS
        );
        assert_eq!(
            samaritan_blessed(&govts, Some(NEUTRAL), Some(TRADERS), blessed, 12.0),
            Reaction::default(),
            "a governed victim"
        );
    }
}
