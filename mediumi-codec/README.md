# mediumi-codec: A Video/Audio Codec toolkit

## Support
- H.264(AVC)
- AAC(ADTS)

## Build
```
$ cargo build -p mediumi-codec
```

## Run example
### Generate test h264 adts file (using ffmpeg)
Need to generate h264 & adts raw data to make a sample input.
```
$ mkdir examples/data && cd examples/data
$ ffmpeg -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -c:v libx264 -f h264 test.h264
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a aac -f adts test.aac
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a ac3 -f ac3 test.ac3
```

### Parse
```
$ cargo run --example h264_parse
$ cargo run --example adts_parse
```

### Roundtrip
```
$ cargo run --example h264_roundtrip
$ cargo run --example adts_roundtrip
```

## Out of scope
- Specific NAL Unit Type (AUD, SEI, ...)
    - These may be supported in the future.