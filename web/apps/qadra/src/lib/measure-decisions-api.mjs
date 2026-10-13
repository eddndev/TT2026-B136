import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { resourceHearingUuid as uuid } from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { measureDecisionCommand } from './measure-decision-command.mjs';
import { measureDecisionPrepared } from './measure-decision-prepared.mjs';
import { measureDecisionOperation } from './measure-decision-operation.mjs';

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
    const id = uuid(item?.origin?.decision_id);
    if (previous !== null && id <= previous) invalid();
    measureDecisionOperation(item, caseId, id);
    previous = id;
  }
  if (value.next_after_id !== (value.has_more ? previous : null)) invalid();
  return value;
}

export function measureDecisionsApi(request, caseId) {
  uuid(caseId);
  const base = `/cases/${caseId}/measure-decisions`;
  let active = true;
  const assertActive = () => {
    if (!active) invalid('La consulta de decisiones cautelares ya no esta abierta.');
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
    const prepared = measureDecisionPrepared(structuredClone(raw));
    if (prepared.review.case_id !== caseId) invalid();
    return prepared;
  }
  function confirmed(value, prepared) {
    const { command } = prepared.review;
    measureDecisionOperation(value, caseId, command.decision_id, command.operation_id);
    if (value.family !== prepared.family || !same(value.group.review, prepared.review))
      invalid('El recibo no corresponde a la decision completa revisada.');
    return value;
  }
  return {
    dispose() {
      active = false;
    },
    async list(raw) {
      assertActive();
      const selected = query(raw);
      const after = selected.afterId === null ? '' : `&after_id=${selected.afterId}`;
      return page(await send(`${base}?limit=${selected.limit}${after}`), caseId, selected);
    },
    async get(id) {
      assertActive();
      uuid(id);
      return measureDecisionOperation(await send(`${base}/${id}`), caseId, id);
    },
    async prepare(raw, principal) {
      assertActive();
      const command = measureDecisionCommand(raw);
      if (command.case_id !== caseId) invalid();
      const actor = precautionaryHearingPrincipal(structuredClone(principal));
      const result = await send(`${base}/prepare`, { method: 'POST', data: command });
      const prepared = measureDecisionPrepared(result);
      if (!same(prepared.review.command, command) || !same(prepared.review.actor, actor)) invalid();
      return prepared;
    },
    async submit(raw) {
      const prepared = retained(raw),
        { review } = prepared;
      const value = await send(`${base}/submit`, {
        method: 'POST',
        data: {
          command: review.command,
          expected_submission_digest: review.submission_digest,
          expected_review_digest: review.review_digest,
        },
      });
      return confirmed(value, prepared);
    },
    async readSubmission(raw) {
      const prepared = retained(raw);
      let value;
      try {
        value = await send(`${base}/operations/${prepared.review.command.operation_id}`);
      } catch (failure) {
        assertActive();
        if (failure?.status === 404 && failure.code === 'measure_decision_not_found')
          return { state: 'unconfirmed' };
        throw failure;
      }
      return { state: 'confirmed', operation: confirmed(value, prepared) };
    },
  };
}
