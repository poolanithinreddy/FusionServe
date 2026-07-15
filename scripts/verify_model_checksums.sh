#!/usr/bin/env bash
# Verify downloaded model artifacts against models/checksums.json.
set -euo pipefail
CK="models/checksums.json"
[[ -f "$CK" ]] || { echo "no $CK; run download_models.py --write-checksums" >&2; exit 1; }
python3 - "$CK" <<'PY'
import hashlib, json, sys
from pathlib import Path
ck = json.loads(Path(sys.argv[1]).read_text())
repo = Path("models/triton")
manifest = {
  "resnet50_onnx": repo/"resnet50_onnx/1/model.onnx",
  "text_embedding": repo/"text_embedding/1/model.onnx",
}
bad = 0
for name, want in ck.items():
    p = manifest.get(name)
    if not p or not p.exists():
        print(f"[miss] {name}: {p}"); bad += 1; continue
    h = hashlib.sha256(p.read_bytes()).hexdigest()
    ok = h == want
    print(f"[{'ok  ' if ok else 'FAIL'}] {name}")
    bad += 0 if ok else 1
sys.exit(1 if bad else 0)
PY
