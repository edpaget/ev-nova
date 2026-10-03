//! Walking the data and plug-ins folders in load order.

use std::path::{Path, PathBuf};

use nova_rsrc::LoadError;

use super::fs::{DirLister, Listing};
use super::order::{Classified, IgnoreReason, MAX_PLUGIN_DEPTH, classify, finder_cmp};
use super::{FailedFile, IgnoredEntry, OpenError, Origin};

/// Everything a walk found, in load order.
#[derive(Debug, Default)]
pub struct Walk {
    /// Files to try loading, in load order.
    pub candidates: Vec<(PathBuf, Origin)>,
    /// Entries skipped, with why.
    pub ignored: Vec<IgnoredEntry>,
    /// Plug-ins sub-folders that could not be listed.
    pub failed: Vec<FailedFile>,
}

/// Walks `data_dir` and then, if given, the `plugins` tree.
///
/// Each folder's entries are sorted with [`finder_cmp`] and classified with
/// [`classify`]; a plug-ins sub-folder is walked in place, at its sorted
/// position, down to [`MAX_PLUGIN_DEPTH`] levels. Only failing to list one
/// of the two roots is an error; a sub-folder that cannot be listed is
/// recorded in [`Walk::failed`].
pub fn load_order(
    dirs: &impl DirLister,
    data_dir: &Path,
    plugins: Option<&Path>,
) -> Result<Walk, OpenError> {
    let mut walk = Walk::default();
    let data = dirs.list(data_dir).map_err(|source| OpenError::DataDir {
        path: data_dir.to_path_buf(),
        source,
    })?;
    walk.visit(dirs, data_dir, data, Origin::Data, 0);
    if let Some(plugins) = plugins {
        let entries = dirs.list(plugins).map_err(|source| OpenError::PlugInsDir {
            path: plugins.to_path_buf(),
            source,
        })?;
        walk.visit(dirs, plugins, entries, Origin::PlugIn, 0);
    }
    Ok(walk)
}

impl Walk {
    /// Adds the `entries` of `dir`, a folder `depth` levels below its root.
    fn visit(
        &mut self,
        dirs: &impl DirLister,
        dir: &Path,
        mut entries: Vec<Listing>,
        origin: Origin,
        depth: usize,
    ) {
        entries.sort_by(|a, b| finder_cmp(&a.name, &b.name));
        for entry in entries {
            let path = dir.join(&entry.name);
            match classify(&entry.name, entry.kind, origin) {
                Classified::Candidate => self.candidates.push((path, origin)),
                Classified::Ignored(reason) => self.ignored.push(IgnoredEntry { path, reason }),
                Classified::Descend if depth >= MAX_PLUGIN_DEPTH => {
                    self.ignored.push(IgnoredEntry {
                        path,
                        reason: IgnoreReason::TooDeep,
                    });
                }
                Classified::Descend => match dirs.list(&path) {
                    Ok(children) => self.visit(dirs, &path, children, origin, depth + 1),
                    Err(source) => self.failed.push(FailedFile {
                        path: path.clone(),
                        origin,
                        error: LoadError::Io { path, source },
                    }),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::store::fake::{EndlessTree, FakeTree};
    use crate::store::fs::EntryKind::{Dir, File, Other, Symlink};

    fn walk(tree: &FakeTree, plugins: Option<&str>) -> Walk {
        load_order(tree, Path::new("/d"), plugins.map(Path::new)).expect("walks")
    }

    // Paths compare as `PathBuf`s, component by component, so a `/`-written
    // expectation matches the walk's native separators on every platform.
    fn paths(walk: &Walk) -> Vec<(PathBuf, Origin)> {
        walk.candidates.clone()
    }

    fn ignored(walk: &Walk) -> Vec<(PathBuf, IgnoreReason)> {
        walk.ignored
            .iter()
            .map(|entry| (entry.path.clone(), entry.reason.clone()))
            .collect()
    }

    fn owned<T: Clone>(items: &[(&str, T)]) -> Vec<(PathBuf, T)> {
        items
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.clone()))
            .collect()
    }

    #[test]
    fn data_files_load_in_finder_order_then_plugins() {
        let tree = FakeTree::new()
            .dir(
                "/d",
                &[("Nova b", File), ("nova A", File), ("Nova C", File)],
            )
            .dir("/p", &[("z", File), ("A", File)]);
        let walk = walk(&tree, Some("/p"));
        assert_eq!(
            paths(&walk),
            owned(&[
                ("/d/nova A", Origin::Data),
                ("/d/Nova b", Origin::Data),
                ("/d/Nova C", Origin::Data),
                ("/p/A", Origin::PlugIn),
                ("/p/z", Origin::PlugIn),
            ])
        );
        assert!(walk.ignored.is_empty());
        assert!(walk.failed.is_empty());
    }

    #[test]
    fn without_plugins_only_the_data_directory_is_listed() {
        let tree = FakeTree::new().dir("/d", &[("a", File)]);
        let walk = walk(&tree, None);
        assert_eq!(paths(&walk), owned(&[("/d/a", Origin::Data)]));
        assert_eq!(tree.listed(), [PathBuf::from("/d")]);
    }

    #[test]
    fn plugin_folders_load_at_their_sorted_position() {
        let tree = FakeTree::new()
            .dir("/d", &[])
            .dir("/p", &[("Charlie", File), ("beta", Dir), ("Alpha", File)])
            .dir("/p/beta", &[("inner", File)]);
        assert_eq!(
            paths(&walk(&tree, Some("/p"))),
            owned(&[
                ("/p/Alpha", Origin::PlugIn),
                ("/p/beta/inner", Origin::PlugIn),
                ("/p/Charlie", Origin::PlugIn),
            ])
        );
    }

    #[test]
    fn nested_folders_recurse_at_their_position() {
        let tree = FakeTree::new()
            .dir("/d", &[])
            .dir("/p", &[("c", File), ("A", Dir)])
            .dir("/p/A", &[("y", File), ("b", Dir), ("Z", File), ("a", File)])
            .dir(
                "/p/A/b",
                &[("x2", File), ("X1", File), ("w", File), ("X3", File)],
            );
        assert_eq!(
            paths(&walk(&tree, Some("/p"))),
            owned(&[
                ("/p/A/a", Origin::PlugIn),
                ("/p/A/b/w", Origin::PlugIn),
                ("/p/A/b/X1", Origin::PlugIn),
                ("/p/A/b/x2", Origin::PlugIn),
                ("/p/A/b/X3", Origin::PlugIn),
                ("/p/A/y", Origin::PlugIn),
                ("/p/A/Z", Origin::PlugIn),
                ("/p/c", Origin::PlugIn),
            ])
        );
    }

    #[test]
    fn data_sub_folders_are_ignored_and_not_listed() {
        let tree = FakeTree::new()
            .dir("/d", &[("Sub", Dir), ("f", File)])
            .dir("/d/Sub", &[("g", File)]);
        let walk = walk(&tree, None);
        assert_eq!(paths(&walk), owned(&[("/d/f", Origin::Data)]));
        assert_eq!(
            ignored(&walk),
            owned(&[("/d/Sub", IgnoreReason::DataSubFolder)])
        );
        assert_eq!(tree.listed(), [PathBuf::from("/d")]);
    }

    #[test]
    fn ignored_entries_are_reported_in_load_order() {
        let tree = FakeTree::new()
            .dir(
                "/d",
                &[("Music.mp3", File), (".DS_Store", File), ("a", File)],
            )
            .dir("/p", &[("L", Symlink), ("fifo", Other), ("x.rez", File)])
            .dir("/p/L", &[("g", File)]);
        let walk = walk(&tree, Some("/p"));
        assert_eq!(
            paths(&walk),
            owned(&[("/d/a", Origin::Data), ("/p/x.rez", Origin::PlugIn)])
        );
        assert_eq!(
            ignored(&walk),
            owned(&[
                ("/d/.DS_Store", IgnoreReason::Hidden),
                ("/d/Music.mp3", IgnoreReason::NotGameData("mp3".to_owned())),
                ("/p/fifo", IgnoreReason::NotAFile),
                ("/p/L", IgnoreReason::Symlink),
            ])
        );
        assert_eq!(tree.listed(), [PathBuf::from("/d"), PathBuf::from("/p")]);
    }

    #[test]
    fn an_unlistable_plugin_folder_fails_and_the_walk_continues() {
        let tree = FakeTree::new()
            .dir("/d", &[])
            .dir("/p", &[("c", File), ("bad", Dir), ("a", File)]);
        let walk = walk(&tree, Some("/p"));
        assert_eq!(
            paths(&walk),
            owned(&[("/p/a", Origin::PlugIn), ("/p/c", Origin::PlugIn)])
        );
        let [failed] = walk.failed.as_slice() else {
            panic!("one failure: {:?}", walk.failed)
        };
        assert_eq!(failed.path, Path::new("/p/bad"));
        assert_eq!(failed.origin, Origin::PlugIn);
        assert!(
            matches!(&failed.error, LoadError::Io { path, .. } if path == Path::new("/p/bad")),
            "{:?}",
            failed.error
        );
    }

    #[test]
    fn an_unlistable_data_directory_is_an_open_error() {
        let tree = FakeTree::new().dir("/p", &[]);
        let err = load_order(&tree, Path::new("/d"), Some(Path::new("/p"))).expect_err("fails");
        assert!(
            matches!(&err, OpenError::DataDir { path, .. } if path == Path::new("/d")),
            "{err:?}"
        );
    }

    #[test]
    fn an_unlistable_plugins_directory_is_an_open_error() {
        let tree = FakeTree::new().dir("/d", &[]);
        let err = load_order(&tree, Path::new("/d"), Some(Path::new("/p"))).expect_err("fails");
        assert!(
            matches!(&err, OpenError::PlugInsDir { path, .. } if path == Path::new("/p")),
            "{err:?}"
        );
    }

    #[test]
    fn an_endless_plugin_tree_stops_at_the_depth_limit() {
        let tree = EndlessTree {
            calls: RefCell::new(0),
            limit: 100,
        };
        let walk = load_order(&tree, Path::new("/d"), Some(Path::new("/p"))).expect("walks");
        // The data directory, the plug-ins directory and one listing per
        // sub-folder level from 1 to the limit.
        assert_eq!(*tree.calls.borrow(), 2 + MAX_PLUGIN_DEPTH);
        let mut deepest = PathBuf::from("/p");
        for _ in 0..=MAX_PLUGIN_DEPTH {
            deepest.push("deeper");
        }
        let too_deep: Vec<_> = walk
            .ignored
            .iter()
            .filter(|entry| entry.reason == IgnoreReason::TooDeep)
            .map(|entry| entry.path.clone())
            .collect();
        assert_eq!(too_deep, [deepest]);
        // One plug-in per listed plug-ins level (0 to the limit).
        let plugins = walk
            .candidates
            .iter()
            .filter(|(_, origin)| *origin == Origin::PlugIn)
            .count();
        assert_eq!(plugins, MAX_PLUGIN_DEPTH + 1);
    }
}
