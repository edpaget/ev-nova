//! The session's side of [`cargo_box`](crate::cargo_box): the boxes a
//! jettison drops behind the player and its trader escorts
//! ([`Session::jettison_cargo`]), scooped up ([`scoop`](crate::scoop)),
//! stepped each flight tick, and cleared on every system change and
//! landing. They are never saved.

use super::{PlayerShip, Session, Space, Statics};
use crate::cargo_box::{CargoBox, escort_share};
use crate::chance::Chance;
use crate::market::Good;
use crate::pilot::Pilot;
use crate::scoop::{Haul, Scooper, has_asteroid_scoop, haul, npc_in_ship_layer, npc_scoop};
use crate::ship_layer::disabled;

impl Session {
    /// The cargo boxes in space (see [`cargo_box`](crate::cargo_box)).
    #[must_use]
    pub fn cargo_boxes(&self) -> &[CargoBox] {
        self.space.cargo_boxes()
    }

    /// Drops the boxes of a jettison of `total` tons, `goods` of them
    /// goods, drawing on `chance` (`_JettisonCargo` @0x3e1c2-0x3e2e7): the
    /// player's share is the whole total; each escort the fleet's holds
    /// count ([`Session::capacity`],
    /// [`EscortHolds::adds_holds`](crate::market::EscortHolds::adds_holds))
    /// whose NPC is in the system drops the goods times its `Holds` over
    /// the fleet's ([`escort_share`]), behind its NPC.
    pub(super) fn drop_cargo_boxes(
        &mut self,
        total: i64,
        goods: i64,
        chance: &mut (impl Chance + ?Sized),
    ) {
        // Precision lost only past 2^53 tons.
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        let player = self.ship.player;
        self.space
            .boxes
            .drop_behind(&player, self.ship.fit.hull.sprite_size, total, chance);
        let fleet_holds = self.capacity();
        let droppers: Vec<_> = self
            .pilot
            .escorts
            .iter()
            .zip(&self.space.escorts.fleet)
            .filter_map(|(escort, &placed)| {
                let holds = self
                    .statics
                    .holds_of(escort)
                    .filter(|holds| holds.adds_holds())?;
                let npc = self.space.npc(placed?)?;
                Some((
                    npc.state,
                    npc.hull.sprite_size,
                    escort_share(goods, holds.holds, fleet_holds),
                ))
            })
            .collect();
        for (state, width, share) in droppers {
            self.space.boxes.drop_behind(&state, width, share, chance);
        }
    }

    /// The player's scoop flag (ship +0xc3, see
    /// [`scoop::player_scoops`](crate::scoop::player_scoops)): it owns an
    /// asteroid scoop and the fleet's holds ([`Session::capacity`]) are
    /// past the cargo it holds ([`Pilot::total_cargo`](crate::pilot::Pilot::total_cargo)).
    ///
    /// Read live. The original keeps the flag from the last
    /// `_ResetPlayerPrecalcedValues` (14 callers, the scoop itself among
    /// them), and the only time that shows is a jünk step in flight
    /// (`market::step_junk` in `before_move`), which can fill or free the
    /// hold with no reset: the original then still scoops one box into a
    /// hold the step just filled, or passes over one it just freed room
    /// for, until the next reset. The engine reads the hold as it is.
    #[must_use]
    pub fn player_scoops(&self) -> bool {
        self.ship.player_scoops(&self.pilot, &self.statics)
    }

    /// Scoops up the asteroid boxes the ships touch, as `_DoPlayGameWork`
    /// collides `_boxLayer` with the ship layer (@0x43e46-0x43e70) before
    /// `_HandleBoxes` (@0x444f0), on both passes (see
    /// [`scoop`](crate::scoop)):
    ///
    /// 1. Nothing unless a box is scoopable, `_miningBoxesActive`. The
    ///    original's flag reflects the previous frame's `_HandleBoxes`
    ///    (@0x2df8e-0x2dfa1, @0x2e26c-0x2e283); read live, a box this
    ///    frame's asteroid break drops is scoopable a pass sooner, and the
    ///    boxes that expired last step are gone. No frame is drawn in
    ///    between, so the player never sees the difference.
    /// 2. The player, unless disabled or breaking up (the player is never
    ///    led, so that alone is its layer test), takes each box its disc
    ///    touches while its scoop flag holds ([`Session::player_scoops`]),
    ///    testing the flag again before each, as the original recomputes
    ///    it after each scoop: a hold with a ton free takes one box. Each
    ///    box adds its [`haul`] and resets the precalced values
    ///    (@0x371ac), an empty one's included.
    /// 3. Then each NPC, in NPC order, that is scooping
    ///    ([`Npc::is_scooping`](crate::traffic::npc::Npc::is_scooping))
    ///    and in the ship layer ([`npc_in_ship_layer`]) takes every box
    ///    its disc touches ([`npc_scoop`]).
    ///
    /// *Ordering against the original.* The positions tested are those at
    /// this point of the pass, before this pass's player move and after
    /// the last traffic tick; a pass's difference in when an overlap is
    /// caught crosses no boundary the player sees. The player tests before
    /// the NPCs and each box is taken once, where the original's order
    /// follows `SpriteWorld`'s layer history. There is no sound and no
    /// message, and the HUD's cargo area is drawn afresh each frame, so
    /// the status redraw flag (@0x37199) needs no counterpart.
    pub(super) fn scoop_boxes(&mut self) {
        if !self.space.boxes.any_scoopable() {
            return;
        }
        if !disabled(self.ship.condition) {
            while self.player_scoops() {
                let Some(taken) = self.space.boxes.take_touching(
                    self.ship.player.position,
                    self.ship.fit.hull.hit_radius,
                    &self.statics.box_frames,
                ) else {
                    break;
                };
                let good = match haul(taken.contents, Scooper::Player) {
                    Haul::Commodity(commodity) => Some(Good::Commodity(commodity)),
                    Haul::Junk(junk) => Some(Good::Junk(junk)),
                    Haul::Nothing => None,
                };
                if let Some(good) = good {
                    let tons = self.pilot.cargo.entry(good).or_insert(0);
                    *tons = tons.saturating_add(1);
                }
                self.reset_precalced();
            }
        }
        let (boxes, frames) = (&mut self.space.boxes, &self.statics.box_frames);
        for npc in self.space.traffic.npcs_mut() {
            if npc.is_scooping() && npc_in_ship_layer(npc) {
                npc_scoop(
                    boxes,
                    frames,
                    npc.state.position,
                    npc.hull.hit_radius,
                    &mut npc.cargo,
                );
            }
        }
    }
}

impl PlayerShip {
    /// The player's scoop flag (ship +0xc3, see
    /// [`scoop::player_scoops`](crate::scoop::player_scoops)): it owns an
    /// asteroid scoop and the fleet's holds ([`Session::capacity`]) are
    /// past the cargo it holds ([`Pilot::total_cargo`](crate::pilot::Pilot::total_cargo)).
    ///
    /// Read live. The original keeps the flag from the last
    /// `_ResetPlayerPrecalcedValues` (14 callers, the scoop itself among
    /// them), and the only time that shows is a jünk step in flight
    /// (`market::step_junk` in `before_move`), which can fill or free the
    /// hold with no reset: the original then still scoops one box into a
    /// hold the step just filled, or passes over one it just freed room
    /// for, until the next reset. The engine reads the hold as it is.
    #[must_use]
    fn player_scoops(&self, pilot: &Pilot, statics: &Statics) -> bool {
        crate::scoop::player_scoops(
            has_asteroid_scoop(&pilot.outfits, &statics.outfits),
            self.capacity(pilot, statics),
            pilot.total_cargo(),
        )
    }
}

impl Space {
    /// The cargo boxes in space (see [`cargo_box`](crate::cargo_box)).
    #[must_use]
    pub(super) fn cargo_boxes(&self) -> &[CargoBox] {
        self.boxes.boxes()
    }

    /// Steps the cargo boxes one tick (see
    /// [`CargoBoxes::step`](crate::cargo_box::CargoBoxes::step)): every flight tick, whatever the player's
    /// state, as `_DoPlayGameWork` runs `_HandleBoxes` (@0x444f0).
    pub(super) fn boxes_tick(&mut self) {
        self.boxes.step();
    }

    /// Clears the cargo boxes. The original sets `_killBoxes` and the
    /// next `_HandleBoxes` kills them (@0x2dfb7); here they go at once,
    /// which no frame drawn in between can tell apart.
    pub(super) fn clear_boxes(&mut self) {
        self.boxes.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::SimSound;
    use crate::ai::Peaceful;
    use crate::catalog::{HullRecord, ShipId, ShipRecord, StellarId, SystemId};
    use crate::catalog::{JunkId, OutfitId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::hull::Condition;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::scan::Latches;
    use crate::scoop::MOD_ASTEROID_SCOOP;
    use crate::session::tests::{at_gate, gated};
    use crate::stats::ShipStats;
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, begin_jump_now, carrying, catalog, fly_out, hull,
        land_now, mission, npc, outfit, ship,
    };
    use crate::testkit::{work, work_with};
    use crate::traffic::npc::{Npc, NpcHold};

    /// The warship escort's ship type: `InherentAI` 3, 60 tons of holds.
    const WARSHIP: ShipId = ShipId(150);
    /// The freighter escort's ship type: `InherentAI` 1, 30 tons of holds.
    const FREIGHTER: ShipId = ShipId(151);

    /// [`catalog`] whose ship 128, 40 pixels across, holds 20 tons, with
    /// the escort ship types: the warship 50 across and the freighter 30.
    fn boxed() -> FakePilotCatalog {
        let fields = |holds| ShipFields { holds, ..FAST };
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    inherent_ai: 3,
                    ..ship(150, fields(60))
                },
                ShipRecord {
                    inherent_ai: 1,
                    ..ship(151, fields(30))
                },
            ],
            hulls: vec![
                HullRecord {
                    size: Some(40),
                    ..hull(128)
                },
                HullRecord {
                    size: Some(50),
                    ..hull(150)
                },
                HullRecord {
                    size: Some(30),
                    ..hull(151)
                },
            ],
            ..catalog()
        }
    }

    /// `catalog`'s session at rest at system 130's centre, facing up,
    /// holding `tons` of food.
    fn flying(catalog: &FakePilotCatalog, tons: u32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(session.landed(), None);
        // Set up, so a pass keeps the NPCs a test adds.
        session.populate(catalog, &mut NeverFires);
        session.pilot.cargo.insert(Good::Commodity(0), tons);
        session
    }

    #[test]
    fn jettisoning_20_tons_in_flight_drops_4_boxes_behind_the_ship() {
        let catalog = boxed();
        let mut session = flying(&catalog, 20);
        session.jettison_cargo(true, &mut NeverFires);
        let boxes = session.cargo_boxes();
        assert_eq!(boxes.len(), 4);
        for made in boxes {
            // Half of 40 behind the ship facing up, kicked 0.3 straight
            // back: the draws' first outcomes.
            assert!((made.position - Vec2::new(0.0, 20.0)).length() < 1e-5);
            assert!((made.velocity - Vec2::new(0.0, 0.3)).length() < 1e-6);
            assert_eq!((made.life, made.frame, made.spin), (180.0, 0, -1));
        }
    }

    #[test]
    fn the_count_follows_the_total_a_box_a_5_tons_from_1_to_12() {
        for (tons, count) in [(1, 1), (4, 1), (10, 2), (59, 11), (100, 12)] {
            let catalog = boxed();
            let mut session = flying(&catalog, tons);
            session.jettison_cargo(false, &mut NeverFires);
            assert_eq!(session.cargo_boxes().len(), count, "{tons}");
        }
    }

    #[test]
    fn jettisoning_nothing_drops_nothing_and_sounds_nothing() {
        let catalog = boxed();
        let mut session = flying(&catalog, 0);
        let _ = session.take_sounds();
        session.jettison_cargo(true, &mut NeverFires);
        assert!(session.cargo_boxes().is_empty());
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn a_jettison_in_flight_sounds_snd_154() {
        let catalog = boxed();
        let mut session = flying(&catalog, 3);
        let _ = session.take_sounds();
        session.jettison_cargo(false, &mut NeverFires);
        assert_eq!(session.take_sounds(), [SimSound::CargoJettisoned]);
    }

    #[test]
    fn a_landed_jettison_drops_nothing_and_sounds_nothing() {
        let catalog = boxed();
        let mut session = flying(&catalog, 20);
        land_now(&mut session).expect("lands");
        let _ = session.take_sounds();
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.pilot().goods_held(), 0, "the goods go");
        assert!(session.cargo_boxes().is_empty());
        assert_eq!(session.take_sounds(), []);
    }

    /// [`boxed`]'s pilot flying with a warship, a freighter and a carried
    /// freighter fighter, its first traffic tick done, holding `tons` of
    /// food.
    fn escorted(catalog: &FakePilotCatalog, tons: u32) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        let escort = |ship, carried| Escort {
            ship,
            reserves: Reserves::default(),
            order: None,
            carried,
            wage: None,
            person: None,
            mark: None,
        };
        pilot.escorts = vec![
            escort(WARSHIP, false),
            escort(FREIGHTER, false),
            escort(FREIGHTER, true),
        ];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        work_with(
            &mut session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
        session.pilot.cargo.insert(Good::Commodity(0), tons);
        session
    }

    #[test]
    fn each_trader_escort_drops_its_share_of_the_goods_behind_it() {
        let catalog = boxed();
        let mut session = escorted(&catalog, 50);
        assert_eq!(
            session.capacity(),
            50,
            "the player's 20 and the freighter's 30"
        );
        let freighter = session
            .space
            .escorts
            .fleet
            .get(1)
            .copied()
            .flatten()
            .and_then(|id| session.space.npc(id))
            .expect("the freighter flies")
            .state;
        session.jettison_cargo(true, &mut NeverFires);
        let boxes = session.cargo_boxes();
        // The player's 50 tons drop 10; the freighter's 30 of them, 6.
        assert_eq!(boxes.len(), 16);
        let behind_freighter = freighter.position
            + crate::flight::facing((freighter.heading.trunc() + 180.0).rem_euclid(360.0)) * 15.0;
        assert_eq!(
            boxes
                .iter()
                .filter(|made| (made.position - behind_freighter).length() < 1e-3)
                .count(),
            6
        );
    }

    #[test]
    fn a_trader_escorts_share_is_of_the_goods_not_the_mission_cargo() {
        let mut catalog = boxed();
        catalog.missions = vec![mission(140)];
        let mut session = escorted(&catalog, 20);
        // 30 tons of an abortable mission's cargo go with the 20 of food.
        session.pilot.missions.fill(carrying(140, 30));
        let freighter = session
            .space
            .escorts
            .fleet
            .get(1)
            .copied()
            .flatten()
            .and_then(|id| session.space.npc(id))
            .expect("the freighter flies")
            .state;
        session.jettison_cargo(true, &mut NeverFires);
        let behind_freighter = freighter.position
            + crate::flight::facing((freighter.heading.trunc() + 180.0).rem_euclid(360.0)) * 15.0;
        let boxes = session.cargo_boxes();
        // The player's 50 tons drop 10; the freighter's 30 of the fleet's
        // 50 of the 20 tons of goods, 12 tons, 2 (of the total, 6).
        assert_eq!(
            boxes
                .iter()
                .filter(|made| (made.position - behind_freighter).length() < 1e-3)
                .count(),
            2
        );
        assert_eq!(boxes.len(), 12);
    }

    #[test]
    fn a_jettison_with_no_trader_escort_drops_the_players_alone() {
        let catalog = boxed();
        let mut session = escorted(&catalog, 50);
        session.pilot.escorts.truncate(1);
        session.space.escorts.fleet.truncate(1);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.cargo_boxes().len(), 10);
    }

    #[test]
    fn the_boxes_drift_each_tick_and_are_gone_after_their_life() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        work(&mut session, &catalog, Controls::default());
        let made = session.cargo_boxes()[0];
        assert!((made.position - Vec2::new(0.0, 20.3)).length() < 1e-5);
        assert_eq!((made.life, made.frame), (179.0, 35));
        for _ in 0..180 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.cargo_boxes().len(), 1);
        work(&mut session, &catalog, Controls::default());
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn the_boxes_step_on_while_the_player_is_destroyed() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        session.ship.condition = Condition::Destroyed;
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.cargo_boxes()[0].life, 179.0);
    }

    #[test]
    fn a_landing_clears_the_boxes() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn a_jump_clears_the_boxes() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.cargo_boxes().len(), 1, "dropped in hyperspace");
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn a_hypergate_clears_the_boxes() {
        let catalog = gated();
        let mut session = at_gate(&catalog);
        session.pilot.cargo.insert(Good::Commodity(0), 5);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.cargo_boxes().len(), 1);
        session.open_hypergate(&catalog).expect("opens");
        session
            .enter_hypergate(Some(SystemId(131)), &catalog, &mut NeverFires)
            .expect("enters");
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn an_m_in_flight_clears_the_boxes() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        session.run_set(
            &SetExpr::parse("M131").expect("parses"),
            &mut Scripted::default(),
        );
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert!(session.cargo_boxes().is_empty());
    }

    /// The scoop's outfit: `ModType` 31 in its first slot.
    const SCOOP: OutfitId = OutfitId(300);

    /// [`boxed`] with the scoop outfit.
    fn scoopable() -> FakePilotCatalog {
        let mut catalog = boxed();
        catalog
            .outfits
            .push(outfit(SCOOP.0, &[(MOD_ASTEROID_SCOOP, 0)]));
        catalog
    }

    /// [`flying`] holding `tons` of food and owning the scoop.
    fn scooper(catalog: &FakePilotCatalog, tons: u32) -> Session {
        let mut session = flying(catalog, tons);
        session.pilot.outfits.insert(SCOOP, 1);
        session
    }

    /// Drops a scoopable box of graphic 1 holding `contents` on the
    /// player's ship.
    fn drop_on_ship(session: &mut Session, contents: Option<i16>) {
        let at = session.player().position;
        session
            .space
            .boxes
            .spawn_scoopable(at, contents, 1, &mut NeverFires);
    }

    /// Every good held, with its tons, leaving out goods of none.
    fn held(session: &Session) -> Vec<(Good, u32)> {
        session
            .pilot()
            .cargo()
            .filter(|&(_, tons)| tons > 0)
            .collect()
    }

    #[test]
    fn the_scoop_takes_a_commodity_box_into_the_hold() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        drop_on_ship(&mut session, Some(2));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(held(&session), [(Good::Commodity(2), 1)]);
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn the_scoop_takes_a_junk_box_into_the_hold() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        drop_on_ship(&mut session, Some(1018));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(held(&session), [(Good::Junk(JunkId(146)), 1)]);
        assert!(session.cargo_boxes().is_empty());
    }

    /// Graphic 1's frame is 40 by 40, so the scoop reaches 20 past the
    /// hull's hit radius along an axis: a box 15 past it is taken, one 25
    /// past it is left.
    #[test]
    fn the_scoop_reaches_the_edge_of_a_boxs_frame_and_no_farther() {
        let mut catalog = scoopable();
        catalog.box_frames[1] = Some((40, 40));
        let mut session = scooper(&catalog, 0);
        let reach = session.ship.fit.hull.hit_radius;
        let at = session.player().position;
        let near = at + Vec2::new(reach + 15.0, 0.0);
        let far = at + Vec2::new(-(reach + 25.0), 0.0);
        session
            .space
            .boxes
            .spawn_scoopable(near, Some(2), 1, &mut NeverFires);
        session
            .space
            .boxes
            .spawn_scoopable(far, Some(3), 1, &mut NeverFires);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(held(&session), [(Good::Commodity(2), 1)]);
        assert_eq!(session.cargo_boxes().len(), 1, "the far box is left");
        assert_eq!(session.cargo_boxes()[0].contents, Some(3));
    }

    /// Ticks `session` once with a scoopable commodity box on its ship,
    /// and asserts that the box stays and the cargo is unchanged.
    fn assert_no_scoop(mut session: Session) {
        let before = held(&session);
        drop_on_ship(&mut session, Some(2));
        work(
            &mut session,
            &crate::testkit::catalog(),
            Controls::default(),
        );
        assert_eq!(held(&session), before);
        assert_eq!(session.cargo_boxes().len(), 1, "the box is left");
    }

    #[test]
    fn no_scoop_outfit_takes_no_box() {
        let catalog = scoopable();
        assert_no_scoop(flying(&catalog, 0));
    }

    #[test]
    fn a_full_hold_takes_no_box() {
        let catalog = scoopable();
        assert_no_scoop(scooper(&catalog, 20));
    }

    #[test]
    fn a_disabled_player_takes_no_box() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        session.ship.condition = Condition::Disabled;
        assert_no_scoop(session);
    }

    #[test]
    fn the_scoop_takes_no_jettisoned_box() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        let on_ship = session.ship.player;
        session
            .space
            .boxes
            .spawn(&on_ship, Some(0.0), &mut NeverFires);
        assert_eq!(session.cargo_boxes()[0].position, on_ship.position);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(held(&session), []);
        assert_eq!(session.cargo_boxes().len(), 1, "the box is left");
    }

    #[test]
    fn a_full_own_hold_scoops_into_a_trader_escorts_holds() {
        let catalog = scoopable();
        let mut session = escorted(&catalog, 20);
        session.pilot.outfits.insert(SCOOP, 1);
        drop_on_ship(&mut session, Some(2));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().held(Good::Commodity(2)), 1);
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn a_hold_with_a_ton_free_takes_one_box_of_two() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 19);
        drop_on_ship(&mut session, Some(2));
        drop_on_ship(&mut session, Some(3));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(
            held(&session),
            [(Good::Commodity(0), 19), (Good::Commodity(2), 1)]
        );
        assert_eq!(session.cargo_boxes().len(), 1);
        assert_eq!(session.cargo_boxes()[0].contents, Some(3));
    }

    #[test]
    fn mission_cargo_aboard_counts_toward_a_full_hold() {
        let mut catalog = scoopable();
        catalog.missions = vec![mission(140)];
        let mut session = scooper(&catalog, 10);
        session.pilot.missions.fill(carrying(140, 10));
        assert_no_scoop(session);
    }

    #[test]
    fn a_box_of_anything_else_is_used_up_for_nothing() {
        for contents in [Some(6), None, Some(-1), Some(1128)] {
            let catalog = scoopable();
            let mut session = scooper(&catalog, 0);
            drop_on_ship(&mut session, contents);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(held(&session), [], "{contents:?}");
            assert!(session.cargo_boxes().is_empty(), "{contents:?}");
        }
    }

    #[test]
    fn a_scoop_sounds_nothing() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        let mut twin = scooper(&catalog, 0);
        let _ = session.take_sounds();
        let _ = twin.take_sounds();
        drop_on_ship(&mut session, Some(2));
        work(&mut session, &catalog, Controls::default());
        work(&mut twin, &catalog, Controls::default());
        assert_eq!(held(&session), [(Good::Commodity(2), 1)]);
        assert_eq!(session.take_sounds(), twin.take_sounds());
    }

    #[test]
    fn every_scoop_resets_the_precalced_values_an_empty_one_too() {
        for contents in [Some(2), Some(6)] {
            let catalog = scoopable();
            let mut session = scooper(&catalog, 0);
            session.ship.latches.scan_latches = Latches {
                junk: false,
                outfits: false,
            };
            drop_on_ship(&mut session, contents);
            work(&mut session, &catalog, Controls::default());
            assert!(session.cargo_boxes().is_empty(), "{contents:?}");
            assert_eq!(
                session.ship.latches.scan_latches,
                Latches::ARMED,
                "{contents:?}"
            );
        }
    }

    #[test]
    fn no_scoop_leaves_the_precalced_values() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 20);
        let unarmed = Latches {
            junk: false,
            outfits: false,
        };
        session.ship.latches.scan_latches = unarmed;
        drop_on_ship(&mut session, Some(2));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.ship.latches.scan_latches, unarmed);
    }

    #[test]
    fn an_npc_not_scooping_leaves_the_box() {
        let catalog = scoopable();
        let mut session = scooper(&catalog, 0);
        let at = Vec2::new(300.0, 0.0);
        let id = session.space.traffic.add_npc(Npc {
            state: ShipState {
                position: at,
                velocity: Vec2::ZERO,
                heading: 0.0,
            },
            ..npc(1, ShipStats::default())
        });
        session
            .space
            .boxes
            .spawn_scoopable(at, Some(2), 1, &mut NeverFires);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.cargo_boxes().len(), 1);
        assert_eq!(
            session.space.npc(id).expect("here").cargo,
            NpcHold::default()
        );
    }

    /// A scooping NPC, a miner in state 17, at `at`, its disc 10 across
    /// its centre, its hold empty.
    fn scooping_npc(at: Vec2) -> Npc {
        let mut npc = npc(1, ShipStats::default());
        npc.goal = crate::ai::Goal::Scoop;
        npc.state.position = at;
        npc.hull.hit_radius = 10.0;
        npc
    }

    /// [`scoopable`] with box graphic 1 40 by 40, so a disc reaches 20
    /// past its radius along an axis.
    fn framed() -> FakePilotCatalog {
        let mut catalog = scoopable();
        catalog.box_frames[1] = Some((40, 40));
        catalog
    }

    #[test]
    fn a_scooping_npc_takes_a_box_by_its_own_disc_into_its_own_hold() {
        let catalog = framed();
        let mut session = flying(&catalog, 0);
        let at = Vec2::new(300.0, 0.0);
        let id = session.space.traffic.add_npc(scooping_npc(at));
        let boxes = &mut session.space.boxes;
        // 25 off on x: within 10 and 20, the box's half frame.
        boxes.spawn_scoopable(at + Vec2::new(25.0, 0.0), Some(2), 1, &mut NeverFires);
        // 35 off on x: beyond them.
        boxes.spawn_scoopable(at - Vec2::new(35.0, 0.0), Some(3), 1, &mut NeverFires);
        // On the player, which owns no scoop: no NPC's.
        boxes.spawn_scoopable(Vec2::ZERO, Some(4), 1, &mut NeverFires);
        work(&mut session, &catalog, Controls::default());
        let hold = session.space.npc(id).expect("here").cargo;
        assert_eq!((hold.tons(2), hold.total()), (1, 1));
        let left: Vec<_> = session
            .cargo_boxes()
            .iter()
            .map(|left| left.contents)
            .collect();
        assert_eq!(left, [Some(3), Some(4)]);
        assert_eq!(held(&session), [], "the player's hold is untouched");
    }

    #[test]
    fn a_disabled_npc_or_the_players_escort_or_fighter_scoops_nothing() {
        let duty = crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        };
        let at = Vec2::new(300.0, 0.0);
        let disabled = Npc {
            condition: Condition::Disabled,
            ..scooping_npc(at)
        };
        let escort = Npc {
            escort: Some(duty),
            ..scooping_npc(at)
        };
        // A fighter launched by the player: its carrier the player, on
        // escort duty as the player's fighters fly.
        let fighter = Npc {
            escort: Some(duty),
            carrier: Some(crate::bay::Carrier {
                ship: crate::combat::ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..scooping_npc(at)
        };
        for (npc, who) in [
            (disabled, "a disabled NPC"),
            (escort, "the player's escort"),
            (fighter, "the player's fighter"),
        ] {
            let catalog = framed();
            let mut session = flying(&catalog, 0);
            let id = session.space.traffic.add_npc(npc);
            session
                .space
                .boxes
                .spawn_scoopable(at, Some(2), 1, &mut NeverFires);
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.cargo_boxes().len(), 1, "the box is left: {who}");
            assert_eq!(
                session.space.npc(id).expect("here").cargo,
                NpcHold::default(),
                "{who}"
            );
        }
    }

    #[test]
    fn the_player_takes_a_box_before_a_scooping_npc_on_it() {
        let catalog = framed();
        let mut session = scooper(&catalog, 0);
        let id = session.space.traffic.add_npc(scooping_npc(Vec2::ZERO));
        drop_on_ship(&mut session, Some(2));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(held(&session), [(Good::Commodity(2), 1)]);
        assert_eq!(
            session.space.npc(id).expect("here").cargo,
            NpcHold::default()
        );
        assert!(session.cargo_boxes().is_empty());
    }
}
