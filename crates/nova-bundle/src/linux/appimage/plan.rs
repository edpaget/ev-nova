//! The AppImage's sequence: read the spec, build `nova`, lay out the
//! AppDir, check the staged binary's links, and run `appimagetool`.
//!
//! [`plan`] is the sequence as data; [`build_appimage`] runs it through
//! the ports, stopping at the first failure.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::host::AppImageHost;
use super::layout::AppImageLayout;
use super::{AppImageError, TARGET};
use crate::bundle::{self, BundleError, Step, execute, metadata};
use crate::linux::desktop::{desktop_entry, desktop_file_name};
use crate::linux::icons::ICON_256;
use crate::linux::{EXECUTABLE, share};
use crate::ports::{Files, Invocation, Process};
use crate::spec::{BundleSpec, GAME_PACKAGE};

/// `cargo build` of `nova` alone, in release, for [`TARGET`], with the
/// default features only (so egui stays out), as the nightly's `Build`
/// step builds it: after that step this is a no-op.
#[must_use]
pub fn build(cargo: &OsStr) -> Invocation {
    Invocation::new(cargo).args([
        "build",
        "--release",
        "--locked",
        "-p",
        GAME_PACKAGE,
        "--target",
        TARGET,
    ])
}

/// `appimagetool` packing the AppDir into the AppImage, embedding
/// `runtime` so it never downloads one.
#[must_use]
pub fn appimagetool(tool: &OsStr, runtime: &Path, layout: &AppImageLayout) -> Invocation {
    Invocation::new(tool)
        .arg("--runtime-file")
        .arg(runtime)
        .arg(layout.app_dir())
        .arg(layout.appimage())
}

/// The steps that build `spec`'s AppImage under `layout`, with the
/// `cargo` and `appimagetool` of `host` and the `runtime` to embed.
#[must_use]
pub fn plan(
    spec: &BundleSpec,
    layout: &AppImageLayout,
    host: &AppImageHost,
    runtime: &Path,
) -> Vec<Step> {
    let app_dir = layout.app_dir();
    let icon = format!("{}.png", spec.identifier);
    let mut steps = vec![
        Step::Run(build(&host.cargo)),
        Step::RemoveDir(layout.staging.clone()),
        Step::CreateDir(layout.usr().join("bin")),
        Step::Copy {
            from: spec
                .target_dir
                .join(TARGET)
                .join("release")
                .join(EXECUTABLE),
            to: layout.executable(),
        },
        // What the AppImage runs.
        Step::Symlink {
            target: Path::new("usr").join("bin").join(EXECUTABLE),
            link: app_dir.join("AppRun"),
        },
        // What integration tools (Gear Lever, AppImageLauncher) install.
        Step::Write {
            path: app_dir.join(desktop_file_name(spec)),
            bytes: desktop_entry(spec).into_bytes(),
        },
        Step::Write {
            path: app_dir.join(&icon),
            bytes: ICON_256.png.to_vec(),
        },
        Step::Symlink {
            target: PathBuf::from(&icon),
            link: app_dir.join(".DirIcon"),
        },
    ];
    steps.extend(share::steps(spec, &layout.usr()));
    steps.extend([
        Step::CheckLinks(layout.executable()),
        Step::CreateDir(layout.out.clone()),
        Step::Run(appimagetool(&host.appimagetool, runtime, layout)),
    ]);
    steps
}

/// Builds the AppImage, returning its path. Fails before running
/// anything when no runtime is given or it is missing.
pub fn build_appimage(
    process: &mut impl Process,
    files: &mut impl Files,
    host: &AppImageHost,
) -> Result<PathBuf, AppImageError> {
    let runtime = host.runtime.as_ref().ok_or(AppImageError::NoRuntime)?;
    if !files.is_file(runtime) {
        return Err(AppImageError::MissingRuntime(runtime.clone()));
    }
    let json = bundle::run(process, &metadata(&host.cargo))?;
    let spec = BundleSpec::from_metadata(&json).map_err(BundleError::from)?;
    let layout = AppImageLayout::new(&spec.target_dir, host.out.clone())?;
    for step in plan(&spec, &layout, host, runtime) {
        execute(&step, process, files)?;
    }
    Ok(layout.appimage())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::rc::Rc;

    use super::*;
    use crate::linux::OutputOverlapsWork;
    use crate::linux::appimage::links::objdump;
    use crate::linux::appimage::links::tests::{NEEDED, objdump_text, portable_text};
    use crate::linux::metainfo::{metainfo, metainfo_file_name};
    use crate::ports::RunError;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{Event, FakeFiles, Log, Recorder, failing, is, starts};

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn runtime() -> PathBuf {
        Path::new("/tools").join("runtime-x86_64")
    }

    fn host() -> AppImageHost {
        AppImageHost {
            cargo: "/rust/bin/cargo".into(),
            appimagetool: "/tools/appimagetool".into(),
            runtime: Some(runtime()),
            out: None,
        }
    }

    struct Built {
        result: Result<PathBuf, AppImageError>,
        events: Vec<Event>,
    }

    impl Built {
        fn runs(&self) -> Vec<Invocation> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    Event::Run(i) => Some(i.clone()),
                    _ => None,
                })
                .collect()
        }

        fn ran(&self, program: &str) -> bool {
            self.runs().iter().any(|i| i.program == program)
        }
    }

    /// Answers `objdump -p` with `objdump`, and the rest as [`Recorder`]
    /// does, failing what `respond` picks.
    fn build_with(
        host: &AppImageHost,
        objdump: String,
        respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
        file_fails: impl Fn(&Event) -> bool + 'static,
        absent: impl Fn(&Path) -> bool + 'static,
    ) -> Built {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(move |i| {
                respond(i).or_else(|| {
                    starts(i, "objdump", &["-p"]).then(|| Ok(objdump.clone().into_bytes()))
                })
            }),
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(file_fails),
            absent: Box::new(absent),
        };
        let result = build_appimage(&mut process, &mut files, host);
        let events = log.borrow().clone();
        Built { result, events }
    }

    fn build_ok(host: &AppImageHost) -> Built {
        build_with(host, portable_text(), |_| None, |_| false, |_| false)
    }

    #[test]
    fn runs_the_whole_sequence_in_order() {
        // Cargo's own spelling of the target dir, so the paths in each
        // command's args (text, not paths) match on Windows.
        let spec = spec();
        let target = spec.target_dir.clone();
        let staging = target.join("appimage").join("staging");
        let app_dir = staging.join("AppDir");
        let usr = app_dir.join("usr");
        let exe = usr.join("bin").join("nova");
        let out = target.join("appimage").join("out");
        let appimage = out.join("ev-nova-x86_64-unknown-linux-gnu.AppImage");

        let built = build_ok(&host());

        assert_eq!(built.result.as_ref().expect("built"), &appimage);
        let mut expected = vec![
            Event::Run(metadata(&OsString::from("/rust/bin/cargo"))),
            Event::Run(
                Invocation::new("/rust/bin/cargo").args(
                    "build --release --locked -p nova --target x86_64-unknown-linux-gnu"
                        .split_whitespace(),
                ),
            ),
            Event::RemoveDir(staging.clone()),
            Event::CreateDir(usr.join("bin")),
            Event::Copy(
                target
                    .join("x86_64-unknown-linux-gnu")
                    .join("release")
                    .join("nova"),
                exe.clone(),
            ),
            Event::Symlink(
                Path::new("usr").join("bin").join("nova"),
                app_dir.join("AppRun"),
            ),
            Event::Write(
                app_dir.join("io.example.nova.desktop"),
                desktop_entry(&spec).into_bytes(),
            ),
            Event::Write(app_dir.join("io.example.nova.png"), ICON_256.png.to_vec()),
            Event::Symlink(
                PathBuf::from("io.example.nova.png"),
                app_dir.join(".DirIcon"),
            ),
        ];
        expected.extend(
            share::steps(&spec, &usr)
                .into_iter()
                .map(|step| match step {
                    Step::CreateDir(dir) => Event::CreateDir(dir),
                    Step::Write { path, bytes } => Event::Write(path, bytes),
                    other => panic!("an unexpected step: {other:?}"),
                }),
        );
        expected.extend([
            Event::Run(Invocation::new("objdump").arg("-p").arg(&exe).captured()),
            Event::CreateDir(out.clone()),
            Event::Run(
                Invocation::new("/tools/appimagetool")
                    .arg("--runtime-file")
                    .arg(runtime())
                    .arg(&app_dir)
                    .arg(&appimage),
            ),
        ]);
        assert_eq!(built.events, expected);
    }

    #[test]
    fn the_app_dir_holds_phase_1_s_share_tree_under_usr() {
        let spec = spec();
        let layout = AppImageLayout::new(&spec.target_dir, None).expect("layout");
        let steps = plan(&spec, &layout, &host(), &runtime());
        let share = layout.usr().join("share");
        for (path, bytes) in [
            (
                share.join("applications").join(desktop_file_name(&spec)),
                desktop_entry(&spec).into_bytes(),
            ),
            (
                share.join("metainfo").join(metainfo_file_name(&spec)),
                metainfo(&spec).into_bytes(),
            ),
        ] {
            assert!(
                steps.contains(&Step::Write {
                    path: path.clone(),
                    bytes
                }),
                "{}",
                path.display()
            );
        }
    }

    #[test]
    fn the_root_icon_is_the_one_the_desktop_file_names() {
        let spec = spec();
        let layout = AppImageLayout::new(&spec.target_dir, None).expect("layout");
        let steps = plan(&spec, &layout, &host(), &runtime());
        let entry = desktop_entry(&spec);
        let icon = entry
            .lines()
            .find_map(|l| l.strip_prefix("Icon="))
            .expect("an Icon key");
        assert!(steps.contains(&Step::Write {
            path: layout.app_dir().join(format!("{icon}.png")),
            bytes: ICON_256.png.to_vec(),
        }));
    }

    #[test]
    fn writes_the_appimage_to_the_output_dir_given() {
        let dist = Path::new("/work").join("ev-nova").join("dist");
        let built = build_ok(&AppImageHost {
            out: Some(dist.clone()),
            ..host()
        });
        let appimage = dist.join("ev-nova-x86_64-unknown-linux-gnu.AppImage");
        assert_eq!(built.result.as_ref().expect("built"), &appimage);
        let tool = built.runs().pop().expect("a run");
        assert_eq!(tool.args.last(), Some(&appimage.into_os_string()));
        assert!(built.events.contains(&Event::CreateDir(dist)));
    }

    #[test]
    fn without_a_runtime_it_runs_nothing() {
        let built = build_ok(&AppImageHost {
            runtime: None,
            ..host()
        });
        assert!(
            matches!(built.result, Err(AppImageError::NoRuntime)),
            "{:?}",
            built.result
        );
        let message = built.result.as_ref().expect_err("fails").to_string();
        assert!(message.contains("NOVA_APPIMAGE_RUNTIME"), "{message}");
        assert_eq!(built.events, []);
    }

    #[test]
    fn a_missing_runtime_runs_nothing() {
        let built = build_with(
            &host(),
            portable_text(),
            |_| None,
            |_| false,
            |p| p == runtime(),
        );
        let Err(AppImageError::MissingRuntime(path)) = &built.result else {
            panic!("{:?}", built.result);
        };
        assert_eq!(path, &runtime());
        assert_eq!(built.events, []);
    }

    #[test]
    fn a_binary_the_floor_refuses_stops_before_appimagetool() {
        for text in [
            objdump_text(&NEEDED, &["GLIBC_2.39"]),
            objdump_text(&["libc.so.6", "libwayland-client.so.0"], &["GLIBC_2.17"]),
        ] {
            let built = build_with(&host(), text, |_| None, |_| false, |_| false);
            let spec = spec();
            let layout = AppImageLayout::new(&spec.target_dir, None).expect("layout");
            let Err(AppImageError::Step(BundleError::NotPortable { command, .. })) = &built.result
            else {
                panic!("{:?}", built.result);
            };
            assert_eq!(command, &objdump(&layout.executable()).to_string());
            assert!(!built.ran("/tools/appimagetool"));
            assert!(!built.events.contains(&Event::CreateDir(layout.out)));
        }
    }

    #[test]
    fn a_failing_metadata_stages_nothing() {
        let built = build_with(
            &host(),
            portable_text(),
            failing(is("/rust/bin/cargo", "metadata")),
            |_| false,
            |_| false,
        );
        assert!(matches!(
            built.result,
            Err(AppImageError::Step(BundleError::Run { .. }))
        ));
        assert_eq!(built.events.len(), 1);
    }

    #[test]
    fn unreadable_metadata_is_a_spec_error() {
        let built = build_with(
            &host(),
            portable_text(),
            |i| starts(i, "/rust/bin/cargo", &["metadata"]).then(|| Ok(b"not json".to_vec())),
            |_| false,
            |_| false,
        );
        assert!(matches!(
            built.result,
            Err(AppImageError::Step(BundleError::Spec(_)))
        ));
    }

    #[test]
    fn a_failing_build_stops_before_staging() {
        let built = build_with(
            &host(),
            portable_text(),
            failing(is("/rust/bin/cargo", "build")),
            |_| false,
            |_| false,
        );
        let message = built.result.as_ref().expect_err("fails").to_string();
        assert!(
            message.contains("/rust/bin/cargo build --release --locked -p nova"),
            "{message}"
        );
        assert_eq!(built.events.len(), 2, "nothing staged");
    }

    #[test]
    fn a_failing_objdump_stops_before_appimagetool() {
        let built = build_with(
            &host(),
            portable_text(),
            failing(is("objdump", "-p")),
            |_| false,
            |_| false,
        );
        let message = built.result.as_ref().expect_err("fails").to_string();
        assert!(message.contains("`objdump -p"), "{message}");
        assert!(!built.ran("/tools/appimagetool"));
    }

    #[test]
    fn a_failing_appimagetool_names_its_command_line() {
        let built = build_with(
            &host(),
            portable_text(),
            failing(is("/tools/appimagetool", "--runtime-file")),
            |_| false,
            |_| false,
        );
        let message = built.result.as_ref().expect_err("fails").to_string();
        assert!(
            message.contains("`/tools/appimagetool --runtime-file"),
            "{message}"
        );
        assert!(message.contains("exited with code 1"), "{message}");
    }

    #[test]
    fn each_file_step_s_failure_stops_it_naming_the_path() {
        type Fails = fn(&Event) -> bool;
        let cases: [(Fails, &str); 5] = [
            (|e| matches!(e, Event::RemoveDir(_)), "remove"),
            (|e| matches!(e, Event::CreateDir(_)), "create"),
            (|e| matches!(e, Event::Write(..)), "write"),
            (|e| matches!(e, Event::Copy(..)), "copy to"),
            (|e| matches!(e, Event::Symlink(..)), "link"),
        ];
        for (fails, expected) in cases {
            let built = build_with(&host(), portable_text(), |_| None, fails, |_| false);
            let Err(AppImageError::Step(BundleError::File { action, .. })) = &built.result else {
                panic!("{expected}: {:?}", built.result);
            };
            assert_eq!(*action, expected);
            assert!(!built.ran("/tools/appimagetool"), "{expected}");
            let failed = built.events.iter().filter(|e| fails(e)).count();
            assert_eq!(failed, 1, "{expected}: stops at the first failure");
        }
    }

    #[test]
    fn an_output_dir_overlapping_staging_stages_nothing() {
        let out = spec().target_dir.join("appimage");
        let built = build_ok(&AppImageHost {
            out: Some(out.clone()),
            ..host()
        });
        let Err(AppImageError::Layout(OutputOverlapsWork { out: refused, .. })) = &built.result
        else {
            panic!("{:?}", built.result);
        };
        assert_eq!(refused, &out);
        assert_eq!(built.events.len(), 1, "only cargo metadata ran");
    }
}
