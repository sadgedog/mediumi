#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Check ffmpeg is available
if ! command -v ffmpeg &>/dev/null; then
    echo "Error: ffmpeg is not installed"
    exit 1
fi

# Create output directories
mkdir -p "$ROOT_DIR/mediumi-aac/examples/data"
mkdir -p "$ROOT_DIR/mediumi-ac3/examples/data"
mkdir -p "$ROOT_DIR/mediumi-h264/examples/data"
mkdir -p "$ROOT_DIR/mediumi-mpeg2ts/examples/data"
mkdir -p "$ROOT_DIR/mediumi-mp4/examples/data"
mkdir -p "$ROOT_DIR/mediumi/examples/data"

# AAC (ADTS)
echo "Generating test.aac ..."
ffmpeg -y -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a aac -f adts \
    "$ROOT_DIR/mediumi-aac/examples/data/test.aac"

# AC-3
echo "Generating test.ac3 ..."
ffmpeg -y -f lavfi -i sine=frequency=440:duration=3:sample_rate=44100 \
    -c:a ac3 -f ac3 \
    "$ROOT_DIR/mediumi-ac3/examples/data/test.ac3"

# H.264
echo "Generating test.h264 ..."
ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -c:v libx264 -x264-params aud=1 -f h264 \
    "$ROOT_DIR/mediumi-h264/examples/data/test.h264"

# MPEG2-TS
echo "Generating test.ts ..."
ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1920x1080:rate=30 \
    -f lavfi -i sine=frequency=440:duration=3 \
    -pix_fmt yuv420p \
    -c:v libx264 -profile:v main -level 4.0 -preset slow \
    -c:a aac -ar 48000 -ac 2 \
    -f mpegts \
    "$ROOT_DIR/mediumi-mpeg2ts/examples/data/test.ts"

# MP4 (non-fragmented)
echo "Generating test.mp4 ..."
ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1280x720:rate=30 \
    -f lavfi -i sine=frequency=440:duration=3 \
    -pix_fmt yuv420p \
    -c:v libx264 -profile:v main -preset medium \
    -c:a aac -ar 48000 -ac 2 \
    -movflags +faststart \
    "$ROOT_DIR/mediumi-mp4/examples/data/test.mp4"

# MP4 (fragmented)
echo "Generating test_init.m4s and test.m4s ..."
(
    cd "$ROOT_DIR/mediumi-mp4/examples/data"
    ffmpeg -y -f lavfi -i testsrc2=duration=3:size=1280x720:rate=30 \
        -pix_fmt yuv420p \
        -c:v libx264 -profile:v main -preset medium -g 30 \
        -f dash \
        -seg_duration 10 \
        -use_template 0 \
        -use_timeline 0 \
        -init_seg_name 'test_init.m4s' \
        -media_seg_name 'test.m4s' \
        test.mpd
    rm -f test.mpd
)

# Mirror the container fixtures used by mediumi (umbrella) integration examples.
# `mediumi/examples/*.rs` consume mp4 + ts data alongside codec crates, so it needs
# its own copies (the source crates remain self-sufficient too).
echo "Mirroring fixtures into mediumi/examples/data ..."
cp "$ROOT_DIR/mediumi-mpeg2ts/examples/data/test.ts"      "$ROOT_DIR/mediumi/examples/data/test.ts"
cp "$ROOT_DIR/mediumi-mp4/examples/data/test.mp4"         "$ROOT_DIR/mediumi/examples/data/test.mp4"
cp "$ROOT_DIR/mediumi-mp4/examples/data/test_init.m4s"    "$ROOT_DIR/mediumi/examples/data/test_init.m4s"
cp "$ROOT_DIR/mediumi-mp4/examples/data/test.m4s"         "$ROOT_DIR/mediumi/examples/data/test.m4s"

echo "All test data generated."
