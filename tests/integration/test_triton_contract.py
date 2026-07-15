"""CPU-only validation of the real-Triton model and request contract.

This does not start Triton. It prevents drift between the checked-in model
configuration, generated KServe payload, gateway registry, and download tool.
"""

import json
import re
import subprocess
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "models/triton/resnet50_onnx/config.pbtxt"


@pytest.mark.parametrize("mode", ["direct", "gateway"])
def test_generated_resnet_request_matches_model_contract(mode):
    command = [sys.executable, str(ROOT / "scripts/triton/make_request.py")]
    if mode == "gateway":
        command.append("gateway")
    payload = json.loads(subprocess.check_output(command, text=True))
    tensor = payload["inputs"][0]
    assert tensor["name"] == "data"
    assert tensor["datatype"] == "FP32"
    assert tensor["shape"] == [1, 3, 224, 224]
    assert len(tensor["data"]) == 3 * 224 * 224
    assert (payload.get("model") == "resnet50_onnx") is (mode == "gateway")


def test_model_config_uses_actual_onnx_tensor_names():
    text = CONFIG.read_text()
    assert 'name: "data"' in text
    assert 'name: "resnetv24_dense0_fwd"' in text
    assert "dims: [ 1, 3, 224, 224 ]" in text
    assert "dims: [ 1, 1000 ]" in text


def test_download_script_pins_a_sha256():
    text = (ROOT / "scripts/triton/download_resnet50.sh").read_text()
    match = re.search(r'EXPECTED="([0-9a-f]{64})"', text)
    assert match, "download must pin an explicit SHA-256"
    assert (
        match.group(1)
        == "79102261eb6e5fd7af5d27f41316293e388c5cb691e5d25bfb035c4f64fefe31"
    )
