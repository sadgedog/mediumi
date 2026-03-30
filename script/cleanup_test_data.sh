#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

rm  "$ROOT_DIR/mediumi-aac/examples/data/test.aac"
rm  "$ROOT_DIR/mediumi-ac3/examples/data/test.ac3"
rm  "$ROOT_DIR/mediumi-h264/examples/data/test.h264"
rm  "$ROOT_DIR/mediumi-mpeg2ts/examples/data/test.ts"

echo "All test data removed."
