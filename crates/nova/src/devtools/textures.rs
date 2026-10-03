//! The resource browser's preview frame as an egui texture.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use nova_data::graphics::Image;
use nova_view::devtools::{ResType, type_code};

/// `image` (straight-alpha RGBA8) as an egui image.
#[must_use]
pub fn color_image(image: &Image) -> ColorImage {
    ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.pixels(),
    )
}

/// Which frame of which resource a texture shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewKey {
    /// The resource's type.
    pub ty: ResType,
    /// The resource's ID.
    pub id: i16,
    /// The frame, from 0.
    pub frame: usize,
}

/// The texture of the frame the browser previews: kept while the frame is
/// the same, replaced when it changes. Dropping a texture's handle frees it
/// through egui's next texture delta.
#[derive(Default)]
pub struct PreviewTextures {
    current: Option<(PreviewKey, TextureHandle)>,
}

impl PreviewTextures {
    /// The texture of `key`'s frame `image`, loaded into `ctx` unless it is
    /// the one already loaded. Sprites are pixel art, so it is sampled
    /// nearest-neighbour.
    pub fn texture_for(&mut self, ctx: &Context, key: PreviewKey, image: &Image) -> &TextureHandle {
        if self
            .current
            .as_ref()
            .is_none_or(|(loaded, _)| *loaded != key)
        {
            let name = format!("{} {} frame {}", type_code(key.ty), key.id, key.frame);
            let texture = ctx.load_texture(name, color_image(image), TextureOptions::NEAREST);
            self.current = Some((key, texture));
        }
        &self.current.as_ref().expect("just loaded").1
    }

    /// Frees the texture, if there is one.
    pub fn release(&mut self) {
        self.current = None;
    }
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Context, RawInput, TextureId};

    use super::*;

    const PICT: ResType = ResType::new(*b"PICT");

    fn input() -> RawInput {
        RawInput {
            max_texture_side: Some(8192),
            ..RawInput::default()
        }
    }

    fn image(width: u32, height: u32, rgba: [u8; 4]) -> Image {
        Image::from_rgba(width, height, rgba.repeat((width * height) as usize)).expect("sized")
    }

    #[test]
    fn a_color_image_has_the_images_size_and_straight_alpha_pixels() {
        let pixels = vec![255, 0, 0, 255, 10, 20, 30, 128];
        let color = color_image(&Image::from_rgba(2, 1, pixels).expect("2x1"));
        assert_eq!(color.size, [2, 1]);
        assert_eq!(
            color.pixels,
            [
                Color32::from_rgba_unmultiplied(255, 0, 0, 255),
                Color32::from_rgba_unmultiplied(10, 20, 30, 128),
            ]
        );
    }

    /// Runs one frame that asks `textures` for `key`'s texture of `image`,
    /// and returns its ID and the frame's freed textures.
    fn frame(
        ctx: &Context,
        textures: &mut PreviewTextures,
        key: PreviewKey,
        image: &Image,
    ) -> (TextureId, [usize; 2], Vec<TextureId>) {
        let mut seen = None;
        let output = ctx.run_ui(input(), |ui| {
            let handle = textures.texture_for(ui.ctx(), key, image);
            seen = Some((handle.id(), handle.size()));
        });
        let freed = output.textures_delta.free.iter().copied().collect();
        output.drop_without_applying_deltas();
        let (id, size) = seen.expect("ran");
        (id, size, freed)
    }

    fn key(id: i16, frame: usize) -> PreviewKey {
        PreviewKey {
            ty: PICT,
            id,
            frame,
        }
    }

    #[test]
    fn a_frame_becomes_a_texture_of_its_size() {
        let ctx = Context::default();
        let mut textures = PreviewTextures::default();
        let (_, size, _) = frame(&ctx, &mut textures, key(128, 0), &image(4, 2, [1; 4]));
        assert_eq!(size, [4, 2]);
    }

    #[test]
    fn the_same_key_keeps_its_texture() {
        let ctx = Context::default();
        let mut textures = PreviewTextures::default();
        let picture = image(4, 2, [1; 4]);
        let (first, _, _) = frame(&ctx, &mut textures, key(128, 0), &picture);
        let allocated = ctx.tex_manager().read().num_allocated();
        let (second, _, freed) = frame(&ctx, &mut textures, key(128, 0), &picture);
        assert_eq!(second, first);
        assert_eq!(ctx.tex_manager().read().num_allocated(), allocated);
        assert_eq!(freed, []);
    }

    #[test]
    fn a_new_key_replaces_the_texture_and_frees_the_old_one() {
        let ctx = Context::default();
        let mut textures = PreviewTextures::default();
        let (first, _, _) = frame(&ctx, &mut textures, key(128, 0), &image(4, 2, [1; 4]));
        let (other_frame, size, freed) =
            frame(&ctx, &mut textures, key(128, 1), &image(3, 3, [2; 4]));
        assert_ne!(other_frame, first);
        assert_eq!(size, [3, 3]);
        assert_eq!(freed, [first]);
        let (other_resource, _, freed) =
            frame(&ctx, &mut textures, key(129, 1), &image(3, 3, [2; 4]));
        assert_ne!(other_resource, other_frame);
        assert_eq!(freed, [other_frame]);
    }

    #[test]
    fn releasing_frees_the_texture() {
        let ctx = Context::default();
        let mut textures = PreviewTextures::default();
        let (first, _, _) = frame(&ctx, &mut textures, key(128, 0), &image(4, 2, [1; 4]));
        textures.release();
        let output = ctx.run_ui(input(), |_| {});
        assert!(output.textures_delta.free.contains(&first));
        output.drop_without_applying_deltas();
    }
}
