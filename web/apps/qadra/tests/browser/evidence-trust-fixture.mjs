// The response of GET /api/v1/evidence/trust used by the mocked browser
// tests, built from the public test PKI of crates/evidence-verify so that
// the real engine accepts its roots and CRL.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const fixtures = new URL('../../../../../crates/evidence-verify/tests/fixtures/', import.meta.url);
const read = (name) => readFileSync(new URL(name, fixtures));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

const root = read('root.der');
const crl = read('root.crl.der');

export const evidenceTrust = {
  'server-seal': {
    root_der_base64: root.toString('base64'),
    root_fingerprint: sha256(root),
    crl_der_base64: crl.toString('base64'),
    crl_digest: sha256(crl),
  },
  'time-stamping': {
    roots: [{ root_der_base64: root.toString('base64'), root_fingerprint: sha256(root) }],
  },
  'individual-signing': null,
};
