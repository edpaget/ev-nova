//! The build ID `nova` was compiled with: `NOVA_BUILD_ID` at compile time,
//! which the bundlers and CI set (`nova_bundle::build_id`), or none for a
//! development build. Cargo rebuilds `nova` when the variable changes,
//! since rustc records the `option_env!` read.

/// The ID, as `YYYYMMDDHHMMSS` in UTC, or `None` for a development build.
pub const BUILD_ID: Option<&str> = option_env!("NOVA_BUILD_ID");

/// The build as the startup log names it.
#[must_use]
pub fn describe(id: Option<&str>) -> String {
    match id {
        Some(id) => format!("build {id}"),
        None => "development build".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_names_the_id_or_development() {
        assert_eq!(describe(Some("20261028123456")), "build 20261028123456");
        assert_eq!(describe(None), "development build");
    }
}
