//! The Flatpak's sequence: read the spec, vendor the crates, stage the
//! cargo config, the share tree and the manifest, build and export with
//! `flatpak-builder`, then make the single-file bundle.
//!
//! [`plan`] is the sequence as data; [`build_flatpak`] runs it through
//! the ports, stopping at the first failure.

use std::ffi::OsStr;
use std::path::PathBuf;

use super::host::FlatpakHost;
use super::layout::FlatpakLayout;
use super::manifest::{cargo_config, render};
use super::{FlatpakError, REMOTE, RUNTIME_REPO};
use crate::build_id::{self, BuildId};
use crate::bundle::{self, BundleError, Step, execute, metadata};
use crate::linux::share;
use crate::ports::{Env, Files, Invocation, Process};
use crate::spec::BundleSpec;

/// `cargo vendor` of the workspace's locked crates into the staging,
/// on the host (the build sandbox has no network). Its output, the
/// config it suggests, is captured and dropped: [`cargo_config`] writes
/// it.
#[must_use]
pub fn vendor(cargo: &OsStr, spec: &BundleSpec, layout: &FlatpakLayout) -> Invocation {
    Invocation::new(cargo)
        .args(["vendor", "--locked", "--manifest-path"])
        .arg(spec.workspace_root.join("Cargo.toml"))
        .arg(layout.vendor())
        .captured()
}

/// `flatpak-builder` building the manifest and exporting it to the
/// repository. It installs the runtime, SDK and Rust extension from the
/// user's [`REMOTE`] remote when they are missing, starts from a clean
/// build directory, keeps its cache in the layout's state directory, and
/// needs no FUSE (`--disable-rofiles-fuse`).
#[must_use]
pub fn flatpak_builder(spec: &BundleSpec, layout: &FlatpakLayout) -> Invocation {
    Invocation::new("flatpak-builder")
        .args([
            "--user",
            &format!("--install-deps-from={REMOTE}"),
            "--disable-rofiles-fuse",
            "--force-clean",
            "--state-dir",
        ])
        .arg(&layout.state)
        .arg("--repo")
        .arg(&layout.repo)
        .arg(&layout.build)
        .arg(layout.manifest(spec))
}

/// `flatpak build-bundle`: the app from the repository into one file,
/// naming Flathub as where its runtime comes from.
#[must_use]
pub fn build_bundle(spec: &BundleSpec, layout: &FlatpakLayout) -> Invocation {
    Invocation::new("flatpak")
        .arg("build-bundle")
        .arg(format!("--runtime-repo={RUNTIME_REPO}"))
        .arg(&layout.repo)
        .arg(layout.bundle())
        .arg(&spec.identifier)
}

/// The steps that build `spec`'s Flatpak, the build `build`, under
/// `layout` with `cargo`.
#[must_use]
pub fn plan(
    spec: &BundleSpec,
    layout: &FlatpakLayout,
    cargo: &OsStr,
    build: &BuildId,
) -> Vec<Step> {
    let mut steps = vec![
        Step::RemoveDir(layout.staging.clone()),
        Step::CreateDir(layout.staging.clone()),
        Step::Run(vendor(cargo, spec, layout)),
        Step::Write {
            path: layout.cargo_config(),
            bytes: cargo_config().into_bytes(),
        },
    ];
    steps.extend(share::steps(spec, build, &layout.staging));
    steps.extend([
        Step::Write {
            path: layout.manifest(spec),
            bytes: render(spec, layout, build).into_bytes(),
        },
        Step::Run(flatpak_builder(spec, layout)),
        Step::CreateDir(layout.out.clone()),
        Step::Run(build_bundle(spec, layout)),
    ]);
    steps
}

/// Builds the Flatpak bundle, returning its path, with the build ID from
/// `env` or git ([`build_id::resolve`]).
pub fn build_flatpak(
    process: &mut impl Process,
    files: &mut impl Files,
    env: &impl Env,
    host: &FlatpakHost,
) -> Result<PathBuf, FlatpakError> {
    let json = bundle::run(process, &metadata(&host.cargo))?;
    let spec = BundleSpec::from_metadata(&json).map_err(BundleError::from)?;
    let build = build_id::resolve(env, process, &spec.workspace_root).map_err(BundleError::from)?;
    let layout = FlatpakLayout::new(&spec.target_dir, host.out.clone())?;
    for step in plan(&spec, &layout, &host.cargo, &build) {
        execute(&step, process, files)?;
    }
    Ok(layout.bundle())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::rc::Rc;

    use super::*;
    use crate::linux::OutputOverlapsWork;
    use crate::ports::RunError;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{
        Event, FakeEnv, FakeFiles, Log, Recorder, build, build_env, failing, is, starts,
    };

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn host() -> FlatpakHost {
        FlatpakHost {
            cargo: "/rust/bin/cargo".into(),
            out: None,
        }
    }

    struct Built {
        result: Result<PathBuf, FlatpakError>,
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

        fn ran(&self, program: &str, first: &str) -> bool {
            self.runs().iter().any(|i| starts(i, program, &[first]))
        }
    }

    fn build_with(
        host: &FlatpakHost,
        respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
        file_fails: impl Fn(&Event) -> bool + 'static,
    ) -> Built {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(respond),
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(file_fails),
            absent: Box::new(|_| false),
        };
        let result = build_flatpak(&mut process, &mut files, &build_env(), host);
        let events = log.borrow().clone();
        Built { result, events }
    }

    #[test]
    fn runs_the_whole_sequence_in_order() {
        // Cargo's own spelling of the workspace and target dirs, so the
        // paths in each command's args (text, not paths) match on Windows.
        let spec = spec();
        let flatpak = spec.target_dir.join("flatpak");
        let staging = flatpak.join("staging");
        let manifest = staging.join("io.example.nova.json");
        let out = flatpak.join("out");
        let bundle = out.join("ev-nova-x86_64-unknown-linux-gnu.flatpak");
        let layout = FlatpakLayout::new(&spec.target_dir, None).expect("layout");

        let built = build_with(&host(), |_| None, |_| false);

        assert_eq!(built.result.as_ref().expect("built"), &bundle);
        let mut expected = vec![
            Event::Run(metadata(&OsString::from("/rust/bin/cargo"))),
            Event::RemoveDir(staging.clone()),
            Event::CreateDir(staging.clone()),
            Event::Run(
                Invocation::new("/rust/bin/cargo")
                    .args(["vendor", "--locked", "--manifest-path"])
                    .arg(spec.workspace_root.join("Cargo.toml"))
                    .arg(staging.join("vendor"))
                    .captured(),
            ),
            Event::Write(
                staging.join("cargo-config.toml"),
                cargo_config().into_bytes(),
            ),
        ];
        expected.extend(share::steps(&spec, &build(), &staging).into_iter().map(
            |step| match step {
                Step::CreateDir(dir) => Event::CreateDir(dir),
                Step::Write { path, bytes } => Event::Write(path, bytes),
                other => panic!("an unexpected step: {other:?}"),
            },
        ));
        expected.extend([
            Event::Write(
                manifest.clone(),
                render(&spec, &layout, &build()).into_bytes(),
            ),
            Event::Run(
                Invocation::new("flatpak-builder")
                    .args([
                        "--user",
                        "--install-deps-from=flathub",
                        "--disable-rofiles-fuse",
                        "--force-clean",
                        "--state-dir",
                    ])
                    .arg(flatpak.join("state"))
                    .arg("--repo")
                    .arg(flatpak.join("repo"))
                    .arg(flatpak.join("build"))
                    .arg(&manifest),
            ),
            Event::CreateDir(out.clone()),
            Event::Run(
                Invocation::new("flatpak")
                    .args([
                        "build-bundle",
                        "--runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo",
                    ])
                    .arg(flatpak.join("repo"))
                    .arg(&bundle)
                    .arg("io.example.nova"),
            ),
        ]);
        assert_eq!(built.events, expected);
    }

    #[test]
    fn the_share_tree_is_staged_where_the_manifest_takes_it_from() {
        let spec = spec();
        let layout = FlatpakLayout::new(&spec.target_dir, None).expect("layout");
        let steps = plan(&spec, &layout, OsStr::new("cargo"), &build());
        for (path, _) in share::files(&spec, &build(), &layout.staging) {
            assert!(path.starts_with(layout.share()), "{}", path.display());
            assert!(
                steps
                    .iter()
                    .any(|s| matches!(s, Step::Write { path: p, .. } if *p == path)),
                "{}",
                path.display()
            );
        }
    }

    #[test]
    fn a_failing_build_id_stages_nothing() {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(|_| None),
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(|_| false),
            absent: Box::new(|_| false),
        };
        let env = FakeEnv(std::collections::HashMap::from([(
            crate::build_id::VAR,
            "".into(),
        )]));
        let result = build_flatpak(&mut process, &mut files, &env, &host());
        assert!(
            matches!(result, Err(FlatpakError::Step(BundleError::BuildId(_)))),
            "{result:?}"
        );
        assert_eq!(log.borrow().len(), 1, "only cargo metadata ran");
    }

    #[test]
    fn writes_the_bundle_to_the_output_dir_given() {
        let dist = Path::new("/work").join("ev-nova").join("dist");
        let built = build_with(
            &FlatpakHost {
                out: Some(dist.clone()),
                ..host()
            },
            |_| None,
            |_| false,
        );
        let bundle = dist.join("ev-nova-x86_64-unknown-linux-gnu.flatpak");
        assert_eq!(built.result.as_ref().expect("built"), &bundle);
        let last = built.runs().pop().expect("a run");
        assert_eq!(last.args[3], bundle.into_os_string());
        assert!(built.events.contains(&Event::CreateDir(dist)));
    }

    #[test]
    fn a_failing_metadata_stages_nothing() {
        let built = build_with(&host(), failing(is("/rust/bin/cargo", "metadata")), |_| {
            false
        });
        assert!(matches!(
            built.result,
            Err(FlatpakError::Step(BundleError::Run { .. }))
        ));
        assert_eq!(built.events.len(), 1);
    }

    #[test]
    fn unreadable_metadata_is_a_spec_error() {
        let built = build_with(
            &host(),
            |i| starts(i, "/rust/bin/cargo", &["metadata"]).then(|| Ok(b"not json".to_vec())),
            |_| false,
        );
        assert!(matches!(
            built.result,
            Err(FlatpakError::Step(BundleError::Spec(_)))
        ));
    }

    #[test]
    fn each_failing_program_stops_it_naming_its_command_line() {
        for (program, first, shown) in [
            (
                "/rust/bin/cargo",
                "vendor",
                "`/rust/bin/cargo vendor --locked",
            ),
            ("flatpak-builder", "--user", "`flatpak-builder --user"),
            (
                "flatpak",
                "build-bundle",
                "`flatpak build-bundle --runtime-repo=",
            ),
        ] {
            let built = build_with(&host(), failing(is(program, first)), |_| false);
            let message = built.result.as_ref().expect_err("fails").to_string();
            assert!(message.contains(shown), "{message}");
            assert!(message.contains("exited with code 1"), "{message}");
            assert!(
                matches!(built.events.last(), Some(Event::Run(i)) if starts(i, program, &[first])),
                "{program}: stops at it"
            );
        }
    }

    #[test]
    fn a_failing_vendor_runs_no_flatpak_tool() {
        let built = build_with(&host(), failing(is("/rust/bin/cargo", "vendor")), |_| false);
        assert!(built.result.is_err());
        assert!(!built.ran("flatpak-builder", "--user"));
        assert!(!built.ran("flatpak", "build-bundle"));
    }

    #[test]
    fn each_file_step_s_failure_stops_it_naming_the_path() {
        type Fails = fn(&Event) -> bool;
        let cases: [(Fails, &str); 3] = [
            (|e| matches!(e, Event::RemoveDir(_)), "remove"),
            (|e| matches!(e, Event::CreateDir(_)), "create"),
            (|e| matches!(e, Event::Write(..)), "write"),
        ];
        for (fails, expected) in cases {
            let built = build_with(&host(), |_| None, fails);
            let Err(FlatpakError::Step(BundleError::File { action, .. })) = &built.result else {
                panic!("{expected}: {:?}", built.result);
            };
            assert_eq!(*action, expected);
            assert!(!built.ran("flatpak-builder", "--user"), "{expected}");
            let failed = built.events.iter().filter(|e| fails(e)).count();
            assert_eq!(failed, 1, "{expected}: stops at the first failure");
        }
    }

    #[test]
    fn a_failing_manifest_write_names_the_manifest() {
        let spec = spec();
        let layout = FlatpakLayout::new(&spec.target_dir, None).expect("layout");
        let manifest = layout.manifest(&spec);
        let target = manifest.clone();
        let built = build_with(
            &host(),
            |_| None,
            move |e| matches!(e, Event::Write(path, _) if *path == target),
        );
        let Err(FlatpakError::Step(BundleError::File { path, .. })) = &built.result else {
            panic!("{:?}", built.result);
        };
        assert_eq!(path, &manifest);
        assert!(!built.ran("flatpak-builder", "--user"));
    }

    #[test]
    fn an_output_dir_overlapping_a_work_dir_stages_nothing() {
        let out = spec().target_dir.join("flatpak").join("repo");
        let built = build_with(
            &FlatpakHost {
                out: Some(out.clone()),
                ..host()
            },
            |_| None,
            |_| false,
        );
        let Err(FlatpakError::Layout(OutputOverlapsWork { out: refused, .. })) = &built.result
        else {
            panic!("{:?}", built.result);
        };
        assert_eq!(refused, &out);
        assert_eq!(built.events.len(), 1, "only cargo metadata ran");
    }
}
