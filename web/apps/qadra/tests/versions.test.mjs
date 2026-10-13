import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';

test('version history and append preserve cursor and expected version in case paths', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json({});
  });
  const documents = api.caseDocuments(caseId);
  await documents.versions(id, { limit: 50, beforeVersion: 8 });
  const file = new Blob(['version bytes']);
  await documents.append(id, 8, file, 'version-nine.txt');
  assert.equal(
    calls[0].url,
    `/api/v1/cases/${caseId}/documents/${id}/versions?limit=50&before_version=8`,
  );
  assert.equal(calls[1].url, `/api/v1/cases/${caseId}/documents/${id}/versions?expected_version=8`);
  assert.equal(calls[1].method, 'POST');
  assert.equal(calls[1].body, file);
  assert.equal(calls[1].headers['X-Document-Name'], 'version-nine.txt');
});

test('selected version scope binds detail, seal, verify and evidence to the exact number', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json(
      { id, version: 2 },
      { headers: { 'X-Document-Id': id, 'X-Document-Version': '2' } },
    );
  });
  const selected = api.caseDocuments(caseId).version(id, 2);
  await selected.detail();
  await selected.seal();
  await selected.verify();
  await selected.evidence();
  const base = `/api/v1/cases/${caseId}/documents/${id}/versions/2`;
  assert.deepEqual(
    calls.map((call) => call.url),
    [base, `${base}/seal`, `${base}/verify`, `${base}/evidence`],
  );
  assert.deepEqual(
    calls.map((call) => call.method),
    ['GET', 'POST', 'POST', 'GET'],
  );
});

test('a discarded version scope rejects a late archive before it can be downloaded', async () => {
  let release;
  const api = createApi(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const selected = api.caseDocuments(caseId).version(id, 1);
  const pending = selected.evidence();
  selected.dispose();
  release(new Response('historical zip'));
  await assert.rejects(pending, /versi\u00f3n/);
});

test('append conflict is returned once and never retried automatically', async () => {
  let calls = 0;
  const api = createApi(async () => {
    calls++;
    return Response.json({ error: { code: 'document_version_conflict' } }, { status: 409 });
  });
  await assert.rejects(
    api.caseDocuments(caseId).append(id, 1, new Blob(['bytes']), 'new.txt'),
    (error) => error.status === 409,
  );
  assert.equal(calls, 1);
});

test('verification rejects a report belonging to another document version', async () => {
  const api = createApi(async () => Response.json({ id, version: 9, verdict: 'valid' }));
  await assert.rejects(
    api.caseDocuments(caseId).version(id, 1).verify(),
    /versi\u00f3n seleccionada/,
  );
});

test('historical evidence rejects an archive labeled as another version', async () => {
  const api = createApi(
    async () =>
      new Response('archive', { headers: { 'X-Document-Id': id, 'X-Document-Version': '2' } }),
  );
  await assert.rejects(
    api.caseDocuments(caseId).version(id, 1).evidence(),
    /versi\u00f3n seleccionada/,
  );
});

test('version precondition and exhaustion errors explain their distinct recovery paths', async () => {
  for (const [code, message] of [
    ['document_version_required', /Selecciona una versi\u00f3n/],
    ['document_version_exhausted', /alcanzado el l\u00edmite de versiones/],
  ]) {
    const api = createApi(async () => Response.json({ error: { code } }, { status: 409 }));
    await assert.rejects(
      api.caseDocuments(caseId).append(id, 1, new Blob(['bytes']), 'new.txt'),
      message,
    );
  }
});
