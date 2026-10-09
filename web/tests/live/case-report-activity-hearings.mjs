import { randomUUID } from 'node:crypto';
import { expect } from '@playwright/test';
import { declaredDate } from '../../../scripts/web-deadline-definitions.mjs';

const unknown = () => ({ kind: 'unknown', reason: 'Dato no declarado en soporte sintetico' });
const envelope = (prepared) => ({
  command: prepared.command,
  expected_submission_digest: prepared.submission_digest,
});

export async function hearingResult(call, path, persist) {
  const context = await call('GET', `${path}/hearings/context`);
  const hearing = await persist(call, `${path}/hearings`, {
    operation_id: randomUUID(),
    hearing_id: randomUUID(),
    change: {
      action: 'schedule',
      expected_revision: 0,
      expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: {
        kind: 'initial',
        scheduled_at: '2026-01-06T10:00:00-06:00',
        modality: 'in_person',
        venue: 'Sede sintetica',
        note: null,
        participants: [],
        conviction_basis: null,
      },
    },
  });
  const route = `${path}/hearings/${hearing.id}/results`;
  const command = {
    operation_id: randomUUID(),
    hearing_id: hearing.id,
    result_id: randomUUID(),
    change: {
      action: 'record',
      expected_revision: 0,
      anchor_revision: hearing.revision,
      continuation: null,
      values: {
        occurrence: 'occurred',
        extent: 'partial',
        event_time: { precision: 'date', date: '2026-01-06', offset: '-06:00' },
        summary: 'Una sesion declarada con dos acuerdos',
        attendees: [],
        agreements: ['Primer acuerdo sintetico', 'Segundo acuerdo sintetico'].map((text) => ({
          id: randomUUID(),
          text,
        })),
        provenance: { kind: 'operator_note', reference: null, support: null },
      },
    },
  };
  const prepared = await call('POST', `${route}/prepare`, command);
  const payload = envelope(prepared);
  const result = await call('POST', route, payload, 201);
  expect(result.receipt.operation_id).toBe(command.operation_id);
  expect(result.receipt.action).toBe('record');
  const repeated = await call('POST', route, payload, 409);
  expect(repeated.error.code).toBe('hearing_result_operation_conflict');
  expect(await call('GET', `${route}/${result.id}/revisions/${result.revision}`)).toEqual(result);
  return { hearing, result };
}

export async function measureDecision(call, path, caseId, hearing, support) {
  const locator = { ...support, locator: 'Pagina 1' };
  const unknownValue = { state: 'unknown', reason: 'Dato no declarado' };
  const proposal = await call('POST', `${path}/participants/proposals/review`, {
    subject: {
      operation: 'create',
      values: {
        kind: 'natural_person',
        name: { state: 'known', value: 'Persona sintetica del informe' },
        curp: unknownValue,
        identity_support: locator,
      },
    },
    participant: { operation: 'create' },
    role: {
      organization: null,
      legal_status: null,
      profile: { kind: 'defendant', custody: unknownValue },
      role_support: locator,
    },
    certificate_base64: null,
  });
  const prepared = {
    proposal: proposal.proposal,
    review: {
      directory_stamp: proposal.directory_stamp,
      selection_reason: 'Identidad sintetica nueva revisada',
      different: [],
    },
    certificate_base64: null,
  };
  await call('POST', `${path}/participants/proposals/prepare`, prepared);
  const participant = await call(
    'POST',
    `${path}/participants/proposals/commit`,
    {
      prepared,
      signature_base64: null,
    },
    201,
  );
  const person = participant.subject;
  const context = (await call('GET', `${path}/precautionary-context`)).expectation;
  const command = {
    case_id: caseId,
    operation_id: randomUUID(),
    decision_id: randomUUID(),
    context,
    values: {
      authority: 'Organo declarado',
      declared_at: declaredDate(),
      justification: 'Decision judicial declarada sin inferir efectos adicionales',
      support,
      locator: 'Pagina 1',
    },
    anchor: {
      kind: 'initial',
      hearing_id: hearing.id,
      revision: hearing.revision,
      values_digest: hearing.values_digest,
      submission_digest: hearing.receipt.submission_digest,
    },
    outcome: {
      kind: 'changes',
      effects: ['periodic_appearance', 'travel_restriction'].map((kind) => ({
        action: 'impose',
        proposal: {
          id: randomUUID(),
          values: {
            subject: {
              id: person.id,
              revision: person.revision,
              values_digest: person.values_digest,
            },
            kind,
            conditions: 'Cumplir las condiciones declaradas en soporte sintetico',
            validity: { start: declaredDate(), statement: 'Vigencia declarada', end: null },
            supervision: unknown(),
          },
        },
      })),
    },
  };
  const route = `${path}/measure-decisions`;
  const { review } = await call('POST', `${route}/prepare`, command);
  const payload = { ...envelope(review), expected_review_digest: review.review_digest };
  const decision = await call('POST', `${route}/submit`, payload, 201);
  expect(decision.origin.decision_id).toBe(command.decision_id);
  expect(decision.group.measures).toHaveLength(2);
  expect(await call('POST', `${route}/submit`, payload, 201)).toEqual(decision);
  return decision;
}
