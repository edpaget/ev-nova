//! The decoding budget: how many pixels a decoder may produce.
//!
//! Every decoder checks the size its header claims against this budget
//! before allocating anything, so a tiny hostile resource cannot ask for
//! gigabytes.

use super::GraphicsError;

/// The most pixels one decode may produce: 64 Mi pixels, 256 MiB of RGBA,
/// about three times the largest stock sprite sheet (22.1 M pixels).
pub const MAX_PIXELS: u64 = 1 << 26;

/// The most pixels one decode may produce per byte of input. The stock
/// maximum is 39 (a sparse sprite sheet); an all-transparent sprite sheet
/// 4096 pixels wide, one 4-byte token per line, sits exactly at the limit.
pub const MAX_PIXELS_PER_INPUT_BYTE: u64 = 1024;

/// Rejects `pixels` with [`GraphicsError::TooLarge`] when it exceeds
/// [`MAX_PIXELS`] or `input_len * MAX_PIXELS_PER_INPUT_BYTE`.
pub(crate) fn check_budget(pixels: u64, input_len: usize) -> Result<(), GraphicsError> {
    let per_input = (input_len as u64).saturating_mul(MAX_PIXELS_PER_INPUT_BYTE);
    if pixels > MAX_PIXELS || pixels > per_input {
        return Err(GraphicsError::TooLarge { pixels, input_len });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn too_large(pixels: u64, input_len: usize) -> Result<(), GraphicsError> {
        Err(GraphicsError::TooLarge { pixels, input_len })
    }

    #[test]
    fn the_absolute_limit_is_inclusive() {
        let input = usize::MAX;
        assert_eq!(check_budget(MAX_PIXELS, input), Ok(()));
        assert_eq!(
            check_budget(MAX_PIXELS + 1, input),
            too_large(MAX_PIXELS + 1, input)
        );
    }

    #[test]
    fn the_per_input_byte_limit_is_inclusive() {
        assert_eq!(check_budget(10 * 1024, 10), Ok(()));
        assert_eq!(
            check_budget(10 * 1024 + 1, 10),
            too_large(10 * 1024 + 1, 10)
        );
        assert_eq!(check_budget(0, 0), Ok(()));
        assert_eq!(check_budget(1, 0), too_large(1, 0));
    }
}
