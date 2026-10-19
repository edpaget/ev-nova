//! Whether a ship hit answers the hit at all: `_DamageShip`'s answer
//! flag (the byte at -0x178, @0x3b408-0x3bb1a in the `EV Nova`
//! executable) and the reaction block's entry test (@0x3bb20-0x3bb74),
//! with the player's stray provocation (ship +0xc910) they read and
//! raise. A strike it clears skips the whole reaction block
//! (@0x3baea-0x3baf1, a jump to 0x3bef9): the ship hit and the lead of an
//! escort hit take no target and no provocation, the Good Samaritans
//! stay put, no person's grudge is held and the ship doesn't cloak when
//! hit (see [`react`](crate::ai::react) and the session's settle step).
//! Its decloak (@0x3bf1f) comes after and still runs for any direct hit.
//!
//! [`verdict`] works the flag out in the original's order. `si` is the
//! attacker, the player (slot 0), an NPC (1-63) or none (-1); the player
//! attacks as an independent with no leader (`_ResetPlayer` sets its
//! +0x98 and +0x9a to -1, @0x1d46f, @0x1d4e5), and a ship's leader is its
//! [`leader_ref`](Npc::leader_ref) (+0x9a). An NPC attacker no longer in
//! the system is read as none.
//!
//! | Branch | Addresses | What it does |
//! |---|---|---|
//! | Entry gate | @0x3b408-0x3b46e | only a direct hit ([`Strike::direct`]) on an NPC, not by itself, is weighed, and not on a ship with no AI ([`Npc::no_ai`], +0x88 at or below 0 @0x3b459: the player's eject hulk); anything else gives no answer and no provocation |
//! | Kin | @0x3b4cd-0x3b4fc | set when a ship of another government hit it, the governments compared as raw words, so two independents are kin ([`RuleKey::IndependentKin`](crate::RuleKey::IndependentKin)); clear for no ship |
//! | Leaders | @0x3b503-0x3b571 | for a led ship (leader L, the player included): set when `si` isn't L, or a ship whose own leader isn't L hit it |
//! | The record | @0x3b577-0x3b67e | a governed ship, in a governed system, not the player's target, cleared for the player or a ship it leads while the player's record there is at least twice its government's `CrimeTol` ([`Surroundings::record`]); then cleared for a ship of its own government. An independent ship: set for the player or a ship it leads |
//! | No damage | @0x3b6f2-0x3b73e | cleared for a hit whose nominal mass and energy damage ([`Strike::nominal`]) are each at or below none and above -32000; the missile lock's -32000 passes ([`Strike::lock`]) |
//! | Aim | @0x3b744-0x3b863 | for a stray hit ([`Strike::intended`] not) still set: the player's on a ship other than its target, when its target's government is the ship's NPC enemy (`_IsShipMyEnemy` @0x8196c, [`Governments::npc_enemies`](crate::govt::Governments::npc_enemies)), clears; otherwise the player's on a governed ship whose government's `ShootPenalty` is at or below none is forgiven 3 times in 4 (`_Rand(4)` @0x3b80a-0x3b847, [`RuleKey::StrayShootPenalty`](crate::RuleKey::StrayShootPenalty)), and then while the ship's stray provocation is below [`STRAY_THRESHOLD`] (read before this hit's add). An NPC's on a ship it doesn't target clears |
//! | Provocation | @0x3b86a-0x3b89b | a stray hit of the player's adds [`STRAY_WEIGHT`] × its [`weight`](Strike::weight) to the ship's stray provocation, whatever the flag ([`Verdict::stray`]) |
//! | Escort near the player | @0x3b8a3-0x3b948 | cleared for a fighter the player launched (AI type 5, its leader the player) hitting a ship that targets the player within [`ESCORT_REACH`] of it on both axes |
//! | Same leader | @0x3b94e-0x3b978 | cleared for a ship of the same leader |
//! | Blessing | @0x3b9a9-0x3ba08 | for a stray hit still set: cleared for a governed ship allied with the player (`_AlliedWithPlayer` @0x6985: led by it, or by a ship it leads) whose government blesses it with peace ([`Surroundings::blessed`]) |
//! | Grand fleet | @0x3ba14-0x3baa1 | cleared when the ship's leader's leader is an NPC attacker's leader's leader, the player included |
//! | Docking | @0x3bab7-0x3bb1a | an NPC's hit on a ship docking (`_AIShipIsDocking` @0x83f95, substate 0xf), here a pirate docking with or docked to a ship it plunders, or one docking with or docked to the player it repairs ([`Goal::Assist`] with [`Help::Repair`]), its [`Npc::docking`] in [`DockSub::Dock`], is not answered |
//! | Entry test | @0x3bb20-0x3bb74 | a fighter-class attacker (ship class +0xe, [`EscortClass::Fighter`]), the player's ship included, on a ship that isn't one, has a target and attacks (`_IsShipAttacking` @0x82f6a, goal 4: [`Goal::attacking`]), is not answered |
//!
//! Left out, for want of what they read (weapons-fidelity phase 37
//! records each; the plunder docking of the table above, once among
//! them, is built):
//!
//! - **Defence ships** (+0x8c, the stellar a ship defends): a self-hit by
//!   one (@0x3b408-0x3b4c8; either way a ship hitting itself never
//!   answers), fleetmates (@0x3b68e-0x3b6ec). Noted on rdm task
//!   planet-domination, with the reaction block's defence-ship branches.
//! - **The jump-out stage** (+0x50 above 0): the entry gate's test
//!   (@0x3b46a), arriving (@0x3b97e, dead code after the gate) and a
//!   victim whose leader jumps out (@0x3bab7-0x3bae4). They read the
//!   jump-out stage of roadmap ship-handling-fidelity's phase 8, and are
//!   rdm phase weapons-fidelity-cross-roadmap/phase-1-damage-ship-jump-out-stage.
//! - **The nag ship**'s damage rewrite (@0x3a8bb-0x3a954) and its test in
//!   `_IsShipMyEnemy` (@0x819a4): the engine has no nag ship. Recorded
//!   only.
//!
//! Each draw is made only on its own path; the 1-in-4 answer is matched,
//! not the order of the draws. Nothing in the flag, the entry test or the
//! provocation's ease (`_HandleShip` @0x35374-0x353b2) plays a sound.

use crate::ai::Surroundings;
#[cfg(doc)]
use crate::ai::{Goal, Help};
use crate::catalog::EscortClass;
use crate::chance::Chance;
use crate::combat::{ShipRef, Strike};
use crate::legal::Crime;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::plunder_dock::DockSub;

/// The stray provocation below which the player's stray hit is forgiven
/// (50.0 @0xdd064, read @0x3b85a).
pub const STRAY_THRESHOLD: f32 = 50.0;
/// What a stray hit of the player's adds to its ship's stray provocation,
/// a time its weight (1.75 @0xdd6e8).
pub const STRAY_WEIGHT: f32 = 1.75;
/// What the stray provocation eases by each tick while above none
/// (-0.5 @0xdd488, `_HandleShip` @0x35374-0x353b2, its frame factor 1).
pub const STRAY_EASE: f32 = 0.5;
/// How near the player, on each axis, a ship targeting it lets the
/// player's own fighter's hit pass (320.0 @0xdd720).
pub const ESCORT_REACH: f32 = 320.0;
/// The sides of the roll that forgives the player's stray hit: any but 0
/// forgives it (`_Rand(4)` @0x3b821).
pub const FORGIVE_SIDES: u16 = 4;
/// The damage at or below which a hit is no hit of no damage: the
/// missile lock's sentinel (-32000, 0x8300 @0x3b716).
pub const LOCK_DAMAGE: f32 = -32_000.0;

/// What a strike's ship makes of it ([`verdict`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Verdict {
    /// Whether it answers: its reaction runs ([`react`](crate::ai::react)).
    pub answers: bool,
    /// What it adds to its stray provocation ([`Npc::stray`]), before an
    /// answer zeroes it (@0x3bb83).
    pub stray: f32,
}

/// The readings of the answer flag's two disputed rules: by default the
/// engine's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlagRules {
    /// Whether a `ShootPenalty` above 0 holds back the forgiving roll:
    /// [`RuleKey::StrayShootPenalty`](crate::RuleKey::StrayShootPenalty).
    pub shoot_penalty: RuleSource,
    /// Whether two independents are kin:
    /// [`RuleKey::IndependentKin`](crate::RuleKey::IndependentKin).
    pub independent_kin: RuleSource,
}

/// Who made a strike, as the flag reads it.
#[derive(Clone, Copy, Debug)]
enum Attacker<'a> {
    /// No ship, or one no longer in the system (`si` -1).
    None,
    /// The player (`si` 0).
    Player,
    /// An NPC in the system (`si` 1-63).
    Npc(&'a Npc),
}

impl<'a> Attacker<'a> {
    /// The maker of `strike` among `around`.
    fn of(strike: &Strike, around: &Surroundings<'a>) -> Self {
        match strike.by {
            Some(ShipRef::Player) => Self::Player,
            Some(ShipRef::Npc(id)) => around.npc(id).map_or(Self::None, Self::Npc),
            None => Self::None,
        }
    }

    /// Its government, the player's and none's none.
    fn govt(self) -> Option<crate::catalog::GovtId> {
        match self {
            Self::Npc(npc) => npc.govt,
            Self::Player | Self::None => None,
        }
    }

    /// Its leader (+0x9a): an NPC's [`leader_ref`](Npc::leader_ref), the
    /// player's none.
    fn leader(self) -> Option<ShipRef> {
        match self {
            Self::Npc(npc) => npc.leader_ref(),
            Self::Player | Self::None => None,
        }
    }

    /// Whether it is the player or a ship the player leads (+0x9a 0).
    fn players(self) -> bool {
        match self {
            Self::Player => true,
            Self::Npc(npc) => npc.leader_ref() == Some(ShipRef::Player),
            Self::None => false,
        }
    }
}

/// What `strike`'s ship among `around` makes of it, by `rules`, rolling
/// on `chance` (see the module docs).
#[must_use]
pub fn verdict(
    strike: &Strike,
    around: &Surroundings,
    rules: FlagRules,
    chance: &mut dyn Chance,
) -> Verdict {
    let ShipRef::Npc(id) = strike.ship else {
        return Verdict::default();
    };
    let Some(victim) = around.npc(id) else {
        return Verdict::default();
    };
    // A victim of AI type -1 skips the whole block, its stray
    // provocation too (@0x3b459-0x3b461).
    if !strike.direct || strike.by == Some(strike.ship) || victim.no_ai {
        return Verdict::default();
    }
    let attacker = Attacker::of(strike, around);
    let stray_hit = !strike.intended;
    let mut answers =
        kin(victim, attacker, rules.independent_kin) || leaders(victim, strike.by, attacker);
    answers = record(answers, victim, attacker, around);
    answers = answers && !no_damage(strike);
    if stray_hit && answers {
        answers = aim(victim, attacker, around, rules.shoot_penalty, chance);
    }
    let stray = if stray_hit && matches!(attacker, Attacker::Player) {
        strike.weight * STRAY_WEIGHT
    } else {
        0.0
    };
    answers =
        answers && !escort_near_player(victim, attacker, around) && !same_leader(victim, attacker);
    if stray_hit && answers {
        answers = !blessing(attacker, around);
    }
    answers = answers
        && !grand_fleet(victim, attacker, around)
        && !docking(victim, attacker)
        && !entry_test(victim, attacker, around);
    Verdict { answers, stray }
}

/// Kin (@0x3b4cd-0x3b4fc): whether a ship of another government hit
/// `victim`. Two independents are kin by the engine, as -1 is -1, and not
/// by the other reading of `independent_kin`. No ship never is.
fn kin(victim: &Npc, attacker: Attacker, independent_kin: RuleSource) -> bool {
    if matches!(attacker, Attacker::None) {
        return false;
    }
    // An independent attacker of a governed victim differs already, so
    // the other reading need ask only whether the attacker is one.
    let govt = attacker.govt();
    govt != victim.govt || (independent_kin == RuleSource::Bible && govt.is_none())
}

/// Leaders (@0x3b503-0x3b571): whether `victim`, led by L, was hit `by`
/// a ship other than L (or none), or by a ship whose own leader isn't L.
fn leaders(victim: &Npc, by: Option<ShipRef>, attacker: Attacker) -> bool {
    let Some(lead) = victim.leader_ref() else {
        return false;
    };
    by != Some(lead) || attacker.leader().is_some_and(|own| own != lead)
}

/// The record (@0x3b577-0x3b67e): the flag `answers` once the player's
/// record among `around` and `victim`'s government have had their say.
fn record(answers: bool, victim: &Npc, attacker: Attacker, around: &Surroundings) -> bool {
    if victim.govt.is_none() {
        return answers || attacker.players();
    }
    let me = ShipRef::Npc(victim.id);
    let targeted = around.player.and_then(|player| player.target) == Some(me);
    let tolerated = i32::from(around.record) >= 2 * i32::from(around.govts.crime_tol(victim.govt));
    let forgiven = around.system_govt.is_some() && !targeted && tolerated && attacker.players();
    let own = matches!(attacker, Attacker::Npc(npc) if npc.govt == victim.govt);
    answers && !forgiven && !own
}

/// No damage (@0x3b6f2-0x3b73e): whether `strike` passed no mass and no
/// energy damage, each at or below none and above [`LOCK_DAMAGE`]. A
/// missile lock, whose nominal damage is none here, passes as the
/// original's -32000 does.
fn no_damage(strike: &Strike) -> bool {
    let none = |damage: f32| damage <= 0.0 && damage > LOCK_DAMAGE;
    !strike.lock && none(strike.nominal.mass) && none(strike.nominal.energy)
}

/// Aim (@0x3b744-0x3b863): whether a stray hit on `victim` among `around`
/// is still answered, rolling the forgiveness on `chance` as
/// `shoot_penalty` reads `ShootPenalty`.
fn aim(
    victim: &Npc,
    attacker: Attacker,
    around: &Surroundings,
    shoot_penalty: RuleSource,
    chance: &mut dyn Chance,
) -> bool {
    let me = ShipRef::Npc(victim.id);
    match attacker {
        Attacker::Player => {
            let target = around.player.and_then(|player| player.target);
            let hostile = target.filter(|&target| target != me).is_some_and(|target| {
                let ShipRef::Npc(id) = target else {
                    return false;
                };
                around
                    .npc(id)
                    .is_some_and(|other| around.govts.npc_enemies(victim.govt, other.govt))
            });
            if hostile {
                return false;
            }
            let rolls = victim.govt.is_some()
                && (shoot_penalty == RuleSource::Bible
                    || around.govts.penalty(victim.govt, Crime::Shoot) <= 0);
            if rolls && chance.roll(FORGIVE_SIDES) != 0 {
                return false;
            }
            victim.stray >= STRAY_THRESHOLD
        }
        Attacker::Npc(npc) => npc.target == Some(me),
        Attacker::None => true,
    }
}

/// Escort near the player (@0x3b8a3-0x3b948): whether a fighter the
/// player launched hit `victim`, which targets the player within
/// [`ESCORT_REACH`] of it on both axes among `around`.
fn escort_near_player(victim: &Npc, attacker: Attacker, around: &Surroundings) -> bool {
    let Attacker::Npc(npc) = attacker else {
        return false;
    };
    let Some(player) = around.player else {
        return false;
    };
    let launched = npc
        .carrier
        .is_some_and(|carrier| carrier.ship == ShipRef::Player);
    let off = victim.state.position - player.state.position;
    launched
        && attacker.players()
        && victim.target == Some(ShipRef::Player)
        && off.x.abs() <= ESCORT_REACH
        && off.y.abs() <= ESCORT_REACH
}

/// Same leader (@0x3b94e-0x3b978): whether a ship of `victim`'s own
/// leader hit it.
fn same_leader(victim: &Npc, attacker: Attacker) -> bool {
    attacker
        .leader()
        .is_some_and(|lead| victim.leader_ref() == Some(lead))
}

/// Blessing (@0x3b9a9-0x3ba08): whether a governed NPC allied with the
/// player ([`Surroundings::allied_with_player`]) whose government blesses
/// the player with peace among `around` hit it.
fn blessing(attacker: Attacker, around: &Surroundings) -> bool {
    let Attacker::Npc(npc) = attacker else {
        return false;
    };
    npc.govt.is_some() && around.allied_with_player(npc) && around.blessed(npc.govt)
}

/// Grand fleet (@0x3ba14-0x3baa1): whether `victim`'s leader's leader is
/// an NPC attacker's leader's leader among `around`, the player
/// included.
fn grand_fleet(victim: &Npc, attacker: Attacker, around: &Surroundings) -> bool {
    let lead_of_lead = |lead: Option<ShipRef>| match lead {
        Some(ShipRef::Npc(id)) => around.npc(id).and_then(Npc::leader_ref),
        _ => None,
    };
    let theirs = lead_of_lead(victim.leader_ref());
    let ours = lead_of_lead(attacker.leader());
    theirs.is_some() && theirs == ours
}

/// Docking (@0x3bab7-0x3bb1a): whether an NPC hit `victim` while it docks
/// (`_AIShipIsDocking` @0x83f95, sub-goal 0xf whatever the goal): with a
/// ship it plunders, under goal 0xd or 4, or with the player it repairs,
/// under goal 0xf, in sub 0xf ([`Npc::docking`] in [`DockSub::Dock`], see
/// [`plunder_dock`](crate::traffic::plunder_dock) and
/// [`repair_dock`](crate::traffic::repair_dock)).
fn docking(victim: &Npc, attacker: Attacker) -> bool {
    matches!(attacker, Attacker::Npc(_))
        && victim
            .docking
            .is_some_and(|docking| docking.sub == DockSub::Dock)
}

/// The reaction block's entry test (@0x3bb20-0x3bb74): whether a
/// fighter-class attacker among `around` hit `victim`, no fighter, while
/// it has a target and attacks.
fn entry_test(victim: &Npc, attacker: Attacker, around: &Surroundings) -> bool {
    let class = match attacker {
        Attacker::Player => around.player.map(|player| player.class),
        Attacker::Npc(npc) => Some(npc.class),
        Attacker::None => None,
    };
    class == Some(EscortClass::Fighter)
        && victim.class != EscortClass::Fighter
        && victim.target.is_some()
        && victim.goal.attacking().is_some()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{NEUTRAL, PIRATES, POLICE, TRADERS, blessing_of, govts, n, ship};
    use crate::ai::{Goal, Help, PlayerSide};
    use crate::bay::Carrier;
    use crate::catalog::{EscortClass, GovtId, GovtRecord, Penalties};
    use crate::combat::fixture::strike;
    use crate::combat::{Nominal, ShipRef};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::geometry::Vec2;
    use crate::govt::Governments;
    use crate::traffic::npc::{AiType, Npc, NpcId};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::govt;

    const P: ShipRef = ShipRef::Player;

    /// NPC `id` of `govt` (none for an independent), a warship at
    /// (`x`, `y`).
    fn at(id: u32, govt: Option<GovtId>, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, x, y);
        npc.govt = govt;
        npc
    }

    /// NPC `id` of `govt` at the centre.
    fn of(id: u32, govt: Option<GovtId>) -> Npc {
        at(id, govt, 0.0, 0.0)
    }

    /// `npc` as the player's hired escort.
    fn escorting(mut npc: Npc) -> Npc {
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        npc
    }

    /// `npc` led by NPC `lead`.
    fn led(mut npc: Npc, lead: u32) -> Npc {
        npc.leader = Some(NpcId(lead));
        npc
    }

    /// `npc` as a fighter the player launched.
    fn players_fighter(npc: Npc) -> Npc {
        let mut npc = escorting(npc);
        npc.carrier = Some(Carrier {
            ship: P,
            window: 0.0,
            reach: 0.0,
        });
        npc
    }

    /// The fixture's governments, the traders' `ShootPenalty` `shoot`.
    fn shooting(shoot: i16) -> Governments {
        let traders = GovtRecord {
            crime_tol: 6,
            penalties: Penalties {
                shoot,
                ..Penalties::default()
            },
            classes: [1, -1, -1, -1],
            ..govt(140)
        };
        Governments::new([
            traders,
            GovtRecord {
                crime_tol: 6,
                allies: [1, -1, -1, -1],
                classes: [2, -1, -1, -1],
                ..govt(141)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                classes: [3, -1, -1, -1],
                ..govt(142)
            },
            govt(143),
        ])
    }

    /// Where the flag is weighed: `npcs`, among `govts`, the player at
    /// the centre as `player` leaves it, in a traders' system where its
    /// record is `record`.
    struct Scene<'a> {
        npcs: &'a [Npc],
        govts: &'a Governments,
        player: PlayerSide,
        system_govt: Option<GovtId>,
        record: i16,
        rules: FlagRules,
    }

    impl<'a> Scene<'a> {
        fn new(npcs: &'a [Npc], govts: &'a Governments) -> Self {
            Self {
                npcs,
                govts,
                player: crate::ai::fixture::player(0.0, 0.0),
                system_govt: Some(TRADERS),
                record: 0,
                rules: FlagRules::default(),
            }
        }

        /// The verdict on `hit`, drawing `draws`, and the draws asked.
        fn judge(&self, hit: &Strike, draws: &[u32]) -> (Verdict, Vec<u32>) {
            let around = Surroundings {
                player: Some(self.player),
                govts: self.govts,
                system_govt: self.system_govt,
                record: self.record,
                ..Surroundings::new(&[], self.npcs)
            };
            let mut chance = Draws::of(draws);
            let verdict = verdict(hit, &around, self.rules, &mut chance);
            (verdict, chance.asked)
        }

        /// Whether `hit` is answered, with no draw made.
        fn answers(&self, hit: &Strike) -> bool {
            let (verdict, asked) = self.judge(hit, &[]);
            assert!(asked.is_empty(), "no draw: {hit:?} drew {asked:?}");
            verdict.answers
        }
    }

    /// `hit` astray: on a ship it was not meant for.
    fn astray(hit: Strike) -> Strike {
        Strike {
            intended: false,
            ..hit
        }
    }

    // The entry gate and the self-hit.

    #[test]
    fn only_a_direct_hit_on_an_npc_can_be_answered() {
        let govts = govts(0, 0);
        let npcs = [of(1, Some(TRADERS)), of(2, Some(PIRATES))];
        let scene = Scene::new(&npcs, &govts);
        assert!(scene.answers(&strike(n(1), n(2))));
        let blast = Strike {
            direct: false,
            ..strike(n(1), n(2))
        };
        assert_eq!(scene.judge(&blast, &[]), (Verdict::default(), vec![]));
        let players_blast = Strike {
            weight: 10.0,
            ..astray(Strike {
                direct: false,
                ..strike(n(1), P)
            })
        };
        assert_eq!(
            scene.judge(&players_blast, &[]),
            (Verdict::default(), vec![]),
            "no stray provocation from a blast"
        );
        assert!(!scene.answers(&strike(P, n(2))), "the player hit");
        assert!(!scene.answers(&strike(n(9), n(2))), "a ship gone");
    }

    #[test]
    fn a_ship_hitting_itself_never_answers_and_adds_nothing() {
        let govts = govts(0, 0);
        let npcs = [of(1, Some(TRADERS))];
        let scene = Scene::new(&npcs, &govts);
        let own = Strike {
            weight: 10.0,
            ..strike(n(1), n(1))
        };
        assert_eq!(scene.judge(&own, &[]), (Verdict::default(), vec![]));
        assert_eq!(scene.judge(&astray(own), &[]), (Verdict::default(), vec![]));
    }

    // Kin and leaders.

    #[test]
    fn a_ship_of_another_government_is_answered_and_one_of_its_own_or_no_ship_is_not() {
        let govts = govts(0, 0);
        let npcs = [
            of(1, Some(TRADERS)),
            of(2, Some(PIRATES)),
            of(3, Some(TRADERS)),
            of(4, None),
            led(of(5, Some(TRADERS)), 3),
        ];
        let mut scene = Scene::new(&npcs, &govts);
        assert!(scene.answers(&strike(n(1), n(2))));
        assert!(scene.answers(&strike(n(1), n(4))), "an independent");
        assert!(!scene.answers(&strike(n(1), n(3))));
        let by_none = Strike {
            by: None,
            ..strike(n(1), n(2))
        };
        assert!(!scene.answers(&by_none));
        assert!(
            !scene.answers(&strike(n(1), n(9))),
            "a ship gone is no ship"
        );
        // A record of 2 x CrimeTol would forgive the player and its
        // escorts, which a ship gone is not.
        scene.record = 12;
        assert!(
            scene.answers(&strike(n(5), n(9))),
            "a ship gone is not a led ship's leader"
        );
    }

    #[test]
    fn two_independent_npcs_let_each_others_hits_pass_unless_independents_are_no_government() {
        let govts = govts(0, 0);
        let npcs = [of(1, None), of(2, None)];
        let mut scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))), "-1 is -1");
        scene.rules.independent_kin = RuleSource::Bible;
        assert!(scene.answers(&strike(n(1), n(2))));
        let by_none = Strike {
            by: None,
            ..strike(n(1), n(2))
        };
        assert!(!scene.answers(&by_none), "still no ship");
    }

    #[test]
    fn a_led_ship_answers_its_own_government_unless_hit_by_its_unled_leader() {
        // Independents, whose kin (-1 is -1) the leaders branch overrides.
        let govts = govts(0, 0);
        let npcs = [
            led(of(1, None), 2),
            of(2, None),
            led(of(3, None), 4),
            of(4, None),
            of(5, None),
        ];
        let scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))), "its unled leader");
        assert!(scene.answers(&strike(n(1), n(5))), "another independent");
        let mut by_led_leader = npcs.clone();
        by_led_leader[1].leader = Some(NpcId(4));
        let scene = Scene::new(&by_led_leader, &govts);
        assert!(scene.answers(&strike(n(1), n(2))), "its leader, led");
        let by_none = Strike {
            by: None,
            ..strike(n(1), n(2))
        };
        assert!(scene.answers(&by_none), "no ship is not its leader");
        assert!(scene.answers(&strike(n(1), n(3))), "a ship led by another");
    }

    // The record.

    #[test]
    fn a_player_whose_record_is_twice_the_victims_crime_tol_goes_unanswered_and_one_below_does_not()
    {
        let govts = govts(0, 0);
        let escort = escorting(of(2, Some(NEUTRAL)));
        let stranger = of(3, Some(NEUTRAL));
        let escorts_escort = led(of(4, Some(NEUTRAL)), 2);
        let npcs = [of(1, Some(TRADERS)), escort, stranger, escorts_escort];
        let mut scene = Scene::new(&npcs, &govts);
        for by in [P, n(2)] {
            scene.record = 12;
            assert!(!scene.answers(&strike(n(1), by)), "{by:?}: 2 x 6");
            scene.record = 11;
            assert!(scene.answers(&strike(n(1), by)), "{by:?}: below");
        }
        scene.record = 12;
        assert!(scene.answers(&strike(n(1), n(3))), "not led by the player");
        assert!(scene.answers(&strike(n(1), n(4))), "an escort's escort");
        scene.player.target = Some(n(1));
        assert!(scene.answers(&strike(n(1), P)), "the player's target");
        scene.player.target = Some(n(3));
        assert!(!scene.answers(&strike(n(1), P)), "another target");
        scene.player.target = None;
        scene.system_govt = None;
        assert!(scene.answers(&strike(n(1), P)), "an independent system");
        let mut independent = npcs.clone();
        independent[0].govt = None;
        let mut scene = Scene::new(&independent, &govts);
        scene.record = 12;
        assert!(scene.answers(&strike(n(1), P)), "an independent victim");
    }

    #[test]
    fn a_ship_of_the_victims_own_government_is_never_answered_whatever_the_record() {
        let govts = govts(0, 0);
        // Led, so the leaders branch sets the flag that kin cleared.
        let npcs = [
            led(of(1, Some(TRADERS)), 2),
            of(2, Some(TRADERS)),
            of(3, Some(TRADERS)),
        ];
        let mut scene = Scene::new(&npcs, &govts);
        for record in [-100, 0, 100] {
            scene.record = record;
            assert!(!scene.answers(&strike(n(1), n(3))), "{record}");
        }
        let mut independent = npcs.clone();
        for npc in &mut independent {
            npc.govt = None;
        }
        let scene = Scene::new(&independent, &govts);
        assert!(scene.answers(&strike(n(1), n(3))), "independents");
    }

    #[test]
    fn an_independent_victim_answers_the_player_and_its_escorts() {
        let govts = govts(0, 0);
        let npcs = [
            of(1, None),
            escorting(of(2, None)),
            of(3, Some(PIRATES)),
            of(4, None),
        ];
        let scene = Scene::new(&npcs, &govts);
        assert!(scene.answers(&strike(n(1), P)));
        assert!(scene.answers(&strike(n(1), n(2))), "its escort");
        assert!(scene.answers(&strike(n(1), n(3))), "a governed ship");
        assert!(!scene.answers(&strike(n(1), n(4))), "an independent");
    }

    #[test]
    fn a_hit_on_a_ship_with_no_ai_is_never_answered_and_adds_no_stray_provocation() {
        let govts = govts(0, 0);
        let mut hulk = of(1, None);
        hulk.no_ai = true;
        let npcs = [hulk, of(2, Some(PIRATES))];
        let scene = Scene::new(&npcs, &govts);
        let weighty = |by| Strike {
            weight: 10.0,
            ..astray(strike(n(1), by))
        };
        for hit in [strike(n(1), P), strike(n(1), n(2)), weighty(P)] {
            assert_eq!(
                scene.judge(&hit, &[]),
                (Verdict::default(), vec![]),
                "{hit:?}"
            );
        }
    }

    // No damage.

    #[test]
    fn a_hit_of_no_mass_or_energy_damage_is_not_answered_but_a_lock_is() {
        let govts = govts(0, 0);
        let npcs = [of(1, Some(TRADERS)), of(2, Some(PIRATES))];
        let scene = Scene::new(&npcs, &govts);
        let passing = |mass: f32, energy: f32, lock: bool| Strike {
            nominal: Nominal { mass, energy },
            lock,
            ..strike(n(1), n(2))
        };
        assert!(!scene.answers(&passing(0.0, 0.0, false)));
        assert!(scene.answers(&passing(0.0, 1.0, false)));
        assert!(scene.answers(&passing(1.0, 0.0, false)));
        assert!(!scene.answers(&passing(-1.0, -1.0, false)));
        assert!(!scene.answers(&passing(-31_999.0, 0.0, false)));
        assert!(!scene.answers(&passing(0.0, -31_999.0, false)));
        assert!(scene.answers(&passing(0.0, 0.0, true)), "a lock");
        assert!(
            scene.answers(&passing(-32_000.0, 0.0, false)),
            "the lock's sentinel"
        );
        assert!(scene.answers(&passing(0.0, -32_000.0, false)));
    }

    // The aim.

    #[test]
    fn the_players_stray_hit_on_its_target_goes_on_to_the_roll() {
        let govts = govts(0, 0);
        let mut victim = of(1, Some(TRADERS));
        victim.stray = 60.0;
        let npcs = [victim];
        let mut scene = Scene::new(&npcs, &govts);
        scene.player.target = Some(n(1));
        let hit = astray(strike(n(1), P));
        assert_eq!(
            scene.judge(&hit, &[0]),
            (
                Verdict {
                    answers: true,
                    stray: 0.0
                },
                vec![4]
            )
        );
        assert!(!scene.judge(&hit, &[1]).0.answers);
        assert!(scene.answers(&strike(n(1), P)), "intended");
    }

    #[test]
    fn the_players_stray_hit_on_an_enemy_of_its_target_is_not_answered_and_rolls_nothing() {
        let govts = govts(0, 0);
        let mut victim = of(1, Some(TRADERS));
        victim.stray = 60.0;
        let npcs = [victim, of(2, Some(PIRATES)), of(3, Some(NEUTRAL))];
        let mut scene = Scene::new(&npcs, &govts);
        scene.player.target = Some(n(2));
        let hit = astray(strike(n(1), P));
        assert!(!scene.answers(&hit), "the pirate its enemy");
        assert!(scene.answers(&strike(n(1), P)), "intended");
        scene.player.target = Some(n(3));
        assert_eq!(
            scene.judge(&hit, &[0]),
            (
                Verdict {
                    answers: true,
                    stray: 0.0
                },
                vec![4]
            )
        );
        scene.player.target = Some(n(9));
        assert_eq!(scene.judge(&hit, &[0]).1, [4], "a target gone");
        scene.player.target = None;
        assert_eq!(scene.judge(&hit, &[0]).1, [4], "no target");
    }

    #[test]
    fn an_npcs_stray_hit_on_a_ship_it_does_not_target_is_not_answered() {
        let govts = govts(0, 0);
        let mut pirate = of(2, Some(PIRATES));
        pirate.target = Some(n(3));
        let npcs = [of(1, Some(TRADERS)), pirate, of(3, Some(TRADERS))];
        let scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&astray(strike(n(1), n(2)))));
        assert!(scene.answers(&strike(n(1), n(2))), "intended");
        let mut on_target = npcs.clone();
        on_target[1].target = Some(n(1));
        let scene = Scene::new(&on_target, &govts);
        assert!(scene.answers(&astray(strike(n(1), n(2)))), "its target");
        let by_none = astray(Strike {
            by: None,
            ..strike(n(1), n(2))
        });
        let led_victim = [led(of(1, Some(TRADERS)), 3), of(3, Some(TRADERS))];
        let scene = Scene::new(&led_victim, &govts);
        assert!(scene.answers(&by_none), "no ship aims at nothing");
    }

    #[test]
    fn a_shoot_penalty_above_0_skips_the_roll_and_0_forgives_3_in_4() {
        let lenient = shooting(0);
        let strict = shooting(5);
        let mut victim = of(1, Some(TRADERS));
        victim.stray = 60.0;
        let mut pirate = of(2, Some(PIRATES));
        pirate.target = Some(n(1));
        let npcs = [victim, pirate];
        let hit = astray(strike(n(1), P));
        let scene = Scene::new(&npcs, &strict);
        assert!(scene.answers(&hit), "ShootPenalty 5: no roll");
        let scene = Scene::new(&npcs, &lenient);
        assert_eq!(
            scene.judge(&hit, &[0]),
            (
                Verdict {
                    answers: true,
                    stray: 0.0
                },
                vec![4]
            )
        );
        assert!(!scene.judge(&hit, &[1]).0.answers);
        assert!(!scene.judge(&hit, &[3]).0.answers);
        assert!(scene.answers(&astray(strike(n(1), n(2)))), "an NPC's hit");
        let mut independent = npcs.clone();
        independent[0].govt = None;
        let scene = Scene::new(&independent, &lenient);
        assert!(scene.answers(&hit), "an independent victim");
        let mut scene = Scene::new(&npcs, &strict);
        scene.rules.shoot_penalty = RuleSource::Bible;
        assert_eq!(scene.judge(&hit, &[1]), (Verdict::default(), vec![4]));
        assert!(scene.judge(&hit, &[0]).0.answers);
    }

    #[test]
    fn the_players_stray_hit_is_answered_only_from_50_stray_provocation() {
        let govts = shooting(5);
        let mut victim = of(1, Some(TRADERS));
        victim.stray = 49.9;
        let npcs = [victim];
        let scene = Scene::new(&npcs, &govts);
        let hit = astray(strike(n(1), P));
        assert!(!scene.answers(&hit));
        assert!(scene.answers(&strike(n(1), P)), "intended");
        let mut provoked = npcs.clone();
        provoked[0].stray = 50.0;
        assert!(Scene::new(&provoked, &govts).answers(&hit));
        provoked[0].stray = 49.0;
        let weighty = Strike {
            weight: 10.0,
            ..hit
        };
        assert_eq!(
            Scene::new(&provoked, &govts).judge(&weighty, &[]),
            (
                Verdict {
                    answers: false,
                    stray: 17.5
                },
                vec![]
            ),
            "read before this hit's add"
        );
        assert_eq!(STRAY_THRESHOLD, 50.0);
    }

    #[test]
    fn a_players_stray_hit_adds_one_and_three_quarter_times_its_weight_whatever_the_flag() {
        let govts = shooting(5);
        let mut victim = of(1, Some(TRADERS));
        victim.stray = 60.0;
        let mut pirate = of(2, Some(PIRATES));
        pirate.target = Some(n(1));
        let npcs = [victim, pirate];
        let scene = Scene::new(&npcs, &govts);
        let weighty = |by| Strike {
            weight: 10.0,
            ..astray(strike(n(1), by))
        };
        assert_eq!(
            scene.judge(&weighty(P), &[]).0,
            Verdict {
                answers: true,
                stray: 17.5
            }
        );
        let no_damage = Strike {
            nominal: Nominal::default(),
            ..weighty(P)
        };
        assert_eq!(
            scene.judge(&no_damage, &[]).0,
            Verdict {
                answers: false,
                stray: 17.5
            }
        );
        let intended = Strike {
            intended: true,
            ..weighty(P)
        };
        assert_eq!(scene.judge(&intended, &[]).0.stray, 0.0);
        assert_eq!(scene.judge(&weighty(n(2)), &[]).0.stray, 0.0, "an NPC's");
        assert_eq!(STRAY_WEIGHT, 1.75);
    }

    // After the add.

    #[test]
    fn the_players_fighter_near_the_player_draws_no_answer_from_a_ship_targeting_the_player() {
        let govts = govts(0, 0);
        let mut victim = at(1, Some(TRADERS), 320.0, -320.0);
        victim.target = Some(P);
        let npcs = [
            victim,
            players_fighter(of(2, Some(NEUTRAL))),
            escorting(of(3, Some(NEUTRAL))),
        ];
        let scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))), "at 320 on each axis");
        assert!(scene.answers(&strike(n(1), n(3))), "a hired escort");
        for (x, y) in [(321.0, 0.0), (0.0, -321.0), (-321.0, 0.0), (0.0, 321.0)] {
            let mut far = npcs.clone();
            far[0].state.position = Vec2::new(x, y);
            assert!(
                Scene::new(&far, &govts).answers(&strike(n(1), n(2))),
                "({x}, {y})"
            );
        }
        let mut elsewhere = npcs.clone();
        elsewhere[0].target = Some(n(3));
        assert!(
            Scene::new(&elsewhere, &govts).answers(&strike(n(1), n(2))),
            "not targeting the player"
        );
        let mut player_moved = Scene::new(&npcs, &govts);
        player_moved.player.state.position = Vec2::new(-1.0, 0.0);
        assert!(player_moved.answers(&strike(n(1), n(2))), "from the player");
        assert_eq!(ESCORT_REACH, 320.0);
    }

    #[test]
    fn ships_of_one_leader_let_each_others_hits_pass() {
        let govts = govts(0, 0);
        let npcs = [
            escorting(of(1, Some(TRADERS))),
            escorting(of(2, Some(NEUTRAL))),
            led(of(3, Some(TRADERS)), 5),
            led(of(4, Some(PIRATES)), 5),
            of(5, Some(NEUTRAL)),
            of(6, Some(NEUTRAL)),
        ];
        let scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))), "two of the player's");
        assert!(!scene.answers(&strike(n(3), n(4))), "two of an NPC's");
        assert!(scene.answers(&strike(n(1), n(6))), "an unled ship");
        assert!(scene.answers(&strike(n(3), n(6))));
    }

    #[test]
    fn a_stray_hit_by_a_blessed_ally_of_the_player_is_not_answered_but_an_intended_one_is() {
        let govts = govts(0, 0);
        let aiming = |mut npc: Npc| {
            npc.target = Some(n(1));
            npc
        };
        let npcs = [
            of(1, Some(TRADERS)),
            aiming(escorting(of(2, Some(POLICE)))),
            aiming(of(3, Some(POLICE))),
            aiming(led(of(4, Some(POLICE)), 2)),
            aiming(escorting(of(5, None))),
            aiming(Npc {
                mission: Some(crate::traffic::npc::MissionMark {
                    slot: 0,
                    behav: crate::traffic::ShipBehav::EscortPlayer,
                    formation: Some(FormationSlot::default()),
                    order: None,
                }),
                ..of(6, Some(POLICE))
            }),
        ];
        let mut scene = Scene::new(&npcs, &govts);
        assert!(scene.answers(&astray(strike(n(1), n(2)))), "unblessed");
        scene.player.blessing = blessing_of(POLICE);
        assert!(!scene.answers(&astray(strike(n(1), n(2)))));
        assert!(scene.answers(&strike(n(1), n(2))), "intended");
        assert!(scene.answers(&astray(strike(n(1), n(3)))), "not allied");
        assert!(
            !scene.answers(&astray(strike(n(1), n(4)))),
            "led by the player's escort"
        );
        assert!(
            scene.answers(&astray(strike(n(1), n(5)))),
            "an independent escort"
        );
        assert!(
            !scene.answers(&astray(strike(n(1), n(6)))),
            "a mission escort of a blessing government"
        );
    }

    #[test]
    fn fighters_of_one_grand_fleet_let_each_others_hits_pass() {
        let govts = govts(0, 0);
        let npcs = [
            // The player's two escorts, each with a fighter.
            escorting(of(1, Some(NEUTRAL))),
            escorting(of(2, Some(NEUTRAL))),
            led(of(3, Some(TRADERS)), 1),
            led(of(4, Some(PIRATES)), 2),
            // An NPC lead's two escorts, each with a fighter.
            of(5, Some(NEUTRAL)),
            led(of(6, Some(NEUTRAL)), 5),
            led(of(7, Some(NEUTRAL)), 5),
            led(of(8, Some(TRADERS)), 6),
            led(of(9, Some(PIRATES)), 7),
            // A fighter of an unled ship.
            of(10, Some(NEUTRAL)),
            led(of(11, Some(PIRATES)), 10),
        ];
        let scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(3), n(4))), "the player's fleet");
        assert!(!scene.answers(&strike(n(8), n(9))), "an NPC's fleet");
        assert!(scene.answers(&strike(n(3), n(11))), "another fleet");
        assert!(scene.answers(&strike(n(3), n(9))));
        assert!(
            scene.answers(&strike(n(1), n(4))),
            "a ship the player leads is no fighter of its fleet"
        );
    }

    #[test]
    fn a_ship_repairing_the_player_ignores_an_npcs_hit_but_not_the_players() {
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let govts = govts(0, 0);
        // Docked with the player in sub 0xf: `_AIShipIsDocking` @0x83f95.
        let repairing = |docking: Option<DockSub>| {
            let mut helper = at(1, Some(TRADERS), 40.0, 0.0);
            helper.goal = Goal::Assist(Help::Repair);
            helper.docking = docking.map(|sub| Docking { ship: P, sub });
            [helper, of(2, Some(PIRATES))]
        };
        let docked = repairing(Some(DockSub::Dock));
        let scene = Scene::new(&docked, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))));
        assert!(scene.answers(&strike(n(1), P)), "the player's hit");
        for (npcs, why) in [
            (repairing(Some(DockSub::Close)), "closing (sub 0xb)"),
            (repairing(None), "no docking, whatever its reach"),
        ] {
            assert!(
                Scene::new(&npcs, &govts).answers(&strike(n(1), n(2))),
                "{why}"
            );
        }
        let mut refuelling = repairing(None);
        refuelling[0].goal = Goal::Assist(Help::Refuel);
        assert!(Scene::new(&refuelling, &govts).answers(&strike(n(1), n(2))));
    }

    #[test]
    fn a_pirate_docking_to_plunder_ignores_an_npcs_hit_but_not_the_players() {
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let govts = govts(0, 0);
        let docking = |goal: Goal, sub: DockSub| {
            let mut pirate = at(1, Some(TRADERS), 40.0, 0.0);
            pirate.goal = goal;
            pirate.docking = Some(Docking { ship: n(5), sub });
            [pirate, of(2, Some(PIRATES)), of(5, Some(TRADERS))]
        };
        let plunder = Goal::Plunder {
            ship: n(5),
            hopeless: false,
        };
        for goal in [plunder, Goal::Attack(n(5))] {
            let npcs = docking(goal, DockSub::Dock);
            let scene = Scene::new(&npcs, &govts);
            assert!(!scene.answers(&strike(n(1), n(2))), "{goal:?}: sub 0xf");
            assert!(
                scene.answers(&strike(n(1), P)),
                "{goal:?}: the player's hit"
            );
        }
        for sub in [DockSub::Approach, DockSub::Close] {
            let npcs = docking(plunder, sub);
            assert!(
                Scene::new(&npcs, &govts).answers(&strike(n(1), n(2))),
                "{sub:?}"
            );
        }
    }

    #[test]
    fn a_fighter_class_attacker_draws_no_answer_from_a_busy_ship() {
        let govts = govts(0, 0);
        let mut busy = of(1, Some(TRADERS));
        busy.target = Some(n(3));
        busy.goal = Goal::Attack(n(3));
        let mut fighter = of(2, Some(PIRATES));
        fighter.class = EscortClass::Fighter;
        let npcs = [busy, fighter, of(3, Some(PIRATES)), of(4, Some(PIRATES))];
        let mut scene = Scene::new(&npcs, &govts);
        assert!(!scene.answers(&strike(n(1), n(2))));
        assert!(scene.answers(&strike(n(1), n(4))), "a freighter");
        scene.player.class = EscortClass::Fighter;
        assert!(!scene.answers(&strike(n(1), P)), "the player in a fighter");
        let mut idle = npcs.clone();
        idle[0].goal = Goal::Idle;
        assert!(Scene::new(&idle, &govts).answers(&strike(n(1), n(2))));
        let mut fleeing = npcs.clone();
        fleeing[0].goal = Goal::Flee(n(3));
        assert!(Scene::new(&fleeing, &govts).answers(&strike(n(1), n(2))));
        let mut untargeted = npcs.clone();
        untargeted[0].target = None;
        assert!(Scene::new(&untargeted, &govts).answers(&strike(n(1), n(2))));
        let mut itself = npcs.clone();
        itself[0].class = EscortClass::Fighter;
        assert!(Scene::new(&itself, &govts).answers(&strike(n(1), n(2))));
    }
}
