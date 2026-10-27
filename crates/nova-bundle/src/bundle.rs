//! The bundler's sequence: read the spec, build for Apple silicon, stage
//! `Nova.app` with the binaries, check they are arm64, sign, verify, and
//! make the `.dmg`.
//!
//! [`plan`] is the whole sequence as data, a function of the spec, the
//! layout and the signing mode: the steps, and the cleanup that runs after
//! them whether they succeed or not. [`bundle`] runs the steps through the
//! ports, stopping at the first failure, then every cleanup step.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use crate::icon::ICON;
use crate::info_plist::{EXECUTABLE, info_plist};
use crate::layout::{DUMP, Layout, LayoutError, VOLUME};
use crate::notary::{Notarization, notarize};
use crate::ports::{Files, Invocation, Process, RunError};
use crate::signing::{self, SecretBytes, Signing, add_to_search_list};
use crate::spec::{BundleSpec, GAME_PACKAGE, SpecError};

/// The one target built: Apple silicon. Intel Macs are not supported.
pub const TARGET: &str = "aarch64-apple-darwin";
/// The architecture `lipo -archs` must list, alone, for each binary.
pub const ARCH: &str = "arm64";

/// What the edge supplies: the tools and the settings the build runs with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    /// The `cargo` to build with (`$CARGO` under `cargo run`).
    pub cargo: OsString,
    /// `MACOSX_DEPLOYMENT_TARGET`, the binaries' minimum macOS.
    pub min_system: String,
    /// The output directory asked for, if any.
    pub out: Option<PathBuf>,
    /// How the bundle is signed.
    pub signing: Signing,
}

/// One step of the sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Removes a directory, if present.
    RemoveDir(PathBuf),
    /// Creates a directory and its parents.
    CreateDir(PathBuf),
    /// Writes a file.
    Write {
        /// The file.
        path: PathBuf,
        /// Its contents.
        bytes: Vec<u8>,
    },
    /// Copies a file.
    Copy {
        /// The source.
        from: PathBuf,
        /// The destination.
        to: PathBuf,
    },
    /// Makes a symbolic link.
    Symlink {
        /// What the link points to.
        target: PathBuf,
        /// The link.
        link: PathBuf,
    },
    /// Writes a secret file, readable by its owner alone.
    WriteSecret {
        /// The file.
        path: PathBuf,
        /// Its contents, which `Debug` never shows.
        bytes: SecretBytes,
    },
    /// Runs a program.
    Run(Invocation),
    /// Fails unless the binary is [`ARCH`] alone: see [`check_archs`].
    CheckArchs(PathBuf),
    /// Puts the keychain first in the user's keychain search list, so
    /// `codesign` and `security` find its identity: see
    /// [`add_to_search_list`].
    AddToSearchList(PathBuf),
    /// Notarizes a file: see [`notarize`].
    Notarize(Notarization),
}

/// Why bundling stopped.
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// A program failed.
    #[error("`{command}` failed: {source}")]
    Run {
        /// The command line.
        command: String,
        /// How it failed.
        source: RunError,
    },
    /// `cargo metadata` gave no spec.
    #[error(transparent)]
    Spec(#[from] SpecError),
    /// The output directory can't be used.
    #[error(transparent)]
    Layout(#[from] LayoutError),
    /// A file step failed.
    #[error("could not {action} {}: {source}", path.display())]
    File {
        /// What the step did.
        action: &'static str,
        /// The path it did it to.
        path: PathBuf,
        /// Why it failed.
        source: io::Error,
    },
    /// A binary is not [`ARCH`] alone.
    #[error("`{command}` lists `{found}`, not {ARCH} alone")]
    NotArm64 {
        /// The `lipo -archs` command line.
        command: String,
        /// What it listed.
        found: String,
    },
    /// `notarytool` printed output the core can't read.
    #[error("`{command}` printed no {reason}")]
    NotaryOutput {
        /// The `notarytool` command line.
        command: String,
        /// What its output lacked.
        reason: String,
    },
    /// The notary service did not accept the file.
    #[error(
        "the notary service did not accept {} (submission {id}): {status}. \
         Its log is printed above",
        path.display()
    )]
    NotNotarized {
        /// The file submitted.
        path: PathBuf,
        /// The submission's id.
        id: String,
        /// The submission's status.
        status: String,
    },
}

/// `cargo metadata`, whose output is the spec.
#[must_use]
pub fn metadata(cargo: &OsString) -> Invocation {
    Invocation::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .captured()
}

/// Every step after `cargo metadata`, and the cleanup after them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The sequence, in order; the first failure stops it.
    pub steps: Vec<Step>,
    /// What runs after the steps, whether they succeeded or not, each step
    /// whatever the one before it did.
    pub cleanup: Vec<Step>,
}

/// `codesign --verify --deep --strict` on a bundle or binary.
pub(crate) fn verify(path: PathBuf) -> Step {
    Step::Run(
        Invocation::new("codesign")
            .args(["--verify", "--deep", "--strict", "--verbose=2"])
            .arg(path),
    )
}

/// `hdiutil create`: the staging tree into the `.dmg`.
pub(crate) fn make_dmg(layout: &Layout) -> Step {
    Step::Run(
        Invocation::new("hdiutil")
            .args(["create", "-volname", VOLUME, "-srcfolder"])
            .arg(&layout.staging)
            .args(["-fs", "HFS+", "-format", "UDZO", "-ov"])
            .arg(layout.dmg()),
    )
}

/// The plan for `spec` under `layout`, signed as `host` says.
#[must_use]
pub fn plan(spec: &BundleSpec, layout: &Layout, host: &Host) -> Plan {
    let mut steps = stage(spec, layout, host);
    match &host.signing {
        Signing::AdHoc => {
            steps.extend(ad_hoc(layout));
            Plan {
                steps,
                cleanup: Vec::new(),
            }
        }
        Signing::DeveloperId(credentials) => {
            steps.extend(signing::developer_id(credentials, layout));
            Plan {
                steps,
                cleanup: signing::teardown(layout),
            }
        }
    }
}

/// Ad-hoc signatures, so Apple silicon runs the binaries, then the
/// `.dmg`. Not `--deep`: the bundle holds one executable and no nested
/// code, and Apple discourages `--deep` for signing. Verifying is
/// `--deep`.
fn ad_hoc(layout: &Layout) -> Vec<Step> {
    let sign = |path: PathBuf| {
        Step::Run(
            Invocation::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(path),
        )
    };
    vec![
        sign(layout.dump()),
        sign(layout.app()),
        verify(layout.app()),
        verify(layout.dump()),
        Step::CreateDir(layout.out.clone()),
        make_dmg(layout),
    ]
}

/// The build and the staging tree, up to signing.
fn stage(spec: &BundleSpec, layout: &Layout, host: &Host) -> Vec<Step> {
    let release = |bin: &str| spec.target_dir.join(TARGET).join("release").join(bin);
    vec![
        // Default features only, so egui stays out of the published build.
        Step::Run(Invocation::new(&host.cargo).args([
            "build",
            "--release",
            "--locked",
            "-p",
            GAME_PACKAGE,
            "-p",
            DUMP,
            "--target",
            TARGET,
        ])),
        Step::RemoveDir(layout.staging.clone()),
        Step::CreateDir(layout.macos()),
        Step::CreateDir(layout.resources()),
        Step::Write {
            path: layout.info_plist(),
            bytes: info_plist(spec, &host.min_system),
        },
        Step::Write {
            path: layout.icon(),
            bytes: ICON.to_vec(),
        },
        Step::Copy {
            from: release(EXECUTABLE),
            to: layout.executable(),
        },
        Step::Copy {
            from: release(DUMP),
            to: layout.dump(),
        },
        Step::CheckArchs(layout.executable()),
        Step::CheckArchs(layout.dump()),
        Step::Copy {
            from: spec.workspace_root.join("packaging").join("README.txt"),
            to: layout.staging.join("README.txt"),
        },
        Step::Copy {
            from: spec.workspace_root.join("LICENSE"),
            to: layout.staging.join("LICENSE"),
        },
        // Drag-install: the volume shows Applications beside the app.
        Step::Symlink {
            target: PathBuf::from("/Applications"),
            link: layout.staging.join("Applications"),
        },
    ]
}

/// Builds the `.dmg`, returning its path.
pub fn bundle(
    process: &mut impl Process,
    files: &mut impl Files,
    host: &Host,
) -> Result<PathBuf, BundleError> {
    let spec = BundleSpec::from_metadata(&run(process, &metadata(&host.cargo))?)?;
    let layout = Layout::new(&spec.target_dir, host.out.clone())?;
    let plan = plan(&spec, &layout, host);
    let done = plan
        .steps
        .iter()
        .try_for_each(|step| execute(step, process, files));
    let mut cleaned = Ok(());
    for step in &plan.cleanup {
        if let Err(e) = execute(step, process, files) {
            cleaned = cleaned.and(Err(e));
        }
    }
    done.and(cleaned).map(|()| layout.dmg())
}

/// Runs `lipo -archs` on `binary` and fails unless it lists [`ARCH`] and
/// nothing else: a universal or Intel binary stops the run before signing.
pub fn check_archs(process: &mut impl Process, binary: &Path) -> Result<(), BundleError> {
    let invocation = Invocation::new("lipo").arg("-archs").arg(binary).captured();
    let stdout = run(process, &invocation)?;
    let found = String::from_utf8_lossy(&stdout);
    if found.split_whitespace().eq([ARCH]) {
        Ok(())
    } else {
        Err(BundleError::NotArm64 {
            command: invocation.to_string(),
            found: found.trim().to_owned(),
        })
    }
}

pub(crate) fn run(
    process: &mut impl Process,
    invocation: &Invocation,
) -> Result<Vec<u8>, BundleError> {
    process.run(invocation).map_err(|source| BundleError::Run {
        command: invocation.to_string(),
        source,
    })
}

fn file(action: &'static str, path: &Path, result: io::Result<()>) -> Result<(), BundleError> {
    result.map_err(|source| BundleError::File {
        action,
        path: path.to_owned(),
        source,
    })
}

fn execute(
    step: &Step,
    process: &mut impl Process,
    files: &mut impl Files,
) -> Result<(), BundleError> {
    match step {
        Step::RemoveDir(path) => file("remove", path, files.remove_dir_all(path)),
        Step::CreateDir(path) => file("create", path, files.create_dir_all(path)),
        Step::Write { path, bytes } => file("write", path, files.write(path, bytes)),
        Step::WriteSecret { path, bytes } => {
            file("write", path, files.write_secret(path, &bytes.0))
        }
        Step::Copy { from, to } => file("copy to", to, files.copy(from, to)),
        Step::Symlink { target, link } => file("link", link, files.symlink(target, link)),
        Step::Run(invocation) => run(process, invocation).map(drop),
        Step::CheckArchs(binary) => check_archs(process, binary),
        Step::AddToSearchList(keychain) => add_to_search_list(process, keychain),
        Step::Notarize(notarization) => notarize(process, notarization),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{self, Event, FakeFiles, Log, Run, failing, is, run_ok_with};

    fn host(out: Option<PathBuf>) -> Host {
        Host {
            cargo: OsString::from("/rust/bin/cargo"),
            min_system: "13.0".into(),
            out,
            signing: Signing::AdHoc,
        }
    }

    fn run_with(
        host: &Host,
        archs: &[u8],
        process_fails: impl Fn(&Invocation) -> bool + 'static,
        file_fails: impl Fn(&Event) -> bool + 'static,
    ) -> Run {
        test_support::run(host, archs, failing(process_fails), file_fails)
    }

    fn run_ok(host: &Host) -> Run {
        run_ok_with(host)
    }

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    /// `program` with the whitespace-separated `words`, then `paths`.
    fn cmd(program: &str, words: &str, paths: &[&Path]) -> Invocation {
        Invocation::new(program)
            .args(words.split_whitespace())
            .args(paths)
    }

    #[test]
    fn runs_the_whole_sequence_in_order() {
        let root = Path::new("/work").join("ev-nova");
        let target = root.join("target");
        let staging = target.join("bundle").join("staging");
        let out = target.join("bundle").join("out");
        let app = staging.join("Nova.app");
        let contents = app.join("Contents");
        let exe = contents.join("MacOS").join("nova");
        let dump = staging.join("nova-dump");
        let dmg = out.join("ev-nova-aarch64-apple-darwin.dmg");
        let arm = |b: &str| target.join("aarch64-apple-darwin").join("release").join(b);
        let cargo = "/rust/bin/cargo";
        let sign = "--force --sign -";
        let verify = "--verify --deep --strict --verbose=2";

        let run = run_ok(&host(None));

        assert_eq!(run.result.as_ref().expect("bundled"), &dmg);
        let expected = vec![
            Event::Run(
                cmd(cargo, "metadata --format-version 1 --no-deps --locked", &[]).captured(),
            ),
            Event::Run(cmd(
                cargo,
                "build --release --locked -p nova -p nova-dump --target aarch64-apple-darwin",
                &[],
            )),
            Event::RemoveDir(staging.clone()),
            Event::CreateDir(contents.join("MacOS")),
            Event::CreateDir(contents.join("Resources")),
            Event::Write(contents.join("Info.plist"), info_plist(&spec(), "13.0")),
            Event::Write(contents.join("Resources").join("Nova.icns"), ICON.to_vec()),
            Event::Copy(arm("nova"), exe.clone()),
            Event::Copy(arm("nova-dump"), dump.clone()),
            Event::Run(cmd("lipo", "-archs", &[&exe]).captured()),
            Event::Run(cmd("lipo", "-archs", &[&dump]).captured()),
            Event::Copy(
                root.join("packaging").join("README.txt"),
                staging.join("README.txt"),
            ),
            Event::Copy(root.join("LICENSE"), staging.join("LICENSE")),
            Event::Symlink(PathBuf::from("/Applications"), staging.join("Applications")),
            Event::Run(cmd("codesign", sign, &[&dump])),
            Event::Run(cmd("codesign", sign, &[&app])),
            Event::Run(cmd("codesign", verify, &[&app])),
            Event::Run(cmd("codesign", verify, &[&dump])),
            Event::CreateDir(out.clone()),
            Event::Run(
                Invocation::new("hdiutil")
                    .args(["create", "-volname", "EV Nova", "-srcfolder"])
                    .arg(&staging)
                    .args(["-fs", "HFS+", "-format", "UDZO", "-ov"])
                    .arg(&dmg),
            ),
        ];
        assert_eq!(run.events, expected);
    }

    #[test]
    fn ad_hoc_runs_no_developer_id_tool_and_cleans_nothing_up() {
        let layout = Layout::new(&spec().target_dir, None).expect("layout");
        assert_eq!(plan(&spec(), &layout, &host(None)).cleanup, []);
        let run = run_ok(&host(None));
        assert!(run.result.is_ok(), "{:?}", run.result);
        for program in ["security", "xcrun", "ditto"] {
            assert!(!run.ran(program), "{program}");
        }
        assert!(
            !run.events
                .iter()
                .any(|e| matches!(e, Event::WriteSecret(..))),
        );
    }

    #[test]
    fn the_icon_is_where_the_plist_names_it() {
        let layout = Layout::new(&spec().target_dir, None).expect("layout");
        let all = plan(&spec(), &layout, &host(None)).steps;
        let plist = all
            .iter()
            .find_map(|s| match s {
                Step::Write { path, bytes } if *path == layout.info_plist() => Some(bytes.clone()),
                _ => None,
            })
            .expect("the plist is written");
        let icon_file = plist::Value::from_reader_xml(plist.as_slice())
            .expect("plist")
            .into_dictionary()
            .expect("dict")
            .get("CFBundleIconFile")
            .and_then(plist::Value::as_string)
            .expect("icon file")
            .to_owned();
        assert!(all.contains(&Step::Write {
            path: layout.resources().join(icon_file),
            bytes: ICON.to_vec(),
        }));
    }

    #[test]
    fn writes_the_dmg_to_the_output_dir_given() {
        let dist = Path::new("/work").join("ev-nova").join("dist");
        let run = run_ok(&host(Some(dist.clone())));
        let dmg = dist.join("ev-nova-aarch64-apple-darwin.dmg");
        assert_eq!(run.result.as_ref().expect("bundled"), &dmg);
        let hdiutil = run.runs().pop().expect("a run");
        assert_eq!(hdiutil.program, "hdiutil");
        assert_eq!(hdiutil.args.last(), Some(&dmg.into_os_string()));
        assert!(run.events.contains(&Event::CreateDir(dist)));
    }

    #[test]
    fn the_minimum_system_reaches_the_plist() {
        let mut host = host(None);
        host.min_system = "14.1".into();
        let run = run_ok(&host);
        let plist = run
            .events
            .iter()
            .find_map(|e| match e {
                Event::Write(path, bytes) if path.ends_with("Info.plist") => Some(bytes.clone()),
                _ => None,
            })
            .expect("a plist");
        assert_eq!(plist, info_plist(&spec(), "14.1"));
    }

    #[test]
    fn an_output_dir_holding_staging_stops_before_building() {
        let target = Path::new("/work").join("ev-nova").join("target");
        let run = run_ok(&host(Some(target)));
        assert!(matches!(run.result, Err(BundleError::Layout(_))));
        assert_eq!(run.runs().len(), 1, "only cargo metadata ran");
    }

    #[test]
    fn a_failing_metadata_stops_everything() {
        let run = run_with(
            &host(None),
            b"arm64",
            is("/rust/bin/cargo", "metadata"),
            |_| false,
        );
        let Err(BundleError::Run { command, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(command, &metadata(&host(None).cargo).to_string());
        assert_eq!(run.events.len(), 1);
    }

    #[test]
    fn a_failing_build_stops_before_staging() {
        let run = run_with(
            &host(None),
            b"arm64",
            is("/rust/bin/cargo", "build"),
            |_| false,
        );
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(
            error.contains("/rust/bin/cargo build --release --locked"),
            "{error}"
        );
        assert!(error.contains("exited with code 1"), "{error}");
        assert_eq!(run.events.len(), 2, "nothing staged");
    }

    #[test]
    fn a_binary_not_arm64_alone_stops_before_signing() {
        for archs in [
            &b"arm64 x86_64\n"[..],
            b"x86_64 arm64\n",
            b"x86_64\n",
            b"arm64e\n",
            b"arm64 arm64\n",
            b"",
        ] {
            let found = String::from_utf8_lossy(archs);
            let run = run_with(&host(None), archs, |_| false, |_| false);
            let error = run.result.as_ref().expect_err("fails").to_string();
            let layout = Layout::new(&spec().target_dir, None).expect("layout");
            let command = Invocation::new("lipo")
                .arg("-archs")
                .arg(layout.executable())
                .to_string();
            assert_eq!(
                error,
                format!("`{command}` lists `{}`, not arm64 alone", found.trim()),
            );
            assert!(!run.ran("codesign"), "{found}");
            assert!(!run.ran("hdiutil"), "{found}");
        }
    }

    #[test]
    fn arm64_alone_passes_however_lipo_spaces_it() {
        for archs in [&b"arm64"[..], b"arm64\n", b"  arm64 \n"] {
            let run = run_with(&host(None), archs, |_| false, |_| false);
            assert!(run.result.is_ok(), "{:?}", run.result);
        }
    }

    /// Answers `lipo -archs` with arm64 for `nova`, but both slices for
    /// `nova-dump`.
    struct DumpUniversal(Log);
    impl Process for DumpUniversal {
        fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
            self.0.borrow_mut().push(Event::Run(invocation.clone()));
            let args: Vec<_> = invocation
                .args
                .iter()
                .map(|a| a.to_string_lossy())
                .collect();
            Ok(match args.first().map(AsRef::as_ref) {
                Some("metadata") => sample_metadata(),
                Some("-archs") if args[1].ends_with("nova-dump") => b"x86_64 arm64".to_vec(),
                Some("-archs") => b"arm64".to_vec(),
                _ => Vec::new(),
            })
        }
    }

    #[test]
    fn a_dump_not_arm64_alone_stops_too() {
        let log = Log::default();
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(|_| false),
        };
        let result = bundle(&mut DumpUniversal(Rc::clone(&log)), &mut files, &host(None));
        let error = result.expect_err("fails").to_string();
        assert!(error.contains("nova-dump` lists `x86_64 arm64`"), "{error}");
        assert!(
            !log.borrow()
                .iter()
                .any(|e| matches!(e, Event::Run(i) if i.program == "codesign"))
        );
    }

    #[test]
    fn a_failing_signature_check_never_makes_the_dmg() {
        let run = run_with(&host(None), b"arm64", is("codesign", "--verify"), |_| false);
        let error = run.result.as_ref().expect_err("fails").to_string();
        assert!(
            error.contains("codesign --verify --deep --strict"),
            "{error}"
        );
        assert!(!run.ran("hdiutil"));
    }

    #[test]
    fn a_failing_signature_stops_before_verifying() {
        let run = run_with(&host(None), b"arm64", is("codesign", "--force"), |_| false);
        assert!(run.result.is_err());
        assert_eq!(
            run.runs()
                .iter()
                .filter(|i| i.program == "codesign")
                .count(),
            1
        );
    }

    #[test]
    fn a_failing_file_step_names_the_path_and_stops() {
        let run = run_with(
            &host(None),
            b"arm64",
            |_| false,
            |e| matches!(e, Event::Write(..)),
        );
        let layout = Layout::new(&spec().target_dir, None).expect("layout");
        let Err(BundleError::File { path, action, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(path, &layout.info_plist());
        assert_eq!(*action, "write");
        assert!(!run.ran("lipo"));
    }

    #[test]
    fn each_file_step_s_failure_is_reported() {
        type Fails = fn(&Event) -> bool;
        let cases: [(Fails, &str); 5] = [
            (|e| matches!(e, Event::RemoveDir(_)), "remove"),
            (|e| matches!(e, Event::CreateDir(_)), "create"),
            (|e| matches!(e, Event::Write(..)), "write"),
            (|e| matches!(e, Event::Copy(..)), "copy to"),
            (|e| matches!(e, Event::Symlink(..)), "link"),
        ];
        for (fails, expected) in cases {
            let run = run_with(&host(None), b"arm64", |_| false, fails);
            let Err(BundleError::File { action, .. }) = &run.result else {
                panic!("{expected}: {:?}", run.result);
            };
            assert_eq!(*action, expected);
        }
    }
}
