import { descriptorKey } from './draft-descriptor.mjs';
import { canResources } from './procedural-resource-errors.mjs';
import { resourceActivityDraft } from './resource-activity-validation.mjs';

export const activityDraftFields = [
  'id',
  'action',
  'base',
  'association',
  'selection',
  'reason',
  'mode',
  'last',
  'inputs',
];
export function captureActivityDraft(value) {
  if (value.last) resourceActivityDraft(value.last);
  return structuredClone({
    ...Object.fromEntries(activityDraftFields.map((key) => [key, value[key]])),
    mode: ['uncertain', 'conflict'].includes(value.mode) ? value.mode : 'draft',
  });
}
export function pendingActivityDrafts(session, caseId, resourceId) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === session.principal()?.id &&
          row.contextId === caseId &&
          row.resourceId === resourceId &&
          row.editorKind === 'resource-activity',
      ) ?? []
  );
}
export function createActivityDraft({ session, caseId, resourceId, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    registration = null,
    root = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canResources(principal.role, 'manage') &&
      session.canAdmit()
    );
  }
  function register(action, id, revision) {
    const next = {
      principalId,
      contextId: caseId,
      resourceId,
      editorKind: 'resource-activity',
      action,
      instanceId: action === 'unlink' ? id : null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    if (!admitted()) throw new Error('La sesion no admite la actividad.');
    if (
      registration &&
      (descriptorKey(next) !== descriptorKey(root) || revision !== root.baseRevision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
    registration ??= session.registry.register(root, { fields: activityDraftFields, capture });
  }
  return {
    admitted,
    register,
    async restore(saved, fresh, apply) {
      let context, failure;
      const result = await session.registry.restore(saved.key, {
        authorize: async (descriptor) => {
          try {
            context = await fresh();
            return descriptor.schemaVersion === 1 && !!context && admitted();
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(value) {
          if (
            !admitted() ||
            !value ||
            value.action !== saved.action ||
            value.base?.id !== resourceId ||
            value.base.case_id !== caseId ||
            value.base.revision !== saved.baseRevision ||
            (value.action === 'unlink' &&
              (value.id !== saved.instanceId || value.association?.id !== value.id)) ||
            !['link', 'unlink'].includes(value.action) ||
            typeof value.reason !== 'string' ||
            (value.mode === 'uncertain' && !value.last)
          )
            throw new Error('Borrador de actividad incompatible.');
          if (
            value.last &&
            (value.last.case_id !== caseId ||
              value.last.resource_id !== resourceId ||
              value.last.recorded_by.id !== principalId ||
              value.last.command.association_id !== value.id ||
              value.last.command.change.action !== value.action ||
              value.last.command.expected_resource_revision !== saved.baseRevision)
          )
            throw new Error('El recibo no pertenece a esta actividad.');
          captureActivityDraft(value);
          register(value.action, value.id, value.base.revision);
          apply(value, context);
        },
      });
      if (failure) throw failure;
      return result;
    },
    close() {
      if (root) session.registry.closeEditor(descriptorKey(root));
      registration?.dispose();
      root = registration = null;
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}

export async function refreshActivityReferences(api, caseId, resourceId, selection, admitted) {
  const resources = api.caseResources(caseId);
  let target;
  const same = (a, b) => a === b;
  try {
    const resource = await resources.revision(resourceId, selection.resource.revision);
    if (!admitted()) return false;
    if (!same(resource.receipt.capture_digest, selection.resource.capture_digest))
      throw new Error('La captura del recurso no coincide.');
    if (selection.act) {
      const exact = await resources.revision(resourceId, selection.act.resource_revision);
      if (!admitted()) return false;
      if (
        exact.act?.id !== selection.act.id ||
        exact.act.revision !== selection.act.revision ||
        exact.receipt.capture_digest !== selection.act.capture_digest
      )
        throw new Error('La captura del acto no coincide.');
    }
    if (selection.target) {
      const ref = selection.target;
      target = ref.kind === 'hearing' ? api.caseHearings(caseId) : api.deadlines(caseId);
      const exact = await target.revision(ref.id, ref.revision);
      if (!admitted()) return false;
      const digest = ref.kind === 'hearing' ? 'submission_digest' : 'capture_digest';
      if (exact.receipt[digest] !== ref[digest])
        throw new Error('La captura de la actividad no coincide.');
    }
    return true;
  } finally {
    resources.dispose();
    target?.dispose();
  }
}
