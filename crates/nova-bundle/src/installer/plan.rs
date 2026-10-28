//! The installer's sequence: read the spec, check the executables are
//! there, download and check WinSparkle ([`winsparkle::stage`]), stage the
//! icon and the WiX source, and run `wix build`.
//!
//! [`plan`] is the sequence as data; [`pack`] runs it through the ports,
//! stopping at the first failure. Neither builds anything: the
//! executables are packed as they are.

use std::ffi::OsStr;
use std::path::PathBuf;

use nova_winres::ico_from_icns;

use super::host::InstallerHost;
use super::layout::InstallerLayout;
use super::wxs::wxs;
use super::{EXES, InstallerError, winsparkle};
use crate::appcast::Platform;
use crate::bundle::{self, BundleError, Step, execute, metadata};
use crate::channel::{FeedStatus, feed_status};
use crate::icon::ICON;
use crate::ports::{Files, Invocation, Process};
use crate::spec::BundleSpec;

/// `wix build`: the staged source into the `.msi`, for 64-bit Windows,
/// with no `.wixpdb` beside it (the output directory is uploaded as it
/// is).
#[must_use]
pub fn wix_build(wix: &OsStr, layout: &InstallerLayout) -> Invocation {
    Invocation::new(wix)
        .args(["build", "-arch", "x64", "-pdbtype", "none", "-o"])
        .arg(layout.msi())
        .arg(layout.wxs())
}

/// The steps that pack `spec`'s installer under `layout` with `wix`.
pub fn plan(
    spec: &BundleSpec,
    layout: &InstallerLayout,
    wix: &OsStr,
) -> Result<Vec<Step>, InstallerError> {
    let source = wxs(spec, layout)?;
    // The same icon nova.exe embeds, made from the same `.icns`.
    let icon = ico_from_icns(ICON)?;
    let mut steps = vec![
        Step::RemoveDir(layout.staging.clone()),
        Step::CreateDir(layout.staging.clone()),
        Step::CreateDir(layout.out.clone()),
    ];
    steps.extend(winsparkle::stage(layout));
    steps.extend([
        Step::Write {
            path: layout.icon(),
            bytes: icon,
        },
        Step::Write {
            path: layout.wxs(),
            bytes: source.into_bytes(),
        },
        Step::Run(wix_build(wix, layout)),
    ]);
    Ok(steps)
}

/// A packed installer: the `.msi`, and the update feed the build names,
/// or why it names none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packed {
    /// The `.msi`.
    pub msi: PathBuf,
    /// What nova's build script compiled into `nova.exe` from the same
    /// channel and key (`nova_winres::windows_update_feed`, pinned to
    /// [`feed_status`] by a test).
    pub feed: FeedStatus,
}

/// Packs the installer. Fails before staging anything when an executable
/// is missing from the executables directory.
pub fn pack(
    process: &mut impl Process,
    files: &mut impl Files,
    host: &InstallerHost,
) -> Result<Packed, InstallerError> {
    let json = bundle::run(process, &metadata(&host.cargo))?;
    let spec = BundleSpec::from_metadata(&json).map_err(BundleError::from)?;
    let layout = InstallerLayout::new(&spec.target_dir, host.out.clone(), host.exes.clone())?;
    if let Some(missing) = EXES
        .iter()
        .map(|exe| layout.exe(exe))
        .find(|exe| !files.is_file(exe))
    {
        return Err(InstallerError::MissingExe(missing));
    }
    for step in plan(&spec, &layout, &host.wix)? {
        execute(&step, process, files)?;
    }
    Ok(Packed {
        msi: layout.msi(),
        feed: feed_status(&spec, host.channel, Platform::Windows),
    })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::rc::Rc;

    use super::*;
    use crate::appcast::Channel;
    use crate::appcast::key::PublicKey;
    use crate::channel::{UpdateFeed, feed_url};
    use crate::digest::DigestTool;
    use crate::ports::RunError;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{Event, FakeFiles, Log, Recorder, failing, is};

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    fn host() -> InstallerHost {
        InstallerHost {
            cargo: "C:/rust/cargo.exe".into(),
            wix: "wix".into(),
            exes: None,
            out: None,
            channel: None,
        }
    }

    struct PackRun {
        result: Result<Packed, InstallerError>,
        events: Vec<Event>,
    }

    impl PackRun {
        fn runs(&self) -> Vec<Invocation> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    Event::Run(i) => Some(i.clone()),
                    _ => None,
                })
                .collect()
        }
    }

    fn pack_with(
        host: &InstallerHost,
        respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
        absent: impl Fn(&Path) -> bool + 'static,
    ) -> PackRun {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(respond),
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(|_| false),
            absent: Box::new(absent),
        };
        let result = pack(&mut process, &mut files, host);
        let events = log.borrow().clone();
        PackRun { result, events }
    }

    /// Answers `certutil` with another digest for the file named `name`.
    fn another_digest_for(
        name: &'static str,
    ) -> impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static {
        move |i| {
            (i.program == "certutil" && i.args.get(1).is_some_and(|p| Path::new(p).ends_with(name)))
                .then(|| Ok(format!("hash:\r\n{}\r\nok\r\n", "0".repeat(64)).into_bytes()))
        }
    }

    #[test]
    fn plans_staging_the_icon_and_source_then_wix_build() {
        let spec = spec();
        let layout = InstallerLayout::new(&spec.target_dir, None, None).expect("layout");
        let steps = plan(&spec, &layout, OsStr::new("wix")).expect("plan");
        let staging = spec.target_dir.join("installer").join("staging");
        let out = spec.target_dir.join("installer").join("out");
        let mut expected = vec![
            Step::RemoveDir(staging.clone()),
            Step::CreateDir(staging.clone()),
            Step::CreateDir(out.clone()),
        ];
        // WinSparkle, downloaded and checked before anything names it.
        expected.extend(winsparkle::stage(&layout));
        assert_eq!(steps[..expected.len()], expected);
        assert_eq!(
            steps[expected.len()..],
            vec![
                Step::Write {
                    path: staging.join("nova.ico"),
                    bytes: ico_from_icns(ICON).expect("ico"),
                },
                Step::Write {
                    path: staging.join("nova.wxs"),
                    bytes: wxs(&spec, &layout).expect("wxs").into_bytes(),
                },
                Step::Run(
                    Invocation::new("wix")
                        .args(["build", "-arch", "x64", "-pdbtype", "none", "-o"])
                        .arg(out.join("ev-nova-x86_64-pc-windows-msvc.msi"))
                        .arg(staging.join("nova.wxs"))
                ),
            ]
        );
    }

    #[test]
    fn the_wix_given_is_the_one_run() {
        let spec = spec();
        let layout = InstallerLayout::new(&spec.target_dir, None, None).expect("layout");
        let steps = plan(&spec, &layout, OsStr::new("D:/wix/wix.exe")).expect("plan");
        let Some(Step::Run(invocation)) = steps.last() else {
            panic!("{steps:?}");
        };
        assert_eq!(invocation.program, "D:/wix/wix.exe");
    }

    #[test]
    fn a_version_an_msi_cannot_hold_plans_nothing() {
        let mut spec = spec();
        spec.version = "0.300.0".into();
        let layout = InstallerLayout::new(&spec.target_dir, None, None).expect("layout");
        assert!(matches!(
            plan(&spec, &layout, OsStr::new("wix")),
            Err(InstallerError::VersionTooLarge(_))
        ));
    }

    #[test]
    fn packs_with_metadata_then_the_plan_and_never_builds() {
        let packed = pack_with(&host(), |_| None, |_| false);
        let spec = spec();
        let layout = InstallerLayout::new(&spec.target_dir, None, None).expect("layout");
        assert_eq!(
            packed.result.as_ref().expect("packed"),
            &Packed {
                msi: layout.msi(),
                feed: FeedStatus::Development,
            }
        );
        let runs = packed.runs();
        let programs: Vec<_> = runs
            .iter()
            .map(|i| i.program.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            programs,
            [
                "C:/rust/cargo.exe",
                "curl",
                "certutil",
                "tar",
                "tar",
                "certutil",
                "wix"
            ]
        );
        assert_eq!(runs[0], metadata(&OsString::from("C:/rust/cargo.exe")));
        assert_eq!(
            runs[2],
            DigestTool::Certutil.invocation(&layout.winsparkle_archive())
        );
        assert_eq!(
            runs[5],
            DigestTool::Certutil.invocation(&layout.winsparkle_dll())
        );
        assert_eq!(runs[6], wix_build(OsStr::new("wix"), &layout));
        assert!(
            !runs
                .iter()
                .any(|i| i.args.first().is_some_and(|a| a == "build") && i.program != "wix"),
            "{runs:?}"
        );
        let writes: Vec<_> = packed
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Write(path, _) => Some(path.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(writes, [layout.icon(), layout.wxs()]);
    }

    #[test]
    fn the_executables_dir_given_reaches_the_source_unchanged() {
        let exes = Path::new("C:/signed").join("exes");
        let out = Path::new("C:/work").join("dist");
        let host = InstallerHost {
            exes: Some(exes.clone()),
            out: Some(out.clone()),
            ..host()
        };
        let checked = Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = Rc::clone(&checked);
        let packed = pack_with(
            &host,
            |_| None,
            move |p| {
                seen.borrow_mut().push(p.to_owned());
                false
            },
        );
        assert_eq!(
            packed.result.as_ref().expect("packed").msi,
            out.join("ev-nova-x86_64-pc-windows-msvc.msi")
        );
        assert_eq!(
            *checked.borrow(),
            [exes.join("nova.exe"), exes.join("nova-dump.exe")]
        );
        let spec = spec();
        let layout = InstallerLayout::new(&spec.target_dir, Some(out), Some(exes)).expect("layout");
        let source = packed
            .events
            .iter()
            .find_map(|e| match e {
                Event::Write(path, bytes) if *path == layout.wxs() => Some(bytes.clone()),
                _ => None,
            })
            .expect("the source is written");
        assert_eq!(source, wxs(&spec, &layout).expect("wxs").into_bytes());
    }

    #[test]
    fn a_missing_executable_is_an_error_before_anything_is_staged() {
        for missing in ["nova.exe", "nova-dump.exe"] {
            let packed = pack_with(&host(), |_| None, move |p| p.ends_with(missing));
            let layout = InstallerLayout::new(&spec().target_dir, None, None).expect("layout");
            let Err(InstallerError::MissingExe(path)) = &packed.result else {
                panic!("{missing}: {:?}", packed.result);
            };
            assert_eq!(path, &layout.exe(missing));
            let message = packed.result.as_ref().expect_err("fails").to_string();
            assert!(message.contains("mise run build-windows"), "{message}");
            assert_eq!(packed.runs().len(), 1, "only cargo metadata ran");
            assert!(
                packed.events.iter().all(|e| matches!(e, Event::Run(_))),
                "{:?}",
                packed.events
            );
        }
    }

    #[test]
    fn a_failing_wix_names_its_command_line() {
        let packed = pack_with(&host(), failing(is("wix", "build")), |_| false);
        let message = packed.result.as_ref().expect_err("fails").to_string();
        assert!(message.contains("`wix build -arch x64"), "{message}");
        assert!(message.contains("exited with code 1"), "{message}");
    }

    #[test]
    fn a_failing_metadata_stages_nothing() {
        let packed = pack_with(
            &host(),
            failing(is("C:/rust/cargo.exe", "metadata")),
            |_| false,
        );
        assert!(matches!(
            packed.result,
            Err(InstallerError::Step(BundleError::Run { .. }))
        ));
        assert_eq!(packed.events.len(), 1);
    }

    #[test]
    fn unreadable_metadata_is_a_spec_error() {
        let packed = pack_with(
            &host(),
            |i| starts_metadata(i).then(|| Ok(b"not json".to_vec())),
            |_| false,
        );
        assert!(matches!(
            packed.result,
            Err(InstallerError::Step(BundleError::Spec(_)))
        ));
    }

    fn starts_metadata(invocation: &Invocation) -> bool {
        invocation.args.first().is_some_and(|a| a == "metadata")
    }

    #[test]
    fn a_failing_file_step_stops_before_wix() {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond: Box::new(|_| None),
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(|e| matches!(e, Event::Write(..))),
            absent: Box::new(|_| false),
        };
        let result = pack(&mut process, &mut files, &host());
        assert!(matches!(
            result,
            Err(InstallerError::Step(BundleError::File {
                action: "write",
                ..
            }))
        ));
        assert!(
            !log.borrow()
                .iter()
                .any(|e| matches!(e, Event::Run(i) if i.program == "wix"))
        );
    }

    #[test]
    fn an_output_dir_overlapping_staging_stages_nothing() {
        let host = InstallerHost {
            out: Some(spec().target_dir.join("installer")),
            ..host()
        };
        let packed = pack_with(&host, |_| None, |_| false);
        assert!(matches!(packed.result, Err(InstallerError::Layout(_))));
        assert_eq!(packed.events.len(), 1);
    }

    /// The pin a run stopped at, if it stopped at a digest check.
    fn checksum_pin(run: &PackRun) -> Option<&'static str> {
        match &run.result {
            Err(InstallerError::Step(BundleError::Checksum { pinned, .. })) => Some(pinned),
            _ => None,
        }
    }

    #[test]
    fn an_archive_with_another_digest_stops_before_unpacking_or_wix() {
        let packed = pack_with(&host(), another_digest_for(winsparkle::ARCHIVE), |_| false);
        assert_eq!(checksum_pin(&packed), Some(winsparkle::ARCHIVE_PIN.what));
        let ran = |program: &str| packed.runs().iter().any(|i| i.program == program);
        assert!(ran("curl") && ran("certutil"));
        assert!(!ran("tar") && !ran("wix"), "{:?}", packed.runs());
    }

    #[test]
    fn a_dll_with_another_digest_stops_before_wix() {
        let packed = pack_with(&host(), another_digest_for(winsparkle::DLL), |_| false);
        assert_eq!(checksum_pin(&packed), Some(winsparkle::DLL_PIN.what));
        let runs = packed.runs();
        assert_eq!(runs.iter().filter(|i| i.program == "tar").count(), 2);
        assert!(!runs.iter().any(|i| i.program == "wix"), "{runs:?}");
        assert!(
            !packed.events.iter().any(|e| matches!(e, Event::Write(..))),
            "no source written"
        );
    }

    #[test]
    fn a_failing_download_stops_before_the_digest_check() {
        let packed = pack_with(&host(), failing(|i| i.program == "curl"), |_| false);
        let Err(InstallerError::Step(BundleError::Run { command, .. })) = &packed.result else {
            panic!("{:?}", packed.result);
        };
        assert!(command.starts_with("curl "), "{command}");
        assert!(
            !packed.runs().iter().any(|i| i.program == "certutil"),
            "{:?}",
            packed.runs()
        );
    }

    #[test]
    fn the_channel_is_carried_through() {
        let nightly = InstallerHost {
            channel: Some(Channel::Nightly),
            ..host()
        };
        for (host, feed) in [
            (host(), FeedStatus::Development),
            (nightly, FeedStatus::NoKey(Channel::Nightly)),
        ] {
            let packed = pack_with(&host, |_| None, |_| false);
            assert_eq!(packed.result.expect("packed").feed, feed);
        }
    }

    #[test]
    fn a_nightly_pack_without_a_key_says_it_has_no_feed() {
        let host = InstallerHost {
            channel: Some(Channel::Nightly),
            ..host()
        };
        let packed = pack_with(&host, |_| None, |_| false);
        assert_eq!(
            packed.result.expect("packed").feed.describe(),
            "no update feed: NOVA_CHANNEL is nightly but the nova package commits no \
             update_public_key"
        );
    }

    #[test]
    fn a_nightly_pack_with_a_key_names_the_windows_feed() {
        let key = PublicKey([5; 32]);
        let mut metadata: serde_json::Value =
            serde_json::from_slice(&sample_metadata()).expect("json");
        let game = metadata["packages"]
            .as_array_mut()
            .expect("packages")
            .iter_mut()
            .find(|p| p["name"] == "nova")
            .expect("the game");
        game["metadata"]["bundle"]["update_public_key"] = key.to_base64().into();
        let json = serde_json::to_vec(&metadata).expect("json");
        let host = InstallerHost {
            channel: Some(Channel::Nightly),
            ..host()
        };
        let packed = pack_with(
            &host,
            move |i| starts_metadata(i).then(|| Ok(json.clone())),
            |_| false,
        );
        assert_eq!(
            packed.result.expect("packed").feed,
            FeedStatus::Feed(UpdateFeed {
                url: feed_url(Platform::Windows, Channel::Nightly),
                public_key: key,
            })
        );
    }
}
