//! Picture and sprite decoders against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so these tests read its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skip, passing,
//! when it is unset. Expected dimensions are read straight from each
//! resource's bytes here, independently of the decoders. Each test collects
//! every failure before asserting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use nova_data::decode_all;
use nova_data::graphics::{Image, decode_cicn, decode_pict, decode_ppat, decode_rled};
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_rsrc::{ResType, ResourceFile};

/// The program edge for these tests: this test binary's only read of `NOVA_DATA`.
fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// Every stock file, opened, with its name.
fn stock_files(dir: &Path) -> Vec<(String, ResourceFile)> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("NOVA_DATA is a readable directory")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ndat"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let file = ResourceFile::open(&path).expect("stock file opens");
            (name, file)
        })
        .collect()
}

fn code(text: &str) -> ResType {
    ResType::from_mac_roman(text).expect("four Mac Roman characters")
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

/// The width and height of the QuickDraw rectangle at `at`.
fn rect_size(data: &[u8], at: usize) -> (u32, u32) {
    let side = |i: usize| i32::from(u16_at(data, at + 2 * i) as i16);
    ((side(3) - side(1)) as u32, (side(2) - side(0)) as u32)
}

fn alphas(image: &Image) -> impl Iterator<Item = u8> + '_ {
    image.pixels().chunks(4).map(|p| p[3])
}

/// Calls `check` on every resource of type `ty`, collecting its problems,
/// and asserts there are none and that `count` resources were checked.
fn check_all(ty: &str, count: usize, mut check: impl FnMut(&[u8], &mut Vec<String>)) {
    let Some(dir) = nova_data() else { return };
    let mut problems = Vec::new();
    let mut checked = 0;
    for (name, file) in stock_files(&dir) {
        for res in file.resources(code(ty)) {
            let mut found = Vec::new();
            check(res.data(), &mut found);
            problems.extend(
                found
                    .into_iter()
                    .map(|p| format!("{name} {ty} {}: {p}", res.id())),
            );
            checked += 1;
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(checked, count, "{ty} count");
}

#[test]
fn every_stock_pict_decodes_opaque_at_its_frame_size() {
    check_all("PICT", 671, |data, problems| match decode_pict(data) {
        Err(e) => problems.push(e.to_string()),
        Ok(image) => {
            let size = (image.width(), image.height());
            if size != rect_size(data, 2) {
                problems.push(format!("size {size:?}, frame {:?}", rect_size(data, 2)));
            }
            if alphas(&image).any(|a| a != 255) {
                problems.push("not opaque".into());
            }
        }
    });
}

#[test]
fn every_stock_rled_decodes_its_frames_at_the_header_size() {
    check_all("rlëD", 282, |data, problems| {
        match decode_rled(data, None) {
            Err(e) => problems.push(e.to_string()),
            Ok(sheet) => {
                let (width, height) = (u32::from(u16_at(data, 0)), u32::from(u16_at(data, 2)));
                let count = usize::from(u16_at(data, 8));
                if sheet.frames().len() != count {
                    problems.push(format!("{} frames, header {count}", sheet.frames().len()));
                }
                if sheet
                    .frames()
                    .iter()
                    .any(|f| (f.width(), f.height()) != (width, height))
                {
                    problems.push(format!("a frame is not {width}x{height}"));
                }
            }
        }
    });
}

#[test]
fn every_stock_cicn_decodes_at_its_bounds_with_its_mask_as_alpha() {
    let mut masked = 0;
    check_all("cicn", 29, |data, problems| match decode_cicn(data) {
        Err(e) => problems.push(e.to_string()),
        Ok(image) => {
            // Bounds follow baseAddr and rowBytes.
            let size = (image.width(), image.height());
            if size != rect_size(data, 6) {
                problems.push(format!("size {size:?}, bounds {:?}", rect_size(data, 6)));
            }
            if alphas(&image).any(|a| a != 0 && a != 255) {
                problems.push("partial alpha".into());
            }
            if alphas(&image).any(|a| a == 0) && alphas(&image).any(|a| a == 255) {
                masked += 1;
            }
        }
    });
    if nova_data().is_some() {
        assert!(masked > 0, "some icon has transparent and opaque pixels");
    }
}

#[test]
fn every_stock_ppat_decodes_opaque_at_its_bounds() {
    check_all("ppat", 10, |data, problems| match decode_ppat(data) {
        Err(e) => problems.push(e.to_string()),
        Ok(image) => {
            let pat_map = u32::from_be_bytes(data[2..6].try_into().unwrap()) as usize;
            let bounds = rect_size(data, pat_map + 6);
            let size = (image.width(), image.height());
            if size != bounds {
                problems.push(format!("size {size:?}, bounds {bounds:?}"));
            }
            if alphas(&image).any(|a| a != 255) {
                problems.push("not opaque".into());
            }
        }
    });
}

/// Every stock sprite sheet arranged by the `spïn` or `shän` that uses it
/// fills exactly the grid that record describes. This lookup by ID is
/// test-only glue.
#[test]
fn stock_sheets_compose_to_their_spin_and_shan_grids() {
    let Some(dir) = nova_data() else { return };
    let files = stock_files(&dir);
    let sheets: HashMap<i16, Vec<u8>> = files
        .iter()
        .flat_map(|(_, file)| {
            file.resources(code("rlëD"))
                .map(|res| (res.id(), res.data().to_vec()))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut problems = Vec::new();

    let mut spins = 0;
    for (_, file) in &files {
        for entry in decode_all::<Spin>(file).entries {
            let spin = entry.record;
            let Some(data) = sheets.get(&spin.sprites_id) else {
                continue;
            };
            spins += 1;
            let sheet = decode_rled(data, spin.sheet_layout()).expect("decodes");
            // The grid is in cells; a cell is the sheet's own frame size,
            // which `x_size`/`y_size` do not always match (spïn 1033).
            let grid = (sheet.columns(), sheet.rows());
            let expected = (
                u32::from(spin.x_tiles as u16),
                u32::from(spin.y_tiles as u16),
            );
            let composed = sheet.compose();
            let size = (composed.width(), composed.height());
            let frame = (u32::from(u16_at(data, 0)), u32::from(u16_at(data, 2)));
            if grid != expected
                || (grid.0 * grid.1) as usize != sheet.frames().len()
                || size != (expected.0 * frame.0, expected.1 * frame.1)
            {
                problems.push(format!(
                    "spïn {}: grid {grid:?} for {} frames, composed {size:?}, expected {expected:?}",
                    entry.id,
                    sheet.frames().len(),
                ));
            }
        }
    }

    let mut shans = 0;
    for (_, file) in &files {
        for entry in decode_all::<ShipAnim>(file).entries {
            let shan = entry.record;
            shans += 1;
            let Some(data) = sheets.get(&shan.base_image_id) else {
                problems.push(format!("shän {}: base image is not an rlëD", entry.id));
                continue;
            };
            let sheet = decode_rled(data, shan.sheet_layout()).expect("decodes");
            let grid = (sheet.columns(), sheet.rows());
            let expected = (
                u32::from(shan.frames_per as u16),
                u32::from(shan.base_set_count as u16),
            );
            if grid != expected || sheet.frames().len() as u32 != expected.0 * expected.1 {
                problems.push(format!(
                    "shän {}: grid {grid:?} for {} frames, expected {expected:?}",
                    entry.id,
                    sheet.frames().len()
                ));
            }
        }
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(spins, 132, "spïns using an rlëD");
    assert_eq!(shans, 288, "shäns");
}
