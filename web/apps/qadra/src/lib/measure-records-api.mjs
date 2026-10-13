import { factObject as object, factInvalid as invalid } from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { measureRecord } from './measure-record.mjs';

function query(raw = {}) {
  object(raw, ['limit', 'afterId'], []);
  const limit = Object.hasOwn(raw, 'limit') ? raw.limit : 10;
  if (!Number.isInteger(limit) || limit < 1 || limit > 20) invalid();
  return { limit, afterId: Object.hasOwn(raw, 'afterId') ? uuid(raw.afterId) : null };
}

function page(value, caseId, { limit, afterId }) {
  object(value, ['case_id', 'items', 'has_more', 'next_after_id']);
  if (
    value.case_id !== caseId ||
    !Array.isArray(value.items) ||
    value.items.length > limit ||
    typeof value.has_more !== 'boolean' ||
    (value.has_more && value.items.length !== limit)
  )
    invalid();
  let previous = afterId;
  for (const item of value.items) {
    const id = uuid(item?.reference?.id);
    if (previous !== null && id <= previous) invalid();
    measureRecord(item, caseId, id);
    previous = id;
  }
  if (value.next_after_id !== (value.has_more ? previous : null)) invalid();
  return value;
}

export function measureRecordsApi(request, caseId) {
  uuid(caseId);
  const base = `/cases/${caseId}/measures`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de medidas ya no esta abierta.');
  };
  async function read(path) {
    assertActive();
    let result;
    try {
      result = await request(path);
    } catch (failure) {
      assertActive();
      throw failure;
    }
    assertActive();
    return result;
  }
  return {
    dispose() {
      active = false;
    },
    async list(raw) {
      assertActive();
      const selected = query(raw);
      const suffix = selected.afterId === null ? '' : `&after_id=${selected.afterId}`;
      return page(await read(`${base}?limit=${selected.limit}${suffix}`), caseId, selected);
    },
    async get(id) {
      assertActive();
      uuid(id);
      return measureRecord(await read(`${base}/${id}`), caseId, id);
    },
    async exact(raw) {
      assertActive();
      const selected = reference(structuredClone(raw));
      const path = `${base}/${selected.id}/revisions/${selected.revision}?capture_digest=${selected.capture_digest}`;
      return measureRecord(await read(path), caseId, selected.id, selected);
    },
  };
}
