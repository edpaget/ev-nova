//! Which art budget (`budgets/<id>.md`) covers a model, and each way a
//! model breaks it.
//!
//! Ported from game-bible's `galaxy` tool (`check/art.rs` `limits`,
//! `tighten` and `over_budget`), for models only: [`limits`] works out
//! what a model asset may cost, [`measure`] what a read model costs, and
//! [`over_budget`] each limit the cost breaks. The texture limits
//! (`kib` and `side` in the default budget's `texture`) arrive with the
//! first texture the engine loads, the planet shader's surfaces
//! (roadmap `engine-3d` phase 7); until then a texture asset has no
//! limits here.

use std::fmt;

use nova_galaxy_format::asset::{AssetKind, Budget};
use nova_galaxy_format::fleet::{Role, resolve};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy};
use nova_galaxy_format::model::{Kind, Visual};

use crate::scene::Scene;

/// The most a model may cost by one measure, and the budget that set it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limit {
    /// The most it may cost.
    pub most: u32,
    /// The budget that set it.
    pub budget: Id,
}

impl Limit {
    /// Where the budget that set it is written: `budgets/<id>.md`.
    #[must_use]
    pub fn file(&self) -> String {
        budget_file(&self.budget)
    }
}

/// What one model may cost, from the budgets in data. A limit no budget
/// gives is `None`, and isn't checked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// The most triangles it may draw.
    pub triangles: Option<Limit>,
    /// The most KiB its files may take.
    pub kib: Option<Limit>,
}

/// What a read model costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    /// The triangles it draws, a mesh once per node that uses it.
    pub triangles: u32,
    /// The bytes its files take: the model and every file it names.
    pub bytes: u64,
}

/// What a limit measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    /// Triangles drawn.
    Triangles,
    /// KiB on disk (its `used` is in bytes).
    Kib,
}

/// One way a model breaks its budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overrun {
    /// What is over.
    pub measure: Measure,
    /// What the model uses: triangles, or bytes for [`Measure::Kib`].
    pub used: u64,
    /// The limit: triangles, or KiB.
    pub most: u32,
    /// The budget that set the limit.
    pub budget: Id,
}

impl Overrun {
    /// Where the budget that set the limit is written: `budgets/<id>.md`.
    #[must_use]
    pub fn file(&self) -> String {
        budget_file(&self.budget)
    }
}

impl fmt::Display for Overrun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (most, file) = (self.most, self.file());
        match self.measure {
            Measure::Triangles => write!(
                f,
                "draws {} triangles, over the {most} budget in `{file}`",
                self.used
            ),
            Measure::Kib => write!(
                f,
                "is {} KiB, over the {most} KiB budget in `{file}`",
                self.used.div_ceil(1024)
            ),
        }
    }
}

/// `budgets/<id>.md`.
fn budget_file(budget: &Id) -> String {
    format!("{}/{budget}.md", Budget::DIR)
}

/// What the model `asset` may cost. Each use of it picks a budget: the
/// one for the role of the finished ship that uses it, or else the
/// default (an outfit's or a stellar's model, a ship whose role no budget
/// names, a model nothing uses). Each limit is the smallest those budgets
/// give. An asset that isn't a model, or isn't in the manifest, has none.
#[must_use]
pub fn limits(galaxy: &Galaxy, asset: &Id) -> Limits {
    let mut limits = Limits::default();
    if galaxy.assets.get(asset).map(|a| a.data.kind) != Some(AssetKind::Model) {
        return limits;
    }
    let budgets = &galaxy.budgets;
    let default = budgets.iter().find(|(_, b)| b.data.roles.is_empty());
    let for_role = |role: Option<Role>| {
        role.and_then(|r| budgets.iter().find(|(_, b)| b.data.roles.contains(&r)))
            .or(default)
    };
    let mut uses: Vec<Option<(&Id, &Doc<Budget>)>> = galaxy
        .ships
        .keys()
        .filter_map(|id| resolve(galaxy, id).ok())
        .filter(|ship| ship.model == Some(asset))
        .map(|ship| for_role(ship.role))
        .collect();
    let elsewhere = galaxy
        .outfits
        .values()
        .any(|o| o.data.model.as_ref() == Some(asset))
        || galaxy
            .stellars
            .values()
            .any(|s| s.data.visual == Some(Visual::Asset(asset.clone())));
    if elsewhere || uses.is_empty() {
        uses.push(default);
    }
    for (id, budget) in uses.into_iter().flatten() {
        if let Some(model) = budget.data.model {
            tighten(&mut limits.triangles, model.triangles, id);
            tighten(&mut limits.kib, model.kib, id);
        }
    }
    limits
}

/// Lowers `limit` to `most`, set by `budget`, unless it is already as
/// low.
fn tighten(limit: &mut Option<Limit>, most: u32, budget: &Id) {
    if limit.as_ref().is_none_or(|l| most < l.most) {
        *limit = Some(Limit {
            most,
            budget: budget.clone(),
        });
    }
}

/// What `scene` costs.
#[must_use]
pub fn measure(scene: &Scene) -> Cost {
    Cost {
        triangles: scene.triangles,
        bytes: scene.disk_bytes,
    }
}

/// Each limit of `limits` that `cost` is over, by any amount.
#[must_use]
pub fn over_budget(limits: &Limits, cost: Cost) -> Vec<Overrun> {
    let mut over = Vec::new();
    if let Some(l) = &limits.triangles
        && cost.triangles > l.most
    {
        over.push(Overrun {
            measure: Measure::Triangles,
            used: u64::from(cost.triangles),
            most: l.most,
            budget: l.budget.clone(),
        });
    }
    if let Some(l) = &limits.kib
        && cost.bytes > u64::from(l.most) * 1024
    {
        over.push(Overrun {
            measure: Measure::Kib,
            used: cost.bytes,
            most: l.most,
            budget: l.budget.clone(),
        });
    }
    over
}

#[cfg(test)]
mod tests {
    use nova_galaxy_format::fixture;
    use nova_galaxy_format::load::load;
    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::fixture::{Builder, ImageAt, PNG};
    use crate::scene::read_scene;

    fn id(s: &str) -> Id {
        Id::try_from(s.to_owned()).unwrap()
    }

    /// The limit `most` set by `budget`, as [`Limits`] holds it.
    #[allow(clippy::unnecessary_wraps)]
    fn limit(most: u32, budget: &str) -> Option<Limit> {
        Some(Limit {
            most,
            budget: id(budget),
        })
    }

    /// The model limits of `asset` in `source`'s galaxy.
    fn limits_in(source: &MemSource, asset: &str) -> Limits {
        limits(&load(source).0, &id(asset))
    }

    /// The format's test galaxy: a skiff and an armed skiff, both
    /// shuttles, on the `hull` model; the `spire` model on a station; a
    /// default budget of 20000 triangles and 4096 KiB.
    fn galaxy() -> MemSource {
        fixture::galaxy()
    }

    const SMALL_CRAFT: &str = "---\nroles: [shuttle]\nmodel: {triangles: 30000, kib: 8000}\n---\n";

    #[test]
    fn a_ship_s_model_takes_its_role_s_budget() {
        // Looser than the default, which no use of `hull` picks.
        let source = galaxy().with("budgets/small-craft.md", SMALL_CRAFT);
        assert_eq!(
            limits_in(&source, "hull"),
            Limits {
                triangles: limit(30000, "small-craft"),
                kib: limit(8000, "small-craft"),
            }
        );
    }

    #[test]
    fn a_role_no_budget_names_takes_the_default() {
        assert_eq!(
            limits_in(&galaxy(), "hull"),
            Limits {
                triangles: limit(20000, "default"),
                kib: limit(4096, "default"),
            }
        );
    }

    #[test]
    fn a_model_two_roles_use_meets_the_smaller_limit_of_each() {
        let source = galaxy()
            .with(
                "ships/cruiser.md",
                "---\nname: Cruiser\nrole: cruiser\nmodel: hull\n\
                 size: {length: 80, beam: 30, collision_radius: 40}\n---\n",
            )
            .with(
                "budgets/small-craft.md",
                "---\nroles: [shuttle]\nmodel: {triangles: 100, kib: 1}\n---\n",
            )
            .with(
                "budgets/capital.md",
                "---\nroles: [cruiser]\nmodel: {triangles: 11, kib: 9}\n---\n",
            );
        assert_eq!(
            limits_in(&source, "hull"),
            Limits {
                triangles: limit(11, "capital"),
                kib: limit(1, "small-craft"),
            }
        );
    }

    #[test]
    fn an_outfit_s_or_stellar_s_model_also_meets_the_default() {
        let default = Limits {
            triangles: limit(20000, "default"),
            kib: limit(4096, "default"),
        };
        let outfit = galaxy().with("budgets/small-craft.md", SMALL_CRAFT).with(
            "outfits/blaster.md",
            "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\nprice: 100\n\
             model: hull\n---\n",
        );
        assert_eq!(limits_in(&outfit, "hull"), default);
        let stellar = galaxy().with("budgets/small-craft.md", SMALL_CRAFT).with(
            "stellars/alpha-yard.md",
            "---\nname: Alpha Yard\nkind: station\nsystem: alpha\nposition: [0, 0]\n\
             visual: {asset: hull}\n---\n",
        );
        assert_eq!(limits_in(&stellar, "hull"), default);
        // A stellar's own model, which no ship uses.
        assert_eq!(limits_in(&galaxy(), "spire"), default);
    }

    #[test]
    fn an_unused_model_takes_the_default() {
        let source = galaxy().with(
            "assets/spare.md",
            "---\nkind: model\nplaceholder: true\n---\n",
        );
        assert_eq!(
            limits_in(&source, "spare"),
            Limits {
                triangles: limit(20000, "default"),
                kib: limit(4096, "default"),
            }
        );
    }

    #[test]
    fn no_budget_limits_nothing() {
        let source = galaxy().without("budgets/default.md");
        assert_eq!(limits_in(&source, "hull"), Limits::default());
        assert_eq!(limits_in(&galaxy(), "nothing"), Limits::default());
        // A texture's limits come with the planet shader's surfaces.
        assert_eq!(limits_in(&galaxy(), "surface"), Limits::default());
        // A budget giving only texture limits limits no model.
        let textures_only = galaxy().with(
            "budgets/default.md",
            "---\ntexture: {kib: 1, side: 1}\n---\n",
        );
        assert_eq!(limits_in(&textures_only, "hull"), Limits::default());
    }

    #[test]
    fn a_model_over_by_one_triangle_or_one_byte_is_over_and_at_the_limit_is_not() {
        let limits = Limits {
            triangles: limit(12, "small-craft"),
            kib: limit(1, "default"),
        };
        let cost = |triangles, bytes| Cost { triangles, bytes };
        assert_eq!(over_budget(&limits, cost(12, 1024)), []);
        assert_eq!(
            over_budget(&limits, cost(13, 1024)),
            [Overrun {
                measure: Measure::Triangles,
                used: 13,
                most: 12,
                budget: id("small-craft"),
            }]
        );
        assert_eq!(
            over_budget(&limits, cost(12, 1025)),
            [Overrun {
                measure: Measure::Kib,
                used: 1025,
                most: 1,
                budget: id("default"),
            }]
        );
        assert_eq!(over_budget(&limits, cost(13, 1025)).len(), 2);
        assert_eq!(
            over_budget(&Limits::default(), cost(u32::MAX, u64::MAX)),
            []
        );
    }

    #[test]
    fn an_overrun_says_what_it_used_the_limit_and_the_budget_file() {
        let triangles = Overrun {
            measure: Measure::Triangles,
            used: 13,
            most: 12,
            budget: id("small-craft"),
        };
        assert_eq!(triangles.file(), "budgets/small-craft.md");
        assert_eq!(
            triangles.to_string(),
            "draws 13 triangles, over the 12 budget in `budgets/small-craft.md`"
        );
        let kib = Overrun {
            measure: Measure::Kib,
            used: 1025,
            most: 1,
            budget: id("default"),
        };
        assert_eq!(
            kib.to_string(),
            "is 2 KiB, over the 1 KiB budget in `budgets/default.md`"
        );
        assert_eq!(limit(1, "capital").unwrap().file(), "budgets/capital.md");
    }

    #[test]
    fn kib_counts_a_gltf_s_buffers_and_images() {
        let (json, bin) = Builder::new([1.0; 3])
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("hull.bin");
        let read = |path: &str| match path {
            "p/hull.bin" => Some(bin.clone()),
            "p/skin.png" => Some(PNG.to_vec()),
            _ => None,
        };
        let scene = read_scene("p/hull.gltf", json.as_bytes(), read).unwrap();
        assert_eq!(
            measure(&scene),
            Cost {
                triangles: 12,
                bytes: (json.len() + bin.len() + PNG.len()) as u64,
            }
        );
        // A GLB's buffer and images are inside its own bytes.
        let glb = Builder::new([1.0; 3]).image(ImageAt::Embedded).glb();
        let scene = read_scene("p/hull.glb", &glb, |_| None).unwrap();
        assert_eq!(measure(&scene).bytes, glb.len() as u64);
    }
}
