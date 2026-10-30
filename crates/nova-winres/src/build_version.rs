//! The version WinSparkle compares: the build ID (`YYYYMMDDHHMMSS`, UTC)
//! as `YYYYMMDD.HHMMSS`.
//!
//! WinSparkle 0.9.4 compares versions part by part, turning each run of
//! digits into an `int` with `atoi` (`UpdateChecker::CompareVersions`,
//! `src/updatechecker.cpp` lines 147-148). The 14-digit build ID is above
//! `INT_MAX`, so MSVC's `atoi` saturates every ID to the same value and no
//! build would ever look newer. Split at the day, each part fits: the date
//! is at most 99991231 and the time at most 235959. The date part orders
//! the days and, within a day, the fixed-width time orders the seconds, so
//! the dotted form orders as the IDs do. The time's leading zeros stay:
//! `atoi` reads `"091500"` as 91500.
//!
//! nova's build script compiles this form in for WinSparkle
//! (`nova::winsparkle::BUILD_VERSION`), and the bundler writes it as the
//! Windows feed's `sparkle:version` (`nova_bundle::build_id::BuildId`), so
//! both sides of the comparison come from this one function. macOS keeps
//! the 14 digits: Sparkle reads each part as a `long long`.

/// The build ID's length: `YYYYMMDDHHMMSS`.
const ID_LEN: usize = 14;

/// Where the date ends and the time begins.
const DATE_LEN: usize = 8;

/// The version WinSparkle compares for the build ID `build_id`,
/// `YYYYMMDD.HHMMSS`, or `None` unless it is 14 ASCII digits. It doesn't
/// check the digits name a time: the bundler does.
#[must_use]
pub fn windows_build_version(build_id: &str) -> Option<String> {
    if build_id.len() != ID_LEN || !build_id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (date, time) = build_id.split_at(DATE_LEN);
    Some(format!("{date}.{time}"))
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    fn version(id: &str) -> String {
        windows_build_version(id).expect("14 digits")
    }

    /// MSVC's `atoi` on a run of ASCII digits: the number, saturated at
    /// `INT_MAX`.
    fn atoi(digits: &str) -> i64 {
        let value: u128 = digits.parse().expect("digits");
        i64::try_from(value.min(i32::MAX as u128)).expect("fits")
    }

    /// Sparkle's `-[NSString longLongValue]` on a run of ASCII digits.
    fn long_long_value(digits: &str) -> i64 {
        digits.parse().expect("fits a long long")
    }

    /// A model of WinSparkle's and Sparkle's comparison for versions of
    /// digits and periods: part by part, each run of digits read by
    /// `number`; when every shared part is equal, the one with more parts
    /// is newer.
    fn compare(a: &str, b: &str, number: fn(&str) -> i64) -> Ordering {
        let (parts_a, parts_b): (Vec<_>, Vec<_>) = (a.split('.').collect(), b.split('.').collect());
        for (x, y) in parts_a.iter().zip(&parts_b) {
            match number(x).cmp(&number(y)) {
                Ordering::Equal => {}
                other => return other,
            }
        }
        parts_a.len().cmp(&parts_b.len())
    }

    #[test]
    fn splits_the_id_at_the_day_keeping_the_time_s_leading_zeros() {
        assert_eq!(version("20261028091500"), "20261028.091500");
        assert_eq!(version("19700101000000"), "19700101.000000");
    }

    #[test]
    fn anything_but_14_ascii_digits_has_none() {
        for bad in [
            "",
            "2026102809150",
            "202610280915000",
            "2026102809150a",
            "20261028.09150",
            "２０２６１０２８０９１５",
        ] {
            assert_eq!(windows_build_version(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn every_part_of_the_last_id_fits_an_int() {
        let last = version("99991231235959");
        for part in last.split('.') {
            let value: i64 = part.parse().expect("digits");
            assert!(value <= i64::from(i32::MAX), "{part}");
        }
    }

    #[test]
    fn the_raw_ids_compare_equal_under_atoi() {
        // The bug the dotted form avoids: both saturate to INT_MAX.
        assert_eq!(
            compare("20261028123456", "20261029000000", atoi),
            Ordering::Equal
        );
    }

    #[test]
    fn the_versions_order_as_the_ids_under_atoi_and_long_long_value() {
        let ids = [
            "19700101000000",
            "20261028000000",
            "20261028000001",
            "20261028091500",
            "20261028123456",
            "20261028235959",
            "20261029000000",
            "20261231235959",
            "20270101000000",
            "99991231235959",
        ];
        for a in ids {
            for b in ids {
                let want = a.cmp(b);
                for (name, number) in [
                    ("atoi", atoi as fn(&str) -> i64),
                    ("longLongValue", long_long_value),
                ] {
                    assert_eq!(
                        compare(&version(a), &version(b), number),
                        want,
                        "{a} {b} {name}"
                    );
                }
            }
        }
    }
}
