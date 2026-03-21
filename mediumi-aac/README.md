# mediumi-aac: AAC toolkit

## About this
mediumi-aac is an AAC (ADTS) parser and serializer.

## Build
```sh
$ cargo build -p mediumi-aac
```

## Run example
### Generate test adts file (using ffmpeg)
Need to generate adts raw data to make a sample input.
```sh
$ ffmpeg -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a aac -f adts test.aac
```

### Parse
```sh
$ cargo run --example adts_parse
```

### Roundtrip
```sh
$ cargo run --example adts_roundtrip
```

## Status
This project is under active development. APIs may change without notice.
