//! Wiring: synthetic multi-type forks decoded through `decode_file`.

use nova_data::records::boom::Boom;
use nova_data::records::spin::Spin;
use nova_data::records::string_list::StrList;
use nova_data::{AnyRecord, Cause, FileReport, MacString, PictId, Record, decode_file};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{ResType, ResourceFile};

fn file(builder: &ForkBuilder) -> ResourceFile {
    ResourceFile::from_bytes(builder.build().bytes).expect("valid fork")
}

fn code(text: &str) -> ResType {
    ResType::from_mac_roman(text).expect("four Mac Roman characters")
}

/// One fork mixing good, bad, trailing-byte, out-of-scope and unknown
/// resources of several types.
fn mixed_report() -> FileReport {
    let spin = [3, 232, 3, 233, 0, 48, 0, 48, 0, 6, 0, 6];
    let mut good_desc = b"Caf\x8E\0".to_vec();
    good_desc.extend([0xFF; 36]);
    let roid_with_trailing_bytes = [0; 44];

    let file = file(
        &ForkBuilder::new()
            .resource(code("spïn"), 1000, Some(b"star"), &spin)
            .resource(code("bööm"), 128, Some(b"short"), &[0, 100, 0])
            .resource(code("PICT"), 128, None, b"pixels")
            .resource(code("STR#"), 128, None, b"\x00\x01\x02\xD2\xD3")
            .resource(code("dësc"), 128, None, b"no terminator")
            .resource(code("dësc"), 129, Some(b"caf\x8E"), &good_desc)
            .resource(code("zzzz"), 1, None, b"?")
            .resource(code("zzzz"), 2, None, b"?")
            .resource(code("röid"), 128, None, &roid_with_trailing_bytes)
            .resource(code("STR#"), 129, Some(b"bad list"), b"\x00\x02\x01a"),
    );
    decode_file(&file)
}

#[test]
fn good_records_decode_in_map_order() {
    let report = mixed_report();
    let decoded: Vec<(ResType, i16, Option<&str>)> = report
        .records
        .iter()
        .map(|entry| (entry.record.res_type(), entry.id, entry.name.as_deref()))
        .collect();
    assert_eq!(
        decoded,
        vec![
            (code("spïn"), 1000, Some("star")),
            (code("STR#"), 128, None),
            (code("dësc"), 129, Some("café")),
            (code("röid"), 128, None),
        ]
    );
    assert_eq!(
        report.records[0].record,
        AnyRecord::Spin(Box::new(Spin {
            sprites_id: 1000,
            masks_id: Some(PictId(1001)),
            x_size: 48,
            y_size: 48,
            x_tiles: 6,
            y_tiles: 6,
        }))
    );
    assert_eq!(
        report.records[1].record,
        AnyRecord::StrList(Box::new(StrList {
            strings: vec![MacString::from("\u{201C}\u{201D}")],
        }))
    );
}

/// `(type, id, name, path, offset, cause)` of one error.
type ErrorSummary<'a> = (ResType, i16, Option<&'a str>, String, u64, &'a Cause);

#[test]
fn every_bad_record_is_reported_with_type_id_name_path_and_offset() {
    let report = mixed_report();
    let errors: Vec<ErrorSummary<'_>> = report
        .errors
        .iter()
        .map(|e| {
            (
                e.res_type,
                e.id,
                e.name.as_deref(),
                e.field.path.to_string(),
                e.field.offset,
                &e.field.cause,
            )
        })
        .collect();
    let invalid = Cause::Invalid("unterminated string".to_owned());
    assert_eq!(
        errors,
        vec![
            (
                Boom::TYPE,
                128,
                Some("short"),
                "Boom → sound_index".to_owned(),
                2,
                &Cause::UnexpectedEnd
            ),
            (
                StrList::TYPE,
                129,
                Some("bad list"),
                "StrList → strings[1]".to_owned(),
                4,
                &Cause::UnexpectedEnd
            ),
            (
                code("dësc"),
                128,
                None,
                "Desc → text".to_owned(),
                0,
                &invalid
            ),
        ]
    );
    assert_eq!(
        report.errors[0].to_string(),
        "bööm 128 \"short\": field Boom → sound_index at byte 0x2: \
         unexpected end of data (record is 3 bytes, layout needs 6)"
    );
}

#[test]
fn trailing_bytes_warn_and_other_types_are_skipped() {
    let report = mixed_report();
    let warnings: Vec<String> = report.warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        ["röid 128: 4 trailing bytes ignored (layout needs 40, record is 44 bytes)"]
    );
    assert_eq!(report.skipped, vec![(code("PICT"), 1), (code("zzzz"), 2)]);
}

#[test]
fn an_empty_file_reports_nothing() {
    let report = decode_file(&file(&ForkBuilder::new()));
    assert!(report.records.is_empty());
    assert!(report.warnings.is_empty());
    assert!(report.errors.is_empty());
    assert!(report.skipped.is_empty());
}

#[test]
fn records_round_trip_through_json() {
    let file = file(
        &ForkBuilder::new()
            .resource(code("spïn"), 1000, None, &[0; 12])
            .resource(code("csüm"), 128, None, &[0x91, 0x84, 0xEE, 0xB0]),
    );
    let report = decode_file(&file);
    assert!(report.errors.is_empty());
    for entry in &report.records {
        let json = serde_json::to_string(&entry.record).expect("serializes");
        let back: AnyRecord = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, entry.record);
    }
    let checksum = serde_json::to_value(&report.records[1].record).expect("serializes");
    assert_eq!(
        checksum,
        serde_json::json!({"type": "Checksum", "record": {"data": "9184eeb0"}})
    );
}
