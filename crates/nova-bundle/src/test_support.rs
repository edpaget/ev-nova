//! Test doubles shared by the `bundle`, `signing` and `notary` tests: a
//! recording [`Process`] and fake [`Files`] writing to one event log, so a
//! test sees every run and file step in the order the core asked for them.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::bundle::{BundleError, Host, bundle};
use crate::ports::{Files, Invocation, Process, RunError};
use crate::signing::{Credentials, Signing};
use crate::spec::BundleSpec;
use crate::spec::tests::sample_metadata;

/// Everything the ports were asked to do, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Run(Invocation),
    RemoveDir(PathBuf),
    CreateDir(PathBuf),
    Write(PathBuf, Vec<u8>),
    WriteSecret(PathBuf, Vec<u8>),
    Copy(PathBuf, PathBuf),
    Symlink(PathBuf, PathBuf),
}

pub(crate) type Log = Rc<RefCell<Vec<Event>>>;

/// A run's outcome chosen by the test, or `None` for [`Recorder`]'s
/// default.
pub(crate) type Respond = Box<dyn Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>>>;

/// What `security list-keychains -d user` prints: indented, quoted paths.
pub(crate) const KEYCHAINS: &[u8] = b"    \"/Users/runner/Library/Keychains/login.keychain-db\"\n    \"/Library/Keychains/System.keychain\"\n";

/// The paths in [`KEYCHAINS`], in order.
pub(crate) const KEYCHAIN_PATHS: [&str; 2] = [
    "/Users/runner/Library/Keychains/login.keychain-db",
    "/Library/Keychains/System.keychain",
];

/// Records each run. Answers with `respond`'s outcome when it gives one,
/// else with canned output: the spec for `cargo metadata`, `archs` for
/// `lipo -archs`, [`KEYCHAINS`] for `security list-keychains -d user`,
/// `{"id":"abc"}` for `notarytool submit` and `{"status":"Accepted"}` for
/// `notarytool info`.
pub(crate) struct Recorder {
    pub log: Log,
    pub archs: Vec<u8>,
    pub respond: Respond,
}

impl Process for Recorder {
    fn run(&mut self, invocation: &Invocation) -> Result<Vec<u8>, RunError> {
        self.log.borrow_mut().push(Event::Run(invocation.clone()));
        if let Some(outcome) = (self.respond)(invocation) {
            return outcome;
        }
        let args: Vec<_> = invocation
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        Ok(match args.as_slice() {
            ["metadata", ..] => sample_metadata(),
            ["-archs", ..] => self.archs.clone(),
            ["list-keychains", "-d", "user"] => KEYCHAINS.to_vec(),
            ["notarytool", "submit", ..] => br#"{"id":"abc","message":"ok"}"#.to_vec(),
            ["notarytool", "info", ..] => br#"{"id":"abc","status":"Accepted"}"#.to_vec(),
            _ => Vec::new(),
        })
    }
}

/// Records each file operation; fails the ones `fails` picks.
pub(crate) struct FakeFiles {
    pub log: Log,
    pub fails: Box<dyn Fn(&Event) -> bool>,
}

impl FakeFiles {
    fn record(&self, event: Event) -> io::Result<()> {
        let failed = (self.fails)(&event);
        self.log.borrow_mut().push(event);
        if failed {
            Err(io::Error::other("disk full"))
        } else {
            Ok(())
        }
    }
}

impl Files for FakeFiles {
    fn remove_dir_all(&mut self, path: &Path) -> io::Result<()> {
        self.record(Event::RemoveDir(path.to_owned()))
    }
    fn create_dir_all(&mut self, path: &Path) -> io::Result<()> {
        self.record(Event::CreateDir(path.to_owned()))
    }
    fn write(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.record(Event::Write(path.to_owned(), bytes.to_vec()))
    }
    fn copy(&mut self, from: &Path, to: &Path) -> io::Result<()> {
        self.record(Event::Copy(from.to_owned(), to.to_owned()))
    }
    fn symlink(&mut self, target: &Path, link: &Path) -> io::Result<()> {
        self.record(Event::Symlink(target.to_owned(), link.to_owned()))
    }
    fn write_secret(&mut self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.record(Event::WriteSecret(path.to_owned(), bytes.to_vec()))
    }
}

/// A finished `bundle` run.
pub(crate) struct Run {
    pub result: Result<PathBuf, BundleError>,
    pub events: Vec<Event>,
}

impl Run {
    /// Every program run, in order.
    pub fn runs(&self) -> Vec<Invocation> {
        self.events
            .iter()
            .filter_map(|e| match e {
                Event::Run(invocation) => Some(invocation.clone()),
                _ => None,
            })
            .collect()
    }

    /// Whether `program` ran at all.
    pub fn ran(&self, program: &str) -> bool {
        self.runs().iter().any(|i| i.program == program)
    }

    /// Whether a run had these leading arguments.
    pub fn ran_args(&self, program: &str, leading: &[&str]) -> bool {
        self.runs().iter().any(|i| starts(i, program, leading))
    }
}

/// Whether `invocation` is `program` with these leading arguments.
pub(crate) fn starts(invocation: &Invocation, program: &str, leading: &[&str]) -> bool {
    invocation.program == program
        && invocation.args.len() >= leading.len()
        && invocation.args.iter().zip(leading).all(|(a, l)| a == l)
}

/// Runs `bundle` over the doubles.
pub(crate) fn run(
    host: &Host,
    archs: &[u8],
    respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
    file_fails: impl Fn(&Event) -> bool + 'static,
) -> Run {
    let log = Log::default();
    let mut process = Recorder {
        log: Rc::clone(&log),
        archs: archs.to_vec(),
        respond: Box::new(respond),
    };
    let mut files = FakeFiles {
        log: Rc::clone(&log),
        fails: Box::new(file_fails),
    };
    let result = bundle(&mut process, &mut files, host);
    let events = log.borrow().clone();
    Run { result, events }
}

/// A [`Respond`] failing, with exit code 1, the runs `fails` picks.
pub(crate) fn failing(
    fails: impl Fn(&Invocation) -> bool + 'static,
) -> impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static {
    move |i| fails(i).then_some(Err(RunError::Failed(Some(1))))
}

/// Picks `program` run with `first` as its first argument.
pub(crate) fn is(program: &'static str, first: &'static str) -> impl Fn(&Invocation) -> bool {
    move |i| starts(i, program, &[first])
}

/// Runs `bundle` with every run and file step succeeding and arm64
/// binaries.
pub(crate) fn run_ok_with(host: &Host) -> Run {
    run(host, b"arm64\n", |_| None, |_| false)
}

/// The certificate's bytes in [`credentials`].
pub(crate) const P12: &[u8] = b"p12 bytes\x00\xff";
/// The API key's bytes in [`credentials`].
pub(crate) const P8: &[u8] = b"-----BEGIN PRIVATE KEY-----";

/// Developer ID inputs, none of whose values holds another.
pub(crate) fn credentials() -> Credentials {
    Credentials {
        identity: "Developer ID Application: Ed (TEAM123456)".into(),
        cert_p12: P12.to_vec(),
        cert_password: "pw-secret-value".into(),
        api_key_p8: P8.to_vec(),
        api_key_id: "KEYID12345".into(),
        api_issuer: "issuer-uuid".into(),
    }
}

/// A host signing with Developer ID with [`credentials`], writing to the
/// default output directory.
pub(crate) fn developer_id_host() -> Host {
    Host {
        cargo: "/rust/bin/cargo".into(),
        min_system: "13.0".into(),
        out: None,
        signing: Signing::DeveloperId(credentials()),
    }
}

/// The same host, signing ad-hoc.
pub(crate) fn ad_hoc_host() -> Host {
    Host {
        signing: Signing::AdHoc,
        ..developer_id_host()
    }
}

/// The paths a run of [`developer_id_host`] uses, under `/work/ev-nova`.
pub(crate) struct Paths {
    pub app: PathBuf,
    pub dump: PathBuf,
    pub dmg: PathBuf,
    pub out: PathBuf,
    pub signing: PathBuf,
    pub keychain: PathBuf,
    pub cert: PathBuf,
    pub key: PathBuf,
    pub zip: PathBuf,
}

pub(crate) fn paths() -> Paths {
    // Cargo's own spelling of the target dir, so the paths in each command's
    // args (text, not paths) match on Windows, where `join` writes `\`.
    let target = BundleSpec::from_metadata(&sample_metadata())
        .expect("spec")
        .target_dir;
    let bundle = target.join("bundle");
    let staging = bundle.join("staging");
    let signing = bundle.join("signing");
    let out = bundle.join("out");
    Paths {
        app: staging.join("Nova.app"),
        dump: staging.join("nova-dump"),
        dmg: out.join("ev-nova-aarch64-apple-darwin.dmg"),
        out,
        keychain: signing.join("nova-signing.keychain-db"),
        cert: signing.join("developer-id.p12"),
        key: signing.join("AuthKey_KEYID12345.p8"),
        zip: signing.join("Nova.zip"),
        signing,
    }
}
