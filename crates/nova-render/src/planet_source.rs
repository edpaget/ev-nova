//! The planet source port: the new game's planets and moons, by stellar,
//! as the [`Surface`]s the planet shader draws.
//!
//! [`PlanetSource::planet`] answers a [`PlanetRequest`] (a stellar's id:
//! a generated look belongs to a stellar, not to an asset) with a
//! [`LoadedPlanet`] or a [`PlanetError`]. A loaded planet draws with one
//! shape, a [`Surface`]: an equirectangular surface twice as wide as tall
//! with power-of-two sides, and its clouds, rings, atmosphere, day and
//! tilt. It holds either that surface already (a texture asset read,
//! `nova_galaxy_media::texture`, or a look somebody baked) or a generated
//! look not baked yet, with the sizes to bake it at
//! (`nova_galaxy_media::planet::bake`). A bake is the slow part of a
//! planet and can't fail, so a source may leave it to whoever draws the
//! planet: the renderer bakes one the first frame that draws it
//! (`crate::planets`). How far a planet reaches
//! ([`LoadedPlanet::reach`]) needs no bake. Models have their own port
//! ([`crate::model_source`]).
//!
//! # Where an addition goes
//!
//! - Something a caller must say about the planet it wants: a field of
//!   [`PlanetRequest`].
//! - A new way a planet can't be had: a [`PlanetError`] variant, its
//!   message naming the stellar (and the asset and file).
//! - Something more a planet draws with: a field of
//!   `nova_galaxy_media::planet::Surface`, baked or read there, and drawn
//!   by the renderer's planets (`crate::planet`).
//! - Another way a planet is handed over before it is ready to draw (a
//!   texture read lazily, say): a kind of [`LoadedPlanet`], which stays
//!   plain data, with its constructor, its turn in
//!   [`LoadedPlanet::into_surface`] and a [`LoadedPlanet::reach`] it can
//!   give without the work. `check_contract` checks what it makes. Its
//!   shape is private, so nothing outside this module changes with it.
//!
//! `FakePlanets` is the hand-written fake (with this crate's tests, or the
//! `recording` feature), and `check_contract` the contract every source
//! meets: the fake runs it in this module's tests, and the galaxy adapter
//! ([`crate::galaxy_planets`]) runs it on every planet of the sample.

use std::rc::Rc;

use nova_galaxy_format::model::Look;
pub use nova_galaxy_media::planet::Surface;
use nova_galaxy_media::planet::{Sizes, bake};

use crate::model_source::Overrun;
use crate::planet;

/// A planet wanted: the stellar (`stellars/<id>.md`) it draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanetRequest {
    /// The stellar's id.
    pub stellar: String,
}

impl PlanetRequest {
    /// The planet of the stellar `stellar`.
    #[must_use]
    pub fn new(stellar: &str) -> Self {
        PlanetRequest {
            stellar: stellar.to_owned(),
        }
    }
}

/// A planet, ready for the renderer
/// ([`Renderer::add_planet`](crate::Renderer::add_planet)): either what it
/// draws with ([`LoadedPlanet::surface`]) or a generated look not baked
/// yet ([`LoadedPlanet::unbaked`]), which whoever draws it bakes
/// ([`into_surface`](Self::into_surface)). How far it reaches is known
/// either way ([`reach`](Self::reach)). Its shape is private, so build one
/// with its constructors.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedPlanet {
    kind: Kind,
}

/// What a [`LoadedPlanet`] holds.
#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// What it draws with: a texture read, or a look baked already.
    Surface(Surface),
    /// A generated look, and the sizes to bake it at.
    Unbaked { look: Look, sizes: Sizes },
}

impl LoadedPlanet {
    /// A planet that draws with `surface`.
    #[must_use]
    pub fn surface(surface: Surface) -> Self {
        LoadedPlanet {
            kind: Kind::Surface(surface),
        }
    }

    /// A planet that draws `look` baked at `sizes`, not baked yet: a bake
    /// is the slow part of loading a planet, so it is left to the first
    /// frame that draws it.
    #[must_use]
    pub fn unbaked(look: Look, sizes: Sizes) -> Self {
        LoadedPlanet {
            kind: Kind::Unbaked { look, sizes },
        }
    }

    /// What it draws with, when it holds that already; `None` for a look
    /// not baked yet.
    #[must_use]
    pub fn baked(&self) -> Option<&Surface> {
        match &self.kind {
            Kind::Surface(surface) => Some(surface),
            Kind::Unbaked { .. } => None,
        }
    }

    /// How far it reaches, in its radii ([`planet::reach`]): from its
    /// surface, or, unbaked, from its look's atmosphere and rings, which
    /// is what its bake's surface reaches.
    #[must_use]
    pub fn reach(&self) -> f32 {
        match &self.kind {
            Kind::Surface(surface) => planet::surface_reach(surface),
            #[allow(clippy::cast_possible_truncation, reason = "a shader's sizes")]
            Kind::Unbaked { look, .. } => planet::reach(
                look.atmosphere.as_ref().map(|a| a.thickness.get() as f32),
                look.rings.as_ref().map(|r| r.outer as f32),
            ),
        }
    }

    /// What it draws with: its surface, or its look baked at its sizes
    /// (`nova_galaxy_media::planet::bake`, which can't fail).
    #[must_use]
    pub fn into_surface(self) -> Surface {
        match self.kind {
            Kind::Surface(surface) => surface,
            Kind::Unbaked { look, sizes } => bake(&look, sizes),
        }
    }

    /// What it draws with, baking its look in place the first time, so it
    /// holds its surface from then on ([`baked`](Self::baked)) and bakes
    /// no more.
    pub fn bake(&mut self) -> &Surface {
        if let Kind::Unbaked { look, sizes } = &self.kind {
            self.kind = Kind::Surface(bake(look, *sizes));
        }
        match &self.kind {
            Kind::Surface(surface) => surface,
            Kind::Unbaked { .. } => unreachable!("baked just above"),
        }
    }
}

/// Why a stellar's planet can't be had. The caller keeps drawing its
/// stand-in.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanetError {
    /// The galaxy has no such stellar.
    #[error("no stellar `{0}`")]
    NoStellar(String),
    /// The stellar is a station or an outpost, or has no visual: there is
    /// no planet to draw.
    #[error("stellar `{0}` is no planet or moon with a visual")]
    NotAPlanet(String),
    /// Its visual names an asset the manifest doesn't have.
    #[error("stellar `{stellar}`'s visual `{asset}` isn't in the manifest")]
    NoAsset {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
    },
    /// Its visual names a model, not a texture.
    #[error("stellar `{stellar}`'s visual `{asset}` is a model, not a texture")]
    NotATexture {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
    },
    /// Its texture asset names no `.png`: a placeholder, say.
    #[error("stellar `{stellar}`'s texture `{asset}` has no file")]
    NoFile {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
    },
    /// Its texture is a Git LFS pointer the checkout hasn't fetched.
    #[error("stellar `{stellar}`'s texture `{asset}` is a Git LFS pointer, not fetched")]
    LfsPointer {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
    },
    /// Its texture's file isn't there.
    #[error("stellar `{stellar}`'s texture `{asset}` file `{file}` is missing")]
    Missing {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
        /// The missing file.
        file: String,
    },
    /// Its texture's file doesn't read as a PNG.
    #[error("stellar `{stellar}`'s texture `{asset}` doesn't read: {why}")]
    Unreadable {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
        /// Why, naming the file.
        why: String,
    },
    /// Its texture isn't twice as wide as tall with power-of-two sides.
    #[error(
        "stellar `{stellar}`'s texture `{asset}` file `{file}` is {width} by {height}: a \
         planet's texture is twice as wide as tall, each side a power of two"
    )]
    Shape {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
        /// The file.
        file: String,
        /// Its width in pixels.
        width: u32,
        /// Its height in pixels.
        height: u32,
    },
    /// Its texture is wider or taller than the GPU's largest texture
    /// (`MAX_TEXTURE_SIDE`), so it can't be uploaded.
    #[error(
        "stellar `{stellar}`'s texture `{asset}` file `{file}` is {side} pixels on its \
         longest side, over the {most} pixel limit of the GPU's textures"
    )]
    TooLarge {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
        /// The file.
        file: String,
        /// Its longest side in pixels.
        side: u32,
        /// The most a GPU texture may be.
        most: u32,
    },
    /// Its texture is over its art budget.
    #[error(
        "stellar `{stellar}`'s texture `{asset}` is over its art budget: {}",
        list(over)
    )]
    OverBudget {
        /// The stellar.
        stellar: String,
        /// The asset.
        asset: String,
        /// Every limit it breaks.
        over: Vec<Overrun>,
    },
}

/// `over`, joined with `; `.
fn list(over: &[Overrun]) -> String {
    over.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// The new game's planets, in the renderer's terms.
pub trait PlanetSource {
    /// The planet of `request.stellar` (see the [module docs](self)).
    fn planet(&self, request: &PlanetRequest) -> Result<LoadedPlanet, PlanetError>;
}

/// A borrowed source is a source.
impl<S: PlanetSource + ?Sized> PlanetSource for &S {
    fn planet(&self, request: &PlanetRequest) -> Result<LoadedPlanet, PlanetError> {
        (**self).planet(request)
    }
}

/// A shared source is a source.
impl<S: PlanetSource + ?Sized> PlanetSource for Rc<S> {
    fn planet(&self, request: &PlanetRequest) -> Result<LoadedPlanet, PlanetError> {
        (**self).planet(request)
    }
}

#[cfg(any(test, feature = "recording"))]
pub use fake::{FakePlanets, check_contract};

#[cfg(any(test, feature = "recording"))]
mod fake {
    use std::collections::BTreeMap;

    use nova_galaxy_media::planet::Image;

    use super::{LoadedPlanet, PlanetError, PlanetRequest, PlanetSource};

    /// A hand-written [`PlanetSource`]: the planets it was given, by
    /// stellar. A stellar it wasn't given is [`PlanetError::NoStellar`].
    #[derive(Clone, Debug, Default)]
    pub struct FakePlanets {
        planets: BTreeMap<String, Result<LoadedPlanet, PlanetError>>,
    }

    impl FakePlanets {
        /// It answers `stellar` with `planet`.
        #[must_use]
        pub fn with(mut self, stellar: &str, planet: LoadedPlanet) -> Self {
            self.planets.insert(stellar.to_owned(), Ok(planet));
            self
        }

        /// It answers `stellar` with `error`.
        #[must_use]
        pub fn with_error(mut self, stellar: &str, error: PlanetError) -> Self {
            self.planets.insert(stellar.to_owned(), Err(error));
            self
        }
    }

    impl PlanetSource for FakePlanets {
        fn planet(&self, request: &PlanetRequest) -> Result<LoadedPlanet, PlanetError> {
            self.planets
                .get(&request.stellar)
                .ok_or_else(|| PlanetError::NoStellar(request.stellar.clone()))?
                .clone()
        }
    }

    /// Panics unless `image`, named `what`, is a planet layer: twice as
    /// wide as tall, each side a power of two, four bytes a texel.
    fn check_layer(stellar: &str, what: &str, image: &Image) {
        assert!(
            image.width.is_power_of_two()
                && image.height.is_power_of_two()
                && image.width == 2 * image.height,
            "`{stellar}`'s {what} is twice as wide as tall, each side a power of two: {} by {}",
            image.width,
            image.height
        );
        assert_eq!(
            image.rgba.len(),
            (image.width * image.height * 4) as usize,
            "`{stellar}`'s {what} has four bytes a texel"
        );
    }

    /// Panics unless `source` meets the port's contract on `request`, a
    /// stellar it holds a planet for, and `absent`, one it doesn't:
    ///
    /// - `request` loads;
    /// - what it draws with (its surface, or its look baked, which this
    ///   check does) reaches as far as the answer said before any bake;
    /// - its surface, and its clouds when it has them, are twice as wide
    ///   as tall with power-of-two sides, four bytes a texel;
    /// - its rings, when it has them, start outside the globe, end beyond
    ///   where they start, and have a profile of at least two texels;
    /// - its atmosphere, when it has one, is a finite thickness from 0
    ///   to 1;
    /// - its day is a positive, finite number of seconds, and its tilt
    ///   finite;
    /// - asking again gives an equal answer;
    /// - `absent` is [`PlanetError::NoStellar`].
    pub fn check_contract(source: &impl PlanetSource, request: &PlanetRequest, absent: &str) {
        let stellar = &request.stellar;
        let planet = source
            .planet(request)
            .unwrap_or_else(|e| panic!("`{stellar}` loads: {e}"));
        let surface = &planet.clone().into_surface();
        let reach = crate::planet::surface_reach(surface);
        // The same sizes through the same arithmetic: the same bits.
        assert_eq!(
            planet.reach().to_bits(),
            reach.to_bits(),
            "`{stellar}` reaches {} before it is baked and {reach} after",
            planet.reach()
        );
        check_layer(stellar, "surface", &surface.surface);
        if let Some(clouds) = &surface.clouds {
            check_layer(stellar, "cloud layer", clouds);
        }
        if let Some(rings) = &surface.rings {
            assert!(
                rings.inner > 1.0 && rings.outer > rings.inner && rings.outer.is_finite(),
                "`{stellar}`'s rings start outside the globe and end beyond: {} to {}",
                rings.inner,
                rings.outer
            );
            assert!(
                rings.profile.len() >= 2,
                "`{stellar}`'s ring profile has two ends"
            );
        }
        if let Some(glow) = &surface.atmosphere {
            assert!(
                (0.0..=1.0).contains(&glow.thickness),
                "`{stellar}`'s atmosphere is 0 to 1 radii thick: {}",
                glow.thickness
            );
        }
        assert!(
            surface.day.is_finite() && surface.day > 0.0,
            "`{stellar}`'s day is a positive number of seconds: {}",
            surface.day
        );
        assert!(surface.tilt.is_finite(), "`{stellar}`'s tilt is finite");
        assert_eq!(
            source.planet(request).as_ref(),
            Ok(&planet),
            "`{stellar}` gives the same answer twice"
        );
        assert!(
            matches!(
                source.planet(&PlanetRequest::new(absent)),
                Err(PlanetError::NoStellar(_))
            ),
            "`{absent}` is no stellar"
        );
    }
}

#[cfg(test)]
mod tests {
    use nova_galaxy_media::planet::{Glow, Image, RingBand};

    use super::*;
    use crate::model_source::Measure;

    /// A 4 by 2 surface with a ring and an atmosphere.
    fn surface() -> Surface {
        Surface {
            surface: Image {
                width: 4,
                height: 2,
                rgba: vec![9; 32],
            },
            clouds: None,
            rings: Some(RingBand {
                inner: 1.5,
                outer: 2.0,
                color: [1, 2, 3],
                profile: vec![0, 200, 0],
            }),
            atmosphere: Some(Glow {
                color: [4, 5, 6],
                thickness: 0.1,
            }),
            day: 300.0,
            tilt: 0.1,
        }
    }

    /// A planet that draws with [`surface`].
    fn planet() -> LoadedPlanet {
        LoadedPlanet::surface(surface())
    }

    /// Looks baked small, so the tests stay fast.
    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    /// A garden world's look with `extra`, lines of its `generated` block
    /// indented four spaces.
    fn look(extra: &str) -> Look {
        use nova_galaxy_format::model::{Stellar, Visual};
        let text = format!(
            "---\nname: Verdance\nkind: planet\nsystem: alpha\nposition: [0, 0]\nvisual:\n\
             \x20 generated:\n    type: garden\n    palette: [\"#3a7a3a\", \"#2a4a8a\"]\n\
             \x20   clouds: 0.5\n{extra}    seed: 77\n---\n"
        );
        let stellar = nova_galaxy_format::frontmatter::parse::<Stellar>(&text).expect("a stellar");
        match stellar.0.visual {
            Some(Visual::Generated(look)) => look,
            other => panic!("a generated look: {other:?}"),
        }
    }

    const RINGS: &str = "    rings: {inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}\n";
    const ATMOSPHERE: &str = "    atmosphere: {color: \"#f0e0c0\", thickness: 0.25}\n";

    #[test]
    fn a_planet_with_a_surface_holds_it_and_reaches_as_far_as_it_does() {
        let planet = planet();
        assert_eq!(planet.baked(), Some(&surface()));
        assert!((planet.reach() - 2.0).abs() < f32::EPSILON, "its rings'");
        assert_eq!(planet.into_surface(), surface());
    }

    #[test]
    fn an_unbaked_planet_bakes_to_its_look_at_its_sizes() {
        let look = look(&format!("{RINGS}{ATMOSPHERE}"));
        let planet = LoadedPlanet::unbaked(look.clone(), SMALL);
        assert_eq!(planet.baked(), None);
        assert_eq!(planet.clone().into_surface(), bake(&look, SMALL));
        let smaller = Sizes {
            surface: 16,
            ..SMALL
        };
        assert_ne!(planet, LoadedPlanet::unbaked(look.clone(), smaller));
        assert_eq!(
            LoadedPlanet::unbaked(look.clone(), smaller).into_surface(),
            bake(&look, smaller)
        );
    }

    #[test]
    fn baking_in_place_gives_the_bake_and_holds_it_from_then_on() {
        let look = look(&format!("{RINGS}{ATMOSPHERE}"));
        let mut planet = LoadedPlanet::unbaked(look.clone(), SMALL);
        let baked = bake(&look, SMALL);
        assert_eq!(planet.bake(), &baked);
        assert_eq!(planet.baked(), Some(&baked));
        assert_eq!(planet, LoadedPlanet::surface(baked.clone()));
        assert_eq!(planet.bake(), &baked, "and again, the same");
        // One holding its surface gives it as it is.
        let mut held = self::planet();
        assert_eq!(held.bake(), &surface());
        assert_eq!(held, self::planet());
    }

    #[test]
    fn an_unbaked_planet_reaches_as_far_as_its_bake_does() {
        for (extra, want) in [
            (format!("{RINGS}{ATMOSPHERE}"), 2.3),
            (RINGS.to_owned(), 2.3),
            (ATMOSPHERE.to_owned(), 1.25),
            (String::new(), 1.0),
        ] {
            let look = look(&extra);
            let planet = LoadedPlanet::unbaked(look.clone(), SMALL);
            let baked = crate::planet::surface_reach(&bake(&look, SMALL));
            assert!((planet.reach() - baked).abs() < f32::EPSILON, "{extra}");
            assert!((planet.reach() - want).abs() < 1e-6, "{extra}");
        }
    }

    #[test]
    fn the_fake_holds_an_unbaked_planet_and_it_meets_the_contract() {
        let unbaked = LoadedPlanet::unbaked(look(&format!("{RINGS}{ATMOSPHERE}")), SMALL);
        let fake = FakePlanets::default().with("verdance", unbaked.clone());
        assert_eq!(fake.planet(&PlanetRequest::new("verdance")), Ok(unbaked));
        check_contract(&fake, &PlanetRequest::new("verdance"), "nothing");
    }

    #[test]
    fn the_fake_answers_what_it_holds_and_no_stellar_otherwise() {
        let pointer = PlanetError::LfsPointer {
            stellar: "dim".into(),
            asset: "dim-surface".into(),
        };
        let fake = FakePlanets::default()
            .with("kestrel-reach", planet())
            .with_error("dim", pointer.clone());
        assert_eq!(
            fake.planet(&PlanetRequest::new("kestrel-reach")),
            Ok(planet())
        );
        assert_eq!(fake.planet(&PlanetRequest::new("dim")), Err(pointer));
        assert_eq!(
            fake.planet(&PlanetRequest::new("nothing")),
            Err(PlanetError::NoStellar("nothing".into()))
        );
        let request = PlanetRequest::new("kestrel-reach");
        let borrowed: &dyn PlanetSource = &&fake;
        assert_eq!(borrowed.planet(&request), fake.planet(&request));
        assert_eq!(
            Rc::new(fake.clone()).planet(&request),
            fake.planet(&request)
        );
    }

    #[test]
    fn the_fake_meets_the_contract() {
        let fake = FakePlanets::default().with("kestrel-reach", planet());
        check_contract(&fake, &PlanetRequest::new("kestrel-reach"), "nothing");
    }

    #[test]
    #[should_panic(expected = "twice as wide as tall")]
    fn the_contract_fails_a_square_surface() {
        let mut square = surface();
        square.surface = Image {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        };
        let fake = FakePlanets::default().with("p", LoadedPlanet::surface(square));
        check_contract(&fake, &PlanetRequest::new("p"), "nothing");
    }

    #[test]
    #[should_panic(expected = "cloud layer is twice as wide as tall")]
    fn the_contract_fails_clouds_that_aren_t_a_power_of_two() {
        let mut odd = surface();
        odd.clouds = Some(Image {
            width: 6,
            height: 3,
            rgba: vec![0; 72],
        });
        let fake = FakePlanets::default().with("p", LoadedPlanet::surface(odd));
        check_contract(&fake, &PlanetRequest::new("p"), "nothing");
    }

    #[test]
    #[should_panic(expected = "rings start outside the globe")]
    fn the_contract_fails_rings_inside_the_globe() {
        let mut low = surface();
        if let Some(rings) = &mut low.rings {
            rings.inner = 0.5;
        }
        let fake = FakePlanets::default().with("p", LoadedPlanet::surface(low));
        check_contract(&fake, &PlanetRequest::new("p"), "nothing");
    }

    #[test]
    #[should_panic(expected = "day is a positive number")]
    fn the_contract_fails_a_day_of_no_length() {
        let mut still = surface();
        still.day = 0.0;
        let fake = FakePlanets::default().with("p", LoadedPlanet::surface(still));
        check_contract(&fake, &PlanetRequest::new("p"), "nothing");
    }

    /// A source that answers every stellar, so none is absent.
    struct Everything;

    impl PlanetSource for Everything {
        fn planet(&self, _: &PlanetRequest) -> Result<LoadedPlanet, PlanetError> {
            Ok(planet())
        }
    }

    #[test]
    #[should_panic(expected = "`nothing` is no stellar")]
    fn the_contract_fails_a_source_with_no_absent_stellar() {
        check_contract(&Everything, &PlanetRequest::new("p"), "nothing");
    }

    #[test]
    fn planet_error_messages_name_the_stellar_the_asset_and_the_budget_file() {
        let over = PlanetError::OverBudget {
            stellar: "marrow-fields".into(),
            asset: "marrow-fields-surface".into(),
            over: vec![Overrun {
                measure: Measure::Side,
                used: 8192,
                most: 4096,
                budget: "budgets/default.md".into(),
            }],
        };
        assert_eq!(
            over.to_string(),
            "stellar `marrow-fields`'s texture `marrow-fields-surface` is over its art budget: \
             is 8192 pixels on its longest side, over the 4096 pixel budget in \
             `budgets/default.md`"
        );
        let shape = PlanetError::Shape {
            stellar: "p".into(),
            asset: "a".into(),
            file: "media/export/x/a.png".into(),
            width: 96,
            height: 48,
        };
        assert_eq!(
            shape.to_string(),
            "stellar `p`'s texture `a` file `media/export/x/a.png` is 96 by 48: a planet's \
             texture is twice as wide as tall, each side a power of two"
        );
        let stellar = || "p".to_owned();
        let asset = || "a".to_owned();
        for error in [
            PlanetError::NoStellar(stellar()),
            PlanetError::NotAPlanet(stellar()),
            PlanetError::NoAsset {
                stellar: stellar(),
                asset: asset(),
            },
            PlanetError::NotATexture {
                stellar: stellar(),
                asset: asset(),
            },
            PlanetError::NoFile {
                stellar: stellar(),
                asset: asset(),
            },
            PlanetError::LfsPointer {
                stellar: stellar(),
                asset: asset(),
            },
            PlanetError::Missing {
                stellar: stellar(),
                asset: asset(),
                file: "f.png".into(),
            },
            PlanetError::Unreadable {
                stellar: stellar(),
                asset: asset(),
                why: "`f.png`: bad".into(),
            },
            PlanetError::TooLarge {
                stellar: stellar(),
                asset: asset(),
                file: "f.png".into(),
                side: 16_384,
                most: 8_192,
            },
        ] {
            let message = error.to_string();
            assert!(message.contains("`p`"), "{message}");
            if !matches!(
                error,
                PlanetError::NoStellar(_) | PlanetError::NotAPlanet(_)
            ) {
                assert!(message.contains("`a`"), "{message}");
            }
        }
    }
}
