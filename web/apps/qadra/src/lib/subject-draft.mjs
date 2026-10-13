import { descriptorKey } from './draft-descriptor.mjs';
import { canParticipants } from './participants.mjs';
import { validateSubjectDraft } from './subject-draft-values.mjs';

export function createSubjectDraft({ session, caseId, api, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    registration = null,
    currentId = null,
    expectedBase = null;
  function descriptor(id, baseRevision = null) {
    return {
      principalId,
      contextId: caseId,
      editorKind: 'participant-subject',
      resourceId: id,
      action: 'replace',
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision,
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
  async function freshContext(id) {
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
    const record = await api.subject(id);
    if (!admitted()) return null;
    if (
      record?.case_id !== caseId ||
      record.id !== id ||
      !record.values ||
      !Number.isSafeInteger(record.revision) ||
      record.revision < 1
    )
      throw new Error('No se pudo confirmar la identidad actual.');
    return { status, record };
  }
  function register(id, expected) {
    if (!admitted()) throw new Error('No se pudo confirmar la sesi\u00f3n actual.');
    if (registration && (currentId !== id || expectedBase !== expected)) {
      registration.dispose();
      registration = null;
    }
    if (!registration)
      registration = session.registry.register(descriptor(id, expected), {
        fields: ['draft', 'reason', 'decisions', 'expected', 'uncertain', 'last'],
        capture,
      });
    currentId = id;
    expectedBase = expected;
  }
  async function restore(id, apply) {
    let context, expected, failure;
    const result = await session.registry.restore(descriptorKey(descriptor(id)), {
      authorize: async (entry) => {
        try {
          expected = entry.baseRevision;
          if (
            entry.schemaVersion !== 1 ||
            !Number.isSafeInteger(expected) ||
            expected < 1 ||
            !admitted()
          )
            return false;
          context = await freshContext(id);
          if (context && context.record.revision < expected)
            throw new Error('La revisi\u00f3n actual no confirma la base del borrador.');
          return context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(value) {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite el borrador.');
        validateSubjectDraft(value, expected);
        register(id, expected);
        apply(value, context);
      },
    });
    if (failure) throw failure;
    return result;
  }
  function close(id = currentId) {
    if (id) session.registry.closeEditor(descriptorKey(descriptor(id)));
    registration?.dispose();
    registration = null;
    currentId = expectedBase = null;
  }
  return {
    admitted,
    freshContext,
    register,
    restore,
    close,
    pending: (id) =>
      session.registry.pending().some((entry) => entry.key === descriptorKey(descriptor(id))),
    supportContext(id, fieldPath, rowId, canApply, onDenied) {
      return {
        caseId,
        principalId,
        ownerDraftKey: descriptorKey(descriptor(id)),
        fieldPath,
        rowId,
        canApply: () => admitted() && currentId === id && canApply(),
        async authorize() {
          if (!admitted() || currentId !== id || !canApply()) return false;
          try {
            const context = await freshContext(id);
            return (
              !!context &&
              context.record.revision === expectedBase &&
              admitted() &&
              currentId === id &&
              canApply()
            );
          } catch (failure) {
            if (admitted() && [403, 404].includes(failure.status)) onDenied(failure);
            throw failure;
          }
        },
      };
    },
    discard(failure, id) {
      if (![403, 404].includes(failure?.status)) return;
      if (failure.code === 'case_not_found') session.registry.denyContext(caseId);
      close(id);
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}

export async function refreshDraftSupports({ draft, decisions, docs, caseId, admitted }) {
  let rejected = false;
  const entries = [
    [draft, 'identity_support'],
    ...Object.values(decisions).map((decision) => [decision, 'support']),
  ];
  for (const [owner, field] of entries) {
    const value = owner[field];
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
        throw new Error('El soporte no coincide con la versi\u00f3n conservada.');
    } catch (failure) {
      if (!admitted()) return false;
      if (failure.code === 'case_not_found') throw failure;
      if (![403, 404].includes(failure.status)) throw failure;
      owner[field] = null;
      rejected = true;
    } finally {
      scoped.dispose();
    }
  }
  return rejected;
}
