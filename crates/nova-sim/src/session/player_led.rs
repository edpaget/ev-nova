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
//! branches for led ships (its disabled branch @0x33950-0x33a36, its
//! jump-out timing @0x347c1-0x348c2 and its self-repair rejoin
//! @0x33a3b-0x33bd9) are not modelled; the step-1 findings of
//! rdm:phase/player-led-ships/phase-10-escort-death-destroys-cargo-share
//! say why.
//!
//! **Drawn.** `_HandleShipDisplay` treats a ship the player leads apart
//! in three branches. It draws it in the escorts' layer, under the other
//! ships and the player and over the disabled ones (@0x2bd24-0x2bd47,
//! [`Session::draw_order`]; see [`ship_layer`](crate::ship_layer)). At a
//! full cloak it draws it faint where a stranger is not drawn at all
//! (@0x2d187-0x2d1cd, [`Session::cloak_shown`]). Its third branch, a
//! trader escort's share of the fleet's cargo destroyed as its final
//! explosion goes off (@0x2dbf5-0x2dc46), cannot run: a led ship's leader
//! is cleared as it is disabled or starts to break up. None of them plays
//! a sound.

use super::Session;
use crate::combat::ShipRef;
use crate::ship_layer::{ShipLayer, disabled, in_draw_order};
use crate::traffic::npc::Npc;

impl Session {
    /// The ships the player leads, in ship-table order (see the module
    /// docs).
    pub fn player_led(&self) -> impl Iterator<Item = &Npc> + '_ {
        self.traffic.player_led()
    }

    /// The ships in the order they are drawn, bottom first: the disabled,
    /// then the ships the player leads, then the rest with the player on
    /// top (see [`ship_layer`](crate::ship_layer)).
    #[must_use]
    pub fn draw_order(&self) -> Vec<ShipRef> {
        let npcs = self.npcs().iter().map(|npc| {
            let layer = ShipLayer::of(disabled(npc.condition), npc.escorts_player());
            (ShipRef::Npc(npc.id), layer)
        });
        // The player last, on top of its tier, as `_InitSprites`'s slot
        // order leaves it.
        let player = (
            ShipRef::Player,
            ShipLayer::of(disabled(self.player_condition()), false),
        );
        in_draw_order(npcs.chain([player]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{MissionShip, ShipId};
    use crate::combat::hull::Condition;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
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

    #[test]
    fn the_ships_the_player_leads_draw_under_the_others_and_over_the_disabled() {
        let (mut session, led, traffic) = with_traffic();
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
        let (mut session, led, traffic) = with_traffic();
        crate::fixture::disable(&mut session, traffic);
        session.condition = Condition::Disabled;
        let mut expected = vec![Some(traffic), None];
        expected.extend(led.iter().copied().map(Some));
        assert_eq!(drawn(&session), expected);
    }
}
