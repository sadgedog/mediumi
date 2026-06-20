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

### Generate test data (using ffmpeg)

For Annex.B input (raw H.264 byte stream with start codes):
```sh
$ mkdir -p examples/data && cd examples/data
$ ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -c:v libx264 -x264-params aud=1 -f h264 test.h264
```

For AVCC input, generate a non-fragmented mp4. The AVCC example extracts the `avcC`
config and `mdat` sample bytes from it:
```sh
$ ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1280x720:rate=30 \
    -pix_fmt yuv420p \
    -c:v libx264 -profile:v main -preset medium \
    -movflags +faststart \
    test.mp4
```

### Parse (Annex.B)
- Walk the NAL units of `test.h264` and dump per-NAL info.
```sh
$ cargo run --example annex_b_parse
```

### Roundtrip (Annex.B)
- Parse `test.h264`, re-serialize, and assert byte-exact equality with the input.
```sh
$ cargo run --example h264_roundtrip
```

### AVCC (extracted from mp4)
- Demux `test.mp4`, locate the `avcC` box and `mdat` samples, then feed them into
  `Processor::from_avcc`. Round-trips back to AVCC via `Processor::to_avcc(&cfg)` and
  verifies the sample bytes match.
```sh
$ cargo run --example h264_avcc
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
