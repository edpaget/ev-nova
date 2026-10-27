//! `VERSIONINFO`'s four numbers from a semver version.

/// A version that is not `major.minor.patch` with numbers that fit.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a major.minor.patch version of 16-bit numbers")]
pub struct VersionError(pub String);

/// The file and product version numbers for the semver `version`:
/// `major.minor.patch` as `[major, minor, patch, 0]`. A pre-release or
/// build part (`-rc.1`, `+abc`) is dropped from the numbers; the version
/// strings keep it.
pub fn version_quad(version: &str) -> Result<[u16; 4], VersionError> {
    let refuse = || VersionError(version.to_owned());
    let numbers = version.split(['-', '+']).next().unwrap_or_default();
    let mut parts = numbers.split('.').map(|part| {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(refuse());
        }
        part.parse::<u16>().map_err(|_| refuse())
    });
    let mut next = || parts.next().ok_or_else(refuse)?;
    let quad = [next()?, next()?, next()?, 0];
    match parts.next() {
        None => Ok(quad),
        Some(_) => Err(refuse()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_is_its_three_numbers_and_a_zero() {
        assert_eq!(version_quad("0.1.0"), Ok([0, 1, 0, 0]));
        assert_eq!(version_quad("12.345.6789"), Ok([12, 345, 6789, 0]));
    }

    #[test]
    fn a_pre_release_or_build_part_is_dropped_from_the_numbers() {
        assert_eq!(version_quad("1.2.3-rc.1"), Ok([1, 2, 3, 0]));
        assert_eq!(version_quad("1.2.3+build.7"), Ok([1, 2, 3, 0]));
        assert_eq!(version_quad("1.2.3-rc.1+build.7"), Ok([1, 2, 3, 0]));
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1.x.3",
            "1.2.",
            "-1.2.3",
            "1.2.65536",
            " 1.2.3",
        ] {
            assert_eq!(
                version_quad(bad),
                Err(VersionError(bad.to_owned())),
                "{bad:?}"
            );
        }
        assert_eq!(version_quad("65535.0.1"), Ok([65535, 0, 1, 0]));
    }
}
