#!/usr/bin/env bash
# SSH credentials exist only for this job and the server key is pinned beforehand.
set -euo pipefail
umask 077
: "${DEPLOY_SSH_KEY:?missing QADRA_SSH_KEY}"
: "${DEPLOY_KNOWN_HOSTS:?missing QADRA_KNOWN_HOSTS}"
: "${DEPLOY_HOST:?missing QADRA_HOST}"
: "${DEPLOY_PORT:?missing QADRA_PORT}"
: "${DEPLOY_USER:?missing QADRA_USER}"
: "${DEPLOY_ROOT:?missing QADRA_ROOT}"
: "${DEPLOY_CONTROLLER_SHA256:?missing QADRA_CONTROLLER_SHA256}"
python3 -B - <<'PY'
import os, re, sys
sys.path.insert(0, 'ops/deploy')
from bundle import validate_version, COMMIT
from host import validate_root
validate_version(os.environ['VERSION'])
assert COMMIT.fullmatch(os.environ['SOURCE_COMMIT'])
assert re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.-]*', os.environ['DEPLOY_HOST'])
assert re.fullmatch(r'[a-z_][a-z0-9_-]*', os.environ['DEPLOY_USER'])
assert os.environ['DEPLOY_PORT'].isdigit() and 1 <= int(os.environ['DEPLOY_PORT']) <= 65535
validate_root(os.environ['DEPLOY_ROOT'])
assert re.fullmatch(r'[0-9a-f]{64}', os.environ['DEPLOY_CONTROLLER_SHA256'])
PY
ssh_dir=$(mktemp -d "${RUNNER_TEMP:-/tmp}/qadra-ssh.XXXXXX")
trap 'rm -rf -- "$ssh_dir"' EXIT
printf '%s\n' "$DEPLOY_SSH_KEY" > "$ssh_dir/key"
printf '%s\n' "$DEPLOY_KNOWN_HOSTS" > "$ssh_dir/known_hosts"
unset DEPLOY_SSH_KEY DEPLOY_KNOWN_HOSTS
ssh-keygen -y -P '' -f "$ssh_dir/key" > "$ssh_dir/public"
grep -q '^ssh-ed25519 ' "$ssh_dir/public"
connection="$DEPLOY_USER@$DEPLOY_HOST"
options=(-i "$ssh_dir/key" -o IdentitiesOnly=yes -o BatchMode=yes
  -o StrictHostKeyChecking=yes -o "UserKnownHostsFile=$ssh_dir/known_hosts"
  -o HostKeyAlgorithms=ssh-ed25519 -o ConnectTimeout=15 -o ServerAliveInterval=15)
artifact="qadra-$VERSION-$SOURCE_COMMIT.tar.gz"
local_file="output/releases/$artifact"
(cd output/releases && sha256sum --check "$artifact.sha256")
checksum=$(sha256sum "$local_file" | cut -d ' ' -f 1)
transfer=$(openssl rand -hex 16)
remote_file="$DEPLOY_ROOT/incoming/$transfer.tar.gz"
ssh "${options[@]}" -p "$DEPLOY_PORT" "$connection" \
  "test -f '$DEPLOY_ROOT/controller_launcher.py' && test -d '$DEPLOY_ROOT/incoming'"
scp "${options[@]}" -P "$DEPLOY_PORT" "$local_file" "$connection:$remote_file"
ssh "${options[@]}" -p "$DEPLOY_PORT" "$connection" \
  "umask 077; /usr/bin/python3 -I -B -S '$DEPLOY_ROOT/controller_launcher.py' --root '$DEPLOY_ROOT' --inventory-sha256 '$DEPLOY_CONTROLLER_SHA256' --entrypoint release.py -- --root '$DEPLOY_ROOT' activate '$remote_file' '$checksum' '$VERSION' '$SOURCE_COMMIT'"
printf 'Deployed %s at commit %s\n' "$VERSION" "$SOURCE_COMMIT" >> "$GITHUB_STEP_SUMMARY"
