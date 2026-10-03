//! Apple IMA4: IMA ADPCM in self-contained 34-byte packets.
//!
//! Each packet holds one channel's 64 samples: a big-endian header word
//! (the predictor's top 9 bits, then a 7-bit step index) and 32 bytes of
//! four-bit codes, low nibble first. Packets share no state.

/// Bytes in one packet.
pub(super) const PACKET_LEN: usize = 34;
/// Samples one packet decodes to.
pub(super) const PACKET_SAMPLES: usize = 64;

/// The largest step index.
const MAX_INDEX: usize = 88;

/// Quantizer step sizes, by step index.
#[rustfmt::skip]
const STEP: [i32; MAX_INDEX + 1] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17,
    19, 21, 23, 25, 28, 31, 34, 37, 41, 45,
    50, 55, 60, 66, 73, 80, 88, 97, 107, 118,
    130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
    337, 371, 408, 449, 494, 544, 598, 658, 724, 796,
    876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358,
    5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899,
    15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// How each code's magnitude (its low three bits) moves the step index.
const INDEX_MOVE: [isize; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// Decodes one packet to its 64 samples.
///
/// A header step index above 88 is clamped to 88, as lenient decoders do;
/// stock sounds never have one.
pub(super) fn decode_packet(packet: &[u8; PACKET_LEN]) -> [i16; PACKET_SAMPLES] {
    let header = u16::from_be_bytes([packet[0], packet[1]]);
    let mut predictor = i32::from((header & 0xFF80) as i16);
    let mut index = usize::from(header & 0x7F).min(MAX_INDEX);
    let mut samples = [0; PACKET_SAMPLES];
    let codes = packet[2..]
        .iter()
        .flat_map(|&byte| [byte & 0x0F, byte >> 4]);
    for (sample, code) in samples.iter_mut().zip(codes) {
        let step = STEP[index];
        let mut diff = step >> 3;
        if code & 1 != 0 {
            diff += step >> 2;
        }
        if code & 2 != 0 {
            diff += step >> 1;
        }
        if code & 4 != 0 {
            diff += step;
        }
        if code & 8 != 0 {
            predictor -= diff;
        } else {
            predictor += diff;
        }
        predictor = predictor.clamp(i32::from(i16::MIN), i32::from(i16::MAX));
        *sample = predictor as i16;
        index = index
            .saturating_add_signed(INDEX_MOVE[usize::from(code & 7)])
            .min(MAX_INDEX);
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::super::fixture::ima4_packet;
    use super::*;

    /// A packet with `header`, then `bytes`, then zeros.
    fn packet(header: u16, bytes: &[u8]) -> [u8; PACKET_LEN] {
        let mut packet = [0; PACKET_LEN];
        packet[..2].copy_from_slice(&header.to_be_bytes());
        packet[2..2 + bytes.len()].copy_from_slice(bytes);
        packet
    }

    #[test]
    fn codes_add_the_step_bits_they_carry() {
        // Index 0 (step 7): code 4 adds 7 and moves to index 2 (step 9);
        // code 7 adds 1 + 2 + 4 + 9 and moves to index 10 (step 19);
        // code 7 adds 34 (index 18, step 41); code 0xF subtracts 76.
        let samples = decode_packet(&packet(0x0000, &[0x74, 0xF7]));
        assert_eq!(samples[..6], [7, 23, 57, -19, -8, 2]);
    }

    #[test]
    fn each_code_bit_contributes_its_share_of_the_step() {
        // Index 88 (step 32767), predictor 0: one code then nothing.
        let first = |code: u8| decode_packet(&packet(0x0058, &[code]))[0];
        assert_eq!(first(0x0), 4095);
        assert_eq!(first(0x1), 4095 + 8191);
        assert_eq!(first(0x2), 4095 + 16383);
        assert_eq!(first(0x3), 4095 + 8191 + 16383);
        assert_eq!(first(0x4), 32767);
        assert_eq!(first(0x8), -4095);
        assert_eq!(first(0x9), -4095 - 8191);
        assert_eq!(first(0xA), -4095 - 16383);
        assert_eq!(first(0xB), -4095 - 8191 - 16383);
        assert_eq!(first(0xC), -32768);
    }

    #[test]
    fn the_predictor_clamps_at_the_top() {
        // Predictor 32640, index 88: every code 7 pushes past 32767.
        let samples = decode_packet(&packet(0x7FD8, &[0x77]));
        assert_eq!(samples[..4], [32767; 4]);
    }

    #[test]
    fn the_step_index_and_predictor_clamp_at_the_bottom_and_top() {
        // Index 127 clamps to 88, so both headers decode alike; code 8 at
        // -32768 stays at the negative clamp.
        let clamped = decode_packet(&packet(0x807F, &[0x88]));
        let at_88 = decode_packet(&packet(0x8058, &[0x88]));
        assert_eq!(clamped, at_88);
        assert_eq!(clamped[..4], [-32768, -32768, -29383, -26306]);
    }

    #[test]
    fn the_step_index_stops_at_zero() {
        // Index 0 with code 0: step 7 adds 7 >> 3 = 0 and the index stays
        // at 0, so the predictor never moves.
        assert_eq!(decode_packet(&packet(0x0100, &[])), [0x0100; 64]);
        // Afterwards code 4 still adds step 7, not a step from below 0,
        // and moves to index 2 (step 9).
        let samples = decode_packet(&packet(0x0000, &[0x00, 0x04]));
        assert_eq!(samples[..4], [0, 0, 7, 8]);
    }

    #[test]
    fn the_predictor_keeps_its_sign() {
        assert_eq!(decode_packet(&packet(0xFF80, &[]))[0], -128);
        assert_eq!(decode_packet(&packet(0x8000, &[]))[63], -32768);
        // The low 7 bits are the index (16, step 34), not the predictor.
        assert_eq!(decode_packet(&packet(0x0090, &[]))[0], 128 + 4);
    }

    #[test]
    fn index_moves_follow_the_code_magnitude() {
        // From index 0, codes 1 to 3 move the index to -1, clamped to 0,
        // so a following code 4 adds 0 + 7. Codes 4 to 7 move it by 2, 4,
        // 6 and 8, so code 4 then adds step >> 3 plus step 9, 11, 13 or 16.
        let after = |code: u8| {
            let s = decode_packet(&packet(0x0000, &[code | 0x40]));
            s[1] - s[0]
        };
        assert_eq!(after(0x1), 7);
        assert_eq!(after(0x2), 7);
        assert_eq!(after(0x3), 7);
        assert_eq!(after(0x4), 10);
        assert_eq!(after(0x5), 12);
        assert_eq!(after(0x6), 14);
        assert_eq!(after(0x7), 18);
        assert_eq!(after(0xF), 18);
    }

    #[test]
    fn a_fixture_packet_round_trips_its_header() {
        let mut codes = [0; 64];
        codes[1] = 0x4;
        let samples = decode_packet(&ima4_packet(-640, 0, codes));
        assert_eq!(samples[..3], [-640, -633, -632]);
    }

    #[test]
    fn every_sample_of_the_packet_is_decoded() {
        // Only the last code, the high nibble of the last byte, is set.
        let mut last = packet(0x0000, &[]);
        last[33] = 0x40;
        let samples = decode_packet(&last);
        assert_eq!(samples[..63], [0; 63]);
        assert_eq!(samples[63], 7);
    }
}
