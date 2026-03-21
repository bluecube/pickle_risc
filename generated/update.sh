#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$(cd "$(dirname "$0")" && pwd)"

libreoffice --headless --convert-to csv "$REPO_ROOT/instruction_encoding.ods" --outdir "$OUT_DIR"
drawio --export --format svg --output "$OUT_DIR/block_diagram.svg" "$REPO_ROOT/block_diagram.drawio"
