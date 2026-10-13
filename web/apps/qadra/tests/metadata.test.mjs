import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeMetadata, normalizeTag, metadataFilters } from '../src/lib/document-metadata.mjs';
import { createApi } from '../src/lib/api.mjs';
const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const values = {
  document_type: 'Escrito',
  classification: 'Civil',
  tags: ['prueba, documental', 'acci\u00f3n'],
};

test('metadata keeps individual Unicode tags, canonical whitespace and scalar lengths', () => {
  assert.deepEqual(
    normalizeMetadata({
      document_type: ' \u2003Escrito\u00a0',
      classification: '',
      tags: [' e\u0301 ', '\u00e9', 'a,b', 'a,b'],
    }),
    {
      document_type: 'Escrito',
      classification: null,
      tags: ['a,b', 'e\u0301', '\u00e9'],
    },
  );
  assert.equal(normalizeTag('\ufeffx\ufeff'), '\ufeffx\ufeff');
  assert.equal(normalizeTag('\u{1f600}'.repeat(40)), '\u{1f600}'.repeat(40));
  assert.throws(() => normalizeTag('\u{1f600}'.repeat(41)), /40/);
  for (const control of ['\n', '\u0085', '\u007f'])
    assert.throws(() => normalizeTag(`${control}x`), /control/);
  assert.throws(() => normalizeMetadata({ ...values, tags: Array(21).fill('same') }), /20/);
  assert.throws(() => normalizeTag(' \u2003 '), /vac\u00eda/);
  assert.throws(() => normalizeMetadata({ ...values, document_type: 'x'.repeat(81) }), /80/);
});

test('classified upload is one multipart request with browser-owned boundary', async () => {
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    return Response.json({ id });
  });
  await api.caseDocuments(caseId).uploadWithMetadata(new Blob(['exact bytes']), 'file.txt', values);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, `/api/v1/cases/${caseId}/documents/with-metadata`);
  assert.equal(calls[0].method, 'POST');
  assert.equal(calls[0].headers['Content-Type'], undefined);
  assert.equal(calls[0].headers['X-Document-Name'], 'file.txt');
  assert.deepEqual([...calls[0].body.keys()], ['file', 'metadata']);
  assert.equal(await calls[0].body.get('file').text(), 'exact bytes');
  assert.deepEqual(JSON.parse(await calls[0].body.get('metadata').text()), values);
});

test('metadata scope binds context, revision, cursor and rejects late responses', async () => {
  const calls = [];
  let release;
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    if (calls.length === 4)
      return new Promise((resolve) => {
        release = resolve;
      });
    return Response.json({ case_id: caseId, id, metadata_revision: 2, ...values });
  });
  const scope = api.caseDocuments(caseId).metadata(id);
  await scope.get();
  await scope.replace(1, values);
  await scope.history({ beforeRevision: 2 });
  assert.equal(calls[1].method, 'PUT');
  assert.deepEqual(JSON.parse(calls[1].body), { expected_metadata_revision: 1, ...values });
  assert.equal(
    calls[2].url,
    `/api/v1/cases/${caseId}/documents/${id}/metadata/history?limit=50&before_revision=2`,
  );
  const pending = scope.get();
  scope.dispose();
  release(Response.json({ case_id: caseId, id }));
  await assert.rejects(pending, /documento/);
});

test('metadata rejects mismatched response context and keeps server conflict code', async () => {
  const bad = createApi(async () => Response.json({ case_id: caseId, id: 'other' }));
  await assert.rejects(bad.caseDocuments(caseId).metadata(id).get(), /documento/);
  const api = createApi(async () =>
    Response.json({ error: { code: 'document_metadata_conflict' } }, { status: 409 }),
  );
  await assert.rejects(
    api.caseDocuments(caseId).metadata(id).replace(0, values),
    (error) => error.code === 'document_metadata_conflict',
  );
});

test('metadata filters keep exact Unicode and comma values in one query parameter', async () => {
  let url;
  const api = createApi(async (path) => {
    url = new URL(path, 'http://fixture');
    return Response.json({});
  });
  const filters = metadataFilters({
    document_type: ' Escrito ',
    classification: 'Civil',
    tag: 'acci\u00f3n, prueba',
  });
  await api.caseDocuments(caseId).list({ offset: 50, name: 'file', sealed: true, ...filters });
  assert.equal(url.searchParams.get('tag'), 'acci\u00f3n, prueba');
  assert.equal(url.searchParams.get('document_type'), 'Escrito');
  assert.equal(url.searchParams.get('offset'), '50');
  assert.deepEqual(metadataFilters({ document_type: '', classification: '', tag: '' }), {});
});

test('classification permission is independent of sealing and excludes Client', async () => {
  const { can } = await import('../src/lib/documents.mjs');
  for (const role of ['owner', 'litigator', 'paralegal']) assert.equal(can(role, 'classify'), true);
  assert.equal(can('client', 'classify'), false);
  assert.equal(can('paralegal', 'seal'), false);
});
