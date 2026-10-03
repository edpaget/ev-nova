//! The one list of record types.
//!
//! The `records!` invocation below is the only place a record type is
//! registered. It generates [`AnyRecord`], [`decode_any`], [`TYPES`], the
//! [`Registered`] impls and, in tests, the generic checks every record type
//! gets: its type code, its fixed size (an all-zero buffer of exactly `SIZE`
//! bytes decodes and uses every byte; one byte fewer fails with an unexpected
//! end), a JSON round trip, and that [`Registered::from_any`] takes its own
//! variant and only its own.

use nova_rsrc::{ResType, Resource};
use serde::{Deserialize, Serialize};

use crate::decode::{Entry, Record, decode};
use crate::error::{DecodeError, DecodeWarning};

/// The result of decoding one registered resource.
pub type AnyDecoded = Result<(Entry<AnyRecord>, Option<DecodeWarning>), DecodeError>;

/// A registered record type: one that [`AnyRecord`] can hold. Generic code
/// (such as the game data store) uses it to get a typed record
/// back out of an [`AnyRecord`].
pub trait Registered: Record {
    /// The record inside `any`, if `any` holds this type.
    fn from_any(any: &AnyRecord) -> Option<&Self>;
}

macro_rules! records {
    ($($variant:ident = $ty:ty, $code:literal $(, sample = $sample:expr)?;)*) => {
        /// A decoded record of any registered type, boxed so small records
        /// do not take a ship's space. JSON:
        /// `{"type": "Ship", "record": {...}}`.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "type", content = "record")]
        pub enum AnyRecord {
            $(
                #[doc = concat!("A `", $code, "` record.")]
                $variant(Box<$ty>),
            )*
        }

        impl AnyRecord {
            /// The resource type this record was decoded from.
            #[must_use]
            pub fn res_type(&self) -> ResType {
                match self {
                    $(Self::$variant(_) => <$ty as Record>::TYPE,)*
                }
            }
        }

        $(
            impl Registered for $ty {
                fn from_any(any: &AnyRecord) -> Option<&Self> {
                    match any {
                        AnyRecord::$variant(record) => Some(record),
                        _ => None,
                    }
                }
            }
        )*

        /// Every registered resource type, in registration order.
        pub const TYPES: &[ResType] = &[$(<$ty as Record>::TYPE),*];

        /// Decodes `res` as its registered record type; `None` if its type is
        /// not registered.
        #[must_use]
        pub fn decode_any(res: &Resource<'_>) -> Option<AnyDecoded> {
            let ty = res.res_type();
            $(
                if ty == <$ty as Record>::TYPE {
                    return Some(
                        decode::<$ty>(res)
                            .map(|(entry, warning)| {
                                (entry.map(|r| AnyRecord::$variant(Box::new(r))), warning)
                            }),
                    );
                }
            )*
            None
        }

        #[cfg(test)]
        mod generic {
            $(
                #[allow(non_snake_case)]
                mod $variant {
                    use crate::decode::{Record, decode_bytes, struct_name};
                    use crate::error::Cause;
                    use nova_rsrc::ResType;

                    type T = $ty;

                    #[test]
                    fn type_code_matches_its_mac_roman_spelling() {
                        assert_eq!(Some(<T as Record>::TYPE), ResType::from_mac_roman($code));
                    }

                    #[test]
                    fn all_zero_record_of_exactly_size_bytes_decodes() {
                        if let Some(size) = <T as Record>::SIZE {
                            let decoded = decode_bytes::<T>(&vec![0; size]).expect("decodes");
                            assert_eq!(decoded.consumed, size);
                        }
                    }

                    #[test]
                    fn one_byte_short_is_an_unexpected_end() {
                        if let Some(size) = <T as Record>::SIZE {
                            let err = decode_bytes::<T>(&vec![0; size - 1]).expect_err("short");
                            assert_eq!(err.cause, Cause::UnexpectedEnd, "{err}");
                            assert_eq!(err.path.0.first().map(String::as_str), Some(struct_name::<T>()));
                            assert!(err.offset < size as u64, "{err}");
                        }
                    }

                    #[test]
                    fn from_any_returns_its_own_variant() {
                        use super::super::{AnyRecord, Registered};
                        let bytes: Vec<u8> = super::super::sample_bytes!(<T as Record>::SIZE $(, $sample)?);
                        let record = decode_bytes::<T>(&bytes).expect("decodes").record;
                        let any = AnyRecord::$variant(Box::new(record.clone()));
                        assert_eq!(<T as Registered>::from_any(&any), Some(&record));
                    }

                    #[test]
                    fn from_any_rejects_other_variants() {
                        use super::super::{AnyRecord, Registered};
                        use crate::records::{checksum::Checksum, version::Version};
                        let other = if <T as Record>::TYPE == <Checksum as Record>::TYPE {
                            AnyRecord::Version(Box::new(decode_bytes::<Version>(&[1, 0, 0x80, 0]).expect("decodes").record))
                        } else {
                            AnyRecord::Checksum(Box::new(decode_bytes::<Checksum>(&[1, 2, 3, 4]).expect("decodes").record))
                        };
                        assert_eq!(<T as Registered>::from_any(&other), None);
                    }

                    #[test]
                    fn patterned_record_round_trips_through_json() {
                        let bytes: Vec<u8> = super::super::sample_bytes!(<T as Record>::SIZE $(, $sample)?);
                        let record = decode_bytes::<T>(&bytes).expect("decodes").record;
                        let json = serde_json::to_string(&record).expect("serializes");
                        let back: T = serde_json::from_str(&json).expect("deserializes");
                        assert_eq!(back, record);
                    }
                }
            )*
        }
    };
}

/// A test record's bytes: its sample if given, otherwise a pattern of its
/// fixed size.
#[cfg(test)]
macro_rules! sample_bytes {
    ($size:expr) => {
        crate::testutil::pattern($size.expect("variable-size records need a sample"))
    };
    ($size:expr, $sample:expr) => {
        $sample.to_vec()
    };
}
#[cfg(test)]
use sample_bytes;

records! {
    Boom = crate::records::boom::Boom, "bööm";
    Character = crate::records::character::Character, "chär";
    Checksum = crate::records::checksum::Checksum, "csüm", sample = [0x91, 0x84, 0xEE, 0xB0];
    Colors = crate::records::colors::Colors, "cölr";
    Cron = crate::records::cron::Cron, "crön";
    Desc = crate::records::desc::Desc, "dësc", sample = crate::records::desc::tests::SAMPLE;
    Disaster = crate::records::disaster::Disaster, "öops";
    Dude = crate::records::dude::Dude, "düde";
    Fleet = crate::records::fleet::Fleet, "flët";
    Govt = crate::records::govt::Govt, "gövt";
    Interface = crate::records::interface::Interface, "ïntf";
    Junk = crate::records::junk::Junk, "jünk";
    Mission = crate::records::mission::Mission, "mïsn";
    Nebula = crate::records::nebula::Nebula, "nëbu";
    Outfit = crate::records::outfit::Outfit, "oütf";
    Person = crate::records::person::Person, "përs";
    Rank = crate::records::rank::Rank, "ränk";
    Roid = crate::records::roid::Roid, "röid";
    Ship = crate::records::ship::Ship, "shïp";
    ShipAnim = crate::records::ship_anim::ShipAnim, "shän";
    Spin = crate::records::spin::Spin, "spïn";
    Stellar = crate::records::stellar::Stellar, "spöb";
    StrList = crate::records::string_list::StrList, "STR#", sample = crate::records::string_list::tests::SAMPLE;
    StrResource = crate::records::string::StrResource, "STR ", sample = crate::records::string::tests::SAMPLE;
    System = crate::records::system::System, "sÿst";
    Version = crate::records::version::Version, "vers", sample = [1, 0, 0x80, 0];
    Weapon = crate::records::weapon::Weapon, "wëap";
}

impl AnyRecord {
    /// The record's typed fields as JSON, without the enum's
    /// `{"type", "record"}` wrapper: what `serde_json::to_value` gives for
    /// the record struct itself.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).expect("records serialize");
        value["record"].take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{roid, spin};
    use nova_rsrc::ResourceFile;
    use nova_rsrc::fixture::ForkBuilder;
    use std::collections::HashSet;

    #[test]
    fn types_are_distinct() {
        let unique: HashSet<_> = TYPES.iter().collect();
        assert_eq!(unique.len(), TYPES.len());
    }

    #[test]
    fn decode_any_dispatches_on_type_and_skips_unregistered_types() {
        let file = ResourceFile::from_bytes(
            ForkBuilder::new()
                .resource(spin::Spin::TYPE, 1, None, &[0; 12])
                .resource(roid::Roid::TYPE, 2, None, &[0; 40])
                .resource(ResType::new(*b"PICT"), 3, None, b"")
                .build()
                .bytes,
        )
        .expect("valid fork");
        let kinds: Vec<Option<ResType>> = file
            .iter()
            .map(|res| decode_any(&res).map(|r| r.expect("decodes").0.record.res_type()))
            .collect();
        assert_eq!(
            kinds,
            vec![Some(spin::Spin::TYPE), Some(roid::Roid::TYPE), None]
        );
    }

    #[test]
    fn to_json_is_the_records_fields_without_the_enum_wrapper() {
        let spin = spin::Spin {
            sprites_id: 1000,
            masks_id: Some(crate::PictId(1001)),
            x_size: 48,
            y_size: 46,
            x_tiles: 6,
            y_tiles: 7,
        };
        let json = AnyRecord::Spin(Box::new(spin.clone())).to_json();
        assert_eq!(
            json,
            serde_json::json!({
                "sprites_id": 1000, "masks_id": 1001, "x_size": 48,
                "y_size": 46, "x_tiles": 6, "y_tiles": 7,
            })
        );
        assert!(json.get("type").is_none());
        assert_eq!(json, serde_json::to_value(&spin).expect("serializes"));
    }
}
