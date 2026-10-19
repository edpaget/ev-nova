//! Where each ship's slot sprite stands as the collision passes walk the
//! ships ([`SpriteRank`]), which decides which ship a shot meets first
//! when it touches two, and in which order a pass's hits land.
//!
//! `_SWCollideSpriteLayer` (@0xacb4b) walks a ship layer from its head by
//! each sprite's next link, so in entry order, bottom first: the order the
//! layer is drawn in (`_SWAddSprite` appends at the tail, @0xac620). And
//! only `_HandleShipDisplay` puts a ship's sprite in a layer
//! (`_PutSpriteInLayer` @0x2bd58-0x2bdad): the player's display runs just
//! before the passes (@0x43e29), each NPC's after its own `_HandleShip`
//! the frame before (@0x444ad). So the passes walk the ships in the order
//! the last frame drew them: [`Session::draw_layers`], by either reading
//! of [`RuleKey::ShipOverlap`](crate::RuleKey::ShipOverlap), a hidden
//! ship ([`Session::is_hidden`]) left out, as its base sprite is hidden
//! and the walk skips it (@0x2cc6b, @0xacba3).
//!
//! The session lays each ship out at its own display step, as the
//! original does (`Session::lay_out_displayed`): the player's just before
//! the passes, each NPC's after its `_HandleShip`, before `_HandleBeams`.
//! So a ship a beam disabled in the last pass is walked in the layer its
//! display gave it before the beam, as the original walks it.

use super::Session;
use crate::combat::{ShipRef, SpriteRank};

impl Session {
    /// Each ship the collision passes walk, with where its slot sprite
    /// stands: its layer and its place in the layer's order, bottom
    /// first, as [`Session::draw_layers`] gives them (see the module
    /// docs). A hidden ship is left out.
    pub(super) fn sprite_ranks(&self) -> Vec<(ShipRef, SpriteRank)> {
        self.draw_layers()
            .into_iter()
            .zip(0..)
            .map(|((ship, layer), entry)| (ship, SpriteRank { layer, entry }))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::MissionShip;
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::combat::{GamePass, Rules};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::mission_ships::tests::{accepted, shipping, ships_catalog, take_off};
    use crate::ship_layer::{ShipLayer, Slot};
    use crate::traffic::npc::NpcId;

    /// In flight with mission 140's two ships, which never join the
    /// player: the session and the ships, by berth.
    fn two_strangers() -> (Session, [NpcId; 2]) {
        let catalog = ships_catalog(MissionShip {
            behav: 0,
            ..shipping(-1, 0)
        });
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let at = |berth| {
            session
                .npcs()
                .iter()
                .find(|npc| npc.berth == berth)
                .expect("a ship at the berth")
                .id
        };
        let ships = [at(1), at(2)];
        for id in ships {
            assert!(!session.npc(id).expect("there").escorts_player());
        }
        (session, ships)
    }

    /// The sprite rank each fighter of the fight takes.
    fn ranks(session: &mut Session) -> BTreeMap<ShipRef, Option<SpriteRank>> {
        let mut ranks = BTreeMap::new();
        let pass = GamePass::Advance { double_time: false };
        session.with_fighters(
            pass,
            Rules::default(),
            &mut NeverFires,
            |_, fighters, _, _| {
                ranks = fighters
                    .iter()
                    .map(|fighter| (fighter.ship, fighter.sprite))
                    .collect();
            },
        );
        ranks
    }

    fn npc(id: NpcId) -> ShipRef {
        ShipRef::Npc(id)
    }

    #[test]
    fn of_two_npcs_a_shot_touches_at_once_the_one_whose_slot_entered_its_layer_first_takes_it() {
        let (mut session, [first, second]) = two_strangers();
        let at_launch = ranks(&mut session);
        let rank =
            |ranks: &BTreeMap<ShipRef, Option<SpriteRank>>, ship| ranks[&ship].expect("walked");
        // `_InitSprites` adds the slots from 63 down: the higher berth is
        // lower in the layer, so walked first.
        assert_eq!(rank(&at_launch, npc(first)).layer, ShipLayer::Ships);
        assert_eq!(rank(&at_launch, npc(second)).layer, ShipLayer::Ships);
        assert!(rank(&at_launch, npc(second)) < rank(&at_launch, npc(first)));
        assert!(rank(&at_launch, npc(first)) < rank(&at_launch, ShipRef::Player));
        // Berth 2 re-enters the layer, on top of every ship in it.
        session
            .slot_sprites
            .layers
            .put(Slot::Berth(2), ShipLayer::Disabled);
        session
            .slot_sprites
            .layers
            .put(Slot::Berth(2), ShipLayer::Ships);
        let after = ranks(&mut session);
        assert!(rank(&after, npc(first)) < rank(&after, ShipRef::Player));
        assert!(rank(&after, ShipRef::Player) < rank(&after, npc(second)));
    }

    #[test]
    fn a_disabled_npc_is_walked_in_the_bottom_layer() {
        let (mut session, [first, _]) = two_strangers();
        crate::fixture::disable(&mut session, first);
        // Laid out at its display step (@0x444ad, @0x2bd24).
        session.tick_combat(Rules::default(), &mut NeverFires);
        let ranks = ranks(&mut session);
        assert_eq!(
            ranks[&npc(first)].expect("walked").layer,
            ShipLayer::Disabled
        );
    }

    #[test]
    fn a_hidden_ship_takes_no_contact_by_the_engine_and_does_by_the_other_reading() {
        let (mut session, [first, second]) = two_strangers();
        crate::fixture::hide(&mut session, first);
        let hidden = ranks(&mut session);
        assert_eq!(hidden[&npc(first)], None);
        assert!(hidden[&npc(second)].is_some());
        let mut session = session
            .with_rules(Rulebook::default().with_override(RuleKey::HiddenShips, RuleSource::Bible));
        assert!(ranks(&mut session)[&npc(first)].is_some());
    }

    #[test]
    fn by_the_other_overlap_reading_the_walk_takes_the_npc_order() {
        let (session, _) = two_strangers();
        let mut session = session
            .with_rules(Rulebook::default().with_override(RuleKey::ShipOverlap, RuleSource::Bible));
        let ranks = ranks(&mut session);
        let order: Vec<ShipRef> = session.npcs().iter().map(|each| npc(each.id)).collect();
        let rank = |ship: ShipRef| ranks[&ship].expect("walked");
        assert!(rank(order[0]) < rank(order[1]));
        assert!(rank(order[1]) < rank(ShipRef::Player), "the player on top");
    }

    // Each ship laid out at its display step (weapons-fidelity phase 39).

    use crate::catalog::WeaponRecord;
    use crate::flight::Controls;
    use crate::session::tests::{FIRE, Firing, armed, blaster, facing_an_npc};
    use crate::testkit::FakePilotCatalog;

    /// [`armed`] with weapon 128 a 200-pixel beam lasting 3 ticks, firing
    /// every 10, doing 5 mass damage and no energy.
    fn beaming() -> FakePilotCatalog {
        FakePilotCatalog {
            weapons: vec![WeaponRecord {
                guidance: 0,
                count: 3,
                reload: 10,
                speed: 0,
                beam_length: 200,
                mass_dmg: 5,
                energy_dmg: 0,
                ..blaster()
            }],
            ..armed()
        }
    }

    /// A pass of the game's work, its NPCs deciding as `behaviour` says.
    fn work(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &impl crate::ai::Behaviour,
    ) {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// The NPC `ship` is.
    fn npc_id(ship: ShipRef) -> NpcId {
        match ship {
            ShipRef::Npc(id) => id,
            ShipRef::Player => panic!("not an NPC"),
        }
    }

    /// The layer `ship` is drawn in.
    fn drawn_in(session: &Session, ship: ShipRef) -> ShipLayer {
        session
            .draw_layers()
            .into_iter()
            .find(|&(drawn, _)| drawn == ship)
            .expect("drawn")
            .1
    }

    /// The layer `ship` is walked in.
    fn walked_in(session: &Session, ship: ShipRef) -> ShipLayer {
        session
            .sprite_ranks()
            .into_iter()
            .find(|&(walked, _)| walked == ship)
            .expect("walked")
            .1
            .layer
    }

    /// [`beaming`]'s session facing its NPC, the NPC's shield down and its
    /// armour of 45 at 16: one tick of the beam disables it.
    fn facing_a_frail_npc(catalog: &FakePilotCatalog) -> (Session, ShipRef) {
        let mut session = facing_an_npc(catalog, 180);
        let target = &mut session.traffic.npcs_mut()[0];
        target.reserves.shield.now = 0.0;
        target.reserves.armor.now = 16.0;
        let id = ShipRef::Npc(target.id);
        (session, id)
    }

    #[test]
    fn an_npc_a_beam_disabled_last_pass_is_walked_in_its_old_layer() {
        // Its display (@0x444ad) ran before `_HandleBeams` (@0x4453d)
        // disabled it, so the next pass's collision passes walk it in the
        // layer that display gave it (@0x43e75-0x43fd8); its display in
        // that pass puts it in the disabled layer (@0x2bd24).
        let catalog = beaming();
        let (mut session, a) = facing_a_frail_npc(&catalog);
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, &crate::ai::NovaAi::default());
        session.hold_trigger(crate::combat::armament::Trigger::default());
        assert_eq!(
            session.npc(npc_id(a)).map(|npc| npc.condition),
            Some(Condition::Disabled)
        );
        assert_eq!(walked_in(&session, a), ShipLayer::Ships, "next pass's walk");
        work(&mut session, &catalog, &crate::ai::NovaAi::default());
        assert_eq!(drawn_in(&session, a), ShipLayer::Disabled);
        assert_eq!(walked_in(&session, a), ShipLayer::Disabled);
    }

    #[test]
    fn the_frame_draws_a_ship_a_beam_disabled_in_its_old_layer() {
        // `_PlayGame` draws the frame (@0x45839) with each ship in the
        // layer its display gave it, before the beam disabled it.
        let catalog = beaming();
        let (mut session, a) = facing_a_frail_npc(&catalog);
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, &crate::ai::NovaAi::default());
        assert_eq!(
            session.npc(npc_id(a)).map(|npc| npc.condition),
            Some(Condition::Disabled)
        );
        assert_eq!(drawn_in(&session, a), ShipLayer::Ships);
    }

    #[test]
    fn the_player_a_beam_disabled_is_walked_in_the_disabled_layer_on_the_next_pass() {
        // The player's display (@0x43e29) runs just before the collision
        // passes, so the pass after a beam disables it walks it in the
        // disabled layer; the frame before draws it in its old one.
        let catalog = beaming();
        let mut session = facing_an_npc(&catalog, 180);
        session.pilot.reserves.shield.now = 0.0;
        session.pilot.reserves.armor.now = 16.0;
        work(&mut session, &catalog, &Firing);
        assert_eq!(session.player_condition(), Condition::Disabled);
        assert_eq!(drawn_in(&session, ShipRef::Player), ShipLayer::Ships);
        session.player_display_step(
            GamePass::Advance { double_time: false },
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(walked_in(&session, ShipRef::Player), ShipLayer::Disabled);
    }
}
