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

use std::io::BufRead;

use crate::{DecompressReader, Result};

#[derive(Clone, Debug)]
pub struct DecompressBuilder {
    pub(crate) trailing_data: bool,
    pub(crate) uncompressed: bool,

    pub(crate) gzip: bool,
    pub(crate) xz: bool,
    pub(crate) zstd: bool,
}

impl DecompressBuilder {
    pub fn new() -> Self {
        Self {
            // uncompressed disabled by default
            gzip: true,
            xz: true,
            zstd: true,
            ..Self::none()
        }
    }

    pub fn none() -> Self {
        Self {
            trailing_data: false,
            uncompressed: false,

            gzip: false,
            xz: false,
            zstd: false,
        }
    }

    pub fn reader<'a, R: BufRead>(&self, source: R) -> Result<DecompressReader<'a, R>> {
        DecompressReader::new_full(source, self.clone())
    }

    pub fn trailing_data(&mut self, enable: bool) -> &mut Self {
        self.trailing_data = enable;
        self
    }

    pub fn uncompressed(&mut self, enable: bool) -> &mut Self {
        self.uncompressed = enable;
        self
    }

    pub fn gzip(&mut self, enable: bool) -> &mut Self {
        self.gzip = enable;
        self
    }

    pub fn xz(&mut self, enable: bool) -> &mut Self {
        self.xz = enable;
        self
    }

    pub fn zstd(&mut self, enable: bool) -> &mut Self {
        self.zstd = enable;
        self
    }
}

impl Default for DecompressBuilder {
    fn default() -> Self {
        Self::new()
    }
}
