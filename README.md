# mediumi: A Media Toolkit

## About this
mediumi is a media toolkit written from scratch in Rust.
It provides the following features.

### Features
- Media container demuxer and muxer for the following containers
    - MPEG2-TS
- Parser and Serializers and for the following codecs
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

## Contributing
Contributions are welcome. \
If you find a bug, have a feature request, or want to improve the implementation, please open an [issue](https://github.com/sadgedog/mediumi/issues) or submit a [pull request](https://github.com/sadgedog/mediumi/pulls).