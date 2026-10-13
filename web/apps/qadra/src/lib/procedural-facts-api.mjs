import { factFailure } from './procedural-fact-errors.mjs';
import { factRequest, factMatches } from './procedural-fact-submission.mjs';
import {
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  factNormalizeCommand,
  factPreparedValue,
  factScope,
  factMetadata,
  factRecordValue,
  factListRow,
} from './procedural-fact-validation.mjs';
function budget(value) {
  if (new TextEncoder().encode(JSON.stringify(value)).length > 524288)
    throw Object.assign(new Error('La declaraci\u00f3n supera el l\u00edmite de 512 KiB.'), {
      status: 413,
    });
}
export function proceduralFactsApi(request, caseId, family, resolutionId = undefined) {
  caseId = uuid(caseId);
  if (!['resolution', 'notification'].includes(family)) invalid();
  const parent = family === 'notification' ? uuid(resolutionId) : undefined;
  const key = encodeURIComponent,
    base = `/cases/${key(caseId)}/resolutions${family === 'notification' ? `/${key(parent)}/notifications` : ''}`;
  let active = true;
  function assertActive() {
    if (!active) invalid('El contexto de esta declaraci\u00f3n ya no est\u00e1 abierto.');
  }
  function scope(command) {
    if (
      command.family !== family ||
      (family === 'notification' && command.resolution_id !== parent)
    )
      invalid('El env\u00edo no corresponde a esta familia y resoluci\u00f3n padre.');
  }
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data) budget(options.data);
    try {
      const value = await request(`${base}${suffix}`, options);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      const failure = error && typeof error === 'object' ? error : new Error(String(error));
      failure.message = factFailure(failure);
      throw failure;
    }
  }
  function detail(value, id, exact) {
    return factRecordValue(value, caseId, family, parent, id, exact);
  }
  function page(value, rowsKey, limit, cursorKey, idKey, descending = false, before) {
    const rows = value?.[rowsKey];
    if (!Array.isArray(rows) || rows.length > limit || typeof value.has_more !== 'boolean')
      invalid('La p\u00e1gina recibida no es v\u00e1lida.');
    const ids = rows.map((row) => row?.[idKey]);
    if (
      ids.some((id) => id == null) ||
      ids.some((id, i) => i > 0 && (descending ? id >= ids[i - 1] : id <= ids[i - 1]))
    )
      invalid('El orden de la consulta no es v\u00e1lido.');
    if (before !== undefined && ids.some((id) => (descending ? id >= before : id <= before)))
      invalid('El cursor no corresponde a la consulta.');
    if (
      value.has_more
        ? rows.length !== limit || value[cursorKey] !== ids.at(-1)
        : value[cursorKey] !== null
    )
      invalid('La continuaci\u00f3n de la consulta no coincide.');
    return rows;
  }
  return {
    dispose() {
      active = false;
    },
    async list({ status = 'all', limit = 20, afterId } = {}) {
      revision(limit, 100);
      if (!['all', 'recorded', 'withdrawn'].includes(status)) invalid();
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ status, limit });
      if (after !== undefined) query.set('after_id', after);
      const result = await call(`?${query}`),
        rows = page(
          result,
          family === 'resolution' ? 'resolutions' : 'notifications',
          limit,
          'next_after_id',
          'id',
          false,
          after,
        );
      for (const row of rows) {
        factListRow(row, caseId, family, parent);
        if (status !== 'all' && row.status !== status) invalid('El estado filtrado no coincide.');
      }
      return result;
    },
    async get(id) {
      id = uuid(id);
      return detail(await call(`/${key(id)}`), id);
    },
    async revision(id, exact) {
      id = uuid(id);
      revision(exact);
      return detail(await call(`/${key(id)}/revisions/${exact}`), id, exact);
    },
    async history(id, { limit = 10, beforeRevision } = {}) {
      id = uuid(id);
      revision(limit, 20);
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) {
        revision(beforeRevision);
        query.set('before_revision', beforeRevision);
      }
      const result = await call(`/${key(id)}/history?${query}`);
      for (const row of page(
        result,
        'revisions',
        limit,
        'next_before_revision',
        'revision',
        true,
        beforeRevision,
      )) {
        factScope(row, caseId, family, parent, id);
        factMetadata(row);
        if (Object.hasOwn(row, 'values') || Object.hasOwn(row, 'sources'))
          invalid('El historial ligero contiene cuerpos inesperados.');
      }
      return result;
    },
    async prepare(raw, actorId) {
      assertActive();
      budget(raw);
      const expected = factNormalizeCommand(structuredClone(raw));
      scope(expected);
      if (typeof actorId !== 'string' || !actorId) invalid('Falta el actor del env\u00edo.');
      const value = await call('/prepare', { method: 'POST', data: structuredClone(expected) });
      factPreparedValue(value);
      if (value.case_id !== caseId || value.actor_id !== actorId || !same(value.command, expected))
        invalid('La preparaci\u00f3n no corresponde al actor y comando enviados.');
      return value;
    },
    async submit(raw) {
      assertActive();
      const prepared = structuredClone(raw),
        body = factRequest(prepared);
      budget(body);
      factPreparedValue(prepared);
      scope(prepared.command);
      if (prepared.case_id !== caseId) invalid('El env\u00edo pertenece a otro expediente.');
      const command = prepared.command,
        action = command.change.action;
      const suffix =
        action === 'record'
          ? ''
          : `/${key(command.id)}${action === 'withdraw' ? '/withdrawal' : ''}`;
      const result = detail(
        await call(suffix, { method: action === 'correct' ? 'PUT' : 'POST', data: body }),
        command.id,
        prepared.result_revision,
      );
      if (!factMatches(result, prepared))
        invalid(
          'No se pudo confirmar el recibo exacto del env\u00edo. Consulta la revisi\u00f3n enviada.',
        );
      return result;
    },
  };
}
