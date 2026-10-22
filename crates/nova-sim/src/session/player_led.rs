//! The ships the player leads ([`Session::player_led`]): its hired and
//! captured escorts, the fighters it launched, and its mission escorts,
//! in ship-table order.
//!
//! **The original's one relation.** The original makes no difference
//! between a hired escort and a mission's `ShipBehav` 1 escort in the
//! ship table: each is a ship whose leader (+0x9a) is the player, 0. A
//! hire, a capture or a launch sets it (`_SpawnEscort` @0x3fedd,
//! `_DoShipCapture` @0x41419, `_LaunchFighter`), and so does a mission
//! escort's join (`_SetupShipsInSystem` @0x4309f-0x430c2, `_AIDispatch`
//! @0x8fc03-0x8fc22). Every site that asks whether a ship is the
//! player's escort reads that one field, and narrows it to the hired
//! escorts (AI type 6 with no mission mark, or the hired flag +0xbb) or
//! the fighters (AI type 5) only where it means to: saving, pay, the
//! escort landing screen and dialog, the hiring room, the fleet's holds,
//! jettison and fighter recall. The step-1 findings of
//! rdm:phase/player-led-ships/phase-1-one-player-led-relation list every
//! site with its address and owner.
//!
//! **Here.** The engine keeps the hired escorts in the pilot's records
//! and [`Session::fleet`](Session), each NPC carrying an
//! [`EscortDuty`](crate::escort::EscortDuty), and marks a mission escort
//! by its mission's mark ([`Npc::mission_escort`]). [`Session::player_led`]
//! is the original's relation over both: the NPCs whose
//! [`Npc::escorts_player`] holds, in ship-table order: by
//! [`Npc::berth`], the slot each takes in the ship table. A new ship takes
//! the first free berth (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), a
//! hired escort as it is hired, in the bar (`_SpawnEscort` @0x3fe8f), and
//! the hired escorts keep theirs across each setup, as
//! `_DeleteAllComputerShips` (@0x3bf5f) keeps them in the table.
//! The sites that ask it narrow it as the original does:
//! [`Npc::is_players`] for the hired escorts and fighters,
//! [`Npc::mission_escort`] for the mission escorts. A disabled mission
//! escort that has joined stays led, as the original leaves a disabled
//! ship's leader as it was (@0x8fc0c); a hired escort disabled or
//! released leaves the fleet and is no longer led. A ship the player
//! leads that starts to break up is no longer led, as `_HandleShip`'s
//! dying branch (@0x350ab-0x3512b) clears its leader, and a trader among
//! them takes its share of the fleet's cargo as it goes (see
//! [`escorts`](super::escorts) and
//! [`mission_escorts`](super::mission_escorts)). Every ship the player
//! leads flies at its top speed × the led speed, as the player does: 1.5
//! for a pilot without Strict Play by the engine (`_ShipMaxSpeed`
//! @0x382e-0x384f; see [`top_speed`](crate::top_speed) and
//! [`Npc::top_speed`]), and its burst out of a gate ends at the lower of
//! its own top speed and the player's (`_HandleShip` @0x34c68-0x34ce6).
//! `_HandleShip`'s other
//! branches for led ships: its "disabled, leaving" branch
//! (@0x339a6-0x33a36) cannot run, as phase 10's step-1 findings say; a
//! disabled former escort that repairs itself rejoins the fleet as what
//! it was (@0x33a3b-0x33bd9, see [`self_repair`](super::self_repair));
//! and a led ship warps out with the player: each warp-stage pass the
//! player's `_MakeEscortsHyperWithParent` puts it on its jump (see the
//! `warp_stage` module), and its warp-out runs on the player's clock and
//! warp factor (@0x347c1-0x348bd; `nova_sim::traffic::warp_out`), so it
//! moves off once t > 0.45·d/w², after the player, and arrives with it
//! (see [`escorts`](super::escorts)).
//!
//! **Drawn.** `_HandleShipDisplay` treats a ship the player leads apart
//! in three branches. It draws it in the escorts' layer, under the other
//! ships and the player and over the disabled ones (@0x2bd24-0x2bd47,
//! [`Session::draw_order`]; see [`ship_layer`](crate::ship_layer)), in
//! the order each ship entered it, so a ship that stops being led goes
//! on top of the other ships and the player, as `_PutSpriteInLayer`
//! leaves it (@0x766c; [`RuleKey::ShipOverlap`] chooses). At a
//! full cloak it draws it faint where a stranger is not drawn at all
//! (@0x2d187-0x2d1cd, [`Session::cloak_shown`]). Its third branch, a
//! trader escort's share of the fleet's cargo destroyed as its final
//! explosion goes off (@0x2dbf5-0x2dc46), cannot run: a led ship's leader
//! is cleared as it is disabled or starts to break up. None of them plays
//! a sound.

use super::{PlayerShip, Session, Setup, Space};
use crate::catalog::ShipId;
use crate::combat::ShipRef;
use crate::combat::hull::HullSpec;
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::{ShipLayer, Slot, SlotSprites, disabled, in_draw_order};
use crate::traffic::npc::Npc;

impl Session {
    /// The ships the player leads, in ship-table order (see the module
    /// docs).
    pub fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        self.space.player_led()
    }

    /// The ships in the order they are drawn, bottom first: the disabled,
    /// then the ships the player leads, then the rest (see
    /// [`ship_layer`](crate::ship_layer)). By [`RuleKey::ShipOverlap`]:
    /// by the engine, each ship in the layer its slot's last display step
    /// put it in, and within each layer in the order each slot entered it
    /// (see [`Session::lay_out_displayed`]); by the other reading, each
    /// ship in the layer it stands in now, in the session's NPC order with
    /// the player on top. A hidden ship ([`Session::is_hidden`]), the
    /// player too, is left out by either.
    ///
    /// By the engine the frame is drawn as `_PlayGame` draws it
    /// (@0x45839), each ship where its own display left it, even when a
    /// beam disabled it after (`_HandleBeams` @0x4453d). A ship its slot
    /// has not yet displayed is left out by either reading, as its slot's
    /// sprites are hidden until then, unless [`RuleKey::UndisplayedShips`]
    /// reads otherwise (`Space::is_displayed`; see the
    /// `slot_visibility` module).
    #[must_use]
    pub fn draw_order(&self) -> Vec<ShipRef> {
        self.draw_layers()
            .into_iter()
            .map(|(ship, _)| ship)
            .collect()
    }

    /// The ships in [`Session::draw_order`], each with the layer it is
    /// drawn in, which the view stacks the shots among
    /// ([`sprite_layer`](crate::sprite_layer)).
    #[must_use]
    pub fn draw_layers(&self) -> Vec<(ShipRef, ShipLayer)> {
        let (order, layers): (Vec<ShipRef>, Vec<(ShipRef, ShipLayer)>) =
            match self.setup.rules.source_for(RuleKey::ShipOverlap) {
                RuleSource::Engine => {
                    let entries = &self.space.slot_sprites.layers;
                    let mut ships = self.shown_slots();
                    ships.retain(|&(ship, slot)| self.space.is_displayed(&self.setup, ship, slot));
                    let order = entries.in_entry_order(ships.iter().copied());
                    let layers = ships
                        .into_iter()
                        .map(|(ship, slot)| (ship, entries.rank(slot).0));
                    (order, layers.collect())
                }
                RuleSource::Bible => {
                    let npcs = self
                        .npcs()
                        .iter()
                        .filter(|npc| self.setup.shown(&npc.hull))
                        .filter(|npc| {
                            self.space
                                .is_displayed(&self.setup, ShipRef::Npc(npc.id), slot_of(npc))
                        })
                        .map(|npc| {
                            let layer =
                                ShipLayer::of(disabled(npc.condition), npc.escorts_player());
                            (ShipRef::Npc(npc.id), layer)
                        });
                    let player = (self.setup.shown(&self.ship.fit.hull)
                        && self
                            .space
                            .is_displayed(&self.setup, ShipRef::Player, Slot::Player))
                    .then(|| (ShipRef::Player, self.ship.player_layer()));
                    let layers: Vec<(ShipRef, ShipLayer)> = npcs.chain(player).collect();
                    (in_draw_order(layers.iter().copied()), layers)
                }
            };
        order
            .into_iter()
            .filter_map(|ship| layers.iter().find(|&&(drawn, _)| drawn == ship).copied())
            .collect()
    }

    /// Whether `ship` is hidden: its type's `Cost` is -1
    /// ([`HullSpec::hidden`]) and [`RuleKey::HiddenShips`] reads the
    /// engine. `_HandleShipDisplay` hides such a ship, the player's too,
    /// as it stands in the player's system (@0x2b7d8-0x2b7f3), and skips
    /// its layer block (@0x2cc6b), so its slot keeps its layer place. The
    /// original's hidden sprite is still positioned (@0x2cd17), so the
    /// pointer still hovers it (see [`hover`](crate::hover)), by the box
    /// of the frames its slot last cloned ([`Session::framed_by`]). A
    /// ship that is not here is not hidden.
    #[must_use]
    pub fn is_hidden(&self, ship: ShipRef) -> bool {
        let hull = match ship {
            ShipRef::Player => Some(self.ship.fit.hull),
            ShipRef::Npc(id) => self.space.npc(id).map(|npc| npc.hull),
        };
        hull.is_some_and(|hull| !self.setup.shown(&hull))
    }

    /// The ship type whose frames `ship`'s slot sprite holds, the sheet
    /// its hover box is sized by, or `None` for a ship that is not here.
    /// A shown ship's slot holds its own type's, as `_HandleShipDisplay`
    /// clones them each frame (@0x2b824). A hidden ship's
    /// ([`Session::is_hidden`]) holds, by
    /// [`RuleKey::HiddenShipHoverBox`]'s engine reading, whatever its
    /// slot last cloned: the type last shown there, or `shïp` 128 in a
    /// slot no shown ship has used this run (see
    /// [`SlotFrames`](crate::ship_layer::SlotFrames)); by the other
    /// reading, its own type's. A ship its slot's last display did not
    /// lay out holds, by [`RuleKey::UndisplayedShips`]' engine reading,
    /// whatever its slot last cloned, as the frames are cloned only at a
    /// display (@0x2b824; see [`Session::hover_place`]).
    #[must_use]
    pub fn framed_by(&self, ship: ShipRef) -> Option<ShipId> {
        let (slot, own, hull) = match ship {
            ShipRef::Player => (Slot::Player, self.pilot.ship, self.ship.fit.hull),
            ShipRef::Npc(id) => {
                let npc = self.space.npc(id)?;
                (slot_of(npc), npc.ship, npc.hull)
            }
        };
        let hidden = !self.setup.shown(&hull)
            && self.setup.rules.source_for(RuleKey::HiddenShipHoverBox) == RuleSource::Engine;
        let stale = hidden || !self.space.is_laid_out(&self.setup, ship, slot);
        Some(if stale {
            self.space.slot_sprites.frames.of(slot)
        } else {
            own
        })
    }

    /// Every ship type [`Session::framed_by`] gives for a ship here now
    /// other than the player's own type for the player, whose sheet is
    /// read with its ship: the sheets the view must read to box them,
    /// the player's slot first when it holds another type's frames.
    pub fn framed_types(&self) -> impl Iterator<Item = ShipId> + '_ {
        let player = self
            .framed_by(ShipRef::Player)
            .filter(|&ship| ship != self.pilot.ship);
        let npcs = self
            .npcs()
            .iter()
            .filter_map(|npc| self.framed_by(ShipRef::Npc(npc.id)));
        player.into_iter().chain(npcs)
    }

    /// Each ship slot's draw layer and when it entered it, as the last
    /// flight tick laid them out ([`Session::draw_order`]), and the frames
    /// it last cloned ([`Session::framed_by`]): the history to hand a
    /// session loaded later in the same run
    /// ([`Session::with_slot_sprites`]).
    #[must_use]
    pub fn slot_sprites(&self) -> &SlotSprites {
        self.space.slot_sprites()
    }

    /// The session, its ship slots' layers and cloned frames, and its shot
    /// slots' places and rolls, starting
    /// from `slot_sprites`, the history a session flown earlier in the
    /// same run left ([`Session::slot_sprites`]), in place of
    /// `_InitSprites`' order and frames. The original's sprites outlive a
    /// pilot: `_InitSprites` runs once, from `_InitEVNova` (@0x47382),
    /// and nothing between pilots re-lays or rebuilds a slot's sprites
    /// (see [`ship_layer`](crate::ship_layer)).
    #[must_use]
    pub fn with_slot_sprites(mut self, slot_sprites: SlotSprites) -> Self {
        self.space.combat.set_shot_slots(slot_sprites.shots.clone());
        self.space.slot_sprites = slot_sprites;
        self
    }

    /// Puts each ship the fight's display steps displayed since they were
    /// last taken ([`Combat::take_displays`](crate::combat::Combat::take_displays)),
    /// in display order, in its layer: the disabled one if `_IsDisabled`
    /// held at its display (@0x2b61f), else the escorts' one if the player
    /// leads it, else the ships' one (`_HandleShipDisplay`'s layer block,
    /// @0x2bd24-0x2bdad; [`LayerEntries::put`](crate::ship_layer::LayerEntries::put)).
    /// A hidden ship ([`Session::is_hidden`]) skips the block (@0x2cc6b),
    /// and its slot keeps its place.
    ///
    /// The session runs it after the player's display step
    /// ([`Session::player_display_step`], @0x43e29) and after the NPCs'
    /// (`Session::ship_loop`, @0x444ad), so the collision passes walk
    /// the player in the layer its display just gave it and each NPC in
    /// the one its last display gave it, and the frame is drawn so. Each
    /// display is noted ([`Session::note_display`]): a ship shown is
    /// displayed from then on ([`Space::is_displayed`]).
    pub(super) fn lay_out_displayed(&mut self) {
        for displayed in self.space.combat.take_displays() {
            let hidden = self.is_hidden(displayed.ship);
            self.note_display(displayed.ship, displayed.slot, !hidden);
            if hidden {
                continue;
            }
            let led = match displayed.ship {
                ShipRef::Player => false,
                ShipRef::Npc(id) => self.space.npc(id).is_some_and(Npc::escorts_player),
            };
            let layer = ShipLayer::of(displayed.disabled, led);
            self.space.slot_sprites.layers.put(displayed.slot, layer);
        }
    }

    /// Keeps the fight's shot slots' history
    /// ([`Combat::shot_slots`](crate::combat::Combat::shot_slots)), for a
    /// later session to start from, and has each shown ship's slot clone
    /// its type's frames
    /// ([`SlotFrames::clone_frames`](crate::ship_layer::SlotFrames::clone_frames)),
    /// as `_HandleShipDisplay` does before its layer block (@0x2b824); a
    /// hidden ship's slot keeps the ones it had, and so, by
    /// [`RuleKey::UndisplayedShips`]' engine reading, does the slot of a
    /// ship it has not displayed (`Space::is_displayed`). Run at the
    /// end of each flight tick ([`Session::fight_tail`]). Recording the
    /// clones at the end of the tick rather than at each ship's display
    /// never crosses a frame boundary. The layers are put at each display
    /// step ([`Session::lay_out_displayed`]).
    pub(super) fn lay_out_ships(&mut self) {
        self.space.slot_sprites.shots = self.space.combat.shot_slots();
        for (ship, slot) in self.shown_slots() {
            if !self.space.is_displayed(&self.setup, ship, slot) {
                continue;
            }
            let ship = match ship {
                ShipRef::Player => Some(self.pilot.ship),
                ShipRef::Npc(id) => self.space.npc(id).map(|npc| npc.ship),
            };
            if let Some(ship) = ship {
                self.space.slot_sprites.frames.clone_frames(slot, ship);
            }
        }
    }

    /// The player, then each NPC in berth order (ships at one berth in
    /// NPC order), with its slot: every ship but the hidden ones
    /// ([`Session::is_hidden`]), whose slots are neither laid out nor
    /// drawn.
    fn shown_slots(&self) -> Vec<(ShipRef, Slot)> {
        let mut npcs: Vec<&Npc> = self
            .npcs()
            .iter()
            .filter(|npc| self.setup.shown(&npc.hull))
            .collect();
        npcs.sort_by_key(|npc| npc.berth);
        let npcs = npcs
            .into_iter()
            .map(|npc| (ShipRef::Npc(npc.id), slot_of(npc)));
        let player = self
            .setup
            .shown(&self.ship.fit.hull)
            .then_some((ShipRef::Player, Slot::Player));
        player.into_iter().chain(npcs).collect()
    }
}

impl Setup {
    /// Whether a ship of `hull` is drawn and laid out (see
    /// [`Session::is_hidden`]).
    fn shown(&self, hull: &HullSpec) -> bool {
        !hull.hidden || self.rules.source_for(RuleKey::HiddenShips) == RuleSource::Bible
    }
}

impl PlayerShip {
    /// The layer the player stands in: never led.
    fn player_layer(&self) -> ShipLayer {
        ShipLayer::of(disabled(self.player_condition()), false)
    }
}

impl Space {
    /// The ships the player leads, in ship-table order (see the module
    /// docs).
    pub(super) fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        self.traffic.player_led()
    }

    /// Each ship slot's draw layer and when it entered it, as the last
    /// flight tick laid them out ([`Session::draw_order`]), and the frames
    /// it last cloned ([`Session::framed_by`]): the history to hand a
    /// session loaded later in the same run
    /// ([`Session::with_slot_sprites`]).
    #[must_use]
    fn slot_sprites(&self) -> &SlotSprites {
        &self.slot_sprites
    }
}

/// An NPC's sprite slot: its berth.
pub(super) fn slot_of(npc: &Npc) -> Slot {
    Slot::Berth(npc.berth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionShip, ShipId};
    use crate::combat::hull::Condition;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::rulebook::Rulebook;
    use crate::session::mission_ships::tests::{
        accepted, jump_to, shipping, ships_catalog, take_off, ticks,
    };
    use crate::traffic::npc::NpcId;
    use crate::traffic::warp_out::WarpGoal;
    use crate::warp;

    /// Mission 140's two ships, of `ShipBehav` `behav`, there from the
    /// start in the system the mission is accepted in.
    fn ships(behav: i16) -> MissionShip {
        MissionShip {
            behav,
            ..shipping(-1, 0)
        }
    }

    /// A hired escort of ship 200.
    fn hired() -> Escort {
        Escort {
            ship: ShipId(200),
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    /// The NPCs the player leads.
    fn led(session: &Session) -> Vec<NpcId> {
        session.player_led().map(|npc| npc.id).collect()
    }

    /// In flight with mission 140's ships of `ShipBehav` `behav` and a
    /// hired escort: the session, the mission's ships and the hired
    /// escort's NPC.
    fn flying(behav: i16) -> (Session, Vec<NpcId>, NpcId) {
        let catalog = ships_catalog(ships(behav));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        let marked = session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some())
            .map(|npc| npc.id)
            .collect();
        let hired = session.space.escorts.fleet[0].expect("placed");
        (session, marked, hired)
    }

    /// Thrusts the player and ticks the traffic `n` times, each NPC 5000
    /// pixels behind the player at the start, and gives the fastest each
    /// ship went: the player's, then each NPC's by id.
    fn fastest_under_thrust(
        session: &mut Session,
        catalog: &crate::testkit::FakePilotCatalog,
        n: usize,
    ) -> (f32, std::collections::BTreeMap<NpcId, f32>) {
        let behind = session.player().position - crate::geometry::Vec2::new(0.0, 5000.0);
        for npc in session.space.traffic.npcs_mut() {
            npc.state.position = behind;
            npc.state.velocity = crate::geometry::Vec2::ZERO;
        }
        let thrust = crate::flight::Controls {
            thrust: true,
            turn: crate::flight::Turn::None,
            reverse: false,
            ..crate::flight::Controls::default()
        };
        let mut player: f32 = 0.0;
        let mut npcs = std::collections::BTreeMap::new();
        for _ in 0..n {
            session.tick(thrust);
            ticks(session, catalog, 1);
            player = player.max(session.player().velocity.length());
            for npc in session.npcs() {
                let top = npcs.entry(npc.id).or_insert(0.0_f32);
                *top = top.max(npc.state.velocity.length());
            }
        }
        (player, npcs)
    }

    #[test]
    fn the_player_and_the_ships_it_leads_fly_half_as_fast_again_and_an_unled_ship_at_its_own() {
        for strict_play in [false, true] {
            let catalog = ships_catalog(ships(1));
            let mut session = accepted(&catalog);
            session.pilot.escorts.push(hired());
            session.pilot.strict_play = strict_play;
            take_off(&mut session, &catalog);
            let mission: Vec<NpcId> = session
                .npcs()
                .iter()
                .filter(|npc| npc.mission.is_some())
                .map(|npc| npc.id)
                .collect();
            let hired = session.space.escorts.fleet[0].expect("placed");
            let class = session.space.npc(hired).expect("there").clone();
            let unled = session.space.traffic.add_npc(Npc {
                escort: None,
                ..class
            });
            assert_eq!(led(&session), [hired, mission[0], mission[1]]);
            let factor = if strict_play { 1.0 } else { 1.5 };
            let own = session.handling().max_speed;
            let (player, npcs) = fastest_under_thrust(&mut session, &catalog, 300);
            assert!(
                (player - own * factor).abs() < 1e-3,
                "{strict_play}: the player at {player}"
            );
            for id in [hired, mission[0], mission[1]] {
                let own = session.space.npc(id).expect("there").stats.handling;
                // Within half a tick's thrust of it, where the autopilot
                // coasts, or past it by under a push, which the per-axis
                // cap lets through.
                let top = own.max_speed * factor;
                assert!(
                    npcs[&id] <= top + own.accel + 1e-4
                        && npcs[&id] >= top - own.accel / 2.0 - 1e-4,
                    "{strict_play}: {id:?} at {}",
                    npcs[&id]
                );
            }
            let own = session.space.npc(unled).expect("there").stats.handling;
            // Its autopilot coasts once within half a tick's thrust of the
            // speed it wants.
            let top = own.max_speed;
            assert!(
                npcs[&unled] <= top + own.accel + 1e-4
                    && npcs[&unled] >= top - own.accel / 2.0 - 1e-4,
                "{strict_play}: the unled ship, flat out at {}",
                npcs[&unled]
            );
        }
    }

    /// NPC `id`'s formation slot, if it has one.
    fn slot(session: &Session, id: NpcId) -> Option<u8> {
        session
            .space
            .npc(id)
            .and_then(Npc::formation)
            .map(|place| place.slot)
    }

    /// NPC `id`'s berth.
    fn berth(session: &Session, id: NpcId) -> u32 {
        session.space.npc(id).expect("in the system").berth
    }

    #[test]
    fn the_player_leads_hired_and_mission_escorts_in_ship_table_order() {
        let (session, mission, hired) = flying(1);
        assert_eq!(mission.len(), 2);
        // The hired escort, in the pilot's records with no ship yet, takes
        // the first slot of the ship table, as a loaded pilot's escorts do
        // (`_LoadPilotData` @0x75a25); the mission's ships, set up with the
        // system, fill the next free slots.
        assert_eq!(led(&session), [hired, mission[0], mission[1]]);
        assert_eq!(berth(&session, hired), 1);
        assert_eq!(slot(&session, hired), Some(2));
        assert_eq!(slot(&session, mission[0]), Some(3));
        assert_eq!(slot(&session, mission[1]), Some(4));
    }

    #[test]
    fn hired_and_mission_escorts_take_formation_slots_in_ship_table_order() {
        let catalog = ships_catalog(MissionShip {
            behav: 1,
            ..shipping(-6, 0)
        });
        let mut session = accepted(&catalog);
        session.pilot.escorts.extend([hired(), hired()]);
        take_off(&mut session, &catalog);
        let (a, b) = (
            session.space.escorts.fleet[0].expect("placed"),
            session.space.escorts.fleet[1].expect("placed"),
        );
        let mission = |session: &Session| -> Vec<NpcId> {
            session
                .npcs()
                .iter()
                .filter(|npc| npc.mission_escort())
                .map(|npc| npc.id)
                .collect()
        };
        let m = mission(&session);
        assert_eq!(m.len(), 2);
        let berths = |session: &Session, ids: &[NpcId]| -> Vec<u32> {
            ids.iter().map(|&id| berth(session, id)).collect()
        };
        assert_eq!(berths(&session, &[a, b, m[0], m[1]]), [1, 2, 3, 4]);
        let slots = |session: &Session, ids: &[NpcId]| -> Vec<Option<u8>> {
            ids.iter().map(|&id| slot(session, id)).collect()
        };
        assert_eq!(
            slots(&session, &[a, b, m[0], m[1]]),
            [Some(2), Some(3), Some(4), Some(5)]
        );

        // A is lost: B keeps its slot of the table across the jump, and the
        // mission escorts, set up again, fill the first free slots around it.
        session.dismiss(a);
        jump_to(&mut session, &catalog, 131);
        let b = session.space.escorts.fleet[0].expect("placed");
        let m = mission(&session);
        assert_eq!(m.len(), 2);
        assert_eq!(berths(&session, &[m[0], b, m[1]]), [1, 2, 3]);
        assert_eq!(led(&session), [m[0], b, m[1]]);
        assert_eq!(
            slots(&session, &[m[0], b, m[1]]),
            [Some(2), Some(3), Some(4)]
        );
        let widths: Vec<Option<f32>> = [m[0], b, m[1]]
            .iter()
            .map(|&id| session.space.npc(id).expect("here").hull.sprite_size)
            .collect();
        let spacing = crate::escort::spacing(session.ship.fit.hull.sprite_size, &widths);
        for id in [m[0], b, m[1]] {
            let place = session
                .space
                .npc(id)
                .and_then(Npc::formation)
                .expect("a slot");
            assert_eq!((place.ships, place.spacing), (4, spacing));
        }
    }

    #[test]
    fn an_escort_with_no_ship_yet_fills_the_gap_a_lost_one_left_in_the_ship_table() {
        let catalog = ships_catalog(MissionShip {
            behav: 1,
            ..shipping(-6, 0)
        });
        let mut session = accepted(&catalog);
        session.pilot.escorts.extend([hired(), hired(), hired()]);
        take_off(&mut session, &catalog);
        let placed: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        let [a, b, c] = [placed[0], placed[1], placed[2]];
        assert_eq!([a, b, c].map(|id| berth(&session, id)), [1, 2, 3]);

        // B is lost, and D, an escort with no ship yet (as a loaded
        // pilot's), takes the gap B left, below the kept escorts' berths,
        // never one of theirs.
        session.dismiss(b);
        session.pilot.escorts.push(hired());
        jump_to(&mut session, &catalog, 131);
        let fleet: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        let (a, c, d) = (fleet[0], fleet[1], fleet[2]);
        let mission: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.mission_escort())
            .map(|npc| npc.id)
            .collect();
        assert_eq!(mission.len(), 2);
        let order = [a, d, c, mission[0], mission[1]];
        assert_eq!(order.map(|id| berth(&session, id)), [1, 2, 3, 4, 5]);
        assert_eq!(led(&session), order);
        assert_eq!(
            order.map(|id| slot(&session, id)),
            [Some(2), Some(3), Some(4), Some(5), Some(6)]
        );
    }

    #[test]
    fn a_mission_escort_set_up_at_take_off_comes_before_an_escort_hired_at_a_busy_spaceport() {
        let mut catalog = ships_catalog(ships(1));
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id.0 == 128) {
                site.flags |= crate::landing::StellarFlags::BAR;
                site.tech_level = 7;
            }
        }
        for record in &mut catalog.ship_records {
            if record.id == ShipId(200) {
                record.hire_random = 100;
            }
        }
        let mut session = accepted(&catalog);
        session.pilot.cash = 1_000_000;
        let ship = || crate::testkit::npc(0, crate::stats::ShipStats::default());
        let crowd = [
            session.space.traffic.add_npc(ship()),
            session.space.traffic.add_npc(ship()),
        ];
        assert_eq!(crowd.map(|id| berth(&session, id)), [1, 2]);
        session
            .hire(ShipId(200), &mut crate::chance::NeverFires)
            .expect("hired");
        // `_SpawnEscort` puts it in the table at once, at the first free
        // slot among the landed system's ships (@0x3fe8f).
        let hired = session.space.escorts.fleet[0].expect("placed while landed");
        assert_eq!(berth(&session, hired), 3);
        take_off(&mut session, &catalog);
        let hired = session.space.escorts.fleet[0].expect("placed again");
        let mission: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.mission_escort())
            .map(|npc| npc.id)
            .collect();
        assert_eq!(mission.len(), 2);
        let order = [mission[0], mission[1], hired];
        assert_eq!(order.map(|id| berth(&session, id)), [1, 2, 3]);
        assert_eq!(led(&session), order);
        assert_eq!(
            order.map(|id| slot(&session, id)),
            [Some(2), Some(3), Some(4)]
        );
    }

    #[test]
    fn a_mission_ship_that_never_joins_is_not_led() {
        let (session, mission, hired) = flying(0);
        assert_eq!(mission.len(), 2);
        assert_eq!(led(&session), [hired]);
    }

    #[test]
    fn a_released_hired_escort_is_no_longer_led() {
        let (mut session, mission, hired) = flying(1);
        session.release(hired);
        assert_eq!(led(&session), mission);
        assert!(
            session.space.npc(hired).is_some(),
            "still here as it leaves"
        );
    }

    #[test]
    fn a_joined_mission_escort_disabled_stays_led() {
        let catalog = ships_catalog(ships(1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let mission = led(&session);
        assert_eq!(mission.len(), 2);
        session.space.traffic.npcs_mut()[0].condition = Condition::Disabled;
        ticks(&mut session, &catalog, 5);
        assert_eq!(led(&session), mission);
    }

    /// The draw order's NPCs as ids, `Player` as `None`.
    fn drawn(session: &Session) -> Vec<Option<NpcId>> {
        session
            .draw_order()
            .into_iter()
            .map(|ship| match ship {
                ShipRef::Npc(id) => Some(id),
                ShipRef::Player => None,
            })
            .collect()
    }

    /// [`flying`] with mission escorts, plus a traffic ship the player
    /// does not lead, first in NPC order: the session, the ships it leads
    /// in NPC order and the traffic ship.
    fn with_traffic() -> (Session, Vec<NpcId>, NpcId) {
        let (mut session, _, hired) = flying(1);
        let class = session.space.npc(hired).expect("there").clone();
        let traffic = session.space.traffic.add_npc(Npc {
            escort: None,
            ..class
        });
        session.space.traffic.npcs_mut().rotate_right(1);
        // Every slot displayed, as a system entry displays them
        // (`_PrepareAllSprites` @0x3a75a).
        session.prepare_all_sprites();
        assert_eq!(session.npcs()[0].id, traffic);
        let led: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.escorts_player())
            .map(|npc| npc.id)
            .collect();
        assert_eq!(led.len(), 3, "a hired escort and two mission escorts");
        assert!(led.contains(&hired));
        (session, led, traffic)
    }

    #[test]
    fn the_ships_the_player_leads_warp_out_with_it_on_its_clock() {
        let catalog = ships_catalog(ships(1));
        let (mut session, led, traffic) = with_traffic();
        let class = session.space.npc(led[0]).expect("there").clone();
        let fighter = session.space.traffic.add_npc(Npc {
            escort: None,
            mission: None,
            leader: Some(led[0]),
            ..class
        });
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        session.ship.player = crate::flight::ShipState {
            position: crate::geometry::Vec2::new(0.0, 1200.0),
            velocity: crate::geometry::Vec2::ZERO,
            heading: 90.0,
        };
        session.begin_jump().expect("jumps");
        assert!(session.jumping().is_some(), "facing the bearing at rest");
        let aim = session
            .chart
            .jump_aim(&session.pilot, crate::catalog::SystemId(131))
            .expect("on the map");
        assert_eq!(aim, 90, "east, as the player faces");
        for &id in &led {
            let npc = session.space.npc_mut(id).expect("there");
            npc.state.velocity = crate::geometry::Vec2::ZERO;
            npc.state.heading = f32::from(aim);
        }
        let traffic_was = session.space.npc(traffic).expect("there").state;
        let state = |session: &Session, id: NpcId| session.space.npc(id).expect("there").state;
        let duration = session.ship.warp_sound(&session.statics);
        let factor = session.player_warp_factor();
        let mut moved_off = None;
        for tick in 1..200 {
            assert!(
                led.iter().all(|&id| session.space.npc(id).is_some()),
                "tick {tick}: {:?}",
                session.space.traffic.departed()
            );
            let before: Vec<_> = led.iter().map(|&id| state(&session, id)).collect();
            session.tick(crate::flight::Controls::default());
            ticks(&mut session, &catalog, 1);
            let clock = session.warp_clock().expect("in the warp");
            let v = warp::npc_warp_out_speed(
                warp::WarpOutClock::Player,
                warp::mac_ticks(clock),
                factor,
                duration,
                false,
            );
            for (&id, was) in led.iter().zip(&before) {
                let npc = session.space.npc(id).expect("there");
                assert_eq!(
                    npc.warp.goal,
                    Some(WarpGoal::Follow(ShipRef::Player)),
                    "tick {tick}"
                );
                let push = if v > 0.0 {
                    crate::trig::accel(aim, v)
                } else {
                    crate::geometry::Vec2::ZERO
                };
                assert_eq!(
                    npc.state.position,
                    was.position + push,
                    "tick {tick}: {id:?}"
                );
            }
            if tick == 1 {
                assert_ne!(
                    state(&session, traffic),
                    traffic_was,
                    "the traffic flies on"
                );
            }
            if v > 0.0 {
                moved_off.get_or_insert(tick);
            }
            if moved_off.is_some_and(|first| tick > first + 3) {
                break;
            }
        }
        // 0.45 × d / w², past the player's own 0.35 × d / w².
        let first = moved_off.expect("they move off");
        let at = warp::mac_ticks(u32::try_from(first).expect("small"));
        assert!(
            f64::from(at) > 0.45 * f64::from(duration) / f64::from(factor * factor) - 2.0,
            "{first}"
        );
        assert_eq!(
            session.space.npc(fighter).expect("there").warp.goal,
            None,
            "a fighter an escort leads is not the player's to take along"
        );
    }

    /// `session` with its ships overlapping in NPC order, the player on
    /// top of its layer: [`RuleKey::ShipOverlap`]'s other reading.
    fn in_npc_order(session: Session) -> Session {
        session
            .with_rules(Rulebook::default().with_override(RuleKey::ShipOverlap, RuleSource::Bible))
    }

    #[test]
    fn the_ships_the_player_leads_draw_under_the_others_and_over_the_disabled() {
        let (session, led, traffic) = with_traffic();
        let mut session = in_npc_order(session);
        let order = |first: &[NpcId], then: &[NpcId]| -> Vec<Option<NpcId>> {
            first
                .iter()
                .chain(then)
                .copied()
                .map(Some)
                .chain([None])
                .collect()
        };
        assert_eq!(
            drawn(&session),
            order(&led, &[traffic]),
            "the escorts, then the traffic ship and the player"
        );
        crate::fixture::disable(&mut session, traffic);
        let mut first = vec![traffic];
        first.extend(&led);
        assert_eq!(drawn(&session), order(&first, &[]), "the disabled first");
    }

    #[test]
    fn the_draw_layers_pair_the_draw_order_with_each_ships_layer_by_either_reading() {
        for overlap in RuleSource::ALL {
            let (session, led, traffic) = with_traffic();
            let mut session = session
                .with_rules(Rulebook::default().with_override(RuleKey::ShipOverlap, overlap));
            combat_tick(&mut session);
            crate::fixture::disable(&mut session, traffic);
            // By the engine, only its display step moves it (@0x2bd24).
            combat_tick(&mut session);
            let layers = session.draw_layers();
            let order: Vec<ShipRef> = layers.iter().map(|&(ship, _)| ship).collect();
            assert_eq!(order, session.draw_order(), "{overlap:?}");
            let layer_of = |ship| {
                layers
                    .iter()
                    .find(|&&(drawn, _)| drawn == ship)
                    .map(|&(_, layer)| layer)
            };
            assert_eq!(layer_of(ShipRef::Npc(traffic)), Some(ShipLayer::Disabled));
            assert_eq!(layer_of(ShipRef::Npc(led[0])), Some(ShipLayer::Led));
            assert_eq!(layer_of(ShipRef::Player), Some(ShipLayer::Ships));
        }
    }

    #[test]
    fn a_disabled_player_draws_in_the_bottom_layer_last() {
        let (session, led, traffic) = with_traffic();
        let mut session = in_npc_order(session);
        crate::fixture::disable(&mut session, traffic);
        session.ship.condition = Condition::Disabled;
        let mut expected = vec![Some(traffic), None];
        expected.extend(led.iter().copied().map(Some));
        assert_eq!(drawn(&session), expected);
    }

    /// Advances the fight a tick, with nothing firing: the ships are laid
    /// out in their layers at its end.
    fn combat_tick(session: &mut Session) {
        session.tick_combat(crate::Rules::default(), &mut crate::chance::NeverFires);
    }

    /// [`with_traffic`] with the ships laid out once: the session, the
    /// hired escort, the two mission escorts and the traffic ship.
    fn laid_out() -> (Session, NpcId, [NpcId; 2], NpcId) {
        let (mut session, led, traffic) = with_traffic();
        let hired = session.space.escorts.fleet[0].expect("placed");
        let mission: Vec<NpcId> = led.iter().copied().filter(|&id| id != hired).collect();
        assert_eq!(berth(&session, hired), 1);
        assert_eq!(
            mission
                .iter()
                .map(|&id| berth(&session, id))
                .collect::<Vec<_>>(),
            [2, 3]
        );
        assert_eq!(berth(&session, traffic), 4);
        combat_tick(&mut session);
        (session, hired, [mission[0], mission[1]], traffic)
    }

    /// Makes NPC `id` whole again, its armour and shield full.
    fn repair(session: &mut Session, id: NpcId) {
        let npc = session.space.npc_mut(id).expect("here");
        npc.reserves.armor.now = npc.reserves.armor.max;
        npc.reserves.shield.now = npc.reserves.shield.max;
        npc.condition = Condition::Intact;
    }

    #[test]
    fn a_released_escort_is_drawn_over_the_player() {
        let (mut session, hired, [m0, m1], traffic) = laid_out();
        assert_eq!(
            drawn(&session),
            [Some(hired), Some(m0), Some(m1), Some(traffic), None],
            "the escorts in the order they entered, and the traffic ship under the player"
        );
        session.release(hired);
        combat_tick(&mut session);
        assert_eq!(
            drawn(&session),
            [Some(m0), Some(m1), Some(traffic), None, Some(hired)]
        );
    }

    #[test]
    fn by_the_other_reading_a_released_escort_is_drawn_under_the_player() {
        let (session, hired, [m0, m1], traffic) = laid_out();
        let mut session = in_npc_order(session);
        session.release(hired);
        combat_tick(&mut session);
        assert_eq!(
            drawn(&session),
            [Some(m0), Some(m1), Some(traffic), Some(hired), None],
            "the session's NPC order, the player on top"
        );
    }

    #[test]
    fn a_ship_entering_a_layer_is_drawn_over_the_ships_in_it() {
        let (mut session, hired, [m0, m1], traffic) = laid_out();
        // A mission escort disabled stays led, but is drawn with the
        // disabled; repaired, it enters the escorts' layer on top.
        crate::fixture::disable(&mut session, m0);
        combat_tick(&mut session);
        assert_eq!(drawn(&session)[0], Some(m0));
        repair(&mut session, m0);
        // Only its display step moves its slot's layer (@0x2bd24).
        combat_tick(&mut session);
        assert_eq!(
            drawn(&session),
            [Some(hired), Some(m1), Some(m0), Some(traffic), None]
        );
        // The traffic ship, then the hired escort, disabled: the later
        // over the earlier, whatever their berths.
        crate::fixture::disable(&mut session, traffic);
        combat_tick(&mut session);
        crate::fixture::disable(&mut session, hired);
        combat_tick(&mut session);
        assert_eq!(&drawn(&session)[..2], [Some(traffic), Some(hired)]);
    }

    #[test]
    fn ships_entering_a_layer_in_one_tick_stack_in_berth_order() {
        let (mut session, hired, _, traffic) = laid_out();
        assert_eq!(session.npcs()[0].id, traffic, "first in NPC order");
        crate::fixture::disable(&mut session, traffic);
        crate::fixture::disable(&mut session, hired);
        combat_tick(&mut session);
        assert_eq!(&drawn(&session)[..2], [Some(hired), Some(traffic)]);
    }

    #[test]
    fn a_ship_that_was_disabled_and_is_whole_again_is_drawn_over_the_player() {
        let (mut session, _, _, traffic) = laid_out();
        crate::fixture::disable(&mut session, traffic);
        combat_tick(&mut session);
        repair(&mut session, traffic);
        // Only its display step moves its slot's layer (@0x2bd24).
        combat_tick(&mut session);
        let order = drawn(&session);
        assert_eq!(&order[order.len() - 2..], [None, Some(traffic)]);
    }

    #[test]
    fn a_player_disabled_and_whole_again_goes_back_on_top_of_its_layer() {
        let (mut session, hired, _, traffic) = laid_out();
        session.release(hired);
        combat_tick(&mut session);
        session.pilot.reserves.shield.now = 0.0;
        session.pilot.reserves.armor.now = 1.0;
        session.ship.condition = Condition::Disabled;
        combat_tick(&mut session);
        assert_eq!(drawn(&session)[0], None);
        session.pilot.reserves.armor.now = session.pilot.reserves.armor.max;
        session.ship.condition = Condition::Intact;
        // Only its display step moves its slot's layer (@0x2bd24).
        combat_tick(&mut session);
        let order = drawn(&session);
        assert_eq!(
            &order[order.len() - 3..],
            [Some(traffic), Some(hired), None]
        );
    }

    #[test]
    fn a_new_ship_at_a_berth_takes_the_place_its_last_ship_left() {
        let (mut session, hired, [m0, m1], traffic) = laid_out();
        session.release(hired);
        combat_tick(&mut session);
        let class = session.space.npc(traffic).expect("there").clone();
        // Berth 1's ship last stood over the player: the next ship there
        // takes that place.
        session.space.traffic.remove(hired);
        let first = session.space.traffic.add_npc(class.clone());
        assert_eq!(berth(&session, first), 1);
        // Berth 2's ship last stood in the escorts' layer: the next ship
        // there enters the top layer on top.
        session.space.traffic.remove(m0);
        let second = session.space.traffic.add_npc(class);
        assert_eq!(berth(&session, second), 2);
        // Each takes its place at its first display step (@0x2bd24).
        combat_tick(&mut session);
        assert_eq!(
            drawn(&session),
            [Some(m1), Some(traffic), None, Some(first), Some(second)]
        );
    }

    #[test]
    fn the_layers_are_kept_across_a_jump() {
        let catalog = ships_catalog(ships(1));
        let mut session = accepted(&catalog);
        session.pilot.escorts.push(hired());
        take_off(&mut session, &catalog);
        combat_tick(&mut session);
        let hired = session.space.escorts.fleet[0].expect("placed");
        session.release(hired);
        combat_tick(&mut session);
        let class = session.space.npc(hired).expect("there").clone();
        jump_to(&mut session, &catalog, 131);
        let stranger = session.space.traffic.add_npc_at(
            Npc {
                escort: None,
                ..class
            },
            1,
        );
        // A ship new at a berth is shown at its slot's first display
        // (@0x2b555-0x2b60a, @0xacba3), where it keeps the slot's place.
        assert!(!drawn(&session).contains(&Some(stranger)));
        combat_tick(&mut session);
        let order = drawn(&session);
        assert_eq!(
            order.last(),
            Some(&Some(stranger)),
            "berth 1 still over the player"
        );
    }

    #[test]
    fn a_slots_layer_place_is_kept_across_a_pilot_loaded_in_the_same_run() {
        // Pilot A flies with a hired escort at berth 1, laid out in the
        // escorts' layer.
        let (mut a, _, hired) = flying(1);
        assert_eq!(berth(&a, hired), 1);
        combat_tick(&mut a);
        assert_eq!(
            a.slot_sprites().layers.rank(Slot::Berth(1)).0,
            ShipLayer::Led
        );

        // Pilot B, loaded in the same run, flies with a mission ship that
        // never joins at berth 1: it leaves the escorts' layer and enters
        // the top one over the player.
        let b = |sprites: Option<SlotSprites>| {
            let catalog = ships_catalog(ships(0));
            let mut session = accepted(&catalog);
            if let Some(sprites) = sprites {
                session = session.with_slot_sprites(sprites);
            }
            take_off(&mut session, &catalog);
            let first = session
                .npcs()
                .iter()
                .find(|npc| npc.berth == 1)
                .expect("a ship at berth 1")
                .id;
            assert!(!session.space.npc(first).expect("there").escorts_player());
            combat_tick(&mut session);
            (session, first)
        };
        let (carried, first) = b(Some(a.slot_sprites().clone()));
        let order = drawn(&carried);
        let place = |ship| order.iter().position(|&drawn| drawn == ship);
        assert!(place(Some(first)) > place(None), "{order:?}");
        assert_eq!(
            carried.slot_sprites().layers.rank(Slot::Berth(1)).0,
            ShipLayer::Ships
        );
        assert!(
            carried.slot_sprites().layers.rank(Slot::Berth(1))
                > carried.slot_sprites().layers.rank(Slot::Player)
        );

        let (fresh, _) = b(None);
        assert_eq!(drawn(&fresh).last(), Some(&None), "the player on top");
    }

    #[test]
    fn a_session_starts_from_the_launch_layers_and_takes_the_ones_it_is_given() {
        let catalog = ships_catalog(ships(1));
        let session = accepted(&catalog);
        assert_eq!(session.slot_sprites().layers, SlotSprites::default().layers);
        assert_eq!(session.slot_sprites().shots, SlotSprites::default().shots);
        // Game entry displays the player's slot, which clones its frames
        // (`_EnterGameFromMainScreen` @0x1a36c, @0x2b824).
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Player),
            session.pilot.ship
        );
        let mut sprites = SlotSprites::default();
        sprites.layers.put(Slot::Berth(3), ShipLayer::Disabled);
        let session = session.with_slot_sprites(sprites.clone());
        assert_eq!(session.slot_sprites(), &sprites);
    }

    /// `session` with hidden ships shown: [`RuleKey::HiddenShips`]'s other
    /// reading.
    fn showing_hidden(session: Session) -> Session {
        session
            .with_rules(Rulebook::default().with_override(RuleKey::HiddenShips, RuleSource::Bible))
    }

    #[test]
    fn a_ship_of_a_cost_minus_one_type_is_left_out_of_the_draw_order_and_the_layer_history() {
        let (mut session, _, hired) = flying(1);
        let berth = berth(&session, hired);
        // A place its own display would never give it: the take-off
        // displayed it in the escorts' layer (`_PrepareAllSprites`
        // @0x3a75a).
        session
            .space
            .slot_sprites
            .layers
            .put(Slot::Berth(berth), ShipLayer::Disabled);
        let had = session.slot_sprites().layers.rank(Slot::Berth(berth));
        crate::fixture::hide(&mut session, hired);
        assert!(session.is_hidden(ShipRef::Npc(hired)));
        combat_tick(&mut session);
        combat_tick(&mut session);
        assert!(!drawn(&session).contains(&Some(hired)));
        assert_eq!(
            session.slot_sprites().layers.rank(Slot::Berth(berth)),
            had,
            "its slot keeps the place it had"
        );

        let session = in_npc_order(session);
        assert!(!drawn(&session).contains(&Some(hired)), "by either overlap");

        let mut session = showing_hidden(session);
        assert!(!session.is_hidden(ShipRef::Npc(hired)));
        // Its slot's last display hid it: it is shown at its next.
        assert!(!drawn(&session).contains(&Some(hired)));
        combat_tick(&mut session);
        assert!(drawn(&session).contains(&Some(hired)));
        assert_eq!(
            session.slot_sprites().layers.rank(Slot::Berth(berth)).0,
            ShipLayer::Led
        );
    }

    #[test]
    fn a_player_flying_a_cost_minus_one_type_is_hidden_and_keeps_its_place() {
        let (mut session, _, _) = flying(1);
        session.ship.fit.hull.hidden = true;
        assert!(session.is_hidden(ShipRef::Player));
        session.ship.condition = Condition::Disabled;
        combat_tick(&mut session);
        assert!(!drawn(&session).contains(&None));
        assert_eq!(
            session.slot_sprites().layers.rank(Slot::Player),
            (ShipLayer::Ships, 0),
            "not moved to the disabled layer"
        );
        let session = in_npc_order(session);
        assert!(!drawn(&session).contains(&None));
        // Its slot's last display hid it: shown again, it waits for its
        // next display, by either overlap.
        for overlap in RuleSource::ALL {
            let mut shown = session.clone().with_rules(
                Rulebook::default()
                    .with_override(RuleKey::HiddenShips, RuleSource::Bible)
                    .with_override(RuleKey::ShipOverlap, overlap),
            );
            assert!(!drawn(&shown).contains(&None), "{overlap:?}");
            combat_tick(&mut shown);
            assert!(drawn(&shown).contains(&None), "{overlap:?}");
        }
    }

    /// A hidden ship of type `ship` put at `berth` in place of whatever
    /// stood there, as a new ship takes a freed berth: its NPC.
    fn hidden_at(session: &mut Session, berth: u32, ship: ShipId) -> NpcId {
        let class = session.npcs()[0].clone();
        if let Some(there) = session.npcs().iter().find(|npc| npc.berth == berth) {
            let there = there.id;
            session.space.traffic.remove(there);
        }
        let mut hull = class.hull;
        hull.hidden = true;
        session.space.traffic.add_npc_at(
            Npc {
                ship,
                hull,
                escort: None,
                mission: None,
                ..class
            },
            berth,
        )
    }

    /// A hidden ship's type, not one any fixture ship flies.
    const HIDDEN: ShipId = ShipId(300);

    #[test]
    fn a_hidden_ship_is_framed_by_its_berths_last_shown_type() {
        let (mut session, mission, hired) = flying(1);
        let berth = berth(&session, hired);
        assert_eq!(session.framed_by(ShipRef::Npc(hired)), Some(ShipId(200)));
        combat_tick(&mut session);
        let hidden = hidden_at(&mut session, berth, HIDDEN);
        assert!(session.is_hidden(ShipRef::Npc(hidden)));
        assert_eq!(session.framed_by(ShipRef::Npc(hidden)), Some(ShipId(200)));
        let shown = ShipId(210);
        session.space.npc_mut(mission[0]).expect("there").ship = shown;
        assert_eq!(session.framed_by(ShipRef::Npc(mission[0])), Some(shown));
        assert_eq!(session.framed_by(ShipRef::Player), Some(session.ship()));
        assert_eq!(session.framed_by(ShipRef::Npc(hired)), None, "gone");
        assert_eq!(
            session
                .framed_types()
                .collect::<std::collections::BTreeSet<_>>(),
            [ShipId(200), shown].into_iter().collect(),
            "not the player's own type"
        );
    }

    #[test]
    fn a_hidden_ship_in_an_unused_berth_is_framed_by_shïp_128() {
        let (mut session, _, _) = flying(1);
        combat_tick(&mut session);
        let hidden = hidden_at(&mut session, 40, HIDDEN);
        combat_tick(&mut session);
        assert_eq!(
            session.framed_by(ShipRef::Npc(hidden)),
            Some(crate::ship_layer::INIT_FRAMES)
        );
        assert!(session.framed_types().any(|ship| ship == ShipId(128)));
        assert!(!session.framed_types().any(|ship| ship == HIDDEN));
    }

    #[test]
    fn a_hidden_ship_does_not_reclone_its_berth() {
        let (mut session, _, hired) = flying(1);
        let berth = berth(&session, hired);
        combat_tick(&mut session);
        hidden_at(&mut session, berth, HIDDEN);
        combat_tick(&mut session);
        combat_tick(&mut session);
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Berth(berth)),
            ShipId(200)
        );
    }

    #[test]
    fn a_hidden_player_is_framed_by_its_last_shown_ship() {
        let (mut session, _, _) = flying(1);
        let flown = session.ship();
        combat_tick(&mut session);
        assert_eq!(session.slot_sprites().frames.of(Slot::Player), flown);
        session.pilot.ship = HIDDEN;
        session.ship.fit.hull.hidden = true;
        combat_tick(&mut session);
        assert_eq!(session.framed_by(ShipRef::Player), Some(flown));
        assert_eq!(session.slot_sprites().frames.of(Slot::Player), flown);
        assert_eq!(
            session.framed_types().next(),
            Some(flown),
            "the player's slot first"
        );
    }

    #[test]
    fn the_other_hover_box_reading_frames_a_hidden_ship_by_its_own_type() {
        let (mut session, _, hired) = flying(1);
        let berth = berth(&session, hired);
        combat_tick(&mut session);
        let hidden = hidden_at(&mut session, berth, HIDDEN);
        let mut session = session.with_rules(
            Rulebook::default().with_override(RuleKey::HiddenShipHoverBox, RuleSource::Bible),
        );
        assert!(session.is_hidden(ShipRef::Npc(hidden)));
        // Once its slot's display has laid it out (by
        // `RuleKey::UndisplayedShips`' engine reading, a ship not yet laid
        // out is framed by its slot's frames whatever its hiding).
        combat_tick(&mut session);
        assert_eq!(session.framed_by(ShipRef::Npc(hidden)), Some(HIDDEN));
        assert!(session.framed_types().any(|ship| ship == HIDDEN));
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Berth(berth)),
            ShipId(200),
            "still not recloned"
        );

        let mut session = showing_hidden(session);
        assert!(!session.is_hidden(ShipRef::Npc(hidden)));
        assert_eq!(session.framed_by(ShipRef::Npc(hidden)), Some(HIDDEN));
        combat_tick(&mut session);
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Berth(berth)),
            HIDDEN,
            "shown, it clones its own"
        );
    }

    #[test]
    fn slot_frames_carry_into_a_later_session() {
        let (mut a, _, hired) = flying(1);
        let berth = berth(&a, hired);
        combat_tick(&mut a);
        let catalog = ships_catalog(ships(0));
        let b = accepted(&catalog).with_slot_sprites(a.slot_sprites().clone());
        assert_eq!(b.slot_sprites().frames.of(Slot::Berth(berth)), ShipId(200));
        assert_eq!(
            accepted(&catalog)
                .slot_sprites()
                .frames
                .of(Slot::Berth(berth)),
            ShipId(128)
        );
    }
}
