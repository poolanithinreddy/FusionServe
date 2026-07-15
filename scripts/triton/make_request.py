#!/usr/bin/env python3
"""Create deterministic zero-valued KServe JSON for ResNet-50."""

import json
import sys

payload = {
    "inputs": [
        {
            "name": "data",
            "shape": [1, 3, 224, 224],
            "datatype": "FP32",
            "data": [0.0] * (3 * 224 * 224),
        }
    ]
}
if len(sys.argv) > 1 and sys.argv[1] == "gateway":
    payload["model"] = "resnet50_onnx"
json.dump(payload, sys.stdout, separators=(",", ":"))
