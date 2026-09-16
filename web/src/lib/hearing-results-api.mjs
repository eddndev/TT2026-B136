import { hearingResultFailure } from './hearing-result-errors.mjs';
import { hearingResultTimeDraft } from './hearing-result-time.mjs';
import { hearingResultMatches, hearingResultRequest } from './hearing-result-submission.mjs';
const digest = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
function integer(value, maximum) {
  if (!Number.isInteger(value) || value < 1 || value > maximum)
    throw new Error('El l\u00edmite o la revisi\u00f3n de la consulta no es v\u00e1lido.');
}
export function hearingResultsApi(request, caseId, hearingId) {
  let active = true;
  const key = encodeURIComponent,
    base = `/cases/${key(caseId)}/hearings/${key(hearingId)}/results`;
  function assertActive() {
    if (!active) throw new Error('El contexto de este registro ya no est\u00e1 abierto.');
  }
  function projections(row) {
    const values = row.values;
    if (!values?.event_time) throw new Error('Falta el tiempo declarado del resultado.');
    if (
      !Array.isArray(row.attendees) ||
      !Array.isArray(values.attendees) ||
      row.attendees.length !== values.attendees.length ||
      values.attendees.some(
        (ref) =>
          !row.attendees.some(
            (person) =>
              person.id === ref.participant_id &&
              person.revision === ref.revision &&
              digest(person.values_digest),
          ),
      )
    )
      throw new Error('Las comparecencias no corresponden a las referencias exactas del registro.');
    const support = values.provenance?.support;
    if (
      support &&
      (row.support?.document_id !== support.document_id ||
        row.support.version !== support.version ||
        row.support.digest !== support.digest)
    )
      throw new Error('El soporte no corresponde a la referencia exacta del registro.');
    hearingResultTimeDraft(values.event_time);
  }
  function validate(row, id, revision) {
    if (!row?.id || row.case_id !== caseId || row.hearing_id !== hearingId || (id && row.id !== id))
      throw new Error('El resultado no corresponde al expediente y audiencia consultados.');
    integer(row.revision, 4294967295);
    if (revision !== undefined && row.revision !== revision)
      throw new Error('El resultado no corresponde a la revisi\u00f3n exacta consultada.');
    if (row.values) projections(row);
    else {
      if (!row.event_time) throw new Error('Falta el tiempo declarado del resultado.');
      hearingResultTimeDraft(row.event_time);
    }
    return row;
  }
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data && new TextEncoder().encode(JSON.stringify(options.data)).length > 524288)
      throw Object.assign(new Error('El registro supera el l\u00edmite admitido de 512 KiB.'), {
        status: 413,
      });
    let value;
    try {
      value = await request(`${base}${suffix}`, options);
    } catch (error) {
      error.message = hearingResultFailure(error);
      throw error;
    }
    assertActive();
    return value;
  }
  return {
    dispose: () => {
      active = false;
    },
    async list({ status = 'all', limit = 20, afterId } = {}) {
      integer(limit, 100);
      if (!['all', 'recorded', 'withdrawn'].includes(status))
        throw new Error('Revisa el estado del registro.');
      const query = new URLSearchParams({ status, limit });
      if (afterId) query.set('after_id', afterId);
      const page = await call(`?${query}`);
      page.results.forEach((row) => validate(row));
      return page;
    },
    async get(id) {
      return validate(await call(`/${key(id)}`), id);
    },
    async revision(id, revision) {
      integer(revision, 4294967295);
      return validate(await call(`/${key(id)}/revisions/${revision}`), id, revision);
    },
    async history(id, { limit = 10, beforeRevision } = {}) {
      integer(limit, 20);
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) {
        integer(beforeRevision, 4294967295);
        query.set('before_revision', beforeRevision);
      }
      const page = await call(`/${key(id)}/history?${query}`);
      page.revisions.forEach((row) => integer(row.revision, 4294967295));
      return page;
    },
    async prepare(command, actorId) {
      if (command.hearing_id !== hearingId)
        throw new Error('La programaci\u00f3n no corresponde al env\u00edo.');
      const value = await call('/prepare', { method: 'POST', data: command });
      const actual = value.command,
        change = command.change;
      if (
        value.case_id !== caseId ||
        value.actor_id !== actorId ||
        actual?.hearing_id !== hearingId ||
        actual.result_id !== command.result_id ||
        actual.operation_id !== command.operation_id ||
        actual.change?.action !== change.action ||
        actual.change.expected_revision !== change.expected_revision ||
        value.result_revision !== change.expected_revision + 1 ||
        !digest(value.values_digest) ||
        !digest(value.submission_digest) ||
        value.anchor?.hearing_id !== hearingId ||
        !digest(value.anchor.values_digest) ||
        !digest(value.anchor.submission_digest)
      )
        throw new Error(
          'La preparaci\u00f3n no corresponde al actor, fuentes y operaci\u00f3n enviados.',
        );
      if (
        change.action === 'record' &&
        (value.anchor.revision !== change.anchor_revision ||
          (change.continuation == null
            ? value.continuation !== null
            : value.continuation?.result_id !== change.continuation.result_id ||
              value.continuation.revision !== change.continuation.revision))
      )
        throw new Error('Las fuentes preparadas no corresponden a la selecci\u00f3n exacta.');
      projections(value);
      return value;
    },
    async submit(prepared) {
      const command = prepared.command,
        action = command.change.action;
      if (
        prepared.case_id !== caseId ||
        command.hearing_id !== hearingId ||
        !['record', 'correct', 'withdraw'].includes(action)
      )
        throw new Error('El env\u00edo no corresponde a este expediente y audiencia.');
      const suffix =
        action === 'record'
          ? ''
          : `/${key(command.result_id)}${action === 'withdraw' ? '/withdrawal' : ''}`;
      const value = validate(
        await call(suffix, {
          method: action === 'correct' ? 'PUT' : 'POST',
          data: hearingResultRequest(prepared),
        }),
        command.result_id,
        prepared.result_revision,
      );
      if (!hearingResultMatches(value, prepared))
        throw new Error(
          'No se pudo confirmar el recibo del env\u00edo. Consulta su revisi\u00f3n exacta.',
        );
      return value;
    },
  };
}
