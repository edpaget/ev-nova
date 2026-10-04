//! The spaceport's port, in the view's own terms: what it reads about the
//! stellar landed on. Its description comes through
//! [`DescriptionSource`](crate::ui::DescriptionSource), as `dësc` = `spöb`
//! ID.

use std::rc::Rc;

pub use nova_sim::StellarId;

/// What the spaceport shows of a stellar, raw from its `spöb`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortRecord {
    /// Its name.
    pub name: String,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `CustPicID`.
    pub cust_pic_id: i16,
    /// Its `Type`.
    pub graphic_type: i16,
}

/// The stellars and pictures the spaceport reads.
pub trait SpaceportCatalog {
    /// Stellar `id`'s record, or why it cannot be read.
    fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String>;
    /// Whether `PICT` `id` is in the game data.
    fn picture_exists(&self, id: i16) -> bool;
}

/// A borrowed catalog is a catalog.
impl<T: SpaceportCatalog + ?Sized> SpaceportCatalog for &T {
    fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
        (**self).stellar_port(id)
    }

    fn picture_exists(&self, id: i16) -> bool {
        (**self).picture_exists(id)
    }
}

/// A shared catalog is a catalog.
impl<T: SpaceportCatalog + ?Sized> SpaceportCatalog for Rc<T> {
    fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
        (**self).stellar_port(id)
    }

    fn picture_exists(&self, id: i16) -> bool {
        (**self).picture_exists(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every stellar is named by its ID; only even pictures exist.
    struct Fake;

    impl SpaceportCatalog for Fake {
        fn stellar_port(&self, id: StellarId) -> Result<PortRecord, String> {
            if id.0 < 0 {
                return Err(format!("no spöb {}", id.0));
            }
            Ok(PortRecord {
                name: id.0.to_string(),
                flags: 1,
                cust_pic_id: -1,
                graphic_type: 2,
            })
        }

        fn picture_exists(&self, id: i16) -> bool {
            id % 2 == 0
        }
    }

    fn read(catalog: impl SpaceportCatalog) -> String {
        format!(
            "{:?} {:?} {} {}",
            catalog.stellar_port(StellarId(130)).map(|port| port.name),
            catalog.stellar_port(StellarId(-1)),
            catalog.picture_exists(10_000),
            catalog.picture_exists(10_001),
        )
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        let direct = r#"Ok("130") Err("no spöb -1") true false"#;
        assert_eq!(read(Fake), direct);
        assert_eq!(read(&Fake), direct);
        assert_eq!(read(Rc::new(Fake)), direct);
    }
}
