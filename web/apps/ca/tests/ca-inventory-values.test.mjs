import test from 'node:test';
import assert from 'node:assert/strict';
import { pageValue, purposeLabel, publishedLabel, validDraft, previewValue,
  certificateValue } from '../src/lib/ca-operations-values.mjs';

const fingerprint = 'a'.repeat(64);
const item = { serial: '1000', fingerprint, subject: 'CN=Public Certificate',
  purpose: 'partner', root_fingerprint: 'b'.repeat(64), not_before: 10, not_after: 30,
  validity: 'valid', local_status: 'issued', published_status: 'unknown' };
const page = (value) => ({ items: [value], total: 1, offset: 0, limit: 20 });

test('inventory preserves each observed publication status independently from local status', () => {
  for (const status of ['unknown', 'not-revoked', 'pending', 'revoked']) {
    const row = pageValue(page({ ...item, published_status: status }), 'inventory', 0).items[0];
    assert.equal(row.published_status, status);
    assert.equal(row.local_status, 'issued');
    assert.ok(publishedLabel(status));
  }
  assert.equal(new Set(['unknown', 'not-revoked', 'pending', 'revoked'].map(publishedLabel)).size, 4);
  assert.throws(() => pageValue(page({ ...item, published_status: 'fully-valid' }), 'inventory', 0));
});

test('TSA is read-only public inventory with a distinct purpose label and exact download', () => {
  const tsa = { ...item, purpose: 'time-stamping' };
  assert.equal(pageValue(page(tsa), 'inventory', 0).items[0].purpose, 'time-stamping');
  assert.equal(purposeLabel(tsa.purpose), 'Sellado de tiempo');
  const certificate = { serial: item.serial, fingerprint,
    certificate_pem: '-----BEGIN CERTIFICATE-----\nPUBLIC TSA\n-----END CERTIFICATE-----\n' };
  assert.deepEqual(certificateValue(certificate, tsa), certificate);
});

test('TSA inventory support does not enable a new issue or preview purpose', () => {
  const csr = '-----BEGIN CERTIFICATE REQUEST-----\nPUBLIC\n-----END CERTIFICATE REQUEST-----\n';
  assert.equal(validDraft({ csr_pem: csr, purpose: 'time-stamping',
    holder: 'Holder', reference: 'Reference', criterion: 'Reviewed' }), false);
  assert.throws(() => previewValue({ csr_sha256: fingerprint, spki_sha256: fingerprint,
    root_fingerprint: fingerprint, subject: 'CN=TSA', purpose: 'time-stamping' }, 'time-stamping'));
});
