#!/usr/bin/env python3
"""Validate FusionServe benchmark JSON before results are published."""

import argparse
import json
import math
import re
import sys
from pathlib import Path
from typing import Any, Dict, Iterable, List, Sequence, Tuple


BACKEND_TYPES = {"mock", "triton", "dynamo", "vllm"}
HARDWARE_TYPES = {"cpu", "single_gpu", "multi_gpu"}
PERCENTILES: Sequence[str] = ("p50", "p90", "p95", "p99", "max")
COMMIT_RE = re.compile(r"^[0-9a-f]{7,40}$")


class ValidationError(ValueError):
    """A benchmark result violates the publication contract."""


def _nonnegative_number(value: Any, field: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValidationError(f"{field} must be numeric")
    number = float(value)
    if not math.isfinite(number) or number < 0:
        raise ValidationError(f"{field} must be finite and non-negative")
    return number


def _nonnegative_integer(value: Any, field: str) -> int:
    number = _nonnegative_number(value, field)
    if not number.is_integer():
        raise ValidationError(f"{field} must be an integer")
    return int(number)


def _validate_percentiles(value: Dict[str, Any], path: str) -> None:
    present = [key for key in PERCENTILES if key in value]
    if not present:
        return
    if not {"p50", "p95", "p99"}.issubset(value):
        raise ValidationError(f"{path} must contain at least p50, p95 and p99")
    numbers = [_nonnegative_number(value[key], f"{path}.{key}") for key in present]
    if numbers != sorted(numbers):
        raise ValidationError(f"{path} percentiles are not monotonic: {present}")


def _walk(value: Any, path: str = "result") -> Iterable[Tuple[str, Any]]:
    yield path, value
    if isinstance(value, dict):
        for key, child in value.items():
            yield from _walk(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from _walk(child, f"{path}[{index}]")


def validate_result(result: Dict[str, Any]) -> None:
    """Validate one processed or raw benchmark result document."""
    required = (
        "schema_version",
        "benchmark_version",
        "run_id",
        "date",
        "git_commit",
        "backend_type",
        "hardware_type",
        "environment",
        "warmup_requests",
        "repetitions",
        "units",
    )
    missing = [field for field in required if field not in result]
    if missing:
        raise ValidationError(f"missing required metadata: {', '.join(missing)}")
    if result["schema_version"] != 2:
        raise ValidationError("schema_version must be 2")
    if (
        not isinstance(result["benchmark_version"], str)
        or not result["benchmark_version"].strip()
    ):
        raise ValidationError("benchmark_version must be a non-empty string")
    if not isinstance(result["run_id"], str) or not result["run_id"].strip():
        raise ValidationError("run_id must be a non-empty string")
    if not isinstance(result["git_commit"], str) or not COMMIT_RE.fullmatch(
        result["git_commit"]
    ):
        raise ValidationError(
            "git_commit must be a 7-40 character lowercase hexadecimal SHA"
        )
    if result["backend_type"] not in BACKEND_TYPES:
        raise ValidationError(f"backend_type must be one of {sorted(BACKEND_TYPES)}")
    if result["hardware_type"] not in HARDWARE_TYPES:
        raise ValidationError(f"hardware_type must be one of {sorted(HARDWARE_TYPES)}")
    _nonnegative_integer(result["warmup_requests"], "warmup_requests")
    if _nonnegative_integer(result["repetitions"], "repetitions") < 1:
        raise ValidationError("repetitions must be at least 1")

    environment = result["environment"]
    if not isinstance(environment, dict):
        raise ValidationError("environment must be an object")
    environment_fields = ("os", "cpu", "memory_bytes", "gpu_count")
    missing_environment = [
        field for field in environment_fields if field not in environment
    ]
    if missing_environment:
        raise ValidationError(
            f"missing hardware metadata: {', '.join(missing_environment)}"
        )
    if not environment["os"] or not environment["cpu"]:
        raise ValidationError("environment os and cpu must be non-empty")
    if (
        _nonnegative_integer(environment["memory_bytes"], "environment.memory_bytes")
        == 0
    ):
        raise ValidationError("environment.memory_bytes must be positive")
    gpu_count = _nonnegative_integer(environment["gpu_count"], "environment.gpu_count")
    if result["hardware_type"] == "cpu" and gpu_count != 0:
        raise ValidationError("CPU results must have gpu_count=0")
    if result["hardware_type"] == "single_gpu" and gpu_count != 1:
        raise ValidationError("single-GPU results must have gpu_count=1")
    if result["hardware_type"] == "multi_gpu" and gpu_count < 2:
        raise ValidationError("multi-GPU results must have gpu_count>=2")

    units = result["units"]
    if (
        not isinstance(units, dict)
        or not units.get("latency")
        or not units.get("throughput")
    ):
        raise ValidationError("units must define latency and throughput")

    found_count_group = False
    for path, value in _walk(result):
        if isinstance(value, dict):
            _validate_percentiles(value, path)
            if "successes" in value or "errors" in value:
                found_count_group = True
                if not {"requests", "successes", "errors"}.issubset(value):
                    raise ValidationError(
                        f"{path} must contain requests, successes and errors together"
                    )
                requests = _nonnegative_integer(value["requests"], f"{path}.requests")
                successes = _nonnegative_integer(
                    value["successes"], f"{path}.successes"
                )
                errors = _nonnegative_integer(value["errors"], f"{path}.errors")
                if requests < 1:
                    raise ValidationError(f"{path}.requests must be positive")
                if successes + errors != requests:
                    raise ValidationError(
                        f"{path} successes + errors must equal requests"
                    )
                if "error_rate" in value:
                    expected = errors / requests
                    actual = _nonnegative_number(
                        value["error_rate"], f"{path}.error_rate"
                    )
                    if not math.isclose(actual, expected, rel_tol=1e-9, abs_tol=1e-9):
                        raise ValidationError(
                            f"{path}.error_rate is inconsistent with counts"
                        )
        leaf = path.rsplit(".", 1)[-1]
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            timing = leaf.endswith(("_ms", "_us", "_seconds"))
            rate = leaf.endswith(("_rps", "_per_second", "_rate"))
            if timing or rate:
                _nonnegative_number(value, path)
    if not found_count_group:
        raise ValidationError("result contains no request-count group")


def _json_files(paths: Sequence[Path]) -> List[Path]:
    files: List[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.json")))
        else:
            files.append(path)
    return files


def validate_files(paths: Sequence[Path]) -> int:
    seen: Dict[str, Path] = {}
    failures: List[str] = []
    files = _json_files(paths)
    if not files:
        raise ValidationError("no JSON result files found")
    for path in files:
        try:
            value = json.loads(path.read_text())
            if not isinstance(value, dict):
                raise ValidationError("top-level JSON value must be an object")
            validate_result(value)
            run_id = value["run_id"]
            if run_id in seen:
                raise ValidationError(f"duplicate run_id also used by {seen[run_id]}")
            seen[run_id] = path
        except (OSError, json.JSONDecodeError, ValidationError) as exc:
            failures.append(f"{path}: {exc}")
    if failures:
        for failure in failures:
            print(f"ERROR: {failure}", file=sys.stderr)
        return 1
    print(f"validated {len(files)} benchmark result file(s)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    try:
        return validate_files(args.paths)
    except ValidationError as exc:
        parser.error(str(exc))
    return 2


if __name__ == "__main__":
    sys.exit(main())
