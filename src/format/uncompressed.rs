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

use super::FormatReader;
use crate::PeekReader;

pub(crate) struct UncompressedReader<R: BufRead> {
    source: PeekReader<R>,
}

impl<R: BufRead> UncompressedReader<R> {
    pub(crate) fn new(source: PeekReader<R>) -> Self {
        Self { source }
    }
}

impl<R: BufRead> FormatReader<R> for UncompressedReader<R> {
    fn get_mut(&mut self) -> &mut PeekReader<R> {
        &mut self.source
    }

    fn into_inner(self) -> PeekReader<R> {
        self.source
    }
}

impl<R: BufRead> Read for UncompressedReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.source.read(out)
    }
}
