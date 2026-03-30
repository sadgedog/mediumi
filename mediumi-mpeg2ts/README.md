# mediumi-mpeg2ts: An MPEG2-TS toolkit

## About this
mediumi-mpeg2ts is an MPEG2-TS demuxer and muxer.

## Build
```sh
$ cargo build -p mediumi-mpeg2ts
```

## Run example
### Generate test TS segment (using ffmpeg)
Need to generate TS segment to make a sample input.
```sh
$ mkdir examples/data && cd examples/data
$ ffmpeg -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -f lavfi -i sine=frequency=440:duration=3 \
    -pix_fmt yuv420p \
    -c:v libx264 -profile:v main -level 4.0 -preset slow \
    -c:a aac -ar 48000 -ac 2 \
    -f mpegts test.ts
```

### Demux
- Demux TS packets into individual TS packet struct.
```sh
$ cargo run --example ts_demux
```
- Demux into PES streams.
```sh
$ cargo run --example pes_demux
```

### Mux
- Demux and mux at TS packet level (lossless round-trip).
```sh
$ cargo run --example ts_mux
```
- Demux and mux at PES level (does not preserve interleaving).
```sh
$ cargo run --example pes_mux
```

## Out of scope
- Specific PSI (CAT, TSDT) and SI (NIT, SDT, BAT, TDT, TOT, RST).
    - These may be supported in the future.

## Status
This project is under active development. APIs may change without notice.
