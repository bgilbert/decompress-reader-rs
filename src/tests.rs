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

use super::*;
use std::io::BufReader;

/// Test that DecompressReader fails if data is appended to the
/// compressed stream.
#[test]
fn trailing_data() {
    #[cfg(feature = "gzip")]
    trailing_data_one(&include_bytes!("../fixtures/1M.gz")[..]);
    #[cfg(feature = "xz")]
    trailing_data_one(&include_bytes!("../fixtures/1M.xz")[..]);
    #[cfg(feature = "zstd")]
    trailing_data_one(&include_bytes!("../fixtures/1M.zst")[..]);
}

#[allow(dead_code)]
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
    let mut reader = DecompressBuilder::new()
        .trailing_data(true)
        .reader(BufReader::with_capacity(32, &*input))
        .unwrap();
    reader.read_to_end(&mut output).unwrap();
    let mut remainder = Vec::new();
    reader.into_reader().read_to_end(&mut remainder).unwrap();
    assert_eq!(&remainder, &[0]);
}
