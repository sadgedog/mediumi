#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

rm  "$ROOT_DIR/mediumi-aac/examples/data/test.aac"
rm  "$ROOT_DIR/mediumi-ac3/examples/data/test.ac3"
rm  "$ROOT_DIR/mediumi-h264/examples/data/test.h264"
rm  "$ROOT_DIR/mediumi-mpeg2ts/examples/data/test.ts"
rm  "$ROOT_DIR/mediumi-mp4/examples/data/test.mp4"
rm  "$ROOT_DIR/mediumi-mp4/examples/data/test_init.m4s"
rm  "$ROOT_DIR/mediumi-mp4/examples/data/test.m4s"

echo "All test data removed."
