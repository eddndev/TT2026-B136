// Existing activities and historical evidence are captured through public HTTP.
import { randomUUID } from 'node:crypto';
import { provisionResources } from './web-resource-fixtures.mjs';
import { persistFollowRecord as persist } from './web-deadline-reevaluation.mjs';
import { calendarValues, profileDefinition, registration } from './web-deadline-definitions.mjs';

const reference = (row) => ({ id: row.id, revision: row.revision, capture_digest: row.receipt.capture_digest });
async function acts(call, scenario) {
  const route = `/cases/${scenario.case.id}/procedural-resources`;
  const resource = scenario.resource;
  scenario.resourceInitial = resource;
  const values = {
    kind: 'interposition', mode: { kind: 'known', value: 'oral' },
    occurred_at: { precision: 'unknown' },
    authority: { kind: 'unknown', reason: 'Autoridad no declarada en esta captura' },
    statement: `Acto original de ${scenario.case.reference}`,
    evidence: [structuredClone(resource.values.resolution_evidence)],
  };
  const actId = randomUUID();
  scenario.resourceAct = await persist(call, route, {
    operation_id: randomUUID(), resource_id: resource.id,
    change: { action: 'record_act', expected_revision: 1, act_id: actId, values },
  }, 'POST', `/${resource.id}/acts`);
  scenario.resource = await persist(call, route, {
    operation_id: randomUUID(), resource_id: resource.id,
    change: { action: 'correct_act', expected_revision: 2, act_id: actId, expected_act_revision: 1,
      values: { ...values, statement: `Acto corregido de ${scenario.case.reference}` },
      reason: 'Precision organizativa del relato declarado' },
  }, 'PUT', `/${resource.id}/acts/${actId}`);
}
async function hearing(call, scenario, scheduledAt) {
  const route = `/cases/${scenario.case.id}/hearings`;
  const context = await call('GET', route + '/context');
  const values = {
    kind: 'initial', scheduled_at: scheduledAt, modality: 'in_person',
    venue: `Sede original ${scenario.case.reference}`, note: null,
    participants: [], conviction_basis: null,
  };
  const command = { operation_id: randomUUID(), hearing_id: randomUUID(), change: {
    action: 'schedule', expected_revision: 0, expected_case_revision: context.case_revision,
    expected_stage_revision: context.stage_revision, values,
  } };
  scenario.hearingInitial = await persist(call, route, command);
  scenario.hearing = await persist(call, route, {
    operation_id: randomUUID(), hearing_id: command.hearing_id, change: {
      action: 'replace', expected_revision: 1, expected_case_revision: context.case_revision,
      expected_stage_revision: context.stage_revision,
      values: { ...values, venue: `Sede actual ${scenario.case.reference}` },
      reason: 'Cambio de sede sin modificar el horario declarado',
    },
  }, 'PUT', `/${command.hearing_id}`);
}
async function deadline(call, scenario, calendar, responsible, future) {
  const route = `/cases/${scenario.case.id}`;
  const values = structuredClone(scenario.resolution.values);
  values.issued_at = { precision: 'date', year: future.getUTCFullYear(),
    month: future.getUTCMonth() + 1, day: future.getUTCDate(), offset_seconds: null };
  values.summary = `Fuente sintetica de actividad ${scenario.case.reference}`;
  const source = await persist(call, route + '/resolutions', {
    family: 'resolution', id: randomUUID(), operation_id: randomUUID(),
    change: { action: 'record', expected_revision: 0, values },
  });
  const definition = profileDefinition(scenario.case.id, 'daily', calendar.values);
  definition.completion.from = calendar.values.coverage.from;
  definition.completion.through = calendar.values.coverage.through;
  const profile = await persist(call, route + '/deadline-profiles', {
    operation_id: randomUUID(), profile_id: randomUUID(),
    change: { action: 'publish', expected_revision: 0, definition },
  });
  const command = registration(scenario.case.id, profile, source, calendar, responsible,
    `Plazo original ${scenario.case.reference}`);
  command.change.tracking = { profile: 'fixed', source: 'fixed', calendar: 'fixed' };
  scenario.deadlineInitial = await persist(call, route + '/deadlines', command);
  await persist(call, route + '/deadlines', {
    operation_id: randomUUID(), deadline_id: command.deadline_id,
    change: { action: 'correct', expected_revision: 1, tracking: command.change.tracking,
      definition: { ...command.change.definition, title: `Plazo actual ${scenario.case.reference}` },
      reason: 'Precision del titulo sin alterar calculo ni atencion' },
  }, 'PUT', `/${command.deadline_id}`);
  scenario.deadline = await call('GET', route + `/deadlines/${command.deadline_id}`);
  if (!scenario.deadline.operational.due_at || scenario.deadline.revision !== 2)
    throw new Error('Resource activity fixture requires a checked future deadline at revision two');
}
async function seedPolicy(call, scenario) {
  const resource = scenario.resource, act = scenario.resourceAct;
  scenario.association = await persist(call,
    `/cases/${scenario.case.id}/procedural-resources/${resource.id}/activities`, {
      case_id: scenario.case.id, resource_id: resource.id, association_id: randomUUID(),
      operation_id: randomUUID(), expected_resource_revision: resource.revision,
      change: { action: 'link', expected_revision: 0, resource: reference(scenario.resourceInitial),
        act: { id: act.act.id, revision: act.act.revision, resource_revision: act.revision,
          capture_digest: act.receipt.capture_digest },
        target: { kind: 'hearing', id: scenario.hearingInitial.id, revision: 1,
          submission_digest: scenario.hearingInitial.receipt.submission_digest },
      },
    });
}
export async function provisionResourceActivities(call) {
  const fixture = await provisionResources(call, 'activity-');
  const future = new Date(Date.now() + 7 * 86400000);
  const values = calendarValues();
  values.scope.title = 'Calendario sintetico de actividades vinculadas';
  values.coverage = { from: `${Math.min(2026, future.getUTCFullYear())}-01-01`,
    through: `${Math.max(2026, future.getUTCFullYear()) + 1}-12-31` };
  const calendar = await persist(call, '/judicial-calendars', {
    operation_id: randomUUID(), calendar_id: randomUUID(),
    change: { action: 'publish', expected_revision: 0, values },
  });
  const scheduledAt = new Date(Date.now() + 36 * 3600000).toISOString().replace(/\.\d{3}Z$/, 'Z');
  for (const key of ['desktop', 'mobile', 'policy']) {
    const scenario = fixture[key];
    await call('PUT', `/cases/${scenario.case.id}/members/${fixture.owner.id}`, undefined, 204);
    await acts(call, scenario);
    await hearing(call, scenario, scheduledAt);
    await deadline(call, scenario, calendar, key === 'desktop' ? fixture.owner.id : fixture.litigator.id, future);
  }
  await seedPolicy(call, fixture.policy);
  return fixture;
}
