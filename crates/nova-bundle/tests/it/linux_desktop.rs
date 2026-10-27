//! The Linux desktop file and AppStream metainfo, as the workspace's own
//! `[package.metadata.bundle]` renders them, run through the real
//! validators: `desktop-file-validate` and `appstreamcli validate`.
//!
//! It skips, passing, unless `NOVA_LINUX_VALIDATORS` is set and not
//! empty, which says the validators are installed, so neither
//! `mise run ci` nor a developer's test run needs them. CI's Linux `ci`
//! leg installs them (`desktop-file-utils` and `appstream`) and sets it.
//! With it set, a missing validator fails the test. It is built on every
//! host, and runs on any where the validators are.

use std::env;
use std::ffi::OsString;

use nova_bundle::linux::desktop::desktop_file_name;
use nova_bundle::linux::host::LinuxHost;
use nova_bundle::linux::metainfo::metainfo_file_name;
use nova_bundle::linux::share::write_share;
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::spec::BundleSpec;
use nova_bundle::system::{StdFiles, StdProcess};

const VARIABLE: &str = "NOVA_LINUX_VALIDATORS";

#[test]
fn the_rendered_desktop_file_and_metainfo_pass_the_validators() {
    if env::var_os(VARIABLE).is_none_or(|value| value.is_empty()) {
        eprintln!("{VARIABLE} is unset: skipping the Linux desktop validators");
        return;
    }
    let tmp = tempfile::tempdir().expect("temp dir");
    let cargo = OsString::from(env!("CARGO"));
    let host = LinuxHost {
        cargo: cargo.clone(),
        out: Some(tmp.path().to_owned()),
    };
    let out = write_share(&mut StdProcess, &mut StdFiles, &host).expect("the share tree");
    let json = StdProcess
        .run(&nova_bundle::bundle::metadata(&cargo))
        .expect("cargo metadata");
    let spec = BundleSpec::from_metadata(&json).expect("spec");
    let share = out.join("share");
    let desktop = share.join("applications").join(desktop_file_name(&spec));
    let metainfo = share.join("metainfo").join(metainfo_file_name(&spec));
    for invocation in [
        Invocation::new("desktop-file-validate").arg(&desktop),
        Invocation::new("appstreamcli")
            .args(["validate", "--no-net"])
            .arg(&metainfo),
    ] {
        // Its findings print to the test's output.
        StdProcess
            .run(&invocation)
            .unwrap_or_else(|e| panic!("`{invocation}`: {e}"));
    }
}
