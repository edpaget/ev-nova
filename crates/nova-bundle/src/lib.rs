//! `nova-bundle`: packs the nightly's and the beta's installers. Its `nova-bundle`
//! binary builds `Nova.app`, the macOS bundle of the game, and packs it
//! into a `.dmg`; its `nova-installer` binary packs the built Windows
//! executables into a per-user MSI ([`installer`]); its `nova-linux`
//! binary writes the Linux desktop file, AppStream metainfo and icons
//! ([`linux`]); its `nova-flatpak` binary packs the game into the Linux
//! Flatpak ([`linux::flatpak`]); its `nova-build-id` binary prints the build ID
//! ([`build_id`]); its `nova-site` binary renders the download page that
//! links them all ([`site`]); and its `nova-appcast` binary signs and
//! writes the update feeds and checks a promotion ([`appcast`]).
//!
//! # Usage
//!
//! ```text
//! mise run bundle -- [OUT_DIR]
//! ```
//!
//! On macOS only: it needs `lipo`, `codesign` and `hdiutil`. It builds
//! `nova` and `nova-dump` for Apple silicon (`aarch64-apple-darwin`; Intel
//! Macs are not supported) in release, stages `Nova.app` (its `Info.plist`
//! from the `nova` package's version and `[package.metadata.bundle]`
//! table, and its icon) with `nova-dump`, `README.txt`, `LICENSE` and an
//! `Applications` link beside it, checks with `lipo -archs` that both
//! binaries are arm64 alone, puts Sparkle in the app (below), signs and
//! verifies them all, and writes `ev-nova-aarch64-apple-darwin.dmg` to
//! `OUT_DIR` (by default `target/bundle/out`). The layout is [`layout`]'s,
//! and the sequence [`bundle::plan`].
//!
//! # Updates (Sparkle)
//!
//! Every bundle carries Sparkle [`sparkle::VERSION`] in
//! `Contents/Frameworks`, so the bundle has one shape and every run signs
//! it. The bundler downloads the release archive with `curl` (bundling
//! needs the network), checks its SHA-256 against [`sparkle::SHA256`]
//! before unpacking it, removes the XPC services (the game isn't
//! sandboxed), thins each Mach-O to arm64 and checks it, copies the
//! framework in with its links, and ships Sparkle's licence as
//! `Contents/Resources/Sparkle-LICENSE.txt`. Both signing modes sign its
//! code inside out ([`sparkle::nested_code`]) before `nova-dump` and the
//! app. See [`sparkle`].
//!
//! `NOVA_CHANNEL` names the channel the bundle is for ([`channel`]): CI
//! sets `nightly`, or `beta` for a promotion, and unset is a development
//! bundle; any other value exits 2. When there is a channel and the game package commits
//! `update_public_key`, `Info.plist` gains Sparkle's three keys:
//! `SUFeedURL` (the channel's macOS appcast, [`channel::feed_url`]),
//! `SUPublicEDKey` (the key, standard base64) and
//! `SUEnableAutomaticChecks`. Otherwise it holds no `SU*` key, so a bundle
//! never names a feed it couldn't verify, and the app runs no updater. The
//! bundler prints which on standard error ([`channel::FeedStatus`]).
//!
//! # Signing
//!
//! Six environment variables, named as the CI secrets that fill them
//! ([`signing::VARS`]), choose how:
//!
//! - `MACOS_CERT_P12_BASE64` and `MACOS_CERT_PASSWORD`: the Developer ID
//!   Application certificate and its key, as a base64 `.p12`, and its
//!   password.
//! - `MACOS_SIGNING_IDENTITY`: the `codesign` identity,
//!   `Developer ID Application: <name> (<team>)`.
//! - `APPLE_API_KEY_P8_BASE64`, `APPLE_API_KEY_ID` and
//!   `APPLE_API_ISSUER_ID`: the App Store Connect API key the notary
//!   service is reached with.
//!
//! With all six set, the bundler signs with Developer ID and the hardened
//! runtime from a temporary keychain, notarizes and staples `Nova.app`,
//! then makes the `.dmg` from it and signs, notarizes and staples that
//! ([`signing::developer_id`], [`notary`]). Gatekeeper then assesses the
//! stapled app and `.dmg` (`spctl --assess`), and a rejection fails the
//! bundle. With none set (a local build,
//! a fork's pull request) it signs ad-hoc and notarizes nothing. Any other
//! mix is an error naming the missing variables.
//!
//! The core builds every argument list and runs them through the
//! [`ports::Process`] port, writes the staging tree through the
//! [`ports::Files`] port, and reads the signing variables through the
//! [`ports::Env`] port; [`system`] holds the real adapters, which `main`
//! wires together.
//!
//! # The Windows installer
//!
//! ```text
//! mise run build-windows
//! mise run pack-installer -- --exes <DIR> [OUT_DIR]
//! ```
//!
//! Or `mise run installer`, which runs both. On Windows only, with WiX v5
//! installed (`dotnet tool install --global wix --version 5.0.2`). The
//! first builds `nova.exe` and `nova-dump.exe` in release for
//! `x86_64-pc-windows-msvc`. The second, `nova-installer`, never builds:
//! it packs the executables in `<DIR>` (by default
//! `target/x86_64-pc-windows-msvc/release`) as they are, with `README.txt`
//! and `LICENSE`, into `ev-nova-x86_64-pc-windows-msvc.msi` in `OUT_DIR`
//! (by default `target/installer/out`), so the executables can be signed
//! between the two steps and the `.msi` after them. See [`installer`].
//!
//! Every installer carries WinSparkle [`installer::winsparkle::VERSION`]
//! beside `nova.exe`, with its two licence files: the packer downloads the
//! release archive with `curl` (packing needs the network), checks its
//! SHA-256 with `certutil` before unpacking it with `tar`, and checks the
//! DLL's before `wix build` packs it ([`digest`]). An update's install
//! relaunches the game when the feed passes
//! [`installer::RELAUNCH_PROPERTY`]. `NOVA_CHANNEL` is read as for the
//! macOS bundle, and the packer prints which feed the build names.

//!
//! # The Linux desktop files
//!
//! ```text
//! mise run linux-desktop -- [OUT_DIR]
//! ```
//!
//! On any host: `nova-linux` writes `<identifier>.desktop`,
//! `<identifier>.metainfo.xml` and the hicolor icons as a `share/` tree
//! in `OUT_DIR` (by default `target/linux`), which the player copies into
//! `~/.local/share`. The nightly's Linux tarball carries it. See
//! [`linux`]. The opt-in tarball test checks with `objdump -p` that the
//! tarball's `nova` needs no glibc above [`linux::links::GLIBC_FLOOR`]
//! and links only [`linux::links::ALLOWED_LIBRARIES`].
//!
//! # The Flatpak
//!
//! ```text
//! mise run flatpak -- [OUT_DIR]
//! ```
//!
//! On x86-64 Linux only. `nova-flatpak` vendors the crates, renders the
//! `flatpak-builder` manifest, builds it offline on the freedesktop
//! runtime and packs `ev-nova-x86_64-unknown-linux-gnu.flatpak` in
//! `OUT_DIR` (by default `target/flatpak/out`); it needs `flatpak`,
//! `flatpak-builder` and the Flathub remote added with `--user`. It
//! exports onto `NOVA_CHANNEL`'s branch (`nightly` in CI), or `master` for
//! a development build ([`linux::flatpak::branch`]).
//!
//! # The Flatpak repository
//!
//! ```text
//! mise run flatpak-repo -- stage [--bundle-dir <DIR>] --work <DIR> --pages <DIR>
//! mise run flatpak-repo -- sign --work <DIR> --pages <DIR> --dist <DIR>
//! ```
//!
//! On x86-64 Linux, and in CI's main-only `flatpak-repo` job, which runs
//! the built binary. `stage` mirrors the live repo from the Pages site,
//! GPG-verified against the committed
//! `packaging/linux/flatpak/repo-key.asc`, and, for the nightly, imports
//! the plain bundle in `--bundle-dir`; `sign`, which alone holds
//! `FLATPAK_SIGNING_KEY`, commits it onto the channel's branch, signs,
//! prunes and makes static deltas, and writes every channel's
//! `ev-nova-<channel>.flatpakref` the repo holds and a bundle that follows
//! the repo. For beta, `stage` imports nothing and `sign` copies the
//! nightly branch's commit of the build instead (see "Promotion" below).
//! Without the channel, the committed key or the secret, `stage` warns and
//! exits 4, and the nightly publishes as before. See
//! [`linux::flatpak::repo`].
//!
//! # The build ID
//!
//! ```text
//! mise run build-id
//! ```
//!
//! Every package names its build by HEAD's committer time in UTC,
//! `YYYYMMDDHHMMSS` ([`build_id::BuildId`]), or by `NOVA_BUILD_ID` when it
//! is set, which is how CI gives every leg of a run the same one
//! ([`build_id::resolve`]). It is `Info.plist`'s `CFBundleVersion` and the
//! AppStream metainfo's `<release>`, and each bundler compiles `nova` with
//! it in `NOVA_BUILD_ID`, so the game reports the build it is.
//! `nova-build-id` prints it, and exits 1 when it has none (no variable
//! and no git checkout).
//!
//! # The download page
//!
//! ```text
//! mise run site -- [OUT_DIR]
//! ```
//!
//! On any host: `nova-site` writes the download page, `index.html`, into
//! `OUT_DIR` (by default `target/site`), linking each nightly asset by the
//! name its packer gives it, and, with `--beta-assets <FILE>`, each asset
//! the beta release holds; with `--pages-flatpak <DIR>`, each channel's
//! served `.flatpakref`. CI's `pages` job deploys it at the GitHub Pages
//! site's root. `nova-site --archive <TARGET>` and
//! `--archive-dir <TARGET>` print the Windows zip's or Linux tarball's
//! file name or top-level directory ([`archive`]), which CI's packaging
//! uses instead of spelling them. See [`site`].
//!
//! # The update feeds
//!
//! ```text
//! mise run appcast -- publish --dist <DIR> --assets <FILE> --current <DIR> \
//!     --feeds <DIR> --prune-list <FILE>
//! mise run appcast -- public-key
//! ```
//!
//! On any host, and in CI's `publish` job. `publish` writes the feeds of
//! the channel `NOVA_CHANNEL` names, which it requires. It reads the stable
//! installers in `--dist` (`ev-nova-aarch64-apple-darwin.dmg` and
//! `ev-nova-x86_64-pc-windows-msvc.msi`), the release's asset names in
//! `--assets` (one per line) and its current feeds, downloaded into
//! `--current`. When the game package's `[package.metadata.bundle]` sets
//! `update_public_key` and `UPDATE_SIGNING_KEY` holds its seed (32 bytes,
//! standard base64), it copies each installer to its build-ID name in
//! `--dist` (`ev-nova-<id>-<target>.<ext>`), signs it with Ed25519, and
//! writes `appcast-macos-<channel>.xml` and `appcast-windows-<channel>.xml`
//! into `--feeds`. When either half is missing it skips the feeds and
//! prints a `::warning::`, and the nightly publishes without them; a seed
//! that is malformed or doesn't match the key fails. Either way it writes
//! the release assets to delete into `--prune-list`, one per line.
//!
//! It refuses a build that isn't newer than a current feed's, so a
//! clock-skewed commit can't roll installs back: an older build exits 1,
//! and the very build the feeds already offer (a re-run) exits 3,
//! writing nothing. The build ID is [`build_id::resolve`]'s. `public-key`
//! prints `UPDATE_SIGNING_KEY`'s public key, the value to commit. See
//! [`appcast`].
//!
//! # Promotion
//!
//! ```text
//! mise run appcast -- check-promotion --build <ID> --repo <DIR> \
//!     --nightly-assets <FILE> --beta-assets <FILE> --beta-current <DIR>
//! ```
//!
//! A beta is a nightly promoted by hand (`promote-beta.yml`): the
//! nightly's commit rebuilt with the same build ID and `NOVA_CHANNEL=beta`,
//! so its `Info.plist` and `nova.exe` name the beta feeds. `check-promotion`
//! is the workflow's first job. It refuses a run whose `GITHUB_REF` isn't
//! `main`, a malformed ID, a build the nightly release's asset list doesn't
//! hold both update files of, or one not newer than the beta feeds in
//! `--beta-current`, naming the IDs (exit 1); then it finds the one commit
//! in `--repo`'s history whose committer time is the build ID and prints
//! `build=<id>` and `commit=<sha>` ([`appcast::promote`]). The Flatpak is
//! not rebuilt: `nova-flatpak-repo` copies the nightly branch's commit onto
//! `beta` ([`linux::flatpak::repo::promote`]).

pub mod appcast;
pub mod archive;
pub mod build_id;
pub mod bundle;
pub mod channel;
pub mod digest;
pub mod host;
pub mod icon;
pub mod info_plist;
pub mod installer;
pub mod layout;
pub mod linux;
pub mod notary;
pub mod ports;
pub mod signing;
pub mod site;
pub mod sparkle;
pub mod spec;
pub mod system;
#[cfg(test)]
mod test_support;
pub mod xml;
