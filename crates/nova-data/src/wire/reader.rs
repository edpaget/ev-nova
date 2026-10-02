//! A record reader that remembers where each read began.
//!
//! `binrw` reports running out of data as a bare I/O error with no position,
//! and seeks back to the start of the struct on failure. Records are read
//! through [`TrackingReader`] so the error report can still say where the
//! failing read started: the start of the field, or of a whole integer array
//! that `binrw` reads in one call.

use std::io::{Cursor, Read, Result, Seek, SeekFrom};

/// A `Read + Seek` view of one record's bytes that tracks read positions.
#[derive(Debug)]
pub struct TrackingReader<'a> {
    cursor: Cursor<&'a [u8]>,
    read_start: u64,
}

impl<'a> TrackingReader<'a> {
    /// Reads `data` from its start.
    #[must_use]
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
            read_start: 0,
        }
    }

    /// The current position (which may be past the end after a seek).
    #[must_use]
    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    /// Where the most recent read began.
    #[must_use]
    pub fn read_start(&self) -> u64 {
        self.read_start
    }
}

impl Read for TrackingReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_start = self.cursor.position();
        self.cursor.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.read_start = self.cursor.position();
        self.cursor.read_exact(buf)
    }
}

impl Seek for TrackingReader<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.cursor.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn records_where_each_read_began() {
        let mut reader = TrackingReader::new(b"abcdef");
        let mut two = [0; 2];
        reader.read_exact(&mut two).expect("reads");
        assert_eq!(reader.read_start(), 0);
        reader.read_exact(&mut two).expect("reads");
        assert_eq!(&two, b"cd");
        assert_eq!(reader.read_start(), 2);
        assert_eq!(reader.position(), 4);
        let mut one = [0; 1];
        assert_eq!(reader.read(&mut one).expect("reads"), 1);
        assert_eq!(&one, b"e");
        assert_eq!(reader.read_start(), 4);
        assert_eq!(reader.position(), 5);
    }

    #[test]
    fn a_failed_read_keeps_its_start() {
        let mut reader = TrackingReader::new(b"abc");
        reader.seek(SeekFrom::Start(1)).expect("seeks");
        let mut four = [0; 4];
        let err = reader.read_exact(&mut four).expect_err("too short");
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
        assert_eq!(reader.read_start(), 1);
    }

    #[test]
    fn seeking_moves_without_reading() {
        let mut reader = TrackingReader::new(b"abc");
        assert_eq!(reader.seek(SeekFrom::Current(2)).expect("seeks"), 2);
        assert_eq!(reader.read_start(), 0, "a seek is not a read");
        assert_eq!(reader.seek(SeekFrom::Start(10)).expect("seeks"), 10);
        assert_eq!(reader.position(), 10, "past the end is allowed");
        let mut one = [0; 1];
        assert_eq!(reader.read(&mut one).expect("reads"), 0);
        assert_eq!(reader.read_start(), 10);
    }
}
