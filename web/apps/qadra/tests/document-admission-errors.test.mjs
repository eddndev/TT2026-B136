import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import {
  admissionRejections,
  privateParserDetail,
  rejectionPayload,
} from './fixtures/document-admission-errors.mjs';

const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const documentId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const base = `/api/v1/cases/${caseId}/documents`;
const operations = [
  { name: 'upload', suffix: '', submit: (api, file) => api.upload(file, 'evidence.pdf') },
  {
    name: 'classified upload',
    suffix: '/with-metadata',
    submit: (api, file, metadata) => api.uploadWithMetadata(file, 'evidence.pdf', metadata),
  },
  {
    name: 'append',
    suffix: `/${documentId}/versions?expected_version=7`,
    submit: (api, file) => api.append(documentId, 7, file, 'evidence.pdf'),
  },
];
for (const rejection of admissionRejections)
  for (const operation of operations)
    test(`${operation.name} preserves ${rejection.code}, draft bytes and session without automatic replay`, async () => {
      const calls = [];
      const api = createApi(async (url, options) => {
        calls.push({ url, ...options });
        if (url.endsWith('/totp'))
          return Response.json({ access_token: 'retained-session', user: { id: documentId } });
        if (url.endsWith('/me')) return Response.json({ id: documentId });
        return Response.json(rejectionPayload(rejection.code), { status: rejection.status });
      });
      await api.mfa('challenge', '123456', 'totp');
      const contents = new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d, 0, 0xff, 0x0a]);
      const file = new Blob([contents], { type: 'application/pdf' });
      const metadata = {
        document_type: 'Escrito',
        classification: 'Interno',
        tags: ['accion, prueba'],
      };
      const before = structuredClone(metadata);
      let failure;
      try {
        await operation.submit(api.caseDocuments(caseId), file, metadata);
      } catch (error) {
        failure = error;
      }
      assert.ok(failure, 'Admission must reject instead of producing a document');
      await api.me();
      const writes = calls.filter((call) => call.url.startsWith(base));
      assert.equal(writes.length, 1);
      assert.equal(writes[0].url, base + operation.suffix);
      assert.equal(writes[0].method, 'POST');
      assert.equal(writes[0].headers.Authorization, 'Bearer retained-session');
      assert.equal(calls.at(-1).headers.Authorization, 'Bearer retained-session');
      assert.equal(writes[0].headers['X-Document-Name'], 'evidence.pdf');
      if (operation.name === 'classified upload') {
        assert.deepEqual(new Uint8Array(await writes[0].body.get('file').arrayBuffer()), contents);
        assert.deepEqual(JSON.parse(await writes[0].body.get('metadata').text()), before);
      } else assert.equal(writes[0].body, file);
      assert.deepEqual(new Uint8Array(await file.arrayBuffer()), contents);
      assert.deepEqual(metadata, before);
      assert.equal(failure.code, rejection.code);
      assert.equal(failure.status, rejection.status);
      assert.ok(!failure.message.includes(privateParserDetail));
      for (const explanation of rejection.explanation) assert.match(failure.message, explanation);
    });
