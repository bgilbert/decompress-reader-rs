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

// lots of helper functions unused with --no-default-features
#![allow(dead_code)]

use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;
use std::io::{BufReader, Cursor, Read};

use crate::*;

lazy_static! {
    static ref GZIP_FIXTURES: HashMap<&'static str, &'static [u8]> = hashmap! {
        "text" => &include_bytes!("../fixtures/text.gz")[..],
        "random" => &include_bytes!("../fixtures/random.gz")[..],
        "large" => &include_bytes!("../fixtures/large.gz")[..],
    };
    static ref XZ_FIXTURES: HashMap<&'static str, &'static [u8]> = hashmap! {
        "text" => &include_bytes!("../fixtures/text.xz")[..],
        "random" => &include_bytes!("../fixtures/random.xz")[..],
        "large" => &include_bytes!("../fixtures/large.xz")[..],
    };
    static ref ZSTD_FIXTURES: HashMap<&'static str, &'static [u8]> = hashmap! {
        "text" => &include_bytes!("../fixtures/text.zst")[..],
        "random" => &include_bytes!("../fixtures/random.zst")[..],
        "large" => &include_bytes!("../fixtures/large.zst")[..],
    };
}

/// Fail compilation unless we've added a test for every format
fn check_format_coverage<'a, R: BufRead>(format: Format<'a, R>) {
    use Format::*;
    match format {
        Uncompressed(_) => (),
        #[cfg(feature = "gzip")]
        Gzip(_) => (),
        #[cfg(feature = "xz")]
        Xz(_) => (),
        #[cfg(feature = "zstd")]
        Zstd(_) => (),
    }
}

#[test]
fn uncompressed() {
    for (name, data) in &*GZIP_FIXTURES {
        let uncompressed = gunzip(data);
        let mut output = Vec::new();
        println!("=== {name} ===");
        DecompressBuilder::new()
            .uncompressed(true)
            .reader(BufReader::with_capacity(32, &*uncompressed))
            .unwrap()
            .read_to_end(&mut output)
            .unwrap();
        assert_eq!(output, uncompressed);
    }
}

#[test]
#[cfg(feature = "gzip")]
fn gzip() {
    test_set(&*GZIP_FIXTURES);
}

#[test]
#[cfg(feature = "xz")]
fn xz() {
    test_set(&*XZ_FIXTURES);
    // test the underlying reader one byte at a time
    small_decode(
        XzReader::new(small_decode_make(XZ_FIXTURES.get("random").unwrap())),
        &get_expected("random"),
    );
}

#[test]
#[cfg(feature = "zstd")]
fn zstd() {
    test_set(&*ZSTD_FIXTURES);
    // test with multiple frames
    test_concatenated_inputs(&*ZSTD_FIXTURES);
    // test the underlying reader one byte at a time
    small_decode(
        ZstdReader::new(small_decode_make(ZSTD_FIXTURES.get("random").unwrap())).unwrap(),
        &get_expected("random"),
    );
}

#[test]
fn invalid() {
    assert!(matches!(
        DecompressReader::new(BufReader::with_capacity(32, &b"hello world"[..])).unwrap_err(),
        DecompressError::UnrecognizedFormat
    ));
}

fn test_set(inputs: &HashMap<&str, &[u8]>) {
    for (name, data) in inputs {
        test_case(name, data, &get_expected(name));
    }
}

fn test_case(name: &str, input: &[u8], expected: &[u8]) {
    let mut input = input.to_vec();
    let mut output = Vec::new();
    println!("=== {name} ===");

    // successful run
    DecompressReader::new(BufReader::with_capacity(32, &*input))
        .unwrap()
        .read_to_end(&mut output)
        .unwrap();
    assert_eq!(&output, expected);

    // drop last byte, make sure we notice
    output.clear();
    DecompressReader::new(BufReader::with_capacity(32, &input[0..input.len() - 1]))
        .unwrap()
        .read_to_end(&mut output)
        .unwrap_err();

    // add trailing garbage, make sure we notice
    input.push(12);
    output.clear();
    DecompressReader::new(BufReader::with_capacity(32, &*input))
        .unwrap()
        .read_to_end(&mut output)
        .unwrap_err();

    // use concatenated mode, make sure we ignore trailing garbage
    output.clear();
    let mut reader = DecompressBuilder::new()
        .trailing_data(true)
        .reader(BufReader::with_capacity(32, &*input))
        .unwrap();
    reader.read_to_end(&mut output).unwrap();
    assert_eq!(&output, expected);
    let mut remainder = Vec::new();
    reader.into_reader().read_to_end(&mut remainder).unwrap();
    assert_eq!(&remainder, &[12]);
}

fn test_concatenated_inputs(cases: &HashMap<&str, &[u8]>) {
    let mut input = Vec::new();
    let mut expected = Vec::new();
    let uncompressed = get_expected("random");
    for _ in 0..3 {
        input.extend(*cases.get("random").unwrap());
        expected.extend(&uncompressed);
    }
    test_case("concatenated random", &input, &expected);
}

fn small_decode<T: Read + FormatReader<BufReader<Cursor<Vec<u8>>>>>(mut d: T, expected: &[u8]) {
    let mut out = Vec::new();
    let mut buf = [0u8];
    loop {
        match d.read(&mut buf).unwrap() {
            0 => break,
            1 => out.push(buf[0]),
            _ => unreachable!(),
        }
    }
    assert_eq!(&out, &expected);
    let mut remainder = Vec::new();
    d.into_inner().read_to_end(&mut remainder).unwrap();
    assert_eq!(&remainder, b"abcdefg");
}

fn small_decode_make(f_compressed: &[u8]) -> PeekReader<BufReader<Cursor<Vec<u8>>>> {
    let mut compressed = f_compressed.to_vec();
    compressed.extend(b"abcdefg");
    PeekReader::new(BufReader::with_capacity(1, Cursor::new(compressed)))
}

fn get_expected(name: &str) -> Vec<u8> {
    gunzip(GZIP_FIXTURES.get(name).unwrap())
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut ret = Vec::new();
    GzDecoder::new(&*data).read_to_end(&mut ret).unwrap();
    ret
}
