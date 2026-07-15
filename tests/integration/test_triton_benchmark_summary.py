import json
import subprocess
import sys
from pathlib import Path


def test_perf_analyzer_csv_is_normalized(tmp_path):
    source = tmp_path / "perf_http_c8.csv"
    source.write_text(
        "Concurrency,Inferences/Second,Client Avg Latency,Server Queue,Server Compute Infer\n"
        "8,340.5,42000,1200,39000\n"
    )
    output = tmp_path / "summary.json"
    root = Path(__file__).resolve().parents[2]
    subprocess.run(
        [
            sys.executable,
            str(root / "benchmarks/triton/summarize.py"),
            str(source),
            "--output",
            str(output),
        ],
        check=True,
    )
    measurement = json.loads(output.read_text())["measurements"][0]
    assert measurement == {
        "source": "perf_http_c8.csv",
        "concurrency": 8,
        "throughput_rps": 340.5,
        "client_latency_us": 42000.0,
        "server_queue_us": 1200.0,
        "server_compute_us": 39000.0,
    }
