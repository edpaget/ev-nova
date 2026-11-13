//! The planet files port: a textured planet's file read and decoded into
//! the [`Surface`] the planet shader draws, when the renderer first draws
//! it.
//!
//! A textured planet is checked at start-up from its file's header and
//! size (`nova_galaxy_media::check_texture`), and handed to the renderer
//! unread ([`LoadedPlanet::unread`](crate::LoadedPlanet::unread)) with a
//! [`TextureFile`]: everything a read needs, as plain data (the file's
//! path, the budget it is held to, and the names its errors give). The
//! renderer reads it through [`PlanetFiles::read`] in the first frame that
//! draws the planet, and again whenever it has to make the planet again
//! (after it released its textures). The galaxy is let go after
//! start-up, so the renderer can't ask the planet source again: this port
//! is what it keeps instead.
//!
//! A read can fail where the check could not see (a file that doesn't
//! decode), or because the file changed since (gone, now an LFS pointer,
//! another size): it fails as the source's [`PlanetError`] would.
//!
//! # Where an addition goes
//!
//! - Something more a read needs: a field of [`TextureFile`], filled
//!   where the planet is checked (`crate::galaxy_planets`).
//! - A new way a read fails: a [`PlanetError`] variant
//!   ([`crate::planet_source`]).
//!
//! `FakePlanetFiles` is the hand-written fake (with this crate's tests, or
//! the `recording` feature), and `check_contract` the contract every
//! reader meets: the fake runs it in this module's tests, and the galaxy
//! adapter (`crate::galaxy_planets::GalaxyPlanetFiles`) in its own.

use std::rc::Rc;

/// The budget limits a [`TextureFile`] is held to, so a caller that names
/// no media crate can build one.
pub use nova_galaxy_media::budget::Limits;

use crate::planet_source::{PlanetError, Surface};

/// A textured planet's file, as the renderer keeps it until it is read:
/// the stellar and asset it is for (named by its errors), the data-folder
/// path of its `.png`, and the budget limits it is held to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureFile {
    /// The stellar it draws.
    pub stellar: String,
    /// Its texture asset.
    pub asset: String,
    /// The data-folder path of its `.png`.
    pub file: String,
    /// The art budget's limits it is held to.
    pub limits: Limits,
}

/// Reads a textured planet's file when the renderer first draws it (see
/// the [module docs](self)).
pub trait PlanetFiles: std::fmt::Debug {
    /// The surface `texture`'s file draws with, read and decoded and
    /// checked again, or why it can't be.
    fn read(&self, texture: &TextureFile) -> Result<Surface, PlanetError>;
}

/// A borrowed reader is a reader.
impl<P: PlanetFiles + ?Sized> PlanetFiles for &P {
    fn read(&self, texture: &TextureFile) -> Result<Surface, PlanetError> {
        (**self).read(texture)
    }
}

/// A shared reader is a reader.
impl<P: PlanetFiles + ?Sized> PlanetFiles for Rc<P> {
    fn read(&self, texture: &TextureFile) -> Result<Surface, PlanetError> {
        (**self).read(texture)
    }
}

/// A reader with no files: every read is [`PlanetError::Missing`]. The
/// renderer reads through it until it is given one.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct NoFiles;

impl PlanetFiles for NoFiles {
    fn read(&self, texture: &TextureFile) -> Result<Surface, PlanetError> {
        Err(missing(texture))
    }
}

/// `texture`'s file isn't there.
fn missing(texture: &TextureFile) -> PlanetError {
    PlanetError::Missing {
        stellar: texture.stellar.clone(),
        asset: texture.asset.clone(),
        file: texture.file.clone(),
    }
}

#[cfg(any(test, feature = "recording"))]
pub use fake::{FakePlanetFiles, check_contract};

#[cfg(any(test, feature = "recording"))]
mod fake {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use nova_galaxy_media::planet::Image;

    use super::{PlanetError, PlanetFiles, Surface, TextureFile, missing};

    /// A hand-written [`PlanetFiles`]: the surfaces (or errors) it was
    /// given, by file, and how often each file was read. A file it wasn't
    /// given is [`PlanetError::Missing`].
    #[derive(Clone, Debug, Default)]
    pub struct FakePlanetFiles {
        files: BTreeMap<String, Result<Surface, PlanetError>>,
        reads: RefCell<BTreeMap<String, u32>>,
    }

    impl FakePlanetFiles {
        /// It reads `file` as `surface`.
        #[must_use]
        pub fn with(mut self, file: &str, surface: Surface) -> Self {
            self.files.insert(file.to_owned(), Ok(surface));
            self
        }

        /// It fails to read `file` with `error`.
        #[must_use]
        pub fn with_error(mut self, file: &str, error: PlanetError) -> Self {
            self.files.insert(file.to_owned(), Err(error));
            self
        }

        /// How often `file` was read.
        #[must_use]
        pub fn reads(&self, file: &str) -> u32 {
            self.reads.borrow().get(file).copied().unwrap_or(0)
        }
    }

    impl PlanetFiles for FakePlanetFiles {
        fn read(&self, texture: &TextureFile) -> Result<Surface, PlanetError> {
            *self
                .reads
                .borrow_mut()
                .entry(texture.file.clone())
                .or_default() += 1;
            self.files
                .get(&texture.file)
                .cloned()
                .unwrap_or_else(|| Err(missing(texture)))
        }
    }

    /// Panics unless `image`, named `what`, is a planet layer: twice as
    /// wide as tall, each side a power of two, four bytes a texel.
    fn check_layer(file: &str, what: &str, image: &Image) {
        assert!(
            image.width.is_power_of_two()
                && image.height.is_power_of_two()
                && image.width == 2 * image.height,
            "`{file}`'s {what} is twice as wide as tall, each side a power of two: {} by {}",
            image.width,
            image.height
        );
        assert_eq!(
            image.rgba.len(),
            (image.width * image.height * 4) as usize,
            "`{file}`'s {what} has four bytes a texel"
        );
    }

    /// Panics unless `files` meets the port's contract on `texture`, a
    /// file it reads, and `absent`, one that isn't there:
    ///
    /// - `texture` reads, to a surface twice as wide as tall with
    ///   power-of-two sides, four bytes a texel;
    /// - reading it again gives an equal answer;
    /// - `absent` is [`PlanetError::Missing`] naming its stellar, asset
    ///   and file.
    pub fn check_contract(files: &impl PlanetFiles, texture: &TextureFile, absent: &TextureFile) {
        let file = &texture.file;
        let surface = files
            .read(texture)
            .unwrap_or_else(|e| panic!("`{file}` reads: {e}"));
        check_layer(file, "surface", &surface.surface);
        assert_eq!(
            files.read(texture).as_ref(),
            Ok(&surface),
            "`{file}` reads the same twice"
        );
        assert_eq!(
            files.read(absent),
            Err(missing(absent)),
            "`{}` is missing, and named",
            absent.file
        );
    }
}

#[cfg(test)]
mod tests {
    use nova_galaxy_media::planet::Image;

    use super::*;

    fn texture(file: &str) -> TextureFile {
        TextureFile {
            stellar: "pale".into(),
            asset: "moon-surface".into(),
            file: file.into(),
            limits: Limits::default(),
        }
    }

    /// A bare 4 by 2 surface in grey `level`.
    fn surface(level: u8) -> Surface {
        Surface::from_texture(nova_galaxy_media::texture::Texture {
            file: "x.png".into(),
            width: 4,
            height: 2,
            disk_bytes: 0,
            rgba: vec![level; 32],
        })
    }

    #[test]
    fn the_fake_reads_what_it_holds_records_each_read_and_misses_the_rest() {
        let broken = PlanetError::Unreadable {
            stellar: "pale".into(),
            asset: "moon-surface".into(),
            why: "`b.png`: bad".into(),
        };
        let fake = FakePlanetFiles::default()
            .with("a.png", surface(9))
            .with_error("b.png", broken.clone());
        assert_eq!(fake.reads("a.png"), 0);
        assert_eq!(fake.read(&texture("a.png")), Ok(surface(9)));
        assert_eq!(fake.read(&texture("a.png")), Ok(surface(9)));
        assert_eq!(fake.read(&texture("b.png")), Err(broken));
        assert_eq!(
            fake.read(&texture("c.png")),
            Err(PlanetError::Missing {
                stellar: "pale".into(),
                asset: "moon-surface".into(),
                file: "c.png".into(),
            })
        );
        assert_eq!(
            (
                fake.reads("a.png"),
                fake.reads("b.png"),
                fake.reads("c.png")
            ),
            (2, 1, 1)
        );
        let borrowed: &dyn PlanetFiles = &&fake;
        assert_eq!(borrowed.read(&texture("a.png")), Ok(surface(9)));
        let shared = Rc::new(fake);
        assert_eq!(shared.read(&texture("a.png")), Ok(surface(9)));
        assert_eq!(shared.reads("a.png"), 4, "both read through it");
    }

    #[test]
    fn the_fake_meets_the_contract() {
        let fake = FakePlanetFiles::default().with("a.png", surface(9));
        check_contract(&fake, &texture("a.png"), &texture("gone.png"));
    }

    #[test]
    #[should_panic(expected = "twice as wide as tall")]
    fn the_contract_fails_a_square_surface() {
        let mut square = surface(9);
        square.surface = Image {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        };
        let fake = FakePlanetFiles::default().with("a.png", square);
        check_contract(&fake, &texture("a.png"), &texture("gone.png"));
    }

    #[test]
    #[should_panic(expected = "`gone.png` is missing, and named")]
    fn the_contract_fails_a_reader_with_no_absent_file() {
        let fake = FakePlanetFiles::default()
            .with("a.png", surface(9))
            .with("gone.png", surface(1));
        check_contract(&fake, &texture("a.png"), &texture("gone.png"));
    }

    #[test]
    fn no_files_misses_every_file() {
        assert_eq!(
            NoFiles.read(&texture("a.png")),
            Err(PlanetError::Missing {
                stellar: "pale".into(),
                asset: "moon-surface".into(),
                file: "a.png".into(),
            })
        );
    }
}
