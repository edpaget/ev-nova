//! Which livery slot paints a material: the one rule both views share.
//!
//! A model's [`Paint`] lists, for each livery slot, the glTF material
//! names that take that slot's colour. [`slot_of`] answers which slot a
//! material name falls in. The top-down silhouette ([`painted`]) and the
//! renderer's 3D model adapter (`nova_render::GalaxyModels`) both read it
//! here, so the two views paint the same materials.
//!
//! The data says a material is in one slot at most (game-bible's
//! `galaxy check` holds it to that, not ev-nova), but a broken `paint`
//! must still draw: a name listed in more than one slot takes the
//! **later** slot (trim over secondary over primary), as the contact
//! sheet's `painted` does, each slot overwriting the one before. A
//! material with no name is in no slot; the callers never ask for one.
//!
//! [`painted`]: crate::outline::painted

use nova_galaxy_format::asset::Paint;
use nova_galaxy_format::fleet::Palette;
use nova_galaxy_format::id::Color;

/// A livery slot: one of a palette's three colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    /// The main hull colour.
    Primary,
    /// Panels and stripes.
    Secondary,
    /// Small details and lights.
    Trim,
}

impl Slot {
    /// The colour `palette` gives this slot.
    #[must_use]
    pub fn color(self, palette: &Palette) -> &Color {
        match self {
            Slot::Primary => &palette.primary,
            Slot::Secondary => &palette.secondary,
            Slot::Trim => &palette.trim,
        }
    }
}

/// The slot `paint` puts the material `name` in, the later slot winning
/// when it names the material twice (see the [module docs](self)); `None`
/// when no slot names it.
#[must_use]
pub fn slot_of(paint: &Paint, name: &str) -> Option<Slot> {
    [
        (Slot::Trim, &paint.trim),
        (Slot::Secondary, &paint.secondary),
        (Slot::Primary, &paint.primary),
    ]
    .into_iter()
    .find(|(_, names)| names.iter().any(|n| n == name))
    .map(|(slot, _)| slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint(primary: &[&str], secondary: &[&str], trim: &[&str]) -> Paint {
        let names = |n: &[&str]| n.iter().map(|s| (*s).to_owned()).collect();
        Paint {
            primary: names(primary),
            secondary: names(secondary),
            trim: names(trim),
        }
    }

    #[test]
    fn each_slot_holds_the_names_it_lists() {
        let paint = paint(&["metal", "hull"], &["metalRed"], &["metalDark"]);
        assert_eq!(slot_of(&paint, "metal"), Some(Slot::Primary));
        assert_eq!(slot_of(&paint, "hull"), Some(Slot::Primary));
        assert_eq!(slot_of(&paint, "metalRed"), Some(Slot::Secondary));
        assert_eq!(slot_of(&paint, "metalDark"), Some(Slot::Trim));
    }

    #[test]
    fn a_name_no_slot_lists_is_in_none() {
        let paint = paint(&["metal"], &["metalRed"], &["metalDark"]);
        assert_eq!(slot_of(&paint, "dark"), None);
        assert_eq!(slot_of(&paint, ""), None);
        assert_eq!(slot_of(&paint, "Metal"), None, "names match exactly");
        assert_eq!(slot_of(&Paint::default(), "metal"), None);
    }

    #[test]
    fn a_name_in_two_slots_takes_the_later() {
        let table = [
            (paint(&["x"], &["x"], &[]), Slot::Secondary),
            (paint(&[], &["x"], &["x"]), Slot::Trim),
            (paint(&["x"], &[], &["x"]), Slot::Trim),
            (paint(&["x"], &["x"], &["x"]), Slot::Trim),
        ];
        for (paint, slot) in table {
            assert_eq!(slot_of(&paint, "x"), Some(slot), "{paint:?}");
        }
    }

    #[test]
    fn a_slot_reads_its_own_colour_from_the_palette() {
        let color = |s: &str| Color::try_from(s.to_owned()).unwrap();
        let palette = Palette {
            primary: color("#ddccaa"),
            secondary: color("#aa8800"),
            trim: color("#222222"),
        };
        assert_eq!(Slot::Primary.color(&palette), &palette.primary);
        assert_eq!(Slot::Secondary.color(&palette), &palette.secondary);
        assert_eq!(Slot::Trim.color(&palette), &palette.trim);
    }
}
