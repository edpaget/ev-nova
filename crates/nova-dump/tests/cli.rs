//! Wiring: the `nova-dump` binary run on synthetic data files and plug-ins
//! written to a temporary directory.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use nova_data::Record;
use nova_data::graphics::RLED;
use nova_data::graphics::fixture::{PictBuilder, RledBuilder};
use nova_data::records::ship::Ship;
use nova_data::records::spin::Spin;
use nova_data::sound::fixture::{Header, SndBuilder, SndFormat};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;
use tempfile::TempDir;

const PICT: ResType = ResType::new(*b"PICT");
const SND: ResType = ResType::new(*b"snd ");

/// A temporary tree: `data/Nova Data` (a picture, a sheet, a sound and a
/// spïn) and, when asked for, `plugins/` holding a plug-in with a
/// truncated shïp and a plug-in whose resource map lies past its end.
struct Tree {
    dir: TempDir,
}

impl Tree {
    fn new(broken_plugins: bool) -> Self {
        let dir = TempDir::new().expect("temp dir");
        let data = dir.path().join("data");
        std::fs::create_dir(&data).expect("mkdir");
        let sheet = RledBuilder::new(1, 1)
            .frame(|f| f.line().pixels(&[0x7C00]))
            .build();
        let snd = SndBuilder::new(
            SndFormat::Two,
            Header::Standard {
                rate: 0x2B77_0000,
                loop_points: (0, 0),
                base_note: 60,
                samples: vec![0x80; 8],
            },
        );
        let spin = [0x03, 0xE8, 0xFF, 0xFF, 0, 1, 0, 1, 0, 1, 0, 1];
        let fork = ForkBuilder::new()
            .resource(
                PICT,
                128,
                Some(b"Landscape"),
                &PictBuilder::new([0, 0, 4, 4]).end().build(),
            )
            .resource(RLED, 1000, None, &sheet)
            .resource(SND, 200, Some(b"zap"), &snd.bytes())
            .resource(Spin::TYPE, 128, None, &spin)
            .build();
        std::fs::write(data.join("Nova Data"), fork.bytes).expect("write");

        if broken_plugins {
            let plugins = dir.path().join("plugins");
            std::fs::create_dir(&plugins).expect("mkdir");
            let bad_ship = ForkBuilder::new()
                .resource(Ship::TYPE, 128, Some(b"Shuttle"), &[0; 400])
                .build();
            std::fs::write(plugins.join("Bad Ship"), bad_ship.bytes).expect("write");
            let mut broken = ForkBuilder::new()
                .resource(Spin::TYPE, 129, None, &spin)
                .build();
            broken.truncate(20);
            std::fs::write(plugins.join("Broken"), broken.bytes).expect("write");
        }
        Self { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Runs `nova-dump data out [extra...]`.
    fn run(&self, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_nova-dump"))
            .arg(self.path("data"))
            .arg(self.path("out"))
            .args(extra)
            .output()
            .expect("nova-dump runs")
    }

    fn plugins_arg(&self) -> String {
        self.path("plugins").to_string_lossy().into_owned()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8")
}

/// Every file under `dir`, relative to it, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).expect("lists") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let rel = path.strip_prefix(dir).expect("under dir");
                found.push(rel.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_clean_run_exits_0_and_writes_every_kind() {
    let tree = Tree::new(false);
    let output = tree.run(&[]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(stdout(&output).ends_with("\nfailures: 0\n"), "{output:?}");
    assert_eq!(
        files(&tree.path("out")),
        [
            "json/spïn.json",
            "png/PICT/128 Landscape.png",
            "png/rlëD/1000.png",
            "wav/200 zap.wav",
        ]
    );
    let wav = std::fs::read(tree.path("out/wav/200 zap.wav")).expect("written");
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(wav.len(), 44 + 16);
}

#[test]
fn a_corrupt_record_and_an_unreadable_plug_in_exit_1_and_are_both_listed() {
    let tree = Tree::new(true);
    let output = tree.run(&["--plugins", &tree.plugins_arg()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    let failures: Vec<&str> = text
        .lines()
        .skip_while(|line| *line != "Failures:")
        .skip(1)
        .collect();
    let bad_ship = tree.path("plugins/Bad Ship");
    let broken = tree.path("plugins/Broken");
    assert_eq!(failures.len(), 3, "{text}");
    assert!(
        failures[0].starts_with(&format!(
            "  shïp 128 \"Shuttle\" in {}: unexpected end of data at byte 0x",
            bad_ship.display()
        )),
        "{text}"
    );
    assert!(
        failures[0].ends_with("(record is 400 bytes, layout needs 1860)"),
        "{text}"
    );
    assert!(
        failures[1].starts_with(&format!(
            "  file {}: not a valid resource fork: ",
            broken.display()
        )),
        "{text}"
    );
    assert_eq!(failures[2], "failures: 2");
    assert!(!text.contains("field:"), "{text}");
    // Everything else is still written.
    assert_eq!(files(&tree.path("out")).len(), 5);
}

#[test]
fn verbose_adds_the_field_path_and_a_hex_window() {
    let tree = Tree::new(true);
    let output = tree.run(&["--plugins", &tree.plugins_arg(), "-v"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = stdout(&output);
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("  shïp 128"))
        .expect("the ship failure");
    assert!(lines[at + 1].starts_with("    field: Ship → "), "{text}");
    assert!(lines[at + 2].starts_with("    offset: 0x"), "{text}");
    assert!(lines[at + 3].starts_with("    0x0"), "{text}");
    assert!(text.contains('['), "{text}");
}

#[test]
fn a_second_run_into_the_same_directory_is_refused() {
    let tree = Tree::new(false);
    assert_eq!(tree.run(&[]).status.code(), Some(0));
    let before = files(&tree.path("out"));
    let output = tree.run(&[]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert!(stderr.ends_with("is not empty\n"), "{stderr}");
    assert_eq!(files(&tree.path("out")), before);
}

#[test]
fn a_usage_error_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_nova-dump"))
        .arg("--bogus")
        .output()
        .expect("nova-dump runs");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("UTF-8");
    assert!(
        stderr.starts_with("nova-dump: unknown option \"--bogus\"\n"),
        "{stderr}"
    );
}
