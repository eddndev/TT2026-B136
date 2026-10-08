import { descriptorKey, plainRecord } from './draft-descriptor.mjs';
import {
  factObject as object,
  factSame as same,
  factInvalid as invalid,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { measureAdministrationPrepared } from './measure-administration-prepared.mjs';
import { measureRecord } from './measure-record.mjs';

const actions = ['correct', 'entered_in_error', 'replace_entered_in_error'];
const fields = [
  'operationId',
  'action',
  'replacementId',
  'base',
  'fields',
  'subject',
  'inputs',
  'mode',
  'last',
];
const textFields = ['reason', 'conditions', 'supervisionText'];
const identity = (principal) => ({
  id: principal?.id,
  email: principal?.email,
  role: principal?.role,
});
const principalKey = (principal) => JSON.stringify(identity(principal));

function validate(value) {
  uuid(value.operationId);
  if (!actions.includes(value.action)) invalid();
  measureRecord(value.base, value.base?.case_id, value.base?.reference.id);
  if (value.action === 'replace_entered_in_error') {
    uuid(value.replacementId);
    if (value.replacementId === value.base.reference.id) invalid();
  } else if (value.replacementId !== null) invalid();
  object(value.fields, [...textFields, 'validity']);
  if (textFields.some((key) => typeof value.fields[key] !== 'string')) invalid();
  const validity = value.fields.validity;
  object(validity, ['start', 'statement', 'end']);
  if (
    !plainRecord(validity.start) ||
    typeof validity.statement !== 'string' ||
    (validity.end !== null && !plainRecord(validity.end))
  )
    invalid();
  if (value.subject !== null && !plainRecord(value.subject)) invalid();
  if (value.inputs !== null && !plainRecord(value.inputs)) invalid();
  if (value.mode === 'uncertain' && !value.last) invalid('Falta el envio exacto retenido.');
  if (value.last) {
    const { command } = measureAdministrationPrepared(value.last);
    if (
      command.case_id !== value.base.case_id ||
      command.operation_id !== value.operationId ||
      command.action.kind !== value.action ||
      !same(command.target, value.base.reference) ||
      (value.action === 'replace_entered_in_error' &&
        command.action.replacement_id !== value.replacementId)
    )
      invalid('El envio retenido no corresponde al borrador y su base exacta.');
  }
}

export function captureMeasureAdministrationDraft(state) {
  const value = structuredClone(Object.fromEntries(fields.map((key) => [key, state[key]])));
  value.mode = ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft';
  validate(value);
  return value;
}

export function pendingMeasureAdministrationDrafts(session, caseId) {
  const principal = session?.principal();
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === principal?.id &&
          row.instanceId === principalKey(principal) &&
          row.contextId === caseId &&
          row.editorKind === 'measure-administration' &&
          actions.includes(row.action),
      ) ?? []
  );
}

export function createMeasureAdministrationDraft({ session, caseId, measureId, action, capture }) {
  uuid(caseId);
  uuid(measureId);
  if (!actions.includes(action)) invalid();
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
  function owned(value, descriptor) {
    validate(value);
    if (
      value.action !== action ||
      value.base.case_id !== caseId ||
      value.base.reference.id !== measureId ||
      value.base.reference.revision !== descriptor.baseRevision ||
      (value.subject !== null && value.subject.case_id !== caseId) ||
      (value.last && (value.last.case_id !== caseId || !same(value.last.actor, principal)))
    )
      invalid('El borrador no corresponde a la identidad, medida y expediente actuales.');
  }
  function register(baseRevision) {
    if (!admitted()) invalid('La sesion no admite el borrador de rectificacion.');
    precautionaryHearingPrincipal(principal);
    revision(baseRevision);
    if (registration && root.baseRevision !== baseRevision) {
      registration.dispose();
      registration = null;
    }
    root = {
      principalId: principal.id,
      contextId: caseId,
      resourceId: measureId,
      editorKind: 'measure-administration',
      action,
      instanceId,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision,
    };
    registration ??= session.registry.register(root, {
      fields: [...fields, 'principal'],
      capture() {
        const value = captureMeasureAdministrationDraft(capture());
        owned(value, root);
        return { ...value, principal: { ...principal } };
      },
    });
  }
  function ownDescriptor(value) {
    return (
      value.principalId === principal.id &&
      value.contextId === caseId &&
      value.resourceId === measureId &&
      value.editorKind === 'measure-administration' &&
      value.action === action &&
      value.instanceId === instanceId &&
      value.schemaVersion === 1 &&
      value.ownerDraftKey === null &&
      value.rowId === null &&
      value.fieldPath.length === 0
    );
  }
  return {
    admitted,
    register,
    async restore(saved, fresh, apply) {
      let context, failure, descriptor;
      const result = await session.registry.restore(saved.key, {
        async authorize(stored) {
          descriptor = stored;
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
          const value = captureMeasureAdministrationDraft(stored);
          owned(value, descriptor);
          register(descriptor.baseRevision);
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
