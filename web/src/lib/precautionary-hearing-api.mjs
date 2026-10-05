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
  async function read(id, selectedRevision) {
    assertActive();
    let value;
    try {
      value = await request(`${base}/${id}/revisions/${selectedRevision}`);
    } catch (failure) {
      assertActive();
      throw failure;
    }
    assertActive();
    return precautionaryHearingRecord(value, caseId, id, selectedRevision);
  }
  return {
    dispose() {
      active = false;
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
