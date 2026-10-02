import { descriptorKey } from './draft-descriptor.mjs';
import { discardDocumentDrafts } from './draft-document-access.mjs';
import { can } from './documents.mjs';

function validate(value) {
  if (
    (value.file !== null && !(value.file instanceof File)) ||
    typeof value.name !== 'string' ||
    typeof value.draft?.document_type !== 'string' ||
    typeof value.draft?.classification !== 'string' ||
    !Array.isArray(value.draft?.tags) ||
    value.draft.tags.some((tag) => typeof tag !== 'string') ||
    typeof value.rawFields?.tag?.pending !== 'string' ||
    typeof value.unconfirmed !== 'boolean'
  )
    throw new TypeError('Invalid upload draft.');
}

export function createRootUploadDraft({ session, caseId, capture }) {
  const principalId = session.principal()?.id;
  const descriptor = {
    principalId,
    contextId: caseId,
    editorKind: 'root-upload',
    resourceId: null,
    action: 'create',
    instanceId: 'documents-upload',
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: null,
  };
  const key = descriptorKey(descriptor);
  let alive = true;
  let registration = null;

  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      can(principal?.role, 'documents') &&
      can(principal?.role, 'classify') &&
      session.canAdmit()
    );
  }

  async function freshContext() {
    if (!admitted()) return null;
    const current = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const administration = current.administration;
    if (
      current.id !== caseId ||
      administration?.case_id !== caseId ||
      !['active', 'closed'].includes(administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    return administration.administrative_status;
  }

  function register() {
    if (!admitted()) throw new Error('No se pudo confirmar la sesi\u00f3n actual.');
    if (!registration)
      registration = session.registry.register(descriptor, {
        fields: ['file', 'name', 'draft', 'rawFields', 'unconfirmed'],
        capture,
      });
  }

  async function restore(apply) {
    let status;
    let failure;
    const result = await session.registry.restore(key, {
      authorize: async (entry) => {
        try {
          if (entry.schemaVersion !== 1 || !admitted()) return false;
          status = await freshContext();
          return status !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply: (value) => {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite el borrador.');
        validate(value);
        register();
        apply(value, status);
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
    pending: () => session.registry.pending().some((entry) => entry.key === key),
    discard: (failure) =>
      discardDocumentDrafts(session.registry, { contextId: caseId, editorKey: key }, failure),
    close,
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
