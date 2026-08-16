# mediumi-h265: H.265 toolkit

## About this

mediumi-h265 is an H.265 parser and serializer.

## Scope and limitations

This crate **DOES NOT** provide any H.265 encoder or decoder implementation.
It only parses and serializes the H.265 bitstream syntax (NAL unit structure,
SPS / PPS / slice header fields, etc...).

## Build

```sh
$ cargo build -p mediumi-h265
```

## Status

This project is under active development. APIs may change without notice.
