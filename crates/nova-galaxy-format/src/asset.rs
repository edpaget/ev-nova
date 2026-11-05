//! The asset manifest (`assets/<id>.md`): one entry per piece of art,
//! naming the files the engine loads, the files artists edit, and where
//! they came from. Data names art only by asset id. Also the art budgets
//! (`budgets/<id>.md`) the art is held to.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::fleet::Role;
use crate::id::{Date, MediaPath, Url};

/// An asset (`assets/<id>.md`): one piece of art and where it came from.
///
/// A placeholder stands in for art nobody has made or found yet; it needs
/// only its `kind`, and may have no files. Any other asset needs `files`,
/// `licence`, `author`, `source_url` and `retrieved`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    /// What kind of art it is.
    pub kind: AssetKind,
    /// Whether it is a stand-in for art still to come.
    pub placeholder: bool,
    /// What the engine loads, under `media/export/`: one `.glb` or
    /// `.gltf` for a model (plus the buffers and images a `.gltf` names),
    /// one `.png` for a texture.
    #[serde(default)]
    pub files: Vec<MediaPath>,
    /// What artists edit, under `media/source/` (`.blend` and the like).
    #[serde(default)]
    pub sources: Vec<MediaPath>,
    /// Which way a model's nose points. glTF says +Z, but packs differ.
    #[serde(default)]
    pub front: Front,
    /// Which of a model's materials take a livery's colours. The engine
    /// sets each listed material's base colour to its slot's colour and
    /// keeps its metallic, roughness and textures; materials not listed
    /// keep their own colour. Only a model has it, and a ship that wears
    /// a livery needs it on its model.
    #[serde(default)]
    pub paint: Option<Paint>,
    /// The licence it is used under.
    #[serde(default)]
    pub licence: Option<Licence>,
    /// The licence text that came with it, copied next to it.
    #[serde(default)]
    pub licence_file: Option<MediaPath>,
    /// Who made it.
    #[serde(default)]
    pub author: Option<String>,
    /// The page it was downloaded from.
    #[serde(default)]
    pub source_url: Option<Url>,
    /// When it was downloaded.
    #[serde(default)]
    pub retrieved: Option<Date>,
}

/// What kind of art an asset is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AssetKind {
    /// A 3D model: a ship, station or outfit.
    Model,
    /// An image: a planet surface, a decal.
    Texture,
}

impl AssetKind {
    /// The kind as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Model => "model",
            AssetKind::Texture => "texture",
        }
    }
}

/// The glTF material names (`materials[].name`) that take each livery
/// slot's colour. A material is in one slot at most, and a material with
/// no name can't be painted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Paint {
    /// Materials that take the livery's `primary` colour.
    #[serde(default)]
    pub primary: Vec<String>,
    /// Materials that take the livery's `secondary` colour.
    #[serde(default)]
    pub secondary: Vec<String>,
    /// Materials that take the livery's `trim` colour.
    #[serde(default)]
    pub trim: Vec<String>,
}

impl Paint {
    /// Each slot's name, as written in data, and its materials.
    #[must_use]
    pub fn slots(&self) -> [(&'static str, &[String]); 3] {
        [
            ("primary", &self.primary),
            ("secondary", &self.secondary),
            ("trim", &self.trim),
        ]
    }

    /// Whether it paints no material at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots().iter().all(|(_, names)| names.is_empty())
    }
}

/// The model axis its nose points along. Up is always +Y.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum Front {
    /// +Z, glTF's convention.
    #[default]
    #[serde(rename = "+z")]
    PlusZ,
    /// -Z (Kenney's ships).
    #[serde(rename = "-z")]
    MinusZ,
    /// +X.
    #[serde(rename = "+x")]
    PlusX,
    /// -X.
    #[serde(rename = "-x")]
    MinusX,
}

impl Front {
    /// The nose's direction in the model's `[x, z]` plane.
    #[must_use]
    pub fn forward(self) -> [f64; 2] {
        match self {
            Front::PlusZ => [0.0, 1.0],
            Front::MinusZ => [0.0, -1.0],
            Front::PlusX => [1.0, 0.0],
            Front::MinusX => [-1.0, 0.0],
        }
    }

    /// The ship's right (starboard) in the model's `[x, z]` plane: the
    /// forward direction crossed with up (+Y).
    #[must_use]
    pub fn right(self) -> [f64; 2] {
        let [x, z] = self.forward();
        [-z, x]
    }
}

/// A licence art may be used under, by its SPDX id. Only licences that
/// allow commercial use and modification without asking are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
pub enum Licence {
    /// Public domain dedication: no conditions.
    #[serde(rename = "CC0-1.0")]
    Cc0,
    /// Attribution required.
    #[serde(rename = "CC-BY-3.0")]
    CcBy3,
    /// Attribution required.
    #[serde(rename = "CC-BY-4.0")]
    CcBy4,
}

impl Licence {
    /// The SPDX id.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Licence::Cc0 => "CC0-1.0",
            Licence::CcBy3 => "CC-BY-3.0",
            Licence::CcBy4 => "CC-BY-4.0",
        }
    }
}

/// An art budget (`budgets/<id>.md`): what each model, and for the
/// default budget each texture, may cost the engine. A design value, set
/// in data. The budget with no `roles` is the default: it covers every
/// model no role budget covers (stations, outfits, unused models, roles
/// with no budget) and every texture. A model several budgets cover meets
/// each of them, so its limit is the smallest.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// The roles whose finished ships' models this budget covers. Empty
    /// or left out for the default budget. A role is in one budget at
    /// most.
    #[serde(default)]
    pub roles: Vec<Role>,
    /// What one model may cost. Left out: models it covers aren't
    /// limited by it.
    #[serde(default)]
    pub model: Option<ModelBudget>,
    /// What one texture may cost. Only the default budget gives it:
    /// textures have no role.
    #[serde(default)]
    pub texture: Option<TextureBudget>,
}

/// What one model may cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelBudget {
    /// The most triangles it may draw, counting a mesh once per node
    /// that uses it.
    pub triangles: u32,
    /// The most KiB its files may take on disk, a `.gltf`'s buffers and
    /// images included.
    pub kib: u32,
}

/// What one texture may cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextureBudget {
    /// The most KiB its file may take on disk.
    pub kib: u32,
    /// The longest side it may have, in pixels.
    pub side: u32,
}

crate::model::kinds! {
    Asset => "asset" in "assets",
    Budget => "budget" in "budgets",
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;

    fn read(yaml: &str) -> Result<Asset, String> {
        parse::<Asset>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    #[test]
    fn a_kind_and_a_licence_are_named_as_in_data() {
        assert_eq!(AssetKind::Model.as_str(), "model");
        assert_eq!(AssetKind::Texture.as_str(), "texture");
        assert_eq!(Licence::Cc0.as_str(), "CC0-1.0");
        assert_eq!(Licence::CcBy3.as_str(), "CC-BY-3.0");
        assert_eq!(Licence::CcBy4.as_str(), "CC-BY-4.0");
    }

    #[test]
    fn each_front_points_forward_along_its_axis_with_right_a_quarter_turn_on() {
        let cases = [
            (Front::PlusZ, [0.0, 1.0], [-1.0, 0.0]),
            (Front::MinusZ, [0.0, -1.0], [1.0, 0.0]),
            (Front::PlusX, [1.0, 0.0], [0.0, 1.0]),
            (Front::MinusX, [-1.0, 0.0], [0.0, -1.0]),
        ];
        for (front, forward, right) in cases {
            assert_eq!(front.forward(), forward, "{front:?}");
            assert_eq!(front.right(), right, "{front:?}");
        }
    }

    #[test]
    fn an_asset_reads_its_files_and_provenance() {
        let asset = read(
            "kind: model\nplaceholder: false\n\
             files: [media/export/kenney-space-kit/craft_cargoB.glb]\n\
             licence: CC0-1.0\nlicence_file: media/export/kenney-space-kit/License.txt\n\
             author: Kenney\nsource_url: https://opengameart.org/content/space-kit-remade\n\
             retrieved: 2026-10-30\n",
        )
        .unwrap();
        assert_eq!(asset.kind, AssetKind::Model);
        assert_eq!(asset.licence, Some(Licence::Cc0));
        assert_eq!(asset.retrieved.unwrap().as_str(), "2026-10-30");
        assert!(asset.sources.is_empty());
    }

    #[test]
    fn a_placeholder_needs_only_its_kind() {
        let asset = read("kind: texture\nplaceholder: true\n").unwrap();
        assert!(asset.placeholder && asset.files.is_empty() && asset.author.is_none());
    }

    #[test]
    fn a_model_faces_plus_z_unless_it_says() {
        let plain = read("kind: model\nplaceholder: true\n").unwrap();
        assert_eq!(plain.front, Front::PlusZ);
        let kenney = read("kind: model\nplaceholder: true\nfront: -z\n").unwrap();
        assert_eq!(kenney.front, Front::MinusZ);
        assert!(read("kind: model\nplaceholder: true\nfront: up\n").is_err());
    }

    #[test]
    fn a_model_maps_livery_slots_to_its_materials() {
        let asset = read(
            "kind: model\nplaceholder: true\npaint: {primary: [metal, hull], trim: [metalDark]}\n",
        )
        .unwrap();
        let paint = asset.paint.unwrap();
        assert_eq!(paint.primary, ["metal", "hull"]);
        assert!(paint.secondary.is_empty());
        assert_eq!(paint.trim, ["metalDark"]);
        assert_eq!(
            paint.slots().map(|(slot, names)| (slot, names.len())),
            [("primary", 2), ("secondary", 0), ("trim", 1)]
        );
        assert!(!paint.is_empty());
        assert!(Paint::default().is_empty());
        assert!(
            read("kind: model\nplaceholder: true\n")
                .unwrap()
                .paint
                .is_none()
        );
    }

    #[test]
    fn paint_names_only_the_three_slots() {
        let err = read("kind: model\nplaceholder: true\npaint: {accent: [metal]}\n").unwrap_err();
        assert!(err.contains("accent"), "{err}");
    }

    fn budget(yaml: &str) -> Result<Budget, String> {
        parse::<Budget>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    #[test]
    fn a_budget_reads_its_roles_and_limits() {
        let b = budget(
            "roles: [cruiser, carrier]\nmodel: {triangles: 60000, kib: 8192}\n\
             texture: {kib: 4096, side: 2048}\n",
        )
        .unwrap();
        assert_eq!(b.roles, [Role::Cruiser, Role::Carrier]);
        assert_eq!(
            b.model,
            Some(ModelBudget {
                triangles: 60000,
                kib: 8192
            })
        );
        assert_eq!(
            b.texture,
            Some(TextureBudget {
                kib: 4096,
                side: 2048
            })
        );
        let default = budget("model: {triangles: 1, kib: 1}\n").unwrap();
        assert!(default.roles.is_empty() && default.texture.is_none());
    }

    #[test]
    fn a_budget_gives_both_numbers_of_a_limit_and_nothing_else() {
        assert!(budget("model: {triangles: 1}\n").is_err());
        assert!(budget("texture: {kib: 1, side: 1, depth: 1}\n").is_err());
        assert!(budget("roles: [tanker]\n").is_err());
        assert!(budget("sound: {kib: 1}\n").is_err());
    }

    #[test]
    fn placeholder_must_be_given() {
        assert!(read("kind: model\n").is_err());
    }

    #[test]
    fn an_unknown_licence_is_an_error() {
        let err = read("kind: model\nplaceholder: true\nlicence: GPL-3.0\n").unwrap_err();
        assert!(err.contains("GPL-3.0"), "{err}");
    }
}
