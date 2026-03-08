# mediumi-codec: A Video/Audio Codec toolkit

## Build
```
$ cargo build -p mediumi-codec
```

## Run example
### Generate test h264 file (using ffmpeg)
Need to generate h264 raw data to make a sample input.
```
$ mkdir examples/data && cd examples/data
$ ffmpeg -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -c:v libx264 -f h264 test.h264
```

### Parse
```
$ cargo run --example h264_parse
```

### Roundtrip
```
$ cargo run --example h264_roundtrip
```

## Out of scope
- Specific NAL Unit Type (AUD, SEI, ...)
    - These may be supported in the future.