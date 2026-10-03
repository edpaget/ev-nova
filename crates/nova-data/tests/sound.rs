//! Wiring: sounds read from a synthetic fork, decoded through the public API
//! only.

use nova_data::sound::decode_snd;
use nova_data::sound::fixture::{Header, SndBuilder, SndFormat, ima4_packet};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{ResType, ResourceFile};

const SND: ResType = ResType::new(*b"snd ");

/// A fork holding a format 1 IMA4 sound (128) and a format 2 standard
/// sound (129).
fn fork() -> ResourceFile {
    let ima4 = SndBuilder::new(
        SndFormat::One,
        Header::Compressed {
            channels: 1,
            rate: 0x5622_0000,
            format: *b"ima4",
            compression_id: -1,
            packets: vec![ima4_packet(0, 0, [4; 64]); 3],
        },
    );
    let standard = SndBuilder::new(
        SndFormat::Two,
        Header::Standard {
            rate: 0x2B77_45D1,
            loop_points: (2, 5),
            base_note: 60,
            samples: vec![0x80, 0x90, 0xA0, 0xB0, 0xC0],
        },
    );
    let fork = ForkBuilder::new()
        .resource(SND, 128, Some(b"zap"), &ima4.bytes())
        .resource(SND, 129, None, &standard.bytes())
        .build();
    ResourceFile::from_bytes(fork.bytes).expect("valid fork")
}

#[test]
fn every_snd_in_a_fork_decodes_with_its_header_rate_and_length() {
    let file = fork();
    let decoded: Vec<_> = file
        .resources(SND)
        .map(|res| (res.id(), decode_snd(res.data()).expect("decodes")))
        .collect();
    assert_eq!(decoded.len(), 2);

    let (id, ima4) = &decoded[0];
    assert_eq!(*id, 128);
    assert_eq!(ima4.sample_rate().fixed(), 0x5622_0000);
    assert_eq!(ima4.sample_rate().nearest_hz(), 22_050);
    assert_eq!(ima4.channels(), 1);
    assert_eq!(ima4.samples().len(), 3 * 64);
    // Code 4 adds a growing step each sample until the top clamp.
    assert_eq!(ima4.samples()[..2], [7, 17]);
    assert!(ima4.samples()[..64].windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(ima4.samples()[63], 32767);

    let (id, standard) = &decoded[1];
    assert_eq!(*id, 129);
    assert_eq!(standard.sample_rate().fixed(), 0x2B77_45D1);
    assert_eq!(standard.sample_rate().nearest_hz(), 11_127);
    assert_eq!(standard.channels(), 1);
    assert_eq!(standard.samples(), [0, 4096, 8192, 12288, 16384]);
    assert_eq!(standard.loop_points(), Some((2, 5)));
}
