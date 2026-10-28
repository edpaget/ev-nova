//! The build ID: HEAD's committer time in UTC, as `YYYYMMDDHHMMSS`.
//!
//! [`resolve`] takes it from [`VAR`] when that is set, so CI can pin one
//! ID for every leg of a run and tests stay hermetic, and otherwise reads
//! HEAD's committer time with `git` ([`committer_time`]). The committer
//! time, not the author time: landing rebases or makes a merge commit,
//! which resets it, so it goes up along `main`.

use std::fmt;
use std::path::Path;

use crate::ports::{Env, Invocation, Process, RunError};

/// The variable that overrides the read of HEAD's committer time. The
/// bundler passes it to the `cargo build` of `nova`, which compiles it in.
pub const VAR: &str = "NOVA_BUILD_ID";

/// A build's ID: always 14 ASCII digits naming a valid UTC time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildId(String);

/// Why a value is not a build ID.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildIdError {
    /// The text is not 14 digits naming a valid UTC time.
    #[error("`{0}` is not a build ID (YYYYMMDDHHMMSS, UTC)")]
    Malformed(String),
    /// The Unix time falls outside the years 1970 to 9999.
    #[error("the time {0} is outside the years 1970 to 9999")]
    OutOfRange(i64),
}

impl BuildId {
    /// The ID of the UTC time `secs` seconds after the Unix epoch.
    ///
    /// # Errors
    ///
    /// [`BuildIdError::OutOfRange`] for a time before 1970 or after 9999.
    pub fn from_unix(secs: i64) -> Result<Self, BuildIdError> {
        if !(0..=LAST_SECOND).contains(&secs) {
            return Err(BuildIdError::OutOfRange(secs));
        }
        let (days, second_of_day) = (secs / SECS_PER_DAY, secs % SECS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        let (hour, minute, second) = (
            second_of_day / 3600,
            second_of_day / 60 % 60,
            second_of_day % 60,
        );
        Ok(Self(format!(
            "{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}"
        )))
    }

    /// Reads an ID: 14 digits naming a valid UTC time.
    ///
    /// # Errors
    ///
    /// [`BuildIdError::Malformed`] for anything else, the empty string
    /// included.
    pub fn parse(text: &str) -> Result<Self, BuildIdError> {
        let malformed = || BuildIdError::Malformed(text.to_owned());
        if text.len() != 14 || !text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(malformed());
        }
        // Each field is ASCII digits, so it parses.
        let field =
            |range: std::ops::Range<usize>| -> u32 { text[range].parse().expect("ASCII digits") };
        let (year, month, day) = (field(0..4), field(4..6), field(6..8));
        let (hour, minute, second) = (field(8..10), field(10..12), field(12..14));
        let valid = (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hour < 24
            && minute < 60
            && second < 60;
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(malformed())
        }
    }

    /// The ID's day, as `YYYY-MM-DD`.
    #[must_use]
    pub fn date(&self) -> String {
        let s = &self.0;
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    }

    /// The 14 digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The days in `month` (1 to 12) of the Gregorian `year`.
fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Whether the Gregorian `year` has a 29 February.
fn is_leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

const SECS_PER_DAY: i64 = 86_400;

/// 9999-12-31 23:59:59 UTC, the last time a 4-digit year can name.
const LAST_SECOND: i64 = 253_402_300_799;

/// The proleptic Gregorian `(year, month, day)` of the day `days` after
/// 1970-01-01, by Howard Hinnant's `civil_from_days`. `days` is never
/// negative here.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    // Days counted from 0000-03-01, so a leap day ends its year.
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

impl fmt::Display for BuildId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `git show` printing HEAD's committer time, in Unix seconds, in the
/// checkout at `repo`.
#[must_use]
pub fn committer_time(repo: &Path) -> Invocation {
    Invocation::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", "-s", "--format=%ct", "HEAD"])
        .captured()
}

/// Why the build ID could not be had.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// [`VAR`] is set, but not to a build ID.
    #[error("{VAR}: {0}")]
    Env(#[source] BuildIdError),
    /// [`VAR`] is unset and `git` could not read HEAD's time.
    #[error("set {VAR} or run inside a git checkout: `{command}` failed: {source}")]
    Git {
        /// The `git` command line.
        command: String,
        /// How it failed.
        source: RunError,
    },
    /// `git` printed something other than a time.
    #[error("`{command}` printed `{output}`, not a Unix time")]
    Output {
        /// The `git` command line.
        command: String,
        /// What it printed, trimmed.
        output: String,
    },
    /// HEAD's time names no build ID.
    #[error(transparent)]
    Time(BuildIdError),
}

/// The build ID: [`VAR`]'s when it is set, else HEAD's committer time in
/// the checkout at `repo`, read with [`committer_time`]. A set but empty
/// variable is an error, so a CI expression that comes out empty fails
/// rather than falling back.
///
/// # Errors
///
/// A malformed [`VAR`], a failing `git` or output that is not a time.
pub fn resolve(
    env: &impl Env,
    process: &mut impl Process,
    repo: &Path,
) -> Result<BuildId, ResolveError> {
    if let Some(value) = env.var(VAR) {
        return BuildId::parse(&value.to_string_lossy()).map_err(ResolveError::Env);
    }
    let invocation = committer_time(repo);
    let stdout = process
        .run(&invocation)
        .map_err(|source| ResolveError::Git {
            command: invocation.to_string(),
            source,
        })?;
    let output = String::from_utf8_lossy(&stdout).trim().to_owned();
    let secs = output.parse().map_err(|_| ResolveError::Output {
        command: invocation.to_string(),
        output: output.clone(),
    })?;
    BuildId::from_unix(secs).map_err(ResolveError::Time)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::*;
    use crate::test_support::{Event, FakeEnv, Log, Recorder, Respond};

    fn recorder(respond: Respond) -> (Recorder, Log) {
        let log = Log::default();
        let recorder = Recorder {
            log: Rc::clone(&log),
            archs: Vec::new(),
            respond,
        };
        (recorder, log)
    }

    fn runs(log: &Log) -> Vec<Invocation> {
        log.borrow()
            .iter()
            .filter_map(|e| match e {
                Event::Run(i) => Some(i.clone()),
                _ => None,
            })
            .collect()
    }

    fn git_prints(output: &'static [u8]) -> Respond {
        Box::new(move |i| (i.program == "git").then(|| Ok(output.to_vec())))
    }

    fn with_var(value: &str) -> FakeEnv {
        FakeEnv(HashMap::from([(VAR, value.into())]))
    }

    fn repo() -> std::path::PathBuf {
        Path::new("/work").join("ev-nova")
    }

    #[test]
    fn the_variable_overrides_git_and_runs_nothing() {
        let (mut process, log) = recorder(git_prints(b"981173106\n"));
        let id = resolve(&with_var("20261028123456"), &mut process, &repo()).expect("an ID");
        assert_eq!(id.as_str(), "20261028123456");
        assert!(runs(&log).is_empty());
    }

    #[test]
    fn a_malformed_or_empty_variable_is_an_error_and_runs_nothing() {
        for bad in ["", "2026", "not-an-id"] {
            let (mut process, log) = recorder(git_prints(b"981173106\n"));
            let error = resolve(&with_var(bad), &mut process, &repo()).expect_err("malformed");
            assert!(
                matches!(&error, ResolveError::Env(BuildIdError::Malformed(v)) if v == bad),
                "{error:?}"
            );
            assert!(error.to_string().starts_with("NOVA_BUILD_ID: "), "{error}");
            assert!(runs(&log).is_empty());
        }
    }

    #[test]
    fn without_the_variable_it_reads_head_s_committer_time() {
        let repo = repo();
        let (mut process, log) = recorder(git_prints(b"981173106\n"));
        let id = resolve(&FakeEnv::default(), &mut process, &repo).expect("an ID");
        assert_eq!(id.as_str(), "20010203040506");
        assert_eq!(runs(&log), vec![committer_time(&repo)]);
    }

    #[test]
    fn committer_time_asks_git_for_head_s_committer_seconds() {
        let repo = repo();
        assert_eq!(
            committer_time(&repo),
            Invocation::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["show", "-s", "--format=%ct", "HEAD"])
                .captured()
        );
    }

    #[test]
    fn a_failing_git_is_an_error_that_names_the_variable() {
        let (mut process, _) = recorder(Box::new(|_| Some(Err(RunError::Failed(Some(128))))));
        let error = resolve(&FakeEnv::default(), &mut process, &repo()).expect_err("fails");
        assert!(
            matches!(
                error,
                ResolveError::Git {
                    source: RunError::Failed(Some(128)),
                    ..
                }
            ),
            "{error:?}"
        );
        let text = error.to_string();
        assert!(text.contains("set NOVA_BUILD_ID"), "{text}");
        assert!(text.contains("git -C"), "{text}");
    }

    #[test]
    fn output_that_is_not_a_time_is_an_error() {
        for (output, shown) in [
            (&b"yesterday\n"[..], "yesterday"),
            (b"", ""),
            (b"\xff\n", "\u{fffd}"),
        ] {
            let (mut process, _) = recorder(Box::new(move |_| Some(Ok(output.to_vec()))));
            let error = resolve(&FakeEnv::default(), &mut process, &repo()).expect_err("fails");
            assert!(
                matches!(&error, ResolveError::Output { output, .. } if output == shown),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_time_before_1970_is_an_error() {
        let (mut process, _) = recorder(git_prints(b"-5\n"));
        let error = resolve(&FakeEnv::default(), &mut process, &repo()).expect_err("fails");
        assert!(
            matches!(error, ResolveError::Time(BuildIdError::OutOfRange(-5))),
            "{error:?}"
        );
    }

    fn id(secs: i64) -> String {
        BuildId::from_unix(secs).expect("in range").to_string()
    }

    #[test]
    fn formats_a_time_with_single_digit_fields_zero_padded() {
        // 2001-02-03 04:05:06 UTC.
        assert_eq!(id(981_173_106), "20010203040506");
    }

    #[test]
    fn the_epoch_is_19700101000000() {
        assert_eq!(id(0), "19700101000000");
    }

    #[test]
    fn a_leap_day_is_formatted() {
        assert_eq!(id(1_709_251_199), "20240229235959");
    }

    #[test]
    fn a_time_before_the_epoch_is_an_error() {
        assert_eq!(BuildId::from_unix(-1), Err(BuildIdError::OutOfRange(-1)));
    }

    #[test]
    fn the_last_second_of_9999_is_the_last_id() {
        assert_eq!(id(253_402_300_799), "99991231235959");
        assert_eq!(
            BuildId::from_unix(253_402_300_800),
            Err(BuildIdError::OutOfRange(253_402_300_800))
        );
    }

    #[test]
    fn the_day_after_february_is_march_first() {
        // 2023-03-01 00:00:00 and 2000-03-01 00:00:00 UTC.
        assert_eq!(id(1_677_628_800), "20230301000000");
        assert_eq!(id(951_868_800), "20000301000000");
        // 2026-12-31 12:00:00 UTC.
        assert_eq!(id(1_798_718_400), "20261231120000");
    }

    #[test]
    fn parses_what_from_unix_formats() {
        for secs in [0, 981_173_106, 1_709_251_199, 951_868_800, 253_402_300_799] {
            let made = BuildId::from_unix(secs).expect("in range");
            assert_eq!(BuildId::parse(made.as_str()), Ok(made));
        }
    }

    #[test]
    fn rejects_each_malformed_id() {
        for bad in [
            "",
            "2001020304050",
            "200102030405066",
            "2001020304050a",
            "+0010203040506",
            "20010003040506",
            "20011303040506",
            "20010200040506",
            "20230229040506",
            "19000229040506",
            "20010431040506",
            "20010132040506",
            "20010203240506",
            "20010203046006",
            "20010203040560",
            "２００１０２０３０４０５",
        ] {
            assert_eq!(
                BuildId::parse(bad),
                Err(BuildIdError::Malformed(bad.to_owned())),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn accepts_the_last_day_of_each_month() {
        for good in [
            "20010131000000",
            "20010228000000",
            "20000229000000",
            "20240229000000",
            "20010331000000",
            "20010430000000",
            "20010531000000",
            "20010630000000",
            "20010731000000",
            "20010831000000",
            "20010930000000",
            "20011031000000",
            "20011130000000",
            "20011231235959",
        ] {
            assert_eq!(
                BuildId::parse(good).map(|id| id.to_string()),
                Ok(good.to_owned())
            );
        }
    }

    #[test]
    fn date_is_the_iso_day() {
        let id = BuildId::parse("20010203040506").expect("an ID");
        assert_eq!(id.date(), "2001-02-03");
    }

    #[test]
    fn an_error_names_the_bad_value() {
        assert_eq!(
            BuildIdError::Malformed("nope".into()).to_string(),
            "`nope` is not a build ID (YYYYMMDDHHMMSS, UTC)"
        );
    }
}
