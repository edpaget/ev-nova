//! A plug-in wrapped in MacBinary reads exactly like the bare fork it
//! wraps, and a corrupt one says what is wrong at both layers.

use std::error::Error as _;
use std::path::Path;

use nova_rsrc::fixture::forks::MemForks;
use nova_rsrc::fixture::{ForkBuilder, MacBinaryBuilder, MacBinaryVersion};
use nova_rsrc::{Fork, Format, LoadError, ParseError, ResType, ResourceFile};

const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);
const PICT: ResType = ResType(*b"PICT");
const DESC: ResType = ResType(*b"d\xEBsc");

fn fork() -> Vec<u8> {
    ForkBuilder::new()
        .resource(SHIP, 128, Some(b"Shuttle"), b"ship-128")
        .resource(PICT, -5, None, b"")
        .resource(SHIP, 129, Some(b"K\x8Ase"), b"ship-129")
        .attributes(0x20)
        .resource(DESC, 0, Some(b""), b"desc")
        .build()
        .bytes
}

/// Every resource's `(type, id, name bytes, name, attributes, data)`.
type View<'a> = Vec<(
    ResType,
    i16,
    Option<&'a [u8]>,
    Option<&'a str>,
    u8,
    &'a [u8],
)>;

fn view(file: &ResourceFile) -> View<'_> {
    file.iter()
        .map(|r| {
            (
                r.res_type(),
                r.id(),
                r.name_bytes(),
                r.name(),
                r.attributes(),
                r.data(),
            )
        })
        .collect()
}

#[test]
fn macbinary_ii_and_iii_plug_ins_read_like_the_bare_fork() {
    let bare = ResourceFile::from_bytes(fork()).expect("fork parses");
    assert_eq!(bare.len(), 4);
    for version in [MacBinaryVersion::II, MacBinaryVersion::III] {
        let wrapped = MacBinaryBuilder::new(fork()).version(version).build();
        let file = ResourceFile::from_bytes(wrapped.bytes).expect("MacBinary parses");
        assert_eq!(
            file.format(),
            Format::MacBinary {
                fork: Fork::Resource
            },
            "{version:?}"
        );
        assert_eq!(
            file.types().collect::<Vec<_>>(),
            bare.types().collect::<Vec<_>>()
        );
        assert_eq!(view(&file), view(&bare), "{version:?}");
    }
}

#[test]
fn a_malformed_fork_inside_macbinary_names_both_layers_when_loaded() {
    let path = Path::new("Plug-Ins/Broken.bin");
    let mut inner = fork();
    inner.truncate(30);
    let wrapped = MacBinaryBuilder::new(inner).build();
    let reader = MemForks::new().file(path, wrapped.bytes);

    let error = ResourceFile::load(&reader, path).expect_err("broken");
    assert!(matches!(&error, LoadError::Parse { path: p, .. } if p == path));
    let text = error.to_string();
    assert!(
        text.contains("in the MacBinary resource fork at byte 128: "),
        "{text}"
    );

    let layer = error.source().expect("MacBinary layer");
    assert!(matches!(
        layer.downcast_ref::<ParseError>(),
        Some(ParseError::MacBinaryResourceFork { offset: 128, .. })
    ));
    let fork_error = layer.source().expect("fork layer");
    assert!(matches!(
        fork_error.downcast_ref::<ParseError>(),
        Some(ParseError::SectionOutOfBounds { .. })
    ));
}
