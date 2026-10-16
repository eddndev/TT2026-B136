import test from 'node:test';
import assert from 'node:assert/strict';
import { portalFixture } from './browser/portal-fixture.mjs';
import { readSession } from '../src/lib/ca-access-values.mjs';

test('browser fixture session remains valid when the clock crosses a millisecond', async (t) => {
  let clock = 1_000_000;
  t.mock.method(Date, 'now', () => clock++);
  let handler;
  await portalFixture({ async route(_pattern, callback) { handler = callback; } });
  for (const path of ['mfa/totp', 'session', 'activity']) {
    let value;
    await handler({
      request: () => ({
        url: () => `http://localhost/api/v1/auth/${path}`,
        headers: () => ({ authorization: 'Bearer browser-bearer' }),
      }),
      fulfill: async (response) => { value = response.json; },
    });
    const admitted = readSession(value, { startedAt: 0, receivedAt: 1 });
    assert.equal(admitted.operator.email, 'ca@example.test');
    assert.equal(value.idle_expires_at_unix_ms - value.server_now_unix_ms, 120000);
    assert.equal(value.absolute_expires_at_unix_ms, 1_300_000);
  }
});
