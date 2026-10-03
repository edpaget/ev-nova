//! The sound decoder against the stock EV Nova data files.
//!
//! The game data is copyrighted and never committed, so this test reads its
//! location from `NOVA_DATA` (the `Nova Files` directory) and skips,
//! passing, when it is unset. Expected values are read straight from each
//! resource's bytes here, independently of the decoder, and every problem
//! is collected before asserting.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nova_data::sound::{Pcm, decode_snd};
use nova_rsrc::{ResType, ResourceFile};

const SND: ResType = ResType::new(*b"snd ");

/// The program edge for this test: the only place `NOVA_DATA` is read.
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

/// FNV-1a (64-bit) over `bytes`, continuing from `hash`.
fn fnv1a(hash: u64, bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(hash, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;

/// FNV-1a over `pcm`'s samples as little-endian bytes, from `hash`.
fn digest(hash: u64, pcm: &Pcm) -> u64 {
    fnv1a(hash, pcm.samples().iter().flat_map(|s| s.to_le_bytes()))
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(data[at..at + 4].try_into().expect("four bytes"))
}

/// What a resource's own bytes say it holds.
#[derive(Debug)]
struct Expected {
    format: u16,
    encode: u8,
    rate: u32,
    samples: usize,
}

/// Reads the format, the single command's header offset, the header's
/// `encode` and Fixed rate, and its sample count (`numBytes` for a
/// standard header, 64 per `numFrames` packet for a compressed one).
fn expected(data: &[u8]) -> Expected {
    let format = u16_at(data, 0);
    let commands = match format {
        1 => 4 + 6 * usize::from(u16_at(data, 2)),
        _ => 4,
    };
    assert_eq!(u16_at(data, commands), 1, "one command");
    let header = u32_at(data, commands + 2 + 4) as usize;
    let encode = data[header + 20];
    let samples = match encode {
        0x00 => u32_at(data, header + 4) as usize,
        _ => 64 * u32_at(data, header + 22) as usize,
    };
    Expected {
        format,
        encode,
        rate: u32_at(data, header + 8),
        samples,
    }
}

#[test]
fn every_stock_snd_decodes_at_its_header_rate_and_length() {
    let Some(dir) = nova_data() else { return };
    let mut problems = Vec::new();
    let mut formats = BTreeMap::new();
    let mut encodings = BTreeMap::new();
    let mut decoded: BTreeMap<i16, Pcm> = BTreeMap::new();
    let mut compressed = Vec::new();
    for (name, file) in stock_files(&dir) {
        for res in file.resources(SND) {
            let id = res.id();
            let want = expected(res.data());
            *formats.entry(want.format).or_insert(0) += 1;
            *encodings.entry(want.encode).or_insert(0) += 1;
            let pcm = match decode_snd(res.data()) {
                Ok(pcm) => pcm,
                Err(e) => {
                    problems.push(format!("{name} snd {id} (format {}): {e}", want.format));
                    continue;
                }
            };
            if pcm.sample_rate().fixed() != want.rate {
                problems.push(format!(
                    "{name} snd {id}: rate {:#010x}, header {:#010x}",
                    pcm.sample_rate().fixed(),
                    want.rate
                ));
            }
            if pcm.channels() != 1 {
                problems.push(format!("{name} snd {id}: {} channels", pcm.channels()));
            }
            if pcm.samples().len() != want.samples {
                problems.push(format!(
                    "{name} snd {id}: {} samples, header {}",
                    pcm.samples().len(),
                    want.samples
                ));
            }
            if want.encode == 0xFE {
                compressed.push(id);
            }
            decoded.insert(id, pcm);
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(decoded.len(), 227, "snd count");
    assert_eq!(formats, BTreeMap::from([(1, 213), (2, 14)]), "formats");
    assert_eq!(
        encodings,
        BTreeMap::from([(0x00, 49), (0xFE, 178)]),
        "header kinds"
    );

    // ion Cannon: format 1, IMA4, 396 packets.
    let ion = &decoded[&230];
    assert_eq!(ion.samples().len(), 396 * 64);
    assert_eq!(ion.sample_rate().fixed(), 0x5622_0000);
    // Etheric Wake: format 1, standard header, numBytes 6065.
    let wake = &decoded[&200];
    assert_eq!(wake.samples().len(), 6065);
    assert_eq!(wake.sample_rate().fixed(), 0x2B77_0000);
    // Beep1: format 2, standard header, loop points past its end.
    let beep = &decoded[&150];
    assert_eq!(beep.samples().len(), 872);
    assert_eq!(beep.sample_rate().fixed(), 0x2B77_45D1);
    assert_eq!(beep.loop_points(), Some((1742, 1743)));

    // Bit-exact with Apple's own IMA4 decoder: these digests are of
    // macOS `afconvert`'s 16-bit output for the same packets (wrapped in
    // an AIFC), every compressed sound in ascending ID order.
    assert_eq!(digest(FNV_OFFSET, ion), 0x5077_2641_7929_D17C, "snd 230");
    compressed.sort_unstable();
    let all = compressed
        .iter()
        .fold(FNV_OFFSET, |hash, id| digest(hash, &decoded[id]));
    assert_eq!(compressed.len(), 178);
    assert_eq!(all, 0xD363_444F_0E23_1CE5, "every IMA4 sound");
}
