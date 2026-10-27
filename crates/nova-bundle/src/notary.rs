//! Notarization: `xcrun notarytool` sends a signed file to Apple's notary
//! service and waits for its verdict.
//!
//! `notarytool submit --wait` doesn't reliably exit non-zero on a rejected
//! submission, and a failed captured run loses its output, so the steps
//! are separate: `submit` (its JSON gives the submission id), `wait` (its
//! progress shown, its exit status ignored), and `info` (its JSON gives the
//! status). Any status but `Accepted` runs `notarytool log`, which prints
//! Apple's reasons into the bundler's output, and fails the run.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::bundle::{BundleError, run};
use crate::ports::{Invocation, Process, RunError};

/// The status of an accepted submission.
pub const ACCEPTED: &str = "Accepted";
/// How long `notarytool wait` waits for a verdict.
pub const TIMEOUT: &str = "1h";

/// One file to notarize, and the API-key arguments that authenticate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notarization {
    /// The `.zip` of the app, or the `.dmg`.
    pub path: PathBuf,
    /// `--key <path> --key-id <id> --issuer <issuer>`.
    pub auth: Vec<OsString>,
}

/// Notarizes `notarization.path`, failing unless the notary accepts it.
pub fn notarize(
    process: &mut impl Process,
    notarization: &Notarization,
) -> Result<(), BundleError> {
    let notarytool = |sub: &str, first: &OsStr| {
        Invocation::new("xcrun")
            .args(["notarytool", sub])
            .arg(first)
            .args(&notarization.auth)
    };
    let json = ["--output-format", "json"];

    let submit = notarytool("submit", notarization.path.as_os_str())
        .args(json)
        .captured();
    let id = read(process, &submit, "id", "submission id")?;
    let id_arg = OsStr::new(&id);

    // Its exit status says nothing reliable: `info` decides.
    let wait = notarytool("wait", id_arg).args(["--timeout", TIMEOUT]);
    if let Err(source @ RunError::Spawn(_)) = process.run(&wait) {
        return Err(BundleError::Run {
            command: wait.to_string(),
            source,
        });
    }

    let info = notarytool("info", id_arg).args(json).captured();
    let status = read(process, &info, "status", "status")?;
    if status == ACCEPTED {
        return Ok(());
    }
    // Apple's reasons, into the bundler's output. The rejection is the
    // error, whatever the log does.
    let _ = process.run(&notarytool("log", id_arg));
    Err(BundleError::NotNotarized {
        path: notarization.path.clone(),
        id,
        status,
    })
}

/// Runs `invocation` and reads the string `key` from its JSON output.
fn read(
    process: &mut impl Process,
    invocation: &Invocation,
    key: &str,
    what: &str,
) -> Result<String, BundleError> {
    let stdout = run(process, invocation)?;
    field(&stdout, key).ok_or_else(|| BundleError::NotaryOutput {
        command: invocation.to_string(),
        reason: what.to_owned(),
    })
}

/// The string `key` of the JSON object `json`.
fn field(json: &[u8], key: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    value.get(key)?.as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::test_support::{self, developer_id_host, failing, paths, starts};

    fn notarytool(sub: &str, first: &str) -> Invocation {
        Invocation::new("xcrun")
            .args(["notarytool", sub, first, "--key"])
            .arg(paths().key)
            .args(["--key-id", "KEYID12345", "--issuer", "issuer-uuid"])
    }

    fn is_notary(sub: &'static str) -> impl Fn(&Invocation) -> bool {
        move |i| starts(i, "xcrun", &["notarytool", sub])
    }

    /// Whether `invocation` names `path` as an argument.
    fn names(invocation: &Invocation, path: &Path) -> bool {
        invocation.args.iter().any(|a| Path::new(a) == path)
    }

    fn json(text: &'static str) -> Vec<u8> {
        text.as_bytes().to_vec()
    }

    /// Answers every `notarytool info` with `text`.
    fn info(text: &'static str) -> impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> {
        move |i| {
            if is_notary("info")(i) {
                Some(Ok(json(text)))
            } else {
                None
            }
        }
    }

    fn developer_id(
        respond: impl Fn(&Invocation) -> Option<Result<Vec<u8>, RunError>> + 'static,
    ) -> test_support::Run {
        test_support::run(&developer_id_host(), b"arm64", respond, |_| false)
    }

    fn cleaned_up(run: &test_support::Run) -> bool {
        run.ran_args("security", &["delete-keychain"])
            && run.events.last() == Some(&test_support::Event::RemoveDir(paths().signing))
    }

    #[test]
    fn a_rejected_app_prints_the_log_and_makes_no_dmg() {
        let run = developer_id(info(r#"{"status":"Invalid"}"#));
        let Err(BundleError::NotNotarized { path, id, status }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(path, &paths().zip);
        assert_eq!(id, "abc");
        assert_eq!(status, "Invalid");
        let text = run.result.as_ref().expect_err("fails").to_string();
        assert!(text.contains("Invalid") && text.contains("abc"), "{text}");
        assert!(run.runs().contains(&notarytool("log", "abc")));
        assert!(!run.ran_args("xcrun", &["stapler"]));
        assert!(!run.ran("hdiutil"));
        assert!(cleaned_up(&run));
    }

    #[test]
    fn a_rejected_dmg_is_never_stapled() {
        let dmg = paths().dmg;
        let run = developer_id(move |i| {
            if is_notary("submit")(i) && names(i, &dmg) {
                Some(Ok(json(r#"{"id":"dmg1"}"#)))
            } else if starts(i, "xcrun", &["notarytool", "info", "dmg1"]) {
                Some(Ok(json(r#"{"status":"Rejected"}"#)))
            } else {
                None
            }
        });
        let Err(BundleError::NotNotarized { path, id, status }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert_eq!(
            (path, id.as_str(), status.as_str()),
            (&paths().dmg, "dmg1", "Rejected")
        );
        assert!(run.runs().contains(&notarytool("log", "dmg1")));
        assert!(run.ran_args("xcrun", &["stapler", "staple"]), "the app was");
        let dmg = paths().dmg;
        assert!(
            !run.runs()
                .iter()
                .any(|i| starts(i, "xcrun", &["stapler"]) && names(i, &dmg))
        );
        assert!(cleaned_up(&run));
    }

    #[test]
    fn a_failing_log_still_reports_the_rejection() {
        let run = developer_id(|i| {
            if is_notary("info")(i) {
                Some(Ok(json(r#"{"status":"Invalid"}"#)))
            } else {
                failing(is_notary("log"))(i)
            }
        });
        assert!(
            matches!(&run.result, Err(BundleError::NotNotarized { status, .. }) if status == "Invalid"),
            "{:?}",
            run.result
        );
    }

    #[test]
    fn a_failing_wait_is_judged_by_the_status() {
        let run = developer_id(failing(is_notary("wait")));
        assert!(run.result.is_ok(), "{:?}", run.result);
        assert!(run.ran_args("xcrun", &["stapler", "staple"]));
    }

    #[test]
    fn a_wait_that_cannot_start_is_an_error() {
        let run = developer_id(|i| {
            is_notary("wait")(i).then(|| Err(RunError::Spawn(std::io::Error::other("no xcrun"))))
        });
        let Err(BundleError::Run { command, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert!(command.contains("notarytool wait abc"), "{command}");
        assert!(!run.ran_args("xcrun", &["notarytool", "info"]));
        assert!(cleaned_up(&run));
    }

    #[test]
    fn a_failing_submit_waits_for_nothing() {
        let run = developer_id(failing(is_notary("submit")));
        let Err(BundleError::Run { command, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert!(command.contains("notarytool submit"), "{command}");
        assert!(!run.ran_args("xcrun", &["notarytool", "wait"]));
        assert!(!run.ran_args("xcrun", &["notarytool", "info"]));
        assert!(cleaned_up(&run));
    }

    #[test]
    fn a_failing_info_is_an_error() {
        let run = developer_id(failing(is_notary("info")));
        let Err(BundleError::Run { command, .. }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert!(command.contains("notarytool info abc"), "{command}");
        assert!(!run.ran_args("xcrun", &["stapler"]));
    }

    #[test]
    fn a_submit_printing_no_id_is_an_error() {
        for output in [&b"not json"[..], br#"{"message":"ok"}"#, br#"{"id":7}"#] {
            let run = developer_id(move |i| is_notary("submit")(i).then(|| Ok(output.to_vec())));
            let Err(BundleError::NotaryOutput { command, reason }) = &run.result else {
                panic!("{:?}", run.result);
            };
            assert!(command.contains("notarytool submit"), "{command}");
            assert!(reason.contains("id"), "{reason}");
            assert!(!run.ran_args("xcrun", &["notarytool", "wait"]));
            assert!(cleaned_up(&run));
        }
    }

    #[test]
    fn an_info_printing_no_status_is_an_error() {
        let run = developer_id(info(r#"{"id":"abc"}"#));
        let Err(BundleError::NotaryOutput { command, reason }) = &run.result else {
            panic!("{:?}", run.result);
        };
        assert!(command.contains("notarytool info abc"), "{command}");
        assert!(reason.contains("status"), "{reason}");
        assert!(!run.ran_args("xcrun", &["notarytool", "log"]));
    }

    #[test]
    fn the_wait_and_log_show_their_output_and_submit_and_info_are_read() {
        let run = developer_id(info(r#"{"status":"Invalid"}"#));
        for (sub, captured) in [
            ("submit", true),
            ("wait", false),
            ("info", true),
            ("log", false),
        ] {
            let invocation = run
                .runs()
                .into_iter()
                .find(|i| is_notary(sub)(i))
                .unwrap_or_else(|| panic!("{sub} ran"));
            assert_eq!(invocation.capture, captured, "{sub}");
        }
    }

    #[test]
    fn the_id_and_status_are_read_from_json() {
        assert_eq!(
            field(br#"{"id":"x-1","path":"/p"}"#, "id"),
            Some("x-1".into())
        );
        assert_eq!(
            field(br#" {"status" : "Accepted"} "#, "status"),
            Some("Accepted".into())
        );
        assert_eq!(field(b"[]", "id"), None);
        assert_eq!(field(br#"{"id":null}"#, "id"), None);
        assert_eq!(field(b"", "id"), None);
    }
}
