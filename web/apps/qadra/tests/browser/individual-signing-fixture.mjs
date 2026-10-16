import { setup, caseId, id, document, validReport } from './helpers.mjs';
import { absoluteSession } from '../fixtures/session.mjs';
import { createHash } from 'node:crypto';
import {
  accountId,
  binding,
  consent,
  pdf,
  digest,
  signature,
  certificate,
} from '../individual-signing-fixture.mjs';
export { pdf, digest, signature, certificate };
export async function signingSetup(page, role = 'litigator', material = null) {
  await setup(page, role, [{ ...document, digest }]);
  const user = { id: accountId, email: 'litigator@example.test', role };
  const state = {
    writes: [],
    current: null,
    receipt: null,
    lostBinding: false,
    lostDocument: false,
  };
  await page.route('**/api/v1/auth/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith('/me')) return route.fulfill({ json: user });
    if (path.includes('/mfa/'))
      return route.fulfill({ json: absoluteSession(user, 'signing-token') });
    return route.fallback();
  });
  await page.route('**/api/v1/identity/signing-certificates/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    const parts = path.split('/'),
      action = parts.at(-1);
    const bindingId = ['prepare', 'register', 'withdraw'].includes(action) ? parts.at(-2) : action;
    if (action === 'current') return route.fulfill({ json: { binding: state.current } });
    const value = binding(action !== 'prepare');
    value.binding_id = bindingId;
    const bytes = Buffer.from(value.statement_base64, 'base64');
    Buffer.from(bindingId.replaceAll('-', ''), 'hex').copy(bytes, 94);
    if (material) {
      Buffer.from(material.fingerprint, 'hex').copy(bytes, 110);
      value.certificate.der_base64 = material.certificate.toString('base64');
      value.certificate.fingerprint = material.fingerprint;
    }
    value.statement_base64 = bytes.toString('base64');
    if (route.request().method() === 'POST')
      state.writes.push({ path, body: route.request().postData() });
    if (action === 'prepare') return route.fulfill({ json: value });
    if (action === 'register') {
      if (material) value.signature_base64 = route.request().postDataJSON().signature_base64;
      state.current = value;
      if (state.lostBinding) {
        state.lostBinding = false;
        return route.abort();
      }
    }
    if (action === 'withdraw') {
      state.current = { ...value, withdrawn_at: '2026-10-10T01:00:00Z' };
      return route.fulfill({ json: state.current });
    }
    if (!state.current)
      return route.fulfill({ status: 404, json: { error: { code: 'signing_not_found' } } });
    return route.fulfill({ json: state.current });
  });
  await page.route('**/api/v1/cases/**/content', (route) =>
    route.fulfill({
      body: pdf,
      contentType: 'application/octet-stream',
      headers: {
        'X-Case-Id': caseId,
        'X-Document-Id': id,
        'X-Document-Version': '1',
        'X-Document-Digest': digest,
      },
    }),
  );
  await page.route('**/api/v1/cases/**/individual-signing/**', async (route) => {
    const request = route.request(),
      path = new URL(request.url()).pathname;
    if (path.endsWith('/verification'))
      return route.fulfill({
        json: {
          kind: state.receipt ? 'individual' : 'legacy_server',
          historical: { ...validReport, document_digest: digest },
          certificate_current: {
            status: state.receipt ? 'rejected' : 'not_individual',
            reason: state.receipt ? 'certificate_revoked' : null,
            checked_at_unix: 2000000000,
          },
          binding_withdrawn_at: state.receipt ? '2026-10-10T01:00:00Z' : null,
          signing: state.receipt,
        },
      });
    if (path.includes('/receipts/'))
      return route.fulfill(
        state.receipt
          ? { json: state.receipt }
          : { status: 404, json: { error: { code: 'signing_not_found' } } },
      );
    const data = request.postDataJSON();
    state.writes.push({ path, body: request.postData() });
    if (path.endsWith('/prepare')) {
      const value = consent();
      value.case_id = caseId;
      value.document_id = id;
      value.version = 1;
      value.name = document.name;
      value.operation_id = data.operation_id;
      value.binding_id = state.current?.binding_id || value.binding_id;
      const bytes = Buffer.from(value.consent_base64, 'base64');
      for (const [uuid, offset] of [
        [value.binding_id, 94],
        [value.operation_id, 142],
        [caseId, 158],
        [id, 174],
      ])
        Buffer.from(uuid.replaceAll('-', ''), 'hex').copy(bytes, offset);
      bytes.writeUInt32BE(1, 190);
      if (material) {
        value.certificate_fingerprint = material.fingerprint;
        Buffer.from(material.fingerprint, 'hex').copy(bytes, 110);
        value.pdf_signature_digest = createHash('sha256')
          .update(Buffer.from(data.pdf_signature_base64, 'base64'))
          .digest('hex');
        Buffer.from(value.pdf_signature_digest, 'hex').copy(bytes, 226);
      }
      value.consent_base64 = bytes.toString('base64');
      state.prepared = value;
      return route.fulfill({ json: value });
    }
    state.receipt = { ...state.prepared, status: 'accepted', accepted_at: '2026-10-10T00:00:00Z' };
    delete state.receipt.name;
    if (state.lostDocument) {
      state.lostDocument = false;
      return route.abort();
    }
    return route.fulfill({ json: state.receipt });
  });
  return state;
}
