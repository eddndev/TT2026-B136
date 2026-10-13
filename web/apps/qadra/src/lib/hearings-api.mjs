import { hearingFailure } from './hearing-errors.mjs';
import { hearingMatches, hearingRequest } from './hearing-submission.mjs';
import { hearingTimeParts } from './hearing-time.mjs';
export function validateHearing(record, caseId, id, revision) {
  if (
    !record ||
    (caseId && record.case_id !== caseId) ||
    !record.case_id ||
    (id && record.id !== id) ||
    !record.id
  )
    throw new Error('La audiencia no corresponde al expediente y recurso consultados.');
  if (
    !Number.isInteger(record.revision) ||
    record.revision < 1 ||
    (revision !== undefined && record.revision !== revision)
  )
    throw new Error('La audiencia no corresponde a la revisi\u00f3n solicitada.');
  hearingTimeParts(record.values?.scheduled_at || record.scheduled_at);
  return record;
}
export function hearingsApi(request, caseId) {
  let active = true;
  const base = `/cases/${encodeURIComponent(caseId)}/hearings`,
    key = encodeURIComponent;
  const assertActive = () => {
    if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
  };
  async function call(suffix = '', options) {
    assertActive();
    let result;
    try {
      result = await request(`${base}${suffix}`, options);
    } catch (error) {
      error.message = hearingFailure(error);
      throw error;
    }
    assertActive();
    return result;
  }
  return {
    dispose: () => {
      active = false;
    },
    async context() {
      const result = await call('/context');
      if (result.case_id !== caseId)
        throw new Error('El contexto no corresponde al expediente abierto.');
      return result;
    },
    async list({ status = 'all', limit = 20, afterId } = {}) {
      const query = new URLSearchParams({ status, limit });
      if (afterId) query.set('after_id', afterId);
      const page = await call(`?${query}`);
      page.hearings.forEach((row) => validateHearing(row, caseId));
      return page;
    },
    async get(id) {
      return validateHearing(await call(`/${key(id)}`), caseId, id);
    },
    async revision(id, revision) {
      return validateHearing(await call(`/${key(id)}/revisions/${revision}`), caseId, id, revision);
    },
    async history(id, { limit = 20, beforeRevision } = {}) {
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) query.set('before_revision', beforeRevision);
      const page = await call(`/${key(id)}/history?${query}`);
      page.revisions.forEach((row) => validateHearing(row, caseId, id));
      return page;
    },
    async prepare(command, actorId) {
      const result = await call('/prepare', { method: 'POST', data: command });
      const actual = result.command,
        expected = command.change;
      if (result.case_id !== caseId)
        throw new Error('La preparaci\u00f3n no corresponde al expediente.');
      if (result.actor_id !== actorId)
        throw new Error('La preparaci\u00f3n no corresponde al actor de esta sesi\u00f3n.');
      if (
        !actual ||
        actual.hearing_id !== command.hearing_id ||
        actual.operation_id !== command.operation_id ||
        actual.change?.action !== expected.action ||
        actual.change.expected_revision !== expected.expected_revision ||
        actual.change.expected_case_revision !== expected.expected_case_revision ||
        actual.change.expected_stage_revision !== expected.expected_stage_revision ||
        result.result_revision !== expected.expected_revision + 1 ||
        !/^[a-f0-9]{64}$/.test(result.submission_digest)
      )
        throw new Error('La preparaci\u00f3n no corresponde a la operaci\u00f3n enviada.');
      hearingTimeParts(result.values.scheduled_at);
      return result;
    },
    async submit(prepared) {
      const command = prepared.command,
        action = command.change.action;
      if (prepared.case_id !== caseId || !['schedule', 'replace', 'cancel'].includes(action))
        throw new Error('El env\u00edo no corresponde a este expediente.');
      const suffix =
        action === 'schedule'
          ? ''
          : `/${key(command.hearing_id)}${action === 'cancel' ? '/cancellation' : ''}`;
      const result = validateHearing(
        await call(suffix, {
          method: action === 'replace' ? 'PUT' : 'POST',
          data: hearingRequest(prepared),
        }),
        caseId,
        command.hearing_id,
        prepared.result_revision,
      );
      if (!hearingMatches(result, prepared))
        throw new Error(
          'No se pudo confirmar el recibo del env\u00edo. Consulta su revisi\u00f3n exacta.',
        );
      return result;
    },
  };
}
