# mediumi-codec: AC-3 toolkit

## Support
- AC-3

## Build
```
$ cargo build -p mediumi-ac3
```

## Run example
### Generate test ac3 file (using ffmpeg)
Need to generate ac3 raw data to make a sample input.
```
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a ac3 -f ac3 test.ac3
```

### Parse
```
$ cargo run --example ac3_parse
```

### Roundtrip
```
$ cargo run --example ac3_roundtrip
```
