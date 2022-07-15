// Copyright 2022 Red Hat
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Implementation of an API similar to zstd::stream::read::Decoder using
// zstd::stream::raw::Decoder.  We need this because read::Decoder returns
// io::ErrorKind::Other if there's trailing data after a zstd stream, which
// can't be disambiguated from an actual error.  By using the low-level API,
// we can check zstd::stream::raw::Status.remaining to see whether the
// decoder thinks it's at the end of a frame, check the upcoming bytes for
// the magic number of another frame, and decide whether we're done.  The
// raw decoder always stops at frame boundaries, so this is reliable.  If
// done, return Ok(0) and allow the caller to decide what it wants to do
// about trailing data.

use bytes::{Buf, BytesMut};
use std::fmt;
use std::io::{self, BufRead, Error, ErrorKind, Read};
use zstd::stream::raw::{Decoder, Operation};
use zstd::zstd_safe::{MAGICNUMBER, MAGIC_SKIPPABLE_MASK, MAGIC_SKIPPABLE_START};

use crate::{FormatReader, PeekReader, Result};

pub(crate) struct ZstdReader<'a, R: BufRead> {
    source: PeekReader<R>,
    buf: BytesMut,
    decoder: Decoder<'a>,
    start_of_frame: bool,
}

impl<'a, R: BufRead + fmt::Debug> fmt::Debug for ZstdReader<'a, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZstdReader")
            .field("source", &self.source)
            .field("buf", &self.buf)
            .field("start_of_frame", &self.start_of_frame)
            .finish_non_exhaustive()
    }
}

impl<R: BufRead> ZstdReader<'_, R> {
    pub(crate) fn detect(source: &mut PeekReader<R>) -> Result<bool> {
        let sniff = source.peek(4)?;
        Ok(sniff.len() == 4 && is_magic(sniff.try_into().unwrap()))
    }

    pub(crate) fn new(source: PeekReader<R>) -> Result<Self> {
        Ok(Self {
            source,
            buf: BytesMut::new(),
            decoder: Decoder::new()?,
            start_of_frame: true,
        })
    }
}

impl<R: BufRead> FormatReader<R> for ZstdReader<'_, R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        &mut self.source
    }

    fn into_inner(self) -> PeekReader<R> {
        self.source
    }
}

impl<R: BufRead> Read for ZstdReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            if !self.buf.is_empty() {
                let count = self.buf.len().min(out.len());
                self.buf.copy_to_slice(&mut out[..count]);
                return Ok(count);
            }
            if self.start_of_frame {
                let peek = self.source.peek(4)?;
                if peek.len() < 4 || !is_magic(peek[0..4].try_into().unwrap()) {
                    // end of compressed data
                    return Ok(0);
                }
                self.start_of_frame = false;
            }
            let in_ = self.source.fill_buf()?;
            if in_.is_empty() {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "premature EOF reading zstd frame",
                ));
            }
            // unfortunately we have to initialize to 0 for safety
            // BUFFER_SIZE is very large; use a smaller buffer to avoid
            // unneeded reinitialization
            self.buf.resize(16384, 0);
            let status = self.decoder.run_on_buffers(in_, &mut self.buf)?;
            self.source.consume(status.bytes_read);
            self.buf.truncate(status.bytes_written);
            if status.remaining == 0 {
                self.start_of_frame = true;
            }
        }
    }
}

fn is_magic(buf: [u8; 4]) -> bool {
    let val = u32::from_le_bytes(buf);
    val == MAGICNUMBER || val & MAGIC_SKIPPABLE_MASK == MAGIC_SKIPPABLE_START
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn small_decode() {
        small_decode_one(
            include_bytes!("../../fixtures/large.gz"),
            ZstdReader::new(small_decode_one_make(include_bytes!(
                "../../fixtures/large.zst"
            )))
            .unwrap(),
        );
    }
}
