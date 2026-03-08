build-all:
	cargo build --workspace

build-mpeg2ts:
	cargo build -p mediumi-mpeg2ts

fmt:
	cargo fmt --all -- --check  

clippy:
	cargo clippy --all-targets --all-features --workspace -- -D warnings

test-all:
	cargo test --workspace

test-mpeg2ts:
	cargo test -p mediumi-mpeg2ts
