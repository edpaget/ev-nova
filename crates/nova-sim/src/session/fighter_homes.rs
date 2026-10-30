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
//!
//! **Use As My Ship** (`_DoShipCapture` @0x41120). The player's own
//! fighters are written nothing, no loop reading parent 0: slot 0 is still
//! their carrier, so they fly home to the new ship, reach it by the new
//! class's sprite ([`Space::set_fighters_reach`], from every change of
//! class), and dock into the new weapons or stay out. They are never lost.
//!
//! The ships the captured ship leads are released
//! ([`Space::release_led`]): the loop @0x41300-0x413b4 writes, for each
//! but the captured ship and the old ship's new slot (whose leader is the
//! player here), +0x6c -1 ([`Npc::travel_set`](crate::traffic::npc::Npc::travel_set)),
//! +0x70 -1 (the target), +0x96 0 (the provocation), +0x9a -1 and +0x88
//! its `InherentAI`, and nothing else, so its pick stays. So the captured
//! carrier's fighters stop being fighters
//! ([`Npc::leave_fleet`](crate::traffic::npc::Npc::leave_fleet)). The goal
//! (+0xc8c8) stays too but for [`Goal::Attack`] and its kin, which hold
//! +0x70 inside the goal ([`Goal::quarry`]) and so go idle. A kept
//! [`Goal::Dock`] or [`Goal::Follow`] finds its ship gone and brakes, as
//! the original's state 5 skips its block with +0x9a -1 (@0x8ebec-0x8ebf0).
//! Use As Escort runs no such release: `_DoPlunderDialog` writes the ship
//! itself and `_AIShipCapturedByPlayer` resets the ships targeting it
//! (@0x896b6-0x89704), not those it leads.

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

    /// Every fighter out of the player's reads `reach`, its carrier's new
    /// sprite width, as the player's class changes. The original reads it
    /// live each tick (`_GetMasterShipSpriteXSize` @0x87f0b-0x87f21); the
    /// write comes before the next flight, so no frame sees the difference.
    pub(super) fn set_fighters_reach(&mut self, reach: f32) {
        for npc in self.traffic.npcs_mut() {
            if let Some(carrier) = &mut npc.carrier
                && carrier.ship == ShipRef::Player
            {
                carrier.reach = reach;
            }
        }
    }

    /// The ships NPC `id` leads are released, as it is taken as the
    /// player's own (see the module docs).
    pub(super) fn release_led(&mut self, id: NpcId) {
        for npc in self.traffic.npcs_mut() {
            if npc.leader != Some(id) {
                continue;
            }
            npc.travel_set = false;
            npc.target = None;
            npc.provoked = 0.0;
            // +0x88 is written with no test (@0x41398), over a bribe too,
            // so `leave_fleet` writes its `InherentAI` once the bribe's
            // word is gone.
            npc.overwrite_bribe();
            npc.leave_fleet();
            if npc.goal.quarry().is_some() {
                npc.goal = Goal::Idle;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::catalog::{OutfitRecord, ShipId, ShipRecord, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::combat::weapon::WeaponSpec;
    use crate::escort::{EscortCommand, EscortGroup};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::rulebook::Rulebook;
    use crate::session::Assigned;
    use crate::session::capture_naming::tests::use_as_my_ship;
    use crate::session::fighters::tests::{
        VIPER, VIPER_FIELDS, VIPERS, bay, carrying, fleet, flying_by, out, pilot, raided,
        until_docked, variant_bay, variants,
    };
    use crate::session::tests::{captured, kitted};
    use crate::stats::ShipStats;
    use crate::testkit::npc as testkit_npc;
    use crate::testkit::{Draws, FAST, FakePilotCatalog, outfit, ship, work};
    use crate::traffic::npc::{AiType, Npc};
    use crate::traffic::warp_out::WarpGoal;

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

    #[test]
    fn a_fighter_turned_back_from_a_jump_with_no_bay_stays_out_that_same_half() {
        // `begin_fly`'s turn-back writes goal 5 (@0x8d575-0x8d625) as the
        // half begins, so the fighter docks that half unless the list of
        // those with no bay is read after it.
        let catalog = bayless();
        let mut session = fleet(&catalog, 0, vec![out()]);
        session.take_save_due();
        let at = session.player().position;
        let fighter = session.space.traffic.npcs_mut()[0].id;
        let npc = session.space.npc_mut(fighter).expect("out");
        npc.state.position = at + Vec2::new(5.0, 0.0);
        npc.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        npc.class_fuel = 0;
        assert_eq!(until_docked(&mut session, &catalog, 1), None, "still out");
        let npc = session.space.npc(fighter).expect("still in the system");
        assert_eq!(npc.goal, Goal::Dock(ShipRef::Player), "turned back");
        assert_eq!(session.pilot().escorts().len(), 1);
        assert!(!session.take_save_due(), "nothing lost");
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

    // Use As My Ship.

    /// [`kitted`] with a Viper and its bay and fighter outfits, for sale;
    /// the player's ship 40 across and the trader, 100.
    fn prize_catalog() -> FakePilotCatalog {
        let mut catalog = kitted();
        catalog.ship_records.push(ShipRecord {
            escort_type: 0,
            inherent_ai: 4,
            ..ship(VIPER.0, VIPER_FIELDS)
        });
        catalog.weapons.push(bay());
        catalog.outfits.extend([
            outfit(157, &[(MOD_WEAPON, 149)]),
            OutfitRecord {
                mass: 0,
                max: 9999,
                ..outfit(158, &[(MOD_AMMO, 149)])
            },
        ]);
        for record in &mut catalog.hulls {
            match record.id.0 {
                128 => record.size = Some(40),
                129 => record.size = Some(100),
                _ => {}
            }
        }
        catalog
    }

    /// A Viper of the player's out, `x` right of the player at the centre.
    fn viper_out(session: &mut Session, x: f32) -> NpcId {
        let mut npc = testkit_npc(0, ShipStats::new(VIPER_FIELDS, &[]));
        npc.ship = VIPER;
        npc.state.position = Vec2::new(x, 0.0);
        let id = session.space.traffic.add_npc(npc);
        session.enlist(id, out());
        id
    }

    /// The player takes the captured ship, fitted with a Viper Bay when
    /// `bay_aboard`, as its own.
    fn take_over(session: &mut Session, bay_aboard: bool) {
        if bay_aboard {
            let prize = session.space.npc_mut(NpcId(0)).expect("the prize");
            prize.armament = Armament::new([(WeaponSpec::new(&bay()), 1)]);
        }
        assert_eq!(
            use_as_my_ship(session, "Prize", &mut Draws::of(&[0])),
            Some(Assigned::MyShip)
        );
    }

    /// Sends the fighters home, and any other escort to formation.
    fn recall(session: &mut Session) {
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
    }

    #[test]
    fn fighters_out_stay_in_the_fleet_when_the_player_takes_a_captured_ship() {
        // `_DoShipCapture` writes nothing to them: no loop reads parent 0.
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let fighter = viper_out(&mut session, 70.0);
        take_over(&mut session, false);
        assert!(session.is_escort(fighter));
        let npc = session.space.npc(fighter).expect("still out");
        assert_eq!(
            npc.carrier.map(|carrier| carrier.ship),
            Some(ShipRef::Player)
        );
        assert!(
            session
                .pilot()
                .escorts()
                .iter()
                .any(|escort| escort.carried && escort.ship == VIPER),
        );
    }

    #[test]
    fn a_fighter_out_docks_at_the_new_ships_sprite_width_into_its_bay() {
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let fighter = viper_out(&mut session, 70.0);
        let reach = |session: &Session| {
            let npc = session.space.npc(fighter).expect("out");
            npc.carrier.expect("a fighter").reach
        };
        assert_eq!(reach(&session), 40.0, "the old ship's");
        take_over(&mut session, true);
        assert_eq!(reach(&session), 100.0, "the new ship's, read live");
        recall(&mut session);
        assert!(
            until_docked(&mut session, &catalog, 3).is_some(),
            "70 off, within the new width and beyond the old"
        );
        assert_eq!(session.pilot().owned(VIPERS), 1, "into its bay");
    }

    #[test]
    fn a_fighter_out_with_no_bay_on_the_new_ship_stays_out() {
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let fighter = viper_out(&mut session, 70.0);
        take_over(&mut session, false);
        session.take_save_due();
        recall(&mut session);
        assert_eq!(until_docked(&mut session, &catalog, 300), None);
        let npc = session.space.npc(fighter).expect("out");
        assert!(matches!(npc.goal, Goal::Dock(_)), "{:?}", npc.goal);
        assert!(session.is_escort(fighter));
        assert_eq!(session.pilot().owned(VIPERS), 0, "no round");
    }

    /// A warship led by the prize (NPC 0), of `InherentAI` 4, its target
    /// the player, provoked, a stellar to travel to and its pick weapon 5,
    /// as `change` makes it: its number.
    fn led_by_prize(session: &mut Session, change: impl FnOnce(&mut Npc)) -> NpcId {
        let mut npc = testkit_npc(0, ShipStats::new(FAST, &[]));
        npc.ai_type = AiType::Warship;
        npc.hull.inherent_ai = 4;
        npc.leader = Some(NpcId(0));
        npc.target = Some(ShipRef::Player);
        npc.provoked = 30.0;
        npc.travel_set = true;
        npc.pick = Some(WeaponId(5));
        npc.state.position = Vec2::new(300.0, 300.0);
        change(&mut npc);
        session.space.traffic.add_npc(npc)
    }

    #[test]
    fn the_captured_carriers_fighter_stops_being_a_fighter() {
        // The loop @0x41300-0x413b4: +0x6c, +0x70, +0x96, +0x9a, +0x88.
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let fighter = led_by_prize(&mut session, |npc| {
            npc.goal = Goal::Dock(ShipRef::Npc(NpcId(0)));
            npc.take_carrier(
                Carrier {
                    ship: ShipRef::Npc(NpcId(0)),
                    window: 100.0,
                    reach: 60.0,
                },
                RuleSource::Engine,
            );
            npc.kept_escort_ai = true;
        });
        take_over(&mut session, false);
        let npc = session.space.npc(fighter).expect("there");
        assert_eq!(npc.leader, None);
        assert_eq!(npc.carrier, None);
        assert_eq!(npc.target, None);
        assert_eq!(npc.provoked, 0.0);
        assert!(!npc.travel_set);
        assert_eq!(npc.ai_type, AiType::Interceptor, "its `InherentAI`");
        assert!(!npc.kept_escort_ai);
        assert_eq!(npc.reserves.armor.max, npc.stats.armor, "its class's");
        assert_eq!(npc.reserves.shield.max, npc.stats.shield);
        assert_eq!(npc.pick, Some(WeaponId(5)), "not one of the five writes");
        assert_eq!(npc.goal, Goal::Dock(ShipRef::Npc(NpcId(0))), "kept");
    }

    #[test]
    fn a_bribed_follower_takes_its_inherent_ai_as_the_loop_tests_nothing() {
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let follower = led_by_prize(&mut session, Npc::take_bribe);
        take_over(&mut session, false);
        let npc = session.space.npc(follower).expect("there");
        assert_eq!(npc.ai_type, AiType::Interceptor);
        assert!(!npc.spared, "@0x41398 writes over the bribe's word");
        assert_eq!(npc.original_ai_type(), AiType::Interceptor.raw());
    }

    #[test]
    fn a_follower_attacking_goes_idle_and_any_other_goal_stays() {
        // `Goal::Attack` and its kin hold +0x70 in the goal, so writing it
        // -1 leaves them; +0xc8c8 is otherwise not written.
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let attacking = led_by_prize(&mut session, |npc| npc.goal = Goal::Attack(ShipRef::Player));
        let waiting = led_by_prize(&mut session, |npc| npc.goal = Goal::Wait);
        take_over(&mut session, false);
        let npc = |id| session.space.npc(id).expect("there");
        assert_eq!(npc(attacking).goal, Goal::Idle);
        assert_eq!(npc(waiting).goal, Goal::Wait);
        for id in [attacking, waiting] {
            assert_eq!(npc(id).pick, Some(WeaponId(5)));
            assert_eq!(npc(id).target, None);
        }
    }

    #[test]
    fn a_ship_the_captured_ship_does_not_lead_is_not_released() {
        let catalog = prize_catalog();
        let mut session = captured(&catalog);
        let other = led_by_prize(&mut session, |npc| {
            npc.leader = None;
            npc.goal = Goal::Attack(ShipRef::Player);
        });
        take_over(&mut session, false);
        let npc = session.space.npc(other).expect("there");
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert_eq!(npc.provoked, 30.0);
        assert!(npc.travel_set);
        assert_eq!(npc.ai_type, AiType::Warship);
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
    }
}
