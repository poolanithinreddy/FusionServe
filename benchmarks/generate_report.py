#!/usr/bin/env python3
"""Assemble benchmark result JSON files into a Markdown report.

Reads every `*.json` in the input directory (produced by the bench drivers) and
renders a single Markdown document. It refuses to invent numbers: if a required
environment record is missing, it flags the run as UNVERIFIED so no benchmark is
presented as measured on hardware it was not measured on.
"""
import argparse
import json
from pathlib import Path

ENV_FIELDS = [
    "gpu", "gpu_memory", "cpu", "ram", "os", "nvidia_driver", "cuda",
    "triton_version", "dynamo_version", "vllm_version", "model", "precision",
]


def load_results(input_dir: Path):
    runs = []
    for p in sorted(input_dir.glob("*.json")):
        try:
            runs.append((p.name, json.loads(p.read_text())))
        except ValueError:
            continue
    return runs


def render(runs) -> str:
    lines = ["# FusionServe Benchmark Results", ""]
    if not runs:
        lines += [
            "> No result files found in `benchmarks/results/`.",
            ">",
            "> Benchmarks have not yet been run on GPU hardware. Numbers will be",
            "> published here only when measured, with a full environment record.",
            "",
        ]
        return "\n".join(lines)

    for name, run in runs:
        env = run.get("environment", {})
        verified = all(env.get(k) for k in ENV_FIELDS)
        badge = "VERIFIED" if verified else "UNVERIFIED (missing environment record)"
        lines.append(f"## {name}")
        lines.append("")
        lines.append(f"**Status:** {badge}")
        lines.append("")
        if env:
            lines.append("| Environment | Value |")
            lines.append("|---|---|")
            for k in ENV_FIELDS:
                lines.append(f"| {k} | {env.get(k, '—')} |")
            lines.append("")
        lines.append("```json")
        lines.append(json.dumps({k: v for k, v in run.items() if k != "environment"}, indent=2))
        lines.append("```")
        lines.append("")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", default="benchmarks/results")
    ap.add_argument("--out", default="docs/benchmark-results.md")
    args = ap.parse_args()
    runs = load_results(Path(args.input))
    Path(args.out).write_text(render(runs))
    print(f"wrote {args.out} ({len(runs)} run(s))")


if __name__ == "__main__":
    main()
