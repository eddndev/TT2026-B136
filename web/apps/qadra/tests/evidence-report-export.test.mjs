import test from 'node:test';
import assert from 'node:assert/strict';
import { exportReport, reportFilename } from '../src/lib/evidence/report-export.mjs';
import { HISTORICAL_NOTE } from '../src/lib/evidence/messages.mjs';

const zipText = 'PK-ZIP-BYTES-THAT-MUST-NOT-LEAVE';
const report = {
  engine: { name: 'evidence-verify', version: '0.1.0', source_digest: 'f'.repeat(64) },
  profile: 'server-seal-v1',
  overall: 'checked',
  components: [{ kind: 'package', outcome: 'checked', findings: [] }],
  instants: [{ label: 'current status', unix: 1, source: 'caller' }],
  digests: [{ name: 'contrato.pdf', sha256: 'e'.repeat(64) }],
  trust: [],
  current_status: [{ outcome: 'checked', code: 'current.instant', detail: '1' }],
};
const policy = {
  pinned: [{ fingerprint: 'a'.repeat(64), purposes: ['server-seal'], label: 'Qadra' }],
  served: { roots: [{ der_base64: 'Uk9PVA==', purposes: ['server-seal'] }], crls_base64: [] },
  admitted_policies: ['1.3.6.1.4.1.13762.3'],
  current_instant: 1793145600,
};
const route = { 'server-seal': { root_der_base64: 'Uk9PVA==' } };
const input = {
  report,
  engine: report.engine,
  policy,
  pinned: policy.pinned,
  route,
  document: { id: '78ac67b1-ab36-49ea-9b08-f951f341f081', version: 3, name: 'contrato.pdf' },
  file: { name: 'evidencia.zip', size: 1234, sha256: 'c'.repeat(64), bytes: zipText },
  generatedAt: new Date('2026-10-28T12:00:00Z'),
};

test('the exported report holds every key of the format', () => {
  const value = exportReport(input);
  assert.deepEqual(Object.keys(value), [
    'format',
    'generated_at',
    'engine',
    'document',
    'file',
    'profile',
    'overall',
    'policy',
    'anchors',
    'instants',
    'digests',
    'trust',
    'components',
    'current_status',
    'note',
  ]);
  assert.equal(value.format, 'qadra-evidence-validation/1');
  assert.equal(value.generated_at, '2026-10-28T12:00:00.000Z');
  assert.deepEqual(value.engine, report.engine);
  assert.deepEqual(value.document, { id: input.document.id, version: 3 });
  assert.deepEqual(value.file, { name: 'evidencia.zip', size: 1234, sha256: 'c'.repeat(64) });
  assert.equal(value.profile, 'server-seal-v1');
  assert.equal(value.overall, 'checked');
  assert.deepEqual(value.policy, {
    admitted_policies: ['1.3.6.1.4.1.13762.3'],
    current_instant: 1793145600,
    clock: 'client',
  });
  assert.deepEqual(value.anchors, { pinned: policy.pinned, route });
  assert.deepEqual(value.instants, report.instants);
  assert.deepEqual(value.digests, report.digests);
  assert.deepEqual(value.trust, report.trust);
  assert.deepEqual(value.components, report.components);
  assert.deepEqual(value.current_status, report.current_status);
  assert.equal(value.note, HISTORICAL_NOTE);
});

test('an unavailable route is exported as null and the ZIP bytes are never included', () => {
  const value = exportReport({ ...input, route: null });
  assert.equal(value.anchors.route, null);
  const text = JSON.stringify(value);
  assert.ok(!text.includes(zipText));
  assert.ok(!text.includes('"bytes"'));
});

test('the exported values are copies, not the live report', () => {
  const value = exportReport(input);
  value.components[0].outcome = 'failed';
  assert.equal(report.components[0].outcome, 'checked');
});

test('the download name carries the document and its version', () => {
  assert.equal(
    reportFilename({ id: '78ac67b1-ab36-49ea-9b08-f951f341f081', version: 3 }),
    'validacion-evidencia-78ac67b1-ab36-49ea-9b08-f951f341f081-v3.json',
  );
});
