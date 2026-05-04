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
- [x] moov (8.2.1) *
    - [x] mvhd (8.2.2) *
    - [x] meta (8.11.1)
    - [x] trak (8.3.1) *
        - [x] tkhd (8.3.2) *
        - [x] tref (8.3.3)
        - [x] trgr (8.3.4)
        - [x] edts (8.6.4)
            - [x] elst (8.6.6)
        - [x] meta (8.11.1)
        - [x] mdia (8.4) *
            - [x] mdhd (8.4.2) *
            - [x] hdlr (8.4.3) *
            - [x] elng (8.4.6)
            - [x] minf (8.4.4) *
                - [x] vmhd (12.1.2)
                - [x] smhd (12.2.2)
                - [x] hmhd (12.4.2)
                - [x] sthd (12.6.2)
                - [x] nmhd (8.4.5.2)
                - [x] dinf (8.7.1) *
                    - [x] dref (8.7.2) *
                - [x] stbl (8.5.1) *
                    - [x] stsd (8.5.2) *
                    - [x] stts (8.6.1.2) *
                    - [x] ctts (8.6.1.3) *
                    - [x] cslg (8.6.1.4)
                    - [x] stsc (8.7.4) *
                    - [x] stsz (8.7.3.2)
                    - [x] stz2 (8.7.3.3)
                    - [x] stco (8.7.5) *
                    - [x] co64 (8.7.5)
                    - [x] stss (8.6.2)
                    - [x] stsh (8.6.3)
                    - [x] padb (8.7.6)
                    - [x] stdp (8.7.6)
                    - [x] sdtp (8.6.4)
                    - [x] sbgp (8.9.2)
                    - [x] sgpd (8.9.3)
                    - [x] subs (8.7.7)
                    - [x] saiz (8.7.8)
                    - [x] saio (8.7.9)
        - [x] udta (8.10.1)
    - [x] mvex (8.8.1)
        - [x] mehd (8.8.2)
        - [x] trex (8.8.3) *
        - [x] leva (8.8.13)
    - [x] udta (8.10.1)
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
