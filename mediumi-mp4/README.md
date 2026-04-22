# mediumi-mp4: A MPEG-4 toolkit

## About this
mediumi-mp4 is a MPEG-4(mp4, fmp4) demuxer and muxer.

## Build
```sh
$ cargo build -p mediumi-mp4
```

## Status
This project is under active development. APIs may change without notice.

- [x] ftyp (4.3)
- [x] mdat (8.2.2)
- [ ] moof (8.8.4)
    - [x] mfhd (8.8.5)
    - [ ] meta (8.11.1)
    - [x] traf (8.8.6)
        - [x] tfhd (8.8.7)
        - [x] trun (8.8.8)
        - [x] sbgp (8.9.2)
        - [x] sgpd (8.9.3)
        - [x] subs (8.7.7)
        - [ ] saiz (8.7.8)
        - [ ] saio (8.7.9)
        - [x] tfdt (8.8.12)
        - [ ] meta (8.11.1)