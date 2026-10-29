//! The repo's GPG key, read through gpg itself.
//!
//! The committed file must hold exactly one public key and no secret
//! one: `gpg --with-colons --import-options show-only --import` lists it
//! without importing it, and [`fingerprint`] refuses any `sec` or `ssb`
//! record and takes the one `pub` record's fingerprint. So an accidentally
//! committed secret key fails loudly and is never published. `gpg
//! --dearmor` gives the binary key, whose base64 is the `.flatpakref`'s
//! `GPGKey=`. The secret key `sign` imports is checked the same way,
//! from `gpg --list-secret-keys`.
//!
//! gpg always runs in the work tree's own home (`--homedir`), in batch
//! mode, so the user's keyring is never read or written.

use std::ffi::OsStr;
use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::RepoError;
use crate::bundle;
use crate::ports::{Invocation, Process};
use crate::signing::SecretBytes;

/// What a key listing or a key holds that the repo can't use.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    /// The listing shows a secret key.
    #[error(
        "it holds a secret key: commit the public key alone \
         (`gpg --armor --export-options export-minimal --export <fingerprint>`)"
    )]
    Secret,
    /// The listing shows no primary key.
    #[error("it holds no {0} key")]
    NoKey(&'static str),
    /// The listing shows more than one primary key.
    #[error("it holds {count} {kind} keys, not one")]
    Several {
        /// How many.
        count: usize,
        /// `public` or `secret`.
        kind: &'static str,
    },
    /// The primary key has no fingerprint record.
    #[error("gpg lists no fingerprint for its {0} key")]
    NoFingerprint(&'static str),
    /// `gpg --dearmor` printed nothing.
    #[error("gpg --dearmor gives no key")]
    Empty,
    /// The secret's text isn't base64. The text is never shown.
    #[error("it is not standard base64")]
    NotBase64,
}

/// Which primary key a listing must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primary {
    /// A public key (`pub`), and no secret record at all.
    Public,
    /// A secret key (`sec`).
    Secret,
}

impl Primary {
    /// The colon listing's record for the primary key.
    fn record(self) -> &'static str {
        match self {
            Self::Public => "pub",
            Self::Secret => "sec",
        }
    }

    /// Its name, in an error.
    fn kind(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Secret => "secret",
        }
    }
}

/// The fingerprint of the one primary key in `listing`, gpg's
/// `--with-colons` output: field 10 of the `fpr` record right after the
/// `pub` (or `sec`) record, not a subkey's after `sub` or `ssb`.
///
/// # Errors
///
/// For [`Primary::Public`], any `sec` or `ssb` record; no primary key,
/// more than one, or one with no fingerprint.
pub fn fingerprint(listing: &str, primary: Primary) -> Result<String, KeyError> {
    let mut keys: Vec<Option<String>> = Vec::new();
    let mut previous = "";
    for line in listing.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        let record = fields[0];
        if primary == Primary::Public && matches!(record, "sec" | "ssb") {
            return Err(KeyError::Secret);
        }
        if record == primary.record() {
            keys.push(None);
        } else if record == "fpr"
            && previous == primary.record()
            && let Some(last) = keys.last_mut()
        {
            *last = fields
                .get(9)
                .filter(|f| !f.is_empty())
                .map(|f| (*f).to_owned());
        }
        previous = record;
    }
    match keys.as_slice() {
        [] => Err(KeyError::NoKey(primary.kind())),
        [Some(fingerprint)] => Ok(fingerprint.clone()),
        [None] => Err(KeyError::NoFingerprint(primary.kind())),
        _ => Err(KeyError::Several {
            count: keys.len(),
            kind: primary.kind(),
        }),
    }
}

/// `gpg` in the home `gnupg`, in batch mode, with these words.
fn gpg(gnupg: &Path, words: &[&str]) -> Invocation {
    Invocation::new("gpg")
        .arg("--homedir")
        .arg(gnupg)
        .args(["--batch", "--no-permission-warning"])
        .args(words)
}

/// Lists the keys in the file `key` without importing them.
#[must_use]
pub fn show(gnupg: &Path, key: &Path) -> Invocation {
    gpg(
        gnupg,
        &["--with-colons", "--import-options", "show-only", "--import"],
    )
    .arg(key)
    .captured()
}

/// The binary key the armored file `key` holds, on standard output.
#[must_use]
pub fn dearmor(gnupg: &Path, key: &Path) -> Invocation {
    gpg(gnupg, &["--output", "-", "--dearmor"])
        .arg(key)
        .captured()
}

/// Imports the key file `file`.
#[must_use]
pub fn import(gnupg: &Path, file: &Path) -> Invocation {
    gpg(gnupg, &["--import"]).arg(file)
}

/// Lists the home's secret keys.
#[must_use]
pub fn list_secret(gnupg: &Path) -> Invocation {
    gpg(gnupg, &["--with-colons", "--list-secret-keys"]).captured()
}

/// Stops the home's gpg-agent, which holds the imported key.
#[must_use]
pub fn kill_agent(gnupg: &Path) -> Invocation {
    Invocation::new("gpgconf")
        .arg("--homedir")
        .arg(gnupg)
        .args(["--kill", "gpg-agent"])
}

/// The committed public key, as gpg reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedKey {
    /// The primary key's fingerprint.
    pub fingerprint: String,
    /// The binary key.
    pub binary: Vec<u8>,
}

impl CommittedKey {
    /// The binary key as standard base64 on one line: the `.flatpakref`'s
    /// `GPGKey=`.
    #[must_use]
    pub fn base64(&self) -> String {
        STANDARD.encode(&self.binary)
    }
}

/// Reads the key file `key` with gpg in the home `gnupg`, which must
/// exist: its one public key's fingerprint, and the binary key.
///
/// # Errors
///
/// A gpg failure (the file isn't a key), or a file holding a secret key,
/// no public key or several ([`fingerprint`]), each naming the file.
pub fn inspect(
    process: &mut impl Process,
    gnupg: &Path,
    key: &Path,
) -> Result<CommittedKey, RepoError> {
    let refused = |source| RepoError::Key {
        path: key.to_owned(),
        source,
    };
    let listing = bundle::run(process, &show(gnupg, key))?;
    let fingerprint =
        fingerprint(&String::from_utf8_lossy(&listing), Primary::Public).map_err(refused)?;
    let binary = bundle::run(process, &dearmor(gnupg, key))?;
    if binary.is_empty() {
        return Err(refused(KeyError::Empty));
    }
    Ok(CommittedKey {
        fingerprint,
        binary,
    })
}

/// The secret key the variable's `text` holds as standard base64,
/// surrounding whitespace aside.
///
/// # Errors
///
/// Text that isn't base64, or is empty. The error never holds the text.
pub fn decode_secret(text: &OsStr) -> Result<SecretBytes, KeyError> {
    let text = text.to_str().ok_or(KeyError::NotBase64)?.trim();
    match STANDARD.decode(text) {
        Ok(bytes) if !bytes.is_empty() => Ok(SecretBytes(bytes)),
        _ => Err(KeyError::NotBase64),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The committed key's fingerprint in [`PUB_LISTING`].
    pub(crate) const FPR: &str = "0123456789ABCDEF0123456789ABCDEF01234567";
    /// Another key's.
    pub(crate) const OTHER_FPR: &str = "FEDCBA9876543210FEDCBA9876543210FEDCBA98";

    /// What `gpg --with-colons --import-options show-only --import` prints
    /// for an Ed25519 signing key with an encryption subkey: the primary's
    /// fingerprint, then the subkey's.
    pub(crate) const PUB_LISTING: &str = "\
pub:-:255:22:89ABCDEF01234567:1761600000:::-:::scESC::::::23::0:
fpr:::::::::0123456789ABCDEF0123456789ABCDEF01234567:
uid:-::::1761600000::ABCDEF::ev-nova test key::::::::::0:
sub:-:255:18:1111222233334444:1761600000::::::e::::::23:
fpr:::::::::99998888777766665555444433332222DDDDCCCC:
";

    /// What `gpg --with-colons --list-secret-keys` prints for `fpr`'s
    /// secret key.
    pub(crate) fn sec_listing(fpr: &str) -> String {
        format!(
            "sec:u:255:22:89ABCDEF01234567:1761600000:::u:::scESC:::+:::23::0:\n\
             fpr:::::::::{fpr}:\n\
             grp:::::::::AAAABBBBCCCCDDDDEEEEFFFF0000111122223333:\n\
             uid:u::::1761600000::ABCDEF::ev-nova test key::::::::::0:\n\
             ssb:u:255:18:1111222233334444:1761600000::::::e:::+:::23:\n\
             fpr:::::::::99998888777766665555444433332222DDDDCCCC:\n"
        )
    }

    #[test]
    fn a_public_listing_gives_the_primary_s_fingerprint_not_the_subkey_s() {
        assert_eq!(fingerprint(PUB_LISTING, Primary::Public), Ok(FPR.into()));
    }

    #[test]
    fn a_secret_listing_gives_the_primary_s_fingerprint_not_the_subkey_s() {
        assert_eq!(
            fingerprint(&sec_listing(FPR), Primary::Secret),
            Ok(FPR.into())
        );
    }

    #[test]
    fn a_committed_file_holding_a_secret_key_is_refused() {
        assert_eq!(
            fingerprint(&sec_listing(FPR), Primary::Public),
            Err(KeyError::Secret)
        );
        let subkey_only = format!("{PUB_LISTING}ssb:u:255:18:1111:1761600000::::::e:::+:::23:\n");
        assert_eq!(
            fingerprint(&subkey_only, Primary::Public),
            Err(KeyError::Secret)
        );
    }

    #[test]
    fn a_listing_with_no_key_two_keys_or_no_fingerprint_is_refused() {
        assert_eq!(
            fingerprint("", Primary::Public),
            Err(KeyError::NoKey("public"))
        );
        assert_eq!(
            fingerprint(PUB_LISTING, Primary::Secret),
            Err(KeyError::NoKey("secret"))
        );
        assert_eq!(
            fingerprint(&format!("{PUB_LISTING}{PUB_LISTING}"), Primary::Public),
            Err(KeyError::Several {
                count: 2,
                kind: "public"
            })
        );
        let unprinted = "pub:-:255:22:89AB:1761600000:::-:::scESC::::::23::0:\n\
                         uid:-::::1761600000::ABCDEF::k::::::::::0:\n";
        assert_eq!(
            fingerprint(unprinted, Primary::Public),
            Err(KeyError::NoFingerprint("public"))
        );
        let empty = "pub:-:255:22:89AB:1761600000:::-:::scESC::::::23::0:\nfpr:::::::::\n";
        assert_eq!(
            fingerprint(empty, Primary::Public),
            Err(KeyError::NoFingerprint("public"))
        );
    }

    #[test]
    fn the_key_s_base64_is_its_binary_on_one_line() {
        let key = CommittedKey {
            fingerprint: FPR.into(),
            binary: vec![0x98, 0x33, 0x04, 0xff, 0x00, 0x10],
        };
        assert_eq!(key.base64(), "mDME/wAQ");
    }

    #[test]
    fn a_secret_decodes_from_base64_and_bad_text_is_refused_without_echoing_it() {
        assert_eq!(
            decode_secret(OsStr::new(" lQ==\n")),
            Ok(SecretBytes(vec![0x95]))
        );
        for bad in ["not base64!", "", "  "] {
            let error = decode_secret(OsStr::new(bad)).expect_err(bad);
            assert_eq!(error, KeyError::NotBase64);
            if !bad.trim().is_empty() {
                assert!(!error.to_string().contains(bad), "{error}");
            }
        }
    }

    #[test]
    fn gpg_runs_in_the_work_tree_s_home_in_batch_mode() {
        let home = Path::new("/w").join("gnupg");
        let key = Path::new("/r").join("repo-key.asc");
        let show = show(&home, &key);
        assert_eq!(show.program, "gpg");
        assert!(show.capture);
        assert_eq!(
            show.args,
            [
                "--homedir".into(),
                home.clone().into_os_string(),
                "--batch".into(),
                "--no-permission-warning".into(),
                "--with-colons".into(),
                "--import-options".into(),
                "show-only".into(),
                "--import".into(),
                key.clone().into_os_string(),
            ]
        );
        assert_eq!(
            dearmor(&home, &key).args[4..],
            [
                "--output".into(),
                "-".into(),
                "--dearmor".into(),
                key.into_os_string()
            ]
        );
        assert!(list_secret(&home).capture);
        assert_eq!(
            kill_agent(&home).args,
            [
                "--homedir".into(),
                home.into_os_string(),
                "--kill".into(),
                "gpg-agent".into()
            ]
        );
    }
}
