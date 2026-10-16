import { expect } from '@playwright/test';

export const csr = '-----BEGIN CERTIFICATE REQUEST-----\nYWJj\n-----END CERTIFICATE REQUEST-----';
const pem = '-----BEGIN CERTIFICATE-----\nYWJj\n-----END CERTIFICATE-----';
const operator = { id: '171e1bf7-c543-4b73-826e-d972ac894119', email: 'ca@example.test' };
const root = 'a'.repeat(64), fingerprint = 'b'.repeat(64);

// Browser contracts use synthetic public material. Native crypto is accepted
// separately by the CA service and executor integration tests.
export async function portalFixture(page, { rejectSavedRequest = false } = {}) {
  const history = [], calls = [];
  let issued = false, revoked = false, published = false, lost = true;
  let saved;
  const start = Date.now();
  const status = () => {
    const now = Date.now();
    return { operator, policy: { absolute_ttl_seconds: 300, idle_ttl_seconds: 120 },
      server_now_unix_ms: now, absolute_expires_at_unix_ms: start + 300000,
      idle_expires_at_unix_ms: Math.min(start + 300000, now + 120000) };
  };
  await page.route('**/api/v1/**', async (route) => {
    const request = route.request(), path = new URL(request.url()).pathname;
    const reply = (value) => route.fulfill({ json: value });
    if (path.endsWith('/auth/login')) return reply({ challenge_token: 'browser-challenge', expires_in_seconds: 300 });
    if (path.endsWith('/mfa/totp')) return reply({ ...status(), access_token: 'browser-bearer',
      token_type: 'Bearer', expires_in_seconds: 120 });
    expect(request.headers().authorization).toBe('Bearer browser-bearer');
    if (path.endsWith('/auth/logout')) return route.fulfill({ status: 204 });
    if (path.endsWith('/auth/activity') || path.endsWith('/auth/session')) return reply(status());
    expect(path).toBe('/api/v1/ca/operations');
    const command = request.postDataJSON();
    expect(command.operator_id).toBeUndefined();
    calls.push(command);
    if (command.action === 'preview') return reply({ subject: 'CN=Titular Demo',
      csr_sha256: 'c'.repeat(64), spki_sha256: 'd'.repeat(64), root_fingerprint: root, purpose: command.purpose });
    if (command.action === 'issue') {
      expect(issued).toBe(false);
      issued = true;
      saved = { ...command, serial: '1000', fingerprint, certificate_pem: pem,
        recorded_at: 1800000000, operator_id: operator.id };
      history.push({ ...saved, kind: 'issue', state: rejectSavedRequest ? 'pending' : 'completed',
        ...(rejectSavedRequest ? { serial: null, fingerprint: null } : {}) });
      if (lost) { lost = false; return route.abort('failed'); }
      return reply(saved);
    }
    if (command.action === 'resume-issue') {
      expect(command.operation_id).toBe(saved.operation_id);
      if (rejectSavedRequest) {
        const rejection = { state: 'rejected', reason: 'csr_subject_policy',
          operation_id: saved.operation_id, operator_id: operator.id,
          recorded_at: saved.recorded_at, rejected_at: saved.recorded_at + 1,
          csr_sha256: 'c'.repeat(64), root_fingerprint: root, reserved_serial: '1000',
          holder: saved.holder, reference: saved.reference, criterion: saved.criterion,
          purpose: saved.purpose };
        history[0] = { ...history[0], state: 'rejected', reason: rejection.reason };
        return reply(rejection);
      }
      return reply(saved);
    }
    if (command.action === 'certificate') return reply({ serial: '1000', fingerprint, certificate_pem: pem });
    if (command.action === 'inventory') return reply({ ...command, total: issued ? 1 : 0,
      items: issued ? [{ serial: '1000', fingerprint, root_fingerprint: root, subject: 'CN=Titular Demo',
        purpose: 'partner', not_before: 1700000000, not_after: 1900000000, validity: 'valid',
        local_status: revoked ? 'revoked' : 'issued',
        published_status: published ? 'revoked' : revoked ? 'pending' : 'not-revoked' }] : [] });
    if (command.action === 'revoke') {
      revoked = true;
      const receipt = { ...command, recorded_at: 1800000001, operator_id: operator.id,
        local_status: 'revoked', published_status: 'pending' };
      history.push({ ...receipt, kind: 'revoke', state: 'completed' });
      return reply(receipt);
    }
    if (command.action === 'status') return reply({ revision: 3, root_fingerprint: root,
      crl_digest: 'e'.repeat(64), crl_number: 4096, valid_from: 1700000000, valid_until: 1900000000,
      checked_at: 1800000000, valid_now: true, maintenance_pending: false, revoked_serials: [] });
    if (command.action === 'renew-crl') {
      expect(command.acknowledge_maintenance).toBe(true);
      published = true;
      const receipt = { ...command, revision: 4, crl_number: 4097, root_fingerprint: root,
        operator_id: operator.id, recorded_at: 1800000002, state: 'published' };
      history.push({ ...receipt, kind: 'publication', state: 'completed' });
      return reply(receipt);
    }
    if (command.action === 'history') return reply({ ...command, total: history.length, items: [...history].reverse() });
    throw new Error(`Unexpected CA browser command: ${command.action}`);
  });
  return calls;
}
