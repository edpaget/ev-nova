//! A rectangle of the window in physical pixels: the shape of
//! [`WindowPort::safe_area`](super::WindowPort::safe_area).

/// A rectangle of the window in physical pixels, from its top-left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PxRect {
    /// The left edge, in pixels from the window's left.
    pub x: u32,
    /// The top edge, in pixels from the window's top.
    pub y: u32,
    /// How wide, in pixels.
    pub width: u32,
    /// How tall, in pixels.
    pub height: u32,
}
