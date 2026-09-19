import { resourceActivityCommand } from './resource-activity-values.mjs';
import {
  resourceActivityDraft,
  resourceActivityRecord,
  resourceActivityView,
  resourceActivityMatches,
} from './resource-activity-validation.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';

export function resourceActivitiesApi(request, caseId, resourceId) {
  caseId = uuid(caseId);
  resourceId = uuid(resourceId);
  const base = `/cases/${caseId}/procedural-resources/${resourceId}/activities`;
  let active = true;
  function assertActive() {
    if (!active) invalid('La consulta del recurso ya no esta abierta.');
  }
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data && new TextEncoder().encode(JSON.stringify(options.data)).length > 16384)
      invalid('La solicitud supera el limite permitido.');
    try {
      const value = await request(base + suffix, options);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  function scope(value) {
    if (value.case_id !== caseId || value.resource_id !== resourceId)
      invalid('La respuesta pertenece a otro recurso o expediente.');
  }
  function page(value, rows, limit, cursorKey, values, descending, cursor) {
    if (!Array.isArray(rows) || rows.length > limit || typeof value.has_more !== 'boolean')
      invalid();
    if (
      values.some(
        (key, i) =>
          key == null ||
          (i && (descending ? key >= values[i - 1] : key <= values[i - 1])) ||
          (cursor !== undefined && (descending ? key >= cursor : key <= cursor)),
      )
    )
      invalid();
    if (
      value.has_more
        ? rows.length !== limit || value[cursorKey] !== values.at(-1)
        : value[cursorKey] !== null
    )
      invalid();
  }
  return {
    dispose() {
      active = false;
    },
    async list({ kind = 'all', status = 'linked', limit = 20, afterId } = {}) {
      revision(limit, 100);
      if (
        !['all', 'hearing', 'deadline'].includes(kind) ||
        !['all', 'linked', 'unlinked'].includes(status)
      )
        invalid();
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ limit });
      if (kind !== 'all') query.set('kind', kind);
      if (status !== 'all') query.set('status', status);
      if (after !== undefined) query.set('after_id', after);
      const value = await call(`?${query}`);
      object(value, ['case_id', 'resource_id', 'associations', 'has_more', 'next_after_id']);
      scope(value);
      const rows = value.associations;
      if (!Array.isArray(rows)) invalid();
      page(
        value,
        rows,
        limit,
        'next_after_id',
        rows.map((r) => r.association?.id),
        false,
        after,
      );
      for (const row of rows) {
        resourceActivityView(row, caseId, resourceId);
        if (
          (status !== 'all' && row.association.status !== status) ||
          (kind !== 'all' && row.association.selection.target.kind !== kind)
        )
          invalid();
      }
      return value;
    },
    async get(id) {
      id = uuid(id);
      return resourceActivityView(await call(`/${id}`), caseId, resourceId, id);
    },
    async revision(id, exact) {
      id = uuid(id);
      revision(exact);
      return resourceActivityView(
        await call(`/${id}/revisions/${exact}`),
        caseId,
        resourceId,
        id,
        exact,
      );
    },
    async history(id, { limit = 10, beforeRevision } = {}) {
      id = uuid(id);
      revision(limit, 20);
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) {
        revision(beforeRevision);
        query.set('before_revision', beforeRevision);
      }
      const value = await call(`/${id}/history?${query}`);
      object(value, [
        'case_id',
        'resource_id',
        'association_id',
        'revisions',
        'has_more',
        'next_before_revision',
      ]);
      scope(value);
      if (value.association_id !== id || !Array.isArray(value.revisions)) invalid();
      const rows = value.revisions;
      page(
        value,
        rows,
        limit,
        'next_before_revision',
        rows.map((r) => r.revision),
        true,
        beforeRevision,
      );
      rows.forEach((row) => resourceActivityRecord(row, caseId, resourceId, id));
      for (let i = 1; i < rows.length; i++) {
        if (
          rows[i - 1].revision !== rows[i].revision + 1 ||
          !same(rows[i - 1].receipt.previous, {
            revision: rows[i].revision,
            capture_digest: rows[i].receipt.capture_digest,
          }) ||
          !same(rows[i - 1].selection, rows[i].selection) ||
          !same(rows[i - 1].sources, rows[i].sources)
        )
          invalid();
      }
      if (
        (!rows.length && beforeRevision !== 1) ||
        (value.has_more && rows.at(-1)?.revision === 1) ||
        (!value.has_more && rows.length && rows.at(-1).revision !== 1)
      )
        invalid();
      return value;
    },
    async prepare(raw, actor) {
      const command = resourceActivityCommand(structuredClone(raw));
      scope(command);
      if (!actor?.id || !actor.email) invalid();
      const value = resourceActivityDraft(
        await call('/prepare', { method: 'POST', data: command }),
      );
      scope(value);
      if (!same(value.command, command) || !same(value.recorded_by, actor))
        invalid('El borrador no corresponde a la operacion e identidad enviadas.');
      return value;
    },
    async submit(raw) {
      const draft = structuredClone(raw);
      resourceActivityDraft(draft);
      scope(draft);
      const command = draft.command;
      const suffix = command.change.action === 'link' ? '' : `/${command.association_id}/unlink`;
      const row = await call(suffix, {
        method: 'POST',
        data: { command, expected_submission_digest: draft.submission_digest },
      });
      if (!resourceActivityMatches(row, draft))
        invalid(
          'No se pudo confirmar el recibo exacto de la asociacion. Consulta la revision enviada.',
        );
      return row;
    },
  };
}
