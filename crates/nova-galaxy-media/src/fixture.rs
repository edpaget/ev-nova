//! Builders for test models, made in code so no art enters the repo:
//! a box as a `.glb` ([`glb`], [`glb_with_materials`]) or as a `.gltf`
//! with its buffer beside it ([`gltf_with_bin`]). Ported from game-bible's
//! `galaxy` tool's `fixture.rs`.

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

/// The glTF JSON for one box mesh whose buffer is `bin_len` bytes at
/// `uri` (`None` for a GLB's own binary chunk), placed by a node at
/// `translation`. The box's triangles are split evenly into one
/// primitive per material, in order; with none, it has one primitive
/// and no material.
fn box_json(
    positions: &[[f32; 3]],
    indices: &[u16],
    uri: Option<&str>,
    bin_len: usize,
    translation: [f32; 3],
    materials: &[Material<'_>],
) -> String {
    let min = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MAX, f32::min));
    let max = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MIN, f32::max));
    let positions_len = positions.len() * 12;
    let mut buffer = serde_json::json!({ "byteLength": bin_len });
    if let Some(uri) = uri {
        buffer["uri"] = uri.into();
    }
    let parts = materials.len().max(1);
    let per = indices.len() / parts;
    let mut accessors = vec![serde_json::json!(
        { "bufferView": 0, "componentType": 5126, "count": positions.len(), "type": "VEC3",
          "min": min.collect::<Vec<_>>(), "max": max.collect::<Vec<_>>() }
    )];
    let mut primitives = Vec::new();
    for part in 0..parts {
        accessors.push(serde_json::json!(
            { "bufferView": 1, "byteOffset": part * per * 2, "componentType": 5123,
              "count": per, "type": "SCALAR" }
        ));
        let mut primitive =
            serde_json::json!({ "attributes": { "POSITION": 0 }, "indices": part + 1 });
        if !materials.is_empty() {
            primitive["material"] = part.into();
        }
        primitives.push(primitive);
    }
    let materials: Vec<_> = materials
        .iter()
        .map(|(name, color)| {
            let mut material =
                serde_json::json!({ "pbrMetallicRoughness": { "baseColorFactor": color } });
            if !name.is_empty() {
                material["name"] = (*name).into();
            }
            material
        })
        .collect();
    let mut json = serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0, "translation": translation }],
        "meshes": [{ "primitives": primitives }],
        "buffers": [buffer],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": positions_len },
            { "buffer": 0, "byteOffset": positions_len, "byteLength": indices.len() * 2 }
        ],
        "accessors": accessors
    });
    if !materials.is_empty() {
        json["materials"] = materials.into();
    }
    json.to_string()
}

/// The binary buffer for `positions` and `indices`, padded to 4 bytes.
fn box_bin(positions: &[[f32; 3]], indices: &[u16]) -> Vec<u8> {
    let mut bin: Vec<u8> = positions
        .iter()
        .flatten()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    bin.extend(indices.iter().flat_map(|i| i.to_le_bytes()));
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    bin
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
    let (positions, indices) = cuboid(size);
    let bin = box_bin(&positions, &indices);
    let mut json = box_json(&positions, &indices, None, bin.len(), at, materials).into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let total = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = b"glTF".to_vec();
    for n in [2, total] {
        glb.extend(
            u32::try_from(n)
                .expect("a test model fits in 4 GiB")
                .to_le_bytes(),
        );
    }
    glb.extend(
        u32::try_from(json.len())
            .expect("a test model fits in 4 GiB")
            .to_le_bytes(),
    );
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend(
        u32::try_from(bin.len())
            .expect("a test model fits in 4 GiB")
            .to_le_bytes(),
    );
    glb.extend(b"BIN\0");
    glb.extend(bin);
    glb
}

/// A `.gltf` JSON file holding a box `size` big at the origin, in one
/// white material named `hull`, with its buffer in a separate file at
/// `bin_uri`, and that buffer's bytes.
#[must_use]
pub fn gltf_with_bin(size: [f32; 3], bin_uri: &str) -> (String, Vec<u8>) {
    let (positions, indices) = cuboid(size);
    let bin = box_bin(&positions, &indices);
    let json = box_json(
        &positions,
        &indices,
        Some(bin_uri),
        bin.len(),
        [0.0; 3],
        &[("hull", [1.0; 4])],
    );
    (json, bin)
}

#[cfg(test)]
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
}
