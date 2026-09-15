import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';

const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const documentId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';

test('case queries, creation and membership use the persistent authorized contract', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json([]);
  });
  await api.cases({ limit: 51, offset: 50 });
  await api.createCase('Defensa inicial', 'NUC-123');
  await api.caseDetail(caseId);
  await api.assignMember(caseId, documentId);
  await api.removeMember(caseId, documentId);
  assert.equal(calls[0].url, '/api/v1/cases?limit=51&offset=50');
  assert.deepEqual(JSON.parse(calls[1].body), { title: 'Defensa inicial', reference: 'NUC-123' });
  assert.equal(calls[2].url, `/api/v1/cases/${caseId}`);
  assert.equal(calls[3].method, 'PUT');
  assert.equal(calls[4].method, 'DELETE');
  assert.equal(calls[3].url, `/api/v1/cases/${caseId}/members/${documentId}`);
});

test('document scope encodes server filters and binds every action to its case', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json({ documents: [], has_more: false });
  });
  const documents = api.caseDocuments(caseId);
  await documents.list({ limit: 50, offset: 100, name: 'acta & inicial', sealed: false });
  await documents.detail(documentId);
  const file = new Blob(['evidence']);
  await documents.upload(file, 'evidence.pdf');
  await documents.seal(documentId);
  await documents.verify(documentId);
  await documents.evidence(documentId);
  assert.equal(
    calls[0].url,
    `/api/v1/cases/${caseId}/documents?limit=50&offset=100&name=acta+%26+inicial&sealed=false`,
  );
  assert.equal(calls[1].method, 'GET');
  assert.equal(calls[1].url, `/api/v1/cases/${caseId}/documents/${documentId}`);
  assert.equal(calls[2].body, file);
  assert.equal(calls[2].headers['X-Document-Name'], 'evidence.pdf');
  assert.equal(calls[3].url, `/api/v1/cases/${caseId}/documents/${documentId}/seal`);
  assert.equal(calls[4].url, `/api/v1/cases/${caseId}/documents/${documentId}/verify`);
  assert.equal(calls[5].url, `/api/v1/cases/${caseId}/documents/${documentId}/evidence`);
});

test('discarded case scope rejects late evidence and cannot issue new requests', async () => {
  let complete;
  let calls = 0;
  const api = createApi(() => {
    calls++;
    return new Promise((resolve) => {
      complete = resolve;
    });
  });
  const scope = api.caseDocuments(caseId);
  const pending = scope.evidence(documentId);
  scope.dispose();
  complete(new Response('zip'));
  await assert.rejects(pending, /expediente/i);
  await assert.rejects(scope.detail(documentId), /expediente/i);
  assert.equal(calls, 1);
});
