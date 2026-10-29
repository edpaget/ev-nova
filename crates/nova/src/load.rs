//! What loading the game data found wrong, as lines for standard error.

use nova_data::{GameData, check_expressions};

/// One line for each data file that could not be loaded, then one for each
/// control-bit expression that does not parse. The game loads anyway.
#[must_use]
pub fn data_warnings(data: &GameData) -> Vec<String> {
    let skipped = data
        .failed()
        .iter()
        .map(|failed| format!("nova: skipped {}: {}", failed.path.display(), failed.error));
    let malformed = check_expressions(data)
        .into_iter()
        .map(|error| format!("nova: {error}"));
    skipped.chain(malformed).collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_data::Record;
    use nova_data::records::mission::Mission;
    use nova_data::store::fixture::{DATA_DIR, FakeTree, one_file};
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;

    /// `/data` holds `Broken`, which has no forks, and `Nova Data`, a fork
    /// holding mïsn 128 and 129.
    fn files() -> FakeTree {
        let file = ForkBuilder::new()
            .resource(Mission::TYPE, 128, None, &mission("b1 Z2"))
            .resource(Mission::TYPE, 129, None, &mission("b3"));
        one_file(file.build().bytes).no_data_fork(Path::new(DATA_DIR).join("Broken"))
    }

    /// A mïsn whose `on_accept` (offset 0x15B) holds `on_accept`.
    fn mission(on_accept: &str) -> Vec<u8> {
        let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
        bytes[0x15B..0x15B + on_accept.len()].copy_from_slice(on_accept.as_bytes());
        bytes
    }

    #[test]
    fn warns_of_each_skipped_file_then_each_malformed_expression() {
        let data = files().load();
        let [failed] = data.failed() else {
            panic!("one failed file: {:?}", data.failed());
        };
        assert_eq!(
            data_warnings(&data),
            [
                format!(
                    "nova: skipped {}: {}",
                    Path::new("/data").join("Broken").display(),
                    failed.error
                ),
                "nova: mïsn 128 `on_accept`: unknown set operator `Z` at byte 3 in \"b1 Z2\""
                    .to_owned(),
            ]
        );
    }
}
