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
use enum_dispatch::enum_dispatch;
use std::io::{self, BufRead, ErrorKind, Read, Seek};

mod format;
mod peek;

use self::format::*;
use self::peek::*;

#[enum_dispatch]
enum Format<'a, R: BufRead> {
    Uncompressed(UncompressedReader<R>),
    Gzip(GzipReader<R>),
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
        let mut source = PeekReader::new(source);
        let sniff = source.peek(6).context("sniffing input")?;
        let reader = if sniff.len() >= 2 && &sniff[0..2] == b"\x1f\x8b" {
            GzipReader::new(source).into()
        } else if sniff.len() >= 6 && &sniff[0..6] == b"\xfd7zXZ\x00" {
            XzReader::new(source).into()
        } else if sniff.len() > 4 && is_zstd_magic(sniff[0..4].try_into().unwrap()) {
            ZstdReader::new(source)?.into()
        } else {
            UncompressedReader::new(source).into()
        };
        Ok(Self {
            reader,
            allow_trailing,
        })
    }

    pub fn into_reader(self) -> impl BufRead {
        self.reader.into_inner()
    }

    fn get_peek_mut(&mut self) -> &mut PeekReader<R> {
        self.reader.get_mut()
    }

    pub fn compressed(&self) -> bool {
        !matches!(&self.reader, Format::Uncompressed(_))
    }
}

impl<R: BufRead + Seek> DecompressReader<'_, R> {
    pub fn into_read_seeker(self) -> impl BufRead + Seek {
        self.reader.into_inner()
    }
}

impl<R: BufRead> Read for DecompressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // enum_dispatch doesn't support supertraits
        // https://gitlab.com/antonok/enum_dispatch/-/issues/56
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
