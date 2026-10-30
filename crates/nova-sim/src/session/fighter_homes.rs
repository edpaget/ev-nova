//! Where a fighter out goes home to: a carrier with no bay for it, and a
//! carrier the player has just taken or left, as `_AIAddFighterToParent`
//! and `_DoShipCapture` have them.
//!
//! **No bay.** A fighter within its carrier's reach calls
//! `_AIAddFighterToParent` (`_LowLevelAIHandler` sub 8 @0x87f0b-0x87f21),
//! which writes nothing when neither of its walks finds a bay (@0x80384):
//! the fighter stays active with its parent and goal, the call skipping
//! the pull (@0x88111) after the steering and thrust have run
//! (@0x87ed1-0x87f02), and it retries next tick. The original's `AmmoType`
//! walks read the carrier's weapons, which the flight can't see, so the
//! session lists such fighters as each fly half begins
//! ([`Session::fighters_staying_out`], [`FlyPass::with_no_bay`]), asking
//! afresh, so one docks the first tick a bay is there. By the other reading
//! of [`RuleKey::FighterNoBay`] none stays out: it docks, and is lost
//! ([`Session::dock_fighter`]).

use std::collections::BTreeSet;

use super::{Session, Space, Statics};
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::rulebook::{RuleKey, RuleSource};
#[cfg(doc)]
use crate::traffic::fly_pass::FlyPass;
use crate::traffic::npc::NpcId;

impl Session {
    /// The fighters that stay out at their carrier this fly half (see the
    /// module docs).
    pub(super) fn fighters_staying_out(&self) -> BTreeSet<NpcId> {
        match self.setup.rules.source_for(RuleKey::FighterNoBay) {
            RuleSource::Engine => self
                .space
                .fighters_without_a_bay(&self.ship.fit.armament, &self.statics),
            RuleSource::Bible => BTreeSet::new(),
        }
    }
}

impl Space {
    /// The fighters docking whose carrier has no
    /// [`dock_bay`](Armament::dock_bay) for them, the player's carrying
    /// `player`'s armament. One whose carrier is gone isn't among them: it
    /// brakes, with nothing to dock with.
    fn fighters_without_a_bay(&self, player: &Armament, statics: &Statics) -> BTreeSet<NpcId> {
        let npcs = self.traffic.npcs().iter();
        npcs.filter(|npc| matches!(npc.goal, Goal::Dock(_)))
            .filter_map(|npc| {
                let armament = match npc.carrier?.ship {
                    ShipRef::Player => player,
                    ShipRef::Npc(carrier) => &self.npc(carrier)?.armament,
                };
                let base = statics.arsenal.sprite_base(npc.ship);
                armament
                    .dock_bay(npc.ship, base)
                    .is_none()
                    .then_some(npc.id)
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::catalog::ShipId;
    use crate::chance::NeverFires;
    use crate::combat::weapon::WeaponSpec;
    use crate::escort::{EscortCommand, EscortGroup};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::Rulebook;
    use crate::session::fighters::tests::{
        VIPER, VIPERS, bay, carrying, fleet, flying_by, out, pilot, raided, until_docked,
        variant_bay, variants,
    };
    use crate::testkit::{FakePilotCatalog, work};

    /// [`carrying`], the player's ship holding no bay.
    fn bayless() -> FakePilotCatalog {
        let mut catalog = carrying();
        catalog.hulls[0].weapons.clear();
        catalog
    }

    /// Gives the player's ship the Viper Bay.
    fn fit_bay(session: &mut Session) {
        session.ship.fit.armament = Armament::new([(WeaponSpec::new(&bay()), 1)]);
    }

    /// `catalog`'s fleet of one Viper out, ordered home, with no save due.
    fn sent_home(catalog: &FakePilotCatalog) -> Session {
        let mut session = fleet(catalog, 0, vec![out()]);
        session.take_save_due();
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        session
    }

    #[test]
    fn a_fighter_whose_bay_is_gone_stays_out_with_nothing_lost_and_no_save_due() {
        let catalog = bayless();
        let mut session = sent_home(&catalog);
        assert_eq!(until_docked(&mut session, &catalog, 600), None, "out");
        assert_eq!(session.pilot().escorts().len(), 1);
        assert_eq!(session.npcs().len(), 1);
        assert_eq!(session.pilot().owned(VIPERS), 0, "no round");
        assert!(!session.take_save_due(), "nothing lost");
        let npc = &session.npcs()[0];
        let off = npc.state.position - session.player().position;
        assert!(off.length() < 200.0, "it hovers at the player: {off:?}");
    }

    #[test]
    fn a_fighter_staying_out_docks_once_a_bay_is_there() {
        let catalog = bayless();
        let mut session = sent_home(&catalog);
        assert_eq!(until_docked(&mut session, &catalog, 300), None);
        fit_bay(&mut session);
        assert!(until_docked(&mut session, &catalog, 300).is_some());
        assert_eq!(session.pilot().owned(VIPERS), 1, "a round");
        assert!(!session.take_save_due(), "as any dock");
    }

    /// `session`'s `fighter` Viper of `raider`, 10 off it within its reach,
    /// ordered home: its NPC.
    fn docking_viper(session: &mut Session, raider: NpcId, reach: f32) -> NpcId {
        let at = session.space.npc(raider).expect("there").state.position;
        let mut fighter =
            crate::testkit::npc(0, crate::stats::ShipStats::new(crate::testkit::FAST, &[]));
        fighter.ship = VIPER;
        fighter.state.position = at + Vec2::new(10.0, 0.0);
        fighter.goal = Goal::Dock(ShipRef::Npc(raider));
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(raider),
            window: 100.0,
            reach,
        });
        session.space.traffic.add_npc(fighter)
    }

    #[test]
    fn an_npc_carriers_fighter_with_no_bay_is_listed_and_one_with_a_bay_is_not() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let fighter = docking_viper(&mut session, raider, 60.0);
        assert_eq!(session.fighters_staying_out(), BTreeSet::new(), "a bay");
        session.space.npc_mut(raider).expect("there").armament = Armament::default();
        assert_eq!(session.fighters_staying_out(), BTreeSet::from([fighter]));
        session.space.npc_mut(fighter).expect("there").goal = Goal::Idle;
        assert_eq!(
            session.fighters_staying_out(),
            BTreeSet::new(),
            "not going home"
        );
    }

    #[test]
    fn a_fighter_whose_carrier_is_gone_is_not_listed() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let _ = docking_viper(&mut session, raider, 60.0);
        session.space.npc_mut(raider).expect("there").armament = Armament::default();
        session.leave(raider);
        assert_eq!(session.fighters_staying_out(), BTreeSet::new());
    }

    #[test]
    fn an_npc_carriers_fighter_with_no_bay_stays_out_and_with_one_docks() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let bays_held = session.space.npc(raider).map(|npc| npc.rounds.clone());
        let fighter = docking_viper(&mut session, raider, 60.0);
        let armament = session.space.npc(raider).expect("there").armament.clone();
        session.space.npc_mut(raider).expect("there").armament = Armament::default();
        work(&mut session, &catalog, Controls::default());
        assert!(session.space.npc(fighter).is_some(), "out");
        session.space.npc_mut(raider).expect("there").armament = armament;
        work(&mut session, &catalog, Controls::default());
        assert!(session.space.npc(fighter).is_none(), "aboard");
        assert_ne!(
            session.space.npc(raider).map(|npc| npc.rounds.clone()),
            bays_held
        );
    }

    #[test]
    fn a_fighter_of_a_variant_docks_into_a_bay_of_its_sprite_base_and_a_sibling_stays_out() {
        let catalog = variants();
        let variant = |ship| crate::pilot::Escort { ship, ..out() };
        let mut session = fleet(&catalog, 0, vec![variant(ShipId(223))]);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert!(until_docked(&mut session, &catalog, 600).is_some());
        assert_eq!(session.pilot().owned(VIPERS), 1, "into the Viper's bay");
        let catalog = variant_bay();
        let mut session = fleet(&catalog, 0, vec![variant(ShipId(224))]);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert_eq!(
            until_docked(&mut session, &catalog, 300),
            None,
            "no walk finds a bay"
        );
        assert_eq!(session.pilot().escorts().len(), 1);
    }

    #[test]
    fn by_the_other_reading_no_fighter_stays_out() {
        let catalog = bayless();
        let rules = Rulebook::default().with_override(RuleKey::FighterNoBay, RuleSource::Bible);
        let mut session = flying_by(&catalog, pilot(&catalog, 0, vec![out()]), rules);
        session.space.traffic.npcs_mut()[0].goal = Goal::Dock(ShipRef::Player);
        assert_eq!(session.fighters_staying_out(), BTreeSet::new());
    }
}
