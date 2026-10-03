//! What a window surface should do before each frame.
//!
//! The surface adapter makes no decisions of its own: before each submit
//! it asks [`surface_action`], then performs the raw calls, and
//! [`acquire_outcome`] decides what the texture it got means.

/// How the last attempt to get a surface texture went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquireOutcome {
    /// A texture was acquired and presented.
    Acquired,
    /// A texture was acquired and presented, but the surface no longer
    /// matches it.
    Suboptimal,
    /// No texture this time (timed out, or the window is hidden); the
    /// surface itself is fine.
    Skipped,
    /// The surface no longer matches the window.
    Outdated,
    /// The surface was lost.
    Lost,
}

/// What asking a window surface for its next texture gave: wgpu's
/// `CurrentSurfaceTexture`, variant for variant, with the texture as `T`
/// so the mapping in [`acquire_outcome`] can be tested without a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquireResult<T> {
    /// A texture, matching the surface.
    Success(T),
    /// A texture, but the surface no longer matches it.
    Suboptimal(T),
    /// Timed out waiting for a texture.
    Timeout,
    /// The window is hidden.
    Occluded,
    /// The surface no longer matches the window.
    Outdated,
    /// The surface was lost.
    Lost,
    /// wgpu raised a validation error.
    Validation,
}

/// The texture to draw into, if any, and the outcome to remember for the
/// next [`surface_action`]. A validation error is treated as a lost
/// surface, so the next frame reconfigures it.
#[must_use]
pub fn acquire_outcome<T>(result: AcquireResult<T>) -> (Option<T>, AcquireOutcome) {
    match result {
        AcquireResult::Success(texture) => (Some(texture), AcquireOutcome::Acquired),
        AcquireResult::Suboptimal(texture) => (Some(texture), AcquireOutcome::Suboptimal),
        AcquireResult::Timeout | AcquireResult::Occluded => (None, AcquireOutcome::Skipped),
        AcquireResult::Outdated => (None, AcquireOutcome::Outdated),
        AcquireResult::Lost | AcquireResult::Validation => (None, AcquireOutcome::Lost),
    }
}

/// What to do with the surface before drawing a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceAction {
    /// Configure the surface for the target size, then draw.
    Reconfigure,
    /// Draw.
    Render,
    /// Draw nothing this frame.
    Drop,
}

/// Decides what to do before a frame, given the size the surface is
/// configured for (if any), the target size and how the last acquire went.
///
/// A zero-sized target (a minimised window) never draws. Otherwise the
/// surface is reconfigured when it is unconfigured, the wrong size, or was
/// last reported lost, outdated or suboptimal; the frame that hit the bad
/// acquire was already dropped, as it had no texture to draw into.
#[must_use]
pub fn surface_action(
    configured: Option<(u32, u32)>,
    target: (u32, u32),
    last_acquire: AcquireOutcome,
) -> SurfaceAction {
    let stale = matches!(
        last_acquire,
        AcquireOutcome::Lost | AcquireOutcome::Outdated | AcquireOutcome::Suboptimal
    );
    if target.0 == 0 || target.1 == 0 {
        SurfaceAction::Drop
    } else if configured != Some(target) || stale {
        SurfaceAction::Reconfigure
    } else {
        SurfaceAction::Render
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_acquired_texture_is_drawn_into() {
        assert_eq!(
            acquire_outcome(AcquireResult::Success("texture")),
            (Some("texture"), AcquireOutcome::Acquired)
        );
        assert_eq!(
            acquire_outcome(AcquireResult::Suboptimal("texture")),
            (Some("texture"), AcquireOutcome::Suboptimal)
        );
    }

    #[test]
    fn a_timeout_or_hidden_window_skips_the_frame() {
        for result in [AcquireResult::Timeout, AcquireResult::Occluded] {
            assert_eq!(
                acquire_outcome::<()>(result),
                (None, AcquireOutcome::Skipped),
                "{result:?}"
            );
        }
    }

    #[test]
    fn an_outdated_surface_is_outdated() {
        assert_eq!(
            acquire_outcome::<()>(AcquireResult::Outdated),
            (None, AcquireOutcome::Outdated)
        );
    }

    #[test]
    fn a_lost_surface_or_validation_error_is_lost() {
        for result in [AcquireResult::Lost, AcquireResult::Validation] {
            assert_eq!(
                acquire_outcome::<()>(result),
                (None, AcquireOutcome::Lost),
                "{result:?}"
            );
        }
    }

    #[test]
    fn a_configured_surface_of_the_right_size_renders() {
        for last in [AcquireOutcome::Acquired, AcquireOutcome::Skipped] {
            assert_eq!(
                surface_action(Some((640, 480)), (640, 480), last),
                SurfaceAction::Render
            );
        }
    }

    #[test]
    fn an_unconfigured_surface_is_configured() {
        assert_eq!(
            surface_action(None, (640, 480), AcquireOutcome::Acquired),
            SurfaceAction::Reconfigure
        );
    }

    #[test]
    fn a_resized_target_reconfigures() {
        assert_eq!(
            surface_action(Some((640, 480)), (641, 480), AcquireOutcome::Acquired),
            SurfaceAction::Reconfigure
        );
        assert_eq!(
            surface_action(Some((640, 480)), (640, 481), AcquireOutcome::Acquired),
            SurfaceAction::Reconfigure
        );
    }

    #[test]
    fn a_lost_outdated_or_suboptimal_surface_reconfigures() {
        for last in [
            AcquireOutcome::Lost,
            AcquireOutcome::Outdated,
            AcquireOutcome::Suboptimal,
        ] {
            assert_eq!(
                surface_action(Some((640, 480)), (640, 480), last),
                SurfaceAction::Reconfigure,
                "{last:?}"
            );
        }
    }

    #[test]
    fn a_zero_sized_target_never_renders() {
        for target in [(0, 480), (640, 0), (0, 0)] {
            assert_eq!(
                surface_action(Some((640, 480)), target, AcquireOutcome::Lost),
                SurfaceAction::Drop
            );
            assert_eq!(
                surface_action(None, target, AcquireOutcome::Acquired),
                SurfaceAction::Drop
            );
        }
    }
}
