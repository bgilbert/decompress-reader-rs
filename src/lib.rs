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

use enum_dispatch::enum_dispatch;
use std::io::{self, BufRead, ErrorKind, Read, Seek};

mod config;
mod error;
mod format;
mod peek;
#[cfg(test)]
mod tests;

pub use self::config::*;
pub use self::error::*;

use self::format::*;
use self::peek::*;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionFormat {
    Uncompressed,
    #[cfg(feature = "gzip")]
    Gzip,
    #[cfg(feature = "xz")]
    Xz,
    #[cfg(feature = "zstd")]
    Zstd,
}

#[enum_dispatch]
#[derive(Debug)]
enum Format<'a, R: BufRead> {
    Uncompressed(UncompressedReader<'a, R>),
    #[cfg(feature = "gzip")]
    Gzip(GzipReader<R>),
    #[cfg(feature = "xz")]
    Xz(XzReader<R>),
    #[cfg(feature = "zstd")]
    Zstd(ZstdReader<'a, R>),
}

#[derive(Debug)]
pub struct DecompressReader<'a, R: BufRead> {
    config: DecompressBuilder,
    reader: Format<'a, R>,
}

/// Format-sniffing decompressor
impl<'a, R: BufRead> DecompressReader<'a, R> {
    pub fn new(source: R) -> Result<Self> {
        Self::new_full(source, DecompressBuilder::new())
    }

    fn new_full(source: R, config: DecompressBuilder) -> Result<Self> {
        Ok(Self {
            reader: Self::get_reader(source, &config)?,
            config,
        })
    }

    fn get_reader(source: R, config: &DecompressBuilder) -> Result<Format<'a, R>> {
        #[allow(unused_mut)]
        let mut source = PeekReader::new(source);

        #[cfg(feature = "gzip")]
        if config.gzip && GzipReader::detect(&mut source)? {
            return Ok(GzipReader::new(source).into());
        }

        #[cfg(feature = "xz")]
        if config.xz && XzReader::detect(&mut source)? {
            return Ok(XzReader::new(source).into());
        }

        #[cfg(feature = "zstd")]
        if config.zstd && ZstdReader::detect(&mut source)? {
            return Ok(ZstdReader::new(source)?.into());
        }

        if config.uncompressed {
            return Ok(UncompressedReader::new(source).into());
        }

        Err(DecompressError::UnrecognizedFormat)
    }

    pub fn into_reader(self) -> impl BufRead {
        self.reader.into_inner()
    }

    fn get_peek_mut(&mut self) -> &mut PeekReader<R> {
        self.reader.get_mut()
    }

    pub fn format(&self) -> CompressionFormat {
        self.reader.as_primitive()
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
            #[cfg(feature = "gzip")]
            Gzip(d) => d.read(buf)?,
            #[cfg(feature = "xz")]
            Xz(d) => d.read(buf)?,
            #[cfg(feature = "zstd")]
            Zstd(d) => d.read(buf)?,
        };
        if count == 0
            && !buf.is_empty()
            && self.format() != CompressionFormat::Uncompressed
            && !self.config.trailing_data
        {
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

impl<R: BufRead> Format<'_, R> {
    fn as_primitive(&self) -> CompressionFormat {
        use CompressionFormat::*;
        match self {
            Self::Uncompressed(_) => Uncompressed,
            #[cfg(feature = "gzip")]
            Self::Gzip(_) => Gzip,
            #[cfg(feature = "xz")]
            Self::Xz(_) => Xz,
            #[cfg(feature = "zstd")]
            Self::Zstd(_) => Zstd,
        }
    }
}
