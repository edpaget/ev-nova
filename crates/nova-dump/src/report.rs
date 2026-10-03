//! The summary printed after a dump, and the exit code it implies.
//!
//! ```text
//! records: 7143 in 25 JSON files
//! pictures: 671 PICT, 29 cicn, 10 ppat
//! sprite sheets: 282 rlëD
//! sounds: 227 snd
//! ignored: 5
//! warnings: 0
//! Failures:
//!   shïp 128 "Shuttle" in /p/Bad Ship: unexpected end of data at byte 0x22 (record is 40 bytes, layout needs 1860)
//!   file /p/Broken: not a valid resource fork: map section (offset 300, length 50) extends past the end of the 20-byte fork
//! failures: 2
//! ```
//!
//! A `not exported:` line lists types that are neither registered records
//! nor pictures, sheets or sounds, when there are any; each warning is
//! listed under `warnings:`. Verbose mode lists each ignored entry, and
//! under each failure adds the field path (records), the failing offset
//! and a hex window around it, or the data length when the error carries
//! no offset.

use std::io::{self, Write};

use nova_data::store::order::IgnoreReason;
use nova_rsrc::{LoadError, ResType};

use crate::export::{Failure, Outcome};
use crate::hex;

/// The exit code for a finished run: 0 without failures, 1 with.
#[must_use]
pub fn exit_code(outcome: &Outcome) -> u8 {
    u8::from(!outcome.failures.is_empty())
}

/// Writes the summary of `outcome` to `out`.
pub fn render(outcome: &Outcome, verbose: bool, out: &mut impl Write) -> io::Result<()> {
    let counts = &outcome.counts;
    writeln!(
        out,
        "records: {} in {} JSON files",
        counts.records, counts.json_files
    )?;
    writeln!(
        out,
        "pictures: {} PICT, {} cicn, {} ppat",
        counts.pict, counts.cicn, counts.ppat
    )?;
    writeln!(out, "sprite sheets: {} rlëD", counts.rled)?;
    writeln!(out, "sounds: {} snd", counts.snd)?;
    if !outcome.not_exported.is_empty() {
        let types: Vec<String> = outcome
            .not_exported
            .iter()
            .map(|(ty, n)| format!("{ty} ({n})"))
            .collect();
        writeln!(out, "not exported: {}", types.join(", "))?;
    }
    writeln!(out, "ignored: {}", outcome.ignored.len())?;
    if verbose {
        for entry in outcome.ignored {
            let reason = ignore_reason(&entry.reason);
            writeln!(out, "  {}: {reason}", entry.path.display())?;
        }
    }
    writeln!(out, "warnings: {}", outcome.warnings.len())?;
    for warning in &outcome.warnings {
        writeln!(out, "  {warning}")?;
    }
    if !outcome.failures.is_empty() {
        writeln!(out, "Failures:")?;
        for failure in &outcome.failures {
            write_failure(failure, verbose, out)?;
        }
    }
    writeln!(out, "failures: {}", outcome.failures.len())
}

fn write_failure(failure: &Failure, verbose: bool, out: &mut impl Write) -> io::Result<()> {
    match failure {
        Failure::Record {
            error,
            source,
            data,
        } => {
            let field = &error.field;
            let lengths = match error.expected_len {
                Some(expected) => {
                    format!(
                        " (record is {} bytes, layout needs {expected})",
                        error.actual_len
                    )
                }
                None => format!(" (record is {} bytes)", error.actual_len),
            };
            writeln!(
                out,
                "  {} in {}: {} at byte {:#x}{lengths}",
                identity(error.res_type, error.id, error.name.as_deref()),
                source.display(),
                field.cause,
                field.offset,
            )?;
            if verbose {
                if !field.path.0.is_empty() {
                    writeln!(out, "    field: {}", field.path)?;
                }
                let offset = usize::try_from(field.offset).unwrap_or(usize::MAX);
                write_bytes(data, Some(offset), out)?;
            }
        }
        Failure::Resource {
            res_type,
            id,
            name,
            source,
            data,
            error,
        } => {
            writeln!(
                out,
                "  {} in {}: {error}",
                identity(*res_type, *id, *name),
                source.display()
            )?;
            if verbose {
                write_bytes(data, error.offset(), out)?;
            }
        }
        Failure::File(failed) => {
            let cause = match &failed.error {
                LoadError::Io { source, .. } => format!("read error: {source}"),
                LoadError::Parse { source, .. } => format!("not a valid resource fork: {source}"),
                LoadError::NoResourceFork { .. } => "no resource fork".to_owned(),
            };
            writeln!(out, "  file {}: {cause}", failed.path.display())?;
        }
    }
    Ok(())
}

/// The failing offset and a hex window around it, or the data length when
/// there is no offset.
fn write_bytes(data: &[u8], offset: Option<usize>, out: &mut impl Write) -> io::Result<()> {
    let Some(offset) = offset else {
        return writeln!(out, "    data: {} bytes", data.len());
    };
    writeln!(out, "    offset: {offset:#x}")?;
    for row in hex::window(data, offset) {
        writeln!(out, "    {row}")?;
    }
    Ok(())
}

/// `shïp 128 "Shuttle"`, or `shïp 128` when unnamed.
fn identity(ty: ResType, id: i16, name: Option<&str>) -> String {
    match name {
        Some(name) => format!("{ty} {id} {name:?}"),
        None => format!("{ty} {id}"),
    }
}

/// Why an entry was skipped, in words.
pub(crate) fn ignore_reason(reason: &IgnoreReason) -> String {
    match reason {
        IgnoreReason::Hidden => "hidden".to_owned(),
        IgnoreReason::Symlink => "symbolic link".to_owned(),
        IgnoreReason::NotAFile => "not a file or folder".to_owned(),
        IgnoreReason::DataSubFolder => "folder inside the data directory".to_owned(),
        IgnoreReason::TooDeep => "plug-ins folder nested too deep".to_owned(),
        IgnoreReason::NotGameData(ext) => format!("not game data (.{ext})"),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};

    use nova_data::graphics::{GraphicsError, RLED};
    use nova_data::sound::SoundError;
    use nova_data::store::order::IgnoreReason;
    use nova_data::store::{FailedFile, IgnoredEntry, Origin};
    use nova_data::{Cause, DecodeError, DecodeWarning, FieldError, FieldPath};
    use nova_rsrc::{LoadError, ParseError, ResType, Section};

    use super::*;
    use crate::export::{Counts, Failure, MediaError, Outcome};

    const SHIP: ResType = ResType::new([b's', b'h', 0x95, b'p']);

    fn counts() -> Counts {
        Counts {
            json_files: 25,
            records: 7143,
            pict: 671,
            cicn: 29,
            ppat: 10,
            rled: 282,
            snd: 227,
        }
    }

    fn outcome(failures: Vec<Failure<'_>>) -> Outcome<'_> {
        Outcome {
            counts: counts(),
            warnings: Vec::new(),
            failures,
            not_exported: Vec::new(),
            ignored: &[],
        }
    }

    /// Hex rows as verbose mode prints them.
    fn indented(rows: &[String]) -> String {
        rows.iter().flat_map(|row| ["    ", row, "\n"]).collect()
    }

    fn text(outcome: &Outcome, verbose: bool) -> String {
        let mut out = Vec::new();
        render(outcome, verbose, &mut out).expect("writes to memory");
        String::from_utf8(out).expect("UTF-8")
    }

    fn short_ship() -> DecodeError {
        DecodeError {
            res_type: SHIP,
            id: 128,
            name: Some("Shuttle".to_owned()),
            expected_len: Some(1860),
            actual_len: 40,
            field: FieldError {
                path: FieldPath(vec!["Ship".to_owned(), "weapons[2]".to_owned()]),
                offset: 0x22,
                cause: Cause::UnexpectedEnd,
            },
        }
    }

    const SHIP_LINE: &str = "  shïp 128 \"Shuttle\" in /p/Bad Ship: unexpected end of data \
                             at byte 0x22 (record is 40 bytes, layout needs 1860)";

    #[test]
    fn a_clean_run_lists_the_counts_and_no_failures() {
        assert_eq!(
            text(&outcome(Vec::new()), false),
            "records: 7143 in 25 JSON files\n\
             pictures: 671 PICT, 29 cicn, 10 ppat\n\
             sprite sheets: 282 rlëD\n\
             sounds: 227 snd\n\
             ignored: 0\n\
             warnings: 0\n\
             failures: 0\n"
        );
        assert_eq!(exit_code(&outcome(Vec::new())), 0);
    }

    #[test]
    fn unexported_types_ignored_entries_and_warnings_are_listed() {
        let warning = DecodeWarning::TrailingBytes {
            res_type: SHIP,
            id: 5,
            name: None,
            consumed: 1860,
            actual_len: 1862,
        };
        let ignored = [
            IgnoredEntry {
                path: PathBuf::from("/d/Music.mp3"),
                reason: IgnoreReason::NotGameData("mp3".to_owned()),
            },
            IgnoredEntry {
                path: PathBuf::from("/d/.DS_Store"),
                reason: IgnoreReason::Hidden,
            },
        ];
        let mut outcome = outcome(Vec::new());
        outcome.warnings = vec![&warning];
        outcome.ignored = &ignored;
        outcome.not_exported = vec![(ResType::new(*b"XYZW"), 2), (ResType::new(*b"vers"), 1)];
        let quiet = text(&outcome, false);
        assert!(
            quiet.contains("\nnot exported: XYZW (2), vers (1)\n"),
            "{quiet}"
        );
        assert!(quiet.contains("\nignored: 2\nwarnings: 1\n"), "{quiet}");
        assert!(
            quiet.contains(&format!("\n  {warning}\nfailures: 0\n")),
            "{quiet}"
        );
        assert_eq!(exit_code(&outcome), 0, "warnings are not failures");

        let verbose = text(&outcome, true);
        assert!(
            verbose.contains(
                "\nignored: 2\n  /d/Music.mp3: not game data (.mp3)\n  /d/.DS_Store: hidden\n"
            ),
            "{verbose}"
        );
    }

    #[test]
    fn every_ignore_reason_reads_as_words() {
        let cases = [
            (IgnoreReason::Hidden, "hidden"),
            (IgnoreReason::Symlink, "symbolic link"),
            (IgnoreReason::NotAFile, "not a file or folder"),
            (
                IgnoreReason::DataSubFolder,
                "folder inside the data directory",
            ),
            (IgnoreReason::TooDeep, "plug-ins folder nested too deep"),
            (
                IgnoreReason::NotGameData("mov".to_owned()),
                "not game data (.mov)",
            ),
        ];
        for (reason, words) in cases {
            assert_eq!(ignore_reason(&reason), words);
        }
    }

    #[test]
    fn a_record_failure_names_the_resource_its_file_and_the_cause_once() {
        let error = short_ship();
        let data = [0xAB; 40];
        let failures = vec![Failure::Record {
            error: &error,
            source: Path::new("/p/Bad Ship"),
            data: &data,
        }];
        let outcome = outcome(failures);
        let quiet = text(&outcome, false);
        assert!(
            quiet.ends_with(&format!(
                "warnings: 0\nFailures:\n{SHIP_LINE}\nfailures: 1\n"
            )),
            "{quiet}"
        );
        assert_eq!(quiet.matches("Shuttle").count(), 1, "{quiet}");
        assert!(!quiet.contains("field:"), "{quiet}");
        assert!(!quiet.contains("offset:"), "{quiet}");
        assert!(!quiet.contains("0x0020:"), "{quiet}");
        assert_eq!(exit_code(&outcome), 1);
    }

    #[test]
    fn a_verbose_record_failure_adds_the_field_offset_and_bytes() {
        let error = short_ship();
        let data: Vec<u8> = (0..40).collect();
        let failures = vec![Failure::Record {
            error: &error,
            source: Path::new("/p/Bad Ship"),
            data: &data,
        }];
        let verbose = text(&outcome(failures), true);
        let hex = indented(&crate::hex::window(&data, 0x22));
        assert!(
            verbose.ends_with(&format!(
                "{SHIP_LINE}\n    field: Ship → weapons[2]\n    offset: 0x22\n{hex}failures: 1\n"
            )),
            "{verbose}"
        );
        assert!(hex.contains("[22]"), "{hex}");
    }

    #[test]
    fn a_record_failure_without_a_path_or_fixed_size_says_less() {
        let mut error = short_ship();
        error.name = None;
        error.expected_len = None;
        error.field.path = FieldPath::default();
        error.field.cause = Cause::Invalid("bad string".to_owned());
        let data = [0; 40];
        let failures = vec![Failure::Record {
            error: &error,
            source: Path::new("/d/N"),
            data: &data,
        }];
        let verbose = text(&outcome(failures), true);
        assert!(
            verbose.contains(
                "\n  shïp 128 in /d/N: bad string at byte 0x22 (record is 40 bytes)\n    offset: 0x22\n"
            ),
            "{verbose}"
        );
        assert!(!verbose.contains("field:"), "{verbose}");
    }

    #[test]
    fn a_media_failure_shows_its_error_and_in_verbose_its_bytes() {
        let data: Vec<u8> = (0..20).collect();
        let failures = vec![
            Failure::Resource {
                res_type: RLED,
                id: 1000,
                name: Some("Shuttle"),
                source: Path::new("/d/Ships"),
                data: &data,
                error: MediaError::Graphics(GraphicsError::UnsupportedToken {
                    token: 9,
                    offset: 0x11,
                }),
            },
            Failure::Resource {
                res_type: ResType::new(*b"snd "),
                id: 500,
                name: None,
                source: Path::new("/d/Sounds"),
                data: &data,
                error: MediaError::Sound(SoundError::UnsupportedFormat { format: 3 }),
            },
        ];
        let outcome = outcome(failures);
        let rled_line =
            "  rlëD 1000 \"Shuttle\" in /d/Ships: unsupported rlëD token 9 at byte 0x11\n";
        let snd_line = "  snd  500 in /d/Sounds: unsupported snd format 3\n";
        assert!(
            text(&outcome, false)
                .ends_with(&format!("Failures:\n{rled_line}{snd_line}failures: 2\n")),
            "{}",
            text(&outcome, false)
        );
        let hex = indented(&crate::hex::window(&data, 0x11));
        let verbose = text(&outcome, true);
        assert!(
            verbose.ends_with(&format!(
                "Failures:\n{rled_line}    offset: 0x11\n{hex}{snd_line}    data: 20 bytes\nfailures: 2\n"
            )),
            "{verbose}"
        );
        assert_eq!(exit_code(&outcome), 1);
    }

    #[test]
    fn a_file_failure_prints_its_cause_exactly_once() {
        let parse = ParseError::SectionOutOfBounds {
            section: Section::Map,
            offset: 300,
            len: 50,
            file_len: 20,
        };
        let path = PathBuf::from("/p/Broken");
        let failed = [
            FailedFile {
                path: path.clone(),
                origin: Origin::PlugIn,
                error: LoadError::Parse {
                    path: path.clone(),
                    source: parse.clone(),
                },
            },
            FailedFile {
                path: path.clone(),
                origin: Origin::PlugIn,
                error: LoadError::Io {
                    path: path.clone(),
                    source: io::Error::other("permission denied"),
                },
            },
            FailedFile {
                path: path.clone(),
                origin: Origin::PlugIn,
                error: LoadError::NoResourceFork { path: path.clone() },
            },
        ];
        let outcome = outcome(failed.iter().map(Failure::File).collect());
        let text = text(&outcome, true);
        let expected = format!(
            "Failures:\n  file /p/Broken: not a valid resource fork: {parse}\n  \
             file /p/Broken: read error: permission denied\n  \
             file /p/Broken: no resource fork\nfailures: 3\n"
        );
        assert!(text.ends_with(&expected), "{text}");
        assert_eq!(text.matches(&parse.to_string()).count(), 1);
        assert_eq!(text.matches("/p/Broken").count(), 3);
    }
}
