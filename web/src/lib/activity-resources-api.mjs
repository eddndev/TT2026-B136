import { resourceActivityView } from './resource-activity-validation.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';

export function activityResourcesApi(request, caseId, target) {
  caseId = uuid(caseId);
  object(target, ['kind', 'id']);
  if (!['hearing', 'deadline'].includes(target.kind)) invalid();
  const reference = { kind: target.kind, id: uuid(target.id) };
  const family = reference.kind === 'hearing' ? 'hearings' : 'deadlines';
  const base = `/cases/${caseId}/${family}/${reference.id}/resource-associations`;
  let active = true;
  function assertActive() {
    if (!active) invalid('La consulta de recursos relacionados ya no est\u00e1 abierta.');
  }
  async function call(parameters) {
    assertActive();
    try {
      const value = await request(`${base}?${parameters}`);
      assertActive();
      return value;
    } catch (error) {
      assertActive();
      throw error;
    }
  }
  function validatePage(value, query) {
    object(value, ['case_id', 'target', 'checked_at', 'associations', 'has_more', 'next_after_id']);
    if (value.case_id !== caseId || !same(value.target, reference)) invalid();
    deadlineInstant(value.checked_at);
    if (value.checked_at.offset_seconds !== 0) invalid();
    const rows = value.associations;
    if (!Array.isArray(rows) || rows.length > query.limit || typeof value.has_more !== 'boolean')
      invalid();
    let previous = query.afterId;
    for (const view of rows) {
      const record = view?.association;
      const resourceId = uuid(record?.resource_id);
      resourceActivityView(view, caseId, resourceId);
      if (
        !same(view.checked_at, value.checked_at) ||
        record.selection.target.kind !== reference.kind ||
        record.selection.target.id !== reference.id ||
        (query.status !== 'all' && record.status !== query.status) ||
        (previous !== undefined && record.id <= previous)
      )
        invalid(
          'La respuesta no conserva la actividad, su observaci\u00f3n o el orden solicitado.',
        );
      previous = record.id;
    }
    if (
      value.has_more
        ? rows.length !== query.limit || value.next_after_id !== previous
        : value.next_after_id !== null
    )
      invalid();
    return value;
  }
  return {
    dispose() {
      active = false;
    },
    async list({ limit = 20, afterId, status = 'linked' } = {}) {
      assertActive();
      revision(limit, 100);
      if (!['linked', 'unlinked', 'all'].includes(status)) invalid();
      const after = afterId === undefined ? undefined : uuid(afterId);
      const query = { limit, status, afterId: after };
      const parameters = new URLSearchParams({ limit, status });
      if (after !== undefined) parameters.set('after_id', after);
      return validatePage(await call(parameters), query);
    },
  };
}
