import { descriptorKey } from './draft-descriptor.mjs';
import {
  factObject as object,
  factSame as same,
  factInvalid as invalid,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { resourceRecordValue } from './procedural-resource-validation.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { resourceHearingActor } from './resource-hearing-capture.mjs';
import { resourceHearingPrepared } from './resource-hearing-prepared.mjs';

const fields = [
  'hearingId',
  'associationId',
  'operationId',
  'base',
  'resource',
  'act',
  'fields',
  'participants',
  'mode',
  'last',
  'inputs',
];
const inputFields = [
  'kind',
  'date',
  'time',
  'offset',
  'modality',
  'venue',
  'note',
  'statement',
  'supportKey',
];
const identity = (principal) => ({
  id: principal?.id,
  email: principal?.email,
  role: principal?.role,
});
const principalKey = (principal) => JSON.stringify(identity(principal));
const reference = (row) => ({
  id: row.id,
  revision: row.revision,
  capture_digest: row.receipt.capture_digest,
});

function validate(value) {
  for (const name of ['hearingId', 'associationId', 'operationId']) uuid(value[name]);
  const resource = value.resource,
    base = value.base;
  uuid(resource?.case_id);
  uuid(resource?.id);
  resourceRecordValue(resource, resource.case_id, resource.id, resource.revision);
  resourceRecordValue(base, resource.case_id, resource.id, base?.revision);
  if (resource.revision > base.revision) invalid();
  if (value.act !== null) {
    resourceRecordValue(value.act, resource.case_id, resource.id, value.act?.revision);
    if (!value.act.act || value.act.revision > base.revision) invalid();
  }
  object(value.fields, inputFields);
  if (inputFields.some((key) => typeof value.fields[key] !== 'string')) invalid();
  if (!Array.isArray(value.participants) || value.participants.length > 32) invalid();
  const ids = new Set();
  for (const row of value.participants) {
    uuid(row.id);
    revision(row.revision);
    if (row.case_id !== resource.case_id || ids.has(row.id)) invalid();
    ids.add(row.id);
  }
  if (value.inputs !== null) {
    object(value.inputs, ['choosingAct', 'choosingParticipant', 'actPicker', 'participantPicker']);
    if (
      typeof value.inputs.choosingAct !== 'boolean' ||
      typeof value.inputs.choosingParticipant !== 'boolean'
    )
      invalid();
  }
  if (value.mode === 'uncertain' && !value.last) invalid('Falta el envio exacto retenido.');
  if (value.last) {
    const last = resourceHearingPrepared(value.last),
      command = last.command;
    if (
      command.case_id !== resource.case_id ||
      command.resource_id !== resource.id ||
      command.hearing_id !== value.hearingId ||
      command.association_id !== value.associationId ||
      command.operation_id !== value.operationId ||
      command.expected_resource_revision !== base.revision ||
      !same(reference(base), last.observed_resource_head) ||
      !same(resource, last.resource) ||
      !same(value.act, last.act)
    )
      invalid();
  }
}

export function captureResourceHearingDraft(state) {
  const value = structuredClone(Object.fromEntries(fields.map((key) => [key, state[key]])));
  value.mode = ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft';
  validate(value);
  return value;
}

export function pendingResourceHearingDrafts(session, caseId, resourceId) {
  const principal = session?.principal();
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === principal?.id &&
          row.instanceId === principalKey(principal) &&
          row.contextId === caseId &&
          row.resourceId === resourceId &&
          row.editorKind === 'resource-hearing' &&
          row.action === 'register',
      ) ?? []
  );
}

export function createResourceHearingDraft({ session, caseId, resourceId, capture }) {
  uuid(caseId);
  uuid(resourceId);
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
      value.resource.case_id !== caseId ||
      value.resource.id !== resourceId ||
      value.base.revision !== descriptor.baseRevision ||
      (value.last && !same(value.last.recorded_by, { id: principal.id, email: principal.email }))
    )
      invalid();
  }
  function register(baseRevision) {
    if (!admitted()) invalid('La sesion no admite el borrador de audiencia.');
    resourceHearingActor({ id: principal.id, email: principal.email });
    revision(baseRevision);
    if (registration && root.baseRevision !== baseRevision) {
      registration.dispose();
      registration = null;
    }
    root = {
      principalId: principal.id,
      contextId: caseId,
      resourceId,
      editorKind: 'resource-hearing',
      action: 'register',
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
        const value = captureResourceHearingDraft(capture());
        owned(value, root);
        return { ...value, principal: { ...principal } };
      },
    });
  }
  function ownDescriptor(value) {
    return (
      value.principalId === principal.id &&
      value.contextId === caseId &&
      value.resourceId === resourceId &&
      value.editorKind === 'resource-hearing' &&
      value.action === 'register' &&
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
            if (!current || typeof current !== 'object' || Array.isArray(current) || !admitted())
              return false;
            context = { ...current, closed: admin.administrative_status === 'closed' };
            return true;
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(stored) {
          if (!admitted() || !same(stored.principal, principal)) invalid();
          const value = captureResourceHearingDraft(stored);
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
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
