# mediumi-ac3: AC-3 toolkit

## About this
mediumi-ac3 is an AC-3 (A/52) parser and serializer.

## Build
```sh
$ cargo build -p mediumi-ac3
```

## Run example
### Generate test ac3 file (using ffmpeg)
Need to generate ac3 raw data to make a sample input.
```sh
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a ac3 -f ac3 test.ac3
```

### Parse
```sh
$ cargo run --example ac3_parse
```

### Roundtrip
```sh
$ cargo run --example ac3_roundtrip
```
