#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DEST="$ROOT/models/triton/resnet50_onnx/1/model.onnx"
URL="https://github.com/onnx/models/raw/main/validated/vision/classification/resnet/model/resnet50-v2-7.onnx"
EXPECTED="79102261eb6e5fd7af5d27f41316293e388c5cb691e5d25bfb035c4f64fefe31"

mkdir -p "$(dirname "$DEST")"
tmp="${DEST}.download"
trap 'rm -f "$tmp"' EXIT
curl --fail --location --retry 3 --output "$tmp" "$URL"
actual="$(shasum -a 256 "$tmp" | awk '{print $1}')"
if [[ "$actual" != "$EXPECTED" ]]; then
  echo "checksum mismatch: expected $EXPECTED, got $actual" >&2
  exit 1
fi
mv "$tmp" "$DEST"
echo "verified ResNet-50 ONNX: $DEST"
