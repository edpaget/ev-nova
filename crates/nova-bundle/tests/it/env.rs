//! `StdEnv` reading the real environment. It only reads: the variable it
//! finds is one cargo sets for every test run.

use std::path::Path;

use nova_bundle::ports::Env;
use nova_bundle::system::StdEnv;

#[test]
fn reads_a_variable_the_runner_sets() {
    let dir = StdEnv.var("CARGO_MANIFEST_DIR").expect("cargo sets it");
    assert_eq!(Path::new(&dir), Path::new(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn an_unset_variable_is_none() {
    assert_eq!(StdEnv.var("NOVA_BUNDLE_NO_SUCH_VARIABLE_EVER"), None);
}
