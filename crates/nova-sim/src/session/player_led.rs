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
//! and a led ship times its warp-out by the player's clock
//! (@0x347c1-0x348bd), which holds here by construction: the player, the
//! traffic and the fight all stand still through the hyperspace stage, so
//! every led ship keeps its place beside the player and arrives with it
//! (see [`escorts`](super::escorts)). No ship has a warp-out motion, nor a
//! disabled ship its drag (@0x34752-0x349c1, @0x3395d-0x339a1): rdm:phase/player-led-ships/phase-26-warp-out-motion-and-disabled-drag.
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

use super::Session;
use crate::combat::ShipRef;
use crate::combat::hull::HullSpec;
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_layer::{LayerEntries, ShipLayer, Slot, disabled, in_draw_order};
use crate::traffic::npc::Npc;

impl Session {
    /// The ships the player leads, in ship-table order (see the module
    /// docs).
    pub fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        self.traffic.player_led()
    }

    /// The ships in the order they are drawn, bottom first: the disabled,
    /// then the ships the player leads, then the rest (see
    /// [`ship_layer`](crate::ship_layer)). Within each layer, by
    /// [`RuleKey::ShipOverlap`]: by the engine, in the order each ship's
    /// slot last entered it, with the ships laid out as they stand now,
    /// as each flight tick lays them out at its end
    /// ([`Session::tick_combat`]); by the other reading, in the
    /// session's NPC order with the player on top. A hidden ship
    /// ([`Session::is_hidden`]), the player too, is left out by either.
    #[must_use]
    pub fn draw_order(&self) -> Vec<ShipRef> {
        match self.rules.source_for(RuleKey::ShipOverlap) {
            RuleSource::Engine => {
                let mut layers = self.layers.clone();
                let ships = self.ship_layers();
                for &(_, slot, layer) in &ships {
                    layers.put(slot, layer);
                }
                layers.in_entry_order(ships.into_iter().map(|(ship, slot, _)| (ship, slot)))
            }
            RuleSource::Bible => {
                let npcs = self
                    .npcs()
                    .iter()
                    .filter(|npc| self.shown(npc.hull))
                    .map(|npc| {
                        let layer = ShipLayer::of(disabled(npc.condition), npc.escorts_player());
                        (ShipRef::Npc(npc.id), layer)
                    });
                let player = self
                    .shown(self.hull)
                    .then(|| (ShipRef::Player, self.player_layer()));
                in_draw_order(npcs.chain(player))
            }
        }
    }

    /// Whether `ship` is hidden: its type's `Cost` is -1
    /// ([`HullSpec::hidden`]) and [`RuleKey::HiddenShips`] reads the
    /// engine. `_HandleShipDisplay` hides such a ship, the player's too,
    /// as it stands in the player's system (@0x2b7d8-0x2b7f3), and skips
    /// its layer block (@0x2cc6b), so its slot keeps its layer place. The
    /// original's hidden sprite is still positioned (@0x2cd17), so the
    /// pointer still hovers it (see [`hover`](crate::hover)). A ship that
    /// is not here is not hidden.
    #[must_use]
    pub fn is_hidden(&self, ship: ShipRef) -> bool {
        let hull = match ship {
            ShipRef::Player => Some(self.hull),
            ShipRef::Npc(id) => self.npc(id).map(|npc| npc.hull),
        };
        hull.is_some_and(|hull| !self.shown(hull))
    }

    /// Whether a ship of `hull` is drawn and laid out (see
    /// [`Session::is_hidden`]).
    fn shown(&self, hull: HullSpec) -> bool {
        !hull.hidden || self.rules.source_for(RuleKey::HiddenShips) == RuleSource::Bible
    }

    /// Each ship slot's draw layer and when it entered it, as the last
    /// flight tick laid them out ([`Session::draw_order`]): the history to
    /// hand a session loaded later in the same run
    /// ([`Session::with_layer_entries`]).
    #[must_use]
    pub fn layer_entries(&self) -> &LayerEntries {
        &self.layers
    }

    /// The session, its ship slots' layers starting from `layers`, the
    /// history a session flown earlier in the same run left
    /// ([`Session::layer_entries`]), in place of `_InitSprites`' order.
    /// The original's sprite layers outlive a pilot: `_InitSprites` runs
    /// once, from `_InitEVNova` (@0x47382), and nothing between pilots
    /// re-lays or rebuilds a slot's sprites (see
    /// [`ship_layer`](crate::ship_layer)).
    #[must_use]
    pub fn with_layer_entries(mut self, layers: LayerEntries) -> Self {
        self.layers = layers;
        self
    }

    /// Puts the player, then each NPC in berth order, in the layer it
    /// stands in, all but the hidden ones ([`Session::is_hidden`]), as `_DoPlayGameWork` runs `_HandleShipDisplay` for slot
    /// 0 (@0x43e29) and then for each slot in turn (@0x444ad), and
    /// `_PrepareAllSprites` for all 64 (@0x3a75a-0x3a78e): see
    /// [`LayerEntries::put`](crate::ship_layer::LayerEntries::put). Run at the end of each flight tick
    /// ([`Session::tick_combat`]); landed or in hyperspace nothing is laid
    /// out, and [`Session::draw_order`] lays the ships out as they stand
    /// whenever it is asked, so a ship that changed layer since is drawn
    /// as the next tick will keep it.
    ///
    /// The original lays each ship out right after its own `_HandleShip`
    /// (@0x44483); here every ship is laid out after the whole tick. The
    /// two differ only when one ship's handling changes the layer of a
    /// ship in a lower slot in the same tick: here that ship enters its
    /// new layer a tick early beside its neighbours. The tick ends before
    /// a frame is drawn, so the player never sees the difference.
    pub(super) fn lay_out_ships(&mut self) {
        for (_, slot, layer) in self.ship_layers() {
            self.layers.put(slot, layer);
        }
    }

    /// The player, then each NPC in berth order (ships at one berth in
    /// NPC order), with its slot and the layer it stands in: every ship
    /// but the hidden ones ([`Session::is_hidden`]), whose slots are
    /// neither laid out nor drawn.
    fn ship_layers(&self) -> Vec<(ShipRef, Slot, ShipLayer)> {
        let mut npcs: Vec<&Npc> = self
            .npcs()
            .iter()
            .filter(|npc| self.shown(npc.hull))
            .collect();
        npcs.sort_by_key(|npc| npc.berth);
        let npcs = npcs.into_iter().map(|npc| {
            let layer = ShipLayer::of(disabled(npc.condition), npc.escorts_player());
            (ShipRef::Npc(npc.id), Slot::Berth(npc.berth), layer)
        });
        let player = self
            .shown(self.hull)
            .then(|| (ShipRef::Player, Slot::Player, self.player_layer()));
        player.into_iter().chain(npcs).collect()
    }

    /// The layer the player stands in: never led.
    fn player_layer(&self) -> ShipLayer {
        ShipLayer::of(disabled(self.player_condition()), false)
    }
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
        let hired = session.fleet[0].expect("placed");
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
        for npc in session.traffic.npcs_mut() {
            npc.state.position = behind;
            npc.state.velocity = crate::geometry::Vec2::ZERO;
        }
        let thrust = crate::flight::Controls {
            thrust: true,
            turn: crate::flight::Turn::None,
            reverse: false,
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
            let hired = session.fleet[0].expect("placed");
            let class = session.npc(hired).expect("there").clone();
            let unled = session.traffic.add_npc(Npc {
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
                let top = session.npc(id).expect("there").stats.handling.max_speed;
                assert!(
                    (npcs[&id] - top * factor).abs() < 0.01,
                    "{strict_play}: {id:?} at {}",
                    npcs[&id]
                );
            }
            let top = session.npc(unled).expect("there").stats.handling.max_speed;
            assert!(
                npcs[&unled] <= top + 1e-4 && npcs[&unled] > top - 0.01,
                "{strict_play}: the unled ship, flat out at {}",
                npcs[&unled]
            );
        }
    }

    /// NPC `id`'s formation slot, if it has one.
    fn slot(session: &Session, id: NpcId) -> Option<u8> {
        session
            .npc(id)
            .and_then(Npc::formation)
            .map(|place| place.slot)
    }

    /// NPC `id`'s berth.
    fn berth(session: &Session, id: NpcId) -> u32 {
        session.npc(id).expect("in the system").berth
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
            session.fleet[0].expect("placed"),
            session.fleet[1].expect("placed"),
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
        let b = session.fleet[0].expect("placed");
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
            .map(|&id| session.npc(id).expect("here").hull.sprite_size)
            .collect();
        let spacing = crate::escort::spacing(session.hull.sprite_size, &widths);
        for id in [m[0], b, m[1]] {
            let place = session.npc(id).and_then(Npc::formation).expect("a slot");
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
        let placed: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
        let [a, b, c] = [placed[0], placed[1], placed[2]];
        assert_eq!([a, b, c].map(|id| berth(&session, id)), [1, 2, 3]);

        // B is lost, and D, an escort with no ship yet (as a loaded
        // pilot's), takes the gap B left, below the kept escorts' berths,
        // never one of theirs.
        session.dismiss(b);
        session.pilot.escorts.push(hired());
        jump_to(&mut session, &catalog, 131);
        let fleet: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
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
            session.traffic.add_npc(ship()),
            session.traffic.add_npc(ship()),
        ];
        assert_eq!(crowd.map(|id| berth(&session, id)), [1, 2]);
        session
            .hire(ShipId(200), &mut crate::chance::NeverFires)
            .expect("hired");
        // `_SpawnEscort` puts it in the table at once, at the first free
        // slot among the landed system's ships (@0x3fe8f).
        let hired = session.fleet[0].expect("placed while landed");
        assert_eq!(berth(&session, hired), 3);
        take_off(&mut session, &catalog);
        let hired = session.fleet[0].expect("placed again");
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
        assert!(session.npc(hired).is_some(), "still here as it leaves");
    }

    #[test]
    fn a_joined_mission_escort_disabled_stays_led() {
        let catalog = ships_catalog(ships(1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let mission = led(&session);
        assert_eq!(mission.len(), 2);
        session.traffic.npcs_mut()[0].condition = Condition::Disabled;
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
        let class = session.npc(hired).expect("there").clone();
        let traffic = session.traffic.add_npc(Npc {
            escort: None,
            ..class
        });
        session.traffic.npcs_mut().rotate_right(1);
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
    fn a_disabled_player_draws_in_the_bottom_layer_last() {
        let (session, led, traffic) = with_traffic();
        let mut session = in_npc_order(session);
        crate::fixture::disable(&mut session, traffic);
        session.condition = Condition::Disabled;
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
        let hired = session.fleet[0].expect("placed");
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
        let npc = session.npc_mut(id).expect("here");
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
        session.condition = Condition::Disabled;
        combat_tick(&mut session);
        assert_eq!(drawn(&session)[0], None);
        session.pilot.reserves.armor.now = session.pilot.reserves.armor.max;
        session.condition = Condition::Intact;
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
        let class = session.npc(traffic).expect("there").clone();
        // Berth 1's ship last stood over the player: the next ship there
        // takes that place.
        session.traffic.remove(hired);
        let first = session.traffic.add_npc(class.clone());
        assert_eq!(berth(&session, first), 1);
        // Berth 2's ship last stood in the escorts' layer: the next ship
        // there enters the top layer on top.
        session.traffic.remove(m0);
        let second = session.traffic.add_npc(class);
        assert_eq!(berth(&session, second), 2);
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
        let hired = session.fleet[0].expect("placed");
        session.release(hired);
        combat_tick(&mut session);
        let class = session.npc(hired).expect("there").clone();
        jump_to(&mut session, &catalog, 131);
        let stranger = session.traffic.add_npc_at(
            Npc {
                escort: None,
                ..class
            },
            1,
        );
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
        assert_eq!(a.layer_entries().rank(Slot::Berth(1)).0, ShipLayer::Led);

        // Pilot B, loaded in the same run, flies with a mission ship that
        // never joins at berth 1: it leaves the escorts' layer and enters
        // the top one over the player.
        let b = |layers: Option<crate::ship_layer::LayerEntries>| {
            let catalog = ships_catalog(ships(0));
            let mut session = accepted(&catalog);
            if let Some(layers) = layers {
                session = session.with_layer_entries(layers);
            }
            take_off(&mut session, &catalog);
            let first = session
                .npcs()
                .iter()
                .find(|npc| npc.berth == 1)
                .expect("a ship at berth 1")
                .id;
            assert!(!session.npc(first).expect("there").escorts_player());
            combat_tick(&mut session);
            (session, first)
        };
        let (carried, first) = b(Some(a.layer_entries().clone()));
        let order = drawn(&carried);
        let place = |ship| order.iter().position(|&drawn| drawn == ship);
        assert!(place(Some(first)) > place(None), "{order:?}");
        assert_eq!(
            carried.layer_entries().rank(Slot::Berth(1)).0,
            ShipLayer::Ships
        );
        assert!(
            carried.layer_entries().rank(Slot::Berth(1))
                > carried.layer_entries().rank(Slot::Player)
        );

        let (fresh, _) = b(None);
        assert_eq!(drawn(&fresh).last(), Some(&None), "the player on top");
    }

    #[test]
    fn a_session_starts_from_the_launch_layers_and_takes_the_ones_it_is_given() {
        let catalog = ships_catalog(ships(1));
        let session = accepted(&catalog);
        assert_eq!(
            session.layer_entries(),
            &crate::ship_layer::LayerEntries::new()
        );
        let mut layers = crate::ship_layer::LayerEntries::new();
        layers.put(Slot::Berth(3), ShipLayer::Disabled);
        let session = session.with_layer_entries(layers.clone());
        assert_eq!(session.layer_entries(), &layers);
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
        crate::fixture::hide(&mut session, hired);
        assert!(session.is_hidden(ShipRef::Npc(hired)));
        combat_tick(&mut session);
        combat_tick(&mut session);
        assert!(!drawn(&session).contains(&Some(hired)));
        assert_eq!(
            session.layer_entries().rank(Slot::Berth(berth)),
            (ShipLayer::Ships, -1 - i64::from(berth)),
            "its slot keeps the place it had"
        );

        let session = in_npc_order(session);
        assert!(!drawn(&session).contains(&Some(hired)), "by either overlap");

        let mut session = showing_hidden(session);
        assert!(!session.is_hidden(ShipRef::Npc(hired)));
        assert!(drawn(&session).contains(&Some(hired)));
        combat_tick(&mut session);
        assert!(drawn(&session).contains(&Some(hired)));
        assert_eq!(
            session.layer_entries().rank(Slot::Berth(berth)).0,
            ShipLayer::Led
        );
    }

    #[test]
    fn a_player_flying_a_cost_minus_one_type_is_hidden_and_keeps_its_place() {
        let (mut session, _, _) = flying(1);
        session.hull.hidden = true;
        assert!(session.is_hidden(ShipRef::Player));
        session.condition = Condition::Disabled;
        combat_tick(&mut session);
        assert!(!drawn(&session).contains(&None));
        assert_eq!(
            session.layer_entries().rank(Slot::Player),
            (ShipLayer::Ships, 0),
            "not moved to the disabled layer"
        );
        let session = in_npc_order(session);
        assert!(!drawn(&session).contains(&None));
    }
}
