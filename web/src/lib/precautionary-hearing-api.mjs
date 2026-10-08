import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
} from './resource-hearing-values.mjs';
import { precautionaryHearingAgendaOverview } from './precautionary-agenda-values.mjs';
import {
  precautionaryHearingRecord,
  precautionaryHearingRecordOverview,
} from './precautionary-hearing-record.mjs';
import { alertInstant } from './alerts-primitives.mjs';
import { precautionaryHearingCommand } from './precautionary-hearing-command.mjs';
import {
  precautionaryHearingPrepared,
  precautionaryHearingPrincipal,
} from './precautionary-hearing-prepared.mjs';
import { precautionaryHearingContext } from './precautionary-hearing-values.mjs';

function listQuery(raw = {}) {
  object(raw, ['limit', 'afterId'], []);
  const limit = Object.hasOwn(raw, 'limit') ? raw.limit : 10;
  if (!Number.isInteger(limit) || limit < 1 || limit > 20) invalid();
  const afterId = Object.hasOwn(raw, 'afterId') ? uuid(raw.afterId) : null;
  return { limit, afterId };
}

function page(value, caseId, { limit, afterId }) {
  object(value, ['case_id', 'items', 'has_more', 'next_after_id']);
  if (
    value.case_id !== caseId ||
    !Array.isArray(value.items) ||
    value.items.length > limit ||
    typeof value.has_more !== 'boolean' ||
    (value.has_more && value.items.length !== limit)
  )
    invalid();
  let previous = afterId;
  for (const item of value.items) {
    const review = item?.capture?.review;
    const id = uuid(review?.command?.hearing_id);
    if (previous !== null && id <= previous) invalid();
    precautionaryHearingRecord(item, caseId, id, review.result_revision);
    previous = id;
  }
  if (value.next_after_id !== (value.has_more ? previous : null)) invalid();
  return value;
}

export function precautionaryHearingsApi(request, caseId) {
  uuid(caseId);
  const base = `/cases/${caseId}/precautionary-hearings`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de audiencia cautelar ya no esta abierta.');
  };
  function scope(selected) {
    if (selected.case_id !== caseId) invalid('La audiencia no pertenece al expediente consultado.');
  }
  async function send(path, options) {
    assertActive();
    let value;
    try {
      value = await request(path, options);
    } catch (failure) {
      assertActive();
      throw failure;
    }
    assertActive();
    return value;
  }
  async function read(id, selectedRevision) {
    const value = await send(`${base}/${id}/revisions/${selectedRevision}`);
    return precautionaryHearingRecord(value, caseId, id, selectedRevision);
  }
  function retained(raw) {
    assertActive();
    const review = precautionaryHearingPrepared(structuredClone(raw));
    scope(review);
    return review;
  }
  function confirmed(value, review) {
    precautionaryHearingRecord(value, caseId, review.command.hearing_id, review.result_revision);
    if (!same(value.capture.review, review))
      invalid('El recibo no corresponde a la revision completa confirmada.');
    return value;
  }
  return {
    dispose() {
      active = false;
    },
    async context() {
      const value = await send(`/cases/${caseId}/precautionary-context`);
      precautionaryHearingContext(value, caseId, true);
      return value;
    },
    async list(raw) {
      assertActive();
      const query = listQuery(raw);
      const suffix = query.afterId === null ? '' : `&after_id=${query.afterId}`;
      const value = await send(`${base}?limit=${query.limit}${suffix}`);
      return page(value, caseId, query);
    },
    async get(id) {
      assertActive();
      uuid(id);
      const value = await send(`${base}/${id}`);
      return precautionaryHearingRecord(value, caseId, id, value?.capture?.review?.result_revision);
    },
    async prepare(raw, principal) {
      assertActive();
      const command = precautionaryHearingCommand(raw);
      scope(command);
      const actor = precautionaryHearingPrincipal(structuredClone(principal));
      const value = await send(`${base}/prepare`, { method: 'POST', data: command });
      const review = precautionaryHearingPrepared(value);
      if (!same(review.command, command) || !same(review.actor, actor))
        invalid('La revision no corresponde al comando y principal declarados.');
      return review;
    },
    async submit(raw) {
      const review = retained(raw);
      const value = await send(`${base}/submit`, {
        method: 'POST',
        data: {
          command: review.command,
          expected_submission_digest: review.submission_digest,
          expected_review_digest: review.review_digest,
        },
      });
      return confirmed(value, review);
    },
    async readSubmission(raw) {
      const review = retained(raw);
      let value;
      try {
        value = await send(`${base}/operations/${review.command.operation_id}`);
      } catch (failure) {
        assertActive();
        if (failure?.status === 404 && failure.code === 'precautionary_hearing_not_found')
          return { state: 'unconfirmed' };
        throw failure;
      }
      return { state: 'confirmed', operation: confirmed(value, review) };
    },
    async exact(overview) {
      assertActive();
      const selected = precautionaryHearingAgendaOverview(structuredClone(overview));
      scope(selected);
      const value = await read(selected.id, selected.revision);
      if (!same(precautionaryHearingRecordOverview(value), selected))
        invalid('La captura no corresponde a la audiencia cautelar seleccionada en Agenda.');
      return value;
    },
    async fromAlert(raw) {
      assertActive();
      const selected = structuredClone(raw);
      object(selected.subject, ['kind', 'case_id', 'id']);
      const subject = selected.subject;
      if (subject.kind !== 'precautionary_hearing') invalid();
      uuid(subject.case_id);
      uuid(subject.id);
      scope(subject);
      object(selected.origin, ['revision', 'evidence_digest']);
      revision(selected.origin.revision);
      digest(selected.origin.evidence_digest);
      object(selected.kind, ['kind', 'lead_hours', 'activity_at']);
      if (
        selected.kind.kind !== 'upcoming' ||
        !Number.isInteger(selected.kind.lead_hours) ||
        selected.kind.lead_hours < 1 ||
        selected.kind.lead_hours > 720
      )
        invalid();
      const at = alertInstant(selected.kind.activity_at);
      const value = await read(subject.id, selected.origin.revision);
      const row = precautionaryHearingRecordOverview(value);
      if (
        row.status !== 'scheduled' ||
        row.capture_digest !== selected.origin.evidence_digest ||
        Date.parse(row.scheduled_at) / 1000 !== at.unix_seconds ||
        at.nanosecond !== 0
      )
        invalid('La captura de audiencia no corresponde al origen y la fecha del aviso.');
      return value;
    },
  };
}
