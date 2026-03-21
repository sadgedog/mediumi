# mediumi-aac: AAC toolkit

## Support
- AAC(ADTS)

## Build
```
$ cargo build -p mediumi-aac
```

## Run example
### Generate test adts file (using ffmpeg)
Need to generate adts raw data to make a sample input.
```
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a aac -f adts test.aac
```

### Parse
```
$ cargo run --example adts_parse
```

### Roundtrip
```
$ cargo run --example adts_roundtrip
```
