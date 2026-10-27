//! The session's tests of the cargo names it gives the view's wording
//! (rdm view-sim-seam phase 12): `TagNames::cargo_name` reads `STR#`
//! 4001 of the string lists the session was given, by the cargo's place
//! in it, as a pickup ship's line names its cargo
//! (`_HandlePlayerBoardAttempt` @0x653fb-0x65543).

use std::rc::Rc;

use super::*;
use crate::catalog::CommCatalog;
use crate::testkit::catalog;
use crate::text_tags::TagNames;

/// `STR#` 4001: "Food" and "*Passengers".
#[derive(Debug)]
struct Cargoes;

impl CommCatalog for Cargoes {
    fn string_list(&self, id: i16) -> Vec<String> {
        match id {
            4001 => vec!["Food".to_owned(), "*Passengers".to_owned()],
            _ => Vec::new(),
        }
    }
}

#[test]
fn a_cargo_is_named_by_its_place_in_str_4001() {
    let catalog = catalog();
    let session = Session::start(&catalog)
        .expect("starts")
        .with_strings(Rc::new(Cargoes));
    assert_eq!(session.cargo_name(0).as_deref(), Some("Food"));
    assert_eq!(
        session.cargo_name(1).as_deref(),
        Some("*Passengers"),
        "its star kept for the line to drop"
    );
    assert_eq!(session.cargo_name(2), None, "past the list");
}
