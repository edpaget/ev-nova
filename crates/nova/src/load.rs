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
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::records::mission::Mission;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader};

    use super::*;

    /// `/data` holds `Broken`, which has no forks, and `Nova Data`, a fork
    /// holding mïsn 128 and 129.
    struct Files;

    impl DirLister for Files {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(["Broken", "Nova Data"]
                .map(|name| Listing {
                    name: name.into(),
                    kind: EntryKind::File,
                })
                .into())
        }
    }

    impl ForkReader for Files {
        fn read_fork(&self, path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            if fork == Fork::Resource || path.ends_with("Broken") {
                return Ok(None);
            }
            let file = ForkBuilder::new()
                .resource(Mission::TYPE, 128, None, &mission("b1 Z2"))
                .resource(Mission::TYPE, 129, None, &mission("b3"));
            Ok(Some(file.build().bytes))
        }
    }

    /// A mïsn whose `on_accept` (offset 0x15B) holds `on_accept`.
    fn mission(on_accept: &str) -> Vec<u8> {
        let mut bytes = vec![0; Mission::SIZE.expect("fixed")];
        bytes[0x15B..0x15B + on_accept.len()].copy_from_slice(on_accept.as_bytes());
        bytes
    }

    #[test]
    fn warns_of_each_skipped_file_then_each_malformed_expression() {
        let data = GameData::load(&Files, &Files, Path::new("/data"), None).expect("opens");
        let [failed] = data.failed() else {
            panic!("one failed file: {:?}", data.failed());
        };
        assert_eq!(
            data_warnings(&data),
            [
                format!("nova: skipped /data/Broken: {}", failed.error),
                "nova: mïsn 128 `on_accept`: unknown set operator `Z` at byte 3 in \"b1 Z2\""
                    .to_owned(),
            ]
        );
    }
}
