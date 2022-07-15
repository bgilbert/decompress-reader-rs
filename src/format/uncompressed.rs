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

use std::io::{self, BufRead, Read};
use std::marker::PhantomData;

use super::FormatReader;
use crate::PeekReader;

pub(crate) struct UncompressedReader<'a, R: BufRead> {
    source: PeekReader<R>,
    // ZstdReader takes a lifetime argument, but can be compiled out, at
    // which point we'd get a compile error on ReaderKind.  We don't want
    // to add an unused ReaderKind variant just to avoid this, and
    // UncompressedReader is always compiled in, so add a lifetime here.
    phantom: PhantomData<&'a R>,
}

impl<R: BufRead> UncompressedReader<'_, R> {
    pub(crate) fn new(source: PeekReader<R>) -> Self {
        Self {
            source,
            phantom: PhantomData,
        }
    }
}

impl<R: BufRead> FormatReader<R> for UncompressedReader<'_, R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        &mut self.source
    }

    fn into_inner(self) -> PeekReader<R> {
        self.source
    }
}

impl<R: BufRead> Read for UncompressedReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.source.read(out)
    }
}
