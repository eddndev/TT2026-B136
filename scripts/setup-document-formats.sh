#!/usr/bin/env bash
# Verify and provision the pinned native parser without system installation.
set -euo pipefail
exec python3 -B "$(dirname -- "${BASH_SOURCE[0]}")/setup-document-formats.py" "$@"
