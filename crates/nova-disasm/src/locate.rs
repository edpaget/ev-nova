//! Where the binary and the cache are.

use std::path::{Path, PathBuf};

use crate::ports::{Env, Workspace};

/// The environment variable naming the binary.
pub const BINARY_VAR: &str = "NOVA_BINARY";

/// The binary inside a checkout: `EV Nova.app/Contents/MacOS/EV Nova`.
#[must_use]
pub fn app_binary(checkout: &Path) -> PathBuf {
    checkout
        .join("EV Nova.app")
        .join("Contents")
        .join("MacOS")
        .join("EV Nova")
}

/// The binary to read: the `--binary` flag, else `NOVA_BINARY`, else the
/// app in the primary checkout (worktrees don't hold the gitignored app),
/// else the app in this workspace when git can't name the primary.
pub fn binary_path(flag: Option<&Path>, env: &dyn Env, workspace: &dyn Workspace) -> PathBuf {
    if let Some(flag) = flag {
        return flag.to_path_buf();
    }
    if let Some(var) = env.var(BINARY_VAR).filter(|v| !v.is_empty()) {
        return PathBuf::from(var);
    }
    let checkout = workspace
        .primary_checkout()
        .unwrap_or_else(|| workspace.workspace_root());
    app_binary(&checkout)
}

/// The cache's directory: `disasm` under `CARGO_TARGET_DIR`, else under
/// the workspace's `target`.
pub fn cache_dir(env: &dyn Env, workspace: &dyn Workspace) -> PathBuf {
    let target = match env.var("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => workspace.workspace_root().join("target"),
    };
    target.join("disasm")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fakes::{FakeEnv, FakeWorkspace};

    fn workspace(primary: Option<&str>) -> FakeWorkspace {
        FakeWorkspace {
            primary: primary.map(PathBuf::from),
            root: PathBuf::from("root"),
        }
    }

    #[test]
    fn the_flag_comes_first() {
        let env = FakeEnv::with(BINARY_VAR, "from-env");
        let got = binary_path(
            Some(Path::new("from-flag")),
            &env,
            &workspace(Some("primary")),
        );
        assert_eq!(got, Path::new("from-flag"));
    }

    #[test]
    fn then_the_environment() {
        let env = FakeEnv::with(BINARY_VAR, "from-env");
        assert_eq!(
            binary_path(None, &env, &workspace(Some("primary"))),
            Path::new("from-env")
        );
    }

    #[test]
    fn an_empty_variable_is_unset() {
        let env = FakeEnv::with(BINARY_VAR, "");
        let ws = workspace(Some("primary"));
        assert_eq!(
            binary_path(None, &env, &ws),
            app_binary(Path::new("primary"))
        );
    }

    #[test]
    fn then_the_primary_checkout() {
        let ws = workspace(Some("primary"));
        assert_eq!(
            binary_path(None, &FakeEnv::default(), &ws),
            app_binary(Path::new("primary"))
        );
    }

    #[test]
    fn then_the_workspace_root() {
        let ws = workspace(None);
        assert_eq!(
            binary_path(None, &FakeEnv::default(), &ws),
            app_binary(Path::new("root"))
        );
    }

    #[test]
    fn the_app_binary_sits_inside_the_bundle() {
        let got = app_binary(Path::new("co"));
        let parts: Vec<_> = got.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(parts, ["co", "EV Nova.app", "Contents", "MacOS", "EV Nova"]);
    }

    #[test]
    fn the_cache_lives_under_the_target_directory() {
        let ws = workspace(None);
        assert_eq!(
            cache_dir(&FakeEnv::default(), &ws),
            Path::new("root").join("target").join("disasm")
        );
        let env = FakeEnv::with("CARGO_TARGET_DIR", "elsewhere");
        assert_eq!(cache_dir(&env, &ws), Path::new("elsewhere").join("disasm"));
    }
}
