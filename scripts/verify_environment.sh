#!/usr/bin/env bash
# Print the environment record required for any benchmark report.
# Fields that cannot be read on this host print "unknown" rather than guessing.
set -uo pipefail
echo "== FusionServe environment =="
echo "date:            $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "os:              $(uname -sr)"
echo "cpu:             $(sysctl -n machdep.cpu.brand_string 2>/dev/null || lscpu 2>/dev/null | grep 'Model name' | head -1 | cut -d: -f2 | xargs || echo unknown)"
echo "ram:             $( (sysctl -n hw.memsize 2>/dev/null | awk '{print $1/1073741824" GiB"}') || (free -h 2>/dev/null | awk '/Mem/{print $2}') || echo unknown)"
if command -v nvidia-smi >/dev/null 2>&1; then
  echo "gpu:             $(nvidia-smi --query-gpu=name --format=csv,noheader | head -1)"
  echo "gpu_memory:      $(nvidia-smi --query-gpu=memory.total --format=csv,noheader | head -1)"
  echo "nvidia_driver:   $(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1)"
else
  echo "gpu:             none (nvidia-smi not found)"
fi
command -v nvcc >/dev/null 2>&1 && echo "cuda:            $(nvcc --version | grep release | sed 's/.*release //')" || echo "cuda:            unknown"
echo "docker:          $(docker --version 2>/dev/null || echo unknown)"
echo "cargo:           $(cargo --version 2>/dev/null || echo unknown)"
