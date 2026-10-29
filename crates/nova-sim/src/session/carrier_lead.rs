//! Tests of an NPC carrier's fighter as `_LaunchFighter` leaves it: led
//! by the carrier (@0x3d866-0x3d870), its target the carrier's unless that
//! is the carrier or a ship the carrier leads (@0x3dd0f-0x3dd47), goal 0
//! (`_AISetup` @0x7f3e8), gliding its bay's `Count` until a think ends it
//! (@0x3d843-0x3d85b), and still of its carrier's lead's side for hits,
//! sparing and point defence (`_ShipsShareParent` @0x7926-0x79c1); the
//! player's fighters glide too. A player's fighter never launches under
//! Return to Hangar, which a joiner's class order drops (`_EscortAI`
//! @0x83b18), so only its Hold glide is pinned.

#![allow(clippy::float_cmp)]

use std::rc::Rc;

use super::Session;
use super::fighters::tests::{
    BAY, VIPER, carrier_npc, carrying, fight, fleet_npcs, flying_by, launch, pilot, pirate, raided,
    raiding_ai, vipers,
};
use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Sortie};
use crate::escort::{EscortCommand, EscortGroup};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::{Npc, NpcId};

/// The Viper Bay's `Count`: its fighter's glide, in ticks.
const GLIDE: u32 = 30;

/// A sortie of a Viper from NPC `carrier`'s bay, at rest at the centre.
fn sortie(carrier: NpcId) -> Sortie {
    Sortie {
        carrier: ShipRef::Npc(carrier),
        bay: BAY,
        ship: VIPER,
        from: ShipState::default(),
        heading: 0.0,
        target: None,
    }
}

#[test]
fn a_led_carriers_fighter_is_led_by_the_carrier_and_of_its_leads_side() {
    let catalog = carrying();
    let (mut session, raider, lead) = raided(&catalog);
    let id = session
        .launch_fighter(&sortie(raider), &mut crate::chance::NeverFires)
        .expect("launched");
    let fighter = session.space.npc(id).expect("there");
    assert_eq!(fighter.leader, Some(raider), "its carrier, not the lead");
    let (_, terms) = session.fight_terms();
    let fleets: Vec<ShipRef> = terms.iter().map(|terms| terms.fleet).collect();
    let ids: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
    assert_eq!(ids, [lead, raider, id]);
    assert_eq!(fleets, [ShipRef::Npc(lead); 3], "one side, the lead's");
}

#[test]
fn its_target_is_its_carriers_unless_that_is_the_carrier_or_a_ship_the_carrier_leads() {
    // The original's narrower test (@0x3dd0f-0x3dd47): the carrier's own
    // lead and that lead's other escort are kept.
    let catalog = carrying();
    let (mut session, raider, lead) = raided(&catalog);
    let mut wing = crate::testkit::npc(0, crate::stats::ShipStats::default());
    wing.leader = Some(raider);
    let wing = session.space.traffic.add_npc(wing);
    let mut other = crate::testkit::npc(0, crate::stats::ShipStats::default());
    other.leader = Some(lead);
    let other = session.space.traffic.add_npc(other);
    let stranger = pirate(&mut session, 500.0, 500.0);
    for (target, kept) in [
        (ShipRef::Npc(raider), false),
        (ShipRef::Npc(wing), false),
        (ShipRef::Npc(lead), true),
        (ShipRef::Npc(other), true),
        (ShipRef::Npc(stranger), true),
        (ShipRef::Player, true),
    ] {
        session.space.npc_mut(raider).expect("there").target = Some(target);
        let id = session
            .launch_fighter(&sortie(raider), &mut crate::chance::NeverFires)
            .expect("launched");
        let fighter = session.space.npc(id).expect("there");
        assert_eq!(fighter.target, kept.then_some(target), "{target:?}");
        assert_eq!(fighter.goal, Goal::Idle, "goal 0, {target:?}");
        session.space.traffic.remove(id);
    }
}

/// Keeps its goal and fires as any ship does at what it attacks: a ship
/// that attacks whatever its lead does.
#[derive(Debug)]
struct Shooting;

impl Behaviour for Shooting {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        crate::ai::fire::trigger(npc, around, crate::ai::fire::FireRules::default())
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.attacking()
    }
}

#[test]
fn shots_between_the_fighter_and_its_carriers_leads_other_escort_still_miss() {
    // `_ShotCanHitShip` asks `_ShipsShareParent` (@0x44b53), which walks
    // each ship's parents to their root: the fighter's carrier's lead.
    let catalog = carrying();
    let (mut session, raider, lead) = raided(&catalog);
    let id = session
        .launch_fighter(&sortie(raider), &mut crate::chance::NeverFires)
        .expect("launched");
    let turret = crate::catalog::WeaponRecord {
        guidance: 4,
        reload: 5,
        speed: 1000,
        count: 30,
        energy_dmg: 5,
        mass_dmg: 2,
        ..crate::testkit::weapon(140)
    };
    let mut gunner = crate::session::fighters::tests::armed_npc(0.0, -60.0, turret);
    gunner.leader = Some(lead);
    gunner.goal = Goal::Attack(ShipRef::Npc(id));
    gunner.target = Some(ShipRef::Npc(id));
    let gunner = session.space.traffic.add_npc(gunner);
    let fighter = session.space.npc_mut(id).expect("there");
    fighter.state.position = Vec2::new(0.0, 0.0);
    fighter.state.velocity = Vec2::ZERO;
    fighter.ai_wait = 1000;
    let full = fighter.reserves.shield.now;
    let ai = NovaAi::default()
        .with_fleet_escorts(Rc::new(Shooting))
        .with(crate::traffic::npc::AiType::Interceptor, Rc::new(Shooting));
    let mut fired = false;
    for _ in 0..30 {
        fight(&mut session, &catalog, &ai);
        fired |= session
            .take_combat_events()
            .iter()
            .any(|event| matches!(event, crate::combat::CombatEvent::Fired { ship, .. } if *ship == ShipRef::Npc(gunner)));
    }
    assert!(fired, "its escort fires at it");
    assert_eq!(
        session.space.npc(id).expect("there").reserves.shield.now,
        full,
        "and never hits it"
    );
}

/// [`raided`], its carrier in berth 5: its fighter launches into a free
/// berth below the carrier's, which its launch frame's AI loop has passed,
/// so it thinks first in the next frame (see the `launch_frame` module).
fn raided_below(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId) {
    let rules = Rulebook::default().with_override(RuleKey::FighterLaunch, RuleSource::Engine);
    let mut session = flying_by(catalog, pilot(catalog, 0, Vec::new()), rules);
    let lead = pirate(&mut session, 2000.0, 2000.0);
    let npc = session.space.npc_mut(lead).expect("there");
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    let raider = session.space.traffic.add_npc_at(carrier_npc(lead, 2), 5);
    (session, raider, lead)
}

/// The fighter of [`raided_below`]'s carrier launched by a pass of
/// [`raiding_ai`]: the session, the fighter, the carrier and its lead.
fn npc_launched(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId, NpcId) {
    let (mut session, raider, lead) = raided_below(catalog);
    fight(&mut session, catalog, &raiding_ai());
    let launched = vipers(&session);
    assert_eq!(launched.len(), 1, "one launched");
    let fighter = &launched[0];
    // It moves in its launch frame's ship loop (@0x4445c-0x444c4), which
    // counts its glide down once.
    assert_eq!((fighter.goal, fighter.ai_wait), (Goal::Idle, GLIDE - 1));
    // `_AISetup` @0x7f40c.
    assert_eq!(fighter.order_word, crate::fleet::OrderWord::NONE);
    (session, fighter.id, raider, lead)
}

/// Whether `id` holds its heading and velocity, moving on by it, through
/// `ticks` fights of `ai`, its glide counting down, and its glide left.
fn coasts(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    ai: &NovaAi,
    id: NpcId,
    ticks: u32,
) -> (bool, u32) {
    let mut held = true;
    for _ in 0..ticks {
        let before = session.space.npc(id).expect("there").state;
        fight(session, catalog, ai);
        let after = session.space.npc(id).expect("there").state;
        held &= after.heading == before.heading
            && after.velocity == before.velocity
            && after.position == before.position + before.velocity;
    }
    (held, session.space.npc(id).expect("there").ai_wait)
}

#[test]
fn an_npcs_fighter_ends_its_glide_at_its_first_think_and_attacks_once_its_carrier_orders_it() {
    // It launches with order -1 (`_AISetup` @0x7f40c): its formation block
    // ends the glide (@0x83ea8-0x83eb0), and it keeps formation until its
    // carrier's next order frame gives it Attack, its carrier attacking
    // with its shield full (`_AIIssueEscortOrders` @0x89a3c-0x89a53).
    let catalog = carrying();
    let (mut session, id, raider, _) = npc_launched(&catalog);
    fight(&mut session, &catalog, &raiding_ai());
    let fighter = session.space.npc(id).expect("there");
    assert_eq!(fighter.ai_wait, 0, "its formation block ends it");
    assert!(
        matches!(fighter.goal, Goal::Follow { lead, .. } if lead == raider),
        "{:?}",
        fighter.goal
    );
    let mut ticks = 0;
    while session.space.npc(id).expect("there").order_word == crate::fleet::OrderWord::NONE {
        fight(&mut session, &catalog, &raiding_ai());
        ticks += 1;
        assert!(ticks < 8, "its carrier orders it within 8 frames");
    }
    let fighter = session.space.npc(id).expect("there");
    assert_eq!(fighter.order_word, crate::fleet::OrderWord(2));
    assert_eq!(fighter.goal, Goal::Attack(ShipRef::Player), "the same tick");
}

#[test]
fn an_npcs_fighter_told_to_return_coasts_its_glide_out() {
    // Its carrier's lead, and so its carrier, no longer fights, and its
    // carrier has issued it Return, whose block leaves +0x4c
    // (@0x83e79-0x83ea3); an idle carrier issues Return again.
    let catalog = carrying();
    let (mut session, id, raider, lead) = npc_launched(&catalog);
    for ship in [lead, raider] {
        let npc = session.space.npc_mut(ship).expect("there");
        npc.goal = Goal::Idle;
        npc.target = None;
    }
    session.space.npc_mut(id).expect("there").order_word = crate::fleet::OrderWord(3);
    let ai = raiding_ai();
    // The launch frame counted one (see `npc_launched`).
    let (held, left) = coasts(&mut session, &catalog, &ai, id, GLIDE - 2);
    assert!(held, "no turn or thrust while it glides");
    assert_eq!(left, 1);
    assert_eq!(
        session.space.npc(id).expect("there").goal,
        Goal::Dock(ShipRef::Npc(raider))
    );
    fight(&mut session, &catalog, &ai);
    assert_eq!(session.space.npc(id).expect("there").ai_wait, 0);
    let (held, _) = coasts(&mut session, &catalog, &ai, id, 5);
    assert!(!held, "it flies home once its glide is out");
}

/// The player's session of [`carrying`], two Vipers aboard, its fighters'
/// kept order `order`, one launched.
fn players_launched(catalog: &FakePilotCatalog, order: Option<EscortCommand>) -> (Session, NpcId) {
    let mut session = flying_by(catalog, pilot(catalog, 2, Vec::new()), Rulebook::default());
    if let Some(order) = order {
        session.command_escorts(EscortGroup::All, order, &mut crate::chance::NeverFires);
    }
    launch(&mut session, catalog);
    let fighter = &fleet_npcs(&session)[0];
    // Launched in `_HandlePlayer` (@0x62c61), it moves in its launch
    // frame's ship loop (@0x4445c-0x444c4), which counts its glide down
    // once.
    assert_eq!(fighter.ai_wait, GLIDE - 1, "{order:?}");
    (session, fighter.id)
}

#[test]
fn the_players_fighter_in_formation_attacking_or_defending_ends_its_glide_at_its_first_think() {
    let catalog = carrying();
    for order in [
        None,
        Some(EscortCommand::Attack),
        Some(EscortCommand::Defend),
    ] {
        let (mut session, id) = players_launched(&catalog, order);
        fight(&mut session, &catalog, &NovaAi::default());
        assert_eq!(
            session.space.npc(id).expect("there").ai_wait,
            0,
            "{order:?}"
        );
    }
}

#[test]
fn the_players_fighter_told_to_hold_coasts_its_glide_out() {
    let catalog = carrying();
    let (mut session, id) = players_launched(&catalog, Some(EscortCommand::Hold));
    let ai = NovaAi::default();
    // The launch frame counted one (see `players_launched`).
    let (held, left) = coasts(&mut session, &catalog, &ai, id, GLIDE - 2);
    assert!(held, "no turn or thrust while it glides");
    assert_eq!(left, 1);
    assert_eq!(session.space.npc(id).expect("there").goal, Goal::Wait);
}

#[test]
fn an_idle_carrier_launches_its_first_fighter_a_reload_after_it_takes_a_target() {
    // `_HandleShip` holds an idle NPC's bays at their `Reload` every frame
    // (@0x3390a-0x3394e), after `_AILaunchFighter` (@0x8e6fa) in the same
    // frame: the frame it takes a target its bay still reads 60, and its
    // launch comes 60 frames on.
    let catalog = carrying();
    let (mut session, raider, lead) = raided(&catalog);
    for ship in [lead, raider] {
        let npc = session.space.npc_mut(ship).expect("there");
        npc.goal = Goal::Idle;
        npc.target = None;
    }
    let ai = raiding_ai();
    for _ in 0..100 {
        fight(&mut session, &catalog, &ai);
    }
    assert_eq!(vipers(&session), [], "idle, it launches nothing");
    let bay =
        |session: &Session| session.space.npc(raider).expect("there").armament.mounts()[0].reload;
    assert_eq!(bay(&session), 60.0, "held at its Reload");
    for ship in [lead, raider] {
        let npc = session.space.npc_mut(ship).expect("there");
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
    }
    let launched = (1..=100).find(|_| {
        fight(&mut session, &catalog, &ai);
        !vipers(&session).is_empty()
    });
    assert_eq!(launched, Some(61), "60 ticks after the first with a target");
}
