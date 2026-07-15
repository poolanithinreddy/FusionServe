#!/usr/bin/env python3
"""Normalize Triton Performance Analyzer CSV files into stable JSON."""

import argparse
import csv
import json
from pathlib import Path


def parse(path: Path):
    with path.open(newline="") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise ValueError(f"{path} contains no measurements")
    row = rows[-1]

    def number(*names):
        for name in names:
            if name in row and row[name] != "":
                return float(row[name])
        return None

    return {
        "source": path.name,
        "concurrency": int(number("Concurrency") or 0),
        "throughput_rps": number(
            "Inferences/Second", "Inferences/Second vs. Client Average Batch Latency"
        ),
        "client_latency_us": number("Client Send", "Client Avg Latency"),
        "server_queue_us": number("Server Queue"),
        "server_compute_us": number("Server Compute Infer", "Server Compute"),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = {"schema_version": 1, "measurements": [parse(path) for path in args.files]}
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
