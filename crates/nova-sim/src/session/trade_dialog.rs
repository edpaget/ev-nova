//! The exchange dialog's words: the session's side of
//! [`trade_dialog`](crate::trade_dialog).
//!
//! [`Session::market`] puts `STR#` 2002's words ([`TradeWords`]) in
//! `Market.words`, as the state layer's `market` cannot read strings and
//! gives the stock ones. The header, the row markers and the cargo
//! summary are told in them.

use crate::catalog::CommCatalog;
use crate::news::MISC_STRINGS;
use crate::trade_dialog::TradeWords;

use super::Session;

impl Session {
    /// `STR#` 2002's words for the exchange's dialog (see the module docs).
    pub(super) fn trade_words(&self) -> TradeWords {
        TradeWords::from_list(&self.setup.strings.0.string_list(MISC_STRINGS))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::catalog::{CommCatalog, StellarId};
    use crate::chance::NeverFires;
    use crate::news::MISC_STRINGS;
    use crate::session::Session;
    use crate::testkit::{carrying, jump_with, land_now};
    use crate::trade_dialog::{HeldIn, TradeWords};

    /// `STR#` 2002 with every entry numbered: "#1", "#2", …
    struct Numbered;

    impl CommCatalog for Numbered {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                MISC_STRINGS => (1..=400).map(|n| format!("#{n}")).collect(),
                _ => Vec::new(),
            }
        }
    }

    /// Landed at the trade center 140, with `STR#` 2002 numbered.
    fn docked() -> Session {
        let catalog = crate::session::tests::surplus();
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Numbered));
        jump_with(&mut session, &catalog, 131, &mut NeverFires);
        assert_eq!(land_now(&mut session), Ok(StellarId(140)));
        session
    }

    #[test]
    fn the_market_carries_str_2002s_trade_words() {
        let market = docked().market().expect("an exchange");
        assert_eq!(market.words.commodity, "#197");
        assert_eq!(market.words.and, "#392");
        let list: Vec<String> = (1..=400).map(|n| format!("#{n}")).collect();
        assert_eq!(market.words, TradeWords::from_list(&list));
    }

    #[test]
    fn the_market_counts_mission_cargo_in_its_summary() {
        let mut session = docked();
        session.pilot.missions.put(2, carrying(140, 4));
        let market = session.market().expect("an exchange");
        assert_eq!(
            (market.summary.mission_kinds, market.summary.mission_tons),
            (1, 4)
        );
        assert_eq!(market.summary.total_cargo, 4);
        assert_eq!(market.ship_capacity, session.stats().capacity);
    }

    #[test]
    fn an_escorts_holds_make_the_header_say_in_fleet() {
        let session = crate::session::tests::landed_with_a_fleet();
        let market = session.market().expect("an exchange");
        assert_eq!((market.ship_capacity, market.capacity), (20, 35));
        assert_eq!(market.held_in(), HeldIn::Fleet);
        assert_eq!(
            docked().market().expect("an exchange").held_in(),
            HeldIn::Hold
        );
    }
}
