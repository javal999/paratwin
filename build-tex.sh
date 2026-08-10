#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# tectonic is not vendored here. Take it from $TECTONIC if set, else from PATH.
TECTONIC="${TECTONIC:-$(command -v tectonic || true)}"

if [ -z "$TECTONIC" ] || [ ! -x "$TECTONIC" ]; then
  echo "error: tectonic not found." >&2
  echo "  Set TECTONIC=/path/to/tectonic, or install it and put it on PATH." >&2
  echo "  Example: TECTONIC=/path/to/tectonic $0" >&2
  exit 1
fi

mkdir -p "$PROJECT_DIR/tex/output"
"$TECTONIC" "$PROJECT_DIR/tex/main.tex" --outdir "$PROJECT_DIR/tex/output"

echo "Compiled PDF at: $PROJECT_DIR/tex/output/main.pdf"
