//! A frame's command buffers, in submission order.
//!
//! The renderer records a frame on one wgpu encoder, but an upscaler that
//! drives the GPU through wgpu's raw (`as_hal`) API needs a command buffer
//! of its own: wgpu panics if one encoder mixes the two APIs. So a frame
//! is a run of buffers: the encoder's commands so far, the upscaler's
//! buffer, then a fresh encoder for the rest ([`FrameCommands::insert`]),
//! all submitted at once in that order.

/// Where encoders come from and what they finish into: the port behind
/// [`FrameCommands`], so its ordering is tested with no GPU.
pub trait Encoders {
    /// What commands are recorded into.
    type Encoder;
    /// A finished encoder, ready to submit.
    type Buffer;

    /// A fresh encoder.
    fn begin(&mut self) -> Self::Encoder;
    /// `encoder`, finished.
    fn finish(&mut self, encoder: Self::Encoder) -> Self::Buffer;
}

/// wgpu's encoders, on a device.
pub struct DeviceEncoders<'a>(pub &'a wgpu::Device);

impl Encoders for DeviceEncoders<'_> {
    type Encoder = wgpu::CommandEncoder;
    type Buffer = wgpu::CommandBuffer;

    fn begin(&mut self) -> wgpu::CommandEncoder {
        self.0
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    fn finish(&mut self, encoder: wgpu::CommandEncoder) -> wgpu::CommandBuffer {
        encoder.finish()
    }
}

/// One frame's commands: the encoder being recorded into, after the
/// buffers already finished, in the order to submit them.
pub struct FrameCommands<E: Encoders> {
    sink: E,
    buffers: Vec<E::Buffer>,
    current: E::Encoder,
}

/// A frame's commands on wgpu's encoders.
pub type WgpuCommands<'a> = FrameCommands<DeviceEncoders<'a>>;

impl<E: Encoders> FrameCommands<E> {
    /// A frame's commands, with a fresh encoder from `sink` to record into.
    pub fn new(mut sink: E) -> Self {
        let current = sink.begin();
        Self {
            sink,
            buffers: Vec::new(),
            current,
        }
    }

    /// The encoder to record into now.
    pub fn encoder(&mut self) -> &mut E::Encoder {
        &mut self.current
    }

    /// Puts `buffer` after everything recorded so far: the encoder is
    /// finished and queued, then `buffer`, and a fresh encoder records
    /// what comes after.
    pub fn insert(&mut self, buffer: E::Buffer) {
        let fresh = self.sink.begin();
        let done = std::mem::replace(&mut self.current, fresh);
        let done = self.sink.finish(done);
        self.buffers.push(done);
        self.buffers.push(buffer);
    }

    /// Every buffer, in the order to submit them, the encoder finished
    /// last.
    pub fn finish(mut self) -> Vec<E::Buffer> {
        let last = self.sink.finish(self.current);
        self.buffers.push(last);
        self.buffers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hands out encoders numbered from 0 and finishes each to its label,
    /// `E<n>`.
    #[derive(Default)]
    struct Numbered {
        begun: usize,
    }

    impl Encoders for Numbered {
        type Encoder = usize;
        type Buffer = String;

        fn begin(&mut self) -> Self::Encoder {
            self.begun += 1;
            self.begun - 1
        }

        fn finish(&mut self, encoder: Self::Encoder) -> Self::Buffer {
            format!("E{encoder}")
        }
    }

    /// Finishes each encoder to what was recorded in it.
    #[derive(Default)]
    struct Recorded;

    impl Encoders for Recorded {
        type Encoder = Vec<&'static str>;
        type Buffer = String;

        fn begin(&mut self) -> Self::Encoder {
            vec![]
        }

        fn finish(&mut self, encoder: Self::Encoder) -> Self::Buffer {
            encoder.join(",")
        }
    }

    #[test]
    fn an_inserted_buffer_lands_between_the_encoders_either_side() {
        let mut commands = FrameCommands::new(Recorded);
        commands.encoder().push("world");
        commands.insert("X".to_owned());
        commands.encoder().push("overlay");
        commands.encoder().push("blit");
        assert_eq!(commands.finish(), ["world", "X", "overlay,blit"]);
    }

    #[test]
    fn without_an_insert_there_is_one_buffer() {
        let mut commands = FrameCommands::new(Recorded);
        commands.encoder().push("all");
        assert_eq!(commands.finish(), ["all"]);
    }

    #[test]
    fn two_inserts_in_a_row_keep_the_empty_encoder_between_them() {
        let mut commands = FrameCommands::new(Numbered::default());
        commands.insert("X".to_owned());
        commands.insert("Y".to_owned());
        assert_eq!(commands.finish(), ["E0", "X", "E1", "Y", "E2"]);
    }

    #[test]
    fn the_encoder_after_an_insert_is_a_fresh_one() {
        let mut commands = FrameCommands::new(Numbered::default());
        assert_eq!(*commands.encoder(), 0);
        assert_eq!(*commands.encoder(), 0, "the same until an insert");
        commands.insert("X".to_owned());
        assert_eq!(*commands.encoder(), 1);
        assert_eq!(commands.finish(), ["E0", "X", "E1"]);
    }
}
