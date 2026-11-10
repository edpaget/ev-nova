//! [`PlanetSource`] for the new game's data: [`GalaxyPlanets`] bakes a
//! stellar's generated look, or reads its texture asset's `.png` through
//! the format's [`Source`] port, needing no GPU.
//!
//! It only chains decisions made elsewhere: how a look bakes
//! (`nova_galaxy_media::planet::bake`), how a texture is read and checked
//! (`nova_galaxy_media::texture::read_texture`), which budget covers it
//! (`nova_galaxy_media::budget::limits`), and how a texture becomes a
//! planet (`nova_galaxy_media::planet::Surface::from_texture`). Those are
//! pinned by their own tests; this module's tests check what it does with
//! them. It caches nothing: the caller decides when to load and which
//! [`PlanetKey`](nova_view::PlanetKey) a planet takes
//! ([`Renderer::add_planet`](crate::Renderer::add_planet)).

use nova_galaxy_format::asset::AssetKind;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::Galaxy;
use nova_galaxy_format::model::{StellarKind, Visual};
use nova_galaxy_format::source::Source;
use nova_galaxy_media::budget::limits;
use nova_galaxy_media::planet::{Sizes, Surface, bake};
use nova_galaxy_media::{NoTexture, read_texture};

use crate::galaxy_models::overrun;
use crate::gpu::MAX_TEXTURE_SIDE;
use crate::planet_source::{LoadedPlanet, PlanetError, PlanetRequest, PlanetSource};

/// The new game's planets: the stellars, asset manifest and budgets of
/// `galaxy`, texture files read through `source`, looks baked at `sizes`.
#[derive(Clone, Copy, Debug)]
pub struct GalaxyPlanets<'a, S: Source> {
    galaxy: &'a Galaxy,
    source: &'a S,
    sizes: Sizes,
}

impl<'a, S: Source> GalaxyPlanets<'a, S> {
    /// The planets `galaxy` names, textures read through `source`, looks
    /// baked at the engine's sizes ([`Sizes::ENGINE`]).
    #[must_use]
    pub fn new(galaxy: &'a Galaxy, source: &'a S) -> Self {
        GalaxyPlanets {
            galaxy,
            source,
            sizes: Sizes::ENGINE,
        }
    }

    /// The same planets with looks baked at `sizes`: smaller, for tests.
    #[must_use]
    pub fn baked_at(self, sizes: Sizes) -> Self {
        GalaxyPlanets { sizes, ..self }
    }
}

impl<S: Source> PlanetSource for GalaxyPlanets<'_, S> {
    fn planet(&self, request: &PlanetRequest) -> Result<LoadedPlanet, PlanetError> {
        let name = &request.stellar;
        let no_stellar = || PlanetError::NoStellar(name.clone());
        let id = Id::try_from(name.clone()).map_err(|_| no_stellar())?;
        let stellar = &self.galaxy.stellars.get(&id).ok_or_else(no_stellar)?.data;
        if matches!(stellar.kind, StellarKind::Station | StellarKind::Outpost) {
            return Err(PlanetError::NotAPlanet(name.clone()));
        }
        let surface = match &stellar.visual {
            None => return Err(PlanetError::NotAPlanet(name.clone())),
            Some(Visual::Generated(look)) => bake(look, self.sizes),
            Some(Visual::Asset(asset)) => self.texture(name, asset)?,
        };
        Ok(LoadedPlanet { surface })
    }
}

impl<S: Source> GalaxyPlanets<'_, S> {
    /// The planet of `stellar` drawn from its texture asset `asset`.
    fn texture(&self, stellar: &str, asset: &Id) -> Result<Surface, PlanetError> {
        let (stellar, name) = (stellar.to_owned(), asset.to_string());
        let Some(doc) = self.galaxy.assets.get(asset) else {
            return Err(PlanetError::NoAsset {
                stellar,
                asset: name,
            });
        };
        if doc.data.kind != AssetKind::Texture {
            return Err(PlanetError::NotATexture {
                stellar,
                asset: name,
            });
        }
        let limits = limits(self.galaxy, asset);
        read_texture(self.source, &doc.data, &limits, MAX_TEXTURE_SIDE)
            .map(Surface::from_texture)
            .map_err(|why| no_texture(stellar, name, why))
    }
}

/// Why `stellar`'s texture `asset` can't be had, as the port says it.
fn no_texture(stellar: String, asset: String, why: NoTexture) -> PlanetError {
    match why {
        NoTexture::None => PlanetError::NoFile { stellar, asset },
        NoTexture::Pointer => PlanetError::LfsPointer { stellar, asset },
        NoTexture::Missing(file) => PlanetError::Missing {
            stellar,
            asset,
            file,
        },
        NoTexture::Unreadable(why) => PlanetError::Unreadable {
            stellar,
            asset,
            why,
        },
        NoTexture::Shape {
            file,
            width,
            height,
        } => PlanetError::Shape {
            stellar,
            asset,
            file,
            width,
            height,
        },
        NoTexture::TooLarge { file, side, most } => PlanetError::TooLarge {
            stellar,
            asset,
            file,
            side,
            most,
        },
        NoTexture::OverBudget(over) => PlanetError::OverBudget {
            stellar,
            asset,
            over: over.iter().map(overrun).collect(),
        },
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::fixture::{CC0, galaxy};
    use nova_galaxy_format::load::load;
    use nova_galaxy_format::source::MemSource;
    use nova_galaxy_media::fixture::png;
    use nova_galaxy_media::planet::TEXTURE_DAY;

    use super::*;
    use crate::model_source::{Measure, Overrun};
    use crate::planet_source::check_contract;

    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    /// Kestrel Reach as the sample writes it, in `alpha`.
    const KESTREL_REACH: &str = "---\nname: Kestrel Reach\nkind: planet\nsystem: alpha\n\
        position: [300, -200]\nvisual:\n  generated:\n    type: gas-giant\n\
        \x20   palette: [\"#c9a36b\", \"#8a5a3c\", \"#e8d8b0\"]\n    clouds: 0.6\n\
        \x20   rings: {inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}\n\
        \x20   atmosphere: {color: \"#f0e0c0\", thickness: 0.04}\n    seed: 4417\n---\n";

    /// A 16 by 8 test pattern: each texel's red its column, green its row.
    fn pattern() -> Vec<u8> {
        (0..8_u8)
            .flat_map(|y| (0..16_u8).flat_map(move |x| [x * 16, y * 32, 77, 255]))
            .collect()
    }

    /// The format's test galaxy (its planet `alpha-prime` on texture
    /// `surface`, at `media/export/pack/surface.png`; the `alpha-yard`
    /// station; a default budget with `texture: {kib: 4096, side: 4096}`),
    /// with Kestrel Reach, a bare moon, and `surface.png` the pattern.
    fn sample() -> MemSource {
        galaxy()
            .with("stellars/kestrel-reach.md", KESTREL_REACH)
            .with(
                "stellars/bare-moon.md",
                "---\nname: Bare Moon\nkind: moon\nsystem: alpha\nposition: [9, 9]\n---\n",
            )
            .with_bytes("media/export/pack/surface.png", &png(16, 8, &pattern()))
    }

    fn ask(source: &MemSource, stellar: &str) -> Result<LoadedPlanet, PlanetError> {
        let (galaxy, _) = load(source);
        GalaxyPlanets::new(&galaxy, source)
            .baked_at(SMALL)
            .planet(&PlanetRequest::new(stellar))
    }

    #[test]
    fn a_generated_stellar_is_its_look_baked() {
        let source = sample();
        let (galaxy, problems) = load(&source);
        assert_eq!(problems, [], "the sample loads cleanly");
        let Some(Visual::Generated(look)) = &galaxy.stellars[&id("kestrel-reach")].data.visual
        else {
            panic!("generated");
        };
        assert_eq!(
            ask(&source, "kestrel-reach"),
            Ok(LoadedPlanet {
                surface: bake(look, SMALL),
            })
        );
        // The engine's sizes, unless told otherwise.
        let engine = GalaxyPlanets::new(&galaxy, &source);
        assert_eq!(engine.sizes, Sizes::ENGINE);
    }

    fn id(s: &str) -> Id {
        Id::try_from(s.to_owned()).unwrap()
    }

    #[test]
    fn a_texture_planet_is_its_texture_with_no_extras() {
        let planet = ask(&sample(), "alpha-prime").unwrap().surface;
        let opaque: Vec<u8> = pattern()
            .chunks(4)
            .flat_map(|t| [t[0], t[1], t[2], 0])
            .collect();
        assert_eq!((planet.surface.width, planet.surface.height), (16, 8));
        assert_eq!(planet.surface.rgba, opaque, "glowing nowhere");
        assert_eq!(planet.clouds, None);
        assert_eq!(planet.rings, None);
        assert_eq!(planet.atmosphere, None);
        assert_eq!(planet.day, TEXTURE_DAY);
    }

    #[test]
    fn a_station_an_outpost_or_a_stellar_with_no_visual_is_no_planet() {
        let source = sample().with(
            "stellars/camp.md",
            "---\nname: Camp\nkind: outpost\nsystem: alpha\nposition: [1, 1]\n\
             visual: {asset: surface}\n---\n",
        );
        for stellar in ["alpha-yard", "camp", "bare-moon"] {
            assert_eq!(
                ask(&source, stellar),
                Err(PlanetError::NotAPlanet(stellar.into()))
            );
        }
        for stellar in ["nothing", "Not An Id"] {
            assert_eq!(
                ask(&source, stellar),
                Err(PlanetError::NoStellar(stellar.into()))
            );
        }
    }

    #[test]
    fn a_visual_not_in_the_manifest_or_a_model_is_reported() {
        let source = sample()
            .with(
                "stellars/lost.md",
                "---\nname: Lost\nkind: planet\nsystem: alpha\nposition: [1, 1]\n\
                 visual: {asset: nowhere}\n---\n",
            )
            .with(
                "stellars/odd.md",
                "---\nname: Odd\nkind: moon\nsystem: alpha\nposition: [2, 2]\n\
                 visual: {asset: hull}\n---\n",
            );
        assert_eq!(
            ask(&source, "lost"),
            Err(PlanetError::NoAsset {
                stellar: "lost".into(),
                asset: "nowhere".into(),
            })
        );
        assert_eq!(
            ask(&source, "odd"),
            Err(PlanetError::NotATexture {
                stellar: "odd".into(),
                asset: "hull".into(),
            })
        );
    }

    #[test]
    fn each_way_a_texture_fails_reaches_the_port() {
        let pointer = sample().with_bytes(
            "media/export/pack/surface.png",
            b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n",
        );
        let named = || ("alpha-prime".to_owned(), "surface".to_owned());
        let (stellar, asset) = named();
        assert_eq!(
            ask(&pointer, "alpha-prime"),
            Err(PlanetError::LfsPointer { stellar, asset })
        );
        let (stellar, asset) = named();
        assert_eq!(
            ask(
                &galaxy().with("stellars/kestrel-reach.md", KESTREL_REACH),
                "alpha-prime"
            ),
            Err(PlanetError::Missing {
                stellar,
                asset,
                file: "media/export/pack/surface.png".into(),
            })
        );
        let broken = sample().with_bytes("media/export/pack/surface.png", b"not a png");
        assert!(matches!(
            ask(&broken, "alpha-prime"),
            Err(PlanetError::Unreadable { ref why, .. }) if why.contains("surface.png")
        ));
        let square = sample().with_bytes("media/export/pack/surface.png", &png(8, 8, &[0; 256]));
        let (stellar, asset) = named();
        assert_eq!(
            ask(&square, "alpha-prime"),
            Err(PlanetError::Shape {
                stellar,
                asset,
                file: "media/export/pack/surface.png".into(),
                width: 8,
                height: 8,
            })
        );
        let placeholder = sample().with(
            "assets/surface.md",
            "---\nkind: texture\nplaceholder: true\n---\n",
        );
        let (stellar, asset) = named();
        assert_eq!(
            ask(&placeholder, "alpha-prime"),
            Err(PlanetError::NoFile { stellar, asset })
        );
    }

    #[test]
    fn a_texture_over_the_default_budget_is_over_budget_naming_its_file() {
        let tight = sample().with(
            "budgets/default.md",
            "---\nmodel: {triangles: 20000, kib: 4096}\ntexture: {kib: 4096, side: 8}\n---\n",
        );
        let (stellar, asset) = ("alpha-prime".to_owned(), "surface".to_owned());
        assert_eq!(
            ask(&tight, "alpha-prime"),
            Err(PlanetError::OverBudget {
                stellar,
                asset,
                over: vec![Overrun {
                    measure: Measure::Side,
                    used: 16,
                    most: 8,
                    budget: "budgets/default.md".into(),
                }],
            })
        );
        let small = sample().with(
            "budgets/default.md",
            "---\ntexture: {kib: 0, side: 4096}\n---\n",
        );
        let Err(PlanetError::OverBudget { over, .. }) = ask(&small, "alpha-prime") else {
            panic!("over its kib");
        };
        assert_eq!(over[0].measure, Measure::Kib);
        assert!(ask(&sample().without("budgets/default.md"), "alpha-prime").is_ok());
    }

    #[test]
    fn a_texture_wider_than_the_gpu_draws_is_refused_not_uploaded() {
        // A surface's shape, no budget to stop it, and only a header: the
        // GPU's limit refuses it before its pixels would decode.
        let mut header = png(2, 1, &[0; 8])[..33].to_vec();
        let wide = MAX_TEXTURE_SIDE * 2;
        header[16..20].copy_from_slice(&wide.to_be_bytes());
        header[20..24].copy_from_slice(&MAX_TEXTURE_SIDE.to_be_bytes());
        let huge = sample()
            .without("budgets/default.md")
            .with_bytes("media/export/pack/surface.png", &header);
        assert_eq!(
            ask(&huge, "alpha-prime"),
            Err(PlanetError::TooLarge {
                stellar: "alpha-prime".into(),
                asset: "surface".into(),
                file: "media/export/pack/surface.png".into(),
                side: wide,
                most: MAX_TEXTURE_SIDE,
            })
        );
    }

    #[test]
    fn the_engine_bakes_no_image_wider_than_the_gpu_draws() {
        let Sizes {
            surface,
            clouds,
            ring,
        } = Sizes::ENGINE;
        assert!(surface.max(clouds).max(ring) <= MAX_TEXTURE_SIDE);
    }

    #[test]
    fn every_planet_of_the_sample_meets_the_contract() {
        let source = sample();
        let (galaxy, _) = load(&source);
        let planets = GalaxyPlanets::new(&galaxy, &source).baked_at(SMALL);
        for stellar in ["kestrel-reach", "alpha-prime"] {
            check_contract(&planets, &PlanetRequest::new(stellar), "nothing");
        }
    }

    #[test]
    fn a_texture_manifest_entry_with_its_licence_loads() {
        // The manifest entry as an artist writes one, beside the fixture's.
        let source = sample()
            .with(
                "assets/moon-surface.md",
                &format!(
                    "---\nkind: texture\nplaceholder: false\n\
                     files: [media/export/pack/moon.png]\n{CC0}---\n"
                ),
            )
            .with(
                "stellars/pale.md",
                "---\nname: Pale\nkind: moon\nsystem: alpha\nposition: [3, 3]\n\
                 visual: {asset: moon-surface}\n---\n",
            )
            .with_bytes("media/export/pack/moon.png", &png(4, 2, &[50; 32]));
        let planet = ask(&source, "pale").unwrap();
        assert_eq!(planet.surface.surface.rgba, [50, 50, 50, 0].repeat(8));
    }
}
