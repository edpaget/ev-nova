//! What the [`Renderer`](crate::Renderer) has uploaded of one kind, by
//! pointer ([`Resident`]): the one table behind a model's textures
//! (`crate::models`) and a sky's images (`crate::sky_images`).
//!
//! An item is numbered and uploaded the first time its pointer is asked
//! for, and every later holder of the same pointer names the same number.
//! Two items that are equal behind two pointers are two uploads: nothing
//! is hashed or compared, at load or on a frame. The numbers run from 0,
//! and none is ever given twice.
//!
//! The table keeps a weak pointer to each item it has numbered, so the
//! item's address stays its own: an address is not handed to another item
//! while any pointer to its first holder, weak or strong, is alive. An
//! item nothing else holds any more therefore keeps its number, and stays
//! on the GPU: the port has no call that drops one.
//!
//! The table knows nothing of the GPU. Its caller says how an item is
//! uploaded, so the one rule serves each kind's own port call.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

/// The items of one kind the GPU holds, by pointer, each under the `Id`
/// it took (see the [module docs](self)).
#[derive(Debug)]
pub(crate) struct Resident<T, Id> {
    /// Each item uploaded, by its pointer's address, with the number it
    /// took. The weak pointer keeps the address its own.
    held: HashMap<usize, (Weak<T>, Id)>,
    /// The number the next new item takes, so how many it has numbered:
    /// open to the crate for the tests of what holds a table, which read
    /// that it numbered nothing.
    pub(crate) next: u32,
}

impl<T, Id> Default for Resident<T, Id> {
    fn default() -> Self {
        Self {
            held: HashMap::new(),
            next: 0,
        }
    }
}

impl<T, Id: Copy + From<u32>> Resident<T, Id> {
    /// The number `item` is uploaded under, uploading it with `upload`
    /// first if this pointer hasn't been.
    pub(crate) fn id(&mut self, item: &Arc<T>, upload: impl FnOnce(Id, &T)) -> Id {
        let address = Arc::as_ptr(item) as usize;
        if let Some((_, id)) = self.held.get(&address) {
            return *id;
        }
        let id = Id::from(self.next);
        self.next += 1;
        upload(id, item);
        self.held.insert(address, (Arc::downgrade(item), id));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A number of the table's, as a kind's own id is.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Number(u32);

    impl From<u32> for Number {
        fn from(number: u32) -> Self {
            Self(number)
        }
    }

    /// The table, and each upload it asked for, in order.
    #[derive(Default)]
    struct Table {
        resident: Resident<String, Number>,
        uploads: Vec<(Number, String)>,
    }

    impl Table {
        /// The number the table gives `item`, recording its upload.
        fn id(&mut self, item: &Arc<String>) -> Number {
            let uploads = &mut self.uploads;
            self.resident
                .id(item, |id, item| uploads.push((id, item.clone())))
        }
    }

    fn item(text: &str) -> Arc<String> {
        Arc::new(text.to_owned())
    }

    #[test]
    fn an_item_is_numbered_and_uploaded_the_first_time_and_only_then() {
        let mut table = Table::default();
        let (first, second) = (item("first"), item("second"));
        assert_eq!(table.id(&first), Number(0));
        assert_eq!(table.id(&second), Number(1));
        // Asked again, by the same pointer or a clone of it.
        assert_eq!(table.id(&first), Number(0));
        assert_eq!(table.id(&Arc::clone(&second)), Number(1));
        assert_eq!(
            table.uploads,
            [
                (Number(0), "first".to_owned()),
                (Number(1), "second".to_owned())
            ]
        );
        assert_eq!(table.resident.next, 2, "the next takes the third number");
    }

    #[test]
    fn two_pointers_to_equal_items_are_two_uploads() {
        let mut table = Table::default();
        let (one, other) = (item("same"), item("same"));
        assert_eq!(*one, *other);
        assert_eq!(table.id(&one), Number(0));
        assert_eq!(table.id(&other), Number(1));
        assert_eq!(table.uploads.len(), 2);
    }

    #[test]
    fn a_dropped_item_s_number_is_never_another_item_s() {
        let mut table = Table::default();
        // Each item is dropped before the next is made, so without the
        // table keeping its address, the next could be given the same one.
        for expected in 0..8 {
            let item = item(&expected.to_string());
            assert_eq!(table.id(&item), Number(expected));
            assert_eq!(table.id(&item), Number(expected));
        }
        let uploaded: Vec<(u32, &str)> = (table.uploads.iter())
            .map(|(id, item)| (id.0, item.as_str()))
            .collect();
        assert_eq!(
            uploaded,
            [
                (0, "0"),
                (1, "1"),
                (2, "2"),
                (3, "3"),
                (4, "4"),
                (5, "5"),
                (6, "6"),
                (7, "7")
            ]
        );
    }

    #[test]
    fn a_new_table_has_numbered_nothing() {
        let table = Table::default();
        assert_eq!(table.resident.next, 0);
        assert!(table.resident.held.is_empty());
    }
}
