//! The expression fields of every record type, and the load-time check
//! that parses them all.

use std::fmt;

use nova_rsrc::ResType;

use super::{SetExpr, TestExpr};
use crate::decode::Record;
use crate::records::character::Character;
use crate::records::cron::Cron;
use crate::records::disaster::Disaster;
use crate::records::fleet::Fleet;
use crate::records::junk::Junk;
use crate::records::mission::Mission;
use crate::records::nebula::Nebula;
use crate::records::outfit::Outfit;
use crate::records::person::Person;
use crate::records::ship::Ship;
use crate::records::stellar::Stellar;
use crate::records::system::System;
use crate::registry::{AnyRecord, Registered};
use crate::store::GameData;

/// Which grammar a field holds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExprKind {
    /// A test expression ([`TestExpr`]).
    Test,
    /// A set expression ([`SetExpr`]).
    Set,
}

/// One expression field of one record type.
#[derive(Copy, Clone, Debug)]
pub struct ExprField {
    /// The record type.
    pub res_type: ResType,
    /// The field's name in the Rust record struct.
    pub field: &'static str,
    /// Which grammar it holds.
    pub kind: ExprKind,
    /// The field's text in a record; `None` for a record of another type.
    pub text: fn(&AnyRecord) -> Option<&str>,
}

/// An [`ExprField`] for `$record.$field`.
macro_rules! field {
    ($record:ty, $field:ident, $kind:ident) => {
        ExprField {
            res_type: <$record as Record>::TYPE,
            field: stringify!($field),
            kind: ExprKind::$kind,
            text: |any| <$record as Registered>::from_any(any).map(|r| r.$field.as_str()),
        }
    };
}

/// Every expression field, by record type.
pub static EXPR_FIELDS: &[ExprField] = &[
    field!(Mission, avail_bits, Test),
    field!(Mission, on_accept, Set),
    field!(Mission, on_refuse, Set),
    field!(Mission, on_success, Set),
    field!(Mission, on_failure, Set),
    field!(Mission, on_abort, Set),
    field!(Mission, on_ship_done, Set),
    field!(Cron, enable_on, Test),
    field!(Cron, on_start, Set),
    field!(Cron, on_end, Set),
    field!(Outfit, availability, Test),
    field!(Outfit, on_purchase, Set),
    field!(Outfit, on_sell, Set),
    field!(Ship, availability, Test),
    field!(Ship, appear_on, Test),
    field!(Ship, on_purchase, Set),
    field!(Ship, on_capture, Set),
    field!(Ship, on_retire, Set),
    field!(System, visibility, Test),
    field!(Fleet, appear_on, Test),
    field!(Person, active_on, Test),
    field!(Nebula, active_on, Test),
    field!(Nebula, on_explore, Set),
    field!(Disaster, activate_on, Test),
    field!(Junk, buy_on, Test),
    field!(Junk, sell_on, Test),
    field!(Stellar, on_dominate, Set),
    field!(Stellar, on_release, Set),
    field!(Stellar, on_destroy, Set),
    field!(Stellar, on_regen, Set),
    field!(Character, on_start, Set),
];

/// An expression in the game data that does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprError {
    /// The record's type.
    pub res_type: ResType,
    /// The record's ID.
    pub id: i16,
    /// The field, as in [`ExprField::field`].
    pub field: &'static str,
    /// What was wrong, with the expression's text.
    pub message: String,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} `{}`: {}",
            self.res_type, self.id, self.field, self.message
        )
    }
}

/// Parses every expression field of every record in `data`, and returns
/// the ones that do not parse: in [`EXPR_FIELDS`] order, then by ascending
/// ID. Records that fail to decode are skipped; the store reports those.
#[must_use]
pub fn check_expressions(data: &GameData) -> Vec<ExprError> {
    let mut errors = Vec::new();
    for field in EXPR_FIELDS {
        for &id in data.ids(field.res_type) {
            let Some(Ok(entry)) = data.get_any(field.res_type, id) else {
                continue;
            };
            let Some(text) = (field.text)(entry.record) else {
                continue;
            };
            let parsed = match field.kind {
                ExprKind::Test => TestExpr::parse(text).map(drop),
                ExprKind::Set => SetExpr::parse(text).map(drop),
            };
            if let Err(err) = parsed {
                errors.push(ExprError {
                    res_type: field.res_type,
                    id,
                    field: field.field,
                    message: format!("{err} in {text:?}"),
                });
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};

    use super::*;
    use crate::store::fs::{DirLister, EntryKind, Listing};
    use crate::testutil::{Buf, buf};

    /// One data file, `/data/Nova Data`, holding a fork.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    fn load(file: &ForkBuilder) -> GameData {
        let file = OneFile(file.build().bytes);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    /// Writes `text` as a C string at `offset`.
    fn text(buf: Buf, offset: usize, text: &str) -> Buf {
        buf.bytes(offset, text.as_bytes())
    }

    /// A mïsn whose `avail_bits` and `on_accept` hold these.
    fn mission(avail_bits: &str, on_accept: &str) -> Vec<u8> {
        text(text(buf::<Mission>(), 0x5C, avail_bits), 0x15B, on_accept).0
    }

    /// A crön with well-formed expressions in each field.
    fn cron() -> Vec<u8> {
        let cron = text(buf::<Cron>(), 0x18, "b1 & !b2");
        text(text(cron, 0x117, "b3 R(b4 b5)"), 0x216, "!b3").0
    }

    #[test]
    fn the_table_names_every_expression_field() {
        let fields: Vec<(String, &str, ExprKind)> = EXPR_FIELDS
            .iter()
            .map(|f| (f.res_type.to_string(), f.field, f.kind))
            .collect();
        let expected: Vec<(String, &str, ExprKind)> = [
            ("mïsn", "avail_bits", ExprKind::Test),
            ("mïsn", "on_accept", ExprKind::Set),
            ("mïsn", "on_refuse", ExprKind::Set),
            ("mïsn", "on_success", ExprKind::Set),
            ("mïsn", "on_failure", ExprKind::Set),
            ("mïsn", "on_abort", ExprKind::Set),
            ("mïsn", "on_ship_done", ExprKind::Set),
            ("crön", "enable_on", ExprKind::Test),
            ("crön", "on_start", ExprKind::Set),
            ("crön", "on_end", ExprKind::Set),
            ("oütf", "availability", ExprKind::Test),
            ("oütf", "on_purchase", ExprKind::Set),
            ("oütf", "on_sell", ExprKind::Set),
            ("shïp", "availability", ExprKind::Test),
            ("shïp", "appear_on", ExprKind::Test),
            ("shïp", "on_purchase", ExprKind::Set),
            ("shïp", "on_capture", ExprKind::Set),
            ("shïp", "on_retire", ExprKind::Set),
            ("sÿst", "visibility", ExprKind::Test),
            ("flët", "appear_on", ExprKind::Test),
            ("përs", "active_on", ExprKind::Test),
            ("nëbu", "active_on", ExprKind::Test),
            ("nëbu", "on_explore", ExprKind::Set),
            ("öops", "activate_on", ExprKind::Test),
            ("jünk", "buy_on", ExprKind::Test),
            ("jünk", "sell_on", ExprKind::Test),
            ("spöb", "on_dominate", ExprKind::Set),
            ("spöb", "on_release", ExprKind::Set),
            ("spöb", "on_destroy", ExprKind::Set),
            ("spöb", "on_regen", ExprKind::Set),
            ("chär", "on_start", ExprKind::Set),
        ]
        .into_iter()
        .map(|(ty, field, kind)| (ty.to_owned(), field, kind))
        .collect();
        assert_eq!(fields, expected);
    }

    #[test]
    fn each_field_reads_its_own_record_type_only() {
        let data = load(
            &ForkBuilder::new()
                .resource(Mission::TYPE, 128, None, &mission("b7", "b8"))
                .resource(Cron::TYPE, 128, None, &cron()),
        );
        let any = |ty| {
            data.get_any(ty, 128)
                .expect("present")
                .expect("decodes")
                .record
        };
        let field = |ty: &str, name: &str| {
            EXPR_FIELDS
                .iter()
                .find(|f| f.res_type.to_string() == ty && f.field == name)
                .expect("listed")
        };
        let avail = field("mïsn", "avail_bits");
        assert_eq!((avail.text)(any(Mission::TYPE)), Some("b7"));
        assert_eq!((avail.text)(any(Cron::TYPE)), None);
        let accept = field("mïsn", "on_accept");
        assert_eq!((accept.text)(any(Mission::TYPE)), Some("b8"));
        let end = field("crön", "on_end");
        assert_eq!((end.text)(any(Cron::TYPE)), Some("!b3"));
        assert_eq!((end.text)(any(Mission::TYPE)), None);
    }

    #[test]
    fn one_malformed_expression_is_one_error_naming_its_record_and_field() {
        let data = load(
            &ForkBuilder::new()
                .resource(
                    Mission::TYPE,
                    128,
                    None,
                    &mission("b1 & (b2 | !b3)", "b12 Z3"),
                )
                .resource(Mission::TYPE, 129, None, &mission("b4", "b5 !b6"))
                .resource(Cron::TYPE, 128, None, &cron()),
        );
        let errors = check_expressions(&data);
        assert_eq!(
            errors,
            [ExprError {
                res_type: Mission::TYPE,
                id: 128,
                field: "on_accept",
                message: "unknown set operator `Z` at byte 4 in \"b12 Z3\"".into(),
            }]
        );
        assert_eq!(
            errors[0].to_string(),
            "mïsn 128 `on_accept`: unknown set operator `Z` at byte 4 in \"b12 Z3\""
        );
    }

    #[test]
    fn errors_come_in_table_order_then_by_id() {
        let data = load(
            &ForkBuilder::new()
                .resource(Mission::TYPE, 130, None, &mission("x", "Z"))
                .resource(Mission::TYPE, 129, None, &mission("y", "b1"))
                .resource(Cron::TYPE, 128, None, &text(buf::<Cron>(), 0x18, "z").0),
        );
        let found: Vec<(String, i16, &str)> = check_expressions(&data)
            .into_iter()
            .map(|e| (e.res_type.to_string(), e.id, e.field))
            .collect();
        assert_eq!(
            found,
            [
                ("mïsn".to_owned(), 129, "avail_bits"),
                ("mïsn".to_owned(), 130, "avail_bits"),
                ("mïsn".to_owned(), 130, "on_accept"),
                ("crön".to_owned(), 128, "enable_on"),
            ]
        );
    }

    #[test]
    fn a_record_that_fails_to_decode_is_skipped() {
        let data = load(
            &ForkBuilder::new()
                .resource(Mission::TYPE, 128, None, &[0; 10])
                .resource(Mission::TYPE, 129, None, &mission("b1", "b2")),
        );
        assert!(matches!(data.get_any(Mission::TYPE, 128), Some(Err(_))));
        assert_eq!(check_expressions(&data), []);
    }
}
