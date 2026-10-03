import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const command = fileURLToPath(new URL('../../scripts/web-demo.sh', import.meta.url));

function invoke(mode, args = [], extra = {}) {
  const directory = mkdtempSync(join(tmpdir(), 'session-demo-command-'));
  try {
    // Intercept backend delegation; these command-contract tests start no services.
    writeFileSync(join(directory, 'bash'), '#!/bin/sh\nexit 42\n', { mode: 0o700 });
    writeFileSync(join(directory, 'node'), '#!/bin/sh\nexit 0\n', { mode: 0o700 });
    return spawnSync('/bin/bash', [command, ...args], {
      env: {
        ...process.env,
        PATH: `${directory}:${process.env.PATH}`,
        TT_WEB_SESSION_ACCEPTANCE: mode,
        TT_WEB_LIVE_SHARD: '',
        ...extra,
      },
      encoding: 'utf8',
      timeout: 5000,
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

test('the regular and isolated session campaigns delegate to disposable backends', () => {
  for (const mode of ['0', '1']) {
    const result = invoke(mode, ['--workers=1']);
    assert.equal(result.error, undefined);
    assert.equal(result.status, 42);
  }
});

test('the session acceptance cannot silently select an empty command-line shard', () => {
  for (const args of [['--shard=2/3'], ['--shard', '2/3']]) {
    const result = invoke('1', args);
    assert.equal(result.status, 2);
    assert.match(result.stderr, /session acceptance requires its own disposable backend campaign/);
  }
});

test('session acceptance rejects fixture partitioning and unsupported mode values', () => {
  assert.equal(invoke('1', [], { TT_WEB_LIVE_SHARD: '1/3' }).status, 2);
  assert.equal(invoke('invalid').status, 2);
});
