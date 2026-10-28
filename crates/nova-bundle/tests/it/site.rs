//! The download page written through the real filesystem adapter, into a
//! directory that doesn't exist yet.

use std::fs;

use nova_bundle::site::{FlatpakRepo, INDEX, render, stock, write_site};
use nova_bundle::system::StdFiles;

#[test]
fn the_stock_page_is_written_as_index_html_in_a_new_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out = tmp.path().join("_site");
    let page = write_site(&mut StdFiles, &out, &stock(FlatpakRepo::Published)).expect("written");
    assert_eq!(page, out.join(INDEX));
    assert_eq!(
        fs::read_to_string(&page).expect("the page"),
        render(&stock(FlatpakRepo::Published))
    );
}
