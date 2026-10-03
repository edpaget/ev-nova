//! Plain data the adapter hands to the GPU: vertex and instance layouts,
//! the projection uniform, surface format choice and read-back rows.

use bytemuck::{Pod, Zeroable};

use crate::gpu::{QuadInstance, SolidQuad};
use crate::viewport::LogicalSize;

/// One sprite quad as the sprite shader reads it, per instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct SpriteInstance {
    /// Destination x, y, width, height in logical units.
    pub dest: [f32; 4],
    /// Texture coordinates u0, v0, u1, v1.
    pub uv: [f32; 4],
    /// RGBA multiplier.
    pub tint: [f32; 4],
}

impl From<&QuadInstance> for SpriteInstance {
    fn from(quad: &QuadInstance) -> Self {
        Self {
            dest: [quad.dest.x, quad.dest.y, quad.dest.w, quad.dest.h],
            uv: [quad.uv.u0, quad.uv.v0, quad.uv.u1, quad.uv.v1],
            tint: quad.tint,
        }
    }
}

/// One vertex of a solid (untextured) triangle.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(crate) struct SolidVertex {
    /// Position in logical units.
    pub position: [f32; 2],
    /// RGBA.
    pub color: [f32; 4],
}

/// A solid quad as two triangles: corners 0, 1, 2 and 0, 2, 3.
pub(crate) fn solid_vertices(quad: &SolidQuad) -> [SolidVertex; 6] {
    [0, 1, 2, 0, 2, 3].map(|i| SolidVertex {
        position: [quad.corners[i].x, quad.corners[i].y],
        color: quad.color,
    })
}

/// The projection uniform: the logical size, padded to 16 bytes.
pub(crate) fn globals(logical: LogicalSize) -> [f32; 4] {
    [logical.w as f32, logical.h as f32, 0.0, 0.0]
}

/// Bytes per row of an RGBA8 read-back buffer `width` pixels wide: wgpu
/// copies rows at multiples of 256 bytes.
pub(crate) fn padded_bytes_per_row(width: u32) -> u32 {
    (width * 4).div_ceil(256) * 256
}

/// The RGBA8 pixels of `width`-pixel rows stored `padded` bytes apart.
pub(crate) fn strip_padding(data: &[u8], width: u32, padded: u32) -> Vec<u8> {
    let row = width as usize * 4;
    data.chunks(padded as usize)
        .flat_map(|line| &line[..row])
        .copied()
        .collect()
}

/// The first format a surface offers that is not sRGB, so colours blend in
/// gamma space and decoded bytes reach the screen unchanged.
pub(crate) fn surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats.iter().copied().find(|format| !format.is_srgb())
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::Point;

    use super::*;
    use crate::gpu::{QuadInstance, Rect, SolidQuad};
    use crate::{LogicalSize, Uv};

    #[test]
    fn a_quad_becomes_an_instance_with_its_fields_in_order() {
        let quad = QuadInstance {
            dest: Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            },
            uv: Uv {
                u0: 0.5,
                v0: 0.25,
                u1: 0.75,
                v1: 1.0,
            },
            tint: [0.125, 0.25, 0.375, 0.5],
        };
        let instance = SpriteInstance::from(&quad);
        assert_eq!(
            instance,
            SpriteInstance {
                dest: [1.0, 2.0, 3.0, 4.0],
                uv: [0.5, 0.25, 0.75, 1.0],
                tint: [0.125, 0.25, 0.375, 0.5],
            }
        );
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&instance));
        assert_eq!(
            floats,
            [
                1.0, 2.0, 3.0, 4.0, 0.5, 0.25, 0.75, 1.0, 0.125, 0.25, 0.375, 0.5
            ]
        );
    }

    #[test]
    fn a_solid_quad_becomes_two_triangles() {
        let p = |x, y| Point::new(x, y);
        let quad = SolidQuad {
            corners: [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.0), p(0.0, 2.0)],
            color: [1.0, 0.5, 0.25, 0.125],
        };
        let vertices = solid_vertices(&quad);
        let positions: Vec<[f32; 2]> = vertices.iter().map(|v| v.position).collect();
        assert_eq!(
            positions,
            [
                [0.0, 0.0],
                [4.0, 0.0],
                [4.0, 2.0],
                [0.0, 0.0],
                [4.0, 2.0],
                [0.0, 2.0]
            ]
        );
        assert!(vertices.iter().all(|v| v.color == quad.color));
        let floats: &[f32] = bytemuck::cast_slice(&vertices[..1]);
        assert_eq!(floats, [0.0, 0.0, 1.0, 0.5, 0.25, 0.125]);
    }

    #[test]
    fn the_projection_uniform_is_the_logical_size() {
        assert_eq!(
            globals(LogicalSize { w: 1024, h: 768 }),
            [1024.0, 768.0, 0.0, 0.0]
        );
    }

    #[test]
    fn read_back_rows_are_padded_to_256_bytes() {
        assert_eq!(padded_bytes_per_row(1), 256);
        assert_eq!(padded_bytes_per_row(64), 256);
        assert_eq!(padded_bytes_per_row(65), 512);
        assert_eq!(padded_bytes_per_row(128), 512);
    }

    #[test]
    fn stripping_padding_keeps_each_rows_pixels() {
        // Two rows of a 2-pixel-wide image, each padded to 12 bytes.
        let padded = [
            1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, //
            9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0,
        ];
        assert_eq!(strip_padding(&padded, 2, 12), (1..=16).collect::<Vec<u8>>());
    }

    #[test]
    fn the_surface_format_is_the_first_non_srgb_one() {
        use ::wgpu::TextureFormat as F;
        assert_eq!(
            surface_format(&[F::Bgra8UnormSrgb, F::Rgba8UnormSrgb, F::Bgra8Unorm]),
            Some(F::Bgra8Unorm)
        );
        assert_eq!(
            surface_format(&[F::Rgba8Unorm, F::Bgra8Unorm]),
            Some(F::Rgba8Unorm)
        );
        assert_eq!(surface_format(&[F::Bgra8UnormSrgb]), None);
        assert_eq!(surface_format(&[]), None);
    }
}
