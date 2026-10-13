import { descriptorKey } from './draft-descriptor.mjs';
import { discardDocumentDrafts } from './draft-document-access.mjs';
import { can } from './documents.mjs';

export function validateAppendDocument(value, caseId, documentId) {
  if (
    value?.case_id !== caseId ||
    value?.id !== documentId ||
    !Number.isSafeInteger(value.version) ||
    value.version < 1 ||
    typeof value.name !== 'string'
  )
    throw new Error('No se pudo confirmar el documento actual.');
}

function validate(value, expected) {
  if (
    (value.file !== null && !(value.file instanceof File)) ||
    typeof value.name !== 'string' ||
    typeof value.unconfirmed !== 'boolean' ||
    !Number.isSafeInteger(value.originalExpectedVersion) ||
    value.originalExpectedVersion < 1 ||
    value.originalExpectedVersion !== expected
  )
    throw new TypeError('Invalid append version draft.');
}

export function createAppendVersionDraft({ session, caseId, documentId, readDocument, capture }) {
  const principalId = session.principal()?.id;
  const descriptor = {
    principalId,
    contextId: caseId,
    editorKind: 'document-version',
    resourceId: documentId,
    action: 'append',
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: null,
  };
  const key = descriptorKey(descriptor);
  let registration = null,
    alive = true;

  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      can(principal?.role, 'documents') &&
      session.canAdmit()
    );
  }

  async function freshContext() {
    if (!admitted()) return null;
    const current = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    if (
      current.id !== caseId ||
      current.administration?.case_id !== caseId ||
      !['active', 'closed'].includes(current.administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const document = await readDocument();
    if (!admitted()) return null;
    validateAppendDocument(document, caseId, documentId);
    return { status: current.administration.administrative_status, document };
  }

  function register(expected) {
    if (!admitted()) throw new Error('No se pudo confirmar la sesi\u00f3n actual.');
    if (!registration)
      registration = session.registry.register(
        { ...descriptor, baseRevision: expected },
        {
          fields: ['file', 'name', 'originalExpectedVersion', 'unconfirmed'],
          capture,
        },
      );
  }

  async function restore(apply) {
    let latest, expected, failure;
    const result = await session.registry.restore(key, {
      authorize: async (entry) => {
        try {
          if (entry.schemaVersion !== 1 || !admitted()) return false;
          expected = entry.baseRevision;
          latest = await freshContext();
          if (latest && latest.document.version < expected)
            throw new Error('La versi\u00f3n actual no confirma la base del borrador.');
          return latest !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply: (value) => {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite el borrador.');
        validate(value, expected);
        register(expected);
        apply(value, latest);
      },
    });
    if (failure) throw failure;
    return result;
  }

  function close() {
    session.registry.closeEditor(key);
    registration?.dispose();
    registration = null;
  }

  return {
    admitted,
    freshContext,
    register,
    restore,
    close,
    pending: () => session.registry.pending().some((entry) => entry.key === key),
    discard: (failure) =>
      discardDocumentDrafts(session.registry, { contextId: caseId, editorKey: key }, failure),
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
