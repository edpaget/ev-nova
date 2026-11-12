//! Test support for tests outside the crate's own unit tests (the
//! `fixture` feature): [`MemoryPilots`], a
//! [`PilotStore`](crate::saves::PilotStore) in memory; the
//! storyline walker, [`StoryWalker`], which drives a session through a
//! mission chain from a script of [`Step`]s (see the `walker` module); and
//! [`bits_allowing`], the control bits that make a test hold, for setting
//! a pilot up directly; [`disable`] and [`destroy`], for disabling or
//! destroying a ship directly;
//! [`set_player_condition`], for downing or repairing the player directly;
//! [`set_player_shield`], for putting the player's shields down directly;
//! [`set_player_armor`], for putting the player's armour down directly;
//! [`set_player_ion`] and [`set_player_tractor`], for ionizing or
//! holding the player directly; [`put_player_over`], for bringing the player over a ship to board it;
//! [`put_player_at`], for bringing the player to a place, a gate among them;
//! [`remember_escort`], for making a ship the player's former escort;
//! [`cloak`] and [`set_player_cloak`], for setting a ship's or the
//! player's cloak directly; [`hide`], for
//! making a ship's type one whose `Cost` is -1; [`place`], for putting a
//! ship somewhere directly; [`bring_in`], for bringing a ship of a given
//! class and government into the system directly; [`sabotage_kill`], for setting off the
//! player's kill outfit directly; [`alarm_stellar`] and
//! [`defence_ships_left`], for a stellar's defence fleet;
//! [`listed_stellars`], a fake catalog's listed stellars derived from its
//! landing sites; and [`SessionBuilder`], a session in a given state
//! reached without the load's rules (see the `builder` module).
//!
//! Two ports' contracts live here too: [`pilot_store`]'s, which every
//! [`PilotStore`](crate::saves::PilotStore) meets, and [`chance`]'s, nova-sim-base's `Chance`
//! contract re-exported so the crates above reach it through this facade.

mod builder;
mod memory_pilots;
pub mod pilot_store;
mod walker;

pub use nova_sim_base::fixture::chance;

pub use builder::SessionBuilder;
pub use memory_pilots::MemoryPilots;

/// The catalog crate's canned pilot catalog, for a test above that needs a
/// session started on a catalog it doesn't build itself.
pub use nova_sim_catalog::fixture::{FakePilotCatalog, catalog as pilot_catalog};
/// The control bits that make a test hold, and the listed stellars a fake
/// catalog gives, from the catalog crate's test support.
pub use nova_sim_catalog::fixture::{bits_allowing, list_also, listed_stellars};
pub use walker::{
    Became, Done, Fault, Looked, Step, StoryWalker, WalkFailure, Walked, candidates, expected_bits,
    expected_pay,
};

use crate::cloak::Cloak;
use crate::combat::hull::Condition;
use crate::session::Session;
use crate::traffic::npc::NpcId;

/// Disables NPC `id` in `session`, if it is in the system, its armour
/// down to 1 and its shield to 0: for a test that needs a ship the
/// player cannot shoot, such as one of its escorts, disabled.
pub fn disable(session: &mut Session, id: NpcId) {
    if let Some(npc) = session.npc_mut(id) {
        npc.reserves.shield.now = 0.0;
        npc.reserves.armor.now = 1.0;
        npc.condition = Condition::Disabled;
    }
}

/// Destroys NPC `id` in `session`, if it is in the system, as the fight
/// does once it has broken up: its condition destroyed, then taken out of
/// the system, so its [`Fate::Destroyed`](crate::story::Fate::Destroyed)
/// is raised for the next settling and, for a mission's special ship, it
/// is counted lost. For a test that needs a ship gone without a fight,
/// which [`disable`] doesn't give: a disabled ship raises no fate.
pub fn destroy(session: &mut Session, id: NpcId) {
    let Some(npc) = session.npc_mut(id) else {
        return;
    };
    npc.condition = Condition::Destroyed;
    session.take_out_the_destroyed(&mut crate::chance::NeverFires);
}

/// Repairs NPC `id` in `session`, if it is in the system, its shield and
/// armour full and intact: for a test that needs a disabled ship well
/// again without its repair.
pub fn repair(session: &mut Session, id: NpcId) {
    if let Some(npc) = session.npc_mut(id) {
        npc.reserves.shield.now = npc.reserves.shield.max;
        npc.reserves.armor.now = npc.reserves.armor.max;
        npc.condition = Condition::Intact;
    }
}

/// Sets the player's condition in `session` to `to`: for a test that
/// needs the player disabled or breaking up, or made intact again,
/// without a fight.
pub fn set_player_condition(session: &mut Session, to: Condition) {
    session.set_player_condition(to);
}

/// Sets the player's shield points in `session` to `now`, its most kept:
/// for a test that needs the player's shields down without a fight, a
/// loaded pilot's being refilled as `_LoadPilotData` refills them.
pub fn set_player_shield(session: &mut Session, now: f32) {
    session.set_player_shield(now);
}

/// Sets the player's armour points in `session` to `now`, its most kept,
/// its condition left as it is: for a test that needs the player one hit
/// from breaking up, so that a real hit's whole path runs.
pub fn set_player_armor(session: &mut Session, now: f32) {
    session.set_player_armor(now);
}

/// Sets the player's ionization in `session` to `ion`: for a test that
/// needs the player ionized without a fight.
pub fn set_player_ion(session: &mut Session, ion: crate::combat::ion::Ion) {
    session.set_player_ion(ion);
}

/// Sets the player's tractor lock in `session` to `lock`: for a test that
/// needs the player held without a fight.
pub fn set_player_tractor(session: &mut Session, lock: Option<crate::combat::impact::TractorLock>) {
    session.set_player_tractor(lock);
}

/// Puts the player's ship in `session` over NPC `id`, at its place,
/// velocity and heading, and targets it, if it is in the system: for a
/// test that boards a ship without flying to it.
pub fn put_player_over(session: &mut Session, id: NpcId) {
    session.put_player_over(id);
}

/// Puts the player's ship in `session` at `at`, at rest, its heading
/// kept: for a test that flies to a stellar, a gate among them, without
/// steering there.
pub fn put_player_at(session: &mut Session, at: crate::geometry::Vec2) {
    session.put_player_at(at);
}

/// Makes NPC `id` in `session`, if it is in the system, remember being
/// the player's escort of `record` ([`Npc::former`](crate::traffic::npc::Npc::former)): for a test that
/// boards a former escort back without losing it in a fight.
pub fn remember_escort(session: &mut Session, id: NpcId, record: crate::pilot::Escort) {
    if let Some(npc) = session.npc_mut(id) {
        npc.former = Some(crate::self_repair::FormerEscort { record });
    }
}

/// Sets the player's cloak in `session` to `to`: for a test that draws
/// the player's ship cloaked.
pub fn set_player_cloak(session: &mut Session, to: Cloak) {
    session.set_player_cloak(to);
}

/// Sets NPC `id`'s cloak in `session` to `to`, if it is in the system:
/// for a test that draws a ship cloaked.
pub fn cloak(session: &mut Session, id: NpcId, to: Cloak) {
    if let Some(npc) = session.npc_mut(id) {
        npc.cloak = to;
    }
}

/// Makes NPC `id`'s type in `session` one whose `Cost` is -1, if it is
/// in the system: for a test that needs a hidden ship
/// ([`Session::is_hidden`]).
pub fn hide(session: &mut Session, id: NpcId) {
    if let Some(npc) = session.npc_mut(id) {
        npc.hull.hidden = true;
    }
}

/// Puts NPC `id` in `session` at `at`, at rest, if it is in the system:
/// for a test that needs a ship somewhere the traffic did not place it,
/// such as in front of the player once it has flown out.
pub fn place(session: &mut Session, id: NpcId, at: crate::geometry::Vec2) {
    if let Some(npc) = session.npc_mut(id) {
        npc.state.position = at;
        npc.state.velocity = crate::geometry::Vec2::ZERO;
    }
}

/// Brings a ship of class `ship`, of `govt`, flying as `ai`, into the
/// player's system in `session` at `at`, at rest, as a placed spawn: no
/// mission, no person, no leader, aggression 3. Its id, or `None` for a
/// class the catalog lacks. For a test that needs a ship of a given class
/// and government in the system without the traffic's draws, such as two
/// enemies set side by side to fight.
pub fn bring_in(
    session: &mut Session,
    ship: crate::catalog::ShipId,
    govt: Option<crate::catalog::GovtId>,
    ai: crate::traffic::npc::AiType,
    at: crate::geometry::Vec2,
) -> Option<NpcId> {
    session.bring_in(ship, govt, ai, at)
}

/// Takes NPC `id` out of `session` and puts a copy of it in its berth at
/// `at`, at rest, if it is in the system: the copy's id. For a test that
/// needs a ship come into a slot since the slot's last display, so boxed
/// where the slot's last ship was ([`Session::hover_place`]).
pub fn refill(session: &mut Session, id: NpcId, at: crate::geometry::Vec2) -> Option<NpcId> {
    session.refill_berth(id, at)
}

/// Marks `stellar` under attack in `session` (+0x47), as the planet comm
/// dialog's Demand Tribute will (rdm:phase/play-feel-4/phase-2-planet-comm-dialog):
/// for a test that needs a stellar sending up its defence ships.
pub fn alarm_stellar(session: &mut Session, stellar: crate::catalog::StellarId) {
    session.alarm_stellar(stellar);
}

/// The defence ships `stellar`, one the player's system lists, has left
/// in `session` (+0x466); none for a stellar the system doesn't list.
#[must_use]
pub fn defence_ships_left(session: &Session, stellar: crate::catalog::StellarId) -> Option<i16> {
    session.defence_ships_left(stellar)
}

/// Sets whether the player dominates `stellar` in `session`, as the planet
/// comm dialog's Demand Tribute and Release do
/// (rdm:phase/play-feel-4/phase-3-domination-readers): for a test that
/// needs a stellar dominated without fighting its fleet first.
pub fn dominate(session: &mut Session, stellar: crate::catalog::StellarId, dominated: bool) {
    session.dominate(stellar, dominated);
}

/// Sets off a sabotage kill outfit on the player in `session` at once, as
/// its sabotage clock does when it runs out: for a test that needs the
/// player's jump given up and its ship breaking up without waiting on the
/// clock.
pub fn sabotage_kill(session: &mut Session) {
    session.sabotage_kill();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::catalog;

    #[test]
    fn setting_the_players_condition_downs_and_repairs_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        set_player_condition(&mut session, Condition::Disabled);
        assert_eq!(session.player_condition(), Condition::Disabled);
        set_player_condition(&mut session, Condition::Intact);
        assert_eq!(session.player_condition(), Condition::Intact);
    }

    #[test]
    fn bringing_a_ship_in_gives_the_id_of_the_ship_it_placed() {
        use crate::catalog::{GovtId, ShipId};
        use crate::testkit::{FAST, FakePilotCatalog, ship};
        use crate::traffic::npc::AiType;
        let catalog = FakePilotCatalog {
            ship_records: vec![ship(129, FAST)],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        let at = crate::geometry::Vec2::new(40.0, 60.0);
        let first =
            bring_in(&mut session, ShipId(129), None, AiType::Warship, at).expect("brought in");
        let second = bring_in(
            &mut session,
            ShipId(129),
            Some(GovtId(131)),
            AiType::Warship,
            at,
        )
        .expect("brought in");
        assert_ne!(first, second);
        let npc = |id| session.npcs().iter().find(|npc| npc.id == id).cloned();
        assert_eq!(npc(first).map(|npc| npc.govt), Some(None));
        assert_eq!(npc(second).map(|npc| npc.govt), Some(Some(GovtId(131))));
        assert_eq!(
            bring_in(&mut session, ShipId(999), None, AiType::Warship, at),
            None
        );
    }

    #[test]
    #[allow(clippy::float_cmp)] // the helper assigns, so the points match exactly
    fn setting_the_players_shield_sets_only_its_points() {
        let mut session = Session::start(&catalog()).expect("starts");
        let before = session.reserves();
        assert!(before.shield.max > 10.0, "{before:?}");
        set_player_shield(&mut session, 10.0);
        let after = session.reserves();
        assert_eq!(after.shield.now, 10.0);
        assert_eq!(after.shield.max, before.shield.max, "its most kept");
        assert_eq!((after.armor, after.fuel), (before.armor, before.fuel));
    }

    #[test]
    fn setting_the_players_ionization_charges_and_colours_it() {
        let mut session = Session::start(&catalog()).expect("starts");
        let ion = crate::combat::ion::Ion {
            charge: 12.0,
            colour: 0x0000_80ff,
            ..crate::combat::ion::Ion::default()
        };
        set_player_ion(&mut session, ion);
        assert_eq!(session.ion(), ion);
    }

    #[test]
    fn setting_the_players_tractor_lock_holds_it() {
        use crate::combat::ShipRef;
        use crate::combat::impact::TractorLock;
        let mut session = Session::start(&catalog()).expect("starts");
        let lock = Some(TractorLock::by(ShipRef::Npc(NpcId(3))));
        set_player_tractor(&mut session, lock);
        assert_eq!(session.tractor(), lock);
        assert!(session.held_by_other());
    }
}
