import { resourceCommand, resourceKinds } from './procedural-resource-values.mjs';
import { resourcePreparedValue, resourceRecordValue } from './procedural-resource-validation.mjs';
import { resourceRequest, resourceMatches } from './procedural-resource-submission.mjs';
import { resourceFailure } from './procedural-resource-errors.mjs';
import {
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
function budget(value) {
  if (new TextEncoder().encode(JSON.stringify(value)).length > 524288)
    throw Object.assign(new Error('El recurso supera el l\u00edmite de 512 KiB.'), { status: 413 });
}
export function proceduralResourcesApi(request, caseId) {
  caseId = uuid(caseId);
  const base = `/cases/${caseId}/procedural-resources`;
  let active = true;
  function assertActive() {
    if (!active) invalid('El contexto del expediente ya no est\u00e1 abierto.');
  }
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data) budget(options.data);
    try {
      const result = await request(base + suffix, options);
      assertActive();
      return result;
    } catch (error) {
      assertActive();
      const failure = error && typeof error === 'object' ? error : new Error(String(error));
      failure.message = resourceFailure(failure);
      throw failure;
    }
  }
  const detail = (value, id, exact) => resourceRecordValue(value, caseId, id, exact);
  function page(value, rowsKey, limit, cursorKey, idKey, descending, cursor) {
    const rows = value?.[rowsKey];
    if (!Array.isArray(rows) || rows.length > limit || typeof value.has_more !== 'boolean')
      invalid();
    const ids = rows.map((row) => row?.[idKey]);
    if (
      ids.some((id) => id == null) ||
      ids.some((id, i) => i > 0 && (descending ? id >= ids[i - 1] : id <= ids[i - 1])) ||
      (cursor !== undefined && ids.some((id) => (descending ? id >= cursor : id <= cursor)))
    )
      invalid();
    if (
      value.has_more
        ? rows.length !== limit || value[cursorKey] !== ids.at(-1)
        : value[cursorKey] !== null
    )
      invalid();
    return rows;
  }
  return {
    dispose() {
      active = false;
    },
    async list({ kind = 'all', status = 'all', limit = 20, afterId } = {}) {
      revision(limit, 100);
      if (
        (kind !== 'all' && !Object.hasOwn(resourceKinds, kind)) ||
        !['all', 'active', 'archived'].includes(status)
      )
        invalid();
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ limit });
      if (kind !== 'all') query.set('kind', kind);
      if (status !== 'all') query.set('status', status);
      if (after !== undefined) query.set('after_id', after);
      const result = await call(`?${query}`);
      for (const row of page(result, 'resources', limit, 'next_after_id', 'id', false, after)) {
        detail(row);
        if (
          (status !== 'all' && row.status !== status) ||
          (kind !== 'all' && row.values.kind !== kind)
        )
          invalid();
      }
      return result;
    },
    async get(id) {
      id = uuid(id);
      return detail(await call(`/${id}`), id);
    },
    async revision(id, exact) {
      id = uuid(id);
      revision(exact);
      return detail(await call(`/${id}/revisions/${exact}`), id, exact);
    },
    async history(id, { limit = 10, beforeRevision } = {}) {
      id = uuid(id);
      revision(limit, 20);
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) {
        revision(beforeRevision);
        query.set('before_revision', beforeRevision);
      }
      const result = await call(`/${id}/history?${query}`);
      for (const row of page(
        result,
        'revisions',
        limit,
        'next_before_revision',
        'revision',
        true,
        beforeRevision,
      ))
        detail(row, id);
      return result;
    },
    async prepare(raw, actor) {
      assertActive();
      budget(raw);
      const command = resourceCommand(structuredClone(raw));
      if (!actor?.id || !actor.email) invalid('Falta la identidad del env\u00edo.');
      const value = resourcePreparedValue(
        await call('/prepare', { method: 'POST', data: structuredClone(command) }),
      );
      if (
        value.case_id !== caseId ||
        !same(value.recorded_by, actor) ||
        !same(value.command, command)
      )
        invalid('La preparaci\u00f3n no corresponde al expediente, actor y comando enviados.');
      return value;
    },
    async submit(raw) {
      assertActive();
      const prepared = structuredClone(raw);
      resourcePreparedValue(prepared);
      if (prepared.case_id !== caseId) invalid('El env\u00edo pertenece a otro expediente.');
      const command = prepared.command,
        action = command.change.action,
        id = command.resource_id;
      const paths = {
        register: '',
        correct: `/${id}`,
        record_act: `/${id}/acts`,
        correct_act: `/${id}/acts/${command.change.act_id}`,
        archive: `/${id}/archive`,
        reactivate: `/${id}/reactivation`,
      };
      const result = detail(
        await call(paths[action], {
          method: ['correct', 'correct_act'].includes(action) ? 'PUT' : 'POST',
          data: resourceRequest(prepared),
        }),
        id,
        prepared.result_revision,
      );
      if (!resourceMatches(result, prepared))
        invalid('No se pudo confirmar el recibo exacto. Consulta la revisi\u00f3n enviada.');
      return result;
    },
  };
}
