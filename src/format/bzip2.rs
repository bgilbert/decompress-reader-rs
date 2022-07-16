// Copyright 2022 Red Hat, Inc.
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

use bzip2::bufread::BzDecoder;
use std::fmt;
use std::io::{self, BufRead, Read};

use crate::{FormatReader, PeekReader, Result};

pub(crate) struct Bzip2Reader<R: BufRead> {
    // needs to be Option so we can replace the decoder
    decompressor: Option<BzDecoder<PeekReader<R>>>,
}

impl<R: BufRead + fmt::Debug> fmt::Debug for Bzip2Reader<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bzip2Reader").finish_non_exhaustive()
    }
}

impl<R: BufRead> Bzip2Reader<R> {
    pub(crate) fn detect(source: &mut PeekReader<R>) -> Result<bool> {
        Ok(has_magic(source)?)
    }

    pub(crate) fn new(source: PeekReader<R>) -> Self {
        Self {
            decompressor: Some(BzDecoder::new(source)),
        }
    }
}

impl<R: BufRead> FormatReader<R> for Bzip2Reader<R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        self.decompressor.as_mut().unwrap().get_mut()
    }

    fn into_inner(self) -> PeekReader<R> {
        self.decompressor.unwrap().into_inner()
    }
}

impl<R: BufRead> Read for Bzip2Reader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = self.decompressor.as_mut().unwrap().read(out)?;
        if count == 0 && has_magic(self.get_mut())? {
            // We reached the end of the stream, but there's another one.
            // Recreate the decompressor and try again.
            self.decompressor = Some(BzDecoder::new(
                self.decompressor.take().unwrap().into_inner(),
            ));
            self.read(out)
        } else {
            Ok(count)
        }
    }
}

fn has_magic<R: BufRead>(source: &mut PeekReader<R>) -> io::Result<bool> {
    let peek = source.peek(4)?;
    Ok(peek.len() == 4 && &peek[0..3] == b"BZh" && peek[3] >= b'1' && peek[3] <= b'9')
}
