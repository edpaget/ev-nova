//! A movie's frames as a renderer draws them: a [`MovieScreen`] a player
//! owns shows each decoded frame, and hands it out as a [`MovieFrame`] for
//! [`DrawCommand::MovieFrame`](crate::DrawCommand::MovieFrame).
//!
//! # For a decoder
//!
//! Make one [`MovieScreen`] per player, and [`show`](MovieScreen::show)
//! each frame as it decodes: straight (not premultiplied) RGBA8, row-major,
//! top row first. Draw [`MovieScreen::frame`] in `draw_frame`'s rect with
//! [`DrawList::movie_frame`](crate::DrawList::movie_frame). Each frame shown
//! is a new generation on the screen's stream, and the renderer uploads a
//! frame only when its stream's generation moves on, so drawing the same
//! frame each display frame costs no upload. The pixels are shared
//! ([`Arc`]), so a frame is cheap to clone into a draw list, and may be
//! decoded on another thread.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use nova_data::graphics::Image;

/// One source of movie frames: a [`MovieScreen`]. No two screens share
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamId(u64);

impl StreamId {
    /// A stream no other has been given.
    fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// One frame of a movie: its pixels, the stream it came from and its
/// generation on that stream. Two frames are equal when their stream and
/// generation are, which by construction means their pixels are too; the
/// pixels are never compared.
#[derive(Clone)]
pub struct MovieFrame {
    stream: StreamId,
    generation: u64,
    image: Arc<Image>,
}

impl MovieFrame {
    /// The stream it came from.
    #[must_use]
    pub fn stream(&self) -> StreamId {
        self.stream
    }

    /// Its generation on its stream: each frame shown has a new one.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Its width and height in pixels.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.image.width(), self.image.height())
    }

    /// Its pixels.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.image
    }
}

impl PartialEq for MovieFrame {
    fn eq(&self, other: &Self) -> bool {
        (self.stream, self.generation) == (other.stream, other.generation)
    }
}

impl fmt::Debug for MovieFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (width, height) = self.size();
        write!(
            f,
            "MovieFrame {{ stream: {:?}, generation: {}, size: {width} x {height} }}",
            self.stream, self.generation
        )
    }
}

/// Where a movie player shows its frames: one stream, whose current frame
/// it draws ([`MovieScreen::frame`]).
#[derive(Debug)]
pub struct MovieScreen {
    stream: StreamId,
    generation: u64,
    frame: Option<MovieFrame>,
}

impl Default for MovieScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl MovieScreen {
    /// A screen on a stream of its own, showing nothing.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stream: StreamId::fresh(),
            generation: 0,
            frame: None,
        }
    }

    /// Shows a `width` x `height` frame of straight RGBA8 `rgba`, row-major
    /// and top row first, as a new generation: whether it was shown. A
    /// buffer that isn't `4 × width × height` bytes is refused, and the
    /// last frame stays.
    pub fn show(&mut self, width: u32, height: u32, rgba: Vec<u8>) -> bool {
        let Some(image) = Image::from_rgba(width, height, rgba) else {
            return false;
        };
        self.generation += 1;
        self.frame = Some(MovieFrame {
            stream: self.stream,
            generation: self.generation,
            image: Arc::new(image),
        });
        true
    }

    /// The frame shown last, if any.
    #[must_use]
    pub fn frame(&self) -> Option<MovieFrame> {
        self.frame.clone()
    }

    /// Shows nothing.
    pub fn clear(&mut self) {
        self.frame = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        rgba.repeat((width * height) as usize)
    }

    fn showing(rgba: [u8; 4]) -> MovieScreen {
        let mut screen = MovieScreen::new();
        assert!(screen.show(2, 1, pixels(2, 1, rgba)));
        screen
    }

    #[test]
    fn a_new_screen_has_no_frame() {
        assert_eq!(MovieScreen::new().frame(), None);
    }

    #[test]
    fn a_shown_frame_carries_its_size_and_pixels() {
        let frame = showing([1, 2, 3, 4]).frame().expect("shown");
        assert_eq!(frame.size(), (2, 1));
        assert_eq!(frame.image().pixels(), [1, 2, 3, 4, 1, 2, 3, 4]);
    }

    #[test]
    fn each_frame_shown_has_a_new_generation_on_the_same_stream() {
        let mut screen = showing([1, 2, 3, 4]);
        let first = screen.frame().expect("shown");
        assert!(screen.show(1, 1, pixels(1, 1, [5, 6, 7, 8])));
        let second = screen.frame().expect("shown");
        assert_eq!(second.stream(), first.stream());
        assert_ne!(second.generation(), first.generation());
        assert_eq!(second.size(), (1, 1));
    }

    #[test]
    fn two_screens_have_different_streams() {
        let one = showing([0; 4]).frame().expect("shown");
        let two = showing([0; 4]).frame().expect("shown");
        assert_ne!(one.stream(), two.stream());
        assert_ne!(one, two);
    }

    #[test]
    fn a_buffer_of_the_wrong_length_is_refused_and_the_last_frame_stays() {
        let mut screen = showing([1, 2, 3, 4]);
        let before = screen.frame();
        assert!(!screen.show(2, 2, pixels(2, 1, [9; 4])));
        assert_eq!(screen.frame(), before);
        assert_eq!(
            screen.frame().expect("kept").image().pixels(),
            [1, 2, 3, 4, 1, 2, 3, 4]
        );
    }

    #[test]
    fn clear_leaves_no_frame() {
        let mut screen = showing([1, 2, 3, 4]);
        screen.clear();
        assert_eq!(screen.frame(), None);
    }

    #[test]
    fn frames_compare_by_stream_and_generation_not_pixels() {
        let mut screen = showing([1, 2, 3, 4]);
        let first = screen.frame().expect("shown");
        assert_eq!(first.clone(), first);
        assert!(screen.show(2, 1, pixels(2, 1, [1, 2, 3, 4])));
        let same_pixels = screen.frame().expect("shown");
        assert_ne!(same_pixels, first, "a new generation, the same pixels");
    }

    #[test]
    fn a_frame_debugs_as_its_stream_generation_and_size() {
        let frame = showing([1, 2, 3, 4]).frame().expect("shown");
        let debugged = format!("{frame:?}");
        assert_eq!(
            debugged,
            format!(
                "MovieFrame {{ stream: {:?}, generation: 1, size: 2 x 1 }}",
                frame.stream()
            )
        );
    }

    #[test]
    fn a_default_screen_is_a_new_one() {
        let screen = MovieScreen::default();
        assert_eq!(screen.frame(), None);
        let other = showing([0; 4]).frame().expect("shown");
        let mut screen = screen;
        assert!(screen.show(1, 1, vec![0; 4]));
        assert_ne!(screen.frame().expect("shown").stream(), other.stream());
    }
}
