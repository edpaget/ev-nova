//! The session's view of the original's shot slots (rdm weapons-fidelity
//! phase 34): the order the shots are drawn in, each shot layer as its
//! slots entered it (see `nova_sim_combat::combat::shot_slots`), and the
//! rules that read the slots.

use super::{Session, Space};
use crate::combat::projectile::Shot;

impl Session {
    /// The shots in flight in the order they are drawn, bottom first: by
    /// layer, and within a layer by
    /// [`RuleKey::ShotOverlap`](crate::rulebook::RuleKey::ShotOverlap), the
    /// order their slots entered it by the engine, firing order by the
    /// other reading
    /// ([`Combat::shots_drawn`](crate::combat::Combat::shots_drawn)).
    ///
    /// For the tests: the frame fills from `Space`'s own
    /// (`Space::fill_shots`).
    #[cfg(test)]
    #[must_use]
    pub(crate) fn shots_drawn(&self) -> Vec<&Shot> {
        self.space.shots_drawn()
    }
}

impl Space {
    /// The shots in flight in the order they are drawn, bottom first: by
    /// layer, and within a layer by
    /// [`RuleKey::ShotOverlap`](crate::rulebook::RuleKey::ShotOverlap), the
    /// order their slots entered it by the engine, firing order by the
    /// other reading
    /// ([`Combat::shots_drawn`](crate::combat::Combat::shots_drawn)).
    #[must_use]
    pub(super) fn shots_drawn(&self) -> Vec<&Shot> {
        self.combat.shots_drawn()
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{FIRE, armed, facing_an_npc};
    use super::super::*;
    use crate::catalog::WeaponRecord;

    use crate::combat::Combat;
    use crate::combat::shot_layer::ShotLayer;
    use crate::flight::Controls;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::testkit::work;
    use nova_sim_catalog::fixture::weapon;

    #[test]
    fn the_shots_are_drawn_in_the_fights_order() {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        while session.shots().len() < 2 {
            work(&mut session, &catalog, Controls::default());
        }
        session.space.combat.shots_mut()[0].layer = ShotLayer::OverShips;
        let ids = |shots: Vec<&Shot>| shots.iter().map(|shot| shot.id).collect::<Vec<_>>();
        let first = session.shots()[0].id;
        assert_eq!(
            ids(session.shots_drawn()),
            ids(session.space.combat.shots_drawn())
        );
        assert_eq!(ids(session.shots_drawn())[1], first, "over the ships");
    }

    #[test]
    fn the_rulebook_says_how_the_shot_layers_overlap() {
        let catalog = armed();
        let session = |rules: Rulebook| Session::start(&catalog).expect("starts").with_rules(rules);
        let mut firing = Combat::default();
        firing.set_shot_overlap(RuleSource::Bible);
        assert_ne!(firing, Combat::default());
        let other = Rulebook::default().with_override(RuleKey::ShotOverlap, RuleSource::Bible);
        assert_eq!(session(other).space.combat, firing);
        assert_eq!(session(Rulebook::default()).space.combat, Combat::default());
    }

    #[test]
    fn the_rulebook_says_what_a_sub_munition_reads_of_its_slots() {
        let catalog = armed();
        let session = |rules: Rulebook| Session::start(&catalog).expect("starts").with_rules(rules);
        let mut firer = Combat::default();
        firer.set_submunition_source_slot(RuleSource::Bible);
        assert_ne!(firer, Combat::default());
        let other =
            Rulebook::default().with_override(RuleKey::SubmunitionSourceSlot, RuleSource::Bible);
        assert_eq!(session(other).space.combat, firer);
        let mut no_roll = Combat::default();
        no_roll.set_reused_shot_slot_roll(RuleSource::Bible);
        assert_ne!(no_roll, Combat::default());
        let other =
            Rulebook::default().with_override(RuleKey::ReusedShotSlotRoll, RuleSource::Bible);
        assert_eq!(session(other).space.combat, no_roll);
        let mut no_spin = Combat::default();
        no_spin.set_reused_shot_slot_spin(RuleSource::Bible);
        assert_ne!(no_spin, Combat::default());
        let other =
            Rulebook::default().with_override(RuleKey::ReusedShotSlotSpin, RuleSource::Bible);
        assert_eq!(session(other).space.combat, no_spin);
    }

    /// [`armed`] with its ships firing a shell that releases one turret
    /// (4) sub-munition at the end of its life, and a class 131 like the
    /// traffic's 129 but of `Flags3` 0x0040.
    fn shell_catalog() -> crate::testkit::FakePilotCatalog {
        let mut catalog = armed();
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            energy_dmg: 1,
            sub_count: 1,
            sub_type: Some(WeaponId(148)),
            ..weapon(128)
        };
        let sub = WeaponRecord {
            guidance: 4,
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        catalog.weapons = vec![shell, sub];
        let mut over = catalog.hulls[1].clone();
        over.id = ShipId(131);
        over.flags3 = 0x0040;
        catalog.hulls.push(over);
        catalog
    }

    #[test]
    fn a_sub_munition_takes_its_layer_from_the_ship_that_took_its_firers_berth() {
        // The NPC at berth 1 fires its shell and leaves; a ship of class
        // 131 takes berth 1 before the shell's end of life, and its turret
        // sub-munition goes over the ships by that class (`_SpawnShot`
        // @0x3f0d6-0x3f109). By the other reading, the parent's bit.
        let catalog = shell_catalog();
        for (source, layer) in [
            (RuleSource::Engine, ShotLayer::OverShips),
            (RuleSource::Bible, ShotLayer::Shots),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::SubmunitionSourceSlot, source);
            let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
            let npc = &mut session.space.traffic.npcs_mut()[0];
            npc.trigger = FIRE;
            npc.target = Some(ShipRef::Player);
            let berth = npc.berth;
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.shots().len(), 1, "the shell");
            let mut next = session.npcs()[0].clone();
            next.ship = ShipId(131);
            next.trigger = Trigger::default();
            session.space.traffic.remove(next.id);
            let id = session.space.traffic.add_npc(next);
            assert_eq!(session.space.npc(id).map(|npc| npc.berth), Some(berth));
            for _ in 0..5 {
                work(&mut session, &catalog, Controls::default());
            }
            let subs: Vec<&Shot> = session
                .shots()
                .iter()
                .filter(|shot| shot.weapon.id == WeaponId(148))
                .collect();
            assert_eq!(subs.len(), 1, "{source:?}");
            assert_eq!(subs[0].layer, layer, "{source:?}");
        }
    }

    #[test]
    fn a_sessions_shot_slots_reach_the_next_session_through_its_slot_sprites() {
        // The shot sprites outlive a pilot, as the ship slots' do
        // (`_InitSprites` runs once, @0x1fa90-0x1fb83).
        let catalog = armed();
        let mut history = crate::combat::shot_slots::ShotSlots::new();
        history.put(0, ShotLayer::Homing);
        history.write_roll(0, Some(40));
        let sprites = crate::ship_layer::SlotSprites {
            shots: history.clone(),
            ..Default::default()
        };
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_slot_sprites(sprites);
        assert_eq!(
            session.space.combat.shot_slots(),
            history,
            "handed to the fight"
        );
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.shots().len(), 1);
        let left = session.slot_sprites().shots.clone();
        assert_eq!(left.rank(0).map(|(layer, _)| layer), Some(ShotLayer::Shots));
        assert!(
            left.rank(127) < left.rank(0),
            "moved to the shot layer's tail"
        );
        assert_eq!(left.roll(0), None, "the blaster's own roll, none");
        let next = Session::start(&catalog)
            .expect("starts")
            .with_slot_sprites(session.slot_sprites().clone());
        assert_eq!(next.space.combat.shot_slots(), left);
    }

    #[test]
    fn a_slots_spin_timer_reaches_the_next_session_and_its_first_spinning_shot() {
        // `_SpawnShot` never writes +0x1c (@0x3e550-0x3f143), and the shot
        // table outlives a system change and a pilot, so a spinning shot
        // starts on what its slot's last one left.
        let mut catalog = armed();
        catalog.weapons[0].flags |= crate::combat::weapon::SPINS;
        catalog.weapons[0].beam_width = 9;
        let mut history = crate::combat::shot_slots::ShotSlots::new();
        history.spin_pass(0, 9);
        history.spin_pass(0, 9);
        let sprites = crate::ship_layer::SlotSprites {
            shots: history,
            ..Default::default()
        };
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_slot_sprites(sprites);
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.shots().len(), 1);
        assert_eq!(session.shots()[0].spin_timer, 2, "the slot's leftover");
        let left = session.slot_sprites().shots.spin(0);
        assert!(left >= 2, "{left}");
        let next = Session::start(&catalog)
            .expect("starts")
            .with_slot_sprites(session.slot_sprites().clone());
        assert_eq!(next.space.combat.shot_slots().spin(0), left);
    }
}
