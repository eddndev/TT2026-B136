import { descriptorKey } from './draft-descriptor.mjs';
import { canDeadlines } from './deadline-errors.mjs';
import { deadlinePreparedValue } from './deadline-validation.mjs';
import { factUuid } from './procedural-fact-primitives.mjs';

const fields = [
  'id',
  'current',
  'definition',
  'policies',
  'attention',
  'reason',
  'step',
  'last',
  'inputs',
];
export function initialDeadline(caseId) {
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
export function captureDeadline(value) {
  if (value.last) deadlinePreparedValue(value.last);
  return structuredClone({
    ...Object.fromEntries(fields.map((key) => [key, value[key]])),
    step: ['uncertain', 'conflict', 'exhausted'].includes(value.step) ? value.step : 'draft',
  });
}
export function pendingDeadlineDrafts(session, caseId) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === session.principal()?.id &&
          row.contextId === caseId &&
          row.editorKind === 'deadline',
      ) ?? []
  );
}
export function createDeadlineDraft({ session, caseId, action, capture }) {
  const principalId = session.principal()?.id;
  let alive = true,
    root = null,
    registration = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canDeadlines(principal.role, 'manage') &&
      session.canAdmit()
    );
  }
  function register(resourceId, revision) {
    if (!admitted()) throw new Error('La sesion no admite el borrador de plazo.');
    const next = {
      principalId,
      contextId: caseId,
      editorKind: 'deadline',
      resourceId,
      action,
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    if (
      registration &&
      (descriptorKey(next) !== descriptorKey(root) || root.baseRevision !== revision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
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
            saved.action !== action ||
            !['register', 'correct', 'set_attention', 'retire'].includes(action) ||
            value.definition?.input?.selection?.case_id !== caseId ||
            typeof value.definition.title !== 'string' ||
            typeof value.reason !== 'string' ||
            (action === 'register'
              ? value.current !== null || saved.resourceId !== null || saved.baseRevision !== 0
              : value.current?.id !== saved.resourceId ||
                value.current.case_id !== caseId ||
                value.current.revision !== saved.baseRevision ||
                value.id !== saved.resourceId) ||
            (value.step === 'uncertain' && !value.last)
          )
            throw new Error('El borrador no pertenece a este plazo.');
          factUuid(value.id);
          if (
            value.last &&
            (value.last.case_id !== caseId ||
              value.last.actor_id !== principalId ||
              value.last.command.deadline_id !== value.id ||
              value.last.command.change.action !== action ||
              value.last.command.change.expected_revision !== saved.baseRevision)
          )
            throw new Error('El envio no pertenece a este borrador.');
          captureDeadline(value);
          register(saved.resourceId, saved.baseRevision);
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
