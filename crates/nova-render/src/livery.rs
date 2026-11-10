//! Liveries painted onto a model's materials.

/// A livery slot: which of a livery's colours paints a material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintSlot {
    /// The main hull colour.
    Primary,
    /// Panels and stripes.
    Secondary,
    /// Small details and lights.
    Trim,
}
