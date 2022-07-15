// Copyright 2021 Red Hat
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

// Implementation of an API similar to xz2::bufread::XzDecoder using
// xz2::write::XzDecoder.  We need this because bufread::XzDecoder returns
// io::ErrorKind::InvalidData if there's trailing data after an xz stream
// (which can't be disambiguated from an actual error) but write::XzDecoder
// returns Ok(0).  Return Ok(0) in this case and allow the caller to decide
// what it wants to do about trailing data.
//
// https://github.com/alexcrichton/xz2-rs/pull/86

use bytes::{Buf, BufMut, BytesMut};
use std::fmt;
use std::io::{self, BufRead, Read, Write};
use xz2::write::XzDecoder;

use crate::{FormatReader, PeekReader, Result};

pub(crate) struct XzReader<R: BufRead> {
    source: PeekReader<R>,
    decompressor: XzDecoder<bytes::buf::Writer<BytesMut>>,
}

impl<R: BufRead + fmt::Debug> fmt::Debug for XzReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XzReader")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl<R: BufRead> XzReader<R> {
    pub(crate) fn detect(source: &mut PeekReader<R>) -> Result<bool> {
        Ok(source.peek(6)? == b"\xfd7zXZ\x00")
    }

    pub(crate) fn new(source: PeekReader<R>) -> Self {
        Self {
            source,
            decompressor: XzDecoder::new(BytesMut::new().writer()),
        }
    }
}

impl<R: BufRead> FormatReader<R> for XzReader<R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        &mut self.source
    }

    fn into_inner(self) -> PeekReader<R> {
        self.source
    }
}

impl<R: BufRead> Read for XzReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            let buf = self.decompressor.get_mut().get_mut();
            if !buf.is_empty() {
                let count = buf.len().min(out.len());
                buf.copy_to_slice(&mut out[..count]);
                return Ok(count);
            }
            let in_ = self.source.fill_buf()?;
            if in_.is_empty() {
                // EOF
                self.decompressor.finish()?;
                return Ok(0);
            }
            let count = self.decompressor.write(in_)?;
            if count == 0 {
                // end of compressed data
                return Ok(0);
            }
            self.source.consume(count);
            // decompressor normally wouldn't fill buf until the next
            // write call
            self.decompressor.flush()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    #[test]
    fn small_decode() {
        small_decode_one(
            include_bytes!("../../fixtures/1M.gz"),
            XzReader::new(small_decode_one_make(include_bytes!(
                "../../fixtures/1M.xz"
            ))),
        );
    }
}
