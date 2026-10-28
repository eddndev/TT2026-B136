// Runs the generated WebAssembly module in Node. It requires the module
// built by bash scripts/build-evidence-verify.sh, as `npm run dev` does.
import test, { before } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { handleValidation } from '../src/lib/evidence-verify-run.mjs';
import { buildPolicy } from '../src/lib/evidence/policy.mjs';
import { parsePinnedRoots } from '../src/lib/evidence/pinned-roots-schema.mjs';

const generated = new URL('../src/generated/evidence-verify/', import.meta.url);
const pinned = parsePinnedRoots(
  JSON.parse(readFileSync(new URL('../src/lib/evidence/pinned-roots.json', import.meta.url))),
);
let validate;
let manifest;

before(async () => {
  const glue = new URL('evidence_verify_wasm.js', generated);
  assert.ok(existsSync(glue), 'Run bash scripts/build-evidence-verify.sh first.');
  const module = await import(glue.href);
  module.initSync({ module: readFileSync(new URL('evidence_verify_wasm_bg.wasm', generated)) });
  validate = module.validate;
  manifest = JSON.parse(readFileSync(new URL('manifest.json', generated), 'utf8'));
});

async function run(zip, policy) {
  const posted = [];
  await handleValidation(
    { op: 1, zip, policy },
    { load: async () => {}, validate, sourceDigest: manifest.source, post: (m) => posted.push(m) },
  );
  return posted.at(-1);
}

test('a file that is not a ZIP is reported as not supported by the package check', async () => {
  const policy = buildPolicy({ pinned, trust: null, now: Date.now() });
  const reply = await run(new TextEncoder().encode('not a zip').buffer, policy);
  assert.equal(reply.ok, true);
  const { report } = reply;
  assert.equal(report.overall, 'not-supported');
  assert.equal(report.profile, null);
  assert.equal(report.engine.name, 'evidence-verify');
  assert.match(report.engine.version, /^\d+\.\d+\.\d+$/);
  assert.equal(report.engine.source_digest, manifest.source);
  assert.equal(report.components[0].kind, 'package');
  assert.match(report.components[0].findings[0].code, /^zip\./);
});

test('the policy built from a trust route is accepted by the engine', async () => {
  const trust = {
    'server-seal': { root_der_base64: 'MAA=', crl_der_base64: 'MAA=' },
    'time-stamping': { roots: [{ root_der_base64: 'MAA=' }] },
    'individual-signing': null,
  };
  const policy = buildPolicy({ pinned, trust, now: Date.now() });
  const reply = await run(new ArrayBuffer(0), policy);
  assert.equal(reply.ok, true);
  assert.equal(reply.report.overall, 'not-supported');
});

test('a rejected policy is reported with a fixed code', async () => {
  assert.throws(
    () => validate(new Uint8Array(0), JSON.stringify({ pinned: [{ fingerprint: 'AB' }] })),
    (error) => error === 'policy.fingerprint',
  );
  const policy = { pinned: [], served: null, current_instant: 'now' };
  assert.deepEqual(await run(new ArrayBuffer(0), policy), {
    op: 1,
    ok: false,
    error: 'policy_invalid',
  });
});
