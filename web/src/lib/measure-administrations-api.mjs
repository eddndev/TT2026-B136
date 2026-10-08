import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { measureAdministrationCommand } from './measure-administration-command.mjs';
import { measureAdministrationPrepared } from './measure-administration-prepared.mjs';
import { measureAdministrationOperation } from './measure-administration-operation.mjs';

function query(raw = {}) {
  object(raw, ['limit', 'afterId'], []);
  const limit = Object.hasOwn(raw, 'limit') ? raw.limit : 10;
  if (!Number.isInteger(limit) || limit < 1 || limit > 20) invalid();
  return { limit, afterId: Object.hasOwn(raw, 'afterId') ? uuid(raw.afterId) : null };
}

function page(value, caseId, { limit, afterId }) {
  object(value, ['case_id', 'items', 'has_more', 'next_after_operation_id']);
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
    const id = uuid(item?.origin?.operation_id);
    if (previous !== null && id <= previous) invalid();
    measureAdministrationOperation(item, caseId, id);
    previous = id;
  }
  if (value.next_after_operation_id !== (value.has_more ? previous : null)) invalid();
  return value;
}

export function measureAdministrationsApi(request, caseId) {
  uuid(caseId);
  const base = `/cases/${caseId}/measure-administrative-operations`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de rectificaciones ya no esta abierta.');
  };
  async function send(path, options) {
    assertActive();
    let value;
    try {
      value = await request(path, options);
    } catch (failure) {
      assertActive();
      throw failure;
    }
    assertActive();
    return value;
  }
  function retained(raw) {
    assertActive();
    const prepared = measureAdministrationPrepared(structuredClone(raw));
    if (prepared.case_id !== caseId) invalid();
    return prepared;
  }
  function confirmed(value, prepared) {
    measureAdministrationOperation(value, caseId, prepared.command.operation_id);
    if (!same(value.capture.review, prepared))
      invalid('El recibo no corresponde a la rectificacion completa revisada.');
    return value;
  }
  return {
    dispose() {
      active = false;
    },
    async list(raw) {
      assertActive();
      const selected = query(raw);
      const after = selected.afterId === null ? '' : `&after_operation_id=${selected.afterId}`;
      return page(await send(`${base}?limit=${selected.limit}${after}`), caseId, selected);
    },
    async get(id) {
      assertActive();
      uuid(id);
      return measureAdministrationOperation(await send(`${base}/${id}`), caseId, id);
    },
    async prepare(raw, principal) {
      assertActive();
      const command = measureAdministrationCommand(raw);
      if (command.case_id !== caseId) invalid();
      const actor = precautionaryHearingPrincipal(structuredClone(principal));
      const value = await send(`${base}/prepare`, { method: 'POST', data: command });
      const prepared = measureAdministrationPrepared(value);
      if (!same(prepared.command, command) || !same(prepared.actor, actor)) invalid();
      return prepared;
    },
    async submit(raw) {
      const prepared = retained(raw);
      const value = await send(`${base}/submit`, {
        method: 'POST',
        data: {
          command: prepared.command,
          expected_submission_digest: prepared.submission_digest,
          expected_review_digest: prepared.review_digest,
        },
      });
      return confirmed(value, prepared);
    },
    async readSubmission(raw) {
      const prepared = retained(raw);
      let value;
      try {
        value = await send(`${base}/${prepared.command.operation_id}`);
      } catch (failure) {
        assertActive();
        if (failure?.status === 404 && failure.code === 'measure_administrative_not_found')
          return { state: 'unconfirmed' };
        throw failure;
      }
      return { state: 'confirmed', operation: confirmed(value, prepared) };
    },
  };
}
