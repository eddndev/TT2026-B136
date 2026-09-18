import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineFailure } from './deadline-errors.mjs';
import { deadlineNormalizeCommand } from './deadline-values.mjs';
import {
  deadlinePreparedValue,
  deadlineRecordValue,
  deadlineListRow,
  deadlineScope,
  deadlineMetadata,
} from './deadline-validation.mjs';
import { deadlineResponsible } from './deadline-material.mjs';
import { deadlineRequest, deadlineMatches } from './deadline-submission.mjs';
import { deadlinePage, deadlineBudget } from './deadline-page.mjs';
export function deadlinesApi(request, caseId) {
  caseId = uuid(caseId);
  const base = `/cases/${caseId}/deadlines`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('El contexto de este plazo ya no est\u00e1 abierto.');
  };
  async function call(suffix = '', options) {
    assertActive();
    if (options?.data) deadlineBudget(options.data);
    try {
      const value = await request(`${base}${suffix}`, options);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      const failure = error && typeof error === 'object' ? error : new Error(String(error));
      failure.message = deadlineFailure(failure);
      throw failure;
    }
  }
  function scope(command) {
    if (command.change.definition && command.change.definition.input.selection.case_id !== caseId)
      invalid('El comando pertenece a otro expediente.');
  }
  const detail = (raw, id, exact) => deadlineRecordValue(raw, caseId, id, exact);
  return {
    dispose() {
      active = false;
    },
    async list({ status = 'active', limit = 20, afterId } = {}) {
      revision(limit, 100);
      if (!['active', 'retired', 'all'].includes(status)) invalid();
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ status, limit });
      if (after !== undefined) query.set('after_id', after);
      const value = await call(`?${query}`);
      object(value, ['case_id', 'deadlines', 'has_more', 'next_after_id']);
      if (value.case_id !== caseId) invalid();
      for (const row of deadlinePage(
        value,
        'deadlines',
        limit,
        'next_after_id',
        'id',
        false,
        after,
      )) {
        deadlineListRow(row, caseId);
        if (status !== 'all' && row.status !== status) invalid();
      }
      return value;
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
      const value = await call(`/${id}/history?${query}`);
      object(value, ['case_id', 'id', 'revisions', 'has_more', 'next_before_revision']);
      if (value.case_id !== caseId || value.id !== id) invalid();
      const rows = deadlinePage(
          value,
          'revisions',
          limit,
          'next_before_revision',
          'revision',
          true,
          beforeRevision,
        ),
        operations = new Set();
      rows.forEach((row, i) => {
        object(row, [
          'id',
          'case_id',
          'revision',
          'status',
          'reason',
          'receipt',
          'state_digest',
          'recorded_at',
          'recorded_by',
        ]);
        deadlineScope(row, caseId, id);
        deadlineMetadata(row);
        digest(row.state_digest);
        if (
          row.state_digest !== row.receipt.review_digest ||
          operations.has(row.receipt.operation_id) ||
          (i && (rows[i - 1].revision !== row.revision + 1 || row.status === 'retired'))
        )
          invalid();
        operations.add(row.receipt.operation_id);
      });
      if (
        (!rows.length && beforeRevision !== 1) ||
        (value.has_more && rows.at(-1)?.revision === 1) ||
        (!value.has_more && rows.length && rows.at(-1).revision !== 1)
      )
        invalid();
      return value;
    },
    async responsibles({ limit = 20, afterId } = {}) {
      revision(limit, 100);
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ limit });
      if (after !== undefined) query.set('after_id', after);
      const value = await call(`/responsibles?${query}`);
      object(value, ['case_id', 'responsibles', 'has_more', 'next_after_id']);
      if (value.case_id !== caseId) invalid();
      deadlinePage(value, 'responsibles', limit, 'next_after_id', 'id', false, after).forEach(
        deadlineResponsible,
      );
      return value;
    },
    async prepare(raw, actorId) {
      assertActive();
      deadlineBudget(raw);
      actorId = uuid(actorId);
      const expected = deadlineNormalizeCommand(structuredClone(raw));
      scope(expected);
      const value = await call('/prepare', { method: 'POST', data: structuredClone(expected) });
      deadlinePreparedValue(value);
      if (value.case_id !== caseId || value.actor_id !== actorId || !same(value.command, expected))
        invalid('La preparaci\u00f3n no corresponde al actor y comando enviados.');
      return value;
    },
    async submit(raw) {
      assertActive();
      const prepared = structuredClone(raw);
      deadlinePreparedValue(prepared);
      scope(prepared.command);
      if (prepared.case_id !== caseId) invalid('El env\u00edo pertenece a otro expediente.');
      const body = deadlineRequest(prepared);
      deadlineBudget(body);
      const c = prepared.command,
        action = c.change.action;
      const suffix =
        action === 'register'
          ? ''
          : `/${c.deadline_id}${action === 'set_attention' ? '/attention' : action === 'retire' ? '/retirement' : ''}`;
      const value = detail(
        await call(suffix, { method: action === 'correct' ? 'PUT' : 'POST', data: body }),
        c.deadline_id,
        prepared.result_revision,
      );
      if (!deadlineMatches(value, prepared))
        invalid(
          'No se pudo confirmar el recibo exacto del plazo. Consulta la revisi\u00f3n enviada.',
        );
      return value;
    },
  };
}
