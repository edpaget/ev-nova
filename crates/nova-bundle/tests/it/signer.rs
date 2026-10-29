//! `Ed25519Signer` against RFC 8032's published test vector, and the
//! rule that it never shows a seed. Every seed here is a test value,
//! never a real key.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use nova_bundle::ports::Signer;
use nova_bundle::system::Ed25519Signer;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

/// RFC 8032, section 7.1, TEST 1: a published test vector.
const SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const SIGNATURE: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065\
                         224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

#[test]
fn signs_rfc_8032_test_1() {
    let signer = Ed25519Signer::from_base64_seed(&STANDARD.encode(hex(SECRET))).expect("a seed");
    assert_eq!(signer.public_key().0.to_vec(), hex(PUBLIC));
    assert_eq!(signer.sign(b"").to_vec(), hex(SIGNATURE));
}

#[test]
fn a_malformed_seed_is_an_error_that_never_shows_it() {
    for seed in [
        STANDARD.encode(b"thirty-one bytes of test seed!!"),
        STANDARD.encode(b"thirty-three bytes of test seed!!"),
        "test seed, not base64".to_owned(),
    ] {
        let error = Ed25519Signer::from_base64_seed(&seed).expect_err("malformed");
        assert!(!error.to_string().contains(&seed), "{error}");
        assert!(!format!("{error:?}").contains(&seed), "{error:?}");
    }
}

#[test]
fn the_signer_s_debug_shows_no_key() {
    let seed = hex(SECRET);
    let signer = Ed25519Signer::from_base64_seed(&STANDARD.encode(&seed)).expect("a seed");
    let debug = format!("{signer:?}");
    assert!(!debug.contains(SECRET), "{debug}");
    assert!(!debug.contains(&STANDARD.encode(&seed)), "{debug}");
    assert!(!debug.contains(&format!("{seed:?}")), "{debug}");
    assert!(debug.contains(&signer.public_key().to_base64()), "{debug}");
}
