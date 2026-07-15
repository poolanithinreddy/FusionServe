import importlib.util
import json
import subprocess
import sys
from copy import deepcopy
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "benchmarks/validate_results.py"
SPEC = importlib.util.spec_from_file_location("validate_results", SCRIPT)
assert SPEC and SPEC.loader
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


@pytest.fixture
def valid_result():
    return {
        "schema_version": 2,
        "benchmark_version": "fusionserve-bench-v2",
        "run_id": "test-c8-r1",
        "date": "2026-07-15",
        "git_commit": "f28e241",
        "backend_type": "mock",
        "hardware_type": "cpu",
        "environment": {
            "os": "test-os",
            "cpu": "test-cpu",
            "memory_bytes": 1024,
            "gpu_count": 0,
        },
        "warmup_requests": 5,
        "repetitions": 1,
        "units": {"latency": "ms", "throughput": "requests/second"},
        "measurement": {
            "requests": 10,
            "successes": 9,
            "errors": 1,
            "error_rate": 0.1,
            "latency_ms": {"p50": 1.0, "p90": 1.5, "p95": 2.0, "p99": 3.0, "max": 4.0},
        },
    }


def test_accepts_valid_complete_result(valid_result):
    VALIDATOR.validate_result(valid_result)


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (lambda result: result["measurement"]["latency_ms"].update(p99=1.5), "not monotonic"),
        (lambda result: result["measurement"].update(errors=2), "must equal requests"),
        (lambda result: result["measurement"].update(error_rate=0.2), "inconsistent"),
        (lambda result: result["measurement"]["latency_ms"].update(p50=-1), "non-negative"),
        (lambda result: result.pop("benchmark_version"), "missing required metadata"),
        (lambda result: result["environment"].pop("gpu_count"), "missing hardware metadata"),
        (lambda result: result.update(hardware_type="single_gpu"), "gpu_count=1"),
    ],
)
def test_rejects_invalid_results(valid_result, mutation, message):
    result = deepcopy(valid_result)
    mutation(result)
    with pytest.raises(VALIDATOR.ValidationError, match=message):
        VALIDATOR.validate_result(result)


def test_cli_rejects_duplicate_run_ids(tmp_path, valid_result):
    first = tmp_path / "first.json"
    second = tmp_path / "second.json"
    first.write_text(json.dumps(valid_result))
    second.write_text(json.dumps(valid_result))
    completed = subprocess.run(
        [sys.executable, str(SCRIPT), str(tmp_path)],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 1
    assert "duplicate run_id" in completed.stderr
