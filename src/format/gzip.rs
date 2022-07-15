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

use anyhow::{Context, Result};
use flate2::bufread::GzDecoder;
use std::io::{self, BufRead, Read};

use super::FormatReader;
use crate::PeekReader;

pub(crate) struct GzipReader<R: BufRead> {
    decompressor: GzDecoder<PeekReader<R>>,
}

impl<R: BufRead> GzipReader<R> {
    pub(crate) fn detect(source: &mut PeekReader<R>) -> Result<bool> {
        Ok(source.peek(2).context("sniffing input")? == b"\x1f\x8b")
    }

    pub(crate) fn new(source: PeekReader<R>) -> Self {
        Self {
            decompressor: GzDecoder::new(source),
        }
    }
}

impl<R: BufRead> FormatReader<R> for GzipReader<R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        self.decompressor.get_mut()
    }

    fn into_inner(self) -> PeekReader<R> {
        self.decompressor.into_inner()
    }
}

impl<R: BufRead> Read for GzipReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.decompressor.read(out)
    }
}
