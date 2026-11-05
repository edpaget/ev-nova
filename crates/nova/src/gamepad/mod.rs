//! Gamepads: the [`Gamepads`] port the app polls once a frame, in the
//! core's own terms, and the pure modules that turn what it reports into
//! screen input.
//!
//! - [`Gamepads`] is the port: one `poll` a frame appends what the pads
//!   did since the last ([`PadEvent`]): a pad connected or lost, a button
//!   by position, a stick axis's value, a trigger's value. Buttons and
//!   axes are `nova_view`'s position types, so no device type leaks past
//!   this module. The real adapter is `platform::gamepads` (gilrs, on the
//!   desktops); an iPad build adds a `GCController` one behind the same
//!   port (rdm:roadmap/ipad-port).
//! - `fixture` (the `fixture` feature, and this crate's tests) publishes
//!   the fake (`FakeGamepads`) and the contract check (`check_contract`)
//!   every adapter runs.
//! - [`inputs`] ([`inputs::PadInputs`]) decides what the events mean for
//!   the screens: buttons pass through, a stick or trigger past the press
//!   threshold presses a button (with hysteresis), two pads merge, and a
//!   lost pad lets go of everything it held.
//!
//! Where an addition goes: a new kind of thing a pad reports is a
//! [`PadEvent`] variant, given in core terms by every adapter and checked
//! by the contract; a new decision about what it means is
//! [`inputs`]'s, pinned by its tests; what the device last used is, is
//! `app::last_device`'s. An adapter forwards and decides nothing.

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
pub mod inputs;

use nova_view::{PadAxis, PadButton};

/// A pad, as the adapter numbers it: stable while it stays connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PadId(pub usize);

/// One thing a pad did, in the core's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadEvent {
    /// The pad connected, or was found connected as the port started.
    Connected(PadId),
    /// The pad was lost.
    Disconnected(PadId),
    /// A button went down or up: a face button, a shoulder, the d-pad,
    /// Start, Select or a stick's click. Never a trigger (its
    /// [`PadEvent::Trigger`] value) and never a stick direction, which
    /// [`inputs`] makes from the axes.
    Button {
        /// The pad.
        pad: PadId,
        /// The button.
        button: PadButton,
        /// Down (`true`) or up.
        pressed: bool,
    },
    /// A stick axis moved to `value`, -1 to 1, X positive right and Y
    /// positive up.
    Axis {
        /// The pad.
        pad: PadId,
        /// The axis.
        axis: PadAxis,
        /// Where it is.
        value: f32,
    },
    /// A trigger (`LeftTrigger` or `RightTrigger`) moved to `value`, 0 to
    /// 1.
    Trigger {
        /// The pad.
        pad: PadId,
        /// The trigger.
        button: PadButton,
        /// How far it is pulled.
        value: f32,
    },
}

impl PadEvent {
    /// The pad the event is about.
    #[must_use]
    pub const fn pad(&self) -> PadId {
        match *self {
            Self::Connected(pad)
            | Self::Disconnected(pad)
            | Self::Button { pad, .. }
            | Self::Axis { pad, .. }
            | Self::Trigger { pad, .. } => pad,
        }
    }
}

/// The gamepads the app reads: polled once a frame from the main thread,
/// never blocking.
pub trait Gamepads {
    /// Appends to `out` what the pads did since the last poll, in order,
    /// leaving what `out` held. The first poll reports each pad already
    /// connected as [`PadEvent::Connected`].
    fn poll(&mut self, out: &mut Vec<PadEvent>);
}
