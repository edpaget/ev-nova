//! A bounds-checked big-endian cursor over a resource's bytes.
//!
//! Every read either advances past what it read or fails with
//! [`GraphicsError::UnexpectedEnd`] naming the offset where the read began,
//! so decoding loops always make progress and errors always have a position.

use super::GraphicsError;

/// A cursor over one resource's bytes.
#[derive(Clone, Debug)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Reads `data` from its start.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// The offset of the next read.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// The next `n` bytes.
    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8], GraphicsError> {
        let end = self.end_of(n)?;
        let bytes = self.data.get(self.pos..end).ok_or(self.end())?;
        self.pos = end;
        Ok(bytes)
    }

    /// Skips `n` bytes.
    pub(crate) fn skip(&mut self, n: usize) -> Result<(), GraphicsError> {
        self.pos = self.end_of(n)?;
        Ok(())
    }

    /// Skips a pad byte if the position is odd.
    pub(crate) fn align2(&mut self) -> Result<(), GraphicsError> {
        self.skip(self.pos % 2)
    }

    /// Moves to `offset`, which may be the end of the data but not past it.
    pub(crate) fn seek_to(&mut self, offset: usize) -> Result<(), GraphicsError> {
        if offset > self.data.len() {
            return Err(GraphicsError::UnexpectedEnd { offset });
        }
        self.pos = offset;
        Ok(())
    }

    pub(crate) fn u8(&mut self) -> Result<u8, GraphicsError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, GraphicsError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn i16(&mut self) -> Result<i16, GraphicsError> {
        Ok(i16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, GraphicsError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], GraphicsError> {
        let bytes = self.bytes(N)?;
        bytes.try_into().map_err(|_| self.end())
    }

    /// The end offset of an `n`-byte read from here, if it fits.
    fn end_of(&self, n: usize) -> Result<usize, GraphicsError> {
        self.pos
            .checked_add(n)
            .filter(|&end| end <= self.data.len())
            .ok_or(self.end())
    }

    fn end(&self) -> GraphicsError {
        GraphicsError::UnexpectedEnd { offset: self.pos }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [u8; 7] = [0x01, 0x02, 0xFF, 0xFE, 0x12, 0x34, 0x56];

    #[test]
    fn reads_big_endian_integers_and_advances() {
        let mut r = Reader::new(&DATA);
        assert_eq!(r.u8(), Ok(0x01));
        assert_eq!(r.pos(), 1);
        assert_eq!(r.u16(), Ok(0x02FF));
        assert_eq!(r.pos(), 3);
        assert_eq!(r.u32(), Ok(0xFE12_3456));
        assert_eq!(r.pos(), 7);
        let mut r = Reader::new(&DATA[2..]);
        assert_eq!(r.i16(), Ok(-2));
        assert_eq!(r.pos(), 2);
    }

    #[test]
    fn bytes_returns_the_slice_and_advances() {
        let mut r = Reader::new(&DATA);
        r.skip(2).unwrap();
        assert_eq!(r.bytes(3), Ok(&DATA[2..5]));
        assert_eq!(r.pos(), 5);
        assert_eq!(r.bytes(0), Ok(&[][..]));
        assert_eq!(r.pos(), 5);
    }

    #[test]
    fn running_out_names_where_the_read_began_and_does_not_advance() {
        fn end<T>(offset: usize) -> Result<T, GraphicsError> {
            Err(GraphicsError::UnexpectedEnd { offset })
        }
        let mut r = Reader::new(&DATA);
        r.skip(6).unwrap();
        assert_eq!(r.u16(), end(6));
        assert_eq!(r.u32(), end(6));
        assert_eq!(r.i16(), end(6));
        assert_eq!(r.bytes(2), end(6));
        assert_eq!(r.skip(2), end(6));
        assert_eq!(r.pos(), 6);
        assert_eq!(r.u8(), Ok(0x56));
        assert_eq!(r.u8(), end(7));
        assert_eq!(r.skip(usize::MAX), end(7));
        assert_eq!(r.skip(0), Ok(()));
    }

    #[test]
    fn align2_skips_one_byte_only_from_an_odd_offset() {
        let mut r = Reader::new(&DATA);
        r.align2().unwrap();
        assert_eq!(r.pos(), 0);
        r.skip(1).unwrap();
        r.align2().unwrap();
        assert_eq!(r.pos(), 2);
        r.skip(5).unwrap();
        assert_eq!(r.align2(), Err(GraphicsError::UnexpectedEnd { offset: 7 }));
    }

    #[test]
    fn seek_to_allows_the_end_but_not_past_it() {
        let mut r = Reader::new(&DATA);
        assert_eq!(r.seek_to(4), Ok(()));
        assert_eq!(r.u8(), Ok(0x12));
        assert_eq!(r.seek_to(7), Ok(()));
        assert_eq!(r.pos(), 7);
        assert_eq!(
            r.seek_to(8),
            Err(GraphicsError::UnexpectedEnd { offset: 8 })
        );
        assert_eq!(r.pos(), 7);
        assert_eq!(r.seek_to(0), Ok(()));
        assert_eq!(r.pos(), 0);
    }
}
