import { caseStagesApi } from './case-stages-api.mjs';
import { caseAdministrationApi } from './case-administration-api.mjs';
import { participantsApi } from './participants-api.mjs';
import { versionApi } from './version-api.mjs';
import { metadataApi } from './metadata-api.mjs';

function query(values) {
  return new URLSearchParams(
    Object.entries(values).filter(([, value]) => value !== undefined && value !== ''),
  ).toString();
}

export function caseApi(transport) {
  const watchers = new Map();
  async function request(path, options) {
    try {
      return await transport(path, options);
    } catch (error) {
      if (error.code === 'case_closed') {
        const id = /^\/cases\/([^/]+)/.exec(path)?.[1];
        for (const listener of watchers.get(id) || []) listener(error);
      }
      throw error;
    }
  }
  const path = (id) => `/cases/${encodeURIComponent(id)}`;
  return {
    ...caseAdministrationApi(request),
    watchCase(id, listener) {
      const listeners = watchers.get(id) || new Set();
      listeners.add(listener);
      watchers.set(id, listeners);
      return () => {
        listeners.delete(listener);
        if (!listeners.size) watchers.delete(id);
      };
    },
    cases: ({ limit = 51, offset = 0 } = {}) => request(`/cases?${query({ limit, offset })}`),
    createCase: (title, reference) =>
      request('/cases', { method: 'POST', data: { title, reference } }),
    caseDetail: (id) => request(path(id)),
    assignMember: (id, userId) =>
      request(`${path(id)}/members/${encodeURIComponent(userId)}`, { method: 'PUT' }),
    removeMember: (id, userId) =>
      request(`${path(id)}/members/${encodeURIComponent(userId)}`, { method: 'DELETE' }),
    caseStages: (id) => caseStagesApi(request, id),
    caseParticipants: (id) => participantsApi(request, id),
    caseDocuments(id) {
      let active = true;
      const base = `${path(id)}/documents`;
      const assertActive = () => {
        if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
      };
      const scoped = async (suffix, options) => {
        assertActive();
        const result = await request(`${base}${suffix}`, options);
        assertActive();
        return result;
      };
      return {
        ...versionApi(scoped),
        ...metadataApi(scoped, id),
        dispose: () => {
          active = false;
        },
        list: ({ limit = 50, offset = 0, name, sealed, document_type, classification, tag } = {}) =>
          scoped(`?${query({ limit, offset, name, sealed, document_type, classification, tag })}`),
        detail: (documentId) => scoped(`/${encodeURIComponent(documentId)}`),
        upload: (file, name) =>
          scoped('', {
            method: 'POST',
            body: file,
            headers: { 'X-Document-Name': name, 'Content-Type': 'application/octet-stream' },
          }),
        seal: (documentId) => scoped(`/${encodeURIComponent(documentId)}/seal`, { method: 'POST' }),
        verify: (documentId) =>
          scoped(`/${encodeURIComponent(documentId)}/verify`, { method: 'POST' }),
        evidence: (documentId) =>
          scoped(`/${encodeURIComponent(documentId)}/evidence`, { binary: true }),
      };
    },
  };
}
