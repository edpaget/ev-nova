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
//! inactive slot is hidden. Each active ship of the player's system
//! (+0x74 the player's, +0xb8 set, @0x2b7d8-0x2b7ed) steps its cloak fade
//! (@0x2d088-0x2d164) and, dying, takes the dying direction
//! (@0x2d169-0x2d17e); a ship failing that test skips to @0x2ded6, past
//! the step. No sound. It runs from `_EnterGameFromMainScreen` @0x1a36c
//! and `_PlayGame`'s entry @0x45834 ([`Session::load`] and the first
//! populate, [`Session::populate`]), `_PlayerEnterHypergate` @0x63e91
//! and `_PlayerEnterWormhole` @0x64743 (a gate entry, once the player's
//! system is the exit's, @0x639fd and @0x642a7, and before the old ships
//! are deleted, @0x67096, and the exit's setup, so the old ships pass the
//! system test only when the exit is in the system gone in),
//! `_HandlePlayerDockRequest` @0x6764e and @0x67658 (the take-off and a
//! gate exit, after the setup; the second changes no layer, as
//! `_PutSpriteInLayer` keeps a slot already there, @0x768c), and
//! `_HandlePlayer` @0x6c59c (the jump's arrival, after `_CreateAsteroids`
//! and `_SWHideAllSprites`).
//!
//! # Here
//!
//! The session keeps which ship each slot last displayed and showed
//! ([`Space::is_displayed`]): each display step records it
//! ([`Session::lay_out_displayed`]), and [`Session::prepare_all_sprites`]
//! records every ship's at once. By [`RuleKey::UndisplayedShips`]' engine
//! reading a ship its slot has not displayed is left out of
//! [`Session::draw_layers`], so out of the frame and of the collision
//! passes' walk ([`Session::sprite_ranks`]); by the other reading every
//! ship is walked and drawn from the moment it is in the system. The
//! record is never carried into another session: every entry into flight
//! runs `_PrepareAllSprites` again.
//!
//! **The fade.** Each [`Session::prepare_all_sprites`] steps the player's
//! cloak fade (slot 0) and then every NPC's once (`Traffic::step_fades`),
//! so before the first frame a take-off and a gate exit step it twice
//! (`Session::lift_off`, and `Session::come_out`) and a jump's arrival
//! once ([`Session::arrive`]). A gate entry steps the player's fade once
//! more first, whatever the exit, as the player's slot always passes the
//! display's system test (@0x2b7d8-0x2b7e0); and when its exit is in the
//! system gone in, the old ships' fades too
//! ([`Space::step_fades_at_gate_entry`]), which an escort's respawn then
//! carries; any other gate entry steps no NPC's. Every NPC in the traffic
//! is of the player's system and active, so each passes the system test
//! there.
//!
//! **The hover and click box** (rdm weapons-fidelity phase 47).
//! `_HandleMouse`'s loop (@0x380e3-0x381ec) and the click block
//! (`_HandlePlayer` @0x6a4ab-0x6a4f4) box each active NPC by its hull
//! sprite's rect (+0x1a-+0x21, @0x38177), with no visibility test. Only a
//! display places that rect (`SWSetSpriteLocation` @0x2cd9d, at
//! `trunc(ship − player)` per axis from the play area's centre,
//! @0x2cd38-0x2cd8e), shown or hidden alike, and clones its frames
//! (@0x2b824). So a ship its slot has not displayed (an escape fighter
//! below its carrier, an arrival after the loop) is hovered and clicked
//! where the slot's last ship was last laid out, in that ship's frames:
//! each display records the slot's place
//! ([`SlotPlaces`](crate::ship_layer::SlotPlaces)) and which ship it laid
//! out, and [`Session::hover_place`] and [`Session::framed_by`] read them,
//! by [`RuleKey::UndisplayedShips`].
//!
//! **`_PrepareAllSprites`' other parts** change nothing the player sees
//! (rdm weapons-fidelity phase 47's findings): no site that runs it holds
//! a dying ship, a live shot, a cargo box or a live asteroid field, and
//! the rest (explosions, smoke, stars, target and escape-pod sprites) is
//! display the view draws from state each frame. Its stellars' part isn't
//! display: it steps their timers and frames. [`Session::stellar_pass`]
//! runs it beside each call of [`Session::prepare_all_sprites`] that needs
//! it, and game entry's where the ships are set up (see the
//! `stellar_passes` module).
//!
//! Left out, each with its owner: the box of a slot never laid out this
//! run (`_InitSprites`' clone rect @0x1f135, here no box), and
//! `_PlayGame`'s frozen `_DoPlayGameWork(1)` at entry (@0x457f3-0x4580c):
//! rdm:phase/weapons-fidelity-followups/phase-2-left-out-of-weapons-fidelity-47.

use super::player_led::slot_of;
use super::{Session, Setup, Space};
use crate::combat::ShipRef;
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::{ShipLayer, Slot, disabled};

/// Where a ship's hover and click box is ([`Session::hover_place`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HoverPlace {
    /// Where the ship is drawn, or would be when hidden.
    Own,
    /// At this whole-pixel offset from where the player is drawn: where
    /// its slot was last laid out, by another ship.
    Stale(Vec2),
    /// Nowhere: its slot has not been laid out this run.
    Unplaced,
}

impl Session {
    /// Notes `slot`'s display of `ship`: shown, it is displayed from now;
    /// hidden ([`Session::is_hidden`]), the slot displays nothing. Either
    /// way the display lays the slot out at the ship's whole-pixel offset
    /// from the player (`SWSetSpriteLocation` @0x2cd9d; the player's own
    /// at none), which its hover box reads ([`Session::hover_place`]).
    pub(super) fn note_display(&mut self, ship: ShipRef, slot: Slot, shown: bool) {
        if shown {
            self.space.slot_shown.insert(slot, ship);
        } else {
            self.space.slot_shown.remove(&slot);
        }
        let offset = match ship {
            ShipRef::Player => Some(Vec2::ZERO),
            ShipRef::Npc(id) => self
                .space
                .npc(id)
                .map(|npc| npc.state.position - self.ship.player.position),
        };
        if let Some(offset) = offset {
            self.space.slot_placed.insert(slot, ship);
            self.space.slot_sprites.places.place(slot, offset);
        }
    }

    /// Where `ship`'s hover and click box is: `_HandleMouse` (@0x380e3-0x381ec)
    /// and the click block (`_HandlePlayer` @0x6a4ab-0x6a4f4) box a slot's
    /// hull sprite rect (@0x38177), which only a display places (see the
    /// module docs). A ship its slot's last display laid out, shown or
    /// hidden, is boxed where it is ([`HoverPlace::Own`]). By
    /// [`RuleKey::UndisplayedShips`]' engine reading, one its slot has not
    /// displayed is boxed at the offset from the player its slot was last
    /// laid out at ([`HoverPlace::Stale`]), in the frames its slot last
    /// cloned ([`Session::framed_by`]), or has no box in a slot no display
    /// has laid out this run ([`HoverPlace::Unplaced`]); by the other
    /// reading every ship is boxed where it is. A ship not here is boxed
    /// where it is, which is nowhere.
    #[must_use]
    pub(crate) fn hover_place(&self, ship: ShipRef) -> HoverPlace {
        self.space.hover_place(&self.setup, ship)
    }

    /// Takes NPC `id` out and puts a copy of it in its berth at `at`, at
    /// rest, for [`fixture::refill`](crate::fixture::refill): the copy's
    /// id, or none for an NPC not in the system.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn refill_berth(
        &mut self,
        id: crate::traffic::npc::NpcId,
        at: Vec2,
    ) -> Option<crate::traffic::npc::NpcId> {
        self.space.refill_berth(id, at)
    }

    /// `_PrepareAllSprites`' ship part (@0x3a776-0x3a78e): every slot is
    /// displayed, the player first, then each NPC by berth. Each ship
    /// shown puts its slot in its layer, the disabled one when
    /// `_IsDisabled` holds for it, else the escorts' one when the player
    /// leads it, else the ships' one, and is displayed from now; a hidden
    /// ship's slot keeps its place and displays nothing; a slot with no
    /// ship displays nothing. Each shown slot clones its type's frames
    /// (`Session::lay_out_ships`). The player's cloak fade steps first, as
    /// slot 0's display steps it, then every NPC's once
    /// (`Traffic::step_fades`), as each slot's display steps it
    /// (@0x2d088-0x2d17e): the step reads only its own ship, so one pass
    /// over the traffic is the slot loop's. See the module docs for where
    /// it runs.
    pub(crate) fn prepare_all_sprites(&mut self) {
        let mut slots = vec![(
            ShipRef::Player,
            Slot::Player,
            // The player defends no stellar.
            disabled(self.player_condition(), false),
            false,
        )];
        let mut npcs: Vec<_> = self
            .npcs()
            .iter()
            .map(|npc| {
                let held =
                    disabled(npc.condition, npc.defends.is_some()) || self.held_disabled(npc);
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
        self.space.slot_shown.clear();
        self.space.slot_placed.clear();
        for (ship, slot, disabled, led) in slots {
            let shown = !self.is_hidden(ship);
            self.note_display(ship, slot, shown);
            if shown {
                self.space
                    .slot_sprites
                    .layers
                    .put(slot, ShipLayer::of(disabled, led));
            }
        }
        self.ship.step_cloak_fade(&self.pilot, &self.statics);
        self.space.traffic.step_fades();
        self.lay_out_ships();
    }
}

impl Space {
    /// `_PlayerEnterHypergate`'s and `_PlayerEnterWormhole`'s
    /// `_PrepareAllSprites` (@0x63e91, @0x64743), at a gate entry whose
    /// exit is in system `exit`, the player in `here`: it runs once the
    /// player's system is the exit's (@0x639fd, @0x642a7) and before the
    /// old ships are deleted (`_DeleteAllComputerShips` @0x67096), so the
    /// old ships pass the display's system test (@0x2b7d8-0x2b7ed) only
    /// when the exit is in the system gone in. Then every NPC's fade
    /// steps once (`Traffic::step_fades`), before an escort's respawn
    /// carries it; otherwise none does. The player's own slot passes that
    /// test whatever the exit, so the session steps its fade on its own
    /// first (`PlayerShip::step_cloak_fade`). The rest of that display
    /// (layers, visibility) is redone by the exit's own two calls after
    /// the setup.
    pub(super) fn step_fades_at_gate_entry(
        &mut self,
        exit: crate::catalog::SystemId,
        here: crate::catalog::SystemId,
    ) {
        if exit == here {
            self.traffic.step_fades();
        }
    }

    /// Whether `ship`, at `slot`, is displayed: its slot's last display
    /// showed it, as [`RuleKey::UndisplayedShips`]' engine reading has it,
    /// or always by the other reading (see the module docs).
    pub(super) fn is_displayed(&self, setup: &Setup, ship: ShipRef, slot: Slot) -> bool {
        setup.rules.source_for(RuleKey::UndisplayedShips) == RuleSource::Bible
            || self.slot_shown.get(&slot) == Some(&ship)
    }

    /// Whether `ship`, at `slot`, was laid out by its slot's last display,
    /// shown or hidden, as [`RuleKey::UndisplayedShips`]' engine reading
    /// has it, or always by the other reading.
    pub(super) fn is_laid_out(&self, setup: &Setup, ship: ShipRef, slot: Slot) -> bool {
        setup.rules.source_for(RuleKey::UndisplayedShips) == RuleSource::Bible
            || self.slot_placed.get(&slot) == Some(&ship)
    }

    /// Where `ship`'s hover and click box is: `_HandleMouse` (@0x380e3-0x381ec)
    /// and the click block (`_HandlePlayer` @0x6a4ab-0x6a4f4) box a slot's
    /// hull sprite rect (@0x38177), which only a display places (see the
    /// module docs). A ship its slot's last display laid out, shown or
    /// hidden, is boxed where it is ([`HoverPlace::Own`]). By
    /// [`RuleKey::UndisplayedShips`]' engine reading, one its slot has not
    /// displayed is boxed at the offset from the player its slot was last
    /// laid out at ([`HoverPlace::Stale`]), in the frames its slot last
    /// cloned ([`Session::framed_by`]), or has no box in a slot no display
    /// has laid out this run ([`HoverPlace::Unplaced`]); by the other
    /// reading every ship is boxed where it is. A ship not here is boxed
    /// where it is, which is nowhere.
    #[must_use]
    fn hover_place(&self, setup: &Setup, ship: ShipRef) -> HoverPlace {
        let slot = match ship {
            ShipRef::Player => Slot::Player,
            ShipRef::Npc(id) => match self.npc(id) {
                Some(npc) => slot_of(npc),
                None => return HoverPlace::Own,
            },
        };
        if self.is_laid_out(setup, ship, slot) {
            return HoverPlace::Own;
        }
        self.slot_sprites
            .places
            .of(slot)
            .map_or(HoverPlace::Unplaced, HoverPlace::Stale)
    }

    /// Takes NPC `id` out and puts a copy of it in its berth at `at`, at
    /// rest, for [`fixture::refill`](crate::fixture::refill): the copy's
    /// id, or none for an NPC not in the system.
    #[cfg(any(test, feature = "fixture"))]
    pub(super) fn refill_berth(
        &mut self,
        id: crate::traffic::npc::NpcId,
        at: Vec2,
    ) -> Option<crate::traffic::npc::NpcId> {
        let mut copy = self.npc(id)?.clone();
        self.traffic.remove(id);
        copy.state.position = at;
        copy.state.velocity = Vec2::ZERO;
        let berth = copy.berth;
        Some(self.traffic.add_npc_at(copy, berth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MissionShip;
    use crate::chance::NeverFires;

    use crate::flight::Controls;
    use crate::gate::GateKind;
    use crate::rulebook::Rulebook;
    use crate::session::gate_follow::tests::{exit_gate_raising, following, gated};
    use crate::session::mission_ships::tests::{
        accepted, jump_to, shipping, ships_catalog, take_off,
    };
    use crate::testkit::FakePilotCatalog;
    use crate::testkit::work;
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
        session.space.traffic.add_npc(npc)
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
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
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
        session.space.traffic.remove(old.id);
        let mut npc = old.clone();
        npc.state.position.x += 500.0;
        let new = session.space.traffic.add_npc_at(npc, old.berth);
        assert!(
            !drawn(&session, ShipRef::Npc(new)),
            "its slot showed another"
        );
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
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
                .space
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
            let layer = ShipLayer::of(
                disabled(npc.condition, npc.defends.is_some()),
                npc.escorts_player(),
            );
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
        session.space.npc_mut(id).expect("there").condition =
            crate::combat::hull::Condition::Intact;
        session
            .space
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
    fn a_defence_ship_breaking_up_is_laid_out_in_the_ships_layer() {
        // `_IsDisabled` never calls a defence ship disabled by its armour
        // (@0x2d6f), so `_HandleShipDisplay` keeps it in `0x3b541c`.
        let bible = Rulebook::default().with_override(RuleKey::ShipOverlap, RuleSource::Bible);
        for rules in [Rulebook::default(), bible] {
            breaking_up_defence_ship_is_in_the_ships_layer(rules);
        }
    }

    fn breaking_up_defence_ship_is_in_the_ships_layer(rules: Rulebook) {
        let (_, mut session) = flying(rules);
        let id = session.npcs()[0].id;
        let npc = session.space.npc_mut(id).expect("there");
        npc.defends = Some(crate::catalog::StellarId(128));
        npc.condition = crate::combat::hull::Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let slot = slot_of(&session.npcs()[0]);
        session
            .space
            .slot_sprites
            .layers
            .put(slot, ShipLayer::Disabled);
        session.prepare_all_sprites();
        let layers = session.draw_layers();
        assert!(
            layers.contains(&(ShipRef::Npc(id), ShipLayer::Ships)),
            "{layers:?}"
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

    /// In flight by `rules` with A, the first NPC, laid out at
    /// (100.7, -20.3) from the player, then gone, and B, of another type,
    /// come into A's slot 500 pixels off since: the session, A's type and
    /// B.
    fn refilled(rules: Rulebook) -> (Session, crate::catalog::ShipId, ShipRef) {
        let (_, mut session) = flying(rules);
        let a = session.npcs()[0].clone();
        let at = session.player().position + crate::geometry::Vec2::new(100.7, -20.3);
        session.space.npc_mut(a.id).expect("there").state.position = at;
        session.prepare_all_sprites();
        let far = session.player().position + crate::geometry::Vec2::new(500.0, 0.0);
        let b = crate::fixture::refill(&mut session, a.id, far).expect("refilled");
        assert!(session.space.npc(a.id).is_none(), "A gone");
        let copy = session.space.npc_mut(b).expect("there");
        assert_eq!((copy.berth, copy.state.position), (a.berth, far));
        copy.ship = crate::catalog::ShipId(a.ship.0 + 1);
        (session, a.ship, ShipRef::Npc(b))
    }

    #[test]
    fn a_ship_its_slot_has_not_displayed_is_hovered_where_the_slot_last_was() {
        // `_HandleMouse` boxes a slot's hull sprite rect (@0x38177), which
        // only a display places (`SWSetSpriteLocation` @0x2cd9d, at the
        // whole-pixel offset from the player @0x2cd38-0x2cd8e), in the
        // frames it last cloned (@0x2b824).
        let (session, a_type, b) = refilled(Rulebook::default());
        assert_eq!(
            session.hover_place(b),
            HoverPlace::Stale(crate::geometry::Vec2::new(100.0, -20.0))
        );
        assert_eq!(session.framed_by(b), Some(a_type));
    }

    #[test]
    fn a_displayed_ship_is_hovered_where_it_is() {
        let (_, mut session) = flying(Rulebook::default());
        let a = ShipRef::Npc(session.npcs()[0].id);
        assert_eq!(session.hover_place(a), HoverPlace::Own);
        assert_eq!(session.hover_place(ShipRef::Player), HoverPlace::Own);
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(session.hover_place(a), HoverPlace::Own);
    }

    #[test]
    fn a_slot_never_laid_out_has_no_box() {
        let (_, mut session) = flying(Rulebook::default());
        let new = ShipRef::Npc(newcomer(&mut session));
        assert_eq!(session.hover_place(new), HoverPlace::Unplaced);
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(session.hover_place(new), HoverPlace::Own, "once displayed");
    }

    #[test]
    fn by_the_other_reading_an_undisplayed_ship_is_hovered_where_it_is() {
        let rules = Rulebook::default().with_override(RuleKey::UndisplayedShips, RuleSource::Bible);
        let (session, _, b) = refilled(rules);
        assert_eq!(session.hover_place(b), HoverPlace::Own);
        let own = session.npcs().iter().find(|npc| ShipRef::Npc(npc.id) == b);
        assert_eq!(session.framed_by(b), own.map(|npc| npc.ship));
    }

    #[test]
    fn prepare_all_sprites_steps_the_players_fade_and_every_npcs_once() {
        // `_HandleShipDisplay`'s fade step (@0x2d088-0x2d164) and dying
        // direction (@0x2d169-0x2d17e) run for every active ship of the
        // system (@0x3a776-0x3a78e), the player's slot 0 first, the
        // frozen flag gating only the freeing (@0x2ded6-0x2dee7).
        use crate::cloak::Cloak;
        use crate::combat::hull::Condition;
        let (_, mut session) = flying(Rulebook::default());
        session.ship.cloak.cloak = Cloak {
            fade: 4.0,
            direction: 1,
        };
        session.prepare_all_sprites();
        let player = Cloak {
            fade: 4.75,
            direction: 1,
        };
        assert_eq!(session.player_cloak(), player, "the player's");
        session.ship.condition = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        session.prepare_all_sprites();
        assert_eq!(
            session.player_cloak(),
            Cloak {
                fade: 5.5,
                direction: crate::cloak::DYING_DIRECTION
            },
            "the player's, dying"
        );
        let (_, mut session) = flying(Rulebook::default());
        let cloaks = [
            (
                Cloak {
                    fade: 4.0,
                    direction: 1,
                },
                false,
            ),
            (
                Cloak {
                    fade: 4.0,
                    direction: -1,
                },
                false,
            ),
            (
                Cloak {
                    fade: 10.0,
                    direction: 0,
                },
                false,
            ),
            (
                Cloak {
                    fade: 32.0,
                    direction: 0,
                },
                false,
            ),
            (
                Cloak {
                    fade: 12.0,
                    direction: 0,
                },
                true,
            ),
        ];
        let mut ids = vec![session.npcs()[0].id, session.npcs()[1].id];
        while ids.len() < cloaks.len() {
            ids.push(newcomer(&mut session));
        }
        for (&id, &(cloak, dying)) in ids.iter().zip(&cloaks) {
            let npc = session.space.npc_mut(id).expect("there");
            npc.cloak = cloak;
            if dying {
                npc.condition = Condition::Dying {
                    ticks_left: 0,
                    starting: true,
                };
            }
        }
        session.prepare_all_sprites();
        for (&id, &(cloak, dying)) in ids.iter().zip(&cloaks) {
            let mut stepped = cloak;
            stepped.display_step(false, dying);
            assert_eq!(session.space.npc(id).expect("there").cloak, stepped);
        }
        assert_eq!(
            session.space.npc(ids[4]).expect("there").cloak,
            Cloak {
                fade: 11.0,
                direction: crate::cloak::DYING_DIRECTION
            }
        );
    }

    #[test]
    fn lay_out_ships_keeps_an_undisplayed_slots_frames() {
        // The frames are cloned only at a display (@0x2b824).
        let (mut session, a_type, b) = refilled(Rulebook::default());
        session.lay_out_ships();
        let ShipRef::Npc(id) = b else {
            unreachable!("an NPC")
        };
        let slot = slot_of(session.space.npc(id).expect("there"));
        assert_eq!(session.slot_sprites().frames.of(slot), a_type);
    }
}
