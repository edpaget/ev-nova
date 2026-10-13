//! The session's side of [`cargo_box`](crate::cargo_box): the boxes a
//! jettison drops behind the player and its trader escorts
//! ([`Session::jettison_cargo`]), stepped each flight tick, and cleared
//! on every system change and landing. They are never saved.

use super::Session;
use crate::cargo_box::{CargoBox, escort_share};
use crate::chance::Chance;

impl Session {
    /// The cargo boxes in space (see [`cargo_box`](crate::cargo_box)).
    #[must_use]
    pub fn cargo_boxes(&self) -> &[CargoBox] {
        self.boxes.boxes()
    }

    /// Drops the boxes of a jettison of `total` tons, `goods` of them
    /// goods, drawing on `chance` (`_JettisonCargo` @0x3e1c2-0x3e2e7): the
    /// player's share is the whole total; each escort the fleet's holds
    /// count ([`Session::capacity`]: not a carried fighter, `InherentAI`
    /// 2 or less) whose NPC is in the system drops the goods times its
    /// `Holds` over the fleet's ([`escort_share`]), behind its NPC.
    pub(super) fn drop_cargo_boxes(
        &mut self,
        total: i64,
        goods: i64,
        chance: &mut (impl Chance + ?Sized),
    ) {
        // Precision lost only past 2^53 tons.
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        let player = self.player;
        self.boxes
            .drop_behind(&player, self.hull.sprite_size, total, chance);
        let fleet_holds = self.capacity();
        let droppers: Vec<_> = self
            .pilot
            .escorts
            .iter()
            .zip(&self.fleet)
            .filter(|(escort, _)| !escort.carried)
            .filter_map(|(escort, &placed)| {
                let record = self.ship_record(escort.ship)?;
                let npc = self.npc(placed?)?;
                (record.inherent_ai <= crate::escort::FREIGHTER_AI).then(|| {
                    (
                        npc.state,
                        npc.hull.sprite_size,
                        escort_share(goods, record.fields.holds, fleet_holds),
                    )
                })
            })
            .collect();
        for (state, width, share) in droppers {
            self.boxes.drop_behind(&state, width, share, chance);
        }
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
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::control::SetExpr;
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::market::Good;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::session::tests::{at_gate, gated};
    use crate::testkit::{
        FAST, FakePilotCatalog, Scripted, begin_jump_now, catalog, fly_out, hull, land_now, ship,
    };

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
        };
        pilot.escorts = vec![
            escort(WARSHIP, false),
            escort(FREIGHTER, false),
            escort(FREIGHTER, true),
        ];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
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
            .fleet
            .get(1)
            .copied()
            .flatten()
            .and_then(|id| session.npc(id))
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
    fn a_jettison_with_no_trader_escort_drops_the_players_alone() {
        let catalog = boxed();
        let mut session = escorted(&catalog, 50);
        session.pilot.escorts.truncate(1);
        session.fleet.truncate(1);
        session.jettison_cargo(true, &mut NeverFires);
        assert_eq!(session.cargo_boxes().len(), 10);
    }

    #[test]
    fn the_boxes_drift_each_tick_and_are_gone_after_their_life() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        session.tick(Controls::default());
        let made = session.cargo_boxes()[0];
        assert!((made.position - Vec2::new(0.0, 20.3)).length() < 1e-5);
        assert_eq!((made.life, made.frame), (179.0, 35));
        for _ in 0..180 {
            session.tick(Controls::default());
        }
        assert_eq!(session.cargo_boxes().len(), 1);
        session.tick(Controls::default());
        assert!(session.cargo_boxes().is_empty());
    }

    #[test]
    fn the_boxes_step_on_while_the_player_is_destroyed() {
        let catalog = boxed();
        let mut session = flying(&catalog, 5);
        session.jettison_cargo(true, &mut NeverFires);
        session.condition = Condition::Destroyed;
        session.tick(Controls::default());
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
        fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
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
}
