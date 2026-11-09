//! The signing tools: `signtool.exe` from the Windows SDK build tools and
//! the Artifact Signing dlib, two `NuGet` packages pinned by version and
//! SHA-256, and the `metadata.json` that tells the dlib which account and
//! profile to sign with.
//!
//! [`stage`] downloads each package with `curl` from `NuGet`'s flat
//! container, checks its SHA-256 with `certutil` before anything unpacks
//! it, and unpacks its x64 directory with `tar` (Windows' own bsdtar, which
//! reads the `.nupkg` zip), the leading directories stripped. A package
//! that isn't the pinned one stops the run before `signtool` runs.
//!
//! The dlib's package asks for its licence (`LICENSE.md`) to be accepted;
//! Microsoft's terms allow this use, signing the project's own files with
//! its own Artifact Signing account.
//!
//! ```text
//! <target>/authenticode/
//!   downloads/   the two .nupkg files, checked
//!   dlib/        the dlib's bin/x64: Azure.CodeSigning.Dlib.dll and its dependencies
//!   signtool/    the SDK's bin/10.0.26100.0/x64: signtool.exe and its DLLs
//!   metadata.json
//! ```
//!
//! # Bumping a pin
//!
//! Change the package's version and digest together. The digest is
//! `certutil -hashfile <nupkg> SHA256` (or `shasum -a 256`) of the
//! downloaded `.nupkg`. Check the x64 directory's path in the new package
//! (`unzip -l`), that `signtool.exe` is at least 10.0.2261.755 and not the
//! 20348 SDK's (the dlib supports neither older nor that one), and pick a
//! release at least two weeks old.

use std::path::{Path, PathBuf};

use super::Account;
use crate::bundle::Step;
use crate::digest::{DigestTool, Pin, download};
use crate::ports::Invocation;

/// A pinned `NuGet` package and the directory in it to unpack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Package {
    /// The package's id, as `NuGet` spells it.
    pub id: &'static str,
    /// Its pinned version.
    pub version: &'static str,
    /// The `.nupkg`'s pin.
    pub pin: Pin,
    /// The x64 directory inside it, unpacked with its leading directories
    /// stripped.
    pub member: &'static str,
}

impl Package {
    /// The `.nupkg`'s file name, `<id-lower>.<version>.nupkg`.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.{}.nupkg", self.id.to_ascii_lowercase(), self.version)
    }

    /// Where `NuGet` serves the `.nupkg`: its flat container.
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "https://api.nuget.org/v3-flatcontainer/{}/{}/{}",
            self.id.to_ascii_lowercase(),
            self.version,
            self.file_name()
        )
    }

    /// How many leading directories `tar` strips: those of [`Self::member`].
    #[must_use]
    pub fn depth(&self) -> usize {
        self.member.split('/').count()
    }
}

/// The Artifact Signing dlib.
pub const DLIB: Package = Package {
    id: "Microsoft.ArtifactSigning.Client",
    version: "1.0.146",
    pin: Pin {
        what: "Microsoft.ArtifactSigning.Client 1.0.146",
        sha256: "7761e5d8f31e54b764da21d33cd8b0a2940e1b05526c7dfcb5e94b0431426f70",
    },
    member: "bin/x64",
};

/// The Windows SDK build tools, for `signtool.exe`.
pub const SIGNTOOL: Package = Package {
    id: "Microsoft.Windows.SDK.BuildTools",
    version: "10.0.26100.4188",
    pin: Pin {
        what: "Microsoft.Windows.SDK.BuildTools 10.0.26100.4188",
        sha256: "180deb372659029864c10a0c04787833234d64aacd1d2c0661d2c00295d8e022",
    },
    member: "bin/10.0.26100.0/x64",
};

/// The dlib's file name in [`DLIB`]'s x64 directory.
pub const DLIB_FILE: &str = "Azure.CodeSigning.Dlib.dll";

/// The credential types the dlib's `DefaultAzureCredential` skips: every
/// one but `AzureCliCredential`, which `azure/login` signs in, so it
/// probes nothing else first (Microsoft Learn, "Set up signing
/// integrations to use Artifact Signing").
pub const EXCLUDED_CREDENTIALS: [&str; 9] = [
    "EnvironmentCredential",
    "WorkloadIdentityCredential",
    "ManagedIdentityCredential",
    "SharedTokenCacheCredential",
    "VisualStudioCredential",
    "VisualStudioCodeCredential",
    "AzurePowerShellCredential",
    "AzureDeveloperCliCredential",
    "InteractiveBrowserCredential",
];

/// Where the tools are downloaded, unpacked and configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticodeLayout {
    /// Everything the signing stages, removed and remade each run.
    pub root: PathBuf,
    /// Where the `.nupkg` files are downloaded.
    pub downloads: PathBuf,
    /// The dlib's x64 directory, unpacked.
    pub dlib_dir: PathBuf,
    /// `signtool.exe`'s directory, unpacked.
    pub signtool_dir: PathBuf,
}

impl AuthenticodeLayout {
    /// The layout under Cargo's `target_dir`.
    #[must_use]
    pub fn new(target_dir: &Path) -> Self {
        let root = target_dir.join("authenticode");
        Self {
            downloads: root.join("downloads"),
            dlib_dir: root.join("dlib"),
            signtool_dir: root.join("signtool"),
            root,
        }
    }

    /// The `.nupkg` of `package`, downloaded.
    #[must_use]
    pub fn nupkg(&self, package: &Package) -> PathBuf {
        self.downloads.join(package.file_name())
    }

    /// `signtool.exe`, unpacked.
    #[must_use]
    pub fn signtool_exe(&self) -> PathBuf {
        self.signtool_dir.join("signtool.exe")
    }

    /// The dlib, unpacked.
    #[must_use]
    pub fn dlib(&self) -> PathBuf {
        self.dlib_dir.join(DLIB_FILE)
    }

    /// The dlib's `metadata.json` (`signtool /dmdf`).
    #[must_use]
    pub fn metadata(&self) -> PathBuf {
        self.root.join("metadata.json")
    }

    /// Each package, with the directory it unpacks into.
    fn packages(&self) -> [(Package, &Path); 2] {
        [(DLIB, &self.dlib_dir), (SIGNTOOL, &self.signtool_dir)]
    }
}

/// The dlib's `metadata.json` for `account`.
#[must_use]
pub fn metadata_json(account: &Account) -> Vec<u8> {
    let mut json = serde_json::json!({
        "Endpoint": account.endpoint,
        "CodeSigningAccountName": account.account,
        "CertificateProfileName": account.profile,
        "ExcludeCredentials": EXCLUDED_CREDENTIALS,
    });
    if let Some(correlation) = &account.correlation {
        json["CorrelationId"] = correlation.as_str().into();
    }
    // A map of strings always serializes.
    serde_json::to_vec_pretty(&json).unwrap_or_default()
}

/// The steps that put the checked tools and `account`'s `metadata.json`
/// under `layout` (see the module docs).
#[must_use]
pub fn stage(layout: &AuthenticodeLayout, account: &Account) -> Vec<Step> {
    let mut steps = vec![
        Step::RemoveDir(layout.root.clone()),
        Step::CreateDir(layout.downloads.clone()),
        Step::CreateDir(layout.dlib_dir.clone()),
        Step::CreateDir(layout.signtool_dir.clone()),
    ];
    for (package, dir) in layout.packages() {
        let nupkg = layout.nupkg(&package);
        steps.extend([
            Step::Run(download(&nupkg, &package.url())),
            Step::CheckSha256 {
                path: nupkg.clone(),
                pin: package.pin,
                tool: DigestTool::Certutil,
            },
            Step::Run(
                Invocation::new("tar")
                    .arg("-xf")
                    .arg(&nupkg)
                    .arg("-C")
                    .arg(dir)
                    .arg("--strip-components")
                    .arg(package.depth().to_string())
                    .arg(package.member),
            ),
        ]);
    }
    steps.push(Step::Write {
        path: layout.metadata(),
        bytes: metadata_json(account),
    });
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> PathBuf {
        Path::new("/work").join("ev-nova").join("target")
    }

    fn account(correlation: Option<&str>) -> Account {
        Account {
            endpoint: "https://eus.codesigning.azure.net/".into(),
            account: "ev-nova-signing".into(),
            profile: "ev-nova-public".into(),
            correlation: correlation.map(str::to_owned),
        }
    }

    fn is_digest(text: &str) -> bool {
        text.len() == 64
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    }

    #[test]
    fn the_layout_lies_under_the_target_dir() {
        let layout = AuthenticodeLayout::new(&target());
        let root = target().join("authenticode");
        assert_eq!(
            layout,
            AuthenticodeLayout {
                root: root.clone(),
                downloads: root.join("downloads"),
                dlib_dir: root.join("dlib"),
                signtool_dir: root.join("signtool"),
            }
        );
        assert_eq!(
            layout.signtool_exe(),
            root.join("signtool").join("signtool.exe")
        );
        assert_eq!(
            layout.dlib(),
            root.join("dlib").join("Azure.CodeSigning.Dlib.dll")
        );
        assert_eq!(layout.metadata(), root.join("metadata.json"));
        assert_eq!(
            layout.nupkg(&DLIB),
            root.join("downloads")
                .join("microsoft.artifactsigning.client.1.0.146.nupkg")
        );
    }

    #[test]
    fn each_pin_is_its_version_s_flat_container_package() {
        assert_eq!(
            DLIB.url(),
            "https://api.nuget.org/v3-flatcontainer/microsoft.artifactsigning.client/1.0.146/\
             microsoft.artifactsigning.client.1.0.146.nupkg"
        );
        assert_eq!(
            SIGNTOOL.url(),
            "https://api.nuget.org/v3-flatcontainer/microsoft.windows.sdk.buildtools/\
             10.0.26100.4188/microsoft.windows.sdk.buildtools.10.0.26100.4188.nupkg"
        );
        for package in [DLIB, SIGNTOOL] {
            assert!(is_digest(package.pin.sha256), "{package:?}");
            assert_eq!(
                package.pin.what,
                format!("{} {}", package.id, package.version)
            );
        }
        assert_ne!(DLIB.pin.sha256, SIGNTOOL.pin.sha256);
        assert_eq!((DLIB.depth(), SIGNTOOL.depth()), (2, 3));
    }

    fn curl(output: &Path, url: &str) -> Step {
        Step::Run(
            Invocation::new("curl")
                .args([
                    "--fail",
                    "--silent",
                    "--show-error",
                    "--location",
                    "--proto",
                    "=https",
                    "--retry",
                    "3",
                    "--output",
                ])
                .arg(output)
                .arg(url),
        )
    }

    #[test]
    fn stage_downloads_checks_and_unpacks_each_package_then_writes_the_metadata() {
        let layout = AuthenticodeLayout::new(&target());
        let account = account(None);
        let root = target().join("authenticode");
        let downloads = root.join("downloads");
        let dlib_pkg = downloads.join("microsoft.artifactsigning.client.1.0.146.nupkg");
        let sdk_pkg = downloads.join("microsoft.windows.sdk.buildtools.10.0.26100.4188.nupkg");
        assert_eq!(
            stage(&layout, &account),
            vec![
                Step::RemoveDir(root.clone()),
                Step::CreateDir(downloads.clone()),
                Step::CreateDir(root.join("dlib")),
                Step::CreateDir(root.join("signtool")),
                curl(&dlib_pkg, &DLIB.url()),
                Step::CheckSha256 {
                    path: dlib_pkg.clone(),
                    pin: DLIB.pin,
                    tool: DigestTool::Certutil,
                },
                Step::Run(
                    Invocation::new("tar")
                        .arg("-xf")
                        .arg(&dlib_pkg)
                        .arg("-C")
                        .arg(root.join("dlib"))
                        .args(["--strip-components", "2", "bin/x64"]),
                ),
                curl(&sdk_pkg, &SIGNTOOL.url()),
                Step::CheckSha256 {
                    path: sdk_pkg.clone(),
                    pin: SIGNTOOL.pin,
                    tool: DigestTool::Certutil,
                },
                Step::Run(
                    Invocation::new("tar")
                        .arg("-xf")
                        .arg(&sdk_pkg)
                        .arg("-C")
                        .arg(root.join("signtool"))
                        .args(["--strip-components", "3", "bin/10.0.26100.0/x64"]),
                ),
                Step::Write {
                    path: root.join("metadata.json"),
                    bytes: metadata_json(&account),
                },
            ]
        );
    }

    fn parsed(account: &Account) -> serde_json::Value {
        serde_json::from_slice(&metadata_json(account)).expect("json")
    }

    #[test]
    fn the_metadata_names_the_account_and_leaves_only_the_cli_credential() {
        let json = parsed(&account(None));
        assert_eq!(
            json,
            serde_json::json!({
                "Endpoint": "https://eus.codesigning.azure.net/",
                "CodeSigningAccountName": "ev-nova-signing",
                "CertificateProfileName": "ev-nova-public",
                "ExcludeCredentials": EXCLUDED_CREDENTIALS,
            })
        );
        assert!(!EXCLUDED_CREDENTIALS.contains(&"AzureCliCredential"));
    }

    #[test]
    fn the_metadata_carries_the_correlation_when_there_is_one() {
        let json = parsed(&account(Some("20261108120000")));
        assert_eq!(json["CorrelationId"], "20261108120000");
        assert_eq!(json["CodeSigningAccountName"], "ev-nova-signing");
    }
}
