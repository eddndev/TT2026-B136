import { descriptorKey, plainRecord } from './draft-descriptor.mjs';
import {
  factObject as object,
  factSame as same,
  factInvalid as invalid,
} from './procedural-fact-primitives.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { measureDecisionPrepared, measureDecisionRows } from './measure-decision-prepared.mjs';

const fields = [
  'decisionId',
  'operationId',
  'fields',
  'support',
  'anchor',
  'effects',
  'inputs',
  'mode',
  'last',
];
const textFields = ['authority', 'justification', 'locator', 'outcome', 'statement'];
const identity = (principal) => ({
  id: principal?.id,
  email: principal?.email,
  role: principal?.role,
});
const principalKey = (principal) => JSON.stringify(identity(principal));

function validate(value) {
  uuid(value.decisionId);
  uuid(value.operationId);
  object(value.fields, [...textFields, 'declaredAt']);
  if (
    textFields.some((key) => typeof value.fields[key] !== 'string') ||
    !plainRecord(value.fields.declaredAt)
  )
    invalid();
  if (value.support !== null && !plainRecord(value.support)) invalid();
  if (value.anchor !== null) {
    object(value.anchor, ['kind', 'record']);
    if (
      !['initial', 'precautionary'].includes(value.anchor.kind) ||
      !plainRecord(value.anchor.record)
    )
      invalid();
  }
  measureDecisionRows(value.effects);
  if (value.effects.some((row) => !plainRecord(row))) invalid();
  if (value.inputs !== null && !plainRecord(value.inputs)) invalid();
  if (value.mode === 'uncertain' && !value.last) invalid('Falta el envio exacto retenido.');
  if (value.last) {
    const { command } = measureDecisionPrepared(value.last).review;
    if (command.decision_id !== value.decisionId || command.operation_id !== value.operationId)
      invalid('El envio retenido no corresponde al borrador de decision.');
  }
}

export function captureMeasureDecisionDraft(state) {
  const value = structuredClone(Object.fromEntries(fields.map((key) => [key, state[key]])));
  value.mode = ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft';
  validate(value);
  return value;
}

export function pendingMeasureDecisionDrafts(session, caseId) {
  const principal = session?.principal();
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === principal?.id &&
          row.instanceId === principalKey(principal) &&
          row.contextId === caseId &&
          row.editorKind === 'measure-decision' &&
          row.action === 'register',
      ) ?? []
  );
}

export function createMeasureDecisionDraft({ session, caseId, decisionId, capture }) {
  uuid(caseId);
  uuid(decisionId);
  const principal = identity(session.principal()),
    instanceId = principalKey(principal);
  let alive = true,
    root = null,
    registration = null;
  function admitted() {
    return (
      alive &&
      same(identity(session.principal()), principal) &&
      ['owner', 'litigator'].includes(principal.role) &&
      session.canAdmit()
    );
  }
  function owned(value) {
    validate(value);
    const anchorCase =
      value.anchor === null
        ? caseId
        : value.anchor.kind === 'initial'
          ? value.anchor.record.case_id
          : value.anchor.record.capture?.review.case_id;
    if (
      value.decisionId !== decisionId ||
      anchorCase !== caseId ||
      (value.support?.case_id !== undefined && value.support.case_id !== caseId) ||
      (value.last &&
        (value.last.review.case_id !== caseId || !same(value.last.review.actor, principal)))
    )
      invalid('El borrador no corresponde a la identidad y el expediente actuales.');
  }
  function register() {
    if (!admitted()) invalid('La sesion no admite el borrador de decision.');
    precautionaryHearingPrincipal(principal);
    root = {
      principalId: principal.id,
      contextId: caseId,
      resourceId: decisionId,
      editorKind: 'measure-decision',
      action: 'register',
      instanceId,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: 0,
    };
    registration ??= session.registry.register(root, {
      fields: [...fields, 'principal'],
      capture() {
        const value = captureMeasureDecisionDraft(capture());
        owned(value);
        return { ...value, principal: { ...principal } };
      },
    });
  }
  function ownDescriptor(value) {
    return (
      value.principalId === principal.id &&
      value.contextId === caseId &&
      value.resourceId === decisionId &&
      value.editorKind === 'measure-decision' &&
      value.action === 'register' &&
      value.instanceId === instanceId &&
      value.schemaVersion === 1 &&
      value.baseRevision === 0 &&
      value.ownerDraftKey === null &&
      value.rowId === null &&
      value.fieldPath.length === 0
    );
  }
  return {
    admitted,
    register,
    async restore(saved, fresh, apply) {
      let context, failure;
      const result = await session.registry.restore(saved.key, {
        async authorize(stored) {
          if (!admitted() || !ownDescriptor(stored)) return false;
          try {
            const record = await session.authorizeCase(caseId);
            if (!admitted()) return false;
            const admin = record?.administration;
            if (
              record?.id !== caseId ||
              admin?.case_id !== caseId ||
              !['active', 'closed'].includes(admin.administrative_status)
            )
              invalid();
            const current = await fresh();
            if (!plainRecord(current) || !admitted()) return false;
            context = { ...current, closed: admin.administrative_status === 'closed' };
            return true;
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(stored) {
          if (!admitted() || !same(stored.principal, principal)) invalid();
          const value = captureMeasureDecisionDraft(stored);
          owned(value);
          register();
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
      if (admitted()) registration?.capture();
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
