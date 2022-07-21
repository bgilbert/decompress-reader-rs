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

    #[cfg(feature = "bzip2")]
    pub(crate) bzip2: bool,
    #[cfg(feature = "gzip")]
    pub(crate) gzip: bool,
    #[cfg(feature = "xz")]
    pub(crate) xz: bool,
    #[cfg(feature = "zstd")]
    pub(crate) zstd: bool,
}

impl DecompressBuilder {
    pub fn new() -> Self {
        Self {
            // uncompressed disabled by default
            #[cfg(feature = "bzip2")]
            bzip2: true,
            #[cfg(feature = "gzip")]
            gzip: true,
            #[cfg(feature = "xz")]
            xz: true,
            #[cfg(feature = "zstd")]
            zstd: true,
            ..Self::none()
        }
    }

    pub fn none() -> Self {
        Self {
            trailing_data: false,
            uncompressed: false,

            #[cfg(feature = "bzip2")]
            bzip2: false,
            #[cfg(feature = "gzip")]
            gzip: false,
            #[cfg(feature = "xz")]
            xz: false,
            #[cfg(feature = "zstd")]
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

    #[cfg(feature = "bzip2")]
    pub fn bzip2(&mut self, enable: bool) -> &mut Self {
        self.bzip2 = enable;
        self
    }

    #[cfg(feature = "gzip")]
    pub fn gzip(&mut self, enable: bool) -> &mut Self {
        self.gzip = enable;
        self
    }

    #[cfg(feature = "xz")]
    pub fn xz(&mut self, enable: bool) -> &mut Self {
        self.xz = enable;
        self
    }

    #[cfg(feature = "zstd")]
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
