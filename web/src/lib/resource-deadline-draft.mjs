import { descriptorKey } from './draft-descriptor.mjs';
import { canDeadlines } from './deadline-errors.mjs';
import { resourceDeadlineDraft } from './resource-deadline-values.mjs';
import { factUuid } from './procedural-fact-primitives.mjs';

const fields = [
  'deadlineId',
  'associationId',
  'base',
  'selection',
  'definition',
  'policies',
  'mode',
  'last',
  'inputs',
];
export function initialResourceDeadline(caseId) {
  return {
    title: '',
    profile: null,
    responsible_id: '',
    input: {
      selection: { case_id: caseId, source: { kind: '' }, qualification: null },
      calendar: null,
      ordered_quantity: null,
      qualification: {
        statement: '',
        locator: '',
        scope_applies: { kind: '' },
        unresolved_incident: { kind: '' },
        conditions: [],
      },
    },
  };
}
export function captureResourceDeadline(value) {
  if (value.last) resourceDeadlineDraft(value.last);
  return structuredClone({
    ...Object.fromEntries(fields.map((key) => [key, value[key]])),
    mode: ['uncertain', 'conflict'].includes(value.mode) ? value.mode : 'draft',
  });
}
export function pendingResourceDeadlineDrafts(session, caseId, resourceId) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === session.principal()?.id &&
          row.contextId === caseId &&
          row.resourceId === resourceId &&
          row.editorKind === 'resource-deadline',
      ) ?? []
  );
}
export function createResourceDeadlineDraft({ session, caseId, resourceId, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    registration = null,
    root = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canDeadlines(principal.role, 'manage') &&
      session.canAdmit()
    );
  }
  function register(revision) {
    if (!admitted()) throw new Error('La sesion no admite el plazo del recurso.');
    if (registration && revision !== root.baseRevision) {
      registration.dispose();
      registration = null;
    }
    root = {
      principalId,
      contextId: caseId,
      resourceId,
      editorKind: 'resource-deadline',
      action: 'register',
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    registration ??= session.registry.register(root, { fields, capture });
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
            saved.action !== 'register' ||
            value.base?.id !== resourceId ||
            value.base.case_id !== caseId ||
            value.base.revision !== saved.baseRevision ||
            value.selection?.resource?.id !== resourceId ||
            value.definition?.input?.selection?.case_id !== caseId ||
            typeof value.definition.title !== 'string' ||
            typeof value.deadlineId !== 'string' ||
            typeof value.associationId !== 'string' ||
            value.deadlineId === value.associationId ||
            (value.mode === 'uncertain' && !value.last)
          )
            throw new Error('El borrador no pertenece al plazo de este recurso.');
          factUuid(value.deadlineId);
          factUuid(value.associationId);
          if (
            value.last &&
            (value.last.command.case_id !== caseId ||
              value.last.command.resource_id !== resourceId ||
              value.last.deadline.actor_id !== principalId ||
              value.last.command.association_id !== value.associationId ||
              value.last.command.deadline.deadline_id !== value.deadlineId ||
              value.last.command.expected_resource_revision !== saved.baseRevision)
          )
            throw new Error('El envio no pertenece a este borrador.');
          captureResourceDeadline(value);
          register(value.base.revision);
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
