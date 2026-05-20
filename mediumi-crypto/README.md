# mediumi-crypto: Media cryptography toolkit

## About this
mediumi-crypto is a cryptography library that implements the **Common Encryption
(CENC, ISO/IEC 23001-7)** sample-level encryption scheme for ISO base media file
format content.

Supported protection schemes:
- `cenc` — AES-128-CTR, full-sample encryption
- `cbcs` — AES-128-CBC, subsample pattern encryption

## Scope and limitations
- AES-128-CTR / AES-128-CBC primitives wired for CENC sample-level operation
- Sample / subsample cipher application for the `cenc` and `cbcs` schemes
- IV handling, subsample pattern (encrypt:skip block ratio) handling
- Encryption and decryption symmetry: given a clear sample and a key the caller controls, the crate can both encrypt and decrypt
- This crate is **NOT** a tool for circumventing DRM.

## Build
```sh
$ cargo build -p mediumi-crypto
```

## Status
This project is under active development. APIs may change without notice.
