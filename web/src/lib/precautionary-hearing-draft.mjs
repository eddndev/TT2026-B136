import { descriptorKey, plainRecord } from './draft-descriptor.mjs';
import {
  factObject as object,
  factSame as same,
  factInvalid as invalid,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { resourceHearingActor } from './resource-hearing-capture.mjs';
import { precautionaryHearingPrepared } from './precautionary-hearing-prepared.mjs';
import { precautionaryHearingRecord } from './precautionary-hearing-record.mjs';

const actions = ['schedule', 'replace', 'cancel'];
const fields = [
  'hearingId',
  'operationId',
  'action',
  'base',
  'fields',
  'participants',
  'support',
  'reviewTargets',
  'inputs',
  'mode',
  'last',
];
const inputFields = [
  'purpose',
  'date',
  'time',
  'offset',
  'modality',
  'venue',
  'note',
  'statement',
  'locator',
  'reason',
];
const identity = (principal) => ({
  id: principal?.id,
  email: principal?.email,
  role: principal?.role,
});
const principalKey = (principal) => JSON.stringify(identity(principal));
const baseRevision = (value) => value.base?.capture.review.result_revision ?? 0;

function validate(value) {
  uuid(value.hearingId);
  uuid(value.operationId);
  if (!actions.includes(value.action)) invalid();
  if (value.action === 'schedule') {
    if (value.base !== null) invalid();
  } else {
    const base = value.base?.capture;
    precautionaryHearingRecord(
      value.base,
      base?.review.case_id,
      value.hearingId,
      base?.review.result_revision,
    );
    if (base.review.status !== 'scheduled') invalid();
  }
  object(value.fields, inputFields);
  if (inputFields.some((key) => typeof value.fields[key] !== 'string')) invalid();
  if (!Array.isArray(value.participants) || value.participants.length > 32) invalid();
  const participants = new Set();
  for (const row of value.participants) {
    uuid(row.case_id);
    uuid(row.id);
    revision(row.revision);
    if (participants.has(row.id)) invalid();
    participants.add(row.id);
  }
  if (value.support !== null && !plainRecord(value.support)) invalid();
  if (!Array.isArray(value.reviewTargets) || value.reviewTargets.length > 32) invalid();
  if (value.inputs !== null && !plainRecord(value.inputs)) invalid();
  if (value.mode === 'uncertain' && !value.last) invalid('Falta el envio exacto retenido.');
  if (value.last) {
    const last = precautionaryHearingPrepared(value.last),
      command = last.command,
      change = command.change;
    if (
      command.hearing_id !== value.hearingId ||
      command.operation_id !== value.operationId ||
      change.action !== value.action ||
      (value.base &&
        (command.case_id !== value.base.capture.review.case_id ||
          change.expected_revision !== baseRevision(value) ||
          change.expected_capture_digest !== value.base.capture.capture_digest))
    )
      invalid('El envio retenido no corresponde al borrador y su base.');
  }
}

export function capturePrecautionaryHearingDraft(state) {
  const value = structuredClone(Object.fromEntries(fields.map((key) => [key, state[key]])));
  value.mode = ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft';
  validate(value);
  return value;
}

export function pendingPrecautionaryHearingDrafts(session, caseId) {
  const principal = session?.principal();
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === principal?.id &&
          row.instanceId === principalKey(principal) &&
          row.contextId === caseId &&
          row.editorKind === 'precautionary-hearing' &&
          actions.includes(row.action),
      ) ?? []
  );
}

export function createPrecautionaryHearingDraft({ session, caseId, action, hearingId, capture }) {
  uuid(caseId);
  uuid(hearingId);
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
      value.hearingId !== hearingId ||
      value.action !== action ||
      baseRevision(value) !== descriptor.baseRevision ||
      (value.base && value.base.capture.review.case_id !== caseId) ||
      value.participants.some((row) => row.case_id !== caseId) ||
      (value.support?.case_id !== undefined && value.support.case_id !== caseId) ||
      value.reviewTargets.some((row) => row.case_id !== undefined && row.case_id !== caseId) ||
      (value.last && (value.last.case_id !== caseId || !same(value.last.actor, principal)))
    )
      invalid('El borrador no corresponde a la identidad y el expediente actuales.');
  }
  function register(selectedRevision) {
    if (!admitted()) invalid('La sesion no admite el borrador de audiencia.');
    resourceHearingActor({ id: principal.id, email: principal.email });
    if (action === 'schedule') {
      if (selectedRevision !== 0) invalid();
    } else revision(selectedRevision);
    if (registration && root.baseRevision !== selectedRevision) {
      registration.dispose();
      registration = null;
    }
    root = {
      principalId: principal.id,
      contextId: caseId,
      resourceId: hearingId,
      editorKind: 'precautionary-hearing',
      action,
      instanceId,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: selectedRevision,
    };
    registration ??= session.registry.register(root, {
      fields: [...fields, 'principal'],
      capture() {
        const value = capturePrecautionaryHearingDraft(capture());
        owned(value, root);
        return { ...value, principal: { ...principal } };
      },
    });
  }
  function ownDescriptor(value) {
    return (
      value.principalId === principal.id &&
      value.contextId === caseId &&
      value.resourceId === hearingId &&
      value.editorKind === 'precautionary-hearing' &&
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
          const value = capturePrecautionaryHearingDraft(stored);
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
