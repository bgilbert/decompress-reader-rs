// Copyright 2019 CoreOS, Inc.
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

use anyhow::{Context, Result};
use flate2::bufread::GzDecoder;
use std::io::{self, BufRead, ErrorKind, Read, Seek};

mod peek;
mod xz;
mod zstd;

use self::peek::*;
use self::xz::*;
use self::zstd::*;

enum Format<'a, R: BufRead> {
    Uncompressed(PeekReader<R>),
    Gzip(GzDecoder<PeekReader<R>>),
    Xz(XzReader<R>),
    Zstd(ZstdReader<'a, R>),
}

pub struct DecompressReader<'a, R: BufRead> {
    reader: Format<'a, R>,
    allow_trailing: bool,
}

/// Format-sniffing decompressor
impl<R: BufRead> DecompressReader<'_, R> {
    pub fn new(source: R) -> Result<Self> {
        Self::new_full(source, false)
    }

    pub fn for_concatenated(source: R) -> Result<Self> {
        Self::new_full(source, true)
    }

    fn new_full(source: R, allow_trailing: bool) -> Result<Self> {
        use Format::*;
        let mut source = PeekReader::new(source);
        let sniff = source.peek(6).context("sniffing input")?;
        let reader = if sniff.len() >= 2 && &sniff[0..2] == b"\x1f\x8b" {
            Gzip(GzDecoder::new(source))
        } else if sniff.len() >= 6 && &sniff[0..6] == b"\xfd7zXZ\x00" {
            Xz(XzReader::new(source))
        } else if sniff.len() > 4 && is_zstd_magic(sniff[0..4].try_into().unwrap()) {
            Zstd(ZstdReader::new(source)?)
        } else {
            Uncompressed(source)
        };
        Ok(Self {
            reader,
            allow_trailing,
        })
    }

    pub fn into_reader(self) -> impl BufRead {
        self.into_inner()
    }

    fn into_inner(self) -> PeekReader<R> {
        use Format::*;
        match self.reader {
            Uncompressed(d) => d,
            Gzip(d) => d.into_inner(),
            Xz(d) => d.into_inner(),
            Zstd(d) => d.into_inner(),
        }
    }

    fn get_peek_mut(&mut self) -> &mut PeekReader<R> {
        use Format::*;
        match &mut self.reader {
            Uncompressed(d) => d,
            Gzip(d) => d.get_mut(),
            Xz(d) => d.get_mut(),
            Zstd(d) => d.get_mut(),
        }
    }

    pub fn compressed(&self) -> bool {
        use Format::*;
        match &self.reader {
            Uncompressed(_) => false,
            Gzip(_) => true,
            Xz(_) => true,
            Zstd(_) => true,
        }
    }
}

impl<R: BufRead + Seek> DecompressReader<'_, R> {
    pub fn into_read_seeker(self) -> impl BufRead + Seek {
        self.into_inner()
    }
}

impl<R: BufRead> Read for DecompressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use Format::*;
        let count = match &mut self.reader {
            Uncompressed(d) => d.read(buf)?,
            Gzip(d) => d.read(buf)?,
            Xz(d) => d.read(buf)?,
            Zstd(d) => d.read(buf)?,
        };
        if count == 0 && !buf.is_empty() && self.compressed() && !self.allow_trailing {
            // Decompressors stop reading as soon as they encounter the
            // compression trailer, so they don't notice trailing data,
            // which indicates something wrong with the input.  Look for
            // one more byte, and fail if there is one.
            if !self.get_peek_mut().peek(1)?.is_empty() {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "found trailing data after compressed stream",
                ));
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    /// Test that DecompressReader fails if data is appended to the
    /// compressed stream.
    #[test]
    fn trailing_data() {
        trailing_data_one(&include_bytes!("../fixtures/1M.gz")[..]);
        trailing_data_one(&include_bytes!("../fixtures/1M.xz")[..]);
        trailing_data_one(&include_bytes!("../fixtures/1M.zst")[..]);
    }

    fn trailing_data_one(input: &[u8]) {
        let mut input = input.to_vec();
        let mut output = Vec::new();

        // successful run
        DecompressReader::new(BufReader::with_capacity(32, &*input))
            .unwrap()
            .read_to_end(&mut output)
            .unwrap();

        // drop last byte, make sure we notice
        DecompressReader::new(BufReader::with_capacity(32, &input[0..input.len() - 1]))
            .unwrap()
            .read_to_end(&mut output)
            .unwrap_err();

        // add trailing garbage, make sure we notice
        input.push(0);
        DecompressReader::new(BufReader::with_capacity(32, &*input))
            .unwrap()
            .read_to_end(&mut output)
            .unwrap_err();

        // use concatenated mode, make sure we ignore trailing garbage
        let mut reader =
            DecompressReader::for_concatenated(BufReader::with_capacity(32, &*input)).unwrap();
        reader.read_to_end(&mut output).unwrap();
        let mut remainder = Vec::new();
        reader.into_reader().read_to_end(&mut remainder).unwrap();
        assert_eq!(&remainder, &[0]);
    }
}
