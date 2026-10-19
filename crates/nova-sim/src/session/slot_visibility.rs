//! Which ship slots are shown: a slot made active since its last display
//! is hidden until its next, and `_PrepareAllSprites` displays every slot
//! at once (rdm weapons-fidelity phase 42).
//!
//! # The original
//!
//! `_DoPlayGameWork`'s ship loop displays every slot 1-63, active or not
//! (@0x4445c-0x444c4, display @0x444ad). An inactive slot's display hides
//! its sprites (`SWSetSpriteVisible(.., 0)` @0x2b555-0x2b60a), and
//! `_SWCollideSpriteLayer` skips a hidden sprite (+0x14 @0xacba3). So a
//! slot made active since its last display stays hidden, unwalked and
//! undrawn, until its own first active display: a fighter `_LaunchFighter`
//! puts in from `_HandlePlayer` (@0x62c61), the eject's hulk (@0x6f30e),
//! a ship spawned below the slot whose `_HandleShip` spawned it. The beam
//! filter and `_CheckShotProximities` read the ship table, not the
//! sprite, so they still reach such a ship, as they reach a hidden one.
//!
//! `_PrepareAllSprites` (@0x3a75a-0x3a806) saves and sets the frozen flag
//! `*0x3b523c` (@0x3a770-0x3a773), which gates only a destroyed slot's
//! freeing and leader clear (@0x2ded6-0x2dee7), and runs
//! `_HandleShipDisplay` for slots 0-63 in order (@0x3a776-0x3a78e): every
//! active ship is shown and laid out (the layer block @0x2bd24-0x2bdad),
//! the player first, then the berths in ascending order, and every
//! inactive slot is hidden. No sound. It runs from
//! `_EnterGameFromMainScreen` @0x1a36c and `_PlayGame`'s entry @0x45834
//! ([`Session::load`] and the first populate, [`Session::populate`]),
//! `_PlayerEnterHypergate` @0x63e91 and `_PlayerEnterWormhole` @0x64743
//! (a gate exit, once the ships the player leads come out of it),
//! `_HandlePlayerDockRequest` @0x6764e and @0x67658 (the take-off; the
//! second changes no layer, as `_PutSpriteInLayer` keeps a slot already
//! there, @0x768c), and `_HandlePlayer` @0x6c59c (the jump's arrival, after
//! `_CreateAsteroids` and `_SWHideAllSprites`).
//!
//! # Here
//!
//! The session keeps which ship each slot last displayed and showed
//! ([`Session::is_displayed`]): each display step records it
//! ([`Session::lay_out_displayed`]), and [`Session::prepare_all_sprites`]
//! records every ship's at once. By [`RuleKey::UndisplayedShips`]' engine
//! reading a ship its slot has not displayed is left out of
//! [`Session::draw_layers`], so out of the frame and of the collision
//! passes' walk ([`Session::sprite_ranks`]); by the other reading every
//! ship is walked and drawn from the moment it is in the system. The
//! record is never carried into another session: every entry into flight
//! runs `_PrepareAllSprites` again.
//!
//! Left out, each with its owner: the rest of `_PrepareAllSprites` (the
//! frozen display's breakup step and final explosion, and the shots',
//! stellars', explosions', smoke's, boxes', asteroids', stars', target
//! sprites' and escape pods' displays), an escape ship's fighter shown in
//! its launch frame, and the hover over a slot not yet displayed:
//! rdm:phase/weapons-fidelity/phase-47-left-out-of-phase-42. The cloak
//! fade step:
//! rdm:phase/cloaking-fidelity/phase-7-escort-cloak-through-warp-prepare-and-capture.

use super::Session;
use super::player_led::slot_of;
use crate::combat::ShipRef;
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::{ShipLayer, Slot, disabled};

impl Session {
    /// Whether `ship`, at `slot`, is displayed: its slot's last display
    /// showed it, as [`RuleKey::UndisplayedShips`]' engine reading has it,
    /// or always by the other reading (see the module docs).
    pub(super) fn is_displayed(&self, ship: ShipRef, slot: Slot) -> bool {
        self.rules.source_for(RuleKey::UndisplayedShips) == RuleSource::Bible
            || self.slot_shown.get(&slot) == Some(&ship)
    }

    /// Notes `slot`'s display of `ship`: shown, it is displayed from now;
    /// hidden ([`Session::is_hidden`]), the slot displays nothing.
    pub(super) fn note_display(&mut self, ship: ShipRef, slot: Slot, shown: bool) {
        if shown {
            self.slot_shown.insert(slot, ship);
        } else {
            self.slot_shown.remove(&slot);
        }
    }

    /// `_PrepareAllSprites`' ship part (@0x3a776-0x3a78e): every slot is
    /// displayed, the player first, then each NPC by berth. Each ship
    /// shown puts its slot in its layer, the disabled one when
    /// `_IsDisabled` holds for it, else the escorts' one when the player
    /// leads it, else the ships' one, and is displayed from now; a hidden
    /// ship's slot keeps its place and displays nothing; a slot with no
    /// ship displays nothing. Each shown slot clones its type's frames
    /// ([`Session::lay_out_ships`]). See the module docs for where it
    /// runs.
    pub fn prepare_all_sprites(&mut self) {
        let mut slots = vec![(
            ShipRef::Player,
            Slot::Player,
            disabled(self.player_condition()),
            false,
        )];
        let mut npcs: Vec<_> = self
            .npcs()
            .iter()
            .map(|npc| {
                let held = disabled(npc.condition) || self.held_disabled(npc);
                (
                    ShipRef::Npc(npc.id),
                    slot_of(npc),
                    held,
                    npc.escorts_player(),
                )
            })
            .collect();
        npcs.sort_by_key(|&(_, slot, _, _)| slot);
        slots.extend(npcs);
        self.slot_shown.clear();
        for (ship, slot, disabled, led) in slots {
            let shown = !self.is_hidden(ship);
            self.note_display(ship, slot, shown);
            if shown {
                self.slot_sprites
                    .layers
                    .put(slot, ShipLayer::of(disabled, led));
            }
        }
        self.lay_out_ships();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MissionShip;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::gate::GateKind;
    use crate::rulebook::Rulebook;
    use crate::session::gate_follow::tests::{exit_gate_raising, following, gated};
    use crate::session::mission_ships::tests::{
        accepted, jump_to, shipping, ships_catalog, take_off,
    };
    use crate::testkit::FakePilotCatalog;
    use crate::traffic::npc::NpcId;

    /// Mission 140's two ships, which never join the player, there from
    /// the start in the system it is accepted in.
    fn strangers() -> FakePilotCatalog {
        ships_catalog(MissionShip {
            behav: 0,
            ..shipping(-1, 0)
        })
    }

    /// In flight after the take-off with [`strangers`], by `rules`.
    fn flying(rules: Rulebook) -> (FakePilotCatalog, Session) {
        let catalog = strangers();
        let mut session = accepted(&catalog).with_rules(rules);
        take_off(&mut session, &catalog);
        (catalog, session)
    }

    /// A copy of the session's first NPC added 500 pixels off, as a ship
    /// that came after every slot was last displayed.
    fn newcomer(session: &mut Session) -> NpcId {
        let mut npc = session.npcs()[0].clone();
        npc.state.position.x += 500.0;
        session.traffic.add_npc(npc)
    }

    fn walked(session: &Session, ship: ShipRef) -> bool {
        session
            .sprite_ranks()
            .iter()
            .any(|&(ranked, _)| ranked == ship)
    }

    fn drawn(session: &Session, ship: ShipRef) -> bool {
        session.draw_order().contains(&ship)
    }

    #[test]
    fn a_ship_new_since_its_slots_last_display_is_neither_walked_nor_drawn_until_it() {
        // An inactive slot's display hides its sprites (@0x2b555-0x2b60a),
        // and the walk skips a hidden sprite (@0xacba3), until the slot's
        // own display (@0x444ad).
        let (_, mut session) = flying(Rulebook::default());
        let new = ShipRef::Npc(newcomer(&mut session));
        assert!(!walked(&session, new), "not walked");
        assert!(!drawn(&session, new), "not drawn");
        let (_, terms) = session.fight_terms();
        assert_eq!(terms.last().map(|terms| terms.sprite), Some(None));
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(walked(&session, new), "walked once displayed");
        assert!(drawn(&session, new), "drawn once displayed");
    }

    #[test]
    fn by_the_other_reading_a_new_ship_is_walked_and_drawn_at_once() {
        let rules = Rulebook::default().with_override(RuleKey::UndisplayedShips, RuleSource::Bible);
        let (_, mut session) = flying(rules);
        let new = ShipRef::Npc(newcomer(&mut session));
        assert!(walked(&session, new));
        assert!(drawn(&session, new));
    }

    #[test]
    fn a_slot_freed_and_refilled_shows_the_new_ship_only_after_its_display() {
        let (_, mut session) = flying(Rulebook::default());
        let old = session.npcs()[0].clone();
        session.traffic.remove(old.id);
        let mut npc = old.clone();
        npc.state.position.x += 500.0;
        let new = session.traffic.add_npc_at(npc, old.berth);
        assert!(
            !drawn(&session, ShipRef::Npc(new)),
            "its slot showed another"
        );
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(drawn(&session, ShipRef::Npc(new)));
    }

    #[test]
    fn a_take_off_lays_every_ship_out_in_slot_order_before_the_first_frame() {
        // `_HandlePlayerDockRequest` runs `_PrepareAllSprites` (@0x6764e):
        // a slot whose last layer was the disabled one is in the ships'
        // layer at once with an intact ship.
        let catalog = strangers();
        let mut session = accepted(&catalog);
        for berth in 1..=2 {
            session
                .slot_sprites
                .layers
                .put(Slot::Berth(berth), ShipLayer::Disabled);
        }
        take_off(&mut session, &catalog);
        let layers = session.draw_layers();
        assert_eq!(layers.len(), 3, "the player and two: {layers:?}");
        for npc in session.npcs() {
            assert!(
                layers.contains(&(ShipRef::Npc(npc.id), ShipLayer::Ships)),
                "{layers:?}"
            );
        }
        // Laid out the player first, then by berth: berth 2 is on top.
        let order = session.draw_order();
        let at = |berth| {
            let npc = session.npcs().iter().find(|npc| npc.berth == berth);
            ShipRef::Npc(npc.expect("there").id)
        };
        assert_eq!(&order[order.len() - 2..], [at(1), at(2)]);
    }

    #[test]
    fn a_jump_arrival_displays_every_ship_before_the_first_frame() {
        // `_HandlePlayer` runs `_PrepareAllSprites` on arrival (@0x6c59c):
        // the mission's escorts that came along are in the escorts' layer.
        let catalog = ships_catalog(following(1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        let layers = session.draw_layers();
        assert!(session.player_led().count() > 0);
        for npc in session.npcs() {
            let layer = ShipLayer::of(disabled(npc.condition), npc.escorts_player());
            assert!(
                layers.contains(&(ShipRef::Npc(npc.id), layer)),
                "{layers:?}"
            );
        }
    }

    #[test]
    fn a_gate_exit_displays_every_ship_with_its_escorts_in_their_layer() {
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = gated(following(1, 0), 0);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            exit_gate_raising(&mut session, &catalog, kind, &mut NeverFires);
            let layers = session.draw_layers();
            let led: Vec<ShipRef> = session
                .player_led()
                .map(|npc| ShipRef::Npc(npc.id))
                .collect();
            assert!(!led.is_empty(), "{kind:?}");
            for ship in led {
                assert!(
                    layers.contains(&(ship, ShipLayer::Led)),
                    "{kind:?}: {layers:?}"
                );
            }
        }
    }

    #[test]
    fn a_ship_held_disabled_is_laid_out_in_the_disabled_layer_whatever_its_condition() {
        // `_HandleShipDisplay` asks `_IsDisabled` (@0x2b61f), which holds a
        // Derelict government's ship at any armour.
        let mut catalog = strangers();
        catalog.govts = vec![crate::catalog::GovtRecord {
            flags: crate::govt::DERELICT,
            ..nova_sim_catalog::fixture::govt(131)
        }];
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let id = session.npcs()[0].id;
        session.npc_mut(id).expect("there").condition = crate::combat::hull::Condition::Intact;
        session
            .slot_sprites
            .layers
            .put(slot_of(&session.npcs()[0]), ShipLayer::Ships);
        session.prepare_all_sprites();
        assert!(
            session
                .draw_layers()
                .contains(&(ShipRef::Npc(id), ShipLayer::Disabled))
        );
    }

    #[test]
    fn a_derelict_is_laid_out_in_the_disabled_layer_as_it_comes_in() {
        let mut catalog = strangers();
        catalog.govts = vec![crate::catalog::GovtRecord {
            flags: crate::govt::DERELICT,
            ..nova_sim_catalog::fixture::govt(131)
        }];
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        for npc in session.npcs() {
            assert!(
                session
                    .draw_layers()
                    .contains(&(ShipRef::Npc(npc.id), ShipLayer::Disabled))
            );
        }
    }
}
