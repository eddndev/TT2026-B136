import { descriptorKey } from './draft-descriptor.mjs';
import { canFacts } from './procedural-fact-errors.mjs';
import { factDraftFields, validateFactDraft } from './fact-draft-values.mjs';
import { stageSupportKey } from './stage-support-draft.mjs';

export function pendingFactDrafts(session, caseId, family, parentId = null) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === session.principal()?.id &&
          row.contextId === caseId &&
          row.editorKind === `fact-${family}` &&
          row.instanceId === parentId,
      ) ?? []
  );
}
export function createFactDraft({ session, caseId, family, parentId, capture, read }) {
  const principalId = session.principal()?.id;
  let alive = true,
    root = null,
    registration = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canFacts(principal.role, 'manage') &&
      session.canAdmit()
    );
  }
  async function fresh() {
    if (!admitted()) return null;
    const record = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const administration = record.administration;
    if (
      record.id !== caseId ||
      administration?.case_id !== caseId ||
      !['active', 'closed'].includes(administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const current = await read();
    return admitted()
      ? { current, closed: administration.administrative_status === 'closed' }
      : null;
  }
  function register(action, resourceId, revision) {
    if (!admitted()) throw new Error('La sesion no admite el borrador.');
    const next = {
      principalId,
      contextId: caseId,
      editorKind: `fact-${family}`,
      resourceId,
      action,
      instanceId: parentId,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    if (
      registration &&
      (descriptorKey(root) !== descriptorKey(next) || root.baseRevision !== revision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
    registration ??= session.registry.register(root, { fields: factDraftFields, capture });
  }
  async function restore(saved, apply) {
    let context, failure;
    const result = await session.registry.restore(saved.key, {
      authorize: async (descriptor) => {
        try {
          context = await fresh();
          return descriptor.schemaVersion === 1 && context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(value) {
        if (!admitted()) throw new Error('La sesion ya no admite el borrador.');
        validateFactDraft(value, saved, family, parentId);
        register(saved.action, saved.resourceId, saved.baseRevision);
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
      principalId,
      caseId,
      ownerDraftKey,
      fieldPath: [...field],
      rowId: null,
      canApply: () =>
        admitted() && root !== null && descriptorKey(root) === ownerDraftKey && canApply(),
      async authorize() {
        if (!context.canApply()) return false;
        try {
          const value = await fresh();
          if (!value || !context.canApply()) return false;
          observe(value);
          return context.canApply();
        } catch (failure) {
          if (context.canApply() && [403, 404].includes(failure.status)) deny(failure);
          throw failure;
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
    fresh,
    register,
    restore,
    supportContext,
    close,
    discardSupport(field) {
      if (root)
        session.registry.closeEditor(
          stageSupportKey({
            principalId,
            caseId,
            ownerDraftKey: descriptorKey(root),
            fieldPath: field,
          }),
        );
    },
    deny(failure) {
      if (failure.code === 'case_not_found' || failure.status === 403)
        session.registry.denyContext(caseId);
      else close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
