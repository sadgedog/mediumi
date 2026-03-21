build-all:
	cargo build --workspace

build-aac:
	cargo build -p mediumi-aac

build-ac3:
	cargo build -p mediumi-ac3

build-h264:
	cargo build -p mediumi-h264

build-mpeg2ts:
	cargo build -p mediumi-mpeg2ts

fmt:
	cargo fmt --all -- --check  

clippy:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

test-all:
	cargo test --workspace

test-aac:
	cargo test -p mediumi-aac

test-ac3:
	cargo test -p mediumi-ac3

test-h264:
	cargo test -p mediumi-h264

test-mpeg2ts:
	cargo test -p mediumi-mpeg2ts
