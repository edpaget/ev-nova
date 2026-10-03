//! `nova-dump` over the stock EV Nova data, end to end.
//!
//! The game data is copyrighted and never committed, so this test gets its
//! location from `common`, the only place `NOVA_DATA` (the `Nova Files`
//! directory) is read, and skips, passing, when it is unset. It runs the
//! dump with the real adapters into a fresh temporary directory, then
//! checks the files against the store's own view of the data, collecting
//! every problem before asserting.

mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use nova_data::graphics::RLED;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::sound::decode_snd;
use nova_data::store::GameData;
use nova_data::{Record, ShipId, TYPES};
use nova_dump::app::run;
use nova_dump::fs::{StdDataSource, StdOutputRoot};
use nova_dump::names::{file_stem, type_component};
use nova_rsrc::ResType;
use tempfile::TempDir;

use common::nova_data;

const PICT: ResType = ResType::new(*b"PICT");
const CICN: ResType = ResType::new(*b"cicn");
const PPAT: ResType = ResType::new(*b"ppat");
const SND: ResType = ResType::new(*b"snd ");

/// The file names in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// The exported file for (`ty`, `id`) under `dir`, named as the dump names
/// it.
fn media_path(data: &GameData, dir: &Path, ty: ResType, id: i16, ext: &str) -> PathBuf {
    let name = data.resource(ty, id).expect("present").resource.name();
    dir.join(format!("{}.{ext}", file_stem(id, name)))
}

/// (width, height) of a PNG file, decoding every pixel.
fn png_size(path: &Path) -> Result<(u32, u32), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let size = reader.output_buffer_size().ok_or("no buffer size")?;
    let mut pixels = vec![0; size];
    let info = reader
        .next_frame(&mut pixels)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((info.width, info.height))
}

fn u32_le(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

/// Runs the dump into `out` with the real adapters; panics unless it exits
/// 0 with `failures: 0` last.
fn dump(dir: &Path, out: &Path) {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let args: Vec<OsString> = vec![dir.into(), out.into()];
    let code = run(
        args,
        &StdDataSource,
        &StdOutputRoot,
        &mut stdout,
        &mut stderr,
    );
    let stdout = String::from_utf8(stdout).expect("UTF-8");
    assert_eq!(code, 0, "{stdout}{}", String::from_utf8_lossy(&stderr));
    assert_eq!(stdout.lines().last(), Some("failures: 0"), "{stdout}");
}

/// The picture, sheet and sound folders, their types and stock counts.
fn media(out: &Path) -> [(PathBuf, ResType, usize); 5] {
    let png = out.join("png");
    [
        (png.join(type_component(PICT)), PICT, 671),
        (png.join(type_component(CICN)), CICN, 29),
        (png.join(type_component(PPAT)), PPAT, 10),
        (png.join(type_component(RLED)), RLED, 282),
        (out.join("wav"), SND, 227),
    ]
}

/// One file per picture, sheet and sound, matching the store's counts, and
/// every PNG decodes.
fn check_media(data: &GameData, out: &Path, problems: &mut Vec<String>) {
    for (folder, ty, stock) in media(out) {
        let files = names(&folder);
        if files.len() != stock || files.len() != data.ids(ty).len() {
            problems.push(format!(
                "{ty}: {} files, {stock} in the survey, {} in the store",
                files.len(),
                data.ids(ty).len()
            ));
        }
        if ty == SND {
            continue;
        }
        for name in files {
            if let Err(e) = png_size(&folder.join(&name)) {
                problems.push(format!("{ty} {name}: {e}"));
            }
        }
    }
}

/// One JSON file per registered type present (25 of the 27: the stock data
/// has no `STR ` or `vers`), each holding every record of its type.
fn check_json(data: &GameData, out: &Path, problems: &mut Vec<String>) {
    let present: Vec<ResType> = TYPES
        .iter()
        .copied()
        .filter(|&ty| !data.ids(ty).is_empty())
        .collect();
    assert_eq!(present.len(), 25);
    let mut expected: Vec<String> = present
        .iter()
        .map(|&ty| format!("{}.json", type_component(ty)))
        .collect();
    expected.sort();
    assert_eq!(names(&out.join("json")), expected);
    let mut records = 0;
    for &ty in &present {
        let path = out
            .join("json")
            .join(format!("{}.json", type_component(ty)));
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).expect("written")).expect("JSON");
        let len = json.as_array().map_or(0, Vec::len);
        records += len;
        if len != data.ids(ty).len() {
            problems.push(format!("{ty}: {len} JSON records, {}", data.ids(ty).len()));
        }
    }
    // No stock file overrides another's registered records, so the union
    // count equals the per-file total.
    assert_eq!(records, 7143);
}

/// Each ship's base sheet is composed as the store resolves it.
fn check_ship_sheets(data: &GameData, out: &Path, problems: &mut Vec<String>) {
    let rled_dir = out.join("png").join(type_component(RLED));
    for &id in data.ids(Ship::TYPE) {
        let sheet = data
            .ship_sprite(ShipId(id))
            .expect("resolves")
            .sheet
            .compose();
        let anim = data.get::<ShipAnim>(id).expect("present").expect("decodes");
        let path = media_path(data, &rled_dir, RLED, anim.record.base_image_id, "png");
        let expected = (sheet.width(), sheet.height());
        match png_size(&path) {
            Ok(size) if size == expected => {}
            other => problems.push(format!("shïp {id}: {other:?}, expected {expected:?}")),
        }
    }
}

/// Every WAV has the sound's rate and all its samples.
fn check_wavs(data: &GameData, out: &Path, problems: &mut Vec<String>) {
    let wav_dir = out.join("wav");
    for &id in data.ids(SND) {
        let res = data.resource(SND, id).expect("present").resource;
        let pcm = decode_snd(res.data()).expect("stock sounds decode");
        let wav = std::fs::read(media_path(data, &wav_dir, SND, id, "wav")).expect("written");
        let rate = u32_le(&wav, 24);
        let data_len = u32_le(&wav, 40) as usize;
        if rate != pcm.sample_rate().nearest_hz()
            || data_len != pcm.samples().len() * 2
            || wav.len() != 44 + data_len
        {
            problems.push(format!("snd {id}: rate {rate}, {data_len} data bytes"));
        }
    }
}

#[test]
fn the_stock_data_dumps_without_failures() {
    let Some(dir) = nova_data() else { return };
    let tmp = TempDir::new().expect("temp dir");
    let out = tmp.path().join("out");
    dump(&dir, &out);

    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut problems = Vec::new();
    check_media(&data, &out, &mut problems);
    check_json(&data, &out, &mut problems);
    check_ship_sheets(&data, &out, &mut problems);
    check_wavs(&data, &out, &mut problems);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
