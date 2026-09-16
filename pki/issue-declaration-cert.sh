#!/usr/bin/env bash
# Issue an internal signing-only certificate with no authentication/email EKU.
# The subject key remains in the local CA directory; only public certificates
# and detached signatures may be submitted to participant workflows.

set -euo pipefail

[ "$#" -eq 1 ] || {
    printf 'usage: issue-declaration-cert.sh COMMON_NAME\n' >&2
    exit 1
}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec bash "$SCRIPT_DIR/issue-cert.sh" "$1" --internal-declaration
