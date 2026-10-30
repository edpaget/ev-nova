//! The download page written through the real filesystem adapter, into a
//! directory that doesn't exist yet, from a `pages-flatpak` tree and a beta
//! asset list read through it.

use std::fs;

use nova_bundle::appcast::Channel;
use nova_bundle::site::{Deploy, INDEX, deploy, render, stock, write_site};
use nova_bundle::system::StdFiles;

#[test]
fn the_stock_page_is_written_as_index_html_in_a_new_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let tree = tmp.path().join("pages-flatpak");
    fs::create_dir_all(tree.join("repo")).expect("the tree");
    fs::write(tree.join("ev-nova-nightly.flatpakref"), "").expect("a ref");
    fs::write(tree.join("ev-nova-beta.flatpakref"), "").expect("a ref");
    let list = tmp.path().join("beta.txt");
    fs::write(&list, "ev-nova-aarch64-apple-darwin.dmg\n").expect("the list");
    let read = deploy(&StdFiles, Some(&tree), Some(&list)).expect("read");
    assert_eq!(
        read,
        Deploy {
            flatpakrefs: vec![Channel::Nightly, Channel::Beta],
            beta_assets: vec!["ev-nova-aarch64-apple-darwin.dmg".to_owned()],
        }
    );
    let out = tmp.path().join("_site");
    let page = write_site(&mut StdFiles, &out, &stock(&read)).expect("written");
    assert_eq!(page, out.join(INDEX));
    assert_eq!(
        fs::read_to_string(&page).expect("the page"),
        render(&stock(&read))
    );
}
