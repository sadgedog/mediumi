# mediumi: A Media Toolkit

## About this
mediumi is a media toolkit written from scratch in Rust.
It provides the following features.

### Features
- Media container demuxer and muxer for the following containers
    - MPEG2-TS
- Parser and serializer for the following codecs
    - AAC (ADTS)
    - AC-3 (A/52)
    - H.264 (AVC)

## Crates
- mediumi-aac
    - AAC (ADTS) bitstream parser and serializer
- mediumi-ac3
    - AC-3 (A/52) bitstream parser and serializer
- mediumi-h264
    - H.264 (AVC) bitstream parser and serializer
- mediumi-mpeg2ts
    - MPEG2-TS container demuxer and muxer

## Build
```sh
$ cargo build --workspace        # Build all crates
$ cargo build -p mediumi-aac     # Build specific crate
$ cargo build -p mediumi-ac3
$ cargo build -p mediumi-h264
$ cargo build -p mediumi-mpeg2ts
```

## Test
```sh
$ cargo test --workspace        # Test all crates
$ cargo test -p mediumi-aac     # Test specific crate
$ cargo test -p mediumi-ac3
$ cargo test -p mediumi-h264
$ cargo test -p mediumi-mpeg2ts
```

## Specification
- AAC (ADTS)
    - ISO/IEC 14496-3
- AC-3 (A/52)
    - ATSC A/52
- H.264
    - ISO/IEC 14496-10
- MPEG2-TS
    - ISO/IEC 13818-1

## Status
This project is under active development. APIs may change without notice.
