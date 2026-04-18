# mediumi-h264: H.264 toolkit

## About this
mediumi-h264 is an H.264 parser and serializer.

## Scope and limitations
This crate **DOES NOT** provide any H.264 encoder or decoder implementation.
It only parses and serializes the H.264 bitstream syntax (NAL unit structure,
SPS / PPS / slice header fields, etc...).

## Build
```sh
$ cargo build -p mediumi-h264
```

## Run example
### Generate test h264 file (using ffmpeg)
Need to generate h264 raw data to make a sample input.
```sh
$ mkdir examples/data && cd examples/data
$ ffmpeg -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -c:v libx264 -f h264 test.h264
```

### Parse
```sh
$ cargo run --example h264_parse
```

### Roundtrip
```sh
$ cargo run --example h264_roundtrip
```

## Out of scope
- Specific NAL Unit Type
    - [ ] PrefixNalUnit
    - [ ] SubsetSPS
    - [ ] DPS
    - [ ] Reserved
    - [ ] AUX
    - [ ] SliceExt
    - [ ] DepthExt


## Status
This project is under active development. APIs may change without notice.
