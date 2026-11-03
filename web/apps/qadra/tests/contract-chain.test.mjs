import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { contractsApi } from '../src/lib/contracts-api.mjs';
import { inputs, requestDetail, preparedStatement } from './contract-fixture.mjs';

const operation = '0e1d2c3b-4a59-4687-9a6b-5c4d3e2f1a0b';
const path = `/contract-requests/${inputs.request_id}/prepare`;
const sha = (value) => createHash('sha256').update(Buffer.from(value, 'base64')).digest('hex');

function prepare(detail, prepared = preparedStatement(2, detail, operation)) {
  const request = async (target) => {
    assert.equal(target, path);
    return prepared;
  };
  return contractsApi(request).requests.prepare(operation, {
    detail,
    position: 2,
    accountId: inputs.firm_account_id,
    fingerprint: detail.firm_certificate_fingerprint,
  });
}

function patchedStatement(prepared, offset, hex) {
  const bytes = Buffer.from(prepared.statement_base64, 'base64');
  Buffer.from(hex, 'hex').copy(bytes, offset);
  return { ...prepared, statement_base64: bytes.toString('base64') };
}

test('the firm statement chains to the stored Client signature and its token', async () => {
  const detail = requestDetail({ status: 'awaiting_firm', clientSignedAt: '2026-11-02T01:00:00Z' });
  const [client] = detail.signatures;
  const value = await prepare(detail);
  assert.equal(value.statement.priorSignatureDigest, sha(client.signature_base64));
  assert.equal(value.statement.priorTokenDigest, sha(client.timestamp_base64));
});

test('a firm statement over another Client signature or token is refused', async () => {
  const detail = requestDetail({ status: 'awaiting_firm', clientSignedAt: '2026-11-02T01:00:00Z' });
  const prepared = preparedStatement(2, detail, operation);
  await assert.rejects(prepare(detail, patchedStatement(prepared, 239, '11'.repeat(32))));
  await assert.rejects(prepare(detail, patchedStatement(prepared, 271, '22'.repeat(32))));
  const [client] = detail.signatures;
  const signature = Buffer.alloc(384, 9).toString('base64');
  await assert.rejects(
    prepare({ ...detail, signatures: [{ ...client, signature_base64: signature }] }),
  );
  await assert.rejects(
    prepare({ ...detail, signatures: [{ ...client, timestamp_base64: 'AAAA' }] }),
  );
});

test('a firm statement under another root or deployment than position 1 is refused', async () => {
  const detail = requestDetail({ status: 'awaiting_firm', clientSignedAt: '2026-11-02T01:00:00Z' });
  const prepared = preparedStatement(2, detail, operation);
  await assert.rejects(prepare(detail, patchedStatement(prepared, 26, '33'.repeat(32))));
  await assert.rejects(
    prepare(detail, patchedStatement(prepared, 10, '0123456789abcdef0123456789abcdef')),
  );
});

test('a firm statement without a stored position 1 signature is refused', async () => {
  const detail = requestDetail({ status: 'awaiting_firm', clientSignedAt: '2026-11-02T01:00:00Z' });
  await assert.rejects(prepare({ ...detail, signatures: [] }));
  const [client] = detail.signatures;
  await assert.rejects(prepare({ ...detail, signatures: [{ ...client, position: 2 }] }));
});
