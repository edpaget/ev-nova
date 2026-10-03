//! How the program ends once the event loop returns.

/// Why the window could not be opened, in the core's terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenFailure {
    /// The window itself could not be created.
    Window(String),
    /// No GPU adapter, or no device on it, could be had.
    NoGpu(String),
    /// A GPU was found, but it cannot draw into the window.
    Surface(String),
}

/// The process's exit code, and what to print to stderr first, if anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exit {
    /// The exit code.
    pub code: u8,
    /// The message for stderr.
    pub message: Option<String>,
}

/// How the program ends after the event loop: success, unless the window
/// could not be opened, which exits 1 with a message saying why and, for a
/// missing GPU, what is needed.
#[must_use]
pub fn exit(failure: Option<&OpenFailure>) -> Exit {
    let Some(failure) = failure else {
        return Exit {
            code: 0,
            message: None,
        };
    };
    let message = match failure {
        OpenFailure::Window(detail) => format!("nova: could not open the window: {detail}"),
        OpenFailure::NoGpu(detail) => format!(
            "nova: no usable GPU ({detail}); nova needs a GPU with Metal, Vulkan or DirectX 12 \
             support"
        ),
        OpenFailure::Surface(detail) => {
            format!("nova: the GPU cannot draw into the window: {detail}")
        }
    };
    Exit {
        code: 1,
        message: Some(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failed(failure: &OpenFailure) -> Exit {
        exit(Some(failure))
    }

    #[test]
    fn a_window_that_opened_exits_0_quietly() {
        assert_eq!(
            exit(None),
            Exit {
                code: 0,
                message: None
            }
        );
    }

    #[test]
    fn a_window_that_could_not_be_created_exits_1() {
        assert_eq!(
            failed(&OpenFailure::Window("no display".into())),
            Exit {
                code: 1,
                message: Some("nova: could not open the window: no display".into())
            }
        );
    }

    #[test]
    fn a_missing_gpu_exits_1_saying_what_is_needed() {
        assert_eq!(
            failed(&OpenFailure::NoGpu("no adapter".into())),
            Exit {
                code: 1,
                message: Some(
                    "nova: no usable GPU (no adapter); nova needs a GPU with Metal, Vulkan \
                     or DirectX 12 support"
                        .into()
                )
            }
        );
    }

    #[test]
    fn a_gpu_that_cannot_draw_into_the_window_exits_1() {
        assert_eq!(
            failed(&OpenFailure::Surface("no format".into())),
            Exit {
                code: 1,
                message: Some("nova: the GPU cannot draw into the window: no format".into())
            }
        );
    }
}
