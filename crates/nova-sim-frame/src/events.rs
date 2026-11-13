//! What the session tells a view about the scene it draws
//! ([`SessionEvent`]), from `nova_sim::session::events`, which re-exports
//! it, raises the events and says what each one asks of the view.

/// A change to what a view draws of the session (see
/// `nova_sim::session::events`' module docs, which say what each asks of
/// the view).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    /// The player's ship became a class, possibly the one it was.
    ShipChanged,
    /// The system whose stellars are flown in was entered afresh.
    SystemChanged,
    /// The player was put at another stellar of the system shown.
    StellarChanged,
    /// The galaxy overlay changed.
    GalaxyChanged,
    /// A rebuild point changed which stellars shown are dead since the
    /// last rebuild, or the sheet whose frame count one of them animates
    /// over (`Session::counted_sheet`): the view draws the stellars again
    /// and shows each changed one's current frame.
    StellarsRebuilt,
    /// The flight's starfield is drawn afresh about the ship.
    StarfieldReset,
    /// Every stellar's frames were reset, as a take-off or a gate's exit
    /// resets them. The session resets every stellar's frames itself;
    /// nothing in the view acts on it.
    StellarFramesReset,
}
