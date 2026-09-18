import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
  factLabel as label,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineProfileScope, deadlineProfileDefinition } from './deadline-profile-values.mjs';
import { deadlinePage } from './deadline-page.mjs';
import { deadlineFailure } from './deadline-errors.mjs';
export function deadlineProfilesApi(request, caseId = null) {
  if (caseId !== null) caseId = uuid(caseId);
  const base = caseId === null ? '/deadline-profiles' : `/cases/${caseId}/deadline-profiles`;
  const collection = caseId === null ? { kind: 'global' } : { kind: 'case', case_id: caseId };
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de perfiles ya no est\u00e1 abierta.');
  };
  async function call(suffix = '') {
    assertActive();
    try {
      const value = await request(`${base}${suffix}`);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      const failure = error && typeof error === 'object' ? error : new Error(String(error));
      failure.message = deadlineFailure(failure);
      throw failure;
    }
  }
  function scope(value) {
    if (!same(value.collection, collection)) invalid('La colecci\u00f3n del perfil no coincide.');
  }
  function header(row, id, exact) {
    uuid(row?.id);
    revision(row.revision);
    digest(row.definition_digest);
    deadlineProfileScope(row.scope, caseId);
    if (
      (id !== undefined && row.id !== id) ||
      (exact !== undefined && row.revision !== exact) ||
      row.algorithm !== 'v1' ||
      !['published', 'retired'].includes(row.status)
    )
      invalid();
  }
  function metadata(row, id, exact) {
    header(row, id, exact);
    const receipt = row.receipt;
    object(receipt, ['operation_id', 'action', 'expected_revision', 'submission_digest']);
    uuid(receipt.operation_id);
    digest(receipt.submission_digest);
    if (
      !['publish', 'replace', 'retire'].includes(receipt.action) ||
      receipt.expected_revision !== row.revision - 1 ||
      (receipt.action === 'publish') !== (row.revision === 1) ||
      (receipt.action === 'retire') !== (row.status === 'retired')
    )
      invalid();
    if (receipt.action === 'publish') {
      if (row.reason !== null) invalid();
    } else text(row.reason);
    object(row.recorded_by, ['id', 'email']);
    uuid(row.recorded_by.id);
    text(row.recorded_by.email, 'Correo', 320, false);
    if (typeof row.recorded_at !== 'string' || !Number.isFinite(Date.parse(row.recorded_at)))
      invalid();
  }
  function detail(row, id, exact) {
    object(row, [
      'collection',
      'id',
      'revision',
      'status',
      'algorithm',
      'definition_digest',
      'scope',
      'reason',
      'receipt',
      'recorded_at',
      'recorded_by',
      'definition',
    ]);
    scope(row);
    metadata(row, id, exact);
    deadlineProfileDefinition(row.definition, caseId);
    if (!same(row.scope, row.definition.scope)) invalid();
    return row;
  }
  return {
    dispose() {
      active = false;
    },
    async list({ status = 'published', limit = 20, afterId } = {}) {
      revision(limit, 100);
      if (!['published', 'all', 'retired'].includes(status)) invalid();
      const after = afterId === undefined ? undefined : uuid(afterId),
        query = new URLSearchParams({ status, limit });
      if (after !== undefined) query.set('after_id', after);
      const value = await call(`?${query}`);
      object(value, ['collection', 'profiles', 'has_more', 'next_after_id']);
      scope(value);
      for (const row of deadlinePage(
        value,
        'profiles',
        limit,
        'next_after_id',
        'id',
        false,
        after,
      )) {
        object(row, [
          'id',
          'revision',
          'status',
          'algorithm',
          'definition_digest',
          'title',
          'scope',
        ]);
        header(row);
        label(row.title);
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
      object(value, ['collection', 'revisions', 'has_more', 'next_before_revision']);
      scope(value);
      const rows = deadlinePage(
        value,
        'revisions',
        limit,
        'next_before_revision',
        'revision',
        true,
        beforeRevision,
      );
      rows.forEach((row, i) => {
        object(row, [
          'id',
          'revision',
          'status',
          'algorithm',
          'definition_digest',
          'scope',
          'reason',
          'receipt',
          'recorded_at',
          'recorded_by',
        ]);
        metadata(row, id);
        if (
          i &&
          (rows[i - 1].revision !== row.revision + 1 ||
            !same(rows[i - 1].scope, row.scope) ||
            row.status === 'retired')
        )
          invalid();
      });
      return value;
    },
  };
}
