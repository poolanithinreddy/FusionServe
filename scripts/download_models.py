#!/usr/bin/env python3
"""Download model artifacts into the Triton model repository.

No model binaries are committed to git (see .gitignore). This script fetches
them and records checksums. It intentionally does NOT hard-code opaque URLs for
proprietary weights — set them via the manifest below or environment.

Usage:
    python3 scripts/download_models.py [--only resnet50_onnx]

For ResNet-50 ONNX, the canonical source is the ONNX Model Zoo. For the LLM
weights (served by vLLM under Dynamo), follow docs/deployment.md — those are
pulled by vLLM/HuggingFace at container start, not by this script.
"""
import argparse
import hashlib
import json
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT / "models" / "triton"

# name -> (url, dest relative to REPO). URLs can be overridden by env vars of the
# form FUSIONSERVE_URL_<NAME>. Left blank where a license click-through or auth
# is required; the script will tell you what to do instead of failing silently.
MANIFEST = {
    "resnet50_onnx": {
        "url": os.environ.get(
            "FUSIONSERVE_URL_RESNET50_ONNX",
            "https://github.com/onnx/models/raw/main/validated/vision/classification/resnet/model/resnet50-v2-7.onnx",
        ),
        "dest": "resnet50_onnx/1/model.onnx",
    },
    "text_embedding": {
        # Requires exporting an embedding model to ONNX; see docs/deployment.md.
        "url": os.environ.get("FUSIONSERVE_URL_TEXT_EMBEDDING", ""),
        "dest": "text_embedding/1/model.onnx",
    },
}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def download(name: str, spec: dict) -> bool:
    url = spec["url"]
    dest = REPO / spec["dest"]
    if not url:
        print(f"[skip] {name}: no URL configured. See docs/deployment.md for how "
              f"to obtain and place it at {dest}.")
        return False
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        print(f"[have] {name}: {dest} ({dest.stat().st_size} bytes)")
        return True
    print(f"[get ] {name}: {url}")
    try:
        urllib.request.urlretrieve(url, dest)
    except Exception as e:  # noqa: BLE001
        print(f"[fail] {name}: {e}", file=sys.stderr)
        return False
    print(f"[ok  ] {name}: {dest} sha256={sha256(dest)}")
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default=None, help="download a single model by name")
    ap.add_argument("--write-checksums", action="store_true")
    args = ap.parse_args()

    items = MANIFEST.items()
    if args.only:
        items = [(args.only, MANIFEST[args.only])]

    checksums = {}
    ok = True
    for name, spec in items:
        got = download(name, spec)
        ok = ok and got
        dest = REPO / spec["dest"]
        if got and dest.exists():
            checksums[name] = sha256(dest)

    if args.write_checksums and checksums:
        out = ROOT / "models" / "checksums.json"
        out.write_text(json.dumps(checksums, indent=2))
        print(f"wrote {out}")

    # Not an error if some models require manual placement; just report.
    return 0


if __name__ == "__main__":
    sys.exit(main())
