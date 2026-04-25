# mediumi-mp4: A MPEG-4 toolkit

## About this
mediumi-mp4 is a MPEG-4(mp4, fmp4) demuxer and muxer.

## Build
```sh
$ cargo build -p mediumi-mp4
```

## Status
This project is under active development. APIs may change without notice.

- [x] ftyp (4.3) *
- [x] mdat (8.1.1) *
- [ ] moov (8.2.1) *
    - [x] mvhd (8.2.2) *
    - [x] meta (8.11.1)
    - [ ] trak (8.3.1) *
        - [ ] tkhd (8.3.2) *
        - [ ] tref (8.3.3)
        - [ ] trgr (8.3.4)
        - [ ] edts (8.6.4)
            - [ ] elst (8.6.6)
        - [ ] meta (8.11.1)
        - [ ] mdia (8.4) *
            - [ ] mdhd (8.4.2) *
            - [ ] hdlr (8.4.3) *
            - [ ] elng (8.4.6)
            - [ ] minf (8.4.4) *
                - [ ] vmhd (12.1.2)
                - [ ] smhd (12.2.2)
                - [ ] hmhd (12.4.2)
                - [ ] sthd (12.6.2)
                - [ ] nmhd (8.4.5.2)
                - [ ] dinf (8.7.1) *
                    - [ ] dref (8.7.2) *
                - [ ] stbl (8.5.1) *
                    - [ ] stsd (8.5.2) *
                    - [ ] stts (8.6.1.2) *
                    - [ ] ctts (8.6.1.3) *
                    - [ ] cslg (8.6.1.4)
                    - [ ] stsc (8.7.4) *
                    - [ ] stsz (8.7.3.2)
                    - [ ] stz2 (8.7.3.3)
                    - [ ] stco (8.7.5) *
                    - [ ] co64 (8.7.5)
                    - [ ] stss (8.6.2)
                    - [ ] stsh (8.6.3)
                    - [ ] padb (8.7.6)
                    - [ ] stdp (8.7.6)
                    - [ ] sdtp (8.6.4)
                    - [ ] sbgp (8.9.2)
                    - [ ] sgpd (8.9.3)
                    - [ ] subs (8.7.7)
                    - [ ] saiz (8.7.8)
                    - [ ] saio (8.7.9)
            - [ ] udta (8.10.1)
        - [ ] mvex (8.8.1)
            - [ ] mehd (8.8.2)
            - [ ] trex (8.8.3) *
            - [ ] leva (8.8.13)
- [x] moof (8.8.4)
    - [x] mfhd (8.8.5)
    - [x] meta (8.11.1)
        - [x] hdlr (8.4.3)
    - [x] traf (8.8.6)
        - [x] tfhd (8.8.7)
        - [x] trun (8.8.8)
        - [x] sbgp (8.9.2)
        - [x] sgpd (8.9.3)
        - [x] subs (8.7.7)
        - [x] saiz (8.7.8)
        - [x] saio (8.7.9)
        - [x] tfdt (8.8.12)
        - [x] meta (8.11.1)
