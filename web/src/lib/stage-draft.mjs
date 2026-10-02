import { descriptorKey } from './draft-descriptor.mjs';
import { manageCase } from './case-administration.mjs';
import { stageSupportFields } from './case-stages.mjs';
import { validateStage } from './stage-draft-values.mjs';
import { stageSupportKey } from './stage-support-draft.mjs';

export function pendingStageDrafts(session, caseId) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.editorKind === 'case-stage' &&
          row.contextId === caseId &&
          row.principalId === session.principal()?.id,
      ) ?? []
  );
}
export function createStageDraft({ session, caseId, api, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    root = null,
    registration = null;
  const descriptor = (action, expected) => ({
    principalId,
    contextId: caseId,
    editorKind: 'case-stage',
    resourceId: caseId,
    action,
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: expected,
  });
  function admitted() {
    const principal = session.principal();
    return (
      alive && principal?.id === principalId && manageCase(principal.role) && session.canAdmit()
    );
  }
  async function freshContext() {
    if (!admitted()) return null;
    const detail = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const administration = detail.administration;
    if (
      detail.id !== caseId ||
      administration?.case_id !== caseId ||
      !['active', 'closed'].includes(administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const result = await api.get();
    if (!admitted()) return null;
    if (
      result.case_id !== caseId ||
      (result.current !== null &&
        (result.current?.case_id !== caseId ||
          !Number.isSafeInteger(result.current.stage_revision) ||
          result.current.stage_revision < 1))
    )
      throw new Error('No se pudo confirmar la etapa actual.');
    return {
      result,
      closed: administration.administrative_status === 'closed',
      incomplete: !administration.profile,
    };
  }
  function register(action, expected) {
    if (!admitted()) throw new Error('La sesi\u00f3n no admite el borrador.');
    const next = descriptor(action, expected);
    if (
      registration &&
      (descriptorKey(root) !== descriptorKey(next) || root.baseRevision !== expected)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
    if (!registration)
      registration = session.registry.register(root, {
        fields: [
          'base',
          'action',
          'draft',
          'uncertain',
          'needsReview',
          'exhausted',
          'lastPayload',
          'lastSupports',
        ],
        capture,
      });
  }
  async function restore(saved, apply) {
    let context, failure;
    const result = await session.registry.restore(saved.key, {
      authorize: async (entry) => {
        try {
          if (entry.schemaVersion !== 1 || !admitted()) return false;
          context = await freshContext();
          return context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(value) {
        if (!admitted()) throw new Error('La sesi\u00f3n ya no admite la etapa.');
        validateStage(value, saved);
        register(saved.action, saved.baseRevision);
        apply(value, context);
      },
    });
    if (failure) throw failure;
    return result;
  }
  function supportContext(field, canApply, observe, deny) {
    if (!root) return null;
    const ownerDraftKey = descriptorKey(root);
    const context = {
      caseId,
      principalId,
      ownerDraftKey,
      fieldPath: [field],
      rowId: null,
      canApply: () =>
        admitted() && root !== null && descriptorKey(root) === ownerDraftKey && canApply(),
      async authorize() {
        if (!context.canApply()) return false;
        try {
          const fresh = await freshContext();
          if (!fresh || !context.canApply()) return false;
          observe(fresh);
          return (
            (fresh.result.current?.stage_revision ?? 0) === root.baseRevision && context.canApply()
          );
        } catch (error) {
          if (context.canApply() && [403, 404].includes(error.status)) deny(error);
          throw error;
        }
      },
    };
    return context;
  }
  function close() {
    if (root) session.registry.closeEditor(descriptorKey(root));
    registration?.dispose();
    root = registration = null;
  }
  return {
    admitted,
    freshContext,
    register,
    restore,
    supportContext,
    close,
    discardSupport(field) {
      if (root)
        session.registry.closeEditor(
          stageSupportKey({
            caseId,
            principalId,
            ownerDraftKey: descriptorKey(root),
            fieldPath: [field],
            rowId: null,
          }),
        );
    },
    discard() {
      session.registry.denyContext(caseId);
      close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}

export async function refreshStageSupports(draft, documents, caseId, admitted) {
  for (const key of Object.keys(stageSupportFields)) {
    const reference = draft[key];
    if (!reference || !admitted()) continue;
    const scoped = documents.version(reference.id, reference.version);
    try {
      const value = await scoped.detail();
      if (!admitted()) return false;
      if (
        value.case_id !== caseId ||
        value.id !== reference.id ||
        value.version !== reference.version ||
        value.digest !== reference.digest
      )
        throw new Error('El soporte no coincide con la versi\u00f3n conservada.');
    } finally {
      scoped.dispose();
    }
  }
  return admitted();
}
