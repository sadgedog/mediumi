build-all:
	cargo build --workspace

build-mpeg2ts:
	cargo build -p mediumi-mpeg2ts

build-codec:
	cargo build -p mediumi-codec

fmt:
	cargo fmt --all -- --check  

clippy:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

test-all:
	cargo test --workspace

test-mpeg2ts:
	cargo test -p mediumi-mpeg2ts

test-codec:
	cargo test -p mediumi-codec
