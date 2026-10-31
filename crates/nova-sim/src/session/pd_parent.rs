//! Wiring tests of point defence's missile branch: `Npc::leader_ref` reaches
//! `defence::choose` through a pass, so a ship defends its parent, +0x9a one
//! level up (`_HandleShipPointDefense` @0x39401-0x39426), and not the root
//! of its chain of leads, which its side keeps. `nova-sim-combat` pins the
//! rule itself.

use crate::ai::{Behaviour, Goal, Surroundings};
use crate::catalog::{WeaponId, WeaponRecord};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::Condition;
use crate::combat::weapon::WeaponSpec;
use crate::rulebook::Rulebook;
use crate::session::Session;
use crate::session::escort_loss::{escorted, shot_into_the_escort};
use crate::session::fight_phase_order::{fired, quad};
use crate::session::fighters::tests::{
    QUAD, armed_npc, carrying, fleet, fleet_npcs, flying_by, game_pass, pilot, warship,
};
use crate::session::tests::armed;
use crate::testkit::{FakePilotCatalog, weapon};
use crate::traffic::npc::{Npc, NpcId};

/// A homing missile, 5 pixels a tick for 100.
const MISSILE: WeaponId = WeaponId(134);

/// [`MISSILE`]'s record.
fn missile() -> WeaponRecord {
    WeaponRecord {
        guidance: 1,
        speed: 500,
        count: 100,
        guided_turn: 70,
        durability: 4,
        mass_dmg: 20,
        energy_dmg: 10,
        ..weapon(MISSILE.0)
    }
}

/// Every NPC keeps its goal; `picker` fires its missile at `at`, and only
/// `defender` asks for point defence.
#[derive(Debug)]
struct Aiming {
    defender: NpcId,
    picker: NpcId,
    at: ShipRef,
}

impl Behaviour for Aiming {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }

    fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
        Trigger {
            point_defence: npc.id == self.defender,
            only: (npc.id == self.picker).then_some(MISSILE),
            ..Trigger::default()
        }
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        (npc.id == self.picker).then_some(self.at)
    }
}

/// Whether `defender` fires its Quad on the pass after a new picker of
/// `session`'s, within range of everything at the centre of the scene,
/// fires a missile at `at`.
fn engages(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    defender: NpcId,
    at: ShipRef,
) -> bool {
    let centre = session.space.npc(defender).expect("there").state.position;
    let picker = session
        .space
        .traffic
        .add_npc(armed_npc(centre.x + 120.0, centre.y, missile()));
    let script = Aiming {
        defender,
        picker,
        at,
    };
    session.take_combat_events();
    game_pass(session, catalog, &script);
    let events = session.take_combat_events();
    assert!(fired(&events, ShipRef::Npc(picker), MISSILE), "{events:?}");
    assert!(
        session
            .shots()
            .iter()
            .any(|shot| shot.weapon.id == MISSILE && shot.aimed_at == Some(at)),
        "aimed at {at:?}"
    );
    game_pass(session, catalog, &script);
    fired(&session.take_combat_events(), ShipRef::Npc(defender), QUAD)
}

/// Flying NPCs with no fleet of the player's: `lead`, `carrier` led by it,
/// and a Quad-armed `fighter` led by `carrier`, in that order.
fn led_carrier(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId, NpcId) {
    let mut session = flying_by(catalog, pilot(catalog, 0, Vec::new()), Rulebook::default());
    let lead = session
        .space
        .traffic
        .add_npc(armed_npc(1000.0, 900.0, quad()));
    let mut wing = armed_npc(1100.0, 1000.0, quad());
    wing.leader = Some(lead);
    let carrier = session.space.traffic.add_npc(wing);
    let mut viper = armed_npc(1000.0, 1000.0, quad());
    viper.leader = Some(carrier);
    let fighter = session.space.traffic.add_npc(viper);
    (session, lead, carrier, fighter)
}

#[test]
fn a_led_carriers_fighter_engages_a_missile_at_its_carrier_not_at_the_fleets_lead() {
    let catalog = carrying();
    let (mut session, _, carrier, fighter) = led_carrier(&catalog);
    assert!(engages(
        &mut session,
        &catalog,
        fighter,
        ShipRef::Npc(carrier)
    ));
    let (mut session, lead, _, fighter) = led_carrier(&catalog);
    assert!(!engages(
        &mut session,
        &catalog,
        fighter,
        ShipRef::Npc(lead)
    ));
}

/// The player's warship escort beside it, and a Quad-armed fighter the
/// escort launched, led by the escort with no escort duty of its own.
fn escort_carrier(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId) {
    let mut session = fleet(catalog, 0, vec![warship(None)]);
    let escort = fleet_npcs(&session)[0].id;
    let at = session.player().position;
    session.space.npc_mut(escort).expect("there").state.position = at;
    let mut viper = armed_npc(at.x + 50.0, at.y, quad());
    viper.leader = Some(escort);
    let fighter = session.space.traffic.add_npc(viper);
    (session, escort, fighter)
}

#[test]
fn a_players_escort_carriers_fighter_engages_a_missile_at_the_escort_not_at_the_player() {
    let catalog = carrying();
    let (mut session, escort, fighter) = escort_carrier(&catalog);
    assert!(engages(
        &mut session,
        &catalog,
        fighter,
        ShipRef::Npc(escort)
    ));
    let (mut session, _, fighter) = escort_carrier(&catalog);
    assert!(!engages(&mut session, &catalog, fighter, ShipRef::Player));
}

#[test]
fn an_escort_of_a_lead_with_no_lead_of_its_own_engages_a_missile_at_its_lead() {
    let catalog = carrying();
    let (mut session, lead, carrier, _) = led_carrier(&catalog);
    assert!(engages(&mut session, &catalog, carrier, ShipRef::Npc(lead)));
}

#[test]
fn a_players_escort_engages_a_missile_at_the_player() {
    let catalog = carrying();
    let (mut session, escort, _) = escort_carrier(&catalog);
    session.space.npc_mut(escort).expect("there").armament =
        Armament::new([(WeaponSpec::new(&quad()), 1)]);
    assert!(engages(&mut session, &catalog, escort, ShipRef::Player));
}

/// Keeps every goal, firing nothing.
#[derive(Debug)]
struct Quiet;

impl Behaviour for Quiet {
    fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        npc.goal
    }
}

/// Whether the player's escort, its Quad fitted and its armour full again,
/// engages a missile at the player, after a hit that disables it or none.
fn defends_the_player_after(hit: bool) -> bool {
    let catalog = armed();
    let mut session = escorted(&catalog);
    if hit {
        shot_into_the_escort(&mut session, &catalog);
        game_pass(&mut session, &catalog, &Quiet);
        assert_ne!(
            session.space.npc(NpcId(0)).expect("there").condition,
            Condition::Intact,
            "disabled"
        );
    }
    let npc = session.space.npc_mut(NpcId(0)).expect("there");
    npc.condition = Condition::Intact;
    npc.reserves.armor.now = npc.reserves.armor.max;
    npc.armament = Armament::new([(WeaponSpec::new(&quad()), 1)]);
    engages(&mut session, &catalog, NpcId(0), ShipRef::Player)
}

#[test]
fn an_escort_a_hit_put_out_of_the_fleet_no_longer_defends_its_old_lead() {
    // `_DamageShip` clears +0x9a (@0x3afdd); a disabled ship's point
    // defence runs no more anyway, so it is seen once the ship is intact.
    assert!(defends_the_player_after(false), "still its escort");
    assert!(!defends_the_player_after(true), "led by no one");
}
