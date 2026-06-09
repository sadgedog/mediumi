# mediumi-crypto: Media cryptography toolkit

## About this
mediumi-crypto is a cryptography library that implements the **Common Encryption
(CENC, ISO/IEC 23001-7)** sample-level encryption scheme for ISO base media file
format content.

Supported protection schemes:
- `cenc` — AES-128-CTR, subsample encryption (no pattern)
- `cbcs` — AES-128-CBC, subsample encryption (1:9 pattern)

## Scope and limitations
- Media encryption for fmp4/cmaf (AES-128-CTR / AES-128-CBC)
- This crate is **NOT** a tool for circumventing DRM.

## Build
```sh
$ cargo build -p mediumi-crypto
```

## Status
This project is under active development. APIs may change without notice.
