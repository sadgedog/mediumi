build-all:
	cargo build --workspace

build-aac:
	cargo build -p mediumi-aac

build-ac3:
	cargo build -p mediumi-ac3

build-crypto:
	cargo build -p mediumi-crypto

build-h264:
	cargo build -p mediumi-h264

build-mp4:
	cargo build -p mediumi-mp4

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

test-crypto:
	cargo test -p mediumi-crypto

test-h264:
	cargo test -p mediumi-h264

test-mp4:
	cargo test -p mediumi-mp4

test-mpeg2ts:
	cargo test -p mediumi-mpeg2ts

test-roundtrip:
	./script/prepare_test_data.sh \
		&& cargo run --example adts_roundtrip \
		&& cargo run --example ac3_roundtrip \
		&& cargo run --example h264_roundtrip \
		&& ./script/cleanup_test_data.sh

test-encrypt:
	./script/prepare_test_data.sh \
		&& cargo run --example encrypt_video \
		&& cargo run --example encrypt_audio \
		&& cargo run --example encrypt_muxed \
		&& ./script/cleanup_test_data.sh
