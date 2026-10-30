//! The `share/` tree's layout under the output directory, and writing it.

use std::path::{Path, PathBuf};

use super::desktop::{desktop_entry, desktop_file_name};
use super::host::LinuxHost;
use super::icons::ICONS;
use super::metainfo::{metainfo, metainfo_file_name};
use crate::build_id::{self, BuildId};
use crate::bundle::{self, BundleError, Step, execute, metadata};
use crate::ports::{Env, Files, Process};
use crate::spec::BundleSpec;

/// The default output directory under Cargo's target directory.
#[must_use]
pub fn default_out(target_dir: &Path) -> PathBuf {
    target_dir.join("linux")
}

/// Every file of the tree for `spec`'s build `build` under `out`, each
/// with its contents: the desktop file, the metainfo, then each icon from
/// the smallest.
#[must_use]
pub fn files(spec: &BundleSpec, build: &BuildId, out: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let share = out.join("share");
    let icon_name = format!("{}.png", spec.identifier);
    let mut files = vec![
        (
            share.join("applications").join(desktop_file_name(spec)),
            desktop_entry(spec).into_bytes(),
        ),
        (
            share.join("metainfo").join(metainfo_file_name(spec)),
            metainfo(spec, build).into_bytes(),
        ),
    ];
    for icon in ICONS {
        let size = format!("{0}x{0}", icon.size);
        let path = share
            .join("icons")
            .join("hicolor")
            .join(size)
            .join("apps")
            .join(&icon_name);
        files.push((path, icon.png.to_vec()));
    }
    files
}

/// The steps that write [`files`]: each file's directory made (once),
/// then the file written. Nothing is removed first, so `out` may hold
/// other files (the tarball's binaries, or a `~/.local` tree).
#[must_use]
pub fn steps(spec: &BundleSpec, build: &BuildId, out: &Path) -> Vec<Step> {
    let mut made: Vec<PathBuf> = Vec::new();
    let mut steps = Vec::new();
    for (path, bytes) in files(spec, build, out) {
        if let Some(dir) = path.parent()
            && !made.iter().any(|m| m == dir)
        {
            made.push(dir.to_path_buf());
            steps.push(Step::CreateDir(dir.to_path_buf()));
        }
        steps.push(Step::Write { path, bytes });
    }
    steps
}

/// Reads the spec through `cargo metadata` and the build ID from `env` or
/// git ([`build_id::resolve`]), and writes the tree into the host's output
/// directory (by default [`default_out`]), returning it. The first step
/// to fail stops it.
pub fn write_share(
    process: &mut impl Process,
    files: &mut impl Files,
    env: &impl Env,
    host: &LinuxHost,
) -> Result<PathBuf, BundleError> {
    let spec = BundleSpec::from_metadata(&bundle::run(process, &metadata(&host.cargo))?)?;
    let build = build_id::resolve(env, process, &spec.workspace_root)?;
    let out = host
        .out
        .clone()
        .unwrap_or_else(|| default_out(&spec.target_dir));
    for step in steps(&spec, &build, &out) {
        execute(&step, process, files)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::spec::tests::sample_metadata;
    use crate::test_support::{Event, FakeEnv, FakeFiles, Log, Recorder, build, build_env};

    fn spec() -> BundleSpec {
        BundleSpec::from_metadata(&sample_metadata()).expect("spec")
    }

    /// Cargo's own spelling of the target directory, so the paths in the
    /// events (compared as paths) and in messages (text) match on Windows.
    fn target() -> PathBuf {
        spec().target_dir
    }

    fn share(out: &Path) -> PathBuf {
        out.join("share")
    }

    #[test]
    fn the_default_output_is_under_the_target_directory() {
        assert_eq!(default_out(&target()), target().join("linux"));
    }

    #[test]
    fn lays_out_the_desktop_file_the_metainfo_and_each_icon() {
        let out = target().join("out");
        let paths: Vec<PathBuf> = files(&spec(), &build(), &out)
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        let mut expected = vec![
            share(&out)
                .join("applications")
                .join("io.example.nova.desktop"),
            share(&out)
                .join("metainfo")
                .join("io.example.nova.metainfo.xml"),
        ];
        for n in [16, 24, 32, 48, 64, 128, 256, 512] {
            expected.push(
                share(&out)
                    .join("icons")
                    .join("hicolor")
                    .join(format!("{n}x{n}"))
                    .join("apps")
                    .join("io.example.nova.png"),
            );
        }
        assert_eq!(paths, expected);
    }

    #[test]
    fn each_file_holds_what_its_renderer_or_icon_gives() {
        let out = target().join("out");
        let files = files(&spec(), &build(), &out);
        assert_eq!(files[0].1, desktop_entry(&spec()).into_bytes());
        assert_eq!(files[1].1, metainfo(&spec(), &build()).into_bytes());
        for (file, icon) in files[2..].iter().zip(ICONS) {
            assert_eq!(file.1, icon.png, "{}", icon.size);
        }
        assert_eq!(files.len(), 2 + ICONS.len());
    }

    #[test]
    fn makes_each_directory_before_writing_into_it_and_removes_nothing() {
        let out = target().join("out");
        let steps = steps(&spec(), &build(), &out);
        let mut made: Vec<PathBuf> = Vec::new();
        let mut written = Vec::new();
        for step in &steps {
            match step {
                Step::CreateDir(dir) => made.push(dir.clone()),
                Step::Write { path, bytes } => {
                    let dir = path.parent().expect("a directory");
                    assert!(made.iter().any(|m| m == dir), "{} made", dir.display());
                    written.push((path.clone(), bytes.clone()));
                }
                other => panic!("an unexpected step: {other:?}"),
            }
        }
        assert_eq!(written, files(&spec(), &build(), &out));
        let mut unique = made.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), made.len(), "each directory made once");
    }

    fn host(out: Option<PathBuf>) -> LinuxHost {
        LinuxHost {
            cargo: "/rust/bin/cargo".into(),
            out,
        }
    }

    fn write_with(
        host: &LinuxHost,
        fails: impl Fn(&Event) -> bool + 'static,
    ) -> (Result<PathBuf, BundleError>, Vec<Event>) {
        write_in(&build_env(), host, Box::new(|_| None), fails)
    }

    fn write_in(
        env: &FakeEnv,
        host: &LinuxHost,
        respond: crate::test_support::Respond,
        fails: impl Fn(&Event) -> bool + 'static,
    ) -> (Result<PathBuf, BundleError>, Vec<Event>) {
        let log = Log::default();
        let mut process = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond,
        };
        let mut files = FakeFiles {
            log: Rc::clone(&log),
            fails: Box::new(fails),
            absent: Box::new(|_| false),
        };
        let result = write_share(&mut process, &mut files, env, host);
        let events = log.borrow().clone();
        (result, events)
    }

    #[test]
    fn reads_the_spec_then_writes_the_tree_into_the_default_output() {
        let (result, events) = write_with(&host(None), |_| false);
        let out = default_out(&target());
        assert_eq!(result.expect("written"), out);
        assert_eq!(events[0], Event::Run(metadata(&"/rust/bin/cargo".into())));
        let expected: Vec<Event> = steps(&spec(), &build(), &out)
            .into_iter()
            .map(|step| match step {
                Step::CreateDir(dir) => Event::CreateDir(dir),
                Step::Write { path, bytes } => Event::Write(path, bytes),
                other => panic!("an unexpected step: {other:?}"),
            })
            .collect();
        assert_eq!(events[1..], expected);
    }

    #[test]
    fn without_the_variable_the_metainfo_names_head_s_committer_time() {
        let respond = Box::new(|i: &crate::ports::Invocation| {
            (i.program == "git").then(|| Ok(b"981173106\n".to_vec()))
        });
        let (result, events) = write_in(&FakeEnv::default(), &host(None), respond, |_| false);
        let out = result.expect("written");
        assert_eq!(
            events[1],
            Event::Run(crate::build_id::committer_time(&spec().workspace_root))
        );
        let id = crate::build_id::BuildId::parse("20010203040506").expect("an ID");
        let path = share(&out)
            .join("metainfo")
            .join("io.example.nova.metainfo.xml");
        assert!(events.contains(&Event::Write(path, metainfo(&spec(), &id).into_bytes())));
    }

    #[test]
    fn a_failing_build_id_writes_nothing() {
        let env = FakeEnv(std::collections::HashMap::from([(
            crate::build_id::VAR,
            "bad".into(),
        )]));
        let (result, events) = write_in(&env, &host(None), Box::new(|_| None), |_| false);
        assert!(matches!(result, Err(BundleError::BuildId(_))), "{result:?}");
        assert_eq!(events.len(), 1, "only cargo metadata ran: {events:?}");
    }

    #[test]
    fn writes_into_the_output_directory_asked_for() {
        let out = Path::new("/elsewhere").join("dist");
        let (result, events) = write_with(&host(Some(out.clone())), |_| false);
        assert_eq!(result.expect("written"), out);
        assert!(events.contains(&Event::CreateDir(share(&out).join("applications"))));
    }

    #[test]
    fn a_failed_write_stops_it_naming_the_file() {
        let (result, events) = write_with(&host(None), |e| matches!(e, Event::Write(..)));
        let desktop = share(&default_out(&target()))
            .join("applications")
            .join("io.example.nova.desktop");
        let error = result.expect_err("fails");
        assert!(
            matches!(&error, BundleError::File { path, .. } if *path == desktop),
            "{error}"
        );
        let writes = events
            .iter()
            .filter(|e| matches!(e, Event::Write(..)))
            .count();
        assert_eq!(writes, 1, "stops at the first failure");
    }
}
