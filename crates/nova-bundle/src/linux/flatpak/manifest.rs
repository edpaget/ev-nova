//! The `flatpak-builder` manifest, rendered as JSON from the spec, and
//! the cargo config its build reads.

use std::path::{Component, Path};

use serde_json::{Value, json};

use super::ffmpeg::FfmpegSource;
use super::layout::FlatpakLayout;
use super::{FINISH_ARGS, MODULE, RUNTIME, RUNTIME_VERSION, RUST_BIN, RUST_EXTENSION, SDK};
use crate::build_id::{self, BuildId};
use crate::fonts::{FONT_DIR, FONT_LICENCES};
use crate::linux::EXECUTABLE;
use crate::spec::{BundleSpec, GAME_PACKAGE};

/// Where the vendored crates land in the module's build directory, and
/// so the cargo config's `directory`: relative to the parent of
/// `CARGO_HOME` (`cargo`), the build directory itself.
pub const VENDOR_DEST: &str = "cargo/vendor";
/// Where the share tree lands in the module's build directory.
pub const SHARE_DEST: &str = "linux-share";
/// Where the FFmpeg tarball lands in the module's build directory.
pub const FFMPEG_DEST: &str = "ffmpeg";
/// The variable naming the FFmpeg tarball to the vendored ffmpeg-sys-next's
/// build script, which then never downloads it.
pub const FFMPEG_TARBALL_VAR: &str = "NOVA_FFMPEG_TARBALL";

/// What the workspace source never copies: Git's data, flatpak-builder's
/// default cache, the nightly's output, and Cargo's target directory,
/// which holds the staging (and so would copy itself).
pub const SKIP: [&str; 4] = [".git", ".flatpak-builder", "dist", "target"];

/// The cargo config replacing crates.io with the vendored crates, as
/// `cargo vendor` prints it, read from `CARGO_HOME` in the sandbox.
#[must_use]
pub fn cargo_config() -> String {
    format!(
        "[source.crates-io]\n\
         replace-with = \"vendored-sources\"\n\
         \n\
         [source.vendored-sources]\n\
         directory = \"{VENDOR_DEST}\"\n"
    )
}

/// The commands that build `nova` offline and install it, with the share
/// tree, into `/app`, and `LICENSE` and the fonts' licences
/// ([`crate::fonts`]) into `/app/share/licenses/<app id>/`, Flathub's
/// place for them. The sources are relative to the module's build
/// directory, spelled with `/`: they are shell text run in the Linux
/// sandbox, not host paths.
#[must_use]
pub fn build_commands(app_id: &str) -> Vec<String> {
    let licenses = format!("/app/share/licenses/{app_id}");
    let font_dir = FONT_DIR.join("/");
    let mut commands = vec![
        format!("cargo --offline build --release --locked -p {GAME_PACKAGE}"),
        format!("install -Dm755 target/release/{EXECUTABLE} /app/bin/{EXECUTABLE}"),
        "mkdir -p /app/share".to_owned(),
        format!("cp -r {SHARE_DEST}/. /app/share/"),
        format!("install -Dm644 LICENSE {licenses}/LICENSE"),
    ];
    commands.extend(
        FONT_LICENCES
            .iter()
            .map(|(source, name)| format!("install -Dm644 {font_dir}/{source} {licenses}/{name}")),
    );
    commands
}

/// The workspace source's skips: [`SKIP`], and Cargo's target directory
/// where it lies in the workspace under another name.
#[must_use]
pub fn skips(spec: &BundleSpec) -> Vec<String> {
    let mut skips: Vec<String> = SKIP.iter().map(|s| (*s).to_owned()).collect();
    if let Ok(relative) = spec.target_dir.strip_prefix(&spec.workspace_root)
        && let Some(relative) = slashed(relative)
        && !skips.contains(&relative)
    {
        skips.push(relative);
    }
    skips
}

/// A relative path as `flatpak-builder` reads a skip: its parts joined by
/// `/`, or `None` when it has none.
fn slashed(path: &Path) -> Option<String> {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The manifest for `spec`'s build `build` under `layout`. The sandboxed
/// build sees none of the host's environment, so the module's build
/// options pass it `NOVA_BUILD_ID`, which `nova` compiles in. It has no
/// network either: the FFmpeg tarball the movie decoder builds
/// ([`FfmpegSource::record`]) is a `file` source, which `flatpak-builder`
/// downloads and checks before the build, and the build options name it
/// in `NOVA_FFMPEG_TARBALL`.
#[must_use]
pub fn manifest(spec: &BundleSpec, layout: &FlatpakLayout, build: &BuildId) -> Value {
    let ffmpeg = FfmpegSource::record();
    json!({
        "id": spec.identifier,
        "runtime": RUNTIME,
        "runtime-version": RUNTIME_VERSION,
        "sdk": SDK,
        "sdk-extensions": [RUST_EXTENSION],
        "command": EXECUTABLE,
        "finish-args": FINISH_ARGS,
        "modules": [{
            "name": MODULE,
            "buildsystem": "simple",
            "build-options": {
                "append-path": RUST_BIN,
                "env": {
                    "CARGO_HOME": format!("/run/build/{MODULE}/cargo"),
                    build_id::VAR: build.as_str(),
                    FFMPEG_TARBALL_VAR: format!(
                        "/run/build/{MODULE}/{FFMPEG_DEST}/{}",
                        ffmpeg.file_name()
                    ),
                },
            },
            "build-commands": build_commands(&spec.identifier),
            "sources": [
                {
                    "type": "dir",
                    "path": text(&spec.workspace_root),
                    "skip": skips(spec),
                },
                {
                    "type": "dir",
                    "path": text(&layout.vendor()),
                    "dest": VENDOR_DEST,
                },
                {
                    "type": "file",
                    "path": text(&layout.cargo_config()),
                    "dest": "cargo",
                    "dest-filename": "config.toml",
                },
                {
                    "type": "dir",
                    "path": text(&layout.share()),
                    "dest": SHARE_DEST,
                },
                {
                    "type": "file",
                    "url": ffmpeg.url,
                    "sha256": ffmpeg.sha256,
                    "dest": FFMPEG_DEST,
                },
            ],
        }],
    })
}

/// [`manifest`] as the JSON file's text.
#[must_use]
pub fn render(spec: &BundleSpec, layout: &FlatpakLayout, build: &BuildId) -> String {
    let mut text = serde_json::to_string_pretty(&manifest(spec, layout, build))
        .unwrap_or_else(|e| unreachable!("a JSON value always renders: {e}"));
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{BUILD, build};

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn layout() -> FlatpakLayout {
        FlatpakLayout::new(&spec().target_dir, None).expect("layout")
    }

    /// The rendered manifest, read back.
    fn read() -> Value {
        serde_json::from_str(&render(&spec(), &layout(), &build())).expect("JSON")
    }

    fn strings(value: &Value) -> Vec<&str> {
        value
            .as_array()
            .expect("an array")
            .iter()
            .map(|v| v.as_str().expect("a string"))
            .collect()
    }

    fn module() -> Value {
        let manifest = read();
        let modules = manifest["modules"].as_array().expect("modules");
        assert_eq!(modules.len(), 1, "one module, nova: no zenity of ours");
        modules[0].clone()
    }

    #[test]
    fn its_id_and_command_come_from_the_spec() {
        let manifest = read();
        assert_eq!(manifest["id"], "io.example.nova");
        assert_eq!(manifest["command"], "nova");
        assert_eq!(manifest["command"], EXECUTABLE);
    }

    #[test]
    fn it_runs_on_the_freedesktop_runtime_with_the_rust_extension() {
        let manifest = read();
        assert_eq!(manifest["runtime"], "org.freedesktop.Platform");
        assert_eq!(manifest["runtime-version"], "26.08");
        assert_eq!(manifest["sdk"], "org.freedesktop.Sdk");
        assert_eq!(
            strings(&manifest["sdk-extensions"]),
            ["org.freedesktop.Sdk.Extension.rust-stable"]
        );
        assert_eq!(
            module()["build-options"]["append-path"],
            "/usr/lib/sdk/rust-stable/bin"
        );
    }

    #[test]
    fn its_permissions_are_exactly_the_recorded_list() {
        assert_eq!(strings(&read()["finish-args"]), FINISH_ARGS);
        assert_eq!(
            FINISH_ARGS,
            [
                "--socket=wayland",
                "--socket=fallback-x11",
                "--share=ipc",
                "--device=dri",
                "--socket=pulseaudio",
                "--filesystem=~/.wine:ro",
                "--filesystem=~/Games/ev-nova:ro",
                "--filesystem=~/Games/EV Nova:ro",
                "--persist=.local/state",
            ]
        );
    }

    #[test]
    fn it_has_no_home_host_network_or_session_bus_access() {
        for arg in FINISH_ARGS {
            for refused in [
                "--filesystem=home",
                "--filesystem=host",
                "--share=network",
                "--socket=session-bus",
                "--socket=system-bus",
                "--device=all",
                "--talk-name",
            ] {
                assert!(!arg.starts_with(refused), "{arg}");
            }
            if let Some(path) = arg.strip_prefix("--filesystem=") {
                assert!(path.ends_with(":ro"), "{arg} is read-only");
            }
        }
    }

    #[test]
    fn it_grants_phase_2_s_three_folders_read_only() {
        for grant in [
            "--filesystem=~/.wine:ro",
            "--filesystem=~/Games/ev-nova:ro",
            "--filesystem=~/Games/EV Nova:ro",
        ] {
            assert!(FINISH_ARGS.contains(&grant), "{grant}");
        }
    }

    #[test]
    fn it_builds_nova_offline_from_the_vendored_crates() {
        let module = module();
        assert_eq!(module["name"], "nova");
        assert_eq!(module["buildsystem"], "simple");
        assert_eq!(
            module["build-options"]["env"]["CARGO_HOME"],
            "/run/build/nova/cargo"
        );
        let commands = strings(&module["build-commands"]);
        assert_eq!(
            commands,
            [
                "cargo --offline build --release --locked -p nova",
                "install -Dm755 target/release/nova /app/bin/nova",
                "mkdir -p /app/share",
                "cp -r linux-share/. /app/share/",
                "install -Dm644 LICENSE /app/share/licenses/io.example.nova/LICENSE",
                "install -Dm644 crates/nova-render/fonts/OFL.txt /app/share/licenses/io.example.nova/NotoSans-OFL.txt",
                "install -Dm644 crates/nova-render/fonts/NotoSerif-OFL.txt /app/share/licenses/io.example.nova/NotoSerif-OFL.txt",
            ]
        );
    }

    #[test]
    fn the_build_gets_the_build_id_and_the_ffmpeg_tarball() {
        let env = &module()["build-options"]["env"];
        assert_eq!(env["NOVA_BUILD_ID"], BUILD);
        assert_eq!(
            env["NOVA_FFMPEG_TARBALL"],
            "/run/build/nova/ffmpeg/ffmpeg-9.0.2.tar.xz"
        );
        assert_eq!(env.as_object().expect("an object").len(), 3, "{env}");
    }

    #[test]
    fn the_ffmpeg_source_is_the_recorded_tarball() {
        let record = FfmpegSource::record();
        let sources = module()["sources"].clone();
        let ffmpeg = sources
            .as_array()
            .expect("sources")
            .iter()
            .find(|source| source["dest"] == FFMPEG_DEST)
            .expect("an FFmpeg source")
            .clone();
        assert_eq!(
            ffmpeg,
            json!({
                "type": "file",
                "url": record.url,
                "sha256": record.sha256,
                "dest": "ffmpeg",
            })
        );
    }

    #[test]
    fn the_ffmpeg_record_names_a_release_tarball_and_its_sha256() {
        let record = FfmpegSource::record();
        assert_eq!(
            record.url,
            format!(
                "https://ffmpeg.org/releases/ffmpeg-{}.tar.xz",
                record.version
            )
        );
        assert_eq!(
            record.file_name(),
            format!("ffmpeg-{}.tar.xz", record.version)
        );
        assert_eq!(record.sha256.len(), 64, "{}", record.sha256);
        assert!(
            record
                .sha256
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{}",
            record.sha256
        );
    }

    #[test]
    fn its_sources_are_the_workspace_the_vendor_tree_the_config_the_share_tree_and_ffmpeg() {
        let layout = layout();
        let spec = spec();
        let sources = module()["sources"].clone();
        let text = |p: &Path| p.to_string_lossy().into_owned();
        assert_eq!(
            sources,
            json!([
                {
                    "type": "dir",
                    "path": text(&spec.workspace_root),
                    "skip": [".git", ".flatpak-builder", "dist", "target"],
                },
                {
                    "type": "dir",
                    "path": text(&layout.vendor()),
                    "dest": "cargo/vendor",
                },
                {
                    "type": "file",
                    "path": text(&layout.cargo_config()),
                    "dest": "cargo",
                    "dest-filename": "config.toml",
                },
                {
                    "type": "dir",
                    "path": text(&layout.share()),
                    "dest": "linux-share",
                },
                {
                    "type": "file",
                    "url": FfmpegSource::record().url,
                    "sha256": FfmpegSource::record().sha256,
                    "dest": "ffmpeg",
                },
            ])
        );
    }

    #[test]
    fn the_config_points_crates_io_at_the_vendored_crates() {
        let config = cargo_config();
        assert_eq!(
            config,
            "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n\
             [source.vendored-sources]\ndirectory = \"cargo/vendor\"\n"
        );
    }

    #[test]
    fn a_target_dir_of_another_name_in_the_workspace_is_skipped_too() {
        let mut spec = spec();
        spec.target_dir = spec.workspace_root.join("build").join("cargo");
        assert_eq!(
            skips(&spec),
            [".git", ".flatpak-builder", "dist", "target", "build/cargo"]
        );
    }

    #[test]
    fn a_target_dir_outside_the_workspace_adds_no_skip() {
        let mut spec = spec();
        spec.target_dir = PathBuf::from("/elsewhere").join("target");
        assert_eq!(skips(&spec), SKIP);
        spec.target_dir = spec.workspace_root.clone();
        assert_eq!(skips(&spec), SKIP, "the root itself adds nothing");
    }

    #[test]
    fn the_file_ends_with_a_newline() {
        assert!(render(&spec(), &layout(), &build()).ends_with("}\n"));
    }
}
