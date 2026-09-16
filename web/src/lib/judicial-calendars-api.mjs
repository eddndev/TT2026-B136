import { calendarValues, calendarUuid, sameCalendarData } from './judicial-calendar-values.mjs';
import { civilDate, civilDays } from './judicial-calendar-time.mjs';
import { calendarMatches, calendarRequest } from './judicial-calendar-submission.mjs';
import { calendarFailure } from './judicial-calendar-labels.mjs';
const digest = (value) => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
function integer(value, max = 4294967295) {
  if (!Number.isInteger(value) || value < 1 || value > max)
    throw new Error('Revisa el l\u00edmite o la revisi\u00f3n de consulta.');
}
function detail(row, id, revision) {
  if (
    row?.id !== id ||
    (revision !== undefined && row.revision !== revision) ||
    !digest(row.values_digest) ||
    !['published', 'retired'].includes(row.status)
  )
    throw new Error('El calendario no corresponde a la identidad o revisi\u00f3n consultadas.');
  integer(row.revision);
  calendarValues(row.values);
  return row;
}
export function judicialCalendarsApi(request) {
  let active = true;
  const base = '/judicial-calendars',
    key = (id) => encodeURIComponent(calendarUuid(id));
  const assertActive = () => {
    if (!active) throw new Error('La consulta de calendarios ya no est\u00e1 abierta.');
  };
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data && new TextEncoder().encode(JSON.stringify(options.data)).length > 1048576)
      throw Object.assign(new Error('El calendario supera el l\u00edmite de 1 MiB.'), {
        status: 413,
      });
    let result;
    try {
      result = await request(`${base}${suffix}`, options);
    } catch (failure) {
      assertActive();
      failure.message = calendarFailure(failure);
      throw failure;
    }
    assertActive();
    return result;
  }
  return {
    dispose() {
      active = false;
    },
    async list({
      status = 'published',
      jurisdiction = '',
      entityCode = '',
      limit = 20,
      afterId,
    } = {}) {
      integer(limit, 100);
      if (
        !['all', 'published', 'retired'].includes(status) ||
        (jurisdiction && !['federal', 'local'].includes(jurisdiction)) ||
        (entityCode && !/^(0[1-9]|[12]\d|3[0-2])$/.test(entityCode))
      )
        throw new Error('Revisa los filtros del calendario.');
      const query = new URLSearchParams({ status, limit });
      if (jurisdiction) query.set('jurisdiction', jurisdiction);
      if (entityCode) query.set('entity_code', entityCode);
      if (afterId) query.set('after_id', calendarUuid(afterId));
      const page = await call(`?${query}`);
      if (!Array.isArray(page.calendars) || page.calendars.length > limit)
        throw new Error('La lista de calendarios no es v\u00e1lida.');
      page.calendars.forEach((row) => {
        calendarUuid(row.id);
        integer(row.revision);
        if (!row.scope || !row.coverage || !digest(row.values_digest))
          throw new Error('Falta el resumen del calendario.');
        civilDate(row.coverage.from);
        civilDate(row.coverage.through);
      });
      return page;
    },
    async get(id) {
      return detail(await call(`/${key(id)}`), id);
    },
    async revision(id, revision) {
      integer(revision);
      return detail(await call(`/${key(id)}/revisions/${revision}`), id, revision);
    },
    async history(id, { limit = 10, beforeRevision } = {}) {
      integer(limit, 20);
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) {
        integer(beforeRevision);
        query.set('before_revision', beforeRevision);
      }
      const page = await call(`/${key(id)}/history?${query}`);
      if (!Array.isArray(page.revisions) || page.revisions.length > limit)
        throw new Error('El historial de calendario no es v\u00e1lido.');
      page.revisions.forEach((row) => {
        integer(row.revision);
        if (
          row.id !== id ||
          !digest(row.values_digest) ||
          (beforeRevision !== undefined && row.revision >= beforeRevision)
        )
          throw new Error('El historial no corresponde al calendario y l\u00edmite consultados.');
      });
      return page;
    },
    async days(id, revision, { from, through }, expectedDigest) {
      integer(revision);
      const length = civilDays(from, through);
      if (length < 1 || length > 62) throw new Error('Consulta de 1 a 62 d\u00edas inclusivos.');
      const row = await call(
        `/${key(id)}/revisions/${revision}/days?${new URLSearchParams({ from, through })}`,
      );
      if (
        row.calendar_id !== id ||
        row.revision !== revision ||
        !digest(row.values_digest) ||
        (expectedDigest && row.values_digest !== expectedDigest) ||
        !Array.isArray(row.days) ||
        row.days.length !== length
      )
        throw new Error('La clasificaci\u00f3n no corresponde al calendario exacto.');
      row.days.forEach((d, i) => {
        if (
          civilDays(from, d.date) !== i + 1 ||
          !['countable', 'excluded', 'unresolved', 'outside_coverage'].includes(d.state) ||
          !Array.isArray(d.source_ids)
        )
          throw new Error('La respuesta de d\u00edas est\u00e1 incompleta.');
        if (d.state === 'outside_coverage') {
          if (d.origin !== null || d.source_ids.length || d.explanation != null)
            throw new Error('Una fecha fuera de cobertura no tiene regla aplicable.');
        } else if (
          !['weekly_pattern', 'exception'].includes(d.origin) ||
          typeof d.explanation !== 'string' ||
          !d.explanation.length ||
          (d.origin === 'weekly_pattern' &&
            (!Number.isInteger(d.weekday) || d.weekday < 1 || d.weekday > 7))
        )
          throw new Error('Falta la regla declarada del d\u00eda.');
        if (d.origin === 'exception') calendarUuid(d.exception_id);
        d.source_ids.forEach(calendarUuid);
      });
      return row;
    },
    async prepare(command, actorId) {
      const row = await call('/prepare', { method: 'POST', data: command });
      if (
        row.actor_id !== actorId ||
        !sameCalendarData(row.command, command) ||
        row.result_revision !== command.change.expected_revision + 1 ||
        !digest(row.values_digest) ||
        !digest(row.submission_digest) ||
        !sameCalendarData(row.initial_scope, row.values?.scope) ||
        (command.change.values && !sameCalendarData(row.values, command.change.values))
      )
        throw new Error(
          'La preparaci\u00f3n no corresponde al actor, valores y operaci\u00f3n enviados.',
        );
      calendarValues(row.values);
      return row;
    },
    async submit(prepared) {
      const c = prepared.command,
        action = c.change.action;
      integer(prepared.result_revision);
      if (!['publish', 'replace', 'retire'].includes(action))
        throw new Error('La acci\u00f3n de calendario no es v\u00e1lida.');
      const suffix =
        action === 'publish'
          ? ''
          : `/${key(c.calendar_id)}${action === 'retire' ? '/retirement' : ''}`;
      const row = detail(
        await call(suffix, {
          method: action === 'replace' ? 'PUT' : 'POST',
          data: calendarRequest(prepared),
        }),
        c.calendar_id,
        prepared.result_revision,
      );
      if (!calendarMatches(row, prepared))
        throw new Error(
          'No se pudo confirmar el recibo del calendario. Consulta su revisi\u00f3n exacta.',
        );
      return row;
    },
  };
}
