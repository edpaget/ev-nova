//! The player's escorts in flight: the session's side of
//! [`escort`](crate::escort).
//!
//! **Standing orders on entering a system.** Each escort's standing
//! order is kept on its record ([`Escort::order`](crate::Escort)) and
//! saved. Whenever the player enters a system (a jump's arrival, the
//! first traffic tick after a take-off, and the first after a pilot is
//! flown, whether new or loaded), the session follows its
//! [`RuleKey::EscortOrders`](crate::RuleKey::EscortOrders) source
//! ([`Session::with_escort_orders`]): by the engine, every escort's order
//! is reset to formation, as the original's `_RespawnEscort` (@0x3d546)
//! and `_HandlePlayer` (@0x6e3bb-0x6e44b) do; by the other reading, the
//! orders are kept.

use super::Session;
use crate::rulebook::RuleSource;

impl Session {
    /// This session with its escorts' standing orders reset, or kept, on
    /// entering a system as `source` says (see the module docs): the
    /// engine's reset by default.
    #[must_use]
    pub fn with_escort_orders(mut self, source: RuleSource) -> Self {
        self.escort_orders = source;
        self
    }

    /// Whether the escorts' standing orders are reset on entering a
    /// system ([`RuleSource::Engine`]) or kept.
    #[must_use]
    pub fn escort_orders(&self) -> RuleSource {
        self.escort_orders
    }

    /// The escorts enter the system: by the engine, their standing
    /// orders are reset.
    pub(super) fn enter_escorts(&mut self) {
        if self.escort_orders == RuleSource::Engine {
            for escort in &mut self.pilot.escorts {
                escort.order = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Peaceful;
    use crate::catalog::{ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::escort::EscortOrder;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::testkit::{FakePilotCatalog, catalog, jump};

    /// A pilot whose fleet is two escorts, both defending the player.
    fn defended(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            Escort {
                ship: ShipId(128),
                reserves: Reserves::full(30.0, 45.0, 300.0),
                order: Some(EscortOrder::Defend),
            };
            2
        ];
        pilot
    }

    fn orders(session: &Session) -> Vec<Option<EscortOrder>> {
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect()
    }

    fn defend_all(session: &mut Session) {
        for escort in &mut session.pilot.escorts {
            escort.order = Some(EscortOrder::Defend);
        }
    }

    fn tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
    }

    /// What each escort's order is after each system entry, `source`
    /// choosing: a jump's arrival, a landing and take-off, and the pilot
    /// saved, loaded and flown again; each time the save holding the
    /// order given before.
    fn after_each_entry(source: RuleSource) -> [Vec<Option<EscortOrder>>; 3] {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, defended(&catalog))
            .expect("flies")
            .with_escort_orders(source);
        assert_eq!(session.escort_orders(), source);
        tick(&mut session, &catalog);
        defend_all(&mut session);
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let jumped = orders(&session);

        let mut session = Session::fly(&catalog, defended(&catalog))
            .expect("flies")
            .with_escort_orders(source);
        tick(&mut session, &catalog);
        defend_all(&mut session);
        session.land().expect("lands on 128, under the ship");
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        session.take_off().expect("takes off");
        tick(&mut session, &catalog);
        let took_off = orders(&session);

        let saved = crate::save::encode(&defended(&catalog));
        assert!(saved.contains("\"defend\""));
        let pilot = crate::save::decode(&saved).expect("loads");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_escort_orders(source);
        tick(&mut session, &catalog);
        let loaded = orders(&session);
        [jumped, took_off, loaded]
    }

    #[test]
    fn by_the_engine_entering_a_system_resets_every_escort_to_formation() {
        let none = vec![None, None];
        assert_eq!(
            after_each_entry(RuleSource::Engine),
            [none.clone(), none.clone(), none]
        );
        assert_eq!(
            Session::start(&catalog()).expect("starts").escort_orders(),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_entering_a_system_keeps_the_standing_orders() {
        let defend = vec![Some(EscortOrder::Defend); 2];
        assert_eq!(
            after_each_entry(RuleSource::Bible),
            [defend.clone(), defend.clone(), defend]
        );
    }

    #[test]
    fn the_orders_stand_until_the_next_entry() {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, defended(&catalog)).expect("flies");
        tick(&mut session, &catalog);
        defend_all(&mut session);
        tick(&mut session, &catalog);
        assert_eq!(orders(&session), [Some(EscortOrder::Defend); 2]);
    }
}
