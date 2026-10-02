import { descriptorKey } from './draft-descriptor.mjs';
import { canParticipants } from './participants.mjs';

function validateRecord(value, caseId, id) {
  if (
    value?.case_id !== caseId ||
    value.id !== id ||
    !Number.isSafeInteger(value.revision) ||
    value.revision < 1 ||
    !['active', 'archived'].includes(value.directory_status)
  )
    throw new Error('No se pudo confirmar la ficha actual del participante.');
}

function validateDraft(value, expected) {
  if (
    !value?.draft ||
    ['display_name', 'procedural_role', 'organization', 'legal_status'].some(
      (key) => typeof value.draft[key] !== 'string',
    ) ||
    !['active', 'archived'].includes(value.draft.directory_status) ||
    value.originalExpectedRevision !== expected ||
    typeof value.unconfirmed !== 'boolean'
  )
    throw new TypeError('Invalid manual participant draft.');
}

export function createManualParticipantDraft({ session, caseId, api, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    registration = null,
    currentKey = null;
  function descriptor(id, expected = null) {
    return {
      principalId,
      contextId: caseId,
      editorKind: 'participant-manual',
      resourceId: id,
      action: id === null ? 'create' : 'replace',
      instanceId: id === null ? 'participants-manual-create' : null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: expected,
    };
  }
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canParticipants(principal?.role, 'manage') &&
      session.canAdmit()
    );
  }
  async function freshCase() {
    if (!admitted()) return null;
    const current = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    if (
      current?.id !== caseId ||
      current.administration?.case_id !== caseId ||
      !['active', 'closed'].includes(current.administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    return current.administration.administrative_status;
  }
  async function freshContext(id) {
    const status = await freshCase();
    if (status === null) return null;
    const record = id === null ? null : await api.get(id);
    if (!admitted()) return null;
    if (id !== null) validateRecord(record, caseId, id);
    return { status, record };
  }
  function register(id, expected) {
    if (!admitted()) throw new Error('No se pudo confirmar la sesi\u00f3n actual.');
    const value = descriptor(id, expected),
      key = descriptorKey(value);
    if (registration && currentKey !== key)
      throw new Error('Cierra la ficha actual antes de abrir otra.');
    if (!registration)
      registration = session.registry.register(value, {
        fields: ['draft', 'originalExpectedRevision', 'unconfirmed'],
        capture,
      });
    currentKey = key;
  }
  async function restore(id, apply) {
    let context, expected, failure;
    const key = descriptorKey(descriptor(id));
    currentKey = key;
    const result = await session.registry.restore(key, {
      authorize: async (entry) => {
        try {
          expected = entry.baseRevision;
          if (
            entry.schemaVersion !== 1 ||
            !admitted() ||
            (id === null ? expected !== null : !Number.isSafeInteger(expected) || expected < 1)
          )
            return false;
          context = await freshContext(id);
          if (context?.record && context.record.revision < expected)
            throw new Error('La revisi\u00f3n actual no confirma la base del borrador.');
          return context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply: (value) => {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite el borrador.');
        validateDraft(value, expected);
        register(id, expected);
        apply(value, context);
      },
    });
    if (failure) throw failure;
    return result;
  }
  async function currentDirectory() {
    const status = await freshCase();
    if (status === null) return null;
    const participants = [],
      cursors = new Set();
    let afterId;
    do {
      const page = await api.list({ limit: 50, status: 'all', profile: 'all', afterId });
      if (!admitted()) return null;
      if (!Array.isArray(page.participants) || typeof page.has_more !== 'boolean')
        throw new Error('No se pudo confirmar el directorio actual.');
      for (const row of page.participants) {
        validateRecord(row, caseId, row.id);
        if (typeof row.id !== 'string' || typeof row.display_name !== 'string')
          throw new Error('No se pudo confirmar una ficha del directorio.');
        participants.push({ id: row.id, name: row.display_name, status: row.directory_status });
      }
      if (!page.has_more) break;
      afterId = page.next_after_id;
      if (
        typeof afterId !== 'string' ||
        !afterId ||
        cursors.has(afterId) ||
        !page.participants.length
      )
        throw new Error('No se pudo completar la consulta del directorio.');
      cursors.add(afterId);
    } while (true);
    return { status, participants };
  }
  function close() {
    if (currentKey) session.registry.closeEditor(currentKey);
    registration?.dispose();
    registration = currentKey = null;
  }
  return {
    admitted,
    freshContext,
    register,
    restore,
    currentDirectory,
    close,
    pending: (id = null) =>
      session.registry.pending().some((entry) => entry.key === descriptorKey(descriptor(id))),
    pendingIds: () =>
      session.registry
        .pending()
        .filter(
          (entry) =>
            entry.principalId === principalId &&
            entry.contextId === caseId &&
            entry.editorKind === 'participant-manual',
        )
        .map((entry) => entry.resourceId),
    discard(failure, id = null) {
      if (![403, 404].includes(failure?.status)) return;
      if (failure.code === 'case_not_found') session.registry.denyContext(caseId);
      else session.registry.closeEditor(descriptorKey(descriptor(id)));
      if (currentKey === descriptorKey(descriptor(id))) close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
