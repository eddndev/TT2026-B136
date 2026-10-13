import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const digest = 'a'.repeat(64);
const bytes = new Uint8Array([0, 255, 1, 13, 10, 128]);
const path = `/api/v1/cases/${caseId}/documents/${id}/versions/3/content`;
const headers = () => ({
  'Content-Type': 'application/octet-stream',
  'Content-Disposition': 'attachment; filename="original-v3.bin"',
  'Cache-Control': 'no-store',
  'X-Content-Type-Options': 'nosniff',
  'X-Case-Id': caseId,
  'X-Document-Id': id,
  'X-Document-Version': '3',
  'X-Document-Digest': digest,
});
const response = (overrides = {}) =>
  new Response(bytes, { headers: { ...headers(), ...overrides } });
const exact = (api) => api.caseDocuments(caseId).version(id, 3);
function deferred() {
  let resolve;
  const promise = new Promise((finish) => (resolve = finish));
  return { promise, resolve };
}

test('content requests the selected exact version and preserves arbitrary bytes without sealing', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return response();
  });
  const result = await exact(api).content(digest);
  assert.deepEqual(new Uint8Array(await result.blob.arrayBuffer()), bytes);
  assert.equal(result.documentId, id);
  assert.equal(result.version, '3');
  assert.equal(result.digest, digest);
  assert.equal(result.blob.type, 'application/octet-stream');
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, path);
  assert.equal(calls[0].method, 'GET');
  assert.equal(calls[0].cache, 'no-store');
  assert.equal(calls[0].redirect, 'error');
  assert.equal(calls[0].body, undefined);
});

test('content rejects foreign identity, noncanonical versions and unbound digests', async () => {
  const invalid = [
    ['X-Case-Id', 'cccccccc-cccc-4ccc-8ccc-cccccccccccc'],
    ['X-Document-Id', 'dddddddd-dddd-4ddd-8ddd-dddddddddddd'],
    ['X-Document-Version', '4'],
    ['X-Document-Version', '03'],
    ['X-Document-Version', '+3'],
    ['X-Document-Version', '3.0'],
    ['X-Document-Digest', 'b'.repeat(64)],
    ['X-Document-Digest', digest.toUpperCase()],
    ['X-Document-Digest', 'not-a-digest'],
  ];
  for (const [name, value] of invalid) {
    const api = createApi(async () => response({ [name]: value }));
    await assert.rejects(
      () => exact(api).content(digest),
      /respuesta|contenido|versi\u00f3n/i,
      name,
    );
  }
});

test('content rejects missing binding headers and a JSON body presented as success', async () => {
  for (const name of ['X-Case-Id', 'X-Document-Id', 'X-Document-Version', 'X-Document-Digest']) {
    const values = headers();
    delete values[name];
    const api = createApi(async () => new Response(bytes, { headers: values }));
    await assert.rejects(
      () => exact(api).content(digest),
      /respuesta|contenido|versi\u00f3n/i,
      name,
    );
  }
  const api = createApi(async () => response({ 'Content-Type': 'application/json' }));
  await assert.rejects(() => exact(api).content(digest), /respuesta|contenido|archivo/i);
});

test('content requires an exact valid expected digest before requesting bytes', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return response();
  });
  for (const value of [undefined, null, '', 'short', digest.toUpperCase()])
    await assert.rejects(() => exact(api).content(value));
  assert.equal(calls, 0);
});

test('validation failure is typed and distinct from service unavailability without automatic retries', async () => {
  for (const [status, code] of [
    [409, 'document_content_validation_failed'],
    [503, 'server_busy'],
  ]) {
    let calls = 0;
    const api = createApi(async () => {
      calls++;
      return Response.json({ error: { code } }, { status });
    });
    await assert.rejects(
      () => exact(api).content(digest),
      (error) => {
        assert.equal(error.status, status);
        assert.equal(error.code, code);
        if (status === 409)
          assert.match(error.message, /validaci\u00f3n.*fall|fall.*validaci\u00f3n/i);
        else assert.doesNotMatch(error.message, /alteraci\u00f3n|validaci\u00f3n.*fall/i);
        return true;
      },
    );
    assert.equal(calls, 1);
  }
});

for (const scope of ['version', 'case', 'session']) {
  test(`content discards a late response after its ${scope} is closed`, async () => {
    const pending = deferred();
    const api = createApi(async (url) =>
      url.endsWith('/logout') ? new Response(null, { status: 204 }) : pending.promise,
    );
    const documents = api.caseDocuments(caseId);
    const selected = documents.version(id, 3);
    const request = selected.content(digest);
    const rejected = assert.rejects(request, /versi\u00f3n|expediente|sesi\u00f3n/i);
    if (scope === 'version') selected.dispose();
    else if (scope === 'case') documents.dispose();
    else await api.logout();
    pending.resolve(response());
    await rejected;
  });
}

test('content checks scope again after a delayed binary body has been read', async () => {
  const started = deferred(),
    body = deferred();
  const api = createApi(async () => {
    const result = response();
    result.blob = () => {
      started.resolve();
      return body.promise;
    };
    return result;
  });
  const selected = exact(api);
  const request = selected.content(digest);
  const rejected = assert.rejects(request, /versi\u00f3n/i);
  await started.promise;
  selected.dispose();
  body.resolve(new Blob([bytes], { type: 'application/octet-stream' }));
  await rejected;
});

test('content preserves access denials and expires the session on 401', async () => {
  for (const status of [401, 403, 404]) {
    let expired = 0;
    const code = { 401: 'invalid_session', 403: 'permission_denied', 404: 'document_not_found' }[
      status
    ];
    const api = createApi(
      async () => Response.json({ error: { code } }, { status }),
      () => expired++,
    );
    await assert.rejects(
      () => exact(api).content(digest),
      (error) => error.status === status,
    );
    assert.equal(expired, status === 401 ? 1 : 0);
  }
});
