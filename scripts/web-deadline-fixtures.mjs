// Provision independent deadline browser accounts and exact synthetic evidence.
import { randomUUID } from 'node:crypto';
import { calendarValues, declaredDate, profileDefinition, registration } from './web-deadline-definitions.mjs';
async function persist(call, route, command, method = 'POST', suffix = '') {
  const draft = await call('POST', `${route}/prepare`, command);
  return call(method, route + suffix, { command: draft.command, expected_submission_digest: draft.submission_digest }, 201);
}
async function resolution(call, caseId, label) {
  const route = `/cases/${caseId}/resolutions`;
  const command = { operation_id: randomUUID(), id: randomUUID(), family: 'resolution', change: {
    action: 'record', expected_revision: 0, values: {
      class: { kind: 'known', value: { kind: 'order' } }, subtype: null, issuer: { kind: 'known', value: 'Autoridad sintetica' },
      issued_at: declaredDate(31), summary: `${label}: resolucion exacta inicial`, provenance: { kind: 'operator_note', note: 'Fuente sintetica sin acreditacion juridica' },
    },
  } };
  const initial = await persist(call, route, command);
  const correction = { ...command, operation_id: randomUUID(), change: { action: 'correct', expected_revision: initial.revision,
    reason: 'Precision narrativa sin cambiar fecha declarada', values: { ...initial.values, summary: `${label}: relato posterior de la fuente` } } };
  const head = await persist(call, route, correction, 'PUT', `/${initial.id}`);
  return { initial, head };
}
export async function provisionDeadlines(call) {
  if (!process.env.IDENTITY_TEST_DATABASE_URL) throw new Error('disposable browser services are required');
  const fixture = {};
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    const password = `deadline browser ${role} password`;
    const row = await call('POST', '/users', { email: `deadlines.${role}@example.com`, password, role }, 201);
    fixture[role] = { id: row.user.id, email: row.user.email, password, recoveryCodes: row.recovery_codes };
  }
  for (const [key, title, reference] of [
    ['case', 'Plazos con calculo reproducible', 'DEADLINE-WORKFLOW'],
    ['policyCase', 'Plazos con autorizacion vigente', 'DEADLINE-POLICY'],
    ['raceCase', 'Plazos con correcciones concurrentes', 'DEADLINE-RACE'],
    ['hiddenCase', 'Plazos de otro expediente', 'DEADLINE-HIDDEN'],
  ]) {
    const row = await call('POST', '/penal-cases', { title, reference, profile: {
      nuc: `${reference}-NUC`, nuc_authority: 'Autoridad declarada', judicial_case_number: `${reference}-CJ`, judicial_authority: 'Organo declarado',
      offenses: ['Descripcion sintetica'], general_information: null, complementary_identifiers: null,
    } }, 201);
    fixture[key] = { id: row.id, title, reference };
    if (key !== 'hiddenCase') for (const role of ['litigator', 'paralegal', 'client'])
      await call('PUT', `/cases/${row.id}/members/${fixture[role].id}`, undefined, 204);
  }
  const values = calendarValues();
  const calendar = await persist(call, '/judicial-calendars', { operation_id: randomUUID(), calendar_id: randomUUID(), change: {
    action: 'publish', expected_revision: 0, values,
  } });
  const revisedValues = structuredClone(calendar.values);
  revisedValues.sources[0].locator = 'Referencia posterior sin modificar dias contables';
  fixture.calendar = calendar;
  fixture.calendarHead = await persist(call, '/judicial-calendars', { operation_id: randomUUID(), calendar_id: calendar.id, change: {
    action: 'replace', expected_revision: 1, reason: 'Precision de referencia sintetica', values: revisedValues,
  } }, 'PUT', `/${calendar.id}`);
  fixture.sources = {};
  fixture.profiles = {};
  for (const key of ['case', 'policyCase', 'raceCase', 'hiddenCase']) {
    fixture.sources[key] = await resolution(call, fixture[key].id, fixture[key].reference);
    const kinds = key === 'case' ? ['daily', 'monthly', 'hourly'] : ['daily'];
    fixture.profiles[key] = {};
    for (const kind of kinds) fixture.profiles[key][kind] = await persist(call, `/cases/${fixture[key].id}/deadline-profiles`, {
      operation_id: randomUUID(), profile_id: randomUUID(), change: { action: 'publish', expected_revision: 0,
        definition: profileDefinition(fixture[key].id, kind, calendar.values) },
    });
    if (key !== 'case') {
      const command = registration(fixture[key].id, fixture.profiles[key].daily, fixture.sources[key].initial,
        calendar, key === 'policyCase' ? fixture.litigator.id : fixture.owner.id, `Plazo inicial ${fixture[key].reference}`);
      fixture[`${key}Deadline`] = await persist(call, `/cases/${fixture[key].id}/deadlines`, command);
    }
  }
  return fixture;
}
