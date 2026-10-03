//! A plug-in written as a `.rez` file reads exactly like the same plug-in
//! written as a resource fork.

use nova_rsrc::fixture::{ForkBuilder, REZ_ENTRY_COUNT_FIELD, RezBuilder};
use nova_rsrc::{ResType, ResourceFile};

const SHIP: ResType = ResType([b's', b'h', 0x95, b'p']);
const PICT: ResType = ResType(*b"PICT");
const DESC: ResType = ResType(*b"d\xEBsc");

/// One resource: `(type, id, name, data)`.
type Spec = (ResType, i16, Option<&'static [u8]>, &'static [u8]);

/// Several types, named and unnamed resources, a
/// high-bit type code and name, empty data, and two resources with the same
/// data so the `.rez` side shares one blob.
///
/// No name is empty: a `.rez` name field cannot tell an empty name from no
/// name, so an empty name reads back as unnamed there, unlike in a fork.
const PLUG_IN: &[Spec] = &[
    (SHIP, 128, Some(b"Shuttle"), b"ship-128"),
    (PICT, -5, None, b""),
    (SHIP, 129, Some(b"K\x8Ase"), b"shared"),
    (DESC, 0, Some(b"Desc"), b"desc"),
    (PICT, 200, None, b"shared"),
    (SHIP, -1, None, b"ship-neg"),
];

/// Every resource's `(type, id, name bytes, name, data)`, type by type.
type View<'a> = Vec<(ResType, i16, Option<&'a [u8]>, Option<&'a str>, &'a [u8])>;

fn view(file: &ResourceFile) -> View<'_> {
    file.iter()
        .map(|r| (r.res_type(), r.id(), r.name_bytes(), r.name(), r.data()))
        .collect()
}

#[test]
fn a_rez_plug_in_reads_like_the_same_fork() {
    let (fork, rez) = PLUG_IN.iter().fold(
        (ForkBuilder::new(), RezBuilder::new()),
        |(fork, rez), &(ty, id, name, data)| {
            (
                fork.resource(ty, id, name, data),
                rez.resource(ty, id, name, data),
            )
        },
    );
    let rez_bytes = rez.build().bytes;

    // Six resources, five distinct blobs, plus the map row.
    let field = &rez_bytes[REZ_ENTRY_COUNT_FIELD..REZ_ENTRY_COUNT_FIELD + 4];
    let rows = u32::from_le_bytes(field.try_into().expect("four bytes"));
    assert_eq!(rows - 1, 5, "blob rows");
    assert!((rows as usize - 1) < PLUG_IN.len(), "data is shared");

    let fork = ResourceFile::from_bytes(fork.build().bytes).expect("fork parses");
    let rez = ResourceFile::from_bytes(rez_bytes).expect(".rez parses");

    assert_eq!(rez.types().collect::<Vec<_>>(), vec![SHIP, PICT, DESC]);
    assert_eq!(
        rez.types().collect::<Vec<_>>(),
        fork.types().collect::<Vec<_>>()
    );
    for ty in fork.types() {
        assert_eq!(rez.count(ty), fork.count(ty), "{ty}");
    }
    assert_eq!(rez.len(), PLUG_IN.len());
    assert_eq!(view(&rez), view(&fork));
    assert_eq!(rez.get(SHIP, 129).expect("present").name(), Some("Käse"));
}
