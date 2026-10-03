//! Walks the store and writes every record, picture, sprite sheet and
//! sound through a [`Sink`].
//!
//! The walk is deterministic: resource types in byte order, IDs ascending.
//! Decode failures never stop it; each becomes a [`Failure`]. A write
//! failure stops it, because the output can no longer be trusted.

use std::io;
use std::path::{Path, PathBuf};

use nova_data::graphics::{
    GraphicsError, RLED, SheetLayout, decode_cicn, decode_pict, decode_ppat, decode_rled,
};
use nova_data::sound::{SoundError, decode_snd};
use nova_data::store::{FailedFile, GameData, IgnoredEntry};
use nova_data::{DecodeError, DecodeWarning, TYPES};
use nova_rsrc::ResType;
use serde::Serialize;

use crate::layouts::sheet_layouts;
use crate::names::{file_stem, type_component};
use crate::ports::Sink;
use crate::wav::WavError;

/// How many of each kind of file were written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// JSON files, one per registered record type present.
    pub json_files: usize,
    /// Records written to the JSON files.
    pub records: usize,
    /// `PICT` PNGs.
    pub pict: usize,
    /// `cicn` PNGs.
    pub cicn: usize,
    /// `ppat` PNGs.
    pub ppat: usize,
    /// `rlëD` sprite-sheet PNGs.
    pub rled: usize,
    /// `snd ` WAVs.
    pub snd: usize,
}

/// What a run exported and what went wrong.
#[derive(Debug)]
pub struct Outcome<'a> {
    /// Files written, by kind.
    pub counts: Counts,
    /// Warnings raised by records that decoded.
    pub warnings: Vec<&'a DecodeWarning>,
    /// Everything that could not be exported, in walk order, then the files
    /// that did not load.
    pub failures: Vec<Failure<'a>>,
    /// Types present but neither registered nor a picture, sheet or sound,
    /// with how many resources each has.
    pub not_exported: Vec<(ResType, usize)>,
    /// Directory entries the store skipped on purpose.
    pub ignored: &'a [IgnoredEntry],
}

/// One thing that could not be exported.
#[derive(Debug)]
pub enum Failure<'a> {
    /// A record that failed to decode.
    Record {
        /// The decode error.
        error: &'a DecodeError,
        /// The file the record came from.
        source: &'a Path,
        /// The record's bytes.
        data: &'a [u8],
    },
    /// A picture, sprite sheet or sound that failed to decode or encode.
    Resource {
        /// The resource's type.
        res_type: ResType,
        /// The resource's ID.
        id: i16,
        /// The resource's name, if it has one.
        name: Option<&'a str>,
        /// The file it came from.
        source: &'a Path,
        /// Its bytes.
        data: &'a [u8],
        /// What went wrong.
        error: MediaError,
    },
    /// A file the store could not load.
    File(&'a FailedFile),
}

/// Why a picture, sprite sheet or sound could not be exported.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// The picture or sheet did not decode.
    #[error(transparent)]
    Graphics(#[from] GraphicsError),
    /// The sound did not decode.
    #[error(transparent)]
    Sound(#[from] SoundError),
    /// The image could not be written as PNG.
    #[error("PNG: {0}")]
    Png(#[from] crate::png::EncodingError),
    /// The sound is too long, or its rate too low, for WAV.
    #[error(transparent)]
    Wav(#[from] WavError),
}

impl MediaError {
    /// The byte offset in the resource where decoding failed, if known.
    #[must_use]
    pub fn offset(&self) -> Option<usize> {
        match self {
            Self::Graphics(error) => error.offset(),
            Self::Sound(error) => error.offset(),
            Self::Png(_) | Self::Wav(_) => None,
        }
    }
}

/// Writing an exported file failed.
#[derive(Debug, thiserror::Error)]
#[error("writing {}: {source}", path.display())]
pub struct WriteError {
    /// The file, relative to the output directory.
    pub path: PathBuf,
    /// The underlying error.
    pub source: io::Error,
}

/// The picture, sheet and sound types, and how each is exported.
#[derive(Clone, Copy)]
enum Media {
    Pict,
    Cicn,
    Ppat,
    Rled,
    Snd,
}

impl Media {
    fn of(ty: ResType) -> Option<Self> {
        match &ty.bytes() {
            b"PICT" => Some(Self::Pict),
            b"cicn" => Some(Self::Cicn),
            b"ppat" => Some(Self::Ppat),
            b"snd " => Some(Self::Snd),
            _ if ty == RLED => Some(Self::Rled),
            _ => None,
        }
    }

    /// The file for resource `id` named `name` of type `ty`.
    fn path(self, ty: ResType, id: i16, name: Option<&str>) -> PathBuf {
        let stem = file_stem(id, name);
        match self {
            Self::Snd => Path::new("wav").join(format!("{stem}.wav")),
            _ => Path::new("png")
                .join(type_component(ty))
                .join(format!("{stem}.png")),
        }
    }

    /// The exported file's bytes.
    fn encode(self, data: &[u8], layout: Option<SheetLayout>) -> Result<Vec<u8>, MediaError> {
        let image = match self {
            Self::Pict => decode_pict(data)?,
            Self::Cicn => decode_cicn(data)?,
            Self::Ppat => decode_ppat(data)?,
            Self::Rled => decode_rled(data, layout)?.compose(),
            Self::Snd => return Ok(crate::wav::encode(&decode_snd(data)?)?),
        };
        Ok(crate::png::encode(&image)?)
    }

    fn count(self, counts: &mut Counts) -> &mut usize {
        match self {
            Self::Pict => &mut counts.pict,
            Self::Cicn => &mut counts.cicn,
            Self::Ppat => &mut counts.ppat,
            Self::Rled => &mut counts.rled,
            Self::Snd => &mut counts.snd,
        }
    }
}

/// One element of a `json/<type>.json` array.
#[derive(Serialize)]
struct JsonEntry<'a> {
    id: i16,
    name: Option<&'a str>,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    record: serde_json::Value,
}

/// Exports everything in `data` through `sink`.
pub fn export<'a>(data: &'a GameData, sink: &mut impl Sink) -> Result<Outcome<'a>, WriteError> {
    let mut outcome = Outcome {
        counts: Counts::default(),
        warnings: Vec::new(),
        failures: Vec::new(),
        not_exported: Vec::new(),
        ignored: data.ignored(),
    };
    let layouts = sheet_layouts(data);
    for ty in data.types() {
        if TYPES.contains(&ty) {
            export_records(data, ty, sink, &mut outcome)?;
        } else if let Some(media) = Media::of(ty) {
            for &id in data.ids(ty) {
                let Some(found) = data.resource(ty, id) else {
                    continue;
                };
                let res = found.resource;
                let name = res.name();
                match media.encode(res.data(), layouts.get(&id).copied()) {
                    Ok(bytes) => {
                        write(sink, &media.path(ty, id, name), &bytes)?;
                        *media.count(&mut outcome.counts) += 1;
                    }
                    Err(error) => outcome.failures.push(Failure::Resource {
                        res_type: ty,
                        id,
                        name,
                        source: &found.source.path,
                        data: res.data(),
                        error,
                    }),
                }
            }
        } else {
            outcome.not_exported.push((ty, data.ids(ty).len()));
        }
    }
    outcome
        .failures
        .extend(data.failed().iter().map(Failure::File));
    Ok(outcome)
}

/// Writes `json/<ty>.json`: every record of `ty` that decodes, by ID.
fn export_records<'a>(
    data: &'a GameData,
    ty: ResType,
    sink: &mut impl Sink,
    outcome: &mut Outcome<'a>,
) -> Result<(), WriteError> {
    let mut entries = Vec::new();
    for &id in data.ids(ty) {
        match data.get_any(ty, id) {
            Some(Ok(entry)) => {
                if let Some(warning) = entry.warning {
                    outcome.warnings.push(warning);
                }
                entries.push(JsonEntry {
                    id,
                    name: entry.name,
                    source: entry.source.path.display().to_string(),
                    warning: entry.warning.map(ToString::to_string),
                    record: entry.record.to_json(),
                });
            }
            Some(Err(error)) => {
                if let Some(found) = data.resource(ty, id) {
                    outcome.failures.push(Failure::Record {
                        error,
                        source: &found.source.path,
                        data: found.resource.data(),
                    });
                }
            }
            None => {}
        }
    }
    outcome.counts.records += entries.len();
    outcome.counts.json_files += 1;
    let mut json = serde_json::to_vec_pretty(&entries).expect("records serialize");
    json.push(b'\n');
    write(
        sink,
        &Path::new("json").join(format!("{}.json", type_component(ty))),
        &json,
    )
}

fn write(sink: &mut impl Sink, path: &Path, bytes: &[u8]) -> Result<(), WriteError> {
    sink.write(path, bytes).map_err(|source| WriteError {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::fixture::{Cicn, Ctab, PictBuilder, Ppat, RledBuilder};
    use nova_data::graphics::{RLED, decode_cicn, decode_pict, decode_ppat, decode_rled};
    use nova_data::records::ship_anim::ShipAnim;
    use nova_data::records::spin::Spin;
    use nova_data::sound::decode_snd;
    use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
    use nova_data::{AnyRecord, Record};
    use nova_rsrc::ResType;
    use serde_json::{Value, json};

    use super::*;
    use crate::names::type_component;
    use crate::testutil::{MemFs, MemSink, fork, record, store};

    const PICT: ResType = ResType::new(*b"PICT");
    const CICN: ResType = ResType::new(*b"cicn");
    const PPAT: ResType = ResType::new(*b"ppat");
    const SND: ResType = ResType::new(*b"snd ");

    type Res = (ResType, i16, Option<&'static str>, Vec<u8>);

    /// A resource failure's type, ID, name, message, offset and data length.
    type FailedRow<'a> = (ResType, i16, Option<&'a str>, String, Option<usize>, usize);

    fn pict() -> Vec<u8> {
        PictBuilder::new([0, 0, 2, 3]).end().build()
    }

    fn ctab() -> Ctab {
        Ctab {
            device: false,
            entries: vec![(0, [0xFFFF, 0, 0]), (1, [0, 0, 0xFFFF])],
        }
    }

    fn cicn() -> Vec<u8> {
        Cicn::new(2, 1, 8, &ctab(), &[0, 1], &[true, false]).bytes()
    }

    fn ppat() -> Vec<u8> {
        Ppat::new(2, 2, 8, &ctab(), &[0, 1, 1, 0]).bytes()
    }

    /// Four 1x1 frames.
    fn rled() -> Vec<u8> {
        let mut sheet = RledBuilder::new(1, 1);
        for colour in [0x7C00, 0x03E0, 0x001F, 0x7FFF] {
            sheet = sheet.frame(|f| f.line().pixels(&[colour]));
        }
        sheet.build()
    }

    fn snd() -> Vec<u8> {
        SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80, 0xFF, 0x00],
            },
        )
        .bytes()
    }

    fn spin(sprites: i16) -> Vec<u8> {
        record(12, &[(0, sprites), (4, 1), (6, 1), (8, 2), (10, 2)])
    }

    /// A `shän` whose base image is `rlëD` 1000, two frames per row.
    fn shan() -> Vec<u8> {
        record(192, &[(0x00, 1000), (0x34, 2)])
    }

    fn everything() -> Vec<Res> {
        vec![
            (PICT, 128, Some("Land/scape"), pict()),
            (CICN, 200, None, cicn()),
            (PPAT, 300, Some("Tile"), ppat()),
            (RLED, 1000, Some("Shuttle"), rled()),
            (RLED, 1001, None, rled()),
            (SND, 500, Some("zap"), snd()),
            (ShipAnim::TYPE, 128, Some("Shuttle"), shan()),
            (Spin::TYPE, 200, None, spin(1001)),
            (Spin::TYPE, 201, Some("Rock"), spin(5)),
        ]
    }

    fn png_path(ty: ResType, stem: &str) -> String {
        format!("png/{}/{stem}.png", type_component(ty))
    }

    fn json_path(ty: ResType) -> String {
        format!("json/{}.json", type_component(ty))
    }

    fn json(sink: &MemSink, ty: ResType) -> Value {
        serde_json::from_slice(sink.get(&json_path(ty))).expect("JSON")
    }

    fn png_of(image: &nova_data::graphics::Image) -> Vec<u8> {
        crate::png::encode(image).expect("encodes")
    }

    #[test]
    fn every_kind_is_written_at_its_path() {
        let data = store(&everything());
        let mut sink = MemSink::default();
        let outcome = export(&data, &mut sink).expect("writes");

        let mut expected = vec![
            png_path(PICT, "128 Land_scape"),
            png_path(CICN, "200"),
            png_path(PPAT, "300 Tile"),
            png_path(RLED, "1000 Shuttle"),
            png_path(RLED, "1001"),
            "wav/500 zap.wav".to_owned(),
            json_path(ShipAnim::TYPE),
            json_path(Spin::TYPE),
        ];
        expected.sort();
        assert_eq!(sink.paths(), expected);
        assert_eq!(
            outcome.counts,
            Counts {
                json_files: 2,
                records: 3,
                pict: 1,
                cicn: 1,
                ppat: 1,
                rled: 2,
                snd: 1,
            }
        );
        assert!(outcome.failures.is_empty());
        assert!(outcome.warnings.is_empty());
        assert!(outcome.not_exported.is_empty());
    }

    #[test]
    fn pictures_and_sounds_hold_their_decoded_content() {
        let data = store(&everything());
        let mut sink = MemSink::default();
        export(&data, &mut sink).expect("writes");

        let pict = decode_pict(&pict()).expect("decodes");
        assert_eq!(sink.get(&png_path(PICT, "128 Land_scape")), png_of(&pict));
        let cicn = decode_cicn(&cicn()).expect("decodes");
        assert_eq!(sink.get(&png_path(CICN, "200")), png_of(&cicn));
        let ppat = decode_ppat(&ppat()).expect("decodes");
        assert_eq!(sink.get(&png_path(PPAT, "300 Tile")), png_of(&ppat));
        let pcm = decode_snd(&snd()).expect("decodes");
        let wav = crate::wav::encode(&pcm).expect("small");
        assert_eq!(sink.get("wav/500 zap.wav"), wav);
    }

    #[test]
    fn sheets_are_composed_with_the_layout_of_the_record_using_them() {
        let data = store(&everything());
        let mut sink = MemSink::default();
        export(&data, &mut sink).expect("writes");

        // rlëD 1000: the shän's 2 columns; rlëD 1001: the spïn's 2 columns.
        let two_columns = decode_rled(&rled(), ShipAnim::sheet_layout(&shan_record()))
            .expect("decodes")
            .compose();
        assert_eq!((two_columns.width(), two_columns.height()), (2, 2));
        assert_eq!(
            sink.get(&png_path(RLED, "1000 Shuttle")),
            png_of(&two_columns)
        );
        assert_eq!(sink.get(&png_path(RLED, "1001")), png_of(&two_columns));

        // Unclaimed: the default layout.
        let data = store(&[(RLED, 1000, None, rled())]);
        let mut sink = MemSink::default();
        export(&data, &mut sink).expect("writes");
        let default = decode_rled(&rled(), None).expect("decodes").compose();
        assert_eq!((default.width(), default.height()), (4, 1));
        assert_eq!(sink.get(&png_path(RLED, "1000")), png_of(&default));
    }

    fn shan_record() -> ShipAnim {
        nova_data::decode_bytes::<ShipAnim>(&shan())
            .expect("decodes")
            .record
    }

    #[test]
    fn each_record_type_is_one_json_array_by_id() {
        let mut padded = spin(7);
        padded.extend([0, 0]);
        let data = store(&[
            (Spin::TYPE, 201, Some("Rock"), spin(5)),
            (Spin::TYPE, 200, None, padded.clone()),
        ]);
        let mut sink = MemSink::default();
        let outcome = export(&data, &mut sink).expect("writes");

        let any = |bytes: &[u8]| {
            let spin = nova_data::decode_bytes::<Spin>(bytes)
                .expect("decodes")
                .record;
            let value = serde_json::to_value(AnyRecord::Spin(Box::new(spin))).expect("JSON");
            value["record"].clone()
        };
        let warning = outcome.warnings[0].to_string();
        assert_eq!(
            json(&sink, Spin::TYPE),
            json!([
                {
                    "id": 200,
                    "name": null,
                    "source": "/data/Nova Data",
                    "warning": warning,
                    "record": any(&padded),
                },
                {
                    "id": 201,
                    "name": "Rock",
                    "source": "/data/Nova Data",
                    "record": any(&spin(5)),
                },
            ])
        );
        assert_eq!(outcome.warnings.len(), 1);
        assert!(warning.contains("2 trailing bytes"), "{warning}");
        let text = String::from_utf8(sink.get(&json_path(Spin::TYPE)).to_vec()).expect("UTF-8");
        assert!(text.ends_with("]\n"), "{text}");
        assert!(text.contains("\n  {\n"), "pretty-printed: {text}");
    }

    #[test]
    fn a_record_that_fails_is_left_out_and_reported() {
        let data = store(&[
            (Spin::TYPE, 200, Some("Short"), vec![0; 10]),
            (Spin::TYPE, 201, None, spin(5)),
            (ShipAnim::TYPE, 128, None, vec![0; 3]),
        ]);
        let mut sink = MemSink::default();
        let outcome = export(&data, &mut sink).expect("writes");

        let ids: Vec<Value> = json(&sink, Spin::TYPE)
            .as_array()
            .expect("array")
            .iter()
            .map(|entry| entry["id"].clone())
            .collect();
        assert_eq!(ids, [json!(201)]);
        assert_eq!(json(&sink, ShipAnim::TYPE), json!([]));
        assert_eq!(outcome.counts.json_files, 2);
        assert_eq!(outcome.counts.records, 1);

        let [first, second] = outcome.failures.as_slice() else {
            panic!("{:?}", outcome.failures)
        };
        let Failure::Record {
            error,
            source,
            data,
        } = first
        else {
            panic!("{first:?}")
        };
        assert_eq!((error.res_type, error.id), (ShipAnim::TYPE, 128));
        assert_eq!(*source, Path::new("/data/Nova Data"));
        assert_eq!(*data, [0; 3]);
        let Failure::Record { error, data, .. } = second else {
            panic!("{second:?}")
        };
        assert_eq!((error.res_type, error.id), (Spin::TYPE, 200));
        assert_eq!(error.name.as_deref(), Some("Short"));
        assert_eq!(data.len(), 10);
    }

    #[test]
    fn a_picture_sheet_or_sound_that_fails_is_reported_and_the_run_goes_on() {
        let mut bad_snd = snd();
        bad_snd.pop();
        let data = store(&[
            (PICT, 128, Some("Bad"), vec![0; 4]),
            (PICT, 129, None, pict()),
            (CICN, 200, None, vec![1, 2]),
            (PPAT, 300, None, vec![]),
            (RLED, 1000, None, RledBuilder::new(1, 1).depth(8).build()),
            (SND, 500, Some("cut"), bad_snd.clone()),
            (SND, 501, None, snd()),
        ]);
        let mut sink = MemSink::default();
        let outcome = export(&data, &mut sink).expect("writes");

        assert_eq!(
            sink.paths(),
            [png_path(PICT, "129"), "wav/501.wav".to_owned()]
        );
        let failed: Vec<FailedRow> = outcome
            .failures
            .iter()
            .map(|failure| match failure {
                Failure::Resource {
                    res_type,
                    id,
                    name,
                    source,
                    data,
                    error,
                } => {
                    assert_eq!(*source, Path::new("/data/Nova Data"));
                    (
                        *res_type,
                        *id,
                        *name,
                        error.to_string(),
                        error.offset(),
                        data.len(),
                    )
                }
                other => panic!("{other:?}"),
            })
            .collect();
        let pict_error = decode_pict(&[0; 4]).expect_err("short");
        let snd_error = decode_snd(&bad_snd).expect_err("short");
        assert_eq!(
            failed,
            [
                (
                    PICT,
                    128,
                    Some("Bad"),
                    pict_error.to_string(),
                    pict_error.offset(),
                    4
                ),
                (
                    CICN,
                    200,
                    None,
                    decode_cicn(&[1, 2]).expect_err("short").to_string(),
                    Some(0),
                    2
                ),
                (
                    PPAT,
                    300,
                    None,
                    decode_ppat(&[]).expect_err("empty").to_string(),
                    Some(0),
                    0
                ),
                (
                    RLED,
                    1000,
                    None,
                    "unsupported rlëD depth 8".to_owned(),
                    None,
                    16
                ),
                (
                    SND,
                    500,
                    Some("cut"),
                    snd_error.to_string(),
                    snd_error.offset(),
                    bad_snd.len()
                ),
            ]
        );
        assert_eq!((outcome.counts.pict, outcome.counts.snd), (1, 1));
        assert_eq!(
            (
                outcome.counts.cicn,
                outcome.counts.ppat,
                outcome.counts.rled
            ),
            (0, 0, 0)
        );
    }

    #[test]
    fn a_write_error_aborts_the_run() {
        let data = store(&everything());
        for fail_at in [1, 5, 8] {
            let mut sink = MemSink {
                fail_at: Some(fail_at),
                ..MemSink::default()
            };
            let error = export(&data, &mut sink).expect_err("fails");
            assert_eq!(sink.writes, fail_at, "stops at the failing write");
            assert_eq!(sink.files.len(), fail_at - 1);
            assert_eq!(error.source.to_string(), "disk full");
            assert!(!sink.files.contains_key(&error.path));
        }
        let mut sink = MemSink {
            fail_at: Some(1),
            ..MemSink::default()
        };
        let error = export(&data, &mut sink).expect_err("fails");
        assert_eq!(
            error.to_string(),
            format!("writing {}: disk full", png_path(PICT, "128 Land_scape"))
        );
    }

    #[test]
    fn a_plug_in_override_exports_the_winner_from_its_file() {
        let data = MemFs::new()
            .file(
                "/data/Nova Data",
                fork(&[
                    (Spin::TYPE, 200, Some("Old"), spin(1)),
                    (SND, 500, None, snd()),
                ]),
            )
            .file(
                "/plugins/Better Rocks",
                fork(&[(Spin::TYPE, 200, Some("New"), spin(2))]),
            )
            .open();
        let mut sink = MemSink::default();
        export(&data, &mut sink).expect("writes");
        let spins = json(&sink, Spin::TYPE);
        assert_eq!(spins[0]["name"], "New");
        assert_eq!(spins[0]["source"], "/plugins/Better Rocks");
        assert_eq!(spins[0]["record"]["sprites_id"], 2);
        assert_eq!(spins.as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn files_that_did_not_load_and_unexported_types_are_reported() {
        let mut broken = fork(&[(Spin::TYPE, 1, None, spin(1))]);
        broken.truncate(20);
        let other = ResType::new(*b"XYZW");
        let data = MemFs::new()
            .file(
                "/data/Nova Data",
                fork(&[(other, 1, None, vec![]), (other, 2, None, vec![])]),
            )
            .file("/data/notes.txt", vec![])
            .file("/plugins/Broken", broken)
            .open();
        let mut sink = MemSink::default();
        let outcome = export(&data, &mut sink).expect("writes");

        assert!(sink.files.is_empty());
        assert_eq!(outcome.not_exported, [(other, 2)]);
        let [Failure::File(failed)] = outcome.failures.as_slice() else {
            panic!("{:?}", outcome.failures)
        };
        assert_eq!(failed.path, Path::new("/plugins/Broken"));
        assert_eq!(outcome.ignored.len(), 1);
        assert_eq!(outcome.ignored[0].path, Path::new("/data/notes.txt"));
    }
}
