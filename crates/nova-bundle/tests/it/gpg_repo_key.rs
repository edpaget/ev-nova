//! The Flatpak repo's key checks (`nova_bundle::linux::flatpak::repo::key`)
//! against the real gpg, with a throwaway key (any Unix, opt-in).
//!
//! It skips, passing, unless `NOVA_GPG` is set, so neither `mise run ci`
//! nor a developer's test run needs gpg. CI's Flatpak leg sets it. The
//! unit tests read canned colon listings; this checks gpg prints what they
//! assume: a committed public key is accepted with its fingerprint and
//! its binary, a committed secret key is refused, and an imported secret
//! key lists under its fingerprint.

use std::env;
use std::fs;

use nova_bundle::linux::flatpak::repo::RepoError;
use nova_bundle::linux::flatpak::repo::key::{self, KeyError, Primary};
use nova_bundle::ports::Process;
use nova_bundle::system::StdProcess;

use crate::throwaway_key::{ThrowawayKey, kill_agent};

const VARIABLE: &str = "NOVA_GPG";

fn enabled() -> bool {
    let on = env::var_os(VARIABLE).is_some_and(|v| !v.is_empty());
    if !on {
        eprintln!("{VARIABLE} is unset: skipping the gpg key test");
    }
    on
}

#[test]
fn the_real_gpg_accepts_a_public_key_and_refuses_a_secret_one() {
    if !enabled() {
        return;
    }
    let generated = ThrowawayKey::new();
    let check = generated.dir().join("check");
    fs::create_dir_all(&check).expect("the check home");

    let public = generated.dir().join("repo-key.asc");
    generated.write_public(&public);
    let committed = key::inspect(&mut StdProcess, &check, &public).expect("a public key");
    assert_eq!(committed.fingerprint, generated.fingerprint);
    assert_eq!(committed.binary, generated.public_binary());
    assert!(!committed.base64().contains('\n'));

    let secret = generated.dir().join("secret.asc");
    generated.write_secret_armored(&secret);
    let refused = key::inspect(&mut StdProcess, &check, &secret).expect_err("a secret key");
    assert!(
        matches!(
            refused,
            RepoError::Key {
                source: KeyError::Secret,
                ..
            }
        ),
        "{refused}"
    );
    fs::remove_file(&secret).expect("the secret file removed");
    kill_agent(&check);
}

#[test]
fn an_imported_secret_lists_under_its_fingerprint() {
    if !enabled() {
        return;
    }
    let generated = ThrowawayKey::new();
    let home = generated.dir().join("signing");
    fs::create_dir_all(&home).expect("the signing home");
    let file = generated.dir().join("key.gpg");
    let bytes = key::decode_secret(generated.secret_base64().as_ref()).expect("base64");
    fs::write(&file, &bytes.0).expect("the secret file");
    StdProcess
        .run(&key::import(&home, &file))
        .expect("gpg --import");
    fs::remove_file(&file).expect("the secret file removed");
    let listing = StdProcess
        .run(&key::list_secret(&home))
        .expect("gpg --list-secret-keys");
    assert_eq!(
        key::fingerprint(&String::from_utf8_lossy(&listing), Primary::Secret),
        Ok(generated.fingerprint.clone())
    );
    StdProcess
        .run(&key::kill_agent(&home))
        .expect("gpgconf --kill gpg-agent");
}
