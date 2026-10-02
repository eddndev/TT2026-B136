import { descriptorKey } from './draft-descriptor.mjs';
import { canParticipants } from './participants.mjs';
import { validateTypedDraft, typedSupportEntries } from './typed-draft-values.mjs';
import { discardOwnedSupport } from './participant-support-draft.mjs';

export function createTypedDraft({ session, caseId, api, manualApi, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    registration = null,
    root = null;
  const children = new Map();
  const descriptor = (entry) => ({
    principalId,
    contextId: caseId,
    editorKind: 'participant-typed',
    resourceId: entry.id,
    action: entry.action,
    instanceId: entry.id === null ? 'participants-typed-create' : null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: entry.expected,
  });
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      session.canAdmit() &&
      canParticipants(principal.role, 'manage')
    );
  }
  async function freshContext(entry) {
    if (!admitted()) return null;
    const current = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const status = current?.administration?.administrative_status;
    if (
      current?.id !== caseId ||
      current.administration?.case_id !== caseId ||
      !['active', 'closed'].includes(status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const record = entry.id === null ? null : await manualApi.get(entry.id);
    if (!admitted()) return null;
    if (
      entry.id !== null &&
      (record?.case_id !== caseId ||
        record.id !== entry.id ||
        !Number.isSafeInteger(record.revision) ||
        record.revision < (entry.expected ?? 1))
    )
      throw new Error('No se pudo confirmar la ficha actual.');
    return { status, record };
  }
  function register(entry) {
    if (!admitted()) throw new Error('La sesi\u00f3n no admite el borrador.');
    if (
      registration &&
      (descriptorKey(descriptor(root)) !== descriptorKey(descriptor(entry)) ||
        root.expected !== entry.expected)
    ) {
      registration.dispose();
      registration = null;
    }
    root = { ...entry };
    if (!registration)
      registration = session.registry.register(descriptor(root), {
        fields: [
          'subject',
          'selectedReference',
          'role',
          'reason',
          'decisions',
          'expected',
          'uncertain',
          'lastSubmission',
          'formDraft',
          'credentialDraft',
        ],
        capture,
      });
  }
  async function restore(entry, apply) {
    let context, failure;
    const result = await session.registry.restore(descriptorKey(descriptor(entry)), {
      authorize: async (saved) => {
        try {
          if (saved.schemaVersion !== 1 || !admitted()) return false;
          entry = { ...entry, expected: saved.baseRevision };
          context = await freshContext(entry);
          return context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(value) {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite la ficha.');
        validateTypedDraft(value, entry.expected);
        register(entry);
        apply(value, context, entry);
      },
    });
    if (failure) throw failure;
    return result;
  }
  function close(entry = root) {
    if (entry) session.registry.closeEditor(descriptorKey(descriptor(entry)));
    registration?.dispose();
    registration = root = null;
    children.clear();
  }
  function supportContext(fieldPath, rowId, canApply, onDenied) {
    if (!root) return null;
    const entry = { ...root },
      ownerDraftKey = descriptorKey(descriptor(entry));
    const context = {
      caseId,
      principalId,
      ownerDraftKey,
      fieldPath,
      rowId,
      canApply: () =>
        admitted() &&
        root !== null &&
        descriptorKey(descriptor(root)) === ownerDraftKey &&
        canApply(),
      async authorize() {
        if (!context.canApply()) return false;
        try {
          const fresh = await freshContext(root);
          return (
            !!fresh &&
            (!fresh.record || fresh.record.revision === root?.expected) &&
            context.canApply()
          );
        } catch (failure) {
          if (context.canApply() && [403, 404].includes(failure.status)) onDenied(failure);
          throw failure;
        }
      },
    };
    children.set(JSON.stringify([fieldPath, rowId]), context);
    return context;
  }
  function discardPath(path, rowId) {
    if (!root) return;
    const owner = descriptorKey(descriptor(root));
    for (const child of session.registry.pending()) {
      if (
        child.ownerDraftKey === owner &&
        path.every((part, index) => child.fieldPath[index] === part) &&
        (rowId === undefined || child.rowId === rowId)
      )
        session.registry.closeEditor(child.key);
    }
    for (const [key, child] of children) {
      if (
        path.every((part, index) => child.fieldPath[index] === part) &&
        (rowId === undefined || child.rowId === rowId)
      ) {
        discardOwnedSupport(session, child);
        children.delete(key);
      }
    }
  }
  return {
    admitted,
    freshContext,
    register,
    restore,
    close,
    supportContext,
    discardPath,
    pending: (entry) =>
      session.registry.pending().some((row) => row.key === descriptorKey(descriptor(entry))),
    pendingEntries: () =>
      session.registry
        .pending()
        .filter(
          (row) =>
            row.editorKind === 'participant-typed' &&
            row.contextId === caseId &&
            row.principalId === principalId,
        )
        .map((row) => ({
          id: row.resourceId,
          action: row.action,
          expected: row.baseRevision,
        })),
    async exactSubject(reference) {
      const value = await api.subjectRevision(reference.id, reference.revision);
      if (!admitted()) return null;
      if (value.values_digest !== reference.values_digest)
        throw new Error('La identidad no corresponde a la revisi\u00f3n conservada.');
      return value;
    },
    discard(failure, entry = root) {
      if (![403, 404].includes(failure?.status)) return;
      if (failure.code === 'case_not_found') session.registry.denyContext(caseId);
      close(entry);
    },
    dispose() {
      alive = false;
      registration?.dispose();
      children.clear();
    },
  };
}

export async function refreshTypedSupports(state, docs, caseId, admitted) {
  let rejected = false;
  const entries = typedSupportEntries(state);
  if (state.selected) entries.push([state.selected.values, 'identity_support']);
  for (const [owner, key] of entries) {
    const value = owner[key];
    if (!value || !admitted()) continue;
    const scoped = docs.version(value.document_id, value.version);
    try {
      const result = await scoped.detail();
      if (!admitted()) return false;
      if (
        result.case_id !== caseId ||
        result.id !== value.document_id ||
        result.version !== value.version ||
        result.digest !== value.digest
      )
        throw new Error('El soporte no corresponde a la versi\u00f3n conservada.');
    } catch (failure) {
      if (!admitted()) return false;
      if (failure.code === 'case_not_found' || ![403, 404].includes(failure.status)) throw failure;
      if (owner === state.selected?.values) {
        failure.documentSupportDenied = true;
        throw failure;
      }
      owner[key] = null;
      rejected = true;
    } finally {
      scoped.dispose();
    }
  }
  return rejected;
}
