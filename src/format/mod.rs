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

use enum_dispatch::enum_dispatch;
use std::io::BufRead;

use crate::PeekReader;

pub(crate) mod uncompressed;

pub(crate) use self::uncompressed::*;

#[cfg(feature = "bzip2")]
pub(crate) mod bzip2;
#[cfg(feature = "gzip")]
pub(crate) mod gzip;
#[cfg(feature = "xz")]
pub(crate) mod xz;
#[cfg(feature = "zstd")]
pub(crate) mod zstd;

#[cfg(feature = "bzip2")]
pub(crate) use self::bzip2::*;
#[cfg(feature = "gzip")]
pub(crate) use self::gzip::*;
#[cfg(feature = "xz")]
pub(crate) use self::xz::*;
#[cfg(feature = "zstd")]
pub(crate) use self::zstd::*;

#[enum_dispatch(Format<R>)]
// We'd like Read as a supertrait but enum_dispatch doesn't support it
// https://gitlab.com/antonok/enum_dispatch/-/issues/56
pub(crate) trait FormatReader<R: BufRead> {
    fn get_mut(&mut self) -> &mut PeekReader<R>;
    fn into_inner(self) -> PeekReader<R>;
}
