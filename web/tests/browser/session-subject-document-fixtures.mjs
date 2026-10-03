import { createHash } from 'node:crypto';
import { document, caseId } from './helpers.mjs';
import { documentsPath } from './session-inactivity-helpers.mjs';

export { documentsPath };
export const exactSupportPath = `${documentsPath}/${document.id}/versions/1`;
export const childUploadPath = `${documentsPath}/with-metadata`;

export async function installSubjectDocuments(page, state, { authorize, wait, reply, unexpected }) {
  Object.assign(state, {
    documents: new Map([[document.id, structuredClone(document)]]),
    deniedVersions: new Set(),
    uploads: [],
    nextUpload: null,
  });
  await page.route(
    (url) => url.pathname.startsWith(documentsPath),
    async (route) => {
      const call = await authorize(route);
      if (!call) return;
      const parts = call.path.slice(documentsPath.length).split('/').filter(Boolean);
      const row = state.documents.get(parts[0]);
      if (call.method === 'GET' && call.body === null) {
        await wait(call);
        if (!parts.length) {
          const query = new URLSearchParams(call.search);
          const rows = [...state.documents.values()].filter((item) =>
            item.name.includes(query.get('name') || ''),
          );
          const offset = Number(query.get('offset') || 0),
            limit = Number(query.get('limit') || 50);
          return reply(route, {
            documents: rows.slice(offset, offset + limit),
            has_more: offset + limit < rows.length,
          });
        }
        if (!row || state.deniedVersions.has(call.path))
          return reply(route, { error: { code: 'document_not_found' } }, 404);
        if (parts.length === 1) return reply(route, row);
        if (parts.length === 2 && parts[1] === 'versions')
          return reply(route, {
            versions: [row],
            has_more: false,
            next_before_version: null,
            first_available_version: 1,
          });
        if (parts.length === 3 && parts[1] === 'versions' && Number(parts[2]) === row.version)
          return reply(route, row);
        return unexpected(route, call);
      }
      if (
        call.path !== childUploadPath ||
        call.method !== 'POST' ||
        call.search ||
        !state.nextUpload
      )
        return unexpected(route, call);
      const mode = state.nextUpload;
      state.nextUpload = null;
      const request = route.request();
      const form = await new Response(request.postDataBuffer(), {
        headers: { 'Content-Type': request.headers()['content-type'] },
      }).formData();
      const file = form.get('file'),
        bytes = Buffer.from(await file.arrayBuffer());
      const metadata = JSON.parse(await form.get('metadata').text());
      const captured = {
        ...call,
        keys: [...form.keys()].sort(),
        filename: file.name,
        type: file.type,
        text: bytes.toString(),
        name: call.headers['x-document-name'],
        metadata,
      };
      state.uploads.push(captured);
      const result = {
        case_id: caseId,
        id: `88888888-8888-4888-8888-${String(state.uploads.length).padStart(12, '0')}`,
        version: 1,
        name: captured.name,
        sealed: false,
        digest: createHash('sha256').update(bytes).digest('hex'),
        current_metadata: { metadata_revision: 1, ...metadata },
      };
      if (mode.commit !== false) state.documents.set(result.id, result);
      await wait(call);
      return mode.status
        ? reply(route, { error: { code: 'service_busy' } }, mode.status)
        : reply(route, result, 201);
    },
  );
}
