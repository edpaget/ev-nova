//! Builders for test models, made in code so no art enters the repo:
//! a box as a `.glb` ([`glb`], [`glb_with_materials`]) or as a `.gltf`
//! with its buffer beside it ([`gltf_with_bin`]). Ported from game-bible's
//! `galaxy` tool's `fixture.rs`.
//!
//! [`Builder`] makes the same box with more of glTF in it, for the 3D
//! reader ([`scene`](crate::scene)): normals, texture coordinates,
//! materials with metallic, roughness, unlit and a texture ([`Mat`]), a
//! tree of nodes ([`Node`]), an image embedded or beside it ([`ImageAt`]),
//! and a primitive that isn't triangles.
//!
//! [`png`] encodes a real PNG from pixels, for the texture reader
//! ([`texture`](crate::texture)) and the image reader
//! ([`image`](crate::image)), and [`JPEG`] is a real JPEG. A [`Builder`]'s
//! image holds the stub [`PNG`] unless it is given real bytes
//! ([`Builder::image_bytes`]).

/// A box `[x, y, z]` wide, deep and tall, centred on the origin, as six
/// faces of two triangles each, with the vertex positions and indices it
/// needs.
fn cuboid(size: [f32; 3]) -> (Vec<[f32; 3]>, Vec<u16>) {
    let [hx, hy, hz] = size.map(|s| s / 2.0);
    let mut positions = Vec::new();
    for x in [-hx, hx] {
        for y in [-hy, hy] {
            for z in [-hz, hz] {
                positions.push([x, y, z]);
            }
        }
    }
    // Corner `i` has x = i & 4, y = i & 2, z = i & 1.
    let quads = [
        [0, 1, 3, 2],
        [4, 6, 7, 5],
        [0, 4, 5, 1],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 5, 7, 3],
    ];
    let indices = quads
        .iter()
        .flat_map(|[a, b, c, d]| [*a, *b, *c, *a, *c, *d])
        .collect();
    (positions, indices)
}

/// A named glTF material and its base colour (RGBA, 0 to 1).
pub type Material<'a> = (&'a str, [f32; 4]);

/// A material a [`Builder`] writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Mat {
    /// Its name; `""` writes none.
    pub name: String,
    /// Its base colour factor, linear RGBA from 0 to 1.
    pub color: [f32; 4],
    /// Its metallic factor.
    pub metallic: f32,
    /// Its roughness factor.
    pub roughness: f32,
    /// Whether it is unlit (`KHR_materials_unlit`).
    pub unlit: bool,
    /// Whether its base colour takes the model's image (texture 0).
    pub textured: bool,
}

impl Mat {
    /// A lit, untextured material `name` of `color`, fully metallic and
    /// rough (glTF's defaults).
    #[must_use]
    pub fn new(name: &str, color: [f32; 4]) -> Self {
        Mat {
            name: name.to_owned(),
            color,
            metallic: 1.0,
            roughness: 1.0,
            unlit: false,
            textured: false,
        }
    }
}

/// A node a [`Builder`] writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Whether it draws the box.
    pub mesh: bool,
    /// Its translation.
    pub translation: [f32; 3],
    /// Its rotation, a unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    /// Its scale.
    pub scale: [f32; 3],
    /// Its children, as indices of the builder's nodes.
    pub children: Vec<usize>,
}

impl Node {
    /// A node drawing the box, moved to `translation`.
    #[must_use]
    pub fn at(translation: [f32; 3]) -> Self {
        Node {
            mesh: true,
            translation,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            children: Vec::new(),
        }
    }

    /// A node drawing nothing, holding `children`.
    #[must_use]
    pub fn group(children: &[usize]) -> Self {
        Node {
            mesh: false,
            children: children.to_vec(),
            ..Node::at([0.0; 3])
        }
    }
}

/// Where a model's one image is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageAt {
    /// In its binary buffer, through a buffer view (`image/png`).
    Embedded,
    /// In a file the model names by this `uri`.
    Uri(String),
}

/// A PNG's signature and the start of its header: the bytes a test
/// model's image holds unless it is given others
/// ([`Builder::image_bytes`]). It doesn't decode.
pub const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

/// A real baseline JPEG, [`JPEG_SIZE`] big, every pixel [`JPEG_RGB`].
///
/// Made once with Pillow 12.3 and pasted here, so no test needs an
/// encoder:
/// `Image.new("RGB", (4, 2), (200, 100, 50)).save(out, "JPEG", quality=95,
/// subsampling=0, optimize=True)`. It holds a JFIF header, two
/// quantisation tables, a baseline frame header (`SOF0`), four Huffman
/// tables and one scan.
pub const JPEG: &[u8] = &[
    0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x02,
    0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x02, 0x02, 0x04, 0x03, 0x02, 0x02, 0x02, 0x02, 0x05, 0x04,
    0x04, 0x03, 0x04, 0x06, 0x05, 0x06, 0x06, 0x06, 0x05, 0x06, 0x06, 0x06, 0x07, 0x09, 0x08, 0x06,
    0x07, 0x09, 0x07, 0x06, 0x06, 0x08, 0x0b, 0x08, 0x09, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x06, 0x08,
    0x0b, 0x0c, 0x0b, 0x0a, 0x0c, 0x09, 0x0a, 0x0a, 0x0a, 0xff, 0xdb, 0x00, 0x43, 0x01, 0x02, 0x02,
    0x02, 0x02, 0x02, 0x02, 0x05, 0x03, 0x03, 0x05, 0x0a, 0x07, 0x06, 0x07, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a,
    0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0x0a, 0xff, 0xc0,
    0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x04, 0x03, 0x01, 0x11, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11,
    0x01, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0xff, 0xc4, 0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xc4, 0x00,
    0x14, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x08, 0xff, 0xc4, 0x00, 0x14, 0x11, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00,
    0x02, 0x11, 0x03, 0x11, 0x00, 0x3f, 0x00, 0x3c, 0x57, 0x36, 0x1f, 0xff, 0xd9,
];

/// The width and height of [`JPEG`]: wider than tall, so a test sees the
/// two swapped.
pub const JPEG_SIZE: (u32, u32) = (4, 2);

/// The red, green and blue every pixel of [`JPEG`] decodes to, give or
/// take a decoder's rounding.
pub const JPEG_RGB: [u8; 3] = [200, 100, 50];

/// A real, decodable PNG `width` by `height`, 8-bit RGBA, from `rgba`
/// (four bytes a pixel, row by row): a test texture, made in code.
///
/// # Panics
///
/// When `rgba` isn't four bytes for each pixel.
#[must_use]
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut out, width, height);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .expect("a PNG header writes to memory");
    writer
        .write_image_data(rgba)
        .expect("four bytes for each pixel");
    writer.finish().expect("a PNG ends in memory");
    out
}

/// A test model: one box mesh, its triangles split evenly into one
/// primitive per material (one primitive with no material when there are
/// none), drawn by each node that says so. By default one node draws it
/// at the origin, with positions only.
#[derive(Clone, Debug, PartialEq)]
pub struct Builder {
    size: [f32; 3],
    materials: Vec<Mat>,
    nodes: Vec<Node>,
    normals: bool,
    texcoords: bool,
    lines: bool,
    image: Option<ImageAt>,
    image_bytes: Vec<u8>,
    spare: Option<Vec<u8>>,
}

impl Builder {
    /// A box `size` big (x wide, y tall, z long), 12 triangles.
    #[must_use]
    pub fn new(size: [f32; 3]) -> Self {
        Builder {
            size,
            materials: Vec::new(),
            nodes: vec![Node::at([0.0; 3])],
            normals: false,
            texcoords: false,
            lines: false,
            image: None,
            image_bytes: PNG.to_vec(),
            spare: None,
        }
    }

    /// Adds `material`, which takes the next share of the triangles.
    #[must_use]
    pub fn material(mut self, material: Mat) -> Self {
        self.materials.push(material);
        self
    }

    /// One node, drawing the box moved to `translation`.
    #[must_use]
    pub fn at(self, translation: [f32; 3]) -> Self {
        self.nodes(vec![Node::at(translation)])
    }

    /// These nodes; the scene holds each that is nobody's child.
    #[must_use]
    pub fn nodes(mut self, nodes: Vec<Node>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Writes a `NORMAL` per vertex: each corner's direction from the
    /// box's middle, a unit vector.
    #[must_use]
    pub fn normals(mut self) -> Self {
        self.normals = true;
        self
    }

    /// Writes a `TEXCOORD_0` per vertex: `[1, _]` on the +x side, `[_, 1]`
    /// on the +z side, 0 otherwise.
    #[must_use]
    pub fn texcoords(mut self) -> Self {
        self.texcoords = true;
        self
    }

    /// Also writes a `LINES` primitive over the box's indices.
    #[must_use]
    pub fn lines(mut self) -> Self {
        self.lines = true;
        self
    }

    /// Gives the model one image, at `at`, as texture 0: [`PNG`], unless
    /// [`image_bytes`](Self::image_bytes) gives it others.
    #[must_use]
    pub fn image(mut self, at: ImageAt) -> Self {
        self.image = Some(at);
        self
    }

    /// Makes the model's image ([`image`](Self::image)) hold `bytes`: a
    /// real PNG ([`png`]) or JPEG ([`JPEG`]), say. One embedded is given
    /// the MIME type of what it holds.
    #[must_use]
    pub fn image_bytes(mut self, bytes: &[u8]) -> Self {
        self.image_bytes = bytes.to_vec();
        self
    }

    /// Embeds a second image holding `bytes`, which no texture and so no
    /// material names, before the model's own: the model's own image is
    /// then image 1, still texture 0.
    #[must_use]
    pub fn spare_image(mut self, bytes: &[u8]) -> Self {
        self.spare = Some(bytes.to_vec());
        self
    }

    /// The model as a GLB.
    #[must_use]
    pub fn glb(&self) -> Vec<u8> {
        let (json, bin) = self.json_and_bin(None);
        pack_glb(json.into_bytes(), bin)
    }

    /// The model as a `.gltf` JSON file whose buffer is the file at
    /// `bin_uri`, and that buffer's bytes.
    #[must_use]
    pub fn gltf(&self, bin_uri: &str) -> (String, Vec<u8>) {
        self.json_and_bin(Some(bin_uri))
    }

    /// The glTF JSON, its buffer at `uri` (`None` for a GLB's own binary
    /// chunk), and the buffer.
    fn json_and_bin(&self, uri: Option<&str>) -> (String, Vec<u8>) {
        use serde_json::json;
        let (positions, indices) = cuboid(self.size);
        let mut bin = Vec::new();
        let mut views = Vec::new();
        let floats = |values: &mut dyn Iterator<Item = f32>| -> Vec<u8> {
            values.flat_map(f32::to_le_bytes).collect()
        };
        let position_view = push_view(
            &mut bin,
            &mut views,
            &floats(&mut positions.iter().flatten().copied()),
        );
        let index_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let index_view = push_view(&mut bin, &mut views, &index_bytes);

        let min = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MAX, f32::min));
        let max = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MIN, f32::max));
        let mut accessors = vec![json!(
            { "bufferView": position_view, "componentType": 5126, "count": positions.len(),
              "type": "VEC3", "min": min.collect::<Vec<_>>(), "max": max.collect::<Vec<_>>() }
        )];
        let mut attributes = json!({ "POSITION": 0 });
        if self.normals {
            let normals = positions.iter().flat_map(|p| {
                let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
                p.map(|c| c / length)
            });
            let view = push_view(&mut bin, &mut views, &floats(&mut normals.into_iter()));
            attributes["NORMAL"] = accessors.len().into();
            accessors.push(json!(
                { "bufferView": view, "componentType": 5126, "count": positions.len(), "type": "VEC3" }
            ));
        }
        if self.texcoords {
            let uvs = positions
                .iter()
                .flat_map(|p| [p[0], p[2]].map(|c| if c > 0.0 { 1.0 } else { 0.0 }));
            let view = push_view(&mut bin, &mut views, &floats(&mut uvs.into_iter()));
            attributes["TEXCOORD_0"] = accessors.len().into();
            accessors.push(json!(
                { "bufferView": view, "componentType": 5126, "count": positions.len(), "type": "VEC2" }
            ));
        }

        let parts = self.materials.len().max(1);
        let per = indices.len() / parts;
        let mut primitives = Vec::new();
        for part in 0..parts {
            let mut primitive = json!({ "attributes": attributes, "indices": accessors.len() });
            accessors.push(json!(
                { "bufferView": index_view, "byteOffset": part * per * 2, "componentType": 5123,
                  "count": per, "type": "SCALAR" }
            ));
            if !self.materials.is_empty() {
                primitive["material"] = part.into();
            }
            primitives.push(primitive);
        }
        if self.lines {
            primitives.push(json!(
                { "attributes": attributes, "indices": accessors.len(), "mode": 1 }
            ));
            accessors.push(json!(
                { "bufferView": index_view, "componentType": 5123, "count": indices.len(),
                  "type": "SCALAR" }
            ));
        }

        let images = self.images_json(&mut bin, &mut views);
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }

        let mut buffer = json!({ "byteLength": bin.len() });
        if let Some(uri) = uri {
            buffer["uri"] = uri.into();
        }
        let (roots, nodes) = self.nodes_json();
        let mut json = json!({
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": roots }],
            "nodes": nodes,
            "meshes": [{ "primitives": primitives }],
            "buffers": [buffer],
            "bufferViews": views,
            "accessors": accessors
        });
        if !self.materials.is_empty() {
            json["materials"] = self.materials_json().into();
        }
        if self.materials.iter().any(|m| m.unlit) {
            json["extensionsUsed"] = json!(["KHR_materials_unlit"]);
        }
        if !images.is_empty() {
            if self.image.is_some() {
                json["textures"] = json!([{ "source": images.len() - 1 }]);
            }
            json["images"] = images.into();
        }
        (json.to_string(), bin)
    }

    /// The images' JSON: the spare one, if any, then the model's own, if
    /// it has one. An embedded image is written to `bin` through a new
    /// view.
    fn images_json(
        &self,
        bin: &mut Vec<u8>,
        views: &mut Vec<serde_json::Value>,
    ) -> Vec<serde_json::Value> {
        let mut embedded = |bytes: &[u8]| {
            let view = push_view(bin, views, bytes);
            serde_json::json!({ "bufferView": view, "mimeType": mime_of(bytes) })
        };
        let mut images = Vec::new();
        if let Some(spare) = &self.spare {
            images.push(embedded(spare));
        }
        match &self.image {
            Some(ImageAt::Embedded) => images.push(embedded(&self.image_bytes)),
            Some(ImageAt::Uri(uri)) => images.push(serde_json::json!({ "uri": uri })),
            None => {}
        }
        images
    }

    /// The materials' JSON.
    fn materials_json(&self) -> Vec<serde_json::Value> {
        use serde_json::json;
        self.materials
            .iter()
            .map(|m| {
                let mut pbr = json!({
                    "baseColorFactor": m.color,
                    "metallicFactor": m.metallic,
                    "roughnessFactor": m.roughness
                });
                if m.textured {
                    pbr["baseColorTexture"] = json!({ "index": 0 });
                }
                let mut material = json!({ "pbrMetallicRoughness": pbr });
                if !m.name.is_empty() {
                    material["name"] = m.name.clone().into();
                }
                if m.unlit {
                    material["extensions"] = json!({ "KHR_materials_unlit": {} });
                }
                material
            })
            .collect()
    }

    /// The scene's root nodes (each that is nobody's child), and every
    /// node's JSON.
    fn nodes_json(&self) -> (Vec<usize>, Vec<serde_json::Value>) {
        let children: Vec<usize> = self.nodes.iter().flat_map(|n| n.children.clone()).collect();
        let roots = (0..self.nodes.len())
            .filter(|i| !children.contains(i))
            .collect();
        let nodes = self
            .nodes
            .iter()
            .map(|n| {
                let mut node = serde_json::json!({
                    "translation": n.translation, "rotation": n.rotation, "scale": n.scale
                });
                if n.mesh {
                    node["mesh"] = 0.into();
                }
                if !n.children.is_empty() {
                    node["children"] = n.children.clone().into();
                }
                node
            })
            .collect();
        (roots, nodes)
    }
}

/// The MIME type an embedded image holding `bytes` is given: a JPEG's by
/// how it starts, and a PNG's otherwise.
fn mime_of(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else {
        "image/png"
    }
}

/// Appends `bytes` to `bin` at a 4-byte boundary as a new buffer view,
/// and returns the view's index.
fn push_view(bin: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, bytes: &[u8]) -> usize {
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    views.push(serde_json::json!(
        { "buffer": 0, "byteOffset": bin.len(), "byteLength": bytes.len() }
    ));
    bin.extend(bytes);
    views.len() - 1
}

/// A GLB of `json` and `bin` (already padded to 4 bytes).
fn pack_glb(mut json: Vec<u8>, bin: Vec<u8>) -> Vec<u8> {
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let word = |n: usize| {
        u32::try_from(n)
            .expect("a test model fits in 4 GiB")
            .to_le_bytes()
    };
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = b"glTF".to_vec();
    glb.extend(word(2));
    glb.extend(word(total));
    glb.extend(word(json.len()));
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend(word(bin.len()));
    glb.extend(b"BIN\0");
    glb.extend(bin);
    glb
}

/// A GLB holding one box `size` big (x wide, y tall, z long), its centre
/// moved to `at` by its node, in one white material named `hull`.
/// 12 triangles.
#[must_use]
pub fn glb(size: [f32; 3], at: [f32; 3]) -> Vec<u8> {
    glb_with_materials(size, at, &[("hull", [1.0; 4])])
}

/// A GLB like [`glb`] whose 12 triangles are split evenly between
/// `materials`, in order (none: no material at all). A material named
/// `""` is written without a name.
#[must_use]
pub fn glb_with_materials(size: [f32; 3], at: [f32; 3], materials: &[Material<'_>]) -> Vec<u8> {
    materials
        .iter()
        .fold(Builder::new(size).at(at), |b, (name, color)| {
            b.material(Mat::new(name, *color))
        })
        .glb()
}

/// A `.gltf` JSON file holding a box `size` big at the origin, in one
/// white material named `hull`, with its buffer in a separate file at
/// `bin_uri`, and that buffer's bytes.
#[must_use]
pub fn gltf_with_bin(size: [f32; 3], bin_uri: &str) -> (String, Vec<u8>) {
    Builder::new(size)
        .material(Mat::new("hull", [1.0; 4]))
        .gltf(bin_uri)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_glb_s_header_gives_its_whole_length() {
        let glb = glb([2.0, 1.0, 4.0], [0.0; 3]);
        let total = u32::from_le_bytes(glb[8..12].try_into().unwrap());
        assert_eq!(total as usize, glb.len());
    }

    #[test]
    fn a_glb_s_chunks_are_padded_to_four_bytes() {
        for materials in [
            &[][..],
            &[("hull", [1.0; 4])],
            &[("a", [1.0; 4]), ("bb", [1.0; 4])],
        ] {
            let glb = glb_with_materials([2.0, 1.0, 4.0], [0.0; 3], materials);
            let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
            let json = word(12);
            let bin = word(20 + json);
            assert_eq!(json % 4, 0, "{materials:?}");
            assert_eq!(bin % 4, 0, "{materials:?}");
            assert_eq!(&glb[16..20], b"JSON");
            assert_eq!(&glb[24 + json..28 + json], b"BIN\0");
            assert_eq!(28 + json + bin, glb.len());
        }
    }

    #[test]
    fn a_builder_with_every_option_writes_a_valid_model() {
        let builder = Builder::new([1.0; 3])
            .normals()
            .texcoords()
            .lines()
            .material(Mat {
                unlit: true,
                textured: true,
                ..Mat::new("a", [1.0; 4])
            })
            .nodes(vec![Node::group(&[1]), Node::at([1.0, 0.0, 0.0])]);
        let glb = builder.clone().image(ImageAt::Embedded).glb();
        let parsed = gltf::Gltf::from_slice(&glb).expect("a valid GLB");
        assert_eq!(parsed.nodes().count(), 2);
        assert_eq!(parsed.images().count(), 1);
        let (json, _) = builder.image(ImageAt::Uri("skin.png".into())).gltf("a.bin");
        assert!(gltf::Gltf::from_slice(json.as_bytes()).is_ok());
    }

    /// The images of `glb`, as the scene reader hands them on.
    fn images(glb: &[u8]) -> Vec<crate::scene::Image> {
        crate::scene::read_scene("p/hull.glb", glb, |_| None)
            .expect("a model that reads")
            .images
    }

    #[test]
    fn a_builder_s_image_holds_the_bytes_it_is_given_and_the_stub_otherwise() {
        let builder = Builder::new([1.0; 3]).image(ImageAt::Embedded);
        let stub = images(&builder.glb());
        assert_eq!(stub[0].bytes, PNG);
        assert_eq!(stub[0].mime.as_deref(), Some("image/png"));

        let real = png(2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let given = images(&builder.clone().image_bytes(&real).glb());
        assert_eq!(given[0].bytes, real);
        assert_eq!(given[0].mime.as_deref(), Some("image/png"));

        let jpeg = images(&builder.image_bytes(JPEG).glb());
        assert_eq!(jpeg[0].bytes, JPEG);
        assert_eq!(jpeg[0].mime.as_deref(), Some("image/jpeg"));
    }

    #[test]
    fn a_spare_image_comes_first_and_the_material_names_the_model_s_own() {
        let real = png(2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let builder = Builder::new([1.0; 3])
            .texcoords()
            .material(Mat {
                textured: true,
                ..Mat::new("hull", [1.0; 4])
            })
            .image(ImageAt::Embedded)
            .image_bytes(&real);
        let own = |glb: &[u8]| {
            crate::scene::read_scene("p/hull.glb", glb, |_| None)
                .expect("a model that reads")
                .materials[0]
                .base_color_texture
        };
        assert_eq!(own(&builder.glb()), Some(0));
        let spared = builder.spare_image(JPEG).glb();
        let held = images(&spared);
        assert_eq!(held.len(), 2);
        assert_eq!(held[0].bytes, JPEG);
        assert_eq!(held[1].bytes, real);
        assert_eq!(own(&spared), Some(1));
        // With no image of its own, the spare is the model's only one.
        let alone = Builder::new([1.0; 3]).spare_image(PNG).glb();
        assert_eq!(images(&alone).len(), 1);
    }

    /// The `NORMAL`s and `TEXCOORD_0`s a box `size` big writes, as stored.
    fn attributes(size: [f32; 3]) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
        let glb = Builder::new(size).normals().texcoords().glb();
        let parsed = gltf::Gltf::from_slice(&glb).unwrap();
        let blob = parsed.blob.clone().unwrap();
        let primitive = parsed.meshes().next().unwrap().primitives().next().unwrap();
        let reader = primitive.reader(|_| Some(blob.as_slice()));
        let normals = reader.read_normals().unwrap().collect();
        let uvs = reader.read_tex_coords(0).unwrap().into_f32().collect();
        (normals, uvs)
    }

    #[test]
    fn a_builder_writes_unit_normals_from_the_middle_and_side_texcoords() {
        let (normals, uvs) = attributes([2.0, 0.0, 4.0]);
        // Corner 5 is (+1, 0, +2).
        let root5 = 5.0_f32.sqrt();
        let expected = [1.0 / root5, 0.0, 2.0 / root5];
        assert!(
            (0..3).all(|k| (normals[5][k] - expected[k]).abs() < 1e-6),
            "{:?}",
            normals[5]
        );
        assert_eq!(uvs[5], [1.0, 1.0]);
        assert_eq!(uvs[0], [0.0, 0.0]);
        // On neither side, a texcoord is 0.
        let (normals, uvs) = attributes([0.0, 2.0, 0.0]);
        assert_eq!(normals[2], [0.0, 1.0, 0.0]);
        assert!(uvs.iter().all(|uv| *uv == [0.0, 0.0]), "{uvs:?}");
    }
}
