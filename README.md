# mediumi: A Media Toolkit

## About this
mediumi is a media toolkit written from scratch in Rust.
It provides the following features.

### Features
- Media container encoders and decoders for the following containers
    - MPEG2-TS
- Serializers and deserializers for the follwoing codecs
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
    - MPEG2-TS container encoder and decoder

## Build
```sh
$ make build-all     # Build all crates
$ make build-aac     # Build specific crate
$ make build-ac3
$ make build-h264
$ make build-mpeg2ts
```

## Test
```sh
$ make test-all     # Test all crates
$ make test-aac     # Test specific crate
$ make test-ac3
$ make test-h264
$ make test-mpeg2ts
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
