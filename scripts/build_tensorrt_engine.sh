#!/usr/bin/env bash
# Build a TensorRT engine (.plan) from the ResNet-50 ONNX model.
# Must run inside the Triton container (or any image with trtexec) on the SAME
# GPU architecture you will serve on — TensorRT engines are not portable across
# GPU generations. See docs/deployment.md.
set -euo pipefail

ONNX="${1:-models/triton/resnet50_onnx/1/model.onnx}"
PLAN="${2:-models/triton/resnet50_tensorrt/1/model.plan}"
PRECISION="${PRECISION:-fp16}"   # fp16 | fp32 | int8

if [[ ! -f "$ONNX" ]]; then
  echo "ONNX model not found at $ONNX. Run scripts/download_models.py first." >&2
  exit 1
fi
if ! command -v trtexec >/dev/null 2>&1; then
  echo "trtexec not found. Run this inside the Triton/TensorRT container." >&2
  exit 1
fi

mkdir -p "$(dirname "$PLAN")"
FLAGS=(--onnx="$ONNX" --saveEngine="$PLAN"
       --minShapes=input:1x3x224x224
       --optShapes=input:16x3x224x224
       --maxShapes=input:32x3x224x224)
[[ "$PRECISION" == "fp16" ]] && FLAGS+=(--fp16)
[[ "$PRECISION" == "int8" ]] && FLAGS+=(--int8)

echo "Building TensorRT engine ($PRECISION) -> $PLAN"
trtexec "${FLAGS[@]}"
echo "done: $PLAN"
